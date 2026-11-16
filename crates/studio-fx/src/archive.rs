//! Just enough of gzip and zip to read Adobe project files: Premiere .prproj (gzip XML) and
//! InDesign .idml (zip of XML).

use std::io::Read;

pub fn gunzip_or_raw(b: &[u8]) -> Result<String, String> {
    if b.starts_with(&[0x1f, 0x8b]) {
        let mut s = String::new();
        flate2::read::GzDecoder::new(b).read_to_string(&mut s).map_err(|e| format!("décompression impossible : {e}"))?;
        Ok(s)
    } else {
        String::from_utf8(b.to_vec()).map_err(|_| "fichier non texte".into())
    }
}

fn u16le(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as usize)
}

fn u32le(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize)
}

/// Every file of a zip archive (stored or deflated), read through the central directory.
pub fn unzip(b: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let eocd = (0..b.len().saturating_sub(21)).rev().find(|&i| b[i..].starts_with(&[0x50, 0x4b, 0x05, 0x06])).ok_or("archive zip invalide")?;
    let (count, mut i) = (u16le(b, eocd + 10).ok_or("zip tronqué")?, u32le(b, eocd + 16).ok_or("zip tronqué")?);
    let mut out = Vec::new();
    for _ in 0..count {
        if !b.get(i..).is_some_and(|r| r.starts_with(&[0x50, 0x4b, 0x01, 0x02])) {
            return Err("répertoire zip invalide".into());
        }
        let bad = || "entrée zip tronquée".to_string();
        let method = u16le(b, i + 10).ok_or_else(bad)?;
        let csize = u32le(b, i + 20).ok_or_else(bad)?;
        let (nlen, xlen, clen) = (u16le(b, i + 28).ok_or_else(bad)?, u16le(b, i + 30).ok_or_else(bad)?, u16le(b, i + 32).ok_or_else(bad)?);
        let local = u32le(b, i + 42).ok_or_else(bad)?;
        let name = String::from_utf8_lossy(b.get(i + 46..i + 46 + nlen).ok_or_else(bad)?).into_owned();
        i += 46 + nlen + xlen + clen;
        let start = local + 30 + u16le(b, local + 26).ok_or_else(bad)? + u16le(b, local + 28).ok_or_else(bad)?;
        let raw = b.get(start..start + csize).ok_or_else(bad)?;
        let data = match method {
            0 => raw.to_vec(),
            8 => {
                let mut v = Vec::new();
                flate2::read::DeflateDecoder::new(raw).read_to_end(&mut v).map_err(|e| format!("{name} : {e}"))?;
                v
            }
            m => return Err(format!("{name} : compression {m} non prise en charge")),
        };
        out.push((name, data));
    }
    Ok(out)
}

/// Text of every `<tag>…</tag>` (no nesting), XML entities decoded.
pub fn tag_texts<'a>(xml: &'a str, tag: &str) -> Vec<String> {
    let (open, close) = (format!("<{tag}>"), format!("</{tag}>"));
    let mut out = Vec::new();
    let mut rest: &'a str = xml;
    while let Some(i) = rest.find(&open) {
        rest = &rest[i + open.len()..];
        let Some(j) = rest.find(&close) else { break };
        out.push(unescape(&rest[..j]));
        rest = &rest[j..];
    }
    out
}

pub fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// Media files referenced by a Premiere project, in order of first appearance, without duplicates.
pub fn premiere_media(prproj: &[u8]) -> Result<Vec<String>, String> {
    let xml = gunzip_or_raw(prproj)?;
    if !xml.contains("<PremiereData") {
        return Err("ce n'est pas un projet Premiere Pro".into());
    }
    let mut out: Vec<String> = Vec::new();
    for tag in ["ActualMediaFilePath", "FilePath"] {
        for p in tag_texts(&xml, tag) {
            let p = p.trim().to_string();
            if p.contains(['\\', '/']) && p.contains('.') && !out.contains(&p) {
                out.push(p);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn premiere_media_from_gzip() {
        let xml = r#"<?xml version="1.0"?><PremiereData Version="3"><Media><FilePath>C:\Rush\A &amp; B.mp4</FilePath><ActualMediaFilePath>C:\Rush\A &amp; B.mp4</ActualMediaFilePath></Media><Media><ActualMediaFilePath>D:/son/musique.wav</ActualMediaFilePath></Media><FilePath>Sans titre</FilePath></PremiereData>"#;
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(xml.as_bytes()).unwrap();
        let m = premiere_media(&gz.finish().unwrap()).unwrap();
        assert_eq!(m, vec![r"C:\Rush\A & B.mp4".to_string(), r"D:/son/musique.wav".to_string()]);
        assert!(premiere_media(b"<xml/>").is_err());
    }

    #[test]
    fn unzip_stored_and_deflated() {
        let mut entries: Vec<(Vec<u8>, Vec<u8>, u16, usize)> = Vec::new();
        let mut zip = Vec::new();
        for (name, data, deflate) in [("a.txt", b"bonjour".to_vec(), false), ("dir/b.xml", b"<x>salut salut salut</x>".to_vec(), true)] {
            let body = if deflate {
                let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(&data).unwrap();
                e.finish().unwrap()
            } else {
                data.clone()
            };
            let off = zip.len();
            zip.extend([0x50, 0x4b, 0x03, 0x04]);
            zip.extend([0u8; 4]);
            zip.extend((if deflate { 8u16 } else { 0 }).to_le_bytes());
            zip.extend([0u8; 8]);
            zip.extend((body.len() as u32).to_le_bytes());
            zip.extend((data.len() as u32).to_le_bytes());
            zip.extend((name.len() as u16).to_le_bytes());
            zip.extend(0u16.to_le_bytes());
            zip.extend(name.as_bytes());
            zip.extend(&body);
            entries.push((name.as_bytes().to_vec(), body, if deflate { 8 } else { 0 }, off));
        }
        let cd = zip.len();
        for (name, body, method, off) in &entries {
            zip.extend([0x50, 0x4b, 0x01, 0x02]);
            zip.extend([0u8; 6]);
            zip.extend(method.to_le_bytes());
            zip.extend([0u8; 8]);
            zip.extend((body.len() as u32).to_le_bytes());
            zip.extend([0u8; 4]);
            zip.extend((name.len() as u16).to_le_bytes());
            zip.extend([0u8; 12]);
            zip.extend((*off as u32).to_le_bytes());
            zip.extend(name);
        }
        let cd_len = zip.len() - cd;
        zip.extend([0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0]);
        zip.extend((entries.len() as u16).to_le_bytes());
        zip.extend((entries.len() as u16).to_le_bytes());
        zip.extend((cd_len as u32).to_le_bytes());
        zip.extend((cd as u32).to_le_bytes());
        zip.extend(0u16.to_le_bytes());
        let files = unzip(&zip).unwrap();
        assert_eq!(files[0], ("a.txt".to_string(), b"bonjour".to_vec()));
        assert_eq!(files[1].1, b"<x>salut salut salut</x>".to_vec());
    }
}
