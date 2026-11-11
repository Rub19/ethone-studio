#![windows_subsystem = "windows"]

mod media;
mod model;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, Frame, Key, Margin, Rect, RichText, Sense, Stroke, StrokeKind,
    TextureHandle, TextureOptions, pos2, vec2,
};
use media::{OUT_RATE, VideoReader};
use model::{Clip, Music, Project, Source};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use studio_ui::{BG, CARD, DANGER, MUTED, OK, PANEL, TEXT};

const ACCENT: Color32 = Color32::from_rgb(168, 110, 255);
const MUSIC: Color32 = Color32::from_rgb(120, 230, 90);
const EXT: [&str; 12] = ["mp4", "mov", "m4v", "mkv", "avi", "wmv", "webm", "mp3", "wav", "m4a", "aac", "wma"];

struct PlayState {
    mix: Arc<Vec<f32>>,
    pos: f64,
    playing: bool,
}

struct Audio {
    state: Arc<Mutex<PlayState>>,
    _stream: Option<cpal::Stream>,
}

impl Audio {
    fn new() -> Self {
        let state = Arc::new(Mutex::new(PlayState { mix: Arc::new(Vec::new()), pos: 0.0, playing: false }));
        let s2 = state.clone();
        let stream = (|| -> Option<cpal::Stream> {
            let dev = cpal::default_host().default_output_device()?;
            let cfg = dev.default_output_config().ok()?;
            let step = OUT_RATE as f64 / cfg.sample_rate() as f64;
            let ch = cfg.channels() as usize;
            let stream = dev
                .build_output_stream::<f32, _, _>(
                    cfg.config(),
                    move |data: &mut [f32], _| {
                        let mut s = s2.lock().unwrap();
                        for frame in data.chunks_mut(ch) {
                            let i = s.pos as usize;
                            if s.playing && i * 2 + 1 < s.mix.len() {
                                let (l, r) = (s.mix[i * 2], s.mix[i * 2 + 1]);
                                for (k, o) in frame.iter_mut().enumerate() {
                                    *o = if k % 2 == 0 { l } else { r };
                                }
                            } else {
                                frame.fill(0.0);
                            }
                            if s.playing {
                                s.pos += step;
                            }
                        }
                    },
                    |e| eprintln!("audio: {e}"),
                    None,
                )
                .ok()?;
            stream.play().ok()?;
            Some(stream)
        })();
        Self { state, _stream: stream }
    }
}

enum Msg {
    Probed(PathBuf, Result<Source, String>),
    Audio(usize, Vec<f32>),
}

struct ExportJob {
    done: Arc<AtomicUsize>,
    total: usize,
    error: Arc<Mutex<Option<String>>>,
    path: PathBuf,
}

#[derive(Clone, Copy, PartialEq)]
enum Sel {
    Clip(usize),
    Music(usize),
}

struct Cut {
    proj: Project,
    saved: Project,
    file: Option<PathBuf>,
    undo: Vec<Project>,
    redo: Vec<Project>,
    editing: bool,
    audio_data: HashMap<usize, Arc<Vec<f32>>>,
    mixed: Option<Project>,
    readers: HashMap<usize, VideoReader>,
    tex: Option<TextureHandle>,
    shown: Option<(usize, f64)>,
    fade: f32,
    t: f64,
    playing: bool,
    sel: Option<Sel>,
    pps: f32,
    drag: Option<(Sel, u8, f64, Clip, f64)>,
    audio: Audio,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    job: Option<ExportJob>,
    error: Option<String>,
    title: String,
}

impl Cut {
    fn new(cc: &eframe::CreationContext) -> Self {
        studio_ui::apply_style(&cc.egui_ctx);
        cc.egui_ctx.all_styles_mut(|s| s.visuals.selection.bg_fill = ACCENT);
        media::startup();
        let (tx, rx) = channel();
        let proj = Project::new();
        Self {
            saved: proj.clone(),
            proj,
            file: None,
            undo: Vec::new(),
            redo: Vec::new(),
            editing: false,
            audio_data: HashMap::new(),
            mixed: None,
            readers: HashMap::new(),
            tex: None,
            shown: None,
            fade: 1.0,
            t: 0.0,
            playing: false,
            sel: None,
            pps: 60.0,
            drag: None,
            audio: Audio::new(),
            tx,
            rx,
            job: None,
            error: None,
            title: String::new(),
        }
    }

    fn import(&mut self, paths: Vec<PathBuf>, ctx: &egui::Context) {
        for p in paths {
            let tx = self.tx.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                media::startup();
                let r = VideoReader::open(&p).map(|(_, i)| Source {
                    path: p.to_string_lossy().into_owned(),
                    duration: i.duration,
                    width: i.width,
                    height: i.height,
                    fps: i.fps,
                    has_video: i.has_video,
                    has_audio: i.has_audio || !i.has_video,
                });
                let _ = tx.send(Msg::Probed(p, r));
                ctx.request_repaint();
            });
        }
    }

    fn load_audio(&self, idx: usize, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let path = PathBuf::from(&self.proj.sources[idx].path);
        std::thread::spawn(move || {
            media::startup();
            let data = media::decode_audio(&path).unwrap_or_default();
            let _ = tx.send(Msg::Audio(idx, data));
            ctx.request_repaint();
        });
    }

    fn pump(&mut self, ctx: &egui::Context) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Probed(_, Ok(src)) => {
                    let idx = self.proj.sources.len();
                    let add = src.has_video;
                    let dur = src.duration;
                    let audio = src.has_audio;
                    self.proj.sources.push(src);
                    if add {
                        self.proj.clips.push(Clip { src: idx, inp: 0.0, out: dur, volume: 1.0, fade_in: 0.0, fade_out: 0.0 });
                    } else {
                        let start = self.t;
                        self.proj.music.push(Music { src: idx, start, inp: 0.0, out: dur, volume: 0.6 });
                    }
                    if audio {
                        self.load_audio(idx, ctx);
                    }
                }
                Msg::Probed(p, Err(e)) => self.error = Some(format!("{} : {e}", p.display())),
                Msg::Audio(i, d) => {
                    self.audio_data.insert(i, Arc::new(d));
                    self.mixed = None;
                }
            }
        }
    }

    fn remix(&mut self) {
        if self.mixed.as_ref() == Some(&self.proj) {
            return;
        }
        let data = &self.audio_data;
        let mix = self.proj.mix(&|i| data.get(&i).cloned(), OUT_RATE);
        let mut s = self.audio.state.lock().unwrap();
        s.mix = Arc::new(mix);
        drop(s);
        self.mixed = Some(self.proj.clone());
    }

    fn set_playing(&mut self, on: bool) {
        self.playing = on && self.proj.duration() > 0.0;
        if self.playing && self.t >= self.proj.duration() - 0.01 {
            self.t = 0.0;
        }
        let mut s = self.audio.state.lock().unwrap();
        s.pos = self.t * OUT_RATE as f64;
        s.playing = self.playing;
    }

    fn seek(&mut self, t: f64) {
        self.t = t.clamp(0.0, self.proj.duration().max(0.0));
        let mut s = self.audio.state.lock().unwrap();
        s.pos = self.t * OUT_RATE as f64;
    }

    fn update_frame(&mut self, ctx: &egui::Context) {
        let Some((ci, st)) = self.proj.clip_at(self.t) else {
            self.shown = None;
            return;
        };
        let src_i = self.proj.clips[ci].src;
        self.fade = self.proj.fade_at(self.t);
        if self.shown.is_some_and(|(s, t)| s == src_i && (t - st).abs() < 0.5 / self.proj.fps as f64) {
            return;
        }
        let src = &self.proj.sources[src_i];
        if !self.readers.contains_key(&src_i) {
            match VideoReader::open(Path::new(&src.path)) {
                Ok((r, _)) => {
                    self.readers.insert(src_i, r);
                }
                Err(e) => {
                    self.error = Some(e);
                    return;
                }
            }
        }
        let fps = src.fps;
        let r = self.readers.get_mut(&src_i).unwrap();
        let (w, h) = (r.width as usize, r.height as usize);
        match r.frame_at(st, fps) {
            Ok(Some((_, rgba))) => {
                let img = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba);
                match &mut self.tex {
                    Some(t) => t.set(img, TextureOptions::LINEAR),
                    None => self.tex = Some(ctx.load_texture("frame", img, TextureOptions::LINEAR)),
                }
                self.shown = Some((src_i, st));
            }
            Ok(None) => {}
            Err(e) => self.error = Some(e),
        }
    }

    fn export(&mut self) {
        if self.proj.clips.is_empty() {
            self.error = Some("La timeline est vide".into());
            return;
        }
        let Some(path) = rfd::FileDialog::new().add_filter("Vidéo MP4", &["mp4"]).set_file_name("montage.mp4").save_file() else { return };
        let proj = self.proj.clone();
        let data = self.audio_data.clone();
        let total = (proj.duration() * proj.fps as f64).ceil() as usize;
        let done = Arc::new(AtomicUsize::new(0));
        let error = Arc::new(Mutex::new(None));
        let (d2, e2, p2) = (done.clone(), error.clone(), path.clone());
        std::thread::spawn(move || {
            let r = (|| -> Result<(), String> {
                media::startup();
                let (w, h, fps) = (proj.width, proj.height, proj.fps);
                let mix = proj.mix(&|i| data.get(&i).cloned(), OUT_RATE);
                let mut enc = media::Encoder::new(&p2, w, h, fps, (w * h * fps / 8).max(2_000_000))?;
                let mut readers: HashMap<usize, VideoReader> = HashMap::new();
                let per = (OUT_RATE / fps) as usize;
                let black = vec![0u8; (w * h * 4) as usize];
                for n in 0..total {
                    let t = n as f64 / fps as f64;
                    let frame = match proj.clip_at(t) {
                        Some((ci, st)) => {
                            let si = proj.clips[ci].src;
                            let src = &proj.sources[si];
                            if !readers.contains_key(&si) {
                                readers.insert(si, VideoReader::open(Path::new(&src.path))?.0);
                            }
                            let r = readers.get_mut(&si).unwrap();
                            let (sw, sh) = (r.width, r.height);
                            match r.frame_at(st, src.fps)? {
                                Some((_, rgba)) => fit(rgba, sw, sh, w, h, proj.fade_at(t)),
                                None => black.clone(),
                            }
                        }
                        None => black.clone(),
                    };
                    enc.video(&frame)?;
                    let a = (n * per * 2).min(mix.len());
                    let b = ((n + 1) * per * 2).min(mix.len());
                    let mut chunk = mix[a..b].to_vec();
                    chunk.resize(per * 2, 0.0);
                    enc.audio(&chunk)?;
                    d2.fetch_add(1, Ordering::Relaxed);
                }
                enc.finish()
            })();
            if let Err(e) = r {
                *e2.lock().unwrap() = Some(e);
            }
            d2.store(usize::MAX, Ordering::Relaxed);
        });
        self.job = Some(ExportJob { done, total, error, path });
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.file, as_new) {
            (Some(p), false) => p.clone(),
            _ => match rfd::FileDialog::new().add_filter("Projet Cut", &["ecut"]).set_file_name("montage.ecut").save_file() {
                Some(p) => p,
                None => return,
            },
        };
        match serde_json::to_vec_pretty(&self.proj).map_err(|e| e.to_string()).and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string())) {
            Ok(()) => {
                self.file = Some(path);
                self.saved = self.proj.clone();
            }
            Err(e) => self.error = Some(format!("Enregistrement : {e}")),
        }
    }

    fn open(&mut self, ctx: &egui::Context) {
        let Some(p) = rfd::FileDialog::new().add_filter("Projet Cut", &["ecut"]).pick_file() else { return };
        match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Project>(&b).map_err(|e| e.to_string())) {
            Ok(proj) => {
                self.set_playing(false);
                self.saved = proj.clone();
                self.proj = proj;
                self.file = Some(p);
                self.readers.clear();
                self.audio_data.clear();
                self.mixed = None;
                self.shown = None;
                self.undo.clear();
                self.redo.clear();
                self.sel = None;
                self.t = 0.0;
                for i in 0..self.proj.sources.len() {
                    if self.proj.sources[i].has_audio {
                        self.load_audio(i, ctx);
                    }
                }
            }
            Err(e) => self.error = Some(format!("Ouverture : {e}")),
        }
    }

    fn delete_sel(&mut self) {
        match self.sel.take() {
            Some(Sel::Clip(i)) if i < self.proj.clips.len() => {
                self.proj.clips.remove(i);
            }
            Some(Sel::Music(i)) if i < self.proj.music.len() => {
                self.proj.music.remove(i);
            }
            _ => {}
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        if sc(M::COMMAND | M::SHIFT, Key::Z) || sc(M::COMMAND, Key::Y) {
            if let Some(p) = self.redo.pop() {
                self.undo.push(std::mem::replace(&mut self.proj, p));
            }
        } else if sc(M::COMMAND, Key::Z)
            && let Some(p) = self.undo.pop()
        {
            self.redo.push(std::mem::replace(&mut self.proj, p));
        }
        if sc(M::COMMAND | M::SHIFT, Key::S) {
            self.save(true);
        } else if sc(M::COMMAND, Key::S) {
            self.save(false);
        }
        let step = 1.0 / self.proj.fps as f64;
        let (space, s, del, left, right, home, end) = ctx.input(|i| {
            (
                i.key_pressed(Key::Space),
                i.key_pressed(Key::S),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::Home),
                i.key_pressed(Key::End),
            )
        });
        if space {
            self.set_playing(!self.playing);
        }
        if s {
            self.proj.split(self.t);
        }
        if del {
            self.delete_sel();
        }
        if left {
            self.seek(self.t - step);
        }
        if right {
            self.seek(self.t + step);
        }
        if home {
            self.seek(0.0);
        }
        if end {
            self.seek(self.proj.duration());
        }
        if matches!(self.sel, Some(Sel::Clip(i)) if i >= self.proj.clips.len()) || matches!(self.sel, Some(Sel::Music(i)) if i >= self.proj.music.len()) {
            self.sel = None;
        }
    }
}

fn fit(rgba: &[u8], sw: u32, sh: u32, w: u32, h: u32, fade: f32) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for px in out.chunks_exact_mut(4) {
        px[3] = 255;
    }
    let Some(src) = image::RgbaImage::from_raw(sw, sh, rgba.to_vec()) else { return out };
    let s = (w as f32 / sw as f32).min(h as f32 / sh as f32);
    let (nw, nh) = (((sw as f32 * s).round() as u32).max(1), ((sh as f32 * s).round() as u32).max(1));
    let scaled = if (nw, nh) == (sw, sh) { src } else { image::imageops::resize(&src, nw, nh, image::imageops::FilterType::Triangle) };
    let (ox, oy) = ((w - nw.min(w)) / 2, (h - nh.min(h)) / 2);
    for y in 0..nh.min(h) {
        for x in 0..nw.min(w) {
            let p = scaled.get_pixel(x, y).0;
            let i = (((oy + y) * w + ox + x) * 4) as usize;
            out[i] = (p[0] as f32 * fade) as u8;
            out[i + 1] = (p[1] as f32 * fade) as u8;
            out[i + 2] = (p[2] as f32 * fade) as u8;
        }
    }
    out
}

fn fmt(t: f64) -> String {
    let m = (t / 60.0).floor();
    format!("{:02}:{:05.2}", m as u64, t - m * 60.0)
}

impl eframe::App for Cut {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let before = self.proj.clone();
        self.pump(ctx);
        self.keys(ctx);
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        if !dropped.is_empty() {
            self.import(dropped, ctx);
        }
        self.remix();
        if self.playing {
            let pos = self.audio.state.lock().unwrap().pos;
            self.t = if self.audio._stream.is_some() { pos / OUT_RATE as f64 } else { self.t + ctx.input(|i| i.stable_dt) as f64 };
            if self.t >= self.proj.duration() {
                self.t = self.proj.duration();
                self.set_playing(false);
            }
            ctx.request_repaint();
        }
        self.update_frame(ctx);

        let name = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = format!("Cut — {name}{}", if self.proj != self.saved { " •" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        self.top(root, ctx);
        self.status(root);
        self.timeline(root);
        self.bin(root, ctx);
        self.inspector(root);
        self.viewer(root);

        if self.proj != before && !self.editing {
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

impl Cut {
    fn top(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        egui::Panel::top("top").frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 6))).show(root, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                studio_ui::app_icon(ui, 22.0, "cut", ACCENT);
                ui.add_space(4.0);
                ui.menu_button("Fichier", |ui| {
                    if ui.button("Importer des médias…").clicked()
                        && let Some(ps) = rfd::FileDialog::new().add_filter("Médias", &EXT).pick_files()
                    {
                        self.import(ps, ctx);
                    }
                    ui.separator();
                    if ui.button("Ouvrir un projet…").clicked() {
                        self.open(ctx);
                    }
                    if ui.button("Enregistrer    Ctrl+S").clicked() {
                        self.save(false);
                    }
                    if ui.button("Enregistrer sous…").clicked() {
                        self.save(true);
                    }
                    ui.separator();
                    let busy = self.job.as_ref().is_some_and(|j| j.done.load(Ordering::Relaxed) != usize::MAX);
                    if ui.add_enabled(!busy, egui::Button::new("Exporter en MP4…")).clicked() {
                        self.export();
                    }
                });
                ui.menu_button("Montage", |ui| {
                    if ui.button("Couper à la tête de lecture    S").clicked() {
                        self.proj.split(self.t);
                    }
                    if ui.add_enabled(self.sel.is_some(), egui::Button::new("Supprimer    Suppr")).clicked() {
                        self.delete_sel();
                    }
                });
                ui.menu_button("Séquence", |ui| {
                    for (label, w, h) in [("1080p 16:9", 1920, 1080), ("720p 16:9", 1280, 720), ("4K 16:9", 3840, 2160), ("Vertical 9:16", 1080, 1920), ("Carré 1:1", 1080, 1080)] {
                        if ui.selectable_label(self.proj.width == w && self.proj.height == h, label).clicked() {
                            self.proj.width = w;
                            self.proj.height = h;
                        }
                    }
                    ui.separator();
                    for f in [24, 25, 30, 50, 60] {
                        if ui.selectable_label(self.proj.fps == f, format!("{f} images/s")).clicked() {
                            self.proj.fps = f;
                        }
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{} × {} · {} i/s", self.proj.width, self.proj.height, self.proj.fps)).size(12.0).color(MUTED));
                });
            });
        });
    }

    fn status(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("status").frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 5))).show(root, |ui| {
            ui.horizontal(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                ui.label(small("Espace lecture · S couper · Suppr supprimer · ← → image par image · glisser les bords pour raccourcir · glisser un clip pour le déplacer".into()));
                if let Some(j) = &self.job {
                    ui.separator();
                    let d = j.done.load(Ordering::Relaxed);
                    if d == usize::MAX {
                        match j.error.lock().unwrap().clone() {
                            Some(e) => ui.label(RichText::new(format!("Export : {e}")).size(12.0).color(DANGER)),
                            None => ui.label(RichText::new(format!("Exporté : {}", j.path.display())).size(12.0).color(OK)),
                        };
                    } else {
                        ui.add(egui::ProgressBar::new(d as f32 / j.total.max(1) as f32).desired_width(200.0).text(format!("Export {d}/{}", j.total)));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
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

    fn bin(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        egui::Panel::left("bin").resizable(false).exact_size(230.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::same(10))).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("MÉDIAS").size(11.0).color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("+ Importer").clicked()
                        && let Some(ps) = rfd::FileDialog::new().add_filter("Médias", &EXT).pick_files()
                    {
                        self.import(ps, ctx);
                    }
                });
            });
            ui.add_space(6.0);
            if self.proj.sources.is_empty() {
                ui.label(RichText::new("Glisse des vidéos ou de la musique ici, ou clique sur Importer.").color(MUTED));
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.proj.sources.len() {
                    let s = &self.proj.sources[i];
                    let kind = if s.has_video { "Vidéo" } else { "Audio" };
                    let loading = s.has_audio && !self.audio_data.contains_key(&i);
                    Frame::new().fill(CARD).corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(8)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(s.name()).color(TEXT));
                        ui.label(RichText::new(format!("{kind} · {}{}", fmt(s.duration), if loading { " · audio…" } else { "" })).size(11.0).color(MUTED));
                        ui.horizontal(|ui| {
                            if s.has_video && ui.small_button("→ Timeline").clicked() {
                                let d = s.duration;
                                self.proj.clips.push(Clip { src: i, inp: 0.0, out: d, volume: 1.0, fade_in: 0.0, fade_out: 0.0 });
                            }
                            if ui.small_button("→ Musique").clicked() {
                                let d = s.duration;
                                let start = self.t;
                                self.proj.music.push(Music { src: i, start, inp: 0.0, out: d, volume: 0.6 });
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
            });
        });
    }

    fn inspector(&mut self, root: &mut egui::Ui) {
        egui::Panel::right("inspector").resizable(false).exact_size(260.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::same(12))).show(root, |ui| {
            match self.sel {
                Some(Sel::Clip(i)) if i < self.proj.clips.len() => {
                    let name = self.proj.sources[self.proj.clips[i].src].name();
                    let max = self.proj.sources[self.proj.clips[i].src].duration;
                    let c = &mut self.proj.clips[i];
                    ui.label(RichText::new("CLIP").size(11.0).color(MUTED));
                    ui.label(RichText::new(name).color(TEXT));
                    ui.add_space(6.0);
                    ui.add(egui::DragValue::new(&mut c.inp).range(0.0..=c.out - 0.05).speed(0.02).prefix("Entrée ").suffix(" s"));
                    ui.add(egui::DragValue::new(&mut c.out).range(c.inp + 0.05..=max).speed(0.02).prefix("Sortie ").suffix(" s"));
                    let mut vol = c.volume * 100.0;
                    if ui.add(egui::Slider::new(&mut vol, 0.0..=200.0).suffix(" %").text("Volume")).changed() {
                        c.volume = vol / 100.0;
                    }
                    let len = c.len();
                    ui.add(egui::Slider::new(&mut c.fade_in, 0.0..=len.min(5.0)).suffix(" s").text("Fondu entrée"));
                    ui.add(egui::Slider::new(&mut c.fade_out, 0.0..=len.min(5.0)).suffix(" s").text("Fondu sortie"));
                }
                Some(Sel::Music(i)) if i < self.proj.music.len() => {
                    let name = self.proj.sources[self.proj.music[i].src].name();
                    let max = self.proj.sources[self.proj.music[i].src].duration;
                    let m = &mut self.proj.music[i];
                    ui.label(RichText::new("MUSIQUE").size(11.0).color(MUTED));
                    ui.label(RichText::new(name).color(TEXT));
                    ui.add_space(6.0);
                    ui.add(egui::DragValue::new(&mut m.start).range(0.0..=36000.0).speed(0.05).prefix("Début ").suffix(" s"));
                    ui.add(egui::DragValue::new(&mut m.inp).range(0.0..=m.out - 0.05).speed(0.05).prefix("Entrée ").suffix(" s"));
                    ui.add(egui::DragValue::new(&mut m.out).range(m.inp + 0.05..=max).speed(0.05).prefix("Sortie ").suffix(" s"));
                    let mut vol = m.volume * 100.0;
                    if ui.add(egui::Slider::new(&mut vol, 0.0..=200.0).suffix(" %").text("Volume")).changed() {
                        m.volume = vol / 100.0;
                    }
                }
                _ => {
                    ui.label(RichText::new("Clique sur un clip de la timeline pour régler son entrée, sa sortie, son volume et ses fondus.").color(MUTED));
                }
            }
        });
    }

    fn viewer(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().frame(Frame::new().fill(BG).inner_margin(Margin::same(14))).show(root, |ui| {
            let area = ui.available_rect_before_wrap();
            let controls_h = 44.0;
            let screen = Rect::from_min_max(area.min, pos2(area.max.x, area.max.y - controls_h));
            let (w, h) = (self.proj.width as f32, self.proj.height as f32);
            let s = (screen.width() / w).min(screen.height() / h);
            let frame = Rect::from_center_size(screen.center(), vec2(w, h) * s);
            let p = ui.painter();
            p.rect_filled(frame, CornerRadius::ZERO, Color32::BLACK);
            if let (Some(t), Some(_)) = (&self.tex, self.shown) {
                let [tw, th] = t.size();
                let k = (frame.width() / tw as f32).min(frame.height() / th as f32);
                let r = Rect::from_center_size(frame.center(), vec2(tw as f32, th as f32) * k);
                let g = (self.fade * 255.0) as u8;
                p.image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::from_rgb(g, g, g));
            } else if self.proj.clips.is_empty() {
                p.text(frame.center(), Align2::CENTER_CENTER, "Importe une vidéo pour commencer", egui::FontId::proportional(16.0), MUTED);
            }
            let bar = Rect::from_min_max(pos2(area.min.x, area.max.y - controls_h + 8.0), area.max);
            ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("⏮").clicked() {
                        self.seek(0.0);
                    }
                    let label = if self.playing { "⏸" } else { "▶" };
                    if ui.add(egui::Button::new(RichText::new(label).strong().color(Color32::from_rgb(14, 15, 20))).fill(ACCENT).min_size(vec2(44.0, 28.0))).clicked() {
                        self.set_playing(!self.playing);
                    }
                    if ui.button("⏭").clicked() {
                        let d = self.proj.duration();
                        self.seek(d);
                    }
                    ui.label(RichText::new(format!("{} / {}", fmt(self.t), fmt(self.proj.duration()))).monospace().size(15.0).color(TEXT));
                });
            });
        });
    }

    fn timeline(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("timeline").resizable(true).default_size(220.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::same(10))).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("TIMELINE").size(11.0).color(MUTED));
                ui.add(egui::Slider::new(&mut self.pps, 5.0..=400.0).logarithmic(true).show_value(false).text("Zoom"));
            });
            let dur = self.proj.duration().max(10.0) + 5.0;
            let label_w = 46.0;
            egui::ScrollArea::horizontal().show(ui, |ui| {
                let width = (dur as f32 * self.pps).max(ui.available_width() - label_w);
                let (rect, _) = ui.allocate_exact_size(vec2(width + label_w, ui.available_height().max(150.0)), Sense::hover());
                let x0 = rect.min.x + label_w;
                let pps = self.pps;
                let x_of = |t: f64| x0 + t as f32 * pps;
                let t_of = |x: f32| ((x - x0) / pps).max(0.0) as f64;
                let ruler = Rect::from_min_max(pos2(x0, rect.min.y), pos2(rect.max.x, rect.min.y + 20.0));
                let v1 = Rect::from_min_max(pos2(x0, ruler.max.y + 6.0), pos2(rect.max.x, ruler.max.y + 66.0));
                let a2 = Rect::from_min_max(pos2(x0, v1.max.y + 6.0), pos2(rect.max.x, v1.max.y + 40.0));
                let p = ui.painter_at(rect);
                p.rect_filled(ruler, CornerRadius::ZERO, BG);
                let step = [0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0].into_iter().find(|s| *s as f32 * pps >= 70.0).unwrap_or(600.0);
                let mut tt = 0.0;
                while tt <= dur {
                    let x = x_of(tt);
                    p.line_segment([pos2(x, ruler.min.y + 12.0), pos2(x, ruler.max.y)], Stroke::new(1.0, MUTED));
                    p.text(pos2(x + 3.0, ruler.min.y + 1.0), Align2::LEFT_TOP, fmt(tt), egui::FontId::monospace(10.0), MUTED);
                    tt += step;
                }
                for (r, l) in [(v1, "V1"), (a2, "A2")] {
                    p.rect_filled(r, CornerRadius::same(4), BG);
                    p.text(pos2(rect.min.x + 8.0, r.center().y), Align2::LEFT_CENTER, l, egui::FontId::proportional(12.0), MUTED);
                }
                let rr = ui.interact(ruler, ui.scope_id().with("ruler"), Sense::click_and_drag());
                if let Some(pp) = rr.interact_pointer_pos() {
                    self.seek(t_of(pp.x));
                }

                let starts = self.proj.starts();
                for (i, c) in self.proj.clips.iter().enumerate() {
                    let r = Rect::from_min_max(pos2(x_of(starts[i]), v1.min.y + 2.0), pos2(x_of(starts[i] + c.len()), v1.max.y - 2.0));
                    let on = self.sel == Some(Sel::Clip(i));
                    p.rect_filled(r, CornerRadius::same(5), ACCENT.gamma_multiply(if on { 0.6 } else { 0.35 }));
                    p.rect_stroke(r, CornerRadius::same(5), Stroke::new(if on { 2.0 } else { 1.0 }, if on { Color32::WHITE } else { ACCENT }), StrokeKind::Inside);
                    let name = self.proj.sources[c.src].name();
                    p.with_clip_rect(r.shrink(4.0)).text(r.min + vec2(8.0, 6.0), Align2::LEFT_TOP, name, egui::FontId::proportional(12.0), TEXT);
                    let resp = ui.interact(r, ui.scope_id().with(("clip", i)), Sense::click_and_drag());
                    if resp.clicked() {
                        self.sel = Some(Sel::Clip(i));
                    }
                    if resp.drag_started()
                        && let Some(pp) = resp.interact_pointer_pos()
                    {
                        let mode = if pp.x - r.min.x < 8.0 { 1 } else if r.max.x - pp.x < 8.0 { 2 } else { 0 };
                        self.drag = Some((Sel::Clip(i), mode, t_of(pp.x), c.clone(), 0.0));
                        self.sel = Some(Sel::Clip(i));
                    }
                    if resp.hovered()
                        && let Some(pp) = resp.hover_pos()
                        && (pp.x - r.min.x < 8.0 || r.max.x - pp.x < 8.0)
                    {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                }
                for (i, m) in self.proj.music.iter().enumerate() {
                    let r = Rect::from_min_max(pos2(x_of(m.start), a2.min.y + 2.0), pos2(x_of(m.start + m.out - m.inp), a2.max.y - 2.0));
                    let on = self.sel == Some(Sel::Music(i));
                    p.rect_filled(r, CornerRadius::same(5), MUSIC.gamma_multiply(if on { 0.5 } else { 0.3 }));
                    p.rect_stroke(r, CornerRadius::same(5), Stroke::new(if on { 2.0 } else { 1.0 }, if on { Color32::WHITE } else { MUSIC }), StrokeKind::Inside);
                    p.with_clip_rect(r.shrink(3.0)).text(r.min + vec2(8.0, 4.0), Align2::LEFT_TOP, self.proj.sources[m.src].name(), egui::FontId::proportional(11.0), TEXT);
                    let resp = ui.interact(r, ui.scope_id().with(("music", i)), Sense::click_and_drag());
                    if resp.clicked() {
                        self.sel = Some(Sel::Music(i));
                    }
                    if resp.drag_started()
                        && let Some(pp) = resp.interact_pointer_pos()
                    {
                        self.drag = Some((Sel::Music(i), 0, t_of(pp.x), Clip { src: 0, inp: 0.0, out: 0.0, volume: 0.0, fade_in: 0.0, fade_out: 0.0 }, m.start));
                        self.sel = Some(Sel::Music(i));
                    }
                }

                if let Some((sel, mode, from, orig, start0)) = self.drag.clone() {
                    let (down, pos) = ui.input(|i| (i.pointer.primary_down(), i.pointer.hover_pos()));
                    if let (true, Some(pp)) = (down, pos) {
                        let d = t_of(pp.x) - from;
                        match sel {
                            Sel::Clip(i) if i < self.proj.clips.len() => {
                                let max = self.proj.sources[orig.src].duration;
                                let c = &mut self.proj.clips[i];
                                match mode {
                                    1 => c.inp = (orig.inp + d).clamp(0.0, c.out - 0.1),
                                    2 => c.out = (orig.out + d).clamp(c.inp + 0.1, max),
                                    _ => {}
                                }
                            }
                            Sel::Music(i) if i < self.proj.music.len() => self.proj.music[i].start = (start0 + d).max(0.0),
                            _ => {}
                        }
                        if mode == 0
                            && let Sel::Clip(_) = sel
                        {
                            p.line_segment([pos2(pp.x, v1.min.y), pos2(pp.x, v1.max.y)], Stroke::new(2.0, Color32::WHITE));
                        }
                    } else {
                        if mode == 0
                            && let (Sel::Clip(i), Some(pp)) = (sel, pos)
                            && i < self.proj.clips.len()
                            && (t_of(pp.x) - from).abs() > 0.1
                        {
                            let t = t_of(pp.x);
                            let starts = self.proj.starts();
                            let mut to = starts.iter().zip(&self.proj.clips).filter(|(s, c)| t > **s + c.len() / 2.0).count();
                            let clip = self.proj.clips.remove(i);
                            if to > i {
                                to -= 1;
                            }
                            let to = to.min(self.proj.clips.len());
                            self.proj.clips.insert(to, clip);
                            self.sel = Some(Sel::Clip(to));
                        }
                        self.drag = None;
                    }
                }

                let x = x_of(self.t);
                p.line_segment([pos2(x, ruler.min.y), pos2(x, a2.max.y)], Stroke::new(2.0, Color32::from_rgb(255, 80, 80)));
                p.add(egui::Shape::convex_polygon(vec![pos2(x - 6.0, ruler.min.y), pos2(x + 6.0, ruler.min.y), pos2(x, ruler.min.y + 8.0)], Color32::from_rgb(255, 80, 80), Stroke::NONE));
                let bg = ui.interact(Rect::from_min_max(pos2(x0, v1.min.y), pos2(rect.max.x, a2.max.y)), ui.scope_id().with("tracks"), Sense::click());
                if bg.clicked()
                    && let Some(pp) = bg.interact_pointer_pos()
                {
                    self.seek(t_of(pp.x));
                    self.sel = None;
                }
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Cut")
            .with_icon(Arc::new(studio_ui::app_window_icon("cut")))
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([1000.0, 640.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Cut", options, Box::new(|cc| Ok(Box::new(Cut::new(cc)))))
}
