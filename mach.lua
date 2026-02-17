#!/usr/bin/env lua
local PATH = "src/config_manager.rs"
local MELCHIOR_PLACEHOLDER = '"MELCHIOR PROMPT FILED BY `mach.lua`"'
local CASPER_I_PLACEHOLDER = '"CASPER I PROMPT FILED BY `mach.lua`"'
local CASPER_II_PLACEHOLDER = '"CASPER II PROMPT FILED BY `mach.lua`"'

local args = {...}
local cmd = args[1] or "cargo build --release"

local function safe_content(filename)
     local f = io.open(filename, "rb")
     local content = f:read("*a")
     f:close()
     return content:gsub("%%", "%%%%")
end

local melchior = 'r##"' .. safe_content("MELCHIOR_PROMPT.md") .. '"##'
local casper_one = 'r##"' .. safe_content("CASPER_I_PROMPT.md") .. '"##'
local casper_two = 'r##"' .. safe_content("CASPER_II_PROMPT.md") .. '"##'

local f_src = io.open(PATH, "rb")
local src = f_src:read("*a")
f_src:close()

local replaced = src:gsub(MELCHIOR_PLACEHOLDER, melchior)
                    :gsub(CASPER_I_PLACEHOLDER, casper_one)
                    :gsub(CASPER_II_PLACEHOLDER, casper_two)

local f_out = io.open(PATH, "wb")
f_out:write(replaced)
f_out:flush()
f_out:close()

print("Status: Building Oxicodent...")
os.execute(cmd)

local f_restore = io.open(PATH, "wb")
f_restore:write(src)
f_restore:close()