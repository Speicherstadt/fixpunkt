import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, redirect } from "@tanstack/react-router";
import { Trash2 } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ErrorText, PageHeader, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input, Label, Select } from "@/components/ui/input";
import { api, type Role } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { usersQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

export const Route = createFileRoute("/_app/users")({
  beforeLoad: ({ context }) => {
    if (context.user.role !== "admin") throw redirect({ to: "/" });
  },
  component: UsersPage,
});

const ROLE_TEXT: Record<Role, () => string> = {
  admin: m.role_admin,
  editor: m.role_editor,
  viewer: m.role_viewer,
};

function UsersPage() {
  const users = useQuery(usersQuery);
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<Role>("editor");
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["users"] });
  const add = useMutation({
    mutationFn: () => api.addUser(name, password, role),
    onSuccess: async () => {
      setName("");
      setPassword("");
      await refresh();
    },
  });
  const remove = useMutation({ mutationFn: api.deleteUser, onSuccess: refresh });
  const submit = (e: FormEvent) => {
    e.preventDefault();
    add.mutate();
  };
  return (
    <>
      <PageHeader title={m.users_title()} subtitle={m.users_subtitle()} />
      <div className="flex flex-col gap-6">
        <Section title={m.users_list()}>
          <ErrorText>{remove.error ? errorText(remove.error) : null}</ErrorText>
          <ul className="m-0 flex list-none flex-col divide-y divide-border p-0">
            {users.data?.map((u) => (
              <li key={u.id} className="flex items-center justify-between gap-3 py-2">
                <span className="flex flex-col">
                  <span className="font-medium">{u.name}</span>
                  <span className="text-[12px] text-muted">{ROLE_TEXT[u.role]()}</span>
                </span>
                <Button
                  variant="ghost"
                  size="iconSm"
                  onClick={() => remove.mutate(u.id)}
                  aria-label={m.users_delete({ name: u.name })}
                >
                  <Trash2 size={16} aria-hidden="true" />
                </Button>
              </li>
            ))}
          </ul>
        </Section>
        <Section title={m.users_new()} description={m.users_new_text()}>
          <form onSubmit={submit} className="grid gap-4 md:grid-cols-3">
            <Label>
              {m.login_name()}
              <Input value={name} onChange={(e) => setName(e.target.value)} required />
            </Label>
            <Label>
              {m.users_initial_password()}
              <Input
                type="password"
                autoComplete="new-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                minLength={8}
              />
            </Label>
            <Label>
              {m.users_role()}
              <Select value={role} onChange={(e) => setRole(e.target.value as Role)}>
                {(["viewer", "editor", "admin"] as const).map((r) => (
                  <option key={r} value={r}>
                    {ROLE_TEXT[r]()}
                  </option>
                ))}
              </Select>
            </Label>
            <div className="flex flex-col gap-2 md:col-span-3">
              <ErrorText>{add.error ? errorText(add.error) : null}</ErrorText>
              <div className="flex justify-end">
                <Button type="submit" variant="primary" disabled={add.isPending}>
                  {m.users_create()}
                </Button>
              </div>
            </div>
          </form>
        </Section>
      </div>
    </>
  );
}
