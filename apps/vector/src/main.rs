#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod model;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, Frame, Key, Margin, Painter, Pos2, Rect, RichText, Sense,
    Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2, pos2, vec2,
};
use model::{Doc, Geom, Item, Node, P};
use resvg::{tiny_skia, usvg};
use std::path::PathBuf;
use std::sync::Arc;
use studio_ui::{BG, CARD, CARD_HOVER, DANGER, MUTED, PANEL, TEXT};

const ACCENT: Color32 = Color32::from_rgb(255, 176, 46);
const MAX_RENDER: f32 = 4096.0;

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Pen,
    Rect,
    Ellipse,
    Text,
    Hand,
}

const TOOLS: [(Tool, &str, Key); 6] = [
    (Tool::Select, "Sélection (V)", Key::V),
    (Tool::Pen, "Plume (P) — clic : point, glisser : courbe, Entrée : terminer", Key::P),
    (Tool::Rect, "Rectangle (R)", Key::R),
    (Tool::Ellipse, "Ellipse (O)", Key::O),
    (Tool::Text, "Texte (T)", Key::T),
    (Tool::Hand, "Main (H, ou Espace)", Key::H),
];

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

struct VectorApp {
    doc: Doc,
    file: Option<PathBuf>,
    saved: Doc,
    undo: Vec<Doc>,
    redo: Vec<Doc>,
    sel: Option<usize>,
    tool: Tool,
    style: Style,
    zoom: f32,
    offset: Vec2,
    fit_pending: bool,
    drag: Drag,
    pen: Option<usize>,
    panel_edit: bool,
    fontdb: Arc<usvg::fontdb::Database>,
    tex: Option<TextureHandle>,
    rendered: Option<(Doc, f32)>,
    checker: TextureHandle,
    error: Option<String>,
    title: String,
    new_size: Option<(f32, f32)>,
}

impl VectorApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        studio_ui::apply_style(&cc.egui_ctx);
        cc.egui_ctx.all_styles_mut(|s| s.visuals.selection.bg_fill = ACCENT);
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        let doc = Doc::new(1200.0, 800.0);
        Self {
            saved: doc.clone(),
            doc,
            file: None,
            undo: Vec::new(),
            redo: Vec::new(),
            sel: None,
            tool: Tool::Select,
            style: Style { fill: Some(ACCENT), stroke: Some(Color32::from_rgb(20, 20, 24)), stroke_w: 2.0 },
            zoom: 1.0,
            offset: Vec2::ZERO,
            fit_pending: true,
            drag: Drag::None,
            pen: None,
            panel_edit: false,
            fontdb: Arc::new(db),
            tex: None,
            rendered: None,
            checker: checker(&cc.egui_ctx),
            error: None,
            title: String::new(),
            new_size: None,
        }
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
        let kind = match geom {
            Geom::Rect { .. } => "Rectangle",
            Geom::Ellipse { .. } => "Ellipse",
            Geom::Path { .. } => "Tracé",
            Geom::Text { .. } => "Texte",
        };
        let is_text = matches!(geom, Geom::Text { .. });
        Item {
            name: format!("{kind} {}", self.doc.items.len() + 1),
            geom,
            fill: if is_text { Some(self.style.stroke.unwrap_or(Color32::BLACK).to_array()) } else { self.style.fill.map(|c| c.to_array()) },
            stroke: if is_text { None } else { self.style.stroke.map(|c| c.to_array()) },
            stroke_w: self.style.stroke_w,
            opacity: 1.0,
            visible: true,
        }
    }

    fn render(&mut self, ctx: &egui::Context) {
        let scale = self.zoom.min(MAX_RENDER / self.doc.width.max(self.doc.height));
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
        let result = usvg::Tree::from_str(&self.doc.to_svg(), &opt)
            .map_err(|e| e.to_string())
            .and_then(|tree| {
                let mut pix = tiny_skia::Pixmap::new((self.doc.width * scale) as u32, (self.doc.height * scale) as u32)
                    .ok_or("taille invalide")?;
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
                self.file = Some(path);
                self.saved = self.doc.clone();
            }
            Err(e) => self.error = Some(format!("Enregistrement : {e}")),
        }
    }

    fn open(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Projet Vector", &["evec"]).pick_file() else { return };
        match std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Doc>(&b).map_err(|e| e.to_string())) {
            Ok(doc) => {
                self.saved = doc.clone();
                self.doc = doc;
                self.file = Some(path);
                self.undo.clear();
                self.redo.clear();
                self.sel = None;
                self.pen = None;
                self.fit_pending = true;
            }
            Err(e) => self.error = Some(format!("Ouverture : {e}")),
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

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        let redo = sc(M::COMMAND | M::SHIFT, Key::Z) || sc(M::COMMAND, Key::Y);
        let undo = sc(M::COMMAND, Key::Z);
        let dup = sc(M::COMMAND, Key::D);
        let save_as = sc(M::COMMAND | M::SHIFT, Key::S);
        let save = sc(M::COMMAND, Key::S);
        let open = sc(M::COMMAND, Key::O);
        let fit = sc(M::COMMAND, Key::Num0);
        if redo {
            self.redo();
        } else if undo {
            self.undo();
        }
        if dup {
            self.duplicate_selected();
        }
        if save_as {
            self.save(true);
        } else if save {
            self.save(false);
        }
        if open {
            self.open();
        }
        if fit {
            self.fit_pending = true;
        }
        let (keys, del, enter, nudge) = ctx.input(|i| {
            let mut keys = None;
            if !i.modifiers.command {
                for (t, _, k) in TOOLS {
                    if i.key_pressed(k) {
                        keys = Some(t);
                    }
                }
            }
            let step = if i.modifiers.shift { 10.0 } else { 1.0 };
            let mut n = [0.0, 0.0];
            if i.key_pressed(Key::ArrowLeft) { n[0] -= step; }
            if i.key_pressed(Key::ArrowRight) { n[0] += step; }
            if i.key_pressed(Key::ArrowUp) { n[1] -= step; }
            if i.key_pressed(Key::ArrowDown) { n[1] += step; }
            (
                keys,
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::Enter) || i.key_pressed(Key::Escape),
                n,
            )
        });
        if let Some(t) = keys {
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
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        self.shortcuts(ctx);
        let name = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = format!("Vector — {name}{}", if self.doc != self.saved { " •" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.menu(root);
        self.status(root);
        self.toolbar(root);
        self.panel(root);
        self.canvas(root);
        self.new_dialog(ctx);
    }
}

impl VectorApp {
    fn menu(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("menu")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 6)))
            .show(root, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    studio_ui::app_icon(ui, 22.0, "vector", ACCENT);
                    ui.add_space(4.0);
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouveau…").clicked() {
                            self.new_size = Some((self.doc.width, self.doc.height));
                        }
                        if ui.button("Ouvrir…    Ctrl+O").clicked() {
                            self.open();
                        }
                        if ui.button("Enregistrer    Ctrl+S").clicked() {
                            self.save(false);
                        }
                        if ui.button("Enregistrer sous…    Ctrl+Maj+S").clicked() {
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
                    });
                    ui.menu_button("Édition", |ui| {
                        if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler    Ctrl+Z")).clicked() {
                            self.undo();
                        }
                        if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir    Ctrl+Y")).clicked() {
                            self.redo();
                        }
                        ui.separator();
                        if ui.add_enabled(self.sel.is_some(), egui::Button::new("Dupliquer    Ctrl+D")).clicked() {
                            self.duplicate_selected();
                        }
                        if ui.add_enabled(self.sel.is_some(), egui::Button::new("Supprimer    Suppr")).clicked() {
                            self.delete_selected();
                        }
                    });
                    ui.menu_button("Objet", |ui| {
                        let has = self.sel.is_some();
                        if ui.add_enabled(has, egui::Button::new("Premier plan")).clicked() {
                            self.reorder(i32::MAX);
                        }
                        if ui.add_enabled(has, egui::Button::new("Avancer")).clicked() {
                            self.reorder(1);
                        }
                        if ui.add_enabled(has, egui::Button::new("Reculer")).clicked() {
                            self.reorder(-1);
                        }
                        if ui.add_enabled(has, egui::Button::new("Arrière-plan")).clicked() {
                            self.reorder(i32::MIN);
                        }
                        ui.separator();
                        for (label, axis) in [("Miroir horizontal", 0), ("Miroir vertical", 1)] {
                            if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                                self.flip(axis);
                            }
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

    fn status(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("status")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 5)))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                    ui.label(small(format!("Plan de travail {} × {}", self.doc.width, self.doc.height)));
                    ui.separator();
                    ui.label(small(format!("{:.0} %", self.zoom * 100.0)));
                    ui.separator();
                    ui.label(small(format!("{} objet(s)", self.doc.items.len())));
                    if self.tool == Tool::Pen {
                        ui.separator();
                        ui.label(small("Plume : clic = point, glisser = courbe, clic sur le 1er point = fermer, Entrée = terminer".into()));
                    }
                    if let Some(e) = self.error.clone() {
                        ui.separator();
                        ui.label(RichText::new(e).size(12.0).color(DANGER));
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
                    tool_icon(ui.painter(), rect.shrink(11.0), tool, if on { ACCENT } else { TEXT });
                    if resp.on_hover_text(tip).clicked() {
                        self.finish_pen();
                        self.tool = tool;
                    }
                }
            });
    }

    fn panel(&mut self, root: &mut egui::Ui) {
        let before = self.doc.clone();
        egui::Panel::right("props")
            .resizable(false)
            .exact_size(280.0)
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::same(12)))
            .show(root, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.sel {
                        Some(i) => item_props(ui, &mut self.doc.items[i]),
                        None => {
                            heading(ui, "Style par défaut");
                            color_opt(ui, "Remplissage", &mut self.style.fill);
                            color_opt(ui, "Contour", &mut self.style.stroke);
                            ui.add(egui::Slider::new(&mut self.style.stroke_w, 0.0..=50.0).text("Épaisseur"));
                            ui.add_space(12.0);
                            heading(ui, "Plan de travail");
                            ui.add(egui::DragValue::new(&mut self.doc.width).range(1.0..=20000.0).prefix("Largeur "));
                            ui.add(egui::DragValue::new(&mut self.doc.height).range(1.0..=20000.0).prefix("Hauteur "));
                            let mut bg = self.doc.background.map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]));
                            color_opt(ui, "Fond", &mut bg);
                            self.doc.background = bg.map(|c| c.to_srgba_unmultiplied());
                        }
                    }
                    ui.add_space(14.0);
                    heading(ui, "Calques");
                    let mut select = None;
                    let mut toggle = None;
                    for i in (0..self.doc.items.len()).rev() {
                        let it = &self.doc.items[i];
                        let on = self.sel == Some(i);
                        let row = Frame::new()
                            .fill(if on { CARD_HOVER } else { CARD })
                            .stroke(Stroke::new(1.0, if on { ACCENT.gamma_multiply(0.6) } else { Color32::TRANSPARENT }))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::symmetric(8, 5))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let (eye, er) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                                    ui.painter().circle_stroke(eye.center(), 5.5, Stroke::new(1.5, if it.visible { TEXT } else { MUTED }));
                                    if it.visible {
                                        ui.painter().circle_filled(eye.center(), 2.2, TEXT);
                                    }
                                    if er.clicked() {
                                        toggle = Some(i);
                                    }
                                    ui.label(RichText::new(&it.name).color(if on { TEXT } else { MUTED }));
                                });
                            })
                            .response;
                        if row.interact(Sense::click()).clicked() {
                            select = Some(i);
                        }
                        ui.add_space(3.0);
                    }
                    if let Some(i) = toggle {
                        self.doc.items[i].visible = !self.doc.items[i].visible;
                    }
                    if let Some(i) = select {
                        self.finish_pen();
                        self.sel = Some(i);
                    }
                });
            });
        if self.doc != before && !self.panel_edit {
            self.undo.push(before);
            self.redo.clear();
            self.panel_edit = true;
        }
        if root.ctx().input(|i| !i.pointer.any_down()) && !root.ctx().egui_wants_keyboard_input() {
            self.panel_edit = false;
        }
    }

    fn canvas(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().frame(Frame::new().fill(BG)).show(root, |ui| {
            let area = ui.max_rect();
            let resp = ui.allocate_rect(area, Sense::click_and_drag());
            let (w, h) = (self.doc.width, self.doc.height);
            if self.fit_pending {
                self.zoom = ((area.width() - 80.0) / w).min((area.height() - 80.0) / h).clamp(0.02, 4.0);
                self.offset = Vec2::ZERO;
                self.fit_pending = false;
            }
            let (scroll, pinch, space, pointer, pressed, released, middle, delta, shift) = ui.input(|i| {
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
                )
            });
            let origin = |zoom: f32, offset: Vec2| area.center() + offset - vec2(w, h) * zoom / 2.0;
            if resp.hovered() {
                let f = pinch * (scroll / 400.0).exp();
                if (f - 1.0).abs() > 1e-4
                    && let Some(p) = pointer
                {
                    let docp = (p - origin(self.zoom, self.offset)) / self.zoom;
                    let nz = (self.zoom * f).clamp(0.02, 64.0);
                    self.offset = p.to_vec2() - docp * nz + vec2(w, h) * nz / 2.0 - area.center().to_vec2();
                    self.zoom = nz;
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

            if !panning && !space && let Some(sp) = pointer {
                let p = to_doc(sp);
                if pressed && resp.hovered() {
                    self.press(p, tol);
                } else if resp.dragged() || matches!(self.drag, Drag::PenHandle { .. }) && ui.input(|i| i.pointer.primary_down()) {
                    self.drag_to(p, shift);
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
            painter.rect_filled(art.expand(1.0), CornerRadius::ZERO, Color32::from_black_alpha(140));
            if self.doc.background.is_none_or(|c| c[3] < 255) {
                painter.image(self.checker.id(), art, Rect::from_min_max(pos2(0.0, 0.0), pos2(art.width() / 16.0, art.height() / 16.0)), Color32::WHITE);
            }
            if let Some(t) = &self.tex {
                painter.image(t.id(), art, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            self.overlay(&painter, &to_scr);

            if resp.hovered() {
                ui.ctx().set_cursor_icon(match self.tool {
                    _ if panning || space => egui::CursorIcon::Grab,
                    Tool::Hand => egui::CursorIcon::Grab,
                    Tool::Select => egui::CursorIcon::Default,
                    Tool::Text => egui::CursorIcon::Text,
                    _ => egui::CursorIcon::Crosshair,
                });
            }
        });
    }

    fn press(&mut self, p: P, tol: f32) {
        match self.tool {
            Tool::Select => {
                if let Some(i) = self.sel {
                    let b = self.doc.items[i].bbox();
                    let corners = b.corners();
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
            Tool::Rect | Tool::Ellipse => {
                self.checkpoint();
                let geom = if self.tool == Tool::Rect {
                    Geom::Rect { x: p[0], y: p[1], w: 0.0, h: 0.0, r: 0.0 }
                } else {
                    Geom::Ellipse { cx: p[0], cy: p[1], rx: 0.0, ry: 0.0 }
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
                        self.pen = None;
                        self.drag = Drag::None;
                        return;
                    }
                    nodes.push(Node::corner(p));
                    self.drag = Drag::PenHandle { node: nodes.len() - 1 };
                }
            }
            Tool::Hand => {}
        }
    }

    fn drag_to(&mut self, p: P, shift: bool) {
        let Some(i) = self.sel else { return };
        let it = &mut self.doc.items[i];
        match &mut self.drag {
            Drag::Move { last } => {
                it.translate([p[0] - last[0], p[1] - last[1]]);
                *last = p;
            }
            Drag::Create { start } => {
                let (mut dx, mut dy) = (p[0] - start[0], p[1] - start[1]);
                if shift {
                    let m = dx.abs().max(dy.abs());
                    dx = m * dx.signum();
                    dy = m * dy.signum();
                }
                let (x, y) = (start[0].min(start[0] + dx), start[1].min(start[1] + dy));
                let (w, h) = (dx.abs(), dy.abs());
                it.geom = match it.geom {
                    Geom::Rect { r, .. } => Geom::Rect { x, y, w, h, r },
                    _ => Geom::Ellipse { cx: x + w / 2.0, cy: y + h / 2.0, rx: w / 2.0, ry: h / 2.0 },
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
        painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, ACCENT), StrokeKind::Middle);
        if let Geom::Path { nodes, .. } = &it.geom {
            for n in nodes {
                for c in [n.cin, n.cout] {
                    if c != n.p {
                        painter.line_segment([to_scr(n.p), to_scr(c)], Stroke::new(1.0, ACCENT.gamma_multiply(0.7)));
                        painter.circle_filled(to_scr(c), 3.5, ACCENT);
                    }
                }
                let q = Rect::from_center_size(to_scr(n.p), Vec2::splat(8.0));
                painter.rect_filled(q, CornerRadius::ZERO, Color32::WHITE);
                painter.rect_stroke(q, CornerRadius::ZERO, Stroke::new(1.0, ACCENT), StrokeKind::Inside);
            }
        }
        if self.pen.is_none() {
            for c in b.corners() {
                let q = Rect::from_center_size(to_scr(c), Vec2::splat(9.0));
                painter.rect_filled(q, CornerRadius::same(2), ACCENT);
            }
        }
    }

    fn new_dialog(&mut self, ctx: &egui::Context) {
        let Some((mut w, mut h)) = self.new_size else { return };
        let mut done = None;
        egui::Window::new("Nouveau dessin")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (l, pw, ph) in [("Full HD", 1920.0, 1080.0), ("Logo", 1000.0, 1000.0), ("Icône", 512.0, 512.0), ("Bannière", 1500.0, 500.0), ("A4", 794.0, 1123.0)] {
                        if ui.button(l).clicked() {
                            w = pw;
                            h = ph;
                        }
                    }
                });
                ui.add(egui::DragValue::new(&mut w).range(1.0..=20000.0).prefix("Largeur "));
                ui.add(egui::DragValue::new(&mut h).range(1.0..=20000.0).prefix("Hauteur "));
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Créer").strong().color(Color32::from_rgb(14, 15, 20))).fill(ACCENT)).clicked() {
                        done = Some(true);
                    }
                    if ui.button("Annuler").clicked() {
                        done = Some(false);
                    }
                });
            });
        match done {
            Some(true) => {
                self.doc = Doc::new(w, h);
                self.saved = self.doc.clone();
                self.file = None;
                self.undo.clear();
                self.redo.clear();
                self.sel = None;
                self.pen = None;
                self.fit_pending = true;
                self.new_size = None;
            }
            Some(false) => self.new_size = None,
            None => self.new_size = Some((w, h)),
        }
    }
}

fn item_props(ui: &mut egui::Ui, it: &mut Item) {
    heading(ui, "Objet");
    ui.text_edit_singleline(&mut it.name);
    ui.add_space(6.0);
    let mut fill = it.fill.map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]));
    color_opt(ui, "Remplissage", &mut fill);
    it.fill = fill.map(|c| c.to_srgba_unmultiplied());
    let mut stroke = it.stroke.map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]));
    color_opt(ui, "Contour", &mut stroke);
    it.stroke = stroke.map(|c| c.to_srgba_unmultiplied());
    ui.add(egui::Slider::new(&mut it.stroke_w, 0.0..=50.0).text("Épaisseur"));
    let mut op = it.opacity * 100.0;
    if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).suffix(" %").text("Opacité")).changed() {
        it.opacity = op / 100.0;
    }
    ui.add_space(8.0);
    match &mut it.geom {
        Geom::Rect { x, y, w, h, r } => {
            heading(ui, "Rectangle");
            xy(ui, x, y);
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(w).range(0.0..=f32::MAX).prefix("L "));
                ui.add(egui::DragValue::new(h).range(0.0..=f32::MAX).prefix("H "));
            });
            ui.add(egui::Slider::new(r, 0.0..=w.min(*h) / 2.0).text("Arrondi"));
        }
        Geom::Ellipse { cx, cy, rx, ry } => {
            heading(ui, "Ellipse");
            xy(ui, cx, cy);
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(rx).range(0.0..=f32::MAX).prefix("Rx "));
                ui.add(egui::DragValue::new(ry).range(0.0..=f32::MAX).prefix("Ry "));
            });
        }
        Geom::Path { nodes, closed } => {
            heading(ui, "Tracé");
            ui.label(RichText::new(format!("{} points", nodes.len())).color(MUTED));
            ui.checkbox(closed, "Fermé");
        }
        Geom::Text { x, y, text, size, family, bold } => {
            heading(ui, "Texte");
            ui.add(egui::TextEdit::multiline(text).desired_rows(3));
            xy(ui, x, y);
            ui.add(egui::Slider::new(size, 4.0..=600.0).logarithmic(true).text("Taille"));
            egui::ComboBox::from_label("Police").selected_text(family.as_str()).show_ui(ui, |ui| {
                for f in ["Segoe UI", "Arial", "Georgia", "Times New Roman", "Consolas", "Impact", "Verdana", "Trebuchet MS"] {
                    ui.selectable_value(family, f.to_string(), f);
                }
            });
            ui.checkbox(bold, "Gras");
        }
    }
}

fn xy(ui: &mut egui::Ui, x: &mut f32, y: &mut f32) {
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(x).prefix("X "));
        ui.add(egui::DragValue::new(y).prefix("Y "));
    });
}

fn color_opt(ui: &mut egui::Ui, label: &str, c: &mut Option<Color32>) {
    ui.horizontal(|ui| {
        let mut on = c.is_some();
        if ui.checkbox(&mut on, label).changed() {
            *c = on.then_some(c.unwrap_or(Color32::BLACK));
        }
        if let Some(col) = c {
            ui.color_edit_button_srgba(col);
        }
    });
}

fn heading(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(t.to_uppercase()).size(11.0).color(MUTED));
    ui.add_space(4.0);
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
            let v = if (x / 8 + y / 8) % 2 == 0 { 58 } else { 44 };
            px.extend_from_slice(&[v, v, v, 255]);
        }
    }
    ctx.load_texture(
        "checker",
        egui::ColorImage::from_rgba_unmultiplied([16, 16], &px),
        TextureOptions { wrap_mode: egui::TextureWrapMode::Repeat, ..TextureOptions::NEAREST },
    )
}

fn tool_icon(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    let s = Stroke::new(2.0, c);
    match tool {
        Tool::Select => {
            p.add(egui::Shape::convex_polygon(vec![at(0.15, 0.0), at(0.85, 0.6), at(0.5, 0.62), at(0.3, 1.0)], c, Stroke::NONE));
        }
        Tool::Pen => {
            p.add(egui::Shape::convex_polygon(vec![at(0.5, 0.0), at(0.85, 0.55), at(0.5, 1.0), at(0.15, 0.55)], Color32::TRANSPARENT, s));
            p.line_segment([at(0.5, 0.35), at(0.5, 0.95)], Stroke::new(1.5, c));
        }
        Tool::Rect => {
            p.rect_stroke(Rect::from_min_max(at(0.05, 0.15), at(0.95, 0.85)), CornerRadius::same(2), s, StrokeKind::Middle);
        }
        Tool::Ellipse => {
            p.circle_stroke(r.center(), r.width() * 0.45, s);
        }
        Tool::Text => {
            p.line_segment([at(0.1, 0.1), at(0.9, 0.1)], Stroke::new(2.5, c));
            p.line_segment([at(0.5, 0.1), at(0.5, 0.95)], Stroke::new(2.5, c));
        }
        Tool::Hand => {
            let m = r.center();
            let d = r.width() * 0.5;
            let k = r.width() * 0.2;
            p.line_segment([m - vec2(d, 0.0), m + vec2(d, 0.0)], s);
            p.line_segment([m - vec2(0.0, d), m + vec2(0.0, d)], s);
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
            .with_title("Vector")
            .with_icon(Arc::new(studio_ui::window_icon()))
            .with_inner_size([1400.0, 880.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("Vector", options, Box::new(|cc| Ok(Box::new(VectorApp::new(cc)))))
}
