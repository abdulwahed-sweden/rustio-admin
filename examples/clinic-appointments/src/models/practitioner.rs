use chrono::{DateTime, Utc};

use rustio_admin::{Model, ModelAdmin, Result, Row, RustioAdmin, Value};

#[derive(Debug, Clone, RustioAdmin)]
pub struct Practitioner {
    pub id: i64,
    #[rustio(belongs_to = "Clinic", display = "name")]
    pub clinic_id: i64,
    pub full_name: String,
    pub specialty: String,
    pub license_no: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

// Manual Model impl kept explicit for teaching/readability.
impl Model for Practitioner {
    const TABLE: &'static str = "practitioners";
    const COLUMNS: &'static [&'static str] = &[
        "id",
        "clinic_id",
        "full_name",
        "specialty",
        "license_no",
        "status",
        "created_at",
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        "clinic_id",
        "full_name",
        "specialty",
        "license_no",
        "status",
        "created_at",
    ];

    fn id(&self) -> i64 {
        self.id
    }

    fn from_row(row: Row<'_>) -> Result<Self> {
        Ok(Practitioner {
            id: row.get_i64("id")?,
            clinic_id: row.get_i64("clinic_id")?,
            full_name: row.get_string("full_name")?,
            specialty: row.get_string("specialty")?,
            license_no: row.get_optional_string("license_no")?,
            status: row.get_string("status")?,
            created_at: row.get_datetime("created_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            Value::from(self.clinic_id),
            Value::from(self.full_name.clone()),
            Value::from(self.specialty.clone()),
            Value::from(self.license_no.clone()),
            Value::from(self.status.clone()),
            Value::from(self.created_at),
        ]
    }
}

impl ModelAdmin for Practitioner {}
