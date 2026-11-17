#Requires -Version 5.1
<#
.SYNOPSIS
    Start the frontend dev server (http://localhost:1420).
#>
[CmdletBinding()]
param()

$root = Split-Path -Parent $PSScriptRoot
$frontend = Join-Path $root "frontend"
if (-not (Test-Path (Join-Path $frontend "node_modules"))) {
    Push-Location $frontend
    try { npm install } finally { Pop-Location }
}
Push-Location $frontend
try {
    npm run dev
} finally {
    Pop-Location
}
