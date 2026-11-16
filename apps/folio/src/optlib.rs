//! Folio option library: watermark stamps, text × colour × place × opacity (Ctrl+K).

use crate::{Folio, engine};
use eframe::egui;
use studio_ui::palette::{Cmd, Library, NAMED_COLORS, color_bank};

pub const TEXTS: [&str; 30] = [
    "CONFIDENTIEL", "BROUILLON", "COPIE", "ORIGINAL", "APPROUVÉ", "REFUSÉ", "PAYÉ", "URGENT", "ANNULÉ", "ÉCHANTILLON", "NE PAS COPIER", "USAGE INTERNE", "FINAL", "VERSION 2", "À SIGNER",
    "SIGNÉ", "ARCHIVÉ", "DUPLICATA", "SPÉCIMEN", "PROVISOIRE", "VALIDÉ", "EN ATTENTE", "PERSONNEL", "SECRET", "RÉVISÉ", "PROJET", "DEVIS", "FACTURE", "REÇU", "BON POUR ACCORD",
];
pub const COLORS: [&str; 8] = ["Rouge", "Bleu roi", "Vert", "Gris", "Noir", "Orange", "Violet", "Bordeaux"];
pub const PLACES: [&str; 7] = ["en diagonale au centre", "en haut à gauche", "en haut au centre", "en haut à droite", "en bas à gauche", "en bas au centre", "en bas à droite"];
pub const OPACITIES: [u8; 4] = [15, 30, 50, 80];

pub const TEMPLATES: [&str; 12] = ["Page {page}", "Page {page} / {pages}", "{page}", "- {page} -", "{page} sur {pages}", "{fichier}", "{date}", "{fichier} · {date}", "{fichier} · page {page}", "Confidentiel · page {page}", "Brouillon du {date}", "Imprimé le {date}"];
pub const HF_SIZES: [f32; 7] = [7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0];
pub const HF_COLORS: [&str; 10] = ["Noir", "Anthracite", "Gris", "Marine", "Bleu roi", "Rouge", "Bordeaux", "Vert", "Orange", "Violet"];

pub fn lib() -> &'static Library {
    static L: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    L.get_or_init(|| {
        let mut v = Vec::new();
        for (t, text) in TEXTS.iter().enumerate() {
            for (c, color) in COLORS.iter().enumerate() {
                for (p, place) in PLACES.iter().enumerate() {
                    for (o, op) in OPACITIES.iter().enumerate() {
                        v.push(Cmd::new(format!("Filigrane « {text} » {} {place} · {op} %", color.to_lowercase()), "Folio · toutes les pages", format!("fo:{t}:{c}:{p}:{o}")));
                    }
                }
            }
        }
        for (t, tpl) in TEMPLATES.iter().enumerate() {
            for (p, place) in PLACES.iter().enumerate().skip(1) {
                for (s, size) in HF_SIZES.iter().enumerate() {
                    for (c, color) in HF_COLORS.iter().enumerate() {
                        let shown = tpl.replace("{page}", "n").replace("{pages}", "total").replace("{fichier}", "nom du fichier").replace("{date}", "date");
                        v.push(Cmd::new(format!("{} « {shown} » {place} · {size} pt {}", if p <= 3 { "En-tête" } else { "Pied de page" }, color.to_lowercase()), "Folio · toutes les pages", format!("fh:{t}:{p}:{s}:{c}")));
                    }
                }
            }
        }
        for (k, (n, c)) in color_bank().iter().enumerate() {
            v.push(Cmd::new(format!("Fond de page « {n} » #{:02x}{:02x}{:02x}", c[0], c[1], c[2]), "Folio · banque de couleurs · toutes les pages", format!("fb:{k}")));
        }
        Library::new(v)
    })
}

pub fn stamp(t: usize, c: usize, p: usize, o: usize) -> Option<engine::Stamp> {
    let name = *COLORS.get(c)?;
    let [r, g, b] = NAMED_COLORS.iter().find(|x| x.0 == name)?.1;
    let center = p == 0;
    Some(engine::Stamp { text: TEXTS.get(t)?.to_string(), size: if center { 60.0 } else { 14.0 }, color: [r, g, b], opacity: *OPACITIES.get(o)? as f32 / 100.0, angle: if center { 45.0 } else { 0.0 }, bold: true, place: p.min(6) as u8, under: false })
}

fn named(name: &str) -> Option<[u8; 3]> {
    NAMED_COLORS.iter().find(|x| x.0 == name).map(|x| x.1)
}

/// Applies a library entry to the listed pages of a PDF; Ok(false) when the id is not one of ours.
pub fn apply_id(d: &mut lopdf::Document, pages: &[usize], id: &str, file: &str) -> engine::R<bool> {
    let Some((pre, rest)) = id.split_once(':') else { return Ok(false) };
    let n: Vec<usize> = rest.split(':').filter_map(|x| x.parse().ok()).collect();
    match (pre, n.as_slice()) {
        ("fo", [t, c, p, o]) => {
            let st = stamp(*t, *c, *p, *o).ok_or("filigrane inconnu")?;
            engine::stamp_text(d, pages, &st, file, None)?;
        }
        ("fh", [t, p, s, c]) => {
            let st = engine::Stamp { text: TEMPLATES.get(*t).ok_or("modèle inconnu")?.to_string(), size: *HF_SIZES.get(*s).ok_or("taille inconnue")?, color: named(HF_COLORS.get(*c).ok_or("couleur inconnue")?).ok_or("couleur inconnue")?, opacity: 1.0, angle: 0.0, bold: false, place: (*p).clamp(1, 6) as u8, under: false };
            engine::stamp_text(d, pages, &st, file, None)?;
        }
        ("fb", [k]) => engine::background(d, pages, color_bank().get(*k).ok_or("couleur inconnue")?.1)?,
        _ => return Ok(false),
    }
    Ok(true)
}

impl Folio {
    pub fn run_lib(&mut self, id: &str, ctx: &egui::Context) -> bool {
        if !["fo:", "fh:", "fb:"].iter().any(|p| id.starts_with(p)) {
            return false;
        }
        let Some(name) = self.od.as_ref().map(|d| d.name.clone()) else {
            crate::toast(ctx, "Ouvre d'abord un PDF", false);
            return true;
        };
        let all = self.scope(1);
        let label = lib().cmds.iter().find(|c| c.id == id).map(|c| c.label.clone()).unwrap_or_default();
        let id = id.to_string();
        self.edit(ctx, &label, |d| apply_id(d, &all, &id, &name).map(|_| ()));
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn library_and_stamps() {
        assert!(super::lib().len() >= 6720 + 10_040, "{}", super::lib().len());
        let blank = crate::engine::images_to_pdf(&[image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 255, 255, 255]))], Some((595.0, 842.0))).unwrap();
        let mut ran = 0;
        for (k, c) in super::lib().cmds.iter().enumerate() {
            let mut d = blank.clone();
            assert!(super::apply_id(&mut d, &[0], &c.id, "essai.pdf").unwrap_or_else(|e| panic!("{} : {e}", c.label)), "{}", c.label);
            if k % 997 == 0 {
                let bytes = crate::engine::save(&mut d).unwrap();
                assert!(crate::engine::load(&bytes, "").is_ok(), "{}", c.label);
            }
            ran += 1;
        }
        assert_eq!(ran, 6720 + 10_040);
        let s = super::stamp(0, 0, 0, 1).unwrap();
        assert_eq!((s.text.as_str(), s.angle, s.opacity), ("CONFIDENTIEL", 45.0, 0.3));
        assert_eq!(super::stamp(3, 7, 6, 3).unwrap().place, 6);
        assert!(super::stamp(99, 0, 0, 0).is_none());
    }
}
