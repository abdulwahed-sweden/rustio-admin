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

  // ---- Generic dropdown wiring ------------------------------------
  // Any `[data-rio-dropdown]` wrapper that contains a
  // `.rio-dropdown-toggle` and a `.rio-dropdown-panel` gets the same
  // open/close machinery: click the toggle to flip `is-open`, click
  // outside to close, Esc to close. The CSS reads `.is-open` and
  // shows the panel + rotates the chevron.
  function initDropdowns() {
    const dropdowns = document.querySelectorAll("[data-rio-dropdown]");
    if (!dropdowns.length) return;

    dropdowns.forEach((dd) => {
      const toggle = dd.querySelector(".rio-dropdown-toggle");
      if (!toggle) return;
      toggle.addEventListener("click", (e) => {
        e.stopPropagation();
        const open = dd.classList.toggle("is-open");
        toggle.setAttribute("aria-expanded", String(open));
      });
    });

    document.addEventListener("click", (e) => {
      dropdowns.forEach((dd) => {
        if (dd.classList.contains("is-open") && !dd.contains(e.target)) {
          dd.classList.remove("is-open");
          const t = dd.querySelector(".rio-dropdown-toggle");
          if (t) t.setAttribute("aria-expanded", "false");
        }
      });
    });

    document.addEventListener("keydown", (e) => {
      if (e.key !== "Escape") return;
      dropdowns.forEach((dd) => {
        if (!dd.classList.contains("is-open")) return;
        dd.classList.remove("is-open");
        const t = dd.querySelector(".rio-dropdown-toggle");
        if (t) {
          t.setAttribute("aria-expanded", "false");
          t.focus();
        }
      });
    });
  }

  // ---- Bulk select ------------------------------------------------
  // The list-view table is wrapped in a `<form data-rio-bulk>`. Each
  // row has a checkbox; the header has a master checkbox; a hidden
  // `_ids` input gets populated with the comma-separated selection
  // before submit. The framework's FormData uses one value per key,
  // so a CSV is the simplest wire format that survives the round
  // trip — see handlers::handle_bulk_delete for the parser.
  function initBulkSelect() {
    const form = document.querySelector("[data-rio-bulk]");
    if (!form) return;
    const all = form.querySelector("[data-rio-bulk-all]");
    const idsInput = form.querySelector("[data-rio-bulk-ids]");
    const countEl = form.querySelector("[data-rio-bulk-count]");
    const clearBtn = form.querySelector("[data-rio-bulk-clear]");
    const rows = Array.from(form.querySelectorAll("[data-rio-bulk-row]"));
    if (!rows.length) return;

    function refresh() {
      const checked = rows.filter((r) => r.checked);
      const count = checked.length;

      // Reflect selected count into the form: hidden CSV field for
      // the POST, label in the bulk bar, `is-selected` on the row,
      // and the bar's visibility (driven by `.is-active` on the form).
      idsInput.value = checked.map((r) => r.value).join(",");
      if (countEl) countEl.textContent = String(count);
      form.classList.toggle("is-active", count > 0);
      rows.forEach((r) => {
        const tr = r.closest("tr");
        if (tr) tr.classList.toggle("is-selected", r.checked);
      });

      // Master checkbox: checked when all selected, indeterminate
      // when partial, unchecked when none.
      if (all) {
        all.checked = count > 0 && count === rows.length;
        all.indeterminate = count > 0 && count < rows.length;
      }
    }

    rows.forEach((r) => r.addEventListener("change", refresh));

    if (all) {
      all.addEventListener("change", () => {
        rows.forEach((r) => { r.checked = all.checked; });
        refresh();
      });
    }

    if (clearBtn) {
      clearBtn.addEventListener("click", () => {
        rows.forEach((r) => { r.checked = false; });
        refresh();
      });
    }

    // Guard against empty submit (Enter key on a focused checkbox,
    // accidental click): block the POST when nothing is selected so
    // the user lands back where they started instead of bouncing
    // through `/admin/:model` via the empty-ids redirect.
    form.addEventListener("submit", (e) => {
      if (!idsInput.value) e.preventDefault();
    });

    refresh();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      initTheme();
      initSidebar();
      initDropdowns();
      initBulkSelect();
    });
  } else {
    initTheme();
    initSidebar();
    initDropdowns();
    initBulkSelect();
  }
})();
