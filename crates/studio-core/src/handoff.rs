use crate::{app_dir, data_dir, exe_path};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

pub const APPS: [(&str, &str); 8] = [("pixel", "Pixel"), ("lumen", "Lumen"), ("vector", "Vector"), ("cut", "Cut"), ("motion", "Motion"), ("page", "Page"), ("sound", "Sound"), ("folio", "Folio")];

pub fn name(id: &str) -> &'static str {
    APPS.iter().find(|a| a.0 == id).map_or("Ethone", |a| a.1)
}

fn root() -> PathBuf {
    data_dir().join("handoff")
}

fn stamp() -> u128 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

fn clean(s: &str) -> String {
    let c: String = s.chars().filter(|c| !"\\/:*?\"<>|".contains(*c)).collect();
    let c = c.trim();
    if c.is_empty() { "envoi".into() } else { c.chars().take(60).collect() }
}

pub fn outgoing(from: &str, stem: &str, ext: &str) -> PathBuf {
    let dir = root().join("out");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("{} (depuis {}) {}.{ext}", clean(stem), name(from), stamp() % 1_000_000))
}

fn alive(id: &str) -> bool {
    std::fs::metadata(root().join(format!("{id}.alive"))).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|d| d < Duration::from_secs(4))
}

fn exe(id: &str) -> Option<PathBuf> {
    let installed = exe_path(id);
    if installed.is_file() {
        return Some(installed);
    }
    let here = std::env::current_exe().ok()?.parent()?.join(format!("{id}{}", std::env::consts::EXE_SUFFIX));
    here.is_file().then_some(here)
}

pub fn send(target: &str, file: &Path) -> Result<String, String> {
    if alive(target) {
        let dir = root().join("inbox").join(target);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dest = dir.join(file.file_name().ok_or("fichier sans nom")?);
        std::fs::copy(file, &dest).map_err(|e| e.to_string())?;
        return Ok(format!("Envoyé à {}", name(target)));
    }
    let exe = exe(target).ok_or_else(|| format!("{} n'est pas installé : installe-le depuis le launcher", name(target)))?;
    let cwd = if app_dir(target).is_dir() { app_dir(target) } else { exe.parent().map(Path::to_path_buf).unwrap_or_default() };
    std::process::Command::new(&exe).arg(file).current_dir(cwd).spawn().map_err(|e| e.to_string())?;
    Ok(format!("{} s'ouvre avec ton envoi", name(target)))
}

pub fn received_dir(id: &str) -> PathBuf {
    root().join("received").join(id)
}

pub struct Inbox {
    id: &'static str,
    last: Option<Instant>,
}

impl Inbox {
    pub fn new(id: &'static str) -> Self {
        Inbox { id, last: None }
    }

    pub fn poll(&mut self) -> Vec<PathBuf> {
        if self.last.is_some_and(|t| t.elapsed() < Duration::from_millis(800)) {
            return Vec::new();
        }
        self.last = Some(Instant::now());
        let _ = std::fs::create_dir_all(root());
        let _ = std::fs::write(root().join(format!("{}.alive", self.id)), stamp().to_string());
        let Ok(rd) = std::fs::read_dir(root().join("inbox").join(self.id)) else { return Vec::new() };
        let out = received_dir(self.id);
        let _ = std::fs::create_dir_all(&out);
        let mut got = Vec::new();
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let dest = out.join(e.file_name());
            if std::fs::rename(&p, &dest).is_ok() {
                got.push(dest);
            }
        }
        got.sort();
        got
    }
}

impl Drop for Inbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(root().join(format!("{}.alive", self.id)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_outgoing() {
        assert_eq!(name("lumen"), "Lumen");
        assert_eq!(clean("a/b:c"), "abc");
        assert_eq!(clean("  "), "envoi");
        let p = outgoing("pixel", "Photo: été", "png");
        assert!(p.to_string_lossy().contains("Photo été (depuis Pixel)"));
        assert!(p.extension().is_some_and(|e| e == "png"));
    }

    #[test]
    fn live_delivery() {
        let mut inbox = Inbox::new("selftest");
        assert!(inbox.poll().is_empty());
        let f = outgoing("pixel", "essai", "txt");
        std::fs::write(&f, "ok").unwrap();
        assert!(send("selftest", &f).unwrap().contains("Envoyé"));
        std::thread::sleep(Duration::from_millis(850));
        let got = inbox.poll();
        assert_eq!(got.len(), 1);
        assert_eq!(std::fs::read_to_string(&got[0]).unwrap(), "ok");
        let _ = std::fs::remove_file(&got[0]);
        let _ = std::fs::remove_file(&f);
    }
}
