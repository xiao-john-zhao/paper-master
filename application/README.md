# paper-master v0.4.4

本机统一启动入口：`paper-master.exe`。带版本备份：`paper-master-v0.4.4.exe`。

普通用户建议使用 [GitHub Release](https://github.com/xiao-john-zhao/paper-master/releases) 的中文 Windows x64 安装器或分享 ZIP：无需开发环境，安装器内含 WebView2 离线运行时。

[中文下载安装使用指南](../docs/下载安装使用指南.md) 说明 AI 结果编辑、图谱局部缩放、LM Studio/其他 API 配置、语义基准与分段分析、缓存备份及分享。

可执行文件由 `scripts/Build-Local.ps1` 在全部测试通过后构建并复制到本目录；此前主入口自动备份到忽略的 handoff/release-backups。二进制、个人论文、密钥及模型不进入 Git。
