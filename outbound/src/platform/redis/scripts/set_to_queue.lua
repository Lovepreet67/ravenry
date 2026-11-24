local items = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1], 'LIMIT', 0, ARGV[2])
if #items > 0 then
    for i = 1, #items do
        redis.call('XADD', KEYS[2], '*', 'payload', items[i])
    end
    redis.call('ZREM', KEYS[1], unpack(items))
end
return #items