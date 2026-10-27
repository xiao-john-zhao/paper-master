[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$releaseVersion = (Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json).version
$installerName = "paper-master_${releaseVersion}_x64-setup.exe"
$installer = Join-Path $projectRoot "src-tauri\target\release\bundle\nsis\$installerName"
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) { throw '请先运行 scripts/Build-Local.ps1 构建并验证安装包。' }
& (Join-Path $PSScriptRoot 'Update-ApplicationManifest.ps1')
$executable = Join-Path $projectRoot 'application\paper-master.exe'
$built = Join-Path $projectRoot 'src-tauri\target\release\paper-master.exe'
if ((Get-FileHash -LiteralPath $executable).Hash -ne (Get-FileHash -LiteralPath $built).Hash) { throw '仓库正式版 EXE 与当前构建不一致。' }
$output = Join-Path $projectRoot "handoff\release-v$releaseVersion"
[IO.Directory]::CreateDirectory($output) | Out-Null
# Fresh isolated staging, plus explicit file lists: an old payload folder can never leak into a release.
$staging = Join-Path $projectRoot "handoff\release-staging\$([Guid]::NewGuid().ToString('N'))"
$setupStage = Join-Path $staging 'setup'
$portableStage = Join-Path $staging 'portable'
foreach ($folder in @($setupStage, (Join-Path $portableStage 'application'))) { [IO.Directory]::CreateDirectory($folder) | Out-Null }
$utf8 = [Text.UTF8Encoding]::new($false)
function Write-PayloadHashes([string]$Root, [string[]]$Files) {
    $lines = @($Files | ForEach-Object { "{0} *{1}" -f (Get-FileHash -LiteralPath (Join-Path $Root $_) -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Replace('\','/') })
    [IO.File]::WriteAllLines((Join-Path $Root 'SHA256SUMS.txt'), $lines, $utf8)
}
Copy-Item -LiteralPath $installer -Destination (Join-Path $setupStage $installerName)
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs\下载安装使用指南.md') -Destination (Join-Path $setupStage 'USER_GUIDE.zh-CN.md')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Install-LMStudio.cmd') -Destination $setupStage
Copy-Item -LiteralPath (Join-Path $projectRoot '快速开始.txt') -Destination $setupStage
$setupFiles = @($installerName, 'USER_GUIDE.zh-CN.md', 'Install-LMStudio.cmd', '快速开始.txt')
Write-PayloadHashes $setupStage $setupFiles
$archive = Join-Path $output "paper-master-v$releaseVersion-windows-x64.zip"
Compress-Archive -LiteralPath @(@($setupFiles + 'SHA256SUMS.txt') | ForEach-Object { Join-Path $setupStage $_ }) -DestinationPath $archive -Force

$applicationFiles = @('paper-master.exe', 'release.json', 'SHA256SUMS.txt', 'README.md')
foreach ($name in $applicationFiles) { Copy-Item -LiteralPath (Join-Path $projectRoot "application\$name") -Destination (Join-Path $portableStage "application\$name") }
foreach ($name in @('启动paper-master.cmd', '快速开始.txt')) { Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination $portableStage }
Copy-Item -LiteralPath (Join-Path $setupStage 'USER_GUIDE.zh-CN.md') -Destination $portableStage
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Install-LMStudio.cmd') -Destination $portableStage
$portableFiles = @('启动paper-master.cmd', '快速开始.txt', 'USER_GUIDE.zh-CN.md', 'Install-LMStudio.cmd') + @($applicationFiles | ForEach-Object { "application/$_" })
Write-PayloadHashes $portableStage $portableFiles
$portableArchive = Join-Path $output "paper-master-v$releaseVersion-portable-windows-x64.zip"
$portableEntries = @('启动paper-master.cmd', '快速开始.txt', 'USER_GUIDE.zh-CN.md', 'Install-LMStudio.cmd', 'SHA256SUMS.txt', 'application')
Compress-Archive -LiteralPath @($portableEntries | ForEach-Object { Join-Path $portableStage $_ }) -DestinationPath $portableArchive -Force
Copy-Item -LiteralPath (Join-Path $setupStage $installerName) -Destination $output
Copy-Item -LiteralPath (Join-Path $setupStage 'USER_GUIDE.zh-CN.md') -Destination $output
Copy-Item -LiteralPath (Join-Path $projectRoot '快速开始.txt') -Destination (Join-Path $output 'QUICK_START.zh-CN.txt')
$assets = @($archive, $portableArchive, (Join-Path $output $installerName))
$checksums = @($assets | ForEach-Object { "{0} *{1}" -f (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant(), [IO.Path]::GetFileName($_) })
[IO.File]::WriteAllLines((Join-Path $output 'SHA256SUMS.txt'), $checksums, $utf8)
Write-Output $output
