use chrono::{DateTime, Utc};

use rustio_admin::{Model, ModelAdmin, Result, Row, RustioAdmin, Value};

#[derive(Debug, Clone, RustioAdmin)]
pub struct Clinic {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub is_open: bool,
    pub created_at: DateTime<Utc>,
}

// Manual Model impl kept explicit for teaching/readability.
impl Model for Clinic {
    const TABLE: &'static str = "clinics";
    const COLUMNS: &'static [&'static str] = &["id", "name", "address", "is_open", "created_at"];
    const INSERT_COLUMNS: &'static [&'static str] = &["name", "address", "is_open", "created_at"];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self> {
        Ok(Clinic {
            id: row.get_i64("id")?,
            name: row.get_string("name")?,
            address: row.get_string("address")?,
            is_open: row.get_bool("is_open")?,
            created_at: row.get_datetime("created_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            Value::from(self.name.clone()),
            Value::from(self.address.clone()),
            Value::from(self.is_open),
            Value::from(self.created_at),
        ]
    }
}

impl ModelAdmin for Clinic {}
