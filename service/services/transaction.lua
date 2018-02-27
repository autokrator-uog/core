bus:register("transaction")

local PREFIX = "trans-"
local ID_KEY = "__transaction_id"

function handle_accepted_or_rejected(transaction_status)
    -- This returns the function that is expected as the argument to add_event_listener but
    -- with the correct status.
    return function(event_type, key, correlation, data)
        log:debug("received " .. event_type .. " event")
        local transaction = redis:get(key)
        -- Update the status of the transaction when we see it was accepted.
        if transaction then
            transaction.status = transaction_status
            redis:set(key, transaction)
        else
            log:warn("received " .. event_type .. " event that is not in redis")
        end
    end
end

-- Handle these in a generic way.
bus:add_event_listener("AcceptedTransaction", handle_accepted_or_rejected("accepted"))
bus:add_event_listener("RejectedTransaction", handle_accepted_or_rejected("rejected"))

bus:add_rebuild_handler("PendingTransaction", function(event_type, key, correlation, data)
    log:debug("received " .. event_type .. " rebuild")
    -- Rebuild the ID if we can.
    redis:incr(ID_KEY)

    local transaction = redis:get(key)
    -- If this transaction is not already in redis (it could be if we got the accepted or rejected
    -- transaction first).
    if not transaction then
        local status = { status = "created" }
        for k, v in pairs(status) do data[k] = v end
        redis:set(key, data)
    end
end)

function rebuild_accepted_or_rejected(transaction_status)
    -- This returns the function that is expected as the argument to add_rebuild_handler but
    -- with the correct status.
    return function(event_type, key, correlation, data)
        log:debug("received " .. event_type .. " rebuild")

        local transaction = redis:get(key)
        -- If this transaction is not already in redis.
        if not transaction then
            local status = { status = transaction_status }
            for k, v in pairs(status) do data[k] = v end
            redis:set(key, data)
        -- If the transaction does exist, just update the status.
        else
            transaction.status = transaction_status
            redis:set(key, transaction)
        end
    end
end

-- Handle these in a generic way.
bus:add_rebuild_handler("AcceptedTransaction", rebuild_accepted_or_rejected("accepted"))
bus:add_rebuild_handler("RejectedTransaction", rebuild_accepted_or_rejected("rejected"))

-- We only need a receipt listener for event types that we send.
bus:add_receipt_listener("PendingTransaction", function(status, event_type, key, correlation, data)
    log:debug("received " .. event_type .. " receipt")
    local transaction = redis:get(key)

    if transaction then
        -- If this transaction was inconsistent (it shouldn't be, we use implicit consistency),
        -- then resend it.
        if status == "inconsistent" then
            -- Resend the event.
            bus:send(event_type, key, false, correlation, data)
        -- If the transaction has not already been accepted/rejected then
        -- we update the status to 'pending'.
        elseif transaction.status == "created" then
            transaction.status = "pending"
            redis:set(key, transaction)
        end
    else
        log:warn("received receipt for " .. event_type .. " that is not in redis")
    end
end)

-- /createTransaction produces an `PendingTransaction` event.
bus:add_route("/createTransaction", "POST", function(method, route, args, data)
    log:debug("received " .. route .. " request")
    -- Get the next ID.
    local next_id = redis:incr(ID_KEY)

    -- While this should be exactly what is in `data` from the request, by constructing this
    -- ourselves, this will error if the wrong fields are provided and a internal server error
    -- response will be sent.
    local event_data = {
        id = next_id,
        fromAccountId = data.fromAccountId,
        toAccountId = data.toAccountId,
        amount = data.amount,
    }

    -- Set our consistency key to be "trans-X" where X is the ID. It's useful to add a prefix
    -- because then we can filter on the key from Redis in other operations.
    local key = PREFIX .. next_id

    -- Store information about this new transaction.
    -- We create a new table with the extra information that we will combine with `event_data`
    -- and then persist.
    local transaction = { status = "created" }
    for k, v in pairs(event_data) do transaction[k] = v end
    redis:set(key, transaction)

    -- Send a implicit consistency - pending transactions don't depend on ordering, only
    -- the accepted/rejected transactions do.
    local implicit = true
    -- This event is not correlated with any others.
    local consistency = nil

    -- Send the new event.
    bus:send("PendingTransaction", key, implicit, consistency, event_data)

    return HTTP_CREATED, { transactionId = next_id }
end)

-- /transactions returns the status of a transaction.
bus:add_route("/transaction/{id}", "GET", function(method, route, args, data)
    log:debug("received " .. route .. " request")
    -- Get the information we have stored about this transaction.
    local data = redis:get(PREFIX .. args.id)

    if data then
        -- Return the data we have.
        return HTTP_OK, data
    else
        -- Return an error if we don't have data.
        return HTTP_NOT_FOUND, { error = "could not find transaction with id: " .. args.id }
    end
end)
