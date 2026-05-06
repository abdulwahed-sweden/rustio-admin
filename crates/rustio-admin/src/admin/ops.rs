//! `ConcreteOps<M>` — the manual runtime that drives every project
//! model registered via `Admin::model::<M>()`.
//!
//! Every framework-level read or write goes through one of the
//! [`AdminOps`](super::types::AdminOps) methods, which in turn calls
//! the matching free function in `crate::orm`. The trait stays
//! `pub(crate)` because handlers route through `AdminEntry::ops`
//! directly; consumers never name `ConcreteOps`.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use crate::error::Result;
use crate::http::FormData;
use crate::orm::{Db, Row};

use super::types::{AdminModel, AdminOps, CreateResult, EditRow, ListOpts, ListRow, UpdateResult};

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
        opts: ListOpts,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ListRow>>> + Send + 'a>> {
        Box::pin(async move {
            // Defense-in-depth: drop any ordering whose column is not
            // declared on `M::COLUMNS`. The handler's URL-param parser
            // already validates, but a stale `ModelAdmin::ordering()`
            // string typo would otherwise reach the SQL planner as a
            // raw identifier (the strings are interpolated, not bound).
            let valid: HashSet<&str> = M::COLUMNS.iter().copied().collect();
            let mut order_parts: Vec<String> = Vec::with_capacity(opts.ordering.len());
            for (col, dir) in &opts.ordering {
                if valid.contains(col.as_str()) {
                    order_parts.push(format!("{} {}", col, dir.sql()));
                }
            }
            let order_clause = if order_parts.is_empty() {
                "id DESC".to_string()
            } else {
                order_parts.join(", ")
            };

            let sql = format!(
                "SELECT {} FROM {} ORDER BY {}",
                M::COLUMNS.join(", "),
                M::TABLE,
                order_clause,
            );
            let rows = sqlx::query(&sql).fetch_all(db.pool()).await?;
            let models: Result<Vec<M>> =
                rows.iter().map(|r| M::from_row(Row::from_pg(r))).collect();
            let models = models?;
            Ok(models
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
