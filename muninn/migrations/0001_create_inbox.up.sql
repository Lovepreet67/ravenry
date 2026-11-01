CREATE TABLE inbox_messages (
    id               BIGSERIAL PRIMARY KEY,

    source           VARCHAR(50)  NOT NULL,
    stream_name      VARCHAR(255)         NOT NULL,
    queue_message_id VARCHAR(255)       NOT NULL,

    message_type     VARCHAR(100) NOT NULL,
    schema_version   INTEGER,

    -- Parsed envelope. NULL when the message could not be parsed as JSON.
    payload          JSONB,
    -- Original bytes. Always kept for invalid messages, optional otherwise.
    raw_payload      BYTEA,

    status           VARCHAR(20)  NOT NULL DEFAULT 'pending',

    attempts         INTEGER      NOT NULL DEFAULT 0,  -- real processing failures
    deferrals        INTEGER      NOT NULL DEFAULT 0,  -- waiting on a dependency
    available_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    locked_by        TEXT,
    locked_until     TIMESTAMPTZ,

    received_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    processed_at     TIMESTAMPTZ,

    last_error       TEXT,

    CONSTRAINT uq_inbox_stream_message
        UNIQUE (stream_name, queue_message_id),

    CONSTRAINT chk_inbox_status
        CHECK (status IN ('pending', 'processing', 'processed', 'failed', 'quarantined')),

    CONSTRAINT chk_inbox_has_body
        CHECK (payload IS NOT NULL OR raw_payload IS NOT NULL),

    CONSTRAINT chk_inbox_lease
        CHECK (status <> 'processing' OR locked_until IS NOT NULL)
);

-- Hot path: processors claim pending rows per source.
CREATE INDEX idx_inbox_claim
    ON inbox_messages (source, available_at, id)
    WHERE status = 'pending';

-- Reaper: find expired leases.
CREATE INDEX idx_inbox_stale
    ON inbox_messages (locked_until)
    WHERE status = 'processing';

-- Operators: inspect failures and quarantined messages.
CREATE INDEX idx_inbox_attention
    ON inbox_messages (source, received_at)
    WHERE status IN ('failed', 'quarantined');