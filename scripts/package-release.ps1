#Requires -Version 5.1
<#
.SYNOPSIS
    Build release binaries for Vigil (service + dashboard) and stage them for
    installer bundling (NSIS). Requires a working Rust toolchain.
.EXAMPLE
    .\scripts\package-release.ps1 -OutDir .\dist
#>
[CmdletBinding()]
param(
    [string]$OutDir = (Join-Path (Split-Path -Parent $PSScriptRoot) "dist")
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

Push-Location $root
try {
    cargo build --release -p vigil-service
    cargo build --release -p vigil-dashboard
} finally {
    Pop-Location
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$target = Join-Path $root "target\x86_64-pc-windows-gnu\release"
Copy-Item (Join-Path $target "vigil-service.exe") $OutDir -Force
Copy-Item (Join-Path $target "vigil-dashboard.exe") $OutDir -Force

Write-Host "Release binaries staged in: $OutDir" -ForegroundColor Green
Write-Host "Next: wrap with NSIS (Vigil.nsi) + set Task Scheduler auto-start." -ForegroundColor Cyan
