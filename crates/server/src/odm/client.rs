//! HTTP-Client für NodeODM (OpenDroneMap als Dienst), REST-API wie in
//! <https://github.com/OpenDroneMap/NodeODM/blob/master/docs/index.adoc>.
//! Fehler meldet NodeODM teils mit Status 200 und `{"error": "…"}`; beides wird zum Fehlerwert.

use anyhow::{anyhow, bail, Context, Result};
use futures::TryStreamExt;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Zustand einer Aufgabe in NodeODM (`status.code`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Running,
    Failed,
    Completed,
    Canceled,
}

impl TaskStatus {
    pub fn from_code(code: i64) -> Option<Self> {
        Some(match code {
            10 => TaskStatus::Queued,
            20 => TaskStatus::Running,
            30 => TaskStatus::Failed,
            40 => TaskStatus::Completed,
            50 => TaskStatus::Canceled,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub status: TaskStatus,
    /// 0–100
    pub progress: f64,
    pub error: Option<String>,
    pub processing_ms: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub version: String,
    #[serde(default)]
    pub task_queue_count: i64,
    #[serde(default)]
    pub engine_version: Option<String>,
}

#[derive(Clone)]
pub struct NodeOdm {
    base: String,
    token: Option<String>,
    http: reqwest::Client,
}

/// rustls braucht einen Krypto-Anbieter; ring statt aws-lc (kein cmake im musl-Build)
fn install_crypto() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// `{"error": "…"}` als Fehler, sonst der Wert
fn checked(v: Value) -> Result<Value> {
    match v.get("error").and_then(Value::as_str) {
        Some(e) => bail!("NodeODM: {e}"),
        None => Ok(v),
    }
}

pub fn parse_task_info(v: &Value) -> Result<TaskInfo> {
    let code = v["status"]["code"]
        .as_i64()
        .context("NodeODM: Antwort ohne status.code")?;
    Ok(TaskInfo {
        status: TaskStatus::from_code(code).ok_or_else(|| anyhow!("NodeODM: Status {code}"))?,
        progress: v["progress"].as_f64().unwrap_or(0.0).clamp(0.0, 100.0),
        error: v["status"]["errorMessage"].as_str().map(String::from),
        processing_ms: v["processingTime"].as_i64().unwrap_or(0),
    })
}

impl NodeOdm {
    pub fn new(base: &str, token: Option<String>) -> Result<Self> {
        install_crypto();
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()?;
        Ok(NodeOdm {
            base: base.trim_end_matches('/').to_string(),
            token,
            http,
        })
    }

    fn url(&self, path: &str) -> String {
        match &self.token {
            Some(t) => format!("{}{path}?token={t}", self.base),
            None => format!("{}{path}", self.base),
        }
    }

    async fn json(&self, req: reqwest::RequestBuilder) -> Result<Value> {
        let res = req.send().await.context("NodeODM nicht erreichbar")?;
        let status = res.status();
        let body = res.text().await?;
        let v: Value = serde_json::from_str(&body)
            .with_context(|| format!("NodeODM: unerwartete Antwort ({status}): {body:.200}"))?;
        let v = checked(v)?;
        if !status.is_success() {
            bail!("NodeODM: HTTP {status}");
        }
        Ok(v)
    }

    pub async fn info(&self) -> Result<NodeInfo> {
        let req = self
            .http
            .get(self.url("/info"))
            .timeout(Duration::from_secs(10));
        Ok(serde_json::from_value(self.json(req).await?)?)
    }

    /// Aufgabe anlegen; Fotos folgen mit [`upload`](Self::upload), dann [`commit`](Self::commit)
    pub async fn init(&self, name: &str, options: &Value) -> Result<String> {
        let form = Form::new()
            .text("name", name.to_string())
            .text("options", options.to_string());
        let v = self
            .json(self.http.post(self.url("/task/new/init")).multipart(form))
            .await?;
        v["uuid"]
            .as_str()
            .map(String::from)
            .context("NodeODM: keine uuid")
    }

    /// Fotos gestreamt hochladen (ein Request für mehrere Dateien)
    pub async fn upload(&self, uuid: &str, files: &[PathBuf]) -> Result<()> {
        let mut form = Form::new();
        for path in files {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .context("Dateiname")?
                .to_string();
            let file = tokio::fs::File::open(path).await?;
            let len = file.metadata().await?.len();
            let body = reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(file));
            form = form.part(
                "images",
                Part::stream_with_length(body, len).file_name(name),
            );
        }
        let path = format!("/task/new/upload/{uuid}");
        self.json(self.http.post(self.url(&path)).multipart(form))
            .await?;
        Ok(())
    }

    pub async fn commit(&self, uuid: &str) -> Result<()> {
        let path = format!("/task/new/commit/{uuid}");
        self.json(self.http.post(self.url(&path))).await?;
        Ok(())
    }

    pub async fn task_info(&self, uuid: &str) -> Result<TaskInfo> {
        let path = format!("/task/{uuid}/info");
        parse_task_info(&self.json(self.http.get(self.url(&path))).await?)
    }

    /// Letzte Zeilen der Konsolenausgabe, für die Fehlermeldung
    pub async fn output_tail(&self, uuid: &str, lines: usize) -> Result<Vec<String>> {
        let path = format!("/task/{uuid}/output");
        let v = self.json(self.http.get(self.url(&path))).await?;
        let all: Vec<String> = v
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|l| l.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(all[all.len().saturating_sub(lines)..].to_vec())
    }

    /// Ergebnisdatei (z. B. `georeferenced_model.laz`) gestreamt nach `dest`
    pub async fn download(&self, uuid: &str, asset: &str, dest: &Path) -> Result<u64> {
        let path = format!("/task/{uuid}/download/{asset}");
        let res = self
            .http
            .get(self.url(&path))
            .send()
            .await
            .context("NodeODM nicht erreichbar")?;
        if !res.status().is_success() {
            bail!("NodeODM: {asset} nicht verfügbar (HTTP {})", res.status());
        }
        let tmp = dest.with_extension("part");
        let mut file = tokio::fs::File::create(&tmp).await?;
        let mut stream = res.bytes_stream();
        let mut size = 0u64;
        while let Some(chunk) = stream.try_next().await? {
            size += chunk.len() as u64;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        tokio::fs::rename(&tmp, dest).await?;
        Ok(size)
    }

    /// Aufgabe samt Dateien in NodeODM löschen (Fehler egal: aufräumen ist Kür)
    pub async fn remove(&self, uuid: &str) {
        let form = Form::new().text("uuid", uuid.to_string());
        let _ = self
            .http
            .post(self.url("/task/remove"))
            .multipart(form)
            .send()
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn task_info_from_nodeodm_json() {
        let i = parse_task_info(&json!({
            "uuid": "a", "status": {"code": 20}, "progress": 42.5, "processingTime": 1200
        }))
        .unwrap();
        assert_eq!(i.status, TaskStatus::Running);
        assert_eq!(i.progress, 42.5);
        assert_eq!(i.processing_ms, 1200);
        let f = parse_task_info(&json!({
            "status": {"code": 30, "errorMessage": "Not enough images"}, "progress": 100
        }))
        .unwrap();
        assert_eq!(f.status, TaskStatus::Failed);
        assert_eq!(f.error.as_deref(), Some("Not enough images"));
        assert!(parse_task_info(&json!({"status": {"code": 99}})).is_err());
        assert!(parse_task_info(&json!({})).is_err());
    }

    #[test]
    fn errors_in_body_become_errors() {
        assert!(checked(json!({"error": "Invalid token"})).is_err());
        assert!(checked(json!({"uuid": "x"})).is_ok());
    }

    #[test]
    fn token_goes_into_the_query() {
        let n = NodeOdm::new("http://odm:3000/", Some("geheim".into())).unwrap();
        assert_eq!(n.url("/info"), "http://odm:3000/info?token=geheim");
        let n = NodeOdm::new("http://odm:3000", None).unwrap();
        assert_eq!(n.url("/task/x/info"), "http://odm:3000/task/x/info");
    }
}
