---
id: phase-98
title: 脱敏策略多样化——硬抹除 (Redaction) 模式支持
priority: medium
tags: [phase]
created: 2026-09-13
---

# 脱敏策略多样化——硬抹除 (Redaction) 模式支持

### 📌 任务概述
阶段九十八：脱敏策略多样化——硬抹除 (Redaction) 模式支持 (P2) (待办)

### 📋 子任务清单 (Checklist)
- [ ] 98.1 **后端脱敏策略扩展 (`src/desensitizer.rs`)**：
  - 抽象脱敏模式枚举 `MaskStyle`：`Masking`（保留首尾星号掩码 `张*三`）与 `Redaction`（硬抹除 `████`）；
  - `desensitize_plain_text` 与多格式导出支持按选定策略执行替换。
- [ ] 98.2 **原生 Office/PDF XML 容器硬抹除支持 (`src/desensitizer.rs`)**：
  - 确保跨 XML `<w:t>`、`<a:t>` 节点及 PDF 内容流支持等长或等宽字符硬抹除，不破坏文档排版结构。
- [ ] 98.3 **前端脱敏导出交互适配 (`web/index.html`, `web/app.js`)**：
  - 导出操作栏增加脱敏模式切换（掩码 / 硬抹除）；
  - 请求后端脱敏导出接口时传递选定的模式参数。
- [ ] 98.4 **端到端测试与格式导出回归**：
  - 验证 Markdown / DOCX / XLSX / PPTX / PDF 在硬抹除模式下的导出保真度。


### 📝 开发记录与进度
- *2026-09-13*：由 todo.md 自动化同步生成。当前完成度: [0/4]。
