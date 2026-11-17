/** Passkeys im eigenen Konto: anlegen (Gerät, Sicherheitsschlüssel, Passwortmanager), auflisten, löschen */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ErrorText, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input, Label } from "@/components/ui/input";
import { api } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { passkeysQuery } from "@/lib/queries";
import { creationOptions, isCancelled, registrationJson } from "@/lib/webauthn";
import { m } from "@/paraglide/messages";
import { CredentialList } from "./credential-list";

/** Passkey im Browser erzeugen und beim Server registrieren; `false` bei Abbruch */
const registerPasskey = async (name: string): Promise<boolean> => {
  const start = await api.passkeyRegisterStart();
  try {
    const cred = (await navigator.credentials.create(
      creationOptions(start.options),
    )) as PublicKeyCredential | null;
    if (!cred) return false;
    await api.passkeyRegisterFinish(start.id, name, registrationJson(cred));
    return true;
  } catch (e) {
    if (isCancelled(e)) return false;
    throw e;
  }
};

export function Passkeys() {
  const qc = useQueryClient();
  const { data = [] } = useQuery(passkeysQuery);
  const [name, setName] = useState("");
  const refresh = () => qc.invalidateQueries({ queryKey: ["passkeys"] });
  const add = useMutation({
    mutationFn: () => registerPasskey(name.trim()),
    onSuccess: (done) => {
      if (done) setName("");
      return refresh();
    },
  });
  const remove = useMutation({
    mutationFn: (id: number) => api.deletePasskey(id),
    onSuccess: refresh,
  });
  const submit = (e: FormEvent) => {
    e.preventDefault();
    add.mutate();
  };
  return (
    <Section title={m.passkeys_title()} description={m.passkeys_description()}>
      <CredentialList
        items={data}
        empty={m.passkeys_empty()}
        onDelete={(c) => remove.mutate(c.id)}
      />
      <form onSubmit={submit} className="flex flex-wrap items-end gap-3">
        <Label className="min-w-48 flex-1">
          {m.passkeys_name()}
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={m.passkeys_name_placeholder()}
          />
        </Label>
        <Button type="submit" variant="primary" disabled={add.isPending}>
          <KeyRound size={16} aria-hidden="true" />
          {m.passkeys_add()}
        </Button>
      </form>
      <ErrorText>
        {add.error ? errorText(add.error) : remove.error ? errorText(remove.error) : null}
      </ErrorText>
    </Section>
  );
}
