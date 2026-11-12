#![windows_subsystem = "windows"]

mod look;
mod media;
mod model;
mod tutos;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, Painter, Pos2,
    Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2, pos2,
    vec2,
};
use media::{OUT_RATE, VideoReader};
use model::{ANIMS, Clip, FONTS, LABELS, LOOKS, Music, PLACES, Project, Source, Title};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, WELL, semibold};

const MUSIC: Color32 = Color32::from_rgb(48, 180, 110);
const TITLE: Color32 = Color32::from_rgb(255, 159, 10);
const PLAYHEAD: Color32 = Color32::from_rgb(255, 69, 58);
const MARK: Color32 = Color32::from_rgb(255, 196, 0);
const EXT: [&str; 12] = ["mp4", "mov", "m4v", "mkv", "avi", "wmv", "webm", "mp3", "wav", "m4a", "aac", "wma"];
const PRESETS: [(&str, &str, u32, u32); 6] = [
    ("1080p", "Paysage 16:9", 1920, 1080),
    ("4K UHD", "Paysage 16:9", 3840, 2160),
    ("720p", "Paysage 16:9", 1280, 720),
    ("Vertical", "Shorts, Reels, TikTok", 1080, 1920),
    ("Carré", "Publication 1:1", 1080, 1080),
    ("Cinéma", "21:9", 2560, 1080),
];
const RESOLUTIONS: [(&str, u32); 4] = [("Séquence", 0), ("720p", 720), ("1080p", 1080), ("4K", 2160)];
const QUALITIES: [(&str, f64); 3] = [("Légère", 0.5), ("Standard", 1.0), ("Haute", 2.0)];

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
    path: PathBuf,
}

struct ExportOpts {
    res: usize,
    quality: usize,
    range: usize,
    audio_only: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Sel {
    Clip(usize),
    Music(usize),
    Title(usize),
}

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Blade,
    Slip,
    Rate,
    Hand,
    Zoom,
}

const TOOLS: [(Tool, &str, G); 6] = [
    (Tool::Select, "Sélection (V)", G::Select),
    (Tool::Blade, "Lame (C) : clique un clip pour le couper", G::Blade),
    (Tool::Slip, "Glissement (Y) : décale le contenu sans changer la durée", G::Slip),
    (Tool::Rate, "Vitesse (R) : tire le bord droit pour accélérer ou ralentir", G::Rate),
    (Tool::Hand, "Main (H) : fait défiler la timeline", G::Hand),
    (Tool::Zoom, "Zoom (Z) : clic pour zoomer, Alt pour dézoomer", G::Zoom),
];

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
    help: studio_ui::help::Help,
    composed: Option<(usize, i64, Clip, Vec<Title>, u8, [u8; 4])>,
    looping: bool,
    snap: bool,
    show_waves: bool,
    show_thumbs: bool,
    track_scale: f32,
    clip_board: Option<Clip>,
    mark_in: Option<f64>,
    mark_out: Option<f64>,
    tab: usize,
    export: Option<ExportOpts>,
    seq_sheet: bool,
    bin_query: String,
    bin_list: usize,
    volume: f32,
    safe: bool,
    focus: bool,
    tl_scroll: f32,
    force_scroll: Option<f32>,
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
            readers: HashMap::new(),
            tex: None,
            shown: None,
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
            help: studio_ui::help::Help::new("Cut", tutos::CUT),
            composed: None,
            looping: false,
            snap: true,
            show_waves: true,
            show_thumbs: true,
            track_scale: 1.0,
            clip_board: None,
            mark_in: None,
            mark_out: None,
            tab: 0,
            export: None,
            seq_sheet: false,
            bin_query: String::new(),
            bin_list: 0,
            volume: 1.0,
            safe: false,
            focus: false,
            tl_scroll: 0.0,
            force_scroll: None,
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
        self.composed = None;
        self.undo.clear();
        self.redo.clear();
        self.sel = None;
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

    fn add_source(&mut self, i: usize) {
        let s = &self.proj.sources[i];
        let d = s.duration;
        if s.has_video {
            self.proj.clips.push(Clip { src: i, out: d, ..Default::default() });
            self.sel = Some(Sel::Clip(self.proj.clips.len() - 1));
        } else {
            self.add_music(i);
        }
    }

    fn add_music(&mut self, i: usize) {
        let d = self.proj.sources[i].duration;
        self.proj.music.push(Music { src: i, start: self.t, inp: 0.0, out: d, volume: 0.6, fade_in: 0.0, fade_out: 0.0, mute: false, pan: 0.0 });
        self.sel = Some(Sel::Music(self.proj.music.len() - 1));
    }

    fn remove_source(&mut self, i: usize) {
        if self.proj.clips.iter().any(|c| c.src == i) || self.proj.music.iter().any(|m| m.src == i) {
            self.error = Some("Ce média est utilisé dans la timeline".into());
            return;
        }
        self.proj.sources.remove(i);
        for c in &mut self.proj.clips {
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
        self.readers.clear();
        self.composed = None;
        self.mixed = None;
    }

    fn pump(&mut self, ctx: &egui::Context) {
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
            self.t = self.mark_in.unwrap_or(0.0);
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

    fn preview_size(&self) -> (u32, u32) {
        let k = (960.0 / self.proj.width.max(self.proj.height) as f32).min(1.0);
        (((self.proj.width as f32 * k) as u32).max(2), ((self.proj.height as f32 * k) as u32).max(2))
    }

    fn update_frame(&mut self, ctx: &egui::Context) {
        let (pw, ph) = self.preview_size();
        let titles: Vec<Title> = self.proj.titles_at(self.t).into_iter().cloned().collect();
        let bg = self.proj.background;
        let Some((ci, st)) = self.proj.clip_at(self.t) else {
            let key = (usize::MAX, (self.t * 30.0) as i64, Clip::default(), titles.clone(), 255, bg);
            if self.composed.as_ref() != Some(&key) {
                let refs: Vec<&Title> = titles.iter().collect();
                let frame = look::compose(&[], 0, 0, pw, ph, &look::Frame { clip: None, fade: 1.0, progress: 0.0, background: bg, titles: &refs, t: self.t });
                self.set_tex(ctx, pw, ph, &frame);
                self.composed = Some(key);
            }
            self.shown = Some((usize::MAX, 0.0));
            return;
        };
        let src_i = self.proj.clips[ci].src;
        let fade = self.proj.fade_at(self.t);
        let clip = self.proj.clips[ci].clone();
        let progress = self.proj.local_at(self.t).map_or(0.0, |(_, l)| (l / clip.len().max(1e-6)) as f32);
        let key = (src_i, (st * self.proj.fps as f64 * 2.0).round() as i64 + (progress * 1000.0) as i64 * (clip.zoom_end > 0.0) as i64, clip.clone(), titles.clone(), (fade * 255.0) as u8, bg);
        if self.composed.as_ref() == Some(&key) {
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
        let (sw, sh) = (r.width, r.height);
        match r.frame_at(st, src.fps) {
            Ok(Some((_, rgba))) => {
                let refs: Vec<&Title> = titles.iter().collect();
                let frame = look::compose(rgba, sw, sh, pw, ph, &look::Frame { clip: Some(&clip), fade, progress, background: bg, titles: &refs, t: self.t });
                self.set_tex(ctx, pw, ph, &frame);
                self.shown = Some((src_i, st));
                self.composed = Some(key);
            }
            Ok(None) => {}
            Err(e) => self.error = Some(e),
        }
    }

    fn set_tex(&mut self, ctx: &egui::Context, w: u32, h: u32, rgba: &[u8]) {
        let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba);
        match &mut self.tex {
            Some(t) => t.set(img, TextureOptions::LINEAR),
            None => self.tex = Some(ctx.load_texture("frame", img, TextureOptions::LINEAR)),
        }
    }

    fn run_export(&mut self, opts: &ExportOpts) {
        if self.proj.clips.is_empty() && self.proj.music.is_empty() {
            self.error = Some("La timeline est vide".into());
            return;
        }
        let (a, b) = if opts.range == 1 { (self.mark_in.unwrap_or(0.0), self.mark_out.unwrap_or(self.proj.duration())) } else { (0.0, self.proj.duration()) };
        let (a, b) = (a.min(b), b.max(a));
        let stem = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("montage".into());
        let proj = self.proj.clone();
        let data = self.audio_data.clone();
        let done = Arc::new(AtomicUsize::new(0));
        let error = Arc::new(Mutex::new(None));
        let (d2, e2) = (done.clone(), error.clone());
        if opts.audio_only {
            let Some(path) = rfd::FileDialog::new().add_filter("Audio WAV", &["wav"]).set_file_name(format!("{stem}.wav")).save_file() else { return };
            let p2 = path.clone();
            std::thread::spawn(move || {
                let mix = proj.mix(&|i| data.get(&i).cloned(), OUT_RATE);
                let s = ((a * OUT_RATE as f64) as usize * 2).min(mix.len());
                let e = ((b * OUT_RATE as f64) as usize * 2).min(mix.len());
                if let Err(err) = write_wav(&p2, &mix[s..e]) {
                    *e2.lock().unwrap() = Some(err);
                }
                d2.store(usize::MAX, Ordering::Relaxed);
            });
            self.job = Some(ExportJob { done, total: 1, error, path });
            return;
        }
        let Some(path) = rfd::FileDialog::new().add_filter("Vidéo MP4", &["mp4"]).set_file_name(format!("{stem}.mp4")).save_file() else { return };
        let target = RESOLUTIONS[opts.res].1;
        let (w, h) = if target == 0 {
            (proj.width, proj.height)
        } else {
            let k = target as f64 / proj.width.min(proj.height) as f64;
            (((proj.width as f64 * k) as u32 / 2 * 2).max(2), ((proj.height as f64 * k) as u32 / 2 * 2).max(2))
        };
        let factor = QUALITIES[opts.quality].1;
        let fps = proj.fps;
        let first = (a * fps as f64).round() as usize;
        let total = ((b - a) * fps as f64).ceil().max(1.0) as usize;
        let p2 = path.clone();
        std::thread::spawn(move || {
            let r = (|| -> Result<(), String> {
                media::startup();
                let mix = proj.mix(&|i| data.get(&i).cloned(), OUT_RATE);
                let mut enc = media::Encoder::new(&p2, w, h, fps, ((w * h * fps / 8) as f64 * factor).max(1_500_000.0) as u32)?;
                let mut readers: HashMap<usize, VideoReader> = HashMap::new();
                let per = (OUT_RATE / fps) as usize;
                for n in 0..total {
                    let t = (first + n) as f64 / fps as f64;
                    let titles = proj.titles_at(t);
                    let base = look::Frame { clip: None, fade: 1.0, progress: 0.0, background: proj.background, titles: &titles, t };
                    let frame = match proj.clip_at(t) {
                        Some((ci, st)) => {
                            let c = &proj.clips[ci];
                            let src = &proj.sources[c.src];
                            if !readers.contains_key(&c.src) {
                                readers.insert(c.src, VideoReader::open(Path::new(&src.path))?.0);
                            }
                            let rd = readers.get_mut(&c.src).ok_or("lecteur")?;
                            let (sw, sh) = (rd.width, rd.height);
                            let progress = proj.local_at(t).map_or(0.0, |(_, l)| (l / c.len().max(1e-6)) as f32);
                            match rd.frame_at(st, src.fps)? {
                                Some((_, rgba)) => look::compose(rgba, sw, sh, w, h, &look::Frame { clip: Some(c), fade: proj.fade_at(t), progress, ..base }),
                                None => look::compose(&[], 0, 0, w, h, &base),
                            }
                        }
                        None => look::compose(&[], 0, 0, w, h, &base),
                    };
                    enc.video(&frame)?;
                    let s = ((first + n) * per * 2).min(mix.len());
                    let e = ((first + n + 1) * per * 2).min(mix.len());
                    let mut chunk = mix[s..e].to_vec();
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

    fn delete_sel(&mut self) {
        match self.sel.take() {
            Some(Sel::Clip(i)) if i < self.proj.clips.len() => {
                self.proj.clips.remove(i);
            }
            Some(Sel::Music(i)) if i < self.proj.music.len() => {
                self.proj.music.remove(i);
            }
            Some(Sel::Title(i)) if i < self.proj.titles.len() => {
                self.proj.titles.remove(i);
            }
            _ => {}
        }
    }

    fn duplicate(&mut self) {
        match self.sel {
            Some(Sel::Clip(i)) if i < self.proj.clips.len() => {
                let c = self.proj.clips[i].clone();
                self.proj.clips.insert(i + 1, c);
                self.sel = Some(Sel::Clip(i + 1));
            }
            Some(Sel::Title(i)) if i < self.proj.titles.len() => {
                let mut t = self.proj.titles[i].clone();
                t.start += t.dur;
                self.proj.titles.push(t);
                self.sel = Some(Sel::Title(self.proj.titles.len() - 1));
            }
            Some(Sel::Music(i)) if i < self.proj.music.len() => {
                let mut m = self.proj.music[i].clone();
                m.start += m.out - m.inp;
                self.proj.music.push(m);
                self.sel = Some(Sel::Music(self.proj.music.len() - 1));
            }
            _ => {}
        }
    }

    fn paste_clip(&mut self) {
        let Some(c) = self.clip_board.clone() else { return };
        let at = match self.proj.clip_at(self.t) {
            Some((i, _)) => {
                self.proj.split(self.t);
                let starts = self.proj.starts();
                starts.iter().position(|s| (*s - self.t).abs() < 1e-6).unwrap_or(i + 1)
            }
            None => self.proj.clips.len(),
        };
        self.proj.clips.insert(at, c);
        self.sel = Some(Sel::Clip(at));
    }

    fn freeze_frame(&mut self) {
        let Some((i, st)) = self.proj.clip_at(self.t) else { return };
        let mut still = self.proj.clips[i].clone();
        still.inp = st;
        still.out = st + 0.04;
        still.hold = 2.0;
        still.fade_in = 0.0;
        still.fade_out = 0.0;
        let at = if self.proj.split(self.t) { i + 1 } else if self.proj.local_at(self.t).is_some_and(|(_, l)| l < 0.05) { i } else { i + 1 };
        self.proj.clips.insert(at, still);
        self.sel = Some(Sel::Clip(at));
    }

    fn nudge_clip(&mut self, by: i32) {
        let Some(Sel::Clip(i)) = self.sel else { return };
        let to = i as i32 + by;
        if to >= 0 && (to as usize) < self.proj.clips.len() {
            self.proj.clips.swap(i, to as usize);
            self.sel = Some(Sel::Clip(to as usize));
        }
    }

    fn add_marker(&mut self) {
        if let Some(k) = self.proj.markers.iter().position(|m| (m - self.t).abs() < 0.05) {
            self.proj.markers.remove(k);
        } else {
            self.proj.markers.push(self.t);
            self.proj.markers.sort_by(f64::total_cmp);
        }
    }

    fn edit_points(&self) -> Vec<f64> {
        let mut stops = self.proj.starts();
        stops.push(self.proj.duration());
        stops.extend(self.proj.markers.iter().copied());
        stops.extend(self.proj.titles.iter().flat_map(|t| [t.start, t.start + t.dur]));
        stops.extend(self.proj.music.iter().flat_map(|m| [m.start, m.start + m.out - m.inp]));
        stops.extend(self.mark_in.iter().chain(self.mark_out.iter()).copied());
        stops
    }

    fn jump(&mut self, dir: i32) {
        let t = self.t;
        let stops = self.edit_points();
        let target = if dir > 0 {
            stops.into_iter().filter(|s| *s > t + 1e-3).fold(f64::INFINITY, f64::min)
        } else {
            stops.into_iter().filter(|s| *s < t - 1e-3).fold(f64::NEG_INFINITY, f64::max)
        };
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
        for s in self.edit_points().into_iter().chain([0.0, self.t]) {
            if (s - t).abs() < dist {
                dist = (s - t).abs();
                best = s;
            }
        }
        best
    }

    fn add_title(&mut self) {
        self.proj.titles.push(Title { text: "Ton titre".into(), start: self.t, dur: 3.0, size: 0.08, place: 1, color: [255, 255, 255, 255], band: false, anim: 1, shadow: true, font: 0, dx: 0.0, dy: 0.0 });
        self.sel = Some(Sel::Title(self.proj.titles.len() - 1));
        self.tab = 0;
    }

    fn snapshot(&mut self) {
        let Some((ci, st)) = self.proj.clip_at(self.t) else {
            self.error = Some("Aucune image à la tête de lecture".into());
            return;
        };
        let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name("image.png").save_file() else { return };
        let clip = self.proj.clips[ci].clone();
        let src = self.proj.sources[clip.src].clone();
        let (w, h) = (self.proj.width, self.proj.height);
        let titles: Vec<Title> = self.proj.titles_at(self.t).into_iter().cloned().collect();
        let fade = self.proj.fade_at(self.t);
        let progress = self.proj.local_at(self.t).map_or(0.0, |(_, l)| (l / clip.len().max(1e-6)) as f32);
        let (bg, t) = (self.proj.background, self.t);
        let r = (|| -> Result<(), String> {
            let (mut rd, _) = VideoReader::open(Path::new(&src.path))?;
            let (sw, sh) = (rd.width, rd.height);
            let Some((_, rgba)) = rd.frame_at(st, src.fps)? else { return Err("image introuvable".into()) };
            let refs: Vec<&Title> = titles.iter().collect();
            let frame = look::compose(rgba, sw, sh, w, h, &look::Frame { clip: Some(&clip), fade, progress, background: bg, titles: &refs, t });
            image::save_buffer(&path, &frame, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())
        })();
        if let Err(e) = r {
            self.error = Some(format!("Image : {e}"));
        }
    }

    fn select_tool(&mut self, t: Tool) {
        self.tool = t;
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
            self.export = Some(ExportOpts { res: 0, quality: 1, range: 0, audio_only: false });
        }
        if sc(M::COMMAND | M::SHIFT, Key::F) {
            self.freeze_frame();
        }
        if sc(M::COMMAND, Key::D) {
            self.duplicate();
        }
        if sc(M::COMMAND, Key::K) {
            self.proj.split(self.t);
        }
        if sc(M::COMMAND, Key::C)
            && let Some(Sel::Clip(i)) = self.sel
        {
            self.clip_board = self.proj.clips.get(i).cloned();
        }
        if sc(M::COMMAND, Key::X)
            && let Some(Sel::Clip(i)) = self.sel
        {
            self.clip_board = self.proj.clips.get(i).cloned();
            self.delete_sel();
        }
        if sc(M::COMMAND, Key::V) {
            self.paste_clip();
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
        let fps = self.proj.fps as f64;
        let k = ctx.input(|i| {
            let plain = !i.modifiers.command && !i.modifiers.alt;
            let p = |k: Key| i.key_pressed(k) && plain;
            (
                i.key_pressed(Key::Space),
                p(Key::S),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::ArrowLeft) && !i.modifiers.alt,
                i.key_pressed(Key::ArrowRight) && !i.modifiers.alt,
                i.key_pressed(Key::Home),
                i.key_pressed(Key::End),
                i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals),
                i.key_pressed(Key::Minus),
                i.modifiers.shift,
                [p(Key::V), p(Key::C), p(Key::Y), p(Key::R), p(Key::H), p(Key::Z)],
                [p(Key::J), p(Key::K), p(Key::L), p(Key::M), p(Key::T), p(Key::I), p(Key::O), p(Key::N), p(Key::F), p(Key::Backslash)],
                if p(Key::Q) { 1 } else if p(Key::W) { 2 } else { 0 },
                if i.key_pressed(Key::ArrowUp) { -1 } else if i.key_pressed(Key::ArrowDown) { 1 } else { 0 },
            )
        });
        let (space, s, del, left, right, home, end, plus, minus, shift, tools, keys, trim, jump) = k;
        let step = if shift { 1.0 } else { 1.0 / fps };
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
        if plus {
            self.pps = (self.pps * 1.25).min(600.0);
        }
        if minus {
            self.pps = (self.pps / 1.25).max(2.0);
        }
        for (pressed, (tool, _, _)) in tools.iter().zip(TOOLS) {
            if *pressed {
                self.select_tool(tool);
            }
        }
        let [j, kk, l, m, t, i, o, n, f, fit] = keys;
        if j {
            self.seek(self.t - 1.0);
        }
        if kk {
            self.set_playing(false);
        }
        if l {
            self.set_playing(true);
        }
        if m {
            self.add_marker();
        }
        if t {
            self.add_title();
        }
        if i {
            self.mark_in = Some(self.t);
        }
        if o {
            self.mark_out = Some(self.t);
        }
        if n {
            self.snap = !self.snap;
        }
        if f {
            self.focus = !self.focus;
        }
        if fit {
            self.pps = 0.0;
        }
        if trim > 0 {
            self.proj.trim_to(self.t, trim == 1);
        }
        if jump != 0 {
            self.jump(jump);
        }
        if matches!(self.sel, Some(Sel::Clip(i)) if i >= self.proj.clips.len()) || matches!(self.sel, Some(Sel::Music(i)) if i >= self.proj.music.len()) || matches!(self.sel, Some(Sel::Title(i)) if i >= self.proj.titles.len()) {
            self.sel = None;
        }
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
            let pos = self.audio.state.lock().map(|s| s.pos).unwrap_or(0.0);
            self.t = if self.audio._stream.is_some() { pos / OUT_RATE as f64 } else { self.t + ctx.input(|i| i.stable_dt) as f64 };
            let end = if self.looping { self.mark_out.unwrap_or(self.proj.duration()) } else { self.proj.duration() };
            if self.t >= end {
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
            let right_w = if self.home { 0.0 } else { 300.0 };
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
                        ui.add(egui::ProgressBar::new(d as f32 / j.total.max(1) as f32).desired_width(140.0).corner_radius(CornerRadius::same(6)).text(format!("Export {:.0} %", d as f32 * 100.0 / j.total.max(1) as f32)));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
                    } else if pro::primary_icon(ui, "Exporter", |p, r, c| glyph(p, r, G::Export, c)).on_hover_text("Ctrl+M").clicked() {
                        self.export = Some(ExportOpts { res: 0, quality: 1, range: if self.mark_in.is_some() || self.mark_out.is_some() { 1 } else { 0 }, audio_only: false });
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
                if ui.button("Enregistrer sous…             Ctrl+Maj+S").clicked() {
                    self.save(true);
                }
                ui.separator();
                if ui.button("Exporter…                          Ctrl+M").clicked() {
                    self.export = Some(ExportOpts { res: 0, quality: 1, range: 0, audio_only: false });
                }
                if ui.button("Exporter l'image affichée (PNG)…").clicked() {
                    self.snapshot();
                }
            });
        });
        ui.add_enabled_ui(!self.home, |ui| {
            ui.menu_button("Édition", |ui| {
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler                         Ctrl+Z")).clicked()
                    && let Some(p) = self.undo.pop()
                {
                    self.redo.push(std::mem::replace(&mut self.proj, p));
                }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir                        Ctrl+Y")).clicked()
                    && let Some(p) = self.redo.pop()
                {
                    self.undo.push(std::mem::replace(&mut self.proj, p));
                }
                ui.separator();
                let is_clip = matches!(self.sel, Some(Sel::Clip(_)));
                if ui.add_enabled(is_clip, egui::Button::new("Couper le clip                 Ctrl+X")).clicked()
                    && let Some(Sel::Clip(i)) = self.sel
                {
                    self.clip_board = self.proj.clips.get(i).cloned();
                    self.delete_sel();
                }
                if ui.add_enabled(is_clip, egui::Button::new("Copier le clip                 Ctrl+C")).clicked()
                    && let Some(Sel::Clip(i)) = self.sel
                {
                    self.clip_board = self.proj.clips.get(i).cloned();
                }
                if ui.add_enabled(self.clip_board.is_some(), egui::Button::new("Coller à la tête de lecture  Ctrl+V")).clicked() {
                    self.paste_clip();
                }
                if ui.add_enabled(self.sel.is_some(), egui::Button::new("Dupliquer                      Ctrl+D")).clicked() {
                    self.duplicate();
                }
                if ui.add_enabled(self.sel.is_some(), egui::Button::new("Supprimer                       Suppr")).clicked() {
                    self.delete_sel();
                }
                ui.separator();
                if ui.button("Scinder à la tête de lecture    S").clicked() {
                    self.proj.split(self.t);
                }
                if ui.button("Rogner le début à la tête       Q").clicked() {
                    self.proj.trim_to(self.t, true);
                }
                if ui.button("Rogner la fin à la tête          W").clicked() {
                    self.proj.trim_to(self.t, false);
                }
                if ui.button("Figer l'image (2 s)            Ctrl+Maj+F").clicked() {
                    self.freeze_frame();
                }
                if ui.add_enabled(is_clip, egui::Button::new("Déplacer le clip à gauche   Alt+←")).clicked() {
                    self.nudge_clip(-1);
                }
                if ui.add_enabled(is_clip, egui::Button::new("Déplacer le clip à droite    Alt+→")).clicked() {
                    self.nudge_clip(1);
                }
            });
            ui.menu_button("Marqueurs", |ui| {
                if ui.button("Ajouter un marqueur              M").clicked() {
                    self.add_marker();
                }
                if ui.button("Point d'entrée                    I").clicked() {
                    self.mark_in = Some(self.t);
                }
                if ui.button("Point de sortie                   O").clicked() {
                    self.mark_out = Some(self.t);
                }
                if ui.button("Effacer entrée et sortie       Alt+X").clicked() {
                    self.mark_in = None;
                    self.mark_out = None;
                }
                ui.separator();
                if ui.button("Point de montage précédent     ↑").clicked() {
                    self.jump(-1);
                }
                if ui.button("Point de montage suivant        ↓").clicked() {
                    self.jump(1);
                }
                if ui.add_enabled(!self.proj.markers.is_empty(), egui::Button::new("Supprimer tous les marqueurs")).clicked() {
                    self.proj.markers.clear();
                }
            });
            ui.menu_button("Titre", |ui| {
                if ui.button("Nouveau titre                      T").clicked() {
                    self.add_title();
                }
                for (label, place, size, band) in [("Titre centré", 1u8, 0.1f32, false), ("Sous-titre en bas", 2, 0.055, false), ("Tiers inférieur", 3, 0.05, true), ("Titre en haut", 0, 0.07, false)] {
                    if ui.button(label).clicked() {
                        self.add_title();
                        if let Some(Sel::Title(i)) = self.sel {
                            let t = &mut self.proj.titles[i];
                            (t.place, t.size, t.band) = (place, size, band);
                        }
                    }
                }
            });
            ui.menu_button("Affichage", |ui| {
                ui.checkbox(&mut self.snap, "Magnétisme                         N");
                ui.checkbox(&mut self.looping, "Lecture en boucle");
                ui.checkbox(&mut self.safe, "Zones de sécurité");
                ui.checkbox(&mut self.focus, "Mode visionneuse                  F");
                ui.separator();
                ui.checkbox(&mut self.show_thumbs, "Vignettes des clips");
                ui.checkbox(&mut self.show_waves, "Formes d'onde");
                ui.horizontal(|ui| {
                    ui.label("Hauteur des pistes");
                    for (label, k) in [("S", 0.75), ("M", 1.0), ("L", 1.4)] {
                        if ui.selectable_label((self.track_scale - k).abs() < 0.01, label).clicked() {
                            self.track_scale = k;
                        }
                    }
                });
                if ui.button("Ajuster la timeline               \\").clicked() {
                    self.pps = 0.0;
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
                    let sel = match self.sel {
                        Some(Sel::Clip(i)) => self.proj.clips.get(i).map(|c| format!("Clip · {} · {}", short(&self.proj.sources[c.src].name(), 26), tc(c.len(), self.proj.fps))),
                        Some(Sel::Music(i)) => self.proj.music.get(i).map(|m| format!("Musique · {}", short(&self.proj.sources[m.src].name(), 26))),
                        Some(Sel::Title(i)) => self.proj.titles.get(i).map(|t| format!("Titre · {}", short(t.text.lines().next().unwrap_or(""), 26))),
                        None => None,
                    };
                    ui.label(small(sel.unwrap_or(format!("{} clip(s) · {} titre(s) · {}", self.proj.clips.len(), self.proj.titles.len(), tc(self.proj.duration(), self.proj.fps)))));
                    if let (Some(a), Some(b)) = (self.mark_in, self.mark_out) {
                        ui.label(RichText::new(format!("Entrée/sortie {}", tc((b - a).abs(), self.proj.fps))).size(12.0).color(MARK));
                    }
                }
                if let Some(j) = &self.job
                    && j.done.load(Ordering::Relaxed) == usize::MAX
                {
                    match j.error.lock().ok().and_then(|e| e.clone()) {
                        Some(e) => ui.label(RichText::new(format!("Export : {e}")).size(12.0).color(RED)),
                        None => ui.label(RichText::new(format!("Exporté : {}", j.path.display())).size(12.0).color(GREEN)),
                    };
                }
                if let Some(e) = self.error.clone() {
                    ui.label(RichText::new(e).size(12.0).color(RED));
                    if ui.small_button("×").clicked() {
                        self.error = None;
                    }
                }
                if !self.home {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(small(if self.snap { "Magnétisme activé".into() } else { "Magnétisme désactivé".into() }));
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
                        ui.label(RichText::new("Monte, coupe, étalonne et exporte tes vidéos en MP4").color(MUTED));
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
        egui::Panel::left("bin").resizable(false).exact_size(272.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 12))).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Médias").font(semibold(13.5)).color(INK_STRONG));
                ui.label(RichText::new(self.proj.sources.len().to_string()).size(12.0).color(FAINT));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::secondary_icon(ui, "", |p, r, c| glyph(p, r, G::Plus, c)).on_hover_text("Importer des médias (Ctrl+I)").clicked() {
                        self.import_dialog(ctx);
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.bin_query).hint_text("Rechercher").desired_width(ui.available_width() - 84.0));
                pro::segmented(ui, &mut self.bin_list, &["▦", "☰"]);
            });
            ui.add_space(8.0);
            if self.proj.sources.is_empty() {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 120.0), Sense::hover());
                ui.painter().rect(r, CornerRadius::same(10), Color32::from_white_alpha(3), Stroke::new(1.0, LINE), StrokeKind::Inside);
                ui.painter().text(r.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, "Glisse tes fichiers ici", semibold(13.0), INK);
                ui.painter().text(r.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, "Vidéos et musiques", FontId::proportional(11.5), FAINT);
                return;
            }
            let q = self.bin_query.to_lowercase();
            let items: Vec<usize> = (0..self.proj.sources.len()).filter(|i| q.is_empty() || self.proj.sources[*i].name().to_lowercase().contains(&q)).collect();
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
                        ui.painter().text(pic.right_top() + vec2(10.0, 2.0), Align2::LEFT_TOP, short(&s.name(), 22), FontId::proportional(12.5), INK);
                        ui.painter().text(pic.right_bottom() + vec2(10.0, -2.0), Align2::LEFT_BOTTOM, format!("{} · {}", if s.has_video { format!("{}×{}", s.width, s.height) } else { "Audio".into() }, tc(s.duration, 30)), FontId::proportional(11.0), FAINT);
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
                                let p = ui.painter();
                                let tag = Rect::from_min_size(pic.right_bottom() - vec2(50.0, 21.0), vec2(46.0, 17.0));
                                p.rect_filled(tag, CornerRadius::same(5), Color32::from_black_alpha(170));
                                p.text(tag.center(), Align2::CENTER_CENTER, tc(s.duration, 30).chars().take(5).collect::<String>(), FontId::proportional(10.5), Color32::WHITE);
                                if resp.hovered() {
                                    p.rect_stroke(pic, CornerRadius::same(8), Stroke::new(1.5, BLUE), StrokeKind::Inside);
                                    let plus = Rect::from_min_size(pic.right_top() + vec2(-26.0, 4.0), Vec2::splat(22.0));
                                    p.circle_filled(plus.center(), 11.0, BLUE);
                                    glyph(p, plus.shrink(5.0), G::Plus, Color32::WHITE);
                                }
                                p.text(pos2(r.left() + 2.0, pic.bottom() + 8.0), Align2::LEFT_TOP, short(&s.name(), (w / 7.2) as usize), FontId::proportional(12.0), INK);
                                p.text(pos2(r.left() + 2.0, pic.bottom() + 23.0), Align2::LEFT_TOP, if s.has_video { format!("{} × {}", s.width, s.height) } else { "Audio".into() }, FontId::proportional(10.5), FAINT);
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
                _ => {}
            }
        });
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

    fn inspector(&mut self, root: &mut Ui) {
        egui::Panel::right("inspector").resizable(false).exact_size(296.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            let w = ui.available_width();
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.set_max_width(w);
                ui.spacing_mut().item_spacing.y = 0.0;
                match self.sel {
                    Some(Sel::Clip(i)) if i < self.proj.clips.len() => self.clip_inspector(ui, i),
                    Some(Sel::Music(i)) if i < self.proj.music.len() => self.music_inspector(ui, i),
                    Some(Sel::Title(i)) if i < self.proj.titles.len() => self.title_inspector(ui, i),
                    _ => self.sequence_inspector(ui),
                }
            });
        });
    }

    fn clip_inspector(&mut self, ui: &mut Ui, i: usize) {
        let src = self.proj.sources[self.proj.clips[i].src].clone();
        Frame::new().inner_margin(Margin::symmetric(12, 10)).show(ui, |ui| {
            ui.horizontal(|ui| {
                let c = LABELS[self.proj.clips[i].label as usize % LABELS.len()].1;
                let (r, _) = ui.allocate_exact_size(vec2(4.0, 30.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(2), Color32::from_rgb(c[0], c[1], c[2]));
                ui.vertical(|ui| {
                    ui.label(RichText::new(short(&src.name(), 30)).font(semibold(13.5)).color(INK_STRONG));
                    ui.label(RichText::new(format!("{} × {} · {:.0} i/s · {}", src.width, src.height, src.fps, tc(self.proj.clips[i].len(), self.proj.fps))).size(11.5).color(FAINT));
                });
            });
            ui.add_space(8.0);
            let mut tab = self.tab;
            if pro::segmented(ui, &mut tab, &["Clip", "Image", "Couleur", "Audio"]) {
                self.tab = tab;
            }
        });
        let fps = self.proj.fps as f64;
        let c = &mut self.proj.clips[i];
        let mut reset = false;
        match self.tab {
            0 => {
                pro::dock_panel(ui, "clip-time", "Durée et vitesse", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    field_time(ui, "Entrée", &mut c.inp, 0.0, c.out - 0.05);
                    field_time(ui, "Sortie", &mut c.out, c.inp + 0.05, src.duration);
                    row_slider(ui, "Vitesse", &mut c.speed, 0.25..=4.0, " ×", true);
                    ui.horizontal_wrapped(|ui| {
                        for v in [0.25, 0.5, 1.0, 1.5, 2.0, 4.0] {
                            if ui.selectable_label((c.speed - v).abs() < 1e-3, format!("{v}×")).clicked() {
                                c.speed = v;
                            }
                        }
                    });
                    let mut frozen = c.hold > 0.0;
                    if ui.checkbox(&mut frozen, "Image figée").changed() {
                        c.hold = if frozen { 2.0 } else { 0.0 };
                        if frozen {
                            c.out = (c.inp + 1.0 / fps).min(src.duration);
                        }
                    }
                    if c.hold > 0.0 {
                        field_time(ui, "Durée figée", &mut c.hold, 0.1, 60.0);
                    }
                });
                pro::dock_panel(ui, "clip-trans", "Transitions", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let len = c.len();
                    row_slider(ui, "Fondu d'entrée", &mut c.fade_in, 0.0..=len.min(5.0), " s", false);
                    row_slider(ui, "Fondu de sortie", &mut c.fade_out, 0.0..=len.min(5.0), " s", false);
                    let mut white = c.fade_white as usize;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Fondu vers").size(12.5).color(MUTED));
                        if pro::segmented(ui, &mut white, &["Noir", "Blanc"]) {
                            c.fade_white = white == 1;
                        }
                    });
                });
                pro::dock_panel(ui, "clip-label", "Étiquette", |ui| {
                    ui.horizontal(|ui| {
                        for (k, (name, rgb)) in LABELS.iter().enumerate() {
                            let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
                            ui.painter().circle_filled(r.center(), 8.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                            if c.label as usize == k {
                                ui.painter().circle_stroke(r.center(), 10.5, Stroke::new(1.5, Color32::WHITE));
                            }
                            if resp.on_hover_text(*name).clicked() {
                                c.label = k as u8;
                            }
                        }
                    });
                });
            }
            1 => {
                pro::dock_panel(ui, "clip-frame", "Cadrage", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    pct_slider(ui, "Échelle", &mut c.zoom, 0.1..=4.0);
                    let mut kb = c.zoom_end > 0.0;
                    if ui.checkbox(&mut kb, "Zoom progressif (Ken Burns)").changed() {
                        c.zoom_end = if kb { c.zoom * 1.15 } else { 0.0 };
                    }
                    if c.zoom_end > 0.0 {
                        pct_slider(ui, "Échelle finale", &mut c.zoom_end, 0.1..=4.0);
                    }
                    row_slider(ui, "Position X", &mut c.pos_x, -1.0..=1.0, "", false);
                    row_slider(ui, "Position Y", &mut c.pos_y, -1.0..=1.0, "", false);
                    row_slider(ui, "Rotation", &mut c.rotation, -180.0..=180.0, "°", false);
                    ui.horizontal(|ui| {
                        if ui.small_button("↺ 90°").clicked() {
                            c.rotation = (c.rotation - 90.0 + 540.0) % 360.0 - 180.0;
                        }
                        if ui.small_button("↻ 90°").clicked() {
                            c.rotation = (c.rotation + 90.0 + 540.0) % 360.0 - 180.0;
                        }
                        ui.checkbox(&mut c.flip, "Miroir");
                    });
                    pct_slider(ui, "Opacité", &mut c.opacity, 0.0..=1.0);
                });
                pro::dock_panel(ui, "clip-crop", "Recadrage", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    for (k, label) in ["Gauche", "Haut", "Droite", "Bas"].into_iter().enumerate() {
                        pct_slider(ui, label, &mut c.crop[k], 0.0..=0.45);
                    }
                });
                pro::dock_panel(ui, "clip-fx", "Effets", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    row_slider(ui, "Flou", &mut c.blur, 0.0..=100.0, "", false);
                    row_slider(ui, "Vignettage", &mut c.vignette, -100.0..=100.0, "", false);
                    ui.add_space(4.0);
                    if pro::secondary(ui, "Réinitialiser l'image").clicked() {
                        reset = true;
                    }
                });
            }
            2 => {
                pro::dock_panel(ui, "clip-look", "Look", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (k, name) in LOOKS.iter().enumerate() {
                            if ui.selectable_label(c.look as usize == k, *name).clicked() {
                                c.look = k as u8;
                            }
                        }
                    });
                });
                pro::dock_panel(ui, "clip-grade", "Correction", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    for (label, v) in [("Luminosité", &mut c.brightness), ("Contraste", &mut c.contrast), ("Saturation", &mut c.saturation), ("Température", &mut c.temperature), ("Teinte", &mut c.tint)] {
                        row_slider(ui, label, v, -100.0..=100.0, "", false);
                    }
                    ui.add_space(4.0);
                    if pro::secondary(ui, "Réinitialiser la couleur").clicked() {
                        (c.brightness, c.contrast, c.saturation, c.temperature, c.tint, c.look) = (0.0, 0.0, 0.0, 0.0, 0.0, 0);
                    }
                });
            }
            _ => {
                pro::dock_panel(ui, "clip-audio", "Son du clip", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.checkbox(&mut c.mute, "Couper le son du clip");
                    let mut db = 20.0 * c.volume.max(1e-4).log10();
                    if row_slider(ui, "Volume", &mut db, -40.0..=12.0, " dB", false) {
                        c.volume = 10f32.powf(db / 20.0);
                    }
                    row_slider(ui, "Balance", &mut c.pan, -1.0..=1.0, "", false);
                    if c.hold > 0.0 {
                        ui.label(RichText::new("Une image figée n'a pas de son.").size(12.0).color(FAINT));
                    }
                });
            }
        }
        if reset {
            c.reset_look();
        }
    }

    fn music_inspector(&mut self, ui: &mut Ui, i: usize) {
        let src = self.proj.sources[self.proj.music[i].src].clone();
        let m = &mut self.proj.music[i];
        pro::dock_panel(ui, "music", "Musique", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.label(RichText::new(short(&src.name(), 30)).font(semibold(13.5)).color(INK));
            field_time(ui, "Début", &mut m.start, 0.0, 36000.0);
            field_time(ui, "Entrée", &mut m.inp, 0.0, m.out - 0.05);
            field_time(ui, "Sortie", &mut m.out, m.inp + 0.05, src.duration);
        });
        pro::dock_panel(ui, "music-mix", "Mixage", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.checkbox(&mut m.mute, "Couper");
            let mut db = 20.0 * m.volume.max(1e-4).log10();
            if row_slider(ui, "Volume", &mut db, -40.0..=12.0, " dB", false) {
                m.volume = 10f32.powf(db / 20.0);
            }
            row_slider(ui, "Balance", &mut m.pan, -1.0..=1.0, "", false);
            let len = m.out - m.inp;
            row_slider(ui, "Fondu d'entrée", &mut m.fade_in, 0.0..=len.min(10.0), " s", false);
            row_slider(ui, "Fondu de sortie", &mut m.fade_out, 0.0..=len.min(10.0), " s", false);
        });
    }

    fn title_inspector(&mut self, ui: &mut Ui, i: usize) {
        let t = &mut self.proj.titles[i];
        pro::dock_panel(ui, "title-text", "Texte", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.add(egui::TextEdit::multiline(&mut t.text).desired_rows(3).desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                ui.label(RichText::new("Police").size(12.5).color(MUTED));
                egui::ComboBox::from_id_salt("font").selected_text(FONTS[t.font as usize % FONTS.len()].0).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                    for (k, (name, _)) in FONTS.iter().enumerate() {
                        ui.selectable_value(&mut t.font, k as u8, *name);
                    }
                });
            });
            pct_slider(ui, "Taille", &mut t.size, 0.02..=0.3);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Couleur").size(12.5).color(MUTED));
                let mut col = Color32::from_rgba_unmultiplied(t.color[0], t.color[1], t.color[2], t.color[3]);
                if ui.color_edit_button_srgba(&mut col).changed() {
                    t.color = col.to_srgba_unmultiplied();
                }
                ui.checkbox(&mut t.shadow, "Ombre");
                ui.checkbox(&mut t.band, "Bandeau");
            });
        });
        pro::dock_panel(ui, "title-place", "Position et durée", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let mut place = t.place as usize;
            if pro::segmented(ui, &mut place, &["Haut", "Centre", "Bas", "Tiers"]) {
                t.place = place as u8;
            }
            row_slider(ui, "Décalage X", &mut t.dx, -0.5..=0.5, "", false);
            row_slider(ui, "Décalage Y", &mut t.dy, -0.5..=0.5, "", false);
            field_time(ui, "Début", &mut t.start, 0.0, 36000.0);
            field_time(ui, "Durée", &mut t.dur, 0.2, 600.0);
        });
        pro::dock_panel(ui, "title-anim", "Animation", |ui| {
            ui.horizontal_wrapped(|ui| {
                for (k, name) in ANIMS.iter().enumerate() {
                    if ui.selectable_label(t.anim as usize == k, *name).clicked() {
                        t.anim = k as u8;
                    }
                }
            });
            ui.label(RichText::new(PLACES.join(" · ")).size(1.0).color(Color32::TRANSPARENT));
        });
    }

    fn sequence_inspector(&mut self, ui: &mut Ui) {
        pro::dock_panel(ui, "sequence", "Séquence", |ui| {
            ui.spacing_mut().item_spacing.y = 5.0;
            pro::label_value(ui, "Format", &format!("{} × {}", self.proj.width, self.proj.height));
            pro::label_value(ui, "Images/s", &self.proj.fps.to_string());
            pro::label_value(ui, "Durée", &tc(self.proj.duration(), self.proj.fps));
            pro::label_value(ui, "Clips · titres · musiques", &format!("{} · {} · {}", self.proj.clips.len(), self.proj.titles.len(), self.proj.music.len()));
            ui.add_space(6.0);
            if pro::secondary(ui, "Réglages de la séquence…").clicked() {
                self.seq_sheet = true;
            }
        });
        pro::dock_panel(ui, "quick", "Ajouter", |ui| {
            ui.horizontal_wrapped(|ui| {
                if pro::secondary_icon(ui, "Titre", |p, r, c| glyph(p, r, G::Title, c)).clicked() {
                    self.add_title();
                }
                if pro::secondary_icon(ui, "Marqueur", |p, r, c| glyph(p, r, G::Marker, c)).clicked() {
                    self.add_marker();
                }
            });
        });
        if !self.proj.markers.is_empty() {
            let fps = self.proj.fps;
            let mut go = None;
            pro::dock_panel(ui, "markers", "Marqueurs", |ui| {
                for (k, m) in self.proj.markers.iter().enumerate() {
                    if pro::ghost(ui, &format!("◆  {}   {}", k + 1, tc(*m, fps))).clicked() {
                        go = Some(*m);
                    }
                }
            });
            if let Some(m) = go {
                self.seek(m);
            }
        }
        pro::dock_panel(ui, "shortcuts", "Raccourcis", |ui| {
            for (k, d) in [("Espace", "Lecture"), ("J K L", "Reculer · pause · lire"), ("S", "Scinder"), ("Q / W", "Rogner début / fin"), ("I / O", "Entrée / sortie"), ("M", "Marqueur"), ("T", "Titre"), ("↑ ↓", "Point de montage"), ("N", "Magnétisme"), ("F", "Mode visionneuse")] {
                ui.horizontal(|ui| {
                    pro::kbd(ui, k);
                    ui.label(RichText::new(d).size(12.0).color(MUTED));
                });
            }
            ui.add_space(4.0);
            if pro::ghost(ui, "Tous les tutoriels  →").clicked() {
                self.help.learn("Lecture et navigation");
            }
        });
    }

    fn viewer(&mut self, root: &mut Ui) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let head = Rect::from_min_size(full.min, vec2(full.width(), 34.0));
            ui.painter().text(head.left_center() + vec2(16.0, 0.0), Align2::LEFT_CENTER, "Programme", semibold(12.5), INK);
            if let Some((i, _)) = self.proj.clip_at(self.t) {
                let src = &self.proj.sources[self.proj.clips[i].src];
                ui.painter().text(head.left_center() + vec2(100.0, 0.0), Align2::LEFT_CENTER, short(&src.name(), 40), FontId::proportional(12.0), FAINT);
            }
            let controls_h = 62.0;
            let screen = Rect::from_min_max(pos2(full.left() + 16.0, head.bottom()), pos2(full.right() - 16.0, full.bottom() - controls_h));
            let (w, h) = (self.proj.width as f32, self.proj.height as f32);
            let s = (screen.width() / w).min(screen.height() / h);
            let frame = Rect::from_center_size(screen.center(), vec2(w, h) * s);
            let p = ui.painter();
            p.add(egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(150) }.as_shape(frame, CornerRadius::same(6)));
            p.rect_filled(frame, CornerRadius::same(6), Color32::BLACK);
            if let (Some(t), Some(_)) = (&self.tex, self.shown) {
                p.image(t.id(), frame, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            } else if self.proj.clips.is_empty() {
                p.text(frame.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "Importe une vidéo pour commencer", semibold(16.0), INK);
                p.text(frame.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "Glisse un fichier ici ou Ctrl+I", FontId::proportional(12.5), MUTED);
            }
            if self.safe {
                for k in [0.9, 0.8] {
                    p.rect_stroke(Rect::from_center_size(frame.center(), frame.size() * k), CornerRadius::ZERO, Stroke::new(1.0, Color32::from_white_alpha(70)), StrokeKind::Middle);
                }
                p.line_segment([frame.center() - vec2(8.0, 0.0), frame.center() + vec2(8.0, 0.0)], Stroke::new(1.0, Color32::from_white_alpha(90)));
                p.line_segment([frame.center() - vec2(0.0, 8.0), frame.center() + vec2(0.0, 8.0)], Stroke::new(1.0, Color32::from_white_alpha(90)));
            }

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
                        2 => self.set_playing(!self.playing),
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
                    ui.spacing_mut().slider_width = 70.0;
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

    fn timeline(&mut self, root: &mut Ui) {
        egui::Panel::bottom("timeline").resizable(true).default_size(320.0).min_size(250.0).frame(Frame::new().fill(PANEL).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let head_h = 42.0;
            let full = ui.max_rect();
            let head = Rect::from_min_size(full.min, vec2(full.width(), head_h));
            ui.painter().rect_filled(head, CornerRadius::ZERO, CHROME);
            ui.painter().line_segment([head.left_bottom(), head.right_bottom()], Stroke::new(1.0, LINE));
            ui.scope_builder(egui::UiBuilder::new().max_rect(head.shrink2(vec2(10.0, 6.0))), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (tool, tip, g) in TOOLS {
                        if pro::icon_button(ui, 30.0, self.tool == tool, tip, |p, r, c| glyph(p, r, g, c)).clicked() {
                            self.select_tool(tool);
                        }
                    }
                    pro::toolbar_sep(ui);
                    let actions: [(G, &str, u8); 5] = [(G::Split, "Scinder à la tête de lecture (S)", 0), (G::Trash, "Supprimer la sélection (Suppr)", 1), (G::Freeze, "Figer l'image 2 s (Ctrl+Maj+F)", 2), (G::Title, "Ajouter un titre (T)", 3), (G::Marker, "Ajouter un marqueur (M)", 4)];
                    for (g, tip, k) in actions {
                        if pro::icon_button(ui, 30.0, false, tip, |p, r, c| glyph(p, r, g, c)).clicked() {
                            match k {
                                0 => {
                                    self.proj.split(self.t);
                                }
                                1 => self.delete_sel(),
                                2 => self.freeze_frame(),
                                3 => self.add_title(),
                                _ => self.add_marker(),
                            }
                        }
                    }
                    pro::toolbar_sep(ui);
                    if pro::icon_button(ui, 30.0, self.snap, "Magnétisme (N)", |p, r, c| glyph(p, r, G::Magnet, c)).clicked() {
                        self.snap = !self.snap;
                    }
                    if pro::icon_button(ui, 30.0, self.show_thumbs, "Vignettes", |p, r, c| glyph(p, r, G::Film, c)).clicked() {
                        self.show_thumbs = !self.show_thumbs;
                    }
                    if pro::icon_button(ui, 30.0, self.show_waves, "Formes d'onde", |p, r, c| glyph(p, r, G::Wave, c)).clicked() {
                        self.show_waves = !self.show_waves;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if pro::icon_button(ui, 28.0, false, "Ajuster à la fenêtre (\\)", |p, r, c| glyph(p, r, G::Fit, c)).clicked() {
                            self.pps = 0.0;
                        }
                        if pro::icon_button(ui, 28.0, false, "Zoom avant (+)", |p, r, c| glyph(p, r, G::Plus, c)).clicked() {
                            self.pps = (self.pps * 1.4).min(600.0);
                        }
                        ui.spacing_mut().slider_width = 120.0;
                        ui.add(egui::Slider::new(&mut self.pps, 2.0..=600.0).logarithmic(true).show_value(false));
                        if pro::icon_button(ui, 28.0, false, "Zoom arrière (−)", |p, r, c| glyph(p, r, G::Minus, c)).clicked() {
                            self.pps = (self.pps / 1.4).max(2.0);
                        }
                    });
                });
            });

            let label_w = 148.0;
            let body = Rect::from_min_max(pos2(full.left(), head.bottom()), full.max);
            if self.pps <= 0.0 {
                self.pps = ((body.width() - label_w - 40.0) / self.proj.duration().max(5.0) as f32).clamp(2.0, 600.0);
                self.force_scroll = Some(0.0);
            }
            let dur = self.proj.duration().max(10.0) + 8.0;
            let ts = self.track_scale;
            let mut sa = egui::ScrollArea::horizontal().auto_shrink(false);
            if let Some(x) = self.force_scroll.take() {
                sa = sa.horizontal_scroll_offset(x.max(0.0));
            }
            let out = ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| sa.show(ui, |ui| {
                let width = (dur as f32 * self.pps).max(ui.available_width() - label_w);
                let (rect, _) = ui.allocate_exact_size(vec2(width + label_w, ui.available_height().max(170.0)), Sense::hover());
                let x0 = rect.min.x + label_w;
                let pps = self.pps;
                let x_of = |t: f64| x0 + t as f32 * pps;
                let t_of = |x: f32| ((x - x0) / pps).max(0.0) as f64;
                let ruler = Rect::from_min_max(pos2(x0, rect.min.y), pos2(rect.max.x, rect.min.y + 26.0));
                let t1 = Rect::from_min_max(pos2(x0, ruler.max.y + 6.0), pos2(rect.max.x, ruler.max.y + 6.0 + 30.0 * ts));
                let v1 = Rect::from_min_max(pos2(x0, t1.max.y + 6.0), pos2(rect.max.x, t1.max.y + 6.0 + 76.0 * ts));
                let a2 = Rect::from_min_max(pos2(x0, v1.max.y + 6.0), pos2(rect.max.x, v1.max.y + 6.0 + 50.0 * ts));
                let p = ui.painter_at(rect);
                p.rect_filled(ruler, CornerRadius::ZERO, WELL);
                if let (Some(a), Some(b)) = (self.mark_in.or(Some(0.0)), self.mark_out.or(Some(self.proj.duration())))
                    && (self.mark_in.is_some() || self.mark_out.is_some())
                {
                    let r = Rect::from_min_max(pos2(x_of(a.min(b)), ruler.min.y), pos2(x_of(a.max(b)), a2.max.y));
                    p.rect_filled(r, CornerRadius::ZERO, MARK.gamma_multiply(0.06));
                    p.rect_filled(Rect::from_min_max(pos2(r.left(), ruler.bottom() - 4.0), pos2(r.right(), ruler.bottom())), CornerRadius::ZERO, MARK.gamma_multiply(0.7));
                }
                let step = [0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0].into_iter().find(|s| *s as f32 * pps >= 80.0).unwrap_or(600.0);
                let minor = step / 5.0;
                let mut k = 0u64;
                while k as f64 * minor <= dur {
                    let tt = k as f64 * minor;
                    let x = x_of(tt);
                    let major = k % 5 == 0;
                    k += 1;
                    p.line_segment([pos2(x, ruler.max.y - if major { 10.0 } else { 4.0 }), pos2(x, ruler.max.y)], Stroke::new(1.0, if major { MUTED } else { FAINT }));
                    if major {
                        p.text(pos2(x + 4.0, ruler.min.y + 4.0), Align2::LEFT_TOP, tc(tt, self.proj.fps), FontId::monospace(10.0), MUTED);
                    }
                }
                let tracks = [(t1, "T1", "Titres", 2u8), (v1, "V1", "Vidéo", 0), (a2, "A2", "Musique", 1)];
                for (r, id, label, kind) in tracks {
                    p.rect_filled(r, CornerRadius::same(6), Color32::from_rgb(0x19, 0x19, 0x1c));
                    let head_r = Rect::from_min_max(pos2(rect.min.x + 6.0, r.min.y), pos2(x0 - 6.0, r.max.y));
                    p.rect_filled(head_r, CornerRadius::same(6), CHROME);
                    p.text(head_r.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, id, semibold(11.5), MUTED);
                    p.text(head_r.left_center() + vec2(34.0, 0.0), Align2::LEFT_CENTER, label, FontId::proportional(12.0), INK);
                    if kind < 2 {
                        let muted = if kind == 0 { self.proj.mute_v1 } else { self.proj.mute_a2 };
                        let mr = Rect::from_center_size(pos2(head_r.right() - 16.0, head_r.center().y), Vec2::splat(22.0));
                        let mresp = ui.interact(mr, ui.make_persistent_id(("mute", kind)), Sense::click()).on_hover_text(if muted { "Réactiver le son de la piste" } else { "Couper le son de la piste" });
                        if mresp.hovered() {
                            p.rect_filled(mr, CornerRadius::same(5), FIELD);
                        }
                        speaker(&p, mr.center(), muted, if muted { RED } else if mresp.hovered() { INK } else { MUTED });
                        if mresp.clicked() {
                            if kind == 0 {
                                self.proj.mute_v1 = !self.proj.mute_v1;
                            } else {
                                self.proj.mute_a2 = !self.proj.mute_a2;
                            }
                        }
                    }
                }
                let rr = ui.interact(ruler, ui.make_persistent_id("ruler"), Sense::click_and_drag());
                if let Some(pp) = rr.interact_pointer_pos() {
                    let t = t_of(pp.x);
                    let t = if ui.input(|i| i.modifiers.shift) { self.snap_time(t, 8.0 / pps as f64) } else { t };
                    self.seek(t);
                }
                let snap_tol = 8.0 / pps as f64;

                for (i, tt) in self.proj.titles.clone().iter().enumerate() {
                    let r = Rect::from_min_max(pos2(x_of(tt.start) + 1.0, t1.min.y + 3.0), pos2(x_of(tt.start + tt.dur) - 1.0, t1.max.y - 3.0));
                    let on = self.sel == Some(Sel::Title(i));
                    p.rect_filled(r, CornerRadius::same(5), TITLE.gamma_multiply(if on { 0.85 } else { 0.55 }));
                    glyph(&p.with_clip_rect(r), Rect::from_min_size(r.left_center() + vec2(6.0, -6.0), Vec2::splat(12.0)), G::Title, Color32::WHITE);
                    p.with_clip_rect(r.shrink(1.0)).text(r.left_center() + vec2(22.0, 0.0), Align2::LEFT_CENTER, tt.text.lines().next().unwrap_or(""), semibold(11.0), Color32::WHITE);
                    if on {
                        p.rect_stroke(r, CornerRadius::same(5), Stroke::new(2.0, Color32::WHITE), StrokeKind::Inside);
                    }
                    let resp = ui.interact(r, ui.make_persistent_id(("title", i)), Sense::click_and_drag());
                    if resp.clicked() {
                        self.sel = Some(Sel::Title(i));
                    }
                    if resp.drag_started()
                        && let Some(pp) = resp.interact_pointer_pos()
                    {
                        let mode = if r.max.x - pp.x < 8.0 { 2 } else { 0 };
                        self.drag = Some((Sel::Title(i), mode, t_of(pp.x), Clip { out: tt.dur, ..Clip::default() }, tt.start));
                        self.sel = Some(Sel::Title(i));
                    }
                    if resp.hover_pos().is_some_and(|hp| r.max.x - hp.x < 8.0) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Dupliquer").clicked() {
                            self.sel = Some(Sel::Title(i));
                            self.duplicate();
                            ui.close();
                        }
                        if ui.button("Supprimer").clicked() {
                            self.sel = Some(Sel::Title(i));
                            self.delete_sel();
                            ui.close();
                        }
                    });
                }

                let starts = self.proj.starts();
                for (i, c) in self.proj.clips.clone().iter().enumerate() {
                    let r = Rect::from_min_max(pos2(x_of(starts[i]) + 1.0, v1.min.y + 3.0), pos2(x_of(starts[i] + c.len()) - 1.0, v1.max.y - 3.0));
                    let on = self.sel == Some(Sel::Clip(i));
                    let lc = LABELS[c.label as usize % LABELS.len()].1;
                    let col = Color32::from_rgb(lc[0], lc[1], lc[2]);
                    p.rect_filled(r, CornerRadius::same(6), col.gamma_multiply(if on { 0.7 } else { 0.45 }));
                    let clip_p = p.with_clip_rect(r.shrink(1.0));
                    let thumb_h = if self.show_waves && !c.mute { r.height() * 0.62 } else { r.height() };
                    if self.show_thumbs
                        && let Some(t) = self.thumbs.get(&c.src)
                    {
                        let [tw, th] = t.size();
                        let tw_w = thumb_h * tw as f32 / th as f32;
                        let mut x = r.left();
                        while x < r.right() && x < r.left() + tw_w * 8.0 {
                            clip_p.image(t.id(), Rect::from_min_size(pos2(x, r.top()), vec2(tw_w, thumb_h)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::from_white_alpha(if c.hold > 0.0 { 120 } else { 200 }));
                            x += tw_w;
                            if c.hold > 0.0 {
                                break;
                            }
                        }
                    }
                    if self.show_waves
                        && !c.mute
                        && c.hold <= 0.0
                        && let Some(data) = self.audio_data.get(&c.src)
                    {
                        let wave = Rect::from_min_max(pos2(r.left(), r.top() + thumb_h + 2.0), r.max);
                        waveform(&clip_p, wave, data, c.inp, c.out, Color32::from_white_alpha(170));
                    }
                    let mut badge = String::new();
                    if (c.speed - 1.0).abs() > 1e-3 && c.hold <= 0.0 {
                        badge.push_str(&format!("  {:.2}×", c.speed));
                    }
                    if c.hold > 0.0 {
                        badge.push_str("  ❄ figée");
                    }
                    if c.look != 0 || c.brightness != 0.0 || c.contrast != 0.0 || c.saturation != 0.0 {
                        badge.push_str("  fx");
                    }
                    let name_bg = Rect::from_min_size(r.left_top() + vec2(4.0, 4.0), vec2((r.width() - 8.0).min(260.0).max(0.0), 18.0));
                    clip_p.rect_filled(name_bg, CornerRadius::same(4), Color32::from_black_alpha(110));
                    clip_p.text(name_bg.left_center() + vec2(6.0, 0.0), Align2::LEFT_CENTER, format!("{}{badge}", self.proj.sources[c.src].name()), semibold(11.0), Color32::WHITE);
                    if c.fade_in > 0.0 {
                        let fw = (c.fade_in as f32 * pps).min(r.width());
                        clip_p.add(Shape::convex_polygon(vec![r.left_top(), pos2(r.left() + fw, r.top()), r.left_bottom()], Color32::from_black_alpha(90), Stroke::NONE));
                    }
                    if c.fade_out > 0.0 {
                        let fw = (c.fade_out as f32 * pps).min(r.width());
                        clip_p.add(Shape::convex_polygon(vec![r.right_top(), r.right_bottom(), pos2(r.right() - fw, r.top())], Color32::from_black_alpha(90), Stroke::NONE));
                    }
                    p.rect_stroke(r, CornerRadius::same(6), Stroke::new(if on { 2.0 } else { 1.0 }, if on { Color32::WHITE } else { col }), StrokeKind::Inside);
                    let resp = ui.interact(r, ui.make_persistent_id(("clip", i)), Sense::click_and_drag());
                    match self.tool {
                        Tool::Blade => {
                            if let Some(hp) = resp.hover_pos() {
                                p.line_segment([pos2(hp.x, v1.min.y), pos2(hp.x, v1.max.y)], Stroke::new(1.5, Color32::WHITE));
                            }
                            if resp.clicked()
                                && let Some(pp) = resp.interact_pointer_pos()
                            {
                                let t = self.snap_time(t_of(pp.x), snap_tol);
                                self.proj.split(t);
                            }
                        }
                        Tool::Hand | Tool::Zoom => {}
                        _ => {
                            if resp.clicked() {
                                self.sel = Some(Sel::Clip(i));
                            }
                            if resp.drag_started()
                                && let Some(pp) = resp.interact_pointer_pos()
                            {
                                let edge = if pp.x - r.min.x < 8.0 { 1 } else if r.max.x - pp.x < 8.0 { 2 } else { 0 };
                                let mode = match self.tool {
                                    Tool::Slip => 3,
                                    Tool::Rate if edge == 2 => 4,
                                    _ => edge,
                                };
                                self.drag = Some((Sel::Clip(i), mode, t_of(pp.x), c.clone(), starts[i]));
                                self.sel = Some(Sel::Clip(i));
                            }
                            if let Some(hp) = resp.hover_pos() {
                                if self.tool == Tool::Slip {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                                } else if hp.x - r.min.x < 8.0 || r.max.x - hp.x < 8.0 {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                }
                            }
                        }
                    }
                    resp.context_menu(|ui| {
                        self.sel = Some(Sel::Clip(i));
                        if ui.button("Scinder à la tête de lecture").clicked() {
                            self.proj.split(self.t);
                            ui.close();
                        }
                        if ui.button("Dupliquer").clicked() {
                            self.duplicate();
                            ui.close();
                        }
                        if ui.button("Copier").clicked() {
                            self.clip_board = Some(c.clone());
                            ui.close();
                        }
                        if ui.button("Figer l'image à la tête").clicked() {
                            self.freeze_frame();
                            ui.close();
                        }
                        ui.menu_button("Vitesse", |ui| {
                            for v in [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 4.0] {
                                if ui.button(format!("{v}×")).clicked() {
                                    self.proj.clips[i].speed = v;
                                    ui.close();
                                }
                            }
                        });
                        ui.menu_button("Look", |ui| {
                            for (k, name) in LOOKS.iter().enumerate() {
                                if ui.button(*name).clicked() {
                                    self.proj.clips[i].look = k as u8;
                                    ui.close();
                                }
                            }
                        });
                        ui.menu_button("Étiquette", |ui| {
                            for (k, (name, _)) in LABELS.iter().enumerate() {
                                if ui.button(*name).clicked() {
                                    self.proj.clips[i].label = k as u8;
                                    ui.close();
                                }
                            }
                        });
                        let mute = self.proj.clips[i].mute;
                        if ui.button(if mute { "Réactiver le son" } else { "Couper le son" }).clicked() {
                            self.proj.clips[i].mute = !mute;
                            ui.close();
                        }
                        if ui.button("Réinitialiser les effets").clicked() {
                            self.proj.clips[i].reset_look();
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Supprimer").clicked() {
                            self.delete_sel();
                            ui.close();
                        }
                    });
                }
                for (i, m) in self.proj.music.clone().iter().enumerate() {
                    let r = Rect::from_min_max(pos2(x_of(m.start) + 1.0, a2.min.y + 3.0), pos2(x_of(m.start + m.out - m.inp) - 1.0, a2.max.y - 3.0));
                    let on = self.sel == Some(Sel::Music(i));
                    p.rect_filled(r, CornerRadius::same(6), MUSIC.gamma_multiply(if on { 0.7 } else { 0.45 }));
                    let clip_p = p.with_clip_rect(r.shrink(1.0));
                    if self.show_waves
                        && !m.mute
                        && let Some(data) = self.audio_data.get(&m.src)
                    {
                        waveform(&clip_p, r.shrink2(vec2(0.0, 4.0)), data, m.inp, m.out, Color32::from_white_alpha(170));
                    }
                    clip_p.text(r.left_top() + vec2(8.0, 3.0), Align2::LEFT_TOP, format!("{}{}", self.proj.sources[m.src].name(), if m.mute { "  (coupé)" } else { "" }), semibold(11.0), Color32::WHITE);
                    p.rect_stroke(r, CornerRadius::same(6), Stroke::new(if on { 2.0 } else { 1.0 }, if on { Color32::WHITE } else { MUSIC }), StrokeKind::Inside);
                    let resp = ui.interact(r, ui.make_persistent_id(("music", i)), Sense::click_and_drag());
                    if resp.clicked() {
                        self.sel = Some(Sel::Music(i));
                    }
                    if resp.drag_started()
                        && let Some(pp) = resp.interact_pointer_pos()
                    {
                        self.drag = Some((Sel::Music(i), 0, t_of(pp.x), Clip::default(), m.start));
                        self.sel = Some(Sel::Music(i));
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Placer à la tête de lecture").clicked() {
                            self.proj.music[i].start = self.t;
                            ui.close();
                        }
                        let mute = self.proj.music[i].mute;
                        if ui.button(if mute { "Réactiver" } else { "Couper le son" }).clicked() {
                            self.proj.music[i].mute = !mute;
                            ui.close();
                        }
                        if ui.button("Supprimer").clicked() {
                            self.sel = Some(Sel::Music(i));
                            self.delete_sel();
                            ui.close();
                        }
                    });
                }

                if let Some((sel, mode, from, orig, start0)) = self.drag.clone() {
                    let (down, pos) = ui.input(|i| (i.pointer.primary_down(), i.pointer.hover_pos()));
                    if let (true, Some(pp)) = (down, pos) {
                        let d = t_of(pp.x) - from;
                        match sel {
                            Sel::Clip(i) if i < self.proj.clips.len() => {
                                let max = self.proj.sources[orig.src].duration;
                                let snapped_end = self.snap_time(start0 + orig.len() + d, snap_tol);
                                let c = &mut self.proj.clips[i];
                                let sp = orig.speed.max(0.05);
                                match mode {
                                    1 => c.inp = (orig.inp + d * sp).clamp(0.0, c.out - 0.1),
                                    2 if c.hold > 0.0 => c.hold = (snapped_end - start0).max(0.1),
                                    2 => c.out = (orig.inp + (snapped_end - start0) * sp).clamp(c.inp + 0.1, max),
                                    3 => {
                                        let len = orig.out - orig.inp;
                                        let inp = (orig.inp - d * sp).clamp(0.0, (max - len).max(0.0));
                                        (c.inp, c.out) = (inp, inp + len);
                                    }
                                    4 => {
                                        let new_len = (snapped_end - start0).max(0.1);
                                        c.speed = ((orig.out - orig.inp) / new_len).clamp(0.25, 4.0);
                                    }
                                    _ => {}
                                }
                            }
                            Sel::Music(i) if i < self.proj.music.len() => {
                                let start = self.snap_time(start0 + d, snap_tol);
                                self.proj.music[i].start = start.max(0.0);
                            }
                            Sel::Title(i) if i < self.proj.titles.len() => {
                                if mode == 2 {
                                    let end = self.snap_time(start0 + orig.out + d, snap_tol);
                                    self.proj.titles[i].dur = (end - start0).max(0.2);
                                } else {
                                    self.proj.titles[i].start = self.snap_time(start0 + d, snap_tol).max(0.0);
                                }
                            }
                            _ => {}
                        }
                        if mode == 0 && matches!(sel, Sel::Clip(_)) {
                            p.line_segment([pos2(pp.x, v1.min.y), pos2(pp.x, v1.max.y)], Stroke::new(2.0, Color32::WHITE));
                        }
                        if mode == 3 {
                            p.text(pp + vec2(12.0, -18.0), Align2::LEFT_BOTTOM, "Glissement", semibold(11.0), Color32::WHITE);
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

                for m in &self.proj.markers {
                    let mx = x_of(*m);
                    p.add(Shape::convex_polygon(vec![pos2(mx - 5.0, ruler.min.y + 14.0), pos2(mx + 5.0, ruler.min.y + 14.0), pos2(mx, ruler.max.y)], MARK, Stroke::NONE));
                    p.line_segment([pos2(mx, ruler.max.y), pos2(mx, a2.max.y)], Stroke::new(1.0, MARK.gamma_multiply(0.35)));
                }
                for (m, label) in [(self.mark_in, "{"), (self.mark_out, "}")] {
                    if let Some(m) = m {
                        p.text(pos2(x_of(m), ruler.max.y - 2.0), Align2::CENTER_BOTTOM, label, semibold(13.0), MARK);
                    }
                }
                let x = x_of(self.t);
                p.line_segment([pos2(x, ruler.min.y), pos2(x, a2.max.y)], Stroke::new(1.5, PLAYHEAD));
                p.add(Shape::convex_polygon(vec![pos2(x - 7.0, ruler.min.y), pos2(x + 7.0, ruler.min.y), pos2(x + 7.0, ruler.min.y + 8.0), pos2(x, ruler.min.y + 14.0), pos2(x - 7.0, ruler.min.y + 8.0)], PLAYHEAD, Stroke::NONE));
                let bg = ui.interact(Rect::from_min_max(pos2(x0, t1.min.y), pos2(rect.max.x, a2.max.y)), ui.make_persistent_id("tracks"), Sense::click_and_drag());
                match self.tool {
                    Tool::Hand => {
                        ui.ctx().set_cursor_icon(if bg.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
                        if bg.dragged() {
                            self.force_scroll = Some(self.tl_scroll - bg.drag_delta().x);
                        }
                    }
                    Tool::Zoom => {
                        if bg.clicked()
                            && let Some(pp) = bg.interact_pointer_pos()
                        {
                            let at = t_of(pp.x);
                            let f = if ui.input(|i| i.modifiers.alt) { 1.0 / 1.5 } else { 1.5 };
                            self.pps = (self.pps * f).clamp(2.0, 600.0);
                            self.force_scroll = Some(at as f32 * self.pps - (pp.x - rect.min.x - label_w) + self.tl_scroll * 0.0);
                        }
                    }
                    _ => {
                        if bg.clicked()
                            && let Some(pp) = bg.interact_pointer_pos()
                        {
                            self.seek(t_of(pp.x));
                            self.sel = None;
                        }
                    }
                }
                if self.playing {
                    let view_right = self.tl_scroll + ui.clip_rect().width() - label_w;
                    let px = self.t as f32 * pps;
                    if px > view_right - 40.0 || px < self.tl_scroll {
                        self.force_scroll = Some((px - 60.0).max(0.0));
                    }
                }
            }));
            self.tl_scroll = out.inner.state.clamped_offset().x;
        });
    }

    fn sheets(&mut self, ctx: &egui::Context) {
        if let Some(mut opts) = self.export.take() {
            let mut go = false;
            let mut cancel = false;
            let keep = pro::sheet(ctx, "export", 440.0, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(9), BLUE.gamma_multiply(0.25));
                    glyph(ui.painter(), r.shrink(9.0), G::Export, BLUE);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Exporter").font(semibold(18.0)).color(INK_STRONG));
                        ui.label(RichText::new("MP4 H.264 + AAC, compatible partout").size(12.0).color(MUTED));
                    });
                });
                ui.add_space(14.0);
                let row = |ui: &mut Ui, label: &str| {
                    ui.label(RichText::new(label).size(12.5).color(MUTED));
                };
                let mut kind = opts.audio_only as usize;
                row(ui, "Format");
                if pro::segmented(ui, &mut kind, &["Vidéo MP4", "Audio WAV"]) {
                    opts.audio_only = kind == 1;
                }
                ui.add_space(8.0);
                if !opts.audio_only {
                    row(ui, "Résolution");
                    let labels: Vec<&str> = RESOLUTIONS.iter().map(|r| r.0).collect();
                    pro::segmented(ui, &mut opts.res, &labels);
                    ui.add_space(8.0);
                    row(ui, "Qualité");
                    let labels: Vec<&str> = QUALITIES.iter().map(|q| q.0).collect();
                    pro::segmented(ui, &mut opts.quality, &labels);
                    ui.add_space(8.0);
                }
                row(ui, "Plage");
                let has_marks = self.mark_in.is_some() || self.mark_out.is_some();
                ui.add_enabled_ui(has_marks, |ui| {
                    pro::segmented(ui, &mut opts.range, &["Toute la séquence", "Entre entrée et sortie"]);
                });
                if !has_marks {
                    opts.range = 0;
                }
                ui.add_space(12.0);
                let (a, b) = if opts.range == 1 { (self.mark_in.unwrap_or(0.0), self.mark_out.unwrap_or(self.proj.duration())) } else { (0.0, self.proj.duration()) };
                let secs = (b - a).abs();
                let size = if opts.audio_only {
                    secs * OUT_RATE as f64 * 4.0
                } else {
                    let target = RESOLUTIONS[opts.res].1;
                    let (w, h) = if target == 0 { (self.proj.width as f64, self.proj.height as f64) } else { let k = target as f64 / self.proj.width.min(self.proj.height) as f64; (self.proj.width as f64 * k, self.proj.height as f64 * k) };
                    let rate = (w * h * self.proj.fps as f64 / 8.0 * QUALITIES[opts.quality].1).max(1_500_000.0);
                    secs * (rate + 192_000.0) / 8.0
                };
                ui.label(RichText::new(format!("Durée {} · environ {:.0} Mo", tc(secs, self.proj.fps), size / 1_000_000.0)).size(12.5).color(FAINT));
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
            let keep = pro::sheet(ctx, "sequence", 460.0, |ui| {
                ui.label(RichText::new("Réglages de la séquence").font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(12.0);
                ui.label(RichText::new("Format").size(12.5).color(MUTED));
                ui.horizontal_wrapped(|ui| {
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
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Images par seconde").size(12.5).color(MUTED));
                ui.horizontal(|ui| {
                    for f in [24, 25, 30, 50, 60] {
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
    }
}

fn media_info(s: &Source) -> String {
    let mut info = format!("{}\nDurée {}", s.name(), tc(s.duration, 30));
    if s.has_video {
        info.push_str(&format!("\n{} × {} · {:.2} i/s", s.width, s.height, s.fps));
    }
    info.push_str(if s.has_audio { "\nAvec son" } else { "\nSans son" });
    info.push_str("\nDouble-clic : ajouter à la timeline");
    info
}

fn media_menu(ui: &mut Ui, s: &Source, i: usize, action: &mut Option<(u8, usize)>) {
    if s.has_video && ui.button("Ajouter à la timeline").clicked() {
        *action = Some((0, i));
        ui.close();
    }
    if s.has_audio && ui.button("Ajouter comme musique (A2)").clicked() {
        *action = Some((1, i));
        ui.close();
    }
    if ui.button("Afficher dans l'explorateur").clicked() {
        *action = Some((2, i));
        ui.close();
    }
    ui.separator();
    if ui.button("Retirer du projet").clicked() {
        *action = Some((3, i));
        ui.close();
    }
}

fn row_slider<N: egui::emath::Numeric>(ui: &mut Ui, label: &str, v: &mut N, range: std::ops::RangeInclusive<N>, suffix: &str, log: bool) -> bool {
    let reset = ui.horizontal(|ui| {
        let r = ui.add(egui::Label::new(RichText::new(label).size(12.5).color(MUTED)).sense(Sense::click()));
        r.on_hover_text("Double-clic pour réinitialiser").double_clicked()
    });
    ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(80.0);
    let mut changed = ui.add(egui::Slider::new(v, range.clone()).suffix(suffix).logarithmic(log).max_decimals(2)).changed();
    if reset.inner {
        let zero = N::from_f64(if range.contains(&N::from_f64(0.0)) { 0.0 } else { range.start().to_f64() });
        let one = N::from_f64(1.0);
        *v = if log && range.contains(&one) { one } else { zero };
        changed = true;
    }
    changed
}

fn pct_slider(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>) {
    let mut p = *v * 100.0;
    if row_slider(ui, label, &mut p, range.start() * 100.0..=range.end() * 100.0, " %", false) {
        *v = p / 100.0;
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
            let pts: Vec<Pos2> = (0..=16).map(|k| {
                let a = std::f32::consts::PI * k as f32 / 16.0;
                at(0.5 + a.cos() * 0.38, 0.5 + a.sin() * 0.38)
            }).collect();
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
            let pts: Vec<Pos2> = (0..=20).map(|k| {
                let a = -0.6 + k as f32 * 0.26;
                at(0.5 + a.cos() * 0.4, 0.5 + a.sin() * 0.4)
            }).collect();
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
