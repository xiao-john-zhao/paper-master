# 统一应用

当前版本 v0.4.0。唯一启动入口为 `paper-master.exe`，在线能力通过 Settings 中的联网开关控制并保存。`scripts/Start-Local.ps1` 直接启动该文件；本机模型是可选运行时。

用 `pwsh -File scripts/Build-Local.ps1` 重建。可执行文件及快捷方式不提交 Git。历史 local/updated 二进制移至 `handoff/legacy-apps-20261024/`，仅供回退。

安装包：`src-tauri/target/release/bundle/nsis/paper-master_0.4.0_x64-setup.exe`。实际数据库继续使用同一个应用标识和数据目录。
