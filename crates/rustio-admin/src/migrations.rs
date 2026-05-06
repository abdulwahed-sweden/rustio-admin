//! Versioned SQL migrations for PostgreSQL. Transactional; the
//! `rustio_migrations` tracking table records which versions have been
//! applied.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::orm::Db;

pub struct MigrationFile {
    pub version: i64,
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct ApplyOptions {
    pub verbose: bool,
}

pub async fn apply(db: &Db, dir: impl AsRef<Path>) -> Result<Vec<String>> {
    apply_with(db, dir, ApplyOptions::default()).await
}

pub async fn apply_with(db: &Db, dir: impl AsRef<Path>, opts: ApplyOptions) -> Result<Vec<String>> {
    ensure_tracking_table(db).await?;

    let files = discover(dir.as_ref())?;
    let already = applied_versions(db).await?;
    let mut newly = Vec::new();

    for file in files {
        if already.contains(&file.version) {
            continue;
        }
        if opts.verbose {
            log::info!("applying migration {:04}_{}", file.version, file.name);
        }

        let sql = fs::read_to_string(&file.path)?;
        let statements = split_statements(&sql);

        let mut tx = db
            .pool()
            .begin()
            .await
            .map_err(|e| Error::Internal(format!("begin tx: {e}")))?;

        for stmt in &statements {
            let trimmed = stmt.trim();
            if trimmed.is_empty() {
                continue;
            }
            sqlx::query(trimmed)
                .execute(&mut *tx)
                .await
                .map_err(|e| Error::Internal(format!("migration {} failed: {e}", file.name)))?;
        }

        sqlx::query(
            "INSERT INTO rustio_migrations (version, name, applied_at)
             VALUES ($1, $2, NOW())",
        )
        .bind(file.version)
        .bind(&file.name)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Internal(format!("tracking insert: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| Error::Internal(format!("commit: {e}")))?;

        newly.push(file.name.clone());
    }

    Ok(newly)
}

pub async fn applied_versions(db: &Db) -> Result<Vec<i64>> {
    ensure_tracking_table(db).await?;
    let rows =
        sqlx::query_scalar::<_, i64>("SELECT version FROM rustio_migrations ORDER BY version ASC")
            .fetch_all(db.pool())
            .await?;
    Ok(rows)
}

pub async fn status(db: &Db, dir: impl AsRef<Path>) -> Result<Vec<(String, bool)>> {
    let applied = applied_versions(db).await?;
    let files = discover(dir.as_ref())?;
    Ok(files
        .into_iter()
        .map(|f| {
            (
                format!("{:04}_{}", f.version, f.name),
                applied.contains(&f.version),
            )
        })
        .collect())
}

pub fn generate(dir: impl AsRef<Path>, name: &str) -> Result<PathBuf> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir)?;
    let existing = discover(dir).unwrap_or_default();
    let next = existing.iter().map(|m| m.version).max().unwrap_or(0) + 1;
    let filename = format!("{:04}_{}.sql", next, slugify(name));
    let path = dir.join(filename);
    fs::write(&path, format!("-- {}\n\n", name))?;
    Ok(path)
}

fn discover(dir: &Path) -> Result<Vec<MigrationFile>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        let (ver_part, name_part) = match stem.split_once('_') {
            Some(p) => p,
            None => continue,
        };
        let version: i64 = match ver_part.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        out.push(MigrationFile {
            version,
            name: name_part.to_string(),
            path,
        });
    }
    out.sort_by_key(|m| m.version);
    Ok(out)
}

async fn ensure_tracking_table(db: &Db) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_migrations (
            version    BIGINT PRIMARY KEY,
            name       TEXT NOT NULL,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(db.pool())
    .await?;
    Ok(())
}

/// Split a multi-statement SQL file on `;`, but not on `;` inside
/// quoted strings, dollar-quoted bodies (Postgres PL/pgSQL), or
/// `--` / `/* */` comments.
fn split_statements(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();
    let mut in_string = false;
    let mut in_dollar = false;
    let mut dollar_tag = String::new();
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(c) = chars.next() {
        if in_line_comment {
            current.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            continue;
        }
        if in_block_comment {
            current.push(c);
            if c == '*' && chars.peek() == Some(&'/') {
                current.push(chars.next().unwrap());
                in_block_comment = false;
            }
            continue;
        }
        if in_dollar {
            current.push(c);
            if c == '$' {
                let rest: String = chars.clone().take(dollar_tag.len()).collect();
                if rest == dollar_tag {
                    for _ in 0..dollar_tag.len() {
                        current.push(chars.next().unwrap());
                    }
                    in_dollar = false;
                    dollar_tag.clear();
                }
            }
            continue;
        }
        if in_string {
            current.push(c);
            if c == '\'' {
                if chars.peek() == Some(&'\'') {
                    current.push(chars.next().unwrap());
                } else {
                    in_string = false;
                }
            }
            continue;
        }

        match c {
            '\'' => {
                in_string = true;
                current.push(c);
            }
            '-' if chars.peek() == Some(&'-') => {
                in_line_comment = true;
                current.push(c);
            }
            '/' if chars.peek() == Some(&'*') => {
                in_block_comment = true;
                current.push(c);
            }
            '$' => {
                let mut tag = String::from("$");
                let mut clone = chars.clone();
                while let Some(&nc) = clone.peek() {
                    if nc == '$' {
                        tag.push('$');
                        break;
                    }
                    if nc.is_alphanumeric() || nc == '_' {
                        tag.push(nc);
                        clone.next();
                    } else {
                        break;
                    }
                }
                if tag.ends_with('$') && tag.len() >= 2 {
                    for _ in 1..tag.len() {
                        current.push(chars.next().unwrap());
                    }
                    current.insert(current.len() - tag.len() + 1, '$');
                    current.push('$');
                    dollar_tag = tag;
                    in_dollar = true;
                } else {
                    current.push(c);
                }
            }
            ';' => {
                out.push(std::mem::take(&mut current));
            }
            other => current.push(other),
        }
    }

    if !current.trim().is_empty() {
        out.push(current);
    }
    out
}

fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_ignores_semicolon_in_string() {
        let sql = "INSERT INTO t VALUES ('a;b'); SELECT 1;";
        let parts = split_statements(sql);
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn split_ignores_line_comments() {
        let sql = "SELECT 1; -- comment with ;\nSELECT 2;";
        let parts = split_statements(sql);
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn slugify_lowercases_and_replaces() {
        assert_eq!(slugify("Add Users Table!"), "add_users_table_");
    }
}
