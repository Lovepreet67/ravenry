CREATE TABLE deliveries (
    delivery_id     UUID PRIMARY KEY,

    tenant_id       UUID        NOT NULL,
    event_id        UUID        NOT NULL,
    topic_id        UUID        NOT NULL,
    destination_id  UUID        NOT NULL,

    status          VARCHAR(20) NOT NULL,
    current_attempt INTEGER     NOT NULL DEFAULT 0,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_delivery_event
        FOREIGN KEY (event_id) REFERENCES events (event_id),

    CONSTRAINT chk_delivery_status
        CHECK (status IN ('pending', 'success', 'failed'))
);

CREATE INDEX idx_deliveries_tenant_created
    ON deliveries (tenant_id, created_at DESC);

CREATE INDEX idx_deliveries_tenant_event
    ON deliveries (tenant_id, event_id);

CREATE INDEX idx_deliveries_tenant_destination
    ON deliveries (tenant_id, destination_id);

CREATE INDEX idx_deliveries_tenant_status
    ON deliveries (tenant_id, status);


CREATE TABLE delivery_attempts (
    attempt_id     UUID PRIMARY KEY,
    delivery_id    UUID        NOT NULL,

    attempt_number INTEGER     NOT NULL,

    success        BOOLEAN     NOT NULL,
    status_code    INTEGER,
    error          TEXT,

    attempted_at   TIMESTAMPTZ NOT NULL,               -- when Outbound ran it
    recorded_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(), -- when we stored it

    CONSTRAINT fk_attempt_delivery
        FOREIGN KEY (delivery_id) REFERENCES deliveries (delivery_id),

    CONSTRAINT uq_delivery_attempt_number
        UNIQUE (delivery_id, attempt_number),

    CONSTRAINT chk_attempt_number
        CHECK (attempt_number > 0)
);

-- Covers per-delivery history, newest first.
CREATE INDEX idx_delivery_attempts_delivery
    ON delivery_attempts (delivery_id, attempt_number DESC);