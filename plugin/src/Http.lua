local HttpService = game:GetService("HttpService")

local Http = {}

local function decodeBody(body)
    if body == nil or body == "" then
        return nil
    end
    local ok, decoded = pcall(function()
        return HttpService:JSONDecode(body)
    end)
    if not ok then
        return nil, decoded
    end
    return decoded
end

function Http.request(method, url, body)
    local request = {
        Url = url,
        Method = method,
        Headers = {
            ["Content-Type"] = "application/json"
        }
    }

    if body ~= nil then
        request.Body = HttpService:JSONEncode(body)
    end

    local ok, response = pcall(function()
        return HttpService:RequestAsync(request)
    end)
    if not ok then
        return nil, tostring(response)
    end

    local decoded, decodeErr = decodeBody(response.Body)
    if not response.Success then
        return nil, string.format("HTTP %s: %s", tostring(response.StatusCode), response.Body or "")
    end
    if decodeErr then
        return nil, tostring(decodeErr)
    end
    return decoded
end

function Http.get(url)
    return Http.request("GET", url)
end

function Http.post(url, body)
    return Http.request("POST", url, body)
end

return Http
