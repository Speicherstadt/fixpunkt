-- Abrechnung: Leistungsposition, Einheitspreis und Preiseinheit je Bereich
ALTER TABLE regions ADD COLUMN lv_position TEXT;
ALTER TABLE regions ADD COLUMN unit_price REAL;
-- `m3` (fest), `m3_loose` (lose) oder `t`
ALTER TABLE regions ADD COLUMN price_unit TEXT;
-- Nutzlast eines LKW in t, für die Zahl der Ladungen
ALTER TABLE sites ADD COLUMN truck_t REAL;

-- Kennwerte je Material einer Baustelle: Dichte in t/m³ (fest) und Auflockerungsfaktor (lose/fest)
CREATE TABLE site_materials (
    site_id INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    name    TEXT    NOT NULL,
    density REAL,
    bulking REAL,
    PRIMARY KEY (site_id, name)
);

-- Lieferscheine und Wiegescheine: Menge je Material und Tag
CREATE TABLE deliveries (
    id           INTEGER PRIMARY KEY,
    site_id      INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    delivered_at INTEGER NOT NULL,
    material     TEXT,
    quantity     REAL    NOT NULL,
    -- `t`, `m3` (lose, wie geladen) oder `loads` (LKW-Ladungen)
    unit         TEXT    NOT NULL,
    reference    TEXT,
    created_at   INTEGER NOT NULL
);
CREATE INDEX deliveries_site ON deliveries(site_id, delivered_at);
