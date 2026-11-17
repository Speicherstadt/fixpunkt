import { useMutation, useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { Passkeys } from "@/components/account/passkeys";
import { Tokens } from "@/components/account/tokens";
import { ErrorText, PageHeader, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input, Label, Select } from "@/components/ui/input";
import { api } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { authQuery } from "@/lib/queries";
import { applyTheme, storedTheme, type Theme } from "@/lib/theme";
import { m } from "@/paraglide/messages";
import { getLocale, locales, setLocale } from "@/paraglide/runtime";

export const Route = createFileRoute("/_app/account")({ component: AccountPage });

function AccountPage() {
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [theme, setTheme] = useState<Theme>(storedTheme);
  const { data: auth } = useQuery(authQuery);
  const change = useMutation({
    mutationFn: () => api.changePassword(current, next),
    onSuccess: () => {
      setCurrent("");
      setNext("");
    },
  });
  const submit = (e: FormEvent) => {
    e.preventDefault();
    change.mutate();
  };
  return (
    <>
      <PageHeader title={m.account_title()} />
      <div className="flex flex-col gap-6">
        <Section title={m.account_display()}>
          <div className="grid gap-4 md:grid-cols-2">
            <Label>
              {m.account_theme()}
              <Select
                value={theme}
                onChange={(e) => {
                  const t = e.target.value as Theme;
                  setTheme(t);
                  applyTheme(t);
                }}
              >
                <option value="system">{m.account_theme_system()}</option>
                <option value="light">{m.account_theme_light()}</option>
                <option value="dark">{m.account_theme_dark()}</option>
              </Select>
            </Label>
            <Label>
              {m.account_language()}
              <Select
                value={getLocale()}
                onChange={(e) => setLocale(e.target.value as (typeof locales)[number])}
              >
                {locales.map((l) => (
                  <option key={l} value={l}>
                    {l === "de" ? "Deutsch" : "English"}
                  </option>
                ))}
              </Select>
            </Label>
          </div>
        </Section>
        <Section title={m.account_password()}>
          <form onSubmit={submit} className="grid gap-4 md:grid-cols-2">
            <Label>
              {m.account_password_current()}
              <Input
                type="password"
                autoComplete="current-password"
                value={current}
                onChange={(e) => setCurrent(e.target.value)}
                required
              />
            </Label>
            <Label>
              {m.account_password_new()}
              <Input
                type="password"
                autoComplete="new-password"
                value={next}
                onChange={(e) => setNext(e.target.value)}
                required
                minLength={8}
              />
            </Label>
            <div className="flex flex-col gap-2 md:col-span-2">
              <ErrorText>{change.error ? errorText(change.error) : null}</ErrorText>
              {change.isSuccess ? (
                <p role="status" className="m-0 text-[13px] text-ok">
                  {m.account_password_changed()}
                </p>
              ) : null}
              <div className="flex justify-end">
                <Button type="submit" variant="primary" disabled={change.isPending}>
                  {m.account_password_submit()}
                </Button>
              </div>
            </div>
          </form>
        </Section>
        {auth?.passkeys ? <Passkeys /> : null}
        <Tokens />
      </div>
    </>
  );
}
