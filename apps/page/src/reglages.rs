//! Préférences propres à Page (Ctrl+,), à côté des réglages communs de la suite.

use crate::PageApp;
use studio_ui::palette::{PrefSection, toggle};

pub const PREFS: &[PrefSection] = &[
    PrefSection { title: "Repères et grille", items: &[toggle("guides", "Marges, colonnes et grille", true), toggle("snap", "Magnétisme", true), toggle("rulers", "Afficher les règles", true)] },
    PrefSection { title: "Affichage", items: &[toggle("show_notes", "Afficher les notes", true)] },
];

impl PageApp {
    /// Copies the saved preferences into the live settings (at start and after each change).
    pub fn apply_prefs(&mut self) {
        let p = &self.palette;
        self.guides = p.pref_on("guides");
        self.snap = p.pref_on("snap");
        self.rulers = p.pref_on("rulers");
        self.show_notes = p.pref_on("show_notes");
    }
}
