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

pub fn exe(id: &str) -> Option<PathBuf> {
    let installed = exe_path(id);
    if installed.is_file() {
        return Some(installed);
    }
    let here = std::env::current_exe().ok()?.parent()?.join(format!("{id}{}", std::env::consts::EXE_SUFFIX));
    here.is_file().then_some(here)
}

/// One delivery between apps, as kept in the transfer history.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transfer {
    /// Unix time in seconds.
    pub at: u64,
    pub from: String,
    pub to: String,
    pub file: PathBuf,
}

const HISTORY_MAX: usize = 50;

fn history_path() -> PathBuf {
    root().join("history.json")
}

/// Newest first, without duplicates of the same file sent to the same app, at most `max` entries.
pub fn push_history(list: &mut Vec<Transfer>, t: Transfer, max: usize) {
    list.retain(|x| !(x.file == t.file && x.to == t.to));
    list.insert(0, t);
    list.truncate(max);
}

/// Recent transfers between the suite's apps, newest first.
pub fn history() -> Vec<Transfer> {
    std::fs::read_to_string(history_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn clear_history() {
    let _ = std::fs::remove_file(history_path());
}

fn this_app() -> String {
    std::env::current_exe().ok().and_then(|e| e.file_stem().map(|s| s.to_string_lossy().into_owned())).map_or("ethone".into(), |s| if s == "ethone-studio" { "launcher".into() } else { s })
}

fn record(target: &str, file: &Path) {
    let mut list = history();
    let at = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    push_history(&mut list, Transfer { at, from: this_app(), to: target.into(), file: file.to_path_buf() }, HISTORY_MAX);
    let _ = std::fs::create_dir_all(root());
    let _ = std::fs::write(history_path(), serde_json::to_string(&list).unwrap_or_default());
}

/// "il y a 3 min", "hier"… for a Unix time in seconds.
pub fn ago(at: u64, now: u64) -> String {
    let d = now.saturating_sub(at);
    match d {
        0..60 => "à l'instant".into(),
        60..3600 => format!("il y a {} min", d / 60),
        3600..86_400 => format!("il y a {} h", d / 3600),
        86_400..172_800 => "hier".into(),
        _ => format!("il y a {} jours", d / 86_400),
    }
}

pub fn send(target: &str, file: &Path) -> Result<String, String> {
    let r = deliver(target, file);
    if r.is_ok() {
        record(target, file);
    }
    r
}

fn deliver(target: &str, file: &Path) -> Result<String, String> {
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

/// Starts an app with extra command-line arguments (e.g. a file and a mode flag).
pub fn open_with(target: &str, args: &[std::ffi::OsString]) -> Result<String, String> {
    let exe = exe(target).ok_or_else(|| format!("{} n'est pas installé : installe-le depuis le launcher", name(target)))?;
    let cwd = if app_dir(target).is_dir() { app_dir(target) } else { exe.parent().map(Path::to_path_buf).unwrap_or_default() };
    std::process::Command::new(&exe).args(args).current_dir(cwd).spawn().map_err(|e| e.to_string())?;
    if let Some(f) = args.iter().rev().map(Path::new).find(|p| p.is_file()) {
        record(target, f);
    }
    Ok(format!("{} s'ouvre", name(target)))
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
mod history_tests {
    use super::*;

    #[test]
    fn newest_first_unique_bounded() {
        let t = |at: u64, to: &str, f: &str| Transfer { at, from: "pixel".into(), to: to.into(), file: PathBuf::from(f) };
        let mut l = Vec::new();
        push_history(&mut l, t(1, "lumen", "a.png"), 3);
        push_history(&mut l, t(2, "page", "a.png"), 3);
        push_history(&mut l, t(3, "lumen", "a.png"), 3);
        assert_eq!(l.iter().map(|x| x.at).collect::<Vec<_>>(), vec![3, 2]);
        push_history(&mut l, t(4, "cut", "b.png"), 3);
        push_history(&mut l, t(5, "cut", "c.png"), 3);
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].at, 5);
        assert_eq!(ago(100, 130), "à l'instant");
        assert_eq!(ago(0, 7_200), "il y a 2 h");
        assert_eq!(ago(0, 100_000), "hier");
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
