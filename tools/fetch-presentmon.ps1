# Downloads the PresentMon console the deep capture runs, and checks it before anything uses it.
# Usage: powershell -ExecutionPolicy Bypass -File tools\fetch-presentmon.ps1
$ErrorActionPreference = 'Stop'

$version = '2.6.0'
$root = Join-Path $PSScriptRoot "..\target\third-party\presentmon\$version"
$files = @(
    @{ Name = "PresentMon-$version-x64.exe"
       Url = "https://github.com/GameTechDev/PresentMon/releases/download/v$version/PresentMon-$version-x64.exe"
       Sha256 = 'B2A706BC6AD475749E3B7E3409263AA1E6906D45BDCF993F6DBC0F660188F1AF' },
    @{ Name = 'LICENSE.txt'
       Url = "https://raw.githubusercontent.com/GameTechDev/PresentMon/v$version/LICENSE.txt"
       Sha256 = '4C949341B1893C8C6AD82F7FB4EEDF622CD1FD9C22A9AF8F19B2DAC19D1947B6' },
    @{ Name = 'THIRD_PARTY.txt'
       Url = "https://raw.githubusercontent.com/GameTechDev/PresentMon/v$version/THIRD_PARTY.txt"
       Sha256 = 'E039937F1A2FC2EB8F24A25B4229551A5A8056026D2DA878507E20269EB56267' }
)

New-Item -ItemType Directory -Force $root | Out-Null
foreach ($file in $files) {
    $path = Join-Path $root $file.Name
    if (-not (Test-Path $path)) {
        Invoke-WebRequest -UseBasicParsing -Uri $file.Url -OutFile $path
    }
    $hash = (Get-FileHash -Algorithm SHA256 $path).Hash
    if ($hash -ne $file.Sha256) {
        Remove-Item $path
        throw "$($file.Name): SHA-256 $hash does not match the pinned $($file.Sha256)"
    }
}

$exe = Join-Path $root "PresentMon-$version-x64.exe"
$signature = Get-AuthenticodeSignature $exe
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '^CN=Intel Corporation,') {
    throw "PresentMon is not validly signed by Intel: $($signature.Status) $($signature.SignerCertificate.Subject)"
}
Write-Output "PresentMon $version ready in $((Resolve-Path $root).Path)"
