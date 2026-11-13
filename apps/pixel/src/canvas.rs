use crate::doc::{Area, Brush, Paint};
use crate::tools::{self, CROP_RATIOS, Tool, brushy, is_marquee, is_shape};
use crate::{Drag, GRID_COLORS, PASTEBOARDS, Pixel, Sheet, TextDraft, shapes};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Frame, Key, Painter, Pos2, Rect, Sense, Shape, Stroke, TextureId, Vec2, pos2, vec2};
use image::imageops;
use studio_ui::pro::{CHROME, FAINT, LINE, MUTED, PANEL, semibold};

#[derive(Clone, Copy)]
pub struct View {
    c: Pos2,
    zoom: f32,
    sn: f32,
    cs: f32,
    half: (f32, f32),
}

impl View {
    pub fn to_scr(&self, d: (f32, f32)) -> Pos2 {
        let (x, y) = ((d.0 - self.half.0) * self.zoom, (d.1 - self.half.1) * self.zoom);
        pos2(self.c.x + x * self.cs - y * self.sn, self.c.y + x * self.sn + y * self.cs)
    }
    pub fn to_doc(&self, p: Pos2) -> (f32, f32) {
        let (x, y) = (p.x - self.c.x, p.y - self.c.y);
        let (u, v) = (x * self.cs + y * self.sn, -x * self.sn + y * self.cs);
        (u / self.zoom + self.half.0, v / self.zoom + self.half.1)
    }
    fn quad(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> [Pos2; 4] {
        [self.to_scr((x0, y0)), self.to_scr((x1, y0)), self.to_scr((x1, y1)), self.to_scr((x0, y1))]
    }
}

fn image_quad(p: &Painter, tex: TextureId, q: [Pos2; 4], uv: Rect) {
    let mut mesh = egui::Mesh::with_texture(tex);
    let uvs = [uv.left_top(), uv.right_top(), uv.right_bottom(), uv.left_bottom()];
    for (pos, u) in q.iter().zip(uvs) {
        mesh.vertices.push(egui::epaint::Vertex { pos: *pos, uv: u, color: Color32::WHITE });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(mesh);
}

fn ants(p: &Painter, pts: &[Pos2], t: f32) {
    p.add(Shape::closed_line(pts.to_vec(), Stroke::new(1.0, Color32::BLACK)));
    let mut closed = pts.to_vec();
    closed.push(pts[0]);
    p.extend(Shape::dashed_line_with_offset(&closed, Stroke::new(1.0, Color32::WHITE), &[5.0], &[5.0], t * 20.0 % 10.0));
}

fn constrain(start: (f32, f32), end: (f32, f32), ratio: f32, center: bool) -> ((f32, f32), (f32, f32)) {
    let (mut dx, mut dy) = (end.0 - start.0, end.1 - start.1);
    if ratio > 0.0 {
        if dx.abs() / ratio > dy.abs() {
            dy = dy.signum() * dx.abs() / ratio;
        } else {
            dx = dx.signum() * dy.abs() * ratio;
        }
        if dy == 0.0 {
            dy = dx.abs() / ratio;
        }
    }
    if center { ((start.0 - dx, start.1 - dy), (start.0 + dx, start.1 + dy)) } else { (start, (start.0 + dx, start.1 + dy)) }
}

impl Pixel {
    fn snap_pt(&self, d: (f32, f32), zoom: f32) -> (f32, f32) {
        if !self.snap {
            return d;
        }
        let th = 8.0 / zoom;
        let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);
        let mut xs = vec![0.0, w, w / 2.0];
        let mut ys = vec![0.0, h, h / 2.0];
        if self.show_guides {
            for (v, p) in &self.guides {
                if *v { xs.push(*p) } else { ys.push(*p) }
            }
        }
        if self.show_grid {
            let g = self.grid.max(2) as f32;
            xs.push((d.0 / g).round() * g);
            ys.push((d.1 / g).round() * g);
        }
        let pick = |v: f32, c: &[f32]| c.iter().copied().filter(|x| (x - v).abs() <= th).min_by(|a, b| (a - v).abs().total_cmp(&(b - v).abs())).unwrap_or(v);
        (pick(d.0, &xs), pick(d.1, &ys))
    }

    fn mag_snap(&self, d: (f32, f32)) -> (f32, f32) {
        let img = &self.composite;
        let (w, h) = (img.width() as i32, img.height() as i32);
        let l = |x: i32, y: i32| {
            let p = img.get_pixel(x.clamp(0, w - 1) as u32, y.clamp(0, h - 1) as u32).0;
            0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32
        };
        let r = self.mag_width as i32;
        let (cx, cy) = (d.0 as i32, d.1 as i32);
        let mut best = (0.0, d);
        for y in cy - r..=cy + r {
            for x in cx - r..=cx + r {
                if (x - cx).pow(2) + (y - cy).pow(2) > r * r {
                    continue;
                }
                let g = (l(x + 1, y) - l(x - 1, y)).abs() + (l(x, y + 1) - l(x, y - 1)).abs();
                let score = g - ((x - cx).pow(2) + (y - cy).pow(2)) as f32 * 0.05;
                if score > best.0 {
                    best = (score, (x as f32 + 0.5, y as f32 + 0.5));
                }
            }
        }
        if best.0 > 12.0 { best.1 } else { d }
    }

    fn make_paint(&mut self, at: (f32, f32)) -> Option<Paint> {
        let layer_img = || self.doc.active_target();
        Some(match self.tool {
            Tool::Brush => Paint::Color(self.primary.to_array()),
            Tool::Pencil => Paint::Pencil(self.primary.to_array()),
            Tool::ColorReplace => Paint::Replace(self.primary.to_array()),
            Tool::Mixer => Paint::Mixer { color: self.primary.to_array(), carry: None, wet: self.mixer_wet },
            Tool::HistoryBrush | Tool::Eraser if self.tool == Tool::HistoryBrush || self.erase_history => {
                let src = self.history_src.clone().filter(|s| s.dimensions() == (self.doc.width(), self.doc.height()));
                match src {
                    Some(src) => Paint::Clone { src, dx: 0.0, dy: 0.0 },
                    None => {
                        self.error = Some("Aucun instantané compatible : clique sur Instantané".into());
                        return None;
                    }
                }
            }
            Tool::Eraser => Paint::Erase,
            Tool::BgEraser => Paint::BgErase { tolerance: self.bg_tol, target: None, continuous: self.bg_continuous },
            Tool::Dodge => Paint::Dodge(self.dodge_range),
            Tool::Burn => Paint::Burn(self.dodge_range),
            Tool::Sponge => Paint::Sponge(if self.sponge_saturate { 1.0 } else { -1.0 }),
            Tool::Blur => Paint::Blur(layer_img()),
            Tool::Sharpen => Paint::Sharpen(layer_img()),
            Tool::Smudge => Paint::Smudge(self.finger_paint.then(|| self.primary.to_array().map(|v| v as f32 / 255.0))),
            Tool::Clone | Tool::Heal => {
                let src = self.clone_src?;
                if !self.clone_aligned {
                    self.clone_offset = None;
                }
                let off = *self.clone_offset.get_or_insert((src.0 - at.0, src.1 - at.1));
                let img = if self.sample_all { self.composite.clone() } else { layer_img() };
                if self.tool == Tool::Clone { Paint::Clone { src: img, dx: off.0, dy: off.1 } } else { Paint::Heal { src: img, dx: off.0, dy: off.1 } }
            }
            _ => return None,
        })
    }

    fn mirrors(&self, p: (f32, f32)) -> Vec<(f32, f32)> {
        let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);
        let mut v = vec![p];
        let sym = self.symmetry && matches!(self.tool, Tool::Brush | Tool::Pencil | Tool::Eraser);
        let symv = self.symmetry_v && matches!(self.tool, Tool::Brush | Tool::Pencil);
        if sym {
            v.push((w - p.0, p.1));
        }
        if symv {
            v.push((p.0, h - p.1));
        }
        if sym && symv {
            v.push((w - p.0, h - p.1));
        }
        v
    }

    fn apply_patch(&mut self, offset: (f32, f32)) {
        let Some(s) = self.doc.selection else { return };
        let base = self.doc.layers[self.doc.active].img.clone();
        let blurred = imageops::fast_blur(&base, self.patch_diffusion);
        let (w, h) = base.dimensions();
        let mut out = base.clone();
        for y in s.y0..s.y1 {
            for x in s.x0..s.x1 {
                if self.doc.sel_weight(x, y) <= 0.0 {
                    continue;
                }
                let (sx, sy) = ((x as f32 + offset.0).round() as i64, (y as f32 + offset.1).round() as i64);
                if sx < 0 || sy < 0 || sx >= w as i64 || sy >= h as i64 {
                    continue;
                }
                let q = base.get_pixel(sx as u32, sy as u32).0;
                let md = blurred.get_pixel(x, y).0;
                let ms = blurred.get_pixel(sx as u32, sy as u32).0;
                let px: [u8; 4] = std::array::from_fn(|k| if k == 3 { q[3] } else { (q[k] as f32 + md[k] as f32 - ms[k] as f32).clamp(0.0, 255.0) as u8 });
                out.put_pixel(x, y, image::Rgba(px));
            }
        }
        self.doc.next_label = Some("Pièce".into());
        self.doc.checkpoint();
        self.doc.put_filtered(&base, &out);
    }

    fn draw_shape(&mut self, start: (f32, f32), end: (f32, f32), shift: bool, alt: bool) {
        let t = self.tool;
        let (a, b) = if t == Tool::Line {
            let mut e = end;
            if shift {
                let (dx, dy) = (end.0 - start.0, end.1 - start.1);
                let ang = (dy.atan2(dx) / (std::f32::consts::PI / 4.0)).round() * std::f32::consts::PI / 4.0;
                let len = (dx * dx + dy * dy).sqrt();
                e = (start.0 + ang.cos() * len, start.1 + ang.sin() * len);
            }
            (start, e)
        } else {
            constrain(start, end, if shift { 1.0 } else { 0.0 }, alt)
        };
        if (b.0 - a.0).abs() < 1.0 && (b.1 - a.1).abs() < 1.0 {
            return;
        }
        let (x0, y0, x1, y1) = (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1));
        if self.shape_layer {
            let n = self.doc.layers.len() + 1;
            self.doc.next_label = Some(tools::tool_name(t).to_string());
            self.doc.add_layer(Some(format!("{} {n}", tools::tool_name(t))), None);
        } else {
            self.doc.next_label = Some(tools::tool_name(t).to_string());
            self.doc.checkpoint();
        }
        let (fill, stroke) = (self.primary.to_array(), if self.shape_fill { self.secondary.to_array() } else { self.primary.to_array() });
        if t == Tool::Line {
            for poly in shapes::line(a, b, self.stroke_w, self.arrow_start, self.arrow_end) {
                self.doc.fill_polygon(&poly, fill, self.shape_mode);
            }
            return;
        }
        let pts = match t {
            Tool::Rect => shapes::rect(x0, y0, x1, y1, self.corner),
            Tool::Ellipse => shapes::ellipse(x0, y0, x1, y1),
            Tool::Polygon => shapes::polygon(x0, y0, x1, y1, self.sides, self.star.then_some(self.inset)),
            _ => shapes::custom(self.custom, x0, y0, x1, y1),
        };
        if self.shape_fill || !self.shape_stroke {
            self.doc.fill_polygon(&pts, fill, self.shape_mode);
        }
        if self.shape_stroke {
            let br = Brush { radius: self.stroke_w / 2.0, hardness: 0.95, ..Brush::new(1.0, 1.0, 1.0) };
            self.doc.stamp(pts[0].0, pts[0].1, &br, stroke, false);
            for k in 0..pts.len() {
                self.doc.stroke(pts[k], pts[(k + 1) % pts.len()], &br, stroke, false);
            }
        }
    }

    pub fn canvas(&mut self, root: &mut egui::Ui) {
        let bg = PASTEBOARDS[self.pasteboard.min(3)].1;
        egui::CentralPanel::default().frame(Frame::new().fill(bg)).show(root, |ui| {
            let full = ui.max_rect();
            let tab_h = 32.0;
            let tab_bar = Rect::from_min_max(full.min, pos2(full.right(), full.top() + tab_h));
            {
                let p = ui.painter();
                p.rect_filled(tab_bar, CornerRadius::ZERO, PANEL);
                let label = format!("{} @ {:.0} %{}{}", self.doc.name, self.zoom * 100.0, if self.doc.edit_mask && self.doc.layers[self.doc.active].mask.is_some() { " (masque)" } else { "" }, if self.doc.modified { " •" } else { "" });
                let g = p.layout_no_wrap(label, FontId::proportional(12.5), studio_ui::pro::INK);
                let tab = Rect::from_min_size(tab_bar.min + vec2(8.0, 4.0), vec2(g.size().x + 28.0, tab_h - 4.0));
                p.rect_filled(tab, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, bg);
                p.galley(tab.left_center() + vec2(14.0, -g.size().y / 2.0), g, studio_ui::pro::INK);
            }
            let rulers = self.rulers && self.view_angle == 0.0;
            let rw = if rulers { 18.0 } else { 0.0 };
            let area = Rect::from_min_max(pos2(full.left() + rw, tab_bar.bottom() + rw), full.max);
            let top_ruler = Rect::from_min_max(pos2(area.left(), tab_bar.bottom()), pos2(area.right(), area.top()));
            let left_ruler = Rect::from_min_max(pos2(full.left(), area.top()), pos2(area.left(), area.bottom()));
            let resp = ui.allocate_rect(area, Sense::click_and_drag());
            let (w, h) = (self.doc.width() as f32, self.doc.height() as f32);
            if self.fit_pending {
                self.zoom = ((area.width() - 80.0) / w).min((area.height() - 80.0) / h).clamp(0.02, 1.0);
                self.offset = Vec2::ZERO;
                self.fit_pending = false;
            }
            let (scroll, pinch, space, pointer, primary_down, middle_down, delta, alt, shift, pressed, released, time) = ui.input(|i| {
                (i.smooth_scroll_delta.y, i.zoom_delta(), i.key_down(Key::Space), i.pointer.hover_pos(), i.pointer.primary_down(), i.pointer.middle_down(), i.pointer.delta(), i.modifiers.alt, i.modifiers.shift, i.pointer.primary_pressed(), i.pointer.primary_released(), i.time as f32)
            });
            let (sn, cs) = self.view_angle.to_radians().sin_cos();
            let view_of = |zoom: f32, offset: Vec2| View { c: area.center() + offset, zoom, sn, cs, half: (w / 2.0, h / 2.0) };
            if resp.hovered() {
                let factor = pinch * (scroll / 400.0).exp();
                if (factor - 1.0).abs() > 1e-4
                    && let Some(at) = pointer
                {
                    let d = view_of(self.zoom, self.offset).to_doc(at);
                    self.zoom = (self.zoom * factor).clamp(0.02, 64.0);
                    let moved = view_of(self.zoom, self.offset).to_scr(d);
                    self.offset += at - moved;
                }
            }
            let panning = (resp.dragged() && (self.tool == Tool::Hand || space)) || (resp.hovered() && middle_down);
            if panning {
                self.offset += delta;
            }
            let v = view_of(self.zoom, self.offset);
            let zoom = self.zoom;
            self.cursor = pointer.filter(|p| area.contains(*p)).map(|p| v.to_doc(p));

            if rulers {
                for (band, vertical) in [(top_ruler, false), (left_ruler, true)] {
                    let r = ui.interact(band, ui.make_persistent_id(("ruler", vertical)), Sense::drag());
                    if r.drag_started() {
                        self.drag = Drag::Guide { idx: None, vertical };
                        self.show_guides = true;
                    }
                }
            }

            if self.tool == Tool::Remove && self.filling.is_none() && !space {
                if resp.is_pointer_button_down_on() && primary_down {
                    if let Some(at) = pointer {
                        let d = v.to_doc(at);
                        if self.remove_stroke.last().is_none_or(|l| (l.0 - d.0).abs() + (l.1 - d.1).abs() > 0.5) {
                            self.remove_stroke.push(d);
                        }
                    }
                } else if !self.remove_stroke.is_empty() {
                    let mask = self.stroke_mask();
                    let ctx = ui.ctx().clone();
                    self.start_fill(mask, &ctx);
                    if self.filling.is_none() {
                        self.remove_stroke.clear();
                    }
                }
            }
            let painting = brushy(self.tool) || matches!(self.tool, Tool::Remove | Tool::QuickSelect);
            let source_pick = matches!(self.tool, Tool::Clone | Tool::Heal) && alt;
            if source_pick && pressed && resp.hovered()
                && let Some(at) = pointer
            {
                self.clone_src = Some(v.to_doc(at));
                self.clone_offset = None;
            } else if (brushy(self.tool) || self.tool == Tool::QuickSelect) && !space && !source_pick && resp.is_pointer_button_down_on() && primary_down {
                if let Some(at) = pointer {
                    let raw = v.to_doc(at);
                    let cur = match self.smooth_pos {
                        Some(s) if self.smoothing > 0.0 => (s.0 + (raw.0 - s.0) * (1.0 - self.smoothing), s.1 + (raw.1 - s.1) * (1.0 - self.smoothing)),
                        _ => raw,
                    };
                    self.smooth_pos = Some(cur);
                    if self.tool == Tool::QuickSelect {
                        if self.last.is_none_or(|l| (l.0 - cur.0).abs() + (l.1 - cur.1).abs() > self.brush.radius * 0.4) {
                            self.doc.quick_select(cur.0, cur.1, self.brush.radius, self.tolerance, alt || self.sel_mode == 2);
                            self.last = Some(cur);
                        }
                    } else {
                        match self.last {
                            None => {
                                let l = &self.doc.layers[self.doc.active];
                                if l.locked {
                                    self.error = Some("Ce calque est verrouillé".into());
                                } else if l.adjust.is_some() && !self.doc.edit_mask {
                                    self.error = Some("Calque de réglage : peins dans son masque".into());
                                } else {
                                    self.paint = self.make_paint(cur);
                                }
                                if self.paint.is_some() {
                                    if matches!(self.tool, Tool::Brush | Tool::Pencil | Tool::Mixer | Tool::ColorReplace) {
                                        self.use_color(self.primary);
                                    }
                                    self.doc.next_label = Some(tools::tool_name(self.tool).to_string());
                                    self.doc.checkpoint();
                                }
                                let pts = self.mirrors(cur);
                                if let Some(paint) = &mut self.paint {
                                    for p in pts {
                                        self.doc.stamp_paint(p.0, p.1, &self.brush, paint);
                                    }
                                }
                            }
                            Some(prev) => {
                                let (a, b) = (self.mirrors(prev), self.mirrors(cur));
                                if let Some(paint) = &mut self.paint {
                                    for (p, q) in a.into_iter().zip(b) {
                                        self.doc.stroke_paint(p, q, &self.brush, paint);
                                    }
                                }
                            }
                        }
                        self.last = Some(cur);
                    }
                }
            } else {
                self.last = None;
                self.paint = None;
                self.smooth_pos = None;
            }

            if !space && resp.hovered() && pressed
                && let Some(at) = pointer
            {
                let raw = v.to_doc(at);
                let d = self.snap_pt(raw, zoom);
                let t = self.tool;
                if is_marquee(t) || matches!(t, Tool::Lasso | Tool::MagLasso | Tool::Patch | Tool::PolyLasso) {
                    self.sel_prev = (self.sel_mode != 0 && self.doc.selection.is_some()).then(|| self.doc.sel_mask());
                }
                match t {
                    Tool::RowMarquee | Tool::ColMarquee => {
                        let a = if t == Tool::RowMarquee { Area::from_points((0.0, d.1.floor()), (w, d.1.floor() + 1.0), w as u32, h as u32) } else { Area::from_points((d.0.floor(), 0.0), (d.0.floor() + 1.0, h), w as u32, h as u32) };
                        self.doc.set_selection(a, None);
                        let prev = self.sel_prev.take();
                        self.doc.combine_selection(prev, self.sel_mode);
                    }
                    Tool::Marquee | Tool::EllipseMarquee if self.marquee_style == 2 => {
                        let (fw, fh) = (self.marquee_size.0 as f32, self.marquee_size.1 as f32);
                        if let Some(a) = Area::from_points((d.0 - fw / 2.0, d.1 - fh / 2.0), (d.0 + fw / 2.0, d.1 + fh / 2.0), w as u32, h as u32) {
                            if t == Tool::EllipseMarquee {
                                self.doc.select_ellipse(a);
                            } else {
                                self.doc.set_selection(Some(a), None);
                            }
                            self.doc.feather_selection(self.feather);
                            let prev = self.sel_prev.take();
                            self.doc.combine_selection(prev, self.sel_mode);
                        }
                    }
                    Tool::Marquee | Tool::EllipseMarquee => self.drag = Drag::Marquee(d),
                    Tool::Lasso | Tool::MagLasso => self.drag = Drag::Lasso(vec![if t == Tool::MagLasso { self.mag_snap(raw) } else { raw }]),
                    Tool::Patch => {
                        if self.doc.selection.is_some() && raw.0 >= 0.0 && raw.1 >= 0.0 && self.doc.selected(raw.0 as u32, raw.1 as u32) {
                            self.drag = Drag::Patch { from: raw };
                            self.gradient_end = Some(raw);
                        } else {
                            self.drag = Drag::Lasso(vec![raw]);
                        }
                    }
                    Tool::Crop => self.drag = Drag::Crop(d),
                    t if is_shape(t) => {
                        self.gradient_end = None;
                        self.drag = Drag::Shape(d);
                    }
                    Tool::Move => {
                        let near = (!self.lock_guides && self.show_guides).then(|| self.guides.iter().position(|(vert, p)| if *vert { (raw.0 - p).abs() * zoom < 5.0 } else { (raw.1 - p).abs() * zoom < 5.0 })).flatten();
                        if let Some(i) = near {
                            self.drag = Drag::Guide { idx: Some(i), vertical: self.guides[i].0 };
                        } else {
                            if self.auto_select
                                && let Some(i) = self.doc.layer_at(raw.0.max(0.0) as u32, raw.1.max(0.0) as u32)
                            {
                                self.doc.active = i;
                                self.doc.edit_mask = false;
                            }
                            let l = &self.doc.layers[self.doc.active];
                            if l.locked || l.lock_pos {
                                self.error = Some("La position de ce calque est verrouillée".into());
                            } else {
                                self.doc.next_label = Some("Déplacement".into());
                                self.doc.checkpoint();
                                self.drag = Drag::Move { from: raw, original: self.doc.layers[self.doc.active].img.clone() };
                            }
                        }
                    }
                    Tool::Gradient => self.drag = Drag::Gradient(d),
                    Tool::Ruler => self.drag = Drag::Ruler(d),
                    Tool::RotateView => {
                        let a = (at - area.center() - self.offset).angle();
                        self.drag = Drag::Rotate { start: self.view_angle, from: a };
                    }
                    _ => {}
                }
            }
            if let Some(at) = pointer {
                let raw = v.to_doc(at);
                let d = self.snap_pt(raw, zoom);
                let ratio = match (self.tool, self.marquee_style, self.crop_ratio) {
                    (Tool::Crop, _, r) => match CROP_RATIOS[r].1 {
                        x if x < 0.0 => w / h,
                        x => x,
                    },
                    (_, 1, _) => self.marquee_ratio.0 / self.marquee_ratio.1.max(0.01),
                    _ => 0.0,
                };
                let ratio = if shift && ratio == 0.0 { 1.0 } else { ratio };
                let mag = if self.tool == Tool::MagLasso && matches!(self.drag, Drag::Lasso(_)) { self.mag_snap(raw) } else { raw };
                match &mut self.drag {
                    Drag::Marquee(start) => {
                        let (a, b) = constrain(*start, d, ratio, alt);
                        let ar = Area::from_points(a, b, w as u32, h as u32);
                        self.doc.set_selection(ar, None);
                    }
                    Drag::Lasso(pts) => {
                        let p = mag;
                        if pts.last().is_none_or(|l| (l.0 - p.0).abs() + (l.1 - p.1).abs() > 1.0) {
                            pts.push(p);
                        }
                    }
                    Drag::Crop(start) => {
                        let (a, b) = constrain(*start, d, ratio, alt);
                        self.crop = Area::from_points(a, b, w as u32, h as u32);
                    }
                    Drag::Shape(_) | Drag::Gradient(_) | Drag::Patch { .. } => self.gradient_end = Some(if matches!(self.drag, Drag::Patch { .. }) { raw } else { d }),
                    Drag::Ruler(start) => self.ruler = Some((*start, d)),
                    Drag::Move { from, original } => {
                        let (dx, dy) = ((d.0 - from.0).round() as i32, (d.1 - from.1).round() as i32);
                        let original = original.clone();
                        self.doc.place_layer(&original, dx, dy);
                    }
                    Drag::Guide { idx, vertical } => {
                        let pos = if *vertical { d.0.round() } else { d.1.round() };
                        match idx {
                            Some(i) => self.guides[*i].1 = pos,
                            None => self.gradient_end = Some(d),
                        }
                    }
                    Drag::Rotate { start, from } => {
                        let a = (at - area.center() - self.offset).angle();
                        let mut ang = *start + (a - *from).to_degrees();
                        ang = (ang + 180.0).rem_euclid(360.0) - 180.0;
                        if shift {
                            ang = (ang / 15.0).round() * 15.0;
                        }
                        self.view_angle = ang;
                    }
                    Drag::None => {}
                }
            }
            if released {
                let pointer_doc = pointer.map(|p| v.to_doc(p));
                match std::mem::replace(&mut self.drag, Drag::None) {
                    Drag::Gradient(start) => {
                        if let Some(mut end) = self.gradient_end.take() {
                            if shift {
                                let (dx, dy) = (end.0 - start.0, end.1 - start.1);
                                let ang = (dy.atan2(dx) / (std::f32::consts::PI / 4.0)).round() * std::f32::consts::PI / 4.0;
                                let len = (dx * dx + dy * dy).sqrt();
                                end = (start.0 + ang.cos() * len, start.1 + ang.sin() * len);
                            }
                            let stops = tools::gradient_stops(self.grad_preset, self.primary.to_array(), self.secondary.to_array());
                            self.doc.next_label = Some("Dégradé".into());
                            self.doc.gradient_ex(start, end, &stops, self.grad_kind as u8, self.grad_reverse, self.grad_opacity, self.grad_dither, self.grad_mode);
                        }
                    }
                    Drag::Marquee(_) => {
                        let prev = self.sel_prev.take();
                        if self.doc.selection.is_some_and(|s| s.width() < 2 && s.height() < 2) {
                            self.doc.set_selection(None, None);
                            if let Some(p) = prev.filter(|_| self.sel_mode != 0) {
                                self.doc.select_mask(p);
                            }
                        } else {
                            if self.tool == Tool::EllipseMarquee
                                && let Some(a) = self.doc.selection
                            {
                                self.doc.select_ellipse(a);
                            }
                            self.doc.feather_selection(self.feather);
                            self.doc.combine_selection(prev, self.sel_mode);
                        }
                    }
                    Drag::Lasso(pts) => {
                        if pts.len() >= 3 {
                            self.finish_lasso(&pts);
                        } else {
                            self.sel_prev = None;
                        }
                    }
                    Drag::Patch { from } => {
                        if let Some(end) = self.gradient_end.take() {
                            self.apply_patch((end.0 - from.0, end.1 - from.1));
                        }
                    }
                    Drag::Crop(_) => {
                        if self.crop.is_some_and(|c| c.width() < 4 || c.height() < 4) {
                            self.crop = None;
                        }
                    }
                    Drag::Shape(start) => {
                        if let Some(end) = self.gradient_end.take() {
                            self.draw_shape(start, end, shift, alt);
                        }
                    }
                    Drag::Guide { idx, vertical } => {
                        let in_doc = pointer.is_some_and(|p| area.contains(p)) && pointer_doc.is_some_and(|d| if vertical { d.0 >= 0.0 && d.0 <= w } else { d.1 >= 0.0 && d.1 <= h });
                        match idx {
                            Some(i) if !in_doc => {
                                self.guides.remove(i);
                            }
                            None if in_doc => {
                                if let Some(d) = self.gradient_end.take() {
                                    self.guides.push((vertical, if vertical { d.0.round() } else { d.1.round() }));
                                }
                            }
                            _ => {}
                        }
                        self.gradient_end = None;
                    }
                    _ => {}
                }
            }

            if resp.clicked() && !space
                && let Some(at) = pointer
            {
                let (x, y) = v.to_doc(at);
                let inside = x >= 0.0 && y >= 0.0 && x < w && y < h;
                match self.tool {
                    Tool::Fill if inside => {
                        let sample = self.sample_all.then(|| self.composite.clone());
                        self.doc.next_label = Some("Pot de peinture".into());
                        self.doc.checkpoint();
                        self.doc.flood_fill_ex(x as u32, y as u32, self.primary.to_array(), self.tolerance, self.contiguous, sample.as_ref(), self.fill_opacity);
                        self.use_color(self.primary);
                    }
                    Tool::MagicEraser if inside => {
                        self.doc.next_label = Some("Gomme magique".into());
                        self.doc.checkpoint();
                        self.doc.magic_erase(x as u32, y as u32, self.tolerance, self.contiguous, self.fill_opacity);
                    }
                    Tool::Wand if inside => {
                        let prev = (self.sel_mode != 0 && self.doc.selection.is_some()).then(|| self.doc.sel_mask());
                        let sample = self.sample_all.then(|| self.composite.clone());
                        self.doc.select_color_in(x as u32, y as u32, self.tolerance, self.contiguous, sample.as_ref());
                        self.doc.combine_selection(prev, self.sel_mode);
                    }
                    Tool::TextH | Tool::TextV if inside => self.text = Some(TextDraft { at: (x, y), text: String::new(), vertical: self.tool == Tool::TextV }),
                    Tool::Picker if inside => {
                        if let Some(c) = self.sample_color(x, y) {
                            if alt { self.secondary = c } else { self.primary = c }
                        }
                    }
                    Tool::Sampler if inside => {
                        if self.samplers.len() >= 4 {
                            self.samplers.remove(0);
                        }
                        self.samplers.push((x, y));
                    }
                    Tool::Note => {
                        let hit = self.notes.iter().position(|(p, _)| (v.to_scr(*p) - at).length() < 12.0);
                        self.sheet = Some(match hit {
                            Some(i) => Sheet::Note { idx: Some(i), at: self.notes[i].0, text: self.notes[i].1.clone() },
                            None => Sheet::Note { idx: None, at: (x, y), text: String::new() },
                        });
                    }
                    Tool::PolyLasso => {
                        let d = (x.clamp(0.0, w), y.clamp(0.0, h));
                        let close = self.poly.first().is_some_and(|f| (v.to_scr(*f) - at).length() < 8.0);
                        if self.poly.is_empty() {
                            self.sel_prev = (self.sel_mode != 0 && self.doc.selection.is_some()).then(|| self.doc.sel_mask());
                        }
                        if (close || resp.double_clicked()) && self.poly.len() >= 3 {
                            let pts = std::mem::take(&mut self.poly);
                            self.finish_lasso(&pts);
                        } else if !resp.double_clicked() {
                            self.poly.push(d);
                        }
                    }
                    Tool::Zoom => {
                        let f = if alt != self.zoom_out { 0.8 } else { 1.25 };
                        let d = v.to_doc(at);
                        self.zoom = (self.zoom * f).clamp(0.02, 64.0);
                        let moved = view_of(self.zoom, self.offset).to_scr(d);
                        self.offset += at - moved;
                    }
                    _ => {}
                }
            }

            let v = view_of(self.zoom, self.offset);
            let zoom = self.zoom;
            let painter = ui.painter_at(area);
            let doc_q = v.quad(0.0, 0.0, w, h);
            if self.view_angle == 0.0 {
                let rect = Rect::from_two_pos(doc_q[0], doc_q[2]);
                painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(120) }.as_shape(rect, CornerRadius::ZERO));
            }
            image_quad(&painter, self.checker.id(), doc_q, Rect::from_min_max(pos2(0.0, 0.0), pos2(w * zoom / 16.0, h * zoom / 16.0)));
            if let Some(t) = &self.tex {
                image_quad(&painter, t.id(), doc_q, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)));
            }
            let corners = [area.left_top(), area.right_top(), area.right_bottom(), area.left_bottom()].map(|p| v.to_doc(p));
            let (vx0, vx1) = (corners.iter().map(|c| c.0).fold(f32::MAX, f32::min), corners.iter().map(|c| c.0).fold(f32::MIN, f32::max));
            let (vy0, vy1) = (corners.iter().map(|c| c.1).fold(f32::MAX, f32::min), corners.iter().map(|c| c.1).fold(f32::MIN, f32::max));
            self.view_doc = Some(Rect::from_min_max(pos2(vx0, vy0), pos2(vx1, vy1)));
            if self.show_grid {
                let g = self.grid.max(2) as f32;
                let gc = GRID_COLORS[self.grid_color.min(4)].1;
                let sub = self.grid_sub.max(1) as f32;
                let step = g / sub;
                if step * zoom >= 5.0 {
                    let mut k = 0;
                    let mut x = 0.0;
                    while x <= w {
                        let major = k % self.grid_sub.max(1) == 0;
                        painter.line_segment([v.to_scr((x, 0.0)), v.to_scr((x, h))], Stroke::new(1.0, Color32::from_rgba_unmultiplied(gc[0], gc[1], gc[2], if major { 80 } else { 35 })));
                        x += step;
                        k += 1;
                    }
                    let (mut y, mut k) = (0.0, 0);
                    while y <= h {
                        let major = k % self.grid_sub.max(1) == 0;
                        painter.line_segment([v.to_scr((0.0, y)), v.to_scr((w, y))], Stroke::new(1.0, Color32::from_rgba_unmultiplied(gc[0], gc[1], gc[2], if major { 80 } else { 35 })));
                        y += step;
                        k += 1;
                    }
                }
            }
            if self.pixel_grid && zoom >= 10.0 {
                let ps = Stroke::new(1.0, Color32::from_black_alpha(40));
                let (x0, x1) = (vx0.floor().max(0.0), vx1.ceil().min(w));
                let (y0, y1) = (vy0.floor().max(0.0), vy1.ceil().min(h));
                let mut x = x0;
                while x <= x1 {
                    painter.line_segment([v.to_scr((x, y0)), v.to_scr((x, y1))], ps);
                    x += 1.0;
                }
                let mut y = y0;
                while y <= y1 {
                    painter.line_segment([v.to_scr((x0, y)), v.to_scr((x1, y))], ps);
                    y += 1.0;
                }
            }
            if self.extras {
                if let Some((_, t)) = &self.mask_tex {
                    image_quad(&painter, t.id(), doc_q, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)));
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                } else if let Some(s) = self.doc.selection {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
                    if matches!(self.drag, Drag::Marquee(_)) && self.tool == Tool::EllipseMarquee {
                        let pts: Vec<Pos2> = shapes::ellipse(s.x0 as f32, s.y0 as f32, s.x1 as f32, s.y1 as f32).into_iter().map(|p| v.to_scr(p)).collect();
                        ants(&painter, &pts, time);
                    } else {
                        ants(&painter, &v.quad(s.x0 as f32, s.y0 as f32, s.x1 as f32, s.y1 as f32), time);
                    }
                }
                if self.show_guides {
                    let gs = Stroke::new(1.0, Color32::from_rgb(0, 220, 255));
                    for (vert, p) in &self.guides {
                        let (a, b) = if *vert { ((*p, -100000.0), (*p, 100000.0)) } else { ((-100000.0, *p), (100000.0, *p)) };
                        painter.line_segment([v.to_scr(a), v.to_scr(b)], gs);
                    }
                }
                for (i, (p, text)) in self.notes.iter().enumerate() {
                    let c = v.to_scr(*p);
                    let r = Rect::from_center_size(c, Vec2::splat(16.0));
                    painter.rect(r, CornerRadius::same(3), Color32::from_rgb(255, 214, 10), Stroke::new(1.0, Color32::from_black_alpha(160)), egui::StrokeKind::Outside);
                    painter.text(r.center(), Align2::CENTER_CENTER, format!("{}", i + 1), FontId::proportional(10.0), Color32::BLACK);
                    if pointer.is_some_and(|q| r.expand(4.0).contains(q)) {
                        let g = painter.layout(text.clone(), FontId::proportional(12.5), Color32::BLACK, 240.0);
                        let tip = Rect::from_min_size(r.right_top() + vec2(6.0, 0.0), g.size() + vec2(16.0, 12.0));
                        painter.rect_filled(tip, CornerRadius::same(6), Color32::from_rgb(255, 240, 160));
                        painter.galley(tip.min + vec2(8.0, 6.0), g, Color32::BLACK);
                    }
                }
                for (i, p) in self.samplers.iter().enumerate() {
                    let c = v.to_scr(*p);
                    painter.circle_stroke(c, 6.0, Stroke::new(1.5, Color32::WHITE));
                    painter.circle_stroke(c, 7.5, Stroke::new(1.0, Color32::BLACK));
                    painter.text(c + vec2(9.0, -9.0), Align2::LEFT_BOTTOM, format!("{}", i + 1), semibold(11.0), Color32::WHITE);
                }
                if self.show_bounds && self.tool == Tool::Move
                    && let Some(b) = self.doc.layer_bounds(self.doc.active)
                {
                    painter.add(Shape::closed_line(v.quad(b.x0 as f32, b.y0 as f32, b.x1 as f32, b.y1 as f32).to_vec(), Stroke::new(1.0, studio_ui::pro::BLUE)));
                    for q in v.quad(b.x0 as f32, b.y0 as f32, b.x1 as f32, b.y1 as f32) {
                        painter.rect(Rect::from_center_size(q, Vec2::splat(7.0)), CornerRadius::ZERO, Color32::WHITE, Stroke::new(1.0, studio_ui::pro::BLUE), egui::StrokeKind::Middle);
                    }
                }
            }
            if let Some((a, b)) = self.ruler {
                let (pa, pb) = (v.to_scr(a), v.to_scr(b));
                painter.line_segment([pa, pb], Stroke::new(3.0, Color32::from_black_alpha(140)));
                painter.line_segment([pa, pb], Stroke::new(1.2, Color32::WHITE));
                for q in [pa, pb] {
                    painter.circle_stroke(q, 4.0, Stroke::new(1.2, Color32::WHITE));
                }
                let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
                painter.text(pb + vec2(8.0, -8.0), Align2::LEFT_BOTTOM, format!("{len:.0} px · {:.1}°", -(b.1 - a.1).atan2(b.0 - a.0).to_degrees()), semibold(12.0), Color32::WHITE);
            }
            if let (Drag::Gradient(start), Some(end)) = (&self.drag, self.gradient_end) {
                let (a, b) = (v.to_scr(*start), v.to_scr(end));
                painter.line_segment([a, b], Stroke::new(3.0, Color32::from_black_alpha(160)));
                painter.line_segment([a, b], Stroke::new(1.5, Color32::WHITE));
                painter.circle(a, 5.0, self.primary, Stroke::new(1.5, Color32::WHITE));
                painter.circle(b, 5.0, self.secondary, Stroke::new(1.5, Color32::WHITE));
            }
            if let (Drag::Patch { from }, Some(end), Some(s)) = (&self.drag, self.gradient_end, self.doc.selection) {
                let (dx, dy) = (end.0 - from.0, end.1 - from.1);
                painter.add(Shape::closed_line(v.quad(s.x0 as f32 + dx, s.y0 as f32 + dy, s.x1 as f32 + dx, s.y1 as f32 + dy).to_vec(), Stroke::new(1.5, Color32::WHITE)));
            }
            if let Drag::Lasso(pts) = &self.drag {
                let pts: Vec<Pos2> = pts.iter().map(|d| v.to_scr(*d)).collect();
                painter.add(Shape::line(pts.clone(), Stroke::new(2.5, Color32::from_black_alpha(160))));
                painter.add(Shape::line(pts, Stroke::new(1.0, Color32::WHITE)));
            }
            if !self.poly.is_empty() {
                let mut pts: Vec<Pos2> = self.poly.iter().map(|d| v.to_scr(*d)).collect();
                if let Some(p) = pointer {
                    pts.push(p);
                }
                painter.add(Shape::line(pts.clone(), Stroke::new(2.5, Color32::from_black_alpha(160))));
                painter.add(Shape::line(pts.clone(), Stroke::new(1.0, Color32::WHITE)));
                painter.circle_stroke(pts[0], 5.0, Stroke::new(1.5, Color32::WHITE));
            }
            if let (Drag::Guide { idx: None, vertical }, Some(d)) = (&self.drag, self.gradient_end) {
                let (a, b) = if *vertical { ((d.0, -100000.0), (d.0, 100000.0)) } else { ((-100000.0, d.1), (100000.0, d.1)) };
                painter.line_segment([v.to_scr(a), v.to_scr(b)], Stroke::new(1.0, Color32::from_rgb(0, 220, 255)));
            }
            if let Some(c) = self.crop {
                let q = v.quad(c.x0 as f32, c.y0 as f32, c.x1 as f32, c.y1 as f32);
                if self.view_angle == 0.0 {
                    let rect = Rect::from_two_pos(doc_q[0], doc_q[2]);
                    let r = Rect::from_two_pos(q[0], q[2]);
                    let shade = Color32::from_black_alpha(150);
                    for band in [
                        Rect::from_min_max(rect.min, pos2(rect.right(), r.top())),
                        Rect::from_min_max(pos2(rect.left(), r.bottom()), rect.max),
                        Rect::from_min_max(pos2(rect.left(), r.top()), pos2(r.left(), r.bottom())),
                        Rect::from_min_max(pos2(r.right(), r.top()), pos2(rect.right(), r.bottom())),
                    ] {
                        painter.rect_filled(band, CornerRadius::ZERO, shade);
                    }
                }
                painter.add(Shape::closed_line(q.to_vec(), Stroke::new(1.5, Color32::WHITE)));
                let gs = Stroke::new(0.8, Color32::from_white_alpha(110));
                let (cw, ch) = (c.width() as f32, c.height() as f32);
                let line = |a: (f32, f32), b: (f32, f32)| painter.line_segment([v.to_scr((c.x0 as f32 + a.0 * cw, c.y0 as f32 + a.1 * ch)), v.to_scr((c.x0 as f32 + b.0 * cw, c.y0 as f32 + b.1 * ch))], gs);
                match self.crop_overlay {
                    1 | 4 => {
                        let f = if self.crop_overlay == 1 { 1.0 / 3.0 } else { 0.382 };
                        for k in [f, 1.0 - f] {
                            line((k, 0.0), (k, 1.0));
                            line((0.0, k), (1.0, k));
                        }
                    }
                    2 => {
                        for k in 1..6 {
                            let t = k as f32 / 6.0;
                            line((t, 0.0), (t, 1.0));
                            line((0.0, t), (1.0, t));
                        }
                    }
                    3 => {
                        line((0.0, 0.0), (1.0, 1.0));
                        line((1.0, 0.0), (0.0, 1.0));
                    }
                    _ => {}
                }
            }
            if let (Drag::Shape(start), Some(end)) = (&self.drag, self.gradient_end) {
                let fill = self.primary.gamma_multiply(0.55);
                let (a, b) = if self.tool == Tool::Line { (*start, end) } else { constrain(*start, end, if shift { 1.0 } else { 0.0 }, alt) };
                let (x0, y0, x1, y1) = (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1));
                let polys: Vec<Vec<(f32, f32)>> = match self.tool {
                    Tool::Line => shapes::line(a, b, self.stroke_w, self.arrow_start, self.arrow_end),
                    Tool::Rect => vec![shapes::rect(x0, y0, x1, y1, self.corner)],
                    Tool::Ellipse => vec![shapes::ellipse(x0, y0, x1, y1)],
                    Tool::Polygon => vec![shapes::polygon(x0, y0, x1, y1, self.sides, self.star.then_some(self.inset))],
                    _ => vec![shapes::custom(self.custom, x0, y0, x1, y1)],
                };
                for poly in polys {
                    let pts: Vec<Pos2> = poly.into_iter().map(|p| v.to_scr(p)).collect();
                    if pts.len() >= 3 {
                        let mut mesh = egui::Mesh::default();
                        let c = pts.iter().fold(Vec2::ZERO, |s, p| s + p.to_vec2()) / pts.len() as f32;
                        mesh.colored_vertex(c.to_pos2(), fill);
                        for p in &pts {
                            mesh.colored_vertex(*p, fill);
                        }
                        for k in 0..pts.len() as u32 {
                            mesh.add_triangle(0, k + 1, (k + 1) % pts.len() as u32 + 1);
                        }
                        painter.add(mesh);
                        painter.add(Shape::closed_line(pts, Stroke::new(1.0, Color32::WHITE)));
                    }
                }
            }
            if let (Tool::Clone | Tool::Heal, Some(src)) = (self.tool, self.clone_src) {
                let c = v.to_scr(match (self.last, self.clone_offset) {
                    (Some(l), Some(o)) => (l.0 + o.0, l.1 + o.1),
                    _ => src,
                });
                painter.circle_stroke(c, 6.0, Stroke::new(1.5, Color32::WHITE));
                painter.line_segment([c - vec2(9.0, 0.0), c + vec2(9.0, 0.0)], Stroke::new(1.0, Color32::WHITE));
                painter.line_segment([c - vec2(0.0, 9.0), c + vec2(0.0, 9.0)], Stroke::new(1.0, Color32::WHITE));
            }
            if !self.remove_stroke.is_empty() {
                let pts: Vec<Pos2> = self.remove_stroke.iter().map(|d| v.to_scr(*d)).collect();
                let width = (self.brush.radius * 2.0 * zoom).max(2.0);
                let red = Color32::from_rgba_unmultiplied(255, 59, 80, 110);
                if pts.len() > 1 {
                    painter.add(Shape::line(pts.clone(), Stroke::new(width, red)));
                }
                painter.circle_filled(pts[0], width / 2.0, red);
                painter.circle_filled(pts[pts.len() - 1], width / 2.0, red);
            }
            if self.filling.is_some() {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
                let tag = Rect::from_center_size(area.center_top() + vec2(0.0, 40.0), vec2(260.0, 40.0));
                painter.rect_filled(tag, CornerRadius::same(20), Color32::from_black_alpha(200));
                let c = tag.left_center() + vec2(26.0, 0.0);
                for k in 0..8 {
                    let a = time * 6.0 + k as f32 * std::f32::consts::TAU / 8.0;
                    painter.circle_filled(c + vec2(a.cos(), a.sin()) * 8.0, 2.0, Color32::from_white_alpha(60 + k * 24));
                }
                painter.text(tag.left_center() + vec2(46.0, 0.0), Align2::LEFT_CENTER, "Reconstruction du fond…", semibold(13.0), Color32::WHITE);
            }
            if rulers {
                let p = ui.painter();
                p.rect_filled(top_ruler, CornerRadius::ZERO, CHROME);
                p.rect_filled(left_ruler, CornerRadius::ZERO, CHROME);
                p.rect_filled(Rect::from_min_max(pos2(full.left(), tab_bar.bottom()), pos2(area.left(), area.top())), CornerRadius::ZERO, CHROME);
                p.line_segment([top_ruler.left_bottom(), top_ruler.right_bottom()], Stroke::new(1.0, LINE));
                p.line_segment([left_ruler.right_top(), left_ruler.right_bottom()], Stroke::new(1.0, LINE));
                let step = [1.0f32, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0].into_iter().find(|s| s * zoom >= 60.0).unwrap_or(10000.0);
                let tick = Stroke::new(1.0, FAINT);
                let mut x = (vx0 / step).floor() * step;
                while x <= vx1 {
                    let sx = v.to_scr((x, 0.0)).x;
                    if sx >= top_ruler.left() {
                        p.line_segment([pos2(sx, top_ruler.top() + 4.0), pos2(sx, top_ruler.bottom())], tick);
                        p.text(pos2(sx + 3.0, top_ruler.top() + 1.0), Align2::LEFT_TOP, format!("{x:.0}"), FontId::proportional(9.5), MUTED);
                    }
                    for k in 1..10 {
                        let sx2 = v.to_scr((x + step * k as f32 / 10.0, 0.0)).x;
                        if sx2 >= top_ruler.left() && sx2 <= top_ruler.right() {
                            p.line_segment([pos2(sx2, top_ruler.bottom() - if k == 5 { 7.0 } else { 4.0 }), pos2(sx2, top_ruler.bottom())], tick);
                        }
                    }
                    x += step;
                }
                let mut y = (vy0 / step).floor() * step;
                while y <= vy1 {
                    let sy = v.to_scr((0.0, y)).y;
                    if sy >= left_ruler.top() {
                        p.line_segment([pos2(left_ruler.left() + 4.0, sy), pos2(left_ruler.right(), sy)], tick);
                        p.text(pos2(left_ruler.left() + 2.0, sy + 2.0), Align2::LEFT_TOP, format!("{y:.0}"), FontId::proportional(9.5), MUTED);
                    }
                    for k in 1..10 {
                        let sy2 = v.to_scr((0.0, y + step * k as f32 / 10.0)).y;
                        if sy2 >= left_ruler.top() && sy2 <= left_ruler.bottom() {
                            p.line_segment([pos2(left_ruler.right() - if k == 5 { 7.0 } else { 4.0 }, sy2), pos2(left_ruler.right(), sy2)], tick);
                        }
                    }
                    y += step;
                }
                if let Some((cx, cy)) = self.cursor {
                    let s = v.to_scr((cx, cy));
                    p.line_segment([pos2(s.x, top_ruler.top()), pos2(s.x, top_ruler.bottom())], Stroke::new(1.0, studio_ui::pro::BLUE));
                    p.line_segment([pos2(left_ruler.left(), s.y), pos2(left_ruler.right(), s.y)], Stroke::new(1.0, studio_ui::pro::BLUE));
                }
            }

            if resp.hovered() {
                ui.ctx().set_cursor_icon(match self.tool {
                    _ if panning || space => egui::CursorIcon::Grabbing,
                    Tool::Hand => egui::CursorIcon::Grab,
                    _ if painting && self.cursor_mode < 2 => egui::CursorIcon::None,
                    Tool::TextH | Tool::TextV => egui::CursorIcon::Text,
                    Tool::Move => egui::CursorIcon::Move,
                    Tool::Zoom => if alt != self.zoom_out { egui::CursorIcon::ZoomOut } else { egui::CursorIcon::ZoomIn },
                    Tool::RotateView => egui::CursorIcon::Alias,
                    _ => egui::CursorIcon::Crosshair,
                });
                if painting && !space
                    && let Some(at) = pointer
                {
                    let r = (self.brush.radius * zoom).max(2.0);
                    if self.cursor_mode < 2 {
                        let round = if brushy(self.tool) { self.brush.roundness.clamp(0.05, 1.0) } else { 1.0 };
                        let angle = if brushy(self.tool) { self.brush.angle.to_radians() } else { 0.0 } + self.view_angle.to_radians();
                        for (rr, c) in [(r, Color32::from_black_alpha(200)), (r + 1.0, Color32::from_white_alpha(200))] {
                            painter.add(egui::epaint::EllipseShape { center: at, radius: vec2(rr, rr * round), fill: Color32::TRANSPARENT, stroke: Stroke::new(1.0, c), angle });
                        }
                    }
                    if self.cursor_mode >= 1 {
                        painter.line_segment([at - vec2(5.0, 0.0), at + vec2(5.0, 0.0)], Stroke::new(1.0, Color32::WHITE));
                        painter.line_segment([at - vec2(0.0, 5.0), at + vec2(0.0, 5.0)], Stroke::new(1.0, Color32::WHITE));
                    }
                }
            }
        });
    }
}
