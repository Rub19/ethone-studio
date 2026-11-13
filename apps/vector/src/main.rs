#![windows_subsystem = "windows"]

mod canvas;
mod dialogs;
mod geom;
#[allow(dead_code)]
mod model;
mod panels;
mod tutos;

use eframe::egui::{self, Color32, Frame, Key, Margin, Pos2, Rect, TextureHandle, Ui, Vec2, pos2, vec2};
use model::{BBox, Board, Doc, Geom, Item, Layer, Node, P};
use resvg::{tiny_skia, usvg};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use studio_ui::pro::{self, CHROME};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tool {
    Select,
    Direct,
    GroupSel,
    Wand,
    Lasso,
    Pen,
    AddPt,
    DelPt,
    ConvPt,
    Curv,
    Text,
    AreaText,
    PathText,
    VText,
    Line,
    Arc,
    Spiral,
    RectGrid,
    PolarGrid,
    Arrow,
    Rect,
    RRect,
    Ellipse,
    Polygon,
    Star,
    Flare,
    Heart,
    Brush,
    Blob,
    Pencil,
    Smooth,
    PathErase,
    Rotate,
    Reflect,
    Scale,
    Distort,
    Reshape,
    Width,
    Warp,
    Twirl,
    Pucker,
    Bloat,
    Builder,
    Bucket,
    Gradient,
    Mesh,
    Picker,
    Measure,
    Scissors,
    Knife,
    Eraser,
    Hand,
    RotView,
    Zoom,
    Board,
}

type Slot = &'static [(Tool, &'static str, &'static str)];

const GROUPS: [Slot; 16] = [
    &[(Tool::Select, "Sélection", "V"), (Tool::Direct, "Sélection directe", "A"), (Tool::GroupSel, "Sélection de groupe", ""), (Tool::Wand, "Baguette magique", "Y"), (Tool::Lasso, "Lasso", "Q")],
    &[(Tool::Pen, "Plume", "P"), (Tool::AddPt, "Ajout de point d'ancrage", "+"), (Tool::DelPt, "Suppression de point d'ancrage", "−"), (Tool::ConvPt, "Conversion de point d'ancrage", "Maj+C"), (Tool::Curv, "Courbure", "Maj+²")],
    &[(Tool::Text, "Texte", "T"), (Tool::AreaText, "Texte captif", ""), (Tool::PathText, "Texte curviligne", ""), (Tool::VText, "Texte vertical", "")],
    &[(Tool::Line, "Segment", "\\"), (Tool::Arc, "Arc", ""), (Tool::Spiral, "Spirale", ""), (Tool::RectGrid, "Grille rectangulaire", ""), (Tool::PolarGrid, "Grille polaire", ""), (Tool::Arrow, "Flèche", "")],
    &[(Tool::Rect, "Rectangle", "M"), (Tool::RRect, "Rectangle arrondi", ""), (Tool::Ellipse, "Ellipse", "L"), (Tool::Polygon, "Polygone", ""), (Tool::Star, "Étoile", ""), (Tool::Flare, "Halo", ""), (Tool::Heart, "Cœur", "")],
    &[(Tool::Brush, "Pinceau", "B"), (Tool::Blob, "Pinceau goutte", "Maj+B"), (Tool::Pencil, "Crayon", "N"), (Tool::Smooth, "Lissage", ""), (Tool::PathErase, "Gomme de tracé", "")],
    &[(Tool::Rotate, "Rotation", "R"), (Tool::Reflect, "Miroir", "O")],
    &[(Tool::Scale, "Échelle", "S"), (Tool::Distort, "Distorsion", ""), (Tool::Reshape, "Remodelage", "")],
    &[(Tool::Width, "Largeur", "Maj+W"), (Tool::Warp, "Déformation", "Maj+R"), (Tool::Twirl, "Torsion", ""), (Tool::Pucker, "Contraction", ""), (Tool::Bloat, "Dilatation", "")],
    &[(Tool::Builder, "Concepteur de forme", "Maj+M"), (Tool::Bucket, "Pot de peinture dynamique", "K")],
    &[(Tool::Gradient, "Dégradé", "G"), (Tool::Mesh, "Filet", "U")],
    &[(Tool::Picker, "Pipette", "I"), (Tool::Measure, "Mesure", "")],
    &[(Tool::Scissors, "Ciseaux", "C"), (Tool::Knife, "Cutter", ""), (Tool::Eraser, "Gomme", "Maj+E")],
    &[(Tool::Hand, "Main", "H"), (Tool::RotView, "Rotation de la vue", "Maj+H")],
    &[(Tool::Zoom, "Zoom", "Z")],
    &[(Tool::Board, "Plan de travail", "Maj+O")],
];

const HOTKEYS: [(Key, bool, Tool); 33] = [
    (Key::V, false, Tool::Select),
    (Key::A, false, Tool::Direct),
    (Key::Y, false, Tool::Wand),
    (Key::Q, false, Tool::Lasso),
    (Key::P, false, Tool::Pen),
    (Key::Plus, false, Tool::AddPt),
    (Key::Equals, false, Tool::AddPt),
    (Key::Minus, false, Tool::DelPt),
    (Key::C, true, Tool::ConvPt),
    (Key::Backtick, true, Tool::Curv),
    (Key::T, false, Tool::Text),
    (Key::Backslash, false, Tool::Line),
    (Key::M, false, Tool::Rect),
    (Key::L, false, Tool::Ellipse),
    (Key::B, false, Tool::Brush),
    (Key::B, true, Tool::Blob),
    (Key::N, false, Tool::Pencil),
    (Key::R, false, Tool::Rotate),
    (Key::O, false, Tool::Reflect),
    (Key::S, false, Tool::Scale),
    (Key::W, true, Tool::Width),
    (Key::R, true, Tool::Warp),
    (Key::M, true, Tool::Builder),
    (Key::K, false, Tool::Bucket),
    (Key::G, false, Tool::Gradient),
    (Key::U, false, Tool::Mesh),
    (Key::I, false, Tool::Picker),
    (Key::C, false, Tool::Scissors),
    (Key::E, true, Tool::Eraser),
    (Key::H, false, Tool::Hand),
    (Key::H, true, Tool::RotView),
    (Key::Z, false, Tool::Zoom),
    (Key::O, true, Tool::Board),
];

fn group_of(t: Tool) -> usize {
    GROUPS.iter().position(|g| g.iter().any(|x| x.0 == t)).unwrap_or(0)
}

fn tool_name(t: Tool) -> &'static str {
    GROUPS.iter().flat_map(|g| g.iter()).find(|x| x.0 == t).map_or("", |x| x.1)
}

const PRESETS: [(&str, &str, f32, f32); 12] = [
    ("Logo", "Carré", 1000.0, 1000.0),
    ("Icône", "Interface", 512.0, 512.0),
    ("Full HD", "Écran 16:9", 1920.0, 1080.0),
    ("4K", "Écran 16:9", 3840.0, 2160.0),
    ("Bannière", "Web", 1500.0, 500.0),
    ("Miniature", "YouTube", 1280.0, 720.0),
    ("Publication", "Réseaux sociaux", 1080.0, 1080.0),
    ("Story", "Réseaux sociaux", 1080.0, 1920.0),
    ("iPhone", "Mobile", 1179.0, 2556.0),
    ("A4", "Impression", 794.0, 1123.0),
    ("A3", "Impression", 1123.0, 1587.0),
    ("Carte de visite", "Impression", 1050.0, 600.0),
];

const FONTS: [&str; 12] = ["Segoe UI", "Arial", "Georgia", "Times New Roman", "Consolas", "Impact", "Verdana", "Trebuchet MS", "Calibri", "Cambria", "Comic Sans MS", "Courier New"];

enum Drag {
    None,
    Move { last: P, start: P, dup: bool },
    Create { start: P },
    Box { start: P, end: P },
    Scale { anchor: P, origs: Vec<(usize, Item)>, from: P, axis: u8 },
    NodePart { node: usize, part: u8, free: bool },
    PenHandle { node: usize },
    ConvHandle { node: usize },
    Rotate { center: P, origs: Vec<(usize, Item)>, start: f32 },
    Reflect { center: P, origs: Vec<(usize, Item)> },
    ScaleTool { center: P, origs: Vec<(usize, Item)>, start: P },
    Distort { corner: usize, orig: Item, bbox: BBox },
    Reshape { last: P },
    Liquify { last: P, targets: Vec<usize> },
    Width { t: f32 },
    Gradient { start: P },
    Band { start: P, end: P },
    Stroke,
    Measure { start: P },
    BoardMove { idx: usize, last: P },
    RotView { start: f32, base: f32 },
    Guide { vertical: bool },
}

struct Style {
    fill: Option<Color32>,
    stroke: Option<Color32>,
    stroke_w: f32,
}

struct NewDoc {
    w: f32,
    h: f32,
    preset: usize,
    transparent: bool,
}

struct Opts {
    sides: u32,
    star_ratio: f32,
    rradius: f32,
    arrow_w: f32,
    turns: f32,
    spiral_ccw: bool,
    arc_slope: f32,
    arc_concave: bool,
    arc_closed: bool,
    grid_rows: u32,
    grid_cols: u32,
    grid_frame: bool,
    polar_rings: u32,
    polar_spokes: u32,
    flare_rays: u32,
    flare_rings: u32,
    pencil_tol: f32,
    brush_w: f32,
    brush_profile: u8,
    blob_w: f32,
    blob_merge: bool,
    brush_r: f32,
    erase_r: f32,
    liquify_r: f32,
    liquify_k: f32,
    wand_tol: f32,
    wand: [bool; 4],
    pick: [bool; 4],
    mesh_r: f32,
    grad_radial: bool,
    text_size: f32,
    text_font: usize,
    measure: Option<(P, P)>,
    scale_strokes: bool,
    nudge: f32,
    snap_angle: f32,
}

#[derive(Clone, PartialEq, Debug)]
enum Dlg {
    Move { dx: f32, dy: f32 },
    Scale { sx: f32, sy: f32, uniform: bool },
    Rotate { deg: f32 },
    Reflect { axis: usize, angle: f32 },
    Shear { deg: f32, vertical: bool },
    Each { sx: f32, sy: f32, dx: f32, dy: f32, deg: f32 },
    Offset { d: f32 },
    Simplify { tol: f32, smooth: bool },
    Round { r: f32 },
    Zigzag { size: f32, ridges: u32, smooth: bool },
    Roughen { size: f32, detail: f32, smooth: bool },
    Pucker { amount: f32 },
    Twist { deg: f32 },
    Scribble { angle: f32, spacing: f32, jitter: f32 },
    Blend { steps: u32 },
    Adjust { hue: f32, sat: f32, light: f32 },
    Repeat { n: u32, dx: f32, dy: f32, rot: f32 },
}

struct DlgState {
    kind: Dlg,
    copy: bool,
    preview: bool,
    base: Doc,
    sel: Vec<usize>,
    applied: Option<(Dlg, bool)>,
}

struct ExportOpts {
    format: usize,
    scale: f32,
    transparent: bool,
    quality: u8,
    scope: usize,
    minify: bool,
}

struct TraceSt {
    item: usize,
    opts: geom::TraceOpts,
    keep_image: bool,
}

struct RecolorSt {
    pairs: Vec<([u8; 4], [u8; 4])>,
    base: Doc,
}

struct VectorApp {
    doc: Doc,
    home: bool,
    recents: Vec<PathBuf>,
    file: Option<PathBuf>,
    saved: Doc,
    undo: Vec<Doc>,
    redo: Vec<Doc>,
    editing: bool,
    sel: Option<usize>,
    extra: Vec<usize>,
    sel_node: Option<usize>,
    tool: Tool,
    style: Style,
    o: Opts,
    zoom: f32,
    rot: f32,
    offset: Vec2,
    area: Rect,
    fit_pending: u8,
    drag: Drag,
    pen: Option<usize>,
    fontdb: Arc<usvg::fontdb::Database>,
    tex: Option<TextureHandle>,
    rendered: Option<(Doc, [f32; 6], [u32; 2], bool)>,
    icon: TextureHandle,
    new_doc: Option<NewDoc>,
    cursor: Option<P>,
    error: Option<String>,
    info: Option<String>,
    title: String,
    help: studio_ui::help::Help,
    grid: f32,
    show_grid: bool,
    snap: bool,
    pencil: Vec<P>,
    mods: (bool, bool, bool),
    clip: Vec<Item>,
    outline_view: bool,
    thirds: bool,
    rulers: bool,
    smart: bool,
    show_guides: bool,
    lock_guides: bool,
    show_edges: bool,
    show_bbox: bool,
    snap_lines: Vec<(usize, f32)>,
    active_board: usize,
    active_layer: usize,
    dock_tab: usize,
    align_to: usize,
    ref_pt: usize,
    lock_ratio: bool,
    last_tf: Option<(Dlg, bool)>,
    dlg: Option<DlgState>,
    export: Option<ExportOpts>,
    trace: Option<TraceSt>,
    recolor: Option<RecolorSt>,
    library: usize,
    symbol_name: String,
}

fn recents_path() -> PathBuf {
    studio_core::data_dir().join("vector").join("recents.txt")
}

fn c32(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

fn arr(c: Color32) -> [u8; 4] {
    c.to_srgba_unmultiplied()
}

fn hsl_adjust(c: [u8; 4], hue: f32, sat: f32, light: f32) -> [u8; 4] {
    let mut h = egui::ecolor::Hsva::from(Color32::from_rgb(c[0], c[1], c[2]));
    h.h = (h.h + hue / 360.0).rem_euclid(1.0);
    h.s = (h.s * (1.0 + sat / 100.0)).clamp(0.0, 1.0);
    h.v = (h.v * (1.0 + light / 100.0)).clamp(0.0, 1.0);
    let o: Color32 = h.into();
    [o.r(), o.g(), o.b(), c[3]]
}

fn map_colors(it: &mut Item, f: &dyn Fn([u8; 4]) -> [u8; 4]) {
    for c in [&mut it.fill, &mut it.fill2, &mut it.stroke].into_iter().flatten() {
        *c = f(*c);
    }
    for s in &mut it.stops {
        s.c = f(s.c);
    }
    for m in &mut it.mesh {
        m.c = f(m.c);
    }
}

#[derive(Clone, Copy)]
struct Cam {
    c: Pos2,
    off: Vec2,
    z: f32,
    s: f32,
    co: f32,
    pc: P,
}

impl Cam {
    fn to_scr(&self, p: P) -> Pos2 {
        let d = [(p[0] - self.pc[0]) * self.z, (p[1] - self.pc[1]) * self.z];
        self.c + self.off + vec2(self.co * d[0] - self.s * d[1], self.s * d[0] + self.co * d[1])
    }
    fn to_doc(&self, q: Pos2) -> P {
        let v = q - self.c - self.off;
        [(self.co * v.x + self.s * v.y) / self.z + self.pc[0], (-self.s * v.x + self.co * v.y) / self.z + self.pc[1]]
    }
}

impl VectorApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        let doc = Doc::new(1000.0, 1000.0);
        let recents = std::fs::read_to_string(recents_path()).map(|s| s.lines().map(PathBuf::from).filter(|p| p.exists()).collect()).unwrap_or_default();
        let mut me = Self {
            saved: doc.clone(),
            doc,
            home: true,
            recents,
            file: None,
            undo: Vec::new(),
            redo: Vec::new(),
            editing: false,
            sel: None,
            extra: Vec::new(),
            sel_node: None,
            tool: Tool::Select,
            style: Style { fill: Some(Color32::from_rgb(255, 176, 46)), stroke: Some(Color32::from_rgb(20, 20, 24)), stroke_w: 2.0 },
            o: Opts {
                sides: 6,
                star_ratio: 0.5,
                rradius: 24.0,
                arrow_w: 16.0,
                turns: 3.0,
                spiral_ccw: false,
                arc_slope: 0.55,
                arc_concave: false,
                arc_closed: false,
                grid_rows: 5,
                grid_cols: 5,
                grid_frame: true,
                polar_rings: 5,
                polar_spokes: 8,
                flare_rays: 16,
                flare_rings: 4,
                pencil_tol: 2.0,
                brush_w: 6.0,
                brush_profile: 1,
                blob_w: 20.0,
                blob_merge: true,
                brush_r: 30.0,
                erase_r: 12.0,
                liquify_r: 60.0,
                liquify_k: 0.5,
                wand_tol: 32.0,
                wand: [true, false, false, false],
                pick: [true, true, true, true],
                mesh_r: 120.0,
                grad_radial: false,
                text_size: 64.0,
                text_font: 0,
                measure: None,
                scale_strokes: true,
                nudge: 1.0,
                snap_angle: 15.0,
            },
            zoom: 1.0,
            rot: 0.0,
            offset: Vec2::ZERO,
            area: Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0)),
            fit_pending: 1,
            drag: Drag::None,
            pen: None,
            fontdb: Arc::new(db),
            tex: None,
            rendered: None,
            icon: studio_ui::icon_texture(&cc.egui_ctx, "vector"),
            new_doc: None,
            cursor: None,
            error: None,
            info: None,
            title: String::new(),
            help: studio_ui::help::Help::new("Vector", tutos::VECTOR),
            grid: 20.0,
            show_grid: false,
            snap: false,
            pencil: Vec::new(),
            mods: (false, false, false),
            clip: Vec::new(),
            outline_view: false,
            thirds: false,
            rulers: true,
            smart: true,
            show_guides: true,
            lock_guides: false,
            show_edges: true,
            show_bbox: true,
            snap_lines: Vec::new(),
            active_board: 0,
            active_layer: 0,
            dock_tab: 0,
            align_to: 0,
            ref_pt: 4,
            lock_ratio: false,
            last_tf: None,
            dlg: None,
            export: None,
            trace: None,
            recolor: None,
            library: 0,
            symbol_name: String::new(),
        };
        if let Some(p) = std::env::args().nth(1) {
            me.open_path(Path::new(&p));
        }
        me
    }

    fn cam(&self) -> Cam {
        let (s, co) = self.rot.to_radians().sin_cos();
        Cam { c: self.area.center(), off: self.offset, z: self.zoom, s, co, pc: [self.doc.width / 2.0, self.doc.height / 2.0] }
    }

    fn modal(&self) -> bool {
        self.new_doc.is_some() || self.dlg.is_some() || self.export.is_some() || self.trace.is_some() || self.recolor.is_some()
    }

    fn set_doc(&mut self, mut doc: Doc, file: Option<PathBuf>) {
        if doc.layers.is_empty() {
            doc.layers.push(Layer { name: "Calque 1".into(), visible: true, locked: false, color: [10, 132, 255] });
        }
        self.saved = doc.clone();
        self.doc = doc;
        self.file = file;
        self.undo.clear();
        self.redo.clear();
        self.sel = None;
        self.extra.clear();
        self.pen = None;
        self.fit_pending = 1;
        self.home = false;
        self.active_board = 0;
        self.active_layer = self.doc.layers.len() - 1;
    }

    fn remember(&mut self, path: &Path) {
        self.recents.retain(|p| p != path);
        self.recents.insert(0, path.to_path_buf());
        self.recents.truncate(8);
        let file = recents_path();
        let _ = std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")));
        let lines: Vec<String> = self.recents.iter().map(|p| p.display().to_string()).collect();
        let _ = std::fs::write(file, lines.join("\n"));
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.doc.clone());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn undo(&mut self) {
        if let Some(d) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, d));
            self.fix_sel();
        }
    }

    fn redo(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, d));
            self.fix_sel();
        }
    }

    fn fix_sel(&mut self) {
        self.pen = None;
        let n = self.doc.items.len();
        if self.sel.is_some_and(|i| i >= n) {
            self.sel = None;
        }
        self.extra.retain(|i| *i < n);
        if self.sel.is_none() {
            self.sel = self.extra.pop();
        }
        self.active_layer = self.active_layer.min(self.doc.layers.len().saturating_sub(1));
        self.active_board = self.active_board.min(self.doc.boards.len());
    }

    fn new_item(&self, geom: Geom) -> Item {
        let kind = match &geom {
            Geom::Rect { .. } => "Rectangle",
            Geom::Ellipse { .. } => "Ellipse",
            Geom::Path { closed: false, .. } => "Tracé",
            Geom::Path { .. } => "Forme",
            Geom::Text { .. } => "Texte",
            Geom::Image { .. } => "Image",
        };
        let is_text = matches!(geom, Geom::Text { .. });
        let is_line = matches!(geom, Geom::Path { closed: false, .. });
        Item {
            name: format!("{kind} {}", self.doc.items.len() + 1),
            geom,
            fill: if is_text { Some(arr(self.style.stroke.or(self.style.fill).unwrap_or(Color32::BLACK))) } else if is_line { None } else { self.style.fill.map(arr) },
            stroke: if is_text { None } else if is_line { Some(arr(self.style.stroke.or(self.style.fill).unwrap_or(Color32::BLACK))) } else { self.style.stroke.map(arr) },
            stroke_w: self.style.stroke_w,
            layer: self.active_layer,
            ..Item::blank()
        }
    }

    fn push_item(&mut self, mut it: Item) -> usize {
        it.layer = it.layer.min(self.doc.layers.len().saturating_sub(1));
        let at = self.doc.items.iter().rposition(|x| x.layer <= it.layer).map_or(0, |p| p + 1);
        self.doc.items.insert(at, it);
        at
    }

    fn add_item(&mut self, it: Item) -> usize {
        let i = self.push_item(it);
        self.sel = Some(i);
        self.extra.clear();
        i
    }

    fn finish_pen(&mut self) {
        if let Some(i) = self.pen.take()
            && let Some(Geom::Path { nodes, .. }) = self.doc.items.get(i).map(|it| &it.geom)
            && nodes.len() < 2
        {
            self.doc.items.remove(i);
            self.sel = None;
        }
    }

    fn set_tool(&mut self, t: Tool) {
        if t != self.tool {
            self.finish_pen();
            self.tool = t;
            self.sel_node = None;
        }
    }

    fn selection(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.sel.into_iter().chain(self.extra.iter().copied()).filter(|i| *i < self.doc.items.len()).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    fn set_selection(&mut self, v: Vec<usize>) {
        self.sel = v.first().copied();
        self.extra = v.into_iter().skip(1).collect();
        self.sel_node = None;
    }

    fn group_members(&self, i: usize) -> Vec<usize> {
        let g = self.doc.items[i].group;
        if g == 0 {
            return vec![i];
        }
        (0..self.doc.items.len()).filter(|k| self.doc.items[*k].group == g).collect()
    }

    fn select_with_group(&mut self, i: usize) {
        let mut m = self.group_members(i);
        m.retain(|k| *k != i);
        self.sel = Some(i);
        self.extra = m;
        self.sel_node = None;
    }

    fn delete_selected(&mut self) {
        let all = self.selection();
        if all.is_empty() {
            return;
        }
        self.checkpoint();
        if let (Some(i), Some(n), [_]) = (self.sel, self.sel_node, all.as_slice())
            && let Geom::Path { nodes, .. } = &mut self.doc.items[i].geom
            && nodes.len() > 2
            && n < nodes.len()
        {
            nodes.remove(n);
            self.sel_node = None;
            return;
        }
        for i in all.into_iter().rev() {
            self.doc.items.remove(i);
        }
        self.sel = None;
        self.extra.clear();
        self.pen = None;
        self.sel_node = None;
    }

    fn insert_copies(&mut self, items: Vec<Item>, d: P) {
        let mut idx = Vec::new();
        let gmap: std::collections::HashMap<u32, u32> = {
            let mut next = self.next_group();
            let mut m = std::collections::HashMap::new();
            for it in &items {
                if it.group != 0 && !m.contains_key(&it.group) {
                    m.insert(it.group, next);
                    next += 1;
                }
            }
            m
        };
        for mut it in items {
            it.translate(d);
            if it.group != 0 {
                it.group = gmap[&it.group];
            }
            idx.push(self.push_item(it));
        }
        let n = idx.len();
        let total = self.doc.items.len();
        let mut fixed: Vec<usize> = idx.clone();
        for k in 0..n {
            fixed[k] = idx[k] + idx[k + 1..].iter().filter(|j| **j <= idx[k]).count();
        }
        fixed.retain(|i| *i < total);
        self.set_selection(fixed);
    }

    fn duplicate_selected(&mut self) {
        let all = self.selection();
        if all.is_empty() {
            return;
        }
        self.checkpoint();
        let items: Vec<Item> = all.iter().map(|i| {
            let mut it = self.doc.items[*i].clone();
            it.name = format!("{} copie", it.name);
            it
        }).collect();
        self.insert_copies(items, [16.0, 16.0]);
        self.last_tf = Some((Dlg::Move { dx: 16.0, dy: 16.0 }, true));
    }

    fn copy_items(&mut self) {
        self.clip = self.selection().into_iter().map(|i| self.doc.items[i].clone()).collect();
    }

    fn paste_items(&mut self, mode: u8) {
        if self.clip.is_empty() {
            return;
        }
        self.checkpoint();
        let mut items = self.clip.clone();
        for it in &mut items {
            it.layer = self.active_layer;
        }
        match mode {
            2 | 3 => {
                let anchor = self.selection();
                let at = if mode == 2 { anchor.last().map_or(self.doc.items.len(), |i| i + 1) } else { anchor.first().copied().unwrap_or(0) };
                let n = items.len();
                for (k, it) in items.into_iter().enumerate() {
                    self.doc.items.insert(at + k, it);
                }
                self.set_selection((at..at + n).collect());
            }
            1 => self.insert_copies(items, [0.0, 0.0]),
            _ => {
                self.insert_copies(items, [20.0, 20.0]);
                self.clip.iter_mut().for_each(|i| i.translate([20.0, 20.0]));
            }
        }
        self.tool = Tool::Select;
    }

    fn select_all(&mut self) {
        let all: Vec<usize> = (0..self.doc.items.len()).filter(|&i| self.doc.editable(i)).collect();
        self.set_selection(all);
        if !matches!(self.tool, Tool::Select | Tool::Direct | Tool::GroupSel) {
            self.tool = Tool::Select;
        }
    }

    fn select_inverse(&mut self) {
        let cur = self.selection();
        let all: Vec<usize> = (0..self.doc.items.len()).filter(|&i| self.doc.editable(i) && !cur.contains(&i)).collect();
        self.set_selection(all);
    }

    fn select_same(&mut self, kind: u8) {
        let Some(i) = self.sel else { return };
        let r = self.doc.items[i].clone();
        let same = |it: &Item| match kind {
            0 => it.fill == r.fill,
            1 => it.stroke == r.stroke,
            2 => (it.stroke_w - r.stroke_w).abs() < 0.01,
            3 => (it.opacity - r.opacity).abs() < 0.01,
            4 => it.blend == r.blend,
            5 => std::mem::discriminant(&it.geom) == std::mem::discriminant(&r.geom),
            6 => it.fill == r.fill && it.stroke == r.stroke && it.stroke_w == r.stroke_w,
            _ => it.sym == r.sym && r.sym != 0,
        };
        let v: Vec<usize> = (0..self.doc.items.len()).filter(|&k| k == i || (self.doc.editable(k) && same(&self.doc.items[k]))).collect();
        self.set_selection(v);
        self.sel = Some(i);
        self.extra.retain(|k| *k != i);
    }

    fn magic_wand(&mut self, i: usize, add: bool) {
        let r = self.doc.items[i].clone();
        let tol = self.o.wand_tol;
        let cd = |a: Option<[u8; 4]>, b: Option<[u8; 4]>| match (a, b) {
            (Some(a), Some(b)) => (0..3).map(|k| (a[k] as f32 - b[k] as f32).abs()).fold(0.0, f32::max) <= tol,
            (None, None) => true,
            _ => false,
        };
        let w = self.o.wand;
        let mut v: Vec<usize> = (0..self.doc.items.len())
            .filter(|&k| {
                let it = &self.doc.items[k];
                self.doc.editable(k) && (!w[0] || cd(it.fill, r.fill)) && (!w[1] || cd(it.stroke, r.stroke)) && (!w[2] || (it.stroke_w - r.stroke_w).abs() <= tol / 10.0) && (!w[3] || (it.opacity - r.opacity).abs() <= tol / 255.0)
            })
            .collect();
        if add {
            v.extend(self.selection());
        }
        v.sort_unstable();
        v.dedup();
        self.set_selection(v);
    }

    fn reference(&self, b: &BBox) -> P {
        let (cx, cy) = ((self.ref_pt % 3) as f32 / 2.0, (self.ref_pt / 3) as f32 / 2.0);
        [b.min[0] + (b.max[0] - b.min[0]) * cx, b.min[1] + (b.max[1] - b.min[1]) * cy]
    }

    fn align_bounds(&self, all: &[usize]) -> BBox {
        match self.align_to {
            1 => {
                let b = &self.doc.all_boards()[self.active_board.min(self.doc.boards.len())];
                BBox { min: [b.x, b.y], max: [b.x + b.w, b.y + b.h] }
            }
            2 => self.sel.map(|i| self.doc.items[i].bbox()).unwrap_or_else(|| self.doc.bounds_of(all).unwrap()),
            _ if all.len() > 1 => self.doc.bounds_of(all).unwrap(),
            _ => {
                let b = &self.doc.all_boards()[self.active_board.min(self.doc.boards.len())];
                BBox { min: [b.x, b.y], max: [b.x + b.w, b.y + b.h] }
            }
        }
    }

    fn align(&mut self, axis: usize, to: f32) {
        let all = self.selection();
        if all.is_empty() {
            return;
        }
        self.checkpoint();
        let b = self.align_bounds(&all);
        let (lo, hi) = (b.min[axis], b.max[axis]);
        for i in all {
            if self.align_to == 2 && Some(i) == self.sel {
                continue;
            }
            let ib = self.doc.items[i].bbox();
            let size = ib.max[axis] - ib.min[axis];
            let target = match to {
                t if t < 0.25 => lo,
                t if t > 0.75 => hi - size,
                _ => (lo + hi - size) / 2.0,
            };
            let mut d = [0.0, 0.0];
            d[axis] = target - ib.min[axis];
            self.doc.items[i].translate(d);
        }
    }

    fn distribute(&mut self, axis: usize, at: f32) {
        let mut all = self.selection();
        if all.len() < 3 {
            return;
        }
        self.checkpoint();
        let key = |b: &BBox| b.min[axis] + (b.max[axis] - b.min[axis]) * at;
        all.sort_by(|a, b| key(&self.doc.items[*a].bbox()).total_cmp(&key(&self.doc.items[*b].bbox())));
        let first = key(&self.doc.items[all[0]].bbox());
        let last = key(&self.doc.items[*all.last().unwrap()].bbox());
        let n = all.len();
        for (k, &i) in all.iter().enumerate().skip(1).take(n - 2) {
            let target = first + (last - first) * k as f32 / (n - 1) as f32;
            let mut d = [0.0, 0.0];
            d[axis] = target - key(&self.doc.items[i].bbox());
            self.doc.items[i].translate(d);
        }
    }

    fn distribute_spacing(&mut self, axis: usize, gap: Option<f32>) {
        let mut all = self.selection();
        if all.len() < 2 || (gap.is_none() && all.len() < 3) {
            return;
        }
        self.checkpoint();
        all.sort_by(|a, b| self.doc.items[*a].bbox().min[axis].total_cmp(&self.doc.items[*b].bbox().min[axis]));
        let boxes: Vec<BBox> = all.iter().map(|i| self.doc.items[*i].bbox()).collect();
        let total: f32 = boxes.iter().map(|b| b.max[axis] - b.min[axis]).sum();
        let span = boxes.last().unwrap().max[axis] - boxes[0].min[axis];
        let gap = gap.unwrap_or((span - total) / (all.len() - 1) as f32);
        let anchor = self.sel.and_then(|s| all.iter().position(|x| *x == s)).filter(|_| self.align_to == 2).unwrap_or(0);
        let mut pos = vec![0.0; all.len()];
        pos[anchor] = boxes[anchor].min[axis];
        for k in anchor + 1..all.len() {
            pos[k] = pos[k - 1] + boxes[k - 1].max[axis] - boxes[k - 1].min[axis] + gap;
        }
        for k in (0..anchor).rev() {
            pos[k] = pos[k + 1] - gap - (boxes[k].max[axis] - boxes[k].min[axis]);
        }
        for (k, &i) in all.iter().enumerate() {
            let mut d = [0.0, 0.0];
            d[axis] = pos[k] - boxes[k].min[axis];
            self.doc.items[i].translate(d);
        }
    }

    fn reorder(&mut self, by: i32) {
        let Some(i) = self.sel else { return };
        let layer = self.doc.items[i].layer;
        let lo = self.doc.items.iter().position(|x| x.layer == layer).unwrap_or(0) as i32;
        let hi = self.doc.items.iter().rposition(|x| x.layer == layer).unwrap_or(0) as i32;
        let to = (i as i32).saturating_add(by).clamp(lo, hi) as usize;
        if to != i {
            self.checkpoint();
            let it = self.doc.items.remove(i);
            self.doc.items.insert(to, it);
            self.sel = Some(to);
            self.extra.clear();
        }
    }

    fn transform_sel(&mut self, m: [f32; 2 * 2], pivot: Option<P>) {
        let all = self.selection();
        let Some(b) = self.doc.bounds_of(&all) else { return };
        let c = pivot.unwrap_or_else(|| self.reference(&b));
        let mut fail = false;
        for i in all {
            fail |= !apply_affine(&mut self.doc.items[i], m, c, self.o.scale_strokes);
        }
        if fail {
            self.info = Some("Certains objets (texte, images) ne peuvent que se déplacer ou s'agrandir : vectorise le texte d'abord".into());
        }
    }

    fn flip(&mut self, axis: usize) {
        if self.sel.is_none() {
            return;
        }
        self.checkpoint();
        self.transform_sel(if axis == 0 { [-1.0, 0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0, -1.0] }, None);
    }

    fn rotate_sel(&mut self, deg: f32) {
        if self.sel.is_none() {
            return;
        }
        self.checkpoint();
        let (s, c) = deg.to_radians().sin_cos();
        self.transform_sel([c, s, -s, c], None);
    }

    fn next_group(&self) -> u32 {
        self.doc.items.iter().map(|i| i.group).max().unwrap_or(0) + 1
    }

    fn group(&mut self) {
        let all = self.selection();
        if all.len() < 2 {
            return;
        }
        self.checkpoint();
        let g = self.next_group();
        let top_layer = all.iter().map(|i| self.doc.items[*i].layer).max().unwrap_or(0);
        for &i in &all {
            self.doc.items[i].group = g;
            self.doc.items[i].layer = top_layer;
        }
        let mut moved: Vec<Item> = all.iter().rev().map(|i| self.doc.items.remove(*i)).collect();
        moved.reverse();
        let at = self.doc.items.iter().rposition(|x| x.layer <= top_layer).map_or(0, |p| p + 1);
        let n = moved.len();
        for (k, it) in moved.into_iter().enumerate() {
            self.doc.items.insert(at + k, it);
        }
        self.set_selection((at..at + n).collect());
    }

    fn ungroup(&mut self) {
        let all = self.selection();
        if all.iter().all(|i| self.doc.items[*i].group == 0) {
            return;
        }
        self.checkpoint();
        for i in all {
            let it = &mut self.doc.items[i];
            it.group = 0;
            it.clip = false;
            it.mask = false;
        }
    }

    fn make_clip(&mut self, mask: bool) {
        let all = self.selection();
        if all.len() < 2 {
            self.info = Some("Sélectionne la forme du masque (au-dessus) et les objets à masquer".into());
            return;
        }
        self.group();
        let all = self.selection();
        let top = *all.last().unwrap();
        let it = &mut self.doc.items[top];
        if mask {
            it.mask = true;
        } else {
            it.clip = true;
        }
    }

    fn release_clip(&mut self) {
        let all = self.selection();
        if !all.iter().any(|i| self.doc.items[*i].clip || self.doc.items[*i].mask) {
            return;
        }
        self.checkpoint();
        for i in all {
            let it = &mut self.doc.items[i];
            if it.clip {
                it.fill = None;
                it.stroke = None;
            }
            it.clip = false;
            it.mask = false;
            it.group = 0;
        }
    }

    fn make_compound(&mut self) {
        let all = self.selection();
        if all.len() < 2 {
            return;
        }
        self.checkpoint();
        let mut subs = Vec::new();
        for &i in &all {
            if let Some((nodes, _)) = self.doc.items[i].path_nodes() {
                subs.push(nodes);
                subs.extend(self.doc.items[i].sub.clone());
            }
        }
        if subs.is_empty() {
            self.undo.pop();
            return;
        }
        let base = self.doc.items[*all.last().unwrap()].clone();
        for &i in all.iter().rev() {
            self.doc.items.remove(i);
        }
        let first = subs.remove(0);
        let it = Item { name: "Tracé transparent composé".into(), geom: Geom::Path { nodes: first, closed: true }, sub: subs, ..base };
        let at = all[0].min(self.doc.items.len());
        self.doc.items.insert(at, it);
        self.set_selection(vec![at]);
    }

    fn release_compound(&mut self) {
        let Some(i) = self.sel else { return };
        if self.doc.items[i].sub.is_empty() {
            return;
        }
        self.checkpoint();
        let subs = std::mem::take(&mut self.doc.items[i].sub);
        let base = self.doc.items[i].clone();
        let mut v = vec![i];
        for (k, s) in subs.into_iter().enumerate() {
            let it = Item { name: format!("{} {}", base.name, k + 2), geom: Geom::Path { nodes: s, closed: true }, ..base.clone() };
            self.doc.items.insert(i + k + 1, it);
            v.push(i + k + 1);
        }
        self.set_selection(v);
    }

    fn join(&mut self) {
        let all = self.selection();
        let open: Vec<usize> = all.iter().copied().filter(|i| matches!(self.doc.items[*i].geom, Geom::Path { closed: false, .. })).collect();
        match open.as_slice() {
            [i] => {
                self.checkpoint();
                if let Geom::Path { closed, .. } = &mut self.doc.items[*i].geom {
                    *closed = true;
                }
            }
            [a, b, ..] => {
                let (Geom::Path { nodes: na, .. }, Geom::Path { nodes: nb, .. }) = (&self.doc.items[*a].geom, &self.doc.items[*b].geom) else { return };
                let (mut na, mut nb) = (na.clone(), nb.clone());
                let ends = [(na[na.len() - 1].p, nb[0].p, false, false), (na[na.len() - 1].p, nb[nb.len() - 1].p, false, true), (na[0].p, nb[0].p, true, false), (na[0].p, nb[nb.len() - 1].p, true, true)];
                let best = ends.iter().min_by(|x, y| model::dist(x.0, x.1).total_cmp(&model::dist(y.0, y.1))).unwrap();
                if best.2 {
                    model::reverse(&mut na);
                }
                if best.3 {
                    model::reverse(&mut nb);
                }
                if model::dist(na[na.len() - 1].p, nb[0].p) < 0.5 {
                    let first = nb.remove(0);
                    na.last_mut().unwrap().cout = first.cout;
                }
                na.extend(nb);
                self.checkpoint();
                if let Geom::Path { nodes, .. } = &mut self.doc.items[*a].geom {
                    *nodes = na;
                }
                self.doc.items.remove(*b);
                self.set_selection(vec![*a]);
            }
            _ => self.info = Some("Joindre : sélectionne un ou deux tracés ouverts".into()),
        }
    }

    fn path_op(&mut self, op: u8) {
        let all = self.selection();
        if all.is_empty() {
            return;
        }
        self.checkpoint();
        for i in all {
            let it = &mut self.doc.items[i];
            if op == 0 {
                it.to_path();
                continue;
            }
            if let Geom::Path { nodes, closed } = &mut it.geom {
                match op {
                    1 => model::smooth(nodes, *closed),
                    2 => {
                        let pts: Vec<P> = nodes.iter().map(|n| n.p).collect();
                        *nodes = model::simplify(&pts, 1.5);
                    }
                    3 => model::reverse(nodes),
                    4 => *closed = !*closed,
                    5 => model::add_midpoints(nodes, *closed),
                    _ => {
                        for n in nodes.iter_mut() {
                            *n = Node::corner(n.p);
                        }
                    }
                }
            }
        }
    }

    fn cleanup(&mut self) {
        self.checkpoint();
        let before = self.doc.items.len();
        self.doc.items.retain(|it| match &it.geom {
            Geom::Path { nodes, .. } => nodes.len() >= 2 && (it.fill.is_some() || it.stroke.is_some() || it.clip || it.mask || !it.mesh.is_empty()),
            Geom::Text { text, .. } => !text.trim().is_empty(),
            _ => true,
        });
        let n = before - self.doc.items.len();
        self.sel = None;
        self.extra.clear();
        self.info = Some(format!("Nettoyage : {n} objet(s) vide(s) supprimé(s)"));
        if n == 0 {
            self.undo.pop();
        }
    }

    fn replace_items(&mut self, idx: &[usize], new: Vec<Item>) {
        let at = idx.iter().copied().max().map_or(self.doc.items.len(), |m| m + 1);
        let layer = idx.first().map_or(self.active_layer, |i| self.doc.items[*i].layer);
        let n = new.len();
        for (k, mut it) in new.into_iter().enumerate() {
            it.layer = layer;
            it.group = 0;
            it.clip = false;
            it.mask = false;
            self.doc.items.insert(at + k, it);
        }
        let mut rm = idx.to_vec();
        rm.sort_unstable();
        for i in rm.iter().rev() {
            self.doc.items.remove(*i);
        }
        let start = at - rm.len();
        self.set_selection((start..start + n).collect());
    }

    fn pathfinder(&mut self, op: usize) {
        let all = self.selection();
        if all.len() < 2 {
            self.info = Some("Pathfinder : sélectionne au moins deux formes".into());
            return;
        }
        let items: Vec<Item> = all.iter().map(|i| self.doc.items[*i].clone()).collect();
        let out = geom::pathfinder(&items, op);
        if out.is_empty() {
            self.info = Some(format!("{} : aucun résultat (les formes se chevauchent-elles ?)", geom::PATHFINDER[op]));
            return;
        }
        self.checkpoint();
        let named: Vec<Item> = out.into_iter().enumerate().map(|(k, mut it)| {
            it.name = format!("{} {}", geom::PATHFINDER[op], k + 1);
            it
        }).collect();
        self.replace_items(&all, named);
    }

    fn outline_selected(&mut self) {
        let all = self.selection();
        let mut done = false;
        let mut snapshot = Some(self.doc.clone());
        for &i in all.iter().rev() {
            let it = self.doc.items[i].clone();
            if it.stroke.is_none() {
                continue;
            }
            let Some(mut out) = geom::outline_stroke(&it) else { continue };
            if let Some(s) = snapshot.take() {
                self.undo.push(s);
                self.redo.clear();
            }
            out.name = format!("{} (contour)", it.name);
            done = true;
            if it.fill.is_some() && !matches!(it.geom, Geom::Path { closed: false, .. }) {
                self.doc.items[i].stroke = None;
                self.doc.items.insert(i + 1, out);
            } else {
                self.doc.items[i] = out;
            }
        }
        if !done {
            self.info = Some("Vectoriser le contour : sélectionne un objet avec un contour".into());
        }
    }

    fn expand(&mut self) {
        let all = self.selection();
        if all.is_empty() {
            return;
        }
        self.checkpoint();
        for &i in all.iter().rev() {
            let it = self.doc.items[i].clone();
            match it.geom {
                Geom::Text { .. } => {
                    if let Some(o) = geom::text_outlines(&it, &usvg::Options { fontdb: self.fontdb.clone(), ..Default::default() }) {
                        self.doc.items[i] = o;
                    }
                }
                _ => {
                    self.doc.items[i].to_path();
                }
            }
        }
    }

    fn text_outlines(&mut self) {
        let all: Vec<usize> = self.selection().into_iter().filter(|i| matches!(self.doc.items[*i].geom, Geom::Text { .. })).collect();
        if all.is_empty() {
            self.info = Some("Vectoriser : sélectionne un texte".into());
            return;
        }
        self.checkpoint();
        let opt = usvg::Options { fontdb: self.fontdb.clone(), ..Default::default() };
        for &i in &all {
            match geom::text_outlines(&self.doc.items[i], &opt) {
                Some(o) => self.doc.items[i] = o,
                None => self.error = Some("Police introuvable : vectorisation impossible".into()),
            }
        }
    }

    fn recolor_all(&mut self, f: &dyn Fn([u8; 4]) -> [u8; 4]) {
        let all = self.selection();
        let targets: Vec<usize> = if all.is_empty() { (0..self.doc.items.len()).collect() } else { all };
        self.checkpoint();
        for i in targets {
            map_colors(&mut self.doc.items[i], f);
        }
    }

    fn swap_fill_stroke(&mut self) {
        let all = self.selection();
        if all.is_empty() {
            std::mem::swap(&mut self.style.fill, &mut self.style.stroke);
            return;
        }
        self.checkpoint();
        for i in all {
            let it = &mut self.doc.items[i];
            std::mem::swap(&mut it.fill, &mut it.stroke);
        }
    }

    fn blend_colors(&mut self, axis: Option<usize>) {
        let mut all = self.selection();
        if all.len() < 3 {
            self.info = Some("Mélange : sélectionne au moins trois objets avec un fond".into());
            return;
        }
        if let Some(a) = axis {
            all.sort_by(|x, y| self.doc.items[*x].bbox().center()[a].total_cmp(&self.doc.items[*y].bbox().center()[a]));
        }
        let (Some(c0), Some(c1)) = (self.doc.items[all[0]].fill, self.doc.items[*all.last().unwrap()].fill) else { return };
        self.checkpoint();
        let n = all.len();
        for (k, i) in all.into_iter().enumerate() {
            let t = k as f32 / (n - 1) as f32;
            self.doc.items[i].fill = Some(std::array::from_fn(|j| (c0[j] as f32 + (c1[j] as f32 - c0[j] as f32) * t).round() as u8));
        }
    }

    fn add_board(&mut self, w: f32, h: f32) {
        self.checkpoint();
        let all = self.doc.all_boards();
        let right = all.iter().map(|b| b.x + b.w).fold(0.0, f32::max);
        self.doc.boards.push(Board { name: format!("Plan de travail {}", all.len() + 1), x: right + 80.0, y: 0.0, w, h });
        self.active_board = self.doc.boards.len();
        self.fit_pending = 2;
    }

    fn delete_board(&mut self, k: usize) {
        if k == 0 || k > self.doc.boards.len() {
            self.info = Some("Le premier plan de travail ne peut pas être supprimé".into());
            return;
        }
        self.checkpoint();
        self.doc.boards.remove(k - 1);
        self.active_board = 0;
    }

    fn fit_board_to(&mut self, b: Option<BBox>) {
        let Some(b) = b else { return };
        self.checkpoint();
        let k = self.active_board;
        if k == 0 {
            let d = [-b.min[0], -b.min[1]];
            for it in &mut self.doc.items {
                it.translate(d);
            }
            for bd in &mut self.doc.boards {
                bd.x += d[0];
                bd.y += d[1];
            }
            for g in &mut self.doc.guides {
                g.pos += if g.vertical { d[0] } else { d[1] };
            }
            self.doc.width = b.size()[0].max(1.0);
            self.doc.height = b.size()[1].max(1.0);
            self.fit_pending = 1;
        } else if let Some(bd) = self.doc.boards.get_mut(k - 1) {
            bd.x = b.min[0];
            bd.y = b.min[1];
            bd.w = b.size()[0].max(1.0);
            bd.h = b.size()[1].max(1.0);
        }
    }

    fn arrange_boards(&mut self) {
        if self.doc.boards.is_empty() {
            return;
        }
        self.checkpoint();
        let cols = ((self.doc.boards.len() + 1) as f32).sqrt().ceil() as usize;
        let (mut x, mut y, mut row_h) = (0.0, 0.0, 0.0f32);
        let (w0, h0) = (self.doc.width, self.doc.height);
        let mut sizes = vec![(w0, h0)];
        sizes.extend(self.doc.boards.iter().map(|b| (b.w, b.h)));
        let mut pos = Vec::new();
        for (k, (w, h)) in sizes.iter().enumerate() {
            if k > 0 && k % cols == 0 {
                x = 0.0;
                y += row_h + 80.0;
                row_h = 0.0;
            }
            pos.push((x, y));
            x += w + 80.0;
            row_h = row_h.max(*h);
        }
        for (k, b) in self.doc.boards.iter_mut().enumerate() {
            b.x = pos[k + 1].0;
            b.y = pos[k + 1].1;
        }
        self.fit_pending = 2;
    }

    fn add_layer(&mut self) {
        self.checkpoint();
        let colors = [[10, 132, 255], [255, 69, 58], [48, 209, 88], [255, 159, 10], [191, 90, 242], [100, 210, 255]];
        let n = self.doc.layers.len();
        self.doc.layers.push(Layer { name: format!("Calque {}", n + 1), visible: true, locked: false, color: colors[n % colors.len()] });
        self.active_layer = n;
    }

    fn delete_layer(&mut self, k: usize) {
        if self.doc.layers.len() < 2 {
            self.info = Some("Un document garde au moins un calque".into());
            return;
        }
        self.checkpoint();
        self.doc.items.retain(|it| it.layer != k);
        for it in &mut self.doc.items {
            if it.layer > k {
                it.layer -= 1;
            }
        }
        self.doc.layers.remove(k);
        self.active_layer = self.active_layer.min(self.doc.layers.len() - 1);
        self.sel = None;
        self.extra.clear();
    }

    fn move_layer(&mut self, k: usize, up: bool) {
        let j = if up { k + 1 } else { k.wrapping_sub(1) };
        if j >= self.doc.layers.len() {
            return;
        }
        self.checkpoint();
        self.doc.layers.swap(k, j);
        for it in &mut self.doc.items {
            if it.layer == k {
                it.layer = j;
            } else if it.layer == j {
                it.layer = k;
            }
        }
        self.doc.items.sort_by_key(|it| it.layer);
        self.active_layer = j;
        self.sel = None;
        self.extra.clear();
    }

    fn sel_to_layer(&mut self, k: usize) {
        let all = self.selection();
        if all.is_empty() {
            return;
        }
        self.checkpoint();
        for &i in &all {
            self.doc.items[i].layer = k;
        }
        let moved: Vec<Item> = all.iter().map(|i| self.doc.items[*i].clone()).collect();
        self.doc.items.sort_by_key(|it| it.layer);
        let v: Vec<usize> = (0..self.doc.items.len()).filter(|i| moved.contains(&self.doc.items[*i])).collect();
        self.set_selection(v);
    }

    fn new_symbol(&mut self) {
        let all = self.selection();
        let Some(b) = self.doc.bounds_of(&all) else {
            self.info = Some("Symbole : sélectionne d'abord une illustration".into());
            return;
        };
        self.checkpoint();
        let items: Vec<Item> = all.iter().map(|i| {
            let mut it = self.doc.items[*i].clone();
            it.translate([-b.min[0], -b.min[1]]);
            it.sym = 0;
            it
        }).collect();
        let name = if self.symbol_name.trim().is_empty() { format!("Symbole {}", self.doc.symbols.len() + 1) } else { self.symbol_name.trim().to_string() };
        self.doc.symbols.push(model::Symbol { name, items });
        let id = self.doc.symbols.len() as u32;
        let g = self.next_group();
        for &i in &all {
            self.doc.items[i].sym = id;
            self.doc.items[i].group = if all.len() > 1 { g } else { 0 };
        }
        self.symbol_name.clear();
    }

    fn place_symbol(&mut self, k: usize) {
        let Some(s) = self.doc.symbols.get(k) else { return };
        let b = self.doc.all_boards()[self.active_board.min(self.doc.boards.len())].clone();
        let items = s.items.clone();
        let Some(sb) = items.iter().map(|i| i.bbox()).reduce(|a, b| a.union(&b)) else { return };
        self.checkpoint();
        let g = if items.len() > 1 { self.next_group() } else { 0 };
        let d = [b.x + (b.w - sb.size()[0]) / 2.0, b.y + (b.h - sb.size()[1]) / 2.0];
        let mut v = Vec::new();
        for mut it in items {
            it.translate(d);
            it.sym = k as u32 + 1;
            it.group = g;
            it.layer = self.active_layer;
            v.push(it);
        }
        for it in v {
            self.push_item(it);
        }
        let idx: Vec<usize> = (0..self.doc.items.len()).filter(|i| self.doc.items[*i].sym == k as u32 + 1 && self.doc.items[*i].group == g).collect();
        self.set_selection(if g == 0 { idx.last().copied().into_iter().collect() } else { idx });
        self.tool = Tool::Select;
    }

    fn redefine_symbol(&mut self, k: usize) {
        let all = self.selection();
        let Some(b) = self.doc.bounds_of(&all) else { return };
        self.checkpoint();
        let id = k as u32 + 1;
        let items: Vec<Item> = all.iter().map(|i| {
            let mut it = self.doc.items[*i].clone();
            it.translate([-b.min[0], -b.min[1]]);
            it
        }).collect();
        self.doc.symbols[k].items = items.clone();
        let mut instances: Vec<(u32, Vec<usize>)> = Vec::new();
        for (i, it) in self.doc.items.iter().enumerate().filter(|(i, it)| it.sym == id && !all.contains(i)) {
            let key = if it.group == 0 { u32::MAX - i as u32 } else { it.group };
            match instances.iter_mut().find(|x| x.0 == key) {
                Some(x) => x.1.push(i),
                None => instances.push((key, vec![i])),
            }
        }
        let sb = items.iter().map(|i| i.bbox()).reduce(|a, b| a.union(&b)).unwrap();
        let mut all_rm = Vec::new();
        let mut adds = Vec::new();
        for (_, idx) in instances {
            let ib = self.doc.bounds_of(&idx).unwrap();
            let (sx, sy) = (ib.size()[0] / sb.size()[0].max(1e-3), ib.size()[1] / sb.size()[1].max(1e-3));
            let layer = self.doc.items[idx[0]].layer;
            let g = if items.len() > 1 { self.next_group() + adds.len() as u32 } else { 0 };
            for it in &items {
                let mut n = it.clone();
                n.map(|p| [ib.min[0] + p[0] * sx, ib.min[1] + p[1] * sy], sx, sy);
                n.sym = id;
                n.layer = layer;
                n.group = g;
                adds.push(n);
            }
            all_rm.extend(idx);
        }
        all_rm.sort_unstable();
        for i in all_rm.iter().rev() {
            self.doc.items.remove(*i);
        }
        for it in adds {
            self.push_item(it);
        }
        self.sel = None;
        self.extra.clear();
        for it in &mut self.doc.items {
            if it.sym == 0 && items.contains(it) {
                it.sym = id;
            }
        }
    }

    fn apply_style(&mut self, src: &Item) {
        let all = self.selection();
        if all.is_empty() {
            self.style.fill = src.fill.map(c32);
            self.style.stroke = src.stroke.map(c32);
            self.style.stroke_w = src.stroke_w;
            return;
        }
        self.checkpoint();
        for i in all {
            self.doc.items[i].copy_look(src);
        }
    }

    fn pick_style(&mut self, src: usize, dst: &[usize]) {
        let s = self.doc.items[src].clone();
        let [f, st, fx, tx] = self.o.pick;
        self.checkpoint();
        for &i in dst {
            if i == src {
                continue;
            }
            let mut full = self.doc.items[i].clone();
            full.copy_look(&s);
            let d = &mut self.doc.items[i];
            if f {
                d.fill = full.fill;
                d.fill2 = full.fill2;
                d.stops = full.stops.clone();
                d.radial = full.radial;
                d.grad_angle = full.grad_angle;
                d.grad_vec = full.grad_vec;
            }
            if st {
                d.stroke = full.stroke;
                d.stroke_w = full.stroke_w;
                d.dash = full.dash;
                d.gap = full.gap;
                d.cap = full.cap;
                d.join = full.join;
                d.profile = full.profile;
            }
            if fx {
                d.opacity = full.opacity;
                d.blend = full.blend;
                d.shadow = full.shadow;
                d.blur = full.blur;
                d.glow_in = full.glow_in;
                d.glow_out = full.glow_out;
                d.feather = full.feather;
                d.rough = full.rough;
                d.grain = full.grain;
            }
            if tx && let (Geom::Text { size, family, bold, .. }, Geom::Text { size: s2, family: f2, bold: b2, .. }) = (&mut d.geom, &s.geom) {
                *size = *s2;
                *family = f2.clone();
                *bold = *b2;
                d.italic = s.italic;
                d.tracking = s.tracking;
                d.leading = s.leading;
                d.case = s.case;
            }
        }
    }

    fn hide_selected(&mut self) {
        let all = self.selection();
        if all.is_empty() {
            return;
        }
        self.checkpoint();
        for i in all {
            self.doc.items[i].visible = false;
        }
        self.sel = None;
        self.extra.clear();
    }

    fn lock_selected(&mut self) {
        let all = self.selection();
        if all.is_empty() {
            return;
        }
        self.checkpoint();
        for i in all {
            self.doc.items[i].locked = true;
        }
        self.sel = None;
        self.extra.clear();
    }

    fn opts(&self) -> usvg::Options<'static> {
        usvg::Options { fontdb: self.fontdb.clone(), ..Default::default() }
    }

    fn rasterize(&self, svg: &str, w: f32, h: f32, scale: f32) -> Result<tiny_skia::Pixmap, String> {
        let tree = usvg::Tree::from_str(svg, &self.opts()).map_err(|e| e.to_string())?;
        let mut pix = tiny_skia::Pixmap::new((w * scale).round().max(1.0) as u32, (h * scale).round().max(1.0) as u32).ok_or("taille invalide")?;
        let sx = pix.width() as f32 / tree.size().width();
        resvg::render(&tree, tiny_skia::Transform::from_scale(sx, sx), &mut pix.as_mut());
        Ok(pix)
    }

    fn export_area(&self, scope: usize) -> Vec<(String, [f32; 4], Doc)> {
        let boards = self.doc.all_boards();
        match scope {
            2 => {
                let all = self.selection();
                let Some(b) = self.doc.bounds_of(&all) else { return Vec::new() };
                let mut d = self.doc.clone();
                d.items = all.iter().map(|i| d.items[*i].clone()).collect();
                let m = 4.0;
                vec![("selection".into(), [b.min[0] - m, b.min[1] - m, b.size()[0] + 2.0 * m, b.size()[1] + 2.0 * m], d)]
            }
            1 => boards.iter().map(|b| (b.name.clone(), [b.x, b.y, b.w, b.h], self.doc.clone())).collect(),
            _ => {
                let b = &boards[self.active_board.min(boards.len() - 1)];
                vec![(b.name.clone(), [b.x, b.y, b.w, b.h], self.doc.clone())]
            }
        }
    }

    fn run_export(&mut self, o: &ExportOpts) {
        let ext = ["png", "jpg", "svg", "pdf"][o.format];
        let parts = self.export_area(o.scope);
        if parts.is_empty() {
            self.error = Some("Rien à exporter".into());
            return;
        }
        let base = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("export".into());
        let single = parts.len() == 1 || o.format == 3;
        let target = if single {
            rfd::FileDialog::new().add_filter(ext.to_uppercase(), &[ext]).set_file_name(format!("{base}.{ext}")).save_file()
        } else {
            rfd::FileDialog::new().set_title("Dossier d'export").pick_folder()
        };
        let Some(target) = target else { return };
        let result = (|| -> Result<(), String> {
            if o.format == 3 {
                use krilla_svg::{SurfaceExt, SvgSettings};
                let mut db = resvg47::usvg::fontdb::Database::new();
                db.load_system_fonts();
                let opt = resvg47::usvg::Options { fontdb: Arc::new(db), ..Default::default() };
                let mut document = krilla::Document::new();
                for (_, r, d) in &parts {
                    let svg = d.export_svg(r[0], r[1], r[2], r[3], 1.0);
                    let tree = resvg47::usvg::Tree::from_str(&svg, &opt).map_err(|e| e.to_string())?;
                    let size = krilla::geom::Size::from_wh(r[2], r[3]).ok_or("taille invalide")?;
                    let mut page = document.start_page_with(krilla::page::PageSettings::new(size));
                    let mut surface = page.surface();
                    surface.draw_svg(&tree, size, SvgSettings::default()).ok_or("rendu PDF impossible")?;
                    surface.finish();
                    page.finish();
                }
                let bytes = document.finish().map_err(|e| format!("{e:?}"))?;
                return std::fs::write(&target, bytes).map_err(|e| e.to_string());
            }
            for (k, (name, r, d)) in parts.iter().enumerate() {
                let path = if single { target.clone() } else { target.join(format!("{base}-{:02}-{}.{ext}", k + 1, name.replace(['/', '\\', ':'], "-"))) };
                let mut d = d.clone();
                if o.transparent && o.format == 0 {
                    d.background = None;
                }
                let svg = d.export_svg(r[0], r[1], r[2], r[3], if o.format == 2 { 1.0 } else { o.scale });
                match o.format {
                    2 => std::fs::write(&path, if o.minify { svg.replace('\n', "") } else { svg }).map_err(|e| e.to_string())?,
                    0 => self.rasterize(&svg, r[2], r[3], o.scale)?.save_png(&path).map_err(|e| e.to_string())?,
                    _ => {
                        let pix = self.rasterize(&svg, r[2], r[3], o.scale)?;
                        let mut rgb = image::RgbImage::new(pix.width(), pix.height());
                        for (px, out) in pix.pixels().iter().zip(rgb.pixels_mut()) {
                            let c = px.demultiply();
                            let a = c.alpha() as f32 / 255.0;
                            let mix = |v: u8| (v as f32 * a + 255.0 * (1.0 - a)).round() as u8;
                            *out = image::Rgb([mix(c.red()), mix(c.green()), mix(c.blue())]);
                        }
                        let f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
                        image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(f), o.quality).encode_image(&rgb).map_err(|e| e.to_string())?;
                    }
                }
            }
            Ok(())
        })();
        match result {
            Ok(()) => self.info = Some(format!("Export {} terminé", ext.to_uppercase())),
            Err(e) => self.error = Some(format!("Export : {e}")),
        }
    }

    fn quick_png(&mut self) {
        self.run_export(&ExportOpts { format: 0, scale: 2.0, transparent: false, quality: 90, scope: 0, minify: false });
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.file, as_new) {
            (Some(p), false) => p.clone(),
            _ => match rfd::FileDialog::new().add_filter("Projet Vector", &["evec"]).set_file_name("dessin.evec").save_file() {
                Some(p) => p,
                None => return,
            },
        };
        match serde_json::to_vec_pretty(&self.doc).map_err(|e| e.to_string()).and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string())) {
            Ok(()) => {
                self.saved = self.doc.clone();
                self.remember(&path);
                self.file = Some(path);
            }
            Err(e) => self.error = Some(format!("Enregistrement : {e}")),
        }
    }

    fn open_path(&mut self, path: &Path) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext == "svg" {
            if self.home {
                self.set_doc(Doc::new(1000.0, 1000.0), None);
            }
            self.import_svg(path);
            return;
        }
        if ["png", "jpg", "jpeg"].contains(&ext.as_str()) {
            if self.home {
                self.set_doc(Doc::new(1000.0, 1000.0), None);
            }
            self.place_image_path(path);
            return;
        }
        match std::fs::read(path).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Doc>(b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&b)).map_err(|e| e.to_string())) {
            Ok(doc) => {
                self.set_doc(doc, Some(path.to_path_buf()));
                self.remember(path);
            }
            Err(e) => self.error = Some(format!("Ouverture : {e}")),
        }
    }

    fn open(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("Fichiers Vector", &["evec", "svg"]).add_filter("Projet Vector", &["evec"]).add_filter("SVG", &["svg"]).pick_file() {
            self.open_path(&path);
        }
    }

    fn import_svg(&mut self, path: &Path) {
        let data = match std::fs::read_to_string(path) {
            Ok(d) => d,
            Err(e) => return self.error = Some(format!("SVG : {e}")),
        };
        match geom::import_svg(&data, &self.opts()) {
            Ok(items) if !items.is_empty() => {
                self.checkpoint();
                let g = if items.len() > 1 { self.next_group() } else { 0 };
                let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or("SVG".into());
                let mut v = Vec::new();
                for (k, mut it) in items.into_iter().enumerate() {
                    if it.name.is_empty() {
                        it.name = format!("{stem} {}", k + 1);
                    }
                    it.group = g;
                    it.layer = self.active_layer;
                    v.push(it);
                }
                self.insert_copies(v, [0.0, 0.0]);
                self.tool = Tool::Select;
            }
            Ok(_) => self.error = Some("SVG vide ou sans tracé".into()),
            Err(e) => self.error = Some(format!("SVG illisible : {e}")),
        }
    }

    fn place_image(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg"]).pick_file() {
            self.place_image_path(&path);
        }
    }

    fn place_image_path(&mut self, path: &Path) {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => return self.error = Some(format!("Image : {e}")),
        };
        let dims = image::ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format().ok().and_then(|r| r.into_dimensions().ok());
        let Some((iw, ih)) = dims else {
            self.error = Some("Image illisible (PNG ou JPEG attendu)".into());
            return;
        };
        let mime = if bytes.starts_with(&[0x89, b'P']) { "image/png" } else { "image/jpeg" };
        let b = self.doc.all_boards()[self.active_board.min(self.doc.boards.len())].clone();
        let s = (b.w * 0.6 / iw as f32).min(b.h * 0.6 / ih as f32).min(1.0);
        let (w, h) = (iw as f32 * s, ih as f32 * s);
        self.checkpoint();
        let it = Item {
            name: path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or("Image".into()),
            geom: Geom::Image { x: b.x + (b.w - w) / 2.0, y: b.y + (b.h - h) / 2.0, w, h, href: format!("data:{mime};base64,{}", model::base64(&bytes)) },
            layer: self.active_layer,
            ..Item::blank()
        };
        self.add_item(it);
        self.tool = Tool::Select;
    }

    fn fit_view(&mut self, all: bool) {
        let boards = self.doc.all_boards();
        let rect = if all {
            boards.iter().map(|b| BBox { min: [b.x, b.y], max: [b.x + b.w, b.y + b.h] }).reduce(|a, b| a.union(&b)).unwrap()
        } else {
            let b = &boards[self.active_board.min(boards.len() - 1)];
            BBox { min: [b.x, b.y], max: [b.x + b.w, b.y + b.h] }
        };
        self.focus(rect, 80.0, 16.0);
    }

    fn focus(&mut self, r: BBox, margin: f32, max: f32) {
        let a = self.area;
        let s = r.size();
        self.zoom = ((a.width() - margin) / s[0].max(1.0)).min((a.height() - margin) / s[1].max(1.0)).clamp(0.02, max);
        let c = r.center();
        let pc = [self.doc.width / 2.0, self.doc.height / 2.0];
        let d = [(c[0] - pc[0]) * self.zoom, (c[1] - pc[1]) * self.zoom];
        let (sn, co) = self.rot.to_radians().sin_cos();
        self.offset = -vec2(co * d[0] - sn * d[1], sn * d[0] + co * d[1]);
    }

    fn zoom_by(&mut self, f: f32) {
        let cam = self.cam();
        let at = self.area.center();
        let docp = cam.to_doc(at);
        self.zoom = (self.zoom * f).clamp(0.02, 64.0);
        let cam = self.cam();
        self.offset += at - cam.to_scr(docp);
    }

    fn zoom_to_selection(&mut self) {
        if let Some(b) = self.doc.bounds_of(&self.selection()) {
            self.focus(b, 200.0, 32.0);
        }
    }

    fn open_dlg(&mut self, kind: Dlg) {
        let sel = self.selection();
        let needs = !matches!(kind, Dlg::Adjust { .. });
        if needs && sel.is_empty() {
            self.info = Some("Sélectionne d'abord un objet".into());
            return;
        }
        self.checkpoint();
        self.dlg = Some(DlgState { kind, copy: false, preview: true, base: self.doc.clone(), sel, applied: None });
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() || self.modal() {
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        if sc(M::COMMAND, Key::N) {
            self.new_doc = Some(NewDoc { w: 1000.0, h: 1000.0, preset: 0, transparent: false });
        }
        if sc(M::COMMAND, Key::O) {
            self.open();
        }
        if self.home {
            return;
        }
        let ca = M::COMMAND | M::ALT;
        let cs = M::COMMAND | M::SHIFT;
        if sc(cs, Key::Z) || sc(M::COMMAND, Key::Y) {
            self.redo();
        } else if sc(M::COMMAND, Key::Z) {
            self.undo();
        }
        if sc(ca | M::SHIFT, Key::D) {
            self.open_dlg(Dlg::Each { sx: 100.0, sy: 100.0, dx: 0.0, dy: 0.0, deg: 0.0 });
        } else if sc(ca, Key::D) {
            self.duplicate_selected();
        } else if sc(M::COMMAND, Key::D) {
            self.transform_again();
        }
        if sc(cs, Key::M) {
            self.open_dlg(Dlg::Move { dx: 10.0, dy: 0.0 });
        }
        if sc(ca, Key::J) {
            self.path_op(6);
        } else if sc(M::COMMAND, Key::J) {
            self.join();
        }
        if sc(M::COMMAND, Key::C) {
            self.copy_items();
        }
        if sc(M::COMMAND, Key::X) {
            self.copy_items();
            self.delete_selected();
        }
        if sc(cs, Key::V) {
            self.paste_items(1);
        } else if sc(M::COMMAND, Key::V) {
            self.paste_items(0);
        }
        if sc(M::COMMAND, Key::F) {
            self.paste_items(2);
        }
        if sc(M::COMMAND, Key::B) {
            self.paste_items(3);
        }
        if sc(cs, Key::A) {
            self.set_selection(Vec::new());
        } else if sc(M::COMMAND, Key::A) {
            self.select_all();
        }
        if sc(cs, Key::G) {
            self.ungroup();
        } else if sc(M::COMMAND, Key::G) {
            self.group();
        }
        if sc(ca, Key::Num7) {
            self.release_clip();
        } else if sc(M::COMMAND, Key::Num7) {
            self.make_clip(false);
        }
        if sc(ca, Key::Num8) {
            self.release_compound();
        } else if sc(M::COMMAND, Key::Num8) {
            self.make_compound();
        }
        if sc(ca, Key::Num2) {
            self.checkpoint();
            self.doc.items.iter_mut().for_each(|it| it.locked = false);
        } else if sc(M::COMMAND, Key::Num2) {
            self.lock_selected();
        }
        if sc(ca, Key::Num3) {
            self.checkpoint();
            self.doc.items.iter_mut().for_each(|it| it.visible = true);
        } else if sc(M::COMMAND, Key::Num3) {
            self.hide_selected();
        }
        if sc(ca, Key::B) {
            self.open_dlg(Dlg::Blend { steps: 5 });
        }
        if sc(cs, Key::O) {
            self.text_outlines();
        }
        if sc(cs, Key::Y) {
            self.outline_view = !self.outline_view;
        }
        if sc(cs, Key::S) {
            self.save(true);
        } else if sc(M::COMMAND, Key::S) {
            self.save(false);
        }
        if sc(ca, Key::E) || sc(cs, Key::E) {
            self.export = Some(ExportOpts { format: 0, scale: 2.0, transparent: false, quality: 90, scope: 0, minify: false });
        } else if sc(M::COMMAND, Key::E) {
            self.quick_png();
        }
        if sc(ca, Key::Num0) {
            self.fit_view(true);
        } else if sc(M::COMMAND, Key::Num0) {
            self.fit_view(false);
        }
        if sc(M::COMMAND, Key::Num1) {
            self.zoom_by(1.0 / self.zoom);
        }
        if sc(M::COMMAND, Key::Plus) || sc(M::COMMAND, Key::Equals) {
            self.zoom_by(1.25);
        }
        if sc(M::COMMAND, Key::Minus) {
            self.zoom_by(0.8);
        }
        if sc(cs, Key::CloseBracket) {
            self.reorder(i32::MAX);
        } else if sc(M::COMMAND, Key::CloseBracket) {
            self.reorder(1);
        }
        if sc(cs, Key::OpenBracket) {
            self.reorder(i32::MIN);
        } else if sc(M::COMMAND, Key::OpenBracket) {
            self.reorder(-1);
        }
        if sc(ca, Key::Quote) {
            self.snap = !self.snap;
        } else if sc(M::COMMAND, Key::Quote) {
            self.show_grid = !self.show_grid;
        }
        if sc(ca, Key::Semicolon) {
            self.lock_guides = !self.lock_guides;
        } else if sc(M::COMMAND, Key::Semicolon) {
            self.show_guides = !self.show_guides;
        }
        if sc(M::COMMAND, Key::R) {
            self.rulers = !self.rulers;
        }
        if sc(M::COMMAND, Key::U) {
            self.smart = !self.smart;
        }
        if sc(M::COMMAND, Key::H) {
            self.show_edges = !self.show_edges;
        }
        if sc(cs, Key::B) {
            self.show_bbox = !self.show_bbox;
        }
        if sc(M::SHIFT, Key::X) {
            self.swap_fill_stroke();
        }
        let (pick, del, enter, nudge, plain) = ctx.input(|i| {
            let plain = !i.modifiers.command && !i.modifiers.alt;
            let mut pick = None;
            if plain {
                for (k, shift, t) in HOTKEYS {
                    if i.key_pressed(k) && i.modifiers.shift == shift {
                        pick = Some(t);
                    }
                }
                if pick.is_none() && i.modifiers.shift {
                    for (k, _, t) in HOTKEYS {
                        if i.key_pressed(k) {
                            pick = Some(t);
                        }
                    }
                }
            }
            let step = if i.modifiers.shift { 10.0 } else { 1.0 };
            let mut n = [0.0, 0.0];
            if i.key_pressed(Key::ArrowLeft) {
                n[0] -= step;
            }
            if i.key_pressed(Key::ArrowRight) {
                n[0] += step;
            }
            if i.key_pressed(Key::ArrowUp) {
                n[1] -= step;
            }
            if i.key_pressed(Key::ArrowDown) {
                n[1] += step;
            }
            (pick, i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace), i.key_pressed(Key::Enter) || i.key_pressed(Key::Escape), n, plain && !i.modifiers.shift)
        });
        if let Some(t) = pick {
            let g = group_of(t);
            let shift_cycle = !HOTKEYS.iter().any(|h| h.2 == t && h.1) && ctx.input(|i| i.modifiers.shift);
            if group_of(self.tool) == g && (self.tool == t || shift_cycle || GROUPS[g][0].0 == t) && GROUPS[g].len() > 1 {
                let k = GROUPS[g].iter().position(|x| x.0 == self.tool).unwrap_or(0);
                let next = GROUPS[g][(k + 1) % GROUPS[g].len()].0;
                self.set_tool(next);
            } else {
                self.set_tool(t);
            }
        }
        if plain {
            ctx.input(|i| {
                if i.key_pressed(Key::D) {
                    self.style = Style { fill: Some(Color32::WHITE), stroke: Some(Color32::BLACK), stroke_w: 1.0 };
                }
                if i.key_pressed(Key::Slash) {
                    self.style.fill = None;
                }
            });
        }
        if enter {
            self.finish_pen();
            self.o.measure = None;
        }
        if del {
            if self.tool == Tool::Board {
                let k = self.active_board;
                self.delete_board(k);
            } else {
                self.delete_selected();
            }
        }
        if nudge != [0.0, 0.0] && self.sel.is_some() {
            self.checkpoint();
            let d = [nudge[0] * self.o.nudge, nudge[1] * self.o.nudge];
            if let (Some(i), Some(n), 1) = (self.sel, self.sel_node, self.selection().len())
                && let Geom::Path { nodes, .. } = &mut self.doc.items[i].geom
                && let Some(nd) = nodes.get_mut(n)
            {
                for q in [&mut nd.p, &mut nd.cin, &mut nd.cout] {
                    q[0] += d[0];
                    q[1] += d[1];
                }
                return;
            }
            for i in self.selection() {
                self.doc.items[i].translate(d);
            }
        }
    }

    fn transform_again(&mut self) {
        match self.last_tf.clone() {
            Some((kind, copy)) if self.sel.is_some() => {
                self.checkpoint();
                self.apply_dlg(&kind, copy);
            }
            _ => self.duplicate_selected(),
        }
    }
}

fn apply_affine(it: &mut Item, m: [f32; 4], c: P, strokes: bool) -> bool {
    let t = [m[0], m[1], m[2], m[3], c[0] - m[0] * c[0] - m[2] * c[1], c[1] - m[1] * c[0] - m[3] * c[1]];
    let ok = it.affine(t);
    if ok && strokes {
        let k = (m[0] * m[3] - m[1] * m[2]).abs().sqrt();
        it.stroke_w *= k;
        it.blur *= k;
    }
    ok
}

impl eframe::App for VectorApp {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let before = self.doc.clone();
        if !self.home && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F1)) {
            if self.help.open {
                self.help.open = false;
            } else {
                self.help.learn(tool_name(self.tool));
            }
        }
        self.shortcuts(ctx);
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty())) {
            self.open_path(&p);
        }
        let name = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = if self.home { "Vector".to_string() } else { format!("{name}{} — Vector", if self.doc != self.saved { " •" } else { "" }) };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.menu_bar(root);
        if self.home {
            self.home_screen(root);
        } else {
            self.options_bar(root);
            self.status_bar(root);
            self.tool_rail(root);
            self.dock(root);
            self.canvas(root, &name);
        }
        self.new_doc_sheet(ctx);
        self.dlg_sheet(ctx);
        self.export_sheet(ctx);
        self.trace_sheet(ctx);
        self.recolor_sheet(ctx);
        self.help.show(ctx);
        if !self.home && !self.modal() && self.doc != before && !self.editing && matches!(self.drag, Drag::None) {
            self.undo.push(before);
            self.redo.clear();
            self.editing = true;
        }
        if ctx.input(|i| !i.pointer.any_down()) && !ctx.egui_wants_keyboard_input() {
            self.editing = false;
        }
    }
}

impl VectorApp {
    fn menu_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(40.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(10, 0))).show(root, |ui| {
            let full = ui.max_rect();
            let right_w = if self.home { 0.0 } else { 300.0 };
            if !self.home {
                let right = Rect::from_min_max(pos2(full.right() - right_w, full.top()), full.max);
                ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let r = pro::primary_icon(ui, "Exporter", |p, r, c| panels::export_glyph(p, r, c)).on_hover_text("Options d'export : Ctrl+Alt+E · PNG ×2 rapide : Ctrl+E");
                        if r.clicked() {
                            self.export = Some(ExportOpts { format: 0, scale: 2.0, transparent: false, quality: 90, scope: 0, minify: false });
                        }
                        ui.add_space(6.0);
                        let boards = self.doc.all_boards();
                        let b = &boards[self.active_board.min(boards.len() - 1)];
                        if pro::chip(ui, &format!("{} · {:.0} × {:.0}", b.name, b.w, b.h)).on_hover_text("Plan de travail actif : clic pour l'ajuster à l'écran").clicked() {
                            self.fit_view(false);
                        }
                    });
                });
            }
            let left = Rect::from_min_max(full.min, pos2(full.right() - right_w, full.bottom()));
            ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(20.0))));
                    ui.add_space(6.0);
                    egui::MenuBar::new().ui(ui, |ui| {
                        ui.menu_button("Fichier", |ui| self.menu_file(ui));
                        ui.add_enabled_ui(!self.home, |ui| {
                            ui.menu_button("Édition", |ui| self.menu_edit(ui));
                            ui.menu_button("Objet", |ui| self.menu_object(ui));
                            ui.menu_button("Texte", |ui| self.menu_text(ui));
                            ui.menu_button("Sélection", |ui| self.menu_select(ui));
                            ui.menu_button("Effet", |ui| self.menu_effect(ui));
                            ui.menu_button("Affichage", |ui| self.menu_view(ui));
                            ui.menu_button("Fenêtre", |ui| self.menu_window(ui));
                        });
                        self.help.menu(ui);
                    });
                });
            });
        });
    }

    fn menu_file(&mut self, ui: &mut Ui) {
        if mi(ui, "Nouveau…", "Ctrl+N", true) {
            self.new_doc = Some(NewDoc { w: 1000.0, h: 1000.0, preset: 0, transparent: false });
        }
        if mi(ui, "Ouvrir…", "Ctrl+O", true) {
            self.open();
        }
        ui.menu_button("Ouvrir un fichier récent", |ui| {
            let mut open = None;
            for p in &self.recents {
                if ui.button(p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).clicked() {
                    open = Some(p.clone());
                }
            }
            if self.recents.is_empty() {
                ui.label("Aucun fichier récent");
            }
            if let Some(p) = open {
                self.open_path(&p);
            }
        });
        let on = !self.home;
        ui.separator();
        if mi(ui, "Enregistrer", "Ctrl+S", on) {
            self.save(false);
        }
        if mi(ui, "Enregistrer sous…", "Ctrl+Maj+S", on) {
            self.save(true);
        }
        ui.separator();
        if mi(ui, "Importer une image…", "", on) {
            self.place_image();
        }
        if mi(ui, "Importer un SVG…", "", on)
            && let Some(p) = rfd::FileDialog::new().add_filter("SVG", &["svg"]).pick_file()
        {
            self.import_svg(&p);
        }
        ui.separator();
        if mi(ui, "Exporter…", "Ctrl+Alt+E", on) {
            self.export = Some(ExportOpts { format: 0, scale: 2.0, transparent: false, quality: 90, scope: 0, minify: false });
        }
        ui.add_enabled_ui(on, |ui| {
            ui.menu_button("Exporter rapidement", |ui| {
                for (label, f, s, t) in [("PNG ×1", 0, 1.0, false), ("PNG ×2", 0, 2.0, false), ("PNG ×3", 0, 3.0, false), ("PNG ×4", 0, 4.0, false), ("PNG ×2 fond transparent", 0, 2.0, true), ("JPEG ×2", 1, 2.0, false), ("SVG", 2, 1.0, false), ("PDF (tous les plans de travail)", 3, 1.0, false)] {
                    if ui.button(label).clicked() {
                        self.run_export(&ExportOpts { format: f, scale: s, transparent: t, quality: 90, scope: if f == 3 { 1 } else { 0 }, minify: false });
                        ui.close();
                    }
                }
                if ui.add_enabled(self.sel.is_some(), egui::Button::new("SVG de la sélection")).clicked() {
                    self.run_export(&ExportOpts { format: 2, scale: 1.0, transparent: true, quality: 90, scope: 2, minify: false });
                    ui.close();
                }
                if ui.add_enabled(self.sel.is_some(), egui::Button::new("PNG ×2 de la sélection")).clicked() {
                    self.run_export(&ExportOpts { format: 0, scale: 2.0, transparent: true, quality: 90, scope: 2, minify: false });
                    ui.close();
                }
            });
        });
        ui.separator();
        if mi(ui, "Fermer", "", on) {
            self.home = true;
        }
    }

    fn menu_edit(&mut self, ui: &mut Ui) {
        let has = self.sel.is_some();
        if mi(ui, "Annuler", "Ctrl+Z", !self.undo.is_empty()) {
            self.undo();
        }
        if mi(ui, "Rétablir", "Ctrl+Y", !self.redo.is_empty()) {
            self.redo();
        }
        ui.separator();
        if mi(ui, "Couper", "Ctrl+X", has) {
            self.copy_items();
            self.delete_selected();
        }
        if mi(ui, "Copier", "Ctrl+C", has) {
            self.copy_items();
        }
        let clip = !self.clip.is_empty();
        if mi(ui, "Coller", "Ctrl+V", clip) {
            self.paste_items(0);
        }
        if mi(ui, "Coller devant", "Ctrl+F", clip) {
            self.paste_items(2);
        }
        if mi(ui, "Coller derrière", "Ctrl+B", clip) {
            self.paste_items(3);
        }
        if mi(ui, "Coller en place", "Ctrl+Maj+V", clip) {
            self.paste_items(1);
        }
        if mi(ui, "Supprimer", "Suppr", has) {
            self.delete_selected();
        }
        ui.separator();
        if mi(ui, "Dupliquer", "Ctrl+Alt+D", has) {
            self.duplicate_selected();
        }
        ui.separator();
        ui.menu_button("Modifier les couleurs", |ui| {
            if ui.button("Recolorer l'illustration…").clicked() {
                self.open_recolor();
                ui.close();
            }
            if ui.button("Régler la teinte, saturation, luminosité…").clicked() {
                self.open_dlg(Dlg::Adjust { hue: 0.0, sat: 0.0, light: 0.0 });
                ui.close();
            }
            ui.separator();
            if ui.button("Convertir en niveaux de gris").clicked() {
                self.recolor_all(&|c| {
                    let l = (0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32) as u8;
                    [l, l, l, c[3]]
                });
            }
            if ui.button("Inverser les couleurs").clicked() {
                self.recolor_all(&|c| [255 - c[0], 255 - c[1], 255 - c[2], c[3]]);
            }
            if ui.button("Sépia").clicked() {
                self.recolor_all(&|c| {
                    let (r, g, b) = (c[0] as f32, c[1] as f32, c[2] as f32);
                    [(0.393 * r + 0.769 * g + 0.189 * b).min(255.0) as u8, (0.349 * r + 0.686 * g + 0.168 * b).min(255.0) as u8, (0.272 * r + 0.534 * g + 0.131 * b).min(255.0) as u8, c[3]]
                });
            }
            if ui.button("Saturer +20 %").clicked() {
                self.recolor_all(&|c| hsl_adjust(c, 0.0, 20.0, 0.0));
            }
            if ui.button("Désaturer −20 %").clicked() {
                self.recolor_all(&|c| hsl_adjust(c, 0.0, -20.0, 0.0));
            }
            if ui.button("Éclaircir +10 %").clicked() {
                self.recolor_all(&|c| hsl_adjust(c, 0.0, 0.0, 10.0));
            }
            if ui.button("Assombrir −10 %").clicked() {
                self.recolor_all(&|c| hsl_adjust(c, 0.0, 0.0, -10.0));
            }
            if ui.button("Teinte +30°").clicked() {
                self.recolor_all(&|c| hsl_adjust(c, 30.0, 0.0, 0.0));
            }
            if ui.button("Couleurs complémentaires").clicked() {
                self.recolor_all(&|c| hsl_adjust(c, 180.0, 0.0, 0.0));
            }
            ui.separator();
            if ui.button("Mélange horizontal").clicked() {
                self.blend_colors(Some(0));
            }
            if ui.button("Mélange vertical").clicked() {
                self.blend_colors(Some(1));
            }
            if ui.button("Mélange avant-arrière").clicked() {
                self.blend_colors(None);
            }
            if ui.button("Échanger fond et contour").clicked() {
                self.swap_fill_stroke();
            }
        });
    }

    fn menu_object(&mut self, ui: &mut Ui) {
        let has = self.sel.is_some();
        let multi = self.selection().len() >= 2;
        ui.menu_button("Transformation", |ui| {
            if mi(ui, "Transformer à nouveau", "Ctrl+D", has) {
                self.transform_again();
            }
            ui.separator();
            for (label, sc, d) in [
                ("Déplacement…", "Ctrl+Maj+M", Dlg::Move { dx: 10.0, dy: 0.0 }),
                ("Rotation…", "", Dlg::Rotate { deg: 45.0 }),
                ("Miroir…", "", Dlg::Reflect { axis: 1, angle: 0.0 }),
                ("Mise à l'échelle…", "", Dlg::Scale { sx: 100.0, sy: 100.0, uniform: true }),
                ("Inclinaison…", "", Dlg::Shear { deg: 15.0, vertical: false }),
                ("Transformer chaque…", "Ctrl+Alt+Maj+D", Dlg::Each { sx: 100.0, sy: 100.0, dx: 0.0, dy: 0.0, deg: 0.0 }),
                ("Répétition…", "", Dlg::Repeat { n: 5, dx: 40.0, dy: 0.0, rot: 0.0 }),
            ] {
                if mi(ui, label, sc, has) {
                    self.open_dlg(d);
                }
            }
            ui.separator();
            for (label, deg) in [("Rotation 90° horaire", 90.0), ("Rotation 90° antihoraire", -90.0), ("Rotation 180°", 180.0)] {
                if mi(ui, label, "", has) {
                    self.rotate_sel(deg);
                }
            }
            if mi(ui, "Symétrie horizontale", "", has) {
                self.flip(0);
            }
            if mi(ui, "Symétrie verticale", "", has) {
                self.flip(1);
            }
        });
        ui.menu_button("Disposition", |ui| {
            for (label, sc, by) in [("Premier plan", "Ctrl+Maj+]", i32::MAX), ("En avant", "Ctrl+]", 1), ("En arrière", "Ctrl+[", -1), ("Arrière-plan", "Ctrl+Maj+[", i32::MIN)] {
                if mi(ui, label, sc, has) {
                    self.reorder(by);
                }
            }
            ui.menu_button("Envoyer vers le calque", |ui| {
                let names: Vec<String> = self.doc.layers.iter().map(|l| l.name.clone()).collect();
                for (k, n) in names.iter().enumerate() {
                    if ui.button(n).clicked() {
                        self.sel_to_layer(k);
                    }
                }
            });
        });
        ui.separator();
        if mi(ui, "Grouper", "Ctrl+G", multi) {
            self.group();
        }
        if mi(ui, "Dissocier", "Ctrl+Maj+G", has) {
            self.ungroup();
        }
        if mi(ui, "Verrouiller la sélection", "Ctrl+2", has) {
            self.lock_selected();
        }
        if mi(ui, "Tout déverrouiller", "Ctrl+Alt+2", true) {
            self.checkpoint();
            self.doc.items.iter_mut().for_each(|it| it.locked = false);
        }
        if mi(ui, "Masquer la sélection", "Ctrl+3", has) {
            self.hide_selected();
        }
        if mi(ui, "Tout afficher", "Ctrl+Alt+3", true) {
            self.checkpoint();
            self.doc.items.iter_mut().for_each(|it| it.visible = true);
        }
        ui.separator();
        if mi(ui, "Décomposer", "", has) {
            self.expand();
        }
        if mi(ui, "Vectoriser le contour", "", has) {
            self.outline_selected();
        }
        ui.menu_button("Tracé", |ui| {
            if mi(ui, "Joindre", "Ctrl+J", has) {
                self.join();
            }
            if mi(ui, "Convertir les points en sommets", "Ctrl+Alt+J", has) {
                self.path_op(6);
            }
            if mi(ui, "Décalage du tracé…", "", has) {
                self.open_dlg(Dlg::Offset { d: 10.0 });
            }
            if mi(ui, "Simplifier…", "", has) {
                self.open_dlg(Dlg::Simplify { tol: 2.0, smooth: true });
            }
            if mi(ui, "Convertir en tracé", "", has) {
                self.path_op(0);
            }
            for (label, op) in [("Lisser le tracé", 1), ("Simplifier (rapide)", 2), ("Inverser le sens", 3), ("Fermer / ouvrir le tracé", 4), ("Ajouter des points d'ancrage", 5)] {
                if mi(ui, label, "", has) {
                    self.path_op(op);
                }
            }
            if mi(ui, "Nettoyer les objets vides", "", true) {
                self.cleanup();
            }
        });
        ui.menu_button("Pathfinder", |ui| {
            for (k, name) in geom::PATHFINDER.iter().enumerate() {
                if k == 4 {
                    ui.separator();
                }
                if mi(ui, name, "", multi) {
                    self.pathfinder(k);
                }
            }
        });
        ui.menu_button("Fusion", |ui| {
            if mi(ui, "Créer une fusion…", "Ctrl+Alt+B", multi) {
                self.open_dlg(Dlg::Blend { steps: 5 });
            }
        });
        ui.menu_button("Masque d'écrêtage", |ui| {
            if mi(ui, "Créer", "Ctrl+7", multi) {
                self.make_clip(false);
            }
            if mi(ui, "Annuler", "Ctrl+Alt+7", has) {
                self.release_clip();
            }
        });
        ui.menu_button("Masque d'opacité", |ui| {
            if mi(ui, "Créer", "", multi) {
                self.make_clip(true);
            }
            if mi(ui, "Annuler", "", has) {
                self.release_clip();
            }
        });
        ui.menu_button("Tracé transparent composé", |ui| {
            if mi(ui, "Créer", "Ctrl+8", multi) {
                self.make_compound();
            }
            if mi(ui, "Annuler", "Ctrl+Alt+8", has) {
                self.release_compound();
            }
        });
        let is_img = self.sel.is_some_and(|i| matches!(self.doc.items[i].geom, Geom::Image { .. }));
        if mi(ui, "Vectorisation dynamique…", "", is_img) {
            self.open_trace();
        }
        ui.menu_button("Plans de travail", |ui| {
            if ui.button("Nouveau plan de travail").clicked() {
                self.add_board(self.doc.width, self.doc.height);
            }
            if ui.button("Ajuster à l'illustration").clicked() {
                let all: Vec<usize> = (0..self.doc.items.len()).filter(|i| self.doc.shown(*i)).collect();
                self.fit_board_to(self.doc.bounds_of(&all));
            }
            if ui.add_enabled(has, egui::Button::new("Ajuster à la sélection")).clicked() {
                self.fit_board_to(self.doc.bounds_of(&self.selection()));
            }
            if ui.button("Réorganiser en grille").clicked() {
                self.arrange_boards();
            }
            if ui.add_enabled(self.active_board > 0, egui::Button::new("Supprimer le plan actif")).clicked() {
                let k = self.active_board;
                self.delete_board(k);
            }
        });
    }

    fn menu_text(&mut self, ui: &mut Ui) {
        let texts: Vec<usize> = self.selection().into_iter().filter(|i| matches!(self.doc.items[*i].geom, Geom::Text { .. })).collect();
        let on = !texts.is_empty();
        ui.menu_button("Police", |ui| {
            for f in FONTS {
                if ui.add_enabled(on, egui::Button::new(f)).clicked() {
                    self.checkpoint();
                    for &i in &texts {
                        if let Geom::Text { family, .. } = &mut self.doc.items[i].geom {
                            *family = f.into();
                        }
                    }
                }
            }
        });
        ui.menu_button("Taille", |ui| {
            for s in [8.0, 12.0, 18.0, 24.0, 36.0, 48.0, 64.0, 72.0, 96.0, 144.0, 200.0] {
                if ui.add_enabled(on, egui::Button::new(format!("{s} px"))).clicked() {
                    self.checkpoint();
                    for &i in &texts {
                        if let Geom::Text { size, .. } = &mut self.doc.items[i].geom {
                            *size = s;
                        }
                    }
                }
            }
        });
        ui.menu_button("Casse", |ui| {
            for (k, label) in ["Normale", "MAJUSCULES", "minuscules", "Initiales En Capitales"].iter().enumerate() {
                if ui.add_enabled(on, egui::Button::new(*label)).clicked() {
                    self.checkpoint();
                    for &i in &texts {
                        self.doc.items[i].case = k as u8;
                    }
                }
            }
        });
        ui.menu_button("Insérer un caractère spécial", |ui| {
            for (c, n) in [("©", "Copyright"), ("®", "Marque déposée"), ("™", "Marque"), ("«  »", "Guillemets"), ("—", "Tiret cadratin"), ("–", "Tiret demi-cadratin"), ("…", "Points de suspension"), ("€", "Euro"), ("°", "Degré"), ("•", "Puce"), ("→", "Flèche"), ("★", "Étoile")] {
                if ui.add_enabled(texts.len() == 1, egui::Button::new(format!("{c}   {n}"))).clicked() {
                    self.checkpoint();
                    if let Geom::Text { text, .. } = &mut self.doc.items[texts[0]].geom {
                        text.push_str(c);
                    }
                }
            }
        });
        ui.separator();
        if mi(ui, "Vectoriser", "Ctrl+Maj+O", on) {
            self.text_outlines();
        }
        if mi(ui, "Convertir en texte captif", "", on) {
            self.checkpoint();
            for &i in &texts {
                let w = self.doc.items[i].bbox().size()[0].max(100.0);
                let it = &mut self.doc.items[i];
                it.box_w = w;
                it.vertical = false;
                it.text_path.clear();
            }
        }
        if mi(ui, "Convertir en texte de point", "", on) {
            self.checkpoint();
            for &i in &texts {
                self.doc.items[i].box_w = 0.0;
                self.doc.items[i].box_h = 0.0;
            }
        }
        if mi(ui, "Orientation horizontale / verticale", "", on) {
            self.checkpoint();
            for &i in &texts {
                self.doc.items[i].vertical = !self.doc.items[i].vertical;
            }
        }
    }

    fn menu_select(&mut self, ui: &mut Ui) {
        let has = self.sel.is_some();
        if mi(ui, "Tout sélectionner", "Ctrl+A", true) {
            self.select_all();
        }
        if mi(ui, "Désélectionner", "Ctrl+Maj+A", has) {
            self.set_selection(Vec::new());
        }
        if mi(ui, "Sélection inverse", "", true) {
            self.select_inverse();
        }
        ui.separator();
        ui.menu_button("Même", |ui| {
            for (k, label) in ["Couleur de fond", "Couleur de contour", "Épaisseur de contour", "Opacité", "Mode de fusion", "Type d'objet", "Aspect", "Occurrence de symbole"].iter().enumerate() {
                if ui.add_enabled(has, egui::Button::new(*label)).clicked() {
                    self.select_same(k as u8);
                }
            }
        });
        if mi(ui, "Objets du calque actif", "", true) {
            let v = (0..self.doc.items.len()).filter(|i| self.doc.items[*i].layer == self.active_layer && self.doc.editable(*i)).collect();
            self.set_selection(v);
        }
        if mi(ui, "Objets du plan de travail actif", "", true) {
            let b = &self.doc.all_boards()[self.active_board.min(self.doc.boards.len())];
            let r = BBox { min: [b.x, b.y], max: [b.x + b.w, b.y + b.h] };
            let v = (0..self.doc.items.len()).filter(|i| self.doc.editable(*i) && r.overlaps(&self.doc.items[*i].bbox())).collect();
            self.set_selection(v);
        }
        if mi(ui, "Tous les textes", "", true) {
            let v = (0..self.doc.items.len()).filter(|i| self.doc.editable(*i) && matches!(self.doc.items[*i].geom, Geom::Text { .. })).collect();
            self.set_selection(v);
        }
        if mi(ui, "Tracés ouverts", "", true) {
            let v = (0..self.doc.items.len()).filter(|i| self.doc.editable(*i) && matches!(self.doc.items[*i].geom, Geom::Path { closed: false, .. })).collect();
            self.set_selection(v);
        }
        if mi(ui, "Images", "", true) {
            let v = (0..self.doc.items.len()).filter(|i| self.doc.editable(*i) && matches!(self.doc.items[*i].geom, Geom::Image { .. })).collect();
            self.set_selection(v);
        }
        if mi(ui, "Masques d'écrêtage", "", true) {
            let v = (0..self.doc.items.len()).filter(|i| self.doc.items[*i].clip || self.doc.items[*i].mask).collect();
            self.set_selection(v);
        }
    }

    fn menu_effect(&mut self, ui: &mut Ui) {
        let has = self.sel.is_some();
        ui.label(egui::RichText::new("Effets dynamiques (modifiables dans Propriétés)").size(11.5).color(pro::FAINT));
        let mut live = |ui: &mut Ui, label: &str, f: &dyn Fn(&mut Item)| {
            if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                self.checkpoint();
                for i in self.selection() {
                    f(&mut self.doc.items[i]);
                }
                self.dock_tab = 0;
            }
        };
        live(ui, "Ombre portée", &|it| it.shadow = true);
        live(ui, "Lueur externe", &|it| it.glow_out = it.glow_out.max(10.0));
        live(ui, "Lueur interne", &|it| it.glow_in = it.glow_in.max(8.0));
        live(ui, "Contour progressif", &|it| it.feather = it.feather.max(6.0));
        live(ui, "Flou gaussien", &|it| it.blur = it.blur.max(4.0));
        live(ui, "Distorsion dynamique", &|it| it.rough = it.rough.max(8.0));
        live(ui, "Grain", &|it| it.grain = it.grain.max(0.3));
        live(ui, "Retirer tous les effets", &|it| {
            it.shadow = false;
            it.glow_out = 0.0;
            it.glow_in = 0.0;
            it.feather = 0.0;
            it.blur = 0.0;
            it.rough = 0.0;
            it.grain = 0.0;
        });
        ui.separator();
        ui.menu_button("Distorsion et transformation", |ui| {
            for (label, d) in [
                ("Zigzag…", Dlg::Zigzag { size: 8.0, ridges: 20, smooth: false }),
                ("Ondulation…", Dlg::Zigzag { size: 8.0, ridges: 12, smooth: true }),
                ("Distorsion aléatoire…", Dlg::Roughen { size: 5.0, detail: 10.0, smooth: false }),
                ("Contraction et dilatation…", Dlg::Pucker { amount: 30.0 }),
                ("Torsion…", Dlg::Twist { deg: 60.0 }),
                ("Transformer…", Dlg::Each { sx: 100.0, sy: 100.0, dx: 0.0, dy: 0.0, deg: 0.0 }),
            ] {
                if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                    self.open_dlg(d);
                    ui.close();
                }
            }
        });
        ui.menu_button("Styliser", |ui| {
            if ui.add_enabled(has, egui::Button::new("Arrondis…")).clicked() {
                self.open_dlg(Dlg::Round { r: 12.0 });
                ui.close();
            }
            if ui.add_enabled(has, egui::Button::new("Griffonnage…")).clicked() {
                self.open_dlg(Dlg::Scribble { angle: 30.0, spacing: 8.0, jitter: 2.0 });
                ui.close();
            }
        });
        ui.menu_button("Tracé", |ui| {
            if ui.add_enabled(has, egui::Button::new("Décalage du tracé…")).clicked() {
                self.open_dlg(Dlg::Offset { d: 10.0 });
                ui.close();
            }
            if ui.add_enabled(has, egui::Button::new("Vectoriser le contour")).clicked() {
                self.outline_selected();
                ui.close();
            }
        });
    }

    fn menu_view(&mut self, ui: &mut Ui) {
        if mi(ui, "Zoom avant", "Ctrl++", true) {
            self.zoom_by(1.25);
        }
        if mi(ui, "Zoom arrière", "Ctrl+−", true) {
            self.zoom_by(0.8);
        }
        if mi(ui, "Ajuster le plan de travail", "Ctrl+0", true) {
            self.fit_view(false);
        }
        if mi(ui, "Tout ajuster", "Ctrl+Alt+0", true) {
            self.fit_view(true);
        }
        if mi(ui, "Taille réelle", "Ctrl+1", true) {
            self.zoom_by(1.0 / self.zoom);
        }
        if mi(ui, "Zoom sur la sélection", "", self.sel.is_some()) {
            self.zoom_to_selection();
        }
        if mi(ui, "Réinitialiser la rotation de la vue", "", self.rot != 0.0) {
            self.rot = 0.0;
            self.fit_view(false);
        }
        ui.separator();
        ui.checkbox(&mut self.outline_view, "Mode tracés (Ctrl+Maj+Y)");
        ui.checkbox(&mut self.show_edges, "Contours de sélection (Ctrl+H)");
        ui.checkbox(&mut self.show_bbox, "Cadre de sélection (Ctrl+Maj+B)");
        ui.separator();
        ui.checkbox(&mut self.rulers, "Règles (Ctrl+R)");
        ui.checkbox(&mut self.show_guides, "Repères (Ctrl+;)");
        ui.checkbox(&mut self.lock_guides, "Verrouiller les repères (Ctrl+Alt+;)");
        if ui.add_enabled(!self.doc.guides.is_empty(), egui::Button::new("Effacer les repères")).clicked() {
            self.checkpoint();
            self.doc.guides.clear();
        }
        ui.checkbox(&mut self.smart, "Repères commentés (Ctrl+U)");
        ui.checkbox(&mut self.thirds, "Repères de composition (centre et tiers)");
        ui.separator();
        ui.checkbox(&mut self.show_grid, "Grille (Ctrl+')");
        ui.checkbox(&mut self.snap, "Magnétisme de la grille (Ctrl+Alt+')");
        ui.horizontal(|ui| {
            ui.label("Pas de la grille");
            ui.add(egui::DragValue::new(&mut self.grid).range(2.0..=500.0).suffix(" px"));
        });
    }

    fn menu_window(&mut self, ui: &mut Ui) {
        for (k, label) in ["Propriétés", "Calques et plans de travail", "Bibliothèques (nuancier, styles, symboles)"].iter().enumerate() {
            if ui.radio(self.dock_tab == k, *label).clicked() {
                self.dock_tab = k;
            }
        }
        ui.separator();
        if ui.button("Recolorer l'illustration…").clicked() {
            self.open_recolor();
        }
        if ui.button("Exporter…").clicked() {
            self.export = Some(ExportOpts { format: 0, scale: 2.0, transparent: false, quality: 90, scope: 0, minify: false });
        }
    }
}

fn mi(ui: &mut Ui, label: &str, sc: &str, enabled: bool) -> bool {
    let b = if sc.is_empty() { egui::Button::new(label) } else { egui::Button::new(label).shortcut_text(sc) };
    let r = ui.add_enabled(enabled, b).clicked();
    if r {
        ui.close();
    }
    r
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Vector")
            .with_icon(Arc::new(studio_ui::app_window_icon("vector")))
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1100.0, 680.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Vector", options, Box::new(|cc| Ok(Box::new(VectorApp::new(cc)))))
}
