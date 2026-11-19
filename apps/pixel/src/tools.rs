use crate::doc::{BLENDS, Blend, blend_name};
use studio_ui::i18n::tl;
use crate::{Pixel, Sheet};
use eframe::egui::{self, Color32, CornerRadius, Key, Painter, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, vec2};
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
    CurvePen,
    MagLasso,
    QuickSelect,
    Wand,
    ObjectSelect,
    Crop,
    PerspCrop,
    Picker,
    Sampler,
    Ruler,
    Note,
    Count,
    Remove,
    Heal,
    Patch,
    ContentMove,
    RedEye,
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
    PhotoText,
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
    (Tool::CurvePen, "Plume de courbure", Key::P, "P"),
    (Tool::QuickSelect, "Sélection rapide", Key::W, "W"),
    (Tool::Wand, "Baguette magique", Key::W, "W"),
    (Tool::ObjectSelect, "Sélection d'objet", Key::W, "W"),
    (Tool::Crop, "Recadrage", Key::C, "C"),
    (Tool::PerspCrop, "Recadrage de la perspective", Key::C, "C"),
    (Tool::Picker, "Pipette", Key::I, "I"),
    (Tool::Sampler, "Échantillonnage de couleur", Key::I, "I"),
    (Tool::Ruler, "Règle", Key::I, "I"),
    (Tool::Note, "Annotation", Key::I, "I"),
    (Tool::Count, "Comptage", Key::I, "I"),
    (Tool::Remove, "Suppression intelligente", Key::J, "J"),
    (Tool::Heal, "Correcteur", Key::J, "J"),
    (Tool::Patch, "Pièce", Key::J, "J"),
    (Tool::ContentMove, "Déplacement basé sur le contenu", Key::J, "J"),
    (Tool::RedEye, "Yeux rouges", Key::J, "J"),
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
    (Tool::PhotoText, "Texte de la photo (OCR)", Key::T, "T"),
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
    &[Tool::Lasso, Tool::PolyLasso, Tool::MagLasso, Tool::CurvePen],
    &[Tool::ObjectSelect, Tool::QuickSelect, Tool::Wand],
    &[Tool::Crop, Tool::PerspCrop],
    &[Tool::Picker, Tool::Sampler, Tool::Ruler, Tool::Note, Tool::Count],
    &[Tool::Remove, Tool::Heal, Tool::Patch, Tool::ContentMove, Tool::RedEye, Tool::Clone],
    &[Tool::HistoryBrush],
    &[Tool::Brush, Tool::Pencil, Tool::ColorReplace, Tool::Mixer],
    &[Tool::Eraser, Tool::BgEraser, Tool::MagicEraser],
    &[Tool::Gradient, Tool::Fill],
    &[Tool::Blur, Tool::Sharpen, Tool::Smudge],
    &[Tool::Dodge, Tool::Burn, Tool::Sponge],
    &[Tool::TextH, Tool::TextV, Tool::PhotoText],
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
    egui::ComboBox::from_id_salt(id).selected_text(tl(items.get(*value).unwrap_or(&""))).width(width).show_ui(ui, |ui| {
        for (k, it) in items.iter().enumerate() {
            if ui.selectable_label(*value == k, tl(it)).clicked() {
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
    pro::tip(ui, t);
}

fn small(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(tl(&t)).size(12.0).color(MUTED));
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
                if resp.on_hover_text(tl("Couleurs principale et secondaire\nClic : inverser (X) · D : noir et blanc")).clicked() {
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
        if pro::segmented(ui, &mut m, &[tl("Nouvelle"), tl("Ajouter"), tl("Soustraire"), tl("Intersection")]) {
            self.sel_mode = m as u8;
        }
        pro::toolbar_sep(ui);
    }

    pub fn options_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("options").exact_size(34.0).frame(egui::Frame::new().fill(pro::CHROME).inner_margin(egui::Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().slider_width = 72.0;
                let (r, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(3), FIELD);
                tool_icon(ui.painter(), r.shrink(5.0), self.tool, INK);
                ui.label(RichText::new(tl(tool_name(self.tool))).font(semibold(12.0)).color(INK));
                pro::toolbar_sep(ui);
                let right = 8.0;
                let avail = ui.available_width() - right;
                ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_size(ui.cursor().min, vec2(avail.max(100.0), 34.0))), |ui| {
                    ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                    egui::ScrollArea::horizontal().id_salt("pixel-options").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                        ui.set_min_height(34.0);
                        ui.horizontal_centered(|ui| self.tool_options(ui));
                    });
                });
            });
        });
    }

    fn tool_options(&mut self, ui: &mut Ui) {
        let t = self.tool;
        match t {
            t if brushy(t) => {
                pro::option_slider(ui, tl("Taille"), &mut self.brush.radius, 0.5..=500.0, " px", true);
                if t != Tool::Pencil {
                    pro::option_slider(ui, tl("Dureté"), &mut self.brush.hardness, 0.0..=1.0, "", false);
                }
                let flow_label = if matches!(t, Tool::Dodge | Tool::Burn) { "Exposition" } else if matches!(t, Tool::Blur | Tool::Sharpen | Tool::Smudge) { "Intensité" } else { "Flux" };
                pro::option_slider(ui, flow_label, &mut self.brush.flow, 0.01..=1.0, "", false);
                match t {
                    Tool::Brush | Tool::Pencil => {
                        small(ui, tl("Mode"));
                        blend_combo(ui, tl("brush-mode"), &mut self.brush.mode, 140.0);
                        pro::option_slider(ui, tl("Lissage"), &mut self.smoothing, 0.0..=0.95, "", false);
                        ui.checkbox(&mut self.symmetry, tl("Symétrie")).on_hover_text(tl("Répète chaque trait en miroir vertical"));
                        ui.checkbox(&mut self.symmetry_v, tl("Miroir H")).on_hover_text(tl("Répète chaque trait en miroir horizontal"));
                    }
                    Tool::Mixer => {
                        pro::option_slider(ui, tl("Humidité"), &mut self.mixer_wet, 0.0..=1.0, "", false);
                        pro::option_slider(ui, tl("Lissage"), &mut self.smoothing, 0.0..=0.95, "", false);
                    }
                    Tool::ColorReplace => hint(ui, tl("Repeint la teinte avec la couleur principale en gardant la lumière")),
                    Tool::Eraser => {
                        ui.checkbox(&mut self.erase_history, tl("Effacer vers l'historique")).on_hover_text(tl("Restaure l'état de l'instantané au lieu de rendre transparent"));
                        ui.checkbox(&mut self.symmetry, tl("Symétrie"));
                    }
                    Tool::BgEraser => {
                        pro::option_slider(ui, tl("Tolérance"), &mut self.bg_tol, 0.01..=1.0, "", false);
                        ui.checkbox(&mut self.bg_continuous, tl("Échantillonnage continu"));
                    }
                    Tool::HistoryBrush => {
                        if pro::secondary(ui, tl("Instantané")).on_hover_text(tl("Mémorise le calque actif comme source")).clicked() {
                            self.snapshot_history();
                        }
                        hint(ui, if self.history_src.is_some() { "Peins pour revenir à l'instantané" } else { "Aucun instantané" });
                    }
                    Tool::Sponge => {
                        let mut k = self.sponge_saturate as usize;
                        if pro::segmented(ui, &mut k, &[tl("Désaturer"), tl("Saturer")]) {
                            self.sponge_saturate = k == 1;
                        }
                    }
                    Tool::Dodge | Tool::Burn => {
                        small(ui, tl("Plage"));
                        let mut r = self.dodge_range as usize;
                        if pro::segmented(ui, &mut r, &[tl("Tons foncés"), tl("Tons moyens"), tl("Tons clairs")]) {
                            self.dodge_range = r as u8;
                        }
                    }
                    Tool::Smudge => {
                        ui.checkbox(&mut self.finger_paint, tl("Peinture au doigt")).on_hover_text(tl("Commence chaque trait avec la couleur principale"));
                    }
                    Tool::Clone | Tool::Heal => {
                        ui.checkbox(&mut self.clone_aligned, tl("Aligné"));
                        ui.checkbox(&mut self.sample_all, tl("Tous les calques"));
                        hint(ui, if self.clone_src.is_some() { "Alt+clic pour changer de source" } else { "Alt+clic sur la zone source" });
                    }
                    _ => {}
                }
            }
            Tool::QuickSelect => {
                self.sel_options(ui);
                pro::option_slider(ui, tl("Taille"), &mut self.brush.radius, 1.0..=300.0, " px", true);
                pro::option_slider(ui, tl("Tolérance"), &mut self.tolerance, 0..=255, "", false);
                hint(ui, tl("Alt pour retirer"));
            }
            Tool::Wand => {
                self.sel_options(ui);
                pro::option_slider(ui, tl("Tolérance"), &mut self.tolerance, 0..=255, "", false);
                ui.checkbox(&mut self.contiguous, tl("Contiguë"));
                ui.checkbox(&mut self.sample_all, tl("Tous les calques"));
            }
            Tool::Marquee | Tool::EllipseMarquee | Tool::RowMarquee | Tool::ColMarquee => {
                self.sel_options(ui);
                pro::option_slider(ui, tl("Contour progressif"), &mut self.feather, 0.0..=100.0, " px", false);
                if matches!(t, Tool::Marquee | Tool::EllipseMarquee) {
                    combo(ui, tl("mstyle"), &mut self.marquee_style, &["Normal", "Proportions fixes", "Taille fixe"], 130.0);
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
                pro::option_slider(ui, tl("Contour progressif"), &mut self.feather, 0.0..=100.0, " px", false);
                if t == Tool::MagLasso {
                    pro::option_slider(ui, tl("Largeur"), &mut self.mag_width, 2.0..=40.0, " px", false);
                }
                if t != Tool::Lasso {
                    hint(ui, if t == Tool::PolyLasso { "Clic par sommet · double-clic ou Entrée pour fermer" } else { "Glisse le long d'un bord" });
                }
            }
            Tool::Crop => {
                combo(ui, tl("cratio"), &mut self.crop_ratio, &CROP_RATIOS.map(|r| r.0), 120.0);
                small(ui, tl("Grille"));
                combo(ui, tl("cover"), &mut self.crop_overlay, &["Aucune", "Règle des tiers", "Grille", "Diagonale", "Nombre d'or"], 130.0);
                if let Some(c) = self.crop {
                    ui.label(RichText::new(format!("{} × {} px", c.width(), c.height())).size(12.0).color(INK));
                    if pro::primary(ui, tl("Valider")).clicked() {
                        self.commit_crop();
                    }
                    if pro::secondary(ui, tl("Annuler")).clicked() {
                        self.crop = None;
                    }
                } else {
                    hint(ui, tl("Glisse pour tracer le cadre à conserver"));
                }
            }
            Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line | Tool::CustomShape => {
                if t != Tool::Line {
                    ui.checkbox(&mut self.shape_fill, tl("Fond"));
                    ui.checkbox(&mut self.shape_stroke, tl("Contour")).on_hover_text(tl("Contour avec la couleur secondaire quand le fond est actif"));
                }
                if self.shape_stroke || t == Tool::Line {
                    pro::option_slider(ui, tl("Épaisseur"), &mut self.stroke_w, 1.0..=200.0, " px", true);
                }
                match t {
                    Tool::Rect => {
                        pro::option_slider(ui, tl("Arrondi"), &mut self.corner, 0.0..=500.0, " px", false);
                    }
                    Tool::Polygon => {
                        pro::option_slider(ui, tl("Côtés"), &mut self.sides, 3..=24, "", false);
                        ui.checkbox(&mut self.star, tl("Étoile"));
                        if self.star {
                            pro::option_slider(ui, tl("Retrait"), &mut self.inset, 0.05..=0.95, "", false);
                        }
                    }
                    Tool::Line => {
                        ui.checkbox(&mut self.arrow_start, tl("Flèche au début"));
                        ui.checkbox(&mut self.arrow_end, tl("Flèche à la fin"));
                    }
                    Tool::CustomShape => {
                        combo(ui, tl("custom"), &mut self.custom, &CUSTOM_SHAPES, 110.0);
                    }
                    _ => {}
                }
                small(ui, tl("Mode"));
                blend_combo(ui, tl("shape-mode"), &mut self.shape_mode, 120.0);
                ui.checkbox(&mut self.shape_layer, tl("Nouveau calque"));
            }
            Tool::PhotoText => {
                let ctx = ui.ctx().clone();
                self.pt_options(ui, &ctx);
            }
            Tool::TextH | Tool::TextV => {
                combo(ui, tl("font"), &mut self.text_font, &crate::shapes::FONTS.map(|f| f.0), 130.0);
                ui.add(egui::DragValue::new(&mut self.text_size).range(4.0..=800.0).suffix(" px"));
                ui.toggle_value(&mut self.text_bold, RichText::new(tl("G")).strong()).on_hover_text(tl("Gras"));
                ui.toggle_value(&mut self.text_italic, RichText::new(tl("I")).italics()).on_hover_text(tl("Italique"));
                ui.toggle_value(&mut self.text_underline, RichText::new(tl("S")).underline()).on_hover_text(tl("Souligné"));
                pro::segmented(ui, &mut self.text_align, &[tl("Gauche"), tl("Centre"), tl("Droite")]);
                small(ui, tl("Approche"));
                ui.add(egui::DragValue::new(&mut self.tracking).range(-50.0..=200.0).suffix(" ‰"));
                small(ui, tl("Interligne"));
                ui.add(egui::DragValue::new(&mut self.leading).range(0.5..=3.0).speed(0.01).suffix(" ×"));
            }
            Tool::Gradient => {
                combo(ui, tl("gpreset"), &mut self.grad_preset, &GRADIENT_PRESETS, 170.0);
                pro::segmented(ui, &mut self.grad_kind, &[tl("Linéaire"), tl("Radial"), tl("Angulaire"), tl("Réfléchi"), tl("Losange")]);
                small(ui, tl("Mode"));
                blend_combo(ui, tl("grad-mode"), &mut self.grad_mode, 120.0);
                pro::option_slider(ui, tl("Opacité"), &mut self.grad_opacity, 0.01..=1.0, "", false);
                ui.checkbox(&mut self.grad_reverse, tl("Inverser"));
                ui.checkbox(&mut self.grad_dither, tl("Tramage"));
            }
            Tool::Fill => {
                pro::option_slider(ui, tl("Tolérance"), &mut self.tolerance, 0..=255, "", false);
                pro::option_slider(ui, tl("Opacité"), &mut self.fill_opacity, 0.01..=1.0, "", false);
                ui.checkbox(&mut self.contiguous, tl("Contigu"));
                ui.checkbox(&mut self.sample_all, tl("Tous les calques"));
            }
            Tool::MagicEraser => {
                pro::option_slider(ui, tl("Tolérance"), &mut self.tolerance, 0..=255, "", false);
                pro::option_slider(ui, tl("Opacité"), &mut self.fill_opacity, 0.01..=1.0, "", false);
                ui.checkbox(&mut self.contiguous, tl("Contigu"));
            }
            Tool::Move => {
                ui.checkbox(&mut self.auto_select, tl("Sélection automatique")).on_hover_text(tl("Le clic choisit le calque visible sous le pointeur"));
                ui.checkbox(&mut self.show_bounds, tl("Cadre de sélection"));
                pro::toolbar_sep(ui);
                for (k, tip) in ["Aligner à gauche", "Centrer horizontalement", "Aligner à droite", "Aligner en haut", "Centrer verticalement", "Aligner en bas"].iter().enumerate() {
                    if pro::icon_button(ui, 28.0, false, &format!("{tip} (sur la sélection ou la zone de travail)"), |p, r, c| align_icon(p, r, k as u8, c)).clicked() {
                        let to = self.doc.selection.unwrap_or(self.doc.full());
                        self.doc.next_label = Some(tip.to_string());
                        self.doc.align_layer(k as u8, to);
                    }
                }
                hint(ui, tl("Flèches : 1 px · Maj : 10 px"));
            }
            Tool::Hand | Tool::Zoom | Tool::RotateView => {
                if t == Tool::Zoom {
                    let mut k = self.zoom_out as usize;
                    if pro::segmented(ui, &mut k, &[tl("Zoom avant"), tl("Zoom arrière")]) {
                        self.zoom_out = k == 1;
                    }
                }
                if t == Tool::RotateView {
                    ui.add(egui::DragValue::new(&mut self.view_angle).range(-180.0..=180.0).suffix("°").speed(0.5));
                    if pro::secondary(ui, tl("Réinitialiser la vue")).clicked() {
                        self.view_angle = 0.0;
                    }
                }
                if pro::secondary(ui, tl("Ajuster à l'écran")).clicked() {
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
                small(ui, tl("Échantillon"));
                combo(ui, tl("psize"), &mut self.picker_size, &PICK_SIZES, 130.0);
                ui.checkbox(&mut self.sample_all, tl("Tous les calques"));
                if t == Tool::Sampler {
                    if !self.samplers.is_empty() && pro::secondary(ui, tl("Effacer")).clicked() {
                        self.samplers.clear();
                    }
                    hint(ui, tl("Jusqu'à 4 points, lus dans Infos"));
                } else {
                    hint(ui, tl("Alt+clic : couleur secondaire"));
                }
            }
            Tool::Ruler => {
                if let Some((a, b)) = self.ruler {
                    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                    ui.label(RichText::new(format!("L {:.1} px   A {:.1}°   ΔX {:.0}   ΔY {:.0}", (dx * dx + dy * dy).sqrt(), -dy.atan2(dx).to_degrees(), dx, dy)).size(12.0).color(INK).monospace());
                    if pro::secondary(ui, tl("Redresser le calque")).clicked() {
                        let mut ang = dy.atan2(dx).to_degrees();
                        if ang.abs() > 45.0 {
                            ang -= 90.0 * ang.signum();
                        }
                        self.doc.next_label = Some("Redresser".into());
                        self.doc.transform_layer(1.0, -ang);
                        self.ruler = None;
                    }
                    if pro::secondary(ui, tl("Effacer")).clicked() {
                        self.ruler = None;
                    }
                } else {
                    hint(ui, tl("Glisse pour mesurer une distance et un angle"));
                }
            }
            Tool::Note => {
                hint(ui, &format!("Clique pour poser une note · {} note(s)", self.notes.len()));
                if !self.notes.is_empty() && pro::secondary(ui, tl("Tout effacer")).clicked() {
                    self.notes.clear();
                }
            }
            Tool::Remove => {
                pro::option_slider(ui, tl("Taille"), &mut self.brush.radius, 2.0..=300.0, " px", true);
                hint(ui, tl("Peins sur ce qui doit disparaître : le fond est reconstruit autour"));
            }
            Tool::Patch => {
                pro::option_slider(ui, tl("Diffusion"), &mut self.patch_diffusion, 1.0..=20.0, "", false);
                hint(ui, if self.doc.selection.is_some() { "Fais glisser la sélection vers une zone saine" } else { "Entoure la zone à réparer" });
            }
            Tool::ContentMove => hint(ui, if self.doc.selection.is_some() { "Fais glisser la sélection : l'objet est déplacé et le fond est reconstruit" } else { "Sélectionne d'abord un objet (lasso, sélection d'objet…)" }),
            Tool::RedEye => {
                pro::option_slider(ui, tl("Taille de la pupille"), &mut self.brush.radius, 2.0..=80.0, " px", false);
                pro::option_slider(ui, tl("Assombrir"), &mut self.red_eye_dark, 0.0..=1.0, "", false);
                hint(ui, tl("Clique sur chaque œil rouge"));
            }
            Tool::ObjectSelect => {
                pro::option_slider(ui, tl("Tolérance"), &mut self.tolerance, 0..=255, "", false);
                hint(ui, tl("Trace un rectangle autour de l'objet : il est détouré automatiquement"));
            }
            Tool::CurvePen => hint(ui, &format!("Clique pour poser des points, double-clic ou clic sur le 1er point pour fermer la courbe en sélection · {} point(s)", self.poly.len())),
            Tool::PerspCrop => {
                hint(ui, &format!("Clique les 4 coins dans l'ordre : haut gauche, haut droit, bas droit, bas gauche · {}/4", self.persp.len()));
                if !self.persp.is_empty() && pro::secondary(ui, tl("Annuler")).clicked() {
                    self.persp.clear();
                }
            }
            Tool::Count => {
                hint(ui, &format!("Clique pour compter · total : {}", self.counts.len()));
                if !self.counts.is_empty() && pro::secondary(ui, tl("Remettre à zéro")).clicked() {
                    self.counts.clear();
                }
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
        Tool::Lasso | Tool::PolyLasso | Tool::MagLasso | Tool::CurvePen => "Lassos",
        Tool::ObjectSelect | Tool::RedEye | Tool::ContentMove | Tool::PerspCrop | Tool::Count => "Outils de Photoshop 2027",
        Tool::QuickSelect | Tool::Wand => "Sélection rapide et baguette",
        Tool::Brush | Tool::Pencil | Tool::ColorReplace | Tool::Mixer => "Pinceaux et dynamique",
        Tool::Eraser | Tool::BgEraser | Tool::MagicEraser => "Gommes",
        Tool::Gradient | Tool::Fill => "Dégradés",
        t if is_shape(t) => "Formes",
        Tool::TextH | Tool::TextV | Tool::PhotoText => "Texte",
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

/// Lucide icon for each tool, so Pixel's tool bar matches the rest of the suite.
pub fn tool_lucide(tool: Tool) -> &'static str {
    use Tool::*;
    match tool {
        Move => "move",
        Marquee => "square-dashed",
        EllipseMarquee => "circle-dashed",
        RowMarquee => "rectangle-horizontal",
        ColMarquee => "rectangle-vertical",
        Lasso => "lasso",
        PolyLasso => "lasso-select",
        CurvePen => "pen-tool",
        MagLasso => "magnet",
        QuickSelect => "scan-line",
        Wand => "wand-sparkles",
        ObjectSelect => "scan-eye",
        Crop => "crop",
        PerspCrop => "frame",
        Picker => "pipette",
        Sampler => "crosshair",
        Ruler => "ruler",
        Note => "sticky-note",
        Count => "hash",
        Remove => "circle-slash",
        Heal => "bandage",
        Patch => "replace",
        ContentMove => "move-diagonal-2",
        RedEye => "eye",
        Clone => "stamp",
        HistoryBrush => "history",
        Brush => "paintbrush",
        Pencil => "pencil",
        ColorReplace => "palette",
        Mixer => "paintbrush-vertical",
        Eraser => "eraser",
        BgEraser => "image-minus",
        MagicEraser => "wand",
        Gradient => "blend",
        Fill => "paint-bucket",
        Blur => "droplet",
        Sharpen => "triangle",
        Smudge => "pointer",
        Dodge => "sun-dim",
        Burn => "flame",
        Sponge => "cloud",
        TextH => "type",
        TextV => "text-cursor",
        PhotoText => "scan-text",
        Rect => "square",
        Ellipse => "circle",
        Polygon => "hexagon",
        Line => "slash",
        CustomShape => "shapes",
        Hand => "hand",
        RotateView => "rotate-ccw",
        Zoom => "zoom-in",
    }
}

pub fn tool_icon(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    pro::lucide(p, r.expand(r.width() * 0.12), tool_lucide(tool), c);
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

#[cfg(test)]
mod icon_tests {
    #[test]
    fn every_tool_has_an_existing_icon() {
        for (t, label, _, _) in super::TOOLS {
            let name = super::tool_lucide(*t);
            assert!(studio_ui::pro::has_lucide(name), "{label} -> {name} is not in the icon set");
        }
    }
}
