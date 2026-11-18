---
description: >-
  Alle Einstellungen von Fixpunkt über Umgebungsvariablen: Datenverzeichnis, Port, erster Admin, gleichzeitige
  Berechnungen, Upload-Grenze, Cookies hinter HTTPS, öffentliche Adresse für Passkeys, NodeODM, Kartenkacheln.
  Reverse-Proxy mit Caddy.
---

# Konfiguration

Fixpunkt liest seine Einstellungen aus Umgebungsvariablen – im Container über `-e` bzw. `environment:`, ohne
Container in `/etc/fixpunkt/fixpunkt.env`.

| Variable | Standard | Bedeutung |
|---|---|---|
| `FIXPUNKT_DATA_DIR` | `/data` | Datenbank (`fixpunkt.db`), Aufnahmen (`captures/`), Ergebnisse (`jobs/`), Dokumentation, Pläne und angefangene Uploads – siehe [Backup](backup.md#was-gesichert-werden-muss) |
| `FIXPUNKT_LISTEN` | `0.0.0.0:8080` | Adresse und Port |
| `FIXPUNKT_USER`, `FIXPUNKT_PASSWORD` | – | Erster Admin, falls es noch keinen Benutzer gibt |
| `FIXPUNKT_MAX_JOBS` | `1` | Berechnungen gleichzeitig; jede braucht für große Flüge einige GB RAM |
| `FIXPUNKT_MAX_UPLOAD_GB` | `20` | Größte Datei beim Hochladen |
| `FIXPUNKT_PUBLIC_URL` | – | Öffentliche Adresse, z. B. `https://fixpunkt.example.de`; schaltet [Passkeys](benutzer.md#passkeys) ein |
| `FIXPUNKT_SECURE_COOKIES` | `false` | Sitzungs-Cookie nur über HTTPS – hinter einem Proxy mit TLS einschalten |
| `FIXPUNKT_NODEODM_URL` | – | NodeODM für [Drohnenfotos](drohnenfotos.md), z. B. `http://nodeodm:3000`; ohne ist die Funktion aus |
| `FIXPUNKT_NODEODM_TOKEN` | – | Token, falls NodeODM mit `--token` läuft |
| `FIXPUNKT_NODEODM_POLL_SECS` | `10` | Abstand der Fortschrittsabfragen bei NodeODM |
| `FIXPUNKT_MAP_TILES` | OpenStreetMap | Kacheln der Übersichtskarte als URL mit `{z}/{x}/{y}`; `none` zeigt die Karte ohne Hintergrund (kein Abruf nach außen) |
| `FIXPUNKT_MAP_ATTRIBUTION` | `© OpenStreetMap-Mitwirkende` | Namensnennung zu den Kacheln |
| `RUST_LOG` | `info` | Ausführlichkeit der Logs, z. B. `debug` |

> [!NOTE]
> Die Kartenkacheln lädt der Browser direkt vom Kachelserver. Bei OpenStreetMap sieht dieser also, welcher
> Kartenausschnitt angezeigt wird – und damit ungefähr, wo die Baustellen liegen. Wer das nicht möchte, nimmt einen
> eigenen Kachelserver oder `FIXPUNKT_MAP_TILES=none`.

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

Bei nginx: `proxy_buffering off;` für `/api/events` und `client_max_body_size` von mindestens `16m`. Große
Dateien kommen in Stücken à 8 MB (fortsetzbarer Upload nach [tus](https://tus.io)), daher genügt das auch für
Punktwolken mit vielen Gigabyte; Pläne und Fotos der Dokumentation kommen in einem Stück.

## Koordinatensystem

Neue Baustellen rechnen in ETRS89/UTM32 (EPSG:25832), dem System der meisten Bundesländer. Aufnahmen und Grenzen
müssen im selben metrischen System vorliegen; Grenzen in Länge/Breite rechnet Fixpunkt selbst um. Mehr dazu unter
[Daten vorbereiten](daten.md).
