//! PDF 纯文本抽取（固定版式 → 可重排阅读）。
use std::path::Path;

pub fn extract_text(path: &Path) -> Result<String, String> {
    let text = pdf_extract::extract_text(path).map_err(|e| format!("PDF 解析失败: {e}"))?;
    let mut out = String::new();
    let mut blank = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            blank += 1;
            if blank <= 1 {
                out.push('\n');
            }
        } else {
            blank = 0;
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }
    Ok(out.trim().to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_file_errors() {
        let err = super::extract_text(std::path::Path::new("/no/such/file.pdf")).unwrap_err();
        assert!(!err.is_empty());
    }
}
