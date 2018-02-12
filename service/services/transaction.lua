bus:register("transaction")

local TRANS_PREFIX = "trans-"
local ID_KEY = "__current_id"

-- We need to keep track of transaction IDs. If there isn't a ID already stored in Redis,
-- then store 0.
if not redis:get(ID_KEY) then
    -- For now, we can only get/set tables from redis.
    redis:set(ID_KEY, { id = 0 })
end

bus:add_event_listener("AcceptedTransaction", function(event_type, key, correlation, data)
    log:debug("received accepted transaction event")
    local transaction = redis:get(key)
    -- Update the status of the transaction when we see it was accepted.
    if transaction then
        transaction.status = "accepted"
        redis:set(key, transaction)
    else
        log:warn("received accepted transaction event that is not in redis")
    end
end)

bus:add_event_listener("RejectedTransaction", function(event_type, key, correlation, data)
    log:debug("received rejected transaction event")
    local transaction = redis:get(key)
    -- Update the status of the transaction when we see it was rejected.
    if transaction then
        transaction.status = "rejected"
        redis:set(key, transaction)
    else
        log:warn("received rejected transaction event that is not in redis")
    end
end)

bus:add_rebuild_handler("PendingTransaction", function(event_type, key, correlation, data)
    local transaction = redis:get(key)
    -- If this transaction is not already in redis (it could be if we got the accepted or rejected
    -- transaction first).
    if not transaction then
        local status = { status = "created" }
        for k, v in pairs(status) do data[k] = v end
        redis:set(key, data)
    end
end)

bus:add_rebuild_handler("AcceptedTransaction", function(event_type, key, correlation, data)
    local transaction = redis:get(key)
    -- If this transaction is not already in redis (it could be if we got the accepted or rejected
    -- transaction first).
    if not transaction then
        local status = { status = "accepted" }
        for k, v in pairs(status) do data[k] = v end
        redis:set(key, data)
    else
        transaction.status = "accepted"
        redis:set(key, transaction)
    end
end)

bus:add_rebuild_handler("RejectedTransaction", function(event_type, key, correlation, data)
    local transaction = redis:get(key)

    -- If this transaction is not already in redis (it could be if we got the accepted or rejected
    -- transaction first).
    if not transaction then
        local status = { status = "rejected" }
        for k, v in pairs(status) do data[k] = v end
        redis:set(key, data)
    else
        transaction.status = "rejected"
        redis:set(key, transaction)
    end
end)

-- We only need a receipt listener for event types that we send.
bus:add_receipt_listener("PendingTransaction", function(status, event_type, key, correlation, data)
    log:debug("received pending transaction receipt")

    local transaction = redis:get(key)

    if transaction then
        -- If this transaction was inconsistent (it shouldn't be, we use implicit consistency),
        -- then resend it.
        if status == "inconsistent" then

            -- Resend the event.
            bus:send("PendingTransaction", key, true, nil, data)

        -- If the transaction has not already been accepted/rejected then
        -- we update the status to 'pending'.
        elseif transaction.status == "created" then
            transaction.status = "pending"
            redis:set(key, transaction)
        end
    else
        log:warn("received receipt for transaction that is not in redis")
    end
end)

-- /createTransaction produces an `PendingTransaction` event.
bus:add_route("/createTransaction", "POST", function(method, route, args, data)
    log:debug("received create transaction request")

    -- Get the next ID.
    local last_id = redis:get(ID_KEY)
    local id = last_id.id + 1
    redis:set(ID_KEY, { id = next_id })

    -- While this should be exactly what is in `data` from the request, by constructing this
    -- ourselves, this will error if the wrong fields are provided and a internal server error
    -- response will be sent.
    local event_data = {
        id = id,
        fromAccountId = data.fromAccountId,
        toAccountId = data.toAccountId,
        amount = data.amount,
    }

    -- Set our consistency key to be "trans-X" where X is the ID. It's useful to add a prefix
    -- because then we can filter on the key from Redis in other operations.
    local key = TRANS_PREFIX .. id

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

    return { transactionId = id }
end)

-- /transactions returns the status of a transaction.
bus:add_route("/transaction/{id}", "GET", function(method, route, args, data)
    log:debug("received transaction request")

    -- Find the ID of the transaction that will have been persisted.
    local key = TRANS_PREFIX .. args.id

    -- Get the information we have stored about this transaction.
    local data = redis:get(key)

    if data then
        -- Return the data we have.
        return data
    else
        -- Return an error if we don't have data.
        return { error = "could not find transaction with id: " .. args.id }
    end
end)
