use super::*;
use model::BEZ_PRESETS;

fn item(ui: &mut Ui, enabled: bool, label: &str, key: &str) -> bool {
    let b = egui::Button::new(label).shortcut_text(key);
    let r = ui.add_enabled(enabled, b).clicked();
    if r {
        ui.close();
    }
    r
}

impl Motion {
    pub(crate) fn create_null_parent(&mut self) {
        let Some(i) = self.sel else { return };
        let t = self.t;
        let (x, y, _, _) = self.comp.world(i, t);
        let n = self.push_layer(Layer::new(format!("Nul {}", self.comp.layers.len() + 1), Content::Null, x.round(), y.round(), self.comp.duration));
        self.comp.layers[n].normalize();
        let child = if n <= i { i + 1 } else { i };
        self.comp.set_parent(child, Some(n), t);
        self.sel = Some(n);
    }

    pub(crate) fn menus(&mut self, ui: &mut Ui) {
        let has = self.sel.is_some();
        ui.menu_button("Fichier", |ui| {
            if let Some(t) = studio_ui::handoff::menu(ui, &[("cut", "ajouter au montage"), ("pixel", "retoucher l'image"), ("page", "mettre en page"), ("vector", "placer l'image"), ("lumen", "développer")]) {
                let ctx = ui.ctx().clone();
                self.send_to(t, &ctx);
            }
            ui.separator();
            if item(ui, true, "Nouvelle composition", "") {
                self.home = true;
            }
            if item(ui, true, "Ouvrir…", "Ctrl+O") {
                self.open();
            }
            ui.menu_button("Ouvrir un projet récent", |ui| {
                let mut open = None;
                for p in &self.recents {
                    if ui.button(p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).clicked() {
                        open = Some(p.clone());
                    }
                }
                if self.recents.is_empty() {
                    ui.label(RichText::new("Aucun projet récent").color(FAINT));
                }
                if let Some(p) = open {
                    self.open_path(&p);
                }
            });
            let on = !self.home;
            ui.separator();
            if item(ui, on, "Enregistrer", "Ctrl+S") {
                self.save(false);
            }
            if item(ui, on, "Enregistrer sous…", "Ctrl+Maj+S") {
                self.save(true);
            }
            ui.separator();
            if item(ui, on, "Exporter…", "Ctrl+M") {
                self.export = Some(self.default_export());
            }
            if item(ui, on, "File de rendu…", "Ctrl+Alt+M") {
                self.queue_sheet = true;
            }
            ui.menu_button("Exporter directement", |ui| {
                for (f, label) in [(3, "Image courante en PNG…"), (4, "Image courante en SVG…"), (6, "Planche contact PNG…"), (7, "Animation Lottie JSON…"), (1, "GIF animé…")] {
                    if item(ui, on, label, "") {
                        let mut o = self.default_export();
                        o.format = f;
                        self.run_export(&o);
                    }
                }
            });
        });
        if self.home {
            self.help.menu(ui);
            return;
        }
        ui.menu_button("Édition", |ui| {
            if item(ui, !self.undo.is_empty(), "Annuler", "Ctrl+Z") {
                self.undo_step();
            }
            if item(ui, !self.redo.is_empty(), "Rétablir", "Ctrl+Y") {
                self.redo_step();
            }
            ui.separator();
            if item(ui, has, if self.ksel.is_empty() { "Copier le calque" } else { "Copier les clés" }, "Ctrl+C") {
                if self.ksel.is_empty() {
                    self.clip_layer = self.sel.and_then(|i| self.comp.layers.get(i).cloned());
                    self.clip_keys = None;
                } else {
                    self.copy_keys();
                }
            }
            if item(ui, self.clip_layer.is_some() || self.clip_keys.is_some(), if self.clip_keys.is_some() { "Coller les clés à la tête" } else { "Coller le calque" }, "Ctrl+V") {
                if self.clip_keys.is_some() {
                    self.paste_keys();
                } else if let Some(mut l) = self.clip_layer.clone() {
                    l.parent = None;
                    self.push_layer(l);
                }
            }
            if item(ui, has, "Dupliquer", "Ctrl+D") {
                self.duplicate();
            }
            if item(ui, has, "Supprimer", "Suppr") {
                if self.ksel.is_empty() {
                    self.delete();
                } else {
                    self.delete_keys();
                }
            }
            ui.separator();
            if item(ui, has, "Copier l'animation", "") {
                self.copy_anim();
            }
            if item(ui, has && self.clip_anim.is_some(), "Coller l'animation à la tête de lecture", "") {
                self.paste_anim();
            }
            ui.separator();
            if item(ui, has, "Sélectionner toutes les clés du calque", "Ctrl+A") {
                if let Some(i) = self.sel {
                    self.ksel = self.comp.layers[i].props.iter().enumerate().flat_map(|(pi, p)| p.keys.iter().map(move |k| (i, pi, k.t))).collect();
                }
            }
            if item(ui, has || !self.ksel.is_empty(), "Tout désélectionner", "F2") {
                self.sel = None;
                self.ksel.clear();
            }
        });
        ui.menu_button("Calque", |ui| {
            ui.menu_button("Nouveau", |ui| {
                for (label, kind) in KINDS {
                    if ui.button(label).clicked() {
                        self.add_kind(kind);
                        ui.close();
                    }
                }
            });
            ui.menu_button("Modèle", |ui| {
                for (k, (name, _)) in TEMPLATES.iter().enumerate() {
                    if ui.button(*name).clicked() {
                        self.template(k);
                        ui.close();
                    }
                }
            });
            ui.separator();
            if item(ui, has, "Fractionner à la tête", "Ctrl+Maj+D") {
                self.split();
            }
            if item(ui, has, "Monter", "Ctrl+]") {
                self.reorder(1);
            }
            if item(ui, has, "Descendre", "Ctrl+[") {
                self.reorder(-1);
            }
            if item(ui, has, "Premier plan", "") && let Some(i) = self.sel {
                self.move_layer(i, self.comp.layers.len() - 1);
            }
            if item(ui, has, "Arrière-plan", "") && let Some(i) = self.sel {
                self.move_layer(i, 0);
            }
            ui.separator();
            ui.add_enabled_ui(has, |ui| {
                ui.menu_button("Transformer", |ui| {
                    if item(ui, true, "Centrer le point d'ancrage", "Ctrl+Alt+Origine") {
                        self.anchor_to(4);
                    }
                    if item(ui, true, "Ajuster à la composition", "Ctrl+Alt+F") {
                        self.fit_to_comp(0);
                    }
                    if item(ui, true, "Ajuster à la largeur", "") {
                        self.fit_to_comp(1);
                    }
                    if item(ui, true, "Ajuster à la hauteur", "") {
                        self.fit_to_comp(2);
                    }
                    if item(ui, true, "Centrer dans la composition", "") {
                        self.align(6);
                    }
                    if let Some(i) = self.sel {
                        let t = self.t;
                        let l = &mut self.comp.layers[i];
                        ui.separator();
                        if item(ui, true, "Rotation +90°", "") {
                            let v = l.props[3].at(t) + 90.0;
                            l.props[3].set(t, v);
                        }
                        if item(ui, true, "Rotation −90°", "") {
                            let v = l.props[3].at(t) - 90.0;
                            l.props[3].set(t, v);
                        }
                        if item(ui, true, "Retourner horizontalement", "") {
                            l.fx.flip_x = !l.fx.flip_x;
                        }
                        if item(ui, true, "Retourner verticalement", "") {
                            l.fx.flip_y = !l.fx.flip_y;
                        }
                        ui.checkbox(&mut l.fx.auto_orient, "Orientation automatique");
                        if item(ui, true, "Réinitialiser la transformation", "") {
                            for pi in [2, 3, 4, 5, 8, 9, 10, 11, 12, 13] {
                                l.props[pi] = Prop::new(model::PROP_DEFAULT[pi]);
                            }
                        }
                    }
                });
                ui.menu_button("Temps", |ui| {
                    if let Some(i) = self.sel {
                        let t = self.t;
                        let dur = self.comp.duration;
                        let l = &mut self.comp.layers[i];
                        if item(ui, true, "Commencer à la tête de lecture", "Alt+[") {
                            l.start = t.min(l.end - 0.05);
                        }
                        if item(ui, true, "Finir à la tête de lecture", "Alt+]") {
                            l.end = t.max(l.start + 0.05);
                        }
                        if item(ui, true, "Déplacer le début à la tête", "[") {
                            let d = t - l.start;
                            l.shift(d);
                        }
                        if item(ui, true, "Déplacer la fin à la tête", "]") {
                            let d = t - l.end;
                            l.shift(d);
                        }
                        if item(ui, true, "Étendre à toute la composition", "") {
                            l.start = 0.0;
                            l.end = dur;
                        }
                        ui.separator();
                        ui.checkbox(&mut l.fx.reverse_time, "Lecture inversée");
                        if item(ui, true, "Figer l'image à la tête", "") {
                            l.fx.freeze = true;
                            l.fx.freeze_at = t;
                        }
                        for (n, v) in [("Vitesse 50 %", 50.0), ("Vitesse 100 %", 100.0), ("Vitesse 200 %", 200.0)] {
                            if item(ui, true, n, "") {
                                l.fx.speed = v;
                            }
                        }
                    }
                });
                ui.menu_button("Mode de fusion", |ui| {
                    if let Some(i) = self.sel {
                        for (k, (_, name)) in BLENDS.iter().enumerate() {
                            ui.selectable_value(&mut self.comp.layers[i].fx.blend, k as u8, *name);
                        }
                    }
                });
                ui.menu_button("Masque", |ui| {
                    if let Some(i) = self.sel {
                        for (k, n) in MATTES.iter().enumerate() {
                            ui.selectable_value(&mut self.comp.layers[i].fx.matte, k as u8, *n);
                        }
                    }
                });
                ui.separator();
                if item(ui, true, "Créer un nul parent", "") {
                    self.create_null_parent();
                }
                if item(ui, self.sel.is_some_and(|i| self.comp.layers[i].parent.is_some()), "Détacher du parent", "") && let Some(i) = self.sel {
                    let t = self.t;
                    self.comp.set_parent(i, None, t);
                }
                if let Some(i) = self.sel {
                    let l = &mut self.comp.layers[i];
                    ui.separator();
                    ui.checkbox(&mut l.visible, "Visible");
                    ui.checkbox(&mut l.locked, "Verrouillé");
                    ui.checkbox(&mut l.solo, "Solo");
                }
            });
        });
        ui.menu_button("Animation", |ui| {
            let keys = has || !self.ksel.is_empty();
            if item(ui, keys, "Courbe douce", "F9") {
                self.easy_ease(0);
            }
            if item(ui, keys, "Ralentir à l'arrivée", "Maj+F9") {
                self.easy_ease(1);
            }
            if item(ui, keys, "Démarrer en douceur", "Ctrl+Maj+F9") {
                self.easy_ease(2);
            }
            if item(ui, keys, "Maintien (bascule)", "Ctrl+Alt+H") {
                self.with_keys(|k| k.set_ease(if k.ease == Ease::Hold && k.ease2 == Ease2::None && k.bez.is_none() { Ease::Linear } else { Ease::Hold }));
            }
            if item(ui, keys, "Linéaire", "") {
                self.with_keys(|k| k.set_ease(Ease::Linear));
            }
            ui.add_enabled_ui(keys, |ui| {
                ui.menu_button("Courbe de Bézier", |ui| {
                    for (n, b) in BEZ_PRESETS {
                        if item(ui, true, n, "") {
                            self.with_keys(|k| k.bez = Some(b));
                        }
                    }
                });
            });
            ui.separator();
            if item(ui, has, "Répartir les clés uniformément", "") {
                self.spread_keys();
            }
            if item(ui, has, "Inverser les images clés", "Ctrl+Alt+R") {
                self.reverse_keys();
            }
            for (n, k) in [("Étirer les clés × 2", 2.0), ("Compresser les clés × ½", 0.5)] {
                if item(ui, has, n, "") && let Some(i) = self.sel {
                    let l = &mut self.comp.layers[i];
                    let from = l.key_times().first().copied().unwrap_or(0.0);
                    for p in &mut l.props {
                        p.stretch(from, k);
                    }
                }
            }
            if item(ui, has, "Supprimer toutes les images clés", "") && let Some(i) = self.sel {
                let t = self.t;
                for p in &mut self.comp.layers[i].props {
                    p.clear(t);
                }
                self.comp.layers[i].color_keys.clear();
                self.comp.layers[i].stroke_keys.clear();
                self.ksel.clear();
            }
            ui.separator();
            ui.add_enabled_ui(has, |ui| {
                ui.menu_button("Animations prêtes", |ui| {
                    for (gi, g) in GROUPS.iter().enumerate() {
                        ui.menu_button(*g, |ui| {
                            for (name, _, grp, id) in ANIMS {
                                if grp == gi && item(ui, true, name, "") {
                                    self.preset(id);
                                }
                            }
                        });
                    }
                });
            });
            ui.menu_button("Afficher dans la timeline", |ui| {
                for (n, k, props) in [("Position", "P", vec![0, 1]), ("Échelle", "S", vec![2, 10, 11]), ("Rotation", "R", vec![3, 12, 13]), ("Opacité", "T", vec![4]), ("Ancre", "A", vec![8, 9]), ("Tracé", "", vec![14, 15, 16, 17])] {
                    if item(ui, has, n, k) {
                        self.tl_filter = Some(props);
                        if let Some(e) = self.sel.and_then(|i| self.expanded.get_mut(i)) {
                            *e = true;
                        }
                    }
                }
                if item(ui, true, "Propriétés animées", "U") {
                    self.tl_filter = None;
                }
            });
        });
        ui.menu_button("Composition", |ui| {
            if item(ui, true, "Réglages de la composition…", "Ctrl+K") {
                self.comp_sheet = true;
            }
            if item(ui, true, "Ajuster la durée à l'animation", "") {
                self.fit_duration();
            }
            ui.menu_button("Durée", |ui| {
                for d in [2.0, 3.0, 5.0, 10.0, 15.0, 30.0, 60.0] {
                    if item(ui, true, &secs(d), "") {
                        self.comp.duration = d;
                    }
                }
            });
            ui.menu_button("Images par seconde", |ui| {
                for f in FPS_CHOICES {
                    if ui.selectable_label(self.comp.fps == f, format!("{f} i/s")).clicked() {
                        self.comp.fps = f;
                        ui.close();
                    }
                }
            });
            ui.menu_button("Format", |ui| {
                for (name, kind, w, h) in PRESETS {
                    if ui.selectable_label(self.comp.width == w && self.comp.height == h, format!("{name} · {w} × {h} · {kind}")).clicked() {
                        self.comp.width = w;
                        self.comp.height = h;
                        ui.close();
                    }
                }
            });
            ui.separator();
            if item(ui, true, "Ajouter un repère", "M") {
                self.add_marker();
            }
            if item(ui, !self.comp.markers.is_empty(), "Supprimer tous les repères", "") {
                self.comp.markers.clear();
            }
            ui.separator();
            if item(ui, true, "Début de la zone de travail", "B") {
                self.set_work(true);
            }
            if item(ui, true, "Fin de la zone de travail", "N") {
                self.set_work(false);
            }
            if item(ui, self.comp.work.is_some(), "Effacer la zone de travail", "") {
                self.comp.work = None;
            }
            ui.separator();
            ui.menu_button("Finition", |ui| {
                for (n, v) in [("Sans vignettage", 0.0), ("Vignettage léger", 30.0), ("Vignettage fort", 70.0)] {
                    if ui.selectable_label(self.comp.vignette == v, n).clicked() {
                        self.comp.vignette = v;
                    }
                }
                ui.separator();
                for (n, v) in [("Sans grain", 0.0), ("Grain léger", 15.0), ("Grain marqué", 40.0)] {
                    if ui.selectable_label(self.comp.grain == v, n).clicked() {
                        self.comp.grain = v;
                    }
                }
                ui.separator();
                for (n, v) in [("Sans bandes", 0.0), ("Bandes 2,39:1", 2.39), ("Bandes 1,85:1", 1.85)] {
                    if ui.selectable_label(self.comp.letterbox == v, n).clicked() {
                        self.comp.letterbox = v;
                    }
                }
            });
        });
        ui.menu_button("Affichage", |ui| {
            ui.menu_button("Espace de travail", studio_ui::dock::menu);
            ui.separator();
            ui.checkbox(&mut self.grid, "Grille et tiers                  Ctrl+'");
            ui.menu_button("Divisions de la grille", |ui| {
                for n in [4u32, 6, 8, 10, 12, 16, 20] {
                    ui.selectable_value(&mut self.grid_div, n, n.to_string());
                }
            });
            ui.checkbox(&mut self.safe, "Zones sûres");
            ui.menu_button("Type de zones sûres", |ui| {
                for (k, (n, _)) in SAFE.iter().enumerate() {
                    ui.selectable_value(&mut self.safe_kind, k, *n);
                }
            });
            ui.checkbox(&mut self.guides, "Axes centraux");
            ui.checkbox(&mut self.rulers, "Règles                              Ctrl+R");
            ui.checkbox(&mut self.user_guides, "Repères de mise en page   Ctrl+;");
            if item(ui, !self.comp.guides.is_empty(), "Effacer les repères", "") {
                self.comp.guides.clear();
            }
            ui.separator();
            ui.checkbox(&mut self.checker, "Damier de transparence");
            ui.checkbox(&mut self.paths, "Trajectoires de mouvement");
            ui.checkbox(&mut self.onion, "Pelure d'oignon");
            ui.menu_button("Réglages de la pelure", |ui| {
                ui.add(egui::DragValue::new(&mut self.onion_n).range(1..=3).prefix("Images de chaque côté "));
                ui.add(egui::DragValue::new(&mut self.onion_step).range(1..=15).prefix("Écart "));
            });
            ui.checkbox(&mut self.snap, "Magnétisme");
            ui.checkbox(&mut self.loop_play, "Lecture en boucle               L");
            ui.menu_button("Qualité de l'aperçu", |ui| {
                for (k, q) in QUALITY.iter().enumerate() {
                    ui.selectable_value(&mut self.quality, k, *q);
                }
            });
            ui.separator();
            if item(ui, true, "Ajuster à la fenêtre", "Ctrl+0") {
                self.zoom = None;
                self.pan = Vec2::ZERO;
            }
            if item(ui, true, "Taille réelle (100 %)", "Ctrl+1") {
                self.zoom = Some(1.0);
            }
            if item(ui, true, "Zoom avant", ".") {
                let z0 = self.zoom.unwrap_or(0.5);
                self.zoom = Some(ZOOMS.iter().copied().find(|z| *z > z0 + 1e-3).unwrap_or(8.0));
            }
            if item(ui, true, "Zoom arrière", ",") {
                let z0 = self.zoom.unwrap_or(0.5);
                self.zoom = Some(ZOOMS.iter().rev().copied().find(|z| *z < z0 - 1e-3).unwrap_or(0.1));
            }
        });
        self.help.menu(ui);
    }
}
