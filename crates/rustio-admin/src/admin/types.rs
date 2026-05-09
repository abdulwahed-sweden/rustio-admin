//! The admin's data vocabulary. Kept separate from rendering and
//! handlers so changes here ripple out predictably.

// `for_testing[_failing_list]` + the PanicOps/FailingOps fixtures
// are part of the admin's test surface but no in-tree test exercises
// them yet (the legacy admin/macro_tests etc. land in a follow-up).
// Keep them gated behind cfg(test) elsewhere; allow dead inside that
// gate.
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::auth::{DefaultPasswordPolicy, SharedPasswordPolicy};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldType {
    I32,
    I64,
    Bool,
    String,
    DateTime,
    OptionalI64,
    OptionalString,
    OptionalDateTime,
}

impl FieldType {
    pub fn widget(&self) -> &'static str {
        match self {
            FieldType::Bool => "checkbox",
            FieldType::DateTime | FieldType::OptionalDateTime => "datetime",
            FieldType::I32 | FieldType::I64 | FieldType::OptionalI64 => "number",
            FieldType::String | FieldType::OptionalString => "text",
        }
    }

    pub fn nullable(&self) -> bool {
        matches!(
            self,
            FieldType::OptionalI64 | FieldType::OptionalString | FieldType::OptionalDateTime
        )
    }
}

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
    pub(crate) ops: Arc<dyn AdminOps>,
}

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
    /// Free-text search: `(term, columns)`. The runtime emits
    /// `WHERE (col1::text ILIKE $N OR col2::text ILIKE $N OR …)`
    /// with `$N = '%term%'`. An empty `term` or empty `columns`
    /// leaves the WHERE alone.
    pub search: Option<(String, Vec<String>)>,
    /// `LIMIT $N` for the data query. The COUNT(*) query never
    /// applies it. `None` → no limit.
    pub limit: Option<i64>,
    /// `OFFSET $N` for the data query. `None` or `Some(0)` → no offset.
    pub offset: Option<i64>,
}

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
    /// a per-row loop. The default impl returns `BadRequest` with the
    /// action name embedded — projects override to match on `name`
    /// and apply the work; an unknown name surfaces as a clear error
    /// page rather than a silent no-op.
    ///
    /// Note: the framework's built-in `delete` action is **not**
    /// dispatched through here. It runs through the cascade-aware
    /// `/bulk_delete` route which calls `delete()` per row. Override
    /// `delete` instead if you need custom delete semantics.
    fn execute_bulk_action<'a>(
        &'a self,
        _db: &'a Db,
        name: &'a str,
        _ids: &'a [i64],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        let owned = name.to_string();
        Box::pin(async move {
            Err(crate::error::Error::BadRequest(format!(
                "bulk action `{owned}` has no project handler — override \
                 AdminOps::execute_bulk_action on this model to implement it"
            )))
        })
    }
}

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

/// The raw field values used to pre-fill the edit form.
#[derive(Debug)]
pub struct EditRow {
    #[allow(dead_code)]
    pub id: i64,
    pub values: Vec<(String, String)>,
}

/// Per-project admin branding. Defaults are RustIO-flavoured;
/// projects override via [`Admin::site_branding`].
#[derive(Clone, Debug)]
pub struct SiteBranding {
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
        Self {
            site_title: "RustIO administration".into(),
            site_header: "RustIO administration".into(),
            index_title: "Site administration".into(),
            footer_copyright: format!("RustIO {}", env!("CARGO_PKG_VERSION")),
            domain: "rustio.local".into(),
        }
    }
}

/// Project-level override patch for the admin chrome palette.
///
/// `admin.css` is the single source of truth for the framework's design
/// tokens (light defaults, dark mode, semantic surfaces, typography
/// scale, …). `AdminTheme` is **purely a patch layer**: every field is
/// `Option<String>` and defaults to `None`, meaning *“don’t override —
/// let the stylesheet decide.”* Out of the box the framework emits no
/// inline `<style>` block at all.
///
/// Set a field — usually via the fluent builder methods or
/// [`Admin::accent_color`] — to inject a `--rio-*` custom-property
/// override on every page. Overrides apply across `data-rio-theme`
/// states (system / light / dark) by emitting a multi-state selector
/// after `admin.css`, so they win cascade ties without `!important`.
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
    /// New empty patch — no overrides emitted, `admin.css` wins.
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` when at least one field is set. Used by the renderer to
    /// decide whether to emit the inline `<style>` block at all.
    pub fn has_overrides(&self) -> bool {
        self.accent.is_some()
            || self.bg.is_some()
            || self.surface.is_some()
            || self.text.is_some()
            || self.text_muted.is_some()
            || self.border.is_some()
    }

    /// Override `--rio-accent`. Hex form, `#` optional.
    pub fn accent(mut self, color: impl Into<String>) -> Self {
        self.accent = Some(normalise_hex(color));
        self
    }

    /// Override `--rio-bg` (page canvas).
    pub fn bg(mut self, color: impl Into<String>) -> Self {
        self.bg = Some(normalise_hex(color));
        self
    }

    /// Override `--rio-surface` (cards, topbar, sidebar, table body).
    pub fn surface(mut self, color: impl Into<String>) -> Self {
        self.surface = Some(normalise_hex(color));
        self
    }

    /// Override `--rio-text` (body text colour).
    pub fn text(mut self, color: impl Into<String>) -> Self {
        self.text = Some(normalise_hex(color));
        self
    }

    /// Override `--rio-text-muted` (secondary text, breadcrumb links).
    pub fn text_muted(mut self, color: impl Into<String>) -> Self {
        self.text_muted = Some(normalise_hex(color));
        self
    }

    /// Override `--rio-border` (default divider, card outline).
    pub fn border(mut self, color: impl Into<String>) -> Self {
        self.border = Some(normalise_hex(color));
        self
    }
}

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
    /// no current 0.5.0 code path reads it directly until
    /// `auth::recovery::issue_reset_token` lands in commit #7. Held
    /// as `Arc<dyn Mailer>` so cloning the field is a single
    /// reference-count bump and the field stays trivially Send +
    /// Sync (the trait's supertraits are `Send + Sync`).
    pub(crate) mailer: SharedMailer,
    /// The active password policy. Defaults to
    /// [`DefaultPasswordPolicy::new`] (`min_len = 10`); projects
    /// override via [`Admin::password_policy`]. Read by R1's reset
    /// consume flow (commit #7) and the corrected `do_password_change`
    /// (commit #11) so a single source of truth governs every
    /// password write across the framework. Held as
    /// `Arc<dyn PasswordPolicy>` for the same reason as the mailer
    /// above (cheap clone, Send + Sync).
    pub(crate) password_policy: SharedPasswordPolicy,
}

impl Default for Admin {
    fn default() -> Self {
        Self::new()
    }
}

impl Admin {
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
            password_policy: Arc::new(DefaultPasswordPolicy::new()),
        }
    }

    /// Override the default RustIO branding.
    pub fn site_branding(mut self, branding: SiteBranding) -> Self {
        self.site_branding = branding;
        self
    }

    /// Read-only access to the active branding.
    pub fn branding(&self) -> &SiteBranding {
        &self.site_branding
    }

    /// Set the admin chrome's accent colour. Hex form, with or without
    /// the leading `#` (`"#1e6ba8"` and `"1e6ba8"` both work). Replaces
    /// any prior accent override; other [`AdminTheme`] fields are
    /// left untouched.
    pub fn accent_color(mut self, color: impl Into<String>) -> Self {
        self.theme.accent = Some(normalise_hex(color));
        self
    }

    /// Replace the entire admin chrome palette patch in one call. See
    /// [`AdminTheme`] for the field-by-field contract.
    pub fn theme(mut self, theme: AdminTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Read-only access to the configured accent colour, if any. `None`
    /// means *“no override — admin.css owns it”*.
    pub fn accent(&self) -> Option<&str> {
        self.theme.accent.as_deref()
    }

    /// Read-only access to the active theme override patch.
    pub fn active_theme(&self) -> &AdminTheme {
        &self.theme
    }

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
        self
    }

    /// Read-only access to the registered mailer. Returns a borrow
    /// of the `Arc` so handlers can `.clone()` it cheaply when they
    /// need to move the handle into an async future. Always returns
    /// a live mailer — `Admin::new()` seeds [`LogMailer`] as the
    /// default, so this never returns `None`.
    pub fn active_mailer(&self) -> &SharedMailer {
        &self.mailer
    }

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

    /// Read-only access to the registered password policy. Returns
    /// a borrow of the `Arc` so handlers can `.clone()` it cheaply
    /// when needed. Always returns a live policy — `Admin::new()`
    /// seeds [`DefaultPasswordPolicy`] so this never returns `None`.
    pub fn active_password_policy(&self) -> &SharedPasswordPolicy {
        &self.password_policy
    }

    pub fn model<M>(mut self) -> Self
    where
        M: super::ModelAdmin + crate::orm::Model,
    {
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
            ordering: M::ordering(),
            list_per_page: M::list_per_page(),
            readonly_fields: M::readonly_fields(),
            fieldsets: M::fieldsets(),
            bulk_actions: M::bulk_actions(),
            ops,
        });
        self
    }

    pub fn entries(&self) -> &[AdminEntry] {
        &self.entries
    }

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

    pub fn find(&self, admin_name: &str) -> Option<&AdminEntry> {
        self.entries.iter().find(|e| e.admin_name == admin_name)
    }

    /// Register the canonical (add/change/delete/view) permissions for
    /// every model. Call during startup after `init_tables`.
    pub async fn seed_permissions(&self, db: &crate::orm::Db) -> crate::error::Result<()> {
        for entry in &self.entries {
            let singular = entry.singular_name.to_ascii_lowercase();
            crate::auth::register_model_permissions(db, entry.admin_name, &singular).await?;
        }
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
        ordering: &["-id"],
        list_per_page: 50,
        readonly_fields: &[],
        fieldsets: &[],
        bulk_actions: &[],
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
        assert!(admin.active_password_policy().validate("nine_char").is_err());
        assert!(admin.active_password_policy().validate("ten_chars_").is_ok());
    }

    #[test]
    fn admin_password_policy_overrides_default() {
        struct StubPolicy;
        impl PasswordPolicy for StubPolicy {
            fn validate(
                &self,
                _candidate: &str,
            ) -> std::result::Result<(), PasswordPolicyError> {
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
}
