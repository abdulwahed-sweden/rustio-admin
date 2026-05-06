//! The admin's data vocabulary. Kept separate from rendering and
//! handlers so changes here ripple out predictably.

// P5 lands the data vocabulary + manual runtime; handlers/render that
// drive the `pub(crate)` AdminOps trait, the AdminEntry::ops field, the
// CoreUserOps stub, and the test fixtures don't exist until P6. Until
// then the dead-code lint flags everything internal — silenced module-
// wide, to be removed in P6.
#![allow(dead_code)]

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::Result;
use crate::http::FormData;
use crate::orm::{Db, Value};

type CreateResult<'a> =
    Pin<Box<dyn Future<Output = Result<std::result::Result<i64, Vec<String>>>> + Send + 'a>>;

type UpdateResult<'a> =
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

/// Runtime metadata about one admin-registered model.
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
    pub(crate) ops: Arc<dyn AdminOps>,
}

/// Type-erased CRUD operations. The `Admin::model::<M>()` call captures
/// a concrete `M: AdminModel + Model` and hides it behind this trait so
/// the router can treat every model uniformly.
pub(crate) trait AdminOps: Send + Sync {
    fn list<'a>(
        &'a self,
        db: &'a Db,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ListRow>>> + Send + 'a>>;

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
}

/// A row as shown on the list page.
#[derive(Debug)]
pub struct ListRow {
    pub id: i64,
    pub cells: Vec<String>,
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

/// Full admin chrome palette. Each field maps onto one of the
/// framework's `--rio-*` design tokens defined in `_base.html`, so
/// overriding these values via `Admin::theme(...)` re-skins the
/// entire admin shell without touching CSS.
///
/// Defaults match the framework's current chrome so a project that
/// doesn't call `.theme(...)` renders unchanged.
///
/// Hex form (`#rrggbb` or `rrggbb`); leading `#` is auto-normalised
/// at render time. Malformed values fall back to framework defaults
/// rather than panic — the admin path never breaks over a config typo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdminTheme {
    pub accent: String,
    pub bg: String,
    pub surface: String,
    pub text: String,
    pub text_muted: String,
    pub border: String,
}

impl Default for AdminTheme {
    fn default() -> Self {
        // Cobalt Blue light palette.
        Self {
            accent: "#2563EB".into(),
            bg: "#F4F6FB".into(),
            surface: "#FFFFFF".into(),
            text: "#111827".into(),
            text_muted: "#4B5563".into(),
            border: "#D1D5DB".into(),
        }
    }
}

/// Builder for the admin. Register models with `.model::<M>()`, then
/// hand it to the router via `register_admin_routes`.
pub struct Admin {
    pub(crate) entries: Vec<AdminEntry>,
    pub(crate) site_branding: SiteBranding,
    pub(crate) user_profile_ext: Option<UserProfileExtensionFn>,
    pub(crate) theme: AdminTheme,
}

impl Default for Admin {
    fn default() -> Self {
        Self::new()
    }
}

impl Admin {
    /// Constructs a new `Admin` with the framework's core entries
    /// pre-seeded. The only core entry is `User`; project models are
    /// added on top via [`Self::model`].
    pub fn new() -> Self {
        Self {
            entries: vec![core_user_entry()],
            site_branding: SiteBranding::default(),
            user_profile_ext: None,
            theme: AdminTheme::default(),
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
    /// the leading `#` (`"#1e6ba8"` and `"1e6ba8"` both work).
    pub fn accent_color(mut self, color: impl Into<String>) -> Self {
        self.theme.accent = normalise_hex(color);
        self
    }

    /// Set the entire admin chrome palette in one call. See
    /// [`AdminTheme`] for the field-by-field contract.
    pub fn theme(mut self, theme: AdminTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Read-only access to the configured accent colour (`#rrggbb`).
    pub fn accent(&self) -> &str {
        &self.theme.accent
    }

    /// Read-only access to the active full theme.
    pub fn active_theme(&self) -> &AdminTheme {
        &self.theme
    }

    pub fn model<M>(mut self) -> Self
    where
        M: AdminModel + crate::orm::Model,
    {
        let ops: Arc<dyn AdminOps> = Arc::new(ConcreteOps::<M>::new());
        self.entries.push(AdminEntry {
            admin_name: M::ADMIN_NAME,
            display_name: M::DISPLAY_NAME,
            singular_name: M::SINGULAR_NAME,
            table: <M as crate::orm::Model>::TABLE,
            fields: M::FIELDS,
            core: false,
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
    /// closure (if any) here.
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

// ---- ConcreteOps — the manual runtime ------------------------------------

pub(crate) struct ConcreteOps<M> {
    _marker: std::marker::PhantomData<M>,
}

impl<M> ConcreteOps<M> {
    pub(crate) fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<M> AdminOps for ConcreteOps<M>
where
    M: AdminModel + crate::orm::Model,
{
    fn list<'a>(
        &'a self,
        db: &'a Db,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ListRow>>> + Send + 'a>> {
        Box::pin(async move {
            let rows = crate::orm::all::<M>(db).await?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let id = AdminModel::id(&r);
                    let cells = r.display_values().into_iter().map(|(_, v)| v).collect();
                    ListRow { id, cells }
                })
                .collect())
        })
    }

    fn find_row<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<EditRow>>> + Send + 'a>> {
        Box::pin(async move {
            let found = crate::orm::find::<M>(db, id).await?;
            Ok(found.map(|m| EditRow {
                id: AdminModel::id(&m),
                values: m.display_values(),
            }))
        })
    }

    fn create<'a>(&'a self, db: &'a Db, form: &'a FormData) -> CreateResult<'a> {
        Box::pin(async move {
            match M::from_form(form) {
                Ok(model) => match crate::orm::create(db, &model).await {
                    Ok(id) => Ok(Ok(id)),
                    // Postgres constraint violations route to
                    // `Error::Conflict` via `From<sqlx::Error>`. Catch
                    // them here so the user sees a re-rendered form
                    // with an inline error instead of a 500.
                    Err(crate::error::Error::Conflict(msg)) => {
                        log::warn!("create rejected by DB constraint: {msg}");
                        Ok(Err(vec!["Invalid value or constraint violation. \
                             Please check the highlighted fields and try again."
                            .into()]))
                    }
                    Err(other) => Err(other),
                },
                Err(errs) => Ok(Err(errs)),
            }
        })
    }

    fn update<'a>(&'a self, db: &'a Db, id: i64, form: &'a FormData) -> UpdateResult<'a> {
        Box::pin(async move {
            match M::from_form(form) {
                Ok(model) => match crate::orm::update(db, id, &model).await {
                    Ok(()) => Ok(Ok(())),
                    Err(crate::error::Error::Conflict(msg)) => {
                        log::warn!("update rejected by DB constraint: {msg}");
                        Ok(Err(vec!["Invalid value or constraint violation. \
                             Please check the highlighted fields and try again."
                            .into()]))
                    }
                    Err(other) => Err(other),
                },
                Err(errs) => Ok(Err(errs)),
            }
        })
    }

    fn delete<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move { crate::orm::delete::<M>(db, id).await })
    }

    fn object_label<'a>(
        &'a self,
        db: &'a Db,
        id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'a>> {
        Box::pin(async move {
            let found = crate::orm::find::<M>(db, id).await?;
            Ok(found.map(|m| m.object_label()))
        })
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
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ListRow>>> + Send + 'a>> {
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

// ---- Test fixtures -------------------------------------------------------

#[cfg(test)]
impl AdminEntry {
    /// Build an `AdminEntry` for test fixtures. Fills `ops` with a
    /// `PanicOps` stub; any test that ends up routing CRUD through the
    /// returned entry will panic loudly at the trait method.
    pub(crate) fn for_testing(
        admin_name: &'static str,
        display_name: &'static str,
        singular_name: &'static str,
        table: &'static str,
        fields: &'static [AdminField],
        core: bool,
    ) -> Self {
        Self {
            admin_name,
            display_name,
            singular_name,
            table,
            fields,
            core,
            ops: Arc::new(PanicOps),
        }
    }

    /// Variant of `for_testing` whose `ops.list()` returns an `Err`.
    /// Lets tests exercise resilience paths that catch-and-log without
    /// spinning up Postgres.
    pub(crate) fn for_testing_failing_list(
        admin_name: &'static str,
        display_name: &'static str,
        singular_name: &'static str,
        table: &'static str,
        fields: &'static [AdminField],
    ) -> Self {
        Self {
            admin_name,
            display_name,
            singular_name,
            table,
            fields,
            core: false,
            ops: Arc::new(FailingOps),
        }
    }
}

#[cfg(test)]
struct PanicOps;

#[cfg(test)]
const PANIC_MSG: &str = "PanicOps is test-only; if you hit this, a test is using AdminEntry for CRUD, which is wrong — use a real Model";

#[cfg(test)]
impl AdminOps for PanicOps {
    fn list<'a>(
        &'a self,
        _db: &'a Db,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ListRow>>> + Send + 'a>> {
        Box::pin(async { unreachable!("{PANIC_MSG}") })
    }

    fn find_row<'a>(
        &'a self,
        _db: &'a Db,
        _id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<EditRow>>> + Send + 'a>> {
        Box::pin(async { unreachable!("{PANIC_MSG}") })
    }

    fn create<'a>(&'a self, _db: &'a Db, _form: &'a FormData) -> CreateResult<'a> {
        Box::pin(async { unreachable!("{PANIC_MSG}") })
    }

    fn update<'a>(&'a self, _db: &'a Db, _id: i64, _form: &'a FormData) -> UpdateResult<'a> {
        Box::pin(async { unreachable!("{PANIC_MSG}") })
    }

    fn delete<'a>(
        &'a self,
        _db: &'a Db,
        _id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { unreachable!("{PANIC_MSG}") })
    }

    fn object_label<'a>(
        &'a self,
        _db: &'a Db,
        _id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'a>> {
        Box::pin(async { unreachable!("{PANIC_MSG}") })
    }
}

#[cfg(test)]
struct FailingOps;

#[cfg(test)]
impl AdminOps for FailingOps {
    fn list<'a>(
        &'a self,
        _db: &'a Db,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ListRow>>> + Send + 'a>> {
        Box::pin(async { Err(crate::error::Error::Internal("simulated db failure".into())) })
    }

    fn find_row<'a>(
        &'a self,
        _db: &'a Db,
        _id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<EditRow>>> + Send + 'a>> {
        Box::pin(async { unreachable!("FailingOps only exercises list()") })
    }

    fn create<'a>(&'a self, _db: &'a Db, _form: &'a FormData) -> CreateResult<'a> {
        Box::pin(async { unreachable!("FailingOps only exercises list()") })
    }

    fn update<'a>(&'a self, _db: &'a Db, _id: i64, _form: &'a FormData) -> UpdateResult<'a> {
        Box::pin(async { unreachable!("FailingOps only exercises list()") })
    }

    fn delete<'a>(
        &'a self,
        _db: &'a Db,
        _id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { unreachable!("FailingOps only exercises list()") })
    }

    fn object_label<'a>(
        &'a self,
        _db: &'a Db,
        _id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'a>> {
        Box::pin(async { unreachable!("FailingOps only exercises list()") })
    }
}
