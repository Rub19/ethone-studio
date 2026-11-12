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
}

impl Ease {
    pub const ALL: [(Ease, &'static str); 8] = [
        (Ease::Linear, "Linéaire"),
        (Ease::Smooth, "Douce"),
        (Ease::In, "Accélère"),
        (Ease::Out, "Ralentit"),
        (Ease::Hold, "Maintien"),
        (Ease::Back, "Dépassement"),
        (Ease::Elastic, "Élastique"),
        (Ease::Bounce, "Rebond"),
    ];
    fn apply(self, t: f32) -> f32 {
        match self {
            Ease::Linear => t,
            Ease::Smooth => t * t * (3.0 - 2.0 * t),
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
    Star { points: u32, radius: f32, inner: f32, color: [u8; 4] },
    Line { w: f32, thickness: f32, color: [u8; 4] },
    Counter { from: f32, to: f32, decimals: u32, size: f32, color: [u8; 4], prefix: String, suffix: String },
}

pub const BLENDS: [(&str, &str); 7] = [
    ("normal", "Normal"),
    ("multiply", "Produit"),
    ("screen", "Superposition"),
    ("overlay", "Incrustation"),
    ("darken", "Obscurcir"),
    ("lighten", "Éclaircir"),
    ("difference", "Différence"),
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
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
    #[serde(default)]
    pub fx: Fx,
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
            fx: Fx::default(),
        }
    }

    pub fn size(&self) -> (f32, f32) {
        match &self.content {
            Content::Rect { w, h, .. } | Content::Ellipse { w, h, .. } | Content::Image { w, h, .. } => (*w, *h),
            Content::Text { text, size, .. } => {
                let cols = text.lines().map(|l| l.chars().count()).max().unwrap_or(1) as f32;
                (cols * size * 0.55, text.lines().count().max(1) as f32 * size * 1.2)
            }
            Content::Star { radius, .. } => (radius * 2.0, radius * 2.0),
            Content::Line { w, thickness, .. } => (*w, thickness.max(8.0)),
            Content::Counter { to, from, decimals, size, prefix, suffix, .. } => {
                let n = format!("{prefix}{:.*}{suffix}", *decimals as usize, from.abs().max(to.abs())).chars().count() as f32;
                (n * size * 0.6, size * 1.2)
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
        for (li, l) in self.layers.iter().enumerate().filter(|(_, l)| l.active(t)) {
            let fx = &l.fx;
            let last = l.props.iter().flat_map(|p| p.keys.last()).map(|k| k.t).fold(0.0f32, f32::max);
            let tt = if fx.loop_keys && last > l.start + 1e-3 { l.start + (t - l.start).rem_euclid(last - l.start) } else { t };
            let [mut x, mut y, sc, rot, op] = l.props.each_ref().map(|p| p.at(tt));
            if fx.wiggle > 0.0 {
                let f = fx.wiggle_freq.max(0.1) * std::f32::consts::TAU;
                let ph = li as f32 * 1.37;
                x += fx.wiggle * ((t * f + ph).sin() * 0.6 + (t * f * 2.31 + ph * 3.1).sin() * 0.4);
                y += fx.wiggle * ((t * f * 1.13 + ph * 2.3).cos() * 0.6 + (t * f * 1.87 + ph).sin() * 0.4);
            }
            let mut defs = String::new();
            if fx.blur > 0.0 || fx.shadow {
                let mut f = String::new();
                if fx.blur > 0.0 {
                    write!(f, r#"<feGaussianBlur stdDeviation="{}" result="b"/>"#, fx.blur).unwrap();
                }
                if fx.shadow {
                    write!(f, r##"<feDropShadow{} dx="0" dy="10" stdDeviation="14" flood-color="#000000" flood-opacity="0.45"/>"##, if fx.blur > 0.0 { r#" in="b""# } else { "" }).unwrap();
                }
                write!(defs, r#"<filter id="f{li}" x="-50%" y="-50%" width="200%" height="200%">{f}</filter>"#).unwrap();
            }
            let fill_of = |c: [u8; 4], defs: &mut String| -> String {
                match fx.gradient {
                    Some(g) => {
                        write!(
                            defs,
                            r##"<linearGradient id="g{li}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#{:02x}{:02x}{:02x}" stop-opacity="{}"/><stop offset="1" stop-color="#{:02x}{:02x}{:02x}" stop-opacity="{}"/></linearGradient>"##,
                            c[0], c[1], c[2], c[3] as f32 / 255.0, g[0], g[1], g[2], g[3] as f32 / 255.0
                        )
                        .unwrap();
                        format!(r#"fill="url(#g{li})""#)
                    }
                    None => color(c),
                }
            };
            let stroke = match fx.stroke {
                Some(c) if fx.stroke_w > 0.0 => format!(r##" stroke="#{:02x}{:02x}{:02x}" stroke-opacity="{}" stroke-width="{}" stroke-linejoin="round""##, c[0], c[1], c[2], c[3] as f32 / 255.0, fx.stroke_w),
                _ => String::new(),
            };
            let mut body = String::new();
            match &l.content {
                Content::Rect { w, h, r, color: c } => {
                    let paint = fill_of(*c, &mut defs);
                    write!(body, r#"<rect x="{}" y="{}" width="{w}" height="{h}" rx="{r}" {paint}{stroke}/>"#, -w / 2.0, -h / 2.0).unwrap()
                }
                Content::Ellipse { w, h, color: c } => {
                    let paint = fill_of(*c, &mut defs);
                    write!(body, r#"<ellipse rx="{}" ry="{}" {paint}{stroke}/>"#, w / 2.0, h / 2.0).unwrap()
                }
                Content::Star { points, radius, inner, color: c } => {
                    let paint = fill_of(*c, &mut defs);
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
                    let paint = fill_of(*c, &mut defs);
                    write!(body, r#"<rect x="{}" y="{}" width="{w}" height="{thickness}" rx="{}" {paint}{stroke}/>"#, -w / 2.0, -thickness / 2.0, thickness / 2.0).unwrap()
                }
                Content::Text { text, size, color: c, bold, family } => {
                    let paint = fill_of(*c, &mut defs);
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
                    write!(body, r#"<text text-anchor="middle" font-size="{size}" font-family="{}" font-weight="{}" {paint}{stroke}>"#, esc(family), if *bold { "bold" } else { "normal" }).unwrap();
                    let mut left = shown.chars().count();
                    for (i, line) in lines.iter().enumerate() {
                        let part: String = line.chars().take(left).collect();
                        left = left.saturating_sub(line.chars().count() + 1);
                        write!(body, r#"<tspan x="0" y="{}">{}</tspan>"#, top + i as f32 * size * 1.2, esc(&part)).unwrap();
                    }
                    body.push_str("</text>");
                }
                Content::Counter { from, to, decimals, size, color: c, prefix, suffix } => {
                    let paint = fill_of(*c, &mut defs);
                    let u = ((t - l.start) / (l.end - l.start).max(1e-3) / 0.8).clamp(0.0, 1.0);
                    let u = 1.0 - (1.0 - u).powi(3);
                    let v = from + (to - from) * u;
                    write!(body, r#"<text text-anchor="middle" y="{}" font-size="{size}" font-family="Segoe UI" font-weight="bold" {paint}{stroke}>{}</text>"#, size * 0.35, esc(&format!("{prefix}{v:.*}{suffix}", *decimals as usize))).unwrap();
                }
                Content::Image { path, w, h } => write!(
                    body,
                    r#"<image x="{}" y="{}" width="{w}" height="{h}" preserveAspectRatio="none" xlink:href="{}"/>"#,
                    -w / 2.0,
                    -h / 2.0,
                    esc(path)
                )
                .unwrap(),
            }
            if !defs.is_empty() {
                writeln!(s, "<defs>{defs}</defs>").unwrap();
            }
            let blend = BLENDS.get(fx.blend as usize).filter(|b| b.0 != "normal").map(|b| format!(r#" style="mix-blend-mode:{}""#, b.0)).unwrap_or_default();
            let filter = if fx.blur > 0.0 || fx.shadow { format!(r#" filter="url(#f{li})""#) } else { String::new() };
            write!(
                s,
                r#"<g transform="translate({x} {y}) rotate({rot}) scale({})" opacity="{}"{blend}><g{filter}>{body}</g></g>"#,
                sc / 100.0,
                (op / 100.0).clamp(0.0, 1.0)
            )
            .unwrap();
            s.push('\n');
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

        for (e, _) in Ease::ALL {
            assert!((e.apply(1.0) - if e == Ease::Hold { 0.0 } else { 1.0 }).abs() < 1e-3, "{e:?}");
        }
        assert!(Ease::Back.apply(0.8) > 1.0);
        let mut c2 = Comp::new();
        let mut word = Layer::new("w".into(), Content::Text { text: "Bonjour".into(), size: 40.0, color: [255; 4], bold: false, family: "Arial".into() }, 0.0, 0.0, 4.0);
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
        assert!(resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).is_ok());
        assert_eq!(c.frames(), 150);
    }
}
