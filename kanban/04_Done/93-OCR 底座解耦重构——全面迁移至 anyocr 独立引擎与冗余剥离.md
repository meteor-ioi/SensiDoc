---
id: phase-93
title: OCR 底座解耦重构——全面迁移至 anyocr 独立引擎与冗余剥离
priority: medium
tags: [phase, ocr, backend]
created: 2026-09-13
---

# OCR 底座解耦重构——全面迁移至 anyocr 独立引擎与冗余剥离

### 📌 任务概述
阶段九十三：OCR 底座解耦重构——全面迁移至 anyocr 独立引擎与冗余剥离 (已完成) · [🔗 对话跳转](conversation://179180f8-e7e7-45e2-aaf7-aada2d975a7b)

### 📋 子任务清单 (Checklist)
- [x] 93.1 **接入 anyocr 独立感知底座依赖与 Cargo 清理 (`Cargo.toml`)**：
  - 引入 `anyocr = { path = "../anyocr" }` 本地独立路径依赖；
  - 解耦对底层 `ort`、`ndarray`、`imageproc` 等冗余重型依赖的手工维护，统一由 `anyocr` 统一管理与按需抽象。
- [x] 93.2 **核心推理适配代理层重塑 (`src/ocr/mod.rs`)**：
  - 将 `src/ocr/mod.rs` 改造为极简、高并发安全的门面适配器（Facade Pattern），底层全权委托 `anyocr::Engine`；
  - 完整保留原有向后兼容公共类型与接口签名（`OcrBoxItem`, `OcrResult`, `OcrEngine::recognize_bytes`, `recognize_image`, `is_loaded`, `unload`）；
  - 保留并加固 3 分钟空闲自动卸载监控（Tokio 后台轻量轮询检测 `OCR_IDLE_TIMEOUT` 与 `Arc::strong_count == 1`），确保长时间无任务时自动回收内存。
- [x] 93.3 **彻底删除 6 个历史冗余底层模块 (解耦瘦身约 5 万行代码)**：
  - 彻底清理删除 `src/ocr/` 内部不再需要的 6 个历史重型文件：
    - `src/ocr/markdown_builder.rs`
    - `src/ocr/matcher.rs`
    - `src/ocr/preprocessor.rs`
    - `src/ocr/table_structure.rs`
    - `src/ocr/text_detector.rs`
    - `src/ocr/text_recognizer.rs`
  - 业务层与底层深度学习推理引擎实现彻底解耦，维护复杂度大幅降低。
- [x] 93.4 **测试并发安全加固与转换异常熔断 (`src/ocr/mod.rs`, `src/converter.rs`)**：
  - 加固单元测试多线程竞争：引入测试互斥锁与动态轮询重试机制，杜绝并行测试时强引用交叉导致的卸载断言失败；
  - `src/converter.rs` PDF 转换增加 `catch_unwind` 异常捕获，防范第三方 PDF 解析排序违反数学全序时导致的未捕获异常退出，优雅回退至 OCR 图像提取流。
- [x] 93.5 **全链路回归测试与业务验证**：
  - 跑通全部 32 项 Cargo 单元测试与端到端测试（100% 绿灯通过）；
  - 验证敏感信息审计、正则/LLM 抽取、快照落盘、前端原图卷帘对比视图无缝衔接。

---

### 🔗 关联实施计划
- [对话跳转](conversation://179180f8-e7e7-45e2-aaf7-aada2d975a7b)

### 📝 开发记录与进度
- *2026-09-13*：由 todo.md 自动化同步生成。当前完成度: [5/5]。
