use crate::filters::{Kind, map};
use crate::{Pixel, Sheet};
use eframe::egui::{self, Align2, Color32, CornerRadius, Frame, Margin, Rect, RichText, Stroke, TextureOptions, Ui, pos2};
use image::{RgbaImage, imageops};
use studio_ui::pro::{CHROME, INK, LINE, MUTED};

pub struct Cube {
    n: usize,
    data: Vec<[f32; 3]>,
    min: [f32; 3],
    max: [f32; 3],
}

impl Cube {
    pub fn parse(text: &str) -> Result<Cube, String> {
        let (mut n, mut data, mut min, mut max) = (0usize, Vec::new(), [0.0f32; 3], [1.0f32; 3]);
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let mut it = line.split_whitespace();
            let head = it.next().unwrap_or("");
            let nums = |it: std::str::SplitWhitespace| -> Result<[f32; 3], String> {
                let v: Vec<f32> = it.map(|t| t.parse::<f32>().map_err(|_| format!("nombre invalide : {t}"))).collect::<Result<_, _>>()?;
                v.get(..3).map(|s| [s[0], s[1], s[2]]).ok_or_else(|| "ligne incomplète".to_string())
            };
            match head {
                "LUT_3D_SIZE" => n = it.next().and_then(|t| t.parse().ok()).ok_or("taille invalide")?,
                "DOMAIN_MIN" => min = nums(it)?,
                "DOMAIN_MAX" => max = nums(it)?,
                "TITLE" | "LUT_1D_SIZE" | "LUT_3D_INPUT_RANGE" => {
                    if head == "LUT_1D_SIZE" {
                        return Err("les LUT 1D ne sont pas prises en charge".into());
                    }
                }
                _ => data.push(nums(line.split_whitespace())?),
            }
        }
        if n < 2 || data.len() != n * n * n {
            return Err(format!("{} valeurs pour une taille {n}", data.len()));
        }
        Ok(Cube { n, data, min, max })
    }

    pub fn apply(&self, c: [f32; 3]) -> [f32; 3] {
        let n = self.n;
        let f: [f32; 3] = std::array::from_fn(|k| ((c[k] - self.min[k]) / (self.max[k] - self.min[k]).max(1e-6)).clamp(0.0, 1.0) * (n - 1) as f32);
        let i0: [usize; 3] = f.map(|v| (v.floor() as usize).min(n - 2));
        let t: [f32; 3] = std::array::from_fn(|k| f[k] - i0[k] as f32);
        let at = |r: usize, g: usize, b: usize| self.data[r + g * n + b * n * n];
        let mut o = [0f32; 3];
        for (dr, wr) in [(0, 1.0 - t[0]), (1, t[0])] {
            for (dg, wg) in [(0, 1.0 - t[1]), (1, t[1])] {
                for (db, wb) in [(0, 1.0 - t[2]), (1, t[2])] {
                    let v = at(i0[0] + dr, i0[1] + dg, i0[2] + db);
                    for k in 0..3 {
                        o[k] += v[k] * wr * wg * wb;
                    }
                }
            }
        }
        o
    }
}

fn shrink(img: &RgbaImage, max_side: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let k = (max_side as f32 / w.max(h) as f32).min(1.0);
    if k >= 1.0 { img.clone() } else { imageops::resize(img, ((w as f32 * k) as u32).max(1), ((h as f32 * k) as u32).max(1), imageops::FilterType::Triangle) }
}

impl Pixel {
    fn history_frames(&mut self, max_side: u32) -> Vec<RgbaImage> {
        let (rows, cur) = self.doc.history();
        let mut out = Vec::new();
        for k in 0..rows.len() {
            self.doc.jump_history(k);
            out.push(shrink(&self.doc.flatten(), max_side));
        }
        self.doc.jump_history(cur);
        out
    }

    fn stem(&self) -> String {
        std::path::Path::new(&self.doc.name).file_stem().map_or("image".into(), |s| s.to_string_lossy().into_owned())
    }

    pub fn toggle_compare(&mut self, ctx: &egui::Context) {
        if self.compare.take().is_some() {
            return;
        }
        let (_, cur) = self.doc.history();
        self.doc.jump_history(0);
        let before = self.doc.flatten();
        self.doc.jump_history(cur);
        let (w, h) = (self.doc.width(), self.doc.height());
        if before.dimensions() != (w, h) {
            self.error = Some("La taille de l'image a changé depuis l'ouverture : comparaison impossible".into());
            return;
        }
        let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], before.as_raw());
        self.compare = Some((ctx.load_texture("compare-before", ci, TextureOptions::LINEAR), 0.5));
    }

    pub fn export_timelapse(&mut self, gif: bool) {
        if self.doc.history().0.len() < 2 {
            self.error = Some("Pas encore d'étapes dans l'historique pour un timelapse".into());
            return;
        }
        let stem = self.stem();
        if gif {
            let Some(path) = rfd::FileDialog::new().add_filter("GIF animé", &["gif"]).set_file_name(format!("{stem} - timelapse.gif")).save_file() else { return };
            let frames = self.history_frames(720);
            let n = frames.len();
            let res = std::fs::File::create(&path).map_err(|e| e.to_string()).and_then(|f| {
                let mut enc = image::codecs::gif::GifEncoder::new_with_speed(f, 10);
                enc.set_repeat(image::codecs::gif::Repeat::Infinite).map_err(|e| e.to_string())?;
                for (i, fr) in frames.into_iter().enumerate() {
                    let ms = if i + 1 == n { 1800 } else { 400 };
                    enc.encode_frame(image::Frame::from_parts(fr, 0, 0, image::Delay::from_numer_denom_ms(ms, 1))).map_err(|e| e.to_string())?;
                }
                Ok(())
            });
            if let Err(e) = res {
                self.error = Some(format!("Timelapse : {e}"));
            }
        } else {
            let Some(dir) = rfd::FileDialog::new().set_title("Dossier des étapes").pick_folder() else { return };
            for (i, fr) in self.history_frames(4096).into_iter().enumerate() {
                if let Err(e) = fr.save(dir.join(format!("{stem} - étape {:03}.png", i + 1))) {
                    self.error = Some(format!("Étapes : {e}"));
                    return;
                }
            }
        }
    }

    pub fn upscale(&mut self, f: u32) {
        let (w, h) = (self.doc.width() * f, self.doc.height() * f);
        if w as u64 * h as u64 > 300_000_000 {
            self.error = Some("Image trop grande pour être agrandie davantage".into());
            return;
        }
        self.doc.set_selection(None, None);
        self.doc.next_label = Some(format!("Agrandir ×{f} (super-résolution)"));
        self.doc.transform(|img| {
            let big = imageops::resize(img, w, h, imageops::FilterType::Lanczos3);
            Kind::SmartSharpen.apply(&big, &[70.0, 0.7 * f as f32, 25.0, 0.0, 10.0])
        });
        self.fit_pending = true;
    }

    pub fn apply_cube_luts(&mut self) {
        let Some(files) = rfd::FileDialog::new().add_filter("LUT 3D", &["cube"]).set_title("Choisis jusqu'à 3 LUT, appliquées dans l'ordre").pick_files() else { return };
        let mut luts = Vec::new();
        for f in files.iter().take(3) {
            match std::fs::read_to_string(f).map_err(|e| e.to_string()).and_then(|t| Cube::parse(&t)) {
                Ok(l) => luts.push(l),
                Err(e) => {
                    self.error = Some(format!("{} : {e}", f.display()));
                    return;
                }
            }
        }
        let base = self.doc.active_target();
        let out = map(&base, |c| luts.iter().fold(c, |c, l| l.apply(c)));
        self.doc.next_label = Some(format!("{} LUT", luts.len()));
        self.doc.checkpoint();
        self.doc.put_filtered(&base, &out);
    }

    pub fn repeat_removal(&mut self, ctx: &egui::Context) {
        match self.last_removal.clone() {
            Some(m) if m.len() == (self.doc.width() * self.doc.height()) as usize => self.start_fill(m, ctx),
            Some(_) => self.error = Some("La dernière suppression vient d'une image d'une autre taille".into()),
            None => self.error = Some("Aucune suppression à répéter : utilise d'abord l'outil Suppression".into()),
        }
    }

    pub fn task_bar(&mut self, ui: &mut Ui, area: Rect) {
        if !self.task_bar || self.hide_panels {
            return;
        }
        let ctx = ui.ctx().clone();
        let has_sel = self.doc.selection.is_some();
        let id = egui::Id::NULL.with("pixel-task-bar");
        egui::Area::new(id).order(egui::Order::Foreground).fixed_pos(pos2(area.center().x, area.bottom() - 16.0)).pivot(Align2::CENTER_BOTTOM).constrain_to(area).show(&ctx, |ui| {
            Frame::new().fill(CHROME).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(12)).inner_margin(Margin::symmetric(8, 5)).shadow(egui::epaint::Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(140) }).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let b = |ui: &mut Ui, t: &str, tip: &str| ui.add(egui::Button::new(RichText::new(t).size(12.0).color(INK)).fill(Color32::TRANSPARENT).stroke(Stroke::NONE).corner_radius(CornerRadius::same(8)).min_size(egui::vec2(0.0, 26.0))).on_hover_text(tip).clicked();
                    if let Some((_, split)) = &mut self.compare {
                        ui.label(RichText::new("Avant").size(12.0).color(MUTED));
                        ui.spacing_mut().slider_width = 160.0;
                        ui.add(egui::Slider::new(split, 0.0..=1.0).show_value(false));
                        ui.label(RichText::new("Après").size(12.0).color(MUTED));
                        if b(ui, "Terminer", "Quitter la comparaison") {
                            self.compare = None;
                        }
                        return;
                    }
                    if has_sel {
                        if b(ui, "Supprimer l'objet", "Remplit la sélection selon le contenu autour") {
                            self.content_fill_selection(&ctx);
                        }
                        if b(ui, "Remplir…", "Couleur, motif ou contenu") {
                            self.sheet = Some(Sheet::Fill { content: 0, opacity: 100.0, blend: 0 });
                        }
                        if b(ui, "Inverser", "Inverser la sélection") {
                            self.doc.invert_selection();
                        }
                        if b(ui, "Rééclairer", "Relight sur la sélection") {
                            self.start_adjust(Kind::Relight);
                        }
                        if b(ui, "Désélectionner", "Ctrl+D") {
                            self.deselect();
                        }
                    } else {
                        for (t, tip, k) in [("Ton auto", "Corrige l'exposition et le contraste", Kind::AutoTone), ("Rééclairer", "Ajoute une ou deux lumières (Relight)", Kind::Relight), ("Retirer la brume", "Supprime brume et effets d'ambiance", Kind::Dehaze), ("Looks", "Trois LUT créatifs empilables", Kind::Looks), ("Imperfections", "Retouche automatique de la peau", Kind::Blemish)] {
                            if b(ui, t, tip) {
                                self.start_adjust(k);
                            }
                        }
                        if b(ui, "Agrandir ×2", "Super-résolution ×2") {
                            self.upscale(2);
                        }
                    }
                    ui.separator();
                    if b(ui, "Comparer", "Avant / après depuis l'ouverture") {
                        self.toggle_compare(&ctx);
                    }
                    if b(ui, "×", "Masquer la barre de tâches (Affichage)") {
                        self.task_bar = false;
                    }
                });
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_identity_and_invert() {
        let mut t = String::from("TITLE \"t\"\nLUT_3D_SIZE 2\n");
        for b in 0..2 {
            for g in 0..2 {
                for r in 0..2 {
                    t += &format!("{} {} {}\n", 1 - r, 1 - g, 1 - b);
                }
            }
        }
        let c = Cube::parse(&t).unwrap();
        let o = c.apply([0.25, 0.5, 1.0]);
        assert!((o[0] - 0.75).abs() < 1e-5 && (o[1] - 0.5).abs() < 1e-5 && o[2].abs() < 1e-5);
        assert!(Cube::parse("LUT_3D_SIZE 3\n0 0 0\n").is_err());
    }
}
