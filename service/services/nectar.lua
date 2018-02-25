bus:register("nectar")

local PREFIX = "nectar-"
local ID_KEY = "__nectar_id"

-- We need to keep track of nectar account IDs. If there isn't a ID already stored in Redis,
-- then store 0.
if not redis:get(ID_KEY) then
    -- For now, we can only get/set tables from redis.
    redis:set(ID_KEY, { id = 0 })
end

-- When £100 or more is spent, we debit the nectar balance by
-- 10% of this amount.
local GET_THRESHOLD = 100.0
local GET_AMOUNT = 0.1

-- If greater than £25.0, if the nectar account has sufficient balance for the amount of
-- the transaction, then 10% of the amount is refunded as a debit.
local USE_THRESHOLD = 25.0
local USE_AMOUNT = 0.1

bus:add_event_listener("AcceptedTransaction", function(event_type, key, correlation, data)
    local nectar_key = PREFIX .. data.from_account_id
    local account = redis:get(nectar_key)

    if data.amount > GET_THRESHOLD then
        log:info("received " .. event_type .. " and debiting nectar account")
        -- Credit nectar equal to GET_AMOUNT% of the transaction value.
        bus:send("NectarCredit", nectar_key, false, correlation, {
            amount = data.amount * GET_AMOUNT
        })
    end

    -- Only give cashback if this account has enough points.
    if data.amount > USE_AMOUNT and account and account.balance > data.amount then
        log:info("received " .. event_type .. " and crediting nectar account")
        -- Take the points away equal to the value of the transaction.
        bus:send("NectarDebit", nectar_key, false, correlation, { amount = data.amount })
        -- Credit GBP equal to USE_AMOUNT% of the transaction value.
        local spent = "Spent " .. data.amount .. " points"
        local remaining = account.balance - data.amount .. " remaining."
        bus:send("ConfirmedCredit", "acc-" .. data.from_account_id, true, correlation, {
            id = data.from_account_id,
            amount = data.amount * USE_AMOUNT,
            note = "Nectar cashback! " .. spent .. ", " .. remaining,
        })
    end
end)

bus:add_event_listener("NectarDebit", function(event_type, key, correlation, data)
    local details = redis:get(key)
    if details then
        log:info("received " .. event_type .. " and updating nectar account")
        -- If the account already exists, then remove to the balance.
        details.balance = details.balance - data.amount
        redis:set(key, details)
    else
        -- If the account doesn't exist, then we can't take balance away.
        log:warn("received " .. event_type .. " without nectar account")
    end
end)

bus:add_event_listener("NectarCredit", function(event_type, key, correlation, data)
    local details = redis:get(key)
    if details then
        log:info("received " .. event_type .. " and updating nectar account")
        -- If the account already exists, then add to the balance.
        details.balance = details.balance + data.amount
        redis:set(key, details)
    else
        log:info("received " .. event_type .. " and creating nectar account")
        -- If the account doesn't exist, create it with the new balance.
        local last_id = redis:get(ID_KEY)
        local next_id = last_id.id + 1
        redis:set(ID_KEY, { id = next_id })

        redis:set(key, {
            balance = data.amount,
            id = next_id,
        })
    end
end)

function handle_receipt(status, event_type, key, correlation, data)
    log:debug("received " .. event_type .. " receipt")
    -- Resend the event.
    if status == "inconsistent" then
        bus:send(event_type, key, event_type == "ConfirmedCredit", correlation, data)
    end
end

-- Only handle the receipts for event types that this service sends out.
bus:add_receipt_listener("NectarDebit", handle_receipt)
bus:add_receipt_listener("NectarCredit", handle_receipt)
bus:add_receipt_listener("ConfirmedCredit", handle_receipt)

function handle_balance_change(event_type, key, correlation, data)
    log:debug("received " .. event_type .. " rebuild")
    local account = redis:get(key)
    if account then
        -- Update the balance to add the amount credited. data.amount should be negative if this
        -- is a debit.
        account.balance = account.balance + data.amount

        -- Save these changes.
        redis:set(key, account)
    else
        -- Get the next ID.
        local last_id = redis:get(ID_KEY)
        local next_id = last_id.id + 1
        redis:set(ID_KEY, { id = next_id })

        -- Create an account with this amount.
        redis:set(key, {
            balance = data.amount,
            id = next_id,
        })
    end
end

function rebuild_id(previous_id)
    -- When rebuilding, make sure we keep the ID correct.
    id = redis:get(ID_KEY)
    if previous_event.id > id then
        redis:set(ID_KEY, { id = previous_event.id })
    end
end

bus:add_rebuild_handler("NectarDebit", function(event_type, key, correlation, data)
    rebuild_id(data.id)
    handle_balance_change(event_type, key, correlation, data)
end)
bus:add_rebuild_handler("NectarCredit", function(event_type, key, correlation, data)
    rebuild_id(data.id)

    -- The NectarDebit event has a positive value so negate this so that the same function
    -- can handle both credit and debit balance changes.
    data.amount = -data.amount
    handle_balance_change(event_type, key, correlation, data)
end)

bus:add_route("/balance/{id}", "GET", function(method, route, args, data)
    log:debug("received " .. route .. " request")
    -- Get the account details.
    local key = PREFIX .. args.id
    local account = redis:get(key)
    if account then
        -- Return the balance.
        return HTTP_OK, { balance = account.balance }
    else
        -- Return an error if we don't have data.
        return HTTP_NOT_FOUND, { error = "could not find nectar account with id: " .. args.id }
    end
end)
