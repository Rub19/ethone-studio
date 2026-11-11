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
}

impl Ease {
    pub const ALL: [(Ease, &'static str); 5] = [
        (Ease::Linear, "Linéaire"),
        (Ease::Smooth, "Douce"),
        (Ease::In, "Accélère"),
        (Ease::Out, "Ralentit"),
        (Ease::Hold, "Maintien"),
    ];
    fn apply(self, t: f32) -> f32 {
        match self {
            Ease::Linear => t,
            Ease::Smooth => t * t * (3.0 - 2.0 * t),
            Ease::In => t * t * t,
            Ease::Out => 1.0 - (1.0 - t).powi(3),
            Ease::Hold => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Key {
    pub t: f32,
    pub v: f32,
    pub ease: Ease,
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
                let u = a.ease.apply((t - a.t) / (b.t - a.t).max(1e-6));
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
        let ease = self.keys.first().map_or(Ease::Smooth, |k| k.ease);
        self.keys.retain(|k| (k.t - t).abs() >= 1e-3);
        let i = self.keys.partition_point(|k| k.t < t);
        self.keys.insert(i, Key { t, v, ease });
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Content {
    Rect { w: f32, h: f32, r: f32, color: [u8; 4] },
    Ellipse { w: f32, h: f32, color: [u8; 4] },
    Text { text: String, size: f32, color: [u8; 4], bold: bool, family: String },
    Image { path: String, w: f32, h: f32 },
}

pub const PROPS: [&str; 5] = ["Position X", "Position Y", "Échelle %", "Rotation °", "Opacité %"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub content: Content,
    pub props: [Prop; 5],
    pub start: f32,
    pub end: f32,
    pub visible: bool,
}

impl Layer {
    pub fn new(name: String, content: Content, x: f32, y: f32, end: f32) -> Self {
        Self {
            name,
            content,
            props: [Prop::new(x), Prop::new(y), Prop::new(100.0), Prop::new(0.0), Prop::new(100.0)],
            start: 0.0,
            end,
            visible: true,
        }
    }

    pub fn size(&self) -> (f32, f32) {
        match &self.content {
            Content::Rect { w, h, .. } | Content::Ellipse { w, h, .. } | Content::Image { w, h, .. } => (*w, *h),
            Content::Text { text, size, .. } => {
                let cols = text.lines().map(|l| l.chars().count()).max().unwrap_or(1) as f32;
                (cols * size * 0.55, text.lines().count().max(1) as f32 * size * 1.2)
            }
        }
    }

    pub fn active(&self, t: f32) -> bool {
        self.visible && t >= self.start && t <= self.end
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comp {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration: f32,
    pub background: [u8; 4],
    pub layers: Vec<Layer>,
}

fn color(c: [u8; 4]) -> String {
    format!(r##"fill="#{:02x}{:02x}{:02x}" fill-opacity="{}""##, c[0], c[1], c[2], c[3] as f32 / 255.0)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

impl Comp {
    pub fn new() -> Self {
        Self { width: 1920, height: 1080, fps: 30, duration: 5.0, background: [16, 18, 26, 255], layers: Vec::new() }
    }

    pub fn frames(&self) -> u32 {
        (self.duration * self.fps as f32).round().max(1.0) as u32
    }

    pub fn to_svg(&self, t: f32) -> String {
        let mut s = String::new();
        writeln!(
            s,
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
            w = self.width,
            h = self.height
        )
        .unwrap();
        writeln!(s, r#"<rect width="100%" height="100%" {}/>"#, color(self.background)).unwrap();
        for l in self.layers.iter().filter(|l| l.active(t)) {
            let [x, y, sc, rot, op] = l.props.each_ref().map(|p| p.at(t));
            write!(
                s,
                r#"<g transform="translate({x} {y}) rotate({rot}) scale({})" opacity="{}">"#,
                sc / 100.0,
                (op / 100.0).clamp(0.0, 1.0)
            )
            .unwrap();
            match &l.content {
                Content::Rect { w, h, r, color: c } => {
                    write!(s, r#"<rect x="{}" y="{}" width="{w}" height="{h}" rx="{r}" {}/>"#, -w / 2.0, -h / 2.0, color(*c)).unwrap()
                }
                Content::Ellipse { w, h, color: c } => {
                    write!(s, r#"<ellipse rx="{}" ry="{}" {}/>"#, w / 2.0, h / 2.0, color(*c)).unwrap()
                }
                Content::Text { text, size, color: c, bold, family } => {
                    let lines: Vec<&str> = text.lines().collect();
                    let n = lines.len().max(1) as f32;
                    let top = -(n - 1.0) * size * 1.2 / 2.0 + size * 0.35;
                    write!(
                        s,
                        r#"<text text-anchor="middle" font-size="{size}" font-family="{}" font-weight="{}" {}>"#,
                        esc(family),
                        if *bold { "bold" } else { "normal" },
                        color(*c)
                    )
                    .unwrap();
                    for (i, line) in lines.iter().enumerate() {
                        write!(s, r#"<tspan x="0" y="{}">{}</tspan>"#, top + i as f32 * size * 1.2, esc(line)).unwrap();
                    }
                    s.push_str("</text>");
                }
                Content::Image { path, w, h } => write!(
                    s,
                    r#"<image x="{}" y="{}" width="{w}" height="{h}" preserveAspectRatio="none" xlink:href="{}"/>"#,
                    -w / 2.0,
                    -h / 2.0,
                    esc(path)
                )
                .unwrap(),
            }
            s.push_str("</g>\n");
        }
        s.push_str("</svg>\n");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut l = Layer::new("T".into(), Content::Text { text: "A&B".into(), size: 40.0, color: [255, 255, 255, 255], bold: true, family: "Arial".into() }, 100.0, 50.0, 2.0);
        l.props[4].add_key(0.0, 0.0);
        l.props[4].add_key(1.0, 100.0);
        c.layers.push(l);
        let svg = c.to_svg(0.5);
        assert!(svg.contains("translate(100 50)"));
        assert!(svg.contains(r#"opacity="0.5""#));
        assert!(svg.contains("A&amp;B"));
        assert!(!c.to_svg(3.0).contains("<g"));
        assert_eq!(c.frames(), 150);
    }
}
