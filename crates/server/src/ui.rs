//! Liefert die gebaute Web-UI aus der Programmdatei aus. Unbekannte Pfade bekommen `index.html`,
//! damit die Routen der Single-Page-App funktionieren. Vorbild: firsttris/haul crates/haul/src/ui.rs

use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Redirect, Response};
use rust_embed::{EmbeddedFile, RustEmbed};

#[derive(RustEmbed)]
#[folder = "../../ui/dist"]
struct Assets;

/// Ohne eingebettete UI (z. B. `cargo run` während `bun run dev`): Weiterleitung zum Vite-Server.
pub fn handler(
    dev_ui: Option<String>,
) -> impl Fn(Uri, HeaderMap) -> std::future::Ready<Response> + Clone {
    move |uri, headers| std::future::ready(serve(&uri, &headers, dev_ui.as_deref()))
}

pub fn serve(uri: &Uri, headers: &HeaderMap, dev_ui: Option<&str>) -> Response {
    let path = uri.path().trim_start_matches('/');
    if let Some(file) = Assets::get(path).filter(|_| !path.is_empty()) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let cache = if path.starts_with("assets/") {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        };
        return embedded(headers, file, mime.as_ref(), cache);
    }
    if path.starts_with("api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    match (Assets::get("index.html"), dev_ui) {
        (Some(index), _) => embedded(headers, index, "text/html; charset=utf-8", "no-cache"),
        (None, Some(dev)) => Redirect::temporary(&format!("{}{}", dev.trim_end_matches('/'), uri)).into_response(),
        (None, None) => (
            StatusCode::NOT_FOUND,
            "UI nicht gebaut. `bun run build` ausführen oder `bun run dev` und http://localhost:5173 öffnen.",
        )
            .into_response(),
    }
}

/// Datei mit ETag aus dem SHA-256 beim Bauen; passendes `If-None-Match` → 304 ohne Inhalt.
fn embedded(headers: &HeaderMap, file: EmbeddedFile, mime: &str, cache: &'static str) -> Response {
    let etag = format!("W/\"{}\"", hex::encode(file.metadata.sha256_hash()));
    let fresh = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| etag_matches(v, &etag));
    let etag = HeaderValue::from_str(&etag).expect("Hex ist ein gültiger Header");
    let cache = HeaderValue::from_static(cache);
    if fresh {
        return (
            StatusCode::NOT_MODIFIED,
            [(header::ETAG, etag), (header::CACHE_CONTROL, cache)],
        )
            .into_response();
    }
    let mime = HeaderValue::from_str(mime).expect("MIME-Typ ist ein gültiger Header");
    (
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, cache),
            (header::ETAG, etag),
        ],
        file.data,
    )
        .into_response()
}

/// Schwacher Vergleich wie für `If-None-Match` (RFC 9110 13.1.2)
fn etag_matches(if_none_match: &str, etag: &str) -> bool {
    let opaque = |t: &str| t.trim().trim_start_matches("W/").to_string();
    let ours = opaque(etag);
    if_none_match
        .split(',')
        .any(|t| t.trim() == "*" || opaque(t) == ours)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn if_none_match() {
        let etag = "W/\"abc\"";
        assert!(etag_matches("W/\"abc\"", etag));
        assert!(etag_matches("\"abc\"", etag));
        assert!(etag_matches("\"x\", W/\"abc\"", etag));
        assert!(etag_matches("*", etag));
        assert!(!etag_matches("\"abcd\"", etag));
    }

    #[test]
    fn api_paths_are_not_the_spa() {
        let r = serve(&Uri::from_static("/api/nope"), &HeaderMap::new(), None);
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
    }
}
