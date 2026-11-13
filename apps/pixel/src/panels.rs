use crate::doc::{BLENDS, rgb_to_hsl};
use crate::tools::{self, BRUSH_PRESETS, brushy};
use crate::{Pixel, SWATCH_SETS, lock_glyph};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use studio_ui::pro::{self, BLUE, FAINT, FIELD, INK, INK_STRONG, LINE, MUTED, PANEL, RED, semibold};

const TAGS: [Color32; 7] = [Color32::TRANSPARENT, Color32::from_rgb(255, 69, 58), Color32::from_rgb(255, 159, 10), Color32::from_rgb(255, 214, 10), Color32::from_rgb(48, 209, 88), Color32::from_rgb(10, 132, 255), Color32::from_rgb(191, 90, 242)];
const TAG_NAMES: [&str; 7] = ["Aucune", "Rouge", "Orange", "Jaune", "Vert", "Bleu", "Violet"];

fn swatch(ui: &mut Ui, c: Color32, size: f32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    ui.painter().rect(r, CornerRadius::same(5), c, Stroke::new(1.0, LINE), StrokeKind::Inside);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn label(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(12.5).color(MUTED));
}

fn fixed_label(ui: &mut Ui, w: f32, t: &str, size: f32, color: Color32) {
    ui.allocate_ui_with_layout(vec2(w, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_min_width(w);
        ui.add(egui::Label::new(RichText::new(t).size(size).color(color)).truncate());
    });
}

fn slider_row(ui: &mut Ui, name: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        fixed_label(ui, 72.0, name, 12.0, MUTED);
        ui.spacing_mut().slider_width = (ui.available_width() - 64.0).max(60.0);
        changed = ui.add(egui::Slider::new(v, range).suffix(suffix).max_decimals(1)).changed();
    });
    changed
}

fn hsv_rgb(h: f32, s: f32, v: f32) -> Color32 {
    egui::ecolor::Hsva::new(h.rem_euclid(1.0), s, v, 1.0).into()
}

impl Pixel {
    pub fn dock(&mut self, root: &mut Ui, ctx: &egui::Context) {
        self.sync_thumbs(ctx);
        egui::Panel::right("dock").resizable(false).exact_size(296.0).frame(egui::Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                pro::dock_panel(ui, "color", "Couleur", |ui| self.color_panel(ui));
                pro::dock_panel(ui, "swatches", "Nuancier", |ui| self.swatch_panel(ui));
                if brushy(self.tool) {
                    pro::dock_panel(ui, "brushes", "Pinceaux", |ui| self.brush_panel(ui));
                }
                pro::dock_panel(ui, "props", "Propriétés", |ui| self.props_panel(ui));
                self.navigator(ui);
                pro::dock_panel(ui, "info", "Infos", |ui| self.info_panel(ui));
                pro::dock_panel(ui, "history", "Historique", |ui| self.history_panel(ui));
                pro::dock_panel(ui, "layers", "Calques", |ui| self.layers_panel(ui));
            });
        });
    }

    fn color_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 6.0;
        let w = ui.available_width();
        pro::color_picker(ui, &mut self.primary, w);
        ui.add_space(2.0);
        pro::segmented(ui, &mut self.color_mode, &["RVB", "TSL", "CMJN"]);
        let c = self.primary;
        match self.color_mode {
            0 => {
                let mut v = [c.r() as f32, c.g() as f32, c.b() as f32];
                let mut ch = false;
                for (k, n) in ["Rouge", "Vert", "Bleu"].iter().enumerate() {
                    ch |= slider_row(ui, n, &mut v[k], 0.0..=255.0, "");
                }
                if ch {
                    self.primary = Color32::from_rgb(v[0] as u8, v[1] as u8, v[2] as u8);
                }
            }
            1 => {
                let (h, s, l) = rgb_to_hsl(c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0);
                let mut v = [h * 360.0, s * 100.0, l * 100.0];
                let mut ch = false;
                ch |= slider_row(ui, "Teinte", &mut v[0], 0.0..=360.0, "°");
                ch |= slider_row(ui, "Saturation", &mut v[1], 0.0..=100.0, " %");
                ch |= slider_row(ui, "Luminosité", &mut v[2], 0.0..=100.0, " %");
                if ch {
                    let (r, g, b) = crate::doc::hsl_to_rgb(v[0] / 360.0, v[1] / 100.0, v[2] / 100.0);
                    self.primary = Color32::from_rgb((r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8);
                }
            }
            _ => {
                let (r, g, b) = (c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0);
                let k = 1.0 - r.max(g).max(b);
                let d = (1.0 - k).max(1e-4);
                let mut v = [(1.0 - r - k) / d * 100.0, (1.0 - g - k) / d * 100.0, (1.0 - b - k) / d * 100.0, k * 100.0];
                let mut ch = false;
                for (i, n) in ["Cyan", "Magenta", "Jaune", "Noir"].iter().enumerate() {
                    ch |= slider_row(ui, n, &mut v[i], 0.0..=100.0, " %");
                }
                if ch {
                    let k = v[3] / 100.0;
                    let f = |x: f32| ((1.0 - x / 100.0) * (1.0 - k) * 255.0).round() as u8;
                    self.primary = Color32::from_rgb(f(v[0]), f(v[1]), f(v[2]));
                }
            }
        }
        ui.horizontal(|ui| {
            label(ui, "Hex");
            let id = ui.make_persistent_id("hex");
            let cur = format!("{:02X}{:02X}{:02X}", self.primary.r(), self.primary.g(), self.primary.b());
            let mut text = ui.ctx().data(|d| d.get_temp::<(String, String)>(id)).filter(|t| t.0 == cur).map(|t| t.1).unwrap_or(cur.clone());
            let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(80.0).char_limit(7).font(egui::TextStyle::Monospace));
            if r.changed()
                && let Ok(v) = u32::from_str_radix(text.trim().trim_start_matches('#'), 16)
                && text.trim().trim_start_matches('#').len() == 6
            {
                self.primary = Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
            }
            let now = format!("{:02X}{:02X}{:02X}", self.primary.r(), self.primary.g(), self.primary.b());
            ui.ctx().data_mut(|d| d.insert_temp(id, (now, text)));
            if ui.small_button("Web").on_hover_text("Arrondit à la couleur sûre pour le Web la plus proche").clicked() {
                let q = |v: u8| ((v as f32 / 51.0).round() * 51.0) as u8;
                self.primary = Color32::from_rgb(q(self.primary.r()), q(self.primary.g()), q(self.primary.b()));
            }
        });
    }

    fn swatch_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 6.0;
        ui.horizontal(|ui| {
            label(ui, "Bibliothèque");
            let mut k = self.swatch_set;
            if tools::combo(ui, "swset", &mut k, &SWATCH_SETS.map(|s| s.0), 130.0) {
                self.swatch_set = k;
                self.swatches = SWATCH_SETS[k].1.iter().map(|c| Color32::from_rgb(c[0], c[1], c[2])).collect();
                self.save_swatches();
            }
        });
        let mut remove = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
            for (i, c) in self.swatches.clone().into_iter().enumerate() {
                let r = swatch(ui, c, 22.0).on_hover_text("Clic : principale · Ctrl+clic : secondaire · clic droit : options");
                if r.clicked() {
                    if ui.input(|i| i.modifiers.command) { self.secondary = c } else { self.primary = c }
                }
                r.context_menu(|ui| {
                    if ui.button("Définir comme couleur principale").clicked() {
                        self.primary = c;
                        ui.close();
                    }
                    if ui.button("Définir comme couleur secondaire").clicked() {
                        self.secondary = c;
                        ui.close();
                    }
                    if ui.button("Supprimer la nuance").clicked() {
                        remove = Some(i);
                        ui.close();
                    }
                });
            }
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
            ui.painter().rect(r, CornerRadius::same(5), FIELD, Stroke::NONE, StrokeKind::Inside);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, "+", FontId::proportional(16.0), INK);
            if resp.on_hover_text("Ajouter la couleur principale").clicked() && !self.swatches.contains(&self.primary) {
                self.swatches.push(self.primary);
                self.save_swatches();
            }
        });
        if let Some(i) = remove {
            self.swatches.remove(i);
            self.save_swatches();
        }
        let hsv = egui::ecolor::Hsva::from(self.primary);
        let harmonies: [(&str, Vec<Color32>); 5] = [
            ("Complémentaire", vec![hsv_rgb(hsv.h + 0.5, hsv.s, hsv.v)]),
            ("Analogues", vec![hsv_rgb(hsv.h - 1.0 / 12.0, hsv.s, hsv.v), hsv_rgb(hsv.h + 1.0 / 12.0, hsv.s, hsv.v)]),
            ("Triade", vec![hsv_rgb(hsv.h + 1.0 / 3.0, hsv.s, hsv.v), hsv_rgb(hsv.h + 2.0 / 3.0, hsv.s, hsv.v)]),
            ("Tétrade", vec![hsv_rgb(hsv.h + 0.25, hsv.s, hsv.v), hsv_rgb(hsv.h + 0.5, hsv.s, hsv.v), hsv_rgb(hsv.h + 0.75, hsv.s, hsv.v)]),
            ("Nuances", vec![hsv_rgb(hsv.h, hsv.s, hsv.v * 0.4), hsv_rgb(hsv.h, hsv.s, hsv.v * 0.7), hsv_rgb(hsv.h, hsv.s * 0.5, (hsv.v * 1.2).min(1.0)), hsv_rgb(hsv.h, hsv.s * 0.25, 1.0)]),
        ];
        label(ui, "Harmonies");
        for (name, cols) in harmonies {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                fixed_label(ui, 100.0, name, 11.5, FAINT);
                for c in cols {
                    if swatch(ui, c, 18.0).on_hover_text("Prendre cette couleur").clicked() {
                        self.primary = c;
                    }
                }
            });
        }
        if !self.recent_colors.is_empty() {
            label(ui, "Récentes");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for c in self.recent_colors.clone() {
                    if swatch(ui, c, 18.0).clicked() {
                        self.primary = c;
                    }
                }
            });
        }
    }

    fn brush_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 6.0;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (i, p) in BRUSH_PRESETS.iter().enumerate() {
                let on = self.brush_preset == i;
                let txt = RichText::new(p.0).size(11.5).color(if on { INK_STRONG } else { INK });
                if ui.add(egui::Button::new(txt).fill(if on { BLUE.gamma_multiply(0.35) } else { FIELD }).corner_radius(6)).clicked() {
                    self.brush_preset = i;
                    let (_, hard, spacing, angle, round, sj, fj, sc) = *p;
                    self.brush.hardness = hard;
                    self.brush.spacing = spacing;
                    self.brush.angle = angle;
                    self.brush.roundness = round;
                    self.brush.size_jitter = sj;
                    self.brush.flow_jitter = fj;
                    self.brush.scatter = sc;
                }
            }
        });
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
        ui.painter().rect_filled(r, CornerRadius::same(6), Color32::from_rgb(0x14, 0x14, 0x17));
        let b = self.brush;
        let k = (16.0 / b.radius.max(1.0)).min(1.0);
        let mut x = r.left() + 14.0;
        let mut n = 0u32;
        while x < r.right() - 14.0 {
            n += 1;
            let jit = crate::filters::hash(n, 1, 9);
            let rad = (b.radius * k * (1.0 - b.size_jitter * jit)).max(1.0);
            let y = r.center().y + ((x - r.left()) / r.width() * 6.28).sin() * 8.0 + (crate::filters::hash(n, 2, 9) - 0.5) * b.scatter * 20.0;
            let a = (b.flow * (1.0 - b.flow_jitter * crate::filters::hash(n, 3, 9)) * 255.0 * (0.4 + b.hardness * 0.6)) as u8;
            ui.painter().add(egui::epaint::EllipseShape { center: pos2(x, y), radius: vec2(rad, rad * b.roundness.max(0.05)), fill: Color32::from_white_alpha(a.max(20)), stroke: Stroke::NONE, angle: b.angle.to_radians() });
            x += (b.radius * 2.0 * b.spacing * k).max(1.5);
        }
        slider_row(ui, "Espacement", &mut self.brush.spacing, 0.01..=2.0, "");
        slider_row(ui, "Angle", &mut self.brush.angle, -180.0..=180.0, "°");
        slider_row(ui, "Rondeur", &mut self.brush.roundness, 0.05..=1.0, "");
        slider_row(ui, "Var. taille", &mut self.brush.size_jitter, 0.0..=1.0, "");
        slider_row(ui, "Var. flux", &mut self.brush.flow_jitter, 0.0..=1.0, "");
        slider_row(ui, "Dispersion", &mut self.brush.scatter, 0.0..=2.0, "");
        slider_row(ui, "Lissage", &mut self.smoothing, 0.0..=0.95, "");
    }

    fn props_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 4.0;
        pro::label_value(ui, "Document", &format!("{} × {} px", self.doc.width(), self.doc.height()));
        pro::label_value(ui, "Calques", &self.doc.layers.len().to_string());
        if let Some(s) = self.doc.selection {
            pro::label_value(ui, "Sélection", &format!("{} × {} à ({}, {})", s.width(), s.height(), s.x0, s.y0));
        }
        ui.add_space(6.0);
        let a = self.doc.active;
        let mut op = self.doc.layers[a].opacity * 100.0;
        if slider_row(ui, "Opacité", &mut op, 0.0..=100.0, " %") {
            self.doc.layers[a].opacity = op / 100.0;
            self.doc.touch_all();
            self.doc.modified = true;
        }
        ui.horizontal(|ui| {
            fixed_label(ui, 72.0, "Fusion", 12.0, MUTED);
            let mut b = self.doc.layers[a].blend;
            if tools::blend_combo(ui, "blend", &mut b, ui.available_width() - 12.0) {
                self.doc.layers[a].blend = b;
                self.doc.touch_all();
                self.doc.modified = true;
            }
        });
        ui.horizontal(|ui| {
            label(ui, "Verrou");
            let l = &mut self.doc.layers[a];
            ui.toggle_value(&mut l.lock_alpha, "Transparence").on_hover_text("Peindre sans modifier la transparence");
            ui.toggle_value(&mut l.lock_pos, "Position");
            ui.toggle_value(&mut l.locked, "Tout");
        });
        ui.horizontal(|ui| {
            label(ui, "Étiquette");
            for (i, c) in TAGS.iter().enumerate().skip(1) {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                ui.painter().circle_filled(r.center(), 6.0, *c);
                if self.doc.layers[a].tag as usize == i {
                    ui.painter().circle_stroke(r.center(), 7.5, Stroke::new(1.5, INK_STRONG));
                }
                if resp.on_hover_text(TAG_NAMES[i]).clicked() {
                    let t = &mut self.doc.layers[a].tag;
                    *t = if *t as usize == i { 0 } else { i as u8 };
                }
            }
        });
        if let Some((k, _)) = &self.doc.layers[a].adjust {
            let title = k.title();
            if pro::secondary(ui, &format!("Modifier : {title}…")).clicked() {
                self.edit_adjustment_layer(a);
            }
        }
        if self.doc.layers[a].mask.is_some() {
            ui.horizontal(|ui| {
                label(ui, "Masque");
                if ui.selectable_label(self.doc.edit_mask, "Modifier").clicked() {
                    self.doc.edit_mask = !self.doc.edit_mask;
                }
                let on = &mut self.doc.layers[a].mask_on;
                if ui.selectable_label(!*on, "Désactivé").clicked() {
                    *on = !*on;
                    self.doc.touch_all();
                }
                if ui.small_button("Inverser").clicked() {
                    self.doc.invert_mask();
                }
            });
        }
    }

    fn navigator(&mut self, ui: &mut Ui) {
        let (dw, dh) = (self.doc.width() as f32, self.doc.height() as f32);
        let tex = self.tex.as_ref().map(|t| t.id());
        let view = self.view_doc;
        let checker = self.checker.id();
        let mut jump = None;
        let mut zoom = self.zoom;
        pro::dock_panel(ui, "navigator", "Navigateur", |ui| {
            let wv = ui.available_width();
            let k = (wv / dw).min(150.0 / dh);
            let (r, resp) = ui.allocate_exact_size(vec2(wv, dh * k), Sense::click_and_drag());
            let img = Rect::from_center_size(r.center(), vec2(dw, dh) * k);
            ui.painter().image(checker, img, Rect::from_min_max(pos2(0.0, 0.0), pos2(img.width() / 16.0, img.height() / 16.0)), Color32::WHITE);
            if let Some(id) = tex {
                ui.painter().image(id, img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            if let Some(v) = view {
                let vr = Rect::from_min_max(img.min + v.min.to_vec2() * k, img.min + v.max.to_vec2() * k).intersect(img);
                ui.painter().rect_stroke(vr, CornerRadius::same(2), Stroke::new(1.5, RED), StrokeKind::Middle);
            }
            if let Some(pp) = resp.interact_pointer_pos() {
                jump = Some(((pp - img.min) / k).to_pos2());
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().slider_width = (wv - 70.0).max(60.0);
                ui.add(egui::Slider::new(&mut zoom, 0.02..=64.0).logarithmic(true).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)));
            });
        });
        self.zoom = zoom;
        if let Some(d) = jump {
            let (sn, cs) = self.view_angle.to_radians().sin_cos();
            let (x, y) = ((dw / 2.0 - d.x) * self.zoom, (dh / 2.0 - d.y) * self.zoom);
            self.offset = vec2(x * cs - y * sn, x * sn + y * cs);
        }
    }

    fn info_panel(&mut self, ui: &mut Ui) {
        if self.hist.as_ref().is_none_or(|h| h.0 != self.doc.version) && self.last.is_none() {
            let mut hst = [[0u32; 256]; 4];
            for p in self.composite.pixels().step_by(3) {
                let l = (0.2126 * p.0[0] as f32 + 0.7152 * p.0[1] as f32 + 0.0722 * p.0[2] as f32) as usize;
                hst[0][l.min(255)] += 1;
                for k in 0..3 {
                    hst[k + 1][p.0[k] as usize] += 1;
                }
            }
            self.hist = Some((self.doc.version, hst));
        }
        pro::segmented(ui, &mut self.hist_channel, &["Luminosité", "Rouge", "Vert", "Bleu"]);
        ui.add_space(4.0);
        if let Some((_, h)) = &self.hist {
            let h = &h[self.hist_channel.min(3)];
            let col = [Color32::from_gray(150), Color32::from_rgb(230, 80, 80), Color32::from_rgb(80, 200, 100), Color32::from_rgb(80, 140, 240)][self.hist_channel.min(3)];
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 60.0), Sense::hover());
            ui.painter().rect_filled(r, CornerRadius::same(6), Color32::from_rgb(0x14, 0x14, 0x17));
            let max = *h.iter().max().unwrap_or(&1) as f32;
            for (k, v) in h.iter().enumerate() {
                let x = r.left() + r.width() * k as f32 / 256.0;
                let hh = (*v as f32 / max.max(1.0)).sqrt() * (r.height() - 4.0);
                ui.painter().line_segment([pos2(x, r.bottom() - 2.0), pos2(x, r.bottom() - 2.0 - hh)], Stroke::new(1.0, col));
            }
            let total: u64 = h.iter().map(|v| *v as u64).sum();
            let mean = h.iter().enumerate().map(|(i, v)| i as u64 * *v as u64).sum::<u64>() as f32 / total.max(1) as f32;
            ui.label(RichText::new(format!("Moyenne {mean:.1} · {} pixels échantillonnés", total)).size(11.5).color(FAINT));
        }
        let pix = |x: f32, y: f32| (x >= 0.0 && y >= 0.0 && (x as u32) < self.composite.width() && (y as u32) < self.composite.height()).then(|| self.composite.get_pixel(x as u32, y as u32).0);
        match self.cursor.and_then(|(x, y)| pix(x, y).map(|c| (x as u32, y as u32, c))) {
            Some((x, y, c)) => {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                    ui.painter().rect(r, CornerRadius::same(4), Color32::from_rgb(c[0], c[1], c[2]), Stroke::new(1.0, LINE), StrokeKind::Inside);
                    ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}  R {} V {} B {} A {}", c[0], c[1], c[2], c[0], c[1], c[2], c[3])).size(11.5).monospace().color(INK));
                });
                let (h, s, l) = rgb_to_hsl(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
                ui.label(RichText::new(format!("T {:.0}°  S {:.0} %  L {:.0} %   x {x}  y {y}", h * 360.0, s * 100.0, l * 100.0)).size(11.5).color(MUTED));
            }
            None => {
                ui.label(RichText::new("Survole l'image pour lire la couleur").size(12.0).color(FAINT));
            }
        }
        for (i, (x, y)) in self.samplers.clone().iter().enumerate() {
            if let Some(c) = pix(*x, *y) {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                    ui.painter().rect(r, CornerRadius::same(3), Color32::from_rgb(c[0], c[1], c[2]), Stroke::new(1.0, LINE), StrokeKind::Inside);
                    ui.label(RichText::new(format!("#{}  R {} V {} B {}", i + 1, c[0], c[1], c[2])).size(11.5).monospace().color(INK));
                });
            }
        }
        if let Some((a, b)) = self.ruler {
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            ui.label(RichText::new(format!("Règle : {:.1} px · {:.1}°", (dx * dx + dy * dy).sqrt(), -dy.atan2(dx).to_degrees())).size(11.5).color(MUTED));
        }
    }

    fn history_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 2.0;
        let (rows, cur) = self.doc.history();
        let mut jump = None;
        egui::ScrollArea::vertical().id_salt("hist").max_height(170.0).stick_to_bottom(true).show(ui, |ui| {
            for (i, name) in rows.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click());
                if i == cur {
                    ui.painter().rect_filled(r, CornerRadius::same(5), BLUE.gamma_multiply(0.3));
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(5), Color32::from_white_alpha(8));
                }
                ui.painter().text(r.left_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, name, FontId::proportional(12.5), if i > cur { FAINT } else { INK });
                if resp.clicked() {
                    jump = Some(i);
                }
            }
        });
        if let Some(i) = jump {
            self.doc.jump_history(i);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.small_button("Instantané").on_hover_text("Source de la forme d'historique").clicked() {
                self.snapshot_history();
            }
            if ui.small_button("Vider l'historique").clicked() {
                self.doc.clear_history();
            }
            ui.label(RichText::new(format!("{} états max", self.doc.undo_limit)).size(11.0).color(FAINT));
        });
    }

    fn layers_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 4.0;
        let active = self.doc.active;
        let mut visibility = None;
        let mut select = None;
        let mut edit_adjust = None;
        let mut mask_click = None;
        for i in (0..self.doc.layers.len()).rev() {
            let selected = i == active;
            let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
            let l = &self.doc.layers[i];
            let p = ui.painter();
            if selected {
                p.rect_filled(row, CornerRadius::same(9), BLUE.gamma_multiply(0.24));
            } else if resp.hovered() {
                p.rect_filled(row, CornerRadius::same(9), Color32::from_white_alpha(8));
            }
            if l.tag > 0 {
                p.rect_filled(Rect::from_min_size(row.min + vec2(0.0, 8.0), vec2(3.0, 36.0)), CornerRadius::same(1), TAGS[l.tag as usize % 7]);
            }
            let eye = Rect::from_center_size(row.left_center() + vec2(15.0, 0.0), Vec2::splat(20.0));
            let ec = if l.visible { INK } else { FAINT };
            p.add(Shape::convex_polygon(vec![eye.left_center() + vec2(1.0, 0.0), eye.center() - vec2(0.0, 5.0), eye.right_center() - vec2(1.0, 0.0), eye.center() + vec2(0.0, 5.0)], Color32::TRANSPARENT, Stroke::new(1.3, ec)));
            if l.visible {
                p.circle_filled(eye.center(), 2.5, ec);
            } else {
                p.line_segment([eye.left_top() + vec2(3.0, 3.0), eye.right_bottom() - vec2(3.0, 3.0)], Stroke::new(1.3, ec));
            }
            let mut x = row.left() + 30.0;
            if l.clip {
                p.text(pos2(x + 4.0, row.center().y), Align2::LEFT_CENTER, "↳", FontId::proportional(14.0), MUTED);
                x += 14.0;
            }
            let thumb = Rect::from_min_size(pos2(x, row.top() + 7.0), Vec2::splat(38.0));
            if l.adjust.is_some() {
                p.rect_filled(thumb, CornerRadius::same(6), Color32::from_rgb(0x2c, 0x2c, 0x31));
                p.circle_stroke(thumb.center(), 10.0, Stroke::new(1.6, INK));
                p.add(Shape::convex_polygon(vec![thumb.center() + vec2(0.0, -10.0), thumb.center() + vec2(0.0, 10.0), thumb.center() + vec2(-8.5, 5.0), thumb.center() + vec2(-8.5, -5.0)], INK, Stroke::NONE));
            } else {
                p.image(self.checker.id(), thumb, Rect::from_min_max(pos2(0.0, 0.0), pos2(2.4, 2.4)), Color32::WHITE);
                if let Some((t, _)) = self.thumbs.get(i) {
                    let [tw, th] = t.size();
                    let s = (38.0 / tw as f32).min(38.0 / th as f32);
                    p.image(t.id(), Rect::from_center_size(thumb.center(), vec2(tw as f32, th as f32) * s), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
            }
            let editing_layer = selected && !(self.doc.edit_mask && l.mask.is_some());
            p.rect_stroke(thumb, CornerRadius::same(3), Stroke::new(if editing_layer { 1.5 } else { 1.0 }, if editing_layer { BLUE } else { LINE }), StrokeKind::Outside);
            let mut name_x = thumb.right() + 10.0;
            if l.mask.is_some() {
                let mr = Rect::from_min_size(pos2(thumb.right() + 6.0, thumb.top()), Vec2::splat(38.0));
                if let Some((_, Some(mt))) = self.thumbs.get(i) {
                    let [tw, th] = mt.size();
                    let s = (38.0 / tw as f32).min(38.0 / th as f32);
                    p.rect_filled(mr, CornerRadius::ZERO, Color32::BLACK);
                    p.image(mt.id(), Rect::from_center_size(mr.center(), vec2(tw as f32, th as f32) * s), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), if l.mask_on { Color32::WHITE } else { Color32::from_gray(90) });
                }
                let on = selected && self.doc.edit_mask;
                p.rect_stroke(mr, CornerRadius::same(3), Stroke::new(if on { 1.5 } else { 1.0 }, if on { BLUE } else { LINE }), StrokeKind::Outside);
                if ui.interact(mr, ui.make_persistent_id(("mthumb", i)), Sense::click()).on_hover_text("Clic : modifier le masque").clicked() {
                    mask_click = Some((i, true));
                }
                if ui.interact(thumb, ui.make_persistent_id(("lthumb", i)), Sense::click()).clicked() {
                    mask_click = Some((i, false));
                }
                name_x = mr.right() + 10.0;
            }
            let eye_resp = ui.interact(eye.expand(4.0), ui.make_persistent_id(("eye", i)), Sense::click());
            if eye_resp.on_hover_text("Afficher ou masquer (Alt : isoler)").clicked() {
                visibility = Some((i, ui.input(|x| x.modifiers.alt)));
            }
            let name_pos = pos2(name_x, row.center().y);
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
                let maxw = row.right() - 30.0 - name_pos.x;
                let g = ui.painter().layout(l.name.clone(), if selected { semibold(13.0) } else { FontId::proportional(13.0) }, if selected { INK_STRONG } else { INK }, maxw.max(20.0));
                ui.painter().galley(name_pos - vec2(0.0, 7.0 + g.size().y / 2.0), g, INK);
                let sub = format!("{:.0} %{}", l.opacity * 100.0, if l.blend != crate::doc::Blend::Normal { format!(" · {}", crate::doc::blend_name(l.blend)) } else { String::new() });
                ui.painter().text(name_pos + vec2(0.0, 10.0), Align2::LEFT_CENTER, sub, FontId::proportional(11.0), FAINT);
            }
            let lock_r = Rect::from_center_size(row.right_center() - vec2(16.0, 0.0), Vec2::splat(20.0));
            let locked = self.doc.layers[i].locked;
            if locked || resp.hovered() {
                lock_glyph(ui.painter(), lock_r, locked, if locked { INK } else { FAINT });
            }
            if ui.interact(lock_r.expand(3.0), ui.make_persistent_id(("lock", i)), Sense::click()).on_hover_text("Verrouiller le calque").clicked() {
                self.doc.layers[i].locked = !locked;
            } else if resp.double_clicked() {
                if self.doc.layers[i].adjust.is_some() {
                    edit_adjust = Some(i);
                } else {
                    self.renaming = Some((i, self.doc.layers[i].name.clone()));
                }
            } else if resp.clicked() {
                select = Some(i);
            }
            resp.context_menu(|ui| {
                self.doc.active = i;
                if ui.button("Renommer").clicked() {
                    self.renaming = Some((i, self.doc.layers[i].name.clone()));
                    ui.close();
                }
                if ui.button("Dupliquer").clicked() {
                    self.doc.duplicate_layer();
                    ui.close();
                }
                if ui.button("Supprimer").clicked() {
                    self.doc.delete_layer();
                    ui.close();
                }
                if ui.button("Sélectionner les pixels").clicked() {
                    self.doc.select_layer_alpha();
                    ui.close();
                }
                ui.separator();
                let clip = self.doc.layers[i].clip;
                if ui.add_enabled(i > 0, egui::Button::new(if clip { "Annuler l'écrêtage" } else { "Masque d'écrêtage" })).clicked() {
                    self.doc.layers[i].clip = !clip;
                    self.doc.touch_all();
                    ui.close();
                }
                if ui.add_enabled(i > 0, egui::Button::new("Fusionner avec le dessous")).clicked() {
                    self.doc.merge_down();
                    ui.close();
                }
                ui.menu_button("Couleur de l'étiquette", |ui| {
                    for (k, n) in TAG_NAMES.iter().enumerate() {
                        if ui.button(*n).clicked() {
                            self.doc.layers[i].tag = k as u8;
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Mode de fusion", |ui| {
                    for (b, n) in BLENDS {
                        if ui.selectable_label(self.doc.layers[i].blend == b, n).clicked() {
                            self.doc.layers[i].blend = b;
                            self.doc.touch_all();
                        }
                    }
                });
            });
        }
        if let Some((i, alt)) = visibility {
            if alt {
                let solo = (0..self.doc.layers.len()).all(|k| k == i || !self.doc.layers[k].visible);
                for k in 0..self.doc.layers.len() {
                    self.doc.layers[k].visible = solo || k == i;
                }
            } else {
                self.doc.layers[i].visible = !self.doc.layers[i].visible;
            }
            self.doc.touch_all();
        }
        if let Some(i) = select {
            if i != self.doc.active {
                self.doc.edit_mask = false;
            }
            self.doc.active = i;
        }
        if let Some((i, mask)) = mask_click {
            self.doc.active = i;
            self.doc.edit_mask = mask;
        }
        if let Some(i) = edit_adjust {
            self.edit_adjustment_layer(i);
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let b = |ui: &mut Ui, tip: &str, glyph: &str| {
                let (r, resp) = ui.allocate_exact_size(vec2(29.0, 28.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(7), Color32::from_white_alpha(14));
                }
                ui.painter().text(r.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(14.0), INK);
                resp.on_hover_text(tip)
            };
            if b(ui, "Nouveau calque", "+").clicked() {
                self.doc.next_label = Some("Nouveau calque".into());
                self.doc.add_layer(None, None);
            }
            if b(ui, "Dupliquer", "⧉").clicked() {
                self.doc.next_label = Some("Dupliquer le calque".into());
                self.doc.duplicate_layer();
            }
            if b(ui, "Ajouter un masque de fusion (depuis la sélection s'il y en a une)", "◐").clicked() && self.doc.layers[self.doc.active].mask.is_none() {
                self.doc.next_label = Some("Masque de fusion".into());
                self.doc.add_mask(if self.doc.selection.is_some() { 2 } else { 0 });
            }
            let fx = b(ui, "Effets de calque", "fx");
            egui::Popup::menu(&fx).show(|ui| {
                for (k, label) in [(0u8, "Ombre portée…"), (4, "Ombre interne…"), (1, "Lueur externe…"), (5, "Lueur interne…"), (6, "Biseau et estampage…"), (2, "Contour…"), (7, "Incrustation couleur…"), (8, "Incrustation dégradé…")] {
                    if ui.button(label).clicked() {
                        self.open_sheet_effect(k);
                    }
                }
            });
            let adj = b(ui, "Nouveau calque de réglage", "◑");
            egui::Popup::menu(&adj).show(|ui| {
                for k in crate::filters::ADJUSTMENTS {
                    if ui.button(k.title()).clicked() {
                        self.new_adjustment_layer(*k);
                    }
                }
            });
            if b(ui, "Monter", "↑").clicked() {
                self.doc.move_layer(true);
            }
            if b(ui, "Descendre", "↓").clicked() {
                self.doc.move_layer(false);
            }
            if b(ui, "Fusionner avec le dessous", "⤓").clicked() {
                self.doc.next_label = Some("Fusionner avec le dessous".into());
                self.doc.merge_down();
            }
            if b(ui, "Supprimer", "🗑").clicked() {
                self.doc.next_label = Some("Supprimer le calque".into());
                self.doc.delete_layer();
            }
        });

    }
}
