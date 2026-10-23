# Application build

The final standalone Windows executable is copied into this folder after a successful release build using the versioned name `paper-master-v<version>.exe`.

Generated `.exe` and `.lnk` files are intentionally excluded from Git. They can always be reproduced with `pnpm tauri build`.

The local takeover build is available as `paper-master-local.exe`. Use `pwsh -NoProfile -File scripts/Build-Local.ps1` from the project root to regenerate it, or `scripts/Start-Local.ps1` to ensure the local LM Studio service/model is ready before opening the app. This worktree adds the requested upcoming v0.3.2 features without creating a new version tag; the embedded version remains 0.3.1.
# 2026-10-23 更新产物

最新可运行文件为 `paper-master-updated.exe`，由已通过测试的 release 构建复制生成。请关闭旧版窗口后启动。启动脚本优先使用此文件；旧的 `paper-master-local.exe` 未在运行中覆盖。NSIS 安装包为 `src-tauri/target/release/bundle/nsis/paper-master_0.3.1_x64-setup.exe`。版本源仍为 0.3.1，功能变更尚未创建发布标签。可执行文件不进入 Git 或迁移包。

本次更新还包含主要任务的进度条与任务中心、字号/排版调整，以及“论文关系图”中的关键词网络、论文网络、关联依据检查和 SVG/PNG 图片导出。图谱使用本地已有关键词与署名；需要更多主题数据时先分析论文。
