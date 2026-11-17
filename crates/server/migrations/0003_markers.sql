-- Dokumentation einer Baustelle: Zwangspunkte (Sollhöhe), Fotos mit Position, Notizen.
-- Ein Foto je Markierung; Datei und Vorschaubild liegen unter markers/<id>/.
CREATE TABLE markers (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    site_id    INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    -- constraint | photo | note
    kind       TEXT    NOT NULL,
    title      TEXT    NOT NULL,
    note       TEXT    NOT NULL DEFAULT '',
    -- Lage im Bezugssystem der Baustelle; leer, solange unbekannt
    x          REAL,
    y          REAL,
    -- Zwangspunkt: Sollhöhe; sonst gemessene Höhe
    z          REAL,
    photo      TEXT,
    taken_at   INTEGER,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX markers_site ON markers(site_id);
