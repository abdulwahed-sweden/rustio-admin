use chrono::{DateTime, Utc};

use rustio_admin::{Model, ModelAdmin, Result, Row, RustioAdmin, Value};

#[derive(Debug, Clone, RustioAdmin)]
pub struct Loan {
    pub id: i64,
    #[rustio(belongs_to = "Patron", display = "full_name")]
    pub patron_id: i64,
    #[rustio(belongs_to = "Item", display = "title")]
    pub item_id: i64,
    pub status: String,
    pub borrowed_at: DateTime<Utc>,
    pub due_at: DateTime<Utc>,
    pub returned_at: Option<DateTime<Utc>>,
}

// Manual Model impl kept explicit for teaching/readability.
impl Model for Loan {
    const TABLE: &'static str = "loans";
    const COLUMNS: &'static [&'static str] = &[
        "id",
        "patron_id",
        "item_id",
        "status",
        "borrowed_at",
        "due_at",
        "returned_at",
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        "patron_id",
        "item_id",
        "status",
        "borrowed_at",
        "due_at",
        "returned_at",
    ];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self> {
        Ok(Loan {
            id: row.get_i64("id")?,
            patron_id: row.get_i64("patron_id")?,
            item_id: row.get_i64("item_id")?,
            status: row.get_string("status")?,
            borrowed_at: row.get_datetime("borrowed_at")?,
            due_at: row.get_datetime("due_at")?,
            returned_at: row.get_optional_datetime("returned_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            Value::from(self.patron_id),
            Value::from(self.item_id),
            Value::from(self.status.clone()),
            Value::from(self.borrowed_at),
            Value::from(self.due_at),
            Value::from(self.returned_at),
        ]
    }
}

impl ModelAdmin for Loan {}
