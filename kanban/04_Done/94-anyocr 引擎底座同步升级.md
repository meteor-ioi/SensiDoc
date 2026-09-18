---
id: phase-94
title: anyocr 引擎底座同步升级
priority: medium
tags: [phase, ocr, backend]
created: 2026-09-18
---

# anyocr 引擎底座同步升级

### 📌 任务概述
阶段九十四：anyocr 引擎底座同步升级 (已完成)

### 📋 子任务清单 (Checklist)
- [x] 94.1 **同步最新 anyocr 上游版本 (`Cargo.lock`)**：
  - 更新 anyocr 锁版本至最新 commit (`11f8d42`)；
  - 继承上游 CoreML 动态形状优化、多核 CPU SIMD、超大图 2560px 自适应视窗钳制保护与原图物理坐标还原。
- [x] 94.2 **适配 EngineConfig 新增配置字段 (`src/ocr/mod.rs`)**：
  - 适配并对齐 `max_dimension: Some(2560)` 配置项，防止极端高分扫描件内存膨胀。
- [x] 94.3 **编译与 OCR 单元测试验证**：
  - `cargo check` 与 `cargo build` 顺利编译；
  - OCR 端到端推理与引擎生命周期卸载测试全部通过（100% 绿灯）。

---


### 📝 开发记录与进度
- *2026-09-18*：由 todo.md 自动化同步生成。当前完成度: [3/3]。
