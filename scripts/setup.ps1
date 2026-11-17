#Requires -Version 5.1
<#
.SYNOPSIS
    One-time environment setup for Vigil: verifies the Rust toolchain (MSVC link
    support) and installs frontend dependencies.
.EXAMPLE
    .\scripts\setup.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

Write-Host "== Vigil environment setup ==" -ForegroundColor Cyan

# --- Rust / Cargo ---
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) {
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path $cargoBin) { $cargo = Get-Command $cargoBin }
}
if (-not $cargo) {
    throw "cargo not found. Install via: https://rustup.rs"
}
Write-Host "[ok] cargo: $($cargo.Source)" -ForegroundColor Green

# --- MSVC linker ---
$link = Get-Command link.exe -ErrorAction SilentlyContinue
if (-not $link) {
    Write-Warning "link.exe not on PATH. Ensure 'Desktop development with C++' (VCTools) is installed; otherwise cargo builds will fail at link time."
} else {
    Write-Host "[ok] link: $($link.Source)" -ForegroundColor Green
}

# --- Node / npm ---
$node = Get-Command node -ErrorAction SilentlyContinue
if (-not $node) { throw "node not found. Install Node.js 20+." }
Write-Host "[ok] node: $($node.Source)" -ForegroundColor Green

# --- Frontend deps ---
$frontend = Join-Path $root "frontend"
if (Test-Path (Join-Path $frontend "package.json")) {
    Push-Location $frontend
    try {
        if (-not (Test-Path (Join-Path $frontend "node_modules"))) {
            Write-Host "Installing frontend dependencies..." -ForegroundColor Yellow
            npm install
        } else {
            Write-Host "[ok] frontend node_modules present" -ForegroundColor Green
        }
    } finally { Pop-Location }
}

Write-Host "Setup complete. Run .\scripts\build-all.ps1 to verify." -ForegroundColor Cyan
