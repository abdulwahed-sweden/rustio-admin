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

// `ModelAdmin` is opt-in per type — an empty impl picks up every
// framework default. The values below override the four most common
// hooks so the demo's list page exercises filter chips, the search
// box, custom column order, and the default sort.
impl ModelAdmin for Post {
    fn list_display() -> &'static [&'static str] {
        &["title", "published", "created_at"]
    }
    fn list_filter() -> &'static [&'static str] {
        &["published"]
    }
    fn search_fields() -> &'static [&'static str] {
        &["title", "body"]
    }
    fn ordering() -> &'static [&'static str] {
        &["-created_at"]
    }
}

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
