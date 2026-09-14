use anydoc::{self, Format};
use std::path::Path;

/// 转换结果及文档类型
#[derive(Debug, Clone)]
pub struct ConvertResult {
    pub markdown: String,
    pub doc_type: String, // "scan" 或 "native"
}

/// 支持的文档转 Markdown 引擎
pub struct DocConverter;

impl DocConverter {
    /// 检查是否为常见图像格式或魔数
    pub fn is_image(filename: &str, bytes: &[u8]) -> bool {
        let ext = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp" | "tiff" | "tif")
            || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            || bytes.starts_with(b"\xff\xd8\xff")
            || bytes.starts_with(b"BM")
            || (bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
            || bytes.starts_with(b"II*\x00")
            || bytes.starts_with(b"MM\x00*")
    }

    /// 将内存字节流转换为 Markdown 字符串 (兼容旧接口)
    pub fn convert_bytes(filename: &str, bytes: &[u8]) -> Result<String, String> {
        Self::convert_bytes_detailed(filename, bytes).map(|res| res.markdown)
    }

    /// 将内存字节流转换为 Markdown 字符串并返回文档类型 ("scan" 或 "native")
    pub fn convert_bytes_detailed(filename: &str, bytes: &[u8]) -> Result<ConvertResult, String> {
        let path = Path::new(filename);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        // 1. 针对纯文本与 Markdown 的极速直通分支
        if ext == "txt" || ext == "md" || ext == "markdown" {
            let (cow, _, _) = encoding_rs::UTF_8.decode(bytes);
            return Ok(ConvertResult {
                markdown: cow.into_owned(),
                doc_type: "native".to_string(),
            });
        }

        // 2. 针对图像格式，拦截并路由至原生 OCR 引擎
        if Self::is_image(filename, bytes) {
            if !crate::paths::is_ocr_ready() {
                return Err("OCR_NOT_READY: 纸质单据与表格 OCR 模型尚未就绪，请先在设置面板中下载轻量模型套件".to_string());
            }

            let ocr_res = crate::ocr::OcrEngine::recognize_bytes(bytes)?;
            return Ok(ConvertResult {
                markdown: ocr_res.markdown,
                doc_type: "scan".to_string(),
            });
        }

        // 3. 针对 PDF 文档：优先提取文本层，扫描版自动回退至原生 OCR 引擎
        if ext == "pdf" {
            let anydoc_res: Result<String, String> = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                anydoc::to_markdown_bytes(bytes, Format::Pdf).map_err(|e| e.to_string())
            }))
            .unwrap_or_else(|_| Err("PDF 文本流排序解析异常".to_string()));

            match anydoc_res {
                Ok(ref text) if text.trim().chars().count() >= 30 => {
                    return Ok(ConvertResult {
                        markdown: text.clone(),
                        doc_type: "native".to_string(),
                    });
                }
                ref err_or_short => {
                    // 判断是否具备扫描件特征（文本过短或 anydoc 明确报错提示 Scanned/OCR）
                    let is_likely_scanned = match &err_or_short {
                        Ok(t) => t.trim().chars().count() < 30,
                        Err(msg) => {
                            msg.contains("Scanned")
                                || msg.contains("OCR")
                                || msg.contains("no extractable text")
                                || msg.contains("PDF 文本流排序解析异常")
                        }
                    };

                    // 若检测到是图片/扫描件 PDF，且 OCR 模型尚未就绪，友好返回 OCR_NOT_READY 引导弹窗
                    if is_likely_scanned && !crate::paths::is_ocr_ready() {
                        return Err("OCR_NOT_READY: 当前 PDF 为图片扫描件，需要 OCR 引擎识别，请先在设置面板中下载轻量模型套件".to_string());
                    }

                    // OCR 模型就绪时，优先执行扫描件内嵌图像提取与 OCR 流水线
                    if crate::paths::is_ocr_ready() {
                        match Self::extract_scanned_pdf(bytes) {
                            Ok(scanned_md) if scanned_md.trim().chars().count() >= 10 => {
                                return Ok(ConvertResult {
                                    markdown: scanned_md,
                                    doc_type: "scan".to_string(),
                                });
                            }
                            Ok(_) | Err(_) => {
                                // 若未能提取到有效图像，回退到原有 anydoc 结果或报错
                                if let Ok(ref text) = anydoc_res {
                                    return Ok(ConvertResult {
                                        markdown: text.clone(),
                                        doc_type: "native".to_string(),
                                    });
                                } else {
                                    return Err(format!("PDF 扫描件图片提取或 OCR 识别失败: {filename}"));
                                }
                            }
                        }
                    }

                    // 兜底回退
                    if let Ok(text) = anydoc_res {
                        return Ok(ConvertResult {
                            markdown: text,
                            doc_type: "native".to_string(),
                        });
                    } else {
                        return Err(format!("PDF 文档解析失败: {filename}"));
                    }
                }
            }
        }

        // 4. 匹配 anydoc 支持的格式
        let detected_format = Format::from_extension(&ext)
            .or_else(|| Format::from_bytes(bytes));

        match detected_format {
            Some(format) => anydoc::to_markdown_bytes(bytes, format)
                .map(|md| ConvertResult {
                    markdown: md,
                    doc_type: "native".to_string(),
                })
                .map_err(|e| format!("文档转换解析失败: {e}")),
            None => {
                // 尝试由 anydoc 自动探测
                anydoc::to_markdown_bytes(bytes, None)
                    .map(|md| ConvertResult {
                        markdown: md,
                        doc_type: "native".to_string(),
                    })
                    .or_else(|_| {
                        // 若不是已知格式，最后尝试作为纯文本 UTF-8 处理
                        if std::str::from_utf8(bytes).is_ok() {
                            let (cow, _, _) = encoding_rs::UTF_8.decode(bytes);
                            Ok(ConvertResult {
                                markdown: cow.into_owned(),
                                doc_type: "native".to_string(),
                            })
                        } else {
                            Err(format!("不支持的文件格式或内容无法解析: {filename}"))
                        }
                    })
            }
        }
    }

    /// 从扫描版 PDF 中提取内嵌图像流并按页聚合 OCR 识别文本
    pub fn extract_scanned_pdf(bytes: &[u8]) -> Result<String, String> {
        let doc = lopdf::Document::load_mem(bytes).map_err(|e| format!("加载 PDF 文档失败: {e}"))?;
        let pages = doc.get_pages();
        if pages.is_empty() {
            return Err("PDF 不包含有效页面".to_string());
        }

        let mut page_results = Vec::new();

        for (&page_num, &page_id) in &pages {
            let mut page_images = Vec::new();
            if let Ok(page_obj) = doc.get_object(page_id) {
                if let Ok(page_dict) = page_obj.as_dict() {
                    let rot = get_page_rotation(&doc, page_dict);

                    let res_obj = page_dict.get(b"Resources").ok().and_then(|r| deref_obj(&doc, r));
                    if let Some(res_dict) = res_obj.and_then(|r| r.as_dict().ok()) {
                        let xobj_obj = res_dict.get(b"XObject").ok().and_then(|x| deref_obj(&doc, x));
                        if let Some(xobj_dict) = xobj_obj.and_then(|x| x.as_dict().ok()) {
                            for (_, val) in xobj_dict.iter() {
                                if let Some(deref_val) = deref_obj(&doc, val) {
                                    if let Ok(stream) = deref_val.as_stream() {
                                        let is_image = stream.dict.get(b"Subtype")
                                            .ok()
                                            .and_then(|s| deref_obj(&doc, s))
                                            .and_then(|s| s.as_name().ok())
                                            .map(|name| name == b"Image")
                                            .unwrap_or(false);

                                        if is_image {
                                            let is_dct = stream.dict.get(b"Filter")
                                                .ok()
                                                .and_then(|f| deref_obj(&doc, f))
                                                .and_then(|f| f.as_name().ok())
                                                .map(|name| name == b"DCTDecode")
                                                .unwrap_or(false);

                                            let loaded = if is_dct {
                                                image::load_from_memory(&stream.content).ok()
                                            } else if let Ok(decompressed) = stream.decompressed_content() {
                                                image::load_from_memory(&decompressed).ok()
                                            } else {
                                                None
                                            };

                                            if let Some(mut img) = loaded {
                                                // 过滤小图标或杂质（尺寸小于 100x100）
                                                if img.width() >= 100 && img.height() >= 100 {
                                                    // 遵从 PDF 页面规范旋转角度
                                                    if rot != 0 {
                                                        img = match (rot % 360 + 360) % 360 {
                                                            90 => img.rotate90(),
                                                            180 => img.rotate180(),
                                                            270 => img.rotate270(),
                                                            _ => img,
                                                        };
                                                    }
                                                    page_images.push(img);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            for (idx, img) in page_images.iter().enumerate() {
                if let Ok(ocr_res) = crate::ocr::OcrEngine::recognize_image(img) {
                    let text = ocr_res.markdown.trim();
                    if !text.is_empty() {
                        let header = if pages.len() > 1 {
                            if page_images.len() > 1 {
                                format!("### 第 {} 页 (图像 {})\n\n", page_num, idx + 1)
                            } else {
                                format!("### 第 {} 页\n\n", page_num)
                            }
                        } else {
                            String::new()
                        };
                        page_results.push(format!("{}{}", header, text));
                    }
                }
            }
        }

        if page_results.is_empty() {
            return Err("PDF 中未提取到有效图像内容".to_string());
        }

        Ok(page_results.join("\n\n---\n\n"))
    }

    /// 从本地文件路径读取并转换为 Markdown
    pub fn convert_file(path: impl AsRef<Path>) -> Result<String, String> {
        let path_ref = path.as_ref();
        let bytes = std::fs::read(path_ref)
            .map_err(|e| format!("读取文件失败 ({}): {e}", path_ref.display()))?;
        let filename = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document");
        Self::convert_bytes(filename, &bytes)
    }
}

fn deref_obj<'a>(doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Option<&'a lopdf::Object> {
    match obj {
        lopdf::Object::Reference(id) => doc.get_object(*id).ok(),
        _ => Some(obj),
    }
}

/// 解析 PDF 页面旋转角度 (包含沿 Parent 链继承的 /Rotate 属性)
fn get_page_rotation(doc: &lopdf::Document, page_dict: &lopdf::Dictionary) -> i64 {
    if let Some(rot) = page_dict.get(b"Rotate").ok().and_then(|r| deref_obj(doc, r)).and_then(|r| r.as_i64().ok()) {
        return rot;
    }
    // 递归检查父节点继承的旋转属性
    let mut current = page_dict.get(b"Parent").ok().and_then(|p| deref_obj(doc, p)).and_then(|p| p.as_dict().ok());
    while let Some(parent_dict) = current {
        if let Some(rot) = parent_dict.get(b"Rotate").ok().and_then(|r| deref_obj(doc, r)).and_then(|r| r.as_i64().ok()) {
            return rot;
        }
        current = parent_dict.get(b"Parent").ok().and_then(|p| deref_obj(doc, p)).and_then(|p| p.as_dict().ok());
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text_conversion() {
        let text = "# 这是一个测试文档\n\n包含个人手机号：13800138000";
        let res = DocConverter::convert_bytes("test.txt", text.as_bytes()).unwrap();
        assert_eq!(res, text);
    }

    #[test]
    fn test_markdown_conversion() {
        let text = "# 敏感合同\n\n甲方：张三 身份证：110101199003072345";
        let res = DocConverter::convert_bytes("contract.md", text.as_bytes()).unwrap();
        assert_eq!(res, text);
    }

    #[test]
    fn test_csv_conversion() {
        let csv_data = "name,id_card,phone\n张三,110101199003072345,13812345678\n李四,310101199501011234,13987654321";
        let res = DocConverter::convert_bytes("contacts.csv", csv_data.as_bytes()).unwrap();
        assert!(res.contains("张三"));
        assert!(res.contains("13812345678"));
    }

    #[test]
    fn test_is_image_detection() {
        assert!(DocConverter::is_image("invoice.png", b"fake"));
        assert!(DocConverter::is_image("receipt.JPG", b"fake"));
        assert!(DocConverter::is_image("scan.bmp", b"fake"));
        assert!(DocConverter::is_image("doc.webp", b"fake"));
        assert!(!DocConverter::is_image("contract.pdf", b"%PDF-1.5"));
        assert!(!DocConverter::is_image("notes.txt", b"plain text"));

        // 魔数探测
        assert!(DocConverter::is_image("unknown", b"\x89PNG\r\n\x1a\n"));
        assert!(DocConverter::is_image("unknown", b"\xff\xd8\xff\xe0"));
        assert!(DocConverter::is_image("unknown", b"BM\x00\x00"));
    }

    #[test]
    fn test_image_ocr_routing_when_not_ready() {
        // 当 OCR 模型文件未就绪时，拦截并提示 OCR_NOT_READY
        let dummy_png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        let res = DocConverter::convert_bytes("invoice.png", dummy_png);
        // 如果在测试环境中模型尚未下载，应返回包含 OCR_NOT_READY 的错误
        if !crate::paths::is_ocr_ready() {
            assert!(res.is_err());
            let err = res.unwrap_err();
            assert!(err.contains("OCR_NOT_READY"));
        }
    }

    #[test]
    #[ignore = "本地特定售前样本调试"]
    fn test_debug_pdf_parsing() {
        let pdf_path = "/Users/icychick/Desktop/售前需求/胜寒 - SUPERL/KH Tax invoice.pdf";
        if !std::path::Path::new(pdf_path).exists() {
            return;
        }

        let bytes = std::fs::read(pdf_path).unwrap();
        let t0 = std::time::Instant::now();
        let res = DocConverter::convert_bytes("test.pdf", &bytes);
        println!("KH Tax invoice 总耗时: {}ms, 结果成功: {}", t0.elapsed().as_millis(), res.is_ok());
        if let Ok(md) = res {
            println!("输出字符数: {}", md.chars().count());
            println!("前 300 字符: {}", &md[..md.len().min(300)]);
        }
    }
}
