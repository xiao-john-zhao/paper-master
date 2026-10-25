# 统一应用

当前版本 v0.4.2。统一启动入口为 `paper-master.exe`，并保留 `paper-master-v0.4.2.exe` 供版本核对。在线能力通过 Settings 中的联网开关控制并保存。`scripts/Start-Local.ps1` 直接启动该文件；本机模型是可选运行时。

用 `pwsh -File scripts/Build-Local.ps1` 重建。可执行文件及快捷方式不提交 Git。历史 local/updated 二进制移至 `handoff/legacy-apps-20261024/`，仅供回退。

安装包：`src-tauri/target/release/bundle/nsis/paper-master_0.4.2_x64-setup.exe`。数据库继续使用同一个应用标识和数据目录；新增 `%APPDATA%/app.papermaster.desktop/library-cache/v1/` 缓存，启动时自动补回缺失论文及分析。论文详情保留编辑接口，Diagnostics 中可打开、同步或恢复缓存。

`paper-master.exe --sync-cache` 可在备份原数据库后执行本地缓存维护。构建前运行 `pwsh -File scripts/Check-Version.ps1`，Build-Local 已集成该校验和原入口备份。本次 v0.4.0 入口备份位于 `handoff/release-backups/`。
