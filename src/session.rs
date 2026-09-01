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

/// 在线 AI 模型 (OpenAI 兼容 API) 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineAiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model_id")]
    pub model_id: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_base_url() -> String {
    "https://api.deepseek.com/v1".to_string()
}

fn default_model_id() -> String {
    "deepseek-chat".to_string()
}

fn default_temperature() -> f32 {
    0.3
}

impl Default for OnlineAiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_base_url(),
            api_key: String::new(),
            model_id: default_model_id(),
            temperature: default_temperature(),
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

/// 工作区持久化数据结构（包括文档、自定义模板与标签库、模型专属提示词、在线 AI 配置）
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
    online_ai_config: OnlineAiConfig,
}

/// 全局文档会话管理器
pub struct SessionManager {
    store_path: PathBuf,
    documents: Arc<RwLock<HashMap<String, DocumentItem>>>,
    custom_templates: Arc<RwLock<Vec<RulePreset>>>,
    field_tags: Arc<RwLock<Vec<RuleField>>>,
    model_prompt_profiles: Arc<RwLock<HashMap<String, ModelPromptProfile>>>,
    online_ai_config: Arc<RwLock<OnlineAiConfig>>,
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
        let store_path = PathBuf::from(".sensidoc_workspace.json");
        let (docs, templates, tags, profiles, online_ai) = Self::load_from_disk(&store_path);

        Self {
            store_path,
            documents: Arc::new(RwLock::new(docs)),
            custom_templates: Arc::new(RwLock::new(templates)),
            field_tags: Arc::new(RwLock::new(tags)),
            model_prompt_profiles: Arc::new(RwLock::new(profiles)),
            online_ai_config: Arc::new(RwLock::new(online_ai)),
        }
    }

    /// 从本地 JSON 读取落盘数据
    fn load_from_disk(path: &Path) -> (HashMap<String, DocumentItem>, Vec<RulePreset>, Vec<RuleField>, HashMap<String, ModelPromptProfile>, OnlineAiConfig) {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(store) = serde_json::from_str::<WorkspaceStore>(&content) {
                    info!(
                        "已从 {} 恢复 {} 个持久化文档记录, {} 个自定义模板, {} 个字段标签, {} 个模型提示词档案, 在线AI启用={}",
                        path.display(),
                        store.documents.len(),
                        store.custom_templates.len(),
                        store.field_tags.len(),
                        store.model_prompt_profiles.len(),
                        store.online_ai_config.enabled
                    );
                    let docs_map = store.documents.into_iter().map(|d| (d.id.clone(), d)).collect();
                    return (docs_map, store.custom_templates, store.field_tags, store.model_prompt_profiles, store.online_ai_config);
                }
            }
        }
        (HashMap::new(), Vec::new(), Vec::new(), HashMap::new(), OnlineAiConfig::default())
    }

    /// 异步刷盘持久化
    pub async fn save_to_disk(&self) {
        let map = self.documents.read().await;
        let templates = self.custom_templates.read().await;
        let tags = self.field_tags.read().await;
        let profiles = self.model_prompt_profiles.read().await;
        let online_ai = self.online_ai_config.read().await;

        let store = WorkspaceStore {
            documents: map.values().cloned().collect(),
            custom_templates: templates.clone(),
            field_tags: tags.clone(),
            model_prompt_profiles: profiles.clone(),
            online_ai_config: online_ai.clone(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&store) {
            let _ = tokio::fs::write(&self.store_path, json).await;
        }
    }
    /// 添加或保存自定义场景模板
    pub async fn save_template(&self, template: RulePreset) {
        let mut list = self.custom_templates.write().await;
        if let Some(existing) = list.iter_mut().find(|t| t.id == template.id) {
            *existing = template;
        } else {
            list.push(template);
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
        let list = self.custom_templates.read().await;
        list.clone()
    }

    /// 保存/新增字段标签到公共标签库
    pub async fn save_field_tag(&self, tag: RuleField) {
        let mut list = self.field_tags.write().await;
        if let Some(existing) = list.iter_mut().find(|t| t.name == tag.name) {
            *existing = tag;
        } else {
            list.push(tag);
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
        if lower.contains("qwen") {
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

    /// 获取当前在线 AI 模型配置
    pub async fn get_online_ai_config(&self) -> OnlineAiConfig {
        let cfg = self.online_ai_config.read().await;
        cfg.clone()
    }

    /// 保存并更新在线 AI 模型配置
    pub async fn save_online_ai_config(&self, cfg: OnlineAiConfig) {
        let mut target = self.online_ai_config.write().await;
        *target = cfg;
        drop(target);
        self.save_to_disk().await;
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

    /// 获取所有文档列表摘要（默认按创建/添加时间先后正序排列）
    pub async fn list_documents(&self) -> Vec<DocumentItem> {
        let map = self.documents.read().await;
        let mut list: Vec<DocumentItem> = map.values().cloned().collect();
        list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        list
    }

    /// 获取单个文档详情
    pub async fn get_document(&self, doc_id: &str) -> Option<DocumentItem> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_and_snapshot_lifecycle() {
        let mgr = SessionManager::new();

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
    }
}
