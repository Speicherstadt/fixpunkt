---
description: >-
  2D-Ansicht in Fixpunkt: Höhenbild jeder Aufnahme mit Schummerung, Differenz oder Soll/Ist-Abweichung
  darübergelegt, messen (Punkt, Strecke, Fläche, Profil) und Abrechnungsbereiche direkt im Browser zeichnen.
---

# Ansicht und Messen

Unter **Ansicht** auf der Seite einer Baustelle zeigt Fixpunkt das Höhenbild einer Aufnahme: Farben von tief
(blau) nach hoch (braun, hell), mit Schummerung aus Nordwesten, damit Böschungen und Kanten plastisch wirken.

![Ansicht der Baugrube: Höhenbild, Soll/Ist-Abweichung der Sohle darübergelegt, Bereiche und ein Profil quer durch die Grube](screenshot-ansicht.png){ width="1024" height="1134" }

## Bedienen

| | Maus | Tastatur |
|---|---|---|
| verschieben | ziehen | Pfeiltasten |
| zoomen | Mausrad | `+` und `−` |
| alles zeigen | – | `0` |
| Punkt setzen | klicken | `Enter` setzt ihn am Fadenkreuz in der Mitte |

Unter dem Bild stehen Ost-, Nordwert und Höhe unter dem Mauszeiger, die Farbskala und der Maßstab.

## Darüber anzeigen

**Darüber anzeigen** legt das Ergebnis einer Berechnung über das Höhenbild: die Differenz eines Aushubs oder die
Abweichung eines Soll/Ist-Vergleichs. Die Farben bedeuten überall dasselbe wie im Mengenbericht:
**orange = Material weg** (abgetragen bzw. noch zu hoch), **blau = Material dazu** (aufgeschüttet bzw. zu tief).

## Messen

| Werkzeug | Ergebnis |
|---|---|
| **Punkt** | Ostwert, Nordwert, Höhe |
| **Strecke** | Länge horizontal und über das Gelände, Höhenunterschied, mittleres Gefälle in % – auch als Polylinie |
| **Fläche** | Fläche horizontal und Oberfläche, Umfang, tiefste und höchste Höhe |
| **Profil** | wie Strecke, dazu der Längsschnitt als Diagramm |

Punkte nacheinander setzen; **Letzten Punkt entfernen** und **Neu beginnen** korrigieren. Die Werte stehen als
Liste unter dem Bild und werden von Screenreadern vorgelesen.

> [!NOTE]
> Gemessen wird auf dem Höhenbild der Ansicht. Bei großen Flächen ist es vergröbert (höchstens eine Million
> Zellen, z. B. 1 m bei 1 km²); für Massen gilt immer die Rasterweite der Berechnung.

## Bereiche zeichnen

Mit **Bereich zeichnen** die Ecken einer Abrechnungsgrenze nacheinander anklicken (mindestens drei, die Fläche
schließt sich selbst), einen Namen eingeben und **Bereich speichern**. Der Bereich steht danach unter **Bereiche**
und in allen Berechnungen zur Auswahl – ganz ohne QGIS. Für genaue Grenzen aus Plänen bleibt der Import als
[GeoJSON](daten.md#abrechnungsgrenzen-aus-qgis).
