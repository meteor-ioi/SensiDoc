# SensiDoc 纸质文档与表格识别方案（方案 B：Rust + ONNX Runtime 原生嵌入）

## 1. 方案背景与目标

### 1.1 现状与痛点
在 SensiDoc 当前版本（v1.3.0）中：
* 文档转换引擎位于 `src/converter.rs`，基于 `anydoc` 解析电子文档（`.docx`, `.xlsx`, `.pdf`, `.csv`, `.txt`, `.md`）。
* **核心短板**：无法直接处理**纸质翻拍图片（`.jpg`, `.png`）**以及**扫描版（纯图片无文字层）的 PDF**。若用户导入发票、送货单、入库单等扫描件，系统无法提取其中的内容。
* **业务需求**：支持将纸质单据（含印刷体、手写修改内容、密集表格）精准还原为结构化的 Markdown / HTML 表格，并无缝复用 SensiDoc 原有的文档高亮、敏感信息审计脱敏与本地大模型分析能力。

### 1.2 方案 B 核心设计原则
* 🦀 **纯 Rust 原生嵌入**：基于 Rust `ort` (ONNX Runtime) 运行推理，**完全摆脱 Python 运行时依赖**。
* 📦 **轻量与单一二进制**：保持 SensiDoc 原生客户端“零环境配置、开箱即用”的特性，模型总权重大约 25MB~30MB（INT8 量化后约 15MB）。
* ⚡ **高吞吐与端侧低时延**：CPU 多线程推理单张单据在 200ms ~ 800ms 内完成，支持 macOS (CoreML) 与 Windows (DirectML) 硬件加速。
* 🎯 **精准物理对齐**：先还原表格拓扑（行列与合并单元格），再填入文字，彻底解决纯视觉大模型直接读图时的“串行、错行与手写数字幻觉”。

---

## 2. 总体架构设计

```text
               [ 纸质翻拍图片 (JPG/PNG) / 扫描版 PDF 页面 ]
                                    │
                                    ▼
┌───────────────────────────────────────────────────────────────────────────┐
│ 1. 输入路由与预处理 (src/converter.rs & src/ocr/preprocessor.rs)          │
│   ├── 文件魔数嗅探：命中图片格式或检测到纯图片 PDF                          │
│   └── 图像标准化：EXIF 旋转纠正、自适应缩放 (Letterbox)、归一化 Tensor    │
└───────────────────────────────────┬───────────────────────────────────────┘
                                    │
                  ┌─────────────────┴─────────────────┐
                  ▼                                   ▼
┌───────────────────────────────────┐   ┌───────────────────────────────────┐
│ 2. 表格结构预测 (SLANet ONNX)     │   │ 3. 文本检测与识别 (PP-OCRv4 ONNX) │
│   ├── 模型：SLANet 表格结构模型   │   │   ├── 检测：DBNet 文字行检测框    │
│   ├── 输出：表格 HTML 骨架标签    │   │   └── 识别：SVTR/CRNN 手写+印刷   │
│   │   (<tr>, <td>, rowspan/colspan)│   │       高精度文字与置信度识别      │
│   └── 产出每个单元格的坐标框      │   │   └── 产出文本内容与多边形坐标    │
│       (Cell BBoxes: [x1,y1,x2,y2])│   │       (Text, Score, BBox)         │
└─────────────────┬─────────────────┘   └─────────────────┬─────────────────┘
                  │                                       │
                  └─────────────────┬─────────────────────┘
                                    ▼
┌───────────────────────────────────────────────────────────────────────────┐
│ 4. 空间拓扑匹配器 (Cell-Text Spatial Matcher)                             │
│   ├── 计算文本多边形中心点 / IOU 与各单元格 BBox 的几何包含关系           │
│   ├── 单元格内文字换行重构与排序（Top-Down, Left-to-Right）              │
│   └── 特殊特征标记：手写体标注（如标记为 `[手写: 980]`）                  │
└───────────────────────────────────┬───────────────────────────────────────┘
                                    │
                                    ▼
┌───────────────────────────────────────────────────────────────────────────┐
│ 5. Markdown / HTML 表格序列化 (src/ocr/markdown_builder.rs)               │
│   ├── 规范化单元格转义，生成标准 GitHub Flavored Markdown (GFM) 表格     │
│   ├── 若含复杂跨行合并，生成兼容型 HTML <table> 块                        │
│   └── 将单据头/尾键值信息与表格整合为统一的 Markdown 文档流              │
└───────────────────────────────────┬───────────────────────────────────────┘
                                    │
                                    ▼
┌───────────────────────────────────────────────────────────────────────────┐
│ 6. 进入 SensiDoc 既有主流水线 (无缝承接)                                  │
│   ├── Web 前端渲染预览与坐标高亮引导                                      │
│   ├── 本地 GGUF / 云端 LLM 进行敏感信息与字段抽取                         │
│   └── 敏感数据脱敏导出 (RFC 4180 CSV / Markdown 副本)                      │
└───────────────────────────────────────────────────────────────────────────┘
```

---

## 3. 技术选型与依赖规划

### 3.1 Rust 依赖库 (Cargo.toml)

```toml
[dependencies]
# ONNX Runtime 跨平台推理引擎 (启用 download-binaries 方便开箱即用)
ort = { version = "2.0.0-rc.9", features = ["download-binaries", "copy-dylibs"] }

# 纯 Rust 图像编解码与像素变换
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "bmp"] }
imageproc = "0.25"

# PDF 页面图像抽取与光栅化
# 可利用已有 lopdf 提取图片，或集成轻量 pdfium-render 进行无失真渲染
```

### 3.2 ONNX 模型配置与量化规格

全部选用经过充分验证的轻量级 ONNX 权重：

| 模块名称 | 基础模型 | ONNX 尺寸 (FP32) | INT8 量化尺寸 | 职责与功能 |
| :--- | :--- | :--- | :--- | :--- |
| **表格结构识别** | `ch_ppstructure_mobile_v2.0_SLANet` | ~8.5 MB | **~2.8 MB** | 预测表格逻辑结构（HTML 语法标签）与每个单元格坐标 |
| **文本区域检测** | `ch_PP-OCRv4_det_infer` | ~4.6 MB | **~1.5 MB** | 定位文档内所有单行印刷体与手写体文字区域 |
| **文本文字识别** | `ch_PP-OCRv4_rec_infer` (微调手写版) | ~12.8 MB | **~4.2 MB** | 识别多边形切片内的汉字、英文字母、符号与手写数字 |
| **版面区域分析** (选配) | `picodet_lcnet_layout` | ~7.2 MB | **~2.4 MB** | 分辨页面中的表格、正文、标题、印章与图表区域 |

> **部署方式**：模型存放在 SensiDoc `models/ocr/` 目录下。初次启动可随 Release 安装包内置，或通过已有的 `model_manager.rs` 实现后台按需静默下载。

---

## 4. 核心模块设计与实现细节

### 4.1 模块代码目录规划
在 `SensiDoc/src/` 下新增 `ocr` 子模块：
```text
src/
├── ocr/
│   ├── mod.rs               # 对外统一暴露 OcrEngine 入口
│   ├── preprocessor.rs      # 图像归一化、Resize、Letterbox 变换
│   ├── table_structure.rs   # SLANet ONNX 推理、HTML 词表反序列化
│   ├── text_detector.rs     # DBNet ONNX 文本行定位后处理
│   ├── text_recognizer.rs   # CRNN/SVTR CTC 解码 (含字典映射)
│   ├── matcher.rs           # 单元格与文本的空间拓扑对齐算法
│   └── markdown_builder.rs  # HTML/表格数据转标准 Markdown
├── converter.rs             # 改造：增加图片与扫描版 PDF 拦截分流
```

### 4.2 空间拓扑匹配算法 (Matcher)
表格识别的关键在于**把文字准确塞进对应的单元格**：
1. **坐标归一化**：将检测出的文本框坐标（$B_{text}$）与 SLANet 预测的单元格坐标（$B_{cell}$）映射到相同的图像原始分辨率空间。
2. **中心点包含法则**：
   * 计算文字框几何中心点 $(x_c, y_c)$：
     $$x_c = \frac{x_1 + x_2}{2}, \quad y_c = \frac{y_1 + y_2}{2}$$
   * 判定 $(x_c, y_c)$ 是否落在单元格 $B_{cell}$ 内部。
3. **IOU 面积交并比兜底**：对于倾斜或边缘压线的文本，采用交集面积与文本框面积比：
   $$\text{Coverage} = \frac{\text{Area}(B_{text} \cap B_{cell})}{\text{Area}(B_{text})} \ge 0.5$$
4. **单元格内文字排序**：若一个单元格内有多行文字（如“规格型号”下同时有型号与批次），按 $y$ 轴自上而下、$x$ 轴自左向右排序后通过空格或换行连接。

### 4.3 转换接入点改造 (`src/converter.rs`)

在 SensiDoc 既有的 `DocConverter::convert_bytes` 中进行透明升级：

```rust
// 伪代码实现示意：
impl DocConverter {
    pub fn convert_bytes(filename: &str, bytes: &[u8]) -> Result<String, String> {
        let ext = get_file_extension(filename);

        // 1. 新增：纸质图片文件直接路由至 OCR 表格引擎
        if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "tiff") {
            return crate::ocr::OcrEngine::image_to_markdown(bytes);
        }

        // 2. 针对 PDF：先尝试 anydoc 解析电子文本
        if ext == "pdf" {
            if let Ok(md) = anydoc::to_markdown_bytes(bytes, Format::Pdf) {
                // 如果提取出有效文本则返回；若字符数极少（扫描件），降级到 OCR
                if md.trim().len() > 30 {
                    return Ok(md);
                }
            }
            // 扫描版 PDF：光栅化为页面图片后依次调用 OCR 并合并
            return crate::ocr::OcrEngine::scanned_pdf_to_markdown(bytes);
        }

        // 3. 保持既有 Office 文档与纯文本分支不变
        // ...
    }
}
```

---

## 5. 输出格式规范与样例

### 5.1 规则标准表格（输出标准 GFM Markdown）
对于规则的二维表格，直接输出干净的 Markdown，便于后续 SensiDoc 渲染和正则/LLM 审计：

```markdown
### 采购送货单

- 单据编号: SH-20260310-092
- 送货日期: 2026-03-10
- 供应商: 胜寒工业科技

| 序号 | 零件编码 | 品名规格 | 订单数量 | 实收数量 | 计量单位 | 备注 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 1 | P-99201 | 六角螺母 M6 | 1000 | 1000 | 件 | 包装完好 |
| 2 | P-99208 | 沉头螺钉 M4*12 | 500 | 480 | 件 | 手写缺20件 |
| 3 | S-10022 | 密封橡胶圈 Ø25 | 200 | 200 | 条 | 检验合格 |
```

### 5.2 复杂跨行合并表格（优雅兼容 HTML 标签）
若单据中出现复杂合并单元格（如多行商品对应同一个分类）：
```html
<table>
  <thead>
    <tr><th>类别</th><th>商品编号</th><th>规格</th><th>数量</th></tr>
  </thead>
  <tbody>
    <tr><td rowspan="2">紧固件</td><td>A101</td><td>M8*20</td><td>50</td></tr>
    <tr><td>A102</td><td>M8*30</td><td>80</td></tr>
  </tbody>
</table>
```
> **注**：SensiDoc 的 Markdown 渲染器原生支持内嵌 HTML `<table>`，因此能够实现 100% 完整的样式保真与对齐渲染。

---

## 6. 实施路线图与里程碑

```text
[阶段 1: ONNX 运行环境与模型准备] (约 3-5 天)
  ├── 集成 Rust `ort` 2.x 依赖，验证 macOS (Metal/CoreML) 与 Windows 编译
  └── 导出/下载 SLANet 表格结构模型与 PP-OCRv4 文本识别 ONNX 权重
        │
        ▼
[阶段 2: 核心推理流水线实现] (约 5-7 天)
  ├── 实现图像 Letterbox 预处理与 Tensor 组装
  ├── 实现 SLANet HTML 标签解码与 Cell BBox 提取
  ├── 实现 DBNet + SVTR 文本定位与文字识别
  └── 实现 Cell-Text 空间拓扑关联匹配算法
        │
        ▼
[阶段 3: SensiDoc 主干集成与接入] (约 2-3 天)
  ├── 改造 `src/converter.rs`，接入图片与扫描版 PDF 路由
  ├── 将解析产出的表格格式化为 Markdown / 兼容 HTML
  └── 在 Web 界面验证纸质单据的直接拖拽解析与全文高亮
        │
        ▼
[阶段 4: 场景专项调优与量化压缩] (约 3-4 天)
  ├── 针对真实纸质发票、出入库单进行手写体/印章干扰测试
  └── 完成 ONNX INT8 量化，控制包体积与端侧推理内存占用
```

---

## 7. 方案总结

方案 B 通过在 SensiDoc 中以**纯 Rust + ONNX Runtime** 的方式嵌入轻量级表格与 OCR 模型：
1. **零外部环境依赖**：无需安装 Python、无需配置复杂的系统环境，保持客户端的极简发布；
2. **文档格式全覆盖**：完美补全 SensiDoc 在纸质单据、图片、扫描版 PDF 领域的空白；
3. **架构无缝兼容**：输出的 Markdown 表格直接进入现有的脱敏、审计与大模型提取流水线，最大程度复用了已有代码资产。
