use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 扫描件/图像 OCR 与 VLM 增强档位
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OcrTier {
    /// 基础毫秒级 OCR (PP-OCRv6 + 结构化表格识别)
    Base,
    /// OCR + 局部微切片自适应快速复核纠偏 (自动先执行基础 OCR)
    FastVlm,
    /// 全量多模态视觉端到端高保真重构
    FullVlm,
}

impl std::fmt::Display for OcrTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrTier::Base => write!(f, "base"),
            OcrTier::FastVlm => write!(f, "fast-vlm"),
            OcrTier::FullVlm => write!(f, "full-vlm"),
        }
    }
}

use crate::converter::DocConverter;
use crate::desensitizer::Desensitizer;
use crate::extractor::{Extractor, RuleField, SensitiveItem};
use crate::model_manager::ModelManager;
use crate::session::SessionManager;

/// SensiDoc 命令行解析器
#[derive(Parser, Debug)]
#[command(
    name = "sensidoc",
    author = "SensiDoc Team",
    version,
    about = "SensiDoc - 本地离线文档敏感信息智能审计与原生排版脱敏工具",
    long_about = "支持 DOCX / PDF / XLSX / PPTX / TXT / CSV 格式的离线快速敏感词抽取与原生等长无损排版脱敏导出。\n支持命令行无头审计 (Agent/CI/CD 自动化) 及桌面视窗 GUI 运行。"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// 以无头 HTTP API 服务模式启动 (兼容历史参数)
    #[arg(long, global = true)]
    pub server: bool,

    /// 以无头模式启动 (兼容历史参数)
    #[arg(long, global = true)]
    pub headless: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 审计目标文档中的敏感信息并输出结构化结果 (Agent 核心接口)
    Audit(AuditArgs),
    /// 对目标文档执行原生等长排版脱敏并导出为新文件
    Mask(MaskArgs),
    /// 将目标文档快速转换为 Markdown 纯文本
    Convert(ConvertArgs),
    /// 查看系统中已保存的场景模板及规则清单
    Templates(TemplatesArgs),
    /// 显式启动 HTTP 服务或桌面应用程序
    Serve(ServeArgs),
    /// 运行基准评测或 10 组多模型协同天梯榜矩阵评测
    Benchmark(BenchmarkArgs),
}

/// benchmark 子命令参数
#[derive(Args, Debug)]
pub struct BenchmarkArgs {
    /// 运行 10 组多模型协同天梯榜矩阵测试
    #[arg(long, default_value = "true")]
    pub matrix: bool,

    /// 本地 Ollama 服务地址 (默认 http://127.0.0.1:11434)
    #[arg(long, default_value = "http://127.0.0.1:11434")]
    pub ollama_url: String,

    /// 限制测试文档数量 (例如 --limit 5 快速测试，默认全量 20 篇)
    #[arg(long)]
    pub limit: Option<usize>,

    /// 指定只跑特定策略组合或包含特定关键字的项 (如 "prop"、"router"、"span"、"1.5b")
    #[arg(long)]
    pub filter: Option<String>,

    /// 指定只测试某些特定文档 ID (逗号分隔，如 "doc_01,doc_11")
    #[arg(long)]
    pub doc_ids: Option<String>,
}

/// audit 子命令参数
#[derive(Args, Debug)]
pub struct AuditArgs {
    /// 目标文档路径 (支持 .docx/.pdf/.xlsx/.pptx/.txt/.csv/.md)
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// 使用已保存的场景模板名称或 ID (如 -t "合同模板")
    #[arg(short = 't', long, value_name = "NAME_OR_ID")]
    pub template: Option<String>,

    /// 自定义规则字段，逗号分隔，格式: 字段名[:风险等级] (如 "甲方企业:高,手机号:高,金额:中")
    #[arg(short = 'r', long, value_name = "RULES")]
    pub rules: Option<String>,

    /// 从外部 JSON 文件加载规则字段定义
    #[arg(long, value_name = "JSON_FILE")]
    pub rules_file: Option<PathBuf>,

    /// 仅使用内置毫秒级正则规则提取 (无需启动 LLM 模型，极速纯离线)
    #[arg(long)]
    pub regex_only: bool,

    /// 指定使用的本地离线模型文件名 (如 qwen2.5-1.5b-instruct-q4_k_m.gguf)
    #[arg(short = 'm', long, value_name = "GGUF")]
    pub model: Option<String>,

    /// 指定使用的在线大模型 ID (需已在 Web 设置中配置)
    #[arg(long, value_name = "ONLINE_ID")]
    pub online: Option<String>,

    /// 结果输出格式 (json, table)
    #[arg(short = 'f', long, default_value = "json", value_name = "FORMAT")]
    pub format: String,

    /// 将审计结果写入指定文件 (默认直接输出至 stdout)
    #[arg(short = 'o', long, value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// 静默模式 (仅向 stdout 输出纯结果，抑制 stderr 进度日志)
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// 当发现高风险敏感项时返回退出码 1 (适用于 CI/CD 安全卡点阻断)
    #[arg(long)]
    pub fail_on_sensitive: bool,

    /// 不将本次审计记录保存到 Web 界面文档列表与快照历史 (无痕模式)
    #[arg(long)]
    pub no_record: bool,

    /// 扫描件/图像 OCR 与 VLM 增强档位 (base: 基础毫秒级 OCR, fast-vlm: OCR+局部微切片快速复核, full-vlm: 全量多模态高保真重构)
    #[arg(long, value_name = "TIER", default_missing_value = "base", num_args = 0..=1)]
    pub ocr: Option<OcrTier>,
}

/// mask 子命令参数
#[derive(Args, Debug)]
pub struct MaskArgs {
    /// 待脱敏的源文档路径 (支持 .docx/.pdf/.xlsx/.pptx/.txt/.csv/.md)
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// 脱敏后的输出文档路径 (默认: [源文件名]_脱敏.[扩展名])
    #[arg(short = 'o', long, value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// 使用已保存的场景模板名称或 ID (如 -t "合同模板")
    #[arg(short = 't', long, value_name = "NAME_OR_ID")]
    pub template: Option<String>,

    /// 自定义规则字段，逗号分隔，格式: 字段名[:风险等级]
    #[arg(short = 'r', long, value_name = "RULES")]
    pub rules: Option<String>,

    /// 从外部 JSON 文件加载规则字段定义
    #[arg(long, value_name = "JSON_FILE")]
    pub rules_file: Option<PathBuf>,

    /// 仅使用内置毫秒级正则规则提取 (免模型极速脱敏)
    #[arg(long)]
    pub regex_only: bool,

    /// 指定使用的本地离线模型文件名
    #[arg(short = 'm', long, value_name = "GGUF")]
    pub model: Option<String>,

    /// 指定使用的在线大模型 ID
    #[arg(long, value_name = "ONLINE_ID")]
    pub online: Option<String>,

    /// 脱敏导出模式 (native: 原生排版无损替换, markdown: 纯文本脱敏)
    #[arg(long, default_value = "native", value_name = "MODE")]
    pub mode: String,

    /// 脱敏打码风格 (masking: 全星号掩码如 ***, redaction: 字符黑块硬抹除如 ████)
    #[arg(long, default_value = "masking", value_name = "STYLE")]
    pub style: String,

    /// 静默模式 (抑制进度与信息日志)
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// 不将本次脱敏执行记录保存到 Web 界面文档列表与快照历史 (无痕模式)
    #[arg(long)]
    pub no_record: bool,

    /// 扫描件/图像 OCR 与 VLM 增强档位 (base: 基础毫秒级 OCR, fast-vlm: OCR+局部微切片快速复核, full-vlm: 全量多模态高保真重构)
    #[arg(long, value_name = "TIER", default_missing_value = "base", num_args = 0..=1)]
    pub ocr: Option<OcrTier>,
}

/// convert 子命令参数
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// 待转换的目标文档路径
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// 输出的 Markdown 文件路径 (默认直接输出至 stdout)
    #[arg(short = 'o', long, value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// 静默模式
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// 扫描件/图像 OCR 与 VLM 增强档位 (base: 基础毫秒级 OCR, fast-vlm: OCR+局部微切片快速复核, full-vlm: 全量多模态高保真重构)
    #[arg(long, value_name = "TIER", default_missing_value = "base", num_args = 0..=1)]
    pub ocr: Option<OcrTier>,
}

/// templates 子命令参数
#[derive(Args, Debug)]
pub struct TemplatesArgs {
    /// 操作指令 (默认 list: 列出所有可用模板)
    #[arg(default_value = "list")]
    pub action: String,

    /// 输出格式 (table, json)
    #[arg(short = 'f', long, default_value = "table")]
    pub format: String,
}

/// serve 子命令参数
#[derive(Args, Debug)]
pub struct ServeArgs {
    /// 监听端口号 (默认 3000)
    #[arg(short = 'p', long, default_value_t = 3000)]
    pub port: u16,

    /// 监听主机地址 (默认 127.0.0.1)
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// 以纯无头 HTTP 后端服务运行 (不拉起本地桌面视窗)
    #[arg(long)]
    pub headless: bool,
}

/// 统一结构化审计结果输出模型
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditOutput {
    pub status: String,
    pub document: DocumentInfo,
    pub summary: AuditSummary,
    pub detected_items: Vec<AuditItem>,
    pub missed_fields: Vec<MissedField>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub filename: String,
    pub char_count: usize,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    pub total_detected: usize,
    pub high_risk_count: usize,
    pub medium_risk_count: usize,
    pub low_risk_count: usize,
    pub has_sensitive: bool,
    pub execution_ms: u64,
    pub model_used: String,
    pub recorded_to_workspace: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditItem {
    pub category: String,
    pub text: String,
    pub priority: String,
    pub count: usize,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MissedField {
    pub category: String,
    pub priority: String,
}

/// CLI 运行调度总入口
pub async fn run_cli(
    command: Commands,
    model_mgr: Arc<ModelManager>,
    session_mgr: Arc<SessionManager>,
) -> Result<i32, Box<dyn std::error::Error>> {
    match command {
        Commands::Audit(args) => run_audit(args, model_mgr, session_mgr).await,
        Commands::Mask(args) => run_mask(args, model_mgr, session_mgr).await,
        Commands::Convert(args) => run_convert(args, model_mgr, session_mgr).await,
        Commands::Templates(args) => run_templates(args, session_mgr).await,
        Commands::Benchmark(args) => run_benchmark_cmd(args).await,
        Commands::Serve(_) => {
            // Serve 由 main.rs 中的主循环统一托管
            Ok(0)
        }
    }
}

/// 解析用户指定的规则字段组合 (支持 -t/--template、--rules、--rules-file 及系统默认规则)
async fn resolve_rule_fields(
    template_name: Option<&str>,
    rules_str: Option<&str>,
    rules_file: Option<&Path>,
    session_mgr: &SessionManager,
) -> Result<Vec<RuleField>, String> {
    let mut fields: Vec<RuleField> = Vec::new();

    // 1. 若指定了模板，从持久化的 custom_templates 中查找
    if let Some(t_name) = template_name {
        let templates = session_mgr.get_custom_templates().await;
        let matched = templates.iter().find(|t| {
            t.name.eq_ignore_ascii_case(t_name)
                || t.id.eq_ignore_ascii_case(t_name)
                || t.name.contains(t_name)
        });

        if let Some(tpl) = matched {
            for f in &tpl.fields {
                if f.is_enabled {
                    fields.push(f.clone());
                }
            }
        } else {
            let available: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
            return Err(format!(
                "未找到名称或ID匹配 '{}' 的场景模板。当前可用模板: {:?}",
                t_name, available
            ));
        }
    }

    // 2. 若传入了 JSON 规则文件，解析并追加
    if let Some(file_path) = rules_file {
        if !file_path.exists() {
            return Err(format!("指定的规则文件不存在: {:?}", file_path));
        }
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("读取规则文件失败: {e}"))?;
        let parsed: Vec<RuleField> = serde_json::from_str(&content)
            .map_err(|e| format!("解析规则文件 JSON 失败: {e}"))?;
        fields.extend(parsed);
    }

    // 3. 若传入了 --rules "甲方:高,手机号:高" 命令行简写，解析并追加/覆盖
    if let Some(r_str) = rules_str {
        for part in r_str.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (name, priority) = if let Some((n, p)) = part.split_once(':') {
                let p_clean = match p.trim() {
                    "高" | "高危" | "high" => "high",
                    "中" | "中危" | "medium" => "medium",
                    "低" | "低危" | "low" => "low",
                    other => other,
                };
                (n.trim(), p_clean.to_string())
            } else {
                (part, "medium".to_string())
            };

            // 若已有同名字段则更新风险等级，否则新增
            if let Some(existing) = fields.iter_mut().find(|f| f.name == name) {
                existing.priority = priority;
                existing.is_enabled = true;
            } else {
                fields.push(RuleField {
                    name: name.to_string(),
                    description: format!("{name}实体敏感信息"),
                    priority,
                    is_enabled: true,
                });
            }
        }
    }

    // 4. 若最终字段为空，使用默认预设字段兜底
    if fields.is_empty() {
        fields = vec![
            RuleField {
                name: "身份证号".into(),
                description: "18位中国大陆居民身份证号码".into(),
                priority: "high".into(),
                is_enabled: true,
            },
            RuleField {
                name: "手机号码".into(),
                description: "11位移动电话号码".into(),
                priority: "high".into(),
                is_enabled: true,
            },
            RuleField {
                name: "银行账号".into(),
                description: "对公或个人银行借记卡/结算账号".into(),
                priority: "high".into(),
                is_enabled: true,
            },
            RuleField {
                name: "电子邮箱".into(),
                description: "电子邮件通信地址".into(),
                priority: "medium".into(),
                is_enabled: true,
            },
            RuleField {
                name: "企业名称".into(),
                description: "公司、企业或机构全称".into(),
                priority: "medium".into(),
                is_enabled: true,
            },
            RuleField {
                name: "法定代表人".into(),
                description: "企业法人代表或负责人姓名".into(),
                priority: "medium".into(),
                is_enabled: true,
            },
            RuleField {
                name: "金额款项".into(),
                description: "合同款项、交易或报价具体金额".into(),
                priority: "medium".into(),
                is_enabled: true,
            },
        ];
    }

    Ok(fields)
}

/// 针对目标文档执行格式解析与三级 OCR/VLM 流水线
/// 支持 base, fast-vlm (自动联动先执行基础 OCR 后微切片纠偏), full-vlm
pub async fn prepare_document_markdown(
    file_path: &Path,
    ocr_tier: Option<OcrTier>,
    model_mgr: Arc<ModelManager>,
    session_mgr: Arc<SessionManager>,
    quiet: bool,
) -> Result<(String, crate::converter::ConvertResult), String> {
    let file_bytes = std::fs::read(file_path)
        .map_err(|e| format!("读取目标文件失败 ({}): {e}", file_path.display()))?;
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document");

    let is_img = DocConverter::is_image(filename, &file_bytes);
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let is_pdf = ext == "pdf";

    // 1. 若用户显式指定了 full-vlm 档位：
    if ocr_tier == Some(OcrTier::FullVlm) {
        if is_img {
            if !quiet {
                eprintln!("🌌 启动端到端全量多模态视觉高保真重构 (Full-VLM)...");
            }
            let engine = crate::ocr::vlm::resolve_vlm_engine(&model_mgr, &session_mgr).await?;
            let md = crate::ocr::vlm::run_full_vlm_pipeline(&file_bytes, None, &engine).await?;
            return Ok((
                md.clone(),
                crate::converter::ConvertResult::scan(md, 0, Vec::new(), None, None),
            ));
        } else if is_pdf {
            let convert_res = DocConverter::convert_bytes_detailed(filename, &file_bytes)?;
            if convert_res.doc_type == "scan" {
                if !quiet {
                    eprintln!("🌌 检测到扫描版 PDF，启动全量多模态高保真重构 (Full-VLM)...");
                }
                let engine = crate::ocr::vlm::resolve_vlm_engine(&model_mgr, &session_mgr).await?;
                let md = crate::ocr::vlm::run_full_vlm_pipeline(
                    &file_bytes,
                    Some(&convert_res.markdown),
                    &engine,
                )
                .await?;
                return Ok((
                    md.clone(),
                    crate::converter::ConvertResult::scan(md, 0, Vec::new(), None, None),
                ));
            } else {
                if !quiet {
                    eprintln!("ℹ️ 目标 PDF 包含原生文本层，已采用原生排版提取 (跳过视觉重构)");
                }
                return Ok((convert_res.markdown.clone(), convert_res));
            }
        } else {
            if !quiet {
                eprintln!("ℹ️ 目标文档为原生排版格式 (.{})，已采用原生直接解析 (跳过视觉重构)", ext);
            }
            let convert_res = DocConverter::convert_bytes_detailed(filename, &file_bytes)?;
            return Ok((convert_res.markdown.clone(), convert_res));
        }
    }

    // 2. 基础 OCR 或 Fast-VLM（以及默认未指定 --ocr 时）：先执行基础详细转换
    let mut convert_res = DocConverter::convert_bytes_detailed(filename, &file_bytes)?;
    let is_scan = convert_res.doc_type == "scan" || is_img;

    // 3. 若用户指定了 fast-vlm 档位：
    // 规则明确：自动先执行基础 OCR（上一步已完成），再进行低置信度局部微切片快速复核
    if ocr_tier == Some(OcrTier::FastVlm) {
        if is_scan {
            if !quiet {
                let total_boxes = convert_res.raw_boxes.len();
                let low_count = convert_res.low_confidence_count;
                eprintln!(
                    "🔍 基础 OCR 完成 (识别 {} 个文字块，{} 处待复核)，正在自动启动局部微切片快速复核 (Fast-VLM)...",
                    total_boxes, low_count
                );
            }
            let engine = crate::ocr::vlm::resolve_vlm_engine(&model_mgr, &session_mgr).await?;
            let vlm_res = crate::ocr::vlm::run_fast_vlm_pipeline(
                Some(&file_bytes),
                &convert_res.markdown,
                &convert_res.raw_boxes,
                &engine,
                0.88,
            )
            .await?;

            if !quiet {
                if vlm_res.corrected_count > 0 {
                    eprintln!(
                        "✨ 已通过轻量多模态引擎 ({}) 完成 {} 处微切片定向纠偏",
                        vlm_res.model_used, vlm_res.corrected_count
                    );
                } else {
                    eprintln!("✅ 局部微切片审查完毕，基础识别内容准确无误");
                }
            }
            convert_res.markdown = vlm_res.markdown.clone();
            convert_res.low_confidence_count = 0;
            return Ok((vlm_res.markdown, convert_res));
        } else {
            if !quiet {
                eprintln!("ℹ️ 目标文档为原生文档 (.{})，无需进行扫描件微切片复核", ext);
            }
            return Ok((convert_res.markdown.clone(), convert_res));
        }
    }

    // 4. 若为 Base 模式且是扫描件，打印友好提示
    if ocr_tier == Some(OcrTier::Base) && is_scan && !quiet {
        eprintln!(
            "⚡ 原生基础 OCR 解析完成 (共 {} 个文字块，{} 处低置信度)",
            convert_res.raw_boxes.len(),
            convert_res.low_confidence_count
        );
    }

    Ok((convert_res.markdown.clone(), convert_res))
}

/// 执行审计任务并返回敏感项及元数据 (支持自动持久化到 Web 界面工作区)
async fn perform_extraction(
    file_path: &Path,
    fields: &[RuleField],
    regex_only: bool,
    template_opt: Option<&str>,
    no_record: bool,
    model_opt: Option<&str>,
    online_opt: Option<&str>,
    ocr_tier: Option<OcrTier>,
    quiet: bool,
    model_mgr: Arc<ModelManager>,
    session_mgr: Arc<SessionManager>,
) -> Result<(String, String, Vec<SensitiveItem>, u64, String, Option<String>, Option<String>), String> {
    let start_time = std::time::Instant::now();

    if !file_path.exists() {
        return Err(format!("目标文件不存在: {:?}", file_path));
    }
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !quiet {
        eprintln!("🔍 正在解析文档格式 (.{})...", ext);
    }

    let (markdown, _) = prepare_document_markdown(
        file_path,
        ocr_tier,
        model_mgr.clone(),
        session_mgr.clone(),
        quiet,
    )
    .await?;

    // 1. 正则极速提取
    let regex_items = Extractor::extract_by_regex(&markdown, fields);

    // 2. 模型推理提取
    let mut ai_items = Vec::new();
    let mut model_used = "内置正则引擎".to_string();

    if !regex_only {
        let system_prompt = Extractor::build_system_prompt(fields, None);
        let chunks = Extractor::chunk_text(&markdown, 2500);

        if let Some(online_id) = online_opt {
            // 在线模型推理
            let online_models = session_mgr.get_online_models().await;
            let cfg = online_models
                .iter()
                .find(|m| m.id == online_id || m.name == online_id)
                .ok_or_else(|| format!("未找到指定的在线大模型配置: {online_id}"))?;

            model_used = format!("[在线] {}", cfg.name);
            if !quiet {
                eprintln!("🤖 调用在线大模型进行智能提取: {}...", cfg.name);
            }

            for (_offset, chunk) in chunks {
                if let Ok(items) = Extractor::query_online_llm(
                    &cfg.base_url,
                    &cfg.api_key,
                    &cfg.model_id,
                    cfg.temperature,
                    cfg.top_k,
                    cfg.repeat_penalty,
                    cfg.max_tokens,
                    cfg.enable_thinking,
                    &system_prompt,
                    &chunk,
                )
                .await
                {
                    ai_items.extend(items);
                }
            }
        } else {
            // 本地离线模型推理：双轨探针，优先复用已有 18188 端口服务
            let is_ready = model_mgr.is_server_ready().await;

            let (target_model, started_temporary_server) = if !is_ready {
                // 探针发现本地未运行服务，单次临时拉起
                let target_model = if let Some(m) = model_opt {
                    m.to_string()
                } else if let Some(active) = model_mgr.get_active_model().await {
                    active
                } else {
                    // 扫描 models 目录中的第一个可用 gguf 模型
                    let local_models = model_mgr.list_local_models();
                    local_models
                        .first()
                        .cloned()
                        .ok_or_else(|| "未检测到已下载的本地 GGUF 模型，请使用 --regex-only 或下载模型".to_string())?
                };

                if !quiet {
                    eprintln!("🚀 临时启动本地离线模型推理服务 ({target_model})...");
                }

                model_mgr
                    .start_model(&target_model)
                    .await
                    .map_err(|e| format!("启动 llama-server 失败: {e}"))?;
                model_used = format!("[本地] {target_model}");
                (target_model, true)
            } else {
                let active = model_mgr.get_active_model().await.unwrap_or_else(|| "default.gguf".to_string());
                model_used = "[本地] 常驻 llama-server (复用已有连接)".to_string();
                if !quiet {
                    eprintln!("⚡ 探测到本地推理服务正在运行，直接复用连接...");
                }
                (active, false)
            };

            let port = model_mgr.server_port();
            let offline_profile = session_mgr.get_offline_model_profile(&target_model).await;
            for (_offset, chunk) in chunks {
                if let Ok(items) = Extractor::query_llm(
                    port,
                    offline_profile.temperature,
                    offline_profile.top_k,
                    offline_profile.repeat_penalty,
                    offline_profile.max_tokens,
                    offline_profile.enable_thinking,
                    &system_prompt,
                    &chunk,
                ).await {
                    ai_items.extend(items);
                }
            }

            // 若由当前 CLI 临时拉起的子服务，提取完成后优雅退出释放资源
            if started_temporary_server {
                if !quiet {
                    eprintln!("🛑 推理完成，优雅释放临时推理进程...");
                }
                model_mgr.stop_server().await;
            }
        }
    }

    // 3. 冲突消解与合并
    let final_items = Extractor::merge_and_resolve(&markdown, regex_items, ai_items, fields);
    let execution_ms = start_time.elapsed().as_millis() as u64;

    // 4. 同步持久化至 Web 界面工作区（除非启用 --no-record）
    let (doc_id, snap_id) = if !no_record {
        let file_bytes = std::fs::read(file_path).map_err(|e| format!("读取文件失败: {e}"))?;
        let filename_str = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document")
            .to_string();

        let doc = session_mgr
            .find_or_create_document(filename_str, markdown.clone())
            .await;

        // 缓存原始二进制到 uploads/{doc_id}.bin，供 Web 界面在线脱敏导出
        let uploads_dir = crate::paths::get_uploads_dir();
        if !uploads_dir.exists() {
            let _ = std::fs::create_dir_all(&uploads_dir);
        }
        let upload_path = uploads_dir.join(format!("{}.bin", doc.id));
        let _ = std::fs::write(&upload_path, &file_bytes);

        // 生成专属 CLI 历史快照
        let template_title = if let Some(t) = template_opt {
            format!("[CLI] {t}")
        } else {
            "[CLI] 自定义规则提取".to_string()
        };

        let system_prompt_used = Extractor::build_system_prompt(fields, None);
        let snap_res = session_mgr
            .add_snapshot(
                &doc.id,
                template_title,
                Some(model_used.clone()),
                Some(system_prompt_used),
                fields.to_vec(),
                final_items.clone(),
                execution_ms,
                None,
            )
            .await;

        match snap_res {
            Ok(s) => (Some(doc.id), Some(s.id)),
            Err(_) => (Some(doc.id), None),
        }
    } else {
        (None, None)
    };

    Ok((
        ext,
        markdown,
        final_items,
        execution_ms,
        model_used,
        doc_id,
        snap_id,
    ))
}

/// 执行 audit 子命令
async fn run_audit(
    args: AuditArgs,
    model_mgr: Arc<ModelManager>,
    session_mgr: Arc<SessionManager>,
) -> Result<i32, Box<dyn std::error::Error>> {
    let fields = resolve_rule_fields(
        args.template.as_deref(),
        args.rules.as_deref(),
        args.rules_file.as_deref(),
        &session_mgr,
    )
    .await?;

    let (ext, markdown, items, execution_ms, model_used, doc_id, snap_id) = perform_extraction(
        &args.file,
        &fields,
        args.regex_only,
        args.template.as_deref(),
        args.no_record,
        args.model.as_deref(),
        args.online.as_deref(),
        args.ocr,
        args.quiet,
        model_mgr,
        session_mgr,
    )
    .await?;

    // 统计各风险等级数量
    let mut high_risk_count = 0;
    let mut medium_risk_count = 0;
    let mut low_risk_count = 0;

    let detected_categories: HashSet<String> = items.iter().map(|i| i.category.clone()).collect();
    let mut detected_items_output = Vec::new();

    for item in &items {
        match item.priority.as_str() {
            "high" => high_risk_count += item.count,
            "low" => low_risk_count += item.count,
            _ => medium_risk_count += item.count,
        }
        detected_items_output.push(AuditItem {
            category: item.category.clone(),
            text: item.text.clone(),
            priority: item.priority.clone(),
            count: item.count,
            source: item.source.clone(),
        });
    }

    let missed_fields: Vec<MissedField> = fields
        .iter()
        .filter(|f| !detected_categories.contains(&f.name))
        .map(|f| MissedField {
            category: f.name.clone(),
            priority: f.priority.clone(),
        })
        .collect();

    let total_detected = items.iter().map(|i| i.count).sum();
    let has_sensitive = !items.is_empty();

    let output_data = AuditOutput {
        status: "success".to_string(),
        document: DocumentInfo {
            id: doc_id,
            filename: args
                .file
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            char_count: markdown.chars().count(),
            format: ext,
        },
        summary: AuditSummary {
            snapshot_id: snap_id,
            total_detected,
            high_risk_count,
            medium_risk_count,
            low_risk_count,
            has_sensitive,
            execution_ms,
            model_used,
            recorded_to_workspace: !args.no_record,
        },
        detected_items: detected_items_output,
        missed_fields,
    };

    let formatted_str = if args.format.eq_ignore_ascii_case("table") {
        render_audit_table(&output_data)
    } else {
        serde_json::to_string_pretty(&output_data)?
    };

    if let Some(out_path) = args.output {
        std::fs::write(&out_path, &formatted_str)?;
        if !args.quiet {
            eprintln!("✅ 审计结果已成功写入: {:?}", out_path);
        }
    } else {
        println!("{}", formatted_str);
    }

    if args.fail_on_sensitive && high_risk_count > 0 {
        return Ok(1);
    }

    Ok(0)
}

/// 执行 mask 脱敏导出子命令
async fn run_mask(
    args: MaskArgs,
    model_mgr: Arc<ModelManager>,
    session_mgr: Arc<SessionManager>,
) -> Result<i32, Box<dyn std::error::Error>> {
    let fields = resolve_rule_fields(
        args.template.as_deref(),
        args.rules.as_deref(),
        args.rules_file.as_deref(),
        &session_mgr,
    )
    .await?;

    let (ext, markdown, items, _ms, _model, _doc_id, _snap_id) = perform_extraction(
        &args.file,
        &fields,
        args.regex_only,
        args.template.as_deref(),
        args.no_record,
        args.model.as_deref(),
        args.online.as_deref(),
        args.ocr,
        args.quiet,
        model_mgr,
        session_mgr,
    )
    .await?;

    let target_out_path = if let Some(out) = args.output {
        out
    } else {
        let parent = args.file.parent().unwrap_or_else(|| Path::new(""));
        let stem = args.file.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
        let ext_with_dot = if ext.is_empty() {
            "".to_string()
        } else {
            format!(".{ext}")
        };
        parent.join(format!("{stem}_脱敏{ext_with_dot}"))
    };

    if !args.quiet {
        eprintln!(
            "🛡️ 正在执行等长排版脱敏 (命中 {} 处敏感信息)...",
            items.iter().map(|i| i.count).sum::<usize>()
        );
    }

    let file_bytes = std::fs::read(&args.file)?;
    let filename_str = args
        .file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("document");

    let mask_style = std::str::FromStr::from_str(&args.style).unwrap_or(crate::desensitizer::MaskStyle::Masking);

    let desensitized_bytes = if args.mode.eq_ignore_ascii_case("markdown") {
        Desensitizer::desensitize_plain_text_with_style(&markdown, &items, mask_style).into_bytes()
    } else {
        let (_name, bytes, _) = Desensitizer::desensitize_document_auto_detailed(
            filename_str,
            Some(&file_bytes),
            &markdown,
            &items,
            mask_style,
            None,
        )?;
        bytes
    };

    std::fs::write(&target_out_path, desensitized_bytes)?;

    if !args.quiet {
        eprintln!("✅ 脱敏完成！新文件已保存至: {:?}", target_out_path);
        if !args.no_record {
            eprintln!("📋 本次调用记录已同步更新至 Web 界面「文档列表」与快照历史");
        }
    } else {
        println!("{}", target_out_path.display());
    }

    Ok(0)
}

/// 执行 convert 格式转换子命令
async fn run_convert(
    args: ConvertArgs,
    model_mgr: Arc<ModelManager>,
    session_mgr: Arc<SessionManager>,
) -> Result<i32, Box<dyn std::error::Error>> {
    if !args.file.exists() {
        return Err(format!("目标文件不存在: {:?}", args.file).into());
    }

    let (markdown, _) = prepare_document_markdown(
        &args.file,
        args.ocr,
        model_mgr,
        session_mgr,
        args.quiet,
    )
    .await
    .map_err(|e| format!("文档转换失败: {e}"))?;

    if let Some(out_path) = args.output {
        std::fs::write(&out_path, &markdown)?;
        if !args.quiet {
            eprintln!("✅ 已转换并导出 Markdown 文件至: {:?}", out_path);
        }
    } else {
        print!("{}", markdown);
    }

    Ok(0)
}

/// 执行 templates 模板查看子命令
async fn run_templates(
    args: TemplatesArgs,
    session_mgr: Arc<SessionManager>,
) -> Result<i32, Box<dyn std::error::Error>> {
    let templates = session_mgr.get_custom_templates().await;

    if args.format.eq_ignore_ascii_case("json") {
        println!("{}", serde_json::to_string_pretty(&templates)?);
        return Ok(0);
    }

    // 表格格式化输出
    println!("┌────────────────────────────────┬─────────┬────────────────────────────────────────────────────────┐");
    println!("│ 模板名称 (ID)                  │ 字段数  │ 规则字段详情                                           │");
    println!("├────────────────────────────────┼─────────┼────────────────────────────────────────────────────────┤");

    if templates.is_empty() {
        println!("│ (暂无自定义模板，可在 Web 端创建保存)                                                   │");
    } else {
        for tpl in &templates {
            let field_names: Vec<&str> = tpl.fields.iter().map(|f| f.name.as_str()).collect();
            let summary = field_names.join(", ");
            let summary_trunc = if summary.chars().count() > 30 {
                let s: String = summary.chars().take(28).collect();
                format!("{s}..")
            } else {
                summary
            };
            println!(
                "│ {:<30} │ {:<7} │ {:<54} │",
                tpl.name,
                format!("{} 项", tpl.fields.len()),
                summary_trunc
            );
        }
    }
    println!("└────────────────────────────────┴─────────┴────────────────────────────────────────────────────────┘");

    Ok(0)
}

/// 纯文本表格渲染辅助函数
fn render_audit_table(data: &AuditOutput) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "\n📄 文档名称: {} | 格式: .{} | 字符数: {}\n",
        data.document.filename, data.document.format, data.document.char_count
    ));
    let record_desc = if data.summary.recorded_to_workspace {
        if let Some(sid) = &data.summary.snapshot_id {
            format!("已同步存入 Web 界面「文档列表」(快照 ID: {}..)", &sid[..sid.len().min(8)])
        } else {
            "已同步存入 Web 界面「文档列表」".to_string()
        }
    } else {
        "未存入工作区 (无痕模式)".to_string()
    };
    out.push_str(&format!(
        "⚡ 耗时: {}ms | 模型: {} | 记录状态: {}\n",
        data.summary.execution_ms, data.summary.model_used, record_desc
    ));
    out.push_str(&format!(
        "🎯 敏感词总数: {} (高危: {}, 中危: {}, 低危: {})\n\n",
        data.summary.total_detected,
        data.summary.high_risk_count,
        data.summary.medium_risk_count,
        data.summary.low_risk_count
    ));

    out.push_str("┌──────────────┬────────┬──────┬────────────────────────────────────────┐\n");
    out.push_str("│ 类别         │ 风险   │ 频次 │ 命中敏感原词                           │\n");
    out.push_str("├──────────────┼────────┼──────┼────────────────────────────────────────┤\n");

    if data.detected_items.is_empty() {
        out.push_str("│ (未检出任何匹配规则的敏感信息)                                        │\n");
    } else {
        for item in &data.detected_items {
            let risk_tag = match item.priority.as_str() {
                "high" => "高危",
                "low" => "低危",
                _ => "中危",
            };
            out.push_str(&format!(
                "│ {:<12} │ {:<6} │ {:<4} │ {:<38} │\n",
                item.category,
                risk_tag,
                item.count,
                item.text
            ));
        }
    }
    out.push_str("└──────────────┴────────┴──────┴────────────────────────────────────────┘\n");

    if !data.missed_fields.is_empty() {
        let missed_names: Vec<&str> = data.missed_fields.iter().map(|m| m.category.as_str()).collect();
        out.push_str(&format!("\n⚪ 未在文档中出现的字段: {}\n", missed_names.join(", ")));
    }

    out
}

/// 执行多模型协同评测基准子命令
async fn run_benchmark_cmd(args: BenchmarkArgs) -> Result<i32, Box<dyn std::error::Error>> {
    eprintln!("🚀 正在启动 SensiDoc 多模型协同天梯榜基准评测...");
    if let Some(lim) = args.limit {
        eprintln!("📌 运行限制: 前 {} 篇文档", lim);
    }
    if let Some(ref filt) = args.filter {
        eprintln!("🔍 候选过滤: 仅匹配包含 \"{}\" 的组合", filt);
    }
    if let Some(ref d_ids) = args.doc_ids {
        eprintln!("📄 指定文档: {}", d_ids);
    }
    eprintln!("🔗 本地 Ollama: {}", args.ollama_url);

    let doc_ids_vec: Option<Vec<String>> = args.doc_ids.as_ref().map(|s| {
        s.split(',').map(|id| id.trim().to_string()).collect()
    });

    let report = crate::benchmark::BenchmarkEngine::run_dual_model_matrix_benchmark(
        &args.ollama_url,
        args.limit,
        args.filter.as_deref(),
        doc_ids_vec.as_deref(),
    ).await.map_err(|e| format!("基准评测失败: {}", e))?;

    crate::benchmark::BenchmarkEngine::print_matrix_report_table(&report);
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_convert_ocr_flag_default_missing_value() {
        let cli = Cli::parse_from(["sensidoc", "convert", "invoice.png", "--ocr"]);
        if let Some(Commands::Convert(args)) = cli.command {
            assert_eq!(args.ocr, Some(OcrTier::Base));
        } else {
            panic!("预期解析为 Convert 子命令");
        }
    }

    #[test]
    fn test_cli_convert_ocr_fast_vlm_parse() {
        let cli = Cli::parse_from(["sensidoc", "convert", "invoice.png", "--ocr", "fast-vlm"]);
        if let Some(Commands::Convert(args)) = cli.command {
            assert_eq!(args.ocr, Some(OcrTier::FastVlm));
        } else {
            panic!("预期解析为 Convert 子命令");
        }
    }

    #[test]
    fn test_cli_convert_ocr_full_vlm_parse() {
        let cli = Cli::parse_from(["sensidoc", "convert", "invoice.png", "--ocr", "full-vlm"]);
        if let Some(Commands::Convert(args)) = cli.command {
            assert_eq!(args.ocr, Some(OcrTier::FullVlm));
        } else {
            panic!("预期解析为 Convert 子命令");
        }
    }

    #[test]
    fn test_cli_convert_ocr_none_by_default() {
        let cli = Cli::parse_from(["sensidoc", "convert", "invoice.png"]);
        if let Some(Commands::Convert(args)) = cli.command {
            assert_eq!(args.ocr, None);
        } else {
            panic!("预期解析为 Convert 子命令");
        }
    }

    #[test]
    fn test_cli_audit_ocr_args() {
        let cli = Cli::parse_from(["sensidoc", "audit", "receipt.jpg", "--ocr", "fast-vlm"]);
        if let Some(Commands::Audit(args)) = cli.command {
            assert_eq!(args.ocr, Some(OcrTier::FastVlm));
        } else {
            panic!("预期解析为 Audit 子命令");
        }
    }

    #[test]
    fn test_cli_mask_ocr_args() {
        let cli = Cli::parse_from(["sensidoc", "mask", "contract.pdf", "--ocr", "base"]);
        if let Some(Commands::Mask(args)) = cli.command {
            assert_eq!(args.ocr, Some(OcrTier::Base));
        } else {
            panic!("预期解析为 Mask 子命令");
        }
    }

    #[test]
    fn test_ocr_tier_display_and_serde() {
        assert_eq!(format!("{}", OcrTier::Base), "base");
        assert_eq!(format!("{}", OcrTier::FastVlm), "fast-vlm");
        assert_eq!(format!("{}", OcrTier::FullVlm), "full-vlm");

        let serialized = serde_json::to_string(&OcrTier::FastVlm).unwrap();
        assert_eq!(serialized, "\"fast-vlm\"");
        let deserialized: OcrTier = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, OcrTier::FastVlm);
    }
}
