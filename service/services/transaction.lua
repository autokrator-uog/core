function new_event(event)
    info(event)
end

function receipt()
end

register("transaction", {"deposit", "withdrawal"}, new_event, receipt)
