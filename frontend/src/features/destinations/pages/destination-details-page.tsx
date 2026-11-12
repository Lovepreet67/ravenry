import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { ArrowLeft, CalendarDays, Pencil } from "lucide-react";

import { getDestination } from "@/features/destinations/api";
import { getApiErrorMessage } from "@/lib/api-error";

function formatDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function DestinationDetailsPage() {
  const { tenantId, destinationId } = useParams({
    from: "/tenants/$tenantId/destinations/$destinationId",
  });

  const query = useQuery({
    queryKey: ["destination", tenantId, destinationId],
    queryFn: () => getDestination(tenantId, destinationId),
  });

  if (query.isPending) {
    return (
      <p className="py-12 text-center text-sm text-muted-foreground">
        Loading destination...
      </p>
    );
  }

  if (query.isError) {
    return (
      <div className="space-y-4">
        <Link
          to="/tenants/$tenantId/destinations"
          params={{ tenantId }}
          className="inline-flex items-center gap-2 text-sm text-muted-foreground"
        >
          <ArrowLeft size={15} />
          Back to destinations
        </Link>
        <p role="alert" className="text-sm text-red-400">
          {getApiErrorMessage(query.error)}
        </p>
      </div>
    );
  }

  const destination = query.data;

  return (
    <div className="mx-auto max-w-5xl space-y-7">
      <Link
        to="/tenants/$tenantId/destinations"
        params={{ tenantId }}
        className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft size={15} />
        Back to destinations
      </Link>

      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            {destination.name}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Destination details and delivery configuration.
          </p>
          <span
            className={`mt-3 inline-flex rounded-full border px-2.5 py-1 text-xs ${
              destination.is_active
                ? "border-emerald-500/30 text-emerald-400"
                : "border-border text-muted-foreground"
            }`}
          >
            {destination.is_active ? "Active" : "Inactive"}
          </span>
        </div>

        <Link
          to="/tenants/$tenantId/destinations/$destinationId/edit"
          params={{ tenantId, destinationId }}
          className="inline-flex items-center gap-2 rounded-lg border border-border px-4 py-2.5 text-sm hover:bg-card"
        >
          <Pencil size={15} />
          Edit destination
        </Link>
      </header>

      <section className="space-y-6 rounded-xl border border-border bg-card p-6">
        <h2 className="text-lg font-semibold">Configuration</h2>

        <div className="grid gap-6 sm:grid-cols-2">
          <div>
            <p className="text-xs text-muted-foreground">Name</p>
            <p className="mt-1 text-sm font-medium">{destination.name}</p>
          </div>

          <div>
            <p className="text-xs text-muted-foreground">Type</p>
            <p className="mt-1 text-sm font-medium uppercase">
              {destination.kind}
            </p>
          </div>

          <div className="sm:col-span-2">
            <p className="text-xs text-muted-foreground">Destination URL</p>
            <a
              href={destination.destination_url}
              target="_blank"
              rel="noreferrer"
              className="mt-1 block break-all text-sm text-primary hover:underline"
            >
              {destination.destination_url}
            </a>
          </div>

          <div>
            <p className="text-xs text-muted-foreground">Delivery timeout</p>
            <p className="mt-1 text-sm font-medium">
              {destination.delivery_timeout_seconds} seconds
            </p>
          </div>

          <div>
            <p className="text-xs text-muted-foreground">Status</p>
            <p className="mt-1 text-sm font-medium">
              {destination.is_active ? "Active" : "Inactive"}
            </p>
          </div>
        </div>
      </section>

      <section className="space-y-5 rounded-xl border border-border bg-card p-6">
        <h2 className="text-lg font-semibold">Audit information</h2>

        <div className="grid gap-6 sm:grid-cols-2">
          <div>
            <p className="text-xs text-muted-foreground">Created by</p>
            <p className="mt-1 break-all text-sm">{destination.created_by}</p>
          </div>

          <div>
            <p className="text-xs text-muted-foreground">Last updated by</p>
            <p className="mt-1 break-all text-sm">
              {destination.last_updated_by}
            </p>
          </div>

          <div className="flex items-start gap-3">
            <CalendarDays size={17} className="mt-0.5 text-muted-foreground" />
            <div>
              <p className="text-xs text-muted-foreground">Created at</p>
              <p className="mt-1 text-sm">
                {formatDate(destination.created_at)}
              </p>
            </div>
          </div>

          <div className="flex items-start gap-3">
            <CalendarDays size={17} className="mt-0.5 text-muted-foreground" />
            <div>
              <p className="text-xs text-muted-foreground">Updated at</p>
              <p className="mt-1 text-sm">
                {formatDate(destination.updated_at)}
              </p>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
