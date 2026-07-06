//! Embedded admin templates for `rustio-admin`.
//!
//! This crate is the single owner of the admin panel's template bytes.
//! They are baked into the binary via `include_str!` at compile time —
//! single-binary deploy is a hard constraint of the framework.
//!
//! Two crates consume this:
//!
//! * `rustio-admin` (the runtime) reads [`EMBEDDED_TEMPLATES`] to build
//!   its minijinja loader and to validate project overrides.
//! * `rustio-admin-cli` reads [`embedded_template_names`] /
//!   [`embedded_template_source`] for the `rustio-admin override`
//!   command, which materialises a copy of a framework template on disk.
//!
//! The crate is deliberately std-only and dependency-free. It carries a
//! low MSRV so the lightweight parts of the CLI can depend on it without
//! pulling in — or inheriting the Rust version of — the runtime stack
//! (`sqlx`, `tokio`, `hyper`). A disk-side override still wins at render
//! time; that lookup lives in the runtime, not here. This crate only
//! owns the *defaults*.

/// Every template baked into the framework binary, by canonical name
/// (e.g. `"admin/list.html"`). Order is stable across builds so a CLI
/// verb that lists them produces deterministic output. The runtime
/// iterates this to build its loader and to validate project overrides;
/// [`embedded_template_names`] and [`embedded_template_source`] are the
/// name/lookup views over it.
pub const EMBEDDED_TEMPLATES: &[(&str, &str)] = &[
    // Shell + partials
    (
        "admin/_base.html",
        include_str!("../assets/templates/admin/_base.html"),
    ),
    (
        "admin/_topbar.html",
        include_str!("../assets/templates/admin/_topbar.html"),
    ),
    (
        "admin/_sidebar.html",
        include_str!("../assets/templates/admin/_sidebar.html"),
    ),
    (
        "admin/_theme.html",
        include_str!("../assets/templates/admin/_theme.html"),
    ),
    (
        "admin/_row_actions.html",
        include_str!("../assets/templates/admin/_row_actions.html"),
    ),
    (
        "admin/includes/_form_field.html",
        include_str!("../assets/templates/admin/includes/_form_field.html"),
    ),
    (
        "admin/includes/_field_errors.html",
        include_str!("../assets/templates/admin/includes/_field_errors.html"),
    ),
    // Generic pages
    (
        "admin/login.html",
        include_str!("../assets/templates/admin/login.html"),
    ),
    (
        "admin/index.html",
        include_str!("../assets/templates/admin/index.html"),
    ),
    (
        "admin/list.html",
        include_str!("../assets/templates/admin/list.html"),
    ),
    (
        "admin/form.html",
        include_str!("../assets/templates/admin/form.html"),
    ),
    (
        "admin/confirm_delete.html",
        include_str!("../assets/templates/admin/confirm_delete.html"),
    ),
    (
        "admin/bulk_confirm_delete.html",
        include_str!("../assets/templates/admin/bulk_confirm_delete.html"),
    ),
    (
        "admin/db_browser.html",
        include_str!("../assets/templates/admin/db_browser.html"),
    ),
    (
        "admin/bulk_confirm_action.html",
        include_str!("../assets/templates/admin/bulk_confirm_action.html"),
    ),
    (
        "admin/error.html",
        include_str!("../assets/templates/admin/error.html"),
    ),
    (
        "admin/forbidden.html",
        include_str!("../assets/templates/admin/forbidden.html"),
    ),
    // Audit / password change
    (
        "admin/object_history.html",
        include_str!("../assets/templates/admin/object_history.html"),
    ),
    (
        "admin/log_entries.html",
        include_str!("../assets/templates/admin/log_entries.html"),
    ),
    (
        "admin/apis_index.html",
        include_str!("../assets/templates/admin/apis_index.html"),
    ),
    (
        "admin/apis_playground.html",
        include_str!("../assets/templates/admin/apis_playground.html"),
    ),
    (
        "admin/health.html",
        include_str!("../assets/templates/admin/health.html"),
    ),
    (
        "admin/feature_flags.html",
        include_str!("../assets/templates/admin/feature_flags.html"),
    ),
    (
        "admin/_list_adaptive.html",
        include_str!("../assets/templates/admin/_list_adaptive.html"),
    ),
    (
        "admin/view_designer.html",
        include_str!("../assets/templates/admin/view_designer.html"),
    ),
    (
        "admin/view_designer_model.html",
        include_str!("../assets/templates/admin/view_designer_model.html"),
    ),
    (
        "admin/branding.html",
        include_str!("../assets/templates/admin/branding.html"),
    ),
    (
        "admin/schema.html",
        include_str!("../assets/templates/admin/schema.html"),
    ),
    (
        "admin/view_layer/_cell.html",
        include_str!("../assets/templates/admin/view_layer/_cell.html"),
    ),
    (
        "admin/view_layer/_row.html",
        include_str!("../assets/templates/admin/view_layer/_row.html"),
    ),
    (
        "admin/notifications.html",
        include_str!("../assets/templates/admin/notifications.html"),
    ),
    (
        "admin/csv_import_result.html",
        include_str!("../assets/templates/admin/csv_import_result.html"),
    ),
    (
        "admin/docs_index.html",
        include_str!("../assets/templates/admin/docs_index.html"),
    ),
    (
        "admin/doc_page.html",
        include_str!("../assets/templates/admin/doc_page.html"),
    ),
    (
        "admin/password_change.html",
        include_str!("../assets/templates/admin/password_change.html"),
    ),
    // Built-in user pages
    (
        "admin/users_list.html",
        include_str!("../assets/templates/admin/users_list.html"),
    ),
    (
        "admin/user_new.html",
        include_str!("../assets/templates/admin/user_new.html"),
    ),
    (
        "admin/user_edit.html",
        include_str!("../assets/templates/admin/user_edit.html"),
    ),
    (
        "admin/user_view.html",
        include_str!("../assets/templates/admin/user_view.html"),
    ),
    (
        "admin/user_confirm_delete.html",
        include_str!("../assets/templates/admin/user_confirm_delete.html"),
    ),
    // Built-in group pages
    (
        "admin/groups_list.html",
        include_str!("../assets/templates/admin/groups_list.html"),
    ),
    (
        "admin/group_new.html",
        include_str!("../assets/templates/admin/group_new.html"),
    ),
    (
        "admin/group_edit.html",
        include_str!("../assets/templates/admin/group_edit.html"),
    ),
    (
        "admin/group_confirm_delete.html",
        include_str!("../assets/templates/admin/group_confirm_delete.html"),
    ),
    // Self-service account pages (R0+)
    (
        "admin/account_sessions.html",
        include_str!("../assets/templates/admin/account_sessions.html"),
    ),
    // Self-service password recovery (R1)
    (
        "admin/forgot_password.html",
        include_str!("../assets/templates/admin/forgot_password.html"),
    ),
    (
        "admin/forgot_password_sent.html",
        include_str!("../assets/templates/admin/forgot_password_sent.html"),
    ),
    (
        "admin/reset_password.html",
        include_str!("../assets/templates/admin/reset_password.html"),
    ),
    // Organisational recovery (R2)
    //
    // These pages are rendered by `admin/admin_recovery_handlers.rs`
    // — the admin-driven reset / lock / re-auth / forced-rotation
    // surface. They were inadvertently omitted from this list when
    // R2 shipped in 0.6.0; the disk files were committed but never
    // hooked into the embedded set. Without them every
    // `/admin/reauth`, `/admin/users/:id/reset-password`,
    // `/admin/users/:id/lock`, and forced-password-change request
    // returns the framework's generic 500 page. The
    // `every_handler_rendered_template_resolves` test in
    // `rustio-admin` is the regression gate that catches this shape
    // of omission.
    (
        "admin/reauth.html",
        include_str!("../assets/templates/admin/reauth.html"),
    ),
    (
        "admin/admin_reset_password.html",
        include_str!("../assets/templates/admin/admin_reset_password.html"),
    ),
    (
        "admin/lock_user.html",
        include_str!("../assets/templates/admin/lock_user.html"),
    ),
    (
        "admin/confirm_admin_action.html",
        include_str!("../assets/templates/admin/confirm_admin_action.html"),
    ),
    (
        "admin/must_change_password.html",
        include_str!("../assets/templates/admin/must_change_password.html"),
    ),
    // TOTP MFA (R3)
    //
    // Rendered by `admin/mfa_handlers.rs` (enrol / verify /
    // regenerate / disable). Same shape of omission as the R2 set
    // above; same regression-gate test covers them.
    (
        "admin/mfa_enroll.html",
        include_str!("../assets/templates/admin/mfa_enroll.html"),
    ),
    (
        "admin/mfa_enroll_complete.html",
        include_str!("../assets/templates/admin/mfa_enroll_complete.html"),
    ),
    (
        "admin/mfa_verify.html",
        include_str!("../assets/templates/admin/mfa_verify.html"),
    ),
    (
        "admin/mfa_disable.html",
        include_str!("../assets/templates/admin/mfa_disable.html"),
    ),
    (
        "admin/mfa_regenerate.html",
        include_str!("../assets/templates/admin/mfa_regenerate.html"),
    ),
    (
        "admin/mfa_regenerate_complete.html",
        include_str!("../assets/templates/admin/mfa_regenerate_complete.html"),
    ),
];

/// Every embedded template name, by canonical path (e.g.
/// `"admin/list.html"`). Order matches [`EMBEDDED_TEMPLATES`] and is
/// stable across builds so `rustio-admin override` enumerates copy
/// candidates deterministically; project code can also iterate this to
/// build documentation pages.
pub fn embedded_template_names() -> Vec<&'static str> {
    EMBEDDED_TEMPLATES.iter().map(|(n, _)| *n).collect()
}

/// Return the byte-for-byte source of an embedded template by name, or
/// `None` when no such template exists. Used by `rustio-admin override`
/// to materialise a copy at `<RUSTIO_TEMPLATE_DIR>/<name>` so the
/// operator can start editing without first having to find the
/// framework source.
pub fn embedded_template_source(name: &str) -> Option<&'static str> {
    EMBEDDED_TEMPLATES
        .iter()
        .find_map(|(n, body)| if *n == name { Some(*body) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table is non-empty and every entry actually baked in bytes.
    /// Catches a `git mv` of the asset tree that leaves a dangling
    /// `include_str!` — that would be a compile error, but this also
    /// guards against an entry being silently emptied.
    #[test]
    fn every_embedded_template_has_bytes() {
        assert!(!EMBEDDED_TEMPLATES.is_empty());
        for (name, body) in EMBEDDED_TEMPLATES {
            assert!(!body.is_empty(), "{name} is empty");
        }
    }

    /// The name view and the lookup view agree with the table.
    #[test]
    fn accessors_round_trip() {
        for name in embedded_template_names() {
            assert!(
                embedded_template_source(name).is_some(),
                "{name} listed but not resolvable"
            );
        }
        assert!(embedded_template_source("admin/does-not-exist.html").is_none());
    }
}
