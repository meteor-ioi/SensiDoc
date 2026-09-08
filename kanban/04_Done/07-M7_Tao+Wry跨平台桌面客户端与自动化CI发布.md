---
priority: high
tags: [desktop, tao, wry, windows, macos, ci]
created: 2026-09-07
---

# M7 - Tao+Wry 跨平台桌面客户端与自动化 CI 发布

### 📌 任务目标
基于 Tao + Wry 框架将 SensiDoc 封装为原生级跨平台桌面客户端，实现 macOS 沉浸式透明标题栏（交通灯避让）与 Windows 无边框拖拽，统一微软雅黑与 Segoe UI 字体，搭建 GitHub Actions 跨平台自动化打包与 Release 发布流水线。

### 📋 子任务清单 (Checklist)
- [x] macOS 沉浸式无边框窗口与交通灯垂直居中精确避让（78px 边距）
- [x] Windows 平台无边框三联控制按钮（最小化、最大化/还原、关闭）与窗口拖拽
- [x] 跨平台窗口关闭统一二次确认弹窗（拦截直接退出，安全终止后台子进程）
- [x] Windows 界面字体统合为“微软雅黑”与 Segoe UI（杜绝宋体混杂）
- [x] Inno Setup Windows 标准安装向导与 PE 图标嵌入 (`sensidoc_win.ico`)
- [x] macOS Universal（Apple Silicon + Intel）通用 DMG 映像打包构建
- [x] GitHub Actions CI/CD 流水线搭建，实现 v1.3.0 正式发布

### 📝 开发记录与进度
- *2026-09-07*：完成桌面无边框窗口与 IPC 拖拽通信。
- *2026-09-08*：完成 Inno Setup 脚本优化、CI 自动化发布闭环与 v1.3.0 正式交付。

### 🔗 关联文件 / 依赖
- 桌面主程序：[`src/main.rs`](file:///Users/icychick/Projects/SensiDoc/src/main.rs)
- macOS 打包脚本：[`scripts/build_mac_app.sh`](file:///Users/icychick/Projects/SensiDoc/scripts/build_mac_app.sh)
- Windows Inno Setup：[`scripts/installer.iss`](file:///Users/icychick/Projects/SensiDoc/scripts/installer.iss)
- CI 发布流水线：[`.github/workflows/release.yml`](file:///Users/icychick/Projects/SensiDoc/.github/workflows/release.yml)
