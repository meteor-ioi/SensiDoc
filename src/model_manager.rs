use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, Mutex};
use tracing::{info, warn};

/// 预设的模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreset {
    pub id: String,
    pub name: String,
    pub filename: String,
    #[serde(default)]
    pub mmproj_filename: Option<String>,
    pub modelscope_id: String,
    pub description: String,
    pub size_desc: String,
    pub is_downloaded: bool,
    #[serde(default)]
    pub is_downloading: bool,
    pub is_active: bool,
    #[serde(default)]
    pub is_main_downloaded: bool,
    #[serde(default)]
    pub is_mmproj_downloaded: bool,
}

/// 下载进度事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub status: String, // "downloading", "completed", "failed", "canceled"
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f64,
    pub speed_mb: f64,
    pub error: Option<String>,
}

/// 模型管理器核心结构体
pub struct ModelManager {
    models_dir: PathBuf,
    llama_bin_path: PathBuf,
    active_child: Arc<Mutex<Option<Child>>>,
    active_model: Arc<Mutex<Option<String>>>,
    server_port: u16,
    progress_tx: broadcast::Sender<DownloadProgress>,
    download_cancellations: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
}

pub const DEFAULT_LLAMA_SERVER_PORT: u16 = 18188;

pub const OCR_BUNDLE_ID: &str = "ocr-ppocrv6-bundle";

pub struct OcrDownloadFileSpec {
    pub filename: &'static str,
    pub download_url: &'static str,
    pub expected_size: u64,
    pub sha256: &'static str,
}

pub const OCR_DOWNLOAD_SPECS: [OcrDownloadFileSpec; 4] = [
    OcrDownloadFileSpec {
        filename: crate::paths::OCR_DET_FILENAME,
        download_url: "https://modelscope.cn/models/RapidAI/RapidOCR/resolve/7d0781614ca1a83d5ad9603f713acb2e74855d72/onnx/PP-OCRv6/det/PP-OCRv6_det_small.onnx",
        expected_size: 9_929_594,
        sha256: "090f04abcd9d9a7498bc4ebf677e4cb9bdce1fe4197ddb7e529f1ef44e1ff94f",
    },
    OcrDownloadFileSpec {
        filename: crate::paths::OCR_REC_FILENAME,
        download_url: "https://modelscope.cn/models/RapidAI/RapidOCR/resolve/7d0781614ca1a83d5ad9603f713acb2e74855d72/onnx/PP-OCRv6/rec/PP-OCRv6_rec_medium.onnx",
        expected_size: 76_629_984,
        sha256: "eef444829dbbe18d7fea59a3f6eb75647518d2b3a9568d27c92e42940204894b",
    },
    OcrDownloadFileSpec {
        filename: crate::paths::OCR_TABLE_FILENAME,
        download_url: "https://modelscope.cn/models/RapidAI/RapidTable/resolve/a484f11b64162cc443ccf14582996ca35be33030/slanet-plus.onnx",
        expected_size: 7_758_305,
        sha256: "d57a942af6a2f57d6a4a0372573c696a2379bf5857c45e2ac69993f3b334514b",
    },
    OcrDownloadFileSpec {
        filename: crate::paths::OCR_DICT_FILENAME,
        download_url: "https://modelscope.cn/models/RapidAI/RapidOCR/resolve/7d0781614ca1a83d5ad9603f713acb2e74855d72/paddle/PP-OCRv6/rec/PP-OCRv6_rec_medium/ppocrv6_dict.txt",
        expected_size: 74_947,
        sha256: "b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d",
    },
];

pub fn get_ocr_total_expected_bytes() -> u64 {
    OCR_DOWNLOAD_SPECS.iter().map(|s| s.expected_size).sum()
}

/// 校验本地指定路径文件的 SHA-256 哈希值是否匹配目标预期 (不区分大小写)
pub fn verify_file_sha256(path: &std::path::Path, expected_sha256: &str) -> Result<bool, std::io::Error> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let actual_hex = format!("{:x}", hasher.finalize());
    Ok(actual_hex.eq_ignore_ascii_case(expected_sha256))
}

impl ModelManager {
    pub fn new() -> Self {
        let (progress_tx, _) = broadcast::channel(100);
        let models_dir = crate::paths::get_models_dir();
        let llama_bin_path = crate::paths::get_llama_bin_path();

        if !models_dir.exists() {
            let _ = std::fs::create_dir_all(&models_dir);
        }

        Self {
            models_dir,
            llama_bin_path,
            active_child: Arc::new(Mutex::new(None)),
            active_model: Arc::new(Mutex::new(None)),
            server_port: DEFAULT_LLAMA_SERVER_PORT,
            progress_tx,
            download_cancellations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn server_port(&self) -> u16 {
        self.server_port
    }

    /// 快速探测底层 llama-server 是否已就绪（超时 600ms）
    pub async fn is_server_ready(&self) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(600))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };
        let health_url = format!("http://127.0.0.1:{}/health", self.server_port);
        if let Ok(resp) = client.get(&health_url).send().await {
            return resp.status().is_success();
        }
        false
    }

    pub fn get_progress_receiver(&self) -> broadcast::Receiver<DownloadProgress> {
        self.progress_tx.subscribe()
    }

    /// 获取内置的魔搭模型预设列表及其当前本地状态
    pub async fn get_presets(&self) -> Vec<ModelPreset> {
        let active = self.get_active_model().await;
        let downloading_ids: std::collections::HashSet<String> = {
            let map = self.download_cancellations.lock().await;
            map.keys().cloned().collect()
        };

        // 预设离线模型：快前置与视觉VLM (Qwen3.5-0.8B-Q4_K_M + 视觉塔) + 慢终审 (MiniCPM5-2B-Q4_K_M)
        let presets = vec![
            ModelPreset {
                id: "qwen3.5-0.8b-q4_k_m".to_string(),
                name: "Qwen3.5-0.8B (Q4_K_M + 视觉塔)".to_string(),
                filename: "Qwen3.5-0.8B-Q4_K_M.gguf".to_string(),
                mmproj_filename: Some("mmproj-BF16.gguf".to_string()),
                modelscope_id: "unsloth/Qwen3.5-0.8B-GGUF".to_string(),
                description: "端侧高保真多模态视觉小模型 (含 BF16 视觉塔)，支持原图端到端视觉重构与微切片纠偏".to_string(),
                size_desc: "~720 MB (含视觉塔)".to_string(),
                is_downloaded: false,
                is_downloading: false,
                is_active: false,
                is_main_downloaded: false,
                is_mmproj_downloaded: false,
            },
            ModelPreset {
                id: "minicpm5-2b-q4_k_m".to_string(),
                name: "MiniCPM5-2B (Q4_K_M)".to_string(),
                filename: "MiniCPM5-2B-Q4_K_M.gguf".to_string(),
                mmproj_filename: None,
                modelscope_id: "OpenBMB/MiniCPM5-2B-gguf".to_string(),
                description: "面壁智能 MiniCPM 2B 旗舰端侧小模型，终审精确率 100% 完美平替 4B".to_string(),
                size_desc: "~1.5 GB".to_string(),
                is_downloaded: false,
                is_downloading: false,
                is_active: false,
                is_main_downloaded: false,
                is_mmproj_downloaded: false,
            },
        ];

        presets
            .into_iter()
            .map(|mut p| {
                let path = self.models_dir.join(&p.filename);
                let main_ok = path.exists() && std::fs::metadata(&path).map(|m| m.len() > 1024 * 1024).unwrap_or(false);
                let mm_ok = if let Some(ref mm) = p.mmproj_filename {
                    let mm_path = self.models_dir.join(mm);
                    mm_path.exists() && std::fs::metadata(&mm_path).map(|m| m.len() > 1024 * 1024).unwrap_or(false)
                } else {
                    true
                };
                p.is_main_downloaded = main_ok;
                p.is_mmproj_downloaded = mm_ok;
                p.is_downloaded = main_ok && mm_ok;
                p.is_downloading = downloading_ids.contains(&p.id);
                p.is_active = active.as_deref() == Some(&p.filename);
                p
            })
            .collect()
    }

    /// 扫描 models/ 目录下的所有可用 gguf 模型文件 (过滤 mmproj 投影权重，避免作为独立语言模型暴露)
    pub fn list_local_models(&self) -> Vec<String> {
        let mut list = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.models_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.ends_with(".gguf") && !name.starts_with("mmproj") && !name.contains("-mmproj") && !name.contains(".mmproj") {
                            list.push(name);
                        }
                    }
                }
            }
        }
        list
    }

    /// 列出所有本地 mmproj 视觉塔权重文件
    pub fn list_local_mmproj(&self) -> Vec<String> {
        let mut list = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.models_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.ends_with(".gguf")
                            && (name.starts_with("mmproj")
                                || name.contains("-mmproj")
                                || name.contains(".mmproj"))
                        {
                            list.push(name);
                        }
                    }
                }
            }
        }
        list
    }

    /// 检查指定模型文件是否具有可用的多模态视觉塔投影 (mmproj)
    pub fn has_mmproj_for(&self, model_filename: &str) -> bool {
        if model_filename == "Qwen3.5-0.8B-Q4_K_M.gguf" {
            let p = self.models_dir.join("mmproj-BF16.gguf");
            return p.exists() && std::fs::metadata(&p).map(|m| m.len() > 1024 * 1024).unwrap_or(false);
        }
        let candidate_names = [
            format!("{}.mmproj.gguf", model_filename.trim_end_matches(".gguf")),
            format!("mmproj-{}.gguf", model_filename.trim_end_matches(".gguf")),
        ];
        for name in &candidate_names {
            let p = self.models_dir.join(name);
            if p.exists() && std::fs::metadata(&p).map(|m| m.len() > 1024 * 1024).unwrap_or(false) {
                return true;
            }
        }
        false
    }

    /// 获取当前运行中的模型名称（主动探测专属端口上的 llama-server 真实运行状态并自动同步）
    pub async fn get_active_model(&self) -> Option<String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build()
            .unwrap_or_default();

        let url = format!("http://127.0.0.1:{}/v1/models", self.server_port);
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(json_body) = resp.json::<serde_json::Value>().await {
                    let detected_name = json_body["data"][0]["id"]
                        .as_str()
                        .or_else(|| json_body["models"][0]["name"].as_str())
                        .or_else(|| json_body["models"][0]["model"].as_str());

                    if let Some(raw_path) = detected_name {
                        let filename = std::path::Path::new(raw_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(raw_path)
                            .to_string();

                        let mut guard = self.active_model.lock().await;
                        *guard = Some(filename.clone());
                        return Some(filename);
                    }
                }
            }
        }

        // 若探测专属端口不可达，清空状态并返回 None
        let mut guard = self.active_model.lock().await;
        *guard = None;
        None
    }

    /// 唤起操作系统原生文件选择对话框选取 .gguf 模型文件
    pub async fn pick_file_dialog(&self) -> Result<Option<String>, String> {
        #[cfg(target_os = "macos")]
        {
            let script = r#"try
  set selectedFile to choose file with prompt "请选择本地 GGUF 模型文件 (.gguf)" of type {"gguf", "public.data"}
  return POSIX path of selectedFile
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
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if path_str.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(path_str))
                }
            } else {
                // 降级无 type 过滤唤起
                let script_fallback = r#"try
  set selectedFile to choose file with prompt "请选择本地 GGUF 模型文件 (.gguf)"
  return POSIX path of selectedFile
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
                    let path_str = String::from_utf8_lossy(&fb_output.stdout).trim().to_string();
                    if path_str.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(path_str))
                    }
                } else {
                    Ok(None)
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let script = r#"[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Filter = 'GGUF 模型文件 (*.gguf)|*.gguf|所有文件 (*.*)|*.*'; $f.Title = '请选择本地 GGUF 模型文件'; if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $f.FileName }"#;
            let output = tokio::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", script])
                .output()
                .await
                .map_err(|e| format!("无法唤起 Windows 文件选择器: {e}"))?;

            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if path.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(path))
                }
            } else {
                Ok(None)
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            if let Ok(output) = tokio::process::Command::new("zenity")
                .args(["--file-selection", "--title=请选择本地 GGUF 模型文件", "--file-filter=GGUF 模型 (*.gguf) | *.gguf"])
                .output()
                .await
            {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Ok(Some(path));
                    }
                }
                return Ok(None);
            }
            if let Ok(output) = tokio::process::Command::new("kdialog")
                .args(["--getopenfilename", ".", "*.gguf"])
                .output()
                .await
            {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Ok(Some(path));
                    }
                }
                return Ok(None);
            }
            Err("未检测到可用的系统文件选择器 (zenity 或 kdialog)".into())
        }
    }

    /// 导入外部本地 GGUF 模型文件（支持软链接或复制到 models/ 目录）
    pub async fn import_external_model(&self, source_path: &str) -> Result<String, String> {
        let src = std::path::Path::new(source_path);
        if !src.exists() {
            return Err("指定的模型文件不存在".into());
        }

        let filename = src
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "无效的文件名".to_string())?;

        if !filename.to_lowercase().ends_with(".gguf") {
            return Err("仅支持 .gguf 格式模型文件".into());
        }

        let dest = self.models_dir.join(filename);
        if !dest.exists() {
            // 优先创建符号链接，极速且节省磁盘空间
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(src, &dest)
                    .map_err(|e| format!("链接模型文件失败: {e}"))?;
            }
            #[cfg(not(unix))]
            {
                tokio::fs::copy(src, &dest)
                    .await
                    .map_err(|e| format!("复制模型文件失败: {e}"))?;
            }
        }

        Ok(filename.to_string())
    }

    /// 停止当前正在运行的 llama-server 子进程，彻底释放专属端口与显存
    pub async fn stop_server(&self) {
        let mut child_guard = self.active_child.lock().await;
        if let Some(mut child) = child_guard.take() {
            info!("正在终止当前 llama-server 进程 (PID: {:?})...", child.id());
            let _ = child.kill().await;
            let _ = child.wait().await;
            info!("SensiDoc llama-server 进程已终止释放。");
        }

        // 仅精准清理占用 SensiDoc 专属端口的孤儿残留，绝不误杀系统其他 llama-server
        #[cfg(unix)]
        {
            let port_str = self.server_port.to_string();
            if let Ok(output) = tokio::process::Command::new("lsof")
                .arg("-ti")
                .arg(format!(":{}", port_str))
                .output()
                .await
            {
                if output.status.success() {
                    let pids = String::from_utf8_lossy(&output.stdout);
                    for pid in pids.lines() {
                        if let Ok(pid_num) = pid.trim().parse::<u32>() {
                            info!("精准清理占用专属端口 {} 的孤儿进程 PID: {}", port_str, pid_num);
                            let _ = tokio::process::Command::new("kill")
                                .arg("-9")
                                .arg(pid_num.to_string())
                                .output()
                                .await;
                        }
                    }
                }
            }
        }

        let mut model_guard = self.active_model.lock().await;
        *model_guard = None;
    }

    /// 启动指定的本地模型（杜绝僵尸孤儿进程）
    pub async fn start_model(&self, model_filename: &str) -> Result<(), String> {
        self.start_model_with_profile(model_filename, None).await
    }

    /// 支持传入模型专属超参数（温度、采样、思考模式开闭）启动本地 llama-server
    pub async fn start_model_with_profile(
        &self,
        model_filename: &str,
        profile: Option<&crate::session::OfflineModelProfile>,
    ) -> Result<(), String> {
        let model_path = self.models_dir.join(model_filename);
        if !model_path.exists() {
            return Err(format!("模型文件不存在: {}", model_path.display()));
        }

        // 先优雅停止已有进程
        self.stop_server().await;

        // 确保 llama_bin 可执行
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&self.llama_bin_path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&self.llama_bin_path, perms);
            }
        }

        let enable_thinking = profile.map(|p| p.enable_thinking).unwrap_or(false);
        let top_k_str = profile.map(|p| p.top_k.to_string()).unwrap_or_else(|| "50".to_string());
        let repeat_penalty_str = profile.map(|p| p.repeat_penalty.to_string()).unwrap_or_else(|| "1.1".to_string());

        info!(
            "启动 llama-server: 路径={:?}, 模型={:?}, 端口={}, 思考模式={}, top_k={}, repeat_penalty={}",
            self.llama_bin_path,
            model_path,
            self.server_port,
            if enable_thinking { "开启" } else { "关闭" },
            top_k_str,
            repeat_penalty_str
        );

        let mut cmd = Command::new(&self.llama_bin_path);
        cmd.arg("-m")
            .arg(&model_path)
            .arg("--port")
            .arg(self.server_port.to_string())
            .arg("-ngl")
            .arg("99") // 开启 macOS Apple Silicon Metal GPU 全量卸载加速
            .arg("-c")
            .arg("8192") // 针对视觉 VLM 扩展上下文容量 (包含图片视觉 tokens 及丰富 Markdown 表格输出)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--top-k")
            .arg(&top_k_str)
            .arg("--repeat-penalty")
            .arg(&repeat_penalty_str);

        // 自动探测或按配置挂载多模态视觉塔 (mmproj)
        let explicit_mm = profile.and_then(|p| p.mmproj.as_deref());
        let mm_target: Option<std::path::PathBuf> = if let Some(m) = explicit_mm {
            if m == "none" || m.is_empty() {
                None
            } else {
                let p = self.models_dir.join(m);
                if p.exists() { Some(p) } else { None }
            }
        } else if model_filename == "Qwen3.5-0.8B-Q4_K_M.gguf" {
            let p = self.models_dir.join("mmproj-BF16.gguf");
            if p.exists() { Some(p) } else { None }
        } else {
            let candidate_mmprojs = [
                self.models_dir.join(format!("{}.mmproj.gguf", model_filename.trim_end_matches(".gguf"))),
                self.models_dir.join(format!("mmproj-{}.gguf", model_filename.trim_end_matches(".gguf"))),
            ];
            candidate_mmprojs.into_iter().find(|p| p.exists() && std::fs::metadata(p).map(|m| m.len() > 1024 * 1024).unwrap_or(false))
        };

        if let Some(mm_path) = mm_target {
            info!("检测到多模态视觉塔权重，已自动挂载 --mmproj: {:?}", mm_path);
            cmd.arg("--mmproj").arg(mm_path);
            cmd.arg("--image-min-tokens").arg("1024");
        }

        // 严格遵循设置面板思考模式：未开启时显式设置 --reasoning off，彻底杜绝模型擅自激活思维链耗尽 tokens
        if enable_thinking {
            cmd.arg("--reasoning").arg("on");
        } else {
            cmd.arg("--reasoning").arg("off");
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // macOS 动态链接库环境变量绑定
        let lib_dir = crate::paths::get_lib_dir();
        if lib_dir.exists() {
            cmd.env("DYLD_LIBRARY_PATH", &lib_dir);
        }

        // Windows 平台静默启动（抑制 CMD 黑色控制台黑框弹出）与同级动态库 PATH 环境变量注入
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);

            if let Some(bin_parent) = self.llama_bin_path.parent() {
                let current_path = std::env::var("PATH").unwrap_or_default();
                cmd.env("PATH", format!("{};{}", bin_parent.display(), current_path));
            }
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("拉起 llama-server 子进程失败: {e}"))?;


        {
            let mut guard = self.active_child.lock().await;
            *guard = Some(child);
        }
        {
            let mut guard = self.active_model.lock().await;
            *guard = Some(model_filename.to_string());
        }

        // 健康检查探测服务端口就绪
        self.wait_for_server_ready().await?;
        info!("llama-server 已成功启动并在端口 {} 就绪！", self.server_port);

        Ok(())
    }

    /// 等待服务探测就绪（最多 25 秒重试）
    async fn wait_for_server_ready(&self) -> Result<(), String> {
        let client = reqwest::Client::new();
        let health_url = format!("http://127.0.0.1:{}/health", self.server_port);

        for _ in 0..50 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Ok(resp) = client.get(&health_url).send().await {
                if resp.status().is_success() {
                    return Ok(());
                }
            }
        }
        Err("llama-server 启动超时或健康检查未通过".to_string())
    }

    /// 从魔搭（ModelScope）下载预设模型，支持多文件（主模型+视觉塔）两阶段下载、断点续传与实时进度广播
    pub async fn download_model(&self, model_id: &str) -> Result<(), String> {
        let presets = self.get_presets().await;
        let target_preset = presets
            .iter()
            .find(|p| p.id == model_id)
            .ok_or_else(|| format!("未知的预设模型 ID: {model_id}"))?;

        let modelscope_repo = target_preset.modelscope_id.clone();
        let mut download_files = vec![target_preset.filename.clone()];
        if let Some(ref mm) = target_preset.mmproj_filename {
            download_files.push(mm.clone());
        }

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut map = self.download_cancellations.lock().await;
            map.insert(model_id.to_string(), cancel_tx);
        }

        let total_files = download_files.len();
        let mut overall_downloaded_bytes = 0u64;
        let mut is_canceled = false;

        for (file_idx, filename) in download_files.iter().enumerate() {
            let target_path = self.models_dir.join(filename);
            let part_path = self.models_dir.join(format!("{filename}.part"));

            // 如果该文件已存在且大小正常（>1MB），跳过下载
            if target_path.exists() {
                if let Ok(meta) = std::fs::metadata(&target_path) {
                    if meta.len() > 1024 * 1024 {
                        overall_downloaded_bytes += meta.len();
                        continue;
                    }
                }
            }

            let download_url = format!(
                "https://modelscope.cn/models/{modelscope_repo}/resolve/master/{filename}"
            );

            // 1. 获取现有 .part 文件已下载的字节数
            let mut downloaded: u64 = if part_path.exists() {
                std::fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };

            // 2. 发送 Range 请求获取剩余数据流
            let mut req = client.get(&download_url);
            if downloaded > 0 {
                req = req.header("Range", format!("bytes={}-", downloaded));
            }

            let resp = tokio::select! {
                res = req.send() => {
                    match res {
                        Ok(r) => r,
                        Err(e) => {
                            let mut map = self.download_cancellations.lock().await;
                            map.remove(model_id);
                            return Err(format!("连接魔搭下载源失败: {e}"));
                        }
                    }
                }
                _ = &mut cancel_rx => {
                    is_canceled = true;
                    if part_path.exists() {
                        let _ = tokio::fs::remove_file(&part_path).await;
                    }
                    break;
                }
            };

            let file_total_size = if let Some(cr) = resp.headers().get("content-range").and_then(|h| h.to_str().ok()) {
                if let Some(slash_idx) = cr.rfind('/') {
                    cr[slash_idx + 1..].parse::<u64>().unwrap_or_else(|_| resp.content_length().unwrap_or(0) + downloaded)
                } else {
                    resp.content_length().unwrap_or(0) + downloaded
                }
            } else {
                match resp.content_length() {
                    Some(len) => len + downloaded,
                    None => downloaded + 500 * 1024 * 1024,
                }
            };

            let mut stream = resp.bytes_stream();
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&part_path)
                .await
                .map_err(|e| {
                    let mgr_cancels = self.download_cancellations.clone();
                    let m_id = model_id.to_string();
                    tokio::spawn(async move {
                        let mut map = mgr_cancels.lock().await;
                        map.remove(&m_id);
                    });
                    format!("打开临时模型文件失败: {e}")
                })?;

            let mut last_broadcast = std::time::Instant::now();
            let mut speed_calc_time = std::time::Instant::now();
            let mut speed_downloaded = 0u64;
            let mut current_speed = 0.0f64;

            loop {
                tokio::select! {
                    _ = &mut cancel_rx => {
                        is_canceled = true;
                        break;
                    }
                    chunk_opt = stream.next() => {
                        match chunk_opt {
                            Some(chunk_result) => {
                                let chunk = match chunk_result {
                                    Ok(c) => c,
                                    Err(e) => {
                                        let mut map = self.download_cancellations.lock().await;
                                        map.remove(model_id);
                                        return Err(format!("下载数据分块失败: {e}"));
                                    }
                                };
                                if let Err(e) = file.write_all(&chunk).await {
                                    let mut map = self.download_cancellations.lock().await;
                                    map.remove(model_id);
                                    return Err(format!("写入模型文件失败: {e}"));
                                }
                                downloaded += chunk.len() as u64;
                                speed_downloaded += chunk.len() as u64;

                                if speed_calc_time.elapsed() >= std::time::Duration::from_millis(500) {
                                    let elapsed_secs = speed_calc_time.elapsed().as_secs_f64();
                                    current_speed = (speed_downloaded as f64 / (1024.0 * 1024.0)) / elapsed_secs;
                                    speed_downloaded = 0;
                                    speed_calc_time = std::time::Instant::now();
                                }

                                if last_broadcast.elapsed() >= std::time::Duration::from_millis(150) || downloaded >= file_total_size {
                                    // 综合加权进度：单文件进度与总文件加权
                                    let file_pct = if file_total_size > 0 {
                                        (downloaded as f64 / file_total_size as f64 * 100.0).min(100.0)
                                    } else {
                                        0.0
                                    };
                                    let total_pct = ((file_idx as f64 + file_pct / 100.0) / total_files as f64 * 100.0).min(99.0);

                                    let _ = self.progress_tx.send(DownloadProgress {
                                        model_id: model_id.to_string(),
                                        downloaded_bytes: overall_downloaded_bytes + downloaded,
                                        total_bytes: overall_downloaded_bytes + file_total_size,
                                        percent: total_pct,
                                        speed_mb: current_speed,
                                        status: "downloading".to_string(),
                                        error: None,
                                    });
                                    last_broadcast = std::time::Instant::now();
                                }
                            }
                            None => break,
                        }
                    }
                }
            }

            if is_canceled {
                drop(file);
                if part_path.exists() {
                    let _ = tokio::fs::remove_file(&part_path).await;
                }
                break;
            }

            file.flush().await.map_err(|e| format!("刷新文件缓冲区失败: {e}"))?;
            drop(file);

            // 单个文件下载完成，将 .part 重命名为正式文件名
            tokio::fs::rename(&part_path, &target_path)
                .await
                .map_err(|e| format!("重命名模型文件失败: {e}"))?;

            overall_downloaded_bytes += file_total_size;
        }

        {
            let mut map = self.download_cancellations.lock().await;
            map.remove(model_id);
        }

        if is_canceled {
            info!("用户取消下载模型 {}，已清理临时缓存", model_id);
            let _ = self.progress_tx.send(DownloadProgress {
                model_id: model_id.to_string(),
                downloaded_bytes: 0,
                total_bytes: 0,
                percent: 0.0,
                speed_mb: 0.0,
                status: "canceled".to_string(),
                error: Some("下载已由用户取消，已清除下载缓存".to_string()),
            });
            return Err("下载已由用户取消".to_string());
        }

        let _ = self.progress_tx.send(DownloadProgress {
            model_id: model_id.to_string(),
            downloaded_bytes: overall_downloaded_bytes,
            total_bytes: overall_downloaded_bytes,
            percent: 100.0,
            speed_mb: 0.0,
            status: "completed".to_string(),
            error: None,
        });

        Ok(())
    }

    /// 取消正在进行的模型下载，并自动清除已下载的缓存文件 (.part)
    pub async fn cancel_download(&self, model_id: &str) -> bool {
        let mut map = self.download_cancellations.lock().await;
        let was_active = if let Some(tx) = map.remove(model_id) {
            let _ = tx.send(());
            true
        } else {
            false
        };
        drop(map);

        // 清除对应的 .part 临时文件缓存（包含主模型与配套 mmproj 视觉塔）
        let presets = self.get_presets().await;
        if let Some(target_preset) = presets.iter().find(|p| p.id == model_id) {
            let part_path = self.models_dir.join(format!("{}.part", target_preset.filename));
            if part_path.exists() {
                info!("主动清理已取消的模型缓存文件: {:?}", part_path);
                let _ = tokio::fs::remove_file(&part_path).await;
            }
            if let Some(ref mm) = target_preset.mmproj_filename {
                let mm_part = self.models_dir.join(format!("{}.part", mm));
                if mm_part.exists() {
                    info!("主动清理已取消的视觉塔缓存文件: {:?}", mm_part);
                    let _ = tokio::fs::remove_file(&mm_part).await;
                }
            }
        }

        let _ = self.progress_tx.send(DownloadProgress {
            model_id: model_id.to_string(),
            downloaded_bytes: 0,
            total_bytes: 0,
            percent: 0.0,
            speed_mb: 0.0,
            status: "canceled".to_string(),
            error: Some("下载已由用户取消，已清除下载缓存".to_string()),
        });

        was_active
    }

    /// 下载全套纸质单据与表格 OCR 模型组件 (包含 PP-OCRv6_det_small, PP-OCRv6_rec_medium, slanet-plus, 字典)
    pub async fn download_ocr_bundle(&self) -> Result<(), String> {
        let ocr_dir = crate::paths::get_user_ocr_models_dir();
        let total_bundle_size = get_ocr_total_expected_bytes();

        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut map = self.download_cancellations.lock().await;
            if map.contains_key(OCR_BUNDLE_ID) {
                return Err("OCR 模型组件正在下载中，请勿重复触发".to_string());
            }
            map.insert(OCR_BUNDLE_ID.to_string(), cancel_tx);
        }

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

        let mut accumulated_bytes = 0u64;
        let mut is_canceled = false;

        for spec in OCR_DOWNLOAD_SPECS.iter() {
            let target_path = ocr_dir.join(spec.filename);
            let part_path = ocr_dir.join(format!("{}.part", spec.filename));

            // 如果文件已存在且大小正常，执行 SHA-256 校验确保未损坏
            if target_path.exists() {
                if let Ok(meta) = target_path.metadata() {
                    if meta.len() >= spec.expected_size.saturating_sub(1024) {
                        if verify_file_sha256(&target_path, spec.sha256).unwrap_or(false) {
                            accumulated_bytes += meta.len();
                            let _ = self.progress_tx.send(DownloadProgress {
                                model_id: OCR_BUNDLE_ID.to_string(),
                                downloaded_bytes: accumulated_bytes,
                                total_bytes: total_bundle_size,
                                percent: (accumulated_bytes as f64 / total_bundle_size as f64 * 100.0).min(99.0),
                                speed_mb: 0.0,
                                status: "downloading".to_string(),
                                error: None,
                            });
                            continue;
                        } else {
                            warn!("本地 OCR 组件 {} 哈希校验不匹配，将被重新下载覆盖", spec.filename);
                            let _ = tokio::fs::remove_file(&target_path).await;
                        }
                    }
                }
            }

            let mut file_downloaded = if part_path.exists() {
                std::fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };

            let mut req = client.get(spec.download_url);
            if file_downloaded > 0 {
                req = req.header("Range", format!("bytes={}-", file_downloaded));
            }

            let resp = tokio::select! {
                res = req.send() => {
                    match res {
                        Ok(r) => r,
                        Err(e) => {
                            let mut map = self.download_cancellations.lock().await;
                            map.remove(OCR_BUNDLE_ID);
                            return Err(format!("连接下载源失败 ({}): {e}", spec.filename));
                        }
                    }
                }
                _ = &mut cancel_rx => {
                    is_canceled = true;
                    let _ = tokio::fs::remove_file(&part_path).await;
                    break;
                }
            };

            let mut stream = resp.bytes_stream();
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&part_path)
                .await
                .map_err(|e| format!("打开临时文件失败 ({}): {e}", spec.filename))?;

            let mut last_broadcast = std::time::Instant::now();
            let mut speed_calc_time = std::time::Instant::now();
            let mut speed_bytes = 0u64;
            let mut current_speed = 0.0f64;

            loop {
                tokio::select! {
                    _ = &mut cancel_rx => {
                        is_canceled = true;
                        break;
                    }
                    chunk_opt = stream.next() => {
                        match chunk_opt {
                            Some(chunk_result) => {
                                let chunk = match chunk_result {
                                    Ok(c) => c,
                                    Err(e) => {
                                        let mut map = self.download_cancellations.lock().await;
                                        map.remove(OCR_BUNDLE_ID);
                                        return Err(format!("下载数据流中断 ({}): {e}", spec.filename));
                                    }
                                };
                                let len = chunk.len() as u64;
                                file_downloaded += len;
                                speed_bytes += len;

                                file.write_all(&chunk).await.map_err(|e| format!("写入数据块失败: {e}"))?;

                                if speed_calc_time.elapsed() >= std::time::Duration::from_millis(500) {
                                    let secs = speed_calc_time.elapsed().as_secs_f64();
                                    current_speed = (speed_bytes as f64 / 1024.0 / 1024.0) / secs;
                                    speed_bytes = 0;
                                    speed_calc_time = std::time::Instant::now();
                                }

                                if last_broadcast.elapsed() >= std::time::Duration::from_millis(150) {
                                    let cur_total = accumulated_bytes + file_downloaded;
                                    let percent = ((cur_total as f64 / total_bundle_size as f64) * 100.0).min(99.9);
                                    let _ = self.progress_tx.send(DownloadProgress {
                                        model_id: OCR_BUNDLE_ID.to_string(),
                                        downloaded_bytes: cur_total,
                                        total_bytes: total_bundle_size,
                                        percent,
                                        speed_mb: current_speed,
                                        status: "downloading".to_string(),
                                        error: None,
                                    });
                                    last_broadcast = std::time::Instant::now();
                                }
                            }
                            None => break,
                        }
                    }
                }
            }

            if is_canceled {
                drop(file);
                if part_path.exists() {
                    let _ = tokio::fs::remove_file(&part_path).await;
                }
                break;
            }

            file.flush().await.map_err(|e| format!("刷新缓冲区失败: {e}"))?;
            drop(file);

            // 强校验 SHA-256 完整性指纹，防止传输截断或 CDN 损坏
            let is_valid = verify_file_sha256(&part_path, spec.sha256)
                .map_err(|e| format!("计算临时文件哈希失败 ({}): {e}", spec.filename))?;

            if !is_valid {
                let _ = tokio::fs::remove_file(&part_path).await;
                let mut map = self.download_cancellations.lock().await;
                map.remove(OCR_BUNDLE_ID);
                return Err(format!(
                    "OCR 组件完整性校验失败 ({}): SHA-256 校验和不匹配，已清除损坏的缓存",
                    spec.filename
                ));
            }

            // 当前组件校验通过，重命名为正式模型文件名
            tokio::fs::rename(&part_path, &target_path)
                .await
                .map_err(|e| format!("重命名文件失败 ({}): {e}", spec.filename))?;

            accumulated_bytes += file_downloaded;
        }

        {
            let mut map = self.download_cancellations.lock().await;
            map.remove(OCR_BUNDLE_ID);
        }

        if is_canceled {
            info!("用户取消了 OCR 模型组件下载，已清理临时缓存");
            let _ = self.progress_tx.send(DownloadProgress {
                model_id: OCR_BUNDLE_ID.to_string(),
                downloaded_bytes: 0,
                total_bytes: total_bundle_size,
                percent: 0.0,
                speed_mb: 0.0,
                status: "canceled".to_string(),
                error: Some("OCR 下载已取消".to_string()),
            });
            return Err("下载已由用户取消".to_string());
        }

        // 全套组件顺利完成
        let _ = self.progress_tx.send(DownloadProgress {
            model_id: OCR_BUNDLE_ID.to_string(),
            downloaded_bytes: total_bundle_size,
            total_bytes: total_bundle_size,
            percent: 100.0,
            speed_mb: 0.0,
            status: "completed".to_string(),
            error: None,
        });

        Ok(())
    }

    /// 取消正在进行的 OCR 组件下载并清除未完成的 .part 缓存
    pub async fn cancel_ocr_download(&self) -> bool {
        let mut map = self.download_cancellations.lock().await;
        let was_active = if let Some(tx) = map.remove(OCR_BUNDLE_ID) {
            let _ = tx.send(());
            true
        } else {
            false
        };
        drop(map);

        let ocr_dir = crate::paths::get_user_ocr_models_dir();
        for spec in OCR_DOWNLOAD_SPECS.iter() {
            let part_path = ocr_dir.join(format!("{}.part", spec.filename));
            if part_path.exists() {
                let _ = tokio::fs::remove_file(&part_path).await;
            }
        }

        let _ = self.progress_tx.send(DownloadProgress {
            model_id: OCR_BUNDLE_ID.to_string(),
            downloaded_bytes: 0,
            total_bytes: 0,
            percent: 0.0,
            speed_mb: 0.0,
            status: "canceled".to_string(),
            error: Some("OCR 下载已取消，已清除缓存".to_string()),
        });

        was_active
    }

    /// 清理并删除已下载的 OCR 模型组件 (释放存储空间)
    pub async fn delete_ocr_bundle(&self) -> Result<(), String> {
        let ocr_dir = crate::paths::get_user_ocr_models_dir();
        for spec in OCR_DOWNLOAD_SPECS.iter() {
            let target_path = ocr_dir.join(spec.filename);
            let part_path = ocr_dir.join(format!("{}.part", spec.filename));
            if target_path.exists() {
                let _ = tokio::fs::remove_file(&target_path).await;
            }
            if part_path.exists() {
                let _ = tokio::fs::remove_file(&part_path).await;
            }
        }
        // 兼容清理历史残留的 small 模型文件
        let legacy_rec = ocr_dir.join("PP-OCRv6_rec_small.onnx");
        if legacy_rec.exists() {
            let _ = tokio::fs::remove_file(&legacy_rec).await;
        }
        Ok(())
    }

    /// 清理并删除已下载的本地 GGUF 模型文件 (若包含从属视觉塔一并清理)
    pub async fn delete_model(&self, filename: &str) -> Result<(), String> {
        let active = self.get_active_model().await;
        if active.as_deref() == Some(filename) {
            self.stop_server().await;
        }

        let main_path = self.models_dir.join(filename);
        if main_path.exists() {
            tokio::fs::remove_file(&main_path)
                .await
                .map_err(|e| format!("删除模型文件失败: {e}"))?;
        }

        let main_part = self.models_dir.join(format!("{}.part", filename));
        if main_part.exists() {
            let _ = tokio::fs::remove_file(&main_part).await;
        }

        if filename == "Qwen3.5-0.8B-Q4_K_M.gguf" {
            let mm = self.models_dir.join("mmproj-BF16.gguf");
            if mm.exists() {
                let _ = tokio::fs::remove_file(&mm).await;
            }
            let mm_part = self.models_dir.join("mmproj-BF16.gguf.part");
            if mm_part.exists() {
                let _ = tokio::fs::remove_file(&mm_part).await;
            }
        } else {
            let candidate_mms = [
                self.models_dir.join(format!("{}.mmproj.gguf", filename.trim_end_matches(".gguf"))),
                self.models_dir.join(format!("mmproj-{}.gguf", filename.trim_end_matches(".gguf"))),
            ];
            for mm in candidate_mms {
                if mm.exists() {
                    let _ = tokio::fs::remove_file(&mm).await;
                }
            }
        }

        Ok(())
    }
}

/// 确保 Drop 时彻底终止子进程，杜绝孤儿/僵尸进程
impl Drop for ModelManager {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.active_child.try_lock() {
            if let Some(mut child) = guard.take() {
                warn!("ModelManager 被销毁，强制清理运行中的 llama-server 子进程...");
                let _ = child.start_kill();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_presets_loading() {
        let mgr = ModelManager::new();
        let presets = mgr.get_presets().await;
        assert_eq!(presets.len(), 2);
        assert_eq!(presets[0].id, "qwen3.5-0.8b-q4_k_m");
        assert_eq!(presets[0].mmproj_filename.as_deref(), Some("mmproj-BF16.gguf"));
        assert_eq!(presets[1].id, "minicpm5-2b-q4_k_m");
    }

    #[tokio::test]
    async fn test_modelscope_connection() {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap();

        let qwen_url = "https://modelscope.cn/models/unsloth/Qwen3.5-0.8B-GGUF/resolve/master/Qwen3.5-0.8B-Q4_K_M.gguf";
        let qwen_resp = client
            .get(qwen_url)
            .header("Range", "bytes=0-1023")
            .send()
            .await
            .unwrap();

        assert!(qwen_resp.status().is_success() || qwen_resp.status() == reqwest::StatusCode::PARTIAL_CONTENT);

        let mmproj_url = "https://modelscope.cn/models/unsloth/Qwen3.5-0.8B-GGUF/resolve/master/mmproj-BF16.gguf";
        let mmproj_resp = client
            .get(mmproj_url)
            .header("Range", "bytes=0-1023")
            .send()
            .await
            .unwrap();

        assert!(mmproj_resp.status().is_success() || mmproj_resp.status() == reqwest::StatusCode::PARTIAL_CONTENT);

        let minicpm_url = "https://modelscope.cn/models/OpenBMB/MiniCPM5-2B-gguf/resolve/master/MiniCPM5-2B-Q4_K_M.gguf";
        let minicpm_resp = client
            .get(minicpm_url)
            .header("Range", "bytes=0-1023")
            .send()
            .await
            .unwrap();

        assert!(minicpm_resp.status().is_success() || minicpm_resp.status() == reqwest::StatusCode::PARTIAL_CONTENT);
    }

    #[tokio::test]
    async fn test_cancel_download_cleans_cache() {
        let mgr = ModelManager::new();
        // 创建一个模拟的 .part 文件
        let test_part = mgr.models_dir.join("Qwen3.5-0.8B-Q4_K_M.gguf.part");
        tokio::fs::write(&test_part, b"temporary download cache").await.unwrap();
        assert!(test_part.exists());

        // 调用 cancel_download 取消
        let _ = mgr.cancel_download("qwen3.5-0.8b-q4_k_m").await;
        // 验证 .part 临时文件已被自动清理
        assert!(!test_part.exists(), "cancel_download 应当自动清理 .part 临时缓存文件");
    }

    #[test]
    fn test_ocr_bundle_metadata() {
        assert_eq!(OCR_DOWNLOAD_SPECS.len(), 4);
        let total_bytes = get_ocr_total_expected_bytes();
        // 4 个文件总大小应在 85MB ~ 105MB 之间 (采用 rec_medium 约 94.4MB)
        assert!(total_bytes > 85 * 1024 * 1024 && total_bytes < 105 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_cancel_ocr_download_cleans_cache() {
        let mgr = ModelManager::new();
        let ocr_dir = crate::paths::get_user_ocr_models_dir();
        let test_part = ocr_dir.join(format!("{}.part", crate::paths::OCR_DET_FILENAME));
        tokio::fs::write(&test_part, b"temporary ocr cache").await.unwrap();
        assert!(test_part.exists());

        let _ = mgr.cancel_ocr_download().await;
        assert!(!test_part.exists(), "cancel_ocr_download 应自动清理 OCR 临时 .part 缓存");
    }

    #[test]
    fn test_verify_file_sha256() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_sha256.txt");
        let content = b"hello sensidoc ocr sha256";
        std::fs::write(&temp_file, content).unwrap();

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        let expected = format!("{:x}", hasher.finalize());

        assert!(verify_file_sha256(&temp_file, &expected).unwrap());
        assert!(!verify_file_sha256(&temp_file, "0000000000000000000000000000000000000000000000000000000000000000").unwrap());
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_ocr_specs_sha256_against_local_files_if_exist() {
        let ocr_dir = crate::paths::get_ocr_models_dir();
        for spec in OCR_DOWNLOAD_SPECS.iter() {
            let path = ocr_dir.join(spec.filename);
            if path.exists() {
                let ok = verify_file_sha256(&path, spec.sha256).expect("计算本地模型哈希失败");
                assert!(ok, "本地已就绪模型文件 {} 的 SHA-256 校验失败", spec.filename);
            }
        }
    }
}

