//! Crash recovery: apps write their unsaved document here every minute and clear it on a clean
//! save or exit, so a file left behind means the previous session ended unexpectedly.

use std::path::{Path, PathBuf};

const INTERVAL: f64 = 60.0;

fn dir_in(root: &Path, app: &str) -> PathBuf {
    root.join(app).join("recovery")
}

fn write_in(root: &Path, app: &str, ext: &str, bytes: &[u8]) -> std::io::Result<()> {
    let dir = dir_in(root, app);
    std::fs::create_dir_all(&dir)?;
    let tmp = dir.join(format!("recovery.{ext}.tmp"));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, dir.join(format!("recovery.{ext}")))
}

fn pending_in(root: &Path, app: &str, ext: &str) -> Option<PathBuf> {
    let p = dir_in(root, app).join(format!("recovery.{ext}"));
    p.is_file().then_some(p)
}

fn clear_in(root: &Path, app: &str) {
    let _ = std::fs::remove_dir_all(dir_in(root, app));
}

/// The document left by a session that did not end cleanly, if any.
pub fn pending(app: &str, ext: &str) -> Option<PathBuf> {
    pending_in(&crate::data_dir(), app, ext)
}

/// Forgets the recovery copy: call after a successful save and on a clean exit.
pub fn clear(app: &str) {
    clear_in(&crate::data_dir(), app);
}

/// Times the autosave: `due` is true at most once a minute, and only while there are unsaved changes.
#[derive(Default)]
pub struct Autosave {
    last: Option<f64>,
}

impl Autosave {
    pub fn due(&mut self, now: f64, dirty: bool) -> bool {
        if !dirty {
            self.last = None;
            return false;
        }
        let start = *self.last.get_or_insert(now);
        if now - start >= INTERVAL {
            self.last = Some(now);
            return true;
        }
        false
    }

    /// Writes the unsaved document; failures are ignored, the next tick tries again.
    pub fn write(app: &str, ext: &str, bytes: &[u8]) {
        let _ = write_in(&crate::data_dir(), app, ext, bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_pending_then_clear() {
        let root = std::env::temp_dir().join(format!("ethone-recovery-test-{}", std::process::id()));
        assert!(pending_in(&root, "page", "epage").is_none());
        write_in(&root, "page", "epage", b"{}").unwrap();
        let p = pending_in(&root, "page", "epage").expect("recovery file");
        assert_eq!(std::fs::read(&p).unwrap(), b"{}");
        assert!(!p.with_extension("epage.tmp").exists());
        clear_in(&root, "page");
        assert!(pending_in(&root, "page", "epage").is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn autosave_waits_a_minute_of_unsaved_changes() {
        let mut a = Autosave::default();
        assert!(!a.due(0.0, true));
        assert!(!a.due(59.0, true));
        assert!(a.due(60.0, true));
        assert!(!a.due(61.0, true));
        assert!(a.due(120.0, true));
        assert!(!a.due(500.0, false));
        assert!(!a.due(501.0, true));
    }
}
