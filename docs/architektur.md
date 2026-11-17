---
description: >-
  Architektur von Fixpunkt: eine Rust-Programmdatei mit axum, SQLite, Job-Warteschlange und eingebetteter
  React-UI; Rechenkern als Bibliothek; Module und Datenfluss.
---

# Architektur

```mermaid
flowchart LR
  B[Browser<br>Web-UI] -- HTTP, SSE --> A[axum<br>/api]
  A --> D[(SQLite)]
  A --> F[Datenverzeichnis<br>Aufnahmen, Ergebnisse]
  A --> Q[Job-Warteschlange]
  Q --> R[Rechenkern<br>crates/processing]
  R --> F
```

**Eine Programmdatei:** der Rust-Server liefert die gebaute Web-UI selbst aus (`rust-embed`), speichert in SQLite
und einem Ordner und rechnet im selben Prozess. Aufbau wie
[haul](https://github.com/firsttris/haul); die Technik folgt
[tech-stack.md](https://github.com/firsttris/workflows/blob/main/tech-stack.md), Abweichungen stehen in `CLAUDE.md`.

| Bereich | Ort | Technik |
|---|---|---|
| Rechenkern | `crates/processing` | Rust: LAS/LAZ, GeoTIFF, Raster, Bodenfilter, Volumen, Helmert, UTM/Geoid, Exporte |
| Server | `crates/server` | tokio, axum, sqlx (SQLite), Server-Sent Events, argon2id |
| Web-UI | `ui` | Bun, Vite, React, TanStack Router + Query, Komponenten im shadcn/ui-Stil auf Radix, Tailwind 4, Paraglide (de/en) |
| E2E | `e2e` | Playwright gegen die echte Programmdatei |
| Auslieferung | `docker`, `scripts/install.sh` | Image in drei Stufen, statische musl-Programmdateien |

## Server

| Modul | Aufgabe |
|---|---|
| `api.rs` | Routen unter `/api`, Live-Ereignisse `/api/events` |
| `auth.rs` | Einrichten, Anmelden, Sitzungen, CSRF, Rollen, Benutzer |
| `sites.rs` | Baustellen und Bereiche (GeoJSON) |
| `captures.rs` | Hochladen (gestreamt), Herunterladen, Löschen |
| `jobs/` | Warteschlange in SQLite, Worker im Prozess, übersteht Neustarts; `spec.rs`: Auftragsarten mit typisierten Parametern |
| `compute/` | Brücke zum Rechenkern: prüfen, Aushub, Haufwerk, Zeitreihe (`volume.rs`), Soll/Ist (`compare.rs`), Exporte (`export.rs`) |
| `ui.rs` | eingebettete UI mit ETag, Weiterleitung auf Vite in der Entwicklung |
| `error.rs` | Fehler als Schlüssel; die UI zeigt den Text in der gewählten Sprache |

## Ablauf einer Berechnung

1. Upload streamt die Datei nach `captures/<id>/`; ein Prüfauftrag liest sie und speichert Punktzahl und Ausdehnung.
2. „Aushub berechnen“ prüft die Eingaben und reiht einen Auftrag ein.
3. Ein Worker übernimmt ihn (`UPDATE … RETURNING` in SQLite), rechnet im Thread-Pool und schreibt Ergebnis und
   `diff.tif` nach `jobs/<id>/`.
4. Jeder Statuswechsel geht als Ereignis an alle offenen Browser; die UI lädt nur die betroffenen Listen neu.

## Datenmodell

Baustelle → Aufnahmen, Bereiche, Aufträge; Benutzer mit Rolle und Sitzungen. Eine Instanz gehört einer Firma;
Mandanten kommen erst, wenn eine Instanz mehrere Firmen bedienen soll.
