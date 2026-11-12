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
    flag: i8,
    label: u8,
}

const LABELS: [(&str, [u8; 3]); 4] = [("Rouge", [255, 69, 58]), ("Jaune", [255, 204, 0]), ("Vert", [48, 209, 88]), ("Bleu", [10, 132, 255])];
const SORTS: [&str; 3] = ["Nom", "Note", "Date"];

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Catalog {
    folder: Option<PathBuf>,
    recent_folders: Vec<PathBuf>,
    photos: HashMap<PathBuf, Entry>,
    user_presets: Vec<(String, Settings)>,
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
    Picks,
    Rejected,
    Label(u8),
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
    export_format: usize,
    export_suffix: String,
    query: String,
    sort: usize,
    picked: Vec<usize>,
    history: Vec<(usize, Settings)>,
    future: Vec<(usize, Settings)>,
    editing: bool,
    split_view: bool,
    split_pos: f32,
    before_tex: Option<(PathBuf, TextureHandle)>,
    clipping: bool,
    clip_tex: Option<TextureHandle>,
    full_zoom: bool,
    pan: Vec2,
    preset_name: Option<String>,
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
            export_format: 0,
            export_suffix: String::new(),
            query: String::new(),
            sort: 0,
            picked: Vec::new(),
            history: Vec::new(),
            future: Vec::new(),
            editing: false,
            split_view: false,
            split_pos: 0.5,
            before_tex: None,
            clipping: false,
            clip_tex: None,
            full_zoom: false,
            pan: Vec2::ZERO,
            preset_name: None,
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
        if self.split_view && self.before_tex.as_ref().is_none_or(|(p, _)| p != path) {
            let orig = develop::process(src, &Settings { crop: s.crop, straighten: s.straighten, rotate: s.rotate, flip: s.flip, distortion: s.distortion, ..Settings::default() });
            self.before_tex = Some((path.clone(), ctx.load_texture("before", color_image(&orig), TextureOptions::LINEAR)));
        }
        if self.shown.as_ref() == Some(&key) {
            return;
        }
        let used = if self.before { Settings::default() } else { s };
        let img = develop::process(src, &used);
        self.hist = Some(develop::histogram(&img));
        self.clip_tex = self.clipping.then(|| {
            let mut px = Vec::with_capacity((img.width() * img.height() * 4) as usize);
            for p in img.pixels() {
                let c = p.0;
                px.extend_from_slice(&if c.iter().any(|v| *v >= 254) { [255, 40, 40, 200] } else if c.iter().all(|v| *v <= 2) { [40, 120, 255, 200] } else { [0, 0, 0, 0] });
            }
            ctx.load_texture("clip", egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], &px), TextureOptions::NEAREST)
        });
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
        let ids: Vec<usize> = if only_current { self.current.into_iter().collect() } else if self.picked.len() > 1 { self.picked.clone() } else { self.visible() };
        let list: Vec<(PathBuf, Settings)> = ids.iter().map(|&i| (self.files[i].clone(), self.get(i).settings)).collect();
        if list.is_empty() {
            return;
        }
        let Some(dir) = rfd::FileDialog::new().set_title("Dossier d'export").pick_folder() else { return };
        let (quality, edge, format, suffix) = (self.export_quality, self.export_edge, self.export_format, self.export_suffix.clone());
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
                    let ext = ["jpg", "png", "tif"][format.min(2)];
                    let target = dir.join(format!("{name}{suffix}.{ext}"));
                    if format == 0 {
                        let file = std::fs::File::create(&target).map_err(|e| e.to_string())?;
                        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), quality);
                        enc.encode_image(&out).map_err(|e| e.to_string())
                    } else {
                        out.save(&target).map_err(|e| e.to_string())
                    }
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
        let q = self.query.to_lowercase();
        let mut v: Vec<usize> = (0..self.files.len())
            .filter(|&i| {
                let e = self.get(i);
                let name_ok = q.is_empty() || self.files[i].file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().contains(&q));
                name_ok
                    && e.rating >= self.min_rating
                    && match self.collection {
                        Collection::All => e.flag >= 0,
                        Collection::Rated => e.rating >= 3,
                        Collection::Edited => !e.settings.is_default(),
                        Collection::Picks => e.flag > 0,
                        Collection::Rejected => e.flag < 0,
                        Collection::Label(l) => e.label == l,
                    }
            })
            .collect();
        match self.sort {
            1 => v.sort_by_key(|&i| std::cmp::Reverse(self.get(i).rating)),
            2 => v.sort_by_key(|&i| std::fs::metadata(&self.files[i]).and_then(|m| m.modified()).ok()),
            _ => {}
        }
        v
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
        let (pk, xk, uk, lab, undo, redo, zk, jk, yk) = ctx.input_mut(|i| {
            use egui::{KeyboardShortcut as K, Modifiers as M};
            let undo = i.consume_shortcut(&K::new(M::COMMAND, Key::Z));
            let redo = i.consume_shortcut(&K::new(M::COMMAND, Key::Y)) || i.consume_shortcut(&K::new(M::COMMAND | M::SHIFT, Key::Z));
            let plain = !i.modifiers.command;
            let mut lab = None;
            for (n, k) in [Key::Num6, Key::Num7, Key::Num8, Key::Num9].into_iter().enumerate() {
                if i.key_pressed(k) && plain {
                    lab = Some(n as u8 + 1);
                }
            }
            (i.key_pressed(Key::P) && plain, i.key_pressed(Key::X) && plain, i.key_pressed(Key::U) && plain, lab, undo, redo, i.key_pressed(Key::Z) && plain, i.key_pressed(Key::J) && plain, i.key_pressed(Key::Y) && plain)
        });
        let targets: Vec<usize> = if self.picked.len() > 1 { self.picked.clone() } else { self.current.into_iter().collect() };
        if pk || xk || uk || lab.is_some() {
            for &t in &targets {
                let e = self.entry(t);
                if pk {
                    e.flag = 1;
                }
                if xk {
                    e.flag = -1;
                }
                if uk {
                    e.flag = 0;
                }
                if let Some(l) = lab {
                    e.label = if e.label == l { 0 } else { l };
                }
            }
            self.dirty_catalog = true;
        }
        if undo
            && let Some((idx, st)) = self.history.pop()
            && idx < self.files.len()
        {
            let cur = self.get(idx).settings;
            self.future.push((idx, cur));
            self.entry(idx).settings = st;
            self.dirty_catalog = true;
        }
        if redo
            && let Some((idx, st)) = self.future.pop()
            && idx < self.files.len()
        {
            let cur = self.get(idx).settings;
            self.history.push((idx, cur));
            self.entry(idx).settings = st;
            self.dirty_catalog = true;
        }
        if zk {
            self.full_zoom = !self.full_zoom;
            self.pan = Vec2::ZERO;
        }
        if jk {
            self.clipping = !self.clipping;
            self.shown = None;
        }
        if yk {
            self.split_view = !self.split_view;
        }
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
        if ctx.input(|i| !i.pointer.any_down()) && !ctx.egui_wants_keyboard_input() {
            self.editing = false;
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
                            ui.label(RichText::new("Format").size(12.5).color(MUTED));
                            pro::segmented(ui, &mut self.export_format, &["JPEG", "PNG", "TIFF"]);
                            ui.label(RichText::new("Suffixe du nom").size(12.5).color(MUTED));
                            ui.add(egui::TextEdit::singleline(&mut self.export_suffix).hint_text("ex. -web").desired_width(140.0));
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
                        let label = if self.picked.len() > 1 { format!("Exporter {}", self.picked.len()) } else if self.view == View::Develop { "Exporter".to_string() } else { format!("Exporter {}", self.visible().len()) };
                        let dev = self.view == View::Develop && self.picked.len() < 2;
                        if pro::primary_icon(ui, &label, |p, r, c| export_glyph(p, r, c)).clicked() {
                            self.start_export(dev);
                        }
                        ui.add_space(8.0);
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
                ui.label(small("← → naviguer · 0-5 noter · P/X drapeau · 6-9 couleur · D développer · G bibliothèque · Y avant|après · J écrêtage · Z 100 %".into()));
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
            let picks = (0..all).filter(|&i| self.get(i).flag > 0).count();
            let rejected = (0..all).filter(|&i| self.get(i).flag < 0).count();
            for (c, label, n) in [(Collection::All, "Toutes les photos", all), (Collection::Picks, "Retenues", picks), (Collection::Rated, "Notées 3★ et plus", rated), (Collection::Edited, "Modifiées", edited), (Collection::Rejected, "Rejetées", rejected)] {
                if nav_row(ui, label, n, self.collection == c) {
                    self.collection = c;
                }
            }
            ui.add_space(12.0);
            heading(ui, "ÉTIQUETTES");
            ui.horizontal(|ui| {
                for (k, (name, rgb)) in LABELS.iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
                    ui.painter().circle_filled(r.center(), 9.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                    if self.collection == Collection::Label(k as u8 + 1) {
                        ui.painter().circle_stroke(r.center(), 11.5, Stroke::new(1.5, Color32::WHITE));
                    }
                    let n = (0..all).filter(|&i| self.get(i).label == k as u8 + 1).count();
                    if resp.on_hover_text(format!("{name} · {n} photo(s) · touche {}", k + 6)).clicked() {
                        self.collection = Collection::Label(k as u8 + 1);
                    }
                }
            });
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
                ui.add_space(10.0);
                pro::toolbar_sep(ui);
                ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Rechercher un nom").desired_width(150.0));
                ui.label(RichText::new("Tri").size(12.5).color(MUTED));
                pro::segmented(ui, &mut self.sort, &SORTS);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().slider_width = 110.0;
                    ui.add(egui::Slider::new(&mut self.thumb_size, 120.0..=340.0).show_value(false));
                    ui.label(RichText::new("Taille").size(12.5).color(MUTED));
                });
            });
        });
        if self.picked.len() > 1 {
            egui::Panel::top("lib-sel").exact_size(44.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(18, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new(format!("{} photos sélectionnées", self.picked.len())).font(semibold(13.0)).color(INK_STRONG));
                    pro::toolbar_sep(ui);
                    let picked = self.picked.clone();
                    if pro::secondary(ui, "Synchroniser les réglages").on_hover_text("Copie les réglages de la photo active sur la sélection").clicked()
                        && let Some(c) = self.current
                    {
                        let st = self.get(c).settings;
                        for &p in &picked {
                            self.entry(p).settings = st;
                        }
                        self.dirty_catalog = true;
                    }
                    ui.menu_button("Appliquer un préréglage", |ui| {
                        let all: Vec<(String, Settings)> = develop::presets().into_iter().map(|(n, s)| (n.to_string(), s)).chain(self.catalog.user_presets.clone()).collect();
                        for (name, st) in all {
                            if ui.button(name).clicked() {
                                for &p in &picked {
                                    self.entry(p).settings = st;
                                }
                                self.dirty_catalog = true;
                                ui.close();
                            }
                        }
                    });
                    if pro::secondary(ui, "Réinitialiser").clicked() {
                        for &p in &picked {
                            self.entry(p).settings = Settings::default();
                        }
                        self.dirty_catalog = true;
                    }
                    for (label, flag) in [("Retenir", 1i8), ("Rejeter", -1)] {
                        if pro::ghost(ui, label).clicked() {
                            for &p in &picked {
                                self.entry(p).flag = flag;
                            }
                            self.dirty_catalog = true;
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::ghost(ui, "Tout désélectionner").clicked() {
                            self.picked.clear();
                        }
                    });
                });
            });
        }
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
                                    self.picked.clear();
                                    self.select(i, ctx);
                                    self.view = View::Develop;
                                }
                                Some(false) => {
                                    let (ctrl, shift) = ui.input(|i| (i.modifiers.command, i.modifiers.shift));
                                    if ctrl {
                                        if self.picked.is_empty()
                                            && let Some(c) = self.current
                                        {
                                            self.picked.push(c);
                                        }
                                        if let Some(k) = self.picked.iter().position(|p| *p == i) {
                                            self.picked.remove(k);
                                        } else {
                                            self.picked.push(i);
                                        }
                                    } else if shift && let Some(c) = self.current {
                                        let (a, b) = (vis.iter().position(|v| *v == c).unwrap_or(0), vis.iter().position(|v| *v == i).unwrap_or(0));
                                        self.picked = vis[a.min(b)..=a.max(b)].to_vec();
                                    } else {
                                        self.picked.clear();
                                        self.select(i, ctx);
                                    }
                                }
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
        let on = self.current == Some(i) || self.picked.contains(&i);
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
        if e.flag < 0 {
            p.rect_filled(img_area, CornerRadius::same(6), Color32::from_black_alpha(150));
            p.text(img_area.right_top() + vec2(-14.0, 14.0), Align2::CENTER_CENTER, "✕", semibold(14.0), RED);
        } else if e.flag > 0 {
            let b = Rect::from_center_size(img_area.right_top() + vec2(-14.0, 14.0), Vec2::splat(20.0));
            p.circle_filled(b.center(), 10.0, GREEN);
            p.text(b.center(), Align2::CENTER_CENTER, "✓", semibold(12.0), Color32::WHITE);
        }
        if e.label > 0 {
            let c = LABELS[(e.label - 1) as usize % LABELS.len()].1;
            p.rect_filled(Rect::from_min_max(pos2(rect.left() + 8.0, rect.bottom() - 4.0), pos2(rect.right() - 8.0, rect.bottom() - 1.0)), CornerRadius::same(2), Color32::from_rgb(c[0], c[1], c[2]));
        }
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
            ui.add_space(14.0);
            heading(ui, "MES PRÉRÉGLAGES");
            let mut remove = None;
            for (k, (name, s)) in self.catalog.user_presets.clone().into_iter().enumerate() {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                let on = current == s;
                if on {
                    ui.painter().rect_filled(rect, CornerRadius::same(8), BLUE.gamma_multiply(0.24));
                } else if resp.hovered() {
                    ui.painter().rect_filled(rect, CornerRadius::same(8), Color32::from_white_alpha(8));
                }
                ui.painter().text(rect.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, short(&name, 24), FontId::proportional(13.5), if on { INK_STRONG } else { INK });
                let resp = resp.on_hover_text("Clic droit : supprimer");
                if resp.clicked() {
                    self.entry(i).settings = s;
                    self.dirty_catalog = true;
                }
                resp.context_menu(|ui| {
                    if ui.button("Supprimer ce préréglage").clicked() {
                        remove = Some(k);
                        ui.close();
                    }
                });
            }
            if let Some(k) = remove {
                self.catalog.user_presets.remove(k);
                self.dirty_catalog = true;
            }
            match &mut self.preset_name {
                Some(name) => {
                    let r = ui.add(egui::TextEdit::singleline(name).hint_text("Nom du préréglage").desired_width(f32::INFINITY));
                    r.request_focus();
                    if r.lost_focus() {
                        let n = name.trim().to_string();
                        if !n.is_empty() {
                            self.catalog.user_presets.push((n, current));
                            self.dirty_catalog = true;
                        }
                        self.preset_name = None;
                    }
                }
                None => {
                    if pro::ghost(ui, "+ Enregistrer les réglages actuels").clicked() {
                        self.preset_name = Some(String::new());
                    }
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
                    ui.add_space(4.0);
                    ui.label(RichText::new("Balance des blancs").size(12.0).color(FAINT));
                    ui.horizontal_wrapped(|ui| {
                        for (name, t, ti) in develop::WB_PRESETS {
                            if ui.small_button(name).clicked() {
                                (s.temperature, s.tint) = (t, ti);
                            }
                        }
                        if ui.small_button("Auto").on_hover_text("Neutralise la dominante de couleur").clicked()
                            && let Some((_, src)) = &self.source
                        {
                            (s.temperature, s.tint) = develop::auto_white_balance(src);
                        }
                    });
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
                pro::dock_panel(ui, "split", "Étalonnage", |ui| {
                    hue_slider(ui, "Teinte des ombres", &mut s.split_shadow_hue);
                    dev_slider(ui, "Saturation des ombres", &mut s.split_shadow_sat, 0.0..=100.0, 1.0);
                    hue_slider(ui, "Teinte des tons moyens", &mut s.split_mid_hue);
                    dev_slider(ui, "Saturation des tons moyens", &mut s.split_mid_sat, 0.0..=100.0, 1.0);
                    hue_slider(ui, "Teinte des hautes lumières", &mut s.split_high_hue);
                    dev_slider(ui, "Saturation des hautes lumières", &mut s.split_high_sat, 0.0..=100.0, 1.0);
                    dev_slider(ui, "Équilibre", &mut s.split_balance, -100.0..=100.0, 1.0);
                });
                pro::dock_panel(ui, "detail", "Détail", |ui| {
                    dev_slider(ui, "Netteté", &mut s.sharpen, 0.0..=150.0, 1.0);
                    dev_slider(ui, "Rayon", &mut s.sharpen_radius, 0.0..=3.0, 0.1);
                    dev_slider(ui, "Réduction du bruit", &mut s.noise, 0.0..=100.0, 1.0);
                });
                pro::dock_panel(ui, "masks", "Masques", |ui| {
                    ui.checkbox(&mut s.grad_on, RichText::new("Filtre gradué").font(semibold(12.5)));
                    if s.grad_on {
                        dev_slider(ui, "Angle", &mut s.grad_angle, -180.0..=180.0, 1.0);
                        let mut pos = s.grad_pos * 100.0;
                        dev_slider(ui, "Position", &mut pos, 0.0..=100.0, 1.0);
                        s.grad_pos = pos / 100.0;
                        let mut f = s.grad_feather * 100.0;
                        dev_slider(ui, "Fondu", &mut f, 2.0..=100.0, 1.0);
                        s.grad_feather = f / 100.0;
                        dev_slider(ui, "Exposition", &mut s.grad_exposure, -3.0..=3.0, 0.01);
                        dev_slider(ui, "Température", &mut s.grad_temp, -100.0..=100.0, 1.0);
                        dev_slider(ui, "Saturation", &mut s.grad_sat, -100.0..=100.0, 1.0);
                        ui.label(RichText::new("Glisse sur la photo pour placer la transition.").size(11.5).color(FAINT));
                    }
                    ui.add_space(6.0);
                    ui.checkbox(&mut s.radial_on, RichText::new("Filtre radial").font(semibold(12.5)));
                    if s.radial_on {
                        let mut size = s.radial_size * 100.0;
                        dev_slider(ui, "Taille", &mut size, 2.0..=100.0, 1.0);
                        s.radial_size = size / 100.0;
                        let mut f = s.radial_feather * 100.0;
                        dev_slider(ui, "Fondu", &mut f, 2.0..=100.0, 1.0);
                        s.radial_feather = f / 100.0;
                        dev_slider(ui, "Exposition", &mut s.radial_exposure, -3.0..=3.0, 0.01);
                        dev_slider(ui, "Température", &mut s.radial_temp, -100.0..=100.0, 1.0);
                        dev_slider(ui, "Saturation", &mut s.radial_sat, -100.0..=100.0, 1.0);
                        ui.checkbox(&mut s.radial_invert, "Agir à l'extérieur");
                        ui.label(RichText::new("Glisse sur la photo pour déplacer le centre.").size(11.5).color(FAINT));
                    }
                    if s.grad_on && s.grad_feather == 0.0 {
                        (s.grad_pos, s.grad_feather) = (0.5, 0.35);
                    }
                    if s.radial_on && s.radial_size == 0.0 {
                        (s.radial_x, s.radial_y, s.radial_size, s.radial_feather) = (0.5, 0.5, 0.3, 0.5);
                    }
                });
                pro::dock_panel(ui, "optics", "Optique", |ui| {
                    dev_slider(ui, "Correction du vignetage", &mut s.lens_vignette, -100.0..=100.0, 1.0);
                    dev_slider(ui, "Distorsion", &mut s.distortion, -100.0..=100.0, 1.0);
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
                    ui.label(RichText::new("Format").size(12.0).color(FAINT));
                    ui.horizontal_wrapped(|ui| {
                        for (name, ratio) in develop::RATIOS {
                            if ui.small_button(name).clicked() {
                                s.crop = match &self.source {
                                    Some((_, src)) if ratio > 0.0 => {
                                        let (w, h) = (src.width() as f32, src.height() as f32);
                                        let (w, h) = if s.rotate % 2 == 1 { (h, w) } else { (w, h) };
                                        let c = develop::crop_for_ratio(w, h, ratio);
                                        if s.rotate % 2 == 1 { [c[1], c[0], c[3], c[2]] } else { c }
                                    }
                                    _ => [0.0; 4],
                                };
                            }
                        }
                    });
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
                if s != start.0 && !self.editing {
                    self.history.push((i, start.0));
                    if self.history.len() > 200 {
                        self.history.remove(0);
                    }
                    self.future.clear();
                    self.editing = true;
                }
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
                        let held = ui.add(egui::Button::new(RichText::new("Maintenir : original").size(12.5)).corner_radius(CornerRadius::same(7)).sense(Sense::click_and_drag()));
                        if held.is_pointer_button_down_on() {
                            self.before = true;
                        }
                        held.on_hover_text("Maintenir pour voir l'original (ou la touche \\)");
                        ui.add_space(6.0);
                        if ui.selectable_label(self.split_view, "Avant | Après").on_hover_text("Comparaison côte à côte (Y)").clicked() {
                            self.split_view = !self.split_view;
                        }
                        if ui.selectable_label(self.clipping, "Écrêtage").on_hover_text("Rouge : blancs brûlés · bleu : noirs bouchés (J)").clicked() {
                            self.clipping = !self.clipping;
                            self.shown = None;
                        }
                        if ui.selectable_label(self.full_zoom, "100 %").on_hover_text("Zoom 1:1, glisse pour te déplacer (Z ou double-clic)").clicked() {
                            self.full_zoom = !self.full_zoom;
                            self.pan = Vec2::ZERO;
                        }
                    });
                });
            });
            let area = Rect::from_min_max(pos2(full.left(), bar.bottom()), full.max).shrink(24.0);
            let p = ui.painter();
            if let Some(t) = &self.preview
                && self.source.as_ref().is_some_and(|(sp, _)| *sp == self.files[i])
            {
                let [w, h] = t.size();
                let fit = (area.width() / w as f32).min(area.height() / h as f32);
                let scale = if self.full_zoom { 1.0 / ui.ctx().pixels_per_point() } else { fit };
                let r = Rect::from_center_size(area.center() + if self.full_zoom { self.pan } else { Vec2::ZERO }, vec2(w as f32, h as f32) * scale);
                let painter = ui.painter_at(area.expand(24.0));
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 26, spread: 0, color: Color32::from_black_alpha(140) }.as_shape(r, CornerRadius::ZERO));
                painter.image(t.id(), r, uv, Color32::WHITE);
                if let Some(ct) = &self.clip_tex {
                    painter.image(ct.id(), r, uv, Color32::WHITE);
                }
                let st = self.catalog.photos.get(&self.files[i]).map(|e| e.settings).unwrap_or_default();
                if st.radial_on {
                    let c = pos2(r.left() + st.radial_x * r.width(), r.top() + st.radial_y * r.height());
                    let rad = st.radial_size * r.width();
                    painter.circle_stroke(c, rad, Stroke::new(1.5, Color32::WHITE));
                    painter.circle_stroke(c, rad * (1.0 - st.radial_feather.clamp(0.02, 1.0)), Stroke::new(1.0, Color32::from_white_alpha(110)));
                    painter.circle(c, 5.0, BLUE, Stroke::new(1.5, Color32::WHITE));
                }
                if st.grad_on {
                    let a = st.grad_angle.to_radians();
                    let (gdx, gdy) = (a.sin(), -a.cos());
                    let off = 0.5 - st.grad_pos;
                    let c = pos2(r.left() + (0.5 - off * gdx) * r.width(), r.top() + (0.5 - off * gdy) * r.height());
                    let dir = vec2(-gdy * r.width(), gdx * r.height());
                    painter.line_segment([c - dir, c + dir], Stroke::new(1.5, Color32::WHITE));
                    painter.circle(c, 5.0, BLUE, Stroke::new(1.5, Color32::WHITE));
                }
                if self.split_view
                    && let Some((bp, bt)) = &self.before_tex
                    && *bp == self.files[i]
                {
                    let sx = r.left() + r.width() * self.split_pos;
                    painter.image(bt.id(), Rect::from_min_max(r.min, pos2(sx, r.bottom())), Rect::from_min_max(pos2(0.0, 0.0), pos2(self.split_pos, 1.0)), Color32::WHITE);
                    painter.line_segment([pos2(sx, r.top()), pos2(sx, r.bottom())], Stroke::new(2.0, Color32::WHITE));
                    painter.circle(pos2(sx, r.center().y), 12.0, Color32::from_black_alpha(160), Stroke::new(2.0, Color32::WHITE));
                    painter.text(pos2(sx, r.center().y), Align2::CENTER_CENTER, "⇔", FontId::proportional(12.0), Color32::WHITE);
                    for (txt, x, al) in [("Avant", r.left() + 10.0, Align2::LEFT_TOP), ("Après", r.right() - 10.0, Align2::RIGHT_TOP)] {
                        painter.text(pos2(x, r.top() + 10.0), al, txt, semibold(12.5), Color32::WHITE);
                    }
                    let handle = ui.interact(Rect::from_center_size(pos2(sx, r.center().y), vec2(28.0, r.height())), ui.make_persistent_id("split"), Sense::drag());
                    if handle.hovered() || handle.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                    if let Some(pp) = handle.interact_pointer_pos() {
                        self.split_pos = ((pp.x - r.left()) / r.width()).clamp(0.02, 0.98);
                    }
                }
                let img = ui.interact(r, ui.make_persistent_id("photo"), Sense::click_and_drag());
                if img.double_clicked() {
                    self.full_zoom = !self.full_zoom;
                    self.pan = Vec2::ZERO;
                } else if img.dragged() {
                    if self.full_zoom {
                        self.pan += img.drag_delta();
                    } else if (st.radial_on || st.grad_on)
                        && let Some(pp) = img.interact_pointer_pos()
                    {
                        let (u, v) = (((pp.x - r.left()) / r.width()).clamp(0.0, 1.0), ((pp.y - r.top()) / r.height()).clamp(0.0, 1.0));
                        let e = self.catalog.photos.entry(self.files[i].clone()).or_default();
                        if st.radial_on {
                            (e.settings.radial_x, e.settings.radial_y) = (u, v);
                        } else {
                            let a = st.grad_angle.to_radians();
                            e.settings.grad_pos = ((u - 0.5) * a.sin() + (v - 0.5) * -a.cos() + 0.5).clamp(0.0, 1.0);
                        }
                        self.dirty_catalog = true;
                    }
                }
                if self.full_zoom && img.hovered() {
                    ui.ctx().set_cursor_icon(if img.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
                }
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

fn export_glyph(p: &egui::Painter, r: Rect, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    p.line_segment([at(0.5, 0.05), at(0.5, 0.65)], Stroke::new(2.0, c));
    p.line_segment([at(0.5, 0.05), at(0.25, 0.3)], Stroke::new(2.0, c));
    p.line_segment([at(0.5, 0.05), at(0.75, 0.3)], Stroke::new(2.0, c));
    p.add(egui::Shape::line(vec![at(0.12, 0.55), at(0.12, 0.95), at(0.88, 0.95), at(0.88, 0.55)], Stroke::new(2.0, c)));
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

