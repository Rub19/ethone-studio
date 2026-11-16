//! 38 one-click image tools (Ctrl+K « Outil : … »), each a pure function of the active layer.

use image::{Rgba, RgbaImage};

type Tool = fn(&RgbaImage) -> RgbaImage;

pub const TOOLS: [(&str, &str, Tool); 38] = [
    ("kaleido4", "Kaléidoscope 4 secteurs", |i| kaleido(i, 4)),
    ("kaleido8", "Kaléidoscope 8 secteurs", |i| kaleido(i, 8)),
    ("mirror-lr", "Symétrie : moitié gauche reflétée à droite", |i| map_xy(i, |x, y, w, _| (x.min(w - 1 - x), y))),
    ("mirror-tb", "Symétrie : moitié haute reflétée en bas", |i| map_xy(i, |x, y, _, h| (x, y.min(h - 1 - y)))),
    ("glitch", "Glitch numérique (bandes et canaux décalés)", glitch),
    ("scanlines", "Lignes de balayage d'écran cathodique", |i| per_pixel(i, |p, _, y| if y % 3 == 2 { scale(p, 0.55) } else { p })),
    ("vhs", "Effet cassette VHS", vhs),
    ("sort-h", "Tri des pixels horizontal (pixel sorting)", |i| pixel_sort(i, false)),
    ("sort-v", "Tri des pixels vertical (pixel sorting)", |i| pixel_sort(i, true)),
    ("dither-fs", "Tramage Floyd-Steinberg noir et blanc", dither_fs),
    ("dither-bayer", "Tramage ordonné Bayer 4×4", dither_bayer),
    ("pointillism", "Pointillisme (points de couleur)", pointillism),
    ("auto-palette8", "Réduire à 8 couleurs dominantes", |i| quantize(i, 8)),
    ("auto-palette16", "Réduire à 16 couleurs dominantes", |i| quantize(i, 16)),
    ("invert-luma", "Inverser la luminosité en gardant les couleurs", invert_luma),
    ("swap-rb", "Échanger les couches rouge et bleue", |i| per_pixel(i, |p, _, _| Rgba([p[2], p[1], p[0], p[3]]))),
    ("hue180", "Couleurs opposées (teinte +180°)", |i| per_pixel(i, |p, _, _| { let l = luma(p); with_alpha([0, 1, 2].map(|k| (2.0 * l * 255.0 - p[k] as f32).clamp(0.0, 255.0) as u8), p[3]) })),
    ("splash-red", "Noir et blanc sauf les rouges", |i| splash(i, 0.0)),
    ("splash-green", "Noir et blanc sauf les verts", |i| splash(i, 120.0)),
    ("splash-blue", "Noir et blanc sauf les bleus", |i| splash(i, 220.0)),
    ("heatmap", "Carte de chaleur de la luminosité", |i| per_pixel(i, |p, _, _| { let c = ramp(luma(p), &[[0, 0, 80], [0, 60, 255], [0, 220, 120], [255, 230, 0], [255, 40, 0]]); Rgba([c[0], c[1], c[2], p[3]]) })),
    ("thermal", "Vision thermique", |i| per_pixel(i, |p, _, _| { let c = ramp(luma(p), &[[10, 0, 30], [120, 0, 160], [230, 40, 60], [255, 160, 0], [255, 255, 220]]); Rgba([c[0], c[1], c[2], p[3]]) })),
    ("infrared", "Infrarouge (feuillages blancs, ciel sombre)", |i| per_pixel(i, |p, _, _| { let v = (p[1] as f32 * 1.4 - p[2] as f32 * 0.4 + p[0] as f32 * 0.2).clamp(0.0, 255.0) as u8; Rgba([v, v, (v as f32 * 0.92) as u8, p[3]]) })),
    ("anaglyph", "Anaglyphe 3D rouge et cyan", anaglyph),
    ("round-corners", "Coins arrondis transparents", |i| alpha_mask(i, |x, y, w, h| { let r = w.min(h) * 0.12; let (dx, dy) = ((r - x).max(x - (w - r)).max(0.0), (r - y).max(y - (h - r)).max(0.0)); 1.0 - ((dx * dx + dy * dy).sqrt() - r + 0.5).clamp(0.0, 1.0) })),
    ("circle-crop", "Découpe en cercle", |i| alpha_mask(i, |x, y, w, h| { let r = w.min(h) / 2.0; let d = ((x - w / 2.0).powi(2) + (y - h / 2.0).powi(2)).sqrt(); (r - d + 0.5).clamp(0.0, 1.0) })),
    ("drop-shadow", "Ombre portée sous l'objet", drop_shadow),
    ("outline", "Contour blanc autour de l'objet (autocollant)", sticker),
    ("white-to-alpha", "Rendre le blanc transparent", |i| per_pixel(i, |p, _, _| { let m = p[0].min(p[1]).min(p[2]); let a = (255.0 - (m as f32 - 200.0).max(0.0) * 255.0 / 55.0).clamp(0.0, 255.0); Rgba([p[0], p[1], p[2], (p[3] as f32 * a / 255.0) as u8]) })),
    ("black-to-alpha", "Rendre le noir transparent", |i| per_pixel(i, |p, _, _| { let m = p[0].max(p[1]).max(p[2]); let a = (m as f32 * 255.0 / 55.0).min(255.0); Rgba([p[0], p[1], p[2], (p[3] as f32 * a / 255.0) as u8]) })),
    ("opaque", "Remplir la transparence en blanc", |i| per_pixel(i, |p, _, _| { let a = p[3] as f32 / 255.0; with_alpha([0, 1, 2].map(|k| (p[k] as f32 * a + 255.0 * (1.0 - a)) as u8), 255) })),
    ("tile2", "Répéter en grille 2×2", |i| map_xy(i, |x, y, w, h| ((x * 2) % w, (y * 2) % h))),
    ("seamless", "Rendre raccordable (motif sans couture)", seamless),
    ("reflection", "Reflet au sol (tiers inférieur)", reflection),
    ("polaroid", "Cadre photo instantanée", polaroid),
    ("light-leak", "Fuite de lumière (coin orangé)", |i| per_pixel_xy(i, |p, u, v| { let t = ((1.0 - ((u - 1.0).powi(2) + v * v).sqrt() / 0.9).max(0.0)).powi(2) * 0.7; screen(p, [255, 140, 40], t) })),
    ("rain", "Pluie", rain),
    ("snow", "Neige", snow),
];

fn with_alpha(c: [u8; 3], a: u8) -> Rgba<u8> {
    Rgba([c[0], c[1], c[2], a])
}

fn luma(p: Rgba<u8>) -> f32 {
    (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) / 255.0
}

fn hash(x: u32, y: u32, s: u32) -> f32 {
    let mut h = x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263) ^ s.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    (h ^ (h >> 16)) as f32 / u32::MAX as f32
}

fn scale(p: Rgba<u8>, k: f32) -> Rgba<u8> {
    Rgba([(p[0] as f32 * k) as u8, (p[1] as f32 * k) as u8, (p[2] as f32 * k) as u8, p[3]])
}

fn screen(p: Rgba<u8>, c: [u8; 3], t: f32) -> Rgba<u8> {
    let s = |k: usize| 255.0 - (255.0 - p[k] as f32) * (255.0 - c[k] as f32) / 255.0;
    with_alpha([0, 1, 2].map(|k| (p[k] as f32 + (s(k) - p[k] as f32) * t.clamp(0.0, 1.0)) as u8), p[3])
}

fn ramp(t: f32, stops: &[[u8; 3]]) -> [u8; 3] {
    let f = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (f as usize).min(stops.len() - 2);
    let u = f - i as f32;
    [0, 1, 2].map(|k| (stops[i][k] as f32 + (stops[i + 1][k] as f32 - stops[i][k] as f32) * u) as u8)
}

fn per_pixel(img: &RgbaImage, f: impl Fn(Rgba<u8>, u32, u32) -> Rgba<u8>) -> RgbaImage {
    RgbaImage::from_fn(img.width(), img.height(), |x, y| f(*img.get_pixel(x, y), x, y))
}

fn per_pixel_xy(img: &RgbaImage, f: impl Fn(Rgba<u8>, f32, f32) -> Rgba<u8>) -> RgbaImage {
    let (w, h) = (img.width().max(1) as f32, img.height().max(1) as f32);
    RgbaImage::from_fn(img.width(), img.height(), |x, y| f(*img.get_pixel(x, y), (x as f32 + 0.5) / w, (y as f32 + 0.5) / h))
}

fn map_xy(img: &RgbaImage, f: impl Fn(u32, u32, u32, u32) -> (u32, u32)) -> RgbaImage {
    let (w, h) = img.dimensions();
    RgbaImage::from_fn(w, h, |x, y| {
        let (sx, sy) = f(x, y, w, h);
        *img.get_pixel(sx.min(w - 1), sy.min(h - 1))
    })
}

fn alpha_mask(img: &RgbaImage, f: impl Fn(f32, f32, f32, f32) -> f32) -> RgbaImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    per_pixel(img, |p, x, y| Rgba([p[0], p[1], p[2], (p[3] as f32 * f(x as f32 + 0.5, y as f32 + 0.5, w, h).clamp(0.0, 1.0)) as u8]))
}

fn kaleido(img: &RgbaImage, n: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let sector = std::f32::consts::TAU / n as f32;
    RgbaImage::from_fn(w, h, |x, y| {
        let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
        let r = (dx * dx + dy * dy).sqrt();
        let mut a = dy.atan2(dx).rem_euclid(sector);
        if a > sector / 2.0 {
            a = sector - a;
        }
        let (sx, sy) = (cx + r * a.cos(), cy + r * a.sin());
        *img.get_pixel((sx.max(0.0) as u32).min(w - 1), (sy.max(0.0) as u32).min(h - 1))
    })
}

fn glitch(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let band = (h / 24).max(1);
    RgbaImage::from_fn(w, h, |x, y| {
        let b = y / band;
        let shift = if hash(b, 7, 1) > 0.6 { ((hash(b, 3, 2) - 0.5) * w as f32 * 0.12) as i64 } else { 0 };
        let at = |dx: i64| *img.get_pixel(((x as i64 + shift + dx).rem_euclid(w as i64)) as u32, y);
        let off = (w as i64 / 80).max(1);
        Rgba([at(off)[0], at(0)[1], at(-off)[2], at(0)[3]])
    })
}

fn vhs(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let off = (w as i64 / 120).max(1);
    RgbaImage::from_fn(w, h, |x, y| {
        let wob = ((y as f32 * 0.07).sin() * 1.5) as i64;
        let at = |dx: i64| *img.get_pixel(((x as i64 + dx + wob).clamp(0, w as i64 - 1)) as u32, y);
        let n = (hash(x / 2, y, 9) - 0.5) * 26.0;
        let c = [at(off)[0] as f32 + n, at(0)[1] as f32 + n, at(-off)[2] as f32 + n];
        let line = if hash(y, 1, 5) > 0.985 { 40.0 } else { 0.0 };
        with_alpha([0, 1, 2].map(|k| (c[k] * 0.92 + 12.0 + line).clamp(0.0, 255.0) as u8), at(0)[3])
    })
}

fn pixel_sort(img: &RgbaImage, vertical: bool) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = img.clone();
    let (lines, len) = if vertical { (w, h) } else { (h, w) };
    for l in 0..lines {
        let at = |k: u32| if vertical { (l, k) } else { (k, l) };
        let mut k = 0;
        while k < len {
            let (x, y) = at(k);
            if luma(*img.get_pixel(x, y)) < 0.35 {
                k += 1;
                continue;
            }
            let start = k;
            while k < len && { let (x, y) = at(k); luma(*img.get_pixel(x, y)) >= 0.35 } {
                k += 1;
            }
            let mut run: Vec<Rgba<u8>> = (start..k).map(|j| { let (x, y) = at(j); *img.get_pixel(x, y) }).collect();
            run.sort_by(|a, b| luma(*a).total_cmp(&luma(*b)));
            for (j, p) in (start..k).zip(run) {
                let (x, y) = at(j);
                out.put_pixel(x, y, p);
            }
        }
    }
    out
}

fn dither_fs(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut lum: Vec<f32> = img.pixels().map(|p| luma(*p) * 255.0).collect();
    let mut out = img.clone();
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * w as usize + x;
            let v = if lum[i] < 128.0 { 0.0 } else { 255.0 };
            let e = lum[i] - v;
            let mut add = |dx: i64, dy: usize, k: f32| {
                let nx = x as i64 + dx;
                if nx >= 0 && (nx as usize) < w as usize && y + dy < h as usize {
                    lum[(y + dy) * w as usize + nx as usize] += e * k;
                }
            };
            add(1, 0, 7.0 / 16.0);
            add(-1, 1, 3.0 / 16.0);
            add(0, 1, 5.0 / 16.0);
            add(1, 1, 1.0 / 16.0);
            let a = img.get_pixel(x as u32, y as u32)[3];
            out.put_pixel(x as u32, y as u32, Rgba([v as u8, v as u8, v as u8, a]));
        }
    }
    out
}

fn dither_bayer(img: &RgbaImage) -> RgbaImage {
    const M: [[f32; 4]; 4] = [[0.0, 8.0, 2.0, 10.0], [12.0, 4.0, 14.0, 6.0], [3.0, 11.0, 1.0, 9.0], [15.0, 7.0, 13.0, 5.0]];
    per_pixel(img, |p, x, y| {
        let t = (M[y as usize % 4][x as usize % 4] + 0.5) / 16.0;
        with_alpha([0, 1, 2].map(|k| if p[k] as f32 / 255.0 > t { 255 } else { 0 }), p[3])
    })
}

fn pointillism(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let cell = (w.min(h) / 40).max(3);
    RgbaImage::from_fn(w, h, |x, y| {
        let (cx, cy) = (x / cell * cell + cell / 2, y / cell * cell + cell / 2);
        let c = *img.get_pixel(cx.min(w - 1), cy.min(h - 1));
        let r = cell as f32 * (0.35 + 0.2 * hash(cx, cy, 3));
        let d = ((x as f32 - cx as f32).powi(2) + (y as f32 - cy as f32).powi(2)).sqrt();
        if d <= r { c } else { Rgba([250, 248, 240, c[3]]) }
    })
}

/// Median-cut style palette: repeatedly split the widest colour box.
fn quantize(img: &RgbaImage, n: usize) -> RgbaImage {
    let step = ((img.width() * img.height()) as usize / 4000).max(1);
    let sample: Vec<[u8; 3]> = img.pixels().step_by(step).map(|p| [p[0], p[1], p[2]]).collect();
    let mut boxes = vec![sample];
    while boxes.len() < n {
        let Some((bi, ch)) = boxes.iter().enumerate().filter(|(_, b)| b.len() > 1).map(|(i, b)| {
            let range = |k: usize| b.iter().map(|c| c[k]).max().unwrap_or(0) - b.iter().map(|c| c[k]).min().unwrap_or(0);
            let ch = (0..3).max_by_key(|k| range(*k)).unwrap_or(0);
            (i, ch, range(ch))
        }).max_by_key(|t| t.2).map(|t| (t.0, t.1)) else { break };
        let mut b = boxes.swap_remove(bi);
        b.sort_by_key(|c| c[ch]);
        let other = b.split_off(b.len() / 2);
        boxes.push(b);
        boxes.push(other);
    }
    let pal: Vec<[f32; 3]> = boxes.iter().filter(|b| !b.is_empty()).map(|b| [0, 1, 2].map(|k| b.iter().map(|c| c[k] as f32).sum::<f32>() / b.len() as f32)).collect();
    per_pixel(img, |p, _, _| {
        let best = pal.iter().min_by(|a, b| {
            let d = |q: &[f32; 3]| (0..3).map(|k| (q[k] - p[k] as f32).powi(2)).sum::<f32>();
            d(a).total_cmp(&d(b))
        });
        best.map_or(p, |q| Rgba([q[0] as u8, q[1] as u8, q[2] as u8, p[3]]))
    })
}

fn invert_luma(img: &RgbaImage) -> RgbaImage {
    per_pixel(img, |p, _, _| {
        let l = luma(p);
        let d = (1.0 - l) - l;
        with_alpha([0, 1, 2].map(|k| (p[k] as f32 + d * 255.0).clamp(0.0, 255.0) as u8), p[3])
    })
}

fn hue(p: Rgba<u8>) -> (f32, f32) {
    let (r, g, b) = (p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let d = mx - mn;
    if d < 1e-4 {
        return (0.0, 0.0);
    }
    let h = if mx == r { ((g - b) / d).rem_euclid(6.0) } else if mx == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
    (h * 60.0, d / mx.max(1e-4))
}

fn splash(img: &RgbaImage, target: f32) -> RgbaImage {
    per_pixel(img, |p, _, _| {
        let (h, s) = hue(p);
        let dist = ((h - target + 180.0).rem_euclid(360.0) - 180.0).abs();
        let keep = (1.0 - ((dist - 25.0) / 20.0).clamp(0.0, 1.0)) * (s * 3.0).min(1.0);
        let l = luma(p) * 255.0;
        with_alpha([0, 1, 2].map(|k| (l + (p[k] as f32 - l) * keep) as u8), p[3])
    })
}

fn anaglyph(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let off = (w / 60).max(1) as i64;
    RgbaImage::from_fn(w, h, |x, y| {
        let at = |dx: i64| *img.get_pixel((x as i64 + dx).clamp(0, w as i64 - 1) as u32, y);
        let (l, r) = (at(-off), at(off));
        Rgba([(luma(l) * 255.0) as u8, r[1], r[2], img.get_pixel(x, y)[3]])
    })
}

fn drop_shadow(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let off = (w.min(h) / 40).max(1) as i64;
    let rad = off;
    let alpha = |x: i64, y: i64| if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 { 0.0 } else { img.get_pixel(x as u32, y as u32)[3] as f32 / 255.0 };
    RgbaImage::from_fn(w, h, |x, y| {
        let p = *img.get_pixel(x, y);
        let mut sum = 0.0;
        let mut n = 0.0;
        for dy in (-rad..=rad).step_by(((rad / 3).max(1)) as usize) {
            for dx in (-rad..=rad).step_by(((rad / 3).max(1)) as usize) {
                sum += alpha(x as i64 - off + dx, y as i64 - off + dy);
                n += 1.0;
            }
        }
        let sh = sum / n * 0.55;
        let a = p[3] as f32 / 255.0;
        let out_a = a + sh * (1.0 - a);
        if out_a <= 0.0 {
            return Rgba([0, 0, 0, 0]);
        }
        with_alpha([0, 1, 2].map(|k| (p[k] as f32 * a / out_a) as u8), (out_a * 255.0) as u8)
    })
}

fn sticker(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let r = (w.min(h) / 50).max(1) as i64;
    RgbaImage::from_fn(w, h, |x, y| {
        let p = *img.get_pixel(x, y);
        if p[3] == 255 {
            return p;
        }
        let near = (-r..=r).any(|dy| (-r..=r).any(|dx| {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            dx * dx + dy * dy <= r * r && nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 && img.get_pixel(nx as u32, ny as u32)[3] > 128
        }));
        if !near {
            return p;
        }
        let a = p[3] as f32 / 255.0;
        with_alpha([0, 1, 2].map(|k| (p[k] as f32 * a + 255.0 * (1.0 - a)) as u8), 255)
    })
}

fn seamless(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    RgbaImage::from_fn(w, h, |x, y| {
        let a = *img.get_pixel(x, y);
        let b = *img.get_pixel((x + w / 2) % w, (y + h / 2) % h);
        let (u, v) = ((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
        let edge = (u.min(1.0 - u).min(v).min(1.0 - v) * 4.0).min(1.0);
        Rgba([0, 1, 2, 3].map(|k| (b[k] as f32 + (a[k] as f32 - b[k] as f32) * edge) as u8))
    })
}

fn reflection(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let top = h * 2 / 3;
    RgbaImage::from_fn(w, h, |x, y| {
        if y < top {
            return *img.get_pixel(x, (y as f32 * h as f32 / top.max(1) as f32) as u32 % h.max(1));
        }
        let src_y = top.saturating_sub(y - top + 1);
        let p = *img.get_pixel(x, (src_y as f32 * h as f32 / top.max(1) as f32).min(h as f32 - 1.0) as u32);
        let fade = 0.45 * (1.0 - (y - top) as f32 / (h - top).max(1) as f32);
        with_alpha([0, 1, 2].map(|k| (p[k] as f32 * fade + 245.0 * (1.0 - fade)) as u8), p[3])
    })
}

fn polaroid(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let m = (w.min(h) as f32 * 0.06).max(1.0);
    let bottom = m * 3.5;
    let (iw, ih) = ((w as f32 - 2.0 * m).max(1.0), (h as f32 - m - bottom).max(1.0));
    RgbaImage::from_fn(w, h, |x, y| {
        let (fx, fy) = (x as f32 + 0.5 - m, y as f32 + 0.5 - m);
        if fx < 0.0 || fy < 0.0 || fx >= iw || fy >= ih {
            return Rgba([248, 246, 240, 255]);
        }
        *img.get_pixel(((fx / iw * w as f32) as u32).min(w - 1), ((fy / ih * h as f32) as u32).min(h - 1))
    })
}

fn rain(img: &RgbaImage) -> RgbaImage {
    per_pixel(img, |p, x, y| {
        let lane = x.wrapping_add(y / 3);
        let streak = hash(lane / 2, (y + lane * 7) / 14, 11) > 0.93 && lane % 2 == 0;
        if streak { screen(p, [200, 210, 225], 0.45) } else { scale(p, 0.9) }
    })
}

fn snow(img: &RgbaImage) -> RgbaImage {
    let s = (img.width().min(img.height()) / 160).max(1);
    per_pixel(img, |p, x, y| {
        let (cx, cy) = (x / (s * 3), y / (s * 3));
        let flake = hash(cx, cy, 21) > 0.96 && (x % (s * 3)) < s * 2 && (y % (s * 3)) < s * 2;
        if flake { Rgba([255, 255, 255, p[3].max(220)]) } else { screen(p, [210, 220, 235], 0.12) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_runs_and_changes_the_image() {
        let img = RgbaImage::from_fn(48, 36, |x, y| {
            let a = if (x as i32 - 24).pow(2) + (y as i32 - 18).pow(2) < 140 { 255 } else if x < 4 { 0 } else { 230 };
            Rgba([(x * 5) as u8, (y * 7) as u8, (((x + y) * 3 + 20) % 256) as u8, a])
        });
        let mut ids = std::collections::HashSet::new();
        for (id, label, f) in TOOLS {
            assert!(ids.insert(id), "{id} en double");
            let out = f(&img);
            assert_eq!(out.dimensions(), img.dimensions(), "{label}");
            assert_ne!(out, img, "{label} ne change rien");
        }
        let q = quantize(&img, 8);
        let mut colors: Vec<[u8; 3]> = q.pixels().map(|p| [p[0], p[1], p[2]]).collect();
        colors.sort();
        colors.dedup();
        assert!(colors.len() <= 8, "{}", colors.len());
        let d = dither_fs(&img);
        assert!(d.pixels().all(|p| p[0] == 0 || p[0] == 255));
    }
}
