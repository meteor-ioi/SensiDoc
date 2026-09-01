use anydoc::{self, Format};
use std::path::Path;

/// 支持的文档转 Markdown 引擎
pub struct DocConverter;

impl DocConverter {
    /// 将内存字节流转换为 Markdown 字符串
    ///
    /// - 优先根据文件名后缀判断格式（支持 doc, docx, ppt, pptx, xls, xlsx, pdf, rtf, epub, ods, odp, csv, txt, md）
    /// - 若无后缀或未命中，则使用 anydoc 基于魔数自动探测
    /// - 对纯文本及 Markdown 文件直接解码为 UTF-8，实现平滑兼容
    pub fn convert_bytes(filename: &str, bytes: &[u8]) -> Result<String, String> {
        let path = Path::new(filename);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        // 1. 针对纯文本与 Markdown 的极速直通分支
        if ext == "txt" || ext == "md" || ext == "markdown" {
            let (cow, _, _) = encoding_rs::UTF_8.decode(bytes);
            return Ok(cow.into_owned());
        }

        // 2. 匹配 anydoc 支持的格式
        let detected_format = Format::from_extension(&ext)
            .or_else(|| Format::from_bytes(bytes));

        match detected_format {
            Some(format) => anydoc::to_markdown_bytes(bytes, format)
                .map_err(|e| format!("文档转换解析失败: {e}")),
            None => {
                // 尝试由 anydoc 自动探测
                anydoc::to_markdown_bytes(bytes, None)
                    .or_else(|_| {
                        // 若不是已知格式，最后尝试作为纯文本 UTF-8 处理
                        if std::str::from_utf8(bytes).is_ok() {
                            let (cow, _, _) = encoding_rs::UTF_8.decode(bytes);
                            Ok(cow.into_owned())
                        } else {
                            Err(format!("不支持的文件格式或内容无法解析: {filename}"))
                        }
                    })
            }
        }
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
}
