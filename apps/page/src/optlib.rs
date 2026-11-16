//! Page option library: every font × size × colour as a text style for the selected text (Ctrl+K).

use crate::model::Kind;
use crate::{FONTS, PageApp};
use eframe::egui;
use studio_ui::palette::{Cmd, Library, color_bank, icon_cmds, icon_file, NAMED_COLORS};

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
        for (k, (n, c)) in color_bank().iter().enumerate() {
            let hex = format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
            v.push(Cmd::new(format!("Couleur du texte « {n} » {hex}"), "Page · banque de couleurs · bloc de texte sélectionné", format!("pt:{k}")));
            v.push(Cmd::new(format!("Fond du bloc « {n} » {hex}"), "Page · banque de couleurs · bloc de texte sélectionné", format!("pb:{k}")));
        }
        v.extend(icon_cmds("Page", "pi:"));
        v.push(Cmd::new("Ouvrir un document InDesign (.idml)…", "Page · Adobe InDesign · pages, textes et images", "pg-idml"));
        Library::new(v)
    })
}

/// Applies a "pg:", "pt:" or "pb:" entry to a text block; false when it does not apply.
pub fn apply_style(kind: &mut Kind, id: &str) -> bool {
    let Some((pre, rest)) = id.split_once(':') else { return false };
    let n: Vec<usize> = rest.split(':').filter_map(|x| x.parse().ok()).collect();
    let (Kind::Text { family, size: sz, color: col, .. } | Kind::TextPath { family, size: sz, color: col, .. }) = kind else { return false };
    match (pre, n.as_slice()) {
        ("pg", [f, s, c]) => {
            let (Some(font), Some(size), Some(color)) = (FONTS.get(*f), SIZES.get(*s), COLORS.get(*c)) else { return false };
            let [r, g, b] = rgb(color);
            (*family, *sz, *col) = (font.to_string(), *size, [r, g, b, col[3]]);
            true
        }
        ("pt", [k]) => color_bank().get(*k).is_some_and(|(_, c)| {
            *col = [c[0], c[1], c[2], col[3]];
            true
        }),
        ("pb", [k]) => {
            let Some((_, c)) = color_bank().get(*k) else { return false };
            if let Kind::Text { background, .. } = kind {
                *background = Some([c[0], c[1], c[2], 255]);
                return true;
            }
            false
        }
        _ => false,
    }
}

impl PageApp {
    pub fn run_lib(&mut self, id: &str, ctx: &egui::Context) -> bool {
        if id == "pg-idml" {
            if let Some(p) = rfd::FileDialog::new().add_filter("InDesign Markup", &["idml"]).pick_file() {
                self.open_path(&p);
            }
            return true;
        }
        if let Some(k) = id.strip_prefix("pi:").and_then(|k| k.parse::<usize>().ok()) {
            match icon_file(k) {
                Ok(p) => self.place_image_path(&p),
                Err(e) => studio_ui::handoff::toast(ctx, format!("Icône : {e}"), false),
            }
            return true;
        }
        if !["pg:", "pt:", "pb:"].iter().any(|p| id.starts_with(p)) {
            return false;
        }
        let Some(i) = self.sel.filter(|i| *i < self.frames_ref().len()) else {
            studio_ui::handoff::toast(ctx, "Sélectionne d'abord un bloc de texte".into(), false);
            return true;
        };
        let label = lib().cmds.iter().find(|c| c.id == id).map(|c| c.label.clone()).unwrap_or_default();
        if apply_style(&mut self.frames()[i].kind, id) {
            studio_ui::handoff::toast(ctx, format!("{label} appliqué"), true);
        } else {
            studio_ui::handoff::toast(ctx, "Sélectionne d'abord un bloc de texte".into(), false);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn library_size() {
        assert!(super::lib().len() >= 6336 + 10_000, "{}", super::lib().len());
        let mut ran = 0;
        let mut probe = crate::model::Kind::text("Essai", 13.5, false);
        if let crate::model::Kind::Text { family, color, .. } = &mut probe {
            (*family, *color) = ("Essai".into(), [3, 1, 4, 255]);
        }
        for c in &super::lib().cmds {
            let mut k = probe.clone();
            if super::apply_style(&mut k, &c.id) {
                ran += 1;
                assert_ne!(k, probe, "{} ne change rien", c.label);
            }
        }
        assert_eq!(ran, 6336 + 10_000);
        assert_eq!(super::rgb("Azur"), [10, 132, 255]);
    }
}
