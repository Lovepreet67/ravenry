-- acquire_delivery.lua

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

if status == "processing" then
    local current_lease = tonumber(
        redis.call("HGET", KEYS[1], "lease_until") or "0"
    )

    if current_lease > now then
        return 0
    end
elseif status ~= "pending" and status ~= "retry_scheduled" then
    return -4
end

local attempt = redis.call("HINCRBY", KEYS[1], "attempts", 1)

redis.call(
    "HSET", KEYS[1],
    "status", "processing",
    "attempt_id", ARGV[3],
    "lease_until", lease_until
)

redis.call("ZREM", KEYS[2], ARGV[4])

return attempt