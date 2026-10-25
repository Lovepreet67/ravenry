use redis::{Script, aio::ConnectionManager};
use serde_json::to_string;
use uuid::Uuid;

use crate::platform::projection::rebuild::TenantProjection;

use iam_contracts::keys::tenant_projection_keys;

const REPLACE_PROJECTION_SCRIPT: &str = r#"
local index_key = KEYS[1]
local raw = ARGV[1]

-- Delete all keys owned by the previous projection.
local old_keys = redis.call('SMEMBERS', index_key)
for _, key in ipairs(old_keys) do
    redis.call('DEL', key)
end
redis.call('DEL', index_key)

-- An empty payload means the tenant was deleted.
if raw == '' then
    return 1
end

local projection = cjson.decode(raw)
local tenant = projection.tenant
local tenant_id = tenant.id

local meta_key = 'v1:tenant:' .. tenant_id .. ':meta'
local topics_key = 'v1:tenant:' .. tenant_id .. ':topics'

-- Register every new key so the next replacement can clean it up.
redis.call('SADD', index_key, meta_key)
redis.call('SADD', index_key, topics_key)

redis.call(
    'HSET', meta_key,
    'status', tenant.is_active and 'active' or 'inactive',
    'slug', tenant.slug,
    'version', '1'
)

-- Only active topics should resolve for inbound publishing.
for _, topic in ipairs(projection.topics) do
    if topic.is_active then
        redis.call('HSET', topics_key, topic.path, topic.id)

        if #topic.destination_ids > 0 then
            local sub_key =
                'v1:tenant:' .. tenant_id .. ':sub:' .. topic.id

            redis.call('SADD', index_key, sub_key)

            for _, destination_id in ipairs(topic.destination_ids) do
                redis.call('SADD', sub_key, destination_id)
            end
        end
    end
end

-- Project destination configuration.
for _, destination in ipairs(projection.destinations) do
    local dest_key =
        'v1:tenant:' .. tenant_id .. ':dest:' .. destination.id

    redis.call('SADD', index_key, dest_key)

    redis.call(
        'HSET', dest_key,
        'name', destination.name,
        'url', destination.destination_url,
        'kind', destination.kind,
        'status', destination.is_active and 'active' or 'inactive',
        'timeout', tostring(destination.delivery_timeout_seconds),
        'version', '1'
    )
end

return 1
"#;

/// Errors returned while replacing a tenant's Redis projection.
#[derive(Debug)]
pub enum ProjectionWriteError {
    Redis(redis::RedisError),
    Serialization(serde_json::Error),
}

impl std::fmt::Display for ProjectionWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Redis(error) => {
                return write!(f, "Redis projection write failed: {error}");
            }
            Self::Serialization(error) => {
                return write!(f, "Projection serialization failed: {error}");
            }
        }
    }
}

impl std::error::Error for ProjectionWriteError {}

impl From<redis::RedisError> for ProjectionWriteError {
    fn from(error: redis::RedisError) -> Self {
        Self::Redis(error)
    }
}

impl From<serde_json::Error> for ProjectionWriteError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

/// Atomically replace a tenant's Redis projection.
///
/// Pass `None` when the tenant no longer exists in PostgreSQL.
pub async fn write_projection(
    connection: &mut ConnectionManager,
    tenant_id: Uuid,
    projection: Option<&TenantProjection>,
) -> Result<(), ProjectionWriteError> {
    // Validate that the projection belongs to the requested tenant.
    if let Some(projection) = projection {
        if projection.tenant.id != tenant_id {
            return Err(ProjectionWriteError::Serialization(serde_json::Error::io(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Projection tenant ID does not match the requested tenant",
                ),
            )));
        }
    }

    let payload = match projection {
        Some(projection) => to_string(projection)?,
        None => String::new(),
    };

    let index_key = tenant_projection_keys(tenant_id);

    let script = Script::new(REPLACE_PROJECTION_SCRIPT);

    let _: i32 = script
        .key(index_key)
        .arg(payload)
        .invoke_async(connection)
        .await?;

    Ok(())
}
