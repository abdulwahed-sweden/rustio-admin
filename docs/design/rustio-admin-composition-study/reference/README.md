# Reference snapshots

Copies of the RustIO stylesheets this study compares against, taken from the
named commits on 2026-10-09. They are **documents**, kept so the comparison is
against what actually renders on each branch rather than against a package's
own `reference/admin.css` (which, on PR #6, differs from the shipped file —
`CRITIQUE.md` §B.11).

| File | Source | Commit |
|---|---|---|
| `rustio/pr5-admin.css` | `rustio-core/assets/static/admin.css` on `fix/admin-ui-system-redesign` | `884bf3b` |
| `rustio/pr5-admin-composition.css` | `rustio-core/assets/static/admin-composition.css` on the same branch (appended to `admin.css` by `build.rs`) | `884bf3b` |
| `rustio/pr6-admin.css` | `rustio-core/assets/static/admin.css` on `claude/awesome-cerf-7tx0ee` | `2887db4` |

RustIO `main` at `8a6da60` is the pre-composition admin; its stylesheet is
not snapshotted because neither implementation is based on keeping it.

The CURRENT frames on the boards use rustio-admin's own 30 CSS fragments at
`d4b61fa`, concatenated in the order `routes.rs` includes them, with comments
stripped. That bundle is derived from this repository and is not stored
here; the boards embed it.
