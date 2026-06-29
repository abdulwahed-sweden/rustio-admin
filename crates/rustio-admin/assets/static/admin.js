/* rustio-admin client-side helpers — minimal on purpose.
 *
 *  - RustIO Console chrome: theme toggle + collapsible command rail.
 *  - Generic dropdown wiring, bulk-select form helper,
 *    foreign-key autocomplete, global ⌘K search palette.
 *
 * Sortable column headers and remote filter widgets land in P9/P10.
 */
(function () {
  "use strict";

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

  // ---- Row-actions kebab menu -------------------------------------
  // Each list-table row carries a <details class="rio-row-actions">
  // whose <summary> is the "⋯" toggle. The native <details> already
  // opens the menu (and the links work) with JS off — this function
  // upgrades the experience when JS runs:
  //
  //   * the open menu is positioned `fixed`, anchored to the toggle's
  //     bounding rect, so it escapes the list card's `overflow:
  //     hidden` instead of being clipped below the last row;
  //   * only one row menu is open at a time;
  //   * outside-click, Esc, scroll, and resize all close it;
  //   * ArrowUp/ArrowDown move between menu items.
  //
  // Positioning is recomputed on open only — the scroll handler
  // closes the menu rather than chasing the toggle, which is the
  // calmer behaviour for a transient row menu.
  function initRowActions() {
    const menus = Array.from(document.querySelectorAll("[data-rio-row-actions]"));
    if (!menus.length) return;

    let openMenu = null;

    function placeMenu(details) {
      const toggle = details.querySelector(".rio-row-actions__toggle");
      const panel = details.querySelector(".rio-row-actions__menu");
      if (!toggle || !panel) return;

      panel.classList.add("is-floating");
      // Measure after the panel is displayed so width/height are real.
      const btn = toggle.getBoundingClientRect();
      const panelRect = panel.getBoundingClientRect();
      const gap = 4;
      const margin = 8;

      // Default: menu hangs below the toggle, right edges aligned.
      let top = btn.bottom + gap;
      let left = btn.right - panelRect.width;

      // Flip above the toggle if it would run past the viewport.
      if (top + panelRect.height > window.innerHeight - margin) {
        top = btn.top - panelRect.height - gap;
      }
      // Keep the menu inside the viewport horizontally.
      if (left < margin) left = margin;
      const maxLeft = window.innerWidth - panelRect.width - margin;
      if (left > maxLeft) left = maxLeft;

      panel.style.top = `${Math.max(margin, top)}px`;
      panel.style.left = `${left}px`;
    }

    function closeMenu() {
      if (!openMenu) return;
      const details = openMenu;
      openMenu = null;
      details.removeAttribute("open");
      const panel = details.querySelector(".rio-row-actions__menu");
      if (panel) {
        panel.classList.remove("is-floating");
        panel.style.top = "";
        panel.style.left = "";
      }
      const toggle = details.querySelector(".rio-row-actions__toggle");
      if (toggle) toggle.setAttribute("aria-expanded", "false");
    }

    function openOne(details) {
      if (openMenu && openMenu !== details) closeMenu();
      details.setAttribute("open", "");
      openMenu = details;
      const toggle = details.querySelector(".rio-row-actions__toggle");
      if (toggle) toggle.setAttribute("aria-expanded", "true");
      placeMenu(details);
    }

    menus.forEach((details) => {
      const toggle = details.querySelector(".rio-row-actions__toggle");
      if (!toggle) return;
      toggle.setAttribute("aria-expanded", "false");

      // Intercept the native <details> toggle so we drive open/close
      // ourselves — the browser would otherwise open the menu
      // before we can position it as `fixed`.
      toggle.addEventListener("click", (e) => {
        e.preventDefault();
        if (details.hasAttribute("open")) closeMenu();
        else openOne(details);
      });

      // Arrow-key navigation between the menu items once open.
      details.addEventListener("keydown", (e) => {
        if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
        const items = Array.from(
          details.querySelectorAll(".rio-row-actions__item")
        );
        if (!items.length) return;
        e.preventDefault();
        const current = items.indexOf(document.activeElement);
        const step = e.key === "ArrowDown" ? 1 : -1;
        const next = (current + step + items.length) % items.length;
        items[next].focus();
      });
    });

    // Outside-click closes the open menu (clicks on the menu's own
    // links fall through so navigation still happens).
    document.addEventListener("click", (e) => {
      if (!openMenu) return;
      if (!openMenu.contains(e.target)) closeMenu();
    });

    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && openMenu) {
        const toggle = openMenu.querySelector(".rio-row-actions__toggle");
        closeMenu();
        if (toggle) toggle.focus();
      }
    });

    // A `fixed` menu would visually detach from its row on scroll —
    // closing is the honest behaviour. `capture: true` catches
    // scrolls on any nested scroller, not just the window.
    window.addEventListener("scroll", closeMenu, true);
    window.addEventListener("resize", closeMenu);
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

  // ---- RustIO Console chrome: theme toggle + collapsible rail -----
  // The flicker-free <head> script sets the initial data-theme; this
  // wires the rail's moon/sun toggle (persisted to localStorage) and
  // the rail collapse/expand arrow.
  function syncThemeIcons(theme) {
    const moon = document.querySelector(".rio-theme-moon");
    const sun = document.querySelector(".rio-theme-sun");
    if (!moon || !sun) return;
    const dark = theme === "dark";
    moon.style.display = dark ? "none" : "";
    sun.style.display = dark ? "" : "none";
  }
  function initConsole() {
    const themeBtn = document.getElementById("themeToggle");
    if (themeBtn) {
      themeBtn.addEventListener("click", () => {
        const next =
          document.documentElement.getAttribute("data-theme") === "dark"
            ? "light"
            : "dark";
        document.documentElement.setAttribute("data-theme", next);
        try { localStorage.setItem("rio-theme", next); } catch (e) {}
        syncThemeIcons(next);
      });
    }
    syncThemeIcons(document.documentElement.getAttribute("data-theme") || "light");

    const rail = document.getElementById("rail");
    const railBtn = document.getElementById("railToggle");
    if (rail && railBtn) {
      // Labeled by default (matches the reference); collapse to the
      // icon-only rail only when the operator has chosen it before.
      try {
        if (localStorage.getItem("rio-rail-open") === "0") {
          rail.classList.remove("rio-rail--open");
          railBtn.setAttribute("aria-expanded", "false");
        } else {
          rail.classList.add("rio-rail--open");
          railBtn.setAttribute("aria-expanded", "true");
        }
      } catch (e) {}
      railBtn.addEventListener("click", () => {
        const open = rail.classList.toggle("rio-rail--open");
        railBtn.setAttribute("aria-expanded", String(open));
        try { localStorage.setItem("rio-rail-open", open ? "1" : "0"); } catch (e) {}
      });
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      initConsole();
      initDropdowns();
      initRowActions();
      initBulkSelect();
      initFkAutocomplete();
      initSearchPalette();
    });
  } else {
    initConsole();
    initDropdowns();
    initRowActions();
    initBulkSelect();
    initFkAutocomplete();
    initSearchPalette();
  }
})();
