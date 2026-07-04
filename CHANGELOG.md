# Changelog

All notable changes to `rustio-admin` are recorded here. Format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
the project adheres to [SemVer](https://semver.org/) once it
leaves the alpha track.


## Unreleased

*Nothing yet — work for the next release lands here.*


## 0.31.0 — 2026-07-03

Rolls up roughly three weeks of work on `main` since `v0.30.0`.

### Audit

Dependency and toolchain floor moved: **sqlx 0.8 → 0.9**, which raises the
**MSRV from 1.88 to 1.94**. The security-sensitive crypto stack (argon2,
aes-gcm, sha2, sha1, hmac, rand) is **unchanged** this cycle — that migration
stays deferred until argon2/aes-gcm ship stable (tracked separately). No schema,
route, auth, permission, or migration change to existing models.

### Features

- Adaptive, deterministic **view layer** for models, a **View designer**
  (`/admin/dev/view-designer`), and the first **Studio** phases: in-admin
  branding + theme wizard, a row-based composition editor, a read-only schema
  review page, and deterministic `schema.json` import.
- `examples/shop`: Swedish-first payment methods and a framework repalette to a
  rust accent + JetBrains Mono. Sign-out is now a POST form (not a GET link).

### Companion & docs

- **`rustio-draft`** (natural-language brief → `schema.json`) was built (F1–F5)
  and **relocated to its own repository**; the integration docs remain here.
- New project docs (`docs/project-status.md`, `docs/versioning.md`), a
  professional sponsorship foundation (`SPONSORS.md`, `docs/sponsorship.md`,
  `docs/commercial-model.md`, `docs/verticals.md`, `.github/FUNDING.yml`), and an
  elevated positioning pass across the README and sponsorship docs.

*Released 2026-07-03 — tag `v0.31.0`, GitHub Release, and published to crates.io.*


## Releases at a glance

| Version   | Date       | Headline                                                                          |
|-----------|------------|-----------------------------------------------------------------------------------|
| **0.31.0** | 2026-07-03 | **Adaptive view layer + Studio, sqlx 0.9 / MSRV 1.94, `rustio-draft` split out, docs & sponsorship.** Rolls up ~3 weeks on `main` since 0.30.0. **Breaking:** sqlx 0.8 → 0.9 raises the **MSRV to 1.94**; the crypto stack is untouched (that migration stays deferred). Adds an adaptive, deterministic **view layer**, a **View designer** (`/admin/dev/view-designer`), and the first **Studio** phases (branding/theme wizard, composition editor, read-only schema review, deterministic `schema.json` import). `examples/shop` gains Swedish-first payment methods and a rust-accent + JetBrains Mono repalette; sign-out becomes a POST form. The **`rustio-draft`** companion (brief → schema) was built and **moved to its own repo**. Documentation: new `project-status` / `versioning` maps, a professional sponsorship foundation, and an elevated positioning pass. No schema / route / auth / permission / migration change to existing models. |
| **0.30.0** | 2026-06-11 | **Admin polish — comfortable dark theme, working ⌘K search, cleaner lists & forms.** The dark theme is re-tuned off near-black slate to a **comfortable graphite-grey** (`--rio-bg #22272e`) with clear surface steps and visible dividers, so elements separate logically and long sessions stay easy on the eyes. The top-bar **search now works**: the `⌘K` palette (grouped, keyboard-navigable, backed by `/admin/_search`) was implemented in JS but never given markup — now wired up. List pages **hide the `id` column** by default (the row links to the record; an explicit `list_display` keeps it). The page **footer goes full-width** with content aligned to the page column. A **subtle elevation pass** (darker hairline + softer surface shadows) lifts every card / panel / table off the near-white background, and the bespoke "console" pages (dashboard, lists, users/groups, sessions, audit, db browser) are brought to full Visual-Contract conformance — Inter labels everywhere, no grey header bands, `·` breadcrumbs, teal permission grid, §9 text-link row actions, and inline related sections now spaced cleanly from the action bar. The `docs/visual-reference/` set is refreshed and **mirrored in dark** under `docs/visual-reference/dark/`. Top bar / sidebar / footer chrome stays frozen; no schema, route, auth, permission, or migration change. |
| **0.29.0** | 2026-06-10 | **Admin Visual Contract conformance — Inter + slate + teal, dark theme, font cleanup.** The admin content area is brought to pixel conformance with the Visual Contract (`docs/design/VISUAL-CONTRACT.md`, v2.1): **Inter** for body and titles, a slate neutral ladder, the teal `#119588` accent, a `#fafcfb` background, 44px controls with a 4px teal focus ring, the three §3 section patterns (legend-on-border / eyebrow / legend-less by page shape), the §8.2 action bar, and no-zebra tables with lowercase status pills — in **light and dark** (token-driven, no per-component dark CSS) and **LTR + RTL** (logical properties + legend/eyebrow letter-spacing neutralization). Retired the unused baked font faces (Spectral, Hanken Grotesk, Geist, Geist Mono): their `@font-face` / woff2 / `include_bytes!` / routes are gone, those `/static/fonts/*.woff2` now **404**, the binary is ~340 KB smaller, and `LICENSE.txt` maps 1:1 to the shipped faces (+ JetBrains Mono OFL). The build-time `rio-theme` emitter now produces dual-block dark-aware `tokens.css` (`DarkPolicy`: auto / light-only / explicit), and the runtime logs a startup WARN on a light-only `RUSTIO_TOKENS_CSS` override. Ships a ten-image visual reference set under `docs/visual-reference/`, plus a doctrine-doc refresh that points all token values at the contract. Two builtin labels change ("Old password" → "Current password"; create-form "Password" → "Temporary password"). **Top bar / sidebar / footer frozen; no functional, schema, route, auth, permission, or migration change.** |
| **0.28.0** | 2026-06-08 | **One declaration per model.** `#[derive(RustioAdmin)]` now also emits the `orm::Model` impl (`TABLE` / `COLUMNS` / `INSERT_COLUMNS` / `id` / `from_row` / `insert_values`) from the same field walk it already runs to build `AdminModel` — the hand-written ORM glue that had to be kept in sync with the struct by hand is gone. Two struct-level escape hatches cover what the walk can't infer: `#[rustio(table = "…")]` (SQL table name differs from the auto slug) and `#[rustio(extra_columns = ["…"])]` (generated/virtual columns like a `tsvector`). **Breaking:** existing hand-written `impl Model` blocks must be deleted — they now collide with the derive. Runtime deps (`chrono` / `rust_decimal` / `uuid` / `sqlx`, plus the `Decimal` / `DateTime` / `Utc` / `NaiveDate` / `NaiveTime` / `Uuid` aliases) are re-exported from `rustio_admin`, so a model crate can depend on `rustio-admin` alone; the scaffold's `Cargo.toml` drops its direct `chrono` / `uuid` / `rust_decimal` / `sqlx` lines. |
| **0.27.6** | 2026-06-05 | **CSV-import guidance + admin redesign.** A CSV header with columns the model doesn't declare no longer dead-ends on a `400` — the rows import, the skipped columns are reported, and a ready-to-run migration + model snippet is generated to capture them. The docs index, doc viewer, API surface, account-sessions, audit-log, and CSV-import-result pages are restyled with the shared card / badge / glyph language. Also the example-first docs rewrite (`getting-started.md`, `modeladmin.md`). |
| **0.27.5** | 2026-06-03 | **Beginner-first help + a binary-name fix.** `--help` is now a grouped, colour-coded map (Getting started · When you need more · Help) with the `RustIO` wordmark in a distinct gold, instead of a flat wall of 21 commands — the full list still follows below. `rustio-admin startapp --help` became the best help in the tool: example-first (SIMPLE / WITH FIELDS), with the field-type list, relations (`fk:Model`) and choices (`choice:a,b`) shown as snippets. After `new`, a model-less project ends with an "Add your first model" example so you always know the next move. **Fix:** the `ai` / `memory` commands printed next-step guidance using the bare name `rustio` (e.g. `rustio ai apply …`) — but the binary is `rustio-admin`, so copy-pasting ran the sibling `rustio` project and failed with `rustio.schema.json not found`; all printed commands now use `rustio-admin`. |
| **0.27.4** | 2026-06-03 | **First-run terminal experience redesigned.** The `rustio-admin new` wizard and scaffold output were flat, cramped, single-colour text; they now share one calm visual language (a warm-amber accent for the *active* element, green `✓`, cyan paths/URLs, bold commands, dim explanations, comfortable spacing), show **Step _n_ of 3** headers, and list each next command on its own copy-clean line. Commands now point forward **contextually**: after `migrate apply` → *create your admin login* (or *launch* if an admin exists); after `user create` → `cargo run`. The `--help` command list was trimmed to one beginner-readable line per command (internal "Doctrine B8" jargon removed; advanced verbs flagged). The `new` wizard no longer advertises `school`/`inventory` as starters they aren't, and `ai status`/`init` state up front that RustIO runs no AI. **Fixes** a blocker where `startapp` generated invalid SQL for reserved-keyword field names (e.g. `order`) — column names are now quoted in the migration DDL. All colour degrades to plain text under `NO_COLOR`/non-TTY/CI. New doctrine: `DESIGN_ONBOARDING.md` §13 (Visual language). |
| **0.27.3** | 2026-06-03 | **Welcome banner leads with value, not mechanism.** The `rustio-admin` welcome (bare command / `--help`) previously explained the differentiator in governance vocabulary (*authority: who may do what, how sessions end, … governed by checked-in contracts*) before a first-time reader had felt any value. It now opens with what the developer **gets** ("describe your data as Rust structs, get a complete admin panel") and reframes "Why RustIO is different" as the **outcome** — login, roles, and audit trail built in as one system, so the admin is secure and accountable on the first run with nothing to assemble — rather than the machinery that delivers it. Value before mechanics (`DESIGN_ONBOARDING.md` §10). Banner text only — no command, flag, or behaviour change; "Start here" and the full command list are unchanged below the separator. |
| **0.27.2** | 2026-06-02 | **The `clinic` project type scaffolds a real clinic.** Choosing *Clinic* in the `rustio-admin new` wizard (or `--preset clinic`) previously produced the neutral empty scaffold — the menu promised a clinic and delivered a blank project. It now ships a working clinic admin out of the box: a `Patient` model (name, date of birth, phone, email) and an `Appointment` model (foreign key to patient, scheduled time, reason, and a status with a CHECK constraint), their migrations **seeded with example rows**, and a `src/main.rs` that already registers both models. The developer reaches a non-empty, searchable clinic admin behind a login without hand-editing a single file — value before mechanics (`DESIGN_ONBOARDING.md` §6). Also fixes a latent gap where content presets left `{{name_title}}` unsubstituted in `Admin::app_name(...)`. |
| **0.27.1** | 2026-06-02 | **Clarity-first onboarding & docs pass.** A first-time developer now meets *understanding before mechanics* at every entry point. The `--help` / no-args **welcome banner** opens by stating what RustIO is (a Postgres-first admin framework for Rust) and what you get, before the "Start here" block. The interactive **`rustio-admin new` wizard** opens with what it does and what you'll have, prefaces each question with one calm line of *why it matters*, and confirms each step with a sober `✓` (`DESIGN_ONBOARDING.md` §9.1). Docs were reworked to lead with understanding (progressive disclosure): README opens with "What is RustIO?"; `DESIGN_CLOUD.md` gains an "in one minute" preamble; a new `docs/memory.md` guide covers `rustio memory`; and `docs/getting-started.md` was rewritten and corrected to the current scaffold (`rustio-admin` binary; neutral `minimal` preset; models via `startapp --field …`). CLI text + docs only — no behaviour, flag, or API change; the wizard's questions, defaults, and validation are unchanged. |
| **0.27.0** | 2026-06-02 | **Project memory — `rustio memory` (CLOUD.md).** A governed project-memory subsystem: the *why* behind a project (business intent, decisions, **rejected ideas**, accepted assumptions) as per-entry TOML-frontmatter files under `.rustio/memory/entries/` with a generated, human-readable `CLOUD.md` view. **Non-authoritative and subordinate to code** (`docs/design/DESIGN_CLOUD.md` is the contract; `DESIGN_CLOUD_IMPL.md` the design). It reuses the `rustio ai` proposal/approval/log lifecycle **unchanged** — one governance pipeline, extracted into a shared `proposal` module: `write_memory` / `supersede_memory` / `redact_memory` are new `needs_approval` capabilities in the existing `.rustio/ai.toml`; entries are **materialised at apply** (stamping the apply date, approver, and a UUID v7 correlation id), **status is derived** (never stored), and the supersession graph is referential-integrity-checked (dangling/cyclic links rejected; a merge fork surfaces as an open tension). **Redaction** is the single bounded append-only exception (two approvers; in-place body excision to a recorded marker; **not** git-history scrubbing — surfaced to the operator). **`--as <email>`** authenticates the approver against `rustio_users` and mirrors decisions to `rustio_admin_actions` (reusing `ai_proposal_*`; new typed **`memory_redacted`** event). Verbs: `render` / `show` / `verify` (freshness + referential + git working-tree tamper backstop) / `remember` / `supersede` / `pending` / `approve` / `reject` / `apply` / `redact` / `chain` / `stats` / `promote-candidates` / `index`. Offline-first (`--by`); the running admin never reads CLOUD.md. **No new dependency, no second runtime, no schema-driven sibling** — TOML frontmatter rides the already-present `toml_edit`. |
| **0.26.0** | 2026-06-02 | **Field-type vocabulary expansion — eight → sixteen `startapp --field` types.** Adds `float` / `date` / `time` / `decimal` / `uuid` / `email` / `phone` / `choice`, each flowing end-to-end through CLI codegen, the `orm::Value` / `Row` layer, the `#[derive(RustioAdmin)]` macro, and the admin UI widgets. `email` / `phone` are validated strings carried by a `#[rustio(format = "…")]` attribute (new public `is_valid_email` / `is_valid_phone` helpers); `choice:v1,v2,…` emits a `CHECK (col IN (…))` constraint and renders a `<select>` via the existing `AdminField.choices` path (no new `FieldType` variant); `decimal` / `uuid` surface as JSON / OpenAPI strings to avoid precision loss. New dependency `rust_decimal` (already in-tree transitively — zero new crates pulled); the `rustio startproject` scaffold gains `uuid` + `rust_decimal` so generated models compile. The CLI's parallel type-mapping `match`es were first unified behind one `FieldKind::spec()` descriptor (pure refactor). No breaking changes — `FieldType` is `#[non_exhaustive]`; existing scaffolds are byte-identical. |
| **0.25.0** | 2026-05-31 | **Full-text search reliability.** Two fixes to the `ModelAdmin::search_index_column()` opt-in: (1) `Admin::model::<M>()` now logs a `warn!` at registration when the FTS column isn't in `Model::COLUMNS` — previously the list search silently degraded to a slow `ILIKE` scan with no signal; (2) the list `SELECT` now **excludes** the FTS column from its projection (it's only needed in `WHERE`), so the unused `tsvector` is no longer fetched per row. `docs/modeladmin.md` documents that the FTS column must be listed in `COLUMNS`. No API change. |
| **0.24.0** | 2026-05-31 | **`/admin/docs` no longer renders dead cross-document links.** The in-app docs viewer serves only the three embedded files (`architecture` / `modeladmin` / `public-api`) by slug, so the relative cross-doc links those pages carry (`design/…`, `archive/…`, sibling `*.md`) rendered as 404-on-click anchors. `admin::docs::render_markdown` now strips any non-navigable link to plain text (keeping its label); external URLs (`http`/`https`/`mailto`) and in-page `#anchor` links are untouched. The canonical `docs/` copies keep their links for GitHub browsing. |
| **0.23.0** | 2026-05-30 | **AI Assistant Permissions — a permissions / approval / audit layer over external AI coding assistants.** New `rustio ai` surface, offline by default: `status` / `init` read a version-controlled `.rustio/ai.toml` policy that sorts a fixed capability catalogue into Allowed / Needs approval / Blocked; `propose` → `review` → `approve` / `reject` → `apply` runs a proposal lifecycle (rules snapshotted at creation, distinct approvers enforced, Blocked refused at propose, `apply` writes staged files); `log` renders the local `.rustio/ai/log.jsonl` record; `allow` / `deny` edit policy buckets in place (comment-preserving, diffed). **DB-backed identity + audit mirroring is opt-in** via `--as <email>` on approve/reject/apply — resolves an active `rustio_users` row whose role meets the policy's `approver_role` and mirrors the decision into `rustio_admin_actions` (three new `AuditEvent` variants: `ai_proposal_approved` / `_rejected` / `_applied`, linked to the local record by `metadata.proposal_id`). Full contract: `docs/design/DESIGN_AI_ASSISTANT.md`. Also in this cycle: **`RUSTIO_TOKENS_CSS`** runtime theme override (a generated `rio-theme` `tokens.css` applied to a running admin without recompiling; new `docs/design/DESIGN_THEME.md`), and a **startup fast-path** that cuts warm-boot idempotent re-work (~138 ms → ~25 ms via a `rustio_admin_meta` version/fingerprint stamp). |
| **0.22.0** | 2026-05-29 | **Binary renamed `rustio` → `rustio-admin`.** The `rustio-admin-cli` crate now produces a binary named `rustio-admin` instead of `rustio` to clear a shell-level collision with an unrelated upstream RustIO project that also publishes a `rustio` binary — back-to-back `cargo install` runs silently overwrote each other. crates.io package names are unchanged; only the emitted executable changes. Every user-facing `rustio <verb>` example in README, ROADMAP, CLAUDE.md, active design docs, the bundled `crates/rustio-admin-cli/templates/` scaffold (newly-scaffolded projects' READMEs and `main.rs` reference `rustio-admin`), library doc comments, admin-template empty-state CTAs (`/admin`, `/admin/health`, `/admin/db-browser`), `examples/.env`, the CI scaffold-pin-guard comment, and CLI error / hint strings is renamed. **Deliberately preserved as on-disk / runtime contracts:** the `@generated by rustio <ver>` codegen sentinel in `builder/codegen.rs` (SchemaHash header), the `.rustio/` state directory (`draft.toml`, `history.jsonl`, `builder.lock`), the `// rustio:` insertion markers in scaffolded `src/main.rs`, the `rustio_admin_actions` audit table, and the `RUSTIO_TEMPLATE_DIR` / `RUSTIO_SECRET_KEY` env vars — renaming any of these would silently break running deployments or diverge newly-generated files from previously-generated ones for any project that scaffolds across the version bump. Migration: `cargo install rustio-admin-cli` after this release installs `~/.cargo/bin/rustio-admin`; the previous `~/.cargo/bin/rustio` (from older installs) is left as-is — `cargo uninstall rustio-admin-cli` from an earlier install removes it, or delete it by hand. Shell scripts, CI steps, Dockerfiles, and project docs that invoke the binary directly need a one-token find-and-replace. No library API change; the `rustio-admin` crate is byte-identical except for the version bump. |
| **0.21.1** | 2026-05-26 | **Admin list-page redesign — Find/Arrange toolbar, framed surfaces, dark thead.** The pre-0.21 "View ▾" mega-dropdown split into two zones with distinct intent: Row 1 Find (search + ≤2 promoted filter chips + More filters overflow + Reset) and Row 2 Arrange (Sort + direction toggle + Rows + Saved view + Export + Import). Find sits inside a soft `--rio-accent-soft` panel; Arrange stays transparent. Table header re-skins to `--rio-surface-chrome` (sidebar slate-900) via a thead-scoped chrome cascade — same trick `layout/shell.css` uses on the sidebar — with active-sort accent lifted to teal-400 for 9.6:1 contrast on the dark bar. New `SortFieldCtx` collapses the Sort menu from field×direction (17 rows on an 8-column model) to one-row-per-field plus a direction toggle that reads field-aware copy ("newest first" / "highest first" / "Z → A" / etc.). Numeric columns gain "highest first" / "lowest first" instead of generic ascending/descending. New CSS file `pages/list.css` scoped under `.rio-list-page`; new icons `inbox` / `rotate-ccw` / `arrow-up-down`; stronger empty state (icon + h2 + sentence + CTA); bottom bar ("Showing N of M"). **Pure CSS / template / render-context change** — no schema, no auth, no permission, no migration, no public-API impact. Forms, sidebar, topbar, auth, docs, dashboard untouched. Existing 0.21.0 projects update by bumping `rustio-admin = "0.21.0"` to `"0.21.1"` in their `Cargo.toml`. |
| **0.21.0** | 2026-05-25 | **Stage 2 — first-model + structural permission defaults.** Four PRs: `rustio startapp --field <name>:<type>` accepts a closed 8-token vocabulary (`str / text / int / bigint / bool / timestamp / json / fk:<Model>`) and produces a real model file + matching migration in one CLI breath; structural permission defaults (`administrator`, `editor`, `viewer` — three groups seeded on fresh databases, lockstepped to `rustio user create --role` values, guarded against re-shaping existing projects); `startapp` pluralization fix (audit-broken `class → classs` now `class → classes`; `bus → buses`; `category → categories`); spatial-orientation `startapp` output with three `// rustio: modules / imports / models` insertion markers in the scaffolded `main.rs` + structured edit instructions referencing them; `rustio docs` becomes useful (probes local server via `x-correlation-id` specificity check, `--open` launches default browser); homepage gains a `sign in required` hint under the nav pills. New doctrine doc `docs/design/DESIGN_PERMISSIONS.md`. **No API breaks** — `Role` enum unchanged; existing 0.20.x projects update by bumping the pin. New deps: `console = "0.15"` (already transitively in tree via indicatif). |
| **0.20.0** | 2026-05-25 | **First-time developer experience overhaul.** The headline release of Stage 1: friendly `rustio new <name>` alias; calm interactive TTY wizard (project name → project type → DB name → auto-`.env`); first-boot homepage at `/` with project-type-aware subtitle and three CSS custom-property tokens for re-skinning; four-part humanised onboarding errors (DATABASE_URL missing, PG unreachable, DB missing, migration SQL failure, invalid `--role`) with verbatim sqlx text preserved in a `Details:` block; calm ASCII spinner during long ops; first-build expectation note. Default scaffold is neutral — no `post.rs`, no posts migration, no blog assumptions; those moved into the `blog` preset where they belong. Stage 0 doctrine doc (`docs/design/DESIGN_ONBOARDING.md`) governs the whole FTUX surface. **Polish & Trust runtime fixes (Stage 1 Reality Audit follow-up):** Postgres NOTICE chatter silenced at the connection layer (boot log: 65 lines → 2); duplicate framework `listening on` log removed; project name flows into admin tab title and login chrome via `Admin::new().app_name(...)`; admin empty-state CTA leads with `rustio startapp <name>` instead of bare Rust; scaffold README restructured into five layered sections matching the wizard reality; `-----` ASCII em-dash replacement reduced to `--` for cleaner mixed RTL/LTR rendering. **No API breaks** — `app_name()` was already in 0.19.0; existing 0.19.0 projects update by bumping the pin. Migration engine semantics unchanged. |
| **0.19.0** | 2026-05-24 | **Visual overhaul — "Quiet Expert" design language.** Six-turn pass over every admin template: foundational primitives (`.rio-section`, `.rio-empty-state`, `.rio-confirm` block w/ `--danger`/`--neutral`, `.rio-env-chip`, `.rio-page-actions__group`), unified stat tile (single white surface + 3-px accent rail instead of cycling pastel fills), unified page-header pattern across every list / detail / form / confirm / settings / admin-tool page, area-chart sparkline replacing bars (gradient fill + baseline rule + per-day dots), shared `_row_actions.html` kebab partial powered by `<details>` + JS upgrade that escapes `.rio-list { overflow: hidden }` via `position: fixed` anchored to the toggle, `--rio-z-*` z-index ladder (sidebar/topbar/dropdown/modal) fixing the "sidebar overlaps topbar on scroll" bug, demo-row seed in the scaffold so freshly-bootstrapped projects aren't a blank slate, dashboard greeting trimmed (no emoji, no filler), MFA settings pages migrated from auth-style `.rio-login` shell to the in-chrome settings pattern. **No backend change** — every Rust handler, route, model, SQLx query, and migration is identical to 0.18.x. Diff is CSS / templates / JS / one new icon (`more-horizontal`). |
| **0.18.6** | 2026-05-23 | **Professional polish pass on top of 0.18.5.** Six refinements after side-by-side screenshot review: (1) topbar search trigger goes wide — `flex:1` so search visually dominates the chrome instead of sitting as a small pill (Linear / Stripe / Vercel pattern). (2) List toolbar reflows to a 2-row pattern — Row 1 = full-width search; Row 2 = secondary controls (Filters / Saved / View) — addresses the "search bar is crowded" feedback at a deeper structural level than 0.18.5. (3) Per-row table actions collapse into a kebab `⋯` dropdown — Linear / Notion / Stripe / GitHub pattern; new `more-horizontal` icon plus `--rio-row-actions` toggle CSS, plus a `.rio-dropdown-item--danger` variant for the Delete row. (4) Dashboard greeting tightens from a 3-line Slack-style hello (emoji + filler "Manage X from one console") to a single page title + app name. (5) Recent-activity sparkline replaced — bars give way to a smooth area chart with gradient fill, polyline data line, baseline rule, and a circle dot at each measurement (Vercel / Grafana / Stripe analytics pattern). (6) DEVELOPMENT badge color confirmed amber (was already correct in the CSS — false-alarm). All features preserved; no library API change. |
| **0.18.5** | 2026-05-23 | **Topbar + list-toolbar redesign + sparkline fix + Post seed.** The topbar's pre-0.18.5 inline strip (Signed-in · 🔔 · Enable MFA · Sessions · Change password · Log out) folded into a single account dropdown anchored on a one-letter avatar + the user email — bell stays on the chrome. List-page toolbar collapsed from 8 controls (Search input · Search btn · Filters · Sort · Per-page · Saved · CSV · Import CSV) to 4 (Search · Filters · Saved · View), with Sort / Per-page / CSV-export / CSV-import folded into a new "View" overflow menu — no features lost. Dashboard sparkline gains a baseline rail per day so empty days anchor the chart; nonzero bars get a 4-px minimum so small counts don't disappear next to large ones. Scaffold template's `Post` migration ships 5 demo rows spread across the last week so a freshly-bootstrapped admin isn't a blank slate. Three new inline-SVG icons (`sliders`, `upload`, plus reused `download`). New utility class `.rio-visually-hidden`. |
| **0.18.4** | 2026-05-23 | **First publish since v0.13.0.** `admin::docs` was reaching `include_str!("../../../../docs/<file>.md")` outside the crate root — fine in-tree, but `cargo publish` builds from a tarball that contains only the crate's own files, so the verification failed. Latent since v0.16.0 (no version since v0.13.0 was published). Fix copies `architecture.md` / `modeladmin.md` / `public-api.md` into `crates/rustio-admin/assets/docs/` and updates the three `include_str!` paths. Workspace docs remain at `docs/` as the canonical source; the in-crate copies are bundled at release time. All four workspace crates (`rustio-admin`, `rustio-admin-macros`, `rustio-admin-cli`, plus the new `rio-theme` — first claim of the name) published to crates.io at 0.18.4 in dependency order. |
| **0.18.3** | 2026-05-23 | Patch: ships green CI. v0.18.2's release commit pushed through but the tagged tree itself had two pre-existing failures — pre-existing rustfmt drift in `crates/rio-theme/` (never run through `cargo fmt` since the crate landed in 0.17.0) and a `clippy::uninlined-format-args` violation in `color.rs` that CI's pinned `rustc 1.88` rejects but newer local toolchains let through (same lint class as `bcdf9b6`). Both fixed (`e5e4900`, `dbe4951`); CI now runs all 15 steps green including the new scaffold-template-pin guard verified end-to-end against simulated drift on PR #2. |
| **0.18.2** | 2026-05-23 | Patch: new CI guard `scaffold template pin tracks workspace minor` extracts `[workspace.package].version` from `Cargo.toml` and the `rustio-admin = "X"` pin from `crates/rustio-admin-cli/templates/project/Cargo.toml.tmpl`, fails the build with an actionable `::error::` line when they disagree. Backstop for the drift that bit 0.17.0 / 0.17.1 / 0.18.0; v0.18.1 patched the immediate drift, this guard prevents the recurrence. Tested locally by temporarily reverting the template pin — guard tripped with the expected message. |
| **0.18.1** | 2026-05-23 | Patch: the `rustio startproject` scaffold template pinned `rustio-admin = "0.16.0"` through three releases (0.17.0, 0.17.1, 0.18.0) — and `0.16.0` is not on crates.io anyway (latest published: 0.13.0), so any fresh scaffold hit a registry resolve failure. The template now tracks the workspace minor. Surfaced during local verification of the v0.18.0 engine-generated `colors.css` render. |
| **0.18.0** | 2026-05-23 | **Engine becomes the source of truth for `colors.css`.** The framework's live `tokens/colors.css` is now generated by `rio-theme` from a single brand input (`#0d9488` — the existing teal). Eleven new canonical `--rio-brand-*` tokens are added; the existing `--rio-*` vocabulary continues to work via drop-in aliases. Brand-derived tokens (`--rio-bg`, `--rio-border`, `--rio-accent-*`, semantic `*-bg`s) shift to engine-computed values; the slate scaffold (`--rio-surface-*`, `--rio-text-*`, `--rio-border-soft`/`-strong`) is unchanged. `--rio-success` rotates from emerald `#059669` to lime `#48a030` — Case 4 fires because teal brand sits inside the success-anchor collision zone; this is the engine's contract, brands and states stay perceptually distinct. Page canvas (`--rio-bg`) gains a faint teal tint (`#f9fcfb` vs prior neutral slate `#F2F4F7`). Hand-tuned design comments in the prior `colors.css` are lost — the file is regenerated; design rationale moves to `docs/design/DESIGN_DOCTRINE.md` and the engine's case definitions in `crates/rio-theme/`. All 18 realistic text-on-surface pairings still clear WCAG. To regenerate with a different brand: `rustio theme generate --brand "#<hex>" --out crates/rustio-admin/assets/static/admin/tokens/colors.css`. |
| **0.17.1** | 2026-05-23 | Patch: `rio-theme`'s `soft_bg` derivation was a flat 92% white mix, which produced an amber `--rio-warning-bg` that left the warning foreground at 2.94 contrast — below AA-large 3.0. Verification against the live framework's WCAG pairings surfaced the regression; engine output is now at full parity. Replaced with a contrast-aware loop that walks lighter in 1% increments until the foreground clears `AA_NON_TEXT` (3.0) against the resulting background. New property test runs 7 brand inputs × 3 semantic colors and asserts the WCAG guarantee. |
| **0.17.0** | 2026-05-23 | **Theme engine release.** New build-time crate `rio-theme` turns raw client brand colors into a safe, computed `tokens.css` via a six-case decision pipeline (contrast guard, palette derivation, vivid taming, brand/state collision avoidance, light/dark mode adaptation, multi-color role assignment). Internal math runs in OKLCH; every emitted token is contrast-measured against the page bg before it leaves the engine. New CLI verb `rustio theme generate --brand <hex> --out <path>` (repeatable `--brand`, zero is valid → safe default) prints a per-case effect report including all seven measured contrast ratios. Output carries both the canonical `--rio-brand-*` vocabulary the engine reasons in and drop-in aliases for the live admin template's `--rio-*` tokens, so the generated file can replace `tokens/colors.css` without changes to consumers. Vivid inputs are guarded — neon brands that would fail `AA_NON_TEXT` against the bg are substituted by the tamed surface and the substitution is logged. Achromatic brands short-circuit the semantic-collision resolver (no spurious rotation off the meaningless stored hue). |
| **0.16.0** | 2026-05-21 | **Operations & i18n release.** Major new operator surfaces: `/admin/docs` (rendered framework markdown), `/admin/health` (web counterpart to `rustio doctor`), `/admin/feature_flags` (with `feature_enabled` helper), `/admin/notifications` (with topbar bell + unread badge), `/admin/apis` (OpenAPI 3.0 + TypeScript SDK + interactive playground). Dashboard gains framework-wide + per-model 7-day creation sparklines and "new this week" KPI. Audit history gains per-field diff render, per-actor filter, and date grouping. CSV import (`POST /admin/<model>/import.csv`) pairs with the existing export. JSON content negotiation now covers write paths (success + validation envelopes). Postgres FTS opt-in via `ModelAdmin::search_index_column`. Per-action bulk permission gate; per-model read-only via `Admin::read_only_model`. New CLI verbs: `rustio reload`, `rustio test-init`, `rustio startproject --preset blog`. New `middleware::locale` parses `Accept-Language` → `Locale` request-context value (foundation for the future message catalog). Dark theme retired — framework is now light-only. |
| **0.15.1** | 2026-05-16 | **Refined colour palette — dark-frame chrome.** Page canvas moved from blue-tinted slate to neutral cool grey (`#E5E7EB`); chrome (topbar / sidebar / footer) jumped to deep slate-blue (`#1F2A37`) so the operator skeleton reads as a confident dark frame around the lighter content area — the Linear / Vercel / Notion / Stripe-Dashboard pattern. Dark mode chrome deepened to near-black (`#0A0E14`) so both modes share the "chrome is darker than canvas" convention. A new chrome-scope CSS cascade in `layout/shell.css` flips `--rio-text-*`, `--rio-surface-2/3`, `--rio-border-*`, and `--rio-accent` (to the lifted `#3FAA9D` variant for contrast) within `.rio-topbar` / `.rio-sidebar` / `.rio-footer` — every component inherits light-on-dark automatically, no per-component edits. Theme-toggle button redesigned as a ghost on chrome. Principle 10 reframed to allow either chrome direction. |
| **0.15.0** | 2026-05-16 | **Visual identity overhaul — calm with authority.** Three new doctrine principles (deeper surface ladder, chrome carries weight, typography hierarchy is a weight choice). Surface scale grows from four rungs to six (`--rio-bg`/`--rio-surface`/`--rio-surface-2`/`--rio-surface-3`/`--rio-surface-chrome`/`--rio-surface-elevated`); chrome (topbar/sidebar/footer) now sits on a distinct deeper tier so the operator skeleton has visible load-bearing weight. Buttons gain a subtle vertical gradient + inset highlight + proper focus-visible ring. Inputs ship with `--rio-shadow-inset` so fields read as "place to type" rather than "drawn rectangle". Table headers retuned to 600 + tracked-allcaps; primary cell in each row gets weight 500 + text-strong as a skim anchor. Topbar height 64 → 72 px. Pure CSS — no template HTML, no public API, no AdminTheme contract change. Existing `AdminTheme` overrides keep working unchanged. |
| **0.14.1** | 2026-05-16 | Patch: Builder migration codegen used a hardcoded 12-char column-name width. Field names ≥ 12 chars (e.g. `engine_displacement_cc`, `detailed_description`) collapsed against the type column, producing malformed SQL like `engine_displacement_ccBIGINT` that Postgres rejected. Replaced with dynamic per-table width + two literal spaces of guaranteed separation. New regression test. No public API change; no SchemaHash input change. |
| **0.14.0** | 2026-05-15 | **Builder MVP** — first release of the deterministic project compiler under `crates/rustio-admin-cli`. New `rustio new / add model / add field / plan / commit` verbs with append-only `.rustio/history.jsonl`, canonical-TOML `.rustio/draft.toml`, version-pinned `.rustio/builder.lock`, and SchemaHash-protected `src/_generated/`. Implementation-grade `DESIGN_BUILDER.md` doctrine (13 numbered invariants, B1–B13) with five CI-enforced grep proofs. MSRV bumped 1.80 → 1.88 to track transitive deps. **Intentionally limited**: no Studio, no Advisory AI, no incremental migrations, no relations / themes / undo / import. |
| **0.13.0** | 2026-05-13 | Phase G privatisation pass: 59 items annotated `// internal:` (since 0.9.0) flipped from `pub` to `pub(crate)`. Public surface narrowed by ~17% (419 → 360 items). Pairs with new `DESIGN_EMAIL.md` doctrine codifying the framework-emitted email conventions stabilised in 0.12.0. |
| **0.12.0** | 2026-05-13 | Three substantial threads: public bulk-action dispatch hook (closes D.4); production password-recovery flow with real SMTP + polished HTML email + project-identity branding architecture (RustIO name no longer leaks to end users); operator-DX `rustio doctor email` with provider presets, `--html-preview`, send cooldown, and a formal `.env` developer contract. End-to-end verified against real Gmail delivery. |
| **0.11.0** | 2026-05-13 | Multilingual typography (Inter + Thai + Devanagari + locale-gated CJK; Noto Naskh promoted to primary Arabic face) + production three-column admin footer with environment badge, render timestamp, real operational links. New `DESIGN_CHROME.md` doctrine. |
| **0.10.2** | 2026-05-13 | `permissions::create_group` is now idempotent (mirrors the `permission_id` ON CONFLICT idiom). Closes one of the two framework gaps that the canonical example documented. |
| **0.10.1** | 2026-05-13 | Integration-pass bugfixes: `plural_snake` learns regular English rules (`-ch/-sh/-x/-z`, consonant+`y`); removed offensive defaulting that synthesised `draft/published` for any field named `status`. Canonical example declares `belongs_to` on its FK fields. |
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

### Changed
- **View designer — Composition editor (Studio Phase 1).** The per-model
  `/admin/dev/view-designer/<model>` page is reworked from a plain form into a
  row-based editor: a live preview on top (with filter chips, server-rendered
  through the runtime render core), one card per field (drag grip, role-coloured
  dot, role dropdown, a Filter toggle, priority + up/down reorder arrows, a
  "Composed" badge), the compositions slots, and a read-only generated-`ViewSpec`
  panel showing the saved JSON. Reordering rewrites the existing `priority__<f>`
  inputs from row order via a small progressive `initViewDesigner()` in
  `admin.js` — with JS off the native role/filter/priority controls still work,
  and the **save contract is unchanged** (`role__`/`priority__`/`filter__`/
  `mode_allowed__`/`comp*`). New `pages/view-designer.css` fragment (token-only,
  added to the `admin.css` ↔ `routes.rs` lock-step); the editor still only
  *produces* a `ViewSpec` and the preview reuses the deterministic renderer — no
  AI, no client-side data rendering.
- **Adaptive view layer — polished card layout.** The `?view=cards` mode
  (`components/adaptive-views.css`, `.av-list--cards` scope only) was a flat
  single-column stack: title, each secondary, and each badge on its own line.
  Cards now compose from the same flat cells via flex line-control — the title
  leads, the secondaries read as one inline meta line (`provider · country`,
  joined by a `·`), and the badges group together right after it. Columns widen
  to 300px and cards size to their content (`align-items: start`) so nothing
  stretches or strands a badge. Token-only — no new tokens, no markup change
  (the deterministic partials are untouched), and `list`/`compact` modes are
  unaffected.
- **Unified the chrome on five utility pages — Audit log, API surface, Health,
  Docs (index + viewer), and Sessions.** They had drifted onto four different
  page-header patterns (`.rio-masthead-top`, `.rio-page-header`, bespoke
  `.pgx-head`, bespoke `.docp-head`) and three of them carried per-page inline
  `<style>` blocks with bespoke `.pgx-*`/`.api-*`/`.docs-*`/`.docp-*` class
  prefixes. All five now use the one canonical `.rio-crumbs` + `.rio-masthead-top`
  + `.rio-masthead-desc` header (API surface keeps its action buttons in the
  `.rio-masthead-cta` slot), and every inline style/`style="…"` attribute moved
  into the existing `pages/tools.css` / `pages/account.css` fragments under
  `.rio-`-prefixed names (`.rio-api-*`, `.rio-doc-*`, `.rio-hist-*`,
  `.rio-sess-revoke`). One hardcoded `#e6edf6` doc-code colour is now the
  `--rio-on-solid` token. **No new tokens, no markup-behaviour change, no
  `admin.css`/`routes.rs` concat change** (reused existing fragments, so the
  `cascade_lockstep` lock-step is untouched).
- **Brand repalette — rust accent + cool ink neutrals + JetBrains Mono.** The
  framework's visual identity moves from teal (`#119588`) on warm near-white to a
  **burnt-copper rust** accent (`--rio-accent #B84318`, hover `#8F3413`) over a
  **cool blue-grey "ink"** neutral scale on a `#EDF1F5` canvas, matching the
  `rustio` reference project. Titles are `#0F141A` (cool ink), borders/inputs use
  the ink-200/ink-300 steps, and status colours retune to the reference set
  (success `#067647`, warn `#B54708`, danger `#B42318`). Dark theme keeps its
  graphite surfaces but its accent lifts to a warm coral (`#F2935E`) for AA. The
  **mono stack now selects JetBrains Mono** (already self-hosted/baked; no new
  asset) ahead of the system fallbacks. All changes are token-value only — every
  `--rio-*` name is unchanged, so components/templates are untouched and dark mode
  re-derives automatically. The Visual Contract (§1/§2) and DESIGN_DOCTRINE /
  DESIGN_SYSTEM are updated to the new canonical values. **Migration impact:**
  downstream admins see the new palette/mono after `cargo update`; projects that
  set `Admin::accent_color("#…")` keep their override.
- **Small text nudged up (+1px).** The smallest content tier (`--rio-text-12` /
  `--rio-text-13`: table headers, pills, hints, stat labels) lifts from 14px to
  **15px** for readability; the ≥14px floor (Visual Contract §2) still holds.
- **List/table readability — tighter row density (visual only).** The admin data
  board's row height drops from an over-tall 72px to a comfortable, more
  scannable 56px (header padding 14px → 12px to match), so ~50% more rows fit a
  viewport without crowding. No new tokens (length values only), no markup,
  context, route, or behaviour change — `components`/`tokens` and the
  `cascade_lockstep` lock-step are untouched. First increment of the redesign
  plan in `REDESIGN_AUDIT.md` (Phase 1: presentation-only polish). Verified in
  light and dark against the `shop` example.
- **Visual-Contract conformance pass across admin pages (visual only).** A
  full audit of every admin page against `VISUAL-CONTRACT.md` v2.1, with the
  token-level fixes applied: removed a stray `.rio-pill` redefinition in
  `pages/detail.css` that was overriding the canonical §9 status pill
  (15px/700) globally with a smaller 14px/600 variant — every status pill now
  renders at contract spec; the required-field asterisk is now `--rio-danger`
  (red, §4) instead of the copper accent; confirm-dialog cards get the §3
  card shell on the MFA/password-change confirmation pages (`mfa_disable`,
  `mfa_regenerate`, `password_change`) and their padding moves to s6 (32px);
  the list/users/groups board-foot counts drop hardcoded `font-size:13px`
  inline styles (the only sub-14px content text in linked CSS, §2 floor) in
  favour of a `.rio-board-foot` rule; and the table boolean / user-active
  status pills now emit lowercase text (`yes`/`no`, `active`/`inactive`, §9).
  No new tokens, no markup-structure, route, schema, or behaviour change;
  `cascade_lockstep` untouched. Remaining audit findings (row-action taxonomy,
  dense-button sizing) are deferred as contract-level decisions.

### Fixed
- **Primary buttons rendered as links no longer lose their label on hover.**
  Link-style buttons (e.g. the list page's “＋ Add &lt;model&gt;”, which is an
  `<a class="rio-btn rio-btn--primary">`) were hit by the global content-link
  rule `a:hover { color: var(--rio-rust-hover) }` (`base/base.css`), whose
  `(0,1,1)` specificity outranks the `.rio-btn--primary` `(0,1,0)` class. On
  hover the white label turned `--rio-rust-hover` while the fill darkened to
  `--rio-rust-solid-hover` — the same value — so the text vanished into the
  button. `buttons.css` now pins each variant's label colour on `:hover`/
  `:active` (specificity `(0,2,0)`, which wins), so buttons own their text
  colour in every state regardless of being an `<a>`. The same pin is applied
  to `.rio-action-link` (and its `--danger`/`--muted` modifiers) so a
  destructive “Delete” text-link stays red on hover instead of drifting to
  copper. Token-value-free.
- **Sidebar now highlights the current page.** The command rail's
  `aria-current="page"` state was never rendered: templates compared against a
  `nav_active` key that no page context ever set, so the comparison was always
  false. `BaseContext` gains a `nav_active` field (set via a new
  `with_nav_active` builder), and every rail-linked page context now sets it —
  `home` (dashboard), a model's `admin_name` (its list/create/edit/delete and
  per-object history), `users`/`groups` (the access pages), `history`, `db`, and
  `view-designer`. New `sidebar_marks_active_nav_item` test guards it. No new
  tokens; markup/CSS unchanged (the `aria-current` styling already existed).

### Added
- **`schema.json` import (Studio Phase 4 — deterministic genesis).** New
  `rustio-admin import <schema.json>` loads a schema document into the Builder
  draft: it validates the whole contract first (a `models` array of
  `{ name, fields: [{ name, type, unique? }] }`, `type` from the closed
  `FIELD_TYPES` list), then records the **same** `add_model` / `add_field`
  events as `add`, so `plan` / `commit` apply it unchanged — a malformed schema
  records nothing. **No AI, no network, no new dependencies.** Per the project's
  *RustIO runs no AI* stance, an external assistant (governed by
  `.rustio/ai.toml`) or a human authors the JSON; RustIO only imports it
  deterministically. Surfaced on the Schema page's CLI handoff. (A live-LLM
  genesis tool stays out of the OSS repo — see `DEFERRED.md`.)
- **Schema review page (Studio Phase 3).** A Developer-gated `/admin/dev/schema`
  page (linked in the rail's Developer section) gives a **read-only** view of the
  registered models as the framework sees them — per-model field tables (name,
  type, nullable, relation target) plus a models/fields/relations stat strip —
  built purely from the live `AdminEntry`/`AdminField` registry. Schema *edits*
  are a build-time `builder` CLI step (`add model` / `add field` / `plan` /
  `commit`), surfaced as a copy-paste handoff; the runtime never generates code.
  New `admin/schema.html` (baked into `EMBEDDED_TEMPLATES`), `schema_ctx`/
  `show_schema`, reusing existing card/table/terminal styles. No new tokens, no
  new CSS fragment, deterministic.
- **Branding page + `theme wizard` (Studio Phase 2).** A Developer-gated
  `/admin/dev/branding` page (linked in the rail's Developer section) lets you
  pick a brand colour and **preview it live** — the rust/accent CSS variables
  are injected into the preview pane only (ephemeral, client-side, never
  persisted) — then copy the build-time **bake** commands: `rustio-admin theme
  generate --brand '#hex'`, the `RUSTIO_TOKENS_CSS=…` wiring, and the
  `Admin::accent_color("#hex")` one-liner. The runtime **never links rio-theme**;
  baking stays a setup-time CLI step. The CLI gains `rustio-admin theme wizard`,
  an interactive front door that prompts for a brand colour and defers to the
  existing `generate` engine (same deterministic rio-theme output + contrast
  report). New `admin/branding.html` (baked into `EMBEDDED_TEMPLATES`),
  `branding_ctx`/`show_branding`, a `.rio-brand-*` section in the existing
  `pages/view-designer.css` fragment, and a progressive `initBranding()` in
  `admin.js`. No new tokens; no build step; deterministic.
- **View designer is now reachable from the sidebar.** The Developer section of
  the command rail (`_sidebar.html`) gains a "View designer" link
  (`/admin/dev/view-designer`, Developer-gated) next to Database, so the editor
  is discoverable instead of URL-only. Housekeeping in the same change: removed
  the orphaned `.rio-sidebar`/`.rio-nav-*`/`.rio-topbar`/`.rio-brand` rules from
  `components/navigation.css` (dead since the chrome moved to `.rio-rail` /
  `.rio-ws*`; the live `.rio-brand-word` rule is kept). No new tokens; no visual
  change to any rendered page.
- **View designer: allowed-modes + compositions editing.** The designer now edits
  the rest of the `ViewSpec`: an **allowed-modes** checkbox group (which layouts
  appear in the `?view=` switcher; the default mode is forced on), and up to three
  **composition slots** that merge a primary field with chosen secondaries into a
  single cell (with a label and a Stacked / Inline-icon / Badge-inline style). The
  save handler parses `mode_allowed__<slug>` and `comp<i>_primary|label|style|
  sec__<name>`; empty or invalid slots are dropped. Adds `ComposeStyle::slug`/
  `from_slug`/`all`. Server-rendered (no new JS). Verified by form→spec parsing
  tests and a full editor-template render of the new sections.
- **Styling for the adaptive card / list / compact layouts.** New token-only CSS
  fragment `components/adaptive-views.css` styles the view-layer render output
  (the `?view=cards|list|compact` modes and the designer preview): a card grid,
  divided list/compact rows with hover, primary/secondary/timestamp cell type,
  semantic badges, and the `.rio-view-modes` switcher (active pill in the teal
  accent). Registered in the `admin.css` `@import` list and the `ADMIN_CSS`
  concat in lock-step (after `components/code.css`). **No new tokens** — reuses
  existing surface/line/shadow/status-tint/accent/typography `--rio-*` tokens, so
  dark mode works automatically (no per-component dark CSS). The view layer's
  badge span is namespaced `av-badge` (was an unprefixed `badge`). Booleans in
  adaptive cells render as **Yes/No** (matching the legacy table's pill, not raw
  `true`/`false`), and badges no longer stretch full-width inside cards.

- **Live list pages consume a saved `ViewSpec` (opt-in adaptive modes).** When a
  model has a spec saved via the view designer, its list page gains a mode
  switcher and renders `?view=list|cards|compact` through the runtime
  `view_layer::render_view`; `?view=table` (and the no-spec case) stays the
  **existing table, byte-for-byte unchanged** — search, filters, sort,
  pagination, bulk actions, FK links, search highlighting, and edit/delete all
  remain on the legacy table path. The data pipeline is untouched (same
  `ConcreteOps::list`); only the rows region switches, behind a fallback (a spec
  lookup failure logs and keeps the table). `RenderedRow` gains an optional `id`
  (via new `render_view_with_ids`) so adaptive cards/list rows link to the
  record. New `_list_adaptive.html` partial; `list.html` wraps just its data
  board in an additive `{% if adaptive %}…{% else %}<legacy table>{% endif %}`.
  Guarded by a full-`list.html` render test for **both** branches (legacy table
  intact vs. adaptive swap) plus `adaptive_for_list` unit tests — hidden fields
  never reach the adaptive HTML.
- **View designer page at `/admin/dev/view-designer` (Developer-gated).** The
  first editor for the adaptive view layer: lists project models, and per model
  shows the effective `ViewSpec` (the saved one, or a deterministically inferred
  draft when none exists) as an editable form — per-field role, priority, and
  filterable, plus the default view mode — with a **live preview rendered through
  the exact runtime `view_layer::render_view`** (placeholder rows, so the page is
  side-effect-free). **Save is the only authority:** it writes the spec to a new
  global per-model store (`rustio_admin_view_specs`, lazy `ensure_table` like the
  feature-flags/saved-filters stores — no migration) and emits an `Update` audit
  row. Inference still only *produces* a draft; nothing reads a spec dynamically.
  Routes are gated on `Role::Developer` and registered before the generic
  `/admin/:admin_name`. Additive: the live list page is **not** wired to consume
  saved specs yet (a deliberate follow-up) — it still uses the existing table
  path. Adds `FieldRole::slug`/`from_slug`/`all`. Verified by pure form→spec
  parsing tests, a shipped-template render test, and a Postgres store round-trip
  (`integration_view_specs`).
- **Adaptive View Layer (`view_layer`) — presentation-only, additive.** A new
  module that lets a model record *visual importance* (which the schema can't
  express) once, as a stable, serde-serializable, versioned `ViewSpec`, and
  render generated tables/lists/cards deterministically from it. Roles
  (`Primary`/`Secondary`/`Badge`/`Timestamp`/`DetailOnly`/`Hidden`), modes
  (`Table`/`List`/`Cards`/`Compact`), and cell compositions
  (`Stacked`/`InlineIcon`/`BadgeInline`) are all explicit on the spec.
  `infer_view_spec` produces a sensible draft deterministically (no AI, no
  network) — sensitive names (`password`/`hash`/`pin`/`token`/`secret`/
  `api_key`/`private_key`/`session`/`reset_token`, substring-matched) and UUIDs
  become `Hidden`; the first human-readable text becomes `Primary`; enum/bool
  become filterable badges; audit timestamps and nullable secondary text become
  `DetailOnly`; FKs stay `Secondary`. **Runtime contract:** rendering reads the
  *saved* spec and nothing else — inference and the future designer only ever
  *produce* a spec. The renderer emits typed, `kind`-tagged `RenderedCell`s;
  templates switch on `cell.kind` and hold no role logic, and `Hidden` values
  are never read from a row so they cannot reach a context or the HTML (proven
  by `safety_tests`). Purely additive: when a model has no saved spec, callers
  fall back to the existing table path unchanged — no search/filter/sort/
  pagination/action/permission behaviour is touched, and no designer is built
  yet (future `/admin/dev/view-designer`). Wired as `pub mod view_layer` in
  `lib.rs`; two additive minijinja partials ship under
  `assets/templates/admin/view_layer/`.

### Changed
- **Dependency refresh (no behaviour change).** Bumped a tier of non-security,
  non-database dependencies to their latest major versions and re-resolved the
  lockfile. Direct major bumps:
  `thiserror` 1 → 2, `testcontainers` 0.23 → 0.27 and `testcontainers-modules`
  0.11 → 0.15 (integration harness, behind the `integration-test` feature),
  `toml_edit` 0.22 → 0.25 (Builder), `console` 0.15 → 0.16 and `indicatif` 0.17
  → 0.18 (CLI onboarding output). No source changes were required; fmt, clippy
  `-D warnings`, the full unit/doc suite, and the integration suite all stay
  green. The security-sensitive crypto stack (`argon2`, `sha2`, `sha1`, `hmac`,
  `aes-gcm`, `rand`) is intentionally left in place — its major bumps are coupled
  (shared RustCrypto `digest` trait versions), touch doctrine subsystems, and the
  upstreams (`argon2 0.6`, `aes-gcm 0.11`) are still release candidates, so they
  belong in a separate, individually reviewed migration once stable.
- **`sqlx` upgraded 0.8 → 0.9; MSRV raised Rust 1.88 → 1.94.** sqlx 0.9's own
  MSRV is Rust 1.94, so the workspace floor moves with it (`Cargo.toml`
  `rust-version`, the three `dtolnay/rust-toolchain@1.94` pins in CI, and the
  `getting-started` prerequisite). **This is a breaking change for anyone
  building rustio-admin on Rust 1.88–1.93.** sqlx 0.9's headline change is the
  new `SqlSafeStr` guard: `query()`/`query_as()`/`query_scalar()` now accept only
  `&'static str` or an explicit `AssertSqlSafe(_)` wrapper. The framework's
  dynamically-assembled SQL — the `ConcreteOps` list builder, the generic CRUD
  helpers in `orm.rs`, FK hydration / dashboard / dropdown queries in
  `handlers.rs`, the audit-log filters, the session-timestamp loads, the
  reset-token purge, the migration runner, and the CLI audit view — is wrapped in
  `sqlx::AssertSqlSafe` at the 18 call sites that build SQL from validated
  identifiers (column/table names already checked against `M::COLUMNS` or a
  fixed allow-list; all user-supplied values stay bound parameters). No behaviour
  change. No `query!`-style compile-time-checked macros, no `QueryBuilder`, no
  `sqlx::Type`/`FromRow` derives, and no sqlx migrator are used, so the rest of
  the 0.9 surface does not apply. Verified against real Postgres: the full
  `integration-test` suite (emergency/MFA/recovery, 27 tests) plus 444 library
  unit tests, fmt, and clippy `-D warnings` all pass; the `clinic` and `shop`
  examples build clean against the upgraded framework.

## [0.30.0] — 2026-06-11

### Changed
- **Dark theme re-tuned to a comfortable graphite-grey.** The dark palette moved
  off the near-black slate (`--rio-bg #0f172a`) to a softer graphite
  (`#22272e`) with clearer surface steps (`--rio-surface #2d333b`, `--rio-raised
  #373e47`) and a visibly lighter divider (`--rio-line #444c56`), so cards and
  tables separate logically instead of dissolving into the background. Text lifts
  to a soft, legible grey ladder (`--rio-text-hi #e6edf3`, `--rio-text #adbac7`).
  The aim: a dark mode that's pleasant for long sessions, not gloomy. Token-only;
  the lifted teal accent and AA contrast are preserved. Applies to both the
  `@media (prefers-color-scheme: dark)` and `[data-theme="dark"]` blocks.
- **Subtle elevation pass for surfaces.** The card/divider border token
  (`--rio-line`) is darkened a step (`#e2e8f0 → #d6dee9`) and the surface shadows
  (`--rio-shadow-sm`, `--rio-shadow-card`) gain a soft second layer, so every
  card / panel / table / input separates cleanly from the near-white background.
  Token-only, light theme; dark is unchanged.
- **Admin list pages hide the `id` column by default.** When a model doesn't set
  `list_display`, the primary-key `id` column is dropped from the table (the row
  still links to the record, and `id` is shown if a project lists it explicitly).
  Keeps lists focused on meaningful columns. An id-only model is guarded so the
  table never loses all columns.
- **Full-width page footer.** The admin footer now spans the whole workspace (a
  surface bar with a top hairline edge-to-edge) with its content aligned to the
  page column and the identity/timestamp pushed to the end — instead of sitting
  inside the constrained content column.

### Fixed
- **Top-bar search now works (⌘K).** The search control linked to a non-existent
  `/admin/search` (404) and the `⌘K` command palette had its full implementation
  in `admin.js` but was never given markup, so it silently no-op'd. Wired the
  trigger + palette dialog to the existing handler: ⌘K (or clicking the bar) opens
  a grouped, keyboard-navigable palette backed by `/admin/_search`.

## [0.29.0] — 2026-06-10

### Changed
- **Admin design overhaul — the "RustIO Console" at an operator-comfort
  scale.** Every admin surface (dashboard, lists, forms, confirms,
  errors, login + recovery/MFA, users & groups + permission grid, account
  / sessions, audit log, db browser, feature flags, API surface, docs,
  health, notifications) is rebuilt on one language: a dark **labeled**
  command rail, a teal accent (`--rio-rust*` switched from cobalt to
  RustIO Teal), and self-hosted Spectral / Hanken Grotesk / JetBrains Mono
  type. The type scale was rescaled to the px it renders — body is 16px,
  sidebar nav 17px, form labels 16px semibold, inputs 16px on a 46px
  control, tables 16px, page titles 42–48px — with generous form spacing
  (40px cards, 32px field gaps) and a wider 1100px form column. The
  **account menu lives in the top-right header** (sessions / change
  password / two-factor / sign out); the rail is navigation only. Theme
  (light default + dim-slate dark) and rail-collapse toggles are wired and
  persisted. New `tokens/compat.css` maps the legacy token names still used
  by the inline-styled tool pages onto the live DS tokens. **No backend
  change** — CSS, templates, JS, and two `admin_css_payload` test updates
  only; no schema, route, auth, permission, or migration impact.
- **Visual Contract v2.0 conformance — Phase 1 (tokens).** The content-area
  palette is re-valued to the contract's measured reference values (light
  theme only this phase; dark is re-derived in a later phase). Plan of record:
  `docs/design/REMEDIATION_V2.md`. Changed `--rio-*` tokens (light): `--rio-bg`
  `#F3F6FD`→`#fafcfb`; `--rio-line` `#DBE0EB`→`#e2e8f0`; `--rio-line-strong`
  `#C2CAD9`→`#94a3b8`; `--rio-text-hi` `#111722`→`#1e293b`; `--rio-text`
  `#3B4453`→`#475569`; `--rio-text-mute` `#677083`→`#64748b`; `--rio-text-faint`
  `#99A1B2`→`#94a3b8`; `--rio-rust`/`--rio-rust-solid` `#0F8C7B`→`#119588`;
  `--rio-rust-hover`/`-solid-hover` `#0C7567`→`#0e7c72`; `--rio-rust-active`/
  `-solid-active` `#095E53`→`#0b655c`; `--rio-rust-tint`/`-tint-2`/`-ring`
  re-hued to `rgba(17,149,136,…)` with the ring opacity `.38`→`.20`; `--rio-danger`
  `#AE382C`→`#dc2626`; `--rio-danger-tint` re-hued; `--rio-syntax-key`
  `#0F8C7B`→`#119588`. New tokens: `--rio-surface-tint` `#edf7f8`,
  `--rio-accent-focus` `#1f8987`, `--rio-danger-hover` `#b91c1c`,
  `--rio-pill-on-bg`/`-on-text`/`-off-bg`/`-off-text`. New compat aliases:
  `--rio-accent-hover`, `--rio-accent-ring`, `--rio-border-input`,
  `--rio-shadow-card`; `--rio-shadow-inset` retuned to the contract value and
  `--rio-accent-rgb` `15,140,123`→`17,149,136`. The content-area dot-grid texture
  is removed and body `letter-spacing` zeroed (flat `#fafcfb` per the reference).
- **Visual Contract v2.0 conformance — Phase 2 (typography & fonts).** The content
  area moves to **Inter** as the single Latin face for both body and titles (the
  serif display is retired — `--rio-font-display` and `--rio-font-body` now point at
  the contract's Inter stack; the already-baked, already-licensed `InterVariable.woff2`
  gains its missing `@font-face`). `--rio-font-mono` switches to the contract's
  `"SFMono-Regular", Consolas, …` stack. Per-script Arabic fallbacks (Naskh / Tajawal)
  are preserved. Enforced the contract's **14px content-area floor**: the small type
  token `--rio-text-12` `0.8125rem`→`0.875rem` (lifts table headers, badges, hints,
  code chrome, stat labels, and the permission-matrix headers to 14px in one move) and
  the lone literal offender `.rio-dropdown-badge` 11px→14px. Page-title tokens:
  `--rio-fs-display` `44px`→`2.25rem` (36px), `--rio-fs-xs` `13px`→`0.875rem` (14px);
  added `--rio-fs-lead` `1.0625rem` (17px). `h1` now inherits weight **800** /
  line-height **1.15** from the base layer rather than per-page overrides. The frozen
  top bar / sidebar / footer use literal sizes and are untouched. Spectral / Hanken
  faces remain baked for now (separate cleanup commit later).
- **Visual Contract v2.0 conformance — Phase 3 (components).** Inputs, selects,
  textareas and buttons land on an 8px control radius (new `--rio-radius-control`
  token); `.rio-input` gains the inset shadow at rest and a **4px** teal focus ring
  (`--rio-accent-focus` border + `--rio-accent-ring`), and a `.rio-input--mono`
  utility renders identifier placeholders in mono (applied to the feature-flag key
  field). Every `.rio-btn` is now **44px / weight 700 / 8px** at the base — `--md`
  and `--lg` match it, so no per-size opt-in is needed to reach the contract size
  (`--sm` stays a compact exception). `.rio-btn--danger` is solid `#dc2626` with a
  `--rio-danger-hover` hover, replacing the brightness-filter hack; new
  `.rio-action-link` (+ `--danger` / `--muted`) defines the red text-link
  destructive action (templates wire it in Phase 5). `.rio-table th` → 14px /
  weight 800 / `--rio-text-mute` in the body font (Inter, so 800 actually renders),
  cells → `--rio-text-strong` with 16px padding, sticky header kept; zebra/striping
  is removed (`--zebra` / `--striped` are no-ops, no row shading) and a competing
  `.rio-table` redefinition in `pages/tools.css` (a gray mono-header "legacy table"
  that shadowed the §9 table globally) was removed so every table renders the one
  contract look. Inline `code` /
  `kbd` become a `--rio-surface-tint` chip (mono, 6px radius) while the block
  `.rio-code` well is unchanged. Radio rows (`.rio-radio`, §6) become full-width
  52px rows: strong input border, 10px radius, 16px padding, 20px teal control
  aligned to the first text line, 12px gap between rows. No new CSS fragments.
- **Visual Contract v2.0 conformance — Phases 4–7 (sections, action bar, pills,
  header).** §3 **legend-on-border**: `.rio-fieldset` is now a 14px-radius card
  (soft border, `--rio-shadow-card`, 32px padding) whose native `<legend>` cuts the
  top border (body font, 14px/700/uppercase/muted); `feature_flags` migrated from the
  bespoke `.rio-section`/`.rio-card` to fieldsets, and `lock_user` radio rows gained
  the §6 bold-lead + normal-weight description split. §8.2 **action bar**: hairline
  top border, s6 margin / s5 padding, one wrapping row with `.rio-action-bar-end
  { margin-inline-start:auto }` (the flex spacer is retired); History/Delete moved out
  of the masthead into the bar as `.rio-action-link--muted` / `--danger`, killing the
  orphaned Cancel (form, user_edit, group_edit, user_new, group_new, password_change,
  lock_user, admin_reset_password). §9 **pills**: one canonical dot-less `.rio-pill` +
  `--on`/`--off` (contract values) and the semantic variants now live in
  `components/data.css`; the duplicate `.rio-pill` in `pages/tools.css` and
  `.rio-pill-stock` in `layout/console.css` are removed (list / users_list migrated to
  `--on`/`--off`). §2.1 **page header**: breadcrumb (15px, link segments strong/700,
  `·` separator) → 36px/800 title → 17px/1.7 lead, on the §10.4 rhythm, reconciled
  across both header systems (`.rio-crumbs` and `.rio-breadcrumbs`). §10.1 shells:
  `.rio-form-shell` 1100px→**880px**, new `.rio-content-shell` 1040px; the 2-col grid
  drops to a 24px gap and stacks at 768px. No new CSS fragments.
- **Visual Contract v2.0 conformance — Phase 7.5 (builtin render).** The builtin
  user-create and password-change forms emitted their paired fields with `span == 2`
  (full-width), so they stacked instead of landing in the §10.2 two-column grid.
  `render::create_user_form_sections` now emits **Email** and **Temporary password**
  (renamed from "Password" to match the reference) at `span: 1` so they pair under the
  IDENTITY legend (Role stays on its own row); `render::password_change_form_sections`
  and `render::must_change_password_form_sections` emit **New password** and **Confirm
  new password** (renamed from "Confirm") at `span: 1` (Old password keeps its own row).
  Behaviour change: two field labels change and the two-column layout engages. The
  public recovery flow (token reset / forgot-password) is unchanged.
- **Visual Contract v2.0 conformance — Phase 8 (dark theme re-derivation + RTL §12).**
  The dark blocks of `tokens/colors.css` (`@media (prefers-color-scheme: dark)` and
  `[data-theme="dark"]`) are re-derived from the new slate+teal light palette,
  **token-driven only — no per-component dark CSS**. Surfaces invert to a slate ladder
  (`--rio-bg` `#0f172a`, `--rio-surface` `#1e293b`, `--rio-raised` `#334155`,
  `--rio-sunken` `#0b1120`, `--rio-overlay` `#1e293b`); borders `--rio-line` `#334155`,
  `--rio-line-strong` `#64748b`; text `--rio-text-hi` `#f1f5f9`, `--rio-text` `#cbd5e1`
  (≥4.5:1), `--rio-text-mute` `#94a3b8`, `--rio-text-faint` `#64748b`. The accent lifts
  to teal-400 for AA on dark: `--rio-rust` `#2dd4bf` (hover `#5eead4`, active `#99f6e4`),
  `--rio-rust-tint`/`-tint-2`/`-ring` re-hued to `rgba(45,212,191,…)` (ring `.45`),
  `--rio-accent-focus` `#2dd4bf`; the filled button stays deep enough for white text
  (`--rio-rust-solid` `#0d9488`, hover `#0f766e`, active `#115e59`). New Phase-1–6 tokens
  gained dark values (previously light-only, would have leaked): `--rio-surface-tint`
  `#122e2c`, `--rio-pill-on-bg` `rgba(111,185,138,.16)` / `--rio-pill-on-text` `#86efac`
  / `--rio-pill-off-bg` `#334155` / `--rio-pill-off-text` `#cbd5e1`, `--rio-danger`
  `#ef4444` / `--rio-danger-hover` `#dc2626` / `--rio-danger-tint` `rgba(239,68,68,.16)`,
  and dark syntax tints. `--rio-shadow-card` / `--rio-shadow-inset` were relocated from
  `tokens/compat.css` to `tokens/shadows.css` so they carry heavier-alpha dark values.
  RTL: the `[dir="rtl"]`/`:lang(ar)` `letter-spacing: 0` neutralization now also covers
  `.rio-fieldset > legend` / `.rio-fieldset-legend` (table headers were already covered).
  `CLAUDE.md` updated — the framework ships light + dark, token-driven only (the
  "light-only" claim is removed). Light theme is unchanged.
- **`rio-theme` emits dark blocks (dual-block dark-aware `tokens.css`).** The theme
  engine's `emit` previously wrote only a stub `:root[data-theme="dark"]` block (just
  `--rio-brand-adaptive`), so a generated override appended after the framework bundle
  leaked light surfaces into dark mode. It now emits the **full dual dark structure**
  the framework uses — `:root[data-theme="dark"]` (higher specificity, tie-breaks the
  toggle) **and** `@media (prefers-color-scheme: dark) { :root { … } }` (auto/OS dark) —
  with identical values. New `DarkPolicy` (re-exported) + `emit_with`: **Auto** derives
  the dark slate ladder and lifts the brand accent for AA on dark; **LightOnly** pins
  dark to framework defaults (a neutral override, never light values); **Explicit**
  lifts an author-supplied dark accent. `emit` defaults to `Auto`. Golden fixtures
  (`tests/golden/dark_{auto,light_only,explicit}.css`) cover all three; the five
  existing goldens were re-blessed (their dark block grew). **Behaviour change in
  generated output** — any project regenerating its `tokens.css` gets dark blocks.
  Contract: `docs/design/TOKENS-EMIT-SPEC.md`. Version is workspace-governed, so a
  release bump is a workspace decision (flagged here, not taken).
- **Runtime warns on a light-only `RUSTIO_TOKENS_CSS` override.** At startup, when the
  appended override defines `:root` color tokens but carries no `[data-theme="dark"]`
  or `prefers-color-scheme` block, the framework logs a WARN naming the file and the
  consequence (light surfaces leak into dark). Detection only — the operator's CSS is
  served verbatim, never rewritten or wrapped.
- **Retired the unused baked font faces Spectral and Hanken Grotesk.** After the
  Visual Contract moved body + display to **Inter** and mono to the SFMono system
  stack, the prior pairing's faces were dead weight. Removed: their `@font-face`
  blocks (`base/fonts.css`), the six woff2 assets, the `include_bytes!`/`FONT_*`
  constants and their static routes in `src/admin/routes.rs`. **Behaviour change** —
  `/static/fonts/Spectral-*.woff2` and `/static/fonts/HankenGrotesk-Variable-*.woff2`
  now return **404**, and any downstream override styling against `"Spectral"` or
  `"Hanken Grotesk"` silently falls back to the next family in its stack. **JetBrains
  Mono is kept**: the framework mono stack is SFMono-system, but the shop example's
  brand override (`--rio-font-mono`) still selects it (audit-confirmed live). Shrinks
  the binary by **~144 KB** (debug). Arabic fallbacks (Noto Naskh, Tajawal) untouched.
- **Retired the unreferenced baked Geist + Geist Mono faces, and fixed font license
  attributions.** Geist/GeistMono were `include_bytes!`'d, routed, and licensed but had
  **no `@font-face`** anywhere (audit: zero references) — pure dead weight. Removed their
  woff2 assets, `FONT_GEIST*` constants, and static routes; `/static/fonts/Geist-*.woff2`
  now **404**. The font routes were refactored into a ctx-free `register_font_routes`
  helper with a unit test asserting shipped faces serve and retired ones 404. **License
  hygiene:** added the missing **JetBrains Mono** OFL attribution to `LICENSE.txt` (it
  ships and is rendered by the shop override but was never attributed); `LICENSE.txt` now
  maps 1:1 to the shipped woff2 (Inter, JetBrains Mono, Noto Naskh, Tajawal, and the
  i18n Noto Thai/Devanagari/JP/KR/SC). Binary **~194 KB** smaller (debug; woff2 + the
  closure-dedup from the route refactor).


## [0.28.0] — 2026-06-08

### Changed
- **`#[derive(RustioAdmin)]` now generates `impl Model` too.** The derive
  already walked every field to build `AdminModel`; it now emits the
  `orm::Model` impl (`TABLE`, `COLUMNS`, `INSERT_COLUMNS`, `id`,
  `from_row`, `insert_values`) from the same walk. Models are declared
  once — the hand-written ORM glue that had to be kept in sync with the
  struct by hand is gone. Two struct-level escape hatches cover what the
  field walk can't infer: `#[rustio(table = "…")]` when the SQL table
  name differs from the auto slug (e.g. `Address` → `addresses`), and
  `#[rustio(extra_columns = ["…"])]` for generated/virtual columns that
  aren't struct fields (e.g. a Postgres `tsvector`). Existing hand-written
  `impl Model` blocks must be removed (they now collide with the derive).
- **Runtime dependencies are re-exported from `rustio_admin`.** `chrono`,
  `rust_decimal`, `uuid`, and `sqlx` (plus the `Decimal` / `DateTime` /
  `Utc` / `NaiveDate` / `NaiveTime` / `Uuid` type aliases) are now
  re-exported. The derive's generated code routes through these paths, so
  a model crate can depend on `rustio-admin` alone; the scaffold's
  `Cargo.toml` drops its direct `chrono` / `uuid` / `rust_decimal` /
  `sqlx` dependencies. One resolved copy of each, no version skew.

### Internal
- Two drift-guard tests with no machine check before: one asserts the
  `admin.css` `@import` manifest matches the `ADMIN_CSS`
  `concat!(include_str!(…))` bundle in `routes.rs`; one asserts
  `rustio_admin_macros::HUMANISE_ACRONYMS` is byte-identical to
  `admin::render::HUMANISE_ACRONYMS`.


## [0.27.6] — 2026-06-05

### Fixed
- **CSV import no longer dead-ends on an unrecognised column.** A header with
  columns the model doesn't declare used to return a bare `400 Bad request`,
  even though the importer was already built to skip them. The handler now
  matches that design: unknown columns are skipped, reported on the result
  page, and turned into a ready-to-run `ALTER TABLE` migration plus the
  matching struct / `from_row` / `insert_values` snippet, with column types
  inferred from the column names. Removes the now-unused
  `ParseError::UnknownColumns` variant.

### Changed
- **Admin pages restyled.** The framework-docs index, the markdown doc viewer,
  the API surface, the account-sessions page, the audit log, and the
  CSV-import result page were redesigned with the shared card / badge / glyph
  component language (page heads, colored status/method badges, avatar cells,
  typeset documentation prose). Token-only; secondary accents read
  `var(--rio-accent2, var(--rio-accent))` so a project that defines an amber
  secondary picks it up while the framework default falls back to its accent.

### Documentation
- **`getting-started.md` rewritten example-first.** Opens with a "See it work
  now" clinic walkthrough (`rustio-admin new clinic` → a real
  Patient/Appointment admin with seeded rows, signed into in minutes, zero
  code), then a "build your own" path where each `startapp` is shown as
  *Creates → Generates → the 3 lines to wire → the live URL*. Reflects the
  current 0.27.x experience (grouped wizard menu, contextual next-steps,
  reserved-keyword quoting).
- **`modeladmin.md` gains a worked example.** One `Article` model and the four
  hooks most projects use (`list_display` / `list_filter` / `search_fields` /
  `ordering`), shown together with the list page they render and a line-by-line
  map of each hook to its effect — before the per-hook reference.
- **`cli.md`** `new` description corrected to match the current CLI help.

### Internal
- `examples/` reorganised into two admin projects: the canonical multi-crate
  reference renamed `clinic-admin` → `clinic`, the unused axum `clinic` demo
  dropped, and a new single-crate `shop` (e-commerce admin: nine related
  models, inline relations, validation, a navy + amber theme) added. Repo
  layout only — not part of the published crate.


## [0.27.5] — 2026-06-03

### Fixed
- **`ai` / `memory` printed the wrong binary name in their guidance.** The
  `ai` and `memory` subcommands told you to run `rustio ai apply …`,
  `rustio ai init`, `rustio memory …`, etc. — but the binary is
  **`rustio-admin`**. Following those instructions literally invoked the
  *sibling* `rustio` project instead, which fails with
  `rustio.schema.json not found`. Every printed command (and the related doc
  text) now uses `rustio-admin`, so copy-pasting the suggested next step runs
  the right tool. (The binary was renamed to `rustio-admin` in v0.22.0; these
  two modules' output strings were missed.)

### Changed
- **`--help` is now grouped, colour-coded, and beginner-first.** The bare
  `rustio-admin` / `--help` welcome was a flat wall of 21 commands; it now opens
  with a grouped, coloured map organised by what you want to *do* — **Getting
  started** (new → startapp → migrate → user → run), **When you need more** (ai,
  memory, builder), **Help** — with the `RustIO` wordmark in a distinct gold,
  group headings in accent, commands bold, descriptions dim, and breathing room
  between groups. The full command list still follows below (nothing hidden).
- **`startapp` now has the best help in the tool — by example, not prose.**
  `rustio-admin startapp --help` opens with a one-line summary, then runnable
  examples (SIMPLE, WITH FIELDS), the full field-type list, RELATIONS
  (`fk:Model`) and CHOICES (`choice:a,b`) shown as snippets, and a closing tip —
  so a beginner copies a working command instead of reading a paragraph.
- **First-model nudge after `new`.** A model-less (`minimal`/`custom`) project
  now ends its scaffold output with `Add your first model:` and a concrete
  `startapp` example, pointing at `startapp --help` for more — so the developer
  always knows the next move instead of stalling at an empty admin.


## [0.27.4] — 2026-06-03

### Changed
- **First-run terminal experience: colour, spacing, and a clear staged
  rhythm.** The `rustio-admin new` wizard and the scaffold summaries were flat,
  cramped, single-colour text; they now share one calm visual language
  (`crates/rustio-admin-cli/src/style.rs`): a single warm-amber accent for the
  *active* element (step titles, the `›` prompt, the "Next" heading), green
  `✓` confirmations, cyan paths/URLs, bold commands, dim explanations — with
  blank-line spacing between stages. The wizard now shows **Step _n_ of 3**
  headers; the post-create block leads with a `Next —` heading and lists each
  command on its own copy-clean line with a dim explanation beneath (so a
  copy-paste never drags prose into the shell). The next-steps now include the
  previously-missing **`createdb`** step on the wizard path. All colour
  degrades to identical plain text under `NO_COLOR` / non-TTY / CI. Doctrine:
  new `DESIGN_ONBOARDING.md` §13 (Visual language), superseding the old
  colourless-wizard stance.
- **Commands now point to the next step (contextual).** After
  `rustio-admin migrate apply`, the CLI shows the single next step for where
  you stand — *create your admin login* if no administrator exists yet, or
  *launch your app* (`cargo run` → the admin URL) if one does. After
  `rustio-admin user create`, it points to `cargo run`. So the first run reads
  as one continuous thread (scaffold → migrate → user → launch) instead of
  leaving you to remember what's next. Interactive terminals only — scripts and
  CI stay quiet (`DESIGN_ONBOARDING.md` §13.4).
- **Clearer `--help` command list.** The command summaries were dense
  multi-sentence paragraphs (and `plan` / `commit` leaked the internal
  "Doctrine B8" jargon into user-facing help). Each command now shows one
  concise, beginner-readable line in the list, with full detail still on
  `<command> --help`; advanced Builder verbs are marked `(advanced)`.

### Fixed
- **`startapp` generated invalid SQL for reserved-keyword field names.** A
  `--field order:fk:Order` produced an unquoted `order` column and `migrate
  apply` failed with `syntax error at or near "order"`. Generated column names
  are now quoted in the migration DDL (declaration + `choice` CHECK), matching
  the already-quoted runtime `COLUMNS`. *(Shipped via #32; recorded here for the
  release notes.)*

### Onboarding
- The `new` wizard no longer advertises `school` / `inventory` as "a familiar
  starting point" — only `clinic` and `blog` ship example models, so the menu
  now labels each type honestly ("example models …" vs "clean slate"). And
  `rustio-admin ai status` / `ai init` state up front that RustIO runs no AI
  itself. *(Shipped via #32.)*


## [0.27.3] — 2026-06-03

### Changed
- **Welcome banner now leads with value, not mechanism.** The `rustio-admin`
  welcome (bare command / `--help`) previously framed the differentiator in
  governance vocabulary — *authority: who may do what, how sessions end, …
  governed by checked-in contracts*. It now opens with what the developer
  **gets** ("describe your data as Rust structs, get a complete admin panel"),
  and reframes the "Why RustIO is different" block as the **outcome** they
  receive — the login, roles, and audit trail come built in, as one system, so
  the admin is secure and accountable on the first run with nothing to
  assemble — rather than the machinery that delivers it. Value before
  mechanics (`DESIGN_ONBOARDING.md` §10): a first-time reader meets the value
  before any contract or authority concept. Banner text only; no command,
  flag, or behaviour change, and "Start here" + the full command list are
  unchanged below the separator.


## [0.27.2] — 2026-06-02

### Added
- **`clinic` project type now scaffolds a real clinic.** Choosing *Clinic*
  in the `rustio-admin new` wizard (or `--preset clinic`) previously produced
  the neutral empty scaffold; it now ships a working clinic admin out of the
  box: a `Patient` model (name, date of birth, phone, email) and an
  `Appointment` model (foreign key to patient, scheduled time, reason, and a
  status with a CHECK constraint), their migrations **seeded with example
  rows**, and a `src/main.rs` that already registers both models. The
  developer reaches a non-empty, searchable clinic admin behind a login
  without hand-editing a single file — value before mechanics
  (`DESIGN_ONBOARDING.md` §6). The Next-steps block reflects the real
  migrations ("creates the patients + appointments tables").

### Fixed
- **Content presets now substitute `{{name_title}}` in `src/main.rs`.** The
  preset override pass replaced `{{name}}` and `{{type_phrase}}` but not
  `{{name_title}}`, so a `blog`-preset project's `Admin::app_name(...)` was
  left as the literal `{{name_title}}`. The new shared preset-layer path
  substitutes all three, fixing `blog` and covering `clinic`.

### Changed
- **First-contact welcome answers "why RustIO is different".** The
  `rustio-admin` welcome (shown on the bare command and `--help`) previously
  led with *what* RustIO is; it now opens with the "Django Admin, for Rust"
  mental model and a labelled **"Why RustIO is different"** block — authority
  (who may do what, how sessions end, how access is recovered, what gets
  recorded) is designed as one system, governed by checked-in contracts, not
  bolted on — followed by a two-step **Start here** path (`new` → `cargo run`).
  A first-time developer now grasps the differentiator in seconds, not after
  reading the docs. Honest framing only — no "AI platform" claim (the
  framework embeds no model; see README *Non-goals*). Banner text only; no
  command, flag, or behaviour change.


## [0.27.1] — 2026-06-02

### Changed
- **Onboarding wizard reads clarity-first.** The interactive `rustio-admin
  new` wizard now opens by saying *what it does* and *what you'll have at the
  end* before the first prompt, prefaces each question (name / type /
  database) with one calm line of *why it matters*, and confirms each
  answered step with a sober `✓` (`DESIGN_ONBOARDING.md` §9.1 — previously not
  emitted). Same three questions, same calm no-emoji tone and single divider;
  clearer framing so a first-time developer never faces a prompt whose purpose
  they have to guess. The PostgreSQL guidance now leads with *why* Postgres
  (one database, by design). Behaviour, flags, and validation are unchanged.
- **Welcome banner leads with what RustIO is.** The `--help` / no-args banner
  previously opened with "Welcome to RustIO Admin"; it now states up front
  that RustIO is a *Postgres-first admin framework for Rust* and what you get
  (an admin panel with auth, sessions, recovery, and audit built in) before
  the **Start here** block (`DESIGN_ONBOARDING.md` §10.2) and the full command
  list. Understanding before the command list; calm, no emoji, single
  divider. No command added, removed, or reordered.

### Documentation
- **Clarity-first onboarding & docs pass.** Reader-facing surfaces were
  reworked to lead with understanding before mechanics (progressive
  disclosure): the README now opens with "What is RustIO?"; `DESIGN_CLOUD.md`
  gained an "in one minute" preamble before its contract body; a new
  [`docs/memory.md`](docs/memory.md) usage guide covers `rustio memory`; and
  [`docs/getting-started.md`](docs/getting-started.md) was rewritten end to
  end — corrected to the current 0.27 scaffold (the binary is `rustio-admin`;
  the default `minimal` preset is neutral, no demo `Post`; models are added
  with `startapp --field …`) and taken deeper, from install to a running
  admin with your own model.


## [0.27.0] — 2026-06-02

### Added
- **`rustio memory` — project memory (CLOUD.md), read/derive slice.** First
  implementation of `docs/design/DESIGN_CLOUD.md` / `DESIGN_CLOUD_IMPL.md`:
  a per-entry store under `.rustio/memory/entries/<ulid>.md` (TOML
  frontmatter + Markdown prose; status **derived**, never stored) and a
  generated, human-readable `CLOUD.md` view. Three offline, read-only verbs:
  `render` (regenerate CLOUD.md — the only writer of that file), `show`
  (exact, mechanical filters: `--subject` / `--type` / `--active` / `--grep`
  — no relevance ranking), and `verify` (CLOUD.md freshness + referential
  integrity: dangling and cyclic `supersedes:` links are rejected, and a
  one-entry-two-successors merge fork is surfaced as an open tension). No
  new dependency — TOML frontmatter is parsed with the already-present
  `toml_edit` (the design's §3.1 was refined from YAML to TOML for exactly
  this reason). Dev-time tooling only — the running admin never reads
  CLOUD.md.
- **`rustio memory` — governed write path.** `remember` and `supersede`
  propose a memory entry; a human `approve`s; `apply` materialises the
  immutable entry file and re-renders CLOUD.md. It reuses the
  `DESIGN_AI_ASSISTANT.md` proposal/approval/log lifecycle **unchanged** —
  one governance pipeline, no parallel path: `write_memory` and
  `supersede_memory` are new `needs_approval` capabilities in the existing
  `.rustio/ai.toml`, and memory proposals live under `.rustio/memory/`. The
  entry is materialised **at apply**, stamping the apply `date`, the
  approver as `ratified_by`, and a fresh UUID v7 `correlation_id` (a memory
  entry's audit join). `--foundational` requires **two approvers**.
  Offline-first: identity via `--by`, the local `log.jsonl` is the record;
  DB-backed identity + audit mirroring is a later slice (as `rustio ai`
  shipped). Verbs: `remember` / `supersede` / `pending` / `approve` /
  `reject` / `apply`. Internally, the shared lifecycle was first extracted
  from `ai.rs` into a new `proposal` module (pure no-op refactor).
- **`rustio memory redact` — the bounded append-only exception.** Removes
  prohibited content (a leaked secret / PII / etc.) from an existing entry:
  the **one** operation that edits an entry in place, gated by **two
  approvers** (`redact_memory` is `needs_approval` + `second_approver_for`).
  Apply replaces the entry's body with a recorded marker
  (`[content removed: class=… · date · by=… · audit=…]`), sets a structural
  `redacted` flag, and re-renders CLOUD.md — the entry stays in the
  chronology, visibly demoted, never deleted. The `--class` taxonomy is
  aligned with the framework's existing redaction helpers (`password` /
  `token` / `mfa_secret` / `backup_code` / `pii` / `credential` /
  `operational`). **Honesty (DESIGN_CLOUD.md §3):** apply prints that
  redaction cleans the working tree only — a genuinely leaked secret
  additionally requires secret rotation and an out-of-band history rewrite
  (`git filter-repo` / BFG). Offline-first like the rest of the write path;
  the typed `memory_redacted` audit event ships with the DB-mirror slice.
- **`rustio memory` — analytics & cache verbs (read-only, mechanical).**
  `chain <id>` shows an entry's supersession lineage (what it supersedes,
  transitively, and what supersedes it); `stats` reports counts by type and
  status plus foundational / redacted counts and subject frequencies;
  `promote-candidates` flags active entries revised ≥ N times (a mechanical
  "consequential" signal) as ADR candidates — suggests only, promotion stays
  a human action (§10); `index` rebuilds the regenerable
  `.rustio/memory/index.json` mechanical cache (§2.5 — counts and links
  only, never a source of truth). All counts, never content interpretation.
- **`rustio memory` — DB-backed identity + audit mirror (`--as`).**
  `approve` / `reject` / `apply` gain `--as <email>` (mutually exclusive
  with `--by`): the email is authenticated against `rustio_users` (active,
  role ≥ the policy's `approver_role`, via the shared identity resolver `ai`
  uses) and the decision is mirrored into `rustio_admin_actions` — reusing
  the `ai_proposal_approved` / `_rejected` / `_applied` events for
  write/supersede, and a **new typed `memory_redacted`** `AuditEvent` for
  redaction (records the class removed, by whom, when — `DESIGN_CLOUD.md`
  §3). Identity resolution is shared with `rustio ai` (one pipeline); the
  audit phrasing and event are memory-specific. `--by` stays the offline
  default. Completes audit-by-default for project memory.
- **`rustio memory verify` — working-tree tamper backstop.** Beyond the
  freshness + referential-integrity checks, `verify` now flags tracked entry
  files changed outside the lifecycle: a **modified** entry without the
  `redacted` flag (a hand-edit of write-once reasoning), a **renamed** entry
  (the filename is the id), or a **deleted** entry (entries are never
  removed). A modification that *is* a redaction (carries `redacted = true`)
  is allowed. Best-effort and git-based: it compares the working tree against
  HEAD, skips cleanly when not in a git repo, and — honestly (§11) — does not
  forensically audit committed history and degrades under squash-merge /
  force-push. Intended to run in **CI** on changes touching `.rustio/memory/`
  or `CLOUD.md`; non-zero exit on any failure. This closes the
  implementation against `DESIGN_CLOUD_IMPL.md` (its §14 open items are all
  resolved or explicitly declined).


## [0.26.0] — 2026-06-02

### Added

- **Three new `startapp --field` scalar types: `float` / `date` / `time`.**
  The closed scaffold vocabulary grows from eight tokens to eleven.
  `name:float` → `f64` / `DOUBLE PRECISION NOT NULL` (`<input type="number">`);
  `name:date` → `chrono::NaiveDate` / `DATE NOT NULL` (`<input type="date">`);
  `name:time` → `chrono::NaiveTime` / `TIME NOT NULL` (`<input type="time">`).
  Each flows end-to-end: the CLI emits the struct field + migration + `from_row`
  getter (`get_f64` / `get_date` / `get_time`, new on `orm::Row`), the
  `#[derive(RustioAdmin)]` macro classifies the Rust type and generates form
  parsing + list display, and the admin UI renders the matching HTML widget.
  `orm::Value` gains `F64` / `Date` / `Time` variants; the runtime
  `FieldType` (non-exhaustive) gains `F64` / `Date` / `Time`. Generated
  models import only the chrono types they use, so a `date`-only model never
  pulls an unused `DateTime, Utc` import. Scaffolds with no field of a given
  type are byte-identical to before.
- **Two more `startapp --field` types: `decimal` / `uuid`.** Vocabulary
  grows to thirteen tokens. `name:decimal` → `rust_decimal::Decimal` /
  `NUMERIC NOT NULL` (`<input type="number">`); `name:uuid` →
  `uuid::Uuid` / `UUID NOT NULL` (`<input type="text">`). Both flow
  end-to-end like the PR-2 scalars. `decimal` and `uuid` are surfaced as
  **strings** in JSON / OpenAPI (`format: decimal` / `format: uuid`) —
  a JSON number can't hold arbitrary-precision `NUMERIC` without
  rounding. `uuid` migrations are bare `UUID NOT NULL` with no
  `DEFAULT gen_random_uuid()` (matching the "no implicit defaults"
  discipline — edit the migration if you want one). New dependency:
  `rust_decimal` (the framework already depended on `uuid`); the
  `rustio startproject` scaffold gains `uuid` + `rust_decimal` deps so
  generated models compile. Existing projects adding a `decimal`/`uuid`
  field must add the matching dependency to their own `Cargo.toml`.
- **`email` / `phone` field types — validated strings, not new storage
  types.** Vocabulary grows to fifteen tokens. Both are `String` /
  `TEXT NOT NULL` at rest (no new dependency, no new `Row` getter, no
  migration machinery); the scaffolder emits the field with a
  `#[rustio(format = "email" | "phone")]` attribute that the derive
  macro reads — the struct's `String` type alone can't distinguish
  them. `email` renders `<input type="email">` and runs
  `is_valid_email`; `phone` renders `<input type="tel">` and runs
  `is_valid_phone`. The checks are deliberately structural typo guards,
  not RFC 5322 / E.164 parsers. Two new public helpers
  `rustio_admin::admin::{is_valid_email, is_valid_phone}`; two new
  `FieldType` variants (`Email` / `Phone`). Both columns are
  text-searchable. JSON / OpenAPI surface them as strings (`email`
  gets `format: email`).
- **`choice` field type — closed value sets.** Vocabulary grows to
  sixteen tokens. New CLI grammar `name:choice:v1,v2,...` (e.g.
  `status:choice:active,inactive`); values are restricted to
  `[A-Za-z0-9_-]` so they embed safely without escaping. Stored as
  `String` / `TEXT NOT NULL` **with a `CHECK (col IN (...))`
  constraint** — the one scaffold token that emits a constraint, since
  the closed set *is* the type. The scaffolder emits
  `#[rustio(choices = [...])]`, which the derive macro reads to
  populate `AdminField.choices` (previously always `None`) — so the
  field renders as a `<select>` (reusing the existing choices
  machinery, no new `FieldType` variant) and `from_form` rejects
  values outside the set with a calm validation error in addition to
  the DB constraint. A Rust-`enum`-backed choice with compile-time
  type safety remains a deliberate non-goal for this crate (a future
  `rustio-pro` concern).


## [0.25.0] — 2026-05-31

### Changed

- **`Admin::model::<M>()` now warns when an FTS opt-in is silently
  ineffective.** `ModelAdmin::search_index_column()` is only honoured by
  the list query when the column is also in `Model::COLUMNS` (an
  injection-safety check); otherwise search quietly falls back to a slow
  `ILIKE` scan. The framework now logs a `warn!` at model registration
  naming the model and column, instead of leaving the misconfig silent.
  `docs/modeladmin.md` documents the COLUMNS requirement.
- **List pages no longer fetch the full-text-search column.** The FTS
  column must be in `Model::COLUMNS` to be usable in the `WHERE` clause,
  but it's a `tsvector` that's never displayed — the list `SELECT` now
  excludes it from the projection (instead of `SELECT …, search_vector`),
  so the wasteful blob isn't fetched per row.


## [0.24.0] — 2026-05-31

### Changed

- **`/admin/docs` no longer renders dead cross-document links.** The
  in-app docs viewer serves only the three embedded files
  (`architecture` / `modeladmin` / `public-api`) by slug, so relative
  cross-doc links in those pages (`design/…`, `archive/…`, sibling
  `*.md`) could never resolve and rendered as 404-on-click anchors.
  `admin::docs::render_markdown` now strips any non-navigable link to
  plain text (keeping its label); external URLs (`http`/`https`/
  `mailto`) and in-page `#anchor` links are untouched. The canonical
  `docs/` copies keep their links for GitHub browsing.


## [0.23.0] — 2026-05-30

### Added

- **AI assistant: DB-backed approver identity + audit mirroring (opt-in
  `--as <email>`).** `rustio ai approve` / `reject` / `apply` accept
  `--as <email>` (mutually exclusive with `--by`): the CLI reads
  `DATABASE_URL`, resolves the email to an **active** `rustio_users` row
  whose role meets the policy's `approver_role`, records that real user,
  and mirrors the decision into `rustio_admin_actions`. The local
  `.rustio/ai/log.jsonl` stays the working record; the audit row is a
  mirror, joinable by `metadata.proposal_id`. The offline `--by <name>`
  path is unchanged and needs no database. CLI trust model: `--as`
  asserts identity (trust boundary = shell + `DATABASE_URL`, as with the
  emergency-access CLI) and authorises it against the users table — no
  password challenge.
- **`rustio ai allow` / `deny` — edit the policy buckets.** `allow <cap>`
  moves a capability into `allowed` (`--needs-approval` into
  `needs_approval`); `deny <cap>` moves it into `blocked`. The edit is
  applied to `.rustio/ai.toml` in place via `toml_edit`, **preserving the
  template's comments and untouched buckets**, and the diff is printed
  before the write. A no-op is detected (byte-identical), moving a
  default-Blocked capability out of `blocked` prints a "this widens what
  the AI may do" note, and the verb requires the file to exist (run
  `rustio ai init` first) so the change is always an explicit, reviewable
  edit. With this, the offline `rustio ai` surface is complete except for
  DB-backed identity / `rustio_admin_actions` mirroring.
- **`rustio ai log` — the action record.** Renders
  `.rustio/ai/log.jsonl` newest-first (suggestions, approvals,
  rejections, applies, and blocked attempts), with `--proposal <id>` to
  scope to one proposal, `--limit N` (default 20), and `--all`. Reads the
  local log only — offline, no database.
- **AI assistant proposal model + approval lifecycle (`rustio ai
  propose` / `list` / `review` / `approve` / `reject` / `apply`).** Builds
  on the policy from the previous slice (DESIGN_AI_ASSISTANT.md §4–§5). A
  proposal is a change the AI wants to make; it moves `Suggested →
  Approved → Applied` (or `→ Rejected`). The bucket and the number of
  required approvals (0 for Allowed, 1 for Needs-approval, 2 when the
  policy lists the capability under `second_approver_for`) are
  snapshotted at creation, so a later policy edit can't retroactively
  change a pending proposal's rules. `propose` refuses a Blocked
  capability (and logs the attempt); `approve` enforces *distinct*
  approvers; `apply` writes the proposal's staged files only once the
  approval gate is satisfied (staged destinations are validated against
  path traversal). Proposals are JSON under `.rustio/ai/proposals/`; every
  state change appends to `.rustio/ai/log.jsonl`. `status` now shows
  pending proposals and recent actions. Still offline — no AI call, no
  database; the approver is a `--by <name>` string and mirroring into
  `rustio_admin_actions` is a later slice.
- **`rustio ai status` / `rustio ai init` — AI assistant permissions
  (first slice).** The read-only, offline start of
  `docs/design/DESIGN_AI_ASSISTANT.md`: a `.rustio/ai.toml` policy file
  that sorts every known capability into **Allowed**, **Needs approval**,
  or **Blocked**, and a `status` verb that prints it (`✓ / ⚠ / ✗`, with a
  "2 approvers" note where the policy requires dual sign-off). The
  capability set is a fixed catalogue; the file only moves capabilities
  between buckets, so `status` always shows the full surface even with a
  partial or absent file — an unknown key is warned and ignored, a
  malformed file is a hard error. `status` reads the policy only (no AI
  call, no database); `init` writes a default policy and refuses to
  clobber an existing one without `--force`. The approval lifecycle,
  proposal record, and the remaining `ai` verbs are later slices.

- **`RUSTIO_TOKENS_CSS` — apply a generated palette to a running admin
  without recompiling.** `rio-theme` could already generate a complete,
  contrast-safe `tokens.css` (`rustio-admin theme generate --brand
  '#<hex>'`), but a *consumer* had no way to use it: `/static/admin.css`
  served a fully-baked `concat!` bundle, so the file was inert unless you
  forked the framework. Set `RUSTIO_TOKENS_CSS=<path>` and the runtime now
  appends that file's bytes *after* the baked bundle, so its `:root` block
  wins the cascade (later `:root` wins, no `!important`) — exactly like the
  `AdminTheme` inline patch. Read once at router-build time (the route is
  `no-cache`, so a restart rolls out an edit); a set-but-unreadable path
  logs a warning and degrades to the baked bundle rather than failing boot.
  The runtime serves bytes only and **does not** link `rio-theme`, so the
  "runtime never depends on the theme engine" invariant holds. The
  `theme generate` report now prints the exact `RUSTIO_TOKENS_CSS=… cargo
  run` next step. New doctrine doc `docs/design/DESIGN_THEME.md` (the
  engine's §1–§12 contract — purpose, color model, contrast thresholds,
  the seven Cases, pipeline order, token vocabulary, consumer integration),
  finally resolving the in-code `DESIGN_THEME §N` citations.

### Audit

- **Three new `AuditEvent` variants** — `ai_proposal_approved`,
  `ai_proposal_rejected`, `ai_proposal_applied` — for AI-assistant
  decisions mirrored from the CLI's `rustio ai` verbs (see Added). Minor
  addition per `DESIGN_AUDIT.md` Appendix A; strings locked by
  `audit_event_existing_variants_have_stable_strings`. Rows are written
  via the public `audit::record` with `model_name = "users"`,
  `object_id` = the acting user, and a UUID-v7 `correlation_id`. No schema
  change.

### Changed

- **Startup is no longer dominated by idempotent re-work.** `auth::init_tables`
  and `Admin::seed_permissions` previously re-issued their full DDL /
  permission `INSERT`s on every boot (~46 ms + ~67 ms of Postgres
  round-trips in a typical project). Both now fast-path through a new
  `rustio_admin_meta` key/value table: `init_tables` stamps the crate
  version and skips when it matches; `seed_permissions` stamps a
  fingerprint of `(crate version + sorted model set)` and skips when it
  matches. A fresh database, a framework upgrade, or a changed model set
  all force a full re-run, so the self-healing-on-boot guarantee is
  preserved — only the redundant work is removed. Warm startup for a
  6-model project drops from ~138 ms to ~25–30 ms. `TRUNCATE
  rustio_admin_meta` forces a full re-run.

### Docs

- **README naming-disambiguation subsection** added near the bottom
  pointing at the sibling [`rustio`](https://github.com/abdulwahed-sweden/rustio)
  project — different scope (SQLite + AI-augmented schema pipeline
  + admin), same name prefix. The note records that v0.22.0's
  binary rename means `cargo install rustio-cli` and `cargo install
  rustio-admin-cli` no longer collide in `~/.cargo/bin`, so both
  binaries can coexist on the same machine.


## [0.22.0] — 2026-05-29

### Changed — CLI binary renamed `rustio` → `rustio-admin`

**Breaking for shell-level invocations.** The `rustio-admin-cli` crate
now produces a binary named `rustio-admin` instead of `rustio`. The
package name on crates.io is unchanged (`rustio-admin-cli`); only the
emitted executable changes. Update any shell scripts, CI steps,
Dockerfiles, or docs that invoke the binary directly:

```sh
# before
rustio startproject myapp
rustio migrate apply

# after
rustio-admin startproject myapp
rustio-admin migrate apply
```

Why: the `rustio` name collided at the OS level with an earlier
unrelated project also called RustIO that publishes a `rustio` binary.
Two `cargo install` invocations on the same machine would silently
overwrite each other with no diagnostic. Renaming the bin to match
the crate (`rustio-admin-cli` → `rustio-admin`) clears the ambiguity.

What was updated:

- `[[bin]] name` in `crates/rustio-admin-cli/Cargo.toml`.
- The clap `name = ...` and the `WELCOME_HELP` banner shown by
  `--help`.
- Every user-facing CLI verb example in README, ROADMAP, CLAUDE.md,
  active design docs, the `crates/rustio-admin-cli/templates/`
  scaffold (so newly-scaffolded projects' READMEs and `main.rs`
  comments reference `rustio-admin`), library doc comments, the
  admin-template empty-state CTAs (`/admin`, `/admin/health`,
  `/admin/db-browser`), `examples/.env`, and one CI guard comment.
- Doc comments in CLI source files and error / hint strings that the
  CLI prints back to the operator.

What was deliberately NOT changed:

- The `@generated by rustio <semver>` codegen sentinel in
  `builder/codegen.rs`. It is the on-disk contract with already-
  scaffolded user projects (the SchemaHash header in `_generated/`
  files); renaming it would diverge newly-generated files from
  previously-generated ones for any project that scaffolds across
  the version bump.
- The `.rustio/` state directory name, the `// rustio:` insertion-
  marker syntax in scaffolded `main.rs` files, the
  `rustio_admin_actions` audit table, and `RUSTIO_*` environment
  variables (`RUSTIO_TEMPLATE_DIR`, `RUSTIO_SECRET_KEY`). These are
  on-disk / runtime contracts; renaming them would silently break
  running deployments.
- Historical CHANGELOG entries below this one. They describe what
  shipped at the time and were truthful at release.

Migration: `cargo install rustio-admin-cli` after the next release
installs `~/.cargo/bin/rustio-admin`. The previous `~/.cargo/bin/
rustio` is left on disk by Cargo as-is — `cargo uninstall
rustio-admin-cli` from an earlier install will remove it, or remove
it manually if it's stale.


## [0.21.1] — 2026-05-26

Admin list-page redesign. Pure CSS + template + render-context
change; no Rust public-API impact, no schema, no auth or permission
behaviour shift.

### Toolbar split

The pre-0.21 "View ▾" overflow dropdown collapsed Sort, Per-page,
CSV export, and CSV import into one mega-menu — operators had to
scan a 17-row list (8 fields × 2 directions + Default + 4 per-page
+ Export + Import) to find any single operation. 0.21.1 splits the
toolbar into two zones with distinct intent:

- **Row 1 "Find"** — search input (flex:1, ≥360px), up to two
  promoted filter chips, a "More filters" overflow for the rest, a
  Reset link that surfaces only when there's state to reset
  (search OR ≥1 active filter).
- **Row 2 "Arrange"** — Sort field menu, direction toggle, Rows
  per page, Saved view, then a flex spacer, then Export / Import
  pinned to the trailing edge. Every control carries icon + label.

### Find panel surface

`.rlp-find` sits inside an accent-tinted card (`--rio-accent-soft`
fill, `--rio-accent-border`, framework radius). No heading or icon
above the panel — the surface itself is the cue. Arrange stays
transparent so the visual weight stays on Find.

### Dark table header

`.rio-list-page .rio-table th` re-skins to `--rio-surface-chrome`
(the same slate-900 the sidebar uses). The `thead` scope locally
redefines `--rio-text-strong` / `--rio-text` / `--rio-text-muted`
/ `--rio-text-subtle` to slate-50…500 and lifts `--rio-accent` to
teal-400 — same chrome-cascade trick `layout/shell.css` already
uses on the sidebar, so hover and active-sort copy stay legible
without per-selector overrides. The header bar reads as one
continuous frame with the sidebar; data rows now contrast hard
against the chrome instead of fading into a neutral.

### Sort menu — field-once + direction toggle

`SortFieldCtx` (one entry per sortable field) replaces the
field×direction menu pattern. Clicking a field row activates
ascending sort; a sibling direction toggle flips to descending and
reads field-type-aware copy:

| Field type           | Asc label       | Desc label      |
|----------------------|-----------------|-----------------|
| `DateTime`           | oldest first    | newest first    |
| `String` / `FilePath`| A → Z           | Z → A           |
| `I32` / `I64`        | lowest first    | highest first   |
| `Bool`               | off → on        | on → off        |

Numeric copy (`lowest first` / `highest first`) is new in 0.21.1;
0.21.0 fell back to the generic `ascending` / `descending` strip.
The unit test was updated to match the new copy.

### Empty state

The pre-0.21 `<p class="rio-empty">No <model> yet</p>` floated as a
single sentence inside a large blank card. 0.21.1 ships a centered
empty-state block: a tinted-disc `inbox` icon, an `h2` title
("No appointments yet"), a 16px body sentence ("Add the first
appointment to get started."), and the primary `Add` CTA. Copy
substitutes the model's plural / singular display name so the
same block reads correctly for every model with no project-side
override.

### Bottom bar

Row count + pagination move out of the toolbar into a footer-style
bar below the table card. `Showing N of M <model-plural>` plus
pagination nav; hidden in the empty state because zero-of-zero is
noise.

### Responsive

`.rio-list-page` page padding cascades: 32px desktop ≥1280px →
24px tablet 768–1279px → 16px mobile <768px, driven by a
`.rio-main:has(> .rio-list-page)` selector so non-list pages keep
their existing padding. The toolbar's `flex-wrap` lets Row 1 and
Row 2 reflow vertically below 768px without code; the table card
remains the only horizontal-scroll surface.

### Render-context additions

`ListCtx` gains:

- `sort_fields: Vec<SortFieldCtx>` — one entry per sortable field.
- `current_sort_field_label: String` — Sort chip caption.
- `current_sort_dir_label: &'static str` — direction toggle text.
- `sort_dir_toggle_link: String` — flips the active sort's direction.
- `default_sort_link: String` — Sort menu's reset row URL.

The legacy `sort_options` / `current_sort_label` fields are kept
for back-compat with project templates that haven't migrated to
the new layout.

### Assets

- New stylesheet `assets/static/admin/pages/list.css`, scoped
  under `.rio-list-page`. Wired into both the `admin.css` `@import`
  manifest and the `ADMIN_CSS` concat block in `routes.rs`;
  `tests/cascade_lockstep.rs` verifies the two lists stay aligned.
- New icons: `inbox` (empty-state disc), `rotate-ccw` (Reset chip),
  `arrow-up-down` (direction toggle decoration). All inline lucide
  stroke paths, no external library.

### Known issue (carried forward, not introduced by 0.21.1)

A foreign-key filter declared in `ModelAdmin::list_filter()` is
silently dropped when the `RelationRegistry` doesn't carry the
relation — `infer_filters_with_registry` falls through to
`FilterKind::NumericExact`, then the list handler hits a `_ => {}`
catch-all with no rendered widget. Pre-existing in 0.21.0; the
new toolbar makes the gap more visible (the "More filters"
dropdown is the natural place to surface secondary filters and
will obviously be empty when only the FK was configured). Fix
requires real widget design for `NumericExact` / `RelationSelect`
and belongs in a later phase, not a UI patch release.

### Migration

Existing 0.21.0 projects update by bumping
`rustio-admin = "0.21.0"` to `"0.21.1"` in their `Cargo.toml` and
running `cargo update -p rustio-admin`. No schema migration. No
template override required — the new layout ships as the embedded
default. Projects that override `templates/admin/list.html`
locally will continue to use their override; remove it to pick up
the new layout.


## [0.21.0] — 2026-05-25

Stage 2 of the FTUX redesign. Four PRs:

- **PR 2.1** — `rustio startapp <name> --field <name>:<type>` (closed
  8-token vocabulary; field-declared model + migration generated in
  one CLI breath; print-and-paste handshake preserved).
- **PR 2.2** — structural permission defaults (three groups
  seeded on fresh databases, lockstepped to `--role` values, never
  silently re-shaping existing projects).
- **Startapp pluralization + spatial-orientation fix** — naive `+s`
  replaced with the proper English rules; new `// rustio: modules
  / imports / models` markers in the scaffolded `main.rs`; structured
  `startapp` output orients the developer against the markers.
- **PR 2.4** — `rustio docs` evolves from static text to a useful
  command (probes the local server with an `x-correlation-id`
  specificity check; `--open` launches the default browser).

PR 2.3 (doctor onboarding mode + `.rustio/onboarding.json` state
machine) was deliberately cut per the Stage 1 Reality Audit — the
audit found doctor already reads clean and the state file would
introduce hidden framework state that solves a problem nobody has.

**No API breaks.** Existing 0.20.x projects update by bumping
`rustio-admin = "0.20.0"` to `"0.21.0"` in their `Cargo.toml` and
running `cargo update -p rustio-admin`. Schema unchanged (the
permission seed is INSERTs only). The framework's `Role` enum
unchanged. `Admin::app_name(...)` chain unchanged.

New doctrine doc: `docs/design/DESIGN_PERMISSIONS.md`.
New direct dep: `console = "0.15"` (already transitively in the
tree via `indicatif`).

See the sections below for per-PR detail.

### Added — onboarding PR 2.4 (docs discoverability)

Smallest of the originally-planned Stage 2 PRs. Three small
affordances on top of the existing `rustio docs` placeholder
from PR 1.1; no new architecture, no new ongoing surfaces.

- **`rustio docs` becomes a useful command** — probes the local
  `/admin/health` endpoint with a 500ms timeout and reflects the
  result in the printed output. Server reachable → "RustIO docs
  are running on this project" with the local URL prominent;
  server unreachable → "RustIO documentation" with a `(start
  the server with cargo run to enable)` hint and the online +
  repo URLs.
- **`rustio docs --open`** — new flag that launches the local
  `/admin/docs` in the default browser (`open` on macOS,
  `xdg-open` on Linux/BSD, `cmd /c start` on Windows). Exits 1
  if the server is unreachable or the browser opener fails so
  scripts piping `--open` can detect the failure. Default off
  — auto-opening a browser is the kind of magic
  `DESIGN_ONBOARDING.md` §4 forbids.
- **Health probe specificity** — the probe requires both a
  2xx/3xx status AND an `x-correlation-id` response header
  (set by the framework's `middleware::correlation_id` on
  every response, BEFORE any auth gate). This is the
  rustio-specific tell that distinguishes a real rustio server
  from a foreign HTTP service bound to port 8000. An audit-
  driven regression test (`health_probe_rejects_foreign_server_
  without_correlation_id`) locks the behaviour.
- **Raw `std::net::TcpStream` for the probe** — no HTTP client
  dep. ~50 lines vs. pulling in `ureq` / `hyper` for one
  endpoint. Failure-closed: any error / refused / hung
  connection returns "not running".
- **New module `crates/rustio-admin-cli/src/docs.rs`** — owns
  the probe + browser-opener + the `console::style` output.
  Six unit tests (probe behaviour with stub servers: 200 with
  correlation_id; 303 with correlation_id; 303 WITHOUT
  correlation_id; 500 with correlation_id; non-HTTP garbage;
  nothing listening).
- **Homepage gains a "sign in required" hint** — one line of
  muted text under the three nav pills (`Admin`, `Docs`,
  `Health`) in `templates/project/templates/home.html` to
  explain why clicking any of them redirects to the login
  page when not signed in. Static label (no login-state
  inference); uses the existing `--muted` CSS custom property.
- **Wizard Next Steps mentions `/admin/docs`** — the
  scaffolded `cargo run` comment now reads
  `# http://127.0.0.1:8000 (homepage) + /admin + /admin/docs`
  in both the wizard path and the non-interactive
  `startproject` path.
- **Generated README §4 (Common commands)** gets two new
  lines: `rustio docs` and `rustio docs --open`.

### Dependencies

- `console = "0.15"` (declared as a direct dep in PR 2.1 for
  startapp styling) is reused. No new dependency in this PR.

### Preserved

- Exit codes: 0 on success, 1 only on `--open` failure or
  browser-launcher failure.
- The existing `rustio docs` no-flags output structure (URLs
  with the same three labels) is preserved; the bytes change
  but scripts that grep for "RustIO" / "docs.rs/rustio-admin"
  still match.
- Non-TTY / CI / NO_COLOR cleanly strip ANSI via the
  `console::style` auto-detection.
- No framework crate changes.
- No new wizard prompt, no new state file, no new CLI verb.
- The framework's `/admin/docs` page itself is unchanged.

### Fixed — `rustio startapp` pluralization + spatial orientation

Two related fixes to the `rustio startapp` developer experience.
Strict constraints upheld: no AST rewriting, no auto-editing of
`src/main.rs`, no auto-registration, no hidden framework state.
The developer still makes the edits by hand -- the CLI only makes
*where* unambiguous.

- **Pluralization bug fix.** Replaced the naive `format!("{name}s")`
  with `app_fields::pluralise_snake()` (extracted as a top-level
  helper from the existing `pluralise_camel_to_snake`):
    - ends in `s` / `x` / `z` / `ch` / `sh` -> add `es`
    - ends in consonant + `y` -> drop `y`, add `ies`
    - ends in vowel + `y` -> keep `y`, add `s`
    - otherwise -> add `s`
  Drives both the generated `migrations/NNNN_create_<plural>.sql`
  filename AND the printed admin URL (`/admin/<plural>`).
  Audit-broken cases (`class -> classs`, `bus -> buss`, `category
  -> categorys`) now produce `classes`, `buses`, `categories`. No
  irregular-noun dictionary (`person` still pluralises to `persons`,
  not `people` -- predictable beats correct here; users rename if
  needed). 6 new unit tests cover every rule branch.
- **Insertion markers in the scaffolded `src/main.rs`.** The
  `templates/project/src/main.rs.tmpl` and
  `templates/project_blog/src/main.rs.tmpl` now contain three
  permanent marker comments at the correct locations:
    - `// rustio: modules` -- `mod <name>;` declarations go directly
      under this marker.
    - `// rustio: imports` -- `use <name>::<Name>;` lines go
      directly under this marker.
    - `// rustio: models` -- `.model::<YourModel>()` calls chain on
      the `Admin::new()` builder between this marker and the closing
      `;`.
  These markers are part of the scaffold contract and stay put as
  the project grows.
- **Spatial-orientation output from `rustio startapp`.** The
  previous prose-y "Edit src/main.rs and add these lines somewhere"
  was replaced with a structured layout that orients the developer
  against the markers. Output shape (NO_COLOR / non-TTY example):

  ```
  MODEL CREATED  Task  (2 fields)
    ✓ src/task.rs
    ✓ migrations/0001_create_tasks.sql

  3 edits in src/main.rs
    under  // rustio: modules  add  mod task;
    under  // rustio: imports   add  use task::Task;
    under  // rustio: models    add  .model::<Task>()

    $ rustio migrate apply
    $ cargo run
    admin  http://127.0.0.1:8000/admin/tasks
  ```

  Operational values (paths, shell commands, admin URL) visually
  stand out via `console::style()` -- paths cyan, commands green,
  URL cyan-underlined, markers dim, paste-this code bold. The
  `console` crate (already transitively present via `indicatif`)
  is now declared as a direct dep so the call site is reviewable.
  NO_COLOR / non-TTY environments strip ANSI sequences cleanly.
- **Missing-marker warning.** If any of the three markers is
  missing from `src/main.rs` when `startapp` runs (older project
  pre-this-fix, or a project that deleted them), the output
  prepends a `WARNING` block telling the developer to add the
  marker(s) manually before applying the edits. The edit
  instructions still print below. The CLI deliberately does NOT
  edit the file -- the developer stays the author.

### Compat note (intentional)

PR 2.1's "byte-identical to 0.20.0 on the no-`--field` path"
guarantee holds for words where naive `+s` happened to be correct
(`patient`, `student`, `task`, `book_review`, etc.). It is
intentionally broken for words where naive `+s` produced invalid
output (`class`, `bus`, `category`, etc.) -- those were generating
unusable table names + admin routes in 0.20.0; this fix corrects
them.

### Preserved

- No AST rewriting, no auto-editing of `src/main.rs`, no
  auto-registration, no hidden framework state.
- Exit codes unchanged.
- Existing 0.20.0 projects' generated models continue to compile.
- Field-type vocabulary, `--field` parser, `--no-interactive`
  flag, interactive prompt loop -- all unchanged from PR 2.1.
- `rustio startproject` / `rustio new` / `rustio doctor` / every
  other verb -- untouched.

### Added — onboarding PR 2.2 (Structural permission defaults)

Most security-critical PR in Stage 2 per the original execution
plan. A fresh database now ships with three named permission
groups every project will end up needing — `administrator`,
`editor`, `viewer` — sized conservatively, lockstepped to the
CLI's `--role` values, and **never silently re-shaping** projects
that have built their own group structure.

- **`auth::seed_default_groups(db)`** — new function called at the
  end of `auth::init_tables` on every boot. Creates the three
  groups idempotently (`create_group` uses `ON CONFLICT (name)`).
  Guarded: skipped entirely when `rustio_groups` contains any
  name NOT in the default set, so existing 0.20.x projects with
  custom groups pick up zero changes on upgrade.
- **`auth::grant_model_to_default_groups(db, app, singular)`** —
  new function called by `Admin::seed_permissions` after each
  model registers its four CRUD permissions. Applies the grant
  matrix (administrator gets all four; editor gets add/change/view;
  viewer gets view only). Missing groups (because the seed was
  skipped) are silent no-ops.
- **`auth::DEFAULT_GROUP_NAMES: [&str; 3]`** — exported constant,
  the single source of truth for the three names.
- **`CliRole::Editor` and `CliRole::Viewer`** — two new
  `--role` values on `rustio user create`. Both map internally to
  `Role::User` (the lowest tier — their authority comes entirely
  from group membership, not from a tier-level bypass). The
  `Administrator` / `Developer` permission-check bypass is
  unchanged.
- **`CliRole::default_group_name()`** — returns `Some("administrator")`
  / `Some("editor")` / `Some("viewer")` for the three new-style
  roles and `None` for the legacy five.
- **Group assignment on user-create** — `rustio user create --role
  <r>` now adds the new user to the matching seeded group when
  `<r>` is `administrator` / `editor` / `viewer`. Output gains an
  `Added to group: <name>` line. Legacy roles (`user`, `staff`,
  `supervisor`, `developer`) keep their existing behaviour (no
  group assignment).
- **`lockstep_default_groups_match_cli_role_names` test** — CI
  guard that asserts `DEFAULT_GROUP_NAMES` and the set of
  `CliRole` variants returning `Some` from `default_group_name()`
  are byte-equal. Renaming either side without the other fails CI.
- **`docs/design/DESIGN_PERMISSIONS.md`** — new doctrine doc
  (eight sections, mirrors the shape of `DESIGN_AUDIT.md` /
  `DESIGN_SESSIONS.md`). Defines the three groups, the grant
  matrix, the seeding conditions, the lockstep contract, the
  `Role` enum vs. seeded-groups distinction, and the seven
  non-negotiable rules.
- **Scaffold's `README.md.tmpl` §3** updated to mention the three
  default groups and link to the doctrine doc.

### Grant matrix (the contract — see DESIGN_PERMISSIONS.md §3)

|              | `add` | `change` | `delete` | `view` |
|--------------|-------|----------|----------|--------|
| `administrator` | ✓     | ✓        | ✓        | ✓      |
| `editor`        | ✓     | ✓        |          | ✓      |
| `viewer`        |       |          |          | ✓      |

`editor` deliberately lacks `delete` — destructive operations
belong to administrators by default. Projects that want
editor-level delete grant `<app>.delete_<model>` to the `editor`
group explicitly via the admin permission-matrix page.

### Preserved

- The framework's `Role` enum (`User / Staff / Supervisor /
  Administrator / Developer`) is unchanged. No variants added,
  none removed. Library API surface unchanged.
- `Role::Administrator` and `Role::Developer` keep their
  permission-check bypass behaviour.
- `rustio user create --role <r>` for the legacy five values
  (`user / staff / supervisor / administrator / developer`) works
  exactly as before. `--role administrator` now additionally adds
  the user to the `administrator` group; the framework bypass
  means functional access is unchanged.
- Schema unchanged. The seed is INSERTs only — no DDL.
- Existing 0.20.x projects with user-defined groups pick up zero
  changes on upgrade.
- Migration engine semantics unchanged.
- Exit codes unchanged.
- No new dependencies.

### Doctrine

- PR 2.2 implements `DESIGN_PERMISSIONS.md` and
  `DESIGN_ONBOARDING.md` §7 (Demo data vs structural defaults).
  Conservative defaults, never broad. Lockstep enforced by CI.
  No silent re-shaping of existing projects.

### Added — onboarding PR 2.1 (`rustio startapp` redesign)

- **`--field <name>:<type>` flag** on `rustio startapp`, repeatable.
  Closed vocabulary: `str / text / int / bigint / bool / timestamp /
  json / fk:<Model>` (eight tokens, nothing more — see the §3
  / §7.1 "no DSL creep" boundary in the PR 2.1 contract). The
  scaffold renders a full model file with the declared fields,
  the matching CREATE TABLE migration, and a `ModelAdmin` impl
  with `list_display` / `search_fields` defaulted from the
  declaration. The "paste these lines into src/main.rs" handshake
  from prior releases is preserved unchanged — `startapp` still
  does not auto-edit the user's `main.rs`.
- **`--no-interactive`** flag on `rustio startapp`, mirroring the
  flag on `rustio new`. Bypasses the field-prompt loop when set.
- **Interactive field prompt** when stdin and stdout are both TTYs,
  `CI` is unset, and `--no-interactive` was not passed. Prompts
  one field per line in the `name:type` format; empty line ends
  the loop; invalid input prints the four-part PR 1.3 error and
  re-prompts without advancing the slot counter. A `Summary`
  block previews the collected fields before any file write.
- **New `app_fields` module** (parser + validator + renderer,
  stdlib only, 26 unit tests). Owns the closed type vocabulary
  in one place; the renderer produces the per-placeholder
  substrings the two new templates consume.
- **Two new templates** alongside the existing ones:
  `templates/app/model_with_fields.rs.tmpl` and
  `templates/app/migration_with_fields.sql.tmpl`. The historical
  templates remain unchanged and are still used on the no-`--field`
  path (byte-identical compat with 0.20.0).
- **`OnboardingError` now derives `Debug`** so `app_fields` tests
  can use `.unwrap_err()`.

### Field-type / SQL contract

| Token         | Rust type           | SQL column                          |
|---------------|---------------------|-------------------------------------|
| `str`         | `String`            | `TEXT NOT NULL`                     |
| `text`        | `String`            | `TEXT NOT NULL`                     |
| `int`         | `i32`               | `INTEGER NOT NULL`                  |
| `bigint`      | `i64`               | `BIGINT NOT NULL`                   |
| `bool`        | `bool`              | `BOOLEAN NOT NULL DEFAULT FALSE`    |
| `timestamp`   | `DateTime<Utc>`     | `TIMESTAMPTZ NOT NULL`              |
| `json`        | `serde_json::Value` | `JSONB NOT NULL`                    |
| `fk:<Model>`  | `i64`               | `BIGINT NOT NULL REFERENCES <models>(id)` |

Per the PR 2.1 design adjustment, `timestamp` does NOT default to
`NOW()` and `json` does NOT default to `'{}'::jsonb` — either
would imply created_at / settings semantics that don't fit every
use. Users add defaults by editing the generated migration.

### Preserved

- `rustio startapp <name>` with no `--field` flags in non-TTY / CI /
  `--no-interactive` mode produces the **byte-identical** model
  file + migration that 0.20.0 ships.
- Exit codes unchanged.
- The "paste these lines into src/main.rs" handshake stays.
- `rustio new`, `rustio doctor`, `rustio migrate`, `rustio user`,
  `rustio group`, `rustio perm`, `rustio audit`, `rustio theme`,
  `rustio override`, `rustio reload`, `rustio test-init`, Builder
  verbs: untouched.
- No new dependencies; stdlib only.
- No framework crate changes.
- The verb stays a scaffold helper — not a schema language, not a
  second ORM DSL, not a hidden codegen system.

### Doctrine

- PR 2.1 implements the contract in the PR 2.1 spec
  (`DESIGN_ONBOARDING.md` §6 — Default scaffold identity). The
  forbidden-scope list (§3) is enforced by code review: no
  `--update`, no nullability / unique / index flags, no
  auto-editing of `src/main.rs`, no FK on-delete syntax, no
  default-value flags.


## [0.20.0] — 2026-05-25

The Stage 1 first-time-developer-experience overhaul, plus the
Polish & Trust runtime trust repairs that surfaced during the
Stage 1 Reality Audit. Five onboarding PRs (1.1 → 1.5) +
Stage 0 doctrine + terminal-compatibility cleanup + the runtime
trust fixes, all shipped together so external users see a coherent
new experience the moment they `cargo install rustio-admin-cli`.

No API breaks. Existing 0.19.0 projects update by bumping
`rustio-admin = "0.19.0"` to `"0.20.0"` in their `Cargo.toml`
and running `cargo update -p rustio-admin`. Schema unchanged.

See the sections below for the per-PR detail.

### Fixed — Polish & Trust PR (Stage 1 Reality Audit follow-up)

A fresh external-project audit (clinic management, outside the
monorepo) found the polished CLI surface contrasted unfavourably
with framework-side noise once `cargo run` succeeded. Eight
surgical fixes — no redesign, no new systems.

- **sqlx NOTICE chatter silenced.** `Db::connect_with` now runs
  `SET client_min_messages = warning` on every pooled connection.
  The framework's `auth::init_tables` and migration runner emit
  60+ `relation "X" already exists, skipping` lines into the
  operator's log on every boot after the first. The trust impact
  was severe — those lines look like errors at first glance and
  buried the real "listening on" signal. Single-line behaviour:
  60+ log lines → 0; boot output drops from 65 lines to 2.
  WARNING + ERROR + LOG bands are unaffected.
- **Duplicate `listening on` log removed.** The framework's
  `Server::run` no longer logs its own `rustio listening on …`
  line. Scaffolded projects already log a canonical URL from
  their own `main.rs`. The duplicate (with conflicting paths —
  `/` from the framework vs `/admin` from the project) read as
  framework self-contradiction.
- **`-----` reduced to `--` in CLI strings.** The BiDi
  compatibility fix replaced em-dashes with five hyphens; five
  reads as ascii-art garbage mid-prose ("metadata for now -----
  does not change generated files yet"). Two hyphens is the
  conventional ASCII em-dash. Mechanical sed across CLI src;
  emergency banner re-aligned by adjusting trailing-space padding
  to keep the box at column 64.
- **Project name flows into admin chrome.** Scaffolded `main.rs`
  now calls `Admin::new().app_name("{{name_title}}")`. The
  login page tab title, login h1, and dashboard tab title now
  carry the project name (e.g. `Polishclinic`) instead of the
  generic `Admin`. New `humanise_name` helper in `scaffold.rs`
  capitalises and de-snakes / de-kebabs the cargo crate name
  (`my-clinic` → `My Clinic`, `school_admin` → `School Admin`).
  `{{name_title}}` template substitution alongside `{{name}}` /
  `{{type_phrase}}`.
- **CSS custom properties on the homepage.** `templates/home.html`
  gains a `:root { --brand: …; --text: …; --muted: …; }` block
  at the top of its inline CSS, with a one-line comment pointing
  at it as the customisation seam. The accent checkmark, subtitle,
  and section eyebrows now reference the tokens. Re-skinning the
  page is a three-edit affair instead of a 16-hex grep.
- **"metadata for now" disclaimer dropped from the wizard.** The
  project-type prompt now reads `"Project type:"` instead of
  `"Project type (metadata for now -- does not change generated
  files yet):"`. The original wording explicitly told users
  their choice was meaningless, which contradicted the
  homepage's `{{type_phrase}}` substitution that uses it.
- **README.md.tmpl restructured.** Sections renamed to match the
  new wizard reality: `1. First run` (drops the `cp .env.example`
  and `cargo install` steps the wizard already did) → `2. First
  model` → `3. Admin usage` → `4. Common commands` → `5. Advanced
  topics` (MFA, R4, custom routes, template overrides folded
  here). Top-of-file wording acknowledges the wizard wrote `.env`.
- **CLI/admin handshake on the empty state.** The dashboard's
  "No models registered yet" copy now leads with the CLI verb
  (`rustio startapp <name>`) before the Rust line, closing the
  loop between the friendly CLI that brought the user to the
  admin and the admin's hint about what to do next.

### Verified end-to-end

Fresh external clinic project at `/tmp/polish-clinic/polishclinic`,
local-path-patched `rustio-admin` dep so the framework fixes apply:

| Metric | Before | After |
|---|---|---|
| Boot log lines | 65 | 2 |
| `sqlx::postgres::notice` lines | 62 | 0 |
| `listening on` log lines | 2 (conflicting) | 1 |
| Admin tab title | `Admin` | `Polishclinic` |
| Login h1 | `Admin` | `Polishclinic` |
| `-----` in CLI strings | many | 0 (replaced) |
| README contradicts the wizard | yes | no |
| Homepage hex-literal customisation | 16-hex grep | 3 tokens |

### Preserved

- Exit codes, security headers, CSRF flow, audit chain, session
  lifecycle, MFA paths — all unchanged.
- Migration engine unchanged.
- Admin redesign untouched. Dashboard h1 still reads "Site
  administration" (not in the audit's allowed-changes list).
- No new dependencies, no new CLI verbs, no new abstractions,
  no onboarding state systems.

### Notes

- Most framework-side fixes (sqlx silence, duplicate-log removal,
  empty-state copy) require a new `rustio-admin` release on
  crates.io to reach users of `cargo install rustio-admin-cli`.
  The Polish & Trust PR completes the local code; the next
  release cycle ships it.

### Added — onboarding PR 1.5 (First-Boot Homepage & Project Identity)

The scaffold finally stops feeling like a renamed blog. A new project
now opens with a calm homepage at `/`, a project-type-aware subtitle,
and a neutral source tree — `Post`, the `posts` migration, and the
blog wording all moved into the `blog` preset where they belong.

- **Default scaffold is neutral** — `templates/project/src/post.rs.tmpl`
  and `templates/project/migrations/0001_create_posts.sql` moved to
  `templates/project_blog/`. The `minimal` preset now ships:
  `Cargo.toml`, `.env.example`, `.gitignore`, `README.md`, `src/main.rs`,
  `templates/home.html`, `migrations/.gitkeep`. No domain model, no
  blog assumptions, no Post reference anywhere in main.rs.
- **First-boot homepage at `/`** — `templates/project/templates/home.html`
  is the calm landing page: project name, project-type subtitle,
  three-item status list, three-line Next Steps code block, three
  navigation pills (Admin / Docs / Health), credit footer. Inline
  CSS only; no JS, no framework, no external assets. Baked into the
  user's binary via `include_str!("../templates/home.html")` so the
  project stays single-binary.
- **Project-type-aware wording** — wizard's `project_type` choice
  now drives a new `{{type_phrase}}` template substitution:
    - `custom`    → "Custom RustIO project initialized."
    - `clinic`    → "Clinic project initialized."
    - `school`    → "School management project initialized."
    - `inventory` → "Inventory project initialized."
    - `blog`      → "Blog project initialized."
  Unknown / non-interactive defaults fall through to the `custom`
  phrasing — neutral, never wrong. The phrase appears on the
  homepage subtitle; no domain models are generated.
- **Wizard maps `project_type == "blog"` to `--preset blog`**
  automatically — the only project type that produces blog-shaped
  source. Every other type uses the neutral minimal scaffold.
- **Blog preset gains `Post` ownership** — `BLOG_EXTRAS` now lists
  `src/post.rs`, `src/comment.rs`, both `0001_create_posts.sql` and
  `0002_create_comments.sql`. `BLOG_OVERRIDES` still rewrites
  `src/main.rs` to register Post + Comment. Blog scaffold behaviour
  is unchanged from the user's perspective; the file ownership
  reflects the doctrine.
- **`README.md.tmpl` debluged** — dropped the Post-specific "what
  to do with `0001_create_posts.sql`" block, the `src/post.rs`
  row in the project-layout table, and the `rustio startapp
  comment` example. Replaced with neutral wording plus a mention
  of the new homepage and `templates/home.html` override path.
  Verb references updated `startproject` → `new` (PR 1.1's
  friendly alias).
- **Generated `main.rs` serves the homepage** — both minimal and
  blog `main.rs.tmpl` end with:
  ```rust
  .get("/", |_req| async { Ok(Response::html(HOMEPAGE_HTML)) })
  ```
  with `HOMEPAGE_HTML` baked from `templates/home.html` via
  `include_str!`. Replaceable: edit the file and re-run; remove
  the route to free `/` for a project-defined handler.
- **`scaffold` API** — `pub fn project_with_db` gains a
  `project_type: &str` parameter (wizard threads its choice
  through). `pub fn project` is unchanged externally and defaults
  internally to `"custom"`.

### Verified end-to-end

- `rustio startproject neutral` → 7 files, no Post, homepage
  reads "Custom RustIO project initialized."
- Wizard with school → 8 files, homepage reads "School management
  project initialized.", main.rs is neutral.
- Wizard with blog → 12 files (Post + Comment + 2 migrations +
  .env), homepage reads "Blog project initialized.", main.rs
  registers Post and Comment.
- `rustio new wp --no-interactive --preset blog` → script path
  produces 11 files, blog preset unchanged.
- Generated minimal project passes `cargo check` cleanly.

### Doctrine

- PR 1.5 implements `DESIGN_ONBOARDING.md` §6 (Default scaffold
  identity) and §11 (Homepage doctrine). The page intentionally
  carries no analytics, no fake health checks, no charts, no JS;
  only project name, type phrase, static status list, Next Steps
  code block, navigation pills, footer. Easy to replace — the
  user owns `templates/home.html` from day one.

### Fixed — terminal compatibility (mixed RTL/LTR rendering)

PR 1.4 introduced terminal rendering problems on systems using
Arabic locales and other mixed RTL/LTR font fallback configurations.
Root cause: the Braille spinner block (U+2800–U+28FF) and U+2014
em-dashes can both trigger font fallback to RTL-aware fonts, which
in turn invokes the BiDi algorithm and corrupts column alignment.
Not a framework bug; a terminal compatibility issue.

Conservative cleanup of CLI-emitted strings:

- **Braille spinner replaced** with the ASCII frames `| / - \`
  (one location: `cli::progress::Step::start`). All other spinner
  semantics — `Step` API, NO_COLOR / CI / non-TTY / `--quiet` /
  `--no-progress` bypass — are unchanged.
- **Em-dash `—` (U+2014) replaced** with five ASCII hyphens (`-----`)
  in every CLI-source string across `crates/rustio-admin-cli/src/`.
  Doc comments, framework templates, and admin HTML are untouched
  (those surfaces have proper Unicode rendering).
- **Emergency banner alignment preserved** — the boxed banner in
  `emergency_ui::render` had its trailing-space padding reduced by
  four columns to keep the right `│` aligned after the em-dash
  expanded to five ASCII chars.
- **Kept Unicode that is widely safe** per spec: `✓`, `✗`, `…`,
  `→`. Box-drawing characters (`─`, `│`, `┌` …) stayed too — not
  in the user-listed change scope; widely supported.

### Preserved

- Exit codes unchanged.
- NO_COLOR / CI / non-TTY / `--quiet` / `--no-progress` bypass
  behaviour unchanged.
- Stdout/stderr separation unchanged (spinner on stderr, result on
  stdout).
- Per-file migration `✓` list intact.
- No output formatting redesign — only character substitution.

### Added — onboarding PR 1.4 (Progress & Feedback Layer)

- **`cli::progress` module** — single-spinner-style status feedback
  for long-running onboarding operations
  (`DESIGN_ONBOARDING.md` §9). One struct (`Step`) with four
  methods (`start` / `done_with` / `failed_with` / `clear`); no
  global TUI state. The spinner ticks on stderr; the `✓ <summary>`
  result line always goes to stdout; the `✗ <summary>` failure line
  always goes to stderr — animated or not, so scripts that grep
  stdout for `✓` see the same bytes in both modes.
- **`indicatif` (0.17) added** as a direct dependency of
  `rustio-admin-cli`. Battle-tested, used by `cargo` itself; its
  own non-TTY draw-target detection is the first gate, with our
  `--quiet` / `--no-progress` / `CI` / `NO_COLOR` check layered on
  top. No TUI framework, no `console` re-export beyond what
  `indicatif` already pulls in transitively.
- **Global `--quiet` and `--no-progress` flags** on `Cli`, both
  marked `global = true` so every subcommand accepts them. Either
  flag forces the progress layer to skip animation; the underlying
  command behaviour is unchanged. Errors still print.
- **Spinner placements** (the four call sites authorised by §9):
    - **`main::db()`** — silent-on-success spinner during
      `Db::connect`. Other commands (`user list`, `group list`, …)
      produce their own output, so no extra `✓ Connected` line is
      printed; the spinner exists only so a slow connect does not
      freeze the terminal.
    - **`migrate::apply`** — silent-on-success spinner around
      `migrations::apply_with`. The existing per-file
      `✓ <migration>` summary block is unchanged.
    - **`doctor`** — explicit `✓ Connected to PostgreSQL` /
      `✗ Could not connect: …` line replaces the previous bare
      println for the connect step; the spinner ticks during the
      round-trip. The remaining doctor checks are unchanged
      (they're fast SQL queries; PR 1.4 deliberately does NOT
      wrap every check — that would be a doctor redesign).
- **First-build expectation note** — the wizard's Next Steps block
  ends with one calm line: *"Note: the first `cargo run` may take
  several minutes — that is normal for a fresh Rust project."* Only
  on the wizard path; the non-interactive scaffold path (script
  users) does not get the note.

### Bypass behaviour (strictly enforced)

The spinner is suppressed entirely under any of:

| Signal | Source |
|---|---|
| `--quiet` | top-level Cli flag |
| `--no-progress` | top-level Cli flag |
| `CI=1` (or any value) | environment |
| `NO_COLOR=1` (or any value) | environment |
| stderr is not a terminal | runtime check via `IsTerminal` |

In all bypass cases the spinner emits zero output; `done_with` /
`failed_with` fall through to a plain `println!` / `eprintln!`
so result lines still print. Verified against piped stdout
(0 ANSI sequences leak), `--quiet`, `--no-progress`, `CI=1`, and
`NO_COLOR=1`.

### Preserved

- Exit codes unchanged.
- Existing command output stays on stdout; spinner is stderr-only.
- Per-file migration `✓` list in `migrate apply` survives untouched
  in both animated and non-animated modes.
- Migration engine, doctor architecture, runtime, and database
  layer are all unmodified — PR 1.4 only adds feedback wrapping
  at four call sites.

### Doctrine

- PR 1.4 implements `DESIGN_ONBOARDING.md` §9 (Motion and feedback
  doctrine): one spinner, one ✓ marker, one ✗ marker; respect
  `NO_COLOR` / non-TTY / `--quiet` / `--no-progress`. Feedback,
  not theatre.

### Added — onboarding PR 1.3 (Humanised Error Doctrine)

- **Four-part onboarding error shape** (`DESIGN_ONBOARDING.md` §8) —
  every rewritten onboarding-facing error now prints as:

  ```
  Problem:  <one line, plain English>
  Why:      <most likely cause>
  Fix:      <exact command or file edit>
  Retry:    <exact retry command>

  Details:
    <verbatim backend error, preserved for senior engineers>
  ```

  Implemented via a new internal helper `cli::ui::OnboardingError`.
  The main exit trampoline detects the four-part shape by its
  `"Problem:"` prefix and skips the redundant `"error: "` label.
- **First wave of rewrites** — the five onboarding-critical errors
  from `DESIGN_ONBOARDING.md` §8 now flow through the helper:
    1. `DATABASE_URL` missing — points the beginner at the wizard
       (PR 1.2) or shows the exact `.env` line to add.
    2. PostgreSQL service unreachable — recognises the framework's
       `Db::connect` wrap chain (`"db connect failed: pool timed out"`)
       and the raw `"connection refused"` shape; Fix line gives
       OS-specific start commands for macOS / Ubuntu / Windows.
    3. Database does not exist — pulls the bare name out of
       `database "X" does not exist`; Fix shows the exact `createdb`
       command using that name.
    4. Migration SQL failure — extracts the migration stem from the
       library's `"migration <stem> failed: <sqlx-error>"` wrap (even
       through the framework's `"500 Internal: "` and the CLI's
       `"apply: "` outer prefixes); Fix points at the matching
       `_<stem>.sql` file and reminds the user migrations are
       append-only.
    5. Invalid `--role` (and any other clap `ValueEnum` mismatch) —
       `Cli::parse()` switched to `try_parse()` so we can intercept
       `ErrorKind::InvalidValue`, read the bad value and the valid
       set from clap's `ContextValue`, and reformat. Role-flag
       errors get role-specific phrasing; other `ValueEnum` errors
       get a generic-but-still-four-part shape.

### Changed

- `main::db()` failures flow through `ui::database_url_missing()` and
  `ui::classify_db_connect_error()` instead of the previous
  `"DATABASE_URL is not set"` / `"could not connect: …"` one-liners.
- `migrate::apply` wraps the library error through
  `ui::classify_migration_error()` instead of the previous
  `"apply: …"` pass-through.
- Main switched from `Cli::parse()` to `Cli::try_parse()` to enable
  intercepting clap's `InvalidValue` errors. All other clap errors
  (missing required arg, `--help`, `--version`, malformed syntax)
  still flow through `clap::Error::exit()` unchanged.

### Preserved

- Exit codes are unchanged (0 success, 1 runtime/validation, 2 clap).
- Plain-`String` errors elsewhere in the CLI are untouched — the
  helper is opt-in per call site.
- Verbatim backend errors are never thrown away; they appear in the
  `Details:` block beneath the four-part guidance. Senior engineers
  who want the raw sqlx text still get it on the same screen.

### Doctrine

- PR 1.3 implements `DESIGN_ONBOARDING.md` §8 (Error-message
  doctrine) and §12 rule 10 ("Do not show raw backend errors to
  beginners without explanation"). The PR explicitly does NOT
  redesign the migration engine, the database layer, permissions,
  the runtime, doctor, or startapp — only the CLI messaging that
  fronts them.

### Added — onboarding PR 1.2 (Interactive Project Wizard)

- **`rustio new <name>` wizard** — when stdin and stdout are both TTYs
  (and `CI` is unset, and `--no-interactive` is not passed), `rustio
  new` runs a calm stdlib-only wizard collecting:
    1. **Project name** — pre-filled with the positional argument when
       provided; validated against the same rules as `startproject`.
    2. **Project type** — curated list (`custom` / `clinic` / `school`
       / `inventory` / `blog`), `custom` is default. Stored as intent
       only; PR 1.5 will branch starter content on it
       (DESIGN_ONBOARDING.md §6).
    3. **Postgres guidance** — text-only install hint for macOS /
       Ubuntu / Windows (DESIGN_ONBOARDING.md §4 — the CLI never
       shells out to package managers).
    4. **Database name** — defaults to `<project>_dev`; validated as a
       PostgreSQL unquoted identifier (≤63 bytes; reserved
       `postgres` / `template0` / `template1` rejected).
  Ends with a one-block summary of the user's choices.
- **Auto-generated `.env`** — the wizard writes `.env` next to the
  scaffold's `.env.example`, with `DATABASE_URL` composed from the
  chosen `DB_NAME` and the standard local defaults. Removes the
  manual `cp .env.example .env` step from the wizard's Next Steps.
  The scaffold's `.gitignore` already excludes `.env`; no edit
  needed.
- **`--no-interactive` flag** — bypasses the wizard explicitly. With
  this flag, `rustio new <name>` is byte-identical to `rustio
  startproject <name>` (verified by tree diff).

### Changed

- `Command::New::name` is now `Option<String>` so the wizard can
  prompt for it. Non-interactive callers (TTY-less stdin/stdout, CI,
  or `--no-interactive`) still get a clear "project name is required"
  error when omitted.
- `scaffold::project` refactored internally — file writing extracted
  to `write_project_files` so the wizard can reuse it before layering
  `.env`. Public `scaffold::project` API and output are unchanged.

### Doctrine

- PR 1.2 implements `DESIGN_ONBOARDING.md` §5 (target flow) and §10
  (command surface). The wizard explicitly does NOT touch homepage
  (PR 1.5), project-typed starter content (PR 1.5), starter security
  defaults (PR 2.2), or error-message humanisation (PR 1.3).

### Added — onboarding PR 1.1 (Beginner Command Surface)

- **`rustio new <name>`** — friendly alias for `rustio startproject
  <name>`. Identical behaviour, identical generated files, identical
  exit codes. Restores the universal `<tool> new <name>` expectation
  for first-time users.
- **`rustio builder new <name>`** — canonical home for the
  declarative-Builder bootstrap previously at the top-level `rustio
  new` slot. Behaviour and generated tree unchanged.
- **`rustio docs`** — tiny Phase-1 placeholder that prints the online,
  repository, and in-project documentation locations plus the running
  admin's `/admin/docs` URL. The browser opener ships with PR 2.4.
- **Welcome banner on `rustio --help`** — promotes `rustio new
  <project-name>` and the helpful `rustio doctor` / `rustio docs`
  verbs above clap's auto-generated command list. The full command
  list is preserved unchanged below the separator (presentation, not
  amputation; see `docs/design/DESIGN_ONBOARDING.md` §10).

### Changed

- The doc comment at the top of `crates/rustio-admin-cli/src/main.rs`
  now mirrors the Phase 1 / full-surface split codified in
  `DESIGN_ONBOARDING.md`.

### Deprecated

- **`rustio new <name>`** as the Builder bootstrap is reachable for
  one release as the hidden `rustio new --builder <name>` shim, which
  prints a calm one-line notice pointing at `rustio builder new`. The
  shim is hidden from `--help`; existing scripts get a soft landing.

### Doctrine

- `docs/design/DESIGN_ONBOARDING.md` (Stage 0 of the FTUX redesign)
  governs which verbs the welcome banner promotes and the
  presentation-not-amputation rule that all PRs in this track must
  respect.


## [0.19.0] — 2026-05-24

A six-turn visual overhaul of the entire admin UI under the "Quiet
Expert" design language. Calm, dense, professional — Stripe Dashboard
/ Linear / Vercel discipline applied to every page. **No backend
change**: every Rust handler, route, model, SQLx query, and migration
is identical to 0.18.x. Diff is CSS / templates / JS / one new icon.

### Added — design primitives

- **`.rio-section`** (cards.css) — page-level content rhythm primitive
  with `__header` / `__heading` / `__label` (uppercase eyebrow) /
  `__title` (H2-sized) / `__lead` / `__meta` / `__link` parts. Every
  major page region adopts it for consistent vertical pacing.
- **`.rio-empty-state`** — proper empty state with icon badge,
  headline, lead copy, and optional CTA. Replaces the pre-0.19
  one-paragraph `.rio-empty` text across every list, form-inline,
  detail-tab, dashboard, and admin-tool template.
- **`.rio-confirm`** (cards.css) — destructive / mutating-action
  confirmation block with `__header` (icon badge + title + lead),
  `__warnings` cluster, `__items` cascade list, `__small` caveat,
  `__section-title` eyebrow. `--danger` and `--neutral` variants
  (left-edge stripe in the variant color, icon-badge tint in the
  variant family). Every `confirm_delete` / `bulk_confirm_*` /
  `lock_user` / `mfa_disable` / `password_change` success template
  uses it.
- **`.rio-env-chip`** — small env/version chip for page-header
  metadata (Development pill, vX.Y.Z monospace badge). Extracted
  from the bespoke `.rio-dashboard-greeting` selectors so other
  pages can reuse it.
- **`.rio-page-actions__group`** — button cluster for multiple
  secondary affordances next to a primary CTA on the page header.
  Used by `apis_index` (OpenAPI / SDK / Playground) and
  `csv_import_result` ("Back" button).

### Changed — unified vocabulary

- **Page header**: every list / detail / form / confirm / settings /
  tool page now uses one `.rio-page-header` shape (breadcrumb · title
  · optional primary action · optional `__lead` deck). Pre-0.19 each
  template had bespoke header HTML; cross-page consistency was the
  biggest "feels random" complaint.
- **Stat tiles**: pre-0.19 cycled through four pastel background
  fills (`--rio-info-bg` / `--rio-success-bg` / `--rio-warning-bg` /
  `--rio-accent`-tint) which read as festive. v0.19 uses one
  consistent white surface with a 3-px top accent rail; color
  is reserved for semantic meaning, not per-tile decoration.
  Variants kept (`.rio-stat--info` etc.) for back-compat — they
  now only repaint the top rail.
- **Cards**: lighter shadow ambient (`--rio-shadow-xs` instead of
  `--rio-shadow`); opt-in `.rio-card--quiet` (no shadow, just border,
  for inline cards inside sections) and `.rio-card--elevated` (heavier
  shadow, for floating panels).
- **Recent-activity sparkline**: bars → smooth area chart with
  gradient fill, polyline line, baseline rule, per-day dots
  (Vercel / Grafana / Stripe analytics convention).
  `vector-effect: non-scaling-stroke` keeps the line crisp.
- **Dashboard greeting**: pre-0.19 emitted "Hello, Admin 👋" + "Site
  administration" + "Manage X from one console." 3-line Slack-style
  block. v0.19: just the page title + project name as a subtitle.

### Changed — pages

- **Dashboard** (`index.html`): unified header, stat tiles use new
  treatment, model grid wrapped in `.rio-section`, activity feed
  and tools split also use `.rio-section`, empty states get proper
  `.rio-empty-state` shapes.
- **List pages** (`list.html`, `users_list`, `groups_list`,
  `log_entries`, `notifications`): unified header w/ lead, counted
  section title, table cards switch to `.rio-card--quiet`, every
  empty state upgraded.
- **Form + confirm** (`form.html`, `user_new/edit/view`,
  `group_new/edit`, `confirm_delete`, `user_confirm_delete`,
  `group_confirm_delete`, `lock_user`, `bulk_confirm_delete`,
  `bulk_confirm_action`, `confirm_admin_action`): page-header leads,
  inline-form empty states, every confirm template adopts the
  `.rio-confirm` block with danger / neutral variants.
- **Settings + auth** (`account_sessions`, `password_change`,
  `mfa_enroll`, `mfa_disable`, `mfa_regenerate`,
  `mfa_enroll_complete`, `mfa_regenerate_complete`): MFA settings
  pages **migrated from the auth-style `.rio-login` centered shell
  to the in-chrome settings pattern** — they're in-chrome pages
  (the user is signed in), not pre-auth flows. Sidebar shows
  correctly now. Login / forgot / reset / reauth / mfa_verify /
  must_change unchanged (correct as-is — those ARE pre-auth flows).
- **Admin tools** (`docs_index`, `apis_index`, `feature_flags`,
  `db_browser`, `object_history`, `csv_import_result`): unified
  header, leads where missing, secondary-button clusters moved into
  `.rio-page-actions__group`, every empty state upgraded,
  `.rio-section` wraps where appropriate.

### Earlier in-progress work folded into this release

- v0.18.6 polish pass (committed under `a875ec4`): topbar search
  trigger goes wide, list-toolbar 2-row reflow, per-row kebab
  dropdown, dashboard greeting trimmed, smooth area-chart
  sparkline.
- v0.18.6 bug-fix bundle (committed under `2a43d59` as drop-in
  pre-tested files): `--rio-z-*` ladder, kebab JS via `<details>`
  + JS upgrade to escape table overflow, shared `_row_actions.html`
  partial, focus-visible polish, post-seed demo rows.

### Migration

- Projects that override `--rio-*` tokens in their own stylesheet:
  no change. Overrides still cascade.
- Templates that use the dropped `.rio-dashboard-greeting*` or
  `.rio-dashboard-section-*` selectors need to switch to the
  unified `.rio-page-header` / `.rio-section`. None of the
  framework's own templates still emit them.
- `.rio-empty` is kept in the cascade for back-compat but the
  framework's own templates all use `.rio-empty-state` now.


## [0.18.6] — 2026-05-23

A second redesign pass on top of 0.18.5 after a side-by-side
screenshot review against Linear / Stripe Dashboard / Vercel /
Notion. 0.18.5's account-dropdown + view-overflow + sparkline-rail
shipped the right structural moves; 0.18.6 takes them further to
match the discipline a daily-driven admin tool needs.

### Changed

- **Topbar search trigger goes wide.** Was a 300-px pill squeezed
  between brand and bell; now `flex: 1` with a 640-px cap so it
  fills the topbar's slack and reads as the primary chrome
  action. Brand left, search center (flex-grow), bell + account
  dropdown right — the canonical Linear / Stripe / Vercel layout.
- **List toolbar reflows to a 2-row stack.** Pre-0.18.6 layout put
  search + 3 dropdowns in one row (4 controls competing for
  attention). New layout: Row 1 = full-width search bar
  (`.rio-toolbar` is now `flex-direction: column`); Row 2 = a
  new `.rio-toolbar__controls` flex row holding Filters / Saved /
  View. Search is unambiguously the primary affordance; the
  rest visually subordinate without losing any feature.
- **Per-row table actions collapse to a kebab dropdown.** Each
  row's `Actions` column used to render Edit + Delete buttons
  inline (~140-px column at default zoom). Now a single `⋯`
  toggle (`.rio-row-actions`) opens a small dropdown menu with
  Edit + Delete. The column shrinks to ~32 px so row data
  dominates. Delete is marked `.rio-dropdown-item--danger` —
  a new variant that stays muted at rest and picks up
  `--rio-danger` color + `--rio-danger-bg` tint on hover.
- **Dashboard greeting trimmed.** Pre-0.18.6 emitted three
  lines: "Hello, Admin 👋" + "Site administration" + "Manage X
  from one console.". Enterprise admin chrome is read every day —
  emoji + filler copy wore thin fast. Now: the page title +
  the project name as a subtitle. The "who am I?" answer lives
  in the chrome's account dropdown; the dashboard owns the data.
- **Recent-activity sparkline replaced with an area chart.**
  0.18.5 fixed the bar chart's empty-day rendering, but bars on
  a 7-day axis are inherently low-fidelity for low-density data.
  New: smooth polyline + gradient-filled area underneath +
  a circle dot at each measurement + a horizontal baseline rule.
  Matches Vercel deploys / Grafana time-series / Stripe analytics
  charts. SVG uses `vector-effect: non-scaling-stroke` so the
  line stays a crisp 2-px regardless of the horizontal stretch.

### Added

- **Two new inline-SVG icons** in
  `crates/rustio-admin/src/admin/icons.rs`:
  - `more-horizontal` — kebab glyph for per-row dropdown
    triggers (three dots in a horizontal line).
  - (`sliders` and `upload` already added in 0.18.5.)
- **`.rio-dropdown-item--danger` menu-row variant** in
  `assets/static/admin/components/dropdowns.css`. Muted at
  rest; picks up `--rio-danger` + `--rio-danger-bg` on hover.
  Used by the per-row kebab's Delete entry.
- **`.rio-toolbar__controls` wrapper class** for the new
  two-row toolbar's secondary-control row.

### Removed (CSS only — visual rule retired)

- `.rio-dashboard-greeting__hello` and its `strong` selector
  in `assets/static/admin/pages/dashboard.css`. The emoji
  greeting element is gone; the rules had no remaining users.
- `.rio-sparkline__bar` and `.rio-sparkline__track` are kept
  in the cascade unchanged (any third-party template that
  still emits them keeps working) — the framework's index.html
  no longer renders either element.

### Notes

- All preserved features: search query roundtrip, hidden-input
  filter state, CSRF on POST forms, per-model permissions,
  Saved-filters per-operator bookmarks, CSV import/export,
  bulk actions, MFA / Sessions / password-change routing.
- No public API change.
- Same `colors.css` engine-generated palette as 0.18.4 — the
  redesign is pure layout + component refinement.


## [0.18.5] — 2026-05-23

### Changed

- **Topbar account area folded into a single dropdown.** The
  pre-0.18.5 inline strip rendered five separate links / buttons
  next to the bell (`Signed in as <email>`, MFA, Sessions,
  Change password, Log out) — visually cramped at desktop
  widths and overflowing onto a second row at narrow viewports.
  Replaced with one `.rio-topbar-account` dropdown anchored on
  a circular monogram avatar + the user email; menu rows host
  the four account actions, with the Log out button promoted to
  a footer-spanning danger-ghost affordance. Notification bell
  stays on the chrome as a status indicator.
- **List-page toolbar collapsed from 8 controls to 4.** Per
  user feedback ("the search bar is crowded"). The new
  ordering is **Search · Filters · Saved · View**:
  - Search input now flex-grows to fill the row; the redundant
    "Search" button is hidden via the new
    `.rio-visually-hidden` utility (Enter on the input submits
    in every modern browser, and a no-JS submit fallback
    remains in the accessibility tree).
  - Sort, Per-page, CSV-export, and CSV-import folded into a
    new "View" overflow dropdown (`.rio-toolbar-view`) with a
    `sliders` icon. Each former toolbar control becomes a
    section inside the dropdown panel: Sort menu, Per-page chip
    row, Data section containing Export CSV link + Import CSV
    file-picker label. No feature lost — same URLs, same CSRF,
    same per-model permissions; only the affordance moved.
- **Dashboard sparkline shows empty days as a baseline rail.**
  Pre-0.18.5: days with zero count rendered no bar; a freshly-
  seeded admin's chart looked like one tall bar in empty
  whitespace (one active day, six invisible days). Now every
  day gets a 2-px baseline track; nonzero bars stack on the
  rail with a 4-px minimum height so a count of 1 is still
  visible next to a count of 50.

### Added

- **Scaffold template ships 5 demo `Post` rows** spread across
  the last week. A blank `posts` table on first boot made the
  dashboard sparkline look broken, the model tile read `0 rows`,
  and the Posts list view felt like an error state. The demo
  rows are explicit, easy to delete, and date-spread so the
  sparkline renders multiple bars out of the box.
- Three icon catalogue entries (`crates/rustio-admin/src/admin/icons.rs`):
  - `sliders` — list-toolbar "View" dropdown trigger.
  - `upload` — mirror of `download`, for the Import CSV row.
  - (`download` reused for Export CSV inside the View menu.)
- **`.rio-visually-hidden` utility class** in
  `assets/static/admin/base/utilities.css`. Standard
  Bootstrap/Tailwind `.sr-only` pattern — used by the search
  form's hidden submit button.

### Fixed

- The cluttered topbar overflowed onto a second row at narrow
  viewports, breaking the sticky-header expectation. Account
  dropdown brings the chrome back to a single row at every
  width.


## [0.18.4] — 2026-05-23

### Fixed

- **`admin::docs` `include_str!` paths reached outside the crate
  root.** Lines like `include_str!("../../../../docs/<file>.md")`
  worked in-tree but `cargo publish` builds the crate from a
  tarball that contains only the crate's own files, so the
  paths failed verification. Pre-existing since v0.16.0 — never
  surfaced because no version >= 0.14.0 had been published.
- Fix: copied `architecture.md`, `modeladmin.md`, and
  `public-api.md` into `crates/rustio-admin/assets/docs/` and
  updated the three `include_str!` lines to read
  `../../assets/docs/<file>.md` (paths now stay inside the
  crate). Workspace `docs/` remains the canonical source for the
  human-facing site / IDE access; the in-crate copies are
  release-bundled.

### Published

- First publish since v0.13.0. All four workspace crates ship to
  crates.io at 0.18.4:
  - `rustio-admin-macros`
  - `rio-theme` (first claim of the name; new build-time crate
    that ships the theme engine added in v0.17.0)
  - `rustio-admin`
  - `rustio-admin-cli`


## [0.18.3] — 2026-05-23

### Fixed

- **`cargo fmt --check` clean on the whole workspace.** Pre-existing
  rustfmt drift in `crates/rio-theme/` (introduced with the crate in
  0.17.0 and never run through fmt locally) plus the CLI's
  `theme.rs`. Mechanical reformat, no behavior change. Was silently
  failing CI's first step across the entire 0.17.x and 0.18.x line.
- **`clippy::uninlined-format-args` violation in
  `crates/rio-theme/src/color.rs`.** `format!("#{:02x}{:02x}{:02x}",
  r, g, b)` → `format!("#{r:02x}{g:02x}{b:02x}")`. CI pins
  `rustc 1.88`, whose clippy enforces this as a hard error under
  `-D warnings`; newer local toolchains lenient. Same lint class
  that bit `bcdf9b6` last quarter — flagging for the release
  checklist.

### Verified

- All 15 CI steps green on
  [run 26337135995](https://github.com/abdulwahed-sweden/rustio-admin/actions/runs/26337135995):
  fmt, clippy, build, test, Tier-2 symbol guard, four Builder
  Doctrine guards, and the new scaffold-template-pin guard.
- The template-pin guard's FAIL path verified end-to-end on
  [run 26337205216](https://github.com/abdulwahed-sweden/rustio-admin/actions/runs/26337205216)
  via throwaway PR #2 (closed without merge): every other step
  green, only the guard step failed with the expected actionable
  message.


## [0.18.2] — 2026-05-23

### Added

- **CI guard: scaffold template pin tracks workspace minor.**
  New step in `.github/workflows/ci.yml` extracts the workspace
  `[workspace.package].version` from the root `Cargo.toml` and the
  `rustio-admin = "X"` pin from
  `crates/rustio-admin-cli/templates/project/Cargo.toml.tmpl`,
  compares them, and fails the build with an actionable
  `::error::` line on mismatch. Closes the gap that allowed
  0.17.0 → 0.17.1 → 0.18.0 to ship with a stale template pin
  (patched in v0.18.1).
- Tested locally by temporarily reverting the template pin to
  0.16.0; the guard emitted
  `::error::Workspace version (0.18.1) and scaffold template pin
  (0.16.0) have drifted.` and would have exited 1.
- In-sync state logs `OK: workspace and scaffold template both pin
  <version>` so the check is visible in green CI runs too.


## [0.18.1] — 2026-05-23

### Fixed

- **Scaffold template pin tracks the workspace minor.** The
  `rustio startproject` template (`crates/rustio-admin-cli/templates/project/Cargo.toml.tmpl`)
  pinned `rustio-admin = "0.16.0"` through three workspace releases
  (0.17.0, 0.17.1, 0.18.0) that bumped the framework but missed the
  template — and `0.16.0` was never published to crates.io anyway
  (latest published: 0.13.0), so any fresh scaffold hit a registry
  resolve failure. The template now reads `"0.18.1"`. Per the
  CLAUDE.md convention (the 0.2.1 lesson: bundled template pin
  matches released framework).
- Surfaced during local verification of the v0.18.0 engine-generated
  `colors.css` render; recommendation: fold the template pin into
  the release checklist or add a CI grep guard so the workspace
  version and the template pin can't drift.


## [0.18.0] — 2026-05-23

The framework's live `crates/rustio-admin/assets/static/admin/tokens/colors.css`
is now **generated by `rio-theme`** from the brand color `#0d9488`. The
engine becomes the source of truth for the color palette; future palette
changes flow through `rustio theme generate` rather than hand-editing
the token file. Operators who want a different brand run one command:

```sh
rustio theme generate --brand "#<hex>" --out crates/rustio-admin/assets/static/admin/tokens/colors.css
```

### Added

- Eleven new canonical `--rio-brand-*` tokens shipped in the live
  file: `--rio-brand-light`, `-dark`, `-adaptive`, `-surface`,
  `-accent`, `-secondary`, `-hover`, `-active`, `-tint`, `-text`,
  plus `--rio-muted`. These name the engine's decision-layer
  outputs; existing template consumers stay on the drop-in
  `--rio-*` aliases.

### Changed

- **Brand-derived tokens** now come from the engine, not hand-picked
  Tailwind ladder rungs:
  - `--rio-accent-hover`: `#0F766E` → `#188278`
  - `--rio-accent-soft`: `#F0FDFA` → `#ebf4f3`
  - `--rio-accent-border`: `#99F6E4` → `#b8d9d4` (calmer mid-tint)
  - `--rio-info-bg`: `#F0FDFA` → `#ebf4f3`
- **Page canvas + border gain a faint brand tint** (engine's
  derivation rule — Case 2):
  - `--rio-bg`: `#F2F4F7` (neutral slate-200) → `#f9fcfb` (faint teal)
  - `--rio-border`: `#CBD5E1` (slate-300) → `#e2f0ee` (teal-tinted)
- **`--rio-success` rotates from emerald to lime**: `#059669` →
  `#48a030`. Case 4 fires because teal brand (`#0d9488`) hue sits
  inside the success anchor's collision zone (gap > MIN_HUE_GAP/2 and
  < MIN_HUE_GAP). The engine's contract is that brand and states
  stay perceptually distinct; the prior emerald deliberately rhymed
  with the brand. The new lime is still in the green family and
  clears AA against both white and the new `--rio-success-bg`.
- **Semantic backgrounds re-derived** through the contrast-aware
  `soft_bg` loop (introduced in 0.17.1):
  - `--rio-success-bg`: `#ECFDF5` → `#f1f8ef`
  - `--rio-warning-bg`: `#FFFBEB` → `#fef7f2`
  - `--rio-danger-bg`: `#FEF2F2` → `#fff0ee`

### Unchanged

- **Slate scaffold** stays brand-agnostic by design — `--rio-surface*`,
  `--rio-text-strong/text/muted/subtle`, `--rio-border-soft/strong`,
  `--rio-surface-chrome`, `--rio-warning`, `--rio-danger`. A brand
  swap will not move the operator chrome.
- **WCAG**: all 18 realistic text-on-surface pairings still clear AA
  (full parity with the prior hand-tuned palette).

### Removed

- The ~100-line design-rationale comment block in the previous
  `colors.css` (v0.16.0 clinical-SaaS overhaul notes). The
  rationale is preserved in git history and in
  `docs/design/DESIGN_DOCTRINE.md`; per-case logic lives in
  `crates/rio-theme/src/`.

### Migration

- For projects that override `--rio-*` tokens in their own
  stylesheet: no change. Overrides still cascade.
- For projects that depended on specific token *values* (e.g.
  expecting `--rio-success` to be exactly `#059669`): regenerate
  with their preferred brand, or hand-edit `colors.css` —
  remember the engine will overwrite hand edits if rerun.


## [0.17.1] — 2026-05-23

### Fixed

- **`rio-theme::emit::soft_bg` was contrast-blind.** The flat 92%
  white mix used to derive `--rio-success-bg` / `--rio-warning-bg`
  / `--rio-danger-bg` from their respective foregrounds left
  amber warning at 2.94 contrast — below `AA_NON_TEXT` (3.0).
  Verification against the live framework's WCAG pairings
  surfaced the regression. Replaced with a contrast-aware loop
  that walks lighter in 1% increments (bounded at 99%) until the
  foreground clears 3.0 against the resulting background.
- Engine output is now at full WCAG parity with the live
  `colors.css` on every realistic text-on-surface pairing the
  admin templates use (text/strong/muted/subtle on surface/bg;
  brand affordances on bg; chrome text on slate-900; semantic
  foregrounds on derived backgrounds).

### Added

- New property test `soft_bg_always_clears_aa_non_text_against_its_foreground`:
  resolves 7 brand inputs (default, calm teal, neon lime, dark
  navy, terracotta, grey, red) and asserts every semantic
  foreground clears AA against its derived background. Guards
  against regressing the fix.


## [0.17.0] — 2026-05-23

### Added

- **`rio-theme` — theme engine crate.** New build-time crate that
  takes a client's raw brand colors and emits a safe, computed
  `tokens.css`. The pipeline is a pure function over six decision
  cases:
  - **Case 1** — contrast guard. Every emitted text-on-surface
    pairing is checked against WCAG AA. Failing pairings are
    substituted with a guaranteed-readable fallback; the rejected
    color is logged, not silently dropped.
  - **Case 2** — palette derive. Hover, active, tint, text, bg,
    border, and brand-tinted muted are computed from the tamed
    surface brand. Semantics (success/warning/danger) are *not*
    derived from brand.
  - **Case 3** — vivid taming. Brands with OKLCH chroma above
    `0.16` are split into a small-touches `brand_accent` (raw) and
    a large-surfaces `brand_surface` (chroma-reduced, lightness
    pinned to a mid band, hue preserved).
  - **Case 4** — brand-vs-state collision resolver. Success,
    warning, danger anchor hues are pushed away from the brand hue
    when they collide. Rotation is bounded to `MAX_STATE_ROTATION
    = 30°` so states stay in their conventional bands (no teal
    "success" for a green brand); brands sitting inside a state's
    family accept coexistence rather than rotate the state out of
    band; achromatic brands (`chroma < 0.02`) short-circuit so the
    meaningless stored hue never triggers a rotation.
  - **Case 5** — mode-adaptive brand. Light and dark variants are
    measured independently against `LIGHT_BG`/`DARK_BG`; only the
    failing mode is adjusted. The token structure ships both
    `--rio-brand-light` and `--rio-brand-dark` (plus
    `--rio-brand-adaptive`) so a future dark-mode return is a
    configuration flip, not a refactor.
  - **Case 6** — multi-color role assignment. `surface_fitness`
    ranks input colors; ranked[0] → primary, ranked[1] →
    secondary (surfaced as `--rio-brand-secondary`), the rest →
    `--rio-chart-N`.
  - **Case 7** — safe default brand `#3f6089` when zero colors
    are supplied.
- **New CLI verb `rustio theme generate`.** Takes `--brand <hex>`
  (repeatable) and `--out <path>`. Writes the generated CSS and
  prints a per-case effect report — which cases fired, what was
  substituted, plus all seven contrast ratios measured during the
  pipeline (`brand_light vs #ffffff`, `brand_dark vs #15161a`,
  `brand_text vs bg`, `brand_accent vs bg`, semantic foregrounds
  vs white). Determinism is property-tested.
- **New `--rio-*` tokens** in the engine's output (none consumed by
  the live framework today — emitted for projects that wire the
  generated file in directly):
  - `--rio-brand-light`, `--rio-brand-dark`, `--rio-brand-adaptive`
  - `--rio-brand-surface`, `--rio-brand-accent`, `--rio-brand-secondary`
  - `--rio-brand-hover`, `--rio-brand-active`, `--rio-brand-tint`
  - `--rio-brand-text`, `--rio-muted`
  - `--rio-success-bg`, `--rio-warning-bg`, `--rio-danger-bg`,
    `--rio-info-bg` (in the drop-in compatibility block — match
    the same-named live tokens)
  - `--rio-chart-N` (variable count per input)

### Notes

- The live `crates/rustio-admin/assets/static/admin/tokens/colors.css`
  is **unchanged** by this release — the engine is opt-in for now.
  Operators who want the engine's output can replace the file with
  `rustio theme generate --brand "#0d9488" --out
  crates/rustio-admin/assets/static/admin/tokens/colors.css`, but
  some hand-tuned values (the brand-agnostic slate canvas
  `--rio-bg`, the deliberately-emerald success that visually
  rhymes with the teal brand) will shift.
- `rio-theme` is build-time only. The runtime library
  (`crates/rustio-admin`) does *not* depend on it.


## [0.16.0] — 2026-05-21

### Removed

- **Dark theme retired — the framework is now light-only.** The
  parallel dark palette was the single largest source of token-
  audit surface area and added a mode the operator UI never
  benefited from in practice. One canonical palette, audited once.
  Projects that need a dark skin still have the full `--rio-*`
  override surface and can patch `:root` from their own
  stylesheet.
  - **Files deleted** —
    `crates/rustio-admin/assets/static/admin/themes/dark.css`
    (117 LOC) and
    `crates/rustio-admin/assets/static/admin/themes/light.css`
    (46 LOC). The whole `themes/` fragment tier is gone; the
    bundle order is now `tokens → base → layout → components →
    pages → responsive → print`. The `@import` list in
    `admin/admin.css` and the `ADMIN_CSS` `concat!` block in
    `src/admin/routes.rs` were updated in lock-step (per the
    cascade-lockstep invariant).
  - **`data-rio-theme` attribute removed.** The inline bootstrap
    script in `<head>` that read `localStorage["rio-theme"]` and
    set `<html data-rio-theme="…">` before first paint is gone.
    The `prefers-color-scheme: dark` media block and the
    `html[data-rio-theme="dark"]` / `[data-rio-theme="light"]`
    selector pairs that lived in `themes/dark.css` and
    `themes/light.css` are gone with them.
  - **Theme-toggle button removed.** The "System / Light / Dark"
    ghost button in the topbar is deleted from
    `_topbar.html`; the supporting `.rio-theme-toggle` CSS block
    in `layout/topbar.css` and the JS `initTheme()` function
    (cycle handler + localStorage read/write) in
    `assets/static/admin.js` are deleted. `admin.js` shrinks
    by ~50 LOC.
  - **`AdminTheme` override selector simplified.** The injected
    `<style>` block in `_theme.html` now uses a single
    `html { ... }` selector at specificity `(0,0,1,0)` instead
    of the previous three-selector list
    `html, html[data-rio-theme="light"], html[data-rio-theme="dark"]`
    that matched the per-mode admin.css blocks at `(0,0,1,1)`.
    The override still wins cascade ties on source order
    without needing `!important`. **Public API unchanged** —
    `AdminTheme` field set and `Admin::accent_color()` /
    `Admin::theme()` behave identically; projects that only
    set fields see no diff.
  - **Tokens — colors / shadows.** No `--rio-*` token in
    `:root` was added, renamed, or removed by this change. The
    light values that previously lived in `tokens/colors.css`
    and `tokens/shadows.css` remain unchanged; only the
    duplicate per-mode override blocks (which lived in
    `themes/dark.css` / `themes/light.css`) are gone. Project
    overrides via `Admin::accent_color()` and the
    `--rio-*` custom-property surface continue to work
    exactly as before.
  - **Chrome-scope accent lift preserved.** The cascade in
    `layout/shell.css` that lifts `--rio-accent` from
    `#0F8C7E` to `#3FAA9D` inside `.rio-topbar` /
    `.rio-sidebar` / `.rio-footer` (so the base teal doesn't
    muddle against the deep-slate chrome) is **kept** — it
    was always a chrome-scope cascade, not a dark-mode
    artefact. The accompanying `--rio-text-*`, `--rio-surface-*`,
    `--rio-border-*` flips inside chrome are unchanged.
  - **Doctrine update.** `DESIGN_DOCTRINE.md` § 5
    ("Dark-mode philosophy") is rewritten as "Light-only
    stance" — three-point rationale plus a pointer for
    projects that want to override `:root` to a dark palette.
    `DESIGN_SYSTEM.md` § 9 accent table relabelled as page-
    canvas vs chrome-scope. `architecture.md`,
    `modeladmin.md`, `DESIGN_BUILDER.md` (the
    `dark_mode_default` toggle row), and `CLAUDE.md`
    aligned with the new stance.

### Changed

- **CSS migrated to logical properties — RTL-ready layout.** Closes
  the roadmap entry "Migrate `margin-left`/`padding-left` to
  `margin-inline-start`/`padding-inline-start` so `[dir="rtl"]`
  flips the layout automatically." 32 occurrences across 13 files
  rewritten as the mechanical pair-up:
  `margin-left` → `margin-inline-start`,
  `margin-right` → `margin-inline-end`,
  `padding-left` → `padding-inline-start`,
  `padding-right` → `padding-inline-end`,
  `border-left` → `border-inline-start`,
  `border-right` → `border-inline-end`,
  `text-align: left` → `text-align: start`,
  `text-align: right` → `text-align: end`.
  - **Timeline accent treated correctly under direction flip.** The
    coloured `border-inline-start` bar mirrors to the right edge in
    RTL — what an accent indicator should do. The shorthand
    `border-radius: 0 X X 0` (which rounded only the right corners
    in LTR) became the longhand pair `border-start-end-radius` +
    `border-end-end-radius` so the rounded corners follow the
    accent bar to the end side regardless of writing direction.
  - **Visual identity preserved in LTR.** Every logical property is
    a strict superset of its physical sibling for the
    default-LTR case, so `cargo test` (which doesn't render
    pixels) couldn't catch regressions here — but the CSS reads
    identically when `dir=ltr` is in effect. The cascade-lockstep
    test passed unchanged; no `@import` order edits were needed.
  - **Direction-aware positioning.** Followed up the bulk
    margin/padding migration with five `position: absolute` /
    `position: fixed` sites that hard-coded `left:` or `right:`
    — search-icon glyph anchor inside the search input, dropdown
    panel anchored to its toggle's end-side edge,
    FK-autocomplete results panel, mobile sidebar drawer
    position, and the login-aside bullet. All five now use
    `inset-inline-start` / `inset-inline-end` so they flip on
    `dir=rtl` without per-direction overrides.
  - **Icon directionality.** The icons renderer
    (`admin::icons::render_inline`) maintains a `DIRECTIONAL_ICONS`
    list (currently `arrow-left` + `log-out`) and appends a
    `rio-icon--directional` class to those glyphs on output. The
    new CSS rule `[dir="rtl"] .rio-icon--directional
    { transform: scaleX(-1); }` flips them horizontally so a "Back"
    arrow follows the reading direction without per-template
    boilerplate. Symbol-shaped icons (`home`, `users`, `database`,
    `key`) and vertical-axis chevrons (`chevron-down`) stay
    unchanged — mirroring those would produce wrong-handed glyphs.
    Four unit tests pin the marker placement: directional +
    explicit class, directional + empty class, non-directional
    icons untouched, vertical chevrons excluded.

### Changed

- **Read-only mode now hides every mutation affordance, not just
  the top-level Add buttons.** The previous release noted "per-row
  Edit / Delete buttons and form Save buttons still render in v1 —
  clicking through hits the middleware's 403 with a clear flash"
  as a deliberate v1 scoping trade-off. This release closes that
  gap: in read-only mode `list.html` drops its per-row Edit /
  Delete anchors (replaced with a quiet "read-only" label), the
  bulk-action form + bar + checkbox columns disappear entirely,
  the empty-state copy drops the "Add the first one" link;
  `form.html` drops the Save / Save-and-continue / Save-and-add-
  another buttons and the per-row Delete link in the action bar
  (replaced with "Read-only mode — saves are disabled.");
  `confirm_delete.html` / `bulk_confirm_delete.html` /
  `bulk_confirm_action.html` swap their destructive submit forms
  for a quiet "deletes/bulk actions are disabled" message + a
  Back link. Live-verified against the clinic-appointments
  example with `READ_ONLY=1`: 0 Add / Edit / Delete buttons on
  list pages, 0 Save inputs on edit forms, 1 read-only banner,
  and any URL-crafted mutation POST still gets the middleware's
  styled 403 (which was the security boundary all along; the
  template hiding is UX polish on top).

  The example crate gains a small `READ_ONLY={1,true,yes}` env
  toggle so an operator can demo both modes without rebuilding.

### Fixed

- **`FilePath` columns now read "A → Z" / "Z → A" in the sort
  dropdown.** They were falling through to the generic
  `ascending` / `descending` copy (e.g. "Photo Path
  (ascending)" on the patients list page), which read
  awkwardly — `FilePath` / `OptionalFilePath` are stored as
  `TEXT` paths and sort lexicographically, so they belong in
  the same copy bucket as `String` / `OptionalString`. The
  numeric branch (`I32` / `I64` / `OptionalI64`) keeps the
  generic copy on purpose: "Count (smallest first)" /
  "(largest first)" would read worse than "Count (ascending)".
  - Updated `sort_direction_label` in `admin::render` to
    cover `FilePath` and `OptionalFilePath` alongside the
    string variants.
  - **Five new unit tests** pin the contract per type bucket:
    strings → "A → Z" / "Z → A", file paths → same (and
    explicitly *not* numeric copy), datetimes → "oldest first"
    / "newest first", booleans → "off → on" / "on → off",
    numerics → "ascending" / "descending" (fallback
    intentionally preserved). Live-verified against the
    clinic-appointments patients page: 0 occurrences of
    "Photo Path (ascending)" or "(descending)"; 2 occurrences
    of "Photo Path (A → Z)" / "(Z → A)".

- **Humanised field labels now recognise acronyms as whole
  words.** A field named `id` used to render as `Id` in every
  surface that consumed `AdminField.label` — list-page column
  headers, the sort dropdown ("Id (ascending)" / "Id
  (descending)"), validation-error labels, and the JSON / CSV
  export headers. Now it renders as `ID`. Same fix applies to
  `ip` → `IP`, `url` → `URL`, `uri` → `URI`, `api` → `API`,
  `uuid` → `UUID`, `mfa` → `MFA`, `csv` → `CSV`, `sql` → `SQL`,
  `html` → `HTML`, `http` → `HTTP`, `https` → `HTTPS`, `json`
  → `JSON`, `tls` → `TLS`, `ssl` → `SSL`, `smtp` → `SMTP`,
  `xml` → `XML`.
  - **Whole-word rule:** the acronym must be its own
    underscore-separated segment. `video` does *not* become
    `vIDeo`; `idle` does *not* become `IDle`. Only standalone
    segments are uppercased, so existing labels that happen to
    contain an acronym substring are unaffected.
  - **Compound names** work as expected: `email_id` →
    `Email ID`, `id_card` → `ID Card`, `mfa_secret_key_id` →
    `MFA Secret Key ID`, `csv_export_path` → `CSV Export Path`.
  - **Mirrored update**: the same fix lives in both
    `rustio_admin_macros::humanise_field` (consumed at macro
    expansion to set `AdminField.label`) and
    `rustio_admin::admin::render::humanise_field` (consumed by
    `bucket_errors_by_label` to route validation errors back
    to their owning field by inverting the label). The two
    copies must stay byte-for-byte identical because the
    macros crate cannot depend on the main crate (proc-macro
    cycle); the `HUMANISE_ACRONYMS` constant is intentionally
    duplicated and documented as such in both files.
  - **Eleven new unit tests** pin the contract — five in
    `admin::render::tests` and six in
    `humanise_field_tests` — covering: standalone acronyms,
    compound acronym placement (start / middle / end), acronym
    substrings *not* uppercased (`video`, `idle`, `hidden`,
    `recipe`), datetime suffix preservation
    (`at` / `by` stay sentence-case), empty input safety, and
    plain snake_case round-trip. Live-verified across three
    pages of the clinic-appointments example:
    `/admin/patients` (0 `Id`, 2 `ID`),
    `/admin/account/sessions` (0 `Ip`, 4 `IP`),
    `/admin/account/mfa` (0 `Mfa`, 1 `MFA`).
  - This is a framework-level fix — every model on every
    consumer crate that uses `#[derive(AdminModel)]` now gets
    the correct labels without touching its own code.

### Added

- **`middleware::locale` — Accept-Language negotiation.** New
  built-in middleware that parses the inbound `Accept-Language`
  header, picks the operator's first preferred language tag,
  validates it (ASCII letters/digits/hyphens, ≤ 16 chars), and
  stashes a `Locale(String)` struct in the request context.
  Handlers read it via `req.ctx().get::<Locale>()`. Falls back
  to `DEFAULT_LOCALE` (`"en"`) when the header is missing or
  contains no valid tag — locale negotiation is best-effort and
  never rejects a request.
  - First-wins instead of q-value sort: real browsers send the
    preferred locale first, so the simpler policy matches client
    intent without traversing a sorted vector per request.
  - Re-exports: `locale`, `parse_accept_language`, `Locale`,
    `DEFAULT_LOCALE`.
  - 10 unit tests on the parser: missing / empty / single tag /
    realistic browser headers / `q=` weight stripping /
    invalid-segment fallthrough / oversized-tag rejection /
    all-invalid / whitespace tolerance / Arabic + CJK tags /
    `Locale::as_str` round-trip.
  - Pure plumbing — no template change, no handler change. The
    upcoming message-catalog work consumes this surface.

### Changed

- **Topbar unread-count badge now consistent across every
  authenticated page.** Closes the "opted-in handlers only"
  caveat from the badge MVP. New `handlers::base_with_unread`
  shared helper fetches the count once + chains
  `with_unread_count` onto a fresh `BaseContext`; 33 inline
  `BaseContext::new(Some(...))` sites across `builtin.rs`,
  `admin_recovery_handlers.rs`, `mfa_handlers.rs`, and
  `db_browser.rs` migrated to it. Four sync-closure error-path
  renders (re-auth wall, MFA-verify failure, MFA-enrol error,
  admin-reset-password error) stay on the bare constructor
  with a comment explaining the limitation — these are
  mid-auth surfaces where the badge isn't critical anyway.
  Browser-walked: `/admin/users`, `/admin/db`,
  `/admin/account/sessions`, `/admin/password_change`, and the
  per-user view all render the badge correctly now.

### Added

- **Built-in framework docs at `/admin/docs`.** Operators read
  the framework's own markdown documentation inside the admin
  chrome — no GitHub round-trip. Three top-level docs ship in v1:
  `architecture.md` (module map), `modeladmin.md` (customisation
  surface), `public-api.md` (the `pub` surface). All three baked
  into the binary via `include_str!` so the single-binary deploy
  is preserved. Rendered server-side via `pulldown-cmark` with
  GFM extensions enabled (tables, fenced code, strikethrough,
  tasklists, smart punctuation). Staff-gated. Footer link from
  every authenticated page.
  - New dep: `pulldown-cmark 0.13`, pure Rust, no C deps.
  - Larger `docs/design/*.md` doctrine files stay out — those
    are PR-review surface, not in-app reading.
  - +5 unit tests: non-empty sources, unique slugs, unknown-slug
    lookup, markdown → HTML rendering, raw-HTML pass-through
    (pins the trusted-source policy; 401 → 406 framework total).

- **Per-model 7-day creation sparkline on every dashboard tile.**
  Each model that declares a `created_at` column now renders a
  tight inline-SVG bar chart below the row count + "new this
  week" stat — same chart shape as the framework-wide audit
  sparkline, scoped to the model's own table. One
  `SELECT DATE(created_at), COUNT(*) GROUP BY day` query per
  qualifying model, padded to seven entries (today-6 .. today
  UTC). Models without `created_at` skip the chart silently —
  the rest of the tile is unchanged. Per-model query failures
  log + skip without breaking the dashboard.

- **Postgres FTS opt-in via `ModelAdmin::search_index_column`.**
  Models that declare a tsvector column (e.g. a `GENERATED ALWAYS
  AS (to_tsvector('english', coalesce(title,'') || ' ' ||
  coalesce(body,''))) STORED` column with a `gin` index) can opt
  into Postgres full-text search by returning
  `Some("search_vector")` from the new trait method. The list-page
  WHERE clause switches from the default `ILIKE` OR-loop to
  `<col> @@ websearch_to_tsquery('english', $N)` — same search
  box, indexed lookup. Pre-existing models with default `None`
  keep the ILIKE path unchanged. Column name is validated against
  `M::COLUMNS` so a typo'd config falls through cleanly to
  ILIKE rather than emitting SQL referencing a missing column.
  The new opt-in propagates through `AdminEntry` → `ListOpts` so
  the global `⌘K` palette + CSV-export filter parser honour it
  too.

- **`rustio reload` CLI verb.** Thin wrapper around
  `cargo watch -x run` — the standard Rust hot-iterate loop.
  Probes for `cargo-watch` first so a missing dependency surfaces
  a one-line install instruction instead of cargo's default
  "no such subcommand" error. Stdio is forwarded transparently
  so compile errors + project stdout/stderr land in the
  terminal exactly as the bare command would.

- **CSV import — `POST /admin/<model>/import.csv`.** Companion
  to the existing CSV export. Operators upload a multipart file;
  the framework parses it RFC 4180-style (quoted fields, doubled
  `""`, CRLF / LF / bare CR line endings) and inserts each row
  via `AdminOps::create` — same path the HTML form uses, so
  model validation runs unchanged. Per-row failures surface on
  a result page alongside the success count rather than aborting
  the batch.
  - 8 MB / 10k-row caps; header columns matched exactly against
    `AdminField.name` (extra columns dropped, missing columns
    flow into `from_form` as empty strings).
  - "Import CSV…" button on every list-page toolbar next to the
    "CSV" download — native file picker submits on change.
  - Requires the model's `change` permission (same gate as create).
  - +7 unit tests for the parser (393 → 400 framework total).

- **Topbar unread-count badge on the notifications bell.** Closes
  the deferred half of the previous cycle. `BaseContext` gains
  an `unread_count: i64` field (defaults `0`) wired via a chained
  `BaseContext::new(...).with_unread_count(n)` setter so the
  framework's 58 existing `BaseContext::new` call sites stay
  untouched. The topbar template renders a red-dot badge on the
  bell when the count is `> 0` (capped at `99+` for readability).
  Page handlers that operators actually visit — dashboard, list
  page, history, per-object history, API surface, playground,
  health, feature flags, notifications — pre-fetch the count via
  `notifications::unread_count(db, user_id)` and chain it. Pages
  that don't bother stay at the default 0 and render the bare
  bell (no regression).

- **Notifications — `rustio_notifications` + topbar bell.** Project
  code emits per-operator notifications via
  `rustio_admin::send_notification(db, user_id, message, url)`;
  operators see them at `/admin/notifications`, reachable via a
  bell icon in the topbar from every authenticated page. The
  list page surfaces the unread count + a "Mark all read"
  button that stamps every unread row with the dismissal
  timestamp.
  - Schema: `id BIGSERIAL PK, user_id BIGINT FK rustio_users
    CASCADE, message TEXT, url TEXT, read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ`.
  - Partial index `(user_id, read_at) WHERE read_at IS NULL`
    keeps unread-count queries fast.
  - New `bell` icon added to the inline-SVG catalog.
  - Real-time topbar count badge is still pending — refactoring
    `BaseContext::new` across 58 call sites is too invasive for
    this cut.

- **Feature flags — `rustio_feature_flags` + admin UI.** Project
  code calls `rustio_admin::feature_enabled(db, "new_signup_flow")`
  from anywhere a `Db` is in scope; reads hit a 60-second per-key
  in-process cache so hot paths stay cheap. Administrators manage
  flags from `/admin/feature_flags` (list with per-row toggle
  button + a "create" form for new keys), linked from the
  dashboard "Framework tools" aside. Every write invalidates the
  cache so toggles are observable on the next read. New
  `flag` icon added to the inline-SVG catalog.

- **TypeScript SDK skeleton at `/admin/apis/sdk.ts`.** Server-
  rendered companion to the OpenAPI spec — one `export interface`
  per registered project model with TypeScript types projected
  from `FieldType` (`Bool` → `boolean`, integers → `number`,
  strings / datetimes / file paths → `string`, optionals union
  with `| null`). Snake_case field names preserved verbatim to
  match the JSON envelope shape. Staff-gated, content-type
  `text/typescript; charset=utf-8` with a
  `content-disposition: attachment; filename="rustio-sdk.ts"`
  header so browsers save it directly. Linked from `/admin/apis`
  alongside the OpenAPI download and the playground. Rust /
  Python language targets and a fetch-client wrapper are still
  pending — v1 is type-only so operators wrap their own
  networking.

- **Health dashboard at `/admin/health`.** Browser counterpart
  to the `rustio doctor` CLI verb: runs the same probes (Postgres
  reachable, auth tables present, ≥1 active administrator,
  `RUSTIO_SECRET_KEY` shape) and renders a status-pill table with
  per-check detail messages and an overall-health banner.
  Administrator-gated. A "Health" link in the footer surfaces the
  page from any authenticated view alongside "Audit log" and
  "API surface". Distinct from the existing
  `/admin/healthz` public liveness probe — that endpoint stays
  machine-readable for load balancers.

- **API playground writes — `create / update / delete`.**
  `/admin/apis/playground` now closes the loop with a method
  selector (`list / detail / create / update / delete`) and
  method-conditional inputs: row-ID input appears for
  detail/update/delete; a form-encoded body textarea appears
  for create/update. The CSRF token is read from the page and
  injected automatically into every POST body — operators don't
  hand-paste it. Response pane and latency telemetry stay the
  same; the previous read-only contract is intact (the default
  `list` method renders identically to before).

- **Interactive API playground at `/admin/apis/playground`.**
  Read-only preview: pick a model + optional search / sort /
  page / per_page, click Send, see the JSON envelope rendered
  in-page with HTTP status + latency. Vanilla `fetch()`
  client-side (no third-party JS); served as one inline `<script>`
  block in the embedded template. Linked from `/admin/apis`
  alongside the OpenAPI download.

- **`rustio startproject --preset blog`.** A two-model starter
  (Post + Comment) for projects that want a relation demo out of
  the box. The preset layers a `src/comment.rs` model with a
  `post_id` FK, a `migrations/0002_create_comments.sql` table
  with a cascading delete on Post, and a `main.rs` that
  registers both `Post` and `Comment`. `--preset minimal`
  (default) reproduces the existing single-model scaffold
  byte-for-byte. Unknown preset names error out listing the
  valid choices.

- **HTML index for the API surface at `/admin/apis`.** Companion to
  the OpenAPI JSON spec: one section per registered model with an
  endpoint table (`GET /admin/<name>`, `POST …`, etc.) and a
  field table (name / type label / nullable). Top of the page
  carries a one-click "Download OpenAPI 3.0 spec →" link to the
  JSON endpoint. Footer now exposes a permanent "API surface"
  link next to "Audit log" so operators can reach it from any
  page.

- **Auto-generated OpenAPI 3.0 spec at `/admin/apis/openapi.json`.**
  A pure-functional walk over `Admin::entries()` emits per-model
  component schemas (`Bool` → boolean, integers → integer with
  `int32`/`int64` formats, strings → string, datetimes → string
  with `format: date-time`, file paths → string with a
  description; optionals carry `nullable: true`) plus full path
  coverage for list / create / detail / update / delete on every
  registered model. Form-encoded request body documented as
  `application/x-www-form-urlencoded` to match the actual wire
  shape. Role gate: Staff. Operators feed the document into API
  browsers (Postman / Insomnia) without hand-writing schemas.

### Fixed

- **Drift:** `rio-model-tile__stat-secondary .rio-model-tile__stat-num`
  used `margin-right` instead of `margin-inline-end`. Migrated to
  the logical property so RTL flips the spacing automatically.
  Slipped in via the "new this week" KPI commit last cycle.

### Added (continued)

- **7-day audit-activity sparkline on the dashboard.** Inline-SVG
  bars above the recent-activity list show admin actions per day
  for the last week, rendered server-side from a single
  `SELECT DATE(timestamp), COUNT(*)` query padded to seven entries.
  Caption reads "N actions in the last 7 days". No JS chart
  library — the SVG is plain Rust→template output that consumes
  the framework's existing accent colour token. Failure-soft on
  the query: empty bars but the rest of the dashboard renders.

- **`rustio test-init` CLI verb.** Writes a stdlib-only
  `tests/smoke.rs` integration test that boots the project binary
  via `cargo run`, polls the bound port via `TcpStream::connect`,
  sends a raw HTTP GET to `/admin/`, and asserts a 302/303
  redirect to `/admin/login`. No new project dependencies —
  ideal as a fresh project's first CI check.
  - `--force` to overwrite an existing `<out>/smoke.rs`
  - `--out <dir>` defaults to `tests` (Cargo's integration
    convention). Refuses to clobber otherwise.
  - `RUSTIO_PORT` environment override on the generated test so
    it doesn't collide with a running admin on the default 3000.

- **"New this week" KPI on the dashboard.** Each model tile now
  shows the exact count of rows with `created_at` within the last
  7 days alongside the existing approximate row total. Skipped
  silently for models that don't declare a `created_at` column.
  One `SELECT COUNT(*)` per qualifying model on dashboard render
  — fast on indexed `created_at`; failures log and skip rather
  than break the page.

- **Date grouping on history pages.** Both `/admin/history` and
  per-object history (`/admin/<model>/<id>/history`) now emit a
  `YYYY-MM-DD` divider row whenever the calendar day changes
  between consecutive audit entries. Computed in
  `map_audit_actions` via two new `HistoryEntryCtx` fields
  (`date_iso`, `is_new_day`); templates branch on `is_new_day` to
  render the divider. Closes the activity-feed item alongside
  the per-actor filter shipped previously.

- **Per-actor filter on `/admin/history`.** `?user_id=N` narrows
  the audit feed to one operator's actions. Every row's `By`
  column is now a click-through to its actor's filtered view
  (`/admin/history?user_id=<id>`). When filtered, an active-
  filter banner at the top of the page reads "Showing actions
  by <email>" with a clear (×) link back to the unfiltered
  feed. `audit::recent` gains a `user_filter: Option<i64>`
  parameter that composes with the existing `model_filter` and
  `action_filter` via AND. Resolves the actor's email
  server-side so the banner reads names, not numbers (falls
  back to `#<id>` for missing rows).

- **JSON response negotiation on write paths.** `POST /admin/<model>`
  (create), `POST /admin/<model>/<id>/edit` (update), and
  `POST /admin/<model>/<id>/delete` (delete) now mirror the
  read-path content negotiation. With `Accept: application/json`
  (or `?format=json`):
  - Success returns
    `{"ok": true, "admin_name": "<slug>", "id": N}` — status 201
    on create, 200 on update / delete.
  - Validation errors from `AdminOps::create` / `update` return
    `{"errors": ["Field: message", ...], "status": 400}`.
  - Framework errors (not-found, internal) collapse to the
    existing `{"error": "...", "status": N}` envelope.
  Request bodies are still parsed as form-encoded / multipart —
  only the response shape switches. Same permission gates, same
  audit row, same constraint-violation surfacing.
  JSON-body request parsing is a future extension.

- **Per-action permission gate on bulk actions.** New
  `BulkAction.permission: Option<&'static str>` field. When `Some`,
  the bulk handler enforces `<admin_name>.<permission>_<singular>`
  on top of the route's `change` gate. Lets destructive actions
  (e.g. `purge`, `archive`) require a scoped permission like
  `delete` without making ordinary edits harder.
  Role-bypass semantics match `perm_guard` —
  Developer/Administrator skip the check. `None` inherits and
  preserves prior behaviour.
  - **Source-compatibility break:** every existing `BulkAction {
    ... }` literal needs an explicit `permission: None,`. The two
    in-repo call sites (clinic-appointments example) plus the
    docstring example were updated.

- **DropdownText filter widget populates from `SELECT DISTINCT`.**
  `Status`-typed columns (`status`, `*_status`) now surface a chip-
  layout dropdown of distinct values queried from the column itself
  (`SELECT DISTINCT col::text FROM <table> WHERE col IS NOT NULL
  ORDER BY col::text LIMIT 50`). The column name is validated
  against `entry.fields` before format-string interpolation;
  clicking a chip composes a filter URL via the existing
  `build_list_url` so search / sort / per-page state survives.
  CSV-export filter parser mirrors the same arm. Verified
  end-to-end against `clinic-appointments`: the appointments list
  filter exposes `cancelled / completed / no_show / scheduled`
  with correct row counts (3 / 9 / 2 / 16 of 30).

- **Per-model read-only toggle.** `Admin::read_only_model("posts")`
  freezes one model — `POST` / `PUT` / `DELETE` under
  `/admin/posts/...` returns 403 with `Model posts is frozen
  (read-only)` while the rest of the admin stays writable.
  Mirrors the existing whole-admin `Admin::read_only(true)`
  pattern; the two compose (a frozen slug stays frozen even when
  the admin is otherwise writable). Useful for archive tables,
  regulatory holds, or per-model incident response without
  flipping the whole admin to read-only. New helper
  `extract_admin_name` parses the `:admin_name` segment for the
  middleware gate.

- **Audit diff on the global history page.** `/admin/history` now
  renders the same per-field before→after table the per-object
  history page already used. Update rows show the changed columns
  inline under the summary (`Title: old → new`, `Status: draft →
  published`); rows without a populated `metadata.changes` (create,
  delete, auth-flow events) render plain. Storage shape, producer
  (`do_update` → `compute_row_diff`), pure extractor
  (`extract_changes_from_metadata`), and per-object template were
  already in place — this commit mirrors the markup into
  `log_entries.html` so an operator can answer "what exactly
  changed?" without opening the object.

- **Global `⌘K` search palette — cross-model topbar search.**
  A new topbar trigger ("Search…  ⌘K") and modal palette let an
  operator search across every registered admin model from any
  page. Replaces "where is that model's list page again?" with
  one keystroke. Per-list-page search stays put — it remains the
  column-aware, filter-preserving in-page query; the palette is
  the cross-model "I know what I'm looking for" surface.
  - **Endpoint:** new `GET /admin/_search?q=<term>` returning
    JSON `{"results": [{"admin_name", "model_label", "label",
    "url"}, ...]}`. Capped at 5 results per model and 20 total.
    Sub-2-char queries short-circuit to an empty result set
    without touching the DB so single-letter input doesn't
    multiply across every registered model.
  - **Per-model `view` permission gating.** Each model the
    palette queries is gated by the operator's
    `<admin_name>.view_<singular>` permission, so the palette
    surfaces exactly the models the operator could already reach
    via `/admin/<model>`. Administrators and Developers
    short-circuit through `Role::bypasses_group_checks` —
    same posture as elsewhere in the framework. Core
    (framework-owned) entries like the synthetic User are
    skipped to mirror `find_project_entry`'s policy; the
    bespoke `/admin/users/*` surface remains the way to search
    users.
  - **`ModelAdmin::search_fields()` honoured.** Only models
    that opt in via `search_fields()` participate. Models with
    no declared search columns are silently skipped — the
    declaration is the consent.
  - **Topbar trigger.** Pill-shaped button between the brand
    and the account nav, resolving through the chrome-scope
    cascade so it reads light-on-dark inside the topbar
    automatically. Mobile (<768px) collapses to icon-only —
    the `⌘K` hint is dropped on touch where keyboard shortcuts
    don't apply.
  - **Modal palette UI.** Centered fixed-position dialog with a
    search input on top, results list grouped by model below,
    and a footer hint strip showing the keyboard surface.
    Results group under the model's `display_name`; each item
    shows the row's display label (same `pick_display_index`
    logic the FK autocomplete uses).
  - **Keyboard surface in `admin.js`.**
    - `⌘K` / `Ctrl+K` toggles open from anywhere.
    - `Esc` closes (only when open, so other Esc handlers stay
      uncontested).
    - `↑` / `↓` move selection with wrap-around; the first
      result is selected by default so `Enter` has an
      unambiguous target.
    - `Enter` navigates to the selected row's edit URL.
    - Backdrop click closes; clicks inside the dialog don't.
    - Debounced 200 ms fetch to `/admin/_search`.
  - **CSS.** New `components/search_palette.css` (~150 LOC).
    Wired into `admin/admin.css` `@import` list and the
    `ADMIN_CSS` `concat!` block in `src/admin/routes.rs`
    in lock-step.
  - **No new tokens.** Every value resolves through the
    existing `--rio-*` surface; no addition to the public
    token API.

- **`rustio audit tail --since <duration>` time-window filter.**
  Extends the audit tail subcommand shipped two commits ago
  with time-based filtering. Operators investigating an
  incident now phrase the query in real-world terms ("what
  happened in the last hour?") instead of guessing a row
  count.
  - **Format:** `<N><unit>` where unit is `s` / `m` / `h`
    / `d`. Examples: `30s`, `15m`, `2h`, `7d`. Composes with
    `--user` and `--model`: each filter intersects.
  - **Loud on typos.** A malformed duration hard-errors:
    ```
    $ rustio audit tail --since 1hr
    error: --since: cannot parse leading number from "1hr"
    (expected e.g. 30s, 15m, 2h, 7d)
    ```
    Silently treating a typo as "no filter applied" would
    mask an incident-response query, so the parser refuses
    to fall back. Unknown units (`5x`), non-numeric leads,
    negative durations all surface the same way.
  - **Implementation:**
    - New `parse_since_cutoff(raw, now) -> Result<DateTime>`
      pure parser. `now` is injected so unit tests pin a
      fixed clock.
    - `tail()`'s WHERE-clause builder switched from a
      fixed-combination match to an iterative builder that
      walks `[user_id, model, since_ts]` and appends each
      `$N` placeholder in the order it gets bound. Scales
      to N optional filters; the previous match was already
      at the limit of what fit with three filters.
    - SQL adds `a.timestamp >= $N` to the WHERE conjunction
      when `--since` is set. The cutoff timestamp is
      computed Rust-side (`now - parsed_duration`) and bound
      as a parameter — no string-formatted `INTERVAL` lit.
  - **Five new unit tests** on `parse_since_cutoff`:
    accepts s / m / h / d units, rejects empty / malformed /
    unknown-unit / negative inputs, trims whitespace,
    treats `0s` as a valid edge case (cutoff == now). Live-
    verified against clinic-appointments DB:
    `--since 1h` (no rows in the last hour given seeded
    timestamps) → `(no audit rows)`;
    `--since 30s` → same;
    `--since 1hr` → typo error message;
    `--user smoke@local --since 1d --limit 3` → 3 most-
    recent login events for that user in the last day,
    confirming filter composition.

- **`rustio group show --name <name>` CLI subcommand.**
  Operational symmetry with `rustio user perms`: that command
  answers "what can this user do?"; this one answers "who has
  access via this group, and what does membership grant?".
  Currently the only way to see a group's profile + members
  + permissions in one place was to read three tables by
  hand and join them mentally.
  - **Output:**
    ```
    Group:        Editors
    ID:           3
    Description:  Content reviewers

    Members (2):
      alice@example.test            editor          active
      bob@example.test              user            inactive

    Permissions granted via this group (3):
      posts.add_post
      posts.change_post
      posts.view_post
    ```
    The member email column auto-widens to the longest
    address in the batch so the role / active columns
    stay aligned across rows.
  - **Active flag is surfaced** alongside role: an inactive
    member is still *in* the group, but every
    `check_permission` denies for them. Pairing the two on
    one line saves operators from cross-referencing.
  - **`(none)` markers** in every section so empty groups,
    empty descriptions, and groups with no permission
    grants don't render as bare blank space.
  - **Unknown group hard-errors** (`error: no group named
    nonexistent`) rather than silently printing an empty
    report — same convention as `user perms`.
  - **Pure formatter** `format_group_show` factored out so
    the six unit tests cover the rendering layer without a
    Postgres pool: empty group `(none)` markers, empty
    description marker, member role + active status
    rendering, permissions listing with count prefix,
    email-column auto-widening on mixed-width batches, and
    members-but-no-permissions composition.
  - **Live-verified** against the clinic-appointments DB:
    `rustio group show --name editors` (which has one
    administrator member and 8 perm grants) →
    full report including all 8 permission names;
    `rustio group show --name nonexistent` →
    clean `error:` line.

- **`rustio audit tail` CLI subcommand.** Operator-facing
  read surface for the `rustio_admin_actions` table —
  sibling to the framework's HTML `/admin/history` page,
  different consumer. Useful for shell-driven incident
  response ("what did Alice do in the last hour?") and
  pipe-into-grep workflows.
  - **Usage:**
    ```sh
    rustio audit tail                          # newest 50 rows
    rustio audit tail --limit 200
    rustio audit tail --user alice@example.test
    rustio audit tail --model patients
    rustio audit tail --user alice@example.test --model patients
    ```
  - **Output:**
    ```
    TIMESTAMP            ACTION           TARGET    USER                SUMMARY
    2026-05-20 12:09:53  login_succeeded  users/2   alice@example.test (id=2)  signed in: alice@example.test
    2026-05-20 12:04:30  update           patients/12  alice@example.test (id=2)  Updated chart_number
    …
    ```
    Columns auto-widen to the widest entry in the batch so
    the table stays aligned regardless of email or model
    length. The `SUMMARY` column is last and unbounded —
    grep / awk on a row works against the structured prefix.
  - **`--user <email>` filter** resolves through
    `find_user_by_email` *before* the audit query, so an
    unknown email hard-errors (`error: no user with email
    nobody@example.test`) rather than silently returning
    zero rows. Filters compose: `--user X --model Y`
    intersects.
  - **Orphan-resilient:** the underlying schema has
    `ON DELETE CASCADE` on `user_id`, but a future relaxation
    could leave audit rows whose `user_id` no longer
    resolves. The formatter falls back to
    `(user id=99 not found)` rather than crashing.
  - **`--limit` is clamped to `[1, 10_000]`** so a typo
    can't drag the whole table into memory; default 50 fits
    a terminal screen.
  - **Read-only by doctrine.** Write-side commands (manual
    event injection, retention pruning) are deliberately
    *not* part of this surface — audit data is append-only.
  - **Pure formatter** `format_audit_tail` factored out so
    the seven unit tests cover the rendering layer without a
    Postgres pool: empty-input marker, header columns, input
    order preservation (no client-side re-sort), user
    "email (id=N)" suffix, orphan-id fallback, target
    formatting (`model/id`), and auto-widening columns on
    mixed-width batches. Live-verified against the
    clinic-appointments DB: unfiltered tail prints 5 recent
    events; `--user smoke@local` narrows to just that
    user's; `--model users` returns the 3 most-recent
    user-targeting rows; `--user nobody@example.test`
    surfaces the clean `error:` line.

- **`rustio user perms --email <email>` CLI subcommand.**
  Operational debugging surface for the question "why can't
  user X see model Y?" Without this command, the only way to
  introspect a user's effective permission set was to read
  the `rustio_user_permissions`, `rustio_group_permissions`,
  and `rustio_user_groups` tables by hand and compute the
  union in your head.
  - **Report shape:**
    ```
    User:      alice@example.test
    ID:        42
    Role:      editor
    Active:    yes

    Groups:
      Editors
      Reviewers

    Direct permissions (rustio_user_permissions):
      posts.delete_post

    Group permissions (inherited via memberships):
      via "Editors":
        posts.change_post
        posts.view_post
      via "Reviewers":
        comments.view_comment

    Effective permissions (what check_permission honours):
      comments.view_comment
      posts.change_post
      posts.delete_post
      posts.view_post
      (4 total)
    ```
  - **Role-bypass aware:** for `Administrator` and `Developer`
    (the roles `bypasses_group_checks()` is `true` for), the
    `Role:` line carries a `(bypasses group checks)` suffix
    and the effective block collapses to the marker
    `★ all permissions (role bypasses group checks) ★` —
    matching what `check_permission` actually does at the
    runtime. Direct + group grants are still printed above
    for transparency (an admin with unusual extra grants
    should be visible during a role-change audit).
  - **Inactive-user aware:** an inactive user fails every
    `check_permission` regardless of grants; the report
    surfaces this with `Active: no — every check_permission
    call denies` and an effective-block marker
    `(none — user is inactive, every check denies)`. Direct
    grants still print for transparency.
  - **Pure formatter** factored out as `format_perms_report`
    so the printing layer is unit-testable without a
    Postgres pool. Seven new tests cover: admin-bypass
    marker, inactive-deny marker, empty-user `(none)`
    markers, effective-set union + dedup across direct +
    multiple groups (`posts.view_post` granted via direct
    *and* a group appears once in `effective_perms()`), demo
    flag rendering, admin-with-grants transparency, and
    `effective_perms` cross-group dedup.
  - **Live-verified** against the clinic-appointments DB:
    `rustio user perms --email smoke@local` (administrator
    with `editors` group membership) → bypass marker plus
    full group-perm listing; `rustio user perms --email
    nobody@example.test` (missing user) → `error: no user
    with email nobody@example.test`;
    `rustio user perms --email clinic-smoke@example.test`
    (developer, no groups) → bypass marker plus `(none)`
    on every other section.

- **`GET /admin/healthz` — public liveness/readiness probe.**
  Unauthenticated by design — load balancers, k8s probes, and
  uptime checkers don't carry session cookies. Returns a small
  typed JSON envelope and a meaningful HTTP status:
  - **200 OK** with
    `{"ok": true, "db": "up", "version": "<crate>"}` when the
    framework can round-trip a `SELECT 1` against the Postgres
    pool.
  - **503 Service Unavailable** with
    `{"ok": false, "db": "down", "version": "<crate>"}` when
    the probe query fails. Readiness probes pull the pod out
    of LB rotation on the 503 without having to parse the body.
  - `version` carries `rustio-admin`'s `CARGO_PKG_VERSION` at
    compile time so operators can debug "which pod is on which
    framework version" without a separate `/version` route.
  - `Cache-Control: no-store` keeps intermediaries from caching
    a 200 across an outage.
  - Registered ahead of the wildcard `/admin/:admin_name` so a
    model literally named `healthz` can't shadow it.
  - **Three unit tests** on the response shape (pure function
    factored out from the handler so tests don't need a real
    pool): `200 + db:up` on the happy path, `503 + db:down` on
    failure, `Content-Type: application/json` and
    `Cache-Control: no-store` headers present. Live-verified
    against the clinic-appointments example:
    `curl http://127.0.0.1:3000/admin/healthz` (no auth) → 200
    `application/json` with body
    `{"db":"up","ok":true,"version":"0.15.1"}`; no `Location`
    redirect to `/admin/login` (probe is genuinely
    unauthenticated).
  - **Scope note:** route auto-`HEAD`-from-`GET` is not wired
    in the framework's router, so `HEAD /admin/healthz` returns
    405. Health-check tools that need HEAD support can do
    `curl -X GET` instead; adding HEAD across the router is a
    wider, separate change.

- **JSON error envelopes on the JSON detail endpoint.** Closes
  a sharp edge in the read-only JSON API shipped two commits
  ago: a client hitting `GET /admin/<model>/<missing-id>` with
  `Accept: application/json` (or `?format=json`) was getting
  the framework's HTML 404 page back, breaking the JSON
  response-shape contract. Now the same path returns a typed
  JSON envelope at the right HTTP status:
  ```
  HTTP/1.1 404 Not Found
  Content-Type: application/json

  {"error": "<message>", "status": 404}
  ```
  - **New helper** `json_api::json_error(Error) -> Response`
    maps every `Error` variant to its declared HTTP status
    (`Error::status()`) and uses `Error::client_message()` for
    the body so the framework's 500-redaction policy carries
    over — internal-error detail stays in logs, the envelope
    sees a generic "Internal Server Error".
  - **`show_object_json`** now captures the `wants_json`
    decision once at the top of the handler and threads it
    through both the model-lookup and row-fetch error paths,
    so any error that escapes those calls renders as JSON
    when the client wanted JSON. Browsers hitting the same URL
    without `Accept` still get the existing 303 redirect to
    `/admin/<model>/<id>/edit` — no change to the HTML UI.
  - **Four unit tests** on the envelope shape: content-type +
    status code carry-through, body-shape `{error, status}`,
    redaction of `Internal` error detail, and a status-mapping
    sweep covering every `Error` variant so a future variant
    can't silently slip through. Live-verified against
    clinic-appointments: `GET /admin/clinics/99999` with
    `Accept: application/json` → `404 application/json
    {"error":"clinics/99999","status":404}`; same with
    `?format=json`; browser default → unchanged `303 → /edit`;
    `GET /admin/clinics/1` JSON-OK path → unchanged.
  - **Scope note:** the list endpoint's JSON branch is left
    untouched. The only error path before the JSON
    short-circuit there is a DB `Error::Internal` on
    `ops.list()`, which is rare and lower-leverage than the
    detail-endpoint 404. JSON error coverage for the list
    endpoint, and for middleware-level 401/403 redirects on
    JSON clients, is a separate follow-up.

- **CSV export on the list endpoint** (`?format=csv` or
  `Accept: text/csv`). Sibling of the JSON read surface — same
  routes, same auth gate, same filter / search / sort /
  pagination state, but RFC 4180 CSV instead of JSON. Detail
  endpoint stays JSON-only — CSV for a single object doesn't
  carry meaningful structure.
  - **Content negotiation** lives in a new `admin::csv_export`
    module. `wants_csv(req)` returns `true` when the `Accept`
    header lists `text/csv` ahead of any `text/html` or
    `application/json` preference, or when the URL carries
    `?format=csv`. Browser defaults (`Accept: text/html`) keep
    returning HTML; clients negotiating both `text/csv` and
    `application/json` get whichever they listed first.
  - **Output is RFC 4180:** CRLF line terminators; fields
    containing `,`, `"`, `\r`, or `\n` are wrapped in double
    quotes; literal `"` inside quoted fields is escaped by
    doubling (`""`). Header row uses `AdminField.label` so the
    spreadsheet columns match the human labels the HTML table
    shows. Data rows reuse the per-cell stringification the
    HTML list page already does (`row.cells`), so booleans stay
    `true`/`false`, null `Optional*` slots become empty cells,
    and datetimes render in the framework's `%Y-%m-%dT%H:%M`
    form.
  - **Download UX:** the response sets
    `Content-Disposition: attachment; filename="<admin_name>.csv"`
    so browsers prompt a download rather than rendering inline.
  - **Auth:** piggybacks on the existing session-cookie path;
    same per-model `view` permission gate as the list page;
    audit emission unchanged.
  - **Seven unit tests** on the writer + negotiator: simple
    cells stay unquoted, comma cells quote, double-quote cells
    double their quote, newline cells quote, CRLF row
    termination, mixed quoted / unquoted cells in one row,
    Accept-header precedence (`text/csv` first wins; `text/html`
    or `application/json` first defeats it). Live-verified
    against the clinic-appointments example:
    `GET /admin/clinics` + `Accept: text/csv` → 200,
    `Content-Disposition: attachment; filename="clinics.csv"`,
    header `id,Name,Address,Is Open,Created At`, 5 data rows;
    `GET /admin/patients?format=csv` → 20 patient rows with
    proper bool serialisation and empty cell for unset
    `photo_path`; HTML / JSON paths unchanged; mixed
    `Accept: text/csv, application/json` → CSV wins; mixed
    `Accept: application/json, text/csv` → JSON wins. `xxd`
    confirmed `0d 0a` CRLF terminators on every row.

- **JSON API (read-only) on the existing list + detail routes.**
  Closes the read-side half of the roadmap entry "CRUD API
  generation. A `?format=json` URL switch (or
  `Accept: application/json`) on existing list / detail /
  create / update / delete routes." Write-side
  (create / update / delete via JSON) is a deferred follow-up —
  error-shape design and partial-success semantics on bulk
  actions warrant a separate ship; the read-side surface here
  doesn't lock in any decision that would constrain it.
  - **Content negotiation** lives in a new `admin::json_api`
    module. `wants_json(req)` returns `true` when the `Accept`
    header carries `application/json` (or `application/*`)
    ahead of any `text/html` preference, or when the URL
    carries `?format=json`. Browser defaults (`Accept: text/html`)
    keep returning HTML — no behaviour change for the
    everyday admin UI.
  - **List endpoint** (`GET /admin/<model>`) now short-circuits
    ahead of the FK-hydration + UI-context assembly when the
    client wants JSON. Returns
    `{"rows":[…], "total":N, "page":N, "per_page":N, "pages":N}`
    under the same filter / search / sort / pagination the
    HTML page honours. Each row is a flat object keyed by
    `AdminField.name` with per-cell type coercion (bool for
    `Bool`, integer for `I32`/`I64`, string for everything
    else; `null` for empty `Optional*` slots).
  - **Detail endpoint** at the brand-new `GET /admin/<model>/<id>`
    route returns the row as a flat JSON object with the same
    typed shape. Browsers landing on the bare URL get a 303 to
    `/admin/<model>/<id>/edit` so the form UI stays the
    primary HTML surface. View permission, same gate as the
    list page.
  - **Authentication** piggybacks on the existing
    session-cookie path; an API client signs in once via
    `POST /admin/login` and reuses the cookie. Per-model
    `view` permission still enforced; audit emission
    unchanged. API-token-based auth for non-browser clients
    is a separate follow-up.
  - **Six unit tests** on the cell-typing helper: bool
    round-trip, integer parse, garbage-in-int-column tolerant
    fallback (renders the raw string rather than dropping the
    row), nullable-string / nullable-int / nullable-filepath
    null-on-empty, required-string keep, file-path
    string-or-null. Live-verified against the
    clinic-appointments example:
    `GET /admin/clinics` + `Accept: application/json` →
    `{"total":5, rows:[{name:"Downtown Family Health",
    address:"120 Main Street", is_open:true, …}, …]}`;
    `GET /admin/patients?format=json` → 20 rows with proper
    typing of `is_active` (bool) and `photo_path` (null when
    unset); `GET /admin/clinics/1` (browser, no Accept) → 303
    to `/edit`; HTML list rendering unaffected.

- **File-upload widget — `#[rustio(file)]`, multipart parsing,
  local-filesystem storage, served-back endpoint.** Closes the
  roadmap entry "File / image upload widget." Scoped to a
  single-process, local-filesystem v1 with end-to-end coverage:
  - **New `FieldType::FilePath` / `OptionalFilePath`** in
    `crate::admin::FieldType` — semantically a string column,
    rendered as `<input type="file">`.
  - **New macro attribute `#[rustio(file)]`** on `String` /
    `Option<String>` fields promotes them to the new field
    types. Compile-error on non-string types so a typo'd
    attribute on an `i64` column doesn't silently render as a
    text input.
  - **Hand-rolled multipart parser** in a new internal module
    (`crate::multipart`). Splits the body by the
    `Content-Type` boundary, parses each part's headers
    (`Content-Disposition` for `name` + `filename`, optional
    `Content-Type`), and returns ordered `MultipartPart`s.
    Header cap at 16 KB; whole-body cap at 16 MB; per-file cap
    at 16 MB. Ten unit tests cover boundary extraction, text-
    only parts, file parts, mixed forms, missing-opening-
    boundary refusal, and unterminated-headers refusal.
  - **`Admin::uploads_dir(path)`** builder + accessor. The
    framework creates the directory lazily on first write;
    leaving it unset makes file inputs render but silently
    drop any uploaded bytes (project hasn't opted in yet).
  - **`Request::form()` is now multipart-aware** — text-only,
    file parts dropped. Lets the CSRF middleware (and any
    other early consumer) read text fields out of a multipart
    body without manually handling the parse.
  - **Handler-side multipart write** in `do_create` /
    `do_update`: `parse_form_with_uploads` writes each file
    part to `<uploads_dir>/<uuid-v4>-<sanitised>` and injects
    the resulting relative path string back into the
    `FormData` slot under the part's `name`. The model's
    generated `from_form` then sees a normal `String` column —
    no special-case persistence path.
  - **Filename sanitisation** strips path components on both
    Unix and Windows separators, replaces all but
    `[A-Za-z0-9._-]` with `_`, trims leading dots
    (`.htaccess` → `htaccess`), and caps at 120 chars. Six
    unit tests pin plain-pass, traversal strip, unsafe-char
    replacement, leading-dot trim, length cap, and empty
    fallback.
  - **`GET /admin/uploads/:filename`** serve route, Staff-role
    gated. Canonicalises the configured root, refuses any
    resolved path that doesn't live inside it; rejected
    lookups return 404. Defense-in-depth: also refuses raw
    `..`, `/`, `\` in the path segment ahead of canonical
    resolution. Per-extension `Content-Type` guesser for
    common image / document MIME types.
  - **Edit-form preview** — when an existing column carries a
    relative path, the form renders a `Current: <link>` row
    below the file input so the operator sees what's already
    persisted before picking a replacement.
  - **Clinic example** gains `Patient.photo_path:
    Option<String>` with `#[rustio(file)]`. New migration
    `0006_patient_photo.sql` adds the column; `main.rs` sets
    `uploads_dir` to `./uploads` (gitignored). Live-verified
    end-to-end: POST multipart upload → 303 redirect, DB row
    carries `<uuid>-<sanitised>` path, on-disk file written,
    `GET /admin/uploads/<rel>` returns the bytes verbatim,
    `GET /admin/uploads/..%2Fetc%2Fpasswd` returns 404.

- **`ModelAdmin::inlines()` — related-children sections below the
  parent edit form (read-only v1).** Closes a roadmap entry —
  scoped as a defensible v1: project authors declare which child
  models to surface inline on the parent edit page, the
  framework fetches up to `max_rows` matching rows, and renders
  each as a click-through table with Edit / Delete buttons +
  "Add new …" and "View all …" affordances. **In-page editing
  of inline rows is deferred** — Django-style nested formset
  semantics need a new POST surface, transaction wrap, and JS
  add/remove flow that didn't fit a single ship-quality commit.
  The read-only display + click-through is a strict improvement
  on "no inline display at all" and lays the surface for the
  edit-in-place iteration.
  - **New `Inline` struct** (re-exported at crate root) with
    fields `target_model` (must match a registered admin
    entry's `SINGULAR_NAME`), `fk_field` (column on the child
    that points at the parent), `label` (optional section
    title), `max_rows` (cap), and `display_field` (optional
    column for the per-row click-through label; falls through
    the framework's `name → title → full_name → email` ladder
    when `None`).
  - **New trait method** `ModelAdmin::inlines() -> &'static [Inline]`
    with default `&[]`. `AdminEntry` captures the slice at
    registration; `show_edit_form` + `do_update`'s error path
    fetch the children via `target.ops.list(...)` filtered by
    `(fk_field, parent_id)` — reuses the existing list-page
    SQL builder, column-name validation, and parameterised
    binding.
  - **Template surface**: the `form.html` block below the action
    bar iterates `inlines` and renders a `rio-inline-section`
    per declared inline with row count ("Showing N of total"),
    a table of click-through rows, and footer actions.
    Read-only mode hides Edit / Delete / Add — rows become
    "View" links instead.
  - **Clinic example** demonstrates three Inlines:
    `Patient → Appointment` by `patient_id` (display by
    `scheduled_at`), `Practitioner → Appointment` by
    `practitioner_id` (same display), `Clinic → Practitioner`
    by `clinic_id` (display by `full_name`). Live-verified:
    `/admin/patients/1/edit` shows Elizabeth Bennet's two
    appointments labelled by timestamp; `/admin/clinics/1/edit`
    lists all six on-staff practitioners by name.

- **Saved filters — per-user bookmarkable list-page presets.**
  Closes the roadmap entry "Bookmarkable named filter presets per
  user. Stored on `rustio_users` or a new join table." A new
  `Saved` dropdown on every list-page toolbar lets the operator
  bookmark the current filter + search + sort state under a
  name, apply any of their saved presets in one click, and
  delete the ones they no longer need.
  - **New table** `rustio_admin_saved_filters` (lazy-created on
    first list-page hit, like `audit::ensure_table`):
    `(id, user_id, admin_name, name, query_string, created_at)`
    with `UNIQUE (user_id, admin_name, name)` so re-saving the
    same name upserts rather than duplicates.
  - **Two new routes** under each model:
    `POST /admin/:admin_name/saved_filters` (save / upsert) and
    `POST /admin/:admin_name/saved_filters/:id/delete` (remove,
    SQL-scoped to `identity.user_id` so an operator can only
    delete their own bookmarks). Both gated at `Role::Staff` —
    the same surface as the list page itself; no separate
    permission to manage.
  - **URL-payload contract.** The stored `query_string` is the
    raw filter / search / sort segment (e.g.
    `q=anna&status=active&sort=scheduled_at&dir=asc`); applying
    a saved filter is `<a href="/admin/<name>?<query_string>">`,
    no per-widget parser. New filter widgets ride for free —
    they show up in saved filters the moment they ship.
  - **Visual `is-active`** marker when the current request's
    query string matches a saved preset, so the operator sees
    which bookmark they're looking at.
  - **Read-only allowlist.** Saved-filter create + delete are
    per-operator UI state (bookmarks), not project-data
    mutations — added to `is_read_only_writable_path` so they
    remain usable when `Admin::read_only(true)` is on.
  - **New `bookmark` icon** in the shared catalogue + new
    `.rio-saved-list*` / `.rio-saved-form*` CSS in
    `components/dropdowns.css`. No new fragment, no `admin.css`
    lock-step disturbance.
  - **Six sanitise-helper unit tests** cover empty / whitespace
    rejection, whitespace trimming, character-count truncation
    at the 120-char name cap, Unicode passthrough, empty-query
    allowed (bookmarks "the default view"), and query truncation
    at the 2048-char cap. Live-verified against the
    clinic-appointments example: save returns 303, empty name
    returns 400, DB carries the exact `(name, query_string)`
    pair, list page renders the dropdown with the proper badge
    count + `is-active` marker on the currently-applied preset,
    delete returns 303 and the DB row count drops accordingly.

- **Approximate row count per model on the dashboard.** The
  per-app model index now carries an "~ N" cell between the
  display name and the Add / View buttons, sourced from
  `pg_class.reltuples` via a single batched
  `WHERE relname = ANY(...)` query against every registered
  project model's table. Refreshed by `ANALYZE` / autovacuum
  (not every INSERT) so it reads as approximate by design — the
  prefix `~` and a `title=…` tooltip make that explicit.
  Operators get at-a-glance volume per model without paying the
  cost of `COUNT(*)` per table on the most-visited page. Failures
  (query error, schema not yet migrated) log a warning and
  render `~ 0` rather than 500ing the dashboard. Cells use a
  monospace + tabular-nums style so 5-digit counts line up
  beneath 2-digit counts. Live-verified against
  clinic-appointments: shows `~ 5` clinics, `~ 20` patients,
  `~ 30` practitioners, `~ 30` appointments after a one-time
  `ANALYZE`.

### Fixed

- **FK autocomplete labels resolved when the target has a
  `full_name` column.** `pick_display_index`'s fallback ladder
  previously checked only `name` / `title`, so a target like
  `Patient` (which uses `full_name`) ended up rendering every
  lookup result as `#<id>`. Extended the ladder to
  `name → title → full_name → email`, covering both Library /
  Catalogue / Article / Movie shapes and Person / Customer /
  Patient / Employee shapes (the universal `full_name` column
  across both shipped examples). Live-verified against the
  clinic-appointments example: `?q=Anna` against
  `/admin/_lookup/patients` now returns
  `[{"id":9,"label":"Anna Karenina"}]` instead of the previous
  `[{"id":9,"label":"#9"}, …]`-with-no-filter shape.
- **Clinic example models declare `search_fields()`.** Without
  this the FK autocomplete endpoint had nothing to ILIKE
  against, so `?q=…` returned every row in the target table
  (the dropdown still rendered, just unfiltered). Patient
  searches now scan `chart_number` + `full_name` + `email`;
  Practitioner scans `full_name` + `license_no`; Clinic scans
  `name` + `address`. The framework's lookup handler already
  knew how to honour this — only the example was missing the
  declarations.

### Changed

- **Canonical example replaced.** `examples/library-circulation/`
  (Branch + Patron + Item + Loan with circulation bulk actions)
  retired in favour of `examples/clinic-appointments/`
  (Clinic + Patient + Practitioner + Appointment with
  appointment-lifecycle bulk actions). Same framework-feature
  coverage — 4 models, 3 FKs, `CHECK`-constrained status enums,
  `belongs_to` relation metadata, SELECT-then-UPDATE bulk
  actions with partial-success semantics, the canonical
  `LettreSmtpMailer` for project-side SMTP, project-identity
  branding through `Admin::app_name` / `app_tagline` /
  `support_email`. Workspace `Cargo.toml`, CLAUDE.md, and
  `docs/design/DESIGN_EMAIL.md` updated to point at the new
  path; the historical `docs/archive/RUSTIO_STRATEGY.md`
  reference is preserved as it documents the strategy in flight
  at the time. Seed dataset (5 clinics, 20 patients, 30
  practitioners, 30 appointments) follows the same shape as the
  retired library data: public-domain literary names,
  `@example.test` emails, `NOW() - INTERVAL` timestamps so a
  fresh seed always reads as recent, three appointments
  deliberately seeded as `scheduled` with a past
  `scheduled_at` so the `mark_no_show` bulk action has obvious
  candidates the moment an operator clicks through.

### Added

- **`/admin/db` — read-only Postgres schema explorer.** Closes the
  roadmap entry "Database browser. A read-only schema explorer at
  `/admin/db` showing every table, its columns, foreign keys, and
  row counts. Useful during development; hidden behind the
  Developer role." Diagnostic surface for inspecting the `public`
  schema without leaving the admin chrome or reaching for `psql`.
  Three `information_schema` / `pg_catalog` queries pull every
  table, its columns, and its foreign-key relationships; the
  template composes a per-table card with column list (name /
  type / nullable / default) and FK lists (outgoing + incoming
  with anchor links between cards).
  - **Approximate row counts.** Uses `pg_class.reltuples` rather
    than `COUNT(*)` per table — cheap and batched. Surfaced as
    "~ N" so operators read it as approximate; ground truth lives
    on the list page where the existing accurate count runs under
    the same WHERE.
  - **Permission boundary.** Route gated by `role_guard(Role::Developer)`
    in `routes.rs`; the sidebar surfaces a "Developer →
    Database" entry only when `identity.is_developer`. No DDL,
    no row sampling, no mutation surface.
  - New `db_browser` module + `admin/db_browser.html` template
    (43rd embedded template; `rustio override` now reports 43).
    New `pages/db_browser.css` wired through both the
    `admin.css` `@import` list and the `ADMIN_CSS` `concat!`
    block in `routes.rs` (cascade-lockstep test still passes).
- **`ModelAdmin::validate` — project-driven business-rule
  validation hook.** Closes the roadmap entry "Project closures
  that return validation errors before the row hits the database,
  surfaced in the same UI as the existing constraint-violation
  flash." New trait method (default `Ok(())`) called by
  `ConcreteOps::create` and `::update` AFTER `from_form` succeeds
  but BEFORE the SQL fires. Project authors override to add cross-
  field or domain rules:
  ```rust
  fn validate(model: &Self) -> Result<(), Vec<FieldValidationError>> {
      let mut errs = Vec::new();
      if model.start_date > model.end_date {
          errs.push(FieldValidationError::field(
              "end_date",
              "End date must not be before the start date.",
          ));
      }
      if errs.is_empty() { Ok(()) } else { Err(errs) }
  }
  ```
  - New `FieldValidationError` struct (re-exported at crate root)
    with two constructors: `::field(name, msg)` attaches the
    error to one input (rendered inline with `aria-invalid`);
    `::global(msg)` lands the message in the form-level banner
    (appropriate for cross-field rules with no single owning
    field). Plain `Send + Sync` owned struct so the result vec
    can cross await points freely.
  - Errors flatten through the existing `bucket_errors_by_label`
    pipeline: field-attached entries get prefixed with the
    field's `AdminField.label` so the bucketer routes them to
    the right input; global entries pass through to the banner.
    A typo'd field name falls through to the banner rather than
    being dropped silently.
  - Synchronous — validation can't query the DB. Database-shape
    errors (UNIQUE violations, FK gone) flow through the
    existing constraint-translation path automatically.
  - Four unit tests cover the four flattening paths
    (field-attached prefixing, global passthrough,
    unknown-field-name fallback, mixed-order preservation)
    against a `StubModel` that exposes a minimal `FIELDS` slice.
- **Project-model CRUD audit + per-update diff view.** Generic
  `do_create` / `do_update` / `do_delete` now emit
  `rustio_admin_actions` rows so per-model history pages
  (`/admin/<model>/<id>/history`) actually populate — pre-this-
  release they always read as empty because the framework only
  audited auth / identity / MFA / bulk flows. **Update events
  capture a before/after diff** of every editable column whose
  value changed; the metadata JSONB carries a `changes` array of
  `{field, label, from, to}` objects, and the object-history
  template renders a tight `<dl>` of changed columns under each
  Update row with strike-through `from` and accent-coloured `to`.
  - Non-editable fields (auto-touched `updated_at`, `password_hash`
    etc.) are excluded from the diff — they aren't user-driven
    changes and including them would bury the real signal.
  - Empty-diff updates still emit an audit row (the human action
    happened), with a summary that reads "Updated Post #42 (no
    field changes)" so the timeline stays honest.
  - `AdminAction` gains a `metadata: Option<serde_json::Value>`
    field; `recent()` and `for_object()` SELECTs now pull the
    JSONB column. The History page renderer extracts the
    `changes` array via a new `extract_changes_from_metadata`
    helper that defends against malformed metadata (missing
    key, wrong type, entries missing `field`) by falling back to
    an empty render rather than panicking.
  - **10 new unit tests** cover the diff helper
    (`compute_row_diff`: editable-only filter, unset→set
    transition, no-change emits nothing, multi-field
    ordering) and the metadata extractor (None, missing key,
    non-array, full round-trip, label fallback, malformed-entry
    skip). All pass against in-memory snapshots — no DB.
- **Whole-admin read-only mode.** New `Admin::read_only(bool)`
  builder method puts the admin into a frozen, view-only state:
  every mutating verb (`POST` / `PUT` / `PATCH` / `DELETE`) under
  `/admin/*` returns 403 from a new `read_only_guard` middleware,
  rendered through the existing styled error pipeline. Auth-flow
  routes are explicitly allowlisted (login / logout / reauth / MFA
  verify / forgot-password / reset-password / must-change-password
  / password-change / own-session management / own-MFA management)
  so operators can still sign in and out of a frozen admin —
  useful for incident response (lock the system while
  investigating) and demo environments (viewers without editors).
  - **Security boundary is the middleware.** Templates that
    surface mutating affordances consult the new
    `BaseContext.read_only` flag to suppress the most prominent
    top-level "Add" buttons on the dashboard and list pages, and
    `_base.html` renders an info-tinted banner ("READ-ONLY —
    project-data mutations are disabled…"). Per-row Edit / Delete
    buttons and form Save buttons still render in v1 — clicking
    through hits the 403 with a clear flash. Hiding every mutation
    button per-template is a known follow-up, deliberately deferred
    so v1 stays narrow.
  - **Allowlist matcher pulled out as a free fn**
    (`is_read_only_writable_path`) so the policy is unit-testable
    without booting the router. Five tests cover the four exact
    auth paths, the three prefix-allowlisted families
    (`/admin/reset-password/`, `/admin/account/sessions/`,
    `/admin/account/mfa/`), the project-data mutations that must
    be blocked, the non-admin paths the middleware never reaches,
    and the HTTP-method discriminator.
- **CSV export per model.** New
  `GET /admin/:admin_name/export.csv` endpoint streams every row
  matching the current list-page state (search + filters + sort) as
  a UTF-8 CSV. The list-page toolbar gains a `Download CSV` button
  that preserves the active query string so an operator's filtered
  view rounds-trips into a filtered export. Capped at 10 000 rows
  per download — operators with bigger tables should narrow the
  filters first; the cap protects the worker from buffering the
  full table into a `Vec<ListRow>`. Header row uses each field's
  humanised label (same source as the on-screen column header);
  cells come straight from `AdminModel::display_values()`, with
  RFC 4180 quoting applied to any cell containing `,` / `"` / `\r`
  / `\n` and internal `"` doubled. Same `view` permission gate as
  the list page; core entries (the synthetic `User`) are blocked
  by `find_project_entry` so `/admin/users/export.csv` 404s
  cleanly. A small `parse_sql_filter_query` helper in `handlers.rs`
  mirrors the SQL-side filter arms inside `list_model` so the CSV
  honors `BoolYesNo`, `MultiSelect`, `DateRange`, and
  `FkAutocomplete` URL parameters exactly the same way the on-
  screen view does (the duplication is called out with a
  `keep-in-lockstep` comment; promoting one source of truth is a
  future refactor). Six unit tests pin the CSV escape function
  against the four RFC 4180 corner cases (plain pass-through,
  comma, embedded quote doubled, newline) plus safe-punctuation
  and Unicode passthrough. New `download` lucide icon in the
  shared catalogue.
- **Per-model template overrides — finishing the partial wiring.**
  `Templates::render_for_model` shipped in an earlier release but
  carried `#[allow(dead_code)]` because no handler consumed it. The
  generic CRUD render call sites (`list_model`, `show_new_form`,
  `do_create`, `show_edit_form`, `do_update`, `show_delete_confirm`,
  `show_object_history`) now all route through it, so a project can
  drop `templates/admin/<admin_name>/list.html`,
  `…/form.html`, `…/confirm_delete.html`, or
  `…/object_history.html` to override just that one page for just
  that one model. Lookup order is: per-model file →
  framework-wide override → embedded default; the orphan-admin-file
  validator was already non-recursive (only scans
  `<disk_root>/admin/*.html`), so model subdirectories are
  invisible to the warn-on-typo pass and don't surface false
  positives. Two new unit tests pin the precedence: per-model wins
  over framework-wide, and a model with no per-model file falls
  through cleanly without bleeding another model's override into
  the result.
- **`rustio theme` — curated `AdminTheme` palette presets.** New
  CLI verb with two subcommands: `rustio theme list` enumerates the
  four out-of-the-box presets (ocean / forest / sunset / monochrome,
  per the roadmap entry); `rustio theme show <name>` prints a
  complete, ready-to-paste `.theme(...)` clause to stdout for the
  operator's `Admin::new()` builder chain. No source mutation — the
  verb never touches `main.rs` or any other project file, matching
  the conservative posture `rustio override` established (copy a
  template to disk, but never edit existing code). Each palette is
  six normalised `#rrggbb` hex values (accent + bg + surface + text
  + text_muted + border) tuned against `DESIGN_DOCTRINE.md`:
  comfortable saturation, no neon, no OLED-black. `show` is
  case-insensitive so `rustio theme show Ocean` works; an unknown
  preset returns an error that lists the available names so
  operators don't need a second `list` run to recover. Five unit
  tests pin the preset count, the hex-form invariant, the
  fluent-builder method coverage of the printed snippet, the
  unknown-preset error path, and the case-insensitive lookup.
- **`rustio override <template>` — copy an embedded admin template
  to disk for editing.** Closes the roadmap entry "the operator
  does this by hand today." The verb pairs with the existing
  `RUSTIO_TEMPLATE_DIR=./templates` disk-loader path: running
  `rustio override admin/list.html` drops the framework default at
  `./templates/admin/list.html`, the operator edits it, and the
  next run picks up the override. With no arguments the verb lists
  every available template name (42 in this release). `--force`
  overwrites an existing on-disk file; without it the verb refuses
  to clobber so in-progress edits stay safe. `--out` overrides the
  destination root (defaults to `./templates`).
  - Two new public helpers in the library —
    `embedded_template_names()` returning every canonical template
    name in stable order, and `embedded_template_source(name)`
    returning the byte-for-byte body. The CLI reads through these
    so the embedded-templates list stays the single source of
    truth across both crates.
  - Four unit tests cover the unknown-template error path, the
    path-traversal defense, the byte-for-byte copy, and the
    refuse-to-clobber-without-force semantics; an end-to-end smoke
    run against the built binary confirmed all three modes
    (list / copy / refuse) work as advertised.
- **List-view search-result highlighting.** When `?q=…` is in
  effect, every cell in a column the search clause actually scanned
  (`ModelAdmin::search_fields()`) gets its matched substring wrapped
  in `<mark>` tags — operators can see at a glance *which* part of
  *which* column was hit, without having to re-scan the row. The
  highlight HTML is built server-side in `render::list_ctx` (a new
  pure helper `highlight_search_match` with eight unit tests pinning
  the matching + HTML-escaping behaviour) and exposed through a new
  `ListRowCtx.highlights: HashMap<String, String>` channel separate
  from the plain `values` map; cells with no match render via the
  existing string path unchanged. The mark element gets calm
  styling in `base/typography.css` — accent-tinted background that
  inherits the cell's foreground colour so dark mode just works.
  - **Matching semantics**: ASCII-case-insensitive
    (`"anna"` matches `"Anna Lindqvist"`); non-ASCII characters
    compare exactly. Full Unicode case folding deferred — the
    common case is ASCII, and the missed-non-ASCII case still
    renders the row, just without highlight.
  - **XSS-safe**: the cell text is HTML-escaped char-by-char inside
    and outside the `<mark>` wrap; eight tests cover the escaping
    around marks, escaping of HTML specials *inside* matched
    substrings (operator searches for a literal `<`), adjacent
    matches, multi-match, and non-ASCII passthrough.
- **Per-session revoke affordance on the user-profile sessions
  tab.** The roadmap entry "the `user_view.html` page already lists
  active sessions; add a 'revoke this session' affordance" is now
  closed. Each row in the Sessions tab gets a per-row `Revoke`
  button that posts to `/admin/users/:id/sessions/:session_id/revoke`
  (Administrator role-gated), narrower than the existing bulk
  `revoke-sessions` admin action — no reason field, no re-auth wall,
  no confirmation step. The handler enforces three orthogonal
  guards: cross-rank (actor's role must dominate target's), session
  ownership (the row must belong to the named user and be active),
  and same-actor-session refusal (defense in depth — the template
  hides the button on the actor's own row, but URL-crafting around
  it still fails with a 400). Audit emits one
  `SessionsRevokedByOther` per revoked session, carrying actor /
  target / correlation_id / IP just like the bulk surface.
  `load_user_sessions` now filters active rows only
  (`revoked_at IS NULL AND expires_at > NOW()`) since a revoked
  row can't be re-revoked through the UI, and threads the viewing
  admin's session id down so the template knows which row to mark
  `is_current`.
- **Audit on authentication events.** Two new `AuditEvent`
  variants — `LoginSucceeded` and `LoginFailed` — close a real
  gap in the security trail: `do_login` previously emitted an
  `AccountLocked` row when the auto-throttle tripped but logged
  nothing for the individual successful / failed attempts that led
  there. The History page now shows every sign-in attempt against
  a known account, including the `reason` discriminator
  (`"wrong_password" | "inactive" | "locked"`) so an operator can
  distinguish "brute force on a real account" from "someone is
  poking the locked account waiting it out." `LoginSucceeded`
  carries `metadata.mfa_pending = true` when the user has MFA
  enrolled and the response redirects to the MFA-verify step, so
  the History pill `"Signed in"` doesn't read as "fully
  authenticated" before the second factor lands.
  - **Known limitation, documented on the `LoginFailed` doc**:
    attempts against an email with no matching `rustio_users` row
    are *not* audited — `rustio_admin_actions.user_id` has a
    `NOT NULL REFERENCES rustio_users(id)` FK, so there's nowhere
    to anchor the row. Promoting that to audit needs a migration
    loosening the FK and is out of scope here. The trace log still
    captures the attempt at `info` level.
  - The four existing drift tests (audit-string uniqueness +
    snake-case + stable-strings; `action_label` coverage +
    `action_pill_class` validity) ride the new variants for free.
- **`FilterKind::FkAutocomplete` — the fourth list-page filter
  widget, closing out the list-view filter roadmap.** Any
  foreign-key column with a resolvable target in the
  `RelationRegistry` (declared via `#[rustio(belongs_to = "…")]`)
  now surfaces a type-ahead picker inside the Filters dropdown
  instead of a raw numeric input.
  - **New endpoint** `GET /admin/_lookup/:admin_name?q=&limit=` —
    returns up to 50 (`limit` clamp 1..=50, default 20) rows of the
    target model as `[{id, label}]` JSON, search-filtered through
    the target's `ModelAdmin::search_fields()`. The label per row
    follows the same resolution ladder as the form view's
    `resolve_relation_options` (display field → `name` → `title` →
    `#<id>`). Identity-gated; uses the target model's `view`
    permission so the surface matches what the user can already
    see on the target's list page.
  - **URL convention**: a single `?<fk_col>=<id>` integer — same
    shape the runtime's `WHERE col::text = $N` already handles, so
    no SQL changes were needed.
  - **Active-pill hydration**: the handler resolves the chosen id
    to a display label via `AdminOps::object_label`, so the strip
    reads `"Author: Anna Lindqvist"` rather than `"Author: 42"`.
    A row that's been deleted falls back to `#<id>` — the page
    never 500s on a stale URL.
  - **JS-driven typeahead** (`initFkAutocomplete` in `admin.js`)
    debounces 250 ms, hits `/admin/_lookup/…`, renders an absolute-
    positioned results panel, and writes the chosen id into a
    hidden input on click. Without JS the visible search input is
    inert as a filter widget, but the hidden id input still carries
    any previously-selected value, so reloads round-trip
    correctly.
  - **New `infer_filters_with_registry`** in `admin::filters`
    extends the existing `infer_filters_with_relations` to consult
    the registry so the lookup URL can bake in the target's admin
    slug. `infer_filters_with_relations` keeps its existing role-
    based behaviour for any caller that already passes a closure.
  - Existing CSS file (`components/dropdowns.css`) gains the
    `.rio-fk-autocomplete*` block — no new fragment, no
    `admin.css` lock-step disturbance.
  Two unit tests pin the two notable inference edge cases (FK
  field without a resolvable target falls back to `NumericExact`;
  declared `choices` on an FK-shaped column still wins as
  `MultiSelect`).
- **`FilterKind::MultiSelect` — the third list-page filter widget.**
  Any `AdminField` that declares a non-empty `choices: &'static [&'static str]`
  slice now surfaces a checkbox list inside the Filters dropdown
  (`choices` was previously consulted only by the form view's
  `<select>`). URL convention is the natural HTML one — one repeated
  `?<col>=v` per checked option (`?status=active&status=pending`).
  `ListOpts` gains a `multi_filters: Vec<(String, Vec<String>)>`
  field; the runtime emits `WHERE col::text IN ($a, $b, …)` with one
  bound parameter per value. Hand-edited URL values that aren't in
  the field's declared `choices` are silently dropped — defense in
  depth on top of the column-name validation that every filter
  already does against `M::COLUMNS`. The combined active-filter
  pill reads `"State: active, pending"` with one remove link that
  drops every selection at once.
- **`FormData` learns multi-value semantics via `get_all`.** Forms
  and query strings with duplicate keys (`?status=active&status=pending`)
  now keep every submission in order via `FormData::get_all(key) -> &[String]`,
  while the existing `get` keeps last-write-wins semantics so legacy
  single-value callers stay correct unchanged. `set` resets the
  multi-value view in lock-step so the two views never disagree.
  Four parser tests cover the new behaviour.
- **`FilterKind::DateRange` is the first list-page widget to ship
  beyond `BoolYesNo`.** Any `DateTime` / `Option<DateTime<Utc>>`
  column on a registered model now surfaces a date-range filter
  inside the Filters dropdown — two `<input type="date">` controls
  and an Apply button. URL convention is Django-style:
  `?<col>__gte=YYYY-MM-DD&<col>__lte=YYYY-MM-DD`. Either bound is
  optional; an open-ended range applies only the side that's set.
  `ListOpts` gains a `date_ranges:
  Vec<(String, Option<String>, Option<String>)>` field
  (column-name validated against `M::COLUMNS`); the runtime emits
  `col::date >= $N::date` and / or `col::date <= $N::date` so both
  `DATE` and `TIMESTAMP` columns compare cleanly. Hand-edited URL
  values that don't parse as `YYYY-MM-DD` are silently dropped to
  the unbounded side — Postgres never sees garbage. The active
  range surfaces as one combined pill in the active-filter strip
  (`Created: 2026-01-01 → 2026-05-19`, `Created: ≥ 2026-01-01`, or
  `Created: ≤ 2026-05-19`) with one remove link that drops both
  bounds. CSS for the in-dropdown form lives in
  `components/dropdowns.css`; no new fragment, no `admin.css`
  lock-step disturbance. Four parser tests pin the empty / trim /
  garbage / unpadded paths.
- **`ModelAdmin::fieldsets()` is honoured on the change form.**
  The trait method has shipped since 0.2 but `form_ctx` always ran
  the `Default` / `System` / `Advanced` name heuristic regardless
  of what the project returned. A non-empty `fieldsets()` now
  replaces the heuristic entirely: each `Fieldset` renders as one
  titled section in declaration order, and the fields inside it
  render in the order the project listed them. Names with no
  matching `FormField` are silently dropped (a typo doesn't panic
  in production); fields that exist on the model but aren't
  referenced anywhere get appended to a trailing "Other" section so
  the form is never silently incomplete. Two unit tests pin the
  ordering and the typo-tolerance.
- **`ModelAdmin::readonly_fields()` is honoured on the change form.**
  The trait method has shipped since 0.2 but `form_ctx` filtered only
  on the macro's per-field `editable: false` flag, so any column the
  project listed in `readonly_fields()` rendered as a normal editable
  input. The renderer now consults `entry.readonly_fields` and emits
  the field with `disabled: true` (asterisk dropped, value pinned to
  the existing row). `do_update` re-injects those columns into the
  parsed `FormData` before handing it to `M::from_form`, so the
  generated parser still sees a complete row and the database write
  leaves readonly columns untouched. Applies to **edit only** — on
  the add form the listed fields stay editable so the project can
  supply the initial value (matches the common "set on create, never
  change after" usage). Documented on the trait method.
- **`FormData::set`** — small public helper used by the handler
  above to inject existing values for disabled-rendered fields.
  Single insert/overwrite semantics; mirrors `HashMap::insert`.

### Fixed

- **Vestigial horizontal scroll track at the bottom of every list card on desktop.**
  `layout/responsive.css` had `.rio-list, .rio-card { overflow-x: auto }`
  applied unconditionally — the comment claimed it was for narrow
  screens but the rule itself wasn't wrapped in a media query, and the
  file loads last so it overrode `.rio-list { overflow: hidden }` from
  `components/tables.css`. The rule rendered a scrollbar track at the
  bottom of every list card regardless of whether the table actually
  overflowed. Scoped to `@media (max-width: 767.98px)` so it applies
  only at the mobile breakpoint where it is actually load-bearing.

### Changed

- **List-page tables no longer overflow horizontally on 1440 px viewports.**
  Wide rows (≥ 6 columns with monospace identifier fields like VIN /
  ISO timestamp / reference code) used to push the trailing `Delete`
  action past the content area and force a horizontal scrollbar on
  the list card. Three small CSS adjustments in
  `components/tables.css`, scoped to `.rio-list .rio-table` so other
  consumers (`.rio-dl` profile show-grid, permissions matrix) are
  untouched: (1) middle-cell horizontal padding `var(--rio-s4)` →
  `var(--rio-s3)`; (2) first/last gutter `var(--rio-s5)` →
  `var(--rio-s4)`; (3) action-cell inter-button gap `var(--rio-s2)`
  → `var(--rio-s1)`. List rows breathe a touch tighter — still
  spacious by Bloomberg / Stripe / Linear standards.
- **List-page headers render with real word boundaries.** The derive
  macro now emits `AdminField.label` as the humanised label
  (`"Performed by technician"`) instead of the raw snake-case
  identifier (`"performed_by_technician"`). CSS uppercase + tracking
  still applies — the header reads `PERFORMED BY TECHNICIAN` — but
  the wrap behaviour and the column-width floor are governed by
  individual words rather than one unbreakable underscore-joined
  token, so headers can wrap on narrow rows. List-page header rule
  in `components/tables.css` also gains `white-space: normal` to
  let that wrap actually happen. Validation messages still use the
  same humanised label.
- **No public API change** — `AdminField.label` was already typed
  `&'static str`. The literal emitted by the macro is now the
  humanised form rather than the raw fname.


## [0.15.1] — 2026-05-16

Refined colour palette — the v0.15.0 ladder was harmonious but the
chrome tier sat between card and canvas (a *subtle* frame). External
feedback was that pages still felt white-heavy and lacked
"prestige." This patch makes chrome go **dark in light mode**
(the Linear / Vercel / Notion / Stripe-Dashboard convention) and
even darker than canvas in dark mode, so the operator skeleton
reads as a confident frame in both directions. Page canvas is also
retuned away from a blue-tinted slate toward a more neutral cool
grey that pairs cleanly with the teal accent.

Pure colour change — no token added or removed, no public API
touched, no template HTML edits, no contract change. Projects with
custom `AdminTheme` overrides keep working unchanged.

### Tokens — light mode

| Token | v0.15.0 | v0.15.1 | Why |
|---|---|---|---|
| `--rio-bg` | `#E4E8EE` | `#E5E7EB` | Page canvas drops its blue tint — more neutral cool grey |
| `--rio-surface` | `#FAFBFC` | `#F9FAFB` | Card surface a hair cooler to harmonise with the new canvas |
| `--rio-surface-2` | `#EFF2F6` | `#EEF0F3` | Table header tracks the card |
| `--rio-surface-3` | `#E5E9EE` | `#E3E6EA` | Row hover deepens slightly |
| `--rio-surface-chrome` | `#DCE0E7` | `#1F2A37` | **Topbar / sidebar / footer** — deep slate-blue, premium dashboard convention |
| `--rio-surface-elevated` | `#FFFFFF` | `#FFFFFF` | Overlay surface unchanged |

### Tokens — dark mode

| Token | v0.15.0 | v0.15.1 | Why |
|---|---|---|---|
| `--rio-bg` | `#1A1F28` | `#131820` | Deeper canvas — needed because chrome moves to near-black |
| `--rio-surface` | `#262C36` | `#1F262F` | Card stays clearly above canvas |
| `--rio-surface-2` | `#2E3540` | `#262E39` | |
| `--rio-surface-3` | `#363D49` | `#2E3742` | |
| `--rio-surface-chrome` | `#1E232C` | `#0A0E14` | **Near-black chrome** — same "darker than canvas" direction as light mode |
| `--rio-surface-elevated` | `#363D49` | `#2E3742` | Slightly lower so chrome stays the deepest tier |

### Chrome-scope CSS cascade — new in `layout/shell.css`

The single architectural move that makes the dark-chrome direction
trivially adoptable. A small CSS block redefines the relevant
custom properties locally for `.rio-topbar`, `.rio-sidebar`, and
`.rio-footer`:

```css
.rio-topbar,
.rio-sidebar,
.rio-footer {
  --rio-text-strong: #F5F6F8;
  --rio-text:        #C9CFD8;
  --rio-text-muted:  #8B919C;
  --rio-text-subtle: #6E737D;
  --rio-surface-2:   #2A3441;
  --rio-surface-3:   #364150;
  --rio-border-soft: #2A3441;
  --rio-border:      #364150;
  --rio-border-strong: #4A5364;
  --rio-accent:      #3FAA9D;
  --rio-accent-rgb:  63 170 157;
}
```

CSS custom properties cascade through descendants. Every component
inside chrome that consumes these tokens (sidebar links, hover
states, dividers, active-nav wash, theme toggle…) automatically
flips to a light-on-dark palette without per-component edits. The
accent is lifted to `#3FAA9D` within chrome scope so the active
sidebar item retains crisp ≥ 5:1 contrast against the deep slate
— same logic the dark theme has used since v0.4.0.

### Component refinements

- **Theme-toggle button** redesigned as a transparent ghost on
  chrome. The v0.15.0 "raised light card inside dark chrome"
  fought the dark-frame aesthetic; ghost + chrome-cascade
  borders/text reads as native.
- **Active sidebar link** wash inherits the lifted accent
  automatically, so the `is-active` row pops more than under
  v0.15.0.

### Doctrine

`docs/DESIGN_DOCTRINE.md` Principle 10 reframed: chrome direction
is a project-aesthetic choice. The framework default went from
subtle frame (v0.15.0) to dark frame (v0.15.1); both satisfy the
"chrome is visually distinct from canvas and card" rule. When the
chrome direction goes dark in light mode, the `layout/shell.css`
chrome-scope cascade handles every descendant in one place.

### Upgrade

```toml
[dependencies]
rustio-admin = "0.15.1"
```

Drop-in. No code changes, no `AdminTheme` adjustment required.
Projects that overrode `bg` / `surface` / `text` / `text-muted` /
`border` via `AdminTheme` may want to revisit those values against
the new defaults — the framework now ships closer to what most
projects were tuning toward.


## [0.15.0] — 2026-05-16

Visual identity overhaul — **calm with authority**. The framework's
visual character has shifted from operator-bland (too much white,
shallow surface ladder, transparent chrome) to operator-confident
(Stripe / Linear / Bloomberg "professional financial software"
aesthetic) without crossing into flashy territory the doctrine
forbids. Pure CSS — no template HTML changes, no public API
changes, no `AdminTheme` contract change. Existing `AdminTheme`
overrides keep working unchanged; values that landed close to the
new defaults (e.g. obddesk's text overrides) become redundant.

The full design rationale, principles, and landing plan live in
[`docs/archive/PLAN_VISUAL_v2.md`](docs/archive/PLAN_VISUAL_v2.md).

### Three new doctrine principles

Added to [`docs/design/DESIGN_DOCTRINE.md`](docs/design/DESIGN_DOCTRINE.md) §6:

- **Principle 9 — Deeper surface ladder.** Adjacent surfaces are
  ≥ 4% apart so the eye never squints to tell canvas from card
  from table-header from row-hover.
- **Principle 10 — Chrome carries weight.** Topbar and sidebar
  render on a distinct chrome tier (`--rio-surface-chrome`),
  deeper than card surface and lighter than page canvas.
- **Principle 11 — Typography hierarchy is a weight choice, not
  just a size.** Display sizes declare gravity through weight
  700–800 *and* deliberate tracking. Body and table cells stay at
  400 for ten-hour-shift legibility.

### Tokens

- **Surface scale** expanded from four rungs to six. Steps now
  ≥ 4% apart (except surface → surface-2 which is a deliberately
  quieter 3% so the table header reads subtle, not pronounced):

  | Token | Light | Dark | Use |
  |---|---|---|---|
  | `--rio-bg` | `#E4E8EE` | `#1A1F28` | Page canvas |
  | `--rio-surface` | `#FAFBFC` | `#262C36` | Cards, panels, table body |
  | `--rio-surface-2` | `#EFF2F6` | `#2E3540` | Table head, hovered row |
  | `--rio-surface-3` | `#E5E9EE` | `#363D49` | Row hover, pressed state |
  | `--rio-surface-chrome` **(new)** | `#DCE0E7` | `#1E232C` | Topbar, sidebar, footer |
  | `--rio-surface-elevated` **(new)** | `#FFFFFF` | `#363D49` | Dropdowns, popovers — the only pure white |

- **Text scale** moved one stop darker across the board:

  | Token | Light | Dark |
  |---|---|---|
  | `--rio-text-strong` | `#0A0D11` (was `#111827`) | `#FFFFFF` |
  | `--rio-text` | `#14181D` (was `#1F2937`) | `#D8DBE0` |
  | `--rio-text-muted` | `#3B4148` (was `#4B5563`) | `#9097A0` |
  | `--rio-text-subtle` | `#5C656F` (was `#6B7280`) | `#6B7280` |

  All combinations against every surface tier still clear WCAG AA.

- **Border scale** firmed by one step:

  | Token | Light | Dark |
  |---|---|---|
  | `--rio-border-soft` | `#D8DDE4` (was `#ECEFF4`) | `#3A4252` |
  | `--rio-border` | `#C9CFD7` (was `#DEE3EC`) | `#4A5364` |
  | `--rio-border-strong` | `#A8B0BA` (was `#C5CCD9`) | `#5E6878` |

- **Shadow scale** converted to two-layer construction
  (near-shadow + far-shadow, Stripe / Linear style) with bumped
  alphas so cards actually rise off the new deeper canvas. New
  `--rio-shadow-inset` used by inputs and pressed states.

- **Typography tokens** add `--rio-fw-heavy: 800` (Geist Variable
  supports it; no new `@font-face` declarations),
  `--rio-tracking-allcaps: 0.06em`, and `--rio-tracking-mono: 0`.
  Display tracking tightened to `-0.035em` (was `-0.022em`),
  heading tracking to `-0.018em` (was `-0.012em`).

- **Spacing token**: `--rio-topbar-h` 64 → 72 px so chrome carries
  presence.

- **Accent unchanged**. `#0F8C7E` light / `#3FAA9D` dark — the
  project's brand colour is a stable invariant per principle 4.

### Components

- **Buttons** gain a Stripe / Linear-style vertical gradient on
  primary (≈ 8% delta from accent-top to accent-bottom; invisible
  at a glance, deliberate on inspection), an inset top highlight,
  accent-coloured drop shadow, and a proper `:focus-visible` ring
  for keyboard navigation. Fixed a stale crimson rgba (pre-0.3
  artefact) on the primary shadow.

- **Inputs / textareas / selects** ship with `--rio-shadow-inset`
  by default — the single biggest UX delta in the overhaul.
  Fields now read as "place to type" rather than "drawn
  rectangle." Focus state stacks the inset with the accent ring
  (bumped from 12% to 22% alpha).

- **Tables**: header weight stepped from 700 to 600 (refined,
  not shouty); hardcoded `letter-spacing` swapped for
  `--rio-tracking-allcaps` token. **Primary-cell weight added** —
  the first data cell in each row (the natural key: `code`,
  `vin`, `email`, …) gets `font-weight: 500` + `color:
  text-strong` so the eye has a column to skim against. Works on
  tables with and without bulk-select checkboxes.

### Chrome

- **Topbar** background → `--rio-surface-chrome`; height bumped
  to 72 px. Theme-toggle button sits raised on `--rio-surface`
  with `--rio-shadow-xs` so the interactive affordance reads
  distinctly against the surrounding chrome strip.

- **Sidebar** background → `--rio-surface-chrome`. Section labels
  (e.g. "AUTHENTICATION") moved to 11 px semibold tracked-allcaps
  — quieter line height, more visual rhythm against long nav.
  Hover lifts to `--rio-surface-2`; active link wash bumped from
  10% to 12% accent alpha; 3 px inset stripe unchanged.

- **Footer** background → `--rio-surface-chrome` matching topbar
  and sidebar so the operator skeleton closes top + sides +
  bottom in one tone.

### Migration impact for downstream projects

- Pure aesthetic upgrade — no `cargo update` action required
  beyond bumping `rustio-admin = "0.15.0"`. The next page load
  shows the new visual character.
- Projects with custom `AdminTheme` overrides keep working
  unchanged. Values that landed close to the new defaults can be
  dropped:
  - `text: "#0F1115"` → essentially the new default `#14181D`
  - `text_muted: "#3B4148"` → identical to the new default
  - `border: "#C9CFD5"` → essentially the new default `#C9CFD7`
- `obddesk` and other consumers should drop their
  `AdminTheme { text, text_muted, border, … }` block from
  `main.rs`; the new framework defaults already deliver the
  intended contrast.

### Hard non-goals respected

- No new font family. Geist Variable already supports weight 800.
- No accent change. `#0F8C7E` stays.
- No new JS. Every change is CSS-only.
- No template HTML changes — the redesign rides on the existing
  class structure. Zero risk of breaking generated projects.
- No new build step. Hand-written CSS, baked via `include_str!`
  per `DESIGN_DOCTRINE.md` §8.
- No marketing surfaces. No hero gradients, no shimmer, no
  illustration. The `DESIGN_DOCTRINE.md` §6.7 rule stands.


## [0.14.1] — 2026-05-16

A single-bug patch surfaced by the first external project to adopt
the Builder MVP (`obddesk`, an OBD-II diagnostics admin) with
realistic-length field names.

### Fixed

- **Builder migration codegen produced malformed SQL for long
  field names.** v0.14.0 emitted CREATE TABLE rows using a
  hardcoded `{:<12}` format width on the column-name slot. Field
  names ≥ 12 characters consumed the entire padded slot, leaving
  zero separator between the identifier and the next token —
  Postgres rejected the resulting `engine_displacement_ccBIGINT`
  as a syntax error before reaching the type lexer.

  Fixed in
  [`crates/rustio-admin-cli/src/builder/codegen.rs`](crates/rustio-admin-cli/src/builder/codegen.rs)
  by computing the name-column width dynamically per table as
  `max(longest_field_name, 10)` and inserting two literal spaces
  between the identifier and type columns. Implicit `id` and
  `created_at` columns are routed through the same dynamic format
  so every row in a `CREATE TABLE` aligns internally.

  Visible behaviour: short-field tables (longest name ≤ 10 chars)
  shift one space in the type column compared to v0.14.0; long-
  field tables now produce syntactically valid SQL. No SchemaHash
  input change — projects regenerate cleanly on the next
  `rustio commit`.

  New regression test:
  `codegen::tests::long_field_names_keep_separator_from_type_column`
  exercises a Vehicle model with `vin` (3 chars) + `engine_displacement_cc`
  (22 chars) and asserts the forbidden substring
  `engine_displacement_ccBIGINT` does not appear in the emission.
  The pre-existing `initial_migration_creates_each_table` was also
  rewritten to use layout-agnostic assertions (no positional
  whitespace check) so future format tweaks don't false-alarm.

### Upgrade

Pure patch — bump `rustio-admin = "0.14.1"` and re-run
`rustio commit`. Existing migrations on disk that already applied
their CREATE TABLE statements are unaffected; only future
emissions change. The doctrine, public API, MSRV (1.88), and all
13 numbered Builder invariants are unchanged from v0.14.0.


## [0.14.0] — 2026-05-15

First release of the **Builder MVP** — a deterministic project
compiler that emits Rust admin projects from a declarative
`.rustio/draft.toml`. The Framework runtime is unchanged in this
release; everything new lives under
`crates/rustio-admin-cli/src/builder/` and the new design
documents under `docs/design/`.

Companion documents:
[`docs/design/DESIGN_BUILDER.md`](docs/design/DESIGN_BUILDER.md)
(authoritative doctrine, 13 numbered invariants),
[`docs/archive/REVIEW_BUILDER_DOCTRINE.md`](docs/archive/REVIEW_BUILDER_DOCTRINE.md)
(defensive review tracking resolved + residual findings),
[`docs/archive/VALIDATION_BUILDER_MVP.md`](docs/archive/VALIDATION_BUILDER_MVP.md)
(end-to-end smoke + generated artefacts + reproduction commands).

### Builder MVP — new tooling layer

- `rustio new <name>` scaffolds a Builder-managed project with
  `Cargo.toml`, `src/main.rs`, `migrations/`, and `.rustio/{draft.toml,
  history.jsonl, builder.lock}`.
- `rustio add model <Name>` records an `add_model` event with
  plural_snake table-name inference matching the proc-macro's
  rule set.
- `rustio add field <Model> <name> <type> [--unique]` records an
  `add_field` event. Field types are the closed list `text`,
  `integer`, `boolean`, `timestamp`. Modifiers limited to
  `required` (implicit, always true in MVP) and `unique`.
- `rustio plan` prints a structured diff of what `commit` would
  apply. Read-only — Doctrine B8 forbids filesystem side effects.
- `rustio commit [--force]` applies the plan atomically via
  `.rustio/tmp/<txn>/` staging. Idempotent: a second `commit`
  with no intervening changes is a true no-op (§6.4).
- `rustio commit --force` quarantines prior content to
  `.rustio/forced/<ULID>/<path>` before overwriting (§5.4 case 3).
- `rustio startproject` (legacy template scaffolder) coexists
  untouched. Adoption of the Builder is opt-in per project.

### Implementation-grade Builder doctrine

13 numbered invariants binding the Builder layer:

| | |
|---|---|
| **B1** | `.rustio/draft.toml` is the sole input to the deterministic generator |
| **B2** | `.rustio/history.jsonl` is append-only; reversal is a new compensating event |
| **B3** | `cli::history::append` is the sole writer of `.rustio/history.jsonl` (analog of Doctrine 22) |
| **B4** | `cli::redact` produces fingerprints, not values, for every secret-category field (analog of Doctrine 11) |
| **B5** | Every file under `src/_generated/` carries a header + SchemaHash; overwrite requires `--force` + event |
| **B6** | Migrations are append-only — no command edits an existing migration |
| **B7** | Doctrine-bound features cannot be disabled via `draft.toml` (audit, auth, csrf, correlation_id, redaction, single-writer invalidation) |
| **B8** | `rustio plan` has zero filesystem side effects; `rustio commit` is atomic |
| **B9** | `rustio plan` and `rustio commit` open no network sockets |
| **B10** | The Framework crate never reads Builder-emitted metadata at runtime |
| **B11** | The Builder version is pinned in `.rustio/builder.lock`; mismatch refuses to run |
| **B12** | Model / field renames performed only with a `rename_*` event; otherwise classified destructive |
| **B13** | Advisory output is never source of truth (boundary contract only in this release) |

### Deterministic project compiler foundation

Foundational primitives proving the doctrine load-bearing:

- **Canonical TOML emitter** (`builder::toml_canon`) — sole emitter,
  alphabetical key sort within tables, LF + UTF-8 NFC + no
  trailing whitespace, byte-stable across runs.
- **Append-only event log** (`builder::history`) — `cli::history::append`
  is the only function authorised to write `history.jsonl`. Lines
  fixed-shape per §4.2.1; 4 KB atomic-write bound enforced.
- **SchemaHash projections** (`builder::hash`) — distinct closed
  projections per generated file (`models/<name>.rs`, `admin.rs`,
  `mod.rs`, `migrations/0001_initial.sql`). Header text is NOT in
  the projection — preserves Doctrine B10.
- **Replay invariant** (`builder::replay`) — events replay against
  an empty draft to reconstruct the current `draft.toml`
  byte-for-byte (§4.2.2).
- **ULID identifiers** (`builder::ulid_gen`) — Crockford base32,
  monotonic within a process, second-precision timestamps in
  events.
- **Redactor** (`builder::redact`) — closed secret-category list
  (`password`, `secret`, `token`, `api_key`, `private_key`,
  `encryption_key`); property test asserts no 4-char input
  substring leaks (symmetric to `DESIGN_AUDIT.md` §5.3).

### CI-enforced doctrine invariants

Five new grep proofs in `.github/workflows/ci.yml`. Each must
produce no output; any match fails the build:

- **B10** — Framework reads no Builder header markers.
- **B3** — only `builder/history.rs` writes `history.jsonl`.
- **B1** — only `builder/toml_canon.rs` emits TOML.
- **B4** — only `builder/redact.rs` defines a Builder-side redactor.
- **§5.6** — no Builder reference to `src/app/`.

These are non-negotiable PR gates from this release onward.

### Validation + hardening

Pre-push defensive review surfaced 38 findings; 6 actionable
issues fixed in a single hardening commit before the release tag:

- Actor degradation no longer silent (`unknown` actor now emits
  stderr warning).
- `find_project_root` no longer follows symlinks; refuses
  symlinked `.rustio/` (new `LifecycleError::SymlinkedRustioDir`).
- `parse_header_hash` strict canonical-prefix match; sha256-hex
  format check; rejects nested doc-comment prefixes (`//!`, `///`).
- Secret-category field types refused at `rustio add field`
  (forward-defensive tripwire for the future `--default` path).
- Table-name uniqueness validated in both `Draft::from_toml` and
  `replay::apply_add_model` (catches `Box` + `Boxes` →
  `boxes` collision).
- `history.jsonl` atomicity model documented + 4 KB line refusal
  added (`MAX_ATOMIC_LINE_BYTES`).

`docs/design/VALIDATION_BUILDER_MVP.md` records the manual smoke
test, doctrine ↔ test map, refusal-path coverage, and
reproduction commands.

### Infrastructure

- MSRV bumped from **Rust 1.80** to **Rust 1.88**. Transitive
  deps now require it — `home@0.5.12` (via `cargo` → `tame-index`
  → `clap_lex 1.1.0`) requires 1.88; the ICU 2.2.x cluster
  requires 1.86; `clap_lex 1.1.0` uses the `edition2024` Cargo
  feature stabilised in 1.85. Declaring 1.80 was no longer
  achievable from a fresh build.
  `workspace.package.rust-version` and the CI pin
  (`dtolnay/rust-toolchain@1.88`) now match.
- `actions/checkout` bumped v4 → v6 (Node.js 24 runtime; silences
  the Node 20 deprecation annotation).
- `dtolnay/rust-toolchain` pinned to `@1.88` (was rolling `@stable`).
  CI now enforces the declared MSRV.
- 20 sites updated for `clippy::uninlined_format_args` (lint
  defaults differ between rustc 1.88 and 1.93).
- `clippy::doc_overindented_list_items` fixes in the
  library-circulation example.

### Visibility recovery

- `auth::recovery_admin`, `auth::mfa`, `auth::sessions::hash_token_for_storage`,
  `auth::recovery::MailerEmailStatus`, and `MfaKey::from_bytes`
  re-promoted to `pub` so the doc-hidden, feature-gated
  `crate::__integration` test door can re-export them. The
  parent modules stay `pub(crate)`, so the external API surface
  is unchanged — `__integration` is still the only door for
  `tests/integration_*.rs`.

### Intentionally out of scope for v0.14.0

The MVP exists to prove the doctrine load-bearing. The
following are intentionally **not** part of this release; each is
acknowledged in `REVIEW_BUILDER_DOCTRINE.md` §4 and
`VALIDATION_BUILDER_MVP.md` §8:

- **Studio** (web wizard) — future `DESIGN_STUDIO.md`.
- **Advisory AI** (`rustio suggest` / `review`) — boundary
  contract is in `DESIGN_BUILDER.md` §9.3; internal contract
  pending in a future `DESIGN_ADVICE.md`. No LLM call sites
  exist in this release.
- **Incremental migrations.** Only the first migration
  (`0001_initial.sql`) is supported. Subsequent `commit`
  invocations against a diverged draft are refused with a
  doctrine-cited error. The four primitives required for safe
  incremental support — diff computation, destructive-op
  classification, rename detection (Doctrine B12), and rollback-
  hint generation — are spelled out in
  `VALIDATION_BUILDER_MVP.md` §5; a `DESIGN_BUILDER.md`
  amendment is required before this lands.
- **`rustio import postgres://`** — future v0.17 milestone.
- **`rustio undo` / `redo`** — event-log replay primitive exists
  but no verb wiring.
- **Theme / branding CLI** — future v0.18 milestone.
- **`[features]` table in `draft.toml`** — doctrine-bound
  features (§8.2) are always-on; toggleable features deferred
  until first toggle is needed.
- **Field relations** (`belongs_to`, `has_many`, `many_to_many`).
  MVP type system covers only scalars.
- **Optional / nullable fields.** All fields are `NOT NULL`.
- **Rename verbs.** Requires `rename_model` / `rename_field`
  events in `HistoryOp` first.
- **`schema_version` migration of `draft.toml` itself.**
  Schema version 1 is the only supported value.
- **`rustio doctor builder`** — drift tests cover the
  implementation-level checks; verb-level operator command
  deferred.
- **`rustio merge`** for git-merge reconciliation of
  `history.jsonl`.
- **Per-file `forced_overwrite` child events.** Parent `commit`
  event records the operation in MVP.

### Upgrade notes

Bump `rustio-admin = "0.14.0"` and Rust toolchain to `≥ 1.88`.
No runtime behaviour changes; existing projects compile
unchanged.

To start using the Builder on an existing project: not yet
supported in v0.14.0 — the MVP scaffolds new projects from
empty state. An import path lands in a future release.


## [0.13.0] — 2026-05-13

Minor release pairing two threads that both prepare the framework
for the 1.0 commitment surface:

  1. **Phase G — privatisation pass.** Every item annotated
     `// internal:` since the 0.9.0 annotation pass (419 items
     tagged `// public:` or `// internal:`, of which 64 were
     internal) has had its visibility narrowed from `pub` to
     `pub(crate)`. **Net effect**: 59 items removed from the
     external surface (~17% reduction). 5 markers stay `pub`
     because they decorate cross-crate test re-exports in
     `lib.rs` gated behind `#[doc(hidden)]` +
     `cfg(feature = "integration-test")`.

  2. **`DESIGN_EMAIL.md` doctrine.** Codifies the
     framework-emitted email conventions stabilised in 0.12.0
     (recovery flow shipped to production against real Gmail).
     Covers plaintext-first MIME shape, app-identity vs framework-
     identity separation, the unconditional security envelope,
     the anti-phishing warning panel, verification-reference
     derivation from `correlation_id`, greeting + signature
     fallback chain, subject-line vocabulary for the five known
     email types, and the hard refusals (no tracking pixels, no
     external `<img>`, no JS, no CDN assets).

**No schema migration. No runtime behaviour change.** Phase G is
a static visibility narrowing; tests + the example
(`library-circulation`) continue to compile and pass against the
in-crate items they always used.

### Migration from 0.12.0

Bump `rustio-admin = "0.13.0"` and run
`cargo update -p rustio-admin`.

For most projects, no source change required. If a project was
reaching into the framework's MFA crypto primitives or R2 admin-
recovery internals (against the `// internal:` annotations'
intent), the compiler will surface the now-`pub(crate)` paths as
"private" — switch to the public R3/R4 surface in
`auth::emergency` and `auth::mfa::policy`, which is what those
items were designed to expose.

### Changed

- **`auth::mfa` privatisation** (20 items):
  - `MfaKey` struct, `MfaKey::from_bytes`, `MfaKey` env loader
  - `wrap_secret`, `unwrap_secret` — AES-256-GCM primitives
  - `generate_totp`, `current_step`, `verify_totp`
  - `generate_backup_codes`, `normalise_backup_code`,
    `hash_backup_code`, `verify_backup_code`
  - `BACKUP_CODE_COUNT`, `BACKUP_CODE_LEN` constants
  - `confirm_enrolment`, `verify_totp_for_user`,
    `consume_backup_code`, `disable_mfa`,
    `regenerate_backup_codes`,
    `promote_session_to_mfa_verified`
  - `migrate_user_mfa_schema`
  All are framework-internal implementations. The public R3 MFA
  policy + admin-driven disable surface lives unchanged in
  `auth::mfa::policy` and `auth::emergency`.

- **`auth::recovery_admin` privatisation** (21 items): R2 admin-
  driven recovery internals — `LockState`, `ThrottleOutcome`,
  `AdminActor`, `AdminIssueOutcome`, `AdminTempPwOutcome`,
  `LockDuration`, `LockOutcome`, `UnlockOutcome`,
  `AdminRevokeOutcome`, plus their associated `pub fn`
  helpers (`record_failed_login`, `record_successful_login`,
  `check_account_lockout`, `lock_user_account`, ...). The public
  R2 admin-driven recovery surface (admin-reset, admin-unlock,
  re-auth wall) is exposed via `auth::emergency`'s public
  functions, which remain `pub`.

- **Handler-internal constructors** (4 items):
  `admin::handlers::AdminCtx::new`,
  `admin::render::BaseContext::new`,
  `admin::recovery_handlers::RecoveryState::new`,
  `admin::icons::render`. These never made sense as public API —
  callers construct via `Admin::new()` builders, which then own
  the contexts.

- **Minor surfaces** (3 items):
  `http::__integration_test_fake` (gated to test feature only),
  `auth::recovery::MailerEmailStatus`,
  `admin::types::AdminTheme::has_overrides`.

### Added

- **`docs/design/DESIGN_EMAIL.md`** (~540 lines) — doctrine
  document for framework-emitted emails. Indexed in
  `docs/README.md` under "Design specs."

### Internal

- All 271 lib unit tests + 1 cascade-lockstep test + workspace
  tests pass unchanged. Per-crate compile time effectively
  identical (privatisation has no monomorphisation impact).
- `docs/public-api.md` requires no updates — Phase C's
  enumeration of 355 public items correctly excluded the 64
  `// internal:` annotated items, so no entries in the
  public-API list reference now-privatised symbols. Verified
  via a structural audit (`grep -wf <privatised-names>`).
- Workspace + CLI bumped to 0.13.0. "Releases at a glance"
  gains a 0.13.0 row.

### Public surface

| Metric | 0.12.0 | 0.13.0 | Delta |
|---|---:|---:|---:|
| `pub` items in framework crate (annotated as `public:`) | 355 | 355 | 0 |
| `pub` items annotated `internal:` (leaked surface) | 59 | 0 | **-59** |
| Cross-crate test re-exports (gated, doc-hidden) | 5 | 5 | 0 |
| **Effective external API surface** | **419** | **360** | **-14%** |

This is the largest pre-1.0 surface narrowing in the project's
history. Future refactors of MFA crypto + R2 internals are now
non-breaking — those signatures are crate-internal and free to
move.


## [0.12.0] — 2026-05-13

Three substantial threads ship together:

  1. **Public bulk-action dispatch hook** — projects can now
     back the buttons that `ModelAdmin::bulk_actions()` already
     declared. Closes the second of the two D.4-documented
     framework gaps. `AdminOps` stays `pub(crate)`; the public
     surface is a single new default method on `ModelAdmin`.
  2. **Production password-recovery flow** — real SMTP transport
     via project-side `lettre` integration, polished HTML email
     body, brand-identity architecture so the framework name no
     longer leaks into user-facing surfaces. End-to-end verified
     against real Gmail delivery to a real inbox.
  3. **Operator DX** — `rustio doctor email` subcommand with
     provider presets (Gmail / Resend / Postmark / Mailgun /
     SendGrid / Ethereal), `--html-preview` mode, accidental-
     send cooldown, and a formal `.env` developer contract.
     Boot-time SMTP smoke test refuses to start the app silently
     against misconfigured SMTP.

**No schema migration that requires manual intervention** — the
new `rustio_users` profile columns (`first_name`, `last_name`,
`display_name`, `job_title`) are added by an idempotent
`ALTER TABLE … ADD COLUMN IF NOT EXISTS` on framework boot.

### Migration from 0.11.0

Bump `rustio-admin = "0.12.0"` and run
`cargo update -p rustio-admin`.

If your project consumes the `--rio-font-arabic` token (0.11.0)
nothing changes. New work for 0.12.0 is opt-in:

  - **Use the new branding API.** Replace
    `Admin::new().site_branding(SiteBranding { site_header:
    "X".into(), ... })` with the canonical builders:
    ```rust
    Admin::new()
        .app_name("My Product")
        .app_tagline("Operational management")
        .support_email("support@example.com")
        .public_url("https://admin.example.com")
    ```
    Legacy `site_branding(...)` still works for backwards compat;
    the defaults for `site_title` / `site_header` were renamed
    from "RustIO administration" to "Admin" so a zero-config
    build no longer leaks the framework name.

  - **Wire a real SMTP transport.** Copy
    `examples/library-circulation/src/mailer.rs` into your
    project (or fork the example) and install via
    `Admin::mailer(Arc::new(LettreSmtpMailer::new(cfg)?))`.
    Configure with `MAIL_PROVIDER=gmail|resend|postmark|...`
    plus `SMTP_USER` + `SMTP_PASSWORD`, or set the explicit
    `SMTP_HOST` / `SMTP_PORT` / `SMTP_TLS` fields. Without
    SMTP configured the framework falls back to `LogMailer`
    (writes to stdout, no delivery).

  - **Validate before booting** via `rustio doctor email`.
    Surfaces wrong App Password / 2FA-off / wrong TLS-port
    combo / blocked egress in < 2 seconds.

### Added

#### Bulk-action dispatch

- **Public `ModelAdmin::execute_bulk_action` hook.** The
  dispatch contract is a single new default method on the
  public `ModelAdmin` trait:
  ```rust
  fn execute_bulk_action<'a>(
      action: &'a str,
      ids: &'a [i64],
      db: &'a Db,
      ctx: &'a BulkActionContext<'a>,
  ) -> Pin<Box<dyn Future<Output = Result<BulkActionResult>> + Send + 'a>>
  ```
  Default returns `BadRequest` with the action name so a
  declared-but-unimplemented action surfaces clearly. The
  internal type-erased `AdminOps` trait remains `pub(crate)`;
  `ConcreteOps<M>::execute_bulk_action` forwards into the
  model's override.
- **New module `rustio_admin::admin::bulk`** exporting three
  public types, all `#[non_exhaustive]` for SemVer headroom:
  `BulkActionContext<'a>`, `BulkActionResult`,
  `BulkActionFailure`. Re-exported at the crate root.
- Handler emits one audit row per bulk submission with
  structured `metadata` (`{kind, action, model, ids,
  succeeded, failed_ids, failure_reasons}`).
- `examples/library-circulation/`'s `Loan` model demonstrates
  the hook with `mark_overdue` + `mark_returned` actions.

#### Recovery / email infrastructure

- **`Mail::with_html(html) -> Self`** chainable builder.
  Mailer transports send both as `multipart/alternative`.
- **`render_recovery_html(RecoveryEmailParts)`** — framework's
  polished HTML body for recovery emails. Calm typography,
  hairline separation, single brand-accent CTA, mobile
  `@media` query, inlined CSS for client compatibility.
  Honours the project's `app_name` / `app_tagline` /
  `support_email`.
- **`RecoveryEmailParts::new(app_name, title, greeting_name,
  intro, cta_url, fine_print, when) -> Self`** constructor
  for external-crate callers (works around `#[non_exhaustive]`).
- **Profile columns on `rustio_users`**: `first_name`,
  `last_name`, `display_name`, `job_title` (idempotent
  migration). `StoredUser::greeting_name()` resolves
  `display_name → first_name → email-local-part → "there"`.
  `StoredUser::signature_lines()` returns
  `(primary, optional_title)` for the email signature block.
- `rustio user create` grows `--first-name`,
  `--last-name`, `--display-name`, `--job-title` flags.

#### Branding architecture

- **`SiteBranding` extended with the user-facing identity
  layer**: `app_name` (primary product identity),
  `app_tagline` (optional descriptor), `support_email`,
  `public_url`, `show_powered_by` (opt-in framework credit;
  default `false`).
- **`Admin` builders**: `.app_name(...)`, `.app_tagline(...)`,
  `.support_email(...)`, `.public_url(...)`,
  `.show_powered_by(...)`. `app_name()` also mirrors into the
  legacy `site_title` / `site_header` fields so old code paths
  stay coherent.
- **`BaseContext`** gains `app_name`, `app_tagline`,
  `show_powered_by` fields. Chrome footer's hard-coded "RustIO
  Admin" string replaced with `{{ app_name }}`; "Powered by
  RustIO" credit is now opt-in via `show_powered_by`.

#### `rustio doctor email`

- **New subcommand** with structured ✓ / ⚠ / ✗ output.
  Validates env-var presence, opens TLS handshake, performs
  EHLO + AUTH, and optionally sends a test message via
  `--to <address>`. No credentials echoed (`SMTP_PASSWORD`
  reported as `(set, N chars)`).
- **Provider presets**: `MAIL_PROVIDER=<key>` auto-fills
  host / port / TLS / default user from a shared table.
  Known: `gmail`, `resend`, `postmark`, `mailgun`,
  `sendgrid`, `ethereal`. Both the CLI doctor and the
  example's `smtp_config_from_env()` honour the same table.
  Explicit `SMTP_*` env vars always override.
- **`--html-preview`** flag renders the recovery email body
  with realistic placeholder data, writes to
  `/tmp/rustio-email-preview.html`, opens it in the
  default browser. No SMTP traffic. Reads `APP_NAME` /
  `SUPPORT_EMAIL` / `MAIL_FOOTER_TEXT` from env.
- **30-second cooldown** on `--to <addr>` sends (stamp at
  `/tmp/rustio-doctor-email-last-send`). Prevents
  accidental-spam loops.

#### Developer contract

- **`.env` excluded from git** via `.gitignore` (`.env`,
  `.env.*`, allowlist `.env.example`). Real credentials
  cannot accidentally be committed.
- **`.env.example` rewritten as the canonical developer
  contract**: `APP_NAME`, `APP_ENV`, `DATABASE_URL`,
  `RUSTIO_SECRET_KEY`, `MAIL_PROVIDER` + `SMTP_*`,
  `SUPPORT_EMAIL`, `RUSTIO_ENV`. Inline Google App Password
  setup steps + Style A (preset) vs Style B (explicit)
  framing.
- **`dotenvy::dotenv()` auto-load** in the
  `library-circulation` example's `main.rs` — no
  `set -a; source .env; set +a` shell ritual.
- **Boot-time SMTP smoke test** via
  `LettreSmtpMailer::smoke_test()` (TCP → TLS → EHLO →
  AUTH → QUIT). Refuses to boot when `SMTP_HOST` is set
  but credentials fail; prints a structured diagnostic.

### Changed

- **`audit::record` accepts `object_id == 0`** as the
  bulk-dispatch row shape (`metadata.kind == "bulk_action"`;
  affected ids in `metadata.ids`). Negative values still
  rejected. Per-object emissions (`object_id > 0`) unaffected.
- **`BulkAction` is now at the crate root**
  (`rustio_admin::BulkAction`) for symmetry with the dispatch
  types. Still available at `rustio_admin::admin::BulkAction`.
- **`ConcreteOps<M>` impl bound tightened** from
  `M: AdminModel + Model` to
  `M: AdminModel + ModelAdmin + Model`. Functionally a no-op —
  `Admin::model::<M>()` always required `ModelAdmin` — lets
  `execute_bulk_action` delegate to `M::execute_bulk_action`
  cleanly.
- **Recovery email tone polish**: subject is now
  `Reset your password — {app_name}` (was
  `{X} — sign-in link`). Body opens with `Hello {greeting_name},`
  and closes with a `Account owner / {name} / {title} / {app_name}`
  signature block. Intro phrase changed from `"on your X account"`
  to `"for your X account"`.
- **Recovery page wording unified**: "This link is no longer
  valid" → "This reset link has expired"; "Send sign-in link"
  → "Send reset link". One vocabulary across page chrome,
  email subject, and email body.
- **Recovery pages gain `.rio-login-aside` panel** — calm
  operational security info under each form (Secure account
  recovery / Check your email / Why links expire).
- **Login card widened** from 400 → 440px to accommodate the
  aside panel cleanly.
- **`SiteBranding` defaults cleansed**: `site_title` and
  `site_header` defaults moved from "RustIO administration"
  to the generic "Admin" placeholder so a zero-config build
  no longer leaks the framework name.

### Behavioural changes (downstream-visible)

These are intentional design changes, but observable for
projects upgrading:

1. **Email subject changes from `{X} — sign-in link` to
   `Reset your password — {app_name}`.** Projects that
   inbox-filter by subject must update their rules.
2. **The chrome footer no longer shows the framework version /
   Documentation link to unauthenticated visitors** unless
   `Admin::show_powered_by(true)` is set. Authenticated
   operators still see in-product navigation (`Audit log`,
   `Sessions`).
3. **Email body uses `app_name` not `site_header`.** Projects
   that overrode `site_header` for branding need to call
   `Admin::app_name(...)` instead — or, for legacy compat,
   `Admin::site_branding(...)` with both fields set.
4. **Default Arabic font face stays Noto Naskh** (carried
   forward from 0.11.0).

### Internal

- New unit tests on `BulkActionResult` constructors
  (`admin::bulk::tests`) — 5 tests.
- New HTML-render tests on `render_recovery_html` covering
  identity escaping, powered-by opt-in, signature
  conditional, support-email line — 3 tests.
- `RecoveryEmailParts` reshaped to carry `app_name` instead
  of `site_header`; legacy callers within the framework
  crate construct via struct literal (in-crate access to
  `#[non_exhaustive]`).
- `lettre = "0.11"` added as a direct dep on the CLI and the
  library-circulation example (the framework crate stays
  lettre-free; that boundary holds).
- Workspace + CLI bumped to 0.12.0. "Releases at a glance"
  gains a 0.12.0 row.


## [0.11.0] — 2026-05-13

Minor release covering the multilingual typography infrastructure
and the production admin footer. Two substantial additions that
ship together: the typography work made Inter actually load (it
was in the token fallback chain since 0.1.1 but never had a
`@font-face`), added Thai / Devanagari / locale-gated CJK
coverage, and promoted Noto Naskh Arabic to the default Arabic
reading face; the footer work replaced the single-line copyright
strip with a three-column operational bar carrying framework
identity, navigation, and request context. Companion
`DESIGN_CHROME.md` doctrine documents both retroactively (topbar)
and prospectively (Phase D bulk-action confirmation bar).

**No schema migration, no public API signature change.** Two
observable behaviours change — see *Behavioural changes* below.

### Migration from 0.10.2

Bump `rustio-admin = "0.11.0"` and run
`cargo update -p rustio-admin`.

If your project consumes `--rio-font-arabic` either directly or
via `:lang(ar)` rendering, expect Noto Naskh Arabic as the
default face rather than Tajawal. Long-form Arabic reading now
uses Naskh's humanist x-height + softer terminals; UI surfaces
follow suit. Tajawal stays in the fallback chain and can be
re-pinned per surface via the new `.rio-arabic-display` utility
class.

If your project overrides `_base.html`, the framework's
production footer is rendered through `BaseContext`'s four new
fields (`framework_version`, `environment_label`,
`environment_kind`, `server_now`). Custom templates that block
the framework footer continue to work; templates that extend
`_base.html` pick up the new footer automatically.

Set `RUSTIO_ENV=Production` (or `=Staging`, `=Sandbox`, etc.)
on your deployment if you want the footer's environment badge to
reflect deploy-time rather than build-time facts. Without it the
badge shows "Development" for debug builds and "Production" for
release builds.

### Added

- **Production admin footer — three-column operational chrome.**
  Replaces the single-line copyright strip with a calm bottom bar
  modeled after Django-admin's maturity but in RustIO's modern
  system aesthetic. Three columns: identity (brand · version ·
  environment badge), navigation (documentation · audit log ·
  sessions), context (current operator · UTC render timestamp).
  Surface grafted to the admin shell (`--rio-surface` + hairline
  top border), tabular numerals on version + timestamp, no
  gradients, no oversized branding.
- `BaseContext` now exposes `framework_version`, `environment_label`,
  `environment_kind`, and `server_now`. `RUSTIO_ENV` env var
  overrides the auto-detected build kind; "Development" /
  "Production" map to amber / green status dots, free-text labels
  ("Staging", "Sandbox", …) collapse to a neutral grey dot. The
  env var is read once and cached in a process-wide `OnceLock` —
  no per-request syscall.
- **Multilingual typography infrastructure.** Six new self-hosted
  variable / single-weight woff2 fonts (~3.1 MB binary), wired
  through a new `base/typography-i18n.css` fragment and matching
  cascade entries in `admin.css` + `routes.rs`:
  - **Inter Variable** (~344 KB, wght 100..900) — the second-tier
    Latin fallback that `--rio-font-sans` already documented but
    never actually loaded. Covers Latin + Latin-ext + Cyrillic
    (basic + extended) + Greek (basic + extended) + Vietnamese
    under one variable file with a unicode-range filter.
  - **Noto Sans Thai Variable** (~27 KB) and **Noto Sans
    Devanagari Variable** (~119 KB) — auto-loaded via unicode-range
    on `U+0E00-0E5B` and `U+0900-097F` respectively. Latin-only
    pages never fetch them.
  - **Noto Sans JP / KR / SC** (static Regular, ~993 / 530 /
    1116 KB) — locale-gated via `:lang(ja)`, `:lang(ko)`,
    `:lang(zh|zh-CN|zh-Hans)` so Han Unification (shared
    `U+4E00-9FFF` between Japanese Kanji and Simplified Chinese
    Hanzi) doesn't render one region's content with the other
    region's shapes. Browsers synthesise bold.
- New CSS tokens in `tokens/typography.css`: `--rio-font-japanese`,
  `--rio-font-korean`, `--rio-font-chinese`, `--rio-font-thai`,
  `--rio-font-devanagari`. Each pairs the Noto face with the
  platform-native CJK / Thai / Devanagari font (Hiragino, PingFang,
  Malgun, Apple SD Gothic Neo, Sukhumvit, Hindi Sangam, etc.) for
  graceful fallback while the woff2 is in flight.
- New utility classes: `.rio-japanese`, `.rio-korean`,
  `.rio-chinese`, `.rio-thai`, `.rio-devanagari` to pin a family
  regardless of cascade order. Plus `.rio-arabic-display` to opt
  back into Tajawal's geometric style on selective accents now
  that Naskh is the default.

### Changed

- **Arabic primary face is now Noto Naskh Arabic** (was Tajawal).
  Both `--rio-font-arabic` (compact UI) and `--rio-font-arabic-body`
  (long-form) lead with Naskh; Tajawal remains in the fallback
  chain and stays available for selective geometric use via the
  new `.rio-arabic-display` utility. Naskh's higher x-height and
  humanist terminals improve readability in admin tables, forms,
  audit logs, and mixed Arabic / English interfaces — the surfaces
  where Tajawal's compact geometry previously felt cramped.
- `tokens/typography.css` header comment expanded to document the
  full multilingual stack and the Han-Unification rationale for
  the locale-gated CJK approach.
- `admin.css` header expanded with the new font inventory and the
  `base/typography-i18n.css` cascade entry.
- `assets/static/fonts/LICENSE.txt` extended with Inter / Noto
  Sans Thai / Devanagari / JP / KR / SC attribution sections.

### Internal

- `routes.rs`: six new `include_bytes!` font consts (`FONT_INTER`,
  `FONT_NOTO_THAI`, `FONT_NOTO_DEVA`, `FONT_NOTO_JP`,
  `FONT_NOTO_KR`, `FONT_NOTO_SC`) and matching `router.get`
  handlers under `/static/fonts/`. Each served with the existing
  `font/woff2` + 1-year immutable cache headers.
- Cascade lockstep test (`tests/cascade_lockstep.rs`) updated
  implicitly — it walks `admin.css` and `routes.rs`, both of which
  now reference the new `base/typography-i18n.css` fragment in
  matching positions. Passes.
- Binary size: +~3.1 MB (six new woff2 files). Download wire cost:
  zero for Latin-only pages, ~144 KB additional on Thai +
  Devanagari pages, ~530 KB–~1.1 MB on CJK pages (one-time, then
  cached for one year).
- New `BaseContext` fields populated in `BaseContext::new`: no
  caller-signature changes; every existing handler picks them up.
- `RUSTIO_ENV` resolved once per process via `OnceLock`; one
  syscall at first request, none thereafter.
- Workspace + CLI bumped to 0.11.0. "Releases at a glance" gains
  a 0.11.0 row.

### Behavioural changes (downstream-visible)

These are intentional design changes, but observable for projects
upgrading without theme overrides:

1. **Default Arabic face changes from Tajawal to Noto Naskh
   Arabic.** Surfaces tagged `lang="ar"` or wrapped in
   `[dir="rtl"]` render in Naskh's humanist style. To pin the
   geometric Tajawal style on a specific surface, apply the new
   `.rio-arabic-display` utility class.
2. **Footer markup expands from one line to a three-column bar.**
   Projects that extend `_base.html` keep working; projects that
   replaced the footer with custom markup are unaffected (their
   override still wins). Footer vertical real estate is roughly
   unchanged (`--rio-fs-xs` text on `--rio-s3` padding is more
   compact per row than the previous `--rio-fs-sm` on
   `--rio-s4`).


## [0.10.2] — 2026-05-13

Patch release closing one of the two framework gaps that
`examples/library-circulation/migrations/0005_seed.sql` listed
under D.4. **No schema migration, no public API signature
change.** One observable behaviour changes — see
*Behavioural changes* below.

### Migration from 0.10.1

Bump `rustio-admin = "0.10.2"` and run
`cargo update -p rustio-admin`.

If your project relied on `permissions::create_group` returning
an error on duplicate names (e.g. as a uniqueness check), switch
to selecting from `rustio_groups` first or treat the returned id
as "create-or-existing." Most projects can drop existence guards
around `create_group` calls; the second call is now a no-op that
returns the existing id.

### Fixed

- **`permissions::create_group` was not idempotent.** Repeated
  calls with the same `name` hit the `UNIQUE` constraint on
  `rustio_groups.name` and surfaced a sqlx unique-violation —
  forcing every Rust seed path to either guard the call or eat
  the error. The function now mirrors the `permission_id` idiom
  in the same module:
  `ON CONFLICT (name) DO UPDATE SET description = rustio_groups.description RETURNING id`.
  The no-op self-assignment exists solely so `RETURNING` fires
  on the conflict path. A second call with the same name returns
  the existing group's id; the stored description is preserved
  (first-write-wins) — idempotency applies when the args match,
  and an explicit description mutator would be a separate
  helper.
  (`crates/rustio-admin/src/auth/permissions.rs:272`)

### Behavioural changes (downstream-visible)

- **`rustio group create <name>` no longer errors on duplicates.**
  The CLI command (sole caller of `create_group`) succeeds on
  re-runs and prints the same `Created group id=X name=...` line
  with the original id. Friendlier for shell scripting; existing
  scripts that relied on the error to detect duplicates need to
  switch to a `rustio group list` check (or accept the new
  idempotent semantics).

### Documentation

- `examples/library-circulation/migrations/0005_seed.sql` footer
  trimmed from two gaps to one (bulk-action dispatch hook stays
  deferred). The migration comment now notes that group seeding
  is *viable* in 0.10.2 but still belongs in Rust at boot, not in
  SQL — SQL migrations run before `admin.seed_permissions()`, so
  the permission rows that group rows would bind to don't exist
  at migration time.
- `examples/library-circulation/README.md` "does not yet
  demonstrate" section updated to match.

### Internal

- Workspace + CLI bumped to 0.10.2. "Releases at a glance" gains
  a 0.10.2 row.


## [0.10.1] — 2026-05-13

Patch release covering two framework bugs and one example
completeness gap, all surfaced by running
`examples/library-circulation/` end-to-end against a fresh
Postgres on the 0.10.0 tag. **No schema migration, no public API
signature change.** Two observable behaviours change — see
*Behavioural changes* below.

### Migration from 0.10.0

Bump `rustio-admin = "0.10.1"` and run
`cargo update -p rustio-admin`.

If your project has a `#[derive(RustioAdmin)]` struct whose name
ends in `-ch`, `-sh`, `-x`, `-z`, or consonant + `y`, the
auto-generated route slug will change in this release (see
*Behavioural changes*). Pin the old slug with
`#[rustio(admin_name = "old_slug")]` if URL stability matters more
than correct grammar.

If your project has a String field literally named `status` and
relied on the (broken) `draft / published` dropdown that the
renderer used to synthesise, set
`AdminField.choices = Some(&["draft", "published"])` (or your
real values) — or accept the new plain-text input.

### Fixed

- **`plural_snake` ignored English plural rules.** Every struct
  name was pluralised by appending a bare `s`, so `Branch` routed
  to `/admin/branchs` and rendered as "Branchs". The macro now
  applies the regular cases: `-ch/-sh/-x/-z` take `-es`,
  consonant + `y` becomes `-ies`, vowel + `y` keeps `-s`, and a
  trailing `s` is left as-is. Irregular plurals (`Person` →
  `People`) still need `#[rustio(admin_name = "...")]`. Four unit
  tests added covering each rule
  (`crates/rustio-admin-macros/src/lib.rs:549`).

- **Form renderer hardcoded a `draft / published` dropdown for
  any field named `status`.** The underlying field stayed a
  `String`, so the synthesised `<select>` permanently shadowed
  every domain state machine — a loan with
  `active / returned / overdue` rendered as a select that only
  let the user pick `draft` or `published`. The synth block is
  removed; status fields without `choices` now render as plain
  text inputs. Projects that want a constrained widget set
  `AdminField.choices` explicitly
  (`crates/rustio-admin/src/admin/render.rs:1447`).

### Changed

- **`examples/library-circulation/`** — `Item.branch_id`,
  `Loan.patron_id`, and `Loan.item_id` now carry
  `#[rustio(belongs_to = "...", display = "...")]`. The admin
  renders proper FK dropdowns on create/edit forms and FK
  navigation links in list views. The framework still leaves FK
  detection explicit — `_id` suffix alone is not a signal — so
  the canonical example must demonstrate the declaration.

### Behavioural changes (downstream-visible)

These are bugfixes, but they change observable behaviour:

1. **Route slug for affected struct names changes.** Anything
   ending in `-ch/-sh/-x/-z` gains an `-es` suffix; consonant +
   `y` becomes `-ies`. Saved URLs / inbound links / hand-written
   navigation that pointed at the old slug must update or pin
   with `#[rustio(admin_name)]`.
2. **`status` fields lose the bogus `draft / published`
   dropdown.** They render as plain text inputs unless
   `AdminField.choices` is set.

### Internal

- Workspace + CLI bumped to 0.10.1. "Releases at a glance" gains
  a 0.10.1 row.


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
  documents (`STRATEGIC_RESET_PLAN.md`,
  `APIS_AND_DOCS_PLAN.md`) are archived alongside
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

See [`DESIGN_R3_MFA.md`](./docs/design/DESIGN_R3_MFA.md) for the state
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

- [`DESIGN_R3_MFA.md`](./docs/design/DESIGN_R3_MFA.md) added — the
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

See [`DESIGN_R2_ORGANISATIONAL.md`](./docs/design/DESIGN_R2_ORGANISATIONAL.md)
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

- [`DESIGN_R2_ORGANISATIONAL.md`](./docs/design/DESIGN_R2_ORGANISATIONAL.md)
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

- [`DESIGN_RECOVERY.md`](./docs/design/DESIGN_RECOVERY.md) added — the
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
  [strategic reset plan](./docs/archive/STRATEGIC_RESET_PLAN.md)
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
