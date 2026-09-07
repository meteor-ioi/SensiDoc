mod benchmark;
mod converter;
mod exporter;
mod extractor;
mod model_manager;
mod paths;
mod session;

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
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sensidoc=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Initializing SensiDoc HTTP Service...");

    let model_mgr = Arc::new(ModelManager::new());
    let session_mgr = Arc::new(SessionManager::new());
    let state = AppState {
        model_mgr: model_mgr.clone(),
        session_mgr: session_mgr.clone(),
    };

    // 优雅停机信号捕获：当用户 Ctrl+C 或发生 SIGTERM 时确保强杀 llama-server
    let mgr_for_shutdown = model_mgr.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::warn!("收到退出信号 (Ctrl+C)，正在终止 llama-server 并退出...");
        mgr_for_shutdown.stop_server().await;
        std::process::exit(0);
    });

    let web_dir = paths::get_web_dir();
    tracing::info!("Using static web directory: {:?}", web_dir);

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/convert", post(convert_document))
        .route("/api/documents", get(list_documents))
        .route("/api/documents/{id}", get(get_document).delete(delete_document))
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
        .route("/api/models/download/progress", get(download_progress_sse))
        .fallback_service(ServeDir::new(web_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // 检查是否有 --server 或 --headless 参数，支持纯无头后端服务模式
    let args: Vec<String> = std::env::args().collect();
    let is_headless = args.iter().any(|a| a == "--server" || a == "--headless");

    // 启动 Axum 后端服务
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => {
            tracing::info!("SensiDoc HTTP Service listening on http://{}", addr);
            l
        }
        Err(e) => {
            tracing::warn!("绑定端口 3000 失败 (可能已有服务在运行): {e}");
            if is_headless {
                return Ok(());
            }
            // 若端口已占且非 headless，直接用 WebView 打开已有服务
            return launch_desktop_gui(model_mgr.clone(), "http://127.0.0.1:3000");
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
        tracing::info!("SensiDoc 以 Headless 无头服务器模式运行中 (访问 http://127.0.0.1:3000)...");
        tokio::signal::ctrl_c().await.ok();
        model_mgr.stop_server().await;
        return Ok(());
    }

    // 默认以独立原生桌面 GUI 窗口模式启动
    launch_desktop_gui(model_mgr, "http://127.0.0.1:3000")
}

/// 启动独立原生桌面客户端窗口 (Tao + Wry)
fn launch_desktop_gui(
    model_mgr: Arc<ModelManager>,
    app_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use tao::{
        dpi::LogicalSize,
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };
    use wry::WebViewBuilder;

    tracing::info!("正在拉起 SensiDoc 独立原生桌面窗口...");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("SensiDoc - 离线信息审计与脱敏工具")
        .with_inner_size(LogicalSize::new(1280.0, 840.0))
        .with_min_inner_size(LogicalSize::new(960.0, 600.0))
        .build(&event_loop)?;

    let _webview = WebViewBuilder::new()
        .with_url(app_url)
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
            } => {
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
            for (_offset, chunk_text) in chunks {
                if let Ok(items) = Extractor::query_llm(8081, &system_prompt_used, &chunk_text).await {
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
    match state.model_mgr.start_model(&payload.filename).await {
        Ok(_) => Ok(Json(serde_json::json!({
            "status": "success",
            "message": format!("模型 {} 启动成功并在 8081 端口就绪", payload.filename)
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
    // 检查并确保该模型在 8081 启动
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

    match BenchmarkEngine::run_benchmark(&filename, 8081).await {
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




