#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod benchmark;
mod cli;
mod converter;
mod desensitizer;
mod exporter;
mod extractor;
mod model_manager;
pub mod ocr;
mod paths;
mod session;

use clap::Parser;
use cli::{Cli, Commands};
use axum::{
    extract::{Multipart, Path as AxumPath, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        Json,
    },
    routing::{delete, get, post},
    Router,
};
use benchmark::BenchmarkEngine;
use desensitizer::Desensitizer;
use extractor::{Extractor, RuleField, RulePreset, StreamingEntityExtractor};
use futures_util::{stream::Stream, StreamExt};
use model_manager::ModelManager;
use serde::{Deserialize, Serialize};
use session::{DocumentItem, ExtractionSnapshot, ModelPromptProfile, OfflineModelProfile, OnlineModelProfile, SessionManager};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    model_mgr: Arc<ModelManager>,
    session_mgr: Arc<SessionManager>,
}

#[derive(Serialize)]
struct ConvertResponse {
    doc_id: String,
    filename: String,
    markdown: String,
    char_count: usize,
    doc_type: String,
    low_confidence_count: usize,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct StartModelRequest {
    filename: String,
}

#[derive(Deserialize)]
struct DownloadModelRequest {
    model_id: String,
}

#[derive(Deserialize)]
struct ImportModelRequest {
    file_path: String,
}

#[derive(Deserialize)]
struct ExtractRequest {
    doc_id: String,
    template_name: String,
    fields: Vec<RuleField>,
    use_ai: bool,
    #[serde(default)]
    model_type: Option<String>,
    #[serde(default)]
    offline_model_name: Option<String>,
    #[serde(default)]
    online_model_id: Option<String>,
    #[serde(default)]
    custom_prompt: Option<String>,
}

#[derive(Deserialize)]
struct PromptPreviewRequest {
    fields: Vec<RuleField>,
    #[serde(default)]
    custom_template: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let model_mgr = Arc::new(ModelManager::new());
    let session_mgr = Arc::new(SessionManager::new());

    // 优雅停机信号捕获：当用户 Ctrl+C 或发生 SIGTERM 时确保强杀 llama-server
    let mgr_for_shutdown = model_mgr.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        mgr_for_shutdown.stop_server().await;
        std::process::exit(0);
    });

    // 检查是否指定了子命令
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Serve(serve_args) => {
                return run_server_mode(serve_args.port, &serve_args.host, serve_args.headless, model_mgr, session_mgr).await;
            }
            subcommand => {
                let exit_code = match cli::run_cli(subcommand, model_mgr.clone(), session_mgr.clone()).await {
                    Ok(code) => code,
                    Err(err) => {
                        eprintln!("❌ 错误: {err}");
                        2
                    }
                };
                model_mgr.stop_server().await;
                std::process::exit(exit_code);
            }
        }
    }

    // 默认或历史兼容模式 (--server / --headless)
    let is_headless = cli.server || cli.headless;
    run_server_mode(3000, "127.0.0.1", is_headless, model_mgr, session_mgr).await
}

async fn run_server_mode(
    port: u16,
    host: &str,
    is_headless: bool,
    model_mgr: Arc<ModelManager>,
    session_mgr: Arc<SessionManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sensidoc=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();

    tracing::info!("Initializing SensiDoc HTTP Service on {}:{}...", host, port);

    let state = AppState {
        model_mgr: model_mgr.clone(),
        session_mgr: session_mgr.clone(),
    };

    let web_dir = paths::get_web_dir();
    tracing::info!("Using static web directory: {:?}", web_dir);

    async fn set_no_cache_headers(
        req: axum::extract::Request,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        let mut response = next.run(req).await;
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        );
        response.headers_mut().insert(
            axum::http::header::PRAGMA,
            axum::http::HeaderValue::from_static("no-cache"),
        );
        response
    }

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/convert", post(convert_document))
        .route("/api/documents", get(list_documents))
        .route("/api/documents/pick", post(pick_documents_handler).get(pick_documents_handler))
        .route("/api/documents/import-paths", post(import_paths_handler))
        .route("/api/documents/{id}", get(get_document).delete(delete_document))
        .route("/api/documents/{id}/file", get(get_document_file))
        .route("/api/documents/{id}/file-status", get(get_document_file_status))
        .route("/api/documents/{id}/pick-and-relink", post(pick_and_relink_document).get(pick_and_relink_document))
        .route("/api/documents/{id}/relink", post(relink_document_handler))
        .route("/api/documents/{id}/open", post(open_document_handler))
        .route("/api/documents/{id}/reveal", post(reveal_document_handler))
        .route("/api/documents/{id}/desensitize", post(desensitize_document).get(desensitize_document))
        .route("/api/documents/{id}/snapshot/{snapshot_id}", post(set_active_snapshot).delete(delete_snapshot))
        .route("/api/documents/{id}/vlm-fast", post(fast_vlm_review_handler))
        .route("/api/documents/{id}/vlm-full/stream", get(full_vlm_stream_handler))
        .route("/api/documents/{id}/ocr-base", post(rerun_base_ocr_handler))
        .route("/api/documents/{id}/ocr-tier", post(set_ocr_tier_handler))
        .route("/api/rules/presets", get(get_rule_presets))
        .route("/api/rules/templates", post(save_custom_template))
        .route("/api/rules/templates/{id}", delete(delete_custom_template))
        .route("/api/rules/tags", get(get_field_tags).post(save_field_tag))
        .route("/api/rules/tags/{name}", delete(delete_field_tag))
        .route("/api/rules/prompt/preview", post(preview_system_prompt))
        .route("/api/rules/ai-generate", post(ai_generate_rules))
        .route("/api/extract", post(extract_sensitive_info))
        .route("/api/extract/stream", post(extract_sensitive_info_stream))
        .route("/api/models/presets", get(get_model_presets))
        .route("/api/models/local", get(list_local_models))
        .route("/api/models/mmprojs", get(list_local_mmprojs))
        .route("/api/models/active", get(get_active_model_status))
        .route("/api/models/prompts/all", get(get_all_model_prompts))
        .route("/api/models/{filename}/prompt", get(get_model_prompt).post(save_model_prompt))
        .route("/api/models/{filename}/benchmark", post(run_model_auto_benchmark))
        .route("/api/models/offline-profiles", get(get_all_offline_profiles).post(save_offline_profile))
        .route("/api/settings/online-models", get(get_online_models).post(save_online_model))
        .route("/api/settings/online-models/active", get(get_active_online_model).post(set_active_online_model))
        .route("/api/settings/online-models/{id}", delete(delete_online_model))
        .route("/api/settings/online-models/test", post(test_online_model))
        .route("/api/models/import", post(import_external_model))
        .route("/api/models/pick-and-import", post(pick_and_import_model).get(pick_and_import_model))
        .route("/api/models/start", post(start_model))
        .route("/api/models/stop", post(stop_model))
        .route("/api/models/download", post(download_model))
        .route("/api/models/download/cancel", post(cancel_download_model))
        .route("/api/models/download/progress", get(download_progress_sse))
        .route("/api/models/delete", post(delete_model_handler))
        .route("/api/ocr/status", get(get_ocr_status_handler))
        .route("/api/ocr/start", post(start_ocr_handler))
        .route("/api/ocr/unload", post(unload_ocr_handler))
        .route("/api/ocr/stop", post(unload_ocr_handler))
        .route("/api/ocr/download", post(download_ocr_handler))
        .route("/api/ocr/cancel", post(cancel_ocr_handler))
        .route("/api/ocr/delete", post(delete_ocr_handler).delete(delete_ocr_handler))
        .fallback_service(ServeDir::new(web_dir))
        .layer(axum::middleware::from_fn(set_no_cache_headers))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let ip: std::net::IpAddr = host.parse().unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
    let addr = SocketAddr::from((ip, port));

    let app_url = format!("http://{}:{}", host, port);

    // 启动 Axum 后端服务
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => {
            tracing::info!("SensiDoc HTTP Service listening on {}", app_url);
            l
        }
        Err(e) => {
            tracing::warn!("绑定端口 {} 失败 (可能已有服务在运行): {e}", port);
            if is_headless {
                return Ok(());
            }
            // 若端口已占且非 headless，直接用 WebView 打开已有服务
            return launch_desktop_gui(model_mgr.clone(), &app_url);
        }
    };

    // 在 Tokio 异步任务中托管 Axum 服务
    tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            tracing::error!("Axum 服务异常: {err}");
        }
    });

    // 等待服务端口就绪
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    if is_headless {
        tracing::info!("SensiDoc 以 Headless 无头服务器模式运行中 (访问 {})...", app_url);
        tokio::signal::ctrl_c().await.ok();
        model_mgr.stop_server().await;
        return Ok(());
    }

    // 默认以独立原生桌面 GUI 窗口模式启动
    launch_desktop_gui(model_mgr, &app_url)
}

/// 启动独立原生桌面客户端窗口 (Tao + Wry)
fn launch_desktop_gui(
    model_mgr: Arc<ModelManager>,
    app_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    use tao::dpi::LogicalPosition;
    use tao::{
        dpi::LogicalSize,
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        window::WindowBuilder,
    };
    #[cfg(target_os = "macos")]
    use tao::platform::macos::WindowBuilderExtMacOS;
    use wry::WebViewBuilder;

    #[derive(Debug)]
    enum UserEvent {
        Minimize,
        Maximize,
        RequestClose,
        ForceClose,
        DragWindow,
        DropFiles(Vec<std::path::PathBuf>),
    }

    tracing::info!("正在拉起 SensiDoc 独立原生桌面窗口 (支持现代沉浸式无菜单标题栏)...");

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    let builder = WindowBuilder::new()
        .with_title("SensiDoc - 敏感信息提取与脱敏工具")
        .with_inner_size(LogicalSize::new(1280.0, 840.0))
        .with_min_inner_size(LogicalSize::new(960.0, 600.0));

    // macOS: 隐藏原生文字标题，开启全尺寸内容视图与透明标题栏，原生的红黄绿三颗交通灯垂直居中浮动于左上角
    #[cfg(target_os = "macos")]
    let builder = builder
        .with_title_hidden(true)
        .with_titlebar_transparent(true)
        .with_fullsize_content_view(true)
        .with_traffic_light_inset(LogicalPosition::new(18.0, 20.0));

    // Windows: 无原生系统标题栏与边框菜单，由 Web UI 顶层托管窗口拖拽与右上角控制
    #[cfg(target_os = "windows")]
    let builder = builder.with_decorations(false);

    let window = builder.build(&event_loop)?;

    let proxy = event_loop.create_proxy();
    let ipc_proxy = proxy.clone();

    let platform_name = if cfg!(target_os = "macos") {
        "mac"
    } else if cfg!(target_os = "windows") {
        "win"
    } else {
        "linux"
    };

    let init_script = format!(
        r#"
        window.__SENSIDOC_DESKTOP__ = true;
        window.__SENSIDOC_PLATFORM__ = "{platform_name}";
        (function() {{
            function applyPlatformClasses() {{
                if (document.documentElement) {{
                    document.documentElement.classList.add("desktop-app", "platform-{platform_name}");
                }}
                if (document.body) {{
                    document.body.classList.add("desktop-app", "platform-{platform_name}");
                }}
            }}
            applyPlatformClasses();
            if (document.readyState === "loading") {{
                document.addEventListener("DOMContentLoaded", applyPlatformClasses);
            }}
        }})();
        "#
    );

    #[cfg(target_os = "macos")]
    use wry::WebViewBuilderExtDarwin;

    let webview_builder = WebViewBuilder::new()
        .with_url(app_url)
        .with_initialization_script(&init_script);

    #[cfg(target_os = "macos")]
    let webview_builder = webview_builder.with_traffic_light_inset(LogicalPosition::new(18.0, 20.0));

    let drop_proxy = proxy.clone();
    let webview_builder = webview_builder.with_drag_drop_handler(move |event| {
        match event {
            wry::DragDropEvent::Drop { paths, .. } => {
                let _ = drop_proxy.send_event(UserEvent::DropFiles(paths));
                false
            }
            _ => false,
        }
    });

    let webview = webview_builder
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let msg = req.body().trim();
            match msg {
                "minimize" => {
                    let _ = ipc_proxy.send_event(UserEvent::Minimize);
                }
                "maximize" => {
                    let _ = ipc_proxy.send_event(UserEvent::Maximize);
                }
                "close" => {
                    let _ = ipc_proxy.send_event(UserEvent::RequestClose);
                }
                "force_close" => {
                    let _ = ipc_proxy.send_event(UserEvent::ForceClose);
                }
                "drag_window" => {
                    let _ = ipc_proxy.send_event(UserEvent::DragWindow);
                }
                _ => {}
            }
        })
        .build(&window)?;

    let shutdown_mgr = model_mgr.clone();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                tracing::info!("SensiDoc 原生窗口初始化成功");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            }
            | Event::UserEvent(UserEvent::RequestClose) => {
                tracing::info!("收到窗口关闭请求，向前端发起二次确认提示...");
                let _ = webview.evaluate_script(
                    "if (typeof window.__handleAppExitRequest === 'function') { \
                        window.__handleAppExitRequest(); \
                     } else { \
                        if (confirm('确定要退出 SensiDoc 吗？未导出的文档与脱敏结果可能会丢失。')) { \
                            if (window.ipc) window.ipc.postMessage('force_close'); \
                        } \
                     }",
                );
            }
            Event::UserEvent(UserEvent::ForceClose) => {
                tracing::info!("用户已确认退出，正在安全释放模型进程并退出客户端...");
                let mgr = shutdown_mgr.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(async {
                        mgr.stop_server().await;
                    });
                    std::process::exit(0);
                });
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::Minimize) => {
                window.set_minimized(true);
            }
            Event::UserEvent(UserEvent::Maximize) => {
                window.set_maximized(!window.is_maximized());
            }
            Event::UserEvent(UserEvent::DragWindow) => {
                let _ = window.drag_window();
            }
            Event::UserEvent(UserEvent::DropFiles(paths)) => {
                let path_strings: Vec<String> = paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();
                if !path_strings.is_empty() {
                    let json = serde_json::to_string(&path_strings).unwrap_or_else(|_| "[]".to_string());
                    let script = format!(
                        "if (typeof window.__handleNativeFilesDrop === 'function') {{ window.__handleNativeFilesDrop({}); }}",
                        json
                    );
                    let _ = webview.evaluate_script(&script);
                }
            }
            _ => (),
        }
    });
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "SensiDoc",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[derive(Deserialize)]
struct ImportPathsRequest {
    paths: Vec<String>,
}

#[derive(Serialize)]
struct ImportedDocItem {
    doc_id: String,
    filename: String,
    char_count: usize,
    doc_type: String,
    low_confidence_count: usize,
    source_path: Option<String>,
}

#[derive(Serialize)]
struct ImportPathsResponse {
    imported: Vec<ImportedDocItem>,
    failed: Vec<String>,
}

async fn convert_document(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ConvertResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut filename = String::from("document.txt");
    let mut file_bytes = Vec::new();
    let mut source_path: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("读取上传字段失败: {e}"),
            }),
        )
    })? {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            if let Some(orig_name) = field.file_name() {
                filename = orig_name.to_string();
            }
            file_bytes = field.bytes().await.map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("读取文件字节流失败: {e}"),
                    }),
                )
            })?.to_vec();
        } else if name == "source_path" {
            if let Ok(val) = field.text().await {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    source_path = Some(trimmed.to_string());
                }
            }
        }
    }

    if file_bytes.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "未收到文件或文件内容为空".into(),
            }),
        ));
    }

    let filename_clone = filename.clone();
    let file_bytes_clone = file_bytes.clone();
    let convert_res = tokio::task::spawn_blocking(move || {
        converter::DocConverter::convert_bytes_detailed(&filename_clone, &file_bytes_clone)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("文档转换任务异常: {e}"),
            }),
        )
    })?;

    match convert_res {
        Ok(res) => {
            let doc = state
                .session_mgr
                .upsert_document_typed(
                    filename,
                    res.markdown,
                    res.doc_type,
                    res.low_confidence_count,
                    res.raw_boxes,
                    res.image_width,
                    res.image_height,
                    source_path,
                )
                .await;
            // 暂存原始上传二进制文件，支持脱敏引擎原样无损导出
            let upload_path = paths::get_uploads_dir().join(format!("{}.bin", doc.id));
            if let Err(e) = std::fs::write(&upload_path, &file_bytes) {
                tracing::warn!("暂存原始文档字节失败 ({}): {}", upload_path.display(), e);
            }
            Ok(Json(ConvertResponse {
                doc_id: doc.id,
                filename: doc.filename,
                markdown: doc.markdown,
                char_count: doc.char_count,
                doc_type: doc.doc_type,
                low_confidence_count: doc.low_confidence_count,
            }))
        }
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: err }),
        )),
    }
}

/// 唤起操作系统原生文件选择对话框多选文档文件
async fn pick_documents_handler() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match pick_files_dialog_native().await {
        Ok(paths) => Ok(Json(serde_json::json!({
            "status": "success",
            "canceled": paths.is_empty(),
            "paths": paths,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )),
    }
}

pub async fn pick_files_dialog_native() -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let script = r#"try
  set selectedFiles to choose file with prompt "请选择要导入提取的文档文件" of type {"pdf", "docx", "doc", "xlsx", "xls", "pptx", "txt", "md", "csv", "png", "jpg", "jpeg", "bmp", "webp", "public.data"} with multiple selections allowed
  set posixPaths to ""
  repeat with aFile in selectedFiles
    set posixPaths to posixPaths & (POSIX path of aFile) & linefeed
  end repeat
  return posixPaths
on error number -128
  return ""
end try"#;
        let output = tokio::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .await
            .map_err(|e| format!("无法唤起原生文件选择器: {e}"))?;

        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if out_str.is_empty() {
                return Ok(Vec::new());
            }
            let paths: Vec<String> = out_str
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return Ok(paths);
        } else {
            // 降级无类型过滤唤起
            let script_fallback = r#"try
  set selectedFiles to choose file with prompt "请选择要导入提取的文档文件" with multiple selections allowed
  set posixPaths to ""
  repeat with aFile in selectedFiles
    set posixPaths to posixPaths & (POSIX path of aFile) & linefeed
  end repeat
  return posixPaths
on error number -128
  return ""
end try"#;
            let fb_output = tokio::process::Command::new("osascript")
                .arg("-e")
                .arg(script_fallback)
                .output()
                .await
                .map_err(|e| format!("无法唤起原生文件选择器: {e}"))?;
            if fb_output.status.success() {
                let out_str = String::from_utf8_lossy(&fb_output.stdout).trim().to_string();
                let paths: Vec<String> = out_str
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                return Ok(paths);
            }
            return Ok(Vec::new());
        }
    }

    #[cfg(target_os = "windows")]
    {
        let script = r#"[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Multiselect = $true; $f.Filter = '所有支持文档 (*.pdf;*.docx;*.doc;*.xlsx;*.xls;*.pptx;*.txt;*.md;*.csv;*.png;*.jpg;*.jpeg;*.bmp;*.webp)|*.pdf;*.docx;*.doc;*.xlsx;*.xls;*.pptx;*.txt;*.md;*.csv;*.png;*.jpg;*.jpeg;*.bmp;*.webp|所有文件 (*.*)|*.*'; $f.Title = '请选择要导入提取的文档文件'; if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { $f.FileNames | ForEach-Object { Write-Output $_ } }"#;
        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .await
            .map_err(|e| format!("无法唤起 Windows 文件选择器: {e}"))?;

        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if out_str.is_empty() {
                return Ok(Vec::new());
            }
            let paths: Vec<String> = out_str
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(paths)
        } else {
            Ok(Vec::new())
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Ok(output) = tokio::process::Command::new("zenity")
            .args(["--file-selection", "--multiple", "--separator=\n", "--title=请选择要导入提取的文档文件"])
            .output()
            .await
        {
            if output.status.success() {
                let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let paths: Vec<String> = out_str
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                return Ok(paths);
            }
        }
        Ok(Vec::new())
    }
}

/// 唤起操作系统原生单文件选择器，用于重新关联单个文档源文件
pub async fn pick_single_document_dialog(prompt_text: &str) -> Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let escaped_prompt = prompt_text.replace('"', "\\\"");
        let script = format!(
            r#"try
  set selectedFile to choose file with prompt "{escaped_prompt}" of type {{"pdf", "docx", "doc", "xlsx", "xls", "pptx", "txt", "md", "csv", "png", "jpg", "jpeg", "bmp", "webp", "public.data"}}
  return POSIX path of selectedFile
on error number -128
  return ""
end try"#
        );
        let output = tokio::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .await
            .map_err(|e| format!("无法唤起原生文件选择器: {e}"))?;

        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if out_str.is_empty() {
                Ok(None)
            } else {
                Ok(Some(out_str))
            }
        } else {
            // 降级无类型过滤唤起
            let script_fallback = format!(
                r#"try
  set selectedFile to choose file with prompt "{escaped_prompt}"
  return POSIX path of selectedFile
on error number -128
  return ""
end try"#
            );
            let fb_output = tokio::process::Command::new("osascript")
                .arg("-e")
                .arg(&script_fallback)
                .output()
                .await
                .map_err(|e| format!("无法唤起原生文件选择器: {e}"))?;
            if fb_output.status.success() {
                let out_str = String::from_utf8_lossy(&fb_output.stdout).trim().to_string();
                if out_str.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(out_str))
                }
            } else {
                Ok(None)
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let escaped_prompt = prompt_text.replace('\'', "''");
        let script = format!(
            r#"[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Multiselect = $false; $f.Filter = '所有支持文档 (*.pdf;*.docx;*.doc;*.xlsx;*.xls;*.pptx;*.txt;*.md;*.csv;*.png;*.jpg;*.jpeg;*.bmp;*.webp)|*.pdf;*.docx;*.doc;*.xlsx;*.xls;*.pptx;*.txt;*.md;*.csv;*.png;*.jpg;*.jpeg;*.bmp;*.webp|所有文件 (*.*)|*.*'; $f.Title = '{escaped_prompt}'; if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ Write-Output $f.FileName }}"#
        );
        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .await
            .map_err(|e| format!("无法唤起 Windows 文件选择器: {e}"))?;

        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if out_str.is_empty() {
                Ok(None)
            } else {
                Ok(Some(out_str))
            }
        } else {
            Ok(None)
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Ok(output) = tokio::process::Command::new("zenity")
            .args(["--file-selection", &format!("--title={prompt_text}")])
            .output()
            .await
        {
            if output.status.success() {
                let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if out_str.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(out_str));
            }
        }
        Ok(None)
    }
}

/// 批量按本地物理绝对路径导入文档并保留 source_path
async fn import_paths_handler(
    State(state): State<AppState>,
    Json(payload): Json<ImportPathsRequest>,
) -> Result<Json<ImportPathsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.paths.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "未传入任何文件路径".into(),
            }),
        ));
    }

    let mut imported = Vec::new();
    let mut failed = Vec::new();

    for path_str in payload.paths {
        let p = std::path::Path::new(&path_str);
        if !p.is_file() {
            failed.push(format!("{path_str}: 文件不存在或不是有效普通文件"));
            continue;
        }

        let filename = match p.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => {
                failed.push(format!("{path_str}: 无法解析文件名"));
                continue;
            }
        };

        let file_bytes = match tokio::fs::read(p).await {
            Ok(b) => b,
            Err(e) => {
                failed.push(format!("{path_str}: 读取文件失败: {e}"));
                continue;
            }
        };

        let filename_clone = filename.clone();
        let file_bytes_clone = file_bytes.clone();
        let convert_res = tokio::task::spawn_blocking(move || {
            converter::DocConverter::convert_bytes_detailed(&filename_clone, &file_bytes_clone)
        })
        .await;

        let res = match convert_res {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                failed.push(format!("{path_str}: 文档解析失败: {e}"));
                continue;
            }
            Err(e) => {
                failed.push(format!("{path_str}: 转换任务异常: {e}"));
                continue;
            }
        };

        let doc = state
            .session_mgr
            .upsert_document_typed(
                filename.clone(),
                res.markdown,
                res.doc_type.clone(),
                res.low_confidence_count,
                res.raw_boxes,
                res.image_width,
                res.image_height,
                Some(path_str.clone()),
            )
            .await;

        // 双重备份：写一份到 uploads/{id}.bin
        let upload_path = paths::get_uploads_dir().join(format!("{}.bin", doc.id));
        if let Err(e) = tokio::fs::write(&upload_path, &file_bytes).await {
            tracing::warn!("备份缓存文件失败 ({}): {}", upload_path.display(), e);
        }

        imported.push(ImportedDocItem {
            doc_id: doc.id,
            filename: doc.filename,
            char_count: doc.char_count,
            doc_type: doc.doc_type,
            low_confidence_count: doc.low_confidence_count,
            source_path: Some(path_str),
        });
    }

    Ok(Json(ImportPathsResponse { imported, failed }))
}

/// 获取预览缓存路径（当源文件不可用但需要带原生扩展名给系统程序打开时）
async fn get_preview_cache_path(doc: &DocumentItem) -> Result<std::path::PathBuf, String> {
    let preview_dir = paths::get_uploads_dir().join("preview_cache");
    let _ = tokio::fs::create_dir_all(&preview_dir).await;
    let safe_name = if doc.filename.is_empty() {
        format!("{}.bin", doc.id)
    } else {
        doc.filename.clone()
    };
    let target = preview_dir.join(&safe_name);
    let bytes = doc.read_original_bytes().await?;
    tokio::fs::write(&target, &bytes)
        .await
        .map_err(|e| format!("写入缓存副本失败: {e}"))?;
    Ok(target)
}

/// 用系统默认关联程序打开文档（优先打开源文件，若源文件不可访问则打开缓存副本）
async fn open_document_handler(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let doc = state.session_mgr.get_document(&doc_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "文档未找到".into(),
            }),
        )
    })?;

    let (target_path, is_source) = if let Some(ref sp) = doc.source_path {
        let p = std::path::Path::new(sp);
        if p.is_file() {
            (std::path::PathBuf::from(sp), true)
        } else {
            let cache_path = get_preview_cache_path(&doc).await.map_err(|e| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: format!("源文件已不存在 ({sp}) 且本地缓存已丢失: {e}"),
                    }),
                )
            })?;
            (cache_path, false)
        }
    } else {
        let cache_path = get_preview_cache_path(&doc).await.map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("文档原始文件未找到: {e}"),
                }),
            )
        })?;
        (cache_path, false)
    };

    open_file_with_default_app(&target_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("调用系统程序打开文件失败: {e}"),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "opened_path": target_path.to_string_lossy(),
        "is_source": is_source,
    })))
}

/// 在系统文件管理器（macOS访达 / Windows资源管理器）中高亮定位文件
async fn reveal_document_handler(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let doc = state.session_mgr.get_document(&doc_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "文档未找到".into(),
            }),
        )
    })?;

    let (target_path, is_source) = if let Some(ref sp) = doc.source_path {
        let p = std::path::Path::new(sp);
        if p.is_file() {
            (std::path::PathBuf::from(sp), true)
        } else {
            let cache_path = get_preview_cache_path(&doc).await.map_err(|e| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: format!("源文件已不存在 ({sp}) 且本地缓存已丢失: {e}"),
                    }),
                )
            })?;
            (cache_path, false)
        }
    } else {
        let cache_path = get_preview_cache_path(&doc).await.map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("文档原始文件未找到: {e}"),
                }),
            )
        })?;
        (cache_path, false)
    };

    reveal_file_in_manager(&target_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("在文件管理器中定位失败: {e}"),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "revealed_path": target_path.to_string_lossy(),
        "is_source": is_source,
    })))
}

fn open_file_with_default_app(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .arg(path)
            .status()
            .map_err(|e| format!("执行 open 命令失败: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("open 命令返回非 0 退出码".to_string())
        }
    }

    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string_lossy().to_string();
        let script = format!("Start-Process -FilePath '{}'", path_str.replace('\'', "''"));
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()
            .map_err(|e| format!("执行 PowerShell 启动失败: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("PowerShell Start-Process 失败".to_string())
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let status = std::process::Command::new("xdg-open")
            .arg(path)
            .status()
            .map_err(|e| format!("执行 xdg-open 失败: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("xdg-open 失败".to_string())
        }
    }
}

fn reveal_file_in_manager(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .map_err(|e| format!("执行 open -R 失败: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("open -R 失败".to_string())
        }
    }

    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string_lossy().to_string();
        let arg = format!("/select,{}", path_str);
        let status = std::process::Command::new("explorer")
            .arg(&arg)
            .status()
            .map_err(|e| format!("执行 explorer /select 失败: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("explorer 失败".to_string())
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let parent = path.parent().unwrap_or(path);
        let status = std::process::Command::new("xdg-open")
            .arg(parent)
            .status()
            .map_err(|e| format!("执行 xdg-open 失败: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("xdg-open 失败".to_string())
        }
    }
}

async fn list_documents(
    State(state): State<AppState>,
) -> Json<Vec<DocumentItem>> {
    Json(state.session_mgr.list_documents().await)
}

async fn get_document(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<DocumentItem>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_mgr.get_document(&doc_id).await {
        Some(doc) => Ok(Json(doc)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "文档未找到".into(),
            }),
        )),
    }
}

async fn delete_document(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if state.session_mgr.delete_document(&doc_id).await {
        let upload_path = paths::get_uploads_dir().join(format!("{doc_id}.bin"));
        let _ = std::fs::remove_file(upload_path);
        Ok(Json(serde_json::json!({ "status": "success" })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "文档未找到".into(),
            }),
        ))
    }
}

async fn get_document_file(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let doc = match state.session_mgr.get_document(&doc_id).await {
        Some(d) => d,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "文档未找到".into(),
                }),
            ));
        }
    };

    let bytes = doc.read_original_bytes().await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("读取原始文件失败: {e}"),
            }),
        )
    })?;

    let ext = std::path::Path::new(&doc.filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "tiff" | "tif" => "image/tiff",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    };

    let headers = [
        (axum::http::header::CONTENT_TYPE, mime),
        (
            axum::http::header::CONTENT_DISPOSITION,
            "inline",
        ),
    ];

    Ok((headers, bytes))
}

/// 查询单据底图与物理源文件完整状态
async fn get_document_file_status(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let doc = match state.session_mgr.get_document(&doc_id).await {
        Some(d) => d,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "文档未找到".into(),
                }),
            ));
        }
    };

    let has_original = doc.has_original_file();
    let source_path_exists = doc
        .source_path
        .as_ref()
        .map(|sp| std::path::Path::new(sp).is_file())
        .unwrap_or(false);
    let upload_path = paths::get_uploads_dir().join(format!("{doc_id}.bin"));
    let cache_exists = upload_path.is_file();

    Ok(Json(serde_json::json!({
        "doc_id": doc.id,
        "filename": doc.filename,
        "has_original_file": has_original,
        "source_path": doc.source_path,
        "source_path_exists": source_path_exists,
        "cache_exists": cache_exists
    })))
}

/// 唤起操作系统原生单选文件对话框，为指定文档重新关联本地物理文件并刷新底图
async fn pick_and_relink_document(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let doc = match state.session_mgr.get_document(&doc_id).await {
        Some(d) => d,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "文档未找到".into(),
                }),
            ));
        }
    };

    let prompt = format!("请为「{}」重新选择本地源文件", doc.filename);
    match pick_single_document_dialog(&prompt).await {
        Ok(Some(path_str)) => {
            let p = std::path::Path::new(&path_str);
            if !p.is_file() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("所选文件不存在或无法访问: {path_str}"),
                    }),
                ));
            }

            let file_bytes = match tokio::fs::read(p).await {
                Ok(b) => b,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: format!("读取所选源文件失败: {e}"),
                        }),
                    ));
                }
            };

            match state
                .session_mgr
                .relink_document_source(&doc_id, Some(path_str.clone()), file_bytes)
                .await
            {
                Ok(updated_doc) => Ok(Json(serde_json::json!({
                    "status": "success",
                    "canceled": false,
                    "source_path": path_str,
                    "doc": updated_doc
                }))),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { error: e }),
                )),
            }
        }
        Ok(None) => Ok(Json(serde_json::json!({
            "status": "canceled",
            "canceled": true
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )),
    }
}

/// 接收指定源路径或上传的文件字节，为文档重新关联/补齐底图
async fn relink_document_handler(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let _doc = match state.session_mgr.get_document(&doc_id).await {
        Some(d) => d,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "文档未找到".into(),
                }),
            ));
        }
    };

    let mut source_path: Option<String> = None;
    let mut file_bytes = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            if let Ok(b) = field.bytes().await {
                file_bytes = b.to_vec();
            }
        } else if name == "source_path" {
            if let Ok(t) = field.text().await {
                let s = t.trim().to_string();
                if !s.is_empty() {
                    source_path = Some(s);
                }
            }
        }
    }

    if file_bytes.is_empty() {
        if let Some(ref sp) = source_path {
            let p = std::path::Path::new(sp);
            if p.is_file() {
                if let Ok(b) = tokio::fs::read(p).await {
                    file_bytes = b;
                }
            }
        }
    }

    if file_bytes.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "未提供有效的文件内容或可读取的源路径".into(),
            }),
        ));
    }

    match state
        .session_mgr
        .relink_document_source(&doc_id, source_path.clone(), file_bytes)
        .await
    {
        Ok(updated_doc) => Ok(Json(serde_json::json!({
            "status": "success",
            "source_path": source_path,
            "doc": updated_doc
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )),
    }
}

#[derive(Deserialize, Default)]
struct DesensitizeRequest {
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    snapshot_id: Option<String>,
    #[serde(default)]
    style: Option<String>,
}

fn percent_encode_filename(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

async fn desensitize_document(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
    axum::extract::Query(query_params): axum::extract::Query<HashMap<String, String>>,
    payload: Option<Json<DesensitizeRequest>>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let req = payload.map(|p| p.0).unwrap_or_default();
    let mode = req
        .mode
        .or_else(|| query_params.get("mode").cloned())
        .unwrap_or_else(|| "native".to_string());
    let snapshot_id = req.snapshot_id.or_else(|| query_params.get("snapshot_id").cloned());
    let style_str = req
        .style
        .or_else(|| query_params.get("style").cloned())
        .unwrap_or_else(|| "masking".to_string());
    let mask_style = std::str::FromStr::from_str(&style_str).unwrap_or(desensitizer::MaskStyle::Masking);

    let doc = match state.session_mgr.get_document(&doc_id).await {
        Some(d) => d,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "文档未找到".into(),
                }),
            ))
        }
    };

    // 确定使用的快照
    let snapshot = if let Some(snap_id) = &snapshot_id {
        doc.snapshots.iter().find(|s| &s.id == snap_id)
    } else if let Some(active_id) = &doc.active_snapshot_id {
        doc.snapshots.iter().find(|s| &s.id == active_id)
    } else {
        doc.snapshots.last()
    };

    let empty_items = Vec::new();
    let detected_items = snapshot.map(|s| &s.items).unwrap_or(&empty_items);

    let (out_filename, bytes, mime_type) = if mode == "markdown" {
        let desensitized_md = Desensitizer::desensitize_plain_text_with_style(&doc.markdown, detected_items, mask_style);
        let stem = std::path::Path::new(&doc.filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");
        (
            format!("{stem}_脱敏.md"),
            desensitized_md.into_bytes(),
            "text/markdown; charset=utf-8",
        )
    } else {
        // native 模式：优先通过 doc.read_original_bytes_sync() 读取原始二进制（优先源路径，回退缓存）
        let orig_bytes = doc.read_original_bytes_sync().ok();

        match Desensitizer::desensitize_document_auto_detailed(
            &doc.filename,
            orig_bytes.as_deref(),
            &doc.markdown,
            detected_items,
            mask_style,
            Some(&doc.raw_boxes),
        ) {
            Ok(res) => res,
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("原生脱敏导出失败: {e}"),
                    }),
                ));
            }
        }
    };

    let encoded_filename = percent_encode_filename(&out_filename);
    let content_disposition = format!(
        "attachment; filename=\"{encoded_filename}\"; filename*=UTF-8''{encoded_filename}"
    );

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_str(mime_type)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&content_disposition)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("attachment")),
    );

    Ok((headers, bytes))
}

async fn set_active_snapshot(
    State(state): State<AppState>,
    AxumPath((doc_id, snapshot_id)): AxumPath<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_mgr.set_active_snapshot(&doc_id, &snapshot_id).await {
        Ok(_) => Ok(Json(serde_json::json!({ "status": "success" }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        )),
    }
}

async fn delete_snapshot(
    State(state): State<AppState>,
    AxumPath((doc_id, snapshot_id)): AxumPath<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_mgr.delete_snapshot(&doc_id, &snapshot_id).await {
        Ok(active_id) => Ok(Json(serde_json::json!({
            "status": "success",
            "active_snapshot_id": active_id
        }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        )),
    }
}

async fn get_rule_presets(
    State(state): State<AppState>,
) -> Json<Vec<RulePreset>> {
    let mut presets = Extractor::get_rule_presets();
    let custom = state.session_mgr.get_custom_templates().await;
    presets.extend(custom);
    Json(presets)
}

async fn save_custom_template(
    State(state): State<AppState>,
    Json(payload): Json<RulePreset>,
) -> Json<serde_json::Value> {
    state.session_mgr.save_template(payload).await;
    Json(serde_json::json!({
        "status": "success",
        "message": "场景模板已成功保存落盘"
    }))
}

async fn delete_custom_template(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.session_mgr.delete_template(&id).await;
    if deleted {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "场景模板已成功删除"
        })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "未找到该场景模板或属于内置模板不可删除".into(),
            }),
        ))
    }
}

async fn get_field_tags(
    State(state): State<AppState>,
) -> Json<Vec<RuleField>> {
    Json(state.session_mgr.get_field_tags().await)
}

async fn save_field_tag(
    State(state): State<AppState>,
    Json(payload): Json<RuleField>,
) -> Json<serde_json::Value> {
    state.session_mgr.save_field_tag(payload).await;
    Json(serde_json::json!({
        "status": "success",
        "message": "字段标签已保存"
    }))
}

async fn delete_field_tag(
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.session_mgr.delete_field_tag(&name).await;
    if deleted {
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "字段标签已成功删除"
        })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "未找到该字段标签".into(),
            }),
        ))
    }
}

async fn preview_system_prompt(
    Json(payload): Json<PromptPreviewRequest>,
) -> Json<serde_json::Value> {
    let combined_prompt = Extractor::build_system_prompt(
        &payload.fields,
        payload.custom_template.as_deref(),
    );

    Json(serde_json::json!({
        "default_template": Extractor::DEFAULT_SYSTEM_PROMPT_TEMPLATE,
        "combined_prompt": combined_prompt,
        "output_format_schema": Extractor::OUTPUT_FORMAT_SCHEMA
    }))
}

async fn extract_sensitive_info(
    State(state): State<AppState>,
    Json(payload): Json<ExtractRequest>,
) -> Result<Json<ExtractionSnapshot>, (StatusCode, Json<ErrorResponse>)> {
    let start_time = Instant::now();

    let doc = state.session_mgr.get_document(&payload.doc_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "指定的文档不存在".into(),
            }),
        )
    })?;

    // 1. 正则按需兜底提取（仅当启用的规则涉及身份证/手机/邮箱/银行卡时才触发）
    let regex_items = Extractor::extract_by_regex(&doc.markdown, &payload.fields);

    // 2. 本地小模型 AI 提取 (若开启且模型就绪)
    let mut ai_items = Vec::new();
    let system_prompt_used = Extractor::build_system_prompt(
        &payload.fields,
        payload.custom_prompt.as_deref(),
    );

    if payload.use_ai {
        let is_online = payload.model_type.as_deref() == Some("online");
        let chunks = Extractor::chunk_text(&doc.markdown, 2500);

        if is_online {
            let online_models = state.session_mgr.get_online_models().await;
            let active_id = state.session_mgr.get_active_online_model_id().await;
            let target_cfg = if let Some(id) = &payload.online_model_id {
                online_models.iter().find(|m| &m.id == id).cloned()
            } else {
                online_models.iter().find(|m| Some(&m.id) == active_id.as_ref()).cloned().or_else(|| online_models.first().cloned())
            };

            if let Some(cfg) = target_cfg {
                for (_offset, chunk_text) in chunks {
                    if let Ok(items) = Extractor::query_online_llm(
                        &cfg.base_url,
                        &cfg.api_key,
                        &cfg.model_id,
                        cfg.temperature,
                        cfg.top_k,
                        cfg.repeat_penalty,
                        cfg.max_tokens,
                        cfg.enable_thinking,
                        &system_prompt_used,
                        &chunk_text,
                    ).await {
                        ai_items.extend(items);
                    }
                }
            }
        } else {
            // 本地离线模型处理
            let requested_model = payload.offline_model_name.as_deref().unwrap_or("").trim();
            let is_dual_engine = requested_model == "dual_engine";

            let target_model_file = if is_dual_engine {
                // 协同引擎模式：初筛基座使用 Qwen3.5-0.8B，若未下载则回退 MiniCPM 或首个可用模型
                let local_models = state.model_mgr.list_local_models();
                if local_models.iter().any(|m| m == "Qwen3.5-0.8B-Q4_K_M.gguf") {
                    "Qwen3.5-0.8B-Q4_K_M.gguf".to_string()
                } else if local_models.iter().any(|m| m == "MiniCPM5-2B-Q4_K_M.gguf") {
                    "MiniCPM5-2B-Q4_K_M.gguf".to_string()
                } else {
                    local_models.first().cloned().unwrap_or_default()
                }
            } else if !requested_model.is_empty() {
                requested_model.to_string()
            } else {
                state.model_mgr.get_active_model().await.unwrap_or_default()
            };

            if !target_model_file.is_empty() {
                // 确保目标模型正在运行
                let active = state.model_mgr.get_active_model().await;
                let is_ready = active.as_deref() == Some(&target_model_file) && state.model_mgr.is_server_ready().await;
                if !is_ready {
                    let profile = state.session_mgr.get_offline_model_profile(&target_model_file).await;
                    if let Err(e) = state.model_mgr.start_model_with_profile(&target_model_file, Some(&profile)).await {
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                error: format!("拉起本地离线模型 {target_model_file} 失败: {e}"),
                            }),
                        ));
                    }
                }
            }

            let port = state.model_mgr.server_port();
            let current_model_name = state.model_mgr.get_active_model().await.unwrap_or_default();
            let offline_profile = state.session_mgr.get_offline_model_profile(&current_model_name).await;
            for (_offset, chunk_text) in chunks {
                if let Ok(items) = Extractor::query_llm(
                    port,
                    offline_profile.temperature,
                    offline_profile.top_k,
                    offline_profile.repeat_penalty,
                    offline_profile.max_tokens,
                    offline_profile.enable_thinking,
                    &system_prompt_used,
                    &chunk_text,
                ).await {
                    ai_items.extend(items);
                }
            }

            // 若为协同引擎模式，挂载方案 A 后置形态白名单拦截器进行终审精筛
            if is_dual_engine {
                ai_items = extractor::PostFilterGuard::sanitize_items(ai_items, &payload.fields, &doc.markdown);
            }
        }
    }

    // 3. 冲突消解、位置回填与合并排序（结合传入的规则自动分类与赋权）
    let final_items = Extractor::merge_and_resolve(&doc.markdown, regex_items, ai_items, &payload.fields);
    let execution_ms = start_time.elapsed().as_millis() as u64;

    // 获取当前实际执行本次提取的模型名称
    let model_name = if payload.use_ai {
        if payload.model_type.as_deref() == Some("online") {
            if let Some(cfg_id) = &payload.online_model_id {
                if let Some(cfg) = state.session_mgr.get_online_model_by_id(cfg_id).await {
                    Some(format!("[在线] {}", cfg.name))
                } else {
                    Some("[在线] 在线大模型".to_string())
                }
            } else {
                Some("[在线] 在线大模型".to_string())
            }
        } else {
            let requested_model = payload.offline_model_name.as_deref().unwrap_or("").trim();
            if requested_model == "dual_engine" {
                Some("[协同引擎] Qwen3.5-0.8B + MiniCPM5-2B".to_string())
            } else {
                let active_name = state.model_mgr.get_active_model().await.unwrap_or_else(|| "本地离线模型".to_string());
                let display = if active_name.contains("Qwen") {
                    "Qwen3.5-0.8B"
                } else if active_name.contains("MiniCPM") {
                    "MiniCPM5-2B"
                } else {
                    &active_name
                };
                Some(format!("[离线] {}", display))
            }
        }
    } else {
        Some("正则规则引擎".to_string())
    };

    let non_stream_log = format!(
        "> 规则提取就绪，启用 {} 项特征规则\n> ═══ 提取全部完成 ═══\n> 耗时: {}ms, 检出: {} 项\n> 执行模型: {}\n",
        payload.fields.iter().filter(|f| f.is_enabled).count(),
        execution_ms,
        final_items.len(),
        model_name.as_deref().unwrap_or("未知模型")
    );

    // 4. 生成历史快照并落盘持久化
    match state
        .session_mgr
        .add_snapshot(
            &payload.doc_id,
            payload.template_name,
            model_name,
            Some(system_prompt_used),
            payload.fields,
            final_items,
            execution_ms,
            Some(non_stream_log),
        )
        .await
    {
        Ok(snapshot) => Ok(Json(snapshot)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )),
    }
}

async fn extract_sensitive_info_stream(
    State(state): State<AppState>,
    Json(payload): Json<ExtractRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let start_time = Instant::now();

        let doc = match state.session_mgr.get_document(&payload.doc_id).await {
            Some(d) => d,
            None => {
                yield Ok(Event::default().event("error").data(serde_json::json!({"error": "指定的文档不存在"}).to_string()));
                return;
            }
        };

        // 发送 init 事件
        let enabled_fields: Vec<RuleField> = payload.fields.iter().filter(|f| f.is_enabled).cloned().collect();
        let mut execution_log = format!("> 正则引擎秒级初筛就绪，启用 {} 项特征规则\n", enabled_fields.len());

        yield Ok(Event::default().event("init").data(serde_json::json!({
            "doc_id": payload.doc_id,
            "total_rules": enabled_fields.len(),
        }).to_string()));

        let mut streamed_extractor = StreamingEntityExtractor::new(doc.markdown.clone(), payload.fields.clone());
        let mut regex_items = Vec::new();
        let mut ai_items = Vec::new();

        // 1. 正则按需秒级提取 (<10ms)
        let raw_regex_items = Extractor::extract_by_regex(&doc.markdown, &payload.fields);
        if !raw_regex_items.is_empty() {
            execution_log.push_str(&format!("> 正则初筛快速检出 {} 项候选实体\n", raw_regex_items.len()));
        }
        for mut item in raw_regex_items {
            item.positions.clear();
            for (idx, _) in doc.markdown.match_indices(&item.text) {
                item.positions.push(idx);
            }
            item.count = item.positions.len();
            streamed_extractor.mark_seen(&item.text);
            if let Ok(data) = serde_json::to_string(&item) {
                yield Ok(Event::default().event("item").data(data));
            }
            regex_items.push(item);
        }

        let system_prompt_used = Extractor::build_system_prompt(
            &payload.fields,
            payload.custom_prompt.as_deref(),
        );

        let mut model_used_display = if payload.use_ai {
            if payload.model_type.as_deref() == Some("online") {
                "[在线] 在线大模型".to_string()
            } else {
                "[离线] 离线模型".to_string()
            }
        } else {
            "正则规则引擎".to_string()
        };

        // 2. 本地小模型 / 在线模型 AI 提取 (流式)
        if payload.use_ai {
            let is_online = payload.model_type.as_deref() == Some("online");
            let chunks = Extractor::chunk_text(&doc.markdown, 2500);

            if is_online {
                let online_models = state.session_mgr.get_online_models().await;
                let active_id = state.session_mgr.get_active_online_model_id().await;
                let target_cfg = if let Some(id) = &payload.online_model_id {
                    online_models.iter().find(|m| &m.id == id).cloned()
                } else {
                    online_models.iter().find(|m| Some(&m.id) == active_id.as_ref()).cloned().or_else(|| online_models.first().cloned())
                };

                if let Some(cfg) = target_cfg {
                    model_used_display = format!("[在线] {}", cfg.name);
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(90))
                        .build()
                        .unwrap_or_else(|_| reqwest::Client::new());

                    let trimmed_url = cfg.base_url.trim_end_matches('/');
                    let url = if trimmed_url.ends_with("/chat/completions") {
                        trimmed_url.to_string()
                    } else {
                        format!("{}/chat/completions", trimmed_url)
                    };

                    let effective_max_tokens = if cfg.max_tokens == 0 {
                        if cfg.enable_thinking { 4096 } else { 2048 }
                    } else {
                        cfg.max_tokens
                    };

                    for (_offset, chunk_text) in chunks {
                        let user_content = if chunk_text.starts_with("<document>") {
                            chunk_text.clone()
                        } else {
                            format!("<document>\n{}\n</document>", chunk_text)
                        };

                        let mut request_payload = serde_json::json!({
                            "model": cfg.model_id,
                            "messages": [
                                { "role": "system", "content": &system_prompt_used },
                                { "role": "user", "content": user_content }
                            ],
                            "temperature": cfg.temperature,
                            "top_k": cfg.top_k,
                            "repeat_penalty": cfg.repeat_penalty,
                            "max_tokens": effective_max_tokens,
                            "stream": true,
                        });

                        if cfg.enable_thinking {
                            request_payload["enable_thinking"] = serde_json::json!(true);
                        }

                        let mut req = client.post(&url).json(&request_payload);
                        if !cfg.api_key.trim().is_empty() {
                            req = req.header("Authorization", format!("Bearer {}", cfg.api_key.trim()));
                        }

                        if let Ok(resp) = req.send().await {
                            if resp.status().is_success() {
                                let mut byte_stream = resp.bytes_stream();
                                let mut line_buffer = String::new();
                                while let Some(item_res) = byte_stream.next().await {
                                    if let Ok(bytes) = item_res {
                                        line_buffer.push_str(&String::from_utf8_lossy(&bytes));
                                        while let Some(pos) = line_buffer.find('\n') {
                                            let line = line_buffer[..pos].to_string();
                                            line_buffer.drain(..=pos);
                                            if let Some(delta) = Extractor::parse_sse_delta(&line) {
                                                execution_log.push_str(&delta);
                                                if let Ok(d_json) = serde_json::to_string(&delta) {
                                                    yield Ok(Event::default().event("delta").data(d_json));
                                                }
                                                let emitted = streamed_extractor.push_delta(&delta);
                                                for em in emitted {
                                                    execution_log.push_str(&format!("\n[✔ 捕获实体] [{}] {} (出现 {} 次)\n", em.category, em.text, em.count));
                                                    if let Ok(data) = serde_json::to_string(&em) {
                                                        yield Ok(Event::default().event("item").data(data));
                                                    }
                                                    ai_items.push(em);
                                                }
                                            }
                                        }
                                    }
                                }
                                let finished_items = streamed_extractor.finish();
                                for em in finished_items {
                                    execution_log.push_str(&format!("\n[✔ 捕获实体] [{}] {} (出现 {} 次)\n", em.category, em.text, em.count));
                                    if let Ok(data) = serde_json::to_string(&em) {
                                        yield Ok(Event::default().event("item").data(data));
                                    }
                                    ai_items.push(em);
                                }
                            }
                        }
                    }
                }
            } else {
                // 本地离线模型
                let requested_model = payload.offline_model_name.as_deref().unwrap_or("").trim();
                let is_dual_engine = requested_model == "dual_engine";

                let target_model_file = if is_dual_engine {
                    let local_models = state.model_mgr.list_local_models();
                    if local_models.iter().any(|m| m == "Qwen3.5-0.8B-Q4_K_M.gguf") {
                        "Qwen3.5-0.8B-Q4_K_M.gguf".to_string()
                    } else if local_models.iter().any(|m| m == "MiniCPM5-2B-Q4_K_M.gguf") {
                        "MiniCPM5-2B-Q4_K_M.gguf".to_string()
                    } else {
                        local_models.first().cloned().unwrap_or_default()
                    }
                } else if !requested_model.is_empty() {
                    requested_model.to_string()
                } else {
                    state.model_mgr.get_active_model().await.unwrap_or_default()
                };

                if !target_model_file.is_empty() {
                    let active = state.model_mgr.get_active_model().await;
                    let is_ready = active.as_deref() == Some(&target_model_file) && state.model_mgr.is_server_ready().await;
                    if !is_ready {
                        let profile = state.session_mgr.get_offline_model_profile(&target_model_file).await;
                        if let Err(e) = state.model_mgr.start_model_with_profile(&target_model_file, Some(&profile)).await {
                            yield Ok(Event::default().event("error").data(serde_json::json!({"error": format!("拉起本地离线模型 {target_model_file} 失败: {e}")}).to_string()));
                            return;
                        }
                    }
                }

                let port = state.model_mgr.server_port();
                let current_model_name = state.model_mgr.get_active_model().await.unwrap_or_default();
                let offline_profile = state.session_mgr.get_offline_model_profile(&current_model_name).await;

                if is_dual_engine {
                    model_used_display = "[协同引擎] Qwen3.5-0.8B + MiniCPM5-2B".to_string();
                } else {
                    let display = if current_model_name.contains("Qwen") {
                        "Qwen3.5-0.8B"
                    } else if current_model_name.contains("MiniCPM") {
                        "MiniCPM5-2B"
                    } else {
                        &current_model_name
                    };
                    model_used_display = format!("[离线] {}", display);
                }

                let client = reqwest::Client::new();
                let url = format!("http://127.0.0.1:{}/v1/chat/completions", port);

                let effective_max_tokens = if offline_profile.max_tokens == 0 {
                    if offline_profile.enable_thinking { 2048 } else { 1024 }
                } else {
                    offline_profile.max_tokens
                };

                for (_offset, chunk_text) in chunks {
                    let user_content = if chunk_text.starts_with("<document>") {
                        chunk_text.clone()
                    } else {
                        format!("<document>\n{}\n</document>", chunk_text)
                    };

                    let mut request_payload = serde_json::json!({
                        "messages": [
                            { "role": "system", "content": &system_prompt_used },
                            { "role": "user", "content": user_content }
                        ],
                        "temperature": offline_profile.temperature,
                        "top_k": offline_profile.top_k,
                        "repeat_penalty": offline_profile.repeat_penalty,
                        "max_tokens": effective_max_tokens,
                        "stream": true,
                    });

                    if !offline_profile.enable_thinking {
                        request_payload["reasoning_budget"] = serde_json::json!(0);
                    }

                    if let Ok(resp) = client.post(&url).json(&request_payload).send().await {
                        if resp.status().is_success() {
                            let mut byte_stream = resp.bytes_stream();
                            let mut line_buffer = String::new();
                            while let Some(item_res) = byte_stream.next().await {
                                if let Ok(bytes) = item_res {
                                    line_buffer.push_str(&String::from_utf8_lossy(&bytes));
                                    while let Some(pos) = line_buffer.find('\n') {
                                        let line = line_buffer[..pos].to_string();
                                        line_buffer.drain(..=pos);
                                        if let Some(delta) = Extractor::parse_sse_delta(&line) {
                                            execution_log.push_str(&delta);
                                            if let Ok(d_json) = serde_json::to_string(&delta) {
                                                yield Ok(Event::default().event("delta").data(d_json));
                                            }
                                            let emitted = streamed_extractor.push_delta(&delta);
                                            for em in emitted {
                                                execution_log.push_str(&format!("\n[✔ 捕获实体] [{}] {} (出现 {} 次)\n", em.category, em.text, em.count));
                                                if let Ok(data) = serde_json::to_string(&em) {
                                                    yield Ok(Event::default().event("item").data(data));
                                                }
                                                ai_items.push(em);
                                            }
                                        }
                                    }
                                }
                            }
                            let finished_items = streamed_extractor.finish();
                            for em in finished_items {
                                execution_log.push_str(&format!("\n[✔ 捕获实体] [{}] {} (出现 {} 次)\n", em.category, em.text, em.count));
                                if let Ok(data) = serde_json::to_string(&em) {
                                    yield Ok(Event::default().event("item").data(data));
                                }
                                ai_items.push(em);
                            }
                        }
                    }
                }

                if is_dual_engine {
                    ai_items = extractor::PostFilterGuard::sanitize_items(ai_items, &payload.fields, &doc.markdown);
                }
            }
        }

        // 3. 最终合并与快照持久化
        let final_items = Extractor::merge_and_resolve(&doc.markdown, regex_items, ai_items, &payload.fields);
        let execution_ms = start_time.elapsed().as_millis() as u64;
        let duration_sec = if execution_ms < 100 { "< 0.1".to_string() } else { format!("{:.1}", execution_ms as f64 / 1000.0) };
        execution_log.push_str(&format!(
            "\n\n> ═══ 提取全部完成 ═══\n> 耗时: {}s ({}ms), 检出: {} 项\n> 执行模型: {}\n",
            duration_sec,
            execution_ms,
            final_items.len(),
            model_used_display
        ));

        let snapshot = state.session_mgr.add_snapshot(
            &payload.doc_id,
            payload.template_name,
            Some(model_used_display),
            Some(system_prompt_used),
            payload.fields,
            final_items,
            execution_ms,
            Some(execution_log),
        ).await;

        match snapshot {
            Ok(snap) => {
                if let Ok(snap_json) = serde_json::to_string(&snap) {
                    yield Ok(Event::default().event("done").data(snap_json));
                }
            }
            Err(err) => {
                yield Ok(Event::default().event("error").data(serde_json::json!({"error": err}).to_string()));
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
}

async fn get_model_presets(
    State(state): State<AppState>,
) -> Json<Vec<model_manager::ModelPreset>> {
    Json(state.model_mgr.get_presets().await)
}

async fn list_local_models(
    State(state): State<AppState>,
) -> Json<Vec<String>> {
    Json(state.model_mgr.list_local_models())
}

async fn list_local_mmprojs(
    State(state): State<AppState>,
) -> Json<Vec<String>> {
    Json(state.model_mgr.list_local_mmproj())
}

async fn start_model(
    State(state): State<AppState>,
    Json(payload): Json<StartModelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let port = state.model_mgr.server_port();
    let profile = state.session_mgr.get_offline_model_profile(&payload.filename).await;
    match state.model_mgr.start_model_with_profile(&payload.filename, Some(&profile)).await {
        Ok(_) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("模型 {} 启动成功并在 {} 端口就绪", payload.filename, port)
        }))),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: err }),
        )),
    }
}

async fn stop_model(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    state.model_mgr.stop_server().await;
    Json(serde_json::json!({
        "status": "success",
        "message": "llama-server 已停止并释放显存与端口"
    }))
}

async fn download_model(
    State(state): State<AppState>,
    Json(payload): Json<DownloadModelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mgr = state.model_mgr.clone();
    let model_id = payload.model_id.clone();

    // 在后台异步下载，通过 SSE 实时向前端汇报进度
    tokio::spawn(async move {
        if let Err(e) = mgr.download_model(&model_id).await {
            tracing::error!("后台下载模型 {} 失败: {}", model_id, e);
        }
    });

    Ok(Json(serde_json::json!({
        "status": "started",
        "message": "已开始在后台下载模型，请监听 SSE 进度通知"
    })))
}

async fn cancel_download_model(
    State(state): State<AppState>,
    Json(payload): Json<DownloadModelRequest>,
) -> Json<serde_json::Value> {
    let mgr = state.model_mgr.clone();
    let model_id = payload.model_id.clone();
    let canceled = mgr.cancel_download(&model_id).await;

    Json(serde_json::json!({
        "status": "success",
        "canceled": canceled,
        "message": "已成功取消下载并清除本地缓存"
    }))
}

#[derive(Deserialize)]
struct DeleteModelRequest {
    filename: String,
}

async fn delete_model_handler(
    State(state): State<AppState>,
    Json(payload): Json<DeleteModelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.model_mgr.delete_model(&payload.filename).await {
        Ok(_) => Ok(Json(serde_json::json!({ "status": "success", "message": "模型已成功从磁盘删除" }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "error": e })),
        )),
    }
}

async fn download_progress_sse(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.model_mgr.get_progress_receiver();

    let stream = async_stream::stream! {
        while let Ok(progress) = rx.recv().await {
            if let Ok(data) = serde_json::to_string(&progress) {
                yield Ok(Event::default().event("progress").data(data));
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
}

async fn get_ocr_status_handler() -> Json<crate::paths::OcrStatus> {
    let mut status = crate::paths::get_ocr_status();
    status.is_loaded = crate::ocr::OcrEngine::is_loaded();
    Json(status)
}

async fn start_ocr_handler() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    tokio::task::spawn_blocking(|| {
        crate::ocr::OcrEngine::get_or_init()
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("OCR 启动任务异常: {e}") })))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e })))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "OCR 原生推理引擎已成功载入内存"
    })))
}

async fn unload_ocr_handler() -> Json<serde_json::Value> {
    let unloaded = crate::ocr::OcrEngine::unload();
    Json(serde_json::json!({
        "status": "success",
        "unloaded": unloaded,
        "message": if unloaded { "OCR 原生推理引擎已成功释放内存" } else { "OCR 引擎未载入或正被占用" }
    }))
}

async fn download_ocr_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mgr = state.model_mgr.clone();

    tokio::spawn(async move {
        if let Err(e) = mgr.download_ocr_bundle().await {
            tracing::error!("后台下载 OCR 模型套件失败: {}", e);
        }
    });

    Ok(Json(serde_json::json!({
        "status": "started",
        "model_id": crate::model_manager::OCR_BUNDLE_ID,
        "message": "已开始在后台下载 OCR 模型套件，请监听 SSE 进度通知"
    })))
}

async fn cancel_ocr_handler(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let mgr = state.model_mgr.clone();
    let canceled = mgr.cancel_ocr_download().await;

    Json(serde_json::json!({
        "status": "success",
        "canceled": canceled,
        "message": "已成功取消 OCR 下载并清除本地缓存"
    }))
}

async fn delete_ocr_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    crate::ocr::OcrEngine::unload();
    let mgr = state.model_mgr.clone();
    mgr.delete_ocr_bundle()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e })))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "已成功删除本地 OCR 模型组件并释放内存"
    })))
}

async fn get_active_model_status(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let active = state.model_mgr.get_active_model().await;
    Json(serde_json::json!({
        "active_model": active
    }))
}

async fn import_external_model(
    State(state): State<AppState>,
    Json(payload): Json<ImportModelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.model_mgr.import_external_model(&payload.file_path).await {
        Ok(filename) => Ok(Json(serde_json::json!({
            "status": "success",
            "filename": filename,
            "message": "模型导入就绪，可直接在列表中启动"
        }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        )),
    }
}

async fn pick_and_import_model(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.model_mgr.pick_file_dialog().await {
        Ok(Some(file_path)) => {
            match state.model_mgr.import_external_model(&file_path).await {
                Ok(filename) => Ok(Json(serde_json::json!({
                    "status": "success",
                    "canceled": false,
                    "filename": filename,
                    "file_path": file_path,
                    "message": "模型导入挂载成功"
                }))),
                Err(e) => Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse { error: e }),
                )),
            }
        }
        Ok(None) => {
            // 用户取消了选择
            Ok(Json(serde_json::json!({
                "status": "canceled",
                "canceled": true
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )),
    }
}

async fn get_model_prompt(
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Json<ModelPromptProfile> {
    Json(state.session_mgr.get_model_prompt_profile(&filename).await)
}

async fn save_model_prompt(
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
    Json(payload): Json<ModelPromptProfile>,
) -> Json<serde_json::Value> {
    state.session_mgr.save_model_prompt_profile(filename, payload).await;
    Json(serde_json::json!({
        "status": "success",
        "message": "模型专属最佳提示词已保存"
    }))
}

async fn get_all_model_prompts(
    State(state): State<AppState>,
) -> Json<HashMap<String, ModelPromptProfile>> {
    Json(state.session_mgr.get_all_model_prompt_profiles().await)
}

async fn run_model_auto_benchmark(
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Json<benchmark::BenchmarkReport>, (StatusCode, Json<ErrorResponse>)> {
    let port = state.model_mgr.server_port();
    // 检查并确保该模型在专属端口启动
    let active = state.model_mgr.get_active_model().await;
    if active.as_deref() != Some(&filename) {
        if let Err(e) = state.model_mgr.start_model(&filename).await {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("拉起模型 {filename} 失败: {e}"),
                }),
            ));
        }
    }

    match BenchmarkEngine::run_benchmark(&filename, port).await {
        Ok(report) => Ok(Json(report)),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: err }),
        )),
    }
}

async fn get_online_models(
    State(state): State<AppState>,
) -> Json<Vec<OnlineModelProfile>> {
    Json(state.session_mgr.get_online_models().await)
}

async fn get_active_online_model(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let active_id = state.session_mgr.get_active_online_model_id().await;
    Json(serde_json::json!({
        "active_id": active_id
    }))
}

#[derive(Deserialize)]
struct SetActiveOnlineModelRequest {
    active_id: Option<String>,
}

async fn set_active_online_model(
    State(state): State<AppState>,
    Json(payload): Json<SetActiveOnlineModelRequest>,
) -> Json<serde_json::Value> {
    state.session_mgr.set_active_online_model_id(payload.active_id).await;
    Json(serde_json::json!({
        "status": "success"
    }))
}

async fn save_online_model(
    State(state): State<AppState>,
    Json(payload): Json<OnlineModelProfile>,
) -> Json<OnlineModelProfile> {
    let saved = state.session_mgr.save_online_model(payload).await;
    Json(saved)
}

async fn delete_online_model(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_mgr.delete_online_model(&id).await {
        Ok(active_id) => Ok(Json(serde_json::json!({
            "status": "success",
            "active_id": active_id
        }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        )),
    }
}

async fn test_online_model(
    Json(payload): Json<OnlineModelProfile>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match Extractor::test_online_model_connection(&payload.base_url, &payload.api_key, &payload.model_id).await {
        Ok(_) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": "在线模型服务连接测试成功！"
        }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        )),
    }
}

#[derive(Deserialize)]
struct SaveOfflineProfileRequest {
    filename: String,
    temperature: f32,
    top_k: u32,
    repeat_penalty: f32,
    #[serde(default = "default_max_tokens_u32")]
    max_tokens: u32,
    #[serde(default)]
    enable_thinking: bool,
    #[serde(default)]
    mmproj: Option<String>,
}

fn default_max_tokens_u32() -> u32 {
    1024
}

async fn get_all_offline_profiles(
    State(state): State<AppState>,
) -> Json<HashMap<String, OfflineModelProfile>> {
    Json(state.session_mgr.get_all_offline_model_profiles().await)
}

async fn save_offline_profile(
    State(state): State<AppState>,
    Json(payload): Json<SaveOfflineProfileRequest>,
) -> Json<serde_json::Value> {
    let profile = OfflineModelProfile {
        temperature: payload.temperature,
        top_k: payload.top_k,
        repeat_penalty: payload.repeat_penalty,
        max_tokens: payload.max_tokens,
        enable_thinking: payload.enable_thinking,
        mmproj: payload.mmproj,
    };
    state.session_mgr.save_offline_model_profile(payload.filename, profile).await;
    Json(serde_json::json!({ "status": "success" }))
}

#[derive(Deserialize)]
struct AiGenerateRulesRequest {
    model_id: Option<String>,
    prompt: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AiGeneratedField {
    name: String,
    level: String, // "high", "medium", "low"
    description: String,
}

#[derive(Serialize)]
struct AiGenerateRulesResponse {
    fields: Vec<AiGeneratedField>,
}

async fn ai_generate_rules(
    State(state): State<AppState>,
    Json(payload): Json<AiGenerateRulesRequest>,
) -> Result<Json<AiGenerateRulesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let prompt = payload.prompt.trim();
    if prompt.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "场景需求描述不能为空".into(),
            }),
        ));
    }

    let raw_model_id = payload.model_id.as_deref().unwrap_or("").trim();
    let online_models = state.session_mgr.get_online_models().await;
    let local_models = state.model_mgr.list_local_models();

    // 智能识别是否为离线模型 (以 offline: 或 local: 开头，或匹配本地已有模型文件名)
    let is_offline = raw_model_id.starts_with("offline:")
        || raw_model_id.starts_with("local:")
        || (!raw_model_id.starts_with("online:")
            && !raw_model_id.is_empty()
            && local_models.iter().any(|m| m == raw_model_id));

    let content = if is_offline {
        let filename = raw_model_id
            .trim_start_matches("offline:")
            .trim_start_matches("local:")
            .trim();

        let target_file = if filename.is_empty() || filename == "dual_engine" {
            if let Some(active) = state.model_mgr.get_active_model().await {
                active
            } else if let Some(first) = local_models.first() {
                first.clone()
            } else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "未找到任何可用的本地离线模型，请先下载模型".into(),
                    }),
                ));
            }
        } else {
            filename.to_string()
        };

        // 确保离线模型已在 llama-server 启动就绪
        let active_model = state.model_mgr.get_active_model().await;
        let is_running = active_model.as_deref() == Some(&target_file) && state.model_mgr.is_server_ready().await;
        if !is_running {
            let profile = state.session_mgr.get_offline_model_profile(&target_file).await;
            if let Err(e) = state.model_mgr.start_model_with_profile(&target_file, Some(&profile)).await {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("拉起本地离线模型 {target_file} 失败: {e}"),
                    }),
                ));
            }
        }

        let port = state.model_mgr.server_port();
        let url = format!("http://127.0.0.1:{}/v1/chat/completions", port);

        let system_instruction = r#"你是一个专业的数据合规、信息脱敏与隐私保护专家。
请根据用户提供的业务文档场景与脱敏需求，提炼出最关键的 3 到 8 个敏感信息字段及其规则定义。
必须直接返回纯 JSON 格式的数组，严禁包含任何 Markdown 格式符号 (如 ```json 或 ```) 或多余解释文字。

JSON 数组中的每个对象结构必须为：
[
  {
    "name": "字段名称 (简明扼要，如：患者姓名、集装箱号、银行卡号)",
    "level": "敏感度等级，只能为 high (高)、medium (中) 或 low (低)",
    "description": "字段脱敏判定说明或匹配特征"
  }
]"#;

        let request_payload = serde_json::json!({
            "messages": [
                { "role": "system", "content": system_instruction },
                { "role": "user", "content": format!("业务场景需求描述：\n{}", prompt) }
            ],
            "temperature": 0.2,
            "max_tokens": 1024
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("构建网络请求失败: {e}") })))?;

        let resp = client.post(&url).json(&request_payload).send().await.map_err(|e| {
            (StatusCode::BAD_GATEWAY, Json(ErrorResponse { error: format!("连接本地离线模型服务失败: {e}") }))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse { error: format!("本地离线模型接口返回错误 [{status}]: {body}") }),
            ));
        }

        let json_resp: serde_json::Value = resp.json().await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("解析离线模型响应失败: {e}") }))
        })?;

        json_resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        // 在线模型
        let clean_online_id = raw_model_id.trim_start_matches("online:").trim();
        let active_id = state.session_mgr.get_active_online_model_id().await;

        let target_model = if !clean_online_id.is_empty() {
            online_models.iter().find(|m| m.id == clean_online_id || m.model_id == clean_online_id)
        } else {
            None
        };

        let target_model = target_model.or_else(|| {
            if let Some(ref aid) = active_id {
                online_models.iter().find(|m| &m.id == aid)
            } else {
                online_models.first()
            }
        });

        let profile = match target_model {
            Some(m) => m,
            None => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "未检测到可用的在线 AI 模型。请先在右上角「设置 - 在线 AI 模型」中添加并配置 API Key (如 DeepSeek/OpenAI 等)".into(),
                    }),
                ));
            }
        };

        let system_instruction = r#"你是一个专业的数据合规、信息脱敏与隐私保护专家。
请根据用户提供的业务文档场景与脱敏需求，提炼出最关键的 3 到 8 个敏感信息字段及其规则定义。
必须直接返回纯 JSON 格式的数组，严禁包含任何 Markdown 格式符号 (如 ```json 或 ```) 或多余解释文字。

JSON 数组中的每个对象结构必须为：
[
  {
    "name": "字段名称 (简明扼要，如：患者姓名、集装箱号、银行卡号)",
    "level": "敏感度等级，只能为 high (高)、medium (中) 或 low (低)",
    "description": "字段脱敏判定说明或匹配特征"
  }
]"#;

        let request_payload = serde_json::json!({
            "model": profile.model_id,
            "messages": [
                { "role": "system", "content": system_instruction },
                { "role": "user", "content": format!("业务场景需求描述：\n{}", prompt) }
            ],
            "temperature": 0.2
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("构建网络请求失败: {e}") })))?;

        let trimmed_url = profile.base_url.trim_end_matches('/');
        let url = if trimmed_url.ends_with("/chat/completions") {
            trimmed_url.to_string()
        } else {
            format!("{}/chat/completions", trimmed_url)
        };

        let mut req = client.post(&url).json(&request_payload);
        if !profile.api_key.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", profile.api_key.trim()));
        }

        let resp = req.send().await.map_err(|e| {
            (StatusCode::BAD_GATEWAY, Json(ErrorResponse { error: format!("连接在线模型服务失败: {e}") }))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse { error: format!("在线模型接口返回错误 [{status}]: {body}") }),
            ));
        }

        let json_resp: serde_json::Value = resp.json().await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("解析在线模型响应失败: {e}") }))
        })?;

        json_resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string()
    };

    // 智能提取 JSON 数组片段
    let cleaned_json = if let (Some(start), Some(end)) = (content.find('['), content.rfind(']')) {
        if end > start {
            &content[start..=end]
        } else {
            &content
        }
    } else {
        &content
    };

    let fields: Vec<AiGeneratedField> = match serde_json::from_str(cleaned_json) {
        Ok(f) => f,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("模型返回的格式不符合预期 JSON: {e}\n原文: {content}"),
                }),
            ));
        }
    };

    Ok(Json(AiGenerateRulesResponse { fields }))
}

#[derive(Debug, Deserialize, Default)]
struct FastVlmRequest {
    pub threshold: Option<f32>,
}

#[derive(Serialize)]
struct FastVlmResponse {
    success: bool,
    corrected_count: usize,
    corrections: Vec<ocr::vlm::FastVlmCorrection>,
    inspected: Vec<ocr::vlm::FastVlmInspectedItem>,
    threshold: f32,
    markdown: String,
    low_confidence_count: usize,
    message: String,
}

/// 解析用于单据复核与全量重构的推理引擎 (委托至 ocr::vlm)
async fn resolve_vlm_engine(state: &AppState) -> Result<ocr::vlm::VlmEngineInfo, String> {
    ocr::vlm::resolve_vlm_engine(&state.model_mgr, &state.session_mgr).await
}

async fn fast_vlm_review_handler(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
    Query(query): Query<FastVlmRequest>,
    payload: Option<Json<FastVlmRequest>>,
) -> Result<Json<FastVlmResponse>, (StatusCode, Json<ErrorResponse>)> {
    let doc = state.session_mgr.get_document(&doc_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "未找到指定的文档".to_string(),
            }),
        )
    })?;

    let engine = resolve_vlm_engine(&state).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        )
    })?;

    let threshold = payload
        .as_ref()
        .and_then(|Json(p)| p.threshold)
        .or(query.threshold)
        .unwrap_or(0.85)
        .clamp(0.1, 0.99);

    let base_markdown = doc.base_markdown.as_ref().unwrap_or(&doc.markdown).clone();
    let img_bytes = doc.read_original_bytes().await.ok();

    let vlm_res = ocr::vlm::run_fast_vlm_pipeline(
        img_bytes.as_deref(),
        &base_markdown,
        &doc.raw_boxes,
        &engine,
        threshold,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )
    })?;

    let updated_doc = state
        .session_mgr
        .update_document_ocr_tier(
            &doc.id,
            "fast_vlm".to_string(),
            Some(vlm_res.markdown.clone()),
            Some(vlm_res.inspected.len()),
        )
        .await
        .unwrap_or(doc);

    let msg = if vlm_res.corrected_count > 0 {
        if vlm_res.low_confidence_count > 10 {
            format!(
                "已将 {} 处片段合并为 Top {} 重点单元格/行，通过 {} 成功修正 {} 处偏差",
                vlm_res.low_confidence_count,
                vlm_res.inspected.len(),
                engine.model_name,
                vlm_res.corrected_count
            )
        } else {
            format!(
                "已通过 {} 完成整行/单元格级智能复核，成功修正 {} 处疑似偏差",
                engine.model_name, vlm_res.corrected_count
            )
        }
    } else if !vlm_res.inspected.is_empty() {
        format!(
            "已通过 {} 完成 {} 个疑难单元格/整行复核确认无误",
            engine.model_name,
            vlm_res.inspected.len()
        )
    } else {
        format!(
            "当前阈值 (< {:.0}%) 下未发现低置信度区块，基础识别已确认无误",
            threshold * 100.0
        )
    };

    Ok(Json(FastVlmResponse {
        success: true,
        corrected_count: vlm_res.corrected_count,
        corrections: vlm_res.corrections,
        inspected: vlm_res.inspected,
        threshold,
        markdown: updated_doc.markdown,
        low_confidence_count: 0,
        message: msg,
    }))
}

async fn full_vlm_stream_handler(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let doc = match state.session_mgr.get_document(&doc_id).await {
            Some(d) => d,
            None => {
                yield Ok(Event::default().event("error").data(serde_json::json!({"error": "未找到指定文档"}).to_string()));
                return;
            }
        };

        let engine = match resolve_vlm_engine(&state).await {
            Ok(e) => e,
            Err(err_msg) => {
                yield Ok(Event::default().event("error").data(serde_json::json!({"error": err_msg}).to_string()));
                return;
            }
        };

        // 发送初始化通知事件，告诉前端当前正在使用的模型
        yield Ok(Event::default().event("init").data(serde_json::json!({
            "model_name": engine.model_name
        }).to_string()));

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                yield Ok(Event::default().event("error").data(serde_json::json!({"error": format!("构建 HTTP 客户端失败: {e}")}).to_string()));
                return;
            }
        };

        let request_payload = if engine.is_vision_native {
            // 在线 Vision 模式：图片 Base64 + Prompt
            let file_bytes = match doc.read_original_bytes().await {
                Ok(b) => b,
                Err(e) => {
                    yield Ok(Event::default().event("error").data(serde_json::json!({"error": format!("读取单据图片失败: {e}")}).to_string()));
                    return;
                }
            };
            let b64_data_url = match tokio::task::spawn_blocking(move || -> Result<String, String> {
                let img = image::load_from_memory(&file_bytes).map_err(|e| format!("加载图像失败: {e}"))?;
                let (w, h) = (img.width(), img.height());
                let target = if w > 2048 || h > 2048 {
                    img.resize(2048, 2048, image::imageops::FilterType::Triangle)
                } else {
                    img
                };
                let mut buf = std::io::Cursor::new(Vec::new());
                target.write_to(&mut buf, image::ImageFormat::Jpeg).map_err(|e| format!("编码图像失败: {e}"))?;
                Ok(format!("data:image/jpeg;base64,{}", crate::ocr::base64_encode(&buf.into_inner())))
            }).await {
                Ok(Ok(url)) => url,
                _ => {
                    yield Ok(Event::default().event("error").data(serde_json::json!({"error": "图像预处理失败"}).to_string()));
                    return;
                }
            };

            serde_json::json!({
                "model": engine.model_id,
                "messages": [
                    {
                        "role": "system",
                        "content": ocr::vlm::FULL_VLM_SYSTEM_PROMPT
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
                "stream": true,
                "temperature": 0.1,
                "max_tokens": 4096,
                "frequency_penalty": 0.2,
                "presence_penalty": 0.1
            })
        } else {
            // 本地 Qwen3.5-0.8B 纯文本模式：全量 OCR 排版高保真重构
            let base_text = doc.base_markdown.as_deref().unwrap_or(&doc.markdown);
            serde_json::json!({
                "model": engine.model_id,
                "messages": [
                    {
                        "role": "system",
                        "content": "你是一名顶级单据与发票高保真结构化排版专家。请根据 OCR 初步识别的单据内容，将其重构为结构清晰、排版工整的高保真 Markdown 格式。\n\n【排版准则】：\n1. 核心表格中的【发票号/采购单号】（如 2605011672 等数字或单号）必须逐行完整保留，严禁丢失！序号与公司名同属于细节描述列；\n2. 纠正所有 OCR 识别错字、漏字和拼写错误；\n3. 所有表格与键值对必须严格排版为标准的 GitHub Flavored Markdown 表格；\n4. 层次清晰分明，标题使用 # / ## 标记，列表使用 - 标记；\n5. 仅直接流式输出重构后的 Markdown 正文，绝对禁止输出任何解释说明、不要使用 ```markdown 代码块包裹。"
                    },
                    {
                        "role": "user",
                        "content": format!("单据原始识别内容如下，请重构为高保真 Markdown（发票号/采购单号必须逐行保留）：\n\n{}", base_text)
                    }
                ],
                "stream": true,
                "temperature": 0.1,
                "max_tokens": 4096,
                "frequency_penalty": 0.2,
                "presence_penalty": 0.1
            })
        };

        let trimmed_url = engine.url;
        let url = if trimmed_url.ends_with("/chat/completions") {
            trimmed_url.to_string()
        } else {
            format!("{}/chat/completions", trimmed_url)
        };

        let mut req = client.post(&url).json(&request_payload);
        if !engine.api_key.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", engine.api_key.trim()));
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                yield Ok(Event::default().event("error").data(serde_json::json!({"error": format!("请求视觉模型失败: {e}")}).to_string()));
                return;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            yield Ok(Event::default().event("error").data(serde_json::json!({"error": format!("视觉模型返回错误 [{status}]: {body}")}).to_string()));
            return;
        }

        let mut byte_stream = resp.bytes_stream();
        let mut line_buf = String::new();
        let mut full_accumulated = String::new();
        let mut in_thinking = false;

        while let Some(item) = byte_stream.next().await {
            let chunk = match item {
                Ok(c) => c,
                Err(e) => {
                    yield Ok(Event::default().event("error").data(serde_json::json!({"error": format!("读取模型流失败: {e}")}).to_string()));
                    return;
                }
            };

            if let Ok(text) = std::str::from_utf8(&chunk) {
                line_buf.push_str(text);
                while let Some(pos) = line_buf.find('\n') {
                    let line = line_buf[..pos].trim().to_string();
                    line_buf = line_buf[pos + 1..].to_string();

                    if line.starts_with("data: ") {
                        let data_str = line[6..].trim();
                        if data_str == "[DONE]" {
                            break;
                        }
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(data_str) {
                            if let Some(delta) = val["choices"][0]["delta"]["content"].as_str() {
                                if !delta.is_empty() {
                                    if delta.contains("<think>") {
                                        in_thinking = true;
                                    }
                                    if in_thinking {
                                        if delta.contains("</think>") {
                                            in_thinking = false;
                                        }
                                    } else {
                                        full_accumulated.push_str(delta);
                                        yield Ok(Event::default().event("chunk").data(serde_json::json!({"delta": delta}).to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let trimmed = full_accumulated.trim();
        let clean_md = if (trimmed.starts_with("```markdown") || trimmed.starts_with("```md")) && trimmed.ends_with("```") {
            let prefix_len = if trimmed.starts_with("```markdown") { 11 } else { 5 };
            trimmed[prefix_len..trimmed.len() - 3].trim().to_string()
        } else if trimmed.starts_with("```") && trimmed.ends_with("```") {
            trimmed[3..trimmed.len() - 3].trim().to_string()
        } else {
            trimmed.to_string()
        };

        let char_count = clean_md.chars().count();
        state.session_mgr.update_document_ocr_tier(&doc_id, "full_vlm".to_string(), Some(clean_md.clone()), Some(0)).await;

        yield Ok(Event::default().event("done").data(serde_json::json!({
            "markdown": clean_md,
            "char_count": char_count,
            "model_name": engine.model_name
        }).to_string()));
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
}

#[derive(Deserialize)]
struct SetOcrTierRequest {
    tier: String,
}

async fn set_ocr_tier_handler(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
    Json(payload): Json<SetOcrTierRequest>,
) -> Result<Json<DocumentItem>, (StatusCode, Json<ErrorResponse>)> {
    let updated = state
        .session_mgr
        .update_document_ocr_tier(&doc_id, payload.tier, None, None)
        .await
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "未找到指定的文档".to_string(),
                }),
            )
        })?;

    Ok(Json(updated))
}

async fn rerun_base_ocr_handler(
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let doc = state.session_mgr.get_document(&doc_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "未找到指定的文档".to_string(),
            }),
        )
    })?;

    let file_bytes = doc.read_original_bytes().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("读取文档原始文件失败: {e}"),
            }),
        )
    })?;

    let filename = doc.filename.clone();
    let res = tokio::task::spawn_blocking(move || {
        converter::DocConverter::convert_bytes_detailed(&filename, &file_bytes)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("重新执行基础 OCR 任务异常: {e}"),
            }),
        )
    })?
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("基础 OCR 识别失败: {e}"),
            }),
        )
    })?;

    let updated = state
        .session_mgr
        .update_document_ocr_tier(
            &doc_id,
            "base".to_string(),
            Some(res.markdown.clone()),
            Some(res.low_confidence_count),
        )
        .await
        .unwrap_or(doc);

    state.session_mgr.update_document_raw_boxes(&doc_id, res.raw_boxes).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "markdown": updated.markdown,
        "low_confidence_count": res.low_confidence_count,
        "message": "已重新完成基础 OCR 识别"
    })))
}
