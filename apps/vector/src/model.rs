use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Write;

pub type P = [f32; 2];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub p: P,
    pub cin: P,
    pub cout: P,
}

impl Node {
    pub fn corner(p: P) -> Self {
        Self { p, cin: p, cout: p }
    }
    pub fn is_corner(&self) -> bool {
        self.cin == self.p && self.cout == self.p
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Geom {
    Rect { x: f32, y: f32, w: f32, h: f32, r: f32 },
    Ellipse { cx: f32, cy: f32, rx: f32, ry: f32 },
    Path { nodes: Vec<Node>, closed: bool },
    Text { x: f32, y: f32, text: String, size: f32, family: String, bold: bool },
    Image { x: f32, y: f32, w: f32, h: f32, href: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    pub t: f32,
    pub c: [u8; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeshPt {
    pub p: P,
    pub c: [u8; 4],
    pub r: f32,
}

fn one() -> f32 {
    1.0
}
fn four() -> f32 {
    4.0
}
fn six() -> f32 {
    6.0
}
fn eight() -> f32 {
    8.0
}
fn freq() -> f32 {
    0.04
}
fn yes() -> bool {
    true
}
fn shadow_c() -> [u8; 4] {
    [0, 0, 0, 89]
}
fn glow_c() -> [u8; 4] {
    [255, 214, 10, 220]
}
fn white_c() -> [u8; 4] {
    [255, 255, 255, 220]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub geom: Geom,
    pub fill: Option<[u8; 4]>,
    pub stroke: Option<[u8; 4]>,
    pub stroke_w: f32,
    pub opacity: f32,
    pub visible: bool,
    #[serde(default)]
    pub fill2: Option<[u8; 4]>,
    #[serde(default)]
    pub grad_angle: f32,
    #[serde(default)]
    pub dash: f32,
    #[serde(default)]
    pub shadow: bool,
    #[serde(default)]
    pub blur: f32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub blend: u8,
    #[serde(default)]
    pub cap: u8,
    #[serde(default)]
    pub join: u8,
    #[serde(default)]
    pub gap: f32,
    #[serde(default)]
    pub arrow_end: bool,
    #[serde(default)]
    pub radial: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub tracking: f32,
    #[serde(default)]
    pub align: u8,
    #[serde(default)]
    pub leading: f32,
    #[serde(default)]
    pub sub: Vec<Vec<Node>>,
    #[serde(default)]
    pub group: u32,
    #[serde(default)]
    pub clip: bool,
    #[serde(default)]
    pub mask: bool,
    #[serde(default)]
    pub layer: usize,
    #[serde(default)]
    pub stops: Vec<Stop>,
    #[serde(default)]
    pub grad_vec: Option<[f32; 4]>,
    #[serde(default)]
    pub mesh: Vec<MeshPt>,
    #[serde(default)]
    pub profile: u8,
    #[serde(default)]
    pub widths: Vec<[f32; 2]>,
    #[serde(default)]
    pub head_start: u8,
    #[serde(default)]
    pub head_end: u8,
    #[serde(default = "one")]
    pub head_scale: f32,
    #[serde(default)]
    pub dash2: f32,
    #[serde(default)]
    pub gap2: f32,
    #[serde(default = "four")]
    pub miter: f32,
    #[serde(default)]
    pub stroke_align: u8,
    #[serde(default)]
    pub sh_x: f32,
    #[serde(default = "six")]
    pub sh_y: f32,
    #[serde(default = "eight")]
    pub sh_blur: f32,
    #[serde(default = "shadow_c")]
    pub sh_color: [u8; 4],
    #[serde(default)]
    pub glow_out: f32,
    #[serde(default = "glow_c")]
    pub glow_out_c: [u8; 4],
    #[serde(default)]
    pub glow_in: f32,
    #[serde(default = "white_c")]
    pub glow_in_c: [u8; 4],
    #[serde(default)]
    pub feather: f32,
    #[serde(default)]
    pub rough: f32,
    #[serde(default = "freq")]
    pub rough_freq: f32,
    #[serde(default)]
    pub grain: f32,
    #[serde(default)]
    pub box_w: f32,
    #[serde(default)]
    pub box_h: f32,
    #[serde(default)]
    pub vertical: bool,
    #[serde(default)]
    pub case: u8,
    #[serde(default)]
    pub deco: u8,
    #[serde(default)]
    pub baseline: f32,
    #[serde(default = "yes")]
    pub kern: bool,
    #[serde(default)]
    pub text_path: Vec<Node>,
    #[serde(default)]
    pub path_offset: f32,
    #[serde(default)]
    pub sym: u32,
}

pub const BLENDS: [(&str, &str); 16] = [
    ("normal", "Normal"),
    ("multiply", "Produit"),
    ("screen", "Superposition"),
    ("overlay", "Incrustation"),
    ("darken", "Obscurcir"),
    ("lighten", "Éclaircir"),
    ("difference", "Différence"),
    ("exclusion", "Exclusion"),
    ("color-dodge", "Densité couleur −"),
    ("color-burn", "Densité couleur +"),
    ("hard-light", "Lumière crue"),
    ("soft-light", "Lumière tamisée"),
    ("hue", "Teinte"),
    ("saturation", "Saturation"),
    ("color", "Couleur"),
    ("luminosity", "Luminosité"),
];
pub const CAPS: [&str; 3] = ["round", "butt", "square"];
pub const JOINS: [&str; 3] = ["round", "miter", "bevel"];
pub const HEADS: [&str; 9] = ["Aucune", "Triangle", "Triangle ouvert", "Flèche fine", "Cercle", "Carré", "Losange", "Barre", "Point"];
pub const PROFILES: [&str; 7] = ["Uniforme", "Lentille", "Pointe à la fin", "Pointe au début", "Goutte", "Vague", "Calligraphie"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Board {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub color: [u8; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub vertical: bool,
    pub pos: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Named {
    pub name: String,
    pub item: Item,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    pub width: f32,
    pub height: f32,
    pub background: Option<[u8; 4]>,
    pub items: Vec<Item>,
    #[serde(default)]
    pub boards: Vec<Board>,
    #[serde(default)]
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub guides: Vec<Guide>,
    #[serde(default)]
    pub symbols: Vec<Symbol>,
    #[serde(default)]
    pub styles: Vec<Named>,
    #[serde(default)]
    pub swatches: Vec<[u8; 4]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BBox {
    pub min: P,
    pub max: P,
}

impl BBox {
    pub fn of(points: impl IntoIterator<Item = P>) -> Self {
        let mut b = BBox { min: [f32::MAX; 2], max: [f32::MIN; 2] };
        for p in points {
            b.min = [b.min[0].min(p[0]), b.min[1].min(p[1])];
            b.max = [b.max[0].max(p[0]), b.max[1].max(p[1])];
        }
        b
    }
    pub fn contains(&self, p: P, tol: f32) -> bool {
        p[0] >= self.min[0] - tol && p[0] <= self.max[0] + tol && p[1] >= self.min[1] - tol && p[1] <= self.max[1] + tol
    }
    pub fn corners(&self) -> [P; 4] {
        [self.min, [self.max[0], self.min[1]], self.max, [self.min[0], self.max[1]]]
    }
    pub fn center(&self) -> P {
        [(self.min[0] + self.max[0]) / 2.0, (self.min[1] + self.max[1]) / 2.0]
    }
    pub fn size(&self) -> P {
        [self.max[0] - self.min[0], self.max[1] - self.min[1]]
    }
    pub fn union(&self, o: &BBox) -> BBox {
        BBox { min: [self.min[0].min(o.min[0]), self.min[1].min(o.min[1])], max: [self.max[0].max(o.max[0]), self.max[1].max(o.max[1])] }
    }
    pub fn overlaps(&self, o: &BBox) -> bool {
        self.max[0] >= o.min[0] && self.min[0] <= o.max[0] && self.max[1] >= o.min[1] && self.min[1] <= o.max[1]
    }
}

pub fn char_w(c: char) -> f32 {
    match c {
        'i' | 'l' | 'j' | '!' | '.' | ',' | '\'' | '|' | ':' | ';' | ' ' => 0.28,
        'f' | 't' | 'r' | 'I' => 0.36,
        'm' | 'w' | 'M' | 'W' => 0.85,
        c if c.is_uppercase() => 0.66,
        _ => 0.55,
    }
}

pub fn text_width(s: &str, size: f32, tracking: f32) -> f32 {
    s.chars().map(|c| char_w(c) * size + tracking).sum()
}

pub fn apply_case(s: &str, case: u8) -> String {
    match case {
        1 => s.to_uppercase(),
        2 => s.to_lowercase(),
        3 => s
            .split(' ')
            .map(|w| {
                let mut c = w.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase()).unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
        _ => s.to_string(),
    }
}

pub fn lerp(a: P, b: P, t: f32) -> P {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

pub fn dist(a: P, b: P) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

pub fn cubic(a: &Node, b: &Node, t: f32) -> P {
    let u = 1.0 - t;
    let q = |i: usize| u * u * u * a.p[i] + 3.0 * u * u * t * a.cout[i] + 3.0 * u * t * t * b.cin[i] + t * t * t * b.p[i];
    [q(0), q(1)]
}

pub fn flatten(nodes: &[Node], closed: bool, steps: usize) -> Vec<P> {
    let n = nodes.len();
    if n == 0 {
        return Vec::new();
    }
    let mut out = vec![nodes[0].p];
    let segs = if closed { n } else { n - 1 };
    for s in 0..segs {
        let (a, b) = (&nodes[s], &nodes[(s + 1) % n]);
        let k = if a.cout == a.p && b.cin == b.p { 1 } else { steps };
        for j in 1..=k {
            out.push(cubic(a, b, j as f32 / k as f32));
        }
    }
    if closed && out.len() > 1 {
        out.pop();
    }
    out
}

pub fn point_in(loops: &[Vec<P>], p: P) -> bool {
    let mut inside = false;
    for l in loops {
        let n = l.len();
        for i in 0..n {
            let (a, b) = (l[i], l[(i + 1) % n]);
            if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]) {
                inside = !inside;
            }
        }
    }
    inside
}

pub fn seg_dist(p: P, a: P, b: P) -> f32 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let l2 = d[0] * d[0] + d[1] * d[1];
    let t = if l2 < 1e-9 { 0.0 } else { (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / l2).clamp(0.0, 1.0) };
    dist(p, [a[0] + d[0] * t, a[1] + d[1] * t])
}

pub fn area(poly: &[P]) -> f32 {
    let n = poly.len();
    (0..n).map(|i| poly[i][0] * poly[(i + 1) % n][1] - poly[(i + 1) % n][0] * poly[i][1]).sum::<f32>() / 2.0
}

pub fn polyline_len(pts: &[P]) -> f32 {
    pts.windows(2).map(|w| dist(w[0], w[1])).sum()
}

pub fn resample(pts: &[P], closed: bool, count: usize) -> Vec<P> {
    let mut path = pts.to_vec();
    if closed && !pts.is_empty() {
        path.push(pts[0]);
    }
    let total = polyline_len(&path);
    if path.len() < 2 || total < 1e-6 || count < 2 {
        return pts.to_vec();
    }
    let n = if closed { count } else { count - 1 };
    let step = total / n as f32;
    let mut out = Vec::with_capacity(count);
    let (mut seg, mut acc) = (0usize, 0.0f32);
    for k in 0..count {
        let target = (k as f32 * step).min(total);
        while seg + 1 < path.len() - 1 && acc + dist(path[seg], path[seg + 1]) < target {
            acc += dist(path[seg], path[seg + 1]);
            seg += 1;
        }
        let l = dist(path[seg], path[seg + 1]).max(1e-6);
        out.push(lerp(path[seg], path[seg + 1], ((target - acc) / l).clamp(0.0, 1.0)));
    }
    out
}

fn normal_at(pts: &[P], i: usize, closed: bool) -> P {
    let n = pts.len();
    let prev = if i == 0 { if closed { pts[n - 1] } else { pts[0] } } else { pts[i - 1] };
    let next = if i + 1 == n { if closed { pts[0] } else { pts[n - 1] } } else { pts[i + 1] };
    let d = [next[0] - prev[0], next[1] - prev[1]];
    let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-6);
    [-d[1] / l, d[0] / l]
}

impl Item {
    pub fn blank() -> Self {
        Item {
            name: String::new(),
            geom: Geom::Path { nodes: Vec::new(), closed: false },
            fill: None,
            stroke: None,
            stroke_w: 1.0,
            opacity: 1.0,
            visible: true,
            fill2: None,
            grad_angle: 90.0,
            dash: 0.0,
            shadow: false,
            blur: 0.0,
            locked: false,
            blend: 0,
            cap: 0,
            join: 0,
            gap: 0.0,
            arrow_end: false,
            radial: false,
            italic: false,
            tracking: 0.0,
            align: 0,
            leading: 0.0,
            sub: Vec::new(),
            group: 0,
            clip: false,
            mask: false,
            layer: 0,
            stops: Vec::new(),
            grad_vec: None,
            mesh: Vec::new(),
            profile: 0,
            widths: Vec::new(),
            head_start: 0,
            head_end: 0,
            head_scale: 1.0,
            dash2: 0.0,
            gap2: 0.0,
            miter: 4.0,
            stroke_align: 0,
            sh_x: 0.0,
            sh_y: 6.0,
            sh_blur: 8.0,
            sh_color: shadow_c(),
            glow_out: 0.0,
            glow_out_c: glow_c(),
            glow_in: 0.0,
            glow_in_c: white_c(),
            feather: 0.0,
            rough: 0.0,
            rough_freq: 0.04,
            grain: 0.0,
            box_w: 0.0,
            box_h: 0.0,
            vertical: false,
            case: 0,
            deco: 0,
            baseline: 0.0,
            kern: true,
            text_path: Vec::new(),
            path_offset: 0.0,
            sym: 0,
        }
    }

    pub fn shown_text(&self) -> String {
        match &self.geom {
            Geom::Text { text, .. } => apply_case(text, self.case),
            _ => String::new(),
        }
    }

    pub fn text_lines(&self) -> Vec<String> {
        let Geom::Text { size, .. } = &self.geom else { return Vec::new() };
        let text = self.shown_text();
        if self.box_w <= 0.0 || self.vertical {
            return text.lines().map(String::from).collect();
        }
        let mut out = Vec::new();
        for para in text.lines() {
            let mut line = String::new();
            for word in para.split(' ') {
                let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if !line.is_empty() && text_width(&cand, *size, self.tracking) > self.box_w {
                    out.push(std::mem::take(&mut line));
                    line = word.to_string();
                } else {
                    line = cand;
                }
            }
            out.push(line);
        }
        out
    }

    pub fn lead(&self) -> f32 {
        if self.leading > 0.0 { self.leading } else { 1.2 }
    }

    pub fn bbox(&self) -> BBox {
        match &self.geom {
            Geom::Rect { x, y, w, h, .. } | Geom::Image { x, y, w, h, .. } => BBox::of([[*x, *y], [x + w, y + h]]),
            Geom::Ellipse { cx, cy, rx, ry } => BBox::of([[cx - rx, cy - ry], [cx + rx, cy + ry]]),
            Geom::Path { nodes, .. } => BBox::of(nodes.iter().chain(self.sub.iter().flatten()).flat_map(|n| [n.p, n.cin, n.cout])),
            Geom::Text { x, y, size, .. } => {
                let y = y + self.baseline;
                if !self.text_path.is_empty() {
                    let b = BBox::of(self.text_path.iter().flat_map(|n| [n.p, n.cin, n.cout]));
                    return BBox { min: [b.min[0], b.min[1] - size * 0.8], max: [b.max[0], b.max[1] + size * 0.25] };
                }
                let lines = self.text_lines();
                let count = lines.len().max(1) as f32;
                if self.vertical {
                    let chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1).max(1) as f32;
                    return BBox::of([[x - (count - 1.0) * size * self.lead() - size * 0.5, y - size * 0.8], [x + size * 0.5, y + (chars - 1.0) * size + size * 0.25]]);
                }
                let wmax = lines.iter().map(|l| text_width(l, *size, self.tracking)).fold(size * 0.3, f32::max);
                let bottom = y + size * (count - 1.0) * self.lead() + size * 0.25;
                if self.box_w > 0.0 {
                    return BBox::of([[*x, y - size * 0.8], [x + self.box_w, bottom.max(y - size * 0.8 + self.box_h)]]);
                }
                let x0 = x - wmax * [0.0, 0.5, 1.0][self.align as usize % 3];
                BBox::of([[x0, y - size * 0.8], [x0 + wmax, bottom]])
            }
        }
    }

    pub fn map(&mut self, f: impl Fn(P) -> P, sx: f32, sy: f32) {
        let mapn = |n: &mut Node| {
            n.p = f(n.p);
            n.cin = f(n.cin);
            n.cout = f(n.cout);
        };
        for s in &mut self.sub {
            s.iter_mut().for_each(mapn);
        }
        self.text_path.iter_mut().for_each(mapn);
        for m in &mut self.mesh {
            m.p = f(m.p);
            m.r *= (sx.abs() * sy.abs()).sqrt();
        }
        match &mut self.geom {
            Geom::Image { x, y, w, h, .. } => {
                let a = f([*x, *y]);
                let b = f([*x + *w, *y + *h]);
                *x = a[0].min(b[0]);
                *y = a[1].min(b[1]);
                *w = (b[0] - a[0]).abs();
                *h = (b[1] - a[1]).abs();
            }
            Geom::Rect { x, y, w, h, r } => {
                let a = f([*x, *y]);
                let b = f([*x + *w, *y + *h]);
                *x = a[0].min(b[0]);
                *y = a[1].min(b[1]);
                *w = (b[0] - a[0]).abs();
                *h = (b[1] - a[1]).abs();
                *r *= sx.abs().min(sy.abs());
            }
            Geom::Ellipse { cx, cy, rx, ry } => {
                let c = f([*cx, *cy]);
                *cx = c[0];
                *cy = c[1];
                *rx *= sx.abs();
                *ry *= sy.abs();
            }
            Geom::Path { nodes, .. } => nodes.iter_mut().for_each(mapn),
            Geom::Text { x, y, size, .. } => {
                let p = f([*x, *y]);
                *x = p[0];
                *y = p[1];
                *size = (*size * sy.abs()).max(1.0);
                self.box_w *= sx.abs();
                self.box_h *= sy.abs();
            }
        }
    }

    pub fn translate(&mut self, d: P) {
        self.map(|p| [p[0] + d[0], p[1] + d[1]], 1.0, 1.0);
    }

    pub fn to_path(&mut self) -> bool {
        const K: f32 = 0.552_284_8;
        let nodes = match self.geom {
            Geom::Rect { x, y, w, h, r } if r <= 0.0 => {
                vec![Node::corner([x, y]), Node::corner([x + w, y]), Node::corner([x + w, y + h]), Node::corner([x, y + h])]
            }
            Geom::Rect { x, y, w, h, r } => {
                let r = r.min(w / 2.0).min(h / 2.0);
                let k = r * K;
                let arc = |a: P, b: P, ca: P, cb: P| [Node { p: a, cin: a, cout: ca }, Node { p: b, cin: cb, cout: b }];
                [
                    arc([x + w - r, y], [x + w, y + r], [x + w - r + k, y], [x + w, y + r - k]),
                    arc([x + w, y + h - r], [x + w - r, y + h], [x + w, y + h - r + k], [x + w - r + k, y + h]),
                    arc([x + r, y + h], [x, y + h - r], [x + r - k, y + h], [x, y + h - r + k]),
                    arc([x, y + r], [x + r, y], [x, y + r - k], [x + r - k, y]),
                ]
                .concat()
            }
            Geom::Ellipse { cx, cy, rx, ry } => {
                let (kx, ky) = (rx * K, ry * K);
                vec![
                    Node { p: [cx, cy - ry], cin: [cx - kx, cy - ry], cout: [cx + kx, cy - ry] },
                    Node { p: [cx + rx, cy], cin: [cx + rx, cy - ky], cout: [cx + rx, cy + ky] },
                    Node { p: [cx, cy + ry], cin: [cx + kx, cy + ry], cout: [cx - kx, cy + ry] },
                    Node { p: [cx - rx, cy], cin: [cx - rx, cy + ky], cout: [cx - rx, cy - ky] },
                ]
            }
            Geom::Path { .. } => return true,
            _ => return false,
        };
        self.geom = Geom::Path { nodes, closed: true };
        true
    }

    pub fn rotate(&mut self, deg: f32) -> bool {
        let c = self.bbox().center();
        self.rotate_about(deg, c)
    }

    pub fn rotate_about(&mut self, deg: f32, c: P) -> bool {
        if !self.to_path() {
            return false;
        }
        let (s, co) = deg.to_radians().sin_cos();
        self.map(|p| [c[0] + (p[0] - c[0]) * co - (p[1] - c[1]) * s, c[1] + (p[0] - c[0]) * s + (p[1] - c[1]) * co], 1.0, 1.0);
        true
    }

    pub fn affine(&mut self, m: [f32; 6]) -> bool {
        let uniform_axis = m[1].abs() < 1e-6 && m[2].abs() < 1e-6;
        if !uniform_axis && !self.to_path() {
            return false;
        }
        let f = |p: P| [m[0] * p[0] + m[2] * p[1] + m[4], m[1] * p[0] + m[3] * p[1] + m[5]];
        let (sx, sy) = ((m[0] * m[0] + m[1] * m[1]).sqrt(), (m[2] * m[2] + m[3] * m[3]).sqrt());
        self.map(f, sx, sy);
        true
    }

    pub fn path_nodes(&self) -> Option<(Vec<Node>, bool)> {
        let mut c = self.clone();
        if !c.to_path() {
            return None;
        }
        match c.geom {
            Geom::Path { nodes, closed } => Some((nodes, closed)),
            _ => None,
        }
    }

    pub fn loops(&self) -> Vec<Vec<P>> {
        match &self.geom {
            Geom::Image { x, y, w, h, .. } => vec![vec![[*x, *y], [x + w, *y], [x + w, y + h], [*x, y + h]]],
            Geom::Text { .. } => {
                let b = self.bbox();
                vec![b.corners().to_vec()]
            }
            _ => {
                let Some((nodes, _)) = self.path_nodes() else { return Vec::new() };
                let mut out = vec![flatten(&nodes, true, 16)];
                out.extend(self.sub.iter().map(|s| flatten(s, true, 16)));
                out.retain(|l| l.len() >= 3);
                out
            }
        }
    }

    pub fn hit_test(&self, p: P, tol: f32) -> bool {
        if !self.bbox().contains(p, tol + self.stroke_w) {
            return false;
        }
        if matches!(self.geom, Geom::Text { .. } | Geom::Image { .. }) {
            return true;
        }
        let Some((nodes, closed)) = self.path_nodes() else { return false };
        let filled = self.fill.is_some() || !self.mesh.is_empty() || self.clip || self.mask;
        let loops = self.loops();
        if (filled || closed) && self.fill.is_some() && point_in(&loops, p) {
            return true;
        }
        if filled && point_in(&loops, p) {
            return true;
        }
        let reach = tol + if self.stroke.is_some() { self.stroke_w / 2.0 } else { 0.0 };
        let line = flatten(&nodes, closed, 16);
        let mut segs: Vec<(P, P)> = line.windows(2).map(|w| (w[0], w[1])).collect();
        if closed && line.len() > 1 {
            segs.push((line[line.len() - 1], line[0]));
        }
        for s in &self.sub {
            let l = flatten(s, true, 16);
            for i in 0..l.len() {
                segs.push((l[i], l[(i + 1) % l.len()]));
            }
        }
        segs.iter().any(|(a, b)| seg_dist(p, *a, *b) <= reach)
    }

    pub fn width_at(&self, t: f32, angle: f32) -> f32 {
        let m = if !self.widths.is_empty() {
            let mut w = self.widths.clone();
            w.sort_by(|a, b| a[0].total_cmp(&b[0]));
            if t <= w[0][0] {
                w[0][1]
            } else if t >= w[w.len() - 1][0] {
                w[w.len() - 1][1]
            } else {
                let k = w.windows(2).position(|p| t >= p[0][0] && t <= p[1][0]).unwrap_or(0);
                let (a, b) = (w[k], w[k + 1]);
                a[1] + (b[1] - a[1]) * ((t - a[0]) / (b[0] - a[0]).max(1e-6))
            }
        } else {
            use std::f32::consts::PI;
            match self.profile {
                1 => (PI * t).sin(),
                2 => (1.0 - t).powf(0.7),
                3 => t.powf(0.7),
                4 => (PI * t.sqrt()).sin(),
                5 => 0.6 + 0.4 * (6.0 * PI * t).sin(),
                6 => (angle - PI / 4.0).sin().abs() * 0.85 + 0.15,
                _ => 1.0,
            }
        };
        (m.max(0.02)) * self.stroke_w
    }

    pub fn variable_stroke(&self) -> bool {
        self.stroke.is_some() && (self.profile != 0 || !self.widths.is_empty()) && matches!(self.geom, Geom::Path { .. })
    }

    pub fn width_outline(&self) -> Vec<Vec<P>> {
        let Geom::Path { nodes, closed } = &self.geom else { return Vec::new() };
        let raw = flatten(nodes, *closed, 24);
        if raw.len() < 2 {
            return Vec::new();
        }
        let pts = resample(&raw, *closed, raw.len().max(120));
        let mut path = pts.clone();
        if *closed {
            path.push(pts[0]);
        }
        let total = polyline_len(&path).max(1e-6);
        let (mut left, mut right) = (Vec::new(), Vec::new());
        let mut acc = 0.0;
        for i in 0..path.len() {
            if i > 0 {
                acc += dist(path[i - 1], path[i]);
            }
            let n = normal_at(&path, i, false);
            let ang = n[1].atan2(n[0]) + std::f32::consts::FRAC_PI_2;
            let w = self.width_at(acc / total, ang) / 2.0;
            left.push([path[i][0] + n[0] * w, path[i][1] + n[1] * w]);
            right.push([path[i][0] - n[0] * w, path[i][1] - n[1] * w]);
        }
        if *closed {
            vec![left, right]
        } else {
            right.reverse();
            left.extend(right);
            vec![left]
        }
    }

    pub fn copy_look(&mut self, src: &Item) {
        let keep = self.clone();
        *self = src.clone();
        self.name = keep.name;
        self.geom = keep.geom;
        self.visible = keep.visible;
        self.locked = keep.locked;
        self.sub = keep.sub;
        self.group = keep.group;
        self.clip = keep.clip;
        self.mask = keep.mask;
        self.layer = keep.layer;
        self.mesh = keep.mesh;
        self.widths = keep.widths;
        self.text_path = keep.text_path;
        self.box_w = keep.box_w;
        self.box_h = keep.box_h;
        self.vertical = keep.vertical;
        self.sym = keep.sym;
        if !matches!(self.geom, Geom::Text { .. }) {
            self.italic = keep.italic;
            self.tracking = keep.tracking;
            self.align = keep.align;
            self.leading = keep.leading;
            self.case = keep.case;
            self.deco = keep.deco;
            self.baseline = keep.baseline;
            self.kern = keep.kern;
            self.path_offset = keep.path_offset;
        }
    }
}

pub fn smooth(nodes: &mut [Node], closed: bool) {
    let n = nodes.len();
    if n < 3 {
        return;
    }
    let pts: Vec<P> = nodes.iter().map(|n| n.p).collect();
    for i in 0..n {
        let prev = if i == 0 { if closed { pts[n - 1] } else { pts[0] } } else { pts[i - 1] };
        let next = if i + 1 == n { if closed { pts[0] } else { pts[n - 1] } } else { pts[i + 1] };
        let t = [(next[0] - prev[0]) / 6.0, (next[1] - prev[1]) / 6.0];
        nodes[i].cin = [pts[i][0] - t[0], pts[i][1] - t[1]];
        nodes[i].cout = [pts[i][0] + t[0], pts[i][1] + t[1]];
    }
}

pub fn simplify(points: &[P], tol: f32) -> Vec<Node> {
    fn rdp(pts: &[P], tol: f32, keep: &mut Vec<bool>, off: usize) {
        if pts.len() < 3 {
            return;
        }
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt().max(1e-6);
        let (mut far, mut idx) = (0.0, 0);
        for (i, p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
            let d = if len < 1e-3 { dist(*p, a) } else { ((b[0] - a[0]) * (a[1] - p[1]) - (a[0] - p[0]) * (b[1] - a[1])).abs() / len };
            if d > far {
                (far, idx) = (d, i);
            }
        }
        if far > tol {
            keep[off + idx] = true;
            rdp(&pts[..=idx], tol, keep, off);
            rdp(&pts[idx..], tol, keep, off + idx);
        }
    }
    if points.len() < 3 {
        return points.iter().map(|p| Node::corner(*p)).collect();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    rdp(points, tol, &mut keep, 0);
    points.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| Node::corner(*p)).collect()
}

pub fn simplify_loop(points: &[P], tol: f32) -> Vec<P> {
    if points.len() < 4 {
        return points.to_vec();
    }
    let far = (1..points.len()).max_by(|a, b| dist(points[0], points[*a]).total_cmp(&dist(points[0], points[*b]))).unwrap_or(1);
    let mut a: Vec<P> = points[..=far].to_vec();
    let mut b: Vec<P> = points[far..].to_vec();
    b.push(points[0]);
    let sa = simplify(&a, tol);
    let sb = simplify(&b, tol);
    a = sa.iter().map(|n| n.p).collect();
    b = sb.iter().map(|n| n.p).collect();
    a.pop();
    b.pop();
    a.extend(b);
    a
}

pub fn fit(points: &[P], closed: bool, max_angle: f32) -> Vec<Node> {
    let n = points.len();
    let mut out: Vec<Node> = points.iter().map(|p| Node::corner(*p)).collect();
    if n < 3 {
        return out;
    }
    for i in 0..n {
        if !closed && (i == 0 || i + 1 == n) {
            continue;
        }
        let (prev, p, next) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
        let a = [p[0] - prev[0], p[1] - prev[1]];
        let b = [next[0] - p[0], next[1] - p[1]];
        let (la, lb) = ((a[0] * a[0] + a[1] * a[1]).sqrt(), (b[0] * b[0] + b[1] * b[1]).sqrt());
        if la < 1e-6 || lb < 1e-6 {
            continue;
        }
        let cos = ((a[0] * b[0] + a[1] * b[1]) / (la * lb)).clamp(-1.0, 1.0);
        if cos.acos().to_degrees() < max_angle {
            let t = [next[0] - prev[0], next[1] - prev[1]];
            let lt = (t[0] * t[0] + t[1] * t[1]).sqrt().max(1e-6);
            let u = [t[0] / lt, t[1] / lt];
            out[i].cin = [p[0] - u[0] * la / 3.0, p[1] - u[1] * la / 3.0];
            out[i].cout = [p[0] + u[0] * lb / 3.0, p[1] + u[1] * lb / 3.0];
        }
    }
    out
}

pub fn reverse(nodes: &mut [Node]) {
    nodes.reverse();
    for n in nodes {
        std::mem::swap(&mut n.cin, &mut n.cout);
    }
}

pub fn arrow(a: P, b: P, width: f32) -> Vec<Node> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (ux, uy) = (dx / len, dy / len);
    let (nx, ny) = (-uy, ux);
    let head = (width * 3.0).min(len * 0.6);
    let shaft = width / 2.0;
    let base = [b[0] - ux * head, b[1] - uy * head];
    [
        [a[0] + nx * shaft, a[1] + ny * shaft],
        [base[0] + nx * shaft, base[1] + ny * shaft],
        [base[0] + nx * width * 1.6, base[1] + ny * width * 1.6],
        b,
        [base[0] - nx * width * 1.6, base[1] - ny * width * 1.6],
        [base[0] - nx * shaft, base[1] - ny * shaft],
        [a[0] - nx * shaft, a[1] - ny * shaft],
    ]
    .into_iter()
    .map(Node::corner)
    .collect()
}

pub fn heart(x: f32, y: f32, w: f32, h: f32) -> Vec<Node> {
    let at = |u: f32, v: f32| [x + u * w, y + v * h];
    vec![
        Node { p: at(0.5, 0.25), cin: at(0.38, 0.0), cout: at(0.62, 0.0) },
        Node { p: at(1.0, 0.3), cin: at(1.0, 0.02), cout: at(1.0, 0.6) },
        Node { p: at(0.5, 1.0), cin: at(0.75, 0.75), cout: at(0.25, 0.75) },
        Node { p: at(0.0, 0.3), cin: at(0.0, 0.6), cout: at(0.0, 0.02) },
    ]
}

pub fn spiral(cx: f32, cy: f32, r: f32, turns: f32) -> Vec<Node> {
    let steps = (turns.abs() * 24.0).max(8.0) as usize;
    let mut nodes: Vec<Node> = (0..=steps)
        .map(|k| {
            let t = k as f32 / steps as f32;
            let a = t * turns * std::f32::consts::TAU;
            Node::corner([cx + a.cos() * r * t, cy + a.sin() * r * t])
        })
        .collect();
    smooth(&mut nodes, false);
    nodes
}

pub fn arc(a: P, b: P, slope: f32, concave: bool) -> Vec<Node> {
    let c = if concave { [b[0], a[1]] } else { [a[0], b[1]] };
    vec![Node { p: a, cin: a, cout: lerp(a, c, slope) }, Node { p: b, cin: lerp(b, c, slope), cout: b }]
}

pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            out.push(if k <= c.len() { T[(n >> (18 - 6 * k) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

pub fn unbase64(s: &str) -> Vec<u8> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let (mut out, mut acc, mut bits) = (Vec::with_capacity(s.len() * 3 / 4), 0u32, 0);
    for v in s.bytes().filter_map(val) {
        acc = acc << 6 | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

fn color(c: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub fn path_d(nodes: &[Node], closed: bool) -> String {
    let mut d = String::new();
    let Some(first) = nodes.first() else { return d };
    write!(d, "M{} {}", first.p[0], first.p[1]).unwrap();
    let seg = |d: &mut String, a: &Node, b: &Node| {
        write!(d, " C{} {} {} {} {} {}", a.cout[0], a.cout[1], b.cin[0], b.cin[1], b.p[0], b.p[1]).unwrap();
    };
    for w in nodes.windows(2) {
        seg(&mut d, &w[0], &w[1]);
    }
    if closed && nodes.len() > 1 {
        seg(&mut d, nodes.last().unwrap(), first);
        d.push_str(" Z");
    }
    d
}

fn poly_d(loops: &[Vec<P>]) -> String {
    let mut d = String::new();
    for l in loops.iter().filter(|l| l.len() > 1) {
        write!(d, "M{} {}", l[0][0], l[0][1]).unwrap();
        for p in &l[1..] {
            write!(d, " L{} {}", p[0], p[1]).unwrap();
        }
        d.push_str(" Z ");
    }
    d
}

fn head_shape(kind: u8, c: &str) -> String {
    match kind {
        2 => format!(r#"<path d="M1 1 L9 5 L1 9" fill="none" stroke="{c}" stroke-width="1.6"/>"#),
        3 => format!(r#"<path d="M0 0 L10 5 L0 10 L3 5 z" fill="{c}"/>"#),
        4 => format!(r#"<circle cx="5" cy="5" r="4" fill="{c}"/>"#),
        5 => format!(r#"<rect x="1" y="1" width="8" height="8" fill="{c}"/>"#),
        6 => format!(r#"<path d="M5 0 L10 5 L5 10 L0 5 z" fill="{c}"/>"#),
        7 => format!(r#"<rect x="4" y="0" width="2" height="10" fill="{c}"/>"#),
        8 => format!(r#"<circle cx="5" cy="5" r="2.2" fill="{c}"/>"#),
        _ => format!(r#"<path d="M0 0 L10 5 L0 10 z" fill="{c}"/>"#),
    }
}

impl Doc {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height, background: Some([255, 255, 255, 255]), items: Vec::new(), boards: Vec::new(), layers: Vec::new(), guides: Vec::new(), symbols: Vec::new(), styles: Vec::new(), swatches: Vec::new() }
    }

    pub fn all_boards(&self) -> Vec<Board> {
        let mut v = vec![Board { name: "Plan de travail 1".into(), x: 0.0, y: 0.0, w: self.width, h: self.height }];
        v.extend(self.boards.iter().cloned());
        v
    }

    pub fn layer_flags(&self, it: &Item) -> (bool, bool) {
        self.layers.get(it.layer).map_or((true, false), |l| (l.visible, l.locked))
    }

    pub fn shown(&self, i: usize) -> bool {
        self.items.get(i).is_some_and(|it| it.visible && self.layer_flags(it).0)
    }

    pub fn editable(&self, i: usize) -> bool {
        self.shown(i) && !self.items[i].locked && !self.layer_flags(&self.items[i]).1
    }

    pub fn to_svg(&self) -> String {
        self.export_svg(0.0, 0.0, self.width, self.height, 1.0)
    }

    pub fn export_svg(&self, x: f32, y: f32, w: f32, h: f32, scale: f32) -> String {
        let mut s = String::new();
        writeln!(s, r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{x} {y} {w} {h}">"#, w * scale, h * scale).unwrap();
        if let Some(bg) = self.background {
            writeln!(s, r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{}" fill-opacity="{}"/>"#, color(bg), bg[3] as f32 / 255.0).unwrap();
        }
        self.body(&mut s);
        s.push_str("</svg>\n");
        s
    }

    pub fn view_svg(&self, pw: f32, ph: f32, m: [f32; 6], cell: f32) -> String {
        let mut s = String::new();
        writeln!(s, r#"<svg xmlns="http://www.w3.org/2000/svg" width="{pw}" height="{ph}" viewBox="0 0 {pw} {ph}">"#).unwrap();
        writeln!(
            s,
            r##"<defs><pattern id="ck" patternUnits="userSpaceOnUse" width="{c2}" height="{c2}"><rect width="{c2}" height="{c2}" fill="#ffffff"/><rect width="{cell}" height="{cell}" fill="#cccccc"/><rect x="{cell}" y="{cell}" width="{cell}" height="{cell}" fill="#cccccc"/></pattern></defs>"##,
            c2 = cell * 2.0
        )
        .unwrap();
        writeln!(s, r#"<g transform="matrix({} {} {} {} {} {})">"#, m[0], m[1], m[2], m[3], m[4], m[5]).unwrap();
        for b in self.all_boards() {
            if self.background.is_none_or(|c| c[3] < 255) {
                writeln!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="url(#ck)"/>"#, b.x, b.y, b.w, b.h).unwrap();
            }
            if let Some(bg) = self.background {
                writeln!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" fill-opacity="{}"/>"#, b.x, b.y, b.w, b.h, color(bg), bg[3] as f32 / 255.0).unwrap();
            }
        }
        self.body(&mut s);
        s.push_str("</g></svg>\n");
        s
    }

    fn body(&self, s: &mut String) {
        let mut clips = HashSet::new();
        let mut masks = HashSet::new();
        for (idx, it) in self.items.iter().enumerate().filter(|(_, i)| i.group != 0) {
            if it.clip && clips.insert(it.group) {
                writeln!(s, r#"<defs><clipPath id="k{}">{}</clipPath></defs>"#, it.group, geom_el(it, idx, "")).unwrap();
            }
            if it.mask && masks.insert(it.group) {
                let mut inner = String::new();
                let mut m = it.clone();
                m.mask = false;
                m.group = 0;
                item_svg(&m, idx, &mut inner, &HashSet::new(), &HashSet::new());
                writeln!(s, r#"<defs><mask id="q{}" maskUnits="userSpaceOnUse" x="-100000" y="-100000" width="200000" height="200000">{inner}</mask></defs>"#, it.group).unwrap();
            }
        }
        for (idx, it) in self.items.iter().enumerate() {
            if self.shown(idx) && !it.clip && !it.mask {
                item_svg(it, idx, s, &clips, &masks);
            }
        }
    }

    pub fn outlined(&self) -> Doc {
        let mut d = self.clone();
        d.background = Some([255, 255, 255, 255]);
        for it in &mut d.items {
            let keep = Item { name: String::new(), geom: it.geom.clone(), sub: it.sub.clone(), layer: it.layer, group: it.group, visible: it.visible, text_path: it.text_path.clone(), box_w: it.box_w, box_h: it.box_h, vertical: it.vertical, align: it.align, leading: it.leading, ..Item::blank() };
            *it = Item { stroke: Some([20, 20, 24, 255]), stroke_w: 1.0, ..keep };
        }
        d
    }

    pub fn to_svg_outline(&self) -> String {
        self.outlined().to_svg()
    }

    pub fn fit_to_content(&mut self, margin: f32) -> bool {
        let vis: Vec<BBox> = (0..self.items.len()).filter(|i| self.shown(*i)).map(|i| self.items[i].bbox()).collect();
        if vis.is_empty() {
            return false;
        }
        let min = [vis.iter().map(|b| b.min[0]).fold(f32::MAX, f32::min), vis.iter().map(|b| b.min[1]).fold(f32::MAX, f32::min)];
        let max = [vis.iter().map(|b| b.max[0]).fold(f32::MIN, f32::max), vis.iter().map(|b| b.max[1]).fold(f32::MIN, f32::max)];
        let d = [margin - min[0], margin - min[1]];
        for it in &mut self.items {
            it.translate(d);
        }
        for b in &mut self.boards {
            b.x += d[0];
            b.y += d[1];
        }
        self.width = (max[0] - min[0] + 2.0 * margin).max(1.0);
        self.height = (max[1] - min[1] + 2.0 * margin).max(1.0);
        true
    }

    pub fn bounds_of(&self, idx: &[usize]) -> Option<BBox> {
        let mut it = idx.iter().filter_map(|i| self.items.get(*i)).map(|i| i.bbox());
        let first = it.next()?;
        Some(it.fold(first, |a, b| a.union(&b)))
    }

    pub fn hit(&self, p: P, tol: f32) -> Option<usize> {
        (0..self.items.len()).rev().find(|&i| self.editable(i) && !self.items[i].clip && !self.items[i].mask && self.items[i].hit_test(p, tol))
    }
}

fn fill_paint(it: &Item, idx: usize, s: &mut String) -> String {
    let stops: Vec<Stop> = if !it.stops.is_empty() {
        it.stops.clone()
    } else {
        match (it.fill, it.fill2) {
            (Some(a), Some(b)) => vec![Stop { t: 0.0, c: a }, Stop { t: 1.0, c: b }],
            _ => Vec::new(),
        }
    };
    if it.fill.is_none() {
        return r#" fill="none""#.into();
    }
    if stops.len() < 2 {
        let c = it.fill.unwrap();
        return format!(r#" fill="{}" fill-opacity="{}""#, color(c), c[3] as f32 / 255.0);
    }
    let st: String = stops.iter().map(|k| format!(r#"<stop offset="{}" stop-color="{}" stop-opacity="{}"/>"#, k.t.clamp(0.0, 1.0), color(k.c), k.c[3] as f32 / 255.0)).collect();
    if it.radial {
        let geo = it.grad_vec.map(|v| format!(r#" cx="{}" cy="{}" r="{}""#, v[0], v[1], dist([v[0], v[1]], [v[2], v[3]]).max(0.01))).unwrap_or_default();
        writeln!(s, r#"<defs><radialGradient id="g{idx}"{geo}>{st}</radialGradient></defs>"#).unwrap();
    } else {
        let geo = match it.grad_vec {
            Some(v) => format!(r#" x1="{}" y1="{}" x2="{}" y2="{}""#, v[0], v[1], v[2], v[3]),
            None => format!(r#" gradientTransform="rotate({} 0.5 0.5)""#, it.grad_angle),
        };
        writeln!(s, r#"<defs><linearGradient id="g{idx}"{geo}>{st}</linearGradient></defs>"#).unwrap();
    }
    format!(r#" fill="url(#g{idx})""#)
}

fn stroke_paint(it: &Item, width: f32) -> String {
    let Some(c) = it.stroke else { return String::new() };
    let mut p = format!(
        r#" stroke="{}" stroke-opacity="{}" stroke-width="{}" stroke-linejoin="{}" stroke-linecap="{}""#,
        color(c),
        c[3] as f32 / 255.0,
        width,
        JOINS[it.join as usize % 3],
        CAPS[it.cap as usize % 3]
    );
    if it.join == 1 && (it.miter - 4.0).abs() > 1e-3 {
        write!(p, r#" stroke-miterlimit="{}""#, it.miter.max(1.0)).unwrap();
    }
    if it.dash > 0.0 {
        if it.dash2 > 0.0 {
            write!(p, r#" stroke-dasharray="{} {} {} {}""#, it.dash, if it.gap > 0.0 { it.gap } else { it.dash }, it.dash2, if it.gap2 > 0.0 { it.gap2 } else { it.dash2 }).unwrap();
        } else {
            write!(p, r#" stroke-dasharray="{} {}""#, it.dash, if it.gap > 0.0 { it.gap } else { it.dash }).unwrap();
        }
    }
    p
}

fn filter_def(it: &Item, idx: usize, s: &mut String) -> bool {
    let any = it.shadow || it.blur > 0.0 || it.glow_out > 0.0 || it.glow_in > 0.0 || it.feather > 0.0 || it.rough > 0.0 || it.grain > 0.0;
    if !any {
        return false;
    }
    let mut f = String::new();
    let mut cur = "SourceGraphic".to_string();
    let mut n = 0;
    let mut next = |f: &mut String, prim: String, cur: &mut String| {
        n += 1;
        let r = format!("r{n}");
        f.push_str(&prim.replace("{in}", cur).replace("{out}", &r));
        *cur = r;
    };
    if it.rough > 0.0 {
        next(&mut f, format!(r#"<feTurbulence type="fractalNoise" baseFrequency="{}" numOctaves="2" seed="{}" result="noise"/>"#, it.rough_freq.max(0.001), idx + 1), &mut cur.clone());
        next(&mut f, format!(r#"<feDisplacementMap in="{{in}}" in2="noise" scale="{}" xChannelSelector="R" yChannelSelector="G" result="{{out}}"/>"#, it.rough), &mut cur);
    }
    if it.blur > 0.0 {
        next(&mut f, format!(r#"<feGaussianBlur in="{{in}}" stdDeviation="{}" result="{{out}}"/>"#, it.blur), &mut cur);
    }
    if it.feather > 0.0 {
        f.push_str(&format!(r#"<feMorphology in="SourceAlpha" operator="erode" radius="{}" result="fe1"/><feGaussianBlur in="fe1" stdDeviation="{}" result="fe2"/>"#, it.feather / 2.0, it.feather / 2.0));
        next(&mut f, r#"<feComposite in="{in}" in2="fe2" operator="in" result="{out}"/>"#.into(), &mut cur);
    }
    if it.glow_in > 0.0 {
        let c = it.glow_in_c;
        f.push_str(&format!(
            r#"<feFlood flood-color="{}" flood-opacity="{}" result="gi1"/><feComposite in="gi1" in2="SourceAlpha" operator="out" result="gi2"/><feGaussianBlur in="gi2" stdDeviation="{}" result="gi3"/><feComposite in="gi3" in2="SourceAlpha" operator="in" result="gi4"/>"#,
            color(c),
            c[3] as f32 / 255.0,
            it.glow_in
        ));
        next(&mut f, r#"<feMerge result="{out}"><feMergeNode in="{in}"/><feMergeNode in="gi4"/></feMerge>"#.into(), &mut cur);
    }
    if it.grain > 0.0 {
        f.push_str(&format!(
            r#"<feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="{}" result="gn1"/><feColorMatrix in="gn1" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 {} 0" result="gn2"/><feComposite in="gn2" in2="SourceAlpha" operator="in" result="gn3"/>"#,
            idx + 7,
            it.grain * 2.0
        ));
        next(&mut f, r#"<feMerge result="{out}"><feMergeNode in="{in}"/><feMergeNode in="gn3"/></feMerge>"#.into(), &mut cur);
    }
    if it.shadow {
        let c = it.sh_color;
        next(&mut f, format!(r#"<feDropShadow in="{{in}}" dx="{}" dy="{}" stdDeviation="{}" flood-color="{}" flood-opacity="{}" result="{{out}}"/>"#, it.sh_x, it.sh_y, it.sh_blur / 2.0, color(c), c[3] as f32 / 255.0), &mut cur);
    }
    if it.glow_out > 0.0 {
        let c = it.glow_out_c;
        f.push_str(&format!(
            r#"<feMorphology in="SourceAlpha" operator="dilate" radius="{}" result="go1"/><feGaussianBlur in="go1" stdDeviation="{}" result="go2"/><feFlood flood-color="{}" flood-opacity="{}" result="go3"/><feComposite in="go3" in2="go2" operator="in" result="go4"/>"#,
            it.glow_out / 3.0,
            it.glow_out / 2.0,
            color(c),
            c[3] as f32 / 255.0
        ));
        next(&mut f, r#"<feMerge result="{out}"><feMergeNode in="go4"/><feMergeNode in="{in}"/></feMerge>"#.into(), &mut cur);
    }
    writeln!(s, r#"<defs><filter id="f{idx}" x="-50%" y="-50%" width="200%" height="200%">{f}</filter></defs>"#).unwrap();
    true
}

fn geom_el(it: &Item, idx: usize, paint: &str) -> String {
    match &it.geom {
        Geom::Rect { x, y, w, h, r } => format!(r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}"{paint}/>"#),
        Geom::Image { x, y, w, h, href } => format!(r#"<image x="{x}" y="{y}" width="{w}" height="{h}" preserveAspectRatio="none" href="{href}"{paint}/>"#),
        Geom::Ellipse { cx, cy, rx, ry } => format!(r#"<ellipse cx="{cx}" cy="{cy}" rx="{rx}" ry="{ry}"{paint}/>"#),
        Geom::Path { nodes, closed } => {
            let mut d = path_d(nodes, *closed);
            for s in &it.sub {
                d.push(' ');
                d.push_str(&path_d(s, true));
            }
            let rule = if it.sub.is_empty() { "" } else { r#" fill-rule="evenodd""# };
            format!(r#"<path d="{d}"{rule}{paint}/>"#)
        }
        Geom::Text { x, y, size, family, bold, .. } => {
            let y = y + it.baseline;
            let lines = it.text_lines();
            let mut s = String::new();
            let anchor = if it.vertical || !it.text_path.is_empty() { "start" } else { ["start", "middle", "end"][it.align as usize % 3] };
            let deco = match it.deco {
                1 => r#" text-decoration="underline""#,
                2 => r#" text-decoration="line-through""#,
                _ => "",
            };
            let kern = if it.kern { "" } else { r#" font-kerning="none""# };
            write!(
                s,
                r#"<text x="{x}" y="{y}" font-size="{size}" font-family="{}" font-weight="{}" font-style="{}" letter-spacing="{}" text-anchor="{anchor}"{deco}{kern}{paint}>"#,
                esc(family),
                if *bold { "bold" } else { "normal" },
                if it.italic { "italic" } else { "normal" },
                it.tracking
            )
            .unwrap();
            if !it.text_path.is_empty() {
                write!(s, r##"<textPath href="#tp{idx}" startOffset="{}%">{}</textPath>"##, it.path_offset, esc(&lines.join(" "))).unwrap();
            } else if it.vertical {
                for (li, line) in lines.iter().enumerate() {
                    let cx = x - li as f32 * size * it.lead();
                    for (ci, ch) in line.chars().enumerate() {
                        write!(s, r#"<tspan x="{}" y="{}" text-anchor="middle">{}</tspan>"#, cx, y + ci as f32 * size, esc(&ch.to_string())).unwrap();
                    }
                }
            } else {
                let tx = if it.box_w > 0.0 { x + it.box_w * [0.0, 0.5, 1.0][it.align as usize % 3] } else { *x };
                for (i, line) in lines.iter().enumerate() {
                    let dy = if i == 0 { 0.0 } else { size * it.lead() };
                    write!(s, r#"<tspan x="{tx}" dy="{dy}">{}</tspan>"#, esc(line)).unwrap();
                }
            }
            s.push_str("</text>");
            s
        }
    }
}

fn item_svg(it: &Item, idx: usize, s: &mut String, clips: &HashSet<u32>, masks: &HashSet<u32>) {
    if !it.text_path.is_empty() {
        writeln!(s, r#"<defs><path id="tp{idx}" d="{}"/></defs>"#, path_d(&it.text_path, false)).unwrap();
    }
    let fill = fill_paint(it, idx, s);
    let mut outer = String::new();
    if filter_def(it, idx, s) {
        write!(outer, r#" filter="url(#f{idx})""#).unwrap();
    }
    if it.blend > 0 {
        write!(outer, r#" style="mix-blend-mode:{}""#, BLENDS[it.blend as usize % BLENDS.len()].0).unwrap();
    }
    if it.opacity < 1.0 {
        write!(outer, r#" opacity="{}""#, it.opacity).unwrap();
    }
    if it.group != 0 && clips.contains(&it.group) {
        write!(outer, r#" clip-path="url(#k{})""#, it.group).unwrap();
    }
    if it.group != 0 && masks.contains(&it.group) {
        write!(outer, r#" mask="url(#q{})""#, it.group).unwrap();
    }
    let open = matches!(it.geom, Geom::Path { closed: false, .. });
    let mut markers = String::new();
    if let Some(c) = it.stroke.filter(|_| open && !it.variable_stroke()) {
        let end = if it.head_end > 0 { it.head_end } else if it.arrow_end { 1 } else { 0 };
        let ms = 5.0 * it.head_scale.max(0.1);
        for (kind, id, attr) in [(end, "m", "marker-end"), (it.head_start, "ms", "marker-start")] {
            if kind > 0 {
                writeln!(s, r#"<defs><marker id="{id}{idx}" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="{ms}" markerHeight="{ms}" orient="auto-start-reverse">{}</marker></defs>"#, head_shape(kind, &color(c))).unwrap();
                write!(markers, r#" {attr}="url(#{id}{idx})""#).unwrap();
            }
        }
    }
    let mut els: Vec<String> = Vec::new();
    if it.variable_stroke() {
        els.push(geom_el(it, idx, &fill));
        let c = it.stroke.unwrap();
        els.push(format!(r#"<path d="{}" fill-rule="evenodd" fill="{}" fill-opacity="{}"/>"#, poly_d(&it.width_outline()), color(c), c[3] as f32 / 255.0));
    } else if it.stroke.is_some() && it.stroke_align == 1 && !open {
        els.push(geom_el(it, idx, &fill));
        writeln!(s, r#"<defs><clipPath id="si{idx}">{}</clipPath></defs>"#, geom_el(it, idx, "")).unwrap();
        els.push(geom_el(it, idx, &format!(r#" fill="none"{} clip-path="url(#si{idx})""#, stroke_paint(it, it.stroke_w * 2.0))));
    } else if it.stroke.is_some() && it.stroke_align == 2 && !open {
        els.push(geom_el(it, idx, &format!(r#" fill="none"{}"#, stroke_paint(it, it.stroke_w * 2.0))));
        els.push(geom_el(it, idx, &fill));
    } else {
        els.push(geom_el(it, idx, &format!("{fill}{}{markers}", stroke_paint(it, it.stroke_w))));
    }
    if !it.mesh.is_empty() {
        writeln!(s, r#"<defs><clipPath id="mc{idx}">{}</clipPath></defs>"#, geom_el(it, idx, "")).unwrap();
        let mut g = format!(r#"<g clip-path="url(#mc{idx})">"#);
        for (k, m) in it.mesh.iter().enumerate() {
            writeln!(s, r#"<defs><radialGradient id="mg{idx}_{k}"><stop offset="0" stop-color="{c}" stop-opacity="{a}"/><stop offset="1" stop-color="{c}" stop-opacity="0"/></radialGradient></defs>"#, c = color(m.c), a = m.c[3] as f32 / 255.0).unwrap();
            write!(g, r#"<circle cx="{}" cy="{}" r="{}" fill="url(#mg{idx}_{k})"/>"#, m.p[0], m.p[1], m.r.max(0.5)).unwrap();
        }
        g.push_str("</g>");
        els.push(g);
    }
    if els.len() == 1 && !outer.is_empty() {
        let el = els.pop().unwrap();
        let cut = if el.ends_with("/>") { el.len() - 2 } else { el.find('>').unwrap_or(el.len()) };
        writeln!(s, "{}{outer}{}", &el[..cut], &el[cut..]).unwrap();
    } else if els.len() == 1 {
        writeln!(s, "{}", els[0]).unwrap();
    } else {
        writeln!(s, "<g{outer}>{}</g>", els.concat()).unwrap();
    }
}

pub fn add_midpoints(nodes: &mut Vec<Node>, closed: bool) {
    let n = nodes.len();
    if n < 2 {
        return;
    }
    let mut out = Vec::with_capacity(n * 2);
    let mid = |a: P, b: P| [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let segs = if closed { n } else { n - 1 };
    let mut cur = nodes[0];
    for i in 0..segs {
        let next = nodes[(i + 1) % n];
        let (p0, p1, p2, p3) = (cur.p, cur.cout, next.cin, next.p);
        let (a, b, c) = (mid(p0, p1), mid(p1, p2), mid(p2, p3));
        let (d, e) = (mid(a, b), mid(b, c));
        let m = mid(d, e);
        out.push(Node { p: cur.p, cin: cur.cin, cout: a });
        out.push(Node { p: m, cin: d, cout: e });
        cur = Node { p: next.p, cin: c, cout: next.cout };
    }
    if closed {
        out[0].cin = cur.cin;
    } else {
        out.push(cur);
    }
    *nodes = out;
}

pub fn split_segment(nodes: &mut Vec<Node>, seg: usize, t: f32) -> usize {
    let n = nodes.len();
    let (a, b) = (nodes[seg], nodes[(seg + 1) % n]);
    let (q0, q1, q2) = (lerp(a.p, a.cout, t), lerp(a.cout, b.cin, t), lerp(b.cin, b.p, t));
    let (r0, r1) = (lerp(q0, q1, t), lerp(q1, q2, t));
    let m = lerp(r0, r1, t);
    nodes[seg].cout = q0;
    nodes[(seg + 1) % n].cin = q2;
    nodes.insert(seg + 1, Node { p: m, cin: r0, cout: r1 });
    seg + 1
}

pub fn nearest_on(nodes: &[Node], closed: bool, p: P) -> Option<(usize, f32, f32)> {
    let n = nodes.len();
    if n < 2 {
        return None;
    }
    let segs = if closed { n } else { n - 1 };
    let mut best: Option<(usize, f32, f32)> = None;
    for s in 0..segs {
        let (a, b) = (&nodes[s], &nodes[(s + 1) % n]);
        for k in 0..=40 {
            let t = k as f32 / 40.0;
            let d = dist(cubic(a, b, t), p);
            if best.is_none_or(|x| d < x.2) {
                best = Some((s, t, d));
            }
        }
    }
    let (s, t0, _) = best?;
    let (a, b) = (&nodes[s], &nodes[(s + 1) % n]);
    let mut bt = (t0, dist(cubic(a, b, t0), p));
    for k in -10..=10 {
        let t = (t0 + k as f32 / 400.0).clamp(0.0, 1.0);
        let d = dist(cubic(a, b, t), p);
        if d < bt.1 {
            bt = (t, d);
        }
    }
    Some((s, bt.0, bt.1))
}

pub fn path_t(nodes: &[Node], closed: bool, seg: usize, t: f32) -> f32 {
    let n = nodes.len();
    let segs = if closed { n } else { n.saturating_sub(1) };
    let lens: Vec<f32> = (0..segs).map(|s| polyline_len(&flatten(&[nodes[s], nodes[(s + 1) % n]], false, 16))).collect();
    let total: f32 = lens.iter().sum::<f32>().max(1e-6);
    (lens[..seg].iter().sum::<f32>() + lens.get(seg).copied().unwrap_or(0.0) * t) / total
}

pub fn toggle_node(nodes: &mut [Node], i: usize, closed: bool) {
    let n = nodes.len();
    if i >= n {
        return;
    }
    if nodes[i].is_corner() {
        let prev = if i == 0 { if closed { nodes[n - 1].p } else { nodes[0].p } } else { nodes[i - 1].p };
        let next = if i + 1 == n { if closed { nodes[0].p } else { nodes[n - 1].p } } else { nodes[i + 1].p };
        let t = [(next[0] - prev[0]) / 6.0, (next[1] - prev[1]) / 6.0];
        let p = nodes[i].p;
        nodes[i].cin = [p[0] - t[0], p[1] - t[1]];
        nodes[i].cout = [p[0] + t[0], p[1] + t[1]];
    } else {
        nodes[i] = Node::corner(nodes[i].p);
    }
}

pub fn zigzag(nodes: &[Node], closed: bool, size: f32, ridges: usize, round: bool, wave: bool) -> Vec<Node> {
    let pts = flatten(nodes, closed, 24);
    let mut count = (ridges.max(1) * 2).max(4);
    if !closed {
        count += 1;
    }
    let r = resample(&pts, closed, count);
    let mut out: Vec<Node> = r
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let n = normal_at(&r, i, closed);
            let sgn = if i % 2 == 0 { 1.0 } else { -1.0 };
            let amp = if !closed && (i == 0 || i + 1 == r.len()) { 0.0 } else { size * sgn };
            Node::corner([p[0] + n[0] * amp, p[1] + n[1] * amp])
        })
        .collect();
    if round || wave {
        smooth(&mut out, closed);
    }
    out
}

pub fn roughen(nodes: &[Node], closed: bool, size: f32, detail: f32, round: bool, seed: u32) -> Vec<Node> {
    let pts = flatten(nodes, closed, 24);
    let mut path = pts.clone();
    if closed {
        path.push(pts[0]);
    }
    let count = ((polyline_len(&path) / 100.0 * detail) as usize).clamp(4, 4000);
    let r = resample(&pts, closed, count);
    let mut state = seed.wrapping_mul(2_654_435_761).max(1);
    let mut rnd = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    let mut out: Vec<Node> = r.iter().map(|p| Node::corner([p[0] + rnd() * size, p[1] + rnd() * size])).collect();
    if round {
        smooth(&mut out, closed);
    }
    out
}

pub fn pucker(nodes: &[Node], closed: bool, amount: f32) -> Vec<Node> {
    let mut n = nodes.to_vec();
    add_midpoints(&mut n, closed);
    let pts: Vec<P> = n.iter().map(|k| k.p).collect();
    let c = BBox::of(pts.iter().copied()).center();
    let mut moved: Vec<P> = pts.clone();
    for (i, p) in moved.iter_mut().enumerate() {
        if i % 2 == 1 {
            *p = lerp(*p, c, amount);
        }
    }
    let mut out: Vec<Node> = moved.iter().map(|p| Node::corner(*p)).collect();
    let len = out.len();
    for i in (1..len).step_by(2) {
        let prev = moved[i - 1];
        let next = moved[(i + 1) % len];
        let t = [(next[0] - prev[0]) / 4.0, (next[1] - prev[1]) / 4.0];
        out[i].cin = [moved[i][0] - t[0], moved[i][1] - t[1]];
        out[i].cout = [moved[i][0] + t[0], moved[i][1] + t[1]];
    }
    out
}

pub fn twist(nodes: &[Node], closed: bool, deg: f32) -> Vec<Node> {
    let mut n = nodes.to_vec();
    add_midpoints(&mut n, closed);
    add_midpoints(&mut n, closed);
    let c = BBox::of(n.iter().map(|k| k.p)).center();
    let rmax = n.iter().map(|k| dist(k.p, c)).fold(1e-3, f32::max);
    let rot = |p: P| {
        let a = (deg * (1.0 - dist(p, c) / rmax).max(0.0)).to_radians();
        let (s, co) = a.sin_cos();
        [c[0] + (p[0] - c[0]) * co - (p[1] - c[1]) * s, c[1] + (p[0] - c[0]) * s + (p[1] - c[1]) * co]
    };
    n.iter().map(|k| Node { p: rot(k.p), cin: rot(k.cin), cout: rot(k.cout) }).collect()
}

pub fn round_corners(nodes: &[Node], closed: bool, r: f32) -> Vec<Node> {
    const K: f32 = 0.552_284_8;
    let n = nodes.len();
    let mut out = Vec::new();
    for i in 0..n {
        let k = nodes[i];
        if !k.is_corner() || (!closed && (i == 0 || i + 1 == n)) || n < 3 {
            out.push(k);
            continue;
        }
        let (prev, next) = (nodes[(i + n - 1) % n].p, nodes[(i + 1) % n].p);
        let (dp, dn) = (dist(prev, k.p), dist(next, k.p));
        if dp < 1e-3 || dn < 1e-3 {
            out.push(k);
            continue;
        }
        let a = lerp(k.p, prev, (r / dp).min(0.5));
        let b = lerp(k.p, next, (r / dn).min(0.5));
        out.push(Node { p: a, cin: a, cout: lerp(a, k.p, K) });
        out.push(Node { p: b, cin: lerp(b, k.p, K), cout: b });
    }
    out
}

pub fn blend_items(a: &Item, b: &Item, steps: usize) -> Vec<Item> {
    let (Some((na, ca)), Some((nb, cb))) = (a.path_nodes(), b.path_nodes()) else { return Vec::new() };
    let closed = ca && cb;
    let (pa, pb) = (flatten(&na, closed, 16), flatten(&nb, closed, 16));
    let count = pa.len().max(pb.len()).clamp(16, 400);
    let (ra, rb) = (resample(&pa, closed, count), resample(&pb, closed, count));
    let mix = |x: Option<[u8; 4]>, y: Option<[u8; 4]>, t: f32| match (x, y) {
        (Some(x), Some(y)) => Some(std::array::from_fn(|k| (x[k] as f32 + (y[k] as f32 - x[k] as f32) * t).round() as u8)),
        (x, y) => if t < 0.5 { x } else { y },
    };
    (1..=steps)
        .map(|k| {
            let t = k as f32 / (steps + 1) as f32;
            let pts: Vec<P> = ra.iter().zip(&rb).map(|(p, q)| lerp(*p, *q, t)).collect();
            let mut nodes = simplify(&pts, 0.3);
            if closed {
                nodes.pop();
            }
            let pts: Vec<P> = nodes.iter().map(|n| n.p).collect();
            Item {
                name: format!("Fusion {k}"),
                geom: Geom::Path { nodes: fit(&pts, closed, 40.0), closed },
                fill: mix(a.fill, b.fill, t),
                stroke: mix(a.stroke, b.stroke, t),
                stroke_w: a.stroke_w + (b.stroke_w - a.stroke_w) * t,
                opacity: a.opacity + (b.opacity - a.opacity) * t,
                layer: a.layer,
                ..Item::blank()
            }
        })
        .collect()
}

pub const SHAPES: [&str; 10] = ["Bulle", "Croix", "Éclair", "Repère", "Nuage", "Coche", "Engrenage", "Goutte", "Lune", "Badge"];

pub fn preset_shape(kind: usize, x: f32, y: f32, w: f32, h: f32) -> (Vec<Node>, bool) {
    let at = |u: f32, v: f32| [x + u * w, y + v * h];
    let poly = |pts: &[(f32, f32)]| pts.iter().map(|(u, v)| Node::corner(at(*u, *v))).collect::<Vec<_>>();
    let round = |pts: &[(f32, f32)]| {
        let mut n = poly(pts);
        smooth(&mut n, true);
        n
    };
    let ring = |k: usize, f: &dyn Fn(f32) -> f32| -> Vec<Node> {
        (0..k)
            .map(|i| {
                let a = i as f32 / k as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                let r = f(a);
                Node::corner(at(0.5 + a.cos() * 0.5 * r, 0.5 + a.sin() * 0.5 * r))
            })
            .collect()
    };
    match kind {
        0 => (poly(&[(0.0, 0.0), (1.0, 0.0), (1.0, 0.75), (0.45, 0.75), (0.2, 1.0), (0.25, 0.75), (0.0, 0.75)]), true),
        1 => (poly(&[(0.35, 0.0), (0.65, 0.0), (0.65, 0.35), (1.0, 0.35), (1.0, 0.65), (0.65, 0.65), (0.65, 1.0), (0.35, 1.0), (0.35, 0.65), (0.0, 0.65), (0.0, 0.35), (0.35, 0.35)]), true),
        2 => (poly(&[(0.6, 0.0), (0.15, 0.55), (0.45, 0.55), (0.35, 1.0), (0.85, 0.4), (0.55, 0.4), (0.75, 0.0)]), true),
        3 => {
            let mut n = vec![
                Node { p: at(0.5, 0.0), cin: at(0.2, 0.0), cout: at(0.8, 0.0) },
                Node { p: at(1.0, 0.38), cin: at(1.0, 0.15), cout: at(1.0, 0.6) },
                Node::corner(at(0.5, 1.0)),
                Node { p: at(0.0, 0.38), cin: at(0.0, 0.6), cout: at(0.0, 0.15) },
            ];
            n[2].cin = at(0.62, 0.8);
            n[2].cout = at(0.38, 0.8);
            (n, true)
        }
        4 => (round(&[(0.1, 0.75), (0.05, 0.55), (0.2, 0.4), (0.3, 0.2), (0.5, 0.15), (0.65, 0.25), (0.8, 0.25), (0.95, 0.45), (0.9, 0.7), (0.6, 0.8), (0.35, 0.8)]), true),
        5 => (poly(&[(0.05, 0.55), (0.38, 0.88), (0.95, 0.15)]), false),
        6 => (ring(48, &|a| if ((a * 8.0 / std::f32::consts::TAU * 2.0).rem_euclid(2.0)) < 1.0 { 1.0 } else { 0.78 }), true),
        7 => (round(&[(0.5, 0.0), (0.82, 0.55), (0.75, 0.88), (0.5, 1.0), (0.25, 0.88), (0.18, 0.55)]), true),
        8 => {
            let mut pts = Vec::new();
            for i in 0..=16 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 16.0 * std::f32::consts::PI;
                pts.push((0.5 + a.cos() * 0.5, 0.5 + a.sin() * 0.5));
            }
            for i in (0..=16).rev() {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 16.0 * std::f32::consts::PI;
                pts.push((0.5 + a.cos() * 0.22, 0.5 + a.sin() * 0.5));
            }
            let mut n = poly(&pts);
            smooth(&mut n, true);
            (n, true)
        }
        _ => (ring(32, &|a| if ((a * 16.0 / std::f32::consts::TAU).rem_euclid(1.0)) < 0.5 { 1.0 } else { 0.84 }), true),
    }
}

pub fn polygon(cx: f32, cy: f32, r: f32, sides: u32, inner: Option<f32>) -> Vec<Node> {
    let sides = sides.max(3);
    let points = if inner.is_some() { sides * 2 } else { sides };
    (0..points)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::TAU / points as f32;
            let rr = match inner {
                Some(ratio) if k % 2 == 1 => r * ratio,
                _ => r,
            };
            Node::corner([cx + a.cos() * rr, cy + a.sin() * rr])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_and_transforms() {
        let mut d = Doc::new(200.0, 100.0);
        d.items.push(Item {
            name: "r".into(),
            geom: Geom::Rect { x: 10.0, y: 10.0, w: 50.0, h: 20.0, r: 4.0 },
            fill: Some([255, 0, 0, 255]),
            stroke: None,
            stroke_w: 1.0,
            opacity: 1.0,
            visible: true,
            ..Item::blank()
        });
        d.items.push(Item {
            name: "p".into(),
            geom: Geom::Path { nodes: vec![Node::corner([0.0, 0.0]), Node::corner([10.0, 0.0]), Node::corner([10.0, 10.0])], closed: true },
            fill: None,
            stroke: Some([0, 0, 0, 255]),
            stroke_w: 2.0,
            opacity: 0.5,
            visible: true,
            ..Item::blank()
        });
        let svg = d.to_svg();
        assert!(svg.contains(r##"<rect x="10" y="10" width="50" height="20" rx="4" fill="#ff0000""##));
        assert!(svg.contains("M0 0 C0 0 10 0 10 0 C10 0 10 10 10 10 C10 10 0 0 0 0 Z"));
        assert!(svg.contains(r#"opacity="0.5""#));

        let anchor = [10.0, 10.0];
        d.items[0].map(|p| [anchor[0] + (p[0] - anchor[0]) * 2.0, anchor[1] + (p[1] - anchor[1]) * 2.0], 2.0, 2.0);
        assert_eq!(d.items[0].bbox(), BBox { min: [10.0, 10.0], max: [110.0, 50.0] });
        assert_eq!(d.hit([100.0, 40.0], 0.0), Some(0));
        assert_eq!(d.hit([5.0, 5.0], 0.0), Some(1));
        assert_eq!(d.hit([190.0, 90.0], 0.0), None);

        let hex = polygon(0.0, 0.0, 10.0, 6, None);
        assert_eq!(hex.len(), 6);
        assert!((hex[0].p[1] + 10.0).abs() < 1e-4);
        let star = polygon(0.0, 0.0, 10.0, 5, Some(0.5));
        assert_eq!(star.len(), 10);
        let d1 = (star[1].p[0].powi(2) + star[1].p[1].powi(2)).sqrt();
        assert!((d1 - 5.0).abs() < 1e-3);

        let mut e = Item { geom: Geom::Ellipse { cx: 50.0, cy: 50.0, rx: 20.0, ry: 10.0 }, ..Item::blank() };
        assert!(e.rotate(90.0));
        let b = e.bbox();
        assert!((b.max[1] - b.min[1] - 40.0).abs() < 2.0, "{b:?}");
        let mut t = Item { geom: Geom::Text { x: 0.0, y: 0.0, text: "a".into(), size: 10.0, family: "x".into(), bold: false }, ..Item::blank() };
        assert!(!t.rotate(45.0));

        let line: Vec<P> = (0..50).map(|k| [k as f32, if k == 25 { 10.0 } else { 0.0 }]).collect();
        assert_eq!(simplify(&line, 1.0).len(), 5);
        assert_eq!(arrow([0.0, 0.0], [100.0, 0.0], 10.0)[3].p, [100.0, 0.0]);
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(unbase64("TWFu"), b"Man");
        assert_eq!(unbase64("TWE="), b"Ma");

        d.items[0].fill2 = Some([0, 0, 255, 255]);
        d.items[0].shadow = true;
        d.items[1].dash = 4.0;
        let svg = d.to_svg();
        assert!(svg.contains(r#"fill="url(#g0)""#) && svg.contains("feDropShadow") && svg.contains("stroke-dasharray"));
        let opt = resvg::usvg::Options::default();
        assert!(resvg::usvg::Tree::from_str(&svg, &opt).is_ok());
        let mut nodes = vec![Node::corner([0.0, 0.0]), Node::corner([10.0, 0.0]), Node::corner([10.0, 10.0])];
        add_midpoints(&mut nodes, true);
        assert_eq!(nodes.len(), 6);
        assert_eq!(nodes[1].p, [5.0, 0.0]);
        toggle_node(&mut nodes, 1, true);
        assert_eq!(nodes[1].cin, nodes[1].p);
        toggle_node(&mut nodes, 1, true);
        assert_ne!(nodes[1].cin, nodes[1].p);
        for k in 0..SHAPES.len() {
            let (n, _) = preset_shape(k, 0.0, 0.0, 100.0, 100.0);
            assert!(n.len() >= 3, "{k}");
        }
        let mut e = Doc::new(500.0, 500.0);
        e.items.push(Item { geom: Geom::Rect { x: 100.0, y: 200.0, w: 50.0, h: 20.0, r: 0.0 }, fill: Some([0; 4]), radial: true, fill2: Some([255; 4]), blend: 1, ..Item::blank() });
        e.items.push(Item { geom: Geom::Path { nodes: vec![Node::corner([0.0, 0.0]), Node::corner([10.0, 0.0])], closed: false }, stroke: Some([0; 4]), arrow_end: true, cap: 1, gap: 3.0, dash: 6.0, ..Item::blank() });
        e.items.push(Item { geom: Geom::Text { x: 0.0, y: 50.0, text: "a\nb".into(), size: 10.0, family: "Arial".into(), bold: false }, italic: true, align: 1, tracking: 2.0, leading: 2.0, ..Item::blank() });
        let svg = e.to_svg();
        assert!(svg.contains("radialGradient") && svg.contains("mix-blend-mode:multiply") && svg.contains("marker-end") && svg.contains("stroke-dasharray=\"6 3\"") && svg.contains("italic") && svg.contains("text-anchor=\"middle\"") && svg.contains("dy=\"20\""));
        assert!(resvg::usvg::Tree::from_str(&svg, &opt).is_ok());
        assert!(resvg::usvg::Tree::from_str(&e.to_svg_outline(), &opt).is_ok());
        assert!(e.fit_to_content(10.0));
        assert_eq!(e.bounds_of(&[0]).unwrap().min[1], 210.0);
    }

    #[test]
    fn new_features_render_and_old_files_load() {
        let old = r#"{"width":100,"height":100,"background":null,"items":[{"name":"a","geom":{"Rect":{"x":0,"y":0,"w":10,"h":10,"r":0}},"fill":[1,2,3,255],"stroke":null,"stroke_w":1,"opacity":1,"visible":true,"shadow":true}]}"#;
        let d: Doc = serde_json::from_str(old).unwrap();
        assert_eq!(d.items[0].sh_y, 6.0);
        assert_eq!(d.items[0].head_scale, 1.0);
        assert!(d.items[0].kern);
        let opt = resvg::usvg::Options::default();
        let mut e = Doc::new(400.0, 400.0);
        e.boards.push(Board { name: "B".into(), x: 500.0, y: 0.0, w: 200.0, h: 200.0 });
        let sq = vec![Node::corner([0.0, 0.0]), Node::corner([100.0, 0.0]), Node::corner([100.0, 100.0]), Node::corner([0.0, 100.0])];
        let hole = vec![Node::corner([30.0, 30.0]), Node::corner([60.0, 30.0]), Node::corner([60.0, 60.0])];
        e.items.push(Item { geom: Geom::Path { nodes: sq.clone(), closed: true }, sub: vec![hole], fill: Some([9, 9, 9, 255]), stops: vec![Stop { t: 0.0, c: [255; 4] }, Stop { t: 0.5, c: [0, 0, 0, 255] }, Stop { t: 1.0, c: [255, 0, 0, 255] }], grad_vec: Some([0.0, 0.0, 1.0, 1.0]), glow_out: 6.0, glow_in: 4.0, feather: 2.0, rough: 3.0, grain: 0.2, shadow: true, mesh: vec![MeshPt { p: [50.0, 50.0], c: [0, 255, 0, 255], r: 40.0 }], ..Item::blank() });
        e.items.push(Item { geom: Geom::Path { nodes: vec![Node::corner([0.0, 200.0]), Node::corner([200.0, 220.0])], closed: false }, stroke: Some([0, 0, 0, 255]), stroke_w: 8.0, profile: 1, ..Item::blank() });
        e.items.push(Item { geom: Geom::Path { nodes: vec![Node::corner([0.0, 300.0]), Node::corner([200.0, 300.0])], closed: false }, stroke: Some([0, 0, 0, 255]), head_start: 4, head_end: 6, dash: 4.0, dash2: 1.0, ..Item::blank() });
        e.items.push(Item { geom: Geom::Rect { x: 0.0, y: 0.0, w: 50.0, h: 50.0, r: 0.0 }, group: 3, clip: true, ..Item::blank() });
        e.items.push(Item { geom: Geom::Ellipse { cx: 40.0, cy: 40.0, rx: 30.0, ry: 30.0 }, group: 3, fill: Some([0, 0, 255, 255]), stroke: Some([0; 4]), stroke_align: 1, ..Item::blank() });
        e.items.push(Item { geom: Geom::Text { x: 10.0, y: 350.0, text: "un texte long qui doit se replier".into(), size: 20.0, family: "Arial".into(), bold: false }, box_w: 120.0, case: 1, deco: 1, fill: Some([0, 0, 0, 255]), ..Item::blank() });
        e.items.push(Item { geom: Geom::Text { x: 10.0, y: 50.0, text: "courbe".into(), size: 20.0, family: "Arial".into(), bold: false }, text_path: sq, fill: Some([0, 0, 0, 255]), ..Item::blank() });
        let svg = e.to_svg();
        assert!(svg.contains("clipPath") && svg.contains("textPath") && svg.contains("UN TEXTE") && svg.contains("feMorphology") && svg.contains("marker-start") && svg.contains("evenodd"));
        assert!(e.items[5].text_lines().len() > 1);
        resvg::usvg::Tree::from_str(&svg, &opt).unwrap();
        resvg::usvg::Tree::from_str(&e.view_svg(800.0, 600.0, [1.0, 0.0, 0.0, 1.0, 10.0, 10.0], 8.0), &opt).unwrap();
        assert!(e.items[1].width_outline()[0].len() > 10);
        let z = zigzag(&polygon(0.0, 0.0, 50.0, 4, None), true, 5.0, 10, false, false);
        assert_eq!(z.len(), 20);
        assert_eq!(round_corners(&polygon(0.0, 0.0, 50.0, 4, None), true, 5.0).len(), 8);
        assert!(pucker(&polygon(0.0, 0.0, 50.0, 4, None), true, 0.4).len() == 8);
        assert!(twist(&polygon(0.0, 0.0, 50.0, 4, None), true, 90.0).len() == 16);
        assert!(roughen(&polygon(0.0, 0.0, 50.0, 4, None), true, 3.0, 20.0, true, 1).len() >= 4);
        let a = Item { geom: Geom::Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0, r: 0.0 }, fill: Some([0, 0, 0, 255]), ..Item::blank() };
        let b = Item { geom: Geom::Ellipse { cx: 100.0, cy: 0.0, rx: 10.0, ry: 10.0 }, fill: Some([255, 255, 255, 255]), ..Item::blank() };
        let bl = blend_items(&a, &b, 3);
        assert_eq!(bl.len(), 3);
        assert_eq!(bl[1].fill, Some([128, 128, 128, 255]));
        let mut nodes = vec![Node::corner([0.0, 0.0]), Node::corner([10.0, 0.0])];
        let (s, t, _) = nearest_on(&nodes, false, [4.0, 1.0]).unwrap();
        split_segment(&mut nodes, s, t);
        assert_eq!(nodes.len(), 3);
        assert!((nodes[1].p[0] - 4.0).abs() < 0.2);
        assert_eq!(apply_case("bonjour le monde", 3), "Bonjour Le Monde");
        assert!(fit(&resample(&flatten(&polygon(0.0, 0.0, 50.0, 40, None), true, 4), true, 40), true, 40.0).iter().all(|n| !n.is_corner()));
    }
}
