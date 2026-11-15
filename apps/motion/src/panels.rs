use super::*;
use model::{CYCLES, Fx, Guide, TEXT_ORDERS, set_color_key};

const LETTERBOX: [(&str, f32); 5] = [("Aucun", 0.0), ("1,85:1", 1.85), ("2:1", 2.0), ("2,35:1", 2.35), ("2,39:1", 2.39)];

fn prop_row(ui: &mut Ui, l: &mut Layer, pi: usize, t: f32, go: &mut Option<f32>, label: &str) {
    let def = l.default_of(pi);
    let p = &mut l.props[pi];
    ui.horizontal(|ui| {
        let has = p.key_at(t).is_some();
        let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
        let c = r.center();
        let d = 5.5;
        ui.painter().add(Shape::convex_polygon(vec![c + vec2(0.0, -d), c + vec2(d, 0.0), c + vec2(0.0, d), c + vec2(-d, 0.0)], if has { KEY } else { Color32::TRANSPARENT }, Stroke::new(1.5, if p.animated() { KEY } else { FAINT })));
        if resp.on_hover_text("Ajouter ou retirer une image clé ici").clicked() {
            p.toggle_key(t);
        }
        let name = if p.expressive() { format!("{label} ≈") } else { label.to_string() };
        let lab = ui.add(egui::Label::new(RichText::new(name).size(12.5).color(if p.animated() || p.expressive() { INK } else { MUTED })).sense(Sense::click()));
        lab.on_hover_text("Clic droit : réinitialiser, tremblement, boucle").context_menu(|ui| {
            if ui.button("Réinitialiser la valeur").clicked() {
                p.set(t, def);
                ui.close();
            }
            if ui.add_enabled(p.animated(), egui::Button::new("Supprimer l'animation")).clicked() {
                p.clear(t);
                ui.close();
            }
            if ui.add_enabled(p.keys.len() > 2, egui::Button::new("Répartir les clés uniformément")).clicked() {
                p.spread();
                ui.close();
            }
            ui.separator();
            ui.label(RichText::new("Expression").size(11.5).color(FAINT));
            ui.horizontal(|ui| {
                ui.label("Tremblement");
                ui.add(egui::DragValue::new(&mut p.wiggle).range(0.0..=2000.0).speed(0.5));
                ui.add(egui::DragValue::new(&mut p.wiggle_freq).range(0.0..=30.0).speed(0.05).suffix(" /s"));
            });
            ui.horizontal(|ui| {
                ui.label("Après la dernière clé");
                egui::ComboBox::from_id_salt(("cycle", pi)).selected_text(CYCLES[(p.cycle as usize).min(4)]).show_ui(ui, |ui| {
                    for (k, n) in CYCLES.iter().enumerate() {
                        ui.selectable_value(&mut p.cycle, k as u8, *n);
                    }
                });
            });
            if ui.add_enabled(p.expressive(), egui::Button::new("Retirer l'expression")).clicked() {
                p.wiggle = 0.0;
                p.cycle = 0;
                ui.close();
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if p.animated() && ui.small_button("›").on_hover_text("Clé suivante").clicked() {
                *go = p.keys.iter().map(|k| k.t).find(|k| *k > t + 1e-3);
            }
            let mut v = p.at(t);
            let speed = match pi {
                2 | 4 | 7 | 10 | 11 | 14 | 15 | 16 => 0.5,
                3 | 5 | 12 | 13 | 18 => 0.5,
                6 | 17 => 0.2,
                _ => 1.0,
            };
            let mut dv = egui::DragValue::new(&mut v).speed(speed).max_decimals(1);
            if matches!(pi, 4 | 7 | 14 | 15) {
                dv = dv.range(0.0..=100.0);
            }
            if matches!(pi, 6 | 17) {
                dv = dv.range(0.0..=400.0);
            }
            if ui.add(dv).changed() {
                p.set(t, v);
            }
            if p.animated() && ui.small_button("‹").on_hover_text("Clé précédente").clicked() {
                *go = p.keys.iter().rev().map(|k| k.t).find(|k| *k < t - 1e-3);
            }
        });
    });
    if let Some(k) = p.key_at(t) {
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label(RichText::new("Courbe").size(11.5).color(FAINT));
            ease_combo(ui, ("ease", pi), &mut p.keys[k]);
        });
    }
}

fn sub(ui: &mut Ui, s: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(s.to_uppercase()).size(10.5).color(FAINT).strong());
}

fn amount(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) -> bool {
    slider(ui, label, v, range, suffix);
    *v != 0.0
}

fn color_anim(ui: &mut Ui, keys: &mut Vec<model::CKey>, t: f32, cur: Option<[u8; 4]>, what: &str) {
    ui.horizontal(|ui| {
        let has = keys.iter().any(|k| (k.t - t).abs() < 1e-3);
        let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
        let c = r.center();
        let d = 5.5;
        ui.painter().add(Shape::convex_polygon(vec![c + vec2(0.0, -d), c + vec2(d, 0.0), c + vec2(0.0, d), c + vec2(-d, 0.0)], if has { KEY } else { Color32::TRANSPARENT }, Stroke::new(1.5, if keys.is_empty() { FAINT } else { KEY })));
        if resp.on_hover_text(format!("Image clé de {what} à la tête de lecture")).clicked() {
            if has {
                keys.retain(|k| (k.t - t).abs() >= 1e-3);
            } else if let Some(c) = cur {
                set_color_key(keys, t, model::color_at(keys, t).unwrap_or(c));
            }
        }
        ui.label(RichText::new(if keys.is_empty() { format!("Animer {what}") } else { format!("{what} animée · {} clés", keys.len()) }).size(12.0).color(if keys.is_empty() { MUTED } else { INK }));
        if !keys.is_empty() && ui.small_button("×").on_hover_text("Supprimer l'animation de couleur").clicked() {
            keys.clear();
        }
    });
}

impl Motion {
    pub(crate) fn comp_panels(&mut self, ui: &mut Ui) {
        pro::dock_panel(ui, "comp", "Composition", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            pro::label_value(ui, "Format", &format!("{} × {}", self.comp.width, self.comp.height));
            pro::label_value(ui, "Images/s", &self.comp.fps.to_string());
            ui.horizontal(|ui| {
                ui.label(RichText::new("Durée").size(12.5).color(MUTED));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(egui::DragValue::new(&mut self.comp.duration).range(0.1..=600.0).speed(0.1).suffix(" s"));
                });
            });
            if pro::secondary(ui, "Réglages…").clicked() {
                self.comp_sheet = true;
            }
            ui.add_space(4.0);
            bg_editor(ui, &mut self.comp);
        });
        pro::dock_panel(ui, "comp-fx", "Finition de l'image", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            slider(ui, "Vignettage", &mut self.comp.vignette, 0.0..=100.0, " %");
            slider(ui, "Grain film", &mut self.comp.grain, 0.0..=100.0, " %");
            ui.label(RichText::new("Bandes cinéma").size(12.5).color(MUTED));
            let mut k = LETTERBOX.iter().position(|l| (l.1 - self.comp.letterbox).abs() < 0.01).unwrap_or(0);
            let labels: Vec<&str> = LETTERBOX.iter().map(|l| l.0).collect();
            if pro::segmented(ui, &mut k, &labels) {
                self.comp.letterbox = LETTERBOX[k].1;
            }
        });
        pro::dock_panel(ui, "markers", "Repères", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            if self.comp.markers.is_empty() {
                ui.label(RichText::new("Appuie sur M pour poser un repère à la tête de lecture : il sert de point d'accroche et de raccourci J/K.").size(12.0).color(FAINT));
            }
            let mut remove = None;
            let mut go = None;
            for (k, m) in self.comp.markers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new(secs(m.t)).monospace().size(11.5)).frame(false)).on_hover_text("Aller au repère").clicked() {
                        go = Some(m.t);
                    }
                    ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(150.0));
                    if ui.small_button("×").clicked() {
                        remove = Some(k);
                    }
                });
            }
            if let Some(k) = remove {
                self.comp.markers.remove(k);
            }
            if let Some(t) = go {
                self.t = t;
            }
            if pro::ghost(ui, "+ Repère à la tête de lecture").clicked() {
                self.add_marker();
            }
        });
        pro::dock_panel(ui, "guides", "Repères de mise en page", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.label(RichText::new("Avec les règles (Ctrl+R), glisse depuis une règle pour tirer un repère ; glisse-le hors de la vue pour le supprimer.").size(12.0).color(FAINT));
            let mut remove = None;
            for (k, g) in self.comp.guides.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if g.vertical { "Vertical" } else { "Horizontal" }).size(12.0).color(MUTED));
                    ui.add(egui::DragValue::new(&mut g.pos).speed(1.0).suffix(" px"));
                    if ui.small_button("×").clicked() {
                        remove = Some(k);
                    }
                });
            }
            if let Some(k) = remove {
                self.comp.guides.remove(k);
            }
            ui.horizontal(|ui| {
                let (cw, ch) = (self.comp.width as f32, self.comp.height as f32);
                if ui.small_button("+ Centre").clicked() {
                    self.comp.guides.push(Guide { vertical: true, pos: cw / 2.0 });
                    self.comp.guides.push(Guide { vertical: false, pos: ch / 2.0 });
                }
                if ui.small_button("+ Tiers").clicked() {
                    for f in [1.0 / 3.0, 2.0 / 3.0] {
                        self.comp.guides.push(Guide { vertical: true, pos: (cw * f).round() });
                        self.comp.guides.push(Guide { vertical: false, pos: (ch * f).round() });
                    }
                }
                if ui.small_button("+ Marges 5 %").clicked() {
                    for (v, p) in [(true, cw * 0.05), (true, cw * 0.95), (false, ch * 0.05), (false, ch * 0.95)] {
                        self.comp.guides.push(Guide { vertical: v, pos: p.round() });
                    }
                }
                if !self.comp.guides.is_empty() && ui.small_button("Tout effacer").clicked() {
                    self.comp.guides.clear();
                }
            });
        });
    }

    pub(crate) fn layer_panels(&mut self, ui: &mut Ui, i: usize) {
        let t = self.t;
        let dur = self.comp.duration;
        let names: Vec<(usize, String)> = self.comp.layers.iter().enumerate().filter(|(k, _)| *k != i && !self.comp.chain_has(*k, i)).map(|(k, l)| (k, l.name.clone())).collect();
        let mut new_parent: Option<Option<usize>> = None;
        let cur_parent = self.comp.layers[i].parent;
        {
            let l = &mut self.comp.layers[i];
            pro::dock_panel(ui, "layer", "Calque", |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.add(egui::TextEdit::singleline(&mut l.name).desired_width(f32::INFINITY));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut l.visible, "Visible");
                    ui.checkbox(&mut l.locked, "Verrou");
                    ui.checkbox(&mut l.solo, "Solo");
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("De").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut l.start).range(0.0..=l.end).speed(0.05).suffix(" s"));
                    ui.label(RichText::new("à").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut l.end).range(l.start..=dur).speed(0.05).suffix(" s"));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Parent").size(12.5).color(MUTED));
                    let label = cur_parent.and_then(|p| names.iter().find(|n| n.0 == p)).map_or("Aucun".to_string(), |n| n.1.clone());
                    egui::ComboBox::from_id_salt("parent").selected_text(short(&label, 22)).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        if ui.selectable_label(cur_parent.is_none(), "Aucun").clicked() {
                            new_parent = Some(None);
                        }
                        for (k, n) in &names {
                            if ui.selectable_label(cur_parent == Some(*k), n).clicked() {
                                new_parent = Some(Some(*k));
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Étiquette").size(12.5).color(MUTED));
                    let cur = l.label.unwrap_or((i % LABELS.len()) as u8);
                    for (k, c) in LABELS.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                        ui.painter().circle_filled(r.center(), 7.0, *c);
                        if cur as usize == k {
                            ui.painter().circle_stroke(r.center(), 9.0, Stroke::new(1.5, Color32::WHITE));
                        }
                        if resp.clicked() {
                            l.label = Some(k as u8);
                        }
                    }
                });
            });
        }
        if let Some(p) = new_parent
            && !self.comp.set_parent(i, p, t)
        {
            self.error = Some("Ce parent créerait une boucle".into());
        }
        let mut go = None;
        let mut center_anchor = false;
        {
            let l = &mut self.comp.layers[i];
            pro::dock_panel(ui, "transform", "Transformation", |ui| {
                ui.spacing_mut().item_spacing.y = 5.0;
                for (pi, label) in [(8, "Ancre X"), (9, "Ancre Y"), (0, "Position X"), (1, "Position Y"), (2, "Échelle %"), (10, "Échelle X %"), (11, "Échelle Y %"), (3, "Rotation °"), (12, "Rotation X ° (3D)"), (13, "Rotation Y ° (3D)"), (5, "Inclinaison °"), (4, "Opacité %")] {
                    prop_row(ui, l, pi, t, &mut go, label);
                }
                ui.checkbox(&mut l.fx.auto_orient, "Orientation auto").on_hover_text("Le calque tourne dans le sens de sa trajectoire");
                ui.horizontal(|ui| {
                    if ui.small_button("Centrer l'ancre").clicked() {
                        center_anchor = true;
                    }
                    if ui.small_button("Réinitialiser").on_hover_text("Remet échelle, rotations, ancre et opacité par défaut").clicked() {
                        for pi in [2, 3, 4, 5, 8, 9, 10, 11, 12, 13] {
                            l.props[pi] = Prop::new(model::PROP_DEFAULT[pi]);
                        }
                    }
                });
            });
        }
        if center_anchor {
            self.anchor_to(4);
        }
        self.graph_panel(ui);
        if let Some(g) = go {
            self.t = g;
        }
        pro::dock_panel(ui, "align", "Aligner sur la composition", |ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            let w = (ui.available_width() - 8.0) / 3.0;
            let mut how = None;
            for row in [[("Gauche", 0u8), ("Centre ↔", 1), ("Droite", 2)], [("Haut", 3), ("Centre ↕", 4), ("Bas", 5)]] {
                ui.horizontal(|ui| {
                    for (label, k) in row {
                        if ui.add_sized([w, 26.0], egui::Button::new(RichText::new(label).size(12.0))).clicked() {
                            how = Some(k);
                        }
                    }
                });
            }
            let mut fit = None;
            ui.horizontal(|ui| {
                for (label, k) in [("Ajuster", 0u8), ("Largeur", 1), ("Hauteur", 2)] {
                    if ui.add_sized([w, 26.0], egui::Button::new(RichText::new(label).size(12.0))).on_hover_text("Met le calque à l'échelle de la composition").clicked() {
                        fit = Some(k);
                    }
                }
            });
            if ui.add_sized([ui.available_width(), 26.0], egui::Button::new(RichText::new("Centrer au milieu").size(12.0))).clicked() {
                how = Some(6);
            }
            if let Some(h) = how {
                self.align(h);
            }
            if let Some(k) = fit {
                self.fit_to_comp(k);
            }
        });
        let focus = std::mem::take(&mut self.focus_text);
        let l = &mut self.comp.layers[i];
        let is_shape = l.content.is_shape();
        let is_text = matches!(l.content, Content::Text { .. });
        pro::dock_panel(ui, "content", "Contenu", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            let before = l.content.color();
            content_editor(ui, &mut l.content, focus);
            let after = l.content.color();
            if !l.color_keys.is_empty()
                && before != after
                && let Some(c) = after
            {
                set_color_key(&mut l.color_keys, t, c);
            }
            if after.is_some() {
                color_anim(ui, &mut l.color_keys, t, after, "la couleur");
            }
        });
        if is_shape {
            let mut go = None;
            pro::dock_panel(ui, "path", "Tracé et opérateurs", |ui| {
                ui.spacing_mut().item_spacing.y = 5.0;
                ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                sub(ui, "Couper le tracé");
                for (pi, label) in [(14, "Début %"), (15, "Fin %"), (16, "Décalage %")] {
                    prop_row(ui, l, pi, t, &mut go, label);
                }
                let mut m = l.fx.trim_mode as usize;
                if pro::segmented(ui, &mut m, &["Simultané", "Un par un"]) {
                    l.fx.trim_mode = m as u8;
                }
                let fx = &mut l.fx;
                sub(ui, "Opérateurs de forme");
                amount(ui, "Arrondir les angles", &mut fx.round, 0.0..=300.0, " px");
                amount(ui, "Fronce et dilatation", &mut fx.pucker, -100.0..=100.0, " %");
                if amount(ui, "Zigzag", &mut fx.zig_amp, -100.0..=100.0, " px") {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut fx.zig_ridges).range(1..=200).prefix("Crêtes "));
                        ui.checkbox(&mut fx.zig_smooth, "Ondulé");
                    });
                }
                if amount(ui, "Agitation", &mut fx.rough, 0.0..=100.0, " px") {
                    slider(ui, "Détail", &mut fx.rough_detail, 0.5..=40.0, " /100 px");
                    slider(ui, "Vitesse d'agitation", &mut fx.rough_speed, 0.0..=20.0, " /s");
                }
                amount(ui, "Torsion", &mut fx.twist, -720.0..=720.0, "°");
            });
            if let Some(g) = go {
                self.t = g;
            }
        }
        let l = &mut self.comp.layers[i];
        let t_now = t;
        pro::dock_panel(ui, "look", "Apparence", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            let mut on = l.fx.stroke.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut on, "Contour").changed() {
                    l.fx.stroke = on.then_some([255, 255, 255, 255]);
                    if l.value(17, t_now) <= 0.0 {
                        l.props[17].set(t_now, 4.0);
                        l.fx.stroke_w = 4.0;
                    }
                }
                if let Some(c) = &mut l.fx.stroke {
                    let before = *c;
                    color_button(ui, c);
                    if !l.stroke_keys.is_empty() && *c != before {
                        set_color_key(&mut l.stroke_keys, t_now, *c);
                    }
                }
            });
            if l.fx.stroke.is_some() {
                let mut go = None;
                prop_row(ui, l, 17, t_now, &mut go, "Épaisseur px");
                let cur = l.fx.stroke;
                color_anim(ui, &mut l.stroke_keys, t_now, cur, "le contour");
                let fx = &mut l.fx;
                if amount(ui, "Pointillés (0 = plein)", &mut fx.dash, 0.0..=200.0, " px") {
                    slider(ui, "Espace entre tirets", &mut fx.dash_gap, 5.0..=400.0, " %");
                }
                ui.label(RichText::new("Extrémités et angles").size(12.5).color(MUTED));
                let mut c = fx.cap as usize;
                if pro::segmented(ui, &mut c, &["Plates", "Rondes", "Carrées"]) {
                    fx.cap = c as u8;
                }
                let mut j = fx.join as usize;
                if pro::segmented(ui, &mut j, &["Arrondis", "Pointus", "Biseautés"]) {
                    fx.join = j as u8;
                }
            }
            let fx = &mut l.fx;
            let mut grad = fx.gradient.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut grad, "Dégradé vers").changed() {
                    fx.gradient = grad.then_some([10, 132, 255, 255]);
                }
                if let Some(c) = &mut fx.gradient {
                    color_button(ui, c);
                }
            });
            if fx.gradient.is_some() {
                let mut k = fx.gradient_kind as usize;
                if pro::segmented(ui, &mut k, &["Linéaire", "Radial"]) {
                    fx.gradient_kind = k as u8;
                }
                if fx.gradient_kind == 0 {
                    slider(ui, "Angle du dégradé", &mut fx.gradient_angle, -180.0..=180.0, "°");
                } else {
                    slider(ui, "Centre X", &mut fx.grad_cx, 0.0..=100.0, " %");
                    slider(ui, "Centre Y", &mut fx.grad_cy, 0.0..=100.0, " %");
                    slider(ui, "Rayon", &mut fx.grad_r, 5.0..=200.0, " %");
                }
            }
            ui.label(RichText::new("Mode de fusion").size(12.5).color(MUTED));
            egui::ComboBox::from_id_salt("blend").selected_text(BLENDS.get(fx.blend as usize).map_or("Normal", |b| b.1)).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                for (k, (_, name)) in BLENDS.iter().enumerate() {
                    ui.selectable_value(&mut fx.blend, k as u8, *name);
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut fx.flip_x, "Miroir horizontal");
                ui.checkbox(&mut fx.flip_y, "Vertical");
            });
        });
        let mut go = None;
        let mut help_open = false;
        pro::dock_panel(ui, "fx", "Flou et lumière", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            prop_row(ui, l, 6, t_now, &mut go, "Flou animé px");
            let fx = &mut l.fx;
            slider(ui, "Flou fixe", &mut fx.blur, 0.0..=80.0, " px");
            if amount(ui, "Flou directionnel", &mut fx.dir_blur, 0.0..=120.0, " px") {
                slider(ui, "Direction", &mut fx.dir_angle, -180.0..=180.0, "°");
            }
            amount(ui, "Flou radial (zoom)", &mut fx.zoom_blur, 0.0..=100.0, " %");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Lueur").size(12.5).color(MUTED));
                color_button(ui, &mut fx.glow_color);
            });
            ui.add(egui::Slider::new(&mut fx.glow, 0.0..=80.0).suffix(" px"));
            ui.horizontal(|ui| {
                ui.checkbox(&mut fx.shadow, "Ombre portée");
                if fx.shadow {
                    color_button(ui, &mut fx.shadow_color);
                }
            });
            if fx.shadow {
                slider(ui, "Décalage X", &mut fx.shadow_dx, -100.0..=100.0, " px");
                slider(ui, "Décalage Y", &mut fx.shadow_dy, -100.0..=100.0, " px");
                slider(ui, "Douceur", &mut fx.shadow_blur, 0.0..=80.0, " px");
                slider(ui, "Opacité", &mut fx.shadow_alpha, 0.0..=100.0, " %");
            }
            ui.horizontal(|ui| {
                ui.checkbox(&mut fx.inner, "Ombre interne");
                if fx.inner {
                    color_button(ui, &mut fx.inner_color);
                }
            });
            if fx.inner {
                slider(ui, "Décalage X ", &mut fx.inner_dx, -60.0..=60.0, " px");
                slider(ui, "Décalage Y ", &mut fx.inner_dy, -60.0..=60.0, " px");
                slider(ui, "Douceur ", &mut fx.inner_blur, 0.0..=60.0, " px");
            }
        });
        pro::dock_panel(ui, "color", "Couleur", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            prop_row(ui, l, 18, t_now, &mut go, "Teinte animée °");
            let fx = &mut l.fx;
            slider(ui, "Teinte", &mut fx.hue, -180.0..=180.0, "°");
            slider(ui, "Saturation", &mut fx.saturation, 0.0..=300.0, " %");
            slider(ui, "Luminosité", &mut fx.brightness, 0.0..=300.0, " %");
            slider(ui, "Contraste", &mut fx.contrast, 0.0..=300.0, " %");
            slider(ui, "Gamma", &mut fx.gamma, 0.2..=3.0, "");
            amount(ui, "Sépia", &mut fx.sepia, 0.0..=100.0, " %");
            if amount(ui, "Teinte bicolore", &mut fx.tint, 0.0..=100.0, " %") {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Ombres").size(12.0).color(MUTED));
                    color_button(ui, &mut fx.tint_dark);
                    ui.label(RichText::new("Lumières").size(12.0).color(MUTED));
                    color_button(ui, &mut fx.tint_light);
                });
            }
            amount(ui, "Inversion", &mut fx.invert, 0.0..=100.0, " %");
            amount(ui, "Seuil (0 = non)", &mut fx.threshold, 0.0..=100.0, " %");
            let mut po = fx.posterize as f32;
            slider(ui, "Postérisation (0 = non)", &mut po, 0.0..=16.0, " niv.");
            fx.posterize = if po < 1.5 { 0 } else { po.round() as u32 };
            ui.horizontal(|ui| {
                ui.checkbox(&mut fx.fill_on, "Remplir d'une couleur");
                if fx.fill_on {
                    color_button(ui, &mut fx.fill_color);
                }
            });
            if ui.small_button("Réinitialiser la couleur").clicked() {
                let d = Fx::default();
                (fx.hue, fx.saturation, fx.brightness, fx.contrast, fx.gamma, fx.sepia, fx.tint, fx.invert, fx.threshold, fx.posterize, fx.fill_on) = (d.hue, d.saturation, d.brightness, d.contrast, d.gamma, d.sepia, d.tint, d.invert, d.threshold, d.posterize, d.fill_on);
            }
        });
        pro::dock_panel(ui, "stylize", "Stylisation", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            let fx = &mut l.fx;
            if amount(ui, "Séparation RVB", &mut fx.rgb_split, 0.0..=60.0, " px") {
                slider(ui, "Angle RVB", &mut fx.rgb_angle, -180.0..=180.0, "°");
            }
            if amount(ui, "Glitch", &mut fx.glitch, 0.0..=300.0, " px") {
                slider(ui, "Fréquence du glitch", &mut fx.glitch_freq, 0.5..=30.0, " /s");
            }
            amount(ui, "Grain", &mut fx.grain, 0.0..=100.0, " %");
            if amount(ui, "Distorsion", &mut fx.distort, 0.0..=200.0, " px") {
                slider(ui, "Taille des ondes", &mut fx.distort_scale, 4.0..=400.0, " px");
                slider(ui, "Évolution", &mut fx.distort_speed, 0.0..=10.0, " /s");
            }
            amount(ui, "Scintillement", &mut fx.flicker, 0.0..=100.0, " %");
        });
        if is_text {
            pro::dock_panel(ui, "textanim", "Animateur de texte", |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                let fx = &mut l.fx;
                slider(ui, "Machine à écrire (0 = non)", &mut fx.typewriter, 0.0..=10.0, " s");
                let ta = &mut fx.ta;
                ui.checkbox(&mut ta.on, "Animer lettre par lettre");
                if ta.on {
                    let mut m = ta.mode as usize;
                    if pro::segmented(ui, &mut m, &["Entrée", "Sortie", "Les deux"]) {
                        ta.mode = m as u8;
                    }
                    ui.label(RichText::new("Ordre").size(12.5).color(MUTED));
                    egui::ComboBox::from_id_salt("ta-order").selected_text(TEXT_ORDERS[(ta.order as usize).min(4)]).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        for (k, n) in TEXT_ORDERS.iter().enumerate() {
                            ui.selectable_value(&mut ta.order, k as u8, *n);
                        }
                    });
                    slider(ui, "Décalage entre lettres", &mut ta.delay, 0.0..=0.5, " s");
                    slider(ui, "Durée par lettre", &mut ta.dur, 0.02..=3.0, " s");
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Courbe").size(12.5).color(MUTED));
                        egui::ComboBox::from_id_salt("ta-ease").selected_text(Ease::ALL.iter().find(|e| e.0 == ta.ease).map_or("", |e| e.1)).show_ui(ui, |ui| {
                            for (e, n) in Ease::ALL {
                                ui.selectable_value(&mut ta.ease, e, n);
                            }
                        });
                    });
                    sub(ui, "Valeurs de départ");
                    slider(ui, "Opacité", &mut ta.opacity, 0.0..=100.0, " %");
                    slider(ui, "Décalage X", &mut ta.dx, -500.0..=500.0, " px");
                    slider(ui, "Décalage Y", &mut ta.dy, -500.0..=500.0, " px");
                    slider(ui, "Échelle", &mut ta.scale, 0.0..=400.0, " %");
                    slider(ui, "Rotation", &mut ta.rot, -360.0..=360.0, "°");
                }
            });
        }
        pro::dock_panel(ui, "motion", "Mouvement", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            let fx = &mut l.fx;
            slider(ui, "Tremblement de position", &mut fx.wiggle, 0.0..=200.0, " px");
            if fx.wiggle > 0.0 {
                if fx.wiggle_freq <= 0.0 {
                    fx.wiggle_freq = 2.0;
                }
                slider(ui, "Fréquence", &mut fx.wiggle_freq, 0.1..=12.0, " /s");
            }
            ui.label(RichText::new("Répéter toutes les clés").size(12.5).color(MUTED));
            let mut mode = if fx.pingpong { 2 } else if fx.loop_keys { 1 } else { 0 };
            if pro::segmented(ui, &mut mode, &["Non", "Boucle", "Aller-retour"]) {
                fx.loop_keys = mode == 1;
                fx.pingpong = mode == 2;
            }
            ui.add_space(4.0);
            let mut echo = fx.echo as f32;
            slider(ui, "Écho (traînée)", &mut echo, 0.0..=12.0, " copies");
            fx.echo = echo.round() as u32;
            if fx.echo > 0 {
                slider(ui, "Délai entre échos", &mut fx.echo_delay, 0.01..=0.5, " s");
            }
            let mut rep = fx.repeat as f32;
            slider(ui, "Répéteur", &mut rep, 0.0..=40.0, " copies");
            fx.repeat = rep.round() as u32;
            if fx.repeat > 0 {
                slider(ui, "Décalage X", &mut fx.rep_dx, -1000.0..=1000.0, " px");
                slider(ui, "Décalage Y", &mut fx.rep_dy, -1000.0..=1000.0, " px");
                slider(ui, "Rotation par copie", &mut fx.rep_rot, -180.0..=180.0, "°");
                slider(ui, "Échelle par copie", &mut fx.rep_scale, 20.0..=200.0, " %");
                slider(ui, "Fondu par copie", &mut fx.rep_fade, 0.0..=100.0, " %");
            }
        });
        pro::dock_panel(ui, "time", "Temps", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            let fx = &mut l.fx;
            slider(ui, "Vitesse", &mut fx.speed, 5.0..=800.0, " %");
            slider(ui, "Décalage temporel", &mut fx.time_offset, -10.0..=10.0, " s");
            ui.horizontal(|ui| {
                ui.checkbox(&mut fx.reverse_time, "Lecture inversée");
                if ui.checkbox(&mut fx.freeze, "Figer").on_hover_text("Fige l'image du calque à l'instant choisi").changed() && fx.freeze {
                    fx.freeze_at = t_now;
                }
            });
            if fx.freeze {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut fx.freeze_at).speed(0.02).suffix(" s").prefix("Figé à "));
                    if ui.small_button("Tête").clicked() {
                        fx.freeze_at = t_now;
                    }
                });
            }
            if ui.small_button("Réinitialiser le temps").clicked() {
                (fx.speed, fx.time_offset, fx.reverse_time, fx.freeze) = (100.0, 0.0, false, false);
            }
        });
        pro::dock_panel(ui, "matte", "Masque", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let fx = &mut l.fx;
            egui::ComboBox::from_id_salt("matte").selected_text(MATTES[(fx.matte as usize).min(MATTES.len() - 1)]).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                for (k, n) in MATTES.iter().enumerate() {
                    ui.selectable_value(&mut fx.matte, k as u8, *n);
                }
            });
            ui.label(RichText::new("Le calque juste au-dessus dans la timeline sert de pochoir : il devient invisible et ne laisse voir ce calque qu'à travers sa forme (ou en dehors, en mode inversé).").size(11.5).color(FAINT));
            ui.add_space(4.0);
            if pro::secondary(ui, "?  Tutoriels").clicked() {
                help_open = true;
            }
        });
        if let Some(g) = go {
            self.t = g;
        }
        if help_open {
            self.help.learn("Effets");
        }
    }
}
