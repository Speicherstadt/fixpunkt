-- Benutzer und Sitzungen
CREATE TABLE users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT    NOT NULL,
    -- admin | editor | viewer
    role          TEXT    NOT NULL,
    created_at    INTEGER NOT NULL
);

CREATE TABLE sessions (
    token_hash TEXT    PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    csrf       TEXT    NOT NULL,
    expires_at INTEGER NOT NULL
);

-- Baustellen und was dazugehört
CREATE TABLE sites (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL DEFAULT '',
    epsg        INTEGER NOT NULL DEFAULT 25832,
    created_at  INTEGER NOT NULL
);

-- Eine Aufnahme: Punktwolke (LAS/LAZ/XYZ) oder Höhenraster (GeoTIFF/ASC)
CREATE TABLE captures (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    site_id     INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL,
    file_name   TEXT    NOT NULL,
    file_size   INTEGER NOT NULL,
    -- Aufnahmedatum (ms), vom Nutzer gesetzt; sonst Upload-Zeit
    captured_at INTEGER NOT NULL,
    -- uploaded | ready | failed
    status      TEXT    NOT NULL,
    -- JSON aus `fixpunkt info`
    info        TEXT,
    error       TEXT,
    created_at  INTEGER NOT NULL
);
CREATE INDEX captures_site ON captures(site_id);

-- Abrechnungsbereich (Polygon, GeoJSON im System der Baustelle)
CREATE TABLE regions (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    site_id    INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    geojson    TEXT    NOT NULL,
    area_m2    REAL    NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX regions_site ON regions(site_id);

-- Rechenaufträge
CREATE TABLE jobs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    site_id     INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    -- inspect | volume | stockpile | dem
    kind        TEXT    NOT NULL,
    params      TEXT    NOT NULL,
    -- queued | running | done | failed
    status      TEXT    NOT NULL,
    result      TEXT,
    error       TEXT,
    created_by  INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at  INTEGER NOT NULL,
    started_at  INTEGER,
    finished_at INTEGER
);
CREATE INDEX jobs_site ON jobs(site_id);
CREATE INDEX jobs_status ON jobs(status);
