import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  ChevronDown,
  ChevronRight,
  Activity,
  RadioTower,
  Send,
  Clock,
  RefreshCw,
} from "lucide-react";

import {
  listEvents,
  listDeliveries,
  listDeliveryAttempts,
} from "@/features/events/api";
import type {
  Delivery,
  DeliveryAttempt,
  EventItem,
} from "@/features/events/types";
import { getApiErrorMessage } from "@/lib/api-error";

const PAGE_SIZE = 10;

function formatDate(value: string) {
  return new Date(value).toLocaleString();
}

function shortId(value: string) {
  return `${value.slice(0, 8)}…`;
}

function StatusBadge({ status }: { status: string }) {
  const normalized = status.toLowerCase();

  const styles =
    normalized === "success"
      ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
      : normalized === "failed"
        ? "border-red-500/30 bg-red-500/10 text-red-400"
        : "border-amber-500/30 bg-amber-500/10 text-amber-400";

  return (
    <span
      className={`inline-flex rounded-full border px-2.5 py-1 text-xs font-medium ${styles}`}
    >
      {status}
    </span>
  );
}

function AttemptsPanel({
  tenantId,
  deliveryId,
}: {
  tenantId: string;
  deliveryId: string;
}) {
  const attemptsQuery = useQuery({
    queryKey: ["delivery-attempts", tenantId, deliveryId],
    queryFn: () =>
      listDeliveryAttempts(tenantId, deliveryId, { limit: PAGE_SIZE }),
  });

  if (attemptsQuery.isPending) {
    return (
      <p className="p-4 text-sm text-muted-foreground">Loading attempts...</p>
    );
  }

  if (attemptsQuery.isError) {
    return (
      <p role="alert" className="p-4 text-sm text-red-400">
        {getApiErrorMessage(attemptsQuery.error)}
      </p>
    );
  }

  const attempts: DeliveryAttempt[] = attemptsQuery.data.attempts;

  if (attempts.length === 0) {
    return (
      <p className="p-4 text-sm text-muted-foreground">No attempts found.</p>
    );
  }

  return (
    <div className="space-y-3 p-4">
      {attempts.map((attempt) => (
        <div
          key={attempt.attempt_id}
          className="rounded-lg border border-border bg-background p-4"
        >
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <span className="text-sm font-semibold">
                Attempt #{attempt.attempt_number}
              </span>
              <StatusBadge status={attempt.success ? "success" : "failed"} />
            </div>
            <span className="text-xs text-muted-foreground">
              {formatDate(attempt.attempted_at)}
            </span>
          </div>

          <div className="mt-3 flex flex-wrap gap-4 text-sm">
            <span className="text-muted-foreground">
              HTTP status:{" "}
              <span className="font-medium text-foreground">
                {attempt.status_code ?? "—"}
              </span>
            </span>
          </div>

          {attempt.error && (
            <pre className="mt-3 overflow-x-auto whitespace-pre-wrap rounded-md bg-card p-3 text-xs text-red-300">
              {attempt.error}
            </pre>
          )}
        </div>
      ))}
    </div>
  );
}

function DeliveryRow({
  tenantId,
  delivery,
  expanded,
  onToggle,
}: {
  tenantId: string;
  delivery: Delivery;
  expanded: boolean;
  onToggle: () => void;
}) {
  return (
    <div className="border-t border-border">
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full flex-wrap items-center gap-3 px-5 py-4 text-left transition-colors hover:bg-card/70"
      >
        {expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
        <Send size={16} className="text-muted-foreground" />

        <span className="font-mono text-xs">
          {shortId(delivery.delivery_id)}
        </span>

        <StatusBadge status={delivery.status} />

        <span className="text-xs text-muted-foreground">
          Attempts: {delivery.current_attempt}
        </span>

        <span className="ml-auto text-xs text-muted-foreground">
          {formatDate(delivery.updated_at)}
        </span>
      </button>

      {expanded && (
        <div className="border-t border-border bg-background/50">
          <div className="grid gap-3 p-4 text-xs sm:grid-cols-2">
            <div>
              <p className="text-muted-foreground">Delivery ID</p>
              <p className="mt-1 break-all font-mono">{delivery.delivery_id}</p>
            </div>
            <div>
              <p className="text-muted-foreground">Destination ID</p>
              <p className="mt-1 break-all font-mono">
                {delivery.destination_id}
              </p>
            </div>
            <div>
              <p className="text-muted-foreground">Event ID</p>
              <p className="mt-1 break-all font-mono">{delivery.event_id}</p>
            </div>
            <div>
              <p className="text-muted-foreground">Created</p>
              <p className="mt-1">{formatDate(delivery.created_at)}</p>
            </div>
          </div>

          <div className="border-t border-border">
            <div className="flex items-center gap-2 px-4 pt-4">
              <Clock size={15} />
              <h4 className="text-sm font-semibold">Delivery attempts</h4>
            </div>
            <AttemptsPanel
              tenantId={tenantId}
              deliveryId={delivery.delivery_id}
            />
          </div>
        </div>
      )}
    </div>
  );
}

function EventRow({
  tenantId,
  event,
  expanded,
  expandedDeliveryId,
  onToggle,
  onToggleDelivery,
}: {
  tenantId: string;
  event: EventItem;
  expanded: boolean;
  expandedDeliveryId: string | null;
  onToggle: () => void;
  onToggleDelivery: (deliveryId: string) => void;
}) {
  const deliveriesQuery = useQuery({
    queryKey: ["event-deliveries", tenantId, event.event_id],
    queryFn: () =>
      listDeliveries(tenantId, event.event_id, { limit: PAGE_SIZE }),
    enabled: expanded,
  });

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-card">
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full flex-wrap items-center gap-3 p-4 text-left transition-colors hover:bg-background/50"
      >
        {expanded ? <ChevronDown size={17} /> : <ChevronRight size={17} />}
        <RadioTower size={17} className="text-muted-foreground" />

        <div className="min-w-0 flex-1">
          <p className="font-medium">{event.topic_path}</p>
          <p className="mt-1 truncate font-mono text-xs text-muted-foreground">
            {event.event_id}
          </p>
        </div>

        <span className="rounded-md border border-border px-2 py-1 text-xs">
          Schema v{event.schema_version}
        </span>

        <span className="text-xs text-muted-foreground">
          {formatDate(event.received_at)}
        </span>
      </button>

      {expanded && (
        <div className="border-t border-border">
          <div className="grid gap-2 bg-background/50 p-4 text-xs sm:grid-cols-2">
            <div>
              <p className="text-muted-foreground">Event ID</p>
              <p className="mt-1 break-all font-mono">{event.event_id}</p>
            </div>
            <div>
              <p className="text-muted-foreground">Topic ID</p>
              <p className="mt-1 break-all font-mono">{event.topic_id}</p>
            </div>
          </div>

          <div className="border-t border-border">
            <div className="flex items-center gap-2 px-4 py-4">
              <Send size={15} />
              <h3 className="text-sm font-semibold">Deliveries</h3>
            </div>

            {deliveriesQuery.isPending ? (
              <p className="px-4 pb-4 text-sm text-muted-foreground">
                Loading deliveries...
              </p>
            ) : deliveriesQuery.isError ? (
              <p role="alert" className="px-4 pb-4 text-sm text-red-400">
                {getApiErrorMessage(deliveriesQuery.error)}
              </p>
            ) : deliveriesQuery.data.items.length === 0 ? (
              <p className="px-4 pb-4 text-sm text-muted-foreground">
                No deliveries found.
              </p>
            ) : (
              <div>
                {deliveriesQuery.data.items.map((delivery) => (
                  <DeliveryRow
                    key={delivery.delivery_id}
                    tenantId={tenantId}
                    delivery={delivery}
                    expanded={expandedDeliveryId === delivery.delivery_id}
                    onToggle={() => onToggleDelivery(delivery.delivery_id)}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export function EventsPage({ tenantId }: { tenantId: string }) {
  const [expandedEventId, setExpandedEventId] = useState<string | null>(null);
  const [expandedDeliveryId, setExpandedDeliveryId] = useState<string | null>(
    null,
  );
  const [cursor, setCursor] = useState<string | null>(null);
  const [cursorHistory, setCursorHistory] = useState<(string | null)[]>([null]);

  const eventsQuery = useQuery({
    queryKey: ["tenant-events", tenantId, cursor],
    queryFn: () => listEvents(tenantId, { limit: PAGE_SIZE, cursor }),
  });

  function goNext() {
    const nextCursor = eventsQuery.data?.next_cursor;
    if (!nextCursor) return;

    setCursorHistory((history) => [...history, nextCursor]);
    setCursor(nextCursor);
    setExpandedEventId(null);
    setExpandedDeliveryId(null);
  }

  function goPrevious() {
    if (cursorHistory.length <= 1) return;

    const history = cursorHistory.slice(0, -1);
    setCursorHistory(history);
    setCursor(history[history.length - 1]);
    setExpandedEventId(null);
    setExpandedDeliveryId(null);
  }

  function refresh() {
    void eventsQuery.refetch();
  }

  return (
    <div className="mx-auto max-w-7xl space-y-6">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex items-start gap-3">
          <div className="rounded-xl border border-border bg-card p-3">
            <Activity size={22} />
          </div>
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">Events</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Inspect tenant events, deliveries, and delivery attempts.
            </p>
          </div>
        </div>

        <button
          type="button"
          onClick={refresh}
          disabled={eventsQuery.isFetching}
          className="inline-flex items-center gap-2 rounded-lg border border-border px-3 py-2 text-sm hover:bg-card disabled:opacity-50"
        >
          <RefreshCw size={15} />
          Refresh
        </button>
      </header>

      {eventsQuery.isPending ? (
        <p className="py-12 text-center text-sm text-muted-foreground">
          Loading events...
        </p>
      ) : eventsQuery.isError ? (
        <div className="rounded-xl border border-border p-6">
          <p role="alert" className="text-sm text-red-400">
            {getApiErrorMessage(eventsQuery.error)}
          </p>
          <button
            type="button"
            onClick={refresh}
            className="mt-3 text-sm text-primary hover:underline"
          >
            Retry
          </button>
        </div>
      ) : eventsQuery.data.items.length === 0 ? (
        <div className="rounded-xl border border-border p-12 text-center">
          <Activity size={28} className="mx-auto text-muted-foreground" />
          <p className="mt-3 font-medium">No events found</p>
          <p className="mt-1 text-sm text-muted-foreground">
            Events for this tenant will appear here.
          </p>
        </div>
      ) : (
        <>
          <div className="flex items-center justify-between text-sm text-muted-foreground">
            <span>Showing {eventsQuery.data.items.length} events</span>
            {eventsQuery.isFetching && <span>Refreshing...</span>}
          </div>

          <div className="space-y-3">
            {eventsQuery.data.items.map((event) => (
              <EventRow
                key={event.event_id}
                tenantId={tenantId}
                event={event}
                expanded={expandedEventId === event.event_id}
                expandedDeliveryId={expandedDeliveryId}
                onToggle={() => {
                  setExpandedEventId((current) =>
                    current === event.event_id ? null : event.event_id,
                  );
                  setExpandedDeliveryId(null);
                }}
                onToggleDelivery={(deliveryId) => {
                  setExpandedDeliveryId((current) =>
                    current === deliveryId ? null : deliveryId,
                  );
                }}
              />
            ))}
          </div>

          <div className="flex items-center justify-between border-t border-border pt-4">
            <button
              type="button"
              onClick={goPrevious}
              disabled={cursorHistory.length <= 1 || eventsQuery.isFetching}
              className="rounded-lg border border-border px-4 py-2 text-sm hover:bg-card disabled:opacity-40"
            >
              Previous
            </button>

            <span className="text-xs text-muted-foreground">
              Page {cursorHistory.length}
            </span>

            <button
              type="button"
              onClick={goNext}
              disabled={!eventsQuery.data.next_cursor || eventsQuery.isFetching}
              className="rounded-lg border border-border px-4 py-2 text-sm hover:bg-card disabled:opacity-40"
            >
              Next
            </button>
          </div>
        </>
      )}
    </div>
  );
}
