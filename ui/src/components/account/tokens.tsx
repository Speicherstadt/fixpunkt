/** API-Tokens für Programme: anlegen (einmal im Klartext angezeigt), auflisten, widerrufen */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Copy, Plus } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ErrorText, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input, Label } from "@/components/ui/input";
import { api } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { tokensQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";
import { CredentialList } from "./credential-list";

export function Tokens() {
  const qc = useQueryClient();
  const { data = [] } = useQuery(tokensQuery);
  const [name, setName] = useState("");
  const [fresh, setFresh] = useState<string | null>(null);
  const refresh = () => qc.invalidateQueries({ queryKey: ["tokens"] });
  const add = useMutation({
    mutationFn: () => api.createToken(name.trim()),
    onSuccess: (t) => {
      setFresh(t.token);
      setName("");
      return refresh();
    },
  });
  const remove = useMutation({
    mutationFn: (id: number) => api.deleteToken(id),
    onSuccess: refresh,
  });
  const submit = (e: FormEvent) => {
    e.preventDefault();
    add.mutate();
  };
  return (
    <Section title={m.tokens_title()} description={m.tokens_description()}>
      {fresh ? (
        <div role="status" className="flex flex-col gap-2 rounded-lg bg-subtle p-3">
          <p className="m-0 text-[13px]">{m.tokens_once()}</p>
          <div className="flex items-center gap-2">
            <code className="tabular min-w-0 flex-1 break-all text-[13px]">{fresh}</code>
            <Button
              variant="ghost"
              size="iconSm"
              aria-label={m.tokens_copy()}
              onClick={() => navigator.clipboard?.writeText(fresh)}
            >
              <Copy size={16} aria-hidden="true" />
            </Button>
          </div>
        </div>
      ) : null}
      <CredentialList items={data} empty={m.tokens_empty()} onDelete={(c) => remove.mutate(c.id)} />
      <form onSubmit={submit} className="flex flex-wrap items-end gap-3">
        <Label className="min-w-48 flex-1">
          {m.tokens_name()}
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={m.tokens_name_placeholder()}
            required
          />
        </Label>
        <Button type="submit" disabled={add.isPending}>
          <Plus size={16} aria-hidden="true" />
          {m.tokens_add()}
        </Button>
      </form>
      <ErrorText>
        {add.error ? errorText(add.error) : remove.error ? errorText(remove.error) : null}
      </ErrorText>
    </Section>
  );
}
