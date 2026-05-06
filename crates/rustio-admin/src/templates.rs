//! Template rendering. Rust code passes typed context; this module
//! owns everything about HTML generation.
//!
//! # Loader contract
//!
//! Per-request lookup via [`minijinja::Environment::set_loader`]. On
//! every `render` call the cache is cleared, forcing the loader closure
//! to re-resolve from disk so a developer can edit a template under
//! `RUSTIO_TEMPLATE_DIR` and see the change on the next request without
//! restarting the process.
//!
//! Lookup order, by template name `<path>`:
//!
//! 1. `<RUSTIO_TEMPLATE_DIR>/<path>` — project disk override.
//! 2. Embedded default — compiled into the binary via `include_str!`.
//!
//! Per-model lookup hook: callers that pass a model context can use
//! [`Templates::render_for_model`] to add a third tier:
//!
//! 1. `<RUSTIO_TEMPLATE_DIR>/admin/<model>/<page>.html`
//! 2. `<RUSTIO_TEMPLATE_DIR>/<path>`
//! 3. Embedded default
//!
//! Phase 2 bootstrap: the embedded list is empty. P7 populates it with
//! the rewritten admin templates.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use minijinja::{Environment, ErrorKind};
use serde::Serialize;

use crate::error::{Error, Result};

pub struct Templates {
    env: Mutex<Environment<'static>>,
}

impl Templates {
    /// Build the environment.
    ///
    /// `project_templates_dir = None` → embedded templates only.
    /// `project_templates_dir = Some(path)` → disk overrides win at
    /// render time. Pass the value of `RUSTIO_TEMPLATE_DIR` (or your
    /// own resolved path) here.
    ///
    /// When a disk root is supplied, the constructor scans it once for
    /// overrides of embedded templates. Each match is logged at INFO;
    /// an override that looks structurally incomplete (no
    /// `{% extends %}`, no `{% block %}`, no `<html>` tag) is logged at
    /// WARN so a one-line stub of an admin template stops being a
    /// silent failure. Non-fatal: the override is still served — the
    /// scan exists only to make the failure mode visible.
    pub fn new(project_templates_dir: Option<PathBuf>) -> Result<Arc<Self>> {
        let disk_root = project_templates_dir;
        if let Some(root) = disk_root.as_deref() {
            for v in validate_overrides(root) {
                match v {
                    OverrideValidation::Loaded { name, bytes } => {
                        log::info!(
                            "templates: project override loaded for `{name}` ({bytes} bytes)"
                        );
                    }
                    OverrideValidation::Suspicious { name, bytes } => {
                        log::warn!(
                            "templates: project override for `{name}` looks incomplete \
                             ({bytes} bytes, no `{{% extends %}}`, no `{{% block %}}`, no \
                             `<html>` tag) — the admin UI may render incorrectly. Either \
                             copy the framework default in full or remove the override."
                        );
                    }
                    OverrideValidation::Unreadable { name, error } => {
                        log::warn!(
                            "templates: project override `{name}` exists but cannot be read: {error}"
                        );
                    }
                    OverrideValidation::OrphanAdminFile { path } => {
                        log::warn!(
                            "templates: `{path}` is in the admin namespace but does not \
                             override any embedded template (typo? framework default \
                             will be served unchanged). Project-specific admin pages \
                             belong outside `templates/admin/`."
                        );
                    }
                }
            }
        }
        let mut env = Environment::new();
        env.set_loader(move |name| load_template(disk_root.as_deref(), name));

        Ok(Arc::new(Self {
            env: Mutex::new(env),
        }))
    }

    /// Render a template by name.
    pub fn render<S: Serialize>(&self, name: &str, ctx: &S) -> Result<String> {
        let mut env = self
            .env
            .lock()
            .map_err(|e| Error::Internal(format!("template env poisoned: {e}")))?;
        // Clear cache so the loader runs again — restart-free dev edits.
        env.clear_templates();
        let tmpl = env
            .get_template(name)
            .map_err(|e| Error::Internal(format!("template {name} not found: {e}")))?;
        tmpl.render(ctx)
            .map_err(|e| Error::Internal(format!("render {name}: {e}")))
    }

    /// Render with a per-model override hook.
    ///
    /// Tries `admin/<model>/<page>` first (where `<page>` is `name`
    /// stripped of any leading `admin/`), falling back to `name`.
    #[allow(dead_code)]
    pub fn render_for_model<S: Serialize>(
        &self,
        model: &str,
        name: &str,
        ctx: &S,
    ) -> Result<String> {
        let page = name.strip_prefix("admin/").unwrap_or(name);
        let per_model = format!("admin/{model}/{page}");
        let mut env = self
            .env
            .lock()
            .map_err(|e| Error::Internal(format!("template env poisoned: {e}")))?;
        env.clear_templates();
        if let Ok(tmpl) = env.get_template(&per_model) {
            return tmpl
                .render(ctx)
                .map_err(|e| Error::Internal(format!("render {per_model}: {e}")));
        }
        let tmpl = env
            .get_template(name)
            .map_err(|e| Error::Internal(format!("template {name} not found: {e}")))?;
        tmpl.render(ctx)
            .map_err(|e| Error::Internal(format!("render {name}: {e}")))
    }
}

/// Outcome of inspecting one project override file at startup.
/// Per-file, not per-render: the cost is paid once when the `Templates`
/// arc is built, not on every request.
///
/// Pure data — `Templates::new` translates each variant to a log line.
/// Returned as a `Vec` so unit tests can assert on the structural
/// classification without scraping log output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OverrideValidation {
    /// File loaded and contains at least one of `{% extends %}`,
    /// `{% block %}`, or `<html`.
    Loaded { name: &'static str, bytes: usize },
    /// File loaded but contains none of the structural markers.
    Suspicious { name: &'static str, bytes: usize },
    /// File exists on disk but `read_to_string` failed.
    Unreadable { name: &'static str, error: String },
    /// File in `templates/admin/` whose name does NOT match any
    /// embedded template — usually a typo or a misplaced project
    /// admin page.
    OrphanAdminFile { path: String },
}

/// Walk `EMBEDDED_TEMPLATES`, classify any project override of one of
/// those names, return the per-file results.
///
/// Files in `disk_root` that do NOT shadow an embedded name are
/// ignored: those are project-only templates and have no framework
/// default to compare against.
pub(crate) fn validate_overrides(disk_root: &std::path::Path) -> Vec<OverrideValidation> {
    let mut results = Vec::new();
    for (name, _embedded) in EMBEDDED_TEMPLATES {
        let path = disk_root.join(name);
        if !path.is_file() {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(body) => {
                let bytes = body.len();
                let has_structure = body.contains("{% extends")
                    || body.contains("{% block")
                    || body.contains("<html");
                if has_structure {
                    results.push(OverrideValidation::Loaded { name, bytes });
                } else {
                    results.push(OverrideValidation::Suspicious { name, bytes });
                }
            }
            Err(e) => {
                results.push(OverrideValidation::Unreadable {
                    name,
                    error: e.to_string(),
                });
            }
        }
    }

    // Orphan-admin-file scan. The framework reserves `templates/admin/`
    // for overrides of embedded admin templates. A file in that
    // namespace whose name doesn't match any embedded template
    // overrides nothing — usually a typo or misunderstanding. Either
    // way the developer's intent and the runtime's behaviour disagree
    // silently; this scan logs a WARN so the disagreement becomes
    // observable.
    let admin_dir = disk_root.join("admin");
    if admin_dir.is_dir() {
        let known: std::collections::HashSet<&'static str> = EMBEDDED_TEMPLATES
            .iter()
            .filter_map(|(n, _)| n.strip_prefix("admin/"))
            .collect();
        if let Ok(entries) = std::fs::read_dir(&admin_dir) {
            // Sort for deterministic ordering — the loop visits files in
            // arbitrary FS order otherwise, which makes log lines and
            // tests non-deterministic.
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.eq_ignore_ascii_case("html"))
                        .unwrap_or(false)
                })
                .collect();
            files.sort_by_key(|e| e.file_name());
            for entry in files {
                let file_name = entry.file_name();
                let Some(stem_html) = file_name.to_str() else {
                    continue;
                };
                if known.contains(stem_html) {
                    continue;
                }
                results.push(OverrideValidation::OrphanAdminFile {
                    path: format!("admin/{stem_html}"),
                });
            }
        }
    }

    results
}

fn load_template(
    disk_root: Option<&std::path::Path>,
    name: &str,
) -> std::result::Result<Option<String>, minijinja::Error> {
    if let Some(root) = disk_root {
        let path = root.join(name);
        if path.exists() {
            return std::fs::read_to_string(&path).map(Some).map_err(|e| {
                minijinja::Error::new(
                    ErrorKind::InvalidOperation,
                    format!("read template {}: {e}", path.display()),
                )
            });
        }
    }
    Ok(EMBEDDED_TEMPLATES.iter().find_map(|(n, b)| {
        if *n == name {
            Some((*b).to_string())
        } else {
            None
        }
    }))
}

/// Baked into the binary. Single-binary deploy is a hard constraint.
/// P2 ships an empty list; P7 populates it with the rewritten admin
/// templates.
const EMBEDDED_TEMPLATES: &[(&str, &str)] = &[];

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use std::io::Write;

    #[derive(Serialize)]
    struct Empty {}

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rustio-admin-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_template_errors_cleanly() {
        let t = Templates::new(None).unwrap();
        let err = t.render("does/not/exist.html", &Empty {}).unwrap_err();
        assert_eq!(err.status(), 500);
    }

    #[test]
    fn disk_loader_finds_project_template() {
        let dir = tempdir();
        let mut f = std::fs::File::create(dir.join("hello.html")).unwrap();
        f.write_all(b"hi from disk").unwrap();
        drop(f);

        let t = Templates::new(Some(dir.clone())).unwrap();
        let body = t.render("hello.html", &Empty {}).unwrap();
        assert_eq!(body, "hi from disk");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
