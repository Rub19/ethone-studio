use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum Ease {
    Linear,
    #[default]
    Smooth,
    In,
    Out,
    Hold,
    Back,
    Elastic,
    Bounce,
    Strong,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum Ease2 {
    #[default]
    None,
    Expo,
    Circ,
    Anticipate,
    Steps,
    Sine,
    ExpoIn,
    ExpoInOut,
    SineIn,
    SineOut,
    CircIn,
    BackInOut,
    Quint,
    Steps8,
}

impl Ease {
    pub const ALL: [(Ease, &'static str); 9] = [
        (Ease::Linear, "Linéaire"),
        (Ease::Smooth, "Douce"),
        (Ease::Strong, "Douce forte"),
        (Ease::In, "Accélère"),
        (Ease::Out, "Ralentit"),
        (Ease::Hold, "Maintien"),
        (Ease::Back, "Dépassement"),
        (Ease::Elastic, "Élastique"),
        (Ease::Bounce, "Rebond"),
    ];
    pub fn apply(self, t: f32) -> f32 {
        match self {
            Ease::Linear => t,
            Ease::Smooth => t * t * (3.0 - 2.0 * t),
            Ease::Strong => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Ease::In => t * t * t,
            Ease::Out => 1.0 - (1.0 - t).powi(3),
            Ease::Hold => 0.0,
            Ease::Back => {
                let c = 1.70158;
                1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2)
            }
            Ease::Elastic => {
                if t <= 0.0 || t >= 1.0 {
                    t
                } else {
                    2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * TAU / 3.0).sin() + 1.0
                }
            }
            Ease::Bounce => {
                let (n, d) = (7.5625, 2.75);
                if t < 1.0 / d {
                    n * t * t
                } else if t < 2.0 / d {
                    let t = t - 1.5 / d;
                    n * t * t + 0.75
                } else if t < 2.5 / d {
                    let t = t - 2.25 / d;
                    n * t * t + 0.9375
                } else {
                    let t = t - 2.625 / d;
                    n * t * t + 0.984375
                }
            }
        }
    }
}

impl Ease2 {
    pub const ALL: [(Ease2, &'static str); 13] = [
        (Ease2::Expo, "Expo"),
        (Ease2::ExpoIn, "Expo entrée"),
        (Ease2::ExpoInOut, "Expo entrée-sortie"),
        (Ease2::Circ, "Circulaire"),
        (Ease2::CircIn, "Circulaire entrée"),
        (Ease2::Sine, "Sinus"),
        (Ease2::SineIn, "Sinus entrée"),
        (Ease2::SineOut, "Sinus sortie"),
        (Ease2::Quint, "Quintique"),
        (Ease2::Anticipate, "Anticipation"),
        (Ease2::BackInOut, "Dépassement double"),
        (Ease2::Steps, "Saccadé"),
        (Ease2::Steps8, "Saccadé fin"),
    ];
    fn apply(self, t: f32) -> Option<f32> {
        Some(match self {
            Ease2::None => return None,
            Ease2::Expo => {
                if t >= 1.0 {
                    1.0
                } else {
                    1.0 - 2f32.powf(-10.0 * t)
                }
            }
            Ease2::ExpoIn => {
                if t <= 0.0 {
                    0.0
                } else {
                    2f32.powf(10.0 * t - 10.0)
                }
            }
            Ease2::ExpoInOut => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else if t < 0.5 {
                    2f32.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2f32.powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            Ease2::Circ => (1.0 - (t - 1.0).powi(2)).max(0.0).sqrt(),
            Ease2::CircIn => 1.0 - (1.0 - t * t).max(0.0).sqrt(),
            Ease2::Anticipate => {
                let c = 1.70158;
                (c + 1.0) * t * t * t - c * t * t
            }
            Ease2::BackInOut => {
                let c = 1.70158 * 1.525;
                if t < 0.5 {
                    (2.0 * t).powi(2) * ((c + 1.0) * 2.0 * t - c) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c + 1.0) * (t * 2.0 - 2.0) + c) + 2.0) / 2.0
                }
            }
            Ease2::Steps => (t * 4.0).floor() / 4.0,
            Ease2::Steps8 => (t * 8.0).floor() / 8.0,
            Ease2::Sine => -((PI * t).cos() - 1.0) / 2.0,
            Ease2::SineIn => 1.0 - (t * PI / 2.0).cos(),
            Ease2::SineOut => (t * PI / 2.0).sin(),
            Ease2::Quint => 1.0 - (1.0 - t).powi(5),
        })
    }
}

pub const BEZ_PRESETS: [(&str, [f32; 4]); 14] = [
    ("Linéaire", [0.0, 0.0, 1.0, 1.0]),
    ("Standard", [0.25, 0.1, 0.25, 1.0]),
    ("Entrée douce", [0.42, 0.0, 1.0, 1.0]),
    ("Sortie douce", [0.0, 0.0, 0.58, 1.0]),
    ("Entrée-sortie", [0.42, 0.0, 0.58, 1.0]),
    ("Material", [0.4, 0.0, 0.2, 1.0]),
    ("Décélération", [0.0, 0.0, 0.2, 1.0]),
    ("Accélération", [0.4, 0.0, 1.0, 1.0]),
    ("Expo sortie", [0.16, 1.0, 0.3, 1.0]),
    ("Expo entrée", [0.7, 0.0, 0.84, 0.0]),
    ("Dépasse en sortie", [0.34, 1.56, 0.64, 1.0]),
    ("Recul en entrée", [0.36, 0.0, 0.66, -0.56]),
    ("Claquement", [0.9, 0.0, 0.1, 1.0]),
    ("Fluide Apple", [0.2, 0.8, 0.2, 1.0]),
];

pub fn bezier(b: [f32; 4], x: f32) -> f32 {
    let (x1, y1, x2, y2) = (b[0].clamp(0.0, 1.0), b[1], b[2].clamp(0.0, 1.0), b[3]);
    let f = |s: f32, p1: f32, p2: f32| 3.0 * (1.0 - s).powi(2) * s * p1 + 3.0 * (1.0 - s) * s * s * p2 + s * s * s;
    let x = x.clamp(0.0, 1.0);
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..32 {
        let mid = (lo + hi) / 2.0;
        if f(mid, x1, x2) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    f((lo + hi) / 2.0, y1, y2)
}

pub fn ease_to_bez(e: Ease) -> [f32; 4] {
    match e {
        Ease::Linear => [0.0, 0.0, 1.0, 1.0],
        Ease::In => [0.55, 0.0, 1.0, 0.45],
        Ease::Out => [0.0, 0.55, 0.45, 1.0],
        Ease::Strong => [0.65, 0.0, 0.35, 1.0],
        Ease::Back => [0.34, 1.56, 0.64, 1.0],
        _ => [0.42, 0.0, 0.58, 1.0],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Key {
    pub t: f32,
    pub v: f32,
    pub ease: Ease,
    #[serde(default)]
    pub ease2: Ease2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bez: Option<[f32; 4]>,
}

impl Key {
    pub fn curve(&self, u: f32) -> f32 {
        if let Some(b) = self.bez {
            return bezier(b, u);
        }
        self.ease2.apply(u).unwrap_or_else(|| self.ease.apply(u))
    }
    pub fn ease_name(&self) -> &'static str {
        if self.bez.is_some() {
            return "Bézier";
        }
        Ease2::ALL.iter().find(|(e, _)| *e == self.ease2).map(|x| x.1).unwrap_or_else(|| Ease::ALL.iter().find(|(e, _)| *e == self.ease).map_or("", |x| x.1))
    }
    pub fn set_ease(&mut self, e: Ease) {
        self.ease = e;
        self.ease2 = Ease2::None;
        self.bez = None;
    }
    pub fn set_ease2(&mut self, e: Ease2) {
        self.ease2 = e;
        self.bez = None;
    }
}

fn zero(v: &f32) -> bool {
    *v == 0.0
}

fn zero8(v: &u8) -> bool {
    *v == 0
}

pub const CYCLES: [&str; 5] = ["Aucune", "Cycle", "Aller-retour", "Décalage", "Continuer"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prop {
    pub base: f32,
    pub keys: Vec<Key>,
    #[serde(default, skip_serializing_if = "zero")]
    pub wiggle: f32,
    #[serde(default, skip_serializing_if = "zero")]
    pub wiggle_freq: f32,
    #[serde(default, skip_serializing_if = "zero8")]
    pub cycle: u8,
}

pub fn noise(x: f32, seed: f32) -> f32 {
    (x * TAU + seed).sin() * 0.6 + (x * TAU * 2.31 + seed * 3.1).sin() * 0.4
}

pub fn hash(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77).rotate_left(13);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h % 10_000) as f32 / 10_000.0
}

impl Prop {
    pub fn new(v: f32) -> Self {
        Self { base: v, keys: Vec::new(), wiggle: 0.0, wiggle_freq: 0.0, cycle: 0 }
    }

    pub fn at(&self, t: f32) -> f32 {
        let k = &self.keys;
        match k.len() {
            0 => self.base,
            _ if t <= k[0].t => k[0].v,
            _ if t >= k[k.len() - 1].t => k[k.len() - 1].v,
            _ => {
                let i = k.iter().rposition(|x| x.t <= t).unwrap();
                let (a, b) = (k[i], k[i + 1]);
                let u = a.curve((t - a.t) / (b.t - a.t).max(1e-6));
                a.v + (b.v - a.v) * u
            }
        }
    }

    pub fn eval(&self, t: f32, seed: usize) -> f32 {
        let k = &self.keys;
        let mut v = if k.len() >= 2 && self.cycle > 0 && t > k[k.len() - 1].t {
            let (a, b) = (k[0].t, k[k.len() - 1].t);
            let per = (b - a).max(1e-4);
            match self.cycle {
                1 => self.at(a + (t - a).rem_euclid(per)),
                2 => {
                    let u = (t - a).rem_euclid(per * 2.0);
                    self.at(a + if u > per { per * 2.0 - u } else { u })
                }
                3 => self.at(a + (t - a).rem_euclid(per)) + ((t - a) / per).floor() * (k[k.len() - 1].v - k[0].v),
                _ => {
                    let h = (per * 0.01).max(1e-3);
                    k[k.len() - 1].v + (self.at(b) - self.at(b - h)) / h * (t - b)
                }
            }
        } else {
            self.at(t)
        };
        if self.wiggle != 0.0 {
            v += self.wiggle * noise(t * self.wiggle_freq.max(0.05), seed as f32 * 1.91 + 0.7);
        }
        v
    }

    pub fn animated(&self) -> bool {
        !self.keys.is_empty()
    }

    pub fn expressive(&self) -> bool {
        self.wiggle != 0.0 || self.cycle > 0
    }

    pub fn key_at(&self, t: f32) -> Option<usize> {
        self.keys.iter().position(|k| (k.t - t).abs() < 1e-3)
    }

    pub fn set(&mut self, t: f32, v: f32) {
        if self.keys.is_empty() {
            self.base = v;
            return;
        }
        match self.key_at(t) {
            Some(i) => self.keys[i].v = v,
            None => self.add_key(t, v),
        }
    }

    pub fn add_key(&mut self, t: f32, v: f32) {
        let (ease, ease2, bez) = self.keys.first().map_or((Ease::Smooth, Ease2::None, None), |k| (k.ease, k.ease2, k.bez));
        self.keys.retain(|k| (k.t - t).abs() >= 1e-3);
        let i = self.keys.partition_point(|k| k.t < t);
        self.keys.insert(i, Key { t, v, ease, ease2, bez });
    }

    pub fn toggle_key(&mut self, t: f32) {
        match self.key_at(t) {
            Some(i) => {
                let v = self.keys[i].v;
                self.keys.remove(i);
                if self.keys.is_empty() {
                    self.base = v;
                }
            }
            None => self.add_key(t, self.at(t)),
        }
    }

    pub fn shift(&mut self, dt: f32) {
        for k in &mut self.keys {
            k.t += dt;
        }
    }

    pub fn reverse(&mut self, a: f32, b: f32) {
        let old = self.keys.clone();
        let n = old.len();
        self.keys = (0..n)
            .map(|j| {
                let src = old[n - 1 - j];
                let prev = if n >= 2 + j { old[n - 2 - j] } else { src };
                Key { t: a + b - src.t, v: src.v, ease: prev.ease, ease2: prev.ease2, bez: prev.bez.map(|b| [1.0 - b[2], 1.0 - b[3], 1.0 - b[0], 1.0 - b[1]]) }
            })
            .collect();
    }

    pub fn clear(&mut self, t: f32) {
        self.base = self.at(t);
        self.keys.clear();
    }

    pub fn spread(&mut self) {
        let n = self.keys.len();
        if n > 2 {
            let (a, b) = (self.keys[0].t, self.keys[n - 1].t);
            for (i, k) in self.keys.iter_mut().enumerate() {
                k.t = a + (b - a) * i as f32 / (n - 1) as f32;
            }
        }
    }

    pub fn stretch(&mut self, from: f32, k: f32) {
        for key in &mut self.keys {
            key.t = from + (key.t - from) * k;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Vtx {
    pub p: [f32; 2],
    #[serde(default)]
    pub i: [f32; 2],
    #[serde(default)]
    pub o: [f32; 2],
}

impl Vtx {
    pub fn at(x: f32, y: f32) -> Self {
        Self { p: [x, y], i: [0.0; 2], o: [0.0; 2] }
    }
    pub fn smooth(x: f32, y: f32, hx: f32, hy: f32) -> Self {
        Self { p: [x, y], i: [-hx, -hy], o: [hx, hy] }
    }
    pub fn corner(&self) -> bool {
        self.i == [0.0; 2] && self.o == [0.0; 2]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Content {
    Rect { w: f32, h: f32, r: f32, color: [u8; 4] },
    Ellipse { w: f32, h: f32, color: [u8; 4] },
    Text {
        text: String,
        size: f32,
        color: [u8; 4],
        bold: bool,
        family: String,
        #[serde(default)]
        italic: bool,
        #[serde(default)]
        tracking: f32,
        #[serde(default)]
        align: u8,
        #[serde(default)]
        vertical: bool,
    },
    Image { path: String, w: f32, h: f32 },
    Star { points: u32, radius: f32, inner: f32, color: [u8; 4] },
    Line { w: f32, thickness: f32, color: [u8; 4] },
    Counter { from: f32, to: f32, decimals: u32, size: f32, color: [u8; 4], prefix: String, suffix: String },
    Ring { radius: f32, thickness: f32, color: [u8; 4] },
    Arrow { w: f32, thickness: f32, color: [u8; 4] },
    Path { pts: Vec<Vtx>, closed: bool, color: [u8; 4], fill: bool },
    Strokes { strokes: Vec<Vec<[f32; 2]>>, width: f32, color: [u8; 4] },
    Null,
    Adjust { w: f32, h: f32 },
}

impl Content {
    pub fn text(text: &str, size: f32, color: [u8; 4], bold: bool) -> Self {
        Content::Text { text: text.into(), size, color, bold, family: "Segoe UI".into(), italic: false, tracking: 0.0, align: 0, vertical: false }
    }
    pub fn styled(mut self, family: &str, italic: bool, tracking: f32, align: u8) -> Self {
        if let Content::Text { family: f, italic: i, tracking: tr, align: a, .. } = &mut self {
            (*f, *i, *tr, *a) = (family.into(), italic, tracking, align);
        }
        self
    }
    pub fn color_mut(&mut self) -> Option<&mut [u8; 4]> {
        match self {
            Content::Rect { color, .. }
            | Content::Ellipse { color, .. }
            | Content::Text { color, .. }
            | Content::Star { color, .. }
            | Content::Line { color, .. }
            | Content::Counter { color, .. }
            | Content::Ring { color, .. }
            | Content::Arrow { color, .. }
            | Content::Path { color, .. }
            | Content::Strokes { color, .. } => Some(color),
            _ => None,
        }
    }
    pub fn color(&self) -> Option<[u8; 4]> {
        self.clone().color_mut().copied()
    }
    pub fn is_shape(&self) -> bool {
        matches!(self, Content::Rect { .. } | Content::Ellipse { .. } | Content::Star { .. } | Content::Line { .. } | Content::Ring { .. } | Content::Arrow { .. } | Content::Path { .. } | Content::Strokes { .. })
    }
}

pub const BLENDS: [(&str, &str); 16] = [
    ("normal", "Normal"),
    ("multiply", "Produit"),
    ("screen", "Superposition"),
    ("overlay", "Incrustation"),
    ("darken", "Obscurcir"),
    ("lighten", "Éclaircir"),
    ("difference", "Différence"),
    ("color-dodge", "Densité couleur −"),
    ("soft-light", "Lumière tamisée"),
    ("hard-light", "Lumière crue"),
    ("color-burn", "Densité couleur +"),
    ("exclusion", "Exclusion"),
    ("hue", "Teinte"),
    ("saturation", "Saturation"),
    ("color", "Couleur"),
    ("luminosity", "Luminosité"),
];

pub const MATTES: [&str; 5] = ["Aucun", "Alpha du calque du dessus", "Luminance du calque du dessus", "Alpha inversé", "Luminance inversée"];
pub const TEXT_ORDERS: [&str; 5] = ["Gauche → droite", "Droite → gauche", "Centre → bords", "Aléatoire", "Bords → centre"];
pub const CAPS: [&str; 3] = ["butt", "round", "square"];
pub const JOINS: [&str; 3] = ["round", "miter", "bevel"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextAnim {
    pub on: bool,
    pub mode: u8,
    pub opacity: f32,
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
    pub rot: f32,
    pub delay: f32,
    pub dur: f32,
    pub order: u8,
    pub ease: Ease,
}

impl Default for TextAnim {
    fn default() -> Self {
        Self { on: false, mode: 0, opacity: 0.0, dx: 0.0, dy: 30.0, scale: 100.0, rot: 0.0, delay: 0.04, dur: 0.4, order: 0, ease: Ease::Out }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fx {
    pub blur: f32,
    pub shadow: bool,
    pub stroke: Option<[u8; 4]>,
    pub stroke_w: f32,
    pub gradient: Option<[u8; 4]>,
    pub blend: u8,
    pub typewriter: f32,
    pub wiggle: f32,
    pub wiggle_freq: f32,
    pub loop_keys: bool,
    pub pingpong: bool,
    pub shadow_dx: f32,
    pub shadow_dy: f32,
    pub shadow_blur: f32,
    pub shadow_alpha: f32,
    pub shadow_color: [u8; 4],
    pub glow: f32,
    pub glow_color: [u8; 4],
    pub gradient_angle: f32,
    pub dash: f32,
    pub hue: f32,
    pub saturation: f32,
    pub brightness: f32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub matte: u8,
    pub echo: u32,
    pub echo_delay: f32,
    pub repeat: u32,
    pub rep_dx: f32,
    pub rep_dy: f32,
    pub rep_rot: f32,
    pub rep_scale: f32,
    pub rep_fade: f32,
    pub speed: f32,
    pub time_offset: f32,
    pub reverse_time: bool,
    pub freeze: bool,
    pub freeze_at: f32,
    pub auto_orient: bool,
    pub gradient_kind: u8,
    pub grad_cx: f32,
    pub grad_cy: f32,
    pub grad_r: f32,
    pub dash_gap: f32,
    pub cap: u8,
    pub join: u8,
    pub round: f32,
    pub pucker: f32,
    pub zig_amp: f32,
    pub zig_ridges: u32,
    pub zig_smooth: bool,
    pub rough: f32,
    pub rough_detail: f32,
    pub rough_speed: f32,
    pub twist: f32,
    pub trim_mode: u8,
    pub dir_blur: f32,
    pub dir_angle: f32,
    pub zoom_blur: f32,
    pub rgb_split: f32,
    pub rgb_angle: f32,
    pub glitch: f32,
    pub glitch_freq: f32,
    pub grain: f32,
    pub distort: f32,
    pub distort_scale: f32,
    pub distort_speed: f32,
    pub tint: f32,
    pub tint_dark: [u8; 4],
    pub tint_light: [u8; 4],
    pub invert: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub sepia: f32,
    pub threshold: f32,
    pub posterize: u32,
    pub fill_on: bool,
    pub fill_color: [u8; 4],
    pub inner: bool,
    pub inner_dx: f32,
    pub inner_dy: f32,
    pub inner_blur: f32,
    pub inner_color: [u8; 4],
    pub flicker: f32,
    pub ta: TextAnim,
}

impl Default for Fx {
    fn default() -> Self {
        Self {
            blur: 0.0,
            shadow: false,
            stroke: None,
            stroke_w: 0.0,
            gradient: None,
            blend: 0,
            typewriter: 0.0,
            wiggle: 0.0,
            wiggle_freq: 0.0,
            loop_keys: false,
            pingpong: false,
            shadow_dx: 0.0,
            shadow_dy: 10.0,
            shadow_blur: 14.0,
            shadow_alpha: 45.0,
            shadow_color: [0, 0, 0, 255],
            glow: 0.0,
            glow_color: [255, 214, 10, 255],
            gradient_angle: 0.0,
            dash: 0.0,
            hue: 0.0,
            saturation: 100.0,
            brightness: 100.0,
            flip_x: false,
            flip_y: false,
            matte: 0,
            echo: 0,
            echo_delay: 0.08,
            repeat: 0,
            rep_dx: 140.0,
            rep_dy: 0.0,
            rep_rot: 0.0,
            rep_scale: 100.0,
            rep_fade: 0.0,
            speed: 100.0,
            time_offset: 0.0,
            reverse_time: false,
            freeze: false,
            freeze_at: 0.0,
            auto_orient: false,
            gradient_kind: 0,
            grad_cx: 50.0,
            grad_cy: 50.0,
            grad_r: 70.0,
            dash_gap: 60.0,
            cap: 0,
            join: 0,
            round: 0.0,
            pucker: 0.0,
            zig_amp: 0.0,
            zig_ridges: 12,
            zig_smooth: false,
            rough: 0.0,
            rough_detail: 6.0,
            rough_speed: 0.0,
            twist: 0.0,
            trim_mode: 0,
            dir_blur: 0.0,
            dir_angle: 0.0,
            zoom_blur: 0.0,
            rgb_split: 0.0,
            rgb_angle: 0.0,
            glitch: 0.0,
            glitch_freq: 8.0,
            grain: 0.0,
            distort: 0.0,
            distort_scale: 60.0,
            distort_speed: 1.0,
            tint: 0.0,
            tint_dark: [20, 20, 60, 255],
            tint_light: [255, 220, 160, 255],
            invert: 0.0,
            contrast: 100.0,
            gamma: 1.0,
            sepia: 0.0,
            threshold: 0.0,
            posterize: 0,
            fill_on: false,
            fill_color: [255, 255, 255, 255],
            inner: false,
            inner_dx: 0.0,
            inner_dy: 6.0,
            inner_blur: 8.0,
            inner_color: [0, 0, 0, 180],
            flicker: 0.0,
            ta: TextAnim::default(),
        }
    }
}

pub const PROPS: [&str; 19] = [
    "Position X",
    "Position Y",
    "Échelle %",
    "Rotation °",
    "Opacité %",
    "Inclinaison °",
    "Flou px",
    "Révélation %",
    "Ancre X",
    "Ancre Y",
    "Échelle X %",
    "Échelle Y %",
    "Rotation X °",
    "Rotation Y °",
    "Début du tracé %",
    "Fin du tracé %",
    "Décalage du tracé %",
    "Épaisseur du contour",
    "Teinte °",
];
pub const PROP_DEFAULT: [f32; 19] = [0.0, 0.0, 100.0, 0.0, 100.0, 0.0, 0.0, 100.0, 0.0, 0.0, 100.0, 100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CKey {
    pub t: f32,
    pub c: [u8; 4],
}

pub fn color_at(keys: &[CKey], t: f32) -> Option<[u8; 4]> {
    let first = keys.first()?;
    let last = keys.last()?;
    if t <= first.t {
        return Some(first.c);
    }
    if t >= last.t {
        return Some(last.c);
    }
    let i = keys.iter().rposition(|k| k.t <= t)?;
    let (a, b) = (keys[i], keys[i + 1]);
    let u = ((t - a.t) / (b.t - a.t).max(1e-6)).clamp(0.0, 1.0);
    let u = u * u * (3.0 - 2.0 * u);
    Some(std::array::from_fn(|k| (a.c[k] as f32 + (b.c[k] as f32 - a.c[k] as f32) * u).round() as u8))
}

pub fn set_color_key(keys: &mut Vec<CKey>, t: f32, c: [u8; 4]) {
    match keys.iter_mut().find(|k| (k.t - t).abs() < 1e-3) {
        Some(k) => k.c = c,
        None => {
            let i = keys.partition_point(|k| k.t < t);
            keys.insert(i, CKey { t, c });
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub content: Content,
    pub props: Vec<Prop>,
    pub start: f32,
    pub end: f32,
    pub visible: bool,
    #[serde(default)]
    pub fx: Fx,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub label: Option<u8>,
    #[serde(default)]
    pub parent: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub color_keys: Vec<CKey>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stroke_keys: Vec<CKey>,
}

pub type Mat = [f32; 6];
pub const IDENT: Mat = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

pub fn mul(m: Mat, n: Mat) -> Mat {
    [m[0] * n[0] + m[2] * n[1], m[1] * n[0] + m[3] * n[1], m[0] * n[2] + m[2] * n[3], m[1] * n[2] + m[3] * n[3], m[0] * n[4] + m[2] * n[5] + m[4], m[1] * n[4] + m[3] * n[5] + m[5]]
}

pub fn apply(m: Mat, x: f32, y: f32) -> (f32, f32) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

pub fn invert(m: Mat) -> Mat {
    let det = m[0] * m[3] - m[1] * m[2];
    if det.abs() < 1e-9 {
        return IDENT;
    }
    let (a, b, c, d) = (m[3] / det, -m[1] / det, -m[2] / det, m[0] / det);
    [a, b, c, d, -(a * m[4] + c * m[5]), -(b * m[4] + d * m[5])]
}

struct Xf {
    x: f32,
    y: f32,
    r: f32,
    k: f32,
    sx: f32,
    sy: f32,
    ax: f32,
    ay: f32,
}

pub struct Outline {
    pub polys: Vec<(Vec<[f32; 2]>, bool)>,
    pub fill: Option<[u8; 4]>,
    pub line: Option<([u8; 4], f32)>,
}

impl Layer {
    pub fn new(name: String, content: Content, x: f32, y: f32, end: f32) -> Self {
        let mut props: Vec<Prop> = PROP_DEFAULT.iter().map(|v| Prop::new(*v)).collect();
        props[0].base = x;
        props[1].base = y;
        Self { name, content, props, start: 0.0, end, visible: true, fx: Fx::default(), locked: false, solo: false, label: None, parent: None, color_keys: Vec::new(), stroke_keys: Vec::new() }
    }

    pub fn default_of(&self, pi: usize) -> f32 {
        if pi == 17 { self.fx.stroke_w } else { PROP_DEFAULT.get(pi).copied().unwrap_or(0.0) }
    }

    pub fn normalize(&mut self) {
        while self.props.len() < PROPS.len() {
            let v = self.default_of(self.props.len());
            self.props.push(Prop::new(v));
        }
    }

    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let mm = |pts: &mut dyn Iterator<Item = [f32; 2]>, pad: f32| {
            let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for p in pts {
                b = (b.0.min(p[0]), b.1.min(p[1]), b.2.max(p[0]), b.3.max(p[1]));
            }
            if b.0 > b.2 { (-20.0, -20.0, 20.0, 20.0) } else { (b.0 - pad, b.1 - pad, b.2 + pad, b.3 + pad) }
        };
        match &self.content {
            Content::Path { pts, closed, .. } => mm(&mut path_points(pts, *closed).into_iter(), 0.0),
            Content::Strokes { strokes, width, .. } => mm(&mut strokes.iter().flatten().copied(), width / 2.0),
            Content::Null => (-40.0, -40.0, 40.0, 40.0),
            Content::Text { align, vertical: false, .. } => {
                let (w, h) = self.size();
                match align {
                    1 => (0.0, -h / 2.0, w, h / 2.0),
                    2 => (-w, -h / 2.0, 0.0, h / 2.0),
                    _ => (-w / 2.0, -h / 2.0, w / 2.0, h / 2.0),
                }
            }
            _ => {
                let (w, h) = self.size();
                (-w / 2.0, -h / 2.0, w / 2.0, h / 2.0)
            }
        }
    }

    pub fn size(&self) -> (f32, f32) {
        match &self.content {
            Content::Rect { w, h, .. } | Content::Ellipse { w, h, .. } | Content::Image { w, h, .. } | Content::Adjust { w, h } => (*w, *h),
            Content::Text { text, size, tracking, vertical, .. } => {
                let cols = text.lines().map(|l| l.chars().count()).max().unwrap_or(1) as f32;
                let rows = text.lines().count().max(1) as f32;
                if *vertical { (rows * size * 1.2, cols * size * 1.05) } else { (cols * (size * 0.55 + tracking), rows * size * 1.2) }
            }
            Content::Star { radius, .. } => (radius * 2.0, radius * 2.0),
            Content::Ring { radius, thickness, .. } => (radius * 2.0 + thickness, radius * 2.0 + thickness),
            Content::Line { w, thickness, .. } => (*w, thickness.max(8.0)),
            Content::Arrow { w, thickness, .. } => (*w, thickness * 3.0),
            Content::Counter { to, from, decimals, size, prefix, suffix, .. } => {
                let n = format!("{prefix}{:.*}{suffix}", *decimals as usize, from.abs().max(to.abs())).chars().count() as f32;
                (n * size * 0.6, size * 1.2)
            }
            Content::Path { .. } | Content::Strokes { .. } | Content::Null => {
                let b = self.bounds();
                (b.2 - b.0, b.3 - b.1)
            }
        }
    }

    pub fn active(&self, t: f32) -> bool {
        self.visible && t >= self.start && t <= self.end
    }

    pub fn local_time(&self, t: f32) -> f32 {
        let fx = &self.fx;
        let t = if fx.freeze {
            fx.freeze_at
        } else {
            let r = if fx.reverse_time { self.start + self.end - t } else { t };
            self.start + (r - self.start) * fx.speed / 100.0 + fx.time_offset
        };
        if !fx.pingpong && !fx.loop_keys {
            return t;
        }
        let last = self.props.iter().flat_map(|p| p.keys.last()).map(|k| k.t).fold(0.0f32, f32::max);
        let first = self.props.iter().flat_map(|p| p.keys.first()).map(|k| k.t).fold(f32::MAX, f32::min).min(last);
        let period = last - first;
        if period <= 1e-3 || t <= first {
            return t;
        }
        if fx.pingpong {
            let u = (t - first).rem_euclid(period * 2.0);
            first + if u > period { period * 2.0 - u } else { u }
        } else {
            first + (t - first).rem_euclid(period)
        }
    }

    pub fn value(&self, pi: usize, t: f32) -> f32 {
        let tt = self.local_time(t);
        self.props.get(pi).map_or_else(|| self.default_of(pi), |p| p.eval(tt, pi))
    }

    pub fn fill_color(&self, t: f32) -> Option<[u8; 4]> {
        color_at(&self.color_keys, self.local_time(t))
    }

    pub fn wiggle(&self, t: f32, seed: usize) -> (f32, f32) {
        if self.fx.wiggle <= 0.0 {
            return (0.0, 0.0);
        }
        let f = self.fx.wiggle_freq.max(0.1) * TAU;
        let ph = seed as f32 * 1.37;
        (
            self.fx.wiggle * ((t * f + ph).sin() * 0.6 + (t * f * 2.31 + ph * 3.1).sin() * 0.4),
            self.fx.wiggle * ((t * f * 1.13 + ph * 2.3).cos() * 0.6 + (t * f * 1.87 + ph).sin() * 0.4),
        )
    }

    pub fn orient(&self, t: f32) -> f32 {
        for h in [1.0 / 60.0, 0.1, 0.5] {
            let dx = self.value(0, t + h) - self.value(0, t - h);
            let dy = self.value(1, t + h) - self.value(1, t - h);
            if dx.hypot(dy) > 1e-3 {
                return dy.atan2(dx).to_degrees();
            }
        }
        0.0
    }

    fn xf(&self, t: f32, seed: usize) -> Xf {
        let (wx, wy) = self.wiggle(t, seed);
        let s = self.value(2, t) / 100.0;
        let mut r = self.value(3, t);
        if self.fx.auto_orient {
            r += self.orient(t);
        }
        Xf {
            x: self.value(0, t) + wx,
            y: self.value(1, t) + wy,
            r,
            k: self.value(5, t),
            sx: s * self.value(10, t) / 100.0 * self.value(13, t).to_radians().cos(),
            sy: s * self.value(11, t) / 100.0 * self.value(12, t).to_radians().cos(),
            ax: self.value(8, t),
            ay: self.value(9, t),
        }
    }

    pub fn own_matrix(&self, t: f32, seed: usize) -> Mat {
        let x = self.xf(t, seed);
        let (s, c) = x.r.to_radians().sin_cos();
        let m = mul([1.0, 0.0, 0.0, 1.0, x.x, x.y], [c, s, -s, c, 0.0, 0.0]);
        let m = mul(m, [1.0, 0.0, x.k.to_radians().tan(), 1.0, 0.0, 0.0]);
        let m = mul(m, [x.sx, 0.0, 0.0, x.sy, 0.0, 0.0]);
        mul(m, [1.0, 0.0, 0.0, 1.0, -x.ax, -x.ay])
    }

    fn own_xform(&self, t: f32, seed: usize) -> String {
        let x = self.xf(t, seed);
        let mut s = format!("translate({} {}) rotate({})", x.x, x.y, x.r);
        if x.k != 0.0 {
            write!(s, " skewX({})", x.k).unwrap();
        }
        write!(s, " scale({} {})", x.sx, x.sy).unwrap();
        if x.ax != 0.0 || x.ay != 0.0 {
            write!(s, " translate({} {})", -x.ax, -x.ay).unwrap();
        }
        s
    }

    pub fn key_times(&self) -> Vec<f32> {
        let mut times: Vec<f32> = self.props.iter().flat_map(|p| p.keys.iter().map(|k| k.t)).chain(self.color_keys.iter().chain(&self.stroke_keys).map(|k| k.t)).collect();
        times.sort_by(f32::total_cmp);
        times.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
        times
    }

    pub fn shift(&mut self, dt: f32) {
        self.start += dt;
        self.end += dt;
        for p in &mut self.props {
            p.shift(dt);
        }
        for k in self.color_keys.iter_mut().chain(self.stroke_keys.iter_mut()) {
            k.t += dt;
        }
    }

    pub fn has_effects(&self) -> bool {
        let f = &self.fx;
        let d = Fx::default();
        f.blur > 0.0 || f.shadow || f.glow > 0.0 || f.hue != 0.0 || f.saturation != d.saturation || f.brightness != d.brightness || f.tint > 0.0 || f.invert > 0.0 || f.contrast != d.contrast || f.gamma != d.gamma || f.sepia > 0.0 || f.threshold > 0.0 || f.posterize > 1 || f.fill_on || f.inner || f.rgb_split > 0.0 || f.grain > 0.0 || f.distort > 0.0 || f.dir_blur > 0.0 || f.zoom_blur > 0.0 || f.glitch > 0.0
    }

    pub fn outline(&self) -> Option<Outline> {
        let fill = self.content.color();
        let (polys, fill, line) = match &self.content {
            Content::Rect { w, h, r, .. } => (vec![(rect_points(*w, *h, *r), true)], fill, None),
            Content::Ellipse { w, h, .. } => (vec![(ellipse_points(w / 2.0, h / 2.0), true)], fill, None),
            Content::Star { points, radius, inner, .. } => (vec![(star_points(*points, *radius, *inner), true)], fill, None),
            Content::Arrow { w, thickness, .. } => (vec![(arrow_points(*w, *thickness), true)], fill, None),
            Content::Line { w, thickness, color } => (vec![(vec![[-w / 2.0 + thickness / 2.0, 0.0], [w / 2.0 - thickness / 2.0, 0.0]], false)], None, Some((*color, *thickness))),
            Content::Ring { radius, thickness, color } => (vec![(ellipse_points(*radius, *radius), true)], None, Some((*color, *thickness))),
            Content::Path { pts, closed, fill: f, .. } => (vec![(path_points(pts, *closed), *closed)], fill.filter(|_| *f), None),
            Content::Strokes { strokes, width, color } => (strokes.iter().map(|s| (s.clone(), false)).collect(), None, Some((*color, *width))),
            _ => return None,
        };
        Some(Outline { polys, fill, line })
    }
}

pub fn rect_points(w: f32, h: f32, r: f32) -> Vec<[f32; 2]> {
    let (hw, hh) = (w / 2.0, h / 2.0);
    let r = r.min(hw).min(hh).max(0.0);
    if r < 0.5 {
        return vec![[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]];
    }
    let mut out = Vec::new();
    for (cx, cy, a0) in [(hw - r, -hh + r, -90.0f32), (hw - r, hh - r, 0.0), (-hw + r, hh - r, 90.0), (-hw + r, -hh + r, 180.0)] {
        for k in 0..=8 {
            let a = (a0 + k as f32 * 90.0 / 8.0).to_radians();
            out.push([cx + a.cos() * r, cy + a.sin() * r]);
        }
    }
    out
}

pub fn ellipse_points(rx: f32, ry: f32) -> Vec<[f32; 2]> {
    (0..96).map(|k| {
        let a = -PI / 2.0 + k as f32 * TAU / 96.0;
        [a.cos() * rx, a.sin() * ry]
    }).collect()
}

pub fn star_points(points: u32, radius: f32, inner: f32) -> Vec<[f32; 2]> {
    let n = points.max(3);
    let star = inner < 0.99;
    let count = if star { n * 2 } else { n };
    (0..count)
        .map(|k| {
            let a = -PI / 2.0 + k as f32 * TAU / count as f32;
            let r = if star && k % 2 == 1 { radius * inner } else { radius };
            [a.cos() * r, a.sin() * r]
        })
        .collect()
}

pub fn arrow_points(w: f32, th: f32) -> Vec<[f32; 2]> {
    let (hw, head) = (w / 2.0, (th * 2.4).min(w * 0.6));
    vec![[-hw, -th / 2.0], [hw - head, -th / 2.0], [hw - head, -th * 1.5], [hw, 0.0], [hw - head, th * 1.5], [hw - head, th / 2.0], [-hw, th / 2.0]]
}

fn cubic(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2], u: f32) -> [f32; 2] {
    let v = 1.0 - u;
    std::array::from_fn(|k| v * v * v * a[k] + 3.0 * v * v * u * b[k] + 3.0 * v * u * u * c[k] + u * u * u * d[k])
}

fn seg_handles(a: &Vtx, b: &Vtx) -> ([f32; 2], [f32; 2]) {
    ([a.p[0] + a.o[0], a.p[1] + a.o[1]], [b.p[0] + b.i[0], b.p[1] + b.i[1]])
}

pub fn path_points(pts: &[Vtx], closed: bool) -> Vec<[f32; 2]> {
    let n = pts.len();
    if n == 0 {
        return Vec::new();
    }
    let mut out = vec![pts[0].p];
    let segs = if closed && n > 2 { n } else { n - 1 };
    for s in 0..segs {
        let (a, b) = (&pts[s], &pts[(s + 1) % n]);
        if a.o == [0.0; 2] && b.i == [0.0; 2] {
            out.push(b.p);
        } else {
            let (c1, c2) = seg_handles(a, b);
            for k in 1..=32 {
                out.push(cubic(a.p, c1, c2, b.p, k as f32 / 32.0));
            }
        }
    }
    if closed && n > 2 && out.len() > 1 {
        out.pop();
    }
    out
}

pub fn path_d(pts: &[Vtx], closed: bool) -> String {
    let n = pts.len();
    let mut d = String::new();
    if n == 0 {
        return d;
    }
    write!(d, "M {:.2} {:.2}", pts[0].p[0], pts[0].p[1]).unwrap();
    let segs = if closed && n > 2 { n } else { n - 1 };
    for s in 0..segs {
        let (a, b) = (&pts[s], &pts[(s + 1) % n]);
        if a.o == [0.0; 2] && b.i == [0.0; 2] {
            write!(d, " L {:.2} {:.2}", b.p[0], b.p[1]).unwrap();
        } else {
            let (c1, c2) = seg_handles(a, b);
            write!(d, " C {:.2} {:.2} {:.2} {:.2} {:.2} {:.2}", c1[0], c1[1], c2[0], c2[1], b.p[0], b.p[1]).unwrap();
        }
    }
    if closed && n > 2 {
        d.push_str(" Z");
    }
    d
}

pub fn split_segment(pts: &mut Vec<Vtx>, seg: usize, u: f32) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let (a, b) = (pts[seg], pts[(seg + 1) % n]);
    let (c1, c2) = seg_handles(&a, &b);
    let lerp = |p: [f32; 2], q: [f32; 2]| [p[0] + (q[0] - p[0]) * u, p[1] + (q[1] - p[1]) * u];
    let (p01, p12, p23) = (lerp(a.p, c1), lerp(c1, c2), lerp(c2, b.p));
    let (p012, p123) = (lerp(p01, p12), lerp(p12, p23));
    let m = lerp(p012, p123);
    let curve = !(a.o == [0.0; 2] && b.i == [0.0; 2]);
    let mut v = Vtx::at(m[0], m[1]);
    if curve {
        pts[seg].o = [p01[0] - a.p[0], p01[1] - a.p[1]];
        let nb = (seg + 1) % n;
        pts[nb].i = [p23[0] - b.p[0], p23[1] - b.p[1]];
        v.i = [p012[0] - m[0], p012[1] - m[1]];
        v.o = [p123[0] - m[0], p123[1] - m[1]];
    }
    pts.insert(seg + 1, v);
}

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn closed_pts(poly: &[[f32; 2]], closed: bool) -> Vec<[f32; 2]> {
    let mut p = poly.to_vec();
    if closed && poly.len() > 1 {
        p.push(poly[0]);
    }
    p
}

fn cumulative(p: &[[f32; 2]]) -> Vec<f32> {
    let mut c = vec![0.0];
    for w in p.windows(2) {
        c.push(c.last().unwrap() + dist(w[0], w[1]));
    }
    c
}

fn point_at(p: &[[f32; 2]], cum: &[f32], d: f32) -> [f32; 2] {
    let i = cum.partition_point(|c| *c < d).clamp(1, p.len().max(2) - 1);
    let (a, b) = (p[i - 1], p[i]);
    let seg = (cum[i] - cum[i - 1]).max(1e-6);
    let u = ((d - cum[i - 1]) / seg).clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u]
}

fn piece(p: &[[f32; 2]], cum: &[f32], a: f32, b: f32) -> Vec<[f32; 2]> {
    if p.len() < 2 {
        return p.to_vec();
    }
    let mut out = vec![point_at(p, cum, a)];
    for (k, c) in cum.iter().enumerate() {
        if *c > a && *c < b {
            out.push(p[k]);
        }
    }
    out.push(point_at(p, cum, b));
    out
}

fn resample(poly: &[[f32; 2]], closed: bool, n: usize) -> Vec<[f32; 2]> {
    let p = closed_pts(poly, closed);
    if p.len() < 2 || n < 2 {
        return poly.to_vec();
    }
    let cum = cumulative(&p);
    let total = *cum.last().unwrap();
    let count = if closed { n } else { n + 1 };
    (0..count).map(|k| point_at(&p, &cum, total * k as f32 / n as f32)).collect()
}

fn normals(p: &[[f32; 2]], closed: bool) -> Vec<[f32; 2]> {
    let n = p.len();
    (0..n)
        .map(|k| {
            let prev = if k == 0 { if closed { p[n - 1] } else { p[0] } } else { p[k - 1] };
            let next = if k + 1 == n { if closed { p[0] } else { p[n - 1] } } else { p[k + 1] };
            let (tx, ty) = (next[0] - prev[0], next[1] - prev[1]);
            let l = tx.hypot(ty).max(1e-6);
            [-ty / l, tx / l]
        })
        .collect()
}

fn centroid(p: &[[f32; 2]]) -> [f32; 2] {
    let n = p.len().max(1) as f32;
    [p.iter().map(|q| q[0]).sum::<f32>() / n, p.iter().map(|q| q[1]).sum::<f32>() / n]
}

fn poly_len(p: &[[f32; 2]], closed: bool) -> f32 {
    *cumulative(&closed_pts(p, closed)).last().unwrap_or(&0.0)
}

pub fn shape_ops(fx: &Fx, polys: Vec<(Vec<[f32; 2]>, bool)>, t: f32) -> Vec<(Vec<[f32; 2]>, bool)> {
    polys
        .into_iter()
        .map(|(mut p, closed)| {
            if p.len() < 2 {
                return (p, closed);
            }
            if fx.round > 0.0 && p.len() >= 3 {
                let n = p.len();
                let mut out = Vec::new();
                for k in 0..n {
                    let v = p[k];
                    if !closed && (k == 0 || k == n - 1) {
                        out.push(v);
                        continue;
                    }
                    let (a, b) = (p[(k + n - 1) % n], p[(k + 1) % n]);
                    let (d1, d2) = (dist(a, v), dist(b, v));
                    let (r1, r2) = (fx.round.min(d1 / 2.0), fx.round.min(d2 / 2.0));
                    let s = [v[0] + (a[0] - v[0]) / d1.max(1e-6) * r1, v[1] + (a[1] - v[1]) / d1.max(1e-6) * r1];
                    let e = [v[0] + (b[0] - v[0]) / d2.max(1e-6) * r2, v[1] + (b[1] - v[1]) / d2.max(1e-6) * r2];
                    for j in 0..=6 {
                        let u = j as f32 / 6.0;
                        let w = 1.0 - u;
                        out.push([w * w * s[0] + 2.0 * w * u * v[0] + u * u * e[0], w * w * s[1] + 2.0 * w * u * v[1] + u * u * e[1]]);
                    }
                }
                p = out;
            }
            if fx.pucker != 0.0 {
                let c = centroid(&p);
                let n = p.len();
                let segs = if closed { n } else { n - 1 };
                let mut out = Vec::new();
                for s in 0..segs {
                    let (a, b) = (p[s], p[(s + 1) % n]);
                    for j in 0..8 {
                        let u = j as f32 / 8.0;
                        let q = [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
                        let f = fx.pucker / 100.0 * (PI * u).sin();
                        out.push([q[0] + (c[0] - q[0]) * f, q[1] + (c[1] - q[1]) * f]);
                    }
                }
                if !closed {
                    out.push(p[n - 1]);
                }
                p = out;
            }
            if fx.zig_amp != 0.0 {
                let ridges = fx.zig_ridges.max(1) as usize;
                let n = if fx.zig_smooth { ridges * 8 } else { ridges * 2 };
                let q = resample(&p, closed, n);
                let nm = normals(&q, closed);
                p = q
                    .iter()
                    .zip(&nm)
                    .enumerate()
                    .map(|(k, (v, m))| {
                        let off = if fx.zig_smooth { (TAU * ridges as f32 * k as f32 / n as f32).sin() } else if k % 2 == 0 { 1.0 } else { -1.0 };
                        let off = if !closed && (k == 0 || k == q.len() - 1) { 0.0 } else { off * fx.zig_amp };
                        [v[0] + m[0] * off, v[1] + m[1] * off]
                    })
                    .collect();
            }
            if fx.rough > 0.0 {
                let n = ((poly_len(&p, closed) * fx.rough_detail.max(0.5) / 100.0).ceil() as usize).clamp(8, 4000);
                let q = resample(&p, closed, n);
                let nm = normals(&q, closed);
                p = q
                    .iter()
                    .zip(&nm)
                    .enumerate()
                    .map(|(k, (v, m))| {
                        let off = fx.rough * noise(t * fx.rough_speed * 0.5 + k as f32 * 0.37, k as f32 * 2.17);
                        [v[0] + m[0] * off, v[1] + m[1] * off]
                    })
                    .collect();
            }
            if fx.twist != 0.0 {
                let c = centroid(&p);
                let maxd = p.iter().map(|q| dist(*q, c)).fold(1e-6f32, f32::max);
                p = p
                    .iter()
                    .map(|q| {
                        let a = (fx.twist * dist(*q, c) / maxd).to_radians();
                        let (s, co) = a.sin_cos();
                        let (dx, dy) = (q[0] - c[0], q[1] - c[1]);
                        [c[0] + dx * co - dy * s, c[1] + dx * s + dy * co]
                    })
                    .collect();
            }
            (p, closed)
        })
        .collect()
}

pub fn trim(polys: Vec<(Vec<[f32; 2]>, bool)>, s: f32, e: f32, o: f32, sequential: bool) -> Vec<(Vec<[f32; 2]>, bool)> {
    let (s, e) = (s.clamp(0.0, 100.0).min(e.clamp(0.0, 100.0)), e.clamp(0.0, 100.0).max(s.clamp(0.0, 100.0)));
    if e - s >= 99.99 {
        return polys;
    }
    if e - s <= 0.01 {
        return Vec::new();
    }
    let (fa, fb) = ((s + o) / 100.0, (e + o) / 100.0);
    let mut out = Vec::new();
    if !sequential {
        for (poly, closed) in polys {
            let p = closed_pts(&poly, closed);
            let cum = cumulative(&p);
            let l = *cum.last().unwrap_or(&0.0);
            if l <= 1e-6 {
                continue;
            }
            let (mut a, mut b) = (fa * l, fb * l);
            let k = (a / l).floor();
            a -= k * l;
            b -= k * l;
            if b <= l + 1e-4 {
                out.push((piece(&p, &cum, a, b.min(l)), false));
            } else if closed {
                let mut first = piece(&p, &cum, a, l);
                first.extend(piece(&p, &cum, 0.0, b - l).into_iter().skip(1));
                out.push((first, false));
            } else {
                out.push((piece(&p, &cum, a, l), false));
                out.push((piece(&p, &cum, 0.0, b - l), false));
            }
        }
        return out;
    }
    let data: Vec<(Vec<[f32; 2]>, Vec<f32>)> = polys
        .into_iter()
        .map(|(poly, closed)| {
            let p = closed_pts(&poly, closed);
            let c = cumulative(&p);
            (p, c)
        })
        .collect();
    let total: f32 = data.iter().map(|(_, c)| *c.last().unwrap_or(&0.0)).sum();
    if total <= 1e-6 {
        return out;
    }
    let (mut a, mut b) = (fa * total, fb * total);
    let k = (a / total).floor();
    a -= k * total;
    b -= k * total;
    let mut ranges = vec![(a, b.min(total))];
    if b > total {
        ranges.push((0.0, b - total));
    }
    let mut off = 0.0;
    for (p, cum) in &data {
        let l = *cum.last().unwrap_or(&0.0);
        for (ra, rb) in &ranges {
            let (x, y) = (ra.max(off), rb.min(off + l));
            if y > x {
                out.push((piece(p, cum, x - off, y - off), false));
            }
        }
        off += l;
    }
    out
}

pub fn polys_d(polys: &[(Vec<[f32; 2]>, bool)]) -> String {
    let mut d = String::new();
    for (p, closed) in polys {
        for (k, q) in p.iter().enumerate() {
            write!(d, "{}{:.2} {:.2} ", if k == 0 { "M " } else { "L " }, q[0], q[1]).unwrap();
        }
        if *closed && p.len() > 2 {
            d.push_str("Z ");
        }
    }
    d
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub t: f32,
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub vertical: bool,
    pub pos: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comp {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration: f32,
    pub background: [u8; 4],
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub background2: Option<[u8; 4]>,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub work: Option<(f32, f32)>,
    #[serde(default, skip_serializing_if = "zero")]
    pub vignette: f32,
    #[serde(default, skip_serializing_if = "zero")]
    pub grain: f32,
    #[serde(default, skip_serializing_if = "zero")]
    pub letterbox: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guides: Vec<Guide>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<studio_fx::stack::Fx>,
}

fn color(c: [u8; 4]) -> String {
    format!(r##"fill="#{:02x}{:02x}{:02x}" fill-opacity="{}""##, c[0], c[1], c[2], c[3] as f32 / 255.0)
}

fn hex(c: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn rotate(x: f32, y: f32, deg: f32) -> (f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    (x * c - y * s, x * s + y * c)
}

fn ranks(n: usize, order: u8) -> Vec<f32> {
    let mid = (n as f32 - 1.0) / 2.0;
    match order {
        1 => (0..n).map(|i| (n - 1 - i) as f32).collect(),
        2 => (0..n).map(|i| (i as f32 - mid).abs()).collect(),
        3 => {
            let mut idx: Vec<usize> = (0..n).collect();
            idx.sort_by(|a, b| hash(*a as u32, 7).total_cmp(&hash(*b as u32, 7)));
            let mut r = vec![0.0; n];
            for (k, i) in idx.into_iter().enumerate() {
                r[i] = k as f32;
            }
            r
        }
        4 => (0..n).map(|i| mid - (i as f32 - mid).abs()).collect(),
        _ => (0..n).map(|i| i as f32).collect(),
    }
}

impl Comp {
    pub fn new() -> Self {
        Self { width: 1920, height: 1080, fps: 30, duration: 5.0, background: [16, 18, 26, 255], layers: Vec::new(), background2: None, markers: Vec::new(), work: None, vignette: 0.0, grain: 0.0, letterbox: 0.0, guides: Vec::new(), effects: Vec::new() }
    }

    pub fn normalize(&mut self) {
        let n = self.layers.len();
        for (i, l) in self.layers.iter_mut().enumerate() {
            l.normalize();
            if l.parent.is_some_and(|p| p >= n || p == i) {
                l.parent = None;
            }
        }
    }

    pub fn frames(&self) -> u32 {
        (self.duration * self.fps as f32).round().max(1.0) as u32
    }

    pub fn range(&self) -> (f32, f32) {
        self.work.map(|(a, b)| (a.min(b).max(0.0), a.max(b).min(self.duration))).filter(|(a, b)| b - a > 1e-3).unwrap_or((0.0, self.duration))
    }

    pub fn world(&self, li: usize, t: f32) -> (f32, f32, f32, f32) {
        self.world_depth(li, t, 0)
    }

    fn parent_of(&self, li: usize, depth: u32) -> Option<usize> {
        self.layers[li].parent.filter(|p| *p < self.layers.len() && *p != li && depth < 8)
    }

    fn world_depth(&self, li: usize, t: f32, depth: u32) -> (f32, f32, f32, f32) {
        let l = &self.layers[li];
        let (wx, wy) = l.wiggle(t, li);
        let (x, y, s) = (l.value(0, t) + wx, l.value(1, t) + wy, l.value(2, t) / 100.0);
        let r = l.value(3, t) + if l.fx.auto_orient { l.orient(t) } else { 0.0 };
        match self.parent_of(li, depth) {
            Some(p) => {
                let (_, _, ps, pr) = self.world_depth(p, t, depth + 1);
                let (px, py) = apply(self.matrix_depth(p, t, depth + 1), x, y);
                (px, py, ps * s, pr + r)
            }
            None => (x, y, s, r),
        }
    }

    fn matrix_depth(&self, li: usize, t: f32, depth: u32) -> Mat {
        let own = self.layers[li].own_matrix(t, li);
        match self.parent_of(li, depth) {
            Some(p) => mul(self.matrix_depth(p, t, depth + 1), own),
            None => own,
        }
    }

    pub fn matrix(&self, li: usize, t: f32) -> Mat {
        self.matrix_depth(li, t, 0)
    }

    pub fn parent_matrix(&self, li: usize, t: f32) -> Mat {
        match self.parent_of(li, 0) {
            Some(p) => self.matrix_depth(p, t, 1),
            None => IDENT,
        }
    }

    pub fn chain_has(&self, from: usize, target: usize) -> bool {
        let mut cur = Some(from);
        let mut n = 0;
        while let Some(c) = cur {
            if c == target {
                return true;
            }
            n += 1;
            if n > 16 {
                return true;
            }
            cur = self.layers.get(c).and_then(|l| l.parent);
        }
        false
    }

    pub fn set_parent(&mut self, li: usize, parent: Option<usize>, t: f32) -> bool {
        if let Some(p) = parent
            && (p >= self.layers.len() || self.chain_has(p, li))
        {
            return false;
        }
        let (wx, wy, ws, wr) = self.world(li, t);
        let (wwx, wwy) = self.layers[li].wiggle(t, li);
        let (x, y, s, r) = match parent {
            Some(p) => {
                let (_, _, ps, pr) = self.world(p, t);
                let (lx, ly) = apply(invert(self.matrix(p, t)), wx, wy);
                (lx, ly, ws / ps.max(1e-4), wr - pr)
            }
            None => (wx, wy, ws, wr),
        };
        let orient = if self.layers[li].fx.auto_orient { self.layers[li].orient(t) } else { 0.0 };
        let l = &mut self.layers[li];
        l.normalize();
        l.parent = parent;
        l.props[0].set(t, x - wwx);
        l.props[1].set(t, y - wwy);
        l.props[2].set(t, s * 100.0);
        l.props[3].set(t, r - orient);
        true
    }

    pub fn next_key(&self, t: f32, forward: bool, only: Option<usize>) -> Option<f32> {
        let mut times: Vec<f32> = self.layers.iter().enumerate().filter(|(i, _)| only.is_none_or(|o| o == *i)).flat_map(|(_, l)| l.key_times()).chain(self.markers.iter().map(|m| m.t)).collect();
        times.sort_by(f32::total_cmp);
        if forward { times.into_iter().find(|k| *k > t + 1e-3) } else { times.into_iter().rev().find(|k| *k < t - 1e-3) }
    }

    fn xform(&self, li: usize, t: f32) -> String {
        let mut chain = vec![self.layers[li].own_xform(t, li)];
        let mut cur = self.layers[li].parent;
        let mut seen = 0;
        while let Some(p) = cur.filter(|p| *p < self.layers.len() && *p != li && seen < 8) {
            chain.push(self.layers[p].own_xform(t, p));
            cur = self.layers[p].parent;
            seen += 1;
        }
        chain.reverse();
        chain.join(" ")
    }

    fn filter_prims(&self, l: &Layer, t: f32, tt: f32, extra_blur: f32) -> String {
        let fx = &l.fx;
        let mut f = String::new();
        let mut last = "SourceGraphic".to_string();
        let hue = fx.hue + l.value(18, t);
        if hue != 0.0 || (fx.saturation - 100.0).abs() > 0.01 {
            write!(f, r#"<feColorMatrix in="{last}" type="hueRotate" values="{hue}" result="h"/><feColorMatrix in="h" type="saturate" values="{}" result="hs"/>"#, fx.saturation / 100.0).unwrap();
            last = "hs".into();
        }
        let matrix = |f: &mut String, last: &mut String, m: [f32; 20], name: &str| {
            let v: Vec<String> = m.iter().map(|x| format!("{x:.4}")).collect();
            write!(f, r#"<feColorMatrix in="{last}" type="matrix" values="{}" result="{name}"/>"#, v.join(" ")).unwrap();
            *last = name.into();
        };
        if fx.sepia > 0.0 {
            let a = (fx.sepia / 100.0).clamp(0.0, 1.0);
            let s = [0.393, 0.769, 0.189, 0.349, 0.686, 0.168, 0.272, 0.534, 0.131];
            let mut m = [0.0f32; 20];
            for r in 0..3 {
                for c in 0..3 {
                    m[r * 5 + c] = (if r == c { 1.0 - a } else { 0.0 }) + a * s[r * 3 + c];
                }
            }
            m[18] = 1.0;
            matrix(&mut f, &mut last, m, "sp");
        }
        if fx.tint > 0.0 {
            let a = (fx.tint / 100.0).clamp(0.0, 1.0);
            let lum = [0.2126, 0.7152, 0.0722];
            let mut m = [0.0f32; 20];
            for r in 0..3 {
                let (d, li) = (fx.tint_dark[r] as f32 / 255.0, fx.tint_light[r] as f32 / 255.0);
                for c in 0..3 {
                    m[r * 5 + c] = (if r == c { 1.0 - a } else { 0.0 }) + a * (li - d) * lum[c];
                }
                m[r * 5 + 4] = a * d;
            }
            m[18] = 1.0;
            matrix(&mut f, &mut last, m, "tn");
        }
        if fx.invert > 0.0 {
            let a = (fx.invert / 100.0).clamp(0.0, 1.0);
            let k = 1.0 - 2.0 * a;
            matrix(&mut f, &mut last, [k, 0.0, 0.0, 0.0, a, 0.0, k, 0.0, 0.0, a, 0.0, 0.0, k, 0.0, a, 0.0, 0.0, 0.0, 1.0, 0.0], "iv");
        }
        if (fx.brightness - 100.0).abs() > 0.01 || (fx.contrast - 100.0).abs() > 0.01 {
            let (k, c) = (fx.brightness / 100.0, fx.contrast / 100.0);
            let (slope, icpt) = (k * c, 0.5 * (1.0 - c));
            write!(f, r#"<feComponentTransfer in="{last}" result="br"><feFuncR type="linear" slope="{slope}" intercept="{icpt}"/><feFuncG type="linear" slope="{slope}" intercept="{icpt}"/><feFuncB type="linear" slope="{slope}" intercept="{icpt}"/></feComponentTransfer>"#).unwrap();
            last = "br".into();
        }
        if (fx.gamma - 1.0).abs() > 0.01 {
            let e = 1.0 / fx.gamma.max(0.05);
            write!(f, r#"<feComponentTransfer in="{last}" result="gm"><feFuncR type="gamma" exponent="{e}"/><feFuncG type="gamma" exponent="{e}"/><feFuncB type="gamma" exponent="{e}"/></feComponentTransfer>"#).unwrap();
            last = "gm".into();
        }
        if fx.threshold > 0.0 {
            let lv = (fx.threshold / 100.0).clamp(0.0, 1.0);
            let table: Vec<&str> = (0..32).map(|i| if (i as f32 + 0.5) / 32.0 >= lv { "1" } else { "0" }).collect();
            let tv = table.join(" ");
            write!(f, r#"<feColorMatrix in="{last}" type="saturate" values="0" result="th0"/><feComponentTransfer in="th0" result="th"><feFuncR type="discrete" tableValues="{tv}"/><feFuncG type="discrete" tableValues="{tv}"/><feFuncB type="discrete" tableValues="{tv}"/></feComponentTransfer>"#).unwrap();
            last = "th".into();
        }
        if fx.posterize > 1 {
            let n = fx.posterize.min(32);
            let tv: Vec<String> = (0..n).map(|i| format!("{:.3}", i as f32 / (n - 1) as f32)).collect();
            let tv = tv.join(" ");
            write!(f, r#"<feComponentTransfer in="{last}" result="po"><feFuncR type="discrete" tableValues="{tv}"/><feFuncG type="discrete" tableValues="{tv}"/><feFuncB type="discrete" tableValues="{tv}"/></feComponentTransfer>"#).unwrap();
            last = "po".into();
        }
        if fx.fill_on {
            write!(f, r#"<feFlood flood-color="{}" flood-opacity="{}" result="fl0"/><feComposite in="fl0" in2="{last}" operator="in" result="fl"/>"#, hex(fx.fill_color), fx.fill_color[3] as f32 / 255.0).unwrap();
            last = "fl".into();
        }
        if fx.rgb_split > 0.0 {
            let (dx, dy) = rotate(fx.rgb_split, 0.0, fx.rgb_angle);
            write!(
                f,
                r#"<feColorMatrix in="{last}" type="matrix" values="1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0" result="cr"/><feOffset in="cr" dx="{dx}" dy="{dy}" result="cro"/><feColorMatrix in="{last}" type="matrix" values="0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 1 0" result="cg"/><feColorMatrix in="{last}" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 1 0" result="cb"/><feOffset in="cb" dx="{}" dy="{}" result="cbo"/><feBlend in="cro" in2="cg" mode="screen" result="crg"/><feBlend in="crg" in2="cbo" mode="screen" result="rgb"/>"#,
                -dx,
                -dy
            )
            .unwrap();
            last = "rgb".into();
        }
        if fx.distort > 0.0 {
            let base = 1.0 / fx.distort_scale.max(2.0);
            let fr = base * (1.0 + 0.15 * (tt * fx.distort_speed).sin());
            write!(f, r#"<feTurbulence type="turbulence" baseFrequency="{fr:.5}" numOctaves="2" seed="3" result="dt"/><feDisplacementMap in="{last}" in2="dt" scale="{}" xChannelSelector="R" yChannelSelector="G" result="dm"/>"#, fx.distort).unwrap();
            last = "dm".into();
        }
        if fx.grain > 0.0 {
            let a = (fx.grain / 100.0).clamp(0.0, 1.0);
            let seed = (tt * 24.0).floor() as i32;
            write!(f, r#"<feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="2" seed="{seed}" result="gn0"/><feColorMatrix in="gn0" type="saturate" values="0" result="gn1"/><feComposite in="gn1" in2="{last}" operator="in" result="gn2"/><feComposite in="gn2" in2="{last}" operator="arithmetic" k1="0" k2="{a}" k3="1" k4="{}" result="gn"/>"#, -a / 2.0).unwrap();
            last = "gn".into();
        }
        let blur = fx.blur + extra_blur.max(0.0);
        if blur > 0.0 {
            write!(f, r#"<feGaussianBlur in="{last}" stdDeviation="{blur}" result="b"/>"#).unwrap();
            last = "b".into();
        }
        if fx.glow > 0.0 {
            write!(
                f,
                r#"<feGaussianBlur in="{last}" stdDeviation="{}" result="g0"/><feFlood flood-color="{}" flood-opacity="{}"/><feComposite in2="g0" operator="in" result="g1"/><feMerge result="gl"><feMergeNode in="g1"/><feMergeNode in="g1"/><feMergeNode in="{last}"/></feMerge>"#,
                fx.glow,
                hex(fx.glow_color),
                fx.glow_color[3] as f32 / 255.0
            )
            .unwrap();
            last = "gl".into();
        }
        if fx.inner {
            write!(
                f,
                r#"<feComponentTransfer in="SourceAlpha" result="ia"><feFuncA type="table" tableValues="1 0"/></feComponentTransfer><feOffset in="ia" dx="{}" dy="{}" result="ib"/><feGaussianBlur in="ib" stdDeviation="{}" result="ic"/><feFlood flood-color="{}" flood-opacity="{}" result="id"/><feComposite in="id" in2="ic" operator="in" result="ie"/><feComposite in="ie" in2="SourceAlpha" operator="in" result="ig"/><feMerge result="is"><feMergeNode in="{last}"/><feMergeNode in="ig"/></feMerge>"#,
                fx.inner_dx,
                fx.inner_dy,
                fx.inner_blur,
                hex(fx.inner_color),
                fx.inner_color[3] as f32 / 255.0
            )
            .unwrap();
            last = "is".into();
        }
        if fx.shadow {
            write!(
                f,
                r#"<feDropShadow in="{last}" dx="{}" dy="{}" stdDeviation="{}" flood-color="{}" flood-opacity="{}"/>"#,
                fx.shadow_dx,
                fx.shadow_dy,
                fx.shadow_blur,
                hex(fx.shadow_color),
                (fx.shadow_alpha / 100.0).clamp(0.0, 1.0)
            )
            .unwrap();
        }
        f
    }

    fn body(&self, li: usize, t: f32, uid: &str, defs: &mut String) -> String {
        let l = &self.layers[li];
        let fx = &l.fx;
        let tt = l.local_time(t);
        let reveal = l.value(7, t).clamp(0.0, 100.0);
        let sw = l.value(17, t).max(0.0);
        let fill_override = l.fill_color(t);
        let fill_of = |c: [u8; 4], defs: &mut String| -> String {
            let c = fill_override.unwrap_or(c);
            match fx.gradient {
                Some(g) => {
                    let stops = format!(r#"<stop offset="0" stop-color="{}" stop-opacity="{}"/><stop offset="1" stop-color="{}" stop-opacity="{}"/>"#, hex(c), c[3] as f32 / 255.0, hex(g), g[3] as f32 / 255.0);
                    if fx.gradient_kind == 1 {
                        write!(defs, r#"<radialGradient id="g{uid}" cx="{}" cy="{}" r="{}">{stops}</radialGradient>"#, fx.grad_cx / 100.0, fx.grad_cy / 100.0, fx.grad_r / 100.0).unwrap();
                    } else {
                        let (dx, dy) = rotate(0.0, 0.5, fx.gradient_angle);
                        write!(defs, r#"<linearGradient id="g{uid}" x1="{}" y1="{}" x2="{}" y2="{}">{stops}</linearGradient>"#, 0.5 - dx, 0.5 - dy, 0.5 + dx, 0.5 + dy).unwrap();
                    }
                    format!(r#"fill="url(#g{uid})""#)
                }
                None => color(c),
            }
        };
        let stroke_color = fx.stroke.map(|c| color_at(&l.stroke_keys, tt).unwrap_or(c));
        let cap = CAPS[(fx.cap as usize).min(2)];
        let join = JOINS[(fx.join as usize).min(2)];
        let stroke = match stroke_color {
            Some(c) if sw > 0.0 => {
                let dash = if fx.dash > 0.0 { format!(r#" stroke-dasharray="{} {}""#, fx.dash, (fx.dash * fx.dash_gap / 100.0).max(0.1)) } else { String::new() };
                format!(r#" stroke="{}" stroke-opacity="{}" stroke-width="{sw}" stroke-linejoin="{join}" stroke-linecap="{cap}"{dash}"#, hex(c), c[3] as f32 / 255.0)
            }
            _ => String::new(),
        };
        let mut body = String::new();
        let (ts, te, to) = (l.value(14, t), l.value(15, t), l.value(16, t));
        let trimmed = ts > 0.01 || te < 99.99;
        let ops = fx.round > 0.0 || fx.pucker != 0.0 || fx.zig_amp != 0.0 || fx.rough > 0.0 || fx.twist != 0.0;
        let route = matches!(l.content, Content::Strokes { .. }) || ((ops || trimmed) && l.content.is_shape()) || (matches!(l.content, Content::Path { .. }) && (ops || trimmed));
        if route && let Some(o) = l.outline() {
            let polys = trim(shape_ops(fx, o.polys, tt), ts, te, to, fx.trim_mode == 1);
            let d = polys_d(&polys);
            if !d.is_empty() {
                if let Some(c) = o.fill {
                    let paint = fill_of(c, defs);
                    write!(body, r#"<path d="{d}" {paint}{stroke}/>"#).unwrap();
                } else if let Some((c, w)) = o.line {
                    let c = if matches!(l.content, Content::Strokes { .. } | Content::Line { .. } | Content::Ring { .. }) { fill_override.unwrap_or(c) } else { c };
                    write!(body, r#"<path d="{d}" fill="none" stroke="{}" stroke-opacity="{}" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round"/>"#, hex(c), c[3] as f32 / 255.0).unwrap();
                } else if !stroke.is_empty() {
                    write!(body, r#"<path d="{d}" fill="none"{stroke}/>"#).unwrap();
                }
            }
        } else {
            match &l.content {
                Content::Rect { w, h, r, color: c } => {
                    let paint = fill_of(*c, defs);
                    write!(body, r#"<rect x="{}" y="{}" width="{w}" height="{h}" rx="{r}" {paint}{stroke}/>"#, -w / 2.0, -h / 2.0).unwrap()
                }
                Content::Ellipse { w, h, color: c } => {
                    let paint = fill_of(*c, defs);
                    write!(body, r#"<ellipse rx="{}" ry="{}" {paint}{stroke}/>"#, w / 2.0, h / 2.0).unwrap()
                }
                Content::Star { points, radius, inner, color: c } => {
                    let paint = fill_of(*c, defs);
                    let pts: Vec<String> = star_points(*points, *radius, *inner).iter().map(|p| format!("{:.2},{:.2}", p[0], p[1])).collect();
                    write!(body, r#"<polygon points="{}" {paint}{stroke}/>"#, pts.join(" ")).unwrap()
                }
                Content::Line { w, thickness, color: c } => {
                    let paint = fill_of(*c, defs);
                    write!(body, r#"<rect x="{}" y="{}" width="{w}" height="{thickness}" rx="{}" {paint}{stroke}/>"#, -w / 2.0, -thickness / 2.0, thickness / 2.0).unwrap()
                }
                Content::Arrow { w, thickness: th, color: c } => {
                    let paint = fill_of(*c, defs);
                    let pts: Vec<String> = arrow_points(*w, *th).iter().map(|p| format!("{:.2},{:.2}", p[0], p[1])).collect();
                    write!(body, r#"<polygon points="{}" {paint}{stroke}/>"#, pts.join(" ")).unwrap()
                }
                Content::Ring { radius, thickness, color: c } => {
                    let c = fill_override.unwrap_or(*c);
                    let sc = format!(r#"fill="none" stroke="{}" stroke-opacity="{}" stroke-width="{thickness}" stroke-linecap="round""#, hex(c), c[3] as f32 / 255.0);
                    if reveal >= 99.95 {
                        write!(body, r#"<circle r="{radius}" {sc}/>"#).unwrap();
                    } else if reveal > 0.05 {
                        let a = (reveal / 100.0 * 360.0 - 90.0).to_radians();
                        let large = if reveal > 50.0 { 1 } else { 0 };
                        write!(body, r#"<path d="M 0 {} A {radius} {radius} 0 {large} 1 {:.3} {:.3}" {sc}/>"#, -radius, a.cos() * radius, a.sin() * radius).unwrap();
                    }
                }
                Content::Path { pts, closed, color: c, fill } => {
                    let d = path_d(pts, *closed);
                    let paint = if *fill { fill_of(*c, defs) } else { r#"fill="none""#.to_string() };
                    if !d.is_empty() {
                        write!(body, r#"<path d="{d}" {paint}{stroke}/>"#).unwrap();
                    }
                }
                Content::Text { text, size, color: c, bold, family, italic, tracking, align, vertical } => {
                    let paint = fill_of(*c, defs);
                    let alpha = fill_override.unwrap_or(*c)[3] as f32 / 255.0;
                    let stroke_alpha = stroke_color.map(|s| s[3] as f32 / 255.0);
                    let total = text.chars().count();
                    let mut left = if fx.typewriter > 0.0 { (((tt - l.start) / fx.typewriter).clamp(0.0, 1.0) * total as f32).round() as usize } else { total };
                    let lines: Vec<&str> = text.lines().collect();
                    let n = lines.len().max(1) as f32;
                    let ta = &fx.ta;
                    let nchars: usize = lines.iter().map(|x| x.chars().count()).sum();
                    let rk = ranks(nchars, ta.order);
                    let maxr = rk.iter().copied().fold(0.0f32, f32::max);
                    let anim = |idx: usize| -> (f32, f32, f32, f32, f32) {
                        let r = rk.get(idx).copied().unwrap_or(0.0);
                        let dur = ta.dur.max(0.01);
                        let fin = 1.0 - ta.ease.apply(((tt - l.start - r * ta.delay) / dur).clamp(0.0, 1.0));
                        let fout = 1.0 - ta.ease.apply(((l.end - tt - (maxr - r) * ta.delay) / dur).clamp(0.0, 1.0));
                        let f = match ta.mode {
                            0 => fin,
                            1 => fout,
                            _ => fin.max(fout),
                        };
                        (100.0 + (ta.opacity - 100.0) * f, ta.dx * f, ta.dy * f, 100.0 + (ta.scale - 100.0) * f, ta.rot * f)
                    };
                    let char_attrs = |op: f32, sc: f32, rot: f32| {
                        let mut a = format!(r#" font-size="{:.2}" fill-opacity="{:.3}""#, (size * sc / 100.0).max(0.01), (alpha * op / 100.0).clamp(0.0, 1.0));
                        if rot != 0.0 {
                            write!(a, r#" rotate="{rot:.2}""#).unwrap();
                        }
                        if let Some(sa) = stroke_alpha {
                            write!(a, r#" stroke-opacity="{:.3}""#, (sa * op / 100.0).clamp(0.0, 1.0)).unwrap();
                        }
                        a
                    };
                    let anchor = if *vertical { "middle" } else { ["middle", "start", "end"][(*align as usize).min(2)] };
                    let extra = format!("{}{}", if *italic { r#" font-style="italic""# } else { "" }, if *tracking != 0.0 && !*vertical { format!(r#" letter-spacing="{tracking}""#) } else { String::new() });
                    write!(body, r#"<text text-anchor="{anchor}" font-size="{size}" font-family="{}" font-weight="{}"{extra} {paint}{stroke}>"#, esc(family), if *bold { "bold" } else { "normal" }).unwrap();
                    let mut idx = 0usize;
                    if *vertical {
                        let m = lines.iter().map(|x| x.chars().count()).max().unwrap_or(1).max(1) as f32;
                        for (i, line) in lines.iter().enumerate() {
                            let x = (i as f32 - (n - 1.0) / 2.0) * size * 1.2;
                            let count = line.chars().count();
                            for (j, ch) in line.chars().enumerate() {
                                if j >= left {
                                    break;
                                }
                                let y = (j as f32 - (m - 1.0) / 2.0) * size * 1.05 + size * 0.35;
                                let (op, dx, dy, sc, rot) = if ta.on { anim(idx + j) } else { (100.0, 0.0, 0.0, 100.0, 0.0) };
                                write!(body, r#"<tspan x="{:.2}" y="{:.2}"{}>{}</tspan>"#, x + dx, y + dy, char_attrs(op, sc, rot), esc(&ch.to_string())).unwrap();
                            }
                            idx += count;
                            left = left.saturating_sub(count + 1);
                        }
                    } else {
                        let top = -(n - 1.0) * size * 1.2 / 2.0 + size * 0.35;
                        for (i, line) in lines.iter().enumerate() {
                            let count = line.chars().count();
                            let part: Vec<char> = line.chars().take(left).collect();
                            left = left.saturating_sub(count + 1);
                            let y = top + i as f32 * size * 1.2;
                            if !ta.on {
                                write!(body, r#"<tspan x="0" y="{y}">{}</tspan>"#, esc(&part.iter().collect::<String>())).unwrap();
                            } else {
                                write!(body, r#"<tspan x="0" y="{y}">"#).unwrap();
                                let (mut px, mut py) = (0.0, 0.0);
                                for (j, ch) in part.iter().enumerate() {
                                    let (op, dx, dy, sc, rot) = anim(idx + j);
                                    write!(body, r#"<tspan dx="{:.2}" dy="{:.2}"{}>{}</tspan>"#, dx - px, dy - py, char_attrs(op, sc, rot), esc(&ch.to_string())).unwrap();
                                    (px, py) = (dx, dy);
                                }
                                body.push_str("</tspan>");
                            }
                            idx += count;
                        }
                    }
                    body.push_str("</text>");
                }
                Content::Counter { from, to, decimals, size, color: c, prefix, suffix } => {
                    let paint = fill_of(*c, defs);
                    let u = ((tt - l.start) / (l.end - l.start).max(1e-3) / 0.8).clamp(0.0, 1.0);
                    let u = 1.0 - (1.0 - u).powi(3);
                    let v = from + (to - from) * u;
                    write!(body, r#"<text text-anchor="middle" y="{}" font-size="{size}" font-family="Segoe UI" font-weight="bold" {paint}{stroke}>{}</text>"#, size * 0.35, esc(&format!("{prefix}{v:.*}{suffix}", *decimals as usize))).unwrap();
                }
                Content::Image { path, w, h } => {
                    write!(body, r#"<image x="{}" y="{}" width="{w}" height="{h}" preserveAspectRatio="none" xlink:href="{}"/>"#, -w / 2.0, -h / 2.0, esc(path)).unwrap()
                }
                Content::Strokes { .. } | Content::Null | Content::Adjust { .. } => {}
            }
        }
        if fx.flip_x || fx.flip_y {
            body = format!(r#"<g transform="scale({} {})">{body}</g>"#, if fx.flip_x { -1 } else { 1 }, if fx.flip_y { -1 } else { 1 });
        }
        let blur = fx.blur + l.value(6, t).max(0.0);
        let f = self.filter_prims(l, t, tt, l.value(6, t));
        if !f.is_empty() {
            write!(defs, r#"<filter id="f{uid}" x="-60%" y="-60%" width="220%" height="220%">{f}</filter>"#).unwrap();
            body = format!(r#"<g filter="url(#f{uid})">{body}</g>"#);
        }
        if fx.dir_blur > 0.0 {
            write!(defs, r#"<filter id="d{uid}" x="-100%" y="-100%" width="300%" height="300%"><feGaussianBlur stdDeviation="{} 0"/></filter>"#, fx.dir_blur).unwrap();
            body = format!(r#"<g transform="rotate({a})"><g filter="url(#d{uid})"><g transform="rotate({})">{body}</g></g></g>"#, -fx.dir_angle, a = fx.dir_angle);
        }
        if fx.zoom_blur > 0.0 {
            let mut z = String::new();
            for k in (1..=8).rev() {
                write!(z, r#"<g transform="scale({:.4})" opacity="{:.3}">{body}</g>"#, 1.0 + fx.zoom_blur / 100.0 * k as f32 / 8.0, 0.45 * (1.0 - k as f32 / 9.0)).unwrap();
            }
            body = z + &body;
        }
        if fx.glitch > 0.0 {
            let (bx0, by0, bx1, by1) = l.bounds();
            let q = (tt * fx.glitch_freq.max(0.1)).floor() as i64 as u32;
            let strips = 7;
            let h = (by1 - by0 + 40.0) / strips as f32;
            let offs: Vec<f32> = (0..strips).map(|k| if hash(q, k) > 0.55 { (hash(q.wrapping_add(91), k) - 0.5) * 2.0 * fx.glitch } else { 0.0 }).collect();
            if offs.iter().any(|o| *o != 0.0) {
                let mut g = String::new();
                for (k, o) in offs.iter().enumerate() {
                    let y = by0 - 20.0 + k as f32 * h;
                    write!(defs, r#"<clipPath id="q{uid}_{k}"><rect x="{}" y="{y}" width="{}" height="{}"/></clipPath>"#, bx0 - 2000.0, bx1 - bx0 + 4000.0, h + 0.5).unwrap();
                    write!(g, r#"<g clip-path="url(#q{uid}_{k})"><g transform="translate({o} 0)">{body}</g></g>"#).unwrap();
                }
                body = g;
            }
        }
        if reveal < 99.95 && !matches!(l.content, Content::Ring { .. }) {
            let (bx0, by0, bx1, by1) = l.bounds();
            let pad = 40.0 + blur * 3.0 + fx.glow * 3.0;
            let (w, h) = (bx1 - bx0, by1 - by0);
            write!(defs, r#"<clipPath id="c{uid}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath>"#, bx0 - pad, by0 - h / 2.0 - pad, (w + pad * 2.0) * reveal / 100.0, h * 2.0 + pad * 2.0).unwrap();
            body = format!(r#"<g clip-path="url(#c{uid})">{body}</g>"#);
        }
        body
    }

    fn layer_svg(&self, li: usize, t: f32, fade: f32, uid: &str, defs: &mut String) -> String {
        let l = &self.layers[li];
        let fx = &l.fx;
        let body = self.body(li, t, uid, defs);
        let mut inner = String::new();
        let copies = fx.repeat.min(60);
        for c in 0..=copies {
            if c == 0 {
                inner.push_str(&body);
                continue;
            }
            let k = c as f32;
            let op = (1.0 - k * fx.rep_fade / 100.0).clamp(0.0, 1.0);
            write!(inner, r#"<g transform="translate({} {}) rotate({}) scale({})" opacity="{op}">{body}</g>"#, fx.rep_dx * k, fx.rep_dy * k, fx.rep_rot * k, (fx.rep_scale / 100.0).powf(k)).unwrap();
        }
        let blend = BLENDS.get(fx.blend as usize).filter(|b| b.0 != "normal").map(|b| format!(r#" style="mix-blend-mode:{}""#, b.0)).unwrap_or_default();
        let flick = if fx.flicker > 0.0 { 1.0 - fx.flicker / 100.0 * hash((l.local_time(t) * 15.0).floor() as i64 as u32, li as u32) } else { 1.0 };
        let op = (l.value(4, t) / 100.0 * fade * flick).clamp(0.0, 1.0);
        format!(r#"<g transform="{}" opacity="{op}"{blend}>{inner}</g>"#, self.xform(li, t))
    }

    pub fn to_svg(&self, t: f32) -> String {
        self.to_svg_with(t, false)
    }

    pub fn to_svg_with(&self, t: f32, transparent: bool) -> String {
        let mut s = String::new();
        writeln!(
            s,
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
            w = self.width,
            h = self.height
        )
        .unwrap();
        if !transparent {
            match self.background2 {
                Some(b2) => writeln!(
                    s,
                    r#"<defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{}" stop-opacity="{}"/><stop offset="1" stop-color="{}" stop-opacity="{}"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#bg)"/>"#,
                    hex(self.background),
                    self.background[3] as f32 / 255.0,
                    hex(b2),
                    b2[3] as f32 / 255.0
                )
                .unwrap(),
                None => writeln!(s, r#"<rect width="100%" height="100%" {}/>"#, color(self.background)).unwrap(),
            }
        }
        let solo = self.layers.iter().any(|l| l.solo);
        let mut defs = String::new();
        let mut acc = String::new();
        for (li, l) in self.layers.iter().enumerate() {
            if !l.active(t) || (solo && !l.solo) {
                continue;
            }
            if li > 0 && self.layers[li - 1].fx.matte > 0 {
                continue;
            }
            match l.content {
                Content::Null => continue,
                Content::Adjust { .. } => {
                    let f = self.filter_prims(l, t, l.local_time(t), l.value(6, t));
                    let op = (l.value(4, t) / 100.0).clamp(0.0, 1.0);
                    if !f.is_empty() && op > 0.001 && !acc.is_empty() {
                        write!(defs, r#"<filter id="a{li}" x="-10%" y="-10%" width="120%" height="120%">{f}</filter>"#).unwrap();
                        acc = if op >= 0.999 { format!(r#"<g filter="url(#a{li})">{acc}</g>"#) } else { format!(r#"<g>{acc}</g><g filter="url(#a{li})" opacity="{op}">{acc}</g>"#) };
                        acc.push('\n');
                    }
                    continue;
                }
                _ => {}
            }
            let mut g = String::new();
            let fx = &l.fx;
            for k in (1..=fx.echo.min(12)).rev() {
                let te = t - k as f32 * fx.echo_delay.max(0.01);
                if te >= l.start {
                    g.push_str(&self.layer_svg(li, te, (1.0 - k as f32 / (fx.echo + 1) as f32) * 0.6, &format!("{li}e{k}"), &mut defs));
                }
            }
            g.push_str(&self.layer_svg(li, t, 1.0, &li.to_string(), &mut defs));
            if fx.matte > 0
                && let Some(m) = self.layers.get(li + 1)
                && m.visible
            {
                let content = if m.active(t) { self.layer_svg(li + 1, t, 1.0, &format!("{}m", li + 1), &mut defs) } else { String::new() };
                let white = r#"<rect x="-100000" y="-100000" width="200000" height="200000" fill="white"/>"#;
                let (kind, content) = match fx.matte {
                    1 => ("alpha", content),
                    3 => {
                        write!(defs, r#"<filter id="k{li}" x="-50%" y="-50%" width="200%" height="200%"><feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0"/></filter>"#).unwrap();
                        ("luminance", format!(r#"{white}<g filter="url(#k{li})">{content}</g>"#))
                    }
                    4 => {
                        write!(defs, r#"<filter id="k{li}" x="-50%" y="-50%" width="200%" height="200%"><feColorMatrix type="matrix" values="-1 0 0 0 1 0 -1 0 0 1 0 0 -1 0 1 0 0 0 1 0"/></filter>"#).unwrap();
                        ("luminance", format!(r#"{white}<g filter="url(#k{li})">{content}</g>"#))
                    }
                    _ => ("luminance", content),
                };
                write!(defs, r#"<mask id="m{li}" mask-type="{kind}" maskUnits="userSpaceOnUse" x="-100000" y="-100000" width="200000" height="200000">{content}</mask>"#).unwrap();
                g = format!(r#"<g mask="url(#m{li})">{g}</g>"#);
            }
            acc.push_str(&g);
            acc.push('\n');
        }
        if !defs.is_empty() {
            writeln!(s, "<defs>{defs}</defs>").unwrap();
        }
        s.push_str(&acc);
        let (w, h) = (self.width as f32, self.height as f32);
        if self.letterbox > 1.0 {
            let bar = ((h - w / self.letterbox) / 2.0).max(0.0);
            if bar > 0.0 {
                writeln!(s, r#"<rect width="{w}" height="{bar}" fill="black"/><rect y="{}" width="{w}" height="{bar}" fill="black"/>"#, h - bar).unwrap();
            }
        }
        if self.vignette > 0.0 {
            writeln!(s, r#"<defs><radialGradient id="vig" cx="0.5" cy="0.5" r="0.75"><stop offset="0.45" stop-color="black" stop-opacity="0"/><stop offset="1" stop-color="black" stop-opacity="{}"/></radialGradient></defs><rect width="100%" height="100%" fill="url(#vig)"/>"#, (self.vignette / 100.0).clamp(0.0, 1.0)).unwrap();
        }
        if self.grain > 0.0 {
            let seed = (t * 24.0).floor() as i32;
            writeln!(s, r#"<defs><filter id="grn" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.8" numOctaves="2" seed="{seed}"/><feColorMatrix type="saturate" values="0"/></filter></defs><rect width="100%" height="100%" filter="url(#grn)" opacity="{}" style="mix-blend-mode:overlay"/>"#, (self.grain / 100.0).clamp(0.0, 1.0)).unwrap();
        }
        s.push_str("</svg>\n");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(svg: &str) {
        resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
    }

    #[test]
    fn keyframes_and_svg() {
        let mut p = Prop::new(10.0);
        assert_eq!(p.at(3.0), 10.0);
        p.set(0.0, 5.0);
        assert_eq!(p.base, 5.0);
        p.toggle_key(0.0);
        p.add_key(2.0, 105.0);
        p.keys[0].ease = Ease::Linear;
        assert_eq!(p.at(-1.0), 5.0);
        assert_eq!(p.at(1.0), 55.0);
        assert_eq!(p.at(9.0), 105.0);
        p.set(1.0, 0.0);
        assert_eq!(p.keys.len(), 3);
        assert_eq!(p.at(1.0), 0.0);
        p.toggle_key(1.0);
        assert_eq!(p.keys.len(), 2);
        p.keys[0].ease = Ease::Hold;
        assert_eq!(p.at(1.9), 5.0);

        let mut c = Comp::new();
        let mut l = Layer::new("T".into(), Content::text("A&B", 40.0, [255, 255, 255, 255], true), 100.0, 50.0, 2.0);
        l.props[4].add_key(0.0, 0.0);
        l.props[4].add_key(1.0, 100.0);
        c.layers.push(l);
        let svg = c.to_svg(0.5);
        assert!(svg.contains("translate(100 50)"));
        assert!(svg.contains(r#"opacity="0.5""#));
        assert!(svg.contains("A&amp;B"));
        assert!(!c.to_svg(3.0).contains("<g"));

        for (e, _) in Ease::ALL {
            assert!((e.apply(1.0) - if e == Ease::Hold { 0.0 } else { 1.0 }).abs() < 1e-3, "{e:?}");
        }
        for (e, _) in Ease2::ALL {
            assert!((e.apply(1.0).unwrap() - 1.0).abs() < 1e-3 && e.apply(0.0).unwrap().abs() < 1e-3, "{e:?}");
        }
        for (_, b) in BEZ_PRESETS {
            assert!(bezier(b, 0.0).abs() < 1e-3 && (bezier(b, 1.0) - 1.0).abs() < 1e-3);
        }
        assert!((bezier([0.0, 0.0, 1.0, 1.0], 0.3) - 0.3).abs() < 1e-3);
        assert!(Ease::Back.apply(0.8) > 1.0);
        let mut c2 = Comp::new();
        let mut word = Layer::new("w".into(), Content::text("Bonjour", 40.0, [255; 4], false), 0.0, 0.0, 4.0);
        word.fx.typewriter = 2.0;
        c2.layers.push(word);
        assert!(c2.to_svg(1.0).contains(">Bonj<") || c2.to_svg(1.0).contains(">Bon<"));
        let mut n = Layer::new("n".into(), Content::Counter { from: 0.0, to: 100.0, decimals: 0, size: 60.0, color: [255; 4], prefix: String::new(), suffix: " %".into() }, 0.0, 0.0, 1.0);
        n.fx.shadow = true;
        n.fx.gradient = Some([0, 0, 0, 255]);
        n.fx.blend = 1;
        c2.layers.push(n);
        let svg = c2.to_svg(1.0);
        assert!(svg.contains("100 %") && svg.contains("feDropShadow") && svg.contains("url(#g1)") && svg.contains("mix-blend-mode:multiply"));
        c2.layers.push(Layer::new("s".into(), Content::Star { points: 5, radius: 50.0, inner: 0.5, color: [255; 4] }, 0.0, 0.0, 4.0));
        let mut wig = Layer::new("l".into(), Content::Line { w: 100.0, thickness: 6.0, color: [255; 4] }, 10.0, 10.0, 4.0);
        wig.fx.wiggle = 20.0;
        wig.fx.wiggle_freq = 2.0;
        c2.layers.push(wig);
        let svg = c2.to_svg(0.3);
        assert!(svg.contains("<polygon") && !svg.contains("translate(10 10)"));
        parse(&svg);
        assert_eq!(c.frames(), 150);
    }

    #[test]
    fn legacy_files_load() {
        let old = r#"{"width":100,"height":100,"fps":30,"duration":2.0,"background":[0,0,0,255],"layers":[{"name":"a","content":{"Text":{"text":"x","size":10.0,"color":[1,2,3,255],"bold":true,"family":"Arial"}},"props":[{"base":1.0,"keys":[{"t":0.0,"v":1.0,"ease":"Smooth"}]},{"base":2.0,"keys":[]},{"base":100.0,"keys":[]},{"base":0.0,"keys":[]},{"base":100.0,"keys":[]}],"start":0.0,"end":2.0,"visible":true,"fx":{"blur":0.0,"shadow":true,"stroke":[255,0,0,255],"stroke_w":3.0}}]}"#;
        let mut c: Comp = serde_json::from_str(old).unwrap();
        assert_eq!(c.layers[0].fx.shadow_dy, 10.0);
        assert_eq!(c.layers[0].fx.speed, 100.0);
        assert_eq!(c.layers[0].value(7, 0.0), 100.0);
        assert_eq!(c.layers[0].value(17, 0.0), 3.0);
        assert!(c.to_svg(1.0).contains(r#"stroke-width="3""#));
        c.normalize();
        assert_eq!(c.layers[0].props.len(), PROPS.len());
        assert_eq!(c.layers[0].props[17].base, 3.0);
        parse(&c.to_svg(1.0));
        let back: Comp = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn parenting_matte_and_effects() {
        let mut c = Comp::new();
        let mut parent = Layer::new("p".into(), Content::Rect { w: 100.0, h: 100.0, r: 0.0, color: [255; 4] }, 500.0, 300.0, 5.0);
        parent.props[3].base = 90.0;
        c.layers.push(parent);
        c.layers.push(Layer::new("c".into(), Content::Ellipse { w: 20.0, h: 20.0, color: [255; 4] }, 600.0, 300.0, 5.0));
        assert!(c.set_parent(1, Some(0), 0.0));
        let (x, y, _, r) = c.world(1, 0.0);
        assert!((x - 600.0).abs() < 0.01 && (y - 300.0).abs() < 0.01 && r.abs() < 0.01, "{x} {y} {r}");
        c.layers[0].props[3].base = 180.0;
        let (x, y, _, _) = c.world(1, 0.0);
        assert!((x - 500.0).abs() < 0.01 && (y - 400.0).abs() < 0.01, "{x} {y}");
        assert!(!c.set_parent(0, Some(1), 0.0));
        assert!(c.set_parent(1, None, 0.0));
        assert!((c.layers[1].props[1].base - 400.0).abs() < 0.01 && (c.layers[1].props[3].base - 90.0).abs() < 0.01);
        let m = c.matrix(0, 0.0);
        let (x, y) = apply(invert(m), apply(m, 3.0, 4.0).0, apply(m, 3.0, 4.0).1);
        assert!((x - 3.0).abs() < 1e-3 && (y - 4.0).abs() < 1e-3);

        let mut c = Comp::new();
        let mut a = Layer::new("a".into(), Content::Rect { w: 200.0, h: 200.0, r: 10.0, color: [255, 0, 0, 255] }, 500.0, 500.0, 5.0);
        a.fx.matte = 1;
        a.fx.glow = 12.0;
        a.fx.hue = 40.0;
        a.fx.brightness = 120.0;
        a.fx.echo = 3;
        a.fx.repeat = 4;
        a.fx.flip_x = true;
        a.fx.stroke = Some([255; 4]);
        a.fx.stroke_w = 4.0;
        a.props[17].base = 4.0;
        a.fx.dash = 10.0;
        a.props[7].base = 50.0;
        a.props[5].base = 12.0;
        c.layers.push(a);
        c.layers.push(Layer::new("m".into(), Content::text("MASK", 80.0, [255; 4], true), 500.0, 500.0, 5.0));
        let mut ring = Layer::new("r".into(), Content::Ring { radius: 80.0, thickness: 12.0, color: [255; 4] }, 300.0, 300.0, 5.0);
        ring.props[7].base = 70.0;
        c.layers.push(ring);
        c.layers.push(Layer::new("ar".into(), Content::Arrow { w: 300.0, thickness: 20.0, color: [255; 4] }, 300.0, 300.0, 5.0));
        c.background2 = Some([0, 0, 80, 255]);
        let svg = c.to_svg(1.0);
        assert!(svg.contains(r#"mask="url(#m0)""#) && svg.contains("feFlood") && svg.contains("hueRotate") && svg.contains("skewX(12)"));
        assert!(svg.contains("clip-path") && svg.contains(" A 80 80 ") && svg.contains("stroke-dasharray") && svg.contains("url(#bg)"));
        assert_eq!(svg.matches("MASK").count(), 1);
        parse(&svg);
        assert!(!c.to_svg_with(1.0, true).contains("url(#bg)"));

        let mut l = Layer::new("k".into(), Content::Rect { w: 1.0, h: 1.0, r: 0.0, color: [255; 4] }, 0.0, 0.0, 9.0);
        l.props[0].add_key(1.0, 0.0);
        l.props[0].add_key(2.0, 10.0);
        l.props[0].keys[0].ease = Ease::Linear;
        l.fx.pingpong = true;
        assert!((l.value(0, 2.5) - 5.0).abs() < 1e-3);
        l.fx.pingpong = false;
        l.fx.loop_keys = true;
        assert!((l.value(0, 2.5) - 5.0).abs() < 1e-3);
        l.fx.loop_keys = false;
        l.props[0].cycle = 3;
        assert!((l.value(0, 2.5) - 15.0).abs() < 1e-3);
        l.props[0].cycle = 4;
        assert!((l.value(0, 3.0) - 20.0).abs() < 0.1);
        l.props[0].cycle = 0;
        l.fx.speed = 200.0;
        assert!((l.value(0, 1.0) - 10.0).abs() < 1e-3);
        l.fx.speed = 100.0;
        l.props[0].reverse(1.0, 2.0);
        assert_eq!(l.props[0].keys[0].v, 10.0);
        l.shift(1.0);
        assert_eq!(l.props[0].keys[0].t, 2.0);
        assert_eq!(l.start, 1.0);
    }

    #[test]
    fn paths_trim_operators_and_new_layers() {
        let sq = vec![([0.0, 0.0], 0), ([100.0, 0.0], 0), ([100.0, 100.0], 0), ([0.0, 100.0], 0)];
        let poly: Vec<[f32; 2]> = sq.iter().map(|p| p.0).collect();
        let half = trim(vec![(poly.clone(), true)], 0.0, 50.0, 0.0, false);
        assert_eq!(half.len(), 1);
        assert_eq!(*half[0].0.last().unwrap(), [100.0, 100.0]);
        let wrap = trim(vec![(poly.clone(), true)], 0.0, 50.0, 75.0, false);
        assert_eq!(wrap[0].0[0], [0.0, 100.0]);
        assert_eq!(*wrap[0].0.last().unwrap(), [100.0, 0.0]);
        let seq = trim(vec![(vec![[0.0, 0.0], [10.0, 0.0]], false), (vec![[0.0, 5.0], [10.0, 5.0]], false)], 0.0, 50.0, 0.0, true);
        assert_eq!(seq.len(), 1);
        assert!(trim(vec![(poly.clone(), true)], 30.0, 30.0, 0.0, false).is_empty());

        let mut pts = vec![Vtx::at(0.0, 0.0), Vtx::smooth(100.0, 50.0, 30.0, 0.0), Vtx::at(200.0, 0.0)];
        split_segment(&mut pts, 0, 0.5);
        assert_eq!(pts.len(), 4);
        assert!(path_d(&pts, true).contains(" C ") && path_d(&pts, true).ends_with('Z'));

        let mut c = Comp::new();
        let mut path = Layer::new("p".into(), Content::Path { pts: pts.clone(), closed: false, color: [255; 4], fill: false }, 400.0, 300.0, 5.0);
        path.fx.stroke = Some([255, 0, 0, 255]);
        path.props[17].base = 6.0;
        path.props[15].base = 60.0;
        path.fx.zig_amp = 6.0;
        c.layers.push(path);
        let mut st = Layer::new("s".into(), Content::Strokes { strokes: vec![vec![[0.0, 0.0], [50.0, 20.0], [90.0, 0.0]]], width: 8.0, color: [0, 255, 0, 255] }, 200.0, 200.0, 5.0);
        st.color_keys = vec![CKey { t: 0.0, c: [0, 0, 0, 255] }, CKey { t: 2.0, c: [255, 255, 255, 255] }];
        c.layers.push(st);
        let mut shape = Layer::new("r".into(), Content::Star { points: 5, radius: 80.0, inner: 0.5, color: [255; 4] }, 600.0, 300.0, 5.0);
        shape.fx.round = 10.0;
        shape.fx.pucker = 20.0;
        shape.fx.twist = 40.0;
        shape.fx.rough = 3.0;
        shape.fx.gradient = Some([0, 0, 255, 255]);
        shape.fx.gradient_kind = 1;
        shape.props[8].base = 10.0;
        shape.props[13].base = 40.0;
        shape.fx.blend = 15;
        c.layers.push(shape);
        let mut txt = Layer::new("t".into(), Content::text("Salut\nvous", 60.0, [255; 4], true), 900.0, 500.0, 5.0);
        txt.fx.ta.on = true;
        txt.fx.ta.order = 3;
        txt.fx.ta.rot = 30.0;
        txt.fx.stroke = Some([0, 0, 0, 255]);
        txt.props[17].base = 2.0;
        c.layers.push(txt);
        let mut vt = Layer::new("v".into(), Content::text("ABC", 50.0, [255; 4], false), 100.0, 500.0, 5.0);
        if let Content::Text { vertical, .. } = &mut vt.content {
            *vertical = true;
        }
        c.layers.push(vt);
        c.layers.push(Layer::new("n".into(), Content::Null, 0.0, 0.0, 5.0));
        let mut adj = Layer::new("a".into(), Content::Adjust { w: 1920.0, h: 1080.0 }, 960.0, 540.0, 5.0);
        adj.fx.sepia = 80.0;
        adj.fx.tint = 40.0;
        adj.fx.invert = 20.0;
        adj.fx.contrast = 130.0;
        adj.fx.gamma = 1.4;
        adj.fx.threshold = 50.0;
        adj.fx.posterize = 4;
        adj.props[4].base = 60.0;
        c.layers.push(adj);
        let mut fxl = Layer::new("f".into(), Content::Ellipse { w: 100.0, h: 80.0, color: [255; 4] }, 300.0, 700.0, 5.0);
        fxl.fx.rgb_split = 6.0;
        fxl.fx.grain = 30.0;
        fxl.fx.distort = 20.0;
        fxl.fx.dir_blur = 10.0;
        fxl.fx.zoom_blur = 20.0;
        fxl.fx.glitch = 30.0;
        fxl.fx.inner = true;
        fxl.fx.fill_on = true;
        fxl.fx.flicker = 50.0;
        fxl.fx.matte = 4;
        fxl.fx.auto_orient = true;
        c.layers.push(fxl);
        c.layers.push(Layer::new("m".into(), Content::Rect { w: 200.0, h: 200.0, r: 0.0, color: [255; 4] }, 300.0, 700.0, 5.0));
        c.vignette = 50.0;
        c.grain = 20.0;
        c.letterbox = 2.39;
        let svg = c.to_svg(0.3);
        assert!(svg.contains("radialGradient") && svg.contains("rotate=") && svg.contains("feTurbulence") && svg.contains("feDisplacementMap"));
        assert!(svg.contains(r#"filter="url(#a6)""#) && svg.contains("mix-blend-mode:luminosity") && svg.contains("fill=\"none\" stroke=\"#0f0f0f\""));
        assert!(svg.contains("url(#vig)") && svg.contains("feBlend") && svg.contains(r#"<tspan x="#));
        parse(&svg);
        let back: Comp = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);
        assert!(c.layers[4].size().1 > c.layers[4].size().0);
        let mut p = Prop::new(0.0);
        p.wiggle = 10.0;
        p.wiggle_freq = 2.0;
        assert!(p.eval(0.3, 1).abs() > 0.01 && p.at(0.3) == 0.0);
    }
}


pub const ANIM_PRESETS: [&str; 9] = ["Apparition en fondu", "Zoom d'entrée avec rebond", "Glisser depuis la gauche", "Glisser depuis la droite", "Monter depuis le bas", "Chute avec rebond", "Entrée en tournant", "Sortie en fondu", "Secousse continue"];

fn put_keys(p: &mut Prop, keys: &[(f32, f32, Ease)]) {
    let (lo, hi) = keys.iter().fold((f32::MAX, f32::MIN), |a, k| (a.0.min(k.0), a.1.max(k.0)));
    p.keys.retain(|k| k.t < lo - 1e-3 || k.t > hi + 1e-3);
    p.keys.extend(keys.iter().map(|(t, v, e)| Key { t: *t, v: *v, ease: *e, ease2: Ease2::default(), bez: None }));
    p.keys.sort_by(|a, b| a.t.total_cmp(&b.t));
}

/// Adds a ready-made animation to a layer, relative to its current values at its start (or end).
pub fn apply_anim_preset(l: &mut Layer, k: usize, comp_w: f32, comp_h: f32) {
    let (s, e) = (l.start, l.end.max(l.start + 0.2));
    let at = |l: &Layer, i: usize, t: f32| l.props.get(i).map_or(0.0, |p| p.at(t));
    let (x, y, sc, rot, op) = (at(l, 0, s), at(l, 1, s), at(l, 2, s), at(l, 3, s), at(l, 4, s));
    let d = 0.6f32.min((e - s) / 2.0);
    match k {
        0 => put_keys(&mut l.props[4], &[(s, 0.0, Ease::Smooth), (s + d, op, Ease::Smooth)]),
        1 => {
            put_keys(&mut l.props[2], &[(s, 0.0, Ease::Back), (s + d, sc, Ease::Smooth)]);
            put_keys(&mut l.props[4], &[(s, 0.0, Ease::Smooth), (s + d * 0.5, op, Ease::Smooth)]);
        }
        2 | 3 => {
            let from = if k == 2 { x - comp_w * 0.6 } else { x + comp_w * 0.6 };
            put_keys(&mut l.props[0], &[(s, from, Ease::Out), (s + d, x, Ease::Smooth)]);
        }
        4 => {
            put_keys(&mut l.props[1], &[(s, y + comp_h * 0.4, Ease::Out), (s + d, y, Ease::Smooth)]);
            put_keys(&mut l.props[4], &[(s, 0.0, Ease::Smooth), (s + d, op, Ease::Smooth)]);
        }
        5 => put_keys(&mut l.props[1], &[(s, y - comp_h * 0.6, Ease::Bounce), (s + d * 1.5, y, Ease::Smooth)]),
        6 => {
            put_keys(&mut l.props[3], &[(s, rot - 180.0, Ease::Out), (s + d, rot, Ease::Smooth)]);
            put_keys(&mut l.props[2], &[(s, 0.0, Ease::Back), (s + d, sc, Ease::Smooth)]);
        }
        7 => {
            let o = at(l, 4, e - d);
            put_keys(&mut l.props[4], &[(e - d, o, Ease::Smooth), (e, 0.0, Ease::Smooth)]);
        }
        _ => {
            for i in [0, 1] {
                l.props[i].wiggle = 8.0;
                l.props[i].wiggle_freq = 6.0;
            }
        }
    }
}

#[cfg(test)]
mod preset_tests {
    use super::*;

    #[test]
    fn presets_animate_from_current_values() {
        let mut l = Layer::new("t".into(), Content::text("Hi", 40.0, [255; 4], true), 300.0, 200.0, 4.0);
        apply_anim_preset(&mut l, 1, 1000.0, 500.0);
        assert_eq!(l.props[2].keys.len(), 2);
        assert!(l.props[2].at(l.start) < 1.0 && (l.props[2].at(l.start + 0.6) - 100.0).abs() < 0.5);
        apply_anim_preset(&mut l, 2, 1000.0, 500.0);
        assert!((l.props[0].at(l.start) - (300.0 - 600.0)).abs() < 0.5);
        apply_anim_preset(&mut l, 7, 1000.0, 500.0);
        assert!(l.props[4].at(l.end) < 1.0);
        apply_anim_preset(&mut l, 8, 1000.0, 500.0);
        assert_eq!(l.props[0].wiggle, 8.0);
    }
}
