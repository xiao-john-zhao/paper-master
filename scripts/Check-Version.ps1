[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$packageVersion = (Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json).version
$tauriVersion = (Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
$cargoSource = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri/Cargo.toml') -Raw
$frontendSource = Get-Content -LiteralPath (Join-Path $projectRoot 'src/config/appConfig.ts') -Raw
$cargoLock = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri/Cargo.lock') -Raw
$cargoVersion = [regex]::Match($cargoSource, '(?m)^version = "([^"]+)"').Groups[1].Value
$frontendVersion = [regex]::Match($frontendSource, 'version: "([^"]+)"').Groups[1].Value
$lockedVersion = [regex]::Match($cargoLock, '(?ms)\[\[package\]\]\r?\nname = "paper-master"\r?\nversion = "([^"]+)"').Groups[1].Value
foreach ($candidate in @($tauriVersion, $cargoVersion, $frontendVersion, $lockedVersion)) {
    if ($candidate -ne $packageVersion) { throw "版本号不一致：期望 $packageVersion，实际 $candidate。" }
}
Write-Host "版本源及 Cargo.lock 一致：v$packageVersion"
