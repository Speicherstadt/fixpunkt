#!/bin/sh
# Installiert oder aktualisiert Fixpunkt als systemd-Dienst, ohne Docker:
#
#   curl -fsSL https://raw.githubusercontent.com/Speicherstadt/fixpunkt/main/scripts/install.sh | sudo sh
#
# Für ein Update einfach erneut ausführen. Was es einrichtet (Vorbild: firsttris/haul scripts/install.sh):
#   /usr/local/bin/fixpunkt                 Server mit eingebauter Web-UI und Rechenkern
#   /etc/fixpunkt/fixpunkt.env              Einstellungen (einmal geschrieben, bei Updates behalten)
#   /etc/systemd/system/fixpunkt.service    läuft als Systembenutzer „fixpunkt“
#   /var/lib/fixpunkt                       Datenbank, Aufnahmen, Ergebnisse
#
# Variablen:
#   FIXPUNKT_VERSION=1.2.3   diese Version statt der neuesten
#   FIXPUNKT_PORT=8080       Port der Web-UI bei der ersten Installation
set -eu

REPO=Speicherstadt/fixpunkt
BIN=/usr/local/bin/fixpunkt
ENV_FILE=/etc/fixpunkt/fixpunkt.env
UNIT=/etc/systemd/system/fixpunkt.service
DATA=/var/lib/fixpunkt

say() { printf '\033[1m%s\033[0m\n' "$*"; }
warn() { printf '\033[33m%s\033[0m\n' "$*" >&2; }
die() { printf '\033[31m%s\033[0m\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

[ "$(id -u)" -eq 0 ] || die "Bitte als root ausführen, z. B.: curl -fsSL … | sudo sh"
[ "$(uname -s)" = Linux ] || die "Die Programmdatei gibt es nur für Linux; sonst Docker verwenden."
[ -d /run/systemd/system ] || die "systemd läuft nicht; Docker verwenden oder $BIN selbst starten."

case "$(uname -m)" in
  x86_64 | amd64) ARCH=x86_64 ;;
  aarch64 | arm64) ARCH=aarch64 ;;
  *) die "Keine Programmdatei für $(uname -m); Docker verwenden oder selbst bauen." ;;
esac

fetch() {
  if have curl; then
    curl -fsSL --retry 3 -o "$2" "$1"
  elif have wget; then
    wget -q -O "$2" "$1"
  else
    die "Weder curl noch wget gefunden."
  fi
}

# ---- 1. Programmdatei ---------------------------------------------------------------------
ASSET="fixpunkt-linux-$ARCH.tar.gz"
if [ -n "${FIXPUNKT_VERSION:-}" ]; then
  URL="https://github.com/$REPO/releases/download/v${FIXPUNKT_VERSION#v}/$ASSET"
else
  URL="https://github.com/$REPO/releases/latest/download/$ASSET"
fi
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
say "Lade $URL"
fetch "$URL" "$TMP/$ASSET" || die "Download fehlgeschlagen: $URL"
fetch "$URL.sha256" "$TMP/$ASSET.sha256" || die "Download fehlgeschlagen: $URL.sha256"
if have sha256sum; then
  (cd "$TMP" && sha256sum -c "$ASSET.sha256" >/dev/null) || die "Prüfsumme von $ASSET stimmt nicht."
else
  warn "sha256sum fehlt, die Prüfsumme wird nicht geprüft."
fi
tar -xzf "$TMP/$ASSET" -C "$TMP"
UPDATE=false
[ -f "$BIN" ] && UPDATE=true
install -m 0755 "$TMP/fixpunkt" "$BIN"

# ---- 2. Benutzer, Ordner, Einstellungen -----------------------------------------------------
if ! id fixpunkt >/dev/null 2>&1; then
  NOLOGIN=$(command -v nologin || echo /bin/false)
  if have useradd; then
    useradd --system --home-dir "$DATA" --no-create-home --shell "$NOLOGIN" fixpunkt
  else
    adduser -S -D -H -h "$DATA" -s "$NOLOGIN" fixpunkt
  fi
fi
mkdir -p "$DATA"
chown fixpunkt:fixpunkt "$DATA"

if [ ! -f "$ENV_FILE" ]; then
  mkdir -p "$(dirname "$ENV_FILE")"
  cat >"$ENV_FILE" <<EOT
# Einstellungen von Fixpunkt, gelesen von fixpunkt.service. Nach einer Änderung: systemctl restart fixpunkt
FIXPUNKT_DATA_DIR=$DATA
FIXPUNKT_LISTEN=0.0.0.0:${FIXPUNKT_PORT:-8080}
# Gleichzeitige Berechnungen (je eine braucht für große Flüge einige GB Arbeitsspeicher)
FIXPUNKT_MAX_JOBS=1
# Hinter einem Reverse-Proxy mit HTTPS einschalten
FIXPUNKT_SECURE_COOKIES=false
EOT
  chown root:fixpunkt "$ENV_FILE"
  chmod 0640 "$ENV_FILE"
  say "Einstellungen in $ENV_FILE"
fi

# ---- 3. Dienst --------------------------------------------------------------------------------
cat >"$UNIT" <<EOT
# Geschrieben von install.sh, wird bei Updates ersetzt. Eigene Änderungen: systemctl edit fixpunkt
[Unit]
Description=Fixpunkt – 3D-Baustellendokumentation
After=network-online.target
Wants=network-online.target

[Service]
User=fixpunkt
Group=fixpunkt
EnvironmentFile=$ENV_FILE
ExecStart=$BIN
Restart=on-failure
RestartSec=5
TimeoutStopSec=30
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=full
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictSUIDSGID=yes
LockPersonality=yes

[Install]
WantedBy=multi-user.target
EOT
systemctl daemon-reload
if [ "$UPDATE" = true ]; then
  systemctl enable fixpunkt >/dev/null 2>&1
  systemctl restart fixpunkt
  say "Fixpunkt aktualisiert: $("$BIN" --version 2>/dev/null || echo "$URL")"
else
  systemctl enable --now fixpunkt >/dev/null 2>&1
  PORT=$(sed -n 's/^FIXPUNKT_LISTEN=.*://p' "$ENV_FILE" | tail -n 1)
  HOST=$(hostname -I 2>/dev/null | awk '{print $1}')
  say "Fixpunkt läuft: http://${HOST:-localhost}:${PORT:-8080}"
  echo "Beim ersten Aufruf legst du den Admin-Zugang an."
  echo "Einstellungen: $ENV_FILE  ·  Logs: journalctl -u fixpunkt -f  ·  Update: dieses Skript erneut ausführen"
fi
