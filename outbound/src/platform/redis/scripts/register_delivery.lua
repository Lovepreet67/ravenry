
-- KEYS[1] = delivery state hash
-- KEYS[2] = serialized delivery request
--
-- ARGV[1] = event_id
-- ARGV[2] = topic_id
-- ARGV[3] = created_at timestamp in milliseconds
-- ARGV[4] = serialized DeliveryRequest
--
-- Returns:
--   1 = registered
--   0 = already registered
--  -1 = inconsistent state (request exists without state)

local state_exists = redis.call("EXISTS", KEYS[1])
local request_exists = redis.call("EXISTS", KEYS[2])

if state_exists == 1 then
    return 0
end

if request_exists == 1 then
    return -1
end

redis.call(
    "HSET", KEYS[1],
    "status", "pending",
    "attempts", 0,
    "event_id", ARGV[1],
    "topic_id", ARGV[2],
    "created_at", ARGV[3]
)

redis.call("SET", KEYS[2], ARGV[4])

return 1
