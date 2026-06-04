# Releasing

Cutting a new `rustio-admin` release (e.g. `0.23.0` → `0.24.0`). All four
crates ship the same version.

## 1. Bump the version (`OLD` → `NEW`)

Update every pin — two of these are enforced by CI guards, so a miss fails the build:

- [ ] `Cargo.toml` — `[workspace.package].version`, **and** both
      `[workspace.dependencies]` pins (`rustio-admin`, `rustio-admin-macros`).
- [ ] `crates/rustio-admin-cli/Cargo.toml` — the inline `version`, the pin
      comment, and the `rio-theme` dependency `version`.
- [ ] `crates/rustio-admin-cli/templates/project/Cargo.toml.tmpl` — the
      scaffold pin. *(CI: “scaffold template pin tracks workspace minor”.)*
- [ ] `examples/clinic/Cargo.toml` — `[workspace.dependencies]`
      `rustio-admin`. *(CI: “reference example pin tracks workspace version”.)*
- [ ] `README.md` — the `rustio-admin = "NEW"` dependency snippet.
- [ ] `cargo build --workspace` to regenerate `Cargo.lock`.

Leftover check: `git grep -n '"OLD"' -- ':!CHANGELOG.md'` (historical prose
in README is fine to leave).

## 2. Roll the CHANGELOG

- [ ] Rename `## [Unreleased]` → `## [NEW] — YYYY-MM-DD`; add a fresh empty
      `## [Unreleased]`.
- [ ] Update the dated row in the "Releases at a glance" table.
- [ ] Call out any audit-impacting change under an `### Audit` subsection.

## 3. Verify (matches CI)

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build  --workspace --all-targets
cargo test   --workspace --all-targets
# reference example builds against HEAD:
( cd examples/clinic && cargo build --workspace )
# Tier-2 guard (expect no matches):
git grep -nE 'HasSchema|ModelSchema|RustType|SchemaOps|from_schema|contract_validator|contract_doctor|RustioModel' \
    -- 'crates/' 'examples/' 'Cargo.toml' ':(exclude,glob)crates/*/assets/**'
```

## 4. Commit & tag

- [ ] `git commit` — `chore(release): cut NEW`
- [ ] `git tag -a vNEW -m "rustio-admin NEW — <headline>"`
- [ ] `git push --follow-tags`

## 5. Publish (only when ready — crates.io is permanent)

Dry-run, then publish in **dependency order** (each `cargo publish` waits
for the index before the next resolves):

```sh
for c in rustio-admin-macros rio-theme rustio-admin rustio-admin-cli; do
  cargo publish --dry-run -p "$c"   # macros/rio-theme verify fully; the
done                                # other two "fail" only on the unpublished
                                    # dep — expected, resolves during real publish
cargo publish -p rustio-admin-macros \
  && cargo publish -p rio-theme \
  && cargo publish -p rustio-admin \
  && cargo publish -p rustio-admin-cli
```

- [ ] Verify each crate is live (e.g. `curl -s https://crates.io/api/v1/crates/<name>/NEW`).

A mistake on a published version can only be `cargo yank`'d (never deleted),
then fixed in a patch release.
