//! Clinic — HTTP server over the `clinic_prod` data layer.
//!
//! Layout: slate-900 topbar + 264px slate-900 sidebar + light content
//! canvas. Inline SVG icons throughout. Multi-section dashboard
//! (Today / Upcoming / Worklist / Recent activity). Standalone — no
//! path-dep on rustio-admin's assets so the crate stays
//! self-contained.
//!
//! Routes:
//!
//!   GET /                  → HTML dashboard (4-section grid + KPIs)
//!   GET /appointments      → HTML table of recent appointments
//!   GET /patients          → HTML list of patients
//!   GET /worklist          → HTML view of v_open_action_items
//!   GET /api/health        → JSON { ok: bool, server: string }
//!   GET /api/dashboard     → JSON of dashboard data
//!   GET /api/appointments  → JSON list of appointments
//!   GET /api/patients      → JSON list of patients
//!   GET /api/worklist      → JSON of v_open_action_items
//!
//! Run:
//!
//!   cargo run -p clinic                  # debug
//!   cargo run --release -p clinic        # release
//!   ./target/release/clinic              # direct binary
//!
//! Then open http://127.0.0.1:8000

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

const BIND: &str = "127.0.0.1:8000";

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::from_path("examples/.env");
    let url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL not set (check examples/.env or shell env)")?;
    println!("clinic — opening {}", redact_password(&url));

    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await?;
    let state = Arc::new(AppState { pool });

    let app = Router::new()
        .route("/", get(page_dashboard))
        .route("/appointments", get(page_appointments))
        .route("/patients", get(page_patients))
        .route("/worklist", get(page_worklist))
        .route("/api/health", get(api_health))
        .route("/api/dashboard", get(api_dashboard))
        .route("/api/appointments", get(api_appointments))
        .route("/api/patients", get(api_patients))
        .route("/api/worklist", get(api_worklist))
        .route("/style.css", get(serve_css))
        .with_state(state);

    let addr: SocketAddr = BIND.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("clinic — listening on http://{addr}");
    println!("clinic — press Ctrl-C to stop");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    println!("\nclinic — shutting down");
}

// ============================================================
// Inline CSS — slate-900 chrome (topbar + sidebar), white-card
// content. Token values match rustio-admin v0.16 design tokens
// but defined here so the crate stays independent.
// ============================================================

const CSS: &str = r#"
:root {
  /* Surface scale */
  --bg: #EEF1F5;
  --surface: #FFFFFF;
  --surface-2: #F6F8FB;
  --surface-3: #EFF3F8;

  /* Chrome (topbar + sidebar) */
  --chrome: #0B1220;
  --chrome-2: #161F31;
  --chrome-3: #243044;
  --chrome-border: #1C2740;
  --chrome-text-strong: #F8FAFC;
  --chrome-text: #CBD5E1;
  --chrome-text-muted: #8794A8;
  --chrome-text-subtle: #5A6781;

  /* Content text */
  --text: #2A3344;
  --text-strong: #0B1220;
  --text-muted: #5A6478;
  --text-subtle: #8590A3;

  /* Lines */
  --border: #E4E9F0;
  --border-strong: #CED6E1;

  /* Accent — teal */
  --accent: #0D9488;
  --accent-hover: #0B7C72;
  --accent-soft: #F0FDFA;
  --accent-border: #99F6E4;
  --chrome-accent: #2DD4BF;

  /* Semantic */
  --success: #059669;
  --warning: #D97706;
  --danger: #DC2626;
  --info: #2563EB;

  /* Shape */
  --radius: 14px;
  --radius-sm: 8px;
  --radius-pill: 999px;

  /* Elevation */
  --shadow-sm: 0 1px 2px rgba(11, 18, 32, .05);
  --shadow: 0 1px 3px rgba(11, 18, 32, .06), 0 8px 24px -12px rgba(11, 18, 32, .12);
  --shadow-lg: 0 2px 6px rgba(11, 18, 32, .07), 0 18px 40px -16px rgba(11, 18, 32, .18);

  /* Motion */
  --ease: cubic-bezier(.16, .84, .44, 1);
}

* { box-sizing: border-box; }

html { -webkit-text-size-adjust: 100%; }

body {
  margin: 0;
  font-family: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI",
    Roboto, "Helvetica Neue", Arial, sans-serif;
  font-size: 16px;
  line-height: 1.6;
  color: var(--text);
  background: var(--bg);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  font-feature-settings: "cv05", "ss01";
}

a {
  color: var(--accent);
  text-decoration: none;
  transition: color .12s var(--ease);
}
a:hover { color: var(--accent-hover); }

svg.icon { vertical-align: middle; flex-shrink: 0; }

::selection { background: var(--accent-border); color: var(--text-strong); }

:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
  border-radius: 4px;
}

/* === Layout shell ============================================ */
.shell { min-height: 100vh; display: flex; flex-direction: column; }
.body { display: flex; flex: 1; align-items: stretch; min-height: 0; }

/* Topbar */
.topbar {
  background: linear-gradient(180deg, #0E1626 0%, var(--chrome) 100%);
  color: var(--chrome-text-strong);
  padding: 0 32px;
  height: 72px;
  display: flex;
  align-items: center;
  gap: 24px;
  position: sticky;
  top: 0;
  z-index: 20;
  border-bottom: 1px solid var(--chrome-border);
}
.topbar .brand {
  display: inline-flex;
  align-items: center;
  gap: 12px;
  font-weight: 700;
  font-size: 18px;
  color: var(--chrome-text-strong);
  letter-spacing: -0.018em;
}
.topbar .brand .brand-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 9px;
  background: linear-gradient(150deg, rgba(45,212,191,.22), rgba(45,212,191,.06));
  border: 1px solid rgba(45,212,191,.28);
  color: var(--chrome-accent);
}
.topbar .topnav { display: flex; gap: 6px; margin-left: auto; }
.topbar .topnav a {
  color: var(--chrome-text-muted);
  font-weight: 500;
  font-size: 14px;
  padding: 7px 12px;
  border-radius: 7px;
  transition: background-color .12s var(--ease), color .12s var(--ease);
}
.topbar .topnav a:hover {
  color: var(--chrome-text-strong);
  background: var(--chrome-2);
}

/* Sidebar — 264px slate rail */
.sidebar {
  flex: 0 0 264px;
  background: var(--chrome);
  color: var(--chrome-text);
  padding: 22px 14px;
  border-right: 1px solid var(--chrome-border);
}
.sidebar-section {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--chrome-text-subtle);
  margin: 24px 14px 8px;
}
.sidebar-section:first-child { margin-top: 0; }
.sidebar-link {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-radius: var(--radius-sm);
  color: var(--chrome-text);
  font-size: 16px;
  font-weight: 500;
  line-height: 1.6;
  margin-bottom: 4px;
  position: relative;
  transition: background-color .14s var(--ease), color .14s var(--ease);
}
.sidebar-link:hover {
  background: var(--chrome-2);
  color: var(--chrome-text-strong);
}
.sidebar-link.active {
  background: linear-gradient(90deg, rgba(45,212,191,.16), rgba(45,212,191,.04));
  color: var(--chrome-accent);
}
.sidebar-link.active::before {
  content: "";
  position: absolute;
  left: 0;
  top: 7px;
  bottom: 7px;
  width: 3px;
  border-radius: 0 3px 3px 0;
  background: var(--chrome-accent);
}
.sidebar-link .icon { color: var(--chrome-text-muted); transition: color .14s var(--ease); }
.sidebar-link:hover .icon { color: var(--chrome-text-strong); }
.sidebar-link.active .icon { color: var(--chrome-accent); }

/* Main content */
.main {
  flex: 1;
  min-width: 0;
  padding: 32px 36px 56px;
}
.main > * { max-width: 1440px; margin-inline: auto; }

/* Headings */
h1 {
  font-size: 32px;
  font-weight: 700;
  color: var(--text-strong);
  margin: 0 0 8px;
  letter-spacing: -0.018em;
  line-height: 1.2;
  display: flex;
  align-items: center;
  gap: 12px;
}
h1 .icon { color: var(--accent); }
h2 {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-strong);
  margin: 0 0 16px;
  letter-spacing: -0.018em;
  line-height: 1.3;
  display: flex;
  align-items: center;
  gap: 10px;
}
h2 .icon { color: var(--accent); }
h3 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-strong);
  margin: 24px 0 12px;
}
.lead {
  color: var(--text-muted);
  font-size: 18px;
  margin: 0 0 32px;
  max-width: 70ch;
}

/* === KPI grid ================================================ */
.kpis {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 16px;
  margin-bottom: 28px;
}
.kpi {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 20px 22px;
  box-shadow: var(--shadow-sm);
  display: flex;
  flex-direction: column;
  gap: 10px;
  position: relative;
  overflow: hidden;
  transition: box-shadow .18s var(--ease), transform .18s var(--ease);
}
.kpi::after {
  content: "";
  position: absolute;
  inset: 0 0 auto 0;
  height: 3px;
  background: var(--border-strong);
  opacity: .55;
}
.kpi:hover {
  box-shadow: var(--shadow);
  transform: translateY(-2px);
}
.kpi-head {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-subtle);
  font-size: 13px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  font-weight: 600;
}
.kpi-head .icon { color: var(--text-subtle); }
.kpi-num {
  font-size: 36px;
  font-weight: 700;
  color: var(--text-strong);
  letter-spacing: -0.02em;
  line-height: 1;
  font-variant-numeric: tabular-nums;
}
.kpi-foot { color: var(--text-subtle); font-size: 14px; }
.kpi.accent::after { background: var(--accent); opacity: 1; }
.kpi.accent .kpi-head .icon,
.kpi.accent .kpi-num { color: var(--accent); }
.kpi.danger::after { background: var(--danger); opacity: 1; }
.kpi.danger .kpi-head .icon,
.kpi.danger .kpi-num { color: var(--danger); }

/* === Section grid (multi-card dashboard) ===================== */
.panels {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 20px;
}
.panel {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 22px 24px;
  box-shadow: var(--shadow-sm);
  transition: box-shadow .18s var(--ease);
}
.panel:hover { box-shadow: var(--shadow); }
.panel.wide { grid-column: span 2; }
.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 14px;
  padding-bottom: 14px;
  border-bottom: 1px solid var(--border);
}
.panel-head h2 { margin: 0; }
.panel-head .panel-more {
  font-size: 14px;
  color: var(--text-subtle);
  font-weight: 500;
  white-space: nowrap;
}
.panel-head .panel-more a { color: var(--accent); }
@media (max-width: 960px) {
  .panels { grid-template-columns: 1fr; }
  .panel.wide { grid-column: auto; }
}

/* === Single content card (non-grid pages) ==================== */
.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 8px 8px;
  box-shadow: var(--shadow-sm);
  margin-bottom: 28px;
  overflow: hidden;
}
.card h2:first-child, .card h3:first-child { margin-top: 0; }

/* === Tables ================================================== */
table { width: 100%; border-collapse: collapse; }
th, td {
  padding: 14px 20px;
  text-align: left;
  vertical-align: middle;
}
td { border-bottom: 1px solid var(--border); }
.panel table th, .panel table td { padding: 12px 16px; }
.panel table th:first-child, .panel table td:first-child { padding-left: 4px; }
.panel table th:last-child, .panel table td:last-child { padding-right: 4px; }
thead th {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  background: var(--surface-2);
  border-bottom: 1px solid var(--border);
  padding-block: 12px;
  white-space: nowrap;
}
.panel table thead th { background: transparent; }
tbody tr { transition: background-color .1s var(--ease); }
tbody tr:last-child td { border-bottom: none; }
tbody tr:hover { background: var(--surface-2); }
.panel tbody tr:hover { background: var(--accent-soft); }
td.primary { font-weight: 600; color: var(--text-strong); }
td.num { text-align: right; font-variant-numeric: tabular-nums; }
th.num { text-align: right; }

/* === Status & severity pills ================================= */
.pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px 4px 9px;
  border-radius: var(--radius-pill);
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  white-space: nowrap;
  border: 1px solid transparent;
}
.pill::before {
  content: "";
  width: 6px;
  height: 6px;
  border-radius: var(--radius-pill);
  background: currentColor;
  display: inline-block;
}
.pill-completed   { background: #ECFDF5; color: var(--success); border-color: #A7F3D0; }
.pill-confirmed   { background: var(--accent-soft); color: var(--accent-hover); border-color: var(--accent-border); }
.pill-booked      { background: #EFF6FF; color: var(--info); border-color: #BFDBFE; }
.pill-checked_in  { background: #FFFBEB; color: var(--warning); border-color: #FDE68A; }
.pill-in_progress { background: #FFFBEB; color: var(--warning); border-color: #FDE68A; }
.pill-cancelled   { background: #FEF2F2; color: var(--danger); border-color: #FECACA; }
.pill-no_show     { background: #FEF2F2; color: var(--danger); border-color: #FECACA; }
.sev-overdue  { background: #FEF2F2; color: var(--danger); border-color: #FECACA; }
.sev-due_soon { background: #FFFBEB; color: var(--warning); border-color: #FDE68A; }
.sev-normal   { background: var(--accent-soft); color: var(--accent-hover); border-color: var(--accent-border); }

.muted { color: var(--text-muted); font-size: 14px; }
.tabular { font-variant-numeric: tabular-nums; }
.empty {
  padding: 36px 0;
  color: var(--text-subtle);
  text-align: center;
  font-size: 15px;
}

/* === Footer ================================================== */
.footer {
  padding: 16px 36px;
  color: var(--text-subtle);
  font-size: 13px;
  border-top: 1px solid var(--border);
  background: var(--surface);
  text-align: center;
}
.footer a {
  color: var(--text-muted);
  font-weight: 500;
}
.footer a:hover { color: var(--accent); }

@media (max-width: 768px) {
  .sidebar { display: none; }
  .main { padding: 20px 16px 40px; }
  .topbar { padding: 0 16px; }
}
"#;

async fn serve_css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], CSS)
}

// ============================================================
// Icons — inline SVG, single source of truth, all the same
// 16×16 viewBox + 2px stroke for a uniform line weight.
// ============================================================

fn icon(name: &str) -> &'static str {
    match name {
        // Tabler-style hairlines. `currentColor` so the link / KPI
        // tinting cascades into the stroke automatically.
        "dashboard" => {
            r#"<svg class="icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="9"/><rect x="14" y="3" width="7" height="5"/><rect x="14" y="12" width="7" height="9"/><rect x="3" y="16" width="7" height="5"/></svg>"#
        }
        "calendar" => {
            r#"<svg class="icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="18" rx="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>"#
        }
        "users" => {
            r#"<svg class="icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>"#
        }
        "list-check" => {
            r#"<svg class="icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 17 5 19 9 15"/><polyline points="3 7 5 9 9 5"/><line x1="13" y1="6" x2="21" y2="6"/><line x1="13" y1="12" x2="21" y2="12"/><line x1="13" y1="18" x2="21" y2="18"/></svg>"#
        }
        "clock" => {
            r#"<svg class="icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>"#
        }
        "activity" => {
            r#"<svg class="icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/></svg>"#
        }
        "alert" => {
            r#"<svg class="icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>"#
        }
        "dollar" => {
            r#"<svg class="icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>"#
        }
        "plus-circle" => {
            r#"<svg class="icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="16"/><line x1="8" y1="12" x2="16" y2="12"/></svg>"#
        }
        "stethoscope" => {
            r#"<svg class="icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 2v2"/><path d="M5 2v2"/><path d="M5 3H4a2 2 0 0 0-2 2v4a6 6 0 0 0 12 0V5a2 2 0 0 0-2-2h-1"/><path d="M8 15a6 6 0 0 0 12 0v-3"/><circle cx="20" cy="10" r="2"/></svg>"#
        }
        _ => "",
    }
}

// ============================================================
// Row structs (sqlx FromRow)
// ============================================================

#[derive(FromRow, Serialize, Clone)]
struct DashboardItem {
    category: String,
    severity: String,
    subject_name: String,
    description: String,
    amount_sek: Option<Decimal>,
    days_outstanding: Option<i32>,
}

#[derive(FromRow, Serialize, Clone)]
struct AppointmentRow {
    appointment_number: String,
    scheduled_start: DateTime<Utc>,
    scheduled_end: DateTime<Utc>,
    patient_name: String,
    doctor_name: String,
    specialty_code: Option<String>,
    status: String,
    chief_complaint: Option<String>,
}

#[derive(FromRow, Serialize)]
struct PatientRow {
    id: Uuid,
    medical_record_no: String,
    full_name: String,
    date_of_birth: Option<NaiveDate>,
    gender: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    allergies: Option<String>,
    chronic_conditions: Option<String>,
}

// ============================================================
// HTML helpers
// ============================================================

fn h(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn pill(class_suffix: &str, label: &str) -> String {
    format!(
        "<span class=\"pill pill-{cls}\">{lbl}</span>",
        cls = h(class_suffix),
        lbl = h(label)
    )
}
fn sev_pill(severity: &str) -> String {
    format!(
        "<span class=\"pill sev-{cls}\">{lbl}</span>",
        cls = h(severity),
        lbl = h(severity)
    )
}
fn money(d: Option<Decimal>) -> String {
    match d {
        Some(v) => format!("{v} SEK"),
        None => "—".into(),
    }
}
fn fmt_local(ts: DateTime<Utc>) -> String {
    let stockholm = chrono::FixedOffset::east_opt(2 * 3600).unwrap();
    ts.with_timezone(&stockholm)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}
fn fmt_local_short(ts: DateTime<Utc>) -> String {
    let stockholm = chrono::FixedOffset::east_opt(2 * 3600).unwrap();
    ts.with_timezone(&stockholm).format("%a %H:%M").to_string()
}

fn sidebar(active: &str) -> String {
    let item = |path: &str, ico: &str, label: &str| -> String {
        let cls = if active == path { " active" } else { "" };
        format!(
            r#"<a href="{path}" class="sidebar-link{cls}">{icon} {label}</a>"#,
            icon = icon(ico),
            label = h(label)
        )
    };
    format!(
        r#"<aside class="sidebar">
  <div class="sidebar-section">Overview</div>
  {dashboard}
  {worklist}
  <div class="sidebar-section">Scheduling</div>
  {appointments}
  <div class="sidebar-section">Records</div>
  {patients}
</aside>"#,
        dashboard = item("/", "dashboard", "Dashboard"),
        worklist = item("/worklist", "list-check", "Worklist"),
        appointments = item("/appointments", "calendar", "Appointments"),
        patients = item("/patients", "users", "Patients"),
    )
}

fn page(active: &str, title: &str, body: &str) -> Html<String> {
    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title_e} — Clinic Management</title>
<link rel="stylesheet" href="/style.css">
</head>
<body>
<div class="shell">
  <header class="topbar">
    <span class="brand"><span class="brand-mark">{steth_icon}</span> Clinic Management</span>
    <nav class="topnav">
      <a href="/api/health">API · health</a>
      <a href="/api/dashboard">API · dashboard</a>
    </nav>
  </header>
  <div class="body">
    {sidebar}
    <main class="main">
      {body}
    </main>
  </div>
  <footer class="footer">
    JSON endpoints: <a href="/api/dashboard">/api/dashboard</a> ·
    <a href="/api/appointments">/api/appointments</a> ·
    <a href="/api/patients">/api/patients</a> ·
    <a href="/api/worklist">/api/worklist</a> ·
    <a href="/api/health">/api/health</a>
  </footer>
</div>
</body>
</html>
"#,
        title_e = h(title),
        steth_icon = icon("stethoscope"),
        sidebar = sidebar(active),
        body = body,
    ))
}

// ============================================================
// SQL constants
// ============================================================

const WORKLIST_SQL: &str = r#"
SELECT category, severity, subject_name, description,
       amount_sek, days_outstanding
FROM v_open_action_items
ORDER BY
  CASE severity WHEN 'overdue' THEN 1 WHEN 'due_soon' THEN 2 ELSE 3 END,
  days_outstanding DESC NULLS LAST
"#;

const APPOINTMENTS_SQL: &str = r#"
SELECT
  a.appointment_number,
  a.scheduled_start,
  a.scheduled_end,
  p.full_name              AS patient_name,
  d.full_name              AS doctor_name,
  sp.code                  AS specialty_code,
  a.status::text           AS status,
  a.chief_complaint
FROM appointments a
JOIN patients p           ON p.id = a.patient_id
JOIN doctors d            ON d.id = a.doctor_id
LEFT JOIN specialties sp  ON sp.id = d.specialty_id
ORDER BY a.scheduled_start DESC
"#;

const TODAY_SQL: &str = r#"
SELECT
  a.appointment_number, a.scheduled_start, a.scheduled_end,
  p.full_name AS patient_name, d.full_name AS doctor_name,
  sp.code AS specialty_code, a.status::text AS status, a.chief_complaint
FROM appointments a
JOIN patients p           ON p.id = a.patient_id
JOIN doctors d            ON d.id = a.doctor_id
LEFT JOIN specialties sp  ON sp.id = d.specialty_id
WHERE (a.scheduled_start AT TIME ZONE 'Europe/Stockholm')::date
    = (now() AT TIME ZONE 'Europe/Stockholm')::date
ORDER BY a.scheduled_start
"#;

const UPCOMING_SQL: &str = r#"
SELECT
  a.appointment_number, a.scheduled_start, a.scheduled_end,
  p.full_name AS patient_name, d.full_name AS doctor_name,
  sp.code AS specialty_code, a.status::text AS status, a.chief_complaint
FROM appointments a
JOIN patients p           ON p.id = a.patient_id
JOIN doctors d            ON d.id = a.doctor_id
LEFT JOIN specialties sp  ON sp.id = d.specialty_id
WHERE a.scheduled_start > now()
  AND a.scheduled_start <= now() + interval '7 days'
  AND a.status NOT IN ('cancelled', 'no_show')
ORDER BY a.scheduled_start
LIMIT 8
"#;

const RECENT_SQL: &str = r#"
SELECT
  a.appointment_number, a.scheduled_start, a.scheduled_end,
  p.full_name AS patient_name, d.full_name AS doctor_name,
  sp.code AS specialty_code, a.status::text AS status, a.chief_complaint
FROM appointments a
JOIN patients p           ON p.id = a.patient_id
JOIN doctors d            ON d.id = a.doctor_id
LEFT JOIN specialties sp  ON sp.id = d.specialty_id
WHERE a.status = 'completed'
ORDER BY a.completed_at DESC NULLS LAST
LIMIT 5
"#;

const PATIENTS_SQL: &str = r#"
SELECT id, medical_record_no, full_name, date_of_birth,
       gender::text AS gender, phone, email, allergies, chronic_conditions
FROM patients
ORDER BY full_name
"#;

// ============================================================
// Render fragments
// ============================================================

fn appt_table(rows: &[AppointmentRow], empty_msg: &str, short_time: bool) -> String {
    if rows.is_empty() {
        return format!(r#"<p class="empty">{}</p>"#, h(empty_msg));
    }
    let mut s = String::from(
        "<table><thead><tr><th>When</th><th>Patient</th><th>Doctor</th><th>Spec</th><th>Status</th></tr></thead><tbody>",
    );
    for r in rows {
        let when = if short_time {
            fmt_local_short(r.scheduled_start)
        } else {
            fmt_local(r.scheduled_start)
        };
        s.push_str(&format!(
            "<tr><td class=\"tabular\">{when}</td><td class=\"primary\">{pat}</td><td>{doc}</td><td>{spec}</td><td>{status}</td></tr>",
            when = h(&when),
            pat = h(&r.patient_name),
            doc = h(&r.doctor_name),
            spec = h(r.specialty_code.as_deref().unwrap_or("—")),
            status = pill(&r.status, &r.status.replace('_', " "))
        ));
    }
    s.push_str("</tbody></table>");
    s
}

fn worklist_table(items: &[DashboardItem], compact: bool) -> String {
    if items.is_empty() {
        return r#"<p class="empty">Nothing pending. Good clinic.</p>"#.to_string();
    }
    let mut s = if compact {
        String::from("<table><thead><tr><th>Severity</th><th>Subject</th><th>Description</th><th class=\"num\">Amount</th></tr></thead><tbody>")
    } else {
        String::from("<table><thead><tr><th>Category</th><th>Severity</th><th>Subject</th><th>Description</th><th class=\"num\">Amount</th><th class=\"num\">Days</th></tr></thead><tbody>")
    };
    for it in items {
        if compact {
            s.push_str(&format!(
                "<tr><td>{sev}</td><td class=\"primary\">{subj}</td><td>{desc}</td><td class=\"num tabular\">{amt}</td></tr>",
                sev = sev_pill(&it.severity),
                subj = h(&it.subject_name),
                desc = h(&it.description),
                amt = money(it.amount_sek)
            ));
        } else {
            s.push_str(&format!(
                "<tr><td>{cat}</td><td>{sev}</td><td class=\"primary\">{subj}</td><td>{desc}</td><td class=\"num tabular\">{amt}</td><td class=\"num tabular\">{days}</td></tr>",
                cat = h(&it.category.replace('_', " ")),
                sev = sev_pill(&it.severity),
                subj = h(&it.subject_name),
                desc = h(&it.description),
                amt = money(it.amount_sek),
                days = it
                    .days_outstanding
                    .map(|d| d.to_string())
                    .unwrap_or("—".into())
            ));
        }
    }
    s.push_str("</tbody></table>");
    s
}

fn kpi(ico: &str, label: &str, num: &str, foot: &str, accent: &str) -> String {
    format!(
        r#"<div class="kpi {accent}"><div class="kpi-head">{icon} {label}</div><div class="kpi-num">{num}</div><div class="kpi-foot">{foot}</div></div>"#,
        icon = icon(ico),
        label = h(label),
        num = h(num),
        foot = h(foot),
        accent = h(accent),
    )
}

// ============================================================
// Page handlers (HTML)
// ============================================================

async fn page_dashboard(State(s): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let items: Vec<DashboardItem> = sqlx::query_as(WORKLIST_SQL).fetch_all(&s.pool).await?;
    let today_rows: Vec<AppointmentRow> = sqlx::query_as(TODAY_SQL).fetch_all(&s.pool).await?;
    let upcoming_rows: Vec<AppointmentRow> =
        sqlx::query_as(UPCOMING_SQL).fetch_all(&s.pool).await?;
    let recent_rows: Vec<AppointmentRow> = sqlx::query_as(RECENT_SQL).fetch_all(&s.pool).await?;
    let appt_count: i64 = sqlx::query_scalar("SELECT count(*) FROM appointments")
        .fetch_one(&s.pool)
        .await?;
    let patient_count: i64 = sqlx::query_scalar("SELECT count(*) FROM patients")
        .fetch_one(&s.pool)
        .await?;
    let total_pending: Decimal = items.iter().filter_map(|i| i.amount_sek).sum();
    let overdue_count = items.iter().filter(|i| i.severity == "overdue").count();

    let mut body = String::new();
    body.push_str(&format!(r#"<h1>{} Dashboard</h1>"#, icon("dashboard")));
    body.push_str(r#"<p class="lead">Today's clinic state at a glance — open work, who's coming in, and what just shipped.</p>"#);

    // KPI row — 4 cards
    body.push_str(r#"<div class="kpis">"#);
    body.push_str(&kpi(
        "list-check",
        "Open items",
        &items.len().to_string(),
        &format!("{overdue_count} overdue"),
        if overdue_count > 0 { "danger" } else { "" },
    ));
    body.push_str(&kpi(
        "dollar",
        "Pending money",
        &format!("{total_pending} SEK"),
        "across all invoices",
        "accent",
    ));
    body.push_str(&kpi(
        "calendar",
        "Appointments",
        &appt_count.to_string(),
        "total in system",
        "",
    ));
    body.push_str(&kpi(
        "users",
        "Patients",
        &patient_count.to_string(),
        "registered",
        "",
    ));
    body.push_str(r#"</div>"#);

    // 4-panel grid
    body.push_str(r#"<div class="panels">"#);

    // Panel 1: Today
    body.push_str(&format!(
        r#"<div class="panel"><div class="panel-head"><h2>{} Today</h2><span class="panel-more">{} appts</span></div>{}</div>"#,
        icon("clock"),
        today_rows.len(),
        appt_table(&today_rows, "Nothing on today's schedule.", true)
    ));

    // Panel 2: Upcoming this week
    body.push_str(&format!(
        r#"<div class="panel"><div class="panel-head"><h2>{} Upcoming this week</h2><span class="panel-more">next 7 days</span></div>{}</div>"#,
        icon("calendar"),
        appt_table(
            &upcoming_rows,
            "No appointments in the next 7 days.",
            true
        )
    ));

    // Panel 3: Worklist (wide)
    body.push_str(&format!(
        r#"<div class="panel wide"><div class="panel-head"><h2>{} Open worklist</h2><span class="panel-more"><a href="/worklist">view all →</a></span></div>{}</div>"#,
        icon("alert"),
        worklist_table(&items, true)
    ));

    // Panel 4: Recent activity (wide)
    body.push_str(&format!(
        r#"<div class="panel wide"><div class="panel-head"><h2>{} Recent activity</h2><span class="panel-more"><a href="/appointments">view all →</a></span></div>{}</div>"#,
        icon("activity"),
        appt_table(&recent_rows, "No completed visits recorded.", false)
    ));

    body.push_str(r#"</div>"#); // .panels

    Ok(page("/", "Dashboard", &body))
}

async fn page_appointments(State(s): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let rows: Vec<AppointmentRow> = sqlx::query_as(APPOINTMENTS_SQL).fetch_all(&s.pool).await?;
    let mut body = String::new();
    body.push_str(&format!(r#"<h1>{} Appointments</h1>"#, icon("calendar")));
    body.push_str(&format!(
        r#"<p class="lead">{} total, newest first.</p>"#,
        rows.len()
    ));
    body.push_str(r#"<div class="card">"#);
    body.push_str("<table><thead><tr><th>When (Stockholm)</th><th>Apt #</th><th>Patient</th><th>Doctor</th><th>Spec</th><th>Status</th><th>Chief complaint</th></tr></thead><tbody>");
    for r in &rows {
        body.push_str(&format!(
            "<tr><td class=\"tabular\">{when}</td><td class=\"tabular\">{num}</td><td class=\"primary\">{pat}</td><td>{doc}</td><td>{spec}</td><td>{status}</td><td>{cc}</td></tr>",
            when = fmt_local(r.scheduled_start),
            num = h(&r.appointment_number),
            pat = h(&r.patient_name),
            doc = h(&r.doctor_name),
            spec = h(r.specialty_code.as_deref().unwrap_or("—")),
            status = pill(&r.status, &r.status.replace('_', " ")),
            cc = h(r.chief_complaint.as_deref().unwrap_or(""))
        ));
    }
    body.push_str("</tbody></table></div>");
    Ok(page("/appointments", "Appointments", &body))
}

async fn page_patients(State(s): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let rows: Vec<PatientRow> = sqlx::query_as(PATIENTS_SQL).fetch_all(&s.pool).await?;
    let mut body = String::new();
    body.push_str(&format!(r#"<h1>{} Patients</h1>"#, icon("users")));
    body.push_str(&format!(
        r#"<p class="lead">{} registered patients.</p>"#,
        rows.len()
    ));
    body.push_str(r#"<div class="card">"#);
    body.push_str("<table><thead><tr><th>Name</th><th>MRN</th><th class=\"num\">Age</th><th>Gender</th><th>Phone</th><th>Allergies</th><th>Chronic</th></tr></thead><tbody>");
    for p in &rows {
        let age = p
            .date_of_birth
            .map(|dob| {
                let today = Utc::now().date_naive();
                let years = today.years_since(dob).unwrap_or(0);
                format!("{years}y")
            })
            .unwrap_or_else(|| "—".into());
        body.push_str(&format!(
            "<tr><td class=\"primary\">{name}</td><td class=\"tabular\">{mrn}</td><td class=\"num tabular\">{age}</td><td>{gender}</td><td class=\"tabular\">{phone}</td><td>{allergies}</td><td>{chronic}</td></tr>",
            name = h(&p.full_name),
            mrn = h(&p.medical_record_no),
            age = h(&age),
            gender = h(p.gender.as_deref().unwrap_or("—")),
            phone = h(p.phone.as_deref().unwrap_or("—")),
            allergies = h(p.allergies.as_deref().unwrap_or("—")),
            chronic = h(p.chronic_conditions.as_deref().unwrap_or("—"))
        ));
    }
    body.push_str("</tbody></table></div>");
    Ok(page("/patients", "Patients", &body))
}

async fn page_worklist(State(s): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let items: Vec<DashboardItem> = sqlx::query_as(WORKLIST_SQL).fetch_all(&s.pool).await?;
    let mut body = String::new();
    body.push_str(&format!(r#"<h1>{} Open worklist</h1>"#, icon("alert")));
    body.push_str(
        r#"<p class="lead">Everything the clinic owes action on, sorted by urgency.</p>"#,
    );
    body.push_str(r#"<div class="card">"#);
    body.push_str(&worklist_table(&items, false));
    body.push_str(r#"</div>"#);
    Ok(page("/worklist", "Worklist", &body))
}

// ============================================================
// JSON API handlers
// ============================================================

async fn api_health(State(s): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let ok = sqlx::query_scalar::<_, String>("SELECT version()")
        .fetch_one(&s.pool)
        .await;
    match ok {
        Ok(v) => Json(serde_json::json!({ "ok": true, "server": v })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

#[derive(Serialize)]
struct DashboardEnvelope {
    total_open_items: usize,
    total_pending_sek: Decimal,
    appointments_count: i64,
    patients_count: i64,
    today: Vec<AppointmentRow>,
    upcoming: Vec<AppointmentRow>,
    recent: Vec<AppointmentRow>,
    worklist: Vec<DashboardItem>,
}

async fn api_dashboard(
    State(s): State<Arc<AppState>>,
) -> Result<Json<DashboardEnvelope>, AppError> {
    let worklist: Vec<DashboardItem> = sqlx::query_as(WORKLIST_SQL).fetch_all(&s.pool).await?;
    let today: Vec<AppointmentRow> = sqlx::query_as(TODAY_SQL).fetch_all(&s.pool).await?;
    let upcoming: Vec<AppointmentRow> = sqlx::query_as(UPCOMING_SQL).fetch_all(&s.pool).await?;
    let recent: Vec<AppointmentRow> = sqlx::query_as(RECENT_SQL).fetch_all(&s.pool).await?;
    let appointments_count: i64 = sqlx::query_scalar("SELECT count(*) FROM appointments")
        .fetch_one(&s.pool)
        .await?;
    let patients_count: i64 = sqlx::query_scalar("SELECT count(*) FROM patients")
        .fetch_one(&s.pool)
        .await?;
    let total_pending_sek: Decimal = worklist.iter().filter_map(|i| i.amount_sek).sum();
    Ok(Json(DashboardEnvelope {
        total_open_items: worklist.len(),
        total_pending_sek,
        appointments_count,
        patients_count,
        today,
        upcoming,
        recent,
        worklist,
    }))
}

async fn api_appointments(
    State(s): State<Arc<AppState>>,
) -> Result<Json<Vec<AppointmentRow>>, AppError> {
    Ok(Json(
        sqlx::query_as(APPOINTMENTS_SQL).fetch_all(&s.pool).await?,
    ))
}

async fn api_patients(State(s): State<Arc<AppState>>) -> Result<Json<Vec<PatientRow>>, AppError> {
    Ok(Json(sqlx::query_as(PATIENTS_SQL).fetch_all(&s.pool).await?))
}

async fn api_worklist(
    State(s): State<Arc<AppState>>,
) -> Result<Json<Vec<DashboardItem>>, AppError> {
    Ok(Json(sqlx::query_as(WORKLIST_SQL).fetch_all(&s.pool).await?))
}

// ============================================================
// Error type
// ============================================================

struct AppError(sqlx::Error);
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError(e)
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("clinic — DB error: {}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": self.0.to_string() })),
        )
            .into_response()
    }
}

// ============================================================
// DSN password redaction
// ============================================================

fn redact_password(url: &str) -> String {
    let Some(scheme_end) = url.find("://") else {
        return url.to_string();
    };
    let after_scheme = scheme_end + 3;
    let Some(at_offset) = url[after_scheme..].find('@') else {
        return url.to_string();
    };
    let at_pos = after_scheme + at_offset;
    let Some(colon_offset) = url[after_scheme..at_pos].find(':') else {
        return url.to_string();
    };
    let colon_pos = after_scheme + colon_offset;
    let head = &url[..colon_pos];
    let tail = &url[at_pos..];
    format!("{head}:***{tail}")
}

#[cfg(test)]
mod tests {
    use super::{h, icon, redact_password};

    #[test]
    fn redacts_password_between_colon_and_at() {
        assert_eq!(
            redact_password("postgres://clinic_app:Gaza1950!@localhost/clinic_prod"),
            "postgres://clinic_app:***@localhost/clinic_prod"
        );
    }

    #[test]
    fn leaves_url_without_password_unchanged() {
        assert_eq!(
            redact_password("postgres://user@localhost/db"),
            "postgres://user@localhost/db"
        );
    }

    #[test]
    fn html_escape_handles_all_xss_metachars() {
        assert_eq!(
            h("<script>alert(\"x\")</script>"),
            "&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;"
        );
        assert_eq!(h("A & B"), "A &amp; B");
        assert_eq!(h("it's"), "it&#39;s");
    }

    #[test]
    fn icon_returns_svg_for_known_names_and_empty_for_unknown() {
        for name in [
            "dashboard",
            "calendar",
            "users",
            "list-check",
            "clock",
            "activity",
            "alert",
            "dollar",
            "plus-circle",
            "stethoscope",
        ] {
            let svg = icon(name);
            assert!(
                svg.starts_with("<svg"),
                "icon {name} should start with <svg"
            );
            assert!(
                svg.ends_with("</svg>"),
                "icon {name} should end with </svg>"
            );
        }
        assert_eq!(icon("nonexistent"), "");
    }
}
