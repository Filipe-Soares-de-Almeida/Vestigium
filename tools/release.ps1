# Writes a version's release notes and the update feed (latest.json) from the changelogs and the signed installer.
# Usage: powershell -ExecutionPolicy Bypass -File tools/release.ps1 -Version 0.1.0 [-Bundle <nsis folder>] [-Out <folder>]
param(
    [Parameter(Mandatory = $true)] [string]$Version,
    [string]$Bundle,
    [string]$Out,
    [string]$Repo = 'Filipe-Soares-de-Almeida/Vestigium',
    [string]$Changelog,
    [string]$Translation
)
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
if (-not $Changelog) { $Changelog = Join-Path $root 'CHANGELOG.md' }
if (-not $Translation) { $Translation = Join-Path $root 'locales\changelog\pt-BR.md' }
if (-not $Bundle) { $Bundle = Join-Path $target 'release\bundle\nsis' }
if (-not $Out) { $Out = Join-Path $target 'release-out' }
$utf8 = New-Object System.Text.UTF8Encoding $false

# The body of `## [<version>] - <date>`, up to the next version or the link references.
function Get-Section([string]$path, [string]$version) {
    if (-not (Test-Path $path)) { return $null }
    $lines = [System.IO.File]::ReadAllText($path, $utf8) -split "`r?`n"
    $heading = '^## \[' + [regex]::Escape($version) + '\] - \d{4}-\d{2}-\d{2}\s*$'
    $start = -1
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match $heading) { $start = $i; break }
    }
    if ($start -lt 0) { return $null }
    $body = New-Object System.Collections.Generic.List[string]
    for ($i = $start + 1; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^## \[' -or $lines[$i] -match '^\[[^\]]+\]: ') { break }
        $body.Add($lines[$i])
    }
    $text = ($body -join "`n").Trim()
    if ($text) { return $text } else { return $null }
}

$cargo = [System.IO.File]::ReadAllText((Join-Path $root 'Cargo.toml'), $utf8)
if ($cargo -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') { throw 'Cargo.toml has no workspace version.' }
if ($Matches[1] -ne $Version) { throw "The workspace version is $($Matches[1]), not $Version." }

$english = Get-Section $Changelog $Version
if (-not $english) { throw "CHANGELOG.md has no '## [$Version] - YYYY-MM-DD' section with entries." }
$portuguese = Get-Section $Translation $Version

$installer = "Vestigium_${Version}_x64-setup.exe"
$setup = Join-Path $Bundle $installer
foreach ($file in @($setup, "$setup.sig")) {
    if (-not (Test-Path $file)) { throw "$file is missing; build with tools/bundle.ps1 and the signing key first." }
}

$notes = [ordered]@{ en = $english }
if ($portuguese) { $notes['pt-BR'] = $portuguese }
$feed = [ordered]@{
    version   = $Version
    notes     = ($notes | ConvertTo-Json -Compress)
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = ([System.IO.File]::ReadAllText("$setup.sig", $utf8)).Trim()
            url       = "https://github.com/$Repo/releases/download/v$Version/$installer"
        }
    }
}

New-Item -ItemType Directory -Force $Out | Out-Null
[System.IO.File]::WriteAllText((Join-Path $Out 'latest.json'), ($feed | ConvertTo-Json -Depth 5), $utf8)
# The page's notes add how to install and verify; the in-app dialog shows only the changelog section.
$footer = @'

---

### Install

Download `{installer}` below and run it. It installs for your user only and needs no administrator rights.

Vestigium {version} is not code-signed yet. Windows may show "Windows protected your PC": check that the file
came from this page and that its SHA-256 matches `SHA256SUMS.txt`, then choose **More info**, then
**Run anyway**. Windows asks for administrator rights only when you start a task that needs them, and that
prompt names an unknown publisher. On Windows 11 with Smart App Control turned on, Windows blocks apps that
are not signed.

### Verify your download

```powershell
(Get-FileHash .\{installer} -Algorithm SHA256).Hash
```

The result must match the installer's line in `SHA256SUMS.txt`, ignoring letter case. With the GitHub CLI,
`gh attestation verify {installer} -R {repo}` checks that this repository's release workflow built it.

[Full changelog](https://github.com/{repo}/blob/v{version}/CHANGELOG.md)
'@
$footer = $footer.Replace('{installer}', $installer).Replace('{version}', $Version).Replace('{repo}', $Repo)
[System.IO.File]::WriteAllText((Join-Path $Out 'release-notes.md'), "$english`n$footer", $utf8)
$languages = if ($portuguese) { 'en, pt-BR' } else { 'en' }
"Release $Version ($languages): $(Join-Path $Out 'latest.json'), $(Join-Path $Out 'release-notes.md')"
