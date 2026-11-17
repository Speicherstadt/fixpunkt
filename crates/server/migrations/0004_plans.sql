-- Pläne (DXF) je Baustelle; Original und aufbereitete Geometrie unter plans/<id>/
CREATE TABLE plans (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    site_id    INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    file_name  TEXT    NOT NULL,
    -- Ebenen, Anzahl Linien/Höhen/Texte, Zonenabzug, Ausdehnung (JSON)
    info       TEXT    NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX plans_site ON plans(site_id);
