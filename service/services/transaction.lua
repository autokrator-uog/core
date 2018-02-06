bus:register("transaction")

bus:add_event_listener("pendingtransaction", function(event)
    bus:debug(event)
end)

bus:add_event_listener("deposit", function(event)
    bus:debug(event)
end)

bus:add_event_listener("withdrawal", function(event)
    bus:debug(event)
end)

bus:add_receipt_listener("pendingtransaction", function(event)
    bus:debug(event)
end)

bus:add_route("/", "POST", function(data)
    bus:debug(data)
    bus:send("deposit", "acc-234", false, nil, { blue = "green" })
end)
