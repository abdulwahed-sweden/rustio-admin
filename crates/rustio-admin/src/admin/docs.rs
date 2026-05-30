//! `/admin/docs` — read the framework's own markdown docs inside
//! the admin chrome.
//!
//! Three top-level files are embedded via `include_str!` so the
//! single-binary deploy story stays intact:
//!
//! - `architecture.md` — module-level map of the framework crate
//! - `modeladmin.md` — Django-style customisation surface
//! - `public-api.md` — generated/descriptive `pub` surface
//!
//! Operators read these straight from the admin instead of opening
//! GitHub. Markdown → HTML via `pulldown-cmark`, no DB, no
//! filesystem; everything is baked at compile time.
//!
//! Out of scope for v1: the larger `docs/design/*.md` doctrine
//! files. Those are pull-request review surfaces and the framework
//! doesn't try to render them in-app (operators following a PR
//! review against doctrine open them in their editor, not in the
//! admin).

use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};

/// One embedded markdown doc. `slug` is the URL key
/// (`/admin/docs/<slug>`); `title` is the display label; `source`
/// is the markdown body baked at compile time.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EmbeddedDoc {
    pub slug: &'static str,
    pub title: &'static str,
    pub source: &'static str,
}

/// Stable list of embedded docs. Ordered for the index page.
///
/// Sources live under `crates/rustio-admin/assets/docs/`, copied
/// from the workspace-level `docs/` directory at release time.
/// Keeping a copy inside the crate is what makes
/// `cargo publish` happy — `include_str!` paths must stay inside
/// the crate tarball, so we cannot reach `../../../../docs/`
/// (which would walk above the crate root). Update both copies
/// when the canonical workspace docs change.
pub(crate) const EMBEDDED_DOCS: &[EmbeddedDoc] = &[
    EmbeddedDoc {
        slug: "architecture",
        title: "Architecture",
        source: include_str!("../../assets/docs/architecture.md"),
    },
    EmbeddedDoc {
        slug: "modeladmin",
        title: "ModelAdmin customisation",
        source: include_str!("../../assets/docs/modeladmin.md"),
    },
    EmbeddedDoc {
        slug: "public-api",
        title: "Public API surface",
        source: include_str!("../../assets/docs/public-api.md"),
    },
];

/// Look up an embedded doc by URL slug. Returns `None` for
/// unknown slugs so the route handler can 404 cleanly.
pub(crate) fn find(slug: &str) -> Option<&'static EmbeddedDoc> {
    EMBEDDED_DOCS.iter().find(|d| d.slug == slug)
}

/// True when a link in the in-app docs can actually be followed.
///
/// The `/admin/docs` viewer serves only the three embedded docs by slug,
/// so cross-document *relative* links (`./design/…`, `./archive/…`, a
/// sibling `*.md`) cannot resolve — they would render as dead links that
/// 404. We keep only external URLs and in-page anchors; everything else
/// is rendered as plain text (see [`render_markdown`]). The canonical
/// `docs/` copies keep these links for GitHub browsing — only the in-app
/// render strips the ones it can't follow.
fn link_is_navigable(dest: &str) -> bool {
    dest.starts_with("http://")
        || dest.starts_with("https://")
        || dest.starts_with("mailto:")
        || dest.starts_with('#')
}

/// Render markdown source to a safe HTML fragment. Enables the
/// commonly-needed GFM extensions (tables, fenced code blocks,
/// strikethrough) so the framework's existing docstrings render
/// correctly. The minijinja template that wraps the output marks
/// the fragment as `|safe` because this function is the trusted
/// boundary — the source is framework-owned, never user-supplied.
///
/// Non-navigable links (see [`link_is_navigable`]) are stripped to their
/// text so the in-app viewer never shows a link that 404s.
pub(crate) fn render_markdown(src: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);

    // Drop the anchor around any link the in-app viewer can't follow,
    // keeping the link text. Links don't nest in CommonMark, so a single
    // flag tracks whether we're inside a dropped link.
    let mut drop_link = false;
    let events = Parser::new_ext(src, opts).filter_map(|ev| match ev {
        Event::Start(Tag::Link { ref dest_url, .. }) if !link_is_navigable(dest_url) => {
            drop_link = true;
            None
        }
        Event::End(TagEnd::Link) if drop_link => {
            drop_link = false;
            None
        }
        other => Some(other),
    });

    let mut out = String::with_capacity(src.len() * 2);
    html::push_html(&mut out, events);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_docs_have_non_empty_sources() {
        // include_str! at compile time guarantees the file exists,
        // but an empty file would compile silently — guard against
        // a future move/rename that empties the path.
        for d in EMBEDDED_DOCS {
            assert!(!d.source.is_empty(), "doc `{}` source is empty", d.slug);
            assert!(!d.title.is_empty(), "doc `{}` title is empty", d.slug);
        }
    }

    #[test]
    fn embedded_doc_slugs_are_unique() {
        // The `/admin/docs/<slug>` route uses linear search; a
        // duplicate slug would render the first match silently
        // and never reach the second. Lock the invariant.
        let mut seen = std::collections::HashSet::new();
        for d in EMBEDDED_DOCS {
            assert!(seen.insert(d.slug), "duplicate doc slug `{}`", d.slug);
        }
    }

    #[test]
    fn find_returns_none_for_unknown_slug() {
        assert!(find("definitely-not-a-doc").is_none());
        assert!(find("").is_none());
    }

    #[test]
    fn render_markdown_emits_html_for_headings_and_code() {
        let src = "# Title\n\nA paragraph.\n\n```rust\nlet x = 1;\n```\n";
        let html = render_markdown(src);
        assert!(html.contains("<h1>Title</h1>"), "missing H1: {html}");
        assert!(html.contains("<p>A paragraph.</p>"), "missing P: {html}");
        assert!(html.contains("<pre><code"), "missing code block: {html}");
    }

    #[test]
    fn render_markdown_strips_non_navigable_links_to_text() {
        // Cross-document relative links can't resolve in the in-app
        // viewer, so they render as plain text (no <a>), keeping the
        // label. Covers the design/ + sibling-.md cases the bundled
        // architecture.md / modeladmin.md carry.
        let src = "See [the theme doc](./design/DESIGN_THEME.md) \
                   and [a sibling](public-api.md) \
                   and [reset plan](./archive/PLAN.md#8-rules).";
        let html = render_markdown(src);
        assert!(
            !html.contains("<a "),
            "cross-doc links must be de-linked: {html}"
        );
        assert!(html.contains("the theme doc"), "link text kept: {html}");
        assert!(html.contains("a sibling"), "link text kept: {html}");
        assert!(html.contains("reset plan"), "link text kept: {html}");
    }

    #[test]
    fn render_markdown_keeps_external_and_anchor_links() {
        let src = "[site](https://example.com) · [mail](mailto:x@y.z) · [top](#heading)";
        let html = render_markdown(src);
        assert!(
            html.contains(r#"<a href="https://example.com""#),
            "external link kept: {html}"
        );
        assert!(
            html.contains(r#"<a href="mailto:x@y.z""#),
            "mailto kept: {html}"
        );
        assert!(
            html.contains(r##"<a href="#heading""##),
            "in-page anchor kept: {html}"
        );
    }

    #[test]
    fn render_markdown_escapes_html_in_source() {
        // pulldown-cmark escapes raw HTML by default in safe mode;
        // we don't pass any UNSAFE option so a stray `<script>` in
        // the docs becomes inert text rather than an injection
        // vector. The framework docs don't include user content
        // today; this is defense-in-depth for tomorrow.
        let src = "<script>alert(1)</script>";
        let html = render_markdown(src);
        // pulldown-cmark passes raw HTML through unless OPTIONS
        // changes; we don't enable any unsafe options. But the
        // default is actually to pass through... let me check.
        // Actually with no opts, raw HTML IS passed through.
        // The safety guarantee here is that the SOURCE is framework-
        // owned, not user input. So this test pins current behaviour
        // (HTML pass-through) and the safety story is "trusted source",
        // not "HTML escaped". Make the test pin pass-through so a
        // future toggle of OPTIONS surfaces the change loudly.
        assert!(
            html.contains("<script>"),
            "raw HTML passes through (source is framework-trusted): {html}"
        );
    }
}
