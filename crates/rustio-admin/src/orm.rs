//! PostgreSQL-backed ORM. A thin shim over sqlx — `Db`, `Model`,
//! `Value`, `Row` — and a handful of generic CRUD helpers. Hand-written
//! `impl Model` is the contract; users keep writing SQL where it matters.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{PgPool, Row as SqlxRow};
use uuid::Uuid;

use crate::error::{Error, Result};

/// Shared handle to the database. Cheap to clone; every handler gets
/// its own clone.
#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    /// Connect with sensible production defaults: 30 max connections,
    /// 1s acquire timeout, 5min idle timeout.
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with(url, DbOptions::default()).await
    }

    pub async fn connect_with(url: &str, opts: DbOptions) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(opts.max_connections)
            .min_connections(opts.min_connections)
            .acquire_timeout(opts.acquire_timeout)
            .idle_timeout(Some(opts.idle_timeout))
            .max_lifetime(Some(opts.max_lifetime))
            .connect(url)
            .await
            .map_err(|e| Error::Internal(format!("db connect failed: {e}")))?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| Error::Internal(format!("health check: {e}")))
    }

    /// Test-only constructor. Builds a `Db` whose pool is real but
    /// never opens a TCP connection — `connect_lazy_with` defers that
    /// until `.acquire()` is called. Tests that don't actually hit the
    /// database can use this.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn for_testing_no_connection() -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://test:test@127.0.0.1:1/never_used")
            .expect("connect_lazy never fails on a syntactically valid URL");
        Self { pool }
    }
}

#[derive(Clone, Debug)]
pub struct DbOptions {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for DbOptions {
    fn default() -> Self {
        Self {
            max_connections: 30,
            min_connections: 2,
            acquire_timeout: Duration::from_secs(1),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

/// The value types the framework understands. Kept small on purpose.
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    I32(i32),
    I64(i64),
    Bool(bool),
    Text(String),
    DateTime(DateTime<Utc>),
    Uuid(Uuid),
    Json(JsonValue),
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::I32(v)
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::I64(v)
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}
impl<'a> From<&'a str> for Value {
    fn from(v: &'a str) -> Self {
        Value::Text(v.to_string())
    }
}
impl From<DateTime<Utc>> for Value {
    fn from(v: DateTime<Utc>) -> Self {
        Value::DateTime(v)
    }
}
impl From<Uuid> for Value {
    fn from(v: Uuid) -> Self {
        Value::Uuid(v)
    }
}
impl From<JsonValue> for Value {
    fn from(v: JsonValue) -> Self {
        Value::Json(v)
    }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

pub struct Row<'a> {
    inner: &'a PgRow,
}

impl<'a> Row<'a> {
    pub fn from_pg(row: &'a PgRow) -> Self {
        Self { inner: row }
    }

    pub fn get_i32(&self, col: &str) -> Result<i32> {
        self.inner
            .try_get::<i32, _>(col)
            .map_err(|e| Error::Internal(format!("get_i32({col}): {e}")))
    }
    pub fn get_i64(&self, col: &str) -> Result<i64> {
        self.inner
            .try_get::<i64, _>(col)
            .map_err(|e| Error::Internal(format!("get_i64({col}): {e}")))
    }
    pub fn get_optional_i64(&self, col: &str) -> Result<Option<i64>> {
        self.inner
            .try_get::<Option<i64>, _>(col)
            .map_err(|e| Error::Internal(format!("{col}: {e}")))
    }
    pub fn get_bool(&self, col: &str) -> Result<bool> {
        self.inner
            .try_get::<bool, _>(col)
            .map_err(|e| Error::Internal(format!("get_bool({col}): {e}")))
    }
    pub fn get_string(&self, col: &str) -> Result<String> {
        self.inner
            .try_get::<String, _>(col)
            .map_err(|e| Error::Internal(format!("get_string({col}): {e}")))
    }
    pub fn get_optional_string(&self, col: &str) -> Result<Option<String>> {
        self.inner
            .try_get::<Option<String>, _>(col)
            .map_err(|e| Error::Internal(format!("{col}: {e}")))
    }
    pub fn get_datetime(&self, col: &str) -> Result<DateTime<Utc>> {
        self.inner
            .try_get::<DateTime<Utc>, _>(col)
            .map_err(|e| Error::Internal(format!("{col}: {e}")))
    }
    pub fn get_optional_datetime(&self, col: &str) -> Result<Option<DateTime<Utc>>> {
        self.inner
            .try_get::<Option<DateTime<Utc>>, _>(col)
            .map_err(|e| Error::Internal(format!("{col}: {e}")))
    }
    pub fn get_uuid(&self, col: &str) -> Result<Uuid> {
        self.inner
            .try_get::<Uuid, _>(col)
            .map_err(|e| Error::Internal(format!("get_uuid({col}): {e}")))
    }
    pub fn get_json(&self, col: &str) -> Result<JsonValue> {
        self.inner
            .try_get::<JsonValue, _>(col)
            .map_err(|e| Error::Internal(format!("{col}: {e}")))
    }
}

pub trait Model: Send + Sync + Sized + 'static {
    const TABLE: &'static str;
    const COLUMNS: &'static [&'static str];
    const INSERT_COLUMNS: &'static [&'static str];

    fn id(&self) -> i64;
    fn from_row(row: Row<'_>) -> Result<Self>;
    fn insert_values(&self) -> Vec<Value>;
}

// ---- Generic CRUD helpers -----------------------------------------------

pub async fn all<M: Model>(db: &Db) -> Result<Vec<M>> {
    let sql = format!(
        "SELECT {} FROM {} ORDER BY id DESC",
        M::COLUMNS.join(", "),
        M::TABLE
    );
    let rows = sqlx::query(&sql).fetch_all(db.pool()).await?;
    rows.iter().map(|r| M::from_row(Row::from_pg(r))).collect()
}

pub async fn page<M: Model>(db: &Db, limit: i64, offset: i64) -> Result<Vec<M>> {
    let sql = format!(
        "SELECT {} FROM {} ORDER BY id DESC LIMIT $1 OFFSET $2",
        M::COLUMNS.join(", "),
        M::TABLE
    );
    let rows = sqlx::query(&sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(db.pool())
        .await?;
    rows.iter().map(|r| M::from_row(Row::from_pg(r))).collect()
}

pub async fn count<M: Model>(db: &Db) -> Result<i64> {
    let sql = format!("SELECT COUNT(*) AS c FROM {}", M::TABLE);
    let row = sqlx::query(&sql).fetch_one(db.pool()).await?;
    row.try_get::<i64, _>("c")
        .map_err(|e| Error::Internal(format!("count: {e}")))
}

pub async fn find<M: Model>(db: &Db, id: i64) -> Result<Option<M>> {
    let sql = format!(
        "SELECT {} FROM {} WHERE id = $1",
        M::COLUMNS.join(", "),
        M::TABLE
    );
    let row = sqlx::query(&sql).bind(id).fetch_optional(db.pool()).await?;
    match row {
        Some(r) => Ok(Some(M::from_row(Row::from_pg(&r))?)),
        None => Ok(None),
    }
}

pub async fn create<M: Model>(db: &Db, model: &M) -> Result<i64> {
    let cols = M::INSERT_COLUMNS.join(", ");
    let placeholders: Vec<String> = (1..=M::INSERT_COLUMNS.len())
        .map(|i| format!("${i}"))
        .collect();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({}) RETURNING id",
        M::TABLE,
        cols,
        placeholders.join(", ")
    );
    let mut query = sqlx::query(&sql);
    for value in model.insert_values() {
        query = bind_value(query, value);
    }
    let row = query.fetch_one(db.pool()).await?;
    let id: i64 = row
        .try_get("id")
        .map_err(|e| Error::Internal(format!("returning id: {e}")))?;
    Ok(id)
}

pub async fn update<M: Model>(db: &Db, id: i64, model: &M) -> Result<()> {
    let sets: Vec<String> = M::INSERT_COLUMNS
        .iter()
        .enumerate()
        .map(|(i, col)| format!("{col} = ${}", i + 1))
        .collect();
    let sql = format!(
        "UPDATE {} SET {} WHERE id = ${}",
        M::TABLE,
        sets.join(", "),
        M::INSERT_COLUMNS.len() + 1
    );
    let mut query = sqlx::query(&sql);
    for value in model.insert_values() {
        query = bind_value(query, value);
    }
    query = query.bind(id);
    query.execute(db.pool()).await?;
    Ok(())
}

pub async fn delete<M: Model>(db: &Db, id: i64) -> Result<()> {
    let sql = format!("DELETE FROM {} WHERE id = $1", M::TABLE);
    sqlx::query(&sql).bind(id).execute(db.pool()).await?;
    Ok(())
}

fn bind_value<'a>(
    q: sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>,
    v: Value,
) -> sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match v {
        Value::Null => q.bind(None::<i64>),
        Value::I32(n) => q.bind(n),
        Value::I64(n) => q.bind(n),
        Value::Bool(b) => q.bind(b),
        Value::Text(s) => q.bind(s),
        Value::DateTime(d) => q.bind(d),
        Value::Uuid(u) => q.bind(u),
        Value::Json(j) => q.bind(j),
    }
}
