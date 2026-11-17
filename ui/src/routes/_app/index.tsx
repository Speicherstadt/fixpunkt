import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { ChevronRight, Plus } from "lucide-react";
import { type FormEvent, useMemo, useState } from "react";
import { EmptyState, ErrorText, PageHeader } from "@/components/page";
import { SitesMap } from "@/components/sites-map";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Input, Label, Textarea } from "@/components/ui/input";
import { PlaceholderRows } from "@/components/ui/placeholder";
import { api } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatDate } from "@/lib/format";
import { authQuery, settingsQuery, sitesQuery } from "@/lib/queries";
import { mapSites } from "@/lib/sites-map";
import { m } from "@/paraglide/messages";

export const Route = createFileRoute("/_app/")({ component: SitesPage });

function SitesPage() {
  const sites = useQuery(sitesQuery);
  const settings = useQuery(settingsQuery);
  const located = useMemo(() => mapSites(sites.data ?? []), [sites.data]);
  const { data: auth } = useQuery(authQuery);
  const canEdit = auth?.user?.role !== "viewer";
  const [open, setOpen] = useState(false);
  return (
    <>
      <PageHeader
        title={m.sites_title()}
        subtitle={m.sites_subtitle()}
        actions={
          canEdit ? (
            <Button variant="primary" onClick={() => setOpen(true)}>
              <Plus size={16} aria-hidden="true" />
              {m.sites_new()}
            </Button>
          ) : null
        }
      />
      {sites.isPending ? (
        <PlaceholderRows />
      ) : sites.data?.length ? (
        <div className="flex flex-col gap-6">
          {located.length && settings.data ? (
            <SitesMap sites={located} tiles={settings.data.mapTiles} />
          ) : null}
          <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-xl border border-border p-0">
            {sites.data.map((s) => (
              <li key={s.id}>
                <Link
                  to="/sites/$siteId"
                  params={{ siteId: String(s.id) }}
                  className="flex items-center gap-4 px-5 py-4 hover:bg-hover"
                >
                  <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                    <span className="truncate font-medium">{s.name}</span>
                    <span className="truncate text-[13px] text-muted">
                      {m.sites_counts({ captures: s.captures, jobs: s.jobs })}
                      {s.lastCaptureAt
                        ? ` · ${m.sites_last({ date: formatDate(s.lastCaptureAt) })}`
                        : ""}
                    </span>
                  </div>
                  <ChevronRight size={16} aria-hidden="true" className="text-muted" />
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ) : (
        <EmptyState>{m.sites_empty()}</EmptyState>
      )}
      <NewSiteDialog open={open} onOpenChange={setOpen} />
    </>
  );
}

function NewSiteDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
}) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const create = useMutation({
    mutationFn: () => api.createSite(name, description),
    onSuccess: async (site) => {
      await queryClient.invalidateQueries({ queryKey: ["sites"] });
      onOpenChange(false);
      setName("");
      setDescription("");
      // eine neue Baustelle braucht zuerst Aufnahmen
      await navigate({ to: "/sites/$siteId/aufnahmen", params: { siteId: String(site.id) } });
    },
  });
  const submit = (e: FormEvent) => {
    e.preventDefault();
    create.mutate();
  };
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={m.sites_new()}
      description={m.sites_new_text()}
    >
      <form onSubmit={submit} className="flex flex-col gap-4">
        <Label>
          {m.sites_name()}
          <Input value={name} onChange={(e) => setName(e.target.value)} required autoFocus />
        </Label>
        <Label>
          {m.sites_description()}
          <Textarea value={description} onChange={(e) => setDescription(e.target.value)} />
        </Label>
        <ErrorText>{create.error ? errorText(create.error) : null}</ErrorText>
        <div className="flex justify-end gap-2">
          <Button onClick={() => onOpenChange(false)}>{m.common_cancel()}</Button>
          <Button type="submit" variant="primary" disabled={create.isPending}>
            {m.sites_create()}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
