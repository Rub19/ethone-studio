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

fn grid_gutter() -> f32 {
    12.0
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
        #[serde(default)]
        underline: bool,
        #[serde(default)]
        upper: bool,
        #[serde(default)]
        list: u8,
        #[serde(default)]
        para_space: f32,
        #[serde(default)]
        indent: f32,
        #[serde(default)]
        valign: u8,
        #[serde(default)]
        link: Option<usize>,
    },
    Image {
        path: String,
        cover: bool,
        #[serde(default)]
        radius: f32,
        #[serde(default)]
        filter: u8,
        #[serde(default)]
        flip: bool,
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
        #[serde(default)]
        arrow: bool,
    },
    Polygon {
        sides: u32,
        inner: f32,
        fill: [u8; 4],
    },
    Table {
        cells: Vec<Vec<String>>,
        size: f32,
        family: String,
        color: [u8; 4],
        line: [u8; 4],
        header: Option<[u8; 4]>,
        stripe: Option<[u8; 4]>,
    },
}

pub const FILTERS: [&str; 4] = ["Aucun", "Noir et blanc", "Sépia", "Délavé"];
pub const LISTS: [&str; 3] = ["Aucune", "Puces", "Numéros"];

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
            underline: false,
            upper: false,
            list: 0,
            para_space: 0.0,
            indent: 0.0,
            valign: 0,
            link: None,
        }
    }

    pub fn table(rows: usize, cols: usize) -> Kind {
        let cells = (0..rows).map(|r| (0..cols).map(|c| if r == 0 { format!("Titre {}", c + 1) } else { String::new() }).collect()).collect();
        Kind::Table { cells, size: 10.0, family: "Segoe UI".into(), color: [20, 20, 24, 255], line: [200, 200, 208, 255], header: Some([30, 30, 40, 255]), stripe: Some([245, 245, 248, 255]) }
    }

    pub fn label(&self) -> String {
        match self {
            Kind::Text { text, .. } => {
                let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("Texte vide");
                let s: String = first.chars().take(26).collect();
                if first.chars().count() > 26 { format!("{s}…") } else { s }
            }
            Kind::Image { path, .. } => std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or("Image vide".into()),
            Kind::Rect { .. } => "Rectangle".into(),
            Kind::Ellipse { .. } => "Ellipse".into(),
            Kind::Line { .. } => "Ligne".into(),
            Kind::Polygon { inner, .. } => if *inner < 0.99 { "Étoile".into() } else { "Polygone".into() },
            Kind::Table { cells, .. } => format!("Tableau {}×{}", cells.len(), cells.first().map_or(0, |r| r.len())),
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
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub shadow: bool,
    #[serde(default)]
    pub dash: f32,
    #[serde(default)]
    pub fill2: Option<[u8; 4]>,
    #[serde(default)]
    pub grad_angle: f32,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub name: String,
}

impl Frame {
    pub fn new(x: f32, y: f32, w: f32, h: f32, kind: Kind) -> Self {
        Frame { x, y, w, h, kind, opacity: 1.0, stroke: None, stroke_w: 1.0, locked: false, rotation: 0.0, shadow: false, dash: 0.0, fill2: None, grad_angle: 90.0, hidden: false, name: String::new() }
    }
    pub fn contains(&self, p: [f32; 2], tol: f32) -> bool {
        p[0] >= self.x - tol && p[0] <= self.x + self.w + tol && p[1] >= self.y - tol && p[1] <= self.y + self.h + tol
    }
    pub fn title(&self) -> String {
        if self.name.is_empty() { self.kind.label() } else { self.name.clone() }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Page {
    pub frames: Vec<Frame>,
    #[serde(default)]
    pub no_master: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub vertical: bool,
    pub pos: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub name: String,
    pub size: f32,
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    pub leading: f32,
    pub tracking: f32,
    pub color: [u8; 4],
    pub para_space: f32,
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
    #[serde(default = "grid_gutter")]
    pub grid_gutter: f32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub bleed: f32,
    #[serde(default)]
    pub baseline: f32,
    #[serde(default)]
    pub guides: Vec<Guide>,
    #[serde(default)]
    pub swatches: Vec<[u8; 4]>,
    #[serde(default)]
    pub styles: Vec<TextStyle>,
    #[serde(default = "one_u")]
    pub first_number: u32,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn hex(c: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn fill(c: [u8; 4]) -> String {
    format!(r#"fill="{}" fill-opacity="{}""#, hex(c), c[3] as f32 / 255.0)
}

fn stroke(f: &Frame) -> String {
    match f.stroke {
        Some(c) if f.stroke_w > 0.0 => {
            let dash = if f.dash > 0.0 { format!(r#" stroke-dasharray="{} {}""#, f.dash, f.dash * 0.7) } else { String::new() };
            format!(r#" stroke="{}" stroke-opacity="{}" stroke-width="{}"{dash}"#, hex(c), c[3] as f32 / 255.0, f.stroke_w)
        }
        _ => String::new(),
    }
}

/// Wrapped lines, each flagged when it ends a paragraph.
pub fn wrap_paras(text: &str, max_w: f32, measure: &dyn Fn(&str) -> f32) -> Vec<(String, bool)> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if measure(&candidate) <= max_w || line.is_empty() {
                line = candidate;
            } else {
                lines.push((std::mem::take(&mut line), false));
                line = word.to_string();
            }
        }
        lines.push((line, true));
    }
    lines
}

pub type Measure<'a> = &'a dyn Fn(&str, &str, bool, f32) -> f32;

pub const MASTER: usize = usize::MAX;

pub struct Laid {
    pub lines: Vec<(String, f32, f32, f32)>,
    pub leftover: Vec<(String, bool)>,
    pub used: f32,
}

impl Doc {
    pub fn a4() -> Self {
        Self {
            width: 595.0,
            height: 842.0,
            margin: 40.0,
            pages: vec![Page::default()],
            master: Page::default(),
            paper: white(),
            grid: 1,
            grid_gutter: 12.0,
            title: String::new(),
            bleed: 0.0,
            baseline: 0.0,
            guides: Vec::new(),
            swatches: Vec::new(),
            styles: Vec::new(),
            first_number: 1,
        }
    }

    pub fn page_label(&self, index: usize) -> u32 {
        index as u32 + self.first_number
    }

    fn prepare(&self, text: &str, page_no: u32, upper: bool, list: u8) -> String {
        let total = (self.pages.len() as u32 + self.first_number).saturating_sub(1);
        let mut t = text.replace("{page}", &page_no.to_string()).replace("{pages}", &total.to_string()).replace("{title}", &self.title);
        if t.contains("{date}") {
            let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
            let (y, m, d) = civil(days);
            t = t.replace("{date}", &format!("{d:02}/{m:02}/{y}"));
        }
        if upper {
            t = t.to_uppercase();
        }
        if list > 0 {
            let mut n = 0;
            t = t
                .split('\n')
                .map(|p| {
                    if p.trim().is_empty() {
                        p.to_string()
                    } else {
                        n += 1;
                        if list == 1 { format!("•\u{a0}\u{a0}{p}") } else { format!("{n}.\u{a0}{p}") }
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
        t
    }

    /// Lays out a text frame: (line, x, y-baseline, word-spacing) and what did not fit.
    pub fn layout(&self, f: &Frame, incoming: Option<&[(String, bool)]>, page_no: u32, measure: Measure, unlimited: bool) -> Option<Laid> {
        let Kind::Text { text, size, family, bold, align, leading, tracking, columns, gutter, padding, upper, list, para_space, indent, valign, .. } = &f.kind else { return None };
        let cols = (*columns).max(1) as usize;
        let (ix, iy, iw, ih) = (f.x + padding, f.y + padding, (f.w - 2.0 * padding).max(1.0), (f.h - 2.0 * padding).max(1.0));
        let col_w = ((iw - gutter * (cols as f32 - 1.0)) / cols as f32).max(1.0);
        let m = |t: &str| measure(t, family, *bold, *size) + tracking * t.chars().count() as f32;
        let src: Vec<(String, bool)> = match incoming {
            Some(lines) => lines
                .iter()
                .flat_map(|(l, end)| {
                    let mut w = wrap_paras(l, col_w, &m);
                    if let Some(last) = w.last_mut() {
                        last.1 = *end;
                    }
                    w
                })
                .collect(),
            None => {
                let t = self.prepare(text, page_no, *upper, *list);
                let mut out = Vec::new();
                for para in t.split('\n') {
                    if *indent > 0.0 {
                        let narrow = wrap_paras(para, (col_w - indent).max(1.0), &m);
                        let first = narrow.first().map(|l| l.0.clone()).unwrap_or_default();
                        if let Some(rest) = para.strip_prefix(first.as_str()) {
                            let rest = rest.trim_start();
                            out.push((first, rest.is_empty()));
                            if !rest.is_empty() {
                                out.extend(wrap_paras(rest, col_w, &m));
                            }
                            continue;
                        }
                    }
                    out.extend(wrap_paras(para, col_w, &m));
                }
                out
            }
        };
        let step = size * leading;
        let line_h = size * 1.05;
        let mut lines = Vec::new();
        let (mut col, mut y, mut used) = (0usize, 0.0f32, 0.0f32);
        let mut para_start = incoming.is_none();
        let mut leftover = Vec::new();
        for (k, (line, end)) in src.iter().enumerate() {
            if !unlimited && y > 0.0 && y + line_h > ih + 0.01 {
                col += 1;
                y = 0.0;
            }
            if col >= cols {
                leftover = src[k..].to_vec();
                break;
            }
            let cx = ix + col as f32 * (col_w + gutter);
            let ind = if para_start && *indent > 0.0 && incoming.is_none() { *indent } else { 0.0 };
            let lw = m(line);
            let (x, ws) = match align {
                1 => (cx + col_w / 2.0, 0.0),
                2 => (cx + col_w, 0.0),
                3 if !end && line.contains(' ') => (cx + ind, ((col_w - ind - lw) / line.matches(' ').count() as f32).max(0.0)),
                _ => (cx + ind, 0.0),
            };
            lines.push((line.clone(), x, iy + size * 0.85 + y, ws));
            used = used.max(y + line_h);
            y += step + if *end { *para_space } else { 0.0 };
            para_start = *end;
        }
        if cols == 1 && !unlimited && *valign > 0 {
            let off = (ih - used).max(0.0) * if *valign == 1 { 0.5 } else { 1.0 };
            for l in &mut lines {
                l.2 += off;
            }
        }
        Some(Laid { lines, leftover, used: used + 2.0 * padding })
    }

    /// Height a text frame needs to show all its lines.
    pub fn fit_height(&self, f: &Frame, measure: Measure) -> Option<f32> {
        let Kind::Text { columns, size, padding, .. } = &f.kind else { return None };
        let cols = (*columns).max(1) as f32;
        let laid = self.layout(f, None, 1, measure, true)?;
        Some(if cols > 1.0 { ((laid.used - 2.0 * padding) / cols + size * 1.2 + 2.0 * padding).ceil() } else { laid.used.ceil() + 1.0 })
    }

    fn defs_paint(&self, f: &Frame, c: [u8; 4], id: &str, s: &mut String) -> String {
        match f.fill2 {
            Some(c2) => {
                let a = f.grad_angle.to_radians();
                let (dx, dy) = (a.cos() * 0.5, a.sin() * 0.5);
                write!(
                    s,
                    r#"<linearGradient id="g{id}" x1="{}" y1="{}" x2="{}" y2="{}"><stop offset="0" stop-color="{}" stop-opacity="{}"/><stop offset="1" stop-color="{}" stop-opacity="{}"/></linearGradient>"#,
                    0.5 - dx,
                    0.5 - dy,
                    0.5 + dx,
                    0.5 + dy,
                    hex(c),
                    c[3] as f32 / 255.0,
                    hex(c2),
                    c2[3] as f32 / 255.0
                )
                .unwrap();
                format!(r#"fill="url(#g{id})""#)
            }
            None => fill(c),
        }
    }

    fn frame_svg(&self, s: &mut String, f: &Frame, id: &str, laid: Option<&Laid>, measure: Measure) {
        if f.hidden {
            return;
        }
        let clip = format!("c{id}");
        writeln!(s, r#"<clipPath id="{clip}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath>"#, f.x, f.y, f.w.max(0.1), f.h.max(0.1)).unwrap();
        if f.shadow {
            writeln!(s, r##"<filter id="s{id}" x="-30%" y="-30%" width="160%" height="160%"><feDropShadow dx="0" dy="{}" stdDeviation="{}" flood-color="#000000" flood-opacity="0.28"/></filter>"##,(f.w.min(f.h) * 0.02).clamp(1.5, 8.0), (f.w.min(f.h) * 0.04).clamp(2.0, 14.0)).unwrap();
        }
        let rot = if f.rotation != 0.0 { format!(r#" transform="rotate({} {} {})""#, f.rotation, f.x + f.w / 2.0, f.y + f.h / 2.0) } else { String::new() };
        let shadow = if f.shadow { format!(r#" filter="url(#s{id})""#) } else { String::new() };
        write!(s, r#"<g opacity="{}"{rot}{shadow}>"#, f.opacity.clamp(0.0, 1.0)).unwrap();
        let st = stroke(f);
        match &f.kind {
            Kind::Rect { fill: c, radius } => {
                let paint = self.defs_paint(f, *c, id, s);
                write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{radius}" {paint}{st}/>"#, f.x, f.y, f.w, f.h).unwrap();
            }
            Kind::Ellipse { fill: c } => {
                let paint = self.defs_paint(f, *c, id, s);
                write!(s, r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" {paint}{st}/>"#, f.x + f.w / 2.0, f.y + f.h / 2.0, f.w / 2.0, f.h / 2.0).unwrap();
            }
            Kind::Polygon { sides, inner, fill: c } => {
                let paint = self.defs_paint(f, *c, id, s);
                let n = (*sides).max(3);
                let star = *inner < 0.99;
                let count = if star { n * 2 } else { n };
                let (cx, cy) = (f.x + f.w / 2.0, f.y + f.h / 2.0);
                let pts: Vec<String> = (0..count)
                    .map(|k| {
                        let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::TAU / count as f32;
                        let r = if star && k % 2 == 1 { *inner } else { 1.0 };
                        format!("{:.2},{:.2}", cx + a.cos() * f.w / 2.0 * r, cy + a.sin() * f.h / 2.0 * r)
                    })
                    .collect();
                write!(s, r#"<polygon points="{}" {paint}{st}/>"#, pts.join(" ")).unwrap();
            }
            Kind::Line { color, width, arrow } => {
                let y = f.y + f.h / 2.0;
                let head = if *arrow { (width * 4.0).max(6.0) } else { 0.0 };
                let dash = if f.dash > 0.0 { format!(r#" stroke-dasharray="{} {}""#, f.dash, f.dash * 0.7) } else { String::new() };
                write!(s, r#"<line x1="{}" y1="{y}" x2="{}" y2="{y}" stroke="{}" stroke-opacity="{}" stroke-width="{width}" stroke-linecap="round"{dash}/>"#, f.x, f.x + f.w - head * 0.6, hex(*color), color[3] as f32 / 255.0).unwrap();
                if *arrow {
                    let e = f.x + f.w;
                    write!(s, r#"<polygon points="{e},{y} {},{} {},{}" {}/>"#, e - head, y - head * 0.55, e - head, y + head * 0.55, fill(*color)).unwrap();
                }
            }
            Kind::Image { path, cover, radius, filter, flip } => {
                let par = if *cover { "xMidYMid slice" } else { "xMidYMid meet" };
                let rclip = format!("r{id}");
                writeln!(s, r#"<clipPath id="{rclip}"><rect x="{}" y="{}" width="{}" height="{}" rx="{radius}"/></clipPath>"#, f.x, f.y, f.w.max(0.1), f.h.max(0.1)).unwrap();
                let fx = match filter {
                    1 => Some(r#"<feColorMatrix type="saturate" values="0"/>"#.to_string()),
                    2 => Some(r#"<feColorMatrix type="matrix" values="0.393 0.769 0.189 0 0 0.349 0.686 0.168 0 0 0.272 0.534 0.131 0 0 0 0 0 1 0"/>"#.to_string()),
                    3 => Some(r#"<feColorMatrix type="saturate" values="0.45"/><feComponentTransfer><feFuncR type="linear" slope="0.82" intercept="0.14"/><feFuncG type="linear" slope="0.82" intercept="0.14"/><feFuncB type="linear" slope="0.82" intercept="0.14"/></feComponentTransfer>"#.to_string()),
                    _ => None,
                };
                let fattr = match fx {
                    Some(body) => {
                        writeln!(s, r#"<filter id="i{id}">{body}</filter>"#).unwrap();
                        format!(r#" filter="url(#i{id})""#)
                    }
                    None => String::new(),
                };
                let flip_t = if *flip { format!(r#" transform="translate({} 0) scale(-1 1)""#, 2.0 * f.x + f.w) } else { String::new() };
                if path.is_empty() {
                    write!(s, r##"<rect x="{}" y="{}" width="{}" height="{}" rx="{radius}" fill="#e8e8ee"/><line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#b8b8c4" stroke-width="1"/><line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#b8b8c4" stroke-width="1"/>"##, f.x, f.y, f.w, f.h, f.x, f.y, f.x + f.w, f.y + f.h, f.x + f.w, f.y, f.x, f.y + f.h).unwrap();
                } else {
                    write!(s, r#"<g clip-path="url(#{rclip})"><image{fattr}{flip_t} x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="{par}" xlink:href="{}"/></g>"#, f.x, f.y, f.w, f.h, esc(path)).unwrap();
                }
                if !st.is_empty() {
                    write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{radius}" fill="none"{st}/>"#, f.x, f.y, f.w, f.h).unwrap();
                }
            }
            Kind::Table { cells, size, family, color, line, header, stripe } => {
                let rows = cells.len().max(1);
                let cols = cells.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                let (cw, rh) = (f.w / cols as f32, f.h / rows as f32);
                for r in 0..rows {
                    let bg = if r == 0 { *header } else if r % 2 == 0 { *stripe } else { None };
                    if let Some(bg) = bg {
                        write!(s, r#"<rect x="{}" y="{}" width="{}" height="{rh}" {}/>"#, f.x, f.y + r as f32 * rh, f.w, fill(bg)).unwrap();
                    }
                }
                let ls = format!(r#"stroke="{}" stroke-opacity="{}" stroke-width="0.75""#, hex(*line), line[3] as f32 / 255.0);
                write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="none" {ls}/>"#, f.x, f.y, f.w, f.h).unwrap();
                for r in 1..rows {
                    write!(s, r#"<line x1="{}" y1="{}" x2="{}" y2="{}" {ls}/>"#, f.x, f.y + r as f32 * rh, f.x + f.w, f.y + r as f32 * rh).unwrap();
                }
                for c in 1..cols {
                    write!(s, r#"<line x1="{}" y1="{}" x2="{}" y2="{}" {ls}/>"#, f.x + c as f32 * cw, f.y, f.x + c as f32 * cw, f.y + f.h).unwrap();
                }
                let head_dark = header.is_some_and(|h| (h[0] as u32 + h[1] as u32 + h[2] as u32) < 380);
                write!(s, r#"<g clip-path="url(#{clip})" font-size="{size}" font-family="{}">"#, esc(family)).unwrap();
                for (r, row) in cells.iter().enumerate() {
                    for (c, cell) in row.iter().enumerate() {
                        let bold = r == 0 && header.is_some();
                        let mut t = cell.clone();
                        let max = cw - 8.0;
                        while !t.is_empty() && measure(&t, family, bold, *size) > max {
                            t.pop();
                            while !t.is_empty() && measure(&format!("{t}…"), family, bold, *size) > max {
                                t.pop();
                            }
                            t.push('…');
                            break;
                        }
                        let ink = if r == 0 && head_dark { [255, 255, 255, 255] } else { *color };
                        write!(s, r#"<text x="{}" y="{}" font-weight="{}" {}>{}</text>"#, f.x + c as f32 * cw + 4.0, f.y + r as f32 * rh + rh / 2.0 + size * 0.35, if bold { "bold" } else { "normal" }, fill(ink), esc(&t)).unwrap();
                    }
                }
                s.push_str("</g>");
            }
            Kind::Text { size, family, bold, color, italic, tracking, background, underline, align, .. } => {
                if let Some(bg) = background {
                    let paint = self.defs_paint(f, *bg, id, s);
                    write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" {paint}{st}/>"#, f.x, f.y, f.w, f.h).unwrap();
                } else if !st.is_empty() {
                    write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="none"{st}/>"#, f.x, f.y, f.w, f.h).unwrap();
                }
                write!(
                    s,
                    r#"<text clip-path="url(#{clip})" font-size="{size}" font-family="{}" font-weight="{}" font-style="{}" letter-spacing="{tracking}"{} {}>"#,
                    esc(family),
                    if *bold { "bold" } else { "normal" },
                    if *italic { "italic" } else { "normal" },
                    if *underline { r#" text-decoration="underline""# } else { "" },
                    fill(*color)
                )
                .unwrap();
                if let Some(laid) = laid {
                    for (line, x, y, ws) in &laid.lines {
                        let anchor = match *align {
                            1 => "middle",
                            2 => "end",
                            _ => "start",
                        };
                        let wsa = if *ws > 0.0 { format!(r#" word-spacing="{ws}""#) } else { String::new() };
                        write!(s, r#"<tspan x="{x}" y="{y}" text-anchor="{anchor}"{wsa} xml:space="preserve">{}</tspan>"#, esc(line)).unwrap();
                    }
                }
                s.push_str("</text>");
            }
        }
        s.push_str("</g>\n");
    }

    fn render_frames(&self, s: &mut String, frames: &[Frame], prefix: &str, page_no: u32, measure: Measure) -> Vec<bool> {
        let n = frames.len();
        let is_text = |i: usize| matches!(frames[i].kind, Kind::Text { .. });
        let link = |i: usize| match &frames[i].kind {
            Kind::Text { link: Some(l), .. } if *l < n && *l != i && is_text(*l) => Some(*l),
            _ => None,
        };
        let targets: Vec<bool> = (0..n).map(|j| (0..n).any(|i| link(i) == Some(j))).collect();
        let mut laid: Vec<Option<Laid>> = (0..n).map(|_| None).collect();
        let mut overflow = vec![false; n];
        for head in (0..n).filter(|i| is_text(*i) && !targets[*i]) {
            let mut cur = head;
            let mut incoming: Option<Vec<(String, bool)>> = None;
            let mut seen = vec![false; n];
            loop {
                seen[cur] = true;
                let Some(l) = self.layout(&frames[cur], incoming.as_deref(), page_no, measure, false) else { break };
                let rest = l.leftover.clone();
                match link(cur).filter(|nx| !seen[*nx]) {
                    Some(nx) => {
                        laid[cur] = Some(l);
                        incoming = Some(rest);
                        cur = nx;
                    }
                    None => {
                        overflow[cur] = !rest.is_empty();
                        laid[cur] = Some(l);
                        break;
                    }
                }
            }
        }
        let rest: Vec<usize> = (0..n).filter(|i| is_text(*i) && laid[*i].is_none()).collect();
        for i in rest {
            let l = self.layout(&frames[i], None, page_no, measure, false);
            overflow[i] = l.as_ref().is_some_and(|l| !l.leftover.is_empty());
            laid[i] = l;
        }
        for (i, f) in frames.iter().enumerate() {
            self.frame_svg(s, f, &format!("{prefix}{i}"), laid[i].as_ref(), measure);
        }
        overflow
    }

    /// `index == MASTER` renders the master page alone.
    pub fn to_svg(&self, index: usize, measure: Measure, guides: bool) -> (String, Vec<bool>) {
        self.to_svg_with(index, measure, guides, false)
    }

    /// With `marks`, the page grows by the bleed plus room for crop marks.
    pub fn to_svg_with(&self, index: usize, measure: Measure, guides: bool, marks: bool) -> (String, Vec<bool>) {
        let mut s = String::new();
        let (w, h) = (self.width, self.height);
        let b = if marks { self.bleed.max(0.0) } else { 0.0 };
        let o = if marks { b + 18.0 } else { 0.0 };
        writeln!(
            s,
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{}" height="{}" viewBox="{} {} {} {}"><rect x="{}" y="{}" width="{}" height="{}" {}/>"#,
            w + 2.0 * o,
            h + 2.0 * o,
            -o,
            -o,
            w + 2.0 * o,
            h + 2.0 * o,
            -b,
            -b,
            w + 2.0 * b,
            h + 2.0 * b,
            fill(self.paper)
        )
        .unwrap();
        if guides {
            let m = self.margin;
            writeln!(s, r##"<rect x="{m}" y="{m}" width="{}" height="{}" fill="none" stroke="#ff2fa0" stroke-opacity="0.35" stroke-width="0.6"/>"##, w - 2.0 * m, h - 2.0 * m).unwrap();
            let g = self.grid.max(1);
            if g > 1 {
                let gut = self.grid_gutter;
                let cw = (w - 2.0 * m - gut * (g as f32 - 1.0)) / g as f32;
                for k in 0..g {
                    let x = m + k as f32 * (cw + gut);
                    writeln!(s, r##"<rect x="{x}" y="{m}" width="{cw}" height="{}" fill="#6a5cff" fill-opacity="0.05" stroke="#6a5cff" stroke-opacity="0.25" stroke-width="0.5"/>"##, h - 2.0 * m).unwrap();
                }
            }
            if self.baseline >= 4.0 {
                let mut y = m + self.baseline;
                while y < h - m {
                    writeln!(s, r##"<line x1="{m}" y1="{y}" x2="{}" y2="{y}" stroke="#3fa9f5" stroke-opacity="0.22" stroke-width="0.4"/>"##, w - m).unwrap();
                    y += self.baseline;
                }
            }
            if self.bleed > 0.0 {
                writeln!(s, r##"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="#ff3b30" stroke-opacity="0.5" stroke-width="0.6"/>"##, -self.bleed, -self.bleed, w + 2.0 * self.bleed, h + 2.0 * self.bleed).unwrap();
            }
        }
        let overflow = if index == MASTER {
            self.render_frames(&mut s, &self.master.frames, "m", self.first_number, measure)
        } else {
            let no = self.page_label(index);
            if !self.pages[index].no_master {
                self.render_frames(&mut s, &self.master.frames, "m", no, measure);
            }
            self.render_frames(&mut s, &self.pages[index].frames, "p", no, measure)
        };
        if marks {
            let k = r##"stroke="#000000" stroke-width="0.5""##;
            for (x, y, sx, sy) in [(0.0, 0.0, -1.0, -1.0), (w, 0.0, 1.0, -1.0), (w, h, 1.0, 1.0), (0.0, h, -1.0, 1.0)] {
                writeln!(s, r#"<line x1="{}" y1="{y}" x2="{}" y2="{y}" {k}/><line x1="{x}" y1="{}" x2="{x}" y2="{}" {k}/>"#, x + sx * (b + 3.0), x + sx * o, y + sy * (b + 3.0), y + sy * o).unwrap();
            }
        }
        s.push_str("</svg>\n");
        (s, overflow)
    }
}

fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap(text: &str, max_w: f32, measure: &dyn Fn(&str) -> f32) -> Vec<String> {
        wrap_paras(text, max_w, measure).into_iter().map(|l| l.0).collect()
    }

    fn parse(svg: &str) {
        resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
    }

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
        d.pages[0].frames.push(Frame::new(0.0, 0.0, 50.0, 4.0, Kind::Line { color: [0, 0, 0, 255], width: 2.0, arrow: true }));
        let svg = d.to_svg(0, &measure, true).0;
        assert!(svg.contains("<ellipse") && svg.contains("<line"));
        parse(&svg);
    }

    #[test]
    fn threading_lists_tables_and_print() {
        let measure = |t: &str, _: &str, _: bool, size: f32| t.chars().count() as f32 * size * 0.5;
        let mut d = Doc::a4();
        let long = "un deux trois quatre cinq six sept huit neuf dix onze douze";
        let mut a = Frame::new(0.0, 0.0, 60.0, 14.0, Kind::text(long, 10.0, false));
        if let Kind::Text { link, .. } = &mut a.kind {
            *link = Some(1);
        }
        let b = Frame::new(100.0, 0.0, 60.0, 200.0, Kind::text("ignoré", 10.0, false));
        d.pages[0].frames.push(a);
        d.pages[0].frames.push(b);
        let (svg, over) = d.to_svg(0, &measure, false);
        assert_eq!(over, vec![false, false], "{svg}");
        assert!(!svg.contains("ignoré") && svg.contains("douze") && svg.contains("un deux"));

        let mut l = Kind::text("pomme\npoire\n\nkiwi", 10.0, false);
        if let Kind::Text { list, upper, .. } = &mut l {
            *list = 2;
            *upper = true;
        }
        d.pages[0].frames.push(Frame::new(0.0, 300.0, 200.0, 100.0, l));
        let svg = d.to_svg(0, &measure, false).0;
        assert!(svg.contains("1.\u{a0}POMME") && svg.contains("3.\u{a0}KIWI"));

        let mut j = Kind::text("aa bb cc dd ee ff gg hh", 10.0, false);
        if let Kind::Text { align, .. } = &mut j {
            *align = 3;
        }
        d.pages[0].frames.push(Frame::new(0.0, 500.0, 60.0, 100.0, j));
        let mut t = Kind::table(3, 2);
        if let Kind::Table { cells, .. } = &mut t {
            cells[1][0] = "une cellule beaucoup trop longue".into();
        }
        d.pages[0].frames.push(Frame::new(300.0, 300.0, 200.0, 90.0, t));
        let mut p = Frame::new(300.0, 500.0, 80.0, 80.0, Kind::Polygon { sides: 5, inner: 0.5, fill: [255, 0, 0, 255] });
        p.fill2 = Some([0, 0, 255, 255]);
        p.rotation = 15.0;
        p.shadow = true;
        d.pages[0].frames.push(p);
        let mut img = Frame::new(300.0, 600.0, 80.0, 80.0, Kind::Image { path: String::new(), cover: true, radius: 4.0, filter: 2, flip: true });
        img.hidden = false;
        d.pages[0].frames.push(img);
        d.bleed = 9.0;
        d.title = "Revue".into();
        d.pages[0].frames.push(Frame::new(0.0, 700.0, 300.0, 20.0, Kind::text("{title} {date}", 10.0, false)));
        let (svg, _) = d.to_svg_with(0, &measure, true, true);
        assert!(svg.contains("word-spacing") && svg.contains("Titre 1") && svg.contains('…') && svg.contains("<polygon") && svg.contains("rotate(15"));
        assert!(svg.contains(r#"viewBox="-27 -27"#) && svg.contains("Revue ") && !svg.contains("{date}"));
        parse(&svg);

        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_000), (2024, 10, 4));
        let legacy = r#"{"width":100,"height":100,"margin":10,"pages":[{"frames":[{"x":0,"y":0,"w":10,"h":10,"kind":{"Image":{"path":"","cover":true}}}]}]}"#;
        let old: Doc = serde_json::from_str(legacy).unwrap();
        assert_eq!(old.first_number, 1);
        assert_eq!(old.pages[0].frames[0].grad_angle, 0.0);
    }
}
