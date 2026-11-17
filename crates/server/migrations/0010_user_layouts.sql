-- Eigener Überblick je Benutzer und Baustelle: Reihenfolge und Sichtbarkeit der Abschnitte als JSON
CREATE TABLE user_layouts (
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    site_id    INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    layout     TEXT    NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, site_id)
);
