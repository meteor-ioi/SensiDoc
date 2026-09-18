---
id: phase-98
title: 脱敏策略多样化——硬抹除 (Redaction) 模式与脱敏预览支持
priority: medium
tags: [phase]
created: 2026-09-18
---

# 脱敏策略多样化——硬抹除 (Redaction) 模式与脱敏预览支持

### 📌 任务概述
阶段九十八：脱敏策略多样化——硬抹除 (Redaction) 模式与脱敏预览支持 (已完成)

### 📋 子任务清单 (Checklist)
- [x] 98.1 **后端脱敏策略扩展与接口支持 (`src/desensitizer.rs`, `src/main.rs`, `src/cli.rs`)**：
  - 抽象脱敏模式枚举 `MaskStyle`：`Masking`（保留首尾星号掩码 `张*三`）与 `Redaction`（硬抹除 `████`）；
  - `desensitize_plain_text` 与多格式导出支持按选定策略执行替换；
  - `/api/documents/{id}/desensitize` 接口支持 `style` 查询参数及 POST 结构体；
  - CLI `sensidoc mask` 增加 `--style` 参数支持（`masking` / `redaction`）。
- [x] 98.2 **原生 Office/PDF XML 容器硬抹除支持 (`src/desensitizer.rs`)**：
  - 确保跨 XML `<w:t>`、`<a:t>` 节点及 PDF 内容流支持等长字符硬抹除，不破坏文档原有排版结构。
- [x] 98.3 **设置面板增加脱敏模式选项与全局偏好持久化 (`web/index.html`, `web/app.js`, `web/style.css`)**：
  - 设置面板中增加「脱敏策略」配置卡片，支持切换默认脱敏模式（星号掩码 vs 块状硬抹除）；
  - 偏好自动持久化至 `localStorage`，作为全局默认设置生效。
- [x] 98.4 **中间面板工具栏新增“脱敏”拨杆按钮与即时脱敏渲染 (`web/index.html`, `web/app.js`, `web/style.css`)**：
  - 工具栏拨杆由 `[渲染] [源码]` 扩展为 `[渲染] [脱敏] [源码]`，保持 2 个汉字极简对齐；
  - 激活“脱敏”视图时，根据设置面板的默认策略（掩码 vs 硬抹除）即时打码渲染，点击打码项无缝联动审计列表。
- [x] 98.5 **前端脱敏导出交互适配与参数透传 (`web/app.js`)**：
  - 右下角「脱敏导出 ▼」在请求后端导出时根据当前选定脱敏模式透传 `style` 参数。
- [x] 98.6 **端到端测试与全格式导出回归验证 (`src/desensitizer.rs`)**：
  - 编写并执行单元测试，验证纯文本、Markdown、DOCX、XLSX、PPTX、PDF 在两种模式下的脱敏正确性。
---


### 📝 开发记录与进度
- *2026-09-18*：由 todo.md 自动化同步生成。当前完成度: [6/6]。
