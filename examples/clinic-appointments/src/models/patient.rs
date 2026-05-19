use chrono::{DateTime, Utc};

use rustio_admin::{Inline, Model, ModelAdmin, Result, Row, RustioAdmin, Value};

#[derive(Debug, Clone, RustioAdmin)]
pub struct Patient {
    pub id: i64,
    pub chart_number: String,
    pub full_name: String,
    pub email: String,
    pub is_active: bool,
    pub registered_at: DateTime<Utc>,
    /// Optional patient photo. `#[rustio(file)]` promotes the
    /// column to the framework's file-upload widget: form renders
    /// `<input type="file">`, the multipart-form handler writes
    /// the uploaded bytes under `Admin::uploads_dir`, and stores
    /// the resulting relative path here.
    #[rustio(file)]
    pub photo_path: Option<String>,
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
        "photo_path",
    ];
    const INSERT_COLUMNS: &'static [&'static str] = &[
        "chart_number",
        "full_name",
        "email",
        "is_active",
        "registered_at",
        "photo_path",
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
            photo_path: row.get_optional_string("photo_path")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            Value::from(self.chart_number.clone()),
            Value::from(self.full_name.clone()),
            Value::from(self.email.clone()),
            Value::from(self.is_active),
            Value::from(self.registered_at),
            Value::from(self.photo_path.clone()),
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

    fn inlines() -> &'static [Inline] {
        // The patient's appointments live one click away on the
        // child model. Surfacing them on the patient edit page
        // saves a navigation hop and gives operators "what does
        // this patient have booked" context without leaving.
        &[Inline {
            target_model: "Appointment",
            fk_field: "patient_id",
            label: Some("Appointments"),
            max_rows: 50,
            // Appointments don't have a natural-name column;
            // surface the scheduled time so operators can scan
            // the patient's visit history at a glance.
            display_field: Some("scheduled_at"),
        }]
    }
}
