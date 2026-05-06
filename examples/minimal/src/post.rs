//! Demo `Post` model. Hand-written `impl Model`; the admin metadata
//! comes from `#[derive(RustioAdmin)]`. P5 only exercises that the
//! derive expansion compiles — the model is not yet registered with
//! an `Admin` (P6) or wired into the router.

use chrono::{DateTime, Utc};
use rustio_admin::orm::{Model, Row, Value};
use rustio_admin::{ModelAdmin, RustioAdmin};

#[derive(RustioAdmin)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub published: bool,
    pub created_at: DateTime<Utc>,
}

// Empty `impl ModelAdmin` opts the model into `Admin::model::<Post>()`
// and inherits every framework default (id-DESC ordering, 50-row
// pagination, no list-filter, no search). Override individual methods
// to customise — see `rustio_admin::ModelAdmin` doc comments.
impl ModelAdmin for Post {}

impl Model for Post {
    const TABLE: &'static str = "posts";
    const COLUMNS: &'static [&'static str] = &["id", "title", "body", "published", "created_at"];
    const INSERT_COLUMNS: &'static [&'static str] = &["title", "body", "published", "created_at"];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> rustio_admin::Result<Self> {
        Ok(Self {
            id: row.get_i64("id")?,
            title: row.get_string("title")?,
            body: row.get_string("body")?,
            published: row.get_bool("published")?,
            created_at: row.get_datetime("created_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            Value::Text(self.title.clone()),
            Value::Text(self.body.clone()),
            Value::Bool(self.published),
            Value::DateTime(self.created_at),
        ]
    }
}
