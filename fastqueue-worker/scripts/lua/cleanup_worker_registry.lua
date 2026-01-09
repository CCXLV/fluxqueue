---@diagnostic disable: undefined-global

-- KEYS[1] = queue name

redis.call("DEL", KEYS[1])
return redis.call("SCARD", KEYS[1])
