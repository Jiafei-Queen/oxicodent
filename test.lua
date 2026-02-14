local handle = io.popen("java -version")
local result = handle:read("*l")
local version = result:match('.+"[%d%._-]+".+')
print(version)