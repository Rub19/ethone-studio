use crate::doc::{BLENDS, Blend, blend_name};
use crate::{Pixel, Sheet};
use eframe::egui::{self, Align, Align2, Color32, CornerRadius, Key, Layout, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use studio_ui::pro::{self, FIELD, INK, LINE, MUTED, PANEL, semibold};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Move,
    Marquee,
    EllipseMarquee,
    RowMarquee,
    ColMarquee,
    Lasso,
    PolyLasso,
    MagLasso,
    QuickSelect,
    Wand,
    Crop,
    Picker,
    Sampler,
    Ruler,
    Note,
    Remove,
    Heal,
    Patch,
    Clone,
    HistoryBrush,
    Brush,
    Pencil,
    ColorReplace,
    Mixer,
    Eraser,
    BgEraser,
    MagicEraser,
    Gradient,
    Fill,
    Blur,
    Sharpen,
    Smudge,
    Dodge,
    Burn,
    Sponge,
    TextH,
    TextV,
    Rect,
    Ellipse,
    Polygon,
    Line,
    CustomShape,
    Hand,
    RotateView,
    Zoom,
}

pub const TOOLS: &[(Tool, &str, Key, &str)] = &[
    (Tool::Move, "Déplacement", Key::V, "V"),
    (Tool::Marquee, "Rectangle de sélection", Key::M, "M"),
    (Tool::EllipseMarquee, "Ellipse de sélection", Key::M, "M"),
    (Tool::RowMarquee, "Sélection d'une ligne", Key::M, ""),
    (Tool::ColMarquee, "Sélection d'une colonne", Key::M, ""),
    (Tool::Lasso, "Lasso", Key::L, "L"),
    (Tool::PolyLasso, "Lasso polygonal", Key::L, "L"),
    (Tool::MagLasso, "Lasso magnétique", Key::L, "L"),
    (Tool::QuickSelect, "Sélection rapide", Key::W, "W"),
    (Tool::Wand, "Baguette magique", Key::W, "W"),
    (Tool::Crop, "Recadrage", Key::C, "C"),
    (Tool::Picker, "Pipette", Key::I, "I"),
    (Tool::Sampler, "Échantillonnage de couleur", Key::I, "I"),
    (Tool::Ruler, "Règle", Key::I, "I"),
    (Tool::Note, "Annotation", Key::I, "I"),
    (Tool::Remove, "Suppression intelligente", Key::J, "J"),
    (Tool::Heal, "Correcteur", Key::J, "J"),
    (Tool::Patch, "Pièce", Key::J, "J"),
    (Tool::Clone, "Tampon de duplication", Key::S, "S"),
    (Tool::HistoryBrush, "Forme d'historique", Key::Y, "Y"),
    (Tool::Brush, "Pinceau", Key::B, "B"),
    (Tool::Pencil, "Crayon", Key::B, "B"),
    (Tool::ColorReplace, "Remplacement de couleur", Key::B, "B"),
    (Tool::Mixer, "Pinceau mélangeur", Key::B, "B"),
    (Tool::Eraser, "Gomme", Key::E, "E"),
    (Tool::BgEraser, "Gomme d'arrière-plan", Key::E, "E"),
    (Tool::MagicEraser, "Gomme magique", Key::E, "E"),
    (Tool::Gradient, "Dégradé", Key::G, "G"),
    (Tool::Fill, "Pot de peinture", Key::G, "G"),
    (Tool::Blur, "Goutte (flou)", Key::R, "R"),
    (Tool::Sharpen, "Netteté", Key::R, "R"),
    (Tool::Smudge, "Doigt", Key::R, "R"),
    (Tool::Dodge, "Densité −", Key::O, "O"),
    (Tool::Burn, "Densité +", Key::O, "O"),
    (Tool::Sponge, "Éponge", Key::O, "O"),
    (Tool::TextH, "Texte horizontal", Key::T, "T"),
    (Tool::TextV, "Texte vertical", Key::T, "T"),
    (Tool::Rect, "Rectangle", Key::U, "U"),
    (Tool::Ellipse, "Ellipse", Key::U, "U"),
    (Tool::Polygon, "Polygone", Key::U, "U"),
    (Tool::Line, "Trait", Key::U, "U"),
    (Tool::CustomShape, "Forme personnalisée", Key::U, "U"),
    (Tool::Hand, "Main", Key::H, "H"),
    (Tool::RotateView, "Rotation de la vue", Key::H, "H"),
    (Tool::Zoom, "Zoom", Key::Z, "Z"),
];

pub const GROUPS: [&[Tool]; 17] = [
    &[Tool::Move],
    &[Tool::Marquee, Tool::EllipseMarquee, Tool::RowMarquee, Tool::ColMarquee],
    &[Tool::Lasso, Tool::PolyLasso, Tool::MagLasso],
    &[Tool::QuickSelect, Tool::Wand],
    &[Tool::Crop],
    &[Tool::Picker, Tool::Sampler, Tool::Ruler, Tool::Note],
    &[Tool::Remove, Tool::Heal, Tool::Patch, Tool::Clone],
    &[Tool::HistoryBrush],
    &[Tool::Brush, Tool::Pencil, Tool::ColorReplace, Tool::Mixer],
    &[Tool::Eraser, Tool::BgEraser, Tool::MagicEraser],
    &[Tool::Gradient, Tool::Fill],
    &[Tool::Blur, Tool::Sharpen, Tool::Smudge],
    &[Tool::Dodge, Tool::Burn, Tool::Sponge],
    &[Tool::TextH, Tool::TextV],
    &[Tool::Rect, Tool::Ellipse, Tool::Polygon, Tool::Line, Tool::CustomShape],
    &[Tool::Hand, Tool::RotateView],
    &[Tool::Zoom],
];

pub const CUSTOM_SHAPES: [&str; 9] = ["Cœur", "Étoile", "Flèche", "Bulle", "Coche", "Croix", "Éclair", "Goutte", "Lune"];
pub const BRUSH_PRESETS: [(&str, f32, f32, f32, f32, f32, f32, f32); 10] = [
    ("Rond dur", 1.0, 0.125, 0.0, 1.0, 0.0, 0.0, 0.0),
    ("Rond doux", 0.0, 0.125, 0.0, 1.0, 0.0, 0.0, 0.0),
    ("Aérographe", 0.0, 0.05, 0.0, 1.0, 0.0, 0.0, 0.0),
    ("Encre calligraphique", 0.9, 0.05, 45.0, 0.25, 0.0, 0.0, 0.0),
    ("Craie", 0.6, 0.3, 0.0, 0.8, 0.3, 0.5, 0.15),
    ("Pointilliste", 0.9, 0.8, 0.0, 1.0, 0.6, 0.3, 0.8),
    ("Marqueur", 0.85, 0.08, 0.0, 0.6, 0.0, 0.0, 0.0),
    ("Pinceau sec", 0.7, 0.45, 0.0, 1.0, 0.2, 0.6, 0.05),
    ("Feuillage", 0.5, 0.6, 0.0, 0.7, 0.7, 0.4, 1.0),
    ("Plat", 0.95, 0.06, 90.0, 0.15, 0.0, 0.0, 0.0),
];
pub const GRADIENT_PRESETS: [&str; 8] = ["Principale → secondaire", "Principale → transparent", "Noir → blanc", "Arc-en-ciel", "Cuivre", "Chrome", "Coucher de soleil", "Océan"];
pub const PICK_SIZES: [&str; 5] = ["Point", "Moyenne 3 × 3", "Moyenne 5 × 5", "Moyenne 11 × 11", "Moyenne 31 × 31"];

pub fn tool_name(t: Tool) -> &'static str {
    TOOLS.iter().find(|x| x.0 == t).map_or("", |x| x.1)
}

pub fn tool_key(t: Tool) -> Option<(Key, &'static str)> {
    TOOLS.iter().find(|x| x.0 == t).map(|x| (x.2, x.3))
}

pub fn group_of(t: Tool) -> usize {
    GROUPS.iter().position(|g| g.contains(&t)).unwrap_or(0)
}

pub fn brushy(t: Tool) -> bool {
    matches!(
        t,
        Tool::Brush | Tool::Pencil | Tool::ColorReplace | Tool::Mixer | Tool::HistoryBrush | Tool::Eraser | Tool::BgEraser | Tool::Clone | Tool::Heal | Tool::Blur | Tool::Sharpen | Tool::Smudge | Tool::Dodge | Tool::Burn | Tool::Sponge
    )
}

pub fn is_shape(t: Tool) -> bool {
    matches!(t, Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line | Tool::CustomShape)
}

pub fn is_marquee(t: Tool) -> bool {
    matches!(t, Tool::Marquee | Tool::EllipseMarquee | Tool::RowMarquee | Tool::ColMarquee)
}

pub fn gradient_stops(preset: usize, a: [u8; 4], b: [u8; 4]) -> Vec<[u8; 4]> {
    match preset {
        1 => vec![a, [a[0], a[1], a[2], 0]],
        2 => vec![[0, 0, 0, 255], [255, 255, 255, 255]],
        3 => vec![[255, 0, 0, 255], [255, 165, 0, 255], [255, 255, 0, 255], [0, 200, 0, 255], [0, 120, 255, 255], [110, 0, 220, 255]],
        4 => vec![[90, 40, 10, 255], [220, 140, 80, 255], [120, 60, 20, 255], [250, 200, 150, 255]],
        5 => vec![[40, 40, 45, 255], [240, 240, 245, 255], [90, 90, 100, 255], [220, 220, 230, 255]],
        6 => vec![[40, 10, 90, 255], [210, 50, 80, 255], [255, 170, 60, 255]],
        7 => vec![[0, 40, 90, 255], [0, 140, 200, 255], [120, 230, 230, 255]],
        _ => vec![a, b],
    }
}

pub fn combo(ui: &mut Ui, id: &str, value: &mut usize, items: &[&str], width: f32) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id).selected_text(*items.get(*value).unwrap_or(&"")).width(width).show_ui(ui, |ui| {
        for (k, it) in items.iter().enumerate() {
            if ui.selectable_label(*value == k, *it).clicked() {
                *value = k;
                changed = true;
            }
        }
    });
    changed
}

pub fn blend_combo(ui: &mut Ui, id: &str, value: &mut Blend, width: f32) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id).selected_text(blend_name(*value)).width(width).show_ui(ui, |ui| {
        for (k, (b, name)) in BLENDS.iter().enumerate() {
            if matches!(k, 2 | 7 | 12 | 19 | 23) {
                ui.separator();
            }
            if ui.selectable_label(*value == *b, *name).clicked() {
                *value = *b;
                changed = true;
            }
        }
    });
    changed
}

fn hint(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(12.5).color(MUTED));
}

fn small(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(12.5).color(MUTED));
}

impl Pixel {
    pub fn select_tool(&mut self, t: Tool) {
        if self.tool != t {
            self.poly.clear();
        }
        self.tool = t;
        self.picks[group_of(t)] = t;
    }

    pub fn key_tool(&mut self, key: Key, shift: bool) {
        let Some(g) = GROUPS.iter().position(|g| g.iter().any(|t| tool_key(*t).is_some_and(|k| k.0 == key))) else { return };
        let with_key: Vec<Tool> = GROUPS[g].iter().copied().filter(|t| tool_key(*t).is_some_and(|k| k.0 == key)).collect();
        let pick = if shift && with_key.contains(&self.tool) {
            with_key[(with_key.iter().position(|t| *t == self.tool).unwrap_or(0) + 1) % with_key.len()]
        } else if with_key.contains(&self.tool) {
            self.tool
        } else if with_key.contains(&self.picks[g]) {
            self.picks[g]
        } else {
            with_key[0]
        };
        self.select_tool(pick);
    }

    pub fn tool_rail(&mut self, root: &mut Ui) {
        egui::Panel::left("tools").resizable(false).exact_size(54.0).frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(7, 8))).show(root, |ui| {
            egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (gi, group) in GROUPS.iter().enumerate() {
                    if matches!(gi, 5 | 10 | 13 | 15) {
                        let (sep, _) = ui.allocate_exact_size(vec2(40.0, 7.0), Sense::hover());
                        ui.painter().line_segment([sep.left_center() + vec2(7.0, 0.0), sep.right_center() - vec2(7.0, 0.0)], Stroke::new(1.0, LINE));
                    }
                    let items: Vec<(Tool, &str, &str)> = group.iter().map(|t| (*t, tool_name(*t), tool_key(*t).map_or("", |k| k.1))).collect();
                    let mut cur = self.tool;
                    if pro::tool_group(ui, 38.0, &mut cur, &items, |p, r, c, t| tool_icon(p, r, t, c)) {
                        self.select_tool(cur);
                    }
                }
                ui.add_space(10.0);
                let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 46.0), Sense::click());
                let back = Rect::from_min_size(rect.min + vec2(15.0, 17.0), Vec2::splat(20.0));
                let front = Rect::from_min_size(rect.min + vec2(4.0, 6.0), Vec2::splat(20.0));
                ui.painter().rect(back, CornerRadius::same(5), self.secondary, Stroke::new(1.0, MUTED), StrokeKind::Outside);
                ui.painter().rect(front, CornerRadius::same(5), self.primary, Stroke::new(1.5, INK), StrokeKind::Outside);
                if resp.on_hover_text("Couleurs principale et secondaire\nClic : inverser (X) · D : noir et blanc").clicked() {
                    std::mem::swap(&mut self.primary, &mut self.secondary);
                }
                ui.add_space(4.0);
                let edit = self.doc.edit_mask && self.doc.layers[self.doc.active].mask.is_some();
                let resp = pro::icon_button(ui, 38.0, edit, "Modifier le masque du calque (\\)", |p, r, c| {
                    p.rect_stroke(r, CornerRadius::same(3), Stroke::new(1.6, c), StrokeKind::Middle);
                    p.circle_filled(r.center(), r.width() * 0.28, c);
                });
                if resp.clicked() {
                    self.toggle_mask_edit();
                }
            });
        });
    }

    pub fn toggle_mask_edit(&mut self) {
        if self.doc.layers[self.doc.active].mask.is_none() {
            self.doc.next_label = Some("Masque de fusion".into());
            self.doc.add_mask(0);
        } else {
            self.doc.edit_mask = !self.doc.edit_mask;
        }
    }

    fn sel_options(&mut self, ui: &mut Ui) {
        let mut m = self.sel_mode as usize;
        if pro::segmented(ui, &mut m, &["Nouvelle", "Ajouter", "Soustraire", "Intersection"]) {
            self.sel_mode = m as u8;
        }
        pro::toolbar_sep(ui);
    }

    pub fn options_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("options").exact_size(44.0).frame(egui::Frame::new().fill(pro::CHROME).inner_margin(egui::Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().slider_width = 90.0;
                let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(7), FIELD);
                tool_icon(ui.painter(), r.shrink(7.0), self.tool, INK);
                ui.label(RichText::new(tool_name(self.tool)).font(semibold(13.0)).color(INK));
                pro::toolbar_sep(ui);
                let right = 120.0;
                let avail = ui.available_width() - right;
                ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_size(ui.cursor().min, vec2(avail.max(100.0), 44.0))), |ui| {
                    ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                    ui.horizontal_centered(|ui| self.tool_options(ui));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::secondary(ui, "?  Tutoriel").on_hover_text("F1").clicked() {
                        self.help.learn(tutorial_for(self.tool));
                    }
                });
            });
        });
    }

    fn tool_options(&mut self, ui: &mut Ui) {
        let t = self.tool;
        match t {
            t if brushy(t) => {
                pro::option_slider(ui, "Taille", &mut self.brush.radius, 0.5..=500.0, " px", true);
                if t != Tool::Pencil {
                    pro::option_slider(ui, "Dureté", &mut self.brush.hardness, 0.0..=1.0, "", false);
                }
                let flow_label = if matches!(t, Tool::Dodge | Tool::Burn) { "Exposition" } else if matches!(t, Tool::Blur | Tool::Sharpen | Tool::Smudge) { "Intensité" } else { "Flux" };
                pro::option_slider(ui, flow_label, &mut self.brush.flow, 0.01..=1.0, "", false);
                match t {
                    Tool::Brush | Tool::Pencil => {
                        small(ui, "Mode");
                        blend_combo(ui, "brush-mode", &mut self.brush.mode, 140.0);
                        pro::option_slider(ui, "Lissage", &mut self.smoothing, 0.0..=0.95, "", false);
                        ui.checkbox(&mut self.symmetry, "Symétrie").on_hover_text("Répète chaque trait en miroir vertical");
                        ui.checkbox(&mut self.symmetry_v, "Miroir H").on_hover_text("Répète chaque trait en miroir horizontal");
                    }
                    Tool::Mixer => {
                        pro::option_slider(ui, "Humidité", &mut self.mixer_wet, 0.0..=1.0, "", false);
                        pro::option_slider(ui, "Lissage", &mut self.smoothing, 0.0..=0.95, "", false);
                    }
                    Tool::ColorReplace => hint(ui, "Repeint la teinte avec la couleur principale en gardant la lumière"),
                    Tool::Eraser => {
                        ui.checkbox(&mut self.erase_history, "Effacer vers l'historique").on_hover_text("Restaure l'état de l'instantané au lieu de rendre transparent");
                        ui.checkbox(&mut self.symmetry, "Symétrie");
                    }
                    Tool::BgEraser => {
                        pro::option_slider(ui, "Tolérance", &mut self.bg_tol, 0.01..=1.0, "", false);
                        ui.checkbox(&mut self.bg_continuous, "Échantillonnage continu");
                    }
                    Tool::HistoryBrush => {
                        if pro::secondary(ui, "Instantané").on_hover_text("Mémorise le calque actif comme source").clicked() {
                            self.snapshot_history();
                        }
                        hint(ui, if self.history_src.is_some() { "Peins pour revenir à l'instantané" } else { "Aucun instantané" });
                    }
                    Tool::Sponge => {
                        let mut k = self.sponge_saturate as usize;
                        if pro::segmented(ui, &mut k, &["Désaturer", "Saturer"]) {
                            self.sponge_saturate = k == 1;
                        }
                    }
                    Tool::Dodge | Tool::Burn => {
                        small(ui, "Plage");
                        let mut r = self.dodge_range as usize;
                        if pro::segmented(ui, &mut r, &["Tons foncés", "Tons moyens", "Tons clairs"]) {
                            self.dodge_range = r as u8;
                        }
                    }
                    Tool::Smudge => {
                        ui.checkbox(&mut self.finger_paint, "Peinture au doigt").on_hover_text("Commence chaque trait avec la couleur principale");
                    }
                    Tool::Clone | Tool::Heal => {
                        ui.checkbox(&mut self.clone_aligned, "Aligné");
                        ui.checkbox(&mut self.sample_all, "Tous les calques");
                        hint(ui, if self.clone_src.is_some() { "Alt+clic pour changer de source" } else { "Alt+clic sur la zone source" });
                    }
                    _ => {}
                }
            }
            Tool::QuickSelect => {
                self.sel_options(ui);
                pro::option_slider(ui, "Taille", &mut self.brush.radius, 1.0..=300.0, " px", true);
                pro::option_slider(ui, "Tolérance", &mut self.tolerance, 0..=255, "", false);
                hint(ui, "Alt pour retirer");
            }
            Tool::Wand => {
                self.sel_options(ui);
                pro::option_slider(ui, "Tolérance", &mut self.tolerance, 0..=255, "", false);
                ui.checkbox(&mut self.contiguous, "Contiguë");
                ui.checkbox(&mut self.sample_all, "Tous les calques");
            }
            Tool::Marquee | Tool::EllipseMarquee | Tool::RowMarquee | Tool::ColMarquee => {
                self.sel_options(ui);
                pro::option_slider(ui, "Contour progressif", &mut self.feather, 0.0..=100.0, " px", false);
                if matches!(t, Tool::Marquee | Tool::EllipseMarquee) {
                    combo(ui, "mstyle", &mut self.marquee_style, &["Normal", "Proportions fixes", "Taille fixe"], 130.0);
                    match self.marquee_style {
                        1 => {
                            ui.add(egui::DragValue::new(&mut self.marquee_ratio.0).range(0.1..=100.0).prefix("L "));
                            ui.add(egui::DragValue::new(&mut self.marquee_ratio.1).range(0.1..=100.0).prefix("H "));
                        }
                        2 => {
                            ui.add(egui::DragValue::new(&mut self.marquee_size.0).range(1..=16384).prefix("L ").suffix(" px"));
                            ui.add(egui::DragValue::new(&mut self.marquee_size.1).range(1..=16384).prefix("H ").suffix(" px"));
                        }
                        _ => {}
                    }
                }
            }
            Tool::Lasso | Tool::PolyLasso | Tool::MagLasso => {
                self.sel_options(ui);
                pro::option_slider(ui, "Contour progressif", &mut self.feather, 0.0..=100.0, " px", false);
                if t == Tool::MagLasso {
                    pro::option_slider(ui, "Largeur", &mut self.mag_width, 2.0..=40.0, " px", false);
                }
                if t != Tool::Lasso {
                    hint(ui, if t == Tool::PolyLasso { "Clic par sommet · double-clic ou Entrée pour fermer" } else { "Glisse le long d'un bord" });
                }
            }
            Tool::Crop => {
                combo(ui, "cratio", &mut self.crop_ratio, &CROP_RATIOS.map(|r| r.0), 120.0);
                small(ui, "Grille");
                combo(ui, "cover", &mut self.crop_overlay, &["Aucune", "Règle des tiers", "Grille", "Diagonale", "Nombre d'or"], 130.0);
                if let Some(c) = self.crop {
                    ui.label(RichText::new(format!("{} × {} px", c.width(), c.height())).size(12.5).color(INK));
                    if pro::primary(ui, "Valider").clicked() {
                        self.commit_crop();
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        self.crop = None;
                    }
                } else {
                    hint(ui, "Glisse pour tracer le cadre à conserver");
                }
            }
            Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line | Tool::CustomShape => {
                if t != Tool::Line {
                    ui.checkbox(&mut self.shape_fill, "Fond");
                    ui.checkbox(&mut self.shape_stroke, "Contour").on_hover_text("Contour avec la couleur secondaire quand le fond est actif");
                }
                if self.shape_stroke || t == Tool::Line {
                    pro::option_slider(ui, "Épaisseur", &mut self.stroke_w, 1.0..=200.0, " px", true);
                }
                match t {
                    Tool::Rect => {
                        pro::option_slider(ui, "Arrondi", &mut self.corner, 0.0..=500.0, " px", false);
                    }
                    Tool::Polygon => {
                        pro::option_slider(ui, "Côtés", &mut self.sides, 3..=24, "", false);
                        ui.checkbox(&mut self.star, "Étoile");
                        if self.star {
                            pro::option_slider(ui, "Retrait", &mut self.inset, 0.05..=0.95, "", false);
                        }
                    }
                    Tool::Line => {
                        ui.checkbox(&mut self.arrow_start, "Flèche au début");
                        ui.checkbox(&mut self.arrow_end, "Flèche à la fin");
                    }
                    Tool::CustomShape => {
                        combo(ui, "custom", &mut self.custom, &CUSTOM_SHAPES, 110.0);
                    }
                    _ => {}
                }
                small(ui, "Mode");
                blend_combo(ui, "shape-mode", &mut self.shape_mode, 120.0);
                ui.checkbox(&mut self.shape_layer, "Nouveau calque");
            }
            Tool::TextH | Tool::TextV => {
                combo(ui, "font", &mut self.text_font, &crate::shapes::FONTS.map(|f| f.0), 130.0);
                ui.add(egui::DragValue::new(&mut self.text_size).range(4.0..=800.0).suffix(" px"));
                ui.toggle_value(&mut self.text_bold, RichText::new("G").strong()).on_hover_text("Gras");
                ui.toggle_value(&mut self.text_italic, RichText::new("I").italics()).on_hover_text("Italique");
                ui.toggle_value(&mut self.text_underline, RichText::new("S").underline()).on_hover_text("Souligné");
                pro::segmented(ui, &mut self.text_align, &["Gauche", "Centre", "Droite"]);
                small(ui, "Approche");
                ui.add(egui::DragValue::new(&mut self.tracking).range(-50.0..=200.0).suffix(" ‰"));
                small(ui, "Interligne");
                ui.add(egui::DragValue::new(&mut self.leading).range(0.5..=3.0).speed(0.01).suffix(" ×"));
            }
            Tool::Gradient => {
                combo(ui, "gpreset", &mut self.grad_preset, &GRADIENT_PRESETS, 170.0);
                pro::segmented(ui, &mut self.grad_kind, &["Linéaire", "Radial", "Angulaire", "Réfléchi", "Losange"]);
                small(ui, "Mode");
                blend_combo(ui, "grad-mode", &mut self.grad_mode, 120.0);
                pro::option_slider(ui, "Opacité", &mut self.grad_opacity, 0.01..=1.0, "", false);
                ui.checkbox(&mut self.grad_reverse, "Inverser");
                ui.checkbox(&mut self.grad_dither, "Tramage");
            }
            Tool::Fill => {
                pro::option_slider(ui, "Tolérance", &mut self.tolerance, 0..=255, "", false);
                pro::option_slider(ui, "Opacité", &mut self.fill_opacity, 0.01..=1.0, "", false);
                ui.checkbox(&mut self.contiguous, "Contigu");
                ui.checkbox(&mut self.sample_all, "Tous les calques");
            }
            Tool::MagicEraser => {
                pro::option_slider(ui, "Tolérance", &mut self.tolerance, 0..=255, "", false);
                pro::option_slider(ui, "Opacité", &mut self.fill_opacity, 0.01..=1.0, "", false);
                ui.checkbox(&mut self.contiguous, "Contigu");
            }
            Tool::Move => {
                ui.checkbox(&mut self.auto_select, "Sélection automatique").on_hover_text("Le clic choisit le calque visible sous le pointeur");
                ui.checkbox(&mut self.show_bounds, "Cadre de sélection");
                pro::toolbar_sep(ui);
                for (k, tip) in ["Aligner à gauche", "Centrer horizontalement", "Aligner à droite", "Aligner en haut", "Centrer verticalement", "Aligner en bas"].iter().enumerate() {
                    if pro::icon_button(ui, 28.0, false, &format!("{tip} (sur la sélection ou la zone de travail)"), |p, r, c| align_icon(p, r, k as u8, c)).clicked() {
                        let to = self.doc.selection.unwrap_or(self.doc.full());
                        self.doc.next_label = Some(tip.to_string());
                        self.doc.align_layer(k as u8, to);
                    }
                }
                hint(ui, "Flèches : 1 px · Maj : 10 px");
            }
            Tool::Hand | Tool::Zoom | Tool::RotateView => {
                if t == Tool::Zoom {
                    let mut k = self.zoom_out as usize;
                    if pro::segmented(ui, &mut k, &["Zoom avant", "Zoom arrière"]) {
                        self.zoom_out = k == 1;
                    }
                }
                if t == Tool::RotateView {
                    ui.add(egui::DragValue::new(&mut self.view_angle).range(-180.0..=180.0).suffix("°").speed(0.5));
                    if pro::secondary(ui, "Réinitialiser la vue").clicked() {
                        self.view_angle = 0.0;
                    }
                }
                if pro::secondary(ui, "Ajuster à l'écran").clicked() {
                    self.fit_pending = true;
                }
                if pro::secondary(ui, "100 %").clicked() {
                    self.zoom = 1.0;
                    self.offset = Vec2::ZERO;
                }
                if pro::secondary(ui, "200 %").clicked() {
                    self.zoom = 2.0;
                }
            }
            Tool::Picker | Tool::Sampler => {
                small(ui, "Échantillon");
                combo(ui, "psize", &mut self.picker_size, &PICK_SIZES, 130.0);
                ui.checkbox(&mut self.sample_all, "Tous les calques");
                if t == Tool::Sampler {
                    if !self.samplers.is_empty() && pro::secondary(ui, "Effacer").clicked() {
                        self.samplers.clear();
                    }
                    hint(ui, "Jusqu'à 4 points, lus dans Infos");
                } else {
                    hint(ui, "Alt+clic : couleur secondaire");
                }
            }
            Tool::Ruler => {
                if let Some((a, b)) = self.ruler {
                    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                    ui.label(RichText::new(format!("L {:.1} px   A {:.1}°   ΔX {:.0}   ΔY {:.0}", (dx * dx + dy * dy).sqrt(), -dy.atan2(dx).to_degrees(), dx, dy)).size(12.5).color(INK).monospace());
                    if pro::secondary(ui, "Redresser le calque").clicked() {
                        let mut ang = dy.atan2(dx).to_degrees();
                        if ang.abs() > 45.0 {
                            ang -= 90.0 * ang.signum();
                        }
                        self.doc.next_label = Some("Redresser".into());
                        self.doc.transform_layer(1.0, -ang);
                        self.ruler = None;
                    }
                    if pro::secondary(ui, "Effacer").clicked() {
                        self.ruler = None;
                    }
                } else {
                    hint(ui, "Glisse pour mesurer une distance et un angle");
                }
            }
            Tool::Note => {
                hint(ui, &format!("Clique pour poser une note · {} note(s)", self.notes.len()));
                if !self.notes.is_empty() && pro::secondary(ui, "Tout effacer").clicked() {
                    self.notes.clear();
                }
            }
            Tool::Remove => {
                pro::option_slider(ui, "Taille", &mut self.brush.radius, 2.0..=300.0, " px", true);
                hint(ui, "Peins sur ce qui doit disparaître : le fond est reconstruit autour");
            }
            Tool::Patch => {
                pro::option_slider(ui, "Diffusion", &mut self.patch_diffusion, 1.0..=20.0, "", false);
                hint(ui, if self.doc.selection.is_some() { "Fais glisser la sélection vers une zone saine" } else { "Entoure la zone à réparer" });
            }
            _ => {}
        }
    }
}

pub const CROP_RATIOS: [(&str, f32); 9] = [("Libre", 0.0), ("Original", -1.0), ("1:1 carré", 1.0), ("4:3", 4.0 / 3.0), ("3:2", 1.5), ("16:9", 16.0 / 9.0), ("9:16", 9.0 / 16.0), ("5:4", 1.25), ("2:3", 2.0 / 3.0)];

pub fn tutorial_for(t: Tool) -> &'static str {
    match t {
        Tool::Remove | Tool::Heal | Tool::Patch => "Suppression",
        Tool::Clone => "Tampon de duplication",
        t if is_marquee(t) => "Sélections",
        Tool::Lasso | Tool::PolyLasso | Tool::MagLasso => "Lassos",
        Tool::QuickSelect | Tool::Wand => "Sélection rapide et baguette",
        Tool::Brush | Tool::Pencil | Tool::ColorReplace | Tool::Mixer => "Pinceaux et dynamique",
        Tool::Eraser | Tool::BgEraser | Tool::MagicEraser => "Gommes",
        Tool::Gradient | Tool::Fill => "Dégradés",
        t if is_shape(t) => "Formes",
        Tool::TextH | Tool::TextV => "Texte",
        Tool::Picker | Tool::Sampler | Tool::Ruler | Tool::Note => "Pipette, règle et notes",
        Tool::Hand | Tool::RotateView | Tool::Zoom => "Navigation",
        Tool::Crop => "Recadrage",
        Tool::Move => "Déplacement et alignement",
        Tool::HistoryBrush => "Historique",
        _ => "Retouche locale",
    }
}

fn align_icon(p: &Painter, r: Rect, k: u8, c: Color32) {
    let s = Stroke::new(1.6, c);
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    if k < 3 {
        let x = [0.05, 0.5, 0.95][k as usize];
        p.line_segment([at(x, 0.0), at(x, 1.0)], s);
        let bars = [(0.2, 0.7), (0.6, 0.45)];
        for (y, w) in bars {
            let x0 = match k {
                0 => 0.12,
                1 => 0.5 - w / 2.0,
                _ => 0.88 - w,
            };
            p.rect_filled(Rect::from_min_max(at(x0, y), at(x0 + w, y + 0.2)), CornerRadius::same(1), c);
        }
    } else {
        let y = [0.05, 0.5, 0.95][(k - 3) as usize];
        p.line_segment([at(0.0, y), at(1.0, y)], s);
        for (x, h) in [(0.2, 0.7), (0.6, 0.45)] {
            let y0 = match k {
                3 => 0.12,
                4 => 0.5 - h / 2.0,
                _ => 0.88 - h,
            };
            p.rect_filled(Rect::from_min_max(at(x, y0), at(x + 0.2, y0 + h)), CornerRadius::same(1), c);
        }
    }
}

fn ring(c: Pos2, rx: f32, ry: f32, n: usize) -> Vec<Pos2> {
    (0..=n).map(|k| {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        c + vec2(a.cos() * rx, a.sin() * ry)
    }).collect()
}

pub fn tool_icon(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    let s = Stroke::new(1.8, c);
    let wd = r.width();
    let brush = |p: &Painter| {
        p.line_segment([at(0.95, 0.05), at(0.42, 0.58)], s);
        p.circle_filled(at(0.28, 0.72), wd * 0.2, c);
    };
    let eraser = |p: &Painter| {
        p.add(Shape::convex_polygon(vec![at(0.55, 0.1), at(0.95, 0.5), at(0.5, 0.95), at(0.1, 0.55)], Color32::TRANSPARENT, s));
        p.line_segment([at(0.3, 0.35), at(0.7, 0.75)], s);
    };
    let picker = |p: &Painter| {
        p.line_segment([at(0.08, 0.92), at(0.62, 0.38)], Stroke::new(2.3, c));
        p.circle_filled(at(0.75, 0.25), wd * 0.2, c);
    };
    let lasso = |p: &Painter| {
        let pts: Vec<Pos2> = (0..=20).map(|k| {
            let a = k as f32 / 20.0 * std::f32::consts::TAU;
            at(0.5 + a.cos() * 0.45, 0.4 + a.sin() * 0.3)
        }).collect();
        p.add(Shape::line(pts, s));
        p.line_segment([at(0.3, 0.68), at(0.22, 1.0)], s);
    };
    match tool {
        Tool::Move => {
            let m = r.center();
            let d = wd * 0.5;
            let k = wd * 0.18;
            p.line_segment([m - vec2(d, 0.0), m + vec2(d, 0.0)], s);
            p.line_segment([m - vec2(0.0, d), m + vec2(0.0, d)], s);
            for u in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                let tip = m + u * d;
                let back = tip - u * k;
                let perp = vec2(-u.y, u.x) * k;
                p.add(Shape::convex_polygon(vec![tip, back + perp, back - perp], c, Stroke::NONE));
            }
        }
        Tool::Marquee => {
            let rr = Rect::from_min_max(at(0.05, 0.15), at(0.95, 0.85));
            p.extend(Shape::dashed_line(&[rr.left_top(), rr.right_top(), rr.right_bottom(), rr.left_bottom(), rr.left_top()], s, 3.0, 2.5));
        }
        Tool::EllipseMarquee => p.extend(Shape::dashed_line(&ring(r.center(), wd * 0.48, wd * 0.35, 24), s, 3.0, 2.5)),
        Tool::RowMarquee => {
            p.extend(Shape::dashed_line(&[at(0.0, 0.5), at(1.0, 0.5)], Stroke::new(2.2, c), 3.0, 2.0));
            p.line_segment([at(0.0, 0.3), at(1.0, 0.3)], Stroke::new(1.0, c.gamma_multiply(0.4)));
            p.line_segment([at(0.0, 0.7), at(1.0, 0.7)], Stroke::new(1.0, c.gamma_multiply(0.4)));
        }
        Tool::ColMarquee => {
            p.extend(Shape::dashed_line(&[at(0.5, 0.0), at(0.5, 1.0)], Stroke::new(2.2, c), 3.0, 2.0));
            p.line_segment([at(0.3, 0.0), at(0.3, 1.0)], Stroke::new(1.0, c.gamma_multiply(0.4)));
            p.line_segment([at(0.7, 0.0), at(0.7, 1.0)], Stroke::new(1.0, c.gamma_multiply(0.4)));
        }
        Tool::Lasso => lasso(p),
        Tool::PolyLasso => {
            p.add(Shape::closed_line(vec![at(0.1, 0.3), at(0.55, 0.05), at(0.95, 0.35), at(0.7, 0.75), at(0.25, 0.65)], s));
            p.line_segment([at(0.25, 0.65), at(0.18, 1.0)], s);
        }
        Tool::MagLasso => {
            lasso(p);
            p.add(Shape::line(vec![at(0.6, 0.62), at(0.6, 0.9), at(0.95, 0.9), at(0.95, 0.62)], Stroke::new(2.2, c)));
        }
        Tool::QuickSelect => {
            brush(p);
            p.extend(Shape::dashed_line(&ring(at(0.28, 0.72), wd * 0.32, wd * 0.32, 16), Stroke::new(1.0, c), 2.0, 2.0));
        }
        Tool::Wand => {
            p.line_segment([at(0.05, 0.95), at(0.6, 0.4)], Stroke::new(2.2, c));
            let k = at(0.75, 0.25);
            for (dx, dy) in [(1.0, 0.0), (0.0, 1.0), (0.7, 0.7), (0.7, -0.7)] {
                let v = vec2(dx, dy) * wd * 0.2;
                p.line_segment([k - v, k + v], Stroke::new(1.4, c));
            }
        }
        Tool::Crop => {
            p.line_segment([at(0.25, 0.0), at(0.25, 0.75)], s);
            p.line_segment([at(0.25, 0.75), at(1.0, 0.75)], s);
            p.line_segment([at(0.0, 0.25), at(0.75, 0.25)], s);
            p.line_segment([at(0.75, 0.25), at(0.75, 1.0)], s);
        }
        Tool::Picker => picker(p),
        Tool::Sampler => {
            picker(p);
            p.circle_stroke(at(0.25, 0.75), wd * 0.18, Stroke::new(1.2, c));
            p.line_segment([at(0.05, 0.75), at(0.45, 0.75)], Stroke::new(1.0, c));
        }
        Tool::Ruler => {
            p.add(Shape::convex_polygon(vec![at(0.0, 0.7), at(0.7, 0.0), at(1.0, 0.3), at(0.3, 1.0)], Color32::TRANSPARENT, s));
            for k in 1..5 {
                let t = k as f32 / 5.0;
                let a = at(0.0 + 0.7 * t, 0.7 - 0.7 * t);
                p.line_segment([a, a + vec2(1.0, 1.0) * wd * if k % 2 == 0 { 0.2 } else { 0.12 }], Stroke::new(1.2, c));
            }
        }
        Tool::Note => {
            p.add(Shape::convex_polygon(vec![at(0.1, 0.1), at(0.9, 0.1), at(0.9, 0.65), at(0.6, 0.95), at(0.1, 0.95)], Color32::TRANSPARENT, s));
            p.line_segment([at(0.25, 0.35), at(0.75, 0.35)], Stroke::new(1.3, c));
            p.line_segment([at(0.25, 0.55), at(0.6, 0.55)], Stroke::new(1.3, c));
        }
        Tool::Remove => {
            p.line_segment([at(0.12, 0.88), at(0.6, 0.4)], Stroke::new(4.5, c));
            p.line_segment([at(0.12, 0.88), at(0.6, 0.4)], Stroke::new(1.8, PANEL));
            let star = at(0.78, 0.22);
            let k = wd * 0.2;
            p.line_segment([star - vec2(k, 0.0), star + vec2(k, 0.0)], Stroke::new(1.6, c));
            p.line_segment([star - vec2(0.0, k), star + vec2(0.0, k)], Stroke::new(1.6, c));
            p.circle_filled(at(0.9, 0.55), wd * 0.07, c);
        }
        Tool::Heal => {
            let pts = |a: Pos2, b: Pos2| {
                let d = (b - a).normalized();
                let n = vec2(-d.y, d.x) * wd * 0.2;
                vec![a + n, b + n, b - n, a - n]
            };
            p.add(Shape::convex_polygon(pts(at(0.12, 0.88), at(0.88, 0.12)), Color32::TRANSPARENT, s));
            p.rect_filled(Rect::from_center_size(r.center(), Vec2::splat(wd * 0.22)), CornerRadius::same(1), c);
        }
        Tool::Patch => {
            p.add(Shape::closed_line(vec![at(0.1, 0.2), at(0.55, 0.05), at(0.9, 0.3), at(0.8, 0.8), at(0.35, 0.95), at(0.05, 0.6)], s));
            for (x, y) in [(0.35, 0.4), (0.6, 0.55), (0.45, 0.7)] {
                p.circle_filled(at(x, y), wd * 0.05, c);
            }
        }
        Tool::Clone => {
            p.rect_filled(Rect::from_min_max(at(0.15, 0.62), at(0.85, 0.8)), CornerRadius::same(2), c);
            p.line_segment([at(0.15, 0.95), at(0.85, 0.95)], s);
            p.line_segment([at(0.5, 0.62), at(0.5, 0.35)], Stroke::new(2.4, c));
            p.circle_filled(at(0.5, 0.2), wd * 0.16, c);
        }
        Tool::HistoryBrush => {
            brush(p);
            p.add(Shape::line(ring(at(0.72, 0.72), wd * 0.2, wd * 0.2, 12)[3..].to_vec(), Stroke::new(1.3, c)));
        }
        Tool::Brush => brush(p),
        Tool::Pencil => {
            p.line_segment([at(0.85, 0.15), at(0.25, 0.75)], Stroke::new(4.0, c));
            p.add(Shape::convex_polygon(vec![at(0.18, 0.68), at(0.32, 0.82), at(0.05, 0.95)], c, Stroke::NONE));
        }
        Tool::ColorReplace => {
            brush(p);
            p.line_segment([at(0.55, 0.85), at(0.95, 0.85)], Stroke::new(1.3, c));
            p.add(Shape::convex_polygon(vec![at(0.95, 0.85), at(0.82, 0.76), at(0.82, 0.94)], c, Stroke::NONE));
        }
        Tool::Mixer => {
            brush(p);
            p.add(Shape::convex_polygon(vec![at(0.78, 0.5), at(0.92, 0.75), at(0.78, 0.95), at(0.64, 0.75)], c, Stroke::NONE));
        }
        Tool::Eraser => eraser(p),
        Tool::BgEraser => {
            eraser(p);
            p.line_segment([at(0.0, 0.0), at(0.3, 0.0)], Stroke::new(1.2, c));
            p.line_segment([at(0.0, 0.0), at(0.0, 0.3)], Stroke::new(1.2, c));
        }
        Tool::MagicEraser => {
            eraser(p);
            let k = at(0.15, 0.15);
            for (dx, dy) in [(1.0, 0.0), (0.0, 1.0)] {
                let v = vec2(dx, dy) * wd * 0.14;
                p.line_segment([k - v, k + v], Stroke::new(1.3, c));
            }
        }
        Tool::Fill => {
            p.add(Shape::convex_polygon(vec![at(0.1, 0.35), at(0.7, 0.35), at(0.6, 0.95), at(0.2, 0.95)], Color32::TRANSPARENT, s));
            p.line_segment([at(0.1, 0.35), at(0.4, 0.05)], s);
            p.circle_filled(at(0.88, 0.7), wd * 0.11, c);
        }
        Tool::Gradient => {
            let rr = Rect::from_min_max(at(0.0, 0.15), at(1.0, 0.85));
            let n = 6;
            for k in 0..n {
                let x0 = rr.left() + rr.width() * k as f32 / n as f32;
                let cell = Rect::from_min_max(pos2(x0, rr.top()), pos2(x0 + rr.width() / n as f32, rr.bottom()));
                p.rect_filled(cell, CornerRadius::ZERO, c.gamma_multiply(1.0 - k as f32 / n as f32));
            }
            p.rect_stroke(rr, CornerRadius::same(2), s, StrokeKind::Middle);
        }
        Tool::Blur => {
            p.add(Shape::convex_polygon(vec![at(0.5, 0.05), at(0.78, 0.55), at(0.5, 0.95), at(0.22, 0.55)], c, Stroke::NONE));
            p.circle_filled(at(0.5, 0.65), wd * 0.3, c);
        }
        Tool::Sharpen => {
            p.add(Shape::convex_polygon(vec![at(0.5, 0.02), at(0.68, 0.98), at(0.32, 0.98)], Color32::TRANSPARENT, s));
        }
        Tool::Smudge => {
            p.line_segment([at(0.3, 0.95), at(0.45, 0.35)], Stroke::new(3.2, c));
            p.circle_filled(at(0.48, 0.25), wd * 0.12, c);
            p.line_segment([at(0.6, 0.6), at(0.95, 0.45)], Stroke::new(1.4, c));
            p.line_segment([at(0.6, 0.78), at(0.95, 0.7)], Stroke::new(1.4, c));
        }
        Tool::Dodge => {
            p.circle_stroke(at(0.5, 0.35), wd * 0.3, s);
            p.line_segment([at(0.5, 0.65), at(0.5, 1.0)], Stroke::new(2.4, c));
        }
        Tool::Burn => {
            p.line_segment([at(0.3, 0.1), at(0.3, 0.5)], Stroke::new(2.4, c));
            p.circle_filled(at(0.55, 0.62), wd * 0.3, c);
        }
        Tool::Sponge => {
            p.rect_stroke(Rect::from_min_max(at(0.1, 0.25), at(0.9, 0.85)), CornerRadius::same(5), s, StrokeKind::Middle);
            for (x, y) in [(0.3, 0.45), (0.55, 0.6), (0.72, 0.42), (0.38, 0.7)] {
                p.circle_filled(at(x, y), wd * 0.06, c);
            }
        }
        Tool::TextH => {
            p.text(r.center(), Align2::CENTER_CENTER, "T", semibold(wd * 1.05), c);
        }
        Tool::TextV => {
            p.text(at(0.38, 0.5), Align2::CENTER_CENTER, "T", semibold(wd * 0.9), c);
            p.line_segment([at(0.85, 0.05), at(0.85, 0.95)], Stroke::new(1.3, c));
            p.add(Shape::convex_polygon(vec![at(0.85, 1.0), at(0.72, 0.8), at(0.98, 0.8)], c, Stroke::NONE));
        }
        Tool::Rect => {
            p.rect_filled(Rect::from_min_max(at(0.08, 0.2), at(0.92, 0.8)), CornerRadius::same(2), c);
        }
        Tool::Ellipse => {
            p.add(egui::epaint::EllipseShape { center: r.center(), radius: vec2(wd * 0.45, r.height() * 0.33), fill: c, stroke: Stroke::NONE, angle: 0.0 });
        }
        Tool::Polygon => {
            let pts: Vec<Pos2> = (0..6).map(|k| {
                let a = k as f32 / 6.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                r.center() + vec2(a.cos(), a.sin()) * wd * 0.48
            }).collect();
            p.add(Shape::convex_polygon(pts, c, Stroke::NONE));
        }
        Tool::Line => {
            p.line_segment([at(0.05, 0.95), at(0.95, 0.05)], Stroke::new(2.2, c));
        }
        Tool::CustomShape => {
            let pts: Vec<Pos2> = crate::shapes::custom(1, 0.0, 0.0, 1.0, 1.0).into_iter().map(|(x, y)| at(x, y)).collect();
            p.add(Shape::closed_line(pts, Stroke::new(1.5, c)));
        }
        Tool::Hand => {
            for (k, h) in [0.55, 0.75, 0.8, 0.7].iter().enumerate() {
                let x = 0.22 + k as f32 * 0.19;
                p.line_segment([at(x, 0.62), at(x, 0.62 - h * 0.6)], Stroke::new(2.4, c));
            }
            p.add(Shape::convex_polygon(vec![at(0.12, 0.6), at(0.85, 0.6), at(0.75, 1.0), at(0.25, 1.0)], c, Stroke::NONE));
        }
        Tool::RotateView => {
            p.add(Shape::line(ring(r.center(), wd * 0.42, wd * 0.42, 24)[2..20].to_vec(), s));
            let tip = ring(r.center(), wd * 0.42, wd * 0.42, 24)[2];
            p.add(Shape::convex_polygon(vec![tip + vec2(0.0, -4.0), tip + vec2(4.0, 2.0), tip + vec2(-4.0, 2.0)], c, Stroke::NONE));
            p.rect_stroke(Rect::from_center_size(r.center(), Vec2::splat(wd * 0.35)), CornerRadius::same(1), Stroke::new(1.3, c), StrokeKind::Middle);
        }
        Tool::Zoom => {
            p.circle_stroke(at(0.42, 0.42), wd * 0.32, s);
            p.line_segment([at(0.66, 0.66), at(0.98, 0.98)], Stroke::new(2.4, c));
        }
    }
}

impl Pixel {
    pub fn open_sheet_effect(&mut self, kind: u8) {
        let (size, dist, color) = match kind {
            0 => (12.0, 12.0, [0, 0, 0, 160]),
            1 => (18.0, 0.0, [255, 230, 120, 220]),
            2 | 3 => (6.0, 0.0, [255, 255, 255, 255]),
            4 => (10.0, 6.0, [0, 0, 0, 170]),
            5 => (12.0, 0.0, [255, 250, 200, 220]),
            6 => (8.0, 0.0, [255, 255, 255, 200]),
            7 => (0.0, 0.0, self.primary.to_array()),
            _ => (0.0, 0.0, self.primary.to_array()),
        };
        self.sheet = Some(Sheet::Effect { kind, size, dist, angle: 120.0, color, color2: if kind == 6 { [0, 0, 0, 200] } else { self.secondary.to_array() }, position: 2, blend: if kind == 7 || kind == 8 { 0 } else { 0 }, depth: 1.0 });
    }
}

pub fn blend_index(b: Blend) -> usize {
    BLENDS.iter().position(|x| x.0 == b).unwrap_or(0)
}
