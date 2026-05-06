/* rustio-admin client-side helpers — minimal on purpose.
 *
 *  - Theme toggle: cycles light/dark/system; persists to localStorage
 *    under `rio-theme`. The CSS picks up `data-rio-theme` on <html>.
 *  - Sidebar drawer toggle on mobile. Adds `data-sidebar="open"` to
 *    the .rio-shell so the CSS reveals the off-canvas panel.
 *
 * Sortable column headers and remote filter widgets land in P9/P10.
 */
(function () {
  "use strict";

  // ---- Theme -----------------------------------------------------
  const THEME_KEY = "rio-theme";
  const root = document.documentElement;

  function applyTheme(value) {
    if (value === "dark" || value === "light") {
      root.setAttribute("data-rio-theme", value);
    } else {
      root.removeAttribute("data-rio-theme");
    }
  }

  function nextTheme(current) {
    if (current === "light") return "dark";
    if (current === "dark") return "system";
    return "light";
  }

  function themeLabel(value) {
    if (value === "light") return "Light";
    if (value === "dark") return "Dark";
    return "System";
  }

  function initTheme() {
    let stored = null;
    try { stored = localStorage.getItem(THEME_KEY); } catch (_) { /* private mode */ }
    if (stored === "light" || stored === "dark") applyTheme(stored);

    const button = document.querySelector("[data-rio-theme-toggle]");
    if (!button) return;

    function refresh() {
      const value = stored || "system";
      button.textContent = themeLabel(value);
      button.setAttribute("aria-label", "Theme: " + themeLabel(value));
    }
    refresh();

    button.addEventListener("click", () => {
      stored = nextTheme(stored || "system");
      try {
        if (stored === "system") localStorage.removeItem(THEME_KEY);
        else localStorage.setItem(THEME_KEY, stored);
      } catch (_) { /* ignore */ }
      applyTheme(stored);
      refresh();
    });
  }

  // ---- Sidebar drawer (mobile) -----------------------------------
  function initSidebar() {
    const shell = document.querySelector(".rio-shell");
    const toggle = document.querySelector("[data-rio-sidebar-toggle]");
    if (!shell || !toggle) return;

    toggle.addEventListener("click", () => {
      const open = shell.getAttribute("data-sidebar") === "open";
      if (open) shell.removeAttribute("data-sidebar");
      else shell.setAttribute("data-sidebar", "open");
    });

    // Close drawer when a nav link is clicked.
    shell.addEventListener("click", (evt) => {
      const link = evt.target.closest(".rio-sidebar-link");
      if (link) shell.removeAttribute("data-sidebar");
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      initTheme();
      initSidebar();
    });
  } else {
    initTheme();
    initSidebar();
  }
})();
