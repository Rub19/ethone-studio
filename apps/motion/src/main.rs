#![windows_subsystem = "windows"]

mod model;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, Frame, Key as EKey, Margin, Pos2, Rect, RichText, Sense,
    Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2, pos2, vec2,
};
use model::{Comp, Content, Ease, Layer, PROPS};
use resvg::{tiny_skia, usvg};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use studio_ui::{BG, CARD, CARD_HOVER, DANGER, MUTED, OK, PANEL, TEXT};

const ACCENT: Color32 = Color32::from_rgb(64, 200, 255);
const LABEL_W: f32 = 190.0;
const ROW_H: f32 = 28.0;

struct Job {
    done: Arc<AtomicUsize>,
    total: usize,
    error: Arc<Mutex<Option<String>>>,
    label: String,
}

struct Motion {
    comp: Comp,
    saved: Comp,
    file: Option<PathBuf>,
    undo: Vec<Comp>,
    redo: Vec<Comp>,
    editing: bool,
    sel: Option<usize>,
    t: f32,
    playing: bool,
    fontdb: Arc<usvg::fontdb::Database>,
    tex: Option<TextureHandle>,
    rendered: Option<(Comp, f32, f32)>,
    drag: Option<(f32, f32, Pos2)>,
    bar_drag: Option<(usize, u8, f32, f32, f32)>,
    job: Option<Job>,
    error: Option<String>,
    title: String,
}

fn render(comp: &Comp, t: f32, scale: f32, db: &Arc<usvg::fontdb::Database>) -> Result<tiny_skia::Pixmap, String> {
    let opt = usvg::Options { fontdb: db.clone(), ..Default::default() };
    let tree = usvg::Tree::from_str(&comp.to_svg(t), &opt).map_err(|e| e.to_string())?;
    let w = ((comp.width as f32 * scale).round() as u32).max(1);
    let h = ((comp.height as f32 * scale).round() as u32).max(1);
    let mut pix = tiny_skia::Pixmap::new(w, h).ok_or("taille invalide")?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
    Ok(pix)
}

fn unpremultiply(pix: &tiny_skia::Pixmap) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(pix.width(), pix.height());
    for (d, s) in img.pixels_mut().zip(pix.pixels()) {
        let c = s.demultiply();
        d.0 = [c.red(), c.green(), c.blue(), c.alpha()];
    }
    img
}

impl Motion {
    fn new(cc: &eframe::CreationContext) -> Self {
        studio_ui::apply_style(&cc.egui_ctx);
        cc.egui_ctx.all_styles_mut(|s| s.visuals.selection.bg_fill = ACCENT);
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        let mut comp = Comp::new();
        let mut title = Layer::new(
            "Titre".into(),
            Content::Text { text: "Ethone Motion".into(), size: 120.0, color: [255, 255, 255, 255], bold: true, family: "Segoe UI".into() },
            960.0,
            540.0,
            5.0,
        );
        title.props[4].add_key(0.0, 0.0);
        title.props[4].add_key(1.0, 100.0);
        title.props[2].add_key(0.0, 80.0);
        title.props[2].add_key(1.0, 100.0);
        comp.layers.push(title);
        Self {
            saved: comp.clone(),
            comp,
            file: None,
            undo: Vec::new(),
            redo: Vec::new(),
            editing: false,
            sel: Some(0),
            t: 0.0,
            playing: false,
            fontdb: Arc::new(db),
            tex: None,
            rendered: None,
            drag: None,
            bar_drag: None,
            job: None,
            error: None,
            title: String::new(),
        }
    }

    fn snap(&self, t: f32) -> f32 {
        (t * self.comp.fps as f32).round() / self.comp.fps as f32
    }

    fn add_layer(&mut self, name: &str, content: Content) {
        let (cx, cy) = (self.comp.width as f32 / 2.0, self.comp.height as f32 / 2.0);
        let mut l = Layer::new(format!("{name} {}", self.comp.layers.len() + 1), content, cx, cy, self.comp.duration);
        l.start = 0.0;
        self.comp.layers.push(l);
        self.sel = Some(self.comp.layers.len() - 1);
    }

    fn add_image(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp"]).pick_file() else { return };
        match image::image_dimensions(&p) {
            Ok((w, h)) => {
                let fit = (self.comp.width as f32 * 0.6 / w as f32).min(self.comp.height as f32 * 0.6 / h as f32).min(1.0);
                let name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or("Image".into());
                self.add_layer(&name, Content::Image { path: p.to_string_lossy().into_owned(), w: w as f32 * fit, h: h as f32 * fit });
            }
            Err(e) => self.error = Some(format!("Image illisible : {e}")),
        }
    }

    fn preset(&mut self, kind: u8) {
        let Some(i) = self.sel else { return };
        let t = self.t;
        let d = 0.6;
        let ch = self.comp.height as f32;
        let l = &mut self.comp.layers[i];
        let x = l.props[0].at(t);
        let y = l.props[1].at(t);
        match kind {
            0 => {
                l.props[4].add_key(t, 0.0);
                l.props[4].add_key(t + d, 100.0);
            }
            1 => {
                l.props[4].add_key(t, 100.0);
                l.props[4].add_key(t + d, 0.0);
            }
            2 => {
                l.props[0].add_key(t, -l.size().0 / 2.0);
                l.props[0].add_key(t + d, x);
                l.props[0].keys.iter_mut().for_each(|k| k.ease = Ease::Out);
            }
            3 => {
                l.props[1].add_key(t, ch + l.size().1 / 2.0);
                l.props[1].add_key(t + d, y);
                l.props[1].keys.iter_mut().for_each(|k| k.ease = Ease::Out);
            }
            4 => {
                l.props[2].add_key(t, 0.0);
                l.props[2].add_key(t + d * 0.7, 115.0);
                l.props[2].add_key(t + d, 100.0);
            }
            _ => {
                l.props[3].add_key(t, 0.0);
                l.props[3].add_key(self.comp.duration, 360.0);
                l.props[3].keys.iter_mut().for_each(|k| k.ease = Ease::Linear);
            }
        }
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.file, as_new) {
            (Some(p), false) => p.clone(),
            _ => match rfd::FileDialog::new().add_filter("Projet Motion", &["emot"]).set_file_name("animation.emot").save_file() {
                Some(p) => p,
                None => return,
            },
        };
        match serde_json::to_vec_pretty(&self.comp).map_err(|e| e.to_string()).and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string())) {
            Ok(()) => {
                self.file = Some(path);
                self.saved = self.comp.clone();
            }
            Err(e) => self.error = Some(format!("Enregistrement : {e}")),
        }
    }

    fn open(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("Projet Motion", &["emot"]).pick_file() else { return };
        match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Comp>(&b).map_err(|e| e.to_string())) {
            Ok(c) => {
                self.saved = c.clone();
                self.comp = c;
                self.file = Some(p);
                self.undo.clear();
                self.redo.clear();
                self.sel = None;
                self.t = 0.0;
            }
            Err(e) => self.error = Some(format!("Ouverture : {e}")),
        }
    }

    fn export(&mut self, gif: bool) {
        let comp = self.comp.clone();
        let db = self.fontdb.clone();
        let total = comp.frames() as usize;
        let done = Arc::new(AtomicUsize::new(0));
        let error = Arc::new(Mutex::new(None));
        let (d2, e2) = (done.clone(), error.clone());
        if gif {
            let Some(path) = rfd::FileDialog::new().add_filter("GIF", &["gif"]).set_file_name("animation.gif").save_file() else { return };
            std::thread::spawn(move || {
                let r = (|| -> Result<(), String> {
                    let scale = (960.0 / comp.width as f32).min(1.0);
                    let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
                    let mut enc = image::codecs::gif::GifEncoder::new_with_speed(std::io::BufWriter::new(file), 10);
                    enc.set_repeat(image::codecs::gif::Repeat::Infinite).map_err(|e| e.to_string())?;
                    for f in 0..comp.frames() {
                        let pix = render(&comp, f as f32 / comp.fps as f32, scale, &db)?;
                        let frame = image::Frame::from_parts(unpremultiply(&pix), 0, 0, image::Delay::from_numer_denom_ms(1000, comp.fps));
                        enc.encode_frame(frame).map_err(|e| e.to_string())?;
                        d2.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(())
                })();
                if let Err(e) = r {
                    *e2.lock().unwrap() = Some(e);
                }
                d2.store(usize::MAX, Ordering::Relaxed);
            });
            self.job = Some(Job { done, total, error, label: "GIF".into() });
        } else {
            let Some(dir) = rfd::FileDialog::new().set_title("Dossier de la séquence PNG").pick_folder() else { return };
            std::thread::spawn(move || {
                let r = (|| -> Result<(), String> {
                    for f in 0..comp.frames() {
                        let pix = render(&comp, f as f32 / comp.fps as f32, 1.0, &db)?;
                        pix.save_png(dir.join(format!("frame_{f:05}.png"))).map_err(|e| e.to_string())?;
                        d2.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(())
                })();
                if let Err(e) = r {
                    *e2.lock().unwrap() = Some(e);
                }
                d2.store(usize::MAX, Ordering::Relaxed);
            });
            self.job = Some(Job { done, total, error, label: "séquence PNG".into() });
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        if sc(M::COMMAND | M::SHIFT, EKey::Z) || sc(M::COMMAND, EKey::Y) {
            if let Some(c) = self.redo.pop() {
                self.undo.push(std::mem::replace(&mut self.comp, c));
            }
        } else if sc(M::COMMAND, EKey::Z)
            && let Some(c) = self.undo.pop()
        {
            self.redo.push(std::mem::replace(&mut self.comp, c));
        }
        if sc(M::COMMAND | M::SHIFT, EKey::S) {
            self.save(true);
        } else if sc(M::COMMAND, EKey::S) {
            self.save(false);
        }
        if sc(M::COMMAND, EKey::O) {
            self.open();
        }
        if sc(M::COMMAND, EKey::D)
            && let Some(i) = self.sel
        {
            let mut l = self.comp.layers[i].clone();
            l.name = format!("{} copie", l.name);
            self.comp.layers.insert(i + 1, l);
            self.sel = Some(i + 1);
        }
        let step = 1.0 / self.comp.fps as f32;
        let (space, left, right, del, home) = ctx.input(|i| {
            (i.key_pressed(EKey::Space), i.key_pressed(EKey::ArrowLeft), i.key_pressed(EKey::ArrowRight), i.key_pressed(EKey::Delete), i.key_pressed(EKey::Home))
        });
        if space {
            self.playing = !self.playing;
        }
        if left {
            self.t = self.snap((self.t - step).max(0.0));
        }
        if right {
            self.t = self.snap((self.t + step).min(self.comp.duration));
        }
        if home {
            self.t = 0.0;
        }
        if del && let Some(i) = self.sel.take() {
            self.comp.layers.remove(i);
        }
        if self.sel.is_some_and(|i| i >= self.comp.layers.len()) {
            self.sel = None;
        }
    }
}

impl eframe::App for Motion {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let before = self.comp.clone();
        self.keys(ctx);
        if self.playing {
            let dt = ctx.input(|i| i.stable_dt).min(0.1);
            self.t += dt;
            if self.t > self.comp.duration {
                self.t = 0.0;
            }
            ctx.request_repaint();
        }
        let name = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = format!("Motion — {name}{}", if self.comp != self.saved { " •" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        self.menu(root);
        self.status(root);
        self.timeline(root);
        self.props(root);
        self.viewer(root);

        if self.comp != before && !self.editing {
            self.undo.push(before);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.editing = true;
        }
        if ctx.input(|i| !i.pointer.any_down()) && !ctx.egui_wants_keyboard_input() {
            self.editing = false;
        }
    }
}

impl Motion {
    fn menu(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("menu")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 6)))
            .show(root, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    studio_ui::app_icon(ui, 22.0, "motion", ACCENT);
                    ui.add_space(4.0);
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouvelle composition").clicked() {
                            self.comp = Comp::new();
                            self.saved = self.comp.clone();
                            self.file = None;
                            self.sel = None;
                            self.t = 0.0;
                        }
                        if ui.button("Ouvrir…    Ctrl+O").clicked() {
                            self.open();
                        }
                        if ui.button("Enregistrer    Ctrl+S").clicked() {
                            self.save(false);
                        }
                        if ui.button("Enregistrer sous…").clicked() {
                            self.save(true);
                        }
                        ui.separator();
                        let busy = self.job.as_ref().is_some_and(|j| j.done.load(Ordering::Relaxed) != usize::MAX);
                        if ui.add_enabled(!busy, egui::Button::new("Exporter en GIF…")).clicked() {
                            self.export(true);
                        }
                        if ui.add_enabled(!busy, egui::Button::new("Exporter en séquence PNG…")).clicked() {
                            self.export(false);
                        }
                    });
                    ui.menu_button("Calque", |ui| {
                        if ui.button("Texte").clicked() {
                            self.add_layer("Texte", Content::Text { text: "Texte".into(), size: 96.0, color: [255, 255, 255, 255], bold: true, family: "Segoe UI".into() });
                        }
                        if ui.button("Rectangle").clicked() {
                            self.add_layer("Rectangle", Content::Rect { w: 400.0, h: 240.0, r: 24.0, color: [64, 200, 255, 255] });
                        }
                        if ui.button("Ellipse").clicked() {
                            self.add_layer("Ellipse", Content::Ellipse { w: 300.0, h: 300.0, color: [255, 92, 120, 255] });
                        }
                        if ui.button("Image…").clicked() {
                            self.add_image();
                        }
                        ui.separator();
                        let has = self.sel.is_some();
                        if ui.add_enabled(has, egui::Button::new("Monter")).clicked() {
                            let i = self.sel.unwrap();
                            if i + 1 < self.comp.layers.len() {
                                self.comp.layers.swap(i, i + 1);
                                self.sel = Some(i + 1);
                            }
                        }
                        if ui.add_enabled(has, egui::Button::new("Descendre")).clicked() {
                            let i = self.sel.unwrap();
                            if i > 0 {
                                self.comp.layers.swap(i, i - 1);
                                self.sel = Some(i - 1);
                            }
                        }
                        if ui.add_enabled(has, egui::Button::new("Supprimer    Suppr")).clicked() {
                            self.comp.layers.remove(self.sel.take().unwrap());
                        }
                    });
                    ui.menu_button("Animations", |ui| {
                        ui.label(RichText::new("Ajoutées au calque à partir du temps actuel").size(11.0).color(MUTED));
                        let has = self.sel.is_some();
                        for (k, label) in ["Fondu entrant", "Fondu sortant", "Glisser depuis la gauche", "Monter depuis le bas", "Zoom pop", "Rotation continue"].into_iter().enumerate() {
                            if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                                self.preset(k as u8);
                            }
                        }
                    });
                    ui.menu_button("Composition", |ui| {
                        ui.add(egui::DragValue::new(&mut self.comp.width).range(16..=7680).prefix("Largeur "));
                        ui.add(egui::DragValue::new(&mut self.comp.height).range(16..=4320).prefix("Hauteur "));
                        ui.add(egui::DragValue::new(&mut self.comp.fps).range(1..=120).prefix("Images/s "));
                        ui.add(egui::DragValue::new(&mut self.comp.duration).range(0.1..=600.0).speed(0.1).prefix("Durée ").suffix(" s"));
                        let mut bg = Color32::from_rgba_unmultiplied(self.comp.background[0], self.comp.background[1], self.comp.background[2], self.comp.background[3]);
                        ui.horizontal(|ui| {
                            ui.label("Fond");
                            ui.color_edit_button_srgba(&mut bg);
                        });
                        self.comp.background = bg.to_srgba_unmultiplied();
                    });
                });
            });
    }

    fn status(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("status")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 5)))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                    ui.label(small(format!("{} × {} · {} i/s · {:.1} s", self.comp.width, self.comp.height, self.comp.fps, self.comp.duration)));
                    ui.separator();
                    ui.label(small("Espace lecture · ← → image par image · ◆ = image clé · glisser le calque dans l'aperçu".into()));
                    if let Some(j) = &self.job {
                        ui.separator();
                        let d = j.done.load(Ordering::Relaxed);
                        if d == usize::MAX {
                            match j.error.lock().unwrap().clone() {
                                Some(e) => ui.label(RichText::new(format!("Export {} : {e}", j.label)).size(12.0).color(DANGER)),
                                None => ui.label(RichText::new(format!("Export {} terminé", j.label)).size(12.0).color(OK)),
                            };
                        } else {
                            ui.add(egui::ProgressBar::new(d as f32 / j.total as f32).desired_width(180.0).text(format!("{} {d}/{}", j.label, j.total)));
                            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
                        }
                    }
                    if let Some(e) = self.error.clone() {
                        ui.separator();
                        ui.label(RichText::new(e).size(12.0).color(DANGER));
                        if ui.small_button("×").clicked() {
                            self.error = None;
                        }
                    }
                });
            });
    }

    fn timeline(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("timeline")
            .resizable(true)
            .default_size(250.0)
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::same(10)))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    let label = if self.playing { "⏸" } else { "▶" };
                    if ui.add(egui::Button::new(RichText::new(label).strong().color(Color32::from_rgb(14, 15, 20))).fill(ACCENT).min_size(vec2(40.0, 26.0))).clicked() {
                        self.playing = !self.playing;
                    }
                    if ui.button("⏮").clicked() {
                        self.t = 0.0;
                    }
                    let frame = (self.t * self.comp.fps as f32).round() as u32;
                    ui.label(RichText::new(format!("{:05.2} s", self.t)).monospace().size(16.0).color(TEXT));
                    ui.label(RichText::new(format!("image {frame}/{}", self.comp.frames())).monospace().color(MUTED));
                });
                ui.add_space(6.0);

                let avail = ui.available_rect_before_wrap();
                let track = Rect::from_min_max(pos2(avail.min.x + LABEL_W, avail.min.y), avail.max);
                let dur = self.comp.duration.max(0.1);
                let x_of = |t: f32| track.min.x + t / dur * track.width();
                let t_of = |x: f32| ((x - track.min.x) / track.width() * dur).clamp(0.0, dur);
                let ruler = Rect::from_min_max(track.min, pos2(track.max.x, track.min.y + 20.0));
                let painter = ui.painter_at(avail);
                painter.rect_filled(ruler, CornerRadius::ZERO, BG);
                let step = [0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0].into_iter().find(|s| dur / s <= 16.0).unwrap_or(120.0);
                let mut tt = 0.0;
                while tt <= dur + 1e-3 {
                    let x = x_of(tt);
                    painter.line_segment([pos2(x, ruler.min.y + 12.0), pos2(x, ruler.max.y)], Stroke::new(1.0, MUTED));
                    painter.text(pos2(x + 3.0, ruler.min.y + 1.0), Align2::LEFT_TOP, format!("{tt:.1}s"), egui::FontId::monospace(10.0), MUTED);
                    tt += step;
                }
                let r = ui.interact(ruler, ui.scope_id().with("ruler"), Sense::click_and_drag());
                if let Some(p) = r.interact_pointer_pos() {
                    self.t = self.snap(t_of(p.x));
                    self.playing = false;
                }

                let mut y = ruler.max.y + 4.0;
                let n = self.comp.layers.len();
                for li in (0..n).rev() {
                    let row = Rect::from_min_max(pos2(avail.min.x, y), pos2(avail.max.x, y + ROW_H));
                    if row.max.y > avail.max.y {
                        break;
                    }
                    let on = self.sel == Some(li);
                    painter.rect_filled(row.shrink2(vec2(0.0, 1.0)), CornerRadius::same(4), if on { CARD_HOVER } else { CARD });
                    let l = &self.comp.layers[li];
                    painter.text(pos2(row.min.x + 10.0, row.center().y), Align2::LEFT_CENTER, &l.name, egui::FontId::proportional(13.0), if on { TEXT } else { MUTED });
                    let lab = ui.interact(Rect::from_min_max(row.min, pos2(track.min.x, row.max.y)), ui.scope_id().with(("lab", li)), Sense::click());
                    if lab.clicked() {
                        self.sel = Some(li);
                    }
                    let bar = Rect::from_min_max(pos2(x_of(l.start), row.min.y + 5.0), pos2(x_of(l.end), row.max.y - 5.0));
                    painter.rect_filled(bar, CornerRadius::same(4), ACCENT.gamma_multiply(if on { 0.45 } else { 0.25 }));
                    let mut times: Vec<f32> = l.props.iter().flat_map(|p| p.keys.iter().map(|k| k.t)).collect();
                    times.sort_by(f32::total_cmp);
                    times.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
                    for kt in &times {
                        let c = pos2(x_of(*kt), row.center().y);
                        let d = 5.0;
                        painter.add(Shape::convex_polygon(vec![c + vec2(0.0, -d), c + vec2(d, 0.0), c + vec2(0.0, d), c + vec2(-d, 0.0)], Color32::WHITE, Stroke::new(1.0, ACCENT)));
                    }
                    let br = ui.interact(bar.expand2(vec2(4.0, 0.0)), ui.scope_id().with(("bar", li)), Sense::click_and_drag());
                    if br.drag_started()
                        && let Some(p) = br.interact_pointer_pos()
                    {
                        let mode = if (p.x - bar.min.x).abs() < 6.0 { 1 } else if (p.x - bar.max.x).abs() < 6.0 { 2 } else { 0 };
                        self.bar_drag = Some((li, mode, t_of(p.x), l.start, l.end));
                        self.sel = Some(li);
                    }
                    if br.clicked() {
                        self.sel = Some(li);
                        if let Some(p) = br.interact_pointer_pos()
                            && let Some(kt) = times.iter().find(|kt| (x_of(**kt) - p.x).abs() < 6.0)
                        {
                            self.t = *kt;
                        }
                    }
                    y += ROW_H;
                }
                if let Some((li, mode, from, s0, e0)) = self.bar_drag {
                    if let Some(p) = ui.input(|i| i.pointer.hover_pos())
                        && ui.input(|i| i.pointer.primary_down())
                    {
                        let d = self.snap(t_of(p.x) - from);
                        let l = &mut self.comp.layers[li];
                        match mode {
                            1 => l.start = (s0 + d).clamp(0.0, l.end - 0.05),
                            2 => l.end = (e0 + d).clamp(l.start + 0.05, dur),
                            _ => {
                                let len = e0 - s0;
                                l.start = (s0 + d).clamp(0.0, dur - len);
                                l.end = l.start + len;
                            }
                        }
                    } else {
                        self.bar_drag = None;
                    }
                }
                let x = x_of(self.t.min(dur));
                painter.line_segment([pos2(x, ruler.min.y), pos2(x, avail.max.y)], Stroke::new(2.0, ACCENT));
                ui.allocate_rect(avail, Sense::hover());
            });
    }

    fn props(&mut self, root: &mut egui::Ui) {
        egui::Panel::right("props")
            .resizable(false)
            .exact_size(290.0)
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::same(12)))
            .show(root, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let Some(i) = self.sel else {
                        ui.label(RichText::new("Sélectionne un calque dans la timeline, ou ajoute-en un depuis le menu Calque.").color(MUTED));
                        return;
                    };
                    let t = self.t;
                    let l = &mut self.comp.layers[i];
                    ui.text_edit_singleline(&mut l.name);
                    ui.checkbox(&mut l.visible, "Visible");
                    ui.add_space(8.0);
                    ui.label(RichText::new("TRANSFORMATION").size(11.0).color(MUTED));
                    for (pi, label) in PROPS.iter().enumerate() {
                        let p = &mut l.props[pi];
                        ui.horizontal(|ui| {
                            let has = p.key_at(t).is_some();
                            let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                            let c = r.center();
                            let d = 6.0;
                            let pts = vec![c + vec2(0.0, -d), c + vec2(d, 0.0), c + vec2(0.0, d), c + vec2(-d, 0.0)];
                            let fill = if has { ACCENT } else { Color32::TRANSPARENT };
                            let stroke = if p.animated() { ACCENT } else { MUTED };
                            ui.painter().add(Shape::convex_polygon(pts, fill, Stroke::new(1.5, stroke)));
                            if resp.on_hover_text("Ajouter / retirer une image clé ici").clicked() {
                                p.toggle_key(t);
                            }
                            let mut v = p.at(t);
                            let speed = if pi >= 2 { 0.5 } else { 1.0 };
                            if ui.add(egui::DragValue::new(&mut v).speed(speed).prefix(format!("{label}  "))).changed() {
                                p.set(t, v);
                            }
                            if let Some(k) = p.key_at(t) {
                                egui::ComboBox::from_id_salt(("ease", pi))
                                    .width(80.0)
                                    .selected_text(Ease::ALL.iter().find(|(e, _)| *e == p.keys[k].ease).map_or("", |x| x.1))
                                    .show_ui(ui, |ui| {
                                        for (e, n) in Ease::ALL {
                                            ui.selectable_value(&mut p.keys[k].ease, e, n);
                                        }
                                    });
                            }
                        });
                    }
                    ui.add_space(10.0);
                    ui.label(RichText::new("DURÉE DU CALQUE").size(11.0).color(MUTED));
                    let dur = self.comp.duration;
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut l.start).range(0.0..=l.end).speed(0.05).prefix("Début ").suffix(" s"));
                        ui.add(egui::DragValue::new(&mut l.end).range(l.start..=dur).speed(0.05).prefix("Fin ").suffix(" s"));
                    });
                    ui.add_space(10.0);
                    ui.label(RichText::new("CONTENU").size(11.0).color(MUTED));
                    match &mut l.content {
                        Content::Rect { w, h, r, color } => {
                            size_row(ui, w, h);
                            ui.add(egui::Slider::new(r, 0.0..=w.min(*h) / 2.0).text("Arrondi"));
                            color_row(ui, color);
                        }
                        Content::Ellipse { w, h, color } => {
                            size_row(ui, w, h);
                            color_row(ui, color);
                        }
                        Content::Text { text, size, color, bold, family } => {
                            ui.add(egui::TextEdit::multiline(text).desired_rows(2));
                            ui.add(egui::Slider::new(size, 8.0..=400.0).logarithmic(true).text("Taille"));
                            egui::ComboBox::from_label("Police").selected_text(family.as_str()).show_ui(ui, |ui| {
                                for f in ["Segoe UI", "Arial", "Impact", "Georgia", "Consolas", "Verdana", "Trebuchet MS"] {
                                    ui.selectable_value(family, f.to_string(), f);
                                }
                            });
                            ui.checkbox(bold, "Gras");
                            color_row(ui, color);
                        }
                        Content::Image { path, w, h } => {
                            ui.label(RichText::new(path.as_str()).size(11.0).color(MUTED));
                            size_row(ui, w, h);
                        }
                    }
                });
            });
    }

    fn viewer(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().frame(Frame::new().fill(BG)).show(root, |ui| {
            let area = ui.max_rect().shrink(24.0);
            let (cw, ch) = (self.comp.width as f32, self.comp.height as f32);
            let s = (area.width() / cw).min(area.height() / ch).max(0.01);
            let view = Rect::from_center_size(area.center(), vec2(cw, ch) * s);
            let px = ui.ctx().pixels_per_point();
            let rs = (s * px).min(1.0);
            let t = self.t;
            let stale = self.rendered.as_ref().is_none_or(|(c, rt, sc)| *c != self.comp || (*rt - t).abs() > 1e-4 || (*sc - rs).abs() > 1e-3);
            if stale {
                match render(&self.comp, t, rs, &self.fontdb) {
                    Ok(pix) => {
                        let img = egui::ColorImage::from_rgba_premultiplied([pix.width() as usize, pix.height() as usize], pix.data());
                        match &mut self.tex {
                            Some(tx) => tx.set(img, TextureOptions::LINEAR),
                            None => self.tex = Some(ui.ctx().load_texture("frame", img, TextureOptions::LINEAR)),
                        }
                        self.rendered = Some((self.comp.clone(), t, rs));
                    }
                    Err(e) => self.error = Some(format!("Rendu : {e}")),
                }
            }
            let painter = ui.painter_at(ui.max_rect());
            painter.rect_filled(view.expand(1.0), CornerRadius::ZERO, Color32::from_black_alpha(150));
            if let Some(tx) = &self.tex {
                painter.image(tx.id(), view, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            let to_scr = |x: f32, y: f32| pos2(view.min.x + x * s, view.min.y + y * s);
            if let Some(i) = self.sel
                && let Some(l) = self.comp.layers.get(i)
                && l.active(t)
            {
                let (w, h) = l.size();
                let sc = l.props[2].at(t) / 100.0;
                let c = to_scr(l.props[0].at(t), l.props[1].at(t));
                let r = Rect::from_center_size(c, vec2(w, h) * sc * s);
                painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, ACCENT), StrokeKind::Middle);
            }
            let resp = ui.allocate_rect(view, Sense::click_and_drag());
            if resp.drag_started()
                && let Some(p) = resp.interact_pointer_pos()
            {
                let hit = (0..self.comp.layers.len()).rev().find(|&i| {
                    let l = &self.comp.layers[i];
                    if !l.active(t) {
                        return false;
                    }
                    let (w, h) = l.size();
                    let sc = l.props[2].at(t) / 100.0;
                    Rect::from_center_size(to_scr(l.props[0].at(t), l.props[1].at(t)), vec2(w, h) * sc * s).contains(p)
                });
                if let Some(i) = hit {
                    self.sel = Some(i);
                    let l = &self.comp.layers[i];
                    self.drag = Some((l.props[0].at(t), l.props[1].at(t), p));
                }
            }
            if let (Some((x0, y0, from)), Some(i), Some(p)) = (self.drag, self.sel, resp.interact_pointer_pos())
                && resp.dragged()
            {
                let l = &mut self.comp.layers[i];
                l.props[0].set(t, (x0 + (p.x - from.x) / s).round());
                l.props[1].set(t, (y0 + (p.y - from.y) / s).round());
            }
            if resp.drag_stopped() {
                self.drag = None;
            }
            if resp.clicked() {
                self.sel = None;
            }
        });
    }
}

fn size_row(ui: &mut egui::Ui, w: &mut f32, h: &mut f32) {
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(w).range(1.0..=10000.0).prefix("L "));
        ui.add(egui::DragValue::new(h).range(1.0..=10000.0).prefix("H "));
    });
}

fn color_row(ui: &mut egui::Ui, c: &mut [u8; 4]) {
    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    ui.horizontal(|ui| {
        ui.label("Couleur");
        ui.color_edit_button_srgba(&mut col);
    });
    *c = col.to_srgba_unmultiplied();
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Motion")
            .with_icon(Arc::new(studio_ui::app_window_icon("motion")))
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([1000.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native("Motion", options, Box::new(|cc| Ok(Box::new(Motion::new(cc)))))
}
