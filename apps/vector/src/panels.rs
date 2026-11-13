use crate::model::{self, Geom, Item, MeshPt, Stop};
use crate::{Dlg, Doc, GROUPS, NewDoc, PRESETS, Style, Tool, VectorApp, arr, c32, geom, tool_name, FONTS};
use eframe::egui::{self, Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};

pub const LIBRARIES: [(&str, [[u8; 3]; 8]); 16] = [
    ("Ethone", [[255, 122, 60], [255, 59, 107], [10, 132, 255], [48, 209, 88], [255, 196, 0], [191, 90, 242], [20, 20, 24], [250, 250, 250]]),
    ("Pastel", [[255, 209, 220], [255, 234, 167], [204, 236, 214], [196, 222, 255], [221, 208, 255], [255, 220, 196], [235, 235, 240], [250, 250, 250]]),
    ("Néon", [[0, 255, 200], [255, 0, 170], [255, 240, 0], [0, 170, 255], [170, 0, 255], [57, 255, 20], [255, 95, 31], [10, 10, 20]]),
    ("Terre", [[120, 72, 48], [176, 120, 72], [214, 180, 130], [104, 120, 72], [60, 72, 52], [196, 98, 45], [90, 60, 40], [240, 230, 210]]),
    ("Océan", [[2, 48, 71], [2, 119, 189], [79, 195, 247], [178, 235, 242], [0, 150, 136], [0, 77, 64], [224, 247, 250], [255, 255, 255]]),
    ("Mono", [[0, 0, 0], [36, 36, 36], [72, 72, 72], [110, 110, 110], [150, 150, 150], [190, 190, 190], [225, 225, 225], [255, 255, 255]]),
    ("Automne", [[153, 27, 30], [204, 85, 0], [230, 145, 56], [242, 196, 98], [128, 90, 40], [85, 107, 47], [62, 39, 35], [250, 240, 220]]),
    ("Printemps", [[255, 183, 197], [255, 223, 186], [255, 255, 186], [186, 255, 201], [186, 225, 255], [144, 238, 144], [255, 105, 180], [255, 250, 240]]),
    ("Rétro", [[230, 57, 70], [241, 250, 238], [168, 218, 220], [69, 123, 157], [29, 53, 87], [244, 162, 97], [233, 196, 106], [42, 157, 143]]),
    ("Material", [[244, 67, 54], [233, 30, 99], [156, 39, 176], [63, 81, 181], [33, 150, 243], [0, 150, 136], [76, 175, 80], [255, 152, 0]]),
    ("Bonbons", [[255, 99, 146], [255, 182, 39], [72, 219, 251], [29, 209, 161], [254, 202, 87], [255, 159, 243], [84, 160, 255], [95, 39, 205]]),
    ("Forêt", [[27, 67, 50], [45, 106, 79], [64, 145, 108], [82, 183, 136], [116, 198, 157], [149, 213, 178], [183, 228, 199], [216, 243, 220]]),
    ("Coucher de soleil", [[53, 80, 112], [109, 89, 122], [181, 101, 118], [229, 107, 111], [234, 172, 139], [255, 205, 178], [255, 180, 162], [229, 152, 155]]),
    ("Métal", [[191, 155, 48], [212, 175, 55], [229, 228, 226], [192, 192, 192], [140, 120, 83], [184, 115, 51], [110, 110, 120], [60, 60, 70]]),
    ("Peau", [[255, 224, 189], [255, 205, 148], [234, 192, 134], [255, 173, 96], [200, 140, 90], [160, 100, 60], [120, 72, 40], [80, 50, 30]]),
    ("Corporate", [[0, 51, 102], [0, 102, 204], [102, 153, 204], [204, 204, 204], [51, 51, 51], [0, 153, 153], [255, 153, 0], [255, 255, 255]]),
];

pub fn graphic_styles() -> Vec<(&'static str, Item)> {
    let b = Item::blank;
    vec![
        ("Plat", Item { fill: Some([10, 132, 255, 255]), stroke: None, ..b() }),
        ("Contour épais", Item { fill: None, stroke: Some([20, 20, 24, 255]), stroke_w: 8.0, join: 0, cap: 0, ..b() }),
        ("Néon", Item { fill: None, stroke: Some([0, 255, 200, 255]), stroke_w: 4.0, glow_out: 14.0, glow_out_c: [0, 255, 200, 230], ..b() }),
        ("Ombre douce", Item { fill: Some([255, 255, 255, 255]), shadow: true, sh_y: 10.0, sh_blur: 24.0, sh_color: [0, 0, 0, 70], ..b() }),
        ("Verre", Item { fill: Some([255, 255, 255, 70]), stroke: Some([255, 255, 255, 180]), stroke_w: 1.5, glow_in: 10.0, ..b() }),
        ("Coucher de soleil", Item { fill: Some([255, 94, 98, 255]), stops: vec![Stop { t: 0.0, c: [255, 195, 113, 255] }, Stop { t: 0.5, c: [255, 94, 98, 255] }, Stop { t: 1.0, c: [94, 53, 177, 255] }], grad_angle: 90.0, ..b() }),
        ("Pointillés", Item { fill: None, stroke: Some([20, 20, 24, 255]), stroke_w: 3.0, dash: 0.1, gap: 8.0, cap: 0, ..b() }),
        ("Croquis", Item { fill: None, stroke: Some([40, 40, 48, 255]), stroke_w: 2.5, rough: 4.0, rough_freq: 0.03, ..b() }),
        ("Relief", Item { fill: Some([255, 176, 46, 255]), shadow: true, sh_y: 4.0, sh_blur: 6.0, glow_in: 6.0, glow_in_c: [255, 255, 255, 160], ..b() }),
        ("Papier", Item { fill: Some([240, 232, 214, 255]), grain: 0.35, shadow: true, sh_y: 3.0, sh_blur: 8.0, sh_color: [60, 40, 0, 60], ..b() }),
        ("Lueur dorée", Item { fill: Some([255, 214, 10, 255]), glow_out: 18.0, glow_out_c: [255, 170, 0, 200], ..b() }),
        ("Flou artistique", Item { fill: Some([191, 90, 242, 255]), blur: 10.0, opacity: 0.85, ..b() }),
        ("Contour progressif", Item { fill: Some([48, 209, 88, 255]), feather: 14.0, ..b() }),
        ("Encre", Item { fill: Some([20, 20, 24, 255]), stroke: Some([20, 20, 24, 255]), stroke_w: 2.0, rough: 2.0, rough_freq: 0.08, ..b() }),
    ]
}

pub fn row_label(ui: &mut Ui, t: &str, w: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
    ui.painter().text(r.left_center(), Align2::LEFT_CENTER, t, FontId::proportional(12.5), MUTED);
}

fn small(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(12.5).color(MUTED));
}

fn hint(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(12.0).color(FAINT));
}

fn sep(ui: &mut Ui) {
    ui.add_space(6.0);
    let (s, _) = ui.allocate_exact_size(vec2(1.0, 22.0), Sense::hover());
    ui.painter().rect_filled(s, CornerRadius::ZERO, LINE);
    ui.add_space(6.0);
}

fn slider(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) -> bool {
    small(ui, label);
    ui.spacing_mut().slider_width = (ui.available_width() - 74.0).max(60.0);
    ui.add(egui::Slider::new(v, range).suffix(suffix)).changed()
}

fn dv(ui: &mut Ui, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) -> bool {
    ui.add(egui::DragValue::new(v).range(range).speed(0.5).max_decimals(1).suffix(suffix)).changed()
}

fn opt_dv<N: egui::emath::Numeric>(ui: &mut Ui, label: &str, v: &mut N, range: std::ops::RangeInclusive<N>, suffix: &str) {
    ui.label(RichText::new(label).size(12.5).color(MUTED));
    ui.add(egui::DragValue::new(v).range(range).suffix(suffix));
    ui.add_space(8.0);
}

pub fn preset_card(ui: &mut Ui, name: &str, detail: &str, w: f32, h: f32, selected: bool, width: f32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 128.0), Sense::click());
    let p = ui.painter();
    p.rect(r, CornerRadius::same(12), if resp.hovered() { Color32::from_rgb(0x2f, 0x2f, 0x35) } else { Color32::from_rgb(0x27, 0x27, 0x2c) }, Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { BLUE } else { LINE }), StrokeKind::Inside);
    let area = Rect::from_center_size(r.center_top() + vec2(0.0, 42.0), vec2(width - 40.0, 52.0));
    let s = (area.width() / w).min(area.height() / h);
    p.rect(Rect::from_center_size(area.center(), vec2(w, h) * s), CornerRadius::same(3), Color32::from_white_alpha(18), Stroke::new(1.2, if selected { BLUE } else { MUTED }), StrokeKind::Inside);
    p.text(pos2(r.center().x, r.bottom() - 34.0), Align2::CENTER_CENTER, name, semibold(13.0), INK_STRONG);
    p.text(pos2(r.center().x, r.bottom() - 16.0), Align2::CENTER_CENTER, detail, FontId::proportional(10.5), MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn color_toggle(ui: &mut Ui, label: &str, c: &mut Option<Color32>) -> bool {
    let open_id = ui.make_persistent_id(("picker-open", label));
    let mut open = ui.ctx().data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
    let before = *c;
    ui.horizontal(|ui| {
        let mut on = c.is_some();
        if ui.checkbox(&mut on, RichText::new(label).color(INK)).changed() {
            *c = on.then_some(c.unwrap_or(Color32::BLACK));
            open = on;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (r, resp) = ui.allocate_exact_size(vec2(44.0, 24.0), Sense::click());
            match c {
                Some(col) => {
                    checker_rect(ui.painter(), r);
                    ui.painter().rect(r, CornerRadius::same(6), *col, Stroke::new(1.0, if open { BLUE } else { LINE }), StrokeKind::Inside);
                    ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}", col.r(), col.g(), col.b())).monospace().size(12.0).color(MUTED));
                }
                None => {
                    ui.painter().rect(r, CornerRadius::same(6), FIELD, Stroke::new(1.0, LINE), StrokeKind::Inside);
                    ui.painter().line_segment([r.left_bottom() + vec2(4.0, -4.0), r.right_top() + vec2(-4.0, 4.0)], Stroke::new(1.5, RED));
                }
            }
            if resp.on_hover_text("Modifier la couleur").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                if c.is_none() {
                    *c = Some(Color32::BLACK);
                }
                open = !open;
            }
        });
    });
    if open && let Some(col) = c {
        ui.add_space(4.0);
        let w = ui.available_width();
        ui.push_id(("picker", label), |ui| {
            let mut opaque = Color32::from_rgb(col.r(), col.g(), col.b());
            if pro::color_picker(ui, &mut opaque, w) || opaque != Color32::from_rgb(col.r(), col.g(), col.b()) {
                *col = Color32::from_rgba_unmultiplied(opaque.r(), opaque.g(), opaque.b(), col.a());
            }
            let mut a = col.a() as f32 / 2.55;
            ui.horizontal(|ui| {
                row_label(ui, "Opacité de la couleur", 130.0);
                if dv(ui, &mut a, 0.0..=100.0, " %") {
                    *col = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), (a * 2.55).round() as u8);
                }
            });
        });
        ui.add_space(6.0);
    }
    ui.ctx().data_mut(|d| d.insert_temp(open_id, open));
    *c != before
}

fn color_opt(ui: &mut Ui, label: &str, c: &mut Option<[u8; 4]>) -> bool {
    let mut x = c.map(c32);
    let ch = color_toggle(ui, label, &mut x);
    if ch {
        *c = x.map(arr);
    }
    ch
}

fn color_btn(ui: &mut Ui, c: &mut [u8; 4]) {
    let mut col = c32(*c);
    if ui.color_edit_button_srgba(&mut col).changed() {
        *c = arr(col);
    }
}

fn checker_rect(p: &Painter, r: Rect) {
    p.rect_filled(r, CornerRadius::same(6), Color32::WHITE);
    let s = 6.0;
    let mut y = r.top();
    let mut row = 0;
    while y < r.bottom() {
        let mut x = r.left() + if row % 2 == 0 { 0.0 } else { s };
        while x < r.right() {
            p.rect_filled(Rect::from_min_max(pos2(x, y), pos2((x + s).min(r.right()), (y + s).min(r.bottom()))), CornerRadius::ZERO, Color32::from_gray(204));
            x += 2.0 * s;
        }
        y += s;
        row += 1;
    }
}

fn style_swatches(ui: &mut Ui, style: &mut Style) {
    ui.label(RichText::new("Fond").size(12.5).color(MUTED));
    swatch_btn(ui, &mut style.fill);
    ui.label(RichText::new("Contour").size(12.5).color(MUTED));
    swatch_btn(ui, &mut style.stroke);
    ui.add(egui::DragValue::new(&mut style.stroke_w).range(0.0..=200.0).speed(0.2).suffix(" px"));
    ui.add_space(8.0);
}

fn swatch_btn(ui: &mut Ui, c: &mut Option<Color32>) {
    let mut on = c.is_some();
    if ui.checkbox(&mut on, "").changed() {
        *c = on.then_some(c.unwrap_or(Color32::BLACK));
    }
    if let Some(col) = c {
        ui.color_edit_button_srgba(col);
    }
}

impl VectorApp {
    pub(crate) fn home_screen(&mut self, root: &mut Ui) {
        egui::Panel::left("home-nav").exact_size(230.0).resizable(false).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(18, 26))).show(root, |ui| {
            if pro::primary(ui, "Nouveau fichier").clicked() {
                self.new_doc = Some(NewDoc { w: 1000.0, h: 1000.0, preset: 0, transparent: false });
            }
            ui.add_space(8.0);
            if pro::secondary(ui, "Ouvrir…").clicked() {
                self.open();
            }
            ui.add_space(26.0);
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
            ui.painter().rect_filled(r, CornerRadius::same(9), BLUE.gamma_multiply(0.22));
            ui.painter().text(r.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, "Accueil", semibold(14.0), INK_STRONG);
            ui.add_space(14.0);
            hint(ui, "Ouvre aussi les fichiers SVG, PNG et JPEG : glisse-les dans la fenêtre.");
        });
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin { left: 40, right: 40, top: 34, bottom: 20 })).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(40.0))));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Bienvenue dans Vector").font(semibold(28.0)).color(INK_STRONG));
                        ui.label(RichText::new("Logos, icônes et illustrations nets à toutes les tailles").color(MUTED));
                    });
                });
                ui.add_space(28.0);
                ui.label(RichText::new("Créer un plan de travail").font(semibold(16.0)).color(INK));
                ui.add_space(12.0);
                let gap = 14.0;
                let card_w = 160.0;
                let cols = (((ui.available_width() + gap) / (card_w + gap)).floor() as usize).max(1);
                let mut chosen = None;
                for row in PRESETS.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for (name, kind, w, h) in row {
                            if preset_card(ui, name, &format!("{w} × {h} · {kind}"), *w, *h, false, card_w).clicked() {
                                chosen = Some((*w, *h));
                            }
                        }
                    });
                    ui.add_space(gap);
                }
                if let Some((w, h)) = chosen {
                    self.set_doc(Doc::new(w, h), None);
                }
                ui.add_space(14.0);
                ui.label(RichText::new("Récents").font(semibold(16.0)).color(INK));
                ui.add_space(10.0);
                if self.recents.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().min(760.0), 110.0), Sense::hover());
                    let p = ui.painter();
                    p.rect(r, CornerRadius::same(14), Color32::from_white_alpha(4), Stroke::new(1.0, LINE), StrokeKind::Inside);
                    p.text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "Tes dessins enregistrés apparaîtront ici", semibold(15.0), INK);
                    p.text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "Projets .evec · glisse-en un pour l'ouvrir", FontId::proportional(13.0), MUTED);
                } else {
                    let mut open = None;
                    for p in &self.recents {
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width().min(760.0), 46.0), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(r, CornerRadius::same(10), Color32::from_white_alpha(10));
                        }
                        let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                        ui.painter().text(r.left_top() + vec2(14.0, 8.0), Align2::LEFT_TOP, name, semibold(14.0), INK);
                        ui.painter().text(r.left_top() + vec2(14.0, 27.0), Align2::LEFT_TOP, p.parent().map(|d| d.display().to_string()).unwrap_or_default(), FontId::proportional(12.0), FAINT);
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open = Some(p.clone());
                        }
                    }
                    if let Some(p) = open {
                        self.open_path(&p);
                    }
                }
                if let Some(e) = self.error.clone() {
                    ui.add_space(12.0);
                    ui.label(RichText::new(e).color(RED));
                }
            });
        });
    }

    pub(crate) fn options_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("options").exact_size(44.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().slider_width = 110.0;
                let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(7), FIELD);
                tool_icon(ui.painter(), r.shrink(7.0), self.tool, INK);
                ui.label(RichText::new(tool_name(self.tool)).font(semibold(13.0)).color(INK));
                sep(ui);
                let has = self.sel.is_some();
                match self.tool {
                    Tool::Select | Tool::GroupSel => {
                        if has {
                            for (tip, axis, to) in [("Aligner à gauche", 0, 0.0), ("Centrer horizontalement", 0, 0.5), ("Aligner à droite", 0, 1.0), ("Aligner en haut", 1, 0.0), ("Centrer verticalement", 1, 0.5), ("Aligner en bas", 1, 1.0)] {
                                if pro::icon_button(ui, 30.0, false, tip, |p, r, c| align_icon(p, r, axis, to, c)).clicked() {
                                    self.align(axis, to);
                                }
                            }
                            sep(ui);
                            for (tip, k) in [("Rotation 90°", 0), ("Symétrie horizontale", 1), ("Symétrie verticale", 2)] {
                                if pro::icon_button(ui, 30.0, false, tip, |p, r, c| quick_icon(p, r, k, c)).clicked() {
                                    match k {
                                        0 => self.rotate_sel(90.0),
                                        1 => self.flip(0),
                                        _ => self.flip(1),
                                    }
                                }
                            }
                            sep(ui);
                            if pro::ghost(ui, "Grouper").clicked() {
                                self.group();
                            }
                            if pro::ghost(ui, "Dissocier").clicked() {
                                self.ungroup();
                            }
                            if pro::ghost(ui, "Dupliquer").clicked() {
                                self.duplicate_selected();
                            }
                        } else {
                            small(ui, "Clic : sélectionner · Maj : ajouter · Alt+glisser : dupliquer · glisse dans le vide : zone");
                        }
                    }
                    Tool::Direct => small(ui, "Clique un point pour le modifier, Suppr l'efface, les flèches le décalent · Alt+clic supprime, Ctrl+clic convertit"),
                    Tool::Wand => {
                        small(ui, "Critères");
                        for (k, n) in ["Fond", "Contour", "Épaisseur", "Opacité"].iter().enumerate() {
                            ui.checkbox(&mut self.o.wand[k], *n);
                        }
                        pro::option_slider(ui, "Tolérance", &mut self.o.wand_tol, 0.0..=255.0, "", false);
                    }
                    Tool::Lasso => small(ui, "Entoure les objets à sélectionner · Maj ajoute à la sélection"),
                    Tool::Pen => small(ui, "Clic : point · glisser : courbe · clic sur le premier point : fermer · Entrée : terminer"),
                    Tool::AddPt => small(ui, "Clique sur un tracé pour ajouter un point sans changer sa forme"),
                    Tool::DelPt => small(ui, "Clique un point d'ancrage pour le supprimer"),
                    Tool::ConvPt => small(ui, "Clic sur un point courbe : sommet · glisser depuis un sommet : poignées symétriques"),
                    Tool::Curv => small(ui, "Chaque clic ajoute un point, la courbe se lisse toute seule · clic sur le premier point : fermer"),
                    Tool::Text | Tool::AreaText | Tool::PathText | Tool::VText => {
                        egui::ComboBox::from_id_salt("optfont").selected_text(FONTS[self.o.text_font]).width(140.0).show_ui(ui, |ui| {
                            for (k, f) in FONTS.iter().enumerate() {
                                ui.selectable_value(&mut self.o.text_font, k, *f);
                            }
                        });
                        opt_dv(ui, "Taille", &mut self.o.text_size, 4.0..=1000.0, " px");
                        small(ui, match self.tool {
                            Tool::AreaText => "Glisse un cadre, ou clique une forme fermée",
                            Tool::PathText => "Clique un tracé pour écrire le long",
                            Tool::VText => "Clique pour écrire de haut en bas",
                            _ => "Clique pour écrire, clique un texte pour le modifier",
                        });
                    }
                    Tool::Line | Tool::Rect | Tool::Ellipse | Tool::Heart => {
                        style_swatches(ui, &mut self.style);
                        hint(ui, "Maj : proportions fixes");
                    }
                    Tool::RRect => {
                        opt_dv(ui, "Rayon", &mut self.o.rradius, 0.0..=500.0, " px");
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::Arc => {
                        pro::option_slider(ui, "Pente", &mut self.o.arc_slope, 0.0..=1.0, "", false);
                        ui.checkbox(&mut self.o.arc_concave, "Concave");
                        ui.checkbox(&mut self.o.arc_closed, "Fermé");
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::Spiral => {
                        pro::option_slider(ui, "Tours", &mut self.o.turns, 0.5..=20.0, "", false);
                        ui.checkbox(&mut self.o.spiral_ccw, "Sens antihoraire");
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::RectGrid => {
                        opt_dv(ui, "Rangées", &mut self.o.grid_rows, 1..=100, "");
                        opt_dv(ui, "Colonnes", &mut self.o.grid_cols, 1..=100, "");
                        ui.checkbox(&mut self.o.grid_frame, "Cadre extérieur");
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::PolarGrid => {
                        opt_dv(ui, "Cercles", &mut self.o.polar_rings, 1..=60, "");
                        opt_dv(ui, "Rayons", &mut self.o.polar_spokes, 0..=72, "");
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::Arrow => {
                        pro::option_slider(ui, "Épaisseur", &mut self.o.arrow_w, 2.0..=120.0, " px", false);
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::Polygon | Tool::Star => {
                        opt_dv(ui, if self.tool == Tool::Star { "Branches" } else { "Côtés" }, &mut self.o.sides, 3..=60, "");
                        if self.tool == Tool::Star {
                            pro::option_slider(ui, "Creux", &mut self.o.star_ratio, 0.05..=0.95, "", false);
                        }
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::Flare => {
                        opt_dv(ui, "Rayons", &mut self.o.flare_rays, 0..=60, "");
                        opt_dv(ui, "Anneaux", &mut self.o.flare_rings, 0..=20, "");
                        hint(ui, "Glisse depuis le centre du halo");
                    }
                    Tool::Brush => {
                        opt_dv(ui, "Taille", &mut self.o.brush_w, 0.5..=200.0, " px");
                        let mut p = self.o.brush_profile as usize;
                        egui::ComboBox::from_id_salt("bprof").selected_text(model::PROFILES[p]).width(130.0).show_ui(ui, |ui| {
                            for (k, n) in model::PROFILES.iter().enumerate() {
                                ui.selectable_value(&mut p, k, *n);
                            }
                        });
                        self.o.brush_profile = p as u8;
                        pro::option_slider(ui, "Lissage", &mut self.o.pencil_tol, 0.5..=12.0, "", false);
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::Blob => {
                        opt_dv(ui, "Taille", &mut self.o.blob_w, 1.0..=400.0, " px");
                        ui.checkbox(&mut self.o.blob_merge, "Fusionner avec la même couleur");
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::Pencil => {
                        pro::option_slider(ui, "Lissage", &mut self.o.pencil_tol, 0.5..=12.0, "", false);
                        style_swatches(ui, &mut self.style);
                    }
                    Tool::Smooth | Tool::PathErase => {
                        opt_dv(ui, "Rayon", &mut self.o.brush_r, 2.0..=300.0, " px");
                        small(ui, "Frotte le tracé sélectionné");
                    }
                    Tool::Rotate | Tool::Reflect | Tool::Scale => {
                        small(ui, "Glisse sur le plan de travail · Alt+clic : valeurs précises · Maj : contraint");
                        sep(ui);
                        ui.checkbox(&mut self.o.scale_strokes, "Contours et effets");
                        if self.tool == Tool::Rotate {
                            opt_dv(ui, "Pas Maj", &mut self.o.snap_angle, 1.0..=90.0, "°");
                        }
                    }
                    Tool::Distort => small(ui, "Tire un coin du cadre pour déformer en perspective libre"),
                    Tool::Reshape => small(ui, "Tire une partie du tracé sélectionné : les points voisins suivent en douceur"),
                    Tool::Width => small(ui, "Glisse depuis un contour pour l'élargir ou l'affiner à cet endroit · Alt+clic : retire le point de largeur"),
                    Tool::Warp | Tool::Twirl | Tool::Pucker | Tool::Bloat => {
                        opt_dv(ui, "Rayon", &mut self.o.liquify_r, 5.0..=600.0, " px");
                        pro::option_slider(ui, "Intensité", &mut self.o.liquify_k, 0.05..=1.0, "", false);
                        hint(ui, if self.tool == Tool::Twirl { "Alt : sens inverse" } else { "Agit sur la sélection, ou sur tout ce qui est sous le pinceau" });
                    }
                    Tool::Builder => small(ui, "Sélectionne des formes qui se chevauchent · clic ou glisser : unir des zones · Alt : supprimer une zone"),
                    Tool::Bucket => {
                        small(ui, "Clique une zone fermée pour la peindre avec le fond");
                        swatch_btn(ui, &mut self.style.fill);
                    }
                    Tool::Gradient => {
                        let mut k = self.o.grad_radial as usize;
                        if pro::segmented(ui, &mut k, &["Linéaire", "Radial"]) {
                            self.o.grad_radial = k == 1;
                        }
                        small(ui, "Glisse sur l'objet sélectionné pour placer le dégradé");
                    }
                    Tool::Mesh => {
                        opt_dv(ui, "Rayon du point", &mut self.o.mesh_r, 5.0..=2000.0, " px");
                        swatch_btn(ui, &mut self.style.fill);
                        hint(ui, "Clic : point de couleur · Alt+clic : le retirer");
                    }
                    Tool::Picker => {
                        small(ui, "Copier");
                        for (k, n) in ["Fond", "Contour", "Effets", "Caractère"].iter().enumerate() {
                            ui.checkbox(&mut self.o.pick[k], *n);
                        }
                    }
                    Tool::Measure => match self.o.measure {
                        Some((a, b)) => {
                            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                            small(ui, &format!("L {:.1} px · Δx {:.1} · Δy {:.1} · angle {:.1}°", (dx * dx + dy * dy).sqrt(), dx, dy, (-dy).atan2(dx).to_degrees()));
                        }
                        None => small(ui, "Glisse pour mesurer une distance et un angle"),
                    },
                    Tool::Scissors => small(ui, "Clique sur un tracé pour le couper à cet endroit"),
                    Tool::Knife => small(ui, "Trace un trait à main levée à travers les formes pour les découper"),
                    Tool::Eraser => {
                        opt_dv(ui, "Taille", &mut self.o.erase_r, 1.0..=300.0, " px");
                        small(ui, "Efface sur la sélection, ou sur tout s'il n'y a rien de sélectionné");
                    }
                    Tool::Hand | Tool::Zoom | Tool::RotView => {
                        if pro::secondary(ui, "Ajuster").clicked() {
                            self.fit_view(false);
                        }
                        if pro::secondary(ui, "Tout").clicked() {
                            self.fit_view(true);
                        }
                        if pro::secondary(ui, "100 %").clicked() {
                            self.zoom_by(1.0 / self.zoom);
                        }
                        if self.tool == Tool::RotView {
                            opt_dv(ui, "Angle", &mut self.rot, -180.0..=180.0, "°");
                            if pro::secondary(ui, "Réinitialiser").clicked() {
                                self.rot = 0.0;
                            }
                        }
                    }
                    Tool::Board => {
                        if pro::secondary(ui, "Nouveau").clicked() {
                            self.add_board(self.doc.width, self.doc.height);
                        }
                        if pro::secondary(ui, "Réorganiser").clicked() {
                            self.arrange_boards();
                        }
                        small(ui, "Glisse pour créer · clique pour activer · déplace un plan secondaire · Suppr");
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::secondary(ui, "?  Tutoriel").on_hover_text("F1").clicked() {
                        self.help.learn(tool_name(self.tool));
                    }
                });
            });
        });
    }

    pub(crate) fn status_bar(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let s = |t: String| RichText::new(t).size(12.0).color(MUTED);
                ui.label(s(format!("{:.0} %", self.zoom * 100.0)));
                if self.rot != 0.0 {
                    ui.label(s(format!("vue {:.0}°", self.rot)));
                }
                ui.label(s(format!("{} objet(s)", self.doc.items.len())));
                let n = self.selection().len();
                if n > 0 {
                    ui.label(s(format!("{n} sélectionné(s)")));
                }
                if let Some(l) = self.doc.layers.get(self.active_layer) {
                    ui.label(s(format!("Calque : {}", l.name)));
                }
                if let Some(p) = self.cursor {
                    ui.label(s(format!("x {:.0}  y {:.0}", p[0], p[1])));
                }
                for (flag, label) in [(self.smart, "Repères commentés"), (self.snap, "Magnétisme"), (self.outline_view, "Mode tracés")] {
                    if flag {
                        ui.label(RichText::new(label).size(11.5).color(BLUE));
                    }
                }
                if let Some(e) = self.error.clone() {
                    ui.label(RichText::new(e).size(12.0).color(RED));
                    if ui.small_button("×").clicked() {
                        self.error = None;
                    }
                } else if let Some(m) = self.info.clone() {
                    ui.label(RichText::new(m).size(12.0).color(INK));
                    if ui.small_button("×").clicked() {
                        self.info = None;
                    }
                }
            });
        });
    }

    pub(crate) fn tool_rail(&mut self, root: &mut Ui) {
        let h = root.ctx().content_rect().height();
        let cols = if h - 150.0 < GROUPS.len() as f32 * 35.0 + 50.0 { 2 } else { 1 };
        egui::Panel::left("tools").resizable(false).exact_size(if cols == 2 { 86.0 } else { 52.0 }).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(7, 10))).show(root, |ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            let mut cur = self.tool;
            let breaks = [3usize, 5, 8, 9, 12, 13];
            let mut k = 0;
            while k < GROUPS.len() {
                ui.horizontal(|ui| {
                    for _ in 0..cols {
                        if k >= GROUPS.len() {
                            break;
                        }
                        pro::tool_group(ui, 34.0, &mut cur, GROUPS[k], |p, r, c, t| tool_icon(p, r, t, c));
                        k += 1;
                    }
                });
                if cols == 1 && breaks.contains(&k) {
                    let (s, _) = ui.allocate_exact_size(vec2(36.0, 5.0), Sense::hover());
                    ui.painter().line_segment([s.left_center() + vec2(6.0, 0.0), s.right_center() - vec2(6.0, 0.0)], Stroke::new(1.0, LINE));
                }
            }
            if cur != self.tool {
                self.set_tool(cur);
            }
            ui.add_space(10.0);
            let (rect, resp) = ui.allocate_exact_size(vec2(38.0, 42.0), Sense::click());
            let back = Rect::from_min_size(rect.min + vec2(14.0, 16.0), Vec2::splat(20.0));
            let front = Rect::from_min_size(rect.min + vec2(4.0, 6.0), Vec2::splat(20.0));
            ui.painter().rect(back, CornerRadius::same(5), PANEL, Stroke::new(4.0, self.style.stroke.unwrap_or(FAINT)), StrokeKind::Inside);
            ui.painter().rect(front, CornerRadius::same(5), self.style.fill.unwrap_or(PANEL), Stroke::new(1.0, INK), StrokeKind::Outside);
            if self.style.fill.is_none() {
                ui.painter().line_segment([front.left_bottom(), front.right_top()], Stroke::new(1.5, RED));
            }
            if resp.on_hover_text("Fond et contour · clic : intervertir (Maj+X) · D : par défaut · / : sans fond").clicked() {
                std::mem::swap(&mut self.style.fill, &mut self.style.stroke);
            }
        });
    }

    pub(crate) fn dock(&mut self, root: &mut Ui) {
        egui::Panel::right("dock").resizable(false).exact_size(300.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                pro::segmented(ui, &mut self.dock_tab, &["Propriétés", "Calques", "Bibliothèques"]);
            });
            ui.add_space(8.0);
            let line = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover()).0;
            ui.painter().rect_filled(line, CornerRadius::ZERO, LINE);
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                match self.dock_tab {
                    1 => {
                        pro::dock_panel(ui, "layers", "Calques", |ui| self.layers_panel(ui));
                        pro::dock_panel(ui, "boards", "Plans de travail", |ui| self.boards_panel(ui));
                    }
                    2 => {
                        pro::dock_panel(ui, "swatches", "Nuancier", |ui| self.swatches_panel(ui));
                        pro::dock_panel(ui, "gstyles", "Styles graphiques", |ui| self.styles_panel(ui));
                        pro::dock_panel(ui, "symbols", "Symboles", |ui| self.symbols_panel(ui));
                        pro::dock_panel(ui, "presets", "Formes prêtes", |ui| self.shapes_panel(ui));
                    }
                    _ => self.properties(ui),
                }
            });
        });
    }

    fn properties(&mut self, ui: &mut Ui) {
        let all = self.selection();
        let Some(i) = self.sel.filter(|i| *i < self.doc.items.len()) else {
            pro::dock_panel(ui, "document", "Document", |ui| self.document_panel(ui));
            pro::dock_panel(ui, "style", "Style des nouveaux objets", |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                color_toggle(ui, "Fond", &mut self.style.fill);
                color_toggle(ui, "Contour", &mut self.style.stroke);
                slider(ui, "Épaisseur", &mut self.style.stroke_w, 0.0..=50.0, " px");
            });
            pro::dock_panel(ui, "gridp", "Grille et repères", |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.checkbox(&mut self.show_grid, "Afficher la grille");
                ui.checkbox(&mut self.snap, "Magnétisme de la grille");
                ui.horizontal(|ui| {
                    row_label(ui, "Pas", 60.0);
                    dv(ui, &mut self.grid, 2.0..=500.0, " px");
                });
                ui.checkbox(&mut self.rulers, "Règles");
                ui.checkbox(&mut self.smart, "Repères commentés");
                ui.checkbox(&mut self.show_guides, "Afficher les repères");
                ui.checkbox(&mut self.thirds, "Centre et tiers");
                ui.horizontal(|ui| {
                    row_label(ui, "Déplacement clavier", 130.0);
                    dv(ui, &mut self.o.nudge, 0.1..=100.0, " px");
                });
                hint(ui, "Tire depuis une règle pour poser un repère, ramène-le dans la règle pour le supprimer.");
            });
            return;
        };
        let n = all.len();
        pro::dock_panel(ui, "transform", if n > 1 { "Transformation (sélection)" } else { "Transformation" }, |ui| self.transform_panel(ui, i));
        pro::dock_panel(ui, "align", "Aligner et répartir", |ui| self.align_panel(ui, n));
        if n >= 2 {
            pro::dock_panel(ui, "pathfinder", "Pathfinder", |ui| self.pathfinder_panel(ui));
        }
        let Some(before) = self.doc.items.get(i).cloned().filter(|_| self.sel == Some(i)) else { return };
        let all = self.selection();
        let mut it = before.clone();
        pro::dock_panel(ui, "appearance", "Apparence", |ui| appearance(ui, &mut it));
        if it.stroke.is_some() {
            pro::dock_panel(ui, "stroke", "Contour", |ui| stroke_panel(ui, &mut it));
        }
        if it.fill.is_some() && !matches!(it.geom, Geom::Image { .. } | Geom::Text { .. }) {
            pro::dock_panel(ui, "gradient", "Dégradé", |ui| gradient_panel(ui, &mut it));
        }
        pro::dock_panel(ui, "effects", "Effets", |ui| effects_panel(ui, &mut it));
        if !matches!(it.geom, Geom::Ellipse { .. }) {
            let title = match it.geom {
                Geom::Rect { .. } => "Rectangle",
                Geom::Text { .. } => "Caractère et paragraphe",
                Geom::Path { .. } => "Tracé",
                Geom::Image { .. } => "Image",
                Geom::Ellipse { .. } => "Ellipse",
            };
            let mut trace = false;
            pro::dock_panel(ui, "shape", title, |ui| trace = shape_props(ui, &mut it));
            if trace {
                self.open_trace();
            }
        }
        pro::dock_panel(ui, "quick", "Actions rapides", |ui| self.quick_actions(ui, n));
        if it != before && self.doc.items.get(i) == Some(&before) {
            let diff = |a: &Item, b: &Item| -> Vec<&'static str> {
                let mut v = Vec::new();
                macro_rules! f {
                    ($($x:ident),*) => {$(if a.$x != b.$x { v.push(stringify!($x)); })*};
                }
                f!(fill, stroke, stroke_w, opacity, fill2, grad_angle, dash, shadow, blur, blend, cap, join, gap, radial, stops, grad_vec, profile, widths, head_start, head_end, head_scale, dash2, gap2, miter, stroke_align, sh_x, sh_y, sh_blur, sh_color, glow_out, glow_out_c, glow_in, glow_in_c, feather, rough, rough_freq, grain, mesh, arrow_end);
                v
            };
            let changed = diff(&before, &it);
            for &k in &all {
                if k == i {
                    continue;
                }
                let t = &mut self.doc.items[k];
                macro_rules! g {
                    ($($x:ident),*) => {$(if changed.contains(&stringify!($x)) { t.$x = it.$x.clone(); })*};
                }
                g!(fill, stroke, stroke_w, opacity, fill2, grad_angle, dash, shadow, blur, blend, cap, join, gap, radial, stops, grad_vec, profile, head_start, head_end, head_scale, dash2, gap2, miter, stroke_align, sh_x, sh_y, sh_blur, sh_color, glow_out, glow_out_c, glow_in, glow_in_c, feather, rough, rough_freq, grain, arrow_end);
            }
            self.doc.items[i] = it;
        }
    }

    fn document_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 6.0;
        let k = self.active_board.min(self.doc.boards.len());
        let b = self.doc.all_boards()[k].clone();
        small(ui, &b.name);
        let (mut w, mut h) = (b.w, b.h);
        let ch = ui.horizontal(|ui| {
            row_label(ui, "L", 14.0);
            let a = dv(ui, &mut w, 1.0..=20000.0, " px");
            row_label(ui, "H", 14.0);
            let c = dv(ui, &mut h, 1.0..=20000.0, " px");
            a || c
        }).inner;
        if ch {
            if k == 0 {
                self.doc.width = w;
                self.doc.height = h;
            } else {
                self.doc.boards[k - 1].w = w;
                self.doc.boards[k - 1].h = h;
            }
        }
        ui.horizontal_wrapped(|ui| {
            for (n, _, pw, ph) in PRESETS.iter().take(8) {
                if pro::chip(ui, n).clicked() {
                    self.checkpoint();
                    if k == 0 {
                        self.doc.width = *pw;
                        self.doc.height = *ph;
                    } else {
                        self.doc.boards[k - 1].w = *pw;
                        self.doc.boards[k - 1].h = *ph;
                    }
                }
            }
        });
        let mut bg = self.doc.background.map(c32);
        if color_toggle(ui, "Fond du document", &mut bg) {
            self.doc.background = bg.map(arr);
        }
        ui.horizontal(|ui| {
            if pro::secondary(ui, "Ajuster au contenu").clicked() {
                let all: Vec<usize> = (0..self.doc.items.len()).filter(|i| self.doc.shown(*i)).collect();
                self.fit_board_to(self.doc.bounds_of(&all));
            }
            if pro::secondary(ui, "Exporter…").clicked() {
                self.export = Some(crate::ExportOpts { format: 0, scale: 2.0, transparent: false, quality: 90, scope: 0, minify: false });
            }
        });
    }

    fn transform_panel(&mut self, ui: &mut Ui, i: usize) {
        ui.spacing_mut().item_spacing.y = 6.0;
        let all = self.selection();
        if all.len() == 1 {
            ui.add(egui::TextEdit::singleline(&mut self.doc.items[i].name).desired_width(f32::INFINITY));
        }
        let b = self.doc.bounds_of(&all).unwrap();
        let rp = self.reference(&b);
        ui.horizontal(|ui| {
            let (grid, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
            ui.painter().rect_stroke(grid.shrink(6.0), CornerRadius::ZERO, Stroke::new(1.0, LINE), StrokeKind::Middle);
            for k in 0..9 {
                let c = grid.min + vec2(6.0 + (k % 3) as f32 * 16.0, 6.0 + (k / 3) as f32 * 16.0);
                let r = Rect::from_center_size(c, Vec2::splat(8.0));
                let resp = ui.interact(r.expand(3.0), ui.make_persistent_id(("refpt", k)), Sense::click()).on_hover_text("Point de référence");
                ui.painter().rect(r, CornerRadius::same(2), if self.ref_pt == k { BLUE } else { PANEL }, Stroke::new(1.0, if self.ref_pt == k { BLUE } else { MUTED }), StrokeKind::Inside);
                if resp.clicked() {
                    self.ref_pt = k;
                }
            }
            ui.add_space(6.0);
            ui.vertical(|ui| {
                let (mut x, mut y, mut w, mut h) = (rp[0], rp[1], b.size()[0], b.size()[1]);
                let (ow, oh) = (w, h);
                let mut moved = false;
                let mut sized = false;
                ui.horizontal(|ui| {
                    row_label(ui, "X", 12.0);
                    moved |= dv(ui, &mut x, -100000.0..=100000.0, "");
                    row_label(ui, "Y", 12.0);
                    moved |= dv(ui, &mut y, -100000.0..=100000.0, "");
                });
                ui.horizontal(|ui| {
                    row_label(ui, "L", 12.0);
                    sized |= dv(ui, &mut w, 0.1..=100000.0, "");
                    row_label(ui, "H", 12.0);
                    sized |= dv(ui, &mut h, 0.1..=100000.0, "");
                    if ui.selectable_label(self.lock_ratio, "🔗").on_hover_text("Conserver les proportions").clicked() {
                        self.lock_ratio = !self.lock_ratio;
                    }
                });
                if moved {
                    for &k in &all {
                        self.doc.items[k].translate([x - rp[0], y - rp[1]]);
                    }
                }
                if sized {
                    if self.lock_ratio {
                        if (w - ow).abs() > 1e-4 {
                            h = oh * w / ow.max(1e-3);
                        } else {
                            w = ow * h / oh.max(1e-3);
                        }
                    }
                    let (sx, sy) = (w / ow.max(1e-3), h / oh.max(1e-3));
                    for &k in &all {
                        let st = self.o.scale_strokes;
                        let it = &mut self.doc.items[k];
                        it.map(|p| [rp[0] + (p[0] - rp[0]) * sx, rp[1] + (p[1] - rp[1]) * sy], sx, sy);
                        if st {
                            it.stroke_w *= (sx * sy).abs().sqrt();
                        }
                    }
                }
            });
        });
        ui.horizontal(|ui| {
            row_label(ui, "Rotation", 64.0);
            for (label, deg) in [("−90°", -90.0), ("−15°", -15.0), ("+15°", 15.0), ("+90°", 90.0)] {
                if pro::ghost(ui, label).clicked() {
                    self.rotate_sel(deg);
                }
            }
        });
        ui.horizontal(|ui| {
            if pro::ghost(ui, "↔ Miroir").clicked() {
                self.flip(0);
            }
            if pro::ghost(ui, "↕ Miroir").clicked() {
                self.flip(1);
            }
            if pro::ghost(ui, "Inclinaison…").clicked() {
                self.open_dlg(Dlg::Shear { deg: 15.0, vertical: false });
            }
        });
        ui.checkbox(&mut self.o.scale_strokes, "Mettre à l'échelle contours et effets");
    }

    fn align_panel(&mut self, ui: &mut Ui, n: usize) {
        ui.spacing_mut().item_spacing.y = 6.0;
        ui.horizontal(|ui| {
            row_label(ui, "Aligner sur", 76.0);
            pro::segmented(ui, &mut self.align_to, &["Sélection", "Plan", "Clé"]);
        });
        if self.align_to == 2 {
            hint(ui, "Objet clé : le premier sélectionné (contour bleu épais).");
        }
        ui.horizontal(|ui| {
            for (tip, axis, to) in [("À gauche", 0, 0.0), ("Centres horizontaux", 0, 0.5), ("À droite", 0, 1.0), ("En haut", 1, 0.0), ("Centres verticaux", 1, 0.5), ("En bas", 1, 1.0)] {
                if pro::icon_button(ui, 30.0, false, tip, |p, r, c| align_icon(p, r, axis, to, c)).clicked() {
                    self.align(axis, to);
                }
            }
        });
        if n >= 3 {
            small(ui, "Répartir les objets");
            ui.horizontal(|ui| {
                for (tip, axis, at) in [("Bords gauches", 0, 0.0), ("Centres horizontaux", 0, 0.5), ("Bords droits", 0, 1.0), ("Bords hauts", 1, 0.0), ("Centres verticaux", 1, 0.5), ("Bords bas", 1, 1.0)] {
                    if pro::icon_button(ui, 30.0, false, tip, |p, r, c| dist_icon(p, r, axis, at, c)).clicked() {
                        self.distribute(axis, at);
                    }
                }
            });
        }
        if n >= 2 {
            small(ui, "Répartir l'espacement");
            let id = ui.make_persistent_id("gapv");
            let mut gap: f32 = ui.ctx().data(|d| d.get_temp(id).unwrap_or(20.0));
            let mut fixed: bool = ui.ctx().data(|d| d.get_temp(id.with("f")).unwrap_or(false));
            ui.horizontal(|ui| {
                ui.checkbox(&mut fixed, "Écart");
                ui.add_enabled(fixed, egui::DragValue::new(&mut gap).range(-1000.0..=5000.0).suffix(" px"));
                if pro::icon_button(ui, 30.0, false, "Espacement horizontal", |p, r, c| dist_icon(p, r, 0, 0.5, c)).clicked() {
                    self.distribute_spacing(0, fixed.then_some(gap));
                }
                if pro::icon_button(ui, 30.0, false, "Espacement vertical", |p, r, c| dist_icon(p, r, 1, 0.5, c)).clicked() {
                    self.distribute_spacing(1, fixed.then_some(gap));
                }
            });
            ui.ctx().data_mut(|d| {
                d.insert_temp(id, gap);
                d.insert_temp(id.with("f"), fixed);
            });
        }
    }

    fn pathfinder_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = vec2(4.0, 6.0);
        small(ui, "Modes de forme");
        ui.horizontal(|ui| {
            for k in 0..4 {
                if pro::icon_button(ui, 34.0, false, geom::PATHFINDER[k], |p, r, c| pf_icon(p, r, k, c)).clicked() {
                    self.pathfinder(k);
                }
            }
        });
        small(ui, "Pathfinders");
        ui.horizontal(|ui| {
            for k in 4..10 {
                if pro::icon_button(ui, 34.0, false, geom::PATHFINDER[k], |p, r, c| pf_icon(p, r, k, c)).clicked() {
                    self.pathfinder(k);
                }
            }
        });
    }

    fn quick_actions(&mut self, ui: &mut Ui, n: usize) {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        ui.horizontal_wrapped(|ui| {
            if n >= 2 && pro::chip(ui, "Grouper").clicked() {
                self.group();
            }
            if pro::chip(ui, "Dissocier").clicked() {
                self.ungroup();
            }
            if n >= 2 && pro::chip(ui, "Masque d'écrêtage").clicked() {
                self.make_clip(false);
            }
            if n >= 2 && pro::chip(ui, "Masque d'opacité").clicked() {
                self.make_clip(true);
            }
            if pro::chip(ui, "Libérer le masque").clicked() {
                self.release_clip();
            }
            if n >= 2 && pro::chip(ui, "Tracé composé").clicked() {
                self.make_compound();
            }
            if pro::chip(ui, "Décalage…").clicked() {
                self.open_dlg(Dlg::Offset { d: 10.0 });
            }
            if pro::chip(ui, "Vectoriser le contour").clicked() {
                self.outline_selected();
            }
            if pro::chip(ui, "Simplifier…").clicked() {
                self.open_dlg(Dlg::Simplify { tol: 2.0, smooth: true });
            }
            if n >= 2 && pro::chip(ui, "Fusion…").clicked() {
                self.open_dlg(Dlg::Blend { steps: 5 });
            }
            if pro::chip(ui, "Recolorer…").clicked() {
                self.open_recolor();
            }
            if pro::chip(ui, "Nouveau symbole").clicked() {
                self.new_symbol();
            }
            if self.sel.is_some_and(|i| self.doc.items[i].sym != 0) && pro::chip(ui, "Rompre le lien").clicked() {
                self.checkpoint();
                for i in self.selection() {
                    self.doc.items[i].sym = 0;
                }
            }
            if pro::chip(ui, "Verrouiller").clicked() {
                self.lock_selected();
            }
            if pro::chip(ui, "Masquer").clicked() {
                self.hide_selected();
            }
        });
    }

    fn layers_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 3.0;
        ui.horizontal(|ui| {
            if pro::ghost(ui, "+ Calque").clicked() {
                self.add_layer();
            }
            if pro::ghost(ui, "Supprimer").clicked() {
                let k = self.active_layer;
                self.delete_layer(k);
            }
            if pro::ghost(ui, "↑").on_hover_text("Monter le calque").clicked() {
                let k = self.active_layer;
                self.move_layer(k, true);
            }
            if pro::ghost(ui, "↓").on_hover_text("Descendre le calque").clicked() {
                let k = self.active_layer;
                self.move_layer(k, false);
            }
        });
        if self.sel.is_some() && pro::ghost(ui, "Placer la sélection sur le calque actif").clicked() {
            let k = self.active_layer;
            self.sel_to_layer(k);
        }
        if let Some(l) = self.doc.layers.get_mut(self.active_layer) {
            ui.add(egui::TextEdit::singleline(&mut l.name).desired_width(f32::INFINITY));
        }
        ui.add_space(4.0);
        let mut select = None;
        let mut select_add = false;
        let mut item_toggle = None;
        let mut item_lock = None;
        let mut layer_vis = None;
        let mut layer_lock = None;
        let mut layer_pick = None;
        let all = self.selection();
        for li in (0..self.doc.layers.len()).rev() {
            let l = self.doc.layers[li].clone();
            let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
            let p = ui.painter();
            p.rect_filled(row, CornerRadius::same(7), if li == self.active_layer { Color32::from_white_alpha(14) } else { Color32::from_white_alpha(5) });
            p.rect_filled(Rect::from_min_size(row.min + vec2(0.0, 4.0), vec2(3.0, 22.0)), CornerRadius::same(1), Color32::from_rgb(l.color[0], l.color[1], l.color[2]));
            let eye = Rect::from_center_size(row.left_center() + vec2(18.0, 0.0), Vec2::splat(18.0));
            eye_icon(p, eye, l.visible);
            let count = self.doc.items.iter().filter(|it| it.layer == li).count();
            p.text(row.left_center() + vec2(36.0, 0.0), Align2::LEFT_CENTER, &l.name, semibold(12.5), INK_STRONG);
            p.text(row.right_center() - vec2(34.0, 0.0), Align2::RIGHT_CENTER, count.to_string(), FontId::proportional(11.5), FAINT);
            let lock = Rect::from_center_size(row.right_center() - vec2(14.0, 0.0), Vec2::splat(16.0));
            lock_icon(p, lock, l.locked, if l.locked { INK } else { FAINT });
            if ui.interact(eye.expand(4.0), ui.make_persistent_id(("leye", li)), Sense::click()).clicked() {
                layer_vis = Some(li);
            } else if ui.interact(lock.expand(4.0), ui.make_persistent_id(("llock", li)), Sense::click()).clicked() {
                layer_lock = Some(li);
            } else if resp.clicked() {
                layer_pick = Some(li);
            }
            for i in (0..self.doc.items.len()).rev().filter(|i| self.doc.items[*i].layer == li) {
                let on = all.contains(&i);
                let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
                let p = ui.painter();
                if on {
                    p.rect_filled(row, CornerRadius::same(7), BLUE.gamma_multiply(0.24));
                } else if resp.hovered() {
                    p.rect_filled(row, CornerRadius::same(7), Color32::from_white_alpha(8));
                }
                let it = &self.doc.items[i];
                let eye = Rect::from_center_size(row.left_center() + vec2(26.0, 0.0), Vec2::splat(16.0));
                eye_icon(p, eye, it.visible);
                let sw = Rect::from_center_size(row.left_center() + vec2(48.0, 0.0), Vec2::splat(16.0));
                p.rect(sw, CornerRadius::same(4), it.fill.map(c32).unwrap_or(Color32::TRANSPARENT), Stroke::new(1.5, it.stroke.map(c32).unwrap_or(LINE)), StrokeKind::Inside);
                let mut name = it.name.clone();
                if it.group != 0 {
                    name = format!("{name}  ·  G{}", it.group);
                }
                if it.clip {
                    name = format!("✂ {name}");
                }
                if it.sym != 0 {
                    name = format!("◆ {name}");
                }
                let g = p.layout(name, if on { semibold(12.5) } else { FontId::proportional(12.5) }, if on { INK_STRONG } else { INK }, row.width() - 100.0);
                p.galley(pos2(row.left() + 62.0, row.center().y - g.size().y / 2.0), g, INK);
                let lock = Rect::from_center_size(row.right_center() - vec2(14.0, 0.0), Vec2::splat(16.0));
                if it.locked || resp.hovered() {
                    lock_icon(p, lock, it.locked, if it.locked { INK } else { FAINT });
                }
                if ui.interact(lock.expand(3.0), ui.make_persistent_id(("vlock", i)), Sense::click()).clicked() {
                    item_lock = Some(i);
                } else if ui.interact(eye.expand(4.0), ui.make_persistent_id(("veye", i)), Sense::click()).clicked() {
                    item_toggle = Some(i);
                } else if resp.clicked() {
                    select = Some(i);
                    select_add = ui.input(|i| i.modifiers.shift || i.modifiers.command);
                }
            }
        }
        if let Some(li) = layer_vis {
            self.doc.layers[li].visible = !self.doc.layers[li].visible;
        }
        if let Some(li) = layer_lock {
            self.doc.layers[li].locked = !self.doc.layers[li].locked;
        }
        if let Some(li) = layer_pick {
            self.active_layer = li;
        }
        if let Some(i) = item_toggle {
            self.doc.items[i].visible = !self.doc.items[i].visible;
        }
        if let Some(i) = item_lock {
            self.doc.items[i].locked = !self.doc.items[i].locked;
            if self.doc.items[i].locked {
                self.extra.retain(|k| *k != i);
                if self.sel == Some(i) {
                    self.sel = self.extra.pop();
                }
            }
        }
        if let Some(i) = select {
            self.finish_pen();
            if select_add {
                if self.sel.is_none() {
                    self.sel = Some(i);
                } else if self.sel != Some(i) && !self.extra.contains(&i) {
                    self.extra.push(i);
                }
            } else {
                self.set_selection(vec![i]);
            }
            self.active_layer = self.doc.items[i].layer;
            if !matches!(self.tool, Tool::Select | Tool::Direct | Tool::GroupSel) {
                self.tool = Tool::Select;
            }
        }
    }

    fn boards_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 4.0;
        let boards = self.doc.all_boards();
        let mut pick = None;
        for (k, b) in boards.iter().enumerate() {
            let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
            let p = ui.painter();
            if k == self.active_board {
                p.rect_filled(row, CornerRadius::same(7), BLUE.gamma_multiply(0.24));
            } else if resp.hovered() {
                p.rect_filled(row, CornerRadius::same(7), Color32::from_white_alpha(8));
            }
            p.text(row.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, format!("{}", k + 1), FontId::proportional(12.0), FAINT);
            p.text(row.left_center() + vec2(30.0, 0.0), Align2::LEFT_CENTER, &b.name, FontId::proportional(12.5), INK);
            p.text(row.right_center() - vec2(10.0, 0.0), Align2::RIGHT_CENTER, format!("{:.0} × {:.0}", b.w, b.h), FontId::proportional(11.5), MUTED);
            if resp.clicked() {
                pick = Some(k);
            }
        }
        if let Some(k) = pick {
            self.active_board = k;
            self.fit_view(false);
        }
        let k = self.active_board;
        if k > 0 && let Some(b) = self.doc.boards.get_mut(k - 1) {
            ui.add(egui::TextEdit::singleline(&mut b.name).desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                row_label(ui, "X", 12.0);
                dv(ui, &mut b.x, -100000.0..=100000.0, "");
                row_label(ui, "Y", 12.0);
                dv(ui, &mut b.y, -100000.0..=100000.0, "");
            });
        }
        ui.horizontal_wrapped(|ui| {
            if pro::chip(ui, "+ Nouveau").clicked() {
                self.add_board(self.doc.width, self.doc.height);
            }
            if pro::chip(ui, "Dupliquer").clicked() {
                let b = self.doc.all_boards()[k].clone();
                self.add_board(b.w, b.h);
            }
            if k > 0 && pro::chip(ui, "Supprimer").clicked() {
                self.delete_board(k);
            }
            if pro::chip(ui, "Réorganiser").clicked() {
                self.arrange_boards();
            }
            if pro::chip(ui, "Tout voir").clicked() {
                self.fit_view(true);
            }
        });
    }

    fn apply_color(&mut self, c: [u8; 4], stroke: bool) {
        let all = self.selection();
        if all.is_empty() {
            if stroke {
                self.style.stroke = Some(c32(c));
            } else {
                self.style.fill = Some(c32(c));
            }
            return;
        }
        self.checkpoint();
        for i in all {
            let it = &mut self.doc.items[i];
            if stroke {
                it.stroke = Some(c);
            } else {
                it.fill = Some(c);
                it.fill2 = None;
                it.stops.clear();
            }
        }
    }

    fn swatch_row(&mut self, ui: &mut Ui, cols: &[[u8; 4]], id: &str) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (k, c) in cols.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
                checker_rect(ui.painter(), r);
                ui.painter().rect(r, CornerRadius::same(6), c32(*c), Stroke::new(1.0, LINE), StrokeKind::Inside);
                let resp = ui.interact(r, ui.make_persistent_id((id, k)), Sense::click()).union(resp).on_hover_text(format!("#{:02X}{:02X}{:02X} · clic : fond · clic droit : contour", c[0], c[1], c[2]));
                if resp.clicked() {
                    self.apply_color(*c, false);
                }
                if resp.secondary_clicked() {
                    self.apply_color(*c, true);
                }
            }
        });
    }

    fn swatches_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 8.0;
        egui::ComboBox::from_id_salt("library").selected_text(LIBRARIES[self.library].0).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
            for (k, (n, _)) in LIBRARIES.iter().enumerate() {
                ui.selectable_value(&mut self.library, k, *n);
            }
        });
        let lib: Vec<[u8; 4]> = LIBRARIES[self.library].1.iter().map(|c| [c[0], c[1], c[2], 255]).collect();
        self.swatch_row(ui, &lib, "lib");
        small(ui, "Gris");
        let grays: Vec<[u8; 4]> = (0..9).map(|k| {
            let v = (k * 255 / 8) as u8;
            [v, v, v, 255]
        }).collect();
        self.swatch_row(ui, &grays, "gray");
        small(ui, "Nuances du document");
        let docs = self.doc.swatches.clone();
        if docs.is_empty() {
            hint(ui, "Ajoute ici les couleurs de ton projet.");
        } else {
            self.swatch_row(ui, &docs, "docsw");
        }
        ui.horizontal_wrapped(|ui| {
            if pro::chip(ui, "+ Couleur du fond").clicked() {
                let c = self.sel.and_then(|i| self.doc.items[i].fill).or(self.style.fill.map(arr));
                if let Some(c) = c && !self.doc.swatches.contains(&c) {
                    self.checkpoint();
                    self.doc.swatches.push(c);
                }
            }
            if pro::chip(ui, "Ajouter la bibliothèque").clicked() {
                self.checkpoint();
                for c in lib {
                    if !self.doc.swatches.contains(&c) {
                        self.doc.swatches.push(c);
                    }
                }
            }
            if pro::chip(ui, "Couleurs utilisées").clicked() {
                self.checkpoint();
                let used: Vec<[u8; 4]> = self.doc.items.iter().flat_map(|it| [it.fill, it.stroke]).flatten().collect();
                for c in used {
                    if !self.doc.swatches.contains(&c) {
                        self.doc.swatches.push(c);
                    }
                }
            }
            if !self.doc.swatches.is_empty() && pro::chip(ui, "Vider").clicked() {
                self.checkpoint();
                self.doc.swatches.clear();
            }
        });
    }

    fn styles_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        let mut all: Vec<(String, Item)> = graphic_styles().into_iter().map(|(n, i)| (n.to_string(), i)).collect();
        all.extend(self.doc.styles.iter().map(|s| (s.name.clone(), s.item.clone())));
        let mut apply = None;
        ui.horizontal_wrapped(|ui| {
            for (k, (n, it)) in all.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(60.0, 60.0), Sense::click());
                let p = ui.painter();
                p.rect_filled(r, CornerRadius::same(8), if resp.hovered() { FIELD } else { Color32::from_rgb(0x26, 0x26, 0x2b) });
                let s = r.shrink(14.0);
                if it.glow_out > 0.0 {
                    p.rect_filled(s.expand(4.0), CornerRadius::same(8), c32(it.glow_out_c).gamma_multiply(0.35));
                }
                if it.shadow {
                    p.rect_filled(s.translate(vec2(0.0, 3.0)), CornerRadius::same(6), Color32::from_black_alpha(120));
                }
                let fill = it.stops.first().map(|s| s.c).or(it.fill).map(c32).unwrap_or(Color32::TRANSPARENT);
                p.rect(s, CornerRadius::same(6), fill, it.stroke.map_or(Stroke::NONE, |c| Stroke::new(it.stroke_w.min(4.0), c32(c))), StrokeKind::Inside);
                if let Some(l) = it.stops.last() {
                    p.rect_filled(Rect::from_min_max(pos2(s.left(), s.center().y), s.max), CornerRadius::same(6), c32(l.c).gamma_multiply(0.8));
                }
                if resp.on_hover_text(format!("{n} · clic : appliquer")).clicked() {
                    apply = Some(k);
                }
            }
        });
        if let Some(k) = apply {
            let it = all[k].1.clone();
            self.apply_style(&it);
        }
        ui.horizontal(|ui| {
            if self.sel.is_some() && pro::secondary(ui, "Nouveau style").clicked() {
                let it = self.doc.items[self.sel.unwrap()].clone();
                self.checkpoint();
                let n = self.doc.styles.len() + 1;
                self.doc.styles.push(model::Named { name: format!("Style {n}"), item: it });
            }
            if !self.doc.styles.is_empty() && pro::ghost(ui, "Retirer le dernier").clicked() {
                self.checkpoint();
                self.doc.styles.pop();
            }
        });
        hint(ui, "Sans sélection, le style devient celui des nouveaux objets.");
    }

    fn symbols_panel(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 6.0;
        let mut place = None;
        let mut redefine = None;
        let mut delete = None;
        for (k, s) in self.doc.symbols.iter().enumerate() {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(8), Color32::from_rgb(0x26, 0x26, 0x2b));
                draw_items(ui.painter(), r.shrink(5.0), &s.items);
                ui.vertical(|ui| {
                    let count = self.doc.items.iter().filter(|i| i.sym == k as u32 + 1).count();
                    ui.label(RichText::new(&s.name).size(12.5).color(INK));
                    ui.horizontal(|ui| {
                        if pro::ghost(ui, "Placer").clicked() {
                            place = Some(k);
                        }
                        if pro::ghost(ui, "Redéfinir").on_hover_text("Remplace le symbole par la sélection et met à jour ses occurrences").clicked() {
                            redefine = Some(k);
                        }
                        if pro::ghost(ui, "×").on_hover_text(format!("Supprimer ({count} occurrence(s) conservée(s))")).clicked() {
                            delete = Some(k);
                        }
                    });
                });
            });
        }
        if let Some(k) = place {
            self.place_symbol(k);
        }
        if let Some(k) = redefine {
            if self.sel.is_some() {
                self.redefine_symbol(k);
            } else {
                self.info = Some("Redéfinir : sélectionne la nouvelle illustration".into());
            }
        }
        if let Some(k) = delete {
            self.checkpoint();
            self.doc.symbols.remove(k);
            let id = k as u32 + 1;
            for it in &mut self.doc.items {
                if it.sym == id {
                    it.sym = 0;
                } else if it.sym > id {
                    it.sym -= 1;
                }
            }
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.symbol_name).hint_text("Nom du symbole").desired_width(150.0));
            if pro::secondary(ui, "Créer").on_hover_text("Fait de la sélection un symbole réutilisable").clicked() {
                self.new_symbol();
            }
        });
    }

    fn shapes_panel(&mut self, ui: &mut Ui) {
        let mut insert = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for (k, name) in model::SHAPES.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::click());
                ui.painter().rect_filled(r, CornerRadius::same(8), if resp.hovered() { FIELD } else { Color32::from_rgb(0x26, 0x26, 0x2b) });
                let (nodes, closed) = model::preset_shape(k, r.left() + 10.0, r.top() + 10.0, 28.0, 28.0);
                let pts: Vec<Pos2> = model::flatten(&nodes, closed, 8).iter().map(|p| pos2(p[0], p[1])).collect();
                if closed {
                    ui.painter().add(Shape::closed_line(pts, Stroke::new(1.6, INK)));
                } else {
                    ui.painter().add(Shape::line(pts, Stroke::new(2.0, INK)));
                }
                if resp.on_hover_text(*name).clicked() {
                    insert = Some(k);
                }
            }
        });
        if let Some(k) = insert {
            let b = self.doc.all_boards()[self.active_board.min(self.doc.boards.len())].clone();
            let s = b.w.min(b.h) * 0.3;
            let (nodes, closed) = model::preset_shape(k, b.x + (b.w - s) / 2.0, b.y + (b.h - s) / 2.0, s, s);
            self.checkpoint();
            let mut it = self.new_item(Geom::Path { nodes, closed });
            it.name = format!("{} {}", model::SHAPES[k], self.doc.items.len() + 1);
            if !closed {
                it.stroke_w = it.stroke_w.max(s / 12.0);
            }
            self.add_item(it);
            self.tool = Tool::Select;
        }
    }
}

fn appearance(ui: &mut Ui, it: &mut Item) {
    ui.spacing_mut().item_spacing.y = 6.0;
    color_opt(ui, "Fond", &mut it.fill);
    color_opt(ui, "Contour", &mut it.stroke);
    let mut op = it.opacity * 100.0;
    if slider(ui, "Opacité", &mut op, 0.0..=100.0, " %") {
        it.opacity = op / 100.0;
    }
    small(ui, "Mode de fusion");
    egui::ComboBox::from_id_salt("vblend").selected_text(model::BLENDS[it.blend as usize % model::BLENDS.len()].1).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
        for (k, (_, name)) in model::BLENDS.iter().enumerate() {
            ui.selectable_value(&mut it.blend, k as u8, *name);
        }
    });
}

fn stroke_panel(ui: &mut Ui, it: &mut Item) {
    ui.spacing_mut().item_spacing.y = 6.0;
    slider(ui, "Épaisseur", &mut it.stroke_w, 0.0..=100.0, " px");
    let closed = !matches!(it.geom, Geom::Path { closed: false, .. });
    if closed {
        small(ui, "Position du contour");
        let mut a = it.stroke_align as usize;
        if pro::segmented(ui, &mut a, &["Centre", "Intérieur", "Extérieur"]) {
            it.stroke_align = a as u8;
        }
    }
    ui.horizontal(|ui| {
        row_label(ui, "Bouts", 64.0);
        let mut cap = it.cap as usize;
        if pro::segmented(ui, &mut cap, &["Ronds", "Plats", "Carrés"]) {
            it.cap = cap as u8;
        }
    });
    ui.horizontal(|ui| {
        row_label(ui, "Angles", 64.0);
        let mut join = it.join as usize;
        if pro::segmented(ui, &mut join, &["Ronds", "Pointus", "Biseaux"]) {
            it.join = join as u8;
        }
    });
    if it.join == 1 {
        ui.horizontal(|ui| {
            row_label(ui, "Limite de pointe", 110.0);
            dv(ui, &mut it.miter, 1.0..=50.0, "×");
        });
    }
    small(ui, "Pointillés");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for (n, d, g, d2, g2, cap) in [("Plein", 0.0, 0.0, 0.0, 0.0, None), ("Tirets", 12.0, 6.0, 0.0, 0.0, Some(1)), ("Points", 0.1, 8.0, 0.0, 0.0, Some(0)), ("Tiret-point", 14.0, 6.0, 0.1, 6.0, Some(0)), ("Longs", 30.0, 10.0, 0.0, 0.0, Some(1)), ("Serrés", 4.0, 3.0, 0.0, 0.0, Some(1))] {
            if pro::chip(ui, n).clicked() {
                it.dash = d;
                it.gap = g;
                it.dash2 = d2;
                it.gap2 = g2;
                if let Some(c) = cap {
                    it.cap = c;
                }
            }
        }
    });
    if it.dash > 0.0 {
        ui.horizontal(|ui| {
            row_label(ui, "Tiret", 40.0);
            dv(ui, &mut it.dash, 0.0..=500.0, "");
            row_label(ui, "Espace", 48.0);
            dv(ui, &mut it.gap, 0.0..=500.0, "");
        });
        ui.horizontal(|ui| {
            row_label(ui, "Tiret 2", 40.0);
            dv(ui, &mut it.dash2, 0.0..=500.0, "");
            row_label(ui, "Espace 2", 48.0);
            dv(ui, &mut it.gap2, 0.0..=500.0, "");
        });
    }
    if matches!(it.geom, Geom::Path { closed: false, .. }) {
        small(ui, "Pointes de flèche (début · fin)");
        let mut end = if it.head_end > 0 { it.head_end } else if it.arrow_end { 1 } else { 0 } as usize;
        let mut start = it.head_start as usize;
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("hstart").selected_text(model::HEADS[start]).width(118.0).show_ui(ui, |ui| {
                for (k, n) in model::HEADS.iter().enumerate() {
                    ui.selectable_value(&mut start, k, *n);
                }
            });
            egui::ComboBox::from_id_salt("hend").selected_text(model::HEADS[end]).width(118.0).show_ui(ui, |ui| {
                for (k, n) in model::HEADS.iter().enumerate() {
                    ui.selectable_value(&mut end, k, *n);
                }
            });
        });
        if start as u8 != it.head_start || end as u8 != it.head_end {
            it.head_start = start as u8;
            it.head_end = end as u8;
            it.arrow_end = false;
        }
        if start > 0 || end > 0 {
            let mut s = it.head_scale * 100.0;
            if slider(ui, "Taille des pointes", &mut s, 20.0..=400.0, " %") {
                it.head_scale = s / 100.0;
            }
        }
    }
    if matches!(it.geom, Geom::Path { .. }) {
        small(ui, "Profil de largeur");
        let mut p = it.profile as usize;
        egui::ComboBox::from_id_salt("sprof").selected_text(model::PROFILES[p]).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
            for (k, n) in model::PROFILES.iter().enumerate() {
                ui.selectable_value(&mut p, k, *n);
            }
        });
        it.profile = p as u8;
        if !it.widths.is_empty() {
            ui.horizontal(|ui| {
                hint(ui, &format!("{} point(s) de largeur", it.widths.len()));
                if pro::ghost(ui, "Réinitialiser").clicked() {
                    it.widths.clear();
                }
            });
        }
    }
}

fn gradient_panel(ui: &mut Ui, it: &mut Item) {
    ui.spacing_mut().item_spacing.y = 6.0;
    let has = it.fill2.is_some() || !it.stops.is_empty();
    let mut kind = if !has { 0 } else if it.radial { 2 } else { 1 };
    if pro::segmented(ui, &mut kind, &["Aucun", "Linéaire", "Radial"]) {
        match kind {
            0 => {
                it.fill2 = None;
                it.stops.clear();
                it.grad_vec = None;
            }
            k => {
                if !has {
                    let a = it.fill.unwrap_or([0, 0, 0, 255]);
                    it.stops = vec![Stop { t: 0.0, c: a }, Stop { t: 1.0, c: [255, 255, 255, 255] }];
                }
                it.radial = k == 2;
            }
        }
    }
    if kind == 0 {
        if !it.mesh.is_empty() {
            mesh_row(ui, &mut it.mesh);
        }
        return;
    }
    if it.stops.is_empty() {
        it.stops = vec![Stop { t: 0.0, c: it.fill.unwrap_or([0; 4]) }, Stop { t: 1.0, c: it.fill2.unwrap_or([255; 4]) }];
        it.fill2 = None;
    }
    let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    let mut mesh = egui::Mesh::default();
    let mut st = it.stops.clone();
    st.sort_by(|a, b| a.t.total_cmp(&b.t));
    for w in st.windows(2) {
        let (x0, x1) = (bar.left() + bar.width() * w[0].t, bar.left() + bar.width() * w[1].t);
        let i = mesh.vertices.len() as u32;
        for (p, c) in [(pos2(x0, bar.top()), w[0].c), (pos2(x1, bar.top()), w[1].c), (pos2(x1, bar.bottom()), w[1].c), (pos2(x0, bar.bottom()), w[0].c)] {
            mesh.colored_vertex(p, c32(c));
        }
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    }
    checker_rect(ui.painter(), bar);
    ui.painter().add(mesh);
    let mut remove = None;
    for (k, s) in it.stops.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.push_id(("stop", k), |ui| color_btn(ui, &mut s.c));
            let mut t = s.t * 100.0;
            ui.spacing_mut().slider_width = 110.0;
            if ui.add(egui::Slider::new(&mut t, 0.0..=100.0).suffix(" %")).changed() {
                s.t = t / 100.0;
            }
            if pro::ghost(ui, "×").clicked() {
                remove = Some(k);
            }
        });
    }
    if let Some(k) = remove.filter(|_| it.stops.len() > 2) {
        it.stops.remove(k);
    }
    ui.horizontal_wrapped(|ui| {
        if pro::chip(ui, "+ Arrêt").clicked() {
            let c = it.stops.last().map_or([255; 4], |s| s.c);
            it.stops.push(Stop { t: 0.5, c });
        }
        if pro::chip(ui, "Inverser").clicked() {
            for s in &mut it.stops {
                s.t = 1.0 - s.t;
            }
        }
        if it.grad_vec.is_some() && pro::chip(ui, "Recentrer").clicked() {
            it.grad_vec = None;
        }
    });
    if !it.radial && it.grad_vec.is_none() {
        slider(ui, "Angle", &mut it.grad_angle, -180.0..=180.0, "°");
    }
    if let Some(f) = it.stops.first() {
        it.fill = Some(f.c);
    }
    if !it.mesh.is_empty() {
        mesh_row(ui, &mut it.mesh);
    }
}

fn mesh_row(ui: &mut Ui, mesh: &mut Vec<MeshPt>) {
    ui.horizontal(|ui| {
        hint(ui, &format!("Filet : {} point(s) de couleur", mesh.len()));
        if pro::ghost(ui, "Effacer").clicked() {
            mesh.clear();
        }
    });
}

fn effects_panel(ui: &mut Ui, it: &mut Item) {
    ui.spacing_mut().item_spacing.y = 6.0;
    ui.horizontal(|ui| {
        ui.checkbox(&mut it.shadow, "Ombre portée");
        if it.shadow {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| color_btn(ui, &mut it.sh_color));
        }
    });
    if it.shadow {
        ui.horizontal(|ui| {
            row_label(ui, "X", 12.0);
            dv(ui, &mut it.sh_x, -200.0..=200.0, "");
            row_label(ui, "Y", 12.0);
            dv(ui, &mut it.sh_y, -200.0..=200.0, "");
            row_label(ui, "Flou", 30.0);
            dv(ui, &mut it.sh_blur, 0.0..=200.0, "");
        });
    }
    let fx = |ui: &mut Ui, label: &str, v: &mut f32, max: f32, c: Option<&mut [u8; 4]>| {
        ui.horizontal(|ui| {
            row_label(ui, label, 100.0);
            ui.spacing_mut().slider_width = 64.0;
            ui.add(egui::Slider::new(v, 0.0..=max));
            if let Some(c) = c {
                color_btn(ui, c);
            }
        });
    };
    fx(ui, "Lueur externe", &mut it.glow_out, 80.0, Some(&mut it.glow_out_c));
    fx(ui, "Lueur interne", &mut it.glow_in, 60.0, Some(&mut it.glow_in_c));
    fx(ui, "Contour progressif", &mut it.feather, 60.0, None);
    fx(ui, "Flou gaussien", &mut it.blur, 60.0, None);
    fx(ui, "Distorsion", &mut it.rough, 60.0, None);
    if it.rough > 0.0 {
        let mut f = it.rough_freq * 1000.0;
        ui.horizontal(|ui| {
            row_label(ui, "Finesse", 100.0);
            ui.spacing_mut().slider_width = 64.0;
            if ui.add(egui::Slider::new(&mut f, 2.0..=300.0)).changed() {
                it.rough_freq = f / 1000.0;
            }
        });
    }
    let mut g = it.grain * 100.0;
    ui.horizontal(|ui| {
        row_label(ui, "Grain", 100.0);
        ui.spacing_mut().slider_width = 64.0;
        if ui.add(egui::Slider::new(&mut g, 0.0..=100.0).suffix(" %")).changed() {
            it.grain = g / 100.0;
        }
    });
}

fn shape_props(ui: &mut Ui, it: &mut Item) -> bool {
    ui.spacing_mut().item_spacing.y = 6.0;
    let mut trace = false;
    match &mut it.geom {
        Geom::Rect { w, h, r, .. } => {
            let max = w.min(*h) / 2.0;
            slider(ui, "Arrondi des coins", r, 0.0..=max, " px");
        }
        Geom::Path { nodes, closed } => {
            hint(ui, &format!("{} points{}", nodes.len(), if it.sub.is_empty() { String::new() } else { format!(" · {} sous-tracé(s)", it.sub.len()) }));
            ui.checkbox(closed, "Tracé fermé");
            ui.horizontal_wrapped(|ui| {
                if pro::chip(ui, "Lisser").clicked() {
                    model::smooth(nodes, *closed);
                }
                if pro::chip(ui, "Simplifier").clicked() {
                    let pts: Vec<model::P> = nodes.iter().map(|n| n.p).collect();
                    *nodes = model::simplify(&pts, 1.5);
                }
                if pro::chip(ui, "Inverser").clicked() {
                    model::reverse(nodes);
                }
                if pro::chip(ui, "+ Points").clicked() {
                    model::add_midpoints(nodes, *closed);
                }
            });
        }
        Geom::Text { text, size, family, bold, .. } => {
            ui.add(egui::TextEdit::multiline(text).desired_rows(3).desired_width(f32::INFINITY));
            egui::ComboBox::from_id_salt("font").selected_text(family.as_str()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                for f in FONTS {
                    ui.selectable_value(family, f.to_string(), f);
                }
            });
            slider(ui, "Taille", size, 4.0..=600.0, " px");
            ui.horizontal(|ui| {
                ui.toggle_value(bold, RichText::new("G").strong()).on_hover_text("Gras");
                ui.toggle_value(&mut it.italic, RichText::new("I").italics()).on_hover_text("Italique");
                let mut u = it.deco == 1;
                if ui.toggle_value(&mut u, RichText::new("S").underline()).on_hover_text("Souligné").changed() {
                    it.deco = if u { 1 } else { 0 };
                }
                let mut s = it.deco == 2;
                if ui.toggle_value(&mut s, RichText::new("B").strikethrough()).on_hover_text("Barré").changed() {
                    it.deco = if s { 2 } else { 0 };
                }
                ui.add_space(6.0);
                let mut cs = it.case as usize;
                if pro::segmented(ui, &mut cs, &["Aa", "AA", "aa", "Titre"]) {
                    it.case = cs as u8;
                }
            });
            let mut al = it.align as usize;
            if pro::segmented(ui, &mut al, &["Gauche", "Centre", "Droite"]) {
                it.align = al as u8;
            }
            ui.horizontal(|ui| {
                row_label(ui, "Approche", 70.0);
                dv(ui, &mut it.tracking, -20.0..=200.0, " px");
                ui.checkbox(&mut it.kern, "Crénage");
            });
            ui.horizontal(|ui| {
                row_label(ui, "Interlignage", 70.0);
                let mut lead = it.lead();
                if ui.add(egui::DragValue::new(&mut lead).range(0.5..=4.0).speed(0.01).suffix("×")).changed() {
                    it.leading = lead;
                }
                row_label(ui, "Ligne de base", 80.0);
                dv(ui, &mut it.baseline, -500.0..=500.0, "");
            });
            ui.horizontal(|ui| {
                row_label(ui, "Largeur du cadre", 110.0);
                dv(ui, &mut it.box_w, 0.0..=10000.0, " px");
            });
            ui.checkbox(&mut it.vertical, "Texte vertical");
            if !it.text_path.is_empty() {
                ui.horizontal(|ui| {
                    row_label(ui, "Départ sur le tracé", 120.0);
                    dv(ui, &mut it.path_offset, 0.0..=100.0, " %");
                });
                if pro::ghost(ui, "Détacher du tracé").clicked() {
                    it.text_path.clear();
                }
            }
        }
        Geom::Image { w, h, .. } => {
            hint(ui, &format!("{:.0} × {:.0} px", w, h));
            trace = pro::secondary(ui, "Vectorisation dynamique…").clicked();
        }
        Geom::Ellipse { .. } => {}
    }
    trace
}

fn draw_items(p: &Painter, r: Rect, items: &[Item]) {
    let Some(b) = items.iter().map(|i| i.bbox()).reduce(|a, b| a.union(&b)) else { return };
    let s = (r.width() / b.size()[0].max(1.0)).min(r.height() / b.size()[1].max(1.0));
    let off = r.center() - vec2(b.size()[0], b.size()[1]) * s / 2.0;
    for it in items {
        let col = it.fill.or(it.stroke).map(c32).unwrap_or(INK);
        for l in it.loops() {
            let pts: Vec<Pos2> = l.iter().map(|q| off + vec2(q[0] - b.min[0], q[1] - b.min[1]) * s).collect();
            p.add(Shape::closed_line(pts, Stroke::new(1.4, col)));
        }
    }
}

fn eye_icon(p: &Painter, eye: Rect, on: bool) {
    let ec = if on { INK } else { FAINT };
    p.add(Shape::convex_polygon(vec![eye.left_center() + vec2(1.0, 0.0), eye.center() - vec2(0.0, 4.5), eye.right_center() - vec2(1.0, 0.0), eye.center() + vec2(0.0, 4.5)], Color32::TRANSPARENT, Stroke::new(1.3, ec)));
    if on {
        p.circle_filled(eye.center(), 2.2, ec);
    }
}

pub fn lock_icon(p: &Painter, r: Rect, locked: bool, c: Color32) {
    let body = Rect::from_min_max(r.min + vec2(3.0, 8.0), r.max - vec2(3.0, 1.0));
    p.rect_filled(body, CornerRadius::same(2), c);
    let top = pos2(r.center().x, r.top() + 2.0);
    let shift = if locked { 0.0 } else { 4.0 };
    p.add(Shape::line(vec![pos2(body.left() + 2.0, body.top()), pos2(body.left() + 2.0, top.y + 3.0 - shift), pos2(top.x, top.y - shift), pos2(body.right() - 2.0, top.y + 3.0 - shift), pos2(body.right() - 2.0, body.top() - shift)], Stroke::new(1.6, c)));
}

pub fn align_icon(p: &Painter, r: Rect, axis: usize, to: f32, c: Color32) {
    let s = Stroke::new(1.6, c);
    if axis == 0 {
        let x = r.left() + r.width() * to;
        p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], s);
        for (k, w) in [r.width() * 0.7, r.width() * 0.45].iter().enumerate() {
            let y = r.top() + r.height() * (0.25 + 0.42 * k as f32);
            p.rect_filled(Rect::from_min_size(pos2(x - w * to, y), vec2(*w, r.height() * 0.22)), CornerRadius::same(2), c);
        }
    } else {
        let y = r.top() + r.height() * to;
        p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], s);
        for (k, h) in [r.height() * 0.7, r.height() * 0.45].iter().enumerate() {
            let x = r.left() + r.width() * (0.25 + 0.42 * k as f32);
            p.rect_filled(Rect::from_min_size(pos2(x, y - h * to), vec2(r.width() * 0.22, *h)), CornerRadius::same(2), c);
        }
    }
}

fn dist_icon(p: &Painter, r: Rect, axis: usize, at: f32, c: Color32) {
    for k in 0..3 {
        let f = k as f32 / 2.0;
        let (a, b) = if axis == 0 {
            let x = r.left() + r.width() * (0.12 + 0.76 * f);
            (Rect::from_center_size(pos2(x, r.center().y), vec2(r.width() * 0.2, r.height() * (0.5 + 0.2 * (k % 2) as f32))), [pos2(x - r.width() * 0.1 + r.width() * 0.2 * at, r.top()), pos2(x - r.width() * 0.1 + r.width() * 0.2 * at, r.bottom())])
        } else {
            let y = r.top() + r.height() * (0.12 + 0.76 * f);
            (Rect::from_center_size(pos2(r.center().x, y), vec2(r.width() * (0.5 + 0.2 * (k % 2) as f32), r.height() * 0.2)), [pos2(r.left(), y - r.height() * 0.1 + r.height() * 0.2 * at), pos2(r.right(), y - r.height() * 0.1 + r.height() * 0.2 * at)])
        };
        p.rect_filled(a, CornerRadius::same(1), c.gamma_multiply(0.7));
        p.line_segment(b, Stroke::new(1.0, c));
    }
}

fn quick_icon(p: &Painter, r: Rect, k: usize, c: Color32) {
    let s = Stroke::new(1.6, c);
    match k {
        0 => {
            let pts: Vec<Pos2> = (0..=14).map(|i| {
                let a = -1.4 + i as f32 / 14.0 * 4.0;
                r.center() + vec2(a.cos(), a.sin()) * r.width() * 0.42
            }).collect();
            let tip = pts[pts.len() - 1];
            p.add(Shape::line(pts, s));
            p.circle_filled(tip, 2.5, c);
        }
        1 => {
            p.line_segment([r.center_top(), r.center_bottom()], Stroke::new(1.0, c));
            p.add(Shape::convex_polygon(vec![r.left_center(), pos2(r.center().x - 2.0, r.top() + 2.0), pos2(r.center().x - 2.0, r.bottom() - 2.0)], c, Stroke::NONE));
            p.add(Shape::convex_polygon(vec![r.right_center(), pos2(r.center().x + 2.0, r.bottom() - 2.0), pos2(r.center().x + 2.0, r.top() + 2.0)], Color32::TRANSPARENT, s));
        }
        _ => {
            p.line_segment([r.left_center(), r.right_center()], Stroke::new(1.0, c));
            p.add(Shape::convex_polygon(vec![r.center_top(), pos2(r.right() - 2.0, r.center().y - 2.0), pos2(r.left() + 2.0, r.center().y - 2.0)], c, Stroke::NONE));
            p.add(Shape::convex_polygon(vec![r.center_bottom(), pos2(r.left() + 2.0, r.center().y + 2.0), pos2(r.right() - 2.0, r.center().y + 2.0)], Color32::TRANSPARENT, s));
        }
    }
}

fn pf_icon(p: &Painter, r: Rect, k: usize, c: Color32) {
    let a = Rect::from_min_size(r.min, r.size() * 0.66);
    let b = Rect::from_min_max(r.min + r.size() * 0.34, r.max);
    let i = a.intersect(b);
    let dim = c.gamma_multiply(0.25);
    let line = Stroke::new(1.2, c);
    let fill = |rr: Rect, col: Color32| p.rect_filled(rr, CornerRadius::ZERO, col);
    match k {
        0 => {
            fill(a, c);
            fill(b, c);
        }
        1 => {
            fill(a, c);
            fill(i, PANEL);
            p.rect_stroke(b, CornerRadius::ZERO, Stroke::new(1.0, dim), StrokeKind::Inside);
        }
        2 => {
            p.rect_stroke(a, CornerRadius::ZERO, Stroke::new(1.0, dim), StrokeKind::Inside);
            p.rect_stroke(b, CornerRadius::ZERO, Stroke::new(1.0, dim), StrokeKind::Inside);
            fill(i, c);
        }
        3 => {
            fill(a, c);
            fill(b, c);
            fill(i, PANEL);
        }
        4 => {
            fill(a, c.gamma_multiply(0.55));
            fill(b, c.gamma_multiply(0.8));
            fill(i, c);
            p.rect_stroke(i, CornerRadius::ZERO, Stroke::new(1.0, PANEL), StrokeKind::Inside);
        }
        5 => {
            fill(a, c.gamma_multiply(0.5));
            fill(b, c);
        }
        6 => {
            fill(a, c);
            fill(b, c);
            p.line_segment([i.left_top(), i.left_bottom()], Stroke::new(1.0, PANEL));
        }
        7 => {
            p.rect_stroke(b, CornerRadius::ZERO, Stroke::new(1.0, dim), StrokeKind::Inside);
            fill(i, c);
        }
        8 => {
            p.rect_stroke(a, CornerRadius::ZERO, line, StrokeKind::Inside);
            p.rect_stroke(b, CornerRadius::ZERO, line, StrokeKind::Inside);
        }
        _ => {
            p.rect_stroke(a, CornerRadius::ZERO, Stroke::new(1.0, dim), StrokeKind::Inside);
            fill(b, c);
            fill(i, PANEL);
        }
    }
}

pub fn export_glyph(p: &Painter, r: Rect, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    p.line_segment([at(0.5, 0.05), at(0.5, 0.65)], Stroke::new(2.0, c));
    p.line_segment([at(0.5, 0.05), at(0.25, 0.3)], Stroke::new(2.0, c));
    p.line_segment([at(0.5, 0.05), at(0.75, 0.3)], Stroke::new(2.0, c));
    p.add(Shape::line(vec![at(0.12, 0.55), at(0.12, 0.95), at(0.88, 0.95), at(0.88, 0.55)], Stroke::new(2.0, c)));
}

fn curve(p: &Painter, r: Rect, nodes: &[model::Node], closed: bool, s: Stroke) {
    let pts: Vec<Pos2> = model::flatten(nodes, closed, 10).iter().map(|q| r.min + vec2(q[0], q[1]) * r.width()).collect();
    if closed {
        p.add(Shape::closed_line(pts, s));
    } else {
        p.add(Shape::line(pts, s));
    }
}

pub fn tool_icon(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    let s = Stroke::new(1.7, c);
    let thin = Stroke::new(1.2, c);
    let arrow_cursor = |fill: bool, dx: f32| {
        let pts = vec![at(0.2 + dx, 0.0), at(0.85 + dx, 0.6), at(0.52 + dx, 0.62), at(0.33 + dx, 1.0)];
        p.add(Shape::convex_polygon(pts, if fill { c } else { Color32::TRANSPARENT }, if fill { Stroke::NONE } else { s }));
    };
    let pen = |p: &Painter| {
        p.add(Shape::convex_polygon(vec![at(0.5, 0.05), at(0.82, 0.55), at(0.5, 0.95), at(0.18, 0.55)], Color32::TRANSPARENT, s));
        p.line_segment([at(0.5, 0.35), at(0.5, 0.9)], thin);
    };
    let badge = |p: &Painter, plus: Option<bool>| {
        let b = at(0.82, 0.18);
        p.line_segment([b - vec2(4.0, 0.0), b + vec2(4.0, 0.0)], thin);
        if plus == Some(true) {
            p.line_segment([b - vec2(0.0, 4.0), b + vec2(0.0, 4.0)], thin);
        }
    };
    let ring = |p: &Painter, rr: f32| {
        p.circle_stroke(r.center(), r.width() * rr, s);
    };
    let brush_tip = |p: &Painter| {
        p.line_segment([at(0.9, 0.1), at(0.4, 0.6)], Stroke::new(2.4, c));
        p.add(Shape::convex_polygon(vec![at(0.3, 0.55), at(0.45, 0.7), at(0.2, 0.92), at(0.08, 0.95)], c, Stroke::NONE));
    };
    match tool {
        Tool::Select => arrow_cursor(true, 0.0),
        Tool::Direct => arrow_cursor(false, 0.0),
        Tool::GroupSel => {
            arrow_cursor(false, -0.1);
            p.line_segment([at(0.78, 0.82), at(1.0, 0.82)], thin);
            p.line_segment([at(0.89, 0.71), at(0.89, 0.93)], thin);
        }
        Tool::Wand => {
            p.line_segment([at(0.1, 0.95), at(0.6, 0.45)], Stroke::new(2.2, c));
            for (x, y) in [(0.75, 0.1), (0.9, 0.3), (0.55, 0.2)] {
                p.circle_filled(at(x, y), 1.6, c);
            }
        }
        Tool::Lasso => {
            p.add(Shape::closed_line((0..24).map(|k| {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                at(0.55 + a.cos() * 0.4, 0.4 + a.sin() * 0.28)
            }).collect(), s));
            p.line_segment([at(0.25, 0.6), at(0.2, 0.95)], s);
        }
        Tool::Pen => pen(p),
        Tool::AddPt => {
            pen(p);
            badge(p, Some(true));
        }
        Tool::DelPt => {
            pen(p);
            badge(p, Some(false));
        }
        Tool::ConvPt => {
            p.add(Shape::line(vec![at(0.1, 0.9), at(0.5, 0.3), at(0.9, 0.9)], s));
            p.line_segment([at(0.2, 0.3), at(0.8, 0.3)], thin);
            p.circle_filled(at(0.5, 0.3), 2.2, c);
        }
        Tool::Curv => {
            curve(p, r, &[model::Node { p: [0.05, 0.8], cin: [0.05, 0.8], cout: [0.3, 0.0] }, model::Node { p: [0.95, 0.5], cin: [0.6, 1.1], cout: [0.95, 0.5] }], false, s);
            p.circle_filled(at(0.05, 0.8), 2.0, c);
            p.circle_filled(at(0.95, 0.5), 2.0, c);
        }
        Tool::Text | Tool::AreaText | Tool::PathText | Tool::VText => {
            if tool == Tool::VText {
                p.line_segment([at(0.1, 0.1), at(0.9, 0.1)], Stroke::new(2.2, c));
                p.line_segment([at(0.9, 0.1), at(0.9, 0.95)], Stroke::new(2.2, c));
            } else {
                p.line_segment([at(0.15, 0.1), at(0.85, 0.1)], Stroke::new(2.2, c));
                p.line_segment([at(0.5, 0.1), at(0.5, 0.9)], Stroke::new(2.2, c));
            }
            if tool == Tool::AreaText {
                p.rect_stroke(Rect::from_min_max(at(0.0, 0.0), at(1.0, 1.0)), CornerRadius::ZERO, Stroke::new(1.0, c.gamma_multiply(0.6)), StrokeKind::Middle);
            }
            if tool == Tool::PathText {
                curve(p, r, &[model::Node { p: [0.0, 0.95], cin: [0.0, 0.95], cout: [0.4, 0.6] }, model::Node { p: [1.0, 0.95], cin: [0.6, 0.6], cout: [1.0, 0.95] }], false, thin);
            }
        }
        Tool::Line => {
            p.line_segment([at(0.05, 0.95), at(0.95, 0.05)], Stroke::new(2.0, c));
        }
        Tool::Arc => curve(p, r, &model::arc([0.05, 0.05], [0.95, 0.95], 0.55, true), false, s),
        Tool::Spiral => {
            let pts: Vec<Pos2> = (0..=60).map(|k| {
                let t = k as f32 / 60.0;
                let a = t * 2.6 * std::f32::consts::TAU;
                r.center() + vec2(a.cos(), a.sin()) * r.width() * 0.48 * t
            }).collect();
            p.add(Shape::line(pts, s));
        }
        Tool::RectGrid => {
            p.rect_stroke(r, CornerRadius::ZERO, thin, StrokeKind::Middle);
            for k in 1..3 {
                let f = k as f32 / 3.0;
                p.line_segment([at(f, 0.0), at(f, 1.0)], thin);
                p.line_segment([at(0.0, f), at(1.0, f)], thin);
            }
        }
        Tool::PolarGrid => {
            ring(p, 0.48);
            ring(p, 0.28);
            p.line_segment([at(0.5, 0.0), at(0.5, 1.0)], thin);
            p.line_segment([at(0.0, 0.5), at(1.0, 0.5)], thin);
        }
        Tool::Arrow => {
            let pts: Vec<Pos2> = model::arrow([0.05, 0.5], [0.95, 0.5], 0.2).iter().map(|n| at(n.p[0], n.p[1])).collect();
            p.add(Shape::closed_line(pts, s));
        }
        Tool::Rect => {
            p.rect_stroke(Rect::from_min_max(at(0.05, 0.15), at(0.95, 0.85)), CornerRadius::same(1), s, StrokeKind::Middle);
        }
        Tool::RRect => {
            p.rect_stroke(Rect::from_min_max(at(0.05, 0.15), at(0.95, 0.85)), CornerRadius::same(5), s, StrokeKind::Middle);
        }
        Tool::Ellipse => ring(p, 0.45),
        Tool::Polygon => {
            let pts: Vec<Pos2> = model::polygon(0.0, 0.0, 1.0, 6, None).iter().map(|n| r.center() + vec2(n.p[0], n.p[1]) * r.width() * 0.5).collect();
            p.add(Shape::closed_line(pts, s));
        }
        Tool::Star => {
            let pts: Vec<Pos2> = model::polygon(0.0, 0.0, 1.0, 5, Some(0.45)).iter().map(|n| r.center() + vec2(n.p[0], n.p[1]) * r.width() * 0.52).collect();
            p.add(Shape::closed_line(pts, s));
        }
        Tool::Flare => {
            p.circle_filled(r.center(), r.width() * 0.16, c);
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                p.line_segment([r.center() + vec2(a.cos(), a.sin()) * r.width() * 0.25, r.center() + vec2(a.cos(), a.sin()) * r.width() * 0.5], thin);
            }
        }
        Tool::Heart => curve(p, r, &model::heart(0.05, 0.1, 0.9, 0.85), true, s),
        Tool::Brush => brush_tip(p),
        Tool::Blob => {
            brush_tip(p);
            p.circle_filled(at(0.85, 0.85), r.width() * 0.13, c);
        }
        Tool::Pencil => {
            p.line_segment([at(0.85, 0.15), at(0.25, 0.75)], Stroke::new(4.0, c));
            p.add(Shape::convex_polygon(vec![at(0.18, 0.68), at(0.32, 0.82), at(0.05, 0.95)], c, Stroke::NONE));
        }
        Tool::Smooth => {
            p.line_segment([at(0.85, 0.15), at(0.35, 0.65)], Stroke::new(4.0, c));
            curve(p, r, &[model::Node { p: [0.0, 0.9], cin: [0.0, 0.9], cout: [0.3, 0.75] }, model::Node { p: [0.6, 0.95], cin: [0.4, 1.0], cout: [0.6, 0.95] }], false, thin);
        }
        Tool::PathErase => {
            p.line_segment([at(0.85, 0.15), at(0.4, 0.6)], Stroke::new(4.0, c));
            p.line_segment([at(0.0, 0.9), at(0.3, 0.9)], thin);
            p.line_segment([at(0.55, 0.9), at(1.0, 0.9)], thin);
        }
        Tool::Rotate | Tool::RotView => {
            let pts: Vec<Pos2> = (0..=18).map(|k| {
                let a = (k as f32 / 18.0) * 4.8 - 0.9;
                r.center() + vec2(a.cos(), a.sin()) * r.width() * 0.42
            }).collect();
            let tip = pts[pts.len() - 1];
            p.add(Shape::line(pts, s));
            p.add(Shape::convex_polygon(vec![tip + vec2(-5.0, -1.0), tip + vec2(3.0, -5.0), tip + vec2(3.0, 3.0)], c, Stroke::NONE));
            if tool == Tool::RotView {
                p.rect_stroke(Rect::from_center_size(r.center(), r.size() * 0.35), CornerRadius::ZERO, thin, StrokeKind::Middle);
            }
        }
        Tool::Reflect => {
            p.line_segment([at(0.5, 0.0), at(0.5, 1.0)], Stroke::new(1.0, c));
            p.add(Shape::convex_polygon(vec![at(0.4, 0.2), at(0.4, 0.85), at(0.0, 0.85)], c, Stroke::NONE));
            p.add(Shape::convex_polygon(vec![at(0.6, 0.2), at(1.0, 0.85), at(0.6, 0.85)], Color32::TRANSPARENT, s));
        }
        Tool::Scale => {
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.35), at(0.65, 1.0)), CornerRadius::ZERO, s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.0), at(1.0, 1.0)), CornerRadius::ZERO, Stroke::new(1.0, c.gamma_multiply(0.6)), StrokeKind::Middle);
            p.line_segment([at(0.45, 0.55), at(0.9, 0.1)], thin);
        }
        Tool::Distort => {
            p.add(Shape::closed_line(vec![at(0.15, 0.1), at(0.95, 0.25), at(0.8, 0.9), at(0.0, 0.8)], s));
            p.circle_filled(at(0.95, 0.25), 2.4, c);
        }
        Tool::Reshape => {
            curve(p, r, &[model::Node { p: [0.0, 0.85], cin: [0.0, 0.85], cout: [0.35, 0.85] }, model::Node { p: [0.5, 0.2], cin: [0.3, 0.2], cout: [0.7, 0.2] }, model::Node { p: [1.0, 0.85], cin: [0.65, 0.85], cout: [1.0, 0.85] }], false, s);
            p.rect_filled(Rect::from_center_size(at(0.5, 0.2), Vec2::splat(5.0)), CornerRadius::ZERO, c);
        }
        Tool::Width => {
            p.add(Shape::convex_polygon(vec![at(0.0, 0.5), at(0.5, 0.2), at(1.0, 0.5), at(0.5, 0.8)], c.gamma_multiply(0.5), s));
            p.line_segment([at(0.5, 0.0), at(0.5, 1.0)], thin);
        }
        Tool::Warp => {
            p.circle_stroke(r.center(), r.width() * 0.45, thin);
            p.line_segment([at(0.25, 0.5), at(0.75, 0.5)], s);
            p.add(Shape::convex_polygon(vec![at(0.8, 0.5), at(0.62, 0.38), at(0.62, 0.62)], c, Stroke::NONE));
        }
        Tool::Twirl => {
            let pts: Vec<Pos2> = (0..=40).map(|k| {
                let t = k as f32 / 40.0;
                let a = t * 1.6 * std::f32::consts::TAU;
                r.center() + vec2(a.cos(), a.sin()) * r.width() * (0.12 + 0.36 * t)
            }).collect();
            p.add(Shape::line(pts, s));
        }
        Tool::Pucker | Tool::Bloat => {
            p.circle_stroke(r.center(), r.width() * 0.45, thin);
            for k in 0..4 {
                let a = k as f32 / 4.0 * std::f32::consts::TAU + 0.785;
                let (o, i) = (r.center() + vec2(a.cos(), a.sin()) * r.width() * 0.4, r.center() + vec2(a.cos(), a.sin()) * r.width() * 0.12);
                let (from, to) = if tool == Tool::Pucker { (o, i) } else { (i, o) };
                p.line_segment([from, to], s);
                p.circle_filled(to, 2.0, c);
            }
        }
        Tool::Builder => {
            p.circle_stroke(at(0.38, 0.5), r.width() * 0.32, thin);
            p.circle_stroke(at(0.62, 0.5), r.width() * 0.32, thin);
            p.line_segment([at(0.85, 0.1), at(0.85, 0.35)], s);
            p.line_segment([at(0.72, 0.22), at(0.98, 0.22)], s);
        }
        Tool::Bucket => {
            p.add(Shape::convex_polygon(vec![at(0.15, 0.45), at(0.5, 0.1), at(0.85, 0.45), at(0.5, 0.8)], Color32::TRANSPARENT, s));
            p.circle_filled(at(0.88, 0.75), r.width() * 0.1, c);
        }
        Tool::Gradient => {
            let mut mesh = egui::Mesh::default();
            let rr = Rect::from_min_max(at(0.0, 0.15), at(1.0, 0.85));
            for (pt, col) in [(rr.left_top(), Color32::TRANSPARENT), (rr.right_top(), c), (rr.right_bottom(), c), (rr.left_bottom(), Color32::TRANSPARENT)] {
                mesh.colored_vertex(pt, col);
            }
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            p.add(mesh);
            p.rect_stroke(rr, CornerRadius::ZERO, thin, StrokeKind::Middle);
        }
        Tool::Mesh => {
            p.rect_stroke(r, CornerRadius::same(2), thin, StrokeKind::Middle);
            curve(p, r, &[model::Node { p: [0.0, 0.45], cin: [0.0, 0.45], cout: [0.4, 0.2] }, model::Node { p: [1.0, 0.55], cin: [0.6, 0.8], cout: [1.0, 0.55] }], false, thin);
            curve(p, r, &[model::Node { p: [0.5, 0.0], cin: [0.5, 0.0], cout: [0.7, 0.4] }, model::Node { p: [0.45, 1.0], cin: [0.3, 0.6], cout: [0.45, 1.0] }], false, thin);
            p.circle_filled(at(0.52, 0.5), 2.4, c);
        }
        Tool::Picker => {
            p.line_segment([at(0.08, 0.92), at(0.62, 0.38)], Stroke::new(2.3, c));
            p.circle_filled(at(0.75, 0.25), r.width() * 0.2, c);
        }
        Tool::Measure => {
            p.add(Shape::closed_line(vec![at(0.0, 0.7), at(0.7, 0.0), at(1.0, 0.3), at(0.3, 1.0)], s));
            for k in 1..5 {
                let f = k as f32 / 5.0;
                p.line_segment([at(0.0 + 0.7 * f, 0.7 - 0.7 * f), at(0.1 + 0.7 * f, 0.8 - 0.7 * f)], thin);
            }
        }
        Tool::Scissors => {
            p.circle_stroke(at(0.25, 0.78), r.width() * 0.16, s);
            p.circle_stroke(at(0.75, 0.78), r.width() * 0.16, s);
            p.line_segment([at(0.35, 0.65), at(0.8, 0.0)], s);
            p.line_segment([at(0.65, 0.65), at(0.2, 0.0)], s);
        }
        Tool::Knife => {
            p.add(Shape::convex_polygon(vec![at(0.1, 0.9), at(0.85, 0.05), at(0.95, 0.15), at(0.3, 0.95)], Color32::TRANSPARENT, s));
        }
        Tool::Eraser => {
            p.add(Shape::convex_polygon(vec![at(0.05, 0.6), at(0.5, 0.15), at(0.95, 0.5), at(0.5, 0.95)], Color32::TRANSPARENT, s));
            p.line_segment([at(0.28, 0.38), at(0.72, 0.72)], thin);
        }
        Tool::Hand => {
            for (k, h) in [0.55, 0.75, 0.8, 0.7].iter().enumerate() {
                let x = 0.22 + k as f32 * 0.19;
                p.line_segment([at(x, 0.62), at(x, 0.62 - h * 0.6)], Stroke::new(2.4, c));
            }
            p.add(Shape::convex_polygon(vec![at(0.12, 0.6), at(0.85, 0.6), at(0.75, 1.0), at(0.25, 1.0)], c, Stroke::NONE));
        }
        Tool::Zoom => {
            p.circle_stroke(at(0.42, 0.42), r.width() * 0.32, s);
            p.line_segment([at(0.66, 0.66), at(0.98, 0.98)], Stroke::new(2.4, c));
        }
        Tool::Board => {
            p.rect_stroke(Rect::from_min_max(at(0.2, 0.2), at(0.8, 0.8)), CornerRadius::ZERO, s, StrokeKind::Middle);
            for (a, b) in [((0.2, 0.0), (0.2, 0.12)), ((0.0, 0.2), (0.12, 0.2)), ((0.8, 0.88), (0.8, 1.0)), ((0.88, 0.8), (1.0, 0.8))] {
                p.line_segment([at(a.0, a.1), at(b.0, b.1)], thin);
            }
        }
    }
}
