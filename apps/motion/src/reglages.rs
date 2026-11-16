//! Préférences propres à Motion (Ctrl+,), à côté des réglages communs de la suite.

use crate::Motion;
use studio_ui::palette::{PrefSection, choice, number, toggle};

pub const PREFS: &[PrefSection] = &[
    PrefSection { title: "Lecture", items: &[choice("quality", "Qualité de l'aperçu", &["Complète", "Demie", "Quart"], 0)] },
    PrefSection { title: "Repères et grille", items: &[toggle("rulers", "Afficher les règles", false), toggle("grid", "Afficher la grille", false), number("grid_div", "Divisions de la grille", 2.0, 24.0, "", 10.0), choice("safe_kind", "Zones de sécurité", &["Classique 90/80 %", "Moderne 93/90 %", "Réseaux sociaux 85/75 %"], 0)] },
    PrefSection { title: "Transparence et pelure", items: &[toggle("checker", "Damier sous la transparence", false), toggle("onion", "Pelure d'oignon", false), number("onion_n", "Images fantômes", 1.0, 3.0, "", 2.0)] },
];

impl Motion {
    /// Copies the saved preferences into the live settings (at start and after each change).
    pub fn apply_prefs(&mut self) {
        let p = &self.palette;
        self.quality = p.pref("quality") as usize;
        self.rulers = p.pref_on("rulers");
        self.grid = p.pref_on("grid");
        self.grid_div = p.pref("grid_div") as u32;
        self.safe_kind = p.pref("safe_kind") as usize;
        self.checker = p.pref_on("checker");
        self.onion = p.pref_on("onion");
        self.onion_n = p.pref("onion_n") as u32;
    }
}
