# Reference snapshots

Copies of the RustIO stylesheets this study compares against, by commit,
taken on 2026-10-09. Documents, not product CSS.

| File | Source | Commit | Status |
|---|---|---|---|
| `rustio/main-00b933c-admin.css` | `rustio-core/assets/static/admin.css` on `main` | `00b933c` | **authoritative** (v3) |
| `rustio/pr6-admin.css` | the same file on `claude/awesome-cerf-7tx0ee` before the squash-merge | `2887db4` | byte-identical to the `main` snapshot; kept for the v2 record |
| `rustio/pr5-admin.css` | `admin.css` on `fix/admin-ui-system-redesign` | `884bf3b` | **unmerged, not authoritative**; kept for the v2 record and `CRITIQUE.md` §H |
| `rustio/pr5-admin-composition.css` | `admin-composition.css` on the same branch | `884bf3b` | as above |

The CURRENT frames on the boards use rustio-admin's own 30 CSS fragments at
`d4b61fa`, concatenated in the order `routes.rs` includes them, comments
stripped. That bundle is derived from this repository and is not stored
here; the boards embed it.
