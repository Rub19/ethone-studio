use serde::{Deserialize, Serialize};
use std::fmt::Write;

fn one() -> f32 {
    1.0
}

fn one_u() -> u32 {
    1
}

fn gutter() -> f32 {
    14.0
}

fn white() -> [u8; 4] {
    [255, 255, 255, 255]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    Text {
        text: String,
        size: f32,
        family: String,
        bold: bool,
        color: [u8; 4],
        align: u8,
        leading: f32,
        #[serde(default)]
        italic: bool,
        #[serde(default)]
        tracking: f32,
        #[serde(default = "one_u")]
        columns: u32,
        #[serde(default = "gutter")]
        gutter: f32,
        #[serde(default)]
        background: Option<[u8; 4]>,
        #[serde(default)]
        padding: f32,
    },
    Image {
        path: String,
        cover: bool,
        #[serde(default)]
        radius: f32,
    },
    Rect {
        fill: [u8; 4],
        radius: f32,
    },
    Ellipse {
        fill: [u8; 4],
    },
    Line {
        color: [u8; 4],
        width: f32,
    },
}

impl Kind {
    pub fn text(text: &str, size: f32, bold: bool) -> Kind {
        Kind::Text {
            text: text.into(),
            size,
            family: "Segoe UI".into(),
            bold,
            color: [20, 20, 24, 255],
            align: 0,
            leading: 1.35,
            italic: false,
            tracking: 0.0,
            columns: 1,
            gutter: 14.0,
            background: None,
            padding: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub kind: Kind,
    #[serde(default = "one")]
    pub opacity: f32,
    #[serde(default)]
    pub stroke: Option<[u8; 4]>,
    #[serde(default)]
    pub stroke_w: f32,
    #[serde(default)]
    pub locked: bool,
}

impl Frame {
    pub fn new(x: f32, y: f32, w: f32, h: f32, kind: Kind) -> Self {
        Frame { x, y, w, h, kind, opacity: 1.0, stroke: None, stroke_w: 1.0, locked: false }
    }
    pub fn contains(&self, p: [f32; 2], tol: f32) -> bool {
        p[0] >= self.x - tol && p[0] <= self.x + self.w + tol && p[1] >= self.y - tol && p[1] <= self.y + self.h + tol
    }
    pub fn corners(&self) -> [[f32; 2]; 4] {
        [[self.x, self.y], [self.x + self.w, self.y], [self.x + self.w, self.y + self.h], [self.x, self.y + self.h]]
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Page {
    pub frames: Vec<Frame>,
    #[serde(default)]
    pub no_master: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    pub width: f32,
    pub height: f32,
    pub margin: f32,
    pub pages: Vec<Page>,
    #[serde(default)]
    pub master: Page,
    #[serde(default = "white")]
    pub paper: [u8; 4],
    #[serde(default = "one_u")]
    pub grid: u32,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn fill(c: [u8; 4]) -> String {
    format!(r##"fill="#{:02x}{:02x}{:02x}" fill-opacity="{}""##, c[0], c[1], c[2], c[3] as f32 / 255.0)
}

fn stroke(f: &Frame) -> String {
    match f.stroke {
        Some(c) if f.stroke_w > 0.0 => format!(r##" stroke="#{:02x}{:02x}{:02x}" stroke-opacity="{}" stroke-width="{}""##, c[0], c[1], c[2], c[3] as f32 / 255.0, f.stroke_w),
        _ => String::new(),
    }
}

pub fn wrap(text: &str, max_w: f32, measure: &dyn Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if measure(&candidate) <= max_w || line.is_empty() {
                line = candidate;
            } else {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            }
        }
        lines.push(line);
    }
    lines
}

pub type Measure<'a> = &'a dyn Fn(&str, &str, bool, f32) -> f32;

pub const MASTER: usize = usize::MAX;

impl Doc {
    pub fn a4() -> Self {
        Self { width: 595.0, height: 842.0, margin: 40.0, pages: vec![Page::default()], master: Page::default(), paper: white(), grid: 1 }
    }

    /// Height a text frame needs to show all its lines (single column).
    pub fn fit_height(&self, f: &Frame, measure: Measure) -> Option<f32> {
        let Kind::Text { text, size, family, bold, leading, tracking, columns, gutter, padding, .. } = &f.kind else { return None };
        let cols = (*columns).max(1) as f32;
        let col_w = ((f.w - 2.0 * padding - gutter * (cols - 1.0)) / cols).max(1.0);
        let m = |t: &str| measure(t, family, *bold, *size) + tracking * t.chars().count() as f32;
        let n = wrap(text, col_w, &m).len() as f32;
        Some(((n / cols).ceil() - 1.0).max(0.0) * size * leading + size * 1.1 + 2.0 * padding)
    }

    fn frame_svg(&self, s: &mut String, f: &Frame, id: &str, page_no: usize, measure: Measure) -> bool {
        let clip = format!("c{id}");
        writeln!(s, r#"<clipPath id="{clip}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath>"#, f.x, f.y, f.w.max(0.1), f.h.max(0.1)).unwrap();
        write!(s, r#"<g opacity="{}">"#, f.opacity.clamp(0.0, 1.0)).unwrap();
        let st = stroke(f);
        let mut over = false;
        match &f.kind {
            Kind::Rect { fill: c, radius } => {
                write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{radius}" {}{st}/>"#, f.x, f.y, f.w, f.h, fill(*c)).unwrap();
            }
            Kind::Ellipse { fill: c } => {
                write!(s, r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" {}{st}/>"#, f.x + f.w / 2.0, f.y + f.h / 2.0, f.w / 2.0, f.h / 2.0, fill(*c)).unwrap();
            }
            Kind::Line { color, width } => {
                write!(s, r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#{:02x}{:02x}{:02x}" stroke-opacity="{}" stroke-width="{width}" stroke-linecap="round"/>"##, f.x, f.y + f.h / 2.0, f.x + f.w, f.y + f.h / 2.0, color[0], color[1], color[2], color[3] as f32 / 255.0).unwrap();
            }
            Kind::Image { path, cover, radius } => {
                let par = if *cover { "xMidYMid slice" } else { "xMidYMid meet" };
                let rclip = format!("r{id}");
                writeln!(s, r#"<clipPath id="{rclip}"><rect x="{}" y="{}" width="{}" height="{}" rx="{radius}"/></clipPath>"#, f.x, f.y, f.w.max(0.1), f.h.max(0.1)).unwrap();
                write!(s, r#"<image clip-path="url(#{rclip})" x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="{par}" xlink:href="{}"/>"#, f.x, f.y, f.w, f.h, esc(path)).unwrap();
                if !st.is_empty() {
                    write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{radius}" fill="none"{st}/>"#, f.x, f.y, f.w, f.h).unwrap();
                }
            }
            Kind::Text { text, size, family, bold, color, align, leading, italic, tracking, columns, gutter, background, padding } => {
                if let Some(bg) = background {
                    write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" {}{st}/>"#, f.x, f.y, f.w, f.h, fill(*bg)).unwrap();
                } else if !st.is_empty() {
                    write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="none"{st}/>"#, f.x, f.y, f.w, f.h).unwrap();
                }
                let text = text.replace("{page}", &page_no.to_string()).replace("{pages}", &self.pages.len().to_string());
                let cols = (*columns).max(1);
                let (ix, iy, iw, ih) = (f.x + padding, f.y + padding, (f.w - 2.0 * padding).max(1.0), (f.h - 2.0 * padding).max(1.0));
                let col_w = ((iw - gutter * (cols as f32 - 1.0)) / cols as f32).max(1.0);
                let m = |t: &str| measure(t, family, *bold, *size) + tracking * t.chars().count() as f32;
                let lines = wrap(&text, col_w, &m);
                let step = size * leading;
                let per_col = (((ih - size * 1.05) / step).floor() as i64 + 1).max(0) as usize;
                write!(
                    s,
                    r#"<text clip-path="url(#{clip})" font-size="{size}" font-family="{}" font-weight="{}" font-style="{}" letter-spacing="{tracking}" {}>"#,
                    esc(family),
                    if *bold { "bold" } else { "normal" },
                    if *italic { "italic" } else { "normal" },
                    fill(*color)
                )
                .unwrap();
                for (i, line) in lines.iter().enumerate() {
                    let (c, row) = if per_col == 0 { (cols as usize, 0) } else { (i / per_col, i % per_col) };
                    if c >= cols as usize {
                        over = true;
                        break;
                    }
                    let cx = ix + c as f32 * (col_w + gutter);
                    let (anchor, ax) = match align {
                        1 => ("middle", cx + col_w / 2.0),
                        2 => ("end", cx + col_w),
                        _ => ("start", cx),
                    };
                    let y = iy + size * 0.85 + row as f32 * step;
                    write!(s, r#"<tspan x="{ax}" y="{y}" text-anchor="{anchor}" xml:space="preserve">{}</tspan>"#, esc(line)).unwrap();
                }
                s.push_str("</text>");
            }
        }
        s.push_str("</g>\n");
        over
    }

    /// `index == MASTER` renders the master page alone.
    pub fn to_svg(&self, index: usize, measure: Measure, guides: bool) -> (String, Vec<bool>) {
        let mut s = String::new();
        let mut overflow = Vec::new();
        writeln!(
            s,
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><rect width="100%" height="100%" {}/>"##,
            fill(self.paper),
            w = self.width,
            h = self.height
        )
        .unwrap();
        if guides {
            let m = self.margin;
            writeln!(s, r##"<rect x="{m}" y="{m}" width="{}" height="{}" fill="none" stroke="#ff2fa0" stroke-opacity="0.35" stroke-width="0.6"/>"##, self.width - 2.0 * m, self.height - 2.0 * m).unwrap();
            let g = self.grid.max(1);
            if g > 1 {
                let gut = 12.0;
                let cw = (self.width - 2.0 * m - gut * (g as f32 - 1.0)) / g as f32;
                for k in 0..g {
                    let x = m + k as f32 * (cw + gut);
                    writeln!(s, r##"<rect x="{x}" y="{m}" width="{cw}" height="{}" fill="#6a5cff" fill-opacity="0.05" stroke="#6a5cff" stroke-opacity="0.25" stroke-width="0.5"/>"##, self.height - 2.0 * m).unwrap();
                }
            }
        }
        if index == MASTER {
            for (fi, f) in self.master.frames.iter().enumerate() {
                overflow.push(self.frame_svg(&mut s, f, &format!("m{fi}"), 1, measure));
            }
        } else {
            if !self.pages[index].no_master {
                for (fi, f) in self.master.frames.iter().enumerate() {
                    self.frame_svg(&mut s, f, &format!("m{fi}"), index + 1, measure);
                }
            }
            for (fi, f) in self.pages[index].frames.iter().enumerate() {
                overflow.push(self.frame_svg(&mut s, f, &fi.to_string(), index + 1, measure));
            }
        }
        s.push_str("</svg>\n");
        (s, overflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_svg_overflow() {
        let mono = |t: &str| t.chars().count() as f32 * 10.0;
        assert_eq!(wrap("aaa bbb ccc", 75.0, &mono), vec!["aaa bbb", "ccc"]);
        assert_eq!(wrap("un\n\ndeux", 100.0, &mono), vec!["un", "", "deux"]);
        assert_eq!(wrap("motimmense", 30.0, &mono), vec!["motimmense"]);

        let mut d = Doc::a4();
        let mut k = Kind::text("Page {page}/{pages} un deux trois quatre cinq six", 10.0, false);
        if let Kind::Text { leading, .. } = &mut k {
            *leading = 1.2;
        }
        d.pages[0].frames.push(Frame::new(10.0, 10.0, 100.0, 30.0, k));
        d.pages.push(Page::default());
        let measure = |t: &str, _: &str, _: bool, size: f32| t.chars().count() as f32 * size * 0.5;
        let (svg, over) = d.to_svg(0, &measure, false);
        assert!(svg.contains("Page 1/2"));
        assert_eq!(over, vec![true]);
        assert_eq!(svg.matches("<tspan").count(), 2);

        let fit = d.fit_height(&d.pages[0].frames[0], &measure).unwrap();
        d.pages[0].frames[0].h = fit;
        assert_eq!(d.to_svg(0, &measure, false).1, vec![false]);

        if let Kind::Text { columns, .. } = &mut d.pages[0].frames[0].kind {
            *columns = 2;
        }
        d.pages[0].frames[0].w = 220.0;
        d.pages[0].frames[0].h = 30.0;
        let (svg, over) = d.to_svg(0, &measure, false);
        assert_eq!(over, vec![false], "{svg}");

        d.master.frames.push(Frame::new(0.0, 800.0, 595.0, 20.0, Kind::text("Pied {page}", 9.0, false)));
        assert!(d.to_svg(1, &measure, false).0.contains("Pied 2"));
        d.pages[1].no_master = true;
        assert!(!d.to_svg(1, &measure, false).0.contains("Pied"));
        assert!(d.to_svg(MASTER, &measure, false).0.contains("Pied 1"));

        d.pages[0].frames.push(Frame::new(0.0, 0.0, 50.0, 50.0, Kind::Ellipse { fill: [255, 0, 0, 255] }));
        d.pages[0].frames.push(Frame::new(0.0, 0.0, 50.0, 4.0, Kind::Line { color: [0, 0, 0, 255], width: 2.0 }));
        let svg = d.to_svg(0, &measure, true).0;
        assert!(svg.contains("<ellipse") && svg.contains("<line"));
        assert!(resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).is_ok());
    }
}
