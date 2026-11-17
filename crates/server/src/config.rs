//! Laufzeit-Konfiguration aus Umgebungsvariablen.

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub listen: SocketAddr,
    /// Datenbank, Aufnahmen, Ergebnisse
    pub data_dir: PathBuf,
    /// Erster Admin, falls noch keiner existiert (sonst Einrichtung in der UI)
    pub initial_user: Option<(String, String)>,
    /// gleichzeitig laufende Rechenaufträge
    pub max_jobs: usize,
    /// Größte Upload-Datei in Bytes
    pub max_upload: u64,
    /// `bun run dev`: Adresse des Vite-Servers, wenn die UI nicht eingebettet ist
    pub dev_ui: Option<String>,
    /// Sitzungs-Cookie nur über HTTPS (hinter einem Reverse-Proxy mit TLS: an)
    pub secure_cookies: bool,
    /// NodeODM für Drohnenfotos → Punktwolke, z. B. `http://nodeodm:3000`
    pub nodeodm_url: Option<String>,
    pub nodeodm_token: Option<String>,
    /// Abstand der Statusabfragen bei NodeODM
    pub nodeodm_poll: std::time::Duration,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::from_vars(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
    }

    /// Testbar: Variablen über eine Funktion statt aus der Prozessumgebung.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let listen = var("FIXPUNKT_LISTEN")
            .unwrap_or_else(|| "0.0.0.0:8080".into())
            .parse()
            .context("FIXPUNKT_LISTEN")?;
        let data_dir = PathBuf::from(var("FIXPUNKT_DATA_DIR").unwrap_or_else(|| "/data".into()));
        let initial_user = var("FIXPUNKT_USER").zip(var("FIXPUNKT_PASSWORD"));
        let max_jobs = match var("FIXPUNKT_MAX_JOBS") {
            Some(v) => v.parse().context("FIXPUNKT_MAX_JOBS")?,
            None => 1,
        };
        let max_upload = match var("FIXPUNKT_MAX_UPLOAD_GB") {
            Some(v) => (v.parse::<f64>().context("FIXPUNKT_MAX_UPLOAD_GB")? * 1e9) as u64,
            None => 20_000_000_000,
        };
        let secure_cookies = matches!(
            var("FIXPUNKT_SECURE_COOKIES").as_deref(),
            Some("1" | "true")
        );
        let nodeodm_poll = match var("FIXPUNKT_NODEODM_POLL_SECS") {
            Some(v) => v.parse::<f64>().context("FIXPUNKT_NODEODM_POLL_SECS")?,
            None => 10.0,
        };
        Ok(Config {
            listen,
            data_dir,
            initial_user,
            max_jobs: max_jobs.max(1),
            max_upload,
            dev_ui: var("FIXPUNKT_DEV_UI"),
            secure_cookies,
            nodeodm_url: var("FIXPUNKT_NODEODM_URL").map(|u| u.trim_end_matches('/').to_string()),
            nodeodm_token: var("FIXPUNKT_NODEODM_TOKEN"),
            nodeodm_poll: std::time::Duration::from_secs_f64(nodeodm_poll.max(0.01)),
        })
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("fixpunkt.db")
    }

    pub fn capture_dir(&self, id: i64) -> PathBuf {
        self.data_dir.join("captures").join(id.to_string())
    }

    pub fn job_dir(&self, id: i64) -> PathBuf {
        self.data_dir.join("jobs").join(id.to_string())
    }

    /// Konfiguration für Tests: Datenverzeichnis, sonst Standardwerte.
    pub fn for_dir(data_dir: impl Into<PathBuf>) -> Self {
        Config {
            listen: "127.0.0.1:0".parse().expect("gültige Adresse"),
            data_dir: data_dir.into(),
            initial_user: None,
            max_jobs: 1,
            max_upload: 20_000_000_000,
            dev_ui: None,
            secure_cookies: false,
            nodeodm_url: None,
            nodeodm_token: None,
            nodeodm_poll: std::time::Duration::from_millis(20),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        let c = Config::from_vars(|_| None).unwrap();
        assert_eq!(c.listen.port(), 8080);
        assert_eq!(c.data_dir, PathBuf::from("/data"));
        assert!(c.initial_user.is_none());
        assert!(c.nodeodm_url.is_none());
        assert_eq!(c.nodeodm_poll.as_secs(), 10);
        let c = Config::from_vars(|k| match k {
            "FIXPUNKT_LISTEN" => Some("127.0.0.1:9000".into()),
            "FIXPUNKT_USER" => Some("admin".into()),
            "FIXPUNKT_PASSWORD" => Some("geheim123".into()),
            "FIXPUNKT_MAX_JOBS" => Some("0".into()),
            "FIXPUNKT_MAX_UPLOAD_GB" => Some("1.5".into()),
            "FIXPUNKT_NODEODM_URL" => Some("http://nodeodm:3000/".into()),
            "FIXPUNKT_NODEODM_POLL_SECS" => Some("2.5".into()),
            _ => None,
        })
        .unwrap();
        assert_eq!(c.listen.port(), 9000);
        assert_eq!(c.initial_user, Some(("admin".into(), "geheim123".into())));
        assert_eq!(c.max_jobs, 1);
        assert_eq!(c.max_upload, 1_500_000_000);
        assert_eq!(c.nodeodm_url.as_deref(), Some("http://nodeodm:3000"));
        assert_eq!(c.nodeodm_poll.as_millis(), 2500);
        assert!(Config::from_vars(|k| (k == "FIXPUNKT_LISTEN").then(|| "x".into())).is_err());
    }
}
