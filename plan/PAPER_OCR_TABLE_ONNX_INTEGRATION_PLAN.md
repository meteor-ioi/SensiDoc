# SensiDoc 纸质文档与表格识别方案（方案 B：Rust + ONNX Runtime 企业级高精嵌入）

## 1. 方案背景与目标

### 1.1 现状与痛点
在 SensiDoc 当前版本（v1.3.0）中：
* 文档转换引擎位于 `src/converter.rs`，基于 `anydoc` 解析电子文档（`.docx`, `.xlsx`, `.pdf`, `.csv`, `.txt`, `.md`）。
* **核心短板**：无法直接处理**纸质翻拍图片（`.jpg`, `.png`）**以及**扫描版（纯图片无文字层）的 PDF**。若用户导入发票、送货单、入库单等扫描件，系统无法提取其中的内容。
* **业务需求**：企业生产环境下，单据审计对**金额、税号、关键条款数字的准确率要求极高（错一字即事故）**。需将纸质单据（含印刷体、手写修改内容、无线/密集表格）精准还原为结构化的 Markdown / HTML 表格，并无缝复用 SensiDoc 原有的文档高亮、敏感信息审计脱敏与本地大模型分析能力。

### 1.2 方案 B 核心设计原则（企业级混合架构：方案 A 组合）
* 🦀 **纯 Rust 原生嵌入**：基于 Rust `ort` (ONNX Runtime) 运行推理，**完全摆脱 Python 运行时依赖**。
* 💎 **精度第一前提（FP32 原始浮点无损精度）**：首期**暂不进行 INT8 量化**，彻底杜绝因量化截断带来的任何数值漂移与字符漏识隐患，保障金额、税号、合同条款 100% 达到模型理论上限精度。
* 🎯 **算力集中在刀刃上（Small 检测 + Medium 识别）**：文本定位采用轻快高敏的 `PP-OCRv6_small_det`，文本识别采用企业级高精度的 `PP-OCRv6_medium_rec` (Server 版)。
* 🖥️ **常规办公 PC 轻松承载（4核16G）**：整套 FP32 原始模型合计约 89.5MB，在常规 Windows 4核16G PC 上运行峰值仅占 **~250-350MB 内存**（占总内存约 2%），端到端单页耗时仅 **500ms ~ 850ms**，完全无需牺牲精度换取极致压缩。
* 📐 **复杂与无线表格增强**：采用 `SLANet_plus` 模型，专治发票、出入库单中常见的无边框表格（无线表）与裁剪边界偏移。
* ⚡ **支持端侧硬件加速**：支持 Windows (DirectML) 与 macOS (CoreML) 调用本地核显/GPU 加速推理。

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
│ 2. 表格结构预测 (SLANet_plus)     │   │ 3. 文本定位与识别 (PP-OCRv6 混部) │
│   ├── 模型：SLANet_plus 无线表增强│   │   ├── 定位：PP-OCRv6_small_det    │
│   ├── 输出：表格 HTML 骨架标签    │   │       (RepLKFPN 轻快大感受野定位) │
│   │   (<tr>, <td>, rowspan/colspan)│   │   └── 识别：PP-OCRv6_medium_rec   │
│   └── 产出每个单元格坐标框        │   │       (Server 级 LightSVTR，手写/ │
│       (Cell BBoxes: [x1,y1,x2,y2])│   │        印章/50种语言企业级高精识别)│
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

### 3.2 ONNX 模型配置与硬件负载规格（方案 A 生产级 FP32 无损高精选型）

坚持**“精度第一”**原则，采用官方原生 **FP32 浮点精度权重**，确保复杂单据与手写修改 100% 无损还原：

| 模块名称 | 基础模型 | ONNX 尺寸 (FP32 交付规格) | 4核16G PC 端侧耗时预估 | 职责与功能 |
| :--- | :--- | :--- | :--- | :--- |
| **表格结构识别** | `ch_ppstructure_SLANet_plus` | **~9.5 MB** | ~120ms ~ 200ms | 预测表格逻辑结构（HTML 语法标签）与每个单元格坐标，专门强化了对发票/单据**无线表（无框线）**与边缘偏移的识别容错率 |
| **文本区域检测** | `PP-OCRv6_small_det` | **~4.5 MB** | ~80ms ~ 150ms | 基于 PPLCNetV4 + RepLKFPN，轻快大感受野，百毫秒级精准定位单行印刷与手写体文字区域 |
| **文本文字识别** | `PP-OCRv6_medium_rec` (Server 高精版) | **~68.0 MB** | ~250ms ~ 450ms (30~50行) | 基于 LightSVTR 架构，Server 级无损高精，原生统一支持 50 种多语言，手写数字与密集金额识别率处于最高水平，彻底消除错字漏字风险 |
| **版面区域分析** (选配) | `PicoDet-L_layout` / `PP-DocLayout` | **~7.5 MB** | ~100ms ~ 150ms | 精准分辨页面中的表格、正文、标题、印章与图表区域（全幅单据默认直通以提升吞吐） |
| **全套汇总** | **方案 A (FP32 全量高精)** | **~89.5 MB** | **单页端到端 ~500ms - 850ms** | **运行时内存峰值仅 ~250MB - 350MB (仅占 16G 内存的 ~2%)** |

> **部署与交付策略**：
> 1. **首期全量 FP32 交付**：整套模型不到 90MB，首次启动可随 Release 安装包内置，或通过 `model_manager.rs` 后台按需静默下载，配套加载 `ppocrv6_dict.txt`。
> 2. **量化策略说明**：因 4核16G PC 运行 FP32 完全轻松且流畅（耗时不到 1 秒），**暂不引入 INT8 量化**，确保企业合规审计中的关键数字与手写体具备绝对最高的准确度。未来仅在极端低配嵌入式设备场景下，才考虑提供可选量化包。

---

## 4. 核心模块设计与实现细节

### 4.1 模块代码目录规划
在 `SensiDoc/src/` 下新增 `ocr` 子模块：
```text
src/
├── ocr/
│   ├── mod.rs               # 对外统一暴露 OcrEngine 入口
│   ├── preprocessor.rs      # 图像归一化、Resize、Letterbox 变换
│   ├── table_structure.rs   # SLANet_plus ONNX 推理、HTML 词表反序列化
│   ├── text_detector.rs     # PP-OCRv6 DBNet (RepLKFPN) 文本定位后处理
│   ├── text_recognizer.rs   # PP-OCRv6 LightSVTR CTC 解码 (含 50 语种统一字典映射)
│   ├── matcher.rs           # 单元格与文本的空间拓扑对齐算法 (中心点+Coverage)
│   └── markdown_builder.rs  # HTML/表格数据转标准 Markdown
├── converter.rs             # 改造：增加图片与扫描版 PDF 拦截分流
```

### 4.2 空间拓扑匹配算法 (Matcher)
表格识别的关键在于**把文字准确塞进对应的单元格**：
1. **坐标归一化**：将检测出的文本框坐标（$B_{text}$）与 SLANet_plus 预测的单元格坐标（$B_{cell}$）映射到相同的图像原始分辨率空间。
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
[阶段 1: ONNX 运行环境与企业级模型准备] (约 3-5 天)
  ├── 集成 Rust `ort` 2.x 依赖，验证 macOS (Metal/CoreML) 与 Windows 跨平台编译
  ├── 导出/拉取 `SLANet_plus` 与 `PP-OCRv6` (Small Det + Medium Rec) 权重
  └── 准备并校验 PP-OCRv6 的 50 种语言统一字符字典 (`ppocrv6_dict.txt`)
        │
        ▼
[阶段 2: 核心推理流水线与拓扑匹配实现] (约 5-7 天)
  ├── 实现图像自适应 Letterbox 预处理与 Tensor 组装
  ├── 实现 `SLANet_plus` HTML 标签解码与 Cell BBox 提取 (含无线表容错)
  ├── 实现 `PP-OCRv6` RepLKFPN 文本定位与 LightSVTR CTC 解码
  └── 实现 Cell-Text 空间拓扑几何包含与文本排序匹配算法 (Matcher)
        │
        ▼
[阶段 3: SensiDoc 主干集成与格式对接] (约 2-3 天)
  ├── 改造 `src/converter.rs`，接入图片与扫描版 PDF 分流 (基于 lopdf 图片流提取)
  ├── 将解析产出的表格格式化为 Markdown / 兼容 HTML <table>
  └── 在 Web 界面验证纸质发票、送货单的拖拽解析与全文高亮脱敏
        │
        ▼
[阶段 4: 企业级场景专项调优与精度封板 (FP32 无损高精)] (约 3-4 天)
  ├── 针对真实企业发票、报销凭证进行税号、金额数字、手写修改与印章遮挡全量召回测试
  ├── 调优 Windows (AVX2/DirectML) 与多线程推理线程池，单页 CPU 耗时控制在 500~800ms
  └── 封板 FP32 原始精度权重（全套 ~89.5MB），验证 4核16G 办公设备内存峰值与长时并发稳定性
```

---

## 7. 方案总结

方案 B（方案 A 生产级混合选型）通过在 SensiDoc 中以**纯 Rust + ONNX Runtime** 的方式原生嵌入：
1. **精度第一，零量化截断损失**：全链路采用 FP32 原始高精模型（“Small 高敏定位 + Medium/Server 高精识别 + SLANet_plus 无线表增强”），在普通办公 PC 上即可跑出最高召回率，彻底杜绝金额与证件号错漏，确保合规审计零失误。
2. **常规硬件无痛承载**：针对 Windows 4核16G 主流环境，全套模型常驻仅占用 ~250-350MB 内存，单页解析耗时 < 1 秒，摆脱 Python 环境依赖，保持客户端极简分发。
3. **架构无缝兼容**：输出的标准 GFM Markdown 与兼容 HTML `<table>` 直接进入现有的脱敏、高亮与本地 LLM 审计流水线，最大程度复用了已有资产。
