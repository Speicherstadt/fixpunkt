import { useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, Link, Outlet, redirect, useNavigate } from "@tanstack/react-router";
import { HardHat, LogOut, UserRound, Users } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { authQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

export const Route = createFileRoute("/_app")({
  beforeLoad: async ({ context }) => {
    const auth = await context.queryClient.fetchQuery(authQuery);
    if (!auth.user) throw redirect({ to: "/login" });
    return { user: auth.user };
  },
  component: AppShell,
});

const navLink =
  "flex h-9 items-center gap-2.5 rounded-lg px-3 text-[14px] text-muted hover:bg-hover hover:text-fg [&.active]:bg-subtle [&.active]:font-medium [&.active]:text-fg";

function AppShell() {
  const { data } = useQuery(authQuery);
  const user = data?.user;
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const logout = async () => {
    await api.logout();
    queryClient.clear();
    await navigate({ to: "/login" });
  };
  return (
    <div className="flex min-h-full flex-col md:flex-row">
      <a
        href="#inhalt"
        className="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-50 focus:rounded-lg focus:bg-bg focus:px-3 focus:py-2"
      >
        {m.nav_skip()}
      </a>
      <aside className="flex shrink-0 flex-col gap-1 border-border bg-side p-3 md:w-sidebar md:border-r">
        <Link to="/" className="mb-3 flex items-center gap-2 px-3 py-2 font-semibold">
          <img src="/favicon.svg" alt="" className="size-6" />
          Fixpunkt
        </Link>
        <nav aria-label={m.nav_label()} className="flex gap-1 md:flex-col">
          <Link to="/" className={navLink} activeOptions={{ exact: true }}>
            <HardHat size={16} aria-hidden="true" />
            {m.nav_sites()}
          </Link>
          {user?.role === "admin" ? (
            <Link to="/users" className={navLink}>
              <Users size={16} aria-hidden="true" />
              {m.nav_users()}
            </Link>
          ) : null}
          <Link to="/account" className={navLink}>
            <UserRound size={16} aria-hidden="true" />
            {m.nav_account()}
          </Link>
        </nav>
        <div className="mt-auto hidden flex-col gap-2 pt-6 md:flex">
          <span className="px-3 text-[12px] text-muted">{user?.name}</span>
          <Button variant="ghost" size="sm" onClick={logout} className="justify-start">
            <LogOut size={14} aria-hidden="true" />
            {m.nav_logout()}
          </Button>
        </div>
      </aside>
      <main id="inhalt" className="min-w-0 flex-1 p-4 md:p-8">
        <div className="mx-auto max-w-5xl">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
