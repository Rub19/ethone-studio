//! Préférences propres à Cut (Ctrl+,), à côté des réglages communs de la suite.

use crate::Cut;
use studio_ui::palette::{PrefSection, choice, toggle};

pub const PREFS: &[PrefSection] = &[
    PrefSection { title: "Lecture", items: &[choice("quality", "Qualité de l'aperçu", &["Complète", "1/2", "1/4"], 0)] },
    PrefSection { title: "Montage", items: &[toggle("snap", "Magnétisme", true), toggle("show_waves", "Formes d'onde sur les clips", true), toggle("show_thumbs", "Vignettes sur les clips", true)] },
    PrefSection { title: "Repères", items: &[toggle("safe", "Zones de sécurité", false), toggle("grid", "Grille des tiers", false), toggle("guides", "Repères", false)] },
];

impl Cut {
    /// Copies the saved preferences into the live settings (at start and after each change).
    pub fn apply_prefs(&mut self) {
        let p = &self.palette;
        self.quality = p.pref("quality") as usize;
        self.snap = p.pref_on("snap");
        self.show_waves = p.pref_on("show_waves");
        self.show_thumbs = p.pref_on("show_thumbs");
        self.safe = p.pref_on("safe");
        self.grid = p.pref_on("grid");
        self.guides = p.pref_on("guides");
    }
}
