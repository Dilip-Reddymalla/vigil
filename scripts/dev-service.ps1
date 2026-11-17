#Requires -Version 5.1
<#
.SYNOPSIS
    Build & run the Vigil background service (streams structured transition logs).
.EXAMPLE
    .\scripts\dev-service.ps1
    .\scripts\dev-service.ps1 -Args "--release"
#>
[CmdletBinding()]
param([string[]]$Args = @())

$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    cargo run -p vigil-service @Args
} finally {
    Pop-Location
}
