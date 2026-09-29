CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE IF NOT EXISTS users(
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(256) NOT NULL UNIQUE,
    email VARCHAR(256) NOT NULL UNIQUE,
    full_name varchar(256) NOT NULL,
    password_hash varchar(256) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TRIGGER  user_updated_at
BEFORE UPDATE ON USERS 
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE IF NOT EXISTS tenants(
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    name varchar(256),
    slug varchar(256) NOT NULL UNIQUE,
    status BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TRIGGER tenants_updated_at 
BEFORE UPDATE ON tenants
FOR EACH ROW 
EXECUTE FUNCTION set_updated_at();

CREATE TABLE IF NOT EXISTS roles(
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(256),
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS memberships(
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    role_id UUID NOT NULL,
    created_at TIMESTAMPTZ,
    -- constraints
    CONSTRAINT membership_user_id  FOREIGN KEY (user_id) REFERENCES users(id),
    CONSTRAINT membership_tenant_id FOREIGN KEY (tenant_id) REFERENCES tenants(id),
    CONSTRAINT membership_role_id FOREIGN KEY (role_id) REFERENCES roles(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_membership_user_id_tenant_id_role_id_unique on 
memberships(user_id,tenant_id,role_id);