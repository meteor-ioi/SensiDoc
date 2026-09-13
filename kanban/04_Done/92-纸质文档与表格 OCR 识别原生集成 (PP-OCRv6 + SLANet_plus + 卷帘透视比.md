---
id: phase-92
title: 纸质文档与表格 OCR 识别原生集成 (PP-OCRv6 + SLANet_plus + 卷帘透视比对)
priority: medium
tags: [phase, ocr]
created: 2026-09-13
---

# 纸质文档与表格 OCR 识别原生集成 (PP-OCRv6 + SLANet_plus + 卷帘透视比对)

### 📌 任务概述
阶段九十二：纸质文档与表格 OCR 识别原生集成 (PP-OCRv6 + SLANet_plus + 卷帘透视比对) (已完成) · [🔗 对话跳转](conversation://6efdb7c6-eaca-4a22-8acc-97baeea17ede)

### 📋 子任务清单 (Checklist)
- [x] 92.1 **基础设施与 ONNX 模型按需下载器 (`Cargo.toml`, `src/paths.rs`, `src/model_manager.rs`, `src/main.rs`)**：
  - 集成 Rust `ort 2.x`（ONNX Runtime 原生绑定）、`ndarray`、`image`、`imageproc` 依赖；
  - `src/paths.rs` 增加 `get_ocr_models_dir()`、4 大组件文件名常量与 `get_ocr_status()` / `is_ocr_ready()` 检测；
  - `src/model_manager.rs` 增加 OCR 专属模型包（`PP-OCRv6_det_small.onnx`, `PP-OCRv6_rec_medium.onnx`, `slanet-plus.onnx`, `ppocrv6_dict.txt`）下载支持，复用 ModelScope RapidAI 官方直链、HTTP Range 断点续传与取消清理；
  - `src/main.rs` 注册 `/api/ocr/status`、`/api/ocr/download`、`/api/ocr/cancel`、`/api/ocr/delete` 接口；
  - 跑通 `test_ocr_bundle_metadata` 与 `test_cancel_ocr_download_cleans_cache` 等 28 项单元测试（100% 绿灯通过）。
- [x] 92.2 **纯 Rust 核心 OCR 推理引擎流水线 (`src/ocr/`)**：
  - `src/ocr/preprocessor.rs`：图像标准化、自适应 Letterbox 缩放与 NCHW 张量组装；
  - `src/ocr/text_detector.rs`：基于 `PP-OCRv6_small_det` (RepLKFPN) 的文本行定位与二值化轮廓提取；
  - `src/ocr/text_recognizer.rs`：基于 `PP-OCRv6_medium_rec` (LightSVTR) 的文本切片推理与 50 种语言统一字典 CTC 解码；
  - `src/ocr/table_structure.rs`：基于 `slanet-plus` 的无线/复杂表格 HTML 骨架预测与 Cell BBox 坐标回归；
  - `src/ocr/matcher.rs`：基于空间几何中心点包含与 Coverage 面积比的单元格-文本拓扑关联匹配器；
  - `src/ocr/markdown_builder.rs`：结构化表格转标准 GFM Markdown / 兼容 HTML `<table>`，并自动规整单据头尾键值对；
  - `src/ocr/mod.rs`：对外暴露 `OcrEngine` 统一入口并编写单元测试，32 项测试全部通过。
- [x] 92.3 **转换引擎多格式分流与扫描版 PDF 提取 (`src/converter.rs`, `src/main.rs`)**：
  - `DocConverter::convert_bytes` 新增图片拦截（`.jpg/.jpeg/.png/.bmp/.webp/.tiff` 及魔数探测）直接分流至 `OcrEngine`；
  - 针对 `.pdf`：当 `anydoc` 提取字符数过少（纯扫描件）时，利用 `lopdf` 提取内嵌图像流并按页聚合 OCR 输出；
  - 提供原图访问接口 `GET /api/documents/{id}/file`，支持内联访问与无损展示。
  - 跑通 `test_is_image_detection` 与 `test_image_ocr_routing_when_not_ready`，34 项单元测试 100% 绿灯通过。
- [x] 92.4 **前端设置面板重构与首拖情境引导卡片 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - 设置模态框 Tab 1 顶部集成「纸质单据与密集表格 OCR 引擎」组件卡片（实时展示就绪状态、89.5MB 体积、一键下载/取消/删除与进度条）；
  - 首次将图片拖入软件且未安装模型时，弹出友好的一键下载引导卡片（`#ocrPromptModal`），支持原地流式展示进度并在下载完成后自动恢复识别处理。
- [x] 92.5 **方案二：视界拨杆与卷帘透视比对视图落地 (`web/index.html`, `web/style.css`, `web/app.js`)**：
  - 中间主舞台工具栏增加智能四态视界拨杆：`[渲染] | [卷帘比对] | [原图] | [源码]`（图片文档自适应激活）；
  - 基于 CSS `clip-path` 与 `var(--curtain-pos)` 实现 60fps 硬件加速自由左右拖拽垂直卷帘滑轨，底层为原始扫描件、顶层为结构化表格；
  - 接入双向滚动联动同步，敏感词高亮标注无缝映射至卷帘顶层，点击单元格敏感词平滑联动右侧审计面板。
- [x] 92.6 **全链路端到端功能验证与回归测试**：
  - 真实跑通 ModelScope 官方源静默按需下载（94.39MB 完整套件就绪）；
  - 端到端真实单据图像 OCR 提取测试通过（原生耗时 ~700ms，生成语义化 HTML/Markdown）；
  - 原图内联访问接口 `GET /api/documents/{id}/file` 验证无损可用；
  - 全部 34 项 Cargo 单元测试 100% 绿灯通过，静态资源版本升级为 `v1.3.0`。
- [x] 92.7 **表格 HTML 骨架与单元格对齐修复 + 文档列表即时入库与状态机扩展 (`src/ocr/`, `web/`)**：
  - **表格拓扑与词表对齐**：彻底排查并对齐 SLANet_plus 官方 50-token 词表，修复因词表偏移导致的 `<tr>`/`<td>` 解码紊乱；
  - **8点坐标与等比缩放修复**：重构 `TableStructurePredictor`，正确支持 8 点多边形 `[min, max]` 提取，并按 `max_dim` 正确映射回原图物理像素坐标；
  - **无畸变预处理与表格组装**：预处理改为按最长边等比例缩放并在 488x488 补 0，对齐 RapidTable 标准；重构 `MarkdownBuilder` 单元格注入逻辑，无 span 自动降级为标准 GFM 管道表格，有合并单元格输出语义化 HTML 表格；
  - **文档列表即时呈现（乐观入库）**：上传文件（图片/文档）瞬间立即插入左侧文件列表并高亮选中，中间视图呈现优雅的骨架加载动画与提示；
  - **文档生命周期状态机扩展**：状态药丸标签扩展支持 `OCR识别中...`（带呼吸脉冲）、`解析中...`、`待提取`、`已审计` 与 `解析失败`，转换完成后无缝切换为正式状态。
- [x] 92.8 **长图海报核心表格单调拓扑对齐与非表格正文段落解包 (`src/ocr/table_structure.rs`, `src/ocr/matcher.rs`, `src/ocr/markdown_builder.rs`)**：
  - **核心多列表格智能识别**：自动探测并提取连续多列 (`cols >= 2`) 的核心表格网格，剥离长图海报顶部标题与底部说明文字的伪 `colspan` 单元格包裹；
  - **行级单调拓扑对齐 (Row-level Monotonic Topological Alignment)**：采用中位数 $y$ 确定稳健表格上下界，将表格候选文本行与核心表格结构行进行单调一对一对应，彻底解决行重叠（Ghost Row）导致的空行与数据合并错位；
  - **单元格精准水平映射与多行换行保护**：同行文本按水平列序一对一注入单元格；单元格内多行垂直文本用 `<br>` 连接并在 HTML 转义中保留；
  - **表外正文与章节小标题自然 Markdown 流化**：表格上方的单据大标题与分类提取为一级/二级标题；表格下方的流程图与申请规则等条款，智能提取为 `### 小节标题` 并逐行渲染为 Markdown 列表项，彻底根治换行丢失与文本压扁问题；
  - **全量测试通过**：37 项单元测试 100% 绿灯，端到端长图海报 (`售前激励政策.jpeg`) 真实提取验证完美无误。
- [x] 92.9 **扫描版/图片型 PDF 智能路由引擎与密集文本框全序排序修复 (`src/converter.rs`, `src/ocr/text_detector.rs`, `src/ocr/matcher.rs`, `src/ocr/markdown_builder.rs`)**：
  - **根本原因诊断**：针对多页扫描版账单 PDF (`CHAY DA IMPORT-C1-MAY-2026-$ 8,545.18.pdf`)，anydoc 明确抛出无文本层扫描件异常；但在路由至 OCR 推理时，底层文本检测器原有的阅读顺序排序函数因阶梯状坐标分布违反了数学全序（Total Order violation），导致 Rust 标准库排序算法 panic 崩溃；且未就绪时未返回 `OCR_NOT_READY` 提示；
  - **严格数学全序重构**：`text_detector.rs` 与 `matcher.rs` 彻底重构为基于行分桶与 IEEE 浮点严格 `total_cmp` 的排序逻辑，彻底根治 driftsort panic 崩溃；
  - **多页扫描版 PDF 自动路由闭环**：
    - `converter.rs` 新增智能探测：当 anydoc 提取字符极少或明确提示 Scanned 时，自动无缝判定为图片类型 PDF 并路由至 OCR；
    - 模型未就绪时精准返回 `OCR_NOT_READY`，前端立即唤起一键按需下载；
- [x] 92.10 **扫描版 PDF 页面旋转规范自适应与高精度文本/表格识别修复 (`src/converter.rs`, `src/ocr/mod.rs`, `src/main.rs`)**：
  - **根本原因诊断**：参考 `industry_PDF` 实践并深入底层 PDF 字典结构分析发现，该 4 页扫描件内部内嵌的原始扫描图像尺寸为横向 (`2340x1654`)，但在 PDF 页面标准中标记了 `/Rotate 270`（逆时针 90° 倒向）。旧代码直接提取原始 DCT 流送入 OCR，导致文本检测器将纵向文本切成细碎横条，识别出乱码字符碎片（如 `33 商 I i 原`、`0 n 7` 等），且 SLANet 强行将横向切片预测为混乱表格；而 `industry_PDF` 通过 MuPDF 渲染时自动执行了旋转与 DPI 转换；
  - **PDF 旋转属性继承与自动矫正**：
    - 在 `src/converter.rs` 中新增 `get_page_rotation`，严格遵从 ISO 32000-1 规范，支持递归溯源 `/Parent` 树继承的 `/Rotate` 属性（支持 90°、180°、270° 等任意倍数角度）；
    - 提取内嵌图像后，智能根据页面角度自动调用 `img.rotate270()` 等精准矫正为直立人眼阅读方向；同时过滤小于 100x100 的小图标/噪点；
  - **动态图像直传与推理优化**：
    - `src/ocr/mod.rs` 新增 `OcrEngine::recognize_image(&DynamicImage)` 接口，扫描件解码旋转后直接在内存中传递给检测器与识别器，免去重复 JPEG 编解码开销；
    - `src/main.rs` 的 `/api/convert` 接口使用 `tokio::task::spawn_blocking` 执行计算密集型 OCR 推理，彻底避免大文档推理对 Tokio 异步运行时的事件饥饿；
  - **全量实机验证**：
    - 矫正后全文档 4 页端到端识别准确率极大提升：
      - 第 1 页：检测出 251 个文字区域，完整准确提取出 `SUPERL (CAMBODIA) CO., LTD.`、`PAYMENT REQUISITION FORM`、各项发票单号（`2605011672` 等）与明细金额；
      - 第 2 页：检测出 51 个文字区域，完整提取出英文审批邮件链（`From: Cyrus Yu`、`Approved` 等）；
      - 第 3 & 4 页：检测出 168 与 196 个文字区域，完整提取出物流对账单（`CHAY DA LOGISTICS CO., LTD.`）与月度明细表格；
    - 所有 38 项 Cargo 单元测试 100% 绿灯通过，服务稳定运行于 3000 端口。

---

### 🔗 关联实施计划
- [对话跳转](conversation://6efdb7c6-eaca-4a22-8acc-97baeea17ede)

### 📝 开发记录与进度
- *2026-09-13*：由 todo.md 自动化同步生成。当前完成度: [10/10]。
