---@diagnostic disable: undefined-global

-- KEYS[1] = queue name
-- ARGV[1] = worker id

redis.call("SADD", KEYS[1], ARGV[1])
return redis.call("SCARD", KEYS[1])
