bus:register("transaction")

bus:add_event_listener("deposit", function(event_type, consistency_key, correlation_id, data)
    bus:debug(event_type)
    bus:debug(consistency_key)
    bus:debug(correlation_id)
    bus:debug(data)
end)

bus:add_receipt_listener("deposit", function(event)
    bus:debug(event)
end)

bus:add_route("/", "POST", function(data)
    bus:debug(data)
    bus:send("deposit", "acc-234", false, nil, { blue = "green" })
end)
