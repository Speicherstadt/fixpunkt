-- Fortschritt 0–100 für lange Aufträge (Photogrammetrie in NodeODM); sonst leer.
-- Neuer Status: draft – Auftrag angelegt, Fotos werden noch hochgeladen; der Worker übergeht ihn.
ALTER TABLE jobs ADD COLUMN progress REAL;
