#![windows_subsystem = "windows"]

mod develop;
mod tutos;

use develop::Settings;
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, Rect, RichText,
    Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2, pos2, vec2,
};
use image::RgbImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};

const PREVIEW_MAX: u32 = 1800;
const THUMB_MAX: u32 = 320;
const STAR: Color32 = Color32::from_rgb(255, 196, 0);

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Entry {
    settings: Settings,
    rating: u8,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Catalog {
    folder: Option<PathBuf>,
    recent_folders: Vec<PathBuf>,
    photos: HashMap<PathBuf, Entry>,
}

impl Catalog {
    fn path() -> PathBuf {
        studio_core::data_dir().join("lumen").join("catalog.json")
    }
    fn load() -> Self {
        std::fs::read(Self::path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
    fn save(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, serde_json::to_vec_pretty(self).unwrap_or_default());
    }
}

enum Msg {
    Thumb(PathBuf, RgbImage),
    Preview(PathBuf, Result<RgbImage, String>),
}

#[derive(PartialEq, Clone, Copy)]
enum View {
    Library,
    Develop,
}

#[derive(PartialEq, Clone, Copy)]
enum Collection {
    All,
    Rated,
    Edited,
}

struct Export {
    done: Arc<AtomicUsize>,
    total: usize,
    errors: Arc<Mutex<Vec<String>>>,
}

struct Lumen {
    catalog: Catalog,
    files: Vec<PathBuf>,
    thumbs: HashMap<PathBuf, TextureHandle>,
    current: Option<usize>,
    source: Option<(PathBuf, RgbImage)>,
    preview: Option<TextureHandle>,
    shown: Option<(PathBuf, Settings, bool)>,
    hist: Option<[[u32; 256]; 4]>,
    loading: bool,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    view: View,
    collection: Collection,
    min_rating: u8,
    thumb_size: f32,
    before: bool,
    clipboard: Option<Settings>,
    export: Option<Export>,
    error: Option<String>,
    dirty_catalog: bool,
    icon: TextureHandle,
    help: studio_ui::help::Help,
    hsl_tab: usize,
    export_quality: u8,
    export_edge: u32,
}

impl Lumen {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        let (tx, rx) = channel();
        let mut me = Self {
            catalog: Catalog::load(),
            files: Vec::new(),
            thumbs: HashMap::new(),
            current: None,
            source: None,
            preview: None,
            shown: None,
            hist: None,
            loading: false,
            tx,
            rx,
            view: View::Library,
            collection: Collection::All,
            min_rating: 0,
            thumb_size: 200.0,
            before: false,
            clipboard: None,
            export: None,
            error: None,
            dirty_catalog: false,
            icon: studio_ui::icon_texture(&cc.egui_ctx, "lumen"),
            help: studio_ui::help::Help::new("Lumen", tutos::LUMEN),
            hsl_tab: 0,
            export_quality: 92,
            export_edge: 0,
        };
        let arg = std::env::args().nth(1).map(PathBuf::from);
        if let Some(file) = arg.filter(|p| p.is_file() && develop::is_supported(p))
            && let Some(dir) = file.parent().map(|d| d.to_path_buf())
        {
            me.open_folder(dir, &cc.egui_ctx);
            if let Some(i) = me.files.iter().position(|f| *f == file) {
                me.select(i, &cc.egui_ctx);
                me.view = View::Develop;
            }
        } else if let Some(f) = me.catalog.folder.clone()
            && f.is_dir()
        {
            me.open_folder(f, &cc.egui_ctx);
        }
        me
    }

    fn open_folder(&mut self, folder: PathBuf, ctx: &egui::Context) {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&folder)
            .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| develop::is_supported(p)).collect())
            .unwrap_or_default();
        files.sort();
        self.catalog.recent_folders.retain(|f| *f != folder);
        self.catalog.recent_folders.insert(0, folder.clone());
        self.catalog.recent_folders.truncate(6);
        self.catalog.folder = Some(folder);
        self.dirty_catalog = true;
        self.thumbs.clear();
        self.current = None;
        self.source = None;
        self.preview = None;
        self.shown = None;
        self.hist = None;
        self.view = View::Library;
        for f in &files {
            let tx = self.tx.clone();
            let f = f.clone();
            let ctx = ctx.clone();
            rayon::spawn(move || {
                if let Ok(img) = develop::load(&f, THUMB_MAX) {
                    let _ = tx.send(Msg::Thumb(f, img));
                    ctx.request_repaint();
                }
            });
        }
        self.files = files;
    }

    fn pick_folder(&mut self, ctx: &egui::Context) {
        if let Some(f) = rfd::FileDialog::new().set_title("Choisir un dossier de photos").pick_folder() {
            self.open_folder(f, ctx);
        }
    }

    fn select(&mut self, i: usize, ctx: &egui::Context) {
        self.current = Some(i);
        let path = self.files[i].clone();
        if self.source.as_ref().is_some_and(|(p, _)| *p == path) {
            return;
        }
        self.loading = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        rayon::spawn(move || {
            let r = develop::load(&path, PREVIEW_MAX);
            let _ = tx.send(Msg::Preview(path, r));
            ctx.request_repaint();
        });
    }

    fn entry(&mut self, i: usize) -> &mut Entry {
        self.catalog.photos.entry(self.files[i].clone()).or_default()
    }

    fn get(&self, i: usize) -> Entry {
        self.catalog.photos.get(&self.files[i]).cloned().unwrap_or_default()
    }

    fn pump(&mut self, ctx: &egui::Context) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Thumb(p, img) => {
                    let s = self.catalog.photos.get(&p).map(|e| e.settings).unwrap_or_default();
                    let img = develop::process(&img, &s);
                    self.thumbs.insert(p.clone(), ctx.load_texture(p.to_string_lossy(), color_image(&img), TextureOptions::LINEAR));
                }
                Msg::Preview(p, r) => {
                    if self.current.is_some_and(|i| self.files.get(i) == Some(&p)) {
                        self.loading = false;
                        match r {
                            Ok(img) => {
                                self.source = Some((p, img));
                                self.shown = None;
                            }
                            Err(e) => self.error = Some(format!("{} : {e}", p.display())),
                        }
                    }
                }
            }
        }
    }

    fn refresh_preview(&mut self, ctx: &egui::Context) {
        let Some((path, src)) = &self.source else { return };
        let s = self.catalog.photos.get(path).map(|e| e.settings).unwrap_or_default();
        let key = (path.clone(), s, self.before);
        if self.shown.as_ref() == Some(&key) {
            return;
        }
        let used = if self.before { Settings::default() } else { s };
        let img = develop::process(src, &used);
        self.hist = Some(develop::histogram(&img));
        let ci = color_image(&img);
        match &mut self.preview {
            Some(t) => t.set(ci, TextureOptions::LINEAR),
            None => self.preview = Some(ctx.load_texture("preview", ci, TextureOptions::LINEAR)),
        }
        if !self.before
            && let Some(thumb) = self.thumbs.get_mut(path)
        {
            let small = image::imageops::thumbnail(&img, THUMB_MAX, THUMB_MAX);
            thumb.set(color_image(&small), TextureOptions::LINEAR);
        }
        self.shown = Some(key);
    }

    fn start_export(&mut self, only_current: bool) {
        let ids: Vec<usize> = if only_current { self.current.into_iter().collect() } else { self.visible() };
        let list: Vec<(PathBuf, Settings)> = ids.iter().map(|&i| (self.files[i].clone(), self.get(i).settings)).collect();
        if list.is_empty() {
            return;
        }
        let Some(dir) = rfd::FileDialog::new().set_title("Dossier d'export").pick_folder() else { return };
        let (quality, edge) = (self.export_quality, self.export_edge);
        let done = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(Mutex::new(Vec::new()));
        self.export = Some(Export { done: done.clone(), total: list.len(), errors: errors.clone() });
        std::thread::spawn(move || {
            use rayon::prelude::*;
            list.par_iter().for_each(|(p, s)| {
                let r = develop::load(p, 0).and_then(|img| {
                    let mut out = develop::process(&img, s);
                    if edge > 0 && out.width().max(out.height()) > edge {
                        let r = edge as f32 / out.width().max(out.height()) as f32;
                        out = image::imageops::resize(&out, (out.width() as f32 * r).round() as u32, (out.height() as f32 * r).round() as u32, image::imageops::FilterType::Lanczos3);
                    }
                    let name = p.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                    let file = std::fs::File::create(dir.join(format!("{name}.jpg"))).map_err(|e| e.to_string())?;
                    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), quality);
                    enc.encode_image(&out).map_err(|e| e.to_string())
                });
                if let Err(e) = r
                    && let Ok(mut list) = errors.lock()
                {
                    list.push(format!("{} : {e}", p.display()));
                }
                done.fetch_add(1, Ordering::Relaxed);
            });
        });
    }

    fn visible(&self) -> Vec<usize> {
        (0..self.files.len())
            .filter(|&i| {
                let e = self.get(i);
                e.rating >= self.min_rating
                    && match self.collection {
                        Collection::All => true,
                        Collection::Rated => e.rating >= 3,
                        Collection::Edited => !e.settings.is_default(),
                    }
            })
            .collect()
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (rating, next, prev, copy, paste, d, g, back, auto) = ctx.input_mut(|i| {
            use egui::{KeyboardShortcut as K, Modifiers as M};
            let copy = i.consume_shortcut(&K::new(M::COMMAND, Key::C));
            let paste = i.consume_shortcut(&K::new(M::COMMAND, Key::V));
            let auto = i.consume_shortcut(&K::new(M::COMMAND, Key::U));
            let mut r = None;
            for (n, k) in [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5].into_iter().enumerate() {
                if i.key_pressed(k) && !i.modifiers.command {
                    r = Some(n as u8);
                }
            }
            (r, i.key_pressed(Key::ArrowRight), i.key_pressed(Key::ArrowLeft), copy, paste, i.key_pressed(Key::D) || i.key_pressed(Key::Enter), i.key_pressed(Key::G) || i.key_pressed(Key::Escape), i.key_down(Key::Backslash), auto)
        });
        self.before = back;
        let vis = self.visible();
        let pos = self.current.and_then(|c| vis.iter().position(|&v| v == c));
        if next || prev {
            let n = match pos {
                Some(p) if next => (p + 1).min(vis.len().saturating_sub(1)),
                Some(p) => p.saturating_sub(1),
                None => 0,
            };
            if let Some(&i) = vis.get(n) {
                self.select(i, ctx);
            }
        }
        if let Some(i) = self.current {
            if let Some(r) = rating {
                self.entry(i).rating = r;
                self.dirty_catalog = true;
            }
            if copy {
                self.clipboard = Some(self.get(i).settings);
            }
            if paste && let Some(s) = self.clipboard {
                self.entry(i).settings = s;
                self.dirty_catalog = true;
            }
            if auto && let Some((_, src)) = &self.source {
                let s = develop::auto(src);
                self.entry(i).settings = s;
                self.dirty_catalog = true;
            }
            if d {
                self.view = View::Develop;
            }
        }
        if g {
            self.view = View::Library;
        }
    }
}

impl eframe::App for Lumen {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        self.pump(ctx);
        self.keys(ctx);
        self.help.show(ctx);
        if self.current.is_none() && self.view == View::Develop {
            self.view = View::Library;
        }

        self.menu_bar(root, ctx);
        self.status_bar(root);
        if self.files.is_empty() {
            self.welcome(root, ctx);
        } else {
            match self.view {
                View::Library => self.library(root, ctx),
                View::Develop => self.develop(root, ctx),
            }
        }
        if self.dirty_catalog && ctx.input(|i| !i.pointer.any_down()) {
            self.catalog.save();
            self.dirty_catalog = false;
        }
    }
}

impl Lumen {
    fn menu_bar(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::Panel::top("menu").exact_size(44.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add(egui::Image::new((self.icon.id(), Vec2::splat(22.0))));
                ui.add_space(4.0);
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Ouvrir un dossier…").clicked() {
                            self.pick_folder(ctx);
                        }
                        let recents = self.catalog.recent_folders.clone();
                        ui.add_enabled_ui(!recents.is_empty(), |ui| {
                            ui.menu_button("Dossiers récents", |ui| {
                                for f in recents {
                                    if ui.button(f.display().to_string()).clicked() {
                                        self.open_folder(f, ctx);
                                    }
                                }
                            });
                        });
                        ui.separator();
                        if ui.add_enabled(self.current.is_some(), egui::Button::new("Exporter la photo…")).clicked() {
                            self.start_export(true);
                        }
                        if ui.add_enabled(!self.files.is_empty(), egui::Button::new("Exporter les photos affichées…")).clicked() {
                            self.start_export(false);
                        }
                        ui.menu_button("Options d'export", |ui| {
                            ui.label(RichText::new("Qualité JPEG").size(12.5).color(MUTED));
                            ui.add(egui::Slider::new(&mut self.export_quality, 40..=100));
                            ui.label(RichText::new("Grand côté").size(12.5).color(MUTED));
                            for (label, edge) in [("Taille d'origine", 0), ("4096 px", 4096), ("2048 px (réseaux)", 2048), ("1080 px (Instagram)", 1080)] {
                                ui.radio_value(&mut self.export_edge, edge, label);
                            }
                        });
                    });
                    ui.add_enabled_ui(self.current.is_some(), |ui| {
                        ui.menu_button("Photo", |ui| {
                            let i = self.current.unwrap_or(0);
                            if ui.button("Réglage automatique      Ctrl+U").clicked()
                                && let Some((_, src)) = &self.source
                            {
                                let s = develop::auto(src);
                                self.entry(i).settings = s;
                                self.dirty_catalog = true;
                            }
                            if ui.button("Copier les réglages      Ctrl+C").clicked() {
                                self.clipboard = Some(self.get(i).settings);
                            }
                            if ui.add_enabled(self.clipboard.is_some(), egui::Button::new("Coller les réglages       Ctrl+V")).clicked()
                                && let Some(s) = self.clipboard
                            {
                                self.entry(i).settings = s;
                                self.dirty_catalog = true;
                            }
                            if ui.button("Réinitialiser").clicked() {
                                self.entry(i).settings = Settings::default();
                                self.dirty_catalog = true;
                            }
                            if ui.add_enabled(self.clipboard.is_some(), egui::Button::new("Coller sur les photos affichées")).clicked()
                                && let Some(c) = self.clipboard
                            {
                                for v in self.visible() {
                                    self.entry(v).settings = c;
                                }
                                self.dirty_catalog = true;
                            }
                            ui.separator();
                            let mut cur = self.get(i).settings;
                            let before = cur;
                            ui.checkbox(&mut cur.mono, "Noir et blanc");
                            ui.checkbox(&mut cur.flip, "Miroir horizontal");
                            if ui.button("Rotation 90° horaire").clicked() {
                                cur.rotate = (cur.rotate + 1) % 4;
                            }
                            if ui.button("Rotation 90° antihoraire").clicked() {
                                cur.rotate = (cur.rotate + 3) % 4;
                            }
                            if cur != before {
                                self.entry(i).settings = cur;
                                self.dirty_catalog = true;
                            }
                            ui.separator();
                            for r in 0..=5u8 {
                                let label = if r == 0 { "Sans note                       0".to_string() } else { format!("{}{}   {r}", "★".repeat(r as usize), " ".repeat(5 - r as usize)) };
                                if ui.button(label).clicked() {
                                    self.entry(i).rating = r;
                                    self.dirty_catalog = true;
                                }
                            }
                        });
                    });
                    self.help.menu(ui);
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !self.files.is_empty() {
                        let lib = self.view == View::Library;
                        segmented(ui, &mut self.view, self.current.is_some(), lib);
                    }
                });
            });
        });
    }

    fn status_bar(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                match &self.catalog.folder {
                    Some(f) => ui.label(small(format!("{} · {} photo(s)", f.display(), self.files.len()))),
                    None => ui.label(small("Aucun dossier ouvert".into())),
                };
                ui.label(small("← → naviguer · 0-5 noter · D développer · G bibliothèque · \\ avant/après".into()));
                if let Some(e) = &self.export {
                    let d = e.done.load(Ordering::Relaxed);
                    let errs = e.errors.lock().map(|l| l.len()).unwrap_or(0);
                    if d < e.total {
                        ui.add(egui::ProgressBar::new(d as f32 / e.total as f32).desired_width(160.0).text(format!("Export {d}/{}", e.total)));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
                    } else if errs > 0 {
                        ui.label(RichText::new(format!("Export terminé, {errs} erreur(s)")).size(12.0).color(RED));
                    } else {
                        ui.label(RichText::new(format!("{} photo(s) exportée(s)", e.total)).size(12.0).color(GREEN));
                    }
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

    fn welcome(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let area = ui.max_rect();
            let card = Rect::from_center_size(area.center(), vec2(520.0, 360.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(card), |ui| {
                Frame::new().fill(CHROME).corner_radius(CornerRadius::same(18)).stroke(Stroke::new(1.0, LINE)).inner_margin(Margin::same(32)).show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(egui::Image::new((self.icon.id(), Vec2::splat(64.0))));
                        ui.add_space(14.0);
                        ui.label(RichText::new("Bienvenue dans Lumen").font(semibold(24.0)).color(INK_STRONG));
                        ui.add_space(4.0);
                        ui.label(RichText::new("Ouvre un dossier de photos : JPEG, PNG, TIFF et RAW (CR2, NEF, ARW, DNG, RAF…)").color(MUTED));
                        ui.add_space(18.0);
                        if pro::primary(ui, "Ouvrir un dossier").clicked() {
                            self.pick_folder(ctx);
                        }
                        let recents: Vec<PathBuf> = self.catalog.recent_folders.iter().filter(|f| f.is_dir()).cloned().collect();
                        if !recents.is_empty() {
                            ui.add_space(16.0);
                            ui.label(RichText::new("Dossiers récents").font(semibold(13.0)).color(INK));
                            for f in recents.into_iter().take(3) {
                                if ui.link(RichText::new(f.display().to_string()).color(BLUE)).clicked() {
                                    self.open_folder(f, ctx);
                                }
                            }
                        }
                    });
                });
            });
        });
    }

    fn library_nav(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::Panel::left("lib-nav").resizable(false).exact_size(232.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 16))).show(root, |ui| {
            heading(ui, "CATALOGUE");
            let all = self.files.len();
            let rated = (0..all).filter(|&i| self.get(i).rating >= 3).count();
            let edited = (0..all).filter(|&i| !self.get(i).settings.is_default()).count();
            for (c, label, n) in [(Collection::All, "Toutes les photos", all), (Collection::Rated, "Notées 3★ et plus", rated), (Collection::Edited, "Modifiées", edited)] {
                if nav_row(ui, label, n, self.collection == c) {
                    self.collection = c;
                }
            }
            ui.add_space(16.0);
            heading(ui, "DOSSIERS");
            let current = self.catalog.folder.clone();
            for f in self.catalog.recent_folders.clone() {
                let name = f.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| f.display().to_string());
                if nav_row(ui, &name, usize::MAX, current.as_ref() == Some(&f)) && current.as_ref() != Some(&f) {
                    self.open_folder(f, ctx);
                }
            }
            ui.add_space(8.0);
            if pro::secondary(ui, "Ouvrir un dossier…").clicked() {
                self.pick_folder(ctx);
            }
        });
    }

    fn library(&mut self, root: &mut Ui, ctx: &egui::Context) {
        self.library_nav(root, ctx);
        egui::Panel::top("lib-bar").exact_size(46.0).frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin::symmetric(18, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(RichText::new("Filtre").size(12.5).color(MUTED));
                for r in 0..=5u8 {
                    let label = if r == 0 { "Toutes".to_string() } else { format!("{r}★+") };
                    let on = self.min_rating == r;
                    let (rect, resp) = ui.allocate_exact_size(vec2(if r == 0 { 58.0 } else { 40.0 }, 26.0), Sense::click());
                    ui.painter().rect_filled(rect, CornerRadius::same(13), if on { BLUE } else if resp.hovered() { FIELD } else { Color32::TRANSPARENT });
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(12.5), if on { Color32::WHITE } else { INK });
                    if resp.clicked() {
                        self.min_rating = r;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Exporter…").clicked() {
                        self.start_export(false);
                    }
                    ui.add_space(12.0);
                    ui.spacing_mut().slider_width = 110.0;
                    ui.add(egui::Slider::new(&mut self.thumb_size, 120.0..=340.0).show_value(false));
                    ui.label(RichText::new("Taille").size(12.5).color(MUTED));
                });
            });
        });
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin { left: 18, right: 18, top: 4, bottom: 8 })).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let vis = self.visible();
                if vis.is_empty() {
                    ui.add_space(60.0);
                    ui.vertical_centered(|ui| ui.label(RichText::new("Aucune photo ne correspond à ce filtre").color(MUTED)));
                    return;
                }
                let cell = self.thumb_size;
                let gap = 12.0;
                let cols = (((ui.available_width() + gap) / (cell + gap)).floor() as usize).max(1);
                for row in vis.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for &i in row {
                            match self.tile(ui, i, cell) {
                                Some(true) => {
                                    self.select(i, ctx);
                                    self.view = View::Develop;
                                }
                                Some(false) => self.select(i, ctx),
                                None => {}
                            }
                        }
                    });
                    ui.add_space(gap);
                }
            });
        });
    }

    fn tile(&mut self, ui: &mut Ui, i: usize, size: f32) -> Option<bool> {
        let (rect, resp) = ui.allocate_exact_size(vec2(size, size + 30.0), Sense::click());
        let on = self.current == Some(i);
        let p = ui.painter();
        p.rect(rect, CornerRadius::same(12), if on || resp.hovered() { Color32::from_rgb(0x2c, 0x2c, 0x32) } else { Color32::from_rgb(0x22, 0x22, 0x26) }, Stroke::new(if on { 2.0 } else { 1.0 }, if on { BLUE } else { LINE }), StrokeKind::Inside);
        let img_area = Rect::from_min_size(rect.min + vec2(8.0, 8.0), Vec2::splat(size - 16.0));
        let path = self.files[i].clone();
        match self.thumbs.get(&path) {
            Some(t) => {
                let [tw, th] = t.size();
                let s = (img_area.width() / tw as f32).min(img_area.height() / th as f32);
                let fit = Rect::from_center_size(img_area.center(), vec2(tw as f32, th as f32) * s);
                p.image(t.id(), fit, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            None => {
                p.rect_filled(img_area.shrink(10.0), CornerRadius::same(8), Color32::from_white_alpha(6));
            }
        }
        let e = self.get(i);
        let y = rect.bottom() - 17.0;
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        p.text(pos2(rect.left() + 12.0, y), Align2::LEFT_CENTER, short(&name, (size / 9.0) as usize), FontId::proportional(12.0), MUTED);
        let stars: String = (0..5).map(|k| if k < e.rating { '★' } else { '☆' }).collect();
        p.text(pos2(rect.right() - 12.0, y), Align2::RIGHT_CENTER, stars, FontId::proportional(12.0), if e.rating > 0 { STAR } else { FAINT });
        if !e.settings.is_default() {
            let badge = Rect::from_min_size(img_area.min + vec2(6.0, 6.0), vec2(22.0, 22.0));
            p.circle_filled(badge.center(), 10.0, Color32::from_black_alpha(150));
            p.text(badge.center(), Align2::CENTER_CENTER, "✎", FontId::proportional(12.0), Color32::WHITE);
        }
        if resp.double_clicked() {
            Some(true)
        } else if resp.clicked() {
            Some(false)
        } else {
            None
        }
    }

    fn develop(&mut self, root: &mut Ui, ctx: &egui::Context) {
        let Some(i) = self.current else { return };
        egui::Panel::bottom("strip").exact_size(112.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 10))).show(root, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for v in self.visible() {
                        let (r, resp) = ui.allocate_exact_size(vec2(110.0, 90.0), Sense::click());
                        let p = ui.painter();
                        p.rect_filled(r, CornerRadius::same(8), Color32::from_rgb(0x22, 0x22, 0x26));
                        if let Some(t) = self.thumbs.get(&self.files[v]) {
                            let [tw, th] = t.size();
                            let s = ((r.width() - 8.0) / tw as f32).min((r.height() - 8.0) / th as f32);
                            p.image(t.id(), Rect::from_center_size(r.center(), vec2(tw as f32, th as f32) * s), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                        }
                        if v == i {
                            p.rect_stroke(r, CornerRadius::same(8), Stroke::new(2.0, BLUE), StrokeKind::Inside);
                        }
                        if resp.clicked() {
                            self.select(v, ctx);
                        }
                    }
                });
            });
        });

        egui::Panel::left("presets").resizable(false).exact_size(220.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 14))).show(root, |ui| {
            if pro::primary(ui, "Réglage automatique").clicked()
                && let Some((_, src)) = &self.source
            {
                let s = develop::auto(src);
                self.entry(i).settings = s;
                self.dirty_catalog = true;
            }
            ui.add_space(16.0);
            heading(ui, "PRÉRÉGLAGES");
            let current = self.get(i).settings;
            for (name, s) in develop::presets() {
                if nav_row(ui, name, usize::MAX, current == s) {
                    self.entry(i).settings = s;
                    self.dirty_catalog = true;
                }
            }
        });

        egui::Panel::right("dev").resizable(false).exact_size(304.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let hist = self.hist;
                pro::dock_panel(ui, "hist", "Histogramme", |ui| draw_histogram(ui, hist.as_ref()));
                let name = self.files[i].file_name().unwrap_or_default().to_string_lossy().into_owned();
                let dims = self.source.as_ref().map(|(_, s)| format!("{} × {}", s.width(), s.height())).unwrap_or_default();
                let mut rating = self.get(i).rating;
                let mut s = self.get(i).settings;
                let start = (s, rating);
                pro::dock_panel(ui, "info", "Photo", |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.label(RichText::new(&name).font(semibold(13.5)).color(INK));
                    ui.label(RichText::new(format!("Aperçu {dims}")).size(12.0).color(FAINT));
                    ui.horizontal(|ui| {
                        for r in 1..=5u8 {
                            let on = rating >= r;
                            if ui.add(egui::Label::new(RichText::new(if on { "★" } else { "☆" }).size(20.0).color(if on { STAR } else { FAINT })).sense(Sense::click())).clicked() {
                                rating = if rating == r { 0 } else { r };
                            }
                        }
                    });
                });
                pro::dock_panel(ui, "light", "Lumière", |ui| {
                    dev_slider(ui, "Exposition", &mut s.exposure, -4.0..=4.0, 0.01);
                    dev_slider(ui, "Contraste", &mut s.contrast, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Hautes lumières", &mut s.highlights, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Ombres", &mut s.shadows, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Blancs", &mut s.whites, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Noirs", &mut s.blacks, -100.0..=100.0, 1.0);
                });
                pro::dock_panel(ui, "color", "Couleur", |ui| {
                    dev_slider(ui, "Température", &mut s.temperature, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Teinte", &mut s.tint, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Vibrance", &mut s.vibrance, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Saturation", &mut s.saturation, -100.0..=100.0, 1.0);
                    ui.checkbox(&mut s.mono, "Noir et blanc");
                });
                pro::dock_panel(ui, "presence", "Présence", |ui| {
                    dev_slider(ui, "Texture", &mut s.texture, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Clarté", &mut s.clarity, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Correction du voile", &mut s.dehaze, -100.0..=100.0, 1.0);
                });
                pro::dock_panel(ui, "curve", "Courbe des tonalités", |ui| {
                    draw_curve(ui, &s);
                    for (k, label) in ["Ombres", "Tons foncés", "Tons clairs", "Hautes lumières"].into_iter().enumerate() {
                        dev_slider(ui, label, &mut s.curve[k], -100.0..=100.0, 1.0);
                    }
                });
                let tab = &mut self.hsl_tab;
                pro::dock_panel(ui, "hsl", "TSL / Couleur", |ui| {
                    ui.horizontal(|ui| {
                        for (k, label) in ["Teinte", "Saturation", "Luminance"].into_iter().enumerate() {
                            if ui.selectable_label(*tab == k, label).clicked() {
                                *tab = k;
                            }
                        }
                    });
                    ui.add_space(6.0);
                    for (b, (name, _, rgb)) in develop::BANDS.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 5.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                            ui.label(RichText::new(*name).size(12.0).color(MUTED));
                        });
                        dev_slider(ui, "", &mut s.hsl[b][*tab], -100.0..=100.0, 1.0);
                    }
                });
                pro::dock_panel(ui, "split", "Virage partiel", |ui| {
                    hue_slider(ui, "Teinte des ombres", &mut s.split_shadow_hue);
                    dev_slider(ui, "Saturation des ombres", &mut s.split_shadow_sat, 0.0..=100.0, 1.0);
                    hue_slider(ui, "Teinte des hautes lumières", &mut s.split_high_hue);
                    dev_slider(ui, "Saturation des hautes lumières", &mut s.split_high_sat, 0.0..=100.0, 1.0);
                    dev_slider(ui, "Équilibre", &mut s.split_balance, -100.0..=100.0, 1.0);
                });
                pro::dock_panel(ui, "detail", "Détail", |ui| {
                    dev_slider(ui, "Netteté", &mut s.sharpen, 0.0..=150.0, 1.0);
                    dev_slider(ui, "Rayon", &mut s.sharpen_radius, 0.0..=3.0, 0.1);
                    dev_slider(ui, "Réduction du bruit", &mut s.noise, 0.0..=100.0, 1.0);
                });
                pro::dock_panel(ui, "effects", "Effets", |ui| {
                    dev_slider(ui, "Vignettage", &mut s.vignette, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Point milieu", &mut s.vignette_mid, 0.0..=100.0, 1.0);
                    dev_slider(ui, "Grain", &mut s.grain, 0.0..=100.0, 1.0);
                    dev_slider(ui, "Taille du grain", &mut s.grain_size, 0.0..=100.0, 1.0);
                });
                pro::dock_panel(ui, "geometry", "Géométrie", |ui| {
                    dev_slider(ui, "Redresser", &mut s.straighten, -45.0..=45.0, 0.1);
                    for (k, label) in ["Recadrage gauche", "Recadrage haut", "Recadrage droit", "Recadrage bas"].into_iter().enumerate() {
                        let mut v = s.crop[k] * 100.0;
                        dev_slider(ui, label, &mut v, 0.0..=45.0, 0.5);
                        s.crop[k] = v / 100.0;
                    }
                    ui.checkbox(&mut s.flip, "Miroir horizontal");
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if pro::secondary(ui, "⟲ 90°").clicked() {
                            s.rotate = (s.rotate + 3) % 4;
                        }
                        if pro::secondary(ui, "⟳ 90°").clicked() {
                            s.rotate = (s.rotate + 1) % 4;
                        }
                    });
                });
                pro::dock_panel(ui, "actions", "Réglages", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if pro::secondary(ui, "?  Tutoriels").clicked() {
                            self.help.learn("Développement");
                        }
                        if pro::secondary(ui, "Copier").clicked() {
                            self.clipboard = Some(s);
                        }
                        if self.clipboard.is_some() && pro::secondary(ui, "Coller").clicked()
                            && let Some(c) = self.clipboard
                        {
                            s = c;
                        }
                        if pro::secondary(ui, "Réinitialiser").clicked() {
                            s = Settings::default();
                        }
                    });
                });
                if (s, rating) != start {
                    let e = self.entry(i);
                    e.settings = s;
                    e.rating = rating;
                    self.dirty_catalog = true;
                }
            });
        });

        self.refresh_preview(ctx);
        egui::CentralPanel::default().frame(Frame::new().fill(Color32::from_rgb(0x12, 0x12, 0x14))).show(root, |ui| {
            let full = ui.max_rect();
            let bar = Rect::from_min_size(full.min, vec2(full.width(), 40.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(14.0, 4.0))), |ui| {
                ui.horizontal_centered(|ui| {
                    let name = self.files[i].file_name().unwrap_or_default().to_string_lossy().into_owned();
                    ui.label(RichText::new(name).size(13.0).color(MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let held = ui.add(egui::Button::new(RichText::new("Avant / après").size(12.5)).corner_radius(CornerRadius::same(13)).sense(Sense::click_and_drag()));
                        if held.is_pointer_button_down_on() {
                            self.before = true;
                        }
                        held.on_hover_text("Maintenir pour voir l'original (ou la touche \\)");
                    });
                });
            });
            let area = Rect::from_min_max(pos2(full.left(), bar.bottom()), full.max).shrink(24.0);
            let p = ui.painter();
            if let Some(t) = &self.preview
                && self.source.as_ref().is_some_and(|(sp, _)| *sp == self.files[i])
            {
                let [w, h] = t.size();
                let s = (area.width() / w as f32).min(area.height() / h as f32);
                let r = Rect::from_center_size(area.center(), vec2(w as f32, h as f32) * s);
                p.add(egui::epaint::Shadow { offset: [0, 6], blur: 26, spread: 0, color: Color32::from_black_alpha(140) }.as_shape(r, CornerRadius::ZERO));
                p.image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                if self.before {
                    let tag = Rect::from_min_size(r.left_top() + vec2(12.0, 12.0), vec2(80.0, 26.0));
                    p.rect_filled(tag, CornerRadius::same(13), Color32::from_black_alpha(170));
                    p.text(tag.center(), Align2::CENTER_CENTER, "Original", FontId::proportional(12.5), Color32::WHITE);
                }
            } else if self.loading {
                p.text(area.center(), Align2::CENTER_CENTER, "Chargement…", FontId::proportional(15.0), MUTED);
            }
        });
    }
}

fn segmented(ui: &mut Ui, view: &mut View, can_develop: bool, _lib: bool) {
    let items = [(View::Develop, "Développement"), (View::Library, "Bibliothèque")];
    let (outer, _) = ui.allocate_exact_size(vec2(232.0, 30.0), Sense::hover());
    ui.painter().rect_filled(outer, CornerRadius::same(15), FIELD);
    for (k, (v, label)) in items.iter().rev().enumerate() {
        let r = Rect::from_min_size(outer.min + vec2(3.0 + k as f32 * 113.0, 3.0), vec2(113.0, 24.0));
        let enabled = *v == View::Library || can_develop;
        let resp = ui.interact(r, ui.make_persistent_id(("seg", k)), if enabled { Sense::click() } else { Sense::hover() });
        let on = *view == *v;
        if on {
            ui.painter().rect_filled(r, CornerRadius::same(12), Color32::from_rgb(0x4a, 0x4a, 0x52));
        }
        ui.painter().text(r.center(), Align2::CENTER_CENTER, *label, if on { semibold(12.5) } else { FontId::proportional(12.5) }, if !enabled { FAINT } else if on { INK_STRONG } else { MUTED });
        if resp.clicked() {
            *view = *v;
        }
    }
}

fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(semibold(11.0)).color(FAINT));
    ui.add_space(4.0);
}

fn nav_row(ui: &mut Ui, label: &str, count: usize, selected: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, CornerRadius::same(8), BLUE.gamma_multiply(0.24));
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(8), Color32::from_white_alpha(8));
    }
    p.text(rect.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, short(label, 26), if selected { semibold(13.5) } else { FontId::proportional(13.5) }, if selected { INK_STRONG } else { INK });
    if count != usize::MAX {
        p.text(rect.right_center() - vec2(10.0, 0.0), Align2::RIGHT_CENTER, count.to_string(), FontId::proportional(12.0), FAINT);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn dev_slider(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, step: f64) {
    ui.spacing_mut().item_spacing.y = 2.0;
    ui.horizontal(|ui| {
        let r = ui.add(egui::Label::new(RichText::new(label).size(12.5).color(if *v != 0.0 { INK } else { MUTED })).sense(Sense::click()));
        if r.on_hover_text("Double-clic pour remettre à zéro").double_clicked() {
            *v = 0.0;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(egui::DragValue::new(v).range(range.clone()).speed(step).max_decimals(2));
        });
    });
    ui.spacing_mut().slider_width = ui.available_width();
    ui.add(egui::Slider::new(v, range).step_by(step).show_value(false).trailing_fill(false));
    ui.add_space(6.0);
}

fn hue_slider(ui: &mut Ui, label: &str, v: &mut f32) {
    ui.label(RichText::new(label).size(12.5).color(if *v != 0.0 { INK } else { MUTED }));
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::click_and_drag());
    let p = ui.painter();
    let n = 36;
    for k in 0..n {
        let h = k as f32 / n as f32;
        let seg = Rect::from_min_max(pos2(r.left() + r.width() * h, r.top() + 3.0), pos2(r.left() + r.width() * (k + 1) as f32 / n as f32 + 0.5, r.bottom() - 3.0));
        let c = egui::ecolor::Hsva::new(h, 0.8, 0.95, 1.0);
        p.rect_filled(seg, CornerRadius::ZERO, Color32::from(c));
    }
    let x = r.left() + r.width() * (*v / 360.0);
    p.circle(pos2(x, r.center().y), 7.0, Color32::WHITE, Stroke::new(1.5, Color32::from_black_alpha(160)));
    if let Some(pos) = resp.interact_pointer_pos() {
        *v = ((pos.x - r.left()) / r.width()).clamp(0.0, 1.0) * 360.0;
    }
    ui.add_space(8.0);
}

fn draw_curve(ui: &mut Ui, s: &Settings) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 110.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(8), Color32::from_rgb(0x14, 0x14, 0x17));
    for k in 1..4 {
        let f = k as f32 / 4.0;
        let g = Stroke::new(1.0, Color32::from_white_alpha(14));
        p.line_segment([pos2(r.left() + r.width() * f, r.top()), pos2(r.left() + r.width() * f, r.bottom())], g);
        p.line_segment([pos2(r.left(), r.top() + r.height() * f), pos2(r.right(), r.top() + r.height() * f)], g);
    }
    p.line_segment([r.left_bottom(), r.right_top()], Stroke::new(1.0, Color32::from_white_alpha(30)));
    let pts: Vec<egui::Pos2> = (0..=64)
        .map(|k| {
            let x = k as f32 / 64.0;
            let mut y = x;
            for (i, c) in s.curve.iter().enumerate() {
                y += c / 100.0 * 0.18 * (-((x - (0.125 + 0.25 * i as f32)) / 0.15).powi(2)).exp();
            }
            pos2(r.left() + r.width() * x, r.bottom() - r.height() * y.clamp(0.0, 1.0))
        })
        .collect();
    p.add(egui::Shape::line(pts, Stroke::new(2.0, INK)));
    ui.add_space(8.0);
}
fn draw_histogram(ui: &mut Ui, hist: Option<&[[u32; 256]; 4]>) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(8), Color32::from_rgb(0x14, 0x14, 0x17));
    let Some(h) = hist else {
        p.text(r.center(), Align2::CENTER_CENTER, "—", FontId::proportional(14.0), FAINT);
        return;
    };
    let max = h[..3].iter().flat_map(|c| c[2..254].iter()).copied().max().unwrap_or(1).max(1) as f32;
    let inner = r.shrink(6.0);
    for (k, color) in [(0, Color32::from_rgba_unmultiplied(255, 70, 70, 110)), (1, Color32::from_rgba_unmultiplied(70, 220, 110, 110)), (2, Color32::from_rgba_unmultiplied(80, 140, 255, 110))] {
        let mut pts = vec![inner.left_bottom()];
        for (x, v) in h[k].iter().enumerate() {
            let fx = inner.left() + inner.width() * x as f32 / 255.0;
            let fy = inner.bottom() - inner.height() * ((*v as f32 / max).sqrt()).min(1.0);
            pts.push(pos2(fx, fy));
        }
        pts.push(inner.right_bottom());
        let mut mesh = egui::Mesh::default();
        for w in pts.windows(2).skip(1).take(255) {
            let i = mesh.vertices.len() as u32;
            mesh.colored_vertex(w[0], color);
            mesh.colored_vertex(w[1], color);
            mesh.colored_vertex(pos2(w[1].x, inner.bottom()), color);
            mesh.colored_vertex(pos2(w[0].x, inner.bottom()), color);
            mesh.add_triangle(i, i + 1, i + 2);
            mesh.add_triangle(i, i + 2, i + 3);
        }
        p.add(mesh);
    }
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>()) }
}

fn color_image(img: &RgbImage) -> egui::ColorImage {
    egui::ColorImage::from_rgb([img.width() as usize, img.height() as usize], img.as_raw())
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lumen")
            .with_icon(Arc::new(studio_ui::app_window_icon("lumen")))
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([1000.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native("Lumen", options, Box::new(|cc| Ok(Box::new(Lumen::new(cc)))))
}

