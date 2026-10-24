    CREATE TYPE  destination_kind  AS ENUM (
    'http'
);
-- Add up migration script here
CREATE TABLE destinations(
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    kind destination_kind NOT NULL,
    destination_url VARCHAR(1023) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    delivery_timeout_seconds integer NOT NULL,
    tenant_id UUID NOT NULL,
    created_by VARCHAR(255) NOT NULL,
    last_updated_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- constraints
    CONSTRAINT destination_tenant_id FOREIGN KEY (tenant_id) REFERENCES tenants(id)
);

CREATE TRIGGER destination_updated_at 
BEFORE UPDATE ON destinations
FOR EACH ROW 
EXECUTE FUNCTION set_updated_at();

CREATE TABLE topics (
    id UUID PRIMARY KEY default uuid_generate_v4(),
    name varchar(255) NOT NULL,
    description TEXT,
    path varchar(511) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    tenant_id UUID NOT NULL,
    created_by varchar(255) NOT NULL,
    last_updated_by varchar(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    --constraints 
    CONSTRAINT topic_tenant_id FOREIGN KEY (tenant_id) REFERENCES tenants(id)
);
CREATE TRIGGER topic_updated_at 
BEFORE UPDATE ON topics 
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE topics_junc_destinations (
    destination_id UUID NOT NULL,
    topic_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    --constratins
    CONSTRAINT destination_id FOREIGN KEY(destination_id) REFERENCES destinations(id),
    CONSTRAINT topic_id FOREIGN KEY(topic_id) REFERENCES topics(id),
    PRIMARY KEY (topic_id, destination_id)
)