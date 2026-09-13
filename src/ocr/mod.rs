use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// OCR 引擎空闲自动释放超时时间：3 分钟无新请求自动释放内存
pub const OCR_IDLE_TIMEOUT: Duration = Duration::from_secs(180);

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
            max_batch_size: 16,
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

        Ok(OcrResult {
            markdown: doc.markdown,
            raw_boxes,
            image_width: page.dimensions.0,
            image_height: page.dimensions.1,
            elapsed_ms: doc.elapsed_ms,
        })
    }
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
}
