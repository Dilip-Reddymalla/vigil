#Requires -Version 5.1
<#
.SYNOPSIS
    Build the full Vigil workspace: Rust crates + frontend.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

Write-Host "== Building Rust workspace ==" -ForegroundColor Cyan
Push-Location $root
try {
    cargo build --workspace
} finally {
    Pop-Location
}

Write-Host "== Building frontend ==" -ForegroundColor Cyan
$frontend = Join-Path $root "frontend"
Push-Location $frontend
try {
    if (-not (Test-Path (Join-Path $frontend "node_modules"))) { npm install }
    npm run build
} finally {
    Pop-Location
}

Write-Host "Build complete." -ForegroundColor Green
