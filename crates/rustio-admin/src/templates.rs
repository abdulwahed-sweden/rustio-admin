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

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use minijinja::{Environment, ErrorKind};
use serde::Serialize;

use crate::error::{Error, Result};

// The embedded template bytes live in the std-only `rustio-admin-assets`
// leaf crate so the CLI can reach them without linking this runtime.
// This module owns the *lookup* (disk override → embedded default); the
// table of defaults is imported here.
use rustio_admin_assets::EMBEDDED_TEMPLATES;

// public:
/// Re-exported from `rustio-admin-assets` so the framework's public API
/// (`rustio_admin::embedded_template_names` /
/// `rustio_admin::embedded_template_source`, via `lib.rs`) is unchanged
/// after the assets split. See that crate for the definitions.
pub use rustio_admin_assets::{embedded_template_names, embedded_template_source};

// public:
pub struct Templates {
    env: Mutex<Environment<'static>>,
}

impl Templates {
    // public:
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

        // `icon(name, class="…")` returns inline SVG for one of the
        // lucide stroke icons baked at compile time. Templates use it
        // to render sidebar nav icons, button icons, and alert glyphs
        // without an extra HTTP round trip. See `admin/icons.rs` for
        // the catalogue.
        env.add_function("icon", |name: &str, kwargs: minijinja::value::Kwargs| {
            let class: String = kwargs.get("class").unwrap_or_default();
            kwargs.assert_all_used().ok();
            // The output is HTML — minijinja's autoescape would
            // mangle it. Wrap in `safe()` so it renders as markup.
            minijinja::value::Value::from_safe_string(crate::admin::icons::render_inline(
                name, &class,
            ))
        });

        Ok(Arc::new(Self {
            env: Mutex::new(env),
        }))
    }

    // public:
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
        tmpl.render(ctx).map_err(|e| {
            log::error!("template render failed for {name}: {e:?}");
            Error::Internal(format!("render {name}: {e}"))
        })
    }

    // public:
    /// Render with a per-model override hook.
    ///
    /// Tries `admin/<model>/<page>` first (where `<page>` is `name`
    /// stripped of any leading `admin/`), falling back to `name`.
    ///
    /// Consumed by every generic-CRUD render in `admin::handlers` so
    /// a project can drop `templates/admin/<admin_name>/list.html`,
    /// `…/form.html`, `…/confirm_delete.html`, or
    /// `…/object_history.html` to override just that one page for
    /// just that one model. The per-model file wins; absent that the
    /// loader falls back to the framework-wide override (the
    /// path-without-model-prefix), then the embedded default.
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

    /// Per-model template override: a file at
    /// `<disk_root>/admin/<model>/list.html` wins over a same-named
    /// disk override AND over the embedded default, but only for
    /// `render_for_model(model, ...)` calls. Other models still see
    /// the framework default (or whatever shadow they have).
    #[test]
    fn render_for_model_prefers_per_model_override() {
        let dir = tempdir();
        std::fs::create_dir_all(dir.join("admin/books")).unwrap();
        let mut f = std::fs::File::create(dir.join("admin/books/list.html")).unwrap();
        f.write_all(b"books-specific list").unwrap();
        drop(f);

        let t = Templates::new(Some(dir.clone())).unwrap();
        // Books model sees the override.
        let body = t
            .render_for_model("books", "admin/list.html", &Empty {})
            .unwrap();
        assert_eq!(body, "books-specific list");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A model with no per-model file falls through to the
    /// framework-default lookup chain. Other models with overrides
    /// don't bleed into this one.
    #[test]
    fn render_for_model_falls_through_to_framework_default() {
        let dir = tempdir();
        // Drop a books-only override; query for a different model.
        std::fs::create_dir_all(dir.join("admin/books")).unwrap();
        let mut f = std::fs::File::create(dir.join("admin/books/list.html")).unwrap();
        f.write_all(b"books override").unwrap();
        drop(f);
        // Drop a framework-wide override too, to confirm the
        // fall-through actually reaches it (and isn't accidentally
        // picking up the books override for the other model).
        std::fs::create_dir_all(dir.join("admin")).unwrap();
        let mut f = std::fs::File::create(dir.join("admin/list.html")).unwrap();
        f.write_all(b"framework-wide list").unwrap();
        drop(f);

        let t = Templates::new(Some(dir.clone())).unwrap();
        // "authors" has no per-model file — falls through to
        // framework-wide override.
        let body = t
            .render_for_model("authors", "admin/list.html", &Empty {})
            .unwrap();
        assert_eq!(body, "framework-wide list");
        // "books" still sees its own override.
        let body = t
            .render_for_model("books", "admin/list.html", &Empty {})
            .unwrap();
        assert_eq!(body, "books override");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every embedded template is registered. Catches typos in
    /// `EMBEDDED_TEMPLATES` (e.g. wrong path, missing entry).
    #[test]
    fn every_embedded_template_loads() {
        let t = Templates::new(None).unwrap();
        for (name, _) in EMBEDDED_TEMPLATES {
            // Render with an empty serializable; minijinja's
            // strict-undefined fails on missing variables, so most
            // pages will Err — but parsing happens before evaluation.
            // We accept any Err whose underlying minijinja error is a
            // template-evaluation problem; an Error::Internal that
            // says "template <name> not found" would mean the loader
            // failed entirely (regression).
            let result = t.render(name, &Empty {});
            if let Err(e) = result {
                let msg = e.to_string();
                assert!(!msg.contains("not found"), "{name} failed to load: {msg}");
            }
        }
    }

    /// The command rail marks exactly the current page's entry with
    /// `aria-current="page"`, driven by the `nav_active` key the page
    /// contexts set via `BaseContext::with_nav_active`. Guards against the
    /// rail silently losing its active-state highlighting again (the key
    /// was unset for a long time, so the comparison was always false).
    #[test]
    fn sidebar_marks_active_nav_item() {
        let t = Templates::new(None).unwrap();
        let render_with = |active: &str| {
            t.render(
                "admin/_sidebar.html",
                &minijinja::context! {
                    app_name => "Test Admin",
                    nav_active => active,
                    identity => minijinja::context! { is_admin => true, is_developer => true },
                    entries => vec![
                        minijinja::context! { admin_name => "customer", display_name => "Customers" },
                    ],
                },
            )
            .unwrap()
        };

        // A built-in section key (Users) highlights its own link and nothing else.
        let users = render_with("users");
        assert!(
            users.contains(r#"href="/admin/users" aria-current="page""#),
            "Users link should be active when nav_active=users"
        );
        assert_eq!(
            users.matches(r#"aria-current="page""#).count(),
            1,
            "exactly one rail item is active"
        );

        // A model's admin_name highlights that model's link.
        let model = render_with("customer");
        assert!(
            model.contains(r#"href="/admin/customer" aria-current="page""#),
            "model link should be active when nav_active matches its admin_name"
        );

        // The new developer entry highlights when active and is always present.
        let designer = render_with("view-designer");
        assert!(designer.contains(r#"href="/admin/dev/view-designer""#));
        assert!(
            designer.contains(r#"href="/admin/dev/view-designer" aria-current="page""#),
            "View designer link should be active when nav_active=view-designer"
        );

        // An unrelated key leaves the rail with no active item.
        let none = render_with("");
        assert_eq!(none.matches(r#"aria-current="page""#).count(), 0);
    }

    /// Regression gate for the 0.7.0 → 0.7.1 fix.
    ///
    /// Scans every `.rs` file under `src/admin/` for string
    /// literals of the shape `"admin/<name>.html"` and asserts each
    /// one resolves via `Templates::new(None)?`. If a handler is
    /// added that renders a new template and the author forgets to
    /// extend `EMBEDDED_TEMPLATES`, this test fails before the
    /// release ships rather than after the first user clicks the
    /// new page.
    ///
    /// The 0.6.0 R2 + 0.7.0 R3 cycles both shipped with this
    /// shape of bug — the disk template files were committed, the
    /// handlers rendered them, the bug was invisible to unit tests
    /// (no integration test boots a real HTTP stack against the
    /// affected routes), and the regression surfaced only when the
    /// flagship downstream walked the surface against a live DB.
    /// This test makes the discipline mechanical.
    #[test]
    fn every_handler_rendered_template_resolves() {
        let admin_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/admin");
        let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        // Bare-literal scan — the framework only uses
        // `"admin/<...>.html"` strings as template names, so
        // finding every literal of that shape catches the entire
        // surface without needing AST parsing or a regex
        // dependency.
        walk_rs_files(&admin_src, &mut |path: &std::path::Path| {
            let content = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            extract_template_names(&content, &mut names);
        });
        assert!(
            !names.is_empty(),
            "no template names found — scan regression?"
        );

        let t = Templates::new(None).unwrap();
        let mut missing: Vec<String> = Vec::new();
        for name in &names {
            let result = t.render(name, &Empty {});
            if let Err(e) = result {
                let msg = e.to_string();
                // `not found` is minijinja's "template not in
                // loader" error. Other errors (strict-undefined,
                // type mismatches) are tolerated — the test cares
                // only about loader resolution, not full render
                // success against an empty context.
                if msg.contains("not found") {
                    missing.push(format!("{name}: {msg}"));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "templates referenced by handlers but not in EMBEDDED_TEMPLATES:\n  {}",
            missing.join("\n  ")
        );
    }

    /// Pull every `"admin/<...>.html"` literal out of `content` and
    /// stuff it into `out`. Bare-string scan; tolerates string
    /// literals that span multiple lines because the closing
    /// `.html"` must appear before the next double-quote.
    fn extract_template_names(content: &str, out: &mut std::collections::BTreeSet<String>) {
        let needle = "\"admin/";
        let mut cursor = 0;
        while let Some(idx) = content[cursor..].find(needle) {
            let start = cursor + idx + 1; // past the opening quote
            let after = &content[start..];
            // The literal ends at the next `"`. If `.html` does
            // not appear before that quote, this isn't a template
            // reference (it could be e.g. a Permission action_name
            // that happens to start with `admin/`).
            if let Some(end_rel) = after.find('"') {
                let literal = &after[..end_rel];
                if literal.ends_with(".html") {
                    out.insert(literal.to_string());
                }
                cursor = start + end_rel + 1;
            } else {
                break;
            }
        }
    }

    /// Recursively walk every `.rs` file under `root`, calling
    /// `visit` for each. Std-only — no `walkdir` dep needed for a
    /// single test.
    fn walk_rs_files(root: &std::path::Path, visit: &mut dyn FnMut(&std::path::Path)) {
        let entries = match std::fs::read_dir(root) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                walk_rs_files(&path, visit);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                visit(&path);
            }
        }
    }
}
