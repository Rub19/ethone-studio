#![windows_subsystem = "windows"]

mod optlib;
mod outils;
mod develop;
mod devui;
mod library;
mod lightroom;
mod meta;
mod tools;
mod tutos;
mod widgets;

use develop::Settings;
use eframe::egui::{self, Align, Color32, CornerRadius, Frame, Key, Layout, Margin, Rect, RichText, Stroke, TextureHandle, TextureOptions, Ui, Vec2, vec2};
use image::RgbImage;
use meta::ExportOpts;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use studio_ui::pro::{self, BLUE, CHROME, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PASTEBOARD, RED, semibold};
use tools::{Drag, Tool};
use widgets::{color_image, heading};

const PREVIEW_MAX: u32 = 1800;
const THUMB_MAX: u32 = 320;
const LARGE_MAX: u32 = 1200;
const STAR: Color32 = Color32::from_rgb(255, 196, 0);

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Entry {
    settings: Settings,
    rating: u8,
    flag: i8,
    label: u8,
    keywords: Vec<String>,
    title: String,
    caption: String,
    stack: u32,
    snapshots: Vec<(String, Settings)>,
}

const LABELS: [(&str, [u8; 3]); 5] = [("Rouge", [255, 69, 58]), ("Jaune", [255, 204, 0]), ("Vert", [48, 209, 88]), ("Bleu", [10, 132, 255]), ("Violet", [191, 90, 242])];
const SORTS: [&str; 7] = ["Nom", "Note", "Date de modification", "Prise de vue", "Taille du fichier", "Type de fichier", "Étiquette"];

#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
struct Smart {
    name: String,
    min_rating: u8,
    flag: i8,
    label: u8,
    keyword: String,
    text: String,
    edited: u8,
    kind: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Catalog {
    folder: Option<PathBuf>,
    recent_folders: Vec<PathBuf>,
    photos: HashMap<PathBuf, Entry>,
    user_presets: Vec<(String, Settings)>,
    smart: Vec<Smart>,
    virtual_copies: HashMap<PathBuf, u32>,
    collapsed: Vec<u32>,
    next_stack: u32,
    recursive: bool,
    export: ExportOpts,
    solo: bool,
    favorites: Vec<String>,
}

impl Default for Catalog {
    fn default() -> Self {
        Catalog {
            folder: None,
            recent_folders: Vec::new(),
            photos: HashMap::new(),
            user_presets: Vec::new(),
            smart: Vec::new(),
            virtual_copies: HashMap::new(),
            collapsed: Vec::new(),
            next_stack: 1,
            recursive: false,
            export: ExportOpts::default(),
            solo: false,
            favorites: Vec::new(),
        }
    }
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

pub fn real(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    match s.split_once('|') {
        Some((a, _)) => PathBuf::from(a),
        None => p.to_path_buf(),
    }
}

pub fn copy_index(p: &Path) -> Option<u32> {
    p.to_string_lossy().split_once("|v").and_then(|(_, n)| n.parse().ok())
}

pub fn display_name(p: &Path) -> String {
    let name = real(p).file_name().unwrap_or_default().to_string_lossy().into_owned();
    match copy_index(p) {
        Some(n) => format!("{name} (copie {n})"),
        None => name,
    }
}

enum Msg {
    Thumb(PathBuf, RgbImage),
    Preview(PathBuf, Result<RgbImage, String>),
    Large(PathBuf, RgbImage),
}

#[derive(PartialEq, Clone, Copy)]
enum View {
    Library,
    Develop,
    Compare,
    Survey,
}

#[derive(PartialEq, Clone)]
enum Collection {
    All,
    Rated,
    Unrated,
    Edited,
    Picks,
    Rejected,
    Virtual,
    Label(u8),
    Smart(usize),
    Keyword(String),
}

struct Export {
    done: Arc<AtomicUsize>,
    total: usize,
    errors: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone, Copy)]
struct FileInfo {
    size: u64,
    modified: Option<std::time::SystemTime>,
}

struct Lumen {
    catalog: Catalog,
    files: Vec<PathBuf>,
    info: HashMap<PathBuf, FileInfo>,
    thumbs: HashMap<PathBuf, TextureHandle>,
    current: Option<usize>,
    source: Option<(PathBuf, RgbImage)>,
    preview: Option<TextureHandle>,
    shown: Option<(PathBuf, Settings, bool, bool)>,
    rendered: Option<RgbImage>,
    hist: Option<[[u32; 256]; 4]>,
    loading: bool,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    view: View,
    collection: Collection,
    min_rating: u8,
    rating_mode: usize,
    thumb_size: f32,
    before: bool,
    clipboard: Option<Settings>,
    export: Option<Export>,
    error: Option<String>,
    notice: Option<(String, f64)>,
    dirty_catalog: bool,
    icon: TextureHandle,
    help: studio_ui::help::Help,
    inbox: studio_ui::handoff::Receiver,
    palette: studio_ui::palette::Palette,
    hsl_tab: usize,
    curve_channel: usize,
    query: String,
    sort: usize,
    sort_desc: bool,
    picked: Vec<usize>,
    history: Vec<(usize, Settings, String)>,
    future: Vec<(usize, Settings, String)>,
    skip_history: bool,
    editing: bool,
    compare: usize,
    split_pos: f32,
    before_tex: Option<(PathBuf, Settings, TextureHandle)>,
    clipping: bool,
    clip_tex: Option<TextureHandle>,
    zoom: f32,
    pan: Vec2,
    preset_name: Option<String>,
    preset_amount: f32,
    last_preset: Option<(Settings, Settings)>,
    tool: Tool,
    brush_size: f32,
    brush_feather: f32,
    brush_flow: f32,
    brush_erase: bool,
    spot_size: f32,
    spot_feather: f32,
    spot_opacity: f32,
    eye_pupil: f32,
    eye_darken: f32,
    sel_spot: Option<usize>,
    sel_eye: Option<usize>,
    sel_mask: Option<usize>,
    sel_part: usize,
    target_mode: usize,
    crop_overlay: usize,
    crop_ratio: usize,
    crop_lock: bool,
    drag: Drag,
    show_overlay: bool,
    overlay_color: usize,
    mask_tex: Option<(PathBuf, Settings, usize, TextureHandle)>,
    export_sheet: bool,
    export_only_current: bool,
    copy_sheet: bool,
    copy_groups: Vec<bool>,
    keyword_input: String,
    smart_edit: Option<(Option<usize>, Smart)>,
    large: HashMap<PathBuf, RgbImage>,
    large_tex: HashMap<PathBuf, (Settings, TextureHandle)>,
    large_pending: Vec<PathBuf>,
    exif_cache: HashMap<PathBuf, Vec<(&'static str, String)>>,
    info_overlay: usize,
    hide_panels: bool,
    snapshot_name: Option<String>,
    rename_mask: Option<String>,
    hover_rgb: Option<[u8; 3]>,
    last_view: Option<(Rect, Rect)>,
}

impl Lumen {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        let (tx, rx) = channel();
        let mut me = Self {
            catalog: Catalog::load(),
            files: Vec::new(),
            info: HashMap::new(),
            thumbs: HashMap::new(),
            current: None,
            source: None,
            preview: None,
            shown: None,
            rendered: None,
            hist: None,
            loading: false,
            tx,
            rx,
            view: View::Library,
            collection: Collection::All,
            min_rating: 0,
            rating_mode: 0,
            thumb_size: 200.0,
            before: false,
            clipboard: None,
            export: None,
            error: None,
            notice: None,
            dirty_catalog: false,
            icon: studio_ui::icon_texture(&cc.egui_ctx, "lumen"),
            help: studio_ui::help::Help::new("Lumen", tutos::LUMEN),
            inbox: studio_ui::handoff::Receiver::new("lumen"),
            palette: studio_ui::palette::Palette::new(&cc.egui_ctx),
            hsl_tab: 0,
            curve_channel: 0,
            query: String::new(),
            sort: 0,
            sort_desc: false,
            picked: Vec::new(),
            history: Vec::new(),
            future: Vec::new(),
            skip_history: false,
            editing: false,
            compare: 0,
            split_pos: 0.5,
            before_tex: None,
            clipping: false,
            clip_tex: None,
            zoom: 0.0,
            pan: Vec2::ZERO,
            preset_name: None,
            preset_amount: 100.0,
            last_preset: None,
            tool: Tool::None,
            brush_size: 40.0,
            brush_feather: 60.0,
            brush_flow: 70.0,
            brush_erase: false,
            spot_size: 30.0,
            spot_feather: 40.0,
            spot_opacity: 100.0,
            eye_pupil: 50.0,
            eye_darken: 50.0,
            sel_spot: None,
            sel_eye: None,
            sel_mask: None,
            sel_part: 0,
            target_mode: 0,
            crop_overlay: 1,
            crop_ratio: 0,
            crop_lock: false,
            drag: Drag::None,
            show_overlay: false,
            overlay_color: 0,
            mask_tex: None,
            export_sheet: false,
            export_only_current: false,
            copy_sheet: false,
            copy_groups: vec![true; meta::GROUPS.len()],
            keyword_input: String::new(),
            smart_edit: None,
            large: HashMap::new(),
            large_tex: HashMap::new(),
            large_pending: Vec::new(),
            exif_cache: HashMap::new(),
            info_overlay: 0,
            hide_panels: false,
            snapshot_name: None,
            rename_mask: None,
            hover_rgb: None,
            last_view: None,
        };
        let arg = std::env::args().nth(1).map(PathBuf::from);
        if let Some(file) = arg.as_ref().filter(|p| p.is_file() && develop::is_supported(p))
            && let Some(dir) = file.parent().map(|d| d.to_path_buf())
        {
            me.open_folder(dir, &cc.egui_ctx);
            if let Some(i) = me.files.iter().position(|f| f == file) {
                me.select(i, &cc.egui_ctx);
                me.view = View::Develop;
            }
        } else if let Some(dir) = arg.filter(|p| p.is_dir()) {
            me.open_folder(dir, &cc.egui_ctx);
        } else if let Some(f) = me.catalog.folder.clone()
            && f.is_dir()
        {
            me.open_folder(f, &cc.egui_ctx);
        }
        me
    }

    fn scan(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>, depth: u8) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if recursive && depth < 6 {
                    Self::scan(&p, recursive, out, depth + 1);
                }
            } else if develop::is_supported(&p) {
                out.push(p);
            }
        }
    }

    fn open_folder(&mut self, folder: PathBuf, ctx: &egui::Context) {
        let mut real_files = Vec::new();
        Self::scan(&folder, self.catalog.recursive, &mut real_files, 0);
        real_files.sort();
        self.info = real_files
            .iter()
            .map(|p| {
                let m = std::fs::metadata(p).ok();
                (p.clone(), FileInfo { size: m.as_ref().map(|m| m.len()).unwrap_or(0), modified: m.and_then(|m| m.modified().ok()) })
            })
            .collect();
        let mut files = Vec::new();
        for f in &real_files {
            files.push(f.clone());
            for n in 1..=self.catalog.virtual_copies.get(f).copied().unwrap_or(0) {
                files.push(PathBuf::from(format!("{}|v{}", f.display(), n + 1)));
            }
        }
        self.catalog.recent_folders.retain(|f| *f != folder);
        self.catalog.recent_folders.insert(0, folder.clone());
        self.catalog.recent_folders.truncate(8);
        self.catalog.folder = Some(folder);
        self.dirty_catalog = true;
        self.thumbs.clear();
        self.large.clear();
        self.large_tex.clear();
        self.current = None;
        self.source = None;
        self.preview = None;
        self.shown = None;
        self.hist = None;
        self.picked.clear();
        self.history.clear();
        self.future.clear();
        self.view = View::Library;
        for f in &files {
            self.load_thumb(f.clone(), ctx);
        }
        self.files = files;
    }

    fn load_thumb(&self, f: PathBuf, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        rayon::spawn(move || {
            if let Ok(img) = develop::load(&real(&f), THUMB_MAX) {
                let _ = tx.send(Msg::Thumb(f, img));
                ctx.request_repaint();
            }
        });
    }

    fn request_large(&mut self, f: &Path, ctx: &egui::Context) {
        if self.large.contains_key(f) || self.large_pending.iter().any(|p| p == f) {
            return;
        }
        self.large_pending.push(f.to_path_buf());
        let (tx, ctx, f) = (self.tx.clone(), ctx.clone(), f.to_path_buf());
        rayon::spawn(move || {
            if let Ok(img) = develop::load(&real(&f), LARGE_MAX) {
                let _ = tx.send(Msg::Large(f, img));
                ctx.request_repaint();
            }
        });
    }

    fn large_texture(&mut self, f: &Path, ctx: &egui::Context) -> Option<TextureHandle> {
        self.request_large(f, ctx);
        let s = self.catalog.photos.get(f).map(|e| e.settings.clone()).unwrap_or_default();
        if let Some((st, t)) = self.large_tex.get(f)
            && *st == s
        {
            return Some(t.clone());
        }
        let src = self.large.get(f)?;
        let img = develop::process(src, &s);
        let t = ctx.load_texture(format!("large-{}", f.display()), color_image(&img), TextureOptions::LINEAR);
        self.large_tex.insert(f.to_path_buf(), (s, t.clone()));
        Some(t)
    }

    fn pick_folder(&mut self, ctx: &egui::Context) {
        if let Some(f) = rfd::FileDialog::new().set_title("Choisir un dossier de photos").pick_folder() {
            self.open_folder(f, ctx);
        }
    }

    fn select(&mut self, i: usize, ctx: &egui::Context) {
        if i >= self.files.len() {
            return;
        }
        self.current = Some(i);
        self.sel_spot = None;
        self.sel_eye = None;
        self.sel_mask = None;
        self.drag = Drag::None;
        let path = self.files[i].clone();
        if self.source.as_ref().is_some_and(|(p, _)| real(p) == real(&path)) {
            self.source.as_mut().map(|s| s.0 = path);
            self.shown = None;
            return;
        }
        self.loading = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        rayon::spawn(move || {
            let r = develop::load(&real(&path), PREVIEW_MAX);
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

    fn settings(&self, i: usize) -> Settings {
        self.catalog.photos.get(&self.files[i]).map(|e| e.settings.clone()).unwrap_or_default()
    }

    fn set_settings(&mut self, i: usize, s: Settings) {
        self.entry(i).settings = s;
        self.dirty_catalog = true;
    }

    fn targets(&self) -> Vec<usize> {
        if self.picked.len() > 1 { self.picked.clone() } else { self.current.into_iter().collect() }
    }

    fn notify(&mut self, text: impl Into<String>, ctx: &egui::Context) {
        self.notice = Some((text.into(), ctx.input(|i| i.time)));
    }

    fn migrate_legacy(s: &mut Settings) {
        if s.grad_on {
            let a = s.grad_angle.to_radians();
            let (gx, gy) = (a.sin(), -a.cos());
            let gs = s.grad_feather.clamp(0.02, 1.0) * 0.5;
            let p = |t: f32| (0.5 + gx * (t + s.grad_pos - 0.5), 0.5 + gy * (t + s.grad_pos - 0.5));
            let ((x1, y1), (x2, y2)) = (p(gs), p(-gs));
            s.masks.push(develop::Mask {
                name: "Filtre gradué".into(),
                parts: vec![develop::Part { kind: develop::PartKind::Linear, x1, y1, x2, y2, ..Default::default() }],
                adj: develop::Local { exposure: s.grad_exposure, temperature: s.grad_temp, saturation: s.grad_sat, ..Default::default() },
                ..Default::default()
            });
            s.grad_on = false;
        }
        if s.radial_on {
            let size = s.radial_size.max(0.02);
            s.masks.push(develop::Mask {
                name: "Filtre radial".into(),
                parts: vec![develop::Part { kind: develop::PartKind::Radial, x1: s.radial_x, y1: s.radial_y, rx: size, ry: size, feather: s.radial_feather, invert: s.radial_invert, ..Default::default() }],
                adj: develop::Local { exposure: s.radial_exposure, temperature: s.radial_temp, saturation: s.radial_sat, ..Default::default() },
                ..Default::default()
            });
            s.radial_on = false;
        }
    }

    fn pump(&mut self, ctx: &egui::Context) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Thumb(p, img) => {
                    let s = self.catalog.photos.get(&p).map(|e| e.settings.clone()).unwrap_or_default();
                    let img = develop::process(&img, &s);
                    self.thumbs.insert(p.clone(), ctx.load_texture(p.to_string_lossy(), color_image(&img), TextureOptions::LINEAR));
                }
                Msg::Large(p, img) => {
                    self.large_pending.retain(|x| *x != p);
                    self.large.insert(p, img);
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

    fn display_settings(&self, s: &Settings) -> Settings {
        let mut d = s.clone();
        if self.tool == Tool::Crop {
            d.crop = [0.0; 4];
        }
        d
    }

    fn refresh_preview(&mut self, ctx: &egui::Context) {
        let Some((path, src)) = &self.source else { return };
        let s = self.display_settings(&self.catalog.photos.get(path).map(|e| e.settings.clone()).unwrap_or_default());
        let key = (path.clone(), s.clone(), self.before, self.clipping);
        if self.compare > 0 {
            let geo = Settings { crop: s.crop, straighten: s.straighten, rotate: s.rotate, flip: s.flip, flip_v: s.flip_v, distortion: s.distortion, persp_v: s.persp_v, persp_h: s.persp_h, persp_rotate: s.persp_rotate, persp_aspect: s.persp_aspect, persp_scale: s.persp_scale, persp_x: s.persp_x, persp_y: s.persp_y, ..Settings::default() };
            if self.before_tex.as_ref().is_none_or(|(p, g, _)| p != path || *g != geo) {
                let orig = develop::process(src, &geo);
                self.before_tex = Some((path.clone(), geo, ctx.load_texture("before", color_image(&orig), TextureOptions::LINEAR)));
            }
        }
        if self.show_overlay
            && let Some(m) = self.sel_mask
            && m < s.masks.len()
            && self.mask_tex.as_ref().is_none_or(|(p, st, k, _)| p != path || *st != s || *k != m)
        {
            if let Some((w, h, v)) = develop::mask_overlay(src, &s, m) {
                let col = [[255u8, 40, 60], [40, 220, 90], [60, 140, 255], [255, 255, 255]][self.overlay_color.min(3)];
                let mut img = image::RgbaImage::new(w, h);
                for (p, a) in img.pixels_mut().zip(v.iter()) {
                    p.0 = [col[0], col[1], col[2], (a * 150.0) as u8];
                }
                let img = match s.rotate % 4 {
                    1 => image::imageops::rotate90(&img),
                    2 => image::imageops::rotate180(&img),
                    3 => image::imageops::rotate270(&img),
                    _ => img,
                };
                let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
                self.mask_tex = Some((path.clone(), s.clone(), m, ctx.load_texture("mask", ci, TextureOptions::LINEAR)));
            }
        }
        if self.shown.as_ref() == Some(&key) {
            return;
        }
        let used = if self.before { Settings { crop: s.crop, rotate: s.rotate, flip: s.flip, straighten: s.straighten, ..Settings::default() } } else { s.clone() };
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
            && self.tool != Tool::Crop
            && let Some(thumb) = self.thumbs.get_mut(path)
        {
            let small = image::imageops::thumbnail(&img, THUMB_MAX, THUMB_MAX);
            thumb.set(color_image(&small), TextureOptions::LINEAR);
        }
        self.rendered = Some(img);
        self.shown = Some(key);
    }

    fn open_export(&mut self, only_current: bool) {
        self.export_only_current = only_current;
        self.export_sheet = true;
    }

    fn export_ids(&self) -> Vec<usize> {
        if self.export_only_current { self.current.into_iter().collect() } else if self.picked.len() > 1 { self.picked.clone() } else { self.visible() }
    }

    fn run_export(&mut self, ctx: &egui::Context) {
        let ids = self.export_ids();
        if ids.is_empty() {
            return;
        }
        let o = self.catalog.export.clone();
        let dir = if o.same_folder {
            None
        } else {
            match o.folder.clone().filter(|d| d.is_dir()) {
                Some(d) => Some(d),
                None => match rfd::FileDialog::new().set_title("Dossier d'export").pick_folder() {
                    Some(d) => {
                        self.catalog.export.folder = Some(d.clone());
                        Some(d)
                    }
                    None => return,
                },
            }
        };
        let jobs: Vec<meta::Job> = ids
            .iter()
            .map(|&i| {
                let e = self.get(i);
                let p = &self.files[i];
                let mut name = real(p).file_stem().unwrap_or_default().to_string_lossy().into_owned();
                if let Some(n) = copy_index(p) {
                    name = format!("{name}-copie{n}");
                }
                meta::Job { path: real(p), name, settings: e.settings, title: e.title, rating: e.rating }
            })
            .collect();
        let mark = (o.watermark && !o.watermark_text.trim().is_empty()).then(|| text_mark(ctx, o.watermark_text.trim())).flatten();
        let done = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(Mutex::new(Vec::new()));
        self.export = Some(Export { done: done.clone(), total: jobs.len(), errors: errors.clone() });
        self.dirty_catalog = true;
        std::thread::spawn(move || meta::run_export(jobs, o, dir, mark, done, errors));
    }

    fn matches(&self, i: usize, e: &Entry) -> bool {
        let q = self.query.to_lowercase();
        let name_ok = q.is_empty()
            || display_name(&self.files[i]).to_lowercase().contains(&q)
            || e.title.to_lowercase().contains(&q)
            || e.caption.to_lowercase().contains(&q)
            || e.keywords.iter().any(|k| k.to_lowercase().contains(&q));
        let rating_ok = match self.rating_mode {
            1 => self.min_rating == 0 || e.rating == self.min_rating,
            2 => self.min_rating == 0 || e.rating <= self.min_rating,
            _ => e.rating >= self.min_rating,
        };
        name_ok
            && rating_ok
            && match &self.collection {
                Collection::All => e.flag >= 0,
                Collection::Rated => e.rating >= 3,
                Collection::Unrated => e.rating == 0 && e.flag >= 0,
                Collection::Edited => !e.settings.is_default(),
                Collection::Picks => e.flag > 0,
                Collection::Rejected => e.flag < 0,
                Collection::Virtual => copy_index(&self.files[i]).is_some(),
                Collection::Label(l) => e.label == *l,
                Collection::Keyword(k) => e.keywords.iter().any(|x| x == k),
                Collection::Smart(k) => self.catalog.smart.get(*k).is_some_and(|sm| self.smart_match(i, e, sm)),
            }
    }

    fn smart_match(&self, i: usize, e: &Entry, sm: &Smart) -> bool {
        let raw = develop::is_raw(&real(&self.files[i]));
        let t = sm.text.to_lowercase();
        e.rating >= sm.min_rating
            && match sm.flag {
                1 => e.flag > 0,
                -1 => e.flag < 0,
                2 => e.flag >= 0,
                _ => true,
            }
            && (sm.label == 0 || e.label == sm.label)
            && (sm.keyword.trim().is_empty() || e.keywords.iter().any(|k| k.eq_ignore_ascii_case(sm.keyword.trim())))
            && (t.is_empty() || display_name(&self.files[i]).to_lowercase().contains(&t) || e.title.to_lowercase().contains(&t) || e.caption.to_lowercase().contains(&t))
            && match sm.edited {
                1 => !e.settings.is_default(),
                2 => e.settings.is_default(),
                _ => true,
            }
            && match sm.kind {
                1 => raw,
                2 => !raw,
                _ => true,
            }
    }

    fn visible(&self) -> Vec<usize> {
        let mut seen_stacks: Vec<u32> = Vec::new();
        let mut v: Vec<usize> = Vec::new();
        for i in 0..self.files.len() {
            let e = self.catalog.photos.get(&self.files[i]).cloned().unwrap_or_default();
            if !self.matches(i, &e) {
                continue;
            }
            if e.stack > 0 && self.catalog.collapsed.contains(&e.stack) {
                if seen_stacks.contains(&e.stack) {
                    continue;
                }
                seen_stacks.push(e.stack);
            }
            v.push(i);
        }
        let info = |i: usize| self.info.get(&real(&self.files[i])).copied();
        match self.sort {
            1 => v.sort_by_key(|&i| std::cmp::Reverse(self.get(i).rating)),
            2 => v.sort_by_key(|&i| info(i).and_then(|f| f.modified)),
            3 => v.sort_by_cached_key(|&i| {
                let p = real(&self.files[i]);
                self.exif_cache.get(&p).and_then(|x| x.iter().find(|(k, _)| *k == "Prise de vue").map(|(_, d)| sortable_date(d))).unwrap_or_default()
            }),
            4 => v.sort_by_key(|&i| info(i).map(|f| f.size)),
            5 => v.sort_by_key(|&i| real(&self.files[i]).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()),
            6 => v.sort_by_key(|&i| {
                let l = self.get(i).label;
                if l == 0 { 99 } else { l }
            }),
            _ => {}
        }
        if self.sort_desc {
            v.reverse();
        }
        v
    }

    fn ensure_exif(&mut self) {
        if self.sort == 3 {
            for f in self.files.clone() {
                let p = real(&f);
                if !self.exif_cache.contains_key(&p) {
                    let ex = meta::read_exif(&p);
                    self.exif_cache.insert(p, ex);
                }
            }
        }
    }

    fn exif_for(&mut self, i: usize) -> Vec<(&'static str, String)> {
        let p = real(&self.files[i]);
        self.exif_cache.entry(p.clone()).or_insert_with(|| meta::read_exif(&p)).clone()
    }

    fn undo(&mut self) {
        if let Some((idx, st, label)) = self.history.pop()
            && idx < self.files.len()
        {
            let cur = self.settings(idx);
            self.future.push((idx, cur, label));
            self.set_settings(idx, st);
            self.skip_history = true;
        }
    }

    fn redo(&mut self) {
        if let Some((idx, st, label)) = self.future.pop()
            && idx < self.files.len()
        {
            let cur = self.settings(idx);
            self.history.push((idx, cur, label));
            self.set_settings(idx, st);
            self.skip_history = true;
        }
    }

    fn virtual_copy(&mut self, ctx: &egui::Context) {
        let Some(i) = self.current else { return };
        let base = real(&self.files[i]);
        let n = self.catalog.virtual_copies.entry(base.clone()).or_insert(0);
        *n += 1;
        let path = PathBuf::from(format!("{}|v{}", base.display(), *n + 1));
        let mut e = self.get(i);
        e.snapshots.clear();
        self.catalog.photos.insert(path.clone(), e);
        let at = (i + 1..self.files.len()).find(|&k| real(&self.files[k]) != base).unwrap_or(self.files.len());
        self.files.insert(at, path.clone());
        self.load_thumb(path, ctx);
        self.picked.clear();
        self.current = Some(at);
        self.dirty_catalog = true;
        self.notify("Copie virtuelle créée", ctx);
    }

    fn remove_virtual(&mut self, i: usize) {
        let p = self.files[i].clone();
        if copy_index(&p).is_none() {
            return;
        }
        self.catalog.photos.remove(&p);
        self.files.remove(i);
        self.picked.clear();
        self.current = None;
        self.history.clear();
        self.future.clear();
        self.dirty_catalog = true;
    }

    fn stack_selection(&mut self) {
        let t = self.targets();
        if t.len() < 2 {
            return;
        }
        let id = self.catalog.next_stack.max(1);
        self.catalog.next_stack = id + 1;
        for k in t {
            self.entry(k).stack = id;
        }
        self.dirty_catalog = true;
    }

    fn unstack(&mut self) {
        for k in self.targets() {
            let s = self.get(k).stack;
            if s > 0 {
                for p in self.catalog.photos.values_mut() {
                    if p.stack == s {
                        p.stack = 0;
                    }
                }
                self.catalog.collapsed.retain(|c| *c != s);
            }
        }
        self.dirty_catalog = true;
    }

    fn toggle_stack(&mut self) {
        if let Some(i) = self.current {
            let s = self.get(i).stack;
            if s > 0 {
                if let Some(k) = self.catalog.collapsed.iter().position(|c| *c == s) {
                    self.catalog.collapsed.remove(k);
                } else {
                    self.catalog.collapsed.push(s);
                }
                self.dirty_catalog = true;
            }
        }
    }

    fn apply_preset(&mut self, preset: &Settings) {
        let targets = self.targets();
        for &i in &targets {
            let base = self.settings(i);
            let mut s = meta::blend(&Settings::default(), preset, self.preset_amount / 100.0);
            s.crop = base.crop;
            s.rotate = base.rotate;
            s.straighten = base.straighten;
            s.flip = base.flip;
            s.spots = base.spots.clone();
            s.red_eyes = base.red_eyes.clone();
            s.masks = base.masks.clone();
            self.set_settings(i, s);
            if Some(i) == self.current {
                self.last_preset = Some((base, preset.clone()));
            }
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            self.before = false;
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let (copy, paste, auto, undo, redo, sel_all, desel, stack, unstack, vcopy, export, copy_sheet, paste_all, snap, reset) = ctx.input_mut(|i| {
            (
                i.consume_shortcut(&K::new(M::COMMAND | M::SHIFT, Key::C)) || i.consume_shortcut(&K::new(M::COMMAND, Key::C)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::V)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::U)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::Z)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::Y)) || i.consume_shortcut(&K::new(M::COMMAND | M::SHIFT, Key::Z)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::A)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::D)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::G)),
                i.consume_shortcut(&K::new(M::COMMAND | M::SHIFT, Key::G)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::Quote)) || i.consume_shortcut(&K::new(M::COMMAND, Key::Backtick)),
                i.consume_shortcut(&K::new(M::COMMAND | M::SHIFT, Key::E)),
                i.modifiers.shift,
                i.consume_shortcut(&K::new(M::COMMAND | M::SHIFT, Key::V)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::N)),
                i.consume_shortcut(&K::new(M::COMMAND | M::SHIFT, Key::R)),
            )
        });
        let plain = ctx.input(|i| !i.modifiers.command && !i.modifiers.alt);
        let shift = ctx.input(|i| i.modifiers.shift);
        let pressed = |k: Key| plain && ctx.input(|i| i.key_pressed(k));
        let develop = self.view == View::Develop;
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if sel_all {
            self.picked = self.visible();
        }
        if desel {
            self.picked.clear();
        }
        if stack && !unstack {
            self.stack_selection();
        }
        if unstack {
            self.unstack();
        }
        if vcopy {
            self.virtual_copy(ctx);
        }
        if export && !self.files.is_empty() {
            self.open_export(develop && self.picked.len() < 2);
        }
        let targets = self.targets();
        let mut rating = None;
        for (n, k) in [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5].into_iter().enumerate() {
            if pressed(k) {
                rating = Some(n as u8);
            }
        }
        let mut lab = None;
        for (n, k) in [Key::Num6, Key::Num7, Key::Num8, Key::Num9].into_iter().enumerate() {
            if pressed(k) {
                lab = Some(n as u8 + 1);
            }
        }
        let (pk, xk, uk) = (pressed(Key::P), pressed(Key::X), pressed(Key::U));
        let brackets = ctx.input(|i| (i.key_pressed(Key::OpenBracket), i.key_pressed(Key::CloseBracket)));
        if !targets.is_empty() && (pk || xk || uk || lab.is_some() || rating.is_some()) {
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
                if let Some(r) = rating {
                    e.rating = r;
                }
            }
            self.dirty_catalog = true;
        }
        if develop {
            if pressed(Key::Z) {
                self.zoom = if self.zoom == 0.0 { 1.0 } else { 0.0 };
                self.pan = Vec2::ZERO;
            }
            if pressed(Key::J) {
                self.clipping = !self.clipping;
                self.shown = None;
            }
            if pressed(Key::Y) {
                self.compare = (self.compare + 1) % 4;
            }
            if pressed(Key::I) {
                self.info_overlay = (self.info_overlay + 1) % 3;
            }
            if pressed(Key::O) {
                self.show_overlay = !self.show_overlay;
            }
            if pressed(Key::Tab) {
                self.hide_panels = !self.hide_panels;
            }
            if pressed(Key::V)
                && let Some(i) = self.current
            {
                let mut s = self.settings(i);
                s.mono = !s.mono;
                self.set_settings(i, s);
            }
            for (key, group) in [(Key::R, 0usize), (Key::Q, 1), (Key::E, 2), (Key::M, 3), (Key::K, 4), (Key::W, 5), (Key::T, 6), (Key::H, 7)] {
                if pressed(key) {
                    self.shortcut_tool(group, shift, ctx);
                }
            }
            if brackets.0 {
                self.brush_size = (self.brush_size * 0.85).max(3.0);
                self.spot_size = (self.spot_size * 0.85).max(3.0);
            }
            if brackets.1 {
                self.brush_size = (self.brush_size * 1.18).min(300.0);
                self.spot_size = (self.spot_size * 1.18).min(300.0);
            }
            if plain && ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
                self.delete_selected();
            }
            if pressed(Key::Enter) && self.tool != Tool::None {
                self.tool = Tool::None;
            }
        } else {
            if pressed(Key::D) || pressed(Key::Enter) {
                if self.current.is_some() {
                    self.view = View::Develop;
                }
            }
            if pressed(Key::C) {
                self.view = if self.view == View::Compare { View::Library } else { View::Compare };
            }
            if pressed(Key::N) {
                self.view = if self.view == View::Survey { View::Library } else { View::Survey };
            }
            if pressed(Key::S) {
                self.toggle_stack();
            }
        }
        if develop && pressed(Key::D) {
            self.tool = Tool::None;
        }
        if pressed(Key::G) || (pressed(Key::Escape) && self.tool == Tool::None && self.drag == Drag::None) {
            self.view = View::Library;
        }
        if pressed(Key::Escape) {
            self.tool = Tool::None;
            self.drag = Drag::None;
        }
        self.before = plain && ctx.input(|i| i.key_down(Key::Backslash));
        let vis = self.visible();
        let pos = self.current.and_then(|c| vis.iter().position(|&v| v == c));
        let (next, prev) = (pressed(Key::ArrowRight), pressed(Key::ArrowLeft));
        if next || prev {
            let n = match pos {
                Some(p) if next => (p + 1).min(vis.len().saturating_sub(1)),
                Some(p) => p.saturating_sub(1),
                None => 0,
            };
            if let Some(&i) = vis.get(n) {
                self.picked.clear();
                self.select(i, ctx);
            }
        }
        if let Some(i) = self.current {
            if copy {
                if copy_sheet {
                    self.copy_sheet = true;
                }
                self.clipboard = Some(self.settings(i));
                self.notify("Réglages copiés", ctx);
            }
            if paste && let Some(c) = self.clipboard.clone() {
                for t in self.targets() {
                    let s = meta::copy_groups(&self.settings(t), &c, &self.copy_groups);
                    self.set_settings(t, s);
                }
            }
            if paste_all && let Some(c) = self.clipboard.clone() {
                for v in self.visible() {
                    let s = meta::copy_groups(&self.settings(v), &c, &self.copy_groups);
                    self.set_settings(v, s);
                }
            }
            if auto && let Some((_, src)) = &self.source {
                let mut s = self.settings(i);
                let a = develop::auto(src);
                (s.exposure, s.contrast, s.shadows, s.highlights, s.vibrance) = (a.exposure, a.contrast, a.shadows, a.highlights, a.vibrance);
                self.set_settings(i, s);
            }
            if snap {
                self.add_snapshot(i, None);
                self.notify("Instantané enregistré", ctx);
            }
            if reset {
                for t in self.targets() {
                    self.set_settings(t, Settings::default());
                }
            }
        }
    }

    fn add_snapshot(&mut self, i: usize, name: Option<String>) {
        let s = self.settings(i);
        let e = self.entry(i);
        let name = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| format!("Instantané {}", e.snapshots.len() + 1));
        e.snapshots.push((name, s));
        self.dirty_catalog = true;
    }

    fn delete_selected(&mut self) {
        let Some(i) = self.current else { return };
        let mut s = self.settings(i);
        if let Some(k) = self.sel_spot.take().filter(|k| *k < s.spots.len()) {
            s.spots.remove(k);
        } else if let Some(k) = self.sel_eye.take().filter(|k| *k < s.red_eyes.len()) {
            s.red_eyes.remove(k);
        } else if let Some(m) = self.sel_mask.filter(|m| *m < s.masks.len()) {
            s.masks.remove(m);
            self.sel_mask = None;
        } else {
            return;
        }
        self.set_settings(i, s);
    }

    fn track_history(&mut self, snap: Option<(usize, Settings)>, ctx: &egui::Context) {
        if let Some((i, before)) = snap
            && i < self.files.len()
            && self.current == Some(i)
        {
            let now = self.settings(i);
            if now != before && !self.skip_history && !self.editing {
                let label = meta::changed_label(&before, &now);
                self.history.push((i, before, label));
                if self.history.len() > 300 {
                    self.history.remove(0);
                }
                self.future.clear();
                self.editing = true;
            }
        }
        self.skip_history = false;
        if ctx.input(|i| !i.pointer.any_down()) {
            self.editing = false;
        }
    }
}

fn sortable_date(d: &str) -> String {
    let p: Vec<&str> = d.split(['/', ' ', ':']).collect();
    if p.len() >= 5 { format!("{}{}{}{}{}", p[2], p[1], p[0], p[3], p[4]) } else { d.to_string() }
}

fn text_mark(ctx: &egui::Context, text: &str) -> Option<meta::Mark> {
    let galley = ctx.fonts_mut(|f| f.layout_no_wrap(text.to_string(), egui::FontId::proportional(72.0), Color32::WHITE));
    let atlas = ctx.fonts(|f| f.image());
    let ppp = ctx.pixels_per_point();
    let (w, h) = ((galley.rect.width() * ppp).ceil() as u32 + 4, (galley.rect.height() * ppp).ceil() as u32 + 4);
    if w < 4 || h < 4 {
        return None;
    }
    let mut alpha = vec![0u8; (w * h) as usize];
    for row in &galley.rows {
        for g in &row.row.glyphs {
            let uv = g.uv_rect;
            let (gw, gh) = ((uv.max[0] - uv.min[0]) as i32, (uv.max[1] - uv.min[1]) as i32);
            let x0 = ((row.pos.x + g.pos.x + uv.offset.x) * ppp).round() as i32 + 2;
            let y0 = ((row.pos.y + g.pos.y + uv.offset.y) * ppp).round() as i32 + 2;
            for y in 0..gh {
                for x in 0..gw {
                    let (sx, sy) = (uv.min[0] as usize + x as usize, uv.min[1] as usize + y as usize);
                    let Some(px) = atlas.pixels.get(sy * atlas.size[0] + sx) else { continue };
                    let (dx, dy) = (x0 + x, y0 + y);
                    if dx >= 0 && dy >= 0 && (dx as u32) < w && (dy as u32) < h {
                        let d = &mut alpha[(dy as u32 * w + dx as u32) as usize];
                        *d = (*d).max(px.a());
                    }
                }
            }
        }
    }
    alpha.iter().any(|a| *a > 0).then_some(meta::Mark { w, h, alpha })
}

impl Lumen {

    fn commands(&self) -> Vec<studio_ui::palette::Cmd> {
        let mut v = vec![
            studio_ui::palette::Cmd::new("Trouver les photos floues (les sélectionner)", "Tri", "blurry:pick"),
            studio_ui::palette::Cmd::new("Rejeter les photos floues", "Tri · drapeau X", "blurry:reject").confirm(),
            studio_ui::palette::Cmd::new("Ton auto sur les photos choisies", "Lot", "batch-auto"),
            studio_ui::palette::Cmd::new("Redresser l'horizon sur les photos choisies", "Lot", "batch-straighten"),
            studio_ui::palette::Cmd::new("Remettre à zéro les photos choisies", "Lot", "batch-reset").confirm(),
            studio_ui::palette::Cmd::new("Synchroniser les réglages vers les photos choisies", "Lot · copie la photo courante", "batch-sync"),
            studio_ui::palette::Cmd::new("Attribuer 5 étoiles aux photos choisies", "Lot · classement", "batch-rate:5"),
            studio_ui::palette::Cmd::new("Marquer les photos choisies comme retenues", "Lot · drapeau P", "batch-flag:1"),
            studio_ui::palette::Cmd::new("Rejeter les photos choisies", "Lot · drapeau X", "batch-flag:-1").confirm(),
            studio_ui::palette::Cmd::new("Effacer les drapeaux des photos choisies", "Lot · neutre", "batch-flag:0"),

        ];
        v.extend(outils::TOOLS.iter().map(|(id, label, _)| studio_ui::palette::Cmd::new(format!("Outil : {label}"), "Lumen · nouvel outil · photo courante ou choisies", format!("tool:{id}"))));
        let photos = self.files.len();
        let current = self.current.is_some_and(|i| i < photos);
        let targets = self.targets().into_iter().filter(|i| *i < photos).count();
        let sync_targets = self.current.map_or(0, |source| self.picked.iter().filter(|i| **i < photos && **i != source).count());
        v = v
            .into_iter()
            .map(|command| {
                let (ok, reason) = match command.id.as_str() {
                    _ if photos == 0 => (false, "Ouvre d'abord un dossier de photos"),
                    id if id.starts_with("blurry:") => (photos > 1, "Il faut au moins deux photos à comparer"),
                    "batch-sync" => (current && sync_targets > 0, "Ouvre la photo modèle puis choisis d'autres photos (Ctrl+clic)"),
                    _ => (targets > 0, "Choisis d'abord des photos (clic ou Ctrl+clic)"),
                };
                command.enabled_if(ok, reason)
            })
            .collect();
        v.extend(studio_ui::palette::send_cmds(&[("pixel", "retoucher"), ("page", "mettre en page"), ("vector", "placer l'image"), ("cut", "ajouter au montage"), ("motion", "animer la photo")]).into_iter().map(|command| command.enabled_if(current, "Choisis d'abord une photo")));
        v.extend(studio_ui::palette::effect_cmds().into_iter().map(|command| command.enabled_if(current, "Choisis d'abord une photo")));
        v
    }

    fn run_cmd(&mut self, id: &str, ctx: &egui::Context) {
        if self.run_lib(id, ctx) {
            return;
        }
        if let Some(rating) = id.strip_prefix("batch-rate:").and_then(|value| value.parse::<u8>().ok()).filter(|value| *value <= 5) {
            let file_count = self.files.len();
            let targets: Vec<usize> = self.targets().into_iter().filter(|index| *index < file_count).collect();
            for index in &targets {
                self.entry(*index).rating = rating;
            }
            self.dirty_catalog |= !targets.is_empty();
            studio_ui::handoff::toast(ctx, format!("{rating} étoile(s) attribuée(s) à {} photo(s)", targets.len()), !targets.is_empty());
            return;
        }
        if let Some(flag) = id.strip_prefix("batch-flag:").and_then(|value| value.parse::<i8>().ok()).filter(|value| (-1..=1).contains(value)) {
            let file_count = self.files.len();
            let targets: Vec<usize> = self.targets().into_iter().filter(|index| *index < file_count).collect();
            for index in &targets {
                self.entry(*index).flag = flag;
            }
            self.dirty_catalog |= !targets.is_empty();
            let action = match flag { -1 => "rejetée(s)", 1 => "retenue(s)", _ => "remise(s) à zéro" };
            studio_ui::handoff::toast(ctx, format!("{} photo(s) {action}", targets.len()), !targets.is_empty());
            return;
        }
        if id == "batch-sync" {
            let Some(source) = self.current.filter(|i| *i < self.files.len()) else {
                studio_ui::handoff::toast(ctx, "Choisis d'abord la photo source".into(), false);
                return;
            };
            let settings = self.settings(source);
            let targets: Vec<usize> = self.picked.iter().copied().filter(|i| *i < self.files.len() && *i != source).collect();
            for target in &targets {
                self.set_settings(*target, settings.clone());
            }
            studio_ui::handoff::toast(ctx, format!("Réglages synchronisés vers {} photo(s)", targets.len()), !targets.is_empty());
            return;
        }
        if let Some(mode) = id.strip_prefix("blurry:") {
            let pool: Vec<usize> = if self.picked.len() > 1 { self.picked.clone() } else { (0..self.files.len()).collect() };
            let scores: Vec<(usize, f32)> = pool.iter().filter_map(|i| develop::load(&self.files[*i], 800).ok().map(|img| (*i, develop::sharpness(&img)))).collect();
            if scores.len() < 2 {
                studio_ui::handoff::toast(ctx, "Il faut au moins deux photos à comparer".into(), false);
                return;
            }
            let mut sorted: Vec<f32> = scores.iter().map(|s| s.1).collect();
            sorted.sort_by(|a, b| a.total_cmp(b));
            let median = sorted[sorted.len() / 2];
            let blurry: Vec<usize> = scores.iter().filter(|s| s.1 < median * 0.3).map(|s| s.0).collect();
            if mode == "reject" {
                for i in &blurry {
                    self.entry(*i).flag = -1;
                }
                self.dirty_catalog = true;
            } else if !blurry.is_empty() {
                self.picked = blurry.clone();
            }
            studio_ui::handoff::toast(ctx, if blurry.is_empty() { "Aucune photo nettement floue".into() } else if mode == "reject" { format!("{} photo(s) floue(s) rejetée(s) (X)", blurry.len()) } else { format!("{} photo(s) floue(s) sélectionnée(s)", blurry.len()) }, !blurry.is_empty());
            return;
        }
        if matches!(id, "batch-auto" | "batch-straighten" | "batch-reset") {
            let targets = self.targets();
            if targets.is_empty() {
                studio_ui::handoff::toast(ctx, "Choisis d'abord des photos (Ctrl+clic dans la bibliothèque)".into(), false);
                return;
            }
            let mut done = 0;
            let n_files = self.files.len();
            for i in targets.iter().copied().filter(|i| *i < n_files) {
                let mut s = self.settings(i);
                if id == "batch-reset" {
                    s = develop::Settings::default();
                } else {
                    let Ok(img) = develop::load(&self.files[i], 640) else { continue };
                    if id == "batch-auto" {
                        let a = develop::auto(&img);
                        (s.exposure, s.contrast, s.shadows, s.highlights, s.vibrance) = (a.exposure, a.contrast, a.shadows, a.highlights, a.vibrance);
                    } else {
                        s.straighten = develop::auto_straighten(&img);
                    }
                }
                self.set_settings(i, s);
                done += 1;
            }
            let what = match id {
                "batch-auto" => "Ton auto appliqué",
                "batch-straighten" => "Horizon redressé",
                _ => "Réglages remis à zéro",
            };
            studio_ui::handoff::toast(ctx, format!("{what} sur {done} photo(s)"), done > 0);
            return;
        }
        if let Some(t) = id.strip_prefix("send:") {
            let t: &'static str = studio_core::handoff::APPS.iter().find(|a| a.0 == t).map_or("pixel", |a| a.0);
            self.send_to(t, ctx);
            return;
        }
        if let Some(effect) = studio_ui::palette::effect_from_command(id) {
            if let Some(i) = self.current.filter(|i| *i < self.files.len()) {
                let mut s = self.settings(i);
                s.creative.push(effect);
                self.set_settings(i, s);
                self.view = View::Develop;
            } else {
                studio_ui::handoff::toast(ctx, "Choisis d'abord une photo".into(), false);
            }
            return;
        }
        match id {

            _ => {}
        }
    }
    fn send_to(&mut self, target: &str, ctx: &egui::Context) {
        let Some(i) = self.current.filter(|i| *i < self.files.len()) else {
            studio_ui::handoff::toast(ctx, "Choisis d'abord une photo".into(), false);
            return;
        };
        let (path, s) = (self.files[i].clone(), self.settings(i));
        let stem = path.file_stem().map_or("photo".into(), |s| s.to_string_lossy().into_owned());
        let (target, ctx) = (target.to_string(), ctx.clone());
        studio_ui::handoff::toast(&ctx, "Développement en pleine résolution…".into(), true);
        std::thread::spawn(move || {
            let out = studio_core::handoff::outgoing("lumen", &stem, "png");
            match develop::load(&path, 12_000).map(|img| develop::process(&img, &s)).and_then(|img| img.save(&out).map_err(|e| e.to_string())) {
                Ok(()) => studio_ui::handoff::send(&ctx, &target, &out),
                Err(e) => studio_ui::handoff::toast(&ctx, format!("Envoi impossible : {e}"), false),
            }
        });
    }

    fn receive(&mut self, p: &Path, ctx: &egui::Context) {
        let Some(dir) = p.parent().map(|d| d.to_path_buf()) else { return };
        self.open_folder(dir, ctx);
        if let Some(i) = self.files.iter().position(|f| f == p) {
            self.select(i, ctx);
            self.view = View::Develop;
        }
    }
}

impl eframe::App for Lumen {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        studio_ui::chrome::resize(ctx);
        for p in self.inbox.poll(ctx) {
            self.receive(&p, ctx);
        }
        self.palette.set_library(optlib::lib);
        self.palette.keys(ctx);
        if self.palette.open {
            let cmds = self.commands();
            if let Some(id) = self.palette.show(ctx, &mut self.help, cmds) {
                self.run_cmd(&id, ctx);
            }
        }
        self.pump(ctx);
        self.ensure_exif();
        let snap = self.current.filter(|i| *i < self.files.len()).map(|i| (i, self.settings(i)));
        self.keys(ctx);
        self.help.show(ctx);
        if self.current.is_none() && matches!(self.view, View::Develop) {
            self.view = View::Library;
        }
        if self.view == View::Develop
            && let Some(i) = self.current
        {
            let s = self.settings(i);
            if s.grad_on || s.radial_on {
                let mut m = s.clone();
                Self::migrate_legacy(&mut m);
                self.set_settings(i, m);
            }
        }

        self.menu_bar(root, ctx);
        self.status_bar(root, ctx);
        if self.files.is_empty() {
            self.welcome(root, ctx);
        } else {
            match self.view {
                View::Library => self.library(root, ctx),
                View::Develop => self.develop(root, ctx),
                View::Compare => self.compare_view(root, ctx),
                View::Survey => self.survey_view(root, ctx),
            }
        }
        self.sheets(ctx);
        self.track_history(snap, ctx);
        if self.dirty_catalog && ctx.input(|i| !i.pointer.any_down()) {
            self.catalog.save();
            self.dirty_catalog = false;
        }
    }
}

impl Lumen {
    fn menu_bar(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::Panel::top("menu").exact_size(44.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let full = studio_ui::chrome::bar(ui, 12.0);
            ui.scope_builder(egui::UiBuilder::new().max_rect(full), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(22.0))));
                    ui.add_space(4.0);
                    egui::MenuBar::new().ui(ui, |ui| {
                        self.file_menu(ui, ctx);
                        self.edit_menu(ui, ctx);
                        self.photo_menu(ui, ctx);
                        self.view_menu(ui);
                        self.help.menu(ui);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !self.files.is_empty() {
                            let label = if self.picked.len() > 1 { format!("Exporter {}", self.picked.len()) } else if self.view == View::Develop { "Exporter".to_string() } else { format!("Exporter {}", self.visible().len()) };
                            let dev = self.view == View::Develop && self.picked.len() < 2;
                            if pro::primary_icon(ui, &label, export_glyph).on_hover_text("Ctrl+Maj+E").clicked() {
                                self.open_export(dev);
                            }
                            ui.add_space(8.0);
                            view_switch(ui, &mut self.view, self.current.is_some());
                        }
                    });
                });
            });
        });
    }

    fn file_menu(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.menu_button("Fichier", |ui| {
            if let Some(t) = studio_ui::handoff::menu(ui, &[("pixel", "retoucher"), ("page", "mettre en page"), ("vector", "placer l'image"), ("cut", "ajouter au montage"), ("motion", "animer la photo")]) {
                let ctx = ui.ctx().clone();
                self.send_to(t, &ctx);
            }
            ui.separator();
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
            if ui.checkbox(&mut self.catalog.recursive, "Inclure les sous-dossiers").changed()
                && let Some(f) = self.catalog.folder.clone()
            {
                self.open_folder(f, ctx);
            }
            if ui.button("Actualiser le dossier").clicked()
                && let Some(f) = self.catalog.folder.clone()
            {
                self.open_folder(f, ctx);
            }
            ui.separator();
            if ui.add_enabled(self.current.is_some(), egui::Button::new("Exporter la photo…")).clicked() {
                self.open_export(true);
            }
            if ui.add_enabled(!self.files.is_empty(), egui::Button::new("Exporter la sélection…        Ctrl+Maj+E")).clicked() {
                self.open_export(false);
            }
            if ui.add_enabled(self.current.is_some(), egui::Button::new("Exporter comme précédemment")).clicked() {
                self.export_only_current = self.picked.len() < 2;
                self.run_export(ctx);
            }
            ui.separator();
            if ui.button("Enregistrer le catalogue").clicked() {
                self.catalog.save();
                self.notify("Catalogue enregistré", ctx);
            }
            if ui.button("Ouvrir le dossier du catalogue").clicked()
                && let Some(d) = Catalog::path().parent()
            {
                let _ = std::process::Command::new("explorer").arg(d).spawn();
            }
        });
    }

    fn edit_menu(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.menu_button("Édition", |ui| {
            if ui.add_enabled(!self.history.is_empty(), egui::Button::new("Annuler                Ctrl+Z")).clicked() {
                self.undo();
            }
            if ui.add_enabled(!self.future.is_empty(), egui::Button::new("Rétablir               Ctrl+Y")).clicked() {
                self.redo();
            }
            ui.separator();
            if ui.button("Tout sélectionner      Ctrl+A").clicked() {
                self.picked = self.visible();
            }
            if ui.button("Désélectionner         Ctrl+D").clicked() {
                self.picked.clear();
            }
            if ui.button("Sélectionner les retenues").clicked() {
                self.picked = self.visible().into_iter().filter(|&i| self.get(i).flag > 0).collect();
            }
            if ui.button("Sélectionner les modifiées").clicked() {
                self.picked = self.visible().into_iter().filter(|&i| !self.get(i).settings.is_default()).collect();
            }
            if ui.button("Inverser la sélection").clicked() {
                let p = self.picked.clone();
                self.picked = self.visible().into_iter().filter(|i| !p.contains(i)).collect();
            }
            ui.separator();
            ui.checkbox(&mut self.catalog.solo, "Mode solo des panneaux");
            if ui.button("Copier les réglages…   Ctrl+Maj+C").clicked()
                && let Some(i) = self.current
            {
                self.clipboard = Some(self.settings(i));
                self.copy_sheet = true;
            }
            let _ = ctx;
        });
    }

    fn photo_menu(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.add_enabled_ui(self.current.is_some(), |ui| {
            ui.menu_button("Photo", |ui| {
                let i = self.current.unwrap_or(0);
                if ui.button("Réglage automatique      Ctrl+U").clicked()
                    && let Some((_, src)) = &self.source
                {
                    let s = develop::auto(src);
                    self.set_settings(i, s);
                }
                if ui.button("Copier les réglages      Ctrl+C").clicked() {
                    self.clipboard = Some(self.settings(i));
                }
                if ui.add_enabled(self.clipboard.is_some(), egui::Button::new("Coller les réglages       Ctrl+V")).clicked()
                    && let Some(c) = self.clipboard.clone()
                {
                    for t in self.targets() {
                        let s = meta::copy_groups(&self.settings(t), &c, &self.copy_groups);
                        self.set_settings(t, s);
                    }
                }
                if ui.add_enabled(self.clipboard.is_some(), egui::Button::new("Coller sur les photos affichées   Ctrl+Maj+V")).clicked()
                    && let Some(c) = self.clipboard.clone()
                {
                    for v in self.visible() {
                        let s = meta::copy_groups(&self.settings(v), &c, &self.copy_groups);
                        self.set_settings(v, s);
                    }
                }
                if ui.button("Réinitialiser               Ctrl+Maj+R").clicked() {
                    for t in self.targets() {
                        self.set_settings(t, Settings::default());
                    }
                }
                ui.separator();
                if ui.button("Créer une copie virtuelle   Ctrl+'").clicked() {
                    self.virtual_copy(ctx);
                }
                if ui.add_enabled(self.files.get(i).is_some_and(|p| copy_index(p).is_some()), egui::Button::new("Supprimer la copie virtuelle")).clicked() {
                    self.remove_virtual(i);
                }
                if ui.button("Nouvel instantané             Ctrl+N").clicked() {
                    self.add_snapshot(i, None);
                }
                ui.separator();
                if ui.button("Empiler la sélection          Ctrl+G").clicked() {
                    self.stack_selection();
                }
                if ui.button("Désempiler                      Ctrl+Maj+G").clicked() {
                    self.unstack();
                }
                if ui.button("Réduire / déplier la pile     S").clicked() {
                    self.toggle_stack();
                }
                ui.separator();
                let mut cur = self.settings(i);
                let before = cur.clone();
                ui.checkbox(&mut cur.mono, "Noir et blanc                   V");
                ui.checkbox(&mut cur.flip, "Miroir horizontal");
                ui.checkbox(&mut cur.flip_v, "Miroir vertical");
                if ui.button("Rotation 90° horaire").clicked() {
                    cur.rotate = (cur.rotate + 1) % 4;
                }
                if ui.button("Rotation 90° antihoraire").clicked() {
                    cur.rotate = (cur.rotate + 3) % 4;
                }
                if cur != before {
                    for t in self.targets() {
                        let mut s = self.settings(t);
                        (s.mono, s.flip, s.flip_v, s.rotate) = (cur.mono, cur.flip, cur.flip_v, cur.rotate);
                        self.set_settings(t, s);
                    }
                }
                ui.separator();
                ui.menu_button("Note", |ui| {
                    for r in 0..=5u8 {
                        let label = if r == 0 { "Sans note   0".to_string() } else { format!("{}   {r}", "★".repeat(r as usize)) };
                        if ui.button(label).clicked() {
                            for t in self.targets() {
                                self.entry(t).rating = r;
                            }
                            self.dirty_catalog = true;
                        }
                    }
                });
                ui.menu_button("Drapeau", |ui| {
                    for (label, f) in [("Retenue   P", 1i8), ("Sans drapeau   U", 0), ("Rejetée   X", -1)] {
                        if ui.button(label).clicked() {
                            for t in self.targets() {
                                self.entry(t).flag = f;
                            }
                            self.dirty_catalog = true;
                        }
                    }
                });
                ui.menu_button("Étiquette de couleur", |ui| {
                    for (k, (name, _)) in std::iter::once(("Aucune", [0u8; 3])).chain(LABELS.iter().copied()).enumerate() {
                        let key = if (1..=4).contains(&k) { format!("   {}", k + 5) } else { String::new() };
                        if ui.button(format!("{name}{key}")).clicked() {
                            for t in self.targets() {
                                self.entry(t).label = k as u8;
                            }
                            self.dirty_catalog = true;
                        }
                    }
                });
                ui.separator();
                if ui.button("Afficher dans l'Explorateur").clicked() {
                    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", real(&self.files[i]).display())).spawn();
                }
                if ui.button("Copier le chemin du fichier").clicked() {
                    ui.ctx().copy_text(real(&self.files[i]).display().to_string());
                }
            });
        });
    }

    fn view_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Affichage", |ui| {
            for (v, label) in [(View::Library, "Grille                         G"), (View::Develop, "Développement              D"), (View::Compare, "Comparaison                 C"), (View::Survey, "Ensemble                     N")] {
                let enabled = v == View::Library || self.current.is_some() || v == View::Survey;
                if ui.add_enabled(enabled, egui::Button::selectable(self.view == v, label)).clicked() {
                    self.view = v;
                    ui.close();
                }
            }
            ui.separator();
            ui.checkbox(&mut self.hide_panels, "Masquer les panneaux       Tab");
            ui.checkbox(&mut self.clipping, "Écrêtage                        J");
            if ui.selectable_label(self.zoom > 0.0, "Zoom 100 %                   Z").clicked() {
                self.zoom = if self.zoom > 0.0 { 0.0 } else { 1.0 };
            }
            ui.menu_button("Avant / après", |ui| {
                for (k, l) in ["Désactivé", "Rideau vertical", "Côte à côte", "Rideau horizontal"].iter().enumerate() {
                    ui.radio_value(&mut self.compare, k, *l);
                }
            });
            ui.menu_button("Infos sur la photo", |ui| {
                for (k, l) in ["Aucune", "Nom et réglages", "Données de prise de vue"].iter().enumerate() {
                    ui.radio_value(&mut self.info_overlay, k, *l);
                }
            });
            ui.menu_button("Taille des vignettes", |ui| {
                for (l, v) in [("Petites", 140.0), ("Moyennes", 200.0), ("Grandes", 280.0), ("Très grandes", 340.0)] {
                    if ui.button(l).clicked() {
                        self.thumb_size = v;
                    }
                }
            });
        });
    }

    fn status_bar(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                match &self.catalog.folder {
                    Some(f) => ui.label(small(format!("{} · {} photo(s) · {} affichée(s){}", f.display(), self.files.len(), self.visible().len(), if self.picked.len() > 1 { format!(" · {} sélectionnée(s)", self.picked.len()) } else { String::new() }))),
                    None => ui.label(small("Aucun dossier ouvert".into())),
                };
                if let Some((t, at)) = self.notice.clone() {
                    if ctx.input(|i| i.time) - at < 2.5 {
                        ui.label(RichText::new(t).size(12.0).color(GREEN));
                        ctx.request_repaint_after(std::time::Duration::from_millis(300));
                    } else {
                        self.notice = None;
                    }
                }
                if let Some(e) = &self.export {
                    let d = e.done.load(Ordering::Relaxed);
                    let errs = e.errors.lock().map(|l| l.len()).unwrap_or(0);
                    if d < e.total {
                        ui.add(egui::ProgressBar::new(d as f32 / e.total as f32).desired_width(160.0).text(format!("Export {d}/{}", e.total)));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
                    } else if errs > 0 {
                        let first = e.errors.lock().ok().and_then(|l| l.first().cloned()).unwrap_or_default();
                        ui.label(RichText::new(format!("Export terminé, {errs} erreur(s)")).size(12.0).color(RED)).on_hover_text(first);
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
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let hint = match self.view {
                        View::Develop => "R recadrer · Q retouche · E yeux rouges · M masques · K pinceau · W pipette · T ciblage · Y avant/après · Tab panneaux",
                        _ => "0-5 noter · P/X drapeau · 6-9 couleur · D développer · C comparer · N ensemble · Ctrl+G empiler",
                    };
                    ui.label(small(hint.into()));
                });
            });
        });
    }

    fn welcome(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let area = ui.max_rect();
            let card = Rect::from_center_size(area.center(), vec2(520.0, 380.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(card), |ui| {
                Frame::new().fill(CHROME).corner_radius(CornerRadius::same(18)).stroke(Stroke::new(1.0, LINE)).inner_margin(Margin::same(32)).show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(egui::Image::new((self.icon.id(), Vec2::splat(64.0))));
                        ui.add_space(14.0);
                        ui.label(RichText::new("Bienvenue dans Lumen").font(semibold(24.0)).color(INK_STRONG));
                        ui.add_space(4.0);
                        ui.label(RichText::new("Ouvre un dossier de photos : JPEG, PNG, TIFF, WebP et RAW (CR2, NEF, ARW, DNG, RAF…)").color(MUTED));
                        ui.add_space(18.0);
                        if pro::primary(ui, "Ouvrir un dossier").clicked() {
                            self.pick_folder(ctx);
                        }
                        ui.add_space(6.0);
                        ui.checkbox(&mut self.catalog.recursive, "Inclure les sous-dossiers");
                        let recents: Vec<PathBuf> = self.catalog.recent_folders.iter().filter(|f| f.is_dir()).cloned().collect();
                        if !recents.is_empty() {
                            ui.add_space(16.0);
                            ui.label(RichText::new("Dossiers récents").font(semibold(13.0)).color(INK));
                            for f in recents.into_iter().take(4) {
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

    fn sheets(&mut self, ctx: &egui::Context) {
        if self.export_sheet {
            let mut go = false;
            let n = self.export_ids().len();
            let keep = pro::sheet(ctx, "export", 560.0, |ui| {
                go = self.export_form(ui, n);
            });
            self.export_sheet = keep && !go;
            if go {
                self.run_export(ctx);
            }
        }
        if self.copy_sheet {
            let mut close = false;
            let keep = pro::sheet(ctx, "copy", 420.0, |ui| {
                ui.label(RichText::new("Réglages à copier").font(semibold(18.0)).color(INK_STRONG));
                widgets::caption(ui, "Coller (Ctrl+V) n'appliquera que les groupes cochés.");
                ui.add_space(8.0);
                egui::Grid::new("copy-groups").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
                    for (k, g) in meta::GROUPS.iter().enumerate() {
                        ui.checkbox(&mut self.copy_groups[k], *g);
                        if k % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if pro::ghost(ui, "Tout cocher").clicked() {
                        self.copy_groups.iter_mut().for_each(|g| *g = true);
                    }
                    if pro::ghost(ui, "Tout décocher").clicked() {
                        self.copy_groups.iter_mut().for_each(|g| *g = false);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::primary(ui, "Copier").clicked() {
                            close = true;
                        }
                    });
                });
            });
            self.copy_sheet = keep && !close;
        }
        if self.smart_edit.is_some() {
            let mut save = false;
            let mut keep = true;
            if let Some((_, sm)) = &mut self.smart_edit {
                keep = pro::sheet(ctx, "smart", 420.0, |ui| {
                    ui.label(RichText::new("Collection dynamique").font(semibold(18.0)).color(INK_STRONG));
                    widgets::caption(ui, "Les photos qui remplissent toutes les règles y apparaissent automatiquement.");
                    ui.add_space(8.0);
                    egui::Grid::new("smart").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                        ui.label("Nom");
                        ui.text_edit_singleline(&mut sm.name);
                        ui.end_row();
                        ui.label("Note minimale");
                        ui.add(egui::Slider::new(&mut sm.min_rating, 0..=5).suffix(" ★"));
                        ui.end_row();
                        ui.label("Drapeau");
                        egui::ComboBox::from_id_salt("sm-flag").selected_text(["Tous", "Retenues", "Non rejetées", "Rejetées"][match sm.flag { 1 => 1, 2 => 2, -1 => 3, _ => 0 }]).show_ui(ui, |ui| {
                            for (v, l) in [(0i8, "Tous"), (1, "Retenues"), (2, "Non rejetées"), (-1, "Rejetées")] {
                                ui.selectable_value(&mut sm.flag, v, l);
                            }
                        });
                        ui.end_row();
                        ui.label("Étiquette");
                        egui::ComboBox::from_id_salt("sm-label").selected_text(if sm.label == 0 { "Toutes" } else { LABELS[(sm.label - 1) as usize].0 }).show_ui(ui, |ui| {
                            ui.selectable_value(&mut sm.label, 0, "Toutes");
                            for (k, (n, _)) in LABELS.iter().enumerate() {
                                ui.selectable_value(&mut sm.label, k as u8 + 1, *n);
                            }
                        });
                        ui.end_row();
                        ui.label("Mot-clé");
                        ui.text_edit_singleline(&mut sm.keyword);
                        ui.end_row();
                        ui.label("Texte contenu");
                        ui.text_edit_singleline(&mut sm.text);
                        ui.end_row();
                        ui.label("Retouche");
                        let mut ed = sm.edited as usize;
                        if pro::segmented(ui, &mut ed, &["Toutes", "Modifiées", "Intactes"]) {
                            sm.edited = ed as u8;
                        }
                        ui.end_row();
                        ui.label("Type");
                        let mut kd = sm.kind as usize;
                        if pro::segmented(ui, &mut kd, &["Tous", "RAW", "JPEG, PNG…"]) {
                            sm.kind = kd as u8;
                        }
                        ui.end_row();
                    });
                    ui.add_space(10.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::primary(ui, "Enregistrer").clicked() {
                            save = true;
                        }
                    });
                });
            }
            if save && let Some((idx, mut sm)) = self.smart_edit.take() {
                if sm.name.trim().is_empty() {
                    sm.name = format!("Collection {}", self.catalog.smart.len() + 1);
                }
                match idx {
                    Some(k) if k < self.catalog.smart.len() => self.catalog.smart[k] = sm,
                    _ => {
                        self.catalog.smart.push(sm);
                        self.collection = Collection::Smart(self.catalog.smart.len() - 1);
                    }
                }
                self.dirty_catalog = true;
            } else if !keep {
                self.smart_edit = None;
            }
        }
    }

    fn export_form(&mut self, ui: &mut Ui, n: usize) -> bool {
        let o = &mut self.catalog.export;
        let before = o.clone();
        let mut go = false;
        ui.label(RichText::new(format!("Exporter {n} photo{}", if n > 1 { "s" } else { "" })).font(semibold(18.0)).color(INK_STRONG));
        ui.add_space(8.0);
        egui::ScrollArea::vertical().max_height(ui.ctx().content_rect().height() * 0.68).show(ui, |ui| {
            heading(ui, "EMPLACEMENT");
            ui.checkbox(&mut o.same_folder, "Même dossier que l'original");
            if !o.same_folder {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(o.folder.as_ref().map(|f| widgets::short(&f.display().to_string(), 46)).unwrap_or_else(|| "Choisi au moment d'exporter".into())).color(MUTED));
                    if pro::ghost(ui, "Choisir…").clicked()
                        && let Some(d) = rfd::FileDialog::new().set_title("Dossier d'export").pick_folder()
                    {
                        o.folder = Some(d);
                    }
                    if o.folder.is_some() && pro::ghost(ui, "Effacer").clicked() {
                        o.folder = None;
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label("Sous-dossier");
                ui.add(egui::TextEdit::singleline(&mut o.subfolder).hint_text("facultatif").desired_width(160.0));
                ui.label("Si le fichier existe");
                egui::ComboBox::from_id_salt("collision").selected_text(meta::COLLISION[o.collision.min(2)]).show_ui(ui, |ui| {
                    for (k, c) in meta::COLLISION.iter().enumerate() {
                        ui.selectable_value(&mut o.collision, k, *c);
                    }
                });
            });
            ui.add_space(10.0);
            heading(ui, "NOM DES FICHIERS");
            ui.horizontal(|ui| {
                let current = meta::TEMPLATES.iter().find(|t| t.1 == o.template).map(|t| t.0).unwrap_or("Personnalisé");
                egui::ComboBox::from_id_salt("template").selected_text(current).width(180.0).show_ui(ui, |ui| {
                    for (name, t) in meta::TEMPLATES {
                        if ui.selectable_label(current == name, name).clicked() {
                            o.template = if t.is_empty() { "{texte}-{date}-{n3}".into() } else { t.into() };
                        }
                    }
                });
                ui.add(egui::TextEdit::singleline(&mut o.template).desired_width(200.0));
            });
            widgets::caption(ui, "Balises : {nom} {n} {n3} {date} {titre} {note} {texte} {suffixe}");
            ui.horizontal(|ui| {
                ui.label("Texte");
                ui.add(egui::TextEdit::singleline(&mut o.custom).hint_text("ex. mariage").desired_width(120.0));
                ui.label("Suffixe");
                ui.add(egui::TextEdit::singleline(&mut o.suffix).hint_text("ex. -web").desired_width(80.0));
                ui.label("Début");
                ui.add(egui::DragValue::new(&mut o.start).range(0..=99999));
            });
            ui.add_space(10.0);
            heading(ui, "FORMAT");
            ui.horizontal(|ui| {
                pro::segmented(ui, &mut o.format, &meta::FORMATS);
                if o.format == 0 {
                    ui.label("Qualité");
                    ui.add(egui::Slider::new(&mut o.quality, 40..=100));
                }
            });
            widgets::caption(ui, match o.format {
                0 => "Espace colorimétrique sRGB, idéal pour le web et les réseaux.",
                1 => "PNG sans perte, sRGB : fichiers plus lourds.",
                2 => "TIFF 8 bits sans perte, sRGB : pour l'impression ou une retouche ailleurs.",
                _ => "WebP sans perte, sRGB : léger et net pour le web.",
            });
            ui.add_space(10.0);
            heading(ui, "TAILLE");
            ui.horizontal_wrapped(|ui| {
                for (name, mode, w, h) in meta::SIZE_PRESETS {
                    let on = o.size == w && (w == 0 || (o.size_mode == mode && (mode != 4 || o.size_h == h)));
                    if ui.selectable_label(on, name).clicked() {
                        (o.size_mode, o.size, o.size_h) = (mode, w, h);
                    }
                }
            });
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("size-mode").selected_text(meta::SIZE_MODES[o.size_mode.min(4)]).show_ui(ui, |ui| {
                    for (k, m) in meta::SIZE_MODES.iter().enumerate() {
                        ui.selectable_value(&mut o.size_mode, k, *m);
                    }
                });
                ui.add(egui::DragValue::new(&mut o.size).range(0..=20000).suffix(" px"));
                if o.size_mode == 4 {
                    ui.label("×");
                    ui.add(egui::DragValue::new(&mut o.size_h).range(0..=20000).suffix(" px"));
                }
                ui.checkbox(&mut o.no_enlarge, "Ne pas agrandir");
            });
            ui.add_space(10.0);
            heading(ui, "ACCENTUATION DE SORTIE");
            ui.horizontal(|ui| {
                pro::segmented(ui, &mut o.sharpen_for, &meta::SHARPEN_FOR);
                if o.sharpen_for > 0 {
                    pro::segmented(ui, &mut o.sharpen_amount, &meta::SHARPEN_AMOUNT);
                }
            });
            ui.add_space(10.0);
            heading(ui, "MÉTADONNÉES");
            egui::ComboBox::from_id_salt("meta").selected_text(meta::METADATA[o.metadata.min(3)]).width(240.0).show_ui(ui, |ui| {
                for (k, m) in meta::METADATA.iter().enumerate() {
                    ui.selectable_value(&mut o.metadata, k, *m);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Auteur");
                ui.add(egui::TextEdit::singleline(&mut o.creator).desired_width(150.0));
                ui.label("Copyright");
                ui.add(egui::TextEdit::singleline(&mut o.copyright).hint_text("© 2026 …").desired_width(150.0));
            });
            ui.add_space(10.0);
            heading(ui, "FILIGRANE");
            ui.checkbox(&mut o.watermark, "Ajouter un filigrane texte");
            if o.watermark {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut o.watermark_text).hint_text("© ton nom").desired_width(200.0));
                    if o.watermark_text.is_empty() && !o.copyright.is_empty() && pro::ghost(ui, "Utiliser le copyright").clicked() {
                        o.watermark_text = o.copyright.clone();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Taille");
                    ui.add(egui::Slider::new(&mut o.watermark_size, 1.0..=20.0).suffix(" %"));
                    ui.label("Opacité");
                    ui.add(egui::Slider::new(&mut o.watermark_opacity, 5.0..=100.0).suffix(" %"));
                });
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("wm-pos").selected_text(meta::POSITIONS[o.watermark_pos.min(4)]).show_ui(ui, |ui| {
                        for (k, p) in meta::POSITIONS.iter().enumerate() {
                            ui.selectable_value(&mut o.watermark_pos, k, *p);
                        }
                    });
                    ui.checkbox(&mut o.watermark_dark, "Texte sombre");
                });
            }
            ui.add_space(10.0);
            heading(ui, "APRÈS L'EXPORT");
            ui.checkbox(&mut o.open_after, "Ouvrir le dossier dans l'Explorateur");
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if pro::ghost(ui, "Réglages par défaut").clicked() {
                *o = ExportOpts::default();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if pro::primary(ui, "Exporter").clicked() {
                    go = true;
                }
            });
        });
        if *o != before {
            self.dirty_catalog = true;
        }
        go
    }
}

fn view_switch(ui: &mut Ui, view: &mut View, can_develop: bool) {
    let items = [(View::Develop, "Développement"), (View::Library, "Bibliothèque")];
    let (outer, _) = ui.allocate_exact_size(vec2(232.0, 30.0), egui::Sense::hover());
    ui.painter().rect_filled(outer, CornerRadius::same(15), FIELD);
    for (k, (v, label)) in items.iter().rev().enumerate() {
        let r = Rect::from_min_size(outer.min + vec2(3.0 + k as f32 * 113.0, 3.0), vec2(113.0, 24.0));
        let enabled = *v == View::Library || can_develop;
        let resp = ui.interact(r, ui.make_persistent_id(("seg", k)), if enabled { egui::Sense::click() } else { egui::Sense::hover() });
        let on = *view == *v || (*v == View::Library && matches!(*view, View::Compare | View::Survey));
        if on {
            ui.painter().rect_filled(r, CornerRadius::same(12), Color32::from_rgb(0x4a, 0x4a, 0x52));
        }
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, *label, if on { semibold(12.5) } else { egui::FontId::proportional(12.5) }, if !enabled { studio_ui::pro::FAINT } else if on { INK_STRONG } else { MUTED });
        if resp.clicked() {
            *view = *v;
        }
    }
}

fn export_glyph(p: &egui::Painter, r: Rect, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    p.line_segment([at(0.5, 0.05), at(0.5, 0.65)], Stroke::new(2.0, c));
    p.line_segment([at(0.5, 0.05), at(0.25, 0.3)], Stroke::new(2.0, c));
    p.line_segment([at(0.5, 0.05), at(0.75, 0.3)], Stroke::new(2.0, c));
    p.add(egui::Shape::line(vec![at(0.12, 0.55), at(0.12, 0.95), at(0.88, 0.95), at(0.88, 0.55)], Stroke::new(2.0, c)));
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_decorations(false)
            .with_title("Lumen")
            .with_icon(Arc::new(studio_ui::app_window_icon("lumen")))
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([1000.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native("Lumen", options, Box::new(|cc| Ok(Box::new(Lumen::new(cc)))))
}
