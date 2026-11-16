use serde::{Deserialize, Serialize};
use std::fmt::Write;

fn one() -> f32 {
    1.0
}

fn one_u() -> u32 {
    1
}

fn six() -> u32 {
    6
}

fn yes() -> bool {
    true
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub p: [f32; 2],
    pub i: [f32; 2],
    pub o: [f32; 2],
}

impl Node {
    pub fn corner(x: f32, y: f32) -> Node {
        Node { p: [x, y], i: [x, y], o: [x, y] }
    }
    pub fn smooth(&self) -> bool {
        self.i != self.p || self.o != self.p
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextOpts {
    pub small_caps: bool,
    pub strike: bool,
    pub baseline_shift: f32,
    pub hscale: f32,
    pub vscale: f32,
    pub outline: Option<[u8; 4]>,
    pub outline_w: f32,
    pub drop_lines: u32,
    pub hyphen: bool,
    pub tab: f32,
    pub leader: bool,
    pub tab_right: bool,
    pub rule_above: bool,
    pub rule_below: bool,
    pub rule_color: [u8; 4],
    pub rule_w: f32,
    pub rule_off: f32,
    pub bullet: String,
    pub list_start: u32,
    pub hang: bool,
    pub space_before: f32,
    pub indent_left: f32,
    pub indent_right: f32,
    pub orphans: bool,
    pub grid_align: bool,
    pub balance: bool,
    pub col_rule: bool,
    pub auto_h: bool,
    pub no_wrap: bool,
    pub style: String,
}

impl Default for TextOpts {
    fn default() -> Self {
        TextOpts {
            small_caps: false,
            strike: false,
            baseline_shift: 0.0,
            hscale: 1.0,
            vscale: 1.0,
            outline: None,
            outline_w: 0.6,
            drop_lines: 0,
            hyphen: false,
            tab: 36.0,
            leader: false,
            tab_right: false,
            rule_above: false,
            rule_below: false,
            rule_color: [20, 20, 24, 255],
            rule_w: 1.0,
            rule_off: 4.0,
            bullet: "•".into(),
            list_start: 1,
            hang: true,
            space_before: 0.0,
            indent_left: 0.0,
            indent_right: 0.0,
            orphans: false,
            grid_align: false,
            balance: false,
            col_rule: false,
            auto_h: false,
            no_wrap: false,
            style: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fx {
    pub wrap: u8,
    pub wrap_off: f32,
    pub corner: u8,
    pub corner_r: f32,
    pub shadow_dx: f32,
    pub shadow_dy: f32,
    pub shadow_blur: f32,
    pub shadow_color: [u8; 4],
    pub glow: bool,
    pub glow_color: [u8; 4],
    pub glow_size: f32,
    pub feather: f32,
    pub blend: u8,
    pub radial: bool,
    pub gfeather: bool,
    pub gf_angle: f32,
    pub gf_start: f32,
    pub gf_end: f32,
    pub skew: f32,
    pub flip_h: bool,
    pub flip_v: bool,
    pub note: String,
    pub layer: usize,
    pub group: u32,
    pub nonprint: bool,
}

impl Default for Fx {
    fn default() -> Self {
        Fx {
            wrap: 0,
            wrap_off: 8.0,
            corner: 0,
            corner_r: 12.0,
            shadow_dx: 3.0,
            shadow_dy: 4.0,
            shadow_blur: 5.0,
            shadow_color: [0, 0, 0, 80],
            glow: false,
            glow_color: [255, 214, 10, 200],
            glow_size: 6.0,
            feather: 0.0,
            blend: 0,
            radial: false,
            gfeather: false,
            gf_angle: 90.0,
            gf_start: 0.0,
            gf_end: 1.0,
            skew: 0.0,
            flip_h: false,
            flip_v: false,
            note: String::new(),
            layer: 0,
            group: 0,
            nonprint: false,
        }
    }
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
        #[serde(default)]
        opts: TextOpts,
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
        #[serde(default)]
        ox: f32,
        #[serde(default)]
        oy: f32,
        #[serde(default = "one")]
        zoom: f32,
        #[serde(default)]
        stretch: bool,
        #[serde(default)]
        shape: u8,
        #[serde(default = "six")]
        sides: u32,
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
        #[serde(default)]
        start_arrow: bool,
        #[serde(default)]
        cap: u8,
        #[serde(default)]
        dir: u8,
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
        #[serde(default)]
        align: u8,
        #[serde(default)]
        first_col: bool,
    },
    Path {
        nodes: Vec<Node>,
        closed: bool,
        fill: Option<[u8; 4]>,
        color: [u8; 4],
        width: f32,
    },
    TextPath {
        text: String,
        size: f32,
        family: String,
        bold: bool,
        italic: bool,
        color: [u8; 4],
        tracking: f32,
        nodes: Vec<Node>,
        closed: bool,
        offset: f32,
        show_path: bool,
    },
}

pub const FILTERS: [&str; 9] = ["Aucun", "Noir et blanc", "Sépia", "Délavé", "Contraste", "Chaud", "Froid", "Négatif", "Flou"];
pub const LISTS: [&str; 8] = ["Aucune", "Puces", "1.", "1)", "a)", "A.", "I.", "i."];
pub const BULLETS: [&str; 10] = ["•", "–", "▪", "◦", "■", "★", "✓", "→", "❯", "♦"];
pub const BLENDS: [(&str, &str); 16] = [
    ("Normal", "normal"),
    ("Produit", "multiply"),
    ("Superposition", "screen"),
    ("Incrustation", "overlay"),
    ("Obscurcir", "darken"),
    ("Éclaircir", "lighten"),
    ("Densité couleur −", "color-dodge"),
    ("Densité couleur +", "color-burn"),
    ("Lumière crue", "hard-light"),
    ("Lumière tamisée", "soft-light"),
    ("Différence", "difference"),
    ("Exclusion", "exclusion"),
    ("Teinte", "hue"),
    ("Saturation", "saturation"),
    ("Couleur", "color"),
    ("Luminosité", "luminosity"),
];
pub const CORNERS: [&str; 5] = ["Aucun", "Arrondi", "Biseau", "Encoche", "Arrondi inversé"];
pub const WRAPS: [&str; 4] = ["Aucun", "Cadre de sélection", "Forme de l'objet", "Sauter l'objet"];
pub const NUMBERING: [&str; 6] = ["1, 2, 3", "I, II, III", "i, ii, iii", "A, B, C", "a, b, c", "01, 02, 03"];

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
            opts: TextOpts::default(),
        }
    }

    pub fn image(path: &str, radius: f32) -> Kind {
        Kind::Image { path: path.into(), cover: true, radius, filter: 0, flip: false, ox: 0.0, oy: 0.0, zoom: 1.0, stretch: false, shape: 0, sides: 6 }
    }

    pub fn line(color: [u8; 4], width: f32) -> Kind {
        Kind::Line { color, width, arrow: false, start_arrow: false, cap: 0, dir: 0 }
    }

    pub fn table(rows: usize, cols: usize) -> Kind {
        let cells = (0..rows).map(|r| (0..cols).map(|c| if r == 0 { format!("Titre {}", c + 1) } else { String::new() }).collect()).collect();
        Kind::Table { cells, size: 10.0, family: "Segoe UI".into(), color: [20, 20, 24, 255], line: [200, 200, 208, 255], header: Some([30, 30, 40, 255]), stripe: Some([245, 245, 248, 255]), align: 0, first_col: false }
    }

    pub fn label(&self) -> String {
        match self {
            Kind::Text { text, .. } | Kind::TextPath { text, .. } => {
                let v = visible(text);
                let first = v.lines().find(|l| !l.trim().is_empty()).unwrap_or("Texte vide").to_string();
                let s: String = first.chars().take(26).collect();
                if first.chars().count() > 26 { format!("{s}…") } else { s }
            }
            Kind::Image { path, .. } => std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or("Bloc image vide".into()),
            Kind::Rect { .. } => "Rectangle".into(),
            Kind::Ellipse { .. } => "Ellipse".into(),
            Kind::Line { .. } => "Ligne".into(),
            Kind::Polygon { inner, .. } => if *inner < 0.99 { "Étoile".into() } else { "Polygone".into() },
            Kind::Table { cells, .. } => format!("Tableau {}×{}", cells.len(), cells.first().map_or(0, |r| r.len())),
            Kind::Path { closed, .. } => if *closed { "Tracé fermé".into() } else { "Tracé".into() },
        }
    }

    pub fn nodes(&self) -> Vec<Node> {
        let c = Node::corner;
        match self {
            Kind::Path { nodes, .. } | Kind::TextPath { nodes, .. } => nodes.clone(),
            Kind::Ellipse { .. } => {
                let k = 0.5523 * 0.5;
                vec![
                    Node { p: [0.5, 0.0], i: [0.5 - k, 0.0], o: [0.5 + k, 0.0] },
                    Node { p: [1.0, 0.5], i: [1.0, 0.5 - k], o: [1.0, 0.5 + k] },
                    Node { p: [0.5, 1.0], i: [0.5 + k, 1.0], o: [0.5 - k, 1.0] },
                    Node { p: [0.0, 0.5], i: [0.0, 0.5 + k], o: [0.0, 0.5 - k] },
                ]
            }
            Kind::Polygon { sides, inner, .. } => polygon_pts(*sides, *inner).into_iter().map(|p| c(p[0], p[1])).collect(),
            Kind::Line { dir, .. } => {
                let [a, b] = line_ends(*dir);
                vec![c(a[0], a[1]), c(b[0], b[1])]
            }
            _ => vec![c(0.0, 0.0), c(1.0, 0.0), c(1.0, 1.0), c(0.0, 1.0)],
        }
    }

    pub fn closed(&self) -> bool {
        match self {
            Kind::Path { closed, .. } | Kind::TextPath { closed, .. } => *closed,
            Kind::Line { .. } => false,
            _ => true,
        }
    }
}

pub fn line_ends(dir: u8) -> [[f32; 2]; 2] {
    match dir {
        1 => [[0.5, 0.0], [0.5, 1.0]],
        2 => [[0.0, 0.0], [1.0, 1.0]],
        3 => [[0.0, 1.0], [1.0, 0.0]],
        _ => [[0.0, 0.5], [1.0, 0.5]],
    }
}

fn polygon_pts(sides: u32, inner: f32) -> Vec<[f32; 2]> {
    let n = sides.max(3);
    let star = inner < 0.99;
    let count = if star { n * 2 } else { n };
    (0..count)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::TAU / count as f32;
            let r = if star && k % 2 == 1 { inner } else { 1.0 };
            [0.5 + a.cos() * 0.5 * r, 0.5 + a.sin() * 0.5 * r]
        })
        .collect()
}

fn lerp(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

pub fn bez(n0: &Node, n1: &Node, t: f32) -> [f32; 2] {
    let (a, b, c) = (lerp(n0.p, n0.o, t), lerp(n0.o, n1.i, t), lerp(n1.i, n1.p, t));
    lerp(lerp(a, b, t), lerp(b, c, t), t)
}

pub fn insert_node(nodes: &mut Vec<Node>, seg: usize, t: f32) {
    let n = nodes.len();
    if n < 2 {
        return;
    }
    let j = (seg + 1) % n;
    let (n0, n1) = (nodes[seg], nodes[j]);
    let (p01, p12, p23) = (lerp(n0.p, n0.o, t), lerp(n0.o, n1.i, t), lerp(n1.i, n1.p, t));
    let (p012, p123) = (lerp(p01, p12, t), lerp(p12, p23, t));
    let mid = Node { p: lerp(p012, p123, t), i: p012, o: p123 };
    nodes[seg].o = p01;
    nodes[j].i = p23;
    nodes.insert(seg + 1, mid);
}

pub fn nearest_seg(nodes: &[Node], closed: bool, w: f32, h: f32, q: [f32; 2]) -> Option<(usize, f32, f32)> {
    let n = nodes.len();
    let segs = if closed { n } else { n.saturating_sub(1) };
    let mut best: Option<(usize, f32, f32)> = None;
    for s in 0..segs {
        for k in 0..=32 {
            let t = k as f32 / 32.0;
            let p = bez(&nodes[s], &nodes[(s + 1) % n], t);
            let d = ((p[0] - q[0]) * w).hypot((p[1] - q[1]) * h);
            if best.is_none_or(|b| d < b.2) {
                best = Some((s, t, d));
            }
        }
    }
    best
}

pub fn smooth_node(nodes: &mut [Node], k: usize, closed: bool) {
    let n = nodes.len();
    if n < 2 {
        return;
    }
    let prev = if k > 0 { Some(k - 1) } else if closed { Some(n - 1) } else { None };
    let next = if k + 1 < n { Some(k + 1) } else if closed { Some(0) } else { None };
    let p = nodes[k].p;
    let a = prev.map_or(p, |j| nodes[j].p);
    let b = next.map_or(p, |j| nodes[j].p);
    let d = [(b[0] - a[0]) / 6.0, (b[1] - a[1]) / 6.0];
    nodes[k].i = [p[0] - d[0], p[1] - d[1]];
    nodes[k].o = [p[0] + d[0], p[1] + d[1]];
}

pub fn path_d(nodes: &[Node], closed: bool, x: f32, y: f32, w: f32, h: f32) -> String {
    let t = |q: [f32; 2]| format!("{:.2} {:.2}", x + q[0] * w, y + q[1] * h);
    let mut d = String::new();
    for (k, n) in nodes.iter().enumerate() {
        if k == 0 {
            write!(d, "M{}", t(n.p)).unwrap();
        } else {
            let a = &nodes[k - 1];
            write!(d, " C{} {} {}", t(a.o), t(n.i), t(n.p)).unwrap();
        }
    }
    if closed && nodes.len() > 2 {
        let (a, b) = (nodes[nodes.len() - 1], nodes[0]);
        write!(d, " C{} {} {} Z", t(a.o), t(b.i), t(b.p)).unwrap();
    }
    d
}

pub fn corner_d(x: f32, y: f32, w: f32, h: f32, kind: u8, r: f32) -> String {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut d = format!("M{} {}", x + r, y);
    let corners = [((x + w, y), (1.0, 0.0), (0.0, 1.0)), ((x + w, y + h), (0.0, 1.0), (-1.0, 0.0)), ((x, y + h), (-1.0, 0.0), (0.0, -1.0)), ((x, y), (0.0, -1.0), (1.0, 0.0))];
    for ((cx, cy), (ix, iy), (ox, oy)) in corners {
        let a = (cx - ix * r, cy - iy * r);
        let b = (cx + ox * r, cy + oy * r);
        match kind {
            1 => write!(d, " L{} {} A{r} {r} 0 0 1 {} {}", a.0, a.1, b.0, b.1),
            2 => write!(d, " L{} {} L{} {}", a.0, a.1, b.0, b.1),
            3 => write!(d, " L{} {} L{} {} L{} {}", a.0, a.1, a.0 + ox * r, a.1 + oy * r, b.0, b.1),
            4 => write!(d, " L{} {} A{r} {r} 0 0 0 {} {}", a.0, a.1, b.0, b.1),
            _ => write!(d, " L{cx} {cy}"),
        }
        .unwrap();
    }
    d.push_str(" Z");
    d
}

fn fx_path(path: &str, effects: &[studio_fx::stack::Fx]) -> String {
    use std::hash::{Hash, Hasher};
    if !studio_fx::stack::active(effects) {
        return path.to_string();
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    format!("{effects:?}").hash(&mut h);
    std::fs::metadata(path).and_then(|m| m.modified()).ok().hash(&mut h);
    let out = std::env::temp_dir().join("ethone-page-fx").join(format!("{:016x}.png", h.finish()));
    if !out.exists() {
        let Ok(img) = image::open(path) else { return path.to_string() };
        let mut img = img.into_rgba8();
        studio_fx::stack::apply(&mut img, effects);
        let _ = std::fs::create_dir_all(out.parent().unwrap_or(std::path::Path::new(".")));
        if img.save(&out).is_err() {
            return path.to_string();
        }
    }
    out.to_string_lossy().into_owned()
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
    #[serde(default)]
    pub fx: Fx,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<studio_fx::stack::Fx>,
}

impl Frame {
    pub fn new(x: f32, y: f32, w: f32, h: f32, kind: Kind) -> Self {
        Frame { x, y, w, h, kind, opacity: 1.0, stroke: None, stroke_w: 1.0, locked: false, rotation: 0.0, shadow: false, dash: 0.0, fill2: None, grad_angle: 90.0, hidden: false, name: String::new(), fx: Fx::default(), effects: Vec::new() }
    }
    pub fn contains(&self, p: [f32; 2], tol: f32) -> bool {
        p[0] >= self.x - tol && p[0] <= self.x + self.w + tol && p[1] >= self.y - tol && p[1] <= self.y + self.h + tol
    }
    pub fn title(&self) -> String {
        if self.name.is_empty() { self.kind.label() } else { self.name.clone() }
    }
    pub fn center(&self) -> [f32; 2] {
        [self.x + self.w / 2.0, self.y + self.h / 2.0]
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Page {
    pub frames: Vec<Frame>,
    #[serde(default)]
    pub no_master: bool,
    #[serde(default)]
    pub size: Option<[f32; 2]>,
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
    #[serde(default)]
    pub align: u8,
    #[serde(default)]
    pub indent: f32,
    #[serde(default)]
    pub upper: bool,
    #[serde(default)]
    pub space_before: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharStyle {
    pub name: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub strike: bool,
    #[serde(default)]
    pub caps: bool,
    #[serde(default)]
    pub color: Option<[u8; 4]>,
    #[serde(default = "one")]
    pub scale: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjStyle {
    pub name: String,
    pub frame: Frame,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "yes")]
    pub print: bool,
    #[serde(default)]
    pub color: [u8; 4],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub start: usize,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default = "one_u")]
    pub first: u32,
    #[serde(default)]
    pub style: u8,
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
    #[serde(default)]
    pub char_styles: Vec<CharStyle>,
    #[serde(default)]
    pub obj_styles: Vec<ObjStyle>,
    #[serde(default)]
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub sections: Vec<Section>,
    #[serde(default)]
    pub facing: bool,
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

fn parse_hex(s: &str) -> Option<[u8; 4]> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8, 255])
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

pub fn numfmt(n: u32, style: u8) -> String {
    match style {
        1 | 2 => {
            let mut v = n.max(1);
            let mut s = String::new();
            for (val, sym) in [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")] {
                while v >= val {
                    s.push_str(sym);
                    v -= val;
                }
            }
            if style == 2 { s.to_lowercase() } else { s }
        }
        3 | 4 => {
            let mut v = n.max(1);
            let mut s = Vec::new();
            while v > 0 {
                v -= 1;
                s.push((b'A' + (v % 26) as u8) as char);
                v /= 26;
            }
            let s: String = s.into_iter().rev().collect();
            if style == 4 { s.to_lowercase() } else { s }
        }
        5 => format!("{n:02}"),
        _ => n.to_string(),
    }
}

enum Tag {
    On(usize),
    Off(usize),
    Color(Option<[u8; 4]>),
    Style(Option<String>),
}

const TAGS: [&str; 7] = ["b", "i", "u", "s", "sup", "sub", "sc"];

fn tag_at(t: &str) -> Option<(usize, Tag)> {
    if !t.starts_with('<') {
        return None;
    }
    let end = t[..t.len().min(48)].find('>')?;
    let body = &t[1..end];
    if body.chars().any(|c| c == '<' || c == ' ') {
        return None;
    }
    let tag = if let Some(name) = body.strip_prefix('/') {
        match name {
            "c" => Tag::Color(None),
            "st" => Tag::Style(None),
            _ => Tag::Off(TAGS.iter().position(|x| *x == name)?),
        }
    } else if let Some(c) = body.strip_prefix("c=") {
        Tag::Color(Some(parse_hex(c)?))
    } else if let Some(n) = body.strip_prefix("st=") {
        Tag::Style(Some(n.to_string()))
    } else {
        Tag::On(TAGS.iter().position(|x| *x == body)?)
    };
    Some((end + 1, tag))
}

pub fn visible(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(c) = rest.chars().next() {
        if let Some((len, _)) = tag_at(rest) {
            rest = &rest[len..];
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

fn map_outside_tags(t: &str, f: impl Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut buf = String::new();
    let mut rest = t;
    while let Some(c) = rest.chars().next() {
        if let Some((len, _)) = tag_at(rest) {
            out.push_str(&f(&buf));
            buf.clear();
            out.push_str(&rest[..len]);
            rest = &rest[len..];
        } else {
            buf.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out.push_str(&f(&buf));
    out
}

#[derive(Clone, Copy, Default, PartialEq)]
struct Run {
    on: [bool; 7],
    c: Option<[u8; 4]>,
    st: Option<usize>,
}

struct Base<'a> {
    size: f32,
    cstyles: &'a [CharStyle],
    sc: bool,
}

fn emit_runs(s: &mut String, text: &str, st: &mut Run, base: &Base) {
    let mut buf = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some((len, tag)) = tag_at(rest) {
            emit_seg(s, &buf, st, base);
            buf.clear();
            match tag {
                Tag::On(k) => st.on[k] = true,
                Tag::Off(k) => st.on[k] = false,
                Tag::Color(c) => st.c = c,
                Tag::Style(n) => st.st = n.and_then(|n| base.cstyles.iter().position(|c| c.name.replace(' ', "_") == n)),
            }
            rest = &rest[len..];
        } else {
            buf.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    emit_seg(s, &buf, st, base);
}

fn emit_seg(s: &mut String, text: &str, st: &Run, base: &Base) {
    if text.is_empty() {
        return;
    }
    let cs = st.st.and_then(|k| base.cstyles.get(k));
    let mut attrs = String::new();
    if st.on[0] || cs.is_some_and(|c| c.bold) {
        attrs.push_str(r#" font-weight="bold""#);
    }
    if st.on[1] || cs.is_some_and(|c| c.italic) {
        attrs.push_str(r#" font-style="italic""#);
    }
    let deco: Vec<&str> = [(st.on[2] || cs.is_some_and(|c| c.underline), "underline"), (st.on[3] || cs.is_some_and(|c| c.strike), "line-through")].into_iter().filter(|d| d.0).map(|d| d.1).collect();
    if !deco.is_empty() {
        write!(attrs, r#" text-decoration="{}""#, deco.join(" ")).unwrap();
    }
    if let Some(c) = st.c.or(cs.and_then(|c| c.color)) {
        write!(attrs, " {}", fill(c)).unwrap();
    }
    let mut size = base.size * cs.map_or(1.0, |c| c.scale.clamp(0.2, 5.0));
    if st.on[4] || st.on[5] {
        size *= 0.62;
        write!(attrs, r#" baseline-shift="{}""#, if st.on[4] { "super" } else { "sub" }).unwrap();
    }
    if (size - base.size).abs() > 1e-3 {
        write!(attrs, r#" font-size="{size}""#).unwrap();
    }
    let sc = st.on[6] || base.sc || cs.is_some_and(|c| c.caps);
    let body = if sc {
        let mut out = String::new();
        let mut cur = String::new();
        let mut low = false;
        let flush = |out: &mut String, cur: &mut String, low: bool| {
            if cur.is_empty() {
                return;
            }
            if low {
                write!(out, r#"<tspan font-size="{}">{}</tspan>"#, size * 0.78, esc(&cur.to_uppercase())).unwrap();
            } else {
                out.push_str(&esc(cur));
            }
            cur.clear();
        };
        for ch in text.chars() {
            let l = ch.is_lowercase();
            if l != low {
                flush(&mut out, &mut cur, low);
                low = l;
            }
            cur.push(ch);
        }
        flush(&mut out, &mut cur, low);
        out
    } else {
        esc(text)
    };
    if attrs.is_empty() {
        s.push_str(&body);
    } else {
        write!(s, "<tspan{attrs}>{body}</tspan>").unwrap();
    }
}

pub type Measure<'a> = &'a dyn Fn(&str, &str, bool, f32) -> f32;

pub const MASTER: usize = usize::MAX;

#[derive(Clone, Debug, PartialEq)]
pub struct Para {
    pub text: String,
    pub start: bool,
    pub hang: usize,
}

pub struct LaidLine {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub ws: f32,
    pub x0: f32,
    pub x1: f32,
    pub start: bool,
    pub end: bool,
}

pub struct Laid {
    pub lines: Vec<LaidLine>,
    pub leftover: Vec<Para>,
    pub used: f32,
    pub drop: Option<(String, f32, f32, f32)>,
}

struct Metrics<'a> {
    measure: Measure<'a>,
    family: &'a str,
    bold: bool,
    size: f32,
    hs: f32,
    tracking: f32,
    sc: bool,
    tab: f32,
}

impl Metrics<'_> {
    fn m(&self, t: &str) -> f32 {
        let v = visible(t);
        let base = if self.sc { (self.measure)(&v.to_uppercase(), self.family, self.bold, self.size) * 0.86 } else { (self.measure)(&v, self.family, self.bold, self.size) };
        base * self.hs + self.tracking * v.chars().count() as f32
    }
    fn stop(&self, x: f32) -> f32 {
        ((x / self.tab).floor() + 1.0) * self.tab
    }
    fn lw(&self, t: &str) -> f32 {
        if !t.contains('\t') {
            return self.m(t);
        }
        let mut x = 0.0;
        for (k, seg) in t.split('\t').enumerate() {
            if k > 0 {
                x = self.stop(x);
            }
            x += self.m(seg);
        }
        x
    }
}

fn wrap_cut(ob: &Frame, b0: f32, b1: f32) -> Option<(f32, f32)> {
    let off = ob.fx.wrap_off;
    let (x0, x1) = (ob.x - off, ob.x + ob.w + off);
    let ellipse = ob.fx.wrap == 2 && matches!(ob.kind, Kind::Ellipse { .. } | Kind::Image { shape: 1, .. });
    if !ellipse {
        return Some((x0, x1));
    }
    let (cx, cy, rx, ry) = (ob.x + ob.w / 2.0, ob.y + ob.h / 2.0, ob.w / 2.0, ob.h / 2.0);
    let yc = cy.clamp(b0, b1);
    let k = 1.0 - ((yc - cy) / ry.max(0.1)).powi(2);
    if k <= 0.0 {
        return None;
    }
    let hw = rx * k.sqrt();
    Some((cx - hw - off, cx + hw + off))
}

fn widest(left: f32, right: f32, cuts: &[(f32, f32)]) -> (f32, f32) {
    let mut segs = vec![(left, right)];
    for &(a, b) in cuts {
        segs = segs.into_iter().flat_map(|(s0, s1)| if b <= s0 || a >= s1 { vec![(s0, s1)] } else { [(s0, a.max(s0)), (b.min(s1), s1)].into_iter().filter(|s| s.1 - s.0 > 0.5).collect() }).collect();
    }
    segs.into_iter().max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0))).unwrap_or((left, left))
}

pub struct RenderOpts {
    pub guides: bool,
    pub marks: bool,
    pub reg: bool,
    pub info: Option<String>,
    pub print: bool,
    pub transparent: bool,
    pub gray: bool,
}

impl RenderOpts {
    pub fn screen(guides: bool) -> Self {
        RenderOpts { guides, marks: false, reg: false, info: None, print: false, transparent: false, gray: false }
    }
    #[cfg(test)]
    pub fn print(marks: bool) -> Self {
        RenderOpts { guides: false, marks, reg: false, info: None, print: true, transparent: false, gray: false }
    }
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
            char_styles: Vec::new(),
            obj_styles: Vec::new(),
            layers: Vec::new(),
            sections: Vec::new(),
            facing: false,
        }
    }

    pub fn page_size(&self, index: usize) -> (f32, f32) {
        match self.pages.get(index).and_then(|p| p.size) {
            Some([w, h]) => (w, h),
            None => (self.width, self.height),
        }
    }

    fn section(&self, index: usize) -> Option<&Section> {
        self.sections.iter().filter(|s| s.start <= index).max_by_key(|s| s.start)
    }

    pub fn page_label(&self, index: usize) -> String {
        match self.section(index) {
            Some(s) => format!("{}{}", s.prefix, numfmt(s.first + (index - s.start) as u32, s.style)),
            None => (index as u32 + self.first_number).to_string(),
        }
    }

    pub fn layer_shown(&self, l: usize, print: bool) -> bool {
        self.layers.get(l).is_none_or(|x| x.visible && (!print || x.print))
    }

    pub fn layer_locked(&self, l: usize) -> bool {
        self.layers.get(l).is_some_and(|x| x.locked)
    }

    pub fn draw_order(&self, frames: &[Frame]) -> Vec<usize> {
        let top = self.layers.len().saturating_sub(1);
        let mut v: Vec<usize> = (0..frames.len()).collect();
        v.sort_by_key(|&i| frames[i].fx.layer.min(top));
        v
    }

    fn prepare(&self, text: &str, index: usize, upper: bool, list: u8, o: &TextOpts) -> Vec<Para> {
        let total = (self.pages.len() as u32 + self.first_number).saturating_sub(1);
        let label = if index == MASTER { self.first_number.to_string() } else { self.page_label(index) };
        let section = if index == MASTER { String::new() } else { self.section(index).map(|s| s.name.clone()).unwrap_or_default() };
        let mut t = text.replace("{page}", &label).replace("{pages}", &total.to_string()).replace("{title}", &self.title).replace("{section}", &section);
        if t.contains("{date}") {
            t = t.replace("{date}", &today());
        }
        if upper {
            t = map_outside_tags(&t, |s| s.to_uppercase());
        }
        let mut n = o.list_start.max(1) - 1;
        t.split('\n')
            .map(|p| {
                if list == 0 || p.trim().is_empty() {
                    return Para { text: p.to_string(), start: true, hang: 0 };
                }
                n += 1;
                let prefix = match list {
                    1 => format!("{}\u{a0}\u{a0}", if o.bullet.is_empty() { "•" } else { o.bullet.as_str() }),
                    3 => format!("{n})\u{a0}"),
                    4 => format!("{})\u{a0}", numfmt(n, 4)),
                    5 => format!("{}.\u{a0}", numfmt(n, 3)),
                    6 => format!("{}.\u{a0}", numfmt(n, 1)),
                    7 => format!("{}.\u{a0}", numfmt(n, 2)),
                    _ => format!("{n}.\u{a0}"),
                };
                Para { hang: if o.hang { prefix.chars().count() } else { 0 }, text: format!("{prefix}{p}"), start: true }
            })
            .collect()
    }

    /// Lays out a text frame around `obstacles`; what did not fit is returned as leftover paragraphs.
    pub fn layout(&self, f: &Frame, incoming: Option<&[Para]>, index: usize, measure: Measure, unlimited: bool, obstacles: &[&Frame]) -> Option<Laid> {
        let Kind::Text { columns, opts, .. } = &f.kind else { return None };
        let (_, _, _, ih) = inner(f);
        if opts.balance && *columns > 1 && !unlimited {
            let mut g = f.clone();
            if let Kind::Text { columns: c, gutter, padding, opts: go, .. } = &mut g.kind {
                let cols = *c as f32;
                g.w = ((f.w - 2.0 * *padding - *gutter * (cols - 1.0)) / cols).max(1.0) + 2.0 * *padding;
                *c = 1;
                go.balance = false;
            }
            let n = self.layout_lim(&g, incoming, index, measure, true, &[], f32::MAX).map_or(0, |l| l.lines.len());
            if let Kind::Text { size, leading, opts, .. } = &f.kind {
                let step = size * leading * opts.vscale;
                let per = (n as f32 / *columns as f32).ceil().max(1.0);
                let lim = ((per - 1.0) * step + size * 1.05 * opts.vscale + 0.5).min(ih);
                let laid = self.layout_lim(f, incoming, index, measure, false, obstacles, lim)?;
                if laid.leftover.is_empty() {
                    return Some(laid);
                }
            }
        }
        self.layout_lim(f, incoming, index, measure, unlimited, obstacles, ih)
    }

    #[allow(clippy::too_many_arguments)]
    fn layout_lim(&self, f: &Frame, incoming: Option<&[Para]>, index: usize, measure: Measure, unlimited: bool, obstacles: &[&Frame], lim: f32) -> Option<Laid> {
        let Kind::Text { text, size, family, bold, align, leading, tracking, columns, gutter, padding, upper, list, para_space, indent, valign, opts: o, .. } = &f.kind else { return None };
        let cols = (*columns).max(1) as usize;
        let (ix, iy, iw, ih) = inner(f);
        let col_w = ((iw - gutter * (cols as f32 - 1.0)) / cols as f32).max(1.0);
        let mt = Metrics { measure, family, bold: *bold, size: *size, hs: o.hscale.clamp(0.2, 5.0), tracking: *tracking, sc: o.small_caps, tab: o.tab.max(4.0) };
        let mut paras: Vec<Para> = match incoming {
            Some(p) => p.to_vec(),
            None => self.prepare(text, index, *upper, *list, o),
        };
        let vs = o.vscale.clamp(0.2, 5.0);
        let step = size * leading * vs;
        let line_h = size * 1.05 * vs;
        let mut drop = None;
        let mut drop_w = 0.0;
        let drop_n = o.drop_lines as usize;
        if incoming.is_none()
            && drop_n > 1
            && let Some(p0) = paras.first_mut()
            && !p0.text.starts_with('<')
            && let Some(ch) = p0.text.chars().next().filter(|c| !c.is_whitespace())
        {
            let ds = (step * (drop_n as f32 - 1.0) + size * 0.72) / 0.72;
            drop_w = measure(&ch.to_string(), family, *bold, ds) * mt.hs + size * 0.3;
            p0.text = p0.text[ch.len_utf8()..].to_string();
            p0.hang = p0.hang.saturating_sub(1);
            drop = Some((ch.to_string(), ix + o.indent_left, iy + size * 0.85 * vs + step * (drop_n as f32 - 1.0), ds));
        }
        let mut lines: Vec<LaidLine> = Vec::new();
        let (mut col, mut y, mut used) = (0usize, 0.0f32, 0.0f32);
        let mut leftover = Vec::new();
        let mut guard = 0;
        'outer: for (pi, para) in paras.iter().enumerate() {
            let mut words: std::collections::VecDeque<String> = para.text.split(' ').map(String::from).collect();
            let mut first = para.start;
            let mut nth = 0usize;
            if para.start && y > 0.0 {
                y += o.space_before;
            }
            let hang_w = if para.hang > 0 { mt.m(&para.text.chars().take(para.hang).collect::<String>()) } else { 0.0 };
            loop {
                let (seg0, seg1) = loop {
                    guard += 1;
                    if guard > 20_000 {
                        break 'outer;
                    }
                    if !unlimited && y > 0.0 && y + line_h > lim + 0.01 {
                        col += 1;
                        y = 0.0;
                    }
                    if col >= cols {
                        let rest: Vec<String> = words.iter().cloned().collect();
                        leftover.push(Para { text: rest.join(" "), start: first, hang: if first { para.hang } else { 0 } });
                        leftover.extend(paras[pi + 1..].iter().cloned());
                        break 'outer;
                    }
                    let cx = ix + col as f32 * (col_w + gutter);
                    if !unlimited && o.orphans && first && nth == 0 && y > 0.0 && y + step + line_h > lim && mt.lw(&para.text) > col_w - o.indent_left - o.indent_right {
                        col += 1;
                        y = 0.0;
                        continue;
                    }
                    if o.grid_align && self.baseline >= 4.0 {
                        let base = iy + y + size * 0.85 * vs;
                        let g = self.baseline;
                        let snapped = self.margin + ((base - self.margin - 0.01) / g).ceil() * g;
                        y += (snapped - base).max(0.0);
                        if !unlimited && y > 0.0 && y + line_h > lim + 0.01 {
                            continue;
                        }
                    }
                    let mut left = cx + o.indent_left + if first { *indent } else { 0.0 } + if !first { hang_w } else { 0.0 };
                    if drop.is_some() && pi == 0 && col == 0 && nth < drop_n {
                        left += drop_w;
                    }
                    let right = cx + col_w - o.indent_right;
                    let (b0, b1) = (iy + y, iy + y + line_h);
                    let mut jump: Option<f32> = None;
                    let mut cuts = Vec::new();
                    for ob in obstacles {
                        if ob.y + ob.h + ob.fx.wrap_off <= b0 || ob.y - ob.fx.wrap_off >= b1 || ob.x + ob.w + ob.fx.wrap_off <= cx || ob.x - ob.fx.wrap_off >= cx + col_w {
                            continue;
                        }
                        if ob.fx.wrap == 3 {
                            jump = Some(jump.unwrap_or(0.0).max(ob.y + ob.h + ob.fx.wrap_off));
                        } else if let Some(c) = wrap_cut(ob, b0, b1) {
                            cuts.push(c);
                        }
                    }
                    if let Some(j) = jump {
                        y = j - iy + 0.01;
                        continue;
                    }
                    let seg = widest(left, right, &cuts);
                    if seg.1 - seg.0 < size * 1.5 && !cuts.is_empty() {
                        y += (size * 0.5).max(1.0);
                        continue;
                    }
                    break seg;
                };
                let avail = seg1 - seg0;
                let mut line = String::new();
                while let Some(w) = words.front().cloned() {
                    let cand = if line.is_empty() { w.clone() } else { format!("{line} {w}") };
                    if mt.lw(&cand) <= avail {
                        line = cand;
                        words.pop_front();
                        continue;
                    }
                    if o.hyphen && !w.contains('<') {
                        let ch: Vec<char> = w.chars().collect();
                        let mut best = None;
                        if ch.len() >= 6 {
                            for k in (3..=ch.len() - 3).rev() {
                                if !(ch[k - 1].is_alphabetic() && ch[k].is_alphabetic()) {
                                    continue;
                                }
                                let head: String = ch[..k].iter().collect();
                                let c2 = if line.is_empty() { format!("{head}-") } else { format!("{line} {head}-") };
                                if mt.lw(&c2) <= avail {
                                    best = Some((c2, ch[k..].iter().collect::<String>()));
                                    break;
                                }
                            }
                        }
                        if let Some((c2, tail)) = best {
                            line = c2;
                            words[0] = tail;
                            break;
                        }
                    }
                    if line.is_empty() {
                        line = w;
                        words.pop_front();
                    }
                    break;
                }
                let end = words.is_empty();
                let lw = mt.lw(&line);
                let spaces = line.matches(' ').count();
                let (x, ws) = match align {
                    1 => ((seg0 + seg1) / 2.0, 0.0),
                    2 => (seg1, 0.0),
                    3 | 4 if (!end || *align == 4) && spaces > 0 && !line.contains('\t') => (seg0, ((avail - lw) / spaces as f32).max(0.0) / mt.hs),
                    _ => (seg0, 0.0),
                };
                lines.push(LaidLine { text: line, x, y: iy + size * 0.85 * vs + y - o.baseline_shift, ws, x0: seg0, x1: seg1, start: first, end });
                used = used.max(y + line_h);
                y += step + if end { *para_space } else { 0.0 };
                first = false;
                nth += 1;
                if end {
                    break;
                }
            }
        }
        if cols == 1 && !unlimited && *valign > 0 && !lines.is_empty() {
            let extra = (ih - used).max(0.0);
            let n = lines.len();
            for (k, l) in lines.iter_mut().enumerate() {
                l.y += match valign {
                    1 => extra * 0.5,
                    2 => extra,
                    _ if n > 1 => extra * k as f32 / (n - 1) as f32,
                    _ => 0.0,
                };
            }
            if let Some(d) = &mut drop {
                d.2 = lines.get(drop_n.saturating_sub(1).min(n - 1)).map_or(d.2, |l| l.y);
            }
        }
        Some(Laid { lines, leftover, used: used + 2.0 * padding, drop })
    }

    /// Height a text frame needs to show all its lines.
    pub fn fit_height(&self, f: &Frame, measure: Measure) -> Option<f32> {
        let Kind::Text { columns, size, padding, .. } = &f.kind else { return None };
        let cols = (*columns).max(1) as f32;
        let laid = self.layout(f, None, 0, measure, true, &[])?;
        Some(if cols > 1.0 { ((laid.used - 2.0 * padding) / cols + size * 1.2 + 2.0 * padding).ceil() } else { laid.used.ceil() + 1.0 })
    }

    fn defs_paint(&self, f: &Frame, c: [u8; 4], id: &str, s: &mut String) -> String {
        match f.fill2 {
            Some(c2) => {
                let stops = format!(r#"<stop offset="0" stop-color="{}" stop-opacity="{}"/><stop offset="1" stop-color="{}" stop-opacity="{}"/>"#, hex(c), c[3] as f32 / 255.0, hex(c2), c2[3] as f32 / 255.0);
                if f.fx.radial {
                    write!(s, r#"<radialGradient id="g{id}" cx="0.5" cy="0.5" r="0.6">{stops}</radialGradient>"#).unwrap();
                } else {
                    let a = f.grad_angle.to_radians();
                    let (dx, dy) = (a.cos() * 0.5, a.sin() * 0.5);
                    write!(s, r#"<linearGradient id="g{id}" x1="{}" y1="{}" x2="{}" y2="{}">{stops}</linearGradient>"#, 0.5 - dx, 0.5 - dy, 0.5 + dx, 0.5 + dy).unwrap();
                }
                format!(r#"fill="url(#g{id})""#)
            }
            None => fill(c),
        }
    }

    fn box_shape(f: &Frame, radius: f32, attrs: &str) -> String {
        if f.fx.corner > 0 {
            format!(r#"<path d="{}" {attrs}/>"#, corner_d(f.x, f.y, f.w, f.h, f.fx.corner, f.fx.corner_r))
        } else {
            format!(r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{radius}" {attrs}/>"#, f.x, f.y, f.w, f.h)
        }
    }

    fn effects(&self, s: &mut String, f: &Frame, id: &str) -> (String, String) {
        let mut outer = String::new();
        let mut inner = String::new();
        let fx = &f.fx;
        if f.shadow || fx.glow {
            let mut body = String::new();
            let mut merge = String::new();
            if f.shadow {
                let c = fx.shadow_color;
                write!(body, r#"<feGaussianBlur in="SourceAlpha" stdDeviation="{}" result="b1"/><feOffset in="b1" dx="{}" dy="{}" result="o1"/><feFlood flood-color="{}" flood-opacity="{}"/><feComposite in2="o1" operator="in" result="sh"/>"#, fx.shadow_blur.max(0.0) / 2.0, fx.shadow_dx, fx.shadow_dy, hex(c), c[3] as f32 / 255.0).unwrap();
                merge.push_str(r#"<feMergeNode in="sh"/>"#);
            }
            if fx.glow {
                let c = fx.glow_color;
                write!(body, r#"<feGaussianBlur in="SourceAlpha" stdDeviation="{}" result="b2"/><feFlood flood-color="{}" flood-opacity="{}"/><feComposite in2="b2" operator="in" result="gl"/>"#, fx.glow_size.max(0.0) / 2.0, hex(c), c[3] as f32 / 255.0).unwrap();
                merge.push_str(r#"<feMergeNode in="gl"/><feMergeNode in="gl"/>"#);
            }
            writeln!(s, r#"<filter id="s{id}" x="-50%" y="-50%" width="200%" height="200%">{body}<feMerge>{merge}<feMergeNode in="SourceGraphic"/></feMerge></filter>"#).unwrap();
            write!(inner, r#" filter="url(#s{id})""#).unwrap();
        }
        if fx.feather > 0.0 {
            let k = fx.feather;
            let shape = match f.kind {
                Kind::Ellipse { .. } | Kind::Image { shape: 1, .. } => format!(r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="white" filter="url(#fb{id})"/>"#, f.x + f.w / 2.0, f.y + f.h / 2.0, (f.w / 2.0 - k).max(0.5), (f.h / 2.0 - k).max(0.5)),
                _ => format!(r#"<rect x="{}" y="{}" width="{}" height="{}" fill="white" filter="url(#fb{id})"/>"#, f.x + k, f.y + k, (f.w - 2.0 * k).max(0.5), (f.h - 2.0 * k).max(0.5)),
            };
            writeln!(s, r#"<filter id="fb{id}" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="{}"/></filter><mask id="mf{id}" maskUnits="userSpaceOnUse" x="{}" y="{}" width="{}" height="{}">{shape}</mask>"#, k / 2.0, f.x - f.w, f.y - f.h, f.w * 3.0, f.h * 3.0).unwrap();
            write!(outer, r#" mask="url(#mf{id})""#).unwrap();
        }
        if fx.gfeather {
            let a = fx.gf_angle.to_radians();
            let (dx, dy) = (a.cos() * 0.5, a.sin() * 0.5);
            writeln!(
                s,
                r#"<linearGradient id="gm{id}" x1="{}" y1="{}" x2="{}" y2="{}"><stop offset="{}" stop-color="white"/><stop offset="{}" stop-color="black"/></linearGradient><mask id="mg{id}" maskUnits="userSpaceOnUse" x="{}" y="{}" width="{}" height="{}"><rect x="{}" y="{}" width="{}" height="{}" fill="url(#gm{id})"/></mask>"#,
                0.5 - dx,
                0.5 - dy,
                0.5 + dx,
                0.5 + dy,
                fx.gf_start.clamp(0.0, 1.0),
                fx.gf_end.clamp(0.0, 1.0).max(fx.gf_start.clamp(0.0, 1.0)),
                f.x - f.w,
                f.y - f.h,
                f.w * 3.0,
                f.h * 3.0,
                f.x,
                f.y,
                f.w.max(0.1),
                f.h.max(0.1)
            )
            .unwrap();
            write!(inner, r#" mask="url(#mg{id})""#).unwrap();
        }
        if let Some((_, css)) = BLENDS.get(fx.blend as usize).filter(|_| fx.blend > 0) {
            write!(outer, r#" style="mix-blend-mode:{css}""#).unwrap();
        }
        (outer, inner)
    }

    fn text_svg(&self, s: &mut String, f: &Frame, id: &str, laid: &Laid, measure: Measure) {
        let Kind::Text { size, family, bold, color, italic, tracking, underline, align, columns, gutter, opts: o, .. } = &f.kind else { return };
        let (hs, vs) = (o.hscale.clamp(0.2, 5.0), o.vscale.clamp(0.2, 5.0));
        let (fx0, fy0) = (f.x, f.y);
        let tx = |x: f32| fx0 + (x - fx0) / hs;
        let ty = |y: f32| fy0 + (y - fy0) / vs;
        let mt = Metrics { measure, family, bold: *bold, size: *size, hs, tracking: *tracking, sc: o.small_caps, tab: o.tab.max(4.0) };
        let (ix, iy, iw, ih) = inner(f);
        let cols = (*columns).max(1);
        if o.col_rule && cols > 1 {
            let cw = (iw - gutter * (cols as f32 - 1.0)) / cols as f32;
            for k in 1..cols {
                let x = ix + k as f32 * (cw + gutter) - gutter / 2.0;
                write!(s, r#"<line x1="{x}" y1="{iy}" x2="{x}" y2="{}" stroke="{}" stroke-opacity="{}" stroke-width="{}"/>"#, iy + ih, hex(o.rule_color), o.rule_color[3] as f32 / 255.0, o.rule_w * 0.5).unwrap();
            }
        }
        for l in &laid.lines {
            let rule = |s: &mut String, y: f32| {
                write!(s, r#"<line x1="{}" y1="{y}" x2="{}" y2="{y}" stroke="{}" stroke-opacity="{}" stroke-width="{}"/>"#, l.x0, l.x1, hex(o.rule_color), o.rule_color[3] as f32 / 255.0, o.rule_w).unwrap();
            };
            if o.rule_above && l.start {
                rule(s, l.y - size * 0.95 * vs - o.rule_off);
            }
            if o.rule_below && l.end {
                rule(s, l.y + size * 0.3 * vs + o.rule_off);
            }
        }
        let scaled = (hs - 1.0).abs() > 1e-3 || (vs - 1.0).abs() > 1e-3;
        if scaled {
            write!(s, r#"<g transform="matrix({hs} 0 0 {vs} {} {})">"#, fx0 * (1.0 - hs), fy0 * (1.0 - vs)).unwrap();
        }
        let deco: Vec<&str> = [(*underline, "underline"), (o.strike, "line-through")].into_iter().filter(|d| d.0).map(|d| d.1).collect();
        let outline = match o.outline {
            Some(c) => format!(r#" stroke="{}" stroke-opacity="{}" stroke-width="{}" paint-order="stroke""#, hex(c), c[3] as f32 / 255.0, o.outline_w),
            None => String::new(),
        };
        let head = format!(
            r#"font-size="{size}" font-family="{}" font-weight="{}" font-style="{}" letter-spacing="{}"{} {}{outline}"#,
            esc(family),
            if *bold { "bold" } else { "normal" },
            if *italic { "italic" } else { "normal" },
            tracking / hs,
            if deco.is_empty() { String::new() } else { format!(r#" text-decoration="{}""#, deco.join(" ")) },
            fill(*color)
        );
        if let Some((ch, x, y, ds)) = &laid.drop {
            write!(s, r#"<text font-size="{ds}" font-family="{}" font-weight="{}" {}{outline}><tspan x="{}" y="{}">{}</tspan></text>"#, esc(family), if *bold { "bold" } else { "normal" }, fill(*color), tx(*x), ty(*y), esc(ch)).unwrap();
        }
        write!(s, "<text {head}>").unwrap();
        let base = Base { size: *size, cstyles: &self.char_styles, sc: o.small_caps };
        let mut st = Run::default();
        let _ = id;
        for l in &laid.lines {
            let (y, wsa) = (ty(l.y), if l.ws > 0.0 { format!(r#" word-spacing="{}""#, l.ws) } else { String::new() });
            if l.text.contains('\t') && (*align == 0 || *align >= 3) {
                let segs: Vec<&str> = l.text.split('\t').collect();
                let mut pos = 0.0f32;
                for (k, seg) in segs.iter().enumerate() {
                    let mut anchor = "start";
                    let mut at = l.x;
                    if k > 0 {
                        let stop = mt.stop(pos);
                        let right = o.tab_right && k == segs.len() - 1;
                        let target = if right { l.x1 - l.x - mt.m(seg) } else { stop };
                        if o.leader {
                            let dot = mt.m(".").max(0.5);
                            let n = ((target - pos - dot) / dot).floor().max(0.0) as usize;
                            if n > 0 {
                                write!(s, r#"<tspan x="{}" y="{y}" xml:space="preserve">{}</tspan>"#, tx(l.x + pos + dot * 0.5), ".".repeat(n)).unwrap();
                            }
                        }
                        if right {
                            anchor = "end";
                            at = l.x1;
                        } else {
                            at = l.x + stop;
                        }
                        pos = if right { l.x1 - l.x } else { stop };
                    }
                    write!(s, r#"<tspan x="{}" y="{y}" text-anchor="{anchor}" xml:space="preserve">"#, tx(at)).unwrap();
                    emit_runs(s, seg, &mut st, &base);
                    s.push_str("</tspan>");
                    pos += mt.m(seg);
                }
                continue;
            }
            let anchor = match *align {
                1 => "middle",
                2 => "end",
                _ => "start",
            };
            write!(s, r#"<tspan x="{}" y="{y}" text-anchor="{anchor}"{wsa} xml:space="preserve">"#, tx(l.x)).unwrap();
            emit_runs(s, &l.text, &mut st, &base);
            s.push_str("</tspan>");
        }
        s.push_str("</text>");
        if scaled {
            s.push_str("</g>");
        }
    }

    fn frame_svg(&self, s: &mut String, f: &Frame, id: &str, laid: Option<&Laid>, measure: Measure) {
        let clip = format!("c{id}");
        writeln!(s, r#"<clipPath id="{clip}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath>"#, f.x, f.y, f.w.max(0.1), f.h.max(0.1)).unwrap();
        let (cx, cy) = (f.x + f.w / 2.0, f.y + f.h / 2.0);
        let mut tr = Vec::new();
        if f.rotation != 0.0 {
            tr.push(format!("rotate({})", f.rotation));
        }
        if f.fx.skew != 0.0 {
            tr.push(format!("skewX({})", f.fx.skew.clamp(-80.0, 80.0)));
        }
        if f.fx.flip_h || f.fx.flip_v {
            tr.push(format!("scale({} {})", if f.fx.flip_h { -1 } else { 1 }, if f.fx.flip_v { -1 } else { 1 }));
        }
        let rot = if tr.is_empty() { String::new() } else { format!(r#" transform="translate({cx} {cy}) {} translate({} {})""#, tr.join(" "), -cx, -cy) };
        let (outer, inner_attrs) = self.effects(s, f, id);
        write!(s, r#"<g opacity="{}"{rot}{outer}><g{inner_attrs}>"#, f.opacity.clamp(0.0, 1.0)).unwrap();
        let st = stroke(f);
        match &f.kind {
            Kind::Rect { fill: c, radius } => {
                let paint = self.defs_paint(f, *c, id, s);
                s.push_str(&Self::box_shape(f, *radius, &format!("{paint}{st}")));
            }
            Kind::Ellipse { fill: c } => {
                let paint = self.defs_paint(f, *c, id, s);
                write!(s, r#"<ellipse cx="{cx}" cy="{cy}" rx="{}" ry="{}" {paint}{st}/>"#, f.w / 2.0, f.h / 2.0).unwrap();
            }
            Kind::Polygon { sides, inner, fill: c } => {
                let paint = self.defs_paint(f, *c, id, s);
                let pts: Vec<String> = polygon_pts(*sides, *inner).into_iter().map(|p| format!("{:.2},{:.2}", f.x + p[0] * f.w, f.y + p[1] * f.h)).collect();
                write!(s, r#"<polygon points="{}" {paint}{st}/>"#, pts.join(" ")).unwrap();
            }
            Kind::Path { nodes, closed, fill: fc, color, width } => {
                let paint = match fc {
                    Some(c) if *closed => self.defs_paint(f, *c, id, s),
                    _ => r#"fill="none""#.into(),
                };
                let dash = if f.dash > 0.0 { format!(r#" stroke-dasharray="{} {}""#, f.dash, f.dash * 0.7) } else { String::new() };
                let line = if *width > 0.0 { format!(r#" stroke="{}" stroke-opacity="{}" stroke-width="{width}" stroke-linejoin="round" stroke-linecap="round"{dash}"#, hex(*color), color[3] as f32 / 255.0) } else { String::new() };
                write!(s, r#"<path d="{}" {paint}{line}/>"#, path_d(nodes, *closed, f.x, f.y, f.w, f.h)).unwrap();
            }
            Kind::TextPath { text, size, family, bold, italic, color, tracking, nodes, closed, offset, show_path } => {
                let d = path_d(nodes, *closed, f.x, f.y, f.w, f.h);
                write!(s, r#"<path id="tp{id}" d="{d}" fill="none""#).unwrap();
                if *show_path {
                    s.push_str(&st);
                    if st.is_empty() {
                        s.push_str(r##" stroke="#999999" stroke-width="0.5""##);
                    }
                } else {
                    s.push_str(r#" stroke="none""#);
                }
                s.push_str("/>");
                write!(s, r#"<text font-size="{size}" font-family="{}" font-weight="{}" font-style="{}" letter-spacing="{tracking}" {}><textPath xlink:href="{}" startOffset="{}%">"#, esc(family), if *bold { "bold" } else { "normal" }, if *italic { "italic" } else { "normal" }, fill(*color), format_args!("#tp{id}"), offset.clamp(0.0, 100.0)).unwrap();
                let mut run = Run::default();
                emit_runs(s, &one_line(text), &mut run, &Base { size: *size, cstyles: &self.char_styles, sc: false });
                s.push_str("</textPath></text>");
            }
            Kind::Line { color, width, arrow, start_arrow, cap, dir } => {
                let [a, b] = line_ends(*dir);
                let (mut x1, mut y1, mut x2, mut y2) = (f.x + a[0] * f.w, f.y + a[1] * f.h, f.x + b[0] * f.w, f.y + b[1] * f.h);
                let len = (x2 - x1).hypot(y2 - y1).max(0.01);
                let (ux, uy) = ((x2 - x1) / len, (y2 - y1) / len);
                let head = (width * 4.0).max(6.0);
                let tip = |s: &mut String, ex: f32, ey: f32, dx: f32, dy: f32| {
                    let (bx, by) = (ex - dx * head, ey - dy * head);
                    write!(s, r#"<polygon points="{ex},{ey} {},{} {},{}" {}/>"#, bx - dy * head * 0.55, by + dx * head * 0.55, bx + dy * head * 0.55, by - dx * head * 0.55, fill(*color)).unwrap();
                };
                let (ex, ey, sx, sy) = (x2, y2, x1, y1);
                if *arrow {
                    x2 -= ux * head * 0.6;
                    y2 -= uy * head * 0.6;
                }
                if *start_arrow {
                    x1 += ux * head * 0.6;
                    y1 += uy * head * 0.6;
                }
                let dash = if f.dash > 0.0 { format!(r#" stroke-dasharray="{} {}""#, f.dash, f.dash * 0.7) } else { String::new() };
                let capn = ["round", "square", "butt"][(*cap as usize).min(2)];
                write!(s, r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{}" stroke-opacity="{}" stroke-width="{width}" stroke-linecap="{capn}"{dash}/>"#, hex(*color), color[3] as f32 / 255.0).unwrap();
                if *arrow {
                    tip(s, ex, ey, ux, uy);
                }
                if *start_arrow {
                    tip(s, sx, sy, -ux, -uy);
                }
            }
            Kind::Image { path, cover, radius, filter, flip, ox, oy, zoom, stretch, shape, sides } => {
                let par = if *stretch {
                    "none"
                } else if *cover {
                    "xMidYMid slice"
                } else {
                    "xMidYMid meet"
                };
                let rclip = format!("r{id}");
                let clip_shape = match shape {
                    1 => format!(r#"<ellipse cx="{cx}" cy="{cy}" rx="{}" ry="{}"/>"#, f.w / 2.0, f.h / 2.0),
                    2 => format!(r#"<polygon points="{}"/>"#, polygon_pts(*sides, 1.0).into_iter().map(|p| format!("{:.2},{:.2}", f.x + p[0] * f.w, f.y + p[1] * f.h)).collect::<Vec<_>>().join(" ")),
                    _ => Self::box_shape(f, *radius, ""),
                };
                writeln!(s, r#"<clipPath id="{rclip}">{clip_shape}</clipPath>"#).unwrap();
                let fx = match filter {
                    1 => Some(r#"<feColorMatrix type="saturate" values="0"/>"#),
                    2 => Some(r#"<feColorMatrix type="matrix" values="0.393 0.769 0.189 0 0 0.349 0.686 0.168 0 0 0.272 0.534 0.131 0 0 0 0 0 1 0"/>"#),
                    3 => Some(r#"<feColorMatrix type="saturate" values="0.45"/><feComponentTransfer><feFuncR type="linear" slope="0.82" intercept="0.14"/><feFuncG type="linear" slope="0.82" intercept="0.14"/><feFuncB type="linear" slope="0.82" intercept="0.14"/></feComponentTransfer>"#),
                    4 => Some(r#"<feComponentTransfer><feFuncR type="linear" slope="1.4" intercept="-0.2"/><feFuncG type="linear" slope="1.4" intercept="-0.2"/><feFuncB type="linear" slope="1.4" intercept="-0.2"/></feComponentTransfer>"#),
                    5 => Some(r#"<feColorMatrix type="matrix" values="1.1 0 0 0 0.04 0 1 0 0 0.01 0 0 0.85 0 0 0 0 0 1 0"/>"#),
                    6 => Some(r#"<feColorMatrix type="matrix" values="0.88 0 0 0 0 0 1 0 0 0.01 0 0 1.12 0 0.04 0 0 0 1 0"/>"#),
                    7 => Some(r#"<feComponentTransfer><feFuncR type="table" tableValues="1 0"/><feFuncG type="table" tableValues="1 0"/><feFuncB type="table" tableValues="1 0"/></feComponentTransfer>"#),
                    8 => Some(r#"<feGaussianBlur stdDeviation="3"/>"#),
                    _ => None,
                };
                let fattr = match fx {
                    Some(body) => {
                        writeln!(s, r#"<filter id="i{id}">{body}</filter>"#).unwrap();
                        format!(r#" filter="url(#i{id})""#)
                    }
                    None => String::new(),
                };
                let z = zoom.clamp(0.05, 20.0);
                let (cw, ch) = (f.w * z, f.h * z);
                let (ix, iy) = (f.x + (f.w - cw) / 2.0 + ox, f.y + (f.h - ch) / 2.0 + oy);
                let flip_t = if *flip { format!(r#" transform="translate({} 0) scale(-1 1)""#, 2.0 * ix + cw) } else { String::new() };
                if path.is_empty() {
                    write!(s, r##"<g clip-path="url(#{rclip})"><rect x="{}" y="{}" width="{}" height="{}" fill="#e8e8ee"/><line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#b8b8c4" stroke-width="1"/><line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#b8b8c4" stroke-width="1"/></g>"##, f.x, f.y, f.w, f.h, f.x, f.y, f.x + f.w, f.y + f.h, f.x + f.w, f.y, f.x, f.y + f.h).unwrap();
                } else {
                    write!(s, r#"<g clip-path="url(#{rclip})"><image{fattr}{flip_t} x="{ix}" y="{iy}" width="{cw}" height="{ch}" preserveAspectRatio="{par}" xlink:href="{}"/></g>"#, esc(&fx_path(path, &f.effects))).unwrap();
                }
                if !st.is_empty() {
                    let outline = match shape {
                        1 => format!(r#"<ellipse cx="{cx}" cy="{cy}" rx="{}" ry="{}" fill="none"{st}/>"#, f.w / 2.0, f.h / 2.0),
                        2 => clip_shape.replace("/>", &format!(r#" fill="none"{st}/>"#)),
                        _ => Self::box_shape(f, *radius, &format!(r#"fill="none"{st}"#)),
                    };
                    s.push_str(&outline);
                }
            }
            Kind::Table { cells, size, family, color, line, header, stripe, align, first_col } => {
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
                let anchor = ["start", "middle", "end"][(*align as usize).min(2)];
                write!(s, r#"<g clip-path="url(#{clip})" font-size="{size}" font-family="{}">"#, esc(family)).unwrap();
                for (r, row) in cells.iter().enumerate() {
                    for (c, cell) in row.iter().enumerate() {
                        let bold = (r == 0 && header.is_some()) || (c == 0 && *first_col);
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
                        let x = f.x + c as f32 * cw + [4.0, cw / 2.0, cw - 4.0][(*align as usize).min(2)];
                        write!(s, r#"<text x="{x}" y="{}" text-anchor="{anchor}" font-weight="{}" {}>{}</text>"#, f.y + r as f32 * rh + rh / 2.0 + size * 0.35, if bold { "bold" } else { "normal" }, fill(ink), esc(&t)).unwrap();
                    }
                }
                s.push_str("</g>");
            }
            Kind::Text { background, .. } => {
                if let Some(bg) = background {
                    let paint = self.defs_paint(f, *bg, id, s);
                    s.push_str(&Self::box_shape(f, 0.0, &format!("{paint}{st}")));
                } else if !st.is_empty() {
                    s.push_str(&Self::box_shape(f, 0.0, &format!(r#"fill="none"{st}"#)));
                }
                if let Some(laid) = laid {
                    write!(s, r#"<g clip-path="url(#{clip})">"#).unwrap();
                    self.text_svg(s, f, id, laid, measure);
                    s.push_str("</g>");
                }
            }
        }
        s.push_str("</g></g>\n");
    }

    fn render_frames(&self, s: &mut String, frames: &[Frame], prefix: &str, index: usize, measure: Measure, obstacles: &[&Frame], o: &RenderOpts) -> Vec<bool> {
        let n = frames.len();
        let is_text = |i: usize| matches!(frames[i].kind, Kind::Text { .. });
        let link = |i: usize| match &frames[i].kind {
            Kind::Text { link: Some(l), .. } if *l < n && *l != i && is_text(*l) => Some(*l),
            _ => None,
        };
        let obs = |i: usize| -> Vec<&Frame> {
            match &frames[i].kind {
                Kind::Text { opts, .. } if !opts.no_wrap => obstacles.iter().copied().filter(|f| !std::ptr::eq(*f, &frames[i])).collect(),
                _ => Vec::new(),
            }
        };
        let targets: Vec<bool> = (0..n).map(|j| (0..n).any(|i| link(i) == Some(j))).collect();
        let mut laid: Vec<Option<Laid>> = (0..n).map(|_| None).collect();
        let mut overflow = vec![false; n];
        for head in (0..n).filter(|i| is_text(*i) && !targets[*i]) {
            let mut cur = head;
            let mut incoming: Option<Vec<Para>> = None;
            let mut seen = vec![false; n];
            loop {
                seen[cur] = true;
                let Some(l) = self.layout(&frames[cur], incoming.as_deref(), index, measure, false, &obs(cur)) else { break };
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
            let l = self.layout(&frames[i], None, index, measure, false, &obs(i));
            overflow[i] = l.as_ref().is_some_and(|l| !l.leftover.is_empty());
            laid[i] = l;
        }
        for i in self.draw_order(frames) {
            let f = &frames[i];
            if f.hidden || !self.layer_shown(f.fx.layer, o.print) || (o.print && f.fx.nonprint) {
                continue;
            }
            self.frame_svg(s, f, &format!("{prefix}{i}"), laid[i].as_ref(), measure);
        }
        overflow
    }

    /// `index == MASTER` renders the master page alone.
    pub fn to_svg(&self, index: usize, measure: Measure, guides: bool) -> (String, Vec<bool>) {
        self.to_svg_opts(index, measure, &RenderOpts { print: !guides, ..RenderOpts::screen(guides) })
    }

    #[cfg(test)]
    pub fn to_svg_with(&self, index: usize, measure: Measure, guides: bool, marks: bool) -> (String, Vec<bool>) {
        self.to_svg_opts(index, measure, &RenderOpts { marks, print: !guides, ..RenderOpts::screen(guides) })
    }

    pub fn to_svg_opts(&self, index: usize, measure: Measure, o: &RenderOpts) -> (String, Vec<bool>) {
        let mut s = String::new();
        let (w, h) = if index == MASTER { (self.width, self.height) } else { self.page_size(index) };
        let b = if o.marks { self.bleed.max(0.0) } else { 0.0 };
        let off = if o.marks { b + 18.0 } else { 0.0 };
        writeln!(s, r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{}" height="{}" viewBox="{} {} {} {}">"#, w + 2.0 * off, h + 2.0 * off, -off, -off, w + 2.0 * off, h + 2.0 * off).unwrap();
        if o.gray {
            s.push_str(r#"<filter id="gray"><feColorMatrix type="saturate" values="0"/></filter><g filter="url(#gray)">"#);
        }
        if !o.transparent {
            writeln!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" {}/>"#, -b, -b, w + 2.0 * b, h + 2.0 * b, fill(self.paper)).unwrap();
        }
        if o.guides {
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
        let shown = |f: &&Frame| f.fx.wrap > 0 && !f.hidden && self.layer_shown(f.fx.layer, o.print);
        let overflow = if index == MASTER {
            let obs: Vec<&Frame> = self.master.frames.iter().filter(shown).collect();
            self.render_frames(&mut s, &self.master.frames, "m", MASTER, measure, &obs, o)
        } else {
            let page = &self.pages[index];
            let master: &[Frame] = if page.no_master { &[] } else { &self.master.frames };
            let obs: Vec<&Frame> = master.iter().chain(page.frames.iter()).filter(shown).collect();
            if !master.is_empty() {
                self.render_frames(&mut s, master, "m", index, measure, &obs, o);
            }
            self.render_frames(&mut s, &page.frames, "p", index, measure, &obs, o)
        };
        if o.gray {
            s.push_str("</g>");
        }
        if o.marks {
            let k = r##"stroke="#000000" stroke-width="0.5""##;
            for (x, y, sx, sy) in [(0.0, 0.0, -1.0, -1.0), (w, 0.0, 1.0, -1.0), (w, h, 1.0, 1.0), (0.0, h, -1.0, 1.0)] {
                writeln!(s, r#"<line x1="{}" y1="{y}" x2="{}" y2="{y}" {k}/><line x1="{x}" y1="{}" x2="{x}" y2="{}" {k}/>"#, x + sx * (b + 3.0), x + sx * off, y + sy * (b + 3.0), y + sy * off).unwrap();
            }
            if o.reg {
                let r = 4.0;
                for (x, y) in [(w / 2.0, -off / 2.0 - b / 2.0), (w / 2.0, h + off / 2.0 + b / 2.0), (-off / 2.0 - b / 2.0, h / 2.0), (w + off / 2.0 + b / 2.0, h / 2.0)] {
                    writeln!(s, r#"<circle cx="{x}" cy="{y}" r="{r}" fill="none" {k}/><line x1="{}" y1="{y}" x2="{}" y2="{y}" {k}/><line x1="{x}" y1="{}" x2="{x}" y2="{}" {k}/>"#, x - r * 1.6, x + r * 1.6, y - r * 1.6, y + r * 1.6).unwrap();
                }
            }
            if let Some(info) = &o.info {
                writeln!(s, r##"<text x="{}" y="{}" font-size="5.5" font-family="Segoe UI" fill="#000000">{}</text>"##, b + 6.0, h + b + 12.0, esc(info)).unwrap();
            }
        }
        s.push_str("</svg>\n");
        (s, overflow)
    }

    pub fn spread_svg(&self, pages: &[usize], measure: Measure, o: &RenderOpts) -> String {
        let sizes: Vec<(f32, f32)> = pages.iter().map(|&i| self.page_size(i)).collect();
        let off = if o.marks { self.bleed.max(0.0) + 18.0 } else { 0.0 };
        let total: f32 = sizes.iter().map(|s| s.0 + 2.0 * off).sum();
        let hmax = sizes.iter().map(|s| s.1 + 2.0 * off).fold(0.0, f32::max);
        let mut s = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{total}" height="{hmax}" viewBox="0 0 {total} {hmax}">"#);
        let mut x = 0.0;
        for (k, &i) in pages.iter().enumerate() {
            let (svg, _) = self.to_svg_opts(i, measure, o);
            let svg = svg.replacen("<svg ", &format!(r#"<svg x="{x}" "#), 1);
            let svg = svg.replace(r#"id=""#, &format!(r#"id="s{k}"#)).replace("url(#", &format!("url(#s{k}")).replace(r##"xlink:href="#"##, &format!(r##"xlink:href="#s{k}"##));
            s.push_str(&svg);
            x += sizes[k].0 + 2.0 * off;
        }
        s.push_str("</svg>\n");
        s
    }
}

fn one_line(t: &str) -> String {
    t.replace('\n', " ")
}

fn inner(f: &Frame) -> (f32, f32, f32, f32) {
    let padding = match &f.kind {
        Kind::Text { padding, .. } => *padding,
        _ => 0.0,
    };
    (f.x + padding, f.y + padding, (f.w - 2.0 * padding).max(1.0), (f.h - 2.0 * padding).max(1.0))
}

pub fn today() -> String {
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
    let (y, m, d) = civil(days);
    format!("{d:02}/{m:02}/{y}")
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

    fn parse(svg: &str) {
        resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap_or_else(|e| panic!("{e}\n{svg}"));
    }

    fn mono(t: &str, _: &str, _: bool, size: f32) -> f32 {
        t.chars().count() as f32 * size * 0.5
    }

    #[test]
    fn wrap_svg_overflow() {
        let mut d = Doc::a4();
        let mut k = Kind::text("Page {page}/{pages} un deux trois quatre cinq six", 10.0, false);
        if let Kind::Text { leading, .. } = &mut k {
            *leading = 1.2;
        }
        d.pages[0].frames.push(Frame::new(10.0, 10.0, 100.0, 30.0, k));
        d.pages.push(Page::default());
        let measure = mono;
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
        let mut l = Kind::line([0, 0, 0, 255], 2.0);
        if let Kind::Line { arrow, start_arrow, dir, .. } = &mut l {
            (*arrow, *start_arrow, *dir) = (true, true, 2);
        }
        d.pages[0].frames.push(Frame::new(0.0, 0.0, 50.0, 4.0, l));
        let svg = d.to_svg(0, &measure, true).0;
        assert!(svg.contains("<ellipse") && svg.contains("<line"));
        parse(&svg);
    }

    #[test]
    fn threading_lists_tables_and_print() {
        let measure = mono;
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
        let mut img = Frame::new(300.0, 600.0, 80.0, 80.0, Kind::image("", 4.0));
        if let Kind::Image { filter, flip, .. } = &mut img.kind {
            (*filter, *flip) = (2, true);
        }
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
        assert!(matches!(old.pages[0].frames[0].kind, Kind::Image { zoom: 1.0, sides: 6, .. }));
    }

    #[test]
    fn markup_wrap_effects_paths_sections() {
        let measure = mono;
        let mut d = Doc::a4();
        d.char_styles.push(CharStyle { name: "Accent fort".into(), bold: true, italic: false, underline: false, strike: false, caps: false, color: Some([255, 0, 0, 255]), scale: 1.2 });
        let mut k = Kind::text("Du <b>gras</b>, de l'<i>italique</i>, <c=#ff0000>rouge</c>, H<sub>2</sub>O, x<sup>2</sup>, <st=Accent_fort>style</st> et <sc>Petites</sc>.\nNom\tPrix\nLigne avec un-motextrêmementlong final", 10.0, false);
        if let Kind::Text { opts, upper, .. } = &mut k {
            opts.leader = true;
            opts.tab_right = true;
            opts.hyphen = true;
            opts.drop_lines = 3;
            opts.rule_below = true;
            opts.outline = Some([0, 0, 0, 255]);
            opts.hscale = 0.9;
            *upper = true;
        }
        d.pages[0].frames.push(Frame::new(50.0, 50.0, 300.0, 300.0, k));
        let mut ob = Frame::new(40.0, 40.0, 120.0, 120.0, Kind::Ellipse { fill: [0, 200, 0, 255] });
        ob.fx.wrap = 2;
        ob.fx.feather = 6.0;
        ob.fx.gfeather = true;
        ob.fx.blend = 1;
        ob.fx.glow = true;
        ob.shadow = true;
        d.pages[0].frames.push(ob);
        let (svg, over) = d.to_svg(0, &measure, false);
        assert!(svg.contains(r#"font-weight="bold">GRAS"#) && svg.contains("ROUGE") && svg.contains("baseline-shift") && svg.contains("mix-blend-mode:multiply"), "{svg}");
        assert!(svg.contains("......") && svg.contains(r#"text-anchor="end""#) && !svg.contains("&lt;b"), "{svg}");
        assert_eq!(over, vec![false, false]);
        parse(&svg);

        let laid = d.layout(&d.pages[0].frames[0], None, 0, &measure, false, &[&d.pages[0].frames[1]]).unwrap();
        assert!(laid.lines.iter().take(3).all(|l| l.x0 >= 150.0), "texte habillé autour de l'ellipse");
        assert!(laid.drop.is_some());

        let mut path = Frame::new(100.0, 400.0, 200.0, 100.0, Kind::Path { nodes: Kind::Ellipse { fill: [0; 4] }.nodes(), closed: true, fill: Some([0, 0, 255, 255]), color: [0, 0, 0, 255], width: 2.0 });
        if let Kind::Path { nodes, .. } = &mut path.kind {
            insert_node(nodes, 0, 0.5);
            assert_eq!(nodes.len(), 5);
            smooth_node(nodes, 1, true);
        }
        path.fx.corner = 1;
        d.pages[0].frames.push(path);
        let tp = Kind::TextPath { text: "Sur le tracé".into(), size: 12.0, family: "Arial".into(), bold: false, italic: false, color: [0, 0, 0, 255], tracking: 0.0, nodes: Kind::line([0; 4], 1.0).nodes(), closed: false, offset: 10.0, show_path: true };
        d.pages[0].frames.push(Frame::new(100.0, 600.0, 200.0, 50.0, tp));
        let mut r = Frame::new(300.0, 600.0, 100.0, 100.0, Kind::Rect { fill: [9, 9, 9, 255], radius: 0.0 });
        r.fx.corner = 3;
        r.fx.skew = 10.0;
        r.fx.flip_v = true;
        d.pages[0].frames.push(r);
        let mut np = Frame::new(0.0, 0.0, 10.0, 10.0, Kind::Rect { fill: [1, 2, 3, 255], radius: 0.0 });
        np.fx.nonprint = true;
        d.pages[0].frames.push(np);
        let svg = d.to_svg(0, &measure, false).0;
        assert!(svg.contains("textPath") && svg.contains(" C") && svg.contains("skewX(10)") && !svg.contains("#010203"));
        parse(&svg);
        assert!(d.to_svg(0, &measure, true).0.contains("#010203"));
        let spread = d.spread_svg(&[0, 0], &measure, &RenderOpts::print(false));
        parse(&spread);

        d.pages.extend([Page::default(), Page { size: Some([300.0, 300.0]), ..Page::default() }]);
        d.sections.push(Section { start: 1, name: "Annexes".into(), prefix: "A-".into(), first: 1, style: 1 });
        assert_eq!(d.page_label(0), "1");
        assert_eq!(d.page_label(2), "A-II");
        assert_eq!(d.page_size(2), (300.0, 300.0));
        assert_eq!(numfmt(28, 3), "AB");
        assert_eq!(numfmt(4, 2), "iv");
        d.layers.push(Layer { name: "Fond".into(), visible: true, locked: false, print: true, color: [0; 4] });
        d.layers.push(Layer { name: "Texte".into(), visible: false, locked: false, print: true, color: [0; 4] });
        d.pages[0].frames[0].fx.layer = 1;
        assert!(!d.to_svg(0, &measure, false).0.contains("Prix"));
        assert_eq!(d.draw_order(&d.pages[0].frames)[d.pages[0].frames.len() - 1], 0);
    }
}


/// Parses a CSV/TSV file (separator guessed from the header: ";", "," or tab; quotes allowed).
pub fn parse_csv(s: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let s = s.trim_start_matches('\u{feff}');
    let head = s.lines().next().unwrap_or_default();
    let sep = [';', '\t', ','].into_iter().max_by_key(|c| head.matches(*c).count()).unwrap_or(',');
    let mut rows: Vec<Vec<String>> = Vec::new();
    let (mut row, mut cell, mut quoted) = (Vec::new(), String::new(), false);
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if quoted && it.peek() == Some(&'"') => {
                cell.push('"');
                it.next();
            }
            '"' => quoted = !quoted,
            c if c == sep && !quoted => row.push(std::mem::take(&mut cell)),
            '\r' if !quoted => {}
            '\n' if !quoted => {
                row.push(std::mem::take(&mut cell));
                if row.iter().any(|x| !x.trim().is_empty()) {
                    rows.push(std::mem::take(&mut row));
                }
                row.clear();
            }
            c => cell.push(c),
        }
    }
    row.push(cell);
    if row.iter().any(|x| !x.trim().is_empty()) {
        rows.push(row);
    }
    if rows.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let head: Vec<String> = rows.remove(0).into_iter().map(|h| h.trim().to_string()).collect();
    (head, rows)
}

/// InDesign-style data merge: the document's pages are repeated once per row, with every
/// «Champ» or <<Champ>> in the texts replaced by that row's value.
pub fn data_merge(doc: &Doc, head: &[String], rows: &[Vec<String>]) -> Doc {
    let mut out = doc.clone();
    out.pages.clear();
    for row in rows {
        for page in &doc.pages {
            let mut p = page.clone();
            for f in &mut p.frames {
                if let Kind::Text { text, .. } | Kind::TextPath { text, .. } = &mut f.kind {
                    for (h, v) in head.iter().zip(row.iter().map(|x| x.trim()).chain(std::iter::repeat(""))) {
                        *text = text.replace(&format!("«{h}»"), v).replace(&format!("« {h} »"), v).replace(&format!("<<{h}>>"), v);
                    }
                }
            }
            out.pages.push(p);
        }
    }
    out
}

/// Field names used in the document («Nom», <<Ville>>…).
pub fn merge_fields(doc: &Doc) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for p in &doc.pages {
        for f in &p.frames {
            if let Kind::Text { text, .. } | Kind::TextPath { text, .. } = &f.kind {
                for (open, close) in [("«", "»"), ("<<", ">>")] {
                    let mut rest = text.as_str();
                    while let Some(a) = rest.find(open) {
                        let after = &rest[a + open.len()..];
                        let Some(b) = after.find(close) else { break };
                        let name = after[..b].trim().to_string();
                        if !name.is_empty() && name.len() < 40 && !v.contains(&name) {
                            v.push(name);
                        }
                        rest = &after[b + close.len()..];
                    }
                }
            }
        }
    }
    v
}


#[cfg(test)]
mod merge_tests {
    use super::*;

    #[test]
    fn csv_and_merge() {
        let (h, rows) = parse_csv("Nom;Ville\nAlice;Paris\n\"Dupont; Jean\";\"Lyon \"\"centre\"\"\"\n");
        assert_eq!(h, vec!["Nom", "Ville"]);
        assert_eq!(rows[1], vec!["Dupont; Jean", "Lyon \"centre\""]);
        let (h2, r2) = parse_csv("a,b\r\n1,2\r\n");
        assert_eq!((h2.len(), r2.len()), (2, 1));
        let mut doc = Doc::a4();
        doc.pages = vec![Page { frames: vec![Frame::new(0.0, 0.0, 100.0, 20.0, Kind::text("Bonjour «Nom» de <<Ville>>", 12.0, false))], ..Default::default() }];
        assert_eq!(merge_fields(&doc), vec!["Nom", "Ville"]);
        let m = data_merge(&doc, &h, &rows);
        assert_eq!(m.pages.len(), 2);
        let Kind::Text { text, .. } = &m.pages[1].frames[0].kind else { panic!() };
        assert_eq!(text, "Bonjour Dupont; Jean de Lyon \"centre\"");
    }
}


/// InDesign's Step and Repeat filling the page: positions of copies of a w×h block in a grid
/// inside the margins with `gap` between them (the original keeps the first cell).
pub fn grid_cells(page_w: f32, page_h: f32, margin: f32, w: f32, h: f32, gap: f32) -> Vec<[f32; 2]> {
    let cols = (((page_w - 2.0 * margin + gap) / (w + gap)).floor() as usize).max(1);
    let rows = (((page_h - 2.0 * margin + gap) / (h + gap)).floor() as usize).max(1);
    let used_w = cols as f32 * w + (cols - 1) as f32 * gap;
    let used_h = rows as f32 * h + (rows - 1) as f32 * gap;
    let (x0, y0) = ((page_w - used_w) / 2.0, (page_h - used_h) / 2.0);
    (0..rows).flat_map(|r| (0..cols).map(move |c| [x0 + c as f32 * (w + gap), y0 + r as f32 * (h + gap)])).collect()
}

#[cfg(test)]
mod grid_tests {
    #[test]
    fn business_cards_on_a4() {
        let cells = super::grid_cells(595.0, 842.0, 20.0, 241.0, 156.0, 10.0);
        assert_eq!(cells.len(), 2 * 4);
        assert!((cells[0][0] - (595.0 - (2.0 * 241.0 + 10.0)) / 2.0).abs() < 0.01);
        assert_eq!(super::grid_cells(100.0, 100.0, 0.0, 500.0, 500.0, 0.0).len(), 1);
    }
}
