import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import {
  ArrowLeft,
  Building2,
  CalendarDays,
  Pencil,
  Users,
} from "lucide-react";

import { getTenant } from "@/features/tenants/api";
import { getApiErrorMessage } from "@/lib/api-error";

function formatDate(value: string) {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function TenantDetailsPage() {
  const { tenantId } = useParams({
    from: "/tenants/$tenantId",
  });

  const tenantQuery = useQuery({
    queryKey: ["tenant", tenantId],
    queryFn: () => getTenant(tenantId),
  });

  if (tenantQuery.isPending) {
    return (
      <p className="py-12 text-center text-sm text-muted-foreground">
        Loading tenant...
      </p>
    );
  }

  if (tenantQuery.isError) {
    return (
      <div className="space-y-4">
        <Link
          to="/tenants"
          className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft size={15} />
          Back to tenants
        </Link>

        <p role="alert" className="text-sm text-red-400">
          {getApiErrorMessage(tenantQuery.error)}
        </p>
      </div>
    );
  }

  const tenant = tenantQuery.data;

  return (
    <div className="mx-auto max-w-6xl space-y-7">
      <Link
        to="/tenants"
        className="inline-flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
      >
        <ArrowLeft size={15} />
        Back to tenants
      </Link>

      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex items-start gap-4">
          <div className="rounded-xl border border-border bg-card p-3">
            <Building2 size={24} />
          </div>

          <div>
            <h1 className="text-2xl font-semibold tracking-tight">
              {tenant.name}
            </h1>
            <p className="mt-1 text-sm text-muted-foreground">{tenant.slug}</p>

            <span
              className={`mt-3 inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-medium ${
                tenant.is_active
                  ? "border-emerald-500/30 text-emerald-400"
                  : "border-border text-muted-foreground"
              }`}
            >
              {tenant.is_active ? "Active" : "Inactive"}
            </span>
          </div>
        </div>

        <Link
          to="/tenants/$tenantId/edit"
          params={{ tenantId: tenant.id }}
          className="inline-flex items-center gap-2 rounded-lg border border-border px-4 py-2.5 text-sm font-medium transition-colors hover:bg-card"
        >
          <Pencil size={15} />
          Edit tenant
        </Link>
      </header>

      <section className="space-y-5 rounded-xl border border-border bg-card p-6">
        <h2 className="text-lg font-semibold">Tenant information</h2>

        <div className="grid gap-6 sm:grid-cols-2">
          <div>
            <p className="text-xs text-muted-foreground">Tenant name</p>
            <p className="mt-1 text-sm font-medium">{tenant.name}</p>
          </div>

          <div>
            <p className="text-xs text-muted-foreground">Slug</p>
            <p className="mt-1 text-sm font-medium">{tenant.slug}</p>
          </div>

          <div className="flex items-start gap-3">
            <CalendarDays size={18} className="mt-0.5 text-muted-foreground" />
            <div>
              <p className="text-xs text-muted-foreground">Created</p>
              <p className="mt-1 text-sm font-medium">
                {formatDate(tenant.created_at)}
              </p>
            </div>
          </div>

          <div className="flex items-start gap-3">
            <CalendarDays size={18} className="mt-0.5 text-muted-foreground" />
            <div>
              <p className="text-xs text-muted-foreground">Last updated</p>
              <p className="mt-1 text-sm font-medium">
                {formatDate(tenant.updated_at)}
              </p>
            </div>
          </div>
        </div>
      </section>

      <section className="rounded-xl border border-border bg-card p-6">
        <div className="flex items-start gap-4">
          <div className="rounded-lg border border-border p-3">
            <Users size={20} />
          </div>

          <div className="flex-1">
            <h2 className="font-semibold">Tenant memberships</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Manage users and their roles across your tenants.
            </p>

            <Link
              to="/memberships"
              className="mt-4 inline-flex items-center gap-2 text-sm font-medium text-primary hover:underline"
            >
              Manage memberships
            </Link>
          </div>
        </div>

        <div className="mt-5 rounded-lg border border-border p-4">
          <p className="text-sm text-muted-foreground">
            Membership management is available from the Memberships page. A
            tenant-specific member list will require a backend endpoint that
            lists memberships by tenant.
          </p>
        </div>
      </section>

      <section className="rounded-xl border border-border bg-card p-6">
        <h2 className="font-semibold">Destinations</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Manage the HTTP endpoints configured for this tenant.
        </p>

        <Link
          to="/tenants/$tenantId/destinations"
          params={{ tenantId: tenant.id }}
          className="mt-4 inline-flex items-center gap-2 rounded-lg border border-border px-4 py-2.5 text-sm font-medium hover:bg-background"
        >
          Manage destinations
        </Link>
      </section>
      <section className="rounded-xl border border-border bg-card p-6">
        <h2 className="font-semibold">Topics</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Manage the Topics for this tenant.
        </p>

        <Link
          to="/tenants/$tenantId/topics"
          params={{ tenantId: tenant.id }}
          className="mt-4 inline-flex items-center gap-2 rounded-lg border border-border px-4 py-2.5 text-sm font-medium hover:bg-background"
        >
          Manage topics
        </Link>
      </section>
    </div>
  );
}
