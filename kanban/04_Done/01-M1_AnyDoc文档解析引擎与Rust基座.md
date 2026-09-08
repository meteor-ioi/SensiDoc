---
priority: high
tags: [backend, parser, core]
created: 2026-09-01
---

# M1 - AnyDoc 文档解析引擎与 Rust 基座

### 📌 任务目标
构建 SensiDoc 底层核心框架，基于 Axum / Tokio 构建高性能本地轻量服务，集成 anydoc 跨平台纯代码文档解析引擎，实现 Word (.docx)、Excel (.xlsx)、PowerPoint (.pptx)、PDF (.pdf)、纯文本 (.txt) 及 CSV 等全格式文档秒级提取为统一 Markdown 结构流。

### 📋 子任务清单 (Checklist)
- [x] 基于 Rust / Cargo 搭建工程骨架与模块解耦架构
- [x] 集成 anydoc 文档转换工具链与跨格式自适应探测
- [x] 实现 Markdown 格式标准化规整与跨平台路径管理 (`src/paths.rs`)
- [x] 搭建 Axum Web 服务框架与基础 RESTful API 路由 (`src/main.rs`)
- [x] 编写单元测试验证 DOCX/XLSX/PDF/TXT 转换准确性与鲁棒性

### 📝 开发记录与进度
- *2026-09-01*：完成项目初始化，集成 anydoc 并验证全格式纯文本解析。
- *2026-09-08*：全套单元测试持续 100% 绿灯通过。

### 🔗 关联文件 / 依赖
- 解析转换：[`src/converter.rs`](file:///Users/icychick/Projects/SensiDoc/src/converter.rs)
- 路径与环境：[`src/paths.rs`](file:///Users/icychick/Projects/SensiDoc/src/paths.rs)
- 服务入口：[`src/main.rs`](file:///Users/icychick/Projects/SensiDoc/src/main.rs)
