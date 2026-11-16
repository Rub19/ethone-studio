//! Préférences propres à Pixel (Ctrl+,), à côté des réglages communs de la suite.

use crate::Pixel;
use studio_ui::palette::{PrefSection, choice, number, toggle};

pub const PREFS: &[PrefSection] = &[
    PrefSection { title: "Curseurs", items: &[choice("cursor_mode", "Curseur de peinture", &["Forme", "Forme et réticule", "Précis"], 0), toggle("pixel_grid", "Grille de pixels aux forts zooms", true)] },
    PrefSection { title: "Repères et grille", items: &[toggle("show_grid", "Afficher la grille", false), number("grid", "Pas de la grille", 2.0, 1000.0, " px", 32.0), number("grid_sub", "Subdivisions", 1.0, 10.0, "", 1.0), toggle("snap", "Magnétisme", true)] },
    PrefSection { title: "Affichage", items: &[choice("pasteboard", "Plan de travail", &["Sombre", "Gris moyen", "Clair", "Noir"], 0), toggle("show_bounds", "Cadre des calques", false), number("smoothing", "Lissage du pinceau", 0.0, 95.0, " %", 0.0)] },
];

impl Pixel {
    /// Copies the saved preferences into the live settings (at start and after each change).
    pub fn apply_prefs(&mut self) {
        let p = &self.palette;
        self.cursor_mode = p.pref("cursor_mode") as usize;
        self.pixel_grid = p.pref_on("pixel_grid");
        self.show_grid = p.pref_on("show_grid");
        self.grid = p.pref("grid") as u32;
        self.grid_sub = p.pref("grid_sub") as u32;
        self.snap = p.pref_on("snap");
        self.pasteboard = p.pref("pasteboard") as usize;
        self.show_bounds = p.pref_on("show_bounds");
        self.smoothing = (p.pref("smoothing") / 100.0) as f32;
    }
}
