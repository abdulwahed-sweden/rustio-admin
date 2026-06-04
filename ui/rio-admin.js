/* ==========================================================================
   RustIO UI — minimal interactivity
   --------------------------------------------------------------------------
   No dependencies, no localStorage. Wires data-attributes to the
   .is-open / .is-active state classes that rio-admin.css styles.

   Markup hooks:
     Dropdown : <div class="rio-dropdown"><button data-rio-dropdown>…</button>
                <div class="rio-dropdown-menu">…</div></div>
     Modal    : opener  <button data-rio-modal-open="#id">
                close    [data-rio-modal-close] / backdrop click / Esc
     Tabs     : <div class="rio-tabs"><button data-rio-tab="#panel">…</button>…</div>
                panels   <div id="panel" class="rio-tab-panel">…</div>
     Toast    : <button data-rio-toast="success" data-rio-toast-title="…"
                        data-rio-toast-text="…">  — or  RIO.toast({…})
   ========================================================================== */
(function () {
  'use strict';

  /* ---- Dropdown ---------------------------------------------------------- */
  document.addEventListener('click', function (e) {
    var toggle = e.target.closest('[data-rio-dropdown]');
    var open = document.querySelectorAll('.rio-dropdown.is-open');
    if (toggle) {
      var dd = toggle.closest('.rio-dropdown');
      var wasOpen = dd.classList.contains('is-open');
      open.forEach(function (m) { m.classList.remove('is-open'); });
      if (!wasOpen) { dd.classList.add('is-open'); }
      return;
    }
    if (!e.target.closest('.rio-dropdown-menu')) {
      open.forEach(function (m) { m.classList.remove('is-open'); });
    }
  });

  /* ---- Modal ------------------------------------------------------------- */
  function closeModal(m) { if (m) { m.classList.remove('is-open'); } }
  document.addEventListener('click', function (e) {
    var opener = e.target.closest('[data-rio-modal-open]');
    if (opener) {
      var m = document.querySelector(opener.getAttribute('data-rio-modal-open'));
      if (m) { m.classList.add('is-open'); }
      return;
    }
    if (e.target.closest('[data-rio-modal-close]')) {
      closeModal(e.target.closest('.rio-modal-backdrop'));
      return;
    }
    if (e.target.classList.contains('rio-modal-backdrop')) { closeModal(e.target); }
  });

  /* ---- Tabs -------------------------------------------------------------- */
  document.addEventListener('click', function (e) {
    var tab = e.target.closest('[data-rio-tab]');
    if (!tab) { return; }
    var group = tab.closest('.rio-tabs');
    group.querySelectorAll('.rio-tab').forEach(function (t) { t.classList.remove('is-active'); });
    tab.classList.add('is-active');
    var scope = group.parentElement;
    scope.querySelectorAll('.rio-tab-panel').forEach(function (p) { p.classList.remove('is-active'); });
    var panel = scope.querySelector(tab.getAttribute('data-rio-tab'));
    if (panel) { panel.classList.add('is-active'); }
  });

  /* ---- Esc closes open modals + dropdowns ------------------------------- */
  document.addEventListener('keydown', function (e) {
    if (e.key !== 'Escape') { return; }
    document.querySelectorAll('.rio-modal-backdrop.is-open').forEach(closeModal);
    document.querySelectorAll('.rio-dropdown.is-open').forEach(function (d) { d.classList.remove('is-open'); });
  });

  /* ---- Toast ------------------------------------------------------------- */
  var ICON = {
    default: 'bi-info-circle-fill', success: 'bi-check-circle-fill',
    danger: 'bi-exclamation-octagon-fill', warning: 'bi-exclamation-triangle-fill'
  };
  function host() {
    var h = document.querySelector('.rio-toast-host');
    if (!h) { h = document.createElement('div'); h.className = 'rio-toast-host'; document.body.appendChild(h); }
    return h;
  }
  function toast(opts) {
    opts = opts || {};
    var variant = opts.variant || 'default';
    var el = document.createElement('div');
    el.className = 'rio-toast' + (variant !== 'default' ? ' rio-toast-' + variant : '');
    el.innerHTML =
      '<i class="bi ' + (ICON[variant] || ICON.default) + ' rio-toast-glyph"></i>' +
      '<div class="rio-toast-body">' +
        (opts.title ? '<div class="rio-toast-title"></div>' : '') +
        (opts.text ? '<div class="rio-toast-text"></div>' : '') +
      '</div>' +
      '<button class="rio-toast-close" type="button" aria-label="Dismiss"><i class="bi bi-x-lg"></i></button>';
    if (opts.title) { el.querySelector('.rio-toast-title').textContent = opts.title; }
    if (opts.text) { el.querySelector('.rio-toast-text').textContent = opts.text; }
    function close() { el.classList.add('is-hiding'); setTimeout(function () { el.remove(); }, 200); }
    el.querySelector('.rio-toast-close').addEventListener('click', close);
    host().appendChild(el);
    if (opts.duration !== 0) { setTimeout(close, opts.duration || 4000); }
    return el;
  }
  document.addEventListener('click', function (e) {
    var t = e.target.closest('[data-rio-toast]');
    if (!t) { return; }
    toast({
      variant: t.getAttribute('data-rio-toast') || 'default',
      title: t.getAttribute('data-rio-toast-title') || '',
      text: t.getAttribute('data-rio-toast-text') || ''
    });
  });

  window.RIO = { toast: toast };
})();
