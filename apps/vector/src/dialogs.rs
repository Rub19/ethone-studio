use crate::model::{self, Geom, Item, Node};
use crate::panels::{preset_card, row_label};
use crate::{Dlg, Doc, PRESETS, RecolorSt, TraceSt, VectorApp, apply_affine, c32, geom, hsl_adjust, map_colors};
use eframe::egui::{self, Align, Layout, RichText, Ui};
use studio_ui::pro::{self, FAINT, INK_STRONG, MUTED, semibold};

fn title(ui: &mut Ui, t: &str, sub: &str) {
    ui.label(RichText::new(t).font(semibold(19.0)).color(INK_STRONG));
    if !sub.is_empty() {
        ui.label(RichText::new(sub).size(12.5).color(MUTED));
    }
    ui.add_space(12.0);
}

fn buttons(ui: &mut Ui, ok: &str) -> (bool, bool) {
    let (mut a, mut c) = (false, false);
    ui.add_space(14.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        a = pro::primary(ui, ok).clicked();
        c = pro::secondary(ui, "Annuler").clicked();
    });
    (a, c)
}

fn num(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) {
    ui.horizontal(|ui| {
        row_label(ui, label, 150.0);
        ui.add(egui::DragValue::new(v).range(range).speed(0.5).suffix(suffix));
    });
}

fn slide(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) {
    ui.label(RichText::new(label).size(12.5).color(MUTED));
    ui.spacing_mut().slider_width = ui.available_width() - 80.0;
    ui.add(egui::Slider::new(v, range).suffix(suffix));
}

fn edit_paths(it: &mut Item, f: &dyn Fn(&[Node], bool) -> Vec<Node>) -> bool {
    if matches!(it.geom, Geom::Text { .. } | Geom::Image { .. }) || !it.to_path() {
        return false;
    }
    if let Geom::Path { nodes, closed } = &mut it.geom {
        *nodes = f(nodes, *closed);
    }
    for s in &mut it.sub {
        *s = f(s, true);
    }
    true
}

impl VectorApp {
    pub(crate) fn apply_dlg(&mut self, d: &Dlg, copy: bool) {
        if copy && !matches!(d, Dlg::Blend { .. } | Dlg::Adjust { .. } | Dlg::Repeat { .. } | Dlg::Offset { .. }) {
            let items: Vec<Item> = self.selection().iter().map(|i| self.doc.items[*i].clone()).collect();
            self.insert_copies(items, [0.0, 0.0]);
        }
        let all = self.selection();
        let rot = |deg: f32| {
            let (s, c) = deg.to_radians().sin_cos();
            [c, s, -s, c]
        };
        match *d {
            Dlg::Move { dx, dy } => all.iter().for_each(|i| self.doc.items[*i].translate([dx, dy])),
            Dlg::Scale { sx, sy, uniform } => self.transform_sel([sx / 100.0, 0.0, 0.0, if uniform { sx } else { sy } / 100.0], None),
            Dlg::Rotate { deg } => self.transform_sel(rot(deg), None),
            Dlg::Reflect { axis, angle } => {
                let (s, c) = (2.0 * angle.to_radians()).sin_cos();
                self.transform_sel(match axis {
                    0 => [1.0, 0.0, 0.0, -1.0],
                    1 => [-1.0, 0.0, 0.0, 1.0],
                    _ => [c, s, s, -c],
                }, None)
            }
            Dlg::Shear { deg, vertical } => {
                let t = deg.to_radians().tan();
                self.transform_sel(if vertical { [1.0, t, 0.0, 1.0] } else { [1.0, 0.0, t, 1.0] }, None)
            }
            Dlg::Each { sx, sy, dx, dy, deg } => {
                let r = rot(deg);
                let m = [r[0] * sx / 100.0, r[1] * sx / 100.0, r[2] * sy / 100.0, r[3] * sy / 100.0];
                for &i in &all {
                    let c = self.doc.items[i].bbox().center();
                    apply_affine(&mut self.doc.items[i], m, c, self.o.scale_strokes);
                    self.doc.items[i].translate([dx, dy]);
                }
            }
            Dlg::Offset { d } => {
                let mut new = Vec::new();
                for &i in all.iter().rev() {
                    if let Some(mut o) = geom::offset(&self.doc.items[i], d) {
                        o.name = format!("{} décalé", self.doc.items[i].name);
                        o.group = 0;
                        let at = if d > 0.0 { i } else { i + 1 };
                        self.doc.items.insert(at, o);
                        new.push(at);
                    }
                }
                if !new.is_empty() {
                    let fixed: Vec<usize> = new.iter().enumerate().map(|(k, a)| a + new[..k].iter().filter(|b| **b <= *a).count()).collect();
                    self.set_selection(fixed);
                }
            }
            Dlg::Simplify { tol, smooth } => {
                for &i in &all {
                    edit_paths(&mut self.doc.items[i], &|n, closed| {
                        let mut pts = model::flatten(n, closed, 16);
                        if closed {
                            pts = model::simplify_loop(&pts, tol);
                        } else {
                            pts = model::simplify(&pts, tol).iter().map(|k| k.p).collect();
                        }
                        if smooth { model::fit(&pts, closed, 50.0) } else { pts.into_iter().map(Node::corner).collect() }
                    });
                }
            }
            Dlg::Round { r } => all.iter().for_each(|i| {
                edit_paths(&mut self.doc.items[*i], &|n, c| model::round_corners(n, c, r));
            }),
            Dlg::Zigzag { size, ridges, smooth } => all.iter().for_each(|i| {
                edit_paths(&mut self.doc.items[*i], &|n, c| model::zigzag(n, c, size, ridges as usize, smooth, smooth));
            }),
            Dlg::Roughen { size, detail, smooth } => all.iter().for_each(|i| {
                let seed = *i as u32 + 7;
                edit_paths(&mut self.doc.items[*i], &|n, c| model::roughen(n, c, size, detail, smooth, seed));
            }),
            Dlg::Pucker { amount } => all.iter().for_each(|i| {
                edit_paths(&mut self.doc.items[*i], &|n, c| model::pucker(n, c, -amount / 100.0));
            }),
            Dlg::Twist { deg } => all.iter().for_each(|i| {
                edit_paths(&mut self.doc.items[*i], &|n, c| model::twist(n, c, deg));
            }),
            Dlg::Scribble { angle, spacing, jitter } => {
                for &i in &all {
                    if let Some(s) = geom::scribble(&self.doc.items[i], angle, spacing.max(1.0), jitter) {
                        self.doc.items[i] = s;
                    }
                }
            }
            Dlg::Blend { steps } => {
                if all.len() < 2 {
                    return;
                }
                let g = self.next_group();
                for &i in &all {
                    self.doc.items[i].group = g;
                }
                let items: Vec<Item> = all.iter().map(|i| self.doc.items[*i].clone()).collect();
                let mut offset = 0;
                for k in 0..items.len() - 1 {
                    let mids = model::blend_items(&items[k], &items[k + 1], steps as usize);
                    let at = all[k] + offset + 1;
                    let n = mids.len();
                    for (j, mut m) in mids.into_iter().enumerate() {
                        m.group = g;
                        m.layer = items[k].layer;
                        self.doc.items.insert(at + j, m);
                    }
                    offset += n;
                }
                self.set_selection((0..self.doc.items.len()).filter(|i| self.doc.items[*i].group == g).collect());
            }
            Dlg::Adjust { hue, sat, light } => {
                let targets: Vec<usize> = if all.is_empty() { (0..self.doc.items.len()).collect() } else { all };
                for i in targets {
                    map_colors(&mut self.doc.items[i], &|c| hsl_adjust(c, hue, sat, light));
                }
            }
            Dlg::Repeat { n, dx, dy, rot: r } => {
                for &i in all.iter().rev() {
                    let base = self.doc.items[i].clone();
                    let center = base.bbox().center();
                    for k in 1..=n {
                        let mut c = base.clone();
                        c.name = format!("{} {}", base.name, k + 1);
                        if r != 0.0 {
                            c.rotate_about(r * k as f32, center);
                        }
                        c.translate([dx * k as f32, dy * k as f32]);
                        self.doc.items.insert(i + k as usize, c);
                    }
                }
            }
        }
    }

    pub(crate) fn dlg_sheet(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.dlg.take() else { return };
        let (heading, sub, transform) = match st.kind {
            Dlg::Move { .. } => ("Déplacement", "Décale la sélection d'une distance précise", true),
            Dlg::Scale { .. } => ("Mise à l'échelle", "Agrandit ou réduit depuis le point de référence", true),
            Dlg::Rotate { .. } => ("Rotation", "Fait pivoter autour du point de référence", true),
            Dlg::Reflect { .. } => ("Miroir", "Retourne la sélection par rapport à un axe", true),
            Dlg::Shear { .. } => ("Inclinaison", "Penche la sélection", true),
            Dlg::Each { .. } => ("Transformer chaque", "Chaque objet autour de son propre centre", true),
            Dlg::Offset { .. } => ("Décalage du tracé", "Crée un contour parallèle, à l'extérieur ou à l'intérieur", false),
            Dlg::Simplify { .. } => ("Simplifier", "Retire des points en gardant l'allure", false),
            Dlg::Round { .. } => ("Arrondis", "Arrondit les angles vifs", false),
            Dlg::Zigzag { smooth: false, .. } => ("Zigzag", "Transforme le tracé en dents de scie", false),
            Dlg::Zigzag { .. } => ("Ondulation", "Transforme le tracé en vagues régulières", false),
            Dlg::Roughen { .. } => ("Distorsion aléatoire", "Rend le tracé irrégulier, façon dessin à la main", false),
            Dlg::Pucker { .. } => ("Contraction et dilatation", "Creuse ou gonfle les côtés", false),
            Dlg::Twist { .. } => ("Torsion", "Fait tourner davantage le centre que les bords", false),
            Dlg::Scribble { .. } => ("Griffonnage", "Remplace le fond par des hachures dessinées", false),
            Dlg::Blend { .. } => ("Fusion", "Crée des étapes intermédiaires entre les objets", false),
            Dlg::Adjust { .. } => ("Régler les couleurs", "Teinte, saturation et luminosité de la sélection", false),
            Dlg::Repeat { .. } => ("Répétition", "Une série de copies décalées, tournées ou non", false),
        };
        let mut ok = false;
        let mut cancel = false;
        let keep = pro::sheet(ctx, "dlg", 380.0, |ui| {
            title(ui, heading, sub);
            ui.spacing_mut().item_spacing.y = 8.0;
            match &mut st.kind {
                Dlg::Move { dx, dy } => {
                    num(ui, "Horizontal", dx, -20000.0..=20000.0, " px");
                    num(ui, "Vertical", dy, -20000.0..=20000.0, " px");
                    let (mut d, mut a) = ((dx.powi(2) + dy.powi(2)).sqrt(), dy.atan2(*dx).to_degrees());
                    let c1 = ui.horizontal(|ui| {
                        row_label(ui, "Distance", 150.0);
                        ui.add(egui::DragValue::new(&mut d).range(0.0..=20000.0).suffix(" px")).changed()
                    }).inner;
                    let c2 = ui.horizontal(|ui| {
                        row_label(ui, "Angle", 150.0);
                        ui.add(egui::DragValue::new(&mut a).range(-360.0..=360.0).suffix("°")).changed()
                    }).inner;
                    if c1 || c2 {
                        *dx = d * a.to_radians().cos();
                        *dy = d * a.to_radians().sin();
                    }
                }
                Dlg::Scale { sx, sy, uniform } => {
                    ui.checkbox(uniform, "Proportionnelle");
                    num(ui, if *uniform { "Échelle" } else { "Horizontale" }, sx, 1.0..=5000.0, " %");
                    if !*uniform {
                        num(ui, "Verticale", sy, 1.0..=5000.0, " %");
                    }
                    ui.checkbox(&mut self.o.scale_strokes, "Mettre à l'échelle les contours et effets");
                    ui.horizontal(|ui| {
                        for p in [25.0, 50.0, 150.0, 200.0] {
                            if pro::chip(ui, &format!("{p} %")).clicked() {
                                *sx = p;
                                *sy = p;
                            }
                        }
                    });
                }
                Dlg::Rotate { deg } => {
                    num(ui, "Angle", deg, -360.0..=360.0, "°");
                    ui.horizontal(|ui| {
                        for p in [-90.0, -45.0, 15.0, 45.0, 90.0, 180.0] {
                            if pro::chip(ui, &format!("{p}°")).clicked() {
                                *deg = p;
                            }
                        }
                    });
                }
                Dlg::Reflect { axis, angle } => {
                    pro::segmented(ui, axis, &["Horizontal", "Vertical", "Angle"]);
                    if *axis == 2 {
                        num(ui, "Angle de l'axe", angle, -180.0..=180.0, "°");
                    }
                }
                Dlg::Shear { deg, vertical } => {
                    num(ui, "Angle d'inclinaison", deg, -80.0..=80.0, "°");
                    let mut v = *vertical as usize;
                    if pro::segmented(ui, &mut v, &["Axe horizontal", "Axe vertical"]) {
                        *vertical = v == 1;
                    }
                }
                Dlg::Each { sx, sy, dx, dy, deg } => {
                    num(ui, "Échelle horizontale", sx, 1.0..=5000.0, " %");
                    num(ui, "Échelle verticale", sy, 1.0..=5000.0, " %");
                    num(ui, "Déplacement horizontal", dx, -5000.0..=5000.0, " px");
                    num(ui, "Déplacement vertical", dy, -5000.0..=5000.0, " px");
                    num(ui, "Rotation", deg, -360.0..=360.0, "°");
                }
                Dlg::Offset { d } => {
                    slide(ui, "Décalage (négatif = vers l'intérieur)", d, -200.0..=200.0, " px");
                    ui.label(RichText::new("Les angles du résultat sont arrondis.").size(12.0).color(FAINT));
                }
                Dlg::Simplify { tol, smooth } => {
                    slide(ui, "Précision (plus haut = moins de points)", tol, 0.1..=40.0, " px");
                    ui.checkbox(smooth, "Courbes lisses");
                    let pts: usize = self.dlg_points(&st.sel, &st.base);
                    let now: usize = st.sel.iter().filter_map(|i| self.doc.items.get(*i)).map(|it| match &it.geom {
                        Geom::Path { nodes, .. } => nodes.len() + it.sub.iter().map(|s| s.len()).sum::<usize>(),
                        _ => 0,
                    }).sum();
                    ui.label(RichText::new(format!("Points : {pts} → {now}")).size(12.5).color(MUTED));
                }
                Dlg::Round { r } => slide(ui, "Rayon", r, 0.5..=300.0, " px"),
                Dlg::Zigzag { size, ridges, smooth } => {
                    slide(ui, "Amplitude", size, 0.5..=200.0, " px");
                    ui.label(RichText::new("Nombre d'ondes").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(ridges, 2..=300));
                    let mut s = *smooth as usize;
                    if pro::segmented(ui, &mut s, &["Pointes", "Arrondis"]) {
                        *smooth = s == 1;
                    }
                }
                Dlg::Roughen { size, detail, smooth } => {
                    slide(ui, "Amplitude", size, 0.2..=100.0, " px");
                    slide(ui, "Détail (points par 100 px)", detail, 1.0..=100.0, "");
                    let mut s = *smooth as usize;
                    if pro::segmented(ui, &mut s, &["Angles", "Arrondis"]) {
                        *smooth = s == 1;
                    }
                }
                Dlg::Pucker { amount } => slide(ui, "Contraction ← → Dilatation", amount, -100.0..=100.0, " %"),
                Dlg::Twist { deg } => slide(ui, "Angle", deg, -720.0..=720.0, "°"),
                Dlg::Scribble { angle, spacing, jitter } => {
                    slide(ui, "Angle des hachures", angle, -180.0..=180.0, "°");
                    slide(ui, "Espacement", spacing, 1.0..=80.0, " px");
                    slide(ui, "Variation", jitter, 0.0..=40.0, " px");
                    ui.horizontal(|ui| {
                        for (n, a, s, j) in [("Serré", 30.0, 4.0, 1.0), ("Croquis", 45.0, 8.0, 6.0), ("Large", 0.0, 16.0, 3.0), ("Nerveux", 70.0, 6.0, 12.0)] {
                            if pro::chip(ui, n).clicked() {
                                (*angle, *spacing, *jitter) = (a, s, j);
                            }
                        }
                    });
                }
                Dlg::Blend { steps } => {
                    ui.label(RichText::new("Étapes intermédiaires").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(steps, 1..=100));
                    ui.label(RichText::new("Formes et couleurs se fondent progressivement.").size(12.0).color(FAINT));
                }
                Dlg::Adjust { hue, sat, light } => {
                    slide(ui, "Teinte", hue, -180.0..=180.0, "°");
                    slide(ui, "Saturation", sat, -100.0..=100.0, " %");
                    slide(ui, "Luminosité", light, -100.0..=100.0, " %");
                }
                Dlg::Repeat { n, dx, dy, rot } => {
                    ui.horizontal(|ui| {
                        row_label(ui, "Copies", 150.0);
                        ui.add(egui::DragValue::new(n).range(1..=200));
                    });
                    num(ui, "Décalage X", dx, -5000.0..=5000.0, " px");
                    num(ui, "Décalage Y", dy, -5000.0..=5000.0, " px");
                    num(ui, "Rotation par copie", rot, -360.0..=360.0, "°");
                    if pro::chip(ui, "Rosace : 12 copies en cercle").clicked() {
                        (*n, *dx, *dy, *rot) = (11, 0.0, 0.0, 30.0);
                    }
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.checkbox(&mut st.preview, "Aperçu");
                if transform || matches!(st.kind, Dlg::Simplify { .. } | Dlg::Round { .. } | Dlg::Zigzag { .. } | Dlg::Roughen { .. } | Dlg::Pucker { .. } | Dlg::Twist { .. }) {
                    ui.checkbox(&mut st.copy, "Appliquer sur une copie");
                }
            });
            (ok, cancel) = buttons(ui, "OK");
        });
        let key = (st.kind.clone(), st.copy);
        if st.preview && st.applied.as_ref() != Some(&key) && !cancel {
            self.doc = st.base.clone();
            self.set_selection(st.sel.clone());
            self.apply_dlg(&key.0, key.1);
            st.applied = Some(key.clone());
        } else if !st.preview && st.applied.is_some() {
            self.doc = st.base.clone();
            self.set_selection(st.sel.clone());
            st.applied = None;
        }
        if ok {
            if st.applied.as_ref() != Some(&key) {
                self.doc = st.base.clone();
                self.set_selection(st.sel.clone());
                self.apply_dlg(&key.0, key.1);
            }
            if transform {
                self.last_tf = Some(key);
            }
            if self.doc == st.base {
                self.undo.pop();
            }
        } else if cancel || !keep {
            self.doc = st.base;
            self.set_selection(st.sel);
            self.undo.pop();
        } else {
            self.dlg = Some(st);
        }
    }

    fn dlg_points(&self, sel: &[usize], base: &Doc) -> usize {
        sel.iter().filter_map(|i| base.items.get(*i)).map(|it| it.path_nodes().map_or(0, |(n, _)| n.len()) + it.sub.iter().map(|s| s.len()).sum::<usize>()).sum()
    }

    pub(crate) fn export_sheet(&mut self, ctx: &egui::Context) {
        let Some(mut o) = self.export.take() else { return };
        let (mut go, mut cancel) = (false, false);
        let parts = self.export_area(o.scope);
        let keep = pro::sheet(ctx, "export", 440.0, |ui| {
            title(ui, "Exporter", "Formats web, impression et fichiers vectoriels");
            ui.spacing_mut().item_spacing.y = 10.0;
            pro::segmented(ui, &mut o.format, &["PNG", "JPEG", "SVG", "PDF"]);
            ui.horizontal(|ui| {
                row_label(ui, "Contenu", 90.0);
                pro::segmented(ui, &mut o.scope, &["Plan actif", "Tous les plans", "Sélection"]);
            });
            if o.format <= 1 {
                ui.horizontal(|ui| {
                    row_label(ui, "Échelle", 90.0);
                    for s in [0.5, 1.0, 2.0, 3.0, 4.0] {
                        let on = (o.scale - s).abs() < 1e-3;
                        if pro::icon_button(ui, 34.0, on, &format!("×{s}"), |p, r, c| {
                            p.text(r.center(), egui::Align2::CENTER_CENTER, format!("{s}×"), egui::FontId::proportional(11.5), c);
                        }).clicked() {
                            o.scale = s;
                        }
                    }
                    ui.add(egui::DragValue::new(&mut o.scale).range(0.05..=16.0).speed(0.05).prefix("×"));
                });
                if let Some((_, r, _)) = parts.first() {
                    ui.label(RichText::new(format!("Taille : {:.0} × {:.0} px{}", r[2] * o.scale, r[3] * o.scale, if parts.len() > 1 { format!(" · {} fichiers", parts.len()) } else { String::new() })).size(12.5).color(MUTED));
                }
            }
            match o.format {
                0 => {
                    ui.checkbox(&mut o.transparent, "Fond transparent");
                }
                1 => {
                    ui.label(RichText::new("Qualité").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(&mut o.quality, 10..=100).suffix(" %"));
                }
                2 => {
                    ui.checkbox(&mut o.minify, "SVG compact (sans retours à la ligne)");
                    ui.label(RichText::new("Texte, dégradés, effets et images intégrés restent modifiables.").size(12.0).color(FAINT));
                }
                _ => {
                    ui.label(RichText::new("Un PDF vectoriel, une page par plan de travail, texte réel.").size(12.0).color(FAINT));
                }
            }
            (go, cancel) = buttons(ui, "Exporter…");
        });
        if go {
            if o.format == 3 && o.scope == 0 && !self.doc.boards.is_empty() {
                o.scope = 1;
            }
            self.run_export(&o);
        } else if keep && !cancel {
            self.export = Some(o);
        }
    }

    pub(crate) fn open_trace(&mut self) {
        let Some(i) = self.sel.filter(|i| matches!(self.doc.items[*i].geom, Geom::Image { .. })) else {
            self.info = Some("Vectorisation : sélectionne une image".into());
            return;
        };
        self.trace = Some(TraceSt { item: i, opts: geom::TraceOpts { mode: 0, threshold: 128.0, colors: 6, noise: 8.0, precision: 500, corners: 40.0, ignore_white: true }, keep_image: false });
    }

    pub(crate) fn trace_sheet(&mut self, ctx: &egui::Context) {
        let Some(mut t) = self.trace.take() else { return };
        let (mut go, mut cancel) = (false, false);
        let keep = pro::sheet(ctx, "trace", 420.0, |ui| {
            title(ui, "Vectorisation dynamique", "Transforme une image en formes vectorielles");
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.horizontal_wrapped(|ui| {
                for (n, m, c, th, no) in [("Logo N&B", 0, 2, 128.0, 8.0), ("Silhouette", 0, 2, 200.0, 30.0), ("Croquis", 0, 2, 90.0, 4.0), ("Gris 4 niveaux", 1, 4, 128.0, 6.0), ("Affiche 6 couleurs", 2, 6, 128.0, 12.0), ("Photo 16 couleurs", 2, 16, 128.0, 4.0)] {
                    if pro::chip(ui, n).clicked() {
                        t.opts.mode = m;
                        t.opts.colors = c;
                        t.opts.threshold = th;
                        t.opts.noise = no;
                    }
                }
            });
            pro::segmented(ui, &mut t.opts.mode, &["Noir et blanc", "Niveaux de gris", "Couleurs"]);
            if t.opts.mode == 0 {
                slide(ui, "Seuil", &mut t.opts.threshold, 1.0..=254.0, "");
            } else {
                ui.label(RichText::new(if t.opts.mode == 1 { "Niveaux" } else { "Couleurs" }).size(12.5).color(MUTED));
                ui.add(egui::Slider::new(&mut t.opts.colors, 2..=16));
            }
            ui.label(RichText::new("Précision").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(&mut t.opts.precision, 100..=1200).suffix(" px"));
            slide(ui, "Bruit ignoré", &mut t.opts.noise, 0.0..=200.0, " px²");
            slide(ui, "Courbes (plus haut = plus arrondi)", &mut t.opts.corners, 0.0..=90.0, "°");
            ui.checkbox(&mut t.opts.ignore_white, "Ignorer le blanc");
            ui.checkbox(&mut t.keep_image, "Conserver l'image d'origine");
            (go, cancel) = buttons(ui, "Vectoriser");
        });
        if go {
            let Some(it) = self.doc.items.get(t.item).cloned() else { return };
            let Geom::Image { x, y, w, h, href } = &it.geom else { return };
            let Some(img) = geom::image_rgba(href) else {
                self.error = Some("Image illisible".into());
                return;
            };
            let out = geom::trace(&img, [*x, *y, *w, *h], &t.opts);
            if out.is_empty() {
                self.info = Some("Vectorisation : aucune forme trouvée, baisse le bruit ou change le seuil".into());
                return;
            }
            self.checkpoint();
            let g = self.next_group();
            let n = out.len();
            let at = t.item + 1;
            for (k, mut o) in out.into_iter().enumerate() {
                o.group = g;
                o.layer = it.layer;
                self.doc.items.insert(at + k, o);
            }
            let mut v: Vec<usize> = (at..at + n).collect();
            if !t.keep_image {
                self.doc.items.remove(t.item);
                v = v.into_iter().map(|i| i - 1).collect();
            }
            self.set_selection(v);
            self.info = Some(format!("Vectorisation : {n} couche(s) de couleur"));
        } else if keep && !cancel {
            self.trace = Some(t);
        }
    }

    pub(crate) fn open_recolor(&mut self) {
        let sel = self.selection();
        let idx: Vec<usize> = if sel.is_empty() { (0..self.doc.items.len()).collect() } else { sel };
        let mut cols: Vec<[u8; 4]> = Vec::new();
        for &i in &idx {
            let it = &self.doc.items[i];
            for c in [it.fill, it.fill2, it.stroke].into_iter().flatten().chain(it.stops.iter().map(|s| s.c)).chain(it.mesh.iter().map(|m| m.c)) {
                if !cols.contains(&c) {
                    cols.push(c);
                }
            }
        }
        if cols.is_empty() {
            self.info = Some("Aucune couleur à recolorer".into());
            return;
        }
        cols.truncate(40);
        self.checkpoint();
        self.recolor = Some(RecolorSt { pairs: cols.iter().map(|c| (*c, *c)).collect(), base: self.doc.clone() });
    }

    pub(crate) fn recolor_sheet(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.recolor.take() else { return };
        let (mut ok, mut cancel) = (false, false);
        let mut shift = 0.0f32;
        let keep = pro::sheet(ctx, "recolor", 420.0, |ui| {
            title(ui, "Recolorer l'illustration", "Remplace chaque couleur, ou fais tourner toute la palette");
            ui.horizontal_wrapped(|ui| {
                for (n, h) in [("Teinte −60°", -60.0), ("−30°", -30.0), ("+30°", 30.0), ("+60°", 60.0), ("Complémentaires", 180.0)] {
                    if pro::chip(ui, n).clicked() {
                        shift = h;
                    }
                }
                if pro::chip(ui, "Réinitialiser").clicked() {
                    for p in &mut st.pairs {
                        p.1 = p.0;
                    }
                }
            });
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                for (k, (from, to)) in st.pairs.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(44.0, 22.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, 5, c32(*from));
                        ui.label(RichText::new("→").color(MUTED));
                        let mut c = c32(*to);
                        ui.push_id(k, |ui| {
                            if ui.color_edit_button_srgba(&mut c).changed() {
                                *to = c.to_srgba_unmultiplied();
                            }
                        });
                        ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}", to[0], to[1], to[2])).monospace().size(12.0).color(MUTED));
                    });
                }
            });
            (ok, cancel) = buttons(ui, "OK");
        });
        if shift != 0.0 {
            for p in &mut st.pairs {
                p.1 = hsl_adjust(p.1, shift, 0.0, 0.0);
            }
        }
        let mut d = st.base.clone();
        let pairs = st.pairs.clone();
        let sel = self.selection();
        for (i, it) in d.items.iter_mut().enumerate() {
            if sel.is_empty() || sel.contains(&i) {
                map_colors(it, &|c| pairs.iter().find(|p| p.0 == c).map_or(c, |p| p.1));
            }
        }
        if d != self.doc {
            self.doc = d;
        }
        if cancel || (!keep && !ok) {
            self.doc = st.base;
            self.undo.pop();
        } else if !ok {
            self.recolor = Some(st);
        }
    }

    pub(crate) fn new_doc_sheet(&mut self, ctx: &egui::Context) {
        let Some(mut nd) = self.new_doc.take() else { return };
        let mut create = false;
        let mut cancel = false;
        let keep = pro::sheet(ctx, "new", 780.0, |ui| {
            title(ui, "Nouveau document", "Choisis un format ou saisis une taille");
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(560.0);
                    for (ri, row) in PRESETS.chunks(4).enumerate() {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            for (k, (name, kind, w, h)) in row.iter().enumerate() {
                                let idx = ri * 4 + k;
                                if preset_card(ui, name, &format!("{w} × {h} · {kind}"), *w, *h, nd.preset == idx, 130.0).clicked() {
                                    nd.preset = idx;
                                    nd.w = *w;
                                    nd.h = *h;
                                }
                            }
                        });
                        ui.add_space(10.0);
                    }
                });
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Largeur").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut nd.w).range(1.0..=20000.0).suffix(" px"));
                    ui.label(RichText::new("Hauteur").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut nd.h).range(1.0..=20000.0).suffix(" px"));
                    if ui.button("⇄ Orientation").clicked() {
                        std::mem::swap(&mut nd.w, &mut nd.h);
                    }
                    ui.add_space(8.0);
                    ui.checkbox(&mut nd.transparent, "Fond transparent");
                });
            });
            let (a, c) = buttons(ui, "Créer");
            create = a;
            cancel = c;
        });
        if create {
            let mut d = Doc::new(nd.w, nd.h);
            if nd.transparent {
                d.background = None;
            }
            self.set_doc(d, None);
        } else if keep && !cancel {
            self.new_doc = Some(nd);
        }
    }
}
