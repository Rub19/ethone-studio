use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

pub const MANIFEST_URL: &str =
    "https://github.com/Rub19/ethone-studio/releases/latest/download/manifest.json";
const BUNDLED_CATALOG: &str = include_str!("../../../catalog.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub category: String,
    pub accent: [u8; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

impl AppEntry {
    pub fn published(&self) -> bool {
        self.url.is_some() && self.sha256.is_some() && self.version.is_some()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub apps: Vec<AppEntry>,
}

impl Catalog {
    pub fn bundled() -> Self {
        serde_json::from_str(BUNDLED_CATALOG).expect("catalog.json invalide")
    }

    pub fn fetch() -> Result<Self, String> {
        let mut res = ureq::get(MANIFEST_URL).call().map_err(|e| match e {
            ureq::Error::StatusCode(404) => "catalogue pas encore publié".to_string(),
            e => e.to_string(),
        })?;
        let text = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("EthoneStudio")
}

pub fn app_dir(id: &str) -> PathBuf {
    data_dir().join("apps").join(id)
}

pub fn exe_path(id: &str) -> PathBuf {
    app_dir(id).join(format!("{id}{}", std::env::consts::EXE_SUFFIX))
}

#[derive(Default, Serialize, Deserialize)]
pub struct Installed {
    pub apps: BTreeMap<String, String>,
}

impl Installed {
    fn path() -> PathBuf {
        data_dir().join("installed.json")
    }

    pub fn load() -> Self {
        let mut s: Self = fs::read(Self::path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        s.apps.retain(|id, _| exe_path(id).exists());
        s
    }

    pub fn save(&self) -> std::io::Result<()> {
        fs::create_dir_all(data_dir())?;
        fs::write(Self::path(), serde_json::to_vec_pretty(self)?)
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn install(app: &AppEntry, progress: impl Fn(u64, u64)) -> Result<(), String> {
    let (Some(url), Some(expected), Some(version)) = (&app.url, &app.sha256, &app.version) else {
        return Err("Application pas encore publiée".into());
    };
    let dir = app_dir(&app.id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("download.part");

    let mut res = ureq::get(url).call().map_err(|e| e.to_string())?;
    let total = res.body().content_length().or(app.size).unwrap_or(0);
    let mut reader = res.body_mut().as_reader();
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        progress(done, total);
    }
    drop(file);

    if !hex(&hasher.finalize()).eq_ignore_ascii_case(expected) {
        let _ = fs::remove_file(&tmp);
        return Err("Fichier corrompu (SHA-256 différent)".into());
    }

    let exe = exe_path(&app.id);
    let _ = fs::remove_file(&exe);
    fs::rename(&tmp, &exe).map_err(|e| e.to_string())?;

    let mut installed = Installed::load();
    installed.apps.insert(app.id.clone(), version.clone());
    installed.save().map_err(|e| e.to_string())
}

pub fn uninstall(id: &str) -> Result<(), String> {
    let dir = app_dir(id);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let mut installed = Installed::load();
    installed.apps.remove(id);
    installed.save().map_err(|e| e.to_string())
}

pub fn launch(id: &str) -> Result<(), String> {
    std::process::Command::new(exe_path(id))
        .current_dir(app_dir(id))
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_and_hash() {
        let c = Catalog::bundled();
        assert!(c.apps.len() >= 5);
        assert!(c.apps.iter().all(|a| !a.published()));
        assert_eq!(
            hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
