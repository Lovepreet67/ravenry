-- Add up migration script here
CREATE TABLE IF NOT EXISTS outbox_events(
    id BIGSERIAL PRIMARY KEY,
    event_type VARCHAR(128) NOT NULL,
    schema_version SMALLINT NOT NULL DEFAULT 1,
    aggregate_id UUID NOT NULL,              -- the tenant id, since each event means "re-project this tenant"
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),   -- lets a failed event be retried later with backoff
    processed_at TIMESTAMPTZ,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);

-- the worker's claim query only ever looks at unprocessed rows,
-- so index just those; the index stays small however large the table gets
CREATE INDEX IF NOT EXISTS idx_outbox_events_pending
ON outbox_events (available_at, id)
WHERE processed_at IS NULL;

-- for claiming by aggregate, so one tenant's events are handled in order
CREATE INDEX IF NOT EXISTS idx_outbox_events_aggregate_pending
ON outbox_events (aggregate_id, id)
WHERE processed_at IS NULL;

-- for the scheduled cleanup of old processed rows
CREATE INDEX IF NOT EXISTS idx_outbox_events_processed
ON outbox_events (processed_at)
WHERE processed_at IS NOT NULL;