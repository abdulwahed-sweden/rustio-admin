//! The admin's data vocabulary. Kept separate from rendering and
//! handlers so changes here ripple out predictably.

// `for_testing[_failing_list]` + the PanicOps/FailingOps fixtures
// are part of the admin's test surface but no in-tree test exercises
// them yet (the legacy admin/macro_tests etc. land in a follow-up).
// Keep them gated behind cfg(test) elsewhere; allow dead inside that
// gate.
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::auth::{
    DefaultPasswordPolicy, DefaultRecoveryPolicy, MfaPolicy, SharedPasswordPolicy,
    SharedRecoveryPolicy,
};
use crate::email::{LogMailer, SharedMailer};
use crate::error::Result;
use crate::http::FormData;
use crate::orm::{Db, Value};

pub(crate) type CreateResult<'a> =
    Pin<Box<dyn Future<Output = Result<std::result::Result<i64, Vec<String>>>> + Send + 'a>>;

pub(crate) type UpdateResult<'a> =
    Pin<Box<dyn Future<Output = Result<std::result::Result<(), Vec<String>>>> + Send + 'a>>;

// ---------------------------------------------------------------------------
// User profile extension API
// ---------------------------------------------------------------------------

// public:
/// One labeled section rendered in the project-extension area of the
/// built-in user profile page (admin/user_view.html — `{% block
/// project_user_fields %}`). A project's extension closure returns
/// `Vec<UserProfileSection>` so it can contribute multiple disjoint
/// areas in a single registration.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UserProfileSection {
    pub label: String,
    pub rows: Vec<UserProfileRow>,
}

// public:
/// One key-value row inside a [`UserProfileSection`]. Both fields are
/// `String` so projects can format whatever shape they need. Rendered
/// escaped — pass plain text; for arbitrary HTML, projects override
/// the template block instead.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UserProfileRow {
    pub label: String,
    pub value: String,
}

/// The boxed-closure shape stored on `Admin`. `pub(crate)` because
/// projects use the generic [`Admin::user_profile_extension`] builder
/// method and never have to name this directly.
pub(crate) type UserProfileExtensionFn =
    Arc<dyn Fn(Db, crate::auth::UserProfile) -> UserProfileExtensionFuture + Send + Sync + 'static>;

pub(crate) type UserProfileExtensionFuture =
    Pin<Box<dyn Future<Output = Result<Vec<UserProfileSection>>> + Send + 'static>>;

// public:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldType {
    I32,
    I64,
    F64,
    Decimal,
    Bool,
    String,
    Email,
    Phone,
    DateTime,
    Date,
    Time,
    Uuid,
    OptionalI64,
    OptionalString,
    OptionalDateTime,
    /// Column persists a relative file path produced by the
    /// framework's multipart-form path. Renders as
    /// `<input type="file">`; on POST the framework writes the
    /// uploaded bytes to `<Admin::uploads_dir>/<rel_path>` and
    /// injects the resulting path string back into the form so
    /// `from_form` sees a normal `String` column.
    FilePath,
    /// Nullable variant of [`Self::FilePath`].
    OptionalFilePath,
}

impl FieldType {
    // public:
    pub fn widget(&self) -> &'static str {
        match self {
            FieldType::Bool => "checkbox",
            FieldType::DateTime | FieldType::OptionalDateTime => "datetime",
            FieldType::Date => "date",
            FieldType::Time => "time",
            FieldType::Email => "email",
            FieldType::Phone => "tel",
            FieldType::I32
            | FieldType::I64
            | FieldType::OptionalI64
            | FieldType::F64
            | FieldType::Decimal => "number",
            FieldType::FilePath | FieldType::OptionalFilePath => "file",
            FieldType::Uuid | FieldType::String | FieldType::OptionalString => "text",
        }
    }

    // public:
    pub fn nullable(&self) -> bool {
        matches!(
            self,
            FieldType::OptionalI64
                | FieldType::OptionalString
                | FieldType::OptionalDateTime
                | FieldType::OptionalFilePath
        )
    }
}

// public:
#[derive(Debug, Clone)]
pub struct AdminField {
    pub name: &'static str,
    pub label: &'static str,
    pub field_type: FieldType,
    pub editable: bool,
    pub relation: Option<AdminRelation>,
    /// Closed list of allowed string values for this field. When
    /// `Some`, the form layer renders a `<select>` with one option per
    /// entry. The values double as labels (raw, not humanised) per
    /// the "no invented content" rule.
    pub choices: Option<&'static [&'static str]>,
}

// public:
#[derive(Debug, Clone)]
pub struct AdminRelation {
    pub target_model: &'static str,
    pub display_field: Option<&'static str>,
    /// `true` for many-to-many relations (form renders
    /// `<select multiple>`), `false` for the default belongs-to
    /// (single `<select>`). Macro emits `false`; consumers that want
    /// M2M behaviour must hand-set this until the macro learns a
    /// `#[rustio(many_to_many)]` attribute.
    pub multi: bool,
}

// public:
/// What the `#[derive(RustioAdmin)]` macro produces for each struct.
pub trait AdminModel: Send + Sync + 'static {
    const ADMIN_NAME: &'static str;
    const DISPLAY_NAME: &'static str;
    const SINGULAR_NAME: &'static str;
    const FIELDS: &'static [AdminField];

    /// Render one row for the list page (column → display string).
    fn display_values(&self) -> Vec<(String, String)>;

    /// Populate a new instance from an HTTP form. Returns a list of
    /// validation errors if anything was wrong.
    fn from_form(form: &FormData) -> std::result::Result<Self, Vec<String>>
    where
        Self: Sized;

    /// A stable label for one instance (used on the delete confirm page).
    fn object_label(&self) -> String;

    fn id(&self) -> i64;

    fn values_to_update(&self) -> Vec<(&'static str, Value)>;
}

// public:
/// Runtime metadata about one admin-registered model. Captures both
/// the [`AdminModel`] static surface and the [`super::ModelAdmin`]
/// customisation values at registration time, so handlers read every
/// per-model knob from this struct instead of re-resolving traits.
pub struct AdminEntry {
    pub admin_name: &'static str,
    pub display_name: &'static str,
    pub singular_name: &'static str,
    /// SQL table name. For user-registered models this is `<M as Model>::TABLE`;
    /// for the synthetic core User entry it's `"rustio_users"`.
    pub table: &'static str,
    pub fields: &'static [AdminField],
    /// `true` only for framework-owned entries (currently just `User`).
    pub core: bool,
    /// `ModelAdmin::list_display()`. Empty → use every column on
    /// `fields`; non-empty → use exactly the listed names in order.
    pub list_display: &'static [&'static str],
    /// `ModelAdmin::list_filter()`. Empty by default.
    pub list_filter: &'static [&'static str],
    /// `ModelAdmin::search_fields()`. Empty by default.
    pub search_fields: &'static [&'static str],
    /// `ModelAdmin::search_index_column()`. When `Some`, the
    /// list-page search uses Postgres FTS against this tsvector
    /// column instead of the default `ILIKE` OR-loop across
    /// `search_fields`.
    pub search_index_column: Option<&'static str>,
    /// `ModelAdmin::ordering()`. Strings parsed via
    /// [`super::modeladmin::parse_order_spec`].
    pub ordering: &'static [&'static str],
    /// `ModelAdmin::list_per_page()`. Default 50.
    pub list_per_page: usize,
    /// `ModelAdmin::readonly_fields()`. Empty by default.
    pub readonly_fields: &'static [&'static str],
    /// `ModelAdmin::fieldsets()`. Empty → fall back to the
    /// framework's name-heuristic grouping.
    pub fieldsets: &'static [super::modeladmin::Fieldset],
    /// `ModelAdmin::bulk_actions()`. Empty by default — the bulk bar
    /// only renders the framework's built-in Delete.
    pub bulk_actions: &'static [super::modeladmin::BulkAction],
    /// `ModelAdmin::inlines()`. Empty by default — no related-
    /// children section renders below the edit form. Project
    /// authors opt in per parent model.
    pub inlines: &'static [super::modeladmin::Inline],
    pub(crate) ops: Arc<dyn AdminOps>,
}

// public:
/// Per-request options for [`AdminOps::list`]. Empty / `None` fields
/// mean "framework default": no ordering override falls back to
/// `id DESC` inside the runtime, no filters skips the WHERE clause,
/// no limit fetches every row.
#[derive(Debug, Clone, Default)]
pub struct ListOpts {
    /// Validated `(column, dir)` pairs to apply as `ORDER BY`. The
    /// column name is bound to the model's `M::COLUMNS` set inside
    /// the runtime, so callers can pass user-supplied names without
    /// SQL-injection risk.
    pub ordering: Vec<(String, super::modeladmin::SortDir)>,
    /// `(column, value)` pairs applied as `WHERE col::text = $N`.
    /// Cast to text so the comparison matches the same string-shape
    /// semantics the in-memory pre-P10 filter used for bool / int /
    /// timestamp columns.
    pub filters: Vec<(String, String)>,
    /// Date-range filters surfaced from
    /// [`super::filters::FilterKind::DateRange`]: each tuple is
    /// `(column, gte, lte)` with `YYYY-MM-DD` strings. Either bound
    /// may be `None` (open-ended). The runtime renders
    /// `WHERE col::date >= $N::date` and / or
    /// `WHERE col::date <= $N::date`. Column names are validated
    /// against `M::COLUMNS`; the date strings are passed through
    /// to Postgres which rejects malformed inputs.
    pub date_ranges: Vec<(String, Option<String>, Option<String>)>,
    /// Multi-select filters surfaced from
    /// [`super::filters::FilterKind::MultiSelect`]: each tuple is
    /// `(column, values)` and renders as
    /// `WHERE col::text IN ($N, $N+1, …)`. An empty `values` list is
    /// silently skipped — "nothing selected" should not collapse the
    /// query to an empty result. Column names are validated against
    /// `M::COLUMNS`; the caller is responsible for restricting
    /// values to the field's declared `choices`.
    pub multi_filters: Vec<(String, Vec<String>)>,
    /// Free-text search: `(term, columns)`. The runtime emits
    /// `WHERE (col1::text ILIKE $N OR col2::text ILIKE $N OR …)`
    /// with `$N = '%term%'`. An empty `term` or empty `columns`
    /// leaves the WHERE alone.
    pub search: Option<(String, Vec<String>)>,
    /// When `Some(col)`, the search WHERE clause uses Postgres
    /// FTS against this tsvector column
    /// (`<col> @@ websearch_to_tsquery('english', $N)`) instead
    /// of the ILIKE OR-loop above. `search` must still carry the
    /// raw term; the columns slice is ignored on this path.
    /// Sourced from `AdminEntry::search_index_column`.
    pub search_index_column: Option<&'static str>,
    /// `LIMIT $N` for the data query. The COUNT(*) query never
    /// applies it. `None` → no limit.
    pub limit: Option<i64>,
    /// `OFFSET $N` for the data query. `None` or `Some(0)` → no offset.
    pub offset: Option<i64>,
}

// public:
/// Result of [`AdminOps::list`]: the requested page plus the total
/// row count under the same WHERE clause (so handlers can render
/// pagination footers without a separate query).
#[derive(Debug, Default)]
pub struct ListPage {
    pub rows: Vec<ListRow>,
    pub total: i64,
}

/// Type-erased CRUD operations. The `Admin::model::<M>()` call captures
/// a concrete `M: AdminModel + Model` and hides it behind this trait so
/// the router can treat every model uniformly. The single live impl is
/// [`super::ops::ConcreteOps<M>`].
pub(crate) trait AdminOps: Send + Sync {
    fn list<'a>(
        &'a self,
        db: &'a Db,
        opts: ListOpts,
    ) -> Pin<Box<dyn Future<Output = Result<ListPage>> + Send + 'a>>;

    fn find_row<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<EditRow>>> + Send + 'a>>;

    fn create<'a>(&'a self, db: &'a Db, form: &'a FormData) -> CreateResult<'a>;

    fn update<'a>(&'a self, db: &'a Db, id: i64, form: &'a FormData) -> UpdateResult<'a>;

    fn delete<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    fn object_label<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'a>>;

    /// Run a project-defined bulk action against the supplied row
    /// ids. Called once per submission with the full id list, so the
    /// implementation can choose between a single bulk SQL update or
    /// a per-row loop.
    ///
    /// The real public dispatcher is
    /// [`super::ModelAdmin::execute_bulk_action`];
    /// [`super::ops::ConcreteOps<M>`] forwards into the model's
    /// override from this trait method. The
    /// [`super::types::CoreUserOps`] entry has no project surface
    /// (the User row is framework-owned), so it returns the
    /// "no project handler" error verbatim.
    ///
    /// Note: the framework's built-in `delete` action is **not**
    /// dispatched through here. It runs through the cascade-aware
    /// `/bulk_delete` route which calls `delete()` per row. Override
    /// `delete` instead if you need custom delete semantics.
    fn execute_bulk_action<'a>(
        &'a self,
        db: &'a Db,
        name: &'a str,
        ids: &'a [i64],
        ctx: &'a super::bulk::BulkActionContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<super::bulk::BulkActionResult>> + Send + 'a>>;
}

// public:
/// A row as shown on the list page.
#[derive(Debug)]
pub struct ListRow {
    pub id: i64,
    pub cells: Vec<String>,
    /// Optional link target per cell, parallel to `cells`. When
    /// `Some`, the renderer wraps that cell's content in an
    /// `<a href="/admin/{admin_name}/{id}/edit">…</a>` so foreign-key
    /// columns become click-throughs to the related row. Populated by
    /// the post-list hydration pass in `handlers::hydrate_fk_cells`;
    /// `ConcreteOps::list` always emits a parallel vector of `None` of
    /// matching length so callers that skip hydration still satisfy
    /// the parallel-vector invariant.
    pub cell_links: Vec<Option<CellLink>>,
}

// public:
/// One resolved foreign-key cell. The renderer turns this into
/// `<a href="/admin/{admin_name}/{id}/edit">…</a>` around the cell's
/// display label.
#[derive(Debug, Clone)]
pub struct CellLink {
    /// Target model's admin slug (e.g. `"categories"` for `Category`).
    pub admin_name: String,
    /// Target row id.
    pub id: i64,
}

// public:
/// The raw field values used to pre-fill the edit form.
#[derive(Debug)]
pub struct EditRow {
    #[allow(dead_code)]
    pub id: i64,
    pub values: Vec<(String, String)>,
}

// public:
/// Per-project admin branding — the user-facing identity layer.
///
/// Production deployments MUST set [`SiteBranding::app_name`] (or
/// use the [`Admin::app_name`] builder) — the framework name
/// `RustIO` should never appear as the visible product identity for
/// end users. RustIO is the vendor; the *deployed application* is
/// what users see in their inbox, on the login page, and in the
/// password-reset flow.
///
/// Field roles:
///
///   - [`app_name`](Self::app_name) — primary user-facing product
///     identity. Used by every framework-emitted surface that
///     reaches end users: email subjects + headers, recovery
///     pages, admin chrome footer, audit summaries.
///   - [`app_tagline`](Self::app_tagline) — optional secondary
///     line. Renders as the descriptor under the brand wordmark
///     in recovery emails. `None` falls back to the framework's
///     generic "Account security notification" caption.
///   - [`support_email`](Self::support_email) — optional contact
///     surfaced in the recovery email footer ("If you didn't
///     request this, contact <support@…>"). `None` omits the
///     line.
///   - [`public_url`](Self::public_url) — canonical public URL
///     used when composing reset links if the request's
///     `Host` / `X-Forwarded-Host` derivation is unreliable.
///     `None` falls back to header-derived URLs.
///   - [`show_powered_by`](Self::show_powered_by) — opt-in
///     "Powered by RustIO" credit in the chrome footer + email
///     footer. Defaults to `false`; the framework name stays
///     invisible to end users unless the project explicitly
///     enables this.
///
/// Legacy fields ([`site_title`](Self::site_title) /
/// [`site_header`](Self::site_header) / etc.) predate this
/// architecture and are still honoured for backwards compatibility,
/// but their defaults were renamed away from "RustIO administration"
/// so a zero-config build no longer leaks the framework name. New
/// code should use the `app_*` fields exclusively.
#[derive(Clone, Debug)]
pub struct SiteBranding {
    /// Primary user-facing product identity. Visible in:
    /// page chrome (topbar, footer), email subjects + headers,
    /// audit summaries, recovery pages, login screen.
    pub app_name: String,
    /// Optional secondary line for use under the brand wordmark in
    /// emails / auth surfaces. Examples: "Operational library
    /// management", "Health-system administration". `None` falls
    /// back to "Account security notification" in recovery emails.
    pub app_tagline: Option<String>,
    /// Optional support contact surfaced in recovery email footer.
    pub support_email: Option<String>,
    /// Optional canonical public URL — e.g. `https://library.example.com`.
    /// Used as the reset-link base when `Host` header isn't trustworthy.
    pub public_url: Option<String>,
    /// `true` → renders a small, low-contrast "Powered by RustIO"
    /// line in the admin chrome footer and at the very bottom of
    /// framework emails. Default: `false` (framework name invisible).
    pub show_powered_by: bool,
    // ---- legacy fields, retained for backwards compatibility -----
    pub site_title: String,
    pub site_header: String,
    pub index_title: String,
    pub footer_copyright: String,
    /// DNS-shape string available to project handlers; not surfaced in
    /// any framework template.
    pub domain: String,
}

impl Default for SiteBranding {
    fn default() -> Self {
        // Generic "Admin" defaults so a zero-config build doesn't
        // leak the framework name. Real projects MUST set app_name
        // via `Admin::app_name(...)` or a full `site_branding(...)`
        // override.
        Self {
            app_name: "Admin".into(),
            app_tagline: None,
            support_email: None,
            public_url: None,
            show_powered_by: false,
            site_title: "Admin".into(),
            site_header: "Admin".into(),
            index_title: "Site administration".into(),
            footer_copyright: String::new(),
            domain: "localhost".into(),
        }
    }
}

// public:
/// Project-level override patch for the admin chrome palette.
///
/// `admin.css` is the single source of truth for the framework's design
/// tokens (palette, semantic surfaces, typography scale, …).
/// `AdminTheme` is **purely a patch layer**: every field is
/// `Option<String>` and defaults to `None`, meaning *“don’t override —
/// let the stylesheet decide.”* Out of the box the framework emits no
/// inline `<style>` block at all.
///
/// Set a field — usually via the fluent builder methods or
/// [`Admin::accent_color`] — to inject a `--rio-*` custom-property
/// override on every page. Overrides are emitted as a single
/// `html { ... }` block after `admin.css`, so they win cascade ties
/// without `!important`. The framework is light-only.
///
/// Values are hex (`#rrggbb` or `rrggbb`); the leading `#` is
/// auto-normalised at construction. Malformed input is rejected at
/// override time rather than panicking — the admin path never breaks
/// over a config typo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AdminTheme {
    pub accent: Option<String>,
    pub bg: Option<String>,
    pub surface: Option<String>,
    pub text: Option<String>,
    pub text_muted: Option<String>,
    pub border: Option<String>,
}

impl AdminTheme {
    // public:
    /// New empty patch — no overrides emitted, `admin.css` wins.
    pub fn new() -> Self {
        Self::default()
    }

    // internal:
    /// `true` when at least one field is set. Used by the renderer to
    /// decide whether to emit the inline `<style>` block at all.
    pub(crate) fn has_overrides(&self) -> bool {
        self.accent.is_some()
            || self.bg.is_some()
            || self.surface.is_some()
            || self.text.is_some()
            || self.text_muted.is_some()
            || self.border.is_some()
    }

    // public:
    /// Override `--rio-accent`. Hex form, `#` optional.
    pub fn accent(mut self, color: impl Into<String>) -> Self {
        self.accent = Some(normalise_hex(color));
        self
    }

    // public:
    /// Override `--rio-bg` (page canvas).
    pub fn bg(mut self, color: impl Into<String>) -> Self {
        self.bg = Some(normalise_hex(color));
        self
    }

    // public:
    /// Override `--rio-surface` (cards, topbar, sidebar, table body).
    pub fn surface(mut self, color: impl Into<String>) -> Self {
        self.surface = Some(normalise_hex(color));
        self
    }

    // public:
    /// Override `--rio-text` (body text colour).
    pub fn text(mut self, color: impl Into<String>) -> Self {
        self.text = Some(normalise_hex(color));
        self
    }

    // public:
    /// Override `--rio-text-muted` (secondary text, breadcrumb links).
    pub fn text_muted(mut self, color: impl Into<String>) -> Self {
        self.text_muted = Some(normalise_hex(color));
        self
    }

    // public:
    /// Override `--rio-border` (default divider, card outline).
    pub fn border(mut self, color: impl Into<String>) -> Self {
        self.border = Some(normalise_hex(color));
        self
    }
}

// public:
/// Builder for the admin. Register models with `.model::<M>()`, then
/// hand it to the router via `register_admin_routes`.
pub struct Admin {
    pub(crate) entries: Vec<AdminEntry>,
    pub(crate) site_branding: SiteBranding,
    pub(crate) user_profile_ext: Option<UserProfileExtensionFn>,
    pub(crate) theme: AdminTheme,
    /// The outbound-mail handle. Defaults to [`LogMailer`]; projects
    /// override via [`Admin::mailer`]. R1+ recovery flows
    /// (`DESIGN_RECOVERY.md` §12) read this to dispatch reset emails;
    /// `auth::recovery::issue_reset_token` (R1 commit #7) reads it
    /// at runtime. Held as `Arc<dyn Mailer>` so cloning the field is
    /// a single reference-count bump and the field stays trivially
    /// Send + Sync (the trait's supertraits are `Send + Sync`).
    pub(crate) mailer: SharedMailer,
    /// Whether [`Admin::mailer`] has been called to replace the
    /// default `LogMailer`. Used by the R1 commit #9 strict-mailer
    /// boot guard to decide whether the project's deployment is
    /// production-ready (see [`Admin::has_custom_mailer`]). Flipped
    /// to `true` on any call to `mailer(...)`, including a call
    /// that re-registers a `LogMailer` instance — explicit operator
    /// override is enough; the framework does not peek inside the
    /// trait object.
    pub(crate) mailer_overridden: bool,
    /// The active password policy. Defaults to
    /// [`DefaultPasswordPolicy::new`] (`min_len = 10`); projects
    /// override via [`Admin::password_policy`]. Read by R1's reset
    /// consume flow (commit #7) and the corrected `do_password_change`
    /// (commit #11) so a single source of truth governs every
    /// password write across the framework. Held as
    /// `Arc<dyn PasswordPolicy>` for the same reason as the mailer
    /// above (cheap clone, Send + Sync).
    pub(crate) password_policy: SharedPasswordPolicy,
    /// The active recovery policy: reset-token TTL, rate-limit
    /// shape, strict-mailer boot guard, public-site-URL derivation.
    /// Defaults to [`DefaultRecoveryPolicy::new`]; projects override
    /// via [`Admin::recovery_policy`]. Read by R1's recovery
    /// handlers (commits #7–#9). Held as `Arc<dyn RecoveryPolicy>`
    /// — same architectural pattern as the mailer and the password
    /// policy above.
    pub(crate) recovery_policy: SharedRecoveryPolicy,
    /// The active MFA enforcement policy. Defaults to
    /// [`MfaPolicy::Optional`]; projects opt into enforcement via
    /// [`Admin::require_mfa`]. Plain `Copy` enum (no `Arc`
    /// indirection) — the four variants encode every operator
    /// choice: rejected (`Disabled`), opt-in (`Optional`),
    /// universal (`Required`), or per-role (`RequiredForRoles`).
    /// The `login_guard` consults this field after successful
    /// password verification (R3 commit #15); this commit lands
    /// the data, the routing follows.
    pub(crate) mfa_policy: MfaPolicy,
    /// Storage root for uploaded files. `None` (default) disables
    /// the file-upload code path entirely — any
    /// `<input type="file">` widget on a model whose framework
    /// was built without this set is treated as inert (the form
    /// renders, but the multipart handler returns an empty
    /// `Form::set` for that field). Projects with file-bearing
    /// columns opt in via [`Admin::uploads_dir`]. The directory
    /// is created lazily on first write; the framework
    /// canonicalises it to refuse path-traversal on the serve
    /// route.
    pub(crate) uploads_dir: Option<std::path::PathBuf>,
    /// `true` puts the admin into whole-admin read-only mode: every
    /// mutating POST under `/admin/*` (project CRUD, bulk actions,
    /// admin-driven user lifecycle) returns 403 with a flash banner;
    /// auth-flow POSTs (login, logout, mfa verify, password recovery,
    /// own-session management) are explicitly allowlisted so the
    /// operator can still get in and out. Useful for incident
    /// response (lock the admin while investigating) and demo
    /// environments where you want viewers but not editors. Off by
    /// default. Project authors opt in via [`Admin::read_only`].
    pub(crate) read_only: bool,
    /// Per-model read-only set: when an admin slug is present here,
    /// the `read_only_guard` middleware returns 403 on mutating
    /// requests targeting `/admin/<slug>/...` while leaving the rest
    /// of the admin live. Off by default — set via
    /// [`Admin::read_only_model`]. Coexists with whole-admin
    /// [`Self::read_only`]: a model frozen here stays frozen even
    /// when the admin is otherwise writable.
    pub(crate) read_only_models: HashSet<String>,
}

impl Default for Admin {
    fn default() -> Self {
        Self::new()
    }
}

impl Admin {
    // public:
    /// Constructs a new `Admin` with the framework's core entries
    /// pre-seeded. The only core entry is `User`; project models are
    /// added on top via [`Self::model`]. The outbound mailer
    /// defaults to [`LogMailer`] — safe for dev / CI / testing,
    /// **not suitable for production** (recovery emails are written
    /// to `log::info!` instead of being sent). Projects opt into a
    /// real mailer via [`Self::mailer`].
    pub fn new() -> Self {
        Self {
            entries: vec![core_user_entry()],
            site_branding: SiteBranding::default(),
            user_profile_ext: None,
            theme: AdminTheme::default(),
            mailer: Arc::new(LogMailer),
            mailer_overridden: false,
            password_policy: Arc::new(DefaultPasswordPolicy::new()),
            recovery_policy: Arc::new(DefaultRecoveryPolicy::new()),
            mfa_policy: MfaPolicy::default(),
            uploads_dir: None,
            read_only: false,
            read_only_models: HashSet::new(),
        }
    }

    // public:
    /// Replace the entire [`SiteBranding`] block. For finer-grained
    /// adjustments, use the per-field builders below
    /// ([`Admin::app_name`], [`Admin::app_tagline`], …).
    pub fn site_branding(mut self, branding: SiteBranding) -> Self {
        self.site_branding = branding;
        self
    }

    // public:
    /// Set the user-facing product identity. **Recommended for every
    /// production deployment** — the framework name "RustIO" should
    /// not appear in operational user surfaces.
    ///
    /// Example: `Admin::new().app_name("Library Circulation")`.
    /// Also mirrors the value into the legacy `site_title` /
    /// `site_header` fields so older paths that still read them stay
    /// coherent with the new identity.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        let n = name.into();
        self.site_branding.app_name = n.clone();
        // Mirror into the legacy fields so any old read path stays
        // consistent with the canonical name. Projects that override
        // `site_branding` directly bypass this mirroring.
        self.site_branding.site_title = n.clone();
        self.site_branding.site_header = n;
        self
    }

    // public:
    /// Set an optional secondary line shown under the brand
    /// wordmark in emails + auth pages.
    pub fn app_tagline(mut self, tagline: impl Into<String>) -> Self {
        self.site_branding.app_tagline = Some(tagline.into());
        self
    }

    // public:
    /// Set the support contact surfaced in recovery emails.
    pub fn support_email(mut self, email: impl Into<String>) -> Self {
        self.site_branding.support_email = Some(email.into());
        self
    }

    // public:
    /// Set the canonical public URL — used as a base when composing
    /// reset links if request-header derivation is unreliable.
    pub fn public_url(mut self, url: impl Into<String>) -> Self {
        self.site_branding.public_url = Some(url.into());
        self
    }

    // public:
    /// Opt in to the small "Powered by RustIO" credit in chrome
    /// footer + email footer. Off by default; the framework name
    /// stays invisible to end users unless this is enabled.
    pub fn show_powered_by(mut self, show: bool) -> Self {
        self.site_branding.show_powered_by = show;
        self
    }

    // public:
    /// Whole-admin read-only toggle. When `true` every mutating
    /// `POST` / `PUT` / `DELETE` under `/admin/*` is rejected with
    /// 403 by the `read_only_guard` middleware; auth-flow routes
    /// (login, logout, MFA verify, password recovery, own-session
    /// management, own MFA management) are explicitly allowlisted
    /// so operators can still sign in / out and complete forced
    /// rotations. Templates that read [`Self::is_read_only`]
    /// surface a banner and hide top-level "Add" affordances; per-
    /// row Edit / Delete buttons still render in v1 (clicking
    /// through hits the middleware), documented as a known
    /// scoping trade-off.
    pub fn read_only(mut self, on: bool) -> Self {
        self.read_only = on;
        self
    }

    // public:
    /// `true` when the admin was constructed with [`Self::read_only`].
    /// Consumed by the chrome (banner) and the list/dashboard
    /// templates (suppress "Add" buttons).
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    // public:
    /// Freeze one model. Mutating requests under `/admin/<admin_name>/...`
    /// return 403; the rest of the admin stays writable. Useful for
    /// archive tables, regulatory holds, or per-model incident
    /// response without flipping the whole admin to read-only.
    ///
    /// `admin_name` is the model's URL slug (the same value the
    /// router matches `:admin_name` against — pluralised, e.g.
    /// `"posts"`, `"users"`). Wrong slugs silently no-op; the
    /// middleware checks set membership, so a typo simply doesn't
    /// freeze anything.
    pub fn read_only_model(mut self, admin_name: impl Into<String>) -> Self {
        self.read_only_models.insert(admin_name.into());
        self
    }

    // public:
    /// `true` when `admin_name` was registered via
    /// [`Self::read_only_model`]. The `read_only_guard` middleware
    /// consults this to gate per-model mutations.
    pub fn is_model_read_only(&self, admin_name: &str) -> bool {
        self.read_only_models.contains(admin_name)
    }

    // public:
    /// Set the storage root for uploaded files. Models declaring
    /// `#[rustio(file)]` columns persist relative paths under this
    /// directory; the framework serves the bytes back via
    /// `GET /admin/uploads/<rel>`. The directory is created lazily
    /// on first write. Leaving this unset (the default) makes any
    /// file-input field inert at submit — the form renders, but
    /// the multipart parse path skips file writes.
    ///
    /// **Path safety contract.** The framework canonicalises the
    /// configured root once at builder time and refuses any
    /// serve-route lookup whose resolved path lands outside the
    /// canonical root; rejected lookups return 404 (no
    /// information leak about whether the path could exist).
    pub fn uploads_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.uploads_dir = Some(dir.into());
        self
    }

    // public:
    /// Read-only access to the configured uploads root, if any.
    pub fn uploads_dir_path(&self) -> Option<&std::path::Path> {
        self.uploads_dir.as_deref()
    }

    // public:
    /// Read-only access to the active branding.
    pub fn branding(&self) -> &SiteBranding {
        &self.site_branding
    }

    // public:
    /// Set the admin chrome's accent colour. Hex form, with or without
    /// the leading `#` (`"#1e6ba8"` and `"1e6ba8"` both work). Replaces
    /// any prior accent override; other [`AdminTheme`] fields are
    /// left untouched.
    pub fn accent_color(mut self, color: impl Into<String>) -> Self {
        self.theme.accent = Some(normalise_hex(color));
        self
    }

    // public:
    /// Replace the entire admin chrome palette patch in one call. See
    /// [`AdminTheme`] for the field-by-field contract.
    pub fn theme(mut self, theme: AdminTheme) -> Self {
        self.theme = theme;
        self
    }

    // public:
    /// Read-only access to the configured accent colour, if any. `None`
    /// means *“no override — admin.css owns it”*.
    pub fn accent(&self) -> Option<&str> {
        self.theme.accent.as_deref()
    }

    // public:
    /// Read-only access to the active theme override patch.
    pub fn active_theme(&self) -> &AdminTheme {
        &self.theme
    }

    // public:
    /// Replace the outbound mailer. Closes the
    /// documented-but-unimplemented gap from 0.4.0 where the doc
    /// comments described this method while the `Admin` struct had
    /// no mailer field; landed in 0.5.0 alongside the R1 recovery
    /// pipeline that consumes it (`DESIGN_RECOVERY.md` §10.3).
    ///
    /// Typical project wiring:
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// let admin = Admin::new()
    ///     .mailer(Arc::new(MyProjectMailer::new(/* SES, Mailgun, … */)));
    /// ```
    ///
    /// The framework imposes no transport. Anything that implements
    /// the [`crate::email::Mailer`] trait (which is `Send + Sync`
    /// and async-friendly) plugs in here. R1's recovery flow reads
    /// this via [`Self::active_mailer`] and dispatches reset
    /// emails through it.
    pub fn mailer(mut self, mailer: SharedMailer) -> Self {
        self.mailer = mailer;
        self.mailer_overridden = true;
        self
    }

    // public:
    /// Read-only access to the registered mailer. Returns a borrow
    /// of the `Arc` so handlers can `.clone()` it cheaply when they
    /// need to move the handle into an async future. Always returns
    /// a live mailer — `Admin::new()` seeds [`LogMailer`] as the
    /// default, so this never returns `None`.
    pub fn active_mailer(&self) -> &SharedMailer {
        &self.mailer
    }

    // public:
    /// Whether the project explicitly called [`Self::mailer`] to
    /// register a mailer. Returns `false` for `Admin::new()` (the
    /// framework's `LogMailer` default is in place); flips to `true`
    /// on any subsequent call to `mailer(...)`, regardless of the
    /// concrete type supplied — the framework trusts the operator's
    /// explicit override.
    ///
    /// Read by the R1 strict-mailer boot guard: when
    /// `RecoveryPolicy::strict_mailer_required() == true` and this
    /// returns `false`, `register_admin_routes` panics at startup
    /// rather than registering the recovery routes against a
    /// production-unsafe default mailer.
    pub fn has_custom_mailer(&self) -> bool {
        self.mailer_overridden
    }

    // public:
    /// Replace the active password policy. R1 ships with the
    /// length-only [`DefaultPasswordPolicy`] (`min_len = 10`);
    /// production deployments commonly override to 12+, and
    /// regulated deployments may ship a full custom impl with breach
    /// blocklists or organisational complexity rules
    /// (`DESIGN_RECOVERY.md` §13).
    ///
    /// Typical project wiring:
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use rustio_admin::auth::DefaultPasswordPolicy;
    ///
    /// let admin = Admin::new()
    ///     .password_policy(Arc::new(DefaultPasswordPolicy::with_min_len(16)));
    /// ```
    pub fn password_policy(mut self, policy: SharedPasswordPolicy) -> Self {
        self.password_policy = policy;
        self
    }

    // public:
    /// Read-only access to the registered password policy. Returns
    /// a borrow of the `Arc` so handlers can `.clone()` it cheaply
    /// when needed. Always returns a live policy — `Admin::new()`
    /// seeds [`DefaultPasswordPolicy`] so this never returns `None`.
    pub fn active_password_policy(&self) -> &SharedPasswordPolicy {
        &self.password_policy
    }

    // public:
    /// Replace the active recovery policy. R1 ships with
    /// [`DefaultRecoveryPolicy`] (TTL 1h, request 5/15min, consume
    /// 10/5min, strict-mailer guard off); production deployments
    /// commonly opt into the strict guard via
    /// `with_strict_mailer_required(true)` after registering a real
    /// mailer (`DESIGN_RECOVERY.md` §12).
    ///
    /// Typical project wiring:
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use rustio_admin::auth::DefaultRecoveryPolicy;
    ///
    /// let admin = Admin::new()
    ///     .recovery_policy(Arc::new(
    ///         DefaultRecoveryPolicy::new()
    ///             .with_strict_mailer_required(true),
    ///     ));
    /// ```
    pub fn recovery_policy(mut self, policy: SharedRecoveryPolicy) -> Self {
        self.recovery_policy = policy;
        self
    }

    // public:
    /// Read-only access to the registered recovery policy. Returns
    /// a borrow of the `Arc`. Always live — `Admin::new()` seeds
    /// [`DefaultRecoveryPolicy`] so this never returns `None`.
    pub fn active_recovery_policy(&self) -> &SharedRecoveryPolicy {
        &self.recovery_policy
    }

    // public:
    /// Replace the active MFA enforcement policy. R3 ships with
    /// [`MfaPolicy::Optional`] as the default — pre-R3 framework
    /// behaviour, no opt-in required. Production deployments that
    /// want MFA enforcement opt in via this builder.
    ///
    /// **Forward-only enforcement (D6).** Switching to
    /// [`MfaPolicy::Required`] does NOT retroactively revoke
    /// existing sessions; the `login_guard` redirects users
    /// without MFA to `/admin/mfa/enroll` at the next request.
    /// The pattern mirrors R2's `must_change_password`
    /// interstitial (`DESIGN_R3_MFA.md` §12.3).
    ///
    /// **Boot guard (D1).** When `MfaPolicy != Disabled`, the
    /// framework refuses to boot if `RUSTIO_SECRET_KEY` is
    /// unset — the env var is required for AES-256-GCM
    /// encryption of TOTP secrets at rest. The boot check lands
    /// in a later R3 commit; this builder records the policy
    /// without the check.
    ///
    /// Typical project wiring:
    ///
    /// ```ignore
    /// use rustio_admin::auth::{MfaPolicy, Role};
    ///
    /// // Universal:
    /// let admin = Admin::new().require_mfa(MfaPolicy::Required);
    ///
    /// // Privileged roles only:
    /// const PRIVILEGED: &[Role] = &[Role::Administrator, Role::Supervisor];
    /// let admin = Admin::new()
    ///     .require_mfa(MfaPolicy::RequiredForRoles(PRIVILEGED));
    /// ```
    pub fn require_mfa(mut self, policy: MfaPolicy) -> Self {
        self.mfa_policy = policy;
        self
    }

    // public:
    /// Read-only access to the active MFA policy. Returns by
    /// value — the policy is `Copy`. Always live — `Admin::new()`
    /// seeds [`MfaPolicy::default`] (`Optional`) so this never
    /// returns `None`.
    pub fn active_mfa_policy(&self) -> MfaPolicy {
        self.mfa_policy
    }

    // public:
    pub fn model<M>(mut self) -> Self
    where
        M: super::ModelAdmin + crate::orm::Model,
    {
        // Loud-fail the FTS footgun: `search_index_column()` is only used
        // by the list query when the column is also in `Model::COLUMNS`
        // (injection-safety, see `ops::ConcreteOps::list`). If it isn't,
        // search silently degrades to a full `ILIKE` scan — so warn at
        // registration rather than leave the operator guessing.
        if let Some(col) = M::search_index_column() {
            if !<M as crate::orm::Model>::COLUMNS.contains(&col) {
                log::warn!(
                    "admin model `{}`: search_index_column() = {col:?} is not in Model::COLUMNS, \
                     so full-text search is disabled and the list falls back to ILIKE. \
                     Add {col:?} to COLUMNS to enable FTS.",
                    M::ADMIN_NAME
                );
            }
        }

        let ops: Arc<dyn AdminOps> = Arc::new(super::ops::ConcreteOps::<M>::new());
        self.entries.push(AdminEntry {
            admin_name: M::ADMIN_NAME,
            display_name: M::DISPLAY_NAME,
            singular_name: M::SINGULAR_NAME,
            table: <M as crate::orm::Model>::TABLE,
            fields: M::FIELDS,
            core: false,
            list_display: M::list_display(),
            list_filter: M::list_filter(),
            search_fields: M::search_fields(),
            search_index_column: M::search_index_column(),
            ordering: M::ordering(),
            list_per_page: M::list_per_page(),
            readonly_fields: M::readonly_fields(),
            fieldsets: M::fieldsets(),
            bulk_actions: M::bulk_actions(),
            inlines: M::inlines(),
            ops,
        });
        self
    }

    // public:
    pub fn entries(&self) -> &[AdminEntry] {
        &self.entries
    }

    // public:
    /// Register a project-specific extension that contributes extra
    /// sections to the built-in user profile page. The closure is
    /// invoked on every render of `GET /admin/users/:id` (Overview tab);
    /// it receives the `Db` handle and the loaded
    /// [`crate::auth::UserProfile`] (no `password_hash`) and returns a
    /// `Vec<UserProfileSection>`. Sections render in the order returned,
    /// immediately after the core profile show-grid.
    ///
    /// Zero-config baseline: don't call this method, and the extension
    /// area stays empty. Projects that need richer layout than key-value
    /// rows override the `{% block project_user_fields %}` template
    /// block in `templates/admin/user_view.html` instead.
    pub fn user_profile_extension<F, Fut>(mut self, ext: F) -> Self
    where
        F: Fn(Db, crate::auth::UserProfile) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<UserProfileSection>>> + Send + 'static,
    {
        self.user_profile_ext = Some(Arc::new(move |db, user| Box::pin(ext(db, user))));
        self
    }

    /// Internal accessor — handlers fetch the registered extension
    /// closure (if any) here. Used by `admin/builtin.rs` (P6.b).
    #[allow(dead_code)]
    pub(crate) fn user_profile_ext(&self) -> Option<&UserProfileExtensionFn> {
        self.user_profile_ext.as_ref()
    }

    // public:
    pub fn find(&self, admin_name: &str) -> Option<&AdminEntry> {
        self.entries.iter().find(|e| e.admin_name == admin_name)
    }

    // public:
    /// Register the canonical (add/change/delete/view) permissions for
    /// every model. Call during startup after `init_tables`.
    ///
    /// Fast-path: the per-model `INSERT`s below are idempotent but cost a
    /// burst of round-trips on every boot. We stamp a fingerprint of
    /// `(crate version + sorted model set)` into `rustio_admin_meta`; when
    /// it already matches we skip the loop. Adding/removing a model
    /// changes the fingerprint and re-seeds; a framework upgrade does too.
    /// To force a re-seed, `TRUNCATE rustio_admin_meta`.
    pub async fn seed_permissions(&self, db: &crate::orm::Db) -> crate::error::Result<()> {
        const STAMP_KEY: &str = "permissions_fingerprint";
        let mut names: Vec<&str> = self.entries.iter().map(|e| e.admin_name).collect();
        names.sort_unstable();
        let fingerprint = format!("{}|{}", env!("CARGO_PKG_VERSION"), names.join(","));
        crate::meta::ensure_table(db).await?;
        if crate::meta::get(db, STAMP_KEY).await?.as_deref() == Some(fingerprint.as_str()) {
            return Ok(());
        }

        for entry in &self.entries {
            let singular = entry.singular_name.to_ascii_lowercase();
            crate::auth::register_model_permissions(db, entry.admin_name, &singular).await?;
            // PR 2.2 — grant the four model-CRUD permissions to the
            // three default groups per the grant matrix in
            // `auth::permissions::grant_model_to_default_groups`. No-op
            // when the seeded groups are absent (user-defined-groups
            // guard fired in `seed_default_groups`).
            crate::auth::grant_model_to_default_groups(db, entry.admin_name, &singular).await?;
        }
        crate::meta::set(db, STAMP_KEY, &fingerprint).await?;
        Ok(())
    }
}

// -------------------------------------------------------------------------
// Core User entry — synthetic, route-only stub
// -------------------------------------------------------------------------
//
// Every project's admin index lists `Users` so operators can navigate
// to the bespoke `/admin/users/*` pages owned by `admin::builtin`. The
// `User` entry is built directly here rather than implementing
// `AdminModel` on a placeholder struct: the auth subsystem already
// owns the live `/admin/users` page with its own logic; routing
// through generic CRUD here would spawn a duplicate page.

const CORE_USER_FIELDS: &[AdminField] = &[
    AdminField {
        name: "id",
        label: "id",
        field_type: FieldType::I64,
        editable: false,
        relation: None,
        choices: None,
    },
    AdminField {
        name: "email",
        label: "email",
        field_type: FieldType::String,
        editable: true,
        relation: None,
        choices: None,
    },
    AdminField {
        name: "password_hash",
        label: "password_hash",
        field_type: FieldType::String,
        editable: false,
        relation: None,
        choices: None,
    },
    AdminField {
        name: "role",
        label: "role",
        field_type: FieldType::String,
        editable: true,
        relation: None,
        choices: None,
    },
    AdminField {
        name: "is_active",
        label: "is_active",
        field_type: FieldType::Bool,
        editable: true,
        relation: None,
        choices: None,
    },
    AdminField {
        name: "created_at",
        label: "created_at",
        field_type: FieldType::DateTime,
        editable: false,
        relation: None,
        choices: None,
    },
];

/// Normalise a user-supplied colour string to `#rrggbb` form. Accepts
/// both `"#1e6ba8"` and `"1e6ba8"`; trims whitespace; does NOT validate
/// that the body is hex (that's the renderer's job, where invalid
/// values fall back to the framework default rather than panic). The
/// `format!()` adds back exactly one leading `#`.
pub(crate) fn normalise_hex(input: impl Into<String>) -> String {
    let raw = input.into();
    let trimmed = raw.trim().trim_start_matches('#');
    format!("#{trimmed}")
}

fn core_user_entry() -> AdminEntry {
    AdminEntry {
        admin_name: "users",
        display_name: "Users",
        singular_name: "User",
        table: "rustio_users",
        fields: CORE_USER_FIELDS,
        core: true,
        list_display: &[],
        list_filter: &[],
        search_fields: &[],
        search_index_column: None,
        ordering: &["-id"],
        list_per_page: 50,
        readonly_fields: &[],
        fieldsets: &[],
        bulk_actions: &[],
        inlines: &[],
        ops: Arc::new(CoreUserOps),
    }
}

/// Route-only stub for the synthetic User entry. The live
/// `/admin/users` page is wired separately by `admin::builtin`, so
/// every method here returns a dedicated error rather than silently
/// half-working. If the generic admin ever routes to this, the error
/// makes the misuse obvious.
struct CoreUserOps;

fn core_user_route_error() -> crate::error::Error {
    crate::error::Error::Internal(
        "the core User entry is route-only — use the dedicated /admin/users page".into(),
    )
}

impl AdminOps for CoreUserOps {
    fn list<'a>(
        &'a self,
        _db: &'a Db,
        _opts: ListOpts,
    ) -> Pin<Box<dyn Future<Output = Result<ListPage>> + Send + 'a>> {
        Box::pin(async { Err(core_user_route_error()) })
    }

    fn find_row<'a>(
        &'a self,
        _db: &'a Db,
        _id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<EditRow>>> + Send + 'a>> {
        Box::pin(async { Err(core_user_route_error()) })
    }

    fn create<'a>(&'a self, _db: &'a Db, _form: &'a FormData) -> CreateResult<'a> {
        Box::pin(async { Err(core_user_route_error()) })
    }

    fn update<'a>(&'a self, _db: &'a Db, _id: i64, _form: &'a FormData) -> UpdateResult<'a> {
        Box::pin(async { Err(core_user_route_error()) })
    }

    fn delete<'a>(
        &'a self,
        _db: &'a Db,
        _id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { Err(core_user_route_error()) })
    }

    fn object_label<'a>(
        &'a self,
        _db: &'a Db,
        _id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'a>> {
        Box::pin(async { Err(core_user_route_error()) })
    }

    fn execute_bulk_action<'a>(
        &'a self,
        _db: &'a Db,
        name: &'a str,
        _ids: &'a [i64],
        _ctx: &'a super::bulk::BulkActionContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<super::bulk::BulkActionResult>> + Send + 'a>> {
        // The User entry has no project-owned bulk actions; the
        // framework manages user row writes through dedicated CLI /
        // admin routes. Surface the same "no project handler" shape
        // as the ModelAdmin default so a misconfigured registration
        // gets a clear error rather than a silent no-op.
        let owned = name.to_string();
        Box::pin(async move {
            Err(crate::error::Error::BadRequest(format!(
                "bulk action `{owned}` is not available on the framework User entry"
            )))
        })
    }
}

// Test fixtures (PanicOps / FailingOps + AdminEntry::for_testing*) live
// with the legacy `admin/macro_tests.rs` etc. that haven't been ported
// yet. Re-add them here when the first in-tree test needs them.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{PasswordPolicy, PasswordPolicyError};

    #[test]
    fn admin_new_installs_default_password_policy() {
        let admin = Admin::new();
        // Default floor is 10 (per DESIGN_RECOVERY.md §13.2).
        assert_eq!(admin.active_password_policy().min_length(), 10);
        // Sanity: a 9-char password is rejected, a 10-char is accepted.
        assert!(admin
            .active_password_policy()
            .validate("nine_char")
            .is_err());
        assert!(admin
            .active_password_policy()
            .validate("ten_chars_")
            .is_ok());
    }

    #[test]
    fn admin_password_policy_overrides_default() {
        struct StubPolicy;
        impl PasswordPolicy for StubPolicy {
            fn validate(&self, _candidate: &str) -> std::result::Result<(), PasswordPolicyError> {
                Err(PasswordPolicyError::Custom("stub rejected".into()))
            }
            fn min_length(&self) -> usize {
                99
            }
        }

        let admin = Admin::new().password_policy(Arc::new(StubPolicy));
        assert_eq!(admin.active_password_policy().min_length(), 99);
        let err = admin
            .active_password_policy()
            .validate("anything-at-all-here")
            .unwrap_err();
        assert_eq!(err, PasswordPolicyError::Custom("stub rejected".into()));
    }

    #[test]
    fn admin_new_installs_default_recovery_policy() {
        let admin = Admin::new();
        let p = admin.active_recovery_policy();
        // Locked defaults from DESIGN_RECOVERY.md §17.
        assert_eq!(p.reset_token_ttl(), chrono::Duration::hours(1));
        assert_eq!(
            p.request_rate_limit(),
            (5, std::time::Duration::from_secs(15 * 60))
        );
        assert_eq!(
            p.consume_rate_limit(),
            (10, std::time::Duration::from_secs(5 * 60))
        );
        assert!(!p.strict_mailer_required());
    }

    #[test]
    fn admin_new_has_no_custom_mailer() {
        let admin = Admin::new();
        assert!(!admin.has_custom_mailer());
    }

    #[test]
    fn admin_mailer_builder_flips_override_flag() {
        // Even when the override happens to register another LogMailer,
        // the explicit call is what the strict-mailer guard reads.
        let admin = Admin::new().mailer(Arc::new(crate::email::LogMailer));
        assert!(admin.has_custom_mailer());
    }

    #[test]
    fn admin_recovery_policy_overrides_default() {
        use crate::auth::RecoveryPolicy;

        struct StubRecoveryPolicy;
        impl RecoveryPolicy for StubRecoveryPolicy {
            fn reset_token_ttl(&self) -> chrono::Duration {
                chrono::Duration::hours(2)
            }
            fn request_rate_limit(&self) -> (u32, std::time::Duration) {
                (1, std::time::Duration::from_secs(60))
            }
            fn consume_rate_limit(&self) -> (u32, std::time::Duration) {
                (2, std::time::Duration::from_secs(120))
            }
            fn strict_mailer_required(&self) -> bool {
                true
            }
            // public_site_url uses the trait's provided default.
        }

        let admin = Admin::new().recovery_policy(Arc::new(StubRecoveryPolicy));
        let p = admin.active_recovery_policy();
        assert_eq!(p.reset_token_ttl(), chrono::Duration::hours(2));
        assert_eq!(
            p.request_rate_limit(),
            (1, std::time::Duration::from_secs(60))
        );
        assert_eq!(
            p.consume_rate_limit(),
            (2, std::time::Duration::from_secs(120))
        );
        assert!(p.strict_mailer_required());
    }
}

/// End-to-end coverage for the `float` / `date` / `time` / `decimal` /
/// `uuid` field types.
///
/// Deriving `RustioAdmin` on a struct that uses `f64` / `NaiveDate` /
/// `NaiveTime` / `Decimal` / `Uuid` is the only thing in this crate
/// that instantiates the macro's `classify_type`, `display_values`,
/// and `from_form` arms for these Rust types — no other in-crate model
/// uses them, so without this fixture a bug in those expansions would
/// ship uncaught.
#[cfg(test)]
mod scalar_field_type_tests {
    use super::FieldType;
    use crate::admin::AdminModel;
    use crate::http::FormData;
    use crate::RustioAdmin;
    use chrono::{NaiveDate, NaiveTime};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use uuid::Uuid;

    #[derive(Debug, RustioAdmin)]
    struct Measurement {
        id: i64,
        weight_kg: f64,
        unit_price: Decimal,
        taken_on: NaiveDate,
        taken_at: NaiveTime,
        public_id: Uuid,
        #[rustio(format = "email")]
        contact_email: String,
        #[rustio(format = "phone")]
        contact_phone: String,
        #[rustio(choices = ["active", "archived"])]
        status: String,
    }

    const SAMPLE_UUID: &str = "550e8400-e29b-41d4-a716-446655440000";

    fn field_type(name: &str) -> FieldType {
        Measurement::FIELDS
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("field {name} present"))
            .field_type
    }

    #[test]
    fn classify_maps_each_rust_type_to_its_field_type() {
        assert_eq!(field_type("weight_kg"), FieldType::F64);
        assert_eq!(field_type("unit_price"), FieldType::Decimal);
        assert_eq!(field_type("taken_on"), FieldType::Date);
        assert_eq!(field_type("taken_at"), FieldType::Time);
        assert_eq!(field_type("public_id"), FieldType::Uuid);
        // email / phone are `String` in Rust — the `#[rustio(format)]`
        // attribute is the only thing that distinguishes them.
        assert_eq!(field_type("contact_email"), FieldType::Email);
        assert_eq!(field_type("contact_phone"), FieldType::Phone);
        // A choice column stays `String` at the type level — the
        // `<select>` is driven by `AdminField.choices`, not FieldType.
        assert_eq!(field_type("status"), FieldType::String);
    }

    #[test]
    fn choice_field_populates_adminfield_choices() {
        let status = Measurement::FIELDS
            .iter()
            .find(|f| f.name == "status")
            .expect("status field present");
        assert_eq!(status.choices, Some(&["active", "archived"][..]));
        // Fields without `#[rustio(choices)]` carry no choice list.
        let email = Measurement::FIELDS
            .iter()
            .find(|f| f.name == "contact_email")
            .unwrap();
        assert_eq!(email.choices, None);
    }

    #[test]
    fn widgets_follow_html_input_types() {
        assert_eq!(FieldType::F64.widget(), "number");
        assert_eq!(FieldType::Decimal.widget(), "number");
        assert_eq!(FieldType::Date.widget(), "date");
        assert_eq!(FieldType::Time.widget(), "time");
        assert_eq!(FieldType::Uuid.widget(), "text");
        assert_eq!(FieldType::Email.widget(), "email");
        assert_eq!(FieldType::Phone.widget(), "tel");
    }

    #[test]
    fn from_form_parses_valid_scalars() {
        let form = FormData::from_urlencoded(&format!(
            "weight_kg=72.5&unit_price=19.99&taken_on=2026-06-02&taken_at=09:30&public_id={SAMPLE_UUID}\
             &contact_email=alice@example.com&contact_phone=%2B1%20555-123-4567&status=active"
        ));
        let m = Measurement::from_form(&form).expect("valid form parses");
        assert_eq!(m.weight_kg, 72.5);
        assert_eq!(m.unit_price, Decimal::from_str("19.99").unwrap());
        assert_eq!(m.taken_on, NaiveDate::from_ymd_opt(2026, 6, 2).unwrap());
        assert_eq!(m.taken_at, NaiveTime::from_hms_opt(9, 30, 0).unwrap());
        assert_eq!(m.public_id, Uuid::parse_str(SAMPLE_UUID).unwrap());
        assert_eq!(m.contact_email, "alice@example.com");
        assert_eq!(m.contact_phone, "+1 555-123-4567");
        assert_eq!(m.status, "active");
    }

    #[test]
    fn display_values_use_html_roundtrip_formats() {
        let m = Measurement {
            id: 1,
            weight_kg: 72.5,
            unit_price: Decimal::from_str("19.99").unwrap(),
            taken_on: NaiveDate::from_ymd_opt(2026, 6, 2).unwrap(),
            taken_at: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            public_id: Uuid::parse_str(SAMPLE_UUID).unwrap(),
            contact_email: "alice@example.com".to_string(),
            contact_phone: "+1 555-123-4567".to_string(),
            status: "active".to_string(),
        };
        let vals: std::collections::HashMap<String, String> =
            m.display_values().into_iter().collect();
        assert_eq!(vals["weight_kg"], "72.5");
        assert_eq!(vals["unit_price"], "19.99");
        assert_eq!(vals["taken_on"], "2026-06-02");
        assert_eq!(vals["taken_at"], "09:30");
        assert_eq!(vals["public_id"], SAMPLE_UUID);
        assert_eq!(vals["contact_email"], "alice@example.com");
        assert_eq!(vals["contact_phone"], "+1 555-123-4567");
        assert_eq!(vals["status"], "active");
    }

    #[test]
    fn from_form_rejects_invalid_scalars() {
        let form = FormData::from_urlencoded(
            "weight_kg=heavy&unit_price=cheap&taken_on=not-a-date&taken_at=99%3A99&public_id=not-a-uuid\
             &contact_email=nope&contact_phone=call-me&status=deleted",
        );
        let errs = Measurement::from_form(&form).unwrap_err();
        // 7 malformed scalars + 1 out-of-set choice value.
        assert_eq!(errs.len(), 8, "one error per malformed field: {errs:?}");
    }

    #[test]
    fn choice_from_form_accepts_in_set_rejects_out_of_set() {
        let base = format!(
            "weight_kg=1&unit_price=1&taken_on=2026-06-02&taken_at=00:00&public_id={SAMPLE_UUID}\
             &contact_email=a@b.co&contact_phone=0701234567"
        );
        // In-set value parses clean.
        let ok = FormData::from_urlencoded(&format!("{base}&status=archived"));
        assert_eq!(Measurement::from_form(&ok).unwrap().status, "archived");
        // Out-of-set value is the only error.
        let bad = FormData::from_urlencoded(&format!("{base}&status=pending"));
        let errs = Measurement::from_form(&bad).unwrap_err();
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].contains("must be one of: active, archived"));
    }
}
