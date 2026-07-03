# RustIO Versioning

How `rustio-admin` and its sibling crates are versioned, tagged, and published.
The step-by-step release procedure lives in [`RELEASING.md`](../RELEASING.md);
this document is the policy behind it.

## Unified workspace versions

These four crates share one `[workspace.package].version` and are released
together:

- `rustio-admin`
- `rustio-admin-macros`
- `rustio-admin-cli`
- `rio-theme`

They should usually carry the same version and be published as a set. A change
to any of them advances the shared version.

## Current public version

- The current published version is **0.30.0** (all four crates).
- `main` may contain unreleased work committed after the `v0.30.0` tag. When it
  does, the code on `main` is ahead of the last published/tagged version even
  though the declared version has not yet been bumped.

## Next intended version

- The next intended version is **0.31.0**.
- Reason: significant feature work plus breaking dependency/MSRV changes since
  `0.30.0` (in a `0.x` line, breaking changes advance the minor).
- Do **not** use a patch (`0.30.1`) for this delta — it is not a patch-level change.

## Semantic versioning

The project follows [SemVer](https://semver.org/). While on the `0.x` line, a
breaking change advances the **minor** (`0.30 → 0.31`) and a backward-compatible
fix advances the **patch** (`0.31.0 → 0.31.1`).

## Tags

- Use **annotated `vX.Y.Z`** tags (e.g. `v0.31.0`).
- Do not tag until the release checks in [`RELEASING.md`](../RELEASING.md) pass.
- GitHub Releases should match tags going forward.

## crates.io

- Published crates.io versions are **permanent** — they cannot be overwritten.
- Do not publish without a `cargo publish --dry-run` and explicit owner approval.
- Publish in dependency order:
  1. `rustio-admin-macros`
  2. `rio-theme`
  3. `rustio-admin`
  4. `rustio-admin-cli`

## Related projects

These are versioned independently of the `rustio-admin` workspace:

- **`rustio-core`** — the earlier RustIO line has its own (older, `2.x`) version
  history. See [project-status.md](./project-status.md).
- **`rustio-draft`** — a separate companion repository with its own release
  cadence: <https://github.com/abdulwahed-sweden/rustio-draft>.
- **`rustio-pro-*`** — reserved; it will get its own versioning policy if and
  when it exists.
