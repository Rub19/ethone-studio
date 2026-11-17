//! Préférences propres à Folio (Ctrl+,), à côté des réglages communs de la suite.

use crate::Folio;
use studio_ui::palette::{PrefSection, choice, number};

/// Annotation colours offered in the preferences (same order as the choice below).
const COLORS: [[u8; 3]; 5] = [[255, 214, 10], [255, 69, 58], [10, 132, 255], [48, 209, 88], [20, 20, 24]];

pub const PREFS: &[PrefSection] = &[
    PrefSection { title: "Affichage", items: &[choice("open_fit", "Zoom à l'ouverture", &["Page entière", "Largeur de la page"], 1), choice("left_tab", "Panneau de gauche", &["Vignettes", "Signets", "Commentaires"], 0)] },
    PrefSection { title: "Annotations", items: &[choice("color", "Couleur", &["Jaune", "Rouge", "Bleu", "Vert", "Noir"], 0), number("width", "Épaisseur des traits", 0.5, 10.0, " pt", 2.0), number("text_size", "Taille du texte", 6.0, 48.0, " pt", 12.0)] },
];

impl Folio {
    /// Copies the saved preferences into the live settings (at start and after each change).
    pub fn apply_prefs(&mut self) {
        let p = &self.palette;
        self.left_tab = p.pref("left_tab") as usize;
        self.color = COLORS[(p.pref("color") as usize).min(COLORS.len() - 1)];
        self.width = p.pref("width") as f32;
        self.text_size = p.pref("text_size") as f32;
    }

    /// How a newly opened PDF is fitted: 0 whole page, 1 page width.
    pub fn open_fit(&self) -> u8 {
        self.palette.pref("open_fit") as u8
    }
}
