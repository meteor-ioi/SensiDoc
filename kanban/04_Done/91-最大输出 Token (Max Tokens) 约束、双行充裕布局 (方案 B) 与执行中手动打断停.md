---
id: phase-91
title: 最大输出 Token (Max Tokens) 约束、双行充裕布局 (方案 B) 与执行中手动打断停止功能
priority: medium
tags: [phase]
created: 2026-09-18
---

# 最大输出 Token (Max Tokens) 约束、双行充裕布局 (方案 B) 与执行中手动打断停止功能

### 📌 任务概述
阶段九十一：最大输出 Token (Max Tokens) 约束、双行充裕布局 (方案 B) 与执行中手动打断停止功能 (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)

### 📋 子任务清单 (Checklist)
- [x] 91.1 **Max Tokens 参数结构与全链路透传 (`src/session.rs`, `src/extractor.rs`, `src/main.rs`, `src/cli.rs`, `src/benchmark.rs`)**：
  - `OfflineModelProfile` 与 `OnlineModelProfile` 新增 `#[serde(default = "default_max_tokens")] pub max_tokens: u32`（离线默认 1024，在线默认 2048），有效防范小模型幻觉复读与死循环；
  - `Extractor::query_llm` 与 `Extractor::query_online_llm` 接收并传递 `max_tokens` 至请求体；
  - `src/main.rs` 与 `src/cli.rs` 动态读取并透传对应 Profile 的 `max_tokens`。
- [x] 91.2 **模型参数面板双行充裕布局重构 (方案 B) (`web/style.css`, `web/index.html`, `web/app.js`)**：
  - **第 1 行（4 列数值网格）**：采样温度 (Temp)、候选范围 (Top-K)、重复惩罚 (Repeat)、最大Token (Max)；
  - **第 2 行（独立状态与操作行）**：
    - 左侧：`思考模式 (Think)：` + 紧凑 Toggle 按钮（`[●──] 未开启` / `[──●] 已开启`） + `(针对推理模型)` 辅助提示；
    - 右侧：状态指示与 `[ 恢复默认 ]` 一键还原按钮（离线抽屉重置为 0.1 / 50 / 1.1 / 1024 / 关闭思考）；
  - 在线模型表单与离线模型抽屉视觉语言 100% 对齐。
- [x] 91.3 **执行中原位切换「停止执行」与前后端打断闭环 (`web/app.js`, `web/style.css`)**：
  - **前端状态流转**：点击「立即执行」后，右侧复合按钮原位切换为醒目的高亮危险红 `[ ⏹ 停止执行 ]`（带呼吸脉冲动效），左侧模型选择器锁定禁用；
  - **用户手动打断**：执行过程中再次点击「停止执行」，触发 `AbortController.abort()` 立即中断 HTTP 请求，前端友好捕获 `AbortError`，零报错弹窗优雅复位；
  - **后端算力释放**：客户端连接断开时，底层 Tokio 自动取消任务并断开与 `llama-server` / API 连接，释放 GPU/CPU 计算资源。
- [x] 91.4 **全链路实机验证与回归测试**：
  - 26 项 Rust 自动化单元测试全部通过（100% 绿灯）；
  - 静态资源版本升级至 `v=1.2.17`；
  - Chrome 实机端到端验证抽屉数值输入、失焦保存、恢复默认、在线模型配置与执行中红色打断停止按钮交互正常。

---

<<<<<<< HEAD

### 🔗 关联实施计划
- [对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)

### 📝 开发记录与进度
- *2026-09-18*：由 todo.md 自动化同步生成。当前完成度: [4/4]。
