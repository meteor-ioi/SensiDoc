---
priority: high
tags: [backend, frontend, llm, hyperparams, cot]
created: 2026-09-08
---

# M9 - 模型独立超参配置与思考模式 (CoT) 开关

### 📌 任务目标
为离线与在线模型提供独立持久化的采样温度 (Temp)、候选范围 (Top-K)、重复惩罚 (Repeat) 参数配置，并引入思考模式（Thinking / CoT）开关（默认关闭），前置剥离 `<think>...</think>` 思维链标签，确保 DeepSeek-R1 / QwQ 等推理模型稳定输出结构化 JSON。

### 📋 子任务清单 (Checklist)
- [x] `OfflineModelProfile` / `OnlineModelProfile` 新增 `enable_thinking: bool` 并持久化落盘
- [x] 提取引擎在开启思考模式时动态扩展生成长度（2048/4096 tokens）
- [x] 解析器智能截断：检测并自动剥离 `</think>` 前置思维链内容，消除杂质文本
- [x] 前端离线模型抽屉与在线模型表单采用紧凑 4 列并排网格（方案二）
- [x] 28px 等高 Toggle 按钮（`[●──] 未开启` / `[──●] 已开启`）与一键“恢复默认”
- [x] 编写单元测试 `test_parse_llm_json_response_with_thinking` 并通过实机验证

### 📝 开发记录与进度
- *2026-09-08*：完成 4 列并排网格、思维链剥离与全套 26 项单元测试验证。

### 🔗 关联文件 / 依赖
- 提取与解析：[`src/extractor.rs`](file:///Users/icychick/Projects/SensiDoc/src/extractor.rs)
- 配置与持久化：[`src/session.rs`](file:///Users/icychick/Projects/SensiDoc/src/session.rs)
- 交互界面：[`web/style.css`](file:///Users/icychick/Projects/SensiDoc/web/style.css), [`web/app.js`](file:///Users/icychick/Projects/SensiDoc/web/app.js)
