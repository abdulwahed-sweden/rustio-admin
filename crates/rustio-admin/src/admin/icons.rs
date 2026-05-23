//! Inline lucide stroke icons for the admin UI.
//!
//! Icons are baked at compile time. Each entry is the inner
//! `<path>`/`<circle>`/etc. of a 24×24 lucide stroke icon. The
//! minijinja `icon(name, class="...")` function (registered in
//! P7's `templates.rs` rebuild) wraps the inner shape in an `<svg>`
//! with `fill="none"`, `stroke="currentColor"`, and a default
//! `width`/`height` of 16, so colour is inherited from the rendering
//! context.
//!
//! Source: <https://lucide.dev>, MIT-licensed.

use once_cell::sync::Lazy;
use std::collections::HashMap;

static ICONS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // Sidebar nav
    m.insert("home",
        r#"<path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/>"#);
    m.insert("table",
        r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 9h18"/><path d="M3 15h18"/><path d="M9 3v18"/><path d="M15 3v18"/>"#);
    m.insert("users",
        r#"<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>"#);
    m.insert("users-2",
        r#"<path d="M14 19a6 6 0 0 0-12 0"/><circle cx="8" cy="9" r="4"/><path d="M22 19a6 6 0 0 0-6-6 4 4 0 1 0 0-8"/>"#);
    m.insert("database",
        r#"<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5V19A9 3 0 0 0 21 19V5"/><path d="M3 12A9 3 0 0 0 21 12"/>"#);
    m.insert(
        "clock",
        r#"<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>"#,
    );
    m.insert(
        "terminal",
        r#"<polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/>"#,
    );

    // Action buttons
    m.insert("plus", r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#);
    m.insert("more-horizontal",
        r#"<circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>"#);
    m.insert("pencil",
        r#"<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>"#);
    m.insert("trash",
        r#"<path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>"#);
    m.insert(
        "arrow-left",
        r#"<path d="m12 19-7-7 7-7"/><path d="M19 12H5"/>"#,
    );
    m.insert("log-out",
        r#"<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" x2="9" y1="12" y2="12"/>"#);
    m.insert("key",
        r#"<path d="m21 2-9.6 9.6"/><circle cx="7.5" cy="15.5" r="5.5"/><path d="m15.5 7.5 3 3L22 7l-3-3"/><path d="m13 10 1.5-1.5"/>"#);

    // Notices / hints
    m.insert("circle-alert",
        r#"<circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/>"#);
    m.insert(
        "circle-x",
        r#"<circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>"#,
    );

    // Sidebar mobile toggle
    m.insert("menu",
        r#"<line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="18" y2="18"/>"#);

    // Toolbar — search input glyph, filter dropdown trigger, chevrons.
    m.insert(
        "search",
        r#"<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>"#,
    );
    m.insert(
        "filter",
        r#"<polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/>"#,
    );
    m.insert("chevron-down", r#"<path d="m6 9 6 6 6-6"/>"#);

    // Export — list-page toolbar "Download CSV" affordance.
    m.insert("download",
        r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>"#);

    // Import — list-page toolbar "Import CSV" affordance. Mirror
    // of `download`; arrow flipped to point up into the box.
    m.insert("upload",
        r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/>"#);

    // Sliders — list-page toolbar "View" overflow menu (sort /
    // per-page / data export+import). Two horizontal tracks with
    // tick handles, the universal "tweak settings" glyph.
    m.insert("sliders",
        r#"<line x1="4" y1="21" x2="4" y2="14"/><line x1="4" y1="10" x2="4" y2="3"/><line x1="12" y1="21" x2="12" y2="12"/><line x1="12" y1="8" x2="12" y2="3"/><line x1="20" y1="21" x2="20" y2="16"/><line x1="20" y1="12" x2="20" y2="3"/><line x1="1" y1="14" x2="7" y2="14"/><line x1="9" y1="8" x2="15" y2="8"/><line x1="17" y1="16" x2="23" y2="16"/>"#);

    // Bookmark — list-page toolbar "Saved" dropdown trigger.
    m.insert(
        "bookmark",
        r#"<path d="m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z"/>"#,
    );

    // Flag — dashboard "Feature flags" tool tile.
    m.insert(
        "flag",
        r#"<path d="M4 22V4a1 1 0 0 1 1-1h13.97a.5.5 0 0 1 .39.81L16 8l3.36 4.19a.5.5 0 0 1-.39.81H5"/>"#,
    );

    // Bell — topbar notifications link.
    m.insert(
        "bell",
        r#"<path d="M10.268 21a2 2 0 0 0 3.464 0"/><path d="M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326"/>"#,
    );

    m
});

/// Icons whose shape carries a left/right direction — chevrons,
/// arrows, "log out" arrow glyphs, etc. The renderer appends the
/// `rio-icon--directional` class on these so the CSS rule under
/// `[dir="rtl"]` flips them horizontally. Symbol-shaped icons
/// (`home`, `users`, `database`, `key`, `clock`) and
/// vertical-axis chevrons (`chevron-down`) stay unchanged in
/// either direction — adding them to this list would mirror
/// glyphs that shouldn't mirror.
const DIRECTIONAL_ICONS: &[&str] = &["arrow-left", "log-out"];

fn is_directional(name: &str) -> bool {
    DIRECTIONAL_ICONS.contains(&name)
}

// internal:
/// Render an icon inline. Returns an empty string if `name` is
/// unknown (templates remain forgiving — a missing icon shouldn't
/// crash the page render).
pub(crate) fn render_inline(name: &str, class: &str) -> String {
    let Some(inner) = ICONS.get(name) else {
        log::warn!("icon({name:?}) not found — rendering empty");
        return String::new();
    };
    // Directional icons get an extra class hook that the CSS picks
    // up under `[dir="rtl"]` to flip the SVG horizontally. Composed
    // here rather than at the call site so every template that
    // emits an arrow / log-out / future-chevron-left mirrors
    // automatically without per-template `rio-icon--directional`
    // copy-paste.
    let composed_class: String = if is_directional(name) {
        if class.is_empty() {
            "rio-icon--directional".to_string()
        } else {
            format!("{class} rio-icon--directional")
        }
    } else {
        class.to_string()
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="{composed_class}" aria-hidden="true">{inner}</svg>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_icons_render() {
        let svg = render_inline("home", "w-4 h-4");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains(r#"class="w-4 h-4""#));
        assert!(svg.contains(r#"stroke="currentColor""#));
        assert!(svg.contains(r#"viewBox="0 0 24 24""#));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn unknown_icon_returns_empty_not_panic() {
        let svg = render_inline("not-a-real-icon", "");
        assert_eq!(svg, "");
    }

    /// Directional icons (`arrow-left`, `log-out`, …) carry the
    /// `rio-icon--directional` class so the CSS rule under
    /// `[dir="rtl"]` flips them horizontally without per-template
    /// boilerplate.
    #[test]
    fn directional_icon_appends_marker_class() {
        let svg = render_inline("arrow-left", "rio-icon");
        assert!(
            svg.contains(r#"class="rio-icon rio-icon--directional""#),
            "rio-icon class chain wrong: {svg}"
        );
    }

    /// Symbol-shaped icons keep the caller's class verbatim — no
    /// `rio-icon--directional` smuggled in, because mirroring a
    /// `home` glyph in RTL would produce a wrong-handed house.
    #[test]
    fn symbol_icon_does_not_get_directional_marker() {
        let svg = render_inline("home", "rio-icon");
        assert!(svg.contains(r#"class="rio-icon""#));
        assert!(!svg.contains("rio-icon--directional"));
    }

    /// Vertical chevrons aren't directional in the LTR/RTL sense —
    /// `down` stays `down` regardless of writing direction.
    #[test]
    fn vertical_chevron_is_not_directional() {
        let svg = render_inline("chevron-down", "rio-chev");
        assert!(!svg.contains("rio-icon--directional"));
    }

    /// Directional icon with no caller-supplied class still carries
    /// the marker alone — protects the edge case where a template
    /// emits `{{ icon("arrow-left") }}` without explicit styling.
    #[test]
    fn directional_icon_with_empty_class_still_marked() {
        let svg = render_inline("arrow-left", "");
        assert!(
            svg.contains(r#"class="rio-icon--directional""#),
            "missing marker with empty class: {svg}"
        );
    }
}
