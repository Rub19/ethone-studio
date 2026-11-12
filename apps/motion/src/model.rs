use serde::{Deserialize, Serialize};
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
                    2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * std::f32::consts::TAU / 3.0).sin() + 1.0
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
    pub const ALL: [(Ease2, &'static str); 5] = [(Ease2::Expo, "Expo"), (Ease2::Circ, "Circulaire"), (Ease2::Anticipate, "Anticipation"), (Ease2::Steps, "Saccadé"), (Ease2::Sine, "Sinus")];
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
            Ease2::Circ => (1.0 - (t - 1.0).powi(2)).max(0.0).sqrt(),
            Ease2::Anticipate => {
                let c = 1.70158;
                (c + 1.0) * t * t * t - c * t * t
            }
            Ease2::Steps => (t * 4.0).floor() / 4.0,
            Ease2::Sine => -((std::f32::consts::PI * t).cos() - 1.0) / 2.0,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Key {
    pub t: f32,
    pub v: f32,
    pub ease: Ease,
    #[serde(default)]
    pub ease2: Ease2,
}

impl Key {
    pub fn curve(&self, u: f32) -> f32 {
        self.ease2.apply(u).unwrap_or_else(|| self.ease.apply(u))
    }
    pub fn ease_name(&self) -> &'static str {
        Ease2::ALL.iter().find(|(e, _)| *e == self.ease2).map(|x| x.1).unwrap_or_else(|| Ease::ALL.iter().find(|(e, _)| *e == self.ease).map_or("", |x| x.1))
    }
    pub fn set_ease(&mut self, e: Ease) {
        self.ease = e;
        self.ease2 = Ease2::None;
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prop {
    pub base: f32,
    pub keys: Vec<Key>,
}

impl Prop {
    pub fn new(v: f32) -> Self {
        Self { base: v, keys: Vec::new() }
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

    pub fn animated(&self) -> bool {
        !self.keys.is_empty()
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
        let (ease, ease2) = self.keys.first().map_or((Ease::Smooth, Ease2::None), |k| (k.ease, k.ease2));
        self.keys.retain(|k| (k.t - t).abs() >= 1e-3);
        let i = self.keys.partition_point(|k| k.t < t);
        self.keys.insert(i, Key { t, v, ease, ease2 });
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
                Key { t: a + b - src.t, v: src.v, ease: prev.ease, ease2: prev.ease2 }
            })
            .collect();
    }

    pub fn clear(&mut self, t: f32) {
        self.base = self.at(t);
        self.keys.clear();
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
    },
    Image { path: String, w: f32, h: f32 },
    Star { points: u32, radius: f32, inner: f32, color: [u8; 4] },
    Line { w: f32, thickness: f32, color: [u8; 4] },
    Counter { from: f32, to: f32, decimals: u32, size: f32, color: [u8; 4], prefix: String, suffix: String },
    Ring { radius: f32, thickness: f32, color: [u8; 4] },
    Arrow { w: f32, thickness: f32, color: [u8; 4] },
}

impl Content {
    pub fn text(text: &str, size: f32, color: [u8; 4], bold: bool) -> Self {
        Content::Text { text: text.into(), size, color, bold, family: "Segoe UI".into(), italic: false, tracking: 0.0, align: 0 }
    }
    pub fn styled(mut self, family: &str, italic: bool, tracking: f32, align: u8) -> Self {
        if let Content::Text { family: f, italic: i, tracking: tr, align: a, .. } = &mut self {
            (*f, *i, *tr, *a) = (family.into(), italic, tracking, align);
        }
        self
    }
}

pub const BLENDS: [(&str, &str); 10] = [
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
];

pub const MATTES: [&str; 3] = ["Aucun", "Alpha du calque du dessus", "Luminance du calque du dessus"];

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
        }
    }
}

pub const PROPS: [&str; 8] = ["Position X", "Position Y", "Échelle %", "Rotation °", "Opacité %", "Inclinaison °", "Flou px", "Révélation %"];
pub const PROP_DEFAULT: [f32; 8] = [0.0, 0.0, 100.0, 0.0, 100.0, 0.0, 0.0, 100.0];

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
}

impl Layer {
    pub fn new(name: String, content: Content, x: f32, y: f32, end: f32) -> Self {
        let mut props: Vec<Prop> = PROP_DEFAULT.iter().map(|v| Prop::new(*v)).collect();
        props[0].base = x;
        props[1].base = y;
        Self { name, content, props, start: 0.0, end, visible: true, fx: Fx::default(), locked: false, solo: false, label: None, parent: None }
    }

    pub fn normalize(&mut self) {
        while self.props.len() < PROPS.len() {
            self.props.push(Prop::new(PROP_DEFAULT[self.props.len()]));
        }
    }

    pub fn size(&self) -> (f32, f32) {
        match &self.content {
            Content::Rect { w, h, .. } | Content::Ellipse { w, h, .. } | Content::Image { w, h, .. } => (*w, *h),
            Content::Text { text, size, tracking, .. } => {
                let cols = text.lines().map(|l| l.chars().count()).max().unwrap_or(1) as f32;
                (cols * (size * 0.55 + tracking), text.lines().count().max(1) as f32 * size * 1.2)
            }
            Content::Star { radius, .. } => (radius * 2.0, radius * 2.0),
            Content::Ring { radius, thickness, .. } => (radius * 2.0 + thickness, radius * 2.0 + thickness),
            Content::Line { w, thickness, .. } => (*w, thickness.max(8.0)),
            Content::Arrow { w, thickness, .. } => (*w, thickness * 3.0),
            Content::Counter { to, from, decimals, size, prefix, suffix, .. } => {
                let n = format!("{prefix}{:.*}{suffix}", *decimals as usize, from.abs().max(to.abs())).chars().count() as f32;
                (n * size * 0.6, size * 1.2)
            }
        }
    }

    pub fn active(&self, t: f32) -> bool {
        self.visible && t >= self.start && t <= self.end
    }

    pub fn local_time(&self, t: f32) -> f32 {
        let last = self.props.iter().flat_map(|p| p.keys.last()).map(|k| k.t).fold(0.0f32, f32::max);
        let first = self.props.iter().flat_map(|p| p.keys.first()).map(|k| k.t).fold(f32::MAX, f32::min).min(last);
        let period = last - first;
        if period <= 1e-3 || t <= first {
            return t;
        }
        if self.fx.pingpong {
            let u = (t - first).rem_euclid(period * 2.0);
            first + if u > period { period * 2.0 - u } else { u }
        } else if self.fx.loop_keys {
            first + (t - first).rem_euclid(period)
        } else {
            t
        }
    }

    pub fn value(&self, pi: usize, t: f32) -> f32 {
        let tt = self.local_time(t);
        self.props.get(pi).map_or(PROP_DEFAULT[pi], |p| p.at(tt))
    }

    pub fn wiggle(&self, t: f32, seed: usize) -> (f32, f32) {
        if self.fx.wiggle <= 0.0 {
            return (0.0, 0.0);
        }
        let f = self.fx.wiggle_freq.max(0.1) * std::f32::consts::TAU;
        let ph = seed as f32 * 1.37;
        (
            self.fx.wiggle * ((t * f + ph).sin() * 0.6 + (t * f * 2.31 + ph * 3.1).sin() * 0.4),
            self.fx.wiggle * ((t * f * 1.13 + ph * 2.3).cos() * 0.6 + (t * f * 1.87 + ph).sin() * 0.4),
        )
    }

    pub fn key_times(&self) -> Vec<f32> {
        let mut times: Vec<f32> = self.props.iter().flat_map(|p| p.keys.iter().map(|k| k.t)).collect();
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
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub t: f32,
    pub name: String,
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

impl Comp {
    pub fn new() -> Self {
        Self { width: 1920, height: 1080, fps: 30, duration: 5.0, background: [16, 18, 26, 255], layers: Vec::new(), background2: None, markers: Vec::new(), work: None }
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

    /// Position, scale (×1) and rotation (°) of a layer in composition space, parents included.
    pub fn world(&self, li: usize, t: f32) -> (f32, f32, f32, f32) {
        self.world_depth(li, t, 0)
    }

    fn world_depth(&self, li: usize, t: f32, depth: u32) -> (f32, f32, f32, f32) {
        let l = &self.layers[li];
        let (wx, wy) = l.wiggle(t, li);
        let (x, y, s, r) = (l.value(0, t) + wx, l.value(1, t) + wy, l.value(2, t) / 100.0, l.value(3, t));
        match l.parent.filter(|p| *p < self.layers.len() && *p != li && depth < 8) {
            Some(p) => {
                let (px, py, ps, pr) = self.world_depth(p, t, depth + 1);
                let (rx, ry) = rotate(x * ps, y * ps, pr);
                (px + rx, py + ry, ps * s, pr + r)
            }
            None => (x, y, s, r),
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

    /// Attaches `li` to `parent` (or detaches it) without moving it on screen at time `t`.
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
                let (px, py, ps, pr) = self.world(p, t);
                let (lx, ly) = rotate(wx - px, wy - py, -pr);
                (lx / ps.max(1e-4), ly / ps.max(1e-4), ws / ps.max(1e-4), wr - pr)
            }
            None => (wx, wy, ws, wr),
        };
        let l = &mut self.layers[li];
        l.normalize();
        l.parent = parent;
        l.props[0].set(t, x - wwx);
        l.props[1].set(t, y - wwy);
        l.props[2].set(t, s * 100.0);
        l.props[3].set(t, r);
        true
    }

    pub fn next_key(&self, t: f32, forward: bool, only: Option<usize>) -> Option<f32> {
        let mut times: Vec<f32> = self.layers.iter().enumerate().filter(|(i, _)| only.is_none_or(|o| o == *i)).flat_map(|(_, l)| l.key_times()).chain(self.markers.iter().map(|m| m.t)).collect();
        times.sort_by(f32::total_cmp);
        if forward { times.into_iter().find(|k| *k > t + 1e-3) } else { times.into_iter().rev().find(|k| *k < t - 1e-3) }
    }

    fn own_xform(&self, li: usize, t: f32) -> String {
        let l = &self.layers[li];
        let (wx, wy) = l.wiggle(t, li);
        let (x, y) = (l.value(0, t) + wx, l.value(1, t) + wy);
        let mut s = format!("translate({x} {y}) rotate({})", l.value(3, t));
        let k = l.value(5, t);
        if k != 0.0 {
            write!(s, " skewX({k})").unwrap();
        }
        write!(s, " scale({})", l.value(2, t) / 100.0).unwrap();
        s
    }

    fn xform(&self, li: usize, t: f32) -> String {
        let mut chain = vec![self.own_xform(li, t)];
        let mut cur = self.layers[li].parent;
        let mut seen = 0;
        while let Some(p) = cur.filter(|p| *p < self.layers.len() && *p != li && seen < 8) {
            chain.push(self.own_xform(p, t));
            cur = self.layers[p].parent;
            seen += 1;
        }
        chain.reverse();
        chain.join(" ")
    }

    fn body(&self, li: usize, t: f32, uid: &str, defs: &mut String) -> String {
        let l = &self.layers[li];
        let fx = &l.fx;
        let reveal = l.value(7, t).clamp(0.0, 100.0);
        let fill_of = |c: [u8; 4], defs: &mut String| -> String {
            match fx.gradient {
                Some(g) => {
                    let (dx, dy) = rotate(0.0, 0.5, fx.gradient_angle);
                    write!(
                        defs,
                        r##"<linearGradient id="g{uid}" x1="{}" y1="{}" x2="{}" y2="{}"><stop offset="0" stop-color="{}" stop-opacity="{}"/><stop offset="1" stop-color="{}" stop-opacity="{}"/></linearGradient>"##,
                        0.5 - dx,
                        0.5 - dy,
                        0.5 + dx,
                        0.5 + dy,
                        hex(c),
                        c[3] as f32 / 255.0,
                        hex(g),
                        g[3] as f32 / 255.0
                    )
                    .unwrap();
                    format!(r#"fill="url(#g{uid})""#)
                }
                None => color(c),
            }
        };
        let stroke = match fx.stroke {
            Some(c) if fx.stroke_w > 0.0 => {
                let dash = if fx.dash > 0.0 { format!(r#" stroke-dasharray="{} {}""#, fx.dash, fx.dash * 0.6) } else { String::new() };
                format!(r#" stroke="{}" stroke-opacity="{}" stroke-width="{}" stroke-linejoin="round"{dash}"#, hex(c), c[3] as f32 / 255.0, fx.stroke_w)
            }
            _ => String::new(),
        };
        let mut body = String::new();
        let tt = l.local_time(t);
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
                let n = (*points).max(3);
                let star = *inner < 0.99;
                let count = if star { n * 2 } else { n };
                let pts: Vec<String> = (0..count)
                    .map(|k| {
                        let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::TAU / count as f32;
                        let r = if star && k % 2 == 1 { radius * inner } else { *radius };
                        format!("{:.2},{:.2}", a.cos() * r, a.sin() * r)
                    })
                    .collect();
                write!(body, r#"<polygon points="{}" {paint}{stroke}/>"#, pts.join(" ")).unwrap()
            }
            Content::Line { w, thickness, color: c } => {
                let paint = fill_of(*c, defs);
                write!(body, r#"<rect x="{}" y="{}" width="{w}" height="{thickness}" rx="{}" {paint}{stroke}/>"#, -w / 2.0, -thickness / 2.0, thickness / 2.0).unwrap()
            }
            Content::Arrow { w, thickness: th, color: c } => {
                let paint = fill_of(*c, defs);
                let (hw, head) = (w / 2.0, (th * 2.4).min(*w * 0.6));
                let pts = [(-hw, -th / 2.0), (hw - head, -th / 2.0), (hw - head, -th * 1.5), (hw, 0.0), (hw - head, th * 1.5), (hw - head, th / 2.0), (-hw, th / 2.0)];
                let pts: Vec<String> = pts.iter().map(|(x, y)| format!("{x:.2},{y:.2}")).collect();
                write!(body, r#"<polygon points="{}" {paint}{stroke}/>"#, pts.join(" ")).unwrap()
            }
            Content::Ring { radius, thickness, color: c } => {
                let sc = format!(r#"fill="none" stroke="{}" stroke-opacity="{}" stroke-width="{thickness}" stroke-linecap="round""#, hex(*c), c[3] as f32 / 255.0);
                if reveal >= 99.95 {
                    write!(body, r#"<circle r="{radius}" {sc}/>"#).unwrap();
                } else if reveal > 0.05 {
                    let a = (reveal / 100.0 * 360.0 - 90.0).to_radians();
                    let large = if reveal > 50.0 { 1 } else { 0 };
                    write!(body, r#"<path d="M 0 {} A {radius} {radius} 0 {large} 1 {:.3} {:.3}" {sc}/>"#, -radius, a.cos() * radius, a.sin() * radius).unwrap();
                }
            }
            Content::Text { text, size, color: c, bold, family, italic, tracking, align } => {
                let paint = fill_of(*c, defs);
                let shown: String = if fx.typewriter > 0.0 {
                    let total = text.chars().count();
                    let k = (((t - l.start) / fx.typewriter).clamp(0.0, 1.0) * total as f32).round() as usize;
                    text.chars().take(k).collect()
                } else {
                    text.clone()
                };
                let lines: Vec<&str> = text.lines().collect();
                let n = lines.len().max(1) as f32;
                let top = -(n - 1.0) * size * 1.2 / 2.0 + size * 0.35;
                let anchor = ["middle", "start", "end"][(*align as usize).min(2)];
                let extra = format!("{}{}", if *italic { r#" font-style="italic""# } else { "" }, if *tracking != 0.0 { format!(r#" letter-spacing="{tracking}""#) } else { String::new() });
                write!(body, r#"<text text-anchor="{anchor}" font-size="{size}" font-family="{}" font-weight="{}"{extra} {paint}{stroke}>"#, esc(family), if *bold { "bold" } else { "normal" }).unwrap();
                let mut left = shown.chars().count();
                for (i, line) in lines.iter().enumerate() {
                    let part: String = line.chars().take(left).collect();
                    left = left.saturating_sub(line.chars().count() + 1);
                    write!(body, r#"<tspan x="0" y="{}">{}</tspan>"#, top + i as f32 * size * 1.2, esc(&part)).unwrap();
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
        }
        if fx.flip_x || fx.flip_y {
            body = format!(r#"<g transform="scale({} {})">{body}</g>"#, if fx.flip_x { -1 } else { 1 }, if fx.flip_y { -1 } else { 1 });
        }
        let mut f = String::new();
        let mut last = "SourceGraphic".to_string();
        if fx.hue != 0.0 || (fx.saturation - 100.0).abs() > 0.01 {
            write!(f, r#"<feColorMatrix in="{last}" type="hueRotate" values="{}" result="h"/><feColorMatrix in="h" type="saturate" values="{}" result="hs"/>"#, fx.hue, fx.saturation / 100.0).unwrap();
            last = "hs".into();
        }
        if (fx.brightness - 100.0).abs() > 0.01 {
            let k = fx.brightness / 100.0;
            write!(f, r#"<feComponentTransfer in="{last}" result="br"><feFuncR type="linear" slope="{k}"/><feFuncG type="linear" slope="{k}"/><feFuncB type="linear" slope="{k}"/></feComponentTransfer>"#).unwrap();
            last = "br".into();
        }
        let blur = fx.blur + l.value(6, t).max(0.0);
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
        if !f.is_empty() {
            write!(defs, r#"<filter id="f{uid}" x="-60%" y="-60%" width="220%" height="220%">{f}</filter>"#).unwrap();
            body = format!(r#"<g filter="url(#f{uid})">{body}</g>"#);
        }
        if reveal < 99.95 && !matches!(l.content, Content::Ring { .. }) {
            let (w, h) = l.size();
            let pad = 40.0 + blur * 3.0 + fx.glow * 3.0;
            write!(defs, r#"<clipPath id="c{uid}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath>"#, -w / 2.0 - pad, -h - pad, (w + pad * 2.0) * reveal / 100.0, h * 2.0 + pad * 2.0).unwrap();
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
        let op = (l.value(4, t) / 100.0 * fade).clamp(0.0, 1.0);
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
        for (li, l) in self.layers.iter().enumerate() {
            if !l.active(t) || (solo && !l.solo) {
                continue;
            }
            if li > 0 && self.layers[li - 1].fx.matte > 0 {
                continue;
            }
            let mut defs = String::new();
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
                let kind = if fx.matte == 1 { "alpha" } else { "luminance" };
                write!(defs, r#"<mask id="m{li}" mask-type="{kind}" maskUnits="userSpaceOnUse" x="-100000" y="-100000" width="200000" height="200000">{content}</mask>"#).unwrap();
                g = format!(r#"<g mask="url(#m{li})">{g}</g>"#);
            }
            if !defs.is_empty() {
                writeln!(s, "<defs>{defs}</defs>").unwrap();
            }
            s.push_str(&g);
            s.push('\n');
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
        let old = r#"{"width":100,"height":100,"fps":30,"duration":2.0,"background":[0,0,0,255],"layers":[{"name":"a","content":{"Text":{"text":"x","size":10.0,"color":[1,2,3,255],"bold":true,"family":"Arial"}},"props":[{"base":1.0,"keys":[{"t":0.0,"v":1.0,"ease":"Smooth"}]},{"base":2.0,"keys":[]},{"base":100.0,"keys":[]},{"base":0.0,"keys":[]},{"base":100.0,"keys":[]}],"start":0.0,"end":2.0,"visible":true,"fx":{"blur":0.0,"shadow":true}}]}"#;
        let mut c: Comp = serde_json::from_str(old).unwrap();
        assert_eq!(c.layers[0].fx.shadow_dy, 10.0);
        assert_eq!(c.layers[0].value(7, 0.0), 100.0);
        c.normalize();
        assert_eq!(c.layers[0].props.len(), PROPS.len());
        parse(&c.to_svg(1.0));
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
        l.props[0].reverse(1.0, 2.0);
        assert_eq!(l.props[0].keys[0].v, 10.0);
        l.shift(1.0);
        assert_eq!(l.props[0].keys[0].t, 2.0);
        assert_eq!(l.start, 1.0);
    }
}
