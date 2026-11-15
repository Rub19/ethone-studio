use image::{GrayImage, RgbImage, imageops};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use studio_fx::filters::Kind;
use std::borrow::Cow;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SpotMode {
    #[default]
    Heal,
    Clone,
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Spot {
    pub x: f32,
    pub y: f32,
    pub sx: f32,
    pub sy: f32,
    pub r: f32,
    pub feather: f32,
    pub opacity: f32,
    pub mode: SpotMode,
}

impl Default for Spot {
    fn default() -> Self {
        Spot { x: 0.5, y: 0.5, sx: 0.6, sy: 0.5, r: 0.03, feather: 0.5, opacity: 1.0, mode: SpotMode::Heal }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RedEye {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub pupil: f32,
    pub darken: f32,
}

impl Default for RedEye {
    fn default() -> Self {
        RedEye { x: 0.5, y: 0.5, r: 0.02, pupil: 0.5, darken: 0.5 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PartKind {
    #[default]
    Linear,
    Radial,
    Brush,
    Sky,
    Subject,
    Luminance,
    Color,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PartMode {
    #[default]
    Add,
    Subtract,
    Intersect,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dab {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub feather: f32,
    pub flow: f32,
    pub erase: bool,
}

impl Default for Dab {
    fn default() -> Self {
        Dab { x: 0.5, y: 0.5, r: 0.03, feather: 0.5, flow: 0.8, erase: false }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Part {
    pub kind: PartKind,
    pub mode: PartMode,
    pub invert: bool,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub rx: f32,
    pub ry: f32,
    pub angle: f32,
    pub feather: f32,
    pub dabs: Vec<Dab>,
    pub lo: f32,
    pub hi: f32,
    pub smooth: f32,
    pub hue: f32,
    pub range: f32,
}

impl Default for Part {
    fn default() -> Self {
        Part {
            kind: PartKind::Linear,
            mode: PartMode::Add,
            invert: false,
            x1: 0.5,
            y1: 0.2,
            x2: 0.5,
            y2: 0.55,
            rx: 0.25,
            ry: 0.25,
            angle: 0.0,
            feather: 0.5,
            dabs: Vec::new(),
            lo: 0.5,
            hi: 1.0,
            smooth: 0.1,
            hue: 220.0,
            range: 30.0,
        }
    }
}

impl Part {
    pub fn new(kind: PartKind) -> Self {
        let mut p = Part { kind, ..Default::default() };
        if kind == PartKind::Radial {
            (p.x1, p.y1) = (0.5, 0.5);
        }
        p
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Local {
    pub exposure: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
    pub temperature: f32,
    pub tint: f32,
    pub saturation: f32,
    pub hue: f32,
    pub clarity: f32,
    pub dehaze: f32,
    pub texture: f32,
    pub sharpness: f32,
    pub noise: f32,
    pub color_hue: f32,
    pub color_sat: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mask {
    pub name: String,
    pub on: bool,
    pub invert: bool,
    pub amount: f32,
    pub parts: Vec<Part>,
    pub adj: Local,
}

impl Default for Mask {
    fn default() -> Self {
        Mask { name: String::new(), on: true, invert: false, amount: 100.0, parts: Vec::new(), adj: Local::default() }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
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
    pub grad_on: bool,
    pub grad_angle: f32,
    pub grad_pos: f32,
    pub grad_feather: f32,
    pub grad_exposure: f32,
    pub grad_temp: f32,
    pub grad_sat: f32,
    pub radial_on: bool,
    pub radial_x: f32,
    pub radial_y: f32,
    pub radial_size: f32,
    pub radial_feather: f32,
    pub radial_exposure: f32,
    pub radial_temp: f32,
    pub radial_sat: f32,
    pub radial_invert: bool,
    pub split_mid_hue: f32,
    pub split_mid_sat: f32,
    pub lens_vignette: f32,
    pub distortion: f32,
    pub profile: u8,
    pub curve_rgb: Vec<[f32; 2]>,
    pub curve_r: Vec<[f32; 2]>,
    pub curve_g: Vec<[f32; 2]>,
    pub curve_b: Vec<[f32; 2]>,
    pub grade_global_hue: f32,
    pub grade_global_sat: f32,
    pub grade_shadow_lum: f32,
    pub grade_mid_lum: f32,
    pub grade_high_lum: f32,
    pub grade_global_lum: f32,
    pub grade_blend: f32,
    pub cal_shadow_tint: f32,
    pub cal_red_hue: f32,
    pub cal_red_sat: f32,
    pub cal_green_hue: f32,
    pub cal_green_sat: f32,
    pub cal_blue_hue: f32,
    pub cal_blue_sat: f32,
    pub sharpen_detail: f32,
    pub sharpen_masking: f32,
    pub noise_detail: f32,
    pub color_noise: f32,
    pub color_noise_detail: f32,
    pub color_noise_smooth: f32,
    pub ca_red: f32,
    pub ca_blue: f32,
    pub defringe_purple: f32,
    pub defringe_green: f32,
    pub lens_vignette_mid: f32,
    pub persp_v: f32,
    pub persp_h: f32,
    pub persp_rotate: f32,
    pub persp_aspect: f32,
    pub persp_scale: f32,
    pub persp_x: f32,
    pub persp_y: f32,
    pub flip_v: bool,
    pub vignette_style: u8,
    pub vignette_round: f32,
    pub vignette_feather: f32,
    pub vignette_highlights: f32,
    pub grain_rough: f32,
    pub glow: f32,
    pub fade: f32,
    pub spots: Vec<Spot>,
    pub red_eyes: Vec<RedEye>,
    pub masks: Vec<Mask>,
    pub creative: Vec<Creative>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Creative {
    pub kind: String,
    pub vals: Vec<f32>,
    pub amount: f32,
    pub on: bool,
}

pub fn creative_groups() -> Vec<(&'static str, &'static [Kind])> {
    let mut g: Vec<(&'static str, &'static [Kind])> = vec![("Réglages", studio_fx::filters::ADJUSTMENTS), ("Rapides", studio_fx::filters::QUICK)];
    g.extend(studio_fx::fx::GROUPS.iter().copied());
    g.extend(studio_fx::filters::FILTERS.iter().copied());
    g
}

pub fn creative_kind(name: &str) -> Option<Kind> {
    creative_groups().into_iter().flat_map(|g| g.1.iter().copied()).find(|k| format!("{k:?}") == name)
}

fn apply_creative(out: RgbImage, list: &[Creative]) -> RgbImage {
    let live: Vec<(Kind, &Creative)> = list.iter().filter(|c| c.on && c.amount > 0.0).filter_map(|c| creative_kind(&c.kind).map(|k| (k, c))).collect();
    if live.is_empty() {
        return out;
    }
    let mut img = image::DynamicImage::ImageRgb8(out).into_rgba8();
    for (k, c) in live {
        let fx = k.apply(&img, &c.vals);
        let a = (c.amount / 100.0).clamp(0.0, 1.0);
        img.pixels_mut().zip(fx.pixels()).for_each(|(p, q)| {
            for i in 0..3 {
                p.0[i] = (p.0[i] as f32 + (q.0[i] as f32 - p.0[i] as f32) * a).round() as u8;
            }
        });
    }
    image::DynamicImage::ImageRgba8(img).into_rgb8()
}

pub const PROFILES: [&str; 6] = ["Couleur", "Vif", "Neutre", "Paysage", "Portrait", "Monochrome"];
pub const VIGNETTE_STYLES: [&str; 3] = ["Priorité couleur", "Priorité hautes lumières", "Superposition de peinture"];
pub const WB_PRESETS: [(&str, f32, f32); 6] = [("Lumière du jour", 0.0, 0.0), ("Nuageux", 18.0, 4.0), ("Ombre", 32.0, 6.0), ("Tungstène", -55.0, 8.0), ("Fluorescent", -25.0, 25.0), ("Flash", 10.0, 0.0)];
pub const RATIOS: [(&str, f32); 11] = [
    ("Libre", 0.0),
    ("1:1", 1.0),
    ("4:5", 0.8),
    ("5:4", 1.25),
    ("3:2", 1.5),
    ("2:3", 2.0 / 3.0),
    ("4:3", 4.0 / 3.0),
    ("3:4", 0.75),
    ("16:9", 16.0 / 9.0),
    ("9:16", 9.0 / 16.0),
    ("2:1", 2.0),
];

pub fn crop_for_ratio(w: f32, h: f32, ratio: f32) -> [f32; 4] {
    if ratio <= 0.0 {
        return [0.0; 4];
    }
    if w / h > ratio {
        let keep = h * ratio / w;
        let side = (1.0 - keep) / 2.0;
        [side, 0.0, side, 0.0]
    } else {
        let keep = w / ratio / h;
        let side = (1.0 - keep) / 2.0;
        [0.0, side, 0.0, side]
    }
}

pub fn auto_white_balance(img: &RgbImage) -> (f32, f32) {
    let n = (img.width() * img.height()).max(1) as f64;
    let mut sum = [0f64; 3];
    for p in img.pixels() {
        for k in 0..3 {
            sum[k] += to_lin(p.0[k] as f32 / 255.0) as f64;
        }
    }
    let avg = sum.map(|v| (v / n).max(1e-4));
    let t = ((avg[2] / avg[0]).ln() as f32 * 100.0 / 0.5).clamp(-100.0, 100.0);
    let g = (avg[0] + avg[2]) / 2.0;
    let ti = (((avg[1] / g).ln()) as f32 * 100.0 / 0.4).clamp(-100.0, 100.0);
    ((t * 0.5).round(), (ti * 0.5).round())
}

pub fn wb_from_pixel(p: [f32; 3]) -> (f32, f32) {
    let [r, g, b] = p.map(|v| to_lin(v.clamp(0.0, 1.0)).max(1e-4));
    let t = ((b - r) / (0.25 * (r + b))).clamp(-1.0, 1.0);
    let gray = r * (1.0 + 0.25 * t);
    let ti = ((1.0 - gray / g) / 0.2).clamp(-1.0, 1.0);
    ((t * 100.0).round(), (ti * 100.0).round())
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

pub fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

pub fn rgb_to_hsl(c: [f32; 3]) -> (f32, f32, f32) {
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

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
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

fn local_adjust(c: &mut [f32; 3], w: f32, exposure: f32, temp: f32, sat: f32) {
    if w <= 0.0 {
        return;
    }
    let gain = 2f32.powf(exposure * w);
    for v in c.iter_mut() {
        *v *= gain;
    }
    let t = temp / 100.0 * 0.12 * w;
    c[0] += t;
    c[2] -= t;
    let l = luma(*c);
    let k = 1.0 + sat / 100.0 * w;
    for v in c.iter_mut() {
        *v = l + (*v - l) * k.max(0.0);
    }
}

fn tone(nl: f32, shadows: f32, highlights: f32, whites: f32, blacks: f32, contrast: f32) -> f32 {
    let mut nl = nl;
    nl += shadows / 100.0 * 0.5 * (1.0 - smooth(0.0, 0.5, nl)) * (1.0 - nl);
    nl += highlights / 100.0 * 0.5 * smooth(0.5, 1.0, nl) * nl;
    nl = nl + blacks / 100.0 * 0.15 * (1.0 - smooth(0.0, 0.25, nl)) + whites / 100.0 * 0.15 * smooth(0.75, 1.0, nl);
    (nl - 0.5) * contrast + 0.5
}

fn set_luma(c: &mut [f32; 3], l: f32, nl: f32) {
    let ratio = if l > 1e-4 { nl / l } else { 1.0 };
    for v in c.iter_mut() {
        *v = if l > 1e-4 { *v * ratio } else { nl };
    }
}

fn mask_adjust(c: &mut [f32; 3], w: f32, a: &Local) {
    if w <= 1e-4 {
        return;
    }
    if a.exposure != 0.0 {
        let g = 2f32.powf(a.exposure * w);
        for v in c.iter_mut() {
            *v *= g;
        }
    }
    if a.temperature != 0.0 || a.tint != 0.0 {
        let t = a.temperature / 100.0 * 0.12 * w;
        c[0] += t;
        c[2] -= t;
        c[1] -= a.tint / 100.0 * 0.1 * w;
    }
    if a.dehaze != 0.0 {
        let k = a.dehaze / 100.0 * w;
        for v in c.iter_mut() {
            *v = if k > 0.0 { (*v - k * 0.12) / (1.0 - k * 0.12) } else { *v + (0.82 - *v) * -k * 0.45 };
        }
    }
    if a.contrast != 0.0 || a.highlights != 0.0 || a.shadows != 0.0 || a.whites != 0.0 || a.blacks != 0.0 {
        let l = luma(*c);
        let nl = tone(l.clamp(0.0, 1.0), a.shadows * w, a.highlights * w, a.whites * w, a.blacks * w, 1.0 + a.contrast / 100.0 * w);
        set_luma(c, l, nl);
    }
    if a.saturation != 0.0 {
        let l = luma(*c);
        let k = (1.0 + a.saturation / 100.0 * w).max(0.0);
        for v in c.iter_mut() {
            *v = l + (*v - l) * k;
        }
    }
    if a.hue != 0.0 {
        let (h, s, l) = rgb_to_hsl(*c);
        *c = hsl_to_rgb(h + a.hue * 0.6 * w, s, l);
    }
    if a.color_sat > 0.0 {
        let t = hsl_to_rgb(a.color_hue, 1.0, 0.5);
        for k in 0..3 {
            c[k] += (t[k] - 0.5) * w * a.color_sat / 100.0 * 0.5;
        }
    }
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

pub fn band_weights(hue: f32) -> [(usize, f32); 2] {
    let h = hue.rem_euclid(360.0);
    for i in 0..8 {
        let (a, b) = (BANDS[i].1, if i == 7 { 360.0 } else { BANDS[i + 1].1 });
        if h >= a && h < b {
            let t = (h - a) / (b - a);
            return [(i, 1.0 - t), ((i + 1) % 8, t)];
        }
    }
    [(0, 1.0), (1, 0.0)]
}

pub fn curve_lut(pts: &[[f32; 2]]) -> Option<Vec<f32>> {
    if pts.len() < 2 {
        return None;
    }
    let mut p: Vec<[f32; 2]> = pts.to_vec();
    p.sort_by(|a, b| a[0].total_cmp(&b[0]));
    p.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-4);
    if p.len() < 2 || p.iter().all(|q| (q[0] - q[1]).abs() < 1e-4) && p.first().is_some_and(|q| q[0] <= 1e-4) && p.last().is_some_and(|q| q[0] >= 1.0 - 1e-4) {
        return None;
    }
    let n = p.len();
    let d: Vec<f32> = (0..n - 1).map(|i| (p[i + 1][1] - p[i][1]) / (p[i + 1][0] - p[i][0]).max(1e-5)).collect();
    let mut m = vec![0f32; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for i in 1..n - 1 {
        m[i] = if d[i - 1] * d[i] <= 0.0 { 0.0 } else { (d[i - 1] + d[i]) / 2.0 };
    }
    for i in 0..n - 1 {
        if d[i].abs() < 1e-6 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
        } else {
            let (a, b) = (m[i] / d[i], m[i + 1] / d[i]);
            let s = a * a + b * b;
            if s > 9.0 {
                let t = 3.0 / s.sqrt();
                m[i] = t * a * d[i];
                m[i + 1] = t * b * d[i];
            }
        }
    }
    Some(
        (0..256)
            .map(|k| {
                let x = k as f32 / 255.0;
                if x <= p[0][0] {
                    return p[0][1].clamp(0.0, 1.0);
                }
                if x >= p[n - 1][0] {
                    return p[n - 1][1].clamp(0.0, 1.0);
                }
                let i = (0..n - 1).find(|&i| x <= p[i + 1][0]).unwrap_or(n - 2);
                let h = p[i + 1][0] - p[i][0];
                let t = (x - p[i][0]) / h;
                let (t2, t3) = (t * t, t * t * t);
                let y = (2.0 * t3 - 3.0 * t2 + 1.0) * p[i][1] + (t3 - 2.0 * t2 + t) * h * m[i] + (-2.0 * t3 + 3.0 * t2) * p[i + 1][1] + (t3 - t2) * h * m[i + 1];
                y.clamp(0.0, 1.0)
            })
            .collect(),
    )
}

fn lut_at(l: &[f32], v: f32) -> f32 {
    let x = v.clamp(0.0, 1.0) * 255.0;
    let i = (x as usize).min(254);
    let t = x - i as f32;
    l[i] + (l[i + 1] - l[i]) * t
}

#[derive(Clone, Copy)]
struct Warp {
    cx: f32,
    cy: f32,
    zoom: f32,
    cs: f32,
    sn: f32,
    ox: f32,
    oy: f32,
    aspect: f32,
    kv: f32,
    kh: f32,
    kd: f32,
    dscale: f32,
    norm: f32,
    ca_r: f32,
    ca_b: f32,
    identity: bool,
}

impl Warp {
    fn new(s: &Settings, w: u32, h: u32) -> Self {
        let (w, h) = (w as f32, h as f32);
        let a = (s.straighten + s.persp_rotate).to_radians();
        let (sn, cs) = (a.sin(), a.cos());
        let zoom = (cs + sn.abs() * (w / h).max(h / w)) * (1.0 + s.persp_scale / 100.0).max(0.1);
        let (cx, cy) = (w / 2.0, h / 2.0);
        let kd = -s.distortion / 100.0 * 0.35;
        let identity = s.straighten == 0.0 && s.persp_rotate == 0.0 && s.persp_scale == 0.0 && s.persp_x == 0.0 && s.persp_y == 0.0 && s.persp_aspect == 0.0 && s.persp_v == 0.0 && s.persp_h == 0.0 && s.distortion == 0.0 && s.ca_red == 0.0 && s.ca_blue == 0.0;
        Warp {
            cx,
            cy,
            zoom,
            cs,
            sn,
            ox: s.persp_x / 100.0 * cx,
            oy: s.persp_y / 100.0 * cy,
            aspect: s.persp_aspect / 100.0 * 0.5,
            kv: s.persp_v / 100.0 * 0.35,
            kh: s.persp_h / 100.0 * 0.35,
            kd,
            dscale: 1.0 / (1.0 + kd.max(0.0)),
            norm: (cx * cx + cy * cy).sqrt(),
            ca_r: s.ca_red / 100.0 * 0.004,
            ca_b: s.ca_blue / 100.0 * 0.004,
            identity,
        }
    }

    fn inv(&self, x: f32, y: f32) -> (f32, f32) {
        let (dx, dy) = ((x - self.cx) / self.zoom - self.ox, (y - self.cy) / self.zoom - self.oy);
        let (mut rx, mut ry) = (dx * self.cs + dy * self.sn, -dx * self.sn + dy * self.cs);
        if self.aspect != 0.0 {
            rx /= 1.0 + self.aspect;
            ry *= 1.0 + self.aspect * 0.5;
        }
        if self.kv != 0.0 || self.kh != 0.0 {
            let (nx, ny) = (rx / self.cx, ry / self.cy);
            rx *= 1.0 + self.kv * ny;
            ry *= 1.0 + self.kh * nx;
        }
        if self.kd != 0.0 {
            let r2 = (rx * rx + ry * ry) / (self.norm * self.norm);
            let f = (1.0 + self.kd * r2) * self.dscale;
            rx *= f;
            ry *= f;
        }
        (self.cx + rx, self.cy + ry)
    }
}

fn bilinear(raw: &[u8], w: usize, h: usize, sx: f32, sy: f32, k: usize) -> f32 {
    let sx = (sx - 0.5).clamp(0.0, w as f32 - 1.001);
    let sy = (sy - 0.5).clamp(0.0, h as f32 - 1.001);
    let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (sx - x0 as f32, sy - y0 as f32);
    let at = |xx: usize, yy: usize| raw[(yy * w + xx) * 3 + k] as f32;
    let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
    let bot = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
    top * (1.0 - fy) + bot * fy
}

fn warp_image(src: &RgbImage, wp: &Warp) -> RgbImage {
    let (w, h) = (src.width() as usize, src.height() as usize);
    let raw = src.as_raw();
    let mut out = RgbImage::new(src.width(), src.height());
    out.par_chunks_mut(w * 3).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let (sx, sy) = wp.inv(x as f32 + 0.5, y as f32 + 0.5);
            for k in 0..3 {
                let ca = if k == 0 { wp.ca_r } else if k == 2 { wp.ca_b } else { 0.0 };
                let (px, py) = if ca != 0.0 { (wp.cx + (sx - wp.cx) * (1.0 + ca), wp.cy + (sy - wp.cy) * (1.0 + ca)) } else { (sx, sy) };
                row[x * 3 + k] = bilinear(raw, w, h, px, py, k).round() as u8;
            }
        }
    });
    out
}

fn crop_of(s: &Settings) -> [f32; 4] {
    s.crop.map(|v| v.clamp(0.0, 0.45))
}

fn geometry(src: &RgbImage, s: &Settings) -> Option<RgbImage> {
    let wp = Warp::new(s, src.width(), src.height());
    if wp.identity && s.crop == [0.0; 4] && !s.flip && !s.flip_v {
        return None;
    }
    let mut img = if wp.identity { src.clone() } else { warp_image(src, &wp) };
    if s.crop != [0.0; 4] {
        let (w, h) = (img.width() as f32, img.height() as f32);
        let c = crop_of(s);
        let (x0, y0) = ((w * c[0]) as u32, (h * c[1]) as u32);
        let cw = ((w * (1.0 - c[0] - c[2])) as u32).max(1);
        let ch = ((h * (1.0 - c[1] - c[3])) as u32).max(1);
        img = imageops::crop_imm(&img, x0, y0, cw, ch).to_image();
    }
    if s.flip {
        imageops::flip_horizontal_in_place(&mut img);
    }
    if s.flip_v {
        imageops::flip_vertical_in_place(&mut img);
    }
    Some(img)
}

#[derive(Clone, Copy)]
pub struct View {
    warp: Warp,
    w: f32,
    h: f32,
    crop: [f32; 4],
    flip: bool,
    flip_v: bool,
    rotate: u8,
}

impl View {
    pub fn new(s: &Settings, w: u32, h: u32) -> Self {
        View { warp: Warp::new(s, w, h), w: w as f32, h: h as f32, crop: crop_of(s), flip: s.flip, flip_v: s.flip_v, rotate: s.rotate % 4 }
    }

    pub fn geo_to_src(&self, u: f32, v: f32) -> (f32, f32) {
        let u = if self.flip { 1.0 - u } else { u };
        let v = if self.flip_v { 1.0 - v } else { v };
        let c = self.crop;
        let (x, y) = ((c[0] + u * (1.0 - c[0] - c[2])) * self.w, (c[1] + v * (1.0 - c[1] - c[3])) * self.h);
        let (sx, sy) = if self.warp.identity { (x, y) } else { self.warp.inv(x, y) };
        (sx / self.w, sy / self.h)
    }

    pub fn to_src(&self, u: f32, v: f32) -> (f32, f32) {
        let (gu, gv) = match self.rotate {
            1 => (v, 1.0 - u),
            2 => (1.0 - u, 1.0 - v),
            3 => (1.0 - v, u),
            _ => (u, v),
        };
        self.geo_to_src(gu, gv)
    }

    pub fn from_src(&self, su: f32, sv: f32) -> (f32, f32) {
        let (mut u, mut v) = (su, sv);
        for _ in 0..12 {
            let (fu, fv) = self.to_src(u, v);
            let (ex, ey) = (fu - su, fv - sv);
            if ex.abs() + ey.abs() < 1e-6 {
                break;
            }
            let e = 1e-3;
            let (au, av) = self.to_src(u + e, v);
            let (bu, bv) = self.to_src(u, v + e);
            let (j00, j10, j01, j11) = ((au - fu) / e, (av - fv) / e, (bu - fu) / e, (bv - fv) / e);
            let det = j00 * j11 - j01 * j10;
            if det.abs() < 1e-9 {
                break;
            }
            u -= (j11 * ex - j01 * ey) / det;
            v -= (-j10 * ex + j00 * ey) / det;
        }
        (u, v)
    }

    pub fn src_aspect(&self) -> f32 {
        self.w / self.h
    }
}

fn ring(raw: &[u8], w: usize, h: usize, cx: f32, cy: f32, rx: f32, ry: f32, n: usize) -> Vec<[f32; 3]> {
    (0..n)
        .map(|k| {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            let mut acc = [0f32; 3];
            for f in [1.08, 1.18, 1.28] {
                let (x, y) = (cx + a.cos() * rx * f, cy + a.sin() * ry * f);
                for (c, v) in acc.iter_mut().enumerate() {
                    *v += bilinear(raw, w, h, x, y, c) / 3.0;
                }
            }
            acc
        })
        .collect()
}

pub fn find_source(img: &RgbImage, x: f32, y: f32, r: f32) -> (f32, f32) {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let raw = img.as_raw();
    let (px, py, rx) = (x * w as f32, y * h as f32, r * w as f32);
    let target = ring(raw, w, h, px, py, rx, rx, 24);
    let mut best = (f32::MAX, (x + r * 2.5).min(0.98), y);
    for dist in [2.3f32, 3.2, 4.5] {
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            let (cx, cy) = (px + a.cos() * rx * dist, py + a.sin() * rx * dist);
            if cx - rx * 1.3 < 0.0 || cy - rx * 1.3 < 0.0 || cx + rx * 1.3 > w as f32 || cy + rx * 1.3 > h as f32 {
                continue;
            }
            let cand = ring(raw, w, h, cx, cy, rx, rx, 24);
            let inner = ring(raw, w, h, cx, cy, rx * 0.5, rx * 0.5, 12);
            let mean: [f32; 3] = std::array::from_fn(|c| cand.iter().map(|p| p[c]).sum::<f32>() / cand.len() as f32);
            let flat: f32 = inner.iter().map(|p| (0..3).map(|c| (p[c] - mean[c]).powi(2)).sum::<f32>()).sum::<f32>() / inner.len() as f32;
            let ssd: f32 = cand.iter().zip(&target).map(|(a, b)| (0..3).map(|c| (a[c] - b[c]).powi(2)).sum::<f32>()).sum::<f32>() / 24.0;
            let score = ssd + flat * 0.6 + dist * 4.0;
            if score < best.0 {
                best = (score, cx / w as f32, cy / h as f32);
            }
        }
    }
    (best.1, best.2)
}

fn apply_spot(out: &mut RgbImage, sp: &Spot) {
    let (w, h) = (out.width() as usize, out.height() as usize);
    let (px, py, rx) = (sp.x * w as f32, sp.y * h as f32, sp.r.max(0.002) * w as f32);
    let (qx, qy) = (sp.sx * w as f32, sp.sy * h as f32);
    let (x0, x1) = ((px - rx - 1.0).max(0.0) as usize, ((px + rx + 1.0) as usize).min(w));
    let (y0, y1) = ((py - rx - 1.0).max(0.0) as usize, ((py + rx + 1.0) as usize).min(h));
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let raw = out.as_raw().clone();
    let heal = sp.mode != SpotMode::Clone;
    let n = 24;
    let (dr, sr) = (ring(&raw, w, h, px, py, rx, rx, n), ring(&raw, w, h, qx, qy, rx, rx, n));
    let diff: Vec<[f32; 3]> = dr.iter().zip(&sr).map(|(a, b)| std::array::from_fn(|c| a[c] - b[c])).collect();
    let mean: [f32; 3] = std::array::from_fn(|c| diff.iter().map(|d| d[c]).sum::<f32>() / n as f32);
    let feather = sp.feather.clamp(0.0, 1.0);
    for y in y0..y1 {
        for x in x0..x1 {
            let (dx, dy) = (x as f32 + 0.5 - px, y as f32 + 0.5 - py);
            let d = (dx * dx + dy * dy).sqrt() / rx;
            if d >= 1.0 {
                continue;
            }
            let alpha = sp.opacity.clamp(0.0, 1.0) * (1.0 - smooth(1.0 - feather.max(0.02), 1.0, d));
            if alpha <= 0.0 {
                continue;
            }
            let fa = (dy.atan2(dx) / std::f32::consts::TAU).rem_euclid(1.0) * n as f32;
            let (i0, t) = (fa as usize % n, fa.fract());
            let i1 = (i0 + 1) % n;
            let o = (y * w + x) * 3;
            for c in 0..3 {
                let s = bilinear(&raw, w, h, qx + dx + 0.5, qy + dy + 0.5, c);
                let v = if heal {
                    let edge = diff[i0][c] * (1.0 - t) + diff[i1][c] * t;
                    let k = d * d;
                    s + mean[c] * (1.0 - k) + edge * k
                } else {
                    s
                };
                let cur = raw[o + c] as f32;
                out.as_mut()[o + c] = (cur + (v - cur) * alpha).clamp(0.0, 255.0).round() as u8;
            }
        }
    }
}

fn apply_red_eye(out: &mut RgbImage, e: &RedEye) {
    let (w, h) = (out.width() as usize, out.height() as usize);
    let (px, py) = (e.x * w as f32, e.y * h as f32);
    let rx = e.r.max(0.002) * w as f32 * (0.5 + e.pupil.clamp(0.0, 1.0));
    let (x0, x1) = ((px - rx).max(0.0) as usize, ((px + rx + 1.0) as usize).min(w));
    let (y0, y1) = ((py - rx).max(0.0) as usize, ((py + rx + 1.0) as usize).min(h));
    for y in y0..y1 {
        for x in x0..x1 {
            let d = ((x as f32 + 0.5 - px).powi(2) + (y as f32 + 0.5 - py).powi(2)).sqrt() / rx;
            if d >= 1.0 {
                continue;
            }
            let p = out.get_pixel_mut(x as u32, y as u32);
            let [r, g, b] = p.0.map(|v| v as f32);
            let red = (r - g.max(b)) / r.max(1.0);
            let wgt = smooth(0.12, 0.35, red) * (1.0 - smooth(0.7, 1.0, d));
            if wgt <= 0.0 {
                continue;
            }
            let dark = 1.0 - e.darken.clamp(0.0, 1.0) * 0.6;
            let nr = (g + b) / 2.0 * dark;
            p.0 = [r + (nr - r) * wgt, g + (g * dark - g) * wgt, b + (b * dark - b) * wgt].map(|v| v.clamp(0.0, 255.0).round() as u8);
        }
    }
}

fn retouch<'a>(src: &'a RgbImage, s: &Settings) -> Cow<'a, RgbImage> {
    if s.spots.is_empty() && s.red_eyes.is_empty() {
        return Cow::Borrowed(src);
    }
    let mut out = src.clone();
    for sp in &s.spots {
        apply_spot(&mut out, sp);
    }
    for e in &s.red_eyes {
        apply_red_eye(&mut out, e);
    }
    Cow::Owned(out)
}

struct Gray {
    w: usize,
    h: usize,
    v: Vec<f32>,
}

impl Gray {
    fn at(&self, u: f32, v: f32) -> f32 {
        let x = (u * self.w as f32 - 0.5).clamp(0.0, self.w as f32 - 1.001);
        let y = (v * self.h as f32 - 0.5).clamp(0.0, self.h as f32 - 1.001);
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let g = |xx: usize, yy: usize| self.v[yy * self.w + xx];
        (g(x0, y0) * (1.0 - fx) + g(x1, y0) * fx) * (1.0 - fy) + (g(x0, y1) * (1.0 - fx) + g(x1, y1) * fx) * fy
    }

    fn blurred(w: usize, h: usize, v: Vec<f32>, sigma: f32) -> Gray {
        let g = GrayImage::from_raw(w as u32, h as u32, v.iter().map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8).collect()).unwrap_or_default();
        let b = imageops::fast_blur(&g, sigma);
        Gray { w, h, v: b.as_raw().iter().map(|x| *x as f32 / 255.0).collect() }
    }
}

fn small(src: &RgbImage, max: u32) -> RgbImage {
    let r = (max as f32 / src.width().max(src.height()) as f32).min(1.0);
    imageops::resize(src, ((src.width() as f32 * r) as u32).max(1), ((src.height() as f32 * r) as u32).max(1), imageops::FilterType::Triangle)
}

fn sky_map(src: &RgbImage) -> Gray {
    let img = small(src, 320);
    let (w, h) = (img.width() as usize, img.height() as usize);
    let px: Vec<[f32; 3]> = img.pixels().map(|p| p.0.map(|v| v as f32 / 255.0)).collect();
    let lum: Vec<f32> = px.iter().map(|c| luma(*c)).collect();
    let cand: Vec<bool> = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let (hh, ss, ll) = rgb_to_hsl(px[i]);
            let blue = (180.0..=260.0).contains(&hh) && ss > 0.08 && ll > 0.28;
            let bright = ll > 0.7 && ss < 0.35;
            let gx = lum[y * w + (x + 1).min(w - 1)] - lum[y * w + x.saturating_sub(1)];
            let gy = lum[(y + 1).min(h - 1) * w + x] - lum[y.saturating_sub(1) * w + x];
            (blue || bright) && (gx * gx + gy * gy).sqrt() < 0.12
        })
        .collect();
    let mut sky = vec![0f32; w * h];
    let mut stack: Vec<usize> = (0..w).filter(|&x| cand[x]).collect();
    for &i in &stack {
        sky[i] = 1.0;
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % w, i / w);
        let mut push = |j: usize| {
            if sky[j] == 0.0 && cand[j] && (lum[j] - lum[i]).abs() < 0.08 {
                sky[j] = 1.0;
                stack.push(j);
            }
        };
        if x > 0 {
            push(i - 1);
        }
        if x + 1 < w {
            push(i + 1);
        }
        if y > 0 {
            push(i - w);
        }
        if y + 1 < h {
            push(i + w);
        }
    }
    Gray::blurred(w, h, sky, 1.5)
}

fn subject_map(src: &RgbImage) -> Gray {
    let img = small(src, 256);
    let (w, h) = (img.width() as usize, img.height() as usize);
    let px: Vec<[f32; 3]> = img.pixels().map(|p| p.0.map(|v| v as f32 / 255.0)).collect();
    let bw = (w.min(h) / 14).max(1);
    let mut segs: Vec<[f32; 3]> = Vec::new();
    for s in 0..16 {
        let mut acc = [0f32; 3];
        let mut n = 0.0;
        for i in 0..w * h {
            let (x, y) = (i % w, i / w);
            let border = x < bw || y < bw || x >= w - bw || y >= h - bw;
            if !border {
                continue;
            }
            let a = ((y as f32 - h as f32 / 2.0).atan2(x as f32 - w as f32 / 2.0) / std::f32::consts::TAU).rem_euclid(1.0);
            if (a * 16.0) as usize == s {
                for k in 0..3 {
                    acc[k] += px[i][k];
                }
                n += 1.0;
            }
        }
        if n > 0.0 {
            segs.push(acc.map(|v| v / n));
        }
    }
    let sal: Vec<f32> = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let d = segs.iter().map(|s| (0..3).map(|k| (px[i][k] - s[k]).powi(2)).sum::<f32>().sqrt()).fold(f32::MAX, f32::min);
            let (nx, ny) = (x as f32 / w as f32 - 0.5, y as f32 / h as f32 - 0.5);
            d * (-(nx * nx + ny * ny) / (2.0 * 0.3 * 0.3)).exp()
        })
        .collect();
    let max = sal.iter().cloned().fold(1e-4, f32::max);
    let hist = {
        let mut hh = [0usize; 64];
        for v in &sal {
            hh[((v / max) * 63.0) as usize] += 1;
        }
        hh
    };
    let total = (w * h) as f32;
    let sum_all: f32 = hist.iter().enumerate().map(|(k, c)| k as f32 * *c as f32).sum();
    let (mut wb, mut sb, mut best, mut th) = (0.0, 0.0, 0.0, 32usize);
    for (k, c) in hist.iter().enumerate() {
        wb += *c as f32;
        if wb == 0.0 || wb == total {
            continue;
        }
        sb += k as f32 * *c as f32;
        let (mb, mf) = (sb / wb, (sum_all - sb) / (total - wb));
        let between = wb * (total - wb) * (mb - mf).powi(2);
        if between > best {
            best = between;
            th = k;
        }
    }
    let fg: Vec<bool> = sal.iter().map(|v| (v / max) * 63.0 > th as f32).collect();
    let mut label = vec![0u32; w * h];
    let (mut best_label, mut best_size, mut next) = (0u32, 0usize, 1u32);
    for start in 0..w * h {
        if !fg[start] || label[start] != 0 {
            continue;
        }
        let mut stack = vec![start];
        label[start] = next;
        let mut size = 0;
        while let Some(i) = stack.pop() {
            size += 1;
            let (x, y) = (i % w, i / w);
            for (ok, j) in [(x > 0, i.wrapping_sub(1)), (x + 1 < w, i + 1), (y > 0, i.wrapping_sub(w)), (y + 1 < h, i + w)] {
                if ok && fg[j] && label[j] == 0 {
                    label[j] = next;
                    stack.push(j);
                }
            }
        }
        if size > best_size {
            (best_size, best_label) = (size, next);
        }
        next += 1;
    }
    let mut outside = vec![false; w * h];
    let mut stack: Vec<usize> = (0..w * h).filter(|&i| (i % w == 0 || i % w == w - 1 || i / w == 0 || i / w == h - 1) && label[i] != best_label).collect();
    for &i in &stack {
        outside[i] = true;
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % w, i / w);
        for (ok, j) in [(x > 0, i.wrapping_sub(1)), (x + 1 < w, i + 1), (y > 0, i.wrapping_sub(w)), (y + 1 < h, i + w)] {
            if ok && !outside[j] && label[j] != best_label {
                outside[j] = true;
                stack.push(j);
            }
        }
    }
    Gray::blurred(w, h, outside.iter().map(|o| if *o { 0.0 } else { 1.0 }).collect(), 1.5)
}

fn brush_map(part: &Part, sw: u32, sh: u32) -> Gray {
    let r = (1024.0 / sw.max(sh) as f32).min(1.0);
    let (w, h) = (((sw as f32 * r) as usize).max(1), ((sh as f32 * r) as usize).max(1));
    let mut v = vec![0f32; w * h];
    for d in &part.dabs {
        let (px, py, rad) = (d.x * w as f32, d.y * h as f32, (d.r * w as f32).max(0.5));
        let (x0, x1) = ((px - rad).max(0.0) as usize, ((px + rad + 1.0) as usize).min(w));
        let (y0, y1) = ((py - rad).max(0.0) as usize, ((py + rad + 1.0) as usize).min(h));
        let f = d.feather.clamp(0.02, 1.0);
        for y in y0..y1 {
            for x in x0..x1 {
                let dist = ((x as f32 + 0.5 - px).powi(2) + (y as f32 + 0.5 - py).powi(2)).sqrt() / rad;
                if dist >= 1.0 {
                    continue;
                }
                let a = d.flow.clamp(0.0, 1.0) * (1.0 - smooth(1.0 - f, 1.0, dist));
                let m = &mut v[y * w + x];
                *m = if d.erase { *m * (1.0 - a) } else { *m + (1.0 - *m) * a };
            }
        }
    }
    Gray { w, h, v }
}

enum PartData {
    None,
    Map(Gray),
}

fn part_weight(p: &Part, data: &PartData, su: f32, sv: f32, aspect: f32, rgb: [f32; 3]) -> f32 {
    let w = match p.kind {
        PartKind::Linear => {
            let (dx, dy) = (p.x2 - p.x1, p.y2 - p.y1);
            let len = dx * dx + dy * dy;
            if len < 1e-8 {
                0.0
            } else {
                let t = ((su - p.x1) * dx + (sv - p.y1) * dy) / len;
                1.0 - smooth(0.0, 1.0, t)
            }
        }
        PartKind::Radial => {
            let (dx, dy) = ((su - p.x1) * aspect, sv - p.y1);
            let a = p.angle.to_radians();
            let (ex, ey) = (dx * a.cos() + dy * a.sin(), -dx * a.sin() + dy * a.cos());
            let d = ((ex / (p.rx.max(0.005) * aspect)).powi(2) + (ey / p.ry.max(0.005)).powi(2)).sqrt();
            1.0 - smooth(1.0 - p.feather.clamp(0.02, 1.0), 1.0, d)
        }
        PartKind::Brush | PartKind::Sky | PartKind::Subject => match data {
            PartData::Map(g) => g.at(su, sv),
            PartData::None => 0.0,
        },
        PartKind::Luminance => {
            let l = luma(rgb);
            let sm = p.smooth.max(0.005);
            smooth(p.lo - sm, p.lo, l) * (1.0 - smooth(p.hi, p.hi + sm, l))
        }
        PartKind::Color => {
            let (hh, ss, _) = rgb_to_hsl(rgb);
            let d = ((hh - p.hue + 180.0).rem_euclid(360.0) - 180.0).abs();
            (1.0 - smooth(p.range * 0.6, p.range.max(1.0), d)) * smooth(0.05, 0.2, ss)
        }
        PartKind::All => 1.0,
    };
    if p.invert { 1.0 - w } else { w }
}

fn mask_maps(src: &RgbImage, geo: &RgbImage, masks: &[&Mask], view: &View) -> Vec<f32> {
    let nm = masks.len();
    let (w, h) = (geo.width() as usize, geo.height() as usize);
    if nm == 0 {
        return Vec::new();
    }
    let datas: Vec<Vec<PartData>> = masks
        .iter()
        .map(|m| {
            m.parts
                .iter()
                .map(|p| match p.kind {
                    PartKind::Brush => PartData::Map(brush_map(p, src.width(), src.height())),
                    PartKind::Sky => PartData::Map(sky_map(src)),
                    PartKind::Subject => PartData::Map(subject_map(src)),
                    _ => PartData::None,
                })
                .collect()
        })
        .collect();
    let aspect = view.src_aspect();
    let raw = geo.as_raw();
    let mut out = vec![0f32; w * h * nm];
    out.par_chunks_mut(w * nm).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let (su, sv) = view.geo_to_src((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
            let o = (y * w + x) * 3;
            let rgb = [raw[o] as f32 / 255.0, raw[o + 1] as f32 / 255.0, raw[o + 2] as f32 / 255.0];
            for (m, mask) in masks.iter().enumerate() {
                let mut acc = 0f32;
                for (p, part) in mask.parts.iter().enumerate() {
                    let pw = part_weight(part, &datas[m][p], su, sv, aspect, rgb);
                    acc = match part.mode {
                        PartMode::Add => acc.max(pw),
                        PartMode::Subtract => acc * (1.0 - pw),
                        PartMode::Intersect => acc * pw,
                    };
                }
                if mask.invert {
                    acc = 1.0 - acc;
                }
                row[x * nm + m] = acc * (mask.amount / 100.0).clamp(0.0, 1.0);
            }
        }
    });
    out
}

pub fn mask_overlay(src: &RgbImage, s: &Settings, index: usize) -> Option<(u32, u32, Vec<f32>)> {
    let m = s.masks.get(index)?;
    let src = retouch(src, s);
    let geo = geometry(&src, s);
    let g = geo.as_ref().unwrap_or(&src);
    let view = View::new(s, src.width(), src.height());
    let mut one = m.clone();
    one.on = true;
    Some((g.width(), g.height(), mask_maps(&src, g, &[&one], &view)))
}

fn effective(s: &Settings) -> Cow<'_, Settings> {
    if s.profile == 0 || s.profile as usize >= PROFILES.len() {
        return Cow::Borrowed(s);
    }
    let mut e = s.clone();
    match s.profile {
        1 => {
            e.contrast += 10.0;
            e.saturation += 12.0;
            e.vibrance += 10.0;
        }
        2 => {
            e.contrast -= 15.0;
            e.saturation -= 8.0;
            e.highlights -= 10.0;
            e.shadows += 10.0;
        }
        3 => {
            e.vibrance += 10.0;
            for (b, d) in [(3usize, 18.0f32), (4, 15.0), (5, 15.0)] {
                e.hsl[b][1] += d;
            }
            e.hsl[5][2] -= 8.0;
            e.contrast += 6.0;
        }
        4 => {
            e.hsl[0][1] -= 8.0;
            e.hsl[1][1] -= 6.0;
            e.hsl[1][2] += 8.0;
            e.contrast -= 4.0;
        }
        _ => e.mono = true,
    }
    Cow::Owned(e)
}

pub fn process(src: &RgbImage, s: &Settings) -> RgbImage {
    let eff = effective(s);
    let s = &*eff;
    let src = retouch(src, s);
    let src = &*src;
    let geo = geometry(src, s);
    let img = geo.as_ref().unwrap_or(src);
    let view = View::new(s, src.width(), src.height());
    let masks: Vec<&Mask> = s.masks.iter().filter(|m| m.on && !m.parts.is_empty()).collect();
    let maps = mask_maps(src, img, &masks, &view);
    let nm = masks.len();
    let lut: Vec<f32> = (0..256).map(|i| to_lin(i as f32 / 255.0)).collect();
    let gain = 2f32.powf(s.exposure);
    let t = s.temperature / 100.0;
    let ti = s.tint / 100.0;
    let wb = [gain * (1.0 + 0.25 * t), gain * (1.0 - 0.2 * ti), gain * (1.0 - 0.25 * t)];
    let contrast = 1.0 + s.contrast / 100.0;
    let (w, h) = img.dimensions();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let maxd = (cx * cx + cy * cy).sqrt();
    let use_hsl = s.hsl.iter().flatten().any(|v| *v != 0.0);
    let tint_sh = hsl_to_rgb(s.split_shadow_hue, 1.0, 0.5);
    let tint_hi = hsl_to_rgb(s.split_high_hue, 1.0, 0.5);
    let tint_mid = hsl_to_rgb(s.split_mid_hue, 1.0, 0.5);
    let tint_gl = hsl_to_rgb(s.grade_global_hue, 1.0, 0.5);
    let use_split = s.split_shadow_sat > 0.0 || s.split_high_sat > 0.0 || s.split_mid_sat > 0.0 || s.grade_global_sat > 0.0 || s.grade_shadow_lum != 0.0 || s.grade_mid_lum != 0.0 || s.grade_high_lum != 0.0 || s.grade_global_lum != 0.0;
    let gp = 2.0 * 2f32.powf(-s.grade_blend / 50.0);
    let (ga, gs) = (s.grad_angle.to_radians(), (s.grad_feather.clamp(0.02, 1.0)) * 0.5);
    let (gdx, gdy) = (ga.sin(), -ga.cos());
    let haze = s.dehaze / 100.0;
    let luts = [&s.curve_rgb, &s.curve_r, &s.curve_g, &s.curve_b].map(|c| curve_lut(c));
    let use_cal = s.cal_red_hue != 0.0 || s.cal_red_sat != 0.0 || s.cal_green_hue != 0.0 || s.cal_green_sat != 0.0 || s.cal_blue_hue != 0.0 || s.cal_blue_sat != 0.0;
    let prim = |h0: f32, dh: f32, ds: f32| {
        let c = hsl_to_rgb(h0 + dh * 0.3, 1.0, 0.5);
        let l = luma(c);
        let k = 1.0 + ds / 100.0;
        c.map(|v| l + (v - l) * k)
    };
    let cal = [prim(0.0, s.cal_red_hue, s.cal_red_sat), prim(120.0, s.cal_green_hue, s.cal_green_sat), prim(240.0, s.cal_blue_hue, s.cal_blue_sat)];
    let lvm = 1.0 + s.lens_vignette_mid / 100.0 * 1.5;
    let vround = s.vignette_round / 100.0;
    let fade = s.fade / 100.0;
    let mut out = RgbImage::new(w, h);
    out.par_chunks_mut(w as usize * 3).enumerate().for_each(|(y, row)| {
        for x in 0..w as usize {
            let p = img.get_pixel(x as u32, y as u32).0;
            let mut c = [0f32; 3];
            for k in 0..3 {
                let lin = lut[p[k] as usize] * wb[k];
                c[k] = to_srgb(if s.lens_vignette != 0.0 {
                    let d2 = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)) / (maxd * maxd);
                    lin * (1.0 + s.lens_vignette / 100.0 * d2.powf(lvm) * 1.2).max(0.0)
                } else {
                    lin
                });
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
            if use_cal {
                let o = c;
                c = std::array::from_fn(|k| o[0] * cal[0][k] + o[1] * cal[1][k] + o[2] * cal[2][k]);
            }
            if s.cal_shadow_tint != 0.0 {
                let l = luma(c).clamp(0.0, 1.0);
                c[1] -= s.cal_shadow_tint / 100.0 * 0.08 * (1.0 - l).powi(2);
            }
            let l = luma(c);
            let mut nl = tone(l, s.shadows, s.highlights, s.whites, s.blacks, contrast);
            for (i, k) in s.curve.iter().enumerate() {
                if *k != 0.0 {
                    let centre = 0.125 + 0.25 * i as f32;
                    nl += k / 100.0 * 0.18 * (-((nl - centre) / 0.15).powi(2)).exp();
                }
            }
            set_luma(&mut c, l, nl);
            if let Some(m) = &luts[0] {
                for v in &mut c {
                    *v = lut_at(m, *v);
                }
            }
            for k in 0..3 {
                if let Some(m) = &luts[k + 1] {
                    c[k] = lut_at(m, c[k]);
                }
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
                let zs = (1.0 - l3).powf(gp) * (1.0 - bal).clamp(0.0, 2.0);
                let zh = l3.powf(gp) * (1.0 + bal).clamp(0.0, 2.0);
                let zm = (4.0 * l3 * (1.0 - l3)).powf(gp / 2.0);
                let (wsh, whi, wmid) = (zs * s.split_shadow_sat / 100.0, zh * s.split_high_sat / 100.0, zm * s.split_mid_sat / 100.0);
                let wgl = s.grade_global_sat / 100.0;
                let dl = (s.grade_shadow_lum * zs + s.grade_mid_lum * zm + s.grade_high_lum * zh + s.grade_global_lum) / 100.0 * 0.2;
                for k in 0..3 {
                    c[k] += (tint_sh[k] - 0.5) * wsh * 0.35 + (tint_hi[k] - 0.5) * whi * 0.35 + (tint_mid[k] - 0.5) * wmid * 0.35 + (tint_gl[k] - 0.5) * wgl * 0.3 + dl;
                }
            }
            let (u, v) = ((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
            if s.grad_on {
                let t = (u - 0.5) * gdx + (v - 0.5) * gdy + 0.5 - s.grad_pos;
                let wgt = smooth(-gs, gs, t);
                local_adjust(&mut c, wgt, s.grad_exposure, s.grad_temp, s.grad_sat);
            }
            if s.radial_on {
                let size = s.radial_size.max(0.02);
                let d = (((u - s.radial_x) / size).powi(2) + ((v - s.radial_y) / size * h as f32 / w as f32).powi(2)).sqrt();
                let f = s.radial_feather.clamp(0.02, 1.0);
                let mut wgt = 1.0 - smooth(1.0 - f, 1.0, d);
                if s.radial_invert {
                    wgt = 1.0 - wgt;
                }
                local_adjust(&mut c, wgt, s.radial_exposure, s.radial_temp, s.radial_sat);
            }
            let i = y * w as usize + x;
            for (m, mask) in masks.iter().enumerate() {
                mask_adjust(&mut c, maps[i * nm + m], &mask.adj);
            }
            if s.vignette != 0.0 {
                let dc = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / maxd;
                let d = if vround == 0.0 {
                    dc
                } else {
                    let (ex, ey) = ((x as f32 - cx) / cx, (y as f32 - cy) / cy);
                    let other = if vround > 0.0 { (ex * ex + ey * ey).sqrt() / std::f32::consts::SQRT_2 } else { ex.abs().max(ey.abs()) };
                    dc + (other - dc) * vround.abs()
                };
                let m = s.vignette_mid / 100.0;
                let fall = if s.vignette_feather != 0.0 {
                    let f = (0.5 + s.vignette_feather / 200.0).clamp(0.02, 1.0);
                    let start = m * 0.8;
                    smooth(start + (1.0 - start) * (1.0 - f) * 0.9, 1.0, d)
                } else if m == 0.0 {
                    d * d
                } else {
                    smooth(m * 0.8, 1.0, d)
                };
                let amt = s.vignette / 100.0;
                match s.vignette_style {
                    1 => {
                        let l = luma(c).clamp(0.0, 1.0);
                        let protect = if amt < 0.0 { 1.0 - s.vignette_highlights / 100.0 * smooth(0.55, 1.0, l) } else { 1.0 };
                        let f = 1.0 + amt * fall * protect;
                        for v in &mut c {
                            *v = to_srgb(to_lin(v.clamp(0.0, 1.5)) * f.max(0.0));
                        }
                    }
                    2 => {
                        let target = if amt < 0.0 { 0.0 } else { 1.0 };
                        for v in &mut c {
                            *v += (target - *v) * amt.abs() * fall;
                        }
                    }
                    _ => {
                        let f = 1.0 + amt * fall;
                        for v in &mut c {
                            *v *= f;
                        }
                    }
                }
            }
            if fade > 0.0 {
                for v in &mut c {
                    *v = 0.1 * fade + *v * (1.0 - 0.18 * fade);
                }
            }
            for k in 0..3 {
                row[x * 3 + k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    });
    local(&mut out, s, &maps, &masks);
    let out = apply_creative(out, &s.creative);
    match s.rotate % 4 {
        1 => imageops::rotate90(&out),
        2 => imageops::rotate180(&out),
        3 => imageops::rotate270(&out),
        _ => out,
    }
}

fn local(img: &mut RgbImage, s: &Settings, maps: &[f32], masks: &[&Mask]) {
    let nm = masks.len();
    let any = |f: fn(&Local) -> f32| masks.iter().any(|m| f(&m.adj) != 0.0);
    let scale = img.width().max(img.height()) as f32 / 1800.0;
    let sc = scale.max(0.5);
    let need_noise = s.noise > 0.0 || any(|a| a.noise);
    let need_clarity = s.clarity != 0.0 || any(|a| a.clarity);
    let need_texture = s.texture != 0.0 || any(|a| a.texture);
    let need_sharp = s.sharpen > 0.0 || any(|a| a.sharpness);
    let need_edge = s.sharpen_masking > 0.0 || s.noise_detail > 0.0 || s.color_noise_detail > 0.0 || s.defringe_purple > 0.0 || s.defringe_green > 0.0;
    let noise = need_noise.then(|| imageops::fast_blur(img, 1.2 * sc));
    let big = need_clarity.then(|| imageops::fast_blur(img, img.width().max(img.height()) as f32 / 60.0));
    let tex = need_texture.then(|| imageops::fast_blur(img, 2.5 * sc));
    let radius = if s.sharpen_radius > 0.0 { s.sharpen_radius } else { 1.0 };
    let sharp = need_sharp.then(|| imageops::fast_blur(img, radius));
    let fine = (need_sharp && s.sharpen_detail > 0.0).then(|| imageops::fast_blur(img, (radius * 0.5).max(0.4)));
    let edge = need_edge.then(|| imageops::fast_blur(img, 1.5 * sc));
    let chroma = (s.color_noise > 0.0).then(|| imageops::fast_blur(img, (2.0 + s.color_noise_smooth / 100.0 * 6.0) * sc));
    let glow = (s.glow > 0.0).then(|| {
        let mut b = img.clone();
        for p in b.pixels_mut() {
            let l = luma(p.0.map(|v| v as f32 / 255.0));
            let f = smooth(0.55, 1.0, l);
            p.0 = p.0.map(|v| (v as f32 * f) as u8);
        }
        imageops::fast_blur(&b, img.width().max(img.height()) as f32 / 45.0)
    });
    if noise.is_none() && big.is_none() && tex.is_none() && sharp.is_none() && s.grain <= 0.0 && edge.is_none() && chroma.is_none() && glow.is_none() {
        return;
    }
    let w = img.width();
    let gsize = ((1.0 + s.grain_size / 25.0) * sc).max(1.0);
    let rough = s.grain_rough / 100.0;
    img.par_chunks_mut(3).enumerate().for_each(|(i, px)| {
        let o = i * 3;
        let at = |im: &Option<RgbImage>, k: usize| im.as_ref().map(|b| b.as_raw()[o + k] as f32 / 255.0);
        let mut c: [f32; 3] = std::array::from_fn(|k| px[k] as f32 / 255.0);
        let mw = |f: fn(&Local) -> f32| -> f32 { masks.iter().enumerate().map(|(m, mask)| maps[i * nm + m] * f(&mask.adj)).sum() };
        let e = edge.as_ref().map(|_| {
            let b: [f32; 3] = std::array::from_fn(|k| at(&edge, k).unwrap_or(c[k]));
            ((luma(c) - luma(b)).abs() * 6.0).clamp(0.0, 1.0)
        });
        let nz = s.noise + if nm > 0 { mw(|a| a.noise) } else { 0.0 };
        if noise.is_some() && nz != 0.0 {
            let keep = e.map(|e| 1.0 - s.noise_detail / 100.0 * e).unwrap_or(1.0);
            for k in 0..3 {
                let b = at(&noise, k).unwrap_or(c[k]);
                c[k] += (b - c[k]) * (nz / 100.0).clamp(0.0, 1.0) * keep;
            }
        }
        if chroma.is_some() {
            let b: [f32; 3] = std::array::from_fn(|k| at(&chroma, k).unwrap_or(c[k]));
            let (lc, lb) = (luma(c), luma(b));
            let amt = s.color_noise / 100.0 * e.map(|e| 1.0 - s.color_noise_detail / 100.0 * e).unwrap_or(1.0);
            for k in 0..3 {
                let target = lc + (b[k] - lb);
                c[k] += (target - c[k]) * amt;
            }
        }
        let l = luma(c).clamp(0.0, 1.0);
        let mid = 4.0 * l * (1.0 - l);
        let (cl, tx, sh) = if nm > 0 { (s.clarity + mw(|a| a.clarity), s.texture + mw(|a| a.texture), s.sharpen + mw(|a| a.sharpness) * 0.8) } else { (s.clarity, s.texture, s.sharpen) };
        let smask = match e {
            Some(e) if s.sharpen_masking > 0.0 => smooth(0.0, (s.sharpen_masking / 100.0 * 0.5).max(0.01), e),
            _ => 1.0,
        };
        for k in 0..3 {
            if let Some(b) = at(&big, k) {
                c[k] += (c[k] - b) * cl / 100.0 * 0.7 * mid;
            }
            if let Some(b) = at(&tex, k) {
                c[k] += (c[k] - b) * tx / 100.0 * 0.9;
            }
            if let Some(b) = at(&sharp, k) {
                let mut hp = (c[k] - b) * sh.max(0.0) / 100.0 * 1.6;
                if let Some(f) = at(&fine, k) {
                    hp += (c[k] - f) * sh.max(0.0) / 100.0 * s.sharpen_detail / 100.0 * 1.2;
                }
                c[k] += hp * smask;
            }
        }
        if let Some(e) = e
            && (s.defringe_purple > 0.0 || s.defringe_green > 0.0)
        {
            let (hh, ss, _) = rgb_to_hsl(c);
            let purple = if (260.0..=335.0).contains(&hh) { s.defringe_purple / 100.0 } else { 0.0 };
            let green = if (70.0..=165.0).contains(&hh) { s.defringe_green / 100.0 } else { 0.0 };
            let k = (purple + green) * e * smooth(0.1, 0.3, ss);
            let lc = luma(c);
            for v in &mut c {
                *v = lc + (*v - lc) * (1.0 - k).max(0.0);
            }
        }
        if glow.is_some() {
            let g = s.glow / 100.0 * 0.9;
            for k in 0..3 {
                let b = at(&glow, k).unwrap_or(0.0);
                c[k] = 1.0 - (1.0 - c[k]) * (1.0 - b * g);
            }
        }
        if s.grain > 0.0 {
            let (xx, yy) = ((i as u32 % w) as f32, (i as u32 / w) as f32);
            let n1 = hash((xx / gsize) as u32, (yy / gsize) as u32) - 0.5;
            let n = if rough > 0.0 {
                let n2 = hash((xx / (gsize * 0.5).max(1.0)) as u32 + 7919, (yy / (gsize * 0.5).max(1.0)) as u32 + 104729) - 0.5;
                n1 * (1.0 - rough * 0.6) + n2 * rough
            } else {
                n1
            };
            let n = n * s.grain / 100.0 * 0.22;
            for v in &mut c {
                *v += n;
            }
        }
        for k in 0..3 {
            px[k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    });
}

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

pub fn auto(img: &RgbImage) -> Settings {
    let n = (img.width() * img.height()).max(1) as f64;
    let mean: f64 = img.pixels().map(|p| to_lin((0.2126 * p.0[0] as f32 + 0.7152 * p.0[1] as f32 + 0.0722 * p.0[2] as f32) / 255.0) as f64).sum::<f64>() / n;
    let exposure = ((0.18 / mean.max(1e-4)).log2() as f32).clamp(-3.0, 3.0);
    Settings { exposure: (exposure * 100.0).round() / 100.0, contrast: 12.0, shadows: 15.0, highlights: -15.0, vibrance: 12.0, ..Default::default() }
}

pub fn auto_straighten(img: &RgbImage) -> f32 {
    let im = imageops::fast_blur(&small(img, 600), 2.0);
    let (w, h) = (im.width() as usize, im.height() as usize);
    let l: Vec<f32> = im.pixels().map(|p| luma(p.0.map(|v| v as f32 / 255.0))).collect();
    let mut hist = vec![0f32; 121];
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let at = |dx: isize, dy: isize| l[(y as isize + dy) as usize * w + (x as isize + dx) as usize];
            let gx = at(1, -1) + 2.0 * at(1, 0) + at(1, 1) - at(-1, -1) - 2.0 * at(-1, 0) - at(-1, 1);
            let gy = at(-1, 1) + 2.0 * at(0, 1) + at(1, 1) - at(-1, -1) - 2.0 * at(0, -1) - at(1, -1);
            let mag = (gx * gx + gy * gy).sqrt();
            if mag < 0.25 {
                continue;
            }
            let phi = gy.atan2(gx).to_degrees() + 90.0;
            let folded = (phi + 45.0).rem_euclid(90.0) - 45.0;
            if folded.abs() <= 15.0 {
                hist[((folded + 15.0) * 4.0).round() as usize] += mag;
            }
        }
    }
    let sm: Vec<f32> = (0..hist.len()).map(|k| (k.saturating_sub(2)..(k + 3).min(hist.len())).map(|j| hist[j]).sum()).collect();
    let peak = sm.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(k, _)| k).unwrap_or(60);
    if sm[peak] <= 0.0 {
        return 0.0;
    }
    let phi = peak as f32 / 4.0 - 15.0;
    ((-phi) * 10.0).round() / 10.0
}

fn hsl_of(list: &[(usize, [f32; 3])]) -> [[f32; 3]; 8] {
    let mut h = [[0.0; 3]; 8];
    for (b, v) in list {
        h[*b] = *v;
    }
    h
}

pub fn preset_catalog() -> Vec<(&'static str, &'static str, Settings)> {
    let d = Settings::default;
    let teal_orange = hsl_of(&[(1, [-10.0, 20.0, 8.0]), (4, [10.0, 15.0, 0.0]), (5, [-15.0, 10.0, -10.0])]);
    vec![
        ("Couleur", "Naturel", d()),
        ("Couleur", "Vif", Settings { contrast: 18.0, vibrance: 35.0, saturation: 8.0, clarity: 10.0, ..d() }),
        ("Couleur", "Doux", Settings { contrast: -18.0, highlights: -20.0, shadows: 20.0, saturation: -10.0, clarity: -15.0, ..d() }),
        ("Couleur", "Chaud", Settings { temperature: 30.0, tint: 6.0, vibrance: 10.0, ..d() }),
        ("Couleur", "Froid", Settings { temperature: -28.0, contrast: 8.0, ..d() }),
        ("Couleur", "Lumineux", Settings { exposure: 0.35, shadows: 30.0, highlights: -20.0, vibrance: 15.0, ..d() }),
        ("Couleur", "Mat", Settings { fade: 45.0, contrast: -10.0, saturation: -10.0, ..d() }),
        ("Couleur", "Pastel", Settings { exposure: 0.25, contrast: -25.0, saturation: -25.0, fade: 30.0, temperature: 5.0, ..d() }),
        ("Couleurs vives", "Pop", Settings { contrast: 25.0, vibrance: 45.0, saturation: 15.0, clarity: 15.0, dehaze: 8.0, ..d() }),
        ("Couleurs vives", "Tropical", Settings { vibrance: 40.0, temperature: 12.0, hsl: hsl_of(&[(3, [-15.0, 30.0, 5.0]), (4, [0.0, 35.0, 5.0]), (5, [-10.0, 25.0, -5.0])]), ..d() }),
        ("Couleurs vives", "Néon", Settings { contrast: 30.0, saturation: 30.0, tint: 18.0, hsl: hsl_of(&[(6, [0.0, 40.0, 10.0]), (7, [0.0, 40.0, 10.0]), (4, [0.0, 30.0, 0.0])]), glow: 25.0, ..d() }),
        ("Couleurs vives", "Saturé", Settings { saturation: 40.0, vibrance: 20.0, contrast: 10.0, ..d() }),
        ("Couleurs vives", "Automne", Settings { temperature: 20.0, hsl: hsl_of(&[(1, [-5.0, 30.0, 0.0]), (2, [-20.0, 25.0, 0.0]), (3, [-40.0, -10.0, 0.0])]), vibrance: 20.0, ..d() }),
        ("Portrait", "Portrait", Settings { texture: -20.0, clarity: -10.0, temperature: 6.0, sharpen: 20.0, ..d() }),
        ("Portrait", "Peau douce", Settings { texture: -35.0, clarity: -15.0, hsl: hsl_of(&[(1, [0.0, -8.0, 10.0])]), ..d() }),
        ("Portrait", "Portrait chaleureux", Settings { temperature: 15.0, tint: 5.0, shadows: 15.0, texture: -15.0, hsl: hsl_of(&[(1, [0.0, 5.0, 8.0])]), ..d() }),
        ("Portrait", "Portrait lumineux", Settings { exposure: 0.3, highlights: -25.0, shadows: 25.0, texture: -15.0, vignette: -15.0, ..d() }),
        ("Portrait", "Portrait contrasté", Settings { contrast: 30.0, clarity: 15.0, blacks: -15.0, vignette: -25.0, saturation: -10.0, ..d() }),
        ("Portrait", "Portrait mat", Settings { fade: 35.0, contrast: -8.0, texture: -15.0, temperature: 8.0, ..d() }),
        ("Paysage", "Paysage", Settings { dehaze: 25.0, clarity: 20.0, vibrance: 25.0, highlights: -30.0, shadows: 20.0, sharpen: 40.0, ..d() }),
        ("Paysage", "Ciel dramatique", Settings { highlights: -60.0, dehaze: 30.0, clarity: 25.0, hsl: hsl_of(&[(5, [0.0, 15.0, -35.0]), (4, [0.0, 10.0, -20.0])]), ..d() }),
        ("Paysage", "Forêt", Settings { hsl: hsl_of(&[(3, [-20.0, 15.0, -10.0]), (2, [-25.0, 10.0, 0.0])]), clarity: 15.0, shadows: 20.0, temperature: -5.0, ..d() }),
        ("Paysage", "Coucher de soleil", Settings { temperature: 35.0, tint: 10.0, vibrance: 30.0, highlights: -35.0, split_high_hue: 30.0, split_high_sat: 30.0, ..d() }),
        ("Paysage", "Montagne", Settings { dehaze: 35.0, clarity: 30.0, contrast: 12.0, temperature: -8.0, sharpen: 45.0, ..d() }),
        ("Paysage", "Mer et ciel", Settings { hsl: hsl_of(&[(4, [-5.0, 30.0, 5.0]), (5, [5.0, 25.0, -10.0])]), vibrance: 20.0, dehaze: 15.0, ..d() }),
        ("Paysage", "Brume matinale", Settings { dehaze: -25.0, exposure: 0.2, contrast: -15.0, temperature: -10.0, glow: 20.0, ..d() }),
        ("Noir et blanc", "Noir et blanc", Settings { mono: true, contrast: 22.0, clarity: 15.0, ..d() }),
        ("Noir et blanc", "Noir et blanc doux", Settings { mono: true, contrast: -10.0, shadows: 25.0, grain: 20.0, ..d() }),
        ("Noir et blanc", "N&B contrasté", Settings { mono: true, contrast: 45.0, clarity: 30.0, blacks: -25.0, whites: 20.0, ..d() }),
        ("Noir et blanc", "Filtre rouge", Settings { mono: true, contrast: 20.0, hsl: hsl_of(&[(0, [0.0, 0.0, 40.0]), (1, [0.0, 0.0, 30.0]), (5, [0.0, 0.0, -50.0]), (4, [0.0, 0.0, -30.0])]), ..d() }),
        ("Noir et blanc", "Filtre jaune", Settings { mono: true, contrast: 15.0, hsl: hsl_of(&[(2, [0.0, 0.0, 30.0]), (1, [0.0, 0.0, 20.0]), (5, [0.0, 0.0, -25.0])]), ..d() }),
        ("Noir et blanc", "Filtre vert", Settings { mono: true, contrast: 12.0, hsl: hsl_of(&[(3, [0.0, 0.0, 35.0]), (0, [0.0, 0.0, -25.0])]), ..d() }),
        ("Noir et blanc", "Infrarouge", Settings { mono: true, contrast: 30.0, glow: 30.0, grain: 15.0, hsl: hsl_of(&[(3, [0.0, 0.0, 90.0]), (2, [0.0, 0.0, 60.0]), (5, [0.0, 0.0, -70.0])]), ..d() }),
        ("Noir et blanc", "Sélénium", Settings { mono: true, contrast: 18.0, split_shadow_hue: 290.0, split_shadow_sat: 25.0, split_high_hue: 40.0, split_high_sat: 12.0, ..d() }),
        ("Noir et blanc", "Sépia", Settings { mono: true, contrast: 10.0, split_mid_hue: 35.0, split_mid_sat: 45.0, split_high_hue: 40.0, split_high_sat: 20.0, fade: 15.0, ..d() }),
        ("Cinéma", "Cinéma", Settings { contrast: 20.0, temperature: -8.0, tint: -6.0, highlights: -25.0, saturation: -12.0, vignette: -25.0, hsl: teal_orange, split_shadow_hue: 190.0, split_shadow_sat: 30.0, split_high_hue: 35.0, split_high_sat: 25.0, ..d() }),
        ("Cinéma", "Teal & Orange", Settings { contrast: 15.0, hsl: teal_orange, split_shadow_hue: 185.0, split_shadow_sat: 45.0, split_high_hue: 30.0, split_high_sat: 40.0, ..d() }),
        ("Cinéma", "Blockbuster", Settings { contrast: 35.0, clarity: 20.0, blacks: -20.0, split_shadow_hue: 200.0, split_shadow_sat: 35.0, split_high_hue: 38.0, split_high_sat: 30.0, vignette: -30.0, ..d() }),
        ("Cinéma", "Nuit américaine", Settings { exposure: -1.0, temperature: -45.0, contrast: 20.0, saturation: -30.0, split_shadow_hue: 220.0, split_shadow_sat: 40.0, ..d() }),
        ("Cinéma", "Bleach bypass", Settings { saturation: -45.0, contrast: 40.0, clarity: 25.0, highlights: -15.0, ..d() }),
        ("Cinéma", "Néo-noir", Settings { contrast: 30.0, saturation: -55.0, split_shadow_hue: 170.0, split_shadow_sat: 30.0, vignette: -40.0, grain: 18.0, ..d() }),
        ("Cinéma", "Western", Settings { temperature: 30.0, saturation: -20.0, contrast: 18.0, fade: 15.0, split_high_hue: 40.0, split_high_sat: 35.0, ..d() }),
        ("Vintage", "Vintage", Settings { temperature: 22.0, contrast: -12.0, blacks: 25.0, saturation: -25.0, vignette: -35.0, grain: 30.0, split_shadow_hue: 200.0, split_shadow_sat: 25.0, split_high_hue: 40.0, split_high_sat: 30.0, ..d() }),
        ("Vintage", "Argentique", Settings { contrast: -8.0, blacks: 18.0, curve: [15.0, 0.0, 0.0, -10.0], grain: 35.0, grain_size: 40.0, saturation: -15.0, ..d() }),
        ("Vintage", "Polaroïd", Settings { fade: 40.0, temperature: 15.0, tint: 8.0, contrast: -10.0, split_shadow_hue: 180.0, split_shadow_sat: 20.0, vignette: -20.0, ..d() }),
        ("Vintage", "Diapo 70", Settings { contrast: 25.0, saturation: 15.0, temperature: 10.0, hsl: hsl_of(&[(0, [5.0, 15.0, 0.0]), (5, [-10.0, 15.0, -10.0])]), grain: 15.0, ..d() }),
        ("Vintage", "Délavé", Settings { fade: 55.0, saturation: -35.0, contrast: -20.0, ..d() }),
        ("Vintage", "Lomo", Settings { contrast: 35.0, saturation: 25.0, vignette: -60.0, vignette_mid: 20.0, tint: -10.0, split_shadow_hue: 200.0, split_shadow_sat: 25.0, ..d() }),
        ("Vintage", "Cross-process", Settings { tint: -20.0, contrast: 20.0, split_shadow_hue: 220.0, split_shadow_sat: 35.0, split_high_hue: 60.0, split_high_sat: 40.0, ..d() }),
        ("Style", "Haute clé", Settings { exposure: 0.6, contrast: -20.0, highlights: -30.0, whites: 20.0, ..d() }),
        ("Style", "Basse clé", Settings { exposure: -0.6, contrast: 25.0, shadows: -20.0, vignette: -35.0, ..d() }),
        ("Style", "Rêveur", Settings { glow: 40.0, clarity: -25.0, exposure: 0.15, saturation: -10.0, ..d() }),
        ("Style", "Dramatique", Settings { contrast: 35.0, clarity: 40.0, highlights: -40.0, shadows: 25.0, saturation: -15.0, vignette: -30.0, ..d() }),
        ("Nuit et ville", "Nuit", Settings { temperature: -35.0, exposure: -0.4, contrast: 25.0, dehaze: 15.0, noise: 40.0, ..d() }),
        ("Nuit et ville", "Urbain", Settings { contrast: 25.0, clarity: 30.0, saturation: -25.0, temperature: -10.0, sharpen: 35.0, ..d() }),
        ("Nuit et ville", "Industriel", Settings { saturation: -40.0, clarity: 35.0, contrast: 20.0, split_shadow_hue: 30.0, split_shadow_sat: 15.0, ..d() }),
        ("Nuit et ville", "Néons de nuit", Settings { tint: 15.0, temperature: -20.0, vibrance: 35.0, glow: 30.0, noise: 30.0, color_noise: 40.0, ..d() }),
        ("Correction", "Déboucher les ombres", Settings { shadows: 50.0, blacks: 10.0, ..d() }),
        ("Correction", "Récupérer le ciel", Settings { highlights: -60.0, whites: -15.0, ..d() }),
        ("Correction", "Netteté écran", Settings { sharpen: 60.0, sharpen_radius: 1.0, sharpen_masking: 30.0, ..d() }),
        ("Correction", "Réduction du bruit", Settings { noise: 50.0, noise_detail: 50.0, color_noise: 50.0, ..d() }),
        ("Correction", "Supprimer la brume", Settings { dehaze: 40.0, contrast: 5.0, ..d() }),
    ]
}

#[allow(dead_code)]
pub fn presets() -> Vec<(&'static str, Settings)> {
    preset_catalog().into_iter().map(|(_, n, s)| (n, s)).collect()
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
        let names: std::collections::HashSet<_> = presets().iter().map(|p| p.0).collect();
        assert_eq!(names.len(), presets().len());
        for old in ["Vif", "Doux", "Chaud", "Froid", "Noir et blanc", "Noir et blanc doux", "Vintage", "Cinéma", "Haute clé", "Paysage", "Portrait", "Argentique", "Nuit"] {
            assert!(names.contains(old), "{old}");
        }
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

        let flat = RgbImage::from_pixel(40, 40, image::Rgb([100, 100, 100]));
        let g = process(&flat, &Settings { grad_on: true, grad_pos: 0.5, grad_feather: 0.1, grad_exposure: -1.5, ..Default::default() });
        assert!(g.get_pixel(20, 2).0[0] < 80 && g.get_pixel(20, 37).0[0] >= 99, "{:?} {:?}", g.get_pixel(20, 2), g.get_pixel(20, 37));
        let r = process(&flat, &Settings { radial_on: true, radial_x: 0.5, radial_y: 0.5, radial_size: 0.3, radial_feather: 0.2, radial_exposure: 1.0, ..Default::default() });
        assert!(r.get_pixel(20, 20).0[0] > 120 && r.get_pixel(1, 1).0[0] <= 101);
        let ri = process(&flat, &Settings { radial_on: true, radial_x: 0.5, radial_y: 0.5, radial_size: 0.3, radial_feather: 0.2, radial_exposure: 1.0, radial_invert: true, ..Default::default() });
        assert!(ri.get_pixel(20, 20).0[0] <= 101 && ri.get_pixel(1, 1).0[0] > 120);
        let lv = process(&flat, &Settings { lens_vignette: 80.0, ..Default::default() });
        assert!(lv.get_pixel(0, 0).0[0] > lv.get_pixel(20, 20).0[0]);
        let dist = process(&grad, &Settings { distortion: 50.0, ..Default::default() });
        assert_eq!(dist.dimensions(), grad.dimensions());
        assert_eq!(crop_for_ratio(200.0, 100.0, 1.0), [0.25, 0.0, 0.25, 0.0]);
        let blue_cast = RgbImage::from_pixel(10, 10, image::Rgb([80, 100, 160]));
        assert!(auto_white_balance(&blue_cast).0 > 0.0);
    }

    #[test]
    fn masks_spots_and_curves_really_apply() {
        let flat = RgbImage::from_pixel(60, 40, image::Rgb([100, 100, 100]));
        let lin = Mask { parts: vec![Part { kind: PartKind::Linear, x1: 0.5, y1: 0.0, x2: 0.5, y2: 0.5, ..Default::default() }], adj: Local { exposure: -1.5, ..Default::default() }, ..Default::default() };
        let out = process(&flat, &Settings { masks: vec![lin.clone()], ..Default::default() });
        assert!(out.get_pixel(30, 1).0[0] < 80 && out.get_pixel(30, 38).0[0] >= 99);
        let mut sub = lin.clone();
        sub.parts.push(Part { kind: PartKind::All, mode: PartMode::Subtract, ..Default::default() });
        assert!(process(&flat, &Settings { masks: vec![sub], ..Default::default() }).get_pixel(30, 1).0[0] >= 99);
        let mut inv = lin.clone();
        inv.invert = true;
        let o = process(&flat, &Settings { masks: vec![inv], ..Default::default() });
        assert!(o.get_pixel(30, 1).0[0] >= 97 && o.get_pixel(30, 38).0[0] < 80, "{:?} {:?}", o.get_pixel(30, 1), o.get_pixel(30, 38));
        let brush = Mask { parts: vec![Part { kind: PartKind::Brush, dabs: vec![Dab { x: 0.25, y: 0.5, r: 0.1, feather: 0.1, flow: 1.0, erase: false }], ..Default::default() }], adj: Local { exposure: 1.0, ..Default::default() }, ..Default::default() };
        let b = process(&flat, &Settings { masks: vec![brush.clone()], ..Default::default() });
        assert!(b.get_pixel(15, 20).0[0] > 120 && b.get_pixel(50, 20).0[0] <= 101);
        let mut erased = brush.clone();
        erased.parts[0].dabs.push(Dab { x: 0.25, y: 0.5, r: 0.12, feather: 0.1, flow: 1.0, erase: true });
        assert!(process(&flat, &Settings { masks: vec![erased], ..Default::default() }).get_pixel(15, 20).0[0] <= 101);
        let mut flipped = brush.clone();
        flipped.adj.exposure = 1.0;
        let fl = process(&flat, &Settings { masks: vec![flipped], flip: true, ..Default::default() });
        assert!(fl.get_pixel(45, 20).0[0] > 120, "mask follows the source when mirrored");

        let mut dot = RgbImage::from_pixel(80, 80, image::Rgb([120, 120, 120]));
        for y in 38..43 {
            for x in 38..43 {
                dot.put_pixel(x, y, image::Rgb([10, 10, 10]));
            }
        }
        let (sx, sy) = find_source(&dot, 0.5, 0.5, 0.06);
        assert!((sx - 0.5).abs() + (sy - 0.5).abs() > 0.08);
        for mode in [SpotMode::Heal, SpotMode::Clone, SpotMode::Remove] {
            let fixed = process(&dot, &Settings { spots: vec![Spot { x: 0.5, y: 0.5, sx, sy, r: 0.06, feather: 0.3, opacity: 1.0, mode }], ..Default::default() });
            assert!(fixed.get_pixel(40, 40).0[0] > 100, "{mode:?} {:?}", fixed.get_pixel(40, 40));
        }
        let mut eye = RgbImage::from_pixel(40, 40, image::Rgb([200, 160, 140]));
        for y in 17..23 {
            for x in 17..23 {
                eye.put_pixel(x, y, image::Rgb([220, 30, 40]));
            }
        }
        let e = process(&eye, &Settings { red_eyes: vec![RedEye { x: 0.5, y: 0.5, r: 0.1, ..Default::default() }], ..Default::default() }).get_pixel(20, 20).0;
        assert!(e[0] < 80, "{e:?}");

        let lut = curve_lut(&[[0.0, 0.0], [0.5, 0.7], [1.0, 1.0]]).unwrap();
        assert!(lut[128] > 0.65 && lut[0] < 0.01 && lut[255] > 0.99);
        assert!(curve_lut(&[[0.0, 0.0], [1.0, 1.0]]).is_none());
        let lifted = process(&flat, &Settings { curve_rgb: vec![[0.0, 0.0], [0.4, 0.6], [1.0, 1.0]], ..Default::default() });
        assert!(lifted.get_pixel(5, 5).0[0] > 120);
        let only_red = process(&flat, &Settings { curve_r: vec![[0.0, 0.2], [1.0, 1.0]], ..Default::default() }).get_pixel(5, 5).0;
        assert!(only_red[0] > only_red[1] + 10);
    }

    #[test]
    fn geometry_tools_and_pickers_are_consistent() {
        let mut img = RgbImage::from_pixel(200, 100, image::Rgb([230, 230, 230]));
        let tan = 5f32.to_radians().tan();
        for (x, y, p) in img.enumerate_pixels_mut() {
            let yc = 50.0 - (x as f32 + 0.5 - 100.0) * tan;
            let cover = (3.0 - (y as f32 + 0.5 - yc).abs()).clamp(0.0, 1.0);
            let v = (230.0 - 210.0 * cover) as u8;
            p.0 = [v, v, v];
        }
        let a = auto_straighten(&img);
        assert!((a - 5.0).abs() < 1.0, "{a}");
        let out = process(&img, &Settings { straighten: a, ..Default::default() });
        let darkest = |x: u32| (0..out.height()).min_by_key(|&y| out.get_pixel(x, y).0[0]).unwrap_or(0) as i32;
        assert!((darkest(40) - darkest(160)).abs() <= 2, "{} {}", darkest(40), darkest(160));

        let s = Settings { straighten: 7.0, crop: [0.1, 0.05, 0.2, 0.1], flip: true, rotate: 1, persp_v: 20.0, ..Default::default() };
        let v = View::new(&s, 300, 200);
        let (su, sv) = v.to_src(0.3, 0.6);
        let (u, vv) = v.from_src(su, sv);
        assert!((u - 0.3).abs() < 1e-3 && (vv - 0.6).abs() < 1e-3, "{u} {vv}");

        let cast = [0.55, 0.5, 0.45];
        let (t, ti) = wb_from_pixel(cast);
        let px = RgbImage::from_pixel(4, 4, image::Rgb(cast.map(|v| (v * 255.0) as u8)));
        let n = process(&px, &Settings { temperature: t, tint: ti, ..Default::default() }).get_pixel(1, 1).0;
        assert!(n[0].abs_diff(n[1]) <= 4 && n[1].abs_diff(n[2]) <= 4, "{n:?}");

        let mut tex = RgbImage::new(64, 64);
        for (x, y, p) in tex.enumerate_pixels_mut() {
            p.0 = if (x / 4 + y / 4) % 2 == 0 { [60, 60, 60] } else { [180, 180, 180] };
        }
        let warped = process(&tex, &Settings { persp_v: 60.0, persp_h: -30.0, persp_aspect: 20.0, persp_scale: 10.0, persp_x: 10.0, ca_red: 50.0, ..Default::default() });
        assert_eq!(warped.dimensions(), (64, 64));
        assert_ne!(warped, process(&tex, &Settings::default()));
        let vflip = process(&tex, &Settings { flip_v: true, ..Default::default() });
        assert_eq!(vflip.get_pixel(0, 0), tex.get_pixel(0, 63));
        for st in 0..3u8 {
            let vig = process(&RgbImage::from_pixel(40, 40, image::Rgb([150, 150, 150])), &Settings { vignette: -80.0, vignette_style: st, vignette_round: 40.0, vignette_feather: 20.0, ..Default::default() });
            assert!(vig.get_pixel(0, 0).0[0] < vig.get_pixel(20, 20).0[0], "style {st}");
        }
        for p in 1..PROFILES.len() as u8 {
            let c = RgbImage::from_pixel(8, 8, image::Rgb([180, 90, 60]));
            assert_ne!(process(&c, &Settings { profile: p, ..Default::default() }), process(&c, &Settings::default()), "profile {p}");
        }
        let col = RgbImage::from_fn(24, 24, |x, y| image::Rgb([(x * 10 + 15) as u8, ((x * 37 + y * 53) % 200 + 40) as u8, (y * 10 + 10) as u8]));
        for s in [
            Settings { cal_red_hue: 60.0, ..Default::default() },
            Settings { grade_global_hue: 220.0, grade_global_sat: 60.0, ..Default::default() },
            Settings { grade_mid_lum: 50.0, ..Default::default() },
            Settings { fade: 60.0, ..Default::default() },
            Settings { glow: 80.0, ..Default::default() },
            Settings { color_noise: 80.0, ..Default::default() },
        ] {
            assert_ne!(process(&col, &s), process(&col, &Settings::default()));
        }
    }
}
