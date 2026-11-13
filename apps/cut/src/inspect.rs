use crate::model::{ALIGNS, ANIMS, Clip, FONTS, KPROPS, LABELS, LOOKS, MARKER_COLORS, Mask, ROLLS, TRANSITIONS, Title};
use crate::{Cut, G, Sel, glyph, short, tc};
use eframe::egui::{self, Align, Color32, CornerRadius, Frame, Layout, Margin, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, Vec2, pos2, vec2};
use studio_ui::pro::{self, BLUE, FAINT, INK, INK_STRONG, LINE, MUTED, WELL, semibold};

pub const EFFECT_PRESETS: [&str; 10] = ["Rêve", "VHS", "Surveillance", "Pixel art", "Négatif", "Bande dessinée", "Film ancien", "Netteté +", "Flou d'arrière-plan", "Néon"];
pub const PIP: [&str; 8] = ["Haut gauche", "Haut droite", "Bas gauche", "Bas droite", "Centre", "Plein écran", "Moitié gauche", "Moitié droite"];
pub const TITLE_PRESETS: [&str; 12] = ["Titre cinéma", "Sous-titre", "Tiers inférieur classique", "Tiers inférieur moderne", "Générique défilant", "Bandeau d'info", "Citation", "Chapitre", "Gros titre YouTube", "Néon", "Minimal", "Titre vertical"];

pub fn effect_preset(c: &mut Clip, k: usize) {
    match k {
        0 => (c.blur, c.brightness, c.saturation, c.vignette, c.look) = (18.0, 12.0, -10.0, 20.0, 14),
        1 => (c.grain, c.saturation, c.blur, c.look, c.vignette) = (45.0, 25.0, 6.0, 12, 30.0),
        2 => (c.look, c.grain, c.vignette, c.contrast, c.look_amt) = (1, 60.0, 55.0, 25.0, 1.0),
        3 => (c.pixelate, c.posterize, c.saturation) = (40.0, 6.0, 20.0),
        4 => c.invert = !c.invert,
        5 => (c.posterize, c.sharpen, c.saturation, c.contrast) = (5.0, 70.0, 35.0, 20.0),
        6 => (c.look, c.grain, c.vignette, c.contrast) = (2, 50.0, 60.0, 10.0),
        7 => c.sharpen = 60.0,
        8 => (c.blur, c.zoom) = (60.0, c.zoom.max(1.1)),
        _ => (c.look, c.saturation, c.contrast, c.sharpen, c.vignette) = (15, 60.0, 30.0, 30.0, 40.0),
    }
}

pub fn pip(c: &mut Clip, k: usize) {
    let (z, x, y, crop) = match k {
        0 => (0.33, -0.62, -0.62, [0.0; 4]),
        1 => (0.33, 0.62, -0.62, [0.0; 4]),
        2 => (0.33, -0.62, 0.62, [0.0; 4]),
        3 => (0.33, 0.62, 0.62, [0.0; 4]),
        4 => (0.5, 0.0, 0.0, [0.0; 4]),
        5 => (1.0, 0.0, 0.0, [0.0; 4]),
        6 => (1.0, -0.5, 0.0, [0.25, 0.0, 0.25, 0.0]),
        _ => (1.0, 0.5, 0.0, [0.25, 0.0, 0.25, 0.0]),
    };
    (c.zoom, c.pos_x, c.pos_y, c.crop, c.zoom_end) = (z, x, y, crop, 0.0);
    c.keys.retain(|k| k.prop > 2);
}

pub fn title_preset(t: &mut Title, k: usize) {
    let keep = (t.text.clone(), t.start, t.dur);
    *t = Title { text: keep.0, start: keep.1, dur: keep.2, ..Title::default() };
    match k {
        0 => (t.font, t.size, t.spacing, t.anim, t.place, t.shadow) = (6, 0.1, 0.25, 8, 1, false),
        1 => (t.size, t.place, t.anim, t.shadow, t.outline) = (0.05, 2, 1, true, 25.0),
        2 => (t.place, t.size, t.band, t.anim) = (3, 0.05, true, 5),
        3 => (t.place, t.size, t.band, t.band_color, t.color, t.anim, t.font) = (3, 0.048, true, [10, 132, 255, 230], [255; 4], 5, 0),
        4 => (t.roll, t.size, t.anim, t.place, t.leading) = (1, 0.05, 0, 1, 0.4),
        5 => (t.roll, t.place, t.band, t.size, t.anim) = (2, 2, true, 0.045, 0),
        6 => (t.font, t.size, t.color, t.anim) = (3, 0.06, [235, 225, 200, 255], 3),
        7 => (t.size, t.place, t.align, t.anim, t.font, t.dx) = (0.09, 2, 1, 5, 9, 0.0),
        8 => (t.font, t.size, t.color, t.outline, t.anim, t.shadow) = (2, 0.14, [255, 214, 10, 255], 70.0, 9, true),
        9 => (t.color, t.outline, t.outline_color, t.size, t.anim, t.shadow) = ([255, 255, 255, 255], 35.0, [255, 45, 200, 255], 0.09, 1, false),
        10 => (t.font, t.size, t.shadow, t.spacing, t.anim) = (10, 0.06, false, 0.1, 1),
        _ => (t.vertical, t.size, t.place, t.anim) = (true, 0.06, 1, 2),
    }
}

pub fn row_slider<N: egui::emath::Numeric>(ui: &mut Ui, label: &str, v: &mut N, range: std::ops::RangeInclusive<N>, suffix: &str, log: bool) -> bool {
    let reset = ui.horizontal(|ui| {
        let r = ui.add(egui::Label::new(RichText::new(label).size(12.5).color(MUTED)).sense(Sense::click()));
        r.on_hover_text("Double-clic pour réinitialiser").double_clicked()
    });
    ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(80.0);
    let mut changed = ui.add(egui::Slider::new(v, range.clone()).suffix(suffix).logarithmic(log).max_decimals(2)).changed();
    if reset.inner {
        let zero = N::from_f64(if range.contains(&N::from_f64(0.0)) { 0.0 } else { range.start().to_f64() });
        let one = N::from_f64(1.0);
        *v = if log && range.contains(&one) { one } else { zero };
        changed = true;
    }
    changed
}

pub fn pct_slider(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>) -> bool {
    let mut p = *v * 100.0;
    let ch = row_slider(ui, label, &mut p, range.start() * 100.0..=range.end() * 100.0, " %", false);
    if ch {
        *v = p / 100.0;
    }
    ch
}

pub fn field_time(ui: &mut Ui, label: &str, v: &mut f64, min: f64, max: f64) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(egui::DragValue::new(v).range(min..=max.max(min)).speed(0.02).max_decimals(2).suffix(" s"));
        });
    });
}

fn chips(ui: &mut Ui, items: &[&str], current: Option<usize>) -> Option<usize> {
    let mut hit = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for (k, name) in items.iter().enumerate() {
            if ui.selectable_label(current == Some(k), *name).clicked() {
                hit = Some(k);
            }
        }
    });
    hit
}

fn diamond(p: &egui::Painter, c: Pos2, r: f32, fill: bool, color: Color32) {
    let pts = vec![c + vec2(0.0, -r), c + vec2(r, 0.0), c + vec2(0.0, r), c + vec2(-r, 0.0)];
    p.add(Shape::convex_polygon(pts, if fill { color } else { Color32::TRANSPARENT }, Stroke::new(1.3, color)));
}

fn kf_row(ui: &mut Ui, c: &mut Clip, local: f64, prop: u8, range: std::ops::RangeInclusive<f32>, scale: f32, suffix: &str) -> Option<f64> {
    let keyed = c.has_keys(prop);
    let v = if keyed { c.key_value(prop, local).unwrap_or(c.prop(prop)) } else { c.prop(prop) };
    let at_key = c.keys.iter().position(|k| k.prop == prop && (k.t - local).abs() < 0.02);
    let mut seek = None;
    let mut reset = false;
    ui.horizontal(|ui| {
        let r = ui.add(egui::Label::new(RichText::new(KPROPS[prop as usize]).size(12.5).color(if keyed { BLUE } else { MUTED })).sense(Sense::click()));
        reset = r.on_hover_text("Double-clic pour réinitialiser").double_clicked();
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if keyed {
                let ts: Vec<f64> = c.keys.iter().filter(|k| k.prop == prop).map(|k| k.t).collect();
                if ui.small_button("›").on_hover_text("Image clé suivante").clicked() {
                    seek = ts.iter().copied().filter(|t| *t > local + 0.02).fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.min(t))));
                }
            }
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
            diamond(ui.painter(), r.center(), 5.0, at_key.is_some(), if keyed { BLUE } else if resp.hovered() { INK } else { FAINT });
            if resp.on_hover_text(if at_key.is_some() { "Supprimer l'image clé" } else { "Ajouter une image clé à la tête de lecture" }).clicked() {
                match at_key {
                    Some(k) => {
                        c.keys.remove(k);
                        if !c.has_keys(prop) {
                            c.set_prop(prop, v);
                        }
                    }
                    None => c.set_key(prop, local, v),
                }
            }
            if keyed {
                let ts: Vec<f64> = c.keys.iter().filter(|k| k.prop == prop).map(|k| k.t).collect();
                if ui.small_button("‹").on_hover_text("Image clé précédente").clicked() {
                    seek = ts.iter().copied().filter(|t| *t < local - 0.02).fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.max(t))));
                }
            }
        });
    });
    ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(80.0);
    let mut d = v * scale;
    let changed = ui.add(egui::Slider::new(&mut d, *range.start() * scale..=*range.end() * scale).suffix(suffix).max_decimals(1)).changed();
    let target = if reset { Some(Clip::default().prop(prop)) } else if changed { Some(d / scale) } else { None };
    if let Some(nv) = target {
        if c.has_keys(prop) {
            c.set_key(prop, local, nv);
        } else {
            c.set_prop(prop, nv);
        }
    }
    seek
}

fn wheel(ui: &mut Ui, label: &str, v: &mut [f32; 2], size: f32) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        let (r, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
        let c = r.center();
        let rad = size / 2.0 - 3.0;
        let p = ui.painter();
        let n = 48;
        for k in 0..n {
            let a0 = k as f32 / n as f32 * std::f32::consts::TAU;
            let a1 = (k + 1) as f32 / n as f32 * std::f32::consts::TAU;
            let col: Color32 = egui::ecolor::Hsva::new(((-a0 / std::f32::consts::TAU) + 1.0 + 0.25).rem_euclid(1.0), 0.55, 0.6, 1.0).into();
            p.add(Shape::convex_polygon(vec![c, c + vec2(a0.cos(), a0.sin()) * rad, c + vec2(a1.cos(), a1.sin()) * rad], col, Stroke::NONE));
        }
        p.circle_filled(c, rad * 0.55, Color32::from_black_alpha(90));
        p.circle_stroke(c, rad, Stroke::new(1.0, LINE));
        let puck = c + vec2(v[0], -v[1]) * rad;
        p.circle(puck, 5.0, Color32::WHITE, Stroke::new(1.5, Color32::from_black_alpha(160)));
        if let Some(pp) = resp.interact_pointer_pos()
            && resp.dragged()
        {
            let mut d = (pp - c) / rad;
            if d.length() > 1.0 {
                d /= d.length();
            }
            *v = [d.x, -d.y];
            changed = true;
        }
        if resp.double_clicked() {
            *v = [0.0, 0.0];
            changed = true;
        }
        ui.add_space(2.0);
        ui.label(RichText::new(label).size(11.0).color(MUTED));
    });
    changed
}

fn curve(ui: &mut Ui, off: &mut [f32; 5], color: Color32) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, (w * 0.7).min(170.0)), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(6), WELL);
    for k in 1..4 {
        let f = k as f32 / 4.0;
        p.line_segment([pos2(r.left() + r.width() * f, r.top()), pos2(r.left() + r.width() * f, r.bottom())], Stroke::new(1.0, Color32::from_white_alpha(10)));
        p.line_segment([pos2(r.left(), r.top() + r.height() * f), pos2(r.right(), r.top() + r.height() * f)], Stroke::new(1.0, Color32::from_white_alpha(10)));
    }
    p.line_segment([r.left_bottom(), r.right_top()], Stroke::new(1.0, Color32::from_white_alpha(25)));
    let lut = crate::look::curve_lut(off);
    let pts: Vec<Pos2> = (0..=64).map(|k| {
        let x = k as f32 / 64.0;
        pos2(r.left() + x * r.width(), r.bottom() - lut[((x * 255.0) as usize).min(255)] * r.height())
    }).collect();
    p.add(Shape::line(pts, Stroke::new(2.0, color)));
    for k in 0..5 {
        let x = k as f32 / 4.0;
        let y = (x + off[k]).clamp(0.0, 1.0);
        let c = pos2(r.left() + x * r.width(), r.bottom() - y * r.height());
        let hit = Rect::from_center_size(c, Vec2::splat(16.0));
        let resp = ui.interact(hit, ui.make_persistent_id(("curve-pt", k, color)), Sense::click_and_drag());
        ui.painter().circle(c, if resp.hovered() { 6.0 } else { 5.0 }, Color32::WHITE, Stroke::new(1.5, color));
        if resp.dragged()
            && let Some(pp) = resp.interact_pointer_pos()
        {
            off[k] = (1.0 - (pp.y - r.top()) / r.height()).clamp(0.0, 1.0) - x;
        }
        if resp.double_clicked() {
            off[k] = 0.0;
        }
    }
}

enum Act {
    Seek(f64),
    Detach,
    Normalize(f32),
    ToTrack,
    Speed,
    CopyGrade,
    PasteGrade,
    GradeAll,
}

impl Cut {
    pub(crate) fn inspector(&mut self, root: &mut Ui) {
        egui::Panel::right("inspector").resizable(false).exact_size(316.0).frame(Frame::new().fill(pro::PANEL)).show(root, |ui| {
            let w = ui.available_width();
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.set_max_width(w);
                ui.spacing_mut().item_spacing.y = 0.0;
                match self.sel {
                    Some(s @ (Sel::Clip(_) | Sel::Over(_))) if self.clip_ref(s).is_some() => self.clip_inspector(ui, s),
                    Some(Sel::Music(i)) if i < self.proj.music.len() => self.music_inspector(ui, i),
                    Some(Sel::Title(i)) if i < self.proj.titles.len() => self.title_inspector(ui, i),
                    _ => self.sequence_inspector(ui),
                }
            });
        });
    }

    fn clip_inspector(&mut self, ui: &mut Ui, s: Sel) {
        let Some(c0) = self.clip_ref(s).cloned() else { return };
        let src = self.proj.sources[c0.src].clone();
        let start = match s {
            Sel::Clip(i) => self.proj.starts()[i],
            _ => c0.start,
        };
        let local = (self.t - start).clamp(0.0, c0.len());
        let first_v1 = matches!(s, Sel::Clip(0));
        let fps = self.proj.fps as f64;
        Frame::new().inner_margin(Margin::symmetric(12, 10)).show(ui, |ui| {
            ui.horizontal(|ui| {
                let col = LABELS[c0.label as usize % LABELS.len()].1;
                let (r, _) = ui.allocate_exact_size(vec2(4.0, 30.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(2), Color32::from_rgb(col[0], col[1], col[2]));
                ui.vertical(|ui| {
                    let title = if c0.name.is_empty() { src.name() } else { c0.name.clone() };
                    ui.label(RichText::new(short(&title, 30)).font(semibold(13.5)).color(INK_STRONG));
                    let track = if matches!(s, Sel::Over(_)) { "V2" } else { "V1" };
                    ui.label(RichText::new(format!("{track} · {} × {} · {}", src.width, src.height, tc(c0.len(), self.proj.fps))).size(11.5).color(FAINT));
                });
            });
            ui.add_space(8.0);
            let mut tab = self.tab;
            if pro::segmented(ui, &mut tab, &["Clip", "Image", "Effets", "Couleur", "Audio"]) {
                self.tab = tab;
            }
        });
        let mut act: Option<Act> = None;
        let mut ch = self.color_ch;
        let has_clipboard = self.grade_board.is_some();
        let max = src.duration;
        let tab = self.tab;
        let Some(c) = self.clip_mut(s) else { return };
        match tab {
            0 => {
                pro::dock_panel(ui, "clip-name", "Clip", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.add(egui::TextEdit::singleline(&mut c.name).hint_text("Nom du clip").desired_width(f32::INFINITY));
                    ui.horizontal(|ui| {
                        let mut on = !c.disabled;
                        if ui.checkbox(&mut on, "Activé").on_hover_text("Maj+E").changed() {
                            c.disabled = !on;
                        }
                        if ui.checkbox(&mut c.reverse, "Lecture inversée").changed() {
                            c.keys.clear();
                        }
                    });
                });
                pro::dock_panel(ui, "clip-time", "Durée et vitesse", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    field_time(ui, "Entrée", &mut c.inp, 0.0, c.out - 0.05);
                    field_time(ui, "Sortie", &mut c.out, c.inp + 0.05, max);
                    row_slider(ui, "Vitesse", &mut c.speed, 0.1..=8.0, " ×", true);
                    if let Some(k) = chips(ui, &["10 %", "25 %", "50 %", "75 %", "100 %", "150 %", "200 %", "400 %", "800 %"], None) {
                        c.speed = [0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 4.0, 8.0][k];
                    }
                    let mut frozen = c.hold > 0.0;
                    if ui.checkbox(&mut frozen, "Image figée").changed() {
                        c.hold = if frozen { 2.0 } else { 0.0 };
                        if frozen {
                            c.out = (c.inp + 1.0 / fps).min(max);
                        }
                    }
                    if c.hold > 0.0 {
                        field_time(ui, "Durée figée", &mut c.hold, 0.1, 600.0);
                    }
                    if pro::secondary(ui, "Vitesse/Durée…").on_hover_text("Ctrl+R").clicked() {
                        act = Some(Act::Speed);
                    }
                });
                if matches!(s, Sel::Clip(_)) && !first_v1 {
                    pro::dock_panel(ui, "clip-trans", "Transition d'entrée", |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        egui::ComboBox::from_id_salt("trans-kind").selected_text(TRANSITIONS[c.trans as usize % TRANSITIONS.len()]).width(ui.available_width() - 4.0).height(420.0).show_ui(ui, |ui| {
                            for (k, n) in TRANSITIONS.iter().enumerate() {
                                ui.selectable_value(&mut c.trans, k as u8, *n);
                            }
                        });
                        if c.trans > 0 {
                            row_slider(ui, "Durée", &mut c.trans_dur, 0.1..=5.0, " s", false);
                            if let Some(k) = chips(ui, &["0,25 s", "0,5 s", "1 s", "2 s", "3 s"], None) {
                                c.trans_dur = [0.25, 0.5, 1.0, 2.0, 3.0][k];
                            }
                            let mut al = c.trans_align as usize;
                            egui::ComboBox::from_id_salt("trans-align").selected_text(ALIGNS[al.min(2)]).width(ui.available_width() - 4.0).show_ui(ui, |ui| {
                                for (k, n) in ALIGNS.iter().enumerate() {
                                    ui.selectable_value(&mut al, k, *n);
                                }
                            });
                            c.trans_align = al as u8;
                            if c.gap > 0.0 {
                                ui.label(RichText::new("Un espace vide précède le clip : la transition est ignorée.").size(11.5).color(FAINT));
                            }
                        }
                    });
                }
                pro::dock_panel(ui, "clip-fades", "Fondus", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let len = c.len();
                    row_slider(ui, "Fondu d'entrée", &mut c.fade_in, 0.0..=len.min(10.0), " s", false);
                    row_slider(ui, "Fondu de sortie", &mut c.fade_out, 0.0..=len.min(10.0), " s", false);
                    let mut white = c.fade_white as usize;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Fondu vers").size(12.5).color(MUTED));
                        if pro::segmented(ui, &mut white, &["Noir", "Blanc"]) {
                            c.fade_white = white == 1;
                        }
                    });
                });
                pro::dock_panel(ui, "clip-label", "Étiquette", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (k, (name, rgb)) in LABELS.iter().enumerate() {
                            let (r, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                            ui.painter().circle_filled(r.center(), 8.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                            if c.label as usize == k {
                                ui.painter().circle_stroke(r.center(), 10.5, Stroke::new(1.5, Color32::WHITE));
                            }
                            if resp.on_hover_text(*name).clicked() {
                                c.label = k as u8;
                            }
                        }
                    });
                });
                pro::dock_panel(ui, "clip-actions", "Actions", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if pro::secondary(ui, if matches!(s, Sel::Over(_)) { "Ramener sur V1" } else { "Monter sur V2" }).clicked() {
                            act = Some(Act::ToTrack);
                        }
                        if src.has_audio && pro::secondary(ui, "Détacher l'audio").on_hover_text("Ctrl+L").clicked() {
                            act = Some(Act::Detach);
                        }
                    });
                });
            }
            1 => {
                pro::dock_panel(ui, "clip-motion", "Trajectoire", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let rows: [(u8, std::ops::RangeInclusive<f32>, f32, &str); 5] = [(0, 0.05..=5.0, 100.0, " %"), (1, -2.0..=2.0, 1.0, ""), (2, -2.0..=2.0, 1.0, ""), (3, -360.0..=360.0, 1.0, "°"), (4, 0.0..=1.0, 100.0, " %")];
                    for (prop, range, scale, suf) in rows {
                        if let Some(t) = kf_row(ui, c, local, prop, range, scale, suf) {
                            act = Some(Act::Seek(start + t));
                        }
                    }
                    let mut kb = c.zoom_end > 0.0;
                    if ui.checkbox(&mut kb, "Zoom progressif (Ken Burns)").changed() {
                        c.zoom_end = if kb { c.zoom * 1.15 } else { 0.0 };
                    }
                    if c.zoom_end > 0.0 {
                        pct_slider(ui, "Échelle finale", &mut c.zoom_end, 0.1..=4.0);
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button("↺ 90°").clicked() {
                            c.rotation = (c.rotation - 90.0 + 540.0) % 360.0 - 180.0;
                        }
                        if ui.small_button("↻ 90°").clicked() {
                            c.rotation = (c.rotation + 90.0 + 540.0) % 360.0 - 180.0;
                        }
                        ui.checkbox(&mut c.flip, "Miroir");
                        ui.checkbox(&mut c.flip_v, "Retourner");
                    });
                });
                pro::dock_panel(ui, "clip-pip", "Incrustation d'image", |ui| {
                    if let Some(k) = chips(ui, &PIP, None) {
                        pip(c, k);
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new("Idéal sur la piste V2, au-dessus de la vidéo principale.").size(11.5).color(FAINT));
                });
                pro::dock_panel(ui, "clip-crop", "Recadrage", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    for (k, label) in ["Gauche", "Haut", "Droite", "Bas"].into_iter().enumerate() {
                        pct_slider(ui, label, &mut c.crop[k], 0.0..=0.49);
                    }
                    if let Some(k) = chips(ui, &["Aucun", "Bandes 2.39:1", "Vertical 9:16", "Carré 1:1"], None) {
                        c.crop = match k {
                            1 => [0.0, 0.12, 0.0, 0.12],
                            2 => [0.342, 0.0, 0.342, 0.0],
                            3 => [0.219, 0.0, 0.219, 0.0],
                            _ => [0.0; 4],
                        };
                    }
                });
                pro::dock_panel(ui, "clip-keys", "Images clés", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.label(RichText::new(format!("{} image(s) clé(s) sur ce clip", c.keys.len())).size(12.0).color(MUTED));
                    ui.horizontal_wrapped(|ui| {
                        let mut smooth = c.keys.iter().any(|k| k.ease);
                        if ui.add_enabled(!c.keys.is_empty(), egui::Checkbox::new(&mut smooth, "Lissage")).changed() {
                            for k in &mut c.keys {
                                k.ease = smooth;
                            }
                        }
                        if ui.add_enabled(!c.keys.is_empty(), egui::Button::new("Tout supprimer")).clicked() {
                            let snap = c.eval(local);
                            c.keys.clear();
                            for p in 0..10 {
                                c.set_prop(p, snap.prop(p));
                            }
                        }
                    });
                });
            }
            2 => {
                pro::dock_panel(ui, "fx-presets", "Préréglages d'effets", |ui| {
                    if let Some(k) = chips(ui, &EFFECT_PRESETS, None) {
                        effect_preset(c, k);
                    }
                });
                pro::dock_panel(ui, "fx-blur", "Flou et netteté", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    if let Some(t) = kf_row(ui, c, local, 5, 0.0..=100.0, 1.0, "") {
                        act = Some(Act::Seek(start + t));
                    }
                    row_slider(ui, "Netteté", &mut c.sharpen, 0.0..=100.0, "", false);
                });
                pro::dock_panel(ui, "fx-style", "Stylisation", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    row_slider(ui, "Mosaïque", &mut c.pixelate, 0.0..=100.0, "", false);
                    row_slider(ui, "Postérisation", &mut c.posterize, 0.0..=16.0, " niveaux", false);
                    row_slider(ui, "Grain", &mut c.grain, 0.0..=100.0, "", false);
                    row_slider(ui, "Vignettage", &mut c.vignette, -100.0..=100.0, "", false);
                    ui.checkbox(&mut c.invert, "Négatif (inverser les couleurs)");
                });
                pro::dock_panel(ui, "fx-key", "Incrustation (fond vert)", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut c.key_on, "Activer");
                        let mut col = Color32::from_rgb(c.key_color[0], c.key_color[1], c.key_color[2]);
                        if ui.color_edit_button_srgba(&mut col).changed() {
                            c.key_color = [col.r(), col.g(), col.b()];
                        }
                        if ui.small_button("Vert").clicked() {
                            (c.key_color, c.key_on) = ([0, 177, 64], true);
                        }
                        if ui.small_button("Bleu").clicked() {
                            (c.key_color, c.key_on) = ([0, 71, 187], true);
                        }
                    });
                    row_slider(ui, "Tolérance", &mut c.key_tol, 0.0..=100.0, "", false);
                    row_slider(ui, "Douceur", &mut c.key_soft, 0.0..=100.0, "", false);
                });
                pro::dock_panel(ui, "fx-mask", "Masque", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let mut kind = c.mask.as_ref().map_or(0, |m| m.kind as usize + 1);
                    if pro::segmented(ui, &mut kind, &["Aucun", "Rectangle", "Ellipse", "Plume"]) {
                        c.mask = match kind {
                            0 => None,
                            3 => Some(Mask { kind: 2, pts: vec![[0.3, 0.25], [0.7, 0.25], [0.75, 0.75], [0.25, 0.75]], feather: 5.0, invert: false }),
                            k => Some(Mask { kind: k as u8 - 1, pts: vec![[0.25, 0.2], [0.75, 0.8]], feather: 5.0, invert: false }),
                        };
                    }
                    if let Some(m) = &mut c.mask {
                        row_slider(ui, "Contour progressif", &mut m.feather, 0.0..=100.0, "", false);
                        ui.checkbox(&mut m.invert, "Inverser le masque");
                    }
                    ui.label(RichText::new("Dessine-le dans le moniteur avec la Plume (P), le Rectangle ou l'Ellipse ; déplace ses points avec la Sélection.").size(11.5).color(FAINT));
                });
                pro::dock_panel(ui, "fx-reset", "Réinitialiser", |ui| {
                    if pro::secondary(ui, "Réinitialiser tous les effets").clicked() {
                        c.reset_look();
                    }
                });
            }
            3 => {
                pro::dock_panel(ui, "color-basic", "Correction de base", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    if let Some(t) = kf_row(ui, c, local, 8, -100.0..=100.0, 1.0, "") {
                        act = Some(Act::Seek(start + t));
                    }
                    row_slider(ui, "Contraste", &mut c.contrast, -100.0..=100.0, "", false);
                    row_slider(ui, "Hautes lumières", &mut c.highlights, -100.0..=100.0, "", false);
                    row_slider(ui, "Ombres", &mut c.shadows, -100.0..=100.0, "", false);
                    row_slider(ui, "Blancs", &mut c.whites, -100.0..=100.0, "", false);
                    row_slider(ui, "Noirs", &mut c.blacks, -100.0..=100.0, "", false);
                    if let Some(t) = kf_row(ui, c, local, 6, -100.0..=100.0, 1.0, "") {
                        act = Some(Act::Seek(start + t));
                    }
                    row_slider(ui, "Température", &mut c.temperature, -100.0..=100.0, "", false);
                    row_slider(ui, "Teinte", &mut c.tint, -100.0..=100.0, "", false);
                    if let Some(t) = kf_row(ui, c, local, 7, -100.0..=100.0, 1.0, "") {
                        act = Some(Act::Seek(start + t));
                    }
                    row_slider(ui, "Vibrance", &mut c.vibrance, -100.0..=100.0, "", false);
                });
                pro::dock_panel(ui, "color-look", "Créatif", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    if let Some(k) = chips(ui, &LOOKS, Some(c.look as usize)) {
                        c.look = k as u8;
                    }
                    pct_slider(ui, "Intensité du look", &mut c.look_amt, 0.0..=1.0);
                    row_slider(ui, "Rotation de teinte", &mut c.hue, -180.0..=180.0, "°", false);
                });
                pro::dock_panel(ui, "color-curves", "Courbes", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    pro::segmented(ui, &mut ch, &["Maître", "Rouge", "Vert", "Bleu"]);
                    ui.add_space(4.0);
                    let col = [Color32::WHITE, Color32::from_rgb(255, 90, 80), Color32::from_rgb(80, 220, 110), Color32::from_rgb(90, 150, 255)][ch.min(3)];
                    curve(ui, &mut c.curves[ch.min(3)], col);
                    if let Some(k) = chips(ui, &["Linéaire", "Contraste doux", "Contraste fort", "Fondu des noirs", "Éclaircir", "Assombrir"], None) {
                        c.curves[ch.min(3)] = match k {
                            1 => [0.0, -0.04, 0.0, 0.04, 0.0],
                            2 => [0.0, -0.09, 0.0, 0.09, 0.0],
                            3 => [0.1, 0.04, 0.0, 0.0, 0.0],
                            4 => [0.0, 0.08, 0.1, 0.06, 0.0],
                            5 => [0.0, -0.08, -0.1, -0.06, 0.0],
                            _ => [0.0; 5],
                        };
                    }
                });
                pro::dock_panel(ui, "color-wheels", "Roues chromatiques", |ui| {
                    ui.horizontal(|ui| {
                        let size = ((ui.available_width() - 16.0) / 3.0).min(86.0);
                        for (k, label) in ["Ombres", "Tons moyens", "Hautes lum."].into_iter().enumerate() {
                            wheel(ui, label, &mut c.wheels[k], size);
                        }
                    });
                    ui.label(RichText::new("Double-clic sur une roue pour la remettre au centre.").size(11.5).color(FAINT));
                });
                pro::dock_panel(ui, "color-tools", "Étalonnage", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if pro::secondary(ui, "Copier").clicked() {
                            act = Some(Act::CopyGrade);
                        }
                        if ui.add_enabled(has_clipboard, egui::Button::new("Coller")).clicked() {
                            act = Some(Act::PasteGrade);
                        }
                        if pro::secondary(ui, "Appliquer à tous").on_hover_text("Copie cet étalonnage sur tous les clips V1 et V2").clicked() {
                            act = Some(Act::GradeAll);
                        }
                        if pro::secondary(ui, "Réinitialiser").clicked() {
                            c.reset_color();
                        }
                    });
                });
            }
            _ => {
                pro::dock_panel(ui, "clip-audio", "Son du clip", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.checkbox(&mut c.mute, "Couper le son du clip");
                    if let Some(t) = kf_row(ui, c, local, 9, -40.0..=12.0, 1.0, " dB") {
                        act = Some(Act::Seek(start + t));
                    }
                    row_slider(ui, "Balance", &mut c.pan, -1.0..=1.0, "", false);
                    if c.hold > 0.0 {
                        ui.label(RichText::new("Une image figée n'a pas de son.").size(12.0).color(FAINT));
                    }
                });
                pro::dock_panel(ui, "clip-norm", "Normalisation", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (label, db) in [("Crête −1 dB", -1.0), ("Crête −3 dB", -3.0), ("Crête −6 dB", -6.0)] {
                            if pro::secondary(ui, label).clicked() {
                                act = Some(Act::Normalize(db));
                            }
                        }
                    });
                });
                pro::dock_panel(ui, "clip-afade", "Fondus audio", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let len = c.len();
                    row_slider(ui, "Fondu d'entrée", &mut c.fade_in, 0.0..=len.min(10.0), " s", false);
                    row_slider(ui, "Fondu de sortie", &mut c.fade_out, 0.0..=len.min(10.0), " s", false);
                    if src.has_audio && pro::secondary(ui, "Détacher l'audio sur A2").clicked() {
                        act = Some(Act::Detach);
                    }
                });
            }
        }
        self.color_ch = ch;
        match act {
            Some(Act::Seek(t)) => self.seek(t),
            Some(Act::Detach) => self.detach_audio(s),
            Some(Act::Normalize(db)) => self.normalize(s, db),
            Some(Act::ToTrack) => self.move_track(s),
            Some(Act::Speed) => self.speed_sheet = true,
            Some(Act::CopyGrade) => self.grade_board = self.clip_ref(s).cloned(),
            Some(Act::PasteGrade) => self.paste_grade(&[s]),
            Some(Act::GradeAll) => {
                self.grade_board = self.clip_ref(s).cloned();
                let all: Vec<Sel> = (0..self.proj.clips.len()).map(Sel::Clip).chain((0..self.proj.overlays.len()).map(Sel::Over)).collect();
                self.paste_grade(&all);
            }
            None => {}
        }
    }

    fn music_inspector(&mut self, ui: &mut Ui, i: usize) {
        let src = self.proj.sources[self.proj.music[i].src].clone();
        let t = self.t;
        let mut norm = None;
        let m = &mut self.proj.music[i];
        pro::dock_panel(ui, "music", "Musique", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.label(RichText::new(short(&src.name(), 30)).font(semibold(13.5)).color(INK));
            field_time(ui, "Début", &mut m.start, 0.0, 36000.0);
            field_time(ui, "Entrée", &mut m.inp, 0.0, m.out - 0.05);
            field_time(ui, "Sortie", &mut m.out, m.inp + 0.05, src.duration);
            if pro::secondary(ui, "Placer à la tête de lecture").clicked() {
                m.start = t;
            }
        });
        pro::dock_panel(ui, "music-mix", "Mixage", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.checkbox(&mut m.mute, "Couper");
            let mut db = 20.0 * m.volume.max(1e-4).log10();
            if row_slider(ui, "Volume", &mut db, -40.0..=12.0, " dB", false) {
                m.volume = 10f32.powf(db / 20.0);
            }
            row_slider(ui, "Balance", &mut m.pan, -1.0..=1.0, "", false);
            let len = m.out - m.inp;
            row_slider(ui, "Fondu d'entrée", &mut m.fade_in, 0.0..=len.min(20.0), " s", false);
            row_slider(ui, "Fondu de sortie", &mut m.fade_out, 0.0..=len.min(20.0), " s", false);
            ui.horizontal_wrapped(|ui| {
                for (label, db) in [("Normaliser −1 dB", -1.0), ("−6 dB", -6.0), ("−12 dB (fond)", -12.0)] {
                    if pro::secondary(ui, label).clicked() {
                        norm = Some(db);
                    }
                }
            });
        });
        pro::dock_panel(ui, "music-label", "Étiquette", |ui| {
            ui.horizontal_wrapped(|ui| {
                for (k, (name, rgb)) in LABELS.iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                    ui.painter().circle_filled(r.center(), 8.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                    if m.label as usize == k {
                        ui.painter().circle_stroke(r.center(), 10.5, Stroke::new(1.5, Color32::WHITE));
                    }
                    if resp.on_hover_text(*name).clicked() {
                        m.label = k as u8;
                    }
                }
            });
        });
        if let Some(db) = norm {
            self.normalize(Sel::Music(i), db);
        }
    }

    fn title_inspector(&mut self, ui: &mut Ui, i: usize) {
        let t = &mut self.proj.titles[i];
        pro::dock_panel(ui, "title-presets", "Styles prédéfinis", |ui| {
            if let Some(k) = chips(ui, &TITLE_PRESETS, None) {
                title_preset(t, k);
            }
        });
        pro::dock_panel(ui, "title-text", "Texte", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.add(egui::TextEdit::multiline(&mut t.text).desired_rows(3).desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                ui.label(RichText::new("Police").size(12.5).color(MUTED));
                egui::ComboBox::from_id_salt("font").selected_text(FONTS[t.font as usize % FONTS.len()].0).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                    for (k, (name, _)) in FONTS.iter().enumerate() {
                        ui.selectable_value(&mut t.font, k as u8, *name);
                    }
                });
            });
            pct_slider(ui, "Taille", &mut t.size, 0.02..=0.3);
            row_slider(ui, "Espacement des lettres", &mut t.spacing, -0.1..=1.0, "", false);
            row_slider(ui, "Interligne", &mut t.leading, -0.5..=2.0, "", false);
            let mut al = t.align as usize;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Alignement").size(12.5).color(MUTED));
                if pro::segmented(ui, &mut al, &["Auto", "Gauche", "Centre", "Droite"]) {
                    t.align = al as u8;
                }
            });
            ui.checkbox(&mut t.vertical, "Texte vertical");
        });
        pro::dock_panel(ui, "title-look", "Apparence", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let color_row = |ui: &mut Ui, label: &str, c: &mut [u8; 4]| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).size(12.5).color(MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                        if ui.color_edit_button_srgba(&mut col).changed() {
                            *c = col.to_srgba_unmultiplied();
                        }
                    });
                });
            };
            color_row(ui, "Couleur du texte", &mut t.color);
            row_slider(ui, "Contour", &mut t.outline, 0.0..=100.0, "", false);
            if t.outline > 0.0 {
                color_row(ui, "Couleur du contour", &mut t.outline_color);
            }
            ui.horizontal(|ui| {
                ui.checkbox(&mut t.shadow, "Ombre portée");
                ui.checkbox(&mut t.band, "Bandeau");
            });
            if t.band || t.place == 3 {
                color_row(ui, "Couleur du bandeau", &mut t.band_color);
            }
        });
        pro::dock_panel(ui, "title-place", "Position et durée", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let mut place = t.place as usize;
            if pro::segmented(ui, &mut place, &["Haut", "Centre", "Bas", "Tiers"]) {
                t.place = place as u8;
            }
            row_slider(ui, "Décalage X", &mut t.dx, -0.5..=0.5, "", false);
            row_slider(ui, "Décalage Y", &mut t.dy, -0.5..=0.5, "", false);
            field_time(ui, "Début", &mut t.start, 0.0, 36000.0);
            field_time(ui, "Durée", &mut t.dur, 0.2, 600.0);
            let mut roll = t.roll as usize;
            ui.label(RichText::new("Défilement").size(12.5).color(MUTED));
            if pro::segmented(ui, &mut roll, &ROLLS) {
                t.roll = roll as u8;
            }
        });
        pro::dock_panel(ui, "title-anim", "Animation d'entrée et de sortie", |ui| {
            if let Some(k) = chips(ui, &ANIMS, Some(t.anim as usize)) {
                t.anim = k as u8;
            }
        });
    }

    fn sequence_inspector(&mut self, ui: &mut Ui) {
        pro::dock_panel(ui, "sequence", "Séquence", |ui| {
            ui.spacing_mut().item_spacing.y = 5.0;
            pro::label_value(ui, "Format", &format!("{} × {}", self.proj.width, self.proj.height));
            pro::label_value(ui, "Images/s", &self.proj.fps.to_string());
            pro::label_value(ui, "Durée", &tc(self.proj.duration(), self.proj.fps));
            pro::label_value(ui, "V1 · V2 · titres · A2", &format!("{} · {} · {} · {}", self.proj.clips.len(), self.proj.overlays.len(), self.proj.titles.len(), self.proj.music.len()));
            ui.add_space(6.0);
            if pro::secondary(ui, "Réglages de la séquence…").clicked() {
                self.seq_sheet = true;
            }
        });
        pro::dock_panel(ui, "quick", "Ajouter", |ui| {
            ui.horizontal_wrapped(|ui| {
                if pro::secondary_icon(ui, "Titre", |p, r, c| glyph(p, r, G::Title, c)).clicked() {
                    self.add_title_at(0.0, 0.0, false);
                }
                if pro::secondary_icon(ui, "Marqueur", |p, r, c| glyph(p, r, G::Marker, c)).clicked() {
                    self.add_marker();
                }
            });
            ui.add_space(4.0);
            if let Some(k) = chips(ui, &crate::model::GENERATORS.map(|g| g.1), None) {
                self.add_generator(k);
            }
        });
        pro::dock_panel(ui, "mixer", "Mixage", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            row_slider(ui, "Volume général", &mut self.proj.master_db, -24.0..=12.0, " dB", false);
            row_slider(ui, "Atténuation musique sous les voix", &mut self.proj.duck_db, -30.0..=0.0, " dB", false);
            ui.label(RichText::new("La musique (A2) baisse automatiquement quand un clip V1 a du son.").size(11.5).color(FAINT));
        });
        if !self.proj.markers.is_empty() {
            let fps = self.proj.fps;
            let mut go = None;
            let mut edit = None;
            pro::dock_panel(ui, "markers", "Marqueurs", |ui| {
                for (k, m) in self.proj.markers.iter().enumerate() {
                    let col = MARKER_COLORS[m.color as usize % MARKER_COLORS.len()].1;
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                        diamond(ui.painter(), r.center(), 5.0, true, Color32::from_rgb(col[0], col[1], col[2]));
                        let name = if m.name.is_empty() { format!("Marqueur {}", k + 1) } else { short(&m.name, 20) };
                        if ui.add(egui::Label::new(RichText::new(format!("{}  {name}", tc(m.t, fps))).size(12.0).color(INK)).sense(Sense::click())).on_hover_text(if m.note.is_empty() { "Clic : aller · double-clic : modifier".to_string() } else { m.note.clone() }).clicked() {
                            go = Some(m.t);
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("✎").on_hover_text("Modifier").clicked() {
                                edit = Some(k);
                            }
                        });
                    });
                }
                ui.add_space(4.0);
                if pro::ghost(ui, "Copier comme chapitres YouTube").clicked() {
                    ui.ctx().copy_text(crate::chapters(&self.proj));
                }
            });
            if let Some(m) = go {
                self.seek(m);
            }
            if edit.is_some() {
                self.marker_edit = edit;
            }
        }
        if !self.undo.is_empty() {
            let mut back = None;
            pro::dock_panel(ui, "history", "Historique", |ui| {
                let n = self.undo.len();
                let from = n.saturating_sub(12);
                for k in (from..n).rev() {
                    let next = if k + 1 < n { &self.undo[k + 1] } else { &self.proj };
                    let label = crate::describe(&self.undo[k], next);
                    if pro::ghost(ui, &format!("{}  {label}", k + 1)).on_hover_text("Revenir juste avant cette étape").clicked() {
                        back = Some(k);
                    }
                }
            });
            if let Some(k) = back {
                self.history_to(k);
            }
        }
        pro::dock_panel(ui, "shortcuts", "Raccourcis", |ui| {
            for (k, d) in [
                ("Espace", "Lecture"),
                ("J K L", "Navette arrière · pause · avant"),
                ("S / Ctrl+K", "Scinder"),
                ("Ctrl+Maj+K", "Scinder toutes les pistes"),
                ("Q / W", "Rogner début / fin"),
                ("Maj+Suppr", "Supprimer et refermer"),
                ("Alt+Suppr", "Lever (laisser un vide)"),
                ("I / O", "Entrée / sortie"),
                ("M", "Marqueur"),
                ("Ctrl+T", "Nouveau titre"),
                ("Ctrl+R", "Vitesse/Durée"),
                ("↑ ↓", "Point de montage"),
                ("Maj+S", "Magnétisme"),
                ("F", "Mode visionneuse"),
            ] {
                ui.horizontal(|ui| {
                    pro::kbd(ui, k);
                    ui.label(RichText::new(d).size(12.0).color(MUTED));
                });
            }
            ui.add_space(4.0);
            if pro::ghost(ui, "Tous les tutoriels  →").clicked() {
                self.help.learn("Lecture et navigation");
            }
        });
    }
}

pub fn draw_keys(p: &egui::Painter, r: Rect, c: &Clip, pps: f32) {
    for k in &c.keys {
        let x = r.left() + k.t as f32 * pps;
        if x >= r.left() - 1.0 && x <= r.right() + 1.0 {
            diamond(p, pos2(x, r.bottom() - 7.0), 3.5, true, Color32::WHITE);
        }
    }
}

