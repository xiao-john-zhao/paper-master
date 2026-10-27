[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$releaseVersion = (Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json).version
$executable = Join-Path $projectRoot 'application\paper-master.exe'
$file = Get-Item -LiteralPath $executable
if ($file.VersionInfo.ProductVersion -ne $releaseVersion) { throw '正式版 EXE 与项目版本不同，请先重新构建。' }
$digest = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant()
$manifest = [ordered]@{
    version = $releaseVersion
    platform = 'windows-x64'
    entrypoint = 'application/paper-master.exe'
    launcher = '启动paper-master.cmd'
    bytes = $file.Length
    sha256 = $digest
    runtime = 'Microsoft Edge WebView2 (installed separately or by the setup installer)'
    personal_data_included = $false
}
$utf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText((Join-Path $projectRoot 'application\release.json'), ($manifest | ConvertTo-Json) + "`n", $utf8)
[IO.File]::WriteAllText((Join-Path $projectRoot 'application\SHA256SUMS.txt'), "$digest *paper-master.exe`n", $utf8)
Write-Host "完整应用清单已更新：v$releaseVersion"
