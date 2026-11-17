---
description: >-
  Fixpunkt sichern und aktualisieren: was im Datenverzeichnis liegt, Backup mit restic oder tar, Wiederherstellen,
  Updates von Container und Programmdatei.
---

# Backup und Updates

## Was gesichert werden muss

Alles liegt im Datenverzeichnis (`/data` im Container, `/var/lib/fixpunkt` ohne Container):

| Pfad | Inhalt |
|---|---|
| `fixpunkt.db`, `fixpunkt.db-wal`, `fixpunkt.db-shm` | Baustellen, Benutzer, Bereiche, Berechnungen (SQLite) |
| `captures/<id>/` | hochgeladene Punktwolken und Höhenmodelle |
| `jobs/<id>/` | Ergebnisse der Berechnungen (z. B. `diff.tif`), bei Drohnenfotos auch die Fotos |
| `markers/<id>/` | Fotos der Dokumentation mit Vorschaubild |
| `plans/<id>/` | DXF-Pläne (Original und aufbereitete Geometrie) |
| `uploads/` | unfertige Uploads; muss nicht ins Backup |

## Backup

Am sichersten bei angehaltenem Dienst, damit Datenbank und Dateien zusammenpassen:

```bash
docker stop fixpunkt
docker run --rm -v fixpunkt:/data -v "$PWD":/backup debian:bookworm-slim \
  tar czf /backup/fixpunkt-$(date +%F).tar.gz -C /data .
docker start fixpunkt
```

Für regelmäßige Sicherungen auf einen zweiten Speicher (z. B. netcup Storage Space) eignet sich
[restic](https://restic.net/) – große, selten geänderte Punktwolken werden dabei nur einmal übertragen.

> [!TIP]
> Die Rohdaten der Drohne (Fotos) gehören zusätzlich ins Archiv: aus ihnen lässt sich jede Punktwolke neu rechnen.

## Wiederherstellen

Dienst anhalten, Datenverzeichnis leeren, Sicherung hineinentpacken, Dienst starten.

## Updates

- **Docker/Podman:** `docker pull tristanteu/fixpunkt && docker rm -f fixpunkt` und neu starten (oder Compose
  `pull` + `up -d`, Podman `AutoUpdate=registry`).
- **Ohne Container:** `install.sh` erneut ausführen.

Beim Start bringt Fixpunkt die Datenbank selbst auf den neuen Stand; Berechnungen, die beim Beenden liefen, werden
wieder eingereiht. Vor einem Update ein Backup machen – Migrationen lassen sich nicht zurückspielen.
