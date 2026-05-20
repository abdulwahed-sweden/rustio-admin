/* rustio-admin client-side helpers — minimal on purpose.
 *
 *  - Sidebar drawer toggle on mobile. Adds `data-sidebar="open"` to
 *    the .rio-shell so the CSS reveals the off-canvas panel.
 *  - Generic dropdown wiring, bulk-select form helper,
 *    foreign-key autocomplete, global ⌘K search palette.
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

  // ---- Global ⌘K search palette ----------------------------------
  // Backed by GET /admin/_search?q=<term> which returns
  //   { results: [{ admin_name, model_label, label, url }, ...] }
  // Capped server-side at 5 per model + 20 total; the palette UI
  // groups results by `model_label`.
  //
  // Keyboard surface:
  //   ⌘K / Ctrl+K   open the palette (anywhere outside an input)
  //   Esc           close, restoring focus to the trigger
  //   ↑ / ↓         move selection between results (with wrap)
  //   Enter         navigate to the selected result
  function initSearchPalette() {
    const trigger = document.querySelector("[data-rio-search-trigger]");
    const palette = document.querySelector("[data-rio-search-palette]");
    if (!palette) return;
    const dialog = palette.querySelector("[data-rio-search-palette-dialog]");
    const input = palette.querySelector("[data-rio-search-palette-input]");
    const list = palette.querySelector("[data-rio-search-palette-results]");
    if (!dialog || !input || !list) return;

    let debounceTimer = 0;
    let selectedIndex = -1;
    let resultItems = [];
    // Render-scoped id counters so each <a role="option"> gets a
    // unique DOM id we can reference from `aria-activedescendant`
    // on the input, and each group's <li role="group"> gets an
    // `aria-labelledby` target. Reset on every render() so ids
    // don't grow unbounded over the session.
    let optionIdCounter = 0;
    let groupIdCounter = 0;

    function open() {
      palette.setAttribute("aria-hidden", "false");
      input.value = "";
      list.innerHTML = "";
      selectedIndex = -1;
      resultItems = [];
      input.removeAttribute("aria-activedescendant");
      // Defer focus so the click that opened us doesn't immediately
      // bubble and close us via the backdrop handler.
      setTimeout(() => input.focus(), 0);
    }

    function close() {
      if (palette.getAttribute("aria-hidden") === "true") return;
      palette.setAttribute("aria-hidden", "true");
      window.clearTimeout(debounceTimer);
      input.removeAttribute("aria-activedescendant");
      if (trigger) trigger.focus();
    }

    function isOpen() {
      return palette.getAttribute("aria-hidden") === "false";
    }

    function setSelected(idx) {
      if (resultItems.length === 0) {
        selectedIndex = -1;
        input.removeAttribute("aria-activedescendant");
        return;
      }
      // Wrap around both directions so ↑ from the first lands on
      // the last, and ↓ from the last lands on the first.
      const n = resultItems.length;
      selectedIndex = ((idx % n) + n) % n;
      resultItems.forEach((el, i) => {
        el.classList.toggle("is-selected", i === selectedIndex);
      });
      resultItems[selectedIndex].scrollIntoView({ block: "nearest" });
      // Screen-reader announcement of the visually-highlighted
      // option — the ARIA-APG combobox/listbox pattern.
      input.setAttribute("aria-activedescendant", resultItems[selectedIndex].id);
    }

    function render(results) {
      list.innerHTML = "";
      resultItems = [];
      selectedIndex = -1;
      input.removeAttribute("aria-activedescendant");
      optionIdCounter = 0;
      groupIdCounter = 0;
      if (!results.length) {
        const empty = document.createElement("li");
        empty.className = "rio-search-palette__empty";
        empty.textContent = "No results.";
        list.appendChild(empty);
        return;
      }
      // Group by model_label preserving server-side order. The
      // server already orders entries by admin registration order,
      // so two passes give us stable section headings.
      const groups = new Map();
      results.forEach((r) => {
        if (!groups.has(r.model_label)) groups.set(r.model_label, []);
        groups.get(r.model_label).push(r);
      });
      groups.forEach((rows, label) => {
        const group = document.createElement("li");
        group.className = "rio-search-palette__group";
        // role="group" + aria-labelledby pattern (ARIA 1.2 §6.7):
        // valid child of role="listbox", with the heading span
        // naming the group for screen readers.
        const headingId = `rio-search-palette__group-${groupIdCounter++}`;
        group.setAttribute("role", "group");
        group.setAttribute("aria-labelledby", headingId);
        const heading = document.createElement("span");
        heading.className = "rio-search-palette__group-label";
        heading.id = headingId;
        heading.textContent = label;
        group.appendChild(heading);
        rows.forEach((r) => {
          const a = document.createElement("a");
          a.className = "rio-search-palette__result";
          a.href = r.url;
          a.id = `rio-search-palette__option-${optionIdCounter++}`;
          a.setAttribute("role", "option");
          // Keep anchors out of the tab order — focus stays on the
          // input; selection is tracked via aria-activedescendant.
          // Stops Tab from escaping the dialog into the topbar/sidebar.
          a.setAttribute("tabindex", "-1");
          const text = document.createElement("span");
          text.className = "rio-search-palette__result-label";
          text.textContent = r.label;
          a.appendChild(text);
          group.appendChild(a);
          resultItems.push(a);
        });
        list.appendChild(group);
      });
      // Default the highlight to the first result so Enter has an
      // unambiguous target right away.
      setSelected(0);
    }

    async function fetchResults(term) {
      try {
        const url = "/admin/_search?q=" + encodeURIComponent(term);
        const resp = await fetch(url, {
          headers: { Accept: "application/json" },
          credentials: "same-origin",
        });
        if (!resp.ok) return;
        const body = await resp.json();
        if (body && Array.isArray(body.results)) render(body.results);
      } catch (_e) {
        // Network blip — leave the previous results visible.
      }
    }

    // ---- wire it up
    if (trigger) trigger.addEventListener("click", open);

    palette.addEventListener("click", (e) => {
      // Backdrop click closes; clicks inside the dialog don't.
      if (e.target === palette) close();
    });

    input.addEventListener("input", () => {
      window.clearTimeout(debounceTimer);
      const term = input.value.trim();
      if (term.length < 2) {
        list.innerHTML = "";
        resultItems = [];
        selectedIndex = -1;
        return;
      }
      debounceTimer = window.setTimeout(() => fetchResults(term), 200);
    });

    input.addEventListener("keydown", (e) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelected(selectedIndex + 1);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelected(selectedIndex - 1);
      } else if (e.key === "Tab") {
        // Focus trap for aria-modal="true". Anchors carry
        // tabindex="-1" so they're out of the natural Tab chain;
        // Tab/Shift+Tab on the input cycle selection rather than
        // letting focus escape into the topbar/sidebar.
        e.preventDefault();
        if (resultItems.length > 0) {
          setSelected(selectedIndex + (e.shiftKey ? -1 : 1));
        }
      } else if (e.key === "Enter") {
        if (selectedIndex >= 0 && resultItems[selectedIndex]) {
          e.preventDefault();
          window.location.href = resultItems[selectedIndex].href;
        }
      }
    });

    // Global shortcuts. Esc closes the palette only when it's open
    // (so it doesn't fight other Esc handlers — dropdowns, FK
    // autocomplete). ⌘K / Ctrl+K opens from anywhere, but stays
    // inert when focus is already in a text input or contenteditable
    // surface (e.g. the per-list-page search) so the operator's
    // local keystroke isn't stolen mid-typing. The palette's own
    // input is excepted so ⌘K still closes the palette when it's
    // focused — that's the canonical "toggle" semantics.
    function focusInOtherTextInput(target) {
      if (!(target instanceof Element)) return false;
      if (target === input) return false;
      return target.matches("input, textarea, [contenteditable], [contenteditable='true']");
    }

    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && isOpen()) {
        e.preventDefault();
        close();
        return;
      }
      const isCmdK = (e.metaKey || e.ctrlKey) && (e.key === "k" || e.key === "K");
      if (isCmdK) {
        if (focusInOtherTextInput(e.target)) return;
        e.preventDefault();
        if (isOpen()) close();
        else open();
      }
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      initSidebar();
      initDropdowns();
      initBulkSelect();
      initFkAutocomplete();
      initSearchPalette();
    });
  } else {
    initSidebar();
    initDropdowns();
    initBulkSelect();
    initFkAutocomplete();
    initSearchPalette();
  }
})();
