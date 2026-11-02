
-- KEYS[1] = destination retry sorted set
--
-- ARGV[1] = current timestamp in milliseconds
-- ARGV[2] = maximum number of IDs to return
-- ARGV[3] = delivery state key prefix
--
-- Returns a list of due delivery IDs.
-- The actual exclusive acquisition happens in acquire_delivery.lua.

local now = tonumber(ARGV[1])
local limit = tonumber(ARGV[2])
local state_prefix = ARGV[3]

if not now or not limit or limit < 1 then
    return redis.error_reply("invalid retry claim arguments")
end

if not state_prefix or state_prefix == "" then
    return redis.error_reply("state key prefix is required")
end

local due = redis.call(
    "ZRANGEBYSCORE",
    KEYS[1],
    "-inf",
    now,
    "LIMIT",
    0,
    limit
)

local claimed = {}

for _, delivery_id in ipairs(due) do
    local state_key = state_prefix .. delivery_id .. ":state"

    local status = redis.call("HGET", state_key, "status")

    if status == "retry_scheduled" then
        table.insert(claimed, delivery_id)
    elseif status ~= "processing" then
        redis.call("ZREM", KEYS[1], delivery_id)
    end
end

return claimed
