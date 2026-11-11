#![windows_subsystem = "windows"]

mod doc;

use doc::{Area, Brush, Document, hsl_to_rgb, rgb_to_hsl};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, Painter, Pos2,
    Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2, pos2,
    vec2,
};
use image::{RgbaImage, imageops};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Move,
    Marquee,
    Brush,
    Eraser,
    Fill,
    Gradient,
    Picker,
    Hand,
    Zoom,
}

const TOOLS: [(Tool, &str, Key); 9] = [
    (Tool::Move, "Déplacement (V)", Key::V),
    (Tool::Marquee, "Sélection rectangulaire (M)", Key::M),
    (Tool::Brush, "Pinceau (B)", Key::B),
    (Tool::Eraser, "Gomme (E)", Key::E),
    (Tool::Fill, "Pot de peinture (K)", Key::K),
    (Tool::Gradient, "Dégradé (G)", Key::G),
    (Tool::Picker, "Pipette (I)", Key::I),
    (Tool::Hand, "Main (H, ou Espace)", Key::H),
    (Tool::Zoom, "Zoom (Z, Alt pour dézoomer)", Key::Z),
];

fn tool_name(t: Tool) -> &'static str {
    TOOLS.iter().find(|x| x.0 == t).map_or("", |x| x.1.split(" (").next().unwrap_or(""))
}

#[derive(Clone, Copy, PartialEq)]
enum Adjust {
    BrightnessContrast { b: f32, c: f32 },
    HueSat { h: f32, s: f32, l: f32 },
    Blur { sigma: f32 },
}

impl Adjust {
    fn title(&self) -> &'static str {
        match self {
            Adjust::BrightnessContrast { .. } => "Luminosité et contraste",
            Adjust::HueSat { .. } => "Teinte et saturation",
            Adjust::Blur { .. } => "Flou gaussien",
        }
    }

    fn apply(&self, doc: &mut Document, checkpoint: bool) {
        match *self {
            Adjust::BrightnessContrast { b, c } => {
                let b = b / 100.0;
                let k = ((c / 100.0) + 1.0).powi(2);
                let f = move |p: &mut [u8; 4]| {
                    for v in &mut p[..3] {
                        let x = *v as f32 / 255.0 + b;
                        *v = (((x - 0.5) * k + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                };
                if checkpoint { doc.adjust(f) } else { doc.adjust_raw(f) }
            }
            Adjust::HueSat { h, s, l } => {
                let f = move |p: &mut [u8; 4]| {
                    let (hh, ss, ll) = rgb_to_hsl(p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0);
                    let ss = (ss * (1.0 + s / 100.0)).clamp(0.0, 1.0);
                    let ll = if l >= 0.0 { ll + (1.0 - ll) * l / 100.0 } else { ll * (1.0 + l / 100.0) };
                    let (r, g, b) = hsl_to_rgb(hh + h / 360.0, ss, ll);
                    p[0] = (r * 255.0).round() as u8;
                    p[1] = (g * 255.0).round() as u8;
                    p[2] = (b * 255.0).round() as u8;
                };
                if checkpoint { doc.adjust(f) } else { doc.adjust_raw(f) }
            }
            Adjust::Blur { sigma } => {
                if checkpoint { doc.blur(sigma) } else { doc.blur_raw(sigma) }
            }
        }
    }
}

struct NewDoc {
    name: String,
    w: u32,
    h: u32,
    background: usize,
    preset: usize,
}

const PRESETS: [(&str, &str, u32, u32); 8] = [
    ("Full HD", "Écran 16:9", 1920, 1080),
    ("4K UHD", "Écran 16:9", 3840, 2160),
    ("Miniature YouTube", "Vidéo", 1280, 720),
    ("Publication carrée", "Réseaux sociaux", 1080, 1080),
    ("Story", "Réseaux sociaux", 1080, 1920),
    ("A4 300 ppp", "Impression", 2480, 3508),
    ("Icône", "Interface", 1024, 1024),
    ("Bannière", "Web", 1500, 500),
];

const BACKGROUNDS: [&str; 4] = ["Blanc", "Transparent", "Noir", "Couleur principale"];

enum Drag {
    None,
    Marquee((f32, f32)),
    Move { from: (f32, f32), original: RgbaImage },
    Gradient((f32, f32)),
}

struct Pixel {
    doc: Document,
    home: bool,
    recents: Vec<PathBuf>,
    composite: RgbaImage,
    tex: Option<TextureHandle>,
    tex_filter: TextureOptions,
    checker: TextureHandle,
    icon: TextureHandle,
    thumbs: Vec<TextureHandle>,
    thumbs_version: u64,
    tool: Tool,
    brush: Brush,
    primary: Color32,
    secondary: Color32,
    swatches: Vec<Color32>,
    tolerance: u8,
    zoom: f32,
    offset: Vec2,
    fit_pending: bool,
    last: Option<(f32, f32)>,
    cursor: Option<(f32, f32)>,
    drag: Drag,
    gradient_end: Option<(f32, f32)>,
    new_doc: Option<NewDoc>,
    adjust: Option<(Adjust, RgbaImage)>,
    resize: Option<(u32, u32, bool)>,
    renaming: Option<(usize, String)>,
    error: Option<String>,
    title: String,
}

fn recents_path() -> PathBuf {
    studio_core::data_dir().join("pixel").join("recents.txt")
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
            cc.egui_ctx.load_texture(
                "checker",
                egui::ColorImage::from_rgba_unmultiplied([16, 16], &px),
                TextureOptions { wrap_mode: egui::TextureWrapMode::Repeat, ..TextureOptions::NEAREST },
            )
        };
        let recents = std::fs::read_to_string(recents_path())
            .map(|s| s.lines().map(PathBuf::from).filter(|p| p.exists()).collect())
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
            brush: Brush { radius: 12.0, hardness: 0.8, flow: 1.0 },
            primary: Color32::from_rgb(20, 20, 24),
            secondary: Color32::WHITE,
            swatches: vec![
                Color32::from_rgb(20, 20, 24),
                Color32::WHITE,
                Color32::from_rgb(255, 69, 58),
                Color32::from_rgb(255, 159, 10),
                Color32::from_rgb(255, 214, 10),
                Color32::from_rgb(48, 209, 88),
                Color32::from_rgb(10, 132, 255),
                Color32::from_rgb(191, 90, 242),
            ],
            tolerance: 32,
            zoom: 1.0,
            offset: Vec2::ZERO,
            fit_pending: true,
            last: None,
            cursor: None,
            drag: Drag::None,
            gradient_end: None,
            new_doc: None,
            adjust: None,
            resize: None,
            renaming: None,
            error: None,
            title: String::new(),
        };
        if let Some(path) = std::env::args().nth(1) {
            me.open_path(Path::new(&path));
        }
        me
    }

    fn set_doc(&mut self, doc: Document) {
        self.doc = doc;
        self.tex = None;
        self.fit_pending = true;
        self.renaming = None;
        self.home = false;
        self.thumbs_version = u64::MAX;
    }

    fn remember(&mut self, path: &Path) {
        self.recents.retain(|p| p != path);
        self.recents.insert(0, path.to_path_buf());
        self.recents.truncate(8);
        let file = recents_path();
        let _ = std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")));
        let text: Vec<String> = self.recents.iter().map(|p| p.display().to_string()).collect();
        let _ = std::fs::write(file, text.join("\n"));
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
                    self.doc.add_layer(name, Some(img.into_rgba8()));
                }
                Err(e) => self.error = Some(format!("Impossible d'importer : {e}")),
            }
        }
    }

    fn export(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{}.png", self.doc.name))
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg", "jpeg"])
            .add_filter("WebP", &["webp"])
            .save_file()
        else {
            return;
        };
        let flat = self.doc.flatten();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let result = if ext == "jpg" || ext == "jpeg" { image::DynamicImage::ImageRgba8(flat).to_rgb8().save(&path) } else { flat.save(&path) };
        match result {
            Ok(()) => {
                self.doc.modified = false;
                self.remember(&path);
            }
            Err(e) => self.error = Some(format!("Export impossible : {e}")),
        }
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
        self.thumbs = self
            .doc
            .layers
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let t = imageops::thumbnail(&l.img, tw, th);
                ctx.load_texture(format!("thumb{i}"), egui::ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], t.as_raw()), TextureOptions::LINEAR)
            })
            .collect();
        self.thumbs_version = self.doc.version;
    }

    fn new_doc_sheet(&mut self) {
        self.new_doc = Some(NewDoc { name: "Sans titre".into(), w: 1920, h: 1080, background: 0, preset: 0 });
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() || self.new_doc.is_some() || self.adjust.is_some() {
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        if sc(M::COMMAND, Key::N) {
            self.new_doc_sheet();
        }
        if sc(M::COMMAND, Key::O) {
            self.open_dialog();
        }
        if self.home {
            return;
        }
        if sc(M::COMMAND | M::SHIFT, Key::Z) || sc(M::COMMAND, Key::Y) {
            self.doc.redo();
        } else if sc(M::COMMAND, Key::Z) {
            self.doc.undo();
        }
        if sc(M::COMMAND, Key::Num0) {
            self.fit_pending = true;
        }
        if sc(M::COMMAND, Key::Num1) {
            self.zoom = 1.0;
            self.offset = Vec2::ZERO;
        }
        if sc(M::COMMAND | M::SHIFT, Key::S) || sc(M::COMMAND, Key::E) {
            self.export();
        }
        if sc(M::COMMAND, Key::A) {
            self.doc.selection = Some(self.doc.full());
        }
        if sc(M::COMMAND, Key::D) {
            self.doc.selection = None;
        }
        if sc(M::COMMAND, Key::J) {
            self.doc.duplicate_layer();
        }
        if sc(M::COMMAND | M::SHIFT, Key::N) {
            self.doc.add_layer(None, None);
        }
        if sc(M::COMMAND, Key::Plus) || sc(M::COMMAND, Key::Equals) {
            self.zoom = (self.zoom * 1.25).min(64.0);
        }
        if sc(M::COMMAND, Key::Minus) {
            self.zoom = (self.zoom / 1.25).max(0.02);
        }
        let (keys, del, swap, reset, smaller, bigger) = ctx.input(|i| {
            let mut pick = None;
            if !i.modifiers.command {
                for (tool, _, key) in TOOLS {
                    if i.key_pressed(key) {
                        pick = Some(tool);
                    }
                }
            }
            (
                pick,
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::X) && !i.modifiers.command,
                i.key_pressed(Key::D) && !i.modifiers.command,
                i.key_pressed(Key::OpenBracket),
                i.key_pressed(Key::CloseBracket),
            )
        });
        if let Some(t) = keys {
            self.tool = t;
        }
        if del {
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
            self.brush.radius = (self.brush.radius / 1.2).max(0.5);
        }
        if bigger {
            self.brush.radius = (self.brush.radius * 1.2).min(500.0);
        }
    }
}

impl eframe::App for Pixel {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        self.shortcuts(ctx);
        if let Some(f) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty())) {
            self.open_path(&f);
        }
        let title = if self.home {
            "Pixel".to_string()
        } else {
            format!("{}{} — Pixel", self.doc.name, if self.doc.modified { " •" } else { "" })
        };
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
            self.dock(root, ctx);
            self.sync_texture(ctx);
            self.canvas(root);
        }
        self.sheets(ctx);
    }
}

impl Pixel {
    fn menu_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(34.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(10, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add(egui::Image::new((self.icon.id(), Vec2::splat(20.0))));
                ui.add_space(6.0);
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouveau…                Ctrl+N").clicked() {
                            self.new_doc_sheet();
                        }
                        if ui.button("Ouvrir…                   Ctrl+O").clicked() {
                            self.open_dialog();
                        }
                        ui.add_enabled_ui(!self.home, |ui| {
                            if ui.button("Importer comme calque…").clicked() {
                                self.import_layer();
                            }
                            ui.separator();
                            if ui.button("Exporter…                Ctrl+E").clicked() {
                                self.export();
                            }
                            ui.separator();
                            if ui.button("Fermer").clicked() {
                                self.home = true;
                            }
                        });
                    });
                    ui.add_enabled_ui(!self.home, |ui| {
                        ui.menu_button("Édition", |ui| {
                            if ui.add_enabled(self.doc.can_undo(), egui::Button::new("Annuler                    Ctrl+Z")).clicked() {
                                self.doc.undo();
                            }
                            if ui.add_enabled(self.doc.can_redo(), egui::Button::new("Rétablir                   Ctrl+Y")).clicked() {
                                self.doc.redo();
                            }
                            ui.separator();
                            if ui.add_enabled(self.doc.selection.is_some(), egui::Button::new("Effacer la sélection   Suppr")).clicked() {
                                self.doc.clear_selection();
                            }
                        });
                        ui.menu_button("Image", |ui| {
                            ui.menu_button("Réglages", |ui| {
                                if ui.button("Luminosité et contraste…").clicked() {
                                    self.start_adjust(Adjust::BrightnessContrast { b: 0.0, c: 0.0 });
                                }
                                if ui.button("Teinte et saturation…").clicked() {
                                    self.start_adjust(Adjust::HueSat { h: 0.0, s: 0.0, l: 0.0 });
                                }
                                ui.separator();
                                if ui.button("Noir et blanc").clicked() {
                                    self.doc.adjust(|p| {
                                        let v = (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32).round() as u8;
                                        p[0] = v;
                                        p[1] = v;
                                        p[2] = v;
                                    });
                                }
                                if ui.button("Négatif").clicked() {
                                    self.doc.adjust(|p| {
                                        p[0] = 255 - p[0];
                                        p[1] = 255 - p[1];
                                        p[2] = 255 - p[2];
                                    });
                                }
                            });
                            ui.separator();
                            if ui.button("Taille de l'image…").clicked() {
                                self.resize = Some((self.doc.width(), self.doc.height(), true));
                            }
                            if ui.add_enabled(self.doc.selection.is_some(), egui::Button::new("Recadrer sur la sélection")).clicked() {
                                self.doc.crop_to_selection();
                                self.fit_pending = true;
                            }
                            ui.separator();
                            if ui.button("Rotation 90° horaire").clicked() {
                                self.doc.selection = None;
                                self.doc.transform(imageops::rotate90);
                                self.fit_pending = true;
                            }
                            if ui.button("Rotation 90° antihoraire").clicked() {
                                self.doc.selection = None;
                                self.doc.transform(imageops::rotate270);
                                self.fit_pending = true;
                            }
                            if ui.button("Symétrie horizontale").clicked() {
                                self.doc.transform(imageops::flip_horizontal);
                            }
                            if ui.button("Symétrie verticale").clicked() {
                                self.doc.transform(imageops::flip_vertical);
                            }
                        });
                        ui.menu_button("Calque", |ui| {
                            if ui.button("Nouveau calque            Ctrl+Maj+N").clicked() {
                                self.doc.add_layer(None, None);
                            }
                            if ui.button("Dupliquer le calque       Ctrl+J").clicked() {
                                self.doc.duplicate_layer();
                            }
                            if ui.button("Fusionner avec le dessous").clicked() {
                                self.doc.merge_down();
                            }
                            if ui.button("Supprimer le calque").clicked() {
                                self.doc.delete_layer();
                            }
                        });
                        ui.menu_button("Sélection", |ui| {
                            if ui.button("Tout sélectionner         Ctrl+A").clicked() {
                                self.doc.selection = Some(self.doc.full());
                            }
                            if ui.button("Désélectionner            Ctrl+D").clicked() {
                                self.doc.selection = None;
                            }
                        });
                        ui.menu_button("Filtre", |ui| {
                            if ui.button("Flou gaussien…").clicked() {
                                self.start_adjust(Adjust::Blur { sigma: 3.0 });
                            }
                        });
                        ui.menu_button("Affichage", |ui| {
                            if ui.button("Ajuster à l'écran         Ctrl+0").clicked() {
                                self.fit_pending = true;
                            }
                            if ui.button("Taille réelle (100 %)   Ctrl+1").clicked() {
                                self.zoom = 1.0;
                                self.offset = Vec2::ZERO;
                            }
                        });
                    });
                });
            });
        });
    }

    fn start_adjust(&mut self, a: Adjust) {
        let base = self.doc.layers[self.doc.active].img.clone();
        self.adjust = Some((a, base));
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
                    p.text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "PNG, JPEG, WebP ou BMP", FontId::proportional(13.0), MUTED);
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
                ui.add_space(8.0);
                let (sep, _) = ui.allocate_exact_size(vec2(1.0, 22.0), Sense::hover());
                ui.painter().rect_filled(sep, CornerRadius::ZERO, LINE);
                ui.add_space(8.0);
                match self.tool {
                    Tool::Brush | Tool::Eraser => {
                        pro::option_slider(ui, "Taille", &mut self.brush.radius, 0.5..=300.0, " px", true);
                        pro::option_slider(ui, "Dureté", &mut self.brush.hardness, 0.0..=1.0, "", false);
                        pro::option_slider(ui, "Flux", &mut self.brush.flow, 0.01..=1.0, "", false);
                    }
                    Tool::Fill => {
                        pro::option_slider(ui, "Tolérance", &mut self.tolerance, 0..=255, "", false);
                    }
                    Tool::Gradient => {
                        ui.label(RichText::new("Glisse sur l'image : de la couleur principale à la couleur secondaire").size(12.5).color(MUTED));
                    }
                    Tool::Marquee => {
                        if pro::secondary(ui, "Tout sélectionner").clicked() {
                            self.doc.selection = Some(self.doc.full());
                        }
                        if self.doc.selection.is_some() {
                            if pro::secondary(ui, "Désélectionner").clicked() {
                                self.doc.selection = None;
                            }
                            if pro::secondary(ui, "Recadrer").clicked() {
                                self.doc.crop_to_selection();
                                self.fit_pending = true;
                            }
                        }
                    }
                    Tool::Move => {
                        ui.label(RichText::new("Glisse pour déplacer le calque actif").size(12.5).color(MUTED));
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
                    Tool::Picker => {
                        ui.label(RichText::new("Clique sur l'image pour prendre une couleur").size(12.5).color(MUTED));
                    }
                }
            });
        });
    }

    fn status_bar(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                ui.label(small(format!("{:.0} %", self.zoom * 100.0)));
                ui.label(small(format!("{} × {} px", self.doc.width(), self.doc.height())));
                if let Some(s) = self.doc.selection {
                    ui.label(small(format!("Sélection {} × {}", s.width(), s.height())));
                }
                if let Some((x, y)) = self.cursor {
                    ui.label(small(format!("x {:.0}  y {:.0}", x.floor(), y.floor())));
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
                if matches!(i, 2 | 6) {
                    let (sep, _) = ui.allocate_exact_size(vec2(38.0, 9.0), Sense::hover());
                    ui.painter().line_segment([sep.left_center() + vec2(6.0, 0.0), sep.right_center() - vec2(6.0, 0.0)], Stroke::new(1.0, LINE));
                }
                let t = *tool;
                if pro::icon_button(ui, 38.0, self.tool == t, tip, |p, r, c| tool_icon(p, r, t, c)).clicked() {
                    self.tool = t;
                }
            }
            ui.add_space(12.0);
            let (rect, resp) = ui.allocate_exact_size(vec2(38.0, 44.0), Sense::click());
            let back = Rect::from_min_size(rect.min + vec2(14.0, 16.0), Vec2::splat(20.0));
            let front = Rect::from_min_size(rect.min + vec2(4.0, 6.0), Vec2::splat(20.0));
            ui.painter().rect(back, CornerRadius::same(5), self.secondary, Stroke::new(1.0, MUTED), StrokeKind::Outside);
            ui.painter().rect(front, CornerRadius::same(5), self.primary, Stroke::new(1.5, INK), StrokeKind::Outside);
            if resp.on_hover_text("Couleurs principale et secondaire (X pour inverser, D par défaut)").clicked() {
                std::mem::swap(&mut self.primary, &mut self.secondary);
            }
        });
    }

    fn dock(&mut self, root: &mut Ui, ctx: &egui::Context) {
        self.sync_thumbs(ctx);
        egui::Panel::right("dock").resizable(false).exact_size(296.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                pro::dock_panel(ui, "color", "Couleur", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let w = ui.available_width();
                    pro::color_picker(ui, &mut self.primary, w);
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for c in self.swatches.clone() {
                            let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
                            ui.painter().rect(r, CornerRadius::same(6), c, Stroke::new(1.0, LINE), StrokeKind::Inside);
                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                self.primary = c;
                            }
                        }
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
                        ui.painter().rect(r, CornerRadius::same(6), FIELD, Stroke::NONE, StrokeKind::Inside);
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, "+", FontId::proportional(16.0), INK);
                        if resp.on_hover_text("Ajouter la couleur principale").clicked() && !self.swatches.contains(&self.primary) {
                            self.swatches.push(self.primary);
                        }
                    });
                });
                pro::dock_panel(ui, "props", "Propriétés", |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    pro::label_value(ui, "Document", &format!("{} × {} px", self.doc.width(), self.doc.height()));
                    pro::label_value(ui, "Calques", &self.doc.layers.len().to_string());
                    if let Some(s) = self.doc.selection {
                        pro::label_value(ui, "Sélection", &format!("{} × {} à ({}, {})", s.width(), s.height(), s.x0, s.y0));
                    }
                    ui.add_space(6.0);
                    let mut op = self.doc.layers[self.doc.active].opacity * 100.0;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Opacité du calque").size(12.5).color(MUTED));
                    });
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).suffix(" %").trailing_fill(true)).changed() {
                        self.doc.layers[self.doc.active].opacity = op / 100.0;
                        self.doc.touch_all();
                        self.doc.modified = true;
                    }
                });
                pro::dock_panel(ui, "layers", "Calques", |ui| self.layers_panel(ui));
            });
        });
    }

    fn layers_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 4.0;
        let active = self.doc.active;
        let mut visibility = None;
        let mut select = None;
        for i in (0..self.doc.layers.len()).rev() {
            let selected = i == active;
            let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
            let p = ui.painter();
            if selected {
                p.rect_filled(row, CornerRadius::same(9), BLUE.gamma_multiply(0.24));
            } else if resp.hovered() {
                p.rect_filled(row, CornerRadius::same(9), Color32::from_white_alpha(8));
            }
            let eye = Rect::from_center_size(row.left_center() + vec2(16.0, 0.0), Vec2::splat(20.0));
            let visible = self.doc.layers[i].visible;
            let ec = if visible { INK } else { FAINT };
            p.add(Shape::convex_polygon(
                vec![eye.left_center() + vec2(1.0, 0.0), eye.center() - vec2(0.0, 5.0), eye.right_center() - vec2(1.0, 0.0), eye.center() + vec2(0.0, 5.0)],
                Color32::TRANSPARENT,
                Stroke::new(1.3, ec),
            ));
            if visible {
                p.circle_filled(eye.center(), 2.5, ec);
            } else {
                p.line_segment([eye.left_top() + vec2(3.0, 3.0), eye.right_bottom() - vec2(3.0, 3.0)], Stroke::new(1.3, ec));
            }
            let thumb = Rect::from_min_size(pos2(row.left() + 34.0, row.top() + 6.0), Vec2::splat(40.0));
            p.image(self.checker.id(), thumb, Rect::from_min_max(pos2(0.0, 0.0), pos2(2.5, 2.5)), Color32::WHITE);
            if let Some(t) = self.thumbs.get(i) {
                let [tw, th] = t.size();
                let s = (40.0 / tw as f32).min(40.0 / th as f32);
                let fit = Rect::from_center_size(thumb.center(), vec2(tw as f32, th as f32) * s);
                p.image(t.id(), fit, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            p.rect_stroke(thumb, CornerRadius::same(3), Stroke::new(1.0, if selected { BLUE } else { LINE }), StrokeKind::Outside);
            let eye_resp = ui.interact(eye.expand(4.0), ui.make_persistent_id(("eye", i)), Sense::click());
            if eye_resp.on_hover_text("Afficher ou masquer").clicked() {
                visibility = Some(i);
            }
            let name_pos = pos2(thumb.right() + 10.0, row.center().y);
            if let Some((ri, text)) = &mut self.renaming
                && *ri == i
            {
                let edit_rect = Rect::from_min_max(name_pos - vec2(0.0, 12.0), pos2(row.right() - 8.0, name_pos.y + 12.0));
                let r = ui.put(edit_rect, egui::TextEdit::singleline(text));
                if r.lost_focus() {
                    let name = text.trim().to_string();
                    if !name.is_empty() {
                        self.doc.layers[i].name = name;
                    }
                    self.renaming = None;
                } else {
                    r.request_focus();
                }
            } else {
                let l = &self.doc.layers[i];
                ui.painter().text(name_pos - vec2(0.0, 7.0), Align2::LEFT_CENTER, &l.name, if selected { semibold(13.5) } else { FontId::proportional(13.5) }, if selected { INK_STRONG } else { INK });
                ui.painter().text(name_pos + vec2(0.0, 10.0), Align2::LEFT_CENTER, format!("{:.0} %", l.opacity * 100.0), FontId::proportional(11.5), FAINT);
            }
            if resp.double_clicked() {
                self.renaming = Some((i, self.doc.layers[i].name.clone()));
            } else if resp.clicked() {
                select = Some(i);
            }
        }
        if let Some(i) = visibility {
            self.doc.layers[i].visible = !self.doc.layers[i].visible;
            self.doc.touch_all();
        }
        if let Some(i) = select {
            self.doc.active = i;
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let b = |ui: &mut Ui, tip: &str, glyph: &str| {
                let (r, resp) = ui.allocate_exact_size(vec2(36.0, 28.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(7), Color32::from_white_alpha(14));
                }
                ui.painter().text(r.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(15.0), INK);
                resp.on_hover_text(tip).clicked()
            };
            if b(ui, "Nouveau calque", "+") {
                self.doc.add_layer(None, None);
            }
            if b(ui, "Dupliquer", "⧉") {
                self.doc.duplicate_layer();
            }
            if b(ui, "Monter", "↑") {
                self.doc.move_layer(true);
            }
            if b(ui, "Descendre", "↓") {
                self.doc.move_layer(false);
            }
            if b(ui, "Fusionner avec le dessous", "⤓") {
                self.doc.merge_down();
            }
            if b(ui, "Supprimer", "🗑") {
                self.doc.delete_layer();
            }
        });
    }

    fn canvas(&mut self, root: &mut Ui) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let tab_h = 32.0;
            let tab_bar = Rect::from_min_max(full.min, pos2(full.right(), full.top() + tab_h));
            let p = ui.painter();
            p.rect_filled(tab_bar, CornerRadius::ZERO, PANEL);
            let label = format!("{} @ {:.0} %{}", self.doc.name, self.zoom * 100.0, if self.doc.modified { " •" } else { "" });
            let g = p.layout_no_wrap(label, FontId::proportional(12.5), INK);
            let tab = Rect::from_min_size(tab_bar.min + vec2(8.0, 4.0), vec2(g.size().x + 28.0, tab_h - 4.0));
            p.rect_filled(tab, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, PASTEBOARD);
            p.galley(tab.left_center() + vec2(14.0, -g.size().y / 2.0), g, INK);

            let area = Rect::from_min_max(pos2(full.left(), tab_bar.bottom()), full.max);
            let resp = ui.allocate_rect(area, Sense::click_and_drag());
            let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);
            if self.fit_pending {
                self.zoom = ((area.width() - 80.0) / w).min((area.height() - 80.0) / h).clamp(0.02, 1.0);
                self.offset = Vec2::ZERO;
                self.fit_pending = false;
            }
            let canvas_min = |zoom: f32, offset: Vec2| area.center() + offset - vec2(w, h) * zoom / 2.0;
            let (scroll, pinch, space, pointer, primary_down, middle_down, delta, alt, pressed, released) = ui.input(|i| {
                (
                    i.smooth_scroll_delta.y,
                    i.zoom_delta(),
                    i.key_down(Key::Space),
                    i.pointer.hover_pos(),
                    i.pointer.primary_down(),
                    i.pointer.middle_down(),
                    i.pointer.delta(),
                    i.modifiers.alt,
                    i.pointer.primary_pressed(),
                    i.pointer.primary_released(),
                )
            });
            let zoom_at = |zoom: &mut f32, offset: &mut Vec2, factor: f32, at: Pos2| {
                let doc_pt = (at - canvas_min(*zoom, *offset)) / *zoom;
                let nz = (*zoom * factor).clamp(0.02, 64.0);
                *offset = at.to_vec2() - doc_pt * nz + vec2(w, h) * nz / 2.0 - area.center().to_vec2();
                *zoom = nz;
            };
            if resp.hovered() {
                let factor = pinch * (scroll / 400.0).exp();
                if (factor - 1.0).abs() > 1e-4
                    && let Some(at) = pointer
                {
                    zoom_at(&mut self.zoom, &mut self.offset, factor, at);
                }
            }
            let panning = (resp.dragged() && (self.tool == Tool::Hand || space)) || (resp.hovered() && middle_down);
            if panning {
                self.offset += delta;
            }
            let min = canvas_min(self.zoom, self.offset);
            let zoom = self.zoom;
            let rect = Rect::from_min_size(min, vec2(w, h) * zoom);
            let to_doc = |p: Pos2| ((p.x - min.x) / zoom, (p.y - min.y) / zoom);
            let to_scr = |d: (f32, f32)| pos2(min.x + d.0 * zoom, min.y + d.1 * zoom);
            self.cursor = pointer.filter(|p| area.contains(*p)).map(to_doc);

            let painting = matches!(self.tool, Tool::Brush | Tool::Eraser);
            if painting && !space && resp.is_pointer_button_down_on() && primary_down {
                if let Some(at) = pointer {
                    let cur = to_doc(at);
                    let erase = self.tool == Tool::Eraser;
                    let color = self.primary.to_array();
                    match self.last {
                        None => {
                            self.doc.checkpoint();
                            self.doc.stamp(cur.0, cur.1, &self.brush, color, erase);
                        }
                        Some(prev) => self.doc.stroke(prev, cur, &self.brush, color, erase),
                    }
                    self.last = Some(cur);
                }
            } else {
                self.last = None;
            }

            if !space && resp.hovered() && pressed
                && let Some(at) = pointer
            {
                let d = to_doc(at);
                match self.tool {
                    Tool::Marquee => self.drag = Drag::Marquee(d),
                    Tool::Move => {
                        self.doc.checkpoint();
                        self.drag = Drag::Move { from: d, original: self.doc.layers[self.doc.active].img.clone() };
                    }
                    Tool::Gradient => self.drag = Drag::Gradient(d),
                    _ => {}
                }
            }
            if let Some(at) = pointer {
                let d = to_doc(at);
                match &self.drag {
                    Drag::Marquee(start) => self.doc.selection = Area::from_points(*start, d, w as u32, h as u32),
                    Drag::Move { from, original } => {
                        let (dx, dy) = ((d.0 - from.0).round() as i32, (d.1 - from.1).round() as i32);
                        let original = original.clone();
                        self.doc.place_layer(&original, dx, dy);
                    }
                    Drag::Gradient(_) => self.gradient_end = Some(d),
                    Drag::None => {}
                }
            }
            if released {
                match std::mem::replace(&mut self.drag, Drag::None) {
                    Drag::Gradient(start) => {
                        if let Some(end) = self.gradient_end.take() {
                            self.doc.gradient(start, end, self.primary.to_array(), self.secondary.to_array());
                        }
                    }
                    Drag::Marquee(_) => {
                        if self.doc.selection.is_some_and(|s| s.width() < 2 && s.height() < 2) {
                            self.doc.selection = None;
                        }
                    }
                    _ => {}
                }
            }

            if resp.clicked() && !space
                && let Some(at) = pointer
            {
                let (x, y) = to_doc(at);
                let inside = x >= 0.0 && y >= 0.0 && x < w && y < h;
                match self.tool {
                    Tool::Fill if inside => {
                        self.doc.checkpoint();
                        self.doc.flood_fill(x as u32, y as u32, self.primary.to_array(), self.tolerance);
                    }
                    Tool::Picker if inside => {
                        let c = self.composite.get_pixel(x as u32, y as u32).0;
                        self.primary = Color32::from_rgb(c[0], c[1], c[2]);
                    }
                    Tool::Zoom => zoom_at(&mut self.zoom, &mut self.offset, if alt { 0.8 } else { 1.25 }, at),
                    _ => {}
                }
            }

            let painter = ui.painter_at(area);
            painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(120) }.as_shape(rect, CornerRadius::ZERO));
            painter.image(self.checker.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(rect.width() / 16.0, rect.height() / 16.0)), Color32::WHITE);
            if let Some(t) = &self.tex {
                painter.image(t.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            if let Some(s) = self.doc.selection {
                let r = Rect::from_min_max(to_scr((s.x0 as f32, s.y0 as f32)), to_scr((s.x1 as f32, s.y1 as f32)));
                let t = ui.input(|i| i.time) as f32;
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
                painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, Color32::BLACK), StrokeKind::Middle);
                let dash = Shape::dashed_line_with_offset(
                    &[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()],
                    Stroke::new(1.0, Color32::WHITE),
                    &[5.0],
                    &[5.0],
                    t * 20.0 % 10.0,
                );
                painter.extend(dash);
            }
            if let (Drag::Gradient(start), Some(end)) = (&self.drag, self.gradient_end) {
                let (a, b) = (to_scr(*start), to_scr(end));
                painter.line_segment([a, b], Stroke::new(3.0, Color32::from_black_alpha(160)));
                painter.line_segment([a, b], Stroke::new(1.5, Color32::WHITE));
                painter.circle(a, 5.0, self.primary, Stroke::new(1.5, Color32::WHITE));
                painter.circle(b, 5.0, self.secondary, Stroke::new(1.5, Color32::WHITE));
            }

            if resp.hovered() {
                ui.ctx().set_cursor_icon(match self.tool {
                    _ if panning || space => egui::CursorIcon::Grabbing,
                    Tool::Hand => egui::CursorIcon::Grab,
                    Tool::Brush | Tool::Eraser => egui::CursorIcon::None,
                    Tool::Move => egui::CursorIcon::Move,
                    Tool::Zoom => if alt { egui::CursorIcon::ZoomOut } else { egui::CursorIcon::ZoomIn },
                    _ => egui::CursorIcon::Crosshair,
                });
                if painting && !space
                    && let Some(at) = pointer
                {
                    let r = (self.brush.radius * zoom).max(2.0);
                    painter.circle_stroke(at, r, Stroke::new(1.0, Color32::from_black_alpha(200)));
                    painter.circle_stroke(at, r + 1.0, Stroke::new(1.0, Color32::from_white_alpha(200)));
                }
            }
        });
    }

    fn sheets(&mut self, ctx: &egui::Context) {
        if let Some(mut nd) = self.new_doc.take() {
            let mut create = false;
            let keep = pro::sheet(ctx, "new", 760.0, |ui| {
                ui.label(RichText::new("Nouveau document").font(semibold(20.0)).color(INK_STRONG));
                ui.add_space(14.0);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(450.0);
                        ui.label(RichText::new("Préréglages").font(semibold(13.0)).color(MUTED));
                        ui.add_space(6.0);
                        for row in PRESETS.chunks(3).enumerate() {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;
                                for (k, (name, kind, w, h)) in row.1.iter().enumerate() {
                                    let idx = row.0 * 3 + k;
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
                        ui.label(RichText::new("Détails").font(semibold(13.0)).color(MUTED));
                        ui.add_space(6.0);
                        ui.add(egui::TextEdit::singleline(&mut nd.name).desired_width(f32::INFINITY));
                        ui.add_space(10.0);
                        ui.label(RichText::new("Largeur").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(&mut nd.w).range(1..=16384).suffix(" px"));
                        ui.label(RichText::new("Hauteur").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(&mut nd.h).range(1..=16384).suffix(" px"));
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Orientation").size(12.5).color(MUTED));
                            if ui.selectable_label(nd.w >= nd.h, "Paysage").clicked() && nd.w < nd.h {
                                std::mem::swap(&mut nd.w, &mut nd.h);
                            }
                            if ui.selectable_label(nd.h > nd.w, "Portrait").clicked() && nd.w >= nd.h {
                                std::mem::swap(&mut nd.w, &mut nd.h);
                            }
                        });
                        ui.add_space(6.0);
                        ui.label(RichText::new("Arrière-plan").size(12.5).color(MUTED));
                        egui::ComboBox::from_id_salt("bg").selected_text(BACKGROUNDS[nd.background]).width(200.0).show_ui(ui, |ui| {
                            for (k, b) in BACKGROUNDS.iter().enumerate() {
                                ui.selectable_value(&mut nd.background, k, *b);
                            }
                        });
                    });
                });
                ui.add_space(16.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Créer").clicked() {
                        create = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        nd.preset = usize::MAX;
                    }
                });
            });
            if create {
                let fill = match nd.background {
                    0 => Some([255, 255, 255, 255]),
                    1 => None,
                    2 => Some([0, 0, 0, 255]),
                    _ => Some(self.primary.to_array()),
                };
                let mut doc = Document::new(nd.w, nd.h, fill);
                doc.name = if nd.name.trim().is_empty() { "Sans titre".into() } else { nd.name.trim().to_string() };
                self.set_doc(doc);
            } else if keep && nd.preset != usize::MAX {
                self.new_doc = Some(nd);
            }
        }

        if let Some((mut adj, base)) = self.adjust.take() {
            let before = adj;
            let mut apply = false;
            let mut cancel = false;
            let keep = pro::sheet(ctx, "adjust", 360.0, |ui| {
                ui.label(RichText::new(adj.title()).font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(12.0);
                ui.spacing_mut().slider_width = 220.0;
                match &mut adj {
                    Adjust::BrightnessContrast { b, c } => {
                        ui.label(RichText::new("Luminosité").size(12.5).color(MUTED));
                        ui.add(egui::Slider::new(b, -100.0..=100.0));
                        ui.label(RichText::new("Contraste").size(12.5).color(MUTED));
                        ui.add(egui::Slider::new(c, -100.0..=100.0));
                    }
                    Adjust::HueSat { h, s, l } => {
                        ui.label(RichText::new("Teinte").size(12.5).color(MUTED));
                        ui.add(egui::Slider::new(h, -180.0..=180.0).suffix("°"));
                        ui.label(RichText::new("Saturation").size(12.5).color(MUTED));
                        ui.add(egui::Slider::new(s, -100.0..=100.0));
                        ui.label(RichText::new("Luminosité").size(12.5).color(MUTED));
                        ui.add(egui::Slider::new(l, -100.0..=100.0));
                    }
                    Adjust::Blur { sigma } => {
                        ui.label(RichText::new("Rayon").size(12.5).color(MUTED));
                        ui.add(egui::Slider::new(sigma, 0.5..=50.0).suffix(" px"));
                    }
                }
                ui.add_space(6.0);
                ui.label(RichText::new("Aperçu en direct sur le calque actif").size(12.0).color(FAINT));
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Appliquer").clicked() {
                        apply = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        cancel = true;
                    }
                });
            });
            let active = self.doc.active;
            if apply {
                self.doc.layers[active].img = base;
                adj.apply(&mut self.doc, true);
            } else if cancel || !keep {
                self.doc.layers[active].img = base;
                self.doc.touch_all();
            } else {
                if adj != before || self.doc.dirty.is_none() && self.thumbs_version == u64::MAX {
                    self.doc.layers[active].img = base.clone();
                    adj.apply(&mut self.doc, false);
                }
                self.adjust = Some((adj, base));
            }
        }

        if let Some((mut w, mut h, mut keep_ratio)) = self.resize.take() {
            let mut apply = false;
            let mut cancel = false;
            let ratio = self.doc.width() as f32 / self.doc.height() as f32;
            let keep = pro::sheet(ctx, "resize", 340.0, |ui| {
                ui.label(RichText::new("Taille de l'image").font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(12.0);
                ui.label(RichText::new("Largeur").size(12.5).color(MUTED));
                if ui.add(egui::DragValue::new(&mut w).range(1..=16384).suffix(" px")).changed() && keep_ratio {
                    h = ((w as f32 / ratio).round() as u32).max(1);
                }
                ui.label(RichText::new("Hauteur").size(12.5).color(MUTED));
                if ui.add(egui::DragValue::new(&mut h).range(1..=16384).suffix(" px")).changed() && keep_ratio {
                    w = ((h as f32 * ratio).round() as u32).max(1);
                }
                ui.checkbox(&mut keep_ratio, "Conserver les proportions");
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Appliquer").clicked() {
                        apply = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        cancel = true;
                    }
                });
            });
            if apply {
                self.doc.selection = None;
                self.doc.transform(|img| imageops::resize(img, w, h, imageops::FilterType::Lanczos3));
                self.fit_pending = true;
            } else if keep && !cancel {
                self.resize = Some((w, h, keep_ratio));
            }
        }
    }
}

fn preset_card(ui: &mut Ui, name: &str, detail: &str, w: u32, h: u32, selected: bool, width: f32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 132.0), Sense::click());
    let hover = resp.hovered();
    let p = ui.painter();
    p.rect(
        r,
        CornerRadius::same(12),
        if hover { Color32::from_rgb(0x2f, 0x2f, 0x35) } else { Color32::from_rgb(0x27, 0x27, 0x2c) },
        Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { BLUE } else { LINE }),
        StrokeKind::Inside,
    );
    let area = Rect::from_center_size(r.center_top() + vec2(0.0, 44.0), vec2(width - 40.0, 56.0));
    let s = (area.width() / w as f32).min(area.height() / h as f32);
    let shape = Rect::from_center_size(area.center(), vec2(w as f32, h as f32) * s);
    p.rect(shape, CornerRadius::same(3), Color32::from_white_alpha(18), Stroke::new(1.2, if selected { BLUE } else { MUTED }), StrokeKind::Inside);
    p.text(pos2(r.center().x, r.bottom() - 36.0), Align2::CENTER_CENTER, name, semibold(13.0), INK_STRONG);
    p.text(pos2(r.center().x, r.bottom() - 17.0), Align2::CENTER_CENTER, detail, FontId::proportional(11.0), MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn image_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
}

fn tool_icon(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    let s = Stroke::new(1.8, c);
    match tool {
        Tool::Move => {
            let m = r.center();
            let d = r.width() * 0.5;
            let k = r.width() * 0.18;
            p.line_segment([m - vec2(d, 0.0), m + vec2(d, 0.0)], s);
            p.line_segment([m - vec2(0.0, d), m + vec2(0.0, d)], s);
            for u in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                let tip = m + u * d;
                let back = tip - u * k;
                let perp = vec2(-u.y, u.x) * k;
                p.add(Shape::convex_polygon(vec![tip, back + perp, back - perp], c, Stroke::NONE));
            }
        }
        Tool::Marquee => {
            let rr = Rect::from_min_max(at(0.05, 0.15), at(0.95, 0.85));
            p.extend(Shape::dashed_line(&[rr.left_top(), rr.right_top(), rr.right_bottom(), rr.left_bottom(), rr.left_top()], s, 3.0, 2.5));
        }
        Tool::Brush => {
            p.line_segment([at(0.95, 0.05), at(0.42, 0.58)], s);
            p.circle_filled(at(0.28, 0.72), r.width() * 0.2, c);
        }
        Tool::Eraser => {
            p.add(Shape::convex_polygon(vec![at(0.55, 0.1), at(0.95, 0.5), at(0.5, 0.95), at(0.1, 0.55)], Color32::TRANSPARENT, s));
            p.line_segment([at(0.3, 0.35), at(0.7, 0.75)], s);
        }
        Tool::Fill => {
            p.add(Shape::convex_polygon(vec![at(0.1, 0.35), at(0.7, 0.35), at(0.6, 0.95), at(0.2, 0.95)], Color32::TRANSPARENT, s));
            p.line_segment([at(0.1, 0.35), at(0.4, 0.05)], s);
            p.circle_filled(at(0.88, 0.7), r.width() * 0.11, c);
        }
        Tool::Gradient => {
            let rr = Rect::from_min_max(at(0.0, 0.15), at(1.0, 0.85));
            let n = 6;
            for k in 0..n {
                let x0 = rr.left() + rr.width() * k as f32 / n as f32;
                let cell = Rect::from_min_max(pos2(x0, rr.top()), pos2(x0 + rr.width() / n as f32, rr.bottom()));
                p.rect_filled(cell, CornerRadius::ZERO, c.gamma_multiply(1.0 - k as f32 / n as f32));
            }
            p.rect_stroke(rr, CornerRadius::same(2), s, StrokeKind::Middle);
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
            .with_title("Pixel")
            .with_icon(Arc::new(studio_ui::app_window_icon("pixel")))
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1000.0, 640.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Pixel", options, Box::new(|cc| Ok(Box::new(Pixel::new(cc)))))
}
