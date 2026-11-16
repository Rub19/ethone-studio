//! Pixel option library: duotones, zone tints, frames, gradient overlays, coloured vignettes and
//! pixel art palettes applied to the active layer (or selection) from Ctrl+K.

use crate::Pixel;
use eframe::egui;
use image::{Rgba, RgbaImage};
use studio_ui::palette::{Cmd, Library, NAMED_COLORS, color_bank};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Duotone(usize, usize),
    Tint(u16, u8, u8),
    Frame(usize, u8, bool),
    Gradient(usize, u8, u8),
    Vignette(usize, u8, bool),
    PixelArt(u32, usize),
    DuoBank(usize, usize),
    TintBank(usize),
}

/// A second, far-away colour of the bank for colour `i` (never `i` itself).
pub fn partner(i: usize, n: usize) -> usize {
    let j = (i * 7919 + n / 2) % n;
    if j == i { (i + 1) % n } else { j }
}

fn bank(i: usize) -> [u8; 3] {
    let b = color_bank();
    b[i % b.len()].1
}

const ZONES: [&str; 3] = ["ombres", "tons moyens", "hautes lumières"];
const DIRS: [(&str, f32, f32); 8] = [("du haut", 0.0, 1.0), ("du bas", 0.0, -1.0), ("de gauche", 1.0, 0.0), ("de droite", -1.0, 0.0), ("du coin haut gauche", 0.7, 0.7), ("du coin haut droit", -0.7, 0.7), ("du coin bas gauche", 0.7, -0.7), ("du coin bas droit", -0.7, -0.7)];
pub const PALETTES: [(&str, &[[u8; 3]]); 8] = [
    ("couleurs d'origine", &[]),
    ("console portable verte", &[[15, 56, 15], [48, 98, 48], [139, 172, 15], [155, 188, 15]]),
    ("noir et blanc", &[[0, 0, 0], [255, 255, 255]]),
    ("sépia 4 tons", &[[44, 33, 22], [112, 84, 58], [181, 146, 108], [238, 220, 190]]),
    ("8 couleurs", &[[0, 0, 0], [255, 255, 255], [220, 40, 40], [40, 180, 60], [40, 80, 220], [250, 220, 40], [240, 120, 30], [150, 70, 200]]),
    ("néon", &[[10, 8, 30], [255, 40, 160], [40, 230, 255], [250, 250, 90], [140, 60, 255]]),
    ("pastel", &[[250, 210, 220], [200, 230, 250], [210, 245, 215], [255, 240, 200], [225, 210, 250], [90, 90, 110]]),
    ("niveaux de gris", &[[0, 0, 0], [85, 85, 85], [170, 170, 170], [255, 255, 255]]),
];

fn col(i: usize) -> [u8; 3] {
    NAMED_COLORS[i % NAMED_COLORS.len()].1
}

pub fn ops() -> &'static Vec<(Op, String, &'static str)> {
    static O: std::sync::OnceLock<Vec<(Op, String, &'static str)>> = std::sync::OnceLock::new();
    O.get_or_init(|| {
        let mut v: Vec<(Op, String, &'static str)> = Vec::new();
        let low = |s: &str| s.to_lowercase();
        for a in 0..NAMED_COLORS.len() {
            for b in 0..NAMED_COLORS.len() {
                if a != b {
                    v.push((Op::Duotone(a, b), format!("Duotone {} et {}", low(NAMED_COLORS[a].0), low(NAMED_COLORS[b].0)), "Duotone · ombres puis lumières"));
                }
            }
        }
        for h in (0..360).step_by(5) {
            for s in (1..=10).map(|k| k * 10) {
                for (z, zn) in ZONES.iter().enumerate() {
                    v.push((Op::Tint(h as u16, s as u8, z as u8), format!("Teinte {h}° sur les {zn} · {s} %"), "Étalonnage"));
                }
            }
        }
        for c in 0..24 {
            for w in 1..=12u8 {
                for round in [false, true] {
                    v.push((Op::Frame(c * 2, w, round), format!("Cadre {} {w} %{}", low(NAMED_COLORS[c * 2].0), if round { " arrondi" } else { "" }), "Cadre"));
                }
            }
        }
        for c in 0..24 {
            for (d, (dn, _, _)) in DIRS.iter().enumerate() {
                for o in [20u8, 35, 50, 65, 80] {
                    v.push((Op::Gradient(c * 2 + 1, d as u8, o), format!("Dégradé {} {dn} · {o} %", low(NAMED_COLORS[c * 2 + 1].0)), "Dégradé"));
                }
            }
        }
        for c in 0..12 {
            for st in (1..=12).map(|k| k * 8) {
                for round in [true, false] {
                    v.push((Op::Vignette(c * 4, st as u8, round), format!("Vignette {} {st} %{}", low(NAMED_COLORS[c * 4].0), if round { "" } else { " rectangulaire" }), "Vignette"));
                }
            }
        }
        for b in 2..=64u32 {
            for (p, (pn, _)) in PALETTES.iter().enumerate() {
                v.push((Op::PixelArt(b, p), format!("Pixel art {b} px · {pn}"), "Rétro"));
            }
        }
        let n = color_bank().len();
        for i in 0..n {
            let (a, b) = (i, partner(i, n));
            let (a, b) = if luma([bank(a)[0], bank(a)[1], bank(a)[2], 255]) <= luma([bank(b)[0], bank(b)[1], bank(b)[2], 255]) { (a, b) } else { (b, a) };
            v.push((Op::DuoBank(a, b), format!("Duotone « {} » et « {} »", color_bank()[a].0, color_bank()[b].0), "Duotone · banque de couleurs"));
            v.push((Op::TintBank(i), format!("Voile de couleur « {} »", color_bank()[i].0), "Teinte · banque de couleurs"));
        }
        v
    })
}

pub fn lib() -> &'static Library {
    static L: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    L.get_or_init(|| {
        let ops = ops().iter().enumerate().map(|(k, (_, label, hint))| Cmd::new(label.clone(), format!("Pixel · {hint} · calque actif"), format!("pl:{k}")));
        let colors = color_bank().iter().enumerate().map(|(k, (n, c))| Cmd::new(format!("Couleur « {n} » #{:02x}{:02x}{:02x}", c[0], c[1], c[2]), "Pixel · banque de couleurs · couleur du pinceau", format!("pc:{k}")));
        let swatch = std::iter::once(Cmd::new("Importer un nuancier Adobe (.ase / .aco)…", "Pixel · Photoshop, Illustrator, InDesign · ajoute les couleurs au nuancier", "pc-ase"));
        Library::new(ops.chain(colors).chain(swatch).collect())
    })
}

fn luma(p: [u8; 4]) -> f32 {
    (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) / 255.0
}

fn mix(p: [u8; 4], c: [u8; 3], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|k| (p[k] as f32 + (c[k] as f32 - p[k] as f32) * t).round() as u8).into_iter().chain([p[3]]).collect::<Vec<_>>().try_into().unwrap_or(p)
}

fn duo(img: &RgbaImage, d: [u8; 3], l: [u8; 3]) -> RgbaImage {
    RgbaImage::from_fn(img.width(), img.height(), |x, y| {
        let p = img.get_pixel(x, y).0;
        let t = luma(p);
        let c = [0, 1, 2].map(|k| (d[k] as f32 + (l[k] as f32 - d[k] as f32) * t).round() as u8);
        Rgba([c[0], c[1], c[2], p[3]])
    })
}

pub fn apply(img: &RgbaImage, op: Op) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (fw, fh) = (w.max(1) as f32, h.max(1) as f32);
    match op {
        Op::Duotone(a, b) => duo(img, col(a), col(b)),
        Op::Tint(hue, s, zone) => {
            let (r, g, b) = studio_fx::hsl_to_rgb(hue as f32 / 360.0, 1.0, 0.5);
            let c = [r * 255.0, g * 255.0, b * 255.0];
            RgbaImage::from_fn(w, h, |x, y| {
                let p = img.get_pixel(x, y).0;
                let l = luma(p);
                let wgt = match zone {
                    0 => (1.0 - l).powi(2),
                    1 => 1.0 - (2.0 * l - 1.0).abs(),
                    _ => l * l,
                } * s as f32 / 100.0 * 0.6;
                Rgba([0, 1, 2].map(|k| (p[k] as f32 + (c[k] - 128.0) * wgt).round().clamp(0.0, 255.0) as u8).into_iter().chain([p[3]]).collect::<Vec<_>>().try_into().unwrap_or(p))
            })
        }
        Op::Frame(ci, pct, round) => {
            let c = col(ci);
            let bw = (fw.min(fh) * pct as f32 / 100.0).max(1.0);
            let r = if round { bw * 2.0 } else { 0.0 };
            RgbaImage::from_fn(w, h, |x, y| {
                let p = img.get_pixel(x, y).0;
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let (ix0, iy0, ix1, iy1) = (bw, bw, fw - bw, fh - bw);
                let dx = (ix0 + r - px).max(px - (ix1 - r)).max(0.0);
                let dy = (iy0 + r - py).max(py - (iy1 - r)).max(0.0);
                let outside = px < ix0 || py < iy0 || px > ix1 || py > iy1 || (r > 0.0 && dx > 0.0 && dy > 0.0 && dx * dx + dy * dy > r * r);
                if outside { Rgba([c[0], c[1], c[2], 255]) } else { Rgba(p) }
            })
        }
        Op::Gradient(ci, d, o) => {
            let c = col(ci);
            let (_, dx, dy) = DIRS[d as usize % DIRS.len()];
            RgbaImage::from_fn(w, h, |x, y| {
                let (u, v) = (x as f32 / fw * 2.0 - 1.0, y as f32 / fh * 2.0 - 1.0);
                let t = (1.0 - (u * dx + v * dy) / (dx.abs() + dy.abs()).max(1e-3)) / 2.0;
                Rgba(mix(img.get_pixel(x, y).0, c, t * o as f32 / 100.0))
            })
        }
        Op::Vignette(ci, st, round) => {
            let c = col(ci);
            RgbaImage::from_fn(w, h, |x, y| {
                let (u, v) = (x as f32 / fw * 2.0 - 1.0, y as f32 / fh * 2.0 - 1.0);
                let d = if round { (u * u + v * v).sqrt() / 1.414 } else { u.abs().max(v.abs()) };
                let t = ((d - 0.35) / 0.65).clamp(0.0, 1.0).powf(1.6);
                Rgba(mix(img.get_pixel(x, y).0, c, t * st as f32 / 100.0))
            })
        }
        Op::DuoBank(a, b) => duo(img, bank(a), bank(b)),
        Op::TintBank(i) => {
            let c = bank(i);
            RgbaImage::from_fn(w, h, |x, y| {
                let p = img.get_pixel(x, y).0;
                let l = luma(p);
                Rgba(mix(p, [0, 1, 2].map(|k| (c[k] as f32 * (0.35 + 0.65 * l)).min(255.0) as u8), 0.35))
            })
        }
        Op::PixelArt(b, pal) => {
            let pal = PALETTES[pal % PALETTES.len()].1;
            let mut out = img.clone();
            for by in (0..h).step_by(b as usize) {
                for bx in (0..w).step_by(b as usize) {
                    let (ex, ey) = ((bx + b).min(w), (by + b).min(h));
                    let mut acc = [0u32; 4];
                    let n = (ex - bx) * (ey - by);
                    for y in by..ey {
                        for x in bx..ex {
                            let p = img.get_pixel(x, y).0;
                            for k in 0..4 {
                                acc[k] += p[k] as u32;
                            }
                        }
                    }
                    let mut m = acc.map(|v| (v / n.max(1)) as u8);
                    if let Some(q) = pal.iter().min_by_key(|q| (0..3).map(|k| (q[k] as i32 - m[k] as i32).pow(2)).sum::<i32>()) {
                        m = [q[0], q[1], q[2], m[3]];
                    }
                    for y in by..ey {
                        for x in bx..ex {
                            out.put_pixel(x, y, Rgba(m));
                        }
                    }
                }
            }
            out
        }
    }
}

impl Pixel {
    pub fn run_lib(&mut self, id: &str, _ctx: &egui::Context) -> bool {
        if let Some((_, label, f)) = id.strip_prefix("tool:").and_then(|t| crate::outils::TOOLS.iter().find(|x| x.0 == t)) {
            if !self.home {
                let base = self.doc.active_target();
                let out = f(&base);
                self.doc.next_label = Some(label.to_string());
                self.doc.checkpoint();
                self.doc.put_filtered(&base, &out);
            }
            return true;
        }
        if id == "pc-ase" {
            if let Some(p) = rfd::FileDialog::new().add_filter("Nuancier Adobe", &["ase", "aco"]).pick_file() {
                match studio_fx::swatches::load(&p) {
                    Ok(c) => {
                        self.swatches.extend(c.iter().map(|(_, c)| egui::Color32::from_rgb(c[0], c[1], c[2])));
                        self.save_swatches();
                    }
                    Err(e) => self.error = Some(format!("Nuancier : {e}")),
                }
            }
            return true;
        }
        if let Some(k) = id.strip_prefix("pc:").and_then(|k| k.parse::<usize>().ok()) {
            if let Some((_, c)) = color_bank().get(k) {
                self.primary = egui::Color32::from_rgb(c[0], c[1], c[2]);
            }
            return true;
        }
        let Some(k) = id.strip_prefix("pl:").and_then(|k| k.parse::<usize>().ok()) else { return false };
        let Some((op, label, _)) = ops().get(k).cloned() else { return true };
        if self.home {
            return true;
        }
        let base = self.doc.active_target();
        let out = apply(&base, op);
        self.doc.next_label = Some(label);
        self.doc.checkpoint();
        self.doc.put_filtered(&base, &out);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_and_ops() {
        assert!(lib().len() >= 21_744, "{}", lib().len());
        let probe = RgbaImage::from_fn(6, 5, |x, y| Rgba([(x * 40) as u8, (y * 50) as u8, 128, 200]));
        for (op, label, _) in ops() {
            let out = apply(&probe, *op);
            assert_eq!(out.dimensions(), probe.dimensions(), "{label}");
            assert!(out.pixels().zip(probe.pixels()).all(|(a, b)| a.0[3] == b.0[3]) || matches!(op, Op::Frame(..) | Op::PixelArt(..)), "{label} change l'alpha");
        }
        let mut seen = std::collections::HashSet::new();
        assert!(lib().cmds.iter().all(|c| seen.insert(c.id.clone())), "identifiant en double");
        let img = RgbaImage::from_fn(20, 20, |x, _| if x < 10 { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) });
        let d = apply(&img, Op::Duotone(39, 4));
        assert_eq!(d.get_pixel(0, 0).0[..3], NAMED_COLORS[39].1);
        assert_eq!(d.get_pixel(19, 0).0[..3], NAMED_COLORS[4].1);
        let f = apply(&img, Op::Frame(12, 10, false));
        assert_eq!(f.get_pixel(0, 5).0[..3], NAMED_COLORS[12].1);
        assert_eq!(f.get_pixel(5, 5).0, [0, 0, 0, 255]);
        let p = apply(&img, Op::PixelArt(20, 2));
        assert_eq!(p.get_pixel(0, 0), p.get_pixel(19, 19));
        let g = apply(&img, Op::Gradient(1, 0, 80));
        assert!(g.get_pixel(15, 0).0[0] != 255 || g.get_pixel(15, 0).0[1] != 255);
        assert_eq!(apply(&img, Op::Tint(0, 100, 2)).get_pixel(2, 2).0, [0, 0, 0, 255]);
    }
}
