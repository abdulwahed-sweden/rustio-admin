use chrono::{DateTime, Utc};

use rustio_admin::{Model, ModelAdmin, Result, Row, RustioAdmin, Value};

#[derive(Debug, Clone, RustioAdmin)]
pub struct Patron {
    pub id: i64,
    pub card_number: String,
    pub full_name: String,
    pub email: String,
    pub is_active: bool,
    pub joined_at: DateTime<Utc>,
}

// Manual Model impl kept explicit for teaching/readability.
impl Model for Patron {
    const TABLE: &'static str = "patrons";
    const COLUMNS: &'static [&'static str] = &[
        "id",
        "card_number",
        "full_name",
        "email",
        "is_active",
        "joined_at",
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        "card_number",
        "full_name",
        "email",
        "is_active",
        "joined_at",
    ];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self> {
        Ok(Patron {
            id: row.get_i64("id")?,
            card_number: row.get_string("card_number")?,
            full_name: row.get_string("full_name")?,
            email: row.get_string("email")?,
            is_active: row.get_bool("is_active")?,
            joined_at: row.get_datetime("joined_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            Value::from(self.card_number.clone()),
            Value::from(self.full_name.clone()),
            Value::from(self.email.clone()),
            Value::from(self.is_active),
            Value::from(self.joined_at),
        ]
    }
}

impl ModelAdmin for Patron {}
