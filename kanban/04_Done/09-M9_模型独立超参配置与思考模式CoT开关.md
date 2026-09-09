---
id: card-578d9be794e4
title: 09-M9 模型独立超参配置与思考模式CoT开关
priority: high
tags:
  - backend
  - frontend
  - llm
  - hyperparams
  - cot
created: 2026-09-08
---
# M9 - 模型独立超参配置与思考模式 (CoT) 开关

### 📌 任务目标
为离线与在线模型提供独立持久化的采样温度 (Temp)、候选范围 (Top-K)、重复惩罚 (Repeat) 参数配置，并引入思考模式（Thinking / CoT）开关（默认关闭），前置剥离 `<think>...</think>` 思维链标签，确保 DeepSeek-R1 / QwQ 等推理模型稳定输出结构化 JSON。

### 📋 子任务清单 (Checklist)
- [x] `OfflineModelProfile` / `OnlineModelProfile` 新增 `max_tokens: u32`（离线 1024 / 在线 2048）与 `enable_thinking: bool` 并持久化落盘
- [x] 提取引擎在发起请求时透传 `max_tokens` 参数，彻底防范小模型幻觉复读与死循环
- [x] 解析器智能截断：检测并自动剥离 `</think>` 前置思维链内容，消除杂质文本
- [x] 前端模型参数面板重构为方案 B 双行充裕布局（第 1 行 4 个数值输入框，第 2 行独立思考模式开关与恢复默认）
- [x] 主工作台执行按钮支持执行中原位切换为醒目的红色 `[ ⏹ 停止执行 ]`，支持点击手动打断并释放后端算力
- [x] 编写单元测试 `test_parse_llm_json_response_with_thinking`，全套 26 项单元测试全部绿灯通过并完成 Chrome 实机端到端全链路验证

### 📝 开发记录与进度
- *2026-09-08*：完成 4 列并排网格、思维链剥离与全套 26 项单元测试验证。
- *2026-09-09*：完成 `max_tokens` 最大输出 Token 限制、参数面板方案 B 充裕布局落地，以及执行中原位切换「停止执行」打断停止全链路闭环。

### 🔗 关联文件 / 依赖
- 提取与解析：[`src/extractor.rs`](file:///Users/icychick/Projects/SensiDoc/src/extractor.rs)
- 配置与持久化：[`src/session.rs`](file:///Users/icychick/Projects/SensiDoc/src/session.rs)
- 交互界面：[`web/style.css`](file:///Users/icychick/Projects/SensiDoc/web/style.css), [`web/index.html`](file:///Users/icychick/Projects/SensiDoc/web/index.html), [`web/app.js`](file:///Users/icychick/Projects/SensiDoc/web/app.js)

