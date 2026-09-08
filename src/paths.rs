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

/// 获取 llama-server 二进制执行路径
pub fn get_llama_bin_path() -> PathBuf {
    if let Some(res) = get_bundle_resources_dir() {
        let bin_in_bundle = res.join("bin").join("llama-server");
        if bin_in_bundle.exists() {
            return bin_in_bundle;
        }
        let macos_bin = res.parent().map(|c| c.join("MacOS").join("llama-server"));
        if let Some(mb) = macos_bin {
            if mb.exists() {
                return mb;
            }
        }
    }

    let local_bin = PathBuf::from("bin/llama-server");
    if local_bin.exists() {
        return local_bin;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let rel_bin = dir.join("bin").join("llama-server");
            if rel_bin.exists() {
                return rel_bin;
            }
        }
    }

    PathBuf::from("bin/llama-server")
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
        assert!(bin_path.exists(), "bin/llama-server 应能正确解析");
    }
}
