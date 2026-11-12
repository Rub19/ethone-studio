use image::{RgbImage, imageops};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const RAW_EXT: [&str; 14] = ["cr2", "cr3", "nef", "nrw", "arw", "srf", "sr2", "dng", "raf", "orf", "rw2", "pef", "srw", "3fr"];
pub const IMG_EXT: [&str; 6] = ["jpg", "jpeg", "png", "webp", "tif", "tiff"];
pub const BANDS: [(&str, f32, [u8; 3]); 8] = [
    ("Rouges", 0.0, [235, 64, 52]),
    ("Oranges", 30.0, [245, 140, 40]),
    ("Jaunes", 60.0, [240, 210, 40]),
    ("Verts", 120.0, [70, 190, 80]),
    ("Turquoises", 180.0, [40, 200, 200]),
    ("Bleus", 240.0, [50, 110, 235]),
    ("Violets", 270.0, [140, 80, 230]),
    ("Magentas", 300.0, [220, 60, 190]),
];

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub exposure: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
    pub temperature: f32,
    pub tint: f32,
    pub vibrance: f32,
    pub saturation: f32,
    pub vignette: f32,
    pub rotate: u8,
    pub texture: f32,
    pub clarity: f32,
    pub dehaze: f32,
    pub curve: [f32; 4],
    pub hsl: [[f32; 3]; 8],
    pub mono: bool,
    pub split_shadow_hue: f32,
    pub split_shadow_sat: f32,
    pub split_high_hue: f32,
    pub split_high_sat: f32,
    pub split_balance: f32,
    pub sharpen: f32,
    pub sharpen_radius: f32,
    pub noise: f32,
    pub vignette_mid: f32,
    pub grain: f32,
    pub grain_size: f32,
    pub straighten: f32,
    pub crop: [f32; 4],
    pub flip: bool,
}

impl Settings {
    pub fn is_default(&self) -> bool {
        *self == Settings::default()
    }
}

pub fn is_supported(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| RAW_EXT.contains(&e.as_str()) || IMG_EXT.contains(&e.as_str()))
}

pub fn is_raw(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| RAW_EXT.contains(&e.to_ascii_lowercase().as_str()))
}

pub fn load(path: &Path, max: u32) -> Result<RgbImage, String> {
    let img = if is_raw(path) {
        let m = max as usize;
        let s = imagepipe::simple_decode_8bit(path, m, m)?;
        RgbImage::from_raw(s.width as u32, s.height as u32, s.data).ok_or("RAW illisible")?
    } else {
        image::open(path).map_err(|e| e.to_string())?.into_rgb8()
    };
    Ok(if max > 0 && (img.width() > max || img.height() > max) {
        let r = max as f32 / img.width().max(img.height()) as f32;
        imageops::resize(&img, (img.width() as f32 * r) as u32, (img.height() as f32 * r) as u32, imageops::FilterType::Triangle)
    } else {
        img
    })
}

fn to_lin(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn to_srgb(v: f32) -> f32 {
    let v = v.max(0.0);
    if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn rgb_to_hsl(c: [f32; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max - min < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    if s <= 0.0 {
        return [l; 3];
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let f = |t: f32| {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let h = h / 360.0;
    [f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0)]
}

fn hash(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xFFFF) as f32 / 65535.0
}

fn band_adjust(hsl: &[[f32; 3]; 8], hue: f32) -> [f32; 3] {
    let h = hue.rem_euclid(360.0);
    for i in 0..8 {
        let (a, b) = (BANDS[i].1, if i == 7 { 360.0 } else { BANDS[i + 1].1 });
        if h >= a && h < b {
            let t = (h - a) / (b - a);
            let next = hsl[(i + 1) % 8];
            return std::array::from_fn(|k| hsl[i][k] + (next[k] - hsl[i][k]) * t);
        }
    }
    hsl[0]
}

/// Straighten (with automatic inner crop so no empty corners show), crop insets, mirror.
fn geometry(src: &RgbImage, s: &Settings) -> Option<RgbImage> {
    if s.straighten == 0.0 && s.crop == [0.0; 4] && !s.flip {
        return None;
    }
    let mut img = if s.straighten != 0.0 {
        let (w, h) = (src.width() as f32, src.height() as f32);
        let a = s.straighten.to_radians();
        let (sn, cs) = (a.sin().abs(), a.cos());
        let zoom = cs + sn * (w / h).max(h / w);
        let (ca, sa) = (a.cos(), a.sin());
        let (cx, cy) = (w / 2.0, h / 2.0);
        let mut out = RgbImage::new(src.width(), src.height());
        let raw = src.as_raw();
        let stride = src.width() as usize * 3;
        out.par_chunks_mut(stride).enumerate().for_each(|(y, row)| {
            for x in 0..src.width() as usize {
                let (dx, dy) = ((x as f32 + 0.5 - cx) / zoom, (y as f32 + 0.5 - cy) / zoom);
                let sx = (cx + dx * ca + dy * sa - 0.5).clamp(0.0, w - 1.001);
                let sy = (cy - dx * sa + dy * ca - 0.5).clamp(0.0, h - 1.001);
                let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
                let (fx, fy) = (sx - x0 as f32, sy - y0 as f32);
                for k in 0..3 {
                    let at = |xx: usize, yy: usize| raw[yy * stride + xx * 3 + k] as f32;
                    let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1, y0) * fx;
                    let bot = at(x0, y0 + 1) * (1.0 - fx) + at(x0 + 1, y0 + 1) * fx;
                    row[x * 3 + k] = (top * (1.0 - fy) + bot * fy).round() as u8;
                }
            }
        });
        out
    } else {
        src.clone()
    };
    if s.crop != [0.0; 4] {
        let (w, h) = (img.width() as f32, img.height() as f32);
        let c = s.crop.map(|v| v.clamp(0.0, 0.45));
        let (x0, y0) = ((w * c[0]) as u32, (h * c[1]) as u32);
        let cw = ((w * (1.0 - c[0] - c[2])) as u32).max(1);
        let ch = ((h * (1.0 - c[1] - c[3])) as u32).max(1);
        img = imageops::crop_imm(&img, x0, y0, cw, ch).to_image();
    }
    if s.flip {
        imageops::flip_horizontal_in_place(&mut img);
    }
    Some(img)
}

pub fn process(src: &RgbImage, s: &Settings) -> RgbImage {
    let geo = geometry(src, s);
    let src = geo.as_ref().unwrap_or(src);
    let lut: Vec<f32> = (0..256).map(|i| to_lin(i as f32 / 255.0)).collect();
    let gain = 2f32.powf(s.exposure);
    let t = s.temperature / 100.0;
    let ti = s.tint / 100.0;
    let wb = [gain * (1.0 + 0.25 * t), gain * (1.0 - 0.2 * ti), gain * (1.0 - 0.25 * t)];
    let contrast = 1.0 + s.contrast / 100.0;
    let (w, h) = src.dimensions();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let maxd = (cx * cx + cy * cy).sqrt();
    let use_hsl = s.hsl.iter().flatten().any(|v| *v != 0.0);
    let use_split = s.split_shadow_sat > 0.0 || s.split_high_sat > 0.0;
    let tint_sh = hsl_to_rgb(s.split_shadow_hue, 1.0, 0.5);
    let tint_hi = hsl_to_rgb(s.split_high_hue, 1.0, 0.5);
    let haze = s.dehaze / 100.0;
    let mut out = RgbImage::new(w, h);
    out.par_chunks_mut(w as usize * 3).enumerate().for_each(|(y, row)| {
        for x in 0..w as usize {
            let p = src.get_pixel(x as u32, y as u32).0;
            let mut c = [0f32; 3];
            for k in 0..3 {
                c[k] = to_srgb(lut[p[k] as usize] * wb[k]);
            }
            if haze > 0.0 {
                let k = haze * 0.12;
                for v in &mut c {
                    *v = (*v - k) / (1.0 - k);
                }
            } else if haze < 0.0 {
                for v in &mut c {
                    *v += (0.82 - *v) * -haze * 0.45;
                }
            }
            let l = luma(c);
            let mut nl = l;
            nl += s.shadows / 100.0 * 0.5 * (1.0 - smooth(0.0, 0.5, nl)) * (1.0 - nl);
            nl += s.highlights / 100.0 * 0.5 * smooth(0.5, 1.0, nl) * nl;
            nl = nl + s.blacks / 100.0 * 0.15 * (1.0 - smooth(0.0, 0.25, nl)) + s.whites / 100.0 * 0.15 * smooth(0.75, 1.0, nl);
            nl = (nl - 0.5) * contrast + 0.5;
            for (i, k) in s.curve.iter().enumerate() {
                if *k != 0.0 {
                    let centre = 0.125 + 0.25 * i as f32;
                    nl += k / 100.0 * 0.18 * (-((nl - centre) / 0.15).powi(2)).exp();
                }
            }
            let ratio = if l > 1e-4 { nl / l } else { 1.0 };
            for v in &mut c {
                *v = if l > 1e-4 { *v * ratio } else { nl };
            }
            let l2 = luma(c);
            let maxc = c[0].max(c[1]).max(c[2]);
            let minc = c[0].min(c[1]).min(c[2]);
            let sat_now = if maxc > 1e-4 { (maxc - minc) / maxc } else { 0.0 };
            let amount = 1.0 + s.saturation / 100.0 + s.vibrance / 100.0 * (1.0 - sat_now) + haze.max(0.0) * 0.15;
            for v in &mut c {
                *v = l2 + (*v - l2) * amount.max(0.0);
            }
            if use_hsl {
                let (hh, ss, ll) = rgb_to_hsl(c);
                let adj = band_adjust(&s.hsl, hh);
                let ll = (ll + adj[2] / 100.0 * 0.3 * ss).clamp(0.0, 1.0);
                c = hsl_to_rgb(hh + adj[0] / 100.0 * 30.0, (ss * (1.0 + adj[1] / 100.0)).clamp(0.0, 1.0), ll);
            }
            if s.mono {
                c = [luma(c); 3];
            }
            if use_split {
                let l3 = luma(c).clamp(0.0, 1.0);
                let bal = s.split_balance / 100.0;
                let wsh = (1.0 - l3).powi(2) * (1.0 - bal).clamp(0.0, 2.0) * s.split_shadow_sat / 100.0;
                let whi = l3.powi(2) * (1.0 + bal).clamp(0.0, 2.0) * s.split_high_sat / 100.0;
                for k in 0..3 {
                    c[k] += (tint_sh[k] - 0.5) * wsh * 0.35 + (tint_hi[k] - 0.5) * whi * 0.35;
                }
            }
            if s.vignette != 0.0 {
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / maxd;
                let m = s.vignette_mid / 100.0;
                let fall = if m == 0.0 { d * d } else { smooth(m * 0.8, 1.0, d) };
                let f = 1.0 + s.vignette / 100.0 * fall;
                for v in &mut c {
                    *v *= f;
                }
            }
            for k in 0..3 {
                row[x * 3 + k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    });
    local(&mut out, s);
    match s.rotate % 4 {
        1 => imageops::rotate90(&out),
        2 => imageops::rotate180(&out),
        3 => imageops::rotate270(&out),
        _ => out,
    }
}

/// Neighbourhood adjustments: noise reduction, clarity, texture, sharpening, grain.
fn local(img: &mut RgbImage, s: &Settings) {
    let scale = img.width().max(img.height()) as f32 / 1800.0;
    let noise = (s.noise > 0.0).then(|| imageops::fast_blur(img, 1.2 * scale.max(0.5)));
    let big = (s.clarity != 0.0).then(|| imageops::fast_blur(img, img.width().max(img.height()) as f32 / 60.0));
    let small = (s.texture != 0.0).then(|| imageops::fast_blur(img, 2.5 * scale.max(0.5)));
    let sharp = (s.sharpen > 0.0).then(|| imageops::fast_blur(img, if s.sharpen_radius > 0.0 { s.sharpen_radius } else { 1.0 }));
    if noise.is_none() && big.is_none() && small.is_none() && sharp.is_none() && s.grain <= 0.0 {
        return;
    }
    let w = img.width();
    let gsize = ((1.0 + s.grain_size / 25.0) * scale.max(0.5)).max(1.0);
    img.par_chunks_mut(3).enumerate().for_each(|(i, px)| {
        let o = i * 3;
        let at = |im: &Option<RgbImage>, k: usize| im.as_ref().map(|b| b.as_raw()[o + k] as f32 / 255.0);
        let mut c: [f32; 3] = std::array::from_fn(|k| px[k] as f32 / 255.0);
        if let Some(_) = &noise {
            for k in 0..3 {
                let b = at(&noise, k).unwrap_or(c[k]);
                c[k] += (b - c[k]) * s.noise / 100.0;
            }
        }
        let l = luma(c).clamp(0.0, 1.0);
        let mid = 4.0 * l * (1.0 - l);
        for k in 0..3 {
            if let Some(b) = at(&big, k) {
                c[k] += (c[k] - b) * s.clarity / 100.0 * 0.7 * mid;
            }
            if let Some(b) = at(&small, k) {
                c[k] += (c[k] - b) * s.texture / 100.0 * 0.9;
            }
            if let Some(b) = at(&sharp, k) {
                c[k] += (c[k] - b) * s.sharpen / 100.0 * 1.6;
            }
        }
        if s.grain > 0.0 {
            let (x, y) = ((i as u32 % w) as f32 / gsize, (i as u32 / w) as f32 / gsize);
            let n = (hash(x as u32, y as u32) - 0.5) * s.grain / 100.0 * 0.22;
            for v in &mut c {
                *v += n;
            }
        }
        for k in 0..3 {
            px[k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    });
}

/// Red, green, blue and luminance counts, 256 bins each.
pub fn histogram(img: &RgbImage) -> [[u32; 256]; 4] {
    let mut h = [[0u32; 256]; 4];
    for p in img.pixels() {
        let [r, g, b] = p.0;
        h[0][r as usize] += 1;
        h[1][g as usize] += 1;
        h[2][b as usize] += 1;
        let l = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32).round() as usize;
        h[3][l.min(255)] += 1;
    }
    h
}

/// A starting point: exposure that brings the average to mid-grey, a little contrast and lift.
pub fn auto(img: &RgbImage) -> Settings {
    let n = (img.width() * img.height()).max(1) as f64;
    let mean: f64 = img.pixels().map(|p| to_lin((0.2126 * p.0[0] as f32 + 0.7152 * p.0[1] as f32 + 0.0722 * p.0[2] as f32) / 255.0) as f64).sum::<f64>() / n;
    let exposure = ((0.18 / mean.max(1e-4)).log2() as f32).clamp(-3.0, 3.0);
    Settings { exposure: (exposure * 100.0).round() / 100.0, contrast: 12.0, shadows: 15.0, highlights: -15.0, vibrance: 12.0, ..Default::default() }
}

pub fn presets() -> Vec<(&'static str, Settings)> {
    let teal_orange = {
        let mut hsl = [[0.0; 3]; 8];
        hsl[1] = [-10.0, 20.0, 8.0];
        hsl[4] = [10.0, 15.0, 0.0];
        hsl[5] = [-15.0, 10.0, -10.0];
        hsl
    };
    vec![
        ("Naturel", Settings::default()),
        ("Vif", Settings { contrast: 18.0, vibrance: 35.0, saturation: 8.0, clarity: 10.0, ..Default::default() }),
        ("Doux", Settings { contrast: -18.0, highlights: -20.0, shadows: 20.0, saturation: -10.0, clarity: -15.0, ..Default::default() }),
        ("Chaud", Settings { temperature: 30.0, tint: 6.0, vibrance: 10.0, ..Default::default() }),
        ("Froid", Settings { temperature: -28.0, contrast: 8.0, ..Default::default() }),
        ("Noir et blanc", Settings { mono: true, contrast: 22.0, clarity: 15.0, ..Default::default() }),
        ("Noir et blanc doux", Settings { mono: true, contrast: -10.0, shadows: 25.0, grain: 20.0, ..Default::default() }),
        ("Vintage", Settings { temperature: 22.0, contrast: -12.0, blacks: 25.0, saturation: -25.0, vignette: -35.0, grain: 30.0, split_shadow_hue: 200.0, split_shadow_sat: 25.0, split_high_hue: 40.0, split_high_sat: 30.0, ..Default::default() }),
        ("Cinéma", Settings { contrast: 20.0, temperature: -8.0, tint: -6.0, highlights: -25.0, saturation: -12.0, vignette: -25.0, hsl: teal_orange, split_shadow_hue: 190.0, split_shadow_sat: 30.0, split_high_hue: 35.0, split_high_sat: 25.0, ..Default::default() }),
        ("Haute clé", Settings { exposure: 0.6, contrast: -20.0, highlights: -30.0, whites: 20.0, ..Default::default() }),
        ("Paysage", Settings { dehaze: 25.0, clarity: 20.0, vibrance: 25.0, highlights: -30.0, shadows: 20.0, sharpen: 40.0, ..Default::default() }),
        ("Portrait", Settings { texture: -20.0, clarity: -10.0, temperature: 6.0, sharpen: 20.0, ..Default::default() }),
        ("Argentique", Settings { contrast: -8.0, blacks: 18.0, curve: [15.0, 0.0, 0.0, -10.0], grain: 35.0, grain_size: 40.0, saturation: -15.0, ..Default::default() }),
        ("Nuit", Settings { temperature: -35.0, exposure: -0.4, contrast: 25.0, dehaze: 15.0, noise: 40.0, ..Default::default() }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_is_identity_and_sliders_move() {
        let mut img = RgbImage::new(16, 8);
        for (i, p) in img.pixels_mut().enumerate() {
            let v = (i * 2) as u8;
            p.0 = [v, v.wrapping_add(40), 200];
        }
        let same = process(&img, &Settings::default());
        for (a, b) in img.pixels().zip(same.pixels()) {
            for k in 0..3 {
                assert!(a.0[k].abs_diff(b.0[k]) <= 1);
            }
        }
        let bright = process(&img, &Settings { exposure: 1.0, ..Default::default() });
        assert!(bright.get_pixel(3, 2).0[0] > img.get_pixel(3, 2).0[0]);
        let gray = process(&img, &Settings { saturation: -100.0, ..Default::default() });
        let p = gray.get_pixel(5, 5).0;
        assert!(p[0].abs_diff(p[2]) <= 1);
        let rot = process(&img, &Settings { rotate: 1, ..Default::default() });
        assert_eq!(rot.dimensions(), (8, 16));

        let h = histogram(&img);
        assert_eq!(h[2][200], 128);
        assert_eq!(h[3].iter().sum::<u32>(), 128);
        let dark = RgbImage::from_pixel(10, 10, image::Rgb([20, 20, 20]));
        assert!(auto(&dark).exposure > 1.0);
        let bright = RgbImage::from_pixel(10, 10, image::Rgb([240, 240, 240]));
        assert!(auto(&bright).exposure < 0.0);
        assert_eq!(presets()[0].1, Settings::default());
    }

    #[test]
    fn new_controls_change_the_right_things() {
        let blue = RgbImage::from_pixel(20, 20, image::Rgb([40, 40, 220]));
        let mut s = Settings::default();
        s.hsl[5][1] = -100.0;
        let p = process(&blue, &s).get_pixel(5, 5).0;
        assert!(p[0].abs_diff(p[2]) < 40, "{p:?}");
        let red = RgbImage::from_pixel(20, 20, image::Rgb([220, 40, 40]));
        assert_eq!(process(&red, &s).get_pixel(5, 5).0, process(&red, &Settings::default()).get_pixel(5, 5).0);

        let m = process(&blue, &Settings { mono: true, ..Default::default() }).get_pixel(3, 3).0;
        assert!(m[0] == m[1] && m[1] == m[2]);
        let crop = process(&blue, &Settings { crop: [0.25, 0.0, 0.25, 0.4], ..Default::default() });
        assert_eq!(crop.dimensions(), (10, 12));
        let st = process(&blue, &Settings { straighten: 10.0, ..Default::default() });
        assert_eq!(st.dimensions(), (20, 20));
        assert_eq!(st.get_pixel(0, 0).0, [40, 40, 220]);

        let mut grad = RgbImage::new(64, 64);
        for (x, _, p) in grad.enumerate_pixels_mut() {
            let v = if x < 32 { 90 } else { 160 };
            p.0 = [v, v, v];
        }
        let sharp = process(&grad, &Settings { sharpen: 100.0, ..Default::default() });
        assert!(sharp.get_pixel(31, 30).0[0] < 90 && sharp.get_pixel(32, 30).0[0] > 160);
        let grain = process(&grad, &Settings { grain: 100.0, ..Default::default() });
        assert!(grain.pixels().map(|p| p.0[0]).collect::<std::collections::HashSet<_>>().len() > 5);
        let warm = process(&grad, &Settings { split_high_hue: 30.0, split_high_sat: 100.0, ..Default::default() }).get_pixel(50, 5).0;
        assert!(warm[0] > warm[2]);
    }
}
