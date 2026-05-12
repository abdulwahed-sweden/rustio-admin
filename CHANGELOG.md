# Changelog

All notable changes to `rustio-admin` are recorded here. Format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
the project adheres to [SemVer](https://semver.org/) once it
leaves the alpha track.


## Releases at a glance

| Version   | Date       | Headline                                                                          |
|-----------|------------|-----------------------------------------------------------------------------------|
| **0.10.0** | 2026-05-13 | Flagship example replacement. `examples/minimal/` retired; `examples/library-circulation/` ships as the canonical demo (4 models, 3 FKs, 135-row deterministic seed). Macro learns `Option<DateTime<Utc>>`. Documentation topology consolidated into `docs/design/` + `docs/archive/`. Zero framework runtime change. |
| **0.9.0** | 2026-05-12 | Surface declaration. Doctrine moved to `docs/`; cascade lockstep invariant CI-enforced; 419/419 pub items annotated `// public:` or `// internal:` (355 + 64); `docs/public-api.md` enumerates the public surface. Zero behavioural / visibility changes. |
| **0.8.2** | 2026-05-12 | Admin stylesheet split into a Primer/Carbon-style multi-file source tree + `DESIGN_DOCTRINE.md`. Pure refactor — bundle byte-stream and visual output preserved; one HTTP request, baked into the binary. |
| **0.8.1** | 2026-05-11 | Visibility recovery pass — model_name slug canonicalisation, error-page chrome, history-label expansion, scaffold middleware + secret-key + README, top-bar MFA, doctor surface, `#[rustio(display_name)]`. |
| **0.8.0** | 2026-05-11 | R4 — CLI emergency recovery: `rustio user reset-password / unlock / disable-mfa / promote / emergency-access`. |
| **0.7.1** | 2026-05-11 | Embed every R2 + R3 page template (fix 500 on /admin/reauth and every MFA flow).  |
| **0.7.0** | 2026-05-11 | TOTP multi-factor authentication + single-use backup codes.                       |
| **0.6.0** | 2026-05-10 | Admin-driven recovery, re-auth wall, login throttling, forced password rotation.  |
| **0.5.0** | 2026-05-09 | Self-service password recovery, active-session controls.                          |
| **0.4.0** | 2026-05-09 | Session lifecycle, centralised invalidation, audit foundations.                   |
| **0.3.0** | 2026-05-08 | Authority guards, design-system stabilisation, audit on user/group writes.        |
| **0.2.1** | 2026-05-07 | CLI scaffold-template fix.                                                        |
| **0.2.0** | 2026-05-07 | List-view toolbar, bulk actions, theme architecture, dark mode.                   |
| **0.1.1** | 2026-05-07 | Self-hosted fonts, typography token system.                                       |
| **0.1.0** | 2026-05-07 | Initial public release.                                                           |


## [Unreleased]

No changes yet.


## [0.10.0] — 2026-05-13

Flagship example replacement, plus a narrow macro fix and a
documentation-topology cleanup that piled up since 0.9.0.
**No framework runtime change, no schema migration, no public
API change.** The privatisation pass originally slated for 0.10.0
in `docs/RUSTIO_STRATEGY.md` is deferred to a later minor — this
release is the consolidation that earns it.

### Migration from 0.9.0

Bump `rustio-admin = "0.10.0"` and run
`cargo update -p rustio-admin`. No source change required in
downstream projects. The new example uses only the public API
declared in `docs/public-api.md`; consuming projects continue to
work unchanged.

No schema columns added or removed. No HTTP routes added or
removed. No middleware order changes. No CSS / token / cascade
changes. No public-API signature changes.

### Added

- **`examples/library-circulation/`** — the framework's new
  canonical demo. Four models (`Branch`, `Patron`, `Item`,
  `Loan`), three foreign keys, five SQL migrations including a
  135-row deterministic seed. Boots via `cargo run -p
  library-circulation` against a local Postgres + a superuser
  bootstrapped through `rustio user create`. Designed for
  contributor approachability over business-domain realism;
  the 10-step linear `main.rs` is the canonical "how do I wire
  up a rustio-admin app?" reference.

  Documented inline at `examples/library-circulation/README.md`.
  See the file's "What this example does not yet demonstrate"
  section for the two framework gaps it intentionally surfaces.

- **`docs/design/`** subdirectory. Seven long-form design specs
  moved from repo root: `DESIGN_AUDIT.md`, `DESIGN_R2_ORGANISATIONAL.md`,
  `DESIGN_R3_MFA.md`, `DESIGN_R4_EMERGENCY.md`, `DESIGN_RECOVERY.md`,
  `DESIGN_SESSIONS.md`, `DESIGN_SYSTEM.md`. Filenames preserved
  so the ~50 source-comment references (e.g., `// see
  DESIGN_R2_ORGANISATIONAL.md §10.3`) remain grep-resolvable.

- **`docs/archive/`** additions. The two superseded planning
  documents (`rustio-admin-strategic-reset-plan.md`,
  `rustio-admin-apis-and-docs-plan.md`) are archived alongside
  `VISIBILITY_AUDIT.md`. Repo-root markdown is now exactly four
  files: `README.md`, `CHANGELOG.md`, `LICENSE`, `ROADMAP.md`.

### Changed

- **`#[derive(RustioAdmin)]` accepts `Option<DateTime<Utc>>`
  fields.** Previously rejected with "unsupported field type for
  RustioAdmin: Option<DateTime<Utc>>" — asymmetric with the
  existing `Option<String>` and `Option<i64>` support. The fix
  is symmetric: `FieldKind::OptionalDateTime` added to the macro,
  mapped to the framework's already-existing
  `FieldType::OptionalDateTime` variant. Non-optional `DateTime<Utc>`
  behaviour unchanged; `Option<String>` and `Option<i64>`
  unchanged; other `Option<T>` types still fail with the same
  error message.

- **Repo-root `README.md`** updated for the new docs topology.
  New "Documentation" section near Install lists the canonical
  paths (`docs/`, `docs/design/`, `docs/public-api.md`,
  `docs/archive/`). All design-doc links migrated from
  `./DESIGN_FOO.md` to `./docs/design/DESIGN_FOO.md`. The
  "Reading paths" example link migrated from
  `examples/minimal/` to `examples/library-circulation/`.

- **`docs/README.md`** updated to index the moved design specs
  and archive entries.

### Removed

- **`examples/minimal/`** — the previous "hello-world" example.
  Superseded by `examples/library-circulation/`. Git history
  preserves the four files for anyone who wants the
  pre-replacement skeleton (`git show v0.9.0:examples/minimal/...`).

### Deferred (documented framework gaps)

Two framework capabilities the new example surfaced as
unreachable from project code today. Both are deliberately not
patched around in this release; future minors may address them
with narrow framework changes:

- **Custom bulk-action implementations.** The framework's
  `AdminOps::execute_bulk_action` dispatch trait is `pub(crate)`
  (`crates/rustio-admin/src/admin/types.rs:232`); project code
  cannot override it. Declaring `ModelAdmin::bulk_actions()`
  without a working dispatch would render dead buttons.
- **Idempotent permission-group seeding.**
  `permissions::create_group` is not idempotent and no public
  `ensure_group` / `find_group_by_name` helper exists.

Both deferrals are documented inline in
`examples/library-circulation/migrations/0005_seed.sql`'s footer
and in the example README's "What this example does not yet
demonstrate" section.

### Internal

- **CHANGELOG ordering** continues to flip pre-existing
  `[Unreleased]` into the new versioned section; "Releases at a
  glance" gains a 0.10.0 row.


## [0.9.0] — 2026-05-12

Surface-declaration release. The framework's public API surface is
now explicitly enumerated; every `pub` item in `rustio-admin` and
`rustio-admin-macros` carries a stability declaration. **No
runtime behaviour, signatures, visibility, or templates changed
in this release** — the work is descriptive, not destructive.

This is the consolidation phase of the path to 1.0.0 documented
in `docs/RUSTIO_STRATEGY.md`. The privatisation pass that acts on
the annotations lands in 0.10.0.

### Migration from 0.8.2

Bump `rustio-admin = "0.9.0"` and run
`cargo update -p rustio-admin`. No source change required in
downstream projects. The annotation comments are inline source-
tree metadata; they do not affect compilation, runtime, or the
served HTTP / CSS / template surfaces.

No schema columns added or removed. No HTTP routes added or
removed. No middleware order changes. No CSS / token / cascade
changes.

### Added

- **`docs/RUSTIO_STRATEGY.md`** + **`docs/DESIGN_DOCTRINE.md`**.
  All human-meant prose documentation moved into `docs/`; root-
  level Markdown sprawl reduced. `VISIBILITY_AUDIT.md` archived
  under `docs/archive/`. Repo root now reads as `README.md +
  CHANGELOG.md + LICENSE + Cargo.toml` plus the new top-level
  `RUSTIO_STRATEGY.md` working file.

- **Cascade lockstep CI test**
  (`crates/rustio-admin/tests/cascade_lockstep.rs`, ≤50 LOC).
  Asserts the `@import url(...)` order in
  `assets/static/admin/admin.css` matches the `include_str!(...)`
  order in `ADMIN_CSS` in `src/admin/routes.rs`. Any future PR
  that drifts the two lists fails the test with a readable
  side-by-side diff. Replaces the previous "comments + code
  review" enforcement of the framework's most fragile invariant.

- **`docs/public-api.md`**. Generated enumeration of every
  `// public:` item across the workspace. Grouped by crate /
  module path. Includes the explicit note that the document is
  descriptive, not normative — annotation does not itself
  guarantee SemVer stability before 1.0.

### Changed

- **Every `pub` item in `crates/rustio-admin/src/` and
  `crates/rustio-admin-macros/src/` is now annotated with exactly
  one of `// public:` or `// internal:`.** The annotation lives on
  the line above any `///` doc block, so rustdoc output and
  doc-comment grouping are unaffected.

  - `// public:` — intended stable surface toward 1.0.
  - `// internal:` — `pub` today, candidate for `pub(crate)` in
    0.10.0. Used only where confidence is high (items inside
    `pub(crate) mod` sub-modules reached only via the doc-hidden
    `__integration` test door, plus a small number of plumbing
    constructors and one renderer-internal probe).

  Coverage: **419 / 419** declared `pub` items classified.
  Distribution: **355 `// public:` + 64 `// internal:`**.

  The annotation pass is descriptive. No `pub` was changed to
  `pub(crate)` in 0.9.0. The privatisation pass lands in 0.10.0,
  giving downstream projects one minor release of advance notice
  of which items become private.

### Internal

- **`pub(crate)` declaration count** is unchanged across the full
  Phase C pass (183 framework-wide, identical to the start of
  0.9.0 work). The annotation pass introduced zero visibility
  changes.

- **Cascade order is now a CI-enforced invariant.** Refactoring
  the `concat!(include_str!, …)` block in `routes.rs` without the
  matching `@import` update in `admin.css` fails the new
  integration test before any byte of CSS reaches a browser.


## [0.8.2] — 2026-05-12

Pure refactor of the admin stylesheet source layout. The 2089-line
single-file `admin.css` is split into a Primer / IBM Carbon-style
multi-file architecture under `assets/static/admin/` (tokens →
themes → base → layout → components → pages → responsive → print,
32 fragments + the contributor-facing `admin.css` `@import`
manifest). A new top-level `DESIGN_DOCTRINE.md` extracts the
visual philosophy — token rules, typography system, surface
hierarchy, spacing scale, dark-mode philosophy, operational UI
principles, source layout, contributor workflow.

**Zero visual change.** Every class name, custom-property name,
selector, declaration, comment, and rationale block from the
original file is preserved. Verified by sorting the
comment-stripped rule streams of the original and the bundled
fragments and diffing — zero deletions; the only additions are
the 4 extra `:root` blocks introduced by splitting the original
token block across 5 token files.

### Migration from 0.8.1

Bump `rustio-admin = "0.8.2"` and run
`cargo update -p rustio-admin`. **No template edit required**:
the served URL stays `/static/admin.css` and the rendered bundle
is byte-equivalent to the 0.8.1 bundle (modulo the `:root`
splitting and the section-header comments added to every
fragment).

No schema columns added or removed. No HTTP routes added or
removed. No public API change in `rustio-admin`,
`rustio-admin-macros`, or `rustio-admin-cli`. No middleware order
changes.

### Changed

- **CSS source layout reorganised.** Contributors now author
  fragments under `crates/rustio-admin/assets/static/admin/`
  instead of editing one 2089-line file. Layout follows the
  Primer / Carbon convention so contributors landing from those
  ecosystems can find their way without a README. Each fragment
  carries an `==========` section header explaining its role and
  cascade dependencies. Original rationale comments preserved
  verbatim where they belong.
- **Delivery uses `concat!(include_str!, …)`.** `ADMIN_CSS` in
  `src/admin/routes.rs` switched from a single `include_str!` to
  a compile-time `concat!` over every fragment in cascade order.
  The browser still receives one bundle at `/static/admin.css` —
  one HTTP request, baked into the binary, no runtime `@import`
  waterfall, no extra routes. Self-hosted / no-FOUT / no-CDN
  doctrine preserved.

### Added

- **`DESIGN_DOCTRINE.md`** at the repo root. Ten sections
  explaining the *why* behind RustIO's visual identity — what
  every token means, when to use which surface rung, how the
  dark palette diverges from the light, what "operational UI"
  means in practice, how the bundle is assembled, and the
  workflow for adding a new fragment. Intended as the canonical
  reference for design decisions in the framework.

### Internal

- **Cascade order documented in two places, kept in lock-step.**
  The `@import` list in `admin.css` and the `concat!` block in
  `routes.rs` are the source of truth for fragment order. A
  comment in each file points to the other; both must move
  together when a new fragment is added.
- **No bundler, no PostCSS, no SCSS, no CSS-in-JS.** Pure
  hand-written CSS, baked into the binary at `cargo build` time.
  The framework's "one binary, no toolchain" deploy story is
  unchanged.


## [0.8.1] — 2026-05-11

Visibility recovery pass. No new features, no rewrites — a
focused recovery of already-built framework capabilities that
were hidden, disconnected, or bypassed in the generated-project
surface. Driven by an audit (`VISIBILITY_AUDIT.md`); the audit's
10 findings reduced to 9 implemented commits, 1 (B2 — sidebar
Auth-block unification) deliberately deferred as too
architecture-y for a recovery pass.

The user-visible payoff: a fresh `rustio startproject foo`
project now feels coherent. The History page links resolve.
Error pages keep the chrome. MFA + Sessions surface from the
top-bar. `rustio doctor` answers seven questions instead of
three. `startapp` teaches the operator what `ModelAdmin` does.
The audit log shows real event names instead of opaque pills.

### Migration from 0.8.0

Bump `rustio-admin = "0.8.1"` and run
`cargo update -p rustio-admin`. The framework's boot-time
`admin::audit::ensure_table` gains an idempotent
`UPDATE rustio_admin_actions SET model_name = 'users' WHERE
model_name IN ('User', 'user', 'rustio_users')` migration (plus
the matching one for groups). Rows from 0.7.x and 0.8.0 that
had drifted `model_name` values are rewritten on next boot;
subsequent boots are a no-op.

No schema columns added or removed. No HTTP routes added or
removed. No middleware order changes (the scaffold's template
gains `correlation_id`; lursystem-style production projects
already wire it).

### Fixed

- **F1 — audit-row `model_name` canonicalised to admin slug.**
  Pre-0.8.1 the framework + CLI wrote four different
  conventions (`"User"`, `"user"`, `"Group"`, `"rustio_users"`)
  to `rustio_admin_actions.model_name`. The History page
  renders this column as a URL slug; three of four 404'd. All
  emission sites now write the canonical slug (`"users"` /
  `"groups"`). The user-visible 404 in
  `VISIBILITY_AUDIT.md` screenshot 1
  (`/admin/rustio_users/2/edit` → "no admin model: rustio_users")
  is the closing demonstration.
- **A2 — error page keeps the operator's chrome.** The outer
  error-render middleware now resolves the operator's identity
  from the session cookie before rendering the 4xx/5xx page,
  and `ErrorCtx` gains the `entries` field needed by the
  sidebar `{% include %}`. Pre-0.8.1 the 404 page was a
  chromeless dead-end with no sidebar, no actor chip, no
  log-out link.
- **B3 — `/admin/history` Action column shows real event names.**
  `action_label` covered only `create/update/delete` and fell
  through to a generic "Action" pill for every R1+ event
  (password resets, MFA, emergency recovery). Now covers all
  25 known `AuditEvent::as_str()` strings with human labels +
  semantic pill classes (success / danger / warning / neutral).

### Added — generated-project surface

- **F4 — scaffold middleware chain gains `correlation_id`.**
  Pre-0.8.1 scaffolded projects shipped a 3-middleware chain;
  the R0-canonical chain is 4 (`logger →
  correlation_id → security_headers → csrf_protect`).
  Scaffolded projects without `correlation_id` wrote NULL into
  the audit column, breaking the cross-request pivot.
- **F5 — scaffold `.env.example` gains `RUSTIO_SECRET_KEY=`.**
  With generation command, scope, and rotation warning.
  Pre-0.8.1 a fresh project's first MFA enrol 500'd at the
  AES-GCM init guard.
- **F6 — `rustio startapp` emits meaningful `ModelAdmin`.**
  `list_display` / `list_filter` / `search_fields` /
  `ordering` shown with starter values + per-method docstrings
  + commented-out `list_per_page` / `bulk_actions` stubs.
  Operators discover the framework's strengths from the first
  generated model.
- **D3/D4 — scaffold `README.md` covers MFA + R4 + custom
  routes.** 100+ lines added: how to enable MFA via
  `Admin::require_mfa`, the five `rustio user <op>` emergency
  commands with worked examples, and the
  mount-before-`register_admin_routes` pattern for project
  routes.

### Added — top-bar discoverability

- **B1 — MFA + Sessions self-service links surfaced.** The
  top-bar account area now shows "Enable MFA" (un-enrolled),
  "Two-factor" (enrolled), and "Sessions" links. Pre-0.8.1
  the R3 enrol / regenerate / disable pages were reachable
  only by typing the URL.

### Added — doctor diagnostic

- **E3 — `rustio doctor` reports four more checks.**
  RUSTIO_SECRET_KEY presence + length, R3 MFA enrolment count,
  R4 emergency-recovery audit row count, audit-slug drift (the
  regression gate paired with F1). Hard failures still exit
  non-zero; new checks are informational.

### Added — macro polish

- **F3 — `#[rustio(admin_name = ..., display_name = ...)]`
  struct attributes.** Project-side override for the macro's
  auto-derived labels. `CaseAction` can now register as
  "Case events" without renaming the struct. Both keys
  optional; unknown keys produce a compile error.

### Regression gates added

- `admin::audit::tests::model_name_uses_admin_slug_not_struct_name` —
  scans every framework `.rs` for legacy `model_name` literals
  in audit-emission contexts; fails if any reappear.
- `admin::render::tests::action_label_covers_every_audit_event_string` —
  asserts no event string falls through to the generic "Action"
  label.
- `admin::render::tests::action_pill_class_returns_known_classes` —
  asserts every pill class is one the CSS defines.

### Deliberately deferred

- **B2 — sidebar Auth-block unification.** The audit flagged
  the parallel "Models" loop + hardcoded "Users / Groups /
  History" block as a structural smell. The hardcoded block
  works correctly today; restructuring it is more risk than
  reward for a recovery pass. Future work, lower priority.

### Test count

  Before: 259 framework + 27 CLI = 286
  After:  262 framework + 27 CLI = 289 (+3)


## [0.8.0] — 2026-05-11

R4 — CLI emergency recovery. The shell-access tier that opens
when every in-band recovery path is closed: a founder who
forgot her password AND lost her TOTP device AND is the only
Administrator on the deployment cannot recover via R1
(no working password), R2 (no admin to act for her), or R3
(no MFA factor). She still has shell access to the machine
running the framework. The CLI is the last-mile recovery
surface that uses that shell access deliberately, audibly,
auditably.

The spec lives in `DESIGN_R4_EMERGENCY.md`. Pull-request
review runs against that doc, not only the diff.

> **Migration from 0.7.x**
>
> Bump `rustio-admin = "0.8"` and run `cargo update -p rustio-admin`.
>
> **No schema migration.** R4 reuses the R0-R3 columns and the
> R1 `rustio_password_reset_tokens` table. Zero new tables, zero
> new columns.
>
> **No HTTP route changes.** R4 is a CLI-only surface;
> `register_admin_routes` is unchanged. Projects that boot via
> the framework's HTTP layer carry forward unmodified.
>
> **No middleware changes.** The R0-locked
> `correlation_id` → `csrf_protect` order still holds.
>
> **One new `SessionInvalidationReason` variant:**
> `RoleChangedByOther` (string `"role_changed_by_other"`).
> Emitted by `rustio user promote` when revoking the target's
> sessions to force a fresh login under the new tier. Reuses
> the existing `rustio_sessions.revoked_reason` TEXT column.
>
> **`AuditEvent::EmergencyRecovery` lights up.** The variant
> has existed since R0/R1 as a reserved slot; R4 promotes it
> to a documented, contract-bearing variant emitted by exactly
> five callsites in `rustio-admin-cli`. The framework crate
> must NOT emit it — the
> `admin::audit::tests::emergency_recovery_is_cli_only` unit
> test fails the default `cargo test --workspace` gate on any
> such regression.
>
> **Doctrine 22 holds.** `auth::sessions::invalidate_sessions`
> remains the sole writer of `revoked_at`. R4's framework-side
> primitives call through the centralised invalidator for every
> session-revoke. Doctrines 1-18 + D1-D8 (R3) all unchanged.
>
> R4 adds four new doctrines (D9-D12) governing CLI-actor
> identity, confirmation discipline, atomicity per command, and
> CLI-only emission scope. See `DESIGN_R4_EMERGENCY.md` §10.

### Highlights

- **Five `rustio user <op>` subcommands.** All share the same
  envelope: `--email <e> --reason "<text>"` (≥ 8 chars) plus
  per-op flags. Each renders a red-ANSI confirmation banner
  (target / reason / OS-actor / time / "audited and
  irreversible") and demands interactive `yes` confirm
  (or `--yes` for scripting; banner still prints).

  - `rustio user reset-password [--temp-password <p>] --reason
    "<r>" [--yes]` — Argon2-hashes the new password (CLI-
    generated 20-char ambiguity-stripped alphanumeric if
    `--temp-password` not supplied), sets
    `must_change_password = TRUE`, revokes every session for
    the user. Plaintext temp password prints to stdout exactly
    once.
  - `rustio user unlock --reason "<r>" [--yes]` — clears
    `locked_until` + `failed_login_count`. Does NOT revoke
    sessions (an unlock is not a session event).
  - `rustio user disable-mfa --reason "<r>" [--yes]` —
    clears the four MFA columns, deletes every backup-code
    row, revokes the user's sessions. If `MfaPolicy::Required`
    is set, the user is redirected to re-enrolment on next
    login.
  - `rustio user promote --to-role <r> --reason "<r>" [--yes]` —
    changes the role; revokes the target's sessions so the
    new tier takes effect on next login. Refuses to demote
    the sole active Administrator (returns
    `SoleAdministratorDemoteRefused`; the deployment is never
    left with zero administrators, even via CLI).
  - `rustio user emergency-access [--ttl-minutes <n>] --reason
    "<r>" [--yes]` — issues a single-use password-reset URL
    bypassing the email mailer. URL prints to stdout exactly
    once; operator hands to target out-of-band. Reuses R1's
    `rustio_password_reset_tokens` table; the consume path is
    R1's existing `/admin/reset-password/<token>` handler. TTL
    defaults to 15 min, clamped to `[1, 60]`.

- **Audit-row emission lives in the CLI crate** (D12). Every
  emergency command writes one `AuditEvent::EmergencyRecovery`
  row through a shared `write_emergency_audit` helper.
  `metadata.cli_operation` distinguishes the variant (`"reset_password"
  | "unlock" | "disable_mfa" | "promote" | "emergency_access"`).
  `metadata.os_actor` is `<whoami>@<hostname>`.
  `metadata.cli_invocation` carries argv with `--reason VALUE`
  redacted (the full reason lives in `metadata.reason` as a
  typed field; double-storing would risk leakage through
  logging surfaces). The argv-redaction helper has 4 unit
  tests covering space-form, equals-form, no-flag, and the
  lone-`--reason`-at-argv-end pathological case.

- **Confirmation banner is irreducible** (D10). 64-char box,
  red ANSI header, locked field order
  (Operation / Target / Reason / Operator / Time). Long values
  (e.g. a reason longer than ~47 chars) extend the box's right
  wall — no truncation. Truncating would defeat the banner's
  forensic purpose. ANSI is auto-detected: enabled only when
  stdout is a TTY and `NO_COLOR` is unset.

- **TTY-and-yes-flag gate.** If stdin is not a TTY and
  `--yes` is absent, every emergency command exits with status
  2 + a clear message ("Refusing to run without a TTY (or
  pass --yes for scripting)"). Prevents accidental piping from
  running an emergency op unnoticed.

- **D11 — atomicity per command.** Every framework primitive
  that mutates more than one row runs the mutations inside one
  sqlx transaction. Session revocation runs after commit
  because `invalidate_sessions` is doctrine-22's single writer
  of `revoked_at` and owns its own atomic statement; the
  transaction boundary keeps the mutation isolated from the
  session sweep.

- **Audit pivot via correlation_id.** Every CLI-emitted audit
  row stamps a fresh UUID v7 hyphenated correlation_id matching
  the format the framework's HTTP middleware writes per
  request, so a future cross-table audit pivot can join
  framework rows and CLI rows on the column without per-source
  post-processing.

- **`emergency-access` URL format hardened.** Reuses R1's
  exact token + hash format via `auth::sessions::random_token()`
  + `hash_token_for_storage()` — the CLI-issued URL round-
  trips through R1's existing `/admin/reset-password/<token>`
  consume path identically to a self-service R1 reset URL. The
  load-bearing
  `emergency_access_token_hash_matches_r1_consume_format`
  integration test (testcontainers, runs against a live
  ephemeral Postgres) catches any drift before publish.

- **13-scenario testcontainers integration suite** at
  `crates/rustio-admin/tests/integration_emergency.rs`, gated
  by the existing `integration-test` Cargo feature. Mirrors
  the R2 and R3 suites' shape. Runs in ~41 s on a warm Docker
  daemon. Run with `cargo test --workspace --features
  integration-test`.

### Doctrines

R4 layers four new doctrines onto the existing 22 + D1-D8.
See `DESIGN_R4_EMERGENCY.md` §10 for the full text.

- **D9.** CLI-actor identity is OS-level. `metadata.os_actor`
  is `<whoami>@<hostname>`. No synthetic system-user invention;
  the audit row's target is the user being acted on, and the
  CLI operator is in metadata.
- **D10.** Confirmation banner is irreducible. Every R4
  command prints the banner. `--yes` skips the prompt, not
  the banner. No flag suppresses the banner.
- **D11.** R4 operations are atomic per command. One
  command = one audit row = one transaction (where DB
  operations span multiple statements). A partial failure
  rolls back the mutation. Half-applied state is the worst
  possible outcome for emergency recovery.
- **D12.** `EmergencyRecovery` is CLI-only by code-walk. The
  framework crate must not emit `AuditEvent::EmergencyRecovery`.
  The `admin::audit::tests::emergency_recovery_is_cli_only`
  unit test fails the default test gate on any such
  regression. A future web handler that needs an
  emergency-shaped operation must introduce a new audit
  variant rather than reusing this one.

### Behaviour parity surfaced during the cycle

Commit #8's first live-DB smoke caught a token-hash format
drift: the CLI emitted SHA-256(token) as lowercase hex while
R1's `hash_token_for_storage` produced URL-safe-base64-no-pad.
The URL printed; the consume path 404'd; the user saw "This
link is no longer valid." Fix bundled with commit #8 (single-
line call-through to R1's helper), regression locked in
commit #9's integration suite.

Worth noting for future phases: this bug was invisible to
unit tests because the issue side and the lookup side both
produced *some* string — they just didn't match. The cross-
check is the live DB. A unit-test gate over the framework's
behaviour would have to know about the contract on both sides
to catch it.

### What R4 does NOT ship

- No daemon mode / REPL — every command is one-shot.
- No `--dry-run` flag — the banner + interactive confirm IS
  the dry-run.
- No bulk operations — every command takes exactly one
  `--email`. A bulk requirement is a script around the CLI.
- No undo of emergency operations. Reversal is a fresh
  emergency operation, audited as such.
- No granular permission delegation — possession of
  `DATABASE_URL` is the authority floor. The host's OS-level
  access controls are the gate; R4 does not re-implement
  them.

### Closing principle

R4 makes the last-mile shell-access recovery path
**indistinguishable in the audit log from any other recovery
operation, but distinguishable in event type**. The auditor
sees `EmergencyRecovery` rows and knows the operator went
around every other tier. That visibility — not the difficulty
of running the command — is the regulatory artefact.

R4 closes the recovery-roadmap loop opened in R0. After R4
the framework supports four tiers — self (R1), peer-admin
(R2), two-factor (R3), shell (R4) — and refuses to silently
lose the audit chain at any tier transition.


## [0.7.1] — 2026-05-11

Patch release. Restores the R2 (admin-driven recovery) and R3
(TOTP MFA) page templates to the embedded set. The disk files
shipped in 0.6.0 and 0.7.0 respectively, but the corresponding
`include_str!` entries in `src/templates.rs` were never added —
so default-deployed binaries returned HTTP 500 with the
framework's generic error page on every R2 / R3 handler that
called `templates.render("admin/<name>.html", …)`.

Surfaced by the lursystem v1 (flagship downstream) live-DB
shakedown: Phase 4 (reporter-identity unmask) and every
Phase 3c terminal status transition bounced to
`/admin/reauth` and 500'd because the template was unresolvable
under `Templates::new(None)`.

### Fixed

- `EMBEDDED_TEMPLATES` now includes the ten missing entries:
  `admin/reauth.html`, `admin/admin_reset_password.html`,
  `admin/lock_user.html`, `admin/confirm_admin_action.html`,
  `admin/must_change_password.html`, `admin/mfa_enroll.html`,
  `admin/mfa_enroll_complete.html`, `admin/mfa_verify.html`,
  `admin/mfa_disable.html`, `admin/mfa_regenerate.html`,
  `admin/mfa_regenerate_complete.html`. Every R2 / R3 handler
  that previously 500'd in a default deploy now renders.

### Added

- `every_handler_rendered_template_resolves` unit test under
  `templates::tests`. Walks `src/admin/`, pulls every
  `"admin/<name>.html"` string literal out of every `.rs` file,
  and asserts each one resolves via `Templates::new(None)?`. A
  handler author who adds a new `.render(...)` call but
  forgets the `EMBEDDED_TEMPLATES` entry will fail this test
  before the release ships rather than after the first user
  clicks the affected page. Std-only — no new dependency.

### Migration from 0.7.0

Bump `rustio-admin = "0.7.1"` and `cargo update -p rustio-admin`.
No schema changes. No middleware changes. No env-var changes.
Projects that worked around the gap by copying the templates
into a project-level `templates/` directory can safely remove
the workaround.


## [0.7.0] — 2026-05-11

R3 of the universal account-recovery architecture. TOTP
multi-factor authentication plus single-use backup codes.
Adds the second-factor login flow, self-service enrolment,
backup-code generation + regeneration, self-disable, and the
trust-escalation token rotation that ties the verified
session back to the R0 session model.

> **Migration from 0.6.x**
>
> Bump `rustio-admin = "0.7"` and run `cargo update -p rustio-admin`.
>
> Schema is additive: four new columns on `rustio_users`
> (`mfa_enabled`, `mfa_secret_ciphertext`, `mfa_secret_key_id`,
> `mfa_last_used_step`) plus a new `rustio_mfa_backup_codes`
> table with a per-user partial index on
> `(user_id) WHERE used_at IS NULL`. Existing users are
> unaffected — `mfa_enabled` defaults to `FALSE`; pre-R3 users
> bypass the MFA gates entirely.
>
> **New env var requirement.** `RUSTIO_SECRET_KEY` (32 bytes,
> URL-safe-base64 encoded, no padding) must be set when any
> user has MFA enabled or when the operator opts into
> `MfaPolicy::Required` / `RequiredForRoles`. Used as the
> AES-256-GCM key for TOTP-secret encryption at rest. A
> deployment with `MfaPolicy::Disabled` (or `Optional` with
> zero enrolments) does not need the env var.
>
> No middleware changes — `correlation_id` BEFORE
> `csrf_protect` (set in 0.4.0) is still the only ordering
> constraint. Eight new routes register through
> `register_admin_routes`.
>
> Doctrine 22 holds. `auth::sessions::invalidate_sessions`
> remains the sole writer of `revoked_at` — four hits in the
> grep proof, all inside `auth/sessions.rs::invalidate_sessions`.
> Trust escalation on the MFA-verify path goes through
> `promote_session_to_mfa_verified`, which delegates revocation
> to the centralised invalidator with reason
> `TrustEscalation`.

### Highlights

- **TOTP enrolment.** `GET/POST /admin/account/mfa/enroll` —
  provisions a 20-byte secret, builds an `otpauth://totp/...`
  URL (Google Authenticator Key URI format), renders the
  manual setup key in base32, verifies the user's first
  6-digit code, AES-256-GCM-encrypts and persists. Re-auth
  gated.
- **Login second-factor verify.** `GET/POST /admin/mfa/verify`
  — accepts a 6-digit TOTP code or an `XXXX-XXXX` backup
  code. TOTP tried first; backup-code fallback on
  `VerifyOutcome::Invalid` only (replay attempts collapse to
  uniform failure without consuming a code). Successful
  verify rotates the session to `trust_level = 'mfa_verified'`
  via `promote_session_to_mfa_verified` (D17 token rotation:
  mint new row, revoke parent via `invalidate_sessions`).
- **8 backup codes per user**, `XXXX-XXXX` shape from a
  31-character ambiguity-stripped alphabet (no `0/O/1/I/L`).
  Argon2id-hashed at rest with low-memory params
  (`m = 16 MiB`, `t = 2`, `p = 1`). Single-use enforced at
  three layers: SELECT filters `used_at IS NULL`, atomic
  conditional UPDATE on consume, partial index on the
  unused predicate.
- **Backup-code regeneration.** `POST /admin/account/mfa/regenerate-codes`
  — atomic transaction: `SELECT … FOR UPDATE` on the user
  row serialises concurrent regenerates, DELETE wipes the
  old batch, INSERT lands the new 8. The old batch is
  unrecoverable from the moment the commit lands (D3).
- **Self-disable.** `POST /admin/account/mfa/disable` —
  clears the four MFA columns, deletes the backup-code rows,
  calls `invalidate_sessions(User, MfaDisabled)`. The user
  is signed out of every device; the next sign-in is
  password-only.
- **Re-auth wall demands both factors.** `/admin/reauth`
  requires password AND TOTP (or backup code) when the
  actor has MFA enrolled. Stamps
  `trust_level = 'mfa_verified'` + `elevated_until` in place;
  non-MFA-enrolled users still see the R2 password-only flow
  unchanged.
- **`MfaPolicy::Required` forward-only enforcement.**
  Operators opt in via `Admin::require_mfa(MfaPolicy::Required)`.
  Existing sessions remain valid; existing users without MFA
  are redirected to `/admin/account/mfa/enroll` at the next
  request, restricted to a tiny whitelist (`/admin/account/mfa/enroll`,
  `/admin/logout`, `/admin/account/sessions`) until they
  enrol. Mirrors R2's `must_change_password` interstitial
  shape exactly.
- **Pending-MFA-verify gate.** MFA-enrolled users whose
  current session is still `trust_level = 'authenticated'`
  (post-login, pre-verify window) are restricted to
  `/admin/mfa/verify` + `/admin/logout` +
  `/admin/account/sessions`. Same whitelist shape as the
  enrolment gate.

### Public API

- `auth::MfaPolicy` enum: `Disabled` / `Optional` (default) /
  `Required` / `RequiredForRoles(&'static [Role])`. Plain
  `Copy` value — no `Arc` indirection.
- `Admin::require_mfa(MfaPolicy) -> Self` builder +
  `Admin::active_mfa_policy(&self) -> MfaPolicy` accessor.
- `auth::MfaKey` newtype around `[u8; 32]`. Constructors
  `MfaKey::from_env()` (reads `RUSTIO_SECRET_KEY`) and
  `MfaKey::from_bytes(bytes: [u8; 32])`.
- `auth::mfa` runtime fns (`pub` for the testcontainers
  re-export pattern; module is `pub(crate)`):
  - `provision_secret()` → `ProvisionedSecret { secret_bytes,
    base32 }`.
  - `confirm_enrolment(db, request, user_id, secret_bytes,
    candidate_code, step_seconds, skew_steps, key, key_id,
    correlation_id)` → `EnrolOutcome::{Enrolled, InvalidCode,
    AlreadyEnrolled}`.
  - `verify_totp_for_user(db, user_id, candidate_str,
    step_seconds, skew_steps, key)` → `VerifyOutcome::{Verified,
    Replay, Invalid, NotEnrolled}`.
  - `consume_backup_code(db, request, user_id, candidate_str,
    via, correlation_id)` → `BackupConsumeOutcome::{Consumed,
    Invalid, NotEnrolled, AlreadyUsed}`.
  - `disable_mfa(db, request, user_id, correlation_id)` →
    `DisableOutcome::{Disabled, NotEnrolled, PolicyRequired}`.
  - `regenerate_backup_codes(db, request, user_id,
    correlation_id)` → `RegenOutcome::{Regenerated,
    NotEnrolled}`.
  - `promote_session_to_mfa_verified(db, current_session_id,
    user_id)` → fresh plaintext token (D17 rotation).
  - `promote_session_mfa_elevated(db, session_id, ttl)` →
    in-place UPDATE for the re-auth path.
  - Plus pure helpers: `wrap_secret`, `unwrap_secret`,
    `generate_backup_codes`, `hash_backup_code`,
    `verify_backup_code`, `normalise_backup_code`,
    `current_step`, `generate_totp`, `verify_totp`,
    `build_otpauth_url`, `base32_decode_no_pad`.
- `auth::mfa::BACKUP_CODE_COUNT = 8`,
  `BACKUP_CODE_LEN = 8` constants.
- `RecoveryPolicy::mfa_step_seconds() -> u64` (default 30)
  and `mfa_skew_steps() -> u32` (default 1). Both
  provided-defaults; existing `RecoveryPolicy` impls compile
  unchanged.
- `Identity::mfa_enabled: bool` — mirrors
  `rustio_users.mfa_enabled`.
- `Identity::trust_level: SessionTrust` — current session's
  trust band (`Authenticated` / `Elevated` / `MfaVerified`).
- `StoredUser::mfa_enabled: bool` parallel field.
- `AuditEvent::MfaCodeConsumed` — `"mfa_code_consumed"`.
  `AuditEvent::BackupCodesRegenerated` —
  `"backup_codes_regenerated"`. Both `#[non_exhaustive]`
  additive.
- 8 new routes registered through `register_admin_routes` —
  `/admin/mfa/verify` (both methods) plus the three
  `/admin/account/mfa/*` route pairs.

See [`DESIGN_R3_MFA.md`](./DESIGN_R3_MFA.md) for the state
machines, audit-metadata schemas, threat model, and locked
decisions (TOTP step interval, skew tolerance, backup-code
count and shape, Argon2id parameters, encryption algorithm).

### Behaviour changes

- **`do_login` MFA branch.** After successful password
  verification + throttle reset + session mint, the handler
  inspects `user.mfa_enabled`. `TRUE` → 303 to
  `/admin/mfa/verify` instead of `/admin`. `FALSE` → 303 to
  `/admin` as before. The session row is minted with
  `trust_level = 'authenticated'` in both cases; only the
  verify-flow handler promotes to `'mfa_verified'`.
- **`/admin/reauth` requires both factors when actor has MFA
  enrolled.** The form renders the second-factor input
  conditionally. POST verifies both factors (TOTP first,
  backup-code fallback on `Invalid`); on success the new
  `promote_session_mfa_elevated` runtime stamps
  `trust_level = 'mfa_verified'` + `elevated_until`. Users
  without MFA see the R2 password-only flow unchanged.
- **`login_guard` two new redirects.** Forward-only per D6:
  - `MfaPolicy::Required` / `RequiredForRoles` + user not
    enrolled → 303 to `/admin/account/mfa/enroll` for every
    non-whitelisted request.
  - `user.mfa_enabled = TRUE` + session
    `trust_level != MfaVerified` → 303 to `/admin/mfa/verify`
    for every non-whitelisted request.
  Both gates layer below the R2 `must_change_password`
  interstitial; combined, the order is rotate → enrol →
  verify.
- **Trust escalation rotates the session token on
  `/admin/mfa/verify`.** Successful TOTP or backup-code
  verification mints a fresh row with
  `trust_level = 'mfa_verified'` and
  `parent_session_id = <current>`, then revokes the parent
  via `invalidate_sessions(Single, TrustEscalation)`. The
  cookie is swapped in the response. Doctrine 17 honoured.

### Schema

Additive, idempotent, runs at boot:

- `rustio_users.mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE`.
- `rustio_users.mfa_secret_ciphertext BYTEA` (nullable;
  AES-256-GCM `nonce || ciphertext || tag`).
- `rustio_users.mfa_secret_key_id INT` (nullable; key
  version for staged rotation).
- `rustio_users.mfa_last_used_step BIGINT` (nullable;
  monotonic replay-protection marker).
- New table `rustio_mfa_backup_codes (id BIGSERIAL,
  user_id BIGINT REFERENCES rustio_users(id) ON DELETE
  CASCADE, code_hash TEXT, created_at TIMESTAMPTZ,
  used_at TIMESTAMPTZ)`.
- Partial index `rustio_mfa_backup_codes_user_unused_idx
  ON (user_id) WHERE used_at IS NULL`.

No data backfill required. Rolling back to 0.6.0 is
data-safe: the new columns become unreferenced; the new
table is unused.

### Documentation

- [`DESIGN_R3_MFA.md`](./DESIGN_R3_MFA.md) added — the
  canonical R3 contract under the doctrine-spec template.
  1300+ lines covering invariants, threat model, authority
  flows, guarantees, schema, audit emission, module layout,
  routes, trait extensions, integration deltas, test plan,
  versioning, locked decisions, and the deferred-work
  appendix.
- `recovery_admin.rs` module-level docs note the R3
  trust-escalation token-rotation primitive
  (`promote_session_to_mfa_verified`) as the heavier sibling
  of the R2 in-place elevated promotion.

#### Internal

- New `auth::mfa` submodule — every MFA runtime fn (and
  every pure helper: TOTP RFC 6238 implementation, base32
  encoder + decoder, otpauth URL builder, AES-256-GCM
  wrappers, Argon2id wrappers).
- New `admin::mfa_handlers` submodule — eight HTTP handlers
  for the four user-facing routes.
- `admin::admin_recovery_handlers::do_reauth` extended for
  the two-factor branch; `ReauthCtx` gains a `mfa_enabled`
  field so the template renders the second-factor input
  conditionally.
- Six new dependencies in `crates/rustio-admin/Cargo.toml`:
  `aes-gcm = "0.10"` (default-features = false, features =
  `["aes", "alloc"]`), `hmac = "0.12"`, `sha1 = "0.10"`.
  The other RustCrypto family members (`argon2`, `sha2`)
  were already pulled.
- `urlencoding` (already pulled for query-string handling)
  is now used for the otpauth URL's issuer + account
  segments.

#### Tests

- +29 unit tests across the R3 commit chain (228 → 257).
  Pure / DB-free:
  - **RFC 6238 Appendix B test vectors** for TOTP-SHA1
    (truncated from 8-digit to 6-digit). Pins the
    hand-rolled implementation against the canonical
    reference.
  - **RFC 4648 §10 base32 progressive vectors**
    (`f`, `fo`, `foo`, `foob`, `fooba`, `foobar`) — the
    encoder + decoder round-trip on the standard reference.
  - AES-256-GCM wrap/unwrap round-trip, tamper detection,
    wrong-key rejection, truncated-input rejection,
    fresh-nonce-per-call invariant.
  - Argon2id backup-code round-trip, fresh-salt-per-call
    invariant, params pinning (`m=16384,t=2,p=1`), invalid
    PHC string rejection.
  - Backup-code alphabet size + ambiguity-free invariant,
    `XXXX-XXXX` shape pin, within-batch uniqueness over 64
    samples, normalisation idempotence + paste-shape
    tolerance.
  - TOTP skew window boundaries, replay rejection,
    underflow guard at `T = 0`.
  - `MfaPolicy::default() == Optional`, `MfaPolicy` Copy
    semantics.
  - `RecoveryPolicy::mfa_step_seconds() == 30`,
    `mfa_skew_steps() == 1` — locked decisions per
    Appendix B.
  - `build_otpauth_url` Google Authenticator Key URI
    format compliance.
- The `AuditEvent` drift tests pick up the two new variants
  (`MfaCodeConsumed`, `BackupCodesRegenerated`)
  automatically.
- Doctrine 22 grep proof unchanged across the entire R3
  chain — three SET arms in
  `auth::sessions::invalidate_sessions`, plus one docstring.

#### Deferred

- testcontainers Postgres integration suite extension for
  the MFA runtime functions end-to-end against an ephemeral
  Postgres. Will exercise:
  `provision_secret` + `confirm_enrolment` round-trip;
  `verify_totp_for_user` accept-current + reject-replay;
  `consume_backup_code` atomic single-use + race resolution;
  `disable_mfa` column-clearing + backup-code-deletion +
  session revocation; `regenerate_backup_codes` atomic
  transaction; `promote_session_to_mfa_verified` row mint +
  parent revoke. Lands as a follow-up to keep this commit
  set focused on the framework code itself.
- Stockholm POS downstream validation pass against the live
  DB (per `DESIGN_R3_MFA.md` §13.4) — exercises the full
  user-visible flow: enrol → sign out → sign in → verify →
  destructive admin → re-auth with both factors → regenerate
  → disable → password-only sign-in.
- Boot guard tying `RUSTIO_SECRET_KEY` presence to
  `MfaPolicy != Disabled`. Today, `MfaKey::from_env()`
  returns `Error::Internal` on miss; the handler-level
  surface is the framework's generic 500. A startup-time
  check ships as a follow-up so the failure surfaces at
  boot, not at first MFA-verify request.
- `MfaSecretKeyResolver` trait + staged-rotation playbook
  in a future `DESIGN_SECRETS.md`. The current runtime
  passes `key_id = 1` unconditionally; the column is
  reserved for the rotation hook.
- QR-code rendering in `/admin/account/mfa/enroll`. The
  current template surfaces the `otpauth://` URL as a
  clickable link (mobile authenticator apps consume it
  directly) plus the base32 manual setup key. Adding an
  SVG QR renderer (e.g. the `qrcode` crate) is a UX
  enhancement, not load-bearing for the contract.
- Admin-driven MFA disable (`MfaDisabledByOther`) — declared
  as an `AuditEvent` variant in 0.5.0 but wires up in R4
  CLI emergency recovery, not in R3 web routes.


## [0.6.0] — 2026-05-10

R2 of the universal account-recovery architecture. Covers the
admin-initiated path: password reset (email or temp-password
mode), lock and unlock, forced rotation, session revocation
without locking. Plus auto-throttle on failed logins and a
re-auth wall on every destructive admin action.

> **Migration from 0.5.x**
>
> Bump `rustio-admin = "0.6"` and run `cargo update -p rustio-admin`.
>
> Schema is additive: three new columns on `rustio_users`
> (`failed_login_count`, `last_failed_login_at`, `locked_until`)
> plus a partial index. Existing users are unaffected — the
> counter defaults to 0, lockout defaults to NULL.
>
> No middleware changes. All routes register through
> `register_admin_routes`.
>
> Doctrine 22 holds: `auth::sessions::invalidate_sessions`
> remains the sole writer of `revoked_at` — three SET arms
> (lines 398 / 413 / 426 in `auth/sessions.rs`).

### Highlights

- **Admin-driven password reset.** `POST /admin/users/:id/reset-password`
  with two modes:
  - **email** — admin-initiated reset token; target consumes via
    R1's `/admin/reset-password/<token>` flow.
  - **temp_pw** — 16-char URL-safe-base64 password rendered once
    on the admin success page; sets `must_change_password = TRUE`,
    revokes every session.
- **Manual lock and unlock.** `POST /admin/users/:id/lock`
  (duration presets: 15 min / 1 h / 24 h / 7 d / indefinite +
  freeform-minutes) and `POST /admin/users/:id/unlock`. Lock
  revokes every session; unlock zeroes the throttle counter.
- **Admin revoke-sessions.** `POST /admin/users/:id/revoke-sessions`
  — sibling of lock without the `locked_until` write.
- **Auto-throttle on failed logins.** 5 failures / 10 min →
  15-minute soft lock. Sessions are not revoked; locking refuses
  future sign-ins.
- **Re-auth wall.** 15-minute elevated-session window. Every
  destructive admin action requires a fresh password verify.
- **Forced password rotation** via `must_change_password` with a
  whitelisted interstitial path.

### Public API

- `auth::recovery_admin::AdminActor` — bundles `user_id` + `email`.
  Email hashes into `metadata.actor_email_hash` (8-char SHA-256).
- `auth::LoginThrottle { max_attempts, window_minutes, lock_minutes }`.
  Default 5 / 10 / 15 via `RecoveryPolicy::login_throttle()`.
- `RecoveryPolicy::reauth_window()` (default 15 minutes) and
  `scope_for(&Identity)` for multi-tenant overrides.
- `LogEntry::actor_user_id: Option<i64>`, persisted under
  `metadata.actor_user_id`. Builder: `LogEntry::with_actor(id)`.
- `Identity::must_change_password: bool`; parallel field on
  `StoredUser`.
- `AuditEvent::ForcedPasswordChangeCompleted` —
  `"forced_password_change_completed"`. The four pre-declared R2
  variants (`PasswordResetByOther`, `AccountLocked`,
  `AccountUnlocked`, `SessionsRevokedByOther`) light up.
- 12 new routes registered through `register_admin_routes` —
  reset, lock, unlock, revoke-sessions, reauth,
  must-change-password.

See [`DESIGN_R2_ORGANISATIONAL.md`](./DESIGN_R2_ORGANISATIONAL.md)
for the state machines, audit metadata schemas, and locked
decisions.

### Behaviour changes

- **`do_login` rewritten.** Pre-R2 returned three response shapes
  (401 wrong creds, 403 inactive, no lockout). Post-R2 returns a
  single uniform 401 across every failure mode.
- **CLI password floor 8 → 10.** `rustio user create` delegates
  to `DefaultPasswordPolicy::new()`. Same floor as
  admin-create-user and self-recovery.
- **Admin Add-user form respects policy override.** Pre-R2 the
  form hardcoded 8 chars; post-R2 reads from
  `Admin::active_password_policy()`. A project with
  `min_length = 16` sees 16 in the hint and has 16 enforced.
- **Admin-edit form: password field removed.** The legacy
  `new_password` input on `/admin/users/:id/edit` is gone
  (`DESIGN_RECOVERY.md` §14.4). Admin password resets go through
  the dedicated `/admin/users/:id/reset-password` route.
- **Auto-throttle does not revoke sessions.** Locking only
  refuses future sign-ins. Existing sessions continue until the
  next request. Doctrine D4 + §3.3.

### Schema

Additive, idempotent, runs at boot:

- `rustio_users.failed_login_count INT NOT NULL DEFAULT 0`.
- `rustio_users.last_failed_login_at TIMESTAMPTZ`.
- `rustio_users.locked_until TIMESTAMPTZ`.
- Partial index `rustio_users_locked_until_idx ON (locked_until) WHERE locked_until IS NOT NULL`.

No data backfill required. Indefinite manual locks encode as a
year-9999 timestamp so the partial index continues to find them.

### Documentation

- [`DESIGN_R2_ORGANISATIONAL.md`](./DESIGN_R2_ORGANISATIONAL.md)
  added — the canonical R2 contract.
- README architecture-doctrine table updated with a row pointing
  to `DESIGN_R2_ORGANISATIONAL.md`.

#### Internal

- New `auth::recovery_admin` submodule — every R2 runtime fn.
- New `admin::admin_recovery_handlers` submodule — every R2 HTTP
  handler.
- New `Row::get_optional_datetime` ORM helper — closes the
  nullable-`TIMESTAMPTZ` read gap.
- `admin::builtin` module visibility raised to `pub(crate)`.
- `admin::handlers::record_session_revocations` raised to
  `pub(super)`.
- `admin::audit::build_persisted_metadata` merge helper inserts
  `actor_user_id` into the metadata object before binding.

#### Tests

- +58 unit tests across the R2 commit chain (162 → 220). Pure /
  DB-free: type-level invariants, pure helpers (`validate_return_to`,
  `parse_lock_duration`, `actor_email_fingerprint`,
  `random_temp_password`, `MUST_CHANGE_WHITELIST` membership,
  `LockDuration` time math). The `AuditEvent` drift tests pick
  up the new variant automatically.
- Doctrine 22 grep is part of every R2 commit's pre-commit gate
  — three SET arms in `auth::sessions::invalidate_sessions`,
  unchanged across the entire R2 chain.

#### Deferred

- Testcontainers Postgres integration suite ships gated behind
  `--features integration-test`. Covers the SQL paths in
  `record_failed_login`, `check_account_lockout`,
  `promote_session_elevated`, `check_session_elevated`,
  `lock_user_account`, `admin_set_temp_password`, plus the
  re-auth and forced-rotation handlers' DB-touching steps.


## [0.5.0] — 2026-05-09

R1 of the universal account-recovery architecture. Self-service
password recovery is end-to-end: forgot link → email → reset
form → sign-in. Every session across every device is revoked at
consume time. Active-session controls (revoke single / others /
all) are wired on the existing `/admin/account/sessions` page.
The authenticated `/admin/password_change` flow is brought into
parity with the recovery doctrine.

> **Migration from 0.4.x**
>
> Bump `rustio-admin = "0.5"` and run `cargo update -p rustio-admin`.
>
> Schema is additive: new `rustio_password_reset_tokens` table
> plus two new columns on `rustio_users`. Existing users and
> sessions are unaffected.
>
> No middleware changes — `correlation_id` BEFORE `csrf_protect`
> (added in 0.4.0) is still the only ordering constraint.
> Recovery routes register through `register_admin_routes`.
>
> Production deployments wiring a real `Mailer` should opt the
> policy into strict mode:
> `RecoveryPolicy::strict_mailer_required(true)` makes the
> framework refuse to start with the default `LogMailer`.

### Highlights

- **Self-service forgot/reset password flow.** Five new routes —
  `GET /admin/forgot-password`, `POST /admin/forgot-password`,
  `GET /admin/forgot-password/sent`,
  `GET /admin/reset-password/:token`,
  `POST /admin/reset-password/:token`.
- **Email-link reset tokens.** 256-bit URL-safe-base64. Plaintext
  leaves the framework only in the email body. The DB stores
  `sha256(token)` only.
- **Atomic single-use consume.** Single SQL statement
  (`UPDATE … RETURNING`) flips the row exclusively. Concurrent
  submissions resolve as exactly one Consumed and one Invalid.
- **Active-sessions revoke buttons.** Three new POSTs at
  `/admin/account/sessions/...` — single revoke, others, all.
- **Authenticated password change revokes other devices.** Goes
  through `invalidate_sessions(UserExceptCurrent, UserRequested)`.
  Current device stays signed in.
- **Reset-token sweeper.** 7-day forensic-retention window;
  integrated into the existing 10-minute session-sweeper tick
  with independent failure isolation.

### Public API

- `auth::PasswordPolicy` trait + `DefaultPasswordPolicy`. Length
  floor 10, Unicode `char` count, no complexity-class rules
  (NIST SP 800-63B Appendix A). Override via
  `Admin::password_policy(Arc::new(...))`.
- `auth::RecoveryPolicy` trait + `DefaultRecoveryPolicy` —
  `reset_token_ttl()` (1h), `request_rate_limit()` (5 / 15min /
  IP), `consume_rate_limit()` (10 / 5min / IP),
  `strict_mailer_required()` (false default), `public_site_url()`
  with a header-derivation default.
- `Admin::mailer(...)` builder + `Admin::active_mailer()`
  accessor. `Admin::has_custom_mailer()` flag drives the
  strict-mailer guard.
- `AuditEvent` promoted to `pub` with `#[non_exhaustive]`. New
  variant `PasswordChangedSelf`. Variant strings locked-in by
  `audit_event_existing_variants_have_stable_strings`.
- `LogEntry::with_event(AuditEvent)` builder — typed-event
  boundary. Adds `event: Option<AuditEvent>` field; existing
  struct-literal call sites pass `event: None`.

### Behaviour changes

- **Authenticated password changes sign out other devices.**
  Pre-R1 left other sessions live. R1 closes the drift through
  `invalidate_sessions(UserExceptCurrent, UserRequested)`.
- **Default password minimum length 8 → 10.** Existing users
  with shorter passwords are not forced to change — the policy
  fires only on new passwords during a change or reset.
- **Recovery tokens retained 7 days after expiry.** Forensic
  window for audit correlation, abuse investigation, and
  operational debugging.
- **Strict-mailer mode can fail startup intentionally.** When
  `strict_mailer_required(true)` is set and the default
  `LogMailer` is still in place, `register_admin_routes` panics
  with an operator-actionable error.
- **`MIN_PASSWORD_LEN` constant removed from `admin/handlers.rs`.**
  Was effectively private. The CLI's parallel constant remains
  at 8 chars in 0.5.0; R2 unifies both surfaces.

### Security

- Token hashes only. Plaintext lives in the email body, never in
  the DB, log lines, or audit metadata. `IssueOutcome` and
  `ConsumeOutcome` Debug formats are property-tested token-free.
- Uniform outward responses on the recovery flow —
  `do_forgot_password` always 303s to
  `/admin/forgot-password/sent`; `do_reset_password` renders the
  same "no longer valid" page across unknown / expired /
  consumed / rate-limited tokens.
- No direct `revoked_at` writes. Doctrine 22 single-writer
  invariant preserved across all R1 paths — a
  `grep -rE "revoked_at\s*="` across `crates/` returns only
  `auth::sessions::invalidate_sessions`.
- CSRF preserved across recovery flows.
- `PasswordPolicyError` carries no plaintext. Display + Debug
  property-tested plaintext-free.

### Schema

Additive, idempotent:

- New table `rustio_password_reset_tokens` with `token_hash`,
  `user_id`, `requested_at`, `expires_at`, `consumed_at`,
  `mail_status`, `requested_ip`, `requested_user_agent`,
  `correlation_id`, plus a partial unique index on
  `(token_hash) WHERE consumed_at IS NULL`.
- `rustio_users.must_change_password BOOLEAN NOT NULL DEFAULT FALSE`
  (R1 declares; R2 enforces).
- `rustio_users.password_changed_at TIMESTAMPTZ`. Stamped by
  `auth::set_password` on every change.

### Documentation

- [`DESIGN_RECOVERY.md`](./DESIGN_RECOVERY.md) added — the
  canonical R1 contract.
- README *Architecture doctrine* section listing the four
  contracts (`DESIGN_SYSTEM`, `DESIGN_SESSIONS`, `DESIGN_AUDIT`,
  `DESIGN_RECOVERY`).
- `docs/getting-started.md` — clarifies CLI password prompts,
  port-8000 troubleshooting, the migrations workflow,
  what-you-get-after-first-login, and the project philosophy.
- `templates/project/README.md.tmpl` mirrors the
  pre-R1 clarifications.

#### Internal

- `auth::recovery` submodule — schema migrations, trait surface,
  runtime primitives (`issue_reset_token`, `consume_reset_token`,
  `check_reset_token_valid`), and the periodic sweeper.
- `admin::recovery_handlers` — five HTTP handlers and the
  `RecoveryState` rate-limit carrier.
- `set_password` stamps `password_changed_at` (single SQL
  statement; signature unchanged).
- `RateLimiter::allow(key)` and `auth::sessions::random_token`
  promoted to `pub(crate)` for the recovery module.
- Recovery sweeper integrated into
  `background::spawn_session_sweeper` with independent failure
  isolation.

#### Public-API summary

The `Admin` struct gains four `pub(crate)` fields and the public
methods `mailer / has_custom_mailer / password_policy /
active_password_policy / recovery_policy / active_recovery_policy
/ active_mailer`. `LogEntry` gains `event: Option<AuditEvent>`
and `with_event(...)`. Existing constructors and accessors are
unchanged; struct-literal call sites add `event: None`.


## [0.4.0] — 2026-05-09

Session lifecycle and recovery foundations. R0 of the universal
account-recovery architecture, documented in
`DESIGN_SESSIONS.md` and `DESIGN_AUDIT.md`. **No password-reset
flow yet** — that ships in R1 (0.5.0). This release lays the
safe foundation: hashed-at-rest session tokens, centralised
invalidation, typed lifecycle vocabulary, audit forensic chain,
and an email abstraction.

> **Migration from 0.3.x**
>
> Bump `rustio-admin = "0.4"` and run `cargo update -p rustio-admin`.
>
> **Add `middleware::correlation_id` BEFORE
> `middleware::csrf_protect`** in your router so audit rows get a
> populated `correlation_id`.
>
> Schema is additive; existing sessions continue to authenticate
> through a 14-day plaintext-fallback window.

### Added

- **Hashed-at-rest session tokens.** `rustio_sessions.token_hash`
  stores `sha256(cookie-token)`; the cookie keeps the plaintext.
  Lookup is hash-first with a plaintext fallback for the 14-day
  transition window.
- **Centralised session invalidation** (`auth::invalidate_sessions`)
  — the single legitimate writer of `rustio_sessions.revoked_at`.
  A `grep -rE "revoked_at\s*="` returns only this function.
  Called by logout, password reset (R1), MFA disable (R3),
  administrative revoke (R2), and trust-escalation rotation.
- **Typed lifecycle vocabulary.** `SessionTrust`
  (`Authenticated < Elevated < MfaVerified` with `satisfies()`),
  `SessionInvalidationReason` (12 variants), `SessionTarget`
  (`User` / `UserExceptCurrent` / `Single`), `Session`
  (read-only view), `InvalidationOutcome`.
- **`auth::list_active_for_user(db, user_id)`** and
  **`auth::current_session_id(db, token)`** for the active-sessions UI.
- **Read-only `/admin/account/sessions` page.** Lists every
  active session for the signed-in user with trust label, IP,
  short user-agent summary, created_at, last-seen, and
  expires-relative time. Revoke buttons land in 0.5.x.
- **`email::Mailer` trait + `LogMailer` default +
  `Mail::framework_envelope`.** Recovery-flow email primitive
  without locking the framework into SMTP. The envelope appends
  a fixed security footer (system name, timestamp, source IP,
  device summary, "if this was not you" guidance).
- **`audit::redact` helpers** — `redact_password`, `redact_token`,
  `redact_mfa_secret`, `redact_backup_code`. `redact_token`
  returns a non-reversible 8-char SHA-256 fingerprint.
- **`middleware::correlation_id`** — UUID v7 stamped on every
  request, surfaced in `x-correlation-id`, stashed in the
  request context for the audit pipeline. Honours an inbound
  `x-correlation-id` header when shape-safe; replaces
  adversarial inputs with a fresh v7. Designed to install
  before `csrf_protect`.
- **Audit row gains structure**: new `metadata JSONB`,
  `correlation_id TEXT`, `session_id BIGINT` columns + partial
  indexes on the latter two.
- **Internal `audit::AuditEvent` enum** (`pub(crate)` in 0.4.0)
  with 18 variants covering R0–R4 actions. Drift tests assert
  `as_str()` uniqueness and snake_case shape. Public typed
  surface lands in 0.5.x.
- **`DESIGN_SESSIONS.md`** — canonical lifecycle reference:
  state machine with invariants, token storage shape, trust
  escalation, single-writer guarantee on `revoked_at`,
  expiration paths, active-sessions UI contract, forensic
  correlation_id story, versioning policy.
- **`DESIGN_AUDIT.md`** — companion: row shape, typed evolution
  path, redaction helpers, forensic chain queries, required
  middleware ordering, reserved metadata JSONB keys.

### Changed

- **Logout is a soft revoke.** `do_logout` calls
  `auth::logout_session` which routes through
  `invalidate_sessions` with `reason = Logout`. The row is
  preserved for audit; `purge_expired_sessions` deletes it
  after `expires_at` passes.
- **Session lookup excludes revoked rows.** `revoked_at IS NULL`
  is part of every active-session query — a logged-out cookie
  cannot re-authenticate regardless of which lookup path
  matched.
- **Required middleware ordering.** Projects must install
  `middleware::correlation_id` before `csrf_protect`.

### Schema

Additive, idempotent, runs on every boot:

- `rustio_sessions`: `session_id BIGINT` (sequence-backed),
  `token_hash TEXT`, `device_id TEXT` (reserved), `trust_level TEXT`
  (CHECK `authenticated`/`elevated`/`mfa_verified`),
  `elevated_until TIMESTAMPTZ`, `parent_session_id BIGINT`,
  `revoked_at TIMESTAMPTZ`, `revoked_reason TEXT`. New unique
  partial indexes on `session_id`, `token_hash`, plus
  `(user_id) WHERE revoked_at IS NULL` and `parent_session_id`
  partial.
- `rustio_admin_actions`: `metadata JSONB`, `correlation_id TEXT`,
  `session_id BIGINT` + partial indexes.

### Dependencies

- `sha2 = "0.10"` (new) — session-token hashing at rest.
- `uuid` gains the `v7` feature for time-sortable correlation
  ids.

[0.4.0]: https://github.com/abdulwahed-sweden/rustio-admin/releases/tag/v0.4.0


## [0.3.0] — 2026-05-08

Authority and design-system stabilisation. Server-side guards
enforce the rank model on every authority mutation; group
permissions render as a model × action matrix; foreign-key
columns on list pages resolve to display labels with
click-throughs; the framework's canonical accent moves to
teal-emerald. New `DESIGN_SYSTEM.md` codifies the authority and
visual vocabulary.

> **Migration from 0.2.x**
>
> Bump `rustio-admin = "0.3"`, run `cargo update -p rustio-admin`,
> then hard-refresh the admin in your browser so the new
> `admin.css` is fetched.
>
> If your project redefined `--rio-accent` in its own CSS to
> swap to teal, that block is now redundant and can be deleted —
> the framework default already serves the same value. Other
> `--rio-*` token redefinitions in project CSS are now
> framework forks; see `DESIGN_SYSTEM.md` §2 for the supported
> override paths.

[0.3.0]: https://github.com/abdulwahed-sweden/rustio-admin/releases/tag/v0.3.0

### Added

- **Authority guards (`auth::guards`).** Five composable
  server-side guards: `enforce_self_demote_safe`,
  `enforce_cross_rank_safe`, `enforce_role_ceiling`,
  `enforce_no_orphan_role`. Guards return `Error::Forbidden`
  with a human-readable reason; the HTTP layer renders that
  reason on the 403 page. Every guard runs on POST regardless
  of what the form said.
- **Generalised orphan-prevention.** `auth::would_orphan_role(role)`
  and `auth::would_orphan_protected()` cover every entry in
  `auth::protected_roles()` (currently `[Administrator, Developer]`)
  instead of only Developer. The pure verdict
  `auth::verdict_for_orphan_role(...)` is exposed for unit
  testing.
- **Audit logging on authority mutations.** `do_user_edit`,
  `do_new_user`, `do_user_delete`, `do_new_group`,
  `do_group_edit`, and `do_group_delete` write
  `rustio_admin_actions` rows with actor, target, IP, and a
  diff summary.
- **Role dropdown ceiling.** `role_select_options(editor_rank)`
  filters out roles strictly above the editor's own rank in
  user-new and user-edit forms. Server-side
  `enforce_role_ceiling` catches forged POSTs as
  defence-in-depth.
- **Group permissions matrix.** The Group edit page lays out
  permissions as a model × action grid (View / Add / Change /
  Delete columns, one row per model) instead of a flat
  alphabetical checkbox list. Permissions whose codename
  doesn't fit `<table>.<action>_<singular>` fall through to a
  collapsed "Other permissions" group below the matrix. Per-row
  "All" button toggles every permission for that model;
  degrades to plain multi-checkbox UX without JS.
- **Foreign-key list-cell hydration.** List pages resolve every
  `belongs_to` column on the current page to the target row's
  display field, wrapping the cell in
  `<a href="/admin/{admin_name}/{id}/edit">…</a>`. N+1-safe by
  construction: at most one batched
  `SELECT id, <display> FROM <target> WHERE id = ANY($1)` per
  FK column. Stale or display-field-less FKs leave the raw id
  in place. New `CellLink` type and parallel `cell_links`
  vector on `ListRow`.
- **`ListRowCtx.links: HashMap<String, String>`** exposes
  per-column FK click-through URLs to the list template.
- **`admin.css` token sectioning.** Banner comments make
  canonical token blocks explicit so feature branches can't
  silently override the design system without a visible diff.

### Changed

- **Canonical accent moved from terracotta to teal-green.**
  Framework default `--rio-accent` is now `#0F8C7E` (light) /
  `#3FAA9D` (dark), replacing `#A0341A` / `#C84934`. Same value
  the Bosphorus & Sham downstream had been overriding to;
  promoting it to the framework default removes the
  duplicate-token-system risk.
- **`Role::rank()` widened to `u32`** with spaced numeric values
  (`User=100 / Staff=300 / Supervisor=600 / Administrator=900 /
  Developer=1000`) so projects extending the rank ladder via
  group labels have headroom. Compare relatively, never match
  literally.
- **`admin/list.html`** wraps a cell in `<a class="rio-fk-link">`
  when `row.links[<field>]` is set. Cells without a registered
  relation render unchanged.

### Deprecated

- **`auth::would_orphan_developers`** — kept as a thin wrapper
  around `would_orphan_role(_, _, Role::Developer, _, _)` for
  backwards compat. New code should use `would_orphan_protected`
  to cover Administrator orphan-prevention too.

### Documentation

- **`DESIGN_SYSTEM.md`** — canonical doctrine for the
  framework's authority and visual vocabulary. Three principles
  stated explicitly (UI hiding is reflection, not security;
  rank controls WHO, permissions control WHAT; groups are
  permission bundles, not authority roots), token ownership
  rules, Arabic typography rules, and a versioning policy.
- **`.github/pull_request_template.md`** — any PR touching
  `admin.css`, token definitions, font-family declarations, or
  `:root` blocks must complete a Token disclosure section
  (tokens changed / migration impact / regression risk) and
  walk an 8-item visual-regression checklist.


## [0.2.1] — 2026-05-07

CLI-only patch. `rustio-admin` and `rustio-admin-macros` stay
at 0.2.0.

### Fixed

- **`rustio startproject` scaffold template** pinned new
  projects to `rustio-admin = "0.1"`. Cargo's `"0.1"`
  constraint resolves to `>=0.1.0, <0.2.0`, so anyone who ran
  `cargo install rustio-admin-cli` immediately after the 0.2.0
  release would get a project locked to the previous framework
  line and miss every 0.2.0 feature. The scaffold now pins to
  `rustio-admin = "0.2"`.

  Re-install with `cargo install rustio-admin-cli --force` to
  pick up the corrected template; existing projects are
  unaffected.


## [0.2.0] — 2026-05-07

Design-language unification across list, form, and Auth pages.
Dark mode shifts from a high-contrast terminal aesthetic to a
calm graphite workspace. New list-view toolbar (filters / sort /
per-page / active-filter pills / numbered pagination / search
glyph) ships with full URL state preservation. Projects can
register custom bulk actions alongside the built-in delete.

> **Breaking change.** `AdminTheme` is now an override-patch
> type with `Option<String>` fields instead of `String`
> snapshots; default is *all `None`* (no inline `<style>`
> emitted at all). `Admin::accent()` returns `Option<&str>`.
> See **Migrating from 0.1.x** below.

### Added — list view toolbar

- **Filters dropdown.** Inferred from `bool` columns +
  `ModelAdmin::list_filter()`. Each filter group has an
  All / Yes / No tri-state. Active selections preserve
  across sort, search, pagination, and per-page changes.
- **Sort dropdown.** Builds from `ModelAdmin::list_display()`
  column metadata, with sortable columns flagged. Active sort
  preserved across filter / search / pagination changes.
- **Per-page picker.** 25 / 50 / 100 / 200 with the active
  value highlighted. URL state preserved on switch.
- **Active-filter pills.** A row of removable pills
  beneath the toolbar shows every active filter / sort /
  search; removing a pill drops just that constraint.
- **Numbered pagination.** Standard windowed pagination
  with "first / prev / 1…7 / next / last", smart truncation
  for large counts, and current-page accent ring.
- **Search-glyph affordance.** `ModelAdmin::search_fields()`
  surfaces a search icon in the toolbar; the search input
  expands inline.
- **URL state preservation across every widget.** Toolbar
  state lives in the query string (`?filter=foo&sort=col-desc&q=…`
  etc.) so back-button / refresh / share-link all behave.

### Added — bulk select + bulk actions

- **Per-row + master checkbox** in the list view's first
  column. Master toggles all rows on the visible page;
  per-row selections survive pagination via `?ids=...` query
  param.
- **Bulk-actions menu.** `ModelAdmin::bulk_actions()` returns
  `Vec<BulkAction>` with `(slug, label, danger)`. Built-in
  delete is registered automatically. Submitting runs the
  framework-supplied `confirm` page first (lists the affected
  ids), then the project-defined handler.
- **Built-in `delete` bulk action.** Walks the IDs through
  `Model::delete_by_id(&Db, id)`, audit-emitting one
  `ActionType::Delete` row per affected object.

### Added — form view + Auth refresh

- **Editorial form pages.** Single column at 880-px max
  width; sticky bottom action bar (Save / Cancel / Delete);
  inline help under fields; consistent typography and spacing
  with list view.
- **Auth pages.** Login / password change / forbidden / 404 /
  500 share the same chrome and tokens as the admin proper.
- **Sticky sidebar at tablet+.** Sidebar follows scroll on
  ≥ 768 px; collapses to a hamburger drawer on mobile.

### Added — design tokens

- **Calm graphite dark mode.** Page background `#0e1216`;
  surfaces `#1a1f25` / `#222932`; text `#e5e7eb` / muted
  `#94a3b8`. Accent stays consistent across themes; borders
  one shade lighter than each theme's surface.
- **Spacing scale 1..8** with `--rio-s1` 4px through
  `--rio-s8` 56px; consumed across every component for
  consistent rhythm.

### Added — code-level hooks

- **`Admin::version()`** returns the framework's published
  version string (`env!("CARGO_PKG_VERSION")`); rendered in
  the sidebar footer.
- **`Admin::has_relation(...)`** + relation registry — the
  scaffolding the 0.3.0 FK list-cell hydration sits on.

### Changed — theme architecture

- **`AdminTheme` is now an override-patch type.** Fields are
  `Option<String>`; default is all `None` (zero inline
  `<style>` emitted). `admin.css` is the single source of
  truth. Projects override per-token via `Admin::theme(...)`.
- **`Admin::accent()` returns `Option<&str>`** instead of
  `&str`. Rendering callers fall back to the CSS-side default.

### Changed — dark mode

- **Refactored from OLED-black to graphite.** New tokens above
  replace the previous near-black palette. Borders +
  muted-text + text are all bumped one notch lighter for
  AAA contrast on the new surfaces.

### Changed — list view rendering

- **Cell rendering is now type-aware.** Boolean columns render
  as Yes / No badges; date columns as relative-time chips;
  text truncates at column-width with a hover popover.
- **Empty-state treatment** ships out of the box for empty
  filters and empty result sets (distinct messaging).

### Changed — layout

- **3 breakpoints** (mobile / tablet / desktop) with a single
  shared layout grid. Sidebar collapses below 768 px.

### Migrating from 0.1.x

If your project doesn't customise the theme, no changes
required.

If your project does call `Admin::theme(AdminTheme { … })`
with `String` field values, switch to the override-patch form:

```diff
- AdminTheme {
-     accent: "#0F8C7E".to_string(),
-     surface: "#ffffff".to_string(),
-     text: "#1f2937".to_string(),
- }
+ AdminTheme {
+     accent: Some("#0F8C7E".to_string()),
+     ..Default::default()
+ }
```

`Admin::accent()` callers handle the new `Option<&str>` return
type — fall through to a CSS-side default when `None`.

The bulk-actions registry adds a `BulkAction { slug, label,
danger }` shape; if a project registered custom bulk actions
in 0.1.x via an experimental override, switch to
`ModelAdmin::bulk_actions()` returning `Vec<BulkAction>`.

The list-view URL parameters now include `filter=…`, `sort=…`,
`q=…`, `per_page=…`, `ids=…`; project-side bookmarks built on
the previous flat URL shape need updating.

The CSS allow-list (the framework's `:root --rio-*` token
namespace) is now stable; project CSS that overrode tokens
via custom selectors should switch to the override-patch
`Admin::theme(...)` API. The framework's allow-list now
starts at 25.


## [0.1.1] — 2026-05-07

Design-system pass. No public API surface changes — the
typography, font, and brand-color work all happens behind the
existing `Admin`, `AdminTheme`, and template-override surfaces.
Drop-in upgrade from 0.1.0.

### Added

- **Self-hosted fonts (SIL OFL-1.1)** baked into the binary,
  served at `/static/fonts/*.woff2` with year-long immutable
  cache:
  - Geist Variable + Geist Mono Variable — Latin UI + code
    (single woff2 each covers full `wght` 100..900).
  - Tajawal 400 / 500 / 700 — Arabic UI surfaces.
  - Noto Naskh Arabic Variable — Arabic body / paragraph copy.
  - All filtered by `unicode-range` so Latin-only pages pay
    zero Arabic download cost. Total embedded: ~270 KB.
- **Complete typography token system** in `admin.css` — three
  family tokens, a 9-step size scale (`--rio-fs-xs` 13px
  through `--rio-fs-display` 40px), four line-height tokens
  including `--rio-lh-arabic: 1.9`, four weight tokens, and
  Latin-only tracking tokens that auto-reset for Arabic / RTL.
- **`:lang(ar)` / `[dir="rtl"]` resolution rules** — Arabic
  text picks up Tajawal (UI) or Noto Naskh (body) automatically;
  Geist's stylistic alternates are stripped so joining-script
  shaping stays intact.
- **`--rio-surface-2` / `--rio-border-strong` / `--rio-text-subtle`**
  tokens for secondary surfaces, heavy outlines, and tertiary
  text.

### Changed

- **Default brand accent is now `#A0341A`** (Andalusian
  crimson), replacing the previous cobalt `#2563EB`. Applies
  in three places — `AdminTheme::default()`, `admin.css`
  `:root --rio-accent`, and `render::hex_to_rgb_triplet`
  fallback. Projects override via `Admin::theme(...)` or
  `Admin::accent_color(...)`.
- **Tightened light palette** for stronger reading contrast:
  `--rio-bg` `#f4f6fb` → `#ebeef4`, `--rio-text-muted`
  `#4b5563` → `#3d4452`, `--rio-border` `#d1d5db` → `#cdd3df`.
  Dark-mode tokens similarly bumped. Every text/surface pair
  clears WCAG AAA in both themes.
- **Body font-size** raised from 14px to **16px**; minimum
  helper-text size enforced at **13px**; table cells at
  **15px**; headings rescaled (h1 34px, h2 26px, h3 22px).
  Mobile bumps `html` to 16.5px below 600px.
- **Sidebar + topbar typography** polished — sidebar links 15px
  medium, topbar identity 15px regular, theme toggle 13px
  medium.
- **`/static/admin.css` and `/static/admin.js`** Cache-Control
  flipped from `public, max-age=3600` to
  `no-cache, must-revalidate` so theme + design tweaks roll
  out the moment the binary restarts. Fonts keep their
  year-long immutable cache.

### Removed

- **IBM Plex Sans Arabic** dropped from the bundled fonts —
  Tajawal + Noto Naskh Arabic cover the UI/body split. Anyone
  who customised templates to reference `"IBM Plex Sans Arabic"`
  by name will need to switch to `"Tajawal"` /
  `"Noto Naskh Arabic"`; the `--rio-font-arabic` and
  `--rio-font-arabic-body` tokens resolve both automatically.


## [0.1.0] — 2026-05-07

First public release. Strategic-reset rollout of phases 1–15
plus the live browser walk and the operator CLI is
feature-complete.

### Added

- **`ModelAdmin` trait** with seven hooks: `list_display`,
  `list_filter`, `search_fields`, `ordering`, `list_per_page`,
  `readonly_fields`, `fieldsets`. Every method has a default
  body — projects write `impl ModelAdmin for X {}` to opt in,
  override individual methods to customise.
- **Generic admin runtime.** `Admin::new().model::<M>()`
  registers a Postgres-backed CRUD page;
  `register_admin_routes` mounts every URL the admin needs
  onto a `Router`. List / create / edit / delete /
  per-object history all wired.
- **Built-in user / group pages** at `/admin/users/*` and
  `/admin/groups/*`, plus `/admin/password_change` and
  `/admin/history`.
- **Server-side filters + ILIKE search + pagination** pushed
  into a single SQL query with column-name validation against
  `M::COLUMNS`.
- **Sortable list-page columns** via `?sort=col&dir=desc`,
  falling back to `ModelAdmin::ordering()`.
- **Hand-written CSS theme.** Six CSS custom properties driven
  by `Admin::theme(...)`. Mobile-first responsive (3
  breakpoints), dark mode via `prefers-color-scheme` plus a
  manual toggle persisted to `localStorage`.
- **Auth and RBAC.** Five-tier role ladder (User → Developer),
  Argon2 password hashing, DB-backed sessions, per-model
  permissions with a 60-second cache, last-developer orphan
  guard on user delete.
- **Audit log.** Every create/update/delete writes to
  `rustio_admin_actions`; surfaced in the dashboard's "Recent
  actions" widget, the global `/admin/history` page, and
  per-object `/admin/<model>/<id>/history`.
- **Middleware bundle.** Rate limit, CSRF (double-submit
  cookie), security headers, gzip, request logger.
- **Migrations runner** that walks numerically prefixed
  `*.sql` files in a directory and applies them
  transactionally with a tracking table.

### Architecture

- **Tier 1, single-binary, Postgres-only.** Schema contracts,
  drift validation, AI planners, multi-database backends, and
  search backends are explicitly out of scope (see the
  [strategic reset plan](./rustio-admin-strategic-reset-plan.md)
  §1, §3, §8).
- **No Tailwind, no PostCSS, no build step.** The CI pipeline
  enforces the no-Tier-2-symbols invariant with a `git grep`
  guard on every PR.

### `rustio` CLI

The binary that ships from `rustio-admin-cli` covers the
operationally critical surface for v0.1.0:

- `rustio migrate apply` / `status` — drives the framework's
  numerically prefixed `migrations/*.sql` runner.
- `rustio user create` / `list` / `role` / `delete` —
  auth-table CRUD with Argon2 hashing and a confirm-twice
  password prompt. Honours the developer-orphan guard.
- `rustio group create` / `list` / `add-user` /
  `remove-user` — group CRUD and membership.
- `rustio perm grant-user` / `grant-group` / `list` —
  permission grants on top of `auth::permissions`.
- `rustio doctor` — read-only health check (DB reachable?
  auth tables present? at least one administrator?). Exits
  non-zero on any blocker so a CI step can gate on it.
- `rustio startproject <name>` — scaffolds a fresh project
  at `./<name>/` with a working `Cargo.toml`, a demo `Post`
  model with a populated `ModelAdmin` impl, starter
  migration, `.env.example`, and a `README.md`. Templates
  are baked into the CLI binary via `include_str!`.
- `rustio startapp <name>` — adds a model + migration to an
  existing project. Generates `src/<name>.rs` (full `Model`
  + empty `ModelAdmin` impl) and
  `migrations/<NNNN>_create_<name>s.sql` with an
  auto-incremented number. Refuses to mutate `src/main.rs`
  automatically; prints the exact `mod` / `use` /
  `.model::<>()` lines instead.

Browser-walked end-to-end against a local Postgres: a single
`rustio startproject blog` plus two `rustio startapp`
invocations generated three models which all rendered their
list pages on `/admin/posts`, `/admin/comments`, and
`/admin/book_reviews` after a copy-paste edit to
`src/main.rs`.

### Released to crates.io

All three workspace crates are on crates.io as of 2026-05-07:

- [`rustio-admin@0.1.0`](https://crates.io/crates/rustio-admin)
- [`rustio-admin-macros@0.1.0`](https://crates.io/crates/rustio-admin-macros)
- [`rustio-admin-cli@0.1.0`](https://crates.io/crates/rustio-admin-cli)

Project consumers add `rustio-admin = "0.1"` to their
`Cargo.toml`; operators install the CLI with
`cargo install rustio-admin-cli`.
