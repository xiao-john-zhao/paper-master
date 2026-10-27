# 完整应用：paper-master v0.4.5

本目录已包含可以直接运行的正式版 `paper-master.exe`，不是下载占位文件。Windows 10/11 x64 用户无需安装开发工具。

## 如何打开

1. 先把仓库 ZIP 或免安装包完整解压。
2. 双击仓库/解压目录根部的 **启动paper-master.cmd**，或直接双击本目录 **paper-master.exe**。
3. 如果系统缺少 WebView2，请改用 [Release 中含离线组件的安装程序](https://github.com/xiao-john-zhao/paper-master/releases/latest)。

本目录的 `release.json` 记录版本、平台、文件大小和 SHA-256；`SHA256SUMS.txt` 可用于校验程序。打包时核对文件版本、项目版本及构建摘要，避免混入旧程序。

## 本目录允许分发的文件

- `paper-master.exe`：当前正式版完整应用（前端资源已嵌入）。
- `release.json`：发布清单，不包含个人配置。
- `SHA256SUMS.txt`：程序校验值。
- `README.md`：本说明。

个人论文、数据库、缓存、日志、临时文件、API密钥、模型和旧版EXE不属于应用运行文件，不进入本目录的Git白名单。带版本备份及构建目录保留在本机忽略路径，不应上传。

[快速开始](../快速开始.txt) · [中文使用指南](../docs/下载安装使用指南.md)
