#![windows_subsystem = "windows"]

mod inspect;
mod look;
mod media;
mod model;
mod render;
mod timeline;
mod tutos;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, Painter, Pos2,
    Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2, pos2,
    vec2,
};
use media::{OUT_RATE, VideoReader};
use model::{Clip, GENERATORS, LABELS, MARKER_COLORS, Marker, Music, Project, STILL_EXT, Source, Title};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, WELL, semibold};

const MUSIC: Color32 = Color32::from_rgb(48, 180, 110);
const TITLE: Color32 = Color32::from_rgb(255, 159, 10);
const PLAYHEAD: Color32 = Color32::from_rgb(255, 69, 58);
const MARK: Color32 = Color32::from_rgb(255, 196, 0);
const EXT: [&str; 18] = ["mp4", "mov", "m4v", "mkv", "avi", "wmv", "webm", "mp3", "wav", "m4a", "aac", "wma", "png", "jpg", "jpeg", "bmp", "webp", "gif"];
const PRESETS: [(&str, &str, u32, u32); 12] = [
    ("1080p", "Paysage 16:9", 1920, 1080),
    ("4K UHD", "Paysage 16:9", 3840, 2160),
    ("1440p", "Paysage 16:9", 2560, 1440),
    ("720p", "Paysage 16:9", 1280, 720),
    ("Vertical", "Shorts, Reels, TikTok", 1080, 1920),
    ("Carré", "Publication 1:1", 1080, 1080),
    ("Portrait 4:5", "Instagram", 1080, 1350),
    ("Cinéma", "21:9", 2560, 1080),
    ("Scope", "2,39:1", 2560, 1072),
    ("4:3", "Format TV", 1440, 1080),
    ("4K DCI", "Cinéma 17:9", 4096, 2160),
    ("Vertical 720", "Stories légères", 720, 1280),
];
const FPS: [u32; 8] = [12, 15, 24, 25, 30, 48, 50, 60];
const KINDS: [&str; 6] = ["Vidéo MP4", "Audio WAV", "GIF animé", "Séquence PNG", "Image PNG", "Image JPEG"];
const ABR: [(&str, u32); 4] = [("96 kb/s", 12_000), ("128 kb/s", 16_000), ("160 kb/s", 20_000), ("192 kb/s", 24_000)];
const EXPORT_PRESETS: [(&str, usize, u32, u32, u32, f32, bool); 16] = [
    ("YouTube 1080p", 0, 1920, 1080, 0, 12.0, false),
    ("YouTube 1440p", 0, 2560, 1440, 0, 24.0, false),
    ("YouTube 4K", 0, 3840, 2160, 0, 45.0, false),
    ("YouTube 720p", 0, 1280, 720, 0, 7.5, false),
    ("TikTok · Reels · Shorts", 0, 1080, 1920, 30, 12.0, true),
    ("Instagram carré", 0, 1080, 1080, 30, 8.0, true),
    ("Instagram portrait", 0, 1080, 1350, 30, 8.0, true),
    ("X / Twitter", 0, 1280, 720, 30, 6.0, false),
    ("Vimeo HD", 0, 1920, 1080, 0, 20.0, false),
    ("Master haute qualité", 0, 0, 0, 0, 60.0, false),
    ("Brouillon rapide", 0, 854, 480, 0, 2.5, false),
    ("GIF animé", 2, 480, 0, 15, 0.0, false),
    ("Audio WAV", 1, 0, 0, 0, 0.0, false),
    ("Séquence PNG", 3, 0, 0, 0, 0.0, false),
    ("Image PNG", 4, 0, 0, 0, 0.0, false),
    ("Image JPEG", 5, 0, 0, 0, 0.0, false),
];

struct PlayState {
    mix: Arc<Vec<f32>>,
    pos: f64,
    playing: bool,
    volume: f32,
}

struct Audio {
    state: Arc<Mutex<PlayState>>,
    _stream: Option<cpal::Stream>,
}

impl Audio {
    fn new() -> Self {
        let state = Arc::new(Mutex::new(PlayState { mix: Arc::new(Vec::new()), pos: 0.0, playing: false, volume: 1.0 }));
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
                                let (l, r) = (s.mix[i * 2] * s.volume, s.mix[i * 2 + 1] * s.volume);
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
    cancel: Arc<AtomicBool>,
    path: PathBuf,
    open_after: bool,
    seen: bool,
}

#[derive(Clone)]
struct ExportOpts {
    preset: usize,
    kind: usize,
    w: u32,
    h: u32,
    fps: u32,
    mbps: f32,
    abr: usize,
    fill: bool,
    range: usize,
    open_after: bool,
}

impl ExportOpts {
    fn from_preset(k: usize) -> Self {
        let (_, kind, w, h, fps, mbps, fill) = EXPORT_PRESETS[k];
        ExportOpts { preset: k, kind, w, h, fps, mbps, abr: 3, fill, range: 0, open_after: false }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Sel {
    Clip(usize),
    Over(usize),
    Music(usize),
    Title(usize),
    Gap(usize),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tool {
    Select,
    TrackFwd,
    TrackBack,
    Ripple,
    Rolling,
    Rate,
    Blade,
    BladeAll,
    Slip,
    Slide,
    Pen,
    Rect,
    Ellipse,
    Hand,
    Zoom,
    Text,
    TextV,
}

type Group = (Key, &'static [(Tool, &'static str, &'static str)]);

const GROUPS: [Group; 7] = [
    (Key::V, &[(Tool::Select, "Sélection", "V"), (Tool::TrackFwd, "Sélection de piste avant", "A"), (Tool::TrackBack, "Sélection de piste arrière", "Maj+A")]),
    (Key::B, &[(Tool::Ripple, "Modification par propagation", "B"), (Tool::Rolling, "Modification par déplacement", "N"), (Tool::Rate, "Allongement de la durée", "R")]),
    (Key::C, &[(Tool::Blade, "Cutter", "C"), (Tool::BladeAll, "Couper toutes les pistes", "Maj+C")]),
    (Key::Y, &[(Tool::Slip, "Glissement", "Y"), (Tool::Slide, "Déplacement", "U")]),
    (Key::P, &[(Tool::Pen, "Plume (masque)", "P"), (Tool::Rect, "Rectangle (masque)", "P"), (Tool::Ellipse, "Ellipse (masque)", "P")]),
    (Key::H, &[(Tool::Hand, "Main", "H"), (Tool::Zoom, "Zoom", "Z")]),
    (Key::T, &[(Tool::Text, "Texte", "T"), (Tool::TextV, "Texte vertical", "T")]),
];

#[derive(Clone)]
struct Drag {
    sel: Sel,
    mode: u8,
    from: f64,
    orig: Project,
    start0: f64,
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
    pics: render::Pics,
    tex: Option<TextureHandle>,
    shown: bool,
    t: f64,
    playing: bool,
    shuttle: f64,
    sel: Option<Sel>,
    extra: Vec<Sel>,
    tool: Tool,
    group_last: [usize; 7],
    pps: f32,
    drag: Option<Drag>,
    audio: Audio,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    job: Option<ExportJob>,
    error: Option<String>,
    title: String,
    icon: TextureHandle,
    help: studio_ui::help::Help,
    composed: Option<(i64, Project, u32, u32)>,
    looping: bool,
    snap: bool,
    show_waves: bool,
    show_thumbs: bool,
    track_scale: f32,
    clip_board: Option<Clip>,
    grade_board: Option<Clip>,
    mark_in: Option<f64>,
    mark_out: Option<f64>,
    tab: usize,
    color_ch: usize,
    export: Option<ExportOpts>,
    seq_sheet: bool,
    speed_sheet: bool,
    marker_edit: Option<usize>,
    bin_query: String,
    bin_list: usize,
    bin_sort: usize,
    bin_filter: usize,
    volume: f32,
    safe: bool,
    grid: bool,
    guides: bool,
    quality: usize,
    focus: bool,
    tl_scroll: f32,
    force_scroll: Option<f32>,
    pen: Vec<[f32; 2]>,
    view_drag: Option<(u8, Pos2, Project, Sel, usize)>,
    ctx_clone: Option<egui::Context>,
}

fn recents_path() -> PathBuf {
    studio_core::data_dir().join("cut").join("recents.txt")
}

fn is_still(p: &Path) -> bool {
    p.extension().is_some_and(|e| STILL_EXT.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

impl Cut {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        media::startup();
        let (tx, rx) = channel();
        let proj = Project::new();
        let recents = std::fs::read_to_string(recents_path()).map(|s| s.lines().map(PathBuf::from).filter(|p| p.exists()).collect()).unwrap_or_default();
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
            pics: render::Pics::default(),
            tex: None,
            shown: false,
            t: 0.0,
            playing: false,
            shuttle: 1.0,
            sel: None,
            extra: Vec::new(),
            tool: Tool::Select,
            group_last: [0; 7],
            pps: 60.0,
            drag: None,
            audio: Audio::new(),
            tx,
            rx,
            job: None,
            error: None,
            title: String::new(),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "cut"),
            help: studio_ui::help::Help::new("Cut", tutos::CUT),
            composed: None,
            looping: false,
            snap: true,
            show_waves: true,
            show_thumbs: true,
            track_scale: 1.0,
            clip_board: None,
            grade_board: None,
            mark_in: None,
            mark_out: None,
            tab: 0,
            color_ch: 0,
            export: None,
            seq_sheet: false,
            speed_sheet: false,
            marker_edit: None,
            bin_query: String::new(),
            bin_list: 0,
            bin_sort: 0,
            bin_filter: 0,
            volume: 1.0,
            safe: false,
            grid: false,
            guides: false,
            quality: 0,
            focus: false,
            tl_scroll: 0.0,
            force_scroll: None,
            pen: Vec::new(),
            view_drag: None,
            ctx_clone: None,
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
        self.pics.clear();
        self.audio_data.clear();
        self.thumbs.clear();
        self.mixed = None;
        self.shown = false;
        self.composed = None;
        self.undo.clear();
        self.redo.clear();
        self.sel = None;
        self.extra.clear();
        self.t = 0.0;
        self.mark_in = None;
        self.mark_out = None;
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
                let r = if is_still(&p) {
                    render::probe_still(&p)
                } else {
                    VideoReader::open(&p).map(|(_, i)| Source { path: p.to_string_lossy().into_owned(), duration: i.duration, width: i.width, height: i.height, fps: i.fps, has_video: i.has_video, has_audio: i.has_audio || !i.has_video, label: 0 })
                };
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
        if !src.has_video {
            return;
        }
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            media::startup();
            let pic: Option<(u32, u32, Vec<u8>)> = if src.is_generator() {
                Some((160, 90, render::generate(&src.path, 160, 90)))
            } else if src.is_still() {
                render::load_still(&src.path).ok().map(|(w, h, px)| (w, h, px.to_vec()))
            } else {
                VideoReader::open(&path).ok().and_then(|(mut r, info)| {
                    let (w, h) = (r.width, r.height);
                    r.frame_at((info.duration / 3.0).min(1.0), info.fps).ok().flatten().map(|(_, px)| (w, h, px.clone()))
                })
            };
            if let Some((w, h, rgba)) = pic
                && let Some(img) = image::RgbaImage::from_raw(w, h, rgba)
            {
                let tw = 240u32;
                let th = ((h as f32 * tw as f32 / w.max(1) as f32) as u32).max(1);
                let small = image::imageops::resize(&img, tw, th, image::imageops::FilterType::Triangle);
                let _ = tx.send(Msg::Thumb(idx, tw, th, small.into_raw()));
                ctx.request_repaint();
            }
        });
    }

    fn new_clip(&self, i: usize) -> Clip {
        let s = &self.proj.sources[i];
        let out = if s.is_generator() || s.is_still() { 5.0 } else { s.duration };
        Clip { src: i, out, ..Default::default() }
    }

    fn add_source(&mut self, i: usize) {
        if self.proj.sources[i].has_video {
            let c = self.new_clip(i);
            self.proj.clips.push(c);
            self.select_one(Sel::Clip(self.proj.clips.len() - 1));
        } else {
            self.add_music(i);
        }
    }

    fn insert_source(&mut self, i: usize) {
        if !self.proj.sources[i].has_video {
            self.add_music(i);
            return;
        }
        let c = self.new_clip(i);
        let at = match self.proj.local_at(self.t) {
            Some((k, local)) if local < 0.05 => k,
            Some((k, _)) => {
                self.proj.split(self.t);
                k + 1
            }
            None => self.proj.starts().iter().position(|s| *s > self.t).unwrap_or(self.proj.clips.len()),
        };
        let len = c.len();
        self.proj.shift_after(self.t, len);
        self.proj.clips.insert(at, c);
        self.select_one(Sel::Clip(at));
    }

    fn overlay_source(&mut self, i: usize) {
        if !self.proj.sources[i].has_video {
            return;
        }
        let c = Clip { start: self.t, ..self.new_clip(i) };
        self.proj.overlays.push(c);
        self.select_one(Sel::Over(self.proj.overlays.len() - 1));
    }

    fn replace_source(&mut self, i: usize) {
        let Some(s) = self.sel else { return };
        let dur = self.proj.sources[i].duration;
        let still = self.proj.sources[i].is_still() || self.proj.sources[i].is_generator();
        if let Some(c) = self.clip_mut(s) {
            let len = c.out - c.inp;
            c.src = i;
            if !still && c.out > dur {
                c.inp = (dur - len).max(0.0);
                c.out = dur;
            }
        }
    }

    fn add_music(&mut self, i: usize) {
        let d = self.proj.sources[i].duration;
        self.proj.music.push(Music { src: i, start: self.t, inp: 0.0, out: d, volume: 0.6, ..Default::default() });
        self.select_one(Sel::Music(self.proj.music.len() - 1));
    }

    fn add_generator(&mut self, k: usize) {
        let (w, h) = (self.proj.width, self.proj.height);
        self.proj.sources.push(Source { path: GENERATORS[k].0.to_string(), duration: 3600.0, width: w, height: h, fps: self.proj.fps as f64, has_video: true, has_audio: false, label: 0 });
        let i = self.proj.sources.len() - 1;
        self.thumbs.remove(&i);
        let ctx = self.ctx_clone.clone();
        if let Some(ctx) = ctx {
            self.load_media(i, &ctx);
        }
        self.insert_source(i);
    }

    fn remove_source(&mut self, i: usize) {
        if self.uses(i) > 0 {
            self.error = Some("Ce média est utilisé dans la timeline".into());
            return;
        }
        self.proj.sources.remove(i);
        for c in self.proj.clips.iter_mut().chain(self.proj.overlays.iter_mut()) {
            if c.src > i {
                c.src -= 1;
            }
        }
        for m in &mut self.proj.music {
            if m.src > i {
                m.src -= 1;
            }
        }
        fn shift<T>(map: &mut HashMap<usize, T>, i: usize) {
            let old = std::mem::take(map);
            for (k, v) in old {
                if k != i {
                    map.insert(if k > i { k - 1 } else { k }, v);
                }
            }
        }
        shift(&mut self.thumbs, i);
        shift(&mut self.audio_data, i);
        self.pics.clear();
        self.composed = None;
        self.mixed = None;
    }

    fn uses(&self, i: usize) -> usize {
        self.proj.clips.iter().chain(&self.proj.overlays).filter(|c| c.src == i).count() + self.proj.music.iter().filter(|m| m.src == i).count()
    }

    fn remove_unused(&mut self) {
        for i in (0..self.proj.sources.len()).rev() {
            if self.uses(i) == 0 {
                self.remove_source(i);
            }
        }
    }

    fn relink(&mut self, i: usize, ctx: &egui::Context) {
        let Some(p) = rfd::FileDialog::new().add_filter("Médias", &EXT).pick_file() else { return };
        let probe = if is_still(&p) {
            render::probe_still(&p)
        } else {
            VideoReader::open(&p).map(|(_, info)| Source { path: p.to_string_lossy().into_owned(), duration: info.duration, width: info.width, height: info.height, fps: info.fps, has_video: info.has_video, has_audio: info.has_audio || !info.has_video, label: 0 })
        };
        match probe {
            Ok(mut s) => {
                s.label = self.proj.sources[i].label;
                self.proj.sources[i] = s;
                self.pics.clear();
                self.thumbs.remove(&i);
                self.audio_data.remove(&i);
                self.mixed = None;
                self.load_media(i, ctx);
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn pump(&mut self, ctx: &egui::Context) {
        self.ctx_clone = Some(ctx.clone());
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Probed(_, Ok(src)) => {
                    let idx = self.proj.sources.len();
                    self.proj.sources.push(src);
                    self.add_source(idx);
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
        if self.mixed.as_ref() == Some(&self.proj) || self.drag.is_some() {
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
        if !self.playing {
            self.shuttle = 1.0;
        }
        if self.playing && self.shuttle > 0.0 && self.t >= self.proj.duration() - 0.01 {
            self.t = self.mark_in.unwrap_or(0.0);
        }
        self.sync_audio();
    }

    fn sync_audio(&mut self) {
        if let Ok(mut s) = self.audio.state.lock() {
            s.pos = self.t * OUT_RATE as f64;
            s.playing = self.playing && (self.shuttle - 1.0).abs() < 1e-6;
        }
    }

    fn shuttle(&mut self, dir: f64) {
        let next = if self.playing && self.shuttle.signum() == dir { (self.shuttle.abs() * 2.0).min(8.0) * dir } else { dir };
        self.shuttle = next;
        self.playing = self.proj.duration() > 0.0;
        self.sync_audio();
    }

    fn seek(&mut self, t: f64) {
        self.t = t.clamp(0.0, self.proj.duration().max(0.0));
        if let Ok(mut s) = self.audio.state.lock() {
            s.pos = self.t * OUT_RATE as f64;
        }
    }

    fn preview_size(&self) -> (u32, u32) {
        let base = [960.0, 640.0, 400.0][self.quality.min(2)];
        let k = (base / self.proj.width.max(self.proj.height) as f32).min(1.0);
        (((self.proj.width as f32 * k) as u32).max(2), ((self.proj.height as f32 * k) as u32).max(2))
    }

    fn update_frame(&mut self, ctx: &egui::Context) {
        let (pw, ph) = self.preview_size();
        let fi = (self.t * self.proj.fps as f64).floor() as i64;
        if let Some((f, p, w, h)) = &self.composed
            && *f == fi
            && *w == pw
            && *h == ph
            && *p == self.proj
        {
            return;
        }
        match render::render(&self.proj, self.t, pw, ph, &mut self.pics) {
            Ok(px) => {
                self.set_tex(ctx, pw, ph, &px);
                self.shown = true;
            }
            Err(e) => self.error = Some(e),
        }
        self.composed = Some((fi, self.proj.clone(), pw, ph));
    }

    fn set_tex(&mut self, ctx: &egui::Context, w: u32, h: u32, rgba: &[u8]) {
        let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba);
        match &mut self.tex {
            Some(t) => t.set(img, TextureOptions::LINEAR),
            None => self.tex = Some(ctx.load_texture("frame", img, TextureOptions::LINEAR)),
        }
    }

    fn out_size(&self, o: &ExportOpts) -> (u32, u32) {
        let (pw, ph) = (self.proj.width, self.proj.height);
        let even = |v: u32| (v / 2 * 2).max(2);
        match (o.w, o.h) {
            (0, 0) => (even(pw), even(ph)),
            (w, 0) => (even(w), even((w as f64 * ph as f64 / pw.max(1) as f64) as u32)),
            (w, h) => (even(w), even(h)),
        }
    }

    fn export_range(&self, range: usize) -> (f64, f64) {
        let (a, b) = match (range, self.sel) {
            (1, _) => (self.mark_in.unwrap_or(0.0), self.mark_out.unwrap_or(self.proj.duration())),
            (2, Some(Sel::Clip(i))) if i < self.proj.clips.len() => {
                let s = self.proj.starts()[i];
                (s, s + self.proj.clips[i].len())
            }
            (2, Some(Sel::Over(i))) if i < self.proj.overlays.len() => (self.proj.overlays[i].start, self.proj.overlays[i].start + self.proj.overlays[i].len()),
            _ => (0.0, self.proj.duration()),
        };
        (a.min(b), b.max(a))
    }

    fn run_export(&mut self, opts: &ExportOpts) {
        if self.proj.duration() <= 0.0 {
            self.error = Some("La timeline est vide".into());
            return;
        }
        let (a, b) = self.export_range(opts.range);
        let stem = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("montage".into());
        let proj = self.proj.clone();
        let data = self.audio_data.clone();
        let (w, h) = self.out_size(opts);
        let fill = opts.fill;
        let done = Arc::new(AtomicUsize::new(0));
        let error = Arc::new(Mutex::new(None));
        let cancel = Arc::new(AtomicBool::new(false));
        let (d2, e2, c2) = (done.clone(), error.clone(), cancel.clone());
        let fps = if opts.fps == 0 { proj.fps } else { opts.fps };
        let total = ((b - a) * fps as f64).ceil().max(1.0) as usize;
        let finish = move |r: Result<(), String>| {
            if let Err(e) = r
                && let Ok(mut slot) = e2.lock()
            {
                *slot = Some(e);
            }
            d2.store(usize::MAX, Ordering::Relaxed);
        };
        let d3 = done.clone();
        let path = match opts.kind {
            1 => {
                let Some(path) = rfd::FileDialog::new().add_filter("Audio WAV", &["wav"]).set_file_name(format!("{stem}.wav")).save_file() else { return };
                let p2 = path.clone();
                std::thread::spawn(move || {
                    let mix = proj.mix(&|i| data.get(&i).cloned(), OUT_RATE);
                    let s = ((a * OUT_RATE as f64) as usize * 2).min(mix.len());
                    let e = ((b * OUT_RATE as f64) as usize * 2).min(mix.len());
                    finish(write_wav(&p2, &mix[s..e]));
                });
                path
            }
            4 | 5 => {
                let ext = if opts.kind == 4 { "png" } else { "jpg" };
                let Some(path) = rfd::FileDialog::new().add_filter(if opts.kind == 4 { "Image PNG" } else { "Image JPEG" }, &[ext]).set_file_name(format!("{stem}.{ext}")).save_file() else { return };
                let mut pics = render::Pics::default();
                let r = render::render_fit(&proj, self.t, w, h, fill, &mut pics).and_then(|px| save_image(&path, &px, w, h));
                finish(r);
                path
            }
            3 => {
                let Some(dir) = rfd::FileDialog::new().set_title("Dossier de la séquence d'images").pick_folder() else { return };
                let p2 = dir.clone();
                std::thread::spawn(move || {
                    let mut pics = render::Pics::default();
                    let r = (|| -> Result<(), String> {
                        media::startup();
                        for n in 0..total {
                            if c2.load(Ordering::Relaxed) {
                                return Err("export annulé".into());
                            }
                            let px = render::render_fit(&proj, a + n as f64 / fps as f64, w, h, fill, &mut pics)?;
                            save_image(&p2.join(format!("{stem}_{:05}.png", n + 1)), &px, w, h)?;
                            d3.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(())
                    })();
                    finish(r);
                });
                dir
            }
            2 => {
                let Some(path) = rfd::FileDialog::new().add_filter("GIF animé", &["gif"]).set_file_name(format!("{stem}.gif")).save_file() else { return };
                let p2 = path.clone();
                std::thread::spawn(move || {
                    let mut pics = render::Pics::default();
                    let r = (|| -> Result<(), String> {
                        media::startup();
                        let file = std::fs::File::create(&p2).map_err(|e| e.to_string())?;
                        let mut enc = image::codecs::gif::GifEncoder::new_with_speed(std::io::BufWriter::new(file), 10);
                        enc.set_repeat(image::codecs::gif::Repeat::Infinite).map_err(|e| e.to_string())?;
                        for n in 0..total {
                            if c2.load(Ordering::Relaxed) {
                                return Err("export annulé".into());
                            }
                            let px = render::render_fit(&proj, a + n as f64 / fps as f64, w, h, fill, &mut pics)?;
                            let img = image::RgbaImage::from_raw(w, h, px).ok_or("image")?;
                            enc.encode_frame(image::Frame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(1000, fps))).map_err(|e| e.to_string())?;
                            d3.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(())
                    })();
                    finish(r);
                });
                path
            }
            _ => {
                let Some(path) = rfd::FileDialog::new().add_filter("Vidéo MP4", &["mp4"]).set_file_name(format!("{stem}.mp4")).save_file() else { return };
                let bitrate = (opts.mbps.max(0.5) * 1_000_000.0) as u32;
                let abr = ABR[opts.abr.min(3)].1;
                let p2 = path.clone();
                std::thread::spawn(move || {
                    let r = (|| -> Result<(), String> {
                        media::startup();
                        let mix = proj.mix(&|i| data.get(&i).cloned(), OUT_RATE);
                        let mut enc = media::Encoder::with_audio(&p2, w, h, fps, bitrate, abr)?;
                        let mut pics = render::Pics::default();
                        for n in 0..total {
                            if c2.load(Ordering::Relaxed) {
                                return Err("export annulé".into());
                            }
                            let t = a + n as f64 / fps as f64;
                            enc.video(&render::render_fit(&proj, t, w, h, fill, &mut pics)?)?;
                            let s = (t * OUT_RATE as f64).round() as usize;
                            let e = ((t + 1.0 / fps as f64) * OUT_RATE as f64).round() as usize;
                            let mut chunk = mix[(s * 2).min(mix.len())..(e * 2).min(mix.len())].to_vec();
                            chunk.resize((e - s) * 2, 0.0);
                            enc.audio(&chunk)?;
                            d3.fetch_add(1, Ordering::Relaxed);
                        }
                        enc.finish()
                    })();
                    finish(r);
                });
                path
            }
        };
        self.job = Some(ExportJob { done, total, error, cancel, path, open_after: opts.open_after, seen: false });
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
        match std::fs::read(p).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Project>(b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&b)).map_err(|e| e.to_string())) {
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

    fn clip_ref(&self, s: Sel) -> Option<&Clip> {
        match s {
            Sel::Clip(i) => self.proj.clips.get(i),
            Sel::Over(i) => self.proj.overlays.get(i),
            _ => None,
        }
    }

    fn clip_mut(&mut self, s: Sel) -> Option<&mut Clip> {
        match s {
            Sel::Clip(i) => self.proj.clips.get_mut(i),
            Sel::Over(i) => self.proj.overlays.get_mut(i),
            _ => None,
        }
    }

    fn is_sel(&self, s: Sel) -> bool {
        self.sel == Some(s) || self.extra.contains(&s)
    }

    fn select_one(&mut self, s: Sel) {
        self.sel = Some(s);
        self.extra.clear();
    }

    fn all_sel(&self) -> Vec<Sel> {
        self.sel.into_iter().chain(self.extra.iter().copied()).collect()
    }

    fn click_select(&mut self, s: Sel, toggle: bool, range: bool) {
        match self.tool {
            Tool::TrackFwd | Tool::TrackBack if matches!(s, Sel::Clip(_)) => {
                let Sel::Clip(i) = s else { return };
                let starts = self.proj.starts();
                let (t0, fwd) = (starts[i], self.tool == Tool::TrackFwd);
                let end_i = starts[i] + self.proj.clips[i].len();
                let mut v: Vec<Sel> = if fwd { (i..self.proj.clips.len()).map(Sel::Clip).collect() } else { (0..=i).map(Sel::Clip).collect() };
                let keep = |a: f64, b: f64| if fwd { a >= t0 - 1e-6 } else { b <= end_i + 1e-6 };
                v.extend(self.proj.overlays.iter().enumerate().filter(|(_, o)| keep(o.start, o.start + o.len())).map(|(k, _)| Sel::Over(k)));
                v.extend(self.proj.titles.iter().enumerate().filter(|(_, t)| keep(t.start, t.start + t.dur)).map(|(k, _)| Sel::Title(k)));
                v.extend(self.proj.music.iter().enumerate().filter(|(_, m)| keep(m.start, m.end())).map(|(k, _)| Sel::Music(k)));
                self.sel = Some(s);
                self.extra = v.into_iter().filter(|x| *x != s).collect();
            }
            _ if toggle => {
                if self.sel == Some(s) {
                    self.sel = self.extra.pop();
                } else if let Some(k) = self.extra.iter().position(|x| *x == s) {
                    self.extra.remove(k);
                } else {
                    if let Some(old) = self.sel {
                        self.extra.push(old);
                    }
                    self.sel = Some(s);
                }
            }
            _ if range => {
                if let (Some(Sel::Clip(a)), Sel::Clip(b)) = (self.sel, s) {
                    self.extra = (a.min(b)..=a.max(b)).map(Sel::Clip).filter(|x| *x != s).collect();
                    self.sel = Some(s);
                } else {
                    self.select_one(s);
                }
            }
            _ => self.select_one(s),
        }
    }

    fn select_all(&mut self) {
        let mut v: Vec<Sel> = (0..self.proj.clips.len()).map(Sel::Clip).collect();
        v.extend((0..self.proj.overlays.len()).map(Sel::Over));
        v.extend((0..self.proj.titles.len()).map(Sel::Title));
        v.extend((0..self.proj.music.len()).map(Sel::Music));
        self.sel = v.first().copied();
        self.extra = v.into_iter().skip(1).collect();
    }

    fn delete_sel(&mut self) {
        self.remove_sel(0);
    }

    fn remove_sel(&mut self, how: u8) {
        let mut all = self.all_sel();
        all.sort_by_key(|s| std::cmp::Reverse(match s {
            Sel::Clip(i) | Sel::Over(i) | Sel::Music(i) | Sel::Title(i) | Sel::Gap(i) => *i,
        }));
        for s in all {
            match s {
                Sel::Clip(i) if i < self.proj.clips.len() => match how {
                    1 => self.proj.lift(i),
                    _ => self.proj.ripple_delete(i),
                },
                Sel::Over(i) if i < self.proj.overlays.len() => {
                    self.proj.overlays.remove(i);
                }
                Sel::Music(i) if i < self.proj.music.len() => {
                    self.proj.music.remove(i);
                }
                Sel::Title(i) if i < self.proj.titles.len() => {
                    self.proj.titles.remove(i);
                }
                Sel::Gap(i) if i < self.proj.clips.len() => self.proj.clips[i].gap = 0.0,
                _ => {}
            }
        }
        self.sel = None;
        self.extra.clear();
    }

    fn duplicate(&mut self) {
        match self.sel {
            Some(Sel::Clip(i)) if i < self.proj.clips.len() => {
                let c = Clip { gap: 0.0, ..self.proj.clips[i].clone() };
                self.proj.clips.insert(i + 1, c);
                self.select_one(Sel::Clip(i + 1));
            }
            Some(Sel::Over(i)) if i < self.proj.overlays.len() => {
                let mut c = self.proj.overlays[i].clone();
                c.start += c.len();
                self.proj.overlays.push(c);
                self.select_one(Sel::Over(self.proj.overlays.len() - 1));
            }
            Some(Sel::Title(i)) if i < self.proj.titles.len() => {
                let mut t = self.proj.titles[i].clone();
                t.start += t.dur;
                self.proj.titles.push(t);
                self.select_one(Sel::Title(self.proj.titles.len() - 1));
            }
            Some(Sel::Music(i)) if i < self.proj.music.len() => {
                let mut m = self.proj.music[i].clone();
                m.start += m.out - m.inp;
                self.proj.music.push(m);
                self.select_one(Sel::Music(self.proj.music.len() - 1));
            }
            _ => {}
        }
    }

    fn copy_sel(&mut self) {
        if let Some(s) = self.sel {
            self.clip_board = self.clip_ref(s).cloned();
        }
    }

    fn paste_clip(&mut self) {
        let Some(c) = self.clip_board.clone() else { return };
        let c = Clip { gap: 0.0, ..c };
        let at = match self.proj.local_at(self.t) {
            Some((i, local)) if local < 0.05 => i,
            Some((i, _)) => {
                self.proj.split(self.t);
                i + 1
            }
            None => self.proj.clips.len(),
        };
        self.proj.clips.insert(at, c);
        self.select_one(Sel::Clip(at));
    }

    fn paste_grade(&mut self, targets: &[Sel]) {
        let Some(g) = self.grade_board.clone() else { return };
        for s in targets {
            if let Some(c) = self.clip_mut(*s) {
                (c.brightness, c.contrast, c.saturation, c.temperature, c.tint, c.look, c.look_amt) = (g.brightness, g.contrast, g.saturation, g.temperature, g.tint, g.look, g.look_amt);
                (c.exposure, c.highlights, c.shadows, c.whites, c.blacks, c.vibrance, c.hue, c.wheels, c.curves) = (g.exposure, g.highlights, g.shadows, g.whites, g.blacks, g.vibrance, g.hue, g.wheels, g.curves);
            }
        }
    }

    fn freeze_frame(&mut self) {
        let Some((i, local)) = self.proj.local_at(self.t) else { return };
        let mut still = self.proj.clips[i].clone();
        let st = still.src_at(local);
        (still.inp, still.out, still.hold, still.fade_in, still.fade_out, still.gap, still.trans, still.reverse) = (st, st + 0.04, 2.0, 0.0, 0.0, 0.0, 0, false);
        still.keys.clear();
        let at = if self.proj.split(self.t) { i + 1 } else if local < 0.05 { i } else { i + 1 };
        self.proj.clips.insert(at, still);
        self.select_one(Sel::Clip(at));
    }

    fn nudge_clip(&mut self, by: i32) {
        let Some(Sel::Clip(i)) = self.sel else { return };
        let to = i as i32 + by;
        if to >= 0 && (to as usize) < self.proj.clips.len() {
            self.proj.clips.swap(i, to as usize);
            self.select_one(Sel::Clip(to as usize));
        }
    }

    fn detach_audio(&mut self, s: Sel) {
        let start = match s {
            Sel::Clip(i) if i < self.proj.clips.len() => self.proj.starts()[i],
            Sel::Over(i) if i < self.proj.overlays.len() => self.proj.overlays[i].start,
            _ => return,
        };
        let Some(c) = self.clip_ref(s).cloned() else { return };
        if c.hold > 0.0 || !self.proj.sources[c.src].has_audio {
            self.error = Some("Ce clip n'a pas de son à détacher".into());
            return;
        }
        if (c.speed - 1.0).abs() > 1e-3 || c.reverse {
            self.error = Some("Détacher l'audio demande un clip à vitesse normale".into());
            return;
        }
        self.proj.music.push(Music { src: c.src, start, inp: c.inp, out: c.out, volume: c.volume, fade_in: c.fade_in, fade_out: c.fade_out, mute: false, pan: c.pan, label: c.label });
        if let Some(c) = self.clip_mut(s) {
            c.mute = true;
        }
        self.select_one(Sel::Music(self.proj.music.len() - 1));
    }

    fn normalize(&mut self, s: Sel, db: f32) {
        let (src, inp, out) = match s {
            Sel::Music(i) => {
                let m = &self.proj.music[i];
                (m.src, m.inp, m.out)
            }
            _ => match self.clip_ref(s) {
                Some(c) => (c.src, c.inp, c.out),
                None => return,
            },
        };
        let Some(data) = self.audio_data.get(&src) else {
            self.error = Some("Le son de ce média n'est pas encore chargé".into());
            return;
        };
        let a = ((inp * OUT_RATE as f64) as usize * 2).min(data.len());
        let b = ((out * OUT_RATE as f64) as usize * 2).min(data.len());
        let peak = data[a..b].iter().fold(0f32, |m, v| m.max(v.abs()));
        if peak < 1e-4 {
            self.error = Some("Ce passage est silencieux".into());
            return;
        }
        let g = (10f32.powf(db / 20.0) / peak).min(16.0);
        match s {
            Sel::Music(i) => self.proj.music[i].volume = g,
            _ => {
                if let Some(c) = self.clip_mut(s) {
                    c.volume = g;
                    c.keys.retain(|k| k.prop != 9);
                }
            }
        }
    }

    fn move_track(&mut self, s: Sel) {
        match s {
            Sel::Clip(i) if i < self.proj.clips.len() => {
                let start = self.proj.starts()[i];
                let c = self.proj.clips[i].clone();
                self.proj.lift(i);
                self.proj.overlays.push(Clip { start, gap: 0.0, trans: 0, ..c });
                self.select_one(Sel::Over(self.proj.overlays.len() - 1));
            }
            Sel::Over(i) if i < self.proj.overlays.len() => {
                let o = self.proj.overlays.remove(i);
                let t = o.start;
                let at = match self.proj.local_at(t) {
                    Some((k, l)) if l < 0.05 => k,
                    Some((k, _)) => {
                        self.proj.split(t);
                        k + 1
                    }
                    None => {
                        let starts = self.proj.starts();
                        starts.iter().position(|s| *s > t).unwrap_or(self.proj.clips.len())
                    }
                };
                let len = o.len();
                let gap = if at == self.proj.clips.len() { (t - self.proj.v1_end()).max(0.0) } else { 0.0 };
                self.proj.shift_after(t, len);
                self.proj.clips.insert(at, Clip { start: 0.0, gap, ..o });
                self.select_one(Sel::Clip(at));
            }
            _ => {}
        }
    }

    fn add_marker(&mut self) {
        if let Some(k) = self.proj.markers.iter().position(|m| (m.t - self.t).abs() < 0.05) {
            self.marker_edit = Some(k);
        } else {
            let color = self.proj.markers.last().map_or(0, |m| m.color);
            self.proj.markers.push(Marker { t: self.t, color, ..Default::default() });
            self.proj.markers.sort_by(|a, b| a.t.total_cmp(&b.t));
        }
    }

    fn remove_marker(&mut self) {
        self.proj.markers.retain(|m| (m.t - self.t).abs() >= 0.05);
    }

    fn edit_points(&self) -> Vec<f64> {
        let mut stops = self.proj.starts();
        stops.extend(self.proj.clips.iter().zip(self.proj.starts()).map(|(c, s)| s + c.len()));
        stops.push(self.proj.duration());
        stops.extend(self.proj.markers.iter().map(|m| m.t));
        stops.extend(self.proj.titles.iter().flat_map(|t| [t.start, t.start + t.dur]));
        stops.extend(self.proj.music.iter().flat_map(|m| [m.start, m.end()]));
        stops.extend(self.proj.overlays.iter().flat_map(|o| [o.start, o.start + o.len()]));
        stops.extend(self.mark_in.iter().chain(self.mark_out.iter()).copied());
        stops
    }

    fn jump(&mut self, dir: i32) {
        let t = self.t;
        let stops = self.edit_points();
        let target = if dir > 0 { stops.into_iter().filter(|s| *s > t + 1e-3).fold(f64::INFINITY, f64::min) } else { stops.into_iter().filter(|s| *s < t - 1e-3).fold(f64::NEG_INFINITY, f64::max) };
        if target.is_finite() {
            self.seek(target);
        }
    }

    fn jump_marker(&mut self, dir: i32) {
        let t = self.t;
        let ms = self.proj.markers.iter().map(|m| m.t);
        let target = if dir > 0 { ms.filter(|s| *s > t + 1e-3).fold(f64::INFINITY, f64::min) } else { ms.filter(|s| *s < t - 1e-3).fold(f64::NEG_INFINITY, f64::max) };
        if target.is_finite() {
            self.seek(target);
        }
    }

    fn snap_time(&self, t: f64, tol: f64) -> f64 {
        if !self.snap {
            return t;
        }
        let mut best = t;
        let mut dist = tol;
        let points = match &self.drag {
            Some(d) => {
                let mut me = Vec::new();
                let p = &d.orig;
                me.extend(p.starts());
                me.extend(p.clips.iter().zip(p.starts()).map(|(c, s)| s + c.len()));
                me.extend(p.markers.iter().map(|m| m.t));
                me.extend(p.titles.iter().flat_map(|t| [t.start, t.start + t.dur]));
                me.extend(p.music.iter().flat_map(|m| [m.start, m.end()]));
                me
            }
            None => self.edit_points(),
        };
        for s in points.into_iter().chain([0.0, self.t]) {
            if (s - t).abs() < dist {
                dist = (s - t).abs();
                best = s;
            }
        }
        best
    }

    fn add_title_at(&mut self, dx: f32, dy: f32, vertical: bool) {
        self.proj.titles.push(Title { start: self.t, dx, dy, vertical, ..Title::default() });
        self.select_one(Sel::Title(self.proj.titles.len() - 1));
        self.tab = 0;
    }

    fn set_tool(&mut self, t: Tool) {
        self.tool = t;
        if !matches!(t, Tool::Pen) {
            self.pen.clear();
        }
    }

    fn press_group(&mut self, g: usize) {
        let tools = GROUPS[g].1;
        let next = match tools.iter().position(|x| x.0 == self.tool) {
            Some(k) => (k + 1) % tools.len(),
            None => self.group_last[g].min(tools.len() - 1),
        };
        self.group_last[g] = next;
        self.set_tool(tools[next].0);
    }

    fn history_to(&mut self, k: usize) {
        while self.undo.len() > k {
            if let Some(p) = self.undo.pop() {
                self.redo.push(std::mem::replace(&mut self.proj, p));
            }
        }
    }

    fn undo_step(&mut self) {
        if let Some(p) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.proj, p));
        }
    }

    fn redo_step(&mut self) {
        if let Some(p) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.proj, p));
        }
    }

    fn menu_action(&mut self, s: Sel, act: u8) {
        self.select_one(s);
        match act {
            0 => self.delete_sel(),
            1 => self.duplicate(),
            2 => {
                self.proj.split(self.t);
            }
            3 => self.copy_sel(),
            4 => match s {
                Sel::Title(i) => self.proj.titles[i].start = self.t,
                Sel::Music(i) => self.proj.music[i].start = self.t,
                _ => {}
            },
            5 => match s {
                Sel::Music(i) => self.proj.music[i].mute = !self.proj.music[i].mute,
                _ => {
                    if let Some(c) = self.clip_mut(s) {
                        c.mute = !c.mute;
                    }
                }
            },
            6 => self.normalize(s, -1.0),
            7 => self.freeze_frame(),
            8 => self.speed_sheet = true,
            9 => {
                if let Some(c) = self.clip_mut(s) {
                    c.reverse = !c.reverse;
                    c.keys.clear();
                }
            }
            10 => {
                if let Some(c) = self.clip_mut(s) {
                    c.disabled = !c.disabled;
                }
            }
            11 => self.detach_audio(s),
            12 => self.move_track(s),
            13 => {
                if let Some(c) = self.clip_mut(s) {
                    c.reset_look();
                }
            }
            14 => self.remove_sel(1),
            15 => self.remove_sel(0),
            100..=106 => {
                if let Some(c) = self.clip_mut(s) {
                    c.speed = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 4.0][(act - 100) as usize];
                }
            }
            120..=145 => {
                if let Some(c) = self.clip_mut(s) {
                    c.trans = act - 120;
                }
            }
            160..=179 => {
                if let Some(c) = self.clip_mut(s) {
                    c.look = act - 160;
                }
            }
            190..=199 => {
                if let Some(c) = self.clip_mut(s) {
                    inspect::effect_preset(c, (act - 190) as usize);
                }
            }
            210..=217 => {
                if let Some(c) = self.clip_mut(s) {
                    c.label = act - 210;
                }
            }
            _ => {}
        }
    }

    fn apply_default_transition(&mut self) {
        let targets: Vec<Sel> = self.all_sel();
        for s in targets {
            if let Sel::Clip(i) = s
                && i > 0
                && let Some(c) = self.proj.clips.get_mut(i)
            {
                c.trans = 1;
                c.trans_dur = 1.0;
            }
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() || self.home {
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        if sc(M::COMMAND | M::SHIFT, Key::Z) || sc(M::COMMAND, Key::Y) {
            self.redo_step();
        } else if sc(M::COMMAND, Key::Z) {
            self.undo_step();
        }
        if sc(M::COMMAND | M::SHIFT, Key::S) {
            self.save(true);
        } else if sc(M::COMMAND, Key::S) {
            self.save(false);
        }
        if sc(M::COMMAND, Key::I) {
            self.import_dialog(ctx);
        }
        if sc(M::COMMAND | M::SHIFT, Key::E) {
            let mut o = ExportOpts::from_preset(14);
            o.range = 0;
            self.run_export(&o);
        }
        if sc(M::COMMAND, Key::M) {
            self.export = Some(ExportOpts::from_preset(0));
        }
        if sc(M::COMMAND | M::SHIFT, Key::F) {
            self.freeze_frame();
        }
        if sc(M::COMMAND | M::SHIFT, Key::D) {
            self.apply_default_transition();
        }
        if sc(M::COMMAND, Key::D) {
            self.duplicate();
        }
        if sc(M::COMMAND | M::SHIFT, Key::K) {
            self.proj.split_all(self.t);
        } else if sc(M::COMMAND, Key::K) {
            self.proj.split(self.t);
        }
        if sc(M::COMMAND, Key::C) {
            self.copy_sel();
        }
        if sc(M::COMMAND, Key::X) {
            self.copy_sel();
            self.delete_sel();
        }
        if sc(M::COMMAND, Key::V) {
            self.paste_clip();
        }
        if sc(M::COMMAND | M::SHIFT, Key::A) {
            self.sel = None;
            self.extra.clear();
        } else if sc(M::COMMAND, Key::A) {
            self.select_all();
        }
        if sc(M::COMMAND, Key::T) {
            self.add_title_at(0.0, 0.0, false);
        }
        if sc(M::COMMAND, Key::R) && matches!(self.sel, Some(Sel::Clip(_) | Sel::Over(_))) {
            self.speed_sheet = true;
        }
        if sc(M::COMMAND, Key::L)
            && let Some(s) = self.sel
        {
            self.detach_audio(s);
        }
        if sc(M::ALT, Key::ArrowLeft) {
            self.nudge_clip(-1);
        }
        if sc(M::ALT, Key::ArrowRight) {
            self.nudge_clip(1);
        }
        if sc(M::ALT, Key::X) {
            self.mark_in = None;
            self.mark_out = None;
        }
        if sc(M::ALT, Key::Delete) || sc(M::ALT, Key::Backspace) {
            self.remove_sel(1);
        }
        if sc(M::SHIFT, Key::Delete) || sc(M::SHIFT, Key::Backspace) {
            self.remove_sel(0);
        }
        if sc(M::SHIFT, Key::A) {
            self.set_tool(Tool::TrackBack);
        }
        if sc(M::SHIFT, Key::C) {
            self.set_tool(Tool::BladeAll);
        }
        if sc(M::SHIFT, Key::S) {
            self.snap = !self.snap;
        }
        if sc(M::SHIFT, Key::M) {
            self.remove_marker();
        }
        if sc(M::SHIFT, Key::I)
            && let Some(t) = self.mark_in
        {
            self.seek(t);
        }
        if sc(M::SHIFT, Key::O)
            && let Some(t) = self.mark_out
        {
            self.seek(t);
        }
        if sc(M::SHIFT, Key::E) {
            for s in self.all_sel() {
                if let Some(c) = self.clip_mut(s) {
                    c.disabled = !c.disabled;
                }
            }
        }
        if sc(M::SHIFT, Key::ArrowUp) {
            self.jump_marker(-1);
        }
        if sc(M::SHIFT, Key::ArrowDown) {
            self.jump_marker(1);
        }
        let fps = self.proj.fps as f64;
        let k = ctx.input(|i| {
            let plain = !i.modifiers.command && !i.modifiers.alt && !i.modifiers.shift;
            let p = |k: Key| i.key_pressed(k) && plain;
            (
                i.key_pressed(Key::Space),
                p(Key::S),
                (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) && plain,
                i.key_pressed(Key::ArrowLeft) && !i.modifiers.alt,
                i.key_pressed(Key::ArrowRight) && !i.modifiers.alt,
                i.key_pressed(Key::Home),
                i.key_pressed(Key::End),
                i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals),
                i.key_pressed(Key::Minus),
                i.modifiers.shift,
                GROUPS.map(|g| p(g.0)),
                [p(Key::A), p(Key::N), p(Key::R), p(Key::U), p(Key::Z)],
                [p(Key::J), p(Key::K), p(Key::L), p(Key::M), p(Key::I), p(Key::O), p(Key::F), p(Key::Backslash), p(Key::Enter), p(Key::Escape)],
                if p(Key::Q) { 1 } else if p(Key::W) { 2 } else { 0 },
                if i.key_pressed(Key::ArrowUp) && plain { -1 } else if i.key_pressed(Key::ArrowDown) && plain { 1 } else { 0 },
            )
        });
        let (space, s, del, left, right, home, end, plus, minus, shift, groups, direct, keys, trim, jump) = k;
        let step = if shift { 1.0 } else { 1.0 / fps };
        if space {
            self.shuttle = 1.0;
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
        if plus {
            self.pps = (self.pps * 1.25).min(600.0);
        }
        if minus {
            self.pps = (self.pps / 1.25).max(2.0);
        }
        for (g, pressed) in groups.iter().enumerate() {
            if *pressed {
                self.press_group(g);
            }
        }
        for (pressed, tool) in direct.iter().zip([Tool::TrackFwd, Tool::Rolling, Tool::Rate, Tool::Slide, Tool::Zoom]) {
            if *pressed {
                self.set_tool(tool);
            }
        }
        let [j, kk, l, m, i, o, f, fit, enter, esc] = keys;
        if j {
            self.shuttle(-1.0);
        }
        if kk {
            self.set_playing(false);
        }
        if l {
            self.shuttle(1.0);
        }
        if m {
            self.add_marker();
        }
        if i {
            self.mark_in = Some(self.t);
        }
        if o {
            self.mark_out = Some(self.t);
        }
        if f {
            self.focus = !self.focus;
        }
        if fit {
            self.pps = 0.0;
        }
        if enter {
            self.close_pen();
        }
        if esc {
            self.pen.clear();
        }
        if trim > 0 {
            self.proj.trim_to(self.t, trim == 1);
        }
        if jump != 0 {
            self.jump(jump);
        }
        let valid = |s: &Sel, p: &Project| match *s {
            Sel::Clip(i) | Sel::Gap(i) => i < p.clips.len(),
            Sel::Over(i) => i < p.overlays.len(),
            Sel::Music(i) => i < p.music.len(),
            Sel::Title(i) => i < p.titles.len(),
        };
        if self.sel.is_some_and(|s| !valid(&s, &self.proj)) {
            self.sel = None;
        }
        let p = self.proj.clone();
        self.extra.retain(|s| valid(s, &p));
    }

    fn mask_target(&self) -> Option<(Sel, f64)> {
        match self.sel {
            Some(Sel::Clip(i)) if self.proj.local_at(self.t).is_some_and(|(k, _)| k == i) => return self.proj.local_at(self.t).map(|(_, l)| (Sel::Clip(i), l)),
            Some(Sel::Over(i)) => {
                if let Some((_, l)) = self.proj.overlays_at(self.t).into_iter().find(|(k, _)| *k == i) {
                    return Some((Sel::Over(i), l));
                }
            }
            _ => {}
        }
        self.proj.local_at(self.t).map(|(i, l)| (Sel::Clip(i), l))
    }

    fn close_pen(&mut self) {
        if self.pen.len() >= 3
            && let Some((s, _)) = self.mask_target()
        {
            let pts = std::mem::take(&mut self.pen);
            if let Some(c) = self.clip_mut(s) {
                c.mask = Some(model::Mask { kind: 2, pts, feather: 4.0, invert: false });
            }
            self.select_one(s);
            self.tab = 2;
        }
        self.pen.clear();
    }
}

fn save_image(path: &Path, px: &[u8], w: u32, h: u32) -> Result<(), String> {
    let jpeg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg"));
    if jpeg {
        let rgb: Vec<u8> = px.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        image::save_buffer(path, &rgb, w, h, image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())
    } else {
        image::save_buffer(path, px, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())
    }
}

fn write_wav(path: &Path, interleaved: &[f32]) -> Result<(), String> {
    let mut b = Vec::with_capacity(44 + interleaved.len() * 2);
    let data_len = (interleaved.len() * 2) as u32;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&OUT_RATE.to_le_bytes());
    b.extend_from_slice(&(OUT_RATE * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for v in interleaved {
        b.extend_from_slice(&((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
    std::fs::write(path, b).map_err(|e| e.to_string())
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

fn chapters(p: &Project) -> String {
    let mut lines = Vec::new();
    if p.markers.first().is_none_or(|m| m.t > 0.5) {
        lines.push("0:00 Introduction".to_string());
    }
    for (k, m) in p.markers.iter().enumerate() {
        let s = m.t.max(0.0) as u64;
        let stamp = if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) };
        let name = if m.name.is_empty() { format!("Chapitre {}", k + 1) } else { m.name.clone() };
        lines.push(format!("{stamp} {name}"));
    }
    lines.join("\n")
}

fn markers_csv(p: &Project) -> String {
    let mut out = String::from("Temps;Nom;Commentaire;Couleur;Durée\n");
    for m in &p.markers {
        out.push_str(&format!("{};{};{};{};{:.2}\n", tc(m.t, p.fps), m.name.replace(';', ","), m.note.replace(';', ",").replace('\n', " "), MARKER_COLORS[m.color as usize % MARKER_COLORS.len()].0, m.dur));
    }
    out
}

fn describe(a: &Project, b: &Project) -> String {
    let n = |x: usize, y: usize, what: &str| match x.cmp(&y) {
        std::cmp::Ordering::Less => Some(format!("{what} ajouté")),
        std::cmp::Ordering::Greater => Some(format!("{what} supprimé")),
        _ => None,
    };
    n(a.clips.len(), b.clips.len(), "Clip")
        .or_else(|| n(a.overlays.len(), b.overlays.len(), "Clip V2"))
        .or_else(|| n(a.titles.len(), b.titles.len(), "Titre"))
        .or_else(|| n(a.music.len(), b.music.len(), "Son"))
        .or_else(|| n(a.markers.len(), b.markers.len(), "Marqueur"))
        .or_else(|| n(a.sources.len(), b.sources.len(), "Média"))
        .unwrap_or_else(|| {
            if a.clips != b.clips || a.overlays != b.overlays {
                "Clip modifié".into()
            } else if a.titles != b.titles {
                "Titre modifié".into()
            } else if a.music != b.music {
                "Son modifié".into()
            } else if a.markers != b.markers {
                "Marqueur modifié".into()
            } else {
                "Séquence modifiée".into()
            }
        })
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
        if let Ok(mut s) = self.audio.state.lock() {
            s.volume = self.volume;
        }
        if self.playing {
            let manual = (self.shuttle - 1.0).abs() > 1e-6 || self.audio._stream.is_none();
            if manual {
                self.t += ctx.input(|i| i.stable_dt) as f64 * self.shuttle;
            } else {
                self.t = self.audio.state.lock().map(|s| s.pos).unwrap_or(0.0) / OUT_RATE as f64;
            }
            let end = if self.looping { self.mark_out.unwrap_or(self.proj.duration()) } else { self.proj.duration() };
            if self.t < 0.0 {
                self.t = 0.0;
                self.set_playing(false);
            } else if self.t >= end {
                if self.looping {
                    self.seek(self.mark_in.unwrap_or(0.0));
                } else {
                    self.t = self.proj.duration();
                    self.set_playing(false);
                }
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
        if let Some(j) = &mut self.job
            && !j.seen
            && j.done.load(Ordering::Relaxed) == usize::MAX
        {
            j.seen = true;
            if j.open_after && j.error.lock().map(|e| e.is_none()).unwrap_or(false) {
                let _ = std::process::Command::new("explorer").arg(format!("/select,{}", j.path.display())).spawn();
            }
        }

        self.top_bar(root, ctx, &name);
        self.status(root);
        if self.home {
            self.home_screen(root, ctx);
        } else {
            self.timeline(root);
            if !self.focus {
                self.bin(root, ctx);
                self.inspector(root);
            }
            self.viewer(root);
        }
        self.sheets(ctx);
        self.help.show(ctx);

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
    fn top_bar(&mut self, root: &mut Ui, ctx: &egui::Context, name: &str) {
        egui::Panel::top("menu").exact_size(44.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let full = ui.max_rect();
            let right_w = if self.home { 0.0 } else { 330.0 };
            let left = Rect::from_min_max(full.min, pos2(full.right() - right_w, full.bottom()));
            ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(22.0))));
                    ui.add_space(4.0);
                    egui::MenuBar::new().ui(ui, |ui| {
                        self.menus(ui, ctx);
                        if !self.home {
                            ui.add_space(12.0);
                            pro::toolbar_sep(ui);
                            ui.label(RichText::new(name).font(semibold(13.0)).color(INK));
                            if self.proj != self.saved {
                                ui.label(RichText::new("•").color(MUTED)).on_hover_text("Modifications non enregistrées");
                            }
                        }
                    });
                });
            });
            if self.home {
                return;
            }
            let right = Rect::from_min_max(pos2(full.right() - right_w, full.top()), full.max);
            ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let busy = self.job.as_ref().is_some_and(|j| j.done.load(Ordering::Relaxed) != usize::MAX);
                    if busy {
                        let j = self.job.as_ref().unwrap();
                        let d = j.done.load(Ordering::Relaxed);
                        if pro::secondary(ui, "Annuler").clicked() {
                            j.cancel.store(true, Ordering::Relaxed);
                        }
                        ui.add(egui::ProgressBar::new(d as f32 / j.total.max(1) as f32).desired_width(140.0).corner_radius(CornerRadius::same(6)).text(format!("Export {:.0} %", d as f32 * 100.0 / j.total.max(1) as f32)));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
                    } else if pro::primary_icon(ui, "Exporter", |p, r, c| glyph(p, r, G::Export, c)).on_hover_text("Ctrl+M").clicked() {
                        let mut o = ExportOpts::from_preset(0);
                        o.range = if self.mark_in.is_some() || self.mark_out.is_some() { 1 } else { 0 };
                        self.export = Some(o);
                    }
                    ui.add_space(6.0);
                    if pro::chip(ui, &format!("{} × {} · {} i/s", self.proj.width, self.proj.height, self.proj.fps)).on_hover_text("Réglages de la séquence").clicked() {
                        self.seq_sheet = true;
                    }
                });
            });
        });
    }

    fn menus(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let row = |ui: &mut Ui, on: bool, label: &str, key: &str| -> bool { ui.add_enabled(on, egui::Button::new(label).shortcut_text(key)).clicked() };
        ui.menu_button("Fichier", |ui| {
            if row(ui, true, "Nouveau projet", "") {
                self.home = true;
            }
            if row(ui, true, "Ouvrir un projet…", "") {
                self.open(ctx);
            }
            if row(ui, true, "Importer des médias…", "Ctrl+I") {
                self.import_dialog(ctx);
            }
            ui.add_enabled_ui(!self.home, |ui| {
                ui.menu_button("Nouvel élément", |ui| {
                    for (k, (_, name)) in GENERATORS.iter().enumerate() {
                        if ui.button(*name).clicked() {
                            self.add_generator(k);
                            ui.close();
                        }
                    }
                });
                ui.separator();
                if row(ui, true, "Enregistrer", "Ctrl+S") {
                    self.save(false);
                }
                if row(ui, true, "Enregistrer sous…", "Ctrl+Maj+S") {
                    self.save(true);
                }
                ui.separator();
                if row(ui, true, "Exporter…", "Ctrl+M") {
                    self.export = Some(ExportOpts::from_preset(0));
                }
                ui.menu_button("Exporter avec un préréglage", |ui| {
                    for (k, p) in EXPORT_PRESETS.iter().enumerate() {
                        if ui.button(p.0).clicked() {
                            self.export = Some(ExportOpts::from_preset(k));
                            ui.close();
                        }
                    }
                });
                if row(ui, true, "Exporter l'image affichée (PNG)…", "Ctrl+Maj+E") {
                    self.run_export(&ExportOpts::from_preset(14));
                }
                if row(ui, !self.proj.markers.is_empty(), "Exporter les marqueurs (CSV)…", "")
                    && let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("marqueurs.csv").save_file()
                    && let Err(e) = std::fs::write(&p, markers_csv(&self.proj))
                {
                    self.error = Some(e.to_string());
                }
                ui.separator();
                if row(ui, self.proj.sources.iter().enumerate().any(|(i, _)| self.uses(i) == 0), "Retirer les médias inutilisés", "") {
                    self.remove_unused();
                }
            });
        });
        ui.add_enabled_ui(!self.home, |ui| {
            ui.menu_button("Édition", |ui| {
                if row(ui, !self.undo.is_empty(), "Annuler", "Ctrl+Z") {
                    self.undo_step();
                }
                if row(ui, !self.redo.is_empty(), "Rétablir", "Ctrl+Y") {
                    self.redo_step();
                }
                ui.separator();
                let is_clip = matches!(self.sel, Some(Sel::Clip(_) | Sel::Over(_)));
                if row(ui, is_clip, "Couper", "Ctrl+X") {
                    self.copy_sel();
                    self.delete_sel();
                }
                if row(ui, is_clip, "Copier", "Ctrl+C") {
                    self.copy_sel();
                }
                if row(ui, self.clip_board.is_some(), "Coller à la tête de lecture", "Ctrl+V") {
                    self.paste_clip();
                }
                if row(ui, self.sel.is_some(), "Dupliquer", "Ctrl+D") {
                    self.duplicate();
                }
                if row(ui, self.sel.is_some(), "Effacer", "Suppr") {
                    self.delete_sel();
                }
                if row(ui, is_clip, "Supprimer et refermer", "Maj+Suppr") {
                    self.remove_sel(0);
                }
                if row(ui, is_clip, "Lever (laisser un vide)", "Alt+Suppr") {
                    self.remove_sel(1);
                }
                ui.separator();
                if row(ui, true, "Tout sélectionner", "Ctrl+A") {
                    self.select_all();
                }
                if row(ui, self.sel.is_some(), "Tout désélectionner", "Ctrl+Maj+A") {
                    self.sel = None;
                    self.extra.clear();
                }
                if let Some(Sel::Clip(i)) = self.sel
                    && row(ui, true, "Sélectionner la même étiquette", "")
                {
                    let l = self.proj.clips[i].label;
                    self.extra = self.proj.clips.iter().enumerate().filter(|(k, c)| c.label == l && *k != i).map(|(k, _)| Sel::Clip(k)).collect();
                }
                ui.separator();
                if row(ui, self.grade_board.is_some() && is_clip, "Coller l'étalonnage", "") {
                    let all = self.all_sel();
                    self.paste_grade(&all);
                }
            });
            ui.menu_button("Clip", |ui| {
                let s = self.sel;
                let is_clip = matches!(s, Some(Sel::Clip(_) | Sel::Over(_)));
                if row(ui, is_clip, "Vitesse/Durée…", "Ctrl+R") {
                    self.speed_sheet = true;
                }
                if row(ui, is_clip, "Activer / désactiver", "Maj+E")
                    && let Some(s) = s
                {
                    self.menu_action(s, 10);
                }
                if row(ui, is_clip, "Inverser la lecture", "")
                    && let Some(s) = s
                {
                    self.menu_action(s, 9);
                }
                if row(ui, is_clip, "Détacher l'audio", "Ctrl+L")
                    && let Some(s) = s
                {
                    self.detach_audio(s);
                }
                if row(ui, is_clip, "Changer de piste (V1 ↔ V2)", "")
                    && let Some(s) = s
                {
                    self.move_track(s);
                }
                if row(ui, matches!(s, Some(Sel::Clip(i)) if i > 0), "Appliquer la transition par défaut", "Ctrl+Maj+D") {
                    self.apply_default_transition();
                }
                if row(ui, is_clip || matches!(s, Some(Sel::Music(_))), "Normaliser l'audio (−1 dB)", "")
                    && let Some(s) = s
                {
                    self.normalize(s, -1.0);
                }
                if row(ui, is_clip, "Réinitialiser les effets", "")
                    && let Some(s) = s
                {
                    self.menu_action(s, 13);
                }
                ui.separator();
                if row(ui, is_clip, "Déplacer à gauche", "Alt+←") {
                    self.nudge_clip(-1);
                }
                if row(ui, is_clip, "Déplacer à droite", "Alt+→") {
                    self.nudge_clip(1);
                }
            });
            ui.menu_button("Séquence", |ui| {
                if row(ui, true, "Réglages de la séquence…", "") {
                    self.seq_sheet = true;
                }
                ui.separator();
                if row(ui, true, "Scinder à la tête de lecture", "S") {
                    self.proj.split(self.t);
                }
                if row(ui, true, "Scinder toutes les pistes", "Ctrl+Maj+K") {
                    self.proj.split_all(self.t);
                }
                if row(ui, true, "Rogner le début à la tête", "Q") {
                    self.proj.trim_to(self.t, true);
                }
                if row(ui, true, "Rogner la fin à la tête", "W") {
                    self.proj.trim_to(self.t, false);
                }
                if row(ui, true, "Figer l'image (2 s)", "Ctrl+Maj+F") {
                    self.freeze_frame();
                }
                if row(ui, self.proj.clips.iter().any(|c| c.gap > 0.0), "Supprimer tous les espaces vides", "") {
                    self.proj.close_gaps();
                }
                ui.separator();
                if row(ui, true, "Insérer un titre", "Ctrl+T") {
                    self.add_title_at(0.0, 0.0, false);
                }
                ui.menu_button("Titre prédéfini", |ui| {
                    for (k, n) in inspect::TITLE_PRESETS.iter().enumerate() {
                        if ui.button(*n).clicked() {
                            self.add_title_at(0.0, 0.0, false);
                            if let Some(Sel::Title(i)) = self.sel {
                                inspect::title_preset(&mut self.proj.titles[i], k);
                                if k == 4 {
                                    self.proj.titles[i].text = "Réalisation\nTon nom\n\nMontage\nCut — Ethone Studio\n\nMerci d'avoir regardé".into();
                                    self.proj.titles[i].dur = 8.0;
                                }
                            }
                            ui.close();
                        }
                    }
                });
                ui.separator();
                if row(ui, self.proj.solo != 0, "Désactiver tous les solos", "") {
                    self.proj.solo = 0;
                }
                if row(ui, true, "Déverrouiller toutes les pistes", "") {
                    self.proj.lock = [false; 4];
                }
                if row(ui, true, "Afficher toutes les pistes", "") {
                    self.proj.hide = [false; 3];
                }
            });
            ui.menu_button("Marqueurs", |ui| {
                if row(ui, true, "Ajouter / modifier un marqueur", "M") {
                    self.add_marker();
                }
                if row(ui, true, "Supprimer le marqueur", "Maj+M") {
                    self.remove_marker();
                }
                if row(ui, true, "Marqueur précédent", "Maj+↑") {
                    self.jump_marker(-1);
                }
                if row(ui, true, "Marqueur suivant", "Maj+↓") {
                    self.jump_marker(1);
                }
                ui.separator();
                if row(ui, true, "Point d'entrée", "I") {
                    self.mark_in = Some(self.t);
                }
                if row(ui, true, "Point de sortie", "O") {
                    self.mark_out = Some(self.t);
                }
                if row(ui, self.mark_in.is_some(), "Aller à l'entrée", "Maj+I")
                    && let Some(t) = self.mark_in
                {
                    self.seek(t);
                }
                if row(ui, self.mark_out.is_some(), "Aller à la sortie", "Maj+O")
                    && let Some(t) = self.mark_out
                {
                    self.seek(t);
                }
                if let Some(s @ (Sel::Clip(_) | Sel::Over(_))) = self.sel
                    && row(ui, true, "Entrée/sortie sur le clip", "")
                {
                    let (a, b) = {
                        let save = self.sel;
                        self.sel = Some(s);
                        let r = self.export_range(2);
                        self.sel = save;
                        r
                    };
                    (self.mark_in, self.mark_out) = (Some(a), Some(b));
                }
                if row(ui, true, "Effacer entrée et sortie", "Alt+X") {
                    self.mark_in = None;
                    self.mark_out = None;
                }
                ui.separator();
                if row(ui, true, "Point de montage précédent", "↑") {
                    self.jump(-1);
                }
                if row(ui, true, "Point de montage suivant", "↓") {
                    self.jump(1);
                }
                ui.separator();
                if row(ui, !self.proj.markers.is_empty(), "Copier comme chapitres YouTube", "") {
                    ui.ctx().copy_text(chapters(&self.proj));
                }
                if row(ui, !self.proj.markers.is_empty(), "Supprimer tous les marqueurs", "") {
                    self.proj.markers.clear();
                }
            });
            ui.menu_button("Outils", |ui| {
                for (g, (_, tools)) in GROUPS.iter().enumerate() {
                    for (k, (tool, name, key)) in tools.iter().enumerate() {
                        if ui.add(egui::Button::selectable(self.tool == *tool, *name).shortcut_text(*key)).clicked() {
                            self.group_last[g] = k;
                            self.set_tool(*tool);
                            ui.close();
                        }
                    }
                    if g + 1 < GROUPS.len() {
                        ui.separator();
                    }
                }
            });
            ui.menu_button("Affichage", |ui| {
                ui.checkbox(&mut self.snap, "Magnétisme (Maj+S)");
                ui.checkbox(&mut self.looping, "Lecture en boucle");
                ui.checkbox(&mut self.focus, "Mode visionneuse (F)");
                ui.separator();
                ui.checkbox(&mut self.safe, "Zones de sécurité");
                ui.checkbox(&mut self.grid, "Grille des tiers");
                ui.checkbox(&mut self.guides, "Repères vertical 9:16");
                ui.horizontal(|ui| {
                    ui.label("Qualité de lecture");
                    for (k, label) in ["Complète", "1/2", "1/4"].iter().enumerate() {
                        if ui.selectable_label(self.quality == k, *label).clicked() {
                            self.quality = k;
                        }
                    }
                });
                ui.separator();
                ui.checkbox(&mut self.show_thumbs, "Vignettes des clips");
                ui.checkbox(&mut self.show_waves, "Formes d'onde");
                ui.horizontal(|ui| {
                    ui.label("Hauteur des pistes");
                    for (label, k) in [("S", 0.75), ("M", 1.0), ("L", 1.4), ("XL", 1.8)] {
                        if ui.selectable_label((self.track_scale - k).abs() < 0.01, label).clicked() {
                            self.track_scale = k;
                        }
                    }
                });
                if row(ui, true, "Ajuster la timeline", "\\") {
                    self.pps = 0.0;
                }
                if row(ui, true, "Zoom avant", "+") {
                    self.pps = (self.pps * 1.25).min(600.0);
                }
                if row(ui, true, "Zoom arrière", "−") {
                    self.pps = (self.pps / 1.25).max(2.0);
                }
            });
        });
        self.help.menu(ui);
    }

    fn status(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                if !self.home {
                    let n = self.all_sel().len();
                    let sel = match self.sel {
                        _ if n > 1 => Some(format!("{n} éléments sélectionnés")),
                        Some(s @ (Sel::Clip(_) | Sel::Over(_))) => self.clip_ref(s).map(|c| format!("Clip · {} · {}", short(&self.proj.sources[c.src].name(), 26), tc(c.len(), self.proj.fps))),
                        Some(Sel::Music(i)) => self.proj.music.get(i).map(|m| format!("Musique · {}", short(&self.proj.sources[m.src].name(), 26))),
                        Some(Sel::Title(i)) => self.proj.titles.get(i).map(|t| format!("Titre · {}", short(t.text.lines().next().unwrap_or(""), 26))),
                        Some(Sel::Gap(i)) => self.proj.clips.get(i).map(|c| format!("Espace vide · {}", tc(c.gap, self.proj.fps))),
                        None => None,
                    };
                    ui.label(small(sel.unwrap_or(format!("{} clip(s) · {} titre(s) · {}", self.proj.clips.len() + self.proj.overlays.len(), self.proj.titles.len(), tc(self.proj.duration(), self.proj.fps)))));
                    if let (Some(a), Some(b)) = (self.mark_in, self.mark_out) {
                        ui.label(RichText::new(format!("Entrée/sortie {}", tc((b - a).abs(), self.proj.fps))).size(12.0).color(MARK));
                    }
                    if (self.shuttle - 1.0).abs() > 1e-6 && self.playing {
                        ui.label(RichText::new(format!("Navette {}{}×", if self.shuttle < 0.0 { "◀ " } else { "▶ " }, self.shuttle.abs())).size(12.0).color(BLUE));
                    }
                    if self.tool == Tool::Pen && !self.pen.is_empty() {
                        ui.label(RichText::new(format!("Plume : {} point(s) · Entrée pour fermer · Échap pour annuler", self.pen.len())).size(12.0).color(BLUE));
                    }
                }
                let mut open_dir = None;
                if let Some(j) = &self.job
                    && j.done.load(Ordering::Relaxed) == usize::MAX
                {
                    match j.error.lock().ok().and_then(|e| e.clone()) {
                        Some(e) => {
                            ui.label(RichText::new(format!("Export : {e}")).size(12.0).color(RED));
                        }
                        None => {
                            ui.label(RichText::new(format!("Exporté : {}", j.path.display())).size(12.0).color(GREEN));
                            if ui.small_button("Afficher").clicked() {
                                open_dir = Some(j.path.clone());
                            }
                        }
                    }
                }
                if let Some(p) = open_dir {
                    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", p.display())).spawn();
                }
                if let Some(e) = self.error.clone() {
                    ui.label(RichText::new(e).size(12.0).color(RED));
                    if ui.small_button("×").clicked() {
                        self.error = None;
                    }
                }
                if !self.home {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let tool = GROUPS.iter().flat_map(|g| g.1.iter()).find(|x| x.0 == self.tool).map_or("", |x| x.1);
                        ui.label(small(format!("{tool} · {}", if self.snap { "Magnétisme activé" } else { "Magnétisme désactivé" })));
                    });
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
            ui.add_space(20.0);
            if pro::ghost(ui, "?  Tutoriels").clicked() {
                self.help.learn("Importer des médias");
            }
        });
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin { left: 40, right: 40, top: 34, bottom: 20 })).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(40.0))));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Bienvenue dans Cut").font(semibold(28.0)).color(INK_STRONG));
                        ui.label(RichText::new("Monte, coupe, étalonne et exporte tes vidéos").color(MUTED));
                    });
                });
                ui.add_space(28.0);
                ui.label(RichText::new("Nouveau projet").font(semibold(16.0)).color(INK));
                ui.add_space(12.0);
                let mut chosen = None;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                    for (name, kind, w, h) in PRESETS.iter().take(8) {
                        if preset_card(ui, name, &format!("{w} × {h} · {kind}"), *w as f32, *h as f32).clicked() {
                            chosen = Some((*w, *h));
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
                    p.text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "Dépose des vidéos, des photos ou de la musique n'importe où", semibold(15.0), INK);
                    p.text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "MP4, MOV, MKV, PNG, JPG, MP3, WAV, M4A…", FontId::proportional(13.0), MUTED);
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
        egui::Panel::left("bin").resizable(false).exact_size(272.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 12))).show(root, |ui| {
            let mut gen_k = None;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Médias").font(semibold(13.5)).color(INK_STRONG));
                ui.label(RichText::new(self.proj.sources.len().to_string()).size(12.0).color(FAINT));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::secondary_icon(ui, "", |p, r, c| glyph(p, r, G::Plus, c)).on_hover_text("Importer des médias (Ctrl+I)").clicked() {
                        self.import_dialog(ctx);
                    }
                    ui.menu_button(RichText::new("✦").color(MUTED), |ui| {
                        ui.label(RichText::new("Nouvel élément").size(12.0).color(FAINT));
                        for (k, (_, name)) in GENERATORS.iter().enumerate() {
                            if ui.button(*name).clicked() {
                                gen_k = Some(k);
                                ui.close();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Nouvel élément : cache couleur, dégradé, mire…");
                });
            });
            if let Some(k) = gen_k {
                self.add_generator(k);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.bin_query).hint_text("Rechercher").desired_width(ui.available_width() - 84.0));
                pro::segmented(ui, &mut self.bin_list, &["▦", "☰"]);
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                pro::segmented(ui, &mut self.bin_filter, &["Tout", "Vidéo", "Son", "Image"]);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.menu_button(RichText::new("⇅").color(MUTED), |ui| {
                        ui.label(RichText::new("Trier par").size(12.0).color(FAINT));
                        for (k, n) in ["Ordre d'import", "Nom", "Durée", "Type", "Étiquette"].iter().enumerate() {
                            if ui.selectable_label(self.bin_sort == k, *n).clicked() {
                                self.bin_sort = k;
                                ui.close();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Trier les médias");
                });
            });
            ui.add_space(8.0);
            if self.proj.sources.is_empty() {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 120.0), Sense::hover());
                ui.painter().rect(r, CornerRadius::same(10), Color32::from_white_alpha(3), Stroke::new(1.0, LINE), StrokeKind::Inside);
                ui.painter().text(r.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, "Glisse tes fichiers ici", semibold(13.0), INK);
                ui.painter().text(r.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, "Vidéos, photos et musiques", FontId::proportional(11.5), FAINT);
                return;
            }
            let q = self.bin_query.to_lowercase();
            let mut items: Vec<usize> = (0..self.proj.sources.len())
                .filter(|i| {
                    let s = &self.proj.sources[*i];
                    (q.is_empty() || s.name().to_lowercase().contains(&q)) && (self.bin_filter == 0 || s.kind() as usize + 1 == self.bin_filter)
                })
                .collect();
            let srcs = &self.proj.sources;
            match self.bin_sort {
                1 => items.sort_by_key(|i| srcs[*i].name().to_lowercase()),
                2 => items.sort_by(|a, b| srcs[*b].duration.total_cmp(&srcs[*a].duration)),
                3 => items.sort_by_key(|i| srcs[*i].kind()),
                4 => items.sort_by_key(|i| (srcs[*i].label == 0, srcs[*i].label)),
                _ => {}
            }
            let mut action: Option<(u8, usize)> = None;
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                if self.bin_list == 1 {
                    for &i in &items {
                        let s = &self.proj.sources[i];
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(r, CornerRadius::same(8), Color32::from_white_alpha(8));
                        }
                        let pic = Rect::from_min_size(r.min + vec2(4.0, 6.0), vec2(56.0, 32.0));
                        self.media_thumb(ui.painter(), pic, i);
                        self.label_tag(ui.painter(), pic, i);
                        ui.painter().text(pic.right_top() + vec2(10.0, 2.0), Align2::LEFT_TOP, short(&s.name(), 22), FontId::proportional(12.5), INK);
                        ui.painter().text(pic.right_bottom() + vec2(10.0, -2.0), Align2::LEFT_BOTTOM, format!("{} · {}{}", kind_label(s), if s.kind() == 2 { "image".into() } else { tc(s.duration, 30) }, match self.uses(i) { 0 => String::new(), n => format!(" · {n}×") }), FontId::proportional(11.0), FAINT);
                        let resp = resp.on_hover_text(media_info(s));
                        if resp.double_clicked() {
                            action = Some((0, i));
                        }
                        resp.context_menu(|ui| media_menu(ui, s, i, &mut action));
                    }
                } else {
                    let w = (ui.available_width() - 8.0) / 2.0;
                    for row in items.chunks(2) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            for &i in row {
                                let s = &self.proj.sources[i];
                                let (r, resp) = ui.allocate_exact_size(vec2(w, w * 0.56 + 38.0), Sense::click());
                                let pic = Rect::from_min_size(r.min, vec2(w, w * 0.56));
                                self.media_thumb(ui.painter(), pic, i);
                                self.label_tag(ui.painter(), pic, i);
                                let p = ui.painter();
                                let tag = Rect::from_min_size(pic.right_bottom() - vec2(50.0, 21.0), vec2(46.0, 17.0));
                                p.rect_filled(tag, CornerRadius::same(5), Color32::from_black_alpha(170));
                                p.text(tag.center(), Align2::CENTER_CENTER, if s.kind() == 2 { "Image".to_string() } else { tc(s.duration, 30).chars().take(5).collect::<String>() }, FontId::proportional(10.5), Color32::WHITE);
                                if self.uses(i) > 0 {
                                    p.circle_filled(pic.left_bottom() + vec2(10.0, -10.0), 4.0, GREEN);
                                }
                                if resp.hovered() {
                                    p.rect_stroke(pic, CornerRadius::same(8), Stroke::new(1.5, BLUE), StrokeKind::Inside);
                                    let plus = Rect::from_min_size(pic.right_top() + vec2(-26.0, 4.0), Vec2::splat(22.0));
                                    p.circle_filled(plus.center(), 11.0, BLUE);
                                    glyph(p, plus.shrink(5.0), G::Plus, Color32::WHITE);
                                }
                                p.text(pos2(r.left() + 2.0, pic.bottom() + 8.0), Align2::LEFT_TOP, short(&s.name(), (w / 7.2) as usize), FontId::proportional(12.0), INK);
                                p.text(pos2(r.left() + 2.0, pic.bottom() + 23.0), Align2::LEFT_TOP, kind_label(s), FontId::proportional(10.5), FAINT);
                                let resp = resp.on_hover_text(media_info(s));
                                let plus_hit = resp.interact_pointer_pos().is_some_and(|pp| pp.x > pic.right() - 28.0 && pp.y < pic.top() + 28.0);
                                if resp.double_clicked() || (resp.clicked() && plus_hit) {
                                    action = Some((0, i));
                                }
                                resp.context_menu(|ui| media_menu(ui, s, i, &mut action));
                            }
                        });
                        ui.add_space(4.0);
                    }
                }
            });
            match action {
                Some((0, i)) => self.add_source(i),
                Some((1, i)) => self.add_music(i),
                Some((2, i)) => {
                    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", self.proj.sources[i].path)).spawn();
                }
                Some((3, i)) => self.remove_source(i),
                Some((4, i)) => self.insert_source(i),
                Some((5, i)) => self.overlay_source(i),
                Some((6, i)) => self.replace_source(i),
                Some((7, i)) => self.relink(i, ctx),
                Some((k, i)) if k >= 20 => self.proj.sources[i].label = k - 20,
                _ => {}
            }
        });
    }

    fn label_tag(&self, p: &Painter, pic: Rect, i: usize) {
        let l = self.proj.sources[i].label;
        if l > 0 {
            let rgb = LABELS[(l as usize - 1) % LABELS.len()].1;
            p.rect_filled(Rect::from_min_size(pic.left_top() + vec2(4.0, 4.0), vec2(4.0, 14.0)), CornerRadius::same(2), Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
        }
    }

    fn media_thumb(&self, p: &Painter, pic: Rect, i: usize) {
        p.rect_filled(pic, CornerRadius::same(7), WELL);
        match self.thumbs.get(&i) {
            Some(t) => {
                let [tw, th] = t.size();
                let k = (pic.width() / tw as f32).min(pic.height() / th as f32);
                p.image(t.id(), Rect::from_center_size(pic.center(), vec2(tw as f32, th as f32) * k), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            None if !self.proj.sources[i].has_video => {
                let n = 9;
                for k in 0..n {
                    let x = pic.center().x - (n as f32 * 3.5) + k as f32 * 7.0;
                    let hh = [6.0, 14.0, 22.0, 12.0, 20.0, 9.0, 16.0, 11.0, 7.0][k] * pic.height() / 60.0;
                    p.line_segment([pos2(x, pic.center().y - hh), pos2(x, pic.center().y + hh)], Stroke::new(2.5, MUSIC));
                }
            }
            None => {}
        }
    }

    fn viewer(&mut self, root: &mut Ui) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let head = Rect::from_min_size(full.min, vec2(full.width(), 34.0));
            ui.painter().text(head.left_center() + vec2(16.0, 0.0), Align2::LEFT_CENTER, "Programme", semibold(12.5), INK);
            if let Some((i, _)) = self.proj.local_at(self.t) {
                let src = &self.proj.sources[self.proj.clips[i].src];
                ui.painter().text(head.left_center() + vec2(100.0, 0.0), Align2::LEFT_CENTER, short(&src.name(), 40), FontId::proportional(12.0), FAINT);
            }
            let controls_h = 62.0;
            let screen = Rect::from_min_max(pos2(full.left() + 16.0, head.bottom()), pos2(full.right() - 34.0, full.bottom() - controls_h));
            let (w, h) = (self.proj.width as f32, self.proj.height as f32);
            let s = (screen.width() / w).min(screen.height() / h);
            let frame = Rect::from_center_size(screen.center(), vec2(w, h) * s);
            let p = ui.painter();
            p.add(egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(150) }.as_shape(frame, CornerRadius::same(6)));
            p.rect_filled(frame, CornerRadius::same(6), Color32::BLACK);
            if let (Some(t), true) = (&self.tex, self.shown) {
                p.image(t.id(), frame, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            } else if self.proj.clips.is_empty() {
                p.text(frame.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "Importe une vidéo pour commencer", semibold(16.0), INK);
                p.text(frame.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "Glisse un fichier ici ou Ctrl+I", FontId::proportional(12.5), MUTED);
            }
            let guide = Stroke::new(1.0, Color32::from_white_alpha(70));
            if self.safe {
                for k in [0.9, 0.8] {
                    p.rect_stroke(Rect::from_center_size(frame.center(), frame.size() * k), CornerRadius::ZERO, guide, StrokeKind::Middle);
                }
                p.line_segment([frame.center() - vec2(8.0, 0.0), frame.center() + vec2(8.0, 0.0)], guide);
                p.line_segment([frame.center() - vec2(0.0, 8.0), frame.center() + vec2(0.0, 8.0)], guide);
            }
            if self.grid {
                for k in [1.0 / 3.0, 2.0 / 3.0] {
                    p.line_segment([pos2(frame.left() + frame.width() * k, frame.top()), pos2(frame.left() + frame.width() * k, frame.bottom())], guide);
                    p.line_segment([pos2(frame.left(), frame.top() + frame.height() * k), pos2(frame.right(), frame.top() + frame.height() * k)], guide);
                }
            }
            if self.guides {
                let vw = (frame.height() * 9.0 / 16.0).min(frame.width());
                let r = Rect::from_center_size(frame.center(), vec2(vw, frame.height()));
                p.rect_filled(Rect::from_min_max(frame.min, pos2(r.left(), frame.bottom())), CornerRadius::ZERO, Color32::from_black_alpha(110));
                p.rect_filled(Rect::from_min_max(pos2(r.right(), frame.top()), frame.max), CornerRadius::ZERO, Color32::from_black_alpha(110));
                p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, BLUE), StrokeKind::Middle);
            }
            self.viewer_tools(ui, frame);
            self.meter(ui, Rect::from_min_max(pos2(full.right() - 26.0, head.bottom() + 6.0), pos2(full.right() - 12.0, full.bottom() - controls_h - 6.0)));

            let bar = Rect::from_min_max(pos2(full.left() + 16.0, full.bottom() - controls_h + 10.0), pos2(full.right() - 16.0, full.bottom() - 10.0));
            let p = ui.painter();
            p.text(bar.left_center(), Align2::LEFT_CENTER, tc(self.t, self.proj.fps), FontId::monospace(18.0), BLUE);
            p.text(bar.left_center() + vec2(98.0, 1.0), Align2::LEFT_CENTER, format!("/ {}", tc(self.proj.duration(), self.proj.fps)), FontId::monospace(12.5), FAINT);
            let step = 1.0 / self.proj.fps as f64;
            let center = bar.center();
            let buttons: [(f32, u8, &str); 5] = [(-100.0, 0, "Point de montage précédent (↑)"), (-54.0, 1, "Image précédente (←)"), (0.0, 2, "Lecture / pause (Espace)"), (54.0, 3, "Image suivante (→)"), (100.0, 4, "Point de montage suivant (↓)")];
            for (dx, kind, tip) in buttons {
                let size = if kind == 2 { 40.0 } else { 32.0 };
                let r = Rect::from_center_size(center + vec2(dx, 0.0), Vec2::splat(size));
                let resp = ui.interact(r, ui.make_persistent_id(("transport", kind)), Sense::click()).on_hover_text(tip);
                let fill = if kind == 2 { BLUE } else if resp.hovered() { FIELD } else { Color32::TRANSPARENT };
                ui.painter().circle_filled(r.center(), size / 2.0, if kind == 2 && resp.hovered() { BLUE.lerp_to_gamma(Color32::WHITE, 0.12) } else { fill });
                transport_glyph(ui.painter(), r.center(), kind, self.playing, if kind == 2 { Color32::WHITE } else { INK });
                if resp.clicked() {
                    match kind {
                        0 => self.jump(-1),
                        1 => self.seek(self.t - step),
                        2 => {
                            self.shuttle = 1.0;
                            self.set_playing(!self.playing);
                        }
                        3 => self.seek(self.t + step),
                        _ => self.jump(1),
                    }
                }
            }
            let right = Rect::from_min_max(pos2(center.x + 140.0, bar.top()), bar.max);
            ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if pro::icon_button(ui, 30.0, self.focus, "Mode visionneuse (F)", |p, r, c| glyph(p, r, G::Focus, c)).clicked() {
                        self.focus = !self.focus;
                    }
                    if pro::icon_button(ui, 30.0, self.safe, "Zones de sécurité", |p, r, c| glyph(p, r, G::Safe, c)).clicked() {
                        self.safe = !self.safe;
                    }
                    if pro::icon_button(ui, 30.0, self.grid, "Grille des tiers", |p, r, c| glyph(p, r, G::Grid, c)).clicked() {
                        self.grid = !self.grid;
                    }
                    if pro::icon_button(ui, 30.0, self.looping, "Lecture en boucle (entre entrée et sortie)", |p, r, c| glyph(p, r, G::Loop, c)).clicked() {
                        self.looping = !self.looping;
                    }
                    if pro::icon_button(ui, 30.0, self.mark_out.is_some(), "Point de sortie (O)", |p, r, c| glyph(p, r, G::Out, c)).clicked() {
                        self.mark_out = Some(self.t);
                    }
                    if pro::icon_button(ui, 30.0, self.mark_in.is_some(), "Point d'entrée (I)", |p, r, c| glyph(p, r, G::In, c)).clicked() {
                        self.mark_in = Some(self.t);
                    }
                    pro::toolbar_sep(ui);
                    ui.spacing_mut().slider_width = 60.0;
                    ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.5).show_value(false)).on_hover_text("Volume d'écoute");
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
                    speaker(ui.painter(), r.center(), self.volume == 0.0, if resp.hovered() { INK_STRONG } else { MUTED });
                    if resp.clicked() {
                        self.volume = if self.volume == 0.0 { 1.0 } else { 0.0 };
                    }
                });
            });
        });
    }

    fn meter(&self, ui: &mut Ui, r: Rect) {
        let (mut l, mut rr) = (0f32, 0f32);
        if let Ok(s) = self.audio.state.lock() {
            let end = (self.t * OUT_RATE as f64) as usize;
            let start = end.saturating_sub(OUT_RATE as usize / 20);
            for k in start..end {
                if k * 2 + 1 < s.mix.len() {
                    l = l.max(s.mix[k * 2].abs());
                    rr = rr.max(s.mix[k * 2 + 1].abs());
                }
            }
        }
        let vol = self.volume;
        let p = ui.painter();
        let db = |v: f32| ((20.0 * (v * vol).max(1e-5).log10() + 48.0) / 48.0).clamp(0.0, 1.0);
        for (k, v) in [l, rr].into_iter().enumerate() {
            let col = Rect::from_min_max(pos2(r.left() + k as f32 * 7.0, r.top()), pos2(r.left() + k as f32 * 7.0 + 5.0, r.bottom()));
            p.rect_filled(col, CornerRadius::same(2), WELL);
            let level = db(v);
            let top = col.bottom() - col.height() * level;
            let color = if level > 0.94 { RED } else if level > 0.8 { Color32::from_rgb(255, 196, 0) } else { GREEN };
            p.rect_filled(Rect::from_min_max(pos2(col.left(), top), col.max), CornerRadius::same(2), color);
        }
        for mark in [0.0, -6.0, -12.0, -24.0, -36.0] {
            let y = r.bottom() - r.height() * ((mark + 48.0) / 48.0);
            p.line_segment([pos2(r.left() - 3.0, y), pos2(r.left() - 1.0, y)], Stroke::new(1.0, FAINT));
        }
        ui.interact(r, ui.make_persistent_id("meter"), Sense::hover()).on_hover_text("Niveau audio du programme (crête, dBFS)");
        if self.playing {
            ui.ctx().request_repaint();
        }
    }

    fn viewer_tools(&mut self, ui: &mut Ui, frame: Rect) {
        let resp = ui.interact(frame, ui.make_persistent_id("program"), Sense::click_and_drag());
        let norm = |p: Pos2| [((p.x - frame.left()) / frame.width()).clamp(0.0, 1.0), ((p.y - frame.top()) / frame.height()).clamp(0.0, 1.0)];
        let to_screen = |n: [f32; 2]| pos2(frame.left() + n[0] * frame.width(), frame.top() + n[1] * frame.height());
        let target = self.mask_target();
        let mask = target.and_then(|(s, _)| self.clip_ref(s).and_then(|c| c.mask.clone()));
        let painter = ui.painter().with_clip_rect(frame.expand(6.0));
        let shows_mask = matches!(self.tool, Tool::Select | Tool::Pen | Tool::Rect | Tool::Ellipse) && (matches!(self.sel, Some(Sel::Clip(_) | Sel::Over(_))) || matches!(self.tool, Tool::Pen | Tool::Rect | Tool::Ellipse));
        let mut handles: Vec<Pos2> = Vec::new();
        if let Some(m) = &mask
            && shows_mask
        {
            let stroke = Stroke::new(1.5, Color32::from_rgb(255, 214, 10));
            match m.kind {
                0 | 1 if m.pts.len() >= 2 => {
                    let r = Rect::from_two_pos(to_screen(m.pts[0]), to_screen(m.pts[1]));
                    if m.kind == 0 {
                        painter.rect_stroke(r, CornerRadius::ZERO, stroke, StrokeKind::Middle);
                    } else {
                        let pts: Vec<Pos2> = (0..64).map(|k| {
                            let a = k as f32 / 64.0 * std::f32::consts::TAU;
                            r.center() + vec2(a.cos() * r.width() / 2.0, a.sin() * r.height() / 2.0)
                        }).collect();
                        painter.add(Shape::closed_line(pts, stroke));
                    }
                    handles = vec![to_screen(m.pts[0]), to_screen(m.pts[1])];
                }
                _ => {
                    let pts: Vec<Pos2> = m.pts.iter().map(|p| to_screen(*p)).collect();
                    painter.add(Shape::closed_line(pts.clone(), stroke));
                    handles = pts;
                }
            }
            for h in &handles {
                painter.rect(Rect::from_center_size(*h, Vec2::splat(8.0)), CornerRadius::same(1), Color32::WHITE, Stroke::new(1.0, Color32::BLACK), StrokeKind::Middle);
            }
        }
        if !self.pen.is_empty() {
            let pts: Vec<Pos2> = self.pen.iter().map(|p| to_screen(*p)).collect();
            painter.add(Shape::line(pts.clone(), Stroke::new(1.5, BLUE)));
            if let Some(hp) = resp.hover_pos() {
                painter.line_segment([*pts.last().unwrap_or(&hp), hp], Stroke::new(1.0, BLUE.gamma_multiply(0.6)));
            }
            for (k, p) in pts.iter().enumerate() {
                painter.circle(*p, if k == 0 { 5.0 } else { 3.5 }, if k == 0 { BLUE } else { Color32::WHITE }, Stroke::new(1.0, BLUE));
            }
        }
        if let Some(Sel::Title(i)) = self.sel
            && let Some(t) = self.proj.titles.get(i)
            && self.t >= t.start
            && self.t < t.start + t.dur
            && self.tool == Tool::Select
        {
            let c = to_screen([0.5 + t.dx, 0.5 + t.dy]);
            painter.circle_stroke(c, 6.0, Stroke::new(1.5, TITLE));
        }
        match self.tool {
            Tool::Text | Tool::TextV => ui.ctx().set_cursor_icon(egui::CursorIcon::Text),
            Tool::Pen | Tool::Rect | Tool::Ellipse => ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair),
            Tool::Select if resp.hovered() => ui.ctx().set_cursor_icon(egui::CursorIcon::Move),
            _ => {}
        }
        match self.tool {
            Tool::Text | Tool::TextV => {
                if resp.clicked()
                    && let Some(pp) = resp.interact_pointer_pos()
                {
                    let n = norm(pp);
                    self.add_title_at(n[0] - 0.5, n[1] - 0.5, self.tool == Tool::TextV);
                }
            }
            Tool::Pen => {
                if resp.clicked()
                    && let Some(pp) = resp.interact_pointer_pos()
                {
                    if self.pen.len() >= 3 && to_screen(self.pen[0]).distance(pp) < 9.0 {
                        self.close_pen();
                    } else {
                        self.pen.push(norm(pp));
                    }
                }
                if resp.double_clicked() {
                    self.close_pen();
                }
            }
            Tool::Rect | Tool::Ellipse => {
                if resp.drag_started()
                    && let (Some(pp), Some((s, _))) = (resp.interact_pointer_pos(), target)
                {
                    self.view_drag = Some((1, pp, self.proj.clone(), s, 0));
                }
                if let Some((1, start, _, s, _)) = self.view_drag.clone()
                    && let Some(pp) = resp.interact_pointer_pos()
                {
                    let kind = if self.tool == Tool::Rect { 0 } else { 1 };
                    if let Some(c) = self.clip_mut(s) {
                        let feather = c.mask.as_ref().map_or(5.0, |m| m.feather);
                        c.mask = Some(model::Mask { kind, pts: vec![norm(start), norm(pp)], feather, invert: false });
                    }
                    self.sel = Some(s);
                }
            }
            Tool::Select => {
                if resp.drag_started()
                    && let Some(pp) = resp.interact_pointer_pos()
                {
                    if let Some(k) = handles.iter().position(|h| h.distance(pp) < 9.0)
                        && let Some((s, _)) = target
                    {
                        self.view_drag = Some((2, pp, self.proj.clone(), s, k));
                    } else if let Some(s) = self.sel {
                        self.view_drag = Some((0, pp, self.proj.clone(), s, 0));
                    } else if let Some((s, _)) = target {
                        self.select_one(s);
                        self.view_drag = Some((0, pp, self.proj.clone(), s, 0));
                    }
                }
                if let Some((mode, start, orig, s, k)) = self.view_drag.clone()
                    && let Some(pp) = resp.interact_pointer_pos()
                {
                    let d = pp - start;
                    let (dx, dy) = (d.x / frame.width(), d.y / frame.height());
                    let local = target.map_or(0.0, |(_, l)| l);
                    self.proj = orig.clone();
                    match (mode, s) {
                        (2, _) => {
                            let n = norm(pp);
                            if let Some(m) = self.clip_mut(s).and_then(|c| c.mask.as_mut())
                                && k < m.pts.len()
                            {
                                m.pts[k] = n;
                            }
                        }
                        (_, Sel::Title(i)) if i < self.proj.titles.len() => {
                            let t = &mut self.proj.titles[i];
                            t.dx = (t.dx + dx).clamp(-0.6, 0.6);
                            t.dy = (t.dy + dy).clamp(-0.6, 0.6);
                        }
                        (_, Sel::Clip(_) | Sel::Over(_)) => {
                            if let Some(c) = self.clip_mut(s) {
                                let ev = c.eval(local);
                                let (nx, ny) = (ev.pos_x + dx * 2.0, ev.pos_y + dy * 2.0);
                                for (prop, v) in [(1u8, nx), (2u8, ny)] {
                                    if c.has_keys(prop) {
                                        c.set_key(prop, local, v);
                                    } else {
                                        c.set_prop(prop, v);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        if !resp.dragged() && !resp.drag_started() && ui.input(|i| !i.pointer.primary_down()) {
            self.view_drag = None;
        }
    }

    fn sheets(&mut self, ctx: &egui::Context) {
        if let Some(mut opts) = self.export.take() {
            let mut go = false;
            let mut cancel = false;
            let keep = pro::sheet(ctx, "export", 520.0, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(9), BLUE.gamma_multiply(0.25));
                    glyph(ui.painter(), r.shrink(9.0), G::Export, BLUE);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Exporter").font(semibold(18.0)).color(INK_STRONG));
                        ui.label(RichText::new("MP4 H.264 + AAC, GIF, WAV, PNG ou JPEG").size(12.0).color(MUTED));
                    });
                });
                ui.add_space(12.0);
                let label = |ui: &mut Ui, t: &str| {
                    ui.label(RichText::new(t).size(12.5).color(MUTED));
                };
                label(ui, "Préréglage");
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                    for (k, p) in EXPORT_PRESETS.iter().enumerate() {
                        if ui.selectable_label(opts.preset == k, p.0).clicked() {
                            let keep_range = opts.range;
                            opts = ExportOpts::from_preset(k);
                            opts.range = keep_range;
                        }
                    }
                });
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                egui::Grid::new("export-grid").num_columns(2).spacing(vec2(14.0, 8.0)).show(ui, |ui| {
                    label(ui, "Format");
                    egui::ComboBox::from_id_salt("exp-kind").selected_text(KINDS[opts.kind]).width(200.0).show_ui(ui, |ui| {
                        for (k, n) in KINDS.iter().enumerate() {
                            ui.selectable_value(&mut opts.kind, k, *n);
                        }
                    });
                    ui.end_row();
                    if opts.kind != 1 {
                        label(ui, "Taille");
                        ui.horizontal(|ui| {
                            let mut seq = opts.w == 0 && opts.h == 0;
                            if ui.checkbox(&mut seq, "Séquence").changed() {
                                (opts.w, opts.h) = if seq { (0, 0) } else { (self.proj.width, self.proj.height) };
                            }
                            if !seq {
                                ui.add(egui::DragValue::new(&mut opts.w).range(16..=7680).suffix(" px"));
                                ui.label("×");
                                ui.add(egui::DragValue::new(&mut opts.h).range(0..=4320).suffix(" px"));
                            }
                        });
                        ui.end_row();
                        label(ui, "Cadrage");
                        let mut f = opts.fill as usize;
                        if pro::segmented(ui, &mut f, &["Ajuster (bandes)", "Remplir (recadrer)"]) {
                            opts.fill = f == 1;
                        }
                        ui.end_row();
                    }
                    if matches!(opts.kind, 0 | 2 | 3) {
                        label(ui, "Images/s");
                        ui.horizontal(|ui| {
                            if ui.selectable_label(opts.fps == 0, "Séquence").clicked() {
                                opts.fps = 0;
                            }
                            for f in [10, 15, 24, 25, 30, 60] {
                                if ui.selectable_label(opts.fps == f, f.to_string()).clicked() {
                                    opts.fps = f;
                                }
                            }
                        });
                        ui.end_row();
                    }
                    if opts.kind == 0 {
                        label(ui, "Débit vidéo");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().slider_width = 200.0;
                            ui.add(egui::Slider::new(&mut opts.mbps, 1.0..=80.0).logarithmic(true).suffix(" Mb/s").max_decimals(1));
                        });
                        ui.end_row();
                        label(ui, "Audio AAC");
                        let names: Vec<&str> = ABR.iter().map(|a| a.0).collect();
                        pro::segmented(ui, &mut opts.abr, &names);
                        ui.end_row();
                    }
                    if opts.kind < 4 {
                        label(ui, "Plage");
                        let has_marks = self.mark_in.is_some() || self.mark_out.is_some();
                        let has_clip = matches!(self.sel, Some(Sel::Clip(_) | Sel::Over(_)));
                        egui::ComboBox::from_id_salt("exp-range").selected_text(["Toute la séquence", "Entre entrée et sortie", "Clip sélectionné"][opts.range.min(2)]).width(200.0).show_ui(ui, |ui| {
                            ui.selectable_value(&mut opts.range, 0, "Toute la séquence");
                            if has_marks {
                                ui.selectable_value(&mut opts.range, 1, "Entre entrée et sortie");
                            }
                            if has_clip {
                                ui.selectable_value(&mut opts.range, 2, "Clip sélectionné");
                            }
                        });
                        ui.end_row();
                    }
                });
                ui.add_space(6.0);
                ui.checkbox(&mut opts.open_after, "Afficher le fichier une fois terminé");
                ui.add_space(8.0);
                let (a, b) = self.export_range(opts.range);
                let secs = b - a;
                let (w, h) = self.out_size(&opts);
                let fps = if opts.fps == 0 { self.proj.fps } else { opts.fps } as f64;
                let info = match opts.kind {
                    1 => format!("Durée {} · environ {:.0} Mo", tc(secs, self.proj.fps), secs * OUT_RATE as f64 * 4.0 / 1e6),
                    2 => format!("{w} × {h} · {} images · environ {:.0} Mo", (secs * fps).ceil(), w as f64 * h as f64 * secs * fps * 0.25 / 1e6),
                    3 => format!("{w} × {h} · {} images PNG", (secs * fps).ceil()),
                    4 | 5 => format!("{w} × {h} · image à {}", tc(self.t, self.proj.fps)),
                    _ => format!("{w} × {h} · {} i/s · durée {} · environ {:.0} Mo", fps, tc(secs, self.proj.fps), secs * (opts.mbps as f64 * 1e6 + ABR[opts.abr.min(3)].1 as f64 * 8.0) / 8e6),
                };
                ui.label(RichText::new(info).size(12.5).color(FAINT));
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Exporter…").clicked() {
                        go = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        cancel = true;
                    }
                });
            });
            if go {
                self.run_export(&opts);
            } else if keep && !cancel {
                self.export = Some(opts);
            }
        }
        if self.seq_sheet {
            let mut close = false;
            let first = self.proj.clips.first().map(|c| self.proj.sources[c.src].clone());
            let keep = pro::sheet(ctx, "sequence", 500.0, |ui| {
                ui.label(RichText::new("Réglages de la séquence").font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(12.0);
                ui.label(RichText::new("Format").size(12.5).color(MUTED));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                    for (label, kind, w, h) in PRESETS {
                        if ui.selectable_label(self.proj.width == w && self.proj.height == h, format!("{label} · {kind}")).clicked() {
                            self.proj.width = w;
                            self.proj.height = h;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.proj.width).range(16..=7680).prefix("L ").suffix(" px"));
                    ui.add(egui::DragValue::new(&mut self.proj.height).range(16..=4320).prefix("H ").suffix(" px"));
                    if ui.small_button("⇄").on_hover_text("Inverser largeur et hauteur").clicked() {
                        (self.proj.width, self.proj.height) = (self.proj.height, self.proj.width);
                    }
                    if let Some(s) = &first
                        && s.width > 0
                        && ui.small_button("Comme le 1er clip").clicked()
                    {
                        (self.proj.width, self.proj.height) = (s.width / 2 * 2, s.height / 2 * 2);
                        self.proj.fps = s.fps.round().clamp(1.0, 120.0) as u32;
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Images par seconde").size(12.5).color(MUTED));
                ui.horizontal(|ui| {
                    for f in FPS {
                        if ui.selectable_label(self.proj.fps == f, format!("{f}")).clicked() {
                            self.proj.fps = f;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Couleur de fond").size(12.5).color(MUTED));
                    let b = self.proj.background;
                    let mut col = Color32::from_rgb(b[0], b[1], b[2]);
                    if ui.color_edit_button_srgba(&mut col).changed() {
                        self.proj.background = [col.r(), col.g(), col.b(), 255];
                    }
                    for (name, c) in [("Noir", [0, 0, 0]), ("Blanc", [255, 255, 255]), ("Gris", [40, 40, 44])] {
                        if ui.small_button(name).clicked() {
                            self.proj.background = [c[0], c[1], c[2], 255];
                        }
                    }
                });
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Terminé").clicked() {
                        close = true;
                    }
                });
            });
            self.seq_sheet = keep && !close;
        }
        if self.speed_sheet {
            let mut close = false;
            let s = self.sel;
            let max = s.and_then(|s| self.clip_ref(s)).map_or(0.0, |c| self.proj.sources[c.src].duration);
            let Some(c) = s.and_then(|s| self.clip_mut(s)) else {
                self.speed_sheet = false;
                return;
            };
            let keep = pro::sheet(ctx, "speed", 380.0, |ui| {
                ui.label(RichText::new("Vitesse/Durée du clip").font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(12.0);
                egui::Grid::new("speed-grid").num_columns(2).spacing(vec2(14.0, 10.0)).show(ui, |ui| {
                    ui.label(RichText::new("Vitesse").color(MUTED));
                    let mut pct = c.speed * 100.0;
                    if ui.add(egui::DragValue::new(&mut pct).range(10.0..=800.0).suffix(" %").speed(1.0)).changed() {
                        c.speed = pct / 100.0;
                    }
                    ui.end_row();
                    ui.label(RichText::new("Durée").color(MUTED));
                    let mut dur = c.len();
                    if ui.add(egui::DragValue::new(&mut dur).range(0.1..=3600.0).suffix(" s").speed(0.05).max_decimals(2)).changed() {
                        if c.hold > 0.0 {
                            c.hold = dur;
                        } else {
                            c.speed = ((c.out - c.inp) / dur.max(0.1)).clamp(0.1, 8.0);
                        }
                    }
                    ui.end_row();
                    ui.label(RichText::new("Options").color(MUTED));
                    ui.vertical(|ui| {
                        if ui.checkbox(&mut c.reverse, "Lecture inversée").changed() {
                            c.keys.clear();
                        }
                        let mut frozen = c.hold > 0.0;
                        if ui.checkbox(&mut frozen, "Image figée (maintien)").changed() {
                            c.hold = if frozen { c.len().max(0.5) } else { 0.0 };
                            if frozen {
                                c.out = (c.inp + 0.04).min(max);
                            }
                        }
                        ui.checkbox(&mut c.mute, "Couper le son");
                    });
                    ui.end_row();
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    for (name, v) in [("Ralenti ×0,25", 0.25), ("×0,5", 0.5), ("Normal", 1.0), ("×2", 2.0), ("Accéléré ×4", 4.0)] {
                        if ui.selectable_label((c.speed - v).abs() < 1e-3, name).clicked() {
                            c.speed = v;
                        }
                    }
                });
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Terminé").clicked() {
                        close = true;
                    }
                });
            });
            self.speed_sheet = keep && !close;
        }
        if let Some(k) = self.marker_edit {
            if k >= self.proj.markers.len() {
                self.marker_edit = None;
                return;
            }
            let mut close = false;
            let mut delete = false;
            let fps = self.proj.fps;
            let m = &mut self.proj.markers[k];
            let keep = pro::sheet(ctx, "marker", 400.0, |ui| {
                ui.label(RichText::new(format!("Marqueur · {}", tc(m.t, fps))).font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(12.0);
                ui.add(egui::TextEdit::singleline(&mut m.name).hint_text("Nom (chapitre)").desired_width(f32::INFINITY));
                ui.add_space(6.0);
                ui.add(egui::TextEdit::multiline(&mut m.note).hint_text("Commentaire").desired_rows(3).desired_width(f32::INFINITY));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Couleur").color(MUTED));
                    for (c, (name, rgb)) in MARKER_COLORS.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
                        ui.painter().circle_filled(r.center(), 7.5, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                        if m.color as usize == c {
                            ui.painter().circle_stroke(r.center(), 10.0, Stroke::new(1.5, Color32::WHITE));
                        }
                        if resp.on_hover_text(*name).clicked() {
                            m.color = c as u8;
                        }
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Durée").color(MUTED));
                    ui.add(egui::DragValue::new(&mut m.dur).range(0.0..=3600.0).suffix(" s").speed(0.05));
                });
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if pro::secondary(ui, "Supprimer").clicked() {
                        delete = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::primary(ui, "Terminé").clicked() {
                            close = true;
                        }
                    });
                });
            });
            if delete {
                self.proj.markers.remove(k);
            }
            if !keep || close || delete {
                self.marker_edit = None;
            }
        }
    }
}

fn kind_label(s: &Source) -> String {
    match s.kind() {
        2 if s.is_generator() => "Élément généré".into(),
        2 => format!("Image · {}×{}", s.width, s.height),
        0 => format!("{}×{}", s.width, s.height),
        _ => "Audio".into(),
    }
}

fn media_info(s: &Source) -> String {
    let mut info = s.name();
    if s.kind() != 2 {
        info.push_str(&format!("\nDurée {}", tc(s.duration, 30)));
    }
    if s.has_video {
        info.push_str(&format!("\n{} × {}", s.width, s.height));
        if s.kind() == 0 {
            info.push_str(&format!(" · {:.2} i/s", s.fps));
        }
    }
    info.push_str(if s.has_audio { "\nAvec son" } else { "\nSans son" });
    info.push_str("\nDouble-clic : ajouter à la fin de la timeline");
    info
}

fn media_menu(ui: &mut Ui, s: &Source, i: usize, action: &mut Option<(u8, usize)>) {
    let item = |ui: &mut Ui, label: &str, code: u8, action: &mut Option<(u8, usize)>| {
        if ui.button(label).clicked() {
            *action = Some((code, i));
            ui.close();
        }
    };
    if s.has_video {
        item(ui, "Ajouter à la fin de V1", 0, action);
        item(ui, "Insérer à la tête de lecture", 4, action);
        item(ui, "Incruster sur V2 à la tête", 5, action);
        item(ui, "Remplacer le clip sélectionné", 6, action);
    }
    if s.has_audio {
        item(ui, "Ajouter comme musique (A2)", 1, action);
    }
    ui.menu_button("Étiquette", |ui| {
        if ui.button("Aucune").clicked() {
            *action = Some((20, i));
            ui.close();
        }
        for (k, (name, _)) in LABELS.iter().enumerate() {
            if ui.button(*name).clicked() {
                *action = Some((21 + k as u8, i));
                ui.close();
            }
        }
    });
    ui.separator();
    if !s.is_generator() {
        item(ui, "Remplacer le fichier…", 7, action);
        item(ui, "Afficher dans l'explorateur", 2, action);
    }
    item(ui, "Retirer du projet", 3, action);
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

#[derive(Clone, Copy)]
enum G {
    Select,
    Blade,
    Slip,
    Rate,
    Hand,
    Zoom,
    Split,
    Trash,
    Freeze,
    Title,
    Marker,
    Magnet,
    Film,
    Wave,
    Plus,
    Minus,
    Fit,
    Export,
    Loop,
    In,
    Out,
    Safe,
    Focus,
    Grid,
    Gap,
    Eye,
    Lock,
}

fn tool_glyph(p: &Painter, r: Rect, c: Color32, t: Tool) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    let s = Stroke::new(1.6, c);
    let arrow = |p: &Painter, tip: Pos2, dir: f32| {
        p.line_segment([tip, tip + vec2(-dir * 3.5, -3.0)], s);
        p.line_segment([tip, tip + vec2(-dir * 3.5, 3.0)], s);
    };
    match t {
        Tool::Select => glyph(p, r, G::Select, c),
        Tool::TrackFwd | Tool::TrackBack => {
            let d = if t == Tool::TrackFwd { 1.0 } else { -1.0 };
            let x0 = if d > 0.0 { 0.15 } else { 0.85 };
            p.line_segment([at(x0, 0.5), at(1.0 - x0, 0.5)], s);
            arrow(p, at(1.0 - x0, 0.5), d);
            arrow(p, at(1.0 - x0 - 0.25 * d, 0.5), d);
            p.line_segment([at(x0, 0.15), at(x0, 0.85)], Stroke::new(2.0, c));
        }
        Tool::Ripple => {
            p.add(Shape::line(vec![at(0.3, 0.1), at(0.5, 0.1), at(0.5, 0.9), at(0.3, 0.9)], s));
            p.line_segment([at(0.6, 0.5), at(1.0, 0.5)], s);
            arrow(p, at(1.0, 0.5), 1.0);
        }
        Tool::Rolling => {
            p.line_segment([at(0.5, 0.05), at(0.5, 0.95)], Stroke::new(2.0, c));
            p.line_segment([at(0.0, 0.5), at(0.35, 0.5)], s);
            p.line_segment([at(0.65, 0.5), at(1.0, 0.5)], s);
            arrow(p, at(0.0, 0.5), -1.0);
            arrow(p, at(1.0, 0.5), 1.0);
        }
        Tool::Rate => glyph(p, r, G::Rate, c),
        Tool::Blade => glyph(p, r, G::Blade, c),
        Tool::BladeAll => {
            glyph(p, Rect::from_min_size(r.min, r.size() * 0.8), G::Blade, c);
            p.line_segment([at(0.75, 0.95), at(1.0, 0.95)], s);
            p.line_segment([at(0.75, 0.75), at(1.0, 0.75)], s);
        }
        Tool::Slip => glyph(p, r, G::Slip, c),
        Tool::Slide => {
            p.rect_filled(Rect::from_min_max(at(0.3, 0.25), at(0.7, 0.75)), CornerRadius::same(2), c);
            p.line_segment([at(0.0, 0.25), at(0.0, 0.75)], s);
            p.line_segment([at(1.0, 0.25), at(1.0, 0.75)], s);
            arrow(p, at(0.15, 0.5), -1.0);
            arrow(p, at(0.85, 0.5), 1.0);
        }
        Tool::Pen => {
            p.add(Shape::convex_polygon(vec![at(0.5, 0.0), at(0.85, 0.55), at(0.5, 1.0), at(0.15, 0.55)], Color32::TRANSPARENT, s));
            p.circle_filled(at(0.5, 0.55), 1.8, c);
            p.line_segment([at(0.5, 0.0), at(0.5, 0.45)], Stroke::new(1.0, c));
        }
        Tool::Rect => {
            p.rect_stroke(Rect::from_min_max(at(0.08, 0.2), at(0.92, 0.8)), CornerRadius::ZERO, s, StrokeKind::Middle);
        }
        Tool::Ellipse => {
            let pts: Vec<Pos2> = (0..32).map(|k| {
                let a = k as f32 / 32.0 * std::f32::consts::TAU;
                at(0.5 + a.cos() * 0.45, 0.5 + a.sin() * 0.33)
            }).collect();
            p.add(Shape::closed_line(pts, s));
        }
        Tool::Hand => glyph(p, r, G::Hand, c),
        Tool::Zoom => glyph(p, r, G::Zoom, c),
        Tool::Text => {
            p.text(r.center(), Align2::CENTER_CENTER, "T", semibold(r.height() * 1.25), c);
        }
        Tool::TextV => {
            p.text(at(0.38, 0.5), Align2::CENTER_CENTER, "T", semibold(r.height() * 1.05), c);
            p.line_segment([at(0.85, 0.05), at(0.85, 0.95)], s);
            arrow(p, at(0.85, 0.95), 0.0);
            p.line_segment([at(0.85, 0.95), at(0.72, 0.8)], s);
            p.line_segment([at(0.85, 0.95), at(0.98, 0.8)], s);
        }
    }
}

fn glyph(p: &Painter, r: Rect, g: G, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    let s = Stroke::new(1.7, c);
    match g {
        G::Select => {
            p.add(Shape::convex_polygon(vec![at(0.22, 0.0), at(0.85, 0.6), at(0.52, 0.62), at(0.33, 1.0)], c, Stroke::NONE));
        }
        G::Blade => {
            p.add(Shape::convex_polygon(vec![at(0.12, 1.0), at(0.72, 0.0), at(1.0, 0.22), at(0.38, 1.0)], c, Stroke::NONE));
        }
        G::Slip => {
            p.rect_stroke(Rect::from_min_max(at(0.18, 0.15), at(0.82, 0.85)), CornerRadius::same(2), s, StrokeKind::Middle);
            p.line_segment([at(0.0, 0.5), at(1.0, 0.5)], s);
            for d in [-1.0, 1.0] {
                let tip = if d < 0.0 { at(0.0, 0.5) } else { at(1.0, 0.5) };
                p.line_segment([tip, tip + vec2(-d * 4.0, -3.5)], s);
                p.line_segment([tip, tip + vec2(-d * 4.0, 3.5)], s);
            }
        }
        G::Rate => {
            p.circle_stroke(at(0.5, 0.5), r.width() * 0.45, s);
            p.line_segment([at(0.5, 0.5), at(0.5, 0.18)], s);
            p.line_segment([at(0.5, 0.5), at(0.75, 0.62)], s);
        }
        G::Hand => {
            for (k, hh) in [0.55, 0.75, 0.8, 0.7].iter().enumerate() {
                let x = 0.22 + k as f32 * 0.19;
                p.line_segment([at(x, 0.62), at(x, 0.62 - hh * 0.6)], Stroke::new(2.2, c));
            }
            p.add(Shape::convex_polygon(vec![at(0.12, 0.6), at(0.85, 0.6), at(0.75, 1.0), at(0.25, 1.0)], c, Stroke::NONE));
        }
        G::Zoom => {
            p.circle_stroke(at(0.42, 0.42), r.width() * 0.32, s);
            p.line_segment([at(0.66, 0.66), at(0.98, 0.98)], Stroke::new(2.4, c));
        }
        G::Split => {
            p.circle_stroke(at(0.25, 0.78), r.width() * 0.16, s);
            p.circle_stroke(at(0.75, 0.78), r.width() * 0.16, s);
            p.line_segment([at(0.35, 0.65), at(0.8, 0.0)], s);
            p.line_segment([at(0.65, 0.65), at(0.2, 0.0)], s);
        }
        G::Trash => {
            p.line_segment([at(0.08, 0.2), at(0.92, 0.2)], s);
            p.line_segment([at(0.35, 0.2), at(0.4, 0.05)], s);
            p.line_segment([at(0.4, 0.05), at(0.6, 0.05)], s);
            p.line_segment([at(0.6, 0.05), at(0.65, 0.2)], s);
            p.add(Shape::convex_polygon(vec![at(0.18, 0.28), at(0.82, 0.28), at(0.74, 1.0), at(0.26, 1.0)], Color32::TRANSPARENT, s));
        }
        G::Freeze => {
            for a in 0..3 {
                let ang = a as f32 * std::f32::consts::PI / 3.0;
                let d = vec2(ang.cos(), ang.sin()) * r.width() * 0.48;
                p.line_segment([r.center() - d, r.center() + d], s);
            }
        }
        G::Title => {
            p.line_segment([at(0.1, 0.12), at(0.9, 0.12)], Stroke::new(2.2, c));
            p.line_segment([at(0.5, 0.12), at(0.5, 0.95)], Stroke::new(2.2, c));
        }
        G::Marker => {
            p.add(Shape::convex_polygon(vec![at(0.5, 0.0), at(0.95, 0.45), at(0.5, 0.9), at(0.05, 0.45)], c, Stroke::NONE));
        }
        G::Magnet => {
            let pts: Vec<Pos2> = (0..=16)
                .map(|k| {
                    let a = std::f32::consts::PI * k as f32 / 16.0;
                    at(0.5 + a.cos() * 0.38, 0.5 + a.sin() * 0.38)
                })
                .collect();
            p.add(Shape::line(pts, Stroke::new(3.0, c)));
            p.line_segment([at(0.12, 0.5), at(0.12, 0.1)], Stroke::new(3.0, c));
            p.line_segment([at(0.88, 0.5), at(0.88, 0.1)], Stroke::new(3.0, c));
        }
        G::Film => {
            p.rect_stroke(Rect::from_min_max(at(0.05, 0.1), at(0.95, 0.9)), CornerRadius::same(2), s, StrokeKind::Middle);
            for k in 0..3 {
                let x = 0.22 + k as f32 * 0.28;
                p.rect_filled(Rect::from_center_size(at(x, 0.22), vec2(3.0, 2.0)), CornerRadius::ZERO, c);
                p.rect_filled(Rect::from_center_size(at(x, 0.78), vec2(3.0, 2.0)), CornerRadius::ZERO, c);
            }
        }
        G::Wave => {
            for (k, hh) in [0.3, 0.7, 1.0, 0.5, 0.8, 0.35].iter().enumerate() {
                let x = 0.1 + k as f32 * 0.16;
                p.line_segment([at(x, 0.5 - hh * 0.45), at(x, 0.5 + hh * 0.45)], Stroke::new(2.0, c));
            }
        }
        G::Plus => {
            p.line_segment([at(0.5, 0.1), at(0.5, 0.9)], Stroke::new(2.0, c));
            p.line_segment([at(0.1, 0.5), at(0.9, 0.5)], Stroke::new(2.0, c));
        }
        G::Minus => {
            p.line_segment([at(0.1, 0.5), at(0.9, 0.5)], Stroke::new(2.0, c));
        }
        G::Fit => {
            for (a, b, d) in [(at(0.05, 0.3), at(0.05, 0.7), 1.0), (at(0.95, 0.3), at(0.95, 0.7), -1.0)] {
                p.line_segment([a, b], s);
                let m = pos2(a.x, (a.y + b.y) / 2.0);
                p.line_segment([m + vec2(d * 2.0, 0.0), m + vec2(d * r.width() * 0.35, 0.0)], s);
            }
        }
        G::Export => {
            p.line_segment([at(0.5, 0.05), at(0.5, 0.65)], Stroke::new(2.0, c));
            p.line_segment([at(0.5, 0.05), at(0.25, 0.3)], Stroke::new(2.0, c));
            p.line_segment([at(0.5, 0.05), at(0.75, 0.3)], Stroke::new(2.0, c));
            p.add(Shape::line(vec![at(0.12, 0.55), at(0.12, 0.95), at(0.88, 0.95), at(0.88, 0.55)], Stroke::new(2.0, c)));
        }
        G::Loop => {
            let pts: Vec<Pos2> = (0..=20)
                .map(|k| {
                    let a = -0.6 + k as f32 * 0.26;
                    at(0.5 + a.cos() * 0.4, 0.5 + a.sin() * 0.4)
                })
                .collect();
            let tip = pts[0];
            p.add(Shape::line(pts, s));
            p.add(Shape::convex_polygon(vec![tip + vec2(-4.0, -2.0), tip + vec2(3.0, -4.0), tip + vec2(1.0, 3.0)], c, Stroke::NONE));
        }
        G::In => {
            p.add(Shape::line(vec![at(0.7, 0.05), at(0.3, 0.05), at(0.3, 0.95), at(0.7, 0.95)], Stroke::new(2.0, c)));
        }
        G::Out => {
            p.add(Shape::line(vec![at(0.3, 0.05), at(0.7, 0.05), at(0.7, 0.95), at(0.3, 0.95)], Stroke::new(2.0, c)));
        }
        G::Safe => {
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.1), at(1.0, 0.9)), CornerRadius::same(2), s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(at(0.2, 0.3), at(0.8, 0.7)), CornerRadius::ZERO, Stroke::new(1.0, c), StrokeKind::Middle);
        }
        G::Focus => {
            for (cx, cy, dx, dy) in [(0.0, 0.0, 1.0, 1.0), (1.0, 0.0, -1.0, 1.0), (0.0, 1.0, 1.0, -1.0), (1.0, 1.0, -1.0, -1.0)] {
                let o = at(cx, cy);
                p.line_segment([o, o + vec2(dx * 5.0, 0.0)], s);
                p.line_segment([o, o + vec2(0.0, dy * 5.0)], s);
            }
        }
        G::Grid => {
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.1), at(1.0, 0.9)), CornerRadius::same(2), s, StrokeKind::Middle);
            for k in [0.36, 0.64] {
                p.line_segment([at(k, 0.1), at(k, 0.9)], Stroke::new(1.0, c));
                p.line_segment([at(0.0, 0.1 + 0.8 * (k - 0.2) / 0.6 * 0.6 + 0.08), at(1.0, 0.1 + 0.8 * (k - 0.2) / 0.6 * 0.6 + 0.08)], Stroke::new(1.0, c));
            }
        }
        G::Gap => {
            p.rect_filled(Rect::from_min_max(at(0.0, 0.25), at(0.3, 0.75)), CornerRadius::same(1), c);
            p.rect_filled(Rect::from_min_max(at(0.7, 0.25), at(1.0, 0.75)), CornerRadius::same(1), c);
            p.line_segment([at(0.35, 0.5), at(0.65, 0.5)], s);
            p.line_segment([at(0.35, 0.5), at(0.45, 0.4)], s);
            p.line_segment([at(0.65, 0.5), at(0.55, 0.4)], s);
        }
        G::Eye => {
            let pts: Vec<Pos2> = (0..=12).map(|k| at(k as f32 / 12.0, 0.5 - (k as f32 / 12.0 * std::f32::consts::PI).sin() * 0.32)).chain((0..=12).rev().map(|k| at(k as f32 / 12.0, 0.5 + (k as f32 / 12.0 * std::f32::consts::PI).sin() * 0.32))).collect();
            p.add(Shape::closed_line(pts, s));
            p.circle_filled(at(0.5, 0.5), r.width() * 0.13, c);
        }
        G::Lock => {
            p.rect_filled(Rect::from_min_max(at(0.15, 0.45), at(0.85, 1.0)), CornerRadius::same(2), c);
            let pts: Vec<Pos2> = (0..=12).map(|k| {
                let a = std::f32::consts::PI * (1.0 + k as f32 / 12.0);
                at(0.5 + a.cos() * 0.25, 0.42 + a.sin() * 0.32)
            }).collect();
            p.add(Shape::line(pts, s));
        }
    }
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
            .with_inner_size([1560.0, 960.0])
            .with_min_inner_size([1180.0, 700.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Cut", options, Box::new(|cc| Ok(Box::new(Cut::new(cc)))))
}
