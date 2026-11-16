//! Préférences propres à Lumen (Ctrl+,), à côté des réglages communs de la suite.

use crate::Lumen;
use studio_ui::palette::{PrefSection, number, toggle};

pub const PREFS: &[PrefSection] = &[
    PrefSection { title: "Bibliothèque", items: &[number("thumb_size", "Taille des vignettes", 120.0, 340.0, " px", 200.0)] },
    PrefSection { title: "Développement", items: &[toggle("show_overlay", "Superposition de recadrage", false)] },
];

impl Lumen {
    /// Copies the saved preferences into the live settings (at start and after each change).
    pub fn apply_prefs(&mut self) {
        let p = &self.palette;
        self.thumb_size = p.pref("thumb_size") as f32;
        self.show_overlay = p.pref_on("show_overlay");
    }
}
