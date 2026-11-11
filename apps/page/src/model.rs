use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    Text { text: String, size: f32, family: String, bold: bool, color: [u8; 4], align: u8, leading: f32 },
    Image { path: String, cover: bool },
    Rect { fill: [u8; 4], radius: f32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub kind: Kind,
}

impl Frame {
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    pub width: f32,
    pub height: f32,
    pub margin: f32,
    pub pages: Vec<Page>,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn fill(c: [u8; 4]) -> String {
    format!(r##"fill="#{:02x}{:02x}{:02x}" fill-opacity="{}""##, c[0], c[1], c[2], c[3] as f32 / 255.0)
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

impl Doc {
    pub fn a4() -> Self {
        Self { width: 595.0, height: 842.0, margin: 40.0, pages: vec![Page::default()] }
    }

    pub fn to_svg(&self, index: usize, measure: Measure, guides: bool) -> (String, Vec<bool>) {
        let mut s = String::new();
        let mut overflow = Vec::new();
        writeln!(
            s,
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><rect width="100%" height="100%" fill="#ffffff"/>"##,
            w = self.width,
            h = self.height
        )
        .unwrap();
        if guides {
            let m = self.margin;
            writeln!(
                s,
                r##"<rect x="{m}" y="{m}" width="{}" height="{}" fill="none" stroke="#ff2fa0" stroke-opacity="0.35" stroke-width="0.6"/>"##,
                self.width - 2.0 * m,
                self.height - 2.0 * m
            )
            .unwrap();
        }
        for (fi, f) in self.pages[index].frames.iter().enumerate() {
            let clip = format!("c{fi}");
            writeln!(s, r#"<clipPath id="{clip}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath>"#, f.x, f.y, f.w.max(0.1), f.h.max(0.1)).unwrap();
            let mut over = false;
            match &f.kind {
                Kind::Rect { fill: c, radius } => {
                    writeln!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{radius}" {}/>"#, f.x, f.y, f.w, f.h, fill(*c)).unwrap();
                }
                Kind::Image { path, cover } => {
                    let par = if *cover { "xMidYMid slice" } else { "xMidYMid meet" };
                    writeln!(
                        s,
                        r#"<image clip-path="url(#{clip})" x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="{par}" xlink:href="{}"/>"#,
                        f.x,
                        f.y,
                        f.w,
                        f.h,
                        esc(path)
                    )
                    .unwrap();
                }
                Kind::Text { text, size, family, bold, color, align, leading } => {
                    let text = text.replace("{page}", &(index + 1).to_string()).replace("{pages}", &self.pages.len().to_string());
                    let m = |t: &str| measure(t, family, *bold, *size);
                    let lines = wrap(&text, f.w, &m);
                    let step = size * leading;
                    let (anchor, ax) = match align {
                        1 => ("middle", f.x + f.w / 2.0),
                        2 => ("end", f.x + f.w),
                        _ => ("start", f.x),
                    };
                    write!(
                        s,
                        r#"<text clip-path="url(#{clip})" font-size="{size}" font-family="{}" font-weight="{}" text-anchor="{anchor}" {}>"#,
                        esc(family),
                        if *bold { "bold" } else { "normal" },
                        fill(*color)
                    )
                    .unwrap();
                    for (i, line) in lines.iter().enumerate() {
                        let y = f.y + size * 0.85 + i as f32 * step;
                        if y + size * 0.2 > f.y + f.h {
                            over = true;
                            break;
                        }
                        write!(s, r#"<tspan x="{ax}" y="{y}" xml:space="preserve">{}</tspan>"#, esc(line)).unwrap();
                    }
                    s.push_str("</text>\n");
                }
            }
            overflow.push(over);
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
        d.pages[0].frames.push(Frame {
            x: 10.0,
            y: 10.0,
            w: 100.0,
            h: 30.0,
            kind: Kind::Text { text: "Page {page}/{pages} un deux trois quatre cinq six".into(), size: 10.0, family: "Arial".into(), bold: false, color: [0, 0, 0, 255], align: 0, leading: 1.2 },
        });
        d.pages.push(Page::default());
        let measure = |t: &str, _: &str, _: bool, size: f32| t.chars().count() as f32 * size * 0.5;
        let (svg, over) = d.to_svg(0, &measure, false);
        assert!(svg.contains("Page 1/2"));
        assert_eq!(over, vec![true]);
        assert_eq!(svg.matches("<tspan").count(), 2);
    }
}
