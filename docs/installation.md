---
description: >-
  Fixpunkt installieren: Docker oder Podman mit dem Image tristanteu/fixpunkt, oder ohne Container als
  systemd-Dienst mit install.sh. Erster Start und Einrichtung des Admins.
---

# Installation

Fixpunkt ist **eine Programmdatei**: Server, Web-UI und Rechenkern zusammen, Daten in SQLite und einem Ordner.
Es braucht keine Datenbank, keinen Objektspeicher und keinen weiteren Dienst.

Keinen eigenen Server oder keine eigene IT? Fixpunkt gibt es auf Anfrage gehostet oder über einen IT-Dienstleister
aus dem Partnerprogramm, siehe [Gehostet oder mit Partner](gehostet.md).

Voraussetzungen: Linux-Server (amd64 oder arm64), ab 4 GB RAM; für große Flüge (mehrere Millionen Punkte) 8–16 GB.
Plattenplatz je nach Datenmenge: eine Punktwolke aus einer Baugrubenbefliegung hat typisch 50–500 MB.

## Docker oder Podman

```bash
docker run -d --name fixpunkt -p 8080:8080 -v fixpunkt:/data --restart unless-stopped tristanteu/fixpunkt
```

Danach `http://<server>:8080` öffnen und den ersten Zugang anlegen – er wird Admin.

=== "Docker Compose"

    ```yaml
    services:
      fixpunkt:
        image: tristanteu/fixpunkt:latest
        restart: unless-stopped
        ports:
          - "8080:8080"
        volumes:
          - fixpunkt:/data
        environment:
          FIXPUNKT_MAX_JOBS: "1"
    volumes:
      fixpunkt:
    ```

=== "Podman Quadlet"

    `/etc/containers/systemd/fixpunkt.container`:

    ```ini
    [Unit]
    Description=Fixpunkt

    [Container]
    Image=docker.io/tristanteu/fixpunkt:latest
    PublishPort=8080:8080
    Volume=fixpunkt.volume:/data
    AutoUpdate=registry

    [Service]
    Restart=always

    [Install]
    WantedBy=multi-user.target
    ```

    Dazu `/etc/containers/systemd/fixpunkt.volume` mit dem Inhalt `[Volume]`, dann
    `systemctl daemon-reload && systemctl start fixpunkt`.

Das Image läuft als Benutzer `fixpunkt` (UID 1000), nicht als root, und hat einen Healthcheck auf `/api/health`.

## Ohne Container (systemd)

```bash
curl -fsSL https://raw.githubusercontent.com/Speicherstadt/fixpunkt/main/scripts/install.sh | sudo sh
```

Das Skript lädt die statische Programmdatei der neuesten Version (Prüfsumme wird kontrolliert) und richtet ein:

| Pfad | Inhalt |
|---|---|
| `/usr/local/bin/fixpunkt` | Server mit Web-UI und Rechenkern |
| `/etc/fixpunkt/fixpunkt.env` | Einstellungen, bei Updates behalten |
| `/etc/systemd/system/fixpunkt.service` | Dienst als Systembenutzer `fixpunkt` |
| `/var/lib/fixpunkt` | Datenbank, Aufnahmen, Ergebnisse |

Für ein Update das Skript erneut ausführen. Variablen: `FIXPUNKT_VERSION=1.2.3` (bestimmte Version),
`FIXPUNKT_PORT=8080` (Port bei der ersten Installation).

## Erster Start

Beim ersten Aufruf fragt Fixpunkt nach Name und Passwort (mindestens 8 Zeichen) für den ersten Zugang. Für eine
Installation ohne Klicken lässt er sich auch vorgeben: `FIXPUNKT_USER` und `FIXPUNKT_PASSWORD`
([Konfiguration](konfiguration.md)).

> [!IMPORTANT]
> Fixpunkt gehört hinter einen Reverse-Proxy mit HTTPS (Caddy, Traefik, nginx), sobald es aus dem Internet
> erreichbar ist – dann `FIXPUNKT_SECURE_COOKIES=true` setzen und für Passkeys die Adresse angeben:
> `FIXPUNKT_PUBLIC_URL=https://fixpunkt.example.de`.

## Drohnenbilder auswerten

Die Fotos der Drohne rechnet [OpenDroneMap](https://opendronemap.org/) zur Punktwolke. Mit einem NodeODM neben
Fixpunkt und `FIXPUNKT_NODEODM_URL` lädt man die Fotos direkt in Fixpunkt hoch, das Ergebnis landet als Aufnahme:
[Drohnenfotos auswerten](drohnenfotos.md). Ohne NodeODM lädt man die Punktwolke aus WebODM hoch.
