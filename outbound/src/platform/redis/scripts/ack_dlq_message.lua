-- KEYS[1] = destination DLQ pending hash
-- KEYS[2] = global pending-DLQ destination set
--
-- ARGV[1] = delivery_id
-- ARGV[2] = tenant_id:destination_id registry member

redis.call("HDEL", KEYS[1], ARGV[1])

if redis.call("HLEN", KEYS[1]) == 0 then
    redis.call("SREM", KEYS[2], ARGV[2])
end

return 1