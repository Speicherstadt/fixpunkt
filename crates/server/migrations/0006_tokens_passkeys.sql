-- API-Tokens (z. B. für die iOS-App oder Skripte): gespeichert wird nur der SHA-256 des Tokens
CREATE TABLE api_tokens (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT    NOT NULL,
    token_hash   TEXT    NOT NULL UNIQUE,
    created_at   INTEGER NOT NULL,
    last_used_at INTEGER
);

-- Passkeys (WebAuthn): öffentlicher Schlüssel und Zähler als JSON von webauthn-rs
CREATE TABLE passkeys (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT    NOT NULL,
    cred_id      TEXT    NOT NULL UNIQUE,
    passkey      TEXT    NOT NULL,
    created_at   INTEGER NOT NULL,
    last_used_at INTEGER
);
