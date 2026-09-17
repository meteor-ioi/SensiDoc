---
id: phase-96
title: OCR 底座质量加固与架构技术债收敛
priority: high
tags: [phase, ocr, backend]
created: 2026-09-17
---

# OCR 底座质量加固与架构技术债收敛

### 📌 任务概述
阶段九十六：OCR 底座质量加固与架构技术债收敛 (P1) (已完成) · [🔗 实施计划](plan/OCR_TECH_DEBT_REMEDIATION_PLAN.md)

### 📋 子任务清单 (Checklist)
- [x] 96.1 **依赖与私有路径治理 (Task 1 & Task 2) (`anyocr`, `Cargo.toml`)**：
  - 剥离 `anyocr/src/asset/downloader.rs` 中遗留的开发者本地绝对路径，改为标准工作区与缓存目录探测并推送到 GitHub (`8525874b`)；
  - 清理 SensiDoc `Cargo.toml` 中冗余声明的 `ort`, `ndarray`, `imageproc` 直接依赖，仅保留直接引用的 `image` crate，消除版本漂移与重复编译风险；
  - 跑通 `anyocr` 13 项单元测试与 SensiDoc 全量 32 项单元测试。
- [x] 96.2 **模型下载与就绪校验引入 SHA-256 强校验与 Commit 版本锁定 (Task 3) (`src/model_manager.rs`, `src/paths.rs`, `scripts/download_ocr_models.sh`)**：
  - 在 `OcrDownloadFileSpec` 中为 4 项 OCR 组件（Det、Rec、Table、Dict）补充权威 SHA-256 哈希指纹；
  - 将 ModelScope 下载 URL 由浮动 `master` 分支锁定为**不可变 Git Commit Revision**（`RapidOCR: 7d07816`、`RapidTable: a484f11`），杜绝云端静默突变导致哈希失效；
  - 在 `src/model_manager.rs` 实现 `verify_file_sha256()` 校验函数；在已存在组件复用与临时下载流 `.part` 转正前执行强哈希校验，遇截断或哈希不匹配自动阻断并删除损坏缓存；
  - 校准 `src/paths.rs` 中各组件大小就绪阈值（Det > 8MB, Rec > 18MB, Table > 6MB, Dict > 50KB）；
  - 新增 `test_verify_file_sha256` 与 `test_ocr_specs_sha256_against_local_files_if_exist`，验证 4 款本地模型 SHA-256 与定义完全匹配。
- [x] 96.3 **全局并发锁中毒 (PoisonError) 显式告警与自动恢复 (Task 4) (`src/ocr/mod.rs`)**：
  - 封装 `acquire_ocr_state()` 核心助手函数，统一拦截并自愈 `PoisonError`；
  - 遭遇锁中毒时记录显式 `tracing::error!` 告警，执行 `OCR_STATE.clear_poison()` 并经由 `into_inner()` 安全重置状态，杜绝非可重入锁自死锁与进程瘫痪；
  - 增加 `test_ocr_state_poison_recovery` 单元测试模拟异常线程中断，验证锁自愈与服务自动恢复 100% 通过。
- [x] 96.4 **模型体积指标 API 动态驱动化与前后端文案解耦 (Task 5) (`src/paths.rs`, `src/converter.rs`, `src/ocr/mod.rs`, `web/app.js`, `web/index.html`)**：
  - `OcrStatus` 接口扩展提供 `expected_total_bytes: u64` 动态数据字段；
  - 前端 `web/app.js` 统一封装 `formatModelSize()` 辅助函数，数据驱动渲染所有界面体积文案；
  - 统一收敛后端错误返回文案，彻底消除硬编码 `~39MB` 文本分散维护的技术债。
- [x] 96.5 **构建与全量回归测试闭环**：
  - `cargo check`、`cargo build` 零报错，编译干净稳定；
  - 全量 35 项单元测试 100% 绿灯通过。

---

### 🔗 关联实施计划
- [实施计划](plan/OCR_TECH_DEBT_REMEDIATION_PLAN.md)

### 📝 开发记录与进度
- *2026-09-17*：由 todo.md 自动化同步生成。当前完成度: [5/5]。
