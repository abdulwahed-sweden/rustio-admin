<!--
  Pull Request template for rustio-admin.

  Read DESIGN_SYSTEM.md before opening a PR that touches CSS, tokens,
  templates, or authority code.
-->

## Summary

<!-- One paragraph: what the PR does and why. Don't paste the diff. -->

## Type of change

<!-- Check every box that applies. -->

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (API or visual surface changed for existing consumers)
- [ ] Documentation only
- [ ] Refactor / dependency / tooling

## Token disclosure

<!--
  REQUIRED if this PR modifies any of:

    - crates/rustio-admin/assets/static/admin.css
    - any file containing --rio-* token definitions
    - font-family declarations or @font-face blocks
    - any :root { ... } block in framework or downstream-installed CSS

  Otherwise write `n/a` and remove this section's body.

  See DESIGN_SYSTEM.md §2 (Token ownership) before answering.
-->

**Tokens changed:**
<!-- e.g. --rio-accent (#A0341A → #0F8C7E), --rio-info-bg
        Or: n/a -->

**Migration impact for downstream projects:**
<!-- Will any project see a visible change after `cargo update`? Identify
     surfaces (login/list/edit/dashboard/group permissions). If projects
     need to update their own CSS, say so. Or: n/a -->

**Visual regression risk:**
<!-- Which surfaces would look different in this branch vs main? Be
     specific: "the dark-mode danger button background" rather than "dark
     mode". Or: n/a -->

## Visual regression checklist

<!--
  REQUIRED for any PR that changes admin.css, admin/* templates, or any
  rendered HTML. Walk every box on a logged-in admin in a real browser.
  Do NOT just check the box — actually look. UI hiding is reflection,
  not security; visual review is reflection, not approval.

  See DESIGN_SYSTEM.md §5.2.
-->

- [ ] Login page
- [ ] Dashboard
- [ ] Tables (list view + bulk select bar)
- [ ] Forms (create + edit + validation error states)
- [ ] Dark mode (toggle in your OS or `data-rio-theme="dark"`)
- [ ] Arabic rendering on at least one Arabic-content page
- [ ] Mobile width (≤ 480px)
- [ ] Permission matrix (`/admin/groups/<id>/edit`)

## Test plan

<!--
  - List the unit / integration tests added or modified.
  - For UI work, name the pages you walked.
  - For authority work, name the guards and the test cases that exercise
    each branch (block + allow).
-->

- [ ] `cargo fmt --all`
- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`

## CHANGELOG

<!--
  Did you add an entry under [Unreleased] in CHANGELOG.md? Token changes
  must be documented; behaviour changes must be documented; pure
  refactors do not need an entry.
-->

- [ ] CHANGELOG.md updated, or n/a (refactor / docs / test-only)

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)
