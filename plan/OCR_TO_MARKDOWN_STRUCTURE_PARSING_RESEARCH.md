# OCR 识别转 Markdown 结构化解析策略调研报告（借鉴 MinerU 与 Docling 方案）

## 1. 调研背景与问题定义

### 1.1 当前痛点
在当前单据/扫描件文档解析链路中（如采用 `PP-OCRv6` 系列），模型的核心能力局限于传统的**“检测（Detection）+ 识别（Recognition）”**：
- **无版面语义（Layout Agnostic）**：仅输出扁平的单行文字框（`[x1, y1, x2, y3]`、文本、置信度），无法区分标题（H1/H2）、段落、列表、表格、图表、页眉页脚与旁注。
- **阅读顺序断裂（Reading Order Distortion）**：双栏/多栏排版时，若按单纯几何坐标（如 Y 轴 Top-Down）排序，文字会被水平横向“穿透串联”，产生严重语序错乱。
- **段落与折行碎片化（Fragmented Paragraphs）**：一句话跨两行被切碎为两个独立块，英文产生断词（Hyphenation），中文被插入多余换行。
- **表格与公式降维退化**：表格被拆解为散装文本，无行列和单元格关联；数学公式无法转为 LaTeX。

### 1.2 调研目标
深入剖析当前行业标杆开源项目（**MinerU**、**Docling**、**Marker** 等）在调用底层 OCR 之后，**如何通过空间几何匹配、版面分析与语法树重组算法，将碎片化的 OCR 结构数据精准解析并序列化为高质量 Markdown**，为后续工程落地提供可直接借鉴的算法与策略体系。

---

## 2. 业界主流开源项目横向对比

| 项目 | GitHub Stars | 核心技术路线 | 中文/扫描件支持 | 特点与适用场景 |
| :--- | :--- | :--- | :--- | :--- |
| [**MinerU**](https://github.com/opendatalab/MinerU) | **~79.6k** | 专用流水线（版面检测 + OCR + UniMERNet 公式 + 表格模型 + 后处理 AST） | 极佳（国产学术/研报天花板） | 中文论文、教材、研报、复杂多栏文档的最高保真还原 |
| [**Docling**](https://github.com/docling-project/docling) | **~66.2k** | 模块化工业级文档引擎（DocLayNet + TableFormer + 多后端 OCR） | 优（支持 EasyOCR / RapidOCR） | 工程集成首选，规范统一的 `DoclingDocument` 语法树模型 |
| [**Marker**](https://github.com/datalab-to/marker) | **~39.6k** | 深度学习流水线（Surya OCR + 启发式布局后处理） | 优 | 转换速度快，专长于学术论文与多语言排版 |
| [**Zerox**](https://github.com/getomni-ai/zerox) | **~12.3k** | 视觉大模型编排（切页转图 ➔ Vision LLM ➔ Markdown） | 极佳（取决于背后 VLM） | 借助 Qwen2.5-VL / Claude / GPT-4o 端到端理解，免维护复杂本地推理链 |
| [**GOT-OCR2.0**](https://github.com/Ucas-HaoranWei/GOT-OCR2.0) | **~8.2k** | 580M 单模型端到端（输入图片直接输出格式化 Markdown） | 优 | 轻量级单一多模态小模型端到端解析 |

---

## 3. 标杆项目端到端处理全景

MinerU 与 Docling 的核心设计原则是**“分而治之 + 空间路由 + 语法树（AST）后处理”**。其整体架构如下：

```text
               [ 扫描版 PDF / 图像输入 ]
                           │
             ┌─────────────┴─────────────┐
             ▼                           ▼
    [ 版面分析 Layout Model ]    [ 基础 OCR 引擎 (如 PP-OCR) ]
    (检测语义 Block 及其边界)    (检测并识别单行文本框)
     - DocTitle / ParagraphTitle  - BBox: [x1, y1, x2, y3]
     - Text / Table / Image       - Text, Confidence
     - Header / Footer / Footnote
             │                           │
             └─────────────┬─────────────┘
                           ▼
          [ 步骤 1：空间归属投影 (Spatial Matching) ]
             - 计算 OCR 行与 Block 的重叠比率 (IoU / Overlap)
             - 过滤 Header / Footer / PageNumber（加入丢弃列表）
                           │
                           ▼
          [ 步骤 2：领域专门引擎分流 (Specialized Routing) ]
             ├── Table Block   ──> 表格结构识别 (SLANet/TableFormer) ──> 填入文字
             ├── Formula Block ──> 公式识别 (UniMERNet) ──> 转为 LaTeX $$...$$
             └── Text / Title  ──> 保留进入文本流处理
                           │
                           ▼
          [ 步骤 3：阅读顺序与分栏重构 (Reading Order) ]
             - X 轴投影直方图检测分栏中缝（White-space Valley）
             - 分栏拓扑排序：顶通栏 -> 左栏 -> 右栏 -> 底通栏
                           │
                           ▼
          [ 步骤 4：行合并与段落切分 (Para Split & Merge) ]
             - 行末标点判断（LINE_STOP_FLAG 截断检测）
             - 首行缩进与右侧留白（狗牙状）判断
             - 中英文折行空格与连字符（Hyphenation）清理
                           │
                           ▼
          [ 步骤 5：语义映射与 Markdown 序列化 (AST 导出) ]
             - 行高/字号聚类 + 序号正则映射标题层级 (#, ##, ###)
             - 悬挂缩进解析为有序/无序列表 (-, 1.)
             - 序列化生成纯净、标准 Markdown
```

---

## 4. 核心策略与算法机制拆解

### 策略 1：空间路由与杂质过滤（Spatial Bounding Box Matching）
*传统痛点：遍历平铺的 OCR 结果时，页眉、页脚、页码常作为正文穿插在段落之间。*
- **容器与子元素投影**：
  版面分析模型首先识别出页面级的容器块（Block）。计算 OCR 行框与容器框的面积相交比率：
  $$\text{OverlapRatio} = \frac{\text{Area}(BBox_{ocr} \cap BBox_{block})}{\text{Area}(BBox_{ocr})}$$
  当 $\text{OverlapRatio} > 0.65$ 时，该文字行归属于对应 Block。
- **页眉页脚熔断过滤**：
  若某 Block 的语义类型为 `Header`、`Footer` 或 `PageNumber`，该 Block 及其包含的所有 OCR 行直接归入 `discarded_blocks`，不进入正文序列化。

### 策略 2：双栏/多栏与阅读顺序重构（Reading Order）
*传统痛点：双栏排版按 Y 轴排序时，左右栏文本在同一行高处被错误拼接。*
- **分栏中缝检测（X 轴投影直方图）**：
  1. 统计页面中段（如 $X \in [0.25W, 0.75W]$）所有文本框的覆盖密度。
  2. 寻找覆盖率为 0（或极低）的连续纵向空白带（Column Separator Valley）。
  3. 若存在明显中缝，判定为双栏排版。
- **两级重排规则**：
  - **通栏块判定**：若某个 Block 的宽度覆盖超过页面宽度的 75%，标记为通栏块（如论文大标题、摘要、全宽表格）。
  - **排序拓扑流**：
    $$\text{Page} \rightarrow [\text{Top Full-width}] \rightarrow [\text{Left Column Top-Down}] \rightarrow [\text{Right Column Top-Down}] \rightarrow [\text{Bottom Full-width}]$$

### 策略 3：段落合并与跨行断句（Paragraph Splitting）
*传统痛点：OCR 结果逐行断开，无法区分“同一段落的自然换行”与“新段落的起行”。*
参考 MinerU `para_split.py` 的工业级规则：
1. **行末断句符号（Line Stop Flag）**：
   - 截断标点集：`LINE_STOP_FLAG = ('.', '!', '?', '。', '！', '？', ':', '：', ';', '；')`
   - **拼接规则**：如果前一行末尾**不包含**断句标点，且下一行与当前行左侧对齐，则认定为**跨行折行**，必须进行合并。
2. **首行缩进检测（Indentation）**：
   - 设单行平均高度为 $H_{line}$。
   - 若某行左侧相对于当前 Block 左边界向右偏移 $> 0.5 \times H_{line}$，且前一行以断句标点结尾，判定为**新段落起始**。
3. **右侧“狗牙状”留白判定（Ragged Right Margin）**：
   - 正常段落的中间行右侧通常是对齐的；只有段落最后一行往往不满行。
   - 若某行右侧边界距离 Block 右边界有显著留白，且末尾为断句标点，下一行强制起新段落。
4. **文字折行清洗**：
   - **英文连字符处理**：若上一行以连字符结尾（如 `trans-`），下一行开头为 `former`，合并时自动移除连字符拼接为 `transformer`。
   - **英文折行补空格**：普通英文行合并时在两行单词间补充一个空格。
   - **中文折行去空格**：中文字符折行拼接时剔除多余换行符与空格，保持语义连贯。

### 策略 4：表格结构恢复与单元格坐标反填（Table Cell Infilling）
*传统痛点：OCR 提取的表格文本错位，丢失行列关系。*
1. **骨架提取**：表格区域被裁剪后，送入轻量表格识别模型（如 `SLANet_plus` 或 `TableFormer`），输出表格的 HTML 结构序列（`<tr>`, `<td>`, `colspan`, `rowspan`）及各单元格在表格图中的相对坐标框（Cell BBoxes）。
2. **文本空间反填（Spatial Infilling）**：
   - 遍历表格区域内的所有 OCR 识别框。
   - 计算各 OCR 行与所有单元格 Cell BBox 的空间交并比（IoU）。
   - 将文本注入匹配度最高的 Cell 中；若单元格内有多行文字，按 Y 轴自上而下用换行拼接。
3. **Markdown / HTML 序列化**：
   - 无合并单元格的规则表格：序列化为标准 Markdown 表格语法（`| 列1 | 列2 |`）。
   - 复杂跨行跨列（`colspan > 1` 或 `rowspan > 1`）表格：转换为内嵌的 HTML `<table>` 标签，保障 Markdown 渲染不崩塌。

### 策略 5：标题层级判定与格式化（Heading Leveling）
*传统痛点：文档大小标题全部退化为普通段落文本。*
1. **行高/字号聚类（Line Height Clustering）**：
   - 统计当前文档/页面内全部 OCR 框的高度的中位数或众数，作为基准正文字号 $H_{body}$。
   - **层级映射规则**：
     - 若 $H > 1.8 \times H_{body}$ ➔ 映射为 Markdown 一级标题（`# 标题`）
     - 若 $1.4 \times H_{body} < H \le 1.8 \times H_{body}$ ➔ 映射为 Markdown 二级标题（`## 标题`）
     - 若 $1.1 \times H_{body} < H \le 1.4 \times H_{body}$ ➔ 映射为 Markdown 三级标题（`### 标题`）
2. **编号规则辅助校准（Regex Boosting）**：
   - 结合正则表达式检测行首标识：
     - `^第[一二三四五六七八九十0-9]+[章节卷篇]` ➔ 提权为 H1/H2
     - `^\d+\.\d+(\.\d+)?` (如 `1.1`, `2.1.3`) ➔ 按点号层级映射为 H2/H3
     - `^[一二三四]、`、`^\([1-9]\)` ➔ 判定为结构化小节或列表项。

---

## 5. 对 SensiDoc (Rust + ONNX Runtime) 的工程落地参考

SensiDoc 当前采用**纯 Rust 原生嵌入（无 Python 运行时依赖）**的架构（`ort` + `PP-OCRv6` + `SLANet_plus`）。
要在 Rust 环境中复现 MinerU/Docling 的高质量解析效果，无需引入重型 Python 依赖，可通过以下步骤实施：

### 5.1 架构实现路径

```text
[src/ocr/]
 ├── detector.rs          (PP-OCRv6 small det 推理)
 ├── recognizer.rs        (PP-OCRv6 medium rec 推理)
 ├── table_slanet.rs      (SLANet_plus 表格骨架推理)
 └── postprocessor/       <-- [新增核心重构模块]
      ├── layout_rule.rs  (基于几何投影的简单版面/分栏判别器)
      ├── reading_order.rs(多栏与拓扑重排引擎)
      ├── para_merger.rs  (断句标点、缩进判定与换行折叠)
      ├── heading.rs      (行高聚类统计与正则定级)
      └── md_serializer.rs(输出 AST 转换为 Markdown 字符串)
```

### 5.2 阶段性落地建议

1. **第一阶段：轻量后处理规则引擎（零额外模型引入）**
   - 实现 `para_merger`：利用 `LINE_STOP_FLAG` 与左边界对齐规则，将相邻两行合并为真正自然段，消除正文每行强制换行的问题。
   - 实现 `heading`：统计整页平均行高，高于 1.4 倍且文字短于 40 字符的行自动标记为 Markdown `##` 标题。
   - 实现页眉页脚丢弃：顶部 5% 与底部 5% 的孤立单行文本过滤。

2. **第二阶段：表格双向映射闭环**
   - 接入已完成的 `SLANet_plus` 表格单元格坐标，通过 Rust 的空间包围盒求交算法，将 `PP-OCRv6` 文字填充进对应单元格，生成结构完好的 Markdown 表格。

3. **第三阶段：多栏排版阅读顺序解析**
   - 引入 X 轴投影直方图分割算法，解决研报/双栏文献的阅读顺序倒错问题。

---

## 6. 总结

- **PP-OCRv6** 解决了**“字是什么、在哪里”**的问题；
- **MinerU / Docling 的核心价值**在于解决了**“这些字是什么层级、属于哪一段、属于哪个单元格、应该按什么顺序读”**的问题；
- 在 SensiDoc 的 Rust 架构下，通过借鉴其**空间路由、断句标点合并、行高聚类判定标题、表格坐标反填**四大策略，即可在不增加复杂重型依赖的前提下，实现 OCR 扫描件解析质量的质的跃升。
