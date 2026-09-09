---
id: card-6b60d2fbad8a
title: 06-M6 统一CLI命令行与Agent自动化联动
priority: high
tags:
  - backend
  - cli
  - agent
  - automation
created: 2026-09-06
---
# M6 - 统一 CLI 命令行与 Agent 自动化联动

### 📌 任务目标
引入 `clap` 构建企业级 CLI 工具链（`sensidoc audit / mask / convert / templates / serve`），实现 stdout 纯净标准输出与 stderr 过程日志分离，支持跨进程与 Web 界面双向实时同步，为外部 AI Agent（如 Claude Code、Antigravity 等）提供零缝隙自动化调用能力。

### 📋 子任务清单 (Checklist)
- [x] 搭建 CLI 命令规范架构与标准 Unix 退出码（0: 放行, 1: 阻断, 2: 参数错误, 3: 推理异常）
- [x] 实现 `audit`（结构化审查 JSON）、`mask`（原格式原生打码）、`convert`（MD转换）等子命令
- [x] 支持场景模板读取（`-t / --template`）、自定义规则注入与 `--regex-only` 极速免模型模式
- [x] 智能复用常驻 llama-server（18188 端口），无实例时按需单次拉起并在退出时清理
- [x] CLI 处理记录与 Web 界面「文档列表」及快照双向实时热感知联动
- [x] 编制详细的 CLI 使用手册与 Agent 自动化集成指南 (`docs/CLI_GUIDE.md`)

### 📝 开发记录与进度
- *2026-09-06*：完成 CLI 五分子命令实现与管道 `jq` 适配。
- *2026-09-07*：实现跨进程 `mtime` 热感知双向联动与单元测试。

### 🔗 关联文件 / 依赖
- 命令行驱动：[`src/cli.rs`](file:///Users/icychick/Projects/SensiDoc/src/cli.rs)
- 文档指南：[`docs/CLI_GUIDE.md`](file:///Users/icychick/Projects/SensiDoc/docs/CLI_GUIDE.md)
