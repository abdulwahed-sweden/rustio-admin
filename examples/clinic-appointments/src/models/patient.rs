use chrono::{DateTime, Utc};

use rustio_admin::{Model, ModelAdmin, Result, Row, RustioAdmin, Value};

#[derive(Debug, Clone, RustioAdmin)]
pub struct Patient {
    pub id: i64,
    pub chart_number: String,
    pub full_name: String,
    pub email: String,
    pub is_active: bool,
    pub registered_at: DateTime<Utc>,
}

// Manual Model impl kept explicit for teaching/readability.
impl Model for Patient {
    const TABLE: &'static str = "patients";
    const COLUMNS: &'static [&'static str] = &[
        "id",
        "chart_number",
        "full_name",
        "email",
        "is_active",
        "registered_at",
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        "chart_number",
        "full_name",
        "email",
        "is_active",
        "registered_at",
    ];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self> {
        Ok(Patient {
            id: row.get_i64("id")?,
            chart_number: row.get_string("chart_number")?,
            full_name: row.get_string("full_name")?,
            email: row.get_string("email")?,
            is_active: row.get_bool("is_active")?,
            registered_at: row.get_datetime("registered_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            Value::from(self.chart_number.clone()),
            Value::from(self.full_name.clone()),
            Value::from(self.email.clone()),
            Value::from(self.is_active),
            Value::from(self.registered_at),
        ]
    }
}

impl ModelAdmin for Patient {
    fn search_fields() -> &'static [&'static str] {
        // `chart_number` + `full_name` + `email` cover the three
        // ways an operator looks a patient up: by externally-known
        // identifier (chart sticker), by the person's name (the
        // common case), or by inbound-email correlation. Indexed
        // by Postgres on chart_number + email (UNIQUE); full_name
        // ILIKE is a sequential scan but the patient list stays
        // small enough that it's not a hotspot.
        &["chart_number", "full_name", "email"]
    }
}
