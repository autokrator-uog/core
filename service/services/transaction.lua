bus:register("transaction")

bus:add_event_listener("deposit", function(event_type, consistency_key, correlation_id, data)
    bus:debug(event_type)
    bus:debug(consistency_key)
    bus:debug(correlation_id)
    bus:debug(data)
end)

bus:add_receipt_listener("deposit", function(event_type, consistency_key, correlation_id, data)
    bus:debug(redis:get("test"))
    bus:debug(event_type)
    bus:debug(consistency_key)
    bus:debug(correlation_id)
    bus:debug(data)
end)

bus:add_route("/", "POST", function(method, route, data)
    bus:debug(method)
    bus:debug(route)
    bus:debug(data)

    redis:set("test", data)

    bus:send("deposit", "acc-234", false, nil, { blue = "green" })

    return { hello = "world" }
end)
