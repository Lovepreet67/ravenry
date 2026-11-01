-- outbound/src/platform/redis/scripts/complete_attempt.lua

local status = redis.call("HGET", KEYS[1], "status")
local current_attempt_id = redis.call("HGET", KEYS[1], "attempt_id")

if not status then
    return -1
end

if status ~= "processing" then
    return -2
end

if current_attempt_id ~= ARGV[1] then
    return -3
end

local outcome_json = ARGV[2]
local success = ARGV[3] == "1"
local will_retry = ARGV[4] == "1"
local delivery_id = ARGV[5]
local next_attempt_at = tonumber(ARGV[6])
local dlq_json = ARGV[7]

local new_status

if success then
    new_status = "succeeded"
elseif will_retry then
    if not next_attempt_at then
        return redis.error_reply("next_attempt_at is required for retry")
    end

    new_status = "retry_scheduled"
elseif not success then
    new_status = "failed"
end

redis.call(
    "HSET", KEYS[1],
    "status", new_status,
    "last_outcome", outcome_json,
    "completed_attempt_id", ARGV[1],
    "lease_until", 0
)

if success then
    redis.call("ZREM", KEYS[2], delivery_id)
elseif will_retry then
    redis.call("HSET", KEYS[1], "next_attempt_at", next_attempt_at)
    redis.call("ZADD", KEYS[2], next_attempt_at, delivery_id)
else
    redis.call("HDEL", KEYS[1], "next_attempt_at")
    redis.call("HSET", KEYS[4], delivery_id, dlq_json)
    redis.call("ZREM", KEYS[2], delivery_id)
end

redis.call(
    "XADD", KEYS[3], "*",
    "delivery_id", delivery_id,
    "attempt_id", ARGV[1],
    "payload", outcome_json
)

return 1