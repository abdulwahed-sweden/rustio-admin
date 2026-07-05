//! Task model — a translation job routed to a translator.
//!
//! Two field types do the interesting work: `status` is a fixed set
//! (`#[rustio(choices = …)]` renders a dropdown; the migration's `CHECK`
//! enforces it in Postgres) and `translator_id` is a foreign key that the
//! admin renders as a link to the translator.

use rustio_admin::{FieldValidationError, ModelAdmin, NaiveDate, RustioAdmin};

#[derive(RustioAdmin)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub source_lang: String,
    pub target_lang: String,
    pub word_count: i32,
    pub deadline: NaiveDate,
    pub translator_id: i64,
    #[rustio(choices = ["available", "in_progress", "review", "completed", "reassigned"])]
    pub status: String,
}

// Admin list-page configuration. Each method overrides a default.
impl ModelAdmin for Task {
    fn list_display() -> &'static [&'static str] {
        &[
            "title",
            "target_lang",
            "word_count",
            "deadline",
            "translator_id",
            "status",
        ]
    }

    fn list_filter() -> &'static [&'static str] {
        &["status", "target_lang"]
    }

    fn search_fields() -> &'static [&'static str] {
        &["title", "source_lang", "target_lang"]
    }

    fn ordering() -> &'static [&'static str] {
        &["-id"]
    }

    // A domain rule — **your** code, not the framework's. RustIO refuses
    // the unauthorised actor; you refuse the wrong data. A translation must
    // have a positive word count; the language-match / availability rule
    // ("this translator can't do Arabic") would live here too. See
    // docs/security.md — "What this does not do" — for the boundary.
    fn validate(task: &Self) -> std::result::Result<(), Vec<FieldValidationError>> {
        if task.word_count <= 0 {
            return Err(vec![FieldValidationError::field(
                "word_count",
                "Word count must be a positive number.",
            )]);
        }
        Ok(())
    }
}
