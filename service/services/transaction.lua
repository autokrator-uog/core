bus:register("transaction")

bus:add_event_listener("deposit", function(event_type, consistency_key, correlation_id, data)
    log:debug(event_type)
    log:debug(consistency_key)
    log:debug(correlation_id)
    log:debug(data)
end)

bus:add_receipt_listener("deposit", function(status, event_type, consistency_key, correlation_id,
                                             data)
    log:debug(redis:get("test"))
    log:debug(status)
    log:debug(event_type)
    log:debug(consistency_key)
    log:debug(correlation_id)
    log:debug(data)
end)

bus:add_route("/{name}/{id}", "POST", function(method, route, args, data)
    log:debug(args.name)
    log:debug(args.id)
    log:debug(method)
    log:debug(route)
    log:debug(data)

    redis:set("test", data)

    bus:send("deposit", "acc-234", false, nil, { blue = "green" })

    return { hello = "world" }
end)
