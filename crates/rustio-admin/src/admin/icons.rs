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

    m
});

// internal:
/// Render an icon inline. Returns an empty string if `name` is
/// unknown (templates remain forgiving — a missing icon shouldn't
/// crash the page render).
pub fn render_inline(name: &str, class: &str) -> String {
    let Some(inner) = ICONS.get(name) else {
        log::warn!("icon({name:?}) not found — rendering empty");
        return String::new();
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="{class}" aria-hidden="true">{inner}</svg>"#
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
}
