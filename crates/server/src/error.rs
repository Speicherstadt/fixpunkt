//! Fehler der API. Der Server schickt nur einen Schlüssel (`{"error":"auth.wrongCredentials"}`),
//! die UI zeigt den Text in der Sprache des Nutzers (`ui/messages/*.json`, Schlüssel `server_…`).

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Alle Fehlerschlüssel – der Test `every_code_has_a_text` prüft, dass es für jeden einen Text gibt.
pub mod code {
    pub const NOT_LOGGED_IN: &str = "auth.notLoggedIn";
    pub const WRONG_CREDENTIALS: &str = "auth.wrongCredentials";
    pub const SETUP_DONE: &str = "auth.setupDone";
    pub const PASSWORD_TOO_SHORT: &str = "auth.passwordTooShort";
    pub const NAME_REQUIRED: &str = "auth.nameRequired";
    pub const NAME_TAKEN: &str = "auth.nameTaken";
    pub const WRONG_PASSWORD: &str = "auth.wrongPassword";
    pub const CSRF: &str = "auth.csrf";
    pub const FORBIDDEN: &str = "auth.forbidden";
    pub const LAST_ADMIN: &str = "auth.lastAdmin";
    pub const NOT_FOUND: &str = "api.notFound";
    pub const INTERNAL: &str = "api.internal";
    pub const INVALID_INPUT: &str = "api.invalidInput";
    pub const UNSUPPORTED_FILE: &str = "captures.unsupportedFile";
    pub const FILE_TOO_LARGE: &str = "captures.fileTooLarge";
    pub const CAPTURE_NOT_READY: &str = "jobs.captureNotReady";
    pub const INVALID_GEOJSON: &str = "regions.invalidGeojson";
    pub const WRONG_SITE: &str = "jobs.wrongSite";
    pub const UNKNOWN_JOB: &str = "jobs.unknownKind";
    pub const ODM_NOT_CONFIGURED: &str = "odm.notConfigured";
    pub const ODM_NOT_DRAFT: &str = "odm.notDraft";
    pub const ODM_TOO_FEW_IMAGES: &str = "odm.tooFewImages";
    pub const ODM_UNSUPPORTED_IMAGE: &str = "odm.unsupportedImage";
    pub const GEOREF_INVALID: &str = "georef.invalid";
    pub const MARKER_NEEDS_POSITION: &str = "markers.needsPosition";
    pub const MARKER_NOT_AN_IMAGE: &str = "markers.notAnImage";
    pub const PLAN_INVALID: &str = "plans.invalid";
    pub const ORTHO_INVALID: &str = "ortho.invalid";
    pub const SESSION_REQUIRED: &str = "auth.sessionRequired";
    pub const PASSKEYS_DISABLED: &str = "passkeys.disabled";
    pub const PASSKEY_FAILED: &str = "passkeys.failed";
    pub const NO_PASSKEY: &str = "passkeys.none";
    pub const SHARE_DECIDED: &str = "shares.decided";

    pub const ALL: &[&str] = &[
        NOT_LOGGED_IN,
        WRONG_CREDENTIALS,
        SETUP_DONE,
        PASSWORD_TOO_SHORT,
        NAME_REQUIRED,
        NAME_TAKEN,
        WRONG_PASSWORD,
        CSRF,
        FORBIDDEN,
        LAST_ADMIN,
        NOT_FOUND,
        INTERNAL,
        INVALID_INPUT,
        UNSUPPORTED_FILE,
        FILE_TOO_LARGE,
        CAPTURE_NOT_READY,
        INVALID_GEOJSON,
        WRONG_SITE,
        UNKNOWN_JOB,
        ODM_NOT_CONFIGURED,
        ODM_NOT_DRAFT,
        ODM_TOO_FEW_IMAGES,
        ODM_UNSUPPORTED_IMAGE,
        GEOREF_INVALID,
        MARKER_NEEDS_POSITION,
        MARKER_NOT_AN_IMAGE,
        PLAN_INVALID,
        ORTHO_INVALID,
        SESSION_REQUIRED,
        PASSKEYS_DISABLED,
        PASSKEY_FAILED,
        NO_PASSKEY,
        SHARE_DECIDED,
    ];
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    /// Zusatz für Logs und Eingabefehler (z. B. Meldung des Rechenkerns)
    pub detail: Option<String>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str) -> Self {
        ApiError {
            status,
            code,
            detail: None,
        }
    }
    pub fn bad_request(code: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code)
    }
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, code::NOT_FOUND)
    }
    pub fn with_detail(mut self, d: impl Into<String>) -> Self {
        self.detail = Some(d.into());
        self
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self {
        let e = e.into();
        tracing::error!("api: {e:#}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, code::INTERNAL)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = match self.detail {
            Some(d) => serde_json::json!({ "error": self.code, "detail": d }),
            None => serde_json::json!({ "error": self.code }),
        };
        (self.status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::code;

    /// Jeder Fehlerschlüssel hat in beiden Sprachen einen Text (`server_<bereich>_<name>`).
    #[test]
    fn every_code_has_a_text() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/messages");
        for lang in ["de", "en"] {
            let text = std::fs::read_to_string(root.join(format!("{lang}.json"))).unwrap();
            let v: serde_json::Value = serde_json::from_str(&text).unwrap();
            for c in code::ALL {
                let key = format!("server_{}", c.replace('.', "_"));
                assert!(
                    v.get(&key).and_then(|t| t.as_str()).is_some(),
                    "{lang}.json: {key} fehlt"
                );
            }
        }
    }
}
