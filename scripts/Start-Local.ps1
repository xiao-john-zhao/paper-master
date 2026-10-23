#requires -Version 7.0
[CmdletBinding()]
param(
    [string]$Model = '',
    [ValidateRange(4096, 131072)][int]$ContextLength = 24576,
    [switch]$CheckOnly,
    [switch]$SkipLocalModel
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$executable = Join-Path $projectRoot 'application\paper-master.exe'
if (-not $CheckOnly) {
    if (-not (Test-Path -LiteralPath $executable)) { throw '请先运行 scripts\Build-Local.ps1。' }
    if (-not (Get-Process -Name 'paper-master' -ErrorAction SilentlyContinue | Where-Object Path -EQ $executable)) {
        Start-Process -FilePath $executable -WorkingDirectory $projectRoot -WindowStyle Hidden
    }
    Write-Host '已启动统一应用；联网开关位于 Settings，本地模型可选。'
    return
}

if ($SkipLocalModel) {
    if (-not $CheckOnly) { Start-Process -FilePath $executable -WorkingDirectory $projectRoot -WindowStyle Hidden }
    Write-Host '已跳过 LM Studio 初始化，使用应用中保存的服务配置。'
    return
}

function Get-LocalModels {
    try { return Invoke-RestMethod -Uri 'http://127.0.0.1:1234/api/v1/models' -NoProxy -TimeoutSec 5 }
    catch { return $null }
}

$cliCommand = Get-Command lms -ErrorAction SilentlyContinue
$cliPath = if ($cliCommand) { $cliCommand.Source } else { Join-Path $env:USERPROFILE '.lmstudio\bin\lms.exe' }
$catalog = Get-LocalModels
if (-not $catalog) {
    if (-not (Test-Path -LiteralPath $cliPath)) { throw '请先安装并启动 LM Studio，再运行本机启动脚本。' }
    & $cliPath server start --port 1234 --bind 127.0.0.1
    if ($LASTEXITCODE -ne 0) { throw 'LM Studio 本地服务启动失败。' }
    $catalog = Get-LocalModels
}
if (-not $catalog) { throw '无法连接本地 LM Studio API。' }
$languageModels = @($catalog.models | Where-Object type -EQ 'llm')
$selected = if ($Model) { $languageModels | Where-Object key -EQ $Model | Select-Object -First 1 }
    else { $languageModels | Where-Object { $_.loaded_instances.Count -gt 0 } | Select-Object -First 1 }
if (-not $selected -and -not $Model) { $selected = $languageModels | Select-Object -First 1 }
if (-not $selected) { throw '未发现可用语言模型。请在 LM Studio 下载模型，或检查 -Model 参数。' }
if ($selected.loaded_instances.Count -eq 0) {
    if (-not (Test-Path -LiteralPath $cliPath)) { throw '模型尚未加载，请先在 LM Studio 加载。' }
    & $cliPath load $selected.key --context-length $ContextLength --parallel 1 --no-speculative-draft-mtp -y
    if ($LASTEXITCODE -ne 0) { throw '模型加载失败，请在 LM Studio 检查可用内存、显存和上下文长度。' }
}
Write-Host "LM Studio 已就绪：$($selected.key)，仅连接 127.0.0.1:1234。"
if (-not $CheckOnly) { Start-Process -FilePath $executable -WorkingDirectory $projectRoot -WindowStyle Hidden }
