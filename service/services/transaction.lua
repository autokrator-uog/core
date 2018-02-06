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
    bus:send([[
    {
        "message_type": "new",
        "events": [
            {
                "consistency": {
                    "key": "acc-234",
                    "value": 0
                },
                "correlation_id": 28374938,
                "event_type": "deposit",
                "data": {
                    "blue": "green"
                }
            }
        ]
    }
    ]])
end)

bus:persist("orange", [[ { "blue": "green" } ]])
bus:debug(bus:query("orange"))
