---
id: card-ba8ddf522803
title: 12-sensidoc agent外部智能体自调优与进化CLI
priority: medium
tags:
  - cli
  - agent
  - eval
  - benchmark
created: 2026-09-08
---
# sensidoc agent 外部智能体自调优与自进化 CLI 模块

### 📌 任务目标
为外部 Agent（如 Claude Code, Cursor, Antigravity）提供内省评估与安全回写接口，使 AI 智能体能够自动化评测模型提取表现、微调提示词策略并安全持久化，实现“评测-微调-回写-生效”的自闭环调优。

### 📋 子任务清单 (Checklist)
- [ ] 实现 `sensidoc agent inspect`：导出当前模型清单、提示词档案与模板规则为纯 JSON
- [ ] 实现 `sensidoc agent eval`：基于真实多场景样本集执行沙盒干跑，输出 F1/Recall/Precision 评分报告
- [ ] 实现 `sensidoc agent apply`：支持外部 Agent 自动化将调优后的专属提示词与场景模板安全回写落盘
- [ ] 编写 Agent 自动化调优沙盒测试脚本与文档

### 📝 开发记录与进度
- *2026-09-08*：创建任务卡片，架构规划已记录于 `todo.md`。

### 🔗 关联文件 / 依赖
- 命令行模块：[`src/cli.rs`](file:///Users/icychick/Projects/SensiDoc/src/cli.rs)
- 基准评测引擎：[`src/benchmark.rs`](file:///Users/icychick/Projects/SensiDoc/src/benchmark.rs)
