use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
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
    pub modelscope_id: String,
    pub description: String,
    pub size_desc: String,
    pub is_downloaded: bool,
    pub is_active: bool,
}

/// 下载进度事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub status: String, // "downloading", "completed", "failed"
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
            server_port: 8081,
            progress_tx,
        }
    }

    pub fn get_progress_receiver(&self) -> broadcast::Receiver<DownloadProgress> {
        self.progress_tx.subscribe()
    }

    /// 获取内置的 3 款魔搭模型预设列表及其当前本地状态
    pub async fn get_presets(&self) -> Vec<ModelPreset> {
        let active = self.get_active_model().await;
        let presets = vec![
            ModelPreset {
                id: "lfm2-350m-extract".to_string(),
                name: "LFM2-350M-Extract (Q8_0)".to_string(),
                filename: "LFM2-350M-Extract-Q8_0.gguf".to_string(),
                modelscope_id: "LiquidAI/LFM2-350M-Extract-GGUF".to_string(),
                description: "专为敏感信息与实体抽取定向优化的超轻量模型，毫秒级推理".to_string(),
                size_desc: "~370 MB".to_string(),
                is_downloaded: false,
                is_active: false,
            },
            ModelPreset {
                id: "lfm2.5-vl-450m".to_string(),
                name: "LFM2.5-VL-450M (Q8_0)".to_string(),
                filename: "LFM2.5-VL-450M-Q8_0.gguf".to_string(),
                modelscope_id: "LiquidAI/LFM2.5-VL-450M-GGUF".to_string(),
                description: "超小身材大视觉/文本理解模型，Q8_0 高精度量化，结构化提取表现优异".to_string(),
                size_desc: "~360 MB".to_string(),
                is_downloaded: false,
                is_active: false,
            },
            ModelPreset {
                id: "qwen2.5-1.5b-instruct".to_string(),
                name: "Qwen2.5-1.5B-Instruct (Q4_K_M)".to_string(),
                filename: "qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
                modelscope_id: "Qwen/Qwen2.5-1.5B-Instruct-GGUF".to_string(),
                description: "通义千问 2.5 经典 1.5B 指令微调版，语义结构理解综合性能强".to_string(),
                size_desc: "~980 MB".to_string(),
                is_downloaded: false,
                is_active: false,
            },
        ];

        presets
            .into_iter()
            .map(|mut p| {
                let path = self.models_dir.join(&p.filename);
                p.is_downloaded = path.exists() && std::fs::metadata(&path).map(|m| m.len() > 1024 * 1024).unwrap_or(false);
                p.is_active = active.as_deref() == Some(&p.filename);
                p
            })
            .collect()
    }

    /// 扫描 models/ 目录下的所有可用 gguf 模型文件
    pub fn list_local_models(&self) -> Vec<String> {
        let mut list = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.models_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.ends_with(".gguf") {
                            list.push(name);
                        }
                    }
                }
            }
        }
        list
    }

    /// 获取当前运行中的模型名称（主动探测 8081 端口上的 llama-server 真实运行状态并自动同步）
    pub async fn get_active_model(&self) -> Option<String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build()
            .unwrap_or_default();

        if let Ok(resp) = client.get("http://127.0.0.1:8081/v1/models").send().await {
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

        // 若探测 8081 端口不可达，清空状态并返回 None
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

    /// 停止当前正在运行的 llama-server 子进程，彻底释放端口与显存
    pub async fn stop_server(&self) {
        let mut child_guard = self.active_child.lock().await;
        if let Some(mut child) = child_guard.take() {
            info!("正在终止当前 llama-server 进程 (PID: {:?})...", child.id());
            let _ = child.kill().await;
            let _ = child.wait().await;
            info!("旧 llama-server 进程已终止释放。");
        }

        // Unix 系统下二次强杀残留孤儿进程，确保 8081 端口绝对释放
        #[cfg(unix)]
        {
            let _ = tokio::process::Command::new("pkill")
                .arg("-f")
                .arg("llama-server")
                .output()
                .await;
        }

        let mut model_guard = self.active_model.lock().await;
        *model_guard = None;
    }

    /// 启动指定的本地模型（杜绝僵尸孤儿进程）
    pub async fn start_model(&self, model_filename: &str) -> Result<(), String> {
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

        info!(
            "启动 llama-server: 路径={:?}, 模型={:?}, 端口={}",
            self.llama_bin_path,
            model_path,
            self.server_port
        );

        let mut cmd = Command::new(&self.llama_bin_path);
        cmd.arg("-m")
            .arg(&model_path)
            .arg("--port")
            .arg(self.server_port.to_string())
            .arg("-ngl")
            .arg("99") // 开启 macOS Apple Silicon Metal GPU 全量卸载加速
            .arg("-c")
            .arg("4096")
            .arg("--host")
            .arg("127.0.0.1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // macOS 动态链接库环境变量绑定
        let lib_dir = crate::paths::get_lib_dir();
        if lib_dir.exists() {
            cmd.env("DYLD_LIBRARY_PATH", &lib_dir);
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

    /// 从魔搭（ModelScope）下载预设模型，支持断点续传与实时进度广播
    pub async fn download_model(&self, model_id: &str) -> Result<(), String> {
        let presets = self.get_presets().await;
        let target_preset = presets
            .iter()
            .find(|p| p.id == model_id)
            .ok_or_else(|| format!("未知的预设模型 ID: {model_id}"))?;

        let filename = &target_preset.filename;
        let modelscope_repo = &target_preset.modelscope_id;
        let target_path = self.models_dir.join(filename);
        let part_path = self.models_dir.join(format!("{filename}.part"));

        let download_url = format!(
            "https://modelscope.cn/models/{modelscope_repo}/resolve/master/{filename}"
        );

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

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

        let resp = req.send().await.map_err(|e| format!("连接魔搭下载源失败: {e}"))?;

        let total_size = match resp.content_length() {
            Some(len) => len + downloaded,
            None => downloaded + 350 * 1024 * 1024,
        };

        let mut stream = resp.bytes_stream();
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&part_path)
            .await
            .map_err(|e| format!("打开临时模型文件失败: {e}"))?;

        let mut last_broadcast = std::time::Instant::now();
        let mut speed_calc_time = std::time::Instant::now();
        let mut speed_downloaded = 0u64;
        let mut current_speed = 0.0f64;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("下载数据分块失败: {e}"))?;
            file.write_all(&chunk).await.map_err(|e| format!("写入模型文件失败: {e}"))?;
            downloaded += chunk.len() as u64;
            speed_downloaded += chunk.len() as u64;

            if speed_calc_time.elapsed() >= std::time::Duration::from_millis(500) {
                let elapsed_secs = speed_calc_time.elapsed().as_secs_f64();
                current_speed = (speed_downloaded as f64 / (1024.0 * 1024.0)) / elapsed_secs;
                speed_downloaded = 0;
                speed_calc_time = std::time::Instant::now();
            }

            if last_broadcast.elapsed() >= std::time::Duration::from_millis(150) || downloaded >= total_size {
                let percent = (downloaded as f64 / total_size as f64 * 100.0).min(100.0);
                let _ = self.progress_tx.send(DownloadProgress {
                    model_id: model_id.to_string(),
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    percent,
                    speed_mb: current_speed,
                    status: if downloaded >= total_size { "completed".to_string() } else { "downloading".to_string() },
                    error: None,
                });
                last_broadcast = std::time::Instant::now();
            }
        }

        file.flush().await.map_err(|e| format!("刷新文件缓冲区失败: {e}"))?;
        drop(file);

        // 下载完成，将 .part 重命名为正式文件名
        tokio::fs::rename(&part_path, &target_path)
            .await
            .map_err(|e| format!("重命名模型文件失败: {e}"))?;

        let _ = self.progress_tx.send(DownloadProgress {
            model_id: model_id.to_string(),
            downloaded_bytes: total_size,
            total_bytes: total_size,
            percent: 100.0,
            speed_mb: 0.0,
            status: "completed".to_string(),
            error: None,
        });

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
        assert_eq!(presets.len(), 3);
        assert!(presets.iter().any(|p| p.id == "lfm2-350m-extract"));
        assert!(presets.iter().any(|p| p.id == "lfm2.5-vl-450m"));
        assert!(presets.iter().any(|p| p.id == "qwen2.5-1.5b-instruct"));
    }

    #[tokio::test]
    async fn test_modelscope_connection() {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap();

        let url = "https://modelscope.cn/models/LiquidAI/LFM2.5-VL-450M-GGUF/resolve/master/LFM2.5-VL-450M-Q8_0.gguf";
        let resp = client
            .get(url)
            .header("Range", "bytes=0-1023")
            .send()
            .await
            .unwrap();

        assert!(resp.status().is_success() || resp.status() == reqwest::StatusCode::PARTIAL_CONTENT);
    }
}

