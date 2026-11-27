# Hinweise für Coding-Agenten

Fixpunkt: Aufmaß und Baustellendokumentation aus Drohnenflügen, selbst gehostet; erster Anwendungsfall ist der
Aushub von Baugruben (DJI Matrice 4E, NodeODM). Rechenkern in `crates/processing`, Rust-Server (axum, SQLite) in
`crates/server`, der die React-UI aus `ui/` eingebettet ausliefert, End-to-End-Tests in `e2e/`.

Die Regeln stehen einmal, in [`docs/mitwirken.md`](docs/mitwirken.md) (Beitragen) und
[`docs/entwicklung.md`](docs/entwicklung.md) (Bauen, Tests, Konventionen), auf Deutsch, für Menschen und Agenten.
Diese Datei sagt, wo was steht und was nie passieren darf. Sie gilt für jeden Agenten; `CLAUDE.md` verweist nur
hierher. Vor dem Start die Abschnitte lesen, die die Änderung berühren.

## Doku

Die Doku liegt in `docs/` (<https://speicherstadt.github.io/fixpunkt/>), gebaut mit Zensical (`mkdocs.yml`,
`requirements-docs.txt`, `.github/workflows/docs.yml`). Vor einer Änderung die betroffenen Seiten lesen:
`architektur.md`, `methode.md`, `aushub.md`, `funktionen.md`, `roadmap.md`; für den Rechenkern
`crates/processing/README.md`.

## Skills

Die Schritte für wiederkehrende Aufgaben stehen in [`.agents/skills`](.agents/skills); jeder Agent folgt denselben
Dateien. Aus der [Werkbank](https://github.com/Speicherstadt/werkbank) (gleich in allen Projekten, Änderungen dort):
`beitrag` (Branch, Commits, Pull Request als Entwurf, dann Selbstprüfung und Review), `beitrag-pruefen` (gewollt,
Quellen, einfachste Lösung, Regeln), `tdd-review` (unabhängiges Review, ein Befund braucht einen fehlschlagenden
Test), `doku-pruefen` (Review des Markdowns), `fork-pushen` (Push auf den Fork einer anderen Person),
`prs-abarbeiten` (alle offenen Pull Requests, vom einfachsten Merge an), `issue-schreiben`, `board-pflegen` (das
Board auf GitHub), `release`, `fehler-eingrenzen` (Ursache eines Fehlers über eine rote Prüfung finden) und
`vorhaben-klaeren` (einen Plan vor dem Bauen mit der Person durchfragen).
Aus diesem Projekt: `checks` (alle Prüfungen: Rust, UI, E2E, Doku).

## Referenzimplementierungen

Technik und Abläufe wie in den anderen Projekten, nicht aus dem Gedächtnis:

- `firsttris/workflows`: `tech-stack.md` (Technologiewahl), wiederverwendbare Workflows, Vorlagen.
- `firsttris/haul`: Rust-Server mit eingebetteter UI (`crates/haul/src/ui.rs`, `build.rs`, `auth.rs`, `events.rs`,
  `db.rs`), `docker/Dockerfile`, `release.yml` (statische musl-Programmdateien), `scripts/install.sh`.
- `firsttris/snapraid-ui`: UI mit shadcn/ui, Tailwind 4, Paraglide JS (`apps/web`).
- `firsttris/haben`: Doku mit Zensical (`mkdocs.yml`, `requirements-docs.txt`, `.github/workflows/docs.yml`).

## Nie

- Auf `main` pushen (Ausnahme: Reparatur eines roten `main` durch oder auf Anweisung des Maintainers), Commits von
  Beitragenden umschreiben, mergen (nur Maintainer oder Collaborator mit dessen Zustimmung), eigenen Code reviewen.
- Einen Test überspringen, abschwächen oder löschen, um grün zu werden; einen übersprungenen Test als bestanden melden.
- Issues und Pull Requests ohne Auftrag anlegen oder kommentieren.
- Eine Abhängigkeit hinzufügen, ohne zuerst eigenen Code zu versuchen und den Grund je Paket unter
  „## Neue Abhängigkeiten“ im Pull Request zu nennen.
- Prozesse der Nutzerin oder des Nutzers stoppen; `sudo` umgehen.
- Vergleiche mit oder Bezüge auf Produkte anderer Anbieter, in Repo und Doku (vergleichende Werbung, UWG).

## Bei jeder Änderung

- Jede berührte Datei sauberer hinterlassen, ohne das Verhalten zu ändern, und unter „Aufgeräumt“ nennen.
- Tests für das, was sich ändert, in der passenden Schicht (`docs/entwicklung.md`, Testschichten); ein Bugfix
  beginnt mit einem fehlschlagenden Test.
- Doku in `docs/` im selben Pull Request, kurz und für Menschen.
- Issues mit `Umgesetzt #…` oder `Refs #…`, nie `Closes`/`Fixes`.
- Prüfungen grün vor jedem Push (Skill `checks`), übersprungene Tests nennen; danach die CI prüfen.
- Die Beschreibung des Pull Requests wird die Squash-Nachricht: bei jedem Push nachziehen, der etwas daran ändert.
- Vor dem Merge ein unabhängiges Review durch einen eigenen Agenten: `tdd-review` für Code, `doku-pruefen` für Markdown.

## Arbeitsweise

- Deutsch für Kommentare, Doku, Commits, Pull Requests und alles, was Nutzer sehen; Bezeichner englisch.
- Funktional, keine Klassen; Grenzen für Dateien und Funktionen und die Abweichungen vom Tech-Stack stehen in
  `docs/entwicklung.md`, Konventionen.
- Fremde, nicht committete Änderungen gehören jemand anderem (oft einem anderen Agenten): beim eigenen Umfang bleiben.
- Quellen nennen, wo sie ein Detail entscheiden; GitHub-Quellen flach (`git clone --depth 1`) in ein temporäres
  Verzeichnis klonen und die Quelldatei im Commit nennen.
- Koordinaten in ETRS89/UTM32 (EPSG:25832), Höhen in DHHN2016 (Quasigeoid GCG2016). Die Testdaten liegen bei
  x ≈ 500 000–512 000, y ≈ 5 400 000: relativ zu einem Bezugspunkt rechnen, sonst frisst die Auslöschung in f32/f64
  die Millimeter.
