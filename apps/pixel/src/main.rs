#![windows_subsystem = "windows"]

mod doc;

use doc::{Brush, Document, hsl_to_rgb, rgb_to_hsl};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, Frame, Key, Margin, Painter, Pos2, Rect, RichText,
    Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2, pos2, vec2,
};
use image::{RgbaImage, imageops};
use std::path::Path;
use std::sync::Arc;
use studio_ui::{BG, CARD, CARD_HOVER, MUTED, PANEL, TEXT};

const ACCENT: Color32 = Color32::from_rgb(255, 92, 120);

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Brush,
    Eraser,
    Fill,
    Picker,
    Hand,
}

const TOOLS: [(Tool, &str, Key); 5] = [
    (Tool::Brush, "Pinceau (B)", Key::B),
    (Tool::Eraser, "Gomme (E)", Key::E),
    (Tool::Fill, "Pot de peinture (G)", Key::G),
    (Tool::Picker, "Pipette (I)", Key::I),
    (Tool::Hand, "Main (H, ou Espace)", Key::H),
];

enum Dialog {
    New { w: u32, h: u32, white: bool },
    BrightnessContrast { b: f32, c: f32 },
    HueSat { h: f32, s: f32, l: f32 },
    Blur { sigma: f32 },
    Resize { w: u32, h: u32, keep: bool },
}

struct Pixel {
    doc: Document,
    composite: RgbaImage,
    tex: Option<TextureHandle>,
    tex_filter: TextureOptions,
    checker: TextureHandle,
    tool: Tool,
    brush: Brush,
    primary: Color32,
    secondary: Color32,
    tolerance: u8,
    zoom: f32,
    offset: Vec2,
    fit_pending: bool,
    last: Option<(f32, f32)>,
    cursor: Option<(f32, f32)>,
    dialog: Option<Dialog>,
    renaming: Option<(usize, String)>,
    error: Option<String>,
    title: String,
}

impl Pixel {
    fn new(cc: &eframe::CreationContext) -> Self {
        studio_ui::apply_style(&cc.egui_ctx);
        cc.egui_ctx.all_styles_mut(|s| s.visuals.selection.bg_fill = ACCENT);
        let checker = {
            let mut px = Vec::with_capacity(16 * 16 * 4);
            for y in 0..16 {
                for x in 0..16 {
                    let v = if (x / 8 + y / 8) % 2 == 0 { 58 } else { 44 };
                    px.extend_from_slice(&[v, v, v, 255]);
                }
            }
            cc.egui_ctx.load_texture(
                "checker",
                egui::ColorImage::from_rgba_unmultiplied([16, 16], &px),
                TextureOptions {
                    wrap_mode: egui::TextureWrapMode::Repeat,
                    ..TextureOptions::NEAREST
                },
            )
        };
        let mut me = Self {
            doc: Document::new(1920, 1080, Some([255, 255, 255, 255])),
            composite: RgbaImage::new(1, 1),
            tex: None,
            tex_filter: TextureOptions::LINEAR,
            checker,
            tool: Tool::Brush,
            brush: Brush { radius: 12.0, hardness: 0.8, flow: 1.0 },
            primary: Color32::from_rgb(20, 20, 24),
            secondary: Color32::WHITE,
            tolerance: 32,
            zoom: 1.0,
            offset: Vec2::ZERO,
            fit_pending: true,
            last: None,
            cursor: None,
            dialog: None,
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
    }

    fn open_path(&mut self, path: &Path) {
        match image::open(path) {
            Ok(img) => {
                let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                self.set_doc(Document::from_image(name, img.into_rgba8()));
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
        let result = if ext == "jpg" || ext == "jpeg" {
            image::DynamicImage::ImageRgba8(flat).to_rgb8().save(&path)
        } else {
            flat.save(&path)
        };
        match result {
            Ok(()) => self.doc.modified = false,
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
                let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], self.composite.as_raw());
                t.set(img, filter);
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
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [area.width() as usize, area.height() as usize],
                    sub.as_raw(),
                );
                t.set_partial([area.x0 as usize, area.y0 as usize], img, self.tex_filter);
            }
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (undo, redo, zero, export, open, new) = ctx.input_mut(|i| {
            use egui::{KeyboardShortcut as K, Modifiers as M};
            (
                i.consume_shortcut(&K::new(M::COMMAND, Key::Z)),
                i.consume_shortcut(&K::new(M::COMMAND | M::SHIFT, Key::Z))
                    || i.consume_shortcut(&K::new(M::COMMAND, Key::Y)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::Num0)),
                i.consume_shortcut(&K::new(M::COMMAND | M::SHIFT, Key::S)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::O)),
                i.consume_shortcut(&K::new(M::COMMAND, Key::N)),
            )
        });
        if redo {
            self.doc.redo();
        } else if undo {
            self.doc.undo();
        }
        if zero {
            self.fit_pending = true;
        }
        if export {
            self.export();
        }
        if open {
            self.open_dialog();
        }
        if new {
            self.dialog = Some(Dialog::New { w: 1920, h: 1080, white: true });
        }
        ctx.input(|i| {
            if i.modifiers.command {
                return;
            }
            for (tool, _, key) in TOOLS {
                if i.key_pressed(key) {
                    self.tool = tool;
                }
            }
            if i.key_pressed(Key::X) {
                std::mem::swap(&mut self.primary, &mut self.secondary);
            }
            if i.key_pressed(Key::OpenBracket) {
                self.brush.radius = (self.brush.radius / 1.2).max(0.5);
            }
            if i.key_pressed(Key::CloseBracket) {
                self.brush.radius = (self.brush.radius * 1.2).min(500.0);
            }
        });
    }
}

impl eframe::App for Pixel {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        self.shortcuts(ctx);

        if let Some(f) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty())) {
            self.open_path(&f);
        }

        let title = format!("Pixel — {}{}", self.doc.name, if self.doc.modified { " •" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        self.menu(root);
        self.status_bar(root);
        self.toolbar(root);
        self.side_panel(root);
        self.sync_texture(ctx);
        self.canvas(root);
        self.dialogs(ctx);
    }
}

impl Pixel {
    fn menu(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("menu")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 6)))
            .show(root, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    studio_ui::app_icon(ui, 22.0, "pixel", ACCENT);
                    ui.add_space(4.0);
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouveau…    Ctrl+N").clicked() {
                            self.dialog = Some(Dialog::New { w: 1920, h: 1080, white: true });
                        }
                        if ui.button("Ouvrir…    Ctrl+O").clicked() {
                            self.open_dialog();
                        }
                        if ui.button("Importer comme calque…").clicked() {
                            self.import_layer();
                        }
                        ui.separator();
                        if ui.button("Exporter…    Ctrl+Maj+S").clicked() {
                            self.export();
                        }
                    });
                    ui.menu_button("Édition", |ui| {
                        if ui.add_enabled(self.doc.can_undo(), egui::Button::new("Annuler    Ctrl+Z")).clicked() {
                            self.doc.undo();
                        }
                        if ui.add_enabled(self.doc.can_redo(), egui::Button::new("Rétablir    Ctrl+Y")).clicked() {
                            self.doc.redo();
                        }
                    });
                    ui.menu_button("Image", |ui| {
                        if ui.button("Luminosité / Contraste…").clicked() {
                            self.dialog = Some(Dialog::BrightnessContrast { b: 0.0, c: 0.0 });
                        }
                        if ui.button("Teinte / Saturation…").clicked() {
                            self.dialog = Some(Dialog::HueSat { h: 0.0, s: 0.0, l: 0.0 });
                        }
                        if ui.button("Flou…").clicked() {
                            self.dialog = Some(Dialog::Blur { sigma: 3.0 });
                        }
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
                        ui.separator();
                        if ui.button("Taille de l'image…").clicked() {
                            self.dialog = Some(Dialog::Resize { w: self.doc.width(), h: self.doc.height(), keep: true });
                        }
                        if ui.button("Rotation 90° horaire").clicked() {
                            self.doc.transform(imageops::rotate90);
                            self.fit_pending = true;
                        }
                        if ui.button("Rotation 90° antihoraire").clicked() {
                            self.doc.transform(imageops::rotate270);
                            self.fit_pending = true;
                        }
                        if ui.button("Miroir horizontal").clicked() {
                            self.doc.transform(imageops::flip_horizontal);
                        }
                        if ui.button("Miroir vertical").clicked() {
                            self.doc.transform(imageops::flip_vertical);
                        }
                    });
                    ui.menu_button("Calque", |ui| {
                        if ui.button("Nouveau calque").clicked() {
                            self.doc.add_layer(None, None);
                        }
                        if ui.button("Dupliquer").clicked() {
                            self.doc.duplicate_layer();
                        }
                        if ui.button("Fusionner avec le dessous").clicked() {
                            self.doc.merge_down();
                        }
                        if ui.button("Supprimer").clicked() {
                            self.doc.delete_layer();
                        }
                    });
                    ui.menu_button("Affichage", |ui| {
                        if ui.button("Ajuster à l'écran    Ctrl+0").clicked() {
                            self.fit_pending = true;
                        }
                        if ui.button("100 %").clicked() {
                            self.zoom = 1.0;
                            self.offset = Vec2::ZERO;
                        }
                    });
                });
            });
    }

    fn status_bar(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("status")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 5)))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                    ui.label(small(format!("{} × {} px", self.doc.width(), self.doc.height())));
                    ui.separator();
                    ui.label(small(format!("{:.0} %", self.zoom * 100.0)));
                    if let Some((x, y)) = self.cursor {
                        ui.separator();
                        ui.label(small(format!("x {:.0}  y {:.0}", x.floor(), y.floor())));
                    }
                    if let Some(e) = self.error.clone() {
                        ui.separator();
                        ui.label(RichText::new(e).size(12.0).color(studio_ui::DANGER));
                        if ui.small_button("×").clicked() {
                            self.error = None;
                        }
                    }
                });
            });
    }

    fn toolbar(&mut self, root: &mut egui::Ui) {
        egui::Panel::left("tools")
            .resizable(false)
            .exact_size(52.0)
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(6, 10)))
            .show(root, |ui| {
                for (tool, tip, _) in TOOLS {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
                    let on = self.tool == tool;
                    if on {
                        ui.painter().rect_filled(rect, CornerRadius::same(8), ACCENT.gamma_multiply(0.22));
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, CornerRadius::same(8), CARD_HOVER);
                    }
                    tool_icon(ui.painter(), rect.shrink(10.0), tool, if on { ACCENT } else { TEXT });
                    if resp.on_hover_text(tip).clicked() {
                        self.tool = tool;
                    }
                }
                ui.add_space(12.0);
                let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
                let back = Rect::from_min_size(rect.min + vec2(14.0, 14.0), Vec2::splat(22.0));
                let front = Rect::from_min_size(rect.min + vec2(4.0, 4.0), Vec2::splat(22.0));
                ui.painter().rect_filled(back, CornerRadius::same(5), self.secondary);
                ui.painter().rect_stroke(back, CornerRadius::same(5), Stroke::new(1.0, MUTED), StrokeKind::Outside);
                ui.painter().rect_filled(front, CornerRadius::same(5), self.primary);
                ui.painter().rect_stroke(front, CornerRadius::same(5), Stroke::new(1.5, TEXT), StrokeKind::Outside);
                if resp.on_hover_text("Inverser les couleurs (X)").clicked() {
                    std::mem::swap(&mut self.primary, &mut self.secondary);
                }
            });
    }

    fn side_panel(&mut self, root: &mut egui::Ui) {
        egui::Panel::right("side")
            .resizable(false)
            .exact_size(270.0)
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::same(12)))
            .show(root, |ui| {
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    ui.set_max_width(246.0);
                    heading(ui, "Couleur");
                    egui::widgets::color_picker::color_picker_color32(
                        ui,
                        &mut self.primary,
                        egui::widgets::color_picker::Alpha::OnlyBlend,
                    );
                    ui.label(
                        RichText::new(format!(
                            "#{:02X}{:02X}{:02X}",
                            self.primary.r(),
                            self.primary.g(),
                            self.primary.b()
                        ))
                        .monospace()
                        .color(MUTED),
                    );
                    ui.add_space(12.0);

                    match self.tool {
                        Tool::Brush | Tool::Eraser => {
                            heading(ui, if self.tool == Tool::Brush { "Pinceau" } else { "Gomme" });
                            ui.add(egui::Slider::new(&mut self.brush.radius, 0.5..=300.0).logarithmic(true).text("Taille"));
                            ui.add(egui::Slider::new(&mut self.brush.hardness, 0.0..=1.0).text("Dureté"));
                            ui.add(egui::Slider::new(&mut self.brush.flow, 0.01..=1.0).text("Flux"));
                        }
                        Tool::Fill => {
                            heading(ui, "Pot de peinture");
                            ui.add(egui::Slider::new(&mut self.tolerance, 0..=255).text("Tolérance"));
                        }
                        _ => {}
                    }
                    ui.add_space(12.0);
                    self.layers_panel(ui);
                });
            });
    }

    fn layers_panel(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Calques");
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("+").on_hover_text("Nouveau calque").clicked() {
                self.doc.add_layer(None, None);
            }
            if ui.small_button("Dupliquer").on_hover_text("Dupliquer").clicked() {
                self.doc.duplicate_layer();
            }
            if ui.small_button("▲").on_hover_text("Monter").clicked() {
                self.doc.move_layer(true);
            }
            if ui.small_button("▼").on_hover_text("Descendre").clicked() {
                self.doc.move_layer(false);
            }
            if ui.small_button("Fusionner").on_hover_text("Fusionner avec le dessous").clicked() {
                self.doc.merge_down();
            }
            if ui.small_button("✕").on_hover_text("Supprimer").clicked() {
                self.doc.delete_layer();
            }
        });
        ui.add_space(6.0);

        let active = self.doc.active;
        let mut changed_visibility = false;
        for i in (0..self.doc.layers.len()).rev() {
            let selected = i == active;
            let row = Frame::new()
                .fill(if selected { CARD_HOVER } else { CARD })
                .stroke(Stroke::new(1.0, if selected { ACCENT.gamma_multiply(0.6) } else { Color32::TRANSPARENT }))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (eye, eye_resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                        let visible = self.doc.layers[i].visible;
                        ui.painter().circle_stroke(eye.center(), 6.0, Stroke::new(1.5, if visible { TEXT } else { MUTED }));
                        if visible {
                            ui.painter().circle_filled(eye.center(), 2.5, TEXT);
                        }
                        if eye_resp.on_hover_text("Afficher / masquer").clicked() {
                            self.doc.layers[i].visible = !visible;
                            changed_visibility = true;
                        }
                        if let Some((ri, text)) = &mut self.renaming
                            && *ri == i
                        {
                            let r = ui.text_edit_singleline(text);
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
                            let r = ui.add(
                                egui::Label::new(RichText::new(&self.doc.layers[i].name).color(if selected { TEXT } else { MUTED }))
                                    .sense(Sense::click())
                                    .truncate(),
                            );
                            if r.double_clicked() {
                                self.renaming = Some((i, self.doc.layers[i].name.clone()));
                            } else if r.clicked() {
                                self.doc.active = i;
                            }
                        }
                    });
                })
                .response;
            if row.interact(Sense::click()).clicked() {
                self.doc.active = i;
            }
            ui.add_space(3.0);
        }
        if changed_visibility {
            self.doc.touch_all();
        }

        ui.add_space(6.0);
        let mut op = self.doc.layers[self.doc.active].opacity * 100.0;
        if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).suffix(" %").text("Opacité")).changed() {
            self.doc.layers[self.doc.active].opacity = op / 100.0;
            self.doc.touch_all();
            self.doc.modified = true;
        }
    }

    fn canvas(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG))
            .show(root, |ui| {
                let area = ui.max_rect();
                let resp = ui.allocate_rect(area, Sense::click_and_drag());
                let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);

                if self.fit_pending {
                    self.zoom = ((area.width() - 60.0) / w).min((area.height() - 60.0) / h).clamp(0.02, 1.0);
                    self.offset = Vec2::ZERO;
                    self.fit_pending = false;
                }

                let canvas_min = |zoom: f32, offset: Vec2| area.center() + offset - vec2(w, h) * zoom / 2.0;

                let (scroll, pinch, space, pointer, primary_down, middle_down, delta) = ui.input(|i| {
                    (
                        i.smooth_scroll_delta.y,
                        i.zoom_delta(),
                        i.key_down(Key::Space),
                        i.pointer.hover_pos(),
                        i.pointer.primary_down(),
                        i.pointer.middle_down(),
                        i.pointer.delta(),
                    )
                });

                if resp.hovered() {
                    let factor = pinch * (scroll / 400.0).exp();
                    if (factor - 1.0).abs() > 1e-4 {
                        if let Some(p) = pointer {
                            let doc_pt = (p - canvas_min(self.zoom, self.offset)) / self.zoom;
                            let nz = (self.zoom * factor).clamp(0.02, 64.0);
                            self.offset = p.to_vec2() - doc_pt * nz + vec2(w, h) * nz / 2.0 - area.center().to_vec2();
                            self.zoom = nz;
                        }
                    }
                }

                let panning = (resp.dragged() && (self.tool == Tool::Hand || space)) || (resp.hovered() && middle_down);
                if panning {
                    self.offset += delta;
                }

                let min = canvas_min(self.zoom, self.offset);
                let rect = Rect::from_min_size(min, vec2(w, h) * self.zoom);
                let to_doc = |p: Pos2| ((p.x - min.x) / self.zoom, (p.y - min.y) / self.zoom);

                self.cursor = pointer.filter(|p| area.contains(*p)).map(to_doc);

                let painting_tool = matches!(self.tool, Tool::Brush | Tool::Eraser);
                if painting_tool && !space && resp.is_pointer_button_down_on() && primary_down {
                    if let Some(p) = pointer {
                        let cur = to_doc(p);
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

                if resp.clicked() && !space {
                    if let Some(p) = pointer {
                        let (x, y) = to_doc(p);
                        if x >= 0.0 && y >= 0.0 && x < w && y < h {
                            match self.tool {
                                Tool::Fill => {
                                    self.doc.checkpoint();
                                    self.doc.flood_fill(x as u32, y as u32, self.primary.to_array(), self.tolerance);
                                }
                                Tool::Picker => {
                                    let c = self.composite.get_pixel(x as u32, y as u32).0;
                                    self.primary = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 255);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                let painter = ui.painter_at(area);
                painter.rect_filled(rect.expand(1.0), CornerRadius::ZERO, Color32::from_black_alpha(120));
                painter.image(
                    self.checker.id(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(rect.width() / 16.0, rect.height() / 16.0)),
                    Color32::WHITE,
                );
                if let Some(t) = &self.tex {
                    painter.image(t.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }

                let cursor_icon = match self.tool {
                    _ if panning || space => egui::CursorIcon::Grab,
                    Tool::Hand => egui::CursorIcon::Grab,
                    Tool::Brush | Tool::Eraser => egui::CursorIcon::None,
                    _ => egui::CursorIcon::Crosshair,
                };
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(cursor_icon);
                    if painting_tool && !space {
                        if let Some(p) = pointer {
                            let r = (self.brush.radius * self.zoom).max(2.0);
                            painter.circle_stroke(p, r, Stroke::new(1.0, Color32::from_black_alpha(200)));
                            painter.circle_stroke(p, r + 1.0, Stroke::new(1.0, Color32::from_white_alpha(200)));
                        }
                    }
                }
            });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else { return };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        let title = match dialog {
            Dialog::New { .. } => "Nouveau document",
            Dialog::BrightnessContrast { .. } => "Luminosité / Contraste",
            Dialog::HueSat { .. } => "Teinte / Saturation",
            Dialog::Blur { .. } => "Flou gaussien",
            Dialog::Resize { .. } => "Taille de l'image",
        };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                match &mut dialog {
                    Dialog::New { w, h, white } => {
                        ui.horizontal_wrapped(|ui| {
                            for (label, pw, ph) in [
                                ("Full HD", 1920, 1080),
                                ("4K", 3840, 2160),
                                ("Carré", 1080, 1080),
                                ("Story", 1080, 1920),
                                ("Miniature YT", 1280, 720),
                                ("A4 300 dpi", 2480, 3508),
                            ] {
                                if ui.button(label).clicked() {
                                    *w = pw;
                                    *h = ph;
                                }
                            }
                        });
                        ui.add_space(6.0);
                        ui.add(egui::DragValue::new(w).range(1..=16384).prefix("Largeur ").suffix(" px"));
                        ui.add(egui::DragValue::new(h).range(1..=16384).prefix("Hauteur ").suffix(" px"));
                        ui.checkbox(white, "Fond blanc (sinon transparent)");
                    }
                    Dialog::BrightnessContrast { b, c } => {
                        ui.add(egui::Slider::new(b, -100.0..=100.0).text("Luminosité"));
                        ui.add(egui::Slider::new(c, -100.0..=100.0).text("Contraste"));
                    }
                    Dialog::HueSat { h, s, l } => {
                        ui.add(egui::Slider::new(h, -180.0..=180.0).text("Teinte").suffix("°"));
                        ui.add(egui::Slider::new(s, -100.0..=100.0).text("Saturation"));
                        ui.add(egui::Slider::new(l, -100.0..=100.0).text("Luminosité"));
                    }
                    Dialog::Blur { sigma } => {
                        ui.add(egui::Slider::new(sigma, 0.5..=50.0).text("Rayon"));
                    }
                    Dialog::Resize { w, h, keep } => {
                        let ratio = self.doc.width() as f32 / self.doc.height() as f32;
                        if ui.add(egui::DragValue::new(w).range(1..=16384).prefix("Largeur ").suffix(" px")).changed() && *keep {
                            *h = ((*w as f32 / ratio).round() as u32).max(1);
                        }
                        if ui.add(egui::DragValue::new(h).range(1..=16384).prefix("Hauteur ").suffix(" px")).changed() && *keep {
                            *w = ((*h as f32 * ratio).round() as u32).max(1);
                        }
                        ui.checkbox(keep, "Conserver les proportions");
                    }
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("Appliquer").strong().color(Color32::from_rgb(14, 15, 20))).fill(ACCENT))
                        .clicked()
                    {
                        apply = true;
                    }
                    if ui.button("Annuler").clicked() {
                        cancel = true;
                    }
                });
            });

        if apply {
            match dialog {
                Dialog::New { w, h, white } => {
                    self.set_doc(Document::new(w, h, white.then_some([255, 255, 255, 255])));
                }
                Dialog::BrightnessContrast { b, c } => {
                    let b = b / 100.0;
                    let k = ((c / 100.0) + 1.0).powi(2);
                    self.doc.adjust(move |p| {
                        for v in &mut p[..3] {
                            let x = *v as f32 / 255.0 + b;
                            *v = (((x - 0.5) * k + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8;
                        }
                    });
                }
                Dialog::HueSat { h, s, l } => {
                    self.doc.adjust(move |p| {
                        let (hh, ss, ll) = rgb_to_hsl(p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0);
                        let ss = (ss * (1.0 + s / 100.0)).clamp(0.0, 1.0);
                        let ll = if l >= 0.0 { ll + (1.0 - ll) * l / 100.0 } else { ll * (1.0 + l / 100.0) };
                        let (r, g, b) = hsl_to_rgb(hh + h / 360.0, ss, ll);
                        p[0] = (r * 255.0).round() as u8;
                        p[1] = (g * 255.0).round() as u8;
                        p[2] = (b * 255.0).round() as u8;
                    });
                }
                Dialog::Blur { sigma } => self.doc.blur(sigma),
                Dialog::Resize { w, h, .. } => {
                    self.doc.transform(|img| imageops::resize(img, w, h, imageops::FilterType::Lanczos3));
                    self.fit_pending = true;
                }
            }
        } else if open && !cancel {
            self.dialog = Some(dialog);
        }
    }
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).size(11.0).color(MUTED));
    ui.add_space(4.0);
}

fn image_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
}

fn tool_icon(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    let s = Stroke::new(2.0, c);
    match tool {
        Tool::Brush => {
            p.line_segment([at(0.95, 0.05), at(0.42, 0.58)], s);
            p.circle_filled(at(0.28, 0.72), r.width() * 0.2, c);
        }
        Tool::Eraser => {
            p.add(Shape::convex_polygon(
                vec![at(0.55, 0.1), at(0.95, 0.5), at(0.5, 0.95), at(0.1, 0.55)],
                Color32::TRANSPARENT,
                s,
            ));
            p.line_segment([at(0.3, 0.35), at(0.7, 0.75)], s);
        }
        Tool::Fill => {
            p.add(Shape::convex_polygon(
                vec![at(0.1, 0.35), at(0.7, 0.35), at(0.6, 0.95), at(0.2, 0.95)],
                Color32::TRANSPARENT,
                s,
            ));
            p.line_segment([at(0.1, 0.35), at(0.4, 0.05)], s);
            p.circle_filled(at(0.88, 0.7), r.width() * 0.11, c);
        }
        Tool::Picker => {
            p.line_segment([at(0.08, 0.92), at(0.62, 0.38)], Stroke::new(2.5, c));
            p.circle_filled(at(0.75, 0.25), r.width() * 0.2, c);
        }
        Tool::Hand => {
            let m = r.center();
            let d = r.width() * 0.5;
            p.line_segment([m - vec2(d, 0.0), m + vec2(d, 0.0)], s);
            p.line_segment([m - vec2(0.0, d), m + vec2(0.0, d)], s);
            let k = r.width() * 0.2;
            for u in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                let tip = m + u * d;
                let back = tip - u * k;
                let perp = vec2(-u.y, u.x) * k;
                p.line_segment([tip, back + perp], s);
                p.line_segment([tip, back - perp], s);
            }
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Pixel")
            .with_icon(Arc::new(studio_ui::app_window_icon("pixel")))
            .with_inner_size([1400.0, 880.0])
            .with_min_inner_size([900.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Pixel", options, Box::new(|cc| Ok(Box::new(Pixel::new(cc)))))
}
