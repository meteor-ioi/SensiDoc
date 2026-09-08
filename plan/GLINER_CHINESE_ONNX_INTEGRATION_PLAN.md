# 基于 GLiNER 架构与中文预训练主干的 SensiDoc 高速实体抽取方案

## 1. 方案背景与目标

### 1.1 现状痛点
在 SensiDoc 当前版本中，敏感信息抽取主要依赖两路：
1. **正则引擎（Regex）**：速度极快（<1ms），但仅能处理格式固定的规则数据（如手机号、身份证、邮箱）；
2. **端侧生成式小模型（LLM，如 Qwen2.5-1.5B / LFM2-350M）**：能够理解开放实体，但在端侧（CPU/低算力环境）存在以下瓶颈：
   * **时延较长**：逐 Token 自回归解码耗时通常在 1.5s ~ 5s；
   * **格式易损毁与死循环**：JSON 结构在小模型上存在解析失败率；
   * **坐标反查复杂**：仅返回字符串，需要复杂的后处理查找算法回填 `positions` 偏移量；
   * **潜在幻觉风险**：小模型可能修改原词或产生非原文存在的词汇。

### 1.2 目标定位
引入 **GLiNER 架构（判别式 Span 匹配）** + **中文专用预训练主干（如 Chinese-RoBERTa / Chinese-DeBERTa）** + **Rust ONNX Runtime (`ort`)**，打造原生的 **L2 毫秒级开集实体抽取层**，实现：
* ⚡ **极速推理**：单文档分块推理耗时降低至 **20ms ~ 60ms**（吞吐量提升 20 倍以上）；
* 🎯 **绝对 0 幻觉 & 原生坐标**：直接输出原文字符起始与结束位置 `[start, end]`；
* 📦 **轻量便携**：INT8 量化后体积仅 **~150MB - 200MB**，内存占用 < 400MB；
* 🔀 **开集泛化**：动态支持用户自定义敏感字段，无需针对新规则重新微调。

---

## 2. 总体架构设计（三级漏斗流水线）

```text
                      [ 待审计文档 Markdown / 文本 ]
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Level 1: 正则极速流水线 (Regex Pipeline, < 5ms)                         │
│ - 匹配高确定性固定格式 PII（身份证、大陆手机号、银行卡号、电子邮箱）    │
│ - 产出高危确诊项，并在文本位置位图中打标记 (Masking)                    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Level 2: GLiNER 中文 ONNX 极速抽取层 (Rust `ort` 引擎, ~20-50ms)        │
│ - 模型：Chinese-RoBERTa / Erlangshen-DeBERTa + GLiNER Span Head         │
│ - 动态接收用户启用的规则字段（甲方企业、法定代表人、项目代号、金额等）   │
│ - 原生输出 [start_idx, end_idx, text, label, score]                     │
│ - 自动完成与 SensiDoc SensitiveItem 数据结构的无缝对齐                  │
└─────────────────────────────────────────────────────────────────────────┘
                                    │ (仅当启用了复杂条件或长条款规则时触发)
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Level 3: 端侧 LLM 深度语义裁决 (Qwen2.5-1.5B / LFM2, 选配)              │
│ - 处理算术比较（如“金额大于100万”）、上下文逻辑否定、整段条款理解      │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      【位置冲突消解与统一去重】                         │
│ - 优先保留 L1 正则确诊项 > L2 Span 抽取项 > L3 LLM 补充项               │
│ - 输出结构化敏感词审计清单与风险等级评定                                │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. 技术选型与组件规格

| 模块 | 选型方案 | 替代方案 / 备选 | 选型理由 |
| :--- | :--- | :--- | :--- |
| **编码器主干 (Backbone)** | `hfl/chinese-roberta-wwm-ext` | `IDEA-CCNL/Erlangshen-DeBERTa-v2-710M-Chinese`<br>`knowledgator/gliner-x-large` | 采用全词掩码 (WWM)，中文字词边界与构词理解极强，参数量适中 (~110M)。 |
| **跨度匹配头 (Span Head)** | GLiNER Bilinear Span Matching ($K \le 12$) | Token-level Tagging | 支持动态开集标签，无缝适配用户自定义字段。 |
| **模型分发格式** | ONNX (INT8 量化) | SafeTensors / GGUF | 跨平台部署最轻量，冷启动仅需百毫秒。 |
| **Rust 推理后端** | `ort` (ONNX Runtime Rust Bindings) | `gline-rs` / `candle` | 生产级稳定，原生支持 Apple Silicon CoreML / CPU 多线程加速。 |
| **分词器 (Tokenizer)** | HuggingFace `tokenizers` crate | `jieba-rs` + CharTokenizer | Rust 原生高性能分词，准确对齐字符级 offset。 |

---

## 4. 分阶段实施路线图

```text
[ 阶段一：开箱测试与基准评测 ]
   ├── 步骤 1.1: 准备 Python 评测脚本，接入 SensiDoc 10 份基准测试文档
   ├── 步骤 1.2: 对比 `gliner_multi-v2.1` 与 `gliner-x-large` 在中文测试集上的表现
   └── 步骤 1.3: 确立基线 F1-Score 与延迟指标
          │
          ▼
[ 阶段二：中文专用模型构建与 ONNX 导出 ]
   ├── 步骤 2.1: 使用 `chinese-roberta-wwm-ext` 结合中文 NER 数据集微调
   ├── 步骤 2.2: 导出动态 Batch / 动态 SeqLen 的 ONNX 模型文件
   └── 步骤 2.3: 执行 INT8 量化，生成体积 < 200MB 的模型权重 (`sensidoc-gliner-zh.onnx`)
          │
          ▼
[ 阶段三：Rust 后端 `ort` 推理管道集成 ]
   ├── 步骤 3.1: 在 `Cargo.toml` 引入 `ort` 与 `tokenizers`
   ├── 步骤 3.2: 实现 `GlinerEngine` 结构体（加载 ONNX、预处理、Span 推理、NMS 后处理）
   └── 步骤 3.3: 改造 `src/extractor.rs`，实现 L1 正则 + L2 GLiNER 串联
          │
          ▼
[ 阶段四：联调优化与上线 ]
   ├── 步骤 4.1: 端到端基准测试回归（对齐 `benchmark.rs`）
   ├── 步骤 4.2: 优化多标签并发与长文本重叠分块 (Chunking) 策略
   └── 步骤 4.3: 统一打包与多平台（macOS / Windows / Linux）二进制验证
```

---

## 5. 核心实施细节与代码参考

### 5.1 Python 端微调与 ONNX 导出代码示例

```python
import torch
from gliner import GLiNERConfig, GLiNER

def train_and_export_chinese_gliner():
    # 1. 采用中文 RoBERTa-wwm 初始化 GLiNER
    config = GLiNERConfig(
        model_name="hfl/chinese-roberta-wwm-ext",
        max_width=12,                 # 中文实体常用最大词跨度
        hidden_size=768,
        words_splitter_type="jieba"
    )
    model = GLiNER(config)

    # 2. 加载中文敏感信息/通用 NER 训练集（如 MSRA / CLUE / 合同专项数据）
    # ... 执行常规 trainer.train() 微调 ...

    # 3. 导出为标准 ONNX 格式
    model.export_to_onnx(
        "sensidoc_gliner_zh.onnx",
        quantize=True  # 导出并生成 INT8 量化版本
    )
    print("模型已成功导出为 ONNX 格式！")
```

### 5.2 Rust 端 `Cargo.toml` 依赖配置

```toml
[dependencies]
# ONNX 运行时支持
ort = { version = "2.0.0-rc.9", features = ["ndarray", "copy-dylibs"] }
ndarray = "0.16"
tokenizers = { version = "0.21", default-features = false, features = ["onig"] }
```

### 5.3 Rust 端推理结构设计草案

```rust
pub struct GlinerExtractor {
    session: ort::session::Session,
    tokenizer: tokenizers::Tokenizer,
    max_width: usize,
    threshold: f32,
}

impl GlinerExtractor {
    pub fn new(model_path: &std::path::Path, tokenizer_path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let session = ort::session::Session::builder()?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;
        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)?;

        Ok(Self {
            session,
            tokenizer,
            max_width: 12,
            threshold: 0.5,
        })
    }

    /// 提取文档中的所有敏感实体并返回带坐标的结构体
    pub fn extract_entities(&self, text: &str, labels: &[&str]) -> Result<Vec<crate::extractor::SensitiveItem>, Box<dyn std::error::Error>> {
        // 1. 拼接标签与文本构建输入张量
        // 2. 运行 ONNX 会话前向推理
        // 3. 解析 Span 得分并执行贪心 NMS 过滤
        // 4. 映射为 SensiDoc 标准 SensitiveItem 列表
        todo!()
    }
}
```

---

## 6. 预期收益与风险应对

### 6.1 预期收益
1. **用户体验质的飞跃**：单篇数万字的长文档审计，AI 提取耗时从 **30~60秒缩短至 1~2秒**。
2. **免去复杂 Prompt 调试**：无需反复针对不同小模型调校提示词格式，判别式结构天然 100% 遵循。
3. **架构极简稳健**：彻底解决小模型长文本括号失配、JSON 乱码、复读死循环等难以排查的生成式边缘 Bug。

### 6.2 风险与应对预案
* **风险 1：特定专业领域术语（如特殊代号）召回不足**
  * **应对**：在 Level 1 正则中开放用户“自定义关键词/正则表达式白名单”，并在模型端提供置信度阈值调节滑块。
* **风险 2：跨语言混合文档（中英夹杂）**
  * **应对**：选用具备跨语言词表兼容能力的预训练权重，或保留多语言分词器映射机制。
