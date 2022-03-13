local function gfind(str, pattern)
    local last = 0;
    return function()
        local s, e = str:find(pattern, last + 1);
        if not(s) then return nil end;
        last = e;
        return s, e;
    end
end

local str_patterns = {
    "[\"'](.-)[^\\][\"']"
}

local x = [[
    local x = "has\"as\"fdfg"
]]

x:gsub(str_patterns[1], print)
print("ok")
