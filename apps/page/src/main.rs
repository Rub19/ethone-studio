#![windows_subsystem = "windows"]

mod model;

use eframe::egui::{
    self, Color32, CornerRadius, Frame as EFrame, Key, Margin, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, TextureHandle, TextureOptions, Vec2, pos2, vec2,
};
use krilla_svg::{SurfaceExt, SvgSettings};
use model::{Doc, Frame, Kind, Page};
use resvg::{tiny_skia, usvg};
use std::path::PathBuf;
use std::sync::Arc;
use studio_ui::{BG, DANGER, MUTED, OK, PANEL};
use usvg::fontdb;

const ACCENT: Color32 = Color32::from_rgb(255, 120, 60);
const FONTS: [&str; 8] = ["Segoe UI", "Arial", "Georgia", "Times New Roman", "Calibri", "Verdana", "Consolas", "Garamond"];

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Text,
    Image,
    Rect,
}

enum Drag {
    None,
    Move { last: [f32; 2] },
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
                    let units: f32 = text
                        .chars()
                        .map(|c| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map_or(upem * 0.5, |a| a as f32))
                        .sum();
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
    notice: Option<(String, bool)>,
    title: String,
}

impl PageApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        studio_ui::apply_style(&cc.egui_ctx);
        cc.egui_ctx.all_styles_mut(|s| s.visuals.selection.bg_fill = ACCENT);
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let mut doc = Doc::a4();
        doc.pages[0].frames.push(Frame {
            x: 40.0,
            y: 60.0,
            w: 515.0,
            h: 60.0,
            kind: Kind::Text { text: "Titre du document".into(), size: 34.0, family: "Segoe UI".into(), bold: true, color: [20, 20, 24, 255], align: 0, leading: 1.15 },
        });
        doc.pages[0].frames.push(Frame {
            x: 40.0,
            y: 790.0,
            w: 515.0,
            h: 20.0,
            kind: Kind::Text { text: "{page} / {pages}".into(), size: 9.0, family: "Segoe UI".into(), bold: false, color: [120, 120, 130, 255], align: 2, leading: 1.2 },
        });
        Self {
            saved: doc.clone(),
            doc,
            file: None,
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
            notice: None,
            title: String::new(),
        }
    }

    fn svg(&self, index: usize, guides: bool) -> (String, Vec<bool>) {
        let m = |t: &str, f: &str, b: bool, s: f32| self.fonts.width(t, f, b, s);
        self.doc.to_svg(index, &m, guides)
    }

    fn opts(&self) -> usvg::Options<'static> {
        usvg::Options { fontdb: self.fonts.db.clone(), ..Default::default() }
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

    fn export_png(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name(format!("page-{}.png", self.page + 1)).save_file() else { return };
        let (svg, _) = self.svg(self.page, false);
        let scale = 300.0 / 72.0;
        let result = usvg::Tree::from_str(&svg, &self.opts()).map_err(|e| e.to_string()).and_then(|tree| {
            let mut pix = tiny_skia::Pixmap::new((self.doc.width * scale) as u32, (self.doc.height * scale) as u32).ok_or("taille")?;
            resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
            pix.save_png(&path).map_err(|e| e.to_string())
        });
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
                self.file = Some(path);
                self.saved = self.doc.clone();
            }
            Err(e) => self.notice = Some((format!("Enregistrement : {e}"), false)),
        }
    }

    fn open(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("Projet Page", &["epage"]).pick_file() else { return };
        match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Doc>(&b).map_err(|e| e.to_string())) {
            Ok(d) => {
                self.saved = d.clone();
                self.doc = d;
                self.file = Some(p);
                self.page = 0;
                self.sel = None;
                self.undo.clear();
                self.redo.clear();
            }
            Err(e) => self.notice = Some((format!("Ouverture : {e}"), false)),
        }
    }

    fn frames(&mut self) -> &mut Vec<Frame> {
        &mut self.doc.pages[self.page].frames
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
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
        if sc(M::COMMAND, Key::O) {
            self.open();
        }
        if sc(M::COMMAND, Key::D)
            && let Some(i) = self.sel
        {
            let mut f = self.frames()[i].clone();
            f.x += 12.0;
            f.y += 12.0;
            self.frames().push(f);
            self.sel = Some(self.frames().len() - 1);
        }
        let (del, v, t, im, r) = ctx.input(|i| {
            (i.key_pressed(Key::Delete), i.key_pressed(Key::V), i.key_pressed(Key::T), i.key_pressed(Key::I), i.key_pressed(Key::R))
        });
        if del && let Some(i) = self.sel.take() {
            self.frames().remove(i);
        }
        for (pressed, tool) in [(v, Tool::Select), (t, Tool::Text), (im, Tool::Image), (r, Tool::Rect)] {
            if pressed {
                self.tool = tool;
            }
        }
        self.page = self.page.min(self.doc.pages.len() - 1);
        if self.sel.is_some_and(|i| i >= self.doc.pages[self.page].frames.len()) {
            self.sel = None;
        }
    }
}

impl eframe::App for PageApp {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let before = self.doc.clone();
        self.keys(ctx);
        let name = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = format!("Page — {name}{}", if self.doc != self.saved { " •" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.menu(root);
        self.status(root);
        self.pages_panel(root);
        self.props(root);
        self.canvas(root);
        if self.doc != before && !self.editing {
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
    fn menu(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("menu").frame(EFrame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 6))).show(root, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                studio_ui::app_icon(ui, 22.0, "page", ACCENT);
                ui.add_space(4.0);
                ui.menu_button("Fichier", |ui| {
                    for (label, w, h) in [("Nouveau A4", 595.0, 842.0), ("Nouveau A5", 420.0, 595.0), ("Nouveau Lettre US", 612.0, 792.0), ("Nouveau carré 1080", 810.0, 810.0)] {
                        if ui.button(label).clicked() {
                            self.doc = Doc { width: w, height: h, ..Doc::a4() };
                            self.saved = self.doc.clone();
                            self.file = None;
                            self.page = 0;
                            self.sel = None;
                        }
                    }
                    ui.separator();
                    if ui.button("Ouvrir…    Ctrl+O").clicked() {
                        self.open();
                    }
                    if ui.button("Enregistrer    Ctrl+S").clicked() {
                        self.save(false);
                    }
                    if ui.button("Enregistrer sous…").clicked() {
                        self.save(true);
                    }
                    ui.separator();
                    if ui.button("Exporter en PDF…").clicked() {
                        self.export_pdf();
                    }
                    if ui.button("Exporter la page en PNG 300 dpi…").clicked() {
                        self.export_png();
                    }
                });
                ui.menu_button("Page", |ui| {
                    if ui.button("Ajouter une page").clicked() {
                        self.doc.pages.insert(self.page + 1, Page::default());
                        self.page += 1;
                        self.sel = None;
                    }
                    if ui.button("Dupliquer la page").clicked() {
                        let p = self.doc.pages[self.page].clone();
                        self.doc.pages.insert(self.page + 1, p);
                        self.page += 1;
                    }
                    if ui.add_enabled(self.doc.pages.len() > 1, egui::Button::new("Supprimer la page")).clicked() {
                        self.doc.pages.remove(self.page);
                        self.page = self.page.min(self.doc.pages.len() - 1);
                        self.sel = None;
                    }
                    ui.separator();
                    ui.add(egui::DragValue::new(&mut self.doc.margin).range(0.0..=200.0).prefix("Marges ").suffix(" pt"));
                });
                ui.menu_button("Affichage", |ui| {
                    for z in [0.5, 0.75, 1.0, 1.5, 2.0] {
                        if ui.button(format!("{:.0} %", z * 100.0)).clicked() {
                            self.zoom = z;
                        }
                    }
                });
            });
        });
    }

    fn status(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("status").frame(EFrame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 5))).show(root, |ui| {
            ui.horizontal(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                ui.label(small(format!("Page {}/{} · {:.0} × {:.0} pt · {:.0} %", self.page + 1, self.doc.pages.len(), self.doc.width, self.doc.height, self.zoom * 100.0)));
                ui.separator();
                ui.label(small("V sélection · T texte · I image · R rectangle · glisser pour créer · {page} et {pages} = numéros".into()));
                if let Some((n, ok)) = self.notice.clone() {
                    ui.separator();
                    ui.label(RichText::new(n).size(12.0).color(if ok { OK } else { DANGER }));
                    if ui.small_button("×").clicked() {
                        self.notice = None;
                    }
                }
            });
        });
    }

    fn pages_panel(&mut self, root: &mut egui::Ui) {
        egui::Panel::left("pages").resizable(false).exact_size(176.0).frame(EFrame::new().fill(PANEL).inner_margin(Margin::same(10))).show(root, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (tool, label) in [(Tool::Select, "V"), (Tool::Text, "T"), (Tool::Image, "I"), (Tool::Rect, "R")] {
                    let tip = match tool {
                        Tool::Select => "Sélection",
                        Tool::Text => "Bloc de texte",
                        Tool::Image => "Bloc image",
                        Tool::Rect => "Rectangle",
                    };
                    if ui.selectable_label(self.tool == tool, RichText::new(label).strong()).on_hover_text(tip).clicked() {
                        self.tool = tool;
                    }
                }
            });
            ui.add_space(10.0);
            ui.label(RichText::new("PAGES").size(11.0).color(MUTED));
            egui::ScrollArea::vertical().show(ui, |ui| {
                let ratio = self.doc.height / self.doc.width;
                for i in 0..self.doc.pages.len() {
                    let w = 100.0;
                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), w * ratio + 22.0), Sense::click());
                    let card = Rect::from_center_size(pos2(r.center().x, r.min.y + w * ratio / 2.0 + 4.0), vec2(w, w * ratio));
                    let p = ui.painter();
                    p.rect_filled(card, CornerRadius::same(2), Color32::WHITE);
                    for f in &self.doc.pages[i].frames {
                        let s = w / self.doc.width;
                        let fr = Rect::from_min_size(card.min + vec2(f.x, f.y) * s, vec2(f.w, f.h) * s);
                        let c = match f.kind {
                            Kind::Text { .. } => Color32::from_gray(170),
                            Kind::Image { .. } => Color32::from_rgb(180, 200, 220),
                            Kind::Rect { fill, .. } => Color32::from_rgb(fill[0], fill[1], fill[2]),
                        };
                        p.rect_filled(fr, CornerRadius::ZERO, c);
                    }
                    if i == self.page {
                        p.rect_stroke(card.expand(2.0), CornerRadius::same(3), Stroke::new(2.0, ACCENT), StrokeKind::Outside);
                    } else if resp.hovered() {
                        p.rect_stroke(card.expand(2.0), CornerRadius::same(3), Stroke::new(1.0, MUTED), StrokeKind::Outside);
                    }
                    p.text(pos2(r.center().x, r.max.y - 8.0), egui::Align2::CENTER_CENTER, (i + 1).to_string(), egui::FontId::proportional(11.0), MUTED);
                    if resp.clicked() {
                        self.page = i;
                        self.sel = None;
                    }
                }
                if ui.add_sized([ui.available_width(), 28.0], egui::Button::new("+ Page")).clicked() {
                    self.doc.pages.push(Page::default());
                    self.page = self.doc.pages.len() - 1;
                    self.sel = None;
                }
            });
        });
    }

    fn props(&mut self, root: &mut egui::Ui) {
        egui::Panel::right("props").resizable(false).exact_size(290.0).frame(EFrame::new().fill(PANEL).inner_margin(Margin::same(12))).show(root, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let Some(i) = self.sel else {
                    ui.label(RichText::new("Choisis un outil (T, I, R) et dessine un bloc sur la page, ou clique sur un bloc pour le modifier.").color(MUTED));
                    return;
                };
                let over = self.overflow.get(i).copied().unwrap_or(false);
                let page = self.page;
                let f = &mut self.doc.pages[page].frames[i];
                ui.label(RichText::new("POSITION").size(11.0).color(MUTED));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut f.x).prefix("X "));
                    ui.add(egui::DragValue::new(&mut f.y).prefix("Y "));
                });
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut f.w).range(1.0..=5000.0).prefix("L "));
                    ui.add(egui::DragValue::new(&mut f.h).range(1.0..=5000.0).prefix("H "));
                });
                ui.add_space(10.0);
                match &mut f.kind {
                    Kind::Text { text, size, family, bold, color, align, leading } => {
                        ui.label(RichText::new("TEXTE").size(11.0).color(MUTED));
                        ui.add(egui::TextEdit::multiline(text).desired_rows(8).desired_width(f32::INFINITY));
                        if over {
                            ui.label(RichText::new("Le texte dépasse du bloc : agrandis-le ou réduis la taille.").size(12.0).color(DANGER));
                        }
                        ui.add(egui::Slider::new(size, 4.0..=200.0).logarithmic(true).text("Taille"));
                        ui.add(egui::Slider::new(leading, 0.8..=2.5).text("Interligne"));
                        egui::ComboBox::from_label("Police").selected_text(family.as_str()).show_ui(ui, |ui| {
                            for name in FONTS {
                                ui.selectable_value(family, name.to_string(), name);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.checkbox(bold, "Gras");
                            ui.selectable_value(align, 0, "Gauche");
                            ui.selectable_value(align, 1, "Centre");
                            ui.selectable_value(align, 2, "Droite");
                        });
                        color_row(ui, color);
                    }
                    Kind::Image { path, cover } => {
                        ui.label(RichText::new("IMAGE").size(11.0).color(MUTED));
                        ui.label(RichText::new(if path.is_empty() { "Aucune image" } else { path.as_str() }).size(11.0).color(MUTED));
                        if ui.button("Choisir une image…").clicked()
                            && let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg"]).pick_file()
                        {
                            *path = p.to_string_lossy().into_owned();
                        }
                        ui.horizontal(|ui| {
                            ui.selectable_value(cover, true, "Remplir");
                            ui.selectable_value(cover, false, "Ajuster");
                        });
                    }
                    Kind::Rect { fill, radius } => {
                        ui.label(RichText::new("RECTANGLE").size(11.0).color(MUTED));
                        color_row(ui, fill);
                        ui.add(egui::Slider::new(radius, 0.0..=100.0).text("Arrondi"));
                    }
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Premier plan").clicked() {
                        let fr = self.doc.pages[page].frames.remove(i);
                        self.doc.pages[page].frames.push(fr);
                        self.sel = Some(self.doc.pages[page].frames.len() - 1);
                    }
                    if ui.button("Arrière-plan").clicked() {
                        let fr = self.doc.pages[page].frames.remove(i);
                        self.doc.pages[page].frames.insert(0, fr);
                        self.sel = Some(0);
                    }
                    if ui.button(RichText::new("Supprimer").color(DANGER)).clicked() {
                        self.doc.pages[page].frames.remove(i);
                        self.sel = None;
                    }
                });
            });
        });
    }

    fn canvas(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().frame(EFrame::new().fill(BG)).show(root, |ui| {
            if self.fit_pending {
                let a = ui.available_size();
                self.zoom = ((a.y - 80.0) / self.doc.height).min((a.x - 80.0) / self.doc.width).clamp(0.25, 2.0);
                self.fit_pending = false;
            }
            let px = ui.ctx().pixels_per_point();
            let size = vec2(self.doc.width, self.doc.height) * self.zoom;
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                let avail = ui.available_size();
                let (outer, _) = ui.allocate_exact_size(vec2(avail.x.max(size.x + 80.0), (size.y + 80.0).max(avail.y)), Sense::hover());
                let page_rect = Rect::from_min_size(pos2(outer.center().x - size.x / 2.0, outer.min.y + 40.0), size);
                let scale = self.zoom * px;
                if self.rendered.as_ref().is_none_or(|(d, p, s)| *d != self.doc || *p != self.page || (*s - scale).abs() > 1e-3) {
                    let (svg, over) = self.svg(self.page, true);
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
                painter.rect_filled(page_rect.translate(vec2(4.0, 6.0)), CornerRadius::ZERO, Color32::from_black_alpha(90));
                if let Some(t) = &self.tex {
                    painter.image(t.id(), page_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
                let z = self.zoom;
                let to_doc = |p: Pos2| [(p.x - page_rect.min.x) / z, (p.y - page_rect.min.y) / z];
                let to_scr = |p: [f32; 2]| pos2(page_rect.min.x + p[0] * z, page_rect.min.y + p[1] * z);
                for (i, f) in self.doc.pages[self.page].frames.iter().enumerate() {
                    let r = Rect::from_min_max(to_scr([f.x, f.y]), to_scr([f.x + f.w, f.y + f.h]));
                    let on = self.sel == Some(i);
                    if on {
                        painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.5, ACCENT), StrokeKind::Middle);
                        for c in f.corners() {
                            painter.rect_filled(Rect::from_center_size(to_scr(c), Vec2::splat(8.0)), CornerRadius::same(2), ACCENT);
                        }
                    } else {
                        painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(0.5, Color32::from_rgba_unmultiplied(80, 140, 255, 90)), StrokeKind::Middle);
                    }
                    if self.overflow.get(i).copied().unwrap_or(false) {
                        let q = Rect::from_center_size(r.right_bottom() - vec2(6.0, 6.0), Vec2::splat(11.0));
                        painter.rect_filled(q, CornerRadius::ZERO, DANGER);
                        painter.text(q.center(), egui::Align2::CENTER_CENTER, "+", egui::FontId::monospace(10.0), Color32::WHITE);
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
                        self.drag_to(p);
                    }
                }
                if resp.drag_stopped() || resp.clicked() {
                    if let (Drag::Create { .. }, Some(i)) = (&self.drag, self.sel)
                        && self.frames()[i].w < 4.0
                    {
                        let kind = self.frames()[i].kind.clone();
                        let f = &mut self.frames()[i];
                        f.w = if matches!(kind, Kind::Text { .. }) { 240.0 } else { 160.0 };
                        f.h = if matches!(kind, Kind::Text { .. }) { 80.0 } else { 120.0 };
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
                self.sel = (0..self.frames().len()).rev().find(|&i| self.doc.pages[self.page].frames[i].contains(p, tol));
                if self.sel.is_some() {
                    self.drag = Drag::Move { last: p };
                }
            }
            tool => {
                let kind = match tool {
                    Tool::Text => Kind::Text { text: "Ton texte ici".into(), size: 12.0, family: "Segoe UI".into(), bold: false, color: [20, 20, 24, 255], align: 0, leading: 1.35 },
                    Tool::Image => Kind::Image { path: String::new(), cover: true },
                    _ => Kind::Rect { fill: [255, 120, 60, 255], radius: 0.0 },
                };
                let pick = matches!(kind, Kind::Image { .. });
                self.frames().push(Frame { x: p[0], y: p[1], w: 0.0, h: 0.0, kind });
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

    fn drag_to(&mut self, p: [f32; 2]) {
        let Some(i) = self.sel else { return };
        let page = self.page;
        let f = &mut self.doc.pages[page].frames[i];
        match &mut self.drag {
            Drag::Move { last } => {
                f.x += p[0] - last[0];
                f.y += p[1] - last[1];
                *last = p;
            }
            Drag::Create { start: a } | Drag::Resize { anchor: a } => {
                f.x = a[0].min(p[0]);
                f.y = a[1].min(p[1]);
                f.w = (p[0] - a[0]).abs();
                f.h = (p[1] - a[1]).abs();
            }
            Drag::None => {}
        }
    }
}

fn color_row(ui: &mut egui::Ui, c: &mut [u8; 4]) {
    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    ui.horizontal(|ui| {
        ui.label("Couleur");
        ui.color_edit_button_srgba(&mut col);
    });
    *c = col.to_srgba_unmultiplied();
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Page")
            .with_icon(Arc::new(studio_ui::app_window_icon("page")))
            .with_inner_size([1360.0, 920.0])
            .with_min_inner_size([960.0, 640.0]),
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
        let mut doc = Doc::a4();
        doc.pages[0].frames.push(Frame { x: 40.0, y: 40.0, w: 300.0, h: 100.0, kind: Kind::Text { text: "Bonjour {page}".into(), size: 20.0, family: "Arial".into(), bold: false, color: [0, 0, 0, 255], align: 0, leading: 1.2 } });
        doc.pages.push(Page::default());
        let m = |t: &str, f: &str, b: bool, s: f32| fonts.width(t, f, b, s);
        assert!(m("WWW", "Arial", false, 20.0) > m("iii", "Arial", false, 20.0));
        let opt = usvg::Options { fontdb: fonts.db.clone(), ..Default::default() };
        let mut document = krilla::Document::new();
        for i in 0..2 {
            let (svg, _) = doc.to_svg(i, &m, false);
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
