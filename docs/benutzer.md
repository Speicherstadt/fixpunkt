---
description: >-
  Benutzer und Rollen in Fixpunkt: Admin, Bearbeiten, Nur lesen. Zugänge anlegen und löschen, Passwort ändern,
  Sicherheit der Anmeldung.
---

# Benutzer und Rollen

| Rolle | darf |
|---|---|
| **Admin** | alles, dazu Benutzer anlegen und löschen |
| **Bearbeiten** | Baustellen, Aufnahmen, Bereiche und Berechnungen anlegen, ändern, löschen |
| **Nur lesen** | alles ansehen und herunterladen, z. B. für Bauherr oder Prüfer |

Admins legen Zugänge unter **Benutzer** an: Name, erstes Passwort, Rolle. Das erste Passwort am besten persönlich
weitergeben; geändert wird es unter **Konto**. Der letzte Admin lässt sich nicht löschen.

## Sicherheit

- Passwörter werden mit **argon2id** gespeichert, nie im Klartext.
- Die Sitzung steckt in einem Cookie, das Skripte nicht lesen können (`HttpOnly`, `SameSite=Strict`), hinter HTTPS
  zusätzlich `Secure` (`FIXPUNKT_SECURE_COOKIES=true`).
- Jede ändernde Anfrage trägt ein **CSRF-Token** der Sitzung.
- Falsche Anmeldungen werden gebremst.

Anmeldung mit Passkeys und API-Tokens für die spätere iOS-App sind geplant.
