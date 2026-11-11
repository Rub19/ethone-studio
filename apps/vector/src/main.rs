#![windows_subsystem = "windows"]

mod model;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, Painter, Pos2,
    Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2, pos2,
    vec2,
};
use model::{BBox, Doc, Geom, Item, Node, P, polygon};
use resvg::{tiny_skia, usvg};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};

const MAX_RENDER: f32 = 4096.0;

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Pen,
    Rect,
    Ellipse,
    Polygon,
    Star,
    Line,
    Text,
    Picker,
    Hand,
    Zoom,
}

const TOOLS: [(Tool, &str, Key); 11] = [
    (Tool::Select, "Sélection (V)", Key::V),
    (Tool::Pen, "Plume (P)", Key::P),
    (Tool::Rect, "Rectangle (M)", Key::M),
    (Tool::Ellipse, "Ellipse (L)", Key::L),
    (Tool::Polygon, "Polygone (Y)", Key::Y),
    (Tool::Star, "Étoile (S)", Key::S),
    (Tool::Line, "Ligne (N)", Key::N),
    (Tool::Text, "Texte (T)", Key::T),
    (Tool::Picker, "Pipette (I)", Key::I),
    (Tool::Hand, "Main (H, ou Espace)", Key::H),
    (Tool::Zoom, "Zoom (Z, Alt pour dézoomer)", Key::Z),
];

fn tool_name(t: Tool) -> &'static str {
    TOOLS.iter().find(|x| x.0 == t).map_or("", |x| x.1.split(" (").next().unwrap_or(""))
}

const PRESETS: [(&str, &str, f32, f32); 8] = [
    ("Logo", "Carré", 1000.0, 1000.0),
    ("Icône", "Interface", 512.0, 512.0),
    ("Full HD", "Écran 16:9", 1920.0, 1080.0),
    ("Bannière", "Web", 1500.0, 500.0),
    ("Publication", "Réseaux sociaux", 1080.0, 1080.0),
    ("Story", "Réseaux sociaux", 1080.0, 1920.0),
    ("A4", "Impression", 794.0, 1123.0),
    ("Carte de visite", "Impression", 1050.0, 600.0),
];

const FONTS: [&str; 8] = ["Segoe UI", "Arial", "Georgia", "Times New Roman", "Consolas", "Impact", "Verdana", "Trebuchet MS"];

enum Drag {
    None,
    Move { last: P },
    Create { start: P },
    Scale { anchor: P, orig: Item, from: P },
    NodePart { node: usize, part: u8 },
    PenHandle { node: usize },
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
    tool: Tool,
    style: Style,
    sides: u32,
    star_ratio: f32,
    zoom: f32,
    offset: Vec2,
    fit_pending: bool,
    drag: Drag,
    pen: Option<usize>,
    fontdb: Arc<usvg::fontdb::Database>,
    tex: Option<TextureHandle>,
    rendered: Option<(Doc, f32)>,
    checker: TextureHandle,
    icon: TextureHandle,
    new_doc: Option<NewDoc>,
    cursor: Option<P>,
    error: Option<String>,
    title: String,
}

fn recents_path() -> PathBuf {
    studio_core::data_dir().join("vector").join("recents.txt")
}

fn c32(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

impl VectorApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        let doc = Doc::new(1000.0, 1000.0);
        let recents = std::fs::read_to_string(recents_path())
            .map(|s| s.lines().map(PathBuf::from).filter(|p| p.exists()).collect())
            .unwrap_or_default();
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
            tool: Tool::Select,
            style: Style { fill: Some(Color32::from_rgb(255, 176, 46)), stroke: Some(Color32::from_rgb(20, 20, 24)), stroke_w: 2.0 },
            sides: 6,
            star_ratio: 0.5,
            zoom: 1.0,
            offset: Vec2::ZERO,
            fit_pending: true,
            drag: Drag::None,
            pen: None,
            fontdb: Arc::new(db),
            tex: None,
            rendered: None,
            checker: checker(&cc.egui_ctx),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "vector"),
            new_doc: None,
            cursor: None,
            error: None,
            title: String::new(),
        };
        if let Some(p) = std::env::args().nth(1) {
            me.open_path(Path::new(&p));
        }
        me
    }

    fn set_doc(&mut self, doc: Doc, file: Option<PathBuf>) {
        self.saved = doc.clone();
        self.doc = doc;
        self.file = file;
        self.undo.clear();
        self.redo.clear();
        self.sel = None;
        self.pen = None;
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

    fn checkpoint(&mut self) {
        self.undo.push(self.doc.clone());
        if self.undo.len() > 100 {
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
        if self.sel.is_some_and(|i| i >= self.doc.items.len()) {
            self.sel = None;
        }
    }

    fn new_item(&self, geom: Geom) -> Item {
        let kind = match &geom {
            Geom::Rect { .. } => "Rectangle",
            Geom::Ellipse { .. } => "Ellipse",
            Geom::Path { closed: false, .. } => "Tracé",
            Geom::Path { .. } => "Forme",
            Geom::Text { .. } => "Texte",
        };
        let is_text = matches!(geom, Geom::Text { .. });
        let is_line = matches!(geom, Geom::Path { closed: false, .. });
        Item {
            name: format!("{kind} {}", self.doc.items.len() + 1),
            geom,
            fill: if is_text {
                Some(self.style.stroke.unwrap_or(Color32::BLACK).to_array())
            } else if is_line {
                None
            } else {
                self.style.fill.map(|c| c.to_array())
            },
            stroke: if is_text { None } else { self.style.stroke.or(Some(Color32::BLACK)).filter(|_| is_line || self.style.stroke.is_some()).map(|c| c.to_array()) },
            stroke_w: self.style.stroke_w,
            opacity: 1.0,
            visible: true,
        }
    }

    fn render(&mut self, ctx: &egui::Context) {
        let scale = (self.zoom * ctx.pixels_per_point()).min(MAX_RENDER / self.doc.width.max(self.doc.height));
        if self.rendered.as_ref().is_some_and(|(d, s)| *d == self.doc && (*s - scale).abs() < 1e-3) {
            return;
        }
        let w = (self.doc.width * scale).ceil().max(1.0) as u32;
        let h = (self.doc.height * scale).ceil().max(1.0) as u32;
        let opt = usvg::Options { fontdb: self.fontdb.clone(), ..Default::default() };
        let tree = match usvg::Tree::from_str(&self.doc.to_svg(), &opt) {
            Ok(t) => t,
            Err(e) => {
                self.error = Some(format!("Rendu impossible : {e}"));
                return;
            }
        };
        let Some(mut pix) = tiny_skia::Pixmap::new(w, h) else { return };
        resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
        let img = egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], pix.data());
        match &mut self.tex {
            Some(t) if t.size() == [w as usize, h as usize] => t.set(img, TextureOptions::LINEAR),
            _ => self.tex = Some(ctx.load_texture("art", img, TextureOptions::LINEAR)),
        }
        self.rendered = Some((self.doc.clone(), scale));
    }

    fn export_png(&mut self, scale: f32) {
        let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name("export.png").save_file() else { return };
        let opt = usvg::Options { fontdb: self.fontdb.clone(), ..Default::default() };
        let result = usvg::Tree::from_str(&self.doc.to_svg(), &opt).map_err(|e| e.to_string()).and_then(|tree| {
            let mut pix = tiny_skia::Pixmap::new((self.doc.width * scale) as u32, (self.doc.height * scale) as u32).ok_or("taille invalide")?;
            resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
            pix.save_png(&path).map_err(|e| e.to_string())
        });
        if let Err(e) = result {
            self.error = Some(format!("Export PNG : {e}"));
        }
    }

    fn export_svg(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("SVG", &["svg"]).set_file_name("export.svg").save_file() else { return };
        if let Err(e) = std::fs::write(path, self.doc.to_svg()) {
            self.error = Some(format!("Export SVG : {e}"));
        }
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
        match std::fs::read(path).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Doc>(&b).map_err(|e| e.to_string())) {
            Ok(doc) => {
                self.set_doc(doc, Some(path.to_path_buf()));
                self.remember(path);
            }
            Err(e) => self.error = Some(format!("Ouverture : {e}")),
        }
    }

    fn open(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("Projet Vector", &["evec"]).pick_file() {
            self.open_path(&path);
        }
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

    fn delete_selected(&mut self) {
        if let Some(i) = self.sel.take() {
            self.checkpoint();
            self.doc.items.remove(i);
            self.pen = None;
        }
    }

    fn duplicate_selected(&mut self) {
        if let Some(i) = self.sel {
            self.checkpoint();
            let mut it = self.doc.items[i].clone();
            it.name = format!("{} copie", it.name);
            it.translate([16.0, 16.0]);
            self.doc.items.insert(i + 1, it);
            self.sel = Some(i + 1);
        }
    }

    fn reorder(&mut self, by: i32) {
        let Some(i) = self.sel else { return };
        let last = self.doc.items.len() as i32 - 1;
        let to = (i as i32).saturating_add(by).clamp(0, last) as usize;
        if to != i {
            self.checkpoint();
            let it = self.doc.items.remove(i);
            self.doc.items.insert(to, it);
            self.sel = Some(to);
        }
    }

    fn flip(&mut self, axis: usize) {
        let Some(i) = self.sel else { return };
        self.checkpoint();
        let b = self.doc.items[i].bbox();
        let c = (b.min[axis] + b.max[axis]) / 2.0;
        self.doc.items[i].map(
            |mut p| {
                p[axis] = 2.0 * c - p[axis];
                p
            },
            1.0,
            1.0,
        );
    }

    fn align(&mut self, axis: usize, to: f32) {
        let Some(i) = self.sel else { return };
        self.checkpoint();
        let b = self.doc.items[i].bbox();
        let size = [self.doc.width, self.doc.height][axis];
        let target = match to {
            t if t < 0.25 => 0.0,
            t if t > 0.75 => size - (b.max[axis] - b.min[axis]),
            _ => (size - (b.max[axis] - b.min[axis])) / 2.0,
        };
        let mut d = [0.0, 0.0];
        d[axis] = target - b.min[axis];
        self.doc.items[i].translate(d);
    }

    fn set_bbox(&mut self, i: usize, nb: BBox) {
        let it = &mut self.doc.items[i];
        let b = it.bbox();
        let sx = (nb.max[0] - nb.min[0]) / (b.max[0] - b.min[0]).max(1e-3);
        let sy = (nb.max[1] - nb.min[1]) / (b.max[1] - b.min[1]).max(1e-3);
        it.map(|p| [nb.min[0] + (p[0] - b.min[0]) * sx, nb.min[1] + (p[1] - b.min[1]) * sy], sx, sy);
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() || self.new_doc.is_some() {
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
        if sc(M::COMMAND | M::SHIFT, Key::Z) || sc(M::COMMAND, Key::Y) {
            self.redo();
        } else if sc(M::COMMAND, Key::Z) {
            self.undo();
        }
        if sc(M::COMMAND, Key::D) || sc(M::COMMAND, Key::J) {
            self.duplicate_selected();
        }
        if sc(M::COMMAND | M::SHIFT, Key::S) {
            self.save(true);
        } else if sc(M::COMMAND, Key::S) {
            self.save(false);
        }
        if sc(M::COMMAND, Key::E) {
            self.export_png(2.0);
        }
        if sc(M::COMMAND, Key::Num0) {
            self.fit_pending = true;
        }
        if sc(M::COMMAND, Key::Num1) {
            self.zoom = 1.0;
            self.offset = Vec2::ZERO;
        }
        if sc(M::COMMAND | M::SHIFT, Key::CloseBracket) {
            self.reorder(i32::MAX);
        } else if sc(M::COMMAND, Key::CloseBracket) {
            self.reorder(1);
        }
        if sc(M::COMMAND | M::SHIFT, Key::OpenBracket) {
            self.reorder(i32::MIN);
        } else if sc(M::COMMAND, Key::OpenBracket) {
            self.reorder(-1);
        }
        let (pick, del, enter, nudge) = ctx.input(|i| {
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
            (pick, i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace), i.key_pressed(Key::Enter) || i.key_pressed(Key::Escape), n)
        });
        if let Some(t) = pick {
            self.finish_pen();
            self.tool = t;
        }
        if enter {
            self.finish_pen();
        }
        if del {
            self.delete_selected();
        }
        if nudge != [0.0, 0.0]
            && let Some(i) = self.sel
        {
            self.checkpoint();
            self.doc.items[i].translate(nudge);
        }
    }
}

impl eframe::App for VectorApp {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let before = self.doc.clone();
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
            self.status_bar(root, &name);
            self.tool_rail(root);
            self.dock(root);
            self.canvas(root, &name);
        }
        self.new_doc_sheet(ctx);
        if !self.home && self.doc != before && !self.editing && matches!(self.drag, Drag::None) {
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
        egui::Panel::top("menu").exact_size(34.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(10, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add(egui::Image::new((self.icon.id(), Vec2::splat(20.0))));
                ui.add_space(6.0);
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouveau…                 Ctrl+N").clicked() {
                            self.new_doc = Some(NewDoc { w: 1000.0, h: 1000.0, preset: 0, transparent: false });
                        }
                        if ui.button("Ouvrir…                    Ctrl+O").clicked() {
                            self.open();
                        }
                        ui.add_enabled_ui(!self.home, |ui| {
                            if ui.button("Enregistrer                Ctrl+S").clicked() {
                                self.save(false);
                            }
                            if ui.button("Enregistrer sous…     Ctrl+Maj+S").clicked() {
                                self.save(true);
                            }
                            ui.separator();
                            if ui.button("Exporter en SVG…").clicked() {
                                self.export_svg();
                            }
                            for s in [1.0, 2.0, 4.0] {
                                if ui.button(format!("Exporter en PNG ×{s}…")).clicked() {
                                    self.export_png(s);
                                }
                            }
                            ui.separator();
                            if ui.button("Fermer").clicked() {
                                self.home = true;
                            }
                        });
                    });
                    ui.add_enabled_ui(!self.home, |ui| {
                        ui.menu_button("Édition", |ui| {
                            if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler                 Ctrl+Z")).clicked() {
                                self.undo();
                            }
                            if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir                Ctrl+Y")).clicked() {
                                self.redo();
                            }
                            ui.separator();
                            if ui.add_enabled(self.sel.is_some(), egui::Button::new("Dupliquer              Ctrl+D")).clicked() {
                                self.duplicate_selected();
                            }
                            if ui.add_enabled(self.sel.is_some(), egui::Button::new("Supprimer              Suppr")).clicked() {
                                self.delete_selected();
                            }
                        });
                        ui.menu_button("Objet", |ui| {
                            let has = self.sel.is_some();
                            for (label, by) in [("Premier plan      Ctrl+Maj+]", i32::MAX), ("Avancer              Ctrl+]", 1), ("Reculer               Ctrl+[", -1), ("Arrière-plan       Ctrl+Maj+[", i32::MIN)] {
                                if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                                    self.reorder(by);
                                }
                            }
                            ui.separator();
                            if ui.add_enabled(has, egui::Button::new("Symétrie horizontale")).clicked() {
                                self.flip(0);
                            }
                            if ui.add_enabled(has, egui::Button::new("Symétrie verticale")).clicked() {
                                self.flip(1);
                            }
                        });
                        ui.menu_button("Affichage", |ui| {
                            if ui.button("Ajuster à l'écran       Ctrl+0").clicked() {
                                self.fit_pending = true;
                            }
                            if ui.button("Taille réelle             Ctrl+1").clicked() {
                                self.zoom = 1.0;
                                self.offset = Vec2::ZERO;
                            }
                        });
                    });
                });
            });
        });
    }

    fn home_screen(&mut self, root: &mut Ui) {
        egui::Panel::left("home-nav").exact_size(230.0).resizable(false).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(18, 26))).show(root, |ui| {
            if pro::primary(ui, "Nouveau fichier").clicked() {
                self.new_doc = Some(NewDoc { w: 1000.0, h: 1000.0, preset: 0, transparent: false });
            }
            ui.add_space(8.0);
            if pro::secondary(ui, "Ouvrir…").clicked() {
                self.open();
            }
            ui.add_space(26.0);
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
            ui.painter().rect_filled(r, CornerRadius::same(9), BLUE.gamma_multiply(0.22));
            ui.painter().text(r.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, "Accueil", semibold(14.0), INK_STRONG);
        });
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin { left: 40, right: 40, top: 34, bottom: 20 })).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(40.0))));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Bienvenue dans Vector").font(semibold(28.0)).color(INK_STRONG));
                        ui.label(RichText::new("Logos, icônes et illustrations nets à toutes les tailles").color(MUTED));
                    });
                });
                ui.add_space(28.0);
                ui.label(RichText::new("Créer un plan de travail").font(semibold(16.0)).color(INK));
                ui.add_space(12.0);
                let gap = 14.0;
                let card_w = 168.0;
                let cols = (((ui.available_width() + gap) / (card_w + gap)).floor() as usize).max(1);
                let mut chosen = None;
                for row in PRESETS.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for (name, kind, w, h) in row {
                            if preset_card(ui, name, &format!("{w} × {h} · {kind}"), *w, *h, false, card_w).clicked() {
                                chosen = Some((*w, *h));
                            }
                        }
                    });
                    ui.add_space(gap);
                }
                if let Some((w, h)) = chosen {
                    self.set_doc(Doc::new(w, h), None);
                }
                ui.add_space(14.0);
                ui.label(RichText::new("Récents").font(semibold(16.0)).color(INK));
                ui.add_space(10.0);
                if self.recents.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().min(760.0), 110.0), Sense::hover());
                    let p = ui.painter();
                    p.rect(r, CornerRadius::same(14), Color32::from_white_alpha(4), Stroke::new(1.0, LINE), StrokeKind::Inside);
                    p.text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "Tes dessins enregistrés apparaîtront ici", semibold(15.0), INK);
                    p.text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "Projets .evec · glisse-en un pour l'ouvrir", FontId::proportional(13.0), MUTED);
                } else {
                    let mut open = None;
                    for p in &self.recents {
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width().min(760.0), 46.0), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(r, CornerRadius::same(10), Color32::from_white_alpha(10));
                        }
                        let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                        ui.painter().text(r.left_top() + vec2(14.0, 8.0), Align2::LEFT_TOP, name, semibold(14.0), INK);
                        ui.painter().text(r.left_top() + vec2(14.0, 27.0), Align2::LEFT_TOP, p.parent().map(|d| d.display().to_string()).unwrap_or_default(), FontId::proportional(12.0), FAINT);
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open = Some(p.clone());
                        }
                    }
                    if let Some(p) = open {
                        self.open_path(&p);
                    }
                }
                if let Some(e) = self.error.clone() {
                    ui.add_space(12.0);
                    ui.label(RichText::new(e).color(RED));
                }
            });
        });
    }

    fn options_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("options").exact_size(44.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(7), FIELD);
                tool_icon(ui.painter(), r.shrink(7.0), self.tool, INK);
                ui.label(RichText::new(tool_name(self.tool)).font(semibold(13.0)).color(INK));
                separator(ui);
                match self.tool {
                    Tool::Select => {
                        if self.sel.is_some() {
                            ui.label(RichText::new("Aligner").size(12.5).color(MUTED));
                            for (tip, axis, to) in [("À gauche", 0, 0.0), ("Au centre (horizontal)", 0, 0.5), ("À droite", 0, 1.0), ("En haut", 1, 0.0), ("Au milieu (vertical)", 1, 0.5), ("En bas", 1, 1.0)] {
                                if pro::icon_button(ui, 30.0, false, tip, |p, r, c| align_icon(p, r, axis, to, c)).clicked() {
                                    self.align(axis, to);
                                }
                            }
                            separator(ui);
                            if pro::secondary(ui, "Dupliquer").clicked() {
                                self.duplicate_selected();
                            }
                        } else {
                            ui.label(RichText::new("Clique un objet pour le sélectionner, glisse ses coins pour le redimensionner").size(12.5).color(MUTED));
                        }
                    }
                    Tool::Pen => {
                        ui.label(RichText::new("Clic : point · glisser : courbe · clic sur le premier point : fermer · Entrée : terminer").size(12.5).color(MUTED));
                    }
                    Tool::Polygon | Tool::Star => {
                        ui.label(RichText::new(if self.tool == Tool::Star { "Branches" } else { "Côtés" }).size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(&mut self.sides).range(3..=24));
                        if self.tool == Tool::Star {
                            pro::option_slider(ui, "Creux", &mut self.star_ratio, 0.1..=0.9, "", false);
                        }
                        separator(ui);
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::Rect | Tool::Ellipse | Tool::Line => {
                        style_swatches(ui, &mut self.style);
                        ui.label(RichText::new("Maj : proportions fixes").size(12.0).color(FAINT));
                    }
                    Tool::Text => {
                        ui.label(RichText::new("Clique sur le plan de travail pour ajouter un texte").size(12.5).color(MUTED));
                    }
                    Tool::Picker => {
                        ui.label(RichText::new("Clique un objet pour copier son style sur la sélection").size(12.5).color(MUTED));
                    }
                    Tool::Hand | Tool::Zoom => {
                        if pro::secondary(ui, "Ajuster à l'écran").clicked() {
                            self.fit_pending = true;
                        }
                        if pro::secondary(ui, "100 %").clicked() {
                            self.zoom = 1.0;
                            self.offset = Vec2::ZERO;
                        }
                    }
                }
            });
        });
    }

    fn status_bar(&mut self, root: &mut Ui, _name: &str) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                ui.label(small(format!("{:.0} %", self.zoom * 100.0)));
                ui.label(small(format!("Plan de travail {} × {}", self.doc.width, self.doc.height)));
                ui.label(small(format!("{} objet(s)", self.doc.items.len())));
                if let Some(p) = self.cursor {
                    ui.label(small(format!("x {:.0}  y {:.0}", p[0], p[1])));
                }
                if let Some(e) = self.error.clone() {
                    ui.label(RichText::new(e).size(12.0).color(RED));
                    if ui.small_button("×").clicked() {
                        self.error = None;
                    }
                }
            });
        });
    }

    fn tool_rail(&mut self, root: &mut Ui) {
        egui::Panel::left("tools").resizable(false).exact_size(52.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(7, 10))).show(root, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for (i, (tool, tip, _)) in TOOLS.iter().enumerate() {
                if matches!(i, 2 | 7 | 9) {
                    let (sep, _) = ui.allocate_exact_size(vec2(38.0, 9.0), Sense::hover());
                    ui.painter().line_segment([sep.left_center() + vec2(6.0, 0.0), sep.right_center() - vec2(6.0, 0.0)], Stroke::new(1.0, LINE));
                }
                let t = *tool;
                if pro::icon_button(ui, 38.0, self.tool == t, tip, |p, r, c| tool_icon(p, r, t, c)).clicked() {
                    self.finish_pen();
                    self.tool = t;
                }
            }
            ui.add_space(12.0);
            let (rect, _) = ui.allocate_exact_size(vec2(38.0, 44.0), Sense::hover());
            let back = Rect::from_min_size(rect.min + vec2(14.0, 16.0), Vec2::splat(20.0));
            let front = Rect::from_min_size(rect.min + vec2(4.0, 6.0), Vec2::splat(20.0));
            ui.painter().rect(back, CornerRadius::same(5), PANEL, Stroke::new(4.0, self.style.stroke.unwrap_or(FAINT)), StrokeKind::Inside);
            ui.painter().rect(front, CornerRadius::same(5), self.style.fill.unwrap_or(PANEL), Stroke::new(1.0, INK), StrokeKind::Outside);
            if self.style.fill.is_none() {
                ui.painter().line_segment([front.left_bottom(), front.right_top()], Stroke::new(1.5, RED));
            }
        });
    }

    fn dock(&mut self, root: &mut Ui) {
        egui::Panel::right("dock").resizable(false).exact_size(300.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                match self.sel {
                    Some(i) if i < self.doc.items.len() => {
                        pro::dock_panel(ui, "transform", "Transformation", |ui| self.transform_panel(ui, i));
                        pro::dock_panel(ui, "appearance", "Apparence", |ui| appearance(ui, &mut self.doc.items[i]));
                        if !matches!(self.doc.items[i].geom, Geom::Ellipse { .. } | Geom::Path { closed: false, .. }) {
                            pro::dock_panel(ui, "shape", shape_title(&self.doc.items[i].geom), |ui| shape_props(ui, &mut self.doc.items[i]));
                        }
                    }
                    _ => {
                        pro::dock_panel(ui, "style", "Style des nouveaux objets", |ui| {
                            ui.spacing_mut().item_spacing.y = 6.0;
                            color_toggle(ui, "Fond", &mut self.style.fill);
                            color_toggle(ui, "Contour", &mut self.style.stroke);
                            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                            ui.add(egui::Slider::new(&mut self.style.stroke_w, 0.0..=50.0).suffix(" px"));
                        });
                        pro::dock_panel(ui, "artboard", "Plan de travail", |ui| {
                            ui.spacing_mut().item_spacing.y = 6.0;
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("L").color(MUTED));
                                ui.add(egui::DragValue::new(&mut self.doc.width).range(1.0..=20000.0).suffix(" px"));
                                ui.label(RichText::new("H").color(MUTED));
                                ui.add(egui::DragValue::new(&mut self.doc.height).range(1.0..=20000.0).suffix(" px"));
                            });
                            let mut bg = self.doc.background.map(c32);
                            color_toggle(ui, "Fond", &mut bg);
                            self.doc.background = bg.map(|c| c.to_srgba_unmultiplied());
                        });
                    }
                }
                pro::dock_panel(ui, "layers", "Calques", |ui| self.layers(ui));
            });
        });
    }

    fn transform_panel(&mut self, ui: &mut Ui, i: usize) {
        ui.spacing_mut().item_spacing.y = 6.0;
        ui.add(egui::TextEdit::singleline(&mut self.doc.items[i].name).desired_width(f32::INFINITY));
        let b = self.doc.items[i].bbox();
        let (mut x, mut y, mut w, mut h) = (b.min[0], b.min[1], b.max[0] - b.min[0], b.max[1] - b.min[1]);
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new("X").color(MUTED));
            changed |= ui.add(egui::DragValue::new(&mut x).speed(1.0).max_decimals(1)).changed();
            ui.label(RichText::new("Y").color(MUTED));
            changed |= ui.add(egui::DragValue::new(&mut y).speed(1.0).max_decimals(1)).changed();
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("L").color(MUTED));
            changed |= ui.add(egui::DragValue::new(&mut w).range(0.1..=50000.0).speed(1.0).max_decimals(1)).changed();
            ui.label(RichText::new("H").color(MUTED));
            changed |= ui.add(egui::DragValue::new(&mut h).range(0.1..=50000.0).speed(1.0).max_decimals(1)).changed();
        });
        if changed {
            self.set_bbox(i, BBox { min: [x, y], max: [x + w, y + h] });
        }
    }

    fn layers(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 3.0;
        if self.doc.items.is_empty() {
            ui.label(RichText::new("Aucun objet. Choisis un outil et dessine sur le plan de travail.").size(12.5).color(FAINT));
            return;
        }
        let mut select = None;
        let mut toggle = None;
        for i in (0..self.doc.items.len()).rev() {
            let on = self.sel == Some(i);
            let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
            let p = ui.painter();
            if on {
                p.rect_filled(row, CornerRadius::same(8), BLUE.gamma_multiply(0.24));
            } else if resp.hovered() {
                p.rect_filled(row, CornerRadius::same(8), Color32::from_white_alpha(8));
            }
            let it = &self.doc.items[i];
            let eye = Rect::from_center_size(row.left_center() + vec2(16.0, 0.0), Vec2::splat(18.0));
            let ec = if it.visible { INK } else { FAINT };
            p.add(Shape::convex_polygon(
                vec![eye.left_center() + vec2(1.0, 0.0), eye.center() - vec2(0.0, 4.5), eye.right_center() - vec2(1.0, 0.0), eye.center() + vec2(0.0, 4.5)],
                Color32::TRANSPARENT,
                Stroke::new(1.3, ec),
            ));
            if it.visible {
                p.circle_filled(eye.center(), 2.2, ec);
            }
            let sw = Rect::from_center_size(row.left_center() + vec2(42.0, 0.0), Vec2::splat(18.0));
            p.rect(sw, CornerRadius::same(4), it.fill.map(c32).unwrap_or(Color32::TRANSPARENT), Stroke::new(1.5, it.stroke.map(c32).unwrap_or(LINE)), StrokeKind::Inside);
            p.text(row.left_center() + vec2(60.0, 0.0), Align2::LEFT_CENTER, &it.name, if on { semibold(13.0) } else { FontId::proportional(13.0) }, if on { INK_STRONG } else { INK });
            if ui.interact(eye.expand(4.0), ui.make_persistent_id(("veye", i)), Sense::click()).clicked() {
                toggle = Some(i);
            } else if resp.clicked() {
                select = Some(i);
            }
        }
        if let Some(i) = toggle {
            self.doc.items[i].visible = !self.doc.items[i].visible;
        }
        if let Some(i) = select {
            self.finish_pen();
            self.sel = Some(i);
            self.tool = Tool::Select;
        }
    }

    fn canvas(&mut self, root: &mut Ui, name: &str) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let tab_h = 32.0;
            let tab_bar = Rect::from_min_max(full.min, pos2(full.right(), full.top() + tab_h));
            let p = ui.painter();
            p.rect_filled(tab_bar, CornerRadius::ZERO, PANEL);
            let label = format!("{name} @ {:.0} %{}", self.zoom * 100.0, if self.doc != self.saved { " •" } else { "" });
            let g = p.layout_no_wrap(label, FontId::proportional(12.5), INK);
            let tab = Rect::from_min_size(tab_bar.min + vec2(8.0, 4.0), vec2(g.size().x + 28.0, tab_h - 4.0));
            p.rect_filled(tab, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, PASTEBOARD);
            p.galley(tab.left_center() + vec2(14.0, -g.size().y / 2.0), g, INK);

            let area = Rect::from_min_max(pos2(full.left(), tab_bar.bottom()), full.max);
            let resp = ui.allocate_rect(area, Sense::click_and_drag());
            let (w, h) = (self.doc.width, self.doc.height);
            if self.fit_pending {
                self.zoom = ((area.width() - 100.0) / w).min((area.height() - 100.0) / h).clamp(0.02, 4.0);
                self.offset = Vec2::ZERO;
                self.fit_pending = false;
            }
            let (scroll, pinch, space, pointer, pressed, released, middle, delta, shift, alt) = ui.input(|i| {
                (
                    i.smooth_scroll_delta.y,
                    i.zoom_delta(),
                    i.key_down(Key::Space),
                    i.pointer.hover_pos(),
                    i.pointer.primary_pressed(),
                    i.pointer.primary_released(),
                    i.pointer.middle_down(),
                    i.pointer.delta(),
                    i.modifiers.shift,
                    i.modifiers.alt,
                )
            });
            let origin = |zoom: f32, offset: Vec2| area.center() + offset - vec2(w, h) * zoom / 2.0;
            let zoom_at = |zoom: &mut f32, offset: &mut Vec2, f: f32, at: Pos2| {
                let docp = (at - origin(*zoom, *offset)) / *zoom;
                let nz = (*zoom * f).clamp(0.02, 64.0);
                *offset = at.to_vec2() - docp * nz + vec2(w, h) * nz / 2.0 - area.center().to_vec2();
                *zoom = nz;
            };
            if resp.hovered() {
                let f = pinch * (scroll / 400.0).exp();
                if (f - 1.0).abs() > 1e-4
                    && let Some(at) = pointer
                {
                    zoom_at(&mut self.zoom, &mut self.offset, f, at);
                }
            }
            let panning = (resp.dragged() && (self.tool == Tool::Hand || space)) || (resp.hovered() && middle);
            if panning {
                self.offset += delta;
            }
            let o = origin(self.zoom, self.offset);
            let zoom = self.zoom;
            let to_doc = |p: Pos2| -> P { [(p.x - o.x) / zoom, (p.y - o.y) / zoom] };
            let to_scr = |p: P| -> Pos2 { pos2(o.x + p[0] * zoom, o.y + p[1] * zoom) };
            let tol = 6.0 / zoom;
            self.cursor = pointer.filter(|p| area.contains(*p)).map(to_doc);

            if !panning && !space && let Some(sp) = pointer {
                let pt = to_doc(sp);
                if pressed && resp.hovered() {
                    match self.tool {
                        Tool::Zoom => zoom_at(&mut self.zoom, &mut self.offset, if alt { 0.8 } else { 1.25 }, sp),
                        Tool::Picker => {
                            if let (Some(src), Some(dst)) = (self.doc.hit(pt, tol), self.sel) {
                                let (f, s, sw) = (self.doc.items[src].fill, self.doc.items[src].stroke, self.doc.items[src].stroke_w);
                                let d = &mut self.doc.items[dst];
                                d.fill = f;
                                d.stroke = s;
                                d.stroke_w = sw;
                            } else if let Some(src) = self.doc.hit(pt, tol) {
                                let it = &self.doc.items[src];
                                self.style.fill = it.fill.map(c32);
                                self.style.stroke = it.stroke.map(c32);
                                self.style.stroke_w = it.stroke_w;
                            }
                        }
                        _ => self.press(pt, tol),
                    }
                } else if resp.dragged() || matches!(self.drag, Drag::PenHandle { .. }) && ui.input(|i| i.pointer.primary_down()) {
                    self.drag_to(pt, shift);
                }
            }
            if released {
                if let Drag::Create { .. } = self.drag
                    && let Some(i) = self.sel
                {
                    let b = self.doc.items[i].bbox();
                    if b.max[0] - b.min[0] < 2.0 && b.max[1] - b.min[1] < 2.0 {
                        self.doc.items.remove(i);
                        self.sel = None;
                        self.undo.pop();
                    }
                }
                self.drag = Drag::None;
            }

            self.render(ui.ctx());
            let art = Rect::from_min_size(o, vec2(w, h) * zoom);
            let painter = ui.painter_at(area);
            painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(120) }.as_shape(art, CornerRadius::ZERO));
            if self.doc.background.is_none_or(|c| c[3] < 255) {
                painter.image(self.checker.id(), art, Rect::from_min_max(pos2(0.0, 0.0), pos2(art.width() / 16.0, art.height() / 16.0)), Color32::WHITE);
            }
            if let Some(t) = &self.tex {
                painter.image(t.id(), art, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            if self.tool == Tool::Select
                && let Some(hp) = pointer
                && let Some(hi) = self.doc.hit(to_doc(hp), tol)
                && Some(hi) != self.sel
            {
                let b = self.doc.items[hi].bbox();
                painter.rect_stroke(Rect::from_min_max(to_scr(b.min), to_scr(b.max)), CornerRadius::ZERO, Stroke::new(1.0, BLUE.gamma_multiply(0.7)), StrokeKind::Middle);
            }
            self.overlay(&painter, &to_scr);

            if resp.hovered() {
                ui.ctx().set_cursor_icon(match self.tool {
                    _ if panning || space => egui::CursorIcon::Grabbing,
                    Tool::Hand => egui::CursorIcon::Grab,
                    Tool::Select => egui::CursorIcon::Default,
                    Tool::Text => egui::CursorIcon::Text,
                    Tool::Zoom => if alt { egui::CursorIcon::ZoomOut } else { egui::CursorIcon::ZoomIn },
                    _ => egui::CursorIcon::Crosshair,
                });
            }
        });
    }

    fn press(&mut self, p: P, tol: f32) {
        match self.tool {
            Tool::Select => {
                if let Some(i) = self.sel {
                    let corners = self.doc.items[i].bbox().corners();
                    if let Some(k) = corners.iter().position(|c| dist(*c, p) <= tol * 1.5) {
                        self.checkpoint();
                        self.drag = Drag::Scale { anchor: corners[(k + 2) % 4], orig: self.doc.items[i].clone(), from: corners[k] };
                        return;
                    }
                    if let Geom::Path { nodes, .. } = &self.doc.items[i].geom {
                        let hit = nodes.iter().enumerate().find_map(|(ni, n)| {
                            [(n.p, 0u8), (n.cin, 1), (n.cout, 2)].into_iter().find(|(q, _)| dist(*q, p) <= tol).map(|(_, part)| (ni, part))
                        });
                        if let Some((node, part)) = hit {
                            self.checkpoint();
                            self.drag = Drag::NodePart { node, part };
                            return;
                        }
                    }
                }
                self.sel = self.doc.hit(p, tol);
                if self.sel.is_some() {
                    self.checkpoint();
                    self.drag = Drag::Move { last: p };
                }
            }
            Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Star | Tool::Line => {
                self.checkpoint();
                let geom = match self.tool {
                    Tool::Rect => Geom::Rect { x: p[0], y: p[1], w: 0.0, h: 0.0, r: 0.0 },
                    Tool::Ellipse => Geom::Ellipse { cx: p[0], cy: p[1], rx: 0.0, ry: 0.0 },
                    Tool::Line => Geom::Path { nodes: vec![Node::corner(p), Node::corner(p)], closed: false },
                    _ => Geom::Path { nodes: polygon(p[0], p[1], 0.1, self.sides, None), closed: true },
                };
                let it = self.new_item(geom);
                self.doc.items.push(it);
                self.sel = Some(self.doc.items.len() - 1);
                self.drag = Drag::Create { start: p };
            }
            Tool::Text => {
                self.checkpoint();
                let it = self.new_item(Geom::Text { x: p[0], y: p[1], text: "Texte".into(), size: 64.0, family: "Segoe UI".into(), bold: false });
                self.doc.items.push(it);
                self.sel = Some(self.doc.items.len() - 1);
                self.tool = Tool::Select;
            }
            Tool::Pen => {
                self.checkpoint();
                let idx = match self.pen {
                    Some(i) => i,
                    None => {
                        let it = self.new_item(Geom::Path { nodes: Vec::new(), closed: false });
                        self.doc.items.push(it);
                        let i = self.doc.items.len() - 1;
                        self.pen = Some(i);
                        i
                    }
                };
                self.sel = Some(idx);
                if let Geom::Path { nodes, closed } = &mut self.doc.items[idx].geom {
                    if nodes.len() >= 2 && dist(nodes[0].p, p) <= tol * 1.5 {
                        *closed = true;
                        if self.doc.items[idx].fill.is_none() {
                            self.doc.items[idx].fill = self.style.fill.map(|c| c.to_array());
                        }
                        self.pen = None;
                        self.drag = Drag::None;
                        return;
                    }
                    nodes.push(Node::corner(p));
                    self.drag = Drag::PenHandle { node: nodes.len() - 1 };
                }
            }
            Tool::Hand | Tool::Zoom | Tool::Picker => {}
        }
    }

    fn drag_to(&mut self, p: P, shift: bool) {
        let Some(i) = self.sel else { return };
        let tool = self.tool;
        let (sides, ratio) = (self.sides, self.star_ratio);
        let it = &mut self.doc.items[i];
        match &mut self.drag {
            Drag::Move { last } => {
                it.translate([p[0] - last[0], p[1] - last[1]]);
                *last = p;
            }
            Drag::Create { start } => {
                let (mut dx, mut dy) = (p[0] - start[0], p[1] - start[1]);
                if shift && tool != Tool::Line {
                    let m = dx.abs().max(dy.abs());
                    dx = m * dx.signum();
                    dy = m * dy.signum();
                }
                if shift && tool == Tool::Line {
                    let a = dy.atan2(dx);
                    let snap = (a / (std::f32::consts::PI / 4.0)).round() * std::f32::consts::PI / 4.0;
                    let len = (dx * dx + dy * dy).sqrt();
                    dx = snap.cos() * len;
                    dy = snap.sin() * len;
                }
                let (x, y) = (start[0].min(start[0] + dx), start[1].min(start[1] + dy));
                let (w, h) = (dx.abs(), dy.abs());
                it.geom = match (&it.geom, tool) {
                    (Geom::Rect { r, .. }, _) => Geom::Rect { x, y, w, h, r: *r },
                    (Geom::Ellipse { .. }, _) => Geom::Ellipse { cx: x + w / 2.0, cy: y + h / 2.0, rx: w / 2.0, ry: h / 2.0 },
                    (_, Tool::Line) => Geom::Path { nodes: vec![Node::corner(*start), Node::corner([start[0] + dx, start[1] + dy])], closed: false },
                    _ => {
                        let r = (dx * dx + dy * dy).sqrt();
                        let inner = (tool == Tool::Star).then_some(ratio);
                        Geom::Path { nodes: polygon(start[0], start[1], r.max(0.1), sides, inner), closed: true }
                    }
                };
            }
            Drag::Scale { anchor, orig, from } => {
                let a = *anchor;
                let sx = safe_div(p[0] - a[0], from[0] - a[0]);
                let mut sy = safe_div(p[1] - a[1], from[1] - a[1]);
                if shift || matches!(orig.geom, Geom::Text { .. }) {
                    sy = sx.abs() * if sy < 0.0 { -1.0 } else { 1.0 };
                }
                let mut next = orig.clone();
                next.map(|q| [a[0] + (q[0] - a[0]) * sx, a[1] + (q[1] - a[1]) * sy], sx, sy);
                *it = next;
            }
            Drag::NodePart { node, part } => {
                if let Geom::Path { nodes, .. } = &mut it.geom {
                    let n = &mut nodes[*node];
                    match part {
                        0 => {
                            let d = [p[0] - n.p[0], p[1] - n.p[1]];
                            n.p = p;
                            n.cin = [n.cin[0] + d[0], n.cin[1] + d[1]];
                            n.cout = [n.cout[0] + d[0], n.cout[1] + d[1]];
                        }
                        1 => {
                            n.cin = p;
                            if !shift {
                                n.cout = [2.0 * n.p[0] - p[0], 2.0 * n.p[1] - p[1]];
                            }
                        }
                        _ => {
                            n.cout = p;
                            if !shift {
                                n.cin = [2.0 * n.p[0] - p[0], 2.0 * n.p[1] - p[1]];
                            }
                        }
                    }
                }
            }
            Drag::PenHandle { node } => {
                if let Geom::Path { nodes, .. } = &mut it.geom {
                    let n = &mut nodes[*node];
                    n.cout = p;
                    n.cin = [2.0 * n.p[0] - p[0], 2.0 * n.p[1] - p[1]];
                }
            }
            Drag::None => {}
        }
    }

    fn overlay(&self, painter: &Painter, to_scr: &impl Fn(P) -> Pos2) {
        let Some(i) = self.sel else { return };
        let Some(it) = self.doc.items.get(i) else { return };
        let b = it.bbox();
        let r = Rect::from_min_max(to_scr(b.min), to_scr(b.max));
        painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, BLUE), StrokeKind::Middle);
        if let Geom::Path { nodes, .. } = &it.geom {
            for n in nodes {
                for c in [n.cin, n.cout] {
                    if c != n.p {
                        painter.line_segment([to_scr(n.p), to_scr(c)], Stroke::new(1.0, BLUE.gamma_multiply(0.8)));
                        painter.circle(to_scr(c), 3.5, Color32::WHITE, Stroke::new(1.0, BLUE));
                    }
                }
                let q = Rect::from_center_size(to_scr(n.p), Vec2::splat(7.0));
                painter.rect(q, CornerRadius::ZERO, Color32::WHITE, Stroke::new(1.0, BLUE), StrokeKind::Inside);
            }
        }
        if self.pen.is_none() {
            for c in b.corners() {
                let q = Rect::from_center_size(to_scr(c), Vec2::splat(9.0));
                painter.rect(q, CornerRadius::same(2), Color32::WHITE, Stroke::new(1.2, BLUE), StrokeKind::Inside);
            }
            let size = format!("{:.0} × {:.0}", b.max[0] - b.min[0], b.max[1] - b.min[1]);
            let g = painter.layout_no_wrap(size, FontId::proportional(11.5), Color32::WHITE);
            let tag = Rect::from_center_size(pos2(r.center().x, r.bottom() + 18.0), g.size() + vec2(14.0, 6.0));
            painter.rect_filled(tag, CornerRadius::same(6), BLUE);
            painter.galley(tag.center() - g.size() / 2.0, g, Color32::WHITE);
        }
    }

    fn new_doc_sheet(&mut self, ctx: &egui::Context) {
        let Some(mut nd) = self.new_doc.take() else { return };
        let mut create = false;
        let mut cancel = false;
        let keep = pro::sheet(ctx, "new", 740.0, |ui| {
            ui.label(RichText::new("Nouveau plan de travail").font(semibold(20.0)).color(INK_STRONG));
            ui.add_space(14.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(450.0);
                    for (ri, row) in PRESETS.chunks(3).enumerate() {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            for (k, (name, kind, w, h)) in row.iter().enumerate() {
                                let idx = ri * 3 + k;
                                if preset_card(ui, name, &format!("{w} × {h} · {kind}"), *w, *h, nd.preset == idx, 142.0).clicked() {
                                    nd.preset = idx;
                                    nd.w = *w;
                                    nd.h = *h;
                                }
                            }
                        });
                        ui.add_space(10.0);
                    }
                });
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Largeur").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut nd.w).range(1.0..=20000.0).suffix(" px"));
                    ui.label(RichText::new("Hauteur").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut nd.h).range(1.0..=20000.0).suffix(" px"));
                    ui.add_space(8.0);
                    ui.checkbox(&mut nd.transparent, "Fond transparent");
                });
            });
            ui.add_space(14.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if pro::primary(ui, "Créer").clicked() {
                    create = true;
                }
                if pro::secondary(ui, "Annuler").clicked() {
                    cancel = true;
                }
            });
        });
        if create {
            let mut d = Doc::new(nd.w, nd.h);
            if nd.transparent {
                d.background = None;
            }
            self.set_doc(d, None);
        } else if keep && !cancel {
            self.new_doc = Some(nd);
        }
    }
}

fn separator(ui: &mut Ui) {
    ui.add_space(6.0);
    let (sep, _) = ui.allocate_exact_size(vec2(1.0, 22.0), Sense::hover());
    ui.painter().rect_filled(sep, CornerRadius::ZERO, LINE);
    ui.add_space(6.0);
}

fn style_swatches(ui: &mut Ui, style: &mut Style) {
    ui.label(RichText::new("Fond").size(12.5).color(MUTED));
    color_button(ui, &mut style.fill);
    ui.label(RichText::new("Contour").size(12.5).color(MUTED));
    color_button(ui, &mut style.stroke);
    ui.add(egui::DragValue::new(&mut style.stroke_w).range(0.0..=50.0).suffix(" px"));
}

fn color_button(ui: &mut Ui, c: &mut Option<Color32>) {
    let mut on = c.is_some();
    if ui.checkbox(&mut on, "").changed() {
        *c = on.then_some(c.unwrap_or(Color32::BLACK));
    }
    if let Some(col) = c {
        ui.color_edit_button_srgba(col);
    }
}

fn color_toggle(ui: &mut Ui, label: &str, c: &mut Option<Color32>) {
    let open_id = ui.make_persistent_id(("picker-open", label));
    let mut open = ui.ctx().data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
    ui.horizontal(|ui| {
        let mut on = c.is_some();
        if ui.checkbox(&mut on, RichText::new(label).color(INK)).changed() {
            *c = on.then_some(c.unwrap_or(Color32::BLACK));
            open = on;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (r, resp) = ui.allocate_exact_size(vec2(44.0, 24.0), Sense::click());
            match c {
                Some(col) => {
                    ui.painter().rect(r, CornerRadius::same(6), *col, Stroke::new(1.0, if open { BLUE } else { LINE }), StrokeKind::Inside);
                    ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}", col.r(), col.g(), col.b())).monospace().size(12.0).color(MUTED));
                }
                None => {
                    ui.painter().rect(r, CornerRadius::same(6), FIELD, Stroke::new(1.0, LINE), StrokeKind::Inside);
                    ui.painter().line_segment([r.left_bottom() + vec2(4.0, -4.0), r.right_top() + vec2(-4.0, 4.0)], Stroke::new(1.5, RED));
                }
            }
            if resp.on_hover_text("Modifier la couleur").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                if c.is_none() {
                    *c = Some(Color32::BLACK);
                }
                open = !open;
            }
        });
    });
    if open
        && let Some(col) = c
    {
        ui.add_space(4.0);
        let w = ui.available_width();
        ui.push_id(("picker", label), |ui| {
            pro::color_picker(ui, col, w);
        });
        ui.add_space(6.0);
    }
    ui.ctx().data_mut(|d| d.insert_temp(open_id, open));
}

fn appearance(ui: &mut Ui, it: &mut Item) {
    ui.spacing_mut().item_spacing.y = 6.0;
    let mut fill = it.fill.map(c32);
    color_toggle(ui, "Fond", &mut fill);
    it.fill = fill.map(|c| c.to_srgba_unmultiplied());
    let mut stroke = it.stroke.map(c32);
    color_toggle(ui, "Contour", &mut stroke);
    it.stroke = stroke.map(|c| c.to_srgba_unmultiplied());
    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
    if it.stroke.is_some() {
        ui.label(RichText::new("Épaisseur du contour").size(12.5).color(MUTED));
        ui.add(egui::Slider::new(&mut it.stroke_w, 0.0..=50.0).suffix(" px"));
    }
    ui.label(RichText::new("Opacité").size(12.5).color(MUTED));
    let mut op = it.opacity * 100.0;
    if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).suffix(" %")).changed() {
        it.opacity = op / 100.0;
    }
}

fn shape_title(g: &Geom) -> &'static str {
    match g {
        Geom::Rect { .. } => "Rectangle",
        Geom::Text { .. } => "Texte",
        Geom::Path { .. } => "Tracé",
        Geom::Ellipse { .. } => "Ellipse",
    }
}

fn shape_props(ui: &mut Ui, it: &mut Item) {
    ui.spacing_mut().item_spacing.y = 6.0;
    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
    match &mut it.geom {
        Geom::Rect { w, h, r, .. } => {
            ui.label(RichText::new("Arrondi des coins").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(r, 0.0..=w.min(*h) / 2.0).suffix(" px"));
        }
        Geom::Path { nodes, closed } => {
            ui.label(RichText::new(format!("{} points", nodes.len())).size(12.5).color(MUTED));
            ui.checkbox(closed, "Tracé fermé");
        }
        Geom::Text { text, size, family, bold, .. } => {
            ui.add(egui::TextEdit::multiline(text).desired_rows(3).desired_width(f32::INFINITY));
            ui.label(RichText::new("Taille").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(size, 4.0..=600.0).logarithmic(true).suffix(" px"));
            egui::ComboBox::from_id_salt("font").selected_text(family.as_str()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                for f in FONTS {
                    ui.selectable_value(family, f.to_string(), f);
                }
            });
            ui.checkbox(bold, "Gras");
        }
        Geom::Ellipse { .. } => {}
    }
}

fn preset_card(ui: &mut Ui, name: &str, detail: &str, w: f32, h: f32, selected: bool, width: f32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 132.0), Sense::click());
    let p = ui.painter();
    p.rect(
        r,
        CornerRadius::same(12),
        if resp.hovered() { Color32::from_rgb(0x2f, 0x2f, 0x35) } else { Color32::from_rgb(0x27, 0x27, 0x2c) },
        Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { BLUE } else { LINE }),
        StrokeKind::Inside,
    );
    let area = Rect::from_center_size(r.center_top() + vec2(0.0, 44.0), vec2(width - 40.0, 56.0));
    let s = (area.width() / w).min(area.height() / h);
    let shape = Rect::from_center_size(area.center(), vec2(w, h) * s);
    p.rect(shape, CornerRadius::same(3), Color32::from_white_alpha(18), Stroke::new(1.2, if selected { BLUE } else { MUTED }), StrokeKind::Inside);
    p.text(pos2(r.center().x, r.bottom() - 36.0), Align2::CENTER_CENTER, name, semibold(13.0), INK_STRONG);
    p.text(pos2(r.center().x, r.bottom() - 17.0), Align2::CENTER_CENTER, detail, FontId::proportional(11.0), MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn dist(a: P, b: P) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn safe_div(a: f32, b: f32) -> f32 {
    if b.abs() < 1e-3 { 1.0 } else { a / b }
}

fn checker(ctx: &egui::Context) -> TextureHandle {
    let mut px = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16 {
        for x in 0..16 {
            let v = if (x / 8 + y / 8) % 2 == 0 { 0xcc } else { 0xff };
            px.extend_from_slice(&[v, v, v, 255]);
        }
    }
    ctx.load_texture(
        "checker",
        egui::ColorImage::from_rgba_unmultiplied([16, 16], &px),
        TextureOptions { wrap_mode: egui::TextureWrapMode::Repeat, ..TextureOptions::NEAREST },
    )
}

fn align_icon(p: &Painter, r: Rect, axis: usize, to: f32, c: Color32) {
    let s = Stroke::new(1.6, c);
    if axis == 0 {
        let x = r.left() + r.width() * to;
        p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], s);
        let w1 = r.width() * 0.7;
        let w2 = r.width() * 0.45;
        for (k, w) in [w1, w2].iter().enumerate() {
            let y = r.top() + r.height() * (0.25 + 0.42 * k as f32);
            let x0 = x - w * to;
            p.rect_filled(Rect::from_min_size(pos2(x0, y), vec2(*w, r.height() * 0.22)), CornerRadius::same(2), c);
        }
    } else {
        let y = r.top() + r.height() * to;
        p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], s);
        let h1 = r.height() * 0.7;
        let h2 = r.height() * 0.45;
        for (k, h) in [h1, h2].iter().enumerate() {
            let x = r.left() + r.width() * (0.25 + 0.42 * k as f32);
            let y0 = y - h * to;
            p.rect_filled(Rect::from_min_size(pos2(x, y0), vec2(r.width() * 0.22, *h)), CornerRadius::same(2), c);
        }
    }
}

fn tool_icon(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    let s = Stroke::new(1.8, c);
    match tool {
        Tool::Select => {
            p.add(Shape::convex_polygon(vec![at(0.2, 0.0), at(0.85, 0.6), at(0.52, 0.62), at(0.33, 1.0)], c, Stroke::NONE));
        }
        Tool::Pen => {
            p.add(Shape::convex_polygon(vec![at(0.5, 0.0), at(0.85, 0.55), at(0.5, 1.0), at(0.15, 0.55)], Color32::TRANSPARENT, s));
            p.line_segment([at(0.5, 0.35), at(0.5, 0.95)], Stroke::new(1.4, c));
        }
        Tool::Rect => {
            p.rect_stroke(Rect::from_min_max(at(0.05, 0.15), at(0.95, 0.85)), CornerRadius::same(2), s, StrokeKind::Middle);
        }
        Tool::Ellipse => {
            p.circle_stroke(r.center(), r.width() * 0.45, s);
        }
        Tool::Polygon => {
            let pts: Vec<Pos2> = polygon(0.0, 0.0, 1.0, 6, None).iter().map(|n| r.center() + vec2(n.p[0], n.p[1]) * r.width() * 0.5).collect();
            p.add(Shape::closed_line(pts, s));
        }
        Tool::Star => {
            let pts: Vec<Pos2> = polygon(0.0, 0.0, 1.0, 5, Some(0.45)).iter().map(|n| r.center() + vec2(n.p[0], n.p[1]) * r.width() * 0.52).collect();
            p.add(Shape::closed_line(pts, s));
        }
        Tool::Line => {
            p.line_segment([at(0.05, 0.95), at(0.95, 0.05)], Stroke::new(2.0, c));
        }
        Tool::Text => {
            p.line_segment([at(0.1, 0.1), at(0.9, 0.1)], Stroke::new(2.2, c));
            p.line_segment([at(0.5, 0.1), at(0.5, 0.95)], Stroke::new(2.2, c));
        }
        Tool::Picker => {
            p.line_segment([at(0.08, 0.92), at(0.62, 0.38)], Stroke::new(2.3, c));
            p.circle_filled(at(0.75, 0.25), r.width() * 0.2, c);
        }
        Tool::Hand => {
            for (k, h) in [0.55, 0.75, 0.8, 0.7].iter().enumerate() {
                let x = 0.22 + k as f32 * 0.19;
                p.line_segment([at(x, 0.62), at(x, 0.62 - h * 0.6)], Stroke::new(2.4, c));
            }
            p.add(Shape::convex_polygon(vec![at(0.12, 0.6), at(0.85, 0.6), at(0.75, 1.0), at(0.25, 1.0)], c, Stroke::NONE));
        }
        Tool::Zoom => {
            p.circle_stroke(at(0.42, 0.42), r.width() * 0.32, s);
            p.line_segment([at(0.66, 0.66), at(0.98, 0.98)], Stroke::new(2.4, c));
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Vector")
            .with_icon(Arc::new(studio_ui::app_window_icon("vector")))
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1000.0, 640.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Vector", options, Box::new(|cc| Ok(Box::new(VectorApp::new(cc)))))
}
