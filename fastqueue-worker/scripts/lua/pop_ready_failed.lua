---@diagnostic disable: undefined-global

-- KEYS[1] = failed zset
-- ARGV[1] = now timestamp

local items = redis.call(
  'ZRANGEBYSCORE',
  KEYS[1],
  '-inf',
  ARGV[1],
  'LIMIT',
  0,
  1
)

if #items == 0 then
  return nil
end

redis.call('ZREM', KEYS[1], items[1])
return items[1]
