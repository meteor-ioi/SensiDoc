---
id: phase-90
title: 离线与在线模型思考模式 (Thinking _ CoT) 开关与容错架构 (方案二：4 列单行紧凑并排)
priority: medium
tags: [phase, backend, ai]
created: 2026-09-18
---

# 离线与在线模型思考模式 (Thinking _ CoT) 开关与容错架构 (方案二：4 列单行紧凑并排)

### 📌 任务概述
阶段九十：离线与在线模型思考模式 (Thinking / CoT) 开关与容错架构 (方案二：4 列单行紧凑并排) (已完成) · [🔗 对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)

### 📋 子任务清单 (Checklist)
- [x] 90.1 **推理思考开关数据结构与双轨透传 (`src/session.rs`, `src/main.rs`)**：
  - `OfflineModelProfile` 与 `OnlineModelProfile` 结构体新增 `#[serde(default)] pub enable_thinking: bool`，默认关闭（`false`）；
  - `/api/models/offline-profiles` 与 `/api/settings/online-models` 接口全面支持 `enable_thinking`；
  - 在线与离线推理调度流程动态读取思考开关配置并透传至提取引擎。
- [x] 90.2 **提取引擎与思维链标签智能剥离容错 (`src/extractor.rs`, `src/cli.rs`, `src/benchmark.rs`)**：
  - `Extractor::query_llm` 与 `Extractor::query_online_llm` 接入 `enable_thinking` 参数，开启思考模式时自动扩展生成长度上限；
  - `Extractor::parse_llm_json_response` 增加智能思维链截断：若检测到 `</think>` 标签，优先截取标签后的正文内容，彻底杜绝 DeepSeek-R1、QwQ 等模型思考过程干扰 JSON 解析；
  - 编写并通过单元测试 `test_parse_llm_json_response_with_thinking`。
- [x] 90.3 **前端 4 列并排紧凑网格与 Toggle 按钮落地 (方案二) (`web/style.css`, `web/index.html`, `web/app.js`)**：
  - `.model-param-grid` 升级为 4 列并排（采样温度、候选范围、重复惩罚、思考模式）；
  - 新增 `.param-toggle-btn` 样式（高度 28px 与输入框严格等高对齐，浅色/深色主题自适应）；
  - 思考模式状态指示直观醒目：`[●──] 未开启` / `[──●] 已开启`，默认全关闭；
  - 在线模型配置表单与离线模型参数抽屉同步对齐 4 列网格设计；
  - 支持一键“恢复默认”（一键还原为关闭思考及基准数值）。
- [x] 90.4 **全链路端到端功能验证与回归测试**：
  - 26 项 Rust 单元测试（`cargo test`）100% 绿灯通过；
  - 静态资源缓存标识升级至 `v=1.2.16`；
  - Chrome 实机端到端验证深浅色主题、4 列对齐、点击切换、持久化落盘与一键恢复默认。

---

### 🔗 关联实施计划
- [对话跳转](conversation://685f5dec-bb61-46f3-845f-db3dcf5d662a)

### 📝 开发记录与进度
- *2026-09-18*：由 todo.md 自动化同步生成。当前完成度: [4/4]。
