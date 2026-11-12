import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { Plus, Search } from "lucide-react";

import { listDestinations } from "@/features/destinations/api";
import { getApiErrorMessage } from "@/lib/api-error";
import type { DestinationType } from "@/features/destinations/types";

const inputClass =
  "w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-primary";

export function DestinationsPage() {
  const { tenantId } = useParams({
    from: "/tenants/$tenantId/destinations",
  });

  const [nameInput, setNameInput] = useState("");
  const [kindInput, setKindInput] = useState("");
  const [nameFilter, setNameFilter] = useState("");
  const [kindFilter, setKindFilter] = useState<DestinationType | undefined>();
  const [pageNo, setPageNo] = useState(1);
  const pageSize = 10;

  const query = useQuery({
    queryKey: [
      "destinations",
      tenantId,
      pageNo,
      pageSize,
      nameFilter,
      kindFilter,
    ],
    queryFn: () =>
      listDestinations(tenantId, {
        page_no: pageNo,
        page_size: pageSize,
        ...(nameFilter ? { name: nameFilter } : {}),
        ...(kindFilter ? { kind: kindFilter } : {}),
      }),
  });

  const destinations = query.data?.list ?? [];
  const pageInfo = query.data?.page_info;
  const totalPages = Math.max(
    1,
    Math.ceil((pageInfo?.total_rows ?? 0) / pageSize),
  );

  function applyFilters(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPageNo(1);
    setNameFilter(nameInput.trim());
    setKindFilter(kindInput ? (kindInput as DestinationType) : undefined);
  }

  return (
    <div className="mx-auto max-w-6xl space-y-7">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            Destinations
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Configure HTTP endpoints that receive your tenant's events.
          </p>
        </div>

        <Link
          to="/tenants/$tenantId/destinations/create"
          params={{ tenantId }}
          className="inline-flex items-center gap-2 rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-background"
        >
          <Plus size={16} />
          Create destination
        </Link>
      </header>

      <form
        onSubmit={applyFilters}
        className="grid gap-3 rounded-xl border border-border bg-card p-4 sm:grid-cols-[1fr_180px_auto]"
      >
        <input
          value={nameInput}
          onChange={(event) => setNameInput(event.target.value)}
          placeholder="Search by destination name"
          className={inputClass}
          aria-label="Destination name"
        />

        <select
          value={kindInput}
          onChange={(event) => setKindInput(event.target.value)}
          className={inputClass}
          aria-label="Destination type"
        >
          <option value="">All types</option>
          <option value="http">HTTP</option>
        </select>

        <button
          type="submit"
          className="inline-flex items-center justify-center gap-2 rounded-lg border border-border px-4 py-2.5 text-sm font-medium hover:bg-background"
        >
          <Search size={15} />
          Apply filters
        </button>
      </form>

      {query.isPending ? (
        <p className="py-12 text-center text-sm text-muted-foreground">
          Loading destinations...
        </p>
      ) : query.isError ? (
        <div className="rounded-xl border border-border p-6">
          <p role="alert" className="text-sm text-red-400">
            {getApiErrorMessage(query.error)}
          </p>
          <button
            onClick={() => void query.refetch()}
            className="mt-3 text-sm text-primary hover:underline"
          >
            Try again
          </button>
        </div>
      ) : destinations.length === 0 ? (
        <div className="rounded-xl border border-border p-10 text-center">
          <h2 className="font-medium">No destinations found</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Create a destination or change your filters.
          </p>
        </div>
      ) : (
        <>
          <div className="overflow-x-auto rounded-xl border border-border">
            <table className="w-full text-left text-sm">
              <thead className="bg-card text-muted-foreground">
                <tr>
                  <th className="px-5 py-3 font-medium">Name</th>
                  <th className="px-5 py-3 font-medium">Type</th>
                  <th className="px-5 py-3 font-medium">URL</th>
                  <th className="px-5 py-3 font-medium">Timeout</th>
                  <th className="px-5 py-3 font-medium">Status</th>
                  <th className="px-5 py-3 font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {destinations.map((destination) => (
                  <tr key={destination.id}>
                    <td className="px-5 py-4">
                      <Link
                        to="/tenants/$tenantId/destinations/$destinationId"
                        params={{
                          tenantId,
                          destinationId: destination.id,
                        }}
                        className="font-medium hover:text-primary"
                      >
                        {destination.name}
                      </Link>
                    </td>
                    <td className="px-5 py-4 uppercase text-muted-foreground">
                      {destination.kind}
                    </td>
                    <td className="max-w-xs truncate px-5 py-4 text-muted-foreground">
                      {destination.destination_url}
                    </td>
                    <td className="px-5 py-4">
                      {destination.delivery_timeout_seconds}s
                    </td>
                    <td className="px-5 py-4">
                      <span
                        className={
                          destination.is_active
                            ? "text-emerald-400"
                            : "text-muted-foreground"
                        }
                      >
                        {destination.is_active ? "Active" : "Inactive"}
                      </span>
                    </td>
                    <td className="px-5 py-4">
                      <Link
                        to="/tenants/$tenantId/destinations/$destinationId/edit"
                        params={{
                          tenantId,
                          destinationId: destination.id,
                        }}
                        className="text-muted-foreground hover:text-foreground"
                      >
                        Edit
                      </Link>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="flex flex-wrap items-center justify-between gap-3">
            <p className="text-sm text-muted-foreground">
              {pageInfo?.total_rows ?? 0} destinations · Page {pageNo} of{" "}
              {totalPages}
            </p>

            <div className="flex gap-2">
              <button
                disabled={pageNo <= 1}
                onClick={() => setPageNo((page) => page - 1)}
                className="rounded-lg border border-border px-3 py-2 text-sm disabled:opacity-40"
              >
                Previous
              </button>
              <button
                disabled={pageNo >= totalPages}
                onClick={() => setPageNo((page) => page + 1)}
                className="rounded-lg border border-border px-3 py-2 text-sm disabled:opacity-40"
              >
                Next
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
