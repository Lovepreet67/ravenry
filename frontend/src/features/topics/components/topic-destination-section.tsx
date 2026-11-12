import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Check, Link2, LoaderCircle, Search, Unlink } from "lucide-react";

import { listDestinations } from "@/features/destinations/api";
import type {
  Destination,
  DestinationType,
} from "@/features/destinations/types";
import type { PaginatedList } from "@/features/destinations/types";
import type {} from "@/features/destinations/types";
import { iamApiClient as apiClient } from "@/lib/api/clients";

import { getApiErrorMessage } from "@/lib/api-error";

const inputClass =
  "w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-primary";

type Props = {
  tenantId: string;
  topicId: string;
};

type DestinationLinkResponse = PaginatedList<Destination>;

async function listTopicDestinations(
  tenantId: string,
  topicId: string,
): Promise<DestinationLinkResponse> {
  const params = {
    topic_id: topicId,
    page_no: 1,
    page_size: 100,
    linked: true,
  };
  return listDestinations(tenantId, params);
}

async function attachDestination(
  tenantId: string,
  topicId: string,
  destinationId: string,
): Promise<void> {
  await apiClient.put(
    `/tenants/${tenantId}/topics/${topicId}/destinations/${destinationId}`,
  );
}

async function detachDestination(
  tenantId: string,
  topicId: string,
  destinationId: string,
): Promise<void> {
  await apiClient.delete(
    `/tenants/${tenantId}/topics/${topicId}/destinations/${destinationId}`,
  );
}

export function TopicDestinationsSection({ tenantId, topicId }: Props) {
  const queryClient = useQueryClient();

  const [nameInput, setNameInput] = useState("");
  const [kindInput, setKindInput] = useState("");
  const [nameFilter, setNameFilter] = useState("");
  const [kindFilter, setKindFilter] = useState<DestinationType | undefined>();
  const [pageNo, setPageNo] = useState(1);
  const pageSize = 10;

  const destinationsQuery = useQuery({
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

  const linkedQuery = useQuery({
    queryKey: ["topic-destinations", tenantId, topicId],
    queryFn: () => listTopicDestinations(tenantId, topicId),
  });

  const invalidateLists = async () => {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: ["topic-destinations", tenantId, topicId],
      }),
      queryClient.invalidateQueries({
        queryKey: ["destinations", tenantId],
      }),
    ]);
  };

  const attachMutation = useMutation({
    mutationFn: (destinationId: string) =>
      attachDestination(tenantId, topicId, destinationId),
    onSuccess: invalidateLists,
  });

  const detachMutation = useMutation({
    mutationFn: (destinationId: string) =>
      detachDestination(tenantId, topicId, destinationId),
    onSuccess: invalidateLists,
  });

  const destinations = destinationsQuery.data?.list ?? [];
  const linkedDestinations = linkedQuery.data?.list ?? [];
  const linkedIds = new Set(
    linkedDestinations.map((destination) => destination.id),
  );

  const pageInfo = destinationsQuery.data?.page_info;
  const totalPages = Math.max(
    1,
    Math.ceil((pageInfo?.total_rows ?? 0) / pageSize),
  );

  const isMutating = attachMutation.isPending || detachMutation.isPending;

  const mutationError = attachMutation.error ?? detachMutation.error;

  function applyFilters(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPageNo(1);
    setNameFilter(nameInput.trim());
    setKindFilter(kindInput ? (kindInput as DestinationType) : undefined);
  }

  return (
    <section className="space-y-5 rounded-xl border border-border bg-card p-6">
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">Destinations</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Manage the destinations subscribed to this topic.
          </p>
        </div>

        <span className="rounded-full border border-border px-2.5 py-1 text-xs text-muted-foreground">
          {linkedQuery.data?.page_info.total_rows ?? linkedDestinations.length}{" "}
          attached
        </span>
      </header>

      {mutationError && (
        <p role="alert" className="text-sm text-red-400">
          {getApiErrorMessage(mutationError)}
        </p>
      )}

      {linkedQuery.isError && (
        <div className="rounded-lg border border-border p-4">
          <p role="alert" className="text-sm text-red-400">
            {getApiErrorMessage(linkedQuery.error)}
          </p>
          <button
            onClick={() => void linkedQuery.refetch()}
            className="mt-2 text-sm text-primary hover:underline"
          >
            Try again
          </button>
        </div>
      )}

      <form
        onSubmit={applyFilters}
        className="grid gap-3 sm:grid-cols-[1fr_150px_auto]"
      >
        <input
          value={nameInput}
          onChange={(event) => setNameInput(event.target.value)}
          placeholder="Search destinations"
          aria-label="Destination name"
          className={inputClass}
        />

        <select
          value={kindInput}
          onChange={(event) => setKindInput(event.target.value)}
          aria-label="Destination type"
          className={inputClass}
        >
          <option value="">All types</option>
          <option value="http">HTTP</option>
        </select>

        <button
          type="submit"
          className="inline-flex items-center justify-center gap-2 rounded-lg border border-border px-4 py-2.5 text-sm hover:bg-background"
        >
          <Search size={15} />
          Filter
        </button>
      </form>

      {destinationsQuery.isPending || linkedQuery.isPending ? (
        <p className="flex items-center justify-center gap-2 py-10 text-sm text-muted-foreground">
          <LoaderCircle size={16} className="animate-spin" />
          Loading destinations...
        </p>
      ) : destinationsQuery.isError ? (
        <div className="rounded-lg border border-border p-5">
          <p role="alert" className="text-sm text-red-400">
            {getApiErrorMessage(destinationsQuery.error)}
          </p>
          <button
            onClick={() => void destinationsQuery.refetch()}
            className="mt-2 text-sm text-primary hover:underline"
          >
            Try again
          </button>
        </div>
      ) : destinations.length === 0 ? (
        <div className="rounded-lg border border-border p-8 text-center">
          <p className="font-medium">No destinations found</p>
          <p className="mt-1 text-sm text-muted-foreground">
            Create a destination or change your filters.
          </p>
          <Link
            to="/tenants/$tenantId/destinations"
            params={{ tenantId }}
            className="mt-3 inline-block text-sm text-primary hover:underline"
          >
            View destinations
          </Link>
        </div>
      ) : (
        <>
          <div className="overflow-x-auto rounded-lg border border-border">
            <table className="w-full text-left text-sm">
              <thead className="bg-background text-muted-foreground">
                <tr>
                  <th className="px-4 py-3 font-medium">Name</th>
                  <th className="px-4 py-3 font-medium">Type</th>
                  <th className="px-4 py-3 font-medium">Status</th>
                  <th className="px-4 py-3 font-medium">Subscription</th>
                  <th className="px-4 py-3 font-medium">Action</th>
                </tr>
              </thead>

              <tbody className="divide-y divide-border">
                {destinations.map((destination) => {
                  const isAttached = linkedIds.has(destination.id);

                  return (
                    <tr key={destination.id}>
                      <td className="px-4 py-4">
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
                        <p className="mt-1 max-w-xs truncate text-xs text-muted-foreground">
                          {destination.destination_url}
                        </p>
                      </td>

                      <td className="px-4 py-4 uppercase text-muted-foreground">
                        {destination.kind}
                      </td>

                      <td className="px-4 py-4">
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

                      <td className="px-4 py-4">
                        {isAttached ? (
                          <span className="inline-flex items-center gap-1.5 text-emerald-400">
                            <Check size={14} />
                            Attached
                          </span>
                        ) : (
                          <span className="text-muted-foreground">
                            Not attached
                          </span>
                        )}
                      </td>

                      <td className="px-4 py-4">
                        {isAttached ? (
                          <button
                            disabled={isMutating}
                            onClick={() =>
                              detachMutation.mutate(destination.id)
                            }
                            className="inline-flex items-center gap-1.5 text-sm text-red-400 hover:text-red-300 disabled:opacity-50"
                          >
                            <Unlink size={14} />
                            Detach
                          </button>
                        ) : (
                          <button
                            disabled={isMutating || !destination.is_active}
                            onClick={() =>
                              attachMutation.mutate(destination.id)
                            }
                            title={
                              !destination.is_active
                                ? "Activate this destination before attaching it"
                                : undefined
                            }
                            className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline disabled:opacity-50"
                          >
                            <Link2 size={14} />
                            Attach
                          </button>
                        )}
                      </td>
                    </tr>
                  );
                })}
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
    </section>
  );
}
