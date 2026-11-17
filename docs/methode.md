---
description: >-
  Wie Fixpunkt Massen berechnet: Rastermethode, Bodenfilter (SMRF), Lücken, Haufwerke gegen Basisfläche,
  Soll/Ist-Vergleich, Genauigkeit und geprüfte Ergebnisse.
---

# Methode und Genauigkeit

## Rastermethode

1. Beide Aufnahmen werden auf **dasselbe Raster** gelegt (Zellgröße nach Wahl, Standard 25 cm). Die Höhe einer
   Zelle ist der Mittelwert ihrer Punkte; Höhenmodelle werden bilinear übertragen.
2. Kleine Lücken (bis 4 Zellen) werden aus den Nachbarzellen geschlossen.
3. Je Zelle im Bereich: **Δh = nachher − vorher**. Negatives Δh zählt als Abtrag, positives als Auftrag,
   jeweils × Zellfläche.
4. Zellen ohne Daten zählen nicht mit; ihr Anteil steht im Bericht (Warnung ab 5 %).

Ein Zellmittelpunkt gehört zu einem Bereich, wenn er im Polygon liegt. Die Fläche im Bericht ist die exakte
Polygonfläche.

## Bodenfilter

Für Aushub zählt das Gelände, nicht der Bagger darauf. Fixpunkt nutzt den *Simple Morphological Filter*
(Pingel et al. 2013): Minimum-Raster, morphologische Öffnung mit wachsendem Fenster (bis 10 m), was stärker als
die zulässige Neigung (Böschung bis 1 : 1,5) hervorsteht, ist Objekt.

Wo ein Objekt stand, ist nach dem Filtern ein Loch. Fixpunkt füllt es aus dem Gelände ringsum – **nur dort, wo
gemessen wurde**. Was keine Aufnahme gesehen hat, bleibt eine Lücke und erscheint als solche im Bericht.

## Haufwerke und Soll/Ist

In der Web-UI unter **Berechnungen** ([Haufwerk, Soll/Ist, Zeitreihe, Export](berechnungen.md)) und auf der
[Kommandozeile](kommandozeile.md):

- **Haufwerk** gegen eine Basisfläche: Ausgleichsebene durch die Höhen entlang der Grenze, tiefster Punkt der
  Grenze oder eine feste Höhe.
- **Soll/Ist** gegen eine Sohlhöhe, eine geneigte Ebene (Gefälle in %, Richtung) oder ein Soll-DGM: Abweichung je
  Zelle, Anteil innerhalb der Toleranz, noch abzutragende und aufzufüllende Massen.
- **Zeitreihe:** jede Aufnahme gegen das Urgelände und gegen die vorige.

## Genauigkeit

Die Rechnung selbst ist exakt bis auf die Rasterweite; die Genauigkeit des Ergebnisses kommt aus den Daten:

| Einfluss | Größenordnung |
|---|---|
| Höhengenauigkeit Drohne mit RTK | 2–5 cm |
| Rauschen der Punktwolke | wirkt sich auf die **Bilanz kaum** aus, erhöht aber Abtrag und Auftrag einzeln |
| Fehler 1 cm Höhe auf 1.000 m² | 10 m³ |

> [!IMPORTANT]
> Die Bilanz (netto) ist der belastbare Wert. Abtrag und Auftrag einzeln enthalten das Rauschen beider Aufnahmen:
> bei 2 cm Rauschen auf 4 ha je rund 150 m³, die sich in der Bilanz aufheben.

## Geprüft

Der Rechenkern wird bei jeder Änderung gegen unabhängig bekannte Ergebnisse getestet:

- Pyramidenstumpf (Grube mit Böschung) gegen die Prismoidformel, Kegel auf geneigtem Gelände gegen π r² h / 3 – Abweichung < 1 %
- 4 Mio. Punkte, 200 × 200 m, 2 cm Rauschen, Grube 25.600 m³ und Bagger: Bilanz −25.600,96 m³ (0,004 %), Bagger herausgefiltert, in 3,7 s
- UTM gegen die numerisch integrierte Meridianbogenlänge (< 0,1 mm), Rundreise Lage → UTM → Lage
- Helmert-Transformation mit vorgegebener Drehung, Maßstab und Verschiebung

Die nächste Stufe ist der Vergleich mit dem Aufmaß eines Vermessungsbüros auf einer echten Baugrube.
