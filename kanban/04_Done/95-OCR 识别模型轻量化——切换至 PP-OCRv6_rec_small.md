---
id: phase-95
title: OCR 识别模型轻量化——切换至 PP-OCRv6_rec_small
priority: medium
tags: [phase, ocr, ai]
created: 2026-09-17
---

# OCR 识别模型轻量化——切换至 PP-OCRv6_rec_small

### 📌 任务概述
阶段九十五：OCR 识别模型轻量化——切换至 PP-OCRv6_rec_small (已完成)

### 📋 子任务清单 (Checklist)
- [x] 95.1 **路径与模型平滑回退设计 (`src/paths.rs`)**：
  - 将默认识别模型文件名更新为 `PP-OCRv6_rec_small.onnx`；
  - `get_ocr_rec_path()` 实现向后兼容回退：优先加载更轻量的 `small` 模型，若本地仅有旧版 `medium` 模型则平滑回退，保障老用户无缝过渡。
- [x] 95.2 **下载器规格与校验重塑 (`src/model_manager.rs`, `scripts/download_ocr_models.sh`)**：
  - 更新 `OCR_DOWNLOAD_SPECS` 中识别模型配置为 `PP-OCRv6_rec_small.onnx` (体积由 ~76.6MB 骤降至 ~21.2MB，OCR 全套总包由 ~94MB 瘦身至 ~39MB)；
  - 对齐 ModelScope RapidAI 官方直链与对应字典路径；
  - 完善 `delete_ocr_bundle` 兼容清理旧版残留 `PP-OCRv6_rec_medium.onnx`；
  - 更新 `test_ocr_bundle_metadata` 断言范围（35MB ~ 45MB）。
- [x] 95.3 **用户交互与提示文案同步更新 (`src/ocr/mod.rs`, `src/converter.rs`, `web/index.html`, `web/app.js`)**：
  - 前后端所有 OCR 模型套件大小提示统一对齐为 `~39MB`；
  - 设置面板 OCR 介绍更新为「PP-OCRv6 轻量极速版 (Small Det + Small Rec)」。
- [x] 95.4 **全量编译与端到端测试闭环**：
  - 单元测试与端到端表格 OCR 推理测试全部通过，推理时延与内存占用显著下降。

---


### 📝 开发记录与进度
- *2026-09-17*：由 todo.md 自动化同步生成。当前完成度: [4/4]。
