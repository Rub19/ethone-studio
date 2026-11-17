#![windows_subsystem = "windows"]

mod optlib;
mod reglages;
mod outils;
mod engine;
mod panels;
mod render;
mod tutos;

use eframe::egui::{self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, Painter, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2, pos2, vec2};
use engine::{Mark, MarkKind};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use studio_ui::pro::{self, BLUE, CHROME, FAINT, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, semibold};

pub const ACCENT: Color32 = Color32::from_rgb(235, 50, 60);
const PX: f32 = 96.0 / 72.0;
const GAP: f32 = 18.0;
static REV: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Hand,
    Highlight,
    Underline,
    Strike,
    Note,
    TextBox,
    Rect,
    Ellipse,
    Line,
    Arrow,
    Ink,
    Stamp,
    Sign,
    Date,
    Redact,
    Measure,
}

pub const TOOLS: [(Tool, &str); 16] = [
    (Tool::Hand, "Main (H)"),
    (Tool::Highlight, "Surligner"),
    (Tool::Underline, "Souligner"),
    (Tool::Strike, "Barrer"),
    (Tool::Note, "Note"),
    (Tool::TextBox, "Zone de texte"),
    (Tool::Rect, "Rectangle"),
    (Tool::Ellipse, "Ellipse"),
    (Tool::Line, "Ligne"),
    (Tool::Arrow, "Flèche"),
    (Tool::Ink, "Dessin à main levée"),
    (Tool::Stamp, "Tampon"),
    (Tool::Sign, "Signature"),
    (Tool::Date, "Date du jour"),
    (Tool::Redact, "Caviardage"),
    (Tool::Measure, "Mesure"),
];

pub const STAMPS: [(&str, [u8; 3]); 10] = [
    ("Approuvé", [30, 140, 60]),
    ("Confidentiel", [200, 30, 40]),
    ("Brouillon", [90, 90, 200]),
    ("Rejeté", [200, 30, 40]),
    ("Final", [30, 140, 60]),
    ("Payé", [30, 140, 60]),
    ("Urgent", [220, 90, 0]),
    ("Copie", [90, 90, 200]),
    ("Reçu", [30, 110, 200]),
    ("Pour information", [90, 90, 90]),
];

pub struct Opened {
    pub path: Option<PathBuf>,
    pub name: String,
    pub bytes: Arc<Vec<u8>>,
    pub pdf: hayro::hayro_syntax::Pdf,
    pub doc: lopdf::Document,
    pub sizes: Vec<(f32, f32)>,
    pub undo: Vec<Arc<Vec<u8>>>,
    pub redo: Vec<Arc<Vec<u8>>>,
    pub dirty: bool,
    pub rev: u64,
    pub texts: Vec<Option<String>>,
}

impl Opened {
    pub fn new(bytes: Vec<u8>, path: Option<PathBuf>, name: String, password: &str) -> engine::R<Self> {
        let mut doc = engine::load(&bytes, password)?;
        let bytes = if password.is_empty() { bytes } else { engine::save(&mut doc)? };
        let bytes = Arc::new(bytes);
        let pdf = render::open(bytes.clone()).ok_or("Ce PDF ne peut pas être affiché")?;
        let mut o = Self { path, name, bytes, pdf, doc, sizes: Vec::new(), undo: Vec::new(), redo: Vec::new(), dirty: !password.is_empty(), rev: 0, texts: Vec::new() };
        o.refresh();
        Ok(o)
    }

    fn refresh(&mut self) {
        self.sizes = engine::pages(&self.doc).iter().map(|p| {
            let (b, r) = engine::geometry(&self.doc, *p);
            engine::view_size(b, r)
        }).collect();
        self.rev = REV.fetch_add(1, Ordering::Relaxed);
        self.texts = vec![None; self.sizes.len()];
    }

    pub fn set_bytes(&mut self, bytes: Arc<Vec<u8>>) -> engine::R<()> {
        let doc = engine::load(&bytes, "")?;
        let pdf = render::open(bytes.clone()).ok_or("Rendu impossible")?;
        self.doc = doc;
        self.pdf = pdf;
        self.bytes = bytes;
        self.refresh();
        Ok(())
    }

    pub fn n(&self) -> usize {
        self.sizes.len()
    }

    pub fn text(&mut self, i: usize) -> &str {
        if self.texts.get(i).is_some_and(|t| t.is_none()) {
            self.texts[i] = Some(engine::page_text(&self.doc, i));
        }
        self.texts.get(i).and_then(|t| t.as_deref()).unwrap_or("")
    }
}

pub enum Job {
    Progress(usize, usize),
    Ocr(Vec<(usize, String, Vec<(String, [f32; 4])>)>, bool),
    Compare(Vec<f32>),
    Err(String),
}

pub struct Folio {
    pub od: Option<Opened>,
    tex: HashMap<usize, (u64, f32, TextureHandle)>,
    thumbs: HashMap<usize, (u64, TextureHandle)>,
    pub zoom: f32,
    pub fit: Option<u8>,
    pub page: usize,
    pub goto: Option<usize>,
    pub sel: Vec<usize>,
    pub tool: Tool,
    pub color: [u8; 3],
    pub width: f32,
    pub text_size: f32,
    pub stamp: usize,
    drag: Option<(usize, [f32; 2], [f32; 2])>,
    ink: Vec<[f32; 2]>,
    typing: Option<(usize, MarkKind, Vec<[f32; 2]>, String)>,
    pub signature: Vec<Vec<[f32; 2]>>,
    pub sign_pad: Option<Vec<Vec<[f32; 2]>>>,
    pub redactions: Vec<(usize, [f32; 4])>,
    pub measure: Option<(usize, [f32; 2], [f32; 2])>,
    pub left_tab: usize,
    pub panel: panels::Panel,
    pub st: panels::State,
    pub help: studio_ui::help::Help,
    palette: studio_ui::palette::Palette,
    inbox: studio_ui::handoff::Receiver,
    icon: TextureHandle,
    pub recent: Vec<PathBuf>,
    ask_pw: Option<(PathBuf, Vec<u8>, String)>,
    pub jobs: Option<mpsc::Receiver<Job>>,
    pub progress: Option<(usize, usize)>,
    title: String,
}

fn recent_file() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("EthoneStudio").join("folio-recent.json"))
}

pub fn toast(ctx: &egui::Context, s: impl Into<String>, ok: bool) {
    studio_ui::handoff::toast(ctx, s.into(), ok);
}

impl Folio {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        let recent = recent_file().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str::<Vec<PathBuf>>(&s).ok()).unwrap_or_default();
        let mut me = Self {
            od: None,
            tex: HashMap::new(),
            thumbs: HashMap::new(),
            zoom: 1.0,
            fit: Some(1),
            page: 0,
            goto: None,
            sel: Vec::new(),
            tool: Tool::Hand,
            color: [255, 214, 10],
            width: 2.0,
            text_size: 12.0,
            stamp: 0,
            drag: None,
            ink: Vec::new(),
            typing: None,
            signature: Vec::new(),
            sign_pad: None,
            redactions: Vec::new(),
            measure: None,
            left_tab: 0,
            panel: panels::Panel::All,
            st: panels::State::default(),
            help: studio_ui::help::Help::new("Folio", tutos::FOLIO),
            palette: studio_ui::palette::Palette::new(&cc.egui_ctx),
            inbox: studio_ui::handoff::Receiver::new("folio"),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "folio"),
            recent,
            ask_pw: None,
            jobs: None,
            progress: None,
            title: String::new(),
        };
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut i = 0;
        let (mut ocr, mut tool) = (false, None);
        let mut file = None;
        while i < args.len() {
            match args[i].as_str() {
                "--ocr" => ocr = true,
                "--outil" => {
                    tool = args.get(i + 1).cloned();
                    i += 1;
                }
                a => file = Some(PathBuf::from(a)),
            }
            i += 1;
        }
        if let Some(p) = file {
            me.open_path(&p, &cc.egui_ctx);
        }
        if let Some(t) = tool {
            me.panel = panels::by_name(&t).unwrap_or(panels::Panel::All);
        }
        if ocr && me.od.is_some() {
            let all = me.scope(1);
            panels::start_ocr(&mut me, all, true);
            me.panel = panels::Panel::Ocr;
        }
        me
    }

    pub fn open_dialog(&mut self, ctx: &egui::Context) {
        if let Some(p) = rfd::FileDialog::new().add_filter("PDF, Illustrator et images", &["pdf", "ai", "png", "jpg", "jpeg"]).pick_file() {
            self.open_path(&p, ctx);
        }
    }

    pub fn open_path(&mut self, p: &Path, ctx: &egui::Context) {
        let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "webp") {
            match image::open(p).map_err(|e| e.to_string()).and_then(|i| engine::images_to_pdf(&[i.to_rgba8()], None)).and_then(|mut d| engine::save(&mut d)) {
                Ok(b) => {
                    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or("Scan".into());
                    self.set_doc(Opened::new(b, None, format!("{stem}.pdf"), ""), ctx);
                    toast(ctx, "Image convertie en PDF : lance « Numériser et OCR » pour en lire le texte", true);
                }
                Err(e) => toast(ctx, format!("Ouverture impossible : {e}"), false),
            }
            return;
        }
        let bytes = match std::fs::read(p) {
            Ok(b) => b,
            Err(e) => return toast(ctx, format!("Lecture impossible : {e}"), false),
        };
        let illustrator = ext == "ai";
        if illustrator && !bytes.starts_with(b"%PDF") {
            return toast(ctx, "Ce fichier Illustrator n'a pas de copie PDF : dans Illustrator, enregistre-le avec « Créer un fichier compatible PDF »", false);
        }
        let (src, name) = if illustrator { (None, format!("{}.pdf", p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default())) } else { (Some(p.to_path_buf()), name) };
        match Opened::new(bytes.clone(), src, name, "") {
            Err(e) if e.contains("mot de passe") || e.contains("assword") || e.contains("ecrypt") => self.ask_pw = Some((p.to_path_buf(), bytes, String::new())),
            r => {
                self.set_doc(r, ctx);
                self.remember(p);
            }
        }
    }

    fn remember(&mut self, p: &Path) {
        self.recent.retain(|r| r != p);
        self.recent.insert(0, p.to_path_buf());
        self.recent.truncate(10);
        if let Some(f) = recent_file() {
            let _ = std::fs::create_dir_all(f.parent().unwrap_or(Path::new(".")));
            let _ = std::fs::write(f, serde_json::to_string(&self.recent).unwrap_or_default());
        }
    }

    pub fn set_doc(&mut self, r: engine::R<Opened>, ctx: &egui::Context) {
        match r {
            Ok(o) => {
                self.tex.clear();
                self.thumbs.clear();
                self.page = 0;
                self.sel.clear();
                self.redactions.clear();
                self.measure = None;
                self.fit = Some(self.open_fit().into());
                self.st.loaded(&o);
                self.od = Some(o);
                self.goto = Some(0);
            }
            Err(e) => toast(ctx, format!("Ouverture impossible : {e}"), false),
        }
    }

    pub fn edit(&mut self, ctx: &egui::Context, label: &str, f: impl FnOnce(&mut lopdf::Document) -> engine::R<()>) -> bool {
        let Some(od) = &mut self.od else { return false };
        let r = f(&mut od.doc).and_then(|_| engine::save(&mut od.doc));
        match r.and_then(|b| {
            let old = od.bytes.clone();
            od.set_bytes(Arc::new(b))?;
            Ok(old)
        }) {
            Ok(old) => {
                od.undo.push(old);
                if od.undo.len() > studio_ui::palette::undo_levels(40) {
                    od.undo.remove(0);
                }
                od.redo.clear();
                od.dirty = true;
                self.page = self.page.min(od.n().saturating_sub(1));
                self.sel.retain(|s| *s < od.n());
                if !label.is_empty() {
                    toast(ctx, label, true);
                }
                true
            }
            Err(e) => {
                if let Ok(d) = engine::load(&od.bytes, "") {
                    od.doc = d;
                }
                toast(ctx, e, false);
                false
            }
        }
    }

    pub fn undo(&mut self, redo: bool) {
        let Some(od) = &mut self.od else { return };
        let Some(b) = (if redo { od.redo.pop() } else { od.undo.pop() }) else { return };
        let cur = od.bytes.clone();
        if od.set_bytes(b).is_ok() {
            if redo { od.undo.push(cur) } else { od.redo.push(cur) }
            od.dirty = true;
            self.page = self.page.min(od.n().saturating_sub(1));
        }
    }

    pub fn save(&mut self, ctx: &egui::Context, as_new: bool) {
        let Some(od) = &mut self.od else { return };
        let path = match (&od.path, as_new) {
            (Some(p), false) => p.clone(),
            _ => match rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name(&od.name).save_file() {
                Some(p) => p,
                None => return,
            },
        };
        match std::fs::write(&path, od.bytes.as_slice()) {
            Ok(()) => {
                od.dirty = false;
                od.name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                od.path = Some(path.clone());
                toast(ctx, format!("Enregistré : {}", path.display()), true);
                self.remember(&path);
            }
            Err(e) => toast(ctx, format!("Enregistrement impossible : {e}"), false),
        }
    }

    pub fn scope(&self, k: usize) -> Vec<usize> {
        let n = self.od.as_ref().map_or(0, |o| o.n());
        scope_indices(n, self.page, &self.sel, k)
    }

    pub fn render_page(&self, i: usize, scale: f32) -> Option<image::RgbaImage> {
        render::page(&self.od.as_ref()?.pdf, i, scale)
    }

    fn texture(&mut self, ctx: &egui::Context, i: usize, scale: f32, budget: &mut u32) -> Option<TextureHandle> {
        let rev = self.od.as_ref()?.rev;
        let ok = self.tex.get(&i).is_some_and(|(r, s, _)| *r == rev && (*s / scale - 1.0).abs() < 0.2);
        if !ok && *budget > 0 {
            *budget -= 1;
            if let Some(img) = self.render_page(i, scale) {
                let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
                let t = ctx.load_texture(format!("folio-p{i}"), ci, TextureOptions::LINEAR);
                self.tex.insert(i, (rev, scale, t));
            }
        } else if !ok {
            ctx.request_repaint();
        }
        self.tex.get(&i).map(|t| t.2.clone())
    }

    pub fn thumb(&mut self, ctx: &egui::Context, i: usize, budget: &mut u32) -> Option<TextureHandle> {
        let rev = self.od.as_ref()?.rev;
        if !self.thumbs.get(&i).is_some_and(|t| t.0 == rev) {
            if *budget == 0 {
                ctx.request_repaint();
                return self.thumbs.get(&i).map(|t| t.1.clone());
            }
            *budget -= 1;
            let (w, _) = self.od.as_ref()?.sizes.get(i).copied()?;
            let img = self.render_page(i, 220.0 / w.max(1.0))?;
            let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
            self.thumbs.insert(i, (rev, ctx.load_texture(format!("folio-t{i}"), ci, TextureOptions::LINEAR)));
        }
        self.thumbs.get(&i).map(|t| t.1.clone())
    }

    fn mark(&self, kind: MarkKind, pts: Vec<[f32; 2]>, text: String) -> Mark {
        let color = match kind {
            MarkKind::Stamp => STAMPS[self.stamp % STAMPS.len()].1,
            MarkKind::Note if self.color == [255, 255, 255] => [255, 210, 30],
            _ => self.color,
        };
        Mark { kind, pts, strokes: Vec::new(), color, width: self.width, text, size: self.text_size }
    }

    fn finish(&mut self, ctx: &egui::Context, page: usize, a: [f32; 2], b: [f32; 2]) {
        let tiny = (a[0] - b[0]).abs() < 3.0 && (a[1] - b[1]).abs() < 3.0;
        let r = [a[0].min(b[0]), a[1].min(b[1]), (a[0] - b[0]).abs(), (a[1] - b[1]).abs()];
        let kind = match self.tool {
            Tool::Highlight => MarkKind::Highlight,
            Tool::Underline => MarkKind::Underline,
            Tool::Strike => MarkKind::Strike,
            Tool::Rect => MarkKind::Rect,
            Tool::Ellipse => MarkKind::Ellipse,
            Tool::Line => MarkKind::Line,
            Tool::Arrow => MarkKind::Arrow,
            Tool::Stamp => {
                let label = STAMPS[self.stamp % STAMPS.len()].0.to_string();
                let (w, h) = if tiny { (engine::text_em(&label.to_uppercase()) * 18.0 + 24.0, 34.0) } else { (r[2], r[3]) };
                let size = (h * 0.5).min(w / (engine::text_em(&label.to_uppercase()) + 0.8)).max(6.0);
                let m = Mark { size, ..self.mark(MarkKind::Stamp, vec![[r[0], r[1]], [r[0] + w, r[1] + h]], label) };
                self.edit(ctx, "Tampon posé", |d| engine::add_mark(d, page, &m));
                return;
            }
            Tool::TextBox => {
                let w = if tiny { 200.0 } else { r[2] };
                let h = if tiny { self.text_size * 3.0 } else { r[3] };
                self.typing = Some((page, MarkKind::TextBox, vec![[r[0], r[1]], [r[0] + w, r[1] + h]], String::new()));
                return;
            }
            Tool::Redact => {
                if !tiny {
                    self.redactions.push((page, r));
                }
                return;
            }
            Tool::Measure => {
                self.measure = Some((page, a, b));
                return;
            }
            _ => return,
        };
        if tiny && !matches!(kind, MarkKind::Line | MarkKind::Arrow) {
            return;
        }
        let pts = if matches!(kind, MarkKind::Line | MarkKind::Arrow) { vec![a, b] } else { vec![[r[0], r[1]], [r[0] + r[2], r[1] + r[3]]] };
        let m = self.mark(kind, pts, String::new());
        self.edit(ctx, "", |d| engine::add_mark(d, page, &m));
    }

    fn click(&mut self, ctx: &egui::Context, page: usize, p: [f32; 2]) {
        match self.tool {
            Tool::Note => self.typing = Some((page, MarkKind::Note, vec![p], String::new())),
            Tool::Date => {
                let d = engine::expand("{date}", 0, 0, "");
                let m = self.mark(MarkKind::TextBox, vec![p, [p[0] + engine::text_em(&d) * self.text_size + 6.0, p[1] + self.text_size * 1.4]], d);
                self.edit(ctx, "Date ajoutée", |doc| engine::add_mark(doc, page, &m));
            }
            Tool::Sign => {
                if self.signature.is_empty() {
                    self.sign_pad = Some(Vec::new());
                    return;
                }
                let (w, h) = (160.0, 160.0 / 3.0);
                let strokes: Vec<Vec<[f32; 2]>> = self.signature.iter().map(|s| s.iter().map(|q| [p[0] + q[0] * w, p[1] + q[1] * h]).collect()).collect();
                let m = Mark { strokes, color: [20, 40, 120], width: 1.6, ..self.mark(MarkKind::Ink, Vec::new(), String::new()) };
                self.edit(ctx, "Signature posée", |doc| engine::add_mark(doc, page, &m));
            }
            _ => {}
        }
    }

    pub fn zoom_to(&mut self, z: f32) {
        self.zoom = z.clamp(0.1, 8.0);
        self.fit = None;
        self.goto = Some(self.page);
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        let (cmd, shift) = ctx.input(|i| (i.modifiers.command, i.modifiers.shift));
        let pressed = |k: Key| ctx.input(|i| i.key_pressed(k));
        if cmd && pressed(Key::O) {
            self.open_dialog(ctx);
        }
        if cmd && pressed(Key::S) {
            self.save(ctx, shift);
        }
        if cmd && pressed(Key::Z) {
            self.undo(shift);
        }
        if cmd && pressed(Key::Y) {
            self.undo(true);
        }
        if cmd && pressed(Key::F) {
            self.panel = panels::Panel::Search;
        }
        if cmd && pressed(Key::E) {
            self.panel = panels::Panel::Export;
        }
        if cmd && (pressed(Key::Plus) || pressed(Key::Equals)) {
            self.zoom_to(self.zoom * 1.25);
        }
        if cmd && pressed(Key::Minus) {
            self.zoom_to(self.zoom / 1.25);
        }
        if cmd && pressed(Key::Num0) {
            self.fit = Some(0);
        }
        if cmd && pressed(Key::Num1) {
            self.zoom_to(1.0);
        }
        if cmd && pressed(Key::Num2) {
            self.fit = Some(1);
        }
        let n = self.od.as_ref().map_or(0, |o| o.n());
        if pressed(Key::PageDown) && self.page + 1 < n {
            self.goto = Some(self.page + 1);
        }
        if pressed(Key::PageUp) && self.page > 0 {
            self.goto = Some(self.page - 1);
        }
        if pressed(Key::Home) && n > 0 {
            self.goto = Some(0);
        }
        if pressed(Key::End) && n > 0 {
            self.goto = Some(n - 1);
        }
        if !cmd && pressed(Key::H) {
            self.tool = Tool::Hand;
        }
        if pressed(Key::Escape) {
            self.tool = Tool::Hand;
            self.drag = None;
            self.measure = None;
        }
        if pressed(Key::F1) {
            self.help.toggle();
        }
    }

    fn top_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(34.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let full = studio_ui::chrome::bar(ui, 12.0);
            let right_w = 420.0;
            ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_max(full.min, pos2(full.right() - right_w, full.bottom()))), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(22.0))));
                    ui.add_space(4.0);
                    egui::MenuBar::new().ui(ui, |ui| self.menus(ui));
                });
            });
            ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(full.right() - right_w, full.top()), full.max)), |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.od.is_some() {
                        if pro::primary(ui, "Enregistrer").on_hover_text("Ctrl+S").clicked() {
                            let ctx = ui.ctx().clone();
                            self.save(&ctx, false);
                        }
                        if pro::secondary(ui, "Tous les outils").clicked() {
                            self.panel = panels::Panel::All;
                        }
                    }
                    if let Some((a, b)) = self.progress {
                        ui.add(egui::Spinner::new());
                        ui.label(RichText::new(format!("{a} / {b}")).size(12.0).color(MUTED));
                    }
                });
            });
        });
    }

    fn menus(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        ui.menu_button("Fichier", |ui| {
            if ui.button("Ouvrir…                         Ctrl+O").clicked() {
                self.open_dialog(&ctx);
            }
            if ui.button("Créer un PDF depuis des images…").clicked() {
                self.panel = panels::Panel::Create;
                panels::create_from_images(self, &ctx, Some((595.0, 842.0)));
            }
            if ui.button("Combiner des fichiers…").clicked() {
                self.panel = panels::Panel::Combine;
            }
            if ui.button("Nouveau PDF vierge (A4)").clicked() {
                panels::blank(self, &ctx);
            }
            if !self.recent.is_empty() {
                ui.menu_button("Récents", |ui| {
                    for p in self.recent.clone() {
                        if ui.button(p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).on_hover_text(p.display().to_string()).clicked() {
                            self.open_path(&p, &ctx);
                        }
                    }
                });
            }
            ui.separator();
            let has = self.od.is_some();
            if ui.add_enabled(has, egui::Button::new("Enregistrer                   Ctrl+S")).clicked() {
                self.save(&ctx, false);
            }
            if ui.add_enabled(has, egui::Button::new("Enregistrer sous…       Ctrl+Maj+S")).clicked() {
                self.save(&ctx, true);
            }
            if ui.add_enabled(has, egui::Button::new("Exporter…                       Ctrl+E")).clicked() {
                self.panel = panels::Panel::Export;
            }
            if has && let Some(t) = studio_ui::handoff::menu(ui, &[("pixel", "retoucher la page"), ("lumen", "développer la page")]) {
                panels::send_page(self, &ctx, t);
            }
            ui.separator();
            if ui.add_enabled(has, egui::Button::new("Fermer le document")).clicked() {
                self.od = None;
            }
        });
        ui.menu_button("Édition", |ui| {
            let (u, r) = self.od.as_ref().map_or((false, false), |o| (!o.undo.is_empty(), !o.redo.is_empty()));
            if ui.add_enabled(u, egui::Button::new("Annuler                 Ctrl+Z")).clicked() {
                self.undo(false);
            }
            if ui.add_enabled(r, egui::Button::new("Rétablir                Ctrl+Y")).clicked() {
                self.undo(true);
            }
            ui.separator();
            if ui.button("Rechercher               Ctrl+F").clicked() {
                self.panel = panels::Panel::Search;
            }
            if ui.button("Supprimer tous les commentaires").clicked() {
                let all = self.scope(1);
                self.edit(&ctx, "Commentaires supprimés", |d| engine::remove_comments(d, &all).map(|_| ()));
            }
        });
        ui.menu_button("Affichage", |ui| {
            if ui.button("Page entière             Ctrl+0").clicked() {
                self.fit = Some(0);
            }
            if ui.button("Largeur de la page     Ctrl+2").clicked() {
                self.fit = Some(1);
            }
            if ui.button("Taille réelle              Ctrl+1").clicked() {
                self.zoom_to(1.0);
            }
            ui.separator();
            for (k, n) in ["Vignettes", "Signets", "Commentaires"].iter().enumerate() {
                if ui.radio(self.left_tab == k, *n).clicked() {
                    self.left_tab = k;
                }
            }
        });
        ui.menu_button("Outils", |ui| {
            for (p, name) in panels::LIST {
                if ui.button(name).clicked() {
                    self.panel = p;
                }
            }
        });
        self.help.menu(ui);
    }

    fn status(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(22.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let Some(n) = self.od.as_ref().map(|o| o.n()) else {
                    ui.label(RichText::new("Glisse un PDF ou une image dans la fenêtre").size(12.0).color(FAINT));
                    return;
                };
                if ui.small_button("◀").on_hover_text("Page précédente").clicked() && self.page > 0 {
                    self.goto = Some(self.page - 1);
                }
                let mut p = self.page as u32 + 1;
                if ui.add(egui::DragValue::new(&mut p).range(1..=n as u32)).changed() {
                    self.goto = Some(p as usize - 1);
                }
                ui.label(RichText::new(format!("/ {n}")).size(12.0).color(MUTED));
                if ui.small_button("▶").on_hover_text("Page suivante").clicked() && self.page + 1 < n {
                    self.goto = Some(self.page + 1);
                }
                pro::toolbar_sep(ui);
                if ui.small_button("−").clicked() {
                    self.zoom_to(self.zoom / 1.25);
                }
                ui.label(RichText::new(format!("{:.0} %", self.zoom * 100.0)).size(12.0).color(INK));
                if ui.small_button("+").clicked() {
                    self.zoom_to(self.zoom * 1.25);
                }
                if ui.small_button("Page").on_hover_text("Ctrl+0").clicked() {
                    self.fit = Some(0);
                }
                if ui.small_button("Largeur").on_hover_text("Ctrl+2").clicked() {
                    self.fit = Some(1);
                }
                if let Some((w, h)) = self.od.as_ref().and_then(|o| o.sizes.get(self.page).copied()) {
                    pro::toolbar_sep(ui);
                    ui.label(RichText::new(format!("{:.0} × {:.0} mm", w * 25.4 / 72.0, h * 25.4 / 72.0)).size(12.0).color(MUTED));
                }
                if let Some((_, a, b)) = self.measure {
                    pro::toolbar_sep(ui);
                    let d = ((a[0] - b[0]).hypot(a[1] - b[1])) * 25.4 / 72.0;
                    ui.label(RichText::new(format!("Mesure : {d:.1} mm ({:.2} cm)", d / 10.0)).size(12.0).color(BLUE));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let tip = TOOLS.iter().find(|t| t.0 == self.tool).map_or("", |t| t.1);
                    ui.label(RichText::new(tip).size(12.0).color(MUTED));
                });
            });
        });
    }

    fn tool_rail(&mut self, root: &mut Ui) {
        egui::Panel::left("rail").resizable(false).exact_size(48.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(6, 10)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (k, (t, name)) in TOOLS.iter().enumerate() {
                    if matches!(k, 1 | 5 | 11 | 14) {
                        ui.add_space(4.0);
                        let (r, _) = ui.allocate_exact_size(vec2(34.0, 1.0), Sense::hover());
                        ui.painter().rect_filled(r, CornerRadius::ZERO, LINE);
                        ui.add_space(4.0);
                    }
                    let on = self.tool == *t;
                    if pro::icon_button(ui, 34.0, on, name, |p, r, c| glyph(p, r, *t, c)).clicked() {
                        self.tool = *t;
                        if *t == Tool::Sign && self.signature.is_empty() {
                            self.sign_pad = Some(Vec::new());
                        }
                        if matches!(t, Tool::Highlight | Tool::Underline | Tool::Strike) && self.color == [20, 40, 120] {
                            self.color = [255, 214, 10];
                        }
                        if matches!(t, Tool::Stamp | Tool::Redact) {
                            self.panel = if *t == Tool::Stamp { panels::Panel::Comment } else { panels::Panel::Redact };
                        }
                    }
                }
            });
        });
    }

    fn left_panel(&mut self, root: &mut Ui) {
        let ctx = root.ctx().clone();
        egui::Panel::left("pages").resizable(false).exact_size(190.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 10))).show(root, |ui| {
            pro::segmented(ui, &mut self.left_tab, &["Pages", "Signets", "Notes"]);
            ui.add_space(8.0);
            match self.left_tab {
                0 => {
                    let n = self.od.as_ref().map_or(0, |o| o.n());
                    let mut budget = 3;
                    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                        for i in 0..n {
                            let (w, h) = self.od.as_ref().map_or((1.0, 1.0), |o| o.sizes[i]);
                            let tw = 120.0;
                            let th = tw * h / w.max(1.0);
                            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), th + 26.0), Sense::click());
                            let pr = Rect::from_center_size(pos2(r.center().x, r.top() + 4.0 + th / 2.0), vec2(tw, th));
                            let visible = ui.is_rect_visible(r);
                            let selected = self.sel.contains(&i);
                            if selected || i == self.page {
                                ui.painter().rect_stroke(pr.expand(3.0), CornerRadius::same(4), Stroke::new(2.0, if selected { BLUE } else { ACCENT }), StrokeKind::Outside);
                            }
                            ui.painter().rect_filled(pr, CornerRadius::ZERO, Color32::WHITE);
                            if visible && let Some(t) = self.thumb(&ctx, i, &mut budget) {
                                ui.painter().image(t.id(), pr, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                            }
                            ui.painter().text(pos2(r.center().x, r.bottom() - 10.0), Align2::CENTER_CENTER, (i + 1).to_string(), FontId::proportional(11.5), MUTED);
                            if resp.clicked() {
                                let (c, s) = ui.input(|x| (x.modifiers.command, x.modifiers.shift));
                                if c {
                                    if let Some(k) = self.sel.iter().position(|x| *x == i) {
                                        self.sel.remove(k);
                                    } else {
                                        self.sel.push(i);
                                    }
                                } else if s {
                                    let (a, b) = (self.page.min(i), self.page.max(i));
                                    self.sel = (a..=b).collect();
                                } else {
                                    self.sel.clear();
                                    self.goto = Some(i);
                                }
                            }
                            resp.on_hover_text("Clic : aller à la page · Ctrl+clic / Maj+clic : sélectionner");
                        }
                    });
                }
                1 => panels::bookmarks_list(self, ui),
                _ => {
                    let Some(od) = &self.od else { return };
                    let list = engine::comments(&od.doc, self.page);
                    ui.label(RichText::new(format!("Page {} · {} commentaire(s)", self.page + 1, list.len())).size(12.0).color(MUTED));
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (kind, text) in list {
                            ui.label(RichText::new(kind).font(semibold(12.0)).color(INK));
                            if !text.is_empty() {
                                ui.label(RichText::new(text).size(12.0).color(MUTED));
                            }
                            ui.add_space(4.0);
                        }
                    });
                }
            }
        });
    }

    fn home(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(30.0);
            ui.horizontal(|ui| {
                ui.add_space(40.0);
                ui.add(egui::Image::new((self.icon.id(), Vec2::splat(40.0))));
                ui.vertical(|ui| {
                    ui.label(RichText::new("Bienvenue dans Folio").font(semibold(28.0)).color(INK_STRONG));
                    ui.label(RichText::new("Lis, annote, signe, organise et numérise tes PDF").size(12.5).color(MUTED));
                });
            });
            ui.add_space(26.0);
            let cards: [(&str, &str, u8); 6] = [
                ("Ouvrir un PDF", "Ou une image, glissée ici", 0),
                ("Créer depuis des images", "Photos et scans en un seul PDF A4", 1),
                ("Combiner des fichiers", "Plusieurs PDF et images en un seul", 2),
                ("Numériser et OCR", "Le texte des scans et photos", 3),
                ("Remplir et signer", "Formulaire, signature manuscrite, date", 4),
                ("PDF vierge", "Une page A4 à remplir", 5),
            ];
            Frame::new().inner_margin(Margin { left: 40, right: 20, top: 0, bottom: 0 }).show(ui, |ui| ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                for (title, sub, k) in cards {
                    let (r, resp) = ui.allocate_exact_size(vec2(290.0, 92.0), Sense::click());
                    let hover = resp.hovered();
                    ui.painter().rect(r, CornerRadius::same(6), if hover { Color32::from_rgb(0x2c, 0x2c, 0x2c) } else { Color32::from_rgb(0x24, 0x24, 0x24) }, Stroke::new(1.0, LINE), StrokeKind::Inside);
                    let ic = Rect::from_min_size(r.min + vec2(16.0, 24.0), Vec2::splat(42.0));
                    ui.painter().rect_filled(ic, CornerRadius::same(5), ACCENT.gamma_multiply(0.22));
                    home_glyph(ui.painter(), ic.shrink(10.0), k, ACCENT);
                    ui.painter().text(r.min + vec2(72.0, 30.0), Align2::LEFT_CENTER, title, semibold(13.0), INK_STRONG);
                    ui.painter().text(r.min + vec2(72.0, 54.0), Align2::LEFT_CENTER, sub, FontId::proportional(11.5), MUTED);
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        match k {
                            0 => self.open_dialog(&ctx),
                            1 => panels::create_from_images(self, &ctx, Some((595.0, 842.0))),
                            2 => self.panel = panels::Panel::Combine,
                            3 => {
                                if let Some(p) = rfd::FileDialog::new().add_filter("Image ou PDF", &["png", "jpg", "jpeg", "pdf"]).pick_file() {
                                    self.open_path(&p, &ctx);
                                    self.panel = panels::Panel::Ocr;
                                }
                            }
                            4 => {
                                self.open_dialog(&ctx);
                                self.panel = panels::Panel::FillSign;
                            }
                            _ => panels::blank(self, &ctx),
                        }
                    }
                }
            }));
            if !self.recent.is_empty() {
                ui.add_space(26.0);
                ui.horizontal(|ui| {
                    ui.add_space(40.0);
                    ui.label(RichText::new("Récents").font(semibold(16.0)).color(INK_STRONG));
                });
                ui.add_space(6.0);
                for p in self.recent.clone() {
                    ui.horizontal(|ui| {
                        ui.add_space(52.0);
                        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        let resp = ui.add(egui::Label::new(RichText::new(&name).font(semibold(12.5)).color(INK)).sense(Sense::click()));
                        ui.label(RichText::new(p.parent().map(|d| d.display().to_string()).unwrap_or_default()).size(11.0).color(FAINT));
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            self.open_path(&p, &ctx);
                        }
                    });
                }
            }
        });
    }

    fn viewer(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let Some(sizes) = self.od.as_ref().map(|o| o.sizes.clone()) else { return };
        let avail = ui.available_size();
        if let Some(f) = self.fit.take() {
            let (w, h) = sizes.get(self.page).copied().unwrap_or((595.0, 842.0));
            self.zoom = if f == 0 { ((avail.y - 2.0 * GAP) / (h * PX)).min((avail.x - 60.0) / (w * PX)) } else { ((avail.x - 60.0) / (w * PX)).min(4.0) }.clamp(0.1, 8.0);
            self.goto = Some(self.page);
        }
        let hovered = ui.rect_contains_pointer(ui.max_rect());
        let zd = ui.input(|i| i.zoom_delta());
        if hovered && (zd - 1.0).abs() > 1e-3 {
            self.zoom = (self.zoom * zd).clamp(0.1, 8.0);
        }
        let z = self.zoom * PX;
        let ppp = ctx.pixels_per_point();
        let maxw = sizes.iter().map(|s| s.0).fold(0.0, f32::max) * z;
        let total = sizes.iter().map(|s| s.1 * z + GAP).sum::<f32>() + GAP;
        let mut budget = 2u32;
        let goto = self.goto.take();
        let tool = self.tool;
        egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
            let (rect, resp) = ui.allocate_exact_size(vec2((maxw + 80.0).max(avail.x), total.max(avail.y)), Sense::click_and_drag());
            let clip = ui.clip_rect();
            let mut y = rect.top() + GAP;
            let mut rects = Vec::with_capacity(sizes.len());
            for (w, h) in &sizes {
                let r = Rect::from_min_size(pos2(rect.center().x - w * z / 2.0, y), vec2(w * z, h * z));
                rects.push(r);
                y += h * z + GAP;
            }
            if let Some(g) = goto.filter(|g| *g < rects.len()) {
                ui.scroll_to_rect(rects[g].translate(vec2(0.0, -GAP)), Some(Align::TOP));
                self.page = g;
            } else if let Some(k) = rects.iter().position(|r| r.contains(pos2(r.center().x, clip.center().y)) || r.top() > clip.center().y) {
                self.page = k;
            }
            for (i, r) in rects.iter().enumerate() {
                if !r.intersects(clip) {
                    continue;
                }
                ui.painter().rect_filled(r.translate(vec2(0.0, 3.0)).expand(1.0), CornerRadius::same(2), Color32::from_black_alpha(90));
                ui.painter().rect_filled(*r, CornerRadius::ZERO, Color32::WHITE);
                if let Some(t) = self.texture(&ctx, i, z * ppp, &mut budget) {
                    ui.painter().image(t.id(), *r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
                if let Some(t) = panels::diff_texture(self, &ctx, i) {
                    ui.painter().image(t.id(), *r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
                for (_, q) in self.redactions.iter().filter(|x| x.0 == i) {
                    let rr = Rect::from_min_size(r.min + vec2(q[0], q[1]) * z, vec2(q[2], q[3]) * z);
                    ui.painter().rect(rr, CornerRadius::ZERO, Color32::from_black_alpha(120), Stroke::new(1.5, Color32::from_rgb(230, 40, 40)), StrokeKind::Middle);
                }
                if let Some((p, a, b)) = self.measure.filter(|m| m.0 == i) {
                    let _ = p;
                    let (pa, pb) = (r.min + vec2(a[0], a[1]) * z, r.min + vec2(b[0], b[1]) * z);
                    ui.painter().line_segment([pa, pb], Stroke::new(2.0, BLUE));
                    let d = (a[0] - b[0]).hypot(a[1] - b[1]) * 25.4 / 72.0;
                    let mid = pa + (pb - pa) / 2.0;
                    let g = ui.painter().layout_no_wrap(format!("{d:.1} mm"), FontId::proportional(12.0), Color32::WHITE);
                    let br = Rect::from_center_size(mid - vec2(0.0, 14.0), g.size() + vec2(10.0, 4.0));
                    ui.painter().rect_filled(br, CornerRadius::same(4), BLUE);
                    ui.painter().galley(br.min + vec2(5.0, 2.0), g, Color32::WHITE);
                }
            }
            let to_pt = |i: usize, pos: egui::Pos2| -> [f32; 2] {
                let r = rects[i];
                [((pos.x - r.min.x) / z).clamp(0.0, r.width() / z), ((pos.y - r.min.y) / z).clamp(0.0, r.height() / z)]
            };
            let ptr = resp.interact_pointer_pos().or_else(|| resp.hover_pos());
            let under = ptr.and_then(|p| rects.iter().position(|r| r.contains(p)));
            if tool != Tool::Hand && resp.hovered() && under.is_some() {
                ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            }
            if tool == Tool::Hand {
                if resp.dragged() {
                    ui.scroll_with_delta(resp.drag_delta());
                    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                } else if resp.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::Grab);
                }
            } else if let Some(p) = ptr {
                if resp.drag_started()
                    && let Some(i) = under
                {
                    let q = to_pt(i, p);
                    self.drag = Some((i, q, q));
                    self.ink = vec![q];
                }
                if resp.dragged()
                    && let Some((i, _, cur)) = &mut self.drag
                {
                    let q = to_pt(*i, p);
                    *cur = q;
                    if tool == Tool::Ink && self.ink.last().is_none_or(|l| (l[0] - q[0]).hypot(l[1] - q[1]) > 0.8) {
                        self.ink.push(q);
                    }
                }
                if let Some((i, a, b)) = self.drag {
                    let r = rects[i];
                    let (pa, pb) = (r.min + vec2(a[0], a[1]) * z, r.min + vec2(b[0], b[1]) * z);
                    let c = Color32::from_rgb(self.color[0], self.color[1], self.color[2]);
                    let st = Stroke::new(1.5, if tool == Tool::Redact { Color32::from_rgb(230, 40, 40) } else { BLUE });
                    match tool {
                        Tool::Ink => {
                            let pts: Vec<egui::Pos2> = self.ink.iter().map(|q| r.min + vec2(q[0], q[1]) * z).collect();
                            ui.painter().add(Shape::line(pts, Stroke::new(self.width * z, c)));
                        }
                        Tool::Line | Tool::Arrow | Tool::Measure => {
                            ui.painter().line_segment([pa, pb], Stroke::new((self.width * z).max(1.5), if tool == Tool::Measure { BLUE } else { c }));
                        }
                        Tool::Highlight => {
                            ui.painter().rect_filled(Rect::from_two_pos(pa, pb), CornerRadius::ZERO, c.gamma_multiply(0.35));
                        }
                        Tool::Ellipse => {
                            let rr = Rect::from_two_pos(pa, pb);
                            let pts: Vec<egui::Pos2> = (0..48).map(|k| {
                                let t = k as f32 / 48.0 * std::f32::consts::TAU;
                                rr.center() + vec2(t.cos() * rr.width() / 2.0, t.sin() * rr.height() / 2.0)
                            }).collect();
                            ui.painter().add(Shape::closed_line(pts, st));
                        }
                        _ => {
                            ui.painter().rect_stroke(Rect::from_two_pos(pa, pb), CornerRadius::ZERO, st, StrokeKind::Middle);
                        }
                    }
                }
                if resp.drag_stopped()
                    && let Some((i, a, b)) = self.drag.take()
                {
                    if tool == Tool::Ink {
                        let m = Mark { strokes: vec![std::mem::take(&mut self.ink)], ..self.mark(MarkKind::Ink, Vec::new(), String::new()) };
                        self.edit(&ctx, "", |d| engine::add_mark(d, i, &m));
                    } else {
                        self.finish(&ctx, i, a, b);
                    }
                }
                if resp.clicked()
                    && let Some(i) = under
                {
                    let q = to_pt(i, p);
                    if matches!(tool, Tool::TextBox | Tool::Stamp) {
                        self.finish(&ctx, i, q, q);
                    } else {
                        self.click(&ctx, i, q);
                    }
                }
            }
            if let Some((i, kind, pts, mut text)) = self.typing.take() {
                let r = rects[i];
                let a = r.min + vec2(pts[0][0], pts[0][1]) * z;
                let mut keep = true;
                let mut commit = false;
                egui::Area::new(egui::Id::NULL.with("folio-typing")).order(egui::Order::Foreground).fixed_pos(a).show(&ctx, |ui| {
                    Frame::new().fill(CHROME).stroke(Stroke::new(1.0, BLUE)).corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(8)).show(ui, |ui| {
                        ui.label(RichText::new(if kind == MarkKind::Note { "Note" } else { "Texte" }).size(12.0).color(MUTED));
                        let te = ui.add(egui::TextEdit::multiline(&mut text).desired_width(240.0).desired_rows(3).hint_text("Écris ici…"));
                        te.request_focus();
                        ui.horizontal(|ui| {
                            if pro::primary(ui, "OK").clicked() {
                                commit = true;
                            }
                            if ui.button("Annuler").clicked() {
                                keep = false;
                            }
                        });
                    });
                });
                if commit && !text.trim().is_empty() {
                    let m = self.mark(kind, pts, text.trim().to_string());
                    let m = if kind == MarkKind::TextBox { Mark { color: if self.color == [255, 214, 10] { [20, 20, 20] } else { self.color }, ..m } } else { m };
                    self.edit(&ctx, "", |d| engine::add_mark(d, i, &m));
                } else if keep && !commit {
                    self.typing = Some((i, kind, pts, text));
                }
            }
        });
    }

    fn sheets(&mut self, ctx: &egui::Context) {
        if let Some((path, bytes, mut pw)) = self.ask_pw.take() {
            let mut go = false;
            let mut cancel = false;
            pro::sheet(ctx, "pw", 380.0, |ui| {
                ui.label(RichText::new("Document protégé").font(semibold(17.0)).color(INK_STRONG));
                ui.label(RichText::new(path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).size(12.0).color(MUTED));
                ui.add_space(10.0);
                let r = ui.add(egui::TextEdit::singleline(&mut pw).password(true).hint_text("Mot de passe").desired_width(f32::INFINITY));
                r.request_focus();
                if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    go = true;
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if pro::primary(ui, "Ouvrir").clicked() {
                        go = true;
                    }
                    if ui.button("Annuler").clicked() {
                        cancel = true;
                    }
                });
            });
            if go {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                match Opened::new(bytes.clone(), Some(path.clone()), name, &pw) {
                    Ok(o) => {
                        self.set_doc(Ok(o), ctx);
                        self.remember(&path);
                        toast(ctx, "Document déverrouillé : Enregistrer garde une copie sans mot de passe", true);
                    }
                    Err(_) => {
                        toast(ctx, "Mot de passe incorrect", false);
                        self.ask_pw = Some((path, bytes, String::new()));
                    }
                }
            } else if !cancel {
                self.ask_pw = Some((path, bytes, pw));
            }
        }
        if let Some(mut pad) = self.sign_pad.take() {
            let mut keep = true;
            pro::sheet(ctx, "sign", 460.0, |ui| {
                ui.label(RichText::new("Créer ma signature").font(semibold(17.0)).color(INK_STRONG));
                ui.label(RichText::new("Dessine à la souris dans le cadre").size(12.0).color(MUTED));
                ui.add_space(8.0);
                let (r, resp) = ui.allocate_exact_size(vec2(420.0, 140.0), Sense::drag());
                ui.painter().rect(r, CornerRadius::same(4), Color32::WHITE, Stroke::new(1.0, LINE), StrokeKind::Inside);
                ui.painter().line_segment([pos2(r.left() + 20.0, r.bottom() - 30.0), pos2(r.right() - 20.0, r.bottom() - 30.0)], Stroke::new(1.0, Color32::from_gray(200)));
                if let Some(p) = resp.interact_pointer_pos() {
                    let q = [((p.x - r.left()) / r.width()).clamp(0.0, 1.0), ((p.y - r.top()) / r.height()).clamp(0.0, 1.0)];
                    if resp.drag_started() {
                        pad.push(vec![q]);
                    } else if resp.dragged()
                        && let Some(s) = pad.last_mut()
                    {
                        s.push(q);
                    }
                }
                for s in &pad {
                    let pts: Vec<egui::Pos2> = s.iter().map(|q| pos2(r.left() + q[0] * r.width(), r.top() + q[1] * r.height())).collect();
                    ui.painter().add(Shape::line(pts, Stroke::new(2.4, Color32::from_rgb(20, 40, 120))));
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if pro::primary(ui, "Utiliser").clicked() && !pad.is_empty() {
                        self.signature = pad.clone();
                        self.tool = Tool::Sign;
                        keep = false;
                        toast(ui.ctx(), "Clique sur la page pour poser ta signature", true);
                    }
                    if ui.button("Effacer").clicked() {
                        pad.clear();
                    }
                    if ui.button("Annuler").clicked() {
                        keep = false;
                    }
                });
            });
            if keep {
                self.sign_pad = Some(pad);
            }
        }
    }

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.jobs else { return };
        ctx.request_repaint_after(std::time::Duration::from_millis(150));
        let msgs: Vec<Job> = rx.try_iter().collect();
        for m in msgs {
            match m {
                Job::Progress(a, b) => self.progress = Some((a, b)),
                Job::Ocr(pages, layer) => {
                    self.progress = None;
                    self.jobs = None;
                    panels::ocr_done(self, ctx, pages, layer);
                }
                Job::Compare(v) => {
                    self.progress = None;
                    self.jobs = None;
                    let diff = v.iter().filter(|d| **d > 0.0005).count();
                    toast(ctx, format!("{diff} page(s) différente(s) sur {}", v.len()), true);
                    if let Some(c) = &mut self.st.cmp {
                        c.diffs = v;
                    }
                }
                Job::Err(e) => {
                    self.progress = None;
                    self.jobs = None;
                    toast(ctx, e, false);
                }
            }
        }
    }

    fn commands(&self) -> Vec<studio_ui::palette::Cmd> {
        use studio_ui::palette::Cmd;
        let mut v = vec![Cmd::new("Ouvrir…", "Ctrl+O", "open"), Cmd::new("Enregistrer", "Ctrl+S", "save"), Cmd::new("Enregistrer sous…", "Ctrl+Maj+S", "saveas"), Cmd::new("Exporter la page courante en PNG…", "Page · rendu 200 ppp", "export-page"), Cmd::new("Pivoter les pages à gauche", "Page courante ou sélection", "rotate-left"), Cmd::new("Pivoter les pages à droite", "Page courante ou sélection", "rotate-right")];
        let has_doc = self.od.is_some();
        v = v
            .into_iter()
            .map(|command| match command.id.as_str() {
                "open" => command,
                _ => command.enabled_if(has_doc, "Ouvre d'abord un PDF"),
            })
            .collect();
        for (p, name) in panels::LIST {
            let standalone = matches!(p, panels::Panel::Combine | panels::Panel::Create);
            v.push(Cmd::new(format!("Outil : {name}"), "Outils", format!("panel:{}", p as u8)).enabled_if(standalone || has_doc, "Ouvre d'abord un PDF"));
        }
        for (id, name, _) in outils::TOOLS {
            v.push(Cmd::new(format!("Outil PDF : {name}"), "Folio · nouvel outil · document ou page courante", format!("oc:{id}")).enabled_if(has_doc, "Ouvre d'abord un PDF"));
        }
        for (t, name) in TOOLS {
            v.push(Cmd::new(format!("Annoter : {name}"), "Barre d'outils", format!("tool:{}", t as u8)).enabled_if(has_doc, "Ouvre d'abord un PDF"));
        }
        v.extend(studio_ui::palette::send_cmds(&[("pixel", "retoucher la page"), ("lumen", "développer la page")]).into_iter().map(|command| command.enabled_if(has_doc, "Ouvre d'abord un PDF")));
        v
    }

    fn run_cmd(&mut self, id: &str, ctx: &egui::Context) {
        if self.run_lib(id, ctx) {
            return;
        }
        if let Some(t) = id.strip_prefix("send:") {
            let t: &'static str = studio_core::handoff::APPS.iter().find(|a| a.0 == t).map_or("pixel", |a| a.0);
            panels::send_page(self, ctx, t);
            return;
        }
        if let Some(k) = id.strip_prefix("panel:").and_then(|k| k.parse::<u8>().ok()) {
            if let Some((p, _)) = panels::LIST.iter().find(|p| p.0 as u8 == k) {
                self.panel = *p;
            }
            return;
        }
        if let Some(k) = id.strip_prefix("tool:").and_then(|k| k.parse::<u8>().ok()) {
            if let Some((t, _)) = TOOLS.iter().find(|t| t.0 as u8 == k) {
                self.tool = *t;
            }
            return;
        }
        match id {
            "open" => self.open_dialog(ctx),
            "save" => self.save(ctx, false),
            "saveas" => self.save(ctx, true),
            "rotate-left" | "rotate-right" => {
                let pages = self.scope(2);
                let deg = if id == "rotate-left" { -90 } else { 90 };
                self.edit(ctx, "Pages pivotées", |doc| engine::rotate(doc, &pages, deg));
            }
            "export-page" => {
                let name = self.od.as_ref().map(|opened| format!("{} - page {}.png", opened.name, self.page + 1)).unwrap_or("page.png".into());
                if let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name(name).save_file()
                    && let Some(image) = self.render_page(self.page, 200.0 / 72.0)
                {
                    match image.save(&path) {
                        Ok(()) => toast(ctx, format!("Page exportée : {}", path.display()), true),
                        Err(error) => toast(ctx, error.to_string(), false),
                    }
                }
            }
            _ => {}
        }
    }
}

impl eframe::App for Folio {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        studio_ui::chrome::resize(ctx);
        for p in self.inbox.poll(ctx) {
            self.open_path(&p, ctx);
        }
        self.palette.set_library(optlib::lib);
        self.palette.keys(ctx);
        if !self.palette.prefs_declared() {
            self.palette.declare_prefs("folio", reglages::PREFS);
        }
        if self.palette.take_prefs_changed() {
            self.apply_prefs();
        }
        if self.palette.open {
            let cmds = self.commands();
            if let Some(id) = self.palette.show(ctx, &mut self.help, cmds) {
                self.run_cmd(&id, ctx);
            }
        }
        self.keys(ctx);
        self.poll_jobs(ctx);
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        if self.panel == panels::Panel::Combine && !dropped.is_empty() {
            self.st.combine.extend(dropped);
        } else if let Some(p) = dropped.first() {
            self.open_path(p, ctx);
        }
        let title = match &self.od {
            Some(o) => format!("{}{} — Folio", o.name, if o.dirty { " •" } else { "" }),
            None => "Folio".into(),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.top_bar(root);
        self.status(root);
        if self.od.is_some() || self.panel == panels::Panel::Combine {
            egui::Panel::right("tools").resizable(false).exact_size(320.0).frame(Frame::new().fill(PANEL).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
                panels::show(self, ui);
            });
        }
        if self.od.is_some() {
            self.tool_rail(root);
            self.left_panel(root);
        }
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            if self.od.is_some() { self.viewer(ui) } else { self.home(ui) }
        });
        self.sheets(ctx);
        self.help.show(ctx);
    }
}

/// Lucide icon names, so this app's icons match the rest of the suite.
fn glyph_lucide(t: Tool) -> &'static str {
    use Tool::*;
    match t {
        Hand => "hand",
        Highlight => "highlighter",
        Underline => "underline",
        Strike => "strikethrough",
        Note => "sticky-note",
        TextBox => "type",
        Rect => "square",
        Ellipse => "circle",
        Line => "slash",
        Arrow => "move-up-right",
        Ink => "pen-line",
        Stamp => "stamp",
        Sign => "signature",
        Date => "calendar",
        Redact => "eye-off",
        Measure => "ruler",
    }
}

pub fn glyph(p: &Painter, r: Rect, t: Tool, c: Color32) {
    studio_ui::pro::lucide(p, r.expand(r.width() * 0.12), glyph_lucide(t), c);
}

fn home_glyph(p: &Painter, r: Rect, k: u8, c: Color32) {
    let s = Stroke::new(2.0, c);
    let at = |x: f32, y: f32| pos2(r.left() + x * r.width(), r.top() + y * r.height());
    let doc = |p: &Painter| p.add(Shape::closed_line(vec![at(0.15, 0.0), at(0.65, 0.0), at(0.9, 0.25), at(0.9, 1.0), at(0.15, 1.0)], s));
    match k {
        0 => {
            doc(p);
            p.line_segment([at(0.3, 0.5), at(0.75, 0.5)], s);
            p.line_segment([at(0.3, 0.72), at(0.65, 0.72)], s);
        }
        1 => {
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.15), at(1.0, 0.85)), CornerRadius::same(3), s, StrokeKind::Middle);
            p.circle_stroke(at(0.3, 0.4), r.width() * 0.08, s);
            p.add(Shape::line(vec![at(0.05, 0.8), at(0.4, 0.5), at(0.6, 0.65), at(0.95, 0.35)], s));
        }
        2 => {
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.0), at(0.55, 0.7)), CornerRadius::same(2), s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(at(0.45, 0.3), at(1.0, 1.0)), CornerRadius::same(2), s, StrokeKind::Middle);
        }
        3 => {
            for (x, y) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                let (dx, dy) = (if x > 0.5 { -0.3 } else { 0.3 }, if y > 0.5 { -0.3 } else { 0.3 });
                p.line_segment([at(x, y), at(x + dx, y)], s);
                p.line_segment([at(x, y), at(x, y + dy)], s);
            }
            p.text(at(0.5, 0.5), Align2::CENTER_CENTER, "Aa", FontId::proportional(r.height() * 0.45), c);
        }
        4 => glyph(p, r, Tool::Sign, c),
        _ => {
            doc(p);
            p.line_segment([at(0.52, 0.4), at(0.52, 0.8)], s);
            p.line_segment([at(0.32, 0.6), at(0.72, 0.6)], s);
        }
    }
}

fn scope_indices(n: usize, page: usize, selected: &[usize], kind: usize) -> Vec<usize> {
    match kind {
        0 => (n > 0).then(|| page.min(n - 1)).into_iter().collect(),
        2 if !selected.is_empty() => {
            let mut pages: Vec<usize> = selected.iter().copied().filter(|index| *index < n).collect();
            pages.sort_unstable();
            pages.dedup();
            pages
        }
        2 => (n > 0).then(|| page.min(n - 1)).into_iter().collect(),
        _ => (0..n).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::scope_indices;

    #[test]
    fn selected_scope_defaults_to_current_page() {
        assert_eq!(scope_indices(5, 3, &[], 2), vec![3]);
        assert_eq!(scope_indices(5, 3, &[1, 4], 2), vec![1, 4]);
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_decorations(false)
            .with_title("Folio")
            .with_icon(Arc::new(studio_ui::app_window_icon("folio")))
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1000.0, 620.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Folio", options, Box::new(|cc| Ok(Box::new(Folio::new(cc)))))
}
