//! What installing an app means on Windows, for the current user only (no administrator rights):
//! the exe in the Ethone Studio apps folder (where the launcher and "Envoyer vers" look for it),
//! Start menu and optional desktop shortcuts, and an entry in « Applications installées ».

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const NO_WINDOW: u32 = 0x0800_0000;
pub const UNINSTALLER: &str = "uninstall.exe";

pub const APPS: [(&str, &str, &str); 9] = [
    ("pixel", "Pixel", "Retouche photo et calques"),
    ("lumen", "Lumen", "Développement et bibliothèque photo"),
    ("vector", "Vector", "Dessin vectoriel et logos"),
    ("cut", "Cut", "Montage vidéo"),
    ("motion", "Motion", "Animation et effets animés"),
    ("page", "Page", "Mise en page et publication"),
    ("folio", "Folio", "PDF, signatures et OCR"),
    ("sound", "Sound", "Édition audio"),
    ("ethone-studio", "Ethone Studio", "Lanceur de la suite"),
];

pub fn name(id: &str) -> String {
    let n = APPS.iter().find(|a| a.0 == id).map_or(id, |a| a.1);
    if n.starts_with("Ethone") { n.to_string() } else { format!("Ethone {n}") }
}

pub fn dir(id: &str) -> PathBuf {
    studio_core::app_dir(id)
}

pub fn exe(id: &str) -> PathBuf {
    dir(id).join(format!("{id}.exe"))
}

fn reg_key(id: &str) -> String {
    format!(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\EthoneStudio.{id}")
}

fn hidden(cmd: &str) -> Command {
    let mut c = Command::new(cmd);
    c.creation_flags(NO_WINDOW);
    c
}

fn run(c: &mut Command) -> Result<(), String> {
    let out = c.output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

fn ps_quote(p: &Path) -> String {
    p.display().to_string().replace('\'', "''")
}

pub fn installed_version(id: &str) -> Option<String> {
    let out = hidden("reg").args(["query", &reg_key(id), "/v", "DisplayVersion"]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().find(|l| l.contains("DisplayVersion")).and_then(|l| l.split_whitespace().last()).map(str::to_string)
}

pub fn close_app(id: &str) {
    let _ = hidden("taskkill").args(["/F", "/IM", &format!("{id}.exe")]).output();
    std::thread::sleep(std::time::Duration::from_millis(300));
}

pub fn shortcuts(id: &str, desktop: bool) -> Result<(), String> {
    let places = if desktop { "[Environment]::GetFolderPath('Programs'),[Environment]::GetFolderPath('Desktop')" } else { "[Environment]::GetFolderPath('Programs')" };
    let script = format!(
        "$s=New-Object -ComObject WScript.Shell;foreach($d in {places}){{$l=$s.CreateShortcut((Join-Path $d '{}.lnk'));$l.TargetPath='{}';$l.WorkingDirectory='{}';$l.Save()}}",
        name(id).replace('\'', "''"),
        ps_quote(&exe(id)),
        ps_quote(&dir(id))
    );
    run(hidden("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]))
}

pub fn remove_shortcuts(id: &str) {
    let script = format!("foreach($d in [Environment]::GetFolderPath('Programs'),[Environment]::GetFolderPath('Desktop')){{Remove-Item (Join-Path $d '{}.lnk') -ErrorAction SilentlyContinue}};exit 0", name(id).replace('\'', "''"));
    let _ = run(hidden("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]));
}

pub fn register(id: &str, version: &str) -> Result<(), String> {
    let key = reg_key(id);
    let un = dir(id).join(UNINSTALLER).display().to_string();
    let size = std::fs::metadata(exe(id)).map(|m| m.len() / 1024).unwrap_or(0).to_string();
    let values = [
        ("DisplayName", "REG_SZ", name(id)),
        ("DisplayVersion", "REG_SZ", version.to_string()),
        ("Publisher", "REG_SZ", "Ethone".to_string()),
        ("DisplayIcon", "REG_SZ", exe(id).display().to_string()),
        ("InstallLocation", "REG_SZ", dir(id).display().to_string()),
        ("UninstallString", "REG_SZ", format!("\"{un}\" --uninstall")),
        ("QuietUninstallString", "REG_SZ", format!("\"{un}\" --uninstall /S")),
        ("EstimatedSize", "REG_DWORD", size),
        ("NoModify", "REG_DWORD", "1".into()),
        ("NoRepair", "REG_DWORD", "1".into()),
    ];
    for (name, kind, value) in values {
        run(hidden("reg").args(["add", &key, "/v", name, "/t", kind, "/d", &value, "/f"]))?;
    }
    Ok(())
}

pub fn unregister(id: &str) {
    let _ = hidden("reg").args(["delete", &reg_key(id), "/f"]).output();
}

pub fn launch(id: &str) {
    let _ = Command::new(exe(id)).current_dir(dir(id)).spawn();
}

/// The running uninstaller cannot delete itself: cmd removes it (and the folder if now empty) once it exits.
pub fn remove_self_later() {
    let Ok(me) = std::env::current_exe() else { return };
    if me.file_name().and_then(|n| n.to_str()) != Some(UNINSTALLER) {
        return;
    }
    let folder = me.parent().map(|p| p.display().to_string()).unwrap_or_default();
    let line = format!("ping -n 3 127.0.0.1 >nul & del /f /q \"{}\" & rmdir \"{folder}\"", me.display());
    let _ = hidden("cmd").arg("/C").raw_arg(&line).spawn();
}

/// (app, its own project types opened by default, other types offered in « Ouvrir avec » only).
const TYPES: [(&str, &[&str], &[&str]); 8] = [
    ("pixel", &["epix"], &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "gif", "psd"]),
    ("lumen", &[], &["jpg", "jpeg", "png", "tif", "tiff", "psd", "dng", "cr2", "cr3", "nef", "arw", "raf", "orf", "rw2"]),
    ("vector", &["evec"], &["svg"]),
    ("cut", &["ecut"], &["mp4", "mov", "mkv", "webm", "avi", "prproj"]),
    ("motion", &["emot"], &[]),
    ("page", &["epage"], &["idml"]),
    ("folio", &[], &["pdf", "ai"]),
    ("sound", &[], &["wav", "mp3", "flac", "ogg", "m4a", "aac"]),
];

fn classes(path: &str) -> String {
    format!(r"HKCU\Software\Classes\{path}")
}

fn prog_id(id: &str) -> String {
    format!("EthoneStudio.{id}")
}

pub fn associate(id: &str) -> Result<(), String> {
    let Some((_, own, other)) = TYPES.iter().find(|t| t.0 == id) else { return Ok(()) };
    let pid = prog_id(id);
    let exe = exe(id).display().to_string();
    let base = classes(&pid);
    run(hidden("reg").args(["add", &base, "/ve", "/d", &name(id), "/f"]))?;
    run(hidden("reg").args(["add", &format!(r"{base}\DefaultIcon"), "/ve", "/d", &format!("{exe},0"), "/f"]))?;
    run(hidden("reg").args(["add", &format!(r"{base}\shell\open\command"), "/ve", "/d", &format!("\"{exe}\" \"%1\""), "/f"]))?;
    for ext in own.iter().chain(other.iter()) {
        run(hidden("reg").args(["add", &classes(&format!(r".{ext}\OpenWithProgids")), "/v", &pid, "/t", "REG_NONE", "/f"]))?;
    }
    for ext in own.iter() {
        run(hidden("reg").args(["add", &classes(&format!(".{ext}")), "/ve", "/d", &pid, "/f"]))?;
    }
    Ok(())
}

pub fn dissociate(id: &str) {
    let Some((_, own, other)) = TYPES.iter().find(|t| t.0 == id) else { return };
    let pid = prog_id(id);
    for ext in own.iter().chain(other.iter()) {
        let _ = hidden("reg").args(["delete", &classes(&format!(r".{ext}\OpenWithProgids")), "/v", &pid, "/f"]).output();
    }
    for ext in own.iter() {
        let _ = hidden("reg").args(["delete", &classes(&format!(".{ext}")), "/ve", "/f"]).output();
    }
    let _ = hidden("reg").args(["delete", &classes(&pid), "/f"]).output();
}

/// Keeps the launcher's list of installed apps in step with the installer.
pub fn track(id: &str, version: &str) {
    let mut i = studio_core::Installed::load();
    i.apps.insert(id.to_string(), version.to_string());
    let _ = i.save();
}

pub fn untrack(id: &str) {
    let mut i = studio_core::Installed::load();
    i.apps.remove(id);
    let _ = i.save();
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_app_with_types_is_known_and_no_type_is_owned_twice() {
        let mut owned = std::collections::HashSet::new();
        for (id, own, _) in super::TYPES {
            assert!(super::APPS.iter().any(|a| a.0 == id), "{id}");
            for ext in own {
                assert!(owned.insert(ext), ".{ext} associé deux fois");
            }
        }
    }
}
