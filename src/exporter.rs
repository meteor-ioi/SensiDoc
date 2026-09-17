use crate::extractor::SensitiveItem;

pub struct Exporter;

impl Exporter {
    /// 生成符合 RFC 4180 标准的 CSV 字符串 (含 UTF-8 BOM，防止 Excel 打开乱码)
    pub fn export_to_csv(items: &[SensitiveItem]) -> String {
        let mut csv = String::from("\u{FEFF}敏感词内容,字段分类,优先级,出现频次,提取来源\n");
        for item in items {
            let escaped_text = item.text.replace('"', "\"\"");
            let pri = match item.priority.as_str() {
                "high" => "高",
                "low" => "低",
                _ => "中",
            };
            csv.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",{},\"{}\"\n",
                escaped_text, item.category, pri, item.count, item.source
            ));
        }
        csv
    }

    /// 对原 Markdown/TXT 文本根据敏感词清单执行马赛克打码脱敏
    pub fn desensitize_text(original: &str, items: &[SensitiveItem]) -> String {
        let mut desensitized = original.to_string();
        for item in items {
            let raw = &item.text;
            if raw.is_empty() {
                continue;
            }

            let masked = "*".repeat(raw.chars().count());
            desensitized = desensitized.replace(raw, &masked);
        }
        desensitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desensitization() {
        let text = "联系人张三，电话 13800138000，身份证 110101199003072345";
        let items = vec![
            SensitiveItem {
                id: "1".into(),
                text: "张三".into(),
                category: "姓名".into(),
                priority: "low".into(),
                count: 1,
                positions: vec![],
                source: "regex".into(),
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

        let result = Exporter::desensitize_text(text, &items);
        assert!(result.contains("联系人**"));
        assert!(result.contains("***********"));
    }
}
