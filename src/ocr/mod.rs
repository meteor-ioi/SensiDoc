pub mod vlm;
pub use vlm::*;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// OCR 引擎空闲自动释放超时时间：15 分钟无新请求自动释放内存 (900 秒)
pub const OCR_IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

struct OcrStateHolder {
    engine: Option<Arc<anyocr::Engine>>,
    last_used: Instant,
}

static OCR_STATE: Mutex<Option<OcrStateHolder>> = Mutex::new(None);
static MONITOR_STARTED: AtomicBool = AtomicBool::new(false);

/// 安全获取全局 OCR 状态锁；遭遇锁中毒 (PoisonError) 时记录显式告警并自愈重置上下文
fn acquire_ocr_state() -> std::sync::MutexGuard<'static, Option<OcrStateHolder>> {
    match OCR_STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::error!("OCR_STATE 互斥锁已中毒 (PoisonError)，正在重置状态以自愈恢复服务...");
            OCR_STATE.clear_poison();
            let mut guard = poisoned.into_inner();
            *guard = None;
            guard
        }
    }
}

fn ensure_idle_monitor_started() {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if MONITOR_STARTED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            handle.spawn(async {
                loop {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    let mut freed = false;
                    {
                        let mut lock = acquire_ocr_state();
                        if let Some(holder) = lock.as_mut() {
                            if let Some(ref engine) = holder.engine {
                                // 确保无外部正在进行的推理引用 (Arc 强引用数为 1)
                                if holder.last_used.elapsed() >= OCR_IDLE_TIMEOUT
                                    && Arc::strong_count(engine) == 1
                                {
                                    holder.engine = None;
                                    freed = true;
                                }
                            }
                        }
                    }
                    if freed {
                        tracing::info!(
                            "OCR 原生推理引擎已空闲超过 {} 秒，已自动卸载并释放内存",
                            OCR_IDLE_TIMEOUT.as_secs()
                        );
                    }
                }
            });
        }
    }
}

/// 单个检测识别框的结构体 (可供前端原图高亮与卷帘联动)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrBoxItem {
    pub text: String,
    pub score: f32,
    pub box_coords: [f32; 4],
}

/// 端到端单据与表格 OCR 识别综合产出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrResult {
    /// 还原排版好的标准 GFM / 语义化 HTML Markdown 文本
    pub markdown: String,
    /// 所有检测识别到的多边形/矩形坐标框清单
    pub raw_boxes: Vec<OcrBoxItem>,
    /// 图像原始宽度
    pub image_width: u32,
    /// 图像原始高度
    pub image_height: u32,
    /// 端到端纯推理耗时 (毫秒)
    pub elapsed_ms: u64,
}

impl OcrResult {
    /// 统计可疑低置信度识别区块数量 (置信度 < 0.88)
    pub fn low_confidence_count(&self) -> usize {
        self.raw_boxes
            .iter()
            .filter(|b| b.score > 0.0 && b.score < 0.88 && !b.text.trim().is_empty())
            .count()
    }
}

/// 自适应上下文边界扩展切片 (Smart Context Expansion)
/// 针对低置信度区块坐标 [x, y, w, h]，结合字高进行自适应扩展：
/// - 高度方向：轻度扩展 15% (避免上下邻行文字污染)
/// - 宽度方向：根据字高向左、右扩展 1.2 ~ 1.5 倍字高 (确保涵盖前后邻近字符/Key语义锚点)
pub fn expand_crop_box(
    coords: [f32; 4],
    img_width: u32,
    img_height: u32,
) -> [u32; 4] {
    let min_x = coords[0].min(coords[2]);
    let min_y = coords[1].min(coords[3]);
    let max_x = coords[0].max(coords[2]);
    let max_y = coords[1].max(coords[3]);
    let _w = (max_x - min_x).max(1.0);
    let h = (max_y - min_y).max(1.0);

    let pad_y = (h * 0.15).max(2.0);
    // 宽度适度扩展，避免横向过度膨胀侵入相邻单元格/列
    let pad_x = (h * 0.4).max(6.0);

    let x0 = (min_x - pad_x).max(0.0) as u32;
    let y0 = (min_y - pad_y).max(0.0) as u32;
    let x1 = (max_x + pad_x).min(img_width as f32) as u32;
    let y1 = (max_y + pad_y).min(img_height as f32) as u32;

    [x0, y0, x1.saturating_sub(x0).max(1), y1.saturating_sub(y0).max(1)]
}

const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// 原生零依赖 Base64 快速编码器
pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        out.push(B64_CHARS[(b0 >> 2) as usize] as char);
        out.push(B64_CHARS[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64_CHARS[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64_CHARS[(b2 & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// 对单据原始二进制图像执行微切片自适应裁剪，并返回切片 PNG 二进制及 Base64 Data URL
pub fn crop_image_box(bytes: &[u8], coords: [f32; 4]) -> Result<(Vec<u8>, String), String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("加载图像失败: {e}"))?;
    let (img_w, img_h) = (img.width(), img.height());
    let [x, y, w, h] = expand_crop_box(coords, img_w, img_h);

    let cropped = img.crop_imm(x, y, w, h);
    let mut buf = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("编码切片图像为 PNG 失败: {e}"))?;
    let data = buf.into_inner();
    let b64 = format!("data:image/png;base64,{}", base64_encode(&data));
    Ok((data, b64))
}

/// 纯 Rust 原生嵌入式 OCR 引擎 (代理委托至 anyocr 核心引擎)
pub struct OcrEngine;

impl OcrEngine {
    /// 检查 OCR 引擎是否当前已载入内存
    pub fn is_loaded() -> bool {
        let lock = acquire_ocr_state();
        if let Some(ref holder) = *lock {
            return holder.engine.is_some();
        }
        false
    }

    /// 主动卸载 OCR 原生推理引擎并释放内存
    pub fn unload() -> bool {
        let mut lock = acquire_ocr_state();
        if let Some(holder) = lock.as_mut() {
            if let Some(ref engine) = holder.engine {
                if Arc::strong_count(engine) == 1 {
                    holder.engine = None;
                    tracing::info!("OCR 原生推理引擎已主动卸载并释放内存");
                    return true;
                } else {
                    tracing::warn!("OCR 原生推理引擎正在执行任务，稍后自动卸载");
                    return false;
                }
            }
        }
        false
    }

    /// 初始化或获取 OCR 引擎，若未载入或已超时释放则自动按需加载
    pub fn get_or_init() -> Result<Arc<anyocr::Engine>, String> {
        ensure_idle_monitor_started();

        let mut lock = acquire_ocr_state();

        let holder = lock.get_or_insert_with(|| OcrStateHolder {
            engine: None,
            last_used: Instant::now(),
        });

        if let Some(ref engine) = holder.engine {
            holder.last_used = Instant::now();
            return Ok(engine.clone());
        }

        if !crate::paths::is_ocr_ready() {
            return Err("OCR 模型组件尚未完整就绪，请先在设置面板中下载轻量模型套件".to_string());
        }

        let det_path = crate::paths::get_ocr_det_path();
        let rec_path = crate::paths::get_ocr_rec_path();
        let table_path = crate::paths::get_ocr_table_path();
        let dict_path = crate::paths::get_ocr_dict_path();

        tracing::info!("正在通过 anyocr 载入 OCR 原生推理模型套件 (PP-OCRv6 + SLANet_plus)...");
        let config = anyocr::EngineConfig {
            profile: anyocr::ModelProfile::Custom {
                det_path,
                rec_path,
                table_path: Some(table_path),
                dict_path: Some(dict_path),
            },
            provider: anyocr::ExecutionProvider::Auto,
            enable_table: true,
            max_batch_size: 1,
            max_dimension: Some(2560),
        };

        let engine = anyocr::Engine::new(config)
            .map_err(|e| format!("初始化 anyocr 引擎失败: {e}"))?;
        let engine_arc = Arc::new(engine);

        holder.engine = Some(engine_arc.clone());
        holder.last_used = Instant::now();
        tracing::info!("anyocr 原生推理引擎初始化成功，已就绪");
        Ok(engine_arc)
    }

    /// 对图片字节流执行完整的文本检测、文字识别、表格预测与拓扑对齐
    pub fn recognize_bytes(bytes: &[u8]) -> Result<OcrResult, String> {
        let engine = Self::get_or_init()?;
        let doc = engine
            .parse(bytes, anyocr::DocumentFormat::Image)
            .map_err(|e| format!("anyocr 推理失败: {e}"))?;

        Self::convert_doc_to_result(doc)
    }

    /// 对 DynamicImage 执行完整的文本检测、文字识别、表格预测与拓扑对齐
    pub fn recognize_image(img: &image::DynamicImage) -> Result<OcrResult, String> {
        let engine = Self::get_or_init()?;
        let doc = engine
            .parse_image(img)
            .map_err(|e| format!("anyocr 图片推理失败: {e}"))?;

        Self::convert_doc_to_result(doc)
    }

    fn convert_doc_to_result(doc: anyocr::ParsedDocument) -> Result<OcrResult, String> {
        let page = doc.pages.first().ok_or("OCR 未产生任何页面输出")?;

        let raw_boxes = page
            .boxes
            .iter()
            .map(|b| OcrBoxItem {
                text: b.text.clone(),
                score: b.score,
                box_coords: b.coords,
            })
            .collect();

        let mut lock = acquire_ocr_state();
        if let Some(holder) = lock.as_mut() {
            holder.last_used = Instant::now();
        }

        let cleaned_markdown = sanitize_redundant_empty_lines(&doc.markdown);

        Ok(OcrResult {
            markdown: cleaned_markdown,
            raw_boxes,
            image_width: page.dimensions.0,
            image_height: page.dimensions.1,
            elapsed_ms: doc.elapsed_ms,
        })
    }
}

/// 自动清除 Markdown 与 OCR 识别文本中多余的空行、空表格行与无效占位标签
///
/// 针对纸质单据、扫描件与长文档识别中产生的大量空白表格行 (`<tr><td></td></tr>`)、
/// 纯空白 Markdown 单元格行 (`| | |`)、孤立空标签以及连续 3 个及以上多余换行，
/// 进行高保真语义清洗，避免冗余空标签污染 LLM 上下文与影响实体抽取准确率。
pub fn sanitize_redundant_empty_lines(text: &str) -> String {
    if text.trim().is_empty() {
        return String::new();
    }

    use std::sync::LazyLock;
    static TR_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?is)<tr\b[^>]*>.*?</tr>").expect("合法正则")
    });
    static TAG_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"<[^>]+>").expect("合法正则")
    });
    static EMPTY_TBODY_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?is)<tbody\b[^>]*>\s*</tbody>").expect("合法正则")
    });
    static EMPTY_THEAD_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?is)<thead\b[^>]*>\s*</thead>").expect("合法正则")
    });
    static TABLE_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?is)<table\b[^>]*>.*?</table>").expect("合法正则")
    });
    static EMPTY_P_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?is)<p\b[^>]*>\s*(?:&nbsp;|&#160;)?\s*</p>").expect("合法正则")
    });
    static EMPTY_DIV_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?is)<div\b[^>]*>\s*(?:&nbsp;|&#160;)?\s*</div>").expect("合法正则")
    });
    static MULTI_NEWLINE_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"\r?\n(?:\s*\r?\n){2,}").expect("合法正则")
    });

    // 1. 过滤 HTML 表格中的空行 (<tr>...</tr> 内部没有可见文本内容)
    let step1 = TR_REGEX.replace_all(text, |caps: &regex::Captures| {
        let tr_str = &caps[0];
        let stripped = TAG_REGEX.replace_all(tr_str, "");
        let clean_text = stripped
            .replace("&nbsp;", " ")
            .replace("&#160;", " ")
            .trim()
            .to_string();
        if clean_text.is_empty() {
            String::new()
        } else {
            tr_str.to_string()
        }
    });

    // 2. 清除清理空 tr 后可能残留的空 thead / tbody / table
    let step2 = EMPTY_TBODY_REGEX.replace_all(&step1, "");
    let step3 = EMPTY_THEAD_REGEX.replace_all(&step2, "");
    let step4 = TABLE_REGEX.replace_all(&step3, |caps: &regex::Captures| {
        let table_str = &caps[0];
        let stripped = TAG_REGEX.replace_all(table_str, "");
        let clean_text = stripped
            .replace("&nbsp;", " ")
            .replace("&#160;", " ")
            .trim()
            .to_string();
        if clean_text.is_empty() {
            String::new()
        } else {
            table_str.to_string()
        }
    });

    // 3. 清理 Markdown 管道符表格中的纯空数据行 (形如 |   |   |)
    let mut cleaned_lines = Vec::new();
    for line in step4.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            let cells: Vec<&str> = trimmed
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim())
                .collect();
            let is_separator = !cells.is_empty()
                && cells.iter().all(|c| {
                    !c.is_empty()
                        && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
                        && c.contains('-')
                });
            let is_all_empty = cells.iter().all(|c| c.is_empty());

            if is_all_empty && !is_separator {
                continue;
            }
        }
        cleaned_lines.push(line);
    }
    let step5 = cleaned_lines.join("\n");

    // 4. 清理空段落及空块级标签 <p></p>、<div></div>
    let step6 = EMPTY_P_REGEX.replace_all(&step5, "");
    let step7 = EMPTY_DIV_REGEX.replace_all(&step6, "");

    // 5. 消除连续多余换行符：将 3 个及以上连续换行（包括中间仅包含空格/制表符的空行）归一化为标准的 2 个换行 (\n\n)
    let step8 = MULTI_NEWLINE_REGEX.replace_all(&step7, "\n\n");

    step8.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    static TEST_MUTEX: StdMutex<()> = StdMutex::new(());

    #[test]
    fn test_sensidoc_ocr_pipeline_via_anyocr() {
        let _lock = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let sample_path = "/Users/icychick/.gemini/antigravity-cli/brain/6efdb7c6-eaca-4a22-8acc-97baeea17ede/.user_uploaded/uploaded_media_1789028772708.png";
        if !std::path::Path::new(sample_path).exists() {
            return;
        }
        if !crate::paths::is_ocr_ready() {
            return;
        }

        let bytes = std::fs::read(sample_path).unwrap();
        let res = OcrEngine::recognize_bytes(&bytes).expect("anyocr OCR 推理失败");
        println!("\n=== SensiDoc 经由 anyocr 代理端到端输出 ===");
        println!("{}", res.markdown);
        println!("=== 耗时: {}ms ===", res.elapsed_ms);
        assert!(!res.markdown.is_empty());
    }

    #[test]
    fn test_sensidoc_ocr_lifecycle_unload() {
        let _lock = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        if !crate::paths::is_ocr_ready() {
            return;
        }

        let engine = OcrEngine::get_or_init().expect("初始化 OCR 失败");
        assert!(OcrEngine::is_loaded());

        // 持有强引用时不会被卸载
        assert!(!OcrEngine::unload());
        assert!(OcrEngine::is_loaded());

        // drop 强引用后等待空闲卸载
        drop(engine);
        let mut unloaded = false;
        for _ in 0..60 {
            if OcrEngine::unload() {
                unloaded = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(unloaded, "OCR 引擎应在空闲时成功卸载");
        assert!(!OcrEngine::is_loaded());
    }

    #[test]
    fn test_ocr_state_poison_recovery() {
        let _lock = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());

        // 模拟外部线程在持有锁时发生 panic 导致锁中毒
        let handle = std::thread::spawn(|| {
            let _guard = OCR_STATE.lock().unwrap();
            panic!("模拟突发异常导致锁中毒");
        });
        let _ = handle.join(); // 捕获 panic，此时 OCR_STATE 处于 poisoned 状态

        // 验证 acquire_ocr_state 能够捕获 PoisonError 并优雅自愈重置上下文，不会死锁或崩溃
        assert!(OCR_STATE.is_poisoned(), "OCR_STATE 应该已被标记为中毒");
        let guard = acquire_ocr_state();
        assert!(guard.is_none(), "自愈后状态容器应被重置为 None");
        drop(guard); // 释放锁，防范不可重入 Mutex 自死锁
        assert!(!OcrEngine::is_loaded(), "引擎处于未加载安全状态");
    }

    #[test]
    fn test_expand_crop_box() {
        // 模拟一个单字符/短数字框 [x0, y0, x1, y1] = [100.0, 200.0, 115.0, 220.0] (宽 15, 高 20)
        let coords = [100.0, 200.0, 115.0, 220.0];
        let [x, y, w, h] = expand_crop_box(coords, 1000, 1000);
        // 字高 20，左右向外扩展 pad_x = (20 * 0.4).max(6.0) = 8px，x 从 100 - 8 = 92 开始
        assert_eq!(x, 92);
        assert!(w >= 15, "扩展后宽度必须包含周围上下文");
        // 上下轻度扩展 15% (3px)，y 应从 200 - 3 = 197 开始
        assert_eq!(y, 197);
        assert_eq!(h, 26);
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(base64_encode(b"sensidoc"), "c2Vuc2lkb2M=");
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn test_crop_image_box() {
        // 创建一个简单的 100x100 内存测试图像
        let img = image::RgbImage::new(100, 100);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).expect("编码测试图像失败");
        let png_bytes = buf.into_inner();

        let coords = [20.0, 30.0, 10.0, 15.0];
        let (crop_bytes, b64_url) = crop_image_box(&png_bytes, coords).expect("切片裁剪失败");
        assert!(!crop_bytes.is_empty());
        assert!(b64_url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn test_sanitize_redundant_empty_lines() {
        // 1. 测试 HTML 表格中的空行清除 (保留有效行，清除纯空行)
        let html_sample = "<table><tr><td>第1行有效</td></tr><tr><td></td></tr><tr><td>   </td></tr><tr><td colspan=\"2\"></td></tr><tr><td>第2行有效</td></tr></table>";
        let cleaned = sanitize_redundant_empty_lines(html_sample);
        assert_eq!(cleaned, "<table><tr><td>第1行有效</td></tr><tr><td>第2行有效</td></tr></table>");

        // 2. 测试全空表格被彻底清除
        let empty_table = "<table><tr><td></td></tr><tr><td>   </td></tr></table>";
        assert_eq!(sanitize_redundant_empty_lines(empty_table), "");

        // 3. 测试 Markdown 管道符表格中的空数据行清除 (保留表头与分割线)
        let md_table = "| 列1 | 列2 |\n| --- | --- |\n|  |  |\n| 内容1 | 内容2 |\n|   |   |";
        let cleaned_md = sanitize_redundant_empty_lines(md_table);
        assert_eq!(cleaned_md, "| 列1 | 列2 |\n| --- | --- |\n| 内容1 | 内容2 |");

        // 4. 测试正文连续超过 2 个的换行收敛为标准的 2 个换行 (\n\n)
        let text_with_blanks = "第一段\n\n\n\n\n第二段\n   \n   \n第三段";
        let cleaned_text = sanitize_redundant_empty_lines(text_with_blanks);
        assert_eq!(cleaned_text, "第一段\n\n第二段\n\n第三段");

        // 5. 测试用户实际截图场景 (信息优先级说明后的大量空 tr 标签清除)
        let user_scenario = "信息优先级标注说明：</td></tr><tr><td></td></tr><tr><td></td></tr><tr><td>★★★高：涉及商业底层逻辑。</td></tr><tr><td></td></tr><tr><td></td></tr><tr><td colspan=\"2\"></td></tr></table>\n\n★★中：传统商业常见的免费手段。";
        let cleaned_user = sanitize_redundant_empty_lines(user_scenario);
        assert!(!cleaned_user.contains("<tr><td></td></tr>"));
        assert!(!cleaned_user.contains("<td colspan=\"2\"></td>"));
        assert!(cleaned_user.contains("信息优先级标注说明：</td></tr><tr><td>★★★高：涉及商业底层逻辑。</td></tr></table>\n\n★★中：传统商业常见的免费手段。"));
    }
}
