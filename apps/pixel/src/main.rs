#![windows_subsystem = "windows"]

mod canvas;
mod doc;
mod filters;
mod inpaint;
mod panels;
mod shapes;
mod sheets;
mod tools;
mod tutos;

use doc::{Area, Blend, Brush, Document, Paint, Xform};
use eframe::egui::{self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, Painter, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2, pos2, vec2};
use filters::Kind;
use image::{ImageEncoder, RgbaImage, imageops};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use studio_ui::pro::{self, BLUE, CHROME, FAINT, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};
use tools::{GROUPS, Tool};

pub use shapes::FONTS;

pub enum Sheet {
    Canvas { w: u32, h: u32, anchor: usize, relative: bool, ext: usize },
    ImageSize { w: u32, h: u32, keep: bool, percent: bool, resample: usize, dpi: f32 },
    Transform(Xform),
    Effect { kind: u8, size: f32, dist: f32, angle: f32, color: [u8; 4], color2: [u8; 4], position: u8, blend: usize, depth: f32 },
    SelectBy { kind: u8, px: f32 },
    ColorRange { mode: usize, fuzz: f32 },
    Fill { content: usize, opacity: f32, blend: usize },
    Export { format: usize, quality: u8, scale: f32, bg: usize, level: usize, ico: [bool; 7], selection: bool },
    RotateCanvas { angle: f32 },
    Guide { vertical: bool, pos: f32 },
    Note { idx: Option<usize>, at: (f32, f32), text: String },
    SaveSelection { name: String },
    Prefs,
    Shortcuts,
}

pub struct TextDraft {
    pub at: (f32, f32),
    pub text: String,
    pub vertical: bool,
}

pub struct Adjust {
    pub kind: Kind,
    pub vals: Vec<f32>,
    pub base: RgbaImage,
    pub layer: Option<usize>,
    pub old: Vec<f32>,
    pub opacity: f32,
    pub blend: usize,
    pub preview: bool,
    pub preset_name: String,
    pub fresh: bool,
}

struct NewDoc {
    name: String,
    w: u32,
    h: u32,
    background: usize,
    preset: usize,
}

const PRESETS: [(&str, &str, u32, u32); 12] = [
    ("Full HD", "Écran 16:9", 1920, 1080),
    ("4K UHD", "Écran 16:9", 3840, 2160),
    ("Miniature YouTube", "Vidéo", 1280, 720),
    ("Publication carrée", "Réseaux sociaux", 1080, 1080),
    ("Portrait 4:5", "Réseaux sociaux", 1080, 1350),
    ("Story", "Réseaux sociaux", 1080, 1920),
    ("A4 300 ppp", "Impression", 2480, 3508),
    ("A5 300 ppp", "Impression", 1748, 2480),
    ("Carte de visite", "Impression", 1050, 600),
    ("Icône", "Interface", 1024, 1024),
    ("Bannière", "Web", 1500, 500),
    ("Fond d'écran", "Bureau", 2560, 1440),
];

const BACKGROUNDS: [&str; 5] = ["Blanc", "Transparent", "Noir", "Couleur principale", "Couleur secondaire"];

pub const SWATCH_SETS: [(&str, &[[u8; 3]]); 7] = [
    ("Essentiel", &[[20, 20, 24], [255, 255, 255], [255, 69, 58], [255, 159, 10], [255, 214, 10], [48, 209, 88], [10, 132, 255], [191, 90, 242]]),
    ("Pastel", &[[255, 209, 220], [255, 229, 180], [255, 250, 205], [204, 255, 229], [204, 229, 255], [229, 204, 255], [240, 240, 240], [200, 220, 210]]),
    ("Terre", &[[94, 60, 40], [140, 90, 60], [190, 140, 90], [225, 190, 140], [110, 120, 70], [70, 90, 60], [160, 110, 70], [60, 45, 35]]),
    ("Néon", &[[255, 0, 128], [0, 255, 200], [255, 240, 0], [0, 160, 255], [180, 0, 255], [255, 90, 0], [57, 255, 20], [255, 255, 255]]),
    ("Gris", &[[0, 0, 0], [36, 36, 36], [73, 73, 73], [109, 109, 109], [146, 146, 146], [182, 182, 182], [219, 219, 219], [255, 255, 255]]),
    ("Peau", &[[255, 224, 196], [240, 200, 160], [224, 172, 132], [198, 134, 98], [160, 104, 72], [120, 76, 52], [86, 54, 38], [60, 38, 28]]),
    ("Matériel", &[[244, 67, 54], [233, 30, 99], [156, 39, 176], [63, 81, 181], [3, 169, 244], [0, 150, 136], [139, 195, 74], [255, 193, 7]]),
];

pub enum Drag {
    None,
    Marquee((f32, f32)),
    Move { from: (f32, f32), original: RgbaImage },
    Gradient((f32, f32)),
    Lasso(Vec<(f32, f32)>),
    Crop((f32, f32)),
    Shape((f32, f32)),
    Ruler((f32, f32)),
    Patch { from: (f32, f32) },
    Guide { idx: Option<usize>, vertical: bool },
    Rotate { start: f32, from: f32 },
}

pub struct Pixel {
    doc: Document,
    home: bool,
    recents: Vec<PathBuf>,
    composite: RgbaImage,
    tex: Option<TextureHandle>,
    tex_filter: TextureOptions,
    checker: TextureHandle,
    icon: TextureHandle,
    thumbs: Vec<(TextureHandle, Option<TextureHandle>)>,
    thumbs_version: u64,
    tool: Tool,
    picks: [Tool; GROUPS.len()],
    brush: Brush,
    brush_preset: usize,
    smoothing: f32,
    primary: Color32,
    secondary: Color32,
    swatches: Vec<Color32>,
    swatch_set: usize,
    recent_colors: Vec<Color32>,
    color_mode: usize,
    tolerance: u8,
    contiguous: bool,
    sample_all: bool,
    sel_mode: u8,
    feather: f32,
    marquee_style: usize,
    marquee_ratio: (f32, f32),
    marquee_size: (u32, u32),
    sel_prev: Option<Vec<u8>>,
    last_sel: Option<Vec<u8>>,
    poly: Vec<(f32, f32)>,
    mag_width: f32,
    zoom: f32,
    offset: Vec2,
    view_angle: f32,
    fit_pending: bool,
    last: Option<(f32, f32)>,
    smooth_pos: Option<(f32, f32)>,
    cursor: Option<(f32, f32)>,
    drag: Drag,
    remove_stroke: Vec<(f32, f32)>,
    filling: Option<(usize, std::sync::mpsc::Receiver<RgbaImage>)>,
    gradient_end: Option<(f32, f32)>,
    new_doc: Option<NewDoc>,
    adjust: Option<Adjust>,
    paint: Option<Paint>,
    clone_src: Option<(f32, f32)>,
    clone_offset: Option<(f32, f32)>,
    clone_aligned: bool,
    history_src: Option<RgbaImage>,
    erase_history: bool,
    sponge_saturate: bool,
    dodge_range: u8,
    mixer_wet: f32,
    finger_paint: bool,
    bg_tol: f32,
    bg_continuous: bool,
    fill_opacity: f32,
    grad_kind: usize,
    grad_reverse: bool,
    grad_opacity: f32,
    grad_dither: bool,
    grad_preset: usize,
    grad_mode: Blend,
    shape_fill: bool,
    shape_stroke: bool,
    stroke_w: f32,
    corner: f32,
    sides: u32,
    star: bool,
    inset: f32,
    custom: usize,
    arrow_start: bool,
    arrow_end: bool,
    shape_layer: bool,
    shape_mode: Blend,
    text_font: usize,
    text_size: f32,
    text_bold: bool,
    text_italic: bool,
    text_underline: bool,
    text_align: usize,
    tracking: f32,
    leading: f32,
    text: Option<TextDraft>,
    crop: Option<Area>,
    crop_ratio: usize,
    crop_overlay: usize,
    auto_select: bool,
    show_bounds: bool,
    zoom_out: bool,
    picker_size: usize,
    samplers: Vec<(f32, f32)>,
    ruler: Option<((f32, f32), (f32, f32))>,
    notes: Vec<((f32, f32), String)>,
    patch_diffusion: f32,
    mask_tex: Option<(usize, TextureHandle)>,
    help: studio_ui::help::Help,
    renaming: Option<(usize, String)>,
    error: Option<String>,
    title: String,
    clipboard: Option<RgbaImage>,
    symmetry: bool,
    symmetry_v: bool,
    show_grid: bool,
    grid: u32,
    grid_sub: u32,
    grid_color: usize,
    pixel_grid: bool,
    rulers: bool,
    guides: Vec<(bool, f32)>,
    show_guides: bool,
    lock_guides: bool,
    snap: bool,
    extras: bool,
    pasteboard: usize,
    cursor_mode: usize,
    hide_panels: bool,
    sheet: Option<Sheet>,
    view_doc: Option<Rect>,
    hist: Option<(u64, [[u32; 256]; 4])>,
    hist_channel: usize,
    saved_sel: Vec<(String, Vec<u8>)>,
    last_filter: Option<(Kind, Vec<f32>)>,
    user_presets: Vec<(Kind, String, Vec<f32>)>,
}

fn data_file(name: &str) -> PathBuf {
    studio_core::data_dir().join("pixel").join(name)
}

fn write_data(name: &str, text: &str) {
    let file = data_file(name);
    let _ = std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")));
    let _ = std::fs::write(file, text);
}

impl Pixel {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        let checker = {
            let mut px = Vec::with_capacity(16 * 16 * 4);
            for y in 0..16 {
                for x in 0..16 {
                    let v = if (x / 8 + y / 8) % 2 == 0 { 0xcc } else { 0xff };
                    px.extend_from_slice(&[v, v, v, 255]);
                }
            }
            cc.egui_ctx.load_texture("checker", egui::ColorImage::from_rgba_unmultiplied([16, 16], &px), TextureOptions { wrap_mode: egui::TextureWrapMode::Repeat, ..TextureOptions::NEAREST })
        };
        let recents = std::fs::read_to_string(data_file("recents.txt")).map(|s| s.lines().map(PathBuf::from).filter(|p| p.exists()).collect()).unwrap_or_default();
        let swatches: Vec<Color32> = std::fs::read_to_string(data_file("swatches.txt"))
            .ok()
            .map(|s| s.lines().filter_map(|l| u32::from_str_radix(l.trim().trim_start_matches('#'), 16).ok()).map(|v| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)).collect::<Vec<_>>())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| SWATCH_SETS[0].1.iter().map(|c| Color32::from_rgb(c[0], c[1], c[2])).collect());
        let user_presets = std::fs::read_to_string(data_file("presets.txt"))
            .map(|s| {
                s.lines()
                    .filter_map(|l| {
                        let mut it = l.splitn(3, '|');
                        let (k, n, v) = (it.next()?, it.next()?, it.next()?);
                        let kind = filters::ADJUSTMENTS.iter().chain(filters::QUICK).chain(filters::FILTERS.iter().flat_map(|g| g.1.iter())).find(|x| format!("{x:?}") == k)?;
                        Some((*kind, n.to_string(), v.split(',').filter_map(|x| x.parse().ok()).collect()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut me = Self {
            doc: Document::new(1920, 1080, Some([255, 255, 255, 255])),
            home: true,
            recents,
            composite: RgbaImage::new(1, 1),
            tex: None,
            tex_filter: TextureOptions::LINEAR,
            checker,
            icon: studio_ui::icon_texture(&cc.egui_ctx, "pixel"),
            thumbs: Vec::new(),
            thumbs_version: u64::MAX,
            tool: Tool::Brush,
            picks: GROUPS.map(|g| g[0]),
            brush: Brush::new(12.0, 0.8, 1.0),
            brush_preset: 0,
            smoothing: 0.0,
            primary: Color32::from_rgb(20, 20, 24),
            secondary: Color32::WHITE,
            swatches,
            swatch_set: 0,
            recent_colors: Vec::new(),
            color_mode: 0,
            tolerance: 32,
            contiguous: true,
            sample_all: false,
            sel_mode: 0,
            feather: 0.0,
            marquee_style: 0,
            marquee_ratio: (16.0, 9.0),
            marquee_size: (512, 512),
            sel_prev: None,
            last_sel: None,
            poly: Vec::new(),
            mag_width: 10.0,
            zoom: 1.0,
            offset: Vec2::ZERO,
            view_angle: 0.0,
            fit_pending: true,
            last: None,
            smooth_pos: None,
            cursor: None,
            drag: Drag::None,
            remove_stroke: Vec::new(),
            filling: None,
            gradient_end: None,
            new_doc: None,
            adjust: None,
            paint: None,
            clone_src: None,
            clone_offset: None,
            clone_aligned: true,
            history_src: None,
            erase_history: false,
            sponge_saturate: false,
            dodge_range: 1,
            mixer_wet: 0.5,
            finger_paint: false,
            bg_tol: 0.25,
            bg_continuous: true,
            fill_opacity: 1.0,
            grad_kind: 0,
            grad_reverse: false,
            grad_opacity: 1.0,
            grad_dither: true,
            grad_preset: 0,
            grad_mode: Blend::Normal,
            shape_fill: true,
            shape_stroke: false,
            stroke_w: 6.0,
            corner: 0.0,
            sides: 6,
            star: false,
            inset: 0.5,
            custom: 0,
            arrow_start: false,
            arrow_end: false,
            shape_layer: true,
            shape_mode: Blend::Normal,
            text_font: 0,
            text_size: 72.0,
            text_bold: false,
            text_italic: false,
            text_underline: false,
            text_align: 0,
            tracking: 0.0,
            leading: 1.0,
            text: None,
            crop: None,
            crop_ratio: 0,
            crop_overlay: 1,
            auto_select: false,
            show_bounds: false,
            zoom_out: false,
            picker_size: 0,
            samplers: Vec::new(),
            ruler: None,
            notes: Vec::new(),
            patch_diffusion: 6.0,
            mask_tex: None,
            help: studio_ui::help::Help::new("Pixel", tutos::PIXEL),
            renaming: None,
            error: None,
            title: String::new(),
            clipboard: None,
            symmetry: false,
            symmetry_v: false,
            show_grid: false,
            grid: 32,
            grid_sub: 1,
            grid_color: 0,
            pixel_grid: true,
            rulers: false,
            guides: Vec::new(),
            show_guides: true,
            lock_guides: false,
            snap: true,
            extras: true,
            pasteboard: 0,
            cursor_mode: 0,
            hide_panels: false,
            sheet: None,
            view_doc: None,
            hist: None,
            hist_channel: 0,
            saved_sel: Vec::new(),
            last_filter: None,
            user_presets,
        };
        if let Some(path) = std::env::args().nth(1) {
            me.open_path(Path::new(&path));
        }
        me
    }

    fn set_doc(&mut self, doc: Document) {
        self.history_src = Some(doc.layers[0].img.clone());
        self.doc = doc;
        self.tex = None;
        self.fit_pending = true;
        self.renaming = None;
        self.home = false;
        self.thumbs_version = u64::MAX;
        self.view_angle = 0.0;
        self.samplers.clear();
        self.notes.clear();
        self.ruler = None;
        self.guides.clear();
        self.crop = None;
        self.poly.clear();
    }

    fn remember(&mut self, path: &Path) {
        self.recents.retain(|p| p != path);
        self.recents.insert(0, path.to_path_buf());
        self.recents.truncate(8);
        let text: Vec<String> = self.recents.iter().map(|p| p.display().to_string()).collect();
        write_data("recents.txt", &text.join("\n"));
    }

    pub fn save_swatches(&self) {
        let text: Vec<String> = self.swatches.iter().map(|c| format!("{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())).collect();
        write_data("swatches.txt", &text.join("\n"));
    }

    pub fn save_presets(&self) {
        let text: Vec<String> = self.user_presets.iter().map(|(k, n, v)| format!("{k:?}|{}|{}", n.replace('|', " "), v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))).collect();
        write_data("presets.txt", &text.join("\n"));
    }

    fn open_path(&mut self, path: &Path) {
        match image::open(path) {
            Ok(img) => {
                let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                self.set_doc(Document::from_image(name, img.into_rgba8()));
                self.remember(path);
            }
            Err(e) => self.error = Some(format!("Impossible d'ouvrir l'image : {e}")),
        }
    }

    fn open_dialog(&mut self) {
        if let Some(p) = image_dialog().pick_file() {
            self.open_path(&p);
        }
    }

    fn import_layer(&mut self) {
        if let Some(p) = image_dialog().pick_file() {
            match image::open(&p) {
                Ok(img) => {
                    let name = p.file_stem().map(|s| s.to_string_lossy().into_owned());
                    self.doc.next_label = Some("Importer".into());
                    self.doc.add_layer(name, Some(img.into_rgba8()));
                }
                Err(e) => self.error = Some(format!("Impossible d'importer : {e}")),
            }
        }
    }

    fn copy_image(&mut self, img: RgbaImage) {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_image(arboard::ImageData { width: img.width() as usize, height: img.height() as usize, bytes: std::borrow::Cow::Owned(img.as_raw().clone()) });
        }
        self.clipboard = Some(img);
    }

    fn copy(&mut self) {
        let img = match self.doc.copy_selection() {
            Some((img, _, _)) => img,
            None => self.doc.layers[self.doc.active].img.clone(),
        };
        self.copy_image(img);
    }

    fn paste(&mut self, in_place: bool) {
        let from_system = arboard::Clipboard::new().ok().and_then(|mut cb| cb.get_image().ok()).and_then(|d| RgbaImage::from_raw(d.width as u32, d.height as u32, d.bytes.into_owned()));
        let Some(img) = from_system.or_else(|| self.clipboard.clone()) else {
            self.error = Some("Le presse-papiers ne contient pas d'image".into());
            return;
        };
        if self.home {
            self.set_doc(Document::from_image("Collage".into(), img));
            return;
        }
        let at = match (in_place, self.doc.selection) {
            (true, Some(s)) => (s.x0 as i64, s.y0 as i64),
            _ => ((self.doc.width() as i64 - img.width() as i64) / 2, (self.doc.height() as i64 - img.height() as i64) / 2),
        };
        self.doc.next_label = Some("Coller".into());
        self.doc.paste_image(img, at, "Collage");
    }

    fn doc_from_clipboard(&mut self) {
        match arboard::Clipboard::new().ok().and_then(|mut cb| cb.get_image().ok()).and_then(|d| RgbaImage::from_raw(d.width as u32, d.height as u32, d.bytes.into_owned())).or_else(|| self.clipboard.clone()) {
            Some(img) => self.set_doc(Document::from_image("Presse-papiers".into(), img)),
            None => self.error = Some("Le presse-papiers ne contient pas d'image".into()),
        }
    }

    pub fn export_with(&mut self, format: usize, quality: u8, scale: f32, bg: usize, level: usize, ico: [bool; 7], selection: bool) {
        let exts = ["png", "jpg", "webp", "bmp", "tif", "gif", "ico"];
        let ext = exts[format.min(6)];
        let Some(path) = rfd::FileDialog::new().set_file_name(format!("{}.{ext}", self.doc.name)).add_filter(ext.to_uppercase(), &[ext]).save_file() else { return };
        let mut flat = if selection { self.doc.copy_merged() } else { self.doc.flatten() };
        if (scale - 100.0).abs() > 0.5 {
            let k = scale / 100.0;
            flat = imageops::resize(&flat, ((flat.width() as f32 * k) as u32).max(1), ((flat.height() as f32 * k) as u32).max(1), imageops::FilterType::Lanczos3);
        }
        let bg_color = match bg {
            0 => [255, 255, 255, 255],
            1 => [0, 0, 0, 255],
            _ => self.secondary.to_array(),
        };
        let matte = |img: &RgbaImage| {
            let mut out = RgbaImage::from_pixel(img.width(), img.height(), image::Rgba(bg_color));
            imageops::overlay(&mut out, img, 0, 0);
            out
        };
        let result = (|| -> Result<(), String> {
            let file = || std::fs::File::create(&path).map(std::io::BufWriter::new).map_err(|e| e.to_string());
            match format {
                0 => {
                    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
                    let c = [CompressionType::Fast, CompressionType::Default, CompressionType::Best][level.min(2)];
                    PngEncoder::new_with_quality(file()?, c, FilterType::Adaptive).write_image(flat.as_raw(), flat.width(), flat.height(), image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())
                }
                1 => {
                    let rgb = image::DynamicImage::ImageRgba8(matte(&flat)).to_rgb8();
                    image::codecs::jpeg::JpegEncoder::new_with_quality(file()?, quality).encode_image(&rgb).map_err(|e| e.to_string())
                }
                3 => image::DynamicImage::ImageRgba8(matte(&flat)).to_rgb8().save(&path).map_err(|e| e.to_string()),
                6 => {
                    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
                    let mut frames = Vec::new();
                    for (s, on) in sizes.iter().zip(ico) {
                        if !on {
                            continue;
                        }
                        let k = *s as f32 / flat.width().max(flat.height()) as f32;
                        let small = imageops::resize(&flat, ((flat.width() as f32 * k) as u32).max(1), ((flat.height() as f32 * k) as u32).max(1), imageops::FilterType::Lanczos3);
                        let mut sq = RgbaImage::new(*s, *s);
                        imageops::overlay(&mut sq, &small, ((*s - small.width()) / 2) as i64, ((*s - small.height()) / 2) as i64);
                        frames.push(image::codecs::ico::IcoFrame::as_png(sq.as_raw(), *s, *s, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())?);
                    }
                    if frames.is_empty() {
                        return Err("choisis au moins une taille d'icône".into());
                    }
                    image::codecs::ico::IcoEncoder::new(file()?).encode_images(&frames).map_err(|e| e.to_string())
                }
                _ => flat.save(&path).map_err(|e| e.to_string()),
            }
        })();
        match result {
            Ok(()) => {
                self.doc.modified = false;
                self.remember(&path);
            }
            Err(e) => self.error = Some(format!("Export impossible : {e}")),
        }
    }

    fn quick_export(&mut self) {
        self.export_with(0, 90, 100.0, 0, 1, [false; 7], false);
    }

    fn export_layers(&mut self) {
        let Some(dir) = rfd::FileDialog::new().pick_folder() else { return };
        let mut n = 0;
        for (i, l) in self.doc.layers.iter().enumerate() {
            if l.adjust.is_some() {
                continue;
            }
            let name: String = l.name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).collect();
            if l.img.save(dir.join(format!("{:02} {}.png", i + 1, name.trim()))).is_ok() {
                n += 1;
            }
        }
        self.error = Some(format!("{n} calque(s) exporté(s) en PNG"));
    }

    pub fn export_sheet(&mut self) {
        self.sheet = Some(Sheet::Export { format: 0, quality: 90, scale: 100.0, bg: 0, level: 1, ico: [true, false, true, true, false, false, true], selection: false });
    }

    fn start_fill(&mut self, mask: Vec<bool>, ctx: &egui::Context) {
        if self.filling.is_some() || !mask.iter().any(|m| *m) {
            return;
        }
        let layer = self.doc.active;
        let img = self.doc.layers[layer].img.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
            let _ = tx.send(inpaint::content_aware_fill(&img, &mask, seed));
            ctx.request_repaint();
        });
        self.filling = Some((layer, rx));
    }

    fn finish_fill(&mut self) {
        let Some((layer, rx)) = &self.filling else { return };
        if let Ok(img) = rx.try_recv() {
            let layer = *layer;
            self.filling = None;
            self.remove_stroke.clear();
            if layer < self.doc.layers.len() && self.doc.layers[layer].img.dimensions() == img.dimensions() {
                self.doc.next_label = Some("Suppression intelligente".into());
                self.doc.checkpoint();
                self.doc.layers[layer].img = img;
                self.doc.touch_all();
            }
        }
    }

    pub fn content_fill_selection(&mut self, ctx: &egui::Context) {
        if self.doc.selection.is_none() {
            return;
        }
        let w = self.doc.width();
        let mask: Vec<bool> = (0..w * self.doc.height()).map(|i| self.doc.selected(i % w, i / w)).collect();
        self.start_fill(mask, ctx);
    }

    fn stroke_mask(&self) -> Vec<bool> {
        let (w, h) = (self.doc.width() as i32, self.doc.height() as i32);
        let mut mask = vec![false; (w * h) as usize];
        let r = self.brush.radius.max(1.0);
        let mut pts = Vec::new();
        for pair in self.remove_stroke.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let n = (((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt() / (r * 0.4)).ceil().max(1.0) as usize;
            for k in 0..=n {
                let t = k as f32 / n as f32;
                pts.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
        }
        if self.remove_stroke.len() == 1 {
            pts.push(self.remove_stroke[0]);
        }
        for (cx, cy) in pts {
            let (x0, x1) = (((cx - r).floor() as i32).max(0), ((cx + r).ceil() as i32).min(w - 1));
            let (y0, y1) = (((cy - r).floor() as i32).max(0), ((cy + r).ceil() as i32).min(h - 1));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if (x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2) <= r * r {
                        mask[(y * w + x) as usize] = true;
                    }
                }
            }
        }
        mask
    }

    fn sync_texture(&mut self, ctx: &egui::Context) {
        let (w, h) = (self.doc.width(), self.doc.height());
        let filter = if self.zoom >= 2.0 { TextureOptions::NEAREST } else { TextureOptions::LINEAR };
        if self.tex.is_none() || self.composite.dimensions() != (w, h) {
            self.composite = RgbaImage::new(w, h);
            self.doc.touch_all();
            self.tex = None;
        }
        let Some(area) = self.doc.dirty.take() else {
            if let Some(t) = &mut self.tex
                && self.tex_filter != filter
            {
                t.set(egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], self.composite.as_raw()), filter);
                self.tex_filter = filter;
            }
            return;
        };
        self.doc.composite_area(&mut self.composite, area);
        match &mut self.tex {
            None => {
                let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], self.composite.as_raw());
                self.tex = Some(ctx.load_texture("canvas", img, filter));
                self.tex_filter = filter;
            }
            Some(t) => {
                let sub = imageops::crop_imm(&self.composite, area.x0, area.y0, area.width(), area.height()).to_image();
                let img = egui::ColorImage::from_rgba_unmultiplied([area.width() as usize, area.height() as usize], sub.as_raw());
                t.set_partial([area.x0 as usize, area.y0 as usize], img, self.tex_filter);
            }
        }
    }

    fn sync_thumbs(&mut self, ctx: &egui::Context) {
        if self.thumbs_version == self.doc.version || self.last.is_some() || !matches!(self.drag, Drag::None) {
            return;
        }
        let (w, h) = (self.doc.width(), self.doc.height());
        let s = 44.0 / w.max(h) as f32;
        let (tw, th) = (((w as f32 * s) as u32).max(1), ((h as f32 * s) as u32).max(1));
        let load = |name: String, img: &RgbaImage| {
            let t = imageops::thumbnail(img, tw, th);
            ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], t.as_raw()), TextureOptions::LINEAR)
        };
        self.thumbs = self.doc.layers.iter().enumerate().map(|(i, l)| (load(format!("thumb{i}"), &l.img), l.mask.as_ref().map(|m| load(format!("mthumb{i}"), m)))).collect();
        self.thumbs_version = self.doc.version;
    }

    fn sync_mask(&mut self, ctx: &egui::Context) {
        let Some(mask) = &self.doc.mask else {
            self.mask_tex = None;
            return;
        };
        let key = mask.as_ptr() as usize;
        if self.mask_tex.as_ref().is_some_and(|m| m.0 == key) {
            return;
        }
        let (w, h) = (self.doc.width() as usize, self.doc.height() as usize);
        let on = |i: usize| mask[i] > 127;
        let mut px = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !on(i) {
                    continue;
                }
                let edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h || !on(i - 1) || !on(i + 1) || !on(i - w) || !on(i + w);
                if edge {
                    let v = if (x + y) / 4 % 2 == 0 { 255 } else { 0 };
                    px[i * 4..i * 4 + 4].copy_from_slice(&[v, v, v, 255]);
                }
            }
        }
        let tex = ctx.load_texture("mask", egui::ColorImage::from_rgba_unmultiplied([w, h], &px), TextureOptions::NEAREST);
        self.mask_tex = Some((key, tex));
    }

    fn new_doc_sheet(&mut self) {
        self.new_doc = Some(NewDoc { name: "Sans titre".into(), w: 1920, h: 1080, background: 0, preset: 0 });
    }

    pub fn snapshot_history(&mut self) {
        self.history_src = Some(self.doc.layers[self.doc.active].img.clone());
    }

    pub fn commit_crop(&mut self) {
        if let Some(c) = self.crop.take() {
            self.doc.next_label = Some("Recadrage".into());
            self.doc.crop_to(c);
            self.fit_pending = true;
            self.guides.clear();
        }
    }

    pub fn use_color(&mut self, c: Color32) {
        self.recent_colors.retain(|x| *x != c);
        self.recent_colors.insert(0, c);
        self.recent_colors.truncate(12);
    }

    pub fn sample_color(&self, x: f32, y: f32) -> Option<Color32> {
        let src = if self.sample_all { &self.composite } else { &self.doc.layers[self.doc.active].img };
        let (w, h) = src.dimensions();
        if x < 0.0 || y < 0.0 || x >= w as f32 || y >= h as f32 {
            return None;
        }
        let r = [0i32, 1, 2, 5, 15][self.picker_size.min(4)];
        let (cx, cy) = (x as i32, y as i32);
        let mut acc = [0u32; 4];
        let mut n = 0;
        for yy in (cy - r).max(0)..=(cy + r).min(h as i32 - 1) {
            for xx in (cx - r).max(0)..=(cx + r).min(w as i32 - 1) {
                let p = src.get_pixel(xx as u32, yy as u32).0;
                for k in 0..4 {
                    acc[k] += p[k] as u32;
                }
                n += 1;
            }
        }
        let a = acc.map(|v| (v / n.max(1)) as u8);
        Some(Color32::from_rgb(a[0], a[1], a[2]))
    }

    pub fn start_adjust(&mut self, k: Kind) {
        let base = self.doc.active_target();
        if k.params().is_empty() {
            let out = k.apply(&base, &[]);
            self.doc.next_label = Some(k.title().to_string());
            self.doc.checkpoint();
            self.doc.put_filtered(&base, &out);
            self.last_filter = Some((k, Vec::new()));
        } else {
            self.adjust = Some(Adjust { kind: k, vals: k.defaults(), base, layer: None, old: Vec::new(), opacity: 1.0, blend: 0, preview: true, preset_name: String::new(), fresh: true });
        }
    }

    pub fn repeat_filter(&mut self) {
        if let Some((k, v)) = self.last_filter.clone() {
            let base = self.doc.active_target();
            let out = k.apply(&base, &v);
            self.doc.next_label = Some(k.title().to_string());
            self.doc.checkpoint();
            self.doc.put_filtered(&base, &out);
        }
    }

    pub fn edit_adjustment_layer(&mut self, i: usize) {
        if let Some((k, v)) = self.doc.layers[i].adjust.clone() {
            self.adjust = Some(Adjust { kind: k, vals: v.clone(), base: RgbaImage::new(1, 1), layer: Some(i), old: v, opacity: 1.0, blend: 0, preview: true, preset_name: String::new(), fresh: false });
        }
    }

    pub fn new_adjustment_layer(&mut self, k: Kind) {
        self.doc.next_label = Some(format!("Calque de réglage {}", k.title()));
        self.doc.add_adjustment(k, k.defaults());
        if !k.params().is_empty() {
            let i = self.doc.active;
            self.edit_adjustment_layer(i);
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() || self.new_doc.is_some() || self.adjust.is_some() || self.text.is_some() || self.sheet.is_some() {
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        let cs = M::COMMAND | M::SHIFT;
        let ca = M::COMMAND | M::ALT;
        let csa = M::COMMAND | M::SHIFT | M::ALT;
        if sc(M::COMMAND, Key::N) {
            self.new_doc_sheet();
        }
        if sc(M::COMMAND, Key::O) {
            self.open_dialog();
        }
        if self.home {
            return;
        }
        if sc(M::COMMAND, Key::W) {
            self.home = true;
        }
        if sc(cs, Key::Z) || sc(M::COMMAND, Key::Y) {
            self.doc.redo();
        } else if sc(ca, Key::Z) || sc(M::COMMAND, Key::Z) {
            self.doc.undo();
        }
        if sc(M::COMMAND, Key::Num0) {
            self.fit_pending = true;
        }
        if sc(M::COMMAND, Key::Num1) {
            self.zoom = 1.0;
            self.offset = Vec2::ZERO;
        }
        if sc(csa, Key::S) {
            self.quick_export();
        }
        if sc(cs, Key::S) || sc(M::COMMAND, Key::E) {
            self.export_sheet();
        }
        if sc(M::COMMAND, Key::A) {
            self.doc.set_selection(Some(self.doc.full()), None);
        }
        if sc(cs, Key::D) {
            if let Some(m) = self.last_sel.clone() {
                self.doc.select_mask(m);
            }
        } else if sc(M::COMMAND, Key::D) {
            self.deselect();
        }
        if sc(cs, Key::I) {
            self.doc.invert_selection();
        }
        if sc(M::SHIFT, Key::F6) {
            self.sheet = Some(Sheet::SelectBy { kind: 3, px: 4.0 });
        }
        if sc(csa, Key::L) {
            self.start_adjust(Kind::AutoContrast);
        } else if sc(cs, Key::L) {
            self.start_adjust(Kind::AutoTone);
        } else if sc(M::COMMAND, Key::L) {
            self.start_adjust(Kind::Levels);
        }
        if sc(M::COMMAND, Key::M) {
            self.start_adjust(Kind::Curves);
        }
        if sc(cs, Key::U) {
            self.start_adjust(Kind::BlackWhite);
        } else if sc(M::COMMAND, Key::U) {
            self.start_adjust(Kind::HueSat);
        }
        if sc(csa, Key::B) {
            self.start_adjust(Kind::BlackWhiteMix);
        } else if sc(cs, Key::B) {
            self.start_adjust(Kind::AutoColor);
        } else if sc(M::COMMAND, Key::B) {
            self.start_adjust(Kind::ColorBalance);
        }
        if sc(ca, Key::I) {
            self.sheet = Some(Sheet::ImageSize { w: self.doc.width(), h: self.doc.height(), keep: true, percent: false, resample: 3, dpi: 72.0 });
        } else if sc(M::COMMAND, Key::I) {
            self.start_adjust(Kind::Invert);
        }
        if sc(ca, Key::C) {
            self.sheet = Some(Sheet::Canvas { w: self.doc.width(), h: self.doc.height(), anchor: 4, relative: false, ext: 0 });
        } else if sc(cs, Key::C) {
            let img = self.doc.copy_merged();
            self.copy_image(img);
        } else if sc(M::COMMAND, Key::C) {
            self.copy();
        }
        if sc(ca, Key::F) {
            self.repeat_filter();
        }
        if sc(cs, Key::J) {
            self.doc.next_label = Some("Calque par coupure".into());
            self.doc.layer_from_selection(true);
        } else if sc(M::COMMAND, Key::J) {
            self.doc.next_label = Some("Calque par copie".into());
            if !self.doc.layer_from_selection(false) {
                self.doc.duplicate_layer();
            }
        }
        if sc(M::COMMAND, Key::X) {
            self.copy();
            self.doc.next_label = Some("Couper".into());
            self.doc.clear_selection();
        }
        if sc(cs, Key::V) {
            self.paste(true);
        } else if sc(M::COMMAND, Key::V) {
            self.paste(false);
        }
        if sc(M::COMMAND, Key::T) {
            self.sheet = Some(Sheet::Transform(Xform::identity()));
        }
        if sc(cs, Key::E) {
            self.doc.next_label = Some("Fusionner les calques visibles".into());
            self.doc.merge_visible();
        } else if sc(ca, Key::E) {
            self.doc.next_label = Some("Fusionner avec le dessous".into());
            self.doc.merge_down();
        }
        if sc(M::SHIFT, Key::F5) {
            self.sheet = Some(Sheet::Fill { content: 0, opacity: 100.0, blend: 0 });
        }
        if sc(M::ALT, Key::Backspace) {
            self.doc.next_label = Some("Remplir (principale)".into());
            self.doc.fill_selection(self.primary.to_array());
        } else if sc(M::COMMAND, Key::Backspace) {
            self.doc.next_label = Some("Remplir (secondaire)".into());
            self.doc.fill_selection(self.secondary.to_array());
        }
        if sc(M::COMMAND, Key::Quote) {
            self.show_grid = !self.show_grid;
        }
        if sc(M::COMMAND, Key::R) {
            self.rulers = !self.rulers;
        }
        if sc(cs, Key::Semicolon) {
            self.snap = !self.snap;
        } else if sc(ca, Key::Semicolon) {
            self.lock_guides = !self.lock_guides;
        } else if sc(M::COMMAND, Key::Semicolon) {
            self.show_guides = !self.show_guides;
        }
        if sc(M::COMMAND, Key::H) {
            self.extras = !self.extras;
        }
        if sc(cs, Key::N) {
            self.doc.next_label = Some("Nouveau calque".into());
            self.doc.add_layer(None, None);
        }
        if sc(ca, Key::G) {
            let a = self.doc.active;
            if a > 0 {
                self.doc.layers[a].clip = !self.doc.layers[a].clip;
                self.doc.touch_all();
            }
        }
        if sc(cs, Key::CloseBracket) {
            self.doc.move_layer_to(true);
        } else if sc(M::COMMAND, Key::CloseBracket) {
            self.doc.move_layer(true);
        }
        if sc(cs, Key::OpenBracket) {
            self.doc.move_layer_to(false);
        } else if sc(M::COMMAND, Key::OpenBracket) {
            self.doc.move_layer(false);
        }
        if sc(M::COMMAND, Key::Plus) || sc(M::COMMAND, Key::Equals) {
            self.zoom = (self.zoom * 1.25).min(64.0);
        }
        if sc(M::COMMAND, Key::Minus) {
            self.zoom = (self.zoom / 1.25).max(0.02);
        }
        if sc(M::NONE, Key::Tab) {
            self.hide_panels = !self.hide_panels;
        }
        if sc(M::NONE, Key::Backslash) {
            self.toggle_mask_edit();
        }
        let input = ctx.input(|i| {
            let mut pick = None;
            if !i.modifiers.command && !i.modifiers.alt {
                for (_, _, key, _) in tools::TOOLS {
                    if i.key_pressed(*key) {
                        pick = Some((*key, i.modifiers.shift));
                    }
                }
            }
            let digit = (0..10).find(|d| i.key_pressed([Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9][*d]) && !i.modifiers.command);
            let arrows = [Key::ArrowLeft, Key::ArrowRight, Key::ArrowUp, Key::ArrowDown].map(|k| i.key_pressed(k));
            (
                pick,
                i.key_pressed(Key::Delete) || (i.key_pressed(Key::Backspace) && !i.modifiers.any()),
                i.key_pressed(Key::X) && !i.modifiers.command,
                i.key_pressed(Key::D) && !i.modifiers.command,
                i.key_pressed(Key::OpenBracket) && !i.modifiers.command,
                i.key_pressed(Key::CloseBracket) && !i.modifiers.command,
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::Escape),
                i.modifiers.shift,
                digit,
                arrows,
            )
        });
        let (pick, del, swap, reset, smaller, bigger, enter, escape, shift, digit, arrows) = input;
        if let Some((k, s)) = pick
            && !matches!(k, Key::X | Key::D)
        {
            self.key_tool(k, s);
        }
        if enter {
            if self.crop.is_some() {
                self.commit_crop();
            } else if self.poly.len() >= 3 {
                let pts = std::mem::take(&mut self.poly);
                self.finish_lasso(&pts);
            }
        }
        if escape {
            self.crop = None;
            self.poly.clear();
            self.drag = Drag::None;
        }
        if del {
            self.doc.next_label = Some("Effacer".into());
            self.doc.clear_selection();
        }
        if swap {
            std::mem::swap(&mut self.primary, &mut self.secondary);
        }
        if reset {
            self.primary = Color32::from_rgb(20, 20, 24);
            self.secondary = Color32::WHITE;
        }
        if smaller {
            if shift {
                self.brush.hardness = (self.brush.hardness - 0.25).max(0.0);
            } else {
                self.brush.radius = (self.brush.radius / 1.2).max(0.5);
            }
        }
        if bigger {
            if shift {
                self.brush.hardness = (self.brush.hardness + 0.25).min(1.0);
            } else {
                self.brush.radius = (self.brush.radius * 1.2).min(500.0);
            }
        }
        if let Some(d) = digit {
            let v = if d == 0 { 1.0 } else { d as f32 / 10.0 };
            match self.tool {
                Tool::Gradient => self.grad_opacity = v,
                Tool::Fill | Tool::MagicEraser => self.fill_opacity = v,
                _ => self.brush.flow = v,
            }
        }
        if self.tool == Tool::Move && arrows.iter().any(|a| *a) && !self.doc.layers[self.doc.active].lock_pos {
            let step = if shift { 10 } else { 1 };
            let dx = (arrows[1] as i32 - arrows[0] as i32) * step;
            let dy = (arrows[3] as i32 - arrows[2] as i32) * step;
            self.doc.next_label = Some("Déplacement".into());
            self.doc.nudge(dx, dy);
        }
    }

    pub fn deselect(&mut self) {
        if self.doc.selection.is_some() {
            self.last_sel = Some(self.doc.sel_mask());
        }
        self.doc.set_selection(None, None);
    }

    pub fn finish_lasso(&mut self, pts: &[(f32, f32)]) {
        let prev = self.sel_prev.take().or_else(|| (self.sel_mode != 0 && self.doc.selection.is_some()).then(|| self.doc.sel_mask()));
        self.doc.select_polygon(pts);
        self.doc.feather_selection(self.feather);
        self.doc.combine_selection(prev, self.sel_mode);
    }

    fn mi(&mut self, ui: &mut Ui, label: &str, shortcut: &str) -> bool {
        self.mie(ui, true, label, shortcut)
    }

    fn mie(&mut self, ui: &mut Ui, enabled: bool, label: &str, shortcut: &str) -> bool {
        let clicked = ui.add_enabled(enabled, egui::Button::new(label).shortcut_text(RichText::new(shortcut).color(FAINT))).clicked();
        if clicked {
            self.doc.next_label = Some(label.trim_end_matches('…').to_string());
        }
        clicked
    }
}

impl eframe::App for Pixel {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        if !self.home && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F1)) {
            if self.help.open {
                self.help.open = false;
            } else {
                self.help.learn(tools::tutorial_for(self.tool));
            }
        }
        self.shortcuts(ctx);
        if let Some(f) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty())) {
            if self.home {
                self.open_path(&f);
            } else {
                match image::open(&f) {
                    Ok(img) => {
                        self.doc.next_label = Some("Importer".into());
                        self.doc.add_layer(f.file_stem().map(|s| s.to_string_lossy().into_owned()), Some(img.into_rgba8()));
                    }
                    Err(e) => self.error = Some(format!("Impossible d'importer : {e}")),
                }
            }
        }
        let title = if self.home { "Pixel".to_string() } else { format!("{}{} — Pixel", self.doc.name, if self.doc.modified { " •" } else { "" }) };
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
            if !self.hide_panels {
                self.tool_rail(root);
                self.dock(root, ctx);
            }
            self.finish_fill();
            self.sync_texture(ctx);
            self.sync_mask(ctx);
            self.canvas(root);
        }
        self.sheets(ctx);
        self.help.show(ctx);
        self.doc.next_label = None;
    }
}

impl Pixel {
    fn menu_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(40.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(10, 0))).show(root, |ui| {
            let full = ui.max_rect();
            let right_w = if self.home { 0.0 } else { 250.0 };
            if !self.home {
                let right = Rect::from_min_max(pos2(full.right() - right_w, full.top()), full.max);
                ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::primary_icon(ui, "Exporter", |p, r, c| export_glyph(p, r, c)).on_hover_text("Ctrl+E").clicked() {
                            self.export_sheet();
                        }
                        ui.add_space(6.0);
                        if pro::chip(ui, &format!("{} × {} px", self.doc.width(), self.doc.height())).on_hover_text("Taille de la zone de travail").clicked() {
                            self.sheet = Some(Sheet::Canvas { w: self.doc.width(), h: self.doc.height(), anchor: 4, relative: false, ext: 0 });
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
                        self.menu_file(ui);
                        ui.add_enabled_ui(!self.home, |ui| {
                            self.menu_edit(ui);
                            self.menu_image(ui);
                            self.menu_layer(ui);
                            self.menu_select(ui);
                            self.menu_filter(ui);
                            self.menu_view(ui);
                        });
                        self.help.menu(ui);
                    });
                });
            });
        });
    }

    fn menu_file(&mut self, ui: &mut Ui) {
        ui.menu_button("Fichier", |ui| {
            if self.mi(ui, "Nouveau…", "Ctrl+N") {
                self.new_doc_sheet();
            }
            if self.mi(ui, "Nouveau depuis le presse-papiers", "") {
                self.doc_from_clipboard();
            }
            if self.mi(ui, "Ouvrir…", "Ctrl+O") {
                self.open_dialog();
            }
            ui.menu_button("Ouvrir les fichiers récents", |ui| {
                let mut open = None;
                for p in &self.recents {
                    if ui.button(p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).clicked() {
                        open = Some(p.clone());
                    }
                }
                if self.recents.is_empty() {
                    ui.label(RichText::new("Aucun fichier récent").color(FAINT));
                }
                if let Some(p) = open {
                    self.open_path(&p);
                }
            });
            let home = self.home;
            if self.mie(ui, !home, "Importer comme calque…", "") {
                self.import_layer();
            }
            ui.separator();
            if self.mie(ui, !home, "Exporter…", "Ctrl+E") {
                self.export_sheet();
            }
            if self.mie(ui, !home, "Exportation rapide en PNG…", "Ctrl+Maj+Alt+S") {
                self.quick_export();
            }
            if self.mie(ui, !home && self.doc.selection.is_some(), "Exporter la sélection…", "") {
                self.sheet = Some(Sheet::Export { format: 0, quality: 90, scale: 100.0, bg: 0, level: 1, ico: [true; 7], selection: true });
            }
            if self.mie(ui, !home, "Exporter les calques en PNG…", "") {
                self.export_layers();
            }
            ui.separator();
            if self.mie(ui, !home, "Fermer", "Ctrl+W") {
                self.home = true;
            }
        });
    }

    fn menu_edit(&mut self, ui: &mut Ui) {
        ui.menu_button("Édition", |ui| {
            if self.mie(ui, self.doc.can_undo(), "Annuler", "Ctrl+Z") {
                self.doc.undo();
            }
            if self.mie(ui, self.doc.can_redo(), "Rétablir", "Ctrl+Y") {
                self.doc.redo();
            }
            if self.mie(ui, self.last_filter.is_some(), "Répéter le dernier filtre", "Ctrl+Alt+F") {
                self.repeat_filter();
            }
            ui.separator();
            let has_sel = self.doc.selection.is_some();
            if self.mie(ui, has_sel, "Couper", "Ctrl+X") {
                self.copy();
                self.doc.clear_selection();
            }
            if self.mi(ui, "Copier", "Ctrl+C") {
                self.copy();
            }
            if self.mi(ui, "Copier avec fusion", "Ctrl+Maj+C") {
                let img = self.doc.copy_merged();
                self.copy_image(img);
            }
            if self.mi(ui, "Coller", "Ctrl+V") {
                self.paste(false);
            }
            if self.mi(ui, "Coller en place", "Ctrl+Maj+V") {
                self.paste(true);
            }
            if self.mie(ui, has_sel, "Effacer", "Suppr") {
                self.doc.clear_selection();
            }
            ui.separator();
            if self.mi(ui, "Remplir…", "Maj+F5") {
                self.sheet = Some(Sheet::Fill { content: 0, opacity: 100.0, blend: 0 });
            }
            if self.mie(ui, has_sel, "Contour…", "") {
                self.open_sheet_effect(3);
            }
            if self.mie(ui, has_sel && self.filling.is_none(), "Remplir selon le contenu", "") {
                let ctx = ui.ctx().clone();
                self.content_fill_selection(&ctx);
            }
            ui.separator();
            if self.mi(ui, "Transformation manuelle…", "Ctrl+T") {
                self.sheet = Some(Sheet::Transform(Xform::identity()));
            }
            ui.menu_button("Transformation", |ui| {
                if self.mi(ui, "Rotation 180°", "") {
                    self.doc.transform_layer_full(&Xform { angle: 180.0, nearest: true, ..Xform::identity() });
                }
                if self.mi(ui, "Rotation 90° horaire", "") {
                    self.doc.transform_layer_full(&Xform { angle: 90.0, nearest: true, ..Xform::identity() });
                }
                if self.mi(ui, "Rotation 90° antihoraire", "") {
                    self.doc.transform_layer_full(&Xform { angle: -90.0, nearest: true, ..Xform::identity() });
                }
                if self.mi(ui, "Symétrie axe horizontal", "") {
                    self.doc.flip_layer(true);
                }
                if self.mi(ui, "Symétrie axe vertical", "") {
                    self.doc.flip_layer(false);
                }
                ui.separator();
                if ui.button("Inclinaison…").clicked() {
                    self.sheet = Some(Sheet::Transform(Xform { skew_x: 15.0, ..Xform::identity() }));
                }
                if ui.button("Perspective…").clicked() {
                    self.sheet = Some(Sheet::Transform(Xform { persp_y: 20.0, ..Xform::identity() }));
                }
            });
            ui.separator();
            if ui.button("Raccourcis clavier…").clicked() {
                self.sheet = Some(Sheet::Shortcuts);
            }
            if ui.button("Préférences…").clicked() {
                self.sheet = Some(Sheet::Prefs);
            }
        });
    }

    fn menu_image(&mut self, ui: &mut Ui) {
        ui.menu_button("Image", |ui| {
            ui.menu_button("Réglages", |ui| {
                let keys = [(Kind::Levels, "Ctrl+L"), (Kind::Curves, "Ctrl+M"), (Kind::HueSat, "Ctrl+U"), (Kind::ColorBalance, "Ctrl+B"), (Kind::BlackWhiteMix, "Ctrl+Alt+Maj+B")];
                for k in filters::ADJUSTMENTS {
                    let sc = keys.iter().find(|x| x.0 == *k).map_or("", |x| x.1);
                    if self.mi(ui, &format!("{}…", k.title()), sc) {
                        self.start_adjust(*k);
                    }
                }
                ui.separator();
                for (k, sc) in [(Kind::BlackWhite, "Ctrl+Maj+U"), (Kind::Invert, "Ctrl+I"), (Kind::Sepia, ""), (Kind::Equalize, ""), (Kind::Solarize, "")] {
                    if self.mi(ui, k.title(), sc) {
                        self.start_adjust(k);
                    }
                }
                ui.separator();
                if self.mi(ui, "Mappage principale → secondaire", "") {
                    self.doc.gradient_map(self.primary.to_array(), self.secondary.to_array());
                }
            });
            ui.separator();
            for (k, sc) in [(Kind::AutoTone, "Ctrl+Maj+L"), (Kind::AutoContrast, "Ctrl+Alt+Maj+L"), (Kind::AutoColor, "Ctrl+Maj+B")] {
                if self.mi(ui, k.title(), sc) {
                    self.start_adjust(k);
                }
            }
            ui.separator();
            if self.mi(ui, "Taille de l'image…", "Ctrl+Alt+I") {
                self.sheet = Some(Sheet::ImageSize { w: self.doc.width(), h: self.doc.height(), keep: true, percent: false, resample: 3, dpi: 72.0 });
            }
            if self.mi(ui, "Taille de la zone de travail…", "Ctrl+Alt+C") {
                self.sheet = Some(Sheet::Canvas { w: self.doc.width(), h: self.doc.height(), anchor: 4, relative: false, ext: 0 });
            }
            ui.menu_button("Rotation de l'image", |ui| {
                for (label, f) in [("180°", 0u8), ("90° horaire", 1), ("90° antihoraire", 2), ("Symétrie horizontale de la zone de travail", 3), ("Symétrie verticale de la zone de travail", 4)] {
                    if self.mi(ui, label, "") {
                        self.doc.set_selection(None, None);
                        match f {
                            0 => self.doc.transform(imageops::rotate180),
                            1 => self.doc.transform(imageops::rotate90),
                            2 => self.doc.transform(imageops::rotate270),
                            3 => self.doc.transform(imageops::flip_horizontal),
                            _ => self.doc.transform(imageops::flip_vertical),
                        }
                        self.fit_pending = true;
                    }
                }
                if ui.button("Paramétrée…").clicked() {
                    self.sheet = Some(Sheet::RotateCanvas { angle: 15.0 });
                }
            });
            if self.mie(ui, self.doc.selection.is_some(), "Recadrer", "") {
                self.doc.crop_to_selection();
                self.fit_pending = true;
            }
            if self.mi(ui, "Rogner les bords transparents", "") {
                if self.doc.trim() {
                    self.fit_pending = true;
                } else {
                    self.error = Some("Aucun bord transparent à rogner".into());
                }
            }
            if self.mi(ui, "Tout faire apparaître", "") {
                let (w, h) = (self.doc.width(), self.doc.height());
                let mut b = Area { x0: 0, y0: 0, x1: w, y1: h };
                for i in 0..self.doc.layers.len() {
                    if let Some(lb) = self.doc.layer_bounds(i) {
                        b = Area { x0: b.x0.min(lb.x0), y0: b.y0.min(lb.y0), x1: b.x1.max(lb.x1), y1: b.y1.max(lb.y1) };
                    }
                }
                self.error = Some(if (b.x0, b.y0, b.x1, b.y1) == (0, 0, w, h) { "Tout le contenu est déjà visible".into() } else { "Contenu déjà dans la zone de travail".into() });
            }
            ui.separator();
            if self.mi(ui, "Mode niveaux de gris", "") {
                self.doc.transform(|img| Kind::BlackWhite.apply(img, &[]));
            }
            if self.mi(ui, "Aplatir l'image", "") {
                self.doc.flatten_all();
            }
        });
    }

    fn menu_layer(&mut self, ui: &mut Ui) {
        ui.menu_button("Calque", |ui| {
            if self.mi(ui, "Nouveau calque", "Ctrl+Maj+N") {
                self.doc.add_layer(None, None);
            }
            ui.menu_button("Nouveau calque de remplissage", |ui| {
                let (w, h) = (self.doc.width(), self.doc.height());
                if self.mi(ui, "Couleur unie (principale)", "") {
                    self.doc.add_layer(Some("Couleur unie".into()), Some(RgbaImage::from_pixel(w, h, image::Rgba(self.primary.to_array()))));
                }
                if self.mi(ui, "Dégradé (principale → secondaire)", "") {
                    self.doc.add_layer(Some("Dégradé".into()), None);
                    self.doc.gradient_kind((0.0, 0.0), (0.0, h as f32), self.primary.to_array(), self.secondary.to_array(), false);
                }
                if self.mi(ui, "Motif damier", "") {
                    let (a, b) = (self.primary.to_array(), self.secondary.to_array());
                    let cell = (w.max(h) / 16).max(4);
                    self.doc.add_layer(Some("Motif".into()), Some(RgbaImage::from_fn(w, h, |x, y| image::Rgba(if (x / cell + y / cell) % 2 == 0 { a } else { b }))));
                }
                if self.mi(ui, "Bruit (texture)", "") {
                    self.doc.add_layer(Some("Bruit".into()), Some(RgbaImage::from_fn(w, h, |x, y| {
                        let v = (filters::hash(x, y, 5) * 255.0) as u8;
                        image::Rgba([v, v, v, 255])
                    })));
                    let a = self.doc.active;
                    self.doc.layers[a].blend = Blend::Overlay;
                    self.doc.layers[a].opacity = 0.3;
                }
            });
            ui.menu_button("Nouveau calque de réglage", |ui| {
                for k in filters::ADJUSTMENTS.iter().chain([Kind::BlackWhite, Kind::Invert, Kind::Sepia, Kind::Solarize].iter()) {
                    if ui.button(format!("{}…", k.title())).clicked() {
                        self.new_adjustment_layer(*k);
                        ui.close();
                    }
                }
            });
            if self.mi(ui, "Dupliquer le calque", "Ctrl+J") {
                self.doc.duplicate_layer();
            }
            if self.mi(ui, "Supprimer le calque", "") {
                self.doc.delete_layer();
            }
            ui.separator();
            let has_sel = self.doc.selection.is_some();
            if self.mie(ui, has_sel, "Calque par copie", "Ctrl+J") {
                self.doc.layer_from_selection(false);
            }
            if self.mie(ui, has_sel, "Calque par coupure", "Ctrl+Maj+J") {
                self.doc.layer_from_selection(true);
            }
            ui.separator();
            ui.menu_button("Masque de fusion", |ui| {
                let has = self.doc.layers[self.doc.active].mask.is_some();
                if self.mie(ui, !has, "Tout faire apparaître", "") {
                    self.doc.add_mask(0);
                }
                if self.mie(ui, !has, "Tout masquer", "") {
                    self.doc.add_mask(1);
                }
                if self.mie(ui, !has && has_sel, "Faire apparaître la sélection", "") {
                    self.doc.add_mask(2);
                }
                ui.separator();
                let a = self.doc.active;
                let on = self.doc.layers[a].mask_on;
                if self.mie(ui, has, if on { "Désactiver le masque" } else { "Activer le masque" }, "") {
                    self.doc.layers[a].mask_on = !on;
                    self.doc.touch_all();
                }
                if self.mie(ui, has, if self.doc.edit_mask { "Modifier le calque" } else { "Modifier le masque" }, "\\") {
                    self.doc.edit_mask = !self.doc.edit_mask;
                }
                if self.mie(ui, has, "Intervertir le masque", "") {
                    self.doc.invert_mask();
                }
                if self.mie(ui, has, "Sélection depuis le masque", "") {
                    self.doc.select_from_mask();
                }
                if self.mie(ui, has, "Appliquer le masque", "") {
                    self.doc.apply_mask();
                }
                if self.mie(ui, has, "Supprimer le masque", "") {
                    self.doc.delete_mask();
                }
            });
            let a = self.doc.active;
            let clip = self.doc.layers[a].clip;
            if self.mie(ui, a > 0, if clip { "Annuler le masque d'écrêtage" } else { "Créer un masque d'écrêtage" }, "Ctrl+Alt+G") {
                self.doc.layers[a].clip = !clip;
                self.doc.touch_all();
            }
            ui.menu_button("Disposition", |ui| {
                if self.mi(ui, "Premier plan", "Ctrl+Maj+]") {
                    self.doc.move_layer_to(true);
                }
                if self.mi(ui, "En avant", "Ctrl+]") {
                    self.doc.move_layer(true);
                }
                if self.mi(ui, "En arrière", "Ctrl+[") {
                    self.doc.move_layer(false);
                }
                if self.mi(ui, "Arrière-plan", "Ctrl+Maj+[") {
                    self.doc.move_layer_to(false);
                }
            });
            ui.menu_button("Verrouillage", |ui| {
                let l = &mut self.doc.layers[a];
                ui.checkbox(&mut l.lock_alpha, "Pixels transparents");
                ui.checkbox(&mut l.lock_pos, "Position");
                ui.checkbox(&mut l.locked, "Tout");
            });
            ui.menu_button("Effets de calque", |ui| {
                for (k, label) in [(0u8, "Ombre portée…"), (4, "Ombre interne…"), (1, "Lueur externe…"), (5, "Lueur interne…"), (6, "Biseau et estampage…"), (2, "Contour…"), (7, "Incrustation couleur…"), (8, "Incrustation dégradé…")] {
                    if ui.button(label).clicked() {
                        self.open_sheet_effect(k);
                        ui.close();
                    }
                }
            });
            ui.separator();
            if self.mi(ui, "Fusionner avec le dessous", "Ctrl+Alt+E") {
                self.doc.merge_down();
            }
            if self.mi(ui, "Fusionner les calques visibles", "Ctrl+Maj+E") {
                self.doc.merge_visible();
            }
            if self.mi(ui, "Aplatir l'image", "") {
                self.doc.flatten_all();
            }
        });
    }

    fn menu_select(&mut self, ui: &mut Ui) {
        ui.menu_button("Sélection", |ui| {
            if self.mi(ui, "Tout sélectionner", "Ctrl+A") {
                self.doc.set_selection(Some(self.doc.full()), None);
            }
            if self.mi(ui, "Désélectionner", "Ctrl+D") {
                self.deselect();
            }
            if self.mie(ui, self.last_sel.is_some(), "Resélectionner", "Ctrl+Maj+D") {
                if let Some(m) = self.last_sel.clone() {
                    self.doc.select_mask(m);
                }
            }
            let has = self.doc.selection.is_some();
            if self.mie(ui, has, "Intervertir", "Ctrl+Maj+I") {
                self.doc.invert_selection();
            }
            ui.separator();
            if self.mi(ui, "Pixels du calque", "") {
                self.doc.select_layer_alpha();
            }
            if ui.button("Plage de couleurs…").clicked() {
                self.sheet = Some(Sheet::ColorRange { mode: 0, fuzz: 40.0 });
            }
            ui.separator();
            ui.add_enabled_ui(has, |ui| {
                ui.menu_button("Modifier", |ui| {
                    for (label, kind, sc) in [("Bordure…", 2u8, ""), ("Lisser…", 4, ""), ("Dilater…", 0, ""), ("Contracter…", 1, ""), ("Contour progressif…", 3, "Maj+F6")] {
                        if ui.add(egui::Button::new(label).shortcut_text(RichText::new(sc).color(FAINT))).clicked() {
                            self.sheet = Some(Sheet::SelectBy { kind, px: 4.0 });
                            ui.close();
                        }
                    }
                });
                if self.mi(ui, "Étendre", "") {
                    self.doc.grow_similar(self.tolerance, true);
                }
                if self.mi(ui, "Généraliser", "") {
                    self.doc.grow_similar(self.tolerance, false);
                }
            });
            ui.separator();
            if self.mie(ui, has, "Mémoriser la sélection…", "") {
                self.sheet = Some(Sheet::SaveSelection { name: format!("Sélection {}", self.saved_sel.len() + 1) });
            }
            ui.add_enabled_ui(!self.saved_sel.is_empty(), |ui| {
                ui.menu_button("Récupérer la sélection", |ui| {
                    let mut load = None;
                    let mut del = None;
                    for (i, (name, _)) in self.saved_sel.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.button(name).clicked() {
                                load = Some(i);
                            }
                            if ui.small_button("×").on_hover_text("Supprimer").clicked() {
                                del = Some(i);
                            }
                        });
                    }
                    if let Some(i) = load {
                        let m = self.saved_sel[i].1.clone();
                        if m.len() == (self.doc.width() * self.doc.height()) as usize {
                            self.doc.select_mask(m);
                        } else {
                            self.error = Some("La taille du document a changé depuis la mémorisation".into());
                        }
                    }
                    if let Some(i) = del {
                        self.saved_sel.remove(i);
                    }
                });
            });
        });
    }

    fn menu_filter(&mut self, ui: &mut Ui) {
        ui.menu_button("Filtre", |ui| {
            let last = self.last_filter.as_ref().map(|l| l.0.title().to_string());
            if self.mie(ui, last.is_some(), &format!("Dernier filtre{}", last.map(|t| format!(" : {t}")).unwrap_or_default()), "Ctrl+Alt+F") {
                self.repeat_filter();
            }
            ui.separator();
            for (group, kinds) in filters::FILTERS {
                ui.menu_button(*group, |ui| {
                    for k in *kinds {
                        let label = if k.params().is_empty() { k.title().to_string() } else { format!("{}…", k.title()) };
                        if ui.button(label).clicked() {
                            self.start_adjust(*k);
                            ui.close();
                        }
                    }
                });
            }
        });
    }

    fn menu_view(&mut self, ui: &mut Ui) {
        ui.menu_button("Affichage", |ui| {
            if self.mi(ui, "Zoom avant", "Ctrl++") {
                self.zoom = (self.zoom * 1.25).min(64.0);
            }
            if self.mi(ui, "Zoom arrière", "Ctrl+−") {
                self.zoom = (self.zoom / 1.25).max(0.02);
            }
            if self.mi(ui, "Ajuster à l'écran", "Ctrl+0") {
                self.fit_pending = true;
            }
            if self.mi(ui, "Taille réelle (100 %)", "Ctrl+1") {
                self.zoom = 1.0;
                self.offset = Vec2::ZERO;
            }
            ui.menu_button("Zoom prédéfini", |ui| {
                for z in [0.25, 0.5, 2.0, 3.0, 4.0, 8.0, 16.0] {
                    if ui.button(format!("{:.0} %", z * 100.0)).clicked() {
                        self.zoom = z;
                    }
                }
            });
            ui.menu_button("Rotation de la vue", |ui| {
                for a in [-90.0f32, -45.0, -15.0, 0.0, 15.0, 45.0, 90.0, 180.0] {
                    if ui.button(if a == 0.0 { "Réinitialiser".to_string() } else { format!("{a:+.0}°") }).clicked() {
                        self.view_angle = a;
                    }
                }
            });
            ui.separator();
            ui.add(egui::Checkbox::new(&mut self.extras, "Extras (contours de sélection, repères)"));
            ui.checkbox(&mut self.rulers, "Règles (Ctrl+R)");
            ui.checkbox(&mut self.show_grid, "Grille (Ctrl+')");
            ui.checkbox(&mut self.pixel_grid, "Grille de pixels (à partir de 1000 %)");
            ui.checkbox(&mut self.show_guides, "Repères (Ctrl+;)");
            ui.checkbox(&mut self.lock_guides, "Verrouiller les repères (Ctrl+Alt+;)");
            ui.checkbox(&mut self.snap, "Magnétisme (Ctrl+Maj+;)");
            ui.separator();
            if ui.button("Nouveau repère…").clicked() {
                self.sheet = Some(Sheet::Guide { vertical: true, pos: self.doc.width() as f32 / 2.0 });
            }
            if ui.button("Repères au centre").clicked() {
                self.guides.push((true, self.doc.width() as f32 / 2.0));
                self.guides.push((false, self.doc.height() as f32 / 2.0));
                self.show_guides = true;
            }
            if ui.button("Repères aux tiers").clicked() {
                for k in [1.0, 2.0] {
                    self.guides.push((true, self.doc.width() as f32 * k / 3.0));
                    self.guides.push((false, self.doc.height() as f32 * k / 3.0));
                }
                self.show_guides = true;
            }
            if ui.add_enabled(!self.guides.is_empty(), egui::Button::new("Effacer les repères")).clicked() {
                self.guides.clear();
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Pas de la grille");
                ui.add(egui::DragValue::new(&mut self.grid).range(2..=1000).suffix(" px"));
            });
            ui.horizontal(|ui| {
                ui.label("Subdivisions");
                ui.add(egui::DragValue::new(&mut self.grid_sub).range(1..=10));
            });
            ui.horizontal(|ui| {
                ui.label("Couleur");
                tools::combo(ui, "gridcol", &mut self.grid_color, &GRID_COLORS.map(|c| c.0), 110.0);
            });
            ui.separator();
            if ui.button(if self.hide_panels { "Afficher les panneaux (Tab)" } else { "Masquer les panneaux (Tab)" }).clicked() {
                self.hide_panels = !self.hide_panels;
            }
        });
    }

    fn home_screen(&mut self, root: &mut Ui) {
        egui::Panel::left("home-nav").exact_size(230.0).resizable(false).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(18, 26))).show(root, |ui| {
            if pro::primary(ui, "Nouveau fichier").clicked() {
                self.new_doc_sheet();
            }
            ui.add_space(8.0);
            if pro::secondary(ui, "Ouvrir…").clicked() {
                self.open_dialog();
            }
            ui.add_space(8.0);
            if pro::secondary(ui, "Depuis le presse-papiers").clicked() {
                self.doc_from_clipboard();
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
                        ui.label(RichText::new("Bienvenue dans Pixel").font(semibold(28.0)).color(INK_STRONG));
                        ui.label(RichText::new("Retouche photo et compositing par calques").color(MUTED));
                    });
                });
                ui.add_space(28.0);
                ui.label(RichText::new("Créer un document").font(semibold(16.0)).color(INK));
                ui.add_space(12.0);
                let gap = 14.0;
                let card_w = 168.0;
                let cols = (((ui.available_width() + gap) / (card_w + gap)).floor() as usize).max(1);
                for row in PRESETS.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for (name, kind, w, h) in row {
                            if preset_card(ui, name, &format!("{w} × {h} · {kind}"), *w, *h, false, card_w).clicked() {
                                self.set_doc(Document::new(*w, *h, Some([255, 255, 255, 255])));
                            }
                        }
                    });
                    ui.add_space(gap);
                }
                ui.add_space(14.0);
                ui.label(RichText::new("Récents").font(semibold(16.0)).color(INK));
                ui.add_space(10.0);
                if self.recents.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().min(760.0), 120.0), Sense::hover());
                    let p = ui.painter();
                    p.rect(r, CornerRadius::same(14), Color32::from_white_alpha(4), Stroke::new(1.0, LINE), StrokeKind::Inside);
                    p.text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "Dépose une image n'importe où pour l'ouvrir", semibold(15.0), INK);
                    p.text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "PNG, JPEG, WebP, BMP, TIFF, GIF ou ICO", FontId::proportional(13.0), MUTED);
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

    fn status_bar(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                ui.label(small(format!("{:.0} %", self.zoom * 100.0)));
                if self.view_angle != 0.0 {
                    ui.label(small(format!("vue {:+.0}°", self.view_angle)));
                }
                ui.label(small(format!("{} × {} px", self.doc.width(), self.doc.height())));
                if let Some(s) = self.doc.selection {
                    ui.label(small(format!("Sélection {} × {}", s.width(), s.height())));
                }
                if let Some((x, y)) = self.cursor {
                    ui.label(small(format!("x {:.0}  y {:.0}", x.floor(), y.floor())));
                }
                if self.doc.edit_mask && self.doc.layers[self.doc.active].mask.is_some() {
                    ui.label(RichText::new("Masque en cours de modification").size(12.0).color(BLUE));
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
}

pub const GRID_COLORS: [(&str, [u8; 3]); 5] = [("Bleu", [10, 132, 255]), ("Gris", [150, 150, 150]), ("Rouge", [255, 69, 58]), ("Vert", [48, 209, 88]), ("Magenta", [255, 0, 200])];
pub const PASTEBOARDS: [(&str, Color32); 4] = [("Sombre", PASTEBOARD), ("Gris moyen", Color32::from_rgb(0x3a, 0x3a, 0x3f)), ("Clair", Color32::from_rgb(0xb8, 0xb8, 0xbd)), ("Noir", Color32::BLACK)];

fn preset_card(ui: &mut Ui, name: &str, detail: &str, w: u32, h: u32, selected: bool, width: f32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 132.0), Sense::click());
    let hover = resp.hovered();
    let p = ui.painter();
    p.rect(r, CornerRadius::same(12), if hover { Color32::from_rgb(0x2f, 0x2f, 0x35) } else { Color32::from_rgb(0x27, 0x27, 0x2c) }, Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { BLUE } else { LINE }), StrokeKind::Inside);
    let area = Rect::from_center_size(r.center_top() + vec2(0.0, 44.0), vec2(width - 40.0, 56.0));
    let s = (area.width() / w as f32).min(area.height() / h as f32);
    let shape = Rect::from_center_size(area.center(), vec2(w as f32, h as f32) * s);
    p.rect(shape, CornerRadius::same(3), Color32::from_white_alpha(18), Stroke::new(1.2, if selected { BLUE } else { MUTED }), StrokeKind::Inside);
    p.text(pos2(r.center().x, r.bottom() - 36.0), Align2::CENTER_CENTER, name, semibold(13.0), INK_STRONG);
    p.text(pos2(r.center().x, r.bottom() - 17.0), Align2::CENTER_CENTER, detail, FontId::proportional(11.0), MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn export_glyph(p: &Painter, r: Rect, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    p.line_segment([at(0.5, 0.05), at(0.5, 0.65)], Stroke::new(2.0, c));
    p.line_segment([at(0.5, 0.05), at(0.25, 0.3)], Stroke::new(2.0, c));
    p.line_segment([at(0.5, 0.05), at(0.75, 0.3)], Stroke::new(2.0, c));
    p.add(Shape::line(vec![at(0.12, 0.55), at(0.12, 0.95), at(0.88, 0.95), at(0.88, 0.55)], Stroke::new(2.0, c)));
}

pub fn lock_glyph(p: &Painter, r: Rect, locked: bool, c: Color32) {
    let body = Rect::from_min_max(r.min + vec2(4.0, 9.0), r.max - vec2(4.0, 2.0));
    p.rect_filled(body, CornerRadius::same(2), c);
    let lift = if locked { 0.0 } else { 4.0 };
    p.add(Shape::line(
        vec![pos2(body.left() + 2.0, body.top()), pos2(body.left() + 2.0, r.top() + 5.0 - lift), pos2(r.center().x, r.top() + 2.0 - lift), pos2(body.right() - 2.0, r.top() + 5.0 - lift), pos2(body.right() - 2.0, body.top() - lift)],
        Stroke::new(1.6, c),
    ));
}

fn image_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "gif", "ico"])
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Pixel").with_icon(Arc::new(studio_ui::app_window_icon("pixel"))).with_inner_size([1440.0, 900.0]).with_min_inner_size([1000.0, 640.0]).with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Pixel", options, Box::new(|cc| Ok(Box::new(Pixel::new(cc)))))
}
