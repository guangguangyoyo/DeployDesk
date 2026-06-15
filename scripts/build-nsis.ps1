param(
    [string]$TargetDir = "$env:TEMP\deploydesk-nsis-target"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path
$CargoToml = Join-Path $RepoRoot "Cargo.toml"
$NsiScript = Join-Path $RepoRoot "packaging\windows\deploydesk.nsi"
$DistDir = Join-Path $RepoRoot "dist"

$CargoText = Get-Content -Path $CargoToml -Raw
if ($CargoText -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw "Could not read package version from Cargo.toml."
}

$Version = $Matches[1]
$VersionCore = ($Version -split "[+-]")[0]
$VersionParts = $VersionCore.Split(".")
while ($VersionParts.Count -lt 4) {
    $VersionParts += "0"
}
$Version4 = ($VersionParts[0..3] -join ".")

$MakensisCommand = Get-Command makensis -ErrorAction SilentlyContinue
$MakensisPath = if ($MakensisCommand) { $MakensisCommand.Source } else { $null }
if (-not $MakensisPath) {
    $CommonPaths = @(
        "C:\Program Files\NSIS\makensis.exe",
        "C:\Program Files (x86)\NSIS\makensis.exe"
    )
    foreach ($Path in $CommonPaths) {
        if (Test-Path $Path) {
            $MakensisPath = (Get-Item $Path).FullName
            break
        }
    }
}

if (-not $MakensisPath) {
    throw "makensis.exe was not found. Install NSIS, or add makensis.exe to PATH, then run this script again."
}

New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

Push-Location $RepoRoot
try {
    cargo build --release --target-dir $TargetDir
    $AppExe = Join-Path $TargetDir "release\deploydesk.exe"
    if (-not (Test-Path $AppExe)) {
        throw "Release executable was not created: $AppExe"
    }

    & $MakensisPath `
        "/DROOT_DIR=$RepoRoot" `
        "/DPRODUCT_VERSION=$Version" `
        "/DPRODUCT_VERSION4=$Version4" `
        "/DAPP_EXE=$AppExe" `
        "/DOUT_DIR=$DistDir" `
        $NsiScript
}
finally {
    Pop-Location
}

$Installer = Join-Path $DistDir "DeployDesk-Setup-$Version.exe"
if (-not (Test-Path $Installer)) {
    throw "Installer was not created: $Installer"
}

Write-Host "Created installer: $Installer"
