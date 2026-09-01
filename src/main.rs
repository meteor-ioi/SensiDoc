mod benchmark;
mod converter;
mod exporter;
mod extractor;
mod model_manager;
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
use session::{DocumentItem, ExtractionSnapshot, ModelPromptProfile, OnlineAiConfig, SessionManager};
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
        .route("/api/settings/online-ai", get(get_online_ai_settings).post(save_online_ai_settings))
        .route("/api/settings/online-ai/test", post(test_online_ai_settings))
        .route("/api/models/import", post(import_external_model))
        .route("/api/models/pick-and-import", post(pick_and_import_model))
        .route("/api/models/start", post(start_model))
        .route("/api/models/stop", post(stop_model))
        .route("/api/models/download", post(download_model))
        .route("/api/models/download/progress", get(download_progress_sse))
        .fallback_service(ServeDir::new("web"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("SensiDoc web server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
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
        // 对超长文档分块 (Chunking) 送入模型处理
        let chunks = Extractor::chunk_text(&doc.markdown, 2500);
        for (_offset, chunk_text) in chunks {
            if let Ok(items) = Extractor::query_llm(8081, &system_prompt_used, &chunk_text).await {
                ai_items.extend(items);
            }
        }
    }

    // 3. 冲突消解、位置回填与合并排序（结合传入的规则自动分类与赋权）
    let final_items = Extractor::merge_and_resolve(&doc.markdown, regex_items, ai_items, &payload.fields);
    let execution_ms = start_time.elapsed().as_millis() as u64;

    // 获取当前实际执行本次提取的模型名称
    let model_name = if payload.use_ai {
        state.model_mgr.get_active_model().await
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

    let online_cfg = state.session_mgr.get_online_ai_config().await;
    let online_cfg_opt = if online_cfg.enabled { Some(&online_cfg) } else { None };

    match BenchmarkEngine::run_benchmark(&filename, 8081, online_cfg_opt).await {
        Ok(report) => Ok(Json(report)),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: err }),
        )),
    }
}

async fn get_online_ai_settings(
    State(state): State<AppState>,
) -> Json<OnlineAiConfig> {
    Json(state.session_mgr.get_online_ai_config().await)
}

async fn save_online_ai_settings(
    State(state): State<AppState>,
    Json(payload): Json<OnlineAiConfig>,
) -> Json<serde_json::Value> {
    state.session_mgr.save_online_ai_config(payload).await;
    Json(serde_json::json!({
        "status": "success",
        "message": "在线 AI 模型设置已成功保存"
    }))
}

async fn test_online_ai_settings(
    Json(payload): Json<OnlineAiConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match BenchmarkEngine::test_online_ai_connection(&payload).await {
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




