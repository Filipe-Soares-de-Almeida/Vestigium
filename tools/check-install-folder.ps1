# Fails when the installed program folder, or a file or folder in it, lets an account that is not an
# administrator write, delete or change permissions, or when the elevated helper is missing.
# Usage: powershell -ExecutionPolicy Bypass -File tools/check-install-folder.ps1 [-Folder <install folder>]
param(
    [string]$Folder = (Join-Path $env:ProgramFiles 'Vestigium'),
    [string]$Helper = 'Vestigium.Helper.exe'
)
$ErrorActionPreference = 'Stop'

# Everyone, Authenticated Users, Users and Interactive.
$open = 'S-1-1-0', 'S-1-5-11', 'S-1-5-32-545', 'S-1-5-4'
$Rights = [System.Security.AccessControl.FileSystemRights]
$change = $Rights::WriteData -bor $Rights::AppendData -bor $Rights::Delete -bor
    $Rights::DeleteSubdirectoriesAndFiles -bor $Rights::ChangePermissions -bor $Rights::TakeOwnership
$inheritOnly = [System.Security.AccessControl.PropagationFlags]::InheritOnly

if (-not (Test-Path -LiteralPath (Join-Path $Folder $Helper) -PathType Leaf)) {
    throw "The helper $Helper is not in $Folder."
}

$problems = foreach ($item in @(Get-Item -LiteralPath $Folder -Force) + @(Get-ChildItem -LiteralPath $Folder -Recurse -Force)) {
    foreach ($rule in (Get-Acl -LiteralPath $item.FullName).Access) {
        if ($rule.AccessControlType -ne 'Allow') { continue }
        if ($rule.PropagationFlags -band $inheritOnly) { continue }
        if (-not ($rule.FileSystemRights -band $change)) { continue }
        $sid = $rule.IdentityReference.Translate([System.Security.Principal.SecurityIdentifier]).Value
        if ($open -contains $sid) {
            "$($item.FullName): $($rule.IdentityReference) holds $($rule.FileSystemRights)"
        }
    }
}
if ($problems) {
    $problems | ForEach-Object { Write-Error $_ -ErrorAction Continue }
    throw "The installed folder $Folder can be changed by an account that is not an administrator."
}
"Only administrators can change $Folder."
