---
id: card-fa3c414efd38
title: 13-扫描件与图片型PDF的OCR像素级光栅化遮盖
priority: low
tags:
  - research
  - ocr
  - pdf
  - computer-vision
created: 2026-09-08
---
# 扫描件与图片型 PDF 的 OCR 像素级光栅化遮盖

### 📌 任务目标
针对纯扫描件或图片型 PDF 等无矢量文字层的特殊文档，预研并集成纯本地离线 OCR 定位引擎，获取敏感词图像像素坐标（Bounding Box），并在图像层实现像素级矩形抹除/高斯模糊，重新压制为高保真脱敏 PDF。

### 📋 子任务清单 (Checklist)
- [ ] 预研纯本地离线轻量 OCR 引擎（如 ONNX Runtime + PP-OCR / RapidOCR）
- [ ] 实现 PDF 页面图像像素坐标与文本位置精确映射算法
- [ ] 实现图像像素级黑色矩形覆盖与高斯模糊遮盖打码器
- [ ] 重新压制生成高保真光栅化脱敏 PDF

### 📝 开发记录与进度
- *2026-09-08*：卡片放入需求池，作为后续演进方向。

### 🔗 关联文件 / 依赖
- 脱敏模块：[`src/desensitizer.rs`](file:///Users/icychick/Projects/SensiDoc/src/desensitizer.rs)
