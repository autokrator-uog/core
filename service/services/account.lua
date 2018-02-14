bus:register("account")

local PREFIX = "acc-"
local ID_KEY = "__account_id"

-- We need to keep track of account IDs. If there isn't a ID already stored in Redis,
-- then store 0.
if not redis:get(ID_KEY) then
    -- For now, we can only get/set tables from redis.
    redis:set(ID_KEY, { id = 0 })
end

-- Accept or reject pending transactions.
bus:add_event_listener("PendingTransaction", function(event_type, key, correlation, data)
    log:debug("received " .. event_type .. " event")
    local from_acc_key = PREFIX .. data.fromAccountId
    local to_acc_key = PREFIX .. data.toAccountId
    local account = redis:get(from_acc_key)
    if account then
        -- If the user can afford the transaction then go for it.
        if account.balance - data.amount > 0 then
            -- If we accept a transaction, send a accepted transaction event out.
            bus:send("AcceptedTransaction", key, true, correlation, { transaction_id = data.id })

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
            bus:send("RejectedTransaction", key, true, correlation, { transaction_id = data.id })
        end
    else
        log:warn("received " .. event_type .. " that is not in redis")
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
        statement_item_id = 0,
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
bus:add_event_listener("AccountCreationRequest", function(event_type, key, correlation, data)
    log:debug("received " .. event_type .. " event")
    -- Get the next ID.
    local last_id = redis:get(ID_KEY)
    local next_id = last_id.id + 1
    redis:set(ID_KEY, { id = next_id })

    -- Create a new account and send the event out.
    create_account(next_id ,data.request_id, true)
end)

function handle_balance_change(event_type, key, correlation, data)
    log:debug("received " .. event_type .. " event")
    local account = redis:get(key)
    if account then
        -- Update the balance to add the amount credited. data.amount should be negative if this
        -- is a debit.
        account.balance = account.balance + data.amount
        -- Insert a new statement that reflects this change.
        table.insert(account.statements, {
            item = account.statement_item_id + 1,
            amount = data.amount,
            note = data.note
        })
        -- Increment this account's statement item id.
        account.statement_item_id = account.statement_item_id + 1
        -- Save these changes.
        redis:set(key, account)
    else
        log:warn("received " .. event_type .. " event that is not in redis")
    end
end

-- Adjust the balance when a confirmed debit or credit happens.
bus:add_event_listener("ConfirmedCredit", handle_balance_change)
bus:add_event_listener("ConfirmedDebit", function(event_type, key, correlation, data)
    -- The ConfirmedDebit event has a positive value so negate this so that the same function
    -- can handle both credit and debit balance changes.
    data.amount = -data.amount
    handle_balance_change(event_type, key, correlation, data)
end)

function rebuild_id(previous_id)
    -- When rebuilding, make sure we keep the ID correct.
    id = redis:get(ID_KEY)
    if previous_event.id > id then
        redis:set(ID_KEY, { id = previous_event.id })
    end
end

bus:add_rebuild_handler("AccountCreated", function(event_type, key, correlation, data)
    -- Rebuild the ID if we can.
    rebuild_id(data.acc_id)
    -- Create a new account with the same request ID but without the event being created.
    create_account(data.acc_id, data.request_id, false)
end)

bus:add_rebuild_handler("ConfirmedCredit", function(event_type, key, correlation, data)
    -- Rebuild the ID if we can.
    rebuild_id(data.id)
    -- Use the same balance addition function as before - it works either way as it doesn't produce
    -- any events.
    handle_balance_change(event_type, key, correlation, data)
end)
bus:add_rebuild_handler("ConfirmedDebit", function(event_type, key, correlation, data)
    -- Rebuild the ID if we can.
    rebuild_id(data.id)
    -- The ConfirmedDebit event has a positive value so negate this so that the same function
    -- can handle both credit and debit balance changes.
    data.amount = -data.amount
    -- Use the same balance addition function as before - it works either way as it doesn't produce
    -- any events.
    handle_balance_change(event_type, key, correlation, data)
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
