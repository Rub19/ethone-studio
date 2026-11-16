//! Folio option library: watermark stamps, text × colour × place × opacity (Ctrl+K).

use crate::{Folio, engine};
use eframe::egui;
use studio_ui::palette::{Cmd, Library, NAMED_COLORS};

pub const TEXTS: [&str; 30] = [
    "CONFIDENTIEL", "BROUILLON", "COPIE", "ORIGINAL", "APPROUVÉ", "REFUSÉ", "PAYÉ", "URGENT", "ANNULÉ", "ÉCHANTILLON", "NE PAS COPIER", "USAGE INTERNE", "FINAL", "VERSION 2", "À SIGNER",
    "SIGNÉ", "ARCHIVÉ", "DUPLICATA", "SPÉCIMEN", "PROVISOIRE", "VALIDÉ", "EN ATTENTE", "PERSONNEL", "SECRET", "RÉVISÉ", "PROJET", "DEVIS", "FACTURE", "REÇU", "BON POUR ACCORD",
];
pub const COLORS: [&str; 8] = ["Rouge", "Bleu roi", "Vert", "Gris", "Noir", "Orange", "Violet", "Bordeaux"];
pub const PLACES: [&str; 7] = ["en diagonale au centre", "en bas à gauche", "en bas au centre", "en bas à droite", "en haut à gauche", "en haut au centre", "en haut à droite"];
pub const OPACITIES: [u8; 4] = [15, 30, 50, 80];

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
        Library::new(v)
    })
}

pub fn stamp(t: usize, c: usize, p: usize, o: usize) -> Option<engine::Stamp> {
    let name = *COLORS.get(c)?;
    let [r, g, b] = NAMED_COLORS.iter().find(|x| x.0 == name)?.1;
    let center = p == 0;
    Some(engine::Stamp { text: TEXTS.get(t)?.to_string(), size: if center { 60.0 } else { 14.0 }, color: [r, g, b], opacity: *OPACITIES.get(o)? as f32 / 100.0, angle: if center { 45.0 } else { 0.0 }, bold: true, place: p.min(6) as u8, under: false })
}

impl Folio {
    pub fn run_lib(&mut self, id: &str, ctx: &egui::Context) -> bool {
        let Some(rest) = id.strip_prefix("fo:") else { return false };
        let n: Vec<usize> = rest.split(':').filter_map(|x| x.parse().ok()).collect();
        let (Some(&t), Some(&c), Some(&p), Some(&o)) = (n.first(), n.get(1), n.get(2), n.get(3)) else { return true };
        let Some(st) = stamp(t, c, p, o) else { return true };
        let Some(name) = self.od.as_ref().map(|d| d.name.clone()) else {
            crate::toast(ctx, "Ouvre d'abord un PDF", false);
            return true;
        };
        let all = self.scope(1);
        let label = format!("Filigrane « {} » ajouté", st.text);
        self.edit(ctx, &label, |d| engine::stamp_text(d, &all, &st, &name, None));
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn library_and_stamps() {
        assert!(super::lib().len() >= 6000, "{}", super::lib().len());
        let s = super::stamp(0, 0, 0, 1).unwrap();
        assert_eq!((s.text.as_str(), s.angle, s.opacity), ("CONFIDENTIEL", 45.0, 0.3));
        assert_eq!(super::stamp(3, 7, 6, 3).unwrap().place, 6);
        assert!(super::stamp(99, 0, 0, 0).is_none());
    }
}
