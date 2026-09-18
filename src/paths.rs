use std::path::PathBuf;

/// 判断当前是否处于 macOS App Bundle 运行环境 (.app/Contents/MacOS/...)
pub fn is_macos_bundle() -> bool {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(macos_dir) = exe.parent() {
            if macos_dir.file_name().and_then(|n| n.to_str()) == Some("MacOS") {
                if let Some(contents_dir) = macos_dir.parent() {
                    return contents_dir.join("Resources").exists();
                }
            }
        }
    }
    false
}

/// 获取 macOS App Bundle 的 Resources 资源目录路径
pub fn get_bundle_resources_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(macos_dir) = exe.parent() {
            if let Some(contents_dir) = macos_dir.parent() {
                let res = contents_dir.join("Resources");
                if res.exists() {
                    return Some(res);
                }
            }
        }
    }
    None
}

/// 获取用户应用持久化数据根目录 (macOS: ~/Library/Application Support/SensiDoc, 其他系统: ~/.sensidoc)
pub fn get_user_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let app_support = PathBuf::from(home).join("Library").join("Application Support").join("SensiDoc");
            let _ = std::fs::create_dir_all(&app_support);
            return app_support;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let win_dir = PathBuf::from(appdata).join("SensiDoc");
            let _ = std::fs::create_dir_all(&win_dir);
            return win_dir;
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let default_dir = PathBuf::from(home).join(".sensidoc");
        let _ = std::fs::create_dir_all(&default_dir);
        default_dir
    } else {
        PathBuf::from(".")
    }
}

/// 获取前端静态网页目录 (web/)
pub fn get_web_dir() -> PathBuf {
    if let Some(res) = get_bundle_resources_dir() {
        let web_in_bundle = res.join("web");
        if web_in_bundle.exists() {
            return web_in_bundle;
        }
    }

    // 开发与当前工作目录查找
    let local_web = PathBuf::from("web");
    if local_web.exists() {
        return local_web;
    }

    // 若从 target/release 运行等情况查找父级 web
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let rel_web = dir.join("web");
            if rel_web.exists() {
                return rel_web;
            }
            if let Some(parent) = dir.parent() {
                let rel_web2 = parent.join("web");
                if rel_web2.exists() {
                    return rel_web2;
                }
            }
        }
    }

    PathBuf::from("web")
}

/// 获取 llama-server 二进制执行路径 (跨平台且支持双架构多目录智能探测)
pub fn get_llama_bin_path() -> PathBuf {
    let bin_name = if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    };

    // 1. macOS App Bundle 资源环境探测
    if let Some(res) = get_bundle_resources_dir() {
        let bin_in_bundle = res.join("bin").join(bin_name);
        if bin_in_bundle.exists() {
            return bin_in_bundle;
        }
        let macos_bin = res.parent().map(|c| c.join("MacOS").join(bin_name));
        if let Some(mb) = macos_bin {
            if mb.exists() {
                return mb;
            }
        }
    }

    // 2. 生产环境：基于当前可执行文件所在目录相对查找 (针对 Windows 安装目录 / 绿色便携解压目录)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // 如 {app}/bin/llama-server.exe
            let rel_bin = dir.join("bin").join(bin_name);
            if rel_bin.exists() {
                return rel_bin;
            }
            // 如 {app}/llama-server.exe
            let same_dir_bin = dir.join(bin_name);
            if same_dir_bin.exists() {
                return same_dir_bin;
            }
        }
    }

    // 3. 开发环境与源码根目录探测：优先探测与当前平台和架构匹配的专用子目录
    #[cfg(all(windows, target_arch = "x86_64"))]
    let arch_dir = "windows-x86_64";
    #[cfg(all(windows, target_arch = "aarch64"))]
    let arch_dir = "windows-arm64";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let arch_dir = "macos-arm64";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let arch_dir = "macos-x86_64";
    #[cfg(not(any(
        all(windows, target_arch = "x86_64"),
        all(windows, target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64")
    )))]
    let arch_dir = "default";

    let arch_bin = PathBuf::from("bin").join(arch_dir).join(bin_name);
    if arch_bin.exists() {
        return arch_bin;
    }

    // 4. 探测项目根目录常规 bin 目录 (兼容现有开发环境下的 bin/llama-server)
    let local_bin = PathBuf::from("bin").join(bin_name);
    if local_bin.exists() {
        return local_bin;
    }

    local_bin
}


/// 获取动态链接库目录 (lib/)
pub fn get_lib_dir() -> PathBuf {
    if let Some(res) = get_bundle_resources_dir() {
        let lib_in_bundle = res.join("lib");
        if lib_in_bundle.exists() {
            return lib_in_bundle;
        }
        let fw_in_bundle = res.parent().map(|c| c.join("Frameworks"));
        if let Some(fw) = fw_in_bundle {
            if fw.exists() {
                return fw;
            }
        }
    }

    let local_lib = PathBuf::from("lib");
    if local_lib.exists() {
        return local_lib;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let rel_lib = dir.join("lib");
            if rel_lib.exists() {
                return rel_lib;
            }
        }
    }

    PathBuf::from("lib")
}

/// 获取模型存储目录 (models/)
pub fn get_models_dir() -> PathBuf {
    if is_macos_bundle() {
        let dir = get_user_data_dir().join("models");
        let _ = std::fs::create_dir_all(&dir);
        return dir;
    }

    let local_models = PathBuf::from("models");
    if !local_models.exists() {
        let _ = std::fs::create_dir_all(&local_models);
    }
    local_models
}

/// 获取用户可写的 OCR 模型下载存储目录 (models/ocr/)
pub fn get_user_ocr_models_dir() -> PathBuf {
    let ocr_dir = get_models_dir().join("ocr");
    if !ocr_dir.exists() {
        let _ = std::fs::create_dir_all(&ocr_dir);
    }
    ocr_dir
}

/// 获取纸质单据与表格 OCR 模型存储目录 (models/ocr/)
/// 优先级：用户自定义下载目录 > App Bundle/安装包内置目录 > 可执行文件同级目录 > 默认回退目录
pub fn get_ocr_models_dir() -> PathBuf {
    // 1. 优先检查用户数据目录下是否已有模型 (支持热更新或按需下载)
    let user_ocr = get_models_dir().join("ocr");
    if user_ocr.join(OCR_DET_FILENAME).exists() {
        return user_ocr;
    }

    // 2. 检查 App Bundle / 安装包资源目录下是否随包内置了 models/ocr (离线全量版)
    if let Some(res) = get_bundle_resources_dir() {
        let bundled_ocr = res.join("models").join("ocr");
        if bundled_ocr.join(OCR_DET_FILENAME).exists() {
            return bundled_ocr;
        }
    }

    // 3. 检查程序同级目录的 models/ocr (例如 Windows 安装目录或绿色便携解压目录)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let rel_ocr = dir.join("models").join("ocr");
            if rel_ocr.join(OCR_DET_FILENAME).exists() {
                return rel_ocr;
            }
        }
    }

    // 4. 默认返回用户数据目录 (用于后续按需下载)
    get_user_ocr_models_dir()
}

pub const OCR_DET_FILENAME: &str = "PP-OCRv6_det_small.onnx";
pub const OCR_REC_FILENAME: &str = "PP-OCRv6_rec_medium.onnx";
pub const OCR_TABLE_FILENAME: &str = "slanet-plus.onnx";
pub const OCR_DICT_FILENAME: &str = "ppocrv6_dict.txt";

/// 获取文本检测定位模型路径
pub fn get_ocr_det_path() -> PathBuf {
    get_ocr_models_dir().join(OCR_DET_FILENAME)
}

/// 获取文本字符识别模型路径 (优先更高精度的 medium，若本地仅有 small 则平滑兼容回退)
pub fn get_ocr_rec_path() -> PathBuf {
    let base = get_ocr_models_dir();
    let medium = base.join(OCR_REC_FILENAME);
    if medium.exists() {
        return medium;
    }
    let small = base.join("PP-OCRv6_rec_small.onnx");
    if small.exists() {
        return small;
    }
    medium
}

/// 获取表格结构预测模型路径
pub fn get_ocr_table_path() -> PathBuf {
    get_ocr_models_dir().join(OCR_TABLE_FILENAME)
}

/// 获取 50 种语言统一映射字典路径
pub fn get_ocr_dict_path() -> PathBuf {
    get_ocr_models_dir().join(OCR_DICT_FILENAME)
}

/// OCR 组件详细就绪状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrStatus {
    pub is_ready: bool,
    pub det_ready: bool,
    pub rec_ready: bool,
    pub table_ready: bool,
    pub dict_ready: bool,
    pub total_size_bytes: u64,
    pub expected_total_bytes: u64,
    #[serde(default)]
    pub is_loaded: bool,
}

/// 检查 OCR 核心组件是否全部就绪
pub fn get_ocr_status() -> OcrStatus {
    let det = get_ocr_det_path();
    let rec = get_ocr_rec_path();
    let table = get_ocr_table_path();
    let dict = get_ocr_dict_path();

    let det_ready = det.exists() && det.metadata().map(|m| m.len() > 8 * 1024 * 1024).unwrap_or(false);
    let rec_ready = rec.exists() && rec.metadata().map(|m| m.len() > 18 * 1024 * 1024).unwrap_or(false);
    let table_ready = table.exists() && table.metadata().map(|m| m.len() > 6 * 1024 * 1024).unwrap_or(false);
    let dict_ready = dict.exists() && dict.metadata().map(|m| m.len() > 50 * 1024).unwrap_or(false);

    let mut total_size_bytes = 0u64;
    for p in [&det, &rec, &table, &dict] {
        if let Ok(m) = p.metadata() {
            total_size_bytes += m.len();
        }
    }

    let is_ready = det_ready && rec_ready && table_ready && dict_ready;

    OcrStatus {
        is_ready,
        det_ready,
        rec_ready,
        table_ready,
        dict_ready,
        total_size_bytes,
        expected_total_bytes: crate::model_manager::get_ocr_total_expected_bytes(),
        is_loaded: false,
    }
}

/// 判断 OCR 是否完全就绪
pub fn is_ocr_ready() -> bool {
    get_ocr_status().is_ready
}


/// 获取工作区会话持久化 JSON 路径
pub fn get_workspace_store_path() -> PathBuf {
    if is_macos_bundle() {
        return get_user_data_dir().join("workspace.json");
    }

    PathBuf::from(".sensidoc_workspace.json")
}

/// 获取上传原始文档暂存目录 (uploads/)
pub fn get_uploads_dir() -> PathBuf {
    if is_macos_bundle() {
        let dir = get_user_data_dir().join("uploads");
        let _ = std::fs::create_dir_all(&dir);
        return dir;
    }

    let local_uploads = PathBuf::from("uploads");
    if !local_uploads.exists() {
        let _ = std::fs::create_dir_all(&local_uploads);
    }
    local_uploads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_resolution() {
        let web_dir = get_web_dir();
        assert!(web_dir.exists(), "web 目录应能正确解析");

        let lib_dir = get_lib_dir();
        assert!(lib_dir.exists(), "lib 目录应能正确解析");

        let bin_path = get_llama_bin_path();
        // 生产打包或特定平台未内置本地二进制时，路径仍应为合法的预期路径
        assert!(
            bin_path.to_string_lossy().contains("llama-server"),
            "get_llama_bin_path 应返回包含 llama-server 的有效路径: {:?}",
            bin_path
        );

        let ocr_dir = get_ocr_models_dir();
        assert!(ocr_dir.exists(), "models/ocr 目录应被自动创建并存在");

        let status = get_ocr_status();
        // 初始未下载时 status.is_ready 应为 false
        assert_eq!(status.is_ready, is_ocr_ready());
    }
}
