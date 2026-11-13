use crate::filters::{Kind, hash};
use image::{Luma, Rgba, RgbaImage, imageops};

#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub img: RgbaImage,
    pub visible: bool,
    pub opacity: f32,
    pub blend: Blend,
    pub locked: bool,
    pub lock_alpha: bool,
    pub lock_pos: bool,
    pub clip: bool,
    pub tag: u8,
    pub mask: Option<RgbaImage>,
    pub mask_on: bool,
    pub adjust: Option<(Kind, Vec<f32>)>,
}

impl Layer {
    pub fn new(name: impl Into<String>, img: RgbaImage) -> Self {
        Self { name: name.into(), img, visible: true, opacity: 1.0, blend: Blend::Normal, locked: false, lock_alpha: false, lock_pos: false, clip: false, tag: 0, mask: None, mask_on: true, adjust: None }
    }

    fn mask_at(&self, x: u32, y: u32) -> f32 {
        match (&self.mask, self.mask_on) {
            (Some(m), true) => {
                let p = m.get_pixel(x, y).0;
                p[0] as f32 / 255.0 * p[3] as f32 / 255.0
            }
            _ => 1.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Blend {
    Normal,
    Dissolve,
    Darken,
    Multiply,
    ColorBurn,
    LinearBurn,
    DarkerColor,
    Lighten,
    Screen,
    ColorDodge,
    Add,
    LighterColor,
    Overlay,
    SoftLight,
    HardLight,
    VividLight,
    LinearLight,
    PinLight,
    HardMix,
    Difference,
    Exclusion,
    Subtract,
    Divide,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

pub const BLENDS: [(Blend, &str); 27] = [
    (Blend::Normal, "Normal"),
    (Blend::Dissolve, "Fondu"),
    (Blend::Darken, "Obscurcir"),
    (Blend::Multiply, "Produit"),
    (Blend::ColorBurn, "Densité couleur +"),
    (Blend::LinearBurn, "Densité linéaire +"),
    (Blend::DarkerColor, "Couleur plus foncée"),
    (Blend::Lighten, "Éclaircir"),
    (Blend::Screen, "Superposition"),
    (Blend::ColorDodge, "Densité couleur −"),
    (Blend::Add, "Densité linéaire − (ajout)"),
    (Blend::LighterColor, "Couleur plus claire"),
    (Blend::Overlay, "Incrustation"),
    (Blend::SoftLight, "Lumière tamisée"),
    (Blend::HardLight, "Lumière crue"),
    (Blend::VividLight, "Lumière vive"),
    (Blend::LinearLight, "Lumière linéaire"),
    (Blend::PinLight, "Lumière ponctuelle"),
    (Blend::HardMix, "Mélange maximal"),
    (Blend::Difference, "Différence"),
    (Blend::Exclusion, "Exclusion"),
    (Blend::Subtract, "Soustraction"),
    (Blend::Divide, "Division"),
    (Blend::Hue, "Teinte"),
    (Blend::Saturation, "Saturation"),
    (Blend::Color, "Couleur"),
    (Blend::Luminosity, "Luminosité"),
];

pub fn blend_name(b: Blend) -> &'static str {
    BLENDS.iter().find(|x| x.0 == b).map_or("Normal", |x| x.1)
}

fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut o = c;
    if n < 0.0 {
        o = o.map(|v| l + (v - l) * l / (l - n).max(1e-6));
    }
    if x > 1.0 {
        o = o.map(|v| l + (v - l) * (1.0 - l) / (x - l).max(1e-6));
    }
    o
}

fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color(c.map(|v| v + d))
}

fn sat(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let mx = c[0].max(c[1]).max(c[2]);
    let mn = c[0].min(c[1]).min(c[2]);
    if mx - mn <= 1e-6 {
        return [0.0; 3];
    }
    c.map(|v| (v - mn) * s / (mx - mn))
}

impl Blend {
    fn one(self, b: f32, s: f32) -> f32 {
        match self {
            Blend::Darken => b.min(s),
            Blend::Multiply => b * s,
            Blend::ColorBurn => if b >= 1.0 { 1.0 } else if s <= 0.0 { 0.0 } else { 1.0 - ((1.0 - b) / s).min(1.0) },
            Blend::LinearBurn => (b + s - 1.0).max(0.0),
            Blend::Lighten => b.max(s),
            Blend::Screen => 1.0 - (1.0 - b) * (1.0 - s),
            Blend::ColorDodge => if b <= 0.0 { 0.0 } else if s >= 1.0 { 1.0 } else { (b / (1.0 - s)).min(1.0) },
            Blend::Add => (b + s).min(1.0),
            Blend::Overlay => Blend::HardLight.one(s, b),
            Blend::SoftLight => (1.0 - 2.0 * s) * b * b + 2.0 * s * b,
            Blend::HardLight => if s < 0.5 { 2.0 * b * s } else { 1.0 - 2.0 * (1.0 - b) * (1.0 - s) },
            Blend::VividLight => if s < 0.5 { Blend::ColorBurn.one(b, 2.0 * s) } else { Blend::ColorDodge.one(b, 2.0 * (s - 0.5)) },
            Blend::LinearLight => (b + 2.0 * s - 1.0).clamp(0.0, 1.0),
            Blend::PinLight => if s < 0.5 { b.min(2.0 * s) } else { b.max(2.0 * (s - 0.5)) },
            Blend::HardMix => if Blend::VividLight.one(b, s) < 0.5 { 0.0 } else { 1.0 },
            Blend::Difference => (b - s).abs(),
            Blend::Exclusion => b + s - 2.0 * b * s,
            Blend::Subtract => (b - s).max(0.0),
            Blend::Divide => if s <= 0.0 { 1.0 } else { (b / s).min(1.0) },
            _ => s,
        }
    }

    pub fn mix3(self, b: [f32; 3], s: [f32; 3]) -> [f32; 3] {
        match self {
            Blend::Normal | Blend::Dissolve => s,
            Blend::DarkerColor => if lum(s) < lum(b) { s } else { b },
            Blend::LighterColor => if lum(s) > lum(b) { s } else { b },
            Blend::Hue => set_lum(set_sat(s, sat(b)), lum(b)),
            Blend::Saturation => set_lum(set_sat(b, sat(s)), lum(b)),
            Blend::Color => set_lum(s, lum(b)),
            Blend::Luminosity => set_lum(b, lum(s)),
            m => std::array::from_fn(|k| m.one(b[k], s[k])),
        }
    }
}

pub enum Paint {
    Color([u8; 4]),
    Pencil([u8; 4]),
    Erase,
    Dodge(u8),
    Burn(u8),
    Sponge(f32),
    Blur(RgbaImage),
    Sharpen(RgbaImage),
    Smudge(Option<[f32; 4]>),
    Clone { src: RgbaImage, dx: f32, dy: f32 },
    Heal { src: RgbaImage, dx: f32, dy: f32 },
    Replace([u8; 4]),
    Mixer { color: [u8; 4], carry: Option<[f32; 4]>, wet: f32 },
    BgErase { tolerance: f32, target: Option<[f32; 3]>, continuous: bool },
}

pub fn inside(sel: Option<Area>, mask: Option<&Vec<u8>>, w: u32, x: u32, y: u32) -> bool {
    weight(sel, mask, w, x, y) > 0.5
}

pub fn weight(sel: Option<Area>, mask: Option<&Vec<u8>>, w: u32, x: u32, y: u32) -> f32 {
    if sel.is_some_and(|s| !s.contains(x, y)) {
        return 0.0;
    }
    mask.map_or(1.0, |m| m[(y * w + x) as usize] as f32 / 255.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl Area {
    fn union(self, o: Area) -> Area {
        Area { x0: self.x0.min(o.x0), y0: self.y0.min(o.y0), x1: self.x1.max(o.x1), y1: self.y1.max(o.y1) }
    }
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }
    pub fn from_points(a: (f32, f32), b: (f32, f32), w: u32, h: u32) -> Option<Area> {
        let x0 = a.0.min(b.0).floor().clamp(0.0, w as f32) as u32;
        let x1 = a.0.max(b.0).ceil().clamp(0.0, w as f32) as u32;
        let y0 = a.1.min(b.1).floor().clamp(0.0, h as f32) as u32;
        let y1 = a.1.max(b.1).ceil().clamp(0.0, h as f32) as u32;
        (x1 > x0 && y1 > y0).then_some(Area { x0, y0, x1, y1 })
    }
    pub fn width(&self) -> u32 {
        self.x1 - self.x0
    }
    pub fn height(&self) -> u32 {
        self.y1 - self.y0
    }
}

#[derive(Clone)]
struct Snapshot {
    layers: Vec<Layer>,
    active: usize,
    label: String,
}

#[derive(Clone, Copy)]
pub struct Brush {
    pub radius: f32,
    pub hardness: f32,
    pub flow: f32,
    pub spacing: f32,
    pub angle: f32,
    pub roundness: f32,
    pub size_jitter: f32,
    pub flow_jitter: f32,
    pub scatter: f32,
    pub mode: Blend,
}

impl Brush {
    pub fn new(radius: f32, hardness: f32, flow: f32) -> Self {
        Self { radius, hardness, flow, spacing: 0.125, angle: 0.0, roundness: 1.0, size_jitter: 0.0, flow_jitter: 0.0, scatter: 0.0, mode: Blend::Normal }
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Xform {
    pub sx: f32,
    pub sy: f32,
    pub angle: f32,
    pub skew_x: f32,
    pub skew_y: f32,
    pub tx: f32,
    pub ty: f32,
    pub persp_x: f32,
    pub persp_y: f32,
    pub nearest: bool,
}

impl Xform {
    pub fn identity() -> Self {
        Self { sx: 1.0, sy: 1.0, ..Default::default() }
    }

    fn matrix(&self, w: f32, h: f32) -> [[f32; 3]; 3] {
        let (cx, cy) = (w / 2.0, h / 2.0);
        let (sn, cs) = self.angle.to_radians().sin_cos();
        let (kx, ky) = (self.skew_x.to_radians().tan(), self.skew_y.to_radians().tan());
        let a = mul(&[[cs, -sn, 0.0], [sn, cs, 0.0], [0.0, 0.0, 1.0]], &mul(&[[1.0, kx, 0.0], [ky, 1.0, 0.0], [0.0, 0.0, 1.0]], &[[self.sx, 0.0, 0.0], [0.0, self.sy, 0.0], [0.0, 0.0, 1.0]]));
        let persp = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [self.persp_x / 100.0 / cx.max(1.0), self.persp_y / 100.0 / cy.max(1.0), 1.0]];
        let m = mul(&persp, &a);
        mul(&[[1.0, 0.0, cx + self.tx], [0.0, 1.0, cy + self.ty], [0.0, 0.0, 1.0]], &mul(&m, &[[1.0, 0.0, -cx], [0.0, 1.0, -cy], [0.0, 0.0, 1.0]]))
    }
}

fn mul(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

fn invert(m: &[[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-9 {
        return None;
    }
    let c = |r0: usize, c0: usize, r1: usize, c1: usize| m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0];
    Some([
        [c(1, 1, 2, 2) / det, -c(0, 1, 2, 2) / det, c(0, 1, 1, 2) / det],
        [-c(1, 0, 2, 2) / det, c(0, 0, 2, 2) / det, -c(0, 0, 1, 2) / det],
        [c(1, 0, 2, 1) / det, -c(0, 0, 2, 1) / det, c(0, 0, 1, 1) / det],
    ])
}

fn warp(src: &RgbaImage, out_w: u32, out_h: u32, inv: &[[f32; 3]; 3], nearest: bool) -> RgbaImage {
    let (w, h) = src.dimensions();
    RgbaImage::from_fn(out_w, out_h, |x, y| {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let z = inv[2][0] * fx + inv[2][1] * fy + inv[2][2];
        if z.abs() < 1e-6 {
            return Rgba([0, 0, 0, 0]);
        }
        let sx = (inv[0][0] * fx + inv[0][1] * fy + inv[0][2]) / z;
        let sy = (inv[1][0] * fx + inv[1][1] * fy + inv[1][2]) / z;
        if sx < 0.0 || sy < 0.0 || sx >= w as f32 || sy >= h as f32 {
            return Rgba([0, 0, 0, 0]);
        }
        if nearest { *src.get_pixel(sx as u32, sy as u32) } else { bilinear(src, sx - 0.5, sy - 0.5) }
    })
}

pub struct Document {
    pub layers: Vec<Layer>,
    pub active: usize,
    pub dirty: Option<Area>,
    pub selection: Option<Area>,
    pub mask: Option<Vec<u8>>,
    pub version: u64,
    pub modified: bool,
    pub name: String,
    pub edit_mask: bool,
    pub next_label: Option<String>,
    pub undo_limit: usize,
    stamps: u32,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

fn target(layers: &mut [Layer], active: usize, edit_mask: bool) -> (&mut RgbaImage, bool) {
    let l = &mut layers[active];
    let lock = l.lock_alpha;
    match (&mut l.mask, edit_mask) {
        (Some(m), true) => (m, false),
        _ => (&mut l.img, lock),
    }
}

impl Document {
    pub fn new(w: u32, h: u32, fill: Option<[u8; 4]>) -> Self {
        let img = RgbaImage::from_pixel(w, h, Rgba(fill.unwrap_or([0, 0, 0, 0])));
        Self::with_layer("Sans titre".into(), "Fond".into(), img)
    }

    pub fn from_image(name: String, img: RgbaImage) -> Self {
        Self::with_layer(name, "Fond".into(), img)
    }

    fn with_layer(name: String, layer: String, img: RgbaImage) -> Self {
        let mut d = Self {
            layers: vec![Layer::new(layer, img)],
            active: 0,
            dirty: None,
            selection: None,
            mask: None,
            version: 0,
            modified: false,
            name,
            edit_mask: false,
            next_label: None,
            undo_limit: 40,
            stamps: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        };
        d.touch_all();
        d
    }

    pub fn width(&self) -> u32 {
        self.layers[0].img.width()
    }
    pub fn height(&self) -> u32 {
        self.layers[0].img.height()
    }

    pub fn full(&self) -> Area {
        Area { x0: 0, y0: 0, x1: self.width(), y1: self.height() }
    }

    pub fn touch_all(&mut self) {
        self.version += 1;
        self.dirty = Some(self.full());
    }

    pub fn touch(&mut self, a: Area) {
        self.version += 1;
        self.dirty = Some(self.dirty.map_or(a, |d| d.union(a)));
    }

    pub fn checkpoint(&mut self) {
        // ponytail: full-layer snapshots, switch to per-tile diffs if memory gets tight on big canvases
        let label = self.next_label.take().unwrap_or_else(|| "Modification".into());
        self.undo.push(Snapshot { layers: self.layers.clone(), active: self.active, label });
        while self.undo.len() > self.undo_limit.max(1) {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.modified = true;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(s) = self.undo.pop() {
            let cur = Snapshot { layers: std::mem::replace(&mut self.layers, s.layers), active: self.active, label: s.label };
            self.redo.push(cur);
            self.active = s.active.min(self.layers.len() - 1);
            self.touch_all();
        }
    }

    pub fn redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            let cur = Snapshot { layers: std::mem::replace(&mut self.layers, s.layers), active: self.active, label: s.label };
            self.undo.push(cur);
            self.active = s.active.min(self.layers.len() - 1);
            self.touch_all();
        }
    }

    pub fn history(&self) -> (Vec<String>, usize) {
        let mut rows = vec![if self.undo.len() >= self.undo_limit { "Début de l'historique".to_string() } else { "Ouverture".to_string() }];
        rows.extend(self.undo.iter().map(|s| s.label.clone()));
        rows.extend(self.redo.iter().rev().map(|s| s.label.clone()));
        (rows, self.undo.len())
    }

    pub fn jump_history(&mut self, to: usize) {
        while self.undo.len() > to && self.can_undo() {
            self.undo();
        }
        while self.undo.len() < to && self.can_redo() {
            self.redo();
        }
    }

    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn composite_area(&self, out: &mut RgbaImage, a: Area) {
        let idx: Vec<usize> = (0..self.layers.len()).collect();
        let acc = self.stack(&idx, a);
        let aw = a.width() as usize;
        for (i, c) in acc.iter().enumerate() {
            out.put_pixel(a.x0 + (i % aw) as u32, a.y0 + (i / aw) as u32, Rgba(to_u8(*c)));
        }
    }

    fn stack(&self, idx: &[usize], a: Area) -> Vec<[f32; 4]> {
        let (aw, ah) = (a.width() as usize, a.height() as usize);
        let mut acc = vec![[0f32; 4]; aw * ah];
        let mut base = vec![1f32; aw * ah];
        for &li in idx {
            let l = &self.layers[li];
            if !l.visible || l.opacity <= 0.0 {
                continue;
            }
            if let Some((k, vals)) = &l.adjust {
                let mut tmp = RgbaImage::new(aw as u32, ah as u32);
                for (p, c) in tmp.pixels_mut().zip(&acc) {
                    p.0 = to_u8(*c);
                }
                let out = k.apply(&tmp, vals);
                for (i, c) in acc.iter_mut().enumerate() {
                    let (x, y) = (a.x0 + (i % aw) as u32, a.y0 + (i / aw) as u32);
                    let mut m = l.opacity * l.mask_at(x, y);
                    if l.clip {
                        m *= base[i];
                    }
                    let o = out.as_raw()[i * 4..i * 4 + 3].iter().map(|v| *v as f32 / 255.0).collect::<Vec<_>>();
                    let mixed = l.blend.mix3([c[0], c[1], c[2]], [o[0], o[1], o[2]]);
                    for k in 0..3 {
                        c[k] += (mixed[k] - c[k]) * m;
                    }
                }
                continue;
            }
            for (i, c) in acc.iter_mut().enumerate() {
                let (x, y) = (a.x0 + (i % aw) as u32, a.y0 + (i / aw) as u32);
                let mut p = l.img.get_pixel(x, y).0;
                let mut alpha = p[3] as f32 / 255.0 * l.mask_at(x, y);
                if l.clip {
                    alpha *= base[i];
                } else {
                    base[i] = alpha;
                }
                p[3] = (alpha * 255.0).round() as u8;
                blend_over(c, p, l.opacity, l.blend, x, y);
            }
        }
        acc
    }

    pub fn flatten(&self) -> RgbaImage {
        let mut out = RgbaImage::new(self.width(), self.height());
        self.composite_area(&mut out, self.full());
        out
    }

    fn baked(&self, idx: &[usize]) -> RgbaImage {
        let a = self.full();
        let acc = self.stack(idx, a);
        let mut out = RgbaImage::new(self.width(), self.height());
        for (p, c) in out.pixels_mut().zip(acc) {
            p.0 = to_u8(c);
        }
        out
    }

    pub fn stamp(&mut self, cx: f32, cy: f32, brush: &Brush, color: [u8; 4], erase: bool) {
        self.stamp_paint(cx, cy, brush, &mut if erase { Paint::Erase } else { Paint::Color(color) });
    }

    pub fn stroke(&mut self, from: (f32, f32), to: (f32, f32), brush: &Brush, color: [u8; 4], erase: bool) {
        self.stroke_paint(from, to, brush, &mut if erase { Paint::Erase } else { Paint::Color(color) });
    }

    pub fn stroke_paint(&mut self, from: (f32, f32), to: (f32, f32), brush: &Brush, paint: &mut Paint) {
        let dist = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
        let step = (brush.radius * 2.0 * brush.spacing).max(0.5);
        let n = (dist / step).ceil().max(1.0) as usize;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            self.stamp_paint(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t, brush, paint);
        }
    }

    pub fn stamp_paint(&mut self, cx: f32, cy: f32, brush: &Brush, paint: &mut Paint) {
        self.stamps = self.stamps.wrapping_add(1);
        let n = self.stamps;
        let (w, h) = (self.width() as i32, self.height() as i32);
        let r = (brush.radius * (1.0 - brush.size_jitter * hash(n, 1, 3))).max(0.5);
        let (cx, cy) = if brush.scatter > 0.0 { (cx + (hash(n, 2, 3) - 0.5) * 2.0 * brush.scatter * brush.radius * 2.0, cy + (hash(n, 3, 3) - 0.5) * 2.0 * brush.scatter * brush.radius * 2.0) } else { (cx, cy) };
        let s = self.selection.unwrap_or(self.full());
        let x0 = ((cx - r - 1.0).floor() as i32).clamp(s.x0 as i32, s.x1 as i32).clamp(0, w);
        let x1 = ((cx + r + 1.0).ceil() as i32).clamp(s.x0 as i32, s.x1 as i32).clamp(0, w);
        let y0 = ((cy - r - 1.0).floor() as i32).clamp(s.y0 as i32, s.y1 as i32).clamp(0, h);
        let y1 = ((cy + r + 1.0).ceil() as i32).clamp(s.y0 as i32, s.y1 as i32).clamp(0, h);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let hard = brush.hardness.clamp(0.0, 0.99);
        let flow = brush.flow * (1.0 - brush.flow_jitter * hash(n, 4, 3));
        let base = match paint {
            Paint::Color(c) | Paint::Pencil(c) | Paint::Replace(c) => c[3] as f32 / 255.0,
            _ => 1.0,
        } * flow;
        let (sn, cs) = brush.angle.to_radians().sin_cos();
        let round = brush.roundness.clamp(0.05, 1.0);
        let mode = brush.mode;
        let (sel, mask, wu) = (self.selection, self.mask.as_ref(), w as u32);
        let (img, lock_alpha) = target(&mut self.layers, self.active, self.edit_mask);
        let center = img.get_pixel(cx.clamp(0.0, w as f32 - 1.0) as u32, cy.clamp(0.0, h as f32 - 1.0) as u32).0.map(|v| v as f32 / 255.0);
        match paint {
            Paint::Smudge(carry) | Paint::Mixer { carry, .. } if carry.is_none() => *carry = Some(center),
            Paint::BgErase { target, continuous, .. } if target.is_none() || *continuous => *target = Some([center[0], center[1], center[2]]),
            _ => {}
        }
        let mut picked = [0f32; 4];
        let mut picked_n = 0.0;
        for y in y0..y1 {
            for x in x0..x1 {
                let wsel = weight(sel, mask, wu, x as u32, y as u32);
                if wsel <= 0.0 {
                    continue;
                }
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let (u, v) = (dx * cs + dy * sn, (-dx * sn + dy * cs) / round);
                let d = (u * u + v * v).sqrt();
                let edge = (r + 0.5 - d).clamp(0.0, 1.0);
                if edge <= 0.0 {
                    continue;
                }
                let t = d / r;
                let soft = if t <= hard { 1.0 } else { smooth(1.0 - (t - hard) / (1.0 - hard)) };
                let a = base * edge * soft * wsel;
                let px = img.get_pixel_mut(x as u32, y as u32);
                let mut c = px.0.map(|v| v as f32 / 255.0);
                let old_a = c[3];
                match paint {
                    Paint::Color(col) => {
                        let src = [col[0] as f32 / 255.0, col[1] as f32 / 255.0, col[2] as f32 / 255.0];
                        let m = if mode == Blend::Normal || c[3] <= 0.0 { src } else { mode.mix3([c[0], c[1], c[2]], src) };
                        over(&mut c, to_u8([m[0], m[1], m[2], 1.0]), a);
                    }
                    Paint::Pencil(col) => {
                        if d <= r {
                            over(&mut c, [col[0], col[1], col[2], 255], base * wsel);
                        }
                    }
                    Paint::Erase => c[3] *= 1.0 - a,
                    Paint::Dodge(range) | Paint::Burn(range) => {
                        let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                        let wr = match range {
                            0 => (1.0 - l).powi(2),
                            2 => l * l,
                            _ => 1.0 - (l - 0.5).abs() * 2.0,
                        } * 2.0;
                        if matches!(paint, Paint::Dodge(_)) {
                            (0..3).for_each(|k| c[k] += (1.0 - c[k]) * a * 0.12 * wr);
                        } else {
                            (0..3).for_each(|k| c[k] *= 1.0 - a * 0.12 * wr);
                        }
                    }
                    Paint::Sponge(dir) => {
                        let (hh, ss, ll) = rgb_to_hsl(c[0], c[1], c[2]);
                        let (r2, g2, b2) = hsl_to_rgb(hh, (ss + *dir * a * 0.08).clamp(0.0, 1.0), ll);
                        (c[0], c[1], c[2]) = (r2, g2, b2);
                    }
                    Paint::Blur(_) | Paint::Sharpen(_) => {
                        let sharpen = matches!(paint, Paint::Sharpen(_));
                        let src = match paint {
                            Paint::Blur(s) | Paint::Sharpen(s) => &*s,
                            _ => unreachable!(),
                        };
                        let avg = local_mean(src, x, y, 2);
                        let orig = src.get_pixel(x as u32, y as u32).0.map(|v| v as f32 / 255.0);
                        for k in 0..4 {
                            let target = if sharpen { orig[k] + (orig[k] - avg[k]) * 1.2 } else { avg[k] };
                            c[k] += (target - c[k]) * a * 0.5;
                        }
                    }
                    Paint::Smudge(Some(carry)) => {
                        let old = c;
                        (0..4).for_each(|k| c[k] += (carry[k] - c[k]) * a * 0.6);
                        (0..4).for_each(|k| picked[k] += old[k] * a);
                        picked_n += a;
                    }
                    Paint::Mixer { color, carry: Some(carry), wet } => {
                        let old = c;
                        let load = 1.0 - *wet;
                        let paint_c: [f32; 4] = std::array::from_fn(|k| carry[k] + (color[k] as f32 / 255.0 - carry[k]) * load);
                        (0..4).for_each(|k| c[k] += (paint_c[k] - c[k]) * a * 0.6);
                        (0..4).for_each(|k| picked[k] += old[k] * a);
                        picked_n += a;
                    }
                    Paint::Smudge(None) | Paint::Mixer { carry: None, .. } => {}
                    Paint::Clone { src, dx, dy } => {
                        let sx = (x as f32 + *dx).round() as i32;
                        let sy = (y as f32 + *dy).round() as i32;
                        if sx >= 0 && sy >= 0 && sx < w && sy < h {
                            let q = src.get_pixel(sx as u32, sy as u32).0;
                            (0..4).for_each(|k| c[k] += (q[k] as f32 / 255.0 - c[k]) * a);
                        }
                    }
                    Paint::Heal { src, dx, dy } => {
                        let sx = (x as f32 + *dx).round() as i32;
                        let sy = (y as f32 + *dy).round() as i32;
                        if sx >= 0 && sy >= 0 && sx < w && sy < h {
                            let q = src.get_pixel(sx as u32, sy as u32).0.map(|v| v as f32 / 255.0);
                            let (ms, md) = (local_mean(src, sx, sy, 3), local_mean(src, x, y, 3));
                            (0..3).for_each(|k| c[k] += (q[k] + md[k] - ms[k] - c[k]) * a);
                        }
                    }
                    Paint::Replace(col) => {
                        let (hh, ss, _) = rgb_to_hsl(col[0] as f32 / 255.0, col[1] as f32 / 255.0, col[2] as f32 / 255.0);
                        let (_, _, ll) = rgb_to_hsl(c[0], c[1], c[2]);
                        let (r2, g2, b2) = hsl_to_rgb(hh, ss, ll);
                        c[0] += (r2 - c[0]) * a;
                        c[1] += (g2 - c[1]) * a;
                        c[2] += (b2 - c[2]) * a;
                    }
                    Paint::BgErase { tolerance, target: Some(t), .. } => {
                        let dist = ((c[0] - t[0]).abs() + (c[1] - t[1]).abs() + (c[2] - t[2]).abs()) / 3.0;
                        let wk = (1.0 - dist / tolerance.max(1e-3)).clamp(0.0, 1.0);
                        c[3] *= 1.0 - a * wk;
                    }
                    Paint::BgErase { target: None, .. } => {}
                }
                if lock_alpha {
                    c[3] = old_a;
                }
                px.0 = to_u8(c);
            }
        }
        if let Paint::Smudge(Some(carry)) | Paint::Mixer { carry: Some(carry), .. } = paint
            && picked_n > 0.0
        {
            (0..4).for_each(|k| carry[k] = carry[k] * 0.7 + picked[k] / picked_n * 0.3);
        }
        self.touch(Area { x0: x0 as u32, y0: y0 as u32, x1: x1 as u32, y1: y1 as u32 });
    }

    pub fn set_selection(&mut self, area: Option<Area>, mask: Option<Vec<u8>>) {
        self.mask = if area.is_some() { mask } else { None };
        self.selection = area;
        self.version += 1;
    }

    pub fn selected(&self, x: u32, y: u32) -> bool {
        inside(self.selection, self.mask.as_ref(), self.width(), x, y)
    }

    pub fn sel_weight(&self, x: u32, y: u32) -> f32 {
        weight(self.selection, self.mask.as_ref(), self.width(), x, y)
    }

    pub fn select_mask(&mut self, mask: Vec<u8>) {
        let w = self.width();
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for (i, _) in mask.iter().enumerate().filter(|m| *m.1 > 0) {
            let (x, y) = (i as u32 % w, i as u32 / w);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        }
        if x1 > x0 {
            self.set_selection(Some(Area { x0, y0, x1, y1 }), Some(mask));
        } else {
            self.set_selection(None, None);
        }
    }

    pub fn sel_mask(&self) -> Vec<u8> {
        let w = self.width();
        (0..w * self.height()).map(|i| if self.selection.is_some() { (self.sel_weight(i % w, i / w) * 255.0).round() as u8 } else { 0 }).collect()
    }

    pub fn combine_selection(&mut self, prev: Option<Vec<u8>>, mode: u8) {
        let Some(prev) = prev else { return };
        if mode == 0 {
            return;
        }
        let cur = self.sel_mask();
        let out: Vec<u8> = prev
            .iter()
            .zip(&cur)
            .map(|(p, c)| match mode {
                1 => (*p).max(*c),
                2 => ((*p as u16 * (255 - *c as u16)) / 255) as u8,
                _ => (*p).min(*c),
            })
            .collect();
        self.select_mask(out);
    }

    pub fn select_ellipse(&mut self, a: Area) {
        let w = self.width();
        let (cx, cy) = ((a.x0 + a.x1) as f32 / 2.0, (a.y0 + a.y1) as f32 / 2.0);
        let (rx, ry) = (a.width() as f32 / 2.0, a.height() as f32 / 2.0);
        let mask = (0..w * self.height())
            .map(|i| {
                let (x, y) = ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
                let d = (((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2)).sqrt();
                (((1.0 - d) * rx.min(ry) + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8
            })
            .collect();
        self.select_mask(mask);
    }

    pub fn select_polygon(&mut self, pts: &[(f32, f32)]) {
        if pts.len() < 3 {
            return self.set_selection(None, None);
        }
        let cov = coverage(pts, self.width(), self.height());
        self.select_mask(cov.into_iter().map(|c| (c * 255.0).round() as u8).collect());
    }

    fn color_mask(&self, x: u32, y: u32, tolerance: u8, contiguous: bool, src: &RgbaImage) -> Vec<bool> {
        let (w, h) = (self.width(), self.height());
        let target = src.get_pixel(x, y).0;
        let close = |p: [u8; 4]| p.iter().zip(target).all(|(a, b)| a.abs_diff(b) <= tolerance);
        let mut mask = vec![false; (w * h) as usize];
        if contiguous {
            let mut stack = vec![(x, y)];
            while let Some((px, py)) = stack.pop() {
                let i = (py * w + px) as usize;
                if mask[i] || !close(src.get_pixel(px, py).0) {
                    continue;
                }
                mask[i] = true;
                if px > 0 { stack.push((px - 1, py)); }
                if px + 1 < w { stack.push((px + 1, py)); }
                if py > 0 { stack.push((px, py - 1)); }
                if py + 1 < h { stack.push((px, py + 1)); }
            }
        } else {
            for (i, p) in src.pixels().enumerate() {
                mask[i] = close(p.0);
            }
        }
        mask
    }

    #[cfg(test)]
    pub fn select_color(&mut self, x: u32, y: u32, tolerance: u8, contiguous: bool) {
        self.select_color_in(x, y, tolerance, contiguous, None);
    }

    pub fn select_color_in(&mut self, x: u32, y: u32, tolerance: u8, contiguous: bool, sample: Option<&RgbaImage>) {
        if x >= self.width() || y >= self.height() {
            return;
        }
        let m = match sample {
            Some(s) => self.color_mask(x, y, tolerance, contiguous, s),
            None => self.color_mask(x, y, tolerance, contiguous, &self.layers[self.active].img),
        };
        self.select_mask(m.into_iter().map(|b| if b { 255 } else { 0 }).collect());
    }

    pub fn quick_select(&mut self, cx: f32, cy: f32, radius: f32, tolerance: u8, subtract: bool) {
        let (w, h) = (self.width(), self.height());
        let (x, y) = (cx.floor(), cy.floor());
        if x < 0.0 || y < 0.0 || x >= w as f32 || y >= h as f32 {
            return;
        }
        let reach = radius * 3.0;
        let img = &self.layers[self.active].img;
        let seed = local_mean(img, x as i32, y as i32, (radius / 2.0).max(1.0) as i32);
        let tol = tolerance as f32 / 255.0;
        let mut add = vec![false; (w * h) as usize];
        let mut stack = vec![(x as u32, y as u32)];
        while let Some((px, py)) = stack.pop() {
            let i = (py * w + px) as usize;
            if add[i] {
                continue;
            }
            let d2 = (px as f32 + 0.5 - cx).powi(2) + (py as f32 + 0.5 - cy).powi(2);
            let p = img.get_pixel(px, py).0.map(|v| v as f32 / 255.0);
            let near = d2 <= radius * radius;
            if !near && (d2 > reach * reach || (0..3).map(|k| (p[k] - seed[k]).abs()).sum::<f32>() / 3.0 > tol) {
                continue;
            }
            add[i] = true;
            if px > 0 { stack.push((px - 1, py)); }
            if px + 1 < w { stack.push((px + 1, py)); }
            if py > 0 { stack.push((px, py - 1)); }
            if py + 1 < h { stack.push((px, py + 1)); }
        }
        let mut m = self.sel_mask();
        for (v, a) in m.iter_mut().zip(add) {
            if a {
                *v = if subtract { 0 } else { 255 };
            }
        }
        self.select_mask(m);
    }

    pub fn select_color_range(&mut self, target: [u8; 3], fuzz: f32, mode: u8) {
        let t = target.map(|v| v as f32 / 255.0);
        let fz = (fuzz / 200.0).max(0.005);
        let img = self.flatten();
        let m = img
            .pixels()
            .map(|p| {
                let c = [p.0[0] as f32 / 255.0, p.0[1] as f32 / 255.0, p.0[2] as f32 / 255.0];
                let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                let (hh, ss, _) = rgb_to_hsl(c[0], c[1], c[2]);
                let hue_w = |center: f32| (1.0 - ((hh - center + 0.5).rem_euclid(1.0) - 0.5).abs() * 6.0 / (fz * 4.0).clamp(0.3, 2.0)).max(0.0) * smooth(ss / 0.2);
                let w = match mode {
                    0 => 1.0 - ((c[0] - t[0]).abs() + (c[1] - t[1]).abs() + (c[2] - t[2]).abs()) / 3.0 / fz,
                    1 => 1.0 - smooth((l - 0.25) / 0.2),
                    2 => 1.0 - ((l - 0.5).abs() / 0.25).min(1.0),
                    3 => smooth((l - 0.6) / 0.2),
                    4..=9 => hue_w((mode - 4) as f32 / 6.0),
                    _ => {
                        let skin = (1.0 - ((hh - 0.06 + 0.5).rem_euclid(1.0) - 0.5).abs() / 0.05).max(0.0);
                        skin * smooth((ss - 0.15) / 0.15) * (1.0 - smooth((ss - 0.75) / 0.15)) * smooth((l - 0.2) / 0.15)
                    }
                };
                (w.clamp(0.0, 1.0) * 255.0).round() as u8
            })
            .collect();
        self.select_mask(m);
    }

    pub fn grow_similar(&mut self, tolerance: u8, contiguous: bool) {
        if self.selection.is_none() {
            return;
        }
        let (w, h) = (self.width(), self.height());
        let img = &self.layers[self.active].img;
        let cur = self.sel_mask();
        let mut sum = [0f64; 3];
        let mut n = 0f64;
        for (i, v) in cur.iter().enumerate() {
            if *v > 127 {
                let p = img.as_raw();
                for k in 0..3 {
                    sum[k] += p[i * 4 + k] as f64;
                }
                n += 1.0;
            }
        }
        let avg = sum.map(|s| (s / n.max(1.0)) as f32);
        let close = |p: [u8; 4]| (0..3).all(|k| (p[k] as f32 - avg[k]).abs() <= tolerance as f32);
        let mut out = cur.clone();
        if contiguous {
            let mut stack: Vec<(u32, u32)> = (0..w * h).filter(|i| cur[*i as usize] > 127).map(|i| (i % w, i / w)).collect();
            let mut seen: Vec<bool> = cur.iter().map(|v| *v > 127).collect();
            while let Some((px, py)) = stack.pop() {
                for (nx, ny) in [(px.wrapping_sub(1), py), (px + 1, py), (px, py.wrapping_sub(1)), (px, py + 1)] {
                    if nx >= w || ny >= h {
                        continue;
                    }
                    let i = (ny * w + nx) as usize;
                    if !seen[i] && close(img.get_pixel(nx, ny).0) {
                        seen[i] = true;
                        out[i] = 255;
                        stack.push((nx, ny));
                    }
                }
            }
        } else {
            for (i, p) in img.pixels().enumerate() {
                if close(p.0) {
                    out[i] = 255;
                }
            }
        }
        self.select_mask(out);
    }

    pub fn invert_selection(&mut self) {
        if self.selection.is_none() {
            return;
        }
        let m = self.sel_mask().into_iter().map(|v| 255 - v).collect();
        self.select_mask(m);
    }

    pub fn feather_selection(&mut self, r: f32) {
        if self.selection.is_none() || r <= 0.0 {
            return;
        }
        let g = image::GrayImage::from_raw(self.width(), self.height(), self.sel_mask()).unwrap_or_default();
        let b = imageops::fast_blur(&g, r);
        self.select_mask(b.into_raw());
    }

    pub fn smooth_selection(&mut self, r: f32) {
        if self.selection.is_none() || r <= 0.0 {
            return;
        }
        let g = image::GrayImage::from_raw(self.width(), self.height(), self.sel_mask()).unwrap_or_default();
        let b = imageops::fast_blur(&g, r);
        self.select_mask(b.pixels().map(|p: &Luma<u8>| if p.0[0] > 127 { 255 } else { 0 }).collect());
    }

    pub fn put_filtered(&mut self, base: &RgbaImage, out: &RgbaImage) {
        let (sel, w) = (self.selection, self.width());
        let mask = self.mask.clone();
        let (img, lock) = target(&mut self.layers, self.active, self.edit_mask);
        for (i, (d, (b, o))) in img.pixels_mut().zip(base.pixels().zip(out.pixels())).enumerate() {
            let k = weight(sel, mask.as_ref(), w, i as u32 % w, i as u32 / w);
            let mut px: [u8; 4] = std::array::from_fn(|c| (b.0[c] as f32 + (o.0[c] as f32 - b.0[c] as f32) * k).round() as u8);
            if lock {
                px[3] = b.0[3];
            }
            d.0 = px;
        }
        self.touch_all();
    }

    pub fn active_target(&self) -> RgbaImage {
        let l = &self.layers[self.active];
        match (&l.mask, self.edit_mask) {
            (Some(m), true) => m.clone(),
            _ => l.img.clone(),
        }
    }

    pub fn set_active_target(&mut self, img: RgbaImage) {
        let (t, _) = target(&mut self.layers, self.active, self.edit_mask);
        if t.dimensions() == img.dimensions() {
            *t = img;
        }
        self.touch_all();
    }

    pub fn fill_polygon(&mut self, pts: &[(f32, f32)], color: [u8; 4], mode: Blend) {
        if pts.len() < 3 {
            return;
        }
        let (w, h) = (self.width(), self.height());
        let cov = coverage(pts, w, h);
        let (sel, mask) = (self.selection, self.mask.clone());
        let (img, lock) = target(&mut self.layers, self.active, self.edit_mask);
        let src = [color[0] as f32 / 255.0, color[1] as f32 / 255.0, color[2] as f32 / 255.0];
        for (i, cv) in cov.iter().enumerate() {
            if *cv <= 0.0 {
                continue;
            }
            let (x, y) = (i as u32 % w, i as u32 / w);
            let k = cv * weight(sel, mask.as_ref(), w, x, y);
            if k <= 0.0 {
                continue;
            }
            let px = img.get_pixel_mut(x, y);
            let mut c = px.0.map(|v| v as f32 / 255.0);
            let old = c[3];
            let m = if mode == Blend::Normal || c[3] <= 0.0 { src } else { mode.mix3([c[0], c[1], c[2]], src) };
            over(&mut c, to_u8([m[0], m[1], m[2], color[3] as f32 / 255.0]), k);
            if lock {
                c[3] = old;
            }
            px.0 = to_u8(c);
        }
        let xs = pts.iter().map(|p| p.0);
        let ys = pts.iter().map(|p| p.1);
        let (mnx, mxx) = (xs.clone().fold(f32::MAX, f32::min), xs.fold(f32::MIN, f32::max));
        let (mny, mxy) = (ys.clone().fold(f32::MAX, f32::min), ys.fold(f32::MIN, f32::max));
        match Area::from_points((mnx - 1.0, mny - 1.0), (mxx + 1.0, mxy + 1.0), w, h) {
            Some(a) => self.touch(a),
            None => self.touch_all(),
        }
    }

    fn flood_mask(&self, x: u32, y: u32, tolerance: u8, contiguous: bool, sample: Option<&RgbaImage>) -> Vec<bool> {
        let src = sample.unwrap_or(&self.layers[self.active].img);
        let mut m = self.color_mask(x, y, tolerance, contiguous, src);
        if self.selection.is_some() {
            let w = self.width();
            for (i, v) in m.iter_mut().enumerate() {
                *v &= self.selected(i as u32 % w, i as u32 / w);
            }
        }
        m
    }

    #[cfg(test)]
    pub fn flood_fill(&mut self, x: u32, y: u32, color: [u8; 4], tolerance: u8) -> usize {
        self.flood_fill_ex(x, y, color, tolerance, true, None, 1.0)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn flood_fill_ex(&mut self, x: u32, y: u32, color: [u8; 4], tolerance: u8, contiguous: bool, sample: Option<&RgbaImage>, opacity: f32) -> usize {
        let (w, h) = (self.width(), self.height());
        if x >= w || y >= h || !self.selected(x, y) && self.selection.is_some() {
            return 0;
        }
        if sample.is_none() && opacity >= 1.0 && self.layers[self.active].img.get_pixel(x, y).0 == color {
            return 0;
        }
        let m = self.flood_mask(x, y, tolerance, contiguous, sample);
        let lock = self.layers[self.active].lock_alpha;
        let img = &mut self.layers[self.active].img;
        let mut count = 0;
        for (i, on) in m.iter().enumerate() {
            if *on {
                let p = img.get_pixel_mut(i as u32 % w, i as u32 / w);
                let mut c = p.0.map(|v| v as f32 / 255.0);
                let old = c[3];
                over(&mut c, color, opacity);
                if lock {
                    c[3] = old;
                }
                p.0 = to_u8(c);
                count += 1;
            }
        }
        self.touch_all();
        count
    }

    pub fn magic_erase(&mut self, x: u32, y: u32, tolerance: u8, contiguous: bool, opacity: f32) -> usize {
        if x >= self.width() || y >= self.height() {
            return 0;
        }
        let m = self.flood_mask(x, y, tolerance, contiguous, None);
        let w = self.width();
        let img = &mut self.layers[self.active].img;
        let mut n = 0;
        for (i, on) in m.iter().enumerate() {
            if *on {
                let p = img.get_pixel_mut(i as u32 % w, i as u32 / w);
                p.0[3] = (p.0[3] as f32 * (1.0 - opacity)).round() as u8;
                n += 1;
            }
        }
        self.touch_all();
        n
    }

    pub fn add_layer(&mut self, name: Option<String>, img: Option<RgbaImage>) {
        self.checkpoint();
        let (w, h) = (self.width(), self.height());
        let img = match img {
            Some(src) => {
                let mut canvas = RgbaImage::new(w, h);
                imageops::overlay(&mut canvas, &src, 0, 0);
                canvas
            }
            None => RgbaImage::new(w, h),
        };
        let name = name.unwrap_or_else(|| format!("Calque {}", self.layers.len() + 1));
        self.active += 1;
        self.layers.insert(self.active, Layer::new(name, img));
        self.edit_mask = false;
        self.touch_all();
    }

    pub fn add_adjustment(&mut self, kind: Kind, vals: Vec<f32>) {
        self.add_layer(Some(kind.title().to_string()), None);
        let full = RgbaImage::from_pixel(self.width(), self.height(), Rgba([255, 255, 255, 255]));
        let l = &mut self.layers[self.active];
        l.adjust = Some((kind, vals));
        l.mask = Some(full);
        self.touch_all();
    }

    pub fn duplicate_layer(&mut self) {
        self.checkpoint();
        let mut l = self.layers[self.active].clone();
        l.name = format!("{} copie", l.name);
        self.active += 1;
        self.layers.insert(self.active, l);
        self.touch_all();
    }

    pub fn delete_layer(&mut self) {
        if self.layers.len() < 2 {
            return;
        }
        self.checkpoint();
        self.layers.remove(self.active);
        self.active = self.active.saturating_sub(1);
        self.touch_all();
    }

    pub fn move_layer(&mut self, up: bool) {
        let to = if up { self.active + 1 } else { self.active.wrapping_sub(1) };
        if to >= self.layers.len() {
            return;
        }
        self.checkpoint();
        self.layers.swap(self.active, to);
        self.active = to;
        self.touch_all();
    }

    pub fn move_layer_to(&mut self, top: bool) {
        let to = if top { self.layers.len() - 1 } else { 0 };
        if to == self.active {
            return;
        }
        self.checkpoint();
        let l = self.layers.remove(self.active);
        self.layers.insert(to, l);
        self.active = to;
        self.touch_all();
    }

    pub fn merge_down(&mut self) {
        if self.active == 0 {
            return;
        }
        self.checkpoint();
        let below = self.active - 1;
        let mut merged = self.layers[below].clone();
        merged.visible = true;
        merged.opacity = 1.0;
        merged.blend = Blend::Normal;
        merged.mask = None;
        merged.clip = false;
        merged.adjust = None;
        let saved = (self.layers[below].opacity, self.layers[below].blend, self.layers[below].visible);
        self.layers[below].opacity = 1.0;
        self.layers[below].blend = Blend::Normal;
        self.layers[below].visible = true;
        merged.img = self.baked(&[below, self.active]);
        let _ = saved;
        self.layers.remove(self.active);
        self.layers[below] = merged;
        self.layers[below].opacity = saved.0;
        self.layers[below].blend = saved.1;
        self.active = below;
        self.touch_all();
    }

    pub fn clear_selection(&mut self) {
        let Some(sel) = self.selection else { return };
        self.checkpoint();
        let w = self.width();
        let mask = self.mask.clone();
        let (img, lock) = target(&mut self.layers, self.active, self.edit_mask);
        for y in sel.y0..sel.y1 {
            for x in sel.x0..sel.x1 {
                let k = weight(Some(sel), mask.as_ref(), w, x, y);
                if k > 0.0 {
                    let p = img.get_pixel_mut(x, y);
                    if lock {
                        p.0[0] = 255;
                        p.0[1] = 255;
                        p.0[2] = 255;
                    } else {
                        p.0[3] = (p.0[3] as f32 * (1.0 - k)).round() as u8;
                    }
                }
            }
        }
        self.touch(sel);
    }

    pub fn crop_to_selection(&mut self) {
        let Some(s) = self.selection.take() else { return };
        self.mask = None;
        self.crop_to(s);
    }

    pub fn crop_to(&mut self, s: Area) {
        self.selection = None;
        self.mask = None;
        self.transform(|img| imageops::crop_imm(img, s.x0, s.y0, s.width(), s.height()).to_image());
    }

    pub fn place_layer(&mut self, original: &RgbaImage, dx: i32, dy: i32) {
        let mut out = RgbaImage::new(original.width(), original.height());
        imageops::overlay(&mut out, original, dx as i64, dy as i64);
        self.layers[self.active].img = out;
        self.touch_all();
    }

    pub fn nudge(&mut self, dx: i32, dy: i32) {
        self.checkpoint();
        let original = self.layers[self.active].img.clone();
        self.place_layer(&original, dx, dy);
    }

    pub fn layer_bounds(&self, i: usize) -> Option<Area> {
        let img = &self.layers[i].img;
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for (x, y, p) in img.enumerate_pixels() {
            if p.0[3] > 0 {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
        (x1 > x0).then_some(Area { x0, y0, x1, y1 })
    }

    pub fn layer_at(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width() || y >= self.height() {
            return None;
        }
        (0..self.layers.len()).rev().find(|&i| {
            let l = &self.layers[i];
            l.visible && l.adjust.is_none() && l.img.get_pixel(x, y).0[3] > 16 && l.mask_at(x, y) > 0.1
        })
    }

    pub fn align_layer(&mut self, how: u8, to: Area) -> bool {
        let Some(b) = self.layer_bounds(self.active) else { return false };
        let (dx, dy) = match how {
            0 => (to.x0 as i32 - b.x0 as i32, 0),
            1 => (((to.x0 + to.x1) as i32 - (b.x0 + b.x1) as i32) / 2, 0),
            2 => (to.x1 as i32 - b.x1 as i32, 0),
            3 => (0, to.y0 as i32 - b.y0 as i32),
            4 => (0, ((to.y0 + to.y1) as i32 - (b.y0 + b.y1) as i32) / 2),
            _ => (0, to.y1 as i32 - b.y1 as i32),
        };
        if (dx, dy) == (0, 0) {
            return false;
        }
        self.nudge(dx, dy);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn gradient_ex(&mut self, from: (f32, f32), to: (f32, f32), stops: &[[u8; 4]], kind: u8, reverse: bool, opacity: f32, dither: bool, mode: Blend) {
        self.checkpoint();
        let sel = self.selection.unwrap_or(self.full());
        let d = (to.0 - from.0, to.1 - from.1);
        let len2 = (d.0 * d.0 + d.1 * d.1).max(1e-3);
        let len = len2.sqrt();
        let (s0, mask, w) = (self.selection, self.mask.clone(), self.width());
        let (img, lock) = target(&mut self.layers, self.active, self.edit_mask);
        for y in sel.y0..sel.y1 {
            for x in sel.x0..sel.x1 {
                let k = weight(s0, mask.as_ref(), w, x, y);
                if k <= 0.0 {
                    continue;
                }
                let (px, py) = (x as f32 + 0.5 - from.0, y as f32 + 0.5 - from.1);
                let along = (px * d.0 + py * d.1) / len2;
                let mut t = match kind {
                    1 => (px * px + py * py).sqrt() / len,
                    2 => ((py.atan2(px) - d.1.atan2(d.0)) / std::f32::consts::TAU).rem_euclid(1.0),
                    3 => along.abs(),
                    4 => {
                        let ux = (px * d.0 + py * d.1) / len;
                        let uy = (-px * d.1 + py * d.0) / len;
                        (ux.abs() + uy.abs()) / len
                    }
                    _ => along,
                }
                .clamp(0.0, 1.0);
                if reverse {
                    t = 1.0 - t;
                }
                if dither {
                    t = (t + (hash(x, y, 77) - 0.5) / 255.0).clamp(0.0, 1.0);
                }
                let f = t * (stops.len() - 1).max(1) as f32;
                let i = (f.floor() as usize).min(stops.len().saturating_sub(2));
                let kk = f - i as f32;
                let (a, b) = (stops[i], stops[(i + 1).min(stops.len() - 1)]);
                let c: [f32; 4] = std::array::from_fn(|q| (a[q] as f32 + (b[q] as f32 - a[q] as f32) * kk) / 255.0);
                let p = img.get_pixel_mut(x, y);
                let mut acc = p.0.map(|v| v as f32 / 255.0);
                let old = acc[3];
                let m = if mode == Blend::Normal || acc[3] <= 0.0 { [c[0], c[1], c[2]] } else { mode.mix3([acc[0], acc[1], acc[2]], [c[0], c[1], c[2]]) };
                over(&mut acc, to_u8([m[0], m[1], m[2], c[3]]), opacity * k);
                if lock {
                    acc[3] = old;
                }
                p.0 = to_u8(acc);
            }
        }
        self.touch(sel);
    }

    pub fn gradient_kind(&mut self, from: (f32, f32), to: (f32, f32), a: [u8; 4], b: [u8; 4], radial: bool) {
        self.gradient_ex(from, to, &[a, b], radial as u8, false, 1.0, false, Blend::Normal);
    }

    pub fn transform(&mut self, f: impl Fn(&RgbaImage) -> RgbaImage) {
        self.checkpoint();
        for l in &mut self.layers {
            l.img = f(&l.img);
            if let Some(m) = &l.mask {
                l.mask = Some(f(m));
            }
        }
        self.touch_all();
    }

    pub fn canvas_size(&mut self, w: u32, h: u32, anchor: (f32, f32), fill: Option<[u8; 4]>) {
        let (ow, oh) = (self.width() as i64, self.height() as i64);
        let dx = ((w as i64 - ow) as f32 * anchor.0).round() as i64;
        let dy = ((h as i64 - oh) as f32 * anchor.1).round() as i64;
        self.selection = None;
        self.mask = None;
        self.checkpoint();
        for (i, l) in self.layers.iter_mut().enumerate() {
            let bg = if i == 0 { fill.unwrap_or([0, 0, 0, 0]) } else { [0, 0, 0, 0] };
            let mut out = RgbaImage::from_pixel(w.max(1), h.max(1), Rgba(bg));
            imageops::overlay(&mut out, &l.img, dx, dy);
            l.img = out;
            if let Some(m) = &l.mask {
                let mut out = RgbaImage::from_pixel(w.max(1), h.max(1), Rgba([255, 255, 255, 255]));
                imageops::overlay(&mut out, m, dx, dy);
                l.mask = Some(out);
            }
        }
        self.touch_all();
    }

    pub fn rotate_canvas(&mut self, angle: f32, fill: Option<[u8; 4]>) {
        let (w, h) = (self.width() as f32, self.height() as f32);
        let (sn, cs) = angle.to_radians().sin_cos();
        let (nw, nh) = ((w * cs.abs() + h * sn.abs()).ceil() as u32, (w * sn.abs() + h * cs.abs()).ceil() as u32);
        let fwd = mul(&[[1.0, 0.0, nw as f32 / 2.0], [0.0, 1.0, nh as f32 / 2.0], [0.0, 0.0, 1.0]], &mul(&[[cs, -sn, 0.0], [sn, cs, 0.0], [0.0, 0.0, 1.0]], &[[1.0, 0.0, -w / 2.0], [0.0, 1.0, -h / 2.0], [0.0, 0.0, 1.0]]));
        let Some(inv) = invert(&fwd) else { return };
        self.selection = None;
        self.mask = None;
        self.checkpoint();
        for (i, l) in self.layers.iter_mut().enumerate() {
            let mut out = warp(&l.img, nw.max(1), nh.max(1), &inv, false);
            if i == 0
                && let Some(f) = fill
            {
                let mut bg = RgbaImage::from_pixel(out.width(), out.height(), Rgba(f));
                imageops::overlay(&mut bg, &out, 0, 0);
                out = bg;
            }
            l.img = out;
            if let Some(m) = &l.mask {
                l.mask = Some(warp(m, nw.max(1), nh.max(1), &inv, false));
            }
        }
        self.touch_all();
    }

    pub fn trim(&mut self) -> bool {
        let (w, h) = (self.width(), self.height());
        let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
        for l in &self.layers {
            for (x, y, p) in l.img.enumerate_pixels() {
                if p.0[3] > 0 {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                }
            }
        }
        if x1 <= x0 || (x0, y0, x1, y1) == (0, 0, w, h) {
            return false;
        }
        self.transform(|img| imageops::crop_imm(img, x0, y0, x1 - x0, y1 - y0).to_image());
        true
    }

    pub fn flatten_all(&mut self) {
        self.checkpoint();
        let flat = self.flatten();
        self.layers = vec![Layer::new("Fond", flat)];
        self.active = 0;
        self.edit_mask = false;
        self.touch_all();
    }

    pub fn merge_visible(&mut self) {
        let visible: Vec<usize> = (0..self.layers.len()).filter(|&i| self.layers[i].visible).collect();
        if visible.len() < 2 {
            return;
        }
        self.checkpoint();
        let out = self.baked(&visible);
        let keep = visible[0];
        for &i in visible.iter().skip(1).rev() {
            self.layers.remove(i);
        }
        self.layers[keep] = Layer::new("Fusion", out);
        self.active = keep;
        self.edit_mask = false;
        self.touch_all();
    }

    pub fn copy_selection(&self) -> Option<(RgbaImage, u32, u32)> {
        let s = self.selection?;
        let src = &self.layers[self.active].img;
        let mut out = RgbaImage::new(s.width(), s.height());
        for y in s.y0..s.y1 {
            for x in s.x0..s.x1 {
                let k = self.sel_weight(x, y);
                if k > 0.0 {
                    let mut p = *src.get_pixel(x, y);
                    p.0[3] = (p.0[3] as f32 * k).round() as u8;
                    out.put_pixel(x - s.x0, y - s.y0, p);
                }
            }
        }
        Some((out, s.x0, s.y0))
    }

    pub fn copy_merged(&self) -> RgbaImage {
        let flat = self.flatten();
        match self.selection {
            Some(s) => RgbaImage::from_fn(s.width(), s.height(), |x, y| {
                let mut p = *flat.get_pixel(x + s.x0, y + s.y0);
                p.0[3] = (p.0[3] as f32 * self.sel_weight(x + s.x0, y + s.y0)).round() as u8;
                p
            }),
            None => flat,
        }
    }

    pub fn layer_from_selection(&mut self, cut: bool) -> bool {
        let Some((img, x, y)) = self.copy_selection() else { return false };
        if cut {
            self.clear_selection();
        }
        self.paste_image(img, (x as i64, y as i64), if cut { "Calque coupé" } else { "Calque copié" });
        true
    }

    pub fn paste_image(&mut self, img: RgbaImage, at: (i64, i64), name: &str) {
        let mut canvas = RgbaImage::new(self.width(), self.height());
        imageops::overlay(&mut canvas, &img, at.0, at.1);
        self.add_layer(Some(name.to_string()), Some(canvas));
    }

    fn morph_selection(&self, px: i32) -> Vec<u8> {
        let (w, h) = (self.width() as usize, self.height() as usize);
        let m: Vec<bool> = self.sel_mask().iter().map(|v| *v > 127).collect();
        let out: Vec<bool> = if px > 0 { dilate(&m, w, h, px as usize) } else { dilate(&m.iter().map(|v| !v).collect::<Vec<_>>(), w, h, (-px) as usize).into_iter().map(|v| !v).collect() };
        out.into_iter().map(|b| if b { 255 } else { 0 }).collect()
    }

    pub fn grow_selection(&mut self, px: i32) {
        if self.selection.is_none() || px == 0 {
            return;
        }
        let m = self.morph_selection(px);
        self.select_mask(m);
    }

    pub fn border_selection(&mut self, px: u32) {
        if self.selection.is_none() || px == 0 {
            return;
        }
        let (outer, inner) = (self.morph_selection(px as i32), self.morph_selection(-(px as i32)));
        self.select_mask(outer.iter().zip(inner).map(|(o, i)| if *o > 0 && i == 0 { 255 } else { 0 }).collect());
    }

    pub fn select_layer_alpha(&mut self) {
        let m: Vec<u8> = self.layers[self.active].img.pixels().map(|p| p.0[3]).collect();
        self.select_mask(m);
    }

    pub fn fill_selection(&mut self, color: [u8; 4]) {
        self.fill_selection_ex(color, 1.0, Blend::Normal);
    }

    pub fn fill_selection_ex(&mut self, color: [u8; 4], opacity: f32, mode: Blend) {
        self.checkpoint();
        let (sel, mask, w) = (self.selection, self.mask.clone(), self.width());
        let s = self.selection.unwrap_or(self.full());
        let (img, lock) = target(&mut self.layers, self.active, self.edit_mask);
        let src = [color[0] as f32 / 255.0, color[1] as f32 / 255.0, color[2] as f32 / 255.0];
        for y in s.y0..s.y1 {
            for x in s.x0..s.x1 {
                let k = weight(sel, mask.as_ref(), w, x, y);
                if k > 0.0 {
                    let px = img.get_pixel_mut(x, y);
                    let mut c = px.0.map(|v| v as f32 / 255.0);
                    let old = c[3];
                    let m = if mode == Blend::Normal || c[3] <= 0.0 { src } else { mode.mix3([c[0], c[1], c[2]], src) };
                    over(&mut c, to_u8([m[0], m[1], m[2], color[3] as f32 / 255.0]), k * opacity);
                    if lock {
                        c[3] = old;
                    }
                    px.0 = to_u8(c);
                }
            }
        }
        self.touch_all();
    }

    pub fn stroke_selection(&mut self, color: [u8; 4], width: u32, position: u8) {
        if self.selection.is_none() {
            return;
        }
        let wd = width.max(1) as i32;
        let (o, i) = match position {
            0 => (0, -wd),
            1 => (wd / 2, -(wd - wd / 2)),
            _ => (wd, 0),
        };
        let outer = if o == 0 { self.sel_mask() } else { self.morph_selection(o) };
        let inner = if i == 0 { self.sel_mask() } else { self.morph_selection(i) };
        self.checkpoint();
        let lock = self.layers[self.active].lock_alpha;
        let img = &mut self.layers[self.active].img;
        for (k, p) in img.pixels_mut().enumerate() {
            if outer[k] > 127 && inner[k] <= 127 {
                let mut c = p.0.map(|v| v as f32 / 255.0);
                let old = c[3];
                over(&mut c, color, 1.0);
                if lock {
                    c[3] = old;
                }
                p.0 = to_u8(c);
            }
        }
        self.touch_all();
    }

    pub fn transform_layer(&mut self, scale: f32, angle_deg: f32) {
        self.transform_layer_full(&Xform { sx: scale.max(0.01), sy: scale.max(0.01), angle: angle_deg, ..Xform::identity() });
    }

    pub fn transform_layer_full(&mut self, t: &Xform) {
        let (w, h) = (self.width(), self.height());
        let Some(inv) = invert(&t.matrix(w as f32, h as f32)) else { return };
        self.checkpoint();
        let l = &mut self.layers[self.active];
        l.img = warp(&l.img, w, h, &inv, t.nearest);
        if let Some(m) = &l.mask {
            l.mask = Some(warp(m, w, h, &inv, t.nearest));
        }
        self.touch_all();
    }

    pub fn flip_layer(&mut self, horizontal: bool) {
        self.checkpoint();
        let l = &mut self.layers[self.active];
        l.img = if horizontal { imageops::flip_horizontal(&l.img) } else { imageops::flip_vertical(&l.img) };
        self.touch_all();
    }

    fn insert_effect(&mut self, name: &str, img: RgbaImage, above: bool, blend: Blend) {
        self.checkpoint();
        let at = self.active;
        let mut l = Layer::new(format!("{name} — {}", self.layers[at].name), img);
        l.blend = blend;
        if above {
            self.layers.insert(at + 1, l);
        } else {
            self.layers.insert(at, l);
            self.active = at + 1;
        }
        self.touch_all();
    }

    fn alpha_vec(&self) -> Vec<f32> {
        self.layers[self.active].img.pixels().map(|p| p.0[3] as f32 / 255.0).collect()
    }

    fn blur_vec(&self, v: &[f32], r: f32) -> Vec<f32> {
        if r <= 0.0 {
            return v.to_vec();
        }
        let g = image::GrayImage::from_raw(self.width(), self.height(), v.iter().map(|a| (a * 255.0).round() as u8).collect()).unwrap_or_default();
        imageops::fast_blur(&g, r).into_raw().into_iter().map(|b| b as f32 / 255.0).collect()
    }

    fn shift_vec(&self, v: &[f32], dx: i64, dy: i64, outside: f32) -> Vec<f32> {
        let (w, h) = (self.width() as i64, self.height() as i64);
        (0..w * h)
            .map(|i| {
                let (x, y) = (i % w - dx, i / w - dy);
                if x < 0 || y < 0 || x >= w || y >= h { outside } else { v[(y * w + x) as usize] }
            })
            .collect()
    }

    fn tinted(&self, alpha: &[f32], color: [u8; 4]) -> RgbaImage {
        let w = self.width();
        RgbaImage::from_fn(w, self.height(), |x, y| {
            let a = alpha[(y * w + x) as usize].clamp(0.0, 1.0) * color[3] as f32 / 255.0;
            Rgba([color[0], color[1], color[2], (a * 255.0).round() as u8])
        })
    }

    pub fn drop_shadow(&mut self, dx: i64, dy: i64, blur: f32, color: [u8; 4]) {
        let a = self.alpha_vec();
        let moved = self.shift_vec(&a, dx, dy, 0.0);
        let out = self.tinted(&self.blur_vec(&moved, blur), color);
        self.insert_effect("Ombre", out, false, Blend::Multiply);
    }

    pub fn outer_glow(&mut self, size: f32, color: [u8; 4]) {
        let (w, h) = (self.width() as usize, self.height() as usize);
        let m: Vec<bool> = self.alpha_vec().iter().map(|a| *a > 0.03).collect();
        let grown: Vec<f32> = dilate(&m, w, h, (size / 3.0) as usize).into_iter().map(|b| if b { 1.0 } else { 0.0 }).collect();
        let out = self.tinted(&self.blur_vec(&grown, size.max(0.5)), color);
        self.insert_effect("Lueur", out, false, Blend::Screen);
    }

    #[cfg(test)]
    pub fn layer_outline(&mut self, width: u32, color: [u8; 4]) {
        self.layer_outline_at(width, color, 2);
    }

    pub fn layer_outline_at(&mut self, width: u32, color: [u8; 4], position: u8) {
        let (w, h) = (self.width() as usize, self.height() as usize);
        let wd = width.max(1) as usize;
        let m: Vec<bool> = self.alpha_vec().iter().map(|a| *a > 0.03).collect();
        let inv: Vec<bool> = m.iter().map(|v| !v).collect();
        let (outer, inner): (Vec<bool>, Vec<bool>) = match position {
            0 => (m.clone(), dilate(&inv, w, h, wd).into_iter().map(|v| !v).collect()),
            1 => (dilate(&m, w, h, wd / 2), dilate(&inv, w, h, wd - wd / 2).into_iter().map(|v| !v).collect()),
            _ => (dilate(&m, w, h, wd), m.clone()),
        };
        let ring: Vec<f32> = outer.iter().zip(&inner).map(|(o, i)| if *o && !i { 1.0 } else { 0.0 }).collect();
        let out = self.tinted(&ring, color);
        self.insert_effect("Contour", out, position < 2, Blend::Normal);
    }

    pub fn inner_shadow(&mut self, dx: i64, dy: i64, blur: f32, color: [u8; 4]) {
        let a = self.alpha_vec();
        let inv: Vec<f32> = a.iter().map(|v| 1.0 - v).collect();
        let moved = self.blur_vec(&self.shift_vec(&inv, dx, dy, 1.0), blur);
        let shade: Vec<f32> = moved.iter().zip(&a).map(|(m, a)| m * a).collect();
        let out = self.tinted(&shade, color);
        self.insert_effect("Ombre interne", out, true, Blend::Multiply);
    }

    pub fn inner_glow(&mut self, size: f32, color: [u8; 4]) {
        let a = self.alpha_vec();
        let inv: Vec<f32> = a.iter().map(|v| 1.0 - v).collect();
        let b = self.blur_vec(&inv, size.max(0.5));
        let glow: Vec<f32> = b.iter().zip(&a).map(|(g, a)| (g * 2.0).min(1.0) * a).collect();
        let out = self.tinted(&glow, color);
        self.insert_effect("Lueur interne", out, true, Blend::Screen);
    }

    pub fn bevel(&mut self, size: f32, depth: f32, angle: f32, highlight: [u8; 4], shadow: [u8; 4]) {
        let (w, h) = (self.width() as i64, self.height() as i64);
        let a = self.alpha_vec();
        let height = self.blur_vec(&a, size.max(0.5));
        let (lx, ly) = (angle.to_radians().cos(), -angle.to_radians().sin());
        let at = |x: i64, y: i64| height[(y.clamp(0, h - 1) * w + x.clamp(0, w - 1)) as usize];
        let out = RgbaImage::from_fn(w as u32, h as u32, |x, y| {
            let (x, y) = (x as i64, y as i64);
            let gx = at(x + 1, y) - at(x - 1, y);
            let gy = at(x, y + 1) - at(x, y - 1);
            let shade = (-(gx * lx + gy * ly) * depth * size.max(1.0) * 0.6).clamp(-1.0, 1.0);
            let base = a[(y * w + x) as usize];
            let (c, k) = if shade >= 0.0 { (highlight, shade) } else { (shadow, -shade) };
            Rgba([c[0], c[1], c[2], (k * base * c[3] as f32).round().clamp(0.0, 255.0) as u8])
        });
        self.insert_effect("Biseau", out, true, Blend::Normal);
    }

    pub fn color_overlay(&mut self, color: [u8; 4], blend: Blend) {
        let a = self.alpha_vec();
        let out = self.tinted(&a, color);
        self.insert_effect("Incrustation couleur", out, true, blend);
    }

    pub fn gradient_overlay(&mut self, from: [u8; 4], to: [u8; 4], angle: f32, blend: Blend) {
        let (w, h) = (self.width(), self.height());
        let a = self.alpha_vec();
        let (dx, dy) = (angle.to_radians().cos(), -angle.to_radians().sin());
        let half = (w as f32 * dx.abs() + h as f32 * dy.abs()) / 2.0;
        let out = RgbaImage::from_fn(w, h, |x, y| {
            let t = (((x as f32 - w as f32 / 2.0) * dx + (y as f32 - h as f32 / 2.0) * dy) / half.max(1.0) * 0.5 + 0.5).clamp(0.0, 1.0);
            let c: [f32; 4] = std::array::from_fn(|k| from[k] as f32 + (to[k] as f32 - from[k] as f32) * t);
            let al = a[(y * w + x) as usize] * c[3];
            Rgba([c[0] as u8, c[1] as u8, c[2] as u8, al.round() as u8])
        });
        self.insert_effect("Incrustation dégradé", out, true, blend);
    }

    pub fn gradient_map(&mut self, dark: [u8; 4], light: [u8; 4]) {
        self.adjust_map(move |p| {
            let l = (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0;
            for k in 0..3 {
                p[k] = (dark[k] as f32 + (light[k] as f32 - dark[k] as f32) * l).round() as u8;
            }
        });
    }

    fn adjust_map(&mut self, f: impl Fn(&mut [u8; 4])) {
        self.checkpoint();
        let (sel, mask, w) = (self.selection, self.mask.clone(), self.width());
        for (x, y, p) in self.layers[self.active].img.enumerate_pixels_mut() {
            if inside(sel, mask.as_ref(), w, x, y) {
                f(&mut p.0);
            }
        }
        self.touch_all();
    }

    pub fn add_mask(&mut self, kind: u8) {
        let (w, h) = (self.width(), self.height());
        let m = match kind {
            0 => RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255])),
            1 => RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 255])),
            _ => {
                let s = self.sel_mask();
                let all = self.selection.is_none();
                RgbaImage::from_fn(w, h, |x, y| {
                    let v = if all { 255 } else { s[(y * w + x) as usize] };
                    Rgba([v, v, v, 255])
                })
            }
        };
        self.checkpoint();
        let l = &mut self.layers[self.active];
        l.mask = Some(m);
        l.mask_on = true;
        self.edit_mask = true;
        self.touch_all();
    }

    pub fn apply_mask(&mut self) {
        if self.layers[self.active].mask.is_none() {
            return;
        }
        self.checkpoint();
        let l = &mut self.layers[self.active];
        if let Some(m) = l.mask.take() {
            for (p, q) in l.img.pixels_mut().zip(m.pixels()) {
                p.0[3] = (p.0[3] as f32 * q.0[0] as f32 / 255.0 * q.0[3] as f32 / 255.0).round() as u8;
            }
        }
        self.edit_mask = false;
        self.touch_all();
    }

    pub fn delete_mask(&mut self) {
        if self.layers[self.active].mask.is_none() {
            return;
        }
        self.checkpoint();
        self.layers[self.active].mask = None;
        self.edit_mask = false;
        self.touch_all();
    }

    pub fn invert_mask(&mut self) {
        if self.layers[self.active].mask.is_none() {
            return;
        }
        self.checkpoint();
        if let Some(m) = &mut self.layers[self.active].mask {
            for p in m.pixels_mut() {
                let v = 255 - (p.0[0] as u16 * p.0[3] as u16 / 255) as u8;
                p.0 = [v, v, v, 255];
            }
        }
        self.touch_all();
    }

    pub fn select_from_mask(&mut self) {
        if let Some(m) = &self.layers[self.active].mask {
            let v: Vec<u8> = m.pixels().map(|p| (p.0[0] as u16 * p.0[3] as u16 / 255) as u8).collect();
            self.select_mask(v);
        }
    }
}

fn coverage(pts: &[(f32, f32)], w: u32, h: u32) -> Vec<f32> {
    let mut cov = vec![0f32; (w * h) as usize];
    let ymin = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor().max(0.0) as u32;
    let ymax = (pts.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil().max(0.0) as u32).min(h);
    const SUB: usize = 4;
    for y in ymin..ymax {
        for s in 0..SUB {
            let fy = y as f32 + (s as f32 + 0.5) / SUB as f32;
            let mut xs: Vec<f32> = Vec::new();
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                if (a.1 <= fy) != (b.1 <= fy) {
                    xs.push(a.0 + (fy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(f32::total_cmp);
            for pair in xs.chunks(2).filter(|p| p.len() == 2) {
                let (x0, x1) = (pair[0].clamp(0.0, w as f32), pair[1].clamp(0.0, w as f32));
                if x1 <= x0 {
                    continue;
                }
                let (i0, i1) = (x0.floor() as u32, (x1.ceil() as u32).min(w));
                for x in i0..i1 {
                    let span = (x1.min(x as f32 + 1.0) - x0.max(x as f32)).max(0.0);
                    cov[(y * w + x) as usize] += span / SUB as f32;
                }
            }
        }
    }
    cov.iter_mut().for_each(|c| *c = c.min(1.0));
    cov
}

fn local_mean(img: &RgbaImage, x: i32, y: i32, r: i32) -> [f32; 4] {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let mut avg = [0f32; 4];
    let mut n = 0.0;
    for yy in (y - r).max(0)..=(y + r).min(h - 1) {
        for xx in (x - r).max(0)..=(x + r).min(w - 1) {
            let q = img.get_pixel(xx as u32, yy as u32).0;
            (0..4).for_each(|k| avg[k] += q[k] as f32 / 255.0);
            n += 1.0;
        }
    }
    avg.map(|v| v / f32::max(n, 1.0))
}

pub fn fade(base: &RgbaImage, out: &RgbaImage, opacity: f32, mode: Blend) -> RgbaImage {
    let mut res = base.clone();
    for (p, q) in res.pixels_mut().zip(out.pixels()) {
        let b = p.0.map(|v| v as f32 / 255.0);
        let s = q.0.map(|v| v as f32 / 255.0);
        let m = mode.mix3([b[0], b[1], b[2]], [s[0], s[1], s[2]]);
        p.0 = to_u8([b[0] + (m[0] - b[0]) * opacity, b[1] + (m[1] - b[1]) * opacity, b[2] + (m[2] - b[2]) * opacity, b[3] + (s[3] - b[3]) * opacity]);
    }
    res
}

fn dilate(m: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    let pass = |src: &[bool], horizontal: bool| -> Vec<bool> {
        let mut out = vec![false; src.len()];
        let (lines, len) = if horizontal { (h, w) } else { (w, h) };
        for line in 0..lines {
            let idx = |k: usize| if horizontal { line * w + k } else { k * w + line };
            let mut count = 0usize;
            for k in 0..=r.min(len - 1) {
                count += src[idx(k)] as usize;
            }
            for k in 0..len {
                out[idx(k)] = count > 0;
                if k + r + 1 < len {
                    count += src[idx(k + r + 1)] as usize;
                }
                if k >= r {
                    count -= src[idx(k - r)] as usize;
                }
            }
        }
        out
    };
    pass(&pass(m, true), false)
}

fn bilinear(img: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let (w, h) = img.dimensions();
    let (x0, y0) = (x.floor().max(0.0) as u32, y.floor().max(0.0) as u32);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = ((x - x0 as f32).clamp(0.0, 1.0), (y - y0 as f32).clamp(0.0, 1.0));
    let (a, b, c, d) = (img.get_pixel(x0.min(w - 1), y0.min(h - 1)).0, img.get_pixel(x1, y0.min(h - 1)).0, img.get_pixel(x0.min(w - 1), y1).0, img.get_pixel(x1, y1).0);
    Rgba(std::array::from_fn(|k| {
        let top = a[k] as f32 * (1.0 - fx) + b[k] as f32 * fx;
        let bot = c[k] as f32 * (1.0 - fx) + d[k] as f32 * fx;
        (top * (1.0 - fy) + bot * fy).round() as u8
    }))
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn over(acc: &mut [f32; 4], p: [u8; 4], opacity: f32) {
    let a = p[3] as f32 / 255.0 * opacity;
    if a <= 0.0 {
        return;
    }
    let out_a = a + acc[3] * (1.0 - a);
    for c in 0..3 {
        acc[c] = (p[c] as f32 / 255.0 * a + acc[c] * acc[3] * (1.0 - a)) / out_a;
    }
    acc[3] = out_a;
}

fn blend_over(acc: &mut [f32; 4], p: [u8; 4], opacity: f32, mode: Blend, x: u32, y: u32) {
    if mode == Blend::Dissolve {
        let a = p[3] as f32 / 255.0 * opacity;
        if hash(x, y, 31) < a {
            over(acc, [p[0], p[1], p[2], 255], 1.0);
        }
        return;
    }
    if mode == Blend::Normal || acc[3] <= 0.0 {
        return over(acc, p, opacity);
    }
    let s = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
    let m = mode.mix3([acc[0], acc[1], acc[2]], s);
    let mixed: [f32; 3] = std::array::from_fn(|k| s[k] + (m[k] - s[k]) * acc[3]);
    over(acc, [(mixed[0].clamp(0.0, 1.0) * 255.0).round() as u8, (mixed[1].clamp(0.0, 1.0) * 255.0).round() as u8, (mixed[2].clamp(0.0, 1.0) * 255.0).round() as u8, p[3]], opacity);
}

fn to_u8(acc: [f32; 4]) -> [u8; 4] {
    acc.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min {
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
    (h / 6.0, s, l)
}

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s == 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let f = |mut t: f32| {
        t = t.rem_euclid(1.0);
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
    (f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_fill_layers_undo() {
        let mut d = Document::new(64, 64, Some([255, 255, 255, 255]));
        let brush = Brush::new(4.0, 1.0, 1.0);
        d.checkpoint();
        d.stroke((10.0, 10.0), (50.0, 10.0), &brush, [255, 0, 0, 255], false);
        assert_eq!(d.layers[0].img.get_pixel(30, 10).0, [255, 0, 0, 255]);
        assert_eq!(d.layers[0].img.get_pixel(30, 40).0, [255, 255, 255, 255]);

        d.checkpoint();
        let filled = d.flood_fill(30, 40, [0, 0, 255, 255], 0);
        assert!(filled > 3000 && filled < 64 * 64);

        d.add_layer(None, None);
        d.layers[1].opacity = 0.5;
        d.checkpoint();
        d.stamp(30.5, 40.5, &brush, [0, 0, 0, 255], false);
        let flat = d.flatten();
        assert_eq!(flat.get_pixel(30, 40).0, [0, 0, 128, 255]);
        assert_eq!(d.history().0.len(), 5);

        d.undo();
        d.undo();
        d.undo();
        assert_eq!(d.layers.len(), 1);
        assert_eq!(d.layers[0].img.get_pixel(30, 40).0, [255, 255, 255, 255]);
        d.redo();
        assert_eq!(d.layers[0].img.get_pixel(30, 40).0, [0, 0, 255, 255]);
        d.jump_history(0);
        assert!(!d.can_undo());
        d.jump_history(4);
        assert_eq!(d.layers.len(), 2);

        let mut s2 = Document::new(20, 20, Some([255, 255, 255, 255]));
        s2.selection = Area::from_points((5.0, 5.0), (10.0, 10.0), 20, 20);
        s2.checkpoint();
        s2.stamp(10.0, 10.0, &Brush::new(8.0, 1.0, 1.0), [0, 0, 0, 255], false);
        assert_eq!(s2.layers[0].img.get_pixel(7, 7).0, [0, 0, 0, 255]);
        assert_eq!(s2.layers[0].img.get_pixel(12, 12).0, [255, 255, 255, 255]);
        s2.gradient_kind((5.0, 0.0), (10.0, 0.0), [255, 0, 0, 255], [0, 0, 255, 255], false);
        assert!(s2.layers[0].img.get_pixel(5, 6).0[0] > 200);
        assert!(s2.layers[0].img.get_pixel(9, 6).0[2] > 150);
        s2.clear_selection();
        assert_eq!(s2.layers[0].img.get_pixel(7, 7).0[3], 0);
        s2.crop_to_selection();
        assert_eq!((s2.width(), s2.height()), (5, 5));

        let mut m = Document::new(20, 20, Some([255, 255, 255, 255]));
        m.select_ellipse(Area { x0: 0, y0: 0, x1: 20, y1: 20 });
        assert!(m.selected(10, 10) && !m.selected(0, 0));
        m.invert_selection();
        assert!(!m.selected(10, 10) && m.selected(0, 0));
        m.select_polygon(&[(2.0, 2.0), (18.0, 2.0), (2.0, 18.0)]);
        assert!(m.selected(4, 4) && !m.selected(16, 16));
        m.checkpoint();
        m.stamp(10.0, 10.0, &Brush::new(20.0, 1.0, 1.0), [0, 0, 0, 255], false);
        assert_eq!(m.layers[0].img.get_pixel(16, 16).0, [255, 255, 255, 255]);
        m.layers[0].img.put_pixel(5, 5, Rgba([10, 10, 10, 255]));
        m.set_selection(None, None);
        m.select_color(0, 0, 0, false);
        assert!(m.selected(15, 15) && !m.selected(4, 4));
        m.add_layer(None, Some(RgbaImage::from_pixel(20, 20, Rgba([128, 128, 128, 255]))));
        m.layers[1].blend = Blend::Multiply;
        assert_eq!(m.flatten().get_pixel(15, 15).0, [128, 128, 128, 255]);

        let mut e = Document::new(40, 40, None);
        e.set_selection(Area::from_points((10.0, 10.0), (20.0, 20.0), 40, 40), None);
        e.fill_selection([255, 0, 0, 255]);
        assert_eq!(e.layers[0].img.get_pixel(15, 15).0, [255, 0, 0, 255]);
        e.grow_selection(3);
        assert!(e.selected(8, 8) && !e.selected(5, 5));
        e.grow_selection(-5);
        assert!(e.selected(15, 15) && !e.selected(11, 11));
        e.select_layer_alpha();
        assert_eq!(e.selection, Some(Area { x0: 10, y0: 10, x1: 20, y1: 20 }));
        assert!(e.layer_from_selection(false));
        assert_eq!(e.layers.len(), 2);
        e.drop_shadow(4, 4, 0.0, [0, 0, 0, 255]);
        assert_eq!(e.layers.len(), 3);
        assert_eq!(e.layers[1].img.get_pixel(22, 22).0[3], 255);
        e.layer_outline(2, [255, 255, 255, 255]);
        assert_eq!(e.layers.len(), 4);
        e.merge_visible();
        assert_eq!(e.layers.len(), 1);
        assert!(e.trim());
        assert_eq!((e.width(), e.height()), (16, 16));
        e.canvas_size(30, 20, (0.0, 0.0), Some([0, 0, 255, 255]));
        assert_eq!((e.width(), e.height()), (30, 20));
        assert_eq!(e.layers[0].img.get_pixel(29, 19).0, [0, 0, 255, 255]);
        e.set_selection(None, None);
        e.transform_layer(0.5, 0.0);
        assert_eq!(e.layers[0].img.get_pixel(0, 0).0[3], 0);
        e.gradient_map([0, 0, 0, 255], [255, 255, 0, 255]);
        e.flatten_all();
        assert_eq!(e.layers.len(), 1);

        let (h, s, l) = rgb_to_hsl(0.2, 0.6, 0.4);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!((r - 0.2).abs() < 1e-4 && (g - 0.6).abs() < 1e-4 && (b - 0.4).abs() < 1e-4);
    }

    #[test]
    fn masks_clipping_adjustments_and_modes() {
        let mut d = Document::new(10, 10, Some([200, 100, 50, 255]));
        d.add_layer(None, Some(RgbaImage::from_pixel(10, 10, Rgba([0, 0, 255, 255]))));
        d.add_mask(1);
        assert_eq!(d.flatten().get_pixel(5, 5).0, [200, 100, 50, 255]);
        d.invert_mask();
        assert_eq!(d.flatten().get_pixel(5, 5).0, [0, 0, 255, 255]);
        d.delete_mask();
        d.layers[1].img = RgbaImage::from_fn(10, 10, |x, _| if x < 5 { Rgba([0, 0, 255, 255]) } else { Rgba([0, 0, 0, 0]) });
        d.add_layer(None, Some(RgbaImage::from_pixel(10, 10, Rgba([0, 255, 0, 255]))));
        d.layers[2].clip = true;
        let f = d.flatten();
        assert_eq!(f.get_pixel(2, 2).0, [0, 255, 0, 255]);
        assert_eq!(f.get_pixel(8, 2).0, [200, 100, 50, 255]);
        d.add_adjustment(Kind::Invert, vec![]);
        assert_eq!(d.flatten().get_pixel(8, 2).0, [55, 155, 205, 255]);
        d.merge_down();
        assert!(d.layers[d.active].adjust.is_none());
        for (b, _) in BLENDS {
            let m = b.mix3([0.3, 0.5, 0.7], [0.6, 0.2, 0.9]);
            assert!(m.iter().all(|v| (-0.001..=1.001).contains(v)), "{b:?}");
        }
        assert_eq!(Blend::Multiply.mix3([0.5; 3], [0.5; 3]), [0.25; 3]);

        let mut s = Document::new(20, 20, Some([255, 255, 255, 255]));
        s.set_selection(Area::from_points((0.0, 0.0), (10.0, 20.0), 20, 20), None);
        let prev = Some(s.sel_mask());
        s.set_selection(Area::from_points((5.0, 0.0), (20.0, 20.0), 20, 20), None);
        s.combine_selection(prev.clone(), 3);
        assert!(s.selected(7, 5) && !s.selected(2, 5) && !s.selected(15, 5));
        s.set_selection(Area::from_points((5.0, 0.0), (20.0, 20.0), 20, 20), None);
        s.combine_selection(prev, 1);
        assert!(s.selected(2, 5) && s.selected(15, 5));
        s.feather_selection(2.0);
        assert!(s.sel_weight(10, 10) > 0.9);
        s.set_selection(None, None);
        s.fill_polygon(&[(2.0, 2.0), (18.0, 2.0), (10.0, 18.0)], [0, 0, 0, 255], Blend::Normal);
        assert_eq!(s.layers[0].img.get_pixel(10, 6).0, [0, 0, 0, 255]);
        assert_eq!(s.layers[0].img.get_pixel(1, 18).0, [255, 255, 255, 255]);
        s.magic_erase(0, 19, 10, true, 1.0);
        assert_eq!(s.layers[0].img.get_pixel(0, 19).0[3], 0);
        s.rotate_canvas(90.0, None);
        assert_eq!((s.width(), s.height()), (20, 20));
        let mut t = Document::new(20, 10, Some([255, 0, 0, 255]));
        t.transform_layer_full(&Xform { tx: 5.0, nearest: true, ..Xform::identity() });
        assert_eq!(t.layers[0].img.get_pixel(2, 5).0[3], 0);
        assert_eq!(t.layers[0].img.get_pixel(8, 5).0, [255, 0, 0, 255]);
        let mut q = Document::new(30, 30, Some([255, 255, 255, 255]));
        q.layers[0].img.put_pixel(15, 15, Rgba([0, 0, 0, 255]));
        q.quick_select(5.0, 5.0, 6.0, 20, false);
        assert!(q.selected(12, 12) && !q.selected(15, 15) && !q.selected(29, 29));
    }
}
