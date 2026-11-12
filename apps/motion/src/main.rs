#![windows_subsystem = "windows"]

mod model;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key as EKey, Layout, Margin, Painter,
    Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2,
    pos2, vec2,
};
use model::{Comp, Content, Ease, Layer, PROPS};
use resvg::{tiny_skia, usvg};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};

const KEY: Color32 = Color32::from_rgb(255, 196, 0);
const PLAYHEAD: Color32 = Color32::from_rgb(255, 69, 58);
const LABELS: [Color32; 6] = [
    Color32::from_rgb(10, 132, 255),
    Color32::from_rgb(255, 92, 120),
    Color32::from_rgb(48, 209, 88),
    Color32::from_rgb(255, 159, 10),
    Color32::from_rgb(191, 90, 242),
    Color32::from_rgb(100, 210, 255),
];
const PRESETS: [(&str, &str, u32, u32); 5] = [
    ("1080p", "Vidéo 16:9", 1920, 1080),
    ("4K UHD", "Vidéo 16:9", 3840, 2160),
    ("Vertical", "Shorts, Reels", 1080, 1920),
    ("Carré", "Publication 1:1", 1080, 1080),
    ("Bannière GIF", "Web", 800, 400),
];
const ANIMS: [(&str, &str); 8] = [
    ("Fondu entrant", "Opacité 0 → 100 %"),
    ("Fondu sortant", "Opacité 100 → 0 %"),
    ("Glisser depuis la gauche", "Entre par le côté"),
    ("Monter depuis le bas", "Entre par le bas"),
    ("Zoom pop", "Grossit avec un rebond"),
    ("Rotation continue", "Un tour complet"),
    ("Rebond", "Tombe et rebondit"),
    ("Pulsation", "Respire en boucle"),
];

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
    home: bool,
    recents: Vec<PathBuf>,
    undo: Vec<Comp>,
    redo: Vec<Comp>,
    editing: bool,
    sel: Option<usize>,
    expanded: Vec<bool>,
    t: f32,
    playing: bool,
    fontdb: Arc<usvg::fontdb::Database>,
    tex: Option<TextureHandle>,
    rendered: Option<(Comp, f32, f32)>,
    drag: Option<(f32, f32, Pos2)>,
    bar_drag: Option<(usize, u8, f32, f32, f32)>,
    key_drag: Option<(usize, usize, f32)>,
    pps: f32,
    job: Option<Job>,
    error: Option<String>,
    title: String,
    icon: TextureHandle,
}

fn recents_path() -> PathBuf {
    studio_core::data_dir().join("motion").join("recents.txt")
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

fn demo_comp(w: u32, h: u32) -> Comp {
    let mut comp = Comp { width: w, height: h, ..Comp::new() };
    let mut title = Layer::new(
        "Titre".into(),
        Content::Text { text: "Ethone Motion".into(), size: (w.min(h) as f32 * 0.11).max(24.0), color: [255, 255, 255, 255], bold: true, family: "Segoe UI".into() },
        w as f32 / 2.0,
        h as f32 / 2.0,
        5.0,
    );
    title.props[4].add_key(0.0, 0.0);
    title.props[4].add_key(1.0, 100.0);
    title.props[2].add_key(0.0, 80.0);
    title.props[2].add_key(1.0, 100.0);
    comp.layers.push(title);
    comp
}

impl Motion {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        let comp = demo_comp(1920, 1080);
        let recents = std::fs::read_to_string(recents_path())
            .map(|s| s.lines().map(PathBuf::from).filter(|p| p.exists()).collect())
            .unwrap_or_default();
        let mut me = Self {
            saved: comp.clone(),
            comp,
            file: None,
            home: true,
            recents,
            undo: Vec::new(),
            redo: Vec::new(),
            editing: false,
            sel: Some(0),
            expanded: vec![true],
            t: 0.0,
            playing: false,
            fontdb: Arc::new(db),
            tex: None,
            rendered: None,
            drag: None,
            bar_drag: None,
            key_drag: None,
            pps: 160.0,
            job: None,
            error: None,
            title: String::new(),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "motion"),
        };
        if let Some(p) = std::env::args().nth(1).map(PathBuf::from) {
            if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("emot")) {
                me.open_path(&p);
            } else if p.as_os_str() == "--demo" {
                me.home = false;
                me.t = 1.2;
            }
        }
        me
    }

    fn snap(&self, t: f32) -> f32 {
        (t * self.comp.fps as f32).round() / self.comp.fps as f32
    }

    fn set_comp(&mut self, comp: Comp, file: Option<PathBuf>) {
        self.saved = comp.clone();
        self.expanded = vec![false; comp.layers.len()];
        self.comp = comp;
        self.file = file;
        self.undo.clear();
        self.redo.clear();
        self.sel = None;
        self.t = 0.0;
        self.playing = false;
        self.home = false;
    }

    fn remember(&mut self, path: &Path) {
        self.recents.retain(|p| p != path);
        self.recents.insert(0, path.to_path_buf());
        self.recents.truncate(8);
        let file = recents_path();
        let _ = std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")));
        let lines: Vec<String> = self.recents.iter().map(|p| p.display().to_string()).collect();
        let _ = std::fs::write(file, lines.join("\n"));
    }

    fn add_layer(&mut self, name: &str, content: Content) {
        let (cx, cy) = (self.comp.width as f32 / 2.0, self.comp.height as f32 / 2.0);
        let l = Layer::new(format!("{name} {}", self.comp.layers.len() + 1), content, cx, cy, self.comp.duration);
        self.comp.layers.push(l);
        self.expanded.push(false);
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

    fn preset(&mut self, kind: usize) {
        let Some(i) = self.sel else { return };
        let t = self.t;
        let d = 0.6;
        let (ch, dur) = (self.comp.height as f32, self.comp.duration);
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
            5 => {
                l.props[3].add_key(t, 0.0);
                l.props[3].add_key(dur, 360.0);
                l.props[3].keys.iter_mut().for_each(|k| k.ease = Ease::Linear);
            }
            6 => {
                let top = y - ch * 0.35;
                for (dt, v, e) in [(0.0, top, Ease::In), (0.45, y, Ease::Out), (0.7, y - ch * 0.08, Ease::In), (0.9, y, Ease::Smooth)] {
                    l.props[1].add_key(t + dt, v);
                    if let Some(k) = l.props[1].key_at(t + dt) {
                        l.props[1].keys[k].ease = e;
                    }
                }
            }
            _ => {
                let mut tt = t;
                let mut big = false;
                while tt <= dur + 1e-3 {
                    l.props[2].add_key(tt, if big { 110.0 } else { 100.0 });
                    big = !big;
                    tt += 0.5;
                }
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
                self.saved = self.comp.clone();
                self.remember(&path);
                self.file = Some(path);
            }
            Err(e) => self.error = Some(format!("Enregistrement : {e}")),
        }
    }

    fn open_path(&mut self, p: &Path) {
        match std::fs::read(p).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Comp>(&b).map_err(|e| e.to_string())) {
            Ok(c) => {
                self.set_comp(c, Some(p.to_path_buf()));
                self.remember(p);
            }
            Err(e) => self.error = Some(format!("Ouverture : {e}")),
        }
    }

    fn open(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("Projet Motion", &["emot"]).pick_file() {
            self.open_path(&p);
        }
    }

    fn export(&mut self, gif: bool) {
        let comp = self.comp.clone();
        let db = self.fontdb.clone();
        let total = comp.frames() as usize;
        let done = Arc::new(AtomicUsize::new(0));
        let error = Arc::new(Mutex::new(None));
        let (d2, e2) = (done.clone(), error.clone());
        let finish = move |r: Result<(), String>, e2: Arc<Mutex<Option<String>>>, d2: Arc<AtomicUsize>| {
            if let Err(e) = r
                && let Ok(mut slot) = e2.lock()
            {
                *slot = Some(e);
            }
            d2.store(usize::MAX, Ordering::Relaxed);
        };
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
                finish(r, e2, d2);
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
                finish(r, e2, d2);
            });
            self.job = Some(Job { done, total, error, label: "séquence PNG".into() });
        }
    }

    fn duplicate(&mut self) {
        if let Some(i) = self.sel {
            let mut l = self.comp.layers[i].clone();
            l.name = format!("{} copie", l.name);
            self.comp.layers.insert(i + 1, l);
            self.expanded.insert(i + 1, false);
            self.sel = Some(i + 1);
        }
    }

    fn delete(&mut self) {
        if let Some(i) = self.sel.take() {
            self.comp.layers.remove(i);
            if i < self.expanded.len() {
                self.expanded.remove(i);
            }
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() || self.home {
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
        if sc(M::COMMAND, EKey::D) {
            self.duplicate();
        }
        if sc(M::COMMAND, EKey::M) {
            self.export(true);
        }
        let step = 1.0 / self.comp.fps as f32;
        let (space, left, right, del, home, end) = ctx.input(|i| {
            (i.key_pressed(EKey::Space), i.key_pressed(EKey::ArrowLeft), i.key_pressed(EKey::ArrowRight), i.key_pressed(EKey::Delete), i.key_pressed(EKey::Home), i.key_pressed(EKey::End))
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
        if end {
            self.t = self.comp.duration;
        }
        if del {
            self.delete();
        }
        self.expanded.resize(self.comp.layers.len(), false);
        if self.sel.is_some_and(|i| i >= self.comp.layers.len()) {
            self.sel = None;
        }
    }
}

impl eframe::App for Motion {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
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
        let title = if self.home { "Motion".to_string() } else { format!("{name}{} — Motion", if self.comp != self.saved { " •" } else { "" }) };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        self.menu_bar(root);
        self.status(root);
        if self.home {
            self.home_screen(root);
        } else {
            self.timeline(root);
            self.library(root);
            self.props(root);
            self.viewer(root, &name);
        }

        if !self.home && self.comp != before && !self.editing {
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
    fn menu_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(40.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(10, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add(egui::Image::new((self.icon.id(), Vec2::splat(22.0))));
                ui.add_space(4.0);
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouvelle composition").clicked() {
                            self.home = true;
                        }
                        if ui.button("Ouvrir…                        Ctrl+O").clicked() {
                            self.open();
                        }
                        ui.add_enabled_ui(!self.home, |ui| {
                            if ui.button("Enregistrer                   Ctrl+S").clicked() {
                                self.save(false);
                            }
                            if ui.button("Enregistrer sous…").clicked() {
                                self.save(true);
                            }
                            ui.separator();
                            if ui.button("Exporter en GIF…           Ctrl+M").clicked() {
                                self.export(true);
                            }
                            if ui.button("Exporter en séquence PNG…").clicked() {
                                self.export(false);
                            }
                        });
                    });
                    ui.add_enabled_ui(!self.home, |ui| {
                        ui.menu_button("Édition", |ui| {
                            if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler              Ctrl+Z")).clicked()
                                && let Some(c) = self.undo.pop()
                            {
                                self.redo.push(std::mem::replace(&mut self.comp, c));
                            }
                            if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir             Ctrl+Y")).clicked()
                                && let Some(c) = self.redo.pop()
                            {
                                self.undo.push(std::mem::replace(&mut self.comp, c));
                            }
                            ui.separator();
                            if ui.add_enabled(self.sel.is_some(), egui::Button::new("Dupliquer le calque  Ctrl+D")).clicked() {
                                self.duplicate();
                            }
                            if ui.add_enabled(self.sel.is_some(), egui::Button::new("Supprimer le calque   Suppr")).clicked() {
                                self.delete();
                            }
                        });
                        ui.menu_button("Composition", |ui| {
                            ui.add(egui::DragValue::new(&mut self.comp.width).range(16..=7680).prefix("Largeur ").suffix(" px"));
                            ui.add(egui::DragValue::new(&mut self.comp.height).range(16..=4320).prefix("Hauteur ").suffix(" px"));
                            ui.add(egui::DragValue::new(&mut self.comp.fps).range(1..=120).prefix("Images/s "));
                            ui.add(egui::DragValue::new(&mut self.comp.duration).range(0.1..=600.0).speed(0.1).prefix("Durée ").suffix(" s"));
                        });
                    });
                });
                if !self.home {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let busy = self.job.as_ref().is_some_and(|j| j.done.load(Ordering::Relaxed) != usize::MAX);
                        if !busy {
                            if pro::primary(ui, "Exporter en GIF").clicked() {
                                self.export(true);
                            }
                            if pro::secondary(ui, "PNG").on_hover_text("Exporter en séquence d'images PNG").clicked() {
                                self.export(false);
                            }
                        }
                        ui.label(RichText::new(format!("{} × {} · {} i/s · {:.1} s", self.comp.width, self.comp.height, self.comp.fps, self.comp.duration)).size(12.5).color(MUTED));
                    });
                }
            });
        });
    }

    fn status(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                if !self.home {
                    ui.label(small("Espace lire · ← → image par image · ◆ image clé · glisse les ◆ dans la timeline · glisse un calque dans la composition".into()));
                }
                if let Some(j) = &self.job {
                    let d = j.done.load(Ordering::Relaxed);
                    if d == usize::MAX {
                        match j.error.lock().ok().and_then(|e| e.clone()) {
                            Some(e) => ui.label(RichText::new(format!("Export {} : {e}", j.label)).size(12.0).color(RED)),
                            None => ui.label(RichText::new(format!("Export {} terminé", j.label)).size(12.0).color(GREEN)),
                        };
                    } else {
                        ui.add(egui::ProgressBar::new(d as f32 / j.total.max(1) as f32).desired_width(200.0).text(format!("{} {d}/{}", j.label, j.total)));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
                    }
                }
                if let Some(e) = self.error.clone() {
                    ui.label(RichText::new(e).size(12.0).color(RED));
                    if ui.small_button("×").clicked() {
                        self.error = None;
                    }
                }
            });
        });
    }

    fn home_screen(&mut self, root: &mut Ui) {
        egui::Panel::left("home-nav").exact_size(230.0).resizable(false).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(18, 26))).show(root, |ui| {
            if pro::primary(ui, "Nouvelle composition").clicked() {
                self.set_comp(demo_comp(1920, 1080), None);
            }
            ui.add_space(8.0);
            if pro::secondary(ui, "Ouvrir…").clicked() {
                self.open();
            }
        });
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin { left: 40, right: 40, top: 34, bottom: 20 })).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(40.0))));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Bienvenue dans Motion").font(semibold(28.0)).color(INK_STRONG));
                        ui.label(RichText::new("Titres animés, logos et graphismes en mouvement").color(MUTED));
                    });
                });
                ui.add_space(28.0);
                ui.label(RichText::new("Nouvelle composition").font(semibold(16.0)).color(INK));
                ui.add_space(12.0);
                let mut chosen = None;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                    for (name, kind, w, h) in PRESETS {
                        if preset_card(ui, name, &format!("{w} × {h} · {kind}"), w as f32, h as f32).clicked() {
                            chosen = Some((w, h));
                        }
                    }
                });
                if let Some((w, h)) = chosen {
                    self.set_comp(demo_comp(w, h), None);
                }
                ui.add_space(26.0);
                ui.label(RichText::new("Projets récents").font(semibold(16.0)).color(INK));
                ui.add_space(10.0);
                if self.recents.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().min(760.0), 100.0), Sense::hover());
                    let p = ui.painter();
                    p.rect(r, CornerRadius::same(14), Color32::from_white_alpha(4), Stroke::new(1.0, LINE), StrokeKind::Inside);
                    p.text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "Tes animations enregistrées apparaîtront ici", semibold(15.0), INK);
                    p.text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "Projets .emot", FontId::proportional(13.0), MUTED);
                } else {
                    let mut open = None;
                    for p in &self.recents {
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width().min(760.0), 46.0), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(r, CornerRadius::same(10), Color32::from_white_alpha(10));
                        }
                        let nm = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                        ui.painter().text(r.left_top() + vec2(14.0, 8.0), Align2::LEFT_TOP, nm, semibold(14.0), INK);
                        ui.painter().text(r.left_top() + vec2(14.0, 27.0), Align2::LEFT_TOP, p.parent().map(|d| d.display().to_string()).unwrap_or_default(), FontId::proportional(12.0), FAINT);
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open = Some(p.clone());
                        }
                    }
                    if let Some(p) = open {
                        self.open_path(&p);
                    }
                }
            });
        });
    }

    fn library(&mut self, root: &mut Ui) {
        egui::Panel::left("library").resizable(false).exact_size(250.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                pro::dock_panel(ui, "add", "Ajouter un calque", |ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    let w = (ui.available_width() - 8.0) / 2.0;
                    let items: [(&str, u8); 4] = [("Texte", 0), ("Rectangle", 1), ("Ellipse", 2), ("Image…", 3)];
                    for row in items.chunks(2) {
                        ui.horizontal(|ui| {
                            for (label, kind) in row {
                                let (r, resp) = ui.allocate_exact_size(vec2(w, 64.0), Sense::click());
                                let p = ui.painter();
                                p.rect(r, CornerRadius::same(10), if resp.hovered() { FIELD } else { Color32::from_rgb(0x26, 0x26, 0x2b) }, Stroke::new(1.0, LINE), StrokeKind::Inside);
                                layer_glyph(p, Rect::from_center_size(r.center() - vec2(0.0, 9.0), Vec2::splat(20.0)), *kind, INK);
                                p.text(r.center() + vec2(0.0, 17.0), Align2::CENTER_CENTER, *label, FontId::proportional(12.5), INK);
                                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    match kind {
                                        0 => self.add_layer("Texte", Content::Text { text: "Texte".into(), size: 96.0, color: [255, 255, 255, 255], bold: true, family: "Segoe UI".into() }),
                                        1 => self.add_layer("Rectangle", Content::Rect { w: 400.0, h: 240.0, r: 24.0, color: [10, 132, 255, 255] }),
                                        2 => self.add_layer("Ellipse", Content::Ellipse { w: 300.0, h: 300.0, color: [255, 92, 120, 255] }),
                                        _ => self.add_image(),
                                    }
                                }
                            }
                        });
                    }
                });
                pro::dock_panel(ui, "anims", "Animations prêtes", |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    if self.sel.is_none() {
                        ui.label(RichText::new("Sélectionne un calque pour lui appliquer une animation.").size(12.0).color(FAINT));
                    }
                    for (k, (name, hint)) in ANIMS.iter().enumerate() {
                        let enabled = self.sel.is_some();
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), if enabled { Sense::click() } else { Sense::hover() });
                        let p = ui.painter();
                        if resp.hovered() && enabled {
                            p.rect_filled(r, CornerRadius::same(8), Color32::from_white_alpha(10));
                        }
                        let c = r.left_center() + vec2(14.0, 0.0);
                        p.add(Shape::convex_polygon(vec![c + vec2(0.0, -5.0), c + vec2(5.0, 0.0), c + vec2(0.0, 5.0), c + vec2(-5.0, 0.0)], if enabled { KEY } else { FAINT }, Stroke::NONE));
                        p.text(r.left_top() + vec2(28.0, 6.0), Align2::LEFT_TOP, *name, FontId::proportional(13.0), if enabled { INK } else { FAINT });
                        p.text(r.left_top() + vec2(28.0, 23.0), Align2::LEFT_TOP, *hint, FontId::proportional(11.0), FAINT);
                        if resp.on_hover_text("Ajoutée à partir de la tête de lecture").clicked() && enabled {
                            self.preset(k);
                        }
                    }
                });
            });
        });
    }

    fn props(&mut self, root: &mut Ui) {
        egui::Panel::right("props").resizable(false).exact_size(296.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let Some(i) = self.sel.filter(|i| *i < self.comp.layers.len()) else {
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
                        let mut bg = Color32::from_rgba_unmultiplied(self.comp.background[0], self.comp.background[1], self.comp.background[2], self.comp.background[3]);
                        ui.label(RichText::new("Fond").size(12.5).color(MUTED));
                        let w = ui.available_width();
                        pro::color_picker(ui, &mut bg, w);
                        self.comp.background = bg.to_srgba_unmultiplied();
                    });
                    return;
                };
                let t = self.t;
                let dur = self.comp.duration;
                let l = &mut self.comp.layers[i];
                pro::dock_panel(ui, "layer", "Calque", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.add(egui::TextEdit::singleline(&mut l.name).desired_width(f32::INFINITY));
                    ui.checkbox(&mut l.visible, "Visible");
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("De").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(&mut l.start).range(0.0..=l.end).speed(0.05).suffix(" s"));
                        ui.label(RichText::new("à").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(&mut l.end).range(l.start..=dur).speed(0.05).suffix(" s"));
                    });
                });
                pro::dock_panel(ui, "transform", "Transformation", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    for (pi, label) in PROPS.iter().enumerate() {
                        let p = &mut l.props[pi];
                        ui.horizontal(|ui| {
                            let has = p.key_at(t).is_some();
                            let (r, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
                            let c = r.center();
                            let d = 6.0;
                            ui.painter().add(Shape::convex_polygon(
                                vec![c + vec2(0.0, -d), c + vec2(d, 0.0), c + vec2(0.0, d), c + vec2(-d, 0.0)],
                                if has { KEY } else { Color32::TRANSPARENT },
                                Stroke::new(1.5, if p.animated() { KEY } else { FAINT }),
                            ));
                            if resp.on_hover_text("Ajouter ou retirer une image clé ici").clicked() {
                                p.toggle_key(t);
                            }
                            ui.label(RichText::new(*label).size(12.5).color(if p.animated() { INK } else { MUTED }));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let mut v = p.at(t);
                                if ui.add(egui::DragValue::new(&mut v).speed(if pi >= 2 { 0.5 } else { 1.0 }).max_decimals(1)).changed() {
                                    p.set(t, v);
                                }
                            });
                        });
                        if let Some(k) = p.key_at(t) {
                            ui.horizontal(|ui| {
                                ui.add_space(26.0);
                                ui.label(RichText::new("Courbe").size(11.5).color(FAINT));
                                egui::ComboBox::from_id_salt(("ease", pi)).width(120.0).selected_text(Ease::ALL.iter().find(|(e, _)| *e == p.keys[k].ease).map_or("", |x| x.1)).show_ui(ui, |ui| {
                                    for (e, n) in Ease::ALL {
                                        ui.selectable_value(&mut p.keys[k].ease, e, n);
                                    }
                                });
                            });
                        }
                    }
                });
                pro::dock_panel(ui, "content", "Contenu", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    match &mut l.content {
                        Content::Rect { w, h, r, color } => {
                            size_row(ui, w, h);
                            ui.label(RichText::new("Arrondi").size(12.5).color(MUTED));
                            ui.add(egui::Slider::new(r, 0.0..=w.min(*h) / 2.0));
                            color_row(ui, color);
                        }
                        Content::Ellipse { w, h, color } => {
                            size_row(ui, w, h);
                            color_row(ui, color);
                        }
                        Content::Text { text, size, color, bold, family } => {
                            ui.add(egui::TextEdit::multiline(text).desired_rows(2).desired_width(f32::INFINITY));
                            ui.label(RichText::new("Taille").size(12.5).color(MUTED));
                            ui.add(egui::Slider::new(size, 8.0..=400.0).logarithmic(true));
                            egui::ComboBox::from_id_salt("font").selected_text(family.as_str()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                                for f in ["Segoe UI", "Arial", "Impact", "Georgia", "Consolas", "Verdana", "Trebuchet MS"] {
                                    ui.selectable_value(family, f.to_string(), f);
                                }
                            });
                            ui.checkbox(bold, "Gras");
                            color_row(ui, color);
                        }
                        Content::Image { path, w, h } => {
                            ui.label(RichText::new(Path::new(path.as_str()).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).size(12.0).color(FAINT));
                            size_row(ui, w, h);
                        }
                    }
                });
            });
        });
    }

    fn viewer(&mut self, root: &mut Ui, name: &str) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            ui.painter().text(full.left_top() + vec2(16.0, 17.0), Align2::LEFT_CENTER, format!("Composition · {name}"), FontId::proportional(12.5), MUTED);
            let controls_h = 56.0;
            let area = Rect::from_min_max(pos2(full.left() + 20.0, full.top() + 34.0), pos2(full.right() - 20.0, full.bottom() - controls_h));
            let (cw, ch) = (self.comp.width as f32, self.comp.height as f32);
            let s = (area.width() / cw).min(area.height() / ch).max(0.01);
            let view = Rect::from_center_size(area.center(), vec2(cw, ch) * s);
            let rs = (s * ui.ctx().pixels_per_point()).min(1.0);
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
            let painter = ui.painter_at(full);
            painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(140) }.as_shape(view, CornerRadius::same(4)));
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
                painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, BLUE), StrokeKind::Middle);
                for q in [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()] {
                    painter.rect(Rect::from_center_size(q, Vec2::splat(8.0)), CornerRadius::same(2), Color32::WHITE, Stroke::new(1.0, BLUE), StrokeKind::Inside);
                }
                painter.circle(c, 4.0, Color32::WHITE, Stroke::new(1.0, BLUE));
            }
            let resp = ui.interact(view, ui.make_persistent_id("view"), Sense::click_and_drag());
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

            let bar = Rect::from_min_max(pos2(full.left() + 20.0, full.bottom() - controls_h + 8.0), pos2(full.right() - 20.0, full.bottom() - 8.0));
            let fps = self.comp.fps;
            painter.text(bar.left_center(), Align2::LEFT_CENTER, tc(self.t, fps), FontId::monospace(16.0), BLUE);
            painter.text(bar.right_center(), Align2::RIGHT_CENTER, tc(self.comp.duration, fps), FontId::monospace(14.0), MUTED);
            let step = 1.0 / fps as f32;
            for (dx, kind, tip) in [(-104.0, 0u8, "Début"), (-56.0, 1, "Image précédente"), (0.0, 2, "Lecture / pause (Espace)"), (56.0, 3, "Image suivante"), (104.0, 4, "Fin")] {
                let size = if kind == 2 { 42.0 } else { 34.0 };
                let r = Rect::from_center_size(bar.center() + vec2(dx, 0.0), Vec2::splat(size));
                let b = ui.interact(r, ui.make_persistent_id(("transport", kind)), Sense::click()).on_hover_text(tip);
                let fill = if kind == 2 { if b.hovered() { BLUE.lerp_to_gamma(Color32::WHITE, 0.12) } else { BLUE } } else if b.hovered() { FIELD } else { Color32::TRANSPARENT };
                ui.painter().circle_filled(r.center(), size / 2.0, fill);
                transport_glyph(ui.painter(), r.center(), kind, self.playing, if kind == 2 { Color32::WHITE } else { INK });
                if b.clicked() {
                    match kind {
                        0 => self.t = 0.0,
                        1 => self.t = self.snap((self.t - step).max(0.0)),
                        2 => self.playing = !self.playing,
                        3 => self.t = self.snap((self.t + step).min(self.comp.duration)),
                        _ => self.t = self.comp.duration,
                    }
                }
            }
        });
    }

    fn timeline(&mut self, root: &mut Ui) {
        egui::Panel::bottom("timeline").resizable(true).default_size(290.0).min_size(220.0).frame(Frame::new().fill(PANEL).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let full = ui.max_rect();
            let head = Rect::from_min_size(full.min, vec2(full.width(), 38.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(head.shrink2(vec2(12.0, 4.0))), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("Timeline").font(semibold(13.0)).color(INK));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().slider_width = 140.0;
                        ui.add(egui::Slider::new(&mut self.pps, 30.0..=800.0).logarithmic(true).show_value(false));
                        ui.label(RichText::new("Zoom").size(12.5).color(MUTED));
                    });
                });
            });
            let body = Rect::from_min_max(pos2(full.left(), head.bottom()), full.max);
            ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
                egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| self.tracks(ui));
            });
        });
    }

    fn tracks(&mut self, ui: &mut Ui) {
        let label_w = 210.0;
        let row_h = 30.0;
        let sub_h = 24.0;
        let dur = self.comp.duration.max(0.1);
        let n_rows: f32 = self.comp.layers.iter().enumerate().map(|(i, _)| row_h + if self.expanded.get(i).copied().unwrap_or(false) { sub_h * 5.0 } else { 0.0 }).sum();
        let width = (dur * self.pps + 40.0).max(ui.available_width() - label_w);
        let (rect, _) = ui.allocate_exact_size(vec2(label_w + width, (26.0 + n_rows + 10.0).max(ui.available_height())), Sense::hover());
        let x0 = rect.min.x + label_w;
        let pps = self.pps;
        let x_of = |t: f32| x0 + 8.0 + t * pps;
        let t_of = |x: f32| ((x - x0 - 8.0) / pps).clamp(0.0, dur);
        let ruler = Rect::from_min_max(pos2(x0, rect.min.y), pos2(rect.max.x, rect.min.y + 24.0));
        let p = ui.painter_at(rect);
        p.rect_filled(ruler, CornerRadius::ZERO, Color32::from_rgb(0x1a, 0x1a, 0x1d));
        p.rect_filled(Rect::from_min_max(pos2(x_of(dur), ruler.max.y), rect.max), CornerRadius::ZERO, Color32::from_black_alpha(70));
        let step = [0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0].into_iter().find(|s| s * pps >= 70.0).unwrap_or(60.0);
        let mut tt = 0.0;
        while tt <= dur + 1e-3 {
            let x = x_of(tt);
            p.line_segment([pos2(x, ruler.min.y + 14.0), pos2(x, ruler.max.y)], Stroke::new(1.0, FAINT));
            p.text(pos2(x + 3.0, ruler.min.y + 2.0), Align2::LEFT_TOP, format!("{tt:.1}s"), FontId::monospace(10.0), MUTED);
            tt += step;
        }
        let r = ui.interact(ruler, ui.make_persistent_id("ruler"), Sense::click_and_drag());
        if let Some(pp) = r.interact_pointer_pos() {
            self.t = self.snap(t_of(pp.x));
            self.playing = false;
        }

        let mut y = ruler.max.y + 4.0;
        for li in (0..self.comp.layers.len()).rev() {
            let color = LABELS[li % LABELS.len()];
            let on = self.sel == Some(li);
            let expanded = self.expanded.get(li).copied().unwrap_or(false);
            let row = Rect::from_min_max(pos2(rect.min.x, y), pos2(rect.max.x, y + row_h));
            p.rect_filled(row.shrink2(vec2(0.0, 1.0)), CornerRadius::ZERO, if on { Color32::from_rgb(0x2a, 0x2c, 0x36) } else { Color32::from_rgb(0x22, 0x22, 0x26) });
            let tw = Rect::from_center_size(pos2(rect.min.x + 14.0, row.center().y), Vec2::splat(16.0));
            let tri = if expanded { vec![tw.center() + vec2(-4.5, -2.5), tw.center() + vec2(4.5, -2.5), tw.center() + vec2(0.0, 3.5)] } else { vec![tw.center() + vec2(-2.5, -4.5), tw.center() + vec2(3.5, 0.0), tw.center() + vec2(-2.5, 4.5)] };
            p.add(Shape::convex_polygon(tri, MUTED, Stroke::NONE));
            if ui.interact(tw.expand(4.0), ui.make_persistent_id(("twirl", li)), Sense::click()).clicked()
                && let Some(e) = self.expanded.get_mut(li)
            {
                *e = !*e;
            }
            p.rect_filled(Rect::from_min_size(pos2(rect.min.x + 28.0, row.center().y - 6.0), vec2(4.0, 12.0)), CornerRadius::same(2), color);
            let l = &self.comp.layers[li];
            p.text(pos2(rect.min.x + 40.0, row.center().y), Align2::LEFT_CENTER, short(&l.name, 20), if on { semibold(12.5) } else { FontId::proportional(12.5) }, if l.visible { INK } else { FAINT });
            if ui.interact(Rect::from_min_max(pos2(rect.min.x + 26.0, row.min.y), pos2(x0, row.max.y)), ui.make_persistent_id(("label", li)), Sense::click()).clicked() {
                self.sel = Some(li);
            }
            let bar = Rect::from_min_max(pos2(x_of(l.start), row.min.y + 5.0), pos2(x_of(l.end), row.max.y - 5.0));
            p.rect_filled(bar, CornerRadius::same(5), color.gamma_multiply(if on { 0.6 } else { 0.38 }));
            if on {
                p.rect_stroke(bar, CornerRadius::same(5), Stroke::new(1.0, color), StrokeKind::Inside);
            }
            let mut times: Vec<f32> = l.props.iter().flat_map(|pr| pr.keys.iter().map(|k| k.t)).collect();
            times.sort_by(f32::total_cmp);
            times.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
            for kt in &times {
                diamond(&p, pos2(x_of(*kt), row.center().y), 5.0, KEY);
            }
            let br = ui.interact(bar.expand2(vec2(4.0, 0.0)), ui.make_persistent_id(("bar", li)), Sense::click_and_drag());
            if br.drag_started()
                && let Some(pp) = br.interact_pointer_pos()
            {
                let mode = if (pp.x - bar.min.x).abs() < 6.0 { 1 } else if (pp.x - bar.max.x).abs() < 6.0 { 2 } else { 0 };
                self.bar_drag = Some((li, mode, t_of(pp.x), l.start, l.end));
                self.sel = Some(li);
            }
            if br.clicked() {
                self.sel = Some(li);
                if let Some(pp) = br.interact_pointer_pos()
                    && let Some(kt) = times.iter().find(|kt| (x_of(**kt) - pp.x).abs() < 6.0)
                {
                    self.t = *kt;
                }
            }
            y += row_h;
            if expanded {
                for (pi, name) in PROPS.iter().enumerate() {
                    let sub = Rect::from_min_max(pos2(rect.min.x, y), pos2(rect.max.x, y + sub_h));
                    p.rect_filled(sub, CornerRadius::ZERO, Color32::from_rgb(0x1c, 0x1c, 0x20));
                    let prop = &self.comp.layers[li].props[pi];
                    p.text(pos2(rect.min.x + 44.0, sub.center().y), Align2::LEFT_CENTER, *name, FontId::proportional(11.5), if prop.animated() { INK } else { FAINT });
                    p.text(pos2(x0 - 10.0, sub.center().y), Align2::RIGHT_CENTER, format!("{:.1}", prop.at(self.t)), FontId::monospace(11.0), if prop.animated() { KEY } else { FAINT });
                    if prop.keys.len() > 1 {
                        p.line_segment([pos2(x_of(prop.keys[0].t), sub.center().y), pos2(x_of(prop.keys[prop.keys.len() - 1].t), sub.center().y)], Stroke::new(1.0, KEY.gamma_multiply(0.35)));
                    }
                    let keys: Vec<f32> = prop.keys.iter().map(|k| k.t).collect();
                    for (ki, kt) in keys.iter().enumerate() {
                        let c = pos2(x_of(*kt), sub.center().y);
                        let kr = Rect::from_center_size(c, Vec2::splat(14.0));
                        let resp = ui.interact(kr, ui.make_persistent_id(("key", li, pi, ki)), Sense::click_and_drag()).on_hover_text("Glisser pour déplacer · clic pour y aller");
                        let active = self.key_drag.is_some_and(|(a, b, _)| a == li * 10 + pi && b == ki);
                        diamond(&p, c, if resp.hovered() || active { 6.5 } else { 5.0 }, if active { Color32::WHITE } else { KEY });
                        if resp.clicked() {
                            self.t = *kt;
                            self.sel = Some(li);
                        }
                        if resp.drag_started() {
                            self.key_drag = Some((li * 10 + pi, ki, *kt));
                        }
                    }
                    y += sub_h;
                }
            }
        }

        if let Some((code, ki, _)) = self.key_drag {
            let (li, pi) = (code / 10, code % 10);
            let (down, pos) = ui.input(|i| (i.pointer.primary_down(), i.pointer.hover_pos()));
            if let (true, Some(pp)) = (down, pos) {
                let nt = self.snap(t_of(pp.x));
                if let Some(prop) = self.comp.layers.get_mut(li).and_then(|l| l.props.get_mut(pi))
                    && let Some(k) = prop.keys.get_mut(ki)
                {
                    k.t = nt;
                }
                self.t = nt;
            } else {
                if let Some(prop) = self.comp.layers.get_mut(li).and_then(|l| l.props.get_mut(pi)) {
                    prop.keys.sort_by(|a, b| a.t.total_cmp(&b.t));
                    prop.keys.dedup_by(|a, b| (a.t - b.t).abs() < 1e-3);
                }
                self.key_drag = None;
            }
        }

        if let Some((li, mode, from, s0, e0)) = self.bar_drag {
            if let Some(pp) = ui.input(|i| i.pointer.hover_pos())
                && ui.input(|i| i.pointer.primary_down())
                && li < self.comp.layers.len()
            {
                let d = self.snap(t_of(pp.x) - from);
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
        p.line_segment([pos2(x, ruler.min.y), pos2(x, rect.max.y)], Stroke::new(1.5, PLAYHEAD));
        p.add(Shape::convex_polygon(vec![pos2(x - 7.0, ruler.min.y), pos2(x + 7.0, ruler.min.y), pos2(x + 7.0, ruler.min.y + 8.0), pos2(x, ruler.min.y + 14.0), pos2(x - 7.0, ruler.min.y + 8.0)], PLAYHEAD, Stroke::NONE));
    }
}

fn diamond(p: &Painter, c: Pos2, d: f32, color: Color32) {
    p.add(Shape::convex_polygon(vec![c + vec2(0.0, -d), c + vec2(d, 0.0), c + vec2(0.0, d), c + vec2(-d, 0.0)], color, Stroke::new(1.0, Color32::from_black_alpha(120))));
}

fn tc(t: f32, fps: u32) -> String {
    let t = t.max(0.0);
    let s = t.floor();
    let f = ((t - s) * fps as f32).floor();
    format!("{:02}:{:02}:{:02}", (s / 60.0) as u32, (s % 60.0) as u32, f as u32)
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>()) }
}

fn layer_glyph(p: &Painter, r: Rect, kind: u8, c: Color32) {
    let s = Stroke::new(1.8, c);
    match kind {
        0 => {
            p.line_segment([r.left_top() + vec2(2.0, 2.0), r.right_top() + vec2(-2.0, 2.0)], Stroke::new(2.4, c));
            p.line_segment([r.center_top() + vec2(0.0, 2.0), r.center_bottom()], Stroke::new(2.4, c));
        }
        1 => {
            p.rect_stroke(r.shrink2(vec2(0.0, 3.0)), CornerRadius::same(3), s, StrokeKind::Middle);
        }
        2 => {
            p.circle_stroke(r.center(), r.width() * 0.45, s);
        }
        _ => {
            p.rect_stroke(r.shrink(1.0), CornerRadius::same(3), s, StrokeKind::Middle);
            p.circle_filled(r.left_top() + vec2(6.5, 6.5), 2.2, c);
            p.add(Shape::convex_polygon(vec![r.left_bottom() + vec2(3.0, -3.0), r.center() + vec2(1.0, 1.0), r.right_bottom() + vec2(-3.0, -3.0)], c, Stroke::NONE));
        }
    }
}

fn transport_glyph(p: &Painter, c: Pos2, kind: u8, playing: bool, color: Color32) {
    let tri = |dir: f32, at: Pos2, s: f32| vec![at + vec2(-s * 0.6 * dir, -s), at + vec2(s * 0.8 * dir, 0.0), at + vec2(-s * 0.6 * dir, s)];
    match kind {
        2 if playing => {
            p.rect_filled(Rect::from_center_size(c - vec2(4.5, 0.0), vec2(4.5, 15.0)), CornerRadius::same(1), color);
            p.rect_filled(Rect::from_center_size(c + vec2(4.5, 0.0), vec2(4.5, 15.0)), CornerRadius::same(1), color);
        }
        2 => {
            p.add(Shape::convex_polygon(tri(1.0, c + vec2(1.5, 0.0), 8.5), color, Stroke::NONE));
        }
        1 | 3 => {
            let dir = if kind == 3 { 1.0 } else { -1.0 };
            p.add(Shape::convex_polygon(tri(dir, c - vec2(2.0 * dir, 0.0), 6.0), color, Stroke::NONE));
            p.line_segment([c + vec2(6.0 * dir, -6.0), c + vec2(6.0 * dir, 6.0)], Stroke::new(2.0, color));
        }
        _ => {
            let dir = if kind == 4 { 1.0 } else { -1.0 };
            p.add(Shape::convex_polygon(tri(dir, c - vec2(4.0 * dir, 0.0), 5.5), color, Stroke::NONE));
            p.add(Shape::convex_polygon(tri(dir, c + vec2(3.0 * dir, 0.0), 5.5), color, Stroke::NONE));
            p.line_segment([c + vec2(8.0 * dir, -6.0), c + vec2(8.0 * dir, 6.0)], Stroke::new(2.0, color));
        }
    }
}

fn size_row(ui: &mut Ui, w: &mut f32, h: &mut f32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("L").color(MUTED));
        ui.add(egui::DragValue::new(w).range(1.0..=10000.0).suffix(" px"));
        ui.label(RichText::new("H").color(MUTED));
        ui.add(egui::DragValue::new(h).range(1.0..=10000.0).suffix(" px"));
    });
}

fn color_row(ui: &mut Ui, c: &mut [u8; 4]) {
    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    ui.label(RichText::new("Couleur").size(12.5).color(MUTED));
    let w = ui.available_width();
    pro::color_picker(ui, &mut col, w);
    *c = col.to_srgba_unmultiplied();
}

fn preset_card(ui: &mut Ui, name: &str, detail: &str, w: f32, h: f32) -> egui::Response {
    let width = 176.0;
    let (r, resp) = ui.allocate_exact_size(vec2(width, 132.0), Sense::click());
    let p = ui.painter();
    p.rect(r, CornerRadius::same(12), if resp.hovered() { Color32::from_rgb(0x2f, 0x2f, 0x35) } else { Color32::from_rgb(0x27, 0x27, 0x2c) }, Stroke::new(1.0, LINE), StrokeKind::Inside);
    let area = Rect::from_center_size(r.center_top() + vec2(0.0, 44.0), vec2(width - 40.0, 56.0));
    let s = (area.width() / w).min(area.height() / h);
    let shape = Rect::from_center_size(area.center(), vec2(w, h) * s);
    p.rect(shape, CornerRadius::same(3), Color32::from_white_alpha(18), Stroke::new(1.2, MUTED), StrokeKind::Inside);
    diamond(p, shape.center(), 5.0, KEY);
    p.text(pos2(r.center().x, r.bottom() - 36.0), Align2::CENTER_CENTER, name, semibold(13.0), INK_STRONG);
    p.text(pos2(r.center().x, r.bottom() - 17.0), Align2::CENTER_CENTER, detail, FontId::proportional(11.0), MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Motion")
            .with_icon(Arc::new(studio_ui::app_window_icon("motion")))
            .with_inner_size([1520.0, 940.0])
            .with_min_inner_size([1100.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native("Motion", options, Box::new(|cc| Ok(Box::new(Motion::new(cc)))))
}
