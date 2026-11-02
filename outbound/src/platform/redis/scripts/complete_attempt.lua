-- KEYS[1] = delivery state hash
-- KEYS[2] = destination retry sorted set
-- KEYS[3] = DELIVERY_RESULTS stream
-- KEYS[4] = destination DLQ pending hash
-- KEYS[5] = global pending-DLQ destination set
--
-- ARGV[1] = attempt_id
-- ARGV[2] = serialized DeliveryOutcome
-- ARGV[3] = success (0/1)
-- ARGV[4] = will_retry (0/1)
-- ARGV[5] = delivery_id
-- ARGV[6] = next_attempt_at milliseconds
-- ARGV[7] = serialized DlqMessage
-- ARGV[8] = tenant_id:destination_id registry member

local status = redis.call("HGET", KEYS[1], "status")
local current_attempt_id = redis.call("HGET", KEYS[1], "attempt_id")

if not status then return -1 end
if status ~= "processing" then return -2 end
if current_attempt_id ~= ARGV[1] then return -3 end

local success = ARGV[3] == "1"
local will_retry = ARGV[4] == "1"
local delivery_id = ARGV[5]
local next_attempt_at = tonumber(ARGV[6])

if ARGV[3] ~= "0" and ARGV[3] ~= "1" then
    return redis.error_reply("invalid success flag")
end

if ARGV[4] ~= "0" and ARGV[4] ~= "1" then
    return redis.error_reply("invalid will_retry flag")
end

if success and will_retry then
    return redis.error_reply("successful delivery cannot be scheduled for retry")
end

if will_retry and (not next_attempt_at or next_attempt_at <= 0) then
    return redis.error_reply("valid next_attempt_at is required for retry")
end

if not success and not will_retry and ARGV[7] == "" then
    return redis.error_reply("DLQ message is required for terminal failure")
end

local new_status

if success then
    new_status = "succeeded"
elseif will_retry then
    new_status = "retry_scheduled"
else
    new_status = "failed"
end

redis.call(
    "HSET",
    KEYS[1],
    "status", new_status,
    "last_outcome", ARGV[2],
    "completed_attempt_id", ARGV[1],
    "lease_until", 0
)

if success then
    redis.call("ZREM", KEYS[2], delivery_id)
    redis.call("HDEL", KEYS[1], "next_attempt_at")

elseif will_retry then
    redis.call("HSET", KEYS[1], "next_attempt_at", next_attempt_at)
    redis.call("ZADD", KEYS[2], next_attempt_at, delivery_id)

else
    redis.call("HDEL", KEYS[1], "next_attempt_at")
    redis.call("HSET", KEYS[4], delivery_id, ARGV[7])
    redis.call("SADD", KEYS[5], ARGV[8])
    redis.call("ZREM", KEYS[2], delivery_id)
end

redis.call(
    "XADD", KEYS[3], "*",
    "delivery_id", delivery_id,
    "attempt_id", ARGV[1],
    "payload", ARGV[2]
)

return 1