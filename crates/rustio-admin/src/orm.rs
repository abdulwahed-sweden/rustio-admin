//! PostgreSQL-backed ORM. A thin shim over sqlx — `Db`, `Model`,
//! `Value`, `Row` — and a handful of generic CRUD helpers. Hand-written
//! `impl Model` is the contract; users keep writing SQL where it matters.

use std::time::Duration;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value as JsonValue;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{PgPool, Row as SqlxRow};
use uuid::Uuid;

use crate::error::{Error, Result};

// public:
/// Shared handle to the database. Cheap to clone; every handler gets
/// its own clone.
#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    // public:
    /// Connect with sensible production defaults: 30 max connections,
    /// 1s acquire timeout, 5min idle timeout.
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with(url, DbOptions::default()).await
    }

    // public:
    pub async fn connect_with(url: &str, opts: DbOptions) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(opts.max_connections)
            .min_connections(opts.min_connections)
            .acquire_timeout(opts.acquire_timeout)
            .idle_timeout(Some(opts.idle_timeout))
            .max_lifetime(Some(opts.max_lifetime))
            // Suppress benign Postgres NOTICE chatter so harmless
            // `CREATE … IF NOT EXISTS` and `ADD COLUMN … IF NOT EXISTS`
            // re-runs don't blast 60+ "relation already exists, skipping"
            // lines into the operator's log on every boot. WARNING +
            // ERROR + LOG levels still surface — only the chatty NOTICE
            // band is silenced. Set per-connection so reconnects in the
            // pool stay quiet too.
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    use sqlx::Executor;
                    conn.execute("SET client_min_messages = warning")
                        .await
                        .map(|_| ())
                })
            })
            .connect(url)
            .await
            .map_err(|e| Error::Internal(format!("db connect failed: {e}")))?;
        Ok(Self { pool })
    }

    // public:
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // public:
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

// public:
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

// public:
/// The value types the framework understands. Kept small on purpose.
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    I32(i32),
    I64(i64),
    F64(f64),
    Decimal(Decimal),
    Bool(bool),
    Text(String),
    DateTime(DateTime<Utc>),
    Date(NaiveDate),
    Time(NaiveTime),
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
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::F64(v)
    }
}
impl From<Decimal> for Value {
    fn from(v: Decimal) -> Self {
        Value::Decimal(v)
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
impl From<NaiveDate> for Value {
    fn from(v: NaiveDate) -> Self {
        Value::Date(v)
    }
}
impl From<NaiveTime> for Value {
    fn from(v: NaiveTime) -> Self {
        Value::Time(v)
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

// public:
pub struct Row<'a> {
    inner: &'a PgRow,
}

impl<'a> Row<'a> {
    // public:
    pub fn from_pg(row: &'a PgRow) -> Self {
        Self { inner: row }
    }

    // public:
    pub fn get_i32(&self, col: &str) -> Result<i32> {
        self.inner
            .try_get::<i32, _>(col)
            .map_err(|e| Error::Internal(format!("get_i32({col}): {e}")))
    }
    // public:
    pub fn get_i64(&self, col: &str) -> Result<i64> {
        self.inner
            .try_get::<i64, _>(col)
            .map_err(|e| Error::Internal(format!("get_i64({col}): {e}")))
    }
    // public:
    pub fn get_f64(&self, col: &str) -> Result<f64> {
        self.inner
            .try_get::<f64, _>(col)
            .map_err(|e| Error::Internal(format!("get_f64({col}): {e}")))
    }
    // public:
    pub fn get_decimal(&self, col: &str) -> Result<Decimal> {
        self.inner
            .try_get::<Decimal, _>(col)
            .map_err(|e| Error::Internal(format!("get_decimal({col}): {e}")))
    }
    // public:
    pub fn get_optional_i64(&self, col: &str) -> Result<Option<i64>> {
        self.inner
            .try_get::<Option<i64>, _>(col)
            .map_err(|e| Error::Internal(format!("{col}: {e}")))
    }
    // public:
    pub fn get_bool(&self, col: &str) -> Result<bool> {
        self.inner
            .try_get::<bool, _>(col)
            .map_err(|e| Error::Internal(format!("get_bool({col}): {e}")))
    }
    // public:
    pub fn get_string(&self, col: &str) -> Result<String> {
        self.inner
            .try_get::<String, _>(col)
            .map_err(|e| Error::Internal(format!("get_string({col}): {e}")))
    }
    // public:
    pub fn get_optional_string(&self, col: &str) -> Result<Option<String>> {
        self.inner
            .try_get::<Option<String>, _>(col)
            .map_err(|e| Error::Internal(format!("{col}: {e}")))
    }
    // public:
    pub fn get_datetime(&self, col: &str) -> Result<DateTime<Utc>> {
        self.inner
            .try_get::<DateTime<Utc>, _>(col)
            .map_err(|e| Error::Internal(format!("{col}: {e}")))
    }
    // public:
    pub fn get_date(&self, col: &str) -> Result<NaiveDate> {
        self.inner
            .try_get::<NaiveDate, _>(col)
            .map_err(|e| Error::Internal(format!("get_date({col}): {e}")))
    }
    // public:
    pub fn get_time(&self, col: &str) -> Result<NaiveTime> {
        self.inner
            .try_get::<NaiveTime, _>(col)
            .map_err(|e| Error::Internal(format!("get_time({col}): {e}")))
    }
    // public:
    pub fn get_optional_datetime(&self, col: &str) -> Result<Option<DateTime<Utc>>> {
        self.inner
            .try_get::<Option<DateTime<Utc>>, _>(col)
            .map_err(|e| Error::Internal(format!("{col}: {e}")))
    }
    // public:
    pub fn get_uuid(&self, col: &str) -> Result<Uuid> {
        self.inner
            .try_get::<Uuid, _>(col)
            .map_err(|e| Error::Internal(format!("get_uuid({col}): {e}")))
    }
    // public:
    pub fn get_json(&self, col: &str) -> Result<JsonValue> {
        self.inner
            .try_get::<JsonValue, _>(col)
            .map_err(|e| Error::Internal(format!("{col}: {e}")))
    }
}

// public:
pub trait Model: Send + Sync + Sized + 'static {
    const TABLE: &'static str;
    const COLUMNS: &'static [&'static str];
    const INSERT_COLUMNS: &'static [&'static str];

    fn id(&self) -> i64;
    fn from_row(row: Row<'_>) -> Result<Self>;
    fn insert_values(&self) -> Vec<Value>;
}

// ---- Generic CRUD helpers -----------------------------------------------

// public:
pub async fn all<M: Model>(db: &Db) -> Result<Vec<M>> {
    let sql = format!(
        "SELECT {} FROM {} ORDER BY id DESC",
        M::COLUMNS.join(", "),
        M::TABLE
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_all(db.pool())
        .await?;
    rows.iter().map(|r| M::from_row(Row::from_pg(r))).collect()
}

// public:
pub async fn page<M: Model>(db: &Db, limit: i64, offset: i64) -> Result<Vec<M>> {
    let sql = format!(
        "SELECT {} FROM {} ORDER BY id DESC LIMIT $1 OFFSET $2",
        M::COLUMNS.join(", "),
        M::TABLE
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(limit)
        .bind(offset)
        .fetch_all(db.pool())
        .await?;
    rows.iter().map(|r| M::from_row(Row::from_pg(r))).collect()
}

// public:
pub async fn count<M: Model>(db: &Db) -> Result<i64> {
    let sql = format!("SELECT COUNT(*) AS c FROM {}", M::TABLE);
    let row = sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_one(db.pool())
        .await?;
    row.try_get::<i64, _>("c")
        .map_err(|e| Error::Internal(format!("count: {e}")))
}

// public:
pub async fn find<M: Model>(db: &Db, id: i64) -> Result<Option<M>> {
    let sql = format!(
        "SELECT {} FROM {} WHERE id = $1",
        M::COLUMNS.join(", "),
        M::TABLE
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(db.pool())
        .await?;
    match row {
        Some(r) => Ok(Some(M::from_row(Row::from_pg(&r))?)),
        None => Ok(None),
    }
}

// public:
pub async fn create<M: Model>(db: &Db, model: &M) -> Result<i64> {
    let cols = M::INSERT_COLUMNS.join(", ");
    let (placeholders, bound) = placeholders_for(model.insert_values());
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({}) RETURNING id",
        M::TABLE,
        cols,
        placeholders.join(", ")
    );
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    for value in bound {
        query = bind_value(query, value);
    }
    let row = query.fetch_one(db.pool()).await?;
    let id: i64 = row
        .try_get("id")
        .map_err(|e| Error::Internal(format!("returning id: {e}")))?;
    Ok(id)
}

// public:
pub async fn update<M: Model>(db: &Db, id: i64, model: &M) -> Result<()> {
    let (placeholders, bound) = placeholders_for(model.insert_values());
    let sets: Vec<String> = M::INSERT_COLUMNS
        .iter()
        .zip(&placeholders)
        .map(|(col, slot)| format!("{col} = {slot}"))
        .collect();
    let sql = format!(
        "UPDATE {} SET {} WHERE id = ${}",
        M::TABLE,
        sets.join(", "),
        bound.len() + 1
    );
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    for value in bound {
        query = bind_value(query, value);
    }
    query = query.bind(id);
    query.execute(db.pool()).await?;
    Ok(())
}

// public:
pub async fn delete<M: Model>(db: &Db, id: i64) -> Result<()> {
    let sql = format!("DELETE FROM {} WHERE id = $1", M::TABLE);
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .execute(db.pool())
        .await?;
    Ok(())
}

/// Decide, per value, whether it travels as a bound parameter or as the SQL
/// literal `NULL`, renumbering the placeholders around the ones that drop out.
///
/// A bound NULL has to carry a type, and [`Value::Null`] does not know one:
/// the blanket `From<Option<T>>` erases it. Binding a guess — it used to be
/// `None::<i64>` — makes Postgres reject the statement the moment the target
/// column is not a `bigint`:
///
/// ```text
/// column "notified_at" is of type timestamp with time zone
/// but expression is of type bigint
/// ```
///
/// which left any row holding a NULL in a nullable non-`bigint` column
/// impossible to update. The literal has no such problem: `SET col = NULL` and
/// `VALUES (NULL)` take their type from the column being written, so Postgres
/// never has to infer one.
///
/// Only the keyword is ever interpolated, never a value, so this stays exactly
/// as injection-safe as the fully parameterised form it replaces.
fn placeholders_for(values: Vec<Value>) -> (Vec<String>, Vec<Value>) {
    let mut placeholders = Vec::with_capacity(values.len());
    let mut bound = Vec::with_capacity(values.len());
    for value in values {
        if matches!(value, Value::Null) {
            placeholders.push("NULL".to_string());
        } else {
            bound.push(value);
            placeholders.push(format!("${}", bound.len()));
        }
    }
    (placeholders, bound)
}

fn bind_value<'a>(
    q: sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>,
    v: Value,
) -> sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match v {
        // Unreachable from `create` / `update`, which route NULLs through
        // `placeholders_for` as SQL literals. Kept so the match stays total.
        Value::Null => q.bind(None::<i64>),
        Value::I32(n) => q.bind(n),
        Value::I64(n) => q.bind(n),
        Value::F64(n) => q.bind(n),
        Value::Decimal(d) => q.bind(d),
        Value::Bool(b) => q.bind(b),
        Value::Text(s) => q.bind(s),
        Value::DateTime(d) => q.bind(d),
        Value::Date(d) => q.bind(d),
        Value::Time(t) => q.bind(t),
        Value::Uuid(u) => q.bind(u),
        Value::Json(j) => q.bind(j),
    }
}

#[cfg(test)]
mod value_conversion_tests {
    use super::Value;
    use chrono::{NaiveDate, NaiveTime};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use uuid::Uuid;

    #[test]
    fn scalar_from_impls_map_to_their_variants() {
        assert!(matches!(Value::from(3.5_f64), Value::F64(v) if v == 3.5));
        let dec = Decimal::from_str("19.99").unwrap();
        assert!(matches!(Value::from(dec), Value::Decimal(v) if v == dec));
        let d = NaiveDate::from_ymd_opt(2026, 6, 2).unwrap();
        assert!(matches!(Value::from(d), Value::Date(v) if v == d));
        let t = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
        assert!(matches!(Value::from(t), Value::Time(v) if v == t));
        let u = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440000);
        assert!(matches!(Value::from(u), Value::Uuid(v) if v == u));
    }
}

#[cfg(test)]
mod null_placeholder_tests {
    use super::{placeholders_for, Value};

    #[test]
    fn every_value_gets_a_numbered_placeholder_when_none_are_null() {
        let (slots, bound) = placeholders_for(vec![
            Value::Text("a".into()),
            Value::I64(1),
            Value::Bool(true),
        ]);
        assert_eq!(slots, ["$1", "$2", "$3"]);
        assert_eq!(bound.len(), 3);
    }

    #[test]
    fn nulls_become_literals_and_do_not_consume_a_placeholder() {
        let (slots, bound) =
            placeholders_for(vec![Value::Text("a".into()), Value::Null, Value::I64(7)]);
        assert_eq!(slots, ["$1", "NULL", "$2"]);
        assert_eq!(bound.len(), 2, "the NULL must not be bound");
    }

    #[test]
    fn leading_and_trailing_nulls_renumber_correctly() {
        let (slots, bound) = placeholders_for(vec![
            Value::Null,
            Value::Text("a".into()),
            Value::Null,
            Value::I32(3),
            Value::Null,
        ]);
        assert_eq!(slots, ["NULL", "$1", "NULL", "$2", "NULL"]);
        assert_eq!(bound.len(), 2);
    }

    #[test]
    fn an_all_null_row_binds_nothing() {
        let (slots, bound) = placeholders_for(vec![Value::Null, Value::Null]);
        assert_eq!(slots, ["NULL", "NULL"]);
        assert!(bound.is_empty());
        // `update` puts the id at `bound.len() + 1`, so it must still be $1.
        assert_eq!(bound.len() + 1, 1);
    }

    #[test]
    fn placeholder_count_always_matches_the_values_that_will_be_bound() {
        let values = vec![
            Value::Null,
            Value::I64(1),
            Value::Null,
            Value::Text("x".into()),
            Value::Bool(false),
        ];
        let expected = values.iter().filter(|v| !matches!(v, Value::Null)).count();
        let (slots, bound) = placeholders_for(values);
        assert_eq!(bound.len(), expected);
        assert_eq!(
            slots.iter().filter(|s| s.starts_with('$')).count(),
            expected
        );
    }

    #[test]
    fn no_value_is_ever_interpolated_into_the_sql() {
        // Only "$n" and the NULL keyword may appear; a value reaching the SQL
        // string would be an injection vector.
        let (slots, _) = placeholders_for(vec![
            Value::Text("'; DROP TABLE orders; --".into()),
            Value::Null,
        ]);
        assert_eq!(slots, ["$1", "NULL"]);
    }
}
