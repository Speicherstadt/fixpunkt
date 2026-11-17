/**
 * WebAuthn im Browser: Optionen vom Server (JSON von webauthn-rs, Binärfelder base64url) in die Form für
 * `navigator.credentials` bringen und die Antwort zurück in JSON. Eigene Umwandlung, weil
 * `PublicKeyCredential.parseCreationOptionsFromJSON` noch nicht überall läuft.
 */

export const fromB64url = (s: string): Uint8Array<ArrayBuffer> => {
  const b64 = s
    .replace(/-/g, "+")
    .replace(/_/g, "/")
    .padEnd(Math.ceil(s.length / 4) * 4, "=");
  const bin = atob(b64);
  const out = new Uint8Array(new ArrayBuffer(bin.length));
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
};

export const toB64url = (buf: ArrayBuffer | ArrayBufferView): string => {
  const bytes =
    buf instanceof ArrayBuffer
      ? new Uint8Array(buf)
      : new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
};

type Descriptor = { id: string; type: string; transports?: string[] };
type Json = Record<string, unknown>;

const descriptors = (list: unknown) =>
  (list as Descriptor[] | undefined)?.map((d) => ({
    ...d,
    id: fromB64url(d.id),
    type: "public-key" as const,
    transports: d.transports as AuthenticatorTransport[] | undefined,
  }));

/** `{ publicKey: … }` von /me/passkeys/start → Optionen für `navigator.credentials.create` */
export const creationOptions = (options: Json): CredentialCreationOptions => {
  const pk = options.publicKey as Json;
  const user = pk.user as Json;
  return {
    publicKey: {
      ...(pk as unknown as PublicKeyCredentialCreationOptions),
      challenge: fromB64url(pk.challenge as string),
      user: {
        ...(user as unknown as PublicKeyCredentialUserEntity),
        id: fromB64url(user.id as string),
      },
      excludeCredentials: descriptors(pk.excludeCredentials),
    },
  };
};

/** `{ publicKey: … }` von /auth/passkey/start → Optionen für `navigator.credentials.get` */
export const requestOptions = (options: Json): CredentialRequestOptions => {
  const pk = options.publicKey as Json;
  return {
    publicKey: {
      ...(pk as unknown as PublicKeyCredentialRequestOptions),
      challenge: fromB64url(pk.challenge as string),
      allowCredentials: descriptors(pk.allowCredentials),
    },
  };
};

/** Antwort von `create` als JSON für webauthn-rs (`RegisterPublicKeyCredential`) */
export const registrationJson = (cred: PublicKeyCredential) => {
  const r = cred.response as AuthenticatorAttestationResponse;
  return {
    id: cred.id,
    rawId: toB64url(cred.rawId),
    type: cred.type,
    response: {
      attestationObject: toB64url(r.attestationObject),
      clientDataJSON: toB64url(r.clientDataJSON),
      transports: r.getTransports?.() ?? [],
    },
    extensions: cred.getClientExtensionResults(),
  };
};

/** Antwort von `get` als JSON für webauthn-rs (`PublicKeyCredential`) */
export const assertionJson = (cred: PublicKeyCredential) => {
  const r = cred.response as AuthenticatorAssertionResponse;
  return {
    id: cred.id,
    rawId: toB64url(cred.rawId),
    type: cred.type,
    response: {
      authenticatorData: toB64url(r.authenticatorData),
      clientDataJSON: toB64url(r.clientDataJSON),
      signature: toB64url(r.signature),
      userHandle: r.userHandle ? toB64url(r.userHandle) : null,
    },
    extensions: cred.getClientExtensionResults(),
  };
};

/** Abbruch durch den Nutzer (Dialog geschlossen, Zeit abgelaufen) – kein Fehler, der angezeigt werden muss */
export const isCancelled = (e: unknown) =>
  e instanceof DOMException && (e.name === "NotAllowedError" || e.name === "AbortError");
