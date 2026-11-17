---
description: >-
  Alle Einstellungen von Fixpunkt über Umgebungsvariablen: Datenverzeichnis, Port, erster Admin, gleichzeitige
  Berechnungen, Upload-Grenze, Cookies hinter HTTPS. Reverse-Proxy mit Caddy.
---

# Konfiguration

Fixpunkt liest seine Einstellungen aus Umgebungsvariablen – im Container über `-e` bzw. `environment:`, ohne
Container in `/etc/fixpunkt/fixpunkt.env`.

| Variable | Standard | Bedeutung |
|---|---|---|
| `FIXPUNKT_DATA_DIR` | `/data` | Datenbank (`fixpunkt.db`), Aufnahmen (`captures/`), Ergebnisse (`jobs/`) |
| `FIXPUNKT_LISTEN` | `0.0.0.0:8080` | Adresse und Port |
| `FIXPUNKT_USER`, `FIXPUNKT_PASSWORD` | – | Erster Admin, falls es noch keinen Benutzer gibt |
| `FIXPUNKT_MAX_JOBS` | `1` | Berechnungen gleichzeitig; jede braucht für große Flüge einige GB RAM |
| `FIXPUNKT_MAX_UPLOAD_GB` | `20` | Größte Datei beim Hochladen |
| `FIXPUNKT_SECURE_COOKIES` | `false` | Sitzungs-Cookie nur über HTTPS – hinter einem Proxy mit TLS einschalten |
| `RUST_LOG` | `info` | Ausführlichkeit der Logs, z. B. `debug` |

## Reverse-Proxy

Mit [Caddy](https://caddyserver.com/) holt sich der Proxy das Zertifikat selbst:

```caddy
fixpunkt.example.de {
    reverse_proxy 127.0.0.1:8080 {
        # Live-Updates (Server-Sent Events) nicht puffern
        flush_interval -1
    }
    request_body {
        max_size 20GB
    }
}
```

Bei nginx: `proxy_buffering off;` für `/api/events` und `client_max_body_size` für große Punktwolken.

## Koordinatensystem

Neue Baustellen rechnen in ETRS89/UTM32 (EPSG:25832), dem System der meisten Bundesländer. Aufnahmen und Grenzen
müssen im selben metrischen System vorliegen; Grenzen in Länge/Breite rechnet Fixpunkt selbst um. Mehr dazu unter
[Daten vorbereiten](daten.md).
