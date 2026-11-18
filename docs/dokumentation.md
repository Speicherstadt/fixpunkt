---
description: >-
  Baustelle dokumentieren in Fixpunkt: Zwangspunkte mit Sollhöhe gegen das aktuelle Gelände prüfen, Leitungen
  nachzeichnen, Fotos mit GPS-Position an ihrer Stelle zeigen, Notizen mit Koordinaten – als PDF-Bericht und DXF.
---

# Dokumentation

Im Reiter **Dokumentation** einer Baustelle stehen die [Pläne](plaene.md) und darunter, was neben den Massen
festgehalten werden muss: Zwangspunkte, Leitungen, Fotos und Notizen. Alles mit Lage erscheint auch in der [Ansicht](ansicht.md) –
Zwangspunkte als Raute, Fotos und Notizen als Punkt, Leitungen als Linie.

![Dokumentation einer Baugrube: Notiz zur Bodenprobe, Zwangspunkt am Kanal mit Soll- und Isthöhe, Foto mit GPS-Position](screenshot-dokumentation.png){ width="1024" height="529" loading="lazy" }

## Zwangspunkte

Ein Zwangspunkt ist eine Höhe, die eingehalten werden muss – die Sohle eines Bestandskanals, ein Anschluss, eine
Gründungshöhe. Fixpunkt vergleicht die **Sollhöhe** mit dem Gelände der neuesten Aufnahme an dieser Stelle und
zeigt **Ist** und den Unterschied: grün bis 3 cm, sonst in Warnfarbe.

**Zwangspunkt oder Notiz anlegen** → Bezeichnung, Ost, Nord und Sollhöhe (DHHN2016). Die Koordinaten einer
Stelle liefert die [Ansicht](ansicht.md) mit **Punkt**.

## Leitungen

Den Verlauf einer Leitung zeichnet man in der [Ansicht](ansicht.md#leitungen-zeichnen) nach: **Leitung zeichnen**,
die Punkte entlang der Leitung anklicken, Name und Notiz (Material, Durchmesser) eingeben, **Leitung speichern**.
Jeder Punkt bekommt die Höhe des Geländes der gezeigten Aufnahme darunter – im offenen Graben also die Höhe, auf
der die Leitung liegt. In der Liste stehen Länge und Zahl der Punkte.

## Fotos

**Fotos hochladen** nimmt eines oder mehrere Fotos (JPG, PNG). Hat ein Foto eine GPS-Position – Handys und
Drohnen speichern sie im Foto (EXIF) –, rechnet Fixpunkt sie in das Bezugssystem der Baustelle um und zeigt das
Foto an seiner Stelle; die Aufnahmezeit kommt ebenfalls aus dem Foto. Ein Klick auf das Vorschaubild öffnet das
Original.

Speichert das Gerät auch die **Blickrichtung** (EXIF `GPSImgDirection`, bei Drohnen und den meisten Handys mit
Kompass), steht sie in der Liste („Blickrichtung 135° (SO)“), im PDF-Bericht und als Sichtkegel am Foto in der
Ansicht. Fixpunkt dreht sie von geografisch Nord auf Gitter-Nord der Baustelle (Meridiankonvergenz, in
Deutschland bis etwa 2°); eine Angabe zu magnetisch Nord wird wie geografisch behandelt (3–4° Unterschied).

> [!NOTE]
> Handy-GPS ist auf einige Meter genau – gut genug, um ein Foto wiederzufinden, nicht zum Messen. Ohne GPS-Position
> steht das Foto nur in der Liste.

## Notizen

Notizen halten fest, was sonst im Bautagebuch verschwindet: Bodenproben, Abstimmungen, Fundstellen. Ost und Nord
sind freiwillig; mit Lage stehen sie in der Ansicht.

## Bericht (PDF)

**Bericht (PDF)** oben im Abschnitt fasst die Dokumentation einer Baustelle zusammen: alle Zwangspunkte mit Lage,
Sollhöhe, Gelände der letzten Aufnahme, Abweichung und Status (eingehalten bis ± 3 cm), die Notizen als Tabelle
und jedes Foto mit Aufnahmezeit, Lage und Notiz. Lesern steht der Bericht genauso zur Verfügung.

[![Seite 1 der Dokumentation als PDF: Zwangspunkt, Notiz und Foto](screenshot-pdf-dokumentation.png){ width="910" height="1287" loading="lazy" }](beispiel-dokumentation.pdf)

[Beispiel herunterladen (PDF)](beispiel-dokumentation.pdf)

## DXF für CAD

**DXF für CAD** lädt die ganze Dokumentation als Zeichnung (DXF R12) herunter, zum Weiterzeichnen im CAD
oder in QGIS – etwa als Grundlage für den Bestandsplan. Jede Art liegt auf einer eigenen Ebene:

| Ebene | Inhalt |
|---|---|
| `FP_LEITUNGEN` | Leitungen als 3D-Polylinie, Name am Anfang |
| `FP_ZWANGSPUNKTE` | Punkt mit Sollhöhe als z, Beschriftung „Name Sollhöhe“ |
| `FP_NOTIZEN` | Punkt und Titel |
| `FP_FOTOS` | Punkt und Titel |

Koordinaten im Bezugssystem der Baustelle (ETRS89/UTM32, Höhen DHHN2016), ohne Zonenkennung im Ostwert.
Punkte einer Leitung ohne Gelände darunter übernehmen die Höhe des Punkts davor.
