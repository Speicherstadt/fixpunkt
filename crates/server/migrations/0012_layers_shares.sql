-- Schichten (Homogenbereiche) je Baustelle: Unterkante in m unter dem Urgelände, die letzte reicht unbegrenzt
-- nach unten (bottom_m leer); dazu Leistungsposition und Preis wie bei einem Bereich
CREATE TABLE site_layers (
    id          INTEGER PRIMARY KEY,
    site_id     INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    sort        INTEGER NOT NULL,
    name        TEXT    NOT NULL,
    bottom_m    REAL,
    lv_position TEXT,
    unit_price  REAL,
    price_unit  TEXT
);
CREATE INDEX site_layers_site ON site_layers(site_id, sort);

-- Freigabe eines Aufmaßes durch den Auftraggeber: Link mit Zufallstoken (gespeichert nur als SHA-256),
-- eingefrorenes Aufmaß, Entscheidung mit Name und Zeit
CREATE TABLE billing_shares (
    id           INTEGER PRIMARY KEY,
    site_id      INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    token_hash   TEXT    NOT NULL UNIQUE,
    from_capture INTEGER NOT NULL,
    to_capture   INTEGER NOT NULL,
    snapshot     TEXT    NOT NULL,
    note         TEXT,
    created_by   INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at   INTEGER NOT NULL,
    revoked_at   INTEGER,
    -- `approved` (anerkannt) oder `objected` (Einwand)
    decision     TEXT,
    decided_name TEXT,
    decided_org  TEXT,
    comment      TEXT,
    decided_at   INTEGER
);
CREATE INDEX billing_shares_site ON billing_shares(site_id);
