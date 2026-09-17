use serde::{Deserialize, Serialize};

use crate::extractor::Extractor;
use crate::model_manager::ModelManager;
use crate::ocr::{self, OcrBoxItem};
use crate::session::SessionManager;

/// 单个聚合后的复核区域 (包含合并后的连续短语文本、最小外接矩形与代表性得分)
#[derive(Debug, Clone)]
pub struct MergedOcrCluster {
    pub text: String,
    pub score: f32,
    pub box_coords: [f32; 4],
    pub member_boxes: Vec<OcrBoxItem>,
    pub context_text: String,
    pub is_cell: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FastVlmCorrection {
    pub old_text: String,
    pub new_text: String,
    pub score: f32,
    pub box_coords: [f32; 4],
    #[serde(default)]
    pub context_text: Option<String>,
    #[serde(default)]
    pub is_cell: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FastVlmInspectedItem {
    pub text: String,
    pub score: f32,
    pub is_corrected: bool,
    pub new_text: Option<String>,
    #[serde(default)]
    pub context_text: Option<String>,
    #[serde(default)]
    pub is_cell: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastVlmResult {
    pub success: bool,
    pub markdown: String,
    pub corrected_count: usize,
    pub corrections: Vec<FastVlmCorrection>,
    pub inspected: Vec<FastVlmInspectedItem>,
    pub low_confidence_count: usize,
    pub model_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmEngineInfo {
    pub url: String,
    pub api_key: String,
    pub model_id: String,
    pub is_vision_native: bool,
    pub model_name: String,
}

/// 解析用于单据复核与全量重构的推理引擎 (默认优先使用本地 Qwen3.5-0.8B 视觉多模态模型，免配置秒级启动)
pub async fn resolve_vlm_engine(
    model_mgr: &ModelManager,
    session_mgr: &SessionManager,
) -> Result<VlmEngineInfo, String> {
    let local_models = model_mgr.list_local_models();
    let qwen_file = "Qwen3.5-0.8B-Q4_K_M.gguf";

    // 1. 默认优先使用本地 Qwen3.5-0.8B (带视觉塔)
    if local_models.iter().any(|m| m == qwen_file) {
        let active = model_mgr.get_active_model().await;
        let is_running = active.as_deref() == Some(qwen_file) && model_mgr.is_server_ready().await;
        if !is_running {
            let profile = session_mgr.get_offline_model_profile(qwen_file).await;
            model_mgr
                .start_model_with_profile(qwen_file, Some(&profile))
                .await
                .map_err(|e| format!("启动本地 Qwen3.5-0.8B 引擎失败: {e}"))?;
        }
        let port = model_mgr.server_port();
        let has_mmproj = model_mgr.has_mmproj_for(qwen_file);
        return Ok(VlmEngineInfo {
            url: format!("http://127.0.0.1:{port}/v1/chat/completions"),
            api_key: String::new(),
            model_id: "qwen3.5-0.8b".to_string(),
            is_vision_native: has_mmproj,
            model_name: if has_mmproj {
                "Qwen3.5-0.8B VLM (本地视觉)".to_string()
            } else {
                "Qwen3.5-0.8B (本地端侧)".to_string()
            },
        });
    }

    // 2. 检查是否有已激活的在线模型 (若用户配置了 Vision API)
    if let Some(online) = session_mgr.get_active_online_model().await {
        let trimmed_url = online.base_url.trim_end_matches('/');
        let url = if trimmed_url.ends_with("/chat/completions") {
            trimmed_url.to_string()
        } else {
            format!("{}/chat/completions", trimmed_url)
        };
        return Ok(VlmEngineInfo {
            url,
            api_key: online.api_key,
            model_id: online.model_id,
            is_vision_native: true,
            model_name: online.name,
        });
    }

    // 3. 兜底尝试首个本地可用模型
    if let Some(first) = local_models.first() {
        let active = model_mgr.get_active_model().await;
        let is_running =
            active.as_deref() == Some(first.as_str()) && model_mgr.is_server_ready().await;
        if !is_running {
            let profile = session_mgr.get_offline_model_profile(first).await;
            model_mgr
                .start_model_with_profile(first, Some(&profile))
                .await
                .map_err(|e| format!("启动本地模型 {first} 失败: {e}"))?;
        }
        let port = model_mgr.server_port();
        let has_mmproj = model_mgr.has_mmproj_for(first);
        return Ok(VlmEngineInfo {
            url: format!("http://127.0.0.1:{port}/v1/chat/completions"),
            api_key: String::new(),
            model_id: first.to_string(),
            is_vision_native: has_mmproj,
            model_name: if has_mmproj {
                format!("{first} (含视觉塔)")
            } else {
                first.to_string()
            },
        });
    }

    Err("未检测到本地 Qwen3.5-0.8B 模型或在线 AI 模型，请先在设置中准备模型".to_string())
}

/// 从复核纠正后的 Markdown 文本中提取对应低置信度原词的替换新词
pub fn extract_replacement(old_raw: &str, orig_md: &str, clean_md: &str) -> Option<String> {
    let orig_line = orig_md.lines().find(|l| l.contains(old_raw))?;
    let orig_tokens: Vec<&str> = orig_line
        .split_whitespace()
        .filter(|t| !t.contains(old_raw))
        .collect();
    for clean_line in clean_md.lines() {
        if (!orig_tokens.is_empty() && orig_tokens.iter().any(|t| clean_line.contains(t)))
            || (orig_line.len() > 5
                && clean_line.len() > 5
                && orig_line.len().abs_diff(clean_line.len()) < 15)
        {
            for clean_word in clean_line.split(|c: char| {
                c.is_whitespace() || c == '|' || c == '(' || c == ')' || c == ':' || c == '-'
            }) {
                let w = clean_word.trim();
                if w.len() >= 2 && !orig_line.contains(w) {
                    return Some(w.to_string());
                }
            }
        }
    }
    None
}

/// 在 Markdown 内容中检索与可疑片段最匹配的完整单元格或整行上下文
pub fn find_markdown_context(markdown: &str, suspect_text: &str) -> Option<(String, bool)> {
    let clean = suspect_text.trim();
    // 忽略过短（<= 2 字符）的单字符/纯标点模糊检索，防止例如单个 'S'、'$' 误触大标题
    if clean.chars().count() < 3 {
        return None;
    }
    for line in markdown.lines() {
        let tline = line.trim();
        if tline.is_empty() {
            continue;
        }
        if tline.contains(clean) {
            if tline.starts_with('|') {
                for cell in tline.split('|') {
                    let ctrim = cell.trim();
                    if ctrim.contains(clean)
                        && !ctrim.is_empty()
                        && !ctrim.chars().all(|c| c == '-' || c == ':')
                    {
                        return Some((ctrim.to_string(), true));
                    }
                }
            }
            return Some((tline.to_string(), false));
        }
    }
    None
}

/// 对零散的低置信度文本框进行自适应空间聚类与合并，将临近文字组合为连贯短语或行组
pub fn cluster_nearby_boxes(
    raw_boxes: &[OcrBoxItem],
    max_clusters: usize,
) -> Vec<MergedOcrCluster> {
    if raw_boxes.is_empty() {
        return Vec::new();
    }

    let to_rect = |c: [f32; 4]| -> [f32; 4] {
        [c[0].min(c[2]), c[1].min(c[3]), c[0].max(c[2]), c[1].max(c[3])]
    };

    let are_neighbors = |b1: &OcrBoxItem, b2: &OcrBoxItem| -> bool {
        let r1 = to_rect(b1.box_coords);
        let r2 = to_rect(b2.box_coords);
        let h1 = (r1[3] - r1[1]).max(10.0);
        let h2 = (r2[3] - r2[1]).max(10.0);
        let avg_h = (h1 + h2) / 2.0;

        let y_overlap = (r1[3].min(r2[3]) - r1[1].max(r2[1])).max(0.0);
        let y_dist = (r1[1].max(r2[1]) - r1[3].min(r2[3])).max(0.0);
        let c1_y = (r1[1] + r1[3]) / 2.0;
        let c2_y = (r2[1] + r2[3]) / 2.0;
        let is_same_line = y_overlap > 0.3 * avg_h || (c1_y - c2_y).abs() < 0.6 * avg_h;

        let x_dist = (r1[0].max(r2[0]) - r1[2].min(r2[2])).max(0.0);
        if is_same_line && x_dist < avg_h * 3.2 {
            return true;
        }

        let w1 = r1[2] - r1[0];
        let w2 = r2[2] - r2[0];
        let x_overlap = (r1[2].min(r2[2]) - r1[0].max(r2[0])).max(0.0);
        if y_dist < avg_h * 1.3 && x_overlap > 0.3 * w1.min(w2) {
            return true;
        }

        false
    };

    let mut visited = vec![false; raw_boxes.len()];
    let mut clusters: Vec<MergedOcrCluster> = Vec::new();

    for i in 0..raw_boxes.len() {
        if visited[i] {
            continue;
        }
        visited[i] = true;
        let mut group = vec![raw_boxes[i].clone()];
        let mut queue = vec![i];

        while let Some(curr) = queue.pop() {
            for j in 0..raw_boxes.len() {
                if !visited[j] && are_neighbors(&raw_boxes[curr], &raw_boxes[j]) {
                    visited[j] = true;
                    group.push(raw_boxes[j].clone());
                    queue.push(j);
                }
            }
        }

        group.sort_by(|a, b| {
            let r_a = to_rect(a.box_coords);
            let r_b = to_rect(b.box_coords);
            r_a[1]
                .partial_cmp(&r_b[1])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    r_a[0]
                        .partial_cmp(&r_b[0])
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut min_score = f32::MAX;
        let mut texts = Vec::new();

        for b in &group {
            let r = to_rect(b.box_coords);
            min_x = min_x.min(r[0]);
            min_y = min_y.min(r[1]);
            max_x = max_x.max(r[2]);
            max_y = max_y.max(r[3]);
            min_score = min_score.min(b.score);
            if !b.text.trim().is_empty() {
                texts.push(b.text.trim().to_string());
            }
        }

        clusters.push(MergedOcrCluster {
            text: texts.join(" "),
            score: if min_score == f32::MAX { 0.8 } else { min_score },
            box_coords: [min_x, min_y, max_x, max_y],
            member_boxes: group,
            context_text: String::new(),
            is_cell: false,
        });
    }

    clusters.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if clusters.len() > max_clusters {
        clusters.truncate(max_clusters);
    }
    clusters
}

/// 执行 Fast-VLM 局部微切片快速复核流水线 (以原生基础 OCR 底稿为基准)
pub async fn run_fast_vlm_pipeline(
    img_bytes: Option<&[u8]>,
    base_markdown: &str,
    raw_boxes: &[OcrBoxItem],
    engine: &VlmEngineInfo,
    threshold: f32,
) -> Result<FastVlmResult, String> {
    let low_boxes: Vec<OcrBoxItem> = raw_boxes
        .iter()
        .filter(|b| b.score > 0.0 && b.score < threshold && b.text.trim().chars().count() >= 2)
        .cloned()
        .collect();

    let total_low_count = low_boxes.len();
    let mut current_markdown = base_markdown.to_string();

    let initial_clusters = cluster_nearby_boxes(&low_boxes, 30);
    let mut context_clusters: Vec<MergedOcrCluster> = Vec::new();

    for mut cl in initial_clusters {
        let ctx = find_markdown_context(&current_markdown, &cl.text).or_else(|| {
            for mb in &cl.member_boxes {
                let m_ctx = find_markdown_context(&current_markdown, &mb.text);
                if m_ctx.is_some() {
                    return m_ctx;
                }
            }
            None
        });
        let (context_text, is_cell) = ctx.unwrap_or_else(|| (cl.text.clone(), false));
        cl.context_text = context_text.clone();
        cl.is_cell = is_cell;

        let existing = context_clusters.iter_mut().find(|c| {
            c.context_text == context_text && (c.box_coords[1] - cl.box_coords[1]).abs() < 40.0
        });

        if let Some(existing) = existing {
            existing.box_coords = [
                existing.box_coords[0].min(cl.box_coords[0]),
                existing.box_coords[1].min(cl.box_coords[1]),
                existing.box_coords[2].max(cl.box_coords[2]),
                existing.box_coords[3].max(cl.box_coords[3]),
            ];
            existing.score = existing.score.min(cl.score);
            if !existing.text.contains(&cl.text) {
                existing.text = format!("{}, {}", existing.text, cl.text);
            }
            existing.member_boxes.extend(cl.member_boxes);
        } else {
            context_clusters.push(cl);
        }
    }

    context_clusters.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if context_clusters.len() > 10 {
        context_clusters.truncate(10);
    }
    let target_clusters = context_clusters;

    let mut corrections = Vec::new();

    if !target_clusters.is_empty() {
        if engine.is_vision_native && img_bytes.is_some() {
            let bytes = img_bytes.unwrap();
            for cluster in &target_clusters {
                if let Ok((_, b64_url)) = ocr::crop_image_box(bytes, cluster.box_coords) {
                    let system_prompt = "你是一名高精度单据与发票文字复核专家。请仔细观察单据微切片图片中的文字，结合整行/单元格上下文修正 OCR 初步识别中的漏字、错字、连笔误识别或符号错误。\n【严格要求】：仅直接输出该切片区域内的真实原文字符，切勿臆造或包含相邻多余文字，严禁输出任何解释、不要加引号、不要输出代码块标记。若复核对象为表格单元格，严禁输出表格竖线分隔符 '|'，必须保持为单单元格连续纯文本。";
                    let target_label = if cluster.is_cell { "单元格" } else { "整行" };
                    let user_prompt = format!(
                        "微切片对应单据{}初步识别内容为：'{}'（其中可疑识别为：'{}'）。请结合上下文纠正该处的真实文字：",
                        target_label, cluster.context_text, cluster.text
                    );

                    if let Ok(corrected) = Extractor::query_online_vision(
                        &engine.url,
                        &engine.api_key,
                        &engine.model_id,
                        system_prompt,
                        &user_prompt,
                        &b64_url,
                    )
                    .await
                    {
                        let clean = sanitize_fast_vlm_output(&corrected, cluster.is_cell);
                        let max_allowed_len = (cluster.context_text.len() * 3).max(60);
                        if !clean.is_empty()
                            && clean != cluster.text.trim()
                            && clean != cluster.context_text.trim()
                            && clean.len() <= max_allowed_len
                        {
                            if current_markdown.contains(&cluster.context_text)
                                && clean.len() >= cluster.context_text.len() / 2
                            {
                                current_markdown =
                                    current_markdown.replacen(&cluster.context_text, &clean, 1);
                                corrections.push(FastVlmCorrection {
                                    old_text: cluster.context_text.clone(),
                                    new_text: clean.clone(),
                                    score: cluster.score,
                                    box_coords: cluster.box_coords,
                                    context_text: Some(cluster.context_text.clone()),
                                    is_cell: cluster.is_cell,
                                });
                            } else if cluster.text.trim().chars().count() >= 3
                                && current_markdown.contains(&cluster.text)
                            {
                                current_markdown =
                                    current_markdown.replacen(&cluster.text, &clean, 1);
                                corrections.push(FastVlmCorrection {
                                    old_text: cluster.text.clone(),
                                    new_text: clean,
                                    score: cluster.score,
                                    box_coords: cluster.box_coords,
                                    context_text: Some(cluster.context_text.clone()),
                                    is_cell: cluster.is_cell,
                                });
                            }
                        }
                    }
                }
            }
        } else {
            // 本地端侧/文本大模型定向纠偏
            let system_prompt = r#"你是一名专业的高精度单据与发票 OCR 智能复核纠偏专家。
用户将提供 OCR 初步识别的单据文本及重点可疑的整行/单元格上下文与可疑片段列表，请仔细分析上下文语境，修正 OCR 识别中的常见错字、漏字、英文单词拼写错误（例如 HEAITH -> HEALTH、Leqal -> Legal、sgmm -> mm/sqmm、dHWWidth -> HWWidth）、标点与数字粘连。
【严格准则】：
1. 仅修正明显错误的错别字与拼写错误，严禁臆造或篡改原文核心数值；
2. 完整保留原有的 Markdown 结构与表格排版；
3. 直接输出纠偏后的最终 Markdown 内容，绝不要输出任何解释说明、不要使用 ```markdown 等代码块包裹。"#;

            let suspect_terms: Vec<String> = target_clusters
                .iter()
                .enumerate()
                .map(|(idx, c)| {
                    let kind = if c.is_cell { "单元格" } else { "整行" };
                    let suspect_words: Vec<String> = c
                        .member_boxes
                        .iter()
                        .map(|b| format!("'{}'({:.0}%)", b.text.trim(), b.score * 100.0))
                        .collect();
                    format!(
                        "{}. 【{}】：\n   完整内容：「{}」\n   待复核低置信度疑似错词：[{}]",
                        idx + 1,
                        kind,
                        c.context_text,
                        suspect_words.join(", ")
                    )
                })
                .collect();
            let suspect_str = format!(
                "重点审查并纠偏以下疑难整行/单元格，请务必结合整行与整格的前后文语义进行精准纠偏：\n{}",
                suspect_terms.join("\n")
            );

            let user_prompt = format!(
                "【复核指引】：\n{}\n\n【单据原内容】：\n{}",
                suspect_str, current_markdown
            );

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .map_err(|e| format!("构建客户端失败: {e}"))?;

            let payload = serde_json::json!({
                "model": engine.model_id,
                "messages": [
                    { "role": "system", "content": system_prompt },
                    { "role": "user", "content": user_prompt }
                ],
                "temperature": 0.1,
                "max_tokens": 2048
            });

            let mut req = client.post(&engine.url).json(&payload);
            if !engine.api_key.trim().is_empty() {
                req = req.header("Authorization", format!("Bearer {}", engine.api_key.trim()));
            }

            if let Ok(resp) = req.send().await {
                if resp.status().is_success() {
                    if let Ok(json_val) = resp.json::<serde_json::Value>().await {
                        if let Some(content) = json_val["choices"][0]["message"]["content"].as_str() {
                            let trimmed = content.trim();
                            let clean = if (trimmed.starts_with("```markdown")
                                || trimmed.starts_with("```md"))
                                && trimmed.ends_with("```")
                            {
                                let p = if trimmed.starts_with("```markdown") { 11 } else { 5 };
                                trimmed[p..trimmed.len() - 3].trim().to_string()
                            } else if trimmed.starts_with("```") && trimmed.ends_with("```") {
                                trimmed[3..trimmed.len() - 3].trim().to_string()
                            } else {
                                trimmed.to_string()
                            };

                            if !clean.is_empty() && clean != current_markdown {
                                let norm_orig: String = current_markdown
                                    .chars()
                                    .filter(|c| !c.is_whitespace())
                                    .collect::<String>()
                                    .to_lowercase();
                                let norm_clean: String = clean
                                    .chars()
                                    .filter(|c| !c.is_whitespace())
                                    .collect::<String>()
                                    .to_lowercase();

                                for cl in &target_clusters {
                                    for b in &cl.member_boxes {
                                        let old_raw = b.text.trim();
                                        if old_raw.len() >= 2 {
                                            let norm_old: String = old_raw
                                                .chars()
                                                .filter(|c| !c.is_whitespace())
                                                .collect::<String>()
                                                .to_lowercase();
                                            if norm_orig.contains(&norm_old)
                                                && !norm_clean.contains(&norm_old)
                                            {
                                                let new_text = extract_replacement(
                                                    old_raw,
                                                    &current_markdown,
                                                    &clean,
                                                )
                                                .unwrap_or_default();
                                                corrections.push(FastVlmCorrection {
                                                    old_text: old_raw.to_string(),
                                                    new_text,
                                                    score: b.score,
                                                    box_coords: b.box_coords,
                                                    context_text: Some(cl.context_text.clone()),
                                                    is_cell: cl.is_cell,
                                                });
                                            }
                                        }
                                    }
                                }
                                current_markdown = clean;
                            }
                        }
                    }
                }
            }
        }
    }

    let mut inspected = Vec::new();
    for cl in &target_clusters {
        let is_corr = corrections.iter().any(|c| {
            c.old_text == cl.text.trim() || c.old_text == cl.context_text.trim()
        });
        inspected.push(FastVlmInspectedItem {
            text: cl.text.clone(),
            score: cl.score,
            is_corrected: is_corr,
            new_text: corrections
                .iter()
                .find(|c| c.old_text == cl.text.trim() || c.old_text == cl.context_text.trim())
                .map(|c| c.new_text.clone()),
            context_text: Some(cl.context_text.clone()),
            is_cell: cl.is_cell,
        });
    }

    Ok(FastVlmResult {
        success: true,
        markdown: current_markdown,
        corrected_count: corrections.len(),
        corrections,
        inspected,
        low_confidence_count: total_low_count,
        model_used: engine.model_name.clone(),
    })
}

pub const FULL_VLM_SYSTEM_PROMPT: &str = "你是一名顶级多模态单据与复杂文档高保真结构化排版专家。请仔细观察单据图像，高保真重构为排版工整、结构严谨的标准 Markdown 格式。\n\n【通用排版准则】：\n1. 【复合单据分表解耦】：单据顶部抬头、核心明细大表、汇总/账户信息、底部审批签名栏必须分为独立的 Markdown 段落或各自独立的 Markdown 表格，严禁跨区域强行合并为单一大表；\n2. 【核心数据项零丢失与防挤占】：\n   - 单据表格中的所有数据项（如序号、项目/商品描述、发票号/采购单号/编码、币种/单位、数量、单价、金额）必须 100% 完整保留在表格中，严禁遗漏任何一列数据；\n   - 【防挤占与列对齐】：当表格首列包含序号与描述时，严禁让描述文本挤占后续的关键单号/编码列！你可以将序号与描述合并在同一单元格（如 `1 描述文本`），或规范拆分为【序号】与【描述】独立两列，但单号/编码必须独立保留在对应列中，绝不能因排版挤压导致单号被吞；\n   - 核心表格存在多行明细时必须逐行全部输出，严禁擅自省略；\n3. 【底部审批与签批栏通用规范 (防虚构人名与防行列错位)】：\n   - 【防人名脑补与空白如实还原】：对于各类单据底部的签字、盖章、审核栏，若包含不可精确辨识的草书或手写签名，统一标注为 [手写签名]；若单元格/区域完全空白无笔迹，必须如实标注为 [未签署/空白]，绝对严禁凭空臆造、联想或生成任何虚构人名与拼音；\n   - 【结构化列表解耦】：审批签名栏统一使用键值列表排版（格式如 `- **角色名称:** [签署状态] (签署日期)`），中英文对照角色标签保持在同一项，严禁拆开；\n   - 【日期归属】：签署日期必须紧随对应的签署角色，严禁将各格底部的日期抽出作为独立的数据行输出；\n4. 层次清晰分明，标题使用 # / ## 标记，直接流式输出 Markdown 正文，绝对禁止包含任何解释说明或 ```markdown 等代码块标记。";

/// 执行 Full-VLM 端到端全量视觉或排版重构流水线
pub async fn run_full_vlm_pipeline(
    file_bytes: &[u8],
    base_markdown: Option<&str>,
    engine: &VlmEngineInfo,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {e}"))?;

    let trimmed_url = engine.url.trim_end_matches('/');
    let url = if trimmed_url.ends_with("/chat/completions") {
        trimmed_url.to_string()
    } else {
        format!("{}/chat/completions", trimmed_url)
    };

    let request_payload = if engine.is_vision_native {
        let b64_data_url = {
            let img = image::load_from_memory(file_bytes)
                .map_err(|e| format!("加载图像失败: {e}"))?;
            let (w, h) = (img.width(), img.height());
            let target = if w > 2048 || h > 2048 {
                img.resize(2048, 2048, image::imageops::FilterType::Triangle)
            } else {
                img
            };
            let mut buf = std::io::Cursor::new(Vec::new());
            target
                .write_to(&mut buf, image::ImageFormat::Jpeg)
                .map_err(|e| format!("编码图像失败: {e}"))?;
            format!(
                "data:image/jpeg;base64,{}",
                crate::ocr::base64_encode(&buf.into_inner())
            )
        };

        serde_json::json!({
            "model": engine.model_id,
            "messages": [
                {
                    "role": "system",
                    "content": FULL_VLM_SYSTEM_PROMPT
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": "请仔细观察图片，完整识别并重构该单据的所有文字与表格排版（逐行全部输出核心明细表格，确保单号/编码与金额无一遗漏）："
                        },
                        { "type": "image_url", "image_url": { "url": b64_data_url } }
                    ]
                }
            ],
            "temperature": 0.1,
            "max_tokens": 4096,
            "frequency_penalty": 0.2,
            "presence_penalty": 0.1
        })
    } else {
        let base_text = base_markdown.unwrap_or("");
        serde_json::json!({
            "model": engine.model_id,
            "messages": [
                {
                    "role": "system",
                    "content": "你是一名顶级单据与发票高保真结构化排版专家。请根据 OCR 初步识别的单据内容，将其重构为结构清晰、排版工整的高保真 Markdown 格式。\n\n【排版准则】：\n1. 核心表格中的【发票号/采购单号】（如 2605011672 等数字或单号）必须逐行完整保留，严禁丢失！序号与公司名同属于细节描述列；\n2. 纠正所有 OCR 识别错字、漏字和拼写错误；\n3. 所有表格与键值对必须严格排版为标准的 GitHub Flavored Markdown 表格；\n4. 层次清晰分明，标题使用 # / ## 标记，列表使用 - 标记；\n5. 仅直接输出重构后的 Markdown 正文，绝对禁止输出任何解释说明、不要使用 ```markdown 代码块包裹。"
                },
                {
                    "role": "user",
                    "content": format!("单据原始识别内容如下，请重构为高保真 Markdown（发票号/采购单号必须逐行保留）：\n\n{}", base_text)
                }
            ],
            "temperature": 0.1,
            "max_tokens": 4096,
            "frequency_penalty": 0.2,
            "presence_penalty": 0.1
        })
    };

    let mut req = client.post(&url).json(&request_payload);
    if !engine.api_key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", engine.api_key.trim()));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("请求视觉模型失败: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("视觉模型返回错误 [{status}]: {body}"));
    }

    let val = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("解析模型响应失败: {e}"))?;

    let content = val["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "模型响应中缺少 content 字段".to_string())?;

    let trimmed = content.trim();
    let clean = if (trimmed.starts_with("```markdown") || trimmed.starts_with("```md"))
        && trimmed.ends_with("```")
    {
        let p = if trimmed.starts_with("```markdown") { 11 } else { 5 };
        trimmed[p..trimmed.len() - 3].trim().to_string()
    } else if trimmed.starts_with("```") && trimmed.ends_with("```") {
        trimmed[3..trimmed.len() - 3].trim().to_string()
    } else {
        trimmed.to_string()
    };

    Ok(clean)
}

/// 对快速微切片 VLM 纠偏输出进行严格的字符安全清洗
/// - 去除外围引号与 Markdown 标记
/// - 若为表格单元格，强制消除管道符 '|' 并合并多余空格，严防破坏 Markdown 表格列结构
pub fn sanitize_fast_vlm_output(raw: &str, is_cell: bool) -> String {
    let clean_raw = raw
        .trim()
        .trim_matches(|c| c == '\'' || c == '"' || c == '`')
        .trim();
    if is_cell {
        clean_raw
            .replace('|', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        clean_raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_fast_vlm_cell_pipe_removal() {
        let raw_with_pipe = "26 |CHAY DA LOGISTICS CO.,LID.";
        let sanitized = sanitize_fast_vlm_output(raw_with_pipe, true);
        assert_eq!(sanitized, "26 CHAY DA LOGISTICS CO.,LID.");

        // 验证非单元格不强制删除合法管道符
        let sanitized_non_cell = sanitize_fast_vlm_output(raw_with_pipe, false);
        assert_eq!(sanitized_non_cell, "26 |CHAY DA LOGISTICS CO.,LID.");
    }

    #[test]
    fn test_markdown_table_cell_replacement_preserves_columns() {
        let orig_markdown = "| 26 CHAY PA LOGISTICS CO. LID. 26 | RE-2605013121 | USD | 100.00 |";
        let raw_vlm = "26 |CHAY DA LOGISTICS CO.,LID.";
        let clean = sanitize_fast_vlm_output(raw_vlm, true);
        let new_markdown = orig_markdown.replacen("26 CHAY PA LOGISTICS CO. LID. 26", &clean, 1);

        assert_eq!(
            new_markdown,
            "| 26 CHAY DA LOGISTICS CO.,LID. | RE-2605013121 | USD | 100.00 |"
        );
        // 关键断言：列数前后必须完全一致，绝不能发生炸列
        assert_eq!(
            orig_markdown.split('|').count(),
            new_markdown.split('|').count()
        );
    }
}
