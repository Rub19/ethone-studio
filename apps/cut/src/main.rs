#![windows_subsystem = "windows"]

mod media;
mod model;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, Painter, Pos2,
    Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2, pos2,
    vec2,
};
use media::{OUT_RATE, VideoReader};
use model::{Clip, Music, Project, Source};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};

const VIDEO: Color32 = Color32::from_rgb(140, 92, 246);
const MUSIC: Color32 = Color32::from_rgb(48, 180, 110);
const PLAYHEAD: Color32 = Color32::from_rgb(255, 69, 58);
const EXT: [&str; 12] = ["mp4", "mov", "m4v", "mkv", "avi", "wmv", "webm", "mp3", "wav", "m4a", "aac", "wma"];
const PRESETS: [(&str, &str, u32, u32); 5] = [
    ("1080p", "Paysage 16:9", 1920, 1080),
    ("4K UHD", "Paysage 16:9", 3840, 2160),
    ("720p", "Paysage 16:9", 1280, 720),
    ("Vertical", "Shorts, Reels, TikTok", 1080, 1920),
    ("Carré", "Publication 1:1", 1080, 1080),
];

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
                        let Ok(mut s) = s2.lock() else { return };
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
    Thumb(usize, u32, u32, Vec<u8>),
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

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Blade,
}

struct Cut {
    proj: Project,
    saved: Project,
    file: Option<PathBuf>,
    home: bool,
    recents: Vec<PathBuf>,
    undo: Vec<Project>,
    redo: Vec<Project>,
    editing: bool,
    audio_data: HashMap<usize, Arc<Vec<f32>>>,
    thumbs: HashMap<usize, TextureHandle>,
    mixed: Option<Project>,
    readers: HashMap<usize, VideoReader>,
    tex: Option<TextureHandle>,
    shown: Option<(usize, f64)>,
    fade: f32,
    t: f64,
    playing: bool,
    sel: Option<Sel>,
    tool: Tool,
    pps: f32,
    drag: Option<(Sel, u8, f64, Clip, f64)>,
    audio: Audio,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    job: Option<ExportJob>,
    error: Option<String>,
    title: String,
    icon: TextureHandle,
}

fn recents_path() -> PathBuf {
    studio_core::data_dir().join("cut").join("recents.txt")
}

impl Cut {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        media::startup();
        let (tx, rx) = channel();
        let proj = Project::new();
        let recents = std::fs::read_to_string(recents_path())
            .map(|s| s.lines().map(PathBuf::from).filter(|p| p.exists()).collect())
            .unwrap_or_default();
        let mut me = Self {
            saved: proj.clone(),
            proj,
            file: None,
            home: true,
            recents,
            undo: Vec::new(),
            redo: Vec::new(),
            editing: false,
            audio_data: HashMap::new(),
            thumbs: HashMap::new(),
            mixed: None,
            readers: HashMap::new(),
            tex: None,
            shown: None,
            fade: 1.0,
            t: 0.0,
            playing: false,
            sel: None,
            tool: Tool::Select,
            pps: 60.0,
            drag: None,
            audio: Audio::new(),
            tx,
            rx,
            job: None,
            error: None,
            title: String::new(),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "cut"),
        };
        let args: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).filter(|p| p.is_file()).collect();
        if let Some(project) = args.iter().find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ecut"))).cloned() {
            me.open_path(&project, &cc.egui_ctx);
        } else if !args.is_empty() {
            me.home = false;
            me.import(args, &cc.egui_ctx);
        }
        me
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

    fn new_project(&mut self, w: u32, h: u32) {
        self.set_playing(false);
        self.proj = Project { width: w, height: h, ..Project::new() };
        self.saved = self.proj.clone();
        self.file = None;
        self.readers.clear();
        self.audio_data.clear();
        self.thumbs.clear();
        self.mixed = None;
        self.shown = None;
        self.undo.clear();
        self.redo.clear();
        self.sel = None;
        self.t = 0.0;
        self.home = false;
    }

    fn import_dialog(&mut self, ctx: &egui::Context) {
        if let Some(ps) = rfd::FileDialog::new().add_filter("Médias", &EXT).pick_files() {
            self.home = false;
            self.import(ps, ctx);
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

    fn load_media(&self, idx: usize, ctx: &egui::Context) {
        let src = self.proj.sources[idx].clone();
        let path = PathBuf::from(&src.path);
        if src.has_audio {
            let (tx, ctx, path) = (self.tx.clone(), ctx.clone(), path.clone());
            std::thread::spawn(move || {
                media::startup();
                let data = media::decode_audio(&path).unwrap_or_default();
                let _ = tx.send(Msg::Audio(idx, data));
                ctx.request_repaint();
            });
        }
        if src.has_video {
            let (tx, ctx) = (self.tx.clone(), ctx.clone());
            std::thread::spawn(move || {
                media::startup();
                let Ok((mut r, info)) = VideoReader::open(&path) else { return };
                let (w, h) = (r.width, r.height);
                if let Ok(Some((_, rgba))) = r.frame_at((info.duration / 3.0).min(1.0), info.fps)
                    && let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone())
                {
                    let tw = 240u32;
                    let th = ((h as f32 * tw as f32 / w.max(1) as f32) as u32).max(1);
                    let small = image::imageops::resize(&img, tw, th, image::imageops::FilterType::Triangle);
                    let _ = tx.send(Msg::Thumb(idx, tw, th, small.into_raw()));
                    ctx.request_repaint();
                }
            });
        }
    }

    fn pump(&mut self, ctx: &egui::Context) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Probed(_, Ok(src)) => {
                    let idx = self.proj.sources.len();
                    let add = src.has_video;
                    let dur = src.duration;
                    self.proj.sources.push(src);
                    if add {
                        self.proj.clips.push(Clip { src: idx, inp: 0.0, out: dur, volume: 1.0, fade_in: 0.0, fade_out: 0.0 });
                    } else {
                        let start = self.t;
                        self.proj.music.push(Music { src: idx, start, inp: 0.0, out: dur, volume: 0.6 });
                    }
                    self.load_media(idx, ctx);
                }
                Msg::Probed(p, Err(e)) => self.error = Some(format!("{} : {e}", p.display())),
                Msg::Audio(i, d) => {
                    self.audio_data.insert(i, Arc::new(d));
                    self.mixed = None;
                }
                Msg::Thumb(i, w, h, px) => {
                    let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &px);
                    self.thumbs.insert(i, ctx.load_texture(format!("thumb{i}"), img, TextureOptions::LINEAR));
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
        if let Ok(mut s) = self.audio.state.lock() {
            s.mix = Arc::new(mix);
        }
        self.mixed = Some(self.proj.clone());
    }

    fn set_playing(&mut self, on: bool) {
        self.playing = on && self.proj.duration() > 0.0;
        if self.playing && self.t >= self.proj.duration() - 0.01 {
            self.t = 0.0;
        }
        if let Ok(mut s) = self.audio.state.lock() {
            s.pos = self.t * OUT_RATE as f64;
            s.playing = self.playing;
        }
    }

    fn seek(&mut self, t: f64) {
        self.t = t.clamp(0.0, self.proj.duration().max(0.0));
        if let Ok(mut s) = self.audio.state.lock() {
            s.pos = self.t * OUT_RATE as f64;
        }
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
        let src = self.proj.sources[src_i].clone();
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
        let Some(r) = self.readers.get_mut(&src_i) else { return };
        let (w, h) = (r.width as usize, r.height as usize);
        match r.frame_at(st, src.fps) {
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
                            let r = readers.get_mut(&si).ok_or("lecteur")?;
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
            if let Err(e) = r
                && let Ok(mut slot) = e2.lock()
            {
                *slot = Some(e);
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
                self.saved = self.proj.clone();
                self.remember(&path);
                self.file = Some(path);
            }
            Err(e) => self.error = Some(format!("Enregistrement : {e}")),
        }
    }

    fn open_path(&mut self, p: &Path, ctx: &egui::Context) {
        match std::fs::read(p).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Project>(&b).map_err(|e| e.to_string())) {
            Ok(proj) => {
                self.new_project(proj.width, proj.height);
                self.saved = proj.clone();
                self.proj = proj;
                self.file = Some(p.to_path_buf());
                self.remember(p);
                for i in 0..self.proj.sources.len() {
                    self.load_media(i, ctx);
                }
            }
            Err(e) => self.error = Some(format!("Ouverture : {e}")),
        }
    }

    fn open(&mut self, ctx: &egui::Context) {
        if let Some(p) = rfd::FileDialog::new().add_filter("Projet Cut", &["ecut"]).pick_file() {
            self.open_path(&p, ctx);
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
        if ctx.egui_wants_keyboard_input() || self.home {
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
        if sc(M::COMMAND, Key::I) {
            self.import_dialog(ctx);
        }
        if sc(M::COMMAND, Key::M) {
            self.export();
        }
        let step = 1.0 / self.proj.fps as f64;
        let (space, s, del, left, right, home, end, v, c, plus, minus) = ctx.input(|i| {
            (
                i.key_pressed(Key::Space),
                i.key_pressed(Key::S),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::Home),
                i.key_pressed(Key::End),
                i.key_pressed(Key::V),
                i.key_pressed(Key::C) && !i.modifiers.command,
                i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals),
                i.key_pressed(Key::Minus),
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
        if v {
            self.tool = Tool::Select;
        }
        if c {
            self.tool = Tool::Blade;
        }
        if plus {
            self.pps = (self.pps * 1.25).min(600.0);
        }
        if minus {
            self.pps = (self.pps / 1.25).max(4.0);
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

fn tc(t: f64, fps: u32) -> String {
    let total = t.max(0.0);
    let m = (total / 60.0).floor();
    let s = (total - m * 60.0).floor();
    let f = ((total - total.floor()) * fps as f64).floor();
    format!("{:02}:{:02}:{:02}", m as u64, s as u64, f as u64)
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>()) }
}

impl eframe::App for Cut {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let before = self.proj.clone();
        self.pump(ctx);
        self.keys(ctx);
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        if !dropped.is_empty() {
            self.home = false;
            self.import(dropped, ctx);
        }
        self.remix();
        if self.playing {
            let pos = self.audio.state.lock().map(|s| s.pos).unwrap_or(0.0);
            self.t = if self.audio._stream.is_some() { pos / OUT_RATE as f64 } else { self.t + ctx.input(|i| i.stable_dt) as f64 };
            if self.t >= self.proj.duration() {
                self.t = self.proj.duration();
                self.set_playing(false);
            }
            ctx.request_repaint();
        }
        if !self.home {
            self.update_frame(ctx);
        }
        let name = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = if self.home { "Cut".to_string() } else { format!("{name}{} — Cut", if self.proj != self.saved { " •" } else { "" }) };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        self.menu_bar(root, ctx);
        self.status(root);
        if self.home {
            self.home_screen(root, ctx);
        } else {
            self.timeline(root);
            self.bin(root, ctx);
            self.inspector(root);
            self.viewer(root, &name);
        }

        if !self.home && self.proj != before && !self.editing {
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
    fn menu_bar(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::Panel::top("menu").exact_size(40.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(10, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add(egui::Image::new((self.icon.id(), Vec2::splat(22.0))));
                ui.add_space(4.0);
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouveau projet").clicked() {
                            self.home = true;
                        }
                        if ui.button("Ouvrir un projet…").clicked() {
                            self.open(ctx);
                        }
                        if ui.button("Importer des médias…        Ctrl+I").clicked() {
                            self.import_dialog(ctx);
                        }
                        ui.add_enabled_ui(!self.home, |ui| {
                            ui.separator();
                            if ui.button("Enregistrer                        Ctrl+S").clicked() {
                                self.save(false);
                            }
                            if ui.button("Enregistrer sous…").clicked() {
                                self.save(true);
                            }
                            ui.separator();
                            if ui.button("Exporter en MP4…               Ctrl+M").clicked() {
                                self.export();
                            }
                        });
                    });
                    ui.add_enabled_ui(!self.home, |ui| {
                        ui.menu_button("Édition", |ui| {
                            if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler                 Ctrl+Z")).clicked()
                                && let Some(p) = self.undo.pop()
                            {
                                self.redo.push(std::mem::replace(&mut self.proj, p));
                            }
                            if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir                Ctrl+Y")).clicked()
                                && let Some(p) = self.redo.pop()
                            {
                                self.undo.push(std::mem::replace(&mut self.proj, p));
                            }
                            ui.separator();
                            if ui.button("Couper à la tête de lecture   S").clicked() {
                                self.proj.split(self.t);
                            }
                            if ui.add_enabled(self.sel.is_some(), egui::Button::new("Supprimer                Suppr")).clicked() {
                                self.delete_sel();
                            }
                        });
                        ui.menu_button("Séquence", |ui| {
                            for (label, kind, w, h) in PRESETS {
                                if ui.selectable_label(self.proj.width == w && self.proj.height == h, format!("{label} · {kind}")).clicked() {
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
                    });
                });
                if !self.home {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let busy = self.job.as_ref().is_some_and(|j| j.done.load(Ordering::Relaxed) != usize::MAX);
                        if !busy && pro::primary(ui, "Exporter").clicked() {
                            self.export();
                        }
                        ui.label(RichText::new(format!("{} × {} · {} i/s", self.proj.width, self.proj.height, self.proj.fps)).size(12.5).color(MUTED));
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
                    ui.label(small(format!("Durée {}", tc(self.proj.duration(), self.proj.fps))));
                    ui.label(small("Espace lire · S couper · V sélection · C lame · + / − zoom".into()));
                }
                if let Some(j) = &self.job {
                    let d = j.done.load(Ordering::Relaxed);
                    if d == usize::MAX {
                        match j.error.lock().ok().and_then(|e| e.clone()) {
                            Some(e) => ui.label(RichText::new(format!("Export : {e}")).size(12.0).color(RED)),
                            None => ui.label(RichText::new(format!("Exporté : {}", j.path.display())).size(12.0).color(GREEN)),
                        };
                    } else {
                        ui.add(egui::ProgressBar::new(d as f32 / j.total.max(1) as f32).desired_width(220.0).text(format!("Export {d}/{}", j.total)));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
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

    fn home_screen(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::Panel::left("home-nav").exact_size(230.0).resizable(false).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(18, 26))).show(root, |ui| {
            if pro::primary(ui, "Nouveau projet").clicked() {
                self.new_project(1920, 1080);
            }
            ui.add_space(8.0);
            if pro::secondary(ui, "Ouvrir…").clicked() {
                self.open(ctx);
            }
            ui.add_space(8.0);
            if pro::secondary(ui, "Importer des médias…").clicked() {
                self.new_project(1920, 1080);
                self.import_dialog(ctx);
            }
        });
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin { left: 40, right: 40, top: 34, bottom: 20 })).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(40.0))));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Bienvenue dans Cut").font(semibold(28.0)).color(INK_STRONG));
                        ui.label(RichText::new("Monte, coupe et exporte tes vidéos en MP4").color(MUTED));
                    });
                });
                ui.add_space(28.0);
                ui.label(RichText::new("Nouveau projet").font(semibold(16.0)).color(INK));
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
                    self.new_project(w, h);
                }
                ui.add_space(26.0);
                ui.label(RichText::new("Projets récents").font(semibold(16.0)).color(INK));
                ui.add_space(10.0);
                if self.recents.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().min(760.0), 110.0), Sense::hover());
                    let p = ui.painter();
                    p.rect(r, CornerRadius::same(14), Color32::from_white_alpha(4), Stroke::new(1.0, LINE), StrokeKind::Inside);
                    p.text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "Dépose des vidéos ou de la musique n'importe où", semibold(15.0), INK);
                    p.text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "MP4, MOV, MKV, MP3, WAV, M4A…", FontId::proportional(13.0), MUTED);
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
                        self.open_path(&p, ctx);
                    }
                }
            });
        });
    }

    fn bin(&mut self, root: &mut Ui, ctx: &egui::Context) {
        egui::Panel::left("bin").resizable(false).exact_size(272.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            pro::dock_panel(ui, "media", "Médias", |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                if pro::secondary(ui, "+ Importer des médias").clicked() {
                    self.import_dialog(ctx);
                }
                if self.proj.sources.is_empty() {
                    ui.label(RichText::new("Glisse des vidéos ou de la musique dans la fenêtre.").size(12.5).color(FAINT));
                }
                let w = (ui.available_width() - 8.0) / 2.0;
                let mut add = None;
                for row in (0..self.proj.sources.len()).collect::<Vec<_>>().chunks(2) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        for &i in row {
                            let s = &self.proj.sources[i];
                            let (r, resp) = ui.allocate_exact_size(vec2(w, w * 0.56 + 38.0), Sense::click());
                            let p = ui.painter();
                            let pic = Rect::from_min_size(r.min, vec2(w, w * 0.56));
                            p.rect_filled(pic, CornerRadius::same(8), Color32::from_rgb(0x14, 0x14, 0x17));
                            match self.thumbs.get(&i) {
                                Some(t) => {
                                    let [tw, th] = t.size();
                                    let k = (pic.width() / tw as f32).min(pic.height() / th as f32);
                                    p.image(t.id(), Rect::from_center_size(pic.center(), vec2(tw as f32, th as f32) * k), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                                }
                                None if !s.has_video => {
                                    for k in 0..7 {
                                        let x = pic.center().x - 24.0 + k as f32 * 8.0;
                                        let hh = [8.0, 18.0, 28.0, 14.0, 24.0, 10.0, 16.0][k];
                                        p.line_segment([pos2(x, pic.center().y - hh / 2.0), pos2(x, pic.center().y + hh / 2.0)], Stroke::new(3.0, MUSIC));
                                    }
                                }
                                None => {}
                            }
                            let tag = Rect::from_min_size(pic.right_bottom() - vec2(52.0, 22.0), vec2(48.0, 18.0));
                            p.rect_filled(tag, CornerRadius::same(5), Color32::from_black_alpha(170));
                            p.text(tag.center(), Align2::CENTER_CENTER, tc(s.duration, 30).chars().take(5).collect::<String>(), FontId::proportional(11.0), Color32::WHITE);
                            if resp.hovered() {
                                p.rect_stroke(pic, CornerRadius::same(8), Stroke::new(1.5, BLUE), StrokeKind::Inside);
                            }
                            p.text(pos2(r.left() + 2.0, pic.bottom() + 10.0), Align2::LEFT_TOP, short(&s.name(), (w / 7.5) as usize), FontId::proportional(12.0), INK);
                            p.text(pos2(r.left() + 2.0, pic.bottom() + 25.0), Align2::LEFT_TOP, if s.has_video { "Vidéo" } else { "Audio" }, FontId::proportional(11.0), FAINT);
                            if resp.on_hover_text("Double-clic pour ajouter à la timeline").double_clicked() {
                                add = Some(i);
                            }
                        }
                    });
                }
                if let Some(i) = add {
                    let s = &self.proj.sources[i];
                    if s.has_video {
                        let d = s.duration;
                        self.proj.clips.push(Clip { src: i, inp: 0.0, out: d, volume: 1.0, fade_in: 0.0, fade_out: 0.0 });
                    } else {
                        let d = s.duration;
                        let start = self.t;
                        self.proj.music.push(Music { src: i, start, inp: 0.0, out: d, volume: 0.6 });
                    }
                }
            });
        });
    }

    fn inspector(&mut self, root: &mut Ui) {
        egui::Panel::right("inspector").resizable(false).exact_size(284.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            match self.sel {
                Some(Sel::Clip(i)) if i < self.proj.clips.len() => {
                    let src = self.proj.sources[self.proj.clips[i].src].clone();
                    let c = &mut self.proj.clips[i];
                    pro::dock_panel(ui, "clip", "Clip", |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.label(RichText::new(src.name()).font(semibold(13.5)).color(INK));
                        ui.label(RichText::new(format!("{} × {} · {:.0} i/s", src.width, src.height, src.fps)).size(12.0).color(FAINT));
                        field_time(ui, "Entrée", &mut c.inp, 0.0, c.out - 0.05);
                        field_time(ui, "Sortie", &mut c.out, c.inp + 0.05, src.duration);
                        pro::label_value(ui, "Durée", &format!("{:.2} s", c.len()));
                    });
                    pro::dock_panel(ui, "audio", "Son et fondus", |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(80.0);
                        ui.label(RichText::new("Volume").size(12.5).color(MUTED));
                        let mut vol = c.volume * 100.0;
                        if ui.add(egui::Slider::new(&mut vol, 0.0..=200.0).suffix(" %")).changed() {
                            c.volume = vol / 100.0;
                        }
                        let len = c.len();
                        ui.label(RichText::new("Fondu d'entrée").size(12.5).color(MUTED));
                        ui.add(egui::Slider::new(&mut c.fade_in, 0.0..=len.min(5.0)).suffix(" s"));
                        ui.label(RichText::new("Fondu de sortie").size(12.5).color(MUTED));
                        ui.add(egui::Slider::new(&mut c.fade_out, 0.0..=len.min(5.0)).suffix(" s"));
                    });
                }
                Some(Sel::Music(i)) if i < self.proj.music.len() => {
                    let src = self.proj.sources[self.proj.music[i].src].clone();
                    let m = &mut self.proj.music[i];
                    pro::dock_panel(ui, "music", "Musique", |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(80.0);
                        ui.label(RichText::new(src.name()).font(semibold(13.5)).color(INK));
                        field_time(ui, "Début", &mut m.start, 0.0, 36000.0);
                        field_time(ui, "Entrée", &mut m.inp, 0.0, m.out - 0.05);
                        field_time(ui, "Sortie", &mut m.out, m.inp + 0.05, src.duration);
                        ui.label(RichText::new("Volume").size(12.5).color(MUTED));
                        let mut vol = m.volume * 100.0;
                        if ui.add(egui::Slider::new(&mut vol, 0.0..=200.0).suffix(" %")).changed() {
                            m.volume = vol / 100.0;
                        }
                    });
                }
                _ => {
                    pro::dock_panel(ui, "sequence", "Séquence", |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        pro::label_value(ui, "Format", &format!("{} × {}", self.proj.width, self.proj.height));
                        pro::label_value(ui, "Images/s", &self.proj.fps.to_string());
                        pro::label_value(ui, "Clips", &self.proj.clips.len().to_string());
                        pro::label_value(ui, "Durée", &tc(self.proj.duration(), self.proj.fps));
                        ui.add_space(6.0);
                        ui.label(RichText::new("Clique un clip de la timeline pour régler son entrée, sa sortie, son volume et ses fondus.").size(12.0).color(FAINT));
                    });
                }
            }
        });
    }

    fn viewer(&mut self, root: &mut Ui, name: &str) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let head = Rect::from_min_size(full.min, vec2(full.width(), 34.0));
            ui.painter().text(head.left_center() + vec2(16.0, 0.0), Align2::LEFT_CENTER, format!("Programme · {name}"), FontId::proportional(12.5), MUTED);
            let controls_h = 56.0;
            let screen = Rect::from_min_max(pos2(full.left() + 16.0, head.bottom()), pos2(full.right() - 16.0, full.bottom() - controls_h));
            let (w, h) = (self.proj.width as f32, self.proj.height as f32);
            let s = (screen.width() / w).min(screen.height() / h);
            let frame = Rect::from_center_size(screen.center(), vec2(w, h) * s);
            let p = ui.painter();
            p.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(140) }.as_shape(frame, CornerRadius::same(4)));
            p.rect_filled(frame, CornerRadius::same(4), Color32::BLACK);
            if let (Some(t), Some(_)) = (&self.tex, self.shown) {
                let [tw, th] = t.size();
                let k = (frame.width() / tw as f32).min(frame.height() / th as f32);
                let r = Rect::from_center_size(frame.center(), vec2(tw as f32, th as f32) * k);
                let g = (self.fade * 255.0) as u8;
                p.image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::from_rgb(g, g, g));
            } else if self.proj.clips.is_empty() {
                p.text(frame.center(), Align2::CENTER_CENTER, "Importe une vidéo pour commencer", FontId::proportional(16.0), MUTED);
            }

            let bar = Rect::from_min_max(pos2(full.left() + 16.0, full.bottom() - controls_h + 8.0), pos2(full.right() - 16.0, full.bottom() - 8.0));
            let p = ui.painter();
            p.text(bar.left_center(), Align2::LEFT_CENTER, tc(self.t, self.proj.fps), FontId::monospace(16.0), BLUE);
            p.text(bar.right_center(), Align2::RIGHT_CENTER, tc(self.proj.duration(), self.proj.fps), FontId::monospace(14.0), MUTED);
            let step = 1.0 / self.proj.fps as f64;
            let center = bar.center();
            let buttons: [(f32, u8, &str); 5] = [(-104.0, 0, "Début"), (-56.0, 1, "Image précédente"), (0.0, 2, "Lecture / pause (Espace)"), (56.0, 3, "Image suivante"), (104.0, 4, "Fin")];
            for (dx, kind, tip) in buttons {
                let size = if kind == 2 { 42.0 } else { 34.0 };
                let r = Rect::from_center_size(center + vec2(dx, 0.0), Vec2::splat(size));
                let resp = ui.interact(r, ui.make_persistent_id(("transport", kind)), Sense::click()).on_hover_text(tip);
                let fill = if kind == 2 { BLUE } else if resp.hovered() { FIELD } else { Color32::TRANSPARENT };
                ui.painter().circle_filled(r.center(), size / 2.0, if kind == 2 && resp.hovered() { BLUE.lerp_to_gamma(Color32::WHITE, 0.12) } else { fill });
                transport_glyph(ui.painter(), r.center(), kind, self.playing, if kind == 2 { Color32::WHITE } else { INK });
                if resp.clicked() {
                    match kind {
                        0 => self.seek(0.0),
                        1 => self.seek(self.t - step),
                        2 => self.set_playing(!self.playing),
                        3 => self.seek(self.t + step),
                        _ => {
                            let d = self.proj.duration();
                            self.seek(d);
                        }
                    }
                }
            }
        });
    }

    fn timeline(&mut self, root: &mut Ui) {
        egui::Panel::bottom("timeline").resizable(true).default_size(300.0).min_size(250.0).frame(Frame::new().fill(PANEL).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let head_h = 40.0;
            let full = ui.max_rect();
            let head = Rect::from_min_size(full.min, vec2(full.width(), head_h));
            ui.scope_builder(egui::UiBuilder::new().max_rect(head.shrink2(vec2(12.0, 4.0))), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("Timeline").font(semibold(13.0)).color(INK));
                    ui.add_space(10.0);
                    if pro::icon_button(ui, 30.0, self.tool == Tool::Select, "Sélection (V)", |p, r, c| {
                        p.add(Shape::convex_polygon(vec![r.min + vec2(r.width() * 0.2, 0.0), r.min + vec2(r.width() * 0.85, r.height() * 0.6), r.min + vec2(r.width() * 0.52, r.height() * 0.62), r.min + vec2(r.width() * 0.33, r.height())], c, Stroke::NONE));
                    }).clicked() {
                        self.tool = Tool::Select;
                    }
                    if pro::icon_button(ui, 30.0, self.tool == Tool::Blade, "Lame (C) : clique un clip pour le couper", |p, r, c| {
                        p.add(Shape::convex_polygon(vec![r.min + vec2(r.width() * 0.15, r.height()), r.min + vec2(r.width() * 0.75, 0.0), r.min + vec2(r.width(), r.height() * 0.25), r.min + vec2(r.width() * 0.4, r.height())], c, Stroke::NONE));
                    }).clicked() {
                        self.tool = Tool::Blade;
                    }
                    ui.add_space(6.0);
                    if pro::secondary(ui, "Couper à la tête (S)").clicked() {
                        self.proj.split(self.t);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().slider_width = 140.0;
                        ui.add(egui::Slider::new(&mut self.pps, 4.0..=600.0).logarithmic(true).show_value(false));
                        ui.label(RichText::new("Zoom").size(12.5).color(MUTED));
                    });
                });
            });

            let label_w = 136.0;
            let dur = self.proj.duration().max(10.0) + 8.0;
            let body = Rect::from_min_max(pos2(full.left(), head.bottom()), full.max);
            ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| egui::ScrollArea::horizontal().auto_shrink(false).show(ui, |ui| {
                let width = (dur as f32 * self.pps).max(ui.available_width() - label_w);
                let (rect, _) = ui.allocate_exact_size(vec2(width + label_w, ui.available_height().max(170.0)), Sense::hover());
                let x0 = rect.min.x + label_w;
                let pps = self.pps;
                let x_of = |t: f64| x0 + t as f32 * pps;
                let t_of = |x: f32| ((x - x0) / pps).max(0.0) as f64;
                let ruler = Rect::from_min_max(pos2(x0, rect.min.y), pos2(rect.max.x, rect.min.y + 24.0));
                let v1 = Rect::from_min_max(pos2(x0, ruler.max.y + 8.0), pos2(rect.max.x, ruler.max.y + 84.0));
                let a2 = Rect::from_min_max(pos2(x0, v1.max.y + 8.0), pos2(rect.max.x, v1.max.y + 56.0));
                let p = ui.painter_at(rect);
                p.rect_filled(ruler, CornerRadius::ZERO, Color32::from_rgb(0x1a, 0x1a, 0x1d));
                let step = [0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0].into_iter().find(|s| *s as f32 * pps >= 80.0).unwrap_or(600.0);
                let mut tt = 0.0;
                while tt <= dur {
                    let x = x_of(tt);
                    p.line_segment([pos2(x, ruler.min.y + 14.0), pos2(x, ruler.max.y)], Stroke::new(1.0, FAINT));
                    p.text(pos2(x + 4.0, ruler.min.y + 3.0), Align2::LEFT_TOP, tc(tt, self.proj.fps), FontId::monospace(10.0), MUTED);
                    tt += step;
                }
                for (r, label, muted, kind) in [(v1, "V1  Vidéo", self.proj.mute_v1, 0), (a2, "A2  Musique", self.proj.mute_a2, 1)] {
                    p.rect_filled(r, CornerRadius::same(6), Color32::from_rgb(0x19, 0x19, 0x1c));
                    let head_r = Rect::from_min_max(pos2(rect.min.x + 6.0, r.min.y), pos2(x0 - 6.0, r.max.y));
                    p.rect_filled(head_r, CornerRadius::same(6), CHROME);
                    p.text(head_r.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, label, semibold(12.0), INK);
                    let mr = Rect::from_center_size(pos2(head_r.right() - 16.0, head_r.center().y), Vec2::splat(22.0));
                    let mresp = ui.interact(mr, ui.make_persistent_id(("mute", kind)), Sense::click()).on_hover_text(if muted { "Réactiver le son" } else { "Couper le son de la piste" });
                    speaker(&p, mr.center(), muted, if muted { RED } else if mresp.hovered() { INK } else { MUTED });
                    if mresp.clicked() {
                        if kind == 0 {
                            self.proj.mute_v1 = !self.proj.mute_v1;
                        } else {
                            self.proj.mute_a2 = !self.proj.mute_a2;
                        }
                    }
                }
                let rr = ui.interact(ruler, ui.make_persistent_id("ruler"), Sense::click_and_drag());
                if let Some(pp) = rr.interact_pointer_pos() {
                    self.seek(t_of(pp.x));
                }

                let starts = self.proj.starts();
                for (i, c) in self.proj.clips.clone().iter().enumerate() {
                    let r = Rect::from_min_max(pos2(x_of(starts[i]) + 1.0, v1.min.y + 3.0), pos2(x_of(starts[i] + c.len()) - 1.0, v1.max.y - 3.0));
                    let on = self.sel == Some(Sel::Clip(i));
                    p.rect_filled(r, CornerRadius::same(6), VIDEO.gamma_multiply(if on { 0.75 } else { 0.5 }));
                    let clip_p = p.with_clip_rect(r.shrink(1.0));
                    if let Some(t) = self.thumbs.get(&c.src) {
                        let [tw, th] = t.size();
                        let th_h = r.height() * 0.62;
                        let tw_w = th_h * tw as f32 / th as f32;
                        let mut x = r.left();
                        while x < r.right() && x < r.left() + tw_w * 6.0 {
                            clip_p.image(t.id(), Rect::from_min_size(pos2(x, r.top()), vec2(tw_w, th_h)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::from_white_alpha(200));
                            x += tw_w;
                        }
                    }
                    if let Some(data) = self.audio_data.get(&c.src) {
                        let wave = Rect::from_min_max(pos2(r.left(), r.top() + r.height() * 0.64), r.max);
                        waveform(&clip_p, wave, data, c.inp, c.out, Color32::from_white_alpha(170));
                    }
                    clip_p.text(r.left_top() + vec2(8.0, 4.0), Align2::LEFT_TOP, self.proj.sources[c.src].name(), semibold(11.5), Color32::WHITE);
                    p.rect_stroke(r, CornerRadius::same(6), Stroke::new(if on { 2.0 } else { 1.0 }, if on { Color32::WHITE } else { VIDEO }), StrokeKind::Inside);
                    let resp = ui.interact(r, ui.make_persistent_id(("clip", i)), Sense::click_and_drag());
                    if self.tool == Tool::Blade {
                        if let Some(hp) = resp.hover_pos() {
                            p.line_segment([pos2(hp.x, v1.min.y), pos2(hp.x, v1.max.y)], Stroke::new(1.5, Color32::WHITE));
                        }
                        if resp.clicked()
                            && let Some(pp) = resp.interact_pointer_pos()
                        {
                            self.proj.split(t_of(pp.x));
                        }
                        continue;
                    }
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
                    if let Some(hp) = resp.hover_pos()
                        && (hp.x - r.min.x < 8.0 || r.max.x - hp.x < 8.0)
                    {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                }
                for (i, m) in self.proj.music.clone().iter().enumerate() {
                    let r = Rect::from_min_max(pos2(x_of(m.start) + 1.0, a2.min.y + 3.0), pos2(x_of(m.start + m.out - m.inp) - 1.0, a2.max.y - 3.0));
                    let on = self.sel == Some(Sel::Music(i));
                    p.rect_filled(r, CornerRadius::same(6), MUSIC.gamma_multiply(if on { 0.7 } else { 0.45 }));
                    let clip_p = p.with_clip_rect(r.shrink(1.0));
                    if let Some(data) = self.audio_data.get(&m.src) {
                        waveform(&clip_p, r.shrink2(vec2(0.0, 4.0)), data, m.inp, m.out, Color32::from_white_alpha(170));
                    }
                    clip_p.text(r.left_top() + vec2(8.0, 3.0), Align2::LEFT_TOP, self.proj.sources[m.src].name(), semibold(11.0), Color32::WHITE);
                    p.rect_stroke(r, CornerRadius::same(6), Stroke::new(if on { 2.0 } else { 1.0 }, if on { Color32::WHITE } else { MUSIC }), StrokeKind::Inside);
                    let resp = ui.interact(r, ui.make_persistent_id(("music", i)), Sense::click_and_drag());
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
                        if mode == 0 && matches!(sel, Sel::Clip(_)) {
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
                p.line_segment([pos2(x, ruler.min.y), pos2(x, a2.max.y)], Stroke::new(1.5, PLAYHEAD));
                p.add(Shape::convex_polygon(vec![pos2(x - 7.0, ruler.min.y), pos2(x + 7.0, ruler.min.y), pos2(x + 7.0, ruler.min.y + 8.0), pos2(x, ruler.min.y + 14.0), pos2(x - 7.0, ruler.min.y + 8.0)], PLAYHEAD, Stroke::NONE));
                let bg = ui.interact(Rect::from_min_max(pos2(x0, v1.min.y), pos2(rect.max.x, a2.max.y)), ui.make_persistent_id("tracks"), Sense::click());
                if bg.clicked()
                    && let Some(pp) = bg.interact_pointer_pos()
                {
                    self.seek(t_of(pp.x));
                    self.sel = None;
                }
            }));
        });
    }
}

fn waveform(p: &Painter, r: Rect, data: &[f32], inp: f64, out: f64, color: Color32) {
    let frames = data.len() / 2;
    if frames == 0 || r.width() < 2.0 {
        return;
    }
    let a = (inp * OUT_RATE as f64) as usize;
    let b = ((out * OUT_RATE as f64) as usize).min(frames);
    if b <= a {
        return;
    }
    let cols = r.width() as usize;
    let per = ((b - a) as f32 / cols as f32).max(1.0);
    let stride = ((per / 64.0) as usize).max(1);
    let mid = r.center().y;
    let amp = r.height() * 0.48;
    for x in 0..cols {
        let s = a + (x as f32 * per) as usize;
        let e = (a + ((x + 1) as f32 * per) as usize).min(b);
        let mut peak = 0f32;
        let mut k = s;
        while k < e {
            peak = peak.max(data[k * 2].abs()).max(data[k * 2 + 1].abs());
            k += stride;
        }
        let h = (peak.min(1.0) * amp).max(0.5);
        let px = r.left() + x as f32 + 0.5;
        p.line_segment([pos2(px, mid - h), pos2(px, mid + h)], Stroke::new(1.0, color));
    }
}

fn speaker(p: &Painter, c: Pos2, muted: bool, color: Color32) {
    p.add(Shape::convex_polygon(vec![c + vec2(-7.0, -3.0), c + vec2(-4.0, -3.0), c + vec2(1.0, -7.0), c + vec2(1.0, 7.0), c + vec2(-4.0, 3.0), c + vec2(-7.0, 3.0)], color, Stroke::NONE));
    if muted {
        p.line_segment([c + vec2(3.5, -3.5), c + vec2(8.5, 3.5)], Stroke::new(1.6, color));
        p.line_segment([c + vec2(8.5, -3.5), c + vec2(3.5, 3.5)], Stroke::new(1.6, color));
    } else {
        p.line_segment([c + vec2(4.0, -3.0), c + vec2(4.0, 3.0)], Stroke::new(1.6, color));
        p.line_segment([c + vec2(7.0, -5.5), c + vec2(7.0, 5.5)], Stroke::new(1.6, color));
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

fn field_time(ui: &mut Ui, label: &str, v: &mut f64, min: f64, max: f64) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(egui::DragValue::new(v).range(min..=max.max(min)).speed(0.02).max_decimals(2).suffix(" s"));
        });
    });
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
    p.add(Shape::convex_polygon(vec![shape.center() + vec2(-5.0, -7.0), shape.center() + vec2(7.0, 0.0), shape.center() + vec2(-5.0, 7.0)], MUTED, Stroke::NONE));
    p.text(pos2(r.center().x, r.bottom() - 36.0), Align2::CENTER_CENTER, name, semibold(13.0), INK_STRONG);
    p.text(pos2(r.center().x, r.bottom() - 17.0), Align2::CENTER_CENTER, detail, FontId::proportional(11.0), MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Cut")
            .with_icon(Arc::new(studio_ui::app_window_icon("cut")))
            .with_inner_size([1520.0, 940.0])
            .with_min_inner_size([1100.0, 680.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Cut", options, Box::new(|cc| Ok(Box::new(Cut::new(cc)))))
}
