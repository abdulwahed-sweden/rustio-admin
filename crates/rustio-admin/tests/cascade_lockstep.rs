//! Cascade lockstep — load-bearing architecture protection.
//!
//! The order of `@import url(...)` in `assets/static/admin/admin.css`
//! MUST match the order of `include_str!(...)` in `ADMIN_CSS` in
//! `src/admin/routes.rs`. The two are the source of cascade order;
//! drift between them ships a bundle that silently differs from the
//! contributor-facing manifest and can break cascade-sensitive
//! overrides (responsive after components, light after dark, etc.)
//! in ways that pass `cargo test` and pass code review.
//!
//! DO NOT delete this test. If you add or remove a fragment, update
//! BOTH lists in the same commit.

use std::fs;

const ADMIN_CSS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/static/admin/admin.css");
const ROUTES_RS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/admin/routes.rs");
const PREFIX: &str = "../../assets/static/admin/";

fn between_quotes(line: &str) -> Option<&str> {
    let s = line.find('"')? + 1;
    let r = &line[s..];
    let e = r.find('"')?;
    Some(&r[..e])
}

fn css_imports() -> Vec<String> {
    fs::read_to_string(ADMIN_CSS).unwrap().lines()
        .filter(|l| l.trim_start().starts_with("@import url("))
        .filter_map(between_quotes)
        .map(String::from)
        .collect()
}

fn rust_includes() -> Vec<String> {
    fs::read_to_string(ROUTES_RS).unwrap().lines()
        .filter(|l| l.contains("include_str!("))
        .filter_map(between_quotes)
        .filter_map(|p| p.strip_prefix(PREFIX).map(String::from))
        .collect()
}

#[test]
fn admin_css_import_order_matches_routes_concat_order() {
    let css = css_imports();
    let rs = rust_includes();
    if css == rs { return; }
    let mut msg = String::from("\nCascade lockstep violation:\n");
    let n = css.len().max(rs.len());
    for i in 0..n {
        let c = css.get(i).map(String::as_str).unwrap_or("(missing)");
        let r = rs.get(i).map(String::as_str).unwrap_or("(missing)");
        let mark = if c == r { "  " } else { "**" };
        msg.push_str(&format!("{} {:>2}  css={:35}  rust={}\n", mark, i, c, r));
    }
    panic!("{}", msg);
}
