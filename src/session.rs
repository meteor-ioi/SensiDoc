use crate::extractor::{RuleField, RulePreset, SensitiveItem};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// 提取历史快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSnapshot {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub template_name: String,
    #[serde(default)]
    pub model_name: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    pub fields_used: Vec<RuleField>,
    pub items: Vec<SensitiveItem>,
    pub execution_ms: u64,
}

/// 单个管理文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentItem {
    pub id: String,
    pub filename: String,
    pub markdown: String,
    pub char_count: usize,
    #[serde(default = "default_created_at")]
    pub created_at: DateTime<Utc>,
    pub snapshots: Vec<ExtractionSnapshot>,
    pub active_snapshot_id: Option<String>,
}

fn default_created_at() -> DateTime<Utc> {
    Utc::now()
}

/// 在线 AI 模型 (OpenAI 兼容 API) 配置档案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineModelProfile {
    pub id: String,
    pub name: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model_id")]
    pub model_id: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default = "default_repeat_penalty")]
    pub repeat_penalty: f32,
    #[serde(default)]
    pub enable_thinking: bool,
}

fn default_base_url() -> String {
    "https://api.deepseek.com/v1".to_string()
}

fn default_model_id() -> String {
    "deepseek-chat".to_string()
}

fn default_temperature() -> f32 {
    0.1
}

fn default_top_k() -> u32 {
    50
}

fn default_repeat_penalty() -> f32 {
    1.1
}

fn default_online_models() -> Vec<OnlineModelProfile> {
    vec![OnlineModelProfile {
        id: "deepseek-v3".to_string(),
        name: "DeepSeek-V3 (推荐)".to_string(),
        base_url: "https://api.deepseek.com/v1".to_string(),
        api_key: String::new(),
        model_id: "deepseek-chat".to_string(),
        temperature: 0.1,
        top_k: 50,
        repeat_penalty: 1.1,
        enable_thinking: false,
    }]
}

/// 离线本地 GGUF 模型推理超参数配置档案（跟着模型走）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfflineModelProfile {
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default = "default_repeat_penalty")]
    pub repeat_penalty: f32,
    #[serde(default)]
    pub enable_thinking: bool,
}

impl Default for OfflineModelProfile {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            top_k: default_top_k(),
            repeat_penalty: default_repeat_penalty(),
            enable_thinking: false,
        }
    }
}

/// 模型专属最佳提示词档案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPromptProfile {
    pub profile_name: String,
    #[serde(default)]
    pub f1_score: Option<f64>,
    pub custom_prompt: String,
}

/// 工作区持久化数据结构（包括文档、自定义模板与标签库、模型专属提示词、在线 AI 模型列表、离线模型超参数）
#[derive(Debug, Default, Serialize, Deserialize)]
struct WorkspaceStore {
    #[serde(default)]
    documents: Vec<DocumentItem>,
    #[serde(default)]
    custom_templates: Vec<RulePreset>,
    #[serde(default)]
    field_tags: Vec<RuleField>,
    #[serde(default)]
    model_prompt_profiles: HashMap<String, ModelPromptProfile>,
    #[serde(default)]
    online_models: Vec<OnlineModelProfile>,
    #[serde(default)]
    active_online_model_id: Option<String>,
    #[serde(default)]
    offline_model_profiles: HashMap<String, OfflineModelProfile>,
}

use std::time::SystemTime;

/// 全局文档会话管理器
pub struct SessionManager {
    store_path: PathBuf,
    last_disk_mtime: Arc<RwLock<Option<SystemTime>>>,
    documents: Arc<RwLock<HashMap<String, DocumentItem>>>,
    custom_templates: Arc<RwLock<Vec<RulePreset>>>,
    field_tags: Arc<RwLock<Vec<RuleField>>>,
    model_prompt_profiles: Arc<RwLock<HashMap<String, ModelPromptProfile>>>,
    online_models: Arc<RwLock<Vec<OnlineModelProfile>>>,
    active_online_model_id: Arc<RwLock<Option<String>>>,
    offline_model_profiles: Arc<RwLock<HashMap<String, OfflineModelProfile>>>,
}

impl SessionManager {
    pub const PROMPT_V4_ULTRA_COMPACT: &'static str = r#"【指令】：从文本中提取所有符合定义的敏感信息，输出纯 JSON 数组。

【字段定义】：
{FIELDS_DEFINITION}

【规则】：
1. 逐行扫描提取所有出现的敏感原词，不要漏掉任何一个。
2. 仅输出 JSON 对象数组：
[
  {"field": "字段名", "text": "原文原词"}
]
无任何多余解释。"#;

    pub const PROMPT_V1_BASELINE: &'static str = crate::extractor::Extractor::DEFAULT_SYSTEM_PROMPT_TEMPLATE;

    pub fn new() -> Self {
        Self::with_store_path(crate::paths::get_workspace_store_path())
    }

    pub fn with_store_path(store_path: PathBuf) -> Self {
        let initial_mtime = std::fs::metadata(&store_path).ok().and_then(|m| m.modified().ok());
        let (docs, templates, tags, profiles, online_models, active_id, offline_profiles) = Self::load_from_disk(&store_path);

        Self {
            store_path,
            last_disk_mtime: Arc::new(RwLock::new(initial_mtime)),
            documents: Arc::new(RwLock::new(docs)),
            custom_templates: Arc::new(RwLock::new(templates)),
            field_tags: Arc::new(RwLock::new(tags)),
            model_prompt_profiles: Arc::new(RwLock::new(profiles)),
            online_models: Arc::new(RwLock::new(online_models)),
            active_online_model_id: Arc::new(RwLock::new(active_id)),
            offline_model_profiles: Arc::new(RwLock::new(offline_profiles)),
        }
    }

    /// 从本地 JSON 读取落盘数据
    fn load_from_disk(path: &Path) -> (
        HashMap<String, DocumentItem>,
        Vec<RulePreset>,
        Vec<RuleField>,
        HashMap<String, ModelPromptProfile>,
        Vec<OnlineModelProfile>,
        Option<String>,
        HashMap<String, OfflineModelProfile>,
    ) {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(store) = serde_json::from_str::<WorkspaceStore>(&content) {
                    let online_models = if store.online_models.is_empty() {
                        default_online_models()
                    } else {
                        store.online_models
                    };
                    let active_id = store.active_online_model_id.or_else(|| online_models.first().map(|m| m.id.clone()));

                    info!(
                        "已从 {} 恢复 {} 个持久化文档记录, {} 个自定义模板, {} 个字段标签, {} 个模型提示词档案, {} 个在线模型配置, {} 个离线模型配置",
                        path.display(),
                        store.documents.len(),
                        store.custom_templates.len(),
                        store.field_tags.len(),
                        store.model_prompt_profiles.len(),
                        online_models.len(),
                        store.offline_model_profiles.len()
                    );
                    let docs_map = store.documents.into_iter().map(|d| (d.id.clone(), d)).collect();
                    return (docs_map, store.custom_templates, store.field_tags, store.model_prompt_profiles, online_models, active_id, store.offline_model_profiles);
                }
            }
        }
        (HashMap::new(), Vec::new(), Vec::new(), HashMap::new(), default_online_models(), Some("deepseek-v3".to_string()), HashMap::new())
    }

    /// 检查磁盘上的 .sensidoc_workspace.json 是否被外部进程（如 CLI）修改，若是则增量/热重载至内存
    pub async fn sync_from_disk_if_modified(&self) {
        let current_mtime = match std::fs::metadata(&self.store_path) {
            Ok(meta) => meta.modified().ok(),
            Err(_) => return,
        };

        let should_reload = {
            let last = self.last_disk_mtime.read().await;
            match (*last, current_mtime) {
                (Some(prev), Some(curr)) => curr > prev,
                (None, Some(_)) => true,
                _ => false,
            }
        };

        if should_reload {
            let (docs, templates, tags, profiles, online_models, active_id, offline_profiles) = Self::load_from_disk(&self.store_path);
            {
                let mut docs_lock = self.documents.write().await;
                *docs_lock = docs;
            }
            {
                let mut t_lock = self.custom_templates.write().await;
                *t_lock = templates;
            }
            {
                let mut tags_lock = self.field_tags.write().await;
                *tags_lock = tags;
            }
            {
                let mut p_lock = self.model_prompt_profiles.write().await;
                *p_lock = profiles;
            }
            {
                let mut m_lock = self.online_models.write().await;
                *m_lock = online_models;
            }
            {
                let mut a_lock = self.active_online_model_id.write().await;
                *a_lock = active_id;
            }
            {
                let mut off_lock = self.offline_model_profiles.write().await;
                *off_lock = offline_profiles;
            }
            {
                let mut last = self.last_disk_mtime.write().await;
                *last = current_mtime;
            }
            info!("已从外部磁盘热同步最新工作区数据 (mtime 更新)");
        }
    }

    /// 异步刷盘持久化
    pub async fn save_to_disk(&self) {
        let map = self.documents.read().await;
        let templates = self.custom_templates.read().await;
        let tags = self.field_tags.read().await;
        let profiles = self.model_prompt_profiles.read().await;
        let online_models = self.online_models.read().await;
        let active_id = self.active_online_model_id.read().await;
        let offline_profiles = self.offline_model_profiles.read().await;

        let store = WorkspaceStore {
            documents: map.values().cloned().collect(),
            custom_templates: templates.clone(),
            field_tags: tags.clone(),
            model_prompt_profiles: profiles.clone(),
            online_models: online_models.clone(),
            active_online_model_id: active_id.clone(),
            offline_model_profiles: offline_profiles.clone(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&store) {
            if tokio::fs::write(&self.store_path, json).await.is_ok() {
                if let Ok(meta) = tokio::fs::metadata(&self.store_path).await {
                    if let Ok(mtime) = meta.modified() {
                        let mut last = self.last_disk_mtime.write().await;
                        *last = Some(mtime);
                    }
                }
            }
        }
    }

    /// 添加或保存自定义场景模板 (后添加的排在前面)
    pub async fn save_template(&self, template: RulePreset) {
        let mut list = self.custom_templates.write().await;
        if let Some(existing) = list.iter_mut().find(|t| t.id == template.id) {
            *existing = template;
        } else {
            list.insert(0, template);
        }
        drop(list);
        self.save_to_disk().await;
    }

    /// 删除自定义场景模板
    pub async fn delete_template(&self, template_id: &str) -> bool {
        let mut list = self.custom_templates.write().await;
        let original_len = list.len();
        list.retain(|t| t.id != template_id);
        let removed = list.len() < original_len;
        drop(list);
        if removed {
            self.save_to_disk().await;
        }
        removed
    }

    /// 获取所有自定义模板
    pub async fn get_custom_templates(&self) -> Vec<RulePreset> {
        self.sync_from_disk_if_modified().await;
        let list = self.custom_templates.read().await;
        list.clone()
    }

    /// 保存/新增字段标签到公共标签库 (后添加的排在前面)
    pub async fn save_field_tag(&self, tag: RuleField) {
        let mut list = self.field_tags.write().await;
        if let Some(existing) = list.iter_mut().find(|t| t.name == tag.name) {
            *existing = tag;
        } else {
            list.insert(0, tag);
        }
        drop(list);
        self.save_to_disk().await;
    }

    /// 删除公共标签库中的指定标签
    pub async fn delete_field_tag(&self, name: &str) -> bool {
        let mut list = self.field_tags.write().await;
        let original_len = list.len();
        list.retain(|t| t.name != name);
        let removed = list.len() < original_len;
        drop(list);
        if removed {
            self.save_to_disk().await;
        }
        removed
    }

    /// 获取全部保存的字段标签
    pub async fn get_field_tags(&self) -> Vec<RuleField> {
        self.sync_from_disk_if_modified().await;
        let list = self.field_tags.read().await;
        list.clone()
    }

    /// 获取指定模型绑定的最佳提示词档案（若无自定义则根据模型名称正则自动匹配内置最佳模板）
    pub async fn get_model_prompt_profile(&self, model_filename: &str) -> ModelPromptProfile {
        let profiles = self.model_prompt_profiles.read().await;
        if let Some(p) = profiles.get(model_filename) {
            return p.clone();
        }

        // 智能内置匹配规则
        let lower = model_filename.to_lowercase();
        if lower.contains("qwen") || lower.contains("tessera") {
            ModelPromptProfile {
                profile_name: "V4_超轻量极简直接抽取版".to_string(),
                f1_score: Some(0.912),
                custom_prompt: Self::PROMPT_V4_ULTRA_COMPACT.to_string(),
            }
        } else if lower.contains("lfm") {
            ModelPromptProfile {
                profile_name: "V1_默认结构加固版".to_string(),
                f1_score: Some(0.8571),
                custom_prompt: Self::PROMPT_V1_BASELINE.to_string(),
            }
        } else {
            ModelPromptProfile {
                profile_name: "V1_通用默认基准版".to_string(),
                f1_score: None,
                custom_prompt: Self::PROMPT_V1_BASELINE.to_string(),
            }
        }
    }

    /// 保存并绑定指定模型的最佳提示词档案
    pub async fn save_model_prompt_profile(&self, model_filename: String, profile: ModelPromptProfile) {
        let mut profiles = self.model_prompt_profiles.write().await;
        profiles.insert(model_filename, profile);
        drop(profiles);
        self.save_to_disk().await;
    }

    /// 获取所有自定义绑定的模型提示词档案映射表
    pub async fn get_all_model_prompt_profiles(&self) -> HashMap<String, ModelPromptProfile> {
        let profiles = self.model_prompt_profiles.read().await;
        profiles.clone()
    }

    /// 添加或更新文档
    pub async fn upsert_document(&self, filename: String, markdown: String) -> DocumentItem {
        let id = uuid::Uuid::new_v4().to_string();
        let char_count = markdown.chars().count();
        let doc = DocumentItem {
            id: id.clone(),
            filename,
            markdown,
            char_count,
            created_at: Utc::now(),
            snapshots: Vec::new(),
            active_snapshot_id: None,
        };

        {
            let mut map = self.documents.write().await;
            map.insert(id.clone(), doc.clone());
        }

        self.save_to_disk().await;
        doc
    }

    /// 查找同名文档或创建新文档 (若已存在则更新正文与时间并复用，方便 CLI 与界面历史连续追加快照)
    pub async fn find_or_create_document(&self, filename: String, markdown: String) -> DocumentItem {
        self.sync_from_disk_if_modified().await;
        let mut map = self.documents.write().await;
        let existing_id = map.values().find(|d| d.filename == filename).map(|d| d.id.clone());

        if let Some(id) = existing_id {
            if let Some(doc) = map.get_mut(&id) {
                doc.markdown = markdown.clone();
                doc.char_count = markdown.chars().count();
                let updated = doc.clone();
                drop(map);
                self.save_to_disk().await;
                return updated;
            }
        }

        let id = uuid::Uuid::new_v4().to_string();
        let char_count = markdown.chars().count();
        let doc = DocumentItem {
            id: id.clone(),
            filename,
            markdown,
            char_count,
            created_at: Utc::now(),
            snapshots: Vec::new(),
            active_snapshot_id: None,
        };

        map.insert(id, doc.clone());
        drop(map);
        self.save_to_disk().await;
        doc
    }

    /// 获取所有文档列表摘要（默认按创建/添加时间先后正序排列）
    pub async fn list_documents(&self) -> Vec<DocumentItem> {
        self.sync_from_disk_if_modified().await;
        let map = self.documents.read().await;
        let mut list: Vec<DocumentItem> = map.values().cloned().collect();
        list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        list
    }

    /// 获取单个文档详情
    pub async fn get_document(&self, doc_id: &str) -> Option<DocumentItem> {
        self.sync_from_disk_if_modified().await;
        let map = self.documents.read().await;
        map.get(doc_id).cloned()
    }

    /// 为文档追加一次提取快照记录
    pub async fn add_snapshot(
        &self,
        doc_id: &str,
        template_name: String,
        model_name: Option<String>,
        system_prompt: Option<String>,
        fields_used: Vec<RuleField>,
        items: Vec<SensitiveItem>,
        execution_ms: u64,
    ) -> Result<ExtractionSnapshot, String> {
        self.sync_from_disk_if_modified().await;
        let snapshot_id = uuid::Uuid::new_v4().to_string();
        let snapshot = ExtractionSnapshot {
            id: snapshot_id.clone(),
            timestamp: Utc::now(),
            template_name,
            model_name,
            system_prompt,
            fields_used,
            items,
            execution_ms,
        };

        let mut map = self.documents.write().await;
        if let Some(doc) = map.get_mut(doc_id) {
            doc.snapshots.push(snapshot.clone());
            doc.active_snapshot_id = Some(snapshot_id);
        } else {
            return Err("未找到指定文档".to_string());
        }

        drop(map);
        self.save_to_disk().await;
        Ok(snapshot)
    }

    /// 切换文档查看的历史快照
    pub async fn set_active_snapshot(&self, doc_id: &str, snapshot_id: &str) -> Result<(), String> {
        self.sync_from_disk_if_modified().await;
        let mut map = self.documents.write().await;
        if let Some(doc) = map.get_mut(doc_id) {
            if doc.snapshots.iter().any(|s| s.id == snapshot_id) {
                doc.active_snapshot_id = Some(snapshot_id.to_string());
                return Ok(());
            }
        }
        Err("快照不存在".to_string())
    }

    /// 删除指定文档
    pub async fn delete_document(&self, doc_id: &str) -> bool {
        self.sync_from_disk_if_modified().await;
        let mut map = self.documents.write().await;
        let removed = map.remove(doc_id).is_some();
        drop(map);
        if removed {
            self.save_to_disk().await;
        }
        removed
    }

    /// 删除指定文档的某个历史快照
    pub async fn delete_snapshot(&self, doc_id: &str, snapshot_id: &str) -> Result<Option<String>, String> {
        self.sync_from_disk_if_modified().await;
        let mut map = self.documents.write().await;
        if let Some(doc) = map.get_mut(doc_id) {
            let original_len = doc.snapshots.len();
            doc.snapshots.retain(|s| s.id != snapshot_id);
            if doc.snapshots.len() == original_len {
                return Err("快照不存在".to_string());
            }

            if doc.active_snapshot_id.as_deref() == Some(snapshot_id) {
                doc.active_snapshot_id = doc.snapshots.last().map(|s| s.id.clone());
            }
            let active_id = doc.active_snapshot_id.clone();
            drop(map);
            self.save_to_disk().await;
            Ok(active_id)
        } else {
            Err("未找到指定文档".to_string())
        }
    }

    /// 获取所有已保存的在线模型配置
    pub async fn get_online_models(&self) -> Vec<OnlineModelProfile> {
        self.sync_from_disk_if_modified().await;
        self.online_models.read().await.clone()
    }

    /// 获取当前激活选中的在线模型配置 ID
    pub async fn get_active_online_model_id(&self) -> Option<String> {
        self.sync_from_disk_if_modified().await;
        self.active_online_model_id.read().await.clone()
    }

    /// 设置当前激活的在线模型配置 ID
    pub async fn set_active_online_model_id(&self, id: Option<String>) {
        let mut active = self.active_online_model_id.write().await;
        *active = id;
        drop(active);
        self.save_to_disk().await;
    }

    /// 根据 ID 获取单个在线模型配置
    pub async fn get_online_model_by_id(&self, id: &str) -> Option<OnlineModelProfile> {
        self.sync_from_disk_if_modified().await;
        let list = self.online_models.read().await;
        list.iter().find(|m| m.id == id).cloned()
    }

    /// 保存或更新在线模型配置
    pub async fn save_online_model(&self, mut profile: OnlineModelProfile) -> OnlineModelProfile {
        if profile.id.trim().is_empty() {
            profile.id = uuid::Uuid::new_v4().to_string();
        }

        let mut list = self.online_models.write().await;
        if let Some(existing) = list.iter_mut().find(|m| m.id == profile.id) {
            *existing = profile.clone();
        } else {
            list.push(profile.clone());
        }

        let mut active = self.active_online_model_id.write().await;
        if active.is_none() {
            *active = Some(profile.id.clone());
        }
        drop(active);
        drop(list);

        self.save_to_disk().await;
        profile
    }

    /// 删除指定在线模型配置
    pub async fn delete_online_model(&self, id: &str) -> Result<Option<String>, String> {
        let mut list = self.online_models.write().await;
        if list.len() <= 1 {
            return Err("至少需要保留一个在线模型配置".to_string());
        }

        let orig_len = list.len();
        list.retain(|m| m.id != id);
        if list.len() == orig_len {
            return Err("未找到指定的模型配置".to_string());
        }

        let mut active = self.active_online_model_id.write().await;
        if active.as_deref() == Some(id) {
            *active = list.first().map(|m| m.id.clone());
        }
        let current_active = active.clone();
        drop(active);
        drop(list);

        self.save_to_disk().await;
        Ok(current_active)
    }

    /// 获取所有离线模型的超参数配置字典
    pub async fn get_all_offline_model_profiles(&self) -> HashMap<String, OfflineModelProfile> {
        self.sync_from_disk_if_modified().await;
        self.offline_model_profiles.read().await.clone()
    }

    /// 获取单个离线模型的超参数配置（未配置则返回默认 0.1 / 50 / 1.1）
    pub async fn get_offline_model_profile(&self, filename: &str) -> OfflineModelProfile {
        self.sync_from_disk_if_modified().await;
        let profiles = self.offline_model_profiles.read().await;
        profiles.get(filename).cloned().unwrap_or_default()
    }

    /// 保存指定离线模型的超参数配置并落盘
    pub async fn save_offline_model_profile(&self, filename: String, profile: OfflineModelProfile) {
        let mut profiles = self.offline_model_profiles.write().await;
        profiles.insert(filename, profile);
        drop(profiles);
        self.save_to_disk().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_and_snapshot_lifecycle() {
        let test_store = std::env::temp_dir().join(format!("sensidoc_test_lifecycle_{}.json", uuid::Uuid::new_v4()));
        let mgr = SessionManager::with_store_path(test_store.clone());

        // 1. 测试文档创建与删除
        let doc = mgr.upsert_document("test.txt".into(), "测试内容 12345".into()).await;
        assert_eq!(doc.filename, "test.txt");
        assert!(mgr.get_document(&doc.id).await.is_some());

        // 2. 测试快照添加与删除
        let snap1 = mgr
            .add_snapshot(&doc.id, "模板A".into(), None, Some("系统提示词A".into()), vec![], vec![], 50)
            .await
            .unwrap();
        let snap2 = mgr
            .add_snapshot(&doc.id, "模板B".into(), None, Some("系统提示词B".into()), vec![], vec![], 60)
            .await
            .unwrap();

        let doc_with_snaps = mgr.get_document(&doc.id).await.unwrap();
        assert_eq!(doc_with_snaps.snapshots.len(), 2);
        assert_eq!(doc_with_snaps.active_snapshot_id, Some(snap2.id.clone()));

        // 删除当前激活的 snap2，active_snapshot_id 应回退至 snap1
        let active_after_del = mgr.delete_snapshot(&doc.id, &snap2.id).await.unwrap();
        assert_eq!(active_after_del, Some(snap1.id.clone()));

        let doc_after_del1 = mgr.get_document(&doc.id).await.unwrap();
        assert_eq!(doc_after_del1.snapshots.len(), 1);
        assert_eq!(doc_after_del1.active_snapshot_id, Some(snap1.id.clone()));

        // 删除最后一个 snap1，active_snapshot_id 应为 None
        let active_after_del2 = mgr.delete_snapshot(&doc.id, &snap1.id).await.unwrap();
        assert_eq!(active_after_del2, None);

        let doc_after_del2 = mgr.get_document(&doc.id).await.unwrap();
        assert_eq!(doc_after_del2.snapshots.len(), 0);
        assert_eq!(doc_after_del2.active_snapshot_id, None);

        // 3. 删除文档
        assert!(mgr.delete_document(&doc.id).await);
        assert!(mgr.get_document(&doc.id).await.is_none());
        assert!(!mgr.delete_document(&doc.id).await);

        let _ = std::fs::remove_file(test_store);
    }

    #[tokio::test]
    async fn test_cross_instance_disk_sync() {
        let test_store = std::env::temp_dir().join(format!("sensidoc_test_sync_{}.json", uuid::Uuid::new_v4()));
        let mgr_server = SessionManager::with_store_path(test_store.clone());
        let mgr_cli = SessionManager::with_store_path(test_store.clone());

        let unique_filename = format!("cli_test_{}.txt", uuid::Uuid::new_v4());
        let doc = mgr_cli
            .find_or_create_document(unique_filename.clone(), "跨进程同步测试内容".into())
            .await;

        let snap = mgr_cli
            .add_snapshot(
                &doc.id,
                "[CLI] 通用模板".into(),
                None,
                None,
                vec![],
                vec![],
                12,
            )
            .await
            .unwrap();

        // 验证常驻的 mgr_server 在调用 list_documents 时自动通过 mtime 检测拉取最新记录
        let server_docs = mgr_server.list_documents().await;
        let found = server_docs.iter().find(|d| d.id == doc.id);
        assert!(found.is_some(), "服务端实例未能感知到 CLI 实例新写入的文档");

        let found_doc = mgr_server.get_document(&doc.id).await.unwrap();
        assert_eq!(found_doc.filename, unique_filename);
        assert_eq!(found_doc.snapshots.len(), 1);
        assert_eq!(found_doc.snapshots[0].id, snap.id);

        // 清理测试数据
        let _ = mgr_server.delete_document(&doc.id).await;
        let _ = std::fs::remove_file(test_store);
    }
}

