@echo off
chcp 65001 >nul
echo 正在通过 Windows 软件包管理器下载并安装可选的 LM Studio。
echo 模型需在安装后按本机内存和显存选择下载，不随 paper-master 分发。
where winget >nul 2>nul
if errorlevel 1 goto website
winget install --exact --id ElementLabs.LMStudio --source winget --accept-source-agreements --accept-package-agreements
if errorlevel 1 goto website
echo 安装完成。请打开 LM Studio 下载模型，并在 Developer 中启动本地服务。
pause
exit /b 0
:website
echo 自动安装不可用，将打开官方下载安装页面。
start "" "https://lmstudio.ai/download?os=windows"
pause
