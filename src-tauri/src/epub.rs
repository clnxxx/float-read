//! 最小 EPUB → 纯文本：按 spine 顺序抽取 XHTML 文本，去标签。
use std::io::Read;
use std::path::Path;

pub fn extract_text(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("打开 EPUB 失败: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("EPUB 不是有效 ZIP: {e}"))?;

    let container = read_zip_str(&mut zip, "META-INF/container.xml")?;
    let opf_path = parse_rootfile(&container).ok_or("EPUB 缺少 OPF 路径")?;
    let opf = read_zip_str(&mut zip, &opf_path)?;
    let opf_dir = parent_of(&opf_path);

    let mut id_to_href: Vec<(String, String)> = Vec::new();
    let mut spine_hrefs: Vec<String> = Vec::new();

    for seg in split_tags(&opf) {
        if seg.starts_with("item ") {
            if let (Some(id), Some(href)) = (attr(seg, "id"), attr(seg, "href")) {
                let htmlish = href.ends_with(".xhtml")
                    || href.ends_with(".html")
                    || href.ends_with(".htm")
                    || attr(seg, "media-type").is_some_and(|t| t.contains("html"));
                if htmlish {
                    id_to_href.push((id, href));
                }
            }
        }
    }
    for seg in split_tags(&opf) {
        if seg.starts_with("itemref ") {
            if let Some(idref) = attr(seg, "idref") {
                if let Some((_, href)) = id_to_href.iter().find(|(id, _)| *id == idref) {
                    spine_hrefs.push(href.clone());
                }
            }
        }
    }
    if spine_hrefs.is_empty() {
        for (_, href) in &id_to_href {
            spine_hrefs.push(href.clone());
        }
    }
    if spine_hrefs.is_empty() {
        return Err("EPUB 未找到章节".into());
    }

    let mut parts: Vec<String> = Vec::new();
    for href in spine_hrefs {
        let full = join_path(&opf_dir, &href);
        let mut chapter: Option<String> = None;
        for cand in normalize_candidates(&full) {
            if let Ok(raw) = read_zip_bytes(&mut zip, &cand) {
                chapter = Some(html_to_text(&String::from_utf8_lossy(&raw)));
                break;
            }
        }
        if let Some(text) = chapter {
            let t = text.trim();
            if !t.is_empty() {
                parts.push(t.to_string());
            }
        }
    }

    if parts.is_empty() {
        return Err("EPUB 章节无文本".into());
    }
    Ok(parts.join("\n\n"))
}

fn read_zip_str<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &str,
) -> Result<String, String> {
    Ok(String::from_utf8_lossy(&read_zip_bytes(zip, name)?).into_owned())
}

fn read_zip_bytes<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>, String> {
    let mut f = zip
        .by_name(name)
        .map_err(|_| format!("EPUB 缺少条目: {name}"))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)
        .map_err(|e| format!("读取 {name} 失败: {e}"))?;
    Ok(buf)
}

fn parse_rootfile(container: &str) -> Option<String> {
    for seg in split_tags(container) {
        if seg.starts_with("rootfile ") {
            if let Some(full) = attr(seg, "full-path") {
                return Some(full);
            }
        }
    }
    None
}

fn split_tags(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if let Some(end) = s[i..].find('>') {
                let inner = &s[i + 1..i + end];
                if !inner.starts_with('/') && !inner.starts_with('!') && !inner.starts_with('?') {
                    out.push(inner.trim());
                }
                i += end + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let lower = tag.to_ascii_lowercase();
    let pos = lower.find(&key)?;
    let rest = &tag[pos + key.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parent_of(path: &str) -> String {
    match path.rfind('/') {
        Some(i) => path[..i].to_string(),
        None => String::new(),
    }
}

fn join_path(dir: &str, href: &str) -> String {
    if dir.is_empty() {
        return href.to_string();
    }
    if href.starts_with('/') {
        return href.trim_start_matches('/').to_string();
    }
    format!("{dir}/{href}")
}

fn normalize_candidates(path: &str) -> Vec<String> {
    let decoded = percent_decode(path);
    let mut cur = String::new();
    for seg in decoded.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            if let Some(i) = cur.rfind('/') {
                cur.truncate(i);
            } else {
                cur.clear();
            }
            continue;
        }
        if !cur.is_empty() {
            cur.push('/');
        }
        cur.push_str(seg);
    }
    vec![cur, decoded, path.to_string()]
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut tag_name = String::new();
    let mut skip_until_close: Option<String> = None;
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        if skip_until_close.is_some() {
            if c == '<' {
                let mut tag = String::new();
                while let Some(&nc) = chars.peek() {
                    tag.push(nc);
                    chars.next();
                    if nc == '>' {
                        break;
                    }
                }
                let t = tag.trim_end_matches('>').trim().to_ascii_lowercase();
                if let Some(name) = &skip_until_close {
                    if t.starts_with('/') && t[1..].starts_with(name.as_str()) {
                        skip_until_close = None;
                    }
                }
            }
            continue;
        }
        if c == '<' {
            in_tag = true;
            tag_name.clear();
            continue;
        }
        if in_tag {
            if c == '>' {
                in_tag = false;
                let t = tag_name.trim().to_ascii_lowercase();
                if t.starts_with("script") || t.starts_with("style") {
                    let name = t.split(' ').next().unwrap_or("").to_string();
                    if !t.ends_with('/') {
                        skip_until_close = Some(name);
                    }
                } else if t.starts_with("br")
                    || t == "p"
                    || t.starts_with("/p")
                    || t.starts_with("/div")
                    || t.starts_with("/h1")
                    || t.starts_with("/h2")
                    || t.starts_with("/h3")
                    || t.starts_with("/h4")
                    || t.starts_with("/section")
                    || t.starts_with("/li")
                    || t.starts_with("/tr")
                    || t.starts_with("/blockquote")
                {
                    out.push_str("\n\n");
                }
                tag_name.clear();
                continue;
            }
            tag_name.push(c);
            continue;
        }
        if c == '&' {
            let mut ent = String::from("&");
            while let Some(&nc) = chars.peek() {
                ent.push(nc);
                chars.next();
                if nc == ';' || ent.len() > 10 {
                    break;
                }
            }
            out.push_str(&decode_entity(&ent));
        } else {
            out.push(c);
        }
    }
    let mut result = String::new();
    let mut blank = 0;
    for line in out.lines() {
        if line.trim().is_empty() {
            blank += 1;
            if blank <= 1 {
                result.push('\n');
            }
        } else {
            blank = 0;
            result.push_str(line.trim());
            result.push('\n');
        }
    }
    result.trim().to_string()
}

fn decode_entity(ent: &str) -> String {
    match ent {
        "&amp;" => "&".into(),
        "&lt;" => "<".into(),
        "&gt;" => ">".into(),
        "&quot;" => "\"".into(),
        "&apos;" | "&#39;" => "'".into(),
        "&nbsp;" => " ".into(),
        "&hellip;" | "&mldr;" => "…".into(),
        "&mdash;" => "—".into(),
        "&ndash;" => "–".into(),
        s if s.starts_with("&#x") || s.starts_with("&#X") => {
            let num = s
                .trim_start_matches("&#x")
                .trim_start_matches("&#X")
                .trim_end_matches(';');
            u32::from_str_radix(num, 16)
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| s.to_string())
        }
        s if s.starts_with("&#") => {
            let num = s.trim_start_matches("&#").trim_end_matches(';');
            num.parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| s.to_string())
        }
        s => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tags_and_keeps_text() {
        let html = "<html><head><style>p{color:red}</style></head><body><h1>第一章</h1><p>你好&nbsp;世界</p><script>x=1</script></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("第一章"));
        assert!(text.contains("你好 世界"));
        assert!(!text.contains("color:red"));
        assert!(!text.contains("x=1"));
    }

    #[test]
    fn parses_rootfile_and_attrs() {
        let c = r#"<?xml version="1.0"?><container><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;
        assert_eq!(parse_rootfile(c).as_deref(), Some("OEBPS/content.opf"));
        assert_eq!(
            attr(r#"item id="c1" href="c1.xhtml""#, "id").as_deref(),
            Some("c1")
        );
    }
}
