import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Plus, RefreshCw, Search } from "lucide-react";

import { listTenants } from "@/features/tenants/api";
import { getApiErrorMessage } from "@/lib/api-error";

type ActiveFilter = "all" | "active" | "inactive";

export function TenantsPage() {
  const [pageNo, setPageNo] = useState(1);
  const [pageSize, setPageSize] = useState(25);
  const [activeFilter, setActiveFilter] = useState<ActiveFilter>("all");

  const query = useQuery({
    queryKey: ["tenants", pageNo, pageSize, activeFilter],
    queryFn: () =>
      listTenants({
        page_no: pageNo,
        page_size: pageSize,
        ...(activeFilter === "all"
          ? {}
          : { is_active: activeFilter === "active" }),
      }),
  });

  const tenants = query.data?.list ?? [];
  const pageInfo = query.data?.page_info;
  const totalRows = pageInfo?.total_rows ?? 0;
  const totalPages = Math.max(1, Math.ceil(totalRows / pageSize));

  return (
    <div className="space-y-7">
      <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
        <div className="space-y-2">
          <p className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
            Workspace / Administration
          </p>
          <h1 className="text-3xl font-semibold tracking-tight">Tenants</h1>
          <p className="text-sm text-muted-foreground">
            Manage organizations and their workspace access.
          </p>
        </div>

        <Link
          to="/tenants/create"
          className="inline-flex h-10 items-center justify-center gap-2 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition hover:opacity-90"
        >
          <Plus size={16} />
          Create tenant
        </Link>
      </div>

      <section className="overflow-hidden rounded-xl border border-border bg-card">
        <div className="flex flex-col gap-3 border-b border-border p-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h2 className="font-medium">All tenants</h2>
            <p className="mt-1 text-xs text-muted-foreground">
              {totalRows} total tenants
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <label className="flex h-9 items-center gap-2 rounded-md border border-border bg-background px-3 text-sm">
              <Search size={15} className="text-muted-foreground" />
              <span className="sr-only">Filter tenant status</span>
              <select
                value={activeFilter}
                onChange={(event) => {
                  setActiveFilter(event.target.value as ActiveFilter);
                  setPageNo(1);
                }}
                className="bg-transparent text-foreground outline-none"
              >
                <option value="all">All statuses</option>
                <option value="active">Active</option>
                <option value="inactive">Inactive</option>
              </select>
            </label>

            <button
              type="button"
              onClick={() => query.refetch()}
              disabled={query.isFetching}
              aria-label="Refresh tenants"
              className="flex size-9 items-center justify-center rounded-md border border-border transition hover:bg-accent disabled:opacity-50"
            >
              <RefreshCw
                size={15}
                className={query.isFetching ? "animate-spin" : ""}
              />
            </button>
          </div>
        </div>

        {query.isPending ? (
          <div className="space-y-3 p-6">
            {Array.from({ length: 5 }, (_, index) => (
              <div
                key={index}
                className="h-10 animate-pulse rounded-md bg-muted"
              />
            ))}
          </div>
        ) : query.isError ? (
          <div className="p-8 text-center">
            <p className="font-medium">Unable to load tenants</p>
            <p className="mt-2 text-sm text-muted-foreground">
              {getApiErrorMessage(query.error)}
            </p>
            <button
              type="button"
              onClick={() => query.refetch()}
              className="mt-4 rounded-md border border-border px-3 py-2 text-sm hover:bg-accent"
            >
              Try again
            </button>
          </div>
        ) : tenants.length === 0 ? (
          <div className="p-12 text-center">
            <p className="font-medium">No tenants found</p>
            <p className="mt-2 text-sm text-muted-foreground">
              Try another status filter or create a tenant.
            </p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="bg-muted/40 text-xs text-muted-foreground">
                <tr>
                  <th className="px-5 py-3 font-medium">Tenant</th>
                  <th className="px-5 py-3 font-medium">Slug</th>
                  <th className="px-5 py-3 font-medium">Status</th>
                  <th className="px-5 py-3 font-medium">Created</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {tenants.map((tenant) => (
                  <tr key={tenant.id} className="transition hover:bg-accent/30">
                    <td className="px-5 py-4">
                      <Link
                        to="/tenants/$tenantId"
                        params={{ tenantId: tenant.id }}
                        className="font-medium hover:text-primary"
                      >
                        {tenant.name}
                      </Link>
                      <p className="mt-1 max-w-52 truncate text-xs text-muted-foreground">
                        {tenant.id}
                      </p>
                    </td>
                    <td className="px-5 py-4 font-mono text-xs text-muted-foreground">
                      {tenant.slug}
                    </td>
                    <td className="px-5 py-4">
                      <span
                        className={`inline-flex items-center gap-2 rounded-full border px-2.5 py-1 text-xs ${
                          tenant.is_active
                            ? "border-emerald-500/20 bg-emerald-500/10 text-emerald-400"
                            : "border-border bg-muted text-muted-foreground"
                        }`}
                      >
                        <span
                          className={`size-1.5 rounded-full ${
                            tenant.is_active
                              ? "bg-emerald-400"
                              : "bg-muted-foreground"
                          }`}
                        />
                        {tenant.is_active ? "Active" : "Inactive"}
                      </span>
                    </td>
                    <td className="whitespace-nowrap px-5 py-4 text-muted-foreground">
                      {new Date(tenant.created_at).toLocaleDateString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <div className="flex flex-col gap-3 border-t border-border px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
          <p className="text-xs text-muted-foreground">
            Page {pageNo} of {totalPages}
            {totalRows > 0 && ` · ${totalRows} records`}
          </p>

          <div className="flex items-center gap-3">
            <label className="flex items-center gap-2 text-xs text-muted-foreground">
              Rows
              <select
                value={pageSize}
                onChange={(event) => {
                  setPageSize(Number(event.target.value));
                  setPageNo(1);
                }}
                className="rounded-md border border-border bg-background px-2 py-1.5 text-foreground"
              >
                <option value={10}>10</option>
                <option value={25}>25</option>
                <option value={50}>50</option>
                <option value={100}>100</option>
              </select>
            </label>

            <button
              type="button"
              disabled={pageNo <= 1 || query.isFetching}
              onClick={() => setPageNo((page) => page - 1)}
              className="rounded-md border border-border px-3 py-1.5 text-xs transition hover:bg-accent disabled:cursor-not-allowed disabled:opacity-40"
            >
              Previous
            </button>

            <button
              type="button"
              disabled={pageNo >= totalPages || query.isFetching}
              onClick={() => setPageNo((page) => page + 1)}
              className="rounded-md border border-border px-3 py-1.5 text-xs transition hover:bg-accent disabled:cursor-not-allowed disabled:opacity-40"
            >
              Next
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
