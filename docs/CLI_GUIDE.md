# SensiDoc CLI 命令行工具与 Agent 集成开发指南 (CLI_GUIDE.md)

> **版本**：v1.0.2  
> **适用受众**：终端开发者、DevOps/安全工程师、AI Agent 编排系统（LangChain、CrewAI、MCP、自定义 Subprocess）  
> **核心定位**：本地离线文档敏感信息智能审计与全格式原生排版等长脱敏内核。

---

## 1. 架构总览与 Unix 哲学设计

SensiDoc CLI 遵循经典的 **Unix 管道哲学**：
- **数据与日志物理隔离**：所有的结构化数据（JSON 响应、Markdown 文本、脱敏文件路径）严格只输出至标准输出 `stdout`；所有人类可读的进度、警告及调试信息全部输出至标准错误 `stderr`；
- **免模型毫秒级极速模式 (`--regex-only`)**：针对高频的通用敏感信息（手机号、身份证号、银行卡号等），提供毫秒级（< 15ms）纯正则兜底抽取，无需唤醒 LLM 大模型，资源占用为 0；
- **智能双轨服务探针**：在需要大模型语义推理时，CLI 会优先探测本地 `18188` 端口是否有常驻的 `llama-server` 实例。若有直接复用连接（延迟 < 500ms），无则单次按需拉起并在推理结束后立即释放。

```text
[调用方: AI Agent / CI-CD / Terminal]
                │
                ▼ (参数输入)
        [sensidoc 命令行引擎]
                │
   ┌────────────┴────────────────────────────────────────┐
   │                                                     │
   ▼                                                     ▼
【audit / mask 审计与脱敏】                        【templates / convert】
   │                                                     │
   ├── anydoc 格式转换 (DOCX/PDF/XLSX/PPTX/TXT/CSV)      ├── templates list (读取场景模板)
   │                                                     └── convert (快速导出 Markdown)
   ├── 规则解析 (-t 模板 / --rules 追加 / --regex-only)
   │
   ├── 提取执行 (双轨机制):
   │     ├─ 优先: 探测 18188 端口复用常驻服务
   │     └─ 降级: 临时启动单次推理进程并优雅回收
   │
   ├── 等长替换 / 冲突消解
   │
   ▼
[stdout 标准输出: 纯结构化 JSON / 目标文件]
[stderr 标准错误: 状态与进度提示]
```

---

## 2. 核心子命令与参数速查

| 子命令 | 功能说明 | 典型场景 |
| :--- | :--- | :--- |
| **`audit`** | 审计文档并输出结构化命中清单 | Agent 获取实体信息、CI/CD 安全合规卡点 |
| **`mask`** | 生成等长排版脱敏后的目标文档 | 原始文件脱敏导出 (保留 Word/PDF/Excel 原生排版) |
| **`convert`** | 快速将任意版式文档解析为 Markdown | 纯文本预处理、RAG 知识库灌库提取 |
| **`templates`** | 列出或查询系统中已保存的场景模板 | Agent 预先获取可用的提取规则模板列表 |
| **`serve`** | 显式启动 HTTP 后端 API 服务或桌面视窗 | 启动后台持久化常驻服务或可视化界面 |

---

## 3. 子命令详细用法

### 3.1 `sensidoc audit`（敏感信息审计提取）

```bash
sensidoc audit <FILE> [OPTIONS]
```

#### 关键参数列表
- `<FILE>`：待审计文档路径，支持 `.docx`、`.pdf`、`.xlsx`、`.pptx`、`.txt`、`.csv`、`.md`。
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

# 2. 终端人类可读表格输出
sensidoc audit contract.pdf -t "合同模板" -f table

# 3. 命令行临时追加自定义字段
sensidoc audit budget.xlsx --rules "采购金额:高,供应商:中,经办人:中"

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
- `<FILE>`：待脱敏的原始文档路径。
- `-o, --output <PATH>`：指定脱敏后输出文件的路径（若不指定，默认在同目录下生成 `[原文件名]_脱敏.[扩展名]`）。
- `-t, --template <NAME_OR_ID>`：指定所依据的场景模板。
- `-r, --rules <RULES>`：追加或自定义脱敏字段。
- `--mode <MODE>`：脱敏格式模式，默认为 `native`（100% 保留原生 DOCX/PDF/XLSX/PPTX 版式样式），可选 `markdown`（导出为轻量纯文本 Markdown）。
- `--regex-only`：免模型极速脱敏。
- `-q, --quiet`：静默模式，仅在 `stdout` 输出最终生成的文件路径。
- `--no-record`：**无痕模式**，不将本次文档与脱敏快照写入 Web 工作区文档列表。

#### 示例
```bash
# 1. 脱敏 Word 文档并保持排版原样
sensidoc mask 采购合同.docx -t "合同模板" -o 采购合同_已脱敏.docx

# 2. 脱敏 PDF 文档（原生 Content Stream 字符级打码，自动抹除 XMP 元数据）
sensidoc mask 简历.pdf -t "HR模板" --regex-only

# 3. 脱敏 Excel 表格并导出为 Markdown 文本
sensidoc mask 薪酬表.xlsx --rules "身份证,手机号,实发工资:高" --mode markdown -o 薪酬表_脱敏.md
```

---

### 3.3 `sensidoc convert`（文档纯文本转换）

```bash
sensidoc convert <FILE> [-o <OUTPUT>] [-q]
```
直接调用底层的 `anydoc` 纯 Rust 转换流，将 Word、PDF、Excel、PPT、RTF 等转换为结构化 Markdown 字符串并输出到 `stdout`。

```bash
sensidoc convert presentation.pptx > presentation.md
```

---

### 3.4 `sensidoc templates`（场景模板管理）

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

### 3.5 `sensidoc serve`（服务与桌面运行）

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

def sensidoc_audit(file_path: str, template: Optional[str] = None, rules: Optional[str] = None, regex_only: bool = True) -> Dict[str, Any]:
    """
    通过 SensiDoc CLI 审计文档敏感信息并返回字典
    """
    cmd = ["sensidoc", "audit", file_path, "-q"]
    if template:
        cmd.extend(["-t", template])
    if rules:
        cmd.extend(["-r", rules])
    if regex_only:
        cmd.append("--regex-only")
        
    proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return json.loads(proc.stdout)

def sensidoc_mask(file_path: str, output_path: str, template: Optional[str] = None) -> str:
    """
    对目标文件进行原生等长排版脱敏并输出到指定路径
    """
    cmd = ["sensidoc", "mask", file_path, "-o", output_path, "-q"]
    if template:
        cmd.extend(["-t", template])
    proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return proc.stdout.strip()

# 使用示例
if __name__ == "__main__":
    # 1. 审计文档
    result = sensidoc_audit("tests/01_合同.docx", template="合同模板")
    print(f"命中敏感词数: {result['summary']['total_detected']}")
    for item in result["detected_items"]:
        print(f"[{item['category']}] {item['text']} (风险: {item['priority']})")
        
    # 2. 原生脱敏
    out_file = sensidoc_mask("tests/01_合同.docx", "dist/01_合同_脱敏.docx", template="合同模板")
    print(f"脱敏文件生成于: {out_file}")
```

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

export async function auditDocument(filePath: string, template = "合同模板"): Promise<AuditResult> {
  const { stdout } = await execFileAsync('sensidoc', [
    'audit', filePath,
    '-t', template,
    '--regex-only',
    '-q'
  ]);
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
