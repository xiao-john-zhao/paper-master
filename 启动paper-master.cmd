@echo off
chcp 65001 >nul
setlocal
if not exist "%~dp0application\paper-master.exe" (
  echo 未找到完整应用：application\paper-master.exe
  echo 请先解压完整下载包，或从下方 Release 下载 Windows 安装器。
  echo https://github.com/xiao-john-zhao/paper-master/releases/latest
  pause
  exit /b 1
)
start "paper-master" /D "%~dp0application" "%~dp0application\paper-master.exe"
if errorlevel 1 (
  echo 启动未成功，请查看“快速开始.txt”，或使用 Release 中的安装程序。
  pause
  exit /b 1
)
exit /b 0
