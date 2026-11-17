import { useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, redirect, useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { ErrorText } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input, Label } from "@/components/ui/input";
import { api } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { authQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

export const Route = createFileRoute("/login")({
  beforeLoad: async ({ context }) => {
    const auth = await context.queryClient.fetchQuery(authQuery);
    if (auth.user) throw redirect({ to: "/" });
  },
  component: LoginPage,
});

function LoginPage() {
  const { data } = useQuery(authQuery);
  const setup = data?.setupRequired ?? false;
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await (setup ? api.setup(name, password) : api.login(name, password));
      await queryClient.invalidateQueries({ queryKey: ["auth"] });
      await navigate({ to: "/" });
    } catch (err) {
      setError(errorText(err));
      setBusy(false);
    }
  };

  return (
    <main className="flex min-h-full items-center justify-center bg-side p-4">
      <form
        onSubmit={submit}
        className="flex w-full max-w-sm flex-col gap-5 rounded-2xl border border-border bg-bg p-6 shadow-sm"
      >
        <div className="flex flex-col gap-1">
          <img src="/favicon.svg" alt="" className="mb-2 size-10" />
          <h1 className="m-0 text-[22px] font-semibold">
            {setup ? m.login_setup_title() : m.login_title()}
          </h1>
          <p className="m-0 text-[13px] text-muted">
            {setup ? m.login_setup_text() : m.login_text()}
          </p>
        </div>
        <Label>
          {m.login_name()}
          <Input
            name="name"
            autoComplete="username"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            autoFocus
          />
        </Label>
        <Label>
          {m.login_password()}
          <Input
            name="password"
            type="password"
            autoComplete={setup ? "new-password" : "current-password"}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={setup ? 8 : undefined}
          />
        </Label>
        <ErrorText>{error}</ErrorText>
        <Button type="submit" variant="primary" disabled={busy}>
          {setup ? m.login_setup_submit() : m.login_submit()}
        </Button>
      </form>
    </main>
  );
}
