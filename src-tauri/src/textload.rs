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

use encoding_rs::{Encoding, GB18030, UTF_16BE, UTF_16LE, UTF_8};

/// 优先严格 UTF-8；失败则识别 UTF-16（带/不带 BOM），最后按 GB18030 解码（GBK 是其子集）。
pub fn decode_bytes(bytes: &[u8]) -> (&'static Encoding, String, bool) {
    let mut decoder = UTF_8.new_decoder();
    let mut out = String::with_capacity(bytes.len());
    let (result, _read, _had_errors) = decoder.decode_to_string(bytes, &mut out, true);
    if result == encoding_rs::CoderResult::InputEmpty {
        return (UTF_8, out, false);
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (text, enc, had_errors) = UTF_16LE.decode(bytes);
        return (enc, text.into_owned(), had_errors);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (text, enc, had_errors) = UTF_16BE.decode(bytes);
        return (enc, text.into_owned(), had_errors);
    }
    if let Some((enc, text, had_errors)) = detect_bomless_utf16(bytes) {
        return (enc, text, had_errors);
    }
    let (text, _, had_errors) = GB18030.decode(bytes);
    (GB18030, text.into_owned(), had_errors)
}

fn decode_with(enc: &'static Encoding, bytes: &[u8]) -> (String, bool) {
    // decode_to_string 不扩容：UTF-16 一个码元最多写成 3 个 UTF-8 字节
    let mut decoder = enc.new_decoder();
    let mut out = String::with_capacity((bytes.len() / 2) * 3 + 8);
    let (_result, _read, had_errors) = decoder.decode_to_string(bytes, &mut out, true);
    (out, had_errors)
}

/// 无 BOM UTF-16 启发式：
/// 1) NUL 字节密集 ⇒ UTF-16（GB18030/UTF-8 文本不含 NUL），按零字节的奇偶位置定端序；
/// 2) 否则按两个端序分别给码元打分，绝大多数落在常见文字区间且含足量非 ASCII 才认定。
/// 评分区间刻意不含谚文：GB18030 常用汉字首字节落在 0xB0–0xD7，读成 BE 码元恰好落进谚文区，
/// 保留会把 GB18030 中文误判成 UTF-16BE。
fn detect_bomless_utf16(bytes: &[u8]) -> Option<(&'static Encoding, String, bool)> {
    if bytes.len() < 4 || bytes.len() % 2 != 0 {
        return None;
    }
    let zeros = bytes.iter().filter(|&&b| b == 0).count();
    if zeros * 10 >= bytes.len() * 3 {
        let zeros_even = bytes.iter().step_by(2).filter(|&&b| b == 0).count();
        let enc = if zeros_even * 2 > zeros { UTF_16BE } else { UTF_16LE };
        let (text, had_errors) = decode_with(enc, bytes);
        return Some((enc, text, had_errors));
    }

    let units = bytes.len() / 2;
    let mut plausible = [0usize; 2];
    let mut non_ascii = [0usize; 2];
    for chunk in bytes.chunks_exact(2) {
        for (k, unit) in [
            u16::from_le_bytes([chunk[0], chunk[1]]),
            u16::from_be_bytes([chunk[0], chunk[1]]),
        ]
        .into_iter()
        .enumerate()
        {
            if is_plausible_text_unit(unit) {
                plausible[k] += 1;
                if unit >= 0x80 {
                    non_ascii[k] += 1;
                }
            }
        }
    }
    let (idx, &best) = plausible.iter().enumerate().max_by_key(|(_, &v)| v)?;
    if best * 100 >= units * 85 && non_ascii[idx] * 100 >= units * 8 {
        let enc = if idx == 0 { UTF_16LE } else { UTF_16BE };
        let (text, had_errors) = decode_with(enc, bytes);
        return Some((enc, text, had_errors));
    }
    None
}

fn is_plausible_text_unit(u: u16) -> bool {
    matches!(u,
        0x0009..=0x000D
            | 0x0020..=0x007E
            | 0x00A0..=0x00FF
            | 0x2000..=0x206F
            | 0x3000..=0x303F
            | 0x3040..=0x30FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0xFF00..=0xFFEF
    )
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
    let encoding = enc.name().to_ascii_lowercase();
    Ok(LoadedText {
        text,
        encoding: if had_errors {
            format!("{encoding}-with-errors")
        } else {
            encoding
        },
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
    fn decodes_utf16le_with_bom() {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "摸鱼看书".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let (enc, text, err) = decode_bytes(&bytes);
        assert_eq!(enc, UTF_16LE);
        assert_eq!(text, "摸鱼看书");
        assert!(!err);
    }

    #[test]
    fn decodes_bomless_utf16le() {
        let mut bytes = Vec::new();
        for unit in "第一章 摸鱼看书\n第二章".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let (enc, text, err) = decode_bytes(&bytes);
        assert_eq!(enc, UTF_16LE);
        assert_eq!(text, "第一章 摸鱼看书\n第二章");
        assert!(!err);
    }

    #[test]
    fn decodes_bomless_utf16be() {
        let mut bytes = Vec::new();
        for unit in "第一章 摸鱼看书".encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        let (enc, text, err) = decode_bytes(&bytes);
        assert_eq!(enc, UTF_16BE);
        assert_eq!(text, "第一章 摸鱼看书");
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
