use crate::doc::{BLENDS, Document, Xform, fade};
use crate::shapes::{self, TextStyle};
use crate::tools::{self, blend_index};
use crate::{BACKGROUNDS, PASTEBOARDS, PRESETS, Pixel, Sheet, preset_card};
use eframe::egui::{self, Align, Color32, CornerRadius, Layout, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use image::{RgbaImage, imageops};
use studio_ui::pro::{self, BLUE, FAINT, FIELD, INK_STRONG, LINE, MUTED, semibold};

const RESAMPLE: [&str; 5] = ["Plus proche voisin", "Bilinéaire", "Bicubique", "Lanczos (net)", "Gaussien (doux)"];
const EXT_COLORS: [&str; 5] = ["Couleur secondaire", "Couleur principale", "Blanc", "Noir", "Transparent"];
const FILL_CONTENT: [&str; 6] = ["Couleur principale", "Couleur secondaire", "Noir", "Blanc", "Gris 50 %", "Selon le contenu"];
const COLOR_RANGE: [&str; 11] = ["Couleur principale", "Tons foncés", "Tons moyens", "Tons clairs", "Rouges", "Jaunes", "Verts", "Cyans", "Bleus", "Magentas", "Tons chair"];
const SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+N / Ctrl+O / Ctrl+W", "Nouveau, ouvrir, fermer"),
    ("Ctrl+E / Ctrl+Maj+Alt+S", "Exporter, exportation rapide PNG"),
    ("Ctrl+Z / Ctrl+Y", "Annuler, rétablir"),
    ("Ctrl+C / Ctrl+Maj+C", "Copier, copier avec fusion"),
    ("Ctrl+V / Ctrl+Maj+V", "Coller, coller en place"),
    ("Ctrl+J / Ctrl+Maj+J", "Calque par copie, par coupure"),
    ("Ctrl+T", "Transformation manuelle"),
    ("Ctrl+L / Ctrl+M / Ctrl+U / Ctrl+B", "Niveaux, courbes, teinte, balance"),
    ("Ctrl+I / Ctrl+Maj+U", "Négatif, noir et blanc"),
    ("Ctrl+Maj+L / Ctrl+Maj+B", "Ton auto, couleur auto"),
    ("Ctrl+Alt+F", "Répéter le dernier filtre"),
    ("Ctrl+Alt+I / Ctrl+Alt+C", "Taille de l'image, de la zone"),
    ("Ctrl+A / Ctrl+D / Ctrl+Maj+D", "Tout, désélectionner, resélectionner"),
    ("Ctrl+Maj+I / Maj+F6", "Intervertir, contour progressif"),
    ("Maj+F5 / Alt+Retour", "Remplir, remplir en couleur principale"),
    ("Ctrl+Maj+N / Ctrl+Alt+G", "Nouveau calque, écrêtage"),
    ("Ctrl+[ / Ctrl+]", "Descendre, monter le calque"),
    ("Ctrl+Maj+E / Ctrl+Alt+E", "Fusionner visibles, avec le dessous"),
    ("Ctrl+R / Ctrl+' / Ctrl+;", "Règles, grille, repères"),
    ("Ctrl+Maj+; / Ctrl+H", "Magnétisme, extras"),
    ("Ctrl+0 / Ctrl+1 / Ctrl++ / Ctrl+−", "Ajuster, 100 %, zoom"),
    ("Lettre / Maj+lettre", "Outil, outil suivant du groupe"),
    ("[ ] / Maj+[ ]", "Taille, dureté du pinceau"),
    ("0 à 9", "Flux ou opacité de l'outil"),
    ("X / D", "Inverser, couleurs par défaut"),
    ("\\", "Modifier le masque du calque"),
    ("Flèches / Maj+flèches", "Déplacer le calque de 1 ou 10 px"),
    ("Espace / Tab / F1", "Main, masquer les panneaux, aide"),
];

fn label(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(12.5).color(MUTED));
}

fn color_row(ui: &mut Ui, name: &str, color: &mut [u8; 4]) {
    ui.horizontal(|ui| {
        label(ui, name);
        let mut c = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
        if ui.color_edit_button_srgba(&mut c).changed() {
            *color = c.to_srgba_unmultiplied();
        }
    });
}

fn buttons(ui: &mut Ui, ok: &str, apply: &mut bool, cancel: &mut bool) {
    ui.add_space(14.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if pro::primary(ui, ok).clicked() {
            *apply = true;
        }
        if pro::secondary(ui, "Annuler").clicked() {
            *cancel = true;
        }
    });
}

impl Pixel {
    pub fn sheets(&mut self, ctx: &egui::Context) {
        self.extra_sheet(ctx);
        self.new_doc_sheet_ui(ctx);
        self.adjust_sheet(ctx);
        self.text_sheet(ctx);
    }

    fn extra_sheet(&mut self, ctx: &egui::Context) {
        let Some(mut sheet) = self.sheet.take() else { return };
        let (mut apply, mut cancel) = (false, false);
        let title = match &sheet {
            Sheet::Canvas { .. } => "Taille de la zone de travail".to_string(),
            Sheet::ImageSize { .. } => "Taille de l'image".into(),
            Sheet::Transform(_) => "Transformation du calque".into(),
            Sheet::Effect { kind, .. } => ["Ombre portée", "Lueur externe", "Contour du calque", "Contour de la sélection", "Ombre interne", "Lueur interne", "Biseau et estampage", "Incrustation couleur", "Incrustation dégradé"][*kind as usize % 9].into(),
            Sheet::SelectBy { kind, .. } => ["Dilater la sélection", "Contracter la sélection", "Bordure de sélection", "Contour progressif", "Lisser la sélection"][*kind as usize % 5].into(),
            Sheet::ColorRange { .. } => "Plage de couleurs".into(),
            Sheet::Fill { .. } => "Remplir".into(),
            Sheet::Export { selection, .. } => if *selection { "Exporter la sélection".into() } else { "Exporter".into() },
            Sheet::RotateCanvas { .. } => "Rotation de l'image".into(),
            Sheet::Guide { .. } => "Nouveau repère".into(),
            Sheet::Note { .. } => "Annotation".into(),
            Sheet::SaveSelection { .. } => "Mémoriser la sélection".into(),
            Sheet::Prefs => "Préférences".into(),
            Sheet::Shortcuts => "Raccourcis clavier".into(),
        };
        let (dw, dh) = (self.doc.width(), self.doc.height());
        let keep = pro::sheet(ctx, "extra", if matches!(sheet, Sheet::Shortcuts) { 520.0 } else { 400.0 }, |ui| {
            ui.label(RichText::new(&title).font(semibold(18.0)).color(INK_STRONG));
            ui.add_space(12.0);
            ui.spacing_mut().slider_width = 250.0;
            match &mut sheet {
                Sheet::Canvas { w, h, anchor, relative, ext } => {
                    ui.checkbox(relative, "Relative (ajoute à la taille actuelle)");
                    let (mut rw, mut rh) = if *relative { (*w as i64 - dw as i64, *h as i64 - dh as i64) } else { (*w as i64, *h as i64) };
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut rw).range(if *relative { -(dw as i64 - 1)..=16384 } else { 1..=16384 }).prefix("L ").suffix(" px"));
                        ui.add(egui::DragValue::new(&mut rh).range(if *relative { -(dh as i64 - 1)..=16384 } else { 1..=16384 }).prefix("H ").suffix(" px"));
                    });
                    if *relative {
                        *w = (dw as i64 + rw).max(1) as u32;
                        *h = (dh as i64 + rh).max(1) as u32;
                    } else {
                        *w = rw.max(1) as u32;
                        *h = rh.max(1) as u32;
                    }
                    ui.add_space(8.0);
                    label(ui, "Position de l'image actuelle");
                    for row in 0..3 {
                        ui.horizontal(|ui| {
                            for col in 0..3 {
                                let k = row * 3 + col;
                                let (r, resp) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
                                ui.painter().rect(r, CornerRadius::same(5), if *anchor == k { BLUE } else { FIELD }, Stroke::new(1.0, LINE), StrokeKind::Inside);
                                if resp.clicked() {
                                    *anchor = k;
                                }
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        label(ui, "Couleur d'extension");
                        tools::combo(ui, "ext", ext, &EXT_COLORS, 160.0);
                    });
                }
                Sheet::ImageSize { w, h, keep, percent, resample, dpi } => {
                    let ratio = dw as f32 / dh as f32;
                    let mut unit = *percent as usize;
                    if pro::segmented(ui, &mut unit, &["Pixels", "Pourcentage"]) {
                        *percent = unit == 1;
                    }
                    if *percent {
                        let mut px = *w as f32 / dw as f32 * 100.0;
                        let mut py = *h as f32 / dh as f32 * 100.0;
                        label(ui, "Largeur");
                        if ui.add(egui::DragValue::new(&mut px).range(1.0..=1000.0).suffix(" %")).changed() {
                            *w = ((dw as f32 * px / 100.0).round() as u32).max(1);
                            if *keep {
                                *h = ((*w as f32 / ratio).round() as u32).max(1);
                            }
                        }
                        label(ui, "Hauteur");
                        if ui.add(egui::DragValue::new(&mut py).range(1.0..=1000.0).suffix(" %")).changed() {
                            *h = ((dh as f32 * py / 100.0).round() as u32).max(1);
                            if *keep {
                                *w = ((*h as f32 * ratio).round() as u32).max(1);
                            }
                        }
                    } else {
                        label(ui, "Largeur");
                        if ui.add(egui::DragValue::new(w).range(1..=16384).suffix(" px")).changed() && *keep {
                            *h = ((*w as f32 / ratio).round() as u32).max(1);
                        }
                        label(ui, "Hauteur");
                        if ui.add(egui::DragValue::new(h).range(1..=16384).suffix(" px")).changed() && *keep {
                            *w = ((*h as f32 * ratio).round() as u32).max(1);
                        }
                    }
                    ui.checkbox(keep, "Conserver les proportions");
                    ui.horizontal(|ui| {
                        label(ui, "Rééchantillonnage");
                        tools::combo(ui, "resample", resample, &RESAMPLE, 170.0);
                    });
                    ui.horizontal(|ui| {
                        label(ui, "Résolution");
                        ui.add(egui::DragValue::new(dpi).range(1.0..=2400.0).suffix(" ppp"));
                    });
                    ui.horizontal(|ui| {
                        for (name, k) in [("50 %", 0.5), ("200 %", 2.0), ("1920 px", -1920.0), ("1080 px", -1080.0)] {
                            if ui.small_button(name).clicked() {
                                if k > 0.0 {
                                    *w = ((dw as f32 * k) as u32).max(1);
                                    *h = ((dh as f32 * k) as u32).max(1);
                                } else {
                                    *w = (-k) as u32;
                                    *h = ((*w as f32 / ratio).round() as u32).max(1);
                                }
                            }
                        }
                    });
                    let cm = |px: u32| px as f32 / dpi.max(1.0) * 2.54;
                    ui.label(RichText::new(format!("Impression : {:.1} × {:.1} cm · {:.1} Mo", cm(*w), cm(*h), (*w as f64 * *h as f64 * 4.0) / 1_048_576.0)).size(12.0).color(FAINT));
                }
                Sheet::Transform(t) => {
                    let mut sx = t.sx * 100.0;
                    let mut sy = t.sy * 100.0;
                    let mut linked = (sx - sy).abs() < 0.01;
                    ui.horizontal(|ui| {
                        label(ui, "Échelle");
                        ui.add(egui::DragValue::new(&mut sx).range(1.0..=1000.0).suffix(" %").prefix("L "));
                        ui.add(egui::DragValue::new(&mut sy).range(1.0..=1000.0).suffix(" %").prefix("H "));
                        ui.checkbox(&mut linked, "Lier");
                    });
                    if linked && (sx / 100.0 - t.sx).abs() > 1e-4 {
                        sy = sx;
                    } else if linked && (sy / 100.0 - t.sy).abs() > 1e-4 {
                        sx = sy;
                    }
                    t.sx = sx / 100.0;
                    t.sy = sy / 100.0;
                    label(ui, "Rotation");
                    ui.add(egui::Slider::new(&mut t.angle, -180.0..=180.0).suffix("°"));
                    ui.horizontal(|ui| {
                        for a in [-90.0, -45.0, 0.0, 45.0, 90.0, 180.0] {
                            if ui.small_button(format!("{a:+}°")).clicked() {
                                t.angle = a;
                            }
                        }
                    });
                    label(ui, "Inclinaison horizontale / verticale");
                    ui.add(egui::Slider::new(&mut t.skew_x, -60.0..=60.0).suffix("°"));
                    ui.add(egui::Slider::new(&mut t.skew_y, -60.0..=60.0).suffix("°"));
                    label(ui, "Perspective horizontale / verticale");
                    ui.add(egui::Slider::new(&mut t.persp_x, -90.0..=90.0).suffix(" %"));
                    ui.add(egui::Slider::new(&mut t.persp_y, -90.0..=90.0).suffix(" %"));
                    ui.horizontal(|ui| {
                        label(ui, "Déplacement");
                        ui.add(egui::DragValue::new(&mut t.tx).prefix("X ").suffix(" px"));
                        ui.add(egui::DragValue::new(&mut t.ty).prefix("Y ").suffix(" px"));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut t.nearest, "Pixels nets (plus proche voisin)");
                        if ui.small_button("Symétrie H").clicked() {
                            t.sx = -t.sx;
                        }
                        if ui.small_button("Symétrie V").clicked() {
                            t.sy = -t.sy;
                        }
                        if ui.small_button("Réinitialiser").clicked() {
                            *t = Xform::identity();
                        }
                    });
                }
                Sheet::Effect { kind, size, dist, angle, color, color2, position, blend, depth } => {
                    let k = *kind;
                    if matches!(k, 0 | 1 | 4 | 5 | 6 | 2 | 3) {
                        label(ui, match k {
                            0 | 4 => "Flou",
                            2 | 3 => "Épaisseur",
                            _ => "Taille",
                        });
                        ui.add(egui::Slider::new(size, 0.0..=80.0).suffix(" px"));
                    }
                    if matches!(k, 0 | 4) {
                        label(ui, "Distance");
                        ui.add(egui::Slider::new(dist, 0.0..=200.0).suffix(" px"));
                    }
                    if matches!(k, 0 | 4 | 6 | 8) {
                        label(ui, "Angle");
                        ui.add(egui::Slider::new(angle, -180.0..=180.0).suffix("°"));
                    }
                    if k == 6 {
                        label(ui, "Profondeur");
                        ui.add(egui::Slider::new(depth, 0.1..=5.0));
                    }
                    if matches!(k, 2 | 3) {
                        ui.horizontal(|ui| {
                            label(ui, "Position");
                            let mut p = *position as usize;
                            if pro::segmented(ui, &mut p, &["Intérieur", "Centre", "Extérieur"]) {
                                *position = p as u8;
                            }
                        });
                    }
                    color_row(ui, if k == 6 { "Rehaut" } else if k == 8 { "Début" } else { "Couleur" }, color);
                    if matches!(k, 6 | 8) {
                        color_row(ui, if k == 6 { "Ombre" } else { "Fin" }, color2);
                    }
                    if matches!(k, 7 | 8) {
                        ui.horizontal(|ui| {
                            label(ui, "Mode");
                            let mut b = BLENDS[*blend].0;
                            if tools::blend_combo(ui, "fxblend", &mut b, 170.0) {
                                *blend = blend_index(b);
                            }
                        });
                    }
                    if k != 3 {
                        ui.label(RichText::new("L'effet est créé sur un calque séparé, modifiable et masquable.").size(12.0).color(FAINT));
                    }
                }
                Sheet::SelectBy { kind, px } => {
                    let max = if *kind >= 3 { 250.0 } else { 200.0 };
                    ui.add(egui::Slider::new(px, if *kind >= 3 { 0.5 } else { 1.0 }..=max).suffix(" px").logarithmic(true));
                }
                Sheet::ColorRange { mode, fuzz } => {
                    ui.horizontal(|ui| {
                        label(ui, "Sélectionner");
                        tools::combo(ui, "crange", mode, &COLOR_RANGE, 180.0);
                    });
                    if *mode == 0 {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                            ui.painter().rect(r, CornerRadius::same(4), self.primary, Stroke::new(1.0, LINE), StrokeKind::Inside);
                            label(ui, "Prends la couleur avec la pipette (I)");
                        });
                        label(ui, "Tolérance");
                        ui.add(egui::Slider::new(fuzz, 1.0..=200.0));
                    }
                    ui.label(RichText::new("La sélection obtenue est progressive : les pixels proches sont pris en partie.").size(12.0).color(FAINT));
                }
                Sheet::Fill { content, opacity, blend } => {
                    ui.horizontal(|ui| {
                        label(ui, "Contenu");
                        tools::combo(ui, "fillc", content, &FILL_CONTENT, 180.0);
                    });
                    if *content < 5 {
                        ui.horizontal(|ui| {
                            label(ui, "Mode");
                            let mut b = BLENDS[*blend].0;
                            if tools::blend_combo(ui, "fillb", &mut b, 180.0) {
                                *blend = blend_index(b);
                            }
                        });
                        label(ui, "Opacité");
                        ui.add(egui::Slider::new(opacity, 1.0..=100.0).suffix(" %"));
                    }
                }
                Sheet::Export { format, quality, scale, bg, level, ico, selection } => {
                    label(ui, "Format");
                    pro::segmented(ui, format, &["PNG", "JPEG", "WebP", "BMP", "TIFF", "GIF", "ICO"]);
                    ui.add_space(6.0);
                    match *format {
                        0 => {
                            ui.horizontal(|ui| {
                                label(ui, "Compression");
                                pro::segmented(ui, level, &["Rapide", "Normale", "Maximale"]);
                            });
                        }
                        1 => {
                            label(ui, "Qualité");
                            ui.add(egui::Slider::new(quality, 10..=100));
                            ui.horizontal(|ui| {
                                for (n, q) in [("Basse", 50u8), ("Moyenne", 70), ("Haute", 85), ("Maximale", 100)] {
                                    if ui.selectable_label(*quality == q, n).clicked() {
                                        *quality = q;
                                    }
                                }
                            });
                        }
                        6 => {
                            label(ui, "Tailles de l'icône");
                            ui.horizontal_wrapped(|ui| {
                                for (i, s) in [16, 24, 32, 48, 64, 128, 256].iter().enumerate() {
                                    ui.checkbox(&mut ico[i], format!("{s}"));
                                }
                            });
                        }
                        _ => {}
                    }
                    if matches!(*format, 1 | 3) {
                        ui.horizontal(|ui| {
                            label(ui, "Fond des zones transparentes");
                            tools::combo(ui, "expbg", bg, &["Blanc", "Noir", "Couleur secondaire"], 150.0);
                        });
                    }
                    if *format != 6 {
                        label(ui, "Échelle");
                        ui.add(egui::Slider::new(scale, 10.0..=400.0).suffix(" %"));
                        ui.horizontal(|ui| {
                            for s in [25.0, 50.0, 100.0, 200.0] {
                                if ui.selectable_label(*scale == s, format!("{s:.0} %")).clicked() {
                                    *scale = s;
                                }
                            }
                        });
                    }
                    let (bw, bh) = match (*selection, self.doc.selection) {
                        (true, Some(s)) => (s.width(), s.height()),
                        _ => (dw, dh),
                    };
                    let (w, h) = ((bw as f32 * *scale / 100.0) as u32, (bh as f32 * *scale / 100.0) as u32);
                    let note = ["transparence conservée", "sans transparence, avec perte", "sans perte, transparence conservée", "sans compression", "sans perte, transparence conservée", "256 couleurs", "icône Windows multi-tailles"][*format];
                    ui.label(RichText::new(format!("{w} × {h} px · {note}")).size(12.0).color(FAINT));
                }
                Sheet::RotateCanvas { angle } => {
                    ui.add(egui::Slider::new(angle, -180.0..=180.0).suffix("°"));
                    ui.label(RichText::new("La zone de travail s'agrandit pour contenir l'image tournée ; le fond prend la couleur secondaire.").size(12.0).color(FAINT));
                }
                Sheet::Guide { vertical, pos } => {
                    let mut v = *vertical as usize;
                    if pro::segmented(ui, &mut v, &["Horizontal", "Vertical"]) {
                        *vertical = v == 1;
                    }
                    ui.add(egui::DragValue::new(pos).range(0.0..=16384.0).suffix(" px").prefix("Position "));
                }
                Sheet::Note { text, .. } => {
                    let r = ui.add(egui::TextEdit::multiline(text).hint_text("Note pour plus tard (non exportée)").desired_rows(4).desired_width(f32::INFINITY));
                    if text.is_empty() {
                        r.request_focus();
                    }
                }
                Sheet::SaveSelection { name } => {
                    ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
                }
                Sheet::Prefs => {
                    label(ui, "États d'historique");
                    ui.add(egui::Slider::new(&mut self.doc.undo_limit, 5..=200));
                    ui.horizontal(|ui| {
                        label(ui, "Plan de travail");
                        tools::combo(ui, "pboard", &mut self.pasteboard, &PASTEBOARDS.map(|p| p.0), 140.0);
                    });
                    ui.horizontal(|ui| {
                        label(ui, "Curseur de peinture");
                        tools::combo(ui, "cmode", &mut self.cursor_mode, &["Forme", "Forme et réticule", "Précis"], 160.0);
                    });
                    ui.checkbox(&mut self.pixel_grid, "Grille de pixels aux forts zooms");
                    ui.checkbox(&mut self.snap, "Magnétisme");
                }
                Sheet::Shortcuts => {
                    egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                        egui::Grid::new("sc").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                            for (k, d) in SHORTCUTS {
                                ui.label(RichText::new(*k).monospace().size(12.0).color(INK_STRONG));
                                ui.label(RichText::new(*d).size(12.5).color(MUTED));
                                ui.end_row();
                            }
                        });
                    });
                }
            }
            let ok = match &sheet {
                Sheet::Export { .. } => "Exporter…",
                Sheet::Prefs | Sheet::Shortcuts => "Fermer",
                Sheet::Note { .. } | Sheet::SaveSelection { .. } => "Enregistrer",
                Sheet::Fill { .. } => "Remplir",
                _ => "Appliquer",
            };
            buttons(ui, ok, &mut apply, &mut cancel);
        });
        if apply {
            self.apply_sheet(sheet, ctx);
        } else if keep && !cancel {
            self.sheet = Some(sheet);
        }
    }

    fn apply_sheet(&mut self, sheet: Sheet, ctx: &egui::Context) {
        let label = |s: &str| Some(s.to_string());
        match sheet {
            Sheet::Canvas { w, h, anchor, ext, .. } => {
                let fill = match ext {
                    0 => Some(self.secondary.to_array()),
                    1 => Some(self.primary.to_array()),
                    2 => Some([255, 255, 255, 255]),
                    3 => Some([0, 0, 0, 255]),
                    _ => None,
                };
                self.doc.next_label = label("Taille de la zone de travail");
                self.doc.canvas_size(w, h, ((anchor % 3) as f32 / 2.0, (anchor / 3) as f32 / 2.0), fill);
                self.fit_pending = true;
            }
            Sheet::ImageSize { w, h, resample, .. } => {
                use imageops::FilterType as F;
                let f = [F::Nearest, F::Triangle, F::CatmullRom, F::Lanczos3, F::Gaussian][resample.min(4)];
                self.doc.set_selection(None, None);
                self.doc.next_label = label("Taille de l'image");
                self.doc.transform(|img| imageops::resize(img, w, h, f));
                self.fit_pending = true;
            }
            Sheet::Transform(t) => {
                self.doc.next_label = label("Transformation");
                self.doc.transform_layer_full(&t);
            }
            Sheet::Effect { kind, size, dist, angle, color, color2, position, blend, depth } => {
                let (dx, dy) = ((angle.to_radians().cos() * -dist) as i64, (angle.to_radians().sin() * dist) as i64);
                let b = BLENDS[blend].0;
                self.doc.next_label = label(["Ombre portée", "Lueur externe", "Contour", "Contour de la sélection", "Ombre interne", "Lueur interne", "Biseau", "Incrustation couleur", "Incrustation dégradé"][kind as usize % 9]);
                match kind {
                    0 => self.doc.drop_shadow(dx, dy, size, color),
                    1 => self.doc.outer_glow(size, color),
                    2 => self.doc.layer_outline_at(size.max(1.0) as u32, color, position),
                    3 => self.doc.stroke_selection(color, size.max(1.0) as u32, position),
                    4 => self.doc.inner_shadow(dx, dy, size, color),
                    5 => self.doc.inner_glow(size, color),
                    6 => self.doc.bevel(size, depth, angle, color, color2),
                    7 => self.doc.color_overlay(color, b),
                    _ => self.doc.gradient_overlay(color, color2, angle, b),
                }
            }
            Sheet::SelectBy { kind, px } => match kind {
                0 => self.doc.grow_selection(px.round() as i32),
                1 => self.doc.grow_selection(-(px.round() as i32)),
                2 => self.doc.border_selection(px.round() as u32),
                3 => self.doc.feather_selection(px),
                _ => self.doc.smooth_selection(px),
            },
            Sheet::ColorRange { mode, fuzz } => {
                let c = self.primary;
                self.doc.select_color_range([c.r(), c.g(), c.b()], fuzz, mode as u8);
            }
            Sheet::Fill { content, opacity, blend } => {
                if content == 5 {
                    self.content_fill_selection(ctx);
                } else {
                    let c = match content {
                        0 => self.primary.to_array(),
                        1 => self.secondary.to_array(),
                        2 => [0, 0, 0, 255],
                        3 => [255, 255, 255, 255],
                        _ => [128, 128, 128, 255],
                    };
                    self.doc.next_label = label("Remplir");
                    self.doc.fill_selection_ex(c, opacity / 100.0, BLENDS[blend].0);
                }
            }
            Sheet::Export { format, quality, scale, bg, level, ico, selection } => self.export_with(format, quality, scale, bg, level, ico, selection),
            Sheet::RotateCanvas { angle } => {
                self.doc.next_label = label("Rotation de l'image");
                self.doc.rotate_canvas(angle, Some(self.secondary.to_array()));
                self.fit_pending = true;
            }
            Sheet::Guide { vertical, pos } => {
                self.guides.push((vertical, pos));
                self.show_guides = true;
            }
            Sheet::Note { idx, at, text } => match (idx, text.trim().is_empty()) {
                (Some(i), true) => {
                    self.notes.remove(i);
                }
                (Some(i), false) => self.notes[i].1 = text,
                (None, false) => self.notes.push((at, text)),
                _ => {}
            },
            Sheet::SaveSelection { name } => {
                if self.doc.selection.is_some() {
                    self.saved_sel.push((name, self.doc.sel_mask()));
                }
            }
            Sheet::Prefs | Sheet::Shortcuts => {}
        }
    }

    fn new_doc_sheet_ui(&mut self, ctx: &egui::Context) {
        let Some(mut nd) = self.new_doc.take() else { return };
        let mut create = false;
        let keep = pro::sheet(ctx, "new", 760.0, |ui| {
            ui.label(RichText::new("Nouveau document").font(semibold(20.0)).color(INK_STRONG));
            ui.add_space(14.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(450.0);
                    ui.label(RichText::new("Préréglages").font(semibold(13.0)).color(MUTED));
                    ui.add_space(6.0);
                    egui::ScrollArea::vertical().max_height(430.0).show(ui, |ui| {
                        for (ri, row) in PRESETS.chunks(3).enumerate() {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;
                                for (k, (name, kind, w, h)) in row.iter().enumerate() {
                                    let idx = ri * 3 + k;
                                    if preset_card(ui, name, &format!("{w} × {h} · {kind}"), *w, *h, nd.preset == idx, 142.0).clicked() {
                                        nd.preset = idx;
                                        nd.w = *w;
                                        nd.h = *h;
                                    }
                                }
                            });
                            ui.add_space(10.0);
                        }
                    });
                });
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Détails").font(semibold(13.0)).color(MUTED));
                    ui.add_space(6.0);
                    ui.add(egui::TextEdit::singleline(&mut nd.name).desired_width(f32::INFINITY));
                    ui.add_space(10.0);
                    label(ui, "Largeur");
                    ui.add(egui::DragValue::new(&mut nd.w).range(1..=16384).suffix(" px"));
                    label(ui, "Hauteur");
                    ui.add(egui::DragValue::new(&mut nd.h).range(1..=16384).suffix(" px"));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        label(ui, "Orientation");
                        if ui.selectable_label(nd.w >= nd.h, "Paysage").clicked() && nd.w < nd.h {
                            std::mem::swap(&mut nd.w, &mut nd.h);
                        }
                        if ui.selectable_label(nd.h > nd.w, "Portrait").clicked() && nd.w >= nd.h {
                            std::mem::swap(&mut nd.w, &mut nd.h);
                        }
                    });
                    ui.add_space(6.0);
                    label(ui, "Arrière-plan");
                    tools::combo(ui, "bg", &mut nd.background, &BACKGROUNDS, 200.0);
                });
            });
            ui.add_space(16.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if pro::primary(ui, "Créer").clicked() {
                    create = true;
                }
                if pro::secondary(ui, "Annuler").clicked() {
                    nd.preset = usize::MAX;
                }
            });
        });
        if create {
            let fill = match nd.background {
                0 => Some([255, 255, 255, 255]),
                1 => None,
                2 => Some([0, 0, 0, 255]),
                3 => Some(self.primary.to_array()),
                _ => Some(self.secondary.to_array()),
            };
            let mut doc = Document::new(nd.w, nd.h, fill);
            doc.name = if nd.name.trim().is_empty() { "Sans titre".into() } else { nd.name.trim().to_string() };
            self.set_doc(doc);
        } else if keep && nd.preset != usize::MAX {
            self.new_doc = Some(nd);
        }
    }

    fn adjust_sheet(&mut self, ctx: &egui::Context) {
        let Some(mut adj) = self.adjust.take() else { return };
        let before = (adj.vals.clone(), adj.opacity, adj.blend, adj.preview);
        let (mut apply, mut cancel, mut reset, mut save) = (false, false, false, false);
        let kind = adj.kind;
        let user: Vec<(usize, String, Vec<f32>)> = self.user_presets.iter().enumerate().filter(|p| p.1.0 == kind).map(|(i, p)| (i, p.1.clone(), p.2.clone())).collect();
        let mut delete = None;
        let keep = pro::sheet(ctx, "adjust", 400.0, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(kind.title()).font(semibold(18.0)).color(INK_STRONG));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new(RichText::new("Réinitialiser").color(BLUE)).frame(false)).clicked() {
                        reset = true;
                    }
                });
            });
            if !kind.presets().is_empty() || !user.is_empty() {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                    for (name, vals) in kind.presets() {
                        if ui.add(egui::Button::new(RichText::new(*name).size(11.5)).fill(FIELD).corner_radius(6)).clicked() {
                            adj.vals = vals.to_vec();
                        }
                    }
                    for (i, name, vals) in &user {
                        let r = ui.add(egui::Button::new(RichText::new(name).size(11.5).color(BLUE)).fill(FIELD).corner_radius(6));
                        if r.clicked() {
                            adj.vals = vals.clone();
                        }
                        r.context_menu(|ui| {
                            if ui.button("Supprimer le préréglage").clicked() {
                                delete = Some(*i);
                                ui.close();
                            }
                        });
                    }
                });
            }
            ui.add_space(10.0);
            egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                ui.spacing_mut().slider_width = 250.0;
                for (v, p) in adj.vals.iter_mut().zip(kind.params()) {
                    if p.is_toggle() {
                        let mut on = *v >= 0.5;
                        if ui.checkbox(&mut on, p.label).changed() {
                            *v = if on { 1.0 } else { 0.0 };
                        }
                    } else if !p.choices.is_empty() {
                        ui.horizontal(|ui| {
                            label(ui, p.label);
                            let mut k = v.round() as usize;
                            if tools::combo(ui, p.label, &mut k, p.choices, 190.0) {
                                *v = k as f32;
                            }
                        });
                    } else {
                        label(ui, p.label);
                        let decimals = if p.max - p.min <= 10.0 { 2 } else { 0 };
                        ui.add(egui::Slider::new(v, p.min..=p.max).suffix(p.suffix).fixed_decimals(decimals));
                    }
                    ui.add_space(2.0);
                }
                if kind == crate::filters::Kind::Curves && adj.vals.len() >= 6 {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(140.0, 140.0), Sense::hover());
                    let p = ui.painter();
                    p.rect(r, CornerRadius::same(4), Color32::from_rgb(0x14, 0x14, 0x17), Stroke::new(1.0, LINE), StrokeKind::Inside);
                    let lut = crate::filters::Kind::Curves.apply(&RgbaImage::from_fn(256, 1, |x, _| image::Rgba([x as u8, x as u8, x as u8, 255])), &[0.0, adj.vals[1], adj.vals[2], adj.vals[3], adj.vals[4], adj.vals[5]]);
                    let pts: Vec<egui::Pos2> = (0..256).step_by(4).map(|x| egui::pos2(r.left() + x as f32 / 255.0 * r.width(), r.bottom() - lut.get_pixel(x, 0).0[0] as f32 / 255.0 * r.height())).collect();
                    p.line_segment([r.left_bottom(), r.right_top()], Stroke::new(1.0, Color32::from_white_alpha(30)));
                    p.add(egui::Shape::line(pts, Stroke::new(1.6, Color32::WHITE)));
                }
            });
            if adj.layer.is_none() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    label(ui, "Estomper");
                    ui.spacing_mut().slider_width = 130.0;
                    let mut o = adj.opacity * 100.0;
                    if ui.add(egui::Slider::new(&mut o, 0.0..=100.0).suffix(" %")).changed() {
                        adj.opacity = o / 100.0;
                    }
                    let mut b = BLENDS[adj.blend].0;
                    if tools::blend_combo(ui, "adjb", &mut b, 110.0) {
                        adj.blend = blend_index(b);
                    }
                });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.checkbox(&mut adj.preview, "Aperçu");
                ui.add(egui::TextEdit::singleline(&mut adj.preset_name).hint_text("Nom du préréglage").desired_width(140.0));
                if ui.add_enabled(!adj.preset_name.trim().is_empty(), egui::Button::new("Enregistrer")).clicked() {
                    save = true;
                }
            });
            let scope = if adj.layer.is_some() {
                "Calque de réglage : modifiable à tout moment"
            } else if self.doc.selection.is_some() {
                "Aperçu en direct · limité à la sélection"
            } else {
                "Aperçu en direct sur le calque actif"
            };
            ui.label(RichText::new(scope).size(12.0).color(FAINT));
            buttons(ui, if adj.layer.is_some() { "OK" } else { "Appliquer" }, &mut apply, &mut cancel);
        });
        if let Some(i) = delete {
            self.user_presets.remove(i);
            self.save_presets();
        }
        if save {
            self.user_presets.push((kind, adj.preset_name.trim().to_string(), adj.vals.clone()));
            adj.preset_name.clear();
            self.save_presets();
        }
        if reset {
            adj.vals = kind.defaults();
            adj.opacity = 1.0;
            adj.blend = 0;
        }
        if let Some(li) = adj.layer {
            if li >= self.doc.layers.len() {
                return;
            }
            if cancel || !keep && !apply {
                self.doc.layers[li].adjust = Some((kind, adj.old.clone()));
                self.doc.touch_all();
            } else if apply {
                let vals = adj.vals.clone();
                self.doc.layers[li].adjust = Some((kind, adj.old.clone()));
                self.doc.next_label = Some(format!("Réglage {}", kind.title()));
                self.doc.checkpoint();
                self.doc.layers[li].adjust = Some((kind, vals));
                self.doc.touch_all();
            } else {
                if before.0 != adj.vals {
                    self.doc.layers[li].adjust = Some((kind, adj.vals.clone()));
                    self.doc.touch_all();
                }
                self.adjust = Some(adj);
            }
            return;
        }
        let result = |adj: &Adjust| {
            let out = kind.apply(&adj.base, &adj.vals);
            if adj.opacity < 1.0 || adj.blend != 0 { fade(&adj.base, &out, adj.opacity, BLENDS[adj.blend].0) } else { out }
        };
        if apply {
            let out = result(&adj);
            self.doc.set_active_target(adj.base.clone());
            self.doc.next_label = Some(kind.title().to_string());
            self.doc.checkpoint();
            self.doc.put_filtered(&adj.base, &out);
            self.last_filter = Some((kind, adj.vals.clone()));
        } else if cancel || !keep {
            self.doc.set_active_target(adj.base);
        } else {
            if adj.fresh || before != (adj.vals.clone(), adj.opacity, adj.blend, adj.preview) {
                if adj.preview {
                    let out = result(&adj);
                    self.doc.put_filtered(&adj.base, &out);
                } else {
                    self.doc.set_active_target(adj.base.clone());
                }
                adj.fresh = false;
            }
            self.adjust = Some(adj);
        }
    }

    fn text_sheet(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.text.take() else { return };
        let (mut add, mut cancel) = (false, false);
        let keep = pro::sheet(ctx, "text", 460.0, |ui| {
            ui.label(RichText::new(if draft.vertical { "Texte vertical" } else { "Texte" }).font(semibold(18.0)).color(INK_STRONG));
            ui.add_space(10.0);
            let r = ui.add(egui::TextEdit::multiline(&mut draft.text).hint_text("Ton texte").desired_rows(3).desired_width(f32::INFINITY));
            if draft.text.is_empty() {
                r.request_focus();
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                label(ui, "Police");
                tools::combo(ui, "tfont", &mut self.text_font, &shapes::FONTS.map(|f| f.0), 150.0);
                ui.toggle_value(&mut self.text_bold, RichText::new("G").strong());
                ui.toggle_value(&mut self.text_italic, RichText::new("I").italics());
                ui.toggle_value(&mut self.text_underline, RichText::new("S").underline());
            });
            label(ui, "Taille");
            ui.spacing_mut().slider_width = 280.0;
            ui.add(egui::Slider::new(&mut self.text_size, 6.0..=800.0).suffix(" px").logarithmic(true));
            ui.horizontal(|ui| {
                label(ui, "Approche");
                ui.add(egui::DragValue::new(&mut self.tracking).range(-50.0..=200.0).suffix(" ‰"));
                label(ui, "Interligne");
                ui.add(egui::DragValue::new(&mut self.leading).range(0.5..=3.0).speed(0.01).suffix(" ×"));
            });
            if !draft.vertical {
                pro::segmented(ui, &mut self.text_align, &["Gauche", "Centre", "Droite"]);
            }
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                ui.painter().rect(r, CornerRadius::same(4), self.primary, Stroke::new(1.0, LINE), StrokeKind::Inside);
                ui.label(RichText::new("Couleur principale · nouveau calque de texte").size(12.0).color(FAINT));
            });
            buttons(ui, "Ajouter", &mut add, &mut cancel);
        });
        if add && !draft.text.trim().is_empty() {
            let st = TextStyle { font: self.text_font, size: self.text_size, bold: self.text_bold, italic: self.text_italic, underline: self.text_underline, align: self.text_align, tracking: self.tracking, leading: self.leading, vertical: draft.vertical, color: self.primary.to_array() };
            match shapes::render_text(&draft.text, &st) {
                Some(glyphs) => {
                    let mut canvas = RgbaImage::new(self.doc.width(), self.doc.height());
                    let x = draft.at.0 as i64 - if draft.vertical { 0 } else { (glyphs.width() as f32 * self.text_align as f32 / 2.0) as i64 };
                    imageops::overlay(&mut canvas, &glyphs, x, draft.at.1 as i64 - (self.text_size * 0.2) as i64);
                    let name: String = draft.text.lines().next().unwrap_or("Texte").chars().take(24).collect();
                    self.doc.next_label = Some("Texte".into());
                    self.doc.add_layer(Some(name), Some(canvas));
                    self.use_color(self.primary);
                }
                None => self.error = Some("Police introuvable".into()),
            }
        } else if keep && !cancel && !add {
            self.text = Some(draft);
        }
    }
}

use crate::Adjust;
