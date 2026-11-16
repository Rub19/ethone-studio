//! 38 one-click tools on the selected objects (Ctrl+K « Outil : … »): arrangements, progressions
//! across the selection, generated copies and batch colour changes.
//! Each tool gets the items, the selection and the page size; it returns the objects to select after.

use crate::model::{Item, P};

type Tool = fn(&mut Vec<Item>, &[usize], P) -> Vec<usize>;

pub const TOOLS: [(&str, &str, usize, Tool); 38] = [
    ("circle", "Disposer en cercle", 2, |it, s, _| { let (c, r) = spread(it, s); for (k, &i) in s.iter().enumerate() { let a = k as f32 / s.len() as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2; move_center(&mut it[i], [c[0] + r * a.cos(), c[1] + r * a.sin()]); } s.to_vec() }),
    ("grid", "Disposer en grille", 2, grid),
    ("row-tight", "Aligner en rangée, bord à bord", 2, |it, s, _| { let mut x = it[s[0]].bbox().min[0]; let y = it[s[0]].bbox().center()[1]; for &i in s { let b = it[i].bbox(); move_center(&mut it[i], [x + b.size()[0] / 2.0, y]); x += b.size()[0]; } s.to_vec() }),
    ("stack-tight", "Empiler en colonne, bord à bord", 2, |it, s, _| { let mut y = it[s[0]].bbox().min[1]; let x = it[s[0]].bbox().center()[0]; for &i in s { let b = it[i].bbox(); move_center(&mut it[i], [x, y + b.size()[1] / 2.0]); y += b.size()[1]; } s.to_vec() }),
    ("shuffle", "Mélanger les positions", 2, |it, s, _| { let centers: Vec<P> = s.iter().map(|&i| it[i].bbox().center()).collect(); for (k, &i) in s.iter().enumerate() { move_center(&mut it[i], centers[(k * 7 + 3) % s.len()]); } s.to_vec() }),
    ("random-size", "Tailles aléatoires (60 à 140 %)", 1, |it, s, _| { for (k, &i) in s.iter().enumerate() { scale(&mut it[i], 0.6 + 0.8 * rnd(k, 1)); } s.to_vec() }),
    ("random-rotate", "Rotations aléatoires", 1, |it, s, _| { for (k, &i) in s.iter().enumerate() { it[i].rotate(rnd(k, 2) * 360.0 - 180.0); } s.to_vec() }),
    ("random-colors", "Couleurs aléatoires (banque de couleurs)", 1, |it, s, _| { let b = studio_ui::palette::color_bank(); for (k, &i) in s.iter().enumerate() { let c = b[(rnd(k, 3) * b.len() as f32) as usize % b.len()].1; it[i].fill = Some([c[0], c[1], c[2], 255]); } s.to_vec() }),
    ("color-ramp", "Dégradé de couleurs du premier au dernier", 3, |it, s, _| { let (a, b) = (paint(&it[s[0]]), paint(&it[s[s.len() - 1]])); for (k, &i) in s.iter().enumerate() { let t = k as f32 / (s.len() - 1) as f32; it[i].fill = Some(std::array::from_fn(|j| (a[j] as f32 + (b[j] as f32 - a[j] as f32) * t).round() as u8)); } s.to_vec() }),
    ("opacity-ramp", "Fondu d'opacité (100 → 20 %)", 2, |it, s, _| { for (k, &i) in s.iter().enumerate() { it[i].opacity = 1.0 - 0.8 * k as f32 / (s.len() - 1) as f32; } s.to_vec() }),
    ("size-ramp", "Progression de taille (100 → 40 %)", 2, |it, s, _| { for (k, &i) in s.iter().enumerate() { scale(&mut it[i], 1.0 - 0.6 * k as f32 / (s.len() - 1) as f32); } s.to_vec() }),
    ("rotate-ramp", "Progression de rotation (tour complet)", 2, |it, s, _| { for (k, &i) in s.iter().enumerate() { it[i].rotate(360.0 * k as f32 / s.len() as f32); } s.to_vec() }),
    ("same-width", "Même largeur que le premier", 2, |it, s, _| { let w = it[s[0]].bbox().size()[0]; for &i in &s[1..] { let b = it[i].bbox(); let k = w / b.size()[0].max(1e-3); let c = b.center(); it[i].map(|p| [c[0] + (p[0] - c[0]) * k, p[1]], k, 1.0); } s.to_vec() }),
    ("same-height", "Même hauteur que le premier", 2, |it, s, _| { let h = it[s[0]].bbox().size()[1]; for &i in &s[1..] { let b = it[i].bbox(); let k = h / b.size()[1].max(1e-3); let c = b.center(); it[i].map(|p| [p[0], c[1] + (p[1] - c[1]) * k], 1.0, k); } s.to_vec() }),
    ("same-size", "Même taille que le premier", 2, |it, s, _| { let z = it[s[0]].bbox().size(); for &i in &s[1..] { let b = it[i].bbox(); let (kx, ky) = (z[0] / b.size()[0].max(1e-3), z[1] / b.size()[1].max(1e-3)); let c = b.center(); it[i].map(|p| [c[0] + (p[0] - c[0]) * kx, c[1] + (p[1] - c[1]) * ky], kx, ky); } s.to_vec() }),
    ("swap", "Échanger les places des deux premiers", 2, |it, s, _| { let (a, b) = (it[s[0]].bbox().center(), it[s[1]].bbox().center()); move_center(&mut it[s[0]], b); move_center(&mut it[s[1]], a); s.to_vec() }),
    ("reverse-order", "Inverser l'ordre d'empilement", 2, |it, s, _| { let picked: Vec<Item> = s.iter().map(|&i| it[i].clone()).collect(); for (k, &i) in s.iter().enumerate() { it[i] = picked[s.len() - 1 - k].clone(); } s.to_vec() }),
    ("sort-size", "Empiler du plus grand au plus petit", 2, |it, s, _| { let mut picked: Vec<Item> = s.iter().map(|&i| it[i].clone()).collect(); picked.sort_by(|a, b| area(b).total_cmp(&area(a))); for (k, &i) in s.iter().enumerate() { it[i] = picked[k].clone(); } s.to_vec() }),
    ("sort-light", "Ranger du plus sombre au plus clair (gauche → droite)", 2, |it, s, _| { let mut xs: Vec<f32> = s.iter().map(|&i| it[i].bbox().center()[0]).collect(); xs.sort_by(f32::total_cmp); let mut order = s.to_vec(); order.sort_by_key(|&i| lum(paint(&it[i]))); for (k, &i) in order.iter().enumerate() { let y = it[i].bbox().center()[1]; move_center(&mut it[i], [xs[k], y]); } s.to_vec() }),
    ("radial-copies", "Copies en rosace (6 autour du centre)", 1, |it, s, _| copies(it, s, 5, |src, k| { let c = src.bbox().center(); let r = src.bbox().size()[0].max(src.bbox().size()[1]) * 0.9; let mut x = src.clone(); x.rotate_about(60.0 * (k + 1) as f32, [c[0], c[1] + r]); x })),
    ("row-copies", "Répéter en rangée (5 copies)", 1, |it, s, _| copies(it, s, 4, |src, k| { let mut x = src.clone(); x.translate([src.bbox().size()[0] * 1.2 * (k + 1) as f32, 0.0]); x })),
    ("grid-copies", "Répéter en grille 3×3", 1, |it, s, _| copies(it, s, 8, |src, k| { let z = src.bbox().size(); let (cx, cy) = ((k + 1) % 3, (k + 1) / 3); let mut x = src.clone(); x.translate([z[0] * 1.2 * cx as f32, z[1] * 1.2 * cy as f32]); x })),
    ("mirror-copy", "Copie miroir à droite", 1, |it, s, _| copies(it, s, 1, |src, _| { let b = src.bbox(); let mut x = src.clone(); let axis = b.max[0]; x.map(|p| [2.0 * axis - p[0], p[1]], -1.0, 1.0); x })),
    ("long-shadow", "Ombre longue", 1, |it, s, _| insert_behind(it, s, |src| { let z = src.bbox().size(); (1..=12).rev().map(|k| { let mut x = src.clone(); x.translate([z[0] * 0.03 * k as f32, z[1] * 0.03 * k as f32]); x.fill = Some([20, 20, 30, 255]); x.stroke = None; x.opacity = 0.35; x }).collect() })),
    ("glow", "Halo derrière l'objet", 1, |it, s, _| insert_behind(it, s, |src| { let mut x = src.clone(); scale(&mut x, 1.15); x.blur = 12.0; x.opacity = 0.6; vec![x] })),
    ("neon", "Effet néon", 1, |it, s, _| { for &i in s { let c = paint(&it[i]); it[i].stroke = Some(c); it[i].stroke_w = it[i].stroke_w.max(3.0); it[i].fill = None; it[i].shadow = true; } s.to_vec() }),
    ("explode", "Écarter du centre (+30 %)", 2, |it, s, _| { push(it, s, 1.3); s.to_vec() }),
    ("implode", "Rapprocher du centre (−30 %)", 2, |it, s, _| { push(it, s, 0.7); s.to_vec() }),
    ("snap-pos", "Arrondir les positions au pixel", 1, |it, s, _| { for &i in s { let m = it[i].bbox().min; it[i].translate([m[0].round() - m[0], m[1].round() - m[1]]); } s.to_vec() }),
    ("center-page", "Centrer sur la page", 1, |it, s, page| { let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]); for &i in s { let b = it[i].bbox(); lo = [lo[0].min(b.min[0]), lo[1].min(b.min[1])]; hi = [hi[0].max(b.max[0]), hi[1].max(b.max[1])]; } let d = [page[0] / 2.0 - (lo[0] + hi[0]) / 2.0, page[1] / 2.0 - (lo[1] + hi[1]) / 2.0]; for &i in s { it[i].translate(d); } s.to_vec() }),
    ("fit-page", "Agrandir pour remplir la page", 1, |it, s, page| { let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]); for &i in s { let b = it[i].bbox(); lo = [lo[0].min(b.min[0]), lo[1].min(b.min[1])]; hi = [hi[0].max(b.max[0]), hi[1].max(b.max[1])]; } let k = ((page[0] * 0.9) / (hi[0] - lo[0]).max(1e-3)).min((page[1] * 0.9) / (hi[1] - lo[1]).max(1e-3)); let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0]; for &i in s { it[i].map(|p| [page[0] / 2.0 + (p[0] - c[0]) * k, page[1] / 2.0 + (p[1] - c[1]) * k], k, k); } s.to_vec() }),
    ("corners", "Placer dans les quatre coins", 1, |it, s, page| { for (k, &i) in s.iter().take(4).enumerate() { let z = it[i].bbox().size(); let m = page[0].min(page[1]) * 0.05; let x = if k % 2 == 0 { m + z[0] / 2.0 } else { page[0] - m - z[0] / 2.0 }; let y = if k < 2 { m + z[1] / 2.0 } else { page[1] - m - z[1] / 2.0 }; move_center(&mut it[i], [x, y]); } s.to_vec() }),
    ("pastel", "Pastelliser les couleurs", 1, |it, s, _| { recolor(it, s, |c| c.map(|v| (v as f32 * 0.55 + 255.0 * 0.45) as u8)); s.to_vec() }),
    ("darken", "Assombrir de 20 %", 1, |it, s, _| { recolor(it, s, |c| c.map(|v| (v as f32 * 0.8) as u8)); s.to_vec() }),
    ("lighten", "Éclaircir de 20 %", 1, |it, s, _| { recolor(it, s, |c| c.map(|v| (v as f32 + (255.0 - v as f32) * 0.2) as u8)); s.to_vec() }),
    ("swap-fill-stroke", "Échanger fond et contour", 1, |it, s, _| { for &i in s { let x = &mut it[i]; (x.fill, x.stroke) = (x.stroke.or(Some([0, 0, 0, 255])), x.fill); if x.stroke.is_some() { x.stroke_w = x.stroke_w.max(1.0); } } s.to_vec() }),
    ("outline-only", "Contours seuls (sans fond)", 1, |it, s, _| { for &i in s { let c = paint(&it[i]); let x = &mut it[i]; x.stroke = Some(x.stroke.unwrap_or(c)); x.stroke_w = x.stroke_w.max(1.5); x.fill = None; x.fill2 = None; } s.to_vec() }),
    ("stroke-by-size", "Épaisseur de contour selon la taille", 1, |it, s, _| { for &i in s { let z = it[i].bbox().size(); let x = &mut it[i]; x.stroke_w = (z[0].min(z[1]) * 0.04).max(0.5); if x.stroke.is_none() { x.stroke = Some([20, 20, 24, 255]); } } s.to_vec() }),
];

fn rnd(k: usize, s: u32) -> f32 {
    let mut h = (k as u32).wrapping_mul(2_654_435_761) ^ s.wrapping_mul(97_531);
    h ^= h >> 15;
    h = h.wrapping_mul(2_246_822_519);
    (h ^ (h >> 13)) as f32 / u32::MAX as f32
}

fn move_center(it: &mut Item, to: P) {
    let c = it.bbox().center();
    it.translate([to[0] - c[0], to[1] - c[1]]);
}

fn scale(it: &mut Item, k: f32) {
    let c = it.bbox().center();
    it.map(|p| [c[0] + (p[0] - c[0]) * k, c[1] + (p[1] - c[1]) * k], k, k);
}

fn area(it: &Item) -> f32 {
    let z = it.bbox().size();
    z[0] * z[1]
}

fn paint(it: &Item) -> [u8; 4] {
    it.fill.or(it.stroke).unwrap_or([30, 30, 36, 255])
}

fn lum(c: [u8; 4]) -> u32 {
    c[0] as u32 * 3 + c[1] as u32 * 6 + c[2] as u32
}

/// Centre and radius of the selection's spread (for circular layouts).
fn spread(it: &[Item], s: &[usize]) -> (P, f32) {
    let cs: Vec<P> = s.iter().map(|&i| it[i].bbox().center()).collect();
    let c = [cs.iter().map(|p| p[0]).sum::<f32>() / cs.len() as f32, cs.iter().map(|p| p[1]).sum::<f32>() / cs.len() as f32];
    let big = s.iter().map(|&i| { let z = it[i].bbox().size(); z[0].max(z[1]) }).fold(0.0, f32::max);
    let r = cs.iter().map(|p| ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2)).sqrt()).fold(0.0, f32::max).max(big * s.len() as f32 / 4.0);
    (c, r)
}

fn grid(it: &mut Vec<Item>, s: &[usize], _: P) -> Vec<usize> {
    let cols = (s.len() as f32).sqrt().ceil() as usize;
    let cell = s.iter().map(|&i| { let z = it[i].bbox().size(); z[0].max(z[1]) }).fold(0.0, f32::max) * 1.25;
    let origin = it[s[0]].bbox().min;
    for (k, &i) in s.iter().enumerate() {
        move_center(&mut it[i], [origin[0] + cell * ((k % cols) as f32 + 0.5), origin[1] + cell * ((k / cols) as f32 + 0.5)]);
    }
    s.to_vec()
}

fn push(it: &mut [Item], s: &[usize], k: f32) {
    let (c, _) = spread(it, s);
    for &i in s {
        let p = it[i].bbox().center();
        move_center(&mut it[i], [c[0] + (p[0] - c[0]) * k, c[1] + (p[1] - c[1]) * k]);
    }
}

fn recolor(it: &mut [Item], s: &[usize], f: impl Fn([u8; 3]) -> [u8; 3]) {
    for &i in s {
        let x = &mut it[i];
        for c in [&mut x.fill, &mut x.stroke, &mut x.fill2].into_iter().flatten() {
            let n = f([c[0], c[1], c[2]]);
            *c = [n[0], n[1], n[2], c[3]];
        }
    }
}

/// Inserts the objects made by `make` just below each selected one; returns the originals' new places.
fn insert_behind(it: &mut Vec<Item>, s: &[usize], make: impl Fn(&Item) -> Vec<Item>) -> Vec<usize> {
    let mut sorted = s.to_vec();
    sorted.sort_unstable();
    let mut added = 0;
    let mut out = Vec::new();
    for &i in &sorted {
        let at = i + added;
        let extra = make(&it[at]);
        let n = extra.len();
        for (k, x) in extra.into_iter().enumerate() {
            it.insert(at + k, x);
        }
        added += n;
        out.push(at + n);
    }
    out
}

/// Appends `n` copies of each selected object made by `make(source, k)`; selects originals and copies.
fn copies(it: &mut Vec<Item>, s: &[usize], n: usize, make: impl Fn(&Item, usize) -> Item) -> Vec<usize> {
    let mut out = s.to_vec();
    for &i in s {
        let src = it[i].clone();
        for k in 0..n {
            it.push(make(&src, k));
            out.push(it.len() - 1);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Doc, Geom};

    #[test]
    fn every_tool_runs_and_changes_the_drawing() {
        let make = || {
            (0..5)
                .map(|k| Item {
                    geom: if k % 2 == 0 { Geom::Rect { x: 20.37 + k as f32 * 70.0, y: 40.0 + k as f32 * 9.0, w: 40.0 + k as f32 * 6.0, h: 30.0, r: 0.0 } } else { Geom::Ellipse { cx: 50.4 + k as f32 * 70.0, cy: 90.6, rx: 18.0, ry: 25.0 } },
                    fill: Some([[200, 30, 90], [20, 180, 40], [250, 220, 10], [60, 60, 200], [10, 10, 10]][k]).map(|c: [u8; 3]| [c[0], c[1], c[2], 255]),
                    stroke: None,
                    stroke_w: 1.0,
                    opacity: 1.0,
                    visible: true,
                    ..Item::blank()
                })
                .collect::<Vec<_>>()
        };
        let opt = resvg::usvg::Options::default();
        let mut ids = std::collections::HashSet::new();
        for (id, label, min, f) in TOOLS {
            assert!(ids.insert(id), "{id} en double");
            let before = make();
            let mut items = before.clone();
            let sel: Vec<usize> = (0..items.len().max(min)).collect();
            let after_sel = f(&mut items, &sel, [800.0, 600.0]);
            assert!(after_sel.iter().all(|i| *i < items.len()) && !after_sel.is_empty(), "{label}");
            assert!(items != before, "{label} ne change rien");
            let mut d = Doc::new(800.0, 600.0);
            d.items = items;
            assert!(resvg::usvg::Tree::from_str(&d.to_svg(), &opt).is_ok(), "{label} : SVG invalide");
        }
    }
}
