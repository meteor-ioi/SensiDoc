# SensiDoc CLI 命令行工具与 Agent 集成开发指南 (CLI_GUIDE.md)

> **版本**：v1.4.0  
> **适用受众**：终端开发者、DevOps/安全工程师、AI Agent 编排系统（LangChain、CrewAI、MCP、自定义 Subprocess）  
> **核心定位**：本地离线文档与扫描件智能审计、全格式原生排版等长脱敏及三级 OCR/VLM 解析内核。

---

## 1. 架构总览与 Unix 哲学设计

SensiDoc CLI 遵循经典的 **Unix 管道哲学**：
- **数据与日志物理隔离**：所有的结构化数据（JSON 响应、Markdown 文本、脱敏文件路径）严格只输出至标准输出 `stdout`；所有人类可读的进度、警告及调试信息全部输出至标准错误 `stderr`；
- **全格式与扫描件支持**：既支持原生排版文档（DOCX/PDF/XLSX/PPTX/TXT/CSV），也完整支持图像单据与扫描版 PDF 的结构化识别；
- **三级 OCR/VLM 智能增强 (`--ocr`)**：支持 `base`（毫秒级基础 OCR）、`fast-vlm`（自动联动基础 OCR + 局部微切片快速复核纠偏）与 `full-vlm`（全量多模态高保真排版重构）；
- **免模型毫秒级极速模式 (`--regex-only`)**：针对高频通用敏感信息（手机号、身份证号、银行卡号等），提供毫秒级（< 15ms）纯正则兜底抽取，无需唤醒 LLM 大模型，资源占用为 0；
- **智能双轨服务探针**：在需要大模型语义推理时，CLI 会优先探测本地 `18188` 端口是否有常驻的 `llama-server` 实例。若有直接复用连接（延迟 < 500ms），无则单次按需拉起并在推理结束后立即释放。

```text
[调用方: AI Agent / CI-CD / Terminal]
                │
                ▼ (参数输入: --ocr / -t / -r / --regex-only)
        [sensidoc 命令行引擎]
                │
    ┌───────────┴─────────────────────────────────────────┐
    │                                                     │
    ▼                                                     ▼
【audit / mask 审计与脱敏】                         【convert / templates】
    │                                                     │
    ├── 文档前置路由 (prepare_document_markdown):          ├── convert (转 Markdown 纯文本)
    │     ├─ 原生文档: AnyDoc 解析                        │     └─ 支持 --ocr base/fast-vlm/full-vlm
    │     ├─ 基础 OCR: PP-OCRv6 + 结构化表格识别          └── templates list (读取场景模板)
    │     ├─ Fast-VLM: 自动先基础 OCR ➔ 局部微切片纠偏
    │     └─ Full-VLM: 端到端全量多模态视觉高保真重构
    │
    ├── 规则解析 (-t 模板 / --rules 追加 / --regex-only)
    │
    ├── 提取执行 (双轨机制):
    │     ├─ 优先: 探测 18188 端口复用常驻服务
    │     └─ 降级: 临时启动单次推理进程并优雅回收
    │
    ├── 等长替换 / 原生排版保护 / 冲突消解
    │
    ▼
[stdout 标准输出: 纯结构化 JSON / 目标文件 / Markdown]
[stderr 标准错误: 状态与进度提示]
```

---

## 2. 核心子命令与参数速查

| 子命令 | 功能说明 | 典型场景 | OCR/VLM 支持 |
| :--- | :--- | :--- | :--- |
| **`convert`** | 快速将任意文档/扫描件解析为 Markdown | 纯文本预处理、RAG 知识库灌库提取、单据 OCR | **`--ocr [<TIER>]`** |
| **`audit`** | 审计文档并输出结构化命中清单 | Agent 获取实体信息、CI/CD 安全合规卡点 | **`--ocr [<TIER>]`** |
| **`mask`** | 生成等长排版脱敏后的目标文档 | 原始文件脱敏导出 (保留 Word/PDF/Excel 原生排版) | **`--ocr [<TIER>]`** |
| **`templates`** | 列出或查询系统中已保存的场景模板 | Agent 预先获取可用的提取规则模板列表 | - |
| **`serve`** | 显式启动 HTTP 后端 API 服务或桌面视窗 | 启动后台持久化常驻服务或可视化界面 | 完整 Web API |
| **`benchmark`** | 运行多模型天梯榜评测基准 | 评估模型在复杂场景下的提取精确度与遵循度 | - |

---

## 3. 子命令详细用法

### 3.1 `sensidoc audit`（敏感信息审计提取）

```bash
sensidoc audit <FILE> [OPTIONS]
```

#### 关键参数列表
- `<FILE>`：待审计文档路径，支持 `.docx`、`.pdf`、`.xlsx`、`.pptx`、`.txt`、`.csv`、`.md` 及图像格式（`.png`、`.jpg`、`.jpeg`、`.bmp`、`.webp`）。
- `--ocr [<TIER>]`：**扫描件/图像 OCR 与 VLM 增强档位**（`base` | `fast-vlm` | `full-vlm`）：
  - `--ocr` 或 `--ocr base`：基础毫秒级 OCR 解析；
  - `--ocr fast-vlm`：**自动先执行基础 OCR**，再自适应提取低置信度（< 0.88）疑难区块切片送入轻量 VLM 进行局部语义纠偏；
  - `--ocr full-vlm`：整图端到端视觉多模态高保真重构。
- `-t, --template <NAME_OR_ID>`：指定使用在 Web 端已保存的场景模板（支持模糊匹配，如 `-t "合同模板"`）。
- `-r, --rules <RULES>`：命令行自定义规则追加，逗号分隔，格式为 `字段名[:风险等级]`（风险等级可选 `高/中/低` 或 `high/medium/low`），如：`--rules "甲方企业:高,手机号:高,优惠折扣:中"`。
- `--rules-file <JSON_PATH>`：从外部 JSON 规则文件加载字段定义。
- `--regex-only`：**极速模式**，仅使用内置预编译正则规则，不启动 LLM 模型，毫秒级响应。
- `-m, --model <GGUF_NAME>`：指定使用的本地 GGUF 模型文件。
- `--online <ONLINE_ID>`：指定使用的在线大模型配置 ID（需在 Web 端预先配置好 Key）。
- `-f, --format <FORMAT>`：输出格式，可选 `json`（默认）或 `table`（终端纯文本表格）。
- `-o, --output <PATH>`：将审计结果写入指定文件（默认输出到 `stdout`）。
- `-q, --quiet`：**静默模式**，抑制 `stderr` 的日志输出，仅向 `stdout` 输出纯净结果，Agent 调用必选。
- `--fail-on-sensitive`：若检测出高危敏感信息，命令将返回**退出码 1**（默认返回 0）。
- `--no-record`：**无痕模式**，不将本次审计文档与提取快照保存至本地工作区（默认会自动录入并同步至 Web 界面「文档列表」供人工复核）。

#### 示例
```bash
# 1. 最轻量、极速审计（推荐 Agent 高频调用）
sensidoc audit contract.docx -t "合同模板" --regex-only -q

# 2. 扫描件/发票图片审计（自动基础 OCR + 局部微切片快速纠偏）
sensidoc audit invoice.png --ocr fast-vlm -r "发票代码:高,购买方:高,金额:中" -f table

# 3. 终端人类可读表格输出
sensidoc audit contract.pdf -t "合同模板" -f table

# 4. CI/CD 安全扫描卡点（若有高危信息直接非零退出阻断流水线）
sensidoc audit report.docx -t "核心机密模板" --fail-on-sensitive -q

# 5. 纯批量无痕模式（不向工作区留存文档与快照）
sensidoc audit test.docx --regex-only --no-record -q
```

---

### 3.2 `sensidoc mask`（全格式原生等长脱敏）

```bash
sensidoc mask <FILE> [OPTIONS]
```

#### 关键参数列表
- `<FILE>`：待脱敏的原始文档路径（支持文档与扫描件图像）。
- `--ocr [<TIER>]`：扫描件/图像 OCR 与 VLM 增强档位（`base` | `fast-vlm` | `full-vlm`）。
- `-o, --output <PATH>`：指定脱敏后输出文件的路径（若不指定，默认在同目录下生成 `[原文件名]_脱敏.[扩展名]`）。
- `-t, --template <NAME_OR_ID>`：指定所依据的场景模板。
- `-r, --rules <RULES>`：追加或自定义脱敏字段。
- `--mode <MODE>`：脱敏格式模式，默认为 `native`（100% 保留原生 DOCX/PDF/XLSX/PPTX 版式样式），可选 `markdown`（导出为轻量纯文本 Markdown）。
- `--style <STYLE>`：脱敏打码风格，可选 `masking`（保留首尾星号掩码如 `张*三`）或 `redaction`（字符硬抹除如 `████`）。
- `--regex-only`：免模型极速脱敏。
- `-q, --quiet`：静默模式，仅在 `stdout` 输出最终生成的文件路径。
- `--no-record`：**无痕模式**，不将本次文档与脱敏快照写入 Web 工作区文档列表。

#### 示例
```bash
# 1. 脱敏 Word 文档并保持排版原样
sensidoc mask 采购合同.docx -t "合同模板" -o 采购合同_已脱敏.docx

# 2. 扫描件合同脱敏（快速 OCR 纠偏后导出 Markdown 脱敏文本）
sensidoc mask 扫描件.png --ocr fast-vlm -t "合同模板" --mode markdown -o 扫描件_脱敏.md

# 3. 脱敏 PDF 文档（原生 Content Stream 字符级打码，自动抹除 XMP 元数据）
sensidoc mask 简历.pdf -t "HR模板" --regex-only

# 4. 硬抹除风格脱敏 Excel 表格
sensidoc mask 薪酬表.xlsx --rules "身份证,手机号,实发工资:高" --style redaction
```

---

### 3.3 `sensidoc convert`（文档与扫描件转 Markdown）

```bash
sensidoc convert <FILE> [OPTIONS]
```

将各种格式的复杂文档、单据图像或扫描件快速转换为排版工整的标准 GitHub Flavored Markdown (GFM) 文本。

#### 关键参数列表
- `<FILE>`：待转换的目标文档路径。
- `--ocr [<TIER>]`：**扫描件/图像 OCR 与 VLM 增强档位**：
  - `base`（缺省 `--ocr` 不带参数时默认）：毫秒级 PP-OCRv6 + 结构化表格识别；
  - `fast-vlm`：**自动先执行基础 OCR**，再自适应定位低置信度（< 0.88）局部疑难单元格/文字块，拉起轻量 VLM 进行局部微切片定向纠偏；
  - `full-vlm`：整图端到端视觉多模态大模型排版高保真重构。
- `-o, --output <PATH>`：输出 Markdown 文件的写入路径（默认直接输出至 `stdout`）。
- `-q, --quiet`：静默模式，仅输出最终 Markdown 结果，抑制进度日志。

#### 示例
```bash
# 1. 原生文档直接转换
sensidoc convert presentation.pptx > presentation.md

# 2. 单据图像毫秒级基础 OCR 转换
sensidoc convert invoice.png --ocr -o invoice.md

# 3. 扫描件先基础 OCR，自动联动局部微切片快速纠偏 (推荐单据提取)
sensidoc convert receipt.jpg --ocr fast-vlm -o receipt.md

# 4. 复杂单据/表格端到端全量视觉多模态排版重构
sensidoc convert complex_bill.png --ocr full-vlm -o complex_bill.md
```

---

### 3.4 扫描件与图像 OCR/VLM 三级识别流水线 (`--ocr` 深度说明)

针对纸质单据、发票、合同扫描件与工业图纸，SensiDoc CLI 统一提供 `--ocr` 档位调度：

```text
[输入文件: 图像 / 扫描版 PDF]
           │
           ▼
    [解析 --ocr 档位]
           │
 ┌─────────┼────────────────────────┐
 │ (未传)  │ (base 或仅 --ocr)      │ (fast-vlm)             │ (full-vlm)
 ▼         ▼                        ▼                        ▼
[原生解析] [基础 ONNX OCR]          [1. 基础 ONNX OCR 产出]  [整图 Base64 编码]
(普通文档) (毫秒级文字与表格识别)              │                       │
                                    ▼                       ▼
                           [2. 聚类 <0.88 疑难区块]  [多模态端到端高保真重构]
                                    │                       │
                                    ▼                       │
                           [3. 局部微切片轻量 VLM 纠偏]     │
                                    │                       │
                                    ▼                       ▼
                         [修正后 Markdown 结构化输出 / 审计 / 脱敏]
```

#### 档位选型指南：
1. **`base`（毫秒级基础 OCR）**：适合版面印刷清晰、对比度高、对速度要求极高（< 300ms）的高频流水线作业；
2. **`fast-vlm`（基础 OCR + 局部微切片快速复核）**：**默认最推荐路线**。系统**自动先执行基础 OCR** 产出不可变底稿，仅对识别置信度低于 0.88 的疑难错别字、连笔数字、英文单位切片送入轻量端侧模型（如 Qwen3.5-0.8B）进行定向纠偏，兼顾极高精度与秒级吞吐；
3. **`full-vlm`（全量多模态重构）**：适合排版极其复杂、跨区域复合单据、严重倾斜扭曲或多层嵌套表格，直接由大型视觉语言模型端到端理解输出。

---

### 3.5 `sensidoc templates`（场景模板管理）

```bash
sensidoc templates [list] [-f table|json]
```
列出当前工作区中持久化存储的所有场景模板名称、包含的规则字段及描述。

```bash
# 查看表格清单
sensidoc templates list

# 获取模板的 JSON 结构 (供 Agent 探测能力)
sensidoc templates list -f json
```

---

### 3.6 `sensidoc serve`（服务与桌面运行）

```bash
# 启动本地无头后端服务（监听指定端口）
sensidoc serve --port 8080 --host 0.0.0.0 --headless

# 不带任何参数默认行为：拉起桌面 GUI 应用程序
sensidoc
```

---

## 4. 标准输出 JSON 规范 (Audit Schema)

当调用 `sensidoc audit <FILE> -q` 时，`stdout` 输出的 JSON 数据格式严格遵循以下结构：

```json
{
  "status": "success",
  "document": {
    "id": "7fd7b745-f0ce-4375-9c9e-56f4d5b62b1b",
    "filename": "01_企业采购商务框架合同.docx",
    "char_count": 687,
    "format": "docx"
  },
  "summary": {
    "snapshot_id": "cc473e0b-702e-42f0-b5fc-a422531b6cff",
    "total_detected": 8,
    "high_risk_count": 6,
    "medium_risk_count": 2,
    "low_risk_count": 0,
    "has_sensitive": true,
    "execution_ms": 12,
    "model_used": "内置正则引擎",
    "recorded_to_workspace": true
  },
  "detected_items": [
    {
      "category": "手机",
      "text": "13810928374",
      "priority": "high",
      "count": 2,
      "source": "regex"
    },
    {
      "category": "银行账号",
      "text": "6228480402837491",
      "priority": "high",
      "count": 1,
      "source": "regex"
    }
  ],
  "missed_fields": [
    {
      "category": "甲方企业",
      "priority": "medium"
    },
    {
      "category": "合同总金额",
      "priority": "high"
    }
  ]
}
```

> **说明**：
> - `document.id` 与 `summary.snapshot_id`：仅在写入工作区时返回（默认开启），可直接用于后续调用 Web API 查询或复核；
> - `summary.recorded_to_workspace`：表明本次 CLI 执行记录是否已同步落盘入库（当指定 `--no-record` 时为 `false`）。

---

## 5. Web 界面与 CLI 实时双向联动机制

SensiDoc 实现了 **“Agent 自动化处理 + 人类可视化复核”** 的闭环协作范式：

```text
[AI Agent / CLI 进程]                    [磁盘共享与热感知]                    [Web / 桌面客户端进程]
        │                                        │                                       │
        ├─ 1. 执行 audit / mask                  │                                       │
        ├─ 2. 转换 Markdown 并提取敏感实体        │                                       │
        ├─ 3. 写入 uploads/{doc_id}.bin ─────────┼──> 保存原始二进制 (备用于原生脱敏下载)   │
        └─ 4. 更新 .sensidoc_workspace.json ─────┼──> 触发文件 mtime 修改 ───────────────┤
                                                 │                                       │
                                                 │                 5. 用户打开或刷新 Web 页面
                                                 │                                       │
                                                 │<── 检测到 mtime 发生外部更新 ──────────┤
                                                 │                                       │
                                                 │─── 热同步重载最新文档与快照数据 ────────>│
                                                                                         │
                                                                           6. 左侧「文档列表」立即呈现
                                                                           7. 快照历史标明 [CLI] 来源
                                                                           8. 支持中间高亮与原生脱敏导出
```

### 联动特性亮点：
1. **自动入库与同名复用**：CLI 审计同名文档时，会自动复用已有文档记录并追加本次提取快照；若为新文档则自动在工作区创建新文档卡片。
2. **完整原文档保留**：CLI 自动将原始文件安全转储在 `uploads/{doc_id}.bin`，保证后续在 Web 端可随时点击「脱敏导出」下载原生打码文档（保留 Word 样式与页眉页脚）。
3. **跨进程实时感知**：后台常驻的 Web HTTP 服务基于文件 `mtime` 状态机实现零侵入热重载，无需重启服务，刷新浏览器即可无缝呈现 CLI 产生的新记录。
4. **无痕隔离模式**：若为纯离线高频批处理、CI 门禁等无须留存历史的场景，只需追加 `--no-record` 开关，即可跳过工作区持久化与临时文件生成。

---

## 6. 进程退出码规范 (Exit Codes)

| 退出码 | 描述 | 适用场景 |
| :--- | :--- | :--- |
| `0` | 执行成功，且未触发敏感阻断 | 正常流程放行 |
| `1` | 执行成功，但**命中高危敏感信息**（配合 `--fail-on-sensitive`） | CI/CD 代码审查门禁阻断、合规告警 |
| `2` | 命令行参数错误、规则解析失败或目标文件不存在 | 客户端调用异常 |
| `3` | 模型加载失败、推理超时或文档解析崩溃 | 运行时系统故障 |

---

## 7. AI Agent 与第三方工具集成实战

### 7.1 Python Agent 工具函数封装

```python
import subprocess
import json
from pathlib import Path
from typing import Dict, Any, Optional

def sensidoc_audit(file_path: str, template: Optional[str] = None, rules: Optional[str] = None, ocr: Optional[str] = None, regex_only: bool = False) -> Dict[str, Any]:
    """
    通过 SensiDoc CLI 审计文档敏感信息并返回字典 (支持文档与扫描件)
    """
    cmd = ["sensidoc", "audit", file_path, "-q"]
    if template:
        cmd.extend(["-t", template])
    if rules:
        cmd.extend(["-r", rules])
    if ocr:
        cmd.extend(["--ocr", ocr])
    if regex_only:
        cmd.append("--regex-only")
        
    proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return json.loads(proc.stdout)

def sensidoc_convert(file_path: str, ocr: Optional[str] = None) -> str:
    """
    通过 SensiDoc CLI 将文档或扫描件转换为 Markdown
    """
    cmd = ["sensidoc", "convert", file_path, "-q"]
    if ocr:
        cmd.extend(["--ocr", ocr])
    proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return proc.stdout

def sensidoc_mask(file_path: str, output_path: str, template: Optional[str] = None, ocr: Optional[str] = None) -> str:
    """
    对目标文件进行原生等长排版脱敏并输出到指定路径
    """
    cmd = ["sensidoc", "mask", file_path, "-o", output_path, "-q"]
    if template:
        cmd.extend(["-t", template])
    if ocr:
        cmd.extend(["--ocr", ocr])
    proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return proc.stdout.strip()

# 使用示例
if __name__ == "__main__":
    # 1. 审计普通文档
    result = sensidoc_audit("tests/01_合同.docx", template="合同模板", regex_only=True)
    print(f"命中敏感词数: {result['summary']['total_detected']}")
    for item in result["detected_items"]:
        print(f"[{item['category']}] {item['text']} (风险: {item['priority']})")
        
    # 2. 针对扫描件单据执行微切片纠偏并审计
    scan_res = sensidoc_audit("tests/invoice.png", ocr="fast-vlm", rules="发票代码:高,金额:高")
    print(f"扫描件命中: {scan_res['summary']['total_detected']}")

    # 3. 扫描件转换为高保真 Markdown
    md_text = sensidoc_convert("tests/receipt.jpg", ocr="fast-vlm")
    print(f"转换结果字数: {len(md_text)}")

### 7.2 Node.js / TypeScript Agent Tool

```typescript
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

export interface AuditResult {
  status: string;
  summary: {
    total_detected: number;
    has_sensitive: boolean;
    high_risk_count: number;
  };
  detected_items: Array<{ category: string; text: string; priority: string }>;
}

export async function auditDocument(
  filePath: string,
  options?: { template?: string; ocr?: 'base' | 'fast-vlm' | 'full-vlm'; regexOnly?: boolean }
): Promise<AuditResult> {
  const args = ['audit', filePath, '-q'];
  if (options?.template) args.push('-t', options.template);
  if (options?.ocr) args.push('--ocr', options.ocr);
  if (options?.regexOnly) args.push('--regex-only');

  const { stdout } = await execFileAsync('sensidoc', args);
  return JSON.parse(stdout);
}
```

### 7.3 GitHub Actions / CI 流水线卡点配置

在代码库提交或 PR 构建流程中，自动对附带的商务文档或测试数据进行合规扫描，防止真实用户 PII 泄露：

```yaml
name: Security Audit Pipeline
on: [push, pull_request]

jobs:
  sensitive-data-check:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Audit Repository Documents
        run: |
          # 扫描 docs 目录下的全部文档，存在高危敏感项时流水线自动失败阻断
          find ./docs -name "*.docx" -o -name "*.pdf" | while read file; do
            sensidoc audit "$file" -t "合同模板" --fail-on-sensitive -q || exit 1
          done
```
