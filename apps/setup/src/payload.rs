//! Self-extracting layout: [installer exe][gzip app exe][footer]. The footer is
//! version (16 bytes) · app id (16 bytes) · stub length (u64 LE) · payload length (u64 LE) · "ETHSETUP".

use flate2::read::GzDecoder;
use std::io::{Read, Write};
use std::path::Path;

pub const MAGIC: &[u8; 8] = b"ETHSETUP";
const FOOTER: usize = 16 + 16 + 8 + 8 + 8;

#[derive(Debug, PartialEq)]
pub struct Footer {
    pub version: String,
    pub id: String,
    pub stub_len: u64,
    pub payload_len: u64,
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).trim_end_matches('\0').to_string()
}

pub fn footer(file: &[u8]) -> Option<Footer> {
    let f = file.get(file.len().checked_sub(FOOTER)?..)?;
    if &f[48..56] != MAGIC {
        return None;
    }
    let stub_len = u64::from_le_bytes(f[32..40].try_into().ok()?);
    let payload_len = u64::from_le_bytes(f[40..48].try_into().ok()?);
    (stub_len + payload_len + FOOTER as u64 == file.len() as u64).then(|| Footer { version: text(&f[..16]), id: text(&f[16..32]), stub_len, payload_len })
}

fn field(s: &str) -> [u8; 16] {
    let mut b = [0u8; 16];
    let src = s.as_bytes();
    b[..src.len().min(16)].copy_from_slice(&src[..src.len().min(16)]);
    b
}

/// Builds an installer from the stub (this program) and an app executable.
pub fn pack(stub: &[u8], app_exe: &[u8], id: &str, version: &str) -> Vec<u8> {
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    let _ = gz.write_all(app_exe);
    let payload = gz.finish().unwrap_or_default();
    let mut out = Vec::with_capacity(stub.len() + payload.len() + FOOTER);
    out.extend_from_slice(stub);
    out.extend_from_slice(&payload);
    out.extend_from_slice(&field(version));
    out.extend_from_slice(&field(id));
    out.extend_from_slice(&(stub.len() as u64).to_le_bytes());
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(MAGIC);
    out
}

/// Decompresses the payload to `target`, through a temporary file so a failure never leaves half an exe.
pub fn extract(file: &[u8], f: &Footer, target: &Path, report: &dyn Fn(f32)) -> Result<(), String> {
    let start = f.stub_len as usize;
    let gz = file.get(start..start + f.payload_len as usize).ok_or("installateur incomplet")?;
    let tmp = target.with_extension("new");
    let mut out = std::fs::File::create(&tmp).map_err(|e| format!("écriture impossible dans {} : {e}", tmp.display()))?;
    let mut dec = GzDecoder::new(gz);
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0usize;
    loop {
        let n = dec.read(&mut buf).map_err(|e| format!("installateur corrompu : {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n;
        report((done as f32 / (f.payload_len as f32 * 2.5)).min(0.95));
    }
    drop(out);
    for attempt in 0..40 {
        match std::fs::rename(&tmp, target) {
            Ok(()) => return Ok(()),
            Err(e) if attempt == 39 => {
                let _ = std::fs::remove_file(&tmp);
                return Err(format!("l'application est encore ouverte, ferme-la puis réessaie ({e})"));
            }
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(250)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_footer_extract_round_trip() {
        let stub = b"MZ fake installer stub".to_vec();
        let app: Vec<u8> = (0..200_000u32).map(|i| (i * 31 % 251) as u8).collect();
        let file = pack(&stub, &app, "pixel", "0.8.0");
        let f = footer(&file).expect("pied d'installateur");
        assert_eq!((f.id.as_str(), f.version.as_str(), f.stub_len), ("pixel", "0.8.0", stub.len() as u64));
        let dir = std::env::temp_dir().join(format!("ethone-setup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("pixel.exe");
        extract(&file, &f, &target, &|_| {}).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), app);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(footer(b"just some exe without footer").is_none());
        assert!(footer(&file[..file.len() - 1]).is_none());
    }
}
