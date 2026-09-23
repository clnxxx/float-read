use serde::Serialize;
use std::path::Path;

use crate::epub;
use crate::pdftext;

#[derive(Debug, Serialize)]
pub struct LoadedText {
    pub text: String,
    pub encoding: String,
    pub path: String,
}

use encoding_rs::{Encoding, GB18030, UTF_8};

/// 优先严格 UTF-8；失败则按 GB18030 解码（GBK 是其子集）。
pub fn decode_bytes(bytes: &[u8]) -> (&'static Encoding, String, bool) {
    let mut decoder = UTF_8.new_decoder();
    let mut out = String::with_capacity(bytes.len());
    let (result, _read, _had_errors) = decoder.decode_to_string(bytes, &mut out, true);
    if result == encoding_rs::CoderResult::InputEmpty {
        return (UTF_8, out, false);
    }
    let (text, _, had_errors) = GB18030.decode(bytes);
    (GB18030, text.into_owned(), had_errors)
}

fn ext_lower(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// 按扩展名打开 txt / epub / pdf
pub fn load_book(path: &str) -> Result<LoadedText, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("文件不存在: {path}"));
    }
    match ext_lower(path).as_str() {
        "epub" => {
            let text = epub::extract_text(p)?;
            Ok(LoadedText {
                text,
                encoding: "epub".into(),
                path: path.to_string(),
            })
        }
        "pdf" => {
            let text = pdftext::extract_text(p)?;
            Ok(LoadedText {
                text,
                encoding: "pdf".into(),
                path: path.to_string(),
            })
        }
        _ => load_txt(path),
    }
}

pub fn load_txt(path: &str) -> Result<LoadedText, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("文件不存在: {path}"));
    }
    let bytes = std::fs::read(p).map_err(|e| format!("读取失败: {e}"))?;
    if bytes.is_empty() {
        return Ok(LoadedText {
            text: String::new(),
            encoding: "utf-8".into(),
            path: path.to_string(),
        });
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    let (enc, text, had_errors) = decode_bytes(bytes);
    if had_errors && enc == GB18030 {
        return Ok(LoadedText {
            text,
            encoding: "gb18030-with-errors".into(),
            path: path.to_string(),
        });
    }
    Ok(LoadedText {
        text,
        encoding: enc.name().to_ascii_lowercase(),
        path: path.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf8() {
        let (enc, text, err) = decode_bytes("你好，浮阅".as_bytes());
        assert_eq!(enc, UTF_8);
        assert_eq!(text, "你好，浮阅");
        assert!(!err);
    }

    #[test]
    fn decodes_gbk_fallback() {
        let gbk_bytes: [u8; 8] = [0xC3, 0xFE, 0xD3, 0xE3, 0xBF, 0xB4, 0xCA, 0xE9];
        let (enc, text, err) = decode_bytes(&gbk_bytes);
        assert_eq!(enc, GB18030);
        assert_eq!(text, "摸鱼看书");
        assert!(!err);
    }

    #[test]
    fn empty_file_ok() {
        let dir = std::env::temp_dir().join("float-read-test-empty.txt");
        std::fs::write(&dir, b"").unwrap();
        let loaded = load_txt(dir.to_str().unwrap()).unwrap();
        assert_eq!(loaded.text, "");
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn unknown_extension_falls_back_to_txt() {
        let dir = std::env::temp_dir().join("float-read-test.unknown");
        std::fs::write(&dir, "abc".as_bytes()).unwrap();
        let loaded = load_book(dir.to_str().unwrap()).unwrap();
        assert_eq!(loaded.text, "abc");
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn loads_sample_epub() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../samples/demo.epub");
        if !path.exists() {
            return;
        }
        let loaded = load_book(path.to_str().unwrap()).unwrap();
        assert!(loaded.text.contains("第一章"), "got {}", loaded.text);
        assert!(loaded.text.contains("第二章"), "got {}", loaded.text);
        assert_eq!(loaded.encoding, "epub");
    }
}
