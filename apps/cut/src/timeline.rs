use crate::inspect::draw_keys;
use crate::model::{Clip, LABELS, MARKER_COLORS, Project, TRANSITIONS};
use crate::{Cut, Drag, G, GROUPS, MARK, MUSIC, PLAYHEAD, Sel, TITLE, Tool, glyph, speaker, tc, tool_glyph, waveform};
use eframe::egui::{self, Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Painter, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use studio_ui::pro::{self, CHROME, FIELD, INK, LINE, MUTED, PANEL, RED, WELL, semibold};

const OVER: Color32 = Color32::from_rgb(90, 200, 250);

impl Cut {
    fn draw_clip(&self, p: &Painter, r: Rect, c: &Clip, on: bool, base: Color32) {
        let col = if c.disabled { Color32::from_gray(90) } else { base };
        p.rect_filled(r, CornerRadius::same(6), col.gamma_multiply(if on { 0.7 } else { 0.45 }));
        let clip_p = p.with_clip_rect(r.shrink(1.0));
        let thumb_h = if self.show_waves && !c.mute { r.height() * 0.62 } else { r.height() };
        if self.show_thumbs
            && let Some(t) = self.thumbs.get(&c.src)
        {
            let [tw, th] = t.size();
            let tw_w = thumb_h * tw as f32 / th as f32;
            let mut x = r.left();
            while x < r.right() && x < r.left() + tw_w * 8.0 {
                clip_p.image(t.id(), Rect::from_min_size(pos2(x, r.top()), vec2(tw_w, thumb_h)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::from_white_alpha(if c.hold > 0.0 || c.disabled { 110 } else { 200 }));
                x += tw_w;
                if c.hold > 0.0 {
                    break;
                }
            }
        }
        if self.show_waves
            && !c.mute
            && c.hold <= 0.0
            && let Some(data) = self.audio_data.get(&c.src)
        {
            let wave = Rect::from_min_max(pos2(r.left(), r.top() + thumb_h + 2.0), r.max);
            waveform(&clip_p, wave, data, c.inp, c.out, Color32::from_white_alpha(170));
        }
        let mut badge = String::new();
        if (c.speed - 1.0).abs() > 1e-3 && c.hold <= 0.0 {
            badge.push_str(&format!("  {:.0} %", c.speed * 100.0));
        }
        if c.reverse {
            badge.push_str("  ◀");
        }
        if c.hold > 0.0 {
            badge.push_str("  ❄");
        }
        if c.graded() || c.styled() {
            badge.push_str("  fx");
        }
        if c.disabled {
            badge.push_str("  (désactivé)");
        }
        let name = if c.name.is_empty() { self.proj.sources[c.src].name() } else { c.name.clone() };
        let name_bg = Rect::from_min_size(r.left_top() + vec2(4.0, 4.0), vec2((r.width() - 8.0).clamp(0.0, 260.0), 18.0));
        clip_p.rect_filled(name_bg, CornerRadius::same(4), Color32::from_black_alpha(110));
        clip_p.text(name_bg.left_center() + vec2(6.0, 0.0), Align2::LEFT_CENTER, format!("{name}{badge}"), semibold(11.0), Color32::WHITE);
        if c.fade_in > 0.0 {
            let fw = (c.fade_in as f32 * self.pps).min(r.width());
            clip_p.add(Shape::convex_polygon(vec![r.left_top(), pos2(r.left() + fw, r.top()), r.left_bottom()], Color32::from_black_alpha(90), Stroke::NONE));
        }
        if c.fade_out > 0.0 {
            let fw = (c.fade_out as f32 * self.pps).min(r.width());
            clip_p.add(Shape::convex_polygon(vec![r.right_top(), r.right_bottom(), pos2(r.right() - fw, r.top())], Color32::from_black_alpha(90), Stroke::NONE));
        }
        draw_keys(&clip_p, r, c, self.pps);
        p.rect_stroke(r, CornerRadius::same(6), Stroke::new(if on { 2.0 } else { 1.0 }, if on { Color32::WHITE } else { col }), StrokeKind::Inside);
    }

    fn clip_mode(&self, s: Sel, edge: u8) -> Option<u8> {
        let v1 = matches!(s, Sel::Clip(_));
        Some(match (self.tool, edge) {
            (Tool::Blade | Tool::BladeAll, _) => return None,
            (Tool::Slip, _) => 3,
            (Tool::Slide, _) if v1 => 7,
            (Tool::Rate, 2) => 4,
            (Tool::Rate, 1) => 5,
            (Tool::Ripple, 1) if v1 => 10,
            (Tool::Ripple, 2) if v1 => 11,
            (Tool::Rolling, 1 | 2) if v1 => 6,
            (Tool::TrackFwd, 0) if v1 => 8,
            (Tool::TrackBack, 0) if v1 => 9,
            (_, e) => e,
        })
    }

    fn track_head(&mut self, ui: &mut Ui, p: &Painter, head: Rect, id: &str, label: &str, track: usize) {
        p.rect_filled(head, CornerRadius::same(6), CHROME);
        p.text(head.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, id, semibold(11.5), MUTED);
        if head.height() > 34.0 {
            p.text(head.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, label, FontId::proportional(11.0), crate::FAINT);
        }
        let mut x = head.right() - 14.0;
        let mut button = |ui: &mut Ui, kind: u8, on: bool, tip: &str| -> bool {
            let r = Rect::from_center_size(pos2(x, head.center().y), Vec2::splat(22.0));
            x -= 23.0;
            let resp = ui.interact(r, ui.make_persistent_id(("trk", track, kind)), Sense::click()).on_hover_text(tip);
            if on {
                p.rect_filled(r, CornerRadius::same(5), if kind == 3 { Color32::from_rgb(200, 150, 0) } else { FIELD });
            } else if resp.hovered() {
                p.rect_filled(r, CornerRadius::same(5), FIELD);
            }
            let ink = if on && kind != 3 { RED } else if resp.hovered() { INK } else { MUTED };
            match kind {
                0 => glyph(p, r.shrink(6.0), G::Eye, if on { RED } else { ink }),
                1 => glyph(p, r.shrink(6.0), G::Lock, if on { Color32::from_rgb(255, 196, 0) } else { ink }),
                2 => speaker(p, r.center(), on, ink),
                _ => {
                    p.text(r.center(), Align2::CENTER_CENTER, "S", semibold(11.0), if on { Color32::BLACK } else { ink });
                }
            }
            resp.clicked()
        };
        let solo_bit = match track {
            1 => 2u8,
            2 => 1,
            3 => 4,
            _ => 0,
        };
        if solo_bit > 0 && button(ui, 3, self.proj.solo & solo_bit != 0, "Solo : n'écouter que cette piste") {
            self.proj.solo ^= solo_bit;
        }
        if solo_bit > 0 {
            let muted = match track {
                1 => self.proj.mute_v2,
                2 => self.proj.mute_v1,
                _ => self.proj.mute_a2,
            };
            if button(ui, 2, muted, if muted { "Réactiver le son de la piste" } else { "Couper le son de la piste" }) {
                match track {
                    1 => self.proj.mute_v2 = !muted,
                    2 => self.proj.mute_v1 = !muted,
                    _ => self.proj.mute_a2 = !muted,
                }
            }
        }
        if button(ui, 1, self.proj.lock[track], "Verrouiller la piste") {
            self.proj.lock[track] = !self.proj.lock[track];
        }
        if track < 3 && button(ui, 0, self.proj.hide[track], "Masquer la piste dans le programme") {
            self.proj.hide[track] = !self.proj.hide[track];
        }
    }

    fn apply_drag(&mut self, d: f64, snap_tol: f64) {
        let Some(dr) = self.drag.clone() else { return };
        let mut p: Project = dr.orig.clone();
        let snap = |me: &Cut, t: f64| me.snap_time(t, snap_tol);
        match dr.sel {
            Sel::Clip(i) if i < p.clips.len() => {
                let max = p.sources[p.clips[i].src].duration;
                let olen = p.clips[i].len();
                let end0 = dr.start0 + olen;
                match dr.mode {
                    1 => {
                        let gap = p.clips[i].gap;
                        let applied = p.clips[i].set_start(d.max(-gap), max);
                        p.clips[i].gap = (gap + applied).max(0.0);
                    }
                    2 => {
                        let dd = snap(self, end0 + d) - end0;
                        let next_gap = p.clips.get(i + 1).map(|n| n.gap);
                        let applied = p.clips[i].set_end(next_gap.map_or(dd, |g| dd.min(g)), max);
                        if let Some(n) = p.clips.get_mut(i + 1) {
                            n.gap = (n.gap - applied).max(0.0);
                        }
                    }
                    3 => {
                        let c = &mut p.clips[i];
                        let len = c.out - c.inp;
                        let inp = (c.inp - d * c.speed * if c.reverse { -1.0 } else { 1.0 }).clamp(0.0, (max - len).max(0.0));
                        (c.inp, c.out) = (inp, inp + len);
                    }
                    4 => {
                        let new_len = (snap(self, end0 + d) - dr.start0).max(0.1);
                        let c = &mut p.clips[i];
                        if c.hold > 0.0 {
                            c.hold = new_len;
                        } else {
                            c.speed = ((c.out - c.inp) / new_len).clamp(0.1, 8.0);
                        }
                    }
                    5 => {
                        let gap = p.clips[i].gap;
                        let new_len = (olen - d).clamp(0.1, olen + gap);
                        let c = &mut p.clips[i];
                        if c.hold > 0.0 {
                            c.hold = new_len;
                        } else {
                            c.speed = ((c.out - c.inp) / new_len).clamp(0.1, 8.0);
                        }
                        let real = p.clips[i].len();
                        p.clips[i].gap = (gap + olen - real).max(0.0);
                    }
                    6 => {
                        let dd = snap(self, end0 + d) - end0;
                        p.roll(i, dd);
                    }
                    7 => p.slide(i, d),
                    8 => {
                        let gap = p.clips[i].gap;
                        let ng = (gap + d).max(0.0);
                        p.clips[i].gap = ng;
                        p.shift_after(dr.start0, ng - gap);
                    }
                    9 => {
                        let lo = -p.clips[0].gap;
                        let hi = p.clips.get(i + 1).map_or(f64::INFINITY, |n| n.gap);
                        let dd = d.clamp(lo, hi.max(lo));
                        p.clips[0].gap += dd;
                        if let Some(n) = p.clips.get_mut(i + 1) {
                            n.gap -= dd;
                        }
                        for m in &mut p.music {
                            if m.end() <= end0 + 1e-6 {
                                m.start = (m.start + dd).max(0.0);
                            }
                        }
                        for t in &mut p.titles {
                            if t.start + t.dur <= end0 + 1e-6 {
                                t.start = (t.start + dd).max(0.0);
                            }
                        }
                        for o in &mut p.overlays {
                            if o.start + o.len() <= end0 + 1e-6 {
                                o.start = (o.start + dd).max(0.0);
                            }
                        }
                    }
                    10 => {
                        let applied = p.clips[i].set_start(d, max);
                        p.shift_after(end0, -applied);
                    }
                    11 => {
                        let dd = snap(self, end0 + d) - end0;
                        let applied = p.clips[i].set_end(dd, max);
                        p.shift_after(end0, applied);
                    }
                    _ => {}
                }
            }
            Sel::Over(i) if i < p.overlays.len() => {
                let max = p.sources[p.overlays[i].src].duration;
                let o = &mut p.overlays[i];
                let end0 = dr.start0 + o.len();
                match dr.mode {
                    0 => o.start = snap(self, dr.start0 + d).max(0.0),
                    1 => {
                        let applied = o.set_start(d.max(-dr.start0), max);
                        o.start = dr.start0 + applied;
                    }
                    2 => {
                        let dd = snap(self, end0 + d) - end0;
                        o.set_end(dd, max);
                    }
                    3 => {
                        let len = o.out - o.inp;
                        let inp = (o.inp - d * o.speed).clamp(0.0, (max - len).max(0.0));
                        (o.inp, o.out) = (inp, inp + len);
                    }
                    4 | 5 => {
                        let new_len = if dr.mode == 4 { snap(self, end0 + d) - dr.start0 } else { end0 - (dr.start0 + d) }.max(0.1);
                        if o.hold > 0.0 {
                            o.hold = new_len;
                        } else {
                            o.speed = ((o.out - o.inp) / new_len).clamp(0.1, 8.0);
                        }
                        if dr.mode == 5 {
                            o.start = end0 - o.len();
                        }
                    }
                    _ => {}
                }
            }
            Sel::Music(i) if i < p.music.len() => {
                let max = p.sources[p.music[i].src].duration;
                let m = &mut p.music[i];
                match dr.mode {
                    1 => {
                        let dd = d.clamp(-m.inp.min(dr.start0), m.out - m.inp - 0.1);
                        (m.inp, m.start) = (m.inp + dd, dr.start0 + dd);
                    }
                    2 => {
                        let end0 = m.end();
                        let dd = snap(self, end0 + d) - end0;
                        m.out = (m.out + dd).clamp(m.inp + 0.1, max);
                    }
                    _ => m.start = snap(self, dr.start0 + d).max(0.0),
                }
            }
            Sel::Title(i) if i < p.titles.len() => {
                let t = &mut p.titles[i];
                match dr.mode {
                    2 => {
                        let end = snap(self, dr.start0 + t.dur + d);
                        t.dur = (end - dr.start0).max(0.2);
                    }
                    1 => {
                        let dd = d.clamp(-dr.start0, t.dur - 0.2);
                        (t.start, t.dur) = (dr.start0 + dd, t.dur - dd);
                    }
                    _ => t.start = snap(self, dr.start0 + d).max(0.0),
                }
            }
            _ => {}
        }
        self.proj = p;
    }

    fn begin_drag(&mut self, sel: Sel, mode: u8, from: f64, start0: f64) {
        self.drag = Some(Drag { sel, mode, from, orig: self.proj.clone(), start0 });
    }

    fn timeline_head(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (g, (_, tools)) in GROUPS.iter().enumerate() {
                let mut t = self.tool;
                if pro::tool_group(ui, 30.0, &mut t, tools, |p, r, c, tool| tool_glyph(p, r, c, tool)) {
                    self.set_tool(t);
                }
                if tools.iter().any(|x| x.0 == self.tool) {
                    self.group_last[g] = tools.iter().position(|x| x.0 == self.tool).unwrap_or(0);
                }
            }
            pro::toolbar_sep(ui);
            let actions: [(G, &str, u8); 6] = [(G::Split, "Scinder à la tête de lecture (S)", 0), (G::Trash, "Supprimer la sélection (Suppr)", 1), (G::Freeze, "Figer l'image 2 s (Ctrl+Maj+F)", 2), (G::Title, "Nouveau titre (Ctrl+T)", 3), (G::Marker, "Ajouter un marqueur (M)", 4), (G::Gap, "Supprimer tous les espaces vides", 5)];
            for (g, tip, k) in actions {
                if pro::icon_button(ui, 30.0, false, tip, |p, r, c| glyph(p, r, g, c)).clicked() {
                    match k {
                        0 => {
                            self.proj.split(self.t);
                        }
                        1 => self.delete_sel(),
                        2 => self.freeze_frame(),
                        3 => self.add_title_at(0.0, 0.0, false),
                        4 => self.add_marker(),
                        _ => self.proj.close_gaps(),
                    }
                }
            }
            pro::toolbar_sep(ui);
            if pro::icon_button(ui, 30.0, self.snap, "Magnétisme (Maj+S)", |p, r, c| glyph(p, r, G::Magnet, c)).clicked() {
                self.snap = !self.snap;
            }
            if pro::icon_button(ui, 30.0, self.show_thumbs, "Vignettes", |p, r, c| glyph(p, r, G::Film, c)).clicked() {
                self.show_thumbs = !self.show_thumbs;
            }
            if pro::icon_button(ui, 30.0, self.show_waves, "Formes d'onde", |p, r, c| glyph(p, r, G::Wave, c)).clicked() {
                self.show_waves = !self.show_waves;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if pro::icon_button(ui, 28.0, false, "Ajuster à la fenêtre (\\)", |p, r, c| glyph(p, r, G::Fit, c)).clicked() {
                    self.pps = 0.0;
                }
                if pro::icon_button(ui, 28.0, false, "Zoom avant (+)", |p, r, c| glyph(p, r, G::Plus, c)).clicked() {
                    self.pps = (self.pps * 1.4).min(600.0);
                }
                ui.spacing_mut().slider_width = 110.0;
                ui.add(egui::Slider::new(&mut self.pps, 2.0..=600.0).logarithmic(true).show_value(false));
                if pro::icon_button(ui, 28.0, false, "Zoom arrière (−)", |p, r, c| glyph(p, r, G::Minus, c)).clicked() {
                    self.pps = (self.pps / 1.4).max(2.0);
                }
            });
        });
    }

    pub(crate) fn timeline(&mut self, root: &mut Ui) {
        egui::Panel::bottom("timeline").resizable(true).default_size(340.0).min_size(260.0).frame(Frame::new().fill(PANEL).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let head_h = 42.0;
            let full = ui.max_rect();
            let head = Rect::from_min_size(full.min, vec2(full.width(), head_h));
            ui.painter().rect_filled(head, CornerRadius::ZERO, CHROME);
            ui.painter().line_segment([head.left_bottom(), head.right_bottom()], Stroke::new(1.0, LINE));
            ui.scope_builder(egui::UiBuilder::new().max_rect(head.shrink2(vec2(10.0, 6.0))), |ui| self.timeline_head(ui));

            let label_w = 176.0;
            let body = Rect::from_min_max(pos2(full.left(), head.bottom()), full.max);
            if self.pps <= 0.0 {
                self.pps = ((body.width() - label_w - 40.0) / self.proj.duration().max(5.0) as f32).clamp(2.0, 600.0);
                self.force_scroll = Some(0.0);
            }
            let dur = self.proj.duration().max(10.0) + 8.0;
            let ts = self.track_scale;
            let mut sa = egui::ScrollArea::horizontal().auto_shrink(false);
            if let Some(x) = self.force_scroll.take() {
                sa = sa.horizontal_scroll_offset(x.max(0.0));
            }
            let out = ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| sa.show(ui, |ui| self.tracks(ui, dur, ts, label_w)));
            self.tl_scroll = out.inner.state.clamped_offset().x;
        });
    }

    fn tracks(&mut self, ui: &mut Ui, dur: f64, ts: f32, label_w: f32) {
        let width = (dur as f32 * self.pps).max(ui.available_width() - label_w);
        let (rect, _) = ui.allocate_exact_size(vec2(width + label_w, ui.available_height().max(200.0)), Sense::hover());
        let x0 = rect.min.x + label_w;
        let pps = self.pps;
        let x_of = |t: f64| x0 + t as f32 * pps;
        let t_of = |x: f32| ((x - x0) / pps).max(0.0) as f64;
        let ruler = Rect::from_min_max(pos2(x0, rect.min.y), pos2(rect.max.x, rect.min.y + 26.0));
        let t1 = Rect::from_min_max(pos2(x0, ruler.max.y + 5.0), pos2(rect.max.x, ruler.max.y + 5.0 + 28.0 * ts));
        let v2 = Rect::from_min_max(pos2(x0, t1.max.y + 4.0), pos2(rect.max.x, t1.max.y + 4.0 + 52.0 * ts));
        let v1 = Rect::from_min_max(pos2(x0, v2.max.y + 4.0), pos2(rect.max.x, v2.max.y + 4.0 + 72.0 * ts));
        let a2 = Rect::from_min_max(pos2(x0, v1.max.y + 4.0), pos2(rect.max.x, v1.max.y + 4.0 + 46.0 * ts));
        let p = ui.painter_at(rect);
        p.rect_filled(ruler, CornerRadius::ZERO, WELL);
        if self.mark_in.is_some() || self.mark_out.is_some() {
            let (a, b) = (self.mark_in.unwrap_or(0.0), self.mark_out.unwrap_or(self.proj.duration()));
            let r = Rect::from_min_max(pos2(x_of(a.min(b)), ruler.min.y), pos2(x_of(a.max(b)), a2.max.y));
            p.rect_filled(r, CornerRadius::ZERO, MARK.gamma_multiply(0.06));
            p.rect_filled(Rect::from_min_max(pos2(r.left(), ruler.bottom() - 4.0), pos2(r.right(), ruler.bottom())), CornerRadius::ZERO, MARK.gamma_multiply(0.7));
        }
        let step = [0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0].into_iter().find(|s| *s as f32 * pps >= 80.0).unwrap_or(600.0);
        let minor = step / 5.0;
        let mut k = 0u64;
        while k as f64 * minor <= dur {
            let tt = k as f64 * minor;
            let x = x_of(tt);
            let major = k % 5 == 0;
            k += 1;
            p.line_segment([pos2(x, ruler.max.y - if major { 10.0 } else { 4.0 }), pos2(x, ruler.max.y)], Stroke::new(1.0, if major { MUTED } else { crate::FAINT }));
            if major {
                p.text(pos2(x + 4.0, ruler.min.y + 4.0), Align2::LEFT_TOP, tc(tt, self.proj.fps), FontId::monospace(10.0), MUTED);
            }
        }
        for (r, id, label, track) in [(t1, "T1", "Titres", 0usize), (v2, "V2", "Incrustations", 1), (v1, "V1", "Vidéo", 2), (a2, "A2", "Musique", 3)] {
            p.rect_filled(r, CornerRadius::same(6), Color32::from_rgb(0x19, 0x19, 0x1c));
            if self.proj.lock[track] {
                let mut x = r.left() - (r.height());
                while x < r.right() {
                    p.line_segment([pos2(x, r.bottom()), pos2(x + r.height(), r.top())], Stroke::new(1.0, Color32::from_white_alpha(8)));
                    x += 10.0;
                }
            }
            let head_r = Rect::from_min_max(pos2(rect.min.x + 6.0, r.min.y), pos2(x0 - 6.0, r.max.y));
            self.track_head(ui, &p, head_r, id, label, track);
        }
        let rr = ui.interact(ruler, ui.make_persistent_id("ruler"), Sense::click_and_drag());
        if let Some(pp) = rr.interact_pointer_pos() {
            let t = t_of(pp.x);
            let t = if ui.input(|i| i.modifiers.shift) { self.snap_time(t, 8.0 / pps as f64) } else { t };
            self.seek(t);
        }
        let snap_tol = 8.0 / pps as f64;
        let mods = ui.input(|i| i.modifiers);
        let mut clicked_sel: Option<Sel> = None;
        let mut new_drag: Option<(Sel, u8, f64, f64)> = None;
        let mut blade: Option<(f64, bool)> = None;
        let mut menu: Option<(Sel, u8)> = None;

        let hover_edge = |r: Rect, hp: egui::Pos2| if hp.x - r.min.x < 8.0 { 1 } else if r.max.x - hp.x < 8.0 { 2 } else { 0 };

        for (i, tt) in self.proj.titles.iter().enumerate() {
            let r = Rect::from_min_max(pos2(x_of(tt.start) + 1.0, t1.min.y + 3.0), pos2(x_of(tt.start + tt.dur) - 1.0, t1.max.y - 3.0));
            let on = self.is_sel(Sel::Title(i));
            p.rect_filled(r, CornerRadius::same(5), TITLE.gamma_multiply(if on { 0.85 } else { 0.55 }));
            glyph(&p.with_clip_rect(r), Rect::from_min_size(r.left_center() + vec2(6.0, -6.0), Vec2::splat(12.0)), G::Title, Color32::WHITE);
            p.with_clip_rect(r.shrink(1.0)).text(r.left_center() + vec2(22.0, 0.0), Align2::LEFT_CENTER, tt.text.lines().next().unwrap_or(""), semibold(11.0), Color32::WHITE);
            if on {
                p.rect_stroke(r, CornerRadius::same(5), Stroke::new(2.0, Color32::WHITE), StrokeKind::Inside);
            }
            if self.proj.lock[0] || matches!(self.tool, Tool::Hand | Tool::Zoom) {
                continue;
            }
            let resp = ui.interact(r, ui.make_persistent_id(("title", i)), Sense::click_and_drag());
            if matches!(self.tool, Tool::Blade | Tool::BladeAll) {
                if resp.clicked()
                    && let Some(pp) = resp.interact_pointer_pos()
                {
                    blade = Some((self.snap_time(t_of(pp.x), snap_tol), true));
                }
                continue;
            }
            if resp.clicked() {
                clicked_sel = Some(Sel::Title(i));
            }
            if resp.drag_started()
                && let Some(pp) = resp.interact_pointer_pos()
            {
                new_drag = Some((Sel::Title(i), hover_edge(r, pp), t_of(pp.x), tt.start));
            }
            if resp.hover_pos().is_some_and(|hp| hover_edge(r, hp) > 0) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if resp.secondary_clicked() {
                clicked_sel = Some(Sel::Title(i));
            }
            resp.context_menu(|ui| {
                if ui.button("Dupliquer").clicked() {
                    menu = Some((Sel::Title(i), 1));
                    ui.close();
                }
                if ui.button("Placer à la tête de lecture").clicked() {
                    menu = Some((Sel::Title(i), 4));
                    ui.close();
                }
                if ui.button("Supprimer").clicked() {
                    menu = Some((Sel::Title(i), 0));
                    ui.close();
                }
            });
        }

        let starts = self.proj.starts();
        let mut prev_end = 0.0;
        for (i, c) in self.proj.clips.iter().enumerate() {
            if c.gap > 1e-6 {
                let g = Rect::from_min_max(pos2(x_of(prev_end) + 1.0, v1.min.y + 3.0), pos2(x_of(starts[i]) - 1.0, v1.max.y - 3.0));
                let on = self.is_sel(Sel::Gap(i));
                if on {
                    p.rect_filled(g, CornerRadius::same(6), Color32::from_white_alpha(18));
                    p.rect_stroke(g, CornerRadius::same(6), Stroke::new(1.5, Color32::WHITE), StrokeKind::Inside);
                }
                if g.width() > 6.0 && !self.proj.lock[2] {
                    let resp = ui.interact(g, ui.make_persistent_id(("gap", i)), Sense::click()).on_hover_text(format!("Espace vide de {} — Suppr pour le refermer", tc(c.gap, self.proj.fps)));
                    if resp.clicked() && matches!(self.tool, Tool::Select | Tool::Ripple) {
                        clicked_sel = Some(Sel::Gap(i));
                    }
                }
            }
            prev_end = starts[i] + c.len();
            let r = Rect::from_min_max(pos2(x_of(starts[i]) + 1.0, v1.min.y + 3.0), pos2(x_of(starts[i] + c.len()) - 1.0, v1.max.y - 3.0));
            let lc = LABELS[c.label as usize % LABELS.len()].1;
            self.draw_clip(&p, r, c, self.is_sel(Sel::Clip(i)), Color32::from_rgb(lc[0], lc[1], lc[2]));
            if c.trans > 0 && i > 0 && c.gap <= 1e-6 {
                let d = c.trans_dur as f32 * pps;
                let (a, b) = match c.trans_align {
                    1 => (0.0, d),
                    2 => (d, 0.0),
                    _ => (d / 2.0, d / 2.0),
                };
                let tr = Rect::from_min_max(pos2(r.left() - a, r.top()), pos2(r.left() + b, r.top() + 14.0));
                p.rect_filled(tr, CornerRadius::same(3), Color32::from_rgb(120, 110, 230));
                p.with_clip_rect(tr).text(tr.center(), Align2::CENTER_CENTER, TRANSITIONS[c.trans as usize % TRANSITIONS.len()], FontId::proportional(9.5), Color32::WHITE);
            }
        }
        for (i, c) in self.proj.overlays.iter().enumerate() {
            let r = Rect::from_min_max(pos2(x_of(c.start) + 1.0, v2.min.y + 3.0), pos2(x_of(c.start + c.len()) - 1.0, v2.max.y - 3.0));
            self.draw_clip(&p, r, c, self.is_sel(Sel::Over(i)), OVER);
        }
        let v1_items: Vec<(Sel, Rect, f64, bool)> = self.proj.clips.iter().enumerate().map(|(i, c)| (Sel::Clip(i), Rect::from_min_max(pos2(x_of(starts[i]) + 1.0, v1.min.y + 3.0), pos2(x_of(starts[i] + c.len()) - 1.0, v1.max.y - 3.0)), starts[i], self.proj.lock[2])).chain(self.proj.overlays.iter().enumerate().map(|(i, c)| (Sel::Over(i), Rect::from_min_max(pos2(x_of(c.start) + 1.0, v2.min.y + 3.0), pos2(x_of(c.start + c.len()) - 1.0, v2.max.y - 3.0)), c.start, self.proj.lock[1]))).collect();
        for (s, r, start, locked) in v1_items {
            if locked || matches!(self.tool, Tool::Hand | Tool::Zoom) {
                continue;
            }
            let id = match s {
                Sel::Clip(i) => ("clip", i),
                Sel::Over(i) => ("over", i),
                _ => continue,
            };
            let resp = ui.interact(r, ui.make_persistent_id(id), Sense::click_and_drag());
            if matches!(self.tool, Tool::Blade | Tool::BladeAll) {
                if let Some(hp) = resp.hover_pos() {
                    let (top, bottom) = if self.tool == Tool::BladeAll { (t1.min.y, a2.max.y) } else { (r.min.y, r.max.y) };
                    p.line_segment([pos2(hp.x, top), pos2(hp.x, bottom)], Stroke::new(1.5, Color32::WHITE));
                }
                if resp.clicked()
                    && let Some(pp) = resp.interact_pointer_pos()
                {
                    let t = self.snap_time(t_of(pp.x), snap_tol);
                    blade = Some((t, matches!(s, Sel::Over(_))));
                }
            } else {
                if resp.clicked() {
                    clicked_sel = Some(s);
                }
                if resp.drag_started()
                    && let Some(pp) = resp.interact_pointer_pos()
                {
                    let edge = hover_edge(r, pp);
                    if let Some(mut mode) = self.clip_mode(s, edge) {
                        let mut sel = s;
                        let mut s0 = start;
                        if mode == 6
                            && let Sel::Clip(i) = s
                        {
                            if edge == 1 {
                                if i == 0 {
                                    mode = 1;
                                } else {
                                    sel = Sel::Clip(i - 1);
                                    s0 = starts[i - 1];
                                }
                            }
                        }
                        new_drag = Some((sel, mode, t_of(pp.x), s0));
                    }
                }
                if let Some(hp) = resp.hover_pos() {
                    let edge = hover_edge(r, hp);
                    match self.tool {
                        Tool::Slip | Tool::Slide => ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn),
                        _ if edge > 0 => ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal),
                        _ => {}
                    }
                }
            }
            if resp.secondary_clicked() {
                clicked_sel = Some(s);
            }
            resp.context_menu(|ui| self.clip_menu(ui, s, &mut menu));
        }
        for (i, m) in self.proj.music.iter().enumerate() {
            let r = Rect::from_min_max(pos2(x_of(m.start) + 1.0, a2.min.y + 3.0), pos2(x_of(m.end()) - 1.0, a2.max.y - 3.0));
            let on = self.is_sel(Sel::Music(i));
            let lc = LABELS[m.label as usize % LABELS.len()].1;
            let col = if m.label == 0 { MUSIC } else { Color32::from_rgb(lc[0], lc[1], lc[2]) };
            p.rect_filled(r, CornerRadius::same(6), col.gamma_multiply(if on { 0.7 } else { 0.45 }));
            let clip_p = p.with_clip_rect(r.shrink(1.0));
            if self.show_waves
                && !m.mute
                && let Some(data) = self.audio_data.get(&m.src)
            {
                waveform(&clip_p, r.shrink2(vec2(0.0, 4.0)), data, m.inp, m.out, Color32::from_white_alpha(170));
            }
            clip_p.text(r.left_top() + vec2(8.0, 3.0), Align2::LEFT_TOP, format!("{}{}", self.proj.sources[m.src].name(), if m.mute { "  (coupé)" } else { "" }), semibold(11.0), Color32::WHITE);
            for (f, left) in [(m.fade_in, true), (m.fade_out, false)] {
                if f > 0.0 {
                    let fw = (f as f32 * pps).min(r.width());
                    let pts = if left { vec![r.left_top(), pos2(r.left() + fw, r.top()), r.left_bottom()] } else { vec![r.right_top(), r.right_bottom(), pos2(r.right() - fw, r.top())] };
                    clip_p.add(Shape::convex_polygon(pts, Color32::from_black_alpha(90), Stroke::NONE));
                }
            }
            p.rect_stroke(r, CornerRadius::same(6), Stroke::new(if on { 2.0 } else { 1.0 }, if on { Color32::WHITE } else { col }), StrokeKind::Inside);
            if self.proj.lock[3] || matches!(self.tool, Tool::Hand | Tool::Zoom) {
                continue;
            }
            let resp = ui.interact(r, ui.make_persistent_id(("music", i)), Sense::click_and_drag());
            if matches!(self.tool, Tool::Blade | Tool::BladeAll) {
                if resp.clicked()
                    && let Some(pp) = resp.interact_pointer_pos()
                {
                    blade = Some((self.snap_time(t_of(pp.x), snap_tol), true));
                }
                continue;
            }
            if resp.clicked() || resp.secondary_clicked() {
                clicked_sel = Some(Sel::Music(i));
            }
            if resp.drag_started()
                && let Some(pp) = resp.interact_pointer_pos()
            {
                new_drag = Some((Sel::Music(i), hover_edge(r, pp), t_of(pp.x), m.start));
            }
            if resp.hover_pos().is_some_and(|hp| hover_edge(r, hp) > 0) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            resp.context_menu(|ui| {
                if ui.button("Placer à la tête de lecture").clicked() {
                    menu = Some((Sel::Music(i), 4));
                    ui.close();
                }
                if ui.button("Couper / réactiver le son").clicked() {
                    menu = Some((Sel::Music(i), 5));
                    ui.close();
                }
                if ui.button("Normaliser (crête −1 dB)").clicked() {
                    menu = Some((Sel::Music(i), 6));
                    ui.close();
                }
                if ui.button("Dupliquer").clicked() {
                    menu = Some((Sel::Music(i), 1));
                    ui.close();
                }
                ui.separator();
                if ui.button("Supprimer").clicked() {
                    menu = Some((Sel::Music(i), 0));
                    ui.close();
                }
            });
        }

        if let Some((t, all)) = blade {
            if self.tool == Tool::BladeAll {
                self.proj.split_all(t);
            } else if all {
                let mut q = self.proj.clone();
                q.split_all(t);
                (self.proj.titles, self.proj.music, self.proj.overlays) = (q.titles, q.music, q.overlays);
            } else {
                self.proj.split(t);
            }
        }
        if let Some(s) = clicked_sel {
            self.click_select(s, mods.command, mods.shift);
        }
        if let Some((s, act)) = menu {
            self.menu_action(s, act);
        }
        if let Some((s, mode, from, s0)) = new_drag {
            self.click_select(s, false, false);
            self.begin_drag(s, mode, from, s0);
        }

        if let Some(dr) = self.drag.clone() {
            let (down, pos) = ui.input(|i| (i.pointer.primary_down(), i.pointer.hover_pos()));
            if let (true, Some(pp)) = (down, pos) {
                let d = t_of(pp.x) - dr.from;
                self.apply_drag(d, snap_tol);
                let label = match dr.mode {
                    3 => "Glissement",
                    6 => "Modification par déplacement",
                    7 => "Déplacement",
                    10 | 11 => "Propagation",
                    4 | 5 => "Allongement",
                    8 | 9 => "Sélection de piste",
                    _ => "",
                };
                if dr.mode == 0 && matches!(dr.sel, Sel::Clip(_)) {
                    p.line_segment([pos2(pp.x, v1.min.y), pos2(pp.x, v1.max.y)], Stroke::new(2.0, Color32::WHITE));
                }
                if !label.is_empty() {
                    p.text(pp + vec2(12.0, -18.0), Align2::LEFT_BOTTOM, format!("{label}  {}{}", if d < 0.0 { "−" } else { "+" }, tc(d.abs(), self.proj.fps)), semibold(11.0), Color32::WHITE);
                }
            } else {
                if dr.mode == 0
                    && let (Sel::Clip(i), Some(pp)) = (dr.sel, pos)
                    && i < self.proj.clips.len()
                    && (t_of(pp.x) - dr.from).abs() > 0.1
                {
                    let t = t_of(pp.x);
                    let starts = self.proj.starts();
                    let mut to = starts.iter().zip(&self.proj.clips).filter(|(s, c)| t > **s + c.len() / 2.0).count();
                    let clip = self.proj.clips.remove(i);
                    if to > i {
                        to -= 1;
                    }
                    let to = to.min(self.proj.clips.len());
                    self.proj.clips.insert(to, clip);
                    self.sel = Some(Sel::Clip(to));
                    self.extra.clear();
                }
                self.drag = None;
            }
        }

        for m in &self.proj.markers {
            let mx = x_of(m.t);
            let rgb = MARKER_COLORS[m.color as usize % MARKER_COLORS.len()].1;
            let col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            if m.dur > 0.0 {
                p.rect_filled(Rect::from_min_max(pos2(mx, ruler.min.y + 14.0), pos2(x_of(m.t + m.dur), ruler.max.y)), CornerRadius::same(2), col.gamma_multiply(0.35));
            }
            p.add(Shape::convex_polygon(vec![pos2(mx - 5.0, ruler.min.y + 14.0), pos2(mx + 5.0, ruler.min.y + 14.0), pos2(mx, ruler.max.y)], col, Stroke::NONE));
            p.line_segment([pos2(mx, ruler.max.y), pos2(mx, a2.max.y)], Stroke::new(1.0, col.gamma_multiply(0.35)));
            if !m.name.is_empty() && pps > 20.0 {
                p.text(pos2(mx + 7.0, ruler.min.y + 13.0), Align2::LEFT_CENTER, &m.name, FontId::proportional(10.0), col);
            }
        }
        for (m, label) in [(self.mark_in, "{"), (self.mark_out, "}")] {
            if let Some(m) = m {
                p.text(pos2(x_of(m), ruler.max.y - 2.0), Align2::CENTER_BOTTOM, label, semibold(13.0), MARK);
            }
        }
        let x = x_of(self.t);
        p.line_segment([pos2(x, ruler.min.y), pos2(x, a2.max.y)], Stroke::new(1.5, PLAYHEAD));
        p.add(Shape::convex_polygon(vec![pos2(x - 7.0, ruler.min.y), pos2(x + 7.0, ruler.min.y), pos2(x + 7.0, ruler.min.y + 8.0), pos2(x, ruler.min.y + 14.0), pos2(x - 7.0, ruler.min.y + 8.0)], PLAYHEAD, Stroke::NONE));
        let bg = ui.interact(Rect::from_min_max(pos2(x0, t1.min.y), pos2(rect.max.x, a2.max.y)), ui.make_persistent_id("tracks"), Sense::click_and_drag());
        match self.tool {
            Tool::Hand => {
                ui.ctx().set_cursor_icon(if bg.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
                if bg.dragged() {
                    self.force_scroll = Some(self.tl_scroll - bg.drag_delta().x);
                }
            }
            Tool::Zoom => {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ZoomIn);
                if bg.clicked()
                    && let Some(pp) = bg.interact_pointer_pos()
                {
                    let at = t_of(pp.x);
                    let f = if ui.input(|i| i.modifiers.alt) { 1.0 / 1.5 } else { 1.5 };
                    self.pps = (self.pps * f).clamp(2.0, 600.0);
                    self.force_scroll = Some(at as f32 * self.pps - (pp.x - rect.min.x - label_w));
                }
            }
            Tool::TrackFwd | Tool::TrackBack => {
                if bg.clicked()
                    && let Some(pp) = bg.interact_pointer_pos()
                {
                    let t = t_of(pp.x);
                    let i = starts.iter().position(|s| *s >= t).unwrap_or(self.proj.clips.len());
                    if self.tool == Tool::TrackFwd && i < self.proj.clips.len() {
                        self.click_select(Sel::Clip(i), false, false);
                    } else if self.tool == Tool::TrackBack && i > 0 {
                        self.click_select(Sel::Clip(i - 1), false, false);
                    }
                }
            }
            _ => {
                if bg.clicked()
                    && let Some(pp) = bg.interact_pointer_pos()
                {
                    self.seek(t_of(pp.x));
                    self.sel = None;
                    self.extra.clear();
                }
            }
        }
        if self.playing {
            let view_right = self.tl_scroll + ui.clip_rect().width() - label_w;
            let px = self.t as f32 * pps;
            if px > view_right - 40.0 || px < self.tl_scroll {
                self.force_scroll = Some((px - 60.0).max(0.0));
            }
        }
    }

    fn clip_menu(&self, ui: &mut Ui, s: Sel, menu: &mut Option<(Sel, u8)>) {
        let item = |ui: &mut Ui, label: &str, code: u8, menu: &mut Option<(Sel, u8)>| {
            if ui.button(label).clicked() {
                *menu = Some((s, code));
                ui.close();
            }
        };
        item(ui, "Scinder à la tête de lecture", 2, menu);
        item(ui, "Dupliquer", 1, menu);
        item(ui, "Copier", 3, menu);
        item(ui, "Figer l'image à la tête", 7, menu);
        item(ui, "Vitesse/Durée…", 8, menu);
        item(ui, "Inverser la lecture", 9, menu);
        item(ui, "Activer / désactiver", 10, menu);
        ui.menu_button("Vitesse", |ui| {
            for (k, v) in ["25 %", "50 %", "75 %", "100 %", "150 %", "200 %", "400 %"].iter().enumerate() {
                if ui.button(*v).clicked() {
                    *menu = Some((s, 100 + k as u8));
                    ui.close();
                }
            }
        });
        if matches!(s, Sel::Clip(i) if i > 0) {
            ui.menu_button("Transition d'entrée", |ui| {
                for (k, n) in TRANSITIONS.iter().enumerate() {
                    if ui.button(*n).clicked() {
                        *menu = Some((s, 120 + k as u8));
                        ui.close();
                    }
                }
            });
        }
        ui.menu_button("Look", |ui| {
            for (k, name) in crate::model::LOOKS.iter().enumerate() {
                if ui.button(*name).clicked() {
                    *menu = Some((s, 160 + k as u8));
                    ui.close();
                }
            }
        });
        ui.menu_button("Effet prédéfini", |ui| {
            for (k, name) in crate::inspect::EFFECT_PRESETS.iter().enumerate() {
                if ui.button(*name).clicked() {
                    *menu = Some((s, 190 + k as u8));
                    ui.close();
                }
            }
        });
        ui.menu_button("Étiquette", |ui| {
            for (k, (name, _)) in LABELS.iter().enumerate() {
                if ui.button(*name).clicked() {
                    *menu = Some((s, 210 + k as u8));
                    ui.close();
                }
            }
        });
        ui.separator();
        item(ui, "Couper / réactiver le son", 5, menu);
        item(ui, "Normaliser l'audio (−1 dB)", 6, menu);
        item(ui, "Détacher l'audio sur A2", 11, menu);
        item(ui, if matches!(s, Sel::Over(_)) { "Ramener sur V1" } else { "Monter sur V2 (à la même place)" }, 12, menu);
        item(ui, "Réinitialiser les effets", 13, menu);
        ui.separator();
        if matches!(s, Sel::Clip(_)) {
            item(ui, "Lever (laisser un vide)", 14, menu);
            item(ui, "Supprimer et refermer", 15, menu);
        } else {
            item(ui, "Supprimer", 0, menu);
        }
    }
}
