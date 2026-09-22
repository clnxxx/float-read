use encoding_rs::{Encoding, GB18030, UTF_8};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct LoadedText {
    pub text: String,
    pub encoding: String,
    pub path: String,
}

/// 优先严格 UTF-8；失败则按 GB18030 解码（覆盖 GBK，兼容中文 TXT）。
pub fn decode_bytes(bytes: &[u8]) -> (&'static Encoding, String, bool) {
    let mut decoder = UTF_8.new_decoder();
    let mut out = String::with_capacity(bytes.len());
    let (result, _read, _had_errors) = decoder.decode_to_string(bytes, &mut out, true);
    if result == encoding_rs::CoderResult::InputEmpty {
        return (UTF_8, out, false);
    }
    // UTF-8 失败 → GB18030（GBK 是其子集）
    let (text, _, had_errors) = GB18030.decode(bytes);
    (GB18030, text.into_owned(), had_errors)
}

pub fn load_text(path: &str) -> Result<LoadedText, String> {
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
    // 跳过 UTF-8 BOM
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF])
        .unwrap_or(&bytes);
    let (enc, text, had_errors) = decode_bytes(bytes);
    if had_errors && enc == GB18030 {
        // 仍返回可读文本，但标注 unknown，避免静默乱码
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
        // "摸鱼看书" in GBK
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
        let loaded = load_text(dir.to_str().unwrap()).unwrap();
        assert_eq!(loaded.text, "");
        let _ = std::fs::remove_file(&dir);
    }
}
