#!/usr/bin/env lua
local FILENAME = ".oxicodent-history.yaml"

---- [ History 分块 ] ----
local content = io.open(FILENAME):read("*a"):gsub("\\n", "\n")

local blocks = {}
local block = ""
for line in content:gmatch("([^\n]+)") do
    if line ~= "---" then
        block = block..line.."\n"
    else
        table.insert(blocks, block)
        block = ""
    end
end

while true do
    print(string.format("已读取 <>：", FILENAME))
    io.write("idx > ")

    local ok, input = pcall(io.read)
    if not ok then
        break
    end

    local idx = tonumber(input)
    if idx and idx <= #blocks then
        print(string.format("\n-------- [ 第 %d 块 ] --------", idx))
        print(blocks[idx])
    end
end
