# OCR 底座质量加固与技术债收敛实施计划 (OCR_TECH_DEBT_REMEDIATION_PLAN.md)

> **计划背景**：  
> 依据 [`evaluations/PHASE_92_94_95_OCR_EVALUATION.md`](../evaluations/PHASE_92_94_95_OCR_EVALUATION.md) 资深工程师架构与代码评审结果，针对阶段 92、94、95 遗留的 5 项关键潜在问题（依赖冗余、硬编码私有路径、缺乏哈希完整性校验、锁中毒容错不显式、前后端体积文案散落硬编码）制定系统性工程加固实施计划。

---

## 一、 实施目标与收益指标

```
[ 依赖与配置治理 ]           [ 运行时安全与完整性 ]           [ 用户交互与维护性 ]
 ├── 剥离 anyocr 私有绝对路径   ├── 引入 SHA-256 哈希指纹校验   ├── API 动态提供 total_bytes
 └── 净化 Cargo.toml 冗余依赖  └── 增加全局 Mutex 锁中毒自愈   └── 前端动态格式化渲染体积文案
```

- **编译构建净化**：移除 SensiDoc 顶层无直接引用的 `ort`、`ndarray`、`imageproc`，统一收敛到底层 `anyocr`；
- **传输与加载安全**：下载器与就绪检测引入 SHA256 强校验，彻底杜绝弱网截断引发的残缺权重加载；
- **并发状态自愈**：消除 `OCR_STATE` 锁中毒后的静默容错盲区，增加显式错误日志与上下文重置；
- **配置与文案解耦**：消除前后端 8 处硬编码 `~39MB`，实现数据驱动的动态界面渲染。

---

## 二、 任务细化与实施步骤

### 任务 1：剥离 `anyocr` 内部私有绝对路径 (P1)
- **问题现状**：`anyocr/src/asset/downloader.rs:60` 硬编码了本地机器私有绝对路径 `/Users/icychick/Projects/SensiDoc-ocr/models/ocr`。
- **改动方案**：
  1. 移除硬编码路径，规范化搜索路径为：`ANYOCR_CACHE_DIR` 环境变量 -> 本地 `models/ocr` -> 系统级缓存目录 (`~/.cache/anyocr/models`)；
  2. 若在开发调试阶段需快捷探测宿主项目模型目录，改用 `cfg(debug_assertions)` 条件编译并在文档中清晰注明；
  3. 提交至 `anyocr` 主干并在 SensiDoc 侧重新锁定 commit。

### 任务 2：SensiDoc `Cargo.toml` 冗余依赖清理 (P1)
- **问题现状**：阶段 93 将 OCR 底层抽象剥离至 `anyocr` 后，SensiDoc 顶层 `Cargo.toml` 依然直接声明了 `ort`、`ndarray`、`imageproc`，但在 `src/` 中全无直接调用。
- **改动方案**：
  1. 从 `Cargo.toml` 中安全移除 `ort`、`ndarray`、`imageproc`；
  2. 仅保留用于图像解码与旋转校正的 `image` crate；
  3. 执行 `cargo check` 与 `cargo build` 验证编译通过，消除符号冗余与多层依赖版本冲突风险。

### 任务 3：OCR 模型文件引入 SHA256 哈希指纹强校验 (P1)
- **问题现状**：`model_manager.rs` 仅根据 Content-Length 和字节大小粗筛，网络截断或 CDN 缓存残损难以侦测。
- **改动方案**：
  1. 在 `src/model_manager.rs` 的 `OcrDownloadFileSpec` 中增加 `sha256: &'static str` 校验指纹：
     - `PP-OCRv6_det_small.onnx`: `090f04abcd9d9a7498bc4ebf677e4cb9bdce1fe4197ddb7e529f1ef44e1ff94f`
     - `PP-OCRv6_rec_small.onnx`: `6f327246b50388f3c176ae304bd95767ea6dc0c9ae92153ef8cbe210b3c14884`
     - `slanet-plus.onnx`: `d57a942af6a2f57d6a4a0372573c696a2379bf5857c45e2ac69993f3b334514b`
     - `ppocrv6_dict.txt`: `b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d`
  2. 在流式写入并重命名文件前，执行实时 SHA256 校验；若哈希不匹配则删除临时缓存并抛出明确校验异常；
  3. 在 `src/paths.rs` 的 `get_ocr_status()` 中校验 `rec_ready` 等大小判定阈值，校准为更贴近真实规格（`> 18MB`）。

### 任务 4：全局 `OCR_STATE` 锁中毒 (PoisonError) 显式告警与自愈重置 (P2)
- **问题现状**：`src/ocr/mod.rs` 中使用全局静态 `OCR_STATE: Mutex<Option<OcrStateHolder>>`，遇到 `PoisonError` 时静默 `continue` 或返回默认值，隐藏跨线程崩溃痕迹。
- **改动方案**：
  1. 统一封装锁获取助手函数 `acquire_ocr_state()`：
     - 若遭遇 `PoisonError`，记录 `tracing::error!("OCR_STATE 锁已中毒，正在重置状态上下文以恢复服务...")`；
     - 通过 `poison_err.into_inner()` 提取锁内部数据并重新初始化 `OcrStateHolder`，实现自动容灾愈合；
  2. 保证单元测试与长时间运行下的高鲁棒性。

### 任务 5：模型体积文案由后端 API 动态驱动，彻底消除硬编码 (P2)
- **问题现状**：`~39MB` 硬编码分散在 Rust 错误信息、HTML 弹窗与 JS 渲染逻辑等 8 个位置，规格调整时极易遗漏。
- **改动方案**：
  1. `src/paths.rs` 的 `OcrStatus` 结构体新增 `expected_total_bytes: u64` 字段，直接返回 `crate::model_manager::get_ocr_total_expected_bytes()`；
  2. 前端 `web/app.js` 统一封装 `formatBytes(bytes)` 辅助函数，在更新状态药丸 (`ocrStatusPill`)、删除确认弹窗 (`deleteOcrBundle`) 与未就绪引导弹窗 (`ocrPromptModal`) 时动态渲染计算结果（如 `~39.0 MB`）；
  3. 将 Rust 错误提示收敛为通用文案（如 `“请先在设置面板中下载轻量 OCR 模型套件”`），去除写死数字。

---

## 三、 验证基准与回归矩阵

| 验证项 | 验证命令 / 操作 | 验收标准 |
| :--- | :--- | :--- |
| **Cargo 编译** | `cargo check && cargo build` | 移除 `ort/ndarray/imageproc` 后编译 0 报错、0 警告 |
| **单元测试** | `cargo test --bin sensidoc` | 32 项自动化测试 100% 绿灯通过 |
| **哈希校验** | 模拟下载中断/残缺文件 | 校验失败时自动阻断并清理，绝不落盘损坏模型 |
| **锁中毒自愈** | 模拟线程内故意 panic 穿透 | 下一次推理能感知错误日志并自愈恢复，不发生死锁 |
| **前端动态渲染** | 切换模型就绪/未就绪状态 | 页面体积文案从 API 动态解析显示为 `~39.0 MB` |
