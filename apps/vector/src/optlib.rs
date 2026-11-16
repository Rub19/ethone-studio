//! Vector option library: 6 000 generated palettes, 2 359 Lucide icons (studio-ui icon bank)
//! and 5 000 named colours for fill or stroke (Ctrl+K).

use crate::VectorApp;
use eframe::egui;
use studio_ui::palette::{Cmd, Library, color_bank, color_name, icon_bank, icon_cmds};

pub const SCHEMES: [(&str, &[f32]); 10] = [
    ("monochrome", &[0.0]),
    ("analogue", &[-30.0, 0.0, 30.0]),
    ("complémentaire", &[0.0, 180.0]),
    ("triade", &[0.0, 120.0, 240.0]),
    ("tétrade", &[0.0, 60.0, 180.0, 240.0]),
    ("complémentaire divisée", &[0.0, 150.0, 210.0]),
    ("carré", &[0.0, 90.0, 180.0, 270.0]),
    ("chaud-froid", &[0.0, 30.0, 190.0, 220.0]),
    ("dégradé", &[0.0, 15.0, 30.0, 45.0, 60.0]),
    ("arc-en-ciel", &[0.0, 72.0, 144.0, 216.0, 288.0]),
];
pub const TONES: [(&str, f32, f32); 10] = [("vif", 0.85, 0.5), ("pastel", 0.6, 0.8), ("sombre", 0.6, 0.28), ("néon", 1.0, 0.55), ("terreux", 0.35, 0.45), ("doux", 0.45, 0.65), ("profond", 0.75, 0.36), ("lumineux", 0.9, 0.66), ("désaturé", 0.18, 0.55), ("contrasté", 0.8, 0.5)];

/// Five colours for a base hue (degrees), scheme and tone.
pub fn palette(hue: f32, scheme: usize, tone: usize) -> Vec<[u8; 3]> {
    let (_, offs) = SCHEMES[scheme % SCHEMES.len()];
    let (_, s, l) = TONES[tone % TONES.len()];
    (0..5)
        .map(|k| {
            let h = (hue + offs[k % offs.len()]).rem_euclid(360.0) / 360.0;
            let spread = if offs.len() == 1 { (k as f32 - 2.0) * 0.12 } else { (k / offs.len()) as f32 * 0.14 };
            let l = if tone == 9 { if k % 2 == 0 { 0.25 } else { 0.75 } } else { (l + spread).clamp(0.08, 0.94) };
            let (r, g, b) = studio_fx::hsl_to_rgb(h, s, l);
            [(r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8]
        })
        .collect()
}

fn lum(c: [u8; 3]) -> u32 {
    c[0] as u32 * 3 + c[1] as u32 * 6 + c[2] as u32
}

/// Maps each distinct colour to the palette colour of the same lightness rank (alpha kept).
pub fn remap(colors: &[[u8; 4]], pal: &[[u8; 3]]) -> Vec<[u8; 4]> {
    let mut distinct: Vec<[u8; 3]> = Vec::new();
    for c in colors {
        let rgb = [c[0], c[1], c[2]];
        if !distinct.contains(&rgb) {
            distinct.push(rgb);
        }
    }
    distinct.sort_by_key(|c| lum(*c));
    let mut sorted = pal.to_vec();
    sorted.sort_by_key(|c| lum(*c));
    colors
        .iter()
        .map(|c| {
            let rank = distinct.iter().position(|d| *d == [c[0], c[1], c[2]]).unwrap_or(0);
            let k = if distinct.len() <= 1 { sorted.len() / 2 } else { rank * (sorted.len() - 1) / (distinct.len() - 1) };
            let p = sorted[k];
            [p[0], p[1], p[2], c[3]]
        })
        .collect()
}

pub const ICON_SIZE: f32 = 192.0;

fn icons() -> &'static [(&'static str, &'static str, &'static str)] {
    icon_bank()
}

fn icon_svg(inner: &str) -> String {
    studio_ui::palette::icon_svg(inner, ICON_SIZE, [30, 30, 36])
}

pub const GRADIENTS: usize = 10_000;

/// The two bank colours of gradient `k` (always different).
pub fn gradient_pair(k: usize) -> (usize, usize) {
    let n = color_bank().len();
    let a = k % n;
    let step = if k < n { 7919 } else { 104_729 };
    let b = (a * step + n / 3 + k / n) % n;
    (a, if b == a { (a + 1) % n } else { b })
}

pub fn lib() -> &'static Library {
    static L: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    L.get_or_init(|| {
        let mut v = Vec::new();
        use crate::model::{Doc, Geom, Item};
        let opt = resvg::usvg::Options::default();
        for k in 0..GRADIENTS {
            let mut d = Doc::new(40.0, 20.0);
            d.items.push(Item { geom: Geom::Rect { x: 2.0, y: 2.0, w: 30.0, h: 10.0, r: 0.0 }, visible: true, opacity: 1.0, ..Item::blank() });
            gradient_fill(&mut d.items[0], k);
            let svg = d.to_svg();
            assert!(svg.contains("url(#g0)"), "dégradé {k} absent du SVG");
            if k % 250 == 0 {
                assert!(resvg::usvg::Tree::from_str(&svg, &opt).is_ok(), "dégradé {k} illisible");
            }
        }
        for h in (0..360).step_by(6) {
            let (r, g, b) = studio_fx::hsl_to_rgb(h as f32 / 360.0, 0.85, 0.5);
            let name = color_name([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]);
            for (s, (sn, _)) in SCHEMES.iter().enumerate() {
                for (t, (tn, _, _)) in TONES.iter().enumerate() {
                    v.push(Cmd::new(format!("Palette {name} {h}° · {sn} · {tn}"), "Vector · recolorer la sélection, sinon tout", format!("vc:{h}:{s}:{t}")));
                }
            }
        }
        for k in 0..GRADIENTS {
            let (a, b) = gradient_pair(k);
            v.push(Cmd::new(format!("Dégradé « {} » → « {} »", color_bank()[a].0, color_bank()[b].0), "Vector · banque de couleurs · remplissage de la sélection", format!("vg:{k}")));
        }
        v.extend(icon_cmds("Vector", "vi:"));
        v.push(Cmd::new("Importer un nuancier Adobe (.ase / .aco)…", "Vector · Illustrator, Photoshop, InDesign · nuancier du document", "vc-ase"));
        for (k, (n, c)) in color_bank().iter().enumerate() {
            let hex = format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
            v.push(Cmd::new(format!("Remplissage « {n} » {hex}"), "Vector · banque de couleurs · sélection", format!("vf:{k}")));
            v.push(Cmd::new(format!("Contour « {n} » {hex}"), "Vector · banque de couleurs · sélection", format!("vs:{k}")));
        }
        Library::new(v)
    })
}

pub fn gradient_fill(it: &mut crate::model::Item, k: usize) {
    let (a, b) = gradient_pair(k);
    let (ca, cb) = (color_bank()[a].1, color_bank()[b].1);
    it.fill = Some([ca[0], ca[1], ca[2], 255]);
    it.fill2 = Some([cb[0], cb[1], cb[2], 255]);
}

impl VectorApp {
    fn run_bank(&mut self, id: &str, ctx: &egui::Context) -> bool {
        if let Some((_, label, min, f)) = id.strip_prefix("tool:").and_then(|t| crate::outils::TOOLS.iter().find(|x| x.0 == t)) {
            let sel = self.selection();
            if sel.len() < *min {
                studio_ui::handoff::toast(ctx, format!("Sélectionne au moins {min} objet(s)"), false);
                return true;
            }
            self.checkpoint();
            let page = [self.doc.width, self.doc.height];
            let next = f(&mut self.doc.items, &sel, page);
            self.set_selection(next);
            studio_ui::handoff::toast(ctx, format!("{label} appliqué"), true);
            return true;
        }
        if id == "vc-ase" {
            if let Some(p) = rfd::FileDialog::new().add_filter("Nuancier Adobe", &["ase", "aco"]).pick_file() {
                match studio_fx::swatches::load(&p) {
                    Ok(c) => {
                        self.checkpoint();
                        self.doc.swatches.extend(c.iter().map(|(_, c)| [c[0], c[1], c[2], 255]));
                        studio_ui::handoff::toast(ctx, format!("{} couleurs ajoutées au nuancier", c.len()), true);
                    }
                    Err(e) => studio_ui::handoff::toast(ctx, format!("Nuancier : {e}"), false),
                }
            }
            return true;
        }
        if let Some((name, _, inner)) = id.strip_prefix("vi:").and_then(|k| k.parse::<usize>().ok()).and_then(|k| icons().get(k)) {
            let at = [(self.doc.width - ICON_SIZE) / 2.0, (self.doc.height - ICON_SIZE) / 2.0];
            self.import_svg_str(&icon_svg(inner), name, at);
            return true;
        }
        if let Some(k) = id.strip_prefix("vg:").and_then(|k| k.parse::<usize>().ok()).filter(|k| *k < GRADIENTS) {
            let targets = self.selection();
            if targets.is_empty() {
                studio_ui::handoff::toast(ctx, "Sélectionne d'abord un objet".into(), false);
                return true;
            }
            self.checkpoint();
            for i in targets {
                gradient_fill(&mut self.doc.items[i], k);
            }
            studio_ui::handoff::toast(ctx, "Dégradé appliqué".into(), true);
            return true;
        }
        let (stroke, rest) = match (id.strip_prefix("vf:"), id.strip_prefix("vs:")) {
            (Some(r), _) => (false, r),
            (_, Some(r)) => (true, r),
            _ => return false,
        };
        let Some((n, c)) = rest.parse::<usize>().ok().and_then(|k| color_bank().get(k)) else { return true };
        let targets = self.selection();
        if targets.is_empty() {
            studio_ui::handoff::toast(ctx, "Sélectionne d'abord un objet".into(), false);
            return true;
        }
        self.checkpoint();
        for i in targets {
            let it = &mut self.doc.items[i];
            let rgba = [c[0], c[1], c[2], 255];
            if stroke { it.stroke = Some(rgba) } else { it.fill = Some(rgba) }
        }
        studio_ui::handoff::toast(ctx, format!("{} « {n} » appliqué", if stroke { "Contour" } else { "Remplissage" }), true);
        true
    }

    pub fn run_lib(&mut self, id: &str, ctx: &egui::Context) -> bool {
        if self.run_bank(id, ctx) {
            return true;
        }
        let Some(rest) = id.strip_prefix("vc:") else { return false };
        let n: Vec<usize> = rest.split(':').filter_map(|x| x.parse().ok()).collect();
        let (Some(&h), Some(&s), Some(&t)) = (n.first(), n.get(1), n.get(2)) else { return true };
        let pal = palette(h as f32, s, t);
        let mut targets = self.selection();
        if targets.is_empty() {
            targets = (0..self.doc.items.len()).filter(|i| self.doc.editable(*i)).collect();
        }
        let mut colors = Vec::new();
        for i in &targets {
            let it = &self.doc.items[*i];
            colors.extend([it.fill, it.stroke, it.fill2].into_iter().flatten());
        }
        if colors.is_empty() {
            studio_ui::handoff::toast(ctx, "Aucune couleur à recolorer".into(), false);
            return true;
        }
        let mapped = remap(&colors, &pal);
        self.checkpoint();
        let mut k = 0;
        for i in &targets {
            let it = &mut self.doc.items[*i];
            for c in [&mut it.fill, &mut it.stroke, &mut it.fill2].into_iter().flatten() {
                *c = mapped[k];
                k += 1;
            }
        }
        studio_ui::handoff::toast(ctx, format!("Palette {} · {} appliquée à {} objet(s)", SCHEMES[s % 10].0, TONES[t % 10].0, targets.len()), true);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_and_remap() {
        assert_eq!(icons().len(), 2359);
        assert_eq!(lib().len(), 6000 + GRADIENTS + 2359 + 10000 + 1);
        let mut pairs = std::collections::HashSet::new();
        for k in 0..GRADIENTS {
            let (a, b) = gradient_pair(k);
            assert!(a != b && pairs.insert((a, b)), "dégradé {k} en double ou uni");
        }
        for h in (0..360).step_by(6) {
            for s in 0..SCHEMES.len() {
                for t in 0..TONES.len() {
                    let pal = palette(h as f32, s, t);
                    assert_eq!(remap(&[[10, 10, 10, 255], [240, 240, 240, 255]], &pal).len(), 2);
                }
            }
        }
        let opt = resvg::usvg::Options::default();
        for (name, _, inner) in icons() {
            let items = crate::geom::import_svg(&icon_svg(inner), &opt).unwrap_or_default();
            assert!(!items.is_empty(), "{name}");
        }
        let p = palette(0.0, 2, 0);
        assert_eq!(p.len(), 5);
        assert!(p[0][0] > 180 && p[1][2] > 150, "{p:?}");
        let colors = [[10, 10, 10, 255], [200, 200, 200, 128], [10, 10, 10, 255]];
        let pal = [[0, 0, 100], [255, 255, 0], [100, 0, 0]];
        let m = remap(&colors, &pal);
        assert_eq!(m[0], [0, 0, 100, 255]);
        assert_eq!(m[1], [255, 255, 0, 128]);
        assert_eq!(m[2], m[0]);
    }
}
