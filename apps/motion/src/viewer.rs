use super::*;
use model::{Guide, Mat, Vtx, apply, invert};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Select,
    Hand,
    ZoomIn,
    ZoomOut,
    Rotate,
    Anchor,
    Rect,
    RoundRect,
    Ellipse,
    Polygon,
    Star,
    Pen,
    PenAdd,
    PenDel,
    PenConv,
    TextH,
    TextV,
    Brush,
    Eraser,
    MaskRect,
    MaskEllipse,
    Puppet,
}

pub const TOOL_GROUPS: [&[(Tool, &str, &str)]; 11] = [
    &[(Tool::Select, "Sélection", "V")],
    &[(Tool::Hand, "Main", "H")],
    &[(Tool::ZoomIn, "Zoom avant", "Z"), (Tool::ZoomOut, "Zoom arrière", "Z")],
    &[(Tool::Rotate, "Rotation", "W")],
    &[(Tool::Anchor, "Point d'ancrage", "Y")],
    &[(Tool::Rect, "Rectangle", "Q"), (Tool::RoundRect, "Rectangle arrondi", "Q"), (Tool::Ellipse, "Ellipse", "Q"), (Tool::Polygon, "Polygone", "Q"), (Tool::Star, "Étoile", "Q")],
    &[(Tool::Pen, "Plume", "G"), (Tool::PenAdd, "Ajout de sommet", "G"), (Tool::PenDel, "Suppression de sommet", "G"), (Tool::PenConv, "Conversion de sommet", "G")],
    &[(Tool::TextH, "Texte horizontal", "Ctrl+T"), (Tool::TextV, "Texte vertical", "Ctrl+T")],
    &[(Tool::Brush, "Pinceau", "Ctrl+B"), (Tool::Eraser, "Gomme", "Ctrl+B")],
    &[(Tool::MaskRect, "Masque rectangulaire", ""), (Tool::MaskEllipse, "Masque elliptique", "")],
    &[(Tool::Puppet, "Marionnette", "")],
];

pub struct ToolOpts {
    pub fill: [u8; 4],
    pub stroke_on: bool,
    pub stroke: [u8; 4],
    pub stroke_w: f32,
    pub round: f32,
    pub sides: u32,
    pub points: u32,
    pub inner: f32,
    pub from_center: bool,
    pub pen_fill: bool,
    pub pen_color: [u8; 4],
    pub pen_w: f32,
    pub pen_stroke: [u8; 4],
    pub text_size: f32,
    pub text_color: [u8; 4],
    pub font: String,
    pub brush: f32,
    pub brush_color: [u8; 4],
    pub smooth: u32,
    pub eraser: f32,
    pub puppet: f32,
    pub mask_invert: bool,
}

impl Default for ToolOpts {
    fn default() -> Self {
        Self {
            fill: [10, 132, 255, 255],
            stroke_on: false,
            stroke: [255, 255, 255, 255],
            stroke_w: 6.0,
            round: 24.0,
            sides: 6,
            points: 5,
            inner: 0.45,
            from_center: false,
            pen_fill: false,
            pen_color: [255, 92, 120, 255],
            pen_w: 8.0,
            pen_stroke: [255, 255, 255, 255],
            text_size: 96.0,
            text_color: [255, 255, 255, 255],
            font: "Segoe UI".into(),
            brush: 14.0,
            brush_color: [255, 214, 10, 255],
            smooth: 2,
            eraser: 30.0,
            puppet: 160.0,
            mask_invert: false,
        }
    }
}

#[derive(Clone, Copy)]
pub enum VDrag {
    Pan { p0: Vec2, from: Pos2 },
    Move { wx: f32, wy: f32, from: Pos2 },
    Scale { ws: f32, d0: f32, c: Pos2 },
    Rotate { r0: f32, a0: f32, c: Pos2 },
    Anchor { inv: Mat, lin: [f32; 4], pos: (f32, f32), anc: (f32, f32) },
    Shape { from: (f32, f32) },
    Vertex { li: usize, vi: usize, part: u8, inv: Mat },
    Brush { li: usize, inv: Mat },
    Erase { li: usize, inv: Mat, r: f32 },
    Puppet { li: usize, inv: Mat, last: (f32, f32), r: f32 },
    Guide { gi: usize },
    NewGuide { vertical: bool },
    PathKey { li: usize, kt: f32 },
}

/// Lucide icon names, so this app's icons match the rest of the suite.
fn tool_glyph_lucide(tool: Tool) -> &'static str {
    use Tool::*;
    match tool {
        Select => "mouse-pointer-2",
        Hand => "hand",
        ZoomIn => "zoom-in",
        ZoomOut => "zoom-out",
        Rotate => "rotate-cw",
        Anchor => "anchor",
        Rect => "square",
        RoundRect => "squircle",
        Ellipse => "circle",
        Polygon => "hexagon",
        Star => "star",
        Pen => "pen-tool",
        PenAdd => "circle-plus",
        PenDel => "circle-minus",
        PenConv => "spline-pointer",
        TextH => "type",
        TextV => "text-cursor",
        Brush => "paintbrush",
        Eraser => "eraser",
        MaskRect => "square-dashed",
        MaskEllipse => "circle-dashed",
        Puppet => "person-standing",
    }
}

pub fn tool_glyph(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    studio_ui::pro::lucide(p, r.expand(r.width() * 0.12), tool_glyph_lucide(tool), c);
}

fn group_of(tool: Tool) -> usize {
    TOOL_GROUPS.iter().position(|g| g.iter().any(|t| t.0 == tool)).unwrap_or(0)
}

fn dsq(a: Pos2, b: Pos2) -> f32 {
    a.distance(b)
}

impl Motion {
    pub(crate) fn set_tool(&mut self, tool: Tool) {
        if !matches!(tool, Tool::Pen | Tool::PenAdd | Tool::PenDel | Tool::PenConv) {
            self.pen = None;
        }
        let g = group_of(tool);
        self.tool_mem[g] = TOOL_GROUPS[g].iter().position(|t| t.0 == tool).unwrap_or(0);
        self.tool = tool;
    }

    pub(crate) fn cycle_group(&mut self, g: usize) {
        let group = TOOL_GROUPS[g];
        let k = if group_of(self.tool) == g { (self.tool_mem[g] + 1) % group.len() } else { self.tool_mem[g].min(group.len() - 1) };
        self.set_tool(group[k].0);
    }

    pub(crate) fn viewer(&mut self, root: &mut Ui) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let (strip_w, bar_h, controls_h) = (46.0, 40.0, 54.0);
            let strip = Rect::from_min_max(full.min, pos2(full.left() + strip_w, full.bottom() - controls_h));
            let bar = Rect::from_min_max(pos2(strip.right(), full.top()), pos2(full.right(), full.top() + bar_h));
            ui.painter().rect_filled(strip, CornerRadius::ZERO, Color32::from_rgb(0x1c, 0x1c, 0x1c));
            ui.painter().line_segment([strip.right_top(), strip.right_bottom()], Stroke::new(1.0, LINE));
            ui.painter().rect_filled(bar, CornerRadius::ZERO, Color32::from_rgb(0x1c, 0x1c, 0x1c));
            ui.painter().line_segment([bar.left_bottom(), bar.right_bottom()], Stroke::new(1.0, LINE));
            ui.scope_builder(egui::UiBuilder::new().max_rect(strip.shrink2(vec2(6.0, 8.0))), |ui| {
                ui.set_clip_rect(strip);
                egui::ScrollArea::vertical().id_salt("motion-tools").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    for (g, group) in TOOL_GROUPS.iter().enumerate() {
                        if matches!(g, 3 | 5 | 7) {
                            ui.add_space(3.0);
                            let (r, _) = ui.allocate_exact_size(vec2(24.0, 1.0), Sense::hover());
                            ui.painter().rect_filled(r, CornerRadius::ZERO, LINE);
                            ui.add_space(3.0);
                        }
                        let mut cur = self.tool;
                        if pro::tool_group(ui, 34.0, &mut cur, group, |p, r, c, t| tool_glyph(p, r, t, c)) || cur != self.tool {
                            self.set_tool(cur);
                        }
                    }
                });
                });
            });
            self.options_bar(ui, bar);
            let canvas = Rect::from_min_max(pos2(strip.right(), bar.bottom()), pos2(full.right(), full.bottom() - controls_h));
            self.canvas(ui, canvas);
            self.transport(ui, Rect::from_min_max(pos2(full.left(), full.bottom() - controls_h), full.max));
        });
    }

    fn options_bar(&mut self, ui: &mut Ui, bar: Rect) {
        let right_w = (bar.width() - 130.0).clamp(240.0, 560.0);
        let left = Rect::from_min_max(bar.min + vec2(12.0, 0.0), pos2(bar.right() - right_w, bar.bottom()));
        let right = Rect::from_min_max(pos2(bar.right() - right_w, bar.top()), bar.max - vec2(8.0, 0.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
            ui.set_clip_rect(left);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let name = TOOL_GROUPS.iter().flat_map(|g| g.iter()).find(|t| t.0 == self.tool).map_or("", |t| t.1);
                ui.label(RichText::new(name).font(semibold(12.0)).color(INK));
                pro::toolbar_sep(ui);
                let lab = |ui: &mut Ui, s: &str| {
                    ui.add(egui::Label::new(RichText::new(s).size(12.0).color(MUTED)).truncate());
                };
                let o = &mut self.topts;
                match self.tool {
                    Tool::Select => lab(ui, "Glisser : déplacer · coins : échelle · rond : rotation · Maj : axe"),
                    Tool::Hand | Tool::ZoomIn | Tool::ZoomOut => {
                        lab(ui, if self.tool == Tool::Hand { "Glisser pour déplacer la vue" } else { "Clic : zoom · Alt+clic : inverse" });
                        for (n, z) in [("Ajuster", None), ("50 %", Some(0.5)), ("100 %", Some(1.0)), ("200 %", Some(2.0))] {
                            if ui.small_button(n).clicked() {
                                self.zoom = z;
                                self.pan = Vec2::ZERO;
                            }
                        }
                    }
                    Tool::Rotate => lab(ui, "Glisser pour tourner le calque · Maj : pas de 15°"),
                    Tool::Anchor => {
                        lab(ui, "Glisser l'ancre sans bouger le calque · Placer :");
                        for (n, k) in [("Centre", 4u8), ("Haut", 1), ("Bas", 7), ("Gauche", 3), ("Droite", 5)] {
                            if ui.small_button(n).clicked() {
                                self.anchor_to(k);
                            }
                        }
                    }
                    Tool::Rect | Tool::RoundRect | Tool::Ellipse | Tool::Polygon | Tool::Star => {
                        lab(ui, "Fond");
                        color_button(ui, &mut o.fill);
                        ui.checkbox(&mut o.stroke_on, "Contour");
                        if o.stroke_on {
                            color_button(ui, &mut o.stroke);
                            ui.add(egui::DragValue::new(&mut o.stroke_w).range(0.5..=80.0).speed(0.2).suffix(" px"));
                        }
                        match self.tool {
                            Tool::RoundRect => {
                                ui.add(egui::DragValue::new(&mut o.round).range(0.0..=500.0).prefix("Arrondi ").suffix(" px"));
                            }
                            Tool::Polygon => {
                                ui.add(egui::DragValue::new(&mut o.sides).range(3..=24).prefix("Côtés "));
                            }
                            Tool::Star => {
                                ui.add(egui::DragValue::new(&mut o.points).range(3..=24).prefix("Branches "));
                                ui.add(egui::DragValue::new(&mut o.inner).range(0.1..=0.95).speed(0.01).prefix("Creux "));
                            }
                            _ => {}
                        }
                        ui.checkbox(&mut o.from_center, "Depuis le centre");
                    }
                    Tool::Pen | Tool::PenAdd | Tool::PenDel | Tool::PenConv => {
                        ui.checkbox(&mut o.pen_fill, "Remplir");
                        if o.pen_fill {
                            color_button(ui, &mut o.pen_color);
                        }
                        lab(ui, "Contour");
                        color_button(ui, &mut o.pen_stroke);
                        ui.add(egui::DragValue::new(&mut o.pen_w).range(0.0..=80.0).speed(0.2).suffix(" px"));
                        if self.pen.is_some() && ui.small_button("Terminer (Entrée)").clicked() {
                            self.pen = None;
                        }
                        if self.pen.is_none() {
                            lab(ui, match self.tool {
                                Tool::Pen => "Clic : sommet · glisser : courbe · clic sur le 1er : fermer",
                                Tool::PenAdd => "Clic sur un segment du tracé sélectionné",
                                Tool::PenDel => "Clic sur un sommet pour le retirer",
                                _ => "Clic : angle ↔ courbe · glisser : nouvelles poignées",
                            });
                        }
                    }
                    Tool::TextH | Tool::TextV => {
                        egui::ComboBox::from_id_salt("tool-font").width(120.0).selected_text(o.font.as_str()).show_ui(ui, |ui| {
                            for f in FONTS {
                                ui.selectable_value(&mut o.font, f.to_string(), f);
                            }
                        });
                        ui.add(egui::DragValue::new(&mut o.text_size).range(8.0..=600.0).suffix(" px"));
                        color_button(ui, &mut o.text_color);
                        lab(ui, "Clic dans la vue pour écrire");
                    }
                    Tool::Brush => {
                        ui.add(egui::DragValue::new(&mut o.brush).range(0.5..=200.0).speed(0.3).prefix("Taille ").suffix(" px"));
                        color_button(ui, &mut o.brush_color);
                        ui.add(egui::DragValue::new(&mut o.smooth).range(0..=8).prefix("Lissage "));
                        lab(ui, "Peint sur le calque Peinture sélectionné");
                    }
                    Tool::MaskRect | Tool::MaskEllipse => {
                        ui.checkbox(&mut o.mask_invert, "Inversé");
                        lab(ui, "Sélectionne un calque puis trace : seule la zone dessinée reste visible");
                    }
                    Tool::Puppet => {
                        ui.add(egui::DragValue::new(&mut o.puppet).range(10.0..=1000.0).speed(1.0).prefix("Portée ").suffix(" px"));
                        lab(ui, "Tire une partie d'un tracé ou d'une peinture : les points proches suivent en douceur");
                    }
                    Tool::Eraser => {
                        ui.add(egui::DragValue::new(&mut o.eraser).range(2.0..=400.0).speed(0.5).prefix("Taille ").suffix(" px"));
                        lab(ui, "Efface les traits du calque Peinture");
                    }
                }
            });
        });
        let fits = (((right.width() - 236.0) / 32.0).floor().max(0.0)) as usize;
        ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
            ui.set_clip_rect(right.intersect(ui.clip_rect()));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let toggles: [(&mut bool, G, &str); 9] = [
                    (&mut self.snap, G::Magnet, "Magnétisme"),
                    (&mut self.paths, G::Path, "Trajectoires de mouvement"),
                    (&mut self.onion, G::Onion, "Pelure d'oignon : images voisines en transparence"),
                    (&mut self.checker, G::Checker, "Damier de transparence"),
                    (&mut self.user_guides, G::Ruler2, "Afficher les repères tirés des règles"),
                    (&mut self.rulers, G::Ruler, "Règles (glisser pour créer un repère)"),
                    (&mut self.guides, G::Guides, "Axes centraux"),
                    (&mut self.safe, G::Safe, "Zones sûres"),
                    (&mut self.grid, G::Grid, "Grille et tiers"),
                ];
                for (on, g, tip) in toggles.into_iter().take(fits) {
                    if pro::icon_button(ui, 30.0, *on, tip, |p, r, c| glyph(p, r, g, c)).clicked() {
                        *on = !*on;
                    }
                }
                pro::toolbar_sep(ui);
                egui::ComboBox::from_id_salt("quality").width(78.0).selected_text(QUALITY[self.quality]).show_ui(ui, |ui| {
                    for (k, q) in QUALITY.iter().enumerate() {
                        ui.selectable_value(&mut self.quality, k, *q);
                    }
                });
                let zl = self.zoom.map_or("Ajuster".to_string(), |z| format!("{:.0} %", z * 100.0));
                egui::ComboBox::from_id_salt("zoom").width(78.0).selected_text(zl).show_ui(ui, |ui| {
                    if ui.selectable_label(self.zoom.is_none(), "Ajuster").clicked() {
                        self.zoom = None;
                        self.pan = Vec2::ZERO;
                    }
                    for z in ZOOMS {
                        if ui.selectable_label(self.zoom == Some(z), format!("{:.0} %", z * 100.0)).clicked() {
                            self.zoom = Some(z);
                        }
                    }
                });
            });
        });
    }

    pub(crate) fn anchor_to(&mut self, k: u8) {
        let Some(i) = self.sel else { return };
        let t = self.t;
        let l = &self.comp.layers[i];
        let (x0, y0, x1, y1) = l.bounds();
        let (ax, ay) = (x0 + (x1 - x0) * (k % 3) as f32 / 2.0, y0 + (y1 - y0) * (k / 3) as f32 / 2.0);
        let own = l.own_matrix(t, i);
        let (oax, oay) = (l.value(8, t), l.value(9, t));
        let (dx, dy) = (ax - oax, ay - oay);
        let (px, py) = (l.props[0].at(t) + own[0] * dx + own[2] * dy, l.props[1].at(t) + own[1] * dx + own[3] * dy);
        let l = &mut self.comp.layers[i];
        l.props[8].set(t, ax);
        l.props[9].set(t, ay);
        l.props[0].set(t, px);
        l.props[1].set(t, py);
    }

    fn hit_layer(&self, cx: f32, cy: f32, s: f32) -> Option<usize> {
        let t = self.t;
        let solo = self.comp.layers.iter().any(|l| l.solo);
        (0..self.comp.layers.len()).rev().find(|&i| {
            let l = &self.comp.layers[i];
            if !l.active(t) || l.locked || (solo && !l.solo) || (i > 0 && self.comp.layers[i - 1].fx.matte > 0) || matches!(l.content, Content::Adjust { .. }) {
                return false;
            }
            let (lx, ly) = apply(invert(self.comp.matrix(i, t)), cx, cy);
            let (x0, y0, x1, y1) = l.bounds();
            let tol = 4.0 / s;
            lx >= x0 - tol && lx <= x1 + tol && ly >= y0 - tol && ly <= y1 + tol
        })
    }

    fn path_of(&mut self, li: usize) -> Option<(&mut Vec<Vtx>, &mut bool)> {
        match &mut self.comp.layers.get_mut(li)?.content {
            Content::Path { pts, closed, .. } => Some((pts, closed)),
            _ => None,
        }
    }

    fn canvas(&mut self, ui: &mut Ui, canvas: Rect) {
        let ruler = if self.rulers { 18.0 } else { 0.0 };
        let inner = Rect::from_min_max(canvas.min + vec2(ruler, ruler), canvas.max);
        let area = inner.shrink(22.0);
        let (cw, ch) = (self.comp.width as f32, self.comp.height as f32);
        let ppp = ui.ctx().pixels_per_point();
        let fit = (area.width() / cw).min(area.height() / ch).max(0.01);
        let s = self.zoom.map_or(fit, |z| z / ppp);
        let view = Rect::from_center_size(area.center() + if self.zoom.is_some() { self.pan } else { Vec2::ZERO }, vec2(cw, ch) * s);
        let rs = (s * ppp * [1.0, 0.5, 0.25][self.quality.min(2)]).min(2.0);
        let (rw, rh) = (((cw * rs).round() as u32).max(1), ((ch * rs).round() as u32).max(1));
        let t = self.t;
        let transparent = self.checker;
        let stale = self.rendered.as_ref().is_none_or(|(c, rt, w, tr)| *c != self.comp || (*rt - t).abs() > 1e-4 || *w != rw || *tr != transparent);
        if stale {
            match render(&self.comp, t, rw, rh, &self.fontdb, transparent) {
                Ok(pix) => {
                    let img = egui::ColorImage::from_rgba_premultiplied([pix.width() as usize, pix.height() as usize], pix.data());
                    match &mut self.tex {
                        Some(tx) => tx.set(img, TextureOptions::LINEAR),
                        None => self.tex = Some(ui.ctx().load_texture("frame", img, TextureOptions::LINEAR)),
                    }
                    self.rendered = Some((self.comp.clone(), t, rw, transparent));
                }
                Err(e) => self.error = Some(format!("Rendu : {e}")),
            }
        }
        let onion_on = self.onion && !self.playing;
        if onion_on {
            let key = (self.comp.clone(), t, rw, self.onion_n, self.onion_step);
            if self.onion_key.as_ref() != Some(&key) {
                let fps = self.comp.fps as f32;
                let mut acc = vec![[0.0f32; 4]; (rw * rh) as usize];
                for k in (1..=self.onion_n).rev() {
                    for (dir, tint) in [(-1.0f32, [255.0, 90.0, 90.0]), (1.0, [90.0, 200.0, 255.0])] {
                        let tf = t + dir * (k * self.onion_step) as f32 / fps;
                        if tf < 0.0 || tf > self.comp.duration {
                            continue;
                        }
                        let w = 0.45 / k as f32;
                        if let Ok(pix) = render(&self.comp, tf, rw, rh, &self.fontdb, true) {
                            for (d, px) in acc.iter_mut().zip(pix.pixels()) {
                                let a = px.alpha() as f32 / 255.0 * w;
                                if a <= 0.0 {
                                    continue;
                                }
                                for c in 0..3 {
                                    d[c] = tint[c] / 255.0 * a + d[c] * (1.0 - a);
                                }
                                d[3] = a + d[3] * (1.0 - a);
                            }
                        }
                    }
                }
                let bytes: Vec<u8> = acc.iter().flat_map(|p| p.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8)).collect();
                let img = egui::ColorImage::from_rgba_premultiplied([rw as usize, rh as usize], &bytes);
                match &mut self.onion_tex {
                    Some(tx) => tx.set(img, TextureOptions::LINEAR),
                    None => self.onion_tex = Some(ui.ctx().load_texture("onion", img, TextureOptions::LINEAR)),
                }
                self.onion_key = Some(key);
            }
        }
        let painter = ui.painter_at(inner);
        painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(140) }.as_shape(view, CornerRadius::same(4)));
        if transparent {
            let cell = 10.0;
            let vis = view.intersect(inner);
            let (nx, ny) = (((vis.width() / cell).ceil() as i32).max(0), ((vis.height() / cell).ceil() as i32).max(0));
            painter.rect_filled(vis, CornerRadius::ZERO, Color32::from_gray(200));
            let ox = ((vis.left() - view.left()) / cell).floor() as i32;
            let oy = ((vis.top() - view.top()) / cell).floor() as i32;
            for y in 0..=ny {
                for x in 0..=nx {
                    if (x + ox + y + oy) % 2 == 0 {
                        let r = Rect::from_min_size(pos2(view.left() + (x + ox) as f32 * cell, view.top() + (y + oy) as f32 * cell), Vec2::splat(cell)).intersect(view);
                        painter.rect_filled(r, CornerRadius::ZERO, Color32::from_gray(150));
                    }
                }
            }
        }
        if let Some(tx) = &self.tex {
            painter.image(tx.id(), view, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        if onion_on && let Some(tx) = &self.onion_tex {
            painter.image(tx.id(), view, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        let to_scr = |x: f32, y: f32| pos2(view.min.x + x * s, view.min.y + y * s);
        let to_comp = |p: Pos2| ((p.x - view.min.x) / s, (p.y - view.min.y) / s);
        let thin = Stroke::new(1.0, Color32::from_white_alpha(46));
        if self.grid {
            let n = self.grid_div.max(2);
            for k in 1..n {
                let f = k as f32 / n as f32;
                painter.line_segment([to_scr(cw * f, 0.0), to_scr(cw * f, ch)], Stroke::new(1.0, Color32::from_white_alpha(16)));
                painter.line_segment([to_scr(0.0, ch * f), to_scr(cw, ch * f)], Stroke::new(1.0, Color32::from_white_alpha(16)));
            }
            for f in [1.0 / 3.0, 2.0 / 3.0] {
                painter.line_segment([to_scr(cw * f, 0.0), to_scr(cw * f, ch)], thin);
                painter.line_segment([to_scr(0.0, ch * f), to_scr(cw, ch * f)], thin);
            }
        }
        if self.safe {
            let (a, b) = SAFE[self.safe_kind.min(SAFE.len() - 1)].1;
            for (k, al) in [(a, 70), (b, 110)] {
                painter.rect_stroke(Rect::from_center_size(view.center(), view.size() * k), CornerRadius::ZERO, Stroke::new(1.0, Color32::from_white_alpha(al)), StrokeKind::Middle);
            }
        }
        if self.guides {
            painter.line_segment([to_scr(cw / 2.0, 0.0), to_scr(cw / 2.0, ch)], Stroke::new(1.0, MARK.gamma_multiply(0.6)));
            painter.line_segment([to_scr(0.0, ch / 2.0), to_scr(cw, ch / 2.0)], Stroke::new(1.0, MARK.gamma_multiply(0.6)));
        }
        if self.user_guides {
            for g in &self.comp.guides {
                if g.vertical {
                    painter.line_segment([pos2(to_scr(g.pos, 0.0).x, inner.top()), pos2(to_scr(g.pos, 0.0).x, inner.bottom())], Stroke::new(1.0, GUIDE));
                } else {
                    painter.line_segment([pos2(inner.left(), to_scr(0.0, g.pos).y), pos2(inner.right(), to_scr(0.0, g.pos).y)], Stroke::new(1.0, GUIDE));
                }
            }
        }
        for (i, l) in self.comp.layers.iter().enumerate() {
            if matches!(l.content, Content::Null) && l.active(t) {
                let m = self.comp.matrix(i, t);
                let q: Vec<Pos2> = [(-40.0, -40.0), (40.0, -40.0), (40.0, 40.0), (-40.0, 40.0)].iter().map(|(x, y)| {
                    let (a, b) = apply(m, *x, *y);
                    to_scr(a, b)
                }).collect();
                painter.add(Shape::closed_line(q, Stroke::new(1.0, Color32::from_rgb(255, 120, 120))));
            }
        }

        let sel = self.sel.filter(|i| *i < self.comp.layers.len() && self.comp.layers[*i].active(t));
        let sel_m = sel.map(|i| self.comp.matrix(i, t));
        let quad = sel.zip(sel_m).map(|(i, m)| {
            let (x0, y0, x1, y1) = self.comp.layers[i].bounds();
            let q = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| {
                let (a, b) = apply(m, x, y);
                to_scr(a, b)
            });
            let (cx, cy) = apply(m, (x0 + x1) / 2.0, (y0 + y1) / 2.0);
            (q, to_scr(cx, cy))
        });
        let rot_handle = |q: &[Pos2; 4], c: Pos2| {
            let top = q[0] + (q[1] - q[0]) / 2.0;
            let d = (top - c).normalized();
            let d = if d.length() < 0.5 { vec2(0.0, -1.0) } else { d };
            (top, top + d * 26.0)
        };
        let path_edit = self.sel.and_then(|i| match &self.comp.layers[i].content {
            Content::Path { pts, closed, .. } if matches!(self.tool, Tool::Select | Tool::Pen | Tool::PenAdd | Tool::PenDel | Tool::PenConv) => Some((i, pts.clone(), *closed)),
            _ => None,
        });
        let path_scr: Vec<(Pos2, Pos2, Pos2)> = match (&path_edit, sel_m) {
            (Some((_, pts, _)), Some(m)) => pts
                .iter()
                .map(|v| {
                    let f = |x: f32, y: f32| {
                        let (a, b) = apply(m, x, y);
                        to_scr(a, b)
                    };
                    (f(v.p[0], v.p[1]), f(v.p[0] + v.i[0], v.p[1] + v.i[1]), f(v.p[0] + v.o[0], v.p[1] + v.o[1]))
                })
                .collect(),
            _ => Vec::new(),
        };
        let motion: Vec<(f32, Pos2)> = match sel {
            Some(i) if self.paths && (self.comp.layers[i].props[0].animated() || self.comp.layers[i].props[1].animated()) => {
                let l = &self.comp.layers[i];
                let mut kts: Vec<f32> = l.props[0].keys.iter().chain(&l.props[1].keys).map(|k| k.t).collect();
                kts.sort_by(f32::total_cmp);
                kts.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
                kts.into_iter().filter(|kt| *kt >= l.start - 1e-3 && *kt <= l.end + 1e-3).map(|kt| {
                    let (x, y, _, _) = self.comp.world(i, kt);
                    (kt, to_scr(x, y))
                }).collect()
            }
            _ => Vec::new(),
        };
        if let Some(i) = sel
            && !motion.is_empty()
        {
            let l = &self.comp.layers[i];
            let step = 1.0 / self.comp.fps as f32;
            let n = (((l.end - l.start) / step) as usize).min(900);
            let pts: Vec<Pos2> = (0..=n).map(|k| {
                let (x, y, _, _) = self.comp.world(i, l.start + k as f32 * step);
                to_scr(x, y)
            }).collect();
            painter.add(Shape::line(pts.clone(), Stroke::new(1.0, Color32::from_white_alpha(70))));
            for p in pts.iter().step_by(2) {
                painter.circle_filled(*p, 1.4, Color32::from_white_alpha(140));
            }
            for (_, p) in &motion {
                painter.rect(Rect::from_center_size(*p, Vec2::splat(7.0)), CornerRadius::same(1), KEY, Stroke::new(1.0, Color32::from_black_alpha(150)), StrokeKind::Middle);
            }
        }
        if let Some((q, c)) = quad {
            painter.add(Shape::closed_line(q.to_vec(), Stroke::new(1.0, BLUE)));
            if path_edit.is_none() {
                for p in q {
                    painter.rect(Rect::from_center_size(p, Vec2::splat(8.0)), CornerRadius::same(2), Color32::WHITE, Stroke::new(1.0, BLUE), StrokeKind::Inside);
                }
                let (top, rh) = rot_handle(&q, c);
                painter.line_segment([top, rh], Stroke::new(1.0, BLUE));
                painter.circle(rh, 5.0, Color32::WHITE, Stroke::new(1.0, BLUE));
            }
            if let Some(i) = sel {
                let (ax, ay) = (self.comp.layers[i].value(0, t), self.comp.layers[i].value(1, t));
                let (px, py) = apply(self.comp.parent_matrix(i, t), ax, ay);
                let ap = to_scr(px, py);
                painter.circle_stroke(ap, 5.0, Stroke::new(1.2, Color32::WHITE));
                painter.line_segment([ap - vec2(9.0, 0.0), ap + vec2(9.0, 0.0)], Stroke::new(1.0, Color32::WHITE));
                painter.line_segment([ap - vec2(0.0, 9.0), ap + vec2(0.0, 9.0)], Stroke::new(1.0, Color32::WHITE));
                if let Some(p) = self.comp.layers[i].parent
                    && p < self.comp.layers.len()
                {
                    let (x, y, _, _) = self.comp.world(p, t);
                    painter.line_segment([ap, to_scr(x, y)], Stroke::new(1.0, Color32::from_white_alpha(60)));
                }
            }
        }
        for (k, (p, hi, ho)) in path_scr.iter().enumerate() {
            for h in [hi, ho] {
                if h.distance(*p) > 0.5 {
                    painter.line_segment([*p, *h], Stroke::new(1.0, BLUE.gamma_multiply(0.8)));
                    painter.circle(*h, 3.5, Color32::WHITE, Stroke::new(1.0, BLUE));
                }
            }
            let first = k == 0 && self.pen.is_some();
            painter.rect(Rect::from_center_size(*p, Vec2::splat(if first { 9.0 } else { 7.0 })), CornerRadius::ZERO, if first { BLUE } else { Color32::WHITE }, Stroke::new(1.0, BLUE), StrokeKind::Middle);
        }

        let resp = ui.interact(canvas, ui.make_persistent_id("view"), Sense::click_and_drag());
        let (shift, alt, ptr, scroll, cmd, middle, origin) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt, i.pointer.interact_pos(), i.smooth_scroll_delta.y, i.modifiers.command, i.pointer.middle_down(), i.pointer.press_origin()));
        if resp.hovered() {
            if cmd && scroll != 0.0 {
                self.zoom = Some((self.zoom.unwrap_or(fit * ppp) * (1.0 + scroll * 0.002)).clamp(0.05, 8.0));
            }
            ui.ctx().set_cursor_icon(match self.tool {
                Tool::Hand => egui::CursorIcon::Grab,
                Tool::ZoomIn => egui::CursorIcon::ZoomIn,
                Tool::ZoomOut => egui::CursorIcon::ZoomOut,
                Tool::Rotate => egui::CursorIcon::Alias,
                Tool::Anchor => egui::CursorIcon::Move,
                Tool::TextH | Tool::TextV => egui::CursorIcon::Text,
                Tool::Select => egui::CursorIcon::Default,
                _ => egui::CursorIcon::Crosshair,
            });
            if let (Some(p), Tool::Brush | Tool::Eraser) = (ptr, self.tool) {
                let r = if self.tool == Tool::Brush { self.topts.brush } else { self.topts.eraser } * s / 2.0;
                painter.circle_stroke(p, r.max(2.0), Stroke::new(1.0, Color32::WHITE));
            }
        }
        let near_vertex = |p: Pos2| -> Option<(usize, u8)> {
            for (k, (v, hi, ho)) in path_scr.iter().enumerate() {
                if ho.distance(*v) > 0.5 && dsq(*ho, p) < 7.0 {
                    return Some((k, 2));
                }
                if hi.distance(*v) > 0.5 && dsq(*hi, p) < 7.0 {
                    return Some((k, 1));
                }
                if dsq(*v, p) < 7.0 {
                    return Some((k, 0));
                }
            }
            None
        };

        if resp.drag_started()
            && let Some(p) = origin
        {
            self.vdrag = None;
            let (cx, cy) = to_comp(p);
            if self.rulers && !inner.contains(p) {
                self.user_guides = true;
                self.vdrag = Some(VDrag::NewGuide { vertical: p.x < inner.left() });
            } else if self.tool == Tool::Hand || middle {
                if self.zoom.is_none() {
                    self.zoom = Some(fit * ppp);
                }
                self.vdrag = Some(VDrag::Pan { p0: self.pan, from: p });
            } else {
                match self.tool {
                    Tool::Select | Tool::Rotate => {
                        let locked = self.sel.is_some_and(|i| self.comp.layers[i].locked);
                        if let (Some((li, _, _)), Some(m), Some((k, part))) = (&path_edit, sel_m, near_vertex(p))
                            && self.tool == Tool::Select
                            && !locked
                        {
                            self.vdrag = Some(VDrag::Vertex { li: *li, vi: k, part, inv: invert(m) });
                        } else if let (Some(i), Some(kt)) = (sel, motion.iter().find(|(_, q)| q.distance(p) < 7.0).map(|m| m.0))
                            && self.tool == Tool::Select
                            && !locked
                        {
                            self.vdrag = Some(VDrag::PathKey { li: i, kt });
                        } else if let (Some(i), Some((q, c))) = (sel, quad)
                            && !locked
                        {
                            if self.tool == Tool::Rotate || (path_edit.is_none() && rot_handle(&q, c).1.distance(p) < 9.0) {
                                self.vdrag = Some(VDrag::Rotate { r0: self.comp.layers[i].props[3].at(t), a0: (p - c).angle().to_degrees(), c });
                            } else if path_edit.is_none() && q.iter().any(|v| v.distance(p) < 9.0) {
                                self.vdrag = Some(VDrag::Scale { ws: self.comp.world(i, t).2, d0: c.distance(p).max(1.0), c });
                            }
                        }
                        if self.vdrag.is_none() && self.tool == Tool::Select && self.user_guides {
                            let gi = self.comp.guides.iter().position(|g| if g.vertical { (to_scr(g.pos, 0.0).x - p.x).abs() < 4.0 } else { (to_scr(0.0, g.pos).y - p.y).abs() < 4.0 });
                            if let Some(gi) = gi
                                && self.hit_layer(cx, cy, s).is_none_or(|h| Some(h) != self.sel)
                            {
                                self.vdrag = Some(VDrag::Guide { gi });
                            }
                        }
                        if self.vdrag.is_none()
                            && self.tool == Tool::Select
                            && let Some(i) = self.hit_layer(cx, cy, s)
                        {
                            self.sel = Some(i);
                            let (wx, wy, _, _) = self.comp.world(i, t);
                            self.vdrag = Some(VDrag::Move { wx, wy, from: p });
                        }
                    }
                    Tool::Anchor => {
                        let target = self.sel.filter(|i| !self.comp.layers[*i].locked).or_else(|| self.hit_layer(cx, cy, s));
                        if let Some(i) = target {
                            self.sel = Some(i);
                            let l = &self.comp.layers[i];
                            let own = l.own_matrix(t, i);
                            self.vdrag = Some(VDrag::Anchor { inv: invert(self.comp.matrix(i, t)), lin: [own[0], own[1], own[2], own[3]], pos: (l.props[0].at(t), l.props[1].at(t)), anc: (l.props[8].at(t), l.props[9].at(t)) });
                        }
                    }
                    Tool::Rect | Tool::RoundRect | Tool::Ellipse | Tool::Polygon | Tool::Star => self.vdrag = Some(VDrag::Shape { from: (cx, cy) }),
                    Tool::MaskRect | Tool::MaskEllipse => {
                        if self.sel.is_none() {
                            self.sel = self.hit_layer(cx, cy, s);
                        }
                        self.vdrag = Some(VDrag::Shape { from: (cx, cy) });
                    }
                    Tool::Puppet => {
                        let target = self.sel.filter(|i| matches!(self.comp.layers[*i].content, Content::Path { .. } | Content::Strokes { .. })).or_else(|| self.hit_layer(cx, cy, s));
                        if let Some(i) = target.filter(|i| matches!(self.comp.layers[*i].content, Content::Path { .. } | Content::Strokes { .. }) && !self.comp.layers[*i].locked) {
                            self.sel = Some(i);
                            let inv = invert(self.comp.matrix(i, t));
                            let ws = self.comp.world(i, t).2.abs().max(1e-3);
                            self.vdrag = Some(VDrag::Puppet { li: i, inv, last: apply(inv, cx, cy), r: self.topts.puppet / ws });
                        }
                    }
                    Tool::Pen => {
                        if let Some((li, k)) = self.pen_click(cx, cy, p, &path_scr) {
                            if let Some(m) = Some(self.comp.matrix(li, t)) {
                                self.vdrag = Some(VDrag::Vertex { li, vi: k, part: 3, inv: invert(m) });
                            }
                        }
                    }
                    Tool::PenConv => {
                        if let (Some((li, _, _)), Some(m), Some((k, part))) = (&path_edit, sel_m, near_vertex(p)) {
                            let part = if part == 0 { 3 } else { part };
                            if part == 3
                                && let Some((pts, _)) = self.path_of(*li)
                            {
                                pts[k].i = [0.0; 2];
                                pts[k].o = [0.0; 2];
                            }
                            self.vdrag = Some(VDrag::Vertex { li: *li, vi: k, part, inv: invert(m) });
                        }
                    }
                    Tool::Brush => {
                        let li = match self.sel {
                            Some(i) if matches!(self.comp.layers[i].content, Content::Strokes { .. }) && !self.comp.layers[i].locked => i,
                            _ => {
                                let o = &self.topts;
                                let i = self.add_layer("Peinture", Content::Strokes { strokes: Vec::new(), width: o.brush, color: o.brush_color });
                                let l = &mut self.comp.layers[i];
                                l.start = 0.0;
                                i
                            }
                        };
                        let inv = invert(self.comp.matrix(li, t));
                        let lp = apply(inv, cx, cy);
                        if let Content::Strokes { strokes, .. } = &mut self.comp.layers[li].content {
                            strokes.push(vec![[lp.0, lp.1]]);
                        }
                        self.vdrag = Some(VDrag::Brush { li, inv });
                    }
                    Tool::Eraser => {
                        if let Some(i) = self.sel.filter(|i| matches!(self.comp.layers[*i].content, Content::Strokes { .. })) {
                            let ws = self.comp.world(i, t).2.abs().max(1e-3);
                            self.vdrag = Some(VDrag::Erase { li: i, inv: invert(self.comp.matrix(i, t)), r: self.topts.eraser / 2.0 / ws });
                        }
                    }
                    _ => {}
                }
            }
        }
        let snap_on = self.snap;
        if resp.dragged()
            && let (Some(d), Some(p)) = (self.vdrag, ptr)
        {
            let (cx, cy) = to_comp(p);
            match d {
                VDrag::Pan { p0, from } => self.pan = p0 + (p - from),
                VDrag::Move { wx, wy, from } => {
                    if let Some(i) = self.sel {
                        let mut dx = (p.x - from.x) / s;
                        let mut dy = (p.y - from.y) / s;
                        if shift {
                            if dx.abs() > dy.abs() {
                                dy = 0.0;
                            } else {
                                dx = 0.0;
                            }
                        }
                        let (mut nx, mut ny) = (wx + dx, wy + dy);
                        if snap_on {
                            let (_, _, ws, _) = self.comp.world(i, t);
                            let (w, h) = self.comp.layers[i].size();
                            let tol = 8.0 / s;
                            let gx: Vec<f32> = self.comp.guides.iter().filter(|g| g.vertical && self.user_guides).map(|g| g.pos).collect();
                            let gy: Vec<f32> = self.comp.guides.iter().filter(|g| !g.vertical && self.user_guides).map(|g| g.pos).collect();
                            for c in [cw / 2.0, w * ws / 2.0, cw - w * ws / 2.0].into_iter().chain(gx) {
                                if (nx - c).abs() < tol {
                                    nx = c;
                                    painter.line_segment([to_scr(c, 0.0), to_scr(c, ch)], Stroke::new(1.0, SNAP));
                                    break;
                                }
                            }
                            for c in [ch / 2.0, h * ws / 2.0, ch - h * ws / 2.0].into_iter().chain(gy) {
                                if (ny - c).abs() < tol {
                                    ny = c;
                                    painter.line_segment([to_scr(0.0, c), to_scr(cw, c)], Stroke::new(1.0, SNAP));
                                    break;
                                }
                            }
                        }
                        self.place(i, nx, ny);
                    }
                }
                VDrag::Scale { ws, d0, c } => {
                    if let Some(i) = self.sel {
                        let k = c.distance(p) / d0;
                        let ps = self.comp.layers[i].parent.filter(|q| *q < self.comp.layers.len()).map_or(1.0, |q| self.comp.world(q, t).2);
                        let v = (ws * k / ps.max(1e-4) * 100.0 * 10.0).round() / 10.0;
                        self.comp.layers[i].props[2].set(t, v.max(0.1));
                    }
                }
                VDrag::Rotate { r0, a0, c } => {
                    if let Some(i) = self.sel {
                        let mut d = (p - c).angle().to_degrees() - a0;
                        if d > 180.0 {
                            d -= 360.0;
                        }
                        if d < -180.0 {
                            d += 360.0;
                        }
                        let v = r0 + d;
                        let v = if shift { (v / 15.0).round() * 15.0 } else { (v * 10.0).round() / 10.0 };
                        self.comp.layers[i].props[3].set(t, v);
                    }
                }
                VDrag::Anchor { inv, lin, pos, anc } => {
                    if let Some(i) = self.sel {
                        let (ax, ay) = apply(inv, cx, cy);
                        let (dx, dy) = (ax - anc.0, ay - anc.1);
                        let l = &mut self.comp.layers[i];
                        l.props[8].set(t, ax.round());
                        l.props[9].set(t, ay.round());
                        l.props[0].set(t, pos.0 + lin[0] * dx + lin[2] * dy);
                        l.props[1].set(t, pos.1 + lin[1] * dx + lin[3] * dy);
                    }
                }
                VDrag::Shape { from } => {
                    let (r, poly) = self.shape_geom(from, (cx, cy), shift, alt);
                    let a = to_scr(r.0, r.1);
                    let b = to_scr(r.2, r.3);
                    let st = Stroke::new(1.2, BLUE);
                    match poly {
                        Some(pts) => {
                            painter.add(Shape::closed_line(pts.iter().map(|q| to_scr(q[0], q[1])).collect(), st));
                        }
                        None if matches!(self.tool, Tool::Ellipse | Tool::MaskEllipse) => {
                            let c = a + (b - a) / 2.0;
                            let pts: Vec<Pos2> = (0..48).map(|k| {
                                let an = k as f32 / 48.0 * std::f32::consts::TAU;
                                c + vec2(an.cos() * (b.x - a.x).abs() / 2.0, an.sin() * (b.y - a.y).abs() / 2.0)
                            }).collect();
                            painter.add(Shape::closed_line(pts, st));
                        }
                        None => {
                            painter.rect_stroke(Rect::from_two_pos(a, b), CornerRadius::same(if self.tool == Tool::RoundRect { (self.topts.round * s).min(60.0) as u8 } else { 0 }), st, StrokeKind::Middle);
                        }
                    }
                }
                VDrag::Vertex { li, vi, part, inv } => {
                    let (lx, ly) = apply(inv, cx, cy);
                    if let Some((pts, _)) = self.path_of(li)
                        && vi < pts.len()
                    {
                        let v = &mut pts[vi];
                        let h = [lx - v.p[0], ly - v.p[1]];
                        match part {
                            0 => v.p = [lx, ly],
                            1 => {
                                v.i = h;
                                if !alt {
                                    v.o = [-h[0], -h[1]];
                                }
                            }
                            _ => {
                                v.o = h;
                                if !alt || part == 3 {
                                    v.i = [-h[0], -h[1]];
                                }
                            }
                        }
                    }
                }
                VDrag::Brush { li, inv } => {
                    let (lx, ly) = apply(inv, cx, cy);
                    let min = 1.5 / s;
                    if let Some(Content::Strokes { strokes, .. }) = self.comp.layers.get_mut(li).map(|l| &mut l.content)
                        && let Some(st) = strokes.last_mut()
                        && st.last().is_none_or(|q| (q[0] - lx).hypot(q[1] - ly) > min)
                    {
                        st.push([lx, ly]);
                    }
                }
                VDrag::Puppet { li, inv, last, r } => {
                    let (lx, ly) = apply(inv, cx, cy);
                    let (dx, dy) = (lx - last.0, ly - last.1);
                    let pull = |q: &mut [f32; 2]| {
                        let d = (q[0] - last.0).hypot(q[1] - last.1) / r;
                        if d < 1.0 {
                            let w = (1.0 - d * d).powi(2);
                            q[0] += dx * w;
                            q[1] += dy * w;
                        }
                    };
                    match self.comp.layers.get_mut(li).map(|l| &mut l.content) {
                        Some(Content::Path { pts, .. }) => pts.iter_mut().for_each(|v| pull(&mut v.p)),
                        Some(Content::Strokes { strokes, .. }) => strokes.iter_mut().flatten().for_each(pull),
                        _ => {}
                    }
                    self.vdrag = Some(VDrag::Puppet { li, inv, last: (lx, ly), r });
                }
                VDrag::Erase { li, inv, r } => {
                    let (lx, ly) = apply(inv, cx, cy);
                    if let Some(Content::Strokes { strokes, .. }) = self.comp.layers.get_mut(li).map(|l| &mut l.content) {
                        let mut out = Vec::new();
                        for st in strokes.drain(..) {
                            let mut run = Vec::new();
                            for q in st {
                                if (q[0] - lx).hypot(q[1] - ly) < r {
                                    if run.len() > 1 {
                                        out.push(std::mem::take(&mut run));
                                    }
                                    run.clear();
                                } else {
                                    run.push(q);
                                }
                            }
                            if run.len() > 1 {
                                out.push(run);
                            }
                        }
                        *strokes = out;
                    }
                }
                VDrag::Guide { gi } => {
                    if let Some(g) = self.comp.guides.get_mut(gi) {
                        g.pos = (if g.vertical { cx } else { cy }).round();
                    }
                }
                VDrag::NewGuide { vertical } => {
                    if vertical {
                        painter.line_segment([pos2(p.x, inner.top()), pos2(p.x, inner.bottom())], Stroke::new(1.0, GUIDE));
                    } else {
                        painter.line_segment([pos2(inner.left(), p.y), pos2(inner.right(), p.y)], Stroke::new(1.0, GUIDE));
                    }
                }
                VDrag::PathKey { li, kt } => {
                    let (gx, gy) = self.comp.layers[li].wiggle(kt, li);
                    let (lx, ly) = apply(invert(self.comp.parent_matrix(li, kt)), cx, cy);
                    let l = &mut self.comp.layers[li];
                    for (pi, v) in [(0, lx - gx), (1, ly - gy)] {
                        if let Some(k) = l.props[pi].key_at(kt) {
                            l.props[pi].keys[k].v = v.round();
                        }
                    }
                }
            }
        }
        if resp.drag_stopped() {
            if let (Some(d), Some(p)) = (self.vdrag, ptr) {
                let (cx, cy) = to_comp(p);
                match d {
                    VDrag::Shape { from } => self.make_shape(from, (cx, cy), shift, alt),
                    VDrag::Brush { li, .. } => {
                        let n = self.topts.smooth;
                        if let Some(Content::Strokes { strokes, .. }) = self.comp.layers.get_mut(li).map(|l| &mut l.content)
                            && let Some(st) = strokes.last_mut()
                        {
                            for _ in 0..n {
                                if st.len() > 2 {
                                    let old = st.clone();
                                    for k in 1..old.len() - 1 {
                                        st[k] = [(old[k - 1][0] + old[k][0] * 2.0 + old[k + 1][0]) / 4.0, (old[k - 1][1] + old[k][1] * 2.0 + old[k + 1][1]) / 4.0];
                                    }
                                }
                            }
                            if st.len() == 1 {
                                let q = st[0];
                                st.push([q[0] + 0.01, q[1]]);
                            }
                        }
                    }
                    VDrag::Guide { gi } => {
                        if !inner.contains(p) && gi < self.comp.guides.len() {
                            self.comp.guides.remove(gi);
                        }
                    }
                    VDrag::NewGuide { vertical } => {
                        if inner.contains(p) {
                            self.comp.guides.push(Guide { vertical, pos: (if vertical { cx } else { cy }).round() });
                        }
                    }
                    _ => {}
                }
            }
            self.vdrag = None;
        }
        if resp.clicked()
            && let Some(p) = ptr
        {
            let (cx, cy) = to_comp(p);
            match self.tool {
                Tool::Select => self.sel = self.hit_layer(cx, cy, s),
                Tool::ZoomIn | Tool::ZoomOut => {
                    let out = (self.tool == Tool::ZoomOut) != alt;
                    let z0 = self.zoom.unwrap_or(fit * ppp);
                    let z = ZOOMS.iter().copied().filter(|z| if out { *z < z0 - 1e-3 } else { *z > z0 + 1e-3 }).fold(None, |acc: Option<f32>, z| Some(match acc { None => z, Some(a) => if out { a.max(z) } else { a.min(z) } })).unwrap_or(z0);
                    let ns = z / ppp;
                    let center = area.center() + if self.zoom.is_some() { self.pan } else { Vec2::ZERO };
                    let rel = p - center;
                    self.pan = (center - area.center()) + rel - rel * (ns / s);
                    self.zoom = Some(z);
                }
                Tool::Rect | Tool::RoundRect | Tool::Ellipse | Tool::Polygon | Tool::Star => {
                    let r = 100.0;
                    self.make_shape((cx - r, cy - r), (cx + r, cy + r), true, false);
                }
                Tool::Pen => {
                    self.pen_click(cx, cy, p, &path_scr);
                }
                Tool::PenAdd => {
                    if let (Some((li, pts, closed)), Some(m)) = (&path_edit, sel_m) {
                        let n = pts.len();
                        let segs = if *closed && n > 2 { n } else { n.saturating_sub(1) };
                        let mut best: Option<(f32, usize, f32)> = None;
                        for sg in 0..segs {
                            let sub = model::path_points(&[pts[sg], pts[(sg + 1) % n]], false);
                            let m2 = sub.len().max(2) - 1;
                            for (k, q) in sub.iter().enumerate() {
                                let (a, b) = apply(m, q[0], q[1]);
                                let d = to_scr(a, b).distance(p);
                                if best.is_none_or(|bb| d < bb.0) {
                                    best = Some((d, sg, k as f32 / m2 as f32));
                                }
                            }
                        }
                        if let Some((d, sg, u)) = best
                            && d < 10.0
                            && let Some((pts, _)) = self.path_of(*li)
                        {
                            model::split_segment(pts, sg, u.clamp(0.02, 0.98));
                        }
                    }
                }
                Tool::PenDel => {
                    if let (Some((li, _, _)), Some((k, 0))) = (&path_edit, near_vertex(p))
                        && let Some((pts, _)) = self.path_of(*li)
                        && pts.len() > 1
                    {
                        pts.remove(k);
                    }
                }
                Tool::PenConv => {
                    if let (Some((li, _, closed)), Some((k, 0))) = (&path_edit, near_vertex(p))
                        && let Some((pts, _)) = self.path_of(*li)
                    {
                        if pts[k].corner() {
                            smooth_vertex(pts, k, *closed);
                        } else {
                            pts[k].i = [0.0; 2];
                            pts[k].o = [0.0; 2];
                        }
                    }
                }
                Tool::TextH | Tool::TextV => {
                    match self.hit_layer(cx, cy, s).filter(|i| matches!(self.comp.layers[*i].content, Content::Text { .. })) {
                        Some(i) => self.sel = Some(i),
                        None => {
                            let o = &self.topts;
                            let mut c = Content::text("Texte", o.text_size, o.text_color, true).styled(&o.font.clone(), false, 0.0, 0);
                            if let Content::Text { vertical, .. } = &mut c {
                                *vertical = self.tool == Tool::TextV;
                            }
                            let i = self.add_layer("Texte", c);
                            self.place(i, cx, cy);
                        }
                    }
                    self.focus_text = true;
                }
                _ => {}
            }
        }
        if resp.double_clicked() && matches!(self.tool, Tool::Select | Tool::Hand) {
            self.zoom = None;
            self.pan = Vec2::ZERO;
        }
        if self.rulers {
            let top = Rect::from_min_max(pos2(inner.left(), canvas.top()), pos2(canvas.right(), inner.top()));
            let left = Rect::from_min_max(pos2(canvas.left(), inner.top()), pos2(inner.left(), canvas.bottom()));
            let rp = ui.painter_at(canvas);
            rp.rect_filled(top, CornerRadius::ZERO, Color32::from_rgb(0x1a, 0x1a, 0x1a));
            rp.rect_filled(left, CornerRadius::ZERO, Color32::from_rgb(0x1a, 0x1a, 0x1a));
            rp.rect_filled(Rect::from_min_max(canvas.min, inner.min), CornerRadius::ZERO, Color32::from_rgb(0x1a, 0x1a, 0x1a));
            let step = [10.0, 25.0, 50.0, 100.0, 200.0, 500.0, 1000.0].into_iter().find(|v| v * s >= 60.0).unwrap_or(2000.0);
            let mut v = 0.0;
            while v <= cw.max(ch) + 1e-3 {
                if v <= cw {
                    let x = to_scr(v, 0.0).x;
                    if x >= top.left() && x <= top.right() {
                        rp.line_segment([pos2(x, top.bottom() - 7.0), pos2(x, top.bottom())], Stroke::new(1.0, FAINT));
                        rp.text(pos2(x + 2.0, top.top() + 1.0), Align2::LEFT_TOP, format!("{v:.0}"), FontId::monospace(9.0), MUTED);
                    }
                }
                if v <= ch {
                    let y = to_scr(0.0, v).y;
                    if y >= left.top() && y <= left.bottom() {
                        rp.line_segment([pos2(left.right() - 7.0, y), pos2(left.right(), y)], Stroke::new(1.0, FAINT));
                        rp.text(pos2(left.left() + 1.0, y + 2.0), Align2::LEFT_TOP, format!("{v:.0}"), FontId::monospace(8.0), MUTED);
                    }
                }
                v += step;
            }
            if let Some(p) = ptr.filter(|p| inner.contains(*p)) {
                rp.line_segment([pos2(p.x, top.top()), pos2(p.x, top.bottom())], Stroke::new(1.0, PLAYHEAD));
                rp.line_segment([pos2(left.left(), p.y), pos2(left.right(), p.y)], Stroke::new(1.0, PLAYHEAD));
            }
            rp.line_segment([top.left_bottom(), top.right_bottom()], Stroke::new(1.0, LINE));
            rp.line_segment([left.right_top(), left.right_bottom()], Stroke::new(1.0, LINE));
        }
        if let Some(p) = ptr.filter(|p| inner.contains(*p))
            && resp.hovered()
        {
            let (x, y) = to_comp(p);
            self.cursor = Some((x, y));
        } else {
            self.cursor = None;
        }
    }

    fn pen_click(&mut self, cx: f32, cy: f32, p: Pos2, path_scr: &[(Pos2, Pos2, Pos2)]) -> Option<(usize, usize)> {
        let t = self.t;
        if let Some(li) = self.pen.filter(|li| *li < self.comp.layers.len() && matches!(self.comp.layers[*li].content, Content::Path { closed: false, .. })) {
            if path_scr.len() >= 2 && path_scr[0].0.distance(p) < 9.0 {
                if let Some((_, closed)) = self.path_of(li) {
                    *closed = true;
                }
                self.pen = None;
                return Some((li, 0));
            }
            let (lx, ly) = apply(invert(self.comp.matrix(li, t)), cx, cy);
            let (pts, _) = self.path_of(li)?;
            pts.push(Vtx::at(lx, ly));
            return Some((li, pts.len() - 1));
        }
        if let Some(li) = self.sel
            && matches!(self.comp.layers[li].content, Content::Path { closed: false, .. })
            && path_scr.last().is_some_and(|q| q.0.distance(p) < 9.0)
        {
            self.pen = Some(li);
            return None;
        }
        let o = &self.topts;
        let (fill, fc, sw, sc) = (o.pen_fill, o.pen_color, o.pen_w, o.pen_stroke);
        let i = self.add_layer("Tracé", Content::Path { pts: vec![Vtx::at(0.0, 0.0)], closed: false, color: fc, fill });
        let l = &mut self.comp.layers[i];
        l.props[0].base = cx.round();
        l.props[1].base = cy.round();
        if sw > 0.0 {
            l.fx.stroke = Some(sc);
            l.fx.stroke_w = sw;
            l.props[17].base = sw;
        }
        l.fx.cap = 1;
        self.pen = Some(i);
        Some((i, 0))
    }

    fn shape_geom(&self, a: (f32, f32), b: (f32, f32), shift: bool, alt: bool) -> ((f32, f32, f32, f32), Option<Vec<[f32; 2]>>) {
        let center = alt || self.topts.from_center;
        match self.tool {
            Tool::Polygon | Tool::Star => {
                let r = (b.0 - a.0).hypot(b.1 - a.1).max(2.0);
                let (n, inner) = if self.tool == Tool::Star { (self.topts.points, self.topts.inner) } else { (self.topts.sides, 1.0) };
                let pts = model::star_points(n, r, inner).into_iter().map(|q| [q[0] + a.0, q[1] + a.1]).collect();
                ((a.0 - r, a.1 - r, a.0 + r, a.1 + r), Some(pts))
            }
            _ => {
                let (mut dx, mut dy) = (b.0 - a.0, b.1 - a.1);
                if shift {
                    let m = dx.abs().max(dy.abs());
                    dx = m * dx.signum();
                    dy = m * dy.signum();
                }
                if center { ((a.0 - dx.abs(), a.1 - dy.abs(), a.0 + dx.abs(), a.1 + dy.abs()), None) } else { ((a.0.min(a.0 + dx), a.1.min(a.1 + dy), a.0.max(a.0 + dx), a.1.max(a.1 + dy)), None) }
            }
        }
    }

    fn make_mask(&mut self, (x0, y0, x1, y1): (f32, f32, f32, f32)) {
        let n = self.comp.layers.len();
        let Some(mut li) = self.sel.filter(|i| *i < n) else {
            return;
        };
        if li > 0 && self.comp.layers[li - 1].fx.matte > 0 {
            li -= 1;
        }
        let (w, h) = ((x1 - x0).max(2.0), (y1 - y0).max(2.0));
        let content = if self.tool == Tool::MaskRect { Content::Rect { w, h, r: 0.0, color: [255; 4] } } else { Content::Ellipse { w, h, color: [255; 4] } };
        let (start, end, name) = (self.comp.layers[li].start, self.comp.layers[li].end, format!("Masque · {}", self.comp.layers[li].name));
        let i = if self.comp.layers[li].fx.matte > 0 && li + 1 < n {
            self.comp.layers[li + 1].content = content;
            li + 1
        } else {
            self.sel = Some(li);
            let i = self.add_layer(&name, content);
            self.comp.layers[i].name = name;
            i
        };
        self.comp.layers[i].start = start;
        self.comp.layers[i].end = end;
        self.comp.layers[li].fx.matte = if self.topts.mask_invert { 3 } else { 1 };
        self.place(i, (x0 + x1) / 2.0, (y0 + y1) / 2.0);
        self.sel = Some(li);
    }

    fn make_shape(&mut self, a: (f32, f32), b: (f32, f32), shift: bool, alt: bool) {
        let ((x0, y0, x1, y1), _) = self.shape_geom(a, b, shift, alt);
        let (w, h) = ((x1 - x0).max(2.0), (y1 - y0).max(2.0));
        if matches!(self.tool, Tool::MaskRect | Tool::MaskEllipse) {
            self.make_mask((x0, y0, x1, y1));
            return;
        }
        let o = &self.topts;
        let (name, content) = match self.tool {
            Tool::Rect => ("Rectangle", Content::Rect { w, h, r: 0.0, color: o.fill }),
            Tool::RoundRect => ("Rectangle arrondi", Content::Rect { w, h, r: o.round.min(w.min(h) / 2.0), color: o.fill }),
            Tool::Ellipse => ("Ellipse", Content::Ellipse { w, h, color: o.fill }),
            Tool::Polygon => ("Polygone", Content::Star { points: o.sides, radius: w / 2.0, inner: 1.0, color: o.fill }),
            _ => ("Étoile", Content::Star { points: o.points, radius: w / 2.0, inner: o.inner, color: o.fill }),
        };
        let (stroke_on, sc, sw) = (o.stroke_on, o.stroke, o.stroke_w);
        let i = self.add_layer(name, content);
        let l = &mut self.comp.layers[i];
        if stroke_on {
            l.fx.stroke = Some(sc);
            l.fx.stroke_w = sw;
            l.props[17].base = sw;
        }
        self.place(i, (x0 + x1) / 2.0, (y0 + y1) / 2.0);
    }

    fn transport(&mut self, ui: &mut Ui, area: Rect) {
        let tb = area.shrink2(vec2(18.0, 6.0));
        ui.painter().line_segment([area.left_top(), area.right_top()], Stroke::new(1.0, LINE));
        let fps = self.comp.fps;
        ui.painter().text(tb.left_center(), Align2::LEFT_CENTER, tc(self.t, fps), FontId::monospace(16.0), BLUE);
        let info = match self.cursor {
            Some((x, y)) => format!("image {} · x {x:.0} y {y:.0}", (self.t * fps as f32).round() as u32),
            None => format!("image {}", (self.t * fps as f32).round() as u32),
        };
        let compact = tb.width() / 2.0 < 300.0;
        let span = if compact { 156.0 } else { 216.0 };
        let center_x = tb.center().x;
        if tb.left() + 104.0 + info.chars().count() as f32 * 6.5 < center_x - span - 8.0 {
            ui.painter().text(tb.left_center() + vec2(104.0, 0.0), Align2::LEFT_CENTER, info, FontId::proportional(11.5), FAINT);
        }
        let (a, b) = self.comp.range();
        let end = if self.comp.work.is_some() { format!("zone {} → {}", tc(a, fps), tc(b, fps)) } else { tc(self.comp.duration, fps) };
        if tb.right() - end.chars().count() as f32 * 8.0 > center_x + span + 8.0 {
            ui.painter().text(tb.right_center(), Align2::RIGHT_CENTER, end, FontId::monospace(13.0), MUTED);
        }
        let step = 1.0 / fps as f32;
        let full: [(f32, u8, &str); 8] = [(-150.0, 0u8, "Début (Origine)"), (-108.0, 5, "Image clé précédente (J)"), (-66.0, 1, "Image précédente (←)"), (0.0, 2, "Lecture / pause (Espace)"), (66.0, 3, "Image suivante (→)"), (108.0, 6, "Image clé suivante (K)"), (150.0, 4, "Fin"), (200.0, 7, "Lecture en boucle (L)")];
        let small: [(f32, u8, &str); 6] = [(-96.0, 0u8, "Début (Origine)"), (-52.0, 1, "Image précédente (←)"), (0.0, 2, "Lecture / pause (Espace)"), (52.0, 3, "Image suivante (→)"), (96.0, 4, "Fin"), (140.0, 7, "Lecture en boucle (L)")];
        let buttons: &[(f32, u8, &str)] = if compact { &small } else { &full };
        for &(dx, kind, tip) in buttons {
            let size = if kind == 2 { 40.0 } else { 32.0 };
            let r = Rect::from_center_size(tb.center() + vec2(dx, 0.0), Vec2::splat(size));
            let bt = ui.interact(r, ui.make_persistent_id(("transport", kind)), Sense::click()).on_hover_text(tip);
            let on = kind == 7 && self.loop_play;
            let fill = if kind == 2 {
                if bt.hovered() { BLUE.lerp_to_gamma(Color32::WHITE, 0.12) } else { BLUE }
            } else if on {
                BLUE.gamma_multiply(0.28)
            } else if bt.hovered() {
                FIELD
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().circle_filled(r.center(), size / 2.0, fill);
            match kind {
                5 | 6 => {
                    let dir = if kind == 6 { 1.0 } else { -1.0 };
                    diamond(ui.painter(), r.center() + vec2(3.0 * dir, 0.0), 5.0, KEY);
                    ui.painter().add(Shape::convex_polygon(vec![r.center() + vec2(-6.0 * dir, -4.5), r.center() + vec2(-2.0 * dir, 0.0), r.center() + vec2(-6.0 * dir, 4.5)], INK, Stroke::NONE));
                }
                7 => glyph(ui.painter(), r.shrink(8.0), G::Loop, if on { INK_STRONG } else { INK }),
                _ => transport_glyph(ui.painter(), r.center(), kind, self.playing, if kind == 2 { Color32::WHITE } else { INK }),
            }
            if bt.clicked() {
                match kind {
                    0 => self.t = a,
                    1 => self.t = self.snap_frame((self.t - step).max(0.0)),
                    2 => self.playing = !self.playing,
                    3 => self.t = self.snap_frame((self.t + step).min(self.comp.duration)),
                    4 => self.t = b,
                    5 => self.t = self.comp.next_key(self.t, false, None).unwrap_or(self.t),
                    6 => self.t = self.comp.next_key(self.t, true, None).unwrap_or(self.t),
                    _ => self.loop_play = !self.loop_play,
                }
            }
        }
    }
}
