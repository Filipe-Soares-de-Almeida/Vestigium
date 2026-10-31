# Builds the per-user installer, with the agent shipped next to the interface.
# Usage: powershell -ExecutionPolicy Bypass -File tools/bundle.ps1
$ErrorActionPreference = 'Stop'

function Invoke-Step([string]$what, [scriptblock]$step) {
    & $step
    if ($LASTEXITCODE -ne 0) { throw "$what failed with exit code $LASTEXITCODE" }
}

$root = Split-Path -Parent $PSScriptRoot
$binaries = Join-Path $root 'apps\ui\src-tauri\binaries'

Push-Location $root
try {
    Invoke-Step 'agent build' { cargo build --release -p vestigium-agent }
    New-Item -ItemType Directory -Force $binaries | Out-Null
    Copy-Item (Join-Path $root 'target\release\vestigium-agent.exe') `
        (Join-Path $binaries 'vestigium-agent-x86_64-pc-windows-msvc.exe') -Force

    Set-Location (Join-Path $root 'apps\ui')
    Invoke-Step 'npm ci' { npm ci --no-audit --no-fund }
    Invoke-Step 'tauri build' { npm run tauri -- build --config src-tauri/tauri.bundle.conf.json }
}
finally {
    Pop-Location
}

Get-ChildItem (Join-Path $root 'target\release\bundle\nsis') -Filter '*-setup.exe' |
    ForEach-Object { "Installer: $($_.FullName) ($([math]::Round($_.Length / 1MB, 1)) MB)" }
