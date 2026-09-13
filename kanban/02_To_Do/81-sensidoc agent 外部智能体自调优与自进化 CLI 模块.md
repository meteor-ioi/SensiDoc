---
id: phase-81
title: sensidoc agent 外部智能体自调优与自进化 CLI 模块
priority: medium
tags: [phase]
created: 2026-09-13
---

# sensidoc agent 外部智能体自调优与自进化 CLI 模块

### 📌 任务概述
阶段八十一：sensidoc agent 外部智能体自调优与自进化 CLI 模块 (待办 / 规划中)

### 📋 子任务清单 (Checklist)
- [ ] 81.1 **`sensidoc agent inspect` (内省与环境导出)**：
  - 支持导出当前模型清单、各参数量模型提示词档案与模板规则配置为机器可读纯 JSON；
  - 供外部 Agent (如 Claude Code, Cursor, Antigravity) 自动化了解当前软件运行基线。
- [ ] 81.2 **`sensidoc agent eval` (沙盒评测与量化打分体系)**：
  - 基于内置 benchmark 真实多场景样本集，支持传入候选 prompt 或策略进行沙盒干跑（dry-run）；
  - 输出机器友好结构化评估报告 JSON (Precision, Recall, F1 分数、推理耗时及具体错题样本)。
- [ ] 81.3 **`sensidoc agent apply` (策略与提示词安全回写)**：
  - 支持 Agent 自动化将调优后的轻量端侧模型专属提示词与场景模板安全持久化；
  - 实现从“评测-微调-回写-生效”的自闭环进化。

---


### 📝 开发记录与进度
- *2026-09-13*：由 todo.md 自动化同步生成。当前完成度: [0/3]。
