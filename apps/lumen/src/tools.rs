use crate::develop::{self, Dab, Mask, Part, PartKind, RedEye, Settings, Spot, SpotMode, View as Geo};
use studio_ui::i18n::tl;
use crate::widgets::{caption, short};
use crate::{Lumen, display_name};
use eframe::egui::{self, Align, Align2, Color32, CornerRadius, FontId, Layout, Painter, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2, pos2, vec2};
use studio_ui::pro::{self, BLUE, INK, MUTED, semibold};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    None,
    Crop,
    Straighten,
    Remove,
    Heal,
    Clone,
    RedEye,
    Linear,
    Radial,
    Brush,
    Sky,
    Subject,
    WbPicker,
    Target,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Drag {
    None,
    Pan,
    CropMove,
    CropEdge(u8),
    CropRotate(f32, f32),
    Ruler(Pos2),
    SpotDest(usize),
    SpotSrc(usize),
    NewSpot(usize),
    EyeMove(usize),
    NewEye(usize, Pos2),
    Handle(usize, usize, u8),
    NewLinear(usize),
    NewRadial(usize),
    Paint(f32, f32),
    Target(f32, f32, f32),
    Split,
}

pub const GROUPS: [&[(Tool, &str, &str)]; 7] = [
    &[(Tool::Crop, "Recadrage", "R"), (Tool::Straighten, "Redressement (règle)", "R")],
    &[(Tool::Remove, "Suppression de taches", "Q"), (Tool::Heal, "Correcteur", "Q"), (Tool::Clone, "Clonage", "Q")],
    &[(Tool::RedEye, "Yeux rouges", "E")],
    &[(Tool::Linear, "Filtre gradué", "M"), (Tool::Radial, "Filtre radial", "Maj+M"), (Tool::Brush, "Pinceau de réglage", "K"), (Tool::Sky, "Sélectionner le ciel", ""), (Tool::Subject, "Sélectionner le sujet", "")],
    &[(Tool::WbPicker, "Pipette de balance des blancs", "W")],
    &[(Tool::Target, "Réglage ciblé", "T")],
    &[(Tool::None, "Main et zoom", "H")],
];

pub const OVERLAYS: [&str; 5] = ["Aucune", "Règle des tiers", "Grille", "Nombre d'or", "Diagonales"];
pub const TARGET_MODES: [&str; 4] = ["Courbe", "Teinte", "Saturation", "Luminance"];

pub fn draw_tool(p: &Painter, r: Rect, c: Color32, t: Tool) {
    let s = Stroke::new(1.6, c);
    let at = |x: f32, y: f32| r.min + vec2(x * r.width(), y * r.height());
    match t {
        Tool::Crop => {
            p.add(egui::Shape::line(vec![at(0.25, 0.0), at(0.25, 0.75), at(1.0, 0.75)], s));
            p.add(egui::Shape::line(vec![at(0.0, 0.25), at(0.75, 0.25), at(0.75, 1.0)], s));
        }
        Tool::Straighten => {
            p.line_segment([at(0.0, 0.8), at(1.0, 0.2)], s);
            for k in 0..4 {
                let f = 0.15 + k as f32 * 0.23;
                let base = at(f, 0.8 - f * 0.6);
                p.line_segment([base, base + vec2(2.5, 4.0)], Stroke::new(1.2, c));
            }
        }
        Tool::Remove | Tool::Heal => {
            p.circle_stroke(r.center(), r.width() * 0.42, s);
            if t == Tool::Heal {
                p.line_segment([at(0.5, 0.28), at(0.5, 0.72)], s);
                p.line_segment([at(0.28, 0.5), at(0.72, 0.5)], s);
            } else {
                p.circle_filled(r.center(), r.width() * 0.14, c);
            }
        }
        Tool::Clone => {
            p.rect_stroke(Rect::from_min_max(at(0.3, 0.5), at(0.7, 1.0)), CornerRadius::same(2), s, egui::StrokeKind::Middle);
            p.line_segment([at(0.5, 0.5), at(0.5, 0.2)], s);
            p.circle_stroke(at(0.5, 0.15), r.width() * 0.13, s);
        }
        Tool::RedEye => {
            p.add(egui::Shape::line((0..=16).map(|k| { let a = k as f32 / 16.0 * std::f32::consts::PI; at(0.5 - a.cos() * 0.5, 0.5 - a.sin() * 0.3) }).collect(), s));
            p.add(egui::Shape::line((0..=16).map(|k| { let a = k as f32 / 16.0 * std::f32::consts::PI; at(0.5 - a.cos() * 0.5, 0.5 + a.sin() * 0.3) }).collect(), s));
            p.circle_filled(r.center(), r.width() * 0.16, Color32::from_rgb(255, 69, 58));
        }
        Tool::Linear => {
            for (k, y) in [0.2f32, 0.5, 0.8].iter().enumerate() {
                p.line_segment([at(0.0, *y), at(1.0, *y)], Stroke::new(if k == 1 { 1.6 } else { 1.0 }, c));
            }
            p.circle_filled(at(0.5, 0.5), 2.5, c);
        }
        Tool::Radial => {
            p.circle_stroke(r.center(), r.width() * 0.46, s);
            p.circle_stroke(r.center(), r.width() * 0.25, Stroke::new(1.0, c.gamma_multiply(0.6)));
            p.circle_filled(r.center(), 2.5, c);
        }
        Tool::Brush => {
            p.line_segment([at(0.95, 0.05), at(0.45, 0.55)], Stroke::new(2.2, c));
            p.add(egui::Shape::convex_polygon(vec![at(0.45, 0.5), at(0.5, 0.55), at(0.3, 0.95), at(0.0, 1.0), at(0.15, 0.7)], c, Stroke::NONE));
        }
        Tool::Sky => {
            p.circle_stroke(at(0.32, 0.4), r.width() * 0.18, s);
            p.add(egui::Shape::line(vec![at(0.0, 1.0), at(0.35, 0.68), at(0.55, 0.85), at(0.8, 0.6), at(1.0, 0.8)], s));
        }
        Tool::Subject => {
            p.circle_stroke(at(0.5, 0.3), r.width() * 0.2, s);
            p.add(egui::Shape::line((0..=12).map(|k| { let a = k as f32 / 12.0 * std::f32::consts::PI; at(0.5 - a.cos() * 0.38, 1.0 - a.sin() * 0.42) }).collect(), s));
        }
        Tool::WbPicker => {
            p.line_segment([at(0.1, 0.9), at(0.62, 0.38)], Stroke::new(2.0, c));
            p.circle_filled(at(0.75, 0.25), r.width() * 0.18, c);
            p.line_segment([at(0.5, 0.25), at(0.75, 0.5)], s);
        }
        Tool::Target => {
            p.circle_stroke(r.center(), r.width() * 0.4, s);
            p.line_segment([at(0.5, 0.0), at(0.5, 1.0)], Stroke::new(1.2, c));
            p.line_segment([at(0.0, 0.5), at(1.0, 0.5)], Stroke::new(1.2, c));
        }
        Tool::None => {
            p.add(egui::Shape::line(vec![at(0.3, 0.95), at(0.12, 0.55), at(0.2, 0.45), at(0.32, 0.6), at(0.32, 0.12), at(0.42, 0.06), at(0.5, 0.15), at(0.5, 0.45)], s));
            p.add(egui::Shape::line(vec![at(0.5, 0.15), at(0.62, 0.1), at(0.7, 0.2), at(0.7, 0.45)], s));
            p.add(egui::Shape::line(vec![at(0.7, 0.25), at(0.82, 0.22), at(0.88, 0.32), at(0.85, 0.7), at(0.7, 0.95)], s));
        }
    }
}

pub fn part_name(k: PartKind) -> &'static str {
    match k {
        PartKind::Linear => "Filtre gradué",
        PartKind::Radial => "Filtre radial",
        PartKind::Brush => "Pinceau",
        PartKind::Sky => "Ciel",
        PartKind::Subject => "Sujet",
        PartKind::Luminance => "Plage de luminance",
        PartKind::Color => "Plage de couleur",
        PartKind::All => "Image entière",
    }
}

fn insets_to_view(c: [f32; 4], s: &Settings) -> [f32; 4] {
    let mut c = c;
    if s.flip {
        c.swap(0, 2);
    }
    if s.flip_v {
        c.swap(1, 3);
    }
    for _ in 0..s.rotate % 4 {
        c = [c[3], c[0], c[1], c[2]];
    }
    c
}

fn insets_from_view(v: [f32; 4], s: &Settings) -> [f32; 4] {
    let mut c = v;
    for _ in 0..s.rotate % 4 {
        c = [c[1], c[2], c[3], c[0]];
    }
    if s.flip_v {
        c.swap(1, 3);
    }
    if s.flip {
        c.swap(0, 2);
    }
    c
}

impl Lumen {
    pub fn shortcut_tool(&mut self, group: usize, shift: bool, ctx: &egui::Context) {
        let target = match group {
            0 => self.cycle(&GROUPS[0]),
            1 => self.cycle(&GROUPS[1]),
            2 => Tool::RedEye,
            3 => {
                if shift {
                    Tool::Radial
                } else if self.tool == Tool::Linear {
                    Tool::Radial
                } else if self.tool == Tool::Radial {
                    Tool::Brush
                } else {
                    Tool::Linear
                }
            }
            4 => Tool::Brush,
            5 => Tool::WbPicker,
            6 => Tool::Target,
            _ => Tool::None,
        };
        let t = if self.tool == target && GROUPS.iter().all(|g| g.len() == 1 || !g.iter().any(|x| x.0 == target)) && target != Tool::None { Tool::None } else { target };
        self.set_tool(t, ctx);
    }

    fn cycle(&self, group: &[(Tool, &str, &str)]) -> Tool {
        let tools: Vec<Tool> = group.iter().map(|g| g.0).filter(|t| !matches!(t, Tool::Sky | Tool::Subject)).collect();
        match tools.iter().position(|t| *t == self.tool) {
            Some(k) => tools[(k + 1) % tools.len()],
            None => tools[0],
        }
    }

    pub fn set_tool(&mut self, t: Tool, ctx: &egui::Context) {
        let Some(i) = self.current else { return };
        self.drag = crate::Drag::None;
        match t {
            Tool::Sky | Tool::Subject => {
                let mut s = self.settings(i);
                let kind = if t == Tool::Sky { PartKind::Sky } else { PartKind::Subject };
                s.masks.push(Mask { name: part_name(kind).into(), parts: vec![Part::new(kind)], ..Default::default() });
                self.sel_mask = Some(s.masks.len() - 1);
                self.sel_part = 0;
                self.set_settings(i, s);
                self.show_overlay = true;
                self.tool = Tool::None;
                self.notify(format!("Masque « {} » créé : règle ses curseurs dans le panneau Masques", part_name(kind)), ctx);
            }
            Tool::Brush => {
                self.tool = Tool::Brush;
                self.ensure_brush_target(i);
            }
            _ => self.tool = t,
        }
    }

    fn ensure_brush_target(&mut self, i: usize) {
        let mut s = self.settings(i);
        if let Some(m) = self.sel_mask.filter(|m| *m < s.masks.len()) {
            if let Some(p) = s.masks[m].parts.iter().rposition(|p| p.kind == PartKind::Brush) {
                self.sel_part = p;
                return;
            }
        }
        s.masks.push(Mask { name: "Pinceau".into(), parts: vec![Part::new(PartKind::Brush)], ..Default::default() });
        self.sel_mask = Some(s.masks.len() - 1);
        self.sel_part = 0;
        self.set_settings(i, s);
    }

    pub fn tool_strip(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.spacing_mut().item_spacing.y = 4.0;
        for (k, group) in GROUPS.iter().enumerate() {
            if k == 6 {
                ui.add_space(6.0);
                let (r, _) = ui.allocate_exact_size(vec2(36.0, 1.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::ZERO, Color32::from_white_alpha(20));
                ui.add_space(6.0);
            }
            let mut cur = self.tool;
            if pro::tool_group(ui, 38.0, &mut cur, group, draw_tool) && cur != self.tool {
                self.set_tool(cur, ctx);
            }
        }
    }

    pub fn option_bar(&mut self, ui: &mut Ui, i: usize) {
        ui.spacing_mut().slider_width = 90.0;
        ui.spacing_mut().item_spacing.x = 8.0;
        let name = GROUPS.iter().flat_map(|g| g.iter()).find(|g| g.0 == self.tool).map(|g| g.1).unwrap_or("Main et zoom");
        ui.label(RichText::new(name).font(semibold(12.0)).color(INK));
        pro::toolbar_sep(ui);
        let mut s = self.settings(i);
        let before = s.clone();
        match self.tool {
            Tool::None => {
                studio_ui::pro::tip(ui, tl("Glisse pour te déplacer quand le zoom est actif · double-clic pour basculer le zoom"));
            }
            Tool::Crop => {
                ui.label(RichText::new(tl("Format")).size(12.0).color(MUTED));
                egui::ComboBox::from_id_salt("crop-ratio").width(72.0).selected_text(develop::RATIOS[self.crop_ratio].0).show_ui(ui, |ui| {
                    for (k, (n, _)) in develop::RATIOS.iter().enumerate() {
                        if ui.selectable_value(&mut self.crop_ratio, k, *n).clicked() {
                            self.apply_ratio(&mut s);
                        }
                    }
                });
                ui.checkbox(&mut self.crop_lock, tl("Verrouiller"));
                if ui.small_button("⇄").on_hover_text(tl("Inverser l'orientation du format")).clicked() {
                    let r = develop::RATIOS[self.crop_ratio].1;
                    if let Some(k) = develop::RATIOS.iter().position(|x| r > 0.0 && (x.1 - 1.0 / r).abs() < 0.01) {
                        self.crop_ratio = k;
                        self.apply_ratio(&mut s);
                    }
                }
                ui.label(RichText::new(tl("Superposition")).size(12.0).color(MUTED));
                egui::ComboBox::from_id_salt("crop-ov").width(110.0).selected_text(OVERLAYS[self.crop_overlay]).show_ui(ui, |ui| {
                    for (k, n) in OVERLAYS.iter().enumerate() {
                        ui.selectable_value(&mut self.crop_overlay, k, *n);
                    }
                });
                ui.label(RichText::new(tl("Angle")).size(12.0).color(MUTED));
                ui.add(egui::Slider::new(&mut s.straighten, -45.0..=45.0).step_by(0.1).suffix("°"));
                if pro::ghost(ui, tl("Auto")).on_hover_text(tl("Redresse l'horizon automatiquement")).clicked()
                    && let Some((_, src)) = &self.source
                {
                    s.straighten = develop::auto_straighten(src);
                }
                if pro::ghost(ui, tl("Réinitialiser")).clicked() {
                    (s.crop, s.straighten) = ([0.0; 4], 0.0);
                }
                if pro::primary(ui, tl("Terminé")).clicked() {
                    self.tool = Tool::None;
                }
            }
            Tool::Straighten => {
                caption(ui, tl("Trace une ligne le long de l'horizon ou d'une verticale"));
                ui.add(egui::Slider::new(&mut s.straighten, -45.0..=45.0).step_by(0.1).suffix("°"));
                if pro::ghost(ui, tl("Auto")).clicked()
                    && let Some((_, src)) = &self.source
                {
                    s.straighten = develop::auto_straighten(src);
                }
                if pro::ghost(ui, tl("Réinitialiser")).clicked() {
                    s.straighten = 0.0;
                }
            }
            Tool::Remove | Tool::Heal | Tool::Clone => {
                ui.add(egui::Slider::new(&mut self.spot_size, 3.0..=200.0).text("Taille"));
                ui.add(egui::Slider::new(&mut self.spot_feather, 0.0..=100.0).text("Contour"));
                ui.add(egui::Slider::new(&mut self.spot_opacity, 1.0..=100.0).text("Opacité"));
                if let Some(k) = self.sel_spot.filter(|k| *k < s.spots.len()) {
                    pro::toolbar_sep(ui);
                    let sp = &mut s.spots[k];
                    egui::ComboBox::from_id_salt("spot-mode").width(96.0).selected_text(spot_mode_name(sp.mode)).show_ui(ui, |ui| {
                        for m in [SpotMode::Remove, SpotMode::Heal, SpotMode::Clone] {
                            ui.selectable_value(&mut sp.mode, m, spot_mode_name(m));
                        }
                    });
                    let mut size = sp.r * 1000.0;
                    if ui.add(egui::DragValue::new(&mut size).range(3.0..=200.0).prefix("⌀ ")).changed() {
                        sp.r = size / 1000.0;
                    }
                    if pro::ghost(ui, tl("Nouvelle source")).on_hover_text(tl("Cherche une autre zone de remplacement")).clicked()
                        && let Some((_, src)) = &self.source
                    {
                        let (dx, dy) = (sp.sx - sp.x, sp.sy - sp.y);
                        let (sx, sy) = develop::find_source(src, sp.x + dy * 0.5, sp.y - dx * 0.5, sp.r);
                        let (sx, sy) = if (sx - sp.sx).abs() + (sy - sp.sy).abs() < 1e-3 { (sp.x - dx, sp.y - dy) } else { (sx + sp.x - (sp.x + dy * 0.5), sy + sp.y - (sp.y - dx * 0.5)) };
                        (sp.sx, sp.sy) = (sx.clamp(0.0, 1.0), sy.clamp(0.0, 1.0));
                    }
                    if pro::ghost(ui, tl("Supprimer")).clicked() {
                        s.spots.remove(k);
                        self.sel_spot = None;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !s.spots.is_empty() && pro::ghost(ui, &format!("Tout effacer ({})", s.spots.len())).clicked() {
                        s.spots.clear();
                        self.sel_spot = None;
                    }
                });
            }
            Tool::RedEye => {
                ui.add(egui::Slider::new(&mut self.eye_pupil, 0.0..=100.0).text("Pupille"));
                ui.add(egui::Slider::new(&mut self.eye_darken, 0.0..=100.0).text("Assombrir"));
                if let Some(k) = self.sel_eye.filter(|k| *k < s.red_eyes.len()) {
                    (s.red_eyes[k].pupil, s.red_eyes[k].darken) = (self.eye_pupil / 100.0, self.eye_darken / 100.0);
                    if pro::ghost(ui, tl("Supprimer")).clicked() {
                        s.red_eyes.remove(k);
                        self.sel_eye = None;
                    }
                }
                caption(ui, tl("Clique sur l'œil ou glisse pour tracer le cercle"));
                if !s.red_eyes.is_empty() && pro::ghost(ui, tl("Tout effacer")).clicked() {
                    s.red_eyes.clear();
                    self.sel_eye = None;
                }
            }
            Tool::Brush => {
                ui.add(egui::Slider::new(&mut self.brush_size, 3.0..=300.0).text("Taille"));
                ui.add(egui::Slider::new(&mut self.brush_feather, 0.0..=100.0).text("Contour"));
                ui.add(egui::Slider::new(&mut self.brush_flow, 1.0..=100.0).text("Débit"));
                ui.toggle_value(&mut self.brush_erase, "Gomme").on_hover_text(tl("Ou maintiens Alt"));
                if pro::ghost(ui, tl("Nouveau masque")).clicked() {
                    self.sel_mask = None;
                    self.set_settings(i, s.clone());
                    self.ensure_brush_target(i);
                    s = self.settings(i);
                }
                ui.checkbox(&mut self.show_overlay, tl("Superposition (O)"));
            }
            Tool::Linear | Tool::Radial => {
                caption(ui, if self.tool == Tool::Linear { "Glisse sur la photo pour tracer un filtre gradué" } else { "Glisse depuis le centre pour tracer un filtre radial" });
                ui.checkbox(&mut self.show_overlay, tl("Superposition (O)"));
            }
            Tool::WbPicker => {
                caption(ui, tl("Clique sur une zone qui devrait être neutre (gris, blanc). Maj+clic pour garder l'outil."));
            }
            Tool::Target => {
                pro::segmented(ui, &mut self.target_mode, &TARGET_MODES);
                caption(ui, tl("Glisse vers le haut ou le bas sur une couleur ou un ton de la photo"));
            }
            Tool::Sky | Tool::Subject => {}
        }
        if s != before {
            self.set_settings(i, s);
        }
    }

    pub fn apply_ratio(&self, s: &mut Settings) {
        let ratio = develop::RATIOS[self.crop_ratio].1;
        if let Some((_, src)) = &self.source {
            s.crop = if ratio > 0.0 {
                let (w, h) = (src.width() as f32, src.height() as f32);
                let (w, h) = if s.rotate % 2 == 1 { (h, w) } else { (w, h) };
                let c = develop::crop_for_ratio(w, h, ratio);
                if s.rotate % 2 == 1 { [c[1], c[0], c[3], c[2]] } else { c }
            } else {
                [0.0; 4]
            };
        }
    }

    pub fn photo_tools(&mut self, ui: &mut Ui, r: Rect, area: Rect, i: usize, ctx: &egui::Context) {
        let Some((_, src)) = &self.source else { return };
        let (sw, sh) = (src.width(), src.height());
        let s = self.settings(i);
        let disp = self.display_settings(&s);
        let geo = Geo::new(&disp, sw, sh);
        let to_screen = |su: f32, sv: f32| {
            let (u, v) = geo.from_src(su, sv);
            r.min + vec2(u * r.width(), v * r.height())
        };
        let to_src = |p: Pos2| geo.to_src((p.x - r.left()) / r.width(), (p.y - r.top()) / r.height());
        let screen_r = |su: f32, sv: f32, rad: f32| to_screen(su + rad, sv).distance(to_screen(su, sv)).max(2.0);
        let painter = ui.painter_at(area);
        let resp = ui.interact(r, ui.make_persistent_id("photo"), Sense::click_and_drag());
        let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
        let alt = ui.input(|i| i.modifiers.alt);
        let shift = ui.input(|i| i.modifiers.shift);
        let mut st = s.clone();
        let white = Stroke::new(1.5, Color32::WHITE);
        let shadow = Stroke::new(3.0, Color32::from_black_alpha(120));

        if let Some(rendered) = &self.rendered
            && let Some(h) = resp.hover_pos()
        {
            let (u, v) = ((h.x - r.left()) / r.width(), (h.y - r.top()) / r.height());
            let (x, y) = ((u * rendered.width() as f32) as u32, (v * rendered.height() as f32) as u32);
            self.hover_rgb = (x < rendered.width() && y < rendered.height()).then(|| rendered.get_pixel(x, y).0);
        } else {
            self.hover_rgb = None;
        }

        match self.tool {
            Tool::Crop => {
                let v = insets_to_view(s.crop.map(|x| x.clamp(0.0, 0.45)), &s);
                let cr = Rect::from_min_max(pos2(r.left() + v[0] * r.width(), r.top() + v[1] * r.height()), pos2(r.right() - v[2] * r.width(), r.bottom() - v[3] * r.height()));
                let dim = Color32::from_black_alpha(150);
                for band in [Rect::from_min_max(r.min, pos2(r.right(), cr.top())), Rect::from_min_max(pos2(r.left(), cr.bottom()), r.max), Rect::from_min_max(pos2(r.left(), cr.top()), pos2(cr.left(), cr.bottom())), Rect::from_min_max(pos2(cr.right(), cr.top()), pos2(r.right(), cr.bottom()))] {
                    painter.rect_filled(band, CornerRadius::ZERO, dim);
                }
                painter.rect_stroke(cr, CornerRadius::ZERO, white, egui::StrokeKind::Middle);
                let thin = Stroke::new(1.0, Color32::from_white_alpha(120));
                let lines: Vec<f32> = match self.crop_overlay {
                    1 => vec![1.0 / 3.0, 2.0 / 3.0],
                    2 => (1..8).map(|k| k as f32 / 8.0).collect(),
                    3 => vec![0.382, 0.618],
                    _ => vec![],
                };
                for f in &lines {
                    painter.line_segment([pos2(cr.left() + cr.width() * f, cr.top()), pos2(cr.left() + cr.width() * f, cr.bottom())], thin);
                    painter.line_segment([pos2(cr.left(), cr.top() + cr.height() * f), pos2(cr.right(), cr.top() + cr.height() * f)], thin);
                }
                if self.crop_overlay == 4 {
                    painter.line_segment([cr.left_top(), cr.right_bottom()], thin);
                    painter.line_segment([cr.right_top(), cr.left_bottom()], thin);
                }
                for c in [cr.left_top(), cr.right_top(), cr.left_bottom(), cr.right_bottom()] {
                    painter.rect_filled(Rect::from_center_size(c, Vec2::splat(9.0)), CornerRadius::same(2), Color32::WHITE);
                }
                for c in [cr.center_top(), cr.center_bottom(), cr.left_center(), cr.right_center()] {
                    painter.rect_filled(Rect::from_center_size(c, Vec2::splat(7.0)), CornerRadius::same(2), Color32::WHITE);
                }
                if let Some(pp) = pointer {
                    let edge = |p: Pos2| {
                        let mut bits = 0u8;
                        let tol = 10.0;
                        let inside_y = p.y > cr.top() - tol && p.y < cr.bottom() + tol;
                        let inside_x = p.x > cr.left() - tol && p.x < cr.right() + tol;
                        if (p.x - cr.left()).abs() < tol && inside_y {
                            bits |= 1;
                        }
                        if (p.y - cr.top()).abs() < tol && inside_x {
                            bits |= 2;
                        }
                        if (p.x - cr.right()).abs() < tol && inside_y {
                            bits |= 4;
                        }
                        if (p.y - cr.bottom()).abs() < tol && inside_x {
                            bits |= 8;
                        }
                        bits
                    };
                    let b = edge(pp);
                    if resp.hovered() && self.drag == crate::Drag::None {
                        ctx.set_cursor_icon(match b {
                            1 | 4 => egui::CursorIcon::ResizeHorizontal,
                            2 | 8 => egui::CursorIcon::ResizeVertical,
                            3 | 12 => egui::CursorIcon::ResizeNwSe,
                            6 | 9 => egui::CursorIcon::ResizeNeSw,
                            _ if cr.contains(pp) => egui::CursorIcon::Move,
                            _ => egui::CursorIcon::Alias,
                        });
                    }
                    if resp.drag_started() {
                        self.drag = if b != 0 {
                            crate::Drag::CropEdge(b)
                        } else if cr.contains(pp) {
                            crate::Drag::CropMove
                        } else {
                            let a = (pp - cr.center()).angle().to_degrees();
                            crate::Drag::CropRotate(a, s.straighten)
                        };
                    }
                    if resp.dragged() {
                        let (fx, fy) = (((pp.x - r.left()) / r.width()).clamp(0.0, 1.0), ((pp.y - r.top()) / r.height()).clamp(0.0, 1.0));
                        let mut v = v;
                        match self.drag {
                            crate::Drag::CropEdge(b) => {
                                if b & 1 != 0 {
                                    v[0] = fx.min(1.0 - v[2] - 0.1);
                                }
                                if b & 4 != 0 {
                                    v[2] = (1.0 - fx).min(1.0 - v[0] - 0.1);
                                }
                                if b & 2 != 0 {
                                    v[1] = fy.min(1.0 - v[3] - 0.1);
                                }
                                if b & 8 != 0 {
                                    v[3] = (1.0 - fy).min(1.0 - v[1] - 0.1);
                                }
                                let ratio = develop::RATIOS[self.crop_ratio].1;
                                let ratio = if ratio > 0.0 { ratio } else if self.crop_lock { cr.width() / cr.height().max(1.0) } else { 0.0 };
                                if ratio > 0.0 {
                                    let cw = (1.0 - v[0] - v[2]) * r.width();
                                    let ch = (1.0 - v[1] - v[3]) * r.height();
                                    if b & 5 != 0 {
                                        let want = (cw / ratio / r.height()).min(1.0);
                                        if b & 2 != 0 {
                                            v[1] = 1.0 - v[3] - want;
                                        } else if b & 8 != 0 {
                                            v[3] = 1.0 - v[1] - want;
                                        } else {
                                            let mid = (v[1] + 1.0 - v[3]) / 2.0;
                                            v[1] = mid - want / 2.0;
                                            v[3] = 1.0 - mid - want / 2.0;
                                        }
                                    } else {
                                        let want = (ch * ratio / r.width()).min(1.0);
                                        let mid = (v[0] + 1.0 - v[2]) / 2.0;
                                        v[0] = mid - want / 2.0;
                                        v[2] = 1.0 - mid - want / 2.0;
                                    }
                                }
                            }
                            crate::Drag::CropMove => {
                                let d = resp.drag_delta();
                                let (dx, dy) = (d.x / r.width(), d.y / r.height());
                                let dx = dx.clamp(-v[0], v[2]);
                                let dy = dy.clamp(-v[1], v[3]);
                                v = [v[0] + dx, v[1] + dy, v[2] - dx, v[3] - dy];
                            }
                            crate::Drag::CropRotate(a0, s0) => {
                                let a = (pp - cr.center()).angle().to_degrees();
                                let sign = if s.flip ^ s.flip_v { -1.0 } else { 1.0 };
                                st.straighten = (s0 + (a - a0) * sign).clamp(-45.0, 45.0);
                            }
                            _ => {}
                        }
                        let v = v.map(|x| x.clamp(0.0, 0.45));
                        st.crop = insets_from_view(v, &s);
                    }
                }
            }
            Tool::Straighten => {
                if resp.drag_started()
                    && let Some(pp) = pointer
                {
                    self.drag = crate::Drag::Ruler(pp);
                }
                if let (crate::Drag::Ruler(a), Some(b)) = (self.drag, pointer) {
                    painter.line_segment([a, b], shadow);
                    painter.line_segment([a, b], Stroke::new(1.5, Color32::from_rgb(255, 214, 10)));
                    let d = b - a;
                    let theta = (-d.y).atan2(d.x).to_degrees();
                    let res = theta - (theta / 90.0).round() * 90.0;
                    painter.text(b + vec2(12.0, -12.0), Align2::LEFT_BOTTOM, format!("{:+.1}°", res), semibold(12.5), Color32::WHITE);
                    if resp.drag_stopped() {
                        if d.length() > 8.0 {
                            let sign = if s.flip ^ s.flip_v { -1.0 } else { 1.0 };
                            st.straighten = (s.straighten + res * sign).clamp(-45.0, 45.0);
                        }
                        self.drag = crate::Drag::None;
                    }
                }
                let thin = Stroke::new(1.0, Color32::from_white_alpha(50));
                for k in 1..6 {
                    let f = k as f32 / 6.0;
                    painter.line_segment([pos2(r.left(), r.top() + r.height() * f), pos2(r.right(), r.top() + r.height() * f)], thin);
                }
            }
            Tool::Remove | Tool::Heal | Tool::Clone => {
                for (k, sp) in s.spots.iter().enumerate() {
                    let c = to_screen(sp.x, sp.y);
                    let rad = screen_r(sp.x, sp.y, sp.r);
                    let sel = self.sel_spot == Some(k);
                    painter.circle_stroke(c, rad, shadow);
                    painter.circle_stroke(c, rad, Stroke::new(if sel { 2.0 } else { 1.2 }, if sel { Color32::WHITE } else { Color32::from_white_alpha(170) }));
                    if sel {
                        let sc = to_screen(sp.sx, sp.sy);
                        painter.line_segment([c, sc], Stroke::new(1.0, Color32::from_white_alpha(140)));
                        painter.circle_stroke(sc, rad, Stroke::new(1.0, Color32::from_white_alpha(170)));
                        painter.circle_filled(sc, 3.0, Color32::WHITE);
                    }
                }
                let size = self.spot_size / 1000.0;
                if let Some(h) = resp.hover_pos() {
                    let (su, sv) = to_src(h);
                    painter.circle_stroke(h, screen_r(su, sv, size), Stroke::new(1.0, Color32::from_white_alpha(200)));
                }
                let cur_sel = self.sel_spot;
                let hit = |p: Pos2| -> Option<(usize, bool)> {
                    for (k, sp) in s.spots.iter().enumerate().rev() {
                        if cur_sel == Some(k) && to_screen(sp.sx, sp.sy).distance(p) < screen_r(sp.x, sp.y, sp.r) {
                            return Some((k, true));
                        }
                        if to_screen(sp.x, sp.y).distance(p) < screen_r(sp.x, sp.y, sp.r) {
                            return Some((k, false));
                        }
                    }
                    None
                };
                let mode = match self.tool {
                    Tool::Heal => SpotMode::Heal,
                    Tool::Clone => SpotMode::Clone,
                    _ => SpotMode::Remove,
                };
                let new_spot = |p: Pos2, st: &mut Settings| {
                    let (x, y) = to_src(p);
                    let (sx, sy) = develop::find_source(src, x, y, size);
                    st.spots.push(Spot { x, y, sx, sy, r: size, feather: self.spot_feather / 100.0, opacity: self.spot_opacity / 100.0, mode });
                    st.spots.len() - 1
                };
                if resp.secondary_clicked()
                    && let Some(p) = pointer
                    && let Some((k, _)) = hit(p)
                {
                    st.spots.remove(k);
                    self.sel_spot = None;
                } else if resp.clicked()
                    && let Some(p) = pointer
                {
                    match hit(p) {
                        Some((k, _)) => self.sel_spot = Some(k),
                        None => self.sel_spot = Some(new_spot(p, &mut st)),
                    }
                }
                if resp.drag_started()
                    && let Some(p) = pointer
                {
                    self.drag = match hit(p) {
                        Some((k, true)) => crate::Drag::SpotSrc(k),
                        Some((k, false)) => crate::Drag::SpotDest(k),
                        None => crate::Drag::NewSpot(new_spot(p, &mut st)),
                    };
                    if let crate::Drag::SpotSrc(k) | crate::Drag::SpotDest(k) | crate::Drag::NewSpot(k) = self.drag {
                        self.sel_spot = Some(k);
                    }
                }
                if resp.dragged()
                    && let Some(p) = pointer
                {
                    let (u, v) = to_src(p);
                    match self.drag {
                        crate::Drag::SpotDest(k) if k < st.spots.len() => {
                            let sp = &mut st.spots[k];
                            let (du, dv) = (u - sp.x, v - sp.y);
                            (sp.x, sp.y) = (u, v);
                            if sp.mode == SpotMode::Remove {
                                (sp.sx, sp.sy) = (sp.sx + du, sp.sy + dv);
                            }
                        }
                        crate::Drag::SpotSrc(k) | crate::Drag::NewSpot(k) if k < st.spots.len() => {
                            (st.spots[k].sx, st.spots[k].sy) = (u, v);
                        }
                        _ => {}
                    }
                }
            }
            Tool::RedEye => {
                for (k, e) in s.red_eyes.iter().enumerate() {
                    let c = to_screen(e.x, e.y);
                    let rad = screen_r(e.x, e.y, e.r);
                    painter.circle_stroke(c, rad, shadow);
                    painter.circle_stroke(c, rad, Stroke::new(if self.sel_eye == Some(k) { 2.0 } else { 1.2 }, Color32::WHITE));
                }
                let hit = |p: Pos2| s.red_eyes.iter().rposition(|e| to_screen(e.x, e.y).distance(p) < screen_r(e.x, e.y, e.r));
                if resp.secondary_clicked()
                    && let Some(k) = pointer.and_then(hit)
                {
                    st.red_eyes.remove(k);
                    self.sel_eye = None;
                } else if resp.clicked()
                    && let Some(p) = pointer
                {
                    match hit(p) {
                        Some(k) => self.sel_eye = Some(k),
                        None => {
                            let (x, y) = to_src(p);
                            st.red_eyes.push(RedEye { x, y, r: 0.02, pupil: self.eye_pupil / 100.0, darken: self.eye_darken / 100.0 });
                            self.sel_eye = Some(st.red_eyes.len() - 1);
                        }
                    }
                }
                if resp.drag_started()
                    && let Some(p) = pointer
                {
                    self.drag = match hit(p) {
                        Some(k) => crate::Drag::EyeMove(k),
                        None => {
                            let (x, y) = to_src(p);
                            st.red_eyes.push(RedEye { x, y, r: 0.005, pupil: self.eye_pupil / 100.0, darken: self.eye_darken / 100.0 });
                            crate::Drag::NewEye(st.red_eyes.len() - 1, p)
                        }
                    };
                }
                if resp.dragged()
                    && let Some(p) = pointer
                {
                    let (u, v) = to_src(p);
                    match self.drag {
                        crate::Drag::EyeMove(k) if k < st.red_eyes.len() => (st.red_eyes[k].x, st.red_eyes[k].y) = (u, v),
                        crate::Drag::NewEye(k, _) if k < st.red_eyes.len() => {
                            let e = &mut st.red_eyes[k];
                            let du = (u - e.x, (v - e.y) * sh as f32 / sw as f32);
                            e.r = (du.0 * du.0 + du.1 * du.1).sqrt().max(0.004);
                            self.sel_eye = Some(k);
                        }
                        _ => {}
                    }
                }
            }
            Tool::WbPicker => {
                if let Some(h) = resp.hover_pos() {
                    let (u, v) = to_src(h);
                    let c = sample(src, u, v);
                    let sw_r = Rect::from_min_size(h + vec2(16.0, 16.0), vec2(120.0, 34.0));
                    painter.rect_filled(sw_r, CornerRadius::same(4), Color32::from_black_alpha(200));
                    painter.rect_filled(Rect::from_min_size(sw_r.min + vec2(6.0, 6.0), Vec2::splat(22.0)), CornerRadius::same(4), Color32::from_rgb((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8));
                    let (t, ti) = develop::wb_from_pixel(c);
                    painter.text(sw_r.min + vec2(34.0, 17.0), Align2::LEFT_CENTER, format!("T {t:+.0} · Tt {ti:+.0}"), FontId::proportional(11.5), Color32::WHITE);
                    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                    if resp.clicked() {
                        (st.temperature, st.tint) = (t, ti);
                        if !shift {
                            self.tool = Tool::None;
                        }
                    }
                }
            }
            Tool::Target => {
                if resp.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }
                if resp.drag_started()
                    && let (Some(p), Some(img)) = (pointer, &self.rendered)
                {
                    let (u, v) = ((p.x - r.left()) / r.width(), (p.y - r.top()) / r.height());
                    let px = img.get_pixel(((u * img.width() as f32) as u32).min(img.width() - 1), ((v * img.height() as f32) as u32).min(img.height() - 1)).0;
                    let c = px.map(|x| x as f32 / 255.0);
                    let (hue, _, _) = develop::rgb_to_hsl(c);
                    self.drag = crate::Drag::Target(hue, develop::luma(c), p.y);
                }
                if let crate::Drag::Target(hue, lum, y0) = self.drag
                    && resp.dragged()
                    && let Some(p) = pointer
                {
                    let amount = -(p.y - y0) * 0.4;
                    if amount != 0.0 {
                        if self.target_mode == 0 {
                            let w: Vec<f32> = (0..4).map(|k| (-((lum - (0.125 + 0.25 * k as f32)) / 0.15).powi(2)).exp()).collect();
                            let max = w.iter().cloned().fold(1e-4, f32::max);
                            for k in 0..4 {
                                st.curve[k] = (st.curve[k] + amount * w[k] / max).clamp(-100.0, 100.0);
                            }
                        } else {
                            let tab = self.target_mode - 1;
                            self.hsl_tab = tab;
                            for (b, wgt) in develop::band_weights(hue) {
                                st.hsl[b][tab] = (st.hsl[b][tab] + amount * wgt).clamp(-100.0, 100.0);
                            }
                        }
                        self.drag = crate::Drag::Target(hue, lum, p.y);
                    }
                }
                if let crate::Drag::Target(hue, lum, _) = self.drag
                    && let Some(p) = pointer
                {
                    let txt = if self.target_mode == 0 { format!("Ton {:.0} %", lum * 100.0) } else { format!("{} · {}", develop::BANDS[develop::band_weights(hue)[0].0].0, TARGET_MODES[self.target_mode]) };
                    painter.text(p + vec2(14.0, -14.0), Align2::LEFT_BOTTOM, txt, semibold(12.0), Color32::WHITE);
                }
            }
            Tool::Brush => {
                let rad = self.brush_size / 1000.0;
                if let Some(h) = resp.hover_pos() {
                    let (su, sv) = to_src(h);
                    let outer = screen_r(su, sv, rad);
                    painter.circle_stroke(h, outer, Stroke::new(1.2, Color32::WHITE));
                    painter.circle_stroke(h, outer * (1.0 - self.brush_feather / 100.0 * 0.9), Stroke::new(1.0, Color32::from_white_alpha(110)));
                    if alt || self.brush_erase {
                        painter.text(h, Align2::CENTER_CENTER, "−", semibold(13.0), Color32::WHITE);
                    }
                    ctx.set_cursor_icon(egui::CursorIcon::None);
                }
                if (resp.drag_started() || resp.clicked())
                    && let Some(p) = pointer
                {
                    self.ensure_brush_target(i);
                    st = self.settings(i);
                    let (u, v) = to_src(p);
                    self.add_dab(&mut st, u, v, alt);
                    self.drag = crate::Drag::Paint(u, v);
                }
                if let crate::Drag::Paint(lu, lv) = self.drag
                    && resp.dragged()
                    && let Some(p) = pointer
                {
                    let (u, v) = to_src(p);
                    let d = ((u - lu).powi(2) + ((v - lv) * sh as f32 / sw as f32).powi(2)).sqrt();
                    let step = (rad * 0.3).max(0.002);
                    if d >= step {
                        let n = (d / step).ceil() as usize;
                        for k in 1..=n {
                            let t = k as f32 / n as f32;
                            self.add_dab(&mut st, lu + (u - lu) * t, lv + (v - lv) * t, alt);
                        }
                        self.drag = crate::Drag::Paint(u, v);
                    }
                }
            }
            _ => {}
        }

        if matches!(self.tool, Tool::None | Tool::Linear | Tool::Radial | Tool::Brush) {
            self.mask_handles(&painter, &resp, pointer, &s, &mut st, &to_screen, &to_src, sw as f32 / sh as f32);
        }

        if self.tool == Tool::None {
            if resp.double_clicked() {
                self.zoom = if self.zoom == 0.0 { 1.0 } else { 0.0 };
                self.pan = Vec2::ZERO;
            } else if resp.dragged() && self.zoom > 0.0 && matches!(self.drag, crate::Drag::None | crate::Drag::Pan) {
                self.drag = crate::Drag::Pan;
                self.pan += resp.drag_delta();
            }
            if self.zoom > 0.0 && resp.hovered() {
                ctx.set_cursor_icon(if resp.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
            }
        }
        if resp.drag_stopped() || (!resp.dragged() && ui.input(|i| !i.pointer.any_down()) && self.drag != crate::Drag::Split) {
            self.drag = crate::Drag::None;
        }
        if st != s {
            self.set_settings(i, st);
        }
    }

    fn add_dab(&self, st: &mut Settings, u: f32, v: f32, alt: bool) {
        if let Some(m) = self.sel_mask.and_then(|m| st.masks.get_mut(m))
            && let Some(p) = m.parts.get_mut(self.sel_part).filter(|p| p.kind == PartKind::Brush)
        {
            p.dabs.push(Dab { x: u, y: v, r: self.brush_size / 1000.0, feather: self.brush_feather / 100.0, flow: self.brush_flow / 100.0, erase: alt || self.brush_erase });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn mask_handles(&mut self, painter: &Painter, resp: &egui::Response, pointer: Option<Pos2>, s: &Settings, st: &mut Settings, to_screen: &dyn Fn(f32, f32) -> Pos2, to_src: &dyn Fn(Pos2) -> (f32, f32), aspect: f32) {
        let white = Stroke::new(1.5, Color32::WHITE);
        let shadow = Stroke::new(3.0, Color32::from_black_alpha(120));
        let mut handles: Vec<(usize, usize, u8, Pos2)> = Vec::new();
        for (m, mask) in s.masks.iter().enumerate() {
            let sel = self.sel_mask == Some(m);
            for (k, p) in mask.parts.iter().enumerate() {
                match p.kind {
                    PartKind::Linear if sel => {
                        let (dx, dy) = (p.x2 - p.x1, p.y2 - p.y1);
                        for (f, w) in [(0.0f32, 1.5f32), (0.5, 1.0), (1.0, 1.5)] {
                            let (cx, cy) = (p.x1 + dx * f, p.y1 + dy * f);
                            let pts: Vec<Pos2> = (-20..=20).map(|t| { let t = t as f32 / 10.0; to_screen(cx - dy * t, cy + dx * t) }).collect();
                            painter.add(egui::Shape::line(pts.clone(), shadow));
                            painter.add(egui::Shape::line(pts, Stroke::new(w, if f == 0.5 { Color32::from_white_alpha(120) } else { Color32::WHITE })));
                        }
                        handles.push((m, k, 1, to_screen(p.x1, p.y1)));
                        handles.push((m, k, 2, to_screen(p.x2, p.y2)));
                        handles.push((m, k, 0, to_screen(p.x1 + dx * 0.5, p.y1 + dy * 0.5)));
                    }
                    PartKind::Radial if sel => {
                        let a = p.angle.to_radians();
                        let ell = |f: f32| -> Vec<Pos2> {
                            (0..=48)
                                .map(|q| {
                                    let t = q as f32 / 48.0 * std::f32::consts::TAU;
                                    let (ex, ey) = (p.rx * aspect * f * t.cos(), p.ry * f * t.sin());
                                    let (dx, dy) = (ex * a.cos() - ey * a.sin(), ex * a.sin() + ey * a.cos());
                                    to_screen(p.x1 + dx / aspect, p.y1 + dy)
                                })
                                .collect()
                        };
                        painter.add(egui::Shape::line(ell(1.0), shadow));
                        painter.add(egui::Shape::line(ell(1.0), white));
                        painter.add(egui::Shape::line(ell(1.0 - p.feather.clamp(0.02, 1.0)), Stroke::new(1.0, Color32::from_white_alpha(110))));
                        handles.push((m, k, 0, to_screen(p.x1, p.y1)));
                        let (ex, ey) = (p.rx * aspect, 0.0);
                        handles.push((m, k, 3, to_screen(p.x1 + (ex * a.cos() - ey * a.sin()) / aspect, p.y1 + ex * a.sin() + ey * a.cos())));
                        let (ex, ey) = (0.0, -p.ry);
                        handles.push((m, k, 4, to_screen(p.x1 + (ex * a.cos() - ey * a.sin()) / aspect, p.y1 + ex * a.sin() + ey * a.cos())));
                    }
                    _ => {}
                }
            }
            let pin = mask.parts.first().map(|p| match p.kind {
                PartKind::Linear => ((p.x1 + p.x2) / 2.0, (p.y1 + p.y2) / 2.0),
                PartKind::Radial => (p.x1, p.y1),
                PartKind::Brush => p.dabs.first().map(|d| (d.x, d.y)).unwrap_or((0.5, 0.5)),
                _ => (0.06 + 0.05 * m as f32, 0.06),
            });
            if let Some((px, py)) = pin {
                let c = to_screen(px, py) + if mask.parts.first().is_some_and(|p| matches!(p.kind, PartKind::Linear | PartKind::Radial)) { vec2(0.0, 0.0) } else { vec2(0.0, 0.0) };
                painter.circle(c, if sel { 7.0 } else { 5.5 }, if sel { BLUE } else { Color32::from_gray(90) }, Stroke::new(1.5, Color32::WHITE));
                handles.push((m, usize::MAX, 9, c));
            }
        }
        let hit = |p: Pos2| handles.iter().rev().find(|h| h.3.distance(p) < 9.0).copied();
        if resp.clicked()
            && let Some(p) = pointer
            && let Some((m, _, _, _)) = hit(p)
        {
            self.sel_mask = Some(m);
            self.sel_part = 0;
        }
        if resp.drag_started()
            && let Some(p) = pointer
        {
            if let Some((m, k, h, _)) = hit(p) {
                self.sel_mask = Some(m);
                if k != usize::MAX {
                    self.sel_part = k;
                    self.drag = crate::Drag::Handle(m, k, h);
                } else {
                    self.drag = crate::Drag::Handle(m, 0, 0);
                }
            } else if matches!(self.tool, Tool::Linear | Tool::Radial) {
                let (u, v) = to_src(p);
                let kind = if self.tool == Tool::Linear { PartKind::Linear } else { PartKind::Radial };
                let part = Part { kind, x1: u, y1: v, x2: u, y2: v + 0.001, rx: 0.001, ry: 0.001, ..Part::new(kind) };
                st.masks.push(Mask { name: part_name(kind).into(), parts: vec![part], ..Default::default() });
                let m = st.masks.len() - 1;
                self.sel_mask = Some(m);
                self.sel_part = 0;
                self.drag = if kind == PartKind::Linear { crate::Drag::NewLinear(m) } else { crate::Drag::NewRadial(m) };
            }
        }
        if resp.dragged()
            && let Some(p) = pointer
        {
            let (u, v) = to_src(p);
            match self.drag {
                crate::Drag::NewLinear(m) => {
                    if let Some(part) = st.masks.get_mut(m).and_then(|x| x.parts.first_mut()) {
                        (part.x2, part.y2) = (u, v);
                    }
                }
                crate::Drag::NewRadial(m) => {
                    if let Some(part) = st.masks.get_mut(m).and_then(|x| x.parts.first_mut()) {
                        let d = (((u - part.x1) * aspect).powi(2) + (v - part.y1).powi(2)).sqrt().max(0.005);
                        (part.rx, part.ry) = (d / aspect, d);
                    }
                }
                crate::Drag::Handle(m, k, h) => {
                    if let Some(part) = st.masks.get_mut(m).and_then(|x| x.parts.get_mut(k)) {
                        match (part.kind, h) {
                            (PartKind::Linear, 1) => (part.x1, part.y1) = (u, v),
                            (PartKind::Linear, 2) => (part.x2, part.y2) = (u, v),
                            (PartKind::Linear, _) => {
                                let (cx, cy) = ((part.x1 + part.x2) / 2.0, (part.y1 + part.y2) / 2.0);
                                let (du, dv) = (u - cx, v - cy);
                                (part.x1, part.y1, part.x2, part.y2) = (part.x1 + du, part.y1 + dv, part.x2 + du, part.y2 + dv);
                            }
                            (PartKind::Radial, 3) => {
                                let a = part.angle.to_radians();
                                let (dx, dy) = ((u - part.x1) * aspect, v - part.y1);
                                part.rx = ((dx * a.cos() + dy * a.sin()).abs() / aspect).max(0.005);
                            }
                            (PartKind::Radial, 4) => {
                                let a = part.angle.to_radians();
                                let (dx, dy) = ((u - part.x1) * aspect, v - part.y1);
                                part.ry = (-dx * a.sin() + dy * a.cos()).abs().max(0.005);
                            }
                            (PartKind::Radial, _) => (part.x1, part.y1) = (u, v),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn info_overlay(&mut self, painter: &Painter, r: Rect, i: usize) {
        if self.info_overlay == 0 {
            return;
        }
        let e = self.get(i);
        let mut lines = vec![display_name(&self.files[i])];
        if self.info_overlay == 1 {
            let s = &e.settings;
            lines.push(format!("Expo {:+.2} · Contraste {:+.0} · Temp {:+.0}", s.exposure, s.contrast, s.temperature));
            lines.push(format!("{} masque(s) · {} retouche(s) · {} instantané(s)", s.masks.len(), s.spots.len(), e.snapshots.len()));
        } else {
            for (k, v) in self.exif_for(i).into_iter().take(6) {
                lines.push(format!("{k} : {v}"));
            }
        }
        let h = lines.len() as f32 * 17.0 + 12.0;
        let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(10) as f32 * 6.6 + 20.0;
        let bg = Rect::from_min_size(r.left_top() + vec2(12.0, 12.0), vec2(w, h));
        painter.rect_filled(bg, CornerRadius::same(4), Color32::from_black_alpha(170));
        for (k, l) in lines.iter().enumerate() {
            painter.text(bg.min + vec2(10.0, 8.0 + k as f32 * 17.0), Align2::LEFT_TOP, short(l, 60), if k == 0 { semibold(12.0) } else { FontId::proportional(12.0) }, if k == 0 { Color32::WHITE } else { Color32::from_white_alpha(200) });
        }
    }
}

pub fn spot_mode_name(m: SpotMode) -> &'static str {
    match m {
        SpotMode::Remove => "Suppression",
        SpotMode::Heal => "Correcteur",
        SpotMode::Clone => "Clonage",
    }
}

fn sample(img: &image::RgbImage, u: f32, v: f32) -> [f32; 3] {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let (cx, cy) = ((u * w as f32) as i32, (v * h as f32) as i32);
    let mut acc = [0f32; 3];
    let mut n = 0.0;
    for dy in -2..=2 {
        for dx in -2..=2 {
            let (x, y) = ((cx + dx).clamp(0, w - 1), (cy + dy).clamp(0, h - 1));
            let p = img.get_pixel(x as u32, y as u32).0;
            for k in 0..3 {
                acc[k] += p[k] as f32 / 255.0;
            }
            n += 1.0;
        }
    }
    acc.map(|v| v / n)
}

