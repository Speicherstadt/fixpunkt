-- Massenziel je Bereich: Material (frei, z. B. „Oberboden“), Soll-Menge in m³ und ob Abtrag oder Auftrag zählt
ALTER TABLE regions ADD COLUMN material TEXT;
ALTER TABLE regions ADD COLUMN target_m3 REAL;
ALTER TABLE regions ADD COLUMN target_kind TEXT;
