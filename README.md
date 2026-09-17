# SensiDoc - 智能文档信息审计与敏感数据脱敏工具

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows-lightgrey.svg)]()
[![Rust](https://img.shields.io/badge/Rust-2024-orange.svg)](https://www.rust-lang.org/)

**SensiDoc** 是一款面向企业与个人的高性能、本地优先（Local-First）文档敏感信息审计与结构化脱敏工具。通过端侧轻量小模型（如 Qwen2.5 / LFM）与云端大模型（如 DeepSeek-V3）的端云协同，为合同、财务报表、政企文档等敏感材料提供毫秒级、零泄漏的隐私保护与脱敏处理。

---

## 核心特性

- **多格式本地直读**：支持 Word (`.docx`, `.doc`)、Excel (`.xlsx`, `.xls`)、PowerPoint (`.pptx`)、PDF、Markdown、TXT 及 CSV，基于本地 AnyDoc 引擎零依赖解析。
- **端云协同智能审计**：
  - **端侧离线模型**：纯本地推理（GGUF / llama.cpp），数据不出设备，安全合规；
  - **在线大模型提炼**：支持自定义规则元提示词提炼，从业务场景描述秒级生成提取规则；
  - **内置规则引擎**：集成手机号、身份证、银行卡等高危敏感字段的正规化高精度快速提取。
- **多版本快照与智能比对**：
  - 提取记录版本化，多模型、不同策略审计结果自由回溯；
  - 精准高亮与循环焦点定位，支持关键词 Geist 双脉冲动画引导。
- **高精度无损脱敏导出**：
  - 支持直接替换原文生成脱敏副本；
  - 一键导出 RFC 4180 标准 CSV 审计清单。
- **现代极简体验**：
  - 原生 macOS (Universal 架构) 与 Windows (Inno Setup 安装版 / 绿色免安装版) 客户端支持；
  - 深色/浅色自适应主题、平滑动效与无障碍排版。

---

## 快速上手

### 1. 下载即用
前往 [Releases 页面](https://github.com/meteor-ioi/SensiDoc/releases) 获取适用于您操作系统的最新发行版：
- **macOS**：下载 `.dmg` 镜像并拖入应用程序；
- **Windows**：下载安装程序 `.exe` 或解压便携版 `.zip`。

### 2. 源码构建运行
需要具备 Rust 工具链 (edition 2024)：

```bash
# 克隆仓库
git clone https://github.com/meteor-ioi/SensiDoc.git
cd SensiDoc

# 1. 命令行直接审计文档或扫描件 (输出结构化 JSON / 表格)
cargo run -- audit contract.docx -t "合同模板" --regex-only -q
cargo run -- audit invoice.png --ocr fast-vlm -r "发票代码:高,金额:高" -f table

# 2. 单据与扫描件快速转换为 Markdown (支持 base / fast-vlm / full-vlm)
cargo run -- convert receipt.jpg --ocr fast-vlm -o receipt.md

# 3. 启动无头 HTTP API 后端服务或桌面视窗
cargo run -- --server
```

启动服务后，访问 `http://127.0.0.1:3000` 即可在浏览器中使用。更多高级命令行集成指南请参阅 [CLI 完整指南](docs/CLI_GUIDE.md)。

---

## 技术架构与工程指南

- [小微端侧多模态模型 (VLM 0.8B~2B) 单据识别与结构化提取工程实践指南](docs/SMALL_VLM_RECOGNITION_BEST_PRACTICES.md)
- [超轻量端侧小模型输出格式与遵循度评估报告](docs/SMALL_MODEL_BENCHMARK_REPORT.md)
- [命令行与无头服务器使用指南](docs/CLI_GUIDE.md)

---

## 许可证

本项目采用 MIT 许可证。
