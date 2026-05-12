use chrono::{DateTime, Utc};

use rustio_admin::{Model, ModelAdmin, Result, Row, RustioAdmin, Value};

#[derive(Debug, Clone, RustioAdmin)]
pub struct Item {
    pub id: i64,
    #[rustio(belongs_to = "Branch", display = "name")]
    pub branch_id: i64,
    pub title: String,
    pub kind: String,
    pub isbn: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

// Manual Model impl kept explicit for teaching/readability.
impl Model for Item {
    const TABLE: &'static str = "items";
    const COLUMNS: &'static [&'static str] = &[
        "id",
        "branch_id",
        "title",
        "kind",
        "isbn",
        "status",
        "created_at",
    ];
    const INSERT_COLUMNS: &'static [&'static str] =
        &["branch_id", "title", "kind", "isbn", "status", "created_at"];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self> {
        Ok(Item {
            id: row.get_i64("id")?,
            branch_id: row.get_i64("branch_id")?,
            title: row.get_string("title")?,
            kind: row.get_string("kind")?,
            isbn: row.get_optional_string("isbn")?,
            status: row.get_string("status")?,
            created_at: row.get_datetime("created_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            Value::from(self.branch_id),
            Value::from(self.title.clone()),
            Value::from(self.kind.clone()),
            Value::from(self.isbn.clone()),
            Value::from(self.status.clone()),
            Value::from(self.created_at),
        ]
    }
}

impl ModelAdmin for Item {}
