//! Préférences propres à Vector (Ctrl+,), à côté des réglages communs de la suite.

use crate::VectorApp;
use studio_ui::palette::{PrefSection, number, toggle};

pub const PREFS: &[PrefSection] = &[
    PrefSection { title: "Repères et grille", items: &[toggle("show_grid", "Afficher la grille", false), number("grid", "Pas de la grille", 2.0, 500.0, " px", 20.0), toggle("snap", "Magnétisme de la grille", false), toggle("show_guides", "Afficher les repères", true)] },
    PrefSection { title: "Unités et règles", items: &[toggle("rulers", "Afficher les règles", true), number("snap_angle", "Pas d'angle avec Maj", 1.0, 90.0, " °", 15.0)] },
];

impl VectorApp {
    /// Copies the saved preferences into the live settings (at start and after each change).
    pub fn apply_prefs(&mut self) {
        let p = &self.palette;
        self.show_grid = p.pref_on("show_grid");
        self.grid = p.pref("grid") as f32;
        self.snap = p.pref_on("snap");
        self.show_guides = p.pref_on("show_guides");
        self.rulers = p.pref_on("rulers");
        self.o.snap_angle = p.pref("snap_angle") as f32;
    }
}
