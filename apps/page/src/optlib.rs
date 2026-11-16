//! Page option library: every font × size × colour as a text style for the selected text (Ctrl+K).

use crate::model::Kind;
use crate::{FONTS, PageApp};
use eframe::egui;
use studio_ui::palette::{Cmd, Library, NAMED_COLORS};

pub const SIZES: [f32; 24] = [8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 28.0, 32.0, 36.0, 40.0, 44.0, 48.0, 54.0, 60.0, 66.0, 72.0, 84.0, 96.0, 120.0];
pub const COLORS: [&str; 12] = ["Noir", "Anthracite", "Blanc", "Rouge", "Corail", "Orange", "Or", "Vert", "Sarcelle", "Azur", "Marine", "Violet"];

fn rgb(name: &str) -> [u8; 3] {
    NAMED_COLORS.iter().find(|c| c.0 == name).map_or([0, 0, 0], |c| c.1)
}

pub fn lib() -> &'static Library {
    static L: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    L.get_or_init(|| {
        let mut v = Vec::new();
        for (f, font) in FONTS.iter().enumerate() {
            for (s, size) in SIZES.iter().enumerate() {
                for (c, color) in COLORS.iter().enumerate() {
                    v.push(Cmd::new(format!("Style de texte : {font} {size} pt {}", color.to_lowercase()), "Page · bloc de texte sélectionné", format!("pg:{f}:{s}:{c}")));
                }
            }
        }
        Library::new(v)
    })
}

impl PageApp {
    pub fn run_lib(&mut self, id: &str, ctx: &egui::Context) -> bool {
        let Some(rest) = id.strip_prefix("pg:") else { return false };
        let n: Vec<usize> = rest.split(':').filter_map(|x| x.parse().ok()).collect();
        let (Some(&f), Some(&s), Some(&c)) = (n.first(), n.get(1), n.get(2)) else { return true };
        let (Some(font), Some(size), Some(color)) = (FONTS.get(f), SIZES.get(s), COLORS.get(c)) else { return true };
        let [r, g, b] = rgb(color);
        let Some(i) = self.sel.filter(|i| *i < self.frames_ref().len()) else {
            studio_ui::handoff::toast(ctx, "Sélectionne d'abord un bloc de texte".into(), false);
            return true;
        };
        match &mut self.frames()[i].kind {
            Kind::Text { family, size: sz, color: col, .. } | Kind::TextPath { family, size: sz, color: col, .. } => {
                *family = font.to_string();
                *sz = *size;
                *col = [r, g, b, col[3]];
                studio_ui::handoff::toast(ctx, format!("Style {font} {size} pt appliqué"), true);
            }
            _ => studio_ui::handoff::toast(ctx, "Sélectionne d'abord un bloc de texte".into(), false),
        }
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn library_size() {
        assert!(super::lib().len() >= 6000, "{}", super::lib().len());
        assert_eq!(super::rgb("Azur"), [10, 132, 255]);
    }
}
