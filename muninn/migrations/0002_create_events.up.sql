CREATE TABLE events (
    event_id       UUID PRIMARY KEY,

    tenant_id      UUID        NOT NULL,
    topic_id       UUID        NOT NULL,
    topic_path     TEXT        NOT NULL,

    schema_version INTEGER     NOT NULL,
    payload        JSONB       NOT NULL,

    received_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_events_tenant_received
    ON events (tenant_id, received_at DESC);

CREATE INDEX idx_events_tenant_topic_received
    ON events (tenant_id, topic_id, received_at DESC);

CREATE INDEX idx_events_tenant_topic_path
    ON events (tenant_id, topic_path);