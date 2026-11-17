use crate::widgets::{caption, heading, nav_row, rgb, short};
use crate::{Collection, LABELS, Lumen, SORTS, STAR, Smart, View, copy_index, display_name, real};
use crate::{develop, meta};
use eframe::egui::{self, Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};

impl Lumen {
    fn library_nav(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::Panel::left("lib-nav").resizable(false).exact_size(236.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 14))).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                heading(ui, "CATALOGUE");
                let all = self.files.len();
                let entries: Vec<crate::Entry> = (0..all).map(|i| self.get(i)).collect();
                let count = |f: &dyn Fn(&crate::Entry) -> bool| entries.iter().filter(|e| f(e)).count();
                let virt = (0..all).filter(|&i| copy_index(&self.files[i]).is_some()).count();
                let rows: [(Collection, &str, usize); 7] = [
                    (Collection::All, "Toutes les photos", count(&|e| e.flag >= 0)),
                    (Collection::Picks, "Retenues", count(&|e| e.flag > 0)),
                    (Collection::Rated, "Notées 3★ et plus", count(&|e| e.rating >= 3)),
                    (Collection::Unrated, "Sans note", count(&|e| e.rating == 0 && e.flag >= 0)),
                    (Collection::Edited, "Modifiées", count(&|e| !e.settings.is_default())),
                    (Collection::Virtual, "Copies virtuelles", virt),
                    (Collection::Rejected, "Rejetées", count(&|e| e.flag < 0)),
                ];
                for (c, label, n) in rows {
                    if nav_row(ui, label, n, self.collection == c).clicked() {
                        self.collection = c;
                    }
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    heading(ui, "COLLECTIONS DYNAMIQUES");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("+").on_hover_text("Nouvelle collection dynamique").clicked() {
                            self.smart_edit = Some((None, Smart { min_rating: 0, ..Default::default() }));
                        }
                    });
                });
                let smarts = self.catalog.smart.clone();
                let mut remove = None;
                for (k, sm) in smarts.iter().enumerate() {
                    let n = (0..all).filter(|&i| self.smart_match(i, &entries[i], sm)).count();
                    let r = nav_row(ui, &sm.name, n, self.collection == Collection::Smart(k));
                    if r.clicked() {
                        self.collection = Collection::Smart(k);
                    }
                    r.context_menu(|ui| {
                        if ui.button("Modifier les règles…").clicked() {
                            self.smart_edit = Some((Some(k), sm.clone()));
                            ui.close();
                        }
                        if ui.button("Supprimer").clicked() {
                            remove = Some(k);
                            ui.close();
                        }
                    });
                }
                if smarts.is_empty() {
                    caption(ui, "Crée des filtres enregistrés : 5★ retenues, RAW non retouchés…");
                }
                if let Some(k) = remove {
                    self.catalog.smart.remove(k);
                    self.collection = Collection::All;
                    self.dirty_catalog = true;
                }
                ui.add_space(12.0);
                heading(ui, "ÉTIQUETTES");
                ui.horizontal(|ui| {
                    for (k, (name, c)) in LABELS.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
                        ui.painter().circle_filled(r.center(), 9.0, rgb(*c));
                        if self.collection == Collection::Label(k as u8 + 1) {
                            ui.painter().circle_stroke(r.center(), 11.5, Stroke::new(1.5, Color32::WHITE));
                        }
                        let n = entries.iter().filter(|e| e.label == k as u8 + 1).count();
                        if resp.on_hover_text(format!("{name} · {n} photo(s)")).clicked() {
                            self.collection = Collection::Label(k as u8 + 1);
                        }
                    }
                });
                ui.add_space(12.0);
                heading(ui, "MOTS-CLÉS");
                let mut kw: Vec<(String, usize)> = Vec::new();
                for e in &entries {
                    for k in &e.keywords {
                        match kw.iter_mut().find(|(n, _)| n == k) {
                            Some(x) => x.1 += 1,
                            None => kw.push((k.clone(), 1)),
                        }
                    }
                }
                kw.sort();
                if kw.is_empty() {
                    caption(ui, "Ajoute des mots-clés dans le panneau de droite.");
                }
                for (k, n) in kw {
                    let on = self.collection == Collection::Keyword(k.clone());
                    if nav_row(ui, &k, n, on).clicked() {
                        self.collection = Collection::Keyword(k);
                    }
                }
                ui.add_space(12.0);
                heading(ui, "DOSSIERS");
                let current = self.catalog.folder.clone();
                if ui.checkbox(&mut self.catalog.recursive, "Inclure les sous-dossiers").changed()
                    && let Some(f) = current.clone()
                {
                    self.open_folder(f, ctx);
                }
                if let Some(cur) = &current
                    && let Ok(rd) = std::fs::read_dir(cur)
                {
                    let mut subs: Vec<std::path::PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
                    subs.sort();
                    if let Some(parent) = cur.parent()
                        && nav_row(ui, "↑ Dossier parent", usize::MAX, false).clicked()
                    {
                        self.open_folder(parent.to_path_buf(), ctx);
                    }
                    for s in subs.into_iter().take(30) {
                        let name = format!("▸ {}", s.file_name().unwrap_or_default().to_string_lossy());
                        if nav_row(ui, &name, usize::MAX, false).clicked() {
                            self.open_folder(s, ctx);
                            return;
                        }
                    }
                }
                ui.add_space(6.0);
                caption(ui, "Récents");
                for f in self.catalog.recent_folders.clone() {
                    let name = f.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| f.display().to_string());
                    if nav_row(ui, &name, usize::MAX, current.as_ref() == Some(&f)).on_hover_text(f.display().to_string()).clicked() && current.as_ref() != Some(&f) {
                        self.open_folder(f, ctx);
                        return;
                    }
                }
                ui.add_space(8.0);
                if pro::secondary(ui, "Ouvrir un dossier…").clicked() {
                    self.pick_folder(ctx);
                }
            });
        });
    }

    fn library_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("lib-bar").exact_size(46.0).frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin::symmetric(18, 0))).show(root, |ui| {
            egui::ScrollArea::horizontal().id_salt("lib-bar-scroll").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
            ui.set_min_height(46.0);
            ui.horizontal_centered(|ui| {
                for (v, l, tip) in [(View::Library, "Grille", "G"), (View::Compare, "Comparer", "C"), (View::Survey, "Ensemble", "N")] {
                    if ui.selectable_label(self.view == v, l).on_hover_text(tip).clicked() {
                        self.view = v;
                    }
                }
                pro::toolbar_sep(ui);
                egui::ComboBox::from_id_salt("rating-mode").width(36.0).selected_text(["≥", "=", "≤"][self.rating_mode]).show_ui(ui, |ui| {
                    for (k, l) in ["≥  au moins", "=  exactement", "≤  au plus"].iter().enumerate() {
                        ui.selectable_value(&mut self.rating_mode, k, *l);
                    }
                });
                for r in 0..=5u8 {
                    let label = if r == 0 { "Toutes".to_string() } else { format!("{r}★") };
                    let on = self.min_rating == r;
                    let (rect, resp) = ui.allocate_exact_size(vec2(if r == 0 { 56.0 } else { 34.0 }, 26.0), Sense::click());
                    ui.painter().rect_filled(rect, CornerRadius::same(6), if on { BLUE } else if resp.hovered() { FIELD } else { Color32::TRANSPARENT });
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(12.0), if on { Color32::WHITE } else { INK });
                    if resp.clicked() {
                        self.min_rating = r;
                    }
                }
                pro::toolbar_sep(ui);
                ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Rechercher").desired_width(100.0));
                if !self.query.is_empty() && ui.small_button("×").clicked() {
                    self.query.clear();
                }
                pro::toolbar_sep(ui);
                egui::ComboBox::from_id_salt("sort").selected_text(SORTS[self.sort]).width(110.0).show_ui(ui, |ui| {
                    for (k, s) in SORTS.iter().enumerate() {
                        ui.selectable_value(&mut self.sort, k, *s);
                    }
                });
                if ui.selectable_label(self.sort_desc, if self.sort_desc { "↓" } else { "↑" }).on_hover_text("Tri · inverser l'ordre").clicked() {
                    self.sort_desc = !self.sort_desc;
                }
                pro::toolbar_sep(ui);
                ui.spacing_mut().slider_width = 80.0;
                ui.add(egui::Slider::new(&mut self.thumb_size, 120.0..=340.0).show_value(false)).on_hover_text("Taille des vignettes");
            });
            });
        });
    }

    fn selection_bar(&mut self, root: &mut Ui, ctx: &egui::Context) {
        if self.picked.len() < 2 {
            return;
        }
        egui::Panel::top("lib-sel").exact_size(44.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(18, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(RichText::new(format!("{} photos sélectionnées", self.picked.len())).font(semibold(12.5)).color(INK_STRONG));
                pro::toolbar_sep(ui);
                let picked = self.picked.clone();
                if pro::secondary(ui, "Synchroniser").on_hover_text("Copie les réglages de la photo active (groupes cochés dans Copier les réglages)").clicked()
                    && let Some(c) = self.current
                {
                    let st = self.settings(c);
                    for &p in &picked {
                        let s = meta::copy_groups(&self.settings(p), &st, &self.copy_groups);
                        self.set_settings(p, s);
                    }
                    self.notify("Réglages synchronisés", ctx);
                }
                ui.menu_button("Préréglage", |ui| {
                    let mut cat = "";
                    for (c, name, st) in develop::preset_catalog() {
                        if c != cat {
                            cat = c;
                            ui.label(RichText::new(c).size(11.0).color(FAINT));
                        }
                        if ui.button(name).clicked() {
                            self.apply_preset(&st);
                            ui.close();
                        }
                    }
                    for (name, st) in self.catalog.user_presets.clone() {
                        if ui.button(name).clicked() {
                            self.apply_preset(&st);
                            ui.close();
                        }
                    }
                });
                if pro::ghost(ui, "Empiler").on_hover_text("Ctrl+G").clicked() {
                    self.stack_selection();
                }
                for (label, flag) in [("Retenir", 1i8), ("Rejeter", -1)] {
                    if pro::ghost(ui, label).clicked() {
                        for &p in &picked {
                            self.entry(p).flag = flag;
                        }
                        self.dirty_catalog = true;
                    }
                }
                if pro::ghost(ui, "Comparer").clicked() {
                    self.view = View::Survey;
                }
                if pro::ghost(ui, "Réinitialiser").clicked() {
                    for &p in &picked {
                        self.set_settings(p, develop::Settings::default());
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::ghost(ui, "Tout désélectionner").clicked() {
                        self.picked.clear();
                    }
                });
            });
        });
    }

    fn library_side(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::Panel::right("lib-side").resizable(false).exact_size(290.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let Some(i) = self.current else {
                    Frame::new().inner_margin(Margin::same(16)).show(ui, |ui| caption(ui, "Sélectionne une photo pour voir ses informations."));
                    return;
                };
                let targets = self.targets();
                pro::dock_panel(ui, "quick", "Développement rapide", |ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 6.0);
                    caption(ui, &format!("S'applique à {} photo{}", targets.len(), if targets.len() > 1 { "s" } else { "" }));
                    let rows: [(&str, fn(&mut develop::Settings, f32), [f32; 4]); 8] = [
                        ("Exposition", |s, d| s.exposure = (s.exposure + d).clamp(-4.0, 4.0), [-1.0, -0.33, 0.33, 1.0]),
                        ("Contraste", |s, d| s.contrast = (s.contrast + d).clamp(-100.0, 100.0), [-20.0, -5.0, 5.0, 20.0]),
                        ("Hautes lumières", |s, d| s.highlights = (s.highlights + d).clamp(-100.0, 100.0), [-20.0, -5.0, 5.0, 20.0]),
                        ("Ombres", |s, d| s.shadows = (s.shadows + d).clamp(-100.0, 100.0), [-20.0, -5.0, 5.0, 20.0]),
                        ("Température", |s, d| s.temperature = (s.temperature + d).clamp(-100.0, 100.0), [-20.0, -5.0, 5.0, 20.0]),
                        ("Clarté", |s, d| s.clarity = (s.clarity + d).clamp(-100.0, 100.0), [-20.0, -5.0, 5.0, 20.0]),
                        ("Vibrance", |s, d| s.vibrance = (s.vibrance + d).clamp(-100.0, 100.0), [-20.0, -5.0, 5.0, 20.0]),
                        ("Saturation", |s, d| s.saturation = (s.saturation + d).clamp(-100.0, 100.0), [-20.0, -5.0, 5.0, 20.0]),
                    ];
                    for (label, f, steps) in rows {
                        ui.horizontal(|ui| {
                            ui.add_sized([96.0, 22.0], egui::Label::new(RichText::new(label).size(12.0).color(MUTED)));
                            for (k, d) in steps.iter().enumerate() {
                                let txt = ["◀◀", "◀", "▶", "▶▶"][k];
                                if ui.add_sized([36.0, 22.0], egui::Button::new(RichText::new(txt).size(10.0))).on_hover_text(format!("{d:+}")).clicked() {
                                    for &t in &targets {
                                        let mut s = self.settings(t);
                                        f(&mut s, *d);
                                        self.set_settings(t, s);
                                    }
                                }
                            }
                        });
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.menu_button("Balance des blancs", |ui| {
                            for (name, t, ti) in develop::WB_PRESETS {
                                if ui.button(name).clicked() {
                                    for &k in &targets {
                                        let mut s = self.settings(k);
                                        (s.temperature, s.tint) = (t, ti);
                                        self.set_settings(k, s);
                                    }
                                }
                            }
                        });
                        if pro::ghost(ui, "Noir et blanc").clicked() {
                            for &k in &targets {
                                let mut s = self.settings(k);
                                s.mono = !s.mono;
                                self.set_settings(k, s);
                            }
                        }
                        if pro::ghost(ui, "Réinitialiser").clicked() {
                            for &k in &targets {
                                self.set_settings(k, develop::Settings::default());
                            }
                        }
                    });
                });
                pro::dock_panel(ui, "keywords", "Mots-clés", |ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 6.0);
                    let e = self.get(i);
                    let r = ui.add(egui::TextEdit::singleline(&mut self.keyword_input).hint_text("Ajouter (virgules acceptées)").desired_width(f32::INFINITY));
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let words: Vec<String> = self.keyword_input.split(',').map(|w| w.trim().to_string()).filter(|w| !w.is_empty()).collect();
                        for &t in &targets {
                            let en = self.entry(t);
                            for w in &words {
                                if !en.keywords.contains(w) {
                                    en.keywords.push(w.clone());
                                }
                            }
                        }
                        self.keyword_input.clear();
                        self.dirty_catalog = true;
                        r.request_focus();
                    }
                    let mut remove = None;
                    ui.horizontal_wrapped(|ui| {
                        for k in &e.keywords {
                            if pro::chip(ui, &format!("{k}  ×")).on_hover_text("Retirer ce mot-clé").clicked() {
                                remove = Some(k.clone());
                            }
                        }
                    });
                    if let Some(k) = remove {
                        for &t in &targets {
                            self.entry(t).keywords.retain(|x| *x != k);
                        }
                        self.dirty_catalog = true;
                    }
                    let mut all: Vec<String> = self.catalog.photos.values().flat_map(|p| p.keywords.clone()).collect();
                    all.sort();
                    all.dedup();
                    all.retain(|k| !e.keywords.contains(k));
                    if !all.is_empty() {
                        caption(ui, "Suggestions");
                        ui.horizontal_wrapped(|ui| {
                            for k in all.into_iter().take(12) {
                                if ui.small_button(&k).clicked() {
                                    for &t in &targets {
                                        let en = self.entry(t);
                                        if !en.keywords.contains(&k) {
                                            en.keywords.push(k.clone());
                                        }
                                    }
                                    self.dirty_catalog = true;
                                }
                            }
                        });
                    }
                });
                let exif = self.exif_for(i);
                pro::dock_panel(ui, "metadata", "Métadonnées", |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let mut e = self.get(i);
                    let before = (e.title.clone(), e.caption.clone(), e.rating, e.label, e.flag);
                    caption(ui, "Titre");
                    ui.add(egui::TextEdit::singleline(&mut e.title).desired_width(f32::INFINITY));
                    caption(ui, "Légende");
                    ui.add(egui::TextEdit::multiline(&mut e.caption).desired_rows(2).desired_width(f32::INFINITY));
                    ui.horizontal(|ui| {
                        for r in 1..=5u8 {
                            let on = e.rating >= r;
                            if ui.add(egui::Label::new(RichText::new(if on { "★" } else { "☆" }).size(18.0).color(if on { STAR } else { FAINT })).sense(Sense::click())).clicked() {
                                e.rating = if e.rating == r { 0 } else { r };
                            }
                        }
                        ui.add_space(8.0);
                        for (k, (_, c)) in LABELS.iter().enumerate() {
                            let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                            ui.painter().circle_filled(r.center(), 6.0, rgb(*c));
                            if e.label == k as u8 + 1 {
                                ui.painter().circle_stroke(r.center(), 8.0, Stroke::new(1.5, Color32::WHITE));
                            }
                            if resp.clicked() {
                                e.label = if e.label == k as u8 + 1 { 0 } else { k as u8 + 1 };
                            }
                        }
                    });
                    if (e.title.clone(), e.caption.clone(), e.rating, e.label, e.flag) != before {
                        let en = self.entry(i);
                        (en.title, en.caption, en.rating, en.label) = (e.title, e.caption, e.rating, e.label);
                        self.dirty_catalog = true;
                    }
                    ui.add_space(6.0);
                    let p = real(&self.files[i]);
                    pro::label_value(ui, "Fichier", &short(&display_name(&self.files[i]), 28));
                    if let Some(info) = self.info.get(&p) {
                        pro::label_value(ui, "Taille", &format!("{:.1} Mo", info.size as f64 / 1_048_576.0));
                    }
                    if let Ok((w, h)) = image::image_dimensions(&p) {
                        pro::label_value(ui, "Dimensions", &format!("{w} × {h} px ({:.1} Mpx)", (w * h) as f32 / 1e6));
                    }
                    pro::label_value(ui, "Type", &p.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_default());
                    for (k, v) in &exif {
                        pro::label_value(ui, k, &short(v, 30));
                    }
                    if exif.is_empty() {
                        caption(ui, "Aucune donnée EXIF lisible dans ce fichier.");
                    }
                    ui.add_space(4.0);
                    if pro::ghost(ui, "Afficher dans l'Explorateur").clicked() {
                        let _ = std::process::Command::new("explorer").arg(format!("/select,{}", p.display())).spawn();
                    }
                });
                let _ = ctx;
            });
        });
    }

    pub fn library(&mut self, root: &mut Ui, ctx: &egui::Context) {
        if !self.hide_panels {
            self.library_nav(root, ctx);
            self.library_side(root, ctx);
        }
        self.library_bar(root);
        self.selection_bar(root, ctx);
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin { left: 18, right: 18, top: 4, bottom: 8 })).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let vis = self.visible();
                if vis.is_empty() {
                    ui.add_space(60.0);
                    ui.vertical_centered(|ui| ui.label(RichText::new("Aucune photo ne correspond à ce filtre").color(MUTED)));
                    return;
                }
                let cell = self.thumb_size;
                let gap = 12.0;
                let cols = (((ui.available_width() + gap) / (cell + gap)).floor() as usize).max(1);
                for row in vis.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for &i in row {
                            let resp = self.tile(ui, i, cell);
                            self.tile_menu(&resp.1, i, ctx);
                            match resp.0 {
                                Some(true) => {
                                    self.picked.clear();
                                    self.select(i, ctx);
                                    self.view = View::Develop;
                                }
                                Some(false) => self.click_tile(i, &vis, ui, ctx),
                                None => {}
                            }
                        }
                    });
                    ui.add_space(gap);
                }
            });
        });
    }

    fn click_tile(&mut self, i: usize, vis: &[usize], ui: &Ui, ctx: &egui::Context) {
        let (ctrl, shift) = ui.input(|i| (i.modifiers.command, i.modifiers.shift));
        if ctrl {
            if self.picked.is_empty()
                && let Some(c) = self.current
            {
                self.picked.push(c);
            }
            if let Some(k) = self.picked.iter().position(|p| *p == i) {
                self.picked.remove(k);
            } else {
                self.picked.push(i);
            }
            self.current = Some(i);
        } else if shift && let Some(c) = self.current {
            let (a, b) = (vis.iter().position(|v| *v == c).unwrap_or(0), vis.iter().position(|v| *v == i).unwrap_or(0));
            self.picked = vis[a.min(b)..=a.max(b)].to_vec();
        } else {
            self.picked.clear();
            self.select(i, ctx);
        }
    }

    fn tile_menu(&mut self, resp: &egui::Response, i: usize, ctx: &egui::Context) {
        resp.context_menu(|ui| {
            if self.current != Some(i) && !self.picked.contains(&i) {
                self.picked.clear();
                self.select(i, ctx);
            }
            if ui.button("Ouvrir en Développement").clicked() {
                self.select(i, ctx);
                self.view = View::Develop;
                ui.close();
            }
            if ui.button("Créer une copie virtuelle").clicked() {
                self.current = Some(i);
                self.virtual_copy(ctx);
                ui.close();
            }
            if copy_index(&self.files[i]).is_some() && ui.button("Supprimer la copie virtuelle").clicked() {
                self.remove_virtual(i);
                ui.close();
                return;
            }
            ui.separator();
            if ui.button("Empiler la sélection").clicked() {
                self.stack_selection();
                ui.close();
            }
            if self.get(i).stack > 0 {
                if ui.button("Réduire / déplier la pile").clicked() {
                    self.current = Some(i);
                    self.toggle_stack();
                    ui.close();
                }
                if ui.button("Désempiler").clicked() {
                    self.unstack();
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Exporter…").clicked() {
                self.open_export(self.picked.len() < 2);
                ui.close();
            }
            if ui.button("Afficher dans l'Explorateur").clicked() {
                let _ = std::process::Command::new("explorer").arg(format!("/select,{}", real(&self.files[i]).display())).spawn();
                ui.close();
            }
        });
    }

    fn tile(&mut self, ui: &mut Ui, i: usize, size: f32) -> (Option<bool>, egui::Response) {
        let (rect, resp) = ui.allocate_exact_size(vec2(size, size + 30.0), Sense::click());
        let on = self.current == Some(i) || self.picked.contains(&i);
        let p = ui.painter();
        p.rect(rect, CornerRadius::same(6), if on || resp.hovered() { Color32::from_rgb(0x2c, 0x2c, 0x2c) } else { Color32::from_rgb(0x22, 0x22, 0x22) }, Stroke::new(if on { 2.0 } else { 1.0 }, if on { BLUE } else { LINE }), StrokeKind::Inside);
        let img_area = Rect::from_min_size(rect.min + vec2(8.0, 8.0), Vec2::splat(size - 16.0));
        let path = self.files[i].clone();
        match self.thumbs.get(&path) {
            Some(t) => {
                let [tw, th] = t.size();
                let s = (img_area.width() / tw as f32).min(img_area.height() / th as f32);
                let fit = Rect::from_center_size(img_area.center(), vec2(tw as f32, th as f32) * s);
                p.image(t.id(), fit, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            None => {
                p.rect_filled(img_area.shrink(10.0), CornerRadius::same(4), Color32::from_white_alpha(6));
            }
        }
        let e = self.get(i);
        let y = rect.bottom() - 17.0;
        let name = display_name(&path);
        p.text(pos2(rect.left() + 12.0, y), Align2::LEFT_CENTER, short(&name, (size / 9.0) as usize), FontId::proportional(12.0), MUTED);
        let stars: String = (0..5).map(|k| if k < e.rating { '★' } else { '☆' }).collect();
        p.text(pos2(rect.right() - 12.0, y), Align2::RIGHT_CENTER, stars, FontId::proportional(12.0), if e.rating > 0 { STAR } else { FAINT });
        if e.flag < 0 {
            p.rect_filled(img_area, CornerRadius::same(6), Color32::from_black_alpha(150));
            p.text(img_area.right_top() + vec2(-14.0, 14.0), Align2::CENTER_CENTER, "✕", semibold(13.0), RED);
        } else if e.flag > 0 {
            let b = Rect::from_center_size(img_area.right_top() + vec2(-14.0, 14.0), Vec2::splat(20.0));
            p.circle_filled(b.center(), 10.0, GREEN);
            p.text(b.center(), Align2::CENTER_CENTER, "✓", semibold(12.0), Color32::WHITE);
        }
        if e.label > 0 {
            let c = LABELS[(e.label - 1) as usize % LABELS.len()].1;
            p.rect_filled(Rect::from_min_max(pos2(rect.left() + 8.0, rect.bottom() - 4.0), pos2(rect.right() - 8.0, rect.bottom() - 1.0)), CornerRadius::same(2), rgb(c));
        }
        let mut badge_x = img_area.left() + 6.0;
        let mut badge = |p: &egui::Painter, txt: &str| {
            let b = Rect::from_min_size(pos2(badge_x, img_area.top() + 6.0), vec2(22.0 + (txt.chars().count() as f32 - 1.0).max(0.0) * 7.0, 22.0));
            p.rect_filled(b, CornerRadius::same(11), Color32::from_black_alpha(160));
            p.text(b.center(), Align2::CENTER_CENTER, txt, FontId::proportional(12.0), Color32::WHITE);
            badge_x = b.right() + 4.0;
        };
        if !e.settings.is_default() {
            badge(p, "✎");
        }
        if copy_index(&path).is_some() {
            badge(p, "⧉");
        }
        if e.stack > 0 {
            let n = self.catalog.photos.values().filter(|x| x.stack == e.stack).count();
            badge(p, &format!("{n}"));
        }
        if !e.keywords.is_empty() {
            badge(p, "#");
        }
        let r = if resp.double_clicked() {
            Some(true)
        } else if resp.clicked() {
            Some(false)
        } else {
            None
        };
        (r, resp.on_hover_text(if e.title.is_empty() { name } else { format!("{}\n{name}", e.title) }))
    }

    pub fn compare_view(&mut self, root: &mut Ui, ctx: &egui::Context) {
        self.library_bar(root);
        let vis = self.visible();
        let a = self.current.or(vis.first().copied());
        let b = self.picked.iter().copied().find(|p| Some(*p) != a).or_else(|| {
            let pos = a.and_then(|c| vis.iter().position(|v| *v == c)).unwrap_or(0);
            vis.get(pos + 1).copied()
        });
        egui::Panel::bottom("cmp-bar").exact_size(46.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(18, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                if pro::ghost(ui, "◀ Candidat précédent").clicked()
                    && let Some(b) = b
                    && let Some(pos) = vis.iter().position(|v| *v == b)
                {
                    let prev = vis[..pos].iter().rev().find(|v| Some(**v) != a).copied();
                    if let Some(p) = prev {
                        self.picked = vec![a.unwrap_or(p), p];
                    }
                }
                if pro::ghost(ui, "Candidat suivant ▶").clicked()
                    && let Some(b) = b
                    && let Some(pos) = vis.iter().position(|v| *v == b)
                {
                    if let Some(n) = vis[pos + 1..].iter().find(|v| Some(**v) != a).copied() {
                        self.picked = vec![a.unwrap_or(n), n];
                    }
                }
                if pro::secondary(ui, "Le candidat devient la référence").clicked()
                    && let Some(b) = b
                {
                    self.picked.clear();
                    self.select(b, ctx);
                }
                caption(ui, "À gauche : la référence (photo active). À droite : le candidat.");
            });
        });
        egui::CentralPanel::default().frame(Frame::new().fill(Color32::from_rgb(0x12, 0x12, 0x12)).inner_margin(Margin::same(16))).show(root, |ui| {
            let full = ui.max_rect();
            let half = full.width() / 2.0 - 8.0;
            for (k, idx) in [a, b].into_iter().enumerate() {
                let r = Rect::from_min_size(full.min + vec2(k as f32 * (half + 16.0), 0.0), vec2(half, full.height()));
                self.big_photo(ui, r, idx, k == 0, ctx);
            }
        });
    }

    fn big_photo(&mut self, ui: &mut Ui, r: Rect, idx: Option<usize>, active: bool, ctx: &egui::Context) -> Option<egui::Response> {
        let Some(i) = idx else {
            ui.painter().text(r.center(), Align2::CENTER_CENTER, "Aucune photo", FontId::proportional(13.0), MUTED);
            return None;
        };
        let path = self.files[i].clone();
        let tex = self.large_texture(&path, ctx).or_else(|| self.thumbs.get(&path).cloned());
        let area = Rect::from_min_max(r.min, r.max - vec2(0.0, 34.0));
        let mut fit_rect = area;
        if let Some(t) = tex {
            let [w, h] = t.size();
            let s = (area.width() / w as f32).min(area.height() / h as f32);
            fit_rect = Rect::from_center_size(area.center(), vec2(w as f32, h as f32) * s);
            ui.painter().image(t.id(), fit_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        if active {
            ui.painter().rect_stroke(fit_rect.expand(3.0), CornerRadius::same(4), Stroke::new(2.0, BLUE), StrokeKind::Outside);
        }
        let e = self.get(i);
        let stars: String = (0..5).map(|k| if k < e.rating { '★' } else { '☆' }).collect();
        ui.painter().text(pos2(r.left() + 4.0, r.bottom() - 14.0), Align2::LEFT_CENTER, display_name(&path), FontId::proportional(12.0), INK);
        ui.painter().text(pos2(r.right() - 4.0, r.bottom() - 14.0), Align2::RIGHT_CENTER, format!("{}{stars}", if e.flag > 0 { "✓  " } else if e.flag < 0 { "✕  " } else { "" }), FontId::proportional(12.5), if e.rating > 0 { STAR } else { FAINT });
        let resp = ui.interact(fit_rect, ui.make_persistent_id(("big", i)), Sense::click());
        if resp.double_clicked() {
            self.picked.clear();
            self.select(i, ctx);
            self.view = View::Develop;
        } else if resp.clicked() {
            self.current = Some(i);
            self.select(i, ctx);
        }
        Some(resp)
    }

    pub fn survey_view(&mut self, root: &mut Ui, ctx: &egui::Context) {
        self.library_bar(root);
        let ids: Vec<usize> = if self.picked.len() > 1 { self.picked.clone() } else { self.visible().into_iter().take(12).collect() };
        egui::Panel::bottom("survey-bar").exact_size(40.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(18, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                caption(ui, if self.picked.len() > 1 { "Ensemble de la sélection : clique pour choisir, × pour écarter une photo, double-clic pour développer." } else { "Sélectionne plusieurs photos (Ctrl+clic) pour les comparer ici. Aperçu des 12 premières." });
            });
        });
        egui::CentralPanel::default().frame(Frame::new().fill(Color32::from_rgb(0x12, 0x12, 0x12)).inner_margin(Margin::same(16))).show(root, |ui| {
            let full = ui.max_rect();
            let n = ids.len().max(1);
            let aspect = full.width() / full.height().max(1.0);
            let cols = ((n as f32 * aspect).sqrt().ceil() as usize).clamp(1, n);
            let rows = n.div_ceil(cols);
            let (cw, ch) = (full.width() / cols as f32, full.height() / rows as f32);
            let mut drop = None;
            for (k, &i) in ids.iter().enumerate() {
                let cell = Rect::from_min_size(full.min + vec2((k % cols) as f32 * cw, (k / cols) as f32 * ch), vec2(cw, ch)).shrink(8.0);
                let active = self.current == Some(i);
                if let Some(resp) = self.big_photo(ui, cell, Some(i), active, ctx)
                    && self.picked.len() > 1
                {
                    let x = Rect::from_center_size(resp.rect.right_top() + vec2(-14.0, 14.0), Vec2::splat(22.0));
                    let xr = ui.interact(x, ui.make_persistent_id(("drop", i)), Sense::click());
                    if resp.hovered() || xr.hovered() {
                        ui.painter().circle_filled(x.center(), 11.0, Color32::from_black_alpha(180));
                        ui.painter().text(x.center(), Align2::CENTER_CENTER, "×", semibold(13.0), Color32::WHITE);
                    }
                    if xr.clicked() {
                        drop = Some(i);
                    }
                }
            }
            if let Some(d) = drop {
                self.picked.retain(|p| *p != d);
            }
        });
    }
}

