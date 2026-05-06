//! `ConcreteOps<M>` — the manual runtime that drives every project
//! model registered via `Admin::model::<M>()`.
//!
//! Every framework-level read or write goes through one of the
//! [`AdminOps`](super::types::AdminOps) methods, which in turn calls
//! the matching free function in `crate::orm`. The trait stays
//! `pub(crate)` because handlers route through `AdminEntry::ops`
//! directly; consumers never name `ConcreteOps`.

use std::future::Future;
use std::pin::Pin;

use crate::error::Result;
use crate::http::FormData;
use crate::orm::Db;

use super::types::{AdminModel, AdminOps, CreateResult, EditRow, ListRow, UpdateResult};

pub(crate) struct ConcreteOps<M> {
    _marker: std::marker::PhantomData<M>,
}

impl<M> ConcreteOps<M> {
    pub(crate) fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<M> AdminOps for ConcreteOps<M>
where
    M: AdminModel + crate::orm::Model,
{
    fn list<'a>(
        &'a self,
        db: &'a Db,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ListRow>>> + Send + 'a>> {
        Box::pin(async move {
            let rows = crate::orm::all::<M>(db).await?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let id = AdminModel::id(&r);
                    let cells = r.display_values().into_iter().map(|(_, v)| v).collect();
                    ListRow { id, cells }
                })
                .collect())
        })
    }

    fn find_row<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<EditRow>>> + Send + 'a>> {
        Box::pin(async move {
            let found = crate::orm::find::<M>(db, id).await?;
            Ok(found.map(|m| EditRow {
                id: AdminModel::id(&m),
                values: m.display_values(),
            }))
        })
    }

    fn create<'a>(&'a self, db: &'a Db, form: &'a FormData) -> CreateResult<'a> {
        Box::pin(async move {
            match M::from_form(form) {
                Ok(model) => match crate::orm::create(db, &model).await {
                    Ok(id) => Ok(Ok(id)),
                    // Postgres constraint violations route to
                    // `Error::Conflict` via `From<sqlx::Error>`. Catch
                    // them here so the user sees a re-rendered form
                    // with an inline error instead of a 500.
                    Err(crate::error::Error::Conflict(msg)) => {
                        log::warn!("create rejected by DB constraint: {msg}");
                        Ok(Err(vec!["Invalid value or constraint violation. \
                             Please check the highlighted fields and try again."
                            .into()]))
                    }
                    Err(other) => Err(other),
                },
                Err(errs) => Ok(Err(errs)),
            }
        })
    }

    fn update<'a>(&'a self, db: &'a Db, id: i64, form: &'a FormData) -> UpdateResult<'a> {
        Box::pin(async move {
            match M::from_form(form) {
                Ok(model) => match crate::orm::update(db, id, &model).await {
                    Ok(()) => Ok(Ok(())),
                    Err(crate::error::Error::Conflict(msg)) => {
                        log::warn!("update rejected by DB constraint: {msg}");
                        Ok(Err(vec!["Invalid value or constraint violation. \
                             Please check the highlighted fields and try again."
                            .into()]))
                    }
                    Err(other) => Err(other),
                },
                Err(errs) => Ok(Err(errs)),
            }
        })
    }

    fn delete<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move { crate::orm::delete::<M>(db, id).await })
    }

    fn object_label<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'a>> {
        Box::pin(async move {
            let found = crate::orm::find::<M>(db, id).await?;
            Ok(found.map(|m| m.object_label()))
        })
    }
}
