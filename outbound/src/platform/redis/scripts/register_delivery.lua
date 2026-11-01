-- register_delivery.lua

if redis.call("EXISTS", KEYS[1]) == 1 then
    return 0
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