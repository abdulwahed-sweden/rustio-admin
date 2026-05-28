# Permission Architecture

The framework ships three structural permission groups on every
fresh database. They are foundational pieces the application builds
on, not throwaway demo data.

This document is the contract for what is seeded, when it is
seeded, the conditions under which seeding is skipped, and how the
seeded groups' names lock to the values of `rustio-admin user create
--role`.

Pull request review for any permissions-touching change runs against
this document, not only the diff.

> **Governing sentence**
> A fresh `rustio-admin migrate apply` leaves the project with three named
> groups whose intent is obvious and whose permission scope is
> conservative enough that no operator can shoot themselves in the
> foot by inheriting them.

---

## 1. Purpose

### 1.1 What this governs

- The three seeded group names: `administrator` / `editor` / `viewer`.
- Their per-model permission grants (the "grant matrix").
- The conditions under which seeding runs vs. skips.
- The exact-name lockstep between the seeded groups and the
  `--role` values accepted by `rustio-admin user create`.
- The CI guard that enforces the lockstep.

### 1.2 What this does not cover

- The five-tier `Role` enum (`User / Staff / Supervisor /
  Administrator / Developer`) — that's the framework's existing
  authority-tier model, separate from the seeded-groups system.
- Authentication / sessions / MFA — see the matching `DESIGN_*.md`.
- Audit emission — see `DESIGN_AUDIT.md`.
- Per-model `ModelAdmin::list_filter` / `search_fields` etc. — those
  are presentation, not authority.

### 1.3 Closing principle

Structural defaults. Conservative. Lockstepped. Never re-shape a
project that already built its own group structure.

---

## 2. The three seeded groups

| Group | Permission scope | Audience |
|---|---|---|
| `administrator` | Every action on every model. Plus user / group / permission management surfaces. | Owner, operator. |
| `editor` | `add` / `change` / `view` on content models. **No** `delete`. **No** user / group / permission / settings management. | Content team, application users. |
| `viewer` | `view` on content models. **No** mutation. **No** user / group / permission / settings access. | Auditors, read-only stakeholders. |

The framework deliberately does NOT seed `staff`, `manager`,
`support`, `moderator`, `contributor`, or any other group name.
Those are application-specific and belong to the project owner.

---

## 3. The grant matrix

After every `rustio-admin startapp <model>` and the project's startup
`Admin::seed_permissions(...)` call, the four CRUD permissions for
each registered model are registered AND granted to the seeded
groups per this matrix:

|              | `add` | `change` | `delete` | `view` |
|--------------|-------|----------|----------|--------|
| `administrator` | ✓     | ✓        | ✓        | ✓      |
| `editor`        | ✓     | ✓        |          | ✓      |
| `viewer`        |       |          |          | ✓      |

### 3.1 Why `editor` lacks `delete`

`editor` is the role you give to your content team. Destructive
operations belong to an administrator by default. Projects that
want editor-level delete access either:

- grant `<app>.delete_<model>` to the `editor` group explicitly
  via the admin permission-matrix UI, or
- move those users to `administrator`.

The doctrine errs on the side of "delete is dangerous, raise the
bar by one tier."

### 3.2 Idempotency

Every grant uses `INSERT … ON CONFLICT DO NOTHING`. Re-running
`Admin::seed_permissions(...)` on a database where the seed has
already run is a no-op for existing grants and adds grants only
for newly-registered models. The framework calls
`seed_permissions` on every boot.

---

## 4. Seeding conditions

### 4.1 When the seed runs

`auth::seed_default_groups(db)` runs at the end of
`auth::init_tables(db)`, which the scaffolded `main.rs` calls on
every boot. The function:

1. Counts rows in `rustio_groups` whose `name` is NOT in
   `('administrator', 'editor', 'viewer')`.
2. If the count is `0` — fresh database, or one that has only the
   defaults — calls `create_group(...)` for each of the three names
   (idempotent via `ON CONFLICT (name)`).
3. If the count is `> 0` — the project has built its own group
   structure — the seed skips entirely.

### 4.2 When the seed does NOT run

Any database where `rustio_groups` contains a row whose `name` is
not one of the three defaults. This is the **never silently
re-shape** guarantee: a project that built `staff`, `regional_lead`,
`support_l2` on 0.20.0 picks up zero changes by upgrading to 0.21.0.
The operator can opt in by manually calling `auth::create_group(...)`
for the three default names, after which the next boot's per-model
grant pass populates them.

### 4.3 What "missing default groups" means at grant-time

`Admin::seed_permissions(...)` calls
`auth::grant_model_to_default_groups(db, app, singular)` for every
registered model. If any of the three groups is missing (because
the user-defined-groups guard fired in §4.1), the grant for that
group is a silent no-op — `grant_to_group` is never called for an
ID that doesn't exist. No error, no log line, no surprise.

---

## 5. The `--role` exact-name lockstep

### 5.1 The contract

The set of group names in `auth::DEFAULT_GROUP_NAMES` MUST equal
the set of `CliRole` variants whose `default_group_name()` returns
`Some(name)`:

- `auth::DEFAULT_GROUP_NAMES` = `["administrator", "editor", "viewer"]`
- `CliRole::Administrator.default_group_name()` = `Some("administrator")`
- `CliRole::Editor.default_group_name()` = `Some("editor")`
- `CliRole::Viewer.default_group_name()` = `Some("viewer")`
- All other `CliRole` variants return `None`.

The strings on both sides MUST match byte-for-byte.

### 5.2 Why

`rustio-admin user create --email alice@x --role editor` creates a user
and immediately adds them to the `editor` group. The role the
operator typed and the group the user landed in are the same
string. If the strings drift, the create succeeds but the new user
ends up in nothing — silently broken authority.

### 5.3 How the lockstep is enforced

The `lockstep_default_groups_match_cli_role_names` test in
`crates/rustio-admin-cli/src/user.rs` builds two `BTreeSet`s — one
from each side of the lockstep — and asserts they're equal. A
rename on either side fails CI.

Reviewers who hit this failure should NOT relax the assertion.
They should either:

- revert the asymmetric change, or
- extend both sides together (with a corresponding update to this
  doctrine doc).

---

## 6. The `Role` enum vs. the seeded groups

These are two separate systems and the distinction matters.

### 6.1 The `Role` enum (`rustio_admin::auth::Role`)

A 5-tier authority ladder:
`User / Staff / Supervisor / Administrator / Developer`.

- Stored as a column on `rustio_users`.
- Used by the existing `check_permission` semantics.
- `Administrator` and `Developer` **bypass** the group permission
  check entirely (see `auth::permissions::check_permission`).
- Has existed since well before PR 2.2.

### 6.2 The seeded groups (PR 2.2)

Three named groups in `rustio_groups`, joined to users via
`rustio_user_groups`.

- `administrator` / `editor` / `viewer`.
- The grant matrix in §3 drives per-model permissions for them.
- Users created via `--role editor` or `--role viewer` get
  `Role::User` (the lowest tier) on `rustio_users.role` — their
  actual authority comes entirely from group membership.

### 6.3 How they interact

| `--role` value at create | Stored `Role` | Default group membership |
|---|---|---|
| `user` (legacy) | `User` | — |
| `staff` (legacy) | `Staff` | — |
| `supervisor` (legacy) | `Supervisor` | — |
| `administrator` | `Administrator` (bypasses checks) | `administrator` |
| `developer` (legacy) | `Developer` (bypasses checks) | — |
| `editor` (PR 2.2) | `User` | `editor` |
| `viewer` (PR 2.2) | `User` | `viewer` |

`Administrator` and `Developer` keep their permission-bypass
behaviour — they have implicit full access regardless of group
membership. The `administrator` group assignment is for UI / audit
clarity (the user shows up in the group's member list) and for
the case where a future PR removes the tier-level bypass.

---

## 7. Non-negotiable rules

1. **Three groups, not more.** Any application-specific group is
   the project owner's responsibility, not the framework's.
2. **Conservative defaults.** `editor` has no `delete`; `viewer`
   has no mutation. The framework never relaxes these without an
   explicit doctrine amendment.
3. **Lockstep is enforced by code.** Adding `Editor` or `Viewer`
   to `CliRole` without adding the same name to
   `DEFAULT_GROUP_NAMES` is impossible to merge.
4. **No silent re-shaping.** A database with user-defined groups
   is never modified by the seed. Period.
5. **Idempotency everywhere.** Every seed operation tolerates
   re-running.
6. **No demo users.** The seed creates groups, not accounts. The
   project's first admin user comes from `rustio-admin user create`.
7. **The grant matrix is the contract.** Reviewers who want to
   add or remove a permission grant must update this document.

---

## 8. Acceptance criteria for this document

This document is complete when:

- It can be cited by PRs that touch the seeded groups or the
  `--role` lockstep.
- It defines exactly which groups exist, which permissions go to
  each, when seeding runs, and when seeding skips.
- It anchors the strict-equality lockstep guarantee.
- Reviewers can resolve any "should this seed run / should this
  permission be granted" question by reading the matrix in §3 and
  the conditions in §4 without further conversation.
