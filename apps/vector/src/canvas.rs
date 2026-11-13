use crate::model::{self, BBox, Geom, Guide, Item, MeshPt, Node, P, dist};
use crate::{Cam, Dlg, Drag, Tool, VectorApp, arr, c32, geom};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Frame, Key, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextureOptions, Ui, Vec2, pos2, vec2};
use resvg::{tiny_skia, usvg};
use studio_ui::pro::{BLUE, FAINT, INK, LINE, MUTED, PANEL, PASTEBOARD};

const MAGENTA: Color32 = Color32::from_rgb(255, 59, 160);
const CYAN: Color32 = Color32::from_rgb(0, 200, 255);

fn origs(app: &VectorApp) -> Vec<(usize, Item)> {
    app.selection().into_iter().map(|i| (i, app.doc.items[i].clone())).collect()
}

fn quad(p: &Painter, cam: &Cam, b: &BBox, s: Stroke) {
    let pts: Vec<Pos2> = b.corners().iter().map(|c| cam.to_scr(*c)).collect();
    p.add(Shape::closed_line(pts, s));
}

impl VectorApp {
    fn render(&mut self, ctx: &egui::Context) {
        let ppp = ctx.pixels_per_point();
        let a = self.area;
        let size = [(a.width() * ppp).ceil().max(1.0) as u32, (a.height() * ppp).ceil().max(1.0) as u32];
        let cam = self.cam();
        let k = ppp * self.zoom;
        let rp = [cam.co * cam.pc[0] - cam.s * cam.pc[1], cam.s * cam.pc[0] + cam.co * cam.pc[1]];
        let o = cam.c + cam.off - a.min.to_vec2();
        let m = [k * cam.co, k * cam.s, -k * cam.s, k * cam.co, ppp * o.x - k * rp[0], ppp * o.y - k * rp[1]];
        if self.rendered.as_ref().is_some_and(|(d, mm, s, ol)| *d == self.doc && *mm == m && *s == size && *ol == self.outline_view) {
            return;
        }
        let opt = usvg::Options { fontdb: self.fontdb.clone(), ..Default::default() };
        let cell = 8.0 / self.zoom;
        let svg = if self.outline_view { self.doc.outlined().view_svg(size[0] as f32, size[1] as f32, m, cell) } else { self.doc.view_svg(size[0] as f32, size[1] as f32, m, cell) };
        let tree = match usvg::Tree::from_str(&svg, &opt) {
            Ok(t) => t,
            Err(e) => {
                self.error = Some(format!("Rendu impossible : {e}"));
                return;
            }
        };
        let Some(mut pix) = tiny_skia::Pixmap::new(size[0], size[1]) else { return };
        resvg::render(&tree, tiny_skia::Transform::identity(), &mut pix.as_mut());
        let img = egui::ColorImage::from_rgba_premultiplied([size[0] as usize, size[1] as usize], pix.data());
        match &mut self.tex {
            Some(t) if t.size() == [size[0] as usize, size[1] as usize] => t.set(img, TextureOptions::LINEAR),
            _ => self.tex = Some(ctx.load_texture("art", img, TextureOptions::LINEAR)),
        }
        self.rendered = Some((self.doc.clone(), m, size, self.outline_view));
    }

    pub(crate) fn canvas(&mut self, root: &mut Ui, name: &str) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let tab_h = 32.0;
            let tab_bar = Rect::from_min_max(full.min, pos2(full.right(), full.top() + tab_h));
            let p = ui.painter();
            p.rect_filled(tab_bar, CornerRadius::ZERO, PANEL);
            let label = format!("{name} @ {:.0} %{}", self.zoom * 100.0, if self.doc != self.saved { " •" } else { "" });
            let g = p.layout_no_wrap(label, FontId::proportional(12.5), INK);
            let tab = Rect::from_min_size(tab_bar.min + vec2(8.0, 4.0), vec2(g.size().x + 28.0, tab_h - 4.0));
            p.rect_filled(tab, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, PASTEBOARD);
            p.galley(tab.left_center() + vec2(14.0, -g.size().y / 2.0), g, INK);

            let area = Rect::from_min_max(pos2(full.left(), tab_bar.bottom()), full.max);
            self.area = area;
            let rulers = self.rulers && self.rot == 0.0;
            let top_ruler = Rect::from_min_max(area.min, pos2(area.right(), area.top() + 20.0));
            let left_ruler = Rect::from_min_max(area.min, pos2(area.left() + 20.0, area.bottom()));
            let resp = ui.allocate_rect(area, Sense::click_and_drag());
            if self.fit_pending > 0 {
                let all = self.fit_pending == 2;
                self.fit_pending = 0;
                self.fit_view(all);
            }
            let (scroll, pinch, space, pointer, pressed, released, middle, delta, shift, alt, ctrl, down) = ui.input(|i| {
                (i.smooth_scroll_delta, i.zoom_delta(), i.key_down(Key::Space), i.pointer.hover_pos(), i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.middle_down(), i.pointer.delta(), i.modifiers.shift, i.modifiers.alt, i.modifiers.command, i.pointer.primary_down())
            });
            self.mods = (shift, alt, ctrl);
            if resp.hovered()
                && let Some(at) = pointer
            {
                if ctrl || alt {
                    let f = pinch * (scroll.y / 300.0).exp();
                    if (f - 1.0).abs() > 1e-4 {
                        self.zoom_at(f, at);
                    }
                } else if (pinch - 1.0).abs() > 1e-4 {
                    self.zoom_at(pinch, at);
                } else if scroll != Vec2::ZERO {
                    self.offset += if shift { vec2(scroll.y + scroll.x, 0.0) } else { scroll };
                }
            }
            let panning = (resp.dragged() && (self.tool == Tool::Hand || space) && !matches!(self.drag, Drag::Guide { .. })) || (resp.hovered() && middle);
            if panning {
                self.offset += delta;
            }
            let cam = self.cam();
            let tol = 6.0 / self.zoom;
            self.cursor = pointer.filter(|p| area.contains(*p)).map(|p| cam.to_doc(p));
            if !panning && !space && let Some(sp) = pointer {
                let raw = cam.to_doc(sp);
                if pressed && resp.hovered() {
                    if rulers && top_ruler.contains(sp) {
                        self.drag = Drag::Guide { vertical: false };
                    } else if rulers && left_ruler.contains(sp) {
                        self.drag = Drag::Guide { vertical: true };
                    } else {
                        let pt = self.snap_point(raw, tol, true);
                        match self.tool {
                            Tool::Zoom => self.zoom_at(if alt { 0.8 } else { 1.25 }, sp),
                            Tool::RotView => {
                                let a = (sp - area.center()).angle().to_degrees();
                                self.drag = Drag::RotView { start: a, base: self.rot };
                            }
                            _ => self.press(pt, raw, tol),
                        }
                    }
                } else if down && !matches!(self.drag, Drag::None) {
                    if let Drag::RotView { start, base } = self.drag {
                        let mut r = base + (sp - area.center()).angle().to_degrees() - start;
                        if shift {
                            r = (r / 15.0).round() * 15.0;
                        }
                        let keep = self.cam().to_doc(area.center());
                        self.rot = (r + 180.0).rem_euclid(360.0) - 180.0;
                        let c2 = self.cam().to_scr(keep);
                        self.offset += area.center() - c2;
                    } else if !matches!(self.drag, Drag::Guide { .. }) {
                        let pt = self.snap_point(raw, tol, false);
                        self.drag_to(pt, raw, tol);
                    }
                }
                if down && matches!(self.drag, Drag::Liquify { .. }) {
                    self.liquify_step(raw);
                    ui.ctx().request_repaint();
                }
            }
            if released {
                if let Drag::Guide { vertical } = self.drag
                    && let Some(sp) = pointer
                    && area.contains(sp)
                    && !(top_ruler.contains(sp) || left_ruler.contains(sp))
                {
                    let d = self.cam().to_doc(sp);
                    self.checkpoint();
                    self.doc.guides.push(Guide { vertical, pos: if vertical { d[0] } else { d[1] } });
                }
                self.release(tol);
                self.snap_lines.clear();
            }

            self.render(ui.ctx());
            let cam = self.cam();
            let painter = ui.painter_at(area);
            if self.rot == 0.0 {
                for b in self.doc.all_boards() {
                    let r = Rect::from_min_max(cam.to_scr([b.x, b.y]), cam.to_scr([b.x + b.w, b.y + b.h]));
                    painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(110) }.as_shape(r, CornerRadius::ZERO));
                }
            }
            if let Some(t) = &self.tex {
                painter.image(t.id(), area, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            for (k, b) in self.doc.all_boards().iter().enumerate() {
                let bb = BBox { min: [b.x, b.y], max: [b.x + b.w, b.y + b.h] };
                let active = k == self.active_board;
                quad(&painter, &cam, &bb, Stroke::new(if active && self.tool == Tool::Board { 2.0 } else { 1.0 }, if active { BLUE.gamma_multiply(0.8) } else { Color32::from_white_alpha(40) }));
                if self.doc.boards.len() > 0 || self.tool == Tool::Board {
                    painter.text(cam.to_scr([b.x, b.y]) - vec2(0.0, 6.0), Align2::LEFT_BOTTOM, &b.name, FontId::proportional(11.5), if active { INK } else { MUTED });
                }
            }
            let (w, h) = (self.doc.width, self.doc.height);
            if self.show_grid && self.grid * self.zoom >= 4.0 {
                let gs = Stroke::new(1.0, Color32::from_rgba_unmultiplied(10, 132, 255, 46));
                let mut x = 0.0;
                while x <= w {
                    painter.line_segment([cam.to_scr([x, 0.0]), cam.to_scr([x, h])], gs);
                    x += self.grid;
                }
                let mut y = 0.0;
                while y <= h {
                    painter.line_segment([cam.to_scr([0.0, y]), cam.to_scr([w, y])], gs);
                    y += self.grid;
                }
            }
            if self.thirds {
                let gs = Stroke::new(1.0, MAGENTA.gamma_multiply(0.45));
                for f in [1.0 / 3.0, 0.5, 2.0 / 3.0] {
                    painter.line_segment([cam.to_scr([w * f, 0.0]), cam.to_scr([w * f, h])], gs);
                    painter.line_segment([cam.to_scr([0.0, h * f]), cam.to_scr([w, h * f])], gs);
                }
            }
            if self.show_guides {
                let far = 1e6;
                for gd in &self.doc.guides {
                    let (a, b) = if gd.vertical { ([gd.pos, -far], [gd.pos, far]) } else { ([-far, gd.pos], [far, gd.pos]) };
                    painter.line_segment([cam.to_scr(a), cam.to_scr(b)], Stroke::new(1.0, CYAN.gamma_multiply(0.8)));
                }
            }
            if let (Drag::Guide { vertical }, Some(sp)) = (&self.drag, pointer) {
                let s = Stroke::new(1.0, CYAN);
                if *vertical {
                    painter.line_segment([pos2(sp.x, area.top()), pos2(sp.x, area.bottom())], s);
                } else {
                    painter.line_segment([pos2(area.left(), sp.y), pos2(area.right(), sp.y)], s);
                }
            }
            if matches!(self.tool, Tool::Select | Tool::Direct | Tool::GroupSel | Tool::Wand)
                && let Some(hp) = pointer
                && let Some(hi) = self.doc.hit(cam.to_doc(hp), tol)
                && !self.selection().contains(&hi)
            {
                quad(&painter, &cam, &self.doc.items[hi].bbox(), Stroke::new(1.0, BLUE.gamma_multiply(0.7)));
            }
            self.overlay(&painter, &cam, pointer);
            if rulers {
                self.draw_rulers(&painter, &cam, top_ruler, left_ruler);
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(match self.tool {
                    _ if panning || space => egui::CursorIcon::Grabbing,
                    _ if rulers && pointer.is_some_and(|p| top_ruler.contains(p) || left_ruler.contains(p)) => egui::CursorIcon::Crosshair,
                    Tool::Hand => egui::CursorIcon::Grab,
                    Tool::Select | Tool::Direct | Tool::GroupSel | Tool::Board => egui::CursorIcon::Default,
                    Tool::Text | Tool::AreaText | Tool::PathText | Tool::VText => egui::CursorIcon::Text,
                    Tool::Rotate | Tool::RotView | Tool::Twirl => egui::CursorIcon::Alias,
                    Tool::Wand | Tool::Picker | Tool::Bucket => egui::CursorIcon::PointingHand,
                    Tool::Zoom => if alt { egui::CursorIcon::ZoomOut } else { egui::CursorIcon::ZoomIn },
                    Tool::Scale | Tool::Distort => egui::CursorIcon::ResizeNwSe,
                    Tool::Blob | Tool::Eraser | Tool::Warp | Tool::Pucker | Tool::Bloat | Tool::Smooth | Tool::PathErase => egui::CursorIcon::None,
                    _ => egui::CursorIcon::Crosshair,
                });
            }
        });
    }

    fn zoom_at(&mut self, f: f32, at: Pos2) {
        let docp = self.cam().to_doc(at);
        self.zoom = (self.zoom * f).clamp(0.02, 64.0);
        let now = self.cam().to_scr(docp);
        self.offset += at - now;
    }

    fn draw_rulers(&self, p: &Painter, cam: &Cam, top: Rect, left: Rect) {
        p.rect_filled(top, CornerRadius::ZERO, PANEL);
        p.rect_filled(left, CornerRadius::ZERO, PANEL);
        p.rect_filled(Rect::from_min_size(top.min, Vec2::splat(20.0)), CornerRadius::ZERO, PANEL);
        let step = [1.0f32, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0].into_iter().find(|s| s * self.zoom >= 60.0).unwrap_or(50000.0);
        let s = Stroke::new(1.0, FAINT);
        let x0 = cam.to_doc(pos2(top.left() + 20.0, top.center().y))[0];
        let x1 = cam.to_doc(pos2(top.right(), top.center().y))[0];
        let mut x = (x0 / step).floor() * step;
        while x <= x1 {
            let sx = cam.to_scr([x, 0.0]).x;
            if sx > top.left() + 20.0 {
                p.line_segment([pos2(sx, top.bottom() - 8.0), pos2(sx, top.bottom())], s);
                p.text(pos2(sx + 2.0, top.top() + 2.0), Align2::LEFT_TOP, format!("{x:.0}"), FontId::proportional(9.5), MUTED);
            }
            let sub = cam.to_scr([x + step / 2.0, 0.0]).x;
            p.line_segment([pos2(sub, top.bottom() - 4.0), pos2(sub, top.bottom())], s);
            x += step;
        }
        let y0 = cam.to_doc(pos2(left.center().x, left.top() + 20.0))[1];
        let y1 = cam.to_doc(pos2(left.center().x, left.bottom()))[1];
        let mut y = (y0 / step).floor() * step;
        while y <= y1 {
            let sy = cam.to_scr([0.0, y]).y;
            if sy > left.top() + 20.0 {
                p.line_segment([pos2(left.right() - 8.0, sy), pos2(left.right(), sy)], s);
                p.text(pos2(left.left() + 2.0, sy + 2.0), Align2::LEFT_TOP, format!("{y:.0}"), FontId::proportional(9.0), MUTED);
            }
            let sub = cam.to_scr([0.0, y + step / 2.0]).y;
            p.line_segment([pos2(left.right() - 4.0, sub), pos2(left.right(), sub)], s);
            y += step;
        }
        if let Some(c) = self.cursor {
            let q = cam.to_scr(c);
            p.line_segment([pos2(q.x, top.top()), pos2(q.x, top.bottom())], Stroke::new(1.0, BLUE));
            p.line_segment([pos2(left.left(), q.y), pos2(left.right(), q.y)], Stroke::new(1.0, BLUE));
        }
        p.line_segment([top.left_bottom(), top.right_bottom()], Stroke::new(1.0, LINE));
        p.line_segment([left.right_top(), left.right_bottom()], Stroke::new(1.0, LINE));
    }

    fn snap_targets(&self, skip: &[usize]) -> [Vec<f32>; 2] {
        let mut t = [Vec::new(), Vec::new()];
        if self.smart {
            for (i, it) in self.doc.items.iter().enumerate() {
                if skip.contains(&i) || !self.doc.shown(i) {
                    continue;
                }
                let b = it.bbox();
                for a in 0..2 {
                    t[a].extend([b.min[a], b.center()[a], b.max[a]]);
                }
            }
            for b in self.doc.all_boards() {
                t[0].extend([b.x, b.x + b.w / 2.0, b.x + b.w]);
                t[1].extend([b.y, b.y + b.h / 2.0, b.y + b.h]);
            }
        }
        if self.show_guides {
            for g in &self.doc.guides {
                t[if g.vertical { 0 } else { 1 }].push(g.pos);
            }
        }
        t
    }

    fn snap_point(&mut self, raw: P, tol: f32, _press: bool) -> P {
        let mut p = raw;
        if self.snap && !matches!(self.tool, Tool::Pencil | Tool::Brush | Tool::Blob | Tool::Lasso | Tool::Knife | Tool::Eraser) {
            let g = self.grid.max(1.0);
            p = [(p[0] / g).round() * g, (p[1] / g).round() * g];
        }
        let creating = matches!(self.tool, Tool::Pen | Tool::Curv | Tool::Line | Tool::Arc | Tool::Rect | Tool::RRect | Tool::Ellipse | Tool::Polygon | Tool::Star | Tool::Arrow | Tool::Heart | Tool::Spiral | Tool::RectGrid | Tool::PolarGrid | Tool::Board | Tool::AreaText | Tool::Text | Tool::Measure) || matches!(self.drag, Drag::NodePart { .. });
        self.snap_lines.clear();
        if creating {
            let skip = if matches!(self.drag, Drag::Create { .. } | Drag::NodePart { .. }) { self.selection() } else { Vec::new() };
            let t = self.snap_targets(&skip);
            for a in 0..2 {
                if let Some(v) = t[a].iter().copied().filter(|v| (v - p[a]).abs() < tol).min_by(|x, y| (x - p[a]).abs().total_cmp(&(y - p[a]).abs())) {
                    p[a] = v;
                    self.snap_lines.push((a, v));
                }
            }
        }
        p
    }

    fn hit_node(&self, i: usize, p: P, tol: f32) -> Option<(usize, u8)> {
        let Geom::Path { nodes, .. } = &self.doc.items[i].geom else { return None };
        let near = |q: P| dist(q, p) <= tol;
        nodes.iter().position(|n| near(n.p)).map(|k| (k, 0)).or_else(|| nodes.iter().enumerate().find_map(|(k, n)| if n.cin != n.p && near(n.cin) { Some((k, 1)) } else if n.cout != n.p && near(n.cout) { Some((k, 2)) } else { None }))
    }

    fn path_target(&self, p: P, tol: f32) -> Option<usize> {
        self.selection().into_iter().rev().find(|i| self.doc.items[*i].hit_test(p, tol * 1.5)).or_else(|| self.doc.hit(p, tol))
    }

    fn handles(&self) -> Option<(BBox, [P; 8])> {
        let b = self.doc.bounds_of(&self.selection())?;
        let c = b.center();
        Some((b, [b.min, [b.max[0], b.min[1]], b.max, [b.min[0], b.max[1]], [c[0], b.min[1]], [b.max[0], c[1]], [c[0], b.max[1]], [b.min[0], c[1]]]))
    }

    fn press(&mut self, p: P, raw: P, tol: f32) {
        let (shift, alt, ctrl) = self.mods;
        match self.tool {
            Tool::Select | Tool::GroupSel => {
                if self.tool == Tool::Select && self.show_bbox && let Some((b, hs)) = self.handles()
                    && let Some(k) = hs.iter().position(|h| dist(*h, p) <= tol * 1.5)
                {
                    let opp = [2, 3, 0, 1, 6, 7, 4, 5][k];
                    let anchor = if alt { b.center() } else { hs[opp] };
                    let axis = match k {
                        4 | 6 => 2,
                        5 | 7 => 1,
                        _ => 0,
                    };
                    self.checkpoint();
                    self.drag = Drag::Scale { anchor, origs: origs(self), from: hs[k], axis };
                    return;
                }
                if self.tool == Tool::Select && self.selection().len() == 1 && let Some(i) = self.sel && let Some((node, part)) = self.hit_node(i, p, tol) {
                    self.node_press(i, node, part, alt, ctrl);
                    return;
                }
                match self.doc.hit(p, tol) {
                    Some(h) if shift => {
                        let members = if self.tool == Tool::Select { self.group_members(h) } else { vec![h] };
                        let mut cur = self.selection();
                        if cur.contains(&h) {
                            cur.retain(|k| !members.contains(k));
                        } else {
                            cur.extend(members);
                        }
                        self.set_selection(cur);
                    }
                    Some(h) => {
                        if !self.selection().contains(&h) {
                            if self.tool == Tool::Select {
                                self.select_with_group(h);
                            } else {
                                self.set_selection(vec![h]);
                            }
                        } else if self.tool == Tool::GroupSel {
                            self.select_with_group(h);
                        }
                        self.active_layer = self.doc.items[h].layer;
                        self.checkpoint();
                        if alt {
                            let items: Vec<Item> = self.selection().iter().map(|i| self.doc.items[*i].clone()).collect();
                            self.insert_copies(items, [0.0, 0.0]);
                        }
                        self.drag = Drag::Move { last: p, start: p, dup: alt };
                    }
                    None => {
                        if !self.lock_guides && self.show_guides && let Some(k) = self.doc.guides.iter().position(|g| (if g.vertical { raw[0] } else { raw[1] } - g.pos).abs() < tol * 0.6) {
                            self.checkpoint();
                            let g = self.doc.guides.remove(k);
                            self.drag = Drag::Guide { vertical: g.vertical };
                            return;
                        }
                        if !shift {
                            self.set_selection(Vec::new());
                        }
                        self.drag = Drag::Band { start: p, end: p };
                    }
                }
            }
            Tool::Direct => {
                if let Some(i) = self.sel && let Some((node, part)) = self.hit_node(i, p, tol) {
                    self.node_press(i, node, part, alt, ctrl);
                    return;
                }
                match self.doc.hit(p, tol) {
                    Some(h) => {
                        if shift {
                            let mut cur = self.selection();
                            cur.push(h);
                            self.set_selection(cur);
                        } else if !self.selection().contains(&h) {
                            self.set_selection(vec![h]);
                        }
                        self.checkpoint();
                        self.drag = Drag::Move { last: p, start: p, dup: false };
                    }
                    None => {
                        if !shift {
                            self.set_selection(Vec::new());
                        }
                        self.drag = Drag::Band { start: p, end: p };
                    }
                }
            }
            Tool::Wand => match self.doc.hit(p, tol) {
                Some(h) => self.magic_wand(h, shift),
                None if !shift => self.set_selection(Vec::new()),
                None => {}
            },
            Tool::Lasso | Tool::Brush | Tool::Blob | Tool::Pencil | Tool::Knife | Tool::Eraser | Tool::PathErase | Tool::Builder => {
                self.pencil = vec![raw];
                self.drag = Drag::Stroke;
            }
            Tool::Smooth | Tool::Warp | Tool::Twirl | Tool::Pucker | Tool::Bloat => {
                let r = if self.tool == Tool::Smooth { self.o.brush_r } else { self.o.liquify_r };
                let mut targets = self.selection();
                if targets.is_empty() {
                    targets = (0..self.doc.items.len()).filter(|i| self.doc.editable(*i) && !matches!(self.doc.items[*i].geom, Geom::Text { .. } | Geom::Image { .. }) && self.doc.items[*i].bbox().contains(raw, r)).collect();
                }
                targets.retain(|i| !matches!(self.doc.items[*i].geom, Geom::Text { .. } | Geom::Image { .. }));
                if targets.is_empty() {
                    self.info = Some("Rien à déformer ici : sélectionne une forme ou passe dessus".into());
                    return;
                }
                self.checkpoint();
                for &i in &targets {
                    let it = &mut self.doc.items[i];
                    it.to_path();
                    if let Geom::Path { nodes, closed } = &mut it.geom {
                        for _ in 0..5 {
                            let pts = model::flatten(nodes, *closed, 1);
                            let avg = model::polyline_len(&pts) / nodes.len().max(1) as f32;
                            if avg <= r / 5.0 || nodes.len() > 1500 {
                                break;
                            }
                            model::add_midpoints(nodes, *closed);
                        }
                    }
                }
                self.drag = Drag::Liquify { last: raw, targets };
            }
            Tool::Pen | Tool::Curv => self.pen_press(p, tol),
            Tool::AddPt => {
                let Some(i) = self.path_target(p, tol) else { return };
                let it = &mut self.doc.items[i];
                let mut c = it.clone();
                if !c.to_path() {
                    return;
                }
                if let Geom::Path { nodes, closed } = &c.geom
                    && let Some((s, t, d)) = model::nearest_on(nodes, *closed, p)
                    && d <= tol * 2.0
                {
                    self.checkpoint();
                    let mut n = nodes.clone();
                    let k = model::split_segment(&mut n, s, t);
                    c.geom = Geom::Path { nodes: n, closed: *closed };
                    self.doc.items[i] = c;
                    self.set_selection(vec![i]);
                    self.sel_node = Some(k);
                }
            }
            Tool::DelPt => {
                let cands: Vec<usize> = self.selection().into_iter().chain(self.doc.hit(p, tol)).collect();
                for i in cands {
                    if let Some((k, 0)) = self.hit_node(i, p, tol)
                        && let Geom::Path { nodes, .. } = &self.doc.items[i].geom
                        && nodes.len() > 2
                    {
                        self.checkpoint();
                        if let Geom::Path { nodes, .. } = &mut self.doc.items[i].geom {
                            nodes.remove(k);
                        }
                        self.set_selection(vec![i]);
                        return;
                    }
                }
                if let Some(h) = self.doc.hit(p, tol) {
                    self.set_selection(vec![h]);
                }
            }
            Tool::ConvPt => {
                let cands: Vec<usize> = self.selection().into_iter().chain(self.doc.hit(p, tol)).collect();
                for i in cands {
                    if let Some((k, part)) = self.hit_node(i, p, tol) {
                        self.checkpoint();
                        self.set_selection(vec![i]);
                        self.sel_node = Some(k);
                        if part == 0 {
                            if let Geom::Path { nodes, .. } = &mut self.doc.items[i].geom {
                                nodes[k] = Node::corner(nodes[k].p);
                            }
                            self.drag = Drag::ConvHandle { node: k };
                        } else {
                            self.drag = Drag::NodePart { node: k, part, free: true };
                        }
                        return;
                    }
                }
                if let Some(h) = self.doc.hit(p, tol) {
                    self.set_selection(vec![h]);
                    self.doc.items[h].to_path();
                }
            }
            Tool::Text | Tool::VText => {
                if let Some(h) = self.doc.hit(p, tol).filter(|h| matches!(self.doc.items[*h].geom, Geom::Text { .. })) {
                    self.set_selection(vec![h]);
                    self.dock_tab = 0;
                    return;
                }
                self.checkpoint();
                let mut it = self.new_item(Geom::Text { x: p[0], y: p[1], text: "Texte".into(), size: self.o.text_size, family: crate::FONTS[self.o.text_font].into(), bold: false });
                it.vertical = self.tool == Tool::VText;
                self.add_item(it);
                self.dock_tab = 0;
            }
            Tool::PathText => {
                let Some(h) = self.doc.hit(p, tol) else {
                    self.info = Some("Texte curviligne : clique sur un tracé".into());
                    return;
                };
                let Some((nodes, closed)) = self.doc.items[h].path_nodes() else { return };
                self.checkpoint();
                let mut n = nodes.clone();
                if closed && let Some(f) = n.first().copied() {
                    n.push(f);
                }
                let src = self.doc.items.remove(h);
                let mut it = self.new_item(Geom::Text { x: n[0].p[0], y: n[0].p[1], text: "Texte le long du tracé".into(), size: self.o.text_size, family: crate::FONTS[self.o.text_font].into(), bold: false });
                it.text_path = n;
                it.layer = src.layer;
                self.doc.items.insert(h, it);
                self.set_selection(vec![h]);
                self.dock_tab = 0;
            }
            Tool::AreaText | Tool::RectGrid | Tool::PolarGrid | Tool::Flare => self.drag = Drag::Box { start: p, end: p },
            Tool::Line | Tool::Arc | Tool::Spiral | Tool::Arrow | Tool::Rect | Tool::RRect | Tool::Ellipse | Tool::Polygon | Tool::Star | Tool::Heart => {
                self.checkpoint();
                let geom = match self.tool {
                    Tool::Rect => Geom::Rect { x: p[0], y: p[1], w: 0.0, h: 0.0, r: 0.0 },
                    Tool::RRect => Geom::Rect { x: p[0], y: p[1], w: 0.0, h: 0.0, r: self.o.rradius },
                    Tool::Ellipse => Geom::Ellipse { cx: p[0], cy: p[1], rx: 0.0, ry: 0.0 },
                    Tool::Line | Tool::Spiral | Tool::Arc if !(self.tool == Tool::Arc && self.o.arc_closed) => Geom::Path { nodes: vec![Node::corner(p), Node::corner(p)], closed: false },
                    _ => Geom::Path { nodes: model::polygon(p[0], p[1], 0.1, self.o.sides, None), closed: true },
                };
                let it = self.new_item(geom);
                self.add_item(it);
                self.drag = Drag::Create { start: p };
            }
            Tool::Rotate | Tool::Reflect | Tool::Scale => {
                if alt {
                    self.open_dlg(match self.tool {
                        Tool::Rotate => Dlg::Rotate { deg: 45.0 },
                        Tool::Reflect => Dlg::Reflect { axis: 1, angle: 0.0 },
                        _ => Dlg::Scale { sx: 100.0, sy: 100.0, uniform: true },
                    });
                    return;
                }
                if self.sel.is_none() {
                    if let Some(h) = self.doc.hit(p, tol) {
                        self.select_with_group(h);
                    }
                    return;
                }
                let Some(b) = self.doc.bounds_of(&self.selection()) else { return };
                let center = self.reference(&b);
                self.checkpoint();
                let o = origs(self);
                self.drag = match self.tool {
                    Tool::Rotate => Drag::Rotate { center, origs: o, start: (p[1] - center[1]).atan2(p[0] - center[0]).to_degrees() },
                    Tool::Reflect => Drag::Reflect { center, origs: o },
                    _ => Drag::ScaleTool { center, origs: o, start: p },
                };
            }
            Tool::Distort => {
                if let (Some(i), 1) = (self.sel, self.selection().len()) {
                    let b = self.doc.items[i].bbox();
                    if let Some(k) = b.corners().iter().position(|c| dist(*c, p) <= tol * 2.0) {
                        let mut orig = self.doc.items[i].clone();
                        if !orig.to_path() {
                            self.info = Some("Distorsion : vectorise d'abord le texte ou l'image".into());
                            return;
                        }
                        self.checkpoint();
                        self.drag = Drag::Distort { corner: k, orig, bbox: b };
                        return;
                    }
                }
                if let Some(h) = self.doc.hit(p, tol) {
                    self.set_selection(vec![h]);
                }
            }
            Tool::Reshape => {
                if let Some(i) = self.sel && self.doc.items[i].hit_test(p, tol * 2.0) && self.doc.items[i].to_path() {
                    self.checkpoint();
                    self.drag = Drag::Reshape { last: p };
                } else if let Some(h) = self.doc.hit(p, tol) {
                    self.set_selection(vec![h]);
                }
            }
            Tool::Width => {
                let Some(i) = self.path_target(p, tol).filter(|i| matches!(self.doc.items[*i].geom, Geom::Path { .. }) && self.doc.items[*i].stroke.is_some()) else {
                    self.info = Some("Largeur : clique sur un tracé avec contour".into());
                    return;
                };
                self.set_selection(vec![i]);
                let Geom::Path { nodes, closed } = &self.doc.items[i].geom else { return };
                let Some((s, t, _)) = model::nearest_on(nodes, *closed, p) else { return };
                let t = model::path_t(nodes, *closed, s, t);
                self.checkpoint();
                if alt {
                    let w = &mut self.doc.items[i].widths;
                    if let Some(k) = w.iter().position(|x| (x[0] - t).abs() < 0.05) {
                        w.remove(k);
                    }
                    return;
                }
                self.drag = Drag::Width { t };
            }
            Tool::Bucket => self.live_paint(p),
            Tool::Gradient => {
                if let Some(i) = self.sel.filter(|i| self.doc.items[*i].fill.is_some()) {
                    self.checkpoint();
                    let _ = i;
                    self.drag = Drag::Gradient { start: p };
                } else if let Some(h) = self.doc.hit(p, tol) {
                    self.set_selection(vec![h]);
                }
            }
            Tool::Mesh => {
                let Some(i) = self.sel.filter(|i| self.doc.items[*i].hit_test(p, tol)).or_else(|| self.doc.hit(p, tol)) else { return };
                if matches!(self.doc.items[i].geom, Geom::Text { .. } | Geom::Image { .. }) {
                    return;
                }
                self.checkpoint();
                self.set_selection(vec![i]);
                let it = &mut self.doc.items[i];
                if alt {
                    if let Some(k) = (0..it.mesh.len()).min_by(|a, b| dist(it.mesh[*a].p, p).total_cmp(&dist(it.mesh[*b].p, p))) {
                        it.mesh.remove(k);
                    }
                } else {
                    let c = self.style.fill.map(arr).unwrap_or([255, 255, 255, 255]);
                    it.mesh.push(MeshPt { p, c, r: self.o.mesh_r });
                    if it.fill.is_none() {
                        it.fill = Some([255, 255, 255, 255]);
                    }
                }
            }
            Tool::Picker => {
                let Some(src) = self.doc.hit(p, tol) else { return };
                let dst = self.selection();
                if dst.is_empty() || dst == vec![src] {
                    let it = &self.doc.items[src];
                    self.style.fill = it.fill.map(c32);
                    self.style.stroke = it.stroke.map(c32);
                    self.style.stroke_w = it.stroke_w;
                    self.info = Some("Style copié pour les nouveaux objets".into());
                } else {
                    self.pick_style(src, &dst);
                }
            }
            Tool::Measure => {
                self.o.measure = Some((p, p));
                self.drag = Drag::Measure { start: p };
            }
            Tool::Scissors => self.scissors(p, tol),
            Tool::Board => {
                let boards = self.doc.all_boards();
                match boards.iter().rposition(|b| BBox { min: [b.x, b.y], max: [b.x + b.w, b.y + b.h] }.contains(p, 0.0)) {
                    Some(k) => {
                        self.active_board = k;
                        if k > 0 {
                            self.checkpoint();
                            self.drag = Drag::BoardMove { idx: k - 1, last: p };
                        }
                    }
                    None => self.drag = Drag::Box { start: p, end: p },
                }
            }
            Tool::Hand | Tool::Zoom | Tool::RotView => {}
        }
    }

    fn node_press(&mut self, i: usize, node: usize, part: u8, alt: bool, ctrl: bool) {
        self.checkpoint();
        self.sel_node = Some(node);
        if part == 0 && (alt || ctrl) {
            if let Geom::Path { nodes, closed } = &mut self.doc.items[i].geom {
                if alt && nodes.len() > 2 {
                    nodes.remove(node);
                    self.sel_node = None;
                } else if ctrl {
                    model::toggle_node(nodes, node, *closed);
                }
            }
            return;
        }
        self.drag = Drag::NodePart { node, part, free: alt };
    }

    fn pen_press(&mut self, p: P, tol: f32) {
        self.checkpoint();
        let curv = self.tool == Tool::Curv;
        let idx = match self.pen.filter(|i| *i < self.doc.items.len()) {
            Some(i) => i,
            None => {
                let it = self.new_item(Geom::Path { nodes: Vec::new(), closed: false });
                let i = self.add_item(it);
                self.pen = Some(i);
                i
            }
        };
        self.set_selection(vec![idx]);
        let fill = self.style.fill.map(arr);
        let it = &mut self.doc.items[idx];
        if let Geom::Path { nodes, closed } = &mut it.geom {
            if nodes.len() >= 2 && dist(nodes[0].p, p) <= tol * 1.5 {
                *closed = true;
                if curv {
                    model::smooth(nodes, true);
                }
                if it.fill.is_none() {
                    it.fill = fill;
                }
                self.pen = None;
                self.drag = Drag::None;
                return;
            }
            nodes.push(Node::corner(p));
            if curv {
                model::smooth(nodes, false);
            } else {
                self.drag = Drag::PenHandle { node: nodes.len() - 1 };
            }
        }
    }

    fn drag_to(&mut self, p: P, raw: P, tol: f32) {
        let (shift, alt, _) = self.mods;
        match &mut self.drag {
            Drag::Stroke => {
                if self.pencil.last().is_none_or(|l| dist(*l, raw) > 0.5 / self.zoom) {
                    self.pencil.push(raw);
                }
                return;
            }
            Drag::Band { end, .. } | Drag::Box { end, .. } => {
                *end = p;
                return;
            }
            Drag::Measure { start } => {
                let mut q = p;
                if shift {
                    let (dx, dy) = (p[0] - start[0], p[1] - start[1]);
                    let a = (dy.atan2(dx) / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
                    let l = (dx * dx + dy * dy).sqrt();
                    q = [start[0] + a.cos() * l, start[1] + a.sin() * l];
                }
                self.o.measure = Some((*start, q));
                return;
            }
            Drag::BoardMove { idx, last } => {
                let d = [p[0] - last[0], p[1] - last[1]];
                if let Some(b) = self.doc.boards.get_mut(*idx) {
                    b.x += d[0];
                    b.y += d[1];
                }
                *last = p;
                return;
            }
            Drag::Move { last, .. } => {
                let mut d = [p[0] - last[0], p[1] - last[1]];
                let sel = self.selection();
                if (self.smart || self.show_guides) && let Some(b) = self.doc.bounds_of(&sel) {
                    let t = self.snap_targets(&sel);
                    self.snap_lines.clear();
                    for a in 0..2 {
                        let cand = [b.min[a] + d[a], b.center()[a] + d[a], b.max[a] + d[a]];
                        let best = t[a].iter().flat_map(|v| cand.iter().map(move |c| (*v - c, *v))).filter(|x| x.0.abs() < tol).min_by(|x, y| x.0.abs().total_cmp(&y.0.abs()));
                        if let Some((off, v)) = best {
                            d[a] += off;
                            self.snap_lines.push((a, v));
                        }
                    }
                }
                if let Drag::Move { last, .. } = &mut self.drag {
                    *last = [last[0] + d[0], last[1] + d[1]];
                }
                for i in sel {
                    self.doc.items[i].translate(d);
                }
                return;
            }
            _ => {}
        }
        let tool = self.tool;
        let scale_strokes = self.o.scale_strokes;
        let snap_angle = self.o.snap_angle;
        match &mut self.drag {
            Drag::Scale { anchor, origs, from, axis } => {
                let a = *anchor;
                let mut sx = if *axis == 2 { 1.0 } else { safe_div(p[0] - a[0], from[0] - a[0]) };
                let mut sy = if *axis == 1 { 1.0 } else { safe_div(p[1] - a[1], from[1] - a[1]) };
                if shift && *axis == 0 {
                    let m = sx.abs().max(sy.abs());
                    sx = m * sx.signum();
                    sy = m * sy.signum();
                } else if shift && *axis == 1 {
                    sy = sx.abs();
                } else if shift && *axis == 2 {
                    sx = sy.abs();
                }
                for (i, o) in origs.iter() {
                    let mut n = o.clone();
                    n.map(|q| [a[0] + (q[0] - a[0]) * sx, a[1] + (q[1] - a[1]) * sy], sx, sy);
                    if scale_strokes {
                        n.stroke_w = o.stroke_w * (sx * sy).abs().sqrt();
                    }
                    self.doc.items[*i] = n;
                }
            }
            Drag::Rotate { center, origs, start } => {
                let mut ang = (p[1] - center[1]).atan2(p[0] - center[0]).to_degrees() - *start;
                if shift {
                    ang = (ang / snap_angle).round() * snap_angle;
                }
                let (s, c) = ang.to_radians().sin_cos();
                for (i, o) in origs.iter() {
                    let mut n = o.clone();
                    crate::apply_affine(&mut n, [c, s, -s, c], *center, false);
                    self.doc.items[*i] = n;
                }
                self.info = Some(format!("Rotation {ang:.1}°"));
            }
            Drag::Reflect { center, origs } => {
                let mut ang = (p[1] - center[1]).atan2(p[0] - center[0]);
                if shift {
                    ang = (ang / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
                }
                let (s, c) = (2.0 * ang).sin_cos();
                for (i, o) in origs.iter() {
                    let mut n = o.clone();
                    crate::apply_affine(&mut n, [c, s, s, -c], *center, false);
                    self.doc.items[*i] = n;
                }
            }
            Drag::ScaleTool { center, origs, start } => {
                let c = *center;
                let mut sx = safe_div(p[0] - c[0], start[0] - c[0]);
                let mut sy = safe_div(p[1] - c[1], start[1] - c[1]);
                if shift {
                    let m = dist(p, c) / dist(*start, c).max(1e-3);
                    sx = m;
                    sy = m;
                }
                for (i, o) in origs.iter() {
                    let mut n = o.clone();
                    n.map(|q| [c[0] + (q[0] - c[0]) * sx, c[1] + (q[1] - c[1]) * sy], sx, sy);
                    if scale_strokes {
                        n.stroke_w = o.stroke_w * (sx * sy).abs().sqrt();
                    }
                    self.doc.items[*i] = n;
                }
                self.info = Some(format!("Échelle {:.0} % × {:.0} %", sx * 100.0, sy * 100.0));
            }
            Drag::Distort { corner, orig, bbox } => {
                let mut q = bbox.corners();
                q[*corner] = p;
                let (b, s) = (*bbox, bbox.size());
                let f = move |pt: P| {
                    let (u, v) = ((pt[0] - b.min[0]) / s[0].max(1e-3), (pt[1] - b.min[1]) / s[1].max(1e-3));
                    let top = model::lerp(q[0], q[1], u);
                    let bot = model::lerp(q[3], q[2], u);
                    model::lerp(top, bot, v)
                };
                let mut n = orig.clone();
                n.map(f, 1.0, 1.0);
                if let Some(i) = self.sel {
                    self.doc.items[i] = n;
                }
            }
            Drag::Reshape { last } => {
                let d = [p[0] - last[0], p[1] - last[1]];
                let l = *last;
                *last = p;
                if let Some(i) = self.sel {
                    let r = { let s = self.doc.items[i].bbox().size(); s[0].max(s[1]) / 3.0 };
                    if let Geom::Path { nodes, .. } = &mut self.doc.items[i].geom {
                        for n in nodes.iter_mut() {
                            let w = (1.0 - dist(n.p, l) / r).max(0.0).powi(2);
                            for q in [&mut n.p, &mut n.cin, &mut n.cout] {
                                q[0] += d[0] * w;
                                q[1] += d[1] * w;
                            }
                        }
                    }
                }
            }
            Drag::Width { t } => {
                let t = *t;
                if let Some(i) = self.sel {
                    let it = &mut self.doc.items[i];
                    if let Geom::Path { nodes, closed } = &it.geom
                        && let Some((_, _, d)) = model::nearest_on(nodes, *closed, p)
                    {
                        let m = (2.0 * d / it.stroke_w.max(0.1)).clamp(0.02, 40.0);
                        if it.widths.is_empty() {
                            it.widths = vec![[0.0, 1.0], [1.0, 1.0]];
                        }
                        match it.widths.iter_mut().find(|x| (x[0] - t).abs() < 0.02) {
                            Some(x) => x[1] = m,
                            None => it.widths.push([t, m]),
                        }
                    }
                }
            }
            Drag::Gradient { start } => {
                let s0 = *start;
                let radial = self.o.grad_radial;
                let second = self.style.stroke.map(arr).unwrap_or([255, 255, 255, 255]);
                for i in self.selection() {
                    let it = &mut self.doc.items[i];
                    let Some(f) = it.fill else { continue };
                    let b = it.bbox();
                    let s = b.size();
                    let rel = |q: P| [(q[0] - b.min[0]) / s[0].max(1e-3), (q[1] - b.min[1]) / s[1].max(1e-3)];
                    let (a, z) = (rel(s0), rel(p));
                    it.grad_vec = Some([a[0], a[1], z[0], z[1]]);
                    it.radial = radial;
                    if it.stops.is_empty() {
                        let c2 = it.fill2.unwrap_or(second);
                        it.stops = vec![model::Stop { t: 0.0, c: f }, model::Stop { t: 1.0, c: c2 }];
                        it.fill2 = None;
                    }
                }
            }
            Drag::NodePart { node, part, free } => {
                let free = *free || shift;
                let (node, part) = (*node, *part);
                if let Some(i) = self.sel
                    && let Geom::Path { nodes, .. } = &mut self.doc.items[i].geom
                    && let Some(n) = nodes.get_mut(node)
                {
                    match part {
                        0 => {
                            let d = [p[0] - n.p[0], p[1] - n.p[1]];
                            n.p = p;
                            n.cin = [n.cin[0] + d[0], n.cin[1] + d[1]];
                            n.cout = [n.cout[0] + d[0], n.cout[1] + d[1]];
                        }
                        1 => {
                            n.cin = p;
                            if !free {
                                n.cout = [2.0 * n.p[0] - p[0], 2.0 * n.p[1] - p[1]];
                            }
                        }
                        _ => {
                            n.cout = p;
                            if !free {
                                n.cin = [2.0 * n.p[0] - p[0], 2.0 * n.p[1] - p[1]];
                            }
                        }
                    }
                }
            }
            Drag::PenHandle { node } | Drag::ConvHandle { node } => {
                let node = *node;
                if let Some(i) = self.sel
                    && let Geom::Path { nodes, .. } = &mut self.doc.items[i].geom
                    && let Some(n) = nodes.get_mut(node)
                {
                    n.cout = p;
                    if !alt {
                        n.cin = [2.0 * n.p[0] - p[0], 2.0 * n.p[1] - p[1]];
                    }
                }
            }
            Drag::Create { start } => {
                let start = *start;
                let Some(i) = self.sel else { return };
                let (mut dx, mut dy) = (p[0] - start[0], p[1] - start[1]);
                let line = matches!(tool, Tool::Line | Tool::Arrow);
                if shift && !line {
                    let m = dx.abs().max(dy.abs());
                    dx = m * dx.signum();
                    dy = m * dy.signum();
                }
                if shift && line {
                    let a = (dy.atan2(dx) / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
                    let len = (dx * dx + dy * dy).sqrt();
                    dx = a.cos() * len;
                    dy = a.sin() * len;
                }
                let (x, y) = (start[0].min(start[0] + dx), start[1].min(start[1] + dy));
                let (w, h) = (dx.abs(), dy.abs());
                let end = [start[0] + dx, start[1] + dy];
                let o = &self.o;
                let it = &mut self.doc.items[i];
                it.geom = match (&it.geom, tool) {
                    (Geom::Rect { r, .. }, _) => Geom::Rect { x, y, w, h, r: r.min(w / 2.0).min(h / 2.0).max(if tool == Tool::RRect { o.rradius.min(w / 2.0).min(h / 2.0) } else { 0.0 }) },
                    (Geom::Ellipse { .. }, _) => Geom::Ellipse { cx: x + w / 2.0, cy: y + h / 2.0, rx: w / 2.0, ry: h / 2.0 },
                    (_, Tool::Line) => Geom::Path { nodes: vec![Node::corner(start), Node::corner(end)], closed: false },
                    (_, Tool::Arc) => {
                        let mut n = model::arc(start, end, o.arc_slope, o.arc_concave);
                        if o.arc_closed {
                            n.push(Node::corner(if o.arc_concave { [start[0], end[1]] } else { [end[0], start[1]] }));
                        }
                        Geom::Path { nodes: n, closed: o.arc_closed }
                    }
                    (_, Tool::Arrow) => Geom::Path { nodes: model::arrow(start, end, o.arrow_w), closed: true },
                    (_, Tool::Heart) => Geom::Path { nodes: model::heart(x, y, w.max(0.1), h.max(0.1)), closed: true },
                    (_, Tool::Spiral) => Geom::Path { nodes: model::spiral(start[0], start[1], (dx * dx + dy * dy).sqrt().max(0.1), if o.spiral_ccw { -o.turns } else { o.turns }), closed: false },
                    _ => {
                        let r = (dx * dx + dy * dy).sqrt();
                        let inner = (tool == Tool::Star).then_some(o.star_ratio);
                        let mut n = model::polygon(start[0], start[1], r.max(0.1), o.sides, inner);
                        let ang = dy.atan2(dx) + std::f32::consts::FRAC_PI_2;
                        if !shift {
                            let (s, c) = ang.sin_cos();
                            for k in &mut n {
                                let d = [k.p[0] - start[0], k.p[1] - start[1]];
                                *k = Node::corner([start[0] + d[0] * c - d[1] * s, start[1] + d[0] * s + d[1] * c]);
                            }
                        }
                        Geom::Path { nodes: n, closed: true }
                    }
                };
                if matches!(tool, Tool::Arc) && o.arc_closed && it.fill.is_none() {
                    it.fill = self.style.fill.map(arr);
                }
            }
            _ => {}
        }
    }

    fn liquify_step(&mut self, p: P) {
        let Drag::Liquify { last, targets } = &mut self.drag else { return };
        let d = [p[0] - last[0], p[1] - last[1]];
        *last = p;
        let targets = targets.clone();
        let (r, k) = if self.tool == Tool::Smooth { (self.o.brush_r, 0.35) } else { (self.o.liquify_r, self.o.liquify_k) };
        let alt = self.mods.1;
        let tool = self.tool;
        if tool == Tool::Warp && d == [0.0, 0.0] || tool == Tool::Smooth && d == [0.0, 0.0] {
            return;
        }
        for i in targets {
            let Some(it) = self.doc.items.get_mut(i) else { continue };
            let Geom::Path { nodes, closed } = &mut it.geom else { continue };
            if tool == Tool::Smooth {
                let n = nodes.len();
                let pts: Vec<P> = nodes.iter().map(|n| n.p).collect();
                let mut touched = false;
                for j in 0..n {
                    if !*closed && (j == 0 || j + 1 == n) {
                        continue;
                    }
                    let w = (1.0 - dist(pts[j], p) / r).max(0.0);
                    if w > 0.0 {
                        let avg = model::lerp(pts[(j + n - 1) % n], pts[(j + 1) % n], 0.5);
                        let np = model::lerp(pts[j], avg, w * k);
                        nodes[j] = Node::corner(np);
                        touched = true;
                    }
                }
                if touched {
                    let c = *closed;
                    let pts: Vec<P> = nodes.iter().map(|n| n.p).collect();
                    let fitted = model::fit(&pts, c, 70.0);
                    *nodes = fitted;
                }
                continue;
            }
            let f = |q: &mut P, w: f32| match tool {
                Tool::Warp => {
                    q[0] += d[0] * w;
                    q[1] += d[1] * w;
                }
                Tool::Twirl => {
                    let a = w * 0.06 * if alt { -1.0 } else { 1.0 };
                    let (s, c) = a.sin_cos();
                    let v = [q[0] - p[0], q[1] - p[1]];
                    *q = [p[0] + v[0] * c - v[1] * s, p[1] + v[0] * s + v[1] * c];
                }
                Tool::Pucker => *q = model::lerp(*q, p, w * 0.05),
                _ => *q = model::lerp(*q, p, -w * 0.05),
            };
            for n in nodes.iter_mut() {
                let w = (1.0 - dist(n.p, p) / r).max(0.0).powi(2) * k;
                if w > 0.0 {
                    f(&mut n.p, w);
                    f(&mut n.cin, w);
                    f(&mut n.cout, w);
                }
            }
        }
    }

    fn release(&mut self, tol: f32) {
        let drag = std::mem::replace(&mut self.drag, Drag::None);
        let (shift, alt, _) = self.mods;
        match drag {
            Drag::Stroke => self.finish_stroke(tol, shift, alt),
            Drag::Band { start, end } => {
                let band = BBox { min: [start[0].min(end[0]), start[1].min(end[1])], max: [start[0].max(end[0]), start[1].max(end[1])] };
                let mut hits: Vec<usize> = (0..self.doc.items.len()).filter(|&i| self.doc.editable(i) && !self.doc.items[i].clip && band.overlaps(&self.doc.items[i].bbox())).collect();
                if self.tool == Tool::Select {
                    let mut all = Vec::new();
                    for h in hits {
                        all.extend(self.group_members(h));
                    }
                    hits = all;
                }
                if shift {
                    hits.extend(self.selection());
                }
                hits.sort_unstable();
                hits.dedup();
                self.set_selection(hits);
            }
            Drag::Create { .. } => {
                if let Some(i) = self.sel {
                    let s = self.doc.items[i].bbox().size();
                    if s[0] < 2.0 && s[1] < 2.0 {
                        self.doc.items.remove(i);
                        self.sel = None;
                        self.undo.pop();
                    }
                }
            }
            Drag::Box { start, end } => self.finish_box(start, end, tol),
            Drag::Move { start, last, dup } => {
                if dup {
                    self.last_tf = Some((Dlg::Move { dx: last[0] - start[0], dy: last[1] - start[1] }, true));
                } else if dist(start, last) < 1e-4 {
                    self.undo.pop();
                }
            }
            Drag::Scale { origs, .. } | Drag::Rotate { origs, .. } | Drag::Reflect { origs, .. } | Drag::ScaleTool { origs, .. } => {
                if origs.iter().all(|(i, o)| self.doc.items.get(*i) == Some(o)) {
                    self.undo.pop();
                }
                self.info = None;
            }
            _ => {}
        }
    }

    fn finish_box(&mut self, start: P, end: P, tol: f32) {
        let (x, y) = (start[0].min(end[0]), start[1].min(end[1]));
        let (w, h) = ((end[0] - start[0]).abs(), (end[1] - start[1]).abs());
        let small = w < 3.0 && h < 3.0;
        match self.tool {
            Tool::Board => {
                if !small {
                    self.checkpoint();
                    let n = self.doc.boards.len() + 2;
                    self.doc.boards.push(model::Board { name: format!("Plan de travail {n}"), x, y, w, h });
                    self.active_board = self.doc.boards.len();
                }
            }
            Tool::AreaText => {
                let (bx, by, bw, bh) = if small {
                    match self.doc.hit(start, tol) {
                        Some(hh) if !matches!(self.doc.items[hh].geom, Geom::Text { .. }) => {
                            let b = self.doc.items[hh].bbox();
                            (b.min[0] + 8.0, b.min[1] + 8.0, b.size()[0] - 16.0, b.size()[1] - 16.0)
                        }
                        _ => (start[0], start[1], 320.0, 200.0),
                    }
                } else {
                    (x, y, w, h)
                };
                self.checkpoint();
                let size = self.o.text_size.min(bh.max(8.0));
                let mut it = self.new_item(Geom::Text { x: bx, y: by + size * 0.8, text: "Texte captif : tape ton paragraphe dans le panneau, il se replie tout seul dans le cadre.".into(), size, family: crate::FONTS[self.o.text_font].into(), bold: false });
                it.box_w = bw.max(20.0);
                it.box_h = bh.max(20.0);
                self.add_item(it);
                self.dock_tab = 0;
            }
            Tool::RectGrid | Tool::PolarGrid | Tool::Flare => {
                if small && self.tool != Tool::Flare {
                    return;
                }
                self.checkpoint();
                let g = self.next_group();
                let stroke = self.style.stroke.or(self.style.fill).map(arr).unwrap_or([0, 0, 0, 255]);
                let line = |a: P, b: P, name: String| Item { name, geom: Geom::Path { nodes: vec![Node::corner(a), Node::corner(b)], closed: false }, stroke: Some(stroke), stroke_w: self.style.stroke_w, group: g, layer: self.active_layer, ..Item::blank() };
                let mut items = Vec::new();
                match self.tool {
                    Tool::RectGrid => {
                        if self.o.grid_frame {
                            let mut f = self.new_item(Geom::Rect { x, y, w, h, r: 0.0 });
                            f.group = g;
                            f.stroke = Some(stroke);
                            items.push(f);
                        }
                        for r in 1..self.o.grid_rows {
                            let yy = y + h * r as f32 / self.o.grid_rows as f32;
                            items.push(line([x, yy], [x + w, yy], format!("Rangée {r}")));
                        }
                        for c in 1..self.o.grid_cols {
                            let xx = x + w * c as f32 / self.o.grid_cols as f32;
                            items.push(line([xx, y], [xx, y + h], format!("Colonne {c}")));
                        }
                    }
                    Tool::PolarGrid => {
                        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
                        for k in 1..=self.o.polar_rings {
                            let f = k as f32 / self.o.polar_rings as f32;
                            let mut e = self.new_item(Geom::Ellipse { cx, cy, rx: w / 2.0 * f, ry: h / 2.0 * f });
                            e.fill = None;
                            e.stroke = Some(stroke);
                            e.group = g;
                            e.name = format!("Cercle {k}");
                            items.push(e);
                        }
                        for k in 0..self.o.polar_spokes {
                            let a = k as f32 / self.o.polar_spokes as f32 * std::f32::consts::TAU;
                            items.push(line([cx, cy], [cx + a.cos() * w / 2.0, cy + a.sin() * h / 2.0], format!("Rayon {}", k + 1)));
                        }
                    }
                    _ => {
                        let c = start;
                        let rr = dist(start, end).max(40.0);
                        let white = |a: u8| [255, 255, 255, a];
                        let radial = |name: &str, geom: Geom, a0: u8| Item { name: name.into(), geom, fill: Some(white(a0)), radial: true, stops: vec![model::Stop { t: 0.0, c: white(a0) }, model::Stop { t: 1.0, c: white(0) }], group: g, layer: self.active_layer, ..Item::blank() };
                        items.push(radial("Halo", Geom::Ellipse { cx: c[0], cy: c[1], rx: rr * 0.6, ry: rr * 0.6 }, 200));
                        if self.o.flare_rays > 0 {
                            items.push(radial("Rayons", Geom::Path { nodes: model::polygon(c[0], c[1], rr * 1.3, self.o.flare_rays, Some(0.04)), closed: true }, 170));
                        }
                        items.push(radial("Centre", Geom::Ellipse { cx: c[0], cy: c[1], rx: rr * 0.14, ry: rr * 0.14 }, 255));
                        let dir = if small { [1.0, 0.6] } else { [(end[0] - c[0]) / rr, (end[1] - c[1]) / rr] };
                        for k in 0..self.o.flare_rings {
                            let t = 1.2 + k as f32 * 0.9;
                            let r2 = rr * (0.12 + 0.1 * ((k * 7 % 5) as f32 / 5.0));
                            items.push(Item { name: format!("Anneau {}", k + 1), geom: Geom::Ellipse { cx: c[0] + dir[0] * rr * t, cy: c[1] + dir[1] * rr * t, rx: r2, ry: r2 }, fill: Some(white(30)), stroke: Some(white(90)), stroke_w: 1.5, group: g, layer: self.active_layer, ..Item::blank() });
                        }
                    }
                }
                self.insert_copies(items, [0.0, 0.0]);
            }
            _ => {}
        }
    }

    fn finish_stroke(&mut self, tol: f32, shift: bool, alt: bool) {
        let pts = std::mem::take(&mut self.pencil);
        if pts.len() < 2 && !matches!(self.tool, Tool::Builder) {
            return;
        }
        match self.tool {
            Tool::Pencil | Tool::Brush => {
                if pts.len() < 3 {
                    return;
                }
                let mut nodes = model::simplify(&pts, self.o.pencil_tol / self.zoom);
                let closed = self.tool == Tool::Pencil && nodes.len() > 3 && dist(pts[0], pts[pts.len() - 1]) < tol * 2.0;
                if closed {
                    nodes.pop();
                }
                model::smooth(&mut nodes, closed);
                self.checkpoint();
                let mut it = self.new_item(Geom::Path { nodes, closed });
                if self.tool == Tool::Brush {
                    it.name = format!("Coup de pinceau {}", self.doc.items.len() + 1);
                    it.fill = None;
                    it.stroke = self.style.stroke.or(self.style.fill).map(arr).or(Some([0, 0, 0, 255]));
                    it.stroke_w = self.o.brush_w;
                    it.profile = self.o.brush_profile;
                    it.cap = 0;
                    it.join = 0;
                }
                self.add_item(it);
            }
            Tool::Blob => {
                let Some(loops) = geom::brush_region(&pts, self.o.blob_w / 2.0) else { return };
                let fill = self.style.fill.or(self.style.stroke).map(arr).unwrap_or([0, 0, 0, 255]);
                let base = Item { name: format!("Goutte {}", self.doc.items.len() + 1), fill: Some(fill), layer: self.active_layer, ..Item::blank() };
                let Some(new) = geom::to_item(&loops, (self.o.blob_w / 60.0).max(0.2), &base) else { return };
                self.checkpoint();
                let nb = new.bbox();
                let mates: Vec<usize> = if self.o.blob_merge {
                    (0..self.doc.items.len()).filter(|i| {
                        let it = &self.doc.items[*i];
                        self.doc.editable(*i) && it.fill == Some(fill) && it.stroke.is_none() && matches!(it.geom, Geom::Path { closed: true, .. }) && it.bbox().overlaps(&nb) && it.layer == self.active_layer
                    }).collect()
                } else {
                    Vec::new()
                };
                if mates.is_empty() {
                    self.add_item(new);
                } else {
                    let mut items: Vec<Item> = mates.iter().map(|i| self.doc.items[*i].clone()).collect();
                    items.push(new);
                    let out = geom::pathfinder(&items, 0);
                    let mut out = out;
                    for o in &mut out {
                        o.name = self.doc.items[mates[0]].name.clone();
                    }
                    self.replace_items(&mates, out);
                }
            }
            Tool::Lasso => {
                let poly = vec![pts.clone()];
                let mut hits: Vec<usize> = (0..self.doc.items.len())
                    .filter(|i| self.doc.editable(*i) && {
                        let it = &self.doc.items[*i];
                        model::point_in(&poly, it.bbox().center()) || it.path_nodes().is_some_and(|(n, _)| n.iter().any(|k| model::point_in(&poly, k.p)))
                    })
                    .collect();
                if shift {
                    hits.extend(self.selection());
                }
                hits.sort_unstable();
                hits.dedup();
                self.set_selection(hits);
            }
            Tool::Knife | Tool::Eraser => {
                let r = if self.tool == Tool::Knife { 0.6 / self.zoom } else { self.o.erase_r };
                let sel = self.selection();
                let pb = BBox::of(pts.iter().copied());
                let cands: Vec<usize> = (0..self.doc.items.len())
                    .filter(|i| (sel.is_empty() || sel.contains(i)) && self.doc.editable(*i) && !matches!(self.doc.items[*i].geom, Geom::Text { .. } | Geom::Image { .. } | Geom::Path { closed: false, .. }) && {
                        let b = self.doc.items[*i].bbox();
                        BBox { min: [b.min[0] - r, b.min[1] - r], max: [b.max[0] + r, b.max[1] + r] }.overlaps(&pb)
                    })
                    .collect();
                let mut snapshot = Some(self.doc.clone());
                let mut changed = 0;
                for &i in cands.iter().rev() {
                    let it = self.doc.items[i].clone();
                    let Some(pieces) = geom::erase(&it, &pts, r) else { continue };
                    let before_area: f32 = it.loops().iter().map(|l| model::area(l).abs()).sum();
                    let after: f32 = pieces.iter().flat_map(|p| p.loops()).map(|l| model::area(&l).abs()).sum();
                    if self.tool == Tool::Knife && pieces.len() < 2 || self.tool == Tool::Eraser && (before_area - after).abs() < before_area * 0.002 {
                        continue;
                    }
                    if let Some(s) = snapshot.take() {
                        self.undo.push(s);
                        self.redo.clear();
                    }
                    self.doc.items.remove(i);
                    for (k, mut p) in pieces.into_iter().enumerate() {
                        p.name = format!("{} {}", it.name, k + 1);
                        self.doc.items.insert(i + k, p);
                    }
                    changed += 1;
                }
                if changed == 0 {
                    self.info = Some(if self.tool == Tool::Knife { "Cutter : traverse entièrement une forme fermée".into() } else { "Gomme : rien à effacer ici".into() });
                }
                self.set_selection(Vec::new());
            }
            Tool::PathErase => {
                let r = self.o.brush_r;
                let sel = self.selection();
                let mut snapshot = Some(self.doc.clone());
                for &i in sel.iter().rev() {
                    let Geom::Path { nodes, closed } = &self.doc.items[i].geom else { continue };
                    let keep: Vec<bool> = nodes.iter().map(|n| !pts.iter().any(|q| dist(*q, n.p) < r)).collect();
                    if keep.iter().all(|k| *k) {
                        continue;
                    }
                    let n = nodes.len();
                    let start = if *closed { keep.iter().position(|k| !*k).unwrap_or(0) } else { 0 };
                    let order: Vec<usize> = (0..n).map(|k| (start + k) % n).collect();
                    let mut runs: Vec<Vec<Node>> = Vec::new();
                    let mut cur = Vec::new();
                    for &k in &order {
                        if keep[k] {
                            cur.push(nodes[k]);
                        } else if !cur.is_empty() {
                            runs.push(std::mem::take(&mut cur));
                        }
                    }
                    if !cur.is_empty() {
                        runs.push(cur);
                    }
                    if let Some(s) = snapshot.take() {
                        self.undo.push(s);
                        self.redo.clear();
                    }
                    let base = self.doc.items.remove(i);
                    for (k, r) in runs.into_iter().filter(|r| r.len() >= 2).enumerate() {
                        self.doc.items.insert(i + k, Item { name: format!("{} {}", base.name, k + 1), geom: Geom::Path { nodes: r, closed: false }, sub: Vec::new(), ..base.clone() });
                    }
                }
                self.set_selection(Vec::new());
            }
            Tool::Builder => {
                let sel = self.selection();
                if sel.len() < 2 {
                    self.info = Some("Concepteur de forme : sélectionne d'abord au moins deux formes qui se chevauchent".into());
                    return;
                }
                let items: Vec<Item> = sel.iter().map(|i| self.doc.items[*i].clone()).collect();
                let mut keys: Vec<u64> = Vec::new();
                let step = (pts.len() / 60).max(1);
                for q in pts.iter().step_by(step).chain(pts.last()) {
                    let m = geom::membership(&items, *q);
                    if m != 0 && !keys.contains(&m) {
                        keys.push(m);
                    }
                }
                if keys.is_empty() {
                    return;
                }
                let out = geom::shape_build(&items, &keys, alt, self.style.fill.map(arr));
                if out.is_empty() {
                    return;
                }
                self.checkpoint();
                self.replace_items(&sel, out);
            }
            _ => {}
        }
    }

    fn live_paint(&mut self, p: P) {
        let idx: Vec<usize> = (0..self.doc.items.len())
            .filter(|i| self.doc.shown(*i) && !matches!(self.doc.items[*i].geom, Geom::Text { .. } | Geom::Image { .. }) && !self.doc.items[*i].clip && self.doc.items[*i].bbox().contains(p, self.doc.items[*i].stroke_w))
            .collect();
        let items: Vec<Item> = idx.iter().map(|i| self.doc.items[*i].clone()).collect();
        if items.len() > 62 || items.is_empty() {
            self.info = Some("Pot de peinture : clique à l'intérieur d'une zone fermée".into());
            return;
        }
        let m = geom::membership(&items, p);
        if m == 0 {
            self.info = Some("Pot de peinture : aucune zone fermée sous le clic".into());
            return;
        }
        let Some(mut it) = geom::build_region(&items, &[m]) else { return };
        it.name = format!("Zone peinte {}", self.doc.items.len() + 1);
        it.fill = self.style.fill.map(arr).or(Some([255, 176, 46, 255]));
        it.stroke = None;
        it.layer = self.active_layer;
        self.checkpoint();
        let top = *idx.last().unwrap();
        self.doc.items.insert(top + 1, it);
        self.set_selection(vec![top + 1]);
    }

    fn scissors(&mut self, p: P, tol: f32) {
        let Some(i) = self.path_target(p, tol) else { return };
        let Some((nodes, closed)) = self.doc.items[i].path_nodes() else { return };
        let Some((s, t, d)) = model::nearest_on(&nodes, closed, p) else { return };
        if d > tol * 2.0 {
            return;
        }
        self.checkpoint();
        let mut n = nodes.clone();
        let k = if t < 0.02 { s } else if t > 0.98 { (s + 1) % n.len() } else { model::split_segment(&mut n, s, t) };
        let base = self.doc.items[i].clone();
        let mk = |nodes: Vec<Node>, suffix: &str| Item { name: format!("{}{suffix}", base.name), geom: Geom::Path { nodes, closed: false }, ..base.clone() };
        if closed {
            let mut r: Vec<Node> = n[k..].to_vec();
            r.extend_from_slice(&n[..=k]);
            let first = r[0];
            let last = r.len() - 1;
            r[last] = Node { p: first.p, cin: r[last].cin, cout: first.p };
            r[0].cin = first.p;
            self.doc.items[i] = mk(r, "");
            self.set_selection(vec![i]);
        } else {
            if k == 0 || k + 1 >= n.len() {
                self.undo.pop();
                return;
            }
            let a = n[..=k].to_vec();
            let b = n[k..].to_vec();
            self.doc.items[i] = mk(a, " a");
            self.doc.items.insert(i + 1, mk(b, " b"));
            self.set_selection(vec![i, i + 1]);
        }
    }

    fn overlay(&self, painter: &Painter, cam: &Cam, pointer: Option<Pos2>) {
        let blue = Stroke::new(1.0, BLUE);
        match &self.drag {
            Drag::Band { start, end } => {
                let b = BBox { min: [start[0].min(end[0]), start[1].min(end[1])], max: [start[0].max(end[0]), start[1].max(end[1])] };
                let pts: Vec<Pos2> = b.corners().iter().map(|c| cam.to_scr(*c)).collect();
                painter.add(Shape::convex_polygon(pts, BLUE.gamma_multiply(0.12), blue));
            }
            Drag::Box { start, end } => {
                if self.tool == Tool::Flare {
                    painter.circle_stroke(cam.to_scr(*start), dist(*start, *end) * self.zoom, blue);
                } else {
                    let b = BBox { min: [start[0].min(end[0]), start[1].min(end[1])], max: [start[0].max(end[0]), start[1].max(end[1])] };
                    let pts: Vec<Pos2> = b.corners().iter().map(|c| cam.to_scr(*c)).collect();
                    let mut closed = pts.clone();
                    closed.push(pts[0]);
                    painter.extend(Shape::dashed_line(&closed, blue, 5.0, 3.0));
                    let s = b.size();
                    painter.text(cam.to_scr(b.max) + vec2(6.0, 6.0), Align2::LEFT_TOP, format!("{:.0} × {:.0}", s[0], s[1]), FontId::proportional(11.5), BLUE);
                }
            }
            _ => {}
        }
        if self.pencil.len() > 1 {
            let pts: Vec<Pos2> = self.pencil.iter().map(|q| cam.to_scr(*q)).collect();
            let (w, col) = match self.tool {
                Tool::Blob => (self.o.blob_w * self.zoom, self.style.fill.unwrap_or(BLUE).gamma_multiply(0.5)),
                Tool::Eraser => (self.o.erase_r * 2.0 * self.zoom, Color32::from_rgba_unmultiplied(255, 69, 58, 70)),
                Tool::Brush => ((self.o.brush_w * self.zoom).max(1.0), self.style.stroke.unwrap_or(BLUE).gamma_multiply(0.7)),
                Tool::Builder => (3.0, Color32::from_rgba_unmultiplied(48, 209, 88, 200)),
                _ => (1.5, BLUE),
            };
            if self.tool == Tool::Lasso {
                let mut c = pts.clone();
                c.push(pts[0]);
                painter.extend(Shape::dashed_line(&c, blue, 4.0, 3.0));
            } else {
                painter.add(Shape::line(pts, Stroke::new(w, col)));
            }
        }
        for (a, v) in &self.snap_lines {
            let far = 1e6;
            let (p0, p1) = if *a == 0 { ([*v, -far], [*v, far]) } else { ([-far, *v], [far, *v]) };
            painter.line_segment([cam.to_scr(p0), cam.to_scr(p1)], Stroke::new(1.0, MAGENTA));
        }
        if let (Some((a, b)), Tool::Measure) = (self.o.measure, self.tool) {
            let (pa, pb) = (cam.to_scr(a), cam.to_scr(b));
            painter.line_segment([pa, pb], Stroke::new(1.5, MAGENTA));
            painter.circle_filled(pa, 3.0, MAGENTA);
            painter.circle_filled(pb, 3.0, MAGENTA);
            let g = painter.layout_no_wrap(format!("{:.1} px · {:.1}°", dist(a, b), (a[1] - b[1]).atan2(b[0] - a[0]).to_degrees()), FontId::proportional(11.5), Color32::WHITE);
            let tag = Rect::from_center_size(pb + vec2(0.0, -18.0), g.size() + vec2(12.0, 6.0));
            painter.rect_filled(tag, CornerRadius::same(6), MAGENTA);
            painter.galley(tag.center() - g.size() / 2.0, g, Color32::WHITE);
        }
        if let Some(sp) = pointer.filter(|p| self.area.contains(*p)) {
            let r = match self.tool {
                Tool::Blob => Some(self.o.blob_w / 2.0),
                Tool::Eraser => Some(self.o.erase_r),
                Tool::Warp | Tool::Twirl | Tool::Pucker | Tool::Bloat => Some(self.o.liquify_r),
                Tool::Smooth | Tool::PathErase => Some(self.o.brush_r),
                _ => None,
            };
            if let Some(r) = r {
                painter.circle_stroke(sp, r * self.zoom, Stroke::new(1.0, Color32::WHITE));
                painter.circle_stroke(sp, r * self.zoom + 1.0, Stroke::new(1.0, Color32::from_black_alpha(160)));
            }
        }
        let all = self.selection();
        if all.is_empty() {
            return;
        }
        if self.show_edges {
            for &k in &all {
                let key = self.align_to == 2 && Some(k) == self.sel && all.len() > 1;
                quad(painter, cam, &self.doc.items[k].bbox(), Stroke::new(if key { 2.5 } else { 1.0 }, BLUE.gamma_multiply(if all.len() > 1 && !key { 0.6 } else { 1.0 })));
            }
        }
        let single = (all.len() == 1).then(|| all[0]);
        if let Some(i) = single
            && let Some(it) = self.doc.items.get(i)
            && let Geom::Path { nodes, .. } = &it.geom
            && self.show_edges
            && !matches!(self.tool, Tool::Rotate | Tool::Reflect | Tool::Scale | Tool::Board)
        {
            for (k, n) in nodes.iter().enumerate() {
                for c in [n.cin, n.cout] {
                    if c != n.p {
                        painter.line_segment([cam.to_scr(n.p), cam.to_scr(c)], Stroke::new(1.0, BLUE.gamma_multiply(0.8)));
                        painter.circle(cam.to_scr(c), 3.5, Color32::WHITE, blue);
                    }
                }
                let q = Rect::from_center_size(cam.to_scr(n.p), Vec2::splat(7.0));
                painter.rect(q, CornerRadius::ZERO, if self.sel_node == Some(k) { BLUE } else { Color32::WHITE }, blue, StrokeKind::Inside);
            }
            if self.tool == Tool::Width {
                for wv in &it.widths {
                    if let Some(pt) = point_at(nodes, matches!(it.geom, Geom::Path { closed: true, .. }), wv[0]) {
                        painter.circle(cam.to_scr(pt), 4.0, MAGENTA, Stroke::new(1.0, Color32::WHITE));
                    }
                }
            }
        }
        if let Some(i) = single && let Some(it) = self.doc.items.get(i) {
            if self.tool == Tool::Gradient && let Some(v) = it.grad_vec {
                let b = it.bbox();
                let s = b.size();
                let a = cam.to_scr([b.min[0] + v[0] * s[0], b.min[1] + v[1] * s[1]]);
                let z = cam.to_scr([b.min[0] + v[2] * s[0], b.min[1] + v[3] * s[1]]);
                painter.line_segment([a, z], Stroke::new(2.0, Color32::WHITE));
                painter.line_segment([a, z], Stroke::new(1.0, Color32::BLACK));
                painter.circle(a, 5.0, it.stops.first().map_or(Color32::WHITE, |s| c32(s.c)), Stroke::new(1.5, Color32::WHITE));
                painter.circle(z, 5.0, it.stops.last().map_or(Color32::WHITE, |s| c32(s.c)), Stroke::new(1.5, Color32::WHITE));
            }
            if self.tool == Tool::Mesh {
                for m in &it.mesh {
                    let c = cam.to_scr(m.p);
                    painter.circle(c, 5.0, c32(m.c), Stroke::new(1.5, Color32::WHITE));
                    painter.circle_stroke(c, m.r * self.zoom, Stroke::new(1.0, Color32::from_white_alpha(60)));
                }
            }
            if self.tool == Tool::Distort {
                for c in it.bbox().corners() {
                    painter.circle(cam.to_scr(c), 5.0, Color32::WHITE, blue);
                }
            }
        }
        if self.tool == Tool::Select && self.show_bbox && self.pen.is_none() && let Some((b, hs)) = self.handles() {
            if all.len() > 1 {
                let pts: Vec<Pos2> = b.corners().iter().map(|c| cam.to_scr(*c)).collect();
                let mut c = pts.clone();
                c.push(pts[0]);
                painter.extend(Shape::dashed_line(&c, blue, 4.0, 3.0));
            }
            for h in hs {
                painter.rect(Rect::from_center_size(cam.to_scr(h), Vec2::splat(8.0)), CornerRadius::same(2), Color32::WHITE, Stroke::new(1.2, BLUE), StrokeKind::Inside);
            }
            let s = b.size();
            let label = if all.len() > 1 { format!("{} objets · {:.0} × {:.0}", all.len(), s[0], s[1]) } else { format!("{:.0} × {:.0}", s[0], s[1]) };
            let g = painter.layout_no_wrap(label, FontId::proportional(11.5), Color32::WHITE);
            let bottom = b.corners().iter().map(|c| cam.to_scr(*c)).fold(pos2(0.0, f32::MIN), |a, q| if q.y > a.y { q } else { a });
            let center = cam.to_scr(b.center());
            let tag = Rect::from_center_size(pos2(center.x, bottom.y + 18.0), g.size() + vec2(14.0, 6.0));
            painter.rect_filled(tag, CornerRadius::same(6), BLUE);
            painter.galley(tag.center() - g.size() / 2.0, g, Color32::WHITE);
        }
        if matches!(self.tool, Tool::Rotate | Tool::Reflect | Tool::Scale) && let Some(b) = self.doc.bounds_of(&all) {
            let c = cam.to_scr(self.reference(&b));
            painter.circle_stroke(c, 6.0, Stroke::new(1.5, CYAN));
            painter.line_segment([c - vec2(9.0, 0.0), c + vec2(9.0, 0.0)], Stroke::new(1.0, CYAN));
            painter.line_segment([c - vec2(0.0, 9.0), c + vec2(0.0, 9.0)], Stroke::new(1.0, CYAN));
        }
    }
}

fn point_at(nodes: &[Node], closed: bool, t: f32) -> Option<P> {
    let pts = model::flatten(nodes, closed, 24);
    let mut path = pts.clone();
    if closed && !pts.is_empty() {
        path.push(pts[0]);
    }
    let total = model::polyline_len(&path);
    let target = total * t.clamp(0.0, 1.0);
    let mut acc = 0.0;
    for w in path.windows(2) {
        let l = dist(w[0], w[1]);
        if acc + l >= target {
            return Some(model::lerp(w[0], w[1], ((target - acc) / l.max(1e-6)).clamp(0.0, 1.0)));
        }
        acc += l;
    }
    path.last().copied()
}

fn safe_div(a: f32, b: f32) -> f32 {
    if b.abs() < 1e-3 { 1.0 } else { a / b }
}
