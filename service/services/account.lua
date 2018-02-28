bus:register("account")

local PREFIX = "acc-"
local ID_KEY = "__account_id"

-- Accept or reject pending transactions.
bus:add_event_listener("PendingTransaction", function(event_type, key, correlation, data, ts)
    log:debug("received " .. event_type .. " event")
    local to_acc_key = PREFIX .. data.toAccountId
    local to_account = redis:get(to_acc_key)
    if not to_account then
        log:warn("received " .. event_type .. " for to account that is not in redis")
        -- If recipient account does not exist then send a rejected transaction and return.
        bus:send("RejectedTransaction", key, true, correlation, {
            transaction_id = data.id,
            reason = "Recipient account does not exist", 
        })
        return
    end

    local from_acc_key = PREFIX .. data.fromAccountId
    local from_account = redis:get(from_acc_key)
    if from_account then
        -- If the user can afford the transaction then go for it.
        if from_account.balance - data.amount > 0 then
            -- If we accept a transaction, send a accepted transaction event out.
            bus:send("AcceptedTransaction", key, true, correlation, {
                transaction_id = data.id,
                from_account_id = data.fromAccountId,
                to_account_id = data.toAccountId,
                amount = data.amount,
            })

            -- Also send the debit and credit. Ensure that these are ordered.
            bus:send("ConfirmedCredit", to_acc_key, false, correlation, {
                id = data.toAccountId,
                amount = data.amount,
                note = "Transaction from " .. data.fromAccountId .. " to " .. data.toAccountId,
            })
            bus:send("ConfirmedDebit", from_acc_key, false, correlation, {
                id = data.fromAccountId,
                amount = data.amount,
                note = "Transaction from " .. data.fromAccountId .. " to " .. data.toAccountId,
            })
        -- If the user can't, don't.
        else
            -- If we reject a transaction, just send a rejected transaction event out.
            bus:send("RejectedTransaction", key, true, correlation, {
                transaction_id = data.id,
                reason = "Insufficient Balance", 
            })
        end
    else
        log:warn("received " .. event_type .. " for from account that is not in redis")
        -- If the sending account does not exist then send a rejected transaction.
        bus:send("RejectedTransaction", key, true, correlation, {
            transaction_id = data.id,
            reason = "Sender account does not exist", 
        })
    end
end)

-- Create a new account in Redis.
function create_account(id, request_id, send_event)
    -- Set our consistency key to be "acc-X" where X is the ID. It's useful to add a prefix
    -- because then we can filter on the key from Redis in other operations.
    local key = PREFIX .. id

    -- Create default information for an account.
    local account = {
        id = id,
        balance = 0,
        statements = {},
        request_id = request_id
    }

    -- Save that data.
    redis:set(key, account)

    if send_event then
        -- Send the account created event.
        bus:send("AccountCreated", key, true, correlation, {
            acc_id = id, request_id = request_id
        })
    end
end

-- Handle request for an account creation.
bus:add_event_listener("AccountCreationRequest", function(event_type, key, correlation, data, ts)
    log:debug("received " .. event_type .. " event")
    -- Get the next ID.
    local next_id = redis:incr(ID_KEY);

    -- Create a new account and send the event out.
    create_account(next_id ,data.request_id, true)
end)

function handle_balance_change(event_type, key, correlation, data, ts)
    log:debug("received " .. event_type .. " event")
    local account = redis:get(key)
    if account then
        -- Update the balance to add the amount credited. data.amount should be negative if this
        -- is a debit.
        account.balance = account.balance + data.amount
        -- Insert a new statement that reflects this change.
        table.insert(account.statements, {
            timestamp = ts,
            amount = data.amount,
            note = data.note
        })
        -- Save these changes.
        redis:set(key, account)
    else
        log:warn("received " .. event_type .. " event that is not in redis")
    end
end

-- Adjust the balance when a confirmed debit or credit happens.
bus:add_event_listener("ConfirmedCredit", handle_balance_change)
bus:add_event_listener("ConfirmedDebit", function(event_type, key, correlation, data, ts)
    -- The ConfirmedDebit event has a positive value so negate this so that the same function
    -- can handle both credit and debit balance changes.
    data.amount = -data.amount
    handle_balance_change(event_type, key, correlation, data, ts)
end)

bus:add_rebuild_handler("AccountCreated", function(event_type, key, correlation, data, ts)
    redis:incr(ID_KEY)
    -- Create a new account with the same request ID but without the event being created.
    create_account(data.acc_id, data.request_id, false)
end)

bus:add_rebuild_handler("ConfirmedCredit", function(event_type, key, correlation, data, ts)
    -- Use the same balance addition function as before - it works either way as it doesn't produce
    -- any events.
    handle_balance_change(event_type, key, correlation, data, ts)
end)
bus:add_rebuild_handler("ConfirmedDebit", function(event_type, key, correlation, data, ts)
    -- The ConfirmedDebit event has a positive value so negate this so that the same function
    -- can handle both credit and debit balance changes.
    data.amount = -data.amount
    -- Use the same balance addition function as before - it works either way as it doesn't produce
    -- any events.
    handle_balance_change(event_type, key, correlation, data, ts)
end)

function handle_receipt(status, event_type, key, correlation, data)
    log:debug("received " .. event_type .. " receipt")
    -- Resend the event.
    if status == "inconsistent" then
        bus:send(event_type, key, false, correlation, data)
    end
end

-- Only handle the receipts for event types that this service sends out.
bus:add_receipt_listener("AccountCreated", handle_receipt)
bus:add_receipt_listener("AcceptedTransaction", handle_receipt)
bus:add_receipt_listener("RejectedTransaction", handle_receipt)
bus:add_receipt_listener("ConfirmedCredit", handle_receipt)
bus:add_receipt_listener("ConfirmedDebit", handle_receipt)

-- /account/{id} returns the balance (and other details) of the requested account.
bus:add_route("/account/{id}", "GET", function(method, route, args, data)
    log:debug("received " .. route .. " request")
    -- Get the information we have stored about this account.
    local account = redis:get(PREFIX .. args.id)
    if account then
        -- Return some of the data.
        return HTTP_OK, { id = account.id, balance = account.balance }
    else
        -- Return an error if we don't have data.
        return HTTP_NOT_FOUND, { error = "could not find account with id: " .. args.id }
    end
end)

-- /account/{id}/statement returns a sequence of credits and debits that have happened to the
-- account.
bus:add_route("/account/{id}/statement", "GET", function(method, route, args, data)
    log:debug("received " .. route .. " request")
    -- Get the information we have stored about this account.
    local account = redis:get(PREFIX .. args.id)
    if account then
        -- Return some of the data.
        return HTTP_OK, { id = account.id, statements = account.statements }
    else
        -- Return an error if we don't have data.
        return HTTP_NOT_FOUND, { error = "could not find account with id: " .. args.id }
    end
end)

-- /account/{id}/deposit makes a deposit to an account.
bus:add_route("/account/{id}/deposit", "POST", function(method, route, args, data)
    log:debug("received " .. route .. " request")
    -- Get the information we have stored about this account.
    local key = PREFIX .. args.id
    local account = redis:get(key)
    if account then

        -- Send the credit event.
        bus:send("ConfirmedCredit", key, false, nil, {
            id = account.id,
            amount = data.amount,
            note = "Deposit to " .. account.id,
        })

        -- Return some of the data.
        return HTTP_CREATED, { id = account.id, status = "pending" }
    else
        -- Return an error if we don't have data.
        return HTTP_NOT_FOUND, { error = "could not find account with id: " .. args.id }
    end
end)

-- /account/{id}/withdrawal makes a deposit to an account.
bus:add_route("/account/{id}/withdrawal", "POST", function(method, route, args, data)
    log:debug("received " .. route .. " request")
    -- Get the information we have stored about this account.
    local key = PREFIX .. args.id
    local account = redis:get(key)
    if account then
        -- Check if the withdrawal can be afforded.
        if account.balance - data.amount > 0 then

            -- Send the credit event.
            bus:send("ConfirmedDebit", key, false, nil, {
                id = account.id,
                amount = data.amount,
                note = "Withdrawal from " .. account.id,
            })

            -- Return some of the data.
            return HTTP_CREATED, { id = account.id, status = "pending" }
        else
            -- Return an error if we don't have data.
            return HTTP_BAD_REQUEST, { error = "not enough funds" }
        end
    else
        -- Return an error if we don't have data.
        return HTTP_NOT_FOUND, { error = "could not find account with id: " .. args.id }
    end
end)
