mod benchmark;
mod cli;
mod converter;
mod desensitizer;
mod exporter;
mod extractor;
mod model_manager;
mod paths;
mod session;

use clap::Parser;
use cli::{Cli, Commands};
use axum::{
    extract::{Multipart, Path as AxumPath, State},
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
use extractor::{Extractor, RuleField, RulePreset};
use futures_util::stream::Stream;
use model_manager::ModelManager;
use serde::{Deserialize, Serialize};
use session::{DocumentItem, ExtractionSnapshot, ModelPromptProfile, OnlineModelProfile, SessionManager};
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

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/convert", post(convert_document))
        .route("/api/documents", get(list_documents))
        .route("/api/documents/{id}", get(get_document).delete(delete_document))
        .route("/api/documents/{id}/desensitize", post(desensitize_document).get(desensitize_document))
        .route("/api/documents/{id}/snapshot/{snapshot_id}", post(set_active_snapshot).delete(delete_snapshot))
        .route("/api/rules/presets", get(get_rule_presets))
        .route("/api/rules/templates", post(save_custom_template))
        .route("/api/rules/templates/{id}", delete(delete_custom_template))
        .route("/api/rules/tags", get(get_field_tags).post(save_field_tag))
        .route("/api/rules/tags/{name}", delete(delete_field_tag))
        .route("/api/rules/prompt/preview", post(preview_system_prompt))
        .route("/api/extract", post(extract_sensitive_info))
        .route("/api/models/presets", get(get_model_presets))
        .route("/api/models/local", get(list_local_models))
        .route("/api/models/active", get(get_active_model_status))
        .route("/api/models/prompts/all", get(get_all_model_prompts))
        .route("/api/models/{filename}/prompt", get(get_model_prompt).post(save_model_prompt))
        .route("/api/models/{filename}/benchmark", post(run_model_auto_benchmark))
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
        .fallback_service(ServeDir::new(web_dir))
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
    use tao::{
        dpi::{LogicalPosition, LogicalSize},
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
        Close,
        DragWindow,
    }

    tracing::info!("正在拉起 SensiDoc 独立原生桌面窗口 (支持现代沉浸式无菜单标题栏)...");

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    let builder = WindowBuilder::new()
        .with_title("SensiDoc - 信息审计与脱敏工具")
        .with_inner_size(LogicalSize::new(1280.0, 840.0))
        .with_min_inner_size(LogicalSize::new(960.0, 600.0));

    // macOS: 隐藏原生文字标题，开启全尺寸内容视图与透明标题栏，原生的红黄绿三颗交通灯垂直居中浮动于左上角
    #[cfg(target_os = "macos")]
    let builder = builder
        .with_title_hidden(true)
        .with_titlebar_transparent(true)
        .with_fullsize_content_view(true)
        .with_traffic_light_inset(LogicalPosition::new(16.0, 15.0));

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

    let _webview = WebViewBuilder::new()
        .with_url(app_url)
        .with_initialization_script(&init_script)
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
                    let _ = ipc_proxy.send_event(UserEvent::Close);
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
            | Event::UserEvent(UserEvent::Close) => {
                tracing::info!("收到窗口关闭事件，正在安全释放模型进程并退出客户端...");
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

async fn convert_document(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ConvertResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut filename = String::from("document.txt");
    let mut file_bytes = Vec::new();

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
            break;
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

    match converter::DocConverter::convert_bytes(&filename, &file_bytes) {
        Ok(markdown) => {
            let doc = state.session_mgr.upsert_document(filename, markdown).await;
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
            }))
        }
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: err }),
        )),
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

#[derive(Deserialize, Default)]
struct DesensitizeRequest {
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    snapshot_id: Option<String>,
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
        let desensitized_md = Desensitizer::desensitize_plain_text(&doc.markdown, detected_items);
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
        // native 模式：优先读取上传暂存的原始二进制
        let upload_path = paths::get_uploads_dir().join(format!("{}.bin", doc.id));
        let orig_bytes = std::fs::read(&upload_path).ok();

        match Desensitizer::desensitize_document_auto(
            &doc.filename,
            orig_bytes.as_deref(),
            &doc.markdown,
            detected_items,
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
                        &system_prompt_used,
                        &chunk_text,
                    ).await {
                        ai_items.extend(items);
                    }
                }
            }
        } else {
            // 本地离线模型处理
            let port = state.model_mgr.server_port();
            for (_offset, chunk_text) in chunks {
                if let Ok(items) = Extractor::query_llm(port, &system_prompt_used, &chunk_text).await {
                    ai_items.extend(items);
                }
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
            state.model_mgr.get_active_model().await
        }
    } else {
        Some("正则规则引擎".to_string())
    };

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

async fn start_model(
    State(state): State<AppState>,
    Json(payload): Json<StartModelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let port = state.model_mgr.server_port();
    match state.model_mgr.start_model(&payload.filename).await {
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
        Ok(msg) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": msg
        }))),
        Err(err) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: err }),
        )),
    }
}




