#![windows_subsystem = "windows"]

mod develop;

use develop::Settings;
use eframe::egui::{
    self, Color32, CornerRadius, Frame, Key, Margin, Rect, RichText, Sense, Stroke, StrokeKind,
    TextureHandle, TextureOptions, Vec2, pos2, vec2,
};
use image::RgbImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use studio_ui::{BG, CARD, DANGER, MUTED, OK, PANEL, TEXT};

const ACCENT: Color32 = Color32::from_rgb(70, 220, 160);
const PREVIEW_MAX: u32 = 1800;
const THUMB_MAX: u32 = 240;

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Entry {
    settings: Settings,
    rating: u8,
}

#[derive(Default, Serialize, Deserialize)]
struct Catalog {
    folder: Option<PathBuf>,
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
        let _ = std::fs::create_dir_all(p.parent().unwrap());
        let _ = std::fs::write(p, serde_json::to_vec_pretty(self).unwrap_or_default());
    }
}

enum Msg {
    Thumb(PathBuf, RgbImage),
    Preview(PathBuf, Result<RgbImage, String>),
}

#[derive(PartialEq)]
enum View {
    Library,
    Develop,
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
    loading: bool,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    view: View,
    clipboard: Option<Settings>,
    export: Option<Export>,
    error: Option<String>,
    min_rating: u8,
    dirty_catalog: bool,
}

impl Lumen {
    fn new(cc: &eframe::CreationContext) -> Self {
        studio_ui::apply_style(&cc.egui_ctx);
        cc.egui_ctx.all_styles_mut(|s| s.visuals.selection.bg_fill = ACCENT);
        let (tx, rx) = channel();
        let catalog = Catalog::load();
        let mut me = Self {
            catalog,
            files: Vec::new(),
            thumbs: HashMap::new(),
            current: None,
            source: None,
            preview: None,
            shown: None,
            loading: false,
            tx,
            rx,
            view: View::Library,
            clipboard: None,
            export: None,
            error: None,
            min_rating: 0,
            dirty_catalog: false,
        };
        if let Some(f) = me.catalog.folder.clone()
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
        self.catalog.folder = Some(folder);
        self.dirty_catalog = true;
        self.thumbs.clear();
        self.current = None;
        self.source = None;
        self.preview = None;
        self.shown = None;
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

    fn refresh_preview(&mut self, ctx: &egui::Context, before: bool) {
        let Some((path, src)) = &self.source else { return };
        let s = self.catalog.photos.get(path).map(|e| e.settings).unwrap_or_default();
        let key = (path.clone(), s, before);
        if self.shown.as_ref() == Some(&key) {
            return;
        }
        let used = if before { Settings::default() } else { s };
        let img = develop::process(src, &used);
        let ci = color_image(&img);
        match &mut self.preview {
            Some(t) => t.set(ci, TextureOptions::LINEAR),
            None => self.preview = Some(ctx.load_texture("preview", ci, TextureOptions::LINEAR)),
        }
        if !before && let Some(src_thumb) = self.thumbs.get_mut(path) {
            let small = image::imageops::thumbnail(&img, THUMB_MAX, THUMB_MAX);
            src_thumb.set(color_image(&small), TextureOptions::LINEAR);
        }
        self.shown = Some(key);
    }

    fn start_export(&mut self, only_current: bool) {
        let list: Vec<(PathBuf, Settings)> = if only_current {
            self.current.map(|i| self.files[i].clone()).into_iter().map(|p| {
                let s = self.catalog.photos.get(&p).map(|e| e.settings).unwrap_or_default();
                (p, s)
            }).collect()
        } else {
            self.visible().into_iter().map(|i| {
                let p = self.files[i].clone();
                let s = self.catalog.photos.get(&p).map(|e| e.settings).unwrap_or_default();
                (p, s)
            }).collect()
        };
        if list.is_empty() {
            return;
        }
        let Some(dir) = rfd::FileDialog::new().set_title("Dossier d'export").pick_folder() else { return };
        let done = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(Mutex::new(Vec::new()));
        self.export = Some(Export { done: done.clone(), total: list.len(), errors: errors.clone() });
        std::thread::spawn(move || {
            use rayon::prelude::*;
            list.par_iter().for_each(|(p, s)| {
                let r = develop::load(p, 0).and_then(|img| {
                    let out = develop::process(&img, s);
                    let name = p.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                    let dest = dir.join(format!("{name}.jpg"));
                    let file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
                    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), 92);
                    enc.encode_image(&out).map_err(|e| e.to_string())
                });
                if let Err(e) = r {
                    errors.lock().unwrap().push(format!("{} : {e}", p.display()));
                }
                done.fetch_add(1, Ordering::Relaxed);
            });
        });
    }

    fn visible(&self) -> Vec<usize> {
        (0..self.files.len())
            .filter(|&i| self.catalog.photos.get(&self.files[i]).map_or(0, |e| e.rating) >= self.min_rating)
            .collect()
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (rating, next, prev, copy, paste, enter, esc) = ctx.input_mut(|i| {
            use egui::{KeyboardShortcut as K, Modifiers as M};
            let copy = i.consume_shortcut(&K::new(M::COMMAND, Key::C));
            let paste = i.consume_shortcut(&K::new(M::COMMAND, Key::V));
            let mut r = None;
            for (n, k) in [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5].into_iter().enumerate() {
                if i.key_pressed(k) {
                    r = Some(n as u8);
                }
            }
            (r, i.key_pressed(Key::ArrowRight), i.key_pressed(Key::ArrowLeft), copy, paste, i.key_pressed(Key::Enter), i.key_pressed(Key::Escape))
        });
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
                self.clipboard = Some(self.entry(i).settings);
            }
            if paste && let Some(s) = self.clipboard {
                self.entry(i).settings = s;
                self.dirty_catalog = true;
            }
            if enter {
                self.view = View::Develop;
            }
        }
        if esc {
            self.view = View::Library;
        }
    }
}

impl eframe::App for Lumen {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        self.pump(ctx);
        self.keys(ctx);

        egui::Panel::top("top")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 8)))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    studio_ui::app_icon(ui, 26.0, "lumen", ACCENT);
                    ui.label(RichText::new("Lumen").size(17.0).strong().color(TEXT));
                    ui.add_space(12.0);
                    if ui.button("Ouvrir un dossier…").clicked()
                        && let Some(f) = rfd::FileDialog::new().pick_folder()
                    {
                        self.open_folder(f, ctx);
                    }
                    ui.separator();
                    ui.selectable_value(&mut self.view, View::Library, "Bibliothèque");
                    ui.add_enabled_ui(self.current.is_some(), |ui| {
                        ui.selectable_value(&mut self.view, View::Develop, "Développement");
                    });
                    ui.separator();
                    ui.label(RichText::new("Filtre").color(MUTED));
                    egui::ComboBox::from_id_salt("rating")
                        .selected_text(if self.min_rating == 0 { "Toutes".to_string() } else { format!("{}★ et +", self.min_rating) })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.min_rating, 0, "Toutes");
                            for r in 1..=5 {
                                ui.selectable_value(&mut self.min_rating, r, format!("{r}★ et +"));
                            }
                        });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let busy = self.export.as_ref().is_some_and(|e| e.done.load(Ordering::Relaxed) < e.total);
                        ui.add_enabled_ui(!busy, |ui| {
                            if ui.button("Exporter la sélection filtrée…").clicked() {
                                self.start_export(false);
                            }
                            if ui.add_enabled(self.current.is_some(), egui::Button::new("Exporter cette photo…")).clicked() {
                                self.start_export(true);
                            }
                        });
                        if let Some(e) = &self.export {
                            let d = e.done.load(Ordering::Relaxed);
                            let errs = e.errors.lock().unwrap().len();
                            if d < e.total {
                                ui.add(egui::ProgressBar::new(d as f32 / e.total as f32).desired_width(160.0).text(format!("Export {d}/{}", e.total)));
                                ctx.request_repaint_after(std::time::Duration::from_millis(200));
                            } else if errs > 0 {
                                ui.label(RichText::new(format!("Export terminé, {errs} erreur(s)")).color(DANGER));
                            } else {
                                ui.label(RichText::new(format!("{} photo(s) exportée(s)", e.total)).color(OK));
                            }
                        }
                    });
                });
            });

        egui::Panel::bottom("status")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 5)))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                    match &self.catalog.folder {
                        Some(f) => ui.label(small(format!("{} — {} photo(s)", f.display(), self.files.len()))),
                        None => ui.label(small("Aucun dossier ouvert".into())),
                    };
                    ui.separator();
                    ui.label(small("← → naviguer · 0-5 note · Ctrl+C / Ctrl+V réglages · Entrée développer".into()));
                    if let Some(e) = self.error.clone() {
                        ui.separator();
                        ui.label(RichText::new(e).size(12.0).color(DANGER));
                        if ui.small_button("×").clicked() {
                            self.error = None;
                        }
                    }
                });
            });

        if self.files.is_empty() {
            egui::CentralPanel::default().frame(Frame::new().fill(BG)).show(root, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 60.0);
                    studio_ui::app_icon(ui, 64.0, "lumen", ACCENT);
                    ui.add_space(10.0);
                    ui.label(RichText::new("Ouvre un dossier de photos pour commencer").size(16.0).color(TEXT));
                    ui.label(RichText::new("JPEG, PNG, WebP, TIFF et RAW (CR2, NEF, ARW, DNG, RAF, ORF, RW2…)").color(MUTED));
                });
            });
            return;
        }

        match self.view {
            View::Library => self.library(root, ctx),
            View::Develop => self.develop(root, ctx),
        }

        if self.dirty_catalog && ctx.input(|i| !i.pointer.any_down()) {
            self.catalog.save();
            self.dirty_catalog = false;
        }
    }
}

impl Lumen {
    fn library(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::new().fill(BG).inner_margin(Margin::same(16))).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let cell = 200.0;
                let gap = 12.0;
                let cols = ((ui.available_width() + gap) / (cell + gap)).floor().max(1.0) as usize;
                let vis = self.visible();
                for row in vis.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for &i in row {
                            if let Some(clicked) = self.tile(ui, i, cell) {
                                self.select(i, ctx);
                                if clicked {
                                    self.view = View::Develop;
                                }
                            }
                        }
                    });
                    ui.add_space(gap);
                }
            });
        });
    }

    fn tile(&mut self, ui: &mut egui::Ui, i: usize, size: f32) -> Option<bool> {
        let (rect, resp) = ui.allocate_exact_size(vec2(size, size + 26.0), Sense::click());
        let p = ui.painter();
        let on = self.current == Some(i);
        p.rect_filled(rect, CornerRadius::same(10), if on || resp.hovered() { Color32::from_rgb(30, 34, 44) } else { CARD });
        if on {
            p.rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.5, ACCENT), StrokeKind::Inside);
        }
        let img_area = Rect::from_min_size(rect.min + vec2(8.0, 8.0), Vec2::splat(size - 16.0));
        let path = &self.files[i];
        match self.thumbs.get(path) {
            Some(t) => {
                let [tw, th] = t.size();
                let r = (img_area.width() / tw as f32).min(img_area.height() / th as f32);
                let fit = Rect::from_center_size(img_area.center(), vec2(tw as f32, th as f32) * r);
                p.image(t.id(), fit, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            None => {
                p.text(img_area.center(), egui::Align2::CENTER_CENTER, "…", egui::FontId::proportional(20.0), MUTED);
            }
        }
        let entry = self.catalog.photos.get(path).cloned().unwrap_or_default();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let y = rect.max.y - 16.0;
        p.text(pos2(rect.min.x + 10.0, y), egui::Align2::LEFT_CENTER, truncate(&name, 18), egui::FontId::proportional(12.0), MUTED);
        let stars: String = (0..5).map(|k| if k < entry.rating { '★' } else { '☆' }).collect();
        p.text(pos2(rect.max.x - 10.0, y), egui::Align2::RIGHT_CENTER, stars, egui::FontId::proportional(12.0), if entry.rating > 0 { ACCENT } else { MUTED.gamma_multiply(0.5) });
        if !entry.settings.is_default() {
            p.circle_filled(rect.min + vec2(18.0, 18.0), 5.0, ACCENT);
        }
        if resp.double_clicked() {
            Some(true)
        } else if resp.clicked() {
            Some(false)
        } else {
            None
        }
    }

    fn develop(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        let Some(i) = self.current else {
            self.view = View::Library;
            return;
        };

        egui::Panel::bottom("strip")
            .exact_size(118.0)
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 8)))
            .show(root, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for v in self.visible() {
                            let (r, resp) = ui.allocate_exact_size(vec2(100.0, 96.0), Sense::click());
                            let p = ui.painter();
                            if let Some(t) = self.thumbs.get(&self.files[v]) {
                                let [tw, th] = t.size();
                                let s = (r.width() / tw as f32).min(r.height() / th as f32);
                                let fit = Rect::from_center_size(r.center(), vec2(tw as f32, th as f32) * s);
                                p.image(t.id(), fit, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                            }
                            if v == i {
                                p.rect_stroke(r, CornerRadius::same(4), Stroke::new(2.0, ACCENT), StrokeKind::Inside);
                            }
                            if resp.clicked() {
                                self.select(v, ctx);
                            }
                        }
                    });
                });
            });

        let mut before = false;
        egui::Panel::right("dev")
            .resizable(false)
            .exact_size(300.0)
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::same(14)))
            .show(root, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let name = self.files[i].file_name().unwrap_or_default().to_string_lossy().into_owned();
                    ui.label(RichText::new(name).strong().color(TEXT));
                    let clip = self.clipboard;
                    let mut copy_req = false;
                    let e = self.entry(i);
                    let start = e.clone();
                    ui.horizontal(|ui| {
                        for r in 1..=5u8 {
                            let on = e.rating >= r;
                            if ui.add(egui::Label::new(RichText::new(if on { "★" } else { "☆" }).size(18.0).color(if on { ACCENT } else { MUTED })).sense(Sense::click())).clicked() {
                                e.rating = if e.rating == r { 0 } else { r };
                            }
                        }
                    });
                    ui.add_space(8.0);
                    let s = &mut e.settings;
                    section(ui, "Lumière");
                    slider(ui, &mut s.exposure, -4.0..=4.0, "Exposition", 0.01);
                    slider(ui, &mut s.contrast, -100.0..=100.0, "Contraste", 1.0);
                    slider(ui, &mut s.highlights, -100.0..=100.0, "Hautes lumières", 1.0);
                    slider(ui, &mut s.shadows, -100.0..=100.0, "Ombres", 1.0);
                    slider(ui, &mut s.whites, -100.0..=100.0, "Blancs", 1.0);
                    slider(ui, &mut s.blacks, -100.0..=100.0, "Noirs", 1.0);
                    section(ui, "Couleur");
                    slider(ui, &mut s.temperature, -100.0..=100.0, "Température", 1.0);
                    slider(ui, &mut s.tint, -100.0..=100.0, "Teinte", 1.0);
                    slider(ui, &mut s.vibrance, -100.0..=100.0, "Vibrance", 1.0);
                    slider(ui, &mut s.saturation, -100.0..=100.0, "Saturation", 1.0);
                    section(ui, "Effets");
                    slider(ui, &mut s.vignette, -100.0..=100.0, "Vignettage", 1.0);
                    ui.horizontal(|ui| {
                        if ui.button("⟲ 90°").clicked() {
                            s.rotate = (s.rotate + 3) % 4;
                        }
                        if ui.button("⟳ 90°").clicked() {
                            s.rotate = (s.rotate + 1) % 4;
                        }
                    });
                    ui.add_space(14.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Réinitialiser").clicked() {
                            *s = Settings::default();
                        }
                        if ui.button("Copier").clicked() {
                            copy_req = true;
                        }
                        if ui.add_enabled(clip.is_some(), egui::Button::new("Coller")).clicked() {
                            *s = clip.unwrap();
                        }
                    });
                    ui.add_space(8.0);
                    let b = ui.add(egui::Button::new("Maintenir pour voir l'original").sense(Sense::click_and_drag()));
                    before = b.is_pointer_button_down_on() || ui.input(|i| i.key_down(Key::Backslash));
                    if copy_req {
                        self.clipboard = Some(self.entry(i).settings);
                    }
                    if self.catalog.photos.get(&self.files[i]).map(|e| (e.settings, e.rating)) != Some((start.settings, start.rating)) {
                        self.dirty_catalog = true;
                    }
                });
            });

        self.refresh_preview(ctx, before);

        egui::CentralPanel::default().frame(Frame::new().fill(Color32::from_rgb(8, 9, 12))).show(root, |ui| {
            let area = ui.max_rect().shrink(20.0);
            if let Some(t) = &self.preview
                && self.source.as_ref().is_some_and(|(p, _)| *p == self.files[i])
            {
                let [w, h] = t.size();
                let s = (area.width() / w as f32).min(area.height() / h as f32);
                let r = Rect::from_center_size(area.center(), vec2(w as f32, h as f32) * s);
                ui.painter().image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                if before {
                    ui.painter().text(r.left_top() + vec2(12.0, 12.0), egui::Align2::LEFT_TOP, "Original", egui::FontId::proportional(14.0), Color32::WHITE);
                }
            } else if self.loading {
                ui.painter().text(area.center(), egui::Align2::CENTER_CENTER, "Chargement…", egui::FontId::proportional(16.0), MUTED);
            }
        });
    }
}

fn color_image(img: &RgbImage) -> egui::ColorImage {
    egui::ColorImage::from_rgb([img.width() as usize, img.height() as usize], img.as_raw())
}

fn section(ui: &mut egui::Ui, t: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(t.to_uppercase()).size(11.0).color(MUTED));
    ui.add_space(2.0);
}

fn slider(ui: &mut egui::Ui, v: &mut f32, range: std::ops::RangeInclusive<f32>, label: &str, step: f64) {
    ui.horizontal(|ui| {
        let r = ui.add(egui::Slider::new(v, range).step_by(step).text(label));
        if r.double_clicked() {
            *v = 0.0;
        }
    });
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lumen")
            .with_icon(Arc::new(studio_ui::app_window_icon("lumen")))
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("Lumen", options, Box::new(|cc| Ok(Box::new(Lumen::new(cc)))))
}
