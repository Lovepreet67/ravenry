
-- KEYS[1] = delivery state hash
-- KEYS[2] = destination retry sorted set
--
-- ARGV[1] = current timestamp in milliseconds
-- ARGV[2] = lease expiry timestamp in milliseconds
-- ARGV[3] = attempt_id
-- ARGV[4] = delivery_id
--
-- Returns:
--  >0 = acquired, value is attempt number
--   0 = busy
--  -1 = missing
--  -2 = already succeeded
--  -3 = already failed
--  -4 = invalid state

local status = redis.call("HGET", KEYS[1], "status")

if not status then
    return -1
end

if status == "succeeded" then
    return -2
end

if status == "failed" then
    return -3
end

local now = tonumber(ARGV[1])
local lease_until = tonumber(ARGV[2])

if not now or not lease_until or lease_until <= now then
    return redis.error_reply("invalid lease timestamps")
end

if not ARGV[3] or ARGV[3] == "" then
    return redis.error_reply("attempt_id is required")
end

if not ARGV[4] or ARGV[4] == "" then
    return redis.error_reply("delivery_id is required")
end

if status == "processing" then
    local current_lease = tonumber(
        redis.call("HGET", KEYS[1], "lease_until") or "0"
    )

    if not current_lease then
        return -4
    end

    if current_lease > now then
        return 0
    end
elseif status ~= "pending" and status ~= "retry_scheduled" then
    return -4
end

local attempts = tonumber(redis.call("HGET", KEYS[1], "attempts") or "0")

if not attempts or attempts < 0 or attempts ~= math.floor(attempts) then
    return -4
end

local attempt = redis.call("HINCRBY", KEYS[1], "attempts", 1)

redis.call(
    "HSET", KEYS[1],
    "status", "processing",
    "attempt_id", ARGV[3],
    "lease_until", lease_until
)

redis.call("HDEL", KEYS[1], "next_attempt_at")
redis.call("ZREM", KEYS[2], ARGV[4])

return attempt
