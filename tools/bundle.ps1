# Builds the per-machine installer (Program Files, administrator permission to install), with the watcher,
# the elevated helper and PresentMon beside the window.
# Usage: powershell -ExecutionPolicy Bypass -File tools/bundle.ps1
$ErrorActionPreference = 'Stop'

function Invoke-Step([string]$what, [scriptblock]$step) {
    & $step
    if ($LASTEXITCODE -ne 0) { throw "$what failed with exit code $LASTEXITCODE" }
}

$root = Split-Path -Parent $PSScriptRoot
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
$release = Join-Path $target 'release'
$binaries = Join-Path $root 'apps\ui\src-tauri\binaries'
$triple = 'x86_64-pc-windows-msvc'
$presentmonVersion = '2.6.0'
$presentmon = Join-Path $root "target\third-party\presentmon\$presentmonVersion"

Push-Location $root
try {
    Invoke-Step 'watcher and helper build' { cargo build --release --locked -p vestigium-agent -p vestigium-helper }
    & (Join-Path $PSScriptRoot 'fetch-presentmon.ps1')

    New-Item -ItemType Directory -Force $binaries | Out-Null
    $licenses = Join-Path $binaries 'licenses\PresentMon'
    New-Item -ItemType Directory -Force $licenses | Out-Null
    Copy-Item (Join-Path $release 'Vestigium.exe') (Join-Path $binaries "Vestigium-$triple.exe") -Force
    Copy-Item (Join-Path $release 'vestigium-helper.exe') (Join-Path $binaries "Vestigium.Helper-$triple.exe") -Force
    Copy-Item (Join-Path $presentmon "PresentMon-$presentmonVersion-x64.exe") `
        (Join-Path $binaries "PresentMon-$presentmonVersion-x64-$triple.exe") -Force
    Copy-Item (Join-Path $presentmon 'LICENSE.txt'), (Join-Path $presentmon 'THIRD_PARTY.txt') $licenses -Force

    Set-Location (Join-Path $root 'apps\ui')
    Invoke-Step 'npm ci' { npm ci --no-audit --no-fund }
    # With a signing key in the environment, the installer also gets the signature the updater checks.
    $configs = @('--config', 'src-tauri/tauri.bundle.conf.json')
    if ($env:TAURI_SIGNING_PRIVATE_KEY) { $configs += @('--config', 'src-tauri/tauri.updater.conf.json') }
    Invoke-Step 'tauri build' { npm run tauri -- build @configs }
}
finally {
    Pop-Location
}

Get-ChildItem (Join-Path $release 'bundle\nsis') -Filter '*-setup.exe*' |
    ForEach-Object { "Built: $($_.FullName) ($([math]::Round($_.Length / 1MB, 1)) MB)" }
