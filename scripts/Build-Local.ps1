#requires -Version 7.0
[CmdletBinding()]
param([switch]$NoInstaller)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$env:Path = "$(Join-Path $env:USERPROFILE '.cargo\bin');$env:Path"
# Avoid competing with a locally loaded model for all available RAM during compilation.
$env:CARGO_BUILD_JOBS = '2'
Push-Location $projectRoot
try {
    foreach ($required in @('node', 'pnpm', 'cargo')) {
        if (-not (Get-Command $required -ErrorAction SilentlyContinue)) { throw "缺少 $required，请先按 README 安装开发环境。" }
    }
    pnpm install --frozen-lockfile
    if ($LASTEXITCODE -ne 0) { throw '前端依赖安装失败。' }
    pnpm test
    if ($LASTEXITCODE -ne 0) { throw '前端测试失败。' }
    cargo test --locked --manifest-path src-tauri/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw '后端测试失败。' }
    if ($NoInstaller) { pnpm tauri build --ci --no-bundle -- --locked }
    else { pnpm tauri build --ci -- --locked }
    if ($LASTEXITCODE -ne 0) { throw '桌面应用构建失败。' }
    Copy-Item -LiteralPath (Join-Path $projectRoot 'src-tauri\target\release\paper-master.exe') -Destination (Join-Path $projectRoot 'application\paper-master-updated.exe')
    Write-Host '本机应用已生成：application\paper-master-updated.exe'
}
finally { Pop-Location }
