[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$releaseVersion = (Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json).version
$installerName = "paper-master_${releaseVersion}_x64-setup.exe"
$installer = Join-Path $projectRoot "src-tauri\target\release\bundle\nsis\$installerName"
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) { throw '请先运行 scripts/Build-Local.ps1 构建并验证安装包。' }
$output = Join-Path $projectRoot "handoff\release-v$releaseVersion"
[IO.Directory]::CreateDirectory($output) | Out-Null
$payload = Join-Path $output 'windows-x64'
[IO.Directory]::CreateDirectory($payload) | Out-Null
Copy-Item -LiteralPath $installer -Destination (Join-Path $payload $installerName)
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs\下载安装使用指南.md') -Destination $payload
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Install-LMStudio.cmd') -Destination $payload
$checksums = @(Get-ChildItem -LiteralPath $payload -File | Where-Object Name -ne 'SHA256SUMS.txt' | ForEach-Object { "{0} *{1}" -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Name })
[IO.File]::WriteAllLines((Join-Path $payload 'SHA256SUMS.txt'), $checksums, [Text.UTF8Encoding]::new($false))
$archive = Join-Path $output "paper-master-v$releaseVersion-windows-x64.zip"
Compress-Archive -LiteralPath @(Get-ChildItem -LiteralPath $payload -File | ForEach-Object FullName) -DestinationPath $archive -Force
Copy-Item -LiteralPath (Join-Path $payload $installerName) -Destination $output
Copy-Item -LiteralPath (Join-Path $payload '下载安装使用指南.md') -Destination $output
$checksums = @($archive, (Join-Path $output $installerName)) | ForEach-Object { "{0} *{1}" -f (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant(), [IO.Path]::GetFileName($_) }
[IO.File]::WriteAllLines((Join-Path $output 'SHA256SUMS.txt'), $checksums, [Text.UTF8Encoding]::new($false))
Write-Output $output
