#![windows_subsystem = "windows"]

mod model;
mod tutos;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame as EFrame, Key, Layout, Margin, Painter,
    Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2,
    pos2, vec2,
};
use krilla_svg::{SurfaceExt, SvgSettings};
use model::{Doc, FILTERS, Frame, Guide, Kind, LISTS, MASTER, Page, TextStyle};
use resvg::{tiny_skia, usvg};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};
use usvg::fontdb;

const FONTS: [&str; 22] = [
    "Segoe UI", "Segoe UI Semibold", "Arial", "Arial Black", "Bahnschrift", "Calibri", "Candara", "Corbel", "Century Gothic", "Franklin Gothic Medium", "Trebuchet MS", "Verdana",
    "Georgia", "Cambria", "Constantia", "Garamond", "Palatino Linotype", "Book Antiqua", "Times New Roman", "Consolas", "Impact", "Comic Sans MS",
];
const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.";
const STYLES: [(&str, f32, bool, bool, f32); 8] = [
    ("Titre 1", 32.0, true, false, 1.1),
    ("Titre 2", 21.0, true, false, 1.15),
    ("Titre 3", 15.0, true, false, 1.2),
    ("Chapeau", 13.0, false, true, 1.4),
    ("Corps", 10.5, false, false, 1.45),
    ("Légende", 8.0, false, true, 1.3),
    ("Citation", 16.0, false, true, 1.35),
    ("Petit texte", 7.5, false, false, 1.3),
];
const PRESETS: [(&str, &str, f32, f32); 10] = [
    ("A4", "Document, magazine", 595.0, 842.0),
    ("A5", "Livret, flyer", 420.0, 595.0),
    ("A3", "Affiche", 842.0, 1191.0),
    ("Lettre US", "Document", 612.0, 792.0),
    ("Carte de visite", "85 × 55 mm", 241.0, 156.0),
    ("Carte postale", "A6", 298.0, 420.0),
    ("Carré", "Publication 1:1", 810.0, 810.0),
    ("Portrait 4:5", "Instagram", 810.0, 1012.0),
    ("Story", "Réseaux sociaux", 540.0, 960.0),
    ("Présentation", "16:9", 960.0, 540.0),
];
const TEMPLATES: [(&str, &str); 6] = [("Magazine", "A4, en-tête, colonnes"), ("Flyer", "A5, événement"), ("CV", "A4, colonne latérale"), ("Carte de visite", "85 × 55 mm"), ("Menu", "A4, carte et prix"), ("Affiche", "A3, grand titre")];
const FORMATS: [&str; 4] = ["PDF", "PNG", "JPEG", "SVG"];
const DPIS: [(&str, f32); 3] = [("72 dpi", 72.0), ("150 dpi", 150.0), ("300 dpi", 300.0)];
const MM: f32 = 72.0 / 25.4;

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Text,
    Image,
    Rect,
    Ellipse,
    Polygon,
    Line,
    Table,
    Pipette,
}

const TOOLS: [(Tool, &str, Key); 9] = [
    (Tool::Select, "Sélection (V)", Key::V),
    (Tool::Text, "Texte (T)", Key::T),
    (Tool::Image, "Image (I)", Key::I),
    (Tool::Rect, "Rectangle (M)", Key::M),
    (Tool::Ellipse, "Ellipse (L)", Key::L),
    (Tool::Polygon, "Polygone et étoile (P)", Key::P),
    (Tool::Line, "Ligne (\\)", Key::Backslash),
    (Tool::Table, "Tableau (G)", Key::G),
    (Tool::Pipette, "Pipette de style (E)", Key::E),
];

fn tool_name(t: Tool) -> &'static str {
    TOOLS.iter().find(|x| x.0 == t).map_or("", |x| x.1.split(" (").next().unwrap_or(""))
}

enum Drag {
    None,
    Move { last: [f32; 2], acc: [f32; 2], origin: [f32; 2] },
    Create { start: [f32; 2] },
    Resize { orig: [f32; 4], hx: f32, hy: f32 },
    Guide { index: usize },
}

struct ExportOpts {
    format: usize,
    pages: usize,
    from: usize,
    to: usize,
    dpi: usize,
    quality: u8,
    marks: bool,
    open_after: bool,
}

struct Fonts {
    db: Arc<fontdb::Database>,
}

impl Fonts {
    fn width(&self, text: &str, family: &str, bold: bool, size: f32) -> f32 {
        let q = fontdb::Query {
            families: &[fontdb::Family::Name(family), fontdb::Family::SansSerif],
            weight: if bold { fontdb::Weight::BOLD } else { fontdb::Weight::NORMAL },
            ..Default::default()
        };
        self.db
            .query(&q)
            .and_then(|id| {
                self.db.with_face_data(id, |data, idx| {
                    let face = ttf_parser::Face::parse(data, idx).ok()?;
                    let upem = face.units_per_em() as f32;
                    let units: f32 = text.chars().map(|c| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map_or(upem * 0.5, |a| a as f32)).sum();
                    Some(units * size / upem)
                })
            })
            .flatten()
            .unwrap_or(text.chars().count() as f32 * size * 0.5)
    }
}

struct PageApp {
    doc: Doc,
    saved: Doc,
    file: Option<PathBuf>,
    home: bool,
    recents: Vec<PathBuf>,
    undo: Vec<Doc>,
    redo: Vec<Doc>,
    editing: bool,
    page: usize,
    sel: Option<usize>,
    tool: Tool,
    drag: Drag,
    fonts: Fonts,
    tex: Option<TextureHandle>,
    rendered: Option<(Doc, usize, f32, bool)>,
    overflow: Vec<bool>,
    zoom: f32,
    fit_pending: bool,
    snap: bool,
    guides: bool,
    rulers: bool,
    preview: bool,
    snap_lines: Vec<(bool, f32)>,
    new_guide: Option<bool>,
    find: Option<(String, String, Option<usize>)>,
    notice: Option<(String, bool)>,
    export: Option<ExportOpts>,
    doc_sheet: bool,
    edit_text: Option<usize>,
    style_name: Option<String>,
    clip: Option<Frame>,
    polygon_sides: u32,
    polygon_inner: f32,
    title: String,
    icon: TextureHandle,
    help: studio_ui::help::Help,
}

fn recents_path() -> PathBuf {
    studio_core::data_dir().join("page").join("recents.txt")
}

fn txt(text: &str, size: f32, bold: bool, color: [u8; 4], align: u8) -> Kind {
    let mut k = Kind::text(text, size, bold);
    if let Kind::Text { color: c, align: a, .. } = &mut k {
        *c = color;
        *a = align;
    }
    k
}

fn styled(mut k: Kind, family: &str, italic: bool, leading: f32, tracking: f32) -> Kind {
    if let Kind::Text { family: f, italic: i, leading: l, tracking: t, .. } = &mut k {
        *f = family.into();
        *i = italic;
        *l = leading;
        *t = tracking;
    }
    k
}

fn starter(w: f32, h: f32) -> Doc {
    let mut doc = Doc { width: w, height: h, margin: (w.min(h) * 0.067).round(), ..Doc::a4() };
    let m = doc.margin;
    let title_size = (w * 0.055).clamp(14.0, 40.0);
    doc.pages[0].frames.push(Frame::new(m, m + 20.0, w - 2.0 * m, title_size * 1.6, Kind::text("Titre du document", title_size, true)));
    doc.master.frames.push(Frame::new(m, h - m * 0.75, w - 2.0 * m, 14.0, txt("{page} / {pages}", 8.0, false, [120, 120, 130, 255], 2)));
    doc
}

fn template(k: usize) -> Doc {
    let dark = [24, 24, 30, 255];
    let white = [255, 255, 255, 255];
    let accent = [255, 112, 52, 255];
    match k {
        0 => {
            let mut d = Doc { grid: 2, title: "Ethone Mag".into(), swatches: vec![accent, dark, [255, 214, 10, 255], [10, 132, 255, 255]], ..Doc::a4() };
            let (w, m) = (d.width, d.margin);
            let mut head = Frame::new(-9.0, -9.0, w + 18.0, 200.0, Kind::Rect { fill: accent, radius: 0.0 });
            head.fill2 = Some([255, 64, 96, 255]);
            head.grad_angle = 0.0;
            d.bleed = 9.0;
            let p = &mut d.pages[0].frames;
            p.push(head);
            p.push(Frame::new(m, 60.0, w - 2.0 * m, 44.0, txt("Le magazine Ethone", 34.0, true, white, 0)));
            p.push(Frame::new(m, 110.0, w * 0.6, 40.0, styled(txt("Mise en page, colonnes liées, styles et export PDF prêt pour l'imprimeur.", 12.0, false, white, 0), "Segoe UI", true, 1.35, 0.0)));
            p.push(Frame::new(m, 230.0, w - 2.0 * m, 160.0, Kind::Image { path: String::new(), cover: true, radius: 6.0, filter: 0, flip: false }));
            let mut body = Frame::new(m, 410.0, w - 2.0 * m, 250.0, txt(&format!("{LOREM}\n{LOREM}\n{LOREM}\n{LOREM}"), 9.5, false, dark, 3));
            if let Kind::Text { columns, para_space, family, indent, .. } = &mut body.kind {
                *columns = 2;
                *para_space = 6.0;
                *indent = 12.0;
                *family = "Georgia".into();
            }
            p.push(body);
            let mut quote = Frame::new(m, 680.0, w - 2.0 * m, 70.0, styled(txt("« Une suite créative entièrement écrite en Rust. »", 16.0, false, accent, 1), "Georgia", true, 1.3, 0.0));
            if let Kind::Text { background, padding, valign, .. } = &mut quote.kind {
                *background = Some([255, 243, 236, 255]);
                *padding = 12.0;
                *valign = 1;
            }
            p.push(quote);
            d.pages.push(Page::default());
            d.master.frames.push(Frame::new(m, d.height - 30.0, w - 2.0 * m, 12.0, txt("{title} · {page} / {pages}", 7.5, false, [130, 130, 140, 255], 2)));
            d
        }
        1 => {
            let mut d = Doc { width: 420.0, height: 595.0, margin: 28.0, bleed: 9.0, ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let mut bg = Frame::new(-9.0, -9.0, w + 18.0, h + 18.0, Kind::Rect { fill: [36, 18, 90, 255], radius: 0.0 });
            bg.fill2 = Some([255, 64, 120, 255]);
            let p = &mut d.pages[0].frames;
            p.push(bg);
            p.push(Frame::new(w - 150.0, 40.0, 120.0, 120.0, Kind::Polygon { sides: 12, inner: 0.8, fill: [255, 214, 10, 255] }));
            p.push(Frame::new(w - 150.0, 82.0, 120.0, 36.0, txt("GRATUIT", 15.0, true, dark, 1)));
            p.push(Frame::new(m, 200.0, w - 2.0 * m, 140.0, styled(txt("SOIRÉE\nCRÉATIVE", 46.0, true, white, 0), "Bahnschrift", false, 1.0, 2.0)));
            p.push(Frame::new(m, 350.0, w - 2.0 * m, 24.0, txt("Samedi 14 décembre · 20 h", 14.0, true, [255, 214, 10, 255], 0)));
            let mut l = Frame::new(m, 385.0, 120.0, 6.0, Kind::Line { color: white, width: 2.0, arrow: false });
            l.dash = 0.0;
            p.push(l);
            p.push(Frame::new(m, 405.0, w - 2.0 * m, 120.0, txt("Ateliers de design\nConcerts\nExposition photo\nBuvette et surprises", 12.0, false, white, 0)));
            if let Kind::Text { list, .. } = &mut p.last_mut().unwrap().kind {
                *list = 1;
            }
            p.push(Frame::new(m, h - 50.0, w - 2.0 * m, 14.0, txt("ethone.dev · entrée libre", 9.0, false, [220, 220, 240, 255], 0)));
            d
        }
        2 => {
            let mut d = Doc { margin: 36.0, ..Doc::a4() };
            let (w, h) = (d.width, d.height);
            let side = 190.0;
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(0.0, 0.0, side, h, Kind::Rect { fill: [28, 36, 54, 255], radius: 0.0 }));
            p.push(Frame::new(45.0, 40.0, 100.0, 100.0, Kind::Image { path: String::new(), cover: true, radius: 50.0, filter: 0, flip: false }));
            p.push(Frame::new(24.0, 165.0, side - 48.0, 16.0, txt("CONTACT", 9.0, true, [140, 170, 255, 255], 0)));
            p.push(Frame::new(24.0, 185.0, side - 48.0, 70.0, txt("prenom@exemple.fr\n06 00 00 00 00\nParis, France", 9.0, false, white, 0)));
            p.push(Frame::new(24.0, 270.0, side - 48.0, 16.0, txt("COMPÉTENCES", 9.0, true, [140, 170, 255, 255], 0)));
            let mut skills = Frame::new(24.0, 290.0, side - 48.0, 120.0, txt("Design graphique\nMise en page\nMontage vidéo\nPhotographie", 9.0, false, white, 0));
            if let Kind::Text { list, para_space, .. } = &mut skills.kind {
                *list = 1;
                *para_space = 3.0;
            }
            p.push(skills);
            let x = side + 30.0;
            let cw = w - x - 36.0;
            p.push(Frame::new(x, 46.0, cw, 40.0, txt("Prénom Nom", 30.0, true, dark, 0)));
            p.push(Frame::new(x, 88.0, cw, 18.0, styled(txt("Graphiste et monteur vidéo", 13.0, false, [90, 100, 130, 255], 0), "Segoe UI", true, 1.2, 0.0)));
            p.push(Frame::new(x, 120.0, cw, 4.0, Kind::Line { color: [28, 36, 54, 255], width: 1.5, arrow: false }));
            for (k, (title, body)) in [("EXPÉRIENCE", "2022 – aujourd'hui · Studio créatif\nConception d'identités visuelles et de supports imprimés.\n\n2019 – 2022 · Agence web\nMaquettes, vidéos courtes et réseaux sociaux."), ("FORMATION", "2019 · Diplôme en design graphique\nÉcole d'arts appliqués")].iter().enumerate() {
                let y = 140.0 + k as f32 * 210.0;
                p.push(Frame::new(x, y, cw, 18.0, txt(title, 11.0, true, [60, 90, 200, 255], 0)));
                let mut b = Frame::new(x, y + 24.0, cw, 170.0, txt(body, 10.0, false, dark, 0));
                if let Kind::Text { para_space, .. } = &mut b.kind {
                    *para_space = 2.0;
                }
                p.push(b);
            }
            d
        }
        3 => {
            let mut d = Doc { width: 241.0, height: 156.0, margin: 14.0, bleed: 6.0, ..Doc::a4() };
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(-6.0, -6.0, 78.0, 168.0, Kind::Rect { fill: [10, 132, 255, 255], radius: 0.0 }));
            p.push(Frame::new(16.0, 52.0, 40.0, 40.0, Kind::Polygon { sides: 6, inner: 1.0, fill: white }));
            p.push(Frame::new(88.0, 34.0, 140.0, 20.0, txt("Prénom Nom", 15.0, true, dark, 0)));
            p.push(Frame::new(88.0, 56.0, 140.0, 14.0, styled(txt("Directeur artistique", 8.5, false, [10, 132, 255, 255], 0), "Segoe UI", true, 1.2, 0.0)));
            p.push(Frame::new(88.0, 76.0, 60.0, 4.0, Kind::Line { color: [200, 200, 210, 255], width: 1.0, arrow: false }));
            p.push(Frame::new(88.0, 86.0, 140.0, 50.0, txt("06 00 00 00 00\nprenom@exemple.fr\nexemple.fr", 7.5, false, [70, 70, 80, 255], 0)));
            d
        }
        4 => {
            let mut d = Doc { margin: 50.0, paper: [250, 246, 238, 255], ..Doc::a4() };
            let (w, m) = (d.width, d.margin);
            let brown = [92, 58, 34, 255];
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(m, 50.0, w - 2.0 * m, 50.0, styled(txt("La Table", 38.0, false, brown, 1), "Georgia", true, 1.1, 1.0)));
            p.push(Frame::new(m, 104.0, w - 2.0 * m, 16.0, styled(txt("CUISINE DE SAISON", 9.0, true, [160, 120, 80, 255], 1), "Segoe UI", false, 1.2, 3.0)));
            p.push(Frame::new(w / 2.0 - 40.0, 128.0, 80.0, 4.0, Kind::Line { color: brown, width: 1.0, arrow: false }));
            for (k, (title, rows)) in [("Entrées", [("Velouté de potimarron", "9 €"), ("Burrata et tomates anciennes", "12 €"), ("Œuf parfait, champignons", "11 €")]), ("Plats", [("Risotto aux cèpes", "19 €"), ("Filet de bar, beurre blanc", "24 €"), ("Joue de bœuf confite", "22 €")]), ("Desserts", [("Tarte au citron meringuée", "8 €"), ("Mousse au chocolat noir", "7 €"), ("Fromages affinés", "9 €")])].iter().enumerate() {
                let y = 160.0 + k as f32 * 200.0;
                p.push(Frame::new(m, y, w - 2.0 * m, 26.0, styled(txt(title, 20.0, false, brown, 1), "Georgia", true, 1.1, 0.0)));
                let cells = rows.iter().map(|(a, b)| vec![a.to_string(), b.to_string()]).collect();
                p.push(Frame::new(m + 20.0, y + 40.0, w - 2.0 * m - 40.0, 96.0, Kind::Table { cells, size: 11.0, family: "Georgia".into(), color: [60, 40, 30, 255], line: [250, 246, 238, 0], header: None, stripe: Some([243, 236, 224, 255]) }));
            }
            d
        }
        _ => {
            let mut d = Doc { width: 842.0, height: 1191.0, margin: 50.0, ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(0.0, 0.0, w, h * 0.55, Kind::Image { path: String::new(), cover: true, radius: 0.0, filter: 0, flip: false }));
            p.push(Frame::new(0.0, h * 0.55, w, h * 0.45, Kind::Rect { fill: [14, 14, 18, 255], radius: 0.0 }));
            p.push(Frame::new(m, h * 0.58, w - 2.0 * m, 220.0, styled(txt("FESTIVAL\nLUMIÈRE", 92.0, true, white, 0), "Bahnschrift", false, 0.95, 4.0)));
            p.push(Frame::new(m, h * 0.78, w - 2.0 * m, 40.0, txt("12 — 15 juin · Parc des expositions", 26.0, false, [255, 214, 10, 255], 0)));
            p.push(Frame::new(w - m - 150.0, h * 0.5, 150.0, 150.0, Kind::Polygon { sides: 16, inner: 0.82, fill: [255, 214, 10, 255] }));
            p.push(Frame::new(w - m - 150.0, h * 0.5 + 55.0, 150.0, 40.0, txt("4 JOURS", 22.0, true, dark, 1)));
            p.push(Frame::new(m, h - 90.0, w - 2.0 * m, 30.0, txt("billetterie sur festival-lumiere.fr", 16.0, false, [180, 180, 190, 255], 0)));
            d
        }
    }
}

impl PageApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let doc = starter(595.0, 842.0);
        let recents = std::fs::read_to_string(recents_path()).map(|s| s.lines().map(PathBuf::from).filter(|p| p.exists()).collect()).unwrap_or_default();
        let mut me = Self {
            saved: doc.clone(),
            doc,
            file: None,
            home: true,
            recents,
            undo: Vec::new(),
            redo: Vec::new(),
            editing: false,
            page: 0,
            sel: None,
            tool: Tool::Select,
            drag: Drag::None,
            fonts: Fonts { db: Arc::new(db) },
            tex: None,
            rendered: None,
            overflow: Vec::new(),
            zoom: 1.0,
            fit_pending: true,
            snap: true,
            guides: true,
            rulers: true,
            preview: false,
            snap_lines: Vec::new(),
            new_guide: None,
            find: None,
            notice: None,
            export: None,
            doc_sheet: false,
            edit_text: None,
            style_name: None,
            clip: None,
            polygon_sides: 5,
            polygon_inner: 0.5,
            title: String::new(),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "page"),
            help: studio_ui::help::Help::new("Page", tutos::PAGE),
        };
        if let Some(p) = std::env::args().nth(1).map(PathBuf::from).filter(|p| p.is_file()) {
            me.open_path(&p);
        }
        me
    }

    fn set_doc(&mut self, doc: Doc, file: Option<PathBuf>) {
        self.saved = doc.clone();
        self.doc = doc;
        self.file = file;
        self.page = 0;
        self.sel = None;
        self.edit_text = None;
        self.undo.clear();
        self.redo.clear();
        self.fit_pending = true;
        self.home = false;
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

    fn measure(&self) -> impl Fn(&str, &str, bool, f32) -> f32 + '_ {
        |t: &str, f: &str, b: bool, s: f32| self.fonts.width(t, f, b, s)
    }

    fn opts(&self) -> usvg::Options<'static> {
        usvg::Options { fontdb: self.fonts.db.clone(), ..Default::default() }
    }

    fn page_svg(&self, index: usize, marks: bool) -> String {
        self.doc.to_svg_with(index, &self.measure(), false, marks).0
    }

    fn raster(&self, index: usize, dpi: f32, marks: bool) -> Result<tiny_skia::Pixmap, String> {
        let svg = self.page_svg(index, marks);
        let tree = usvg::Tree::from_str(&svg, &self.opts()).map_err(|e| e.to_string())?;
        let scale = dpi / 72.0;
        let size = tree.size();
        let mut pix = tiny_skia::Pixmap::new((size.width() * scale).ceil() as u32, (size.height() * scale).ceil() as u32).ok_or("taille")?;
        resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
        Ok(pix)
    }

    fn default_export(&self) -> ExportOpts {
        ExportOpts { format: 0, pages: 0, from: 1, to: self.doc.pages.len(), dpi: 2, quality: 90, marks: false, open_after: true }
    }

    fn run_export(&mut self, o: &ExportOpts) {
        let n = self.doc.pages.len();
        let cur = if self.page == MASTER { 0 } else { self.page };
        let list: Vec<usize> = match o.pages {
            1 => vec![cur],
            2 => (o.from.clamp(1, n) - 1..o.to.clamp(1, n)).collect(),
            _ => (0..n).collect(),
        };
        if list.is_empty() {
            self.notice = Some(("Aucune page à exporter".into(), false));
            return;
        }
        let ext = ["pdf", "png", "jpg", "svg"][o.format];
        let base = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("document".into());
        let single = o.format == 0 || list.len() == 1;
        let target = if single {
            rfd::FileDialog::new().add_filter(FORMATS[o.format], &[ext]).set_file_name(format!("{base}.{ext}")).save_file()
        } else {
            rfd::FileDialog::new().set_title("Dossier d'export").pick_folder()
        };
        let Some(target) = target else { return };
        let marks = o.marks && self.doc.bleed >= 0.0;
        let dpi = DPIS[o.dpi].1;
        let result = (|| -> Result<PathBuf, String> {
            match o.format {
                0 => {
                    let opt = self.opts();
                    let mut document = krilla::Document::new();
                    for &i in &list {
                        let svg = self.page_svg(i, marks);
                        let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| e.to_string())?;
                        let size = krilla::geom::Size::from_wh(tree.size().width(), tree.size().height()).ok_or("taille invalide")?;
                        let mut page = document.start_page_with(krilla::page::PageSettings::new(size));
                        let mut surface = page.surface();
                        surface.draw_svg(&tree, size, SvgSettings::default()).ok_or("rendu SVG impossible")?;
                        surface.finish();
                        page.finish();
                    }
                    let bytes = document.finish().map_err(|e| format!("{e:?}"))?;
                    std::fs::write(&target, bytes).map_err(|e| e.to_string())?;
                    Ok(target.clone())
                }
                f => {
                    for &i in &list {
                        let path = if single { target.clone() } else { target.join(format!("{base}-{:02}.{ext}", self.doc.page_label(i))) };
                        match f {
                            3 => std::fs::write(&path, self.page_svg(i, marks)).map_err(|e| e.to_string())?,
                            1 => self.raster(i, dpi, marks)?.save_png(&path).map_err(|e| e.to_string())?,
                            _ => {
                                let pix = self.raster(i, dpi, marks)?;
                                let mut rgb = image::RgbImage::new(pix.width(), pix.height());
                                for (d, s) in rgb.pixels_mut().zip(pix.pixels()) {
                                    let c = s.demultiply();
                                    let a = c.alpha() as u32;
                                    let mix = |v: u8| ((v as u32 * a + 255 * (255 - a)) / 255) as u8;
                                    d.0 = [mix(c.red()), mix(c.green()), mix(c.blue())];
                                }
                                let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
                                let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), o.quality);
                                enc.encode_image(&rgb).map_err(|e| e.to_string())?;
                            }
                        }
                    }
                    Ok(target.clone())
                }
            }
        })();
        self.notice = Some(match result {
            Ok(p) => {
                if o.open_after {
                    let _ = std::process::Command::new("explorer").arg(&p).spawn();
                }
                (format!("{} exporté : {}", FORMATS[o.format], p.display()), true)
            }
            Err(e) => (format!("Export {} : {e}", FORMATS[o.format]), false),
        });
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.file, as_new) {
            (Some(p), false) => p.clone(),
            _ => match rfd::FileDialog::new().add_filter("Projet Page", &["epage"]).set_file_name("document.epage").save_file() {
                Some(p) => p,
                None => return,
            },
        };
        match serde_json::to_vec_pretty(&self.doc).map_err(|e| e.to_string()).and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string())) {
            Ok(()) => {
                self.remember(&path);
                self.file = Some(path);
                self.saved = self.doc.clone();
            }
            Err(e) => self.notice = Some((format!("Enregistrement : {e}"), false)),
        }
    }

    fn open_path(&mut self, p: &Path) {
        match std::fs::read(p).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Doc>(b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&b)).map_err(|e| e.to_string())) {
            Ok(d) => {
                self.set_doc(d, Some(p.to_path_buf()));
                self.remember(p);
            }
            Err(e) => self.notice = Some((format!("Ouverture : {e}"), false)),
        }
    }

    fn open(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("Projet Page", &["epage"]).pick_file() {
            self.open_path(&p);
        }
    }

    fn frames(&mut self) -> &mut Vec<Frame> {
        if self.page == MASTER { &mut self.doc.master.frames } else { &mut self.doc.pages[self.page].frames }
    }

    fn frames_ref(&self) -> &Vec<Frame> {
        if self.page == MASTER { &self.doc.master.frames } else { &self.doc.pages[self.page].frames }
    }

    fn push_frame(&mut self, f: Frame) {
        self.frames().push(f);
        self.sel = Some(self.frames().len() - 1);
    }

    fn remove_frame(&mut self, i: usize) {
        let fr = self.frames();
        if i >= fr.len() {
            return;
        }
        fr.remove(i);
        for f in fr.iter_mut() {
            if let Kind::Text { link, .. } = &mut f.kind {
                *link = match *link {
                    Some(l) if l == i => None,
                    Some(l) if l > i => Some(l - 1),
                    l => l,
                };
            }
        }
        self.sel = None;
        self.edit_text = None;
    }

    fn duplicate(&mut self) {
        if let Some(i) = self.sel {
            let mut f = self.frames()[i].clone();
            f.x += 12.0;
            f.y += 12.0;
            if let Kind::Text { link, .. } = &mut f.kind {
                *link = None;
            }
            self.push_frame(f);
        }
    }

    fn paste(&mut self, in_place: bool) {
        if let Some(mut f) = self.clip.clone() {
            if !in_place {
                f.x += 12.0;
                f.y += 12.0;
            }
            if let Kind::Text { link, .. } = &mut f.kind {
                *link = None;
            }
            self.push_frame(f);
        }
    }

    fn order(&mut self, to: i32) {
        let Some(i) = self.sel else { return };
        let n = self.frames().len();
        let j = match to {
            i32::MAX => n - 1,
            i32::MIN => 0,
            d => (i as i32 + d).clamp(0, n as i32 - 1) as usize,
        };
        if i == j {
            return;
        }
        let mut order: Vec<usize> = (0..n).collect();
        let x = order.remove(i);
        order.insert(j, x);
        let new_of: Vec<usize> = (0..n).map(|old| order.iter().position(|o| *o == old).unwrap()).collect();
        let old = std::mem::take(self.frames());
        let mut slots: Vec<Option<Frame>> = old.into_iter().map(Some).collect();
        let mut fresh: Vec<Frame> = order.iter().map(|o| slots[*o].take().unwrap()).collect();
        for f in &mut fresh {
            if let Kind::Text { link: Some(l), .. } = &mut f.kind {
                *l = new_of[*l];
            }
        }
        *self.frames() = fresh;
        self.sel = Some(j);
    }

    fn align(&mut self, axis: usize, to: f32, page: bool) {
        let Some(i) = self.sel else { return };
        let (m, pw, ph) = (if page { 0.0 } else { self.doc.margin }, self.doc.width, self.doc.height);
        let f = &mut self.frames()[i];
        let (size, span) = if axis == 0 { (f.w, pw) } else { (f.h, ph) };
        let v = m + (span - 2.0 * m - size) * to;
        if axis == 0 { f.x = v } else { f.y = v }
    }

    fn fit_text(&mut self) {
        let Some(i) = self.sel else { return };
        let f = self.frames_ref()[i].clone();
        let h = self.doc.fit_height(&f, &self.measure());
        if let Some(h) = h {
            self.frames()[i].h = h.ceil();
        }
    }

    fn fit_frame_to_image(&mut self) {
        let Some(i) = self.sel else { return };
        let Kind::Image { path, .. } = &self.frames_ref()[i].kind else { return };
        if let Ok((w, h)) = image::image_dimensions(path) {
            let f = &mut self.frames()[i];
            f.h = f.w * h as f32 / w.max(1) as f32;
        }
    }

    fn add_page(&mut self, at: usize, copy: bool) {
        let p = if copy && self.page != MASTER {
            let mut p = self.doc.pages[self.page].clone();
            for f in &mut p.frames {
                if let Kind::Text { link, .. } = &mut f.kind {
                    *link = None;
                }
            }
            p
        } else {
            Page::default()
        };
        let at = at.min(self.doc.pages.len());
        self.doc.pages.insert(at, p);
        self.page = at;
        self.sel = None;
    }

    fn move_page(&mut self, by: i32) {
        if self.page == MASTER {
            return;
        }
        let to = self.page as i32 + by;
        if to >= 0 && (to as usize) < self.doc.pages.len() {
            self.doc.pages.swap(self.page, to as usize);
            self.page = to as usize;
        }
    }

    fn delete_page(&mut self, i: usize) {
        if self.doc.pages.len() > 1 && i < self.doc.pages.len() {
            self.doc.pages.remove(i);
            self.page = self.page.min(self.doc.pages.len() - 1);
            self.sel = None;
        }
    }

    fn replace_all(&mut self, find: &str, with: &str) -> usize {
        if find.is_empty() {
            return 0;
        }
        let mut n = 0;
        for f in self.doc.pages.iter_mut().flat_map(|p| p.frames.iter_mut()).chain(self.doc.master.frames.iter_mut()) {
            match &mut f.kind {
                Kind::Text { text, .. } => {
                    n += text.matches(find).count();
                    *text = text.replace(find, with);
                }
                Kind::Table { cells, .. } => {
                    for c in cells.iter_mut().flatten() {
                        n += c.matches(find).count();
                        *c = c.replace(find, with);
                    }
                }
                _ => {}
            }
        }
        n
    }

    fn word_count(&self) -> usize {
        self.doc.pages.iter().flat_map(|p| p.frames.iter()).map(|f| match &f.kind {
            Kind::Text { text, .. } => text.split_whitespace().count(),
            Kind::Table { cells, .. } => cells.iter().flatten().map(|c| c.split_whitespace().count()).sum(),
            _ => 0,
        }).sum()
    }

    fn apply_style(&mut self, k: usize) {
        let Some(i) = self.sel else { return };
        let user = self.doc.styles.clone();
        if let Kind::Text { size, bold, italic, leading, family, color, tracking, para_space, .. } = &mut self.frames()[i].kind {
            if k < STYLES.len() {
                let (_, sz, b, it, lead) = STYLES[k];
                (*size, *bold, *italic, *leading) = (sz, b, it, lead);
            } else if let Some(s) = user.get(k - STYLES.len()) {
                (*size, *bold, *italic, *leading, *tracking, *para_space) = (s.size, s.bold, s.italic, s.leading, s.tracking, s.para_space);
                *family = s.family.clone();
                *color = s.color;
            }
        }
    }

    fn save_style(&mut self, name: String) {
        let Some(i) = self.sel else { return };
        if let Kind::Text { size, bold, italic, leading, family, color, tracking, para_space, .. } = &self.frames_ref()[i].kind {
            let st = TextStyle { name, size: *size, family: family.clone(), bold: *bold, italic: *italic, leading: *leading, tracking: *tracking, color: *color, para_space: *para_space };
            self.doc.styles.retain(|s| s.name != st.name);
            self.doc.styles.push(st);
        }
    }

    fn copy_style(&mut self, from: usize, to: usize) {
        let src = self.frames_ref()[from].clone();
        let dst = &mut self.frames()[to];
        dst.opacity = src.opacity;
        dst.stroke = src.stroke;
        dst.stroke_w = src.stroke_w;
        dst.shadow = src.shadow;
        dst.dash = src.dash;
        dst.fill2 = src.fill2;
        dst.grad_angle = src.grad_angle;
        match (&src.kind, &mut dst.kind) {
            (Kind::Text { size, family, bold, color, align, leading, italic, tracking, background, padding, underline, upper, para_space, .. }, Kind::Text { size: s2, family: f2, bold: b2, color: c2, align: a2, leading: l2, italic: i2, tracking: t2, background: bg2, padding: p2, underline: u2, upper: up2, para_space: ps2, .. }) => {
                (*s2, *b2, *c2, *a2, *l2, *i2, *t2, *bg2, *p2, *u2, *up2, *ps2) = (*size, *bold, *color, *align, *leading, *italic, *tracking, *background, *padding, *underline, *upper, *para_space);
                *f2 = family.clone();
            }
            (Kind::Rect { fill, radius }, Kind::Rect { fill: f2, radius: r2 }) => {
                *f2 = *fill;
                *r2 = *radius;
            }
            (Kind::Rect { fill, .. } | Kind::Ellipse { fill } | Kind::Polygon { fill, .. }, Kind::Rect { fill: f2, .. } | Kind::Ellipse { fill: f2 } | Kind::Polygon { fill: f2, .. }) => *f2 = *fill,
            (Kind::Line { color, width, arrow }, Kind::Line { color: c2, width: w2, arrow: a2 }) => (*c2, *w2, *a2) = (*color, *width, *arrow),
            (Kind::Image { radius, filter, .. }, Kind::Image { radius: r2, filter: f2, .. }) => (*r2, *f2) = (*radius, *filter),
            _ => {}
        }
    }

    fn undo_step(&mut self) {
        if let Some(d) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, d));
        }
    }

    fn redo_step(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, d));
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            if self.edit_text.is_some() && ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.edit_text = None;
            }
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        if sc(M::COMMAND, Key::N) {
            self.home = true;
        }
        if sc(M::COMMAND, Key::O) {
            self.open();
        }
        if self.home {
            return;
        }
        if sc(M::COMMAND | M::SHIFT, Key::Z) || sc(M::COMMAND, Key::Y) {
            self.redo_step();
        } else if sc(M::COMMAND, Key::Z) {
            self.undo_step();
        }
        if sc(M::COMMAND | M::SHIFT, Key::S) {
            self.save(true);
        } else if sc(M::COMMAND, Key::S) {
            self.save(false);
        }
        if sc(M::COMMAND, Key::E) || sc(M::COMMAND, Key::P) {
            self.export = Some(self.default_export());
        }
        if sc(M::COMMAND, Key::K) {
            self.doc_sheet = true;
        }
        if sc(M::COMMAND, Key::D) {
            self.duplicate();
        }
        if sc(M::COMMAND, Key::C)
            && let Some(i) = self.sel
        {
            self.clip = self.frames_ref().get(i).cloned();
        }
        if sc(M::COMMAND, Key::X)
            && let Some(i) = self.sel
        {
            self.clip = self.frames_ref().get(i).cloned();
            self.remove_frame(i);
        }
        if sc(M::COMMAND | M::SHIFT, Key::V) {
            self.paste(true);
        } else if sc(M::COMMAND, Key::V) {
            self.paste(false);
        }
        if sc(M::COMMAND, Key::F) {
            self.find = Some((String::new(), String::new(), None));
        }
        if sc(M::COMMAND, Key::L)
            && let Some(i) = self.sel.take()
        {
            self.frames()[i].locked = true;
        }
        if sc(M::COMMAND, Key::R) {
            self.rulers = !self.rulers;
        }
        if sc(M::COMMAND | M::SHIFT, Key::CloseBracket) {
            self.order(i32::MAX);
        } else if sc(M::COMMAND, Key::CloseBracket) {
            self.order(1);
        }
        if sc(M::COMMAND | M::SHIFT, Key::OpenBracket) {
            self.order(i32::MIN);
        } else if sc(M::COMMAND, Key::OpenBracket) {
            self.order(-1);
        }
        if sc(M::COMMAND, Key::Num0) {
            self.fit_pending = true;
        }
        if sc(M::COMMAND, Key::Num1) {
            self.zoom = 1.0;
        }
        if sc(M::COMMAND, Key::Plus) || sc(M::COMMAND, Key::Equals) {
            self.zoom = (self.zoom * 1.25).min(6.0);
        }
        if sc(M::COMMAND, Key::Minus) {
            self.zoom = (self.zoom / 1.25).max(0.1);
        }
        let (del, pick, nudge, pg, w_key, enter, esc) = ctx.input(|i| {
            let mut pick = None;
            if !i.modifiers.command {
                for (t, _, k) in TOOLS {
                    if i.key_pressed(k) {
                        pick = Some(t);
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
            let pg = if i.key_pressed(Key::PageDown) { 1 } else if i.key_pressed(Key::PageUp) { -1 } else { 0 };
            (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace), pick, n, pg, i.key_pressed(Key::W) && !i.modifiers.command, i.key_pressed(Key::Enter), i.key_pressed(Key::Escape))
        });
        if let Some(t) = pick {
            self.tool = t;
        }
        if w_key {
            self.preview = !self.preview;
        }
        if esc {
            self.edit_text = None;
            self.sel = None;
            self.tool = Tool::Select;
        }
        if enter && let Some(i) = self.sel.filter(|i| matches!(self.frames_ref().get(*i).map(|f| &f.kind), Some(Kind::Text { .. }))) {
            self.edit_text = Some(i);
        }
        if del && let Some(i) = self.sel {
            self.remove_frame(i);
        }
        if nudge != [0.0, 0.0]
            && let Some(i) = self.sel
        {
            let f = &mut self.frames()[i];
            f.x += nudge[0];
            f.y += nudge[1];
        }
        if pg != 0 && self.page != MASTER {
            self.page = (self.page as i32 + pg).clamp(0, self.doc.pages.len() as i32 - 1) as usize;
            self.sel = None;
        }
        if self.page != MASTER {
            self.page = self.page.min(self.doc.pages.len() - 1);
        }
        if self.sel.is_some_and(|i| i >= self.frames_ref().len()) {
            self.sel = None;
        }
        if self.edit_text.is_some_and(|i| i >= self.frames_ref().len()) {
            self.edit_text = None;
        }
    }
}

impl eframe::App for PageApp {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let before = self.doc.clone();
        self.keys(ctx);
        let name = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = if self.home { "Page".to_string() } else { format!("{name}{} — Page", if self.doc != self.saved { " •" } else { "" }) };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.top_bar(root);
        if self.home {
            self.home_screen(root);
        } else {
            self.options_bar(root);
            self.status(root);
            self.tool_rail(root);
            self.pages_panel(root);
            self.props(root);
            self.canvas(root, &name);
            self.text_editor(ctx);
        }
        self.sheets(ctx);
        self.help.show(ctx);
        if !self.home && self.doc != before && !self.editing {
            self.undo.push(before);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.editing = true;
        }
        if ctx.input(|i| !i.pointer.any_down()) && !ctx.egui_wants_keyboard_input() {
            self.editing = false;
        }
    }
}

impl PageApp {
    fn top_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(44.0).frame(EFrame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let full = ui.max_rect();
            let right_w = if self.home { 0.0 } else { 300.0 };
            let left = Rect::from_min_max(full.min, pos2(full.right() - right_w, full.bottom()));
            ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(22.0))));
                    ui.add_space(4.0);
                    egui::MenuBar::new().ui(ui, |ui| self.menus(ui));
                });
            });
            if !self.home {
                let right = Rect::from_min_max(pos2(full.right() - right_w, full.top()), full.max);
                ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::primary_icon(ui, "Exporter", export_glyph).on_hover_text("PDF, PNG, JPEG ou SVG · Ctrl+E").clicked() {
                            self.export = Some(self.default_export());
                        }
                        ui.add_space(4.0);
                        let fmt = PRESETS.iter().find(|p| (p.2 - self.doc.width).abs() < 0.5 && (p.3 - self.doc.height).abs() < 0.5).map_or(format!("{:.0} × {:.0} mm", self.doc.width / MM, self.doc.height / MM), |p| p.0.to_string());
                        let n = self.doc.pages.len();
                        if pro::chip(ui, &format!("{fmt} · {n} page{}", if n > 1 { "s" } else { "" })).on_hover_text("Format du document · Ctrl+K").clicked() {
                            self.doc_sheet = true;
                        }
                    });
                });
            }
        });
    }

    fn menus(&mut self, ui: &mut Ui) {
        ui.menu_button("Fichier", |ui| {
            if ui.button("Nouveau…                    Ctrl+N").clicked() {
                self.home = true;
            }
            if ui.button("Ouvrir…                       Ctrl+O").clicked() {
                self.open();
            }
            ui.menu_button("Ouvrir un document récent", |ui| {
                let mut open = None;
                for p in &self.recents {
                    if ui.button(p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).clicked() {
                        open = Some(p.clone());
                    }
                }
                if self.recents.is_empty() {
                    ui.label(RichText::new("Aucun document récent").color(FAINT));
                }
                if let Some(p) = open {
                    self.open_path(&p);
                }
            });
            ui.add_enabled_ui(!self.home, |ui| {
                ui.separator();
                if ui.button("Enregistrer                   Ctrl+S").clicked() {
                    self.save(false);
                }
                if ui.button("Enregistrer sous…      Ctrl+Maj+S").clicked() {
                    self.save(true);
                }
                ui.separator();
                if ui.button("Exporter…                      Ctrl+E").clicked() {
                    self.export = Some(self.default_export());
                }
                if ui.button("Format du document…    Ctrl+K").clicked() {
                    self.doc_sheet = true;
                }
            });
        });
        if self.home {
            self.help.menu(ui);
            return;
        }
        let has = self.sel.is_some();
        ui.menu_button("Édition", |ui| {
            if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler                    Ctrl+Z")).clicked() {
                self.undo_step();
            }
            if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir                   Ctrl+Y")).clicked() {
                self.redo_step();
            }
            ui.separator();
            if ui.add_enabled(has, egui::Button::new("Couper                       Ctrl+X")).clicked()
                && let Some(i) = self.sel
            {
                self.clip = self.frames_ref().get(i).cloned();
                self.remove_frame(i);
            }
            if ui.add_enabled(has, egui::Button::new("Copier                       Ctrl+C")).clicked()
                && let Some(i) = self.sel
            {
                self.clip = self.frames_ref().get(i).cloned();
            }
            if ui.add_enabled(self.clip.is_some(), egui::Button::new("Coller                        Ctrl+V")).clicked() {
                self.paste(false);
            }
            if ui.add_enabled(self.clip.is_some(), egui::Button::new("Coller sur place        Ctrl+Maj+V")).clicked() {
                self.paste(true);
            }
            if ui.add_enabled(has, egui::Button::new("Dupliquer                  Ctrl+D")).clicked() {
                self.duplicate();
            }
            if ui.add_enabled(has, egui::Button::new("Supprimer                  Suppr")).clicked()
                && let Some(i) = self.sel
            {
                self.remove_frame(i);
            }
            ui.separator();
            if ui.button("Rechercher et remplacer…   Ctrl+F").clicked() {
                self.find = Some((String::new(), String::new(), None));
            }
        });
        ui.menu_button("Objet", |ui| {
            for (label, to) in [("Premier plan          Ctrl+Maj+]", i32::MAX), ("Avancer                    Ctrl+]", 1), ("Reculer                     Ctrl+[", -1), ("Arrière-plan           Ctrl+Maj+[", i32::MIN)] {
                if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                    self.order(to);
                }
            }
            ui.separator();
            ui.add_enabled_ui(has, |ui| {
                ui.menu_button("Aligner sur les marges", |ui| {
                    for (label, axis, to) in [("Gauche", 0, 0.0), ("Centre horizontal", 0, 0.5), ("Droite", 0, 1.0), ("Haut", 1, 0.0), ("Centre vertical", 1, 0.5), ("Bas", 1, 1.0)] {
                        if ui.button(label).clicked() {
                            self.align(axis, to, false);
                        }
                    }
                });
                if ui.button("Centrer sur la page").clicked() {
                    self.align(0, 0.5, true);
                    self.align(1, 0.5, true);
                }
                if ui.button("Remplir la page (fond perdu compris)").clicked()
                    && let Some(i) = self.sel
                {
                    let (w, h, b) = (self.doc.width, self.doc.height, self.doc.bleed);
                    let f = &mut self.frames()[i];
                    (f.x, f.y, f.w, f.h) = (-b, -b, w + 2.0 * b, h + 2.0 * b);
                }
            });
            ui.separator();
            if ui.add_enabled(has, egui::Button::new("Verrouiller                 Ctrl+L")).clicked()
                && let Some(i) = self.sel.take()
            {
                self.frames()[i].locked = true;
            }
            if ui.button("Tout déverrouiller").clicked() {
                self.frames().iter_mut().for_each(|f| f.locked = false);
            }
            if ui.button("Tout afficher").clicked() {
                self.frames().iter_mut().for_each(|f| f.hidden = false);
            }
        });
        ui.menu_button("Texte", |ui| {
            let is_text = self.sel.is_some_and(|i| matches!(self.frames_ref()[i].kind, Kind::Text { .. }));
            ui.add_enabled_ui(is_text, |ui| {
                ui.menu_button("Style de paragraphe", |ui| {
                    for (k, (name, ..)) in STYLES.iter().enumerate() {
                        if ui.button(*name).clicked() {
                            self.apply_style(k);
                        }
                    }
                    let user: Vec<String> = self.doc.styles.iter().map(|s| s.name.clone()).collect();
                    if !user.is_empty() {
                        ui.separator();
                    }
                    for (k, n) in user.iter().enumerate() {
                        if ui.button(n).clicked() {
                            self.apply_style(STYLES.len() + k);
                        }
                    }
                });
                if ui.button("Modifier le texte              Entrée").clicked() {
                    self.edit_text = self.sel;
                }
                if ui.button("Ajuster la hauteur au texte").clicked() {
                    self.fit_text();
                }
                if ui.button("Remplir avec du faux texte").clicked() {
                    self.lorem();
                }
            });
            ui.separator();
            ui.label(RichText::new("Variables : {page} {pages} {title} {date}").size(11.5).color(FAINT));
        });
        ui.menu_button("Page", |ui| {
            let cur = if self.page == MASTER { 0 } else { self.page };
            if ui.button("Ajouter une page après").clicked() {
                self.add_page(cur + 1, false);
            }
            if ui.button("Ajouter une page avant").clicked() {
                self.add_page(cur, false);
            }
            if ui.button("Dupliquer la page").clicked() {
                self.add_page(cur + 1, true);
            }
            if ui.add_enabled(self.page != MASTER && self.page > 0, egui::Button::new("Déplacer vers le haut")).clicked() {
                self.move_page(-1);
            }
            if ui.add_enabled(self.page != MASTER && self.page + 1 < self.doc.pages.len(), egui::Button::new("Déplacer vers le bas")).clicked() {
                self.move_page(1);
            }
            if ui.add_enabled(self.page != MASTER && self.doc.pages.len() > 1, egui::Button::new("Supprimer la page")).clicked() {
                self.delete_page(self.page);
            }
            ui.separator();
            if ui.button("Modifier le gabarit").clicked() {
                self.page = MASTER;
                self.sel = None;
            }
        });
        ui.menu_button("Affichage", |ui| {
            if ui.button("Ajuster à la fenêtre        Ctrl+0").clicked() {
                self.fit_pending = true;
            }
            if ui.button("Taille réelle                    Ctrl+1").clicked() {
                self.zoom = 1.0;
            }
            for z in [0.5, 1.5, 2.0] {
                if ui.button(format!("{:.0} %", z * 100.0)).clicked() {
                    self.zoom = z;
                }
            }
            ui.separator();
            ui.checkbox(&mut self.preview, "Aperçu (masquer les repères)   W");
            ui.checkbox(&mut self.guides, "Marges, colonnes et grille de base");
            ui.checkbox(&mut self.rulers, "Règles                                Ctrl+R");
            ui.checkbox(&mut self.snap, "Magnétisme");
            ui.separator();
            if ui.add_enabled(!self.doc.guides.is_empty(), egui::Button::new("Effacer tous les repères")).clicked() {
                self.doc.guides.clear();
            }
        });
        self.help.menu(ui);
    }

    fn lorem(&mut self) {
        let Some(i) = self.sel else { return };
        if let Kind::Text { text, .. } = &mut self.frames()[i].kind {
            *text = format!("{LOREM}\n\n{LOREM}");
        }
    }

    fn home_screen(&mut self, root: &mut Ui) {
        egui::CentralPanel::default().frame(EFrame::new().fill(PASTEBOARD).inner_margin(Margin { left: 40, right: 40, top: 34, bottom: 20 })).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(40.0))));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Bienvenue dans Page").font(semibold(28.0)).color(INK_STRONG));
                        ui.label(RichText::new("Mise en page de documents, magazines, affiches et PDF").color(MUTED));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::secondary(ui, "Ouvrir…").clicked() {
                            self.open();
                        }
                    });
                });
                ui.add_space(26.0);
                ui.label(RichText::new("Nouveau document").font(semibold(16.0)).color(INK));
                ui.add_space(12.0);
                let mut chosen = None;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                    for (name, kind, w, h) in PRESETS {
                        if preset_card(ui, name, &format!("{kind} · {w:.0} × {h:.0} pt"), w, h, false).clicked() {
                            chosen = Some(starter(w, h));
                        }
                    }
                });
                ui.add_space(24.0);
                ui.label(RichText::new("Modèles").font(semibold(16.0)).color(INK));
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                    for (k, (name, hint)) in TEMPLATES.iter().enumerate() {
                        let d = template(k);
                        if preset_card(ui, name, hint, d.width, d.height, true).clicked() {
                            chosen = Some(d);
                        }
                    }
                });
                if let Some(d) = chosen {
                    self.set_doc(d, None);
                }
                ui.add_space(26.0);
                ui.label(RichText::new("Documents récents").font(semibold(16.0)).color(INK));
                ui.add_space(10.0);
                if self.recents.is_empty() {
                    ui.label(RichText::new("Tes documents .epage apparaîtront ici").color(FAINT));
                }
                let mut open = None;
                for p in &self.recents {
                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width().min(760.0), 46.0), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(r, CornerRadius::same(10), Color32::from_white_alpha(10));
                    }
                    let nm = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    ui.painter().text(r.left_top() + vec2(14.0, 8.0), Align2::LEFT_TOP, nm, semibold(14.0), INK);
                    ui.painter().text(r.left_top() + vec2(14.0, 27.0), Align2::LEFT_TOP, p.parent().map(|d| d.display().to_string()).unwrap_or_default(), FontId::proportional(12.0), FAINT);
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = Some(p.clone());
                    }
                }
                if let Some(p) = open {
                    self.open_path(&p);
                }
            });
        });
    }

    fn options_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("options").exact_size(42.0).frame(EFrame::new().fill(Color32::from_rgb(0x20, 0x20, 0x24)).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(7), FIELD);
                tool_icon(ui.painter(), r.shrink(7.0), self.tool, INK);
                ui.label(RichText::new(tool_name(self.tool)).font(semibold(13.0)).color(INK));
                pro::toolbar_sep(ui);
                let swatches = self.doc.swatches.clone();
                let sel = self.sel.filter(|i| *i < self.frames_ref().len());
                match (self.tool, sel) {
                    (Tool::Polygon, _) => {
                        ui.label(RichText::new("Côtés").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(&mut self.polygon_sides).range(3..=24));
                        ui.label(RichText::new("Creux").size(12.5).color(MUTED));
                        ui.add(egui::Slider::new(&mut self.polygon_inner, 0.2..=1.0).show_value(false));
                        ui.label(RichText::new(if self.polygon_inner < 0.99 { "étoile" } else { "polygone" }).size(12.0).color(FAINT));
                    }
                    (Tool::Select, Some(i)) => {
                        let mut style = None;
                        let f = &mut self.frames()[i];
                        match &mut f.kind {
                            Kind::Text { family, size, bold, italic, underline, upper, align, color, .. } => {
                                egui::ComboBox::from_id_salt("ofont").width(150.0).selected_text(family.as_str()).show_ui(ui, |ui| {
                                    for name in FONTS {
                                        ui.selectable_value(family, name.to_string(), name);
                                    }
                                });
                                ui.add(egui::DragValue::new(size).range(4.0..=400.0).speed(0.2).suffix(" pt").max_decimals(1));
                                ui.toggle_value(bold, RichText::new("G").strong()).on_hover_text("Gras");
                                ui.toggle_value(italic, RichText::new("I").italics()).on_hover_text("Italique");
                                ui.toggle_value(underline, RichText::new("S").underline()).on_hover_text("Souligné");
                                ui.toggle_value(upper, RichText::new("AA")).on_hover_text("Majuscules");
                                pro::toolbar_sep(ui);
                                let mut a = *align as usize;
                                if pro::segmented(ui, &mut a, &["Gauche", "Centre", "Droite", "Justifié"]) {
                                    *align = a as u8;
                                }
                                pro::toolbar_sep(ui);
                                swatch_button(ui, color, &swatches);
                                ui.menu_button("Styles", |ui| {
                                    for (k, (name, ..)) in STYLES.iter().enumerate() {
                                        if ui.button(*name).clicked() {
                                            style = Some(k);
                                        }
                                    }
                                });
                            }
                            Kind::Rect { fill, .. } | Kind::Ellipse { fill } | Kind::Polygon { fill, .. } => {
                                ui.label(RichText::new("Fond").size(12.5).color(MUTED));
                                swatch_button(ui, fill, &swatches);
                                ui.label(RichText::new("Contour").size(12.5).color(MUTED));
                                let mut on = f.stroke.is_some();
                                if ui.checkbox(&mut on, "").changed() {
                                    f.stroke = on.then_some([20, 20, 24, 255]);
                                }
                                if let Some(c) = &mut f.stroke {
                                    swatch_button(ui, c, &swatches);
                                    ui.add(egui::DragValue::new(&mut f.stroke_w).range(0.25..=40.0).speed(0.1).suffix(" pt"));
                                }
                            }
                            Kind::Line { color, width, arrow } => {
                                swatch_button(ui, color, &swatches);
                                ui.add(egui::DragValue::new(width).range(0.25..=40.0).speed(0.1).suffix(" pt"));
                                ui.checkbox(arrow, "Flèche");
                            }
                            Kind::Image { cover, filter, .. } => {
                                let mut c = if *cover { 0 } else { 1 };
                                if pro::segmented(ui, &mut c, &["Remplir", "Ajuster"]) {
                                    *cover = c == 0;
                                }
                                let mut fl = *filter as usize;
                                if pro::segmented(ui, &mut fl, &FILTERS) {
                                    *filter = fl as u8;
                                }
                            }
                            Kind::Table { size, .. } => {
                                ui.label(RichText::new("Taille du texte").size(12.5).color(MUTED));
                                ui.add(egui::DragValue::new(size).range(4.0..=72.0).speed(0.2).suffix(" pt"));
                            }
                        }
                        if let Some(k) = style {
                            self.apply_style(k);
                        }
                    }
                    _ => {
                        let hint = match self.tool {
                            Tool::Select => "Clique un bloc pour le modifier · double-clic sur un texte pour l'écrire · glisse depuis les règles pour poser un repère",
                            Tool::Image => "Trace le cadre puis choisis l'image",
                            Tool::Table => "Trace le tableau, puis règle lignes et colonnes à droite",
                            Tool::Pipette => "Sélectionne un bloc, puis clique un autre bloc pour lui prendre son style",
                            _ => "Trace le bloc sur la page · Maj garde les proportions",
                        };
                        ui.label(RichText::new(hint).size(12.5).color(MUTED));
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::ghost(ui, "?  Tutoriel").on_hover_text("F1").clicked() {
                        self.help.learn(tool_name(self.tool));
                    }
                });
            });
        });
    }

    fn status(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(28.0).frame(EFrame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                let where_ = if self.page == MASTER { "Gabarit".to_string() } else { format!("Page {} sur {}", self.page + 1, self.doc.pages.len()) };
                ui.label(small(format!("{where_} · {:.0} × {:.0} mm · {} mots", self.doc.width / MM, self.doc.height / MM, self.word_count())));
                if self.overflow.iter().any(|o| *o) {
                    ui.label(RichText::new("· texte en excès sur cette page").size(12.0).color(RED));
                }
                if let Some((n, ok)) = self.notice.clone() {
                    ui.label(RichText::new(n).size(12.0).color(if ok { GREEN } else { RED }));
                    if ui.small_button("×").clicked() {
                        self.notice = None;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Ajuster").on_hover_text("Ctrl+0").clicked() {
                        self.fit_pending = true;
                    }
                    if ui.small_button("+").clicked() {
                        self.zoom = (self.zoom * 1.25).min(6.0);
                    }
                    ui.label(RichText::new(format!("{:.0} %", self.zoom * 100.0)).size(12.0).color(INK).monospace());
                    if ui.small_button("−").clicked() {
                        self.zoom = (self.zoom / 1.25).max(0.1);
                    }
                    ui.add_space(8.0);
                    ui.toggle_value(&mut self.preview, RichText::new("Aperçu").size(12.0)).on_hover_text("Masque cadres et repères (W)");
                });
            });
        });
    }

    fn tool_rail(&mut self, root: &mut Ui) {
        egui::Panel::left("tools").resizable(false).exact_size(52.0).frame(EFrame::new().fill(PANEL).inner_margin(Margin::symmetric(7, 10))).show(root, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for (i, (tool, tip, _)) in TOOLS.iter().enumerate() {
                if i == 1 || i == 8 {
                    let (sep, _) = ui.allocate_exact_size(vec2(38.0, 9.0), Sense::hover());
                    ui.painter().line_segment([sep.left_center() + vec2(6.0, 0.0), sep.right_center() - vec2(6.0, 0.0)], Stroke::new(1.0, LINE));
                }
                let t = *tool;
                if pro::icon_button(ui, 38.0, self.tool == t, tip, |p, r, c| tool_icon(p, r, t, c)).clicked() {
                    self.tool = t;
                }
            }
        });
    }

    fn thumb(&self, ui: &mut Ui, page: Option<&Page>, frames: &[Frame], selected: bool, label: &str) -> egui::Response {
        let ratio = self.doc.height / self.doc.width;
        let w = 96.0f32.min(130.0 / ratio);
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), w * ratio + 24.0), Sense::click());
        let card = Rect::from_center_size(pos2(r.center().x, r.min.y + w * ratio / 2.0 + 4.0), vec2(w, w * ratio));
        let p = ui.painter();
        p.add(egui::epaint::Shadow { offset: [0, 2], blur: 6, spread: 0, color: Color32::from_black_alpha(90) }.as_shape(card, CornerRadius::same(2)));
        p.rect_filled(card, CornerRadius::same(2), Color32::from_rgb(self.doc.paper[0], self.doc.paper[1], self.doc.paper[2]));
        let s = w / self.doc.width;
        let master: &[Frame] = if page.is_some_and(|p| !p.no_master) { &self.doc.master.frames } else { &[] };
        let pc = p.with_clip_rect(card);
        for f in master.iter().chain(frames).filter(|f| !f.hidden) {
            let fr = Rect::from_min_size(card.min + vec2(f.x, f.y) * s, vec2(f.w, f.h) * s);
            match &f.kind {
                Kind::Text { size, color, background, .. } => {
                    if let Some(bg) = background {
                        pc.rect_filled(fr, CornerRadius::ZERO, Color32::from_rgb(bg[0], bg[1], bg[2]));
                    }
                    let lh = (size * s * 1.3).max(1.5);
                    let mut y = fr.top() + lh * 0.5;
                    let c = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 150);
                    while y < fr.bottom() {
                        pc.line_segment([pos2(fr.left(), y), pos2(fr.right() - (y * 7.0) % (fr.width() * 0.3).max(1.0), y)], Stroke::new((size * s * 0.55).max(0.6), c));
                        y += lh;
                    }
                }
                Kind::Image { .. } => {
                    pc.rect_filled(fr, CornerRadius::ZERO, Color32::from_rgb(180, 196, 214));
                }
                Kind::Rect { fill, .. } | Kind::Ellipse { fill } | Kind::Polygon { fill, .. } => {
                    pc.rect_filled(fr, CornerRadius::ZERO, Color32::from_rgb(fill[0], fill[1], fill[2]));
                }
                Kind::Line { color, .. } => {
                    pc.line_segment([fr.left_center(), fr.right_center()], Stroke::new(1.0, Color32::from_rgb(color[0], color[1], color[2])));
                }
                Kind::Table { .. } => {
                    pc.rect_stroke(fr, CornerRadius::ZERO, Stroke::new(0.8, Color32::from_gray(150)), StrokeKind::Inside);
                }
            }
        }
        if selected {
            p.rect_stroke(card.expand(3.0), CornerRadius::same(4), Stroke::new(2.0, BLUE), StrokeKind::Outside);
        } else if resp.hovered() {
            p.rect_stroke(card.expand(3.0), CornerRadius::same(4), Stroke::new(1.0, MUTED), StrokeKind::Outside);
        }
        p.text(pos2(r.center().x, r.max.y - 9.0), Align2::CENTER_CENTER, label, FontId::proportional(11.5), if selected { INK } else { MUTED });
        resp
    }

    fn pages_panel(&mut self, root: &mut Ui) {
        egui::Panel::left("pages").resizable(false).exact_size(170.0).frame(EFrame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 12))).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("PAGES").font(semibold(11.0)).color(FAINT));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("+").on_hover_text("Ajouter une page à la fin").clicked() {
                        let at = self.doc.pages.len();
                        self.add_page(at, false);
                    }
                });
            });
            ui.add_space(6.0);
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let master = self.doc.master.frames.clone();
                if self.thumb(ui, None, &master, self.page == MASTER, "Gabarit").on_hover_text("Ses blocs apparaissent sur toutes les pages").clicked() {
                    self.page = MASTER;
                    self.sel = None;
                }
                ui.add_space(6.0);
                let mut act: Option<(usize, u8)> = None;
                for i in 0..self.doc.pages.len() {
                    let pg = self.doc.pages[i].clone();
                    let label = format!("{}{}", self.doc.page_label(i), if pg.no_master { " · sans gabarit" } else { "" });
                    let r = self.thumb(ui, Some(&pg), &pg.frames, self.page == i, &label);
                    if r.clicked() {
                        self.page = i;
                        self.sel = None;
                    }
                    r.context_menu(|ui| {
                        for (k, l) in ["Insérer une page avant", "Insérer une page après", "Dupliquer", "Monter", "Descendre", "Utiliser / retirer le gabarit", "Supprimer"].iter().enumerate() {
                            if ui.button(*l).clicked() {
                                act = Some((i, k as u8));
                                ui.close();
                            }
                        }
                    });
                }
                if let Some((i, k)) = act {
                    self.page = i;
                    match k {
                        0 => self.add_page(i, false),
                        1 => self.add_page(i + 1, false),
                        2 => self.add_page(i + 1, true),
                        3 => self.move_page(-1),
                        4 => self.move_page(1),
                        5 => self.doc.pages[i].no_master = !self.doc.pages[i].no_master,
                        _ => self.delete_page(i),
                    }
                }
                ui.add_space(4.0);
                if pro::secondary(ui, "+ Page").clicked() {
                    let at = self.doc.pages.len();
                    self.add_page(at, false);
                }
            });
        });
    }

    fn props(&mut self, root: &mut Ui) {
        egui::Panel::right("props").resizable(false).exact_size(304.0).frame(EFrame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                match self.sel.filter(|i| *i < self.frames_ref().len()) {
                    Some(i) => self.frame_panels(ui, i),
                    None => self.doc_panel(ui),
                }
                self.objects_panel(ui);
            });
        });
    }

    fn frame_panels(&mut self, ui: &mut Ui, i: usize) {
        let over = self.overflow.get(i).copied().unwrap_or(false);
        let swatches = self.doc.swatches.clone();
        let user_styles: Vec<String> = self.doc.styles.iter().map(|s| s.name.clone()).collect();
        let texts: Vec<(usize, String)> = self.frames_ref().iter().enumerate().filter(|(k, f)| *k != i && matches!(f.kind, Kind::Text { .. })).map(|(k, f)| (k, f.title())).collect();
        let (mut fit, mut order, mut align, mut style, mut save_style, mut fit_img, mut edit, mut add_swatch) = (false, None, None, None, false, false, false, None);
        let f = &mut self.frames()[i];
        pro::dock_panel(ui, "pos", "Bloc", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.add(egui::TextEdit::singleline(&mut f.name).hint_text(f.kind.label()).desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                ui.label(RichText::new("X").color(MUTED));
                ui.add(egui::DragValue::new(&mut f.x).max_decimals(1).suffix(" pt"));
                ui.label(RichText::new("Y").color(MUTED));
                ui.add(egui::DragValue::new(&mut f.y).max_decimals(1).suffix(" pt"));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("L").color(MUTED));
                ui.add(egui::DragValue::new(&mut f.w).range(1.0..=5000.0).max_decimals(1).suffix(" pt"));
                ui.label(RichText::new("H").color(MUTED));
                ui.add(egui::DragValue::new(&mut f.h).range(1.0..=5000.0).max_decimals(1).suffix(" pt"));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Rotation").color(MUTED));
                ui.add(egui::DragValue::new(&mut f.rotation).range(-180.0..=180.0).speed(0.5).suffix("°"));
                for a in [-90.0, 0.0, 90.0] {
                    if ui.small_button(format!("{a:.0}°")).clicked() {
                        f.rotation = a;
                    }
                }
            });
        });
        match &mut f.kind {
            Kind::Text { text, size, family, bold, color, align: al, leading, italic, tracking, columns, gutter, background, padding, underline, upper, list, para_space, indent, valign, link } => {
                pro::dock_panel(ui, "text", "Texte", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    egui::ScrollArea::vertical().id_salt("textbox").max_height(170.0).show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(text).desired_rows(6).desired_width(f32::INFINITY));
                    });
                    if over {
                        ui.label(RichText::new("Le texte dépasse du bloc.").size(12.0).color(RED));
                        ui.horizontal(|ui| {
                            if pro::secondary(ui, "Ajuster la hauteur").clicked() {
                                fit = true;
                            }
                        });
                    }
                    if pro::ghost(ui, "Écrire sur la page (Entrée)").clicked() {
                        edit = true;
                    }
                });
                pro::dock_panel(ui, "char", "Caractère", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    egui::ComboBox::from_id_salt("font").selected_text(family.as_str()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        for name in FONTS {
                            ui.selectable_value(family, name.to_string(), name);
                        }
                    });
                    ui.label(RichText::new("Taille").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(size, 4.0..=200.0).logarithmic(true).suffix(" pt"));
                    ui.label(RichText::new("Approche").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(tracking, -2.0..=20.0).suffix(" pt"));
                    ui.horizontal(|ui| {
                        ui.toggle_value(bold, RichText::new(" G ").strong());
                        ui.toggle_value(italic, RichText::new(" I ").italics());
                        ui.toggle_value(underline, RichText::new(" S ").underline());
                        ui.toggle_value(upper, RichText::new(" AA "));
                    });
                    color_row(ui, "Couleur", color, &swatches);
                });
                pro::dock_panel(ui, "para", "Paragraphe", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    let mut a = *al as usize;
                    if pro::segmented(ui, &mut a, &["Gauche", "Centre", "Droite", "Justifié"]) {
                        *al = a as u8;
                    }
                    ui.label(RichText::new("Interligne").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(leading, 0.8..=2.5));
                    ui.label(RichText::new("Espace après paragraphe").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(para_space, 0.0..=40.0).suffix(" pt"));
                    ui.label(RichText::new("Retrait de première ligne").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(indent, 0.0..=60.0).suffix(" pt"));
                    ui.label(RichText::new("Liste").size(12.5).color(MUTED));
                    let mut l = *list as usize;
                    if pro::segmented(ui, &mut l, &LISTS) {
                        *list = l as u8;
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Style").size(12.5).color(MUTED));
                        egui::ComboBox::from_id_salt("pstyle").selected_text("Appliquer…").width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                            for (k, (n, ..)) in STYLES.iter().enumerate() {
                                if ui.button(*n).clicked() {
                                    style = Some(k);
                                }
                            }
                            for (k, n) in user_styles.iter().enumerate() {
                                if ui.button(n).clicked() {
                                    style = Some(STYLES.len() + k);
                                }
                            }
                        });
                    });
                    if pro::ghost(ui, "+ Créer un style à partir de ce texte").clicked() {
                        save_style = true;
                    }
                });
                pro::dock_panel(ui, "tframe", "Bloc de texte", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Colonnes").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(columns).range(1..=6));
                        ui.label(RichText::new("Gouttière").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(gutter).range(0.0..=80.0).suffix(" pt"));
                    });
                    ui.label(RichText::new("Alignement vertical").size(12.5).color(MUTED));
                    let mut v = *valign as usize;
                    if pro::segmented(ui, &mut v, &["Haut", "Milieu", "Bas"]) {
                        *valign = v as u8;
                    }
                    ui.label(RichText::new("Marge intérieure").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(padding, 0.0..=60.0).suffix(" pt"));
                    let mut on = background.is_some();
                    if ui.checkbox(&mut on, "Fond coloré").changed() {
                        *background = on.then_some([245, 245, 247, 255]);
                    }
                    if let Some(bg) = background {
                        color_row(ui, "Fond", bg, &swatches);
                    }
                    ui.label(RichText::new("Suite du texte dans").size(12.5).color(MUTED));
                    let cur = link.and_then(|l| texts.iter().find(|t| t.0 == l)).map_or("Aucun bloc".to_string(), |t| t.1.clone());
                    egui::ComboBox::from_id_salt("link").selected_text(cur).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        ui.selectable_value(link, None, "Aucun bloc");
                        for (k, n) in &texts {
                            ui.selectable_value(link, Some(*k), n);
                        }
                    });
                    ui.label(RichText::new("Le texte qui ne tient pas continue dans ce bloc (texte chaîné).").size(11.5).color(FAINT));
                });
            }
            Kind::Image { path, cover, radius, filter, flip } => {
                pro::dock_panel(ui, "img", "Image", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    ui.label(RichText::new(if path.is_empty() { "Aucune image" } else { path.as_str() }).size(11.5).color(FAINT));
                    ui.horizontal(|ui| {
                        if pro::secondary(ui, "Choisir…").clicked()
                            && let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif", "svg"]).pick_file()
                        {
                            *path = p.to_string_lossy().into_owned();
                        }
                        if pro::ghost(ui, "Cadre aux proportions").clicked() {
                            fit_img = true;
                        }
                    });
                    let mut c = if *cover { 0 } else { 1 };
                    if pro::segmented(ui, &mut c, &["Remplir le cadre", "Ajuster dans le cadre"]) {
                        *cover = c == 0;
                    }
                    ui.label(RichText::new("Filtre").size(12.5).color(MUTED));
                    let mut fl = *filter as usize;
                    if pro::segmented(ui, &mut fl, &FILTERS) {
                        *filter = fl as u8;
                    }
                    ui.label(RichText::new("Arrondi des coins").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(radius, 0.0..=300.0).suffix(" pt"));
                    ui.checkbox(flip, "Miroir horizontal");
                });
            }
            Kind::Rect { fill, radius } => {
                pro::dock_panel(ui, "shape", "Forme", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    color_row(ui, "Fond", fill, &swatches);
                    ui.label(RichText::new("Arrondi").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(radius, 0.0..=300.0).suffix(" pt"));
                    gradient_row(ui, &mut f.fill2, &mut f.grad_angle, &swatches);
                });
            }
            Kind::Ellipse { fill } => {
                pro::dock_panel(ui, "shape", "Forme", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    color_row(ui, "Fond", fill, &swatches);
                    gradient_row(ui, &mut f.fill2, &mut f.grad_angle, &swatches);
                });
            }
            Kind::Polygon { sides, inner, fill } => {
                pro::dock_panel(ui, "shape", "Forme", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    color_row(ui, "Fond", fill, &swatches);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Côtés / branches").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(sides).range(3..=48));
                    });
                    ui.label(RichText::new("Creux (1 = polygone)").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(inner, 0.1..=1.0));
                    gradient_row(ui, &mut f.fill2, &mut f.grad_angle, &swatches);
                });
            }
            Kind::Line { color, width, arrow } => {
                pro::dock_panel(ui, "shape", "Ligne", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    color_row(ui, "Couleur", color, &swatches);
                    ui.label(RichText::new("Épaisseur").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(width, 0.25..=30.0).suffix(" pt"));
                    ui.label(RichText::new("Pointillés (0 = plein)").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(&mut f.dash, 0.0..=40.0).suffix(" pt"));
                    ui.checkbox(arrow, "Pointe de flèche");
                });
            }
            Kind::Table { cells, size, family, color, line, header, stripe } => {
                pro::dock_panel(ui, "table", "Tableau", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let cols = cells.iter().map(|r| r.len()).max().unwrap_or(1);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{} lignes", cells.len())).size(12.5).color(MUTED));
                        if ui.small_button("−").clicked() && cells.len() > 1 {
                            cells.pop();
                        }
                        if ui.small_button("+").clicked() {
                            cells.push(vec![String::new(); cols]);
                        }
                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("{cols} colonnes")).size(12.5).color(MUTED));
                        if ui.small_button("−").clicked() && cols > 1 {
                            cells.iter_mut().for_each(|r| {
                                r.truncate(cols - 1);
                            });
                        }
                        if ui.small_button("+").clicked() {
                            cells.iter_mut().for_each(|r| r.resize(cols + 1, String::new()));
                        }
                    });
                    let cw = (ui.available_width() - 4.0 * cols as f32) / cols.max(1) as f32;
                    egui::Grid::new("cells").spacing(vec2(4.0, 4.0)).show(ui, |ui| {
                        for row in cells.iter_mut() {
                            row.resize(cols, String::new());
                            for c in row.iter_mut() {
                                ui.add(egui::TextEdit::singleline(c).desired_width(cw - 8.0));
                            }
                            ui.end_row();
                        }
                    });
                    egui::ComboBox::from_id_salt("tfont").selected_text(family.as_str()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        for name in FONTS {
                            ui.selectable_value(family, name.to_string(), name);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Taille").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(size).range(4.0..=72.0).speed(0.2).suffix(" pt"));
                    });
                    color_row(ui, "Texte", color, &swatches);
                    color_row(ui, "Traits", line, &swatches);
                    let mut on = header.is_some();
                    if ui.checkbox(&mut on, "Ligne d'en-tête").changed() {
                        *header = on.then_some([30, 30, 40, 255]);
                    }
                    if let Some(h) = header {
                        color_row(ui, "Fond de l'en-tête", h, &swatches);
                    }
                    let mut on = stripe.is_some();
                    if ui.checkbox(&mut on, "Lignes alternées").changed() {
                        *stripe = on.then_some([245, 245, 248, 255]);
                    }
                    if let Some(s) = stripe {
                        color_row(ui, "Fond alterné", s, &swatches);
                    }
                });
            }
        }
        pro::dock_panel(ui, "look", "Aspect", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            ui.label(RichText::new("Opacité").size(12.5).color(MUTED));
            let mut op = f.opacity * 100.0;
            if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).suffix(" %")).changed() {
                f.opacity = op / 100.0;
            }
            if !matches!(f.kind, Kind::Line { .. }) {
                let mut on = f.stroke.is_some();
                if ui.checkbox(&mut on, "Contour").changed() {
                    f.stroke = on.then_some([20, 20, 24, 255]);
                }
                if let Some(c) = &mut f.stroke {
                    color_row(ui, "Couleur du contour", c, &swatches);
                    ui.add(egui::Slider::new(&mut f.stroke_w, 0.25..=20.0).suffix(" pt"));
                    ui.label(RichText::new("Pointillés").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(&mut f.dash, 0.0..=40.0).suffix(" pt"));
                }
            }
            ui.checkbox(&mut f.shadow, "Ombre portée");
            ui.horizontal(|ui| {
                ui.checkbox(&mut f.locked, "Verrouillé");
                ui.checkbox(&mut f.hidden, "Masqué");
            });
            if let Kind::Rect { fill, .. } | Kind::Ellipse { fill } | Kind::Polygon { fill, .. } = &f.kind
                && pro::ghost(ui, "+ Ajouter la couleur au nuancier").clicked()
            {
                add_swatch = Some(*fill);
            }
        });
        pro::dock_panel(ui, "arrange", "Disposition", |ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            let w = (ui.available_width() - 8.0) / 3.0;
            ui.label(RichText::new("Aligner sur les marges").size(12.5).color(MUTED));
            for row in [[("Gauche", 0, 0.0), ("Centre ↔", 0, 0.5), ("Droite", 0, 1.0)], [("Haut", 1, 0.0), ("Milieu ↕", 1, 0.5), ("Bas", 1, 1.0)]] {
                ui.horizontal(|ui| {
                    for (label, axis, to) in row {
                        if ui.add_sized([w, 26.0], egui::Button::new(RichText::new(label).size(12.0))).clicked() {
                            align = Some((axis, to));
                        }
                    }
                });
            }
            ui.add_space(4.0);
            ui.label(RichText::new("Ordre").size(12.5).color(MUTED));
            let w4 = (ui.available_width() - 12.0) / 4.0;
            ui.horizontal(|ui| {
                for (label, to) in [("Devant", i32::MAX), ("Avancer", 1), ("Reculer", -1), ("Derrière", i32::MIN)] {
                    if ui.add_sized([w4, 26.0], egui::Button::new(RichText::new(label).size(11.5))).clicked() {
                        order = Some(to);
                    }
                }
            });
        });
        if fit {
            self.fit_text();
        }
        if fit_img {
            self.fit_frame_to_image();
        }
        if edit {
            self.edit_text = Some(i);
        }
        if let Some(to) = order {
            self.order(to);
        }
        if let Some((axis, to)) = align {
            self.align(axis, to, false);
        }
        if let Some(k) = style {
            self.apply_style(k);
        }
        if save_style {
            self.style_name = Some(format!("Style {}", self.doc.styles.len() + 1));
        }
        if let Some(c) = add_swatch
            && !self.doc.swatches.contains(&c)
        {
            self.doc.swatches.push(c);
        }
    }

    fn doc_panel(&mut self, ui: &mut Ui) {
        let page = self.page;
        pro::dock_panel(ui, "doc", "Document", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{:.0} × {:.0} mm", self.doc.width / MM, self.doc.height / MM)).size(12.5).color(INK));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::secondary(ui, "Format…").clicked() {
                        self.doc_sheet = true;
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Titre").size(12.5).color(MUTED));
                ui.add(egui::TextEdit::singleline(&mut self.doc.title).hint_text("{title}").desired_width(ui.available_width() - 16.0));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Marges").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.margin).range(0.0..=300.0).suffix(" pt"));
                ui.label(RichText::new("Colonnes").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.grid).range(1..=12));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Gouttière").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.grid_gutter).range(0.0..=80.0).suffix(" pt"));
                ui.label(RichText::new("Grille").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.baseline).range(0.0..=60.0).suffix(" pt")).on_hover_text("Grille de base (0 = masquée)");
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Fond perdu").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.bleed).range(0.0..=36.0).suffix(" pt")).on_hover_text("3 mm = 8,5 pt pour un imprimeur");
                ui.label(RichText::new("1re page n°").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.first_number).range(1..=9999));
            });
            let sw = self.doc.swatches.clone();
            color_row(ui, "Papier", &mut self.doc.paper, &sw);
            if page != MASTER {
                let mut use_master = !self.doc.pages[page].no_master;
                if ui.checkbox(&mut use_master, "Utiliser le gabarit sur cette page").changed() {
                    self.doc.pages[page].no_master = !use_master;
                }
            } else {
                ui.label(RichText::new("Tu modifies le gabarit : ses blocs apparaissent sur toutes les pages.").size(12.0).color(FAINT));
            }
            if pro::secondary(ui, "?  Tutoriels").clicked() {
                self.help.learn("Texte");
            }
        });
        pro::dock_panel(ui, "swatches", "Nuancier", |ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            if self.doc.swatches.is_empty() {
                ui.label(RichText::new("Les couleurs du document, réutilisables partout d'un clic.").size(12.0).color(FAINT));
            }
            let mut remove = None;
            ui.horizontal_wrapped(|ui| {
                for (k, c) in self.doc.swatches.iter_mut().enumerate() {
                    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                    let r = ui.color_edit_button_srgba(&mut col);
                    *c = col.to_srgba_unmultiplied();
                    r.context_menu(|ui| {
                        if ui.button("Supprimer cette couleur").clicked() {
                            remove = Some(k);
                            ui.close();
                        }
                    });
                }
                if ui.button("+").on_hover_text("Ajouter une couleur").clicked() {
                    self.doc.swatches.push([10, 132, 255, 255]);
                }
            });
            if let Some(k) = remove {
                self.doc.swatches.remove(k);
            }
        });
        pro::dock_panel(ui, "styles", "Styles de paragraphe", |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            if self.doc.styles.is_empty() {
                ui.label(RichText::new("Sélectionne un texte puis « Créer un style » pour réutiliser sa mise en forme.").size(12.0).color(FAINT));
            }
            let mut remove = None;
            for (k, s) in self.doc.styles.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&s.name).size(12.5).color(INK));
                    ui.label(RichText::new(format!("{} {:.1} pt", s.family, s.size)).size(11.0).color(FAINT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("×").clicked() {
                            remove = Some(k);
                        }
                    });
                });
            }
            if let Some(k) = remove {
                self.doc.styles.remove(k);
            }
        });
        pro::dock_panel(ui, "guides", "Repères", |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            if self.doc.guides.is_empty() {
                ui.label(RichText::new("Glisse depuis une règle pour poser un repère ; ramène-le sur la règle pour le retirer.").size(12.0).color(FAINT));
            }
            let mut remove = None;
            for (k, g) in self.doc.guides.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if g.vertical { "Vertical" } else { "Horizontal" }).size(12.0).color(MUTED));
                    ui.add(egui::DragValue::new(&mut g.pos).speed(0.5).suffix(" pt"));
                    if ui.small_button("×").clicked() {
                        remove = Some(k);
                    }
                });
            }
            if let Some(k) = remove {
                self.doc.guides.remove(k);
            }
            if pro::ghost(ui, "+ Repère vertical au centre").clicked() {
                self.doc.guides.push(Guide { vertical: true, pos: self.doc.width / 2.0 });
            }
            if pro::ghost(ui, "+ Repère horizontal au centre").clicked() {
                self.doc.guides.push(Guide { vertical: false, pos: self.doc.height / 2.0 });
            }
        });
    }

    fn objects_panel(&mut self, ui: &mut Ui) {
        let mut pick = None;
        let mut toggles: Vec<(usize, u8)> = Vec::new();
        let sel = self.sel;
        let items: Vec<(usize, String, bool, bool, u8)> = self
            .frames_ref()
            .iter()
            .enumerate()
            .rev()
            .map(|(i, f)| {
                let k = match f.kind {
                    Kind::Text { .. } => 0,
                    Kind::Image { .. } => 1,
                    Kind::Rect { .. } => 2,
                    Kind::Ellipse { .. } => 3,
                    Kind::Line { .. } => 4,
                    Kind::Polygon { .. } => 5,
                    Kind::Table { .. } => 6,
                };
                (i, f.title(), f.hidden, f.locked, k)
            })
            .collect();
        pro::dock_panel(ui, "objects", "Objets de la page", |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            if items.is_empty() {
                ui.label(RichText::new("Aucun bloc sur cette page.").size(12.0).color(FAINT));
            }
            for (i, name, hidden, locked, k) in items {
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
                let p = ui.painter();
                if sel == Some(i) {
                    p.rect_filled(r, CornerRadius::same(6), BLUE.gamma_multiply(0.28));
                } else if resp.hovered() {
                    p.rect_filled(r, CornerRadius::same(6), Color32::from_white_alpha(8));
                }
                let t = [Tool::Text, Tool::Image, Tool::Rect, Tool::Ellipse, Tool::Line, Tool::Polygon, Tool::Table][k as usize];
                tool_icon(p, Rect::from_center_size(r.left_center() + vec2(14.0, 0.0), Vec2::splat(12.0)), t, MUTED);
                p.text(r.left_center() + vec2(28.0, 0.0), Align2::LEFT_CENTER, name, FontId::proportional(12.5), if hidden { FAINT } else { INK });
                for (n, (label, on)) in [("👁", !hidden), ("🔒", locked)].into_iter().enumerate() {
                    let br = Rect::from_center_size(r.right_center() - vec2(14.0 + n as f32 * 22.0, 0.0), Vec2::splat(20.0));
                    let b = ui.interact(br, ui.make_persistent_id(("obj", i, n)), Sense::click());
                    let shown = if n == 0 { true } else { on || b.hovered() };
                    if shown {
                        ui.painter().text(br.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), if (n == 0 && on) || (n == 1 && on) { INK } else { FAINT });
                    }
                    if b.clicked() {
                        toggles.push((i, n as u8));
                    }
                }
                if resp.clicked() {
                    pick = Some(i);
                }
            }
        });
        for (i, n) in toggles {
            let f = &mut self.frames()[i];
            if n == 0 {
                f.hidden = !f.hidden;
            } else {
                f.locked = !f.locked;
            }
        }
        if let Some(i) = pick {
            self.sel = Some(i);
        }
    }

    fn snap_value(&self, v: f32, size: f32, axis: usize, tol: f32, skip: Option<usize>) -> (f32, Option<f32>) {
        if !self.snap {
            return (v, None);
        }
        let (m, span) = (self.doc.margin, if axis == 0 { self.doc.width } else { self.doc.height });
        let mut lines = vec![0.0, m, span / 2.0, span - m, span];
        lines.extend(self.doc.guides.iter().filter(|g| g.vertical == (axis == 0)).map(|g| g.pos));
        let g = self.doc.grid.max(1);
        if axis == 0 && g > 1 {
            let cw = (span - 2.0 * m - self.doc.grid_gutter * (g as f32 - 1.0)) / g as f32;
            for k in 0..g {
                let x = m + k as f32 * (cw + self.doc.grid_gutter);
                lines.push(x);
                lines.push(x + cw);
            }
        }
        for (i, f) in self.frames_ref().iter().enumerate() {
            if Some(i) == skip || f.hidden {
                continue;
            }
            let (a, s) = if axis == 0 { (f.x, f.w) } else { (f.y, f.h) };
            lines.extend([a, a + s / 2.0, a + s]);
        }
        let mut best: Option<(f32, f32, f32)> = None;
        for (edge, off) in [(v, 0.0), (v + size / 2.0, size / 2.0), (v + size, size)] {
            for l in &lines {
                let d = (edge - l).abs();
                if d <= tol && best.is_none_or(|b| d < b.0) {
                    best = Some((d, l - off, *l));
                }
            }
        }
        match best {
            Some((_, nv, line)) => (nv, Some(line)),
            None => (v, None),
        }
    }

    fn canvas(&mut self, root: &mut Ui, name: &str) {
        egui::CentralPanel::default().frame(EFrame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let tab_bar = Rect::from_min_max(full.min, pos2(full.right(), full.top() + 32.0));
            ui.painter().rect_filled(tab_bar, CornerRadius::ZERO, PANEL);
            let where_ = if self.page == MASTER { "Gabarit".to_string() } else { format!("page {}", self.doc.page_label(self.page)) };
            let label = format!("{name} · {where_} @ {:.0} %{}", self.zoom * 100.0, if self.doc != self.saved { " •" } else { "" });
            let g = ui.painter().layout_no_wrap(label, FontId::proportional(12.5), INK);
            let tab = Rect::from_min_size(tab_bar.min + vec2(8.0, 4.0), vec2(g.size().x + 28.0, 28.0));
            ui.painter().rect_filled(tab, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, PASTEBOARD);
            ui.painter().galley(tab.left_center() + vec2(14.0, -g.size().y / 2.0), g, INK);
            let rw = if self.rulers { 18.0 } else { 0.0 };
            let area = Rect::from_min_max(pos2(full.left() + rw, tab_bar.bottom() + rw), full.max);
            if self.fit_pending {
                self.zoom = ((area.height() - 80.0) / self.doc.height).min((area.width() - 80.0) / self.doc.width).clamp(0.1, 4.0);
                self.fit_pending = false;
            }
            let (scroll, ctrl) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.command));
            if ctrl && scroll != 0.0 && ui.rect_contains_pointer(area) {
                self.zoom = (self.zoom * (scroll / 400.0).exp()).clamp(0.1, 6.0);
            }
            let px = ui.ctx().pixels_per_point();
            let size = vec2(self.doc.width, self.doc.height) * self.zoom;
            let show_guides = self.guides && !self.preview;
            let page_rect = ui
                .scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                        let avail = ui.available_size();
                        let pad = 60.0 + self.doc.bleed * self.zoom;
                        let (outer, _) = ui.allocate_exact_size(vec2(avail.x.max(size.x + 2.0 * pad), (size.y + 2.0 * pad).max(avail.y)), Sense::hover());
                        let page_rect = Rect::from_min_size(pos2(outer.center().x - size.x / 2.0, outer.min.y + pad), size);
                        let scale = self.zoom * px;
                        if self.rendered.as_ref().is_none_or(|(d, p, s, gd)| *d != self.doc || *p != self.page || (*s - scale).abs() > 1e-3 || *gd != show_guides) {
                            let (svg, over) = self.doc.to_svg(self.page, &self.measure(), show_guides);
                            self.overflow = over;
                            match usvg::Tree::from_str(&svg, &self.opts()) {
                                Ok(tree) => {
                                    if let Some(mut pix) = tiny_skia::Pixmap::new((self.doc.width * scale).ceil() as u32, (self.doc.height * scale).ceil() as u32) {
                                        resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
                                        let img = egui::ColorImage::from_rgba_premultiplied([pix.width() as usize, pix.height() as usize], pix.data());
                                        match &mut self.tex {
                                            Some(t) => t.set(img, TextureOptions::LINEAR),
                                            None => self.tex = Some(ui.ctx().load_texture("page", img, TextureOptions::LINEAR)),
                                        }
                                    }
                                }
                                Err(e) => self.notice = Some((format!("Rendu : {e}"), false)),
                            }
                            self.rendered = Some((self.doc.clone(), self.page, scale, show_guides));
                        }
                        let painter = ui.painter();
                        let z = self.zoom;
                        let to_doc = |p: Pos2| [(p.x - page_rect.min.x) / z, (p.y - page_rect.min.y) / z];
                        let to_scr = |p: [f32; 2]| pos2(page_rect.min.x + p[0] * z, page_rect.min.y + p[1] * z);
                        painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(120) }.as_shape(page_rect, CornerRadius::ZERO));
                        if let Some(t) = &self.tex {
                            painter.image(t.id(), page_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                        }
                        if !self.preview {
                            let b = self.doc.bleed;
                            if b > 0.0 && show_guides {
                                painter.rect_stroke(Rect::from_min_max(to_scr([-b, -b]), to_scr([self.doc.width + b, self.doc.height + b])), CornerRadius::ZERO, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 59, 48, 140)), StrokeKind::Middle);
                            }
                            for gd in &self.doc.guides {
                                let (a, c) = if gd.vertical { (to_scr([gd.pos, -40.0]), to_scr([gd.pos, self.doc.height + 40.0])) } else { (to_scr([-40.0, gd.pos]), to_scr([self.doc.width + 40.0, gd.pos])) };
                                painter.line_segment([a, c], Stroke::new(1.0, Color32::from_rgb(0, 200, 255)));
                            }
                            for (vertical, pos) in &self.snap_lines {
                                let (a, c) = if *vertical { (to_scr([*pos, -20.0]), to_scr([*pos, self.doc.height + 20.0])) } else { (to_scr([-20.0, *pos]), to_scr([self.doc.width + 20.0, *pos])) };
                                painter.line_segment([a, c], Stroke::new(1.0, Color32::from_rgb(255, 47, 160)));
                            }
                            for (i, f) in self.frames_ref().iter().enumerate() {
                                let r = Rect::from_min_max(to_scr([f.x, f.y]), to_scr([f.x + f.w, f.y + f.h]));
                                if self.sel == Some(i) {
                                    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.5, BLUE), StrokeKind::Middle);
                                    for (hx, hy) in HANDLES {
                                        let c = to_scr([f.x + f.w * hx, f.y + f.h * hy]);
                                        painter.rect(Rect::from_center_size(c, Vec2::splat(8.0)), CornerRadius::same(2), Color32::WHITE, Stroke::new(1.2, BLUE), StrokeKind::Inside);
                                    }
                                    if let Kind::Text { link: Some(l), .. } = &f.kind
                                        && let Some(t) = self.frames_ref().get(*l)
                                    {
                                        let a = r.right_bottom();
                                        let b = to_scr([t.x, t.y]);
                                        painter.line_segment([a, b], Stroke::new(1.2, Color32::from_rgb(0, 200, 255)));
                                        painter.circle_filled(b, 3.5, Color32::from_rgb(0, 200, 255));
                                    }
                                } else if show_guides && !f.hidden {
                                    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(0.5, Color32::from_rgba_unmultiplied(80, 140, 255, 90)), StrokeKind::Middle);
                                }
                                if f.locked {
                                    painter.text(r.right_top() + vec2(-6.0, 8.0), Align2::RIGHT_CENTER, "🔒", FontId::proportional(10.0), FAINT);
                                }
                                if self.overflow.get(i).copied().unwrap_or(false) {
                                    let q = Rect::from_center_size(r.right_bottom() - vec2(6.0, 6.0), Vec2::splat(11.0));
                                    painter.rect_filled(q, CornerRadius::ZERO, RED);
                                    painter.text(q.center(), Align2::CENTER_CENTER, "+", FontId::monospace(10.0), Color32::WHITE);
                                }
                            }
                        }
                        let resp = ui.interact(outer, ui.scope_id().with("page"), Sense::click_and_drag());
                        let tol = 6.0 / z;
                        if let Some(sp) = resp.interact_pointer_pos() {
                            let p = to_doc(sp);
                            if resp.double_clicked() {
                                if let Some(i) = self.sel
                                    && matches!(self.frames_ref()[i].kind, Kind::Text { .. })
                                    && self.frames_ref()[i].contains(p, tol)
                                {
                                    self.edit_text = Some(i);
                                }
                            } else if resp.drag_started() || (resp.clicked() && matches!(self.drag, Drag::None)) {
                                let shift = ui.input(|i| i.modifiers.shift);
                                self.press(p, tol, shift);
                            }
                            if resp.dragged() {
                                let shift = ui.input(|i| i.modifiers.shift);
                                self.drag_to(p, tol, shift);
                            }
                        }
                        if resp.drag_stopped() || resp.clicked() {
                            if let (Drag::Create { .. }, Some(i)) = (&self.drag, self.sel)
                                && self.frames()[i].w < 4.0
                            {
                                let kind = self.frames()[i].kind.clone();
                                let f = &mut self.frames()[i];
                                (f.w, f.h) = match kind {
                                    Kind::Text { .. } => (240.0, 80.0),
                                    Kind::Line { .. } => (200.0, 6.0),
                                    Kind::Table { .. } => (280.0, 110.0),
                                    _ => (160.0, 120.0),
                                };
                            }
                            if let Drag::Guide { index } = self.drag
                                && let Some(g) = self.doc.guides.get(index)
                            {
                                let span = if g.vertical { self.doc.width } else { self.doc.height };
                                if g.pos < -30.0 || g.pos > span + 30.0 {
                                    self.doc.guides.remove(index);
                                }
                            }
                            if matches!(self.drag, Drag::Create { .. }) {
                                if let Some(i) = self.sel
                                    && matches!(self.frames_ref()[i].kind, Kind::Text { .. })
                                {
                                    self.edit_text = Some(i);
                                }
                                self.tool = Tool::Select;
                            }
                            self.drag = Drag::None;
                            self.snap_lines.clear();
                        }
                        if resp.hovered()
                            && let Some(hp) = resp.hover_pos()
                        {
                            let p = to_doc(hp);
                            let cursor = match self.tool {
                                Tool::Select => self
                                    .sel
                                    .and_then(|i| self.frames_ref().get(i))
                                    .and_then(|f| HANDLES.iter().find(|(hx, hy)| (f.x + f.w * hx - p[0]).abs() <= tol && (f.y + f.h * hy - p[1]).abs() <= tol))
                                    .map(|(hx, hy)| match (*hx == 0.5, *hy == 0.5, (hx - 0.5) * (hy - 0.5) > 0.0) {
                                        (true, _, _) => egui::CursorIcon::ResizeVertical,
                                        (_, true, _) => egui::CursorIcon::ResizeHorizontal,
                                        (_, _, true) => egui::CursorIcon::ResizeNwSe,
                                        _ => egui::CursorIcon::ResizeNeSw,
                                    })
                                    .unwrap_or(egui::CursorIcon::Default),
                                Tool::Pipette => egui::CursorIcon::Copy,
                                _ => egui::CursorIcon::Crosshair,
                            };
                            ui.ctx().set_cursor_icon(cursor);
                        }
                        page_rect
                    })
                    .inner
                })
                .inner;
            if self.rulers {
                self.draw_rulers(ui, full, tab_bar, area, page_rect);
            }
        });
    }

    fn draw_rulers(&mut self, ui: &mut Ui, full: Rect, tab_bar: Rect, area: Rect, page_rect: Rect) {
        let z = self.zoom;
        let top = Rect::from_min_max(pos2(area.left(), tab_bar.bottom()), pos2(full.right(), area.top()));
        let left = Rect::from_min_max(pos2(full.left(), area.top()), pos2(area.left(), full.bottom()));
        let corner = Rect::from_min_max(pos2(full.left(), tab_bar.bottom()), pos2(area.left(), area.top()));
        let p = ui.painter();
        let bg = Color32::from_rgb(0x1d, 0x1d, 0x21);
        for r in [top, left, corner] {
            p.rect_filled(r, CornerRadius::ZERO, bg);
        }
        p.line_segment([top.left_bottom(), top.right_bottom()], Stroke::new(1.0, LINE));
        p.line_segment([left.right_top(), left.right_bottom()], Stroke::new(1.0, LINE));
        let step_mm = [1.0, 5.0, 10.0, 20.0, 50.0, 100.0].into_iter().find(|s| s * MM * z >= 40.0).unwrap_or(200.0);
        let sub = step_mm / 5.0;
        let pt = p.with_clip_rect(top);
        let mut k = (-(page_rect.left() - top.left()) / (sub * MM * z)).floor() as i64 - 1;
        loop {
            let mm = k as f32 * sub;
            let x = page_rect.left() + mm * MM * z;
            if x > top.right() {
                break;
            }
            let major = (mm / step_mm).fract().abs() < 1e-3;
            pt.line_segment([pos2(x, top.bottom() - if major { 9.0 } else { 4.0 }), pos2(x, top.bottom())], Stroke::new(1.0, FAINT));
            if major {
                pt.text(pos2(x + 2.0, top.top() + 1.0), Align2::LEFT_TOP, format!("{mm:.0}"), FontId::monospace(9.0), MUTED);
            }
            k += 1;
        }
        let pl = p.with_clip_rect(left);
        let mut k = (-(page_rect.top() - left.top()) / (sub * MM * z)).floor() as i64 - 1;
        loop {
            let mm = k as f32 * sub;
            let y = page_rect.top() + mm * MM * z;
            if y > left.bottom() {
                break;
            }
            let major = (mm / step_mm).fract().abs() < 1e-3;
            pl.line_segment([pos2(left.right() - if major { 9.0 } else { 4.0 }, y), pos2(left.right(), y)], Stroke::new(1.0, FAINT));
            if major {
                pl.text(pos2(left.left() + 2.0, y + 2.0), Align2::LEFT_TOP, format!("{mm:.0}"), FontId::monospace(9.0), MUTED);
            }
            k += 1;
        }
        p.text(corner.center(), Align2::CENTER_CENTER, "mm", FontId::proportional(8.0), FAINT);
        for (rect, vertical) in [(top, false), (left, true)] {
            let r = ui.interact(rect, ui.make_persistent_id(("ruler", vertical)), Sense::drag()).on_hover_text("Glisse vers la page pour poser un repère");
            if r.drag_started() {
                self.new_guide = Some(vertical);
            }
            if self.new_guide == Some(vertical)
                && let Some(pp) = ui.input(|i| i.pointer.hover_pos())
            {
                let line = if vertical { [pos2(pp.x, area.top()), pos2(pp.x, area.bottom())] } else { [pos2(area.left(), pp.y), pos2(area.right(), pp.y)] };
                ui.painter().line_segment(line, Stroke::new(1.0, Color32::from_rgb(0, 200, 255)));
                if !ui.input(|i| i.pointer.primary_down()) {
                    if area.contains(pp) {
                        let pos = if vertical { (pp.x - page_rect.left()) / z } else { (pp.y - page_rect.top()) / z };
                        self.doc.guides.push(Guide { vertical, pos: (pos * 2.0).round() / 2.0 });
                    }
                    self.new_guide = None;
                }
            }
        }
    }

    fn press(&mut self, p: [f32; 2], tol: f32, _shift: bool) {
        self.snap_lines.clear();
        match self.tool {
            Tool::Select | Tool::Pipette => {
                if self.tool == Tool::Select
                    && let Some(i) = self.sel
                    && let Some(f) = self.frames_ref().get(i)
                    && !f.locked
                    && let Some((hx, hy)) = HANDLES.iter().find(|(hx, hy)| (f.x + f.w * hx - p[0]).abs() <= tol && (f.y + f.h * hy - p[1]).abs() <= tol)
                {
                    self.drag = Drag::Resize { orig: [f.x, f.y, f.w, f.h], hx: *hx, hy: *hy };
                    return;
                }
                let hit = (0..self.frames_ref().len()).rev().find(|&i| !self.frames_ref()[i].locked && !self.frames_ref()[i].hidden && self.frames_ref()[i].contains(p, tol));
                if self.tool == Tool::Pipette {
                    match (self.sel, hit) {
                        (Some(to), Some(from)) if to != from => {
                            self.copy_style(from, to);
                            self.notice = Some(("Style copié".into(), true));
                        }
                        (None, Some(from)) => {
                            self.sel = Some(from);
                            self.notice = Some(("Bloc cible choisi : clique maintenant le bloc dont tu veux le style".into(), true));
                        }
                        _ => {}
                    }
                    return;
                }
                if hit.is_none()
                    && !self.preview
                    && let Some(g) = self.doc.guides.iter().position(|g| ((if g.vertical { p[0] } else { p[1] }) - g.pos).abs() <= tol)
                {
                    self.drag = Drag::Guide { index: g };
                    return;
                }
                if hit != self.sel {
                    self.edit_text = None;
                }
                self.sel = hit;
                if let Some(i) = self.sel {
                    let f = &self.frames_ref()[i];
                    self.drag = Drag::Move { last: p, acc: [0.0, 0.0], origin: [f.x, f.y] };
                }
            }
            tool => {
                let kind = match tool {
                    Tool::Text => Kind::text("", 12.0, false),
                    Tool::Image => Kind::Image { path: String::new(), cover: true, radius: 0.0, filter: 0, flip: false },
                    Tool::Ellipse => Kind::Ellipse { fill: [255, 120, 60, 255] },
                    Tool::Line => Kind::Line { color: [20, 20, 24, 255], width: 1.5, arrow: false },
                    Tool::Polygon => Kind::Polygon { sides: self.polygon_sides, inner: self.polygon_inner, fill: [255, 196, 0, 255] },
                    Tool::Table => Kind::table(4, 3),
                    _ => Kind::Rect { fill: [255, 120, 60, 255], radius: 0.0 },
                };
                let pick = matches!(kind, Kind::Image { .. });
                self.push_frame(Frame::new(p[0], p[1], 0.0, 0.0, kind));
                self.drag = Drag::Create { start: p };
                if pick
                    && let Some(path) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif", "svg"]).pick_file()
                    && let Some(i) = self.sel
                    && let Kind::Image { path: pp, .. } = &mut self.frames()[i].kind
                {
                    *pp = path.to_string_lossy().into_owned();
                }
            }
        }
    }

    fn drag_to(&mut self, p: [f32; 2], tol: f32, shift: bool) {
        if let Drag::Guide { index } = self.drag {
            if let Some(g) = self.doc.guides.get_mut(index) {
                g.pos = if g.vertical { p[0] } else { p[1] };
            }
            return;
        }
        let Some(i) = self.sel else { return };
        let (fw, fh) = (self.frames_ref()[i].w, self.frames_ref()[i].h);
        self.snap_lines.clear();
        let moved = match &mut self.drag {
            Drag::Move { last, acc, origin } => {
                acc[0] += p[0] - last[0];
                acc[1] += p[1] - last[1];
                *last = p;
                let mut t = [origin[0] + acc[0], origin[1] + acc[1]];
                if shift {
                    if acc[0].abs() > acc[1].abs() {
                        t[1] = origin[1];
                    } else {
                        t[0] = origin[0];
                    }
                }
                Some(t)
            }
            _ => None,
        };
        if let Some([x, y]) = moved {
            let (x, lx) = self.snap_value(x, fw, 0, tol, Some(i));
            let (y, ly) = self.snap_value(y, fh, 1, tol, Some(i));
            self.snap_lines.extend(lx.map(|v| (true, v)).into_iter().chain(ly.map(|v| (false, v))));
            let f = &mut self.frames()[i];
            f.x = x;
            f.y = y;
            return;
        }
        let (qx, lx) = self.snap_value(p[0], 0.0, 0, tol, Some(i));
        let (qy, ly) = self.snap_value(p[1], 0.0, 1, tol, Some(i));
        self.snap_lines.extend(lx.map(|v| (true, v)).into_iter().chain(ly.map(|v| (false, v))));
        let mut q = [qx, qy];
        match self.drag {
            Drag::Create { start } => {
                if shift {
                    let d = (q[0] - start[0]).abs().max((q[1] - start[1]).abs());
                    q = [start[0] + d * (q[0] - start[0]).signum(), start[1] + d * (q[1] - start[1]).signum()];
                }
                let f = &mut self.frames()[i];
                f.x = start[0].min(q[0]);
                f.y = start[1].min(q[1]);
                f.w = (q[0] - start[0]).abs();
                f.h = (q[1] - start[1]).abs();
            }
            Drag::Resize { orig, hx, hy } => {
                let (mut x0, mut y0, mut x1, mut y1) = (orig[0], orig[1], orig[0] + orig[2], orig[1] + orig[3]);
                if hx == 0.0 {
                    x0 = q[0];
                } else if hx == 1.0 {
                    x1 = q[0];
                }
                if hy == 0.0 {
                    y0 = q[1];
                } else if hy == 1.0 {
                    y1 = q[1];
                }
                if shift && hx != 0.5 && hy != 0.5 && orig[3] > 0.0 {
                    let ratio = orig[2] / orig[3];
                    let w = (x1 - x0).abs();
                    let h = w / ratio;
                    if hy == 0.0 {
                        y0 = y1 - h * (y1 - y0).signum();
                    } else {
                        y1 = y0 + h * (y1 - y0).signum();
                    }
                }
                let f = &mut self.frames()[i];
                f.x = x0.min(x1);
                f.y = y0.min(y1);
                f.w = (x1 - x0).abs().max(1.0);
                f.h = (y1 - y0).abs().max(1.0);
            }
            _ => {}
        }
    }

    fn text_editor(&mut self, ctx: &egui::Context) {
        let Some(i) = self.edit_text else { return };
        let page = self.page;
        let mut close = false;
        let mut fit = false;
        let fr = if page == MASTER { &mut self.doc.master.frames } else { &mut self.doc.pages[page].frames };
        let Some(f) = fr.get_mut(i) else {
            self.edit_text = None;
            return;
        };
        let Kind::Text { text, family, size, .. } = &mut f.kind else {
            self.edit_text = None;
            return;
        };
        let fam = family.clone();
        let sz = *size;
        egui::Window::new("Modifier le texte")
            .id(egui::Id::NULL.with("text-editor"))
            .collapsible(false)
            .resizable(true)
            .default_width(420.0)
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -48.0))
            .frame(EFrame::window(&ctx.global_style()).fill(CHROME).corner_radius(CornerRadius::same(14)).inner_margin(Margin::same(14)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{fam} · {sz:.1} pt")).size(12.0).color(MUTED));
                    ui.label(RichText::new("Le rendu se met à jour sur la page en direct").size(11.5).color(FAINT));
                });
                let r = ui.add(egui::TextEdit::multiline(text).desired_rows(8).desired_width(f32::INFINITY).font(FontId::proportional(14.0)));
                if !r.has_focus() && ui.memory(|m| m.focused().is_none()) {
                    r.request_focus();
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Échap pour fermer · {page} {pages} {title} {date}").size(11.5).color(FAINT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::primary(ui, "Terminé").clicked() {
                            close = true;
                        }
                        if pro::ghost(ui, "Ajuster la hauteur").clicked() {
                            fit = true;
                        }
                    });
                });
            });
        if fit {
            self.sel = Some(i);
            self.fit_text();
        }
        if close {
            self.edit_text = None;
        }
    }

    fn sheets(&mut self, ctx: &egui::Context) {
        if let Some(mut o) = self.export.take() {
            let mut go = false;
            let mut cancel = false;
            let n = self.doc.pages.len();
            let keep = pro::sheet(ctx, "export", 470.0, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(9), BLUE.gamma_multiply(0.25));
                    export_glyph(ui.painter(), r.shrink(9.0), BLUE);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Exporter le document").font(semibold(18.0)).color(INK_STRONG));
                        ui.label(RichText::new(["Vectoriel, toutes les pages dans un fichier, prêt à imprimer", "Image sans perte, transparence conservée", "Image légère pour le web et les messages", "Vectoriel modifiable dans Vector"][o.format]).size(12.0).color(MUTED));
                    });
                });
                ui.add_space(14.0);
                let row = |ui: &mut Ui, s: &str| {
                    ui.label(RichText::new(s).size(12.5).color(MUTED));
                };
                row(ui, "Format");
                pro::segmented(ui, &mut o.format, &FORMATS);
                ui.add_space(8.0);
                row(ui, "Pages");
                ui.horizontal(|ui| {
                    pro::segmented(ui, &mut o.pages, &["Toutes", "Page affichée", "Plage"]);
                    if o.pages == 2 {
                        ui.add(egui::DragValue::new(&mut o.from).range(1..=n).prefix("de "));
                        ui.add(egui::DragValue::new(&mut o.to).range(1..=n).prefix("à "));
                    }
                });
                ui.add_space(8.0);
                if o.format == 1 || o.format == 2 {
                    row(ui, "Résolution");
                    let labels: Vec<&str> = DPIS.iter().map(|d| d.0).collect();
                    pro::segmented(ui, &mut o.dpi, &labels);
                    let s = DPIS[o.dpi].1 / 72.0;
                    ui.label(RichText::new(format!("{:.0} × {:.0} px par page", self.doc.width * s, self.doc.height * s)).size(11.5).color(FAINT));
                    ui.add_space(8.0);
                }
                if o.format == 2 {
                    row(ui, "Qualité JPEG");
                    ui.add(egui::Slider::new(&mut o.quality, 40..=100).suffix(" %"));
                    ui.add_space(8.0);
                }
                ui.add_enabled_ui(self.doc.bleed > 0.0, |ui| {
                    ui.checkbox(&mut o.marks, "Fond perdu et traits de coupe (impression)");
                });
                if self.doc.bleed <= 0.0 {
                    ui.label(RichText::new("Règle un fond perdu dans le format du document pour l'activer.").size(11.5).color(FAINT));
                }
                ui.checkbox(&mut o.open_after, "Afficher le fichier après l'export");
                if self.overflow.iter().any(|v| *v) {
                    ui.label(RichText::new("Attention : du texte dépasse d'un bloc sur la page affichée.").size(12.0).color(RED));
                }
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Exporter…").clicked() {
                        go = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        cancel = true;
                    }
                });
            });
            if go {
                self.run_export(&o);
            } else if keep && !cancel {
                self.export = Some(o);
            }
        }
        if self.doc_sheet {
            let mut done = false;
            let keep = pro::sheet(ctx, "docfmt", 500.0, |ui| {
                ui.label(RichText::new("Format du document").font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    for (name, _, w, h) in PRESETS {
                        let on = (self.doc.width - w).abs() < 0.5 && (self.doc.height - h).abs() < 0.5;
                        if ui.selectable_label(on, name).clicked() {
                            self.doc.width = w;
                            self.doc.height = h;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let mut wm = self.doc.width / MM;
                    let mut hm = self.doc.height / MM;
                    ui.label(RichText::new("Largeur").size(12.5).color(MUTED));
                    if ui.add(egui::DragValue::new(&mut wm).range(10.0..=2000.0).speed(0.5).suffix(" mm").max_decimals(1)).changed() {
                        self.doc.width = wm * MM;
                    }
                    ui.label(RichText::new("Hauteur").size(12.5).color(MUTED));
                    if ui.add(egui::DragValue::new(&mut hm).range(10.0..=2000.0).speed(0.5).suffix(" mm").max_decimals(1)).changed() {
                        self.doc.height = hm * MM;
                    }
                });
                ui.horizontal(|ui| {
                    let portrait = self.doc.height >= self.doc.width;
                    if ui.selectable_label(portrait, "Portrait").clicked() && !portrait || ui.selectable_label(!portrait, "Paysage").clicked() && portrait {
                        std::mem::swap(&mut self.doc.width, &mut self.doc.height);
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let mut mm = self.doc.margin / MM;
                    ui.label(RichText::new("Marges").size(12.5).color(MUTED));
                    if ui.add(egui::DragValue::new(&mut mm).range(0.0..=100.0).speed(0.2).suffix(" mm").max_decimals(1)).changed() {
                        self.doc.margin = mm * MM;
                    }
                    let mut bm = self.doc.bleed / MM;
                    ui.label(RichText::new("Fond perdu").size(12.5).color(MUTED));
                    if ui.add(egui::DragValue::new(&mut bm).range(0.0..=12.0).speed(0.1).suffix(" mm").max_decimals(1)).changed() {
                        self.doc.bleed = bm * MM;
                    }
                    if ui.small_button("3 mm").on_hover_text("Valeur standard des imprimeurs").clicked() {
                        self.doc.bleed = 3.0 * MM;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Colonnes").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut self.doc.grid).range(1..=12));
                    ui.label(RichText::new("Gouttière").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut self.doc.grid_gutter).range(0.0..=80.0).suffix(" pt"));
                    ui.label(RichText::new("Grille de base").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut self.doc.baseline).range(0.0..=60.0).suffix(" pt"));
                });
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Terminé").clicked() {
                        done = true;
                    }
                });
            });
            if done || !keep {
                self.doc_sheet = false;
            }
        }
        if let Some(mut name) = self.style_name.take() {
            let mut ok = false;
            let mut cancel = false;
            let keep = pro::sheet(ctx, "style", 360.0, |ui| {
                ui.label(RichText::new("Nouveau style de paragraphe").font(semibold(17.0)).color(INK_STRONG));
                ui.add_space(8.0);
                ui.label(RichText::new("Police, taille, graisse, couleur, interligne et espacements du texte sélectionné.").size(12.0).color(MUTED));
                ui.add_space(8.0);
                let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY));
                r.request_focus();
                if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    ok = true;
                }
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Créer").clicked() {
                        ok = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        cancel = true;
                    }
                });
            });
            if ok && !name.trim().is_empty() {
                self.save_style(name.trim().to_string());
            } else if keep && !cancel {
                self.style_name = Some(name);
            }
        }
        let Some((mut find, mut with, mut count)) = self.find.take() else { return };
        let mut run = false;
        let mut close = false;
        let keep = pro::sheet(ctx, "find", 380.0, |ui| {
            ui.label(RichText::new("Rechercher et remplacer").font(semibold(18.0)).color(INK_STRONG));
            ui.add_space(10.0);
            ui.label(RichText::new("Rechercher").size(12.5).color(MUTED));
            ui.add(egui::TextEdit::singleline(&mut find).desired_width(f32::INFINITY));
            ui.label(RichText::new("Remplacer par").size(12.5).color(MUTED));
            ui.add(egui::TextEdit::singleline(&mut with).desired_width(f32::INFINITY));
            if let Some(n) = count {
                ui.label(RichText::new(format!("{n} remplacement(s) dans tout le document, tableaux compris")).size(12.5).color(if n > 0 { GREEN } else { MUTED }));
            }
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if pro::primary(ui, "Tout remplacer").clicked() {
                    run = true;
                }
                if pro::secondary(ui, "Fermer").clicked() {
                    close = true;
                }
            });
        });
        if run {
            count = Some(self.replace_all(&find, &with));
        }
        if keep && !close {
            self.find = Some((find, with, count));
        }
    }
}

const HANDLES: [(f32, f32); 8] = [(0.0, 0.0), (0.5, 0.0), (1.0, 0.0), (1.0, 0.5), (1.0, 1.0), (0.5, 1.0), (0.0, 1.0), (0.0, 0.5)];

fn swatch_button(ui: &mut Ui, c: &mut [u8; 4], swatches: &[[u8; 4]]) {
    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    if ui.color_edit_button_srgba(&mut col).changed() {
        *c = col.to_srgba_unmultiplied();
    }
    for s in swatches.iter().take(6) {
        let (r, resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
        ui.painter().circle_filled(r.center(), 6.5, Color32::from_rgb(s[0], s[1], s[2]));
        if resp.on_hover_text("Couleur du nuancier").clicked() {
            *c = *s;
        }
    }
}

fn color_row(ui: &mut Ui, label: &str, c: &mut [u8; 4], swatches: &[[u8; 4]]) {
    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.color_edit_button_srgba(&mut col);
            for s in swatches.iter().rev().take(7) {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                ui.painter().circle_filled(r.center(), 6.0, Color32::from_rgb(s[0], s[1], s[2]));
                if resp.clicked() {
                    col = Color32::from_rgba_unmultiplied(s[0], s[1], s[2], s[3]);
                }
            }
        });
    });
    *c = col.to_srgba_unmultiplied();
}

fn gradient_row(ui: &mut Ui, fill2: &mut Option<[u8; 4]>, angle: &mut f32, swatches: &[[u8; 4]]) {
    let mut on = fill2.is_some();
    if ui.checkbox(&mut on, "Dégradé").changed() {
        *fill2 = on.then_some([10, 132, 255, 255]);
    }
    if let Some(c) = fill2 {
        color_row(ui, "Vers", c, swatches);
        ui.label(RichText::new("Angle").size(12.5).color(MUTED));
        ui.add(egui::Slider::new(angle, -180.0..=180.0).suffix("°"));
    }
}

fn export_glyph(p: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(1.6, c);
    let cen = r.center();
    let u = r.width() / 2.0;
    p.line_segment([cen + vec2(0.0, u * 0.3), cen + vec2(0.0, -u * 0.95)], s);
    p.line_segment([cen + vec2(-u * 0.45, -u * 0.5), cen + vec2(0.0, -u * 0.95)], s);
    p.line_segment([cen + vec2(u * 0.45, -u * 0.5), cen + vec2(0.0, -u * 0.95)], s);
    p.add(Shape::line(vec![cen + vec2(-u * 0.9, -u * 0.1), cen + vec2(-u * 0.9, u * 0.9), cen + vec2(u * 0.9, u * 0.9), cen + vec2(u * 0.9, -u * 0.1)], s));
}

fn preset_card(ui: &mut Ui, name: &str, kind: &str, w: f32, h: f32, template: bool) -> egui::Response {
    let width = 168.0;
    let (r, resp) = ui.allocate_exact_size(vec2(width, 140.0), Sense::click());
    let p = ui.painter();
    p.rect(r, CornerRadius::same(12), if resp.hovered() { Color32::from_rgb(0x2f, 0x2f, 0x35) } else { Color32::from_rgb(0x27, 0x27, 0x2c) }, Stroke::new(1.0, LINE), StrokeKind::Inside);
    let area = Rect::from_center_size(r.center_top() + vec2(0.0, 48.0), vec2(width - 60.0, 64.0));
    let s = (area.width() / w).min(area.height() / h);
    let sheet = Rect::from_center_size(area.center(), vec2(w, h) * s);
    p.rect(sheet, CornerRadius::same(2), Color32::from_white_alpha(230), Stroke::NONE, StrokeKind::Inside);
    if template {
        let band = Rect::from_min_size(sheet.min, vec2(sheet.width(), sheet.height() * 0.3));
        p.rect_filled(band, CornerRadius { nw: 2, ne: 2, sw: 0, se: 0 }, Color32::from_rgb(255, 112, 52));
        for k in 0..4 {
            let y = band.bottom() + 5.0 + k as f32 * 5.0;
            if y < sheet.bottom() - 3.0 {
                p.line_segment([pos2(sheet.left() + 4.0, y), pos2(sheet.right() - 4.0 - (k as f32 * 6.0) % 14.0, y)], Stroke::new(1.5, Color32::from_gray(170)));
            }
        }
    }
    p.text(pos2(r.center().x, r.bottom() - 36.0), Align2::CENTER_CENTER, name, semibold(13.0), INK_STRONG);
    p.text(pos2(r.center().x, r.bottom() - 17.0), Align2::CENTER_CENTER, kind, FontId::proportional(10.5), MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn tool_icon(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    let s = Stroke::new(1.8, c);
    match tool {
        Tool::Select => {
            p.add(Shape::convex_polygon(vec![at(0.2, 0.0), at(0.85, 0.6), at(0.52, 0.62), at(0.33, 1.0)], c, Stroke::NONE));
        }
        Tool::Text => {
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.0), at(1.0, 1.0)), CornerRadius::same(2), Stroke::new(1.2, c), StrokeKind::Middle);
            p.text(r.center(), Align2::CENTER_CENTER, "T", semibold(r.width() * 0.8), c);
        }
        Tool::Image => {
            p.rect_stroke(r.shrink(1.0), CornerRadius::same(3), s, StrokeKind::Middle);
            p.circle_filled(at(0.3, 0.3), r.width() * 0.1, c);
            p.add(Shape::convex_polygon(vec![at(0.1, 0.9), at(0.5, 0.45), at(0.9, 0.9)], c, Stroke::NONE));
        }
        Tool::Rect => {
            p.rect_filled(Rect::from_min_max(at(0.05, 0.15), at(0.95, 0.85)), CornerRadius::same(2), c);
        }
        Tool::Ellipse => {
            p.circle_stroke(r.center(), r.width() * 0.45, s);
        }
        Tool::Polygon => {
            let pts: Vec<Pos2> = (0..10)
                .map(|k| {
                    let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::TAU / 10.0;
                    let rr = if k % 2 == 1 { 0.22 } else { 0.5 };
                    r.center() + vec2(a.cos(), a.sin()) * r.width() * rr
                })
                .collect();
            p.add(Shape::closed_line(pts, Stroke::new(1.5, c)));
        }
        Tool::Line => {
            p.line_segment([at(0.0, 0.5), at(1.0, 0.5)], Stroke::new(2.4, c));
        }
        Tool::Table => {
            let b = Rect::from_min_max(at(0.0, 0.1), at(1.0, 0.9));
            p.rect_stroke(b, CornerRadius::same(2), Stroke::new(1.4, c), StrokeKind::Middle);
            p.rect_filled(Rect::from_min_max(b.min, pos2(b.right(), b.top() + b.height() / 3.0)), CornerRadius::same(2), c);
            p.line_segment([pos2(b.left(), b.top() + b.height() * 2.0 / 3.0), pos2(b.right(), b.top() + b.height() * 2.0 / 3.0)], Stroke::new(1.0, c));
            p.line_segment([pos2(b.center().x, b.top()), pos2(b.center().x, b.bottom())], Stroke::new(1.0, c));
        }
        Tool::Pipette => {
            p.line_segment([at(0.15, 0.85), at(0.62, 0.38)], Stroke::new(2.4, c));
            p.circle_filled(at(0.72, 0.28), r.width() * 0.2, c);
            p.circle_filled(at(0.12, 0.88), r.width() * 0.07, c);
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Page")
            .with_icon(Arc::new(studio_ui::app_window_icon("page")))
            .with_inner_size([1480.0, 940.0])
            .with_min_inner_size([1100.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native("Page", options, Box::new(|cc| Ok(Box::new(PageApp::new(cc)))))
}

#[cfg(test)]
mod pdf_tests {
    use super::*;

    #[test]
    fn pdf_export_produces_two_pages() {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let fonts = Fonts { db: Arc::new(db) };
        let mut doc = starter(595.0, 842.0);
        doc.pages.push(Page::default());
        let m = |t: &str, f: &str, b: bool, s: f32| fonts.width(t, f, b, s);
        assert!(m("WWW", "Arial", false, 20.0) > m("iii", "Arial", false, 20.0));
        let opt = usvg::Options { fontdb: fonts.db.clone(), ..Default::default() };
        let mut document = krilla::Document::new();
        for i in 0..2 {
            let (svg, _) = doc.to_svg(i, &m, false);
            assert!(svg.contains(&format!("{} / 2", i + 1)));
            let tree = usvg::Tree::from_str(&svg, &opt).unwrap();
            let size = krilla::geom::Size::from_wh(doc.width, doc.height).unwrap();
            let mut page = document.start_page_with(krilla::page::PageSettings::new(size));
            let mut surface = page.surface();
            surface.draw_svg(&tree, size, SvgSettings::default()).unwrap();
            surface.finish();
            page.finish();
        }
        let bytes = document.finish().unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 1000);
    }

    #[test]
    fn templates_render() {
        let m = |t: &str, _: &str, _: bool, s: f32| t.chars().count() as f32 * s * 0.5;
        for k in 0..TEMPLATES.len() {
            let d = template(k);
            for i in 0..d.pages.len() {
                let (svg, _) = d.to_svg_with(i, &m, false, true);
                usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap_or_else(|e| panic!("modèle {k} : {e}"));
            }
        }
    }
}

#[cfg(test)]
mod dump {
    #[test]
    #[ignore]
    fn write_template() {
        let Ok(path) = std::env::var("PAGE_DEMO") else { return };
        std::fs::write(path, serde_json::to_vec_pretty(&super::template(0)).unwrap()).unwrap();
    }
}
