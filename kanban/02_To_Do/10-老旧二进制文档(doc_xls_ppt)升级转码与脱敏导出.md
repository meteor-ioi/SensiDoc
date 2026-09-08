---
priority: medium
tags: [backend, desensitizer, legacy-formats]
created: 2026-09-08
---

# 老旧二进制文档 (.doc / .xls / .ppt 97-2003) 升级转码与脱敏导出

### 📌 任务目标
解决企业历史资产中大量存在的 Office 97-2003 二进制格式（.doc, .xls, .ppt）脱敏难题。采用纯代码解析二进制流并升级重构为现代 Office OpenXML 格式（.docx, .xlsx, .pptx），实现安全脱敏与格式升级双重收益。

### 📋 子任务清单 (Checklist)
- [ ] 调研与集成 `calamine` + `rust_xlsxwriter`，解析 BIFF8 二进制流并转码导出脱敏 `.xlsx`
- [ ] 基于 `cfb`（复合文档二进制解析器）读取 WordDocument Stream，提取段落并转码导出脱敏 `.docx`
- [ ] 评估轻量提取幻灯片文本框并升级打包为 `.pptx` 的可行性方案
- [ ] 扩展 `Desensitizer::desensitize_document_auto` 实现自动升级路由分发
- [ ] 编写旧版二进制格式脱敏单元测试与回归测试用例

### 📝 开发记录与进度
- *2026-09-08*：创建任务卡片，技术路线已于 `IMPLEMENTATION_PLAN_v2.md` 中规划完毕。

### 🔗 关联文件 / 依赖
- 脱敏引擎：[`src/desensitizer.rs`](file:///Users/icychick/Projects/SensiDoc/src/desensitizer.rs)
- 转换引擎：[`src/converter.rs`](file:///Users/icychick/Projects/SensiDoc/src/converter.rs)
