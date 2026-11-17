//! SQLite: Verbindung, Migrationen, Zeilentypen. Vorbild: firsttris/haul crates/haul/src/db.rs

use anyhow::Result;
use serde::Serialize;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{FromRow, SqlitePool};
use std::path::Path;
use std::str::FromStr;

pub type Db = SqlitePool;

pub async fn connect(path: &Path) -> Result<Db> {
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(10));
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub name: String,
    #[serde(skip)]
    pub password_hash: String,
    pub role: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub epsg: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    pub id: i64,
    pub site_id: i64,
    pub name: String,
    pub file_name: String,
    pub file_size: i64,
    pub captured_at: i64,
    pub status: String,
    #[serde(serialize_with = "raw_json")]
    pub info: Option<String>,
    pub error: Option<String>,
    pub created_at: i64,
    /// Orthofoto: Größe und Ecken (JSON), sonst leer
    #[serde(serialize_with = "raw_json")]
    pub ortho: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RegionRow {
    pub id: i64,
    pub site_id: i64,
    pub name: String,
    #[serde(serialize_with = "raw_json_required")]
    pub geojson: String,
    pub area_m2: f64,
    pub created_at: i64,
    /// Massenziel: Material (frei), Soll-Menge in m³, `cut` (Abtrag) oder `fill` (Auftrag)
    pub material: Option<String>,
    pub target_m3: Option<f64>,
    pub target_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: i64,
    pub site_id: i64,
    pub kind: String,
    #[serde(serialize_with = "raw_json_required")]
    pub params: String,
    pub status: String,
    #[serde(serialize_with = "raw_json")]
    pub result: Option<String>,
    pub error: Option<String>,
    pub created_by: Option<i64>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    /// 0–100 bei langen Aufträgen (Photogrammetrie), sonst leer
    pub progress: Option<f64>,
}

/// Gespeichertes JSON als Objekt ausgeben, nicht als String.
pub fn raw_json<S: serde::Serializer>(v: &Option<String>, s: S) -> Result<S::Ok, S::Error> {
    match v.as_deref().map(serde_json::from_str::<serde_json::Value>) {
        Some(Ok(j)) => s.serialize_some(&j),
        _ => s.serialize_none(),
    }
}

pub fn raw_json_required<S: serde::Serializer>(v: &str, s: S) -> Result<S::Ok, S::Error> {
    let j: serde_json::Value = serde_json::from_str(v).unwrap_or(serde_json::Value::Null);
    serde::Serialize::serialize(&j, s)
}

pub mod status {
    pub const UPLOADED: &str = "uploaded";
    pub const READY: &str = "ready";
    pub const FAILED: &str = "failed";
    pub const QUEUED: &str = "queued";
    pub const RUNNING: &str = "running";
    pub const DONE: &str = "done";
    /// Photogrammetrie: Fotos werden noch hochgeladen
    pub const DRAFT: &str = "draft";
}

pub async fn get_site(db: &Db, id: i64) -> Result<Option<Site>> {
    Ok(sqlx::query_as("SELECT * FROM sites WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await?)
}

pub async fn get_capture(db: &Db, id: i64) -> Result<Option<Capture>> {
    Ok(sqlx::query_as("SELECT * FROM captures WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await?)
}

pub async fn get_job(db: &Db, id: i64) -> Result<Option<Job>> {
    Ok(sqlx::query_as("SELECT * FROM jobs WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await?)
}

pub async fn user_count(db: &Db) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_and_json_columns() {
        let dir = tempfile::tempdir().unwrap();
        let db = connect(&dir.path().join("t.db")).await.unwrap();
        sqlx::query("INSERT INTO sites(name, created_at) VALUES('A', 1)")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO jobs(site_id, kind, params, status, result, created_at) VALUES(1, 'dem', '{\"cell\":0.5}', 'done', '{\"x\":1}', 2)",
        )
        .execute(&db)
        .await
        .unwrap();
        let job = get_job(&db, 1).await.unwrap().unwrap();
        let v = serde_json::to_value(&job).unwrap();
        assert_eq!(v["params"]["cell"], 0.5);
        assert_eq!(v["result"]["x"], 1);
        assert_eq!(v["siteId"], 1);
        // Löschen der Baustelle löscht ihre Aufträge (Fremdschlüssel)
        sqlx::query("DELETE FROM sites WHERE id = 1")
            .execute(&db)
            .await
            .unwrap();
        assert!(get_job(&db, 1).await.unwrap().is_none());
    }
}
