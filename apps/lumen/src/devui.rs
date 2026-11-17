use crate::develop::{self, Local, Mask, Part, PartKind, PartMode, Settings};
use studio_ui::i18n::tl;
use crate::tools::{Tool, part_name};
use crate::widgets::{caption, color_wheel, curve_editor, draw_histogram, heading, hue_slider, nav_row, pct, rgb, short, slider, slider_full, slider_track};
use crate::{Collection, Drag, Lumen, STAR, display_name, meta};
use eframe::egui::{self, Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use studio_ui::pro::{self, BLUE, FAINT, INK, INK_STRONG, MUTED, PANEL, semibold};

const ZOOMS: [(&str, f32); 6] = [("Ajuster", 0.0), ("25 %", 0.25), ("50 %", 0.5), ("100 %", 1.0), ("200 %", 2.0), ("400 %", 4.0)];
const COLD_WARM: [Color32; 2] = [Color32::from_rgb(70, 120, 255), Color32::from_rgb(255, 205, 60)];
const GREEN_MAGENTA: [Color32; 2] = [Color32::from_rgb(80, 200, 90), Color32::from_rgb(220, 80, 210)];
const DARK_LIGHT: [Color32; 2] = [Color32::from_rgb(20, 20, 20), Color32::from_rgb(235, 235, 240)];
const PANELS: [(&str, &str, bool); 14] = [
    ("basic", "Réglages de base", true),
    ("curve", "Courbe des tonalités", true),
    ("hsl", "TSL / Couleur", true),
    ("grading", "Étalonnage des couleurs", false),
    ("detail", "Détail", false),
    ("optics", "Optique", false),
    ("transform", "Transformation", false),
    ("geometry", "Recadrage et rotation", false),
    ("effects", "Effets", false),
    ("masks", "Masques", true),
    ("retouch", "Retouche", false),
    ("calib", "Étalonnage de l'appareil", false),
    ("actions", "Réglages", true),
    ("hist", "Histogramme", true),
];

fn hue_color(h: f32) -> Color32 {
    let c = develop::hsl_to_rgb(h, 0.8, 0.55);
    Color32::from_rgb((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8)
}

fn local_sliders(ui: &mut Ui, a: &mut Local) {
    slider_track(ui, tl("Température"), &mut a.temperature, -100.0..=100.0, 1.0, COLD_WARM);
    slider_track(ui, tl("Teinte"), &mut a.tint, -100.0..=100.0, 1.0, GREEN_MAGENTA);
    slider(ui, tl("Exposition"), &mut a.exposure, -4.0..=4.0, 0.01);
    slider(ui, tl("Contraste"), &mut a.contrast, -100.0..=100.0, 1.0);
    slider(ui, tl("Hautes lumières"), &mut a.highlights, -100.0..=100.0, 1.0);
    slider(ui, tl("Ombres"), &mut a.shadows, -100.0..=100.0, 1.0);
    slider(ui, tl("Blancs"), &mut a.whites, -100.0..=100.0, 1.0);
    slider(ui, tl("Noirs"), &mut a.blacks, -100.0..=100.0, 1.0);
    slider(ui, tl("Texture"), &mut a.texture, -100.0..=100.0, 1.0);
    slider(ui, tl("Clarté"), &mut a.clarity, -100.0..=100.0, 1.0);
    slider(ui, tl("Correction du voile"), &mut a.dehaze, -100.0..=100.0, 1.0);
    slider(ui, tl("Décalage de teinte"), &mut a.hue, -100.0..=100.0, 1.0);
    slider(ui, tl("Saturation"), &mut a.saturation, -100.0..=100.0, 1.0);
    slider(ui, tl("Netteté"), &mut a.sharpness, -100.0..=100.0, 1.0);
    slider(ui, tl("Bruit"), &mut a.noise, 0.0..=100.0, 1.0);
    hue_slider(ui, tl("Couleur"), &mut a.color_hue);
    slider_track(ui, tl("Intensité couleur"), &mut a.color_sat, 0.0..=100.0, 1.0, [Color32::from_gray(90), hue_color(a.color_hue)]);
}

fn kind_menu(ui: &mut Ui) -> Option<PartKind> {
    let mut out = None;
    for k in [PartKind::Linear, PartKind::Radial, PartKind::Brush, PartKind::Sky, PartKind::Subject, PartKind::Luminance, PartKind::Color, PartKind::All] {
        if ui.button(part_name(k)).clicked() {
            out = Some(k);
            ui.close();
        }
    }
    out
}

fn solo_dock(solo: bool, ui: &mut Ui, opened: &mut Option<&'static str>, id: &'static str, title: &str, add: impl FnOnce(&mut Ui)) {
        let key = ui.make_persistent_id(("dock", id));
        let default = PANELS.iter().find(|p| p.0 == id).map(|p| p.2).unwrap_or(true);
        let was = ui.ctx().data_mut(|d| *d.get_persisted_mut_or(key, default));
        pro::dock_panel(ui, id, title, add);
        let now = ui.ctx().data_mut(|d| *d.get_persisted_mut_or(key, default));
    if solo && !was && now {
        *opened = Some(id);
    }
}

impl Lumen {

    pub fn develop(&mut self, root: &mut Ui, ctx: &egui::Context) {
        let Some(i) = self.current else { return };
        self.filmstrip(root, ctx, i);
        if !self.hide_panels {
            self.left_panel(root, ctx, i);
            self.right_panel(root, ctx, i);
        }
        self.refresh_preview(ctx);
        self.central(root, ctx, i);
    }

    fn filmstrip(&mut self, root: &mut Ui, ctx: &egui::Context, i: usize) {
        egui::Panel::bottom("strip").exact_size(if self.hide_panels { 0.0 } else { 132.0 }).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 6))).show(root, |ui| {
            if self.hide_panels {
                return;
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let vis = self.visible();
                let pos = vis.iter().position(|v| *v == i).map(|p| p + 1).unwrap_or(0);
                ui.label(RichText::new(format!("{pos} / {} · {}", vis.len(), short(&display_name(&self.files[i]), 40))).size(12.0).color(MUTED));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    for r in (0..=5u8).rev() {
                        if ui.selectable_label(self.min_rating == r, if r == 0 { "Toutes".to_string() } else { format!("{r}★") }).clicked() {
                            self.min_rating = r;
                        }
                    }
                    ui.label(RichText::new(tl("Note")).size(12.0).color(FAINT));
                    for (c, l) in [(Collection::Edited, "Modifiées"), (Collection::Picks, "Retenues"), (Collection::All, "Toutes")] {
                        if ui.selectable_label(self.collection == c, l).clicked() {
                            self.collection = c;
                        }
                    }
                    ui.label(RichText::new(tl("Filtre")).size(12.0).color(FAINT));
                });
            });
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for v in self.visible() {
                        let (r, resp) = ui.allocate_exact_size(vec2(110.0, 90.0), Sense::click());
                        let p = ui.painter();
                        p.rect_filled(r, CornerRadius::same(4), Color32::from_rgb(0x22, 0x22, 0x22));
                        if let Some(t) = self.thumbs.get(&self.files[v]) {
                            let [tw, th] = t.size();
                            let s = ((r.width() - 8.0) / tw as f32).min((r.height() - 8.0) / th as f32);
                            p.image(t.id(), Rect::from_center_size(r.center(), vec2(tw as f32, th as f32) * s), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                        }
                        let e = self.get(v);
                        if e.rating > 0 {
                            p.text(r.left_bottom() + vec2(6.0, -6.0), Align2::LEFT_BOTTOM, "★".repeat(e.rating as usize), FontId::proportional(10.0), STAR);
                        }
                        if v == i {
                            p.rect_stroke(r, CornerRadius::same(4), Stroke::new(2.0, BLUE), StrokeKind::Inside);
                        }
                        if resp.on_hover_text(display_name(&self.files[v])).clicked() {
                            self.select(v, ctx);
                        }
                    }
                });
            });
        });
    }

    fn left_panel(&mut self, root: &mut Ui, ctx: &egui::Context, i: usize) {
        egui::Panel::left("presets").resizable(false).exact_size(236.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 12))).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                self.navigator(ui);
                ui.add_space(10.0);
                heading(ui, tl("PRÉRÉGLAGES"));
                ui.spacing_mut().slider_width = 96.0;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tl("Quantité")).size(12.0).color(MUTED));
                    if ui.add(egui::Slider::new(&mut self.preset_amount, 0.0..=200.0).suffix(" %")).changed()
                        && let Some((base, preset)) = self.last_preset.clone()
                    {
                        let mut s = meta::blend(&Settings::default(), &preset, self.preset_amount / 100.0);
                        (s.crop, s.rotate, s.straighten, s.flip, s.spots, s.red_eyes, s.masks) = (base.crop, base.rotate, base.straighten, base.flip, base.spots.clone(), base.red_eyes.clone(), base.masks.clone());
                        self.set_settings(i, s);
                    }
                });
                ui.add_space(4.0);
                let current = self.settings(i);
                let catalog = develop::preset_catalog();
                let mut cats: Vec<&str> = Vec::new();
                for c in catalog.iter().map(|c| c.0) {
                    if !cats.contains(&c) {
                        cats.push(c);
                    }
                }
                for cat in cats {
                    egui::CollapsingHeader::new(RichText::new(tl(&cat)).size(12.0).color(INK)).id_salt(("cat", cat)).default_open(cat == "Couleur").show(ui, |ui| {
                        for (_, name, st) in catalog.iter().filter(|c| c.0 == cat) {
                            let on = current == *st;
                            let r = nav_row(ui, name, usize::MAX, on);
                            let fav = self.catalog.favorites.iter().any(|f| f == name);
                            if r.clicked() {
                                self.apply_preset(st);
                            }
                            r.context_menu(|ui| {
                                if ui.button(if fav { "Retirer des favoris" } else { "Ajouter aux favoris" }).clicked() {
                                    if fav {
                                        self.catalog.favorites.retain(|f| f != name);
                                    } else {
                                        self.catalog.favorites.push(name.to_string());
                                    }
                                    self.dirty_catalog = true;
                                    ui.close();
                                }
                            });
                        }
                    });
                }
                let favs: Vec<(&str, Settings)> = catalog.iter().filter(|c| self.catalog.favorites.iter().any(|f| f == c.1)).map(|c| (c.1, c.2.clone())).collect();
                if !favs.is_empty() {
                    egui::CollapsingHeader::new(RichText::new(tl("★ Favoris")).size(12.0).color(INK)).id_salt("cat-fav").default_open(true).show(ui, |ui| {
                        for (name, st) in favs {
                            if nav_row(ui, name, usize::MAX, current == st).clicked() {
                                self.apply_preset(&st);
                            }
                        }
                    });
                }
                egui::CollapsingHeader::new(RichText::new(tl("Mes préréglages")).size(12.0).color(INK)).id_salt("cat-user").default_open(true).show(ui, |ui| {
                    let mut remove = None;
                    for (k, (name, st)) in self.catalog.user_presets.clone().into_iter().enumerate() {
                        let r = nav_row(ui, &name, usize::MAX, current == st).on_hover_text(tl("Clic droit : options"));
                        if r.clicked() {
                            self.apply_preset(&st);
                        }
                        r.context_menu(|ui| {
                            if ui.button(tl("Mettre à jour avec les réglages actuels")).clicked() {
                                self.catalog.user_presets[k].1 = current.clone();
                                self.dirty_catalog = true;
                                ui.close();
                            }
                            if ui.button(tl("Supprimer ce préréglage")).clicked() {
                                remove = Some(k);
                                ui.close();
                            }
                        });
                    }
                    if let Some(k) = remove {
                        self.catalog.user_presets.remove(k);
                        self.dirty_catalog = true;
                    }
                    match &mut self.preset_name {
                        Some(name) => {
                            let r = ui.add(egui::TextEdit::singleline(name).hint_text(tl("Nom du préréglage")).desired_width(f32::INFINITY));
                            r.request_focus();
                            if r.lost_focus() {
                                let n = name.trim().to_string();
                                if !n.is_empty() {
                                    let mut keep = current.clone();
                                    (keep.crop, keep.straighten, keep.rotate, keep.spots, keep.red_eyes) = ([0.0; 4], 0.0, 0, Vec::new(), Vec::new());
                                    self.catalog.user_presets.push((n, keep));
                                    self.dirty_catalog = true;
                                }
                                self.preset_name = None;
                            }
                        }
                        None => {
                            if pro::ghost(ui, tl("+ Enregistrer les réglages actuels")).clicked() {
                                self.preset_name = Some(String::new());
                            }
                        }
                    }
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    heading(ui, tl("INSTANTANÉS"));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("+").on_hover_text(tl("Nouvel instantané (Ctrl+N)")).clicked() {
                            self.snapshot_name = Some(format!("Instantané {}", self.get(i).snapshots.len() + 1));
                        }
                    });
                });
                if let Some(name) = &mut self.snapshot_name {
                    let r = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
                    r.request_focus();
                    if r.lost_focus() {
                        let n = name.clone();
                        self.snapshot_name = None;
                        self.add_snapshot(i, Some(n));
                    }
                }
                let snaps = self.get(i).snapshots;
                if snaps.is_empty() {
                    caption(ui, tl("Mémorise une version pour y revenir."));
                }
                let mut remove = None;
                for (k, (name, st)) in snaps.iter().enumerate() {
                    let r = nav_row(ui, name, usize::MAX, current == *st).on_hover_text(tl("Clic : appliquer · clic droit : options"));
                    if r.clicked() {
                        self.set_settings(i, st.clone());
                    }
                    r.context_menu(|ui| {
                        if ui.button(tl("Remplacer par les réglages actuels")).clicked() {
                            self.entry(i).snapshots[k].1 = current.clone();
                            self.dirty_catalog = true;
                            ui.close();
                        }
                        if ui.button(tl("Supprimer")).clicked() {
                            remove = Some(k);
                            ui.close();
                        }
                    });
                }
                if let Some(k) = remove {
                    self.entry(i).snapshots.remove(k);
                    self.dirty_catalog = true;
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    heading(ui, tl("HISTORIQUE"));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button(tl("Effacer")).clicked() {
                            self.history.retain(|h| h.0 != i);
                            self.future.retain(|h| h.0 != i);
                        }
                    });
                });
                let steps: Vec<(usize, String)> = self.history.iter().enumerate().filter(|(_, h)| h.0 == i).map(|(k, h)| (k, h.2.clone())).collect();
                if steps.is_empty() {
                    caption(ui, tl("Chaque réglage apparaît ici."));
                }
                let mut target = None;
                if nav_row(ui, tl("État actuel"), usize::MAX, true).clicked() {}
                for (n, (k, label)) in steps.iter().enumerate().rev().take(40) {
                    if nav_row(ui, &format!("{}. {label}", n + 1), usize::MAX, false).on_hover_text(tl("Revenir à l'état avant cette étape")).clicked() {
                        target = Some(*k);
                    }
                }
                if let Some(k) = target {
                    while self.history.len() > k {
                        self.undo();
                    }
                }
                if !steps.is_empty() {
                    let base = self.history.iter().find(|h| h.0 == i).map(|h| h.1.clone());
                    if nav_row(ui, tl("Importation (original)"), usize::MAX, false).clicked()
                        && let Some(b) = base
                    {
                        self.set_settings(i, b);
                    }
                }
                let _ = ctx;
            });
        });
    }

    fn navigator(&mut self, ui: &mut Ui) {
        heading(ui, tl("NAVIGATEUR"));
        let (box_r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 132.0), Sense::click_and_drag());
        ui.painter().rect_filled(box_r, CornerRadius::same(4), Color32::from_rgb(0x14, 0x14, 0x14));
        let Some(t) = &self.preview else { return };
        let [w, h] = t.size();
        let s = (box_r.width() / w as f32).min(box_r.height() / h as f32);
        let ir = Rect::from_center_size(box_r.center(), vec2(w as f32, h as f32) * s);
        ui.painter().image(t.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        if self.zoom > 0.0
            && let Some((area, r)) = self.last_view
        {
            let vis = area.intersect(r);
            let f = |p: egui::Pos2| ir.min + vec2((p.x - r.left()) / r.width() * ir.width(), (p.y - r.top()) / r.height() * ir.height());
            ui.painter().rect_stroke(Rect::from_min_max(f(vis.min), f(vis.max)), CornerRadius::same(2), Stroke::new(1.5, Color32::WHITE), StrokeKind::Middle);
            if let Some(p) = resp.interact_pointer_pos()
                && (resp.clicked() || resp.dragged())
            {
                let (fu, fv) = (((p.x - ir.left()) / ir.width()).clamp(0.0, 1.0), ((p.y - ir.top()) / ir.height()).clamp(0.0, 1.0));
                self.pan = vec2(r.width() * (0.5 - fu), r.height() * (0.5 - fv));
            }
        } else if resp.clicked() {
            self.zoom = 1.0;
            if let Some(p) = resp.interact_pointer_pos()
                && let Some((_, r)) = self.last_view
            {
                let (fu, fv) = ((p.x - ir.left()) / ir.width(), (p.y - ir.top()) / ir.height());
                let sc = 1.0 / ui.ctx().pixels_per_point() * w as f32 / r.width().max(1.0);
                self.pan = vec2(r.width() * sc * (0.5 - fu), r.height() * sc * (0.5 - fv));
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0);
            for (n, z) in [ZOOMS[0], ZOOMS[2], ZOOMS[3], ZOOMS[4]] {
                if ui.selectable_label(self.zoom == z, RichText::new(tl(&n)).size(11.0)).clicked() {
                    self.zoom = z;
                    self.pan = Vec2::ZERO;
                }
            }
        });
    }

    fn right_panel(&mut self, root: &mut Ui, ctx: &egui::Context, i: usize) {
        egui::Panel::right("dev").resizable(false).exact_size(312.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let mut opened: Option<&'static str> = None;
                let solo = self.catalog.solo;
                let mut s = self.settings(i);
                let start = s.clone();
                let hist = self.hist;
                let hover = self.hover_rgb;
                let mut clipping = self.clipping;
                solo_dock(solo, ui, &mut opened, "hist", "Histogramme", |ui| {
                    draw_histogram(ui, hist.as_ref(), (clipping, clipping));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        match hover {
                            Some(c) => caption(ui, &format!("R {:.0} %   V {:.0} %   B {:.0} %", c[0] as f32 / 2.55, c[1] as f32 / 2.55, c[2] as f32 / 2.55)),
                            None => caption(ui, tl("Survole la photo pour lire les valeurs RVB")),
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.toggle_value(&mut clipping, RichText::new(tl("Écrêtage")).size(11.5)).on_hover_text(tl("J"));
                        });
                    });
                });
                if clipping != self.clipping {
                    self.clipping = clipping;
                    self.shown = None;
                }
                let src = self.source.as_ref().map(|(_, s)| s.clone());
                let mut tool_req: Option<Tool> = None;
                solo_dock(solo, ui, &mut opened, "basic", "Réglages de base", |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tl("Profil")).size(12.0).color(MUTED));
                        egui::ComboBox::from_id_salt("profile").width(120.0).selected_text(develop::PROFILES[(s.profile as usize).min(5)]).show_ui(ui, |ui| {
                            for (k, p) in develop::PROFILES.iter().enumerate() {
                                ui.selectable_value(&mut s.profile, k as u8, *p);
                            }
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if pro::ghost(ui, tl("Auto")).on_hover_text(tl("Ctrl+U")).clicked()
                                && let Some(src) = &src
                            {
                                let a = develop::auto(src);
                                (s.exposure, s.contrast, s.shadows, s.highlights, s.vibrance) = (a.exposure, a.contrast, a.shadows, a.highlights, a.vibrance);
                            }
                        });
                    });
                    let mut bw = s.mono as usize;
                    if pro::segmented(ui, &mut bw, &[tl("Couleur"), tl("Noir et blanc")]) {
                        s.mono = bw == 1;
                    }
                    ui.add_space(6.0);
                    caption(ui, tl("Balance des blancs"));
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            if ui.small_button(tl("Pipette")).on_hover_text(tl("W")).clicked() {
                                tool_req = Some(Tool::WbPicker);
                            }
                            if ui.small_button(tl("Auto")).clicked()
                                && let Some(src) = &src
                            {
                                (s.temperature, s.tint) = develop::auto_white_balance(src);
                            }
                            ui.menu_button(RichText::new(tl("Préréglage")).size(12.0), |ui| {
                                if ui.button(tl("Tel quel")).clicked() {
                                    (s.temperature, s.tint) = (0.0, 0.0);
                                }
                                for (name, t, ti) in develop::WB_PRESETS {
                                    if ui.button(tl(&name)).clicked() {
                                        (s.temperature, s.tint) = (t, ti);
                                    }
                                }
                            });
                        });
                    });
                    slider_track(ui, tl("Température"), &mut s.temperature, -100.0..=100.0, 1.0, COLD_WARM);
                    slider_track(ui, tl("Teinte"), &mut s.tint, -100.0..=100.0, 1.0, GREEN_MAGENTA);
                    ui.add_space(6.0);
                    caption(ui, tl("Tonalité"));
                    slider_full(ui, tl("Exposition"), &mut s.exposure, -4.0..=4.0, 0.01, 0.0, None, " IL");
                    slider(ui, tl("Contraste"), &mut s.contrast, -100.0..=100.0, 1.0);
                    slider(ui, tl("Hautes lumières"), &mut s.highlights, -100.0..=100.0, 1.0);
                    slider(ui, tl("Ombres"), &mut s.shadows, -100.0..=100.0, 1.0);
                    slider(ui, tl("Blancs"), &mut s.whites, -100.0..=100.0, 1.0);
                    slider(ui, tl("Noirs"), &mut s.blacks, -100.0..=100.0, 1.0);
                    ui.add_space(6.0);
                    caption(ui, tl("Présence"));
                    slider(ui, tl("Texture"), &mut s.texture, -100.0..=100.0, 1.0);
                    slider(ui, tl("Clarté"), &mut s.clarity, -100.0..=100.0, 1.0);
                    slider(ui, tl("Correction du voile"), &mut s.dehaze, -100.0..=100.0, 1.0);
                    slider(ui, tl("Vibrance"), &mut s.vibrance, -100.0..=100.0, 1.0);
                    slider(ui, tl("Saturation"), &mut s.saturation, -100.0..=100.0, 1.0);
                });
                let mut channel = self.curve_channel;
                solo_dock(solo, ui, &mut opened, "curve", "Courbe des tonalités", |ui| {
                    pro::segmented(ui, &mut channel, &[tl("Param."), tl("RVB"), tl("R"), tl("V"), tl("B")]);
                    ui.add_space(6.0);
                    let lum = hist.as_ref().map(|h| h[3]);
                    match channel {
                        0 => {
                            let mut dummy = s.curve_rgb.clone();
                            curve_editor(ui, &mut dummy, INK, Some(&s.curve), lum.as_ref());
                            for (k, label) in ["Ombres", "Tons foncés", "Tons clairs", "Hautes lumières"].into_iter().enumerate() {
                                slider(ui, label, &mut s.curve[k], -100.0..=100.0, 1.0);
                            }
                        }
                        c => {
                            let (pts, ink) = match c {
                                1 => (&mut s.curve_rgb, INK),
                                2 => (&mut s.curve_r, Color32::from_rgb(255, 90, 90)),
                                3 => (&mut s.curve_g, Color32::from_rgb(90, 220, 110)),
                                _ => (&mut s.curve_b, Color32::from_rgb(90, 150, 255)),
                            };
                            curve_editor(ui, pts, ink, None, lum.as_ref());
                            ui.horizontal_wrapped(|ui| {
                                for (name, p) in [("Linéaire", vec![]), ("Contraste moyen", vec![[0.0, 0.0], [0.25, 0.21], [0.75, 0.8], [1.0, 1.0]]), ("Contraste fort", vec![[0.0, 0.0], [0.25, 0.17], [0.75, 0.85], [1.0, 1.0]]), ("Film mat", vec![[0.0, 0.08], [0.3, 0.3], [0.8, 0.82], [1.0, 0.95]]), ("Inversion", vec![[0.0, 1.0], [1.0, 0.0]])] {
                                    if ui.small_button(tl(&name)).clicked() {
                                        *pts = p;
                                    }
                                }
                            });
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui.small_button(tl("Réglage ciblé")).on_hover_text(tl("T : glisse sur la photo")).clicked() {
                            tool_req = Some(Tool::Target);
                        }
                        if ui.small_button(tl("Réinitialiser la courbe")).clicked() {
                            (s.curve, s.curve_rgb, s.curve_r, s.curve_g, s.curve_b) = ([0.0; 4], vec![], vec![], vec![], vec![]);
                        }
                    });
                });
                self.curve_channel = channel;
                let mut tab = self.hsl_tab;
                solo_dock(solo, ui, &mut opened, "hsl", "TSL / Couleur", |ui| {
                    pro::segmented(ui, &mut tab, &[tl("Teinte"), tl("Saturation"), tl("Luminance"), tl("Tout")]);
                    ui.add_space(6.0);
                    let tabs: Vec<usize> = if tab == 3 { vec![0, 1, 2] } else { vec![tab] };
                    for t in tabs {
                        if tab == 3 {
                            caption(ui, ["Teinte", "Saturation", "Luminance"][t]);
                        }
                        for (b, (name, hue, c)) in develop::BANDS.iter().enumerate() {
                            let track = match t {
                                0 => [hue_color(hue - 30.0), hue_color(hue + 30.0)],
                                1 => [Color32::from_gray(120), rgb(*c)],
                                _ => [Color32::from_gray(30), rgb(*c).lerp_to_gamma(Color32::WHITE, 0.4)],
                            };
                            slider_track(ui, name, &mut s.hsl[b][t], -100.0..=100.0, 1.0, track);
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui.small_button(tl("Réglage ciblé")).clicked() {
                            self.target_mode = tab.min(2) + 1;
                            tool_req = Some(Tool::Target);
                        }
                        if ui.small_button(tl("Réinitialiser")).clicked() {
                            s.hsl = [[0.0; 3]; 8];
                        }
                    });
                    if s.mono {
                        caption(ui, tl("En noir et blanc, la luminance des couleurs règle leur gris (mélangeur N&B)."));
                    }
                });
                self.hsl_tab = tab;
                solo_dock(solo, ui, &mut opened, "grading", "Étalonnage des couleurs", |ui| {
                    let w = (ui.available_width() / 3.0 - 6.0).min(90.0);
                    ui.columns(3, |cols| {
                        color_wheel(&mut cols[0], "Ombres", w, &mut s.split_shadow_hue, &mut s.split_shadow_sat);
                        color_wheel(&mut cols[1], "Tons moyens", w, &mut s.split_mid_hue, &mut s.split_mid_sat);
                        color_wheel(&mut cols[2], "Hautes lum.", w, &mut s.split_high_hue, &mut s.split_high_sat);
                    });
                    ui.add_space(4.0);
                    slider_track(ui, tl("Lum. ombres"), &mut s.grade_shadow_lum, -100.0..=100.0, 1.0, DARK_LIGHT);
                    slider_track(ui, tl("Lum. tons moyens"), &mut s.grade_mid_lum, -100.0..=100.0, 1.0, DARK_LIGHT);
                    slider_track(ui, tl("Lum. hautes lum."), &mut s.grade_high_lum, -100.0..=100.0, 1.0, DARK_LIGHT);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        color_wheel(ui, tl("Global"), 90.0, &mut s.grade_global_hue, &mut s.grade_global_sat);
                    });
                    slider_track(ui, tl("Lum. globale"), &mut s.grade_global_lum, -100.0..=100.0, 1.0, DARK_LIGHT);
                    let mut blend = s.grade_blend + 50.0;
                    if slider_full(ui, tl("Fusion"), &mut blend, 0.0..=100.0, 1.0, 50.0, None, "") {
                        s.grade_blend = blend - 50.0;
                    }
                    slider(ui, tl("Équilibre"), &mut s.split_balance, -100.0..=100.0, 1.0);
                });
                solo_dock(solo, ui, &mut opened, "detail", "Détail", |ui| {
                    caption(ui, tl("Netteté"));
                    slider(ui, tl("Gain"), &mut s.sharpen, 0.0..=150.0, 1.0);
                    slider_full(ui, tl("Rayon"), &mut s.sharpen_radius, 0.0..=3.0, 0.1, 0.0, None, " px");
                    slider(ui, tl("Détail"), &mut s.sharpen_detail, 0.0..=100.0, 1.0);
                    slider(ui, tl("Masquage"), &mut s.sharpen_masking, 0.0..=100.0, 1.0);
                    ui.add_space(6.0);
                    caption(ui, tl("Réduction du bruit"));
                    slider(ui, tl("Luminance"), &mut s.noise, 0.0..=100.0, 1.0);
                    slider(ui, tl("Détail lum."), &mut s.noise_detail, 0.0..=100.0, 1.0);
                    slider(ui, tl("Couleur"), &mut s.color_noise, 0.0..=100.0, 1.0);
                    slider(ui, tl("Détail couleur"), &mut s.color_noise_detail, 0.0..=100.0, 1.0);
                    slider(ui, tl("Lissage"), &mut s.color_noise_smooth, 0.0..=100.0, 1.0);
                });
                solo_dock(solo, ui, &mut opened, "optics", "Optique", |ui| {
                    caption(ui, tl("Vignetage de l'objectif"));
                    slider(ui, tl("Quantité"), &mut s.lens_vignette, -100.0..=100.0, 1.0);
                    slider(ui, tl("Point milieu"), &mut s.lens_vignette_mid, -100.0..=100.0, 1.0);
                    slider(ui, tl("Distorsion"), &mut s.distortion, -100.0..=100.0, 1.0);
                    ui.add_space(6.0);
                    caption(ui, tl("Aberration chromatique"));
                    slider_track(ui, tl("Rouge / cyan"), &mut s.ca_red, -100.0..=100.0, 1.0, [Color32::from_rgb(60, 210, 220), Color32::from_rgb(240, 70, 70)]);
                    slider_track(ui, tl("Bleu / jaune"), &mut s.ca_blue, -100.0..=100.0, 1.0, [Color32::from_rgb(240, 210, 60), Color32::from_rgb(70, 110, 250)]);
                    slider(ui, tl("Frange violette"), &mut s.defringe_purple, 0.0..=100.0, 1.0);
                    slider(ui, tl("Frange verte"), &mut s.defringe_green, 0.0..=100.0, 1.0);
                    if ui.small_button(tl("Supprimer l'aberration (auto)")).on_hover_text(tl("Applique une correction de frange standard")).clicked() {
                        (s.defringe_purple, s.defringe_green) = (40.0, 25.0);
                    }
                });
                solo_dock(solo, ui, &mut opened, "transform", "Transformation", |ui| {
                    ui.horizontal(|ui| {
                        if ui.small_button(tl("Niveau auto")).clicked()
                            && let Some(src) = &src
                        {
                            s.straighten = develop::auto_straighten(src);
                        }
                        if ui.small_button(tl("Règle")).on_hover_text(tl("Trace une ligne sur l'horizon")).clicked() {
                            tool_req = Some(Tool::Straighten);
                        }
                        if ui.small_button(tl("Réinitialiser")).clicked() {
                            (s.persp_v, s.persp_h, s.persp_rotate, s.persp_aspect, s.persp_scale, s.persp_x, s.persp_y) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
                        }
                    });
                    slider(ui, tl("Verticale"), &mut s.persp_v, -100.0..=100.0, 1.0);
                    slider(ui, tl("Horizontale"), &mut s.persp_h, -100.0..=100.0, 1.0);
                    slider_full(ui, tl("Rotation"), &mut s.persp_rotate, -10.0..=10.0, 0.1, 0.0, None, "°");
                    slider(ui, tl("Aspect"), &mut s.persp_aspect, -100.0..=100.0, 1.0);
                    let mut scale = s.persp_scale + 100.0;
                    if slider_full(ui, tl("Échelle"), &mut scale, 50.0..=150.0, 1.0, 100.0, None, " %") {
                        s.persp_scale = scale - 100.0;
                    }
                    slider(ui, tl("Décalage X"), &mut s.persp_x, -100.0..=100.0, 0.5);
                    slider(ui, tl("Décalage Y"), &mut s.persp_y, -100.0..=100.0, 0.5);
                    ui.checkbox(&mut s.flip_v, tl("Miroir vertical"));
                });
                solo_dock(solo, ui, &mut opened, "geometry", "Recadrage et rotation", |ui| {
                    ui.horizontal(|ui| {
                        if pro::secondary(ui, tl("Outil Recadrage")).on_hover_text(tl("R")).clicked() {
                            tool_req = Some(Tool::Crop);
                        }
                        if ui.small_button(tl("Auto")).on_hover_text(tl("Redressement automatique")).clicked()
                            && let Some(src) = &src
                        {
                            s.straighten = develop::auto_straighten(src);
                        }
                    });
                    slider_full(ui, tl("Redresser"), &mut s.straighten, -45.0..=45.0, 0.1, 0.0, None, "°");
                    for (k, label) in ["Marge gauche", "Marge haute", "Marge droite", "Marge basse"].into_iter().enumerate() {
                        pct(ui, label, &mut s.crop[k], 0.0, 0.45, 0.0);
                    }
                    caption(ui, tl("Format"));
                    ui.horizontal_wrapped(|ui| {
                        for (k, (name, _)) in develop::RATIOS.iter().enumerate() {
                            if ui.small_button(tl(*name)).clicked() {
                                self.crop_ratio = k;
                                let mut t = s.clone();
                                self.apply_ratio(&mut t);
                                s.crop = t.crop;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut s.flip, tl("Miroir horizontal"));
                        if ui.small_button(tl("⟲ 90°")).clicked() {
                            s.rotate = (s.rotate + 3) % 4;
                        }
                        if ui.small_button(tl("⟳ 90°")).clicked() {
                            s.rotate = (s.rotate + 1) % 4;
                        }
                    });
                });
                solo_dock(solo, ui, &mut opened, "effects", "Effets", |ui| {
                    caption(ui, tl("Vignettage après recadrage"));
                    egui::ComboBox::from_id_salt("vig-style").width(ui.available_width() - 4.0).selected_text(develop::VIGNETTE_STYLES[(s.vignette_style as usize).min(2)]).show_ui(ui, |ui| {
                        for (k, n) in develop::VIGNETTE_STYLES.iter().enumerate() {
                            ui.selectable_value(&mut s.vignette_style, k as u8, *n);
                        }
                    });
                    slider_track(ui, tl("Quantité"), &mut s.vignette, -100.0..=100.0, 1.0, DARK_LIGHT);
                    slider(ui, tl("Point milieu"), &mut s.vignette_mid, 0.0..=100.0, 1.0);
                    slider(ui, tl("Arrondi"), &mut s.vignette_round, -100.0..=100.0, 1.0);
                    let mut f = s.vignette_feather + 50.0;
                    if slider_full(ui, tl("Contour progressif"), &mut f, 0.0..=100.0, 1.0, 50.0, None, "") {
                        s.vignette_feather = f - 50.0;
                    }
                    if s.vignette_style == 1 {
                        slider(ui, tl("Hautes lumières"), &mut s.vignette_highlights, 0.0..=100.0, 1.0);
                    }
                    ui.add_space(6.0);
                    caption(ui, tl("Grain"));
                    slider(ui, tl("Quantité "), &mut s.grain, 0.0..=100.0, 1.0);
                    slider(ui, tl("Taille"), &mut s.grain_size, 0.0..=100.0, 1.0);
                    slider(ui, tl("Rugosité"), &mut s.grain_rough, 0.0..=100.0, 1.0);
                    ui.add_space(6.0);
                    caption(ui, tl("Style"));
                    slider(ui, tl("Halo lumineux"), &mut s.glow, 0.0..=100.0, 1.0);
                    slider(ui, tl("Délavé"), &mut s.fade, 0.0..=100.0, 1.0);
                });
                solo_dock(solo, ui, &mut opened, "masks", "Masques", |ui| self.masks_panel(ui, &mut s, &mut tool_req));
                solo_dock(solo, ui, &mut opened, "retouch", "Retouche", |ui| {
                    pro::label_value(ui, tl("Corrections de taches"), &s.spots.len().to_string());
                    pro::label_value(ui, tl("Yeux rouges corrigés"), &s.red_eyes.len().to_string());
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button(tl("Suppression (Q)")).clicked() {
                            tool_req = Some(Tool::Remove);
                        }
                        if ui.small_button(tl("Correcteur")).clicked() {
                            tool_req = Some(Tool::Heal);
                        }
                        if ui.small_button(tl("Clonage")).clicked() {
                            tool_req = Some(Tool::Clone);
                        }
                        if ui.small_button(tl("Yeux rouges (E)")).clicked() {
                            tool_req = Some(Tool::RedEye);
                        }
                    });
                    ui.horizontal(|ui| {
                        if !s.spots.is_empty() && ui.small_button(tl("Effacer les taches")).clicked() {
                            s.spots.clear();
                        }
                        if !s.red_eyes.is_empty() && ui.small_button(tl("Effacer les yeux")).clicked() {
                            s.red_eyes.clear();
                        }
                    });
                });
                solo_dock(solo, ui, &mut opened, "calib", "Étalonnage de l'appareil", |ui| {
                    slider_track(ui, tl("Teinte des ombres"), &mut s.cal_shadow_tint, -100.0..=100.0, 1.0, GREEN_MAGENTA);
                    for (name, h0, hue, sat) in [("Primaire rouge", 0.0f32, &mut s.cal_red_hue, &mut s.cal_red_sat), ("Primaire verte", 120.0, &mut s.cal_green_hue, &mut s.cal_green_sat), ("Primaire bleue", 240.0, &mut s.cal_blue_hue, &mut s.cal_blue_sat)] {
                        caption(ui, name);
                        slider_track(ui, tl("Teinte"), hue, -100.0..=100.0, 1.0, [hue_color(h0 - 30.0), hue_color(h0 + 30.0)]);
                        slider_track(ui, tl("Saturation"), sat, -100.0..=100.0, 1.0, [Color32::from_gray(120), hue_color(h0)]);
                    }
                });
                solo_dock(solo, ui, &mut opened, "actions", "Réglages", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if pro::secondary(ui, tl("?  Tutoriels")).clicked() {
                            self.help.learn("Développement");
                        }
                        if pro::secondary(ui, tl("Copier…")).on_hover_text(tl("Ctrl+Maj+C : choisir les groupes")).clicked() {
                            self.clipboard = Some(s.clone());
                            self.copy_sheet = true;
                        }
                        if self.clipboard.is_some()
                            && pro::secondary(ui, tl("Coller")).clicked()
                            && let Some(c) = &self.clipboard
                        {
                            s = meta::copy_groups(&s, c, &self.copy_groups);
                        }
                        if pro::secondary(ui, tl("Réinitialiser")).clicked() {
                            s = Settings::default();
                        }
                    });
                    ui.add_space(4.0);
                    ui.menu_button(RichText::new(tl("Réinitialiser un groupe…")).size(12.0), |ui| {
                        for (k, g) in meta::GROUPS.iter().enumerate() {
                            if ui.button(tl(*g)).clicked() {
                                let mut only = vec![false; meta::GROUPS.len()];
                                only[k] = true;
                                s = meta::copy_groups(&s, &Settings::default(), &only);
                                ui.close();
                            }
                        }
                    });
                    ui.checkbox(&mut self.catalog.solo, tl("Mode solo : un seul panneau ouvert"));
                });
                solo_dock(solo, ui, &mut opened, "creative", "Effets créatifs", |ui| {
                    studio_ui::fxpanel::effects_panel(ui, "lumen", &mut s.creative);
                });
                if let Some(id) = opened {
                    for (pid, _, _) in PANELS {
                        if pid != id {
                            let key = ui.make_persistent_id(("dock", pid));
                            ui.ctx().data_mut(|d| d.insert_persisted(key, false));
                        }
                    }
                }
                if s != start {
                    self.set_settings(i, s);
                }
                if let Some(t) = tool_req {
                    self.set_tool(t, ctx);
                }
            });
        });
    }

    fn masks_panel(&mut self, ui: &mut Ui, s: &mut Settings, tool_req: &mut Option<Tool>) {
        ui.spacing_mut().item_spacing.y = 3.0;
        ui.horizontal(|ui| {
            ui.menu_button(RichText::new(tl("+ Nouveau masque")).size(12.0).color(BLUE), |ui| {
                if let Some(k) = kind_menu(ui) {
                    match k {
                        PartKind::Linear => *tool_req = Some(Tool::Linear),
                        PartKind::Radial => *tool_req = Some(Tool::Radial),
                        PartKind::Brush => {
                            s.masks.push(Mask { name: "Pinceau".into(), parts: vec![Part::new(PartKind::Brush)], ..Default::default() });
                            self.sel_mask = Some(s.masks.len() - 1);
                            *tool_req = Some(Tool::Brush);
                        }
                        _ => {
                            s.masks.push(Mask { name: part_name(k).into(), parts: vec![Part::new(k)], ..Default::default() });
                            self.sel_mask = Some(s.masks.len() - 1);
                        }
                    }
                    self.sel_part = 0;
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.checkbox(&mut self.show_overlay, RichText::new(tl("Superposition")).size(12.0)).on_hover_text(tl("O"));
            });
        });
        if s.masks.is_empty() {
            caption(ui, tl("Filtre gradué (M), radial (Maj+M), pinceau (K), ciel ou sujet : chaque masque a tous ses réglages."));
            return;
        }
        let mut remove = None;
        let mut dup = None;
        for (k, m) in s.masks.iter_mut().enumerate() {
            let sel = self.sel_mask == Some(k);
            ui.horizontal(|ui| {
                ui.checkbox(&mut m.on, "").on_hover_text(tl("Activer ou masquer"));
                let name = if m.name.is_empty() { format!("Masque {}", k + 1) } else { m.name.clone() };
                if ui.add(egui::Button::selectable(sel, RichText::new(short(&name, 20)).size(12.0).color(if sel { INK_STRONG } else { INK }))).clicked() {
                    self.sel_mask = if sel { None } else { Some(k) };
                    self.sel_part = 0;
                    self.rename_mask = None;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("×").on_hover_text(tl("Supprimer le masque")).clicked() {
                        remove = Some(k);
                    }
                    if ui.small_button("⧉").on_hover_text(tl("Dupliquer")).clicked() {
                        dup = Some(k);
                    }
                });
            });
        }
        if let Some(k) = dup {
            let mut m = s.masks[k].clone();
            m.name = format!("{} (copie)", if m.name.is_empty() { "Masque" } else { &m.name });
            s.masks.push(m);
            self.sel_mask = Some(s.masks.len() - 1);
        }
        if let Some(k) = remove {
            s.masks.remove(k);
            self.sel_mask = None;
            return;
        }
        let Some(mi) = self.sel_mask.filter(|m| *m < s.masks.len()) else { return };
        ui.add_space(6.0);
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(r, CornerRadius::ZERO, Color32::from_white_alpha(20));
        ui.add_space(4.0);
        let m = &mut s.masks[mi];
        ui.horizontal(|ui| {
            ui.label(RichText::new(tl("Nom")).size(12.0).color(MUTED));
            ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(140.0));
            ui.checkbox(&mut m.invert, tl("Inverser"));
        });
        slider_full(ui, tl("Quantité"), &mut m.amount, 0.0..=100.0, 1.0, 100.0, None, " %");
        caption(ui, tl("Composants"));
        let mut drop = None;
        for (k, p) in m.parts.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let sel = self.sel_part == k;
                if ui.add(egui::Button::selectable(sel, RichText::new(part_name(p.kind)).size(12.0))).clicked() {
                    self.sel_part = k;
                }
                egui::ComboBox::from_id_salt(("pmode", mi, k)).width(78.0).selected_text(match p.mode { PartMode::Add => "Ajouter", PartMode::Subtract => "Soustraire", PartMode::Intersect => "Intersecter" }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut p.mode, PartMode::Add, tl("Ajouter"));
                    ui.selectable_value(&mut p.mode, PartMode::Subtract, tl("Soustraire"));
                    ui.selectable_value(&mut p.mode, PartMode::Intersect, tl("Intersecter"));
                });
                ui.checkbox(&mut p.invert, "").on_hover_text(tl("Inverser ce composant"));
                if ui.small_button("×").clicked() {
                    drop = Some(k);
                }
            });
        }
        if let Some(k) = drop {
            m.parts.remove(k);
            self.sel_part = 0;
        }
        ui.horizontal(|ui| {
            for (label, mode) in [("Ajouter", PartMode::Add), ("Soustraire", PartMode::Subtract), ("Intersecter", PartMode::Intersect)] {
                ui.menu_button(RichText::new(tl(&label)).size(12.0), |ui| {
                    if let Some(k) = kind_menu(ui) {
                        m.parts.push(Part { mode, ..Part::new(k) });
                        self.sel_part = m.parts.len() - 1;
                        if k == PartKind::Brush {
                            *tool_req = Some(Tool::Brush);
                        }
                    }
                });
            }
        });
        if let Some(p) = m.parts.get_mut(self.sel_part) {
            match p.kind {
                PartKind::Linear => caption(ui, tl("Glisse les poignées sur la photo (outil M ou Main).")),
                PartKind::Radial => {
                    pct(ui, tl("Contour progressif"), &mut p.feather, 0.02, 1.0, 0.5);
                    slider_full(ui, tl("Angle"), &mut p.angle, -90.0..=90.0, 1.0, 0.0, None, "°");
                    pct(ui, tl("Largeur"), &mut p.rx, 0.005, 1.5, 0.25);
                    pct(ui, tl("Hauteur"), &mut p.ry, 0.005, 1.5, 0.25);
                }
                PartKind::Brush => {
                    ui.horizontal(|ui| {
                        caption(ui, &format!("{} touche(s) de pinceau", p.dabs.len()));
                        if ui.small_button(tl("Peindre")).clicked() {
                            *tool_req = Some(Tool::Brush);
                        }
                        if ui.small_button(tl("Effacer le tracé")).clicked() {
                            p.dabs.clear();
                        }
                    });
                }
                PartKind::Luminance => {
                    pct(ui, tl("Minimum"), &mut p.lo, 0.0, 1.0, 0.5);
                    pct(ui, tl("Maximum"), &mut p.hi, 0.0, 1.0, 1.0);
                    pct(ui, tl("Lissage"), &mut p.smooth, 0.0, 0.5, 0.1);
                }
                PartKind::Color => {
                    hue_slider(ui, tl("Teinte ciblée"), &mut p.hue);
                    slider_full(ui, tl("Tolérance"), &mut p.range, 5.0..=120.0, 1.0, 30.0, None, "°");
                }
                PartKind::Sky => caption(ui, tl("Détection automatique du ciel (zones bleues ou claires reliées au haut de l'image).")),
                PartKind::Subject => caption(ui, tl("Détection automatique du sujet (ce qui se distingue du fond, au centre).")),
                PartKind::All => caption(ui, tl("Couvre toute l'image : combine avec Soustraire pour exclure une zone.")),
            }
        }
        ui.add_space(6.0);
        caption(ui, tl("Réglages du masque"));
        local_sliders(ui, &mut m.adj);
        if ui.small_button(tl("Réinitialiser les réglages du masque")).clicked() {
            m.adj = Local::default();
        }
        let _ = Id::NULL;
    }

    fn central(&mut self, root: &mut Ui, ctx: &egui::Context, i: usize) {
        egui::CentralPanel::default().frame(Frame::new().fill(Color32::from_rgb(0x12, 0x12, 0x12))).show(root, |ui| {
            let full = ui.max_rect();
            let bar = Rect::from_min_size(full.min, vec2(full.width(), 38.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(14.0, 4.0))), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let held = ui.add(egui::Button::new(RichText::new(tl("Original")).size(12.0)).corner_radius(CornerRadius::same(7)).sense(Sense::click_and_drag()));
                        if held.is_pointer_button_down_on() {
                            self.before = true;
                        }
                        held.on_hover_text(tl("Maintenir pour voir l'original (ou \\)"));
                        egui::ComboBox::from_id_salt("compare").width(120.0).selected_text(["Avant / après", "Rideau vertical", "Côte à côte", "Rideau horizontal"][self.compare]).show_ui(ui, |ui| {
                            for (k, l) in ["Désactivé", "Rideau vertical", "Côte à côte", "Rideau horizontal"].iter().enumerate() {
                                ui.selectable_value(&mut self.compare, k, *l);
                            }
                        });
                        if ui.selectable_label(self.clipping, tl("Écrêtage")).on_hover_text(tl("Rouge : blancs brûlés · bleu : noirs bouchés (J)")).clicked() {
                            self.clipping = !self.clipping;
                            self.shown = None;
                        }
                        egui::ComboBox::from_id_salt("zoom").width(70.0).selected_text(ZOOMS.iter().find(|z| z.1 == self.zoom).map(|z| z.0).unwrap_or("Zoom")).show_ui(ui, |ui| {
                            for (n, z) in ZOOMS {
                                if ui.selectable_value(&mut self.zoom, z, n).clicked() {
                                    self.pan = Vec2::ZERO;
                                }
                            }
                        });
                        if ui.selectable_label(self.info_overlay > 0, tl("Infos")).on_hover_text(tl("I")).clicked() {
                            self.info_overlay = (self.info_overlay + 1) % 3;
                        }
                        if ui.selectable_label(self.hide_panels, tl("Plein écran")).on_hover_text(tl("Tab : masquer les panneaux")).clicked() {
                            self.hide_panels = !self.hide_panels;
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.add(egui::Label::new(RichText::new(display_name(&self.files[i])).size(12.5).color(MUTED)).truncate());
                        });
                    });
                });
            });
            let obar = Rect::from_min_size(pos2(full.left(), bar.bottom()), vec2(full.width(), 36.0));
            ui.painter().rect_filled(obar, CornerRadius::ZERO, Color32::from_rgb(0x1a, 0x1a, 0x1a));
            ui.scope_builder(egui::UiBuilder::new().max_rect(obar.shrink2(vec2(14.0, 3.0))), |ui| {
                ui.horizontal_centered(|ui| self.option_bar(ui, i));
            });
            let strip = Rect::from_min_max(pos2(full.left(), obar.bottom()), pos2(full.left() + 52.0, full.bottom()));
            ui.painter().rect_filled(strip, CornerRadius::ZERO, Color32::from_rgb(0x1a, 0x1a, 0x1a));
            ui.scope_builder(egui::UiBuilder::new().max_rect(strip.shrink2(vec2(7.0, 10.0))), |ui| {
                ui.vertical(|ui| self.tool_strip(ui, ctx));
            });
            let area = Rect::from_min_max(pos2(strip.right(), obar.bottom()), full.max).shrink(20.0);
            self.photo(ui, area, i, ctx);
        });
    }

    fn photo(&mut self, ui: &mut Ui, area: Rect, i: usize, ctx: &egui::Context) {
        let p = ui.painter_at(area.expand(20.0));
        let ready = self.preview.is_some() && self.source.as_ref().is_some_and(|(sp, _)| *sp == self.files[i]);
        if !ready {
            if self.loading {
                p.text(area.center(), Align2::CENTER_CENTER, tl("Chargement…"), FontId::proportional(15.0), MUTED);
            }
            return;
        }
        let Some(t) = self.preview.clone() else { return };
        let [w, h] = t.size();
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        let side = self.compare == 2 && self.before_tex.is_some();
        let main_area = if side { Rect::from_min_max(pos2(area.center().x + 6.0, area.top()), area.max) } else { area };
        let fit = (main_area.width() / w as f32).min(main_area.height() / h as f32);
        let scale = if self.zoom > 0.0 { self.zoom / ui.ctx().pixels_per_point() } else { fit };
        let r = Rect::from_center_size(main_area.center() + if self.zoom > 0.0 { self.pan } else { Vec2::ZERO }, vec2(w as f32, h as f32) * scale);
        self.last_view = Some((main_area, r));
        let painter = ui.painter_at(main_area.expand(20.0));
        painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 26, spread: 0, color: Color32::from_black_alpha(140) }.as_shape(r, CornerRadius::ZERO));
        painter.image(t.id(), r, uv, Color32::WHITE);
        if let Some(ct) = &self.clip_tex {
            painter.image(ct.id(), r, uv, Color32::WHITE);
        }
        if self.show_overlay
            && let Some((_, _, m, mt)) = &self.mask_tex
            && Some(*m) == self.sel_mask
            && self.tool != Tool::Crop
        {
            painter.image(mt.id(), r, uv, Color32::WHITE);
        }
        if self.compare > 0
            && let Some((bp, _, bt)) = &self.before_tex
            && *bp == self.files[i]
            && self.tool != Tool::Crop
        {
            let bt = bt.clone();
            match self.compare {
                2 => {
                    let left = Rect::from_min_max(area.min, pos2(area.center().x - 6.0, area.bottom()));
                    let [bw, bh] = bt.size();
                    let bf = (left.width() / bw as f32).min(left.height() / bh as f32);
                    let br = Rect::from_center_size(left.center(), vec2(bw as f32, bh as f32) * bf);
                    p.image(bt.id(), br, uv, Color32::WHITE);
                    p.text(br.left_top() + vec2(10.0, 10.0), Align2::LEFT_TOP, tl("Avant"), semibold(12.0), Color32::WHITE);
                    painter.text(r.left_top() + vec2(10.0, 10.0), Align2::LEFT_TOP, tl("Après"), semibold(12.0), Color32::WHITE);
                }
                c => {
                    let vertical = c == 1;
                    let at = if vertical { r.left() + r.width() * self.split_pos } else { r.top() + r.height() * self.split_pos };
                    let (dst, src_uv) = if vertical { (Rect::from_min_max(r.min, pos2(at, r.bottom())), Rect::from_min_max(pos2(0.0, 0.0), pos2(self.split_pos, 1.0))) } else { (Rect::from_min_max(r.min, pos2(r.right(), at)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, self.split_pos))) };
                    painter.image(bt.id(), dst, src_uv, Color32::WHITE);
                    let (a, b) = if vertical { (pos2(at, r.top()), pos2(at, r.bottom())) } else { (pos2(r.left(), at), pos2(r.right(), at)) };
                    painter.line_segment([a, b], Stroke::new(2.0, Color32::WHITE));
                    let knob = if vertical { pos2(at, r.center().y) } else { pos2(r.center().x, at) };
                    painter.circle(knob, 12.0, Color32::from_black_alpha(160), Stroke::new(2.0, Color32::WHITE));
                    painter.text(knob, Align2::CENTER_CENTER, if vertical { "⇔" } else { "⇕" }, FontId::proportional(12.0), Color32::WHITE);
                    painter.text(r.left_top() + vec2(10.0, 10.0), Align2::LEFT_TOP, tl("Avant"), semibold(12.0), Color32::WHITE);
                    painter.text(r.right_bottom() - vec2(10.0, 10.0), Align2::RIGHT_BOTTOM, tl("Après"), semibold(12.0), Color32::WHITE);
                    let hr = if vertical { Rect::from_center_size(knob, vec2(28.0, r.height())) } else { Rect::from_center_size(knob, vec2(r.width(), 28.0)) };
                    let handle = ui.interact(hr, ui.make_persistent_id("split"), Sense::drag());
                    if handle.hovered() || handle.dragged() {
                        ctx.set_cursor_icon(if vertical { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::ResizeVertical });
                    }
                    if let Some(pp) = handle.interact_pointer_pos() {
                        self.drag = Drag::Split;
                        self.split_pos = if vertical { (pp.x - r.left()) / r.width() } else { (pp.y - r.top()) / r.height() }.clamp(0.02, 0.98);
                    }
                    if handle.drag_stopped() {
                        self.drag = Drag::None;
                    }
                }
            }
        }
        if self.drag != Drag::Split {
            self.photo_tools(ui, r, main_area.expand(20.0), i, ctx);
        }
        self.info_overlay(&painter, r, i);
        if self.before {
            let tag = Rect::from_min_size(r.left_top() + vec2(12.0, r.height() - 38.0), vec2(80.0, 26.0));
            painter.rect_filled(tag, CornerRadius::same(13), Color32::from_black_alpha(170));
            painter.text(tag.center(), Align2::CENTER_CENTER, tl("Original"), FontId::proportional(12.0), Color32::WHITE);
        }
    }
}

