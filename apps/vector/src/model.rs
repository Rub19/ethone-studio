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
}

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
    pub fn bbox(&self) -> BBox {
        match &self.geom {
            Geom::Rect { x, y, w, h, .. } => BBox::of([[*x, *y], [x + w, y + h]]),
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
        for it in self.items.iter().filter(|i| i.visible) {
            let mut paint = String::new();
            match it.fill {
                Some(c) => write!(paint, r#" fill="{}" fill-opacity="{}""#, color(c), c[3] as f32 / 255.0).unwrap(),
                None => paint.push_str(r#" fill="none""#),
            }
            if let Some(c) = it.stroke {
                write!(
                    paint,
                    r#" stroke="{}" stroke-opacity="{}" stroke-width="{}" stroke-linejoin="round" stroke-linecap="round""#,
                    color(c),
                    c[3] as f32 / 255.0,
                    it.stroke_w
                )
                .unwrap();
            }
            if it.opacity < 1.0 {
                write!(paint, r#" opacity="{}""#, it.opacity).unwrap();
            }
            match &it.geom {
                Geom::Rect { x, y, w, h, r } => {
                    writeln!(s, r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}"{paint}/>"#).unwrap()
                }
                Geom::Ellipse { cx, cy, rx, ry } => {
                    writeln!(s, r#"<ellipse cx="{cx}" cy="{cy}" rx="{rx}" ry="{ry}"{paint}/>"#).unwrap()
                }
                Geom::Path { nodes, closed } => {
                    writeln!(s, r#"<path d="{}"{paint}/>"#, path_d(nodes, *closed)).unwrap()
                }
                Geom::Text { x, y, text, size, family, bold } => {
                    write!(
                        s,
                        r#"<text x="{x}" y="{y}" font-size="{size}" font-family="{}" font-weight="{}"{paint}>"#,
                        esc(family),
                        if *bold { "bold" } else { "normal" }
                    )
                    .unwrap();
                    for (i, line) in text.lines().enumerate() {
                        let dy = if i == 0 { 0.0 } else { size * 1.2 };
                        write!(s, r#"<tspan x="{x}" dy="{dy}">{}</tspan>"#, esc(line)).unwrap();
                    }
                    writeln!(s, "</text>").unwrap();
                }
            }
        }
        s.push_str("</svg>\n");
        s
    }

    pub fn hit(&self, p: P, tol: f32) -> Option<usize> {
        (0..self.items.len()).rev().find(|&i| self.items[i].visible && self.items[i].bbox().contains(p, tol))
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
        });
        d.items.push(Item {
            name: "p".into(),
            geom: Geom::Path { nodes: vec![Node::corner([0.0, 0.0]), Node::corner([10.0, 0.0]), Node::corner([10.0, 10.0])], closed: true },
            fill: None,
            stroke: Some([0, 0, 0, 255]),
            stroke_w: 2.0,
            opacity: 0.5,
            visible: true,
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
    }
}
