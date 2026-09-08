use crate::extractor::SensitiveItem;
use lopdf::{content::Content, Document, Object};
use regex::Regex;
use std::io::{Cursor, Read, Write};
use std::sync::LazyLock;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

static RE_P_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<w:p(?:\s+[^>]*)?>.*?</w:p>").unwrap()
});

static RE_WT_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)(<w:t(?:\s+[^>]*)?>)(.*?)(</w:t>)").unwrap()
});

static RE_PPT_P_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<a:p(?:\s+[^>]*)?>.*?</a:p>").unwrap()
});

static RE_AT_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)(<a:t(?:\s+[^>]*)?>)(.*?)(</a:t>)").unwrap()
});

static RE_XLSX_SI_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<si(?:\s+[^>]*)?>.*?</si>").unwrap()
});

static RE_T_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)(<t(?:\s+[^>]*)?>)(.*?)(</t>)").unwrap()
});

pub struct Desensitizer;

impl Desensitizer {
    /// 标准脱敏打码策略：
    /// - 长度 <= 2 的词：全替换为 `*` (如 "张三" -> "**", "李" -> "*")
    /// - 长度 > 2 的词：保留首尾字符，中间字符替换为等长 `*` (如 "13800138000" -> "1*********0")
    pub fn mask_text(raw: &str) -> String {
        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        if len == 0 {
            return String::new();
        }
        if len <= 2 {
            "*".repeat(len)
        } else {
            let first = chars[0];
            let last = chars[len - 1];
            let stars = "*".repeat(len - 2);
            format!("{first}{stars}{last}")
        }
    }

    /// 纯文本 / Markdown / CSV 通用打码脱敏
    pub fn desensitize_plain_text(original: &str, items: &[SensitiveItem]) -> String {
        let mut result = original.to_string();
        for item in items {
            let raw = &item.text;
            if raw.is_empty() {
                continue;
            }
            let masked = Self::mask_text(raw);
            result = result.replace(raw, &masked);
        }
        result
    }

    /// 针对 CSV 文件的规范脱敏导出 (保留逗号与引号结构)
    pub fn desensitize_csv(original_csv: &str, items: &[SensitiveItem]) -> String {
        Self::desensitize_plain_text(original_csv, items)
    }

    /// 针对 DOCX 文件的内存原生无损脱敏
    ///
    /// - 解压 DOCX ZIP 容器
    /// - 对 `word/document.xml`、`word/header*.xml`、`word/footer*.xml`、`word/footnotes.xml` 执行跨 Run 节点合并打码
    /// - 其余所有图片、样式、关系文件原样写回并重新打包为标准 .docx
    pub fn desensitize_docx(bytes: &[u8], items: &[SensitiveItem]) -> Result<Vec<u8>, String> {
        let cursor = Cursor::new(bytes);
        let mut zip_in = ZipArchive::new(cursor).map_err(|e| format!("无法解压 DOCX 容器: {e}"))?;
        
        let out_buf = Vec::new();
        let mut zip_out = ZipWriter::new(Cursor::new(out_buf));

        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).map_err(|e| format!("读取 ZIP 条目失败: {e}"))?;
            let name = file.name().to_string();
            let is_docx_text_xml = name == "word/document.xml"
                || name.starts_with("word/header")
                || name.starts_with("word/footer")
                || name == "word/footnotes.xml"
                || name == "word/endnotes.xml";

            let options = SimpleFileOptions::default()
                .compression_method(file.compression())
                .unix_permissions(file.unix_mode().unwrap_or(0o644));

            zip_out.start_file(&name, options).map_err(|e| format!("创建 ZIP 条目失败 ({name}): {e}"))?;

            if is_docx_text_xml {
                let mut content = String::new();
                file.read_to_string(&mut content).map_err(|e| format!("读取 XML 文本失败 ({name}): {e}"))?;
                let desensitized_xml = Self::desensitize_docx_xml(&content, items);
                zip_out.write_all(desensitized_xml.as_bytes()).map_err(|e| format!("写入 XML 失败: {e}"))?;
            } else {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).map_err(|e| format!("读取文件字节失败 ({name}): {e}"))?;
                zip_out.write_all(&buffer).map_err(|e| format!("写入文件字节失败: {e}"))?;
            }
        }

        let cursor_out = zip_out.finish().map_err(|e| format!("封装 DOCX 失败: {e}"))?;
        Ok(cursor_out.into_inner())
    }

    /// DOCX XML 段落级跨 `<w:t>` 节点合并与精准回写算法
    pub fn desensitize_docx_xml(xml: &str, items: &[SensitiveItem]) -> String {
        if items.is_empty() {
            return xml.to_string();
        }

        // 遍历所有 <w:p> 段落
        RE_P_BLOCK.replace_all(xml, |caps: &regex::Captures| {
            let p_block = &caps[0];
            Self::desensitize_xml_container(p_block, &RE_WT_TAG, items)
        }).into_owned()
    }

    /// 针对 XLSX 文件的内存原生无损脱敏
    pub fn desensitize_xlsx(bytes: &[u8], items: &[SensitiveItem]) -> Result<Vec<u8>, String> {
        let cursor = Cursor::new(bytes);
        let mut zip_in = ZipArchive::new(cursor).map_err(|e| format!("无法解压 XLSX 容器: {e}"))?;
        
        let out_buf = Vec::new();
        let mut zip_out = ZipWriter::new(Cursor::new(out_buf));

        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).map_err(|e| format!("读取 ZIP 条目失败: {e}"))?;
            let name = file.name().to_string();
            let is_shared_strings = name == "xl/sharedStrings.xml";
            let is_worksheet = name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml");

            let options = SimpleFileOptions::default()
                .compression_method(file.compression())
                .unix_permissions(file.unix_mode().unwrap_or(0o644));

            zip_out.start_file(&name, options).map_err(|e| format!("创建 ZIP 条目失败 ({name}): {e}"))?;

            if is_shared_strings {
                let mut content = String::new();
                file.read_to_string(&mut content).map_err(|e| format!("读取 sharedStrings.xml 失败: {e}"))?;
                let desensitized_xml = Self::desensitize_xlsx_shared_strings(&content, items);
                zip_out.write_all(desensitized_xml.as_bytes()).map_err(|e| format!("写入 sharedStrings.xml 失败: {e}"))?;
            } else if is_worksheet {
                let mut content = String::new();
                file.read_to_string(&mut content).map_err(|e| format!("读取 worksheet 失败 ({name}): {e}"))?;
                let desensitized_xml = Self::desensitize_xlsx_worksheet(&content, items);
                zip_out.write_all(desensitized_xml.as_bytes()).map_err(|e| format!("写入 worksheet 失败: {e}"))?;
            } else {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).map_err(|e| format!("读取文件字节失败 ({name}): {e}"))?;
                zip_out.write_all(&buffer).map_err(|e| format!("写入文件字节失败: {e}"))?;
            }
        }

        let cursor_out = zip_out.finish().map_err(|e| format!("封装 XLSX 失败: {e}"))?;
        Ok(cursor_out.into_inner())
    }

    /// XLSX sharedStrings.xml 字符串池脱敏
    pub fn desensitize_xlsx_shared_strings(xml: &str, items: &[SensitiveItem]) -> String {
        if items.is_empty() {
            return xml.to_string();
        }

        RE_XLSX_SI_BLOCK.replace_all(xml, |caps: &regex::Captures| {
            let si_block = &caps[0];
            Self::desensitize_xml_container(si_block, &RE_T_TAG, items)
        }).into_owned()
    }

    /// XLSX worksheet 行内字符串与直接值脱敏
    pub fn desensitize_xlsx_worksheet(xml: &str, items: &[SensitiveItem]) -> String {
        if items.is_empty() {
            return xml.to_string();
        }
        Self::desensitize_xml_container(xml, &RE_T_TAG, items)
    }

    /// 针对 PPTX 演示文稿的内存原生无损脱敏
    pub fn desensitize_pptx(bytes: &[u8], items: &[SensitiveItem]) -> Result<Vec<u8>, String> {
        let cursor = Cursor::new(bytes);
        let mut zip_in = ZipArchive::new(cursor).map_err(|e| format!("无法解压 PPTX 容器: {e}"))?;
        
        let out_buf = Vec::new();
        let mut zip_out = ZipWriter::new(Cursor::new(out_buf));

        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).map_err(|e| format!("读取 ZIP 条目失败: {e}"))?;
            let name = file.name().to_string();
            let is_slide_xml = (name.starts_with("ppt/slides/slide") || name.starts_with("ppt/notesSlides/notesSlide"))
                && name.ends_with(".xml");

            let options = SimpleFileOptions::default()
                .compression_method(file.compression())
                .unix_permissions(file.unix_mode().unwrap_or(0o644));

            zip_out.start_file(&name, options).map_err(|e| format!("创建 ZIP 条目失败 ({name}): {e}"))?;

            if is_slide_xml {
                let mut content = String::new();
                file.read_to_string(&mut content).map_err(|e| format!("读取 slide XML 失败 ({name}): {e}"))?;
                let desensitized_xml = Self::desensitize_pptx_xml(&content, items);
                zip_out.write_all(desensitized_xml.as_bytes()).map_err(|e| format!("写入 slide XML 失败: {e}"))?;
            } else {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).map_err(|e| format!("读取文件字节失败 ({name}): {e}"))?;
                zip_out.write_all(&buffer).map_err(|e| format!("写入文件字节失败: {e}"))?;
            }
        }

        let cursor_out = zip_out.finish().map_err(|e| format!("封装 PPTX 失败: {e}"))?;
        Ok(cursor_out.into_inner())
    }

    /// PPTX 幻灯片 XML 段落脱敏
    pub fn desensitize_pptx_xml(xml: &str, items: &[SensitiveItem]) -> String {
        if items.is_empty() {
            return xml.to_string();
        }

        RE_PPT_P_BLOCK.replace_all(xml, |caps: &regex::Captures| {
            let p_block = &caps[0];
            Self::desensitize_xml_container(p_block, &RE_AT_TAG, items)
        }).into_owned()
    }

    /// 通用容器内文本 Run 跨标签合并、检索与就地字符映射替换引擎
    fn desensitize_xml_container(
        container_xml: &str,
        tag_regex: &Regex,
        items: &[SensitiveItem],
    ) -> String {
        struct TagMatch<'a> {
            open_tag: &'a str,
            close_tag: &'a str,
            unescaped_text: String,
            match_start: usize,
            match_end: usize,
        }

        let mut tags: Vec<TagMatch> = Vec::new();
        for m in tag_regex.captures_iter(container_xml) {
            let full_m = m.get(0).unwrap();
            let open_tag = m.get(1).unwrap().as_str();
            let raw_text = m.get(2).unwrap().as_str();
            let close_tag = m.get(3).unwrap().as_str();

            let unescaped = quick_xml::escape::unescape(raw_text)
                .map(|cow| cow.into_owned())
                .unwrap_or_else(|_| raw_text.to_string());

            tags.push(TagMatch {
                open_tag,
                close_tag,
                unescaped_text: unescaped,
                match_start: full_m.start(),
                match_end: full_m.end(),
            });
        }

        if tags.is_empty() {
            return container_xml.to_string();
        }

        // 1. 组装容器内全部纯文本并记录每个标签的字符区间 (Unicode char index)
        let mut full_text = String::new();
        let mut char_ranges: Vec<(usize, usize)> = Vec::new(); // [char_start, char_end)

        for tag in &tags {
            let start_char_idx = full_text.chars().count();
            let count = tag.unescaped_text.chars().count();
            full_text.push_str(&tag.unescaped_text);
            char_ranges.push((start_char_idx, start_char_idx + count));
        }

        // 2. 检查是否有任何敏感词命中该容器文本
        let mut chars: Vec<char> = full_text.chars().collect();
        let mut any_hit = false;

        for item in items {
            let raw = &item.text;
            if raw.is_empty() {
                continue;
            }
            let target_chars: Vec<char> = raw.chars().collect();
            let target_len = target_chars.len();
            if target_len == 0 || target_len > chars.len() {
                continue;
            }

            let masked_chars: Vec<char> = Self::mask_text(raw).chars().collect();

            // 搜索全部出现点 (滑动窗口精确匹配 Unicode 字符序列)
            let mut i = 0;
            while i + target_len <= chars.len() {
                if chars[i..i + target_len] == target_chars[..] {
                    // 命中！用掩码字符等长替换
                    for k in 0..target_len {
                        chars[i + k] = masked_chars[k];
                    }
                    any_hit = true;
                    i += target_len;
                } else {
                    i += 1;
                }
            }
        }

        if !any_hit {
            return container_xml.to_string();
        }

        // 3. 将打码后的字符数组按原区间分割回写至各 XML 标签
        let mut new_container = String::with_capacity(container_xml.len());
        let mut last_byte_idx = 0;

        for (idx, tag) in tags.iter().enumerate() {
            let (c_start, c_end) = char_ranges[idx];
            let tag_chars = &chars[c_start..c_end];
            let new_plain: String = tag_chars.iter().collect();
            let escaped_text = quick_xml::escape::escape(&new_plain);

            new_container.push_str(&container_xml[last_byte_idx..tag.match_start]);
            new_container.push_str(tag.open_tag);
            new_container.push_str(&escaped_text);
            new_container.push_str(tag.close_tag);

            last_byte_idx = tag.match_end;
        }

        new_container.push_str(&container_xml[last_byte_idx..]);
        new_container
    }

    /// 针对原生 PDF 文件的内存 Content Stream 指令级精准脱敏
    ///
    /// - 解析 PDF 对象树与页面 /Contents 流
    /// - 捕获 `Tj` / `TJ` / `'` / `"` 文字绘制算子
    /// - 对 ASCII、UTF-16BE 及 GB18030 字符流进行等长掩码替换
    /// - 清除文档全局 /Metadata (XMP) 与 /Info
    pub fn desensitize_pdf(bytes: &[u8], items: &[SensitiveItem]) -> Result<Vec<u8>, String> {
        let mut doc = Document::load_mem(bytes).map_err(|e| format!("加载 PDF 文档失败: {e}"))?;
        let pages = doc.get_pages();

        for (&_page_num, &page_id) in &pages {
            let content_data = match doc.get_page_content(page_id) {
                Ok(data) => data,
                Err(_) => continue,
            };

            let mut content = match Content::decode(&content_data) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut page_modified = false;
            for op in &mut content.operations {
                match op.operator.as_str() {
                    "Tj" | "'" => {
                        if let Some(obj) = op.operands.first_mut() {
                            if Self::desensitize_pdf_object(obj, items) {
                                page_modified = true;
                            }
                        }
                    }
                    "\"" => {
                        if let Some(obj) = op.operands.get_mut(2) {
                            if Self::desensitize_pdf_object(obj, items) {
                                page_modified = true;
                            }
                        }
                    }
                    "TJ" => {
                        if let Some(Object::Array(arr)) = op.operands.first_mut() {
                            if Self::desensitize_pdf_tj_array(arr, items) {
                                page_modified = true;
                            }
                        }
                    }
                    _ => {}
                }
            }

            if page_modified {
                if let Ok(encoded) = content.encode() {
                    let _ = doc.change_page_content(page_id, encoded);
                }
            }
        }

        // 清理文档级敏感元数据
        doc.trailer.remove(b"Info");
        doc.trailer.remove(b"Metadata");

        let mut out_buf = Vec::new();
        doc.save_to(&mut out_buf).map_err(|e| format!("序列化 PDF 失败: {e}"))?;
        Ok(out_buf)
    }

    /// 单个 PDF String 对象的自适应编码等长脱敏
    fn desensitize_pdf_object(obj: &mut Object, items: &[SensitiveItem]) -> bool {
        if let Object::String(bytes, _format) = obj {
            if bytes.is_empty() || items.is_empty() {
                return false;
            }

            // 1. UTF-16BE 编码 (以 \xFE\xFF 开头)
            if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
                let u16_slice: Vec<u16> = bytes[2..]
                    .chunks_exact(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                if let Ok(decoded_str) = String::from_utf16(&u16_slice) {
                    let mut new_str = decoded_str.clone();
                    let mut changed = false;
                    for item in items {
                        if item.text.is_empty() {
                            continue;
                        }
                        if new_str.contains(&item.text) {
                            let masked = Self::mask_text(&item.text);
                            new_str = new_str.replace(&item.text, &masked);
                            changed = true;
                        }
                    }
                    if changed {
                        let mut new_bytes = vec![0xFE, 0xFF];
                        for ch in new_str.encode_utf16() {
                            new_bytes.extend_from_slice(&ch.to_be_bytes());
                        }
                        *bytes = new_bytes;
                        return true;
                    }
                }
                return false;
            }

            // 2. UTF-8 编码尝试
            if let Ok(utf8_str) = std::str::from_utf8(bytes) {
                let mut new_str = utf8_str.to_string();
                let mut changed = false;
                for item in items {
                    if item.text.is_empty() {
                        continue;
                    }
                    if new_str.contains(&item.text) {
                        let masked = Self::mask_text(&item.text);
                        new_str = new_str.replace(&item.text, &masked);
                        changed = true;
                    }
                }
                if changed {
                    *bytes = new_str.into_bytes();
                    return true;
                }
            }

            // 3. GB18030 / GBK 编码尝试 (老旧中文 PDF 兼容)
            let (cow, _, had_errors) = encoding_rs::GB18030.decode(bytes);
            if !had_errors {
                let mut new_str = cow.into_owned();
                let mut changed = false;
                for item in items {
                    if item.text.is_empty() {
                        continue;
                    }
                    if new_str.contains(&item.text) {
                        let masked = Self::mask_text(&item.text);
                        new_str = new_str.replace(&item.text, &masked);
                        changed = true;
                    }
                }
                if changed {
                    let (encoded_bytes, _, _) = encoding_rs::GB18030.encode(&new_str);
                    *bytes = encoded_bytes.into_owned();
                    return true;
                }
            }
        }
        false
    }

    /// PDF TJ 算子数组脱敏
    fn desensitize_pdf_tj_array(arr: &mut Vec<Object>, items: &[SensitiveItem]) -> bool {
        let mut changed = false;
        for obj in arr.iter_mut() {
            if Self::desensitize_pdf_object(obj, items) {
                changed = true;
            }
        }
        changed
    }

    /// 全格式自动路由脱敏分发器
    ///
    /// - 输入：原文件名、原文件字节 (若有)、转换后的 Markdown 原文、已检出敏感项列表
    /// - 输出：(脱敏后文件名, 文件字节流, HTTP Content-Type)
    pub fn desensitize_document_auto(
        filename: &str,
        original_bytes: Option<&[u8]>,
        fallback_markdown: &str,
        items: &[SensitiveItem],
    ) -> Result<(String, Vec<u8>, &'static str), String> {
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        let stem = std::path::Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");

        // 0. PDF 原生脱敏
        if ext == "pdf" {
            if let Some(bytes) = original_bytes {
                if let Ok(out_bytes) = Self::desensitize_pdf(bytes, items) {
                    return Ok((
                        format!("{stem}_脱敏.pdf"),
                        out_bytes,
                        "application/pdf",
                    ));
                }
            }
        }

        // 1. DOCX 原生脱敏
        if ext == "docx" {
            if let Some(bytes) = original_bytes {
                let out_bytes = Self::desensitize_docx(bytes, items)?;
                return Ok((format!("{stem}_脱敏.docx"), out_bytes, "application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
            }
        }

        // 2. XLSX 原生脱敏
        if ext == "xlsx" {
            if let Some(bytes) = original_bytes {
                let out_bytes = Self::desensitize_xlsx(bytes, items)?;
                return Ok((format!("{stem}_脱敏.xlsx"), out_bytes, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
            }
        }

        // 3. PPTX 原生脱敏
        if ext == "pptx" {
            if let Some(bytes) = original_bytes {
                let out_bytes = Self::desensitize_pptx(bytes, items)?;
                return Ok((format!("{stem}_脱敏.pptx"), out_bytes, "application/vnd.openxmlformats-officedocument.presentationml.presentation"));
            }
        }

        // 4. CSV 纯文本脱敏
        if ext == "csv" {
            let csv_src = if let Some(bytes) = original_bytes {
                let (cow, _, _) = encoding_rs::UTF_8.decode(bytes);
                cow.into_owned()
            } else {
                fallback_markdown.to_string()
            };
            let out_str = Self::desensitize_csv(&csv_src, items);
            return Ok((format!("{stem}_脱敏.csv"), out_str.into_bytes(), "text/csv; charset=utf-8"));
        }

        // 5. TXT 纯文本脱敏
        if ext == "txt" {
            let txt_src = if let Some(bytes) = original_bytes {
                let (cow, _, _) = encoding_rs::UTF_8.decode(bytes);
                cow.into_owned()
            } else {
                fallback_markdown.to_string()
            };
            let out_str = Self::desensitize_plain_text(&txt_src, items);
            return Ok((format!("{stem}_脱敏.txt"), out_str.into_bytes(), "text/plain; charset=utf-8"));
        }

        // 6. 其他所有格式（或未提供原始字节时）：平滑回退输出脱敏后的 Markdown 文档
        let desensitized_md = Self::desensitize_plain_text(fallback_markdown, items);
        Ok((format!("{stem}_脱敏.md"), desensitized_md.into_bytes(), "text/markdown; charset=utf-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_text() {
        assert_eq!(Desensitizer::mask_text(""), "");
        assert_eq!(Desensitizer::mask_text("李"), "*");
        assert_eq!(Desensitizer::mask_text("张三"), "**");
        assert_eq!(Desensitizer::mask_text("张建国"), "张*国");
        assert_eq!(Desensitizer::mask_text("13800138000"), "1*********0");
        assert_eq!(
            Desensitizer::mask_text("北京华云智远科技有限公司"),
            "北**********司"
        );
    }

    #[test]
    fn test_plain_text_desensitization() {
        let text = "法定代表人：张建国，手机号：13800138000，公司：北京华云智远科技有限公司。";
        let items = vec![
            SensitiveItem {
                id: "1".into(),
                text: "张建国".into(),
                category: "法人".into(),
                priority: "high".into(),
                count: 1,
                positions: vec![],
                source: "ai".into(),
            },
            SensitiveItem {
                id: "2".into(),
                text: "13800138000".into(),
                category: "电话".into(),
                priority: "high".into(),
                count: 1,
                positions: vec![],
                source: "regex".into(),
            },
            SensitiveItem {
                id: "3".into(),
                text: "北京华云智远科技有限公司".into(),
                category: "公司".into(),
                priority: "medium".into(),
                count: 1,
                positions: vec![],
                source: "ai".into(),
            },
        ];

        let res = Desensitizer::desensitize_plain_text(text, &items);
        assert_eq!(
            res,
            "法定代表人：张*国，手机号：1*********0，公司：北**********司。"
        );
    }

    #[test]
    fn test_docx_xml_fragmented_runs() {
        // 模拟 Word 中被拆分成 2 个 Run 节点的敏感词 "张建国"
        let xml = r#"<w:p><w:r><w:t>法定代表人：张</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>建国 先生</w:t></w:r></w:p>"#;
        let items = vec![SensitiveItem {
            id: "1".into(),
            text: "张建国".into(),
            category: "法人".into(),
            priority: "high".into(),
            count: 1,
            positions: vec![],
            source: "ai".into(),
        }];

        let res = Desensitizer::desensitize_docx_xml(xml, &items);
        assert!(res.contains("<w:t>法定代表人：张</w:t>"));
        assert!(res.contains("<w:t>*国 先生</w:t>"));
    }

    #[test]
    fn test_docx_xml_multiple_items_and_entities() {
        let xml = r#"<w:p><w:r><w:t>采购方 &amp; 甲方：北京华云智远科技有限公司</w:t></w:r><w:r><w:t>，总金额：￥1,860,000.00</w:t></w:r></w:p>"#;
        let items = vec![
            SensitiveItem {
                id: "1".into(),
                text: "北京华云智远科技有限公司".into(),
                category: "企业".into(),
                priority: "high".into(),
                count: 1,
                positions: vec![],
                source: "ai".into(),
            },
            SensitiveItem {
                id: "2".into(),
                text: "￥1,860,000.00".into(),
                category: "金额".into(),
                priority: "high".into(),
                count: 1,
                positions: vec![],
                source: "ai".into(),
            },
        ];

        let res = Desensitizer::desensitize_docx_xml(xml, &items);
        assert!(res.contains("采购方 &amp; 甲方：北**********司"));
        assert!(res.contains("总金额：￥***********0"));
    }

    #[test]
    fn test_docx_zip_pipeline() {
        // 创建一个模拟 DOCX 内存 ZIP
        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buf));
            zip.start_file("word/document.xml", SimpleFileOptions::default()).unwrap();
            zip.write_all(r#"<w:document><w:body><w:p><w:r><w:t>联系人：张建国</w:t></w:r></w:p></w:body></w:document>"#.as_bytes()).unwrap();
            zip.start_file("word/styles.xml", SimpleFileOptions::default()).unwrap();
            zip.write_all(b"<styles><style name=\"Normal\"/></styles>").unwrap();
            zip.finish().unwrap();
        }

        let items = vec![SensitiveItem {
            id: "1".into(),
            text: "张建国".into(),
            category: "人名".into(),
            priority: "high".into(),
            count: 1,
            positions: vec![],
            source: "ai".into(),
        }];

        let desensitized_zip_bytes = Desensitizer::desensitize_docx(&buf, &items).expect("DOCX脱敏成功");
        
        // 解开脱敏后的 ZIP 检验
        let mut zip_in = ZipArchive::new(Cursor::new(desensitized_zip_bytes)).unwrap();
        let mut doc_xml = String::new();
        zip_in.by_name("word/document.xml").unwrap().read_to_string(&mut doc_xml).unwrap();
        assert!(doc_xml.contains("联系人：张*国"));

        let mut styles_xml = String::new();
        zip_in.by_name("word/styles.xml").unwrap().read_to_string(&mut styles_xml).unwrap();
        assert_eq!(styles_xml, "<styles><style name=\"Normal\"/></styles>");
    }

    #[test]
    fn test_xlsx_zip_pipeline() {
        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buf));
            zip.start_file("xl/sharedStrings.xml", SimpleFileOptions::default()).unwrap();
            zip.write_all(r#"<sst><si><t>供应商：上海创科恒通网络设备有限公司</t></si></sst>"#.as_bytes()).unwrap();
            zip.finish().unwrap();
        }

        let items = vec![SensitiveItem {
            id: "1".into(),
            text: "上海创科恒通网络设备有限公司".into(),
            category: "企业".into(),
            priority: "medium".into(),
            count: 1,
            positions: vec![],
            source: "ai".into(),
        }];

        let out_bytes = Desensitizer::desensitize_xlsx(&buf, &items).expect("XLSX脱敏成功");
        let mut zip_in = ZipArchive::new(Cursor::new(out_bytes)).unwrap();
        let mut sst_xml = String::new();
        zip_in.by_name("xl/sharedStrings.xml").unwrap().read_to_string(&mut sst_xml).unwrap();
        assert!(sst_xml.contains("供应商：上************司"));
    }

    #[test]
    fn test_pptx_zip_pipeline() {
        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buf));
            zip.start_file("ppt/slides/slide1.xml", SimpleFileOptions::default()).unwrap();
            zip.write_all(r#"<p:sld><a:p><a:r><a:t>汇报人：周建国</a:t></a:r></a:p></p:sld>"#.as_bytes()).unwrap();
            zip.finish().unwrap();
        }

        let items = vec![SensitiveItem {
            id: "1".into(),
            text: "周建国".into(),
            category: "高管".into(),
            priority: "high".into(),
            count: 1,
            positions: vec![],
            source: "ai".into(),
        }];

        let out_bytes = Desensitizer::desensitize_pptx(&buf, &items).expect("PPTX脱敏成功");
        let mut zip_in = ZipArchive::new(Cursor::new(out_bytes)).unwrap();
        let mut slide_xml = String::new();
        zip_in.by_name("ppt/slides/slide1.xml").unwrap().read_to_string(&mut slide_xml).unwrap();
        assert!(slide_xml.contains("汇报人：周*国"));
    }

    #[test]
    fn test_desensitize_document_auto_dispatch() {
        let items = vec![SensitiveItem {
            id: "1".into(),
            text: "13800138000".into(),
            category: "手机号".into(),
            priority: "high".into(),
            count: 1,
            positions: vec![],
            source: "regex".into(),
        }];

        // TXT 测试
        let (name, bytes, mime) = Desensitizer::desensitize_document_auto(
            "test_user.txt",
            Some(b"phone: 13800138000"),
            "",
            &items,
        ).unwrap();
        assert_eq!(name, "test_user_脱敏.txt");
        assert_eq!(mime, "text/plain; charset=utf-8");
        assert_eq!(String::from_utf8(bytes).unwrap(), "phone: 1*********0");

        // CSV 测试
        let (name_csv, bytes_csv, mime_csv) = Desensitizer::desensitize_document_auto(
            "records.csv",
            Some(b"id,phone\n1,13800138000\n"),
            "",
            &items,
        ).unwrap();
        assert_eq!(name_csv, "records_脱敏.csv");
        assert_eq!(mime_csv, "text/csv; charset=utf-8");
        assert_eq!(String::from_utf8(bytes_csv).unwrap(), "id,phone\n1,1*********0\n");

        // Markdown 回退测试 (未提供原始字节时)
        let (name_md, bytes_md, mime_md) = Desensitizer::desensitize_document_auto(
            "unknown.xyz",
            None,
            "# 合同内容\n联系电话：13800138000",
            &items,
        ).unwrap();
        assert_eq!(name_md, "unknown_脱敏.md");
        assert_eq!(mime_md, "text/markdown; charset=utf-8");
        assert_eq!(String::from_utf8(bytes_md).unwrap(), "# 合同内容\n联系电话：1*********0");
    }

    #[test]
    fn test_pdf_content_stream_pipeline() {
        use lopdf::content::Operation;
        use lopdf::{dictionary, Document, Object, Stream};

        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
        });

        // 构造含有敏感词 "13800138000" 和 "Zhou Jianguo" 的 PDF Content Stream
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Td", vec![100.into(), 600.into()]),
                Operation::new("Tj", vec![Object::string_literal("Contact: 13800138000")]),
                Operation::new("Td", vec![0.into(), (-20).into()]),
                Operation::new("Tj", vec![Object::string_literal("Manager: Zhou Jianguo")]),
                Operation::new("ET", vec![]),
            ],
        };

        let content_stream = Stream::new(dictionary! {}, content.encode().unwrap());
        let content_id = doc.add_object(content_stream);

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });

        let pages_dict = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        doc.set_object(pages_id, pages_dict);

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let mut pdf_bytes = Vec::new();
        doc.save_to(&mut pdf_bytes).unwrap();

        let items = vec![
            SensitiveItem {
                id: "1".into(),
                text: "13800138000".into(),
                category: "手机号".into(),
                priority: "high".into(),
                count: 1,
                positions: vec![],
                source: "regex".into(),
            },
            SensitiveItem {
                id: "2".into(),
                text: "Zhou Jianguo".into(),
                category: "姓名".into(),
                priority: "high".into(),
                count: 1,
                positions: vec![],
                source: "ai".into(),
            },
        ];

        let out_bytes = Desensitizer::desensitize_pdf(&pdf_bytes, &items).expect("PDF脱敏成功");
        
        // 重新加载脱敏后的 PDF 检验内容
        let doc_out = Document::load_mem(&out_bytes).unwrap();
        let pages = doc_out.get_pages();
        let &first_page = pages.values().next().unwrap();
        let page_content = doc_out.get_page_content(first_page).unwrap();
        let content_out = Content::decode(&page_content).unwrap();

        let mut all_texts = Vec::new();
        for op in &content_out.operations {
            if op.operator == "Tj" {
                if let Some(Object::String(bytes, _)) = op.operands.first() {
                    all_texts.push(String::from_utf8_lossy(bytes).into_owned());
                }
            }
        }

        assert!(all_texts.iter().any(|t| t.contains("1*********0")));
        assert!(all_texts.iter().any(|t| t.contains("Z**********o")));
        assert!(!all_texts.iter().any(|t| t.contains("13800138000")));

        // 测试 auto 路由
        let (name_pdf, auto_pdf_bytes, mime_pdf) = Desensitizer::desensitize_document_auto(
            "report.pdf",
            Some(&pdf_bytes),
            "",
            &items,
        ).unwrap();
        assert_eq!(name_pdf, "report_脱敏.pdf");
        assert_eq!(mime_pdf, "application/pdf");
        assert!(!auto_pdf_bytes.is_empty());
    }
}
