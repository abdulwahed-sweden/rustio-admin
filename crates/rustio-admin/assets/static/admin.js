/* rustio-admin client-side helpers — minimal on purpose.
 *
 *  - Sidebar drawer toggle on mobile. Adds `data-sidebar="open"` to
 *    the .rio-shell so the CSS reveals the off-canvas panel.
 *  - Generic dropdown wiring, bulk-select form helper,
 *    foreign-key autocomplete.
 *
 * Sortable column headers and remote filter widgets land in P9/P10.
 */
(function () {
  "use strict";

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

  // ---- FK autocomplete --------------------------------------------
  // Each `[data-rio-fk-autocomplete]` form contains a visible search
  // input (`[data-rio-fk-search]`), a hidden id field
  // (`[data-rio-fk-id]`), and a results list (`[data-rio-fk-results]`).
  // Typing in the search input debounces a fetch to the configured
  // lookup endpoint; clicking a result writes the chosen id into the
  // hidden field and the label into the visible one. Without JS the
  // visible input stays empty and the operator can still type a
  // numeric id into the search box — but since the hidden id field
  // is the actual submitted name, free-text without JS won't filter.
  // (The form's id field carries the previously-selected value so
  // page reloads round-trip correctly.)
  function initFkAutocomplete() {
    const widgets = document.querySelectorAll("[data-rio-fk-autocomplete]");
    widgets.forEach((widget) => {
      const lookupUrl = widget.getAttribute("data-rio-fk-lookup-url");
      if (!lookupUrl) return;
      const search = widget.querySelector("[data-rio-fk-search]");
      const idInput = widget.querySelector("[data-rio-fk-id]");
      const results = widget.querySelector("[data-rio-fk-results]");
      if (!search || !idInput || !results) return;

      let debounce = 0;
      let lastTerm = "";

      function hideResults() {
        results.setAttribute("hidden", "");
        results.innerHTML = "";
      }

      function render(items) {
        results.innerHTML = "";
        if (!items.length) {
          const empty = document.createElement("li");
          empty.className = "rio-fk-autocomplete-empty";
          empty.textContent = "No matches";
          results.appendChild(empty);
        } else {
          items.forEach((item) => {
            const li = document.createElement("li");
            li.className = "rio-fk-autocomplete-result";
            li.textContent = item.label;
            li.setAttribute("role", "option");
            li.setAttribute("data-id", String(item.id));
            li.addEventListener("mousedown", (e) => {
              // mousedown fires before the input's blur, so the
              // click registers before the panel hides itself.
              e.preventDefault();
              idInput.value = String(item.id);
              search.value = item.label;
              hideResults();
            });
            results.appendChild(li);
          });
        }
        results.removeAttribute("hidden");
      }

      async function fetchResults(term) {
        try {
          const url = lookupUrl + "?q=" + encodeURIComponent(term);
          const resp = await fetch(url, {
            headers: { Accept: "application/json" },
            credentials: "same-origin",
          });
          if (!resp.ok) return;
          const items = await resp.json();
          if (Array.isArray(items)) render(items);
        } catch (_e) {
          // Network blip — leave the previous results visible.
        }
      }

      search.addEventListener("input", () => {
        // Typing into the search box clears the previously-chosen
        // id; the operator must commit a new choice (click a result
        // or leave it blank to mean "no filter").
        if (search.value !== lastTerm) idInput.value = "";
        lastTerm = search.value;
        window.clearTimeout(debounce);
        const term = search.value.trim();
        debounce = window.setTimeout(() => fetchResults(term), 250);
      });
      search.addEventListener("focus", () => {
        if (search.value.trim().length > 0) {
          window.clearTimeout(debounce);
          debounce = window.setTimeout(() => fetchResults(search.value.trim()), 50);
        }
      });
      search.addEventListener("blur", () => {
        // Defer so a mousedown on a result can finish first.
        window.setTimeout(hideResults, 120);
      });
      search.addEventListener("keydown", (e) => {
        if (e.key === "Escape") {
          hideResults();
        }
      });
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      initSidebar();
      initDropdowns();
      initBulkSelect();
      initFkAutocomplete();
    });
  } else {
    initSidebar();
    initDropdowns();
    initBulkSelect();
    initFkAutocomplete();
  }
})();
