function new_event(event)
    info("received event in lua")
    send([[{
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
    }]])
end

function http(method, uri, event)
    info("http handler called")
    debug(method)
    debug(uri)
    return [[{ "orange": "green" }]]
end

function receipt()
end

register("transaction", {"deposit", "withdrawal"}, new_event, receipt, http)
