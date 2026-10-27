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
    & (Join-Path $PSScriptRoot 'Check-Version.ps1')
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
    $releaseVersion = (Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json).version
    $entryPath = Join-Path $projectRoot 'application\paper-master.exe'
    if (Test-Path -LiteralPath $entryPath) {
        $backupDirectory = Join-Path $projectRoot 'handoff\release-backups'
        [IO.Directory]::CreateDirectory($backupDirectory) | Out-Null
        Copy-Item -LiteralPath $entryPath -Destination (Join-Path $backupDirectory "paper-master-before-$releaseVersion-$(Get-Date -Format yyyyMMdd-HHmmss).exe")
    }
    Copy-Item -LiteralPath (Join-Path $projectRoot 'src-tauri\target\release\paper-master.exe') -Destination (Join-Path $projectRoot "application\paper-master-v$releaseVersion.exe")
    try {
        Copy-Item -LiteralPath (Join-Path $projectRoot 'src-tauri\target\release\paper-master.exe') -Destination $entryPath -ErrorAction Stop
    }
    catch {
        if (-not (Test-Path -LiteralPath $entryPath) -or -not ($_.Exception.GetBaseException() -is [IO.IOException])) { throw }
        # Windows permits renaming a running image even when replacing its bytes is locked.
        # Keep that image alive in the ignored backup directory; never terminate the user's work.
        $backupDirectory = Join-Path $projectRoot 'handoff\release-backups'
        [IO.Directory]::CreateDirectory($backupDirectory) | Out-Null
        $retiredPath = Join-Path $backupDirectory "paper-master-running-before-$releaseVersion-$([Guid]::NewGuid().ToString('N')).exe"
        [IO.File]::Move($entryPath, $retiredPath)
        try { Copy-Item -LiteralPath (Join-Path $projectRoot 'src-tauri\target\release\paper-master.exe') -Destination $entryPath -ErrorAction Stop }
        catch {
            if (-not (Test-Path -LiteralPath $entryPath)) { [IO.File]::Move($retiredPath, $entryPath) }
            throw
        }
        Write-Host '旧应用仍在运行，已保留其进程和文件备份；退出后重新打开即可使用新版。'
    }
    & (Join-Path $PSScriptRoot 'Update-ApplicationManifest.ps1')
    Write-Host '本机应用已生成：application\paper-master.exe'
}
finally { Pop-Location }
