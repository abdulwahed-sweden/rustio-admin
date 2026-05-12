# RustIO — Public API enumeration

_This document is descriptive, not normative. Annotation does not itself guarantee SemVer stability before 1.0._

Generated from `// public:` annotations across the workspace.

**Total items: 355**

---

## crate `rustio_admin`

### `rustio_admin`

- `pub mod admin;`
- `pub mod auth;`
- `pub mod background;`
- `pub mod email;`
- `pub mod error;`
- `pub mod http;`
- `pub mod middleware;`
- `pub mod migrations;`
- `pub mod orm;`
- `pub mod router;`
- `pub mod server;`
- `pub mod templates;`
- `pub use crate::admin::`
- `pub use crate::auth::`
- `pub use crate::error::`
- `pub use crate::http::`
- `pub use crate::orm::`
- `pub use crate::router::`
- `pub use crate::server::Server;`
- `pub use rustio_admin_macros::RustioAdmin;`

### `rustio_admin::admin`

- `pub mod audit;`
- `pub mod filters;`
- `pub mod modeladmin;`
- `pub mod redact;`
- `pub mod relations;`
- `pub use audit::`
- `pub use filters::`
- `pub use modeladmin::`
- `pub use redact::`
- `pub use relations::`
- `pub use routes::register_admin_routes;`
- `pub use types::`

### `rustio_admin::admin::audit`

- `pub async fn ensure_table(db: &Db) -> Result<()>`
- `pub enum ActionType`
- `pub fn as_str(self) -> &'static str`
- `pub fn parse(s: &str) -> Option<Self>`
- `pub fn label(self) -> &'static str`
- `pub fn pill_class(self) -> &'static str`
- `pub struct AdminAction`
- `pub struct LogEntry<'a>`
- `pub fn new(user_id: i64, action_type: ActionType, model_name: &'a str, object_id: i64) -> Self`
- `pub fn with_actor(mut self, actor_user_id: i64) -> Self`
- `pub fn with_event(mut self, event: AuditEvent) -> Self`
- `pub async fn record(db: &Db, entry: LogEntry<'_>) -> Result<()>`
- `pub enum AuditEvent`
- `pub const fn as_str(self) -> &'static str`
- `pub async fn recent(`
- `pub async fn for_object(db: &Db, model_name: &str, object_id: i64) -> Result<Vec<AdminAction>>`

### `rustio_admin::admin::filters`

- `pub enum FieldRole`
- `pub fn is_sensitive(self) -> bool`
- `pub struct FieldUI`
- `pub enum FilterKind`
- `pub struct FilterDef`
- `pub fn classify_field(f: &AdminField) -> FieldRole`
- `pub fn field_ui_metadata(f: &AdminField) -> FieldUI`
- `pub fn field_ui_metadata_with_relation(f: &AdminField, relation_target: Option<&str>) -> FieldUI`
- `pub fn format_relation_cell(id: i64, target: Option<&str>) -> String`
- `pub fn infer_filters(fields: &[AdminField]) -> Vec<FilterDef>`
- `pub fn infer_filters_with_relations<F>(`
- `pub fn mask_pii(value: &str) -> String`

### `rustio_admin::admin::modeladmin`

- `pub struct Fieldset`
- `pub trait ModelAdmin: AdminModel`
- `pub struct BulkAction`
- `pub enum SortDir`
- `pub fn sql(self) -> &'static str`
- `pub fn parse_order_spec(spec: &str) -> (String, SortDir)`

### `rustio_admin::admin::redact`

- `pub const fn redact_password() -> &'static str`
- `pub fn redact_token(token: &str) -> String`
- `pub const fn redact_mfa_secret() -> &'static str`
- `pub const fn redact_backup_code() -> &'static str`

### `rustio_admin::admin::relations`

- `pub const RELATION_FILTER_DROPDOWN_CAP: usize = 500;`
- `pub struct ResolvedRelation`
- `pub struct InverseRelation`
- `pub enum RegistryError`
- `pub struct RelationRegistry`
- `pub fn empty() -> Self`
- `pub fn from_admin_entries(entries: &[AdminEntry]) -> Self`
- `pub fn belongs_to(&self, model: &str, field: &str) -> Option<&ResolvedRelation>`
- `pub fn belongs_to_of(&self, model: &str) -> &[ResolvedRelation]`
- `pub fn has_many(&self, model: &str) -> &[InverseRelation]`
- `pub fn is_empty(&self) -> bool`
- `pub fn validate(&self, entries: &[AdminEntry]) -> Vec<RegistryError>`
- `pub fn iter_belongs_to(&self) -> impl Iterator<Item = &ResolvedRelation>`

### `rustio_admin::admin::routes`

- `pub fn register_admin_routes(`

### `rustio_admin::admin::types`

- `pub struct UserProfileSection`
- `pub struct UserProfileRow`
- `pub enum FieldType`
- `pub fn widget(&self) -> &'static str`
- `pub fn nullable(&self) -> bool`
- `pub struct AdminField`
- `pub struct AdminRelation`
- `pub trait AdminModel: Send + Sync + 'static`
- `pub struct AdminEntry`
- `pub struct ListOpts`
- `pub struct ListPage`
- `pub struct ListRow`
- `pub struct CellLink`
- `pub struct EditRow`
- `pub struct SiteBranding`
- `pub struct AdminTheme`
- `pub fn new() -> Self`
- `pub fn accent(mut self, color: impl Into<String>) -> Self`
- `pub fn bg(mut self, color: impl Into<String>) -> Self`
- `pub fn surface(mut self, color: impl Into<String>) -> Self`
- `pub fn text(mut self, color: impl Into<String>) -> Self`
- `pub fn text_muted(mut self, color: impl Into<String>) -> Self`
- `pub fn border(mut self, color: impl Into<String>) -> Self`
- `pub struct Admin`
- `pub fn new() -> Self`
- `pub fn site_branding(mut self, branding: SiteBranding) -> Self`
- `pub fn branding(&self) -> &SiteBranding`
- `pub fn accent_color(mut self, color: impl Into<String>) -> Self`
- `pub fn theme(mut self, theme: AdminTheme) -> Self`
- `pub fn accent(&self) -> Option<&str>`
- `pub fn active_theme(&self) -> &AdminTheme`
- `pub fn mailer(mut self, mailer: SharedMailer) -> Self`
- `pub fn active_mailer(&self) -> &SharedMailer`
- `pub fn has_custom_mailer(&self) -> bool`
- `pub fn password_policy(mut self, policy: SharedPasswordPolicy) -> Self`
- `pub fn active_password_policy(&self) -> &SharedPasswordPolicy`
- `pub fn recovery_policy(mut self, policy: SharedRecoveryPolicy) -> Self`
- `pub fn active_recovery_policy(&self) -> &SharedRecoveryPolicy`
- `pub fn require_mfa(mut self, policy: MfaPolicy) -> Self`
- `pub fn active_mfa_policy(&self) -> MfaPolicy`
- `pub fn model<M>(mut self) -> Self`
- `pub fn entries(&self) -> &[AdminEntry]`
- `pub fn user_profile_extension<F, Fut>(mut self, ext: F) -> Self`
- `pub fn find(&self, admin_name: &str) -> Option<&AdminEntry>`
- `pub async fn seed_permissions(&self, db: &crate::orm::Db) -> crate::error::Result<()>`

### `rustio_admin::auth`

- `pub mod emergency;`
- `pub mod guards;`
- `pub use mfa::MfaPolicy;`
- `pub use permissions::`
- `pub use recovery::`
- `pub use role::`
- `pub use sessions::`
- `pub use users::would_orphan_developers;`
- `pub use users::`
- `pub async fn init_tables(db: &Db) -> Result<()>`

### `rustio_admin::auth::emergency`

- `pub enum ResetOutcome`
- `pub enum UnlockOutcome`
- `pub enum DisableMfaOutcome`
- `pub enum PromoteOutcome`
- `pub enum EmergencyAccessOutcome`
- `pub async fn reset_password(`
- `pub async fn unlock(db: &Db, target_user_id: i64) -> Result<UnlockOutcome>`
- `pub async fn disable_mfa(db: &Db, target_user_id: i64) -> Result<DisableMfaOutcome>`
- `pub async fn promote(db: &Db, target_user_id: i64, new_role: Role) -> Result<PromoteOutcome>`
- `pub async fn emergency_access(`
- `pub fn generate_temp_password(len: usize) -> String`
- `pub fn fresh_correlation_id() -> String`

### `rustio_admin::auth::guards`

- `pub fn enforce_self_demote_safe(`
- `pub fn enforce_cross_rank_safe(actor: &Identity, target_id: i64, target_role: Role) -> Result<()>`
- `pub fn enforce_role_ceiling(actor: &Identity, requested_role: Role) -> Result<()>`
- `pub async fn enforce_no_orphan_role(`

### `rustio_admin::auth::mfa`

- `pub enum MfaPolicy`

### `rustio_admin::auth::permissions`

- `pub struct Superuser;`
- `pub struct Permission`
- `pub enum PermissionError`
- `pub async fn init_permission_tables(db: &Db) -> Result<()>`
- `pub async fn permissions_for_user(db: &Db, user_id: i64) -> Result<Arc<HashSet<String>>>`
- `pub async fn check_permission(db: &Db, identity: &Identity, permission: &str) -> Result<bool>`
- `pub async fn grant_to_user(db: &Db, user_id: i64, permission: &str) -> Result<()>`
- `pub async fn grant_to_group(db: &Db, group_id: i64, permission: &str) -> Result<()>`
- `pub async fn create_group(db: &Db, name: &str, description: &str) -> Result<i64>`
- `pub async fn add_user_to_group(db: &Db, user_id: i64, group_id: i64) -> Result<()>`
- `pub async fn remove_user_from_group(db: &Db, user_id: i64, group_id: i64) -> Result<()>`
- `pub async fn register_model_permissions(db: &Db, app: &str, singular: &str) -> Result<()>`

### `rustio_admin::auth::recovery`

- `pub trait PasswordPolicy: Send + Sync`
- `pub type SharedPasswordPolicy = Arc<dyn PasswordPolicy>;`
- `pub enum PasswordPolicyError`
- `pub struct DefaultPasswordPolicy`
- `pub const fn new() -> Self`
- `pub const fn with_min_len(min_len: usize) -> Self`
- `pub struct LoginThrottle`
- `pub const DEFAULT: Self = Self`
- `pub trait RecoveryPolicy: Send + Sync`
- `pub type SharedRecoveryPolicy = Arc<dyn RecoveryPolicy>;`
- `pub struct DefaultRecoveryPolicy`
- `pub fn new() -> Self`
- `pub fn with_reset_token_ttl(mut self, ttl: ChronoDuration) -> Self`
- `pub fn with_request_rate_limit(mut self, capacity: u32, window: StdDuration) -> Self`
- `pub fn with_consume_rate_limit(mut self, capacity: u32, window: StdDuration) -> Self`
- `pub fn with_strict_mailer_required(mut self, required: bool) -> Self`

### `rustio_admin::auth::role`

- `pub enum Role`
- `pub const fn rank(self) -> u32`
- `pub fn includes(self, other: Role) -> bool`
- `pub fn as_str(self) -> &'static str`
- `pub fn label(self) -> &'static str`
- `pub fn parse(s: &str) -> Result<Self>`
- `pub fn can_access_panel(self) -> bool`
- `pub fn bypasses_group_checks(self) -> bool`
- `pub const fn protected_roles() -> &'static [Role]`

### `rustio_admin::auth::sessions`

- `pub const SESSION_COOKIE: &str = "rustio_session";`
- `pub enum SessionTrust`
- `pub const fn as_str(self) -> &'static str`
- `pub const fn rank(self) -> u8`
- `pub const fn satisfies(self, other: SessionTrust) -> bool`
- `pub fn parse(s: &str) -> Self`
- `pub enum SessionInvalidationReason`
- `pub const fn as_str(self) -> &'static str`
- `pub enum SessionTarget`
- `pub struct Session`
- `pub struct InvalidationOutcome`
- `pub async fn init_session_tables(db: &Db) -> Result<()>`
- `pub async fn create_session(db: &Db, user_id: i64) -> Result<String>`
- `pub async fn delete_session(db: &Db, token: &str) -> Result<()>`
- `pub async fn invalidate_sessions(`
- `pub async fn logout_session(db: &Db, token: &str) -> Result<()>`
- `pub async fn list_active_for_user(db: &Db, user_id: i64) -> Result<Vec<Session>>`
- `pub async fn current_session_id(db: &Db, token: &str) -> Result<Option<i64>>`
- `pub async fn identity_from_session(db: &Db, token: &str) -> Result<Option<Identity>>`
- `pub async fn purge_expired_sessions(db: &Db) -> Result<u64>`
- `pub fn session_token_from_cookie(cookie_header: &str) -> Option<String>`

### `rustio_admin::auth::users`

- `pub struct Identity`
- `pub fn is_admin(&self) -> bool`
- `pub fn can_access_admin(&self) -> bool`
- `pub struct StoredUser`
- `pub struct UserProfile`
- `pub async fn init_user_tables(db: &Db) -> Result<()>`
- `pub async fn migrate_user_schema(db: &Db) -> Result<()>`
- `pub fn hash_password(plain: &str) -> Result<String>`
- `pub fn verify_password(plain: &str, stored_hash: &str) -> bool`
- `pub async fn create_user(db: &Db, email: &str, password: &str, role: Role) -> Result<i64>`
- `pub async fn find_user_by_email(db: &Db, email: &str) -> Result<Option<StoredUser>>`
- `pub async fn load_user_profile(db: &Db, user_id: i64) -> Result<Option<UserProfile>>`
- `pub async fn set_password(db: &Db, user_id: i64, new_password: &str) -> Result<()>`
- `pub async fn update_user_role(db: &Db, user_id: i64, role: Role) -> Result<()>`
- `pub fn verdict_for_orphan_role(`
- `pub async fn would_orphan_role(`
- `pub async fn would_orphan_protected(`
- `pub async fn would_orphan_developers(`
- `pub async fn login(db: &Db, email: &str, password: &str) -> Result<String>`

### `rustio_admin::background`

- `pub fn spawn_housekeeping(db: Db)`
- `pub fn spawn_session_sweeper(db: Db)`

### `rustio_admin::email`

- `pub struct Mail`
- `pub fn framework_envelope(`
- `pub enum MailerError`
- `pub trait Mailer: Send + Sync`
- `pub struct LogMailer;`
- `pub fn new() -> Self`
- `pub type SharedMailer = Arc<dyn Mailer>;`

### `rustio_admin::error`

- `pub type Result<T> = std::result::Result<T, Error>;`
- `pub enum Error`
- `pub fn status(&self) -> u16`
- `pub fn client_message(&self) -> &str`

### `rustio_admin::http`

- `pub struct Context`
- `pub fn insert<T: Any + Send + Sync>(&mut self, value: T)`
- `pub fn get<T: Any + Send + Sync>(&self) -> Option<&T>`
- `pub fn get_mut<T: Any + Send + Sync>(&mut self) -> Option<&mut T>`
- `pub struct Request`
- `pub fn method(&self) -> &Method`
- `pub fn path(&self) -> &str`
- `pub fn query_string(&self) -> &str`
- `pub fn query(&self) -> FormData`
- `pub fn header(&self, name: &str) -> Option<&str>`
- `pub fn param(&self, name: &str) -> Option<&str>`
- `pub fn body(&self) -> &[u8]`
- `pub fn body_text(&self) -> Result<&str>`
- `pub fn form(&self) -> Result<FormData>`
- `pub fn ctx(&self) -> &Context`
- `pub fn ctx_mut(&mut self) -> &mut Context`
- `pub struct FormData`
- `pub fn from_urlencoded(input: &str) -> Self`
- `pub fn get(&self, key: &str) -> Option<&str>`
- `pub fn required(&self, key: &str) -> Result<&str>`
- `pub fn bool_flag(&self, key: &str) -> bool`
- `pub fn contains(&self, key: &str) -> bool`
- `pub fn as_map(&self) -> &HashMap<String, String>`
- `pub struct Response`
- `pub fn new(status: StatusCode, body: impl Into<Bytes>) -> Self`
- `pub fn ok(body: impl Into<Bytes>) -> Self`
- `pub fn html(body: impl Into<String>) -> Self`
- `pub fn json_raw(body: impl Into<String>) -> Self`
- `pub fn redirect(to: impl Into<String>) -> Self`
- `pub fn text(body: impl Into<String>) -> Self`
- `pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self`
- `pub fn with_status(mut self, status: StatusCode) -> Self`

### `rustio_admin::middleware`

- `pub use compression::gzip;`
- `pub use correlation_id::`
- `pub use csrf::`
- `pub use logger::logger;`
- `pub use rate_limit::`
- `pub use security_headers::security_headers;`

### `rustio_admin::middleware::compression`

- `pub async fn gzip(req: Request, next: Next) -> Result<Response>`

### `rustio_admin::middleware::correlation_id`

- `pub const CORRELATION_ID_HEADER: &str = "x-correlation-id";`
- `pub struct CorrelationId(pub String);`
- `pub fn as_str(&self) -> &str`
- `pub async fn correlation_id(mut req: Request, next: Next) -> Result<Response>`

### `rustio_admin::middleware::csrf`

- `pub const CSRF_COOKIE: &str = "rustio_csrf";`
- `pub const CSRF_HEADER: &str = "x-csrf-token";`
- `pub const CSRF_FIELD: &str = "_csrf";`
- `pub struct CsrfGuard`
- `pub async fn csrf_protect(mut req: Request, next: Next) -> Result<Response>`

### `rustio_admin::middleware::logger`

- `pub async fn logger(req: Request, next: Next) -> Result<Response>`

### `rustio_admin::middleware::rate_limit`

- `pub struct RateLimiter`
- `pub fn new(capacity: u32, window: Duration) -> Self`
- `pub fn default_limits() -> Self`
- `pub fn rate_limit(`

### `rustio_admin::middleware::security_headers`

- `pub async fn security_headers(req: Request, next: Next) -> Result<Response>`

### `rustio_admin::migrations`

- `pub struct MigrationFile`
- `pub struct ApplyOptions`
- `pub async fn apply(db: &Db, dir: impl AsRef<Path>) -> Result<Vec<String>>`
- `pub async fn apply_with(db: &Db, dir: impl AsRef<Path>, opts: ApplyOptions) -> Result<Vec<String>>`
- `pub async fn applied_versions(db: &Db) -> Result<Vec<i64>>`
- `pub async fn status(db: &Db, dir: impl AsRef<Path>) -> Result<Vec<(String, bool)>>`
- `pub fn generate(dir: impl AsRef<Path>, name: &str) -> Result<PathBuf>`

### `rustio_admin::orm`

- `pub struct Db`
- `pub async fn connect(url: &str) -> Result<Self>`
- `pub async fn connect_with(url: &str, opts: DbOptions) -> Result<Self>`
- `pub fn pool(&self) -> &PgPool`
- `pub async fn health_check(&self) -> Result<()>`
- `pub struct DbOptions`
- `pub enum Value`
- `pub struct Row<'a>`
- `pub fn from_pg(row: &'a PgRow) -> Self`
- `pub fn get_i32(&self, col: &str) -> Result<i32>`
- `pub fn get_i64(&self, col: &str) -> Result<i64>`
- `pub fn get_optional_i64(&self, col: &str) -> Result<Option<i64>>`
- `pub fn get_bool(&self, col: &str) -> Result<bool>`
- `pub fn get_string(&self, col: &str) -> Result<String>`
- `pub fn get_optional_string(&self, col: &str) -> Result<Option<String>>`
- `pub fn get_datetime(&self, col: &str) -> Result<DateTime<Utc>>`
- `pub fn get_optional_datetime(&self, col: &str) -> Result<Option<DateTime<Utc>>>`
- `pub fn get_uuid(&self, col: &str) -> Result<Uuid>`
- `pub fn get_json(&self, col: &str) -> Result<JsonValue>`
- `pub trait Model: Send + Sync + Sized + 'static`
- `pub async fn all<M: Model>(db: &Db) -> Result<Vec<M>>`
- `pub async fn page<M: Model>(db: &Db, limit: i64, offset: i64) -> Result<Vec<M>>`
- `pub async fn count<M: Model>(db: &Db) -> Result<i64>`
- `pub async fn find<M: Model>(db: &Db, id: i64) -> Result<Option<M>>`
- `pub async fn create<M: Model>(db: &Db, model: &M) -> Result<i64>`
- `pub async fn update<M: Model>(db: &Db, id: i64, model: &M) -> Result<()>`
- `pub async fn delete<M: Model>(db: &Db, id: i64) -> Result<()>`

### `rustio_admin::router`

- `pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;`
- `pub type HandlerFn =`
- `pub type MiddlewareFn =`
- `pub struct Next`
- `pub fn run(mut self, req: Request) -> BoxFuture<'static, Result<Response>>`
- `pub struct Router`
- `pub fn new() -> Self`
- `pub fn middleware<F, Fut>(mut self, mw: F) -> Self`
- `pub fn get<F, Fut>(self, path: &str, handler: F) -> Self`
- `pub fn post<F, Fut>(self, path: &str, handler: F) -> Self`
- `pub fn route<F, Fut>(mut self, method: Method, path: &str, handler: F) -> Self`
- `pub async fn dispatch(&self, mut req: Request) -> Response`

### `rustio_admin::server`

- `pub struct Server`
- `pub fn new(router: Router, addr: SocketAddr) -> Self`
- `pub async fn run(self) -> Result<()>`
- `pub async fn serve_static(root: std::path::PathBuf, name: &str) -> Result<Response>`

### `rustio_admin::templates`

- `pub struct Templates`
- `pub fn new(project_templates_dir: Option<PathBuf>) -> Result<Arc<Self>>`
- `pub fn render<S: Serialize>(&self, name: &str, ctx: &S) -> Result<String>`
- `pub fn render_for_model<S: Serialize>(`

## crate `rustio_admin_macros`

### `rustio_admin_macros`

- `pub fn derive_rustio_admin(input: TokenStream) -> TokenStream`

