use crate::extractor::SensitiveItem;
use image::{DynamicImage, GenericImageView, ImageFormat};
use lopdf::{content::Content, Document, Object};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::io::{Cursor, Read, Write};
use std::str::FromStr;
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

/// 脱敏替换策略枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MaskStyle {
    /// 默认星号掩码：所有字符全量等长替换为 * (如 ***, ***********)
    #[default]
    Masking,
    /// 字符硬抹除：所有字符全量等长替换为黑块 █ (如 ███, ███████████)
    Redaction,
}

impl FromStr for MaskStyle {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "redaction" | "redact" | "erase" | "block" | "硬抹除" | "抹除" => Ok(MaskStyle::Redaction),
            _ => Ok(MaskStyle::Masking),
        }
    }
}

impl MaskStyle {
    pub fn mask_text(&self, raw: &str) -> String {
        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        if len == 0 {
            return String::new();
        }
        match self {
            MaskStyle::Masking => {
                "*".repeat(len)
            }
            MaskStyle::Redaction => {
                "█".repeat(len)
            }
        }
    }
}

pub struct Desensitizer;

impl Desensitizer {
    /// 标准脱敏打码策略（默认掩码模式）：
    /// - 所有字符全量等长替换为 `*` (彻底脱敏，不保留首尾，如 "张三" -> "**", "13800138000" -> "***********")
    pub fn mask_text(raw: &str) -> String {
        MaskStyle::Masking.mask_text(raw)
    }

    /// 支持指定策略的脱敏打码
    pub fn mask_text_with_style(raw: &str, style: MaskStyle) -> String {
        style.mask_text(raw)
    }

    /// 纯文本 / Markdown / CSV 通用打码脱敏（默认掩码模式）
    pub fn desensitize_plain_text(original: &str, items: &[SensitiveItem]) -> String {
        Self::desensitize_plain_text_with_style(original, items, MaskStyle::Masking)
    }

    /// 纯文本 / Markdown / CSV 通用打码脱敏（支持指定策略）
    pub fn desensitize_plain_text_with_style(original: &str, items: &[SensitiveItem], style: MaskStyle) -> String {
        let mut result = original.to_string();
        for item in items {
            let raw = &item.text;
            if raw.is_empty() {
                continue;
            }
            let masked = style.mask_text(raw);
            result = result.replace(raw, &masked);
        }
        result
    }

    /// 针对 CSV 文件的规范脱敏导出 (保留逗号与引号结构)
    pub fn desensitize_csv(original_csv: &str, items: &[SensitiveItem]) -> String {
        Self::desensitize_csv_with_style(original_csv, items, MaskStyle::Masking)
    }

    /// 针对 CSV 文件的规范脱敏导出 (支持指定策略)
    pub fn desensitize_csv_with_style(original_csv: &str, items: &[SensitiveItem], style: MaskStyle) -> String {
        Self::desensitize_plain_text_with_style(original_csv, items, style)
    }

    /// 针对 DOCX 文件的内存原生无损脱敏（默认掩码模式）
    pub fn desensitize_docx(bytes: &[u8], items: &[SensitiveItem]) -> Result<Vec<u8>, String> {
        Self::desensitize_docx_with_style(bytes, items, MaskStyle::Masking)
    }

    /// 针对 DOCX 文件的内存原生无损脱敏（支持指定策略）
    pub fn desensitize_docx_with_style(bytes: &[u8], items: &[SensitiveItem], style: MaskStyle) -> Result<Vec<u8>, String> {
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
                let desensitized_xml = Self::desensitize_docx_xml_with_style(&content, items, style);
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

    /// DOCX XML 段落级跨 `<w:t>` 节点合并与精准回写算法（默认掩码模式）
    pub fn desensitize_docx_xml(xml: &str, items: &[SensitiveItem]) -> String {
        Self::desensitize_docx_xml_with_style(xml, items, MaskStyle::Masking)
    }

    /// DOCX XML 段落级跨 `<w:t>` 节点合并与精准回写算法（支持指定策略）
    pub fn desensitize_docx_xml_with_style(xml: &str, items: &[SensitiveItem], style: MaskStyle) -> String {
        if items.is_empty() {
            return xml.to_string();
        }

        // 遍历所有 <w:p> 段落
        RE_P_BLOCK.replace_all(xml, |caps: &regex::Captures| {
            let p_block = &caps[0];
            Self::desensitize_xml_container(p_block, &RE_WT_TAG, items, style)
        }).into_owned()
    }

    /// 针对 XLSX 文件的内存原生无损脱敏（默认掩码模式）
    pub fn desensitize_xlsx(bytes: &[u8], items: &[SensitiveItem]) -> Result<Vec<u8>, String> {
        Self::desensitize_xlsx_with_style(bytes, items, MaskStyle::Masking)
    }

    /// 针对 XLSX 文件的内存原生无损脱敏（支持指定策略）
    pub fn desensitize_xlsx_with_style(bytes: &[u8], items: &[SensitiveItem], style: MaskStyle) -> Result<Vec<u8>, String> {
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
                let desensitized_xml = Self::desensitize_xlsx_shared_strings_with_style(&content, items, style);
                zip_out.write_all(desensitized_xml.as_bytes()).map_err(|e| format!("写入 sharedStrings.xml 失败: {e}"))?;
            } else if is_worksheet {
                let mut content = String::new();
                file.read_to_string(&mut content).map_err(|e| format!("读取 worksheet 失败 ({name}): {e}"))?;
                let desensitized_xml = Self::desensitize_xlsx_worksheet_with_style(&content, items, style);
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

    /// XLSX sharedStrings.xml 字符串池脱敏（默认掩码模式）
    pub fn desensitize_xlsx_shared_strings(xml: &str, items: &[SensitiveItem]) -> String {
        Self::desensitize_xlsx_shared_strings_with_style(xml, items, MaskStyle::Masking)
    }

    /// XLSX sharedStrings.xml 字符串池脱敏（支持指定策略）
    pub fn desensitize_xlsx_shared_strings_with_style(xml: &str, items: &[SensitiveItem], style: MaskStyle) -> String {
        if items.is_empty() {
            return xml.to_string();
        }

        RE_XLSX_SI_BLOCK.replace_all(xml, |caps: &regex::Captures| {
            let si_block = &caps[0];
            Self::desensitize_xml_container(si_block, &RE_T_TAG, items, style)
        }).into_owned()
    }

    /// XLSX worksheet 行内字符串与直接值脱敏（默认掩码模式）
    pub fn desensitize_xlsx_worksheet(xml: &str, items: &[SensitiveItem]) -> String {
        Self::desensitize_xlsx_worksheet_with_style(xml, items, MaskStyle::Masking)
    }

    /// XLSX worksheet 行内字符串与直接值脱敏（支持指定策略）
    pub fn desensitize_xlsx_worksheet_with_style(xml: &str, items: &[SensitiveItem], style: MaskStyle) -> String {
        if items.is_empty() {
            return xml.to_string();
        }
        Self::desensitize_xml_container(xml, &RE_T_TAG, items, style)
    }

    /// 针对 PPTX 演示文稿的内存原生无损脱敏（默认掩码模式）
    pub fn desensitize_pptx(bytes: &[u8], items: &[SensitiveItem]) -> Result<Vec<u8>, String> {
        Self::desensitize_pptx_with_style(bytes, items, MaskStyle::Masking)
    }

    /// 针对 PPTX 演示文稿的内存原生无损脱敏（支持指定策略）
    pub fn desensitize_pptx_with_style(bytes: &[u8], items: &[SensitiveItem], style: MaskStyle) -> Result<Vec<u8>, String> {
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
                let desensitized_xml = Self::desensitize_pptx_xml_with_style(&content, items, style);
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

    /// PPTX 幻灯片 XML 段落脱敏（默认掩码模式）
    pub fn desensitize_pptx_xml(xml: &str, items: &[SensitiveItem]) -> String {
        Self::desensitize_pptx_xml_with_style(xml, items, MaskStyle::Masking)
    }

    /// PPTX 幻灯片 XML 段落脱敏（支持指定策略）
    pub fn desensitize_pptx_xml_with_style(xml: &str, items: &[SensitiveItem], style: MaskStyle) -> String {
        if items.is_empty() {
            return xml.to_string();
        }

        RE_PPT_P_BLOCK.replace_all(xml, |caps: &regex::Captures| {
            let p_block = &caps[0];
            Self::desensitize_xml_container(p_block, &RE_AT_TAG, items, style)
        }).into_owned()
    }

    /// 通用容器内文本 Run 跨标签合并、检索与就地字符映射替换引擎
    fn desensitize_xml_container(
        container_xml: &str,
        tag_regex: &Regex,
        items: &[SensitiveItem],
        style: MaskStyle,
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

            let masked_chars: Vec<char> = style.mask_text(raw).chars().collect();

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

    /// 针对原生 PDF 文件的内存 Content Stream 指令级精准脱敏（默认掩码模式）
    pub fn desensitize_pdf(bytes: &[u8], items: &[SensitiveItem]) -> Result<Vec<u8>, String> {
        Self::desensitize_pdf_with_style(bytes, items, MaskStyle::Masking)
    }

    /// 针对 PDF 文档的全能脱敏引擎（支持原生矢量文字层与扫描件内嵌图像物理高斯模糊脱敏）
    pub fn desensitize_pdf_with_style(bytes: &[u8], items: &[SensitiveItem], style: MaskStyle) -> Result<Vec<u8>, String> {
        let mut doc = Document::load_mem(bytes).map_err(|e| format!("加载 PDF 文档失败: {e}"))?;
        let pages = doc.get_pages();

        for (&_page_num, &page_id) in &pages {
            // 1. 原生矢量/文字层 Content Stream 指令脱敏
            if let Ok(content_data) = doc.get_page_content(page_id) {
                if let Ok(mut content) = Content::decode(&content_data) {
                    let mut page_modified = false;
                    for op in &mut content.operations {
                        match op.operator.as_str() {
                            "Tj" | "'" => {
                                if let Some(obj) = op.operands.first_mut() {
                                    if Self::desensitize_pdf_object(obj, items, style) {
                                        page_modified = true;
                                    }
                                }
                            }
                            "\"" => {
                                if let Some(obj) = op.operands.get_mut(2) {
                                    if Self::desensitize_pdf_object(obj, items, style) {
                                        page_modified = true;
                                    }
                                }
                            }
                            "TJ" => {
                                if let Some(Object::Array(arr)) = op.operands.first_mut() {
                                    if Self::desensitize_pdf_tj_array(arr, items, style) {
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
            }

            // 2. 扫描件内嵌图像流物理高斯模糊脱敏
            if crate::paths::is_ocr_ready() && !items.is_empty() {
                let mut images_to_update = Vec::new();

                if let Ok(page_obj) = doc.get_object(page_id) {
                    if let Ok(page_dict) = page_obj.as_dict() {
                        let rot = crate::converter::get_page_rotation(&doc, page_dict);
                        let res_obj = page_dict.get(b"Resources").ok().and_then(|r| crate::converter::deref_obj(&doc, r));
                        if let Some(res_dict) = res_obj.and_then(|r| r.as_dict().ok()) {
                            let xobj_obj = res_dict.get(b"XObject").ok().and_then(|x| crate::converter::deref_obj(&doc, x));
                            if let Some(xobj_dict) = xobj_obj.and_then(|x| x.as_dict().ok()) {
                                for (_key, val) in xobj_dict.iter() {
                                    let target_id = match val {
                                        Object::Reference(id) => Some(*id),
                                        _ => None,
                                    };
                                    if let Some(id) = target_id {
                                        if let Ok(stream_obj) = doc.get_object(id) {
                                            if let Ok(stream) = stream_obj.as_stream() {
                                                let is_image = stream.dict.get(b"Subtype")
                                                    .ok()
                                                    .and_then(|s| crate::converter::deref_obj(&doc, s))
                                                    .and_then(|s| s.as_name().ok())
                                                    .map(|n| n == b"Image")
                                                    .unwrap_or(false);

                                                if is_image {
                                                    let is_dct = stream.dict.get(b"Filter")
                                                        .ok()
                                                        .and_then(|f| crate::converter::deref_obj(&doc, f))
                                                        .and_then(|f| f.as_name().ok())
                                                        .map(|n| n == b"DCTDecode")
                                                        .unwrap_or(false);

                                                    let loaded = if is_dct {
                                                        image::load_from_memory(&stream.content).ok()
                                                    } else if let Ok(decompressed) = stream.decompressed_content() {
                                                        image::load_from_memory(&decompressed).ok()
                                                    } else {
                                                        None
                                                    };

                                                    if let Some(img) = loaded {
                                                        if img.width() >= 100 && img.height() >= 100 {
                                                            images_to_update.push((id, img, rot, is_dct));
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
                }

                for (id, img, rot, is_dct) in images_to_update {
                    let normalized_rot = (rot % 360 + 360) % 360;
                    let ocr_img = match normalized_rot {
                        90 => img.rotate90(),
                        180 => img.rotate180(),
                        270 => img.rotate270(),
                        _ => img.clone(),
                    };

                    if let Ok(ocr_res) = crate::ocr::OcrEngine::recognize_image(&ocr_img) {
                        let rects = Self::locate_sensitive_boxes(items, &ocr_res.raw_boxes, ocr_img.width(), ocr_img.height());
                        if !rects.is_empty() {
                            let mut blurred_ocr_img = ocr_img;
                            Self::blur_image_regions(&mut blurred_ocr_img, &rects);

                            let final_img = match normalized_rot {
                                90 => blurred_ocr_img.rotate270(),
                                180 => blurred_ocr_img.rotate180(),
                                270 => blurred_ocr_img.rotate90(),
                                _ => blurred_ocr_img,
                            };

                            if let Ok(target_obj) = doc.get_object_mut(id) {
                                if let Ok(stream) = target_obj.as_stream_mut() {
                                    let mut buf = Cursor::new(Vec::new());
                                    if final_img.write_to(&mut buf, ImageFormat::Jpeg).is_ok() {
                                        stream.content = buf.into_inner();
                                        if !is_dct {
                                            stream.dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
                                        }
                                    }
                                }
                            }
                        }
                    }
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
    fn desensitize_pdf_object(obj: &mut Object, items: &[SensitiveItem], style: MaskStyle) -> bool {
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
                            let masked = style.mask_text(&item.text);
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
                        let masked = style.mask_text(&item.text);
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
                        let masked = style.mask_text(&item.text);
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
    fn desensitize_pdf_tj_array(arr: &mut Vec<Object>, items: &[SensitiveItem], style: MaskStyle) -> bool {
        let mut changed = false;
        for obj in arr.iter_mut() {
            if Self::desensitize_pdf_object(obj, items, style) {
                changed = true;
            }
        }
        changed
    }

    /// 对图像中的多个矩形 ROI 区域执行不可逆的物理高斯模糊涂抹
    /// rects 格式为 `[x, y, w, h]`
    pub fn blur_image_regions(img: &mut DynamicImage, rects: &[[u32; 4]]) {
        let (img_w, img_h) = (img.width(), img.height());
        for &[x, y, w, h] in rects {
            if w == 0 || h == 0 || x >= img_w || y >= img_h {
                continue;
            }
            let actual_w = w.min(img_w - x);
            let actual_h = h.min(img_h - y);

            // 截取 ROI 区域
            let sub = img.crop_imm(x, y, actual_w, actual_h);
            // 自适应计算模糊核 sigma：基准为框高的 0.45 倍，限制在 8.0 到 28.0 之间，确保字符笔画被彻底熔化不可逆
            let sigma = ((actual_h as f32) * 0.45).clamp(8.0, 28.0);
            let blurred = sub.blur(sigma);

            // 回写模糊后的图像到原图
            image::imageops::overlay(img, &blurred, x as i64, y as i64);
        }
    }

    /// 根据敏感词列表与 OCR 文本框列表，匹配并计算出需要模糊的矩形区域 [x, y, w, h]
    /// 遵循安全优先的整框匹配策略：只要框内文本包含敏感词（或敏感词包含该框），即对整框施加模糊
    pub fn locate_sensitive_boxes(
        items: &[SensitiveItem],
        raw_boxes: &[crate::ocr::OcrBoxItem],
        img_w: u32,
        img_h: u32,
    ) -> Vec<[u32; 4]> {
        let mut hit_rects = Vec::new();
        if items.is_empty() || raw_boxes.is_empty() {
            return hit_rects;
        }

        for b in raw_boxes {
            let b_text = b.text.trim();
            if b_text.is_empty() {
                continue;
            }

            let mut matched = false;
            for it in items {
                let s_text = it.text.trim();
                if s_text.is_empty() {
                    continue;
                }
                // 双向包含判定：应对整行大框或细粒度切分框
                if b_text.contains(s_text) || (s_text.len() >= 4 && s_text.contains(b_text)) {
                    matched = true;
                    break;
                }
            }

            if matched {
                let coords = b.box_coords;
                let min_x = coords[0].min(coords[2]);
                let min_y = coords[1].min(coords[3]);
                let max_x = coords[0].max(coords[2]);
                let max_y = coords[1].max(coords[3]);
                let h = (max_y - min_y).max(1.0);

                // 上下适度扩展 15% (至少 2px)，左右扩展 4px，防止文字边缘笔画残留
                let pad_y = (h * 0.15).max(2.0);
                let pad_x = 4.0;

                let x0 = (min_x - pad_x).max(0.0) as u32;
                let y0 = (min_y - pad_y).max(0.0) as u32;
                let x1 = (max_x + pad_x).min(img_w as f32) as u32;
                let y1 = (max_y + pad_y).min(img_h as f32) as u32;

                let w = x1.saturating_sub(x0);
                let h = y1.saturating_sub(y0);
                if w > 0 && h > 0 {
                    hit_rects.push([x0, y0, w, h]);
                }
            }
        }
        hit_rects
    }

    /// 针对常见图片（PNG/JPEG/WEBP/BMP/TIFF）的内存物理高斯模糊脱敏
    pub fn desensitize_image(
        bytes: &[u8],
        items: &[SensitiveItem],
        raw_boxes: &[crate::ocr::OcrBoxItem],
        format_hint: Option<ImageFormat>,
    ) -> Result<(Vec<u8>, &'static str), String> {
        let mut img = image::load_from_memory(bytes)
            .map_err(|e| format!("无法加载图像数据: {e}"))?;
        let (img_w, img_h) = (img.width(), img.height());

        let detected_boxes: Vec<crate::ocr::OcrBoxItem>;
        let boxes_ref = if raw_boxes.is_empty() && crate::paths::is_ocr_ready() {
            if let Ok(ocr_res) = crate::ocr::OcrEngine::recognize_image(&img) {
                detected_boxes = ocr_res.raw_boxes;
                &detected_boxes[..]
            } else {
                raw_boxes
            }
        } else {
            raw_boxes
        };

        let rects = Self::locate_sensitive_boxes(items, boxes_ref, img_w, img_h);
        if !rects.is_empty() {
            Self::blur_image_regions(&mut img, &rects);
        }

        let fmt = format_hint.or_else(|| image::guess_format(bytes).ok()).unwrap_or(ImageFormat::Png);
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, fmt)
            .map_err(|e| format!("编码脱敏图像失败: {e}"))?;

        let mime = match fmt {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Bmp => "image/bmp",
            _ => "image/png",
        };

        Ok((out.into_inner(), mime))
    }

    /// 全格式自动路由脱敏分发器（默认掩码模式）
    pub fn desensitize_document_auto(
        filename: &str,
        original_bytes: Option<&[u8]>,
        fallback_markdown: &str,
        items: &[SensitiveItem],
    ) -> Result<(String, Vec<u8>, &'static str), String> {
        Self::desensitize_document_auto_with_style(filename, original_bytes, fallback_markdown, items, MaskStyle::Masking)
    }

    /// 全格式自动路由脱敏分发器（支持指定策略）
    pub fn desensitize_document_auto_with_style(
        filename: &str,
        original_bytes: Option<&[u8]>,
        fallback_markdown: &str,
        items: &[SensitiveItem],
        style: MaskStyle,
    ) -> Result<(String, Vec<u8>, &'static str), String> {
        Self::desensitize_document_auto_detailed(filename, original_bytes, fallback_markdown, items, style, None)
    }

    /// 全格式自动路由脱敏分发器（支持指定策略与 OCR 候选框透传）
    pub fn desensitize_document_auto_detailed(
        filename: &str,
        original_bytes: Option<&[u8]>,
        fallback_markdown: &str,
        items: &[SensitiveItem],
        style: MaskStyle,
        raw_boxes: Option<&[crate::ocr::OcrBoxItem]>,
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

        // 0. 图像文件物理高斯模糊脱敏分支 (PNG/JPG/WEBP/BMP/TIFF)
        let is_img = crate::converter::DocConverter::is_image(filename, original_bytes.unwrap_or_default());
        if is_img {
            if let Some(bytes) = original_bytes {
                let empty_boxes = [];
                let boxes_slice = raw_boxes.unwrap_or(&empty_boxes);
                let (out_bytes, mime) = Self::desensitize_image(bytes, items, boxes_slice, None)?;
                let final_ext = match mime {
                    "image/jpeg" => "jpg",
                    "image/webp" => "webp",
                    "image/bmp" => "bmp",
                    _ => "png",
                };
                return Ok((format!("{stem}_脱敏.{final_ext}"), out_bytes, mime));
            }
        }

        // 1. PDF 原生/扫描件综合脱敏
        if ext == "pdf" {
            if let Some(bytes) = original_bytes {
                if let Ok(out_bytes) = Self::desensitize_pdf_with_style(bytes, items, style) {
                    return Ok((
                        format!("{stem}_脱敏.pdf"),
                        out_bytes,
                        "application/pdf",
                    ));
                }
            }
        }

        // 2. DOCX 原生脱敏
        if ext == "docx" {
            if let Some(bytes) = original_bytes {
                let out_bytes = Self::desensitize_docx_with_style(bytes, items, style)?;
                return Ok((format!("{stem}_脱敏.docx"), out_bytes, "application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
            }
        }

        // 3. XLSX 原生脱敏
        if ext == "xlsx" {
            if let Some(bytes) = original_bytes {
                let out_bytes = Self::desensitize_xlsx_with_style(bytes, items, style)?;
                return Ok((format!("{stem}_脱敏.xlsx"), out_bytes, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
            }
        }

        // 4. PPTX 原生脱敏
        if ext == "pptx" {
            if let Some(bytes) = original_bytes {
                let out_bytes = Self::desensitize_pptx_with_style(bytes, items, style)?;
                return Ok((format!("{stem}_脱敏.pptx"), out_bytes, "application/vnd.openxmlformats-officedocument.presentationml.presentation"));
            }
        }

        // 5. CSV 纯文本脱敏
        if ext == "csv" {
            let csv_src = if let Some(bytes) = original_bytes {
                let (cow, _, _) = encoding_rs::UTF_8.decode(bytes);
                cow.into_owned()
            } else {
                fallback_markdown.to_string()
            };
            let out_str = Self::desensitize_csv_with_style(&csv_src, items, style);
            return Ok((format!("{stem}_脱敏.csv"), out_str.into_bytes(), "text/csv; charset=utf-8"));
        }

        // 6. TXT 纯文本脱敏
        if ext == "txt" {
            let txt_src = if let Some(bytes) = original_bytes {
                let (cow, _, _) = encoding_rs::UTF_8.decode(bytes);
                cow.into_owned()
            } else {
                fallback_markdown.to_string()
            };
            let out_str = Self::desensitize_plain_text_with_style(&txt_src, items, style);
            return Ok((format!("{stem}_脱敏.txt"), out_str.into_bytes(), "text/plain; charset=utf-8"));
        }

        // 7. 其他所有格式（或未提供原始字节时）：平滑回退输出脱敏后的 Markdown 文档
        let desensitized_md = Self::desensitize_plain_text_with_style(fallback_markdown, items, style);
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
        assert_eq!(Desensitizer::mask_text("张建国"), "***");
        assert_eq!(Desensitizer::mask_text("13800138000"), "***********");
        assert_eq!(
            Desensitizer::mask_text("北京华云智远科技有限公司"),
            "************"
        );
    }

    #[test]
    fn test_mask_text_redaction() {
        assert_eq!(MaskStyle::Redaction.mask_text(""), "");
        assert_eq!(MaskStyle::Redaction.mask_text("李"), "█");
        assert_eq!(MaskStyle::Redaction.mask_text("张三"), "██");
        assert_eq!(MaskStyle::Redaction.mask_text("张建国"), "███");
        assert_eq!(MaskStyle::Redaction.mask_text("13800138000"), "███████████");
        assert_eq!(
            MaskStyle::Redaction.mask_text("北京华云智远科技有限公司"),
            "████████████"
        );
    }

    #[test]
    fn test_plain_text_redaction() {
        let text = "法定代表人：张建国，手机号：13800138000。";
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
        ];
        let res = Desensitizer::desensitize_plain_text_with_style(text, &items, MaskStyle::Redaction);
        assert_eq!(res, "法定代表人：███，手机号：███████████。");
    }

    #[test]
    fn test_docx_xml_redaction() {
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
        let res = Desensitizer::desensitize_docx_xml_with_style(xml, &items, MaskStyle::Redaction);
        assert_eq!(
            res,
            r#"<w:p><w:r><w:t>法定代表人：█</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>██ 先生</w:t></w:r></w:p>"#
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
            "法定代表人：***，手机号：***********，公司：************。"
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
        assert!(res.contains("<w:t>法定代表人：*</w:t>"));
        assert!(res.contains("<w:t>** 先生</w:t>"));
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
        assert!(res.contains("采购方 &amp; 甲方：************"));
        assert!(res.contains("总金额：*************"));
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
        assert!(doc_xml.contains("联系人：***"));

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
        assert!(sst_xml.contains("供应商：**************"));
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
        assert!(slide_xml.contains("汇报人：***"));
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
        assert_eq!(String::from_utf8(bytes).unwrap(), "phone: ***********");

        // CSV 测试
        let (name_csv, bytes_csv, mime_csv) = Desensitizer::desensitize_document_auto(
            "records.csv",
            Some(b"id,phone\n1,13800138000\n"),
            "",
            &items,
        ).unwrap();
        assert_eq!(name_csv, "records_脱敏.csv");
        assert_eq!(mime_csv, "text/csv; charset=utf-8");
        assert_eq!(String::from_utf8(bytes_csv).unwrap(), "id,phone\n1,***********\n");

        // Markdown 回退测试 (未提供原始字节时)
        let (name_md, bytes_md, mime_md) = Desensitizer::desensitize_document_auto(
            "unknown.xyz",
            None,
            "# 合同内容\n联系电话：13800138000",
            &items,
        ).unwrap();
        assert_eq!(name_md, "unknown_脱敏.md");
        assert_eq!(mime_md, "text/markdown; charset=utf-8");
        assert_eq!(String::from_utf8(bytes_md).unwrap(), "# 合同内容\n联系电话：***********");
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

        assert!(all_texts.iter().any(|t| t.contains("***********")));
        assert!(all_texts.iter().any(|t| t.contains("************")));
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

    #[test]
    fn test_locate_sensitive_boxes() {
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
        ];

        let raw_boxes = vec![
            crate::ocr::OcrBoxItem {
                text: "电话: 13800138000".into(),
                score: 0.98,
                box_coords: [50.0, 100.0, 250.0, 140.0],
            },
            crate::ocr::OcrBoxItem {
                text: "地址: 北京市朝阳区建国路".into(),
                score: 0.95,
                box_coords: [50.0, 160.0, 300.0, 200.0],
            },
        ];

        let rects = Desensitizer::locate_sensitive_boxes(&items, &raw_boxes, 800, 600);
        assert_eq!(rects.len(), 1);
        let [x, y, w, h] = rects[0];
        assert!(x <= 50);
        assert!(y <= 100);
        assert!(w >= 200);
        assert!(h >= 40);
    }

    #[test]
    fn test_image_physical_blur_desensitization() {
        // 创建一个包含不同颜色像素的 200x200 测试图像
        let mut img = image::RgbImage::new(200, 200);
        for y in 0..200 {
            for x in 0..200 {
                if x >= 50 && x <= 150 && y >= 50 && y <= 100 {
                    img.put_pixel(x, y, image::Rgb([0, 0, 0])); // 黑色文字区域
                } else {
                    img.put_pixel(x, y, image::Rgb([255, 255, 255])); // 白色背景
                }
            }
        }

        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Png).unwrap();
        let png_bytes = buf.into_inner();

        let items = vec![SensitiveItem {
            id: "1".into(),
            text: "机密身份证".into(),
            category: "身份证".into(),
            priority: "high".into(),
            count: 1,
            positions: vec![],
            source: "ai".into(),
        }];

        let raw_boxes = vec![crate::ocr::OcrBoxItem {
            text: "机密身份证 110101".into(),
            score: 0.99,
            box_coords: [50.0, 50.0, 150.0, 100.0],
        }];

        let (blurred_bytes, mime) = Desensitizer::desensitize_image(
            &png_bytes,
            &items,
            &raw_boxes,
            Some(ImageFormat::Png),
        ).unwrap();

        assert_eq!(mime, "image/png");
        assert!(!blurred_bytes.is_empty());

        // 验证高斯模糊后的图像在原黑色区域的像素值已被平滑模糊（不再是全纯黑 [0,0,0]）
        let blurred_img = image::load_from_memory(&blurred_bytes).unwrap();
        let pixel = blurred_img.get_pixel(52, 52);
        // 边缘过度像素不再是全纯黑
        assert_ne!(pixel, image::Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn test_auto_dispatch_image_file() {
        let img = image::RgbImage::new(100, 100);
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Png).unwrap();
        let png_bytes = buf.into_inner();

        let items = vec![SensitiveItem {
            id: "1".into(),
            text: "test".into(),
            category: "test".into(),
            priority: "low".into(),
            count: 1,
            positions: vec![],
            source: "regex".into(),
        }];

        let (out_name, out_bytes, mime) = Desensitizer::desensitize_document_auto(
            "receipt.png",
            Some(&png_bytes),
            "",
            &items,
        ).unwrap();

        assert_eq!(out_name, "receipt_脱敏.png");
        assert_eq!(mime, "image/png");
        assert!(!out_bytes.is_empty());
    }
}
