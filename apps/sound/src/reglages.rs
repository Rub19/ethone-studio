//! Préférences propres à Sound (Ctrl+,), à côté des réglages communs de la suite.

use crate::Sound;
use studio_ui::palette::{PrefSection, choice, toggle};

pub const PREFS: &[PrefSection] = &[
    PrefSection { title: "Affichage", items: &[choice("view_mode", "Vue au démarrage", &["Forme d'onde", "Spectrogramme"], 0), toggle("spec_log", "Échelle de fréquences logarithmique", true)] },
    PrefSection { title: "Lecture", items: &[toggle("follow", "La vue suit la lecture", true)] },
];

impl Sound {
    /// Copies the saved preferences into the live settings (at start and after each change).
    pub fn apply_prefs(&mut self) {
        let p = &self.palette;
        self.view_mode = p.pref("view_mode") as usize;
        self.spec_log = p.pref_on("spec_log");
        self.follow = p.pref_on("follow");
    }
}
