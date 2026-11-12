#![windows_subsystem = "windows"]

mod model;
mod tutos;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame as EFrame, Key, Layout, Margin, Painter,
    Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2,
    pos2, vec2,
};
use krilla_svg::{SurfaceExt, SvgSettings};
use model::{Doc, Frame, Kind, MASTER, Page};
use resvg::{tiny_skia, usvg};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};
use usvg::fontdb;

const FONTS: [&str; 10] = ["Segoe UI", "Arial", "Calibri", "Georgia", "Times New Roman", "Garamond", "Verdana", "Trebuchet MS", "Consolas", "Impact"];
const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.";
const STYLES: [(&str, f32, bool, bool, f32); 6] = [
    ("Titre 1", 32.0, true, false, 1.1),
    ("Titre 2", 21.0, true, false, 1.15),
    ("Chapeau", 13.0, false, true, 1.4),
    ("Corps", 10.5, false, false, 1.45),
    ("Légende", 8.0, false, true, 1.3),
    ("Citation", 16.0, false, true, 1.35),
];
const PRESETS: [(&str, &str, f32, f32); 8] = [
    ("A4", "Document, magazine", 595.0, 842.0),
    ("A5", "Livret, flyer", 420.0, 595.0),
    ("A3", "Affiche", 842.0, 1191.0),
    ("Lettre US", "Document", 612.0, 792.0),
    ("Carte de visite", "85 × 55 mm", 241.0, 156.0),
    ("Carré", "Publication 1:1", 810.0, 810.0),
    ("Story", "Réseaux sociaux", 540.0, 960.0),
    ("Présentation", "16:9", 960.0, 540.0),
];

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Text,
    Image,
    Rect,
    Ellipse,
    Line,
}

const TOOLS: [(Tool, &str, Key); 6] = [
    (Tool::Select, "Sélection (V)", Key::V),
    (Tool::Text, "Texte (T)", Key::T),
    (Tool::Image, "Image (I)", Key::I),
    (Tool::Rect, "Rectangle (M)", Key::M),
    (Tool::Ellipse, "Ellipse (L)", Key::L),
    (Tool::Line, "Ligne (\\)", Key::Backslash),
];

fn tool_name(t: Tool) -> &'static str {
    TOOLS.iter().find(|x| x.0 == t).map_or("", |x| x.1.split(" (").next().unwrap_or(""))
}

enum Drag {
    None,
    Move { last: [f32; 2], acc: [f32; 2], origin: [f32; 2] },
    Create { start: [f32; 2] },
    Resize { anchor: [f32; 2] },
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
    rendered: Option<(Doc, usize, f32)>,
    overflow: Vec<bool>,
    zoom: f32,
    fit_pending: bool,
    snap: bool,
    guides: bool,
    find: Option<(String, String, Option<usize>)>,
    notice: Option<(String, bool)>,
    title: String,
    icon: TextureHandle,
    help: studio_ui::help::Help,
}

fn recents_path() -> PathBuf {
    studio_core::data_dir().join("page").join("recents.txt")
}

fn starter(w: f32, h: f32) -> Doc {
    let mut doc = Doc { width: w, height: h, margin: (w.min(h) * 0.067).round(), ..Doc::a4() };
    let m = doc.margin;
    let title_size = (w * 0.055).clamp(14.0, 40.0);
    doc.pages[0].frames.push(Frame::new(m, m + 20.0, w - 2.0 * m, title_size * 1.6, Kind::text("Titre du document", title_size, true)));
    doc.master.frames.push(Frame::new(m, h - m * 0.75, w - 2.0 * m, 14.0, {
        let mut k = Kind::text("{page} / {pages}", 8.0, false);
        if let Kind::Text { align, color, .. } = &mut k {
            *align = 2;
            *color = [120, 120, 130, 255];
        }
        k
    }));
    doc
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
            find: None,
            notice: None,
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

    fn svg(&self, index: usize, guides: bool) -> (String, Vec<bool>) {
        self.doc.to_svg(index, &self.measure(), guides)
    }

    fn opts(&self) -> usvg::Options<'static> {
        usvg::Options { fontdb: self.fonts.db.clone(), ..Default::default() }
    }

    fn render_png(&self, index: usize, path: &Path) -> Result<(), String> {
        let (svg, _) = self.svg(index, false);
        let scale = 300.0 / 72.0;
        let tree = usvg::Tree::from_str(&svg, &self.opts()).map_err(|e| e.to_string())?;
        let mut pix = tiny_skia::Pixmap::new((self.doc.width * scale) as u32, (self.doc.height * scale) as u32).ok_or("taille")?;
        resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
        pix.save_png(path).map_err(|e| e.to_string())
    }

    fn export_pdf(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name("document.pdf").save_file() else { return };
        let opt = self.opts();
        let result = (|| -> Result<(), String> {
            let mut document = krilla::Document::new();
            for i in 0..self.doc.pages.len() {
                let (svg, _) = self.svg(i, false);
                let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| e.to_string())?;
                let size = krilla::geom::Size::from_wh(self.doc.width, self.doc.height).ok_or("taille invalide")?;
                let mut page = document.start_page_with(krilla::page::PageSettings::new(size));
                let mut surface = page.surface();
                surface.draw_svg(&tree, size, SvgSettings::default()).ok_or("rendu SVG impossible")?;
                surface.finish();
                page.finish();
            }
            let bytes = document.finish().map_err(|e| format!("{e:?}"))?;
            std::fs::write(&path, bytes).map_err(|e| e.to_string())
        })();
        self.notice = Some(match result {
            Ok(()) => (format!("PDF exporté : {}", path.display()), true),
            Err(e) => (format!("Export PDF : {e}"), false),
        });
    }

    fn export_png(&mut self, all: bool) {
        let result = if all {
            let Some(dir) = rfd::FileDialog::new().set_title("Dossier d'export").pick_folder() else { return };
            (0..self.doc.pages.len()).try_for_each(|i| self.render_png(i, &dir.join(format!("page-{:02}.png", i + 1))))
        } else {
            let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name(format!("page-{}.png", self.page.min(self.doc.pages.len() - 1) + 1)).save_file() else { return };
            self.render_png(self.page.min(self.doc.pages.len() - 1), &path)
        };
        self.notice = Some(match result {
            Ok(()) => ("PNG 300 dpi exporté".into(), true),
            Err(e) => (format!("Export PNG : {e}"), false),
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

    fn duplicate(&mut self) {
        if let Some(i) = self.sel {
            let mut f = self.frames()[i].clone();
            f.x += 12.0;
            f.y += 12.0;
            self.frames().push(f);
            self.sel = Some(self.frames().len() - 1);
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
        let f = self.frames().remove(i);
        self.frames().insert(j, f);
        self.sel = Some(j);
    }

    fn align(&mut self, axis: usize, to: f32) {
        let Some(i) = self.sel else { return };
        let (m, pw, ph) = (self.doc.margin, self.doc.width, self.doc.height);
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

    fn add_page(&mut self, at: usize, copy: bool) {
        let p = if copy && self.page != MASTER { self.doc.pages[self.page].clone() } else { Page::default() };
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

    fn replace_all(&mut self, find: &str, with: &str) -> usize {
        if find.is_empty() {
            return 0;
        }
        let mut n = 0;
        for f in self.doc.pages.iter_mut().flat_map(|p| p.frames.iter_mut()).chain(self.doc.master.frames.iter_mut()) {
            if let Kind::Text { text, .. } = &mut f.kind {
                n += text.matches(find).count();
                *text = text.replace(find, with);
            }
        }
        n
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
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
            if let Some(d) = self.redo.pop() {
                self.undo.push(std::mem::replace(&mut self.doc, d));
            }
        } else if sc(M::COMMAND, Key::Z)
            && let Some(d) = self.undo.pop()
        {
            self.redo.push(std::mem::replace(&mut self.doc, d));
        }
        if sc(M::COMMAND | M::SHIFT, Key::S) {
            self.save(true);
        } else if sc(M::COMMAND, Key::S) {
            self.save(false);
        }
        if sc(M::COMMAND, Key::E) {
            self.export_pdf();
        }
        if sc(M::COMMAND, Key::D) {
            self.duplicate();
        }
        if sc(M::COMMAND, Key::F) {
            self.find = Some((String::new(), String::new(), None));
        }
        if sc(M::COMMAND, Key::L)
            && let Some(i) = self.sel.take()
        {
            self.frames()[i].locked = true;
        }
        if sc(M::COMMAND, Key::Num0) {
            self.fit_pending = true;
        }
        let (del, pick, nudge, pg) = ctx.input(|i| {
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
            if i.key_pressed(Key::ArrowLeft) { n[0] -= step; }
            if i.key_pressed(Key::ArrowRight) { n[0] += step; }
            if i.key_pressed(Key::ArrowUp) { n[1] -= step; }
            if i.key_pressed(Key::ArrowDown) { n[1] += step; }
            let pg = if i.key_pressed(Key::PageDown) { 1 } else if i.key_pressed(Key::PageUp) { -1 } else { 0 };
            (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace), pick, n, pg)
        });
        if let Some(t) = pick {
            self.tool = t;
        }
        if del && let Some(i) = self.sel.take() {
            self.frames().remove(i);
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
        self.menu(root);
        if self.home {
            self.home_screen(root);
        } else {
            self.options_bar(root);
            self.status(root);
            self.tool_rail(root);
            self.pages_panel(root);
            self.props(root);
            self.canvas(root, &name);
        }
        self.find_sheet(ctx);
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
    fn menu(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(34.0).frame(EFrame::new().fill(CHROME).inner_margin(Margin::symmetric(10, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add(egui::Image::new((self.icon.id(), Vec2::splat(20.0))));
                ui.add_space(6.0);
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouveau…                  Ctrl+N").clicked() {
                            self.home = true;
                        }
                        if ui.button("Ouvrir…                     Ctrl+O").clicked() {
                            self.open();
                        }
                        ui.add_enabled_ui(!self.home, |ui| {
                            if ui.button("Enregistrer                 Ctrl+S").clicked() {
                                self.save(false);
                            }
                            if ui.button("Enregistrer sous…").clicked() {
                                self.save(true);
                            }
                            ui.separator();
                            if ui.button("Exporter en PDF…          Ctrl+E").clicked() {
                                self.export_pdf();
                            }
                            if ui.button("Exporter la page en PNG 300 dpi…").clicked() {
                                self.export_png(false);
                            }
                            if ui.button("Exporter toutes les pages en PNG…").clicked() {
                                self.export_png(true);
                            }
                        });
                    });
                    ui.add_enabled_ui(!self.home, |ui| {
                        ui.menu_button("Édition", |ui| {
                            if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler                 Ctrl+Z")).clicked()
                                && let Some(d) = self.undo.pop()
                            {
                                self.redo.push(std::mem::replace(&mut self.doc, d));
                            }
                            if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir                Ctrl+Y")).clicked()
                                && let Some(d) = self.redo.pop()
                            {
                                self.undo.push(std::mem::replace(&mut self.doc, d));
                            }
                            ui.separator();
                            if ui.add_enabled(self.sel.is_some(), egui::Button::new("Dupliquer              Ctrl+D")).clicked() {
                                self.duplicate();
                            }
                            if ui.add_enabled(self.sel.is_some(), egui::Button::new("Supprimer              Suppr")).clicked()
                                && let Some(i) = self.sel.take()
                            {
                                self.frames().remove(i);
                            }
                            ui.separator();
                            if ui.button("Rechercher et remplacer…   Ctrl+F").clicked() {
                                self.find = Some((String::new(), String::new(), None));
                            }
                        });
                        ui.menu_button("Objet", |ui| {
                            let has = self.sel.is_some();
                            for (label, to) in [("Premier plan", i32::MAX), ("Avancer", 1), ("Reculer", -1), ("Arrière-plan", i32::MIN)] {
                                if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                                    self.order(to);
                                }
                            }
                            ui.separator();
                            if ui.add_enabled(has, egui::Button::new("Verrouiller               Ctrl+L")).clicked()
                                && let Some(i) = self.sel.take()
                            {
                                self.frames()[i].locked = true;
                            }
                            if ui.button("Tout déverrouiller").clicked() {
                                self.frames().iter_mut().for_each(|f| f.locked = false);
                            }
                        });
                        ui.menu_button("Texte", |ui| {
                            let is_text = self.sel.is_some_and(|i| matches!(self.frames_ref()[i].kind, Kind::Text { .. }));
                            for (k, (name, ..)) in STYLES.iter().enumerate() {
                                if ui.add_enabled(is_text, egui::Button::new(format!("Style : {name}"))).clicked() {
                                    self.apply_style(k);
                                }
                            }
                            ui.separator();
                            if ui.add_enabled(is_text, egui::Button::new("Ajuster la hauteur au texte")).clicked() {
                                self.fit_text();
                            }
                            if ui.add_enabled(is_text, egui::Button::new("Remplir avec du faux texte")).clicked() {
                                self.lorem();
                            }
                        });
                        ui.menu_button("Page", |ui| {
                            let cur = if self.page == MASTER { 0 } else { self.page };
                            if ui.button("Ajouter une page après").clicked() {
                                self.add_page(cur + 1, false);
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
                                self.doc.pages.remove(self.page);
                                self.page = self.page.min(self.doc.pages.len() - 1);
                                self.sel = None;
                            }
                            ui.separator();
                            if ui.button("Modifier le gabarit").clicked() {
                                self.page = MASTER;
                                self.sel = None;
                            }
                        });
                        ui.menu_button("Affichage", |ui| {
                            if ui.button("Ajuster à la fenêtre      Ctrl+0").clicked() {
                                self.fit_pending = true;
                            }
                            for z in [0.5, 1.0, 1.5, 2.0] {
                                if ui.button(format!("{:.0} %", z * 100.0)).clicked() {
                                    self.zoom = z;
                                }
                            }
                            ui.separator();
                            ui.checkbox(&mut self.guides, "Repères (marges et colonnes)");
                            ui.checkbox(&mut self.snap, "Magnétisme");
                        });
                    });
                    self.help.menu(ui);
                });
            });
        });
    }

    fn apply_style(&mut self, k: usize) {
        let Some(i) = self.sel else { return };
        let (_, sz, b, it, lead) = STYLES[k];
        if let Kind::Text { size, bold, italic, leading, .. } = &mut self.frames()[i].kind {
            *size = sz;
            *bold = b;
            *italic = it;
            *leading = lead;
        }
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
                        if preset_card(ui, name, kind, w, h).clicked() {
                            chosen = Some((w, h));
                        }
                    }
                });
                if let Some((w, h)) = chosen {
                    self.set_doc(starter(w, h), None);
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
        egui::Panel::top("options").exact_size(44.0).frame(EFrame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(7), FIELD);
                tool_icon(ui.painter(), r.shrink(7.0), self.tool, INK);
                ui.label(RichText::new(tool_name(self.tool)).font(semibold(13.0)).color(INK));
                ui.add_space(8.0);
                let is_text = self.sel.is_some_and(|i| matches!(self.frames_ref()[i].kind, Kind::Text { .. }));
                if is_text {
                    ui.label(RichText::new("Style").size(12.5).color(MUTED));
                    for (k, (name, ..)) in STYLES.iter().enumerate() {
                        if ui.small_button(*name).clicked() {
                            self.apply_style(k);
                        }
                    }
                } else if self.sel.is_some() {
                    ui.label(RichText::new("Aligner sur les marges").size(12.5).color(MUTED));
                    for (label, axis, to) in [("⇤", 0, 0.0), ("↔", 0, 0.5), ("⇥", 0, 1.0), ("⤒", 1, 0.0), ("↕", 1, 0.5), ("⤓", 1, 1.0)] {
                        if ui.small_button(label).clicked() {
                            self.align(axis, to);
                        }
                    }
                } else {
                    let hint = match self.tool {
                        Tool::Select => "Clique un bloc pour le modifier, glisse ses coins pour le redimensionner",
                        Tool::Image => "Trace le cadre puis choisis l'image",
                        _ => "Trace le bloc sur la page",
                    };
                    ui.label(RichText::new(hint).size(12.5).color(MUTED));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::secondary(ui, "?  Tutoriel").on_hover_text("F1").clicked() {
                        self.help.learn(tool_name(self.tool));
                    }
                });
            });
        });
    }

    fn status(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(26.0).frame(EFrame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                let where_ = if self.page == MASTER { "Gabarit".to_string() } else { format!("Page {}/{}", self.page + 1, self.doc.pages.len()) };
                ui.label(small(format!("{where_} · {:.0} × {:.0} pt · {:.0} %", self.doc.width, self.doc.height, self.zoom * 100.0)));
                if let Some((n, ok)) = self.notice.clone() {
                    ui.label(RichText::new(n).size(12.0).color(if ok { GREEN } else { RED }));
                    if ui.small_button("×").clicked() {
                        self.notice = None;
                    }
                }
            });
        });
    }

    fn tool_rail(&mut self, root: &mut Ui) {
        egui::Panel::left("tools").resizable(false).exact_size(52.0).frame(EFrame::new().fill(PANEL).inner_margin(Margin::symmetric(7, 10))).show(root, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for (i, (tool, tip, _)) in TOOLS.iter().enumerate() {
                if i == 1 {
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
        let w = 96.0f32.min(140.0 / ratio);
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), w * ratio + 24.0), Sense::click());
        let card = Rect::from_center_size(pos2(r.center().x, r.min.y + w * ratio / 2.0 + 4.0), vec2(w, w * ratio));
        let p = ui.painter();
        p.rect_filled(card, CornerRadius::same(2), Color32::from_rgba_unmultiplied(self.doc.paper[0], self.doc.paper[1], self.doc.paper[2], 255));
        let s = w / self.doc.width;
        let master: &[Frame] = if page.is_some_and(|p| !p.no_master) { &self.doc.master.frames } else { &[] };
        for f in master.iter().chain(frames) {
            let fr = Rect::from_min_size(card.min + vec2(f.x, f.y) * s, vec2(f.w, f.h) * s);
            let c = match &f.kind {
                Kind::Text { .. } => Color32::from_gray(170),
                Kind::Image { .. } => Color32::from_rgb(180, 200, 220),
                Kind::Rect { fill, .. } | Kind::Ellipse { fill } => Color32::from_rgb(fill[0], fill[1], fill[2]),
                Kind::Line { color, .. } => Color32::from_rgb(color[0], color[1], color[2]),
            };
            p.rect_filled(fr, CornerRadius::ZERO, c);
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
        egui::Panel::left("pages").resizable(false).exact_size(168.0).frame(EFrame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 12))).show(root, |ui| {
            ui.label(RichText::new("PAGES").font(semibold(11.0)).color(FAINT));
            ui.add_space(6.0);
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let master = self.doc.master.frames.clone();
                if self.thumb(ui, None, &master, self.page == MASTER, "Gabarit").clicked() {
                    self.page = MASTER;
                    self.sel = None;
                }
                ui.add_space(6.0);
                for i in 0..self.doc.pages.len() {
                    let pg = self.doc.pages[i].clone();
                    if self.thumb(ui, Some(&pg), &pg.frames, self.page == i, &(i + 1).to_string()).clicked() {
                        self.page = i;
                        self.sel = None;
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
        egui::Panel::right("props").resizable(false).exact_size(292.0).frame(EFrame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let Some(i) = self.sel else {
                    self.doc_panel(ui);
                    return;
                };
                let over = self.overflow.get(i).copied().unwrap_or(false);
                let mut fit = false;
                let mut order = None;
                let f = &mut self.frames()[i];
                pro::dock_panel(ui, "pos", "Position", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("X").color(MUTED));
                        ui.add(egui::DragValue::new(&mut f.x).max_decimals(1));
                        ui.label(RichText::new("Y").color(MUTED));
                        ui.add(egui::DragValue::new(&mut f.y).max_decimals(1));
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("L").color(MUTED));
                        ui.add(egui::DragValue::new(&mut f.w).range(1.0..=5000.0).max_decimals(1));
                        ui.label(RichText::new("H").color(MUTED));
                        ui.add(egui::DragValue::new(&mut f.h).range(1.0..=5000.0).max_decimals(1));
                    });
                });
                pro::dock_panel(ui, "content", "Contenu", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    match &mut f.kind {
                        Kind::Text { text, size, family, bold, color, align, leading, italic, tracking, columns, gutter, background, padding } => {
                            ui.add(egui::TextEdit::multiline(text).desired_rows(7).desired_width(f32::INFINITY));
                            if over {
                                ui.label(RichText::new("Le texte dépasse du bloc.").size(12.0).color(RED));
                                if pro::secondary(ui, "Ajuster la hauteur au texte").clicked() {
                                    fit = true;
                                }
                            }
                            egui::ComboBox::from_id_salt("font").selected_text(family.as_str()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                                for name in FONTS {
                                    ui.selectable_value(family, name.to_string(), name);
                                }
                            });
                            ui.label(RichText::new("Taille").size(12.5).color(MUTED));
                            ui.add(egui::Slider::new(size, 4.0..=200.0).logarithmic(true).suffix(" pt"));
                            ui.label(RichText::new("Interligne").size(12.5).color(MUTED));
                            ui.add(egui::Slider::new(leading, 0.8..=2.5));
                            ui.label(RichText::new("Approche").size(12.5).color(MUTED));
                            ui.add(egui::Slider::new(tracking, -2.0..=10.0).suffix(" pt"));
                            ui.horizontal(|ui| {
                                ui.toggle_value(bold, RichText::new("G").strong());
                                ui.toggle_value(italic, RichText::new("I").italics());
                                ui.add_space(8.0);
                                ui.selectable_value(align, 0, "Gauche");
                                ui.selectable_value(align, 1, "Centre");
                                ui.selectable_value(align, 2, "Droite");
                            });
                            color_row(ui, "Couleur", color);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Colonnes").size(12.5).color(MUTED));
                                ui.add(egui::DragValue::new(columns).range(1..=6));
                                ui.label(RichText::new("Gouttière").size(12.5).color(MUTED));
                                ui.add(egui::DragValue::new(gutter).range(0.0..=80.0).suffix(" pt"));
                            });
                            let mut on = background.is_some();
                            if ui.checkbox(&mut on, "Fond coloré").changed() {
                                *background = on.then_some([245, 245, 247, 255]);
                            }
                            if let Some(bg) = background {
                                color_row(ui, "Fond", bg);
                            }
                            ui.label(RichText::new("Marge intérieure").size(12.5).color(MUTED));
                            ui.add(egui::Slider::new(padding, 0.0..=60.0).suffix(" pt"));
                        }
                        Kind::Image { path, cover, radius } => {
                            ui.label(RichText::new(if path.is_empty() { "Aucune image" } else { path.as_str() }).size(11.5).color(FAINT));
                            if pro::secondary(ui, "Choisir une image…").clicked()
                                && let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg"]).pick_file()
                            {
                                *path = p.to_string_lossy().into_owned();
                            }
                            ui.horizontal(|ui| {
                                ui.selectable_value(cover, true, "Remplir");
                                ui.selectable_value(cover, false, "Ajuster");
                            });
                            ui.label(RichText::new("Arrondi").size(12.5).color(MUTED));
                            ui.add(egui::Slider::new(radius, 0.0..=200.0));
                        }
                        Kind::Rect { fill, radius } => {
                            color_row(ui, "Couleur", fill);
                            ui.label(RichText::new("Arrondi").size(12.5).color(MUTED));
                            ui.add(egui::Slider::new(radius, 0.0..=200.0));
                        }
                        Kind::Ellipse { fill } => color_row(ui, "Couleur", fill),
                        Kind::Line { color, width } => {
                            color_row(ui, "Couleur", color);
                            ui.label(RichText::new("Épaisseur").size(12.5).color(MUTED));
                            ui.add(egui::Slider::new(width, 0.25..=20.0).suffix(" pt"));
                        }
                    }
                });
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
                            color_row(ui, "Couleur du contour", c);
                            ui.add(egui::Slider::new(&mut f.stroke_w, 0.25..=20.0).suffix(" pt"));
                        }
                    }
                    ui.checkbox(&mut f.locked, "Verrouillé");
                });
                pro::dock_panel(ui, "arrange", "Disposition", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (label, to) in [("Premier plan", i32::MAX), ("Avancer", 1), ("Reculer", -1), ("Arrière-plan", i32::MIN)] {
                            if ui.small_button(label).clicked() {
                                order = Some(to);
                            }
                        }
                    });
                });
                if fit {
                    self.fit_text();
                }
                if let Some(to) = order {
                    self.order(to);
                }
                pro::dock_panel(ui, "align", "Aligner sur les marges", |ui| {
                    ui.horizontal(|ui| {
                        for (label, axis, to) in [("Gauche", 0, 0.0), ("Centre", 0, 0.5), ("Droite", 0, 1.0)] {
                            if ui.small_button(label).clicked() {
                                self.align(axis, to);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        for (label, axis, to) in [("Haut", 1, 0.0), ("Milieu", 1, 0.5), ("Bas", 1, 1.0)] {
                            if ui.small_button(label).clicked() {
                                self.align(axis, to);
                            }
                        }
                    });
                });
            });
        });
    }

    fn doc_panel(&mut self, ui: &mut Ui) {
        let page = self.page;
        pro::dock_panel(ui, "doc", "Document", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.horizontal(|ui| {
                ui.label(RichText::new("L").color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.width).range(50.0..=5000.0).suffix(" pt"));
                ui.label(RichText::new("H").color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.height).range(50.0..=5000.0).suffix(" pt"));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Marges").color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.margin).range(0.0..=300.0).suffix(" pt"));
                ui.label(RichText::new("Colonnes").color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.grid).range(1..=12));
            });
            color_row(ui, "Papier", &mut self.doc.paper);
            if page != MASTER {
                let mut use_master = !self.doc.pages[page].no_master;
                if ui.checkbox(&mut use_master, "Utiliser le gabarit sur cette page").changed() {
                    self.doc.pages[page].no_master = !use_master;
                }
            } else {
                ui.label(RichText::new("Tu modifies le gabarit : ses blocs apparaissent sur toutes les pages.").size(12.0).color(FAINT));
            }
            ui.add_space(6.0);
            ui.label(RichText::new("Choisis un outil et trace un bloc, ou clique un bloc pour le modifier.").size(12.0).color(FAINT));
            if pro::secondary(ui, "?  Tutoriels").clicked() {
                self.help.learn("Texte");
            }
        });
    }

    fn snap_value(&self, v: f32, size: f32, axis: usize, tol: f32) -> f32 {
        if !self.snap {
            return v;
        }
        let (m, span) = (self.doc.margin, if axis == 0 { self.doc.width } else { self.doc.height });
        let lines = [0.0, m, span / 2.0, span - m, span];
        for (edge, off) in [(v, 0.0), (v + size / 2.0, size / 2.0), (v + size, size)] {
            if let Some(l) = lines.iter().find(|l| (edge - **l).abs() <= tol) {
                return l - off;
            }
        }
        v
    }

    fn canvas(&mut self, root: &mut Ui, name: &str) {
        egui::CentralPanel::default().frame(EFrame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let tab_bar = Rect::from_min_max(full.min, pos2(full.right(), full.top() + 32.0));
            ui.painter().rect_filled(tab_bar, CornerRadius::ZERO, PANEL);
            let where_ = if self.page == MASTER { "Gabarit".to_string() } else { format!("page {}", self.page + 1) };
            let label = format!("{name} · {where_} @ {:.0} %{}", self.zoom * 100.0, if self.doc != self.saved { " •" } else { "" });
            let g = ui.painter().layout_no_wrap(label, FontId::proportional(12.5), INK);
            let tab = Rect::from_min_size(tab_bar.min + vec2(8.0, 4.0), vec2(g.size().x + 28.0, 28.0));
            ui.painter().rect_filled(tab, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, PASTEBOARD);
            ui.painter().galley(tab.left_center() + vec2(14.0, -g.size().y / 2.0), g, INK);
            let area = Rect::from_min_max(pos2(full.left(), tab_bar.bottom()), full.max);
            if self.fit_pending {
                self.zoom = ((area.height() - 80.0) / self.doc.height).min((area.width() - 80.0) / self.doc.width).clamp(0.2, 3.0);
                self.fit_pending = false;
            }
            let (scroll, ctrl) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.command));
            if ctrl && scroll != 0.0 && ui.rect_contains_pointer(area) {
                self.zoom = (self.zoom * (scroll / 400.0).exp()).clamp(0.1, 6.0);
            }
            let px = ui.ctx().pixels_per_point();
            let size = vec2(self.doc.width, self.doc.height) * self.zoom;
            ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                    let avail = ui.available_size();
                    let (outer, _) = ui.allocate_exact_size(vec2(avail.x.max(size.x + 80.0), (size.y + 80.0).max(avail.y)), Sense::hover());
                    let page_rect = Rect::from_min_size(pos2(outer.center().x - size.x / 2.0, outer.min.y + 40.0), size);
                    let scale = self.zoom * px;
                    if self.rendered.as_ref().is_none_or(|(d, p, s)| *d != self.doc || *p != self.page || (*s - scale).abs() > 1e-3) {
                        let (svg, over) = self.svg(self.page, self.guides);
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
                        self.rendered = Some((self.doc.clone(), self.page, scale));
                    }
                    let painter = ui.painter();
                    painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(120) }.as_shape(page_rect, CornerRadius::ZERO));
                    if let Some(t) = &self.tex {
                        painter.image(t.id(), page_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    let z = self.zoom;
                    let to_doc = |p: Pos2| [(p.x - page_rect.min.x) / z, (p.y - page_rect.min.y) / z];
                    let to_scr = |p: [f32; 2]| pos2(page_rect.min.x + p[0] * z, page_rect.min.y + p[1] * z);
                    for (i, f) in self.frames_ref().iter().enumerate() {
                        let r = Rect::from_min_max(to_scr([f.x, f.y]), to_scr([f.x + f.w, f.y + f.h]));
                        if self.sel == Some(i) {
                            painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.5, BLUE), StrokeKind::Middle);
                            for c in f.corners() {
                                painter.rect(Rect::from_center_size(to_scr(c), Vec2::splat(8.0)), CornerRadius::same(2), Color32::WHITE, Stroke::new(1.2, BLUE), StrokeKind::Inside);
                            }
                        } else if self.guides {
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
                    let resp = ui.interact(page_rect.expand(20.0), ui.scope_id().with("page"), Sense::click_and_drag());
                    let tol = 6.0 / z;
                    if let Some(sp) = resp.interact_pointer_pos() {
                        let p = to_doc(sp);
                        if resp.drag_started() || (resp.clicked() && matches!(self.drag, Drag::None)) {
                            self.press(p, tol);
                        }
                        if resp.dragged() {
                            self.drag_to(p, tol);
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
                                _ => (160.0, 120.0),
                            };
                        }
                        if matches!(self.drag, Drag::Create { .. }) {
                            self.tool = Tool::Select;
                        }
                        self.drag = Drag::None;
                    }
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(if self.tool == Tool::Select { egui::CursorIcon::Default } else { egui::CursorIcon::Crosshair });
                    }
                });
            });
        });
    }

    fn press(&mut self, p: [f32; 2], tol: f32) {
        match self.tool {
            Tool::Select => {
                if let Some(i) = self.sel {
                    let corners = self.frames()[i].corners();
                    if let Some(k) = corners.iter().position(|c| (c[0] - p[0]).abs() <= tol && (c[1] - p[1]).abs() <= tol) {
                        self.drag = Drag::Resize { anchor: corners[(k + 2) % 4] };
                        return;
                    }
                }
                self.sel = (0..self.frames_ref().len()).rev().find(|&i| !self.frames_ref()[i].locked && self.frames_ref()[i].contains(p, tol));
                if let Some(i) = self.sel {
                    let f = &self.frames_ref()[i];
                    self.drag = Drag::Move { last: p, acc: [0.0, 0.0], origin: [f.x, f.y] };
                }
            }
            tool => {
                let kind = match tool {
                    Tool::Text => Kind::text("Ton texte ici", 12.0, false),
                    Tool::Image => Kind::Image { path: String::new(), cover: true, radius: 0.0 },
                    Tool::Ellipse => Kind::Ellipse { fill: [255, 120, 60, 255] },
                    Tool::Line => Kind::Line { color: [20, 20, 24, 255], width: 1.5 },
                    _ => Kind::Rect { fill: [255, 120, 60, 255], radius: 0.0 },
                };
                let pick = matches!(kind, Kind::Image { .. });
                self.frames().push(Frame::new(p[0], p[1], 0.0, 0.0, kind));
                self.sel = Some(self.frames().len() - 1);
                self.drag = Drag::Create { start: p };
                if pick
                    && let Some(path) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg"]).pick_file()
                    && let Some(i) = self.sel
                    && let Kind::Image { path: pp, .. } = &mut self.frames()[i].kind
                {
                    *pp = path.to_string_lossy().into_owned();
                }
            }
        }
    }

    fn drag_to(&mut self, p: [f32; 2], tol: f32) {
        let Some(i) = self.sel else { return };
        let (fw, fh) = (self.frames_ref()[i].w, self.frames_ref()[i].h);
        let snapped = match &mut self.drag {
            Drag::Move { last, acc, origin } => {
                acc[0] += p[0] - last[0];
                acc[1] += p[1] - last[1];
                *last = p;
                Some([origin[0] + acc[0], origin[1] + acc[1]])
            }
            _ => None,
        };
        if let Some([x, y]) = snapped {
            let (x, y) = (self.snap_value(x, fw, 0, tol), self.snap_value(y, fh, 1, tol));
            let f = &mut self.frames()[i];
            f.x = x;
            f.y = y;
            return;
        }
        let anchor = match &self.drag {
            Drag::Create { start } => *start,
            Drag::Resize { anchor } => *anchor,
            _ => return,
        };
        let q = [self.snap_value(p[0], 0.0, 0, tol), self.snap_value(p[1], 0.0, 1, tol)];
        let f = &mut self.frames()[i];
        f.x = anchor[0].min(q[0]);
        f.y = anchor[1].min(q[1]);
        f.w = (q[0] - anchor[0]).abs();
        f.h = (q[1] - anchor[1]).abs();
    }

    fn find_sheet(&mut self, ctx: &egui::Context) {
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
                ui.label(RichText::new(format!("{n} remplacement(s) dans tout le document")).size(12.5).color(if n > 0 { GREEN } else { MUTED }));
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

fn color_row(ui: &mut Ui, label: &str, c: &mut [u8; 4]) {
    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.color_edit_button_srgba(&mut col);
        });
    });
    *c = col.to_srgba_unmultiplied();
}

fn preset_card(ui: &mut Ui, name: &str, kind: &str, w: f32, h: f32) -> egui::Response {
    let width = 168.0;
    let (r, resp) = ui.allocate_exact_size(vec2(width, 140.0), Sense::click());
    let p = ui.painter();
    p.rect(r, CornerRadius::same(12), if resp.hovered() { Color32::from_rgb(0x2f, 0x2f, 0x35) } else { Color32::from_rgb(0x27, 0x27, 0x2c) }, Stroke::new(1.0, LINE), StrokeKind::Inside);
    let area = Rect::from_center_size(r.center_top() + vec2(0.0, 48.0), vec2(width - 60.0, 64.0));
    let s = (area.width() / w).min(area.height() / h);
    p.rect(Rect::from_center_size(area.center(), vec2(w, h) * s), CornerRadius::same(2), Color32::from_white_alpha(230), Stroke::NONE, StrokeKind::Inside);
    p.text(pos2(r.center().x, r.bottom() - 36.0), Align2::CENTER_CENTER, name, semibold(13.0), INK_STRONG);
    p.text(pos2(r.center().x, r.bottom() - 17.0), Align2::CENTER_CENTER, format!("{kind} · {w:.0} × {h:.0} pt"), FontId::proportional(10.5), MUTED);
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
        Tool::Line => {
            p.line_segment([at(0.0, 0.5), at(1.0, 0.5)], Stroke::new(2.4, c));
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Page")
            .with_icon(Arc::new(studio_ui::app_window_icon("page")))
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([1000.0, 640.0]),
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
}
