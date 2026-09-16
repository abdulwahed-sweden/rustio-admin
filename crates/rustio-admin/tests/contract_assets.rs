//! **Contract: the shipped template and stylesheet inventory.**
//!
//! Three promises a downstream project depends on, all checked from outside
//! the crate the way a consumer sees them:
//!
//! 1. Every embedded template name resolves to real bytes
//!    (`embedded_template_names` / `embedded_template_source` are public API,
//!    and `rustio-admin override <name>` is built on them).
//! 2. The template lookup honours `RUSTIO_TEMPLATE_DIR`, including the
//!    per-model override path, and falls back to the embedded default.
//! 3. Every `@import` in the CSS manifest resolves to a file that exists.
//!
//! **Not** duplicated here: the `@import`-order vs `ADMIN_CSS`-concat
//! lock-step. That invariant is owned by
//! `admin::routes::css_lockstep_tests::import_manifest_matches_concat_bundle`,
//! which reads both lists through `include_str!` and therefore also holds
//! inside a published crate tarball. Point 3 below is the complement it does
//! not cover: the manifest can be internally consistent and still name a
//! fragment that is not on disk.

use std::fs;
use std::path::{Path, PathBuf};

use rustio_admin::templates::Templates;
use rustio_admin::{embedded_template_names, embedded_template_source};

fn assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

// ---- 1. embedded inventory ------------------------------------------------

#[test]
fn every_embedded_template_name_resolves_to_bytes() {
    let names = embedded_template_names();
    assert!(!names.is_empty(), "the embedded table must not be empty");
    for name in names.iter() {
        let src = embedded_template_source(name)
            .unwrap_or_else(|| panic!("{name} is listed but does not resolve"));
        assert!(!src.is_empty(), "{name} resolves to empty bytes");
    }
}

#[test]
fn an_unknown_template_name_resolves_to_none() {
    assert!(embedded_template_source("admin/does-not-exist.html").is_none());
}

#[test]
fn the_embedded_table_matches_the_files_on_disk() {
    // A `git mv` of the asset tree that misses `lib.rs` is a compile error;
    // this catches the opposite drift — a file added to the tree that nobody
    // wired into the table, which would silently never ship.
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rustio-admin-assets/assets/templates");
    let mut on_disk = Vec::new();
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("readable asset dir") {
            let p = entry.expect("dir entry").path();
            if p.is_dir() {
                walk(&p, base, out);
            } else if p.extension().is_some_and(|e| e == "html") {
                out.push(
                    p.strip_prefix(base)
                        .expect("under base")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    walk(&root, &root, &mut on_disk);
    on_disk.sort();

    let mut embedded: Vec<String> = embedded_template_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    embedded.sort();

    assert_eq!(
        embedded, on_disk,
        "EMBEDDED_TEMPLATES and the template tree have drifted"
    );
}

// ---- 2. the override contract --------------------------------------------

#[test]
fn a_disk_override_wins_over_the_embedded_default() {
    let dir = tempdir();
    let page = dir.join("admin").join("login.html");
    fs::create_dir_all(page.parent().expect("parent")).expect("mkdir");
    fs::write(&page, "OVERRIDDEN").expect("write override");

    let t = Templates::new(Some(dir.clone())).expect("Templates::new");
    let out = t
        .render("admin/login.html", &serde_json::json!({}))
        .expect("render");
    assert_eq!(out.trim(), "OVERRIDDEN");
}

#[test]
fn a_per_model_override_wins_over_the_generic_page() {
    let dir = tempdir();
    let generic = dir.join("admin").join("list.html");
    let per_model = dir.join("admin").join("widgets").join("list.html");
    fs::create_dir_all(generic.parent().expect("parent")).expect("mkdir");
    fs::create_dir_all(per_model.parent().expect("parent")).expect("mkdir");
    fs::write(&generic, "GENERIC").expect("write");
    fs::write(&per_model, "PER-MODEL").expect("write");

    let t = Templates::new(Some(dir.clone())).expect("Templates::new");
    let out = t
        .render_for_model("widgets", "admin/list.html", &serde_json::json!({}))
        .expect("render");
    assert_eq!(out.trim(), "PER-MODEL");
}

#[test]
fn a_model_without_an_override_falls_back_to_the_framework_default() {
    let dir = tempdir();
    let generic = dir.join("admin").join("list.html");
    fs::create_dir_all(generic.parent().expect("parent")).expect("mkdir");
    fs::write(&generic, "GENERIC").expect("write");

    let t = Templates::new(Some(dir)).expect("Templates::new");
    let out = t
        .render_for_model("widgets", "admin/list.html", &serde_json::json!({}))
        .expect("render");
    assert_eq!(out.trim(), "GENERIC");
}

#[test]
fn with_no_override_dir_the_embedded_templates_still_load() {
    let t = Templates::new(None).expect("Templates::new");
    // `error.html` renders against an empty context without needing a request.
    t.render("admin/error.html", &serde_json::json!({}))
        .expect("embedded default must render");
}

// ---- 3. the CSS manifest --------------------------------------------------

#[test]
fn every_css_import_resolves_to_a_file_on_disk() {
    let manifest_path = assets_root().join("static/admin/admin.css");
    let manifest = fs::read_to_string(&manifest_path).expect("admin.css is readable");
    let dir = manifest_path.parent().expect("parent");

    let mut checked = 0;
    for line in manifest.lines() {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix("@import url(\"") else {
            continue;
        };
        let frag = &rest[..rest.find("\")").expect("terminated @import")];
        let path = dir.join(frag);
        assert!(
            path.is_file(),
            "admin.css imports {frag}, which does not exist at {}",
            path.display()
        );
        checked += 1;
    }
    assert!(checked > 0, "no @import found — did the manifest move?");
}

/// **Contract: directional icons mirror exactly once in RTL.**
///
/// `admin::icons` stamps `rio-icon--directional` onto arrows and
/// `log-out` but not onto symbol glyphs. That marker is inert unless the
/// stylesheet acts on it — which is how the Activity pagination shipped
/// with arrows that never flipped. Two halves, asserted together:
/// the rule exists, and it is the *only* icon mirror in the bundle, so a
/// marked icon cannot be flipped twice back to its original hand.
#[test]
fn rtl_flips_directional_icons_exactly_once() {
    let dir = assets_root().join("static/admin");
    let manifest = fs::read_to_string(dir.join("admin.css")).expect("admin.css is readable");
    let mut bundle = String::new();
    for line in manifest.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("@import url(\"") {
            let frag = &rest[..rest.find("\")").expect("terminated @import")];
            bundle.push_str(&fs::read_to_string(dir.join(frag)).expect("fragment is readable"));
            bundle.push('\n');
        }
    }

    let rule_start = bundle
        .find("[dir=\"rtl\"] .rio-icon--directional")
        .expect("RTL mirror rule for .rio-icon--directional is missing from the bundle");
    let block_end = bundle[rule_start..]
        .find('}')
        .expect("unterminated RTL mirror rule")
        + rule_start;
    let block = &bundle[rule_start..block_end];
    assert!(
        block.contains("scaleX(-1)"),
        "the RTL rule must mirror horizontally; got: {block}"
    );

    // Exactly one horizontal mirror in the whole bundle. A second would
    // flip a directional icon back to its LTR hand.
    assert_eq!(
        bundle.matches("scaleX(-1)").count(),
        1,
        "more than one scaleX(-1) in the bundle would double-mirror directional icons"
    );
    // The mirror must be gated on RTL; an ungated rule would flip LTR too.
    assert_eq!(
        bundle.matches(".rio-icon--directional").count(),
        1,
        ".rio-icon--directional must be styled in exactly one place"
    );
}

#[test]
fn the_admin_js_bundle_ships() {
    assert!(assets_root().join("static/admin.js").is_file());
}

// ---- 4. the shared page-header contract -----------------------------------

/// The console carries two page-header shapes. `.rio-page-header` wraps its
/// own content, so `components/page-header.css` can style it as one box.
/// The other shape — `.rio-crumbs` followed by `.rio-masthead-top` — has no
/// wrapper, so that fragment caps the two halves (top corners on the crumbs,
/// bottom corners on the masthead) to render them as one surface.
///
/// That capping is unconditional, which is only safe while the two classes
/// are a matched pair: every template carrying one carries the other,
/// exactly once. A template that grew a lone `.rio-crumbs` would render an
/// open-bottomed box, and a lone `.rio-masthead-top` an open-topped one.
/// Neither is a compile error, and neither shows up in a unit test — hence
/// this check.
#[test]
fn the_page_header_halves_are_a_matched_pair() {
    let mut checked = 0usize;
    for name in embedded_template_names().iter() {
        let src = embedded_template_source(name).expect("listed template resolves");
        let crumbs = src.matches("\"rio-crumbs\"").count();
        let masthead = src.matches("\"rio-masthead-top\"").count();
        if crumbs == 0 && masthead == 0 {
            continue;
        }
        assert_eq!(
            crumbs, masthead,
            "{name} has {crumbs} `rio-crumbs` and {masthead} `rio-masthead-top` — \
             the page-header halves must appear together or the capped surface \
             renders with an open edge (see components/page-header.css)"
        );
        assert_eq!(
            crumbs, 1,
            "{name} repeats the page-header pair {crumbs} times; the capped \
             surface assumes one header per page"
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no template uses the crumbs/masthead header — did the contract move?"
    );
}

/// A unique scratch directory under the target dir. Avoids a dev-dependency
/// on `tempfile` for three call sites.
fn tempdir() -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "rustio-contract-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}
