use serde::{Deserialize, Serialize};
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Geom {
    Rect { x: f32, y: f32, w: f32, h: f32, r: f32 },
    Ellipse { cx: f32, cy: f32, rx: f32, ry: f32 },
    Path { nodes: Vec<Node>, closed: bool },
    Text { x: f32, y: f32, text: String, size: f32, family: String, bold: bool },
    Image { x: f32, y: f32, w: f32, h: f32, href: String },
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
}

pub const BLENDS: [(&str, &str); 7] = [("normal", "Normal"), ("multiply", "Produit"), ("screen", "Superposition"), ("overlay", "Incrustation"), ("darken", "Obscurcir"), ("lighten", "Éclaircir"), ("difference", "Différence")];
pub const CAPS: [&str; 3] = ["round", "butt", "square"];
pub const JOINS: [&str; 3] = ["round", "miter", "bevel"];


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    pub width: f32,
    pub height: f32,
    pub background: Option<[u8; 4]>,
    pub items: Vec<Item>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BBox {
    pub min: P,
    pub max: P,
}

impl BBox {
    fn of(points: impl IntoIterator<Item = P>) -> Self {
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
        }
    }

    pub fn bbox
(&self) -> BBox {
        match &self.geom {
            Geom::Rect { x, y, w, h, .. } | Geom::Image { x, y, w, h, .. } => BBox::of([[*x, *y], [x + w, y + h]]),
            Geom::Ellipse { cx, cy, rx, ry } => BBox::of([[cx - rx, cy - ry], [cx + rx, cy + ry]]),
            Geom::Path { nodes, .. } => BBox::of(nodes.iter().flat_map(|n| [n.p, n.cin, n.cout])),
            Geom::Text { x, y, text, size, .. } => {
                let w = text.lines().map(|l| l.chars().count()).max().unwrap_or(1) as f32 * size * 0.55;
                let lines = text.lines().count().max(1) as f32;
                BBox::of([[*x, y - size * 0.8], [x + w, y + size * (lines - 1.0) * 1.2 + size * 0.25]])
            }
        }
    }

    pub fn map(&mut self, f: impl Fn(P) -> P, sx: f32, sy: f32) {
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
            Geom::Path { nodes, .. } => {
                for n in nodes {
                    n.p = f(n.p);
                    n.cin = f(n.cin);
                    n.cout = f(n.cout);
                }
            }
            Geom::Text { x, y, size, .. } => {
                let p = f([*x, *y]);
                *x = p[0];
                *y = p[1];
                *size = (*size * sy.abs()).max(1.0);
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
        let b = self.bbox();
        let c = [(b.min[0] + b.max[0]) / 2.0, (b.min[1] + b.max[1]) / 2.0];
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
}

/// Catmull-Rom tangents: turns any polyline into a smooth curve through the same anchors.
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

/// Ramer-Douglas-Peucker on the anchors; handles are reset to corners.
pub fn simplify(points: &[P], tol: f32) -> Vec<Node> {
    fn rdp(pts: &[P], tol: f32, keep: &mut Vec<bool>, off: usize) {
        if pts.len() < 3 {
            return;
        }
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt().max(1e-6);
        let (mut far, mut idx) = (0.0, 0);
        for (i, p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
            let d = ((b[0] - a[0]) * (a[1] - p[1]) - (a[0] - p[0]) * (b[1] - a[1])).abs() / len;
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
    let steps = (turns * 24.0).max(8.0) as usize;
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

impl Doc {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height, background: Some([255, 255, 255, 255]), items: Vec::new() }
    }

    pub fn to_svg(&self) -> String {
        let mut s = String::new();
        writeln!(
            s,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
            w = self.width,
            h = self.height
        )
        .unwrap();
        if let Some(bg) = self.background {
            writeln!(s, r#"<rect width="100%" height="100%" fill="{}" fill-opacity="{}"/>"#, color(bg), bg[3] as f32 / 255.0).unwrap();
        }
        for (idx, it) in self.items.iter().enumerate().filter(|(_, i)| i.visible) {
            let mut paint = String::new();
            match (it.fill, it.fill2) {
                (Some(a), Some(b)) => {
                    let stops = format!(r#"<stop offset="0" stop-color="{}" stop-opacity="{}"/><stop offset="1" stop-color="{}" stop-opacity="{}"/>"#, color(a), a[3] as f32 / 255.0, color(b), b[3] as f32 / 255.0);
                    if it.radial {
                        writeln!(s, r#"<defs><radialGradient id="g{idx}">{stops}</radialGradient></defs>"#).unwrap();
                    } else {
                        writeln!(s, r#"<defs><linearGradient id="g{idx}" gradientTransform="rotate({} 0.5 0.5)">{stops}</linearGradient></defs>"#, it.grad_angle).unwrap();
                    }
                    write!(paint, r#" fill="url(#g{idx})""#).unwrap();
                }
                (Some(c), None) => write!(paint, r#" fill="{}" fill-opacity="{}""#, color(c), c[3] as f32 / 255.0).unwrap(),
                (None, _) => paint.push_str(r#" fill="none""#),
            }
            if it.shadow || it.blur > 0.0 {
                let mut f = String::new();
                if it.blur > 0.0 {
                    write!(f, r#"<feGaussianBlur stdDeviation="{}" result="b"/>"#, it.blur).unwrap();
                }
                if it.shadow {
                    write!(f, r##"<feDropShadow{} dx="0" dy="6" stdDeviation="8" flood-color="#000000" flood-opacity="0.35"/>"##, if it.blur > 0.0 { r#" in="b""# } else { "" }).unwrap();
                }
                writeln!(s, r#"<defs><filter id="f{idx}" x="-50%" y="-50%" width="200%" height="200%">{f}</filter></defs>"#).unwrap();
                write!(paint, r#" filter="url(#f{idx})""#).unwrap();
            }
            if it.dash > 0.0 && it.stroke.is_some() {
                write!(paint, r#" stroke-dasharray="{} {}""#, it.dash, if it.gap > 0.0 { it.gap } else { it.dash }).unwrap();
            }

            if let Some(c) = it.stroke {
                write!(
                    paint,
                    r#" stroke="{}" stroke-opacity="{}" stroke-width="{}" stroke-linejoin="{}" stroke-linecap="{}""#,
                    color(c),
                    c[3] as f32 / 255.0,
                    it.stroke_w,
                    JOINS[it.join as usize % 3],
                    CAPS[it.cap as usize % 3]
                )
                .unwrap();
                if it.arrow_end && matches!(it.geom, Geom::Path { closed: false, .. }) {
                    writeln!(s, r#"<defs><marker id="m{idx}" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 z" fill="{}"/></marker></defs>"#, color(c)).unwrap();
                    write!(paint, r#" marker-end="url(#m{idx})""#).unwrap();
                }
            }
            if it.blend > 0 {
                write!(paint, r#" style="mix-blend-mode:{}""#, BLENDS[it.blend as usize % BLENDS.len()].0).unwrap();
            }
            if it.opacity < 1.0 {
                write!(paint, r#" opacity="{}""#, it.opacity).unwrap();
            }
            match &it.geom {
                Geom::Rect { x, y, w, h, r } => {
                    writeln!(s, r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}"{paint}/>"#).unwrap()
                }
                Geom::Image { x, y, w, h, href } => {
                    writeln!(s, r#"<image x="{x}" y="{y}" width="{w}" height="{h}" preserveAspectRatio="none" href="{href}"{paint}/>"#).unwrap()
                }
                Geom::Ellipse { cx, cy, rx, ry } => {
                    writeln!(s, r#"<ellipse
 cx="{cx}" cy="{cy}" rx="{rx}" ry="{ry}"{paint}/>"#).unwrap()
                }
                Geom::Path { nodes, closed } => {
                    writeln!(s, r#"<path d="{}"{paint}/>"#, path_d(nodes, *closed)).unwrap()
                }
                Geom::Text { x, y, text, size, family, bold } => {
                    let anchor = ["start", "middle", "end"][it.align as usize % 3];
                    write!(
                        s,
                        r#"<text x="{x}" y="{y}" font-size="{size}" font-family="{}" font-weight="{}" font-style="{}" letter-spacing="{}" text-anchor="{anchor}"{paint}>"#,
                        esc(family),
                        if *bold { "bold" } else { "normal" },
                        if it.italic { "italic" } else { "normal" },
                        it.tracking
                    )
                    .unwrap();
                    let lead = if it.leading > 0.0 { it.leading } else { 1.2 };
                    for (i, line) in text.lines().enumerate() {
                        let dy = if i == 0 { 0.0 } else { size * lead };
                        write!(s, r#"<tspan x="{x}" dy="{dy}">{}</tspan>"#, esc(line)).unwrap();
                    }
                    writeln!(s, "</text>").unwrap();
                }
            }
        }
        s.push_str("</svg>\n");
        s
    }

    /// Every visible item as a thin black outline: shows hidden overlaps and construction.
    pub fn to_svg_outline(&self) -> String {
        let mut d = self.clone();
        d.background = Some([255, 255, 255, 255]);
        for it in &mut d.items {
            it.fill = None;
            it.fill2 = None;
            it.stroke = Some([20, 20, 24, 255]);
            it.stroke_w = 1.0;
            it.shadow = false;
            it.blur = 0.0;
            it.opacity = 1.0;
            it.dash = 0.0;
            it.blend = 0;
        }
        d.to_svg()
    }

    /// Resizes the artboard to the content plus a margin and moves everything inside.
    pub fn fit_to_content(&mut self, margin: f32) -> bool {
        let vis: Vec<BBox> = self.items.iter().filter(|i| i.visible).map(|i| i.bbox()).collect();
        if vis.is_empty() {
            return false;
        }
        let min = [vis.iter().map(|b| b.min[0]).fold(f32::MAX, f32::min), vis.iter().map(|b| b.min[1]).fold(f32::MAX, f32::min)];
        let max = [vis.iter().map(|b| b.max[0]).fold(f32::MIN, f32::max), vis.iter().map(|b| b.max[1]).fold(f32::MIN, f32::max)];
        let d = [margin - min[0], margin - min[1]];
        for it in &mut self.items {
            it.translate(d);
        }
        self.width = (max[0] - min[0] + 2.0 * margin).max(1.0);
        self.height = (max[1] - min[1] + 2.0 * margin).max(1.0);
        true
    }

    /// Union box of several items.
    pub fn bounds_of(&self, idx: &[usize]) -> Option<BBox> {
        let mut it = idx.iter().filter_map(|i| self.items.get(*i)).map(|i| i.bbox());
        let first = it.next()?;
        Some(it.fold(first, |a, b| BBox { min: [a.min[0].min(b.min[0]), a.min[1].min(b.min[1])], max: [a.max[0].max(b.max[0]), a.max[1].max(b.max[1])] }))
    }
    pub fn hit(&self, p: P, tol: f32) -> Option<usize> {
        (0..self.items.len()).rev().find(|&i| self.items[i].visible && !self.items[i].locked && self.items[i].bbox().contains(p, tol))
    }
}

/// Splits every segment of a path in two (De Casteljau), keeping the exact curve.
pub fn add_midpoints(nodes: &mut Vec<Node>, closed: bool) {
    let n = nodes.len();
    if n < 2 {
        return;
    }
    let mut out = Vec::with_capacity(n * 2);
    let lerp = |a: P, b: P| [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let segs = if closed { n } else { n - 1 };
    let mut cur = nodes[0];
    for i in 0..segs {
        let next = nodes[(i + 1) % n];
        let (p0, p1, p2, p3) = (cur.p, cur.cout, next.cin, next.p);
        let (a, b, c) = (lerp(p0, p1), lerp(p1, p2), lerp(p2, p3));
        let (d, e) = (lerp(a, b), lerp(b, c));
        let m = lerp(d, e);
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

/// Corner ⇄ smooth for one node.
pub fn toggle_node(nodes: &mut [Node], i: usize, closed: bool) {
    let n = nodes.len();
    if i >= n {
        return;
    }
    if nodes[i].cin == nodes[i].p && nodes[i].cout == nodes[i].p {
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

pub const SHAPES: [&str; 10] = ["Bulle", "Croix", "Éclair", "Repère", "Nuage", "Coche", "Engrenage", "Goutte", "Lune", "Badge"];

/// Ready-made shapes fitted to a box: (nodes, closed).
pub fn preset_shape(kind: usize, x: f32, y: f32, w: f32, h: f32) -> (Vec<Node>, bool) {
    let at = |u: f32, v: f32| [x + u * w, y + v * h];
    let poly = |pts: &[(f32, f32)]| pts.iter().map(|(u, v)| Node::corner(at(*u, *v))).collect::<Vec<_>>();
    let round = |pts: &[(f32, f32)]| {
        let mut n = poly(pts);
        smooth(&mut n, true);
        n
    };
    let ring = |k: usize, f: &dyn Fn(f32) -> f32| -> Vec<Node> {
        (0..k).map(|i| {
            let a = i as f32 / k as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let r = f(a);
            Node::corner(at(0.5 + a.cos() * 0.5 * r, 0.5 + a.sin() * 0.5 * r))
        }).collect()
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
/// A regular polygon (`inner` = None) or a star (`inner` = ratio of the inner radius), as path nodes.
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
        assert_eq!(e.bounds_of(&[0]).unwrap().min, [110.0, 210.0]);

    }
}
