-- archive.lua
function generate_manifest(message)
    local manifest = [[
version = 2

[meta]
title = "Dynamic Example"
author = "Lua"
web_archive = false
dynamic_archive = true

[external_pages]
urls = ["https://wanker.nz/assets/img/hon-jacinda-ardern-00.webp"]
]]
    return manifest
end