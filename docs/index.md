---
title: Aushub und Haufwerke aus Drohnenflügen berechnen
description: >-
  Fixpunkt rechnet aus Punktwolken und Höhenmodellen Aushub, Auftrag und Haufwerke je Abrechnungsbereich –
  selbst gehostet, eine Programmdatei, ETRS89/UTM32. Ein Rundgang in Bildern.
image: social-preview.png
hide:
  - navigation
  - toc
---

<!-- Die Seite zum Weitergeben: was Fixpunkt kann, in Bildern. Die Bilder entstehen mit
     bun run docs:screenshots (entwicklung.md). Aufbau wie firsttris/haben docs/index.md. -->

<div class="showcase-hero" markdown>

<p class="showcase-kicker">Baustellendokumentation, selbst gehostet</p>

# Der Aushub in m³, <span class="showcase-akzent">am Tag des Flugs.</span>

<p class="showcase-claim">Drohne fliegt, OpenDroneMap rechnet die Punktwolke, Fixpunkt vergleicht sie mit dem
Urgelände und liefert die Massen je Abrechnungsbereich – mit Bagger und Bewuchs herausgerechnet. Auf deinem
Server, als eine einzige Programmdatei.</p>

[Rundgang starten](#rundgang){ .md-button .md-button--primary } [Installieren](installation.md){ .md-button } [So wird gerechnet](methode.md){ .md-button }

<p class="showcase-hinweis">Fixpunkt ist in der ersten Phase: vor der Abrechnung mit Lieferscheinen oder Aufmaß plausibilisieren.</p>

</div>

<div class="showcase-shot showcase-shot--hero" markdown>
![Mengenbericht der Baugrube Kirchstraße 12: Abtrag 800,1 m³, Auftrag 0,0 m³, je Bereich Grube und Bagger](screenshot-bericht.png#only-light){ width="1440" height="900" }
![Mengenbericht der Baugrube Kirchstraße 12: Abtrag 800,1 m³, Auftrag 0,0 m³, je Bereich Grube und Bagger](screenshot-bericht-dunkel.png#only-dark){ width="1440" height="900" }
</div>

<div class="showcase-feature" id="rundgang" markdown>
<div markdown>
<span class="showcase-kicker">Baustellen</span>

## Jede Baustelle mit ihren Flügen

Für jede Baustelle sammelt Fixpunkt die Aufnahmen über die ganze Bauphase: Urgelände, Zwischenstände,
Endzustand. Auf einen Blick, wie viele Flüge es gibt und wann zuletzt geflogen wurde.

[Erste Schritte](erste-schritte.md)
</div>
<div class="showcase-shot" markdown>
![Liste der Baustellen mit Anzahl der Aufnahmen und Berechnungen und dem Datum der letzten Aufnahme](screenshot-baustellen.png#only-light){ width="1440" height="900" loading="lazy" }
![Liste der Baustellen mit Anzahl der Aufnahmen und Berechnungen und dem Datum der letzten Aufnahme](screenshot-baustellen-dunkel.png#only-dark){ width="1440" height="900" loading="lazy" }
</div>
</div>

<div class="showcase-feature showcase-feature--flip" markdown>
<div markdown>
<span class="showcase-kicker">Aufnahmen und Grenzen</span>

## Hochladen, was die Drohne liefert

Punktwolken als LAS, LAZ oder XYZ, Höhenmodelle als GeoTIFF – direkt aus OpenDroneMap. Fixpunkt prüft jede
Datei sofort und zeigt Punktzahl und Höhen. Die Abrechnungsgrenzen kommen als GeoJSON aus QGIS, metrisch oder
in Länge/Breite.

[Daten vorbereiten](daten.md)
</div>
<div class="showcase-shot" markdown>
![Reiter „Massen“ einer Baustelle: Abrechnungsbereiche und Berechnungen vom Urgelände bis zum fertigen Aushub](screenshot-baustelle.png#only-light){ width="1440" loading="lazy" }
![Reiter „Massen“ einer Baustelle: Abrechnungsbereiche und Berechnungen vom Urgelände bis zum fertigen Aushub](screenshot-baustelle-dunkel.png#only-dark){ width="1440" loading="lazy" }
</div>
</div>

<div class="showcase-feature" markdown>
<div markdown>
<span class="showcase-kicker">Mengenbericht</span>

## Abtrag, Auftrag, Bilanz – je Bereich

Zwei Aufnahmen auswählen, „Aushub berechnen“, fertig. Der Bericht zeigt die Massen je Abrechnungsbereich,
warnt bei Lücken in den Daten und geht als PDF mit Karte, als CSV oder Differenzraster (GeoTIFF) an Bauleitung und Büro.

[Aushub berechnen](aushub.md)
</div>
<div class="showcase-shot" markdown>
![Mengenbericht mit Kennzahlen und Tabelle je Bereich](screenshot-bericht.png#only-light){ width="1440" height="900" loading="lazy" }
![Mengenbericht mit Kennzahlen und Tabelle je Bereich](screenshot-bericht-dunkel.png#only-dark){ width="1440" height="900" loading="lazy" }
</div>
</div>

<div class="showcase-feature showcase-feature--flip" markdown>
<div markdown>
<span class="showcase-kicker">Ansicht und Messen</span>

## Messen und zeichnen, ohne QGIS

Das Höhenbild jeder Aufnahme mit der Differenz oder der Soll/Ist-Abweichung darüber. Strecken, Gefälle,
Flächen und Profile direkt im Bild messen, Abrechnungsbereiche einfach anklicken – mit Maus oder Tastatur.

[Ansicht und Messen](ansicht.md)
</div>
<div class="showcase-shot" markdown>
![Ansicht der Baugrube mit Soll/Ist-Abweichung, Bereichen und Profil](screenshot-ansicht.png){ width="1024" height="1216" loading="lazy" }
</div>
</div>

<div class="showcase-feature" markdown>
<div markdown>
<span class="showcase-kicker">Auf der Baustelle</span>

## Am Handy wie am Schreibtisch

Fixpunkt lässt sich auf dem Tablet oder Handy installieren. Der Bericht passt sich an, die Zahlen bleiben
lesbar – auch draußen, hell oder dunkel.

[Benutzer und Rollen](benutzer.md)
</div>
<div class="showcase-phone" markdown>
![Mengenbericht am Handy mit Kennzahlen und einer Karte je Bereich](screenshot-handy.png){ width="390" height="844" loading="lazy" }
</div>
</div>

<div class="showcase-center" markdown>

## Was dahintersteckt

</div>

<div class="showcase-cards" markdown>
<div markdown>
**Bodenfilter**

Bagger, Container und Bewuchs werden aus der Punktwolke entfernt; darunter wird das Gelände ergänzt, wo gemessen wurde.
</div>
<div markdown>
**Amtliche Koordinaten**

ETRS89/UTM32 (EPSG:25832), Höhen in DHHN2016 über das Quasigeoid GCG2016 – ohne PROJ oder GDAL.
</div>
<div markdown>
**Eine Programmdatei**

Server, Web-UI und Rechenkern in einer Datei, mit SQLite. Docker-Image für amd64 und arm64.
</div>
<div markdown>
**Geprüft**

Volumen gegen Kegel- und Pyramidenstumpfformel, 4 Mio. Punkte in Sekunden, Tests auf jeder Schicht.
</div>
</div>

<div class="showcase-center" markdown>

[Installieren](installation.md){ .md-button .md-button--primary } [Funktionsumfang](funktionen.md){ .md-button } [Quellcode auf GitHub](https://github.com/Speicherstadt/fixpunkt){ .md-button }

</div>
