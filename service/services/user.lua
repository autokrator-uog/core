bus:register("user")
local USER_PREFIX = "user-"
local ACCOUNT_PREFIX = "creation-"
local ID_KEY = "__request_id"

-- Listen for when an account is created, add it to its corresponding user.
bus:add_event_listener("AccountCreated", function(event_type, key, correlation, data, ts)
    log:debug("received account created event")

    local request_key = ACCOUNT_PREFIX .. data.request_id
    local account = redis:get(request_key)
    if not account then
        -- If the account is not in the store, add it in a set it as created.
        account = { status = "created" }
        for k, v in pairs(data) do account[k] = v end
    else
        -- Else set status to created
        account.status = "created"
    end
    redis:set(request_key, account)

    -- Add the new account to it's respective user.
    local user_key = USER_PREFIX .. account.username
    user = redis:get(user_key)
    if user then
        table.insert(user.accounts,data.acc_id)
        redis:set(user_key, user)
    else
        log:warn("received account for a user that is not yet in redis")
    end
end)

bus:add_rebuild_handler("UserCreated", function(event_type, key, correlation, data, ts)
    local user = redis:get(key)
    if not user then
        user = { accounts = {} }
        for k, v in pairs(data) do user[k] = v end
        redis:set(key, user)
    else
        log:warn("tried to rebuild user that already exists, how did this happen?")
    end
end)

bus:add_rebuild_handler("AccountCreated", function(event_type, key, correlation, data, ts)
    log:debug("received account created event")
    local request_key = ACCOUNT_PREFIX .. data.request_id
    local account = redis:get(request_key)
    if not account then
        -- If the account is not in the store, add it in a set it as created.
        account = { status = "created" }
        for k, v in pairs(data) do account[k] = v end
    else
        -- Else set status to created
        account.status = "created"
    end
    redis:set(request_key, account)

    -- Add the new account to it's respective user.
    local user_key = USER_PREFIX .. account.username
    user = redis:get(user_key)
    if user then
        table.insert(user.accounts,data.acc_id)
        redis:set(user_key, user)
    else
        log:warn("received account for a user that is not yet in redis")
    end
end)

bus:add_rebuild_handler("AccountCreationRequest", function(event_type, key, correlation, data, ts)
    redis:incr(ID_KEY)

    local account = redis:get(key)
    -- If account exists in redis, we have already received AccountCreated, so ignore.
    if not account then
        account = { status = "pending" }
        for k, v in pairs(data) do account[k] = v end
        redis:set(key, account)
    end
end)

function handle_receipt(status, event_type, key, correlation, data)
    log:debug("received " .. event_type .. " receipt")
    -- Resend the event.
    if status == "inconsistent" then
        bus:send(event_type, key, false, correlation, data)
    end
end

bus:add_receipt_listener("AccountCreationRequest", handle_receipt)
bus:add_receipt_listener("UserCreated", handle_receipt)

bus:add_route("/user", "POST", function(method, route, args, data)
    log:debug("recieved create user request")
    local key = USER_PREFIX .. data.username
    if redis:get(key) then
        log:debug("create user failed, username already taken")
        return HTTP_BAD_REQUEST, { status = "failed" }
    end

    -- Passwords are not implemented at current.
    local event_data = { username = data.username, password = "" }
    local user = { accounts = {} }
    for k, v in pairs(event_data) do user[k] = v end
    redis:set(key, user)

    bus:send("UserCreated", key, true, nil, event_data)
    return HTTP_OK, { status = "created" }
end)

bus:add_route("/user/{username}", "GET", function(method, route, args, data)
    log:debug("received get accounts request")

    key = USER_PREFIX .. args.username
    local user = redis:get(key)

    if not user then
        return HTTP_NOT_FOUND, { error = "user not found: ".. args.username }
    end

    return HTTP_OK, { accounts = user.accounts }
end)

bus:add_route("/user/{username}", "POST", function(method, route, args, data)
    log:debug("recieved create account request")
    local next_id = redis:incr(ID_KEY)

    local key = ACCOUNT_PREFIX .. next_id

    event_data = { request_id = next_id, username = args.username }
    local account = { status = "pending" }
    for k, v in pairs(event_data) do account[k] = v end
    redis:set(key, account)

    bus:send("AccountCreationRequest", key, true, nil, event_data)
    return HTTP_CREATED, { status = "pending", request_id = next_id }
end)

bus:add_route("/user/{username}/{request_id}", "GET", function(method, route, args, data)
    local key = ACCOUNT_PREFIX .. args.request_id
    request = redis:get(key)

    if not request then
        log:debug("recieved request with unknown id")
        return HTTP_NOT_FOUND, { status = "unknown" }
    end

    return HTTP_OK, { status = request.status }
end)
