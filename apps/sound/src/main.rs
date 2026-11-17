#![windows_subsystem = "windows"]

mod optlib;
mod reglages;
mod outils;
mod sfx;
mod audio;
mod dsp;
mod effects;
mod tools;
mod tutos;

use audio::Clip;
use studio_ui::i18n::tl;
use tools::{SpecSel, SpecShape, Tool};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, TextureOptions, Ui, Vec2, pos2, vec2,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};

const ACCENT: Color32 = Color32::from_rgb(100, 95, 240);
const WAVE: Color32 = Color32::from_rgb(140, 135, 255);
const MARK: Color32 = Color32::from_rgb(255, 196, 0);
const UNDO_LIMIT: usize = 30;
const LUFS: [(&str, f32); 4] = [("−14", -14.0), ("−16", -16.0), ("−19", -19.0), ("−23", -23.0)];
const EQ_BANDS: [(f32, &str); 5] = [(60.0, "60 Hz"), (250.0, "250 Hz"), (1000.0, "1 kHz"), (4000.0, "4 kHz"), (12000.0, "12 kHz")];
const PRESETS: [(&str, &str); 10] = [
    ("Voix de podcast", "Nettoie, compresse et règle à −16 LUFS"),
    ("Voix radio", "Pleine, présente et dense"),
    ("Téléphone", "Bande étroite 400 Hz – 3,4 kHz"),
    ("Mégaphone", "Médiums saturés"),
    ("Robot", "Modulation en anneau et bits réduits"),
    ("Voix grave", "−4 demi-tons, même durée"),
    ("Voix aiguë", "+5 demi-tons, même durée"),
    ("Lo-fi", "Grain de vieux sampler"),
    ("Grande salle", "Réverbération ample"),
    ("Sous l'eau", "Étouffé et ondulant"),
];
const WAVES: [&str; 4] = ["Sinus", "Carré", "Scie", "Triangle"];

struct Shared {
    clip: Arc<Clip>,
    pos: f64,
    playing: bool,
    start: usize,
    end: usize,
    looping: bool,
}

struct Player {
    shared: Arc<Mutex<Shared>>,
    _stream: Option<cpal::Stream>,
    error: Option<String>,
}

impl Player {
    fn new(clip: Arc<Clip>) -> Self {
        let shared = Arc::new(Mutex::new(Shared { clip, pos: 0.0, playing: false, start: 0, end: 0, looping: false }));
        let (stream, error) = match Self::open(shared.clone()) {
            Ok(s) => (Some(s), None),
            Err(e) => (None, Some(e)),
        };
        Self { shared, _stream: stream, error }
    }

    fn open(shared: Arc<Mutex<Shared>>) -> Result<cpal::Stream, String> {
        let host = cpal::default_host();
        let dev = host.default_output_device().ok_or("aucune sortie audio")?;
        let cfg = dev.default_output_config().map_err(|e| e.to_string())?;
        let out_rate = cfg.sample_rate() as f64;
        let out_ch = cfg.channels() as usize;
        let stream = dev
            .build_output_stream::<f32, _, _>(
                cfg.config(),
                move |data: &mut [f32], _| {
                    let mut s = shared.lock().unwrap();
                    let step = s.clip.rate as f64 / out_rate;
                    let nch = s.clip.ch.len().max(1);
                    for frame in data.chunks_mut(out_ch) {
                        if s.playing && s.looping && s.pos as usize >= s.end && s.end > s.start {
                            s.pos = s.start as f64;
                        }
                        let i = s.pos as usize;
                        if s.playing && i < s.end {
                            for (c, out) in frame.iter_mut().enumerate() {
                                *out = s.clip.ch[c % nch][i];
                            }
                            s.pos += step;
                        } else {
                            s.playing = false;
                            frame.fill(0.0);
                        }
                    }
                },
                |e| eprintln!("audio: {e}"),
                None,
            )
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(stream)
    }

    fn set_clip(&self, clip: Arc<Clip>) {
        let mut s = self.shared.lock().unwrap();
        s.playing = false;
        s.clip = clip;
    }

    fn play(&self, from: usize, to: usize, looping: bool) {
        let mut s = self.shared.lock().unwrap();
        s.pos = from as f64;
        s.start = from;
        s.end = to.min(s.clip.len());
        s.looping = looping;
        s.playing = true;
    }

    fn stop(&self) {
        self.shared.lock().unwrap().playing = false;
    }

    fn state(&self) -> (bool, usize) {
        let s = self.shared.lock().unwrap();
        (s.playing, s.pos as usize)
    }
}

struct Recorder {
    _stream: cpal::Stream,
    data: Arc<Mutex<Vec<f32>>>,
    channels: usize,
    rate: u32,
}

impl Recorder {
    fn start() -> Result<Self, String> {
        let dev = cpal::default_host().default_input_device().ok_or("aucun micro détecté")?;
        let cfg = dev.default_input_config().map_err(|e| e.to_string())?;
        if cfg.sample_format() != cpal::SampleFormat::F32 {
            return Err(format!("format micro non pris en charge ({:?})", cfg.sample_format()));
        }
        let data = Arc::new(Mutex::new(Vec::new()));
        let sink = data.clone();
        let stream = dev
            .build_input_stream::<f32, _, _>(cfg.config(), move |input: &[f32], _| sink.lock().unwrap().extend_from_slice(input), |e| eprintln!("micro: {e}"), None)
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self { _stream: stream, data, channels: cfg.channels() as usize, rate: cfg.sample_rate() })
    }

    fn seconds(&self) -> f32 {
        self.data.lock().map(|d| d.len()).unwrap_or(0) as f32 / (self.channels.max(1) as f32 * self.rate as f32)
    }

    fn level(&self) -> f32 {
        self.data.lock().map(|d| d.iter().rev().take(2048).fold(0f32, |m, v| m.max(v.abs()))).unwrap_or(0.0)
    }

    fn finish(self) -> Clip {
        let raw = self.data.lock().map(|d| d.clone()).unwrap_or_default();
        let n = self.channels.max(1);
        let ch = (0..n.min(2)).map(|c| raw.chunks(n).map(|f| f[c.min(f.len() - 1)]).collect()).collect();
        Clip { rate: self.rate, ch }
    }
}

struct Peaks {
    key: (usize, f64, usize, u64),
    cols: Vec<Vec<(f32, f32)>>,
}

struct Fx {
    gain: f32,
    peak: f32,
    lufs: usize,
    ceiling: f32,
    fade_shape: usize,
    eq: [f32; 5],
    comp: [f32; 3],
    gate: f32,
    drive: f32,
    crush: [f32; 2],
    echo: [f32; 3],
    verb: [f32; 2],
    chorus: [f32; 3],
    flanger: [f32; 3],
    trem: [f32; 2],
    autopan: [f32; 2],
    ring: f32,
    speed: f32,
    tempo: f32,
    semis: f32,
    pan: f32,
    width: f32,
    denoise: f32,
    silence: [f32; 3],
    low_cut: f32,
    high_cut: f32,
    wave: usize,
    noise_kind: usize,
    freq: f32,
    freq2: f32,
    amp: f32,
    bpm: f32,
    secs: f32,
}

struct ExportOpts {
    bits: usize,
    rate: usize,
    channels: usize,
    range: usize,
    normalize: bool,
    dither: usize,
    cues: bool,
    hq: bool,
}

const INFO_IDS: [(&[u8; 4], &str); 6] = [(b"INAM", "Titre"), (b"IART", "Artiste"), (b"IPRD", "Album"), (b"ICRD", "Année"), (b"IGNR", "Genre"), (b"ICMT", "Commentaire")];
const FFT_SIZES: [usize; 4] = [512, 1024, 2048, 4096];

#[derive(Clone, Copy)]
enum Drag {
    None,
    Select(usize),
    EdgeA,
    EdgeB,
    Zoom(usize),
    Heal(usize),
    Spec(usize, f32),
    Pencil(usize, Option<(usize, f32)>),
    Fade(bool),
    Env(usize),
}

type Op = Box<dyn FnOnce(&mut Clip, usize, usize)>;

struct Stats {
    key: (u64, usize, usize),
    peak: f32,
    rms: f32,
    lufs: f32,
    clipped: usize,
    true_peak: f32,
}

struct Sound {
    clip: Clip,
    version: u64,
    file: Option<PathBuf>,
    modified: bool,
    player: Player,
    recorder: Option<Recorder>,
    undo: Vec<(Clip, String)>,
    redo: Vec<(Clip, String)>,
    clipboard: Option<Clip>,
    cursor: usize,
    sel: Option<(usize, usize)>,
    markers: Vec<(usize, String)>,
    looping: bool,
    follow: bool,
    drag: Drag,
    view_start: f64,
    spp: f64,
    amp_zoom: f32,
    view_mode: usize,
    fit_pending: bool,
    peaks: Option<Peaks>,
    overview: Option<(u64, usize, Vec<f32>)>,
    specgram: Option<((usize, u64, usize, u64, usize), egui::TextureHandle)>,
    spectrum: Option<(usize, u64, Vec<f32>)>,
    stats: Option<Stats>,
    fx: Fx,
    noise: Option<Vec<f32>>,
    preview: Option<usize>,
    export: Option<ExportOpts>,
    props_sheet: bool,
    notice: Option<(String, bool)>,
    title: String,
    icon: egui::TextureHandle,
    help: studio_ui::help::Help,
    inbox: studio_ui::handoff::Receiver,
    palette: studio_ui::palette::Palette,
    tool: Tool,
    spec_sel: Option<SpecSel>,
    spec_log: bool,
    fft: usize,
    spec_db: f32,
    brush: f32,
    fades: [(usize, f32); 2],
    env: Vec<(usize, f32)>,
    rack_cat: usize,
    rack_pick: usize,
    rack_vals: Vec<Vec<f32>>,
    rack_search: String,
    meta: [String; 6],
    bpm: Option<(f32, usize)>,
    regions: [f32; 3],
}

impl Sound {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        studio_ui::i18n::register(include_str!("../i18n/en.tsv"));
        let clip = Clip { rate: 44_100, ch: vec![vec![0.0; 44_100 * 5], vec![0.0; 44_100 * 5]] };
        let player = Player::new(Arc::new(clip.clone()));
        let mut me = Self {
            notice: player.error.clone().map(|e| (format!("Lecture audio indisponible : {e}"), false)),
            clip,
            version: 0,
            file: None,
            modified: false,
            player,
            recorder: None,
            undo: Vec::new(),
            redo: Vec::new(),
            clipboard: None,
            cursor: 0,
            sel: None,
            markers: Vec::new(),
            looping: false,
            follow: true,
            drag: Drag::None,
            view_start: 0.0,
            spp: 100.0,
            amp_zoom: 1.0,
            view_mode: 0,
            fit_pending: true,
            peaks: None,
            overview: None,
            specgram: None,
            spectrum: None,
            stats: None,
            fx: Fx {
                gain: 0.0,
                peak: -1.0,
                lufs: 1,
                ceiling: -1.0,
                fade_shape: 1,
                eq: [0.0; 5],
                comp: [-18.0, 4.0, 3.0],
                gate: -45.0,
                drive: 0.3,
                crush: [8.0, 2.0],
                echo: [250.0, 0.35, 0.4],
                verb: [0.5, 0.3],
                chorus: [6.0, 0.8, 0.5],
                flanger: [0.25, 3.0, 0.6],
                trem: [5.0, 0.5],
                autopan: [0.5, 0.8],
                ring: 60.0,
                speed: 100.0,
                tempo: 100.0,
                semis: 0.0,
                pan: 0.0,
                width: 100.0,
                denoise: 0.6,
                silence: [-45.0, 600.0, 200.0],
                low_cut: 80.0,
                high_cut: 12_000.0,
                wave: 0,
                noise_kind: 0,
                freq: 440.0,
                freq2: 4000.0,
                amp: 0.5,
                bpm: 120.0,
                secs: 2.0,
            },
            noise: None,
            preview: None,
            export: None,
            props_sheet: false,
            title: String::new(),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "sound"),
            help: studio_ui::help::Help::new("Sound", tutos::SOUND),
            inbox: studio_ui::handoff::Receiver::new("sound"),
            palette: studio_ui::palette::Palette::new(&cc.egui_ctx),
            tool: Tool::Select,
            spec_sel: None,
            spec_log: true,
            fft: 2,
            spec_db: -24.0,
            brush: 14.0,
            fades: [(0, 0.0); 2],
            env: Vec::new(),
            rack_cat: 1,
            rack_pick: 0,
            rack_vals: (0..effects::DEFS.len()).map(effects::defaults).collect(),
            rack_search: String::new(),
            meta: Default::default(),
            bpm: None,
            regions: [-40.0, 300.0, 120.0],
        };
        if let Some(p) = std::env::args().nth(1) {
            me.open_path(Path::new(&p));
        }
        me
    }

    fn open_path(&mut self, p: &Path) {
        match Clip::decode(p) {
            Ok(c) => {
                self.clip = c;
                self.file = Some(p.to_path_buf());
                self.modified = false;
                self.undo.clear();
                self.redo.clear();
                self.cursor = 0;
                self.sel = None;
                self.markers.clear();
                self.fit_pending = true;
                self.changed();
            }
            Err(e) => self.notice = Some((format!("Ouverture impossible : {e}"), false)),
        }
    }

    fn range(&self) -> (usize, usize) {
        self.sel.unwrap_or((0, self.clip.len()))
    }

    fn changed(&mut self) {
        self.version += 1;
        self.preview = None;
        self.player.set_clip(Arc::new(self.clip.clone()));
        let n = self.clip.len();
        self.cursor = self.cursor.min(n);
        self.sel = self.sel.map(|(a, b)| (a.min(n), b.min(n))).filter(|(a, b)| b > a);
        self.markers.retain(|m| m.0 <= n);
    }

    fn cut_silences(&mut self, min_ms: f32) {
        let (a, b) = self.sel.unwrap_or((0, self.clip.len()));
        let mut c = self.clip.clone();
        let n = c.cut_silences(a, b, -40.0, min_ms, 120.0);
        if n == 0 {
            self.notice = Some(("Aucun silence assez long à supprimer".into(), false));
            return;
        }
        self.snapshot("Suppression des silences");
        let before = self.clip.secs(self.clip.len());
        self.clip = c;
        self.sel = None;
        self.changed();
        self.notice = Some((format!("{n} silence(s) supprimé(s) · {:.1} s gagnées", before - self.clip.secs(self.clip.len())), true));
    }

    fn snapshot(&mut self, label: &str) {
        self.undo.push((self.clip.clone(), label.to_string()));
        if self.undo.len() > studio_ui::palette::undo_levels(UNDO_LIMIT) {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn edit(&mut self, label: &str, f: impl FnOnce(&mut Clip, usize, usize)) {
        if self.clip.len() == 0 {
            return;
        }
        let (a, b) = self.range();
        self.snapshot(label);
        let before = self.clip.len();
        f(&mut self.clip, a, b);
        if self.sel.is_some() && self.clip.len() != before {
            let nb = (b as i64 + self.clip.len() as i64 - before as i64).max(a as i64 + 1) as usize;
            self.sel = Some((a, nb.min(self.clip.len())));
        }
        self.modified = true;
        self.changed();
        self.notice = Some((format!("{label} appliqué {}", if self.sel.is_some() { "à la sélection" } else { "à tout le fichier" }), true));
    }

    fn preview_fx(&mut self, f: impl FnOnce(&mut Clip, usize, usize)) {
        let (a, b) = self.range();
        let b = b.min(a + self.clip.rate as usize * 15);
        if b <= a {
            return;
        }
        let mut part = self.clip.slice(a, b);
        let n = part.len();
        f(&mut part, 0, n);
        let len = part.len();
        self.player.set_clip(Arc::new(part));
        self.player.play(0, len, false);
        self.preview = Some(a);
    }

    fn undo(&mut self) {
        if let Some((c, l)) = self.undo.pop() {
            self.redo.push((std::mem::replace(&mut self.clip, c), l));
            self.modified = true;
            self.changed();
        }
    }

    fn redo(&mut self) {
        if let Some((c, l)) = self.redo.pop() {
            self.undo.push((std::mem::replace(&mut self.clip, c), l));
            self.modified = true;
            self.changed();
        }
    }

    fn cut(&mut self, keep_copy: bool) {
        let Some((a, b)) = self.sel else { return };
        if keep_copy {
            self.clipboard = Some(self.clip.slice(a, b));
        }
        self.snapshot(if keep_copy { "Couper" } else { "Supprimer" });
        self.clip.remove(a, b);
        self.shift_markers(a, -((b - a) as i64));
        self.cursor = a;
        self.sel = None;
        self.modified = true;
        self.changed();
    }

    fn shift_markers(&mut self, from: usize, by: i64) {
        for m in &mut self.markers {
            if m.0 >= from {
                m.0 = (m.0 as i64 + by).max(from as i64) as usize;
            }
        }
    }

    fn insert_sfx(&mut self, sfx: Clip, name: &str) {
        let label = format!("Bruitage « {name} »");
        if self.clip.len() == 0 {
            self.snapshot(&label);
            self.clip = sfx;
            self.sel = None;
            self.cursor = 0;
        } else {
            let at = self.sel.map_or(self.cursor, |s| s.0).min(self.clip.len());
            self.snapshot(&label);
            self.clip.insert(at, &sfx);
            let len = sfx.resampled(self.clip.rate).len();
            self.sel = Some((at, at + len));
        }
        self.modified = true;
        self.changed();
        self.notice = Some((format!("{label} inséré"), true));
    }

    fn paste(&mut self) {
        let Some(cb) = self.clipboard.clone() else { return };
        let at = self.sel.map_or(self.cursor, |s| s.0);
        let replace = self.sel.take();
        self.snapshot("Coller");
        if let Some((a, b)) = replace {
            self.clip.remove(a, b);
        }
        self.clip.insert(at, &cb);
        let len = cb.resampled(self.clip.rate).len();
        self.sel = Some((at, at + len));
        self.modified = true;
        self.changed();
    }

    fn duplicate(&mut self) {
        let Some((a, b)) = self.sel else { return };
        let part = self.clip.slice(a, b);
        self.snapshot("Dupliquer");
        self.clip.insert(b, &part);
        self.shift_markers(b, (b - a) as i64);
        self.sel = Some((b, b + (b - a)));
        self.modified = true;
        self.changed();
    }

    fn trim(&mut self) {
        let Some((a, b)) = self.sel else { return };
        self.snapshot("Rogner");
        self.clip = self.clip.slice(a, b);
        self.markers.retain(|m| m.0 >= a && m.0 <= b);
        self.markers.iter_mut().for_each(|m| m.0 -= a);
        self.sel = None;
        self.cursor = 0;
        self.fit_pending = true;
        self.modified = true;
        self.changed();
    }

    fn insert_clip(&mut self, part: Clip, label: &str) {
        self.snapshot(label);
        let at = self.cursor;
        let len = part.resampled(self.clip.rate).len();
        if self.clip.len() == 0 || self.clip.peak(0, self.clip.len()) == 0.0 && self.file.is_none() && !self.modified {
            self.clip = part.resampled(self.clip.rate);
            self.sel = Some((0, self.clip.len()));
            self.fit_pending = true;
        } else {
            self.clip.insert(at, &part);
            self.shift_markers(at, len as i64);
            self.sel = Some((at, at + len));
        }
        self.modified = true;
        self.changed();
    }

    fn toggle_record(&mut self) {
        match self.recorder.take() {
            Some(r) => {
                let take = r.finish();
                if take.len() > 0 {
                    self.insert_clip(take, "Enregistrement");
                }
            }
            None => {
                self.player.stop();
                match Recorder::start() {
                    Ok(r) => self.recorder = Some(r),
                    Err(e) => self.notice = Some((format!("Enregistrement : {e}"), false)),
                }
            }
        }
    }

    fn prepare_export(&self, part: Clip, o: &ExportOpts) -> Clip {
        let target = [0u32, 44_100, 48_000, 22_050, 96_000][o.rate];
        let mut c = match target {
            0 => part,
            r if o.hq => part.resample_hq(r),
            r => part.resampled(r),
        };
        match o.channels {
            1 => {
                c.to_mono();
                c.ch.truncate(1);
            }
            2 if c.ch.len() == 1 => c.ch.push(c.ch[0].clone()),
            _ => {}
        }
        if o.normalize {
            let n = c.len();
            c.normalize(0, n, -1.0);
        }
        c
    }

    fn run_export(&mut self, o: &ExportOpts) {
        let bits = [16u16, 24, 32, 8][o.bits];
        let info: Vec<(&[u8; 4], String)> = INFO_IDS.iter().zip(&self.meta).map(|((id, _), v)| (*id, v.clone())).collect();
        let dither = o.dither as u8;
        let save = |c: &Clip, path: &Path, cues: &[(usize, String)]| dsp::export_wav(c, path, bits, dither, &info, cues);
        let stem = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("audio".into());
        if o.range == 2 {
            let Some(dir) = rfd::FileDialog::new().set_title("Dossier des morceaux").pick_folder() else { return };
            let mut cuts: Vec<usize> = self.markers.iter().map(|m| m.0).filter(|m| *m > 0 && *m < self.clip.len()).collect();
            cuts.sort();
            cuts.dedup();
            let mut bounds = vec![0];
            bounds.extend(cuts);
            bounds.push(self.clip.len());
            let mut n = 0;
            for (k, w) in bounds.windows(2).enumerate() {
                if w[1] <= w[0] {
                    continue;
                }
                let name = self.markers.iter().find(|m| m.0 == w[0]).map(|m| clean(&m.1)).filter(|s| !s.is_empty()).unwrap_or_else(|| format!("partie {}", k + 1));
                let part = self.prepare_export(self.clip.slice(w[0], w[1]), o);
                if let Err(e) = save(&part, &dir.join(format!("{stem} - {:02} - {name}.wav", k + 1)), &[]) {
                    self.notice = Some((format!("Export : {e}"), false));
                    return;
                }
                n += 1;
            }
            self.notice = Some((format!("{n} morceaux exportés dans {}", dir.display()), true));
            return;
        }
        let Some(path) = rfd::FileDialog::new().add_filter("WAV", &["wav"]).set_file_name(format!("{stem}.wav")).save_file() else { return };
        let (a, b) = if o.range == 1 { self.range() } else { (0, self.clip.len()) };
        let part = self.prepare_export(self.clip.slice(a, b), o);
        let k = part.rate as f64 / self.clip.rate as f64;
        let cues: Vec<(usize, String)> = if o.cues { self.markers.iter().filter(|m| m.0 >= a && m.0 < b).map(|m| (((m.0 - a) as f64 * k) as usize, m.1.clone())).collect() } else { Vec::new() };
        match save(&part, &path, &cues) {
            Ok(()) => {
                if o.range == 0 {
                    self.modified = false;
                }
                self.notice = Some((format!("Exporté : {}", path.display()), true));
            }
            Err(e) => self.notice = Some((format!("Export : {e}"), false)),
        }
    }

    fn toggle_play(&mut self) {
        let (playing, _) = self.player.state();
        if playing {
            self.player.stop();
        } else {
            if self.preview.take().is_some() {
                self.player.set_clip(Arc::new(self.clip.clone()));
            }
            let (a, b) = self.sel.unwrap_or((self.cursor, self.clip.len()));
            self.player.play(a, b, self.looping && self.sel.is_some());
        }
    }

    fn add_marker(&mut self) {
        let tol = (self.clip.rate / 50) as usize;
        if let Some(k) = self.markers.iter().position(|m| m.0.abs_diff(self.cursor) < tol) {
            self.markers.remove(k);
        } else {
            let n = self.markers.len() + 1;
            self.markers.push((self.cursor, format!("Marqueur {n}")));
            self.markers.sort_by_key(|m| m.0);
        }
    }

    fn jump(&mut self, dir: i32) {
        let c = self.cursor;
        let target = if dir > 0 { self.markers.iter().map(|m| m.0).find(|m| *m > c) } else { self.markers.iter().rev().map(|m| m.0).find(|m| *m < c) };
        if let Some(t) = target {
            self.cursor = t;
            self.sel = None;
            self.reveal(t);
        }
    }

    fn reveal(&mut self, s: usize) {
        let w = self.peaks.as_ref().map_or(1000, |p| p.key.2) as f64;
        let visible = w * self.spp;
        if (s as f64) < self.view_start || s as f64 > self.view_start + visible {
            self.view_start = (s as f64 - visible * 0.2).max(0.0);
        }
    }

    fn zoom_by(&mut self, k: f64) {
        let w = self.peaks.as_ref().map_or(1000, |p| p.key.2) as f64;
        let centre = self.view_start + w * self.spp / 2.0;
        self.spp = (self.spp * k).clamp(0.05, (self.clip.len() as f64 / w).max(0.05));
        self.view_start = centre - w * self.spp / 2.0;
    }

    fn zoom_sel(&mut self) {
        if let Some((a, b)) = self.sel {
            let w = self.peaks.as_ref().map_or(1000, |p| p.key.2) as f64;
            let span = (b - a) as f64 * 1.1;
            self.spp = (span / w).max(0.05);
            self.view_start = a as f64 - (b - a) as f64 * 0.05;
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        if sc(M::COMMAND | M::SHIFT, Key::Z) || sc(M::COMMAND, Key::Y) {
            self.redo();
        }
        if sc(M::COMMAND, Key::Z) {
            self.undo();
        }
        if sc(M::COMMAND, Key::X) {
            self.cut(true);
        }
        if sc(M::COMMAND, Key::C)
            && let Some((a, b)) = self.sel
        {
            self.clipboard = Some(self.clip.slice(a, b));
        }
        if sc(M::COMMAND, Key::V) {
            self.paste();
        }
        if sc(M::COMMAND, Key::D) {
            self.duplicate();
        }
        if sc(M::COMMAND, Key::T) {
            self.trim();
        }
        if sc(M::COMMAND, Key::A) {
            self.sel = Some((0, self.clip.len()));
        }
        if sc(M::COMMAND, Key::O) {
            self.open_dialog();
        }
        if sc(M::COMMAND, Key::E) || sc(M::COMMAND, Key::S) {
            self.export = Some(self.export_opts());
        }
        if sc(M::COMMAND, Key::Num0) {
            self.fit_pending = true;
        }
        if sc(M::COMMAND, Key::Plus) || sc(M::COMMAND, Key::Equals) {
            self.zoom_by(0.5);
        }
        if sc(M::COMMAND, Key::Minus) {
            self.zoom_by(2.0);
        }
        let (space, del, home, end, letters, up, down, left, right, shift) = ctx.input(|i| {
            let p = |k| i.key_pressed(k) && !i.modifiers.command;
            (
                i.key_pressed(Key::Space),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::Home),
                i.key_pressed(Key::End),
                [p(Key::M), p(Key::L), p(Key::R), p(Key::Z), p(Key::S), p(Key::F)],
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::ArrowDown),
                i.key_pressed(Key::ArrowLeft) && !i.modifiers.command,
                i.key_pressed(Key::ArrowRight) && !i.modifiers.command,
                i.modifiers.shift,
            )
        });
        let [m, l, r, z, s, f] = letters;
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F1)) {
            self.help.learn(self.tool.name());
        }
        for (_, key) in tools::GROUPS {
            if ctx.input(|i| i.key_pressed(key) && !i.modifiers.command) {
                self.cycle_tool(key);
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.spec_sel = None;
            self.sel = None;
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            if !self.env.is_empty() {
                self.apply_env();
            }
            if self.fades[0].0 + self.fades[1].0 > 0 {
                self.apply_fades();
            }
            if self.spec_sel.is_some() {
                let g = self.spec_db;
                self.spectral_apply(dsp::SpecMode::Gain(g), "Atténuation spectrale");
            }
        }
        if space {
            self.toggle_play();
        }
        if del {
            self.cut(false);
        }
        if home {
            if shift {
                self.sel = Some((0, self.sel.map_or(self.cursor, |s| s.1)));
            } else {
                self.cursor = 0;
                self.sel = None;
            }
            self.view_start = 0.0;
        }
        if end {
            if shift {
                self.sel = Some((self.sel.map_or(self.cursor, |s| s.0), self.clip.len()));
            } else {
                self.cursor = self.clip.len();
                self.sel = None;
            }
            self.reveal(self.clip.len());
        }
        let step = (self.spp * 20.0).max(1.0) as usize;
        if left {
            self.cursor = self.cursor.saturating_sub(step);
            self.sel = None;
        }
        if right {
            self.cursor = (self.cursor + step).min(self.clip.len());
            self.sel = None;
        }
        if m {
            self.add_marker();
        }
        if l {
            self.looping = !self.looping;
        }
        if r {
            self.toggle_record();
        }
        if z {
            self.zoom_sel();
        }
        if s {
            self.view_mode = 1 - self.view_mode;
        }
        if f {
            self.follow = !self.follow;
        }
        if up {
            self.jump(-1);
        }
        if down {
            self.jump(1);
        }
    }

    fn open_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("Audio", &["wav", "mp3", "flac", "ogg", "m4a", "aac", "mp4", "mkv", "webm"]).pick_file() {
            self.open_path(&p);
        }
    }

    fn export_opts(&self) -> ExportOpts {
        ExportOpts { bits: 0, rate: 0, channels: 0, range: if self.sel.is_some() { 1 } else { 0 }, normalize: false, dither: 1, cues: true, hq: true }
    }

    fn set_tool(&mut self, t: Tool) {
        self.tool = t;
        if t.spectral() {
            self.view_mode = 1;
        }
        self.notice = Some((format!("{} : {}", t.name(), t.help()), true));
    }

    fn cycle_tool(&mut self, key: Key) {
        let Some(g) = tools::GROUPS.iter().find(|g| g.1 == key).map(|g| g.0) else { return };
        let next = match g.iter().position(|t| t.0 == self.tool) {
            Some(k) => g[(k + 1) % g.len()].0,
            None => g[0].0,
        };
        self.set_tool(next);
    }

    fn spectral_apply(&mut self, mode: dsp::SpecMode, label: &str) {
        let Some(sel) = self.spec_sel.clone() else { return };
        let nyq = self.clip.rate as f32 / 2.0;
        let log = self.spec_log;
        let (a, b) = (sel.a.min(self.clip.len()), sel.b.min(self.clip.len()));
        if b <= a {
            return;
        }
        self.snapshot(label);
        let yf = move |f: f32| dsp::y_frac(f, nyq, log);
        self.clip.spectral_edit(a, b, &|s, f| sel.contains(s, f, &yf), mode);
        self.modified = true;
        self.changed();
        self.notice = Some((format!("{label} appliqué à la zone spectrale"), true));
    }

    fn env_gain(&self, s: usize) -> f32 {
        let e = &self.env;
        match e.iter().position(|p| p.0 > s) {
            None => e.last().map_or(0.0, |p| p.1),
            Some(0) => e[0].1,
            Some(k) => {
                let (p, q) = (e[k - 1], e[k]);
                p.1 + (q.1 - p.1) * (s - p.0) as f32 / (q.0 - p.0).max(1) as f32
            }
        }
    }

    fn apply_env(&mut self) {
        if self.env.is_empty() {
            return;
        }
        let gains: Vec<f32> = (0..self.clip.len()).map(|s| dsp::db_to(self.env_gain(s))).collect();
        self.snapshot("Enveloppe de volume");
        let n = self.clip.len();
        self.clip.each(0, n, |i, v| v * gains[i]);
        self.env.clear();
        self.modified = true;
        self.changed();
        self.notice = Some(("Enveloppe de volume appliquée".into(), true));
    }

    fn apply_fades(&mut self) {
        let [(fi, ci), (fo, co)] = self.fades;
        let n = self.clip.len();
        if fi + fo == 0 || n == 0 {
            return;
        }
        self.snapshot("Fondus");
        let (fi, fo) = (fi.min(n), fo.min(n));
        self.clip.each(0, fi, |i, v| v * dsp::fade_gain(i as f32 / fi.max(1) as f32, ci));
        self.clip.each(n - fo, n, |i, v| v * dsp::fade_gain(1.0 - i as f32 / fo.max(1) as f32, co));
        self.fades = [(0, 0.0); 2];
        self.modified = true;
        self.changed();
        self.notice = Some(("Fondus appliqués".into(), true));
    }

    fn run_rack(&mut self, k: usize, preview: bool) {
        let d = &effects::DEFS[k];
        let vals = self.rack_vals[k].clone();
        let id = d.id;
        if preview {
            self.preview_fx(move |c, a, b| {
                effects::run(id, c, a, b, &vals);
            });
            return;
        }
        let mut msg = None;
        let whole = self.sel.is_none();
        self.edit(d.name, |c, a, b| msg = effects::run(id, c, a, b, &vals));
        if let Some(m) = msg {
            self.notice = Some((format!("{} : {m}", d.name), true));
        }
        if whole {
            self.fit_pending = true;
        }
    }

    fn apply_preset(&mut self, k: usize) {
        let name = PRESETS[k].0;
        self.edit(name, |c, a, b| match k {
            0 => {
                c.filter(a, b, 4, 80.0, 0.0, 0.707);
                c.gate(a, b, -50.0);
                c.compress(a, b, -20.0, 3.0, 4.0);
                c.filter(a, b, 0, 3000.0, 3.0, 1.0);
                c.loudness_normalize(a, b, -16.0);
            }
            1 => {
                c.filter(a, b, 4, 100.0, 0.0, 0.707);
                c.compress(a, b, -24.0, 6.0, 6.0);
                c.saturate(a, b, 0.2);
                c.filter(a, b, 0, 2500.0, 4.0, 0.9);
                c.loudness_normalize(a, b, -14.0);
            }
            2 => {
                c.filter(a, b, 4, 400.0, 0.0, 0.707);
                c.filter(a, b, 3, 3400.0, 0.0, 0.707);
                c.saturate(a, b, 0.3);
            }
            3 => {
                c.filter(a, b, 4, 700.0, 0.0, 0.707);
                c.filter(a, b, 3, 2600.0, 0.0, 0.707);
                c.saturate(a, b, 0.6);
                c.normalize(a, b, -1.0);
            }
            4 => {
                c.ring(a, b, 60.0);
                c.bitcrush(a, b, 8.0, 2);
                c.normalize(a, b, -1.0);
            }
            5 => c.pitch(a, b, -4.0),
            6 => c.pitch(a, b, 5.0),
            7 => {
                c.bitcrush(a, b, 10.0, 2);
                c.filter(a, b, 3, 6000.0, 0.0, 0.707);
                c.saturate(a, b, 0.25);
            }
            8 => c.reverb(a, b, 0.9, 0.45),
            _ => {
                c.filter(a, b, 3, 600.0, 0.0, 0.9);
                c.mod_delay(a, b, 8.0, 6.0, 0.4, 0.0, 0.6);
                c.reverb(a, b, 0.6, 0.3);
            }
        });
    }
}

fn clean(s: &str) -> String {
    s.chars().filter(|c| !"\\/:*?\"<>|".contains(*c)).collect::<String>().trim().to_string()
}

fn fmt_time(s: f64) -> String {
    let m = (s / 60.0).floor();
    format!("{:02}:{:06.3}", m as u64, s - m * 60.0)
}

fn db(v: f32) -> f32 {
    20.0 * v.max(1e-6).log10()
}

impl Sound {

    fn commands(&self) -> Vec<studio_ui::palette::Cmd> {
        let mut v = vec![
            studio_ui::palette::Cmd::new(tl("Ouvrir un son…"), tl("Ctrl+O"), "open"),
            studio_ui::palette::Cmd::new(tl("Exporter…"), tl("Ctrl+E"), "export"),
            studio_ui::palette::Cmd::new(tl("Tout sélectionner"), tl("Ctrl+A"), "select-all"),
            studio_ui::palette::Cmd::new(tl("Lecture / pause"), tl("Espace"), "play"),
            studio_ui::palette::Cmd::new(tl("Ajouter un marqueur"), tl("M"), "marker"),
            studio_ui::palette::Cmd::new(tl("Supprimer les silences (plus de 0,8 s)"), tl("Édition"), "silences").confirm_if(self.sel.is_none()),
            studio_ui::palette::Cmd::new(tl("Supprimer les silences courts (plus de 0,4 s)"), tl("Édition"), "silences-short").confirm_if(self.sel.is_none()),
            studio_ui::palette::Cmd::new(tl("Rogner les silences au début et à la fin"), tl("Nettoyage · conserve les pauses internes"), "trim-edges").confirm(),
            studio_ui::palette::Cmd::new(tl("Normaliser la crête à −1 dB"), tl("Volume · sélection ou fichier entier"), "normalize-peak"),
            studio_ui::palette::Cmd::new(tl("Normaliser le volume perçu à −16 LUFS"), tl("Podcast et voix · sélection ou fichier entier"), "normalize-lufs"),
            studio_ui::palette::Cmd::new(tl("Ajouter des micro-fondus aux bords"), tl("20 ms · évite les clics"), "micro-fades"),
            studio_ui::palette::Cmd::new(tl("Découper aux silences en fichiers WAV…"), tl("Prises, samples, chapitres"), "split-files"),
            studio_ui::palette::Cmd::new(tl("Boucle parfaite (fondu de 0,5 s)"), tl("Musique de jeu, ambiance"), "loop:500").confirm(),
            studio_ui::palette::Cmd::new(tl("Boucle parfaite (fondu de 2 s)"), tl("Musique de jeu, ambiance"), "loop:2000").confirm(),
            studio_ui::palette::Cmd::new(tl("Annuler"), tl("Ctrl+Z"), "undo"),
        ];
        let has_audio = self.clip.len() > 0;
        let can_undo = !self.undo.is_empty();
        v = v
            .into_iter()
            .map(|command| match command.id.as_str() {
                "open" => command,
                "undo" => command.enabled_if(can_undo, "Rien à annuler"),
                _ => command.enabled_if(has_audio, "Ouvre ou enregistre d'abord un son"),
            })
            .collect();
        v.extend(outils::TOOLS.iter().map(|(id, label, _)| studio_ui::palette::Cmd::new(format!("Outil : {label}"), "Sound · nouvel outil · sélection ou fichier", format!("tool:{id}")).enabled_if(has_audio, "Ouvre ou enregistre d'abord un son")));
        v.extend(studio_ui::palette::send_cmds(&[("cut", "ajouter au montage (sélection ou tout)")]).into_iter().map(|command| command.enabled_if(has_audio, "Ouvre ou enregistre d'abord un son")));
        for (k, d) in effects::DEFS.iter().enumerate() {
            v.push(studio_ui::palette::Cmd::new(format!("Effet audio : {}", d.name), effects::CATS[d.cat], format!("rack:{k}")));
            v.push(studio_ui::palette::Cmd::new(format!("Appliquer : {}", d.name), "Réglages actuels", format!("apply:{k}")).enabled_if(has_audio, "Ouvre ou enregistre d'abord un son"));
        }
        v
    }

    fn run_cmd(&mut self, id: &str, ctx: &egui::Context) {
        if self.run_lib(id, ctx) {
            return;
        }
        if let Some(t) = id.strip_prefix("send:") {
            let t: &'static str = studio_core::handoff::APPS.iter().find(|a| a.0 == t).map_or("pixel", |a| a.0);
            self.send_to(t, ctx);
            return;
        }
        if let Some(k) = id.strip_prefix("rack:").and_then(|k| k.parse::<usize>().ok()).filter(|k| *k < effects::DEFS.len()) {
            self.rack_pick = k;
            self.rack_cat = effects::DEFS[k].cat;
            return;
        }
        if let Some(k) = id.strip_prefix("apply:").and_then(|k| k.parse::<usize>().ok()).filter(|k| *k < effects::DEFS.len()) {
            self.run_rack(k, false);
            return;
        }
        match id {
            "open" => self.open_dialog(),
            "export" => self.export = Some(self.export_opts()),
            "select-all" => self.sel = Some((0, self.clip.len())),
            "play" => self.toggle_play(),
            "marker" => self.add_marker(),
            "silences" | "silences-short" => self.cut_silences(if id == "silences" { 800.0 } else { 400.0 }),
            "normalize-peak" => self.edit("Normalisation à −1 dB", |clip, a, b| clip.normalize(a, b, -1.0)),
            "normalize-lufs" => self.edit("Normalisation à −16 LUFS", |clip, a, b| clip.loudness_normalize(a, b, -16.0)),
            "micro-fades" => self.edit("Micro-fondus de bord", |clip, a, b| {
                let edge = ((clip.rate as f32 * 0.02) as usize).min((b - a) / 2);
                if edge > 0 {
                    clip.fade(a, a + edge, true);
                    clip.fade(b - edge, b, false);
                }
            }),
            "trim-edges" => match self.clip.audible_bounds(-42.0, 10.0) {
                Some((a, b)) if a > 0 || b < self.clip.len() => {
                    self.snapshot("Rogner les silences de bord");
                    self.clip = self.clip.slice(a, b);
                    self.sel = None;
                    self.changed();
                    self.notice = Some((format!("Silences rognés : {:.2} s conservées", self.clip.len() as f32 / self.clip.rate as f32), true));
                }
                Some(_) => self.notice = Some(("Aucun silence de bord à rogner".into(), false)),
                None => self.notice = Some(("Le fichier semble entièrement silencieux".into(), false)),
            },
            "split-files" => {
                let segs = self.clip.segments(-42.0, 350.0, 120.0);
                if segs.len() < 2 {
                    self.notice = Some(("Pas assez de silences pour découper (il faut au moins deux passages)".into(), false));
                } else if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    let stem = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("son".into());
                    let pad = (self.clip.rate as f32 * 0.05) as usize;
                    let mut ok = 0;
                    for (k, (a, b)) in segs.iter().enumerate() {
                        let part = self.clip.slice(a.saturating_sub(pad), (b + pad).min(self.clip.len()));
                        ok += usize::from(part.save_wav(&dir.join(format!("{stem} - {:02}.wav", k + 1)), 16).is_ok());
                    }
                    self.notice = Some((format!("{ok} fichier(s) créés dans {}", dir.display()), ok > 0));
                }
            }
            x if x.starts_with("loop:") => {
                let ms: f32 = x[5..].parse().unwrap_or(500.0);
                let mut c = self.clip.clone();
                let n = c.make_loop((self.clip.rate as f32 * ms / 1000.0) as usize);
                if n == 0 {
                    self.notice = Some(("Son trop court pour une boucle".into(), false));
                } else {
                    self.snapshot("Boucle parfaite");
                    self.clip = c;
                    self.sel = None;
                    self.changed();
                    self.looping = true;
                    self.notice = Some(("Boucle prête : la fin s'enchaîne sur le début (lecture en boucle activée)".into(), true));
                }
            }
            "undo" => self.undo(),
            _ => {}
        }
    }
    fn send_to(&mut self, target: &str, ctx: &egui::Context) {
        let (a, b) = self.range();
        let stem = self.file.as_ref().and_then(|p| p.file_stem()).map_or("audio".into(), |s| s.to_string_lossy().into_owned());
        let p = studio_core::handoff::outgoing("sound", &stem, "wav");
        match dsp::export_wav(&self.clip.slice(a, b), &p, 16, 1, &[], &[]) {
            Ok(()) => studio_ui::handoff::send(ctx, target, &p),
            Err(e) => studio_ui::handoff::toast(ctx, format!("Envoi impossible : {e}"), false),
        }
    }

    fn receive(&mut self, p: &Path) {
        if self.file.is_none() && !self.modified {
            self.open_path(p);
            return;
        }
        match Clip::decode(p) {
            Ok(c) => self.insert_clip(c, "Reçu"),
            Err(e) => self.notice = Some((format!("Fichier reçu illisible : {e}"), false)),
        }
    }
}

impl eframe::App for Sound {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        studio_ui::chrome::resize(ctx);
        for p in self.inbox.poll(ctx) {
            self.receive(&p);
        }
        self.palette.set_library(optlib::lib);
        self.palette.keys(ctx);
        if !self.palette.prefs_declared() {
            self.palette.declare_prefs("sound", reglages::PREFS);
        }
        if self.palette.take_prefs_changed() {
            self.apply_prefs();
        }
        if self.palette.open {
            let cmds = self.commands();
            if let Some(id) = self.palette.show(ctx, &mut self.help, cmds) {
                self.run_cmd(&id, ctx);
            }
        }
        self.keys(ctx);
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf())).filter(|p| !p.as_os_str().is_empty()) {
            self.open_path(&p);
        }
        let name = self.file.as_ref().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = format!("{name}{} — Sound", if self.modified { " •" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        let (playing, raw_pos) = self.player.state();
        if !playing && self.preview.take().is_some() {
            self.player.set_clip(Arc::new(self.clip.clone()));
        }
        let pos = raw_pos + self.preview.unwrap_or(0);
        if playing || self.recorder.is_some() {
            ctx.request_repaint();
        }
        self.top_bar(root);
        self.transport(root, playing, pos);
        self.status(root);
        self.dock(root, pos, playing);
        self.tool_rail(root);
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin::same(12))).show(root, |ui| {
            self.waveform(ui, playing, pos);
        });
        self.sheets(ctx);
        self.help.show(ctx);
    }
}

impl Sound {
    fn top_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(34.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let full = studio_ui::chrome::bar(ui, 12.0);
            let right_w = 320.0;
            let left = Rect::from_min_max(full.min, pos2(full.right() - right_w, full.bottom()));
            ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(22.0))));
                    ui.add_space(4.0);
                    egui::MenuBar::new().ui(ui, |ui| self.menus(ui));
                });
            });
            let right = Rect::from_min_max(pos2(full.right() - right_w, full.top()), full.max);
            ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary_icon(ui, tl("Exporter"), export_glyph).on_hover_text(tl("WAV 16/24/32 bits, sélection ou morceaux · Ctrl+E")).clicked() {
                        self.export = Some(self.export_opts());
                    }
                    ui.add_space(4.0);
                    let chip = format!("{} Hz · {} · {}", self.clip.rate, if self.clip.ch.len() > 1 { "stéréo" } else { "mono" }, fmt_time(self.clip.secs(self.clip.len())));
                    if pro::chip(ui, &chip).on_hover_text(tl("Fréquence d'échantillonnage et canaux")).clicked() {
                        self.props_sheet = true;
                    }
                });
            });
        });
    }

    fn menus(&mut self, ui: &mut Ui) {
        ui.menu_button(tl("Fichier"), |ui| {
            if let Some(t) = studio_ui::handoff::menu(ui, &[("cut", "ajouter au montage (sélection ou tout)")]) {
                let ctx = ui.ctx().clone();
                self.send_to(t, &ctx);
            }
            ui.separator();
            if ui.button(tl("Nouveau (5 s de silence)")).clicked() {
                self.clip = Clip { rate: 44_100, ch: vec![vec![0.0; 44_100 * 5]; 2] };
                self.file = None;
                self.modified = false;
                self.undo.clear();
                self.markers.clear();
                self.fit_pending = true;
                self.changed();
            }
            if ui.button(tl("Ouvrir…                         Ctrl+O")).clicked() {
                self.open_dialog();
            }
            ui.separator();
            if ui.button(tl("Exporter…                       Ctrl+E")).clicked() {
                self.export = Some(self.export_opts());
            }
            if ui.button(tl("Propriétés du son…")).clicked() {
                self.props_sheet = true;
            }
        });
        ui.menu_button(tl("Édition"), |ui| {
            let has = self.sel.is_some();
            let ul = self.undo.last().map(|u| u.1.clone()).unwrap_or_default();
            let rl = self.redo.last().map(|u| u.1.clone()).unwrap_or_default();
            if ui.add_enabled(!self.undo.is_empty(), egui::Button::new(format!("Annuler {ul}"))).clicked() {
                self.undo();
            }
            if ui.add_enabled(!self.redo.is_empty(), egui::Button::new(format!("Rétablir {rl}"))).clicked() {
                self.redo();
            }
            ui.separator();
            for (label, k) in [("Couper                     Ctrl+X", 0), ("Copier                     Ctrl+C", 1), ("Dupliquer               Ctrl+D", 2), ("Rogner                     Ctrl+T", 3), ("Supprimer               Suppr", 4)] {
                if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                    match k {
                        0 => self.cut(true),
                        1 => {
                            if let Some((a, b)) = self.sel {
                                self.clipboard = Some(self.clip.slice(a, b));
                            }
                        }
                        2 => self.duplicate(),
                        3 => self.trim(),
                        _ => self.cut(false),
                    }
                }
            }
            if ui.add_enabled(self.clipboard.is_some(), egui::Button::new("Coller                      Ctrl+V")).clicked() {
                self.paste();
            }
            ui.separator();
            if ui.button(tl("Tout sélectionner         Ctrl+A")).clicked() {
                self.sel = Some((0, self.clip.len()));
            }
            if ui.button(tl("Du curseur au début     Maj+Origine")).clicked() {
                self.sel = Some((0, self.cursor.max(1)));
            }
            if ui.button(tl("Du curseur à la fin       Maj+Fin")).clicked() {
                self.sel = Some((self.cursor, self.clip.len()));
            }
            ui.separator();
            if ui.button(tl("Ajouter / retirer un marqueur   M")).clicked() {
                self.add_marker();
            }
            if ui.add_enabled(!self.markers.is_empty(), egui::Button::new("Supprimer tous les marqueurs")).clicked() {
                self.markers.clear();
            }
        });
        ui.menu_button(tl("Effets"), |ui| {
            for (label, k) in [("Normaliser (−1 dB)", 0), ("Volume perçu −16 LUFS", 1), ("Limiteur −1 dB", 2), ("Volume moyen (RMS −18 dB)", 12), ("Fondu entrant", 3), ("Fondu sortant", 4), ("Coupe-bas 80 Hz", 5), ("Supprimer le décalage (DC)", 6), ("À l'envers", 7), ("Inverser la polarité", 8), ("Rendre silencieux", 9), ("Convertir en mono", 10), ("Échanger gauche et droite", 11)] {
                if k == 3 || k == 5 || k == 7 || k == 10 {
                    ui.separator();
                }
                if ui.button(label).clicked() {
                    let shape = self.fx.fade_shape as u8;
                    self.edit(label, move |c, a, b| match k {
                        0 => c.normalize(a, b, -1.0),
                        1 => c.loudness_normalize(a, b, -16.0),
                        2 => c.limiter(a, b, -1.0),
                        12 => c.rms_normalize(a, b, -18.0),
                        3 => c.fade_shape(a, b, true, shape),
                        4 => c.fade_shape(a, b, false, shape),
                        5 => c.filter(a, b, 4, 80.0, 0.0, 0.707),
                        6 => c.remove_dc(a, b),
                        7 => c.reverse(a, b),
                        8 => c.invert(a, b),
                        9 => c.silence(a, b),
                        10 => c.to_mono(),
                        _ => c.swap(),
                    });
                }
            }
            ui.separator();
            ui.menu_button(tl("Préréglages"), |ui| {
                for (k, (name, _)) in PRESETS.iter().enumerate() {
                    if ui.button(*name).clicked() {
                        self.apply_preset(k);
                    }
                }
            });
        });
        ui.menu_button(tl("Générer"), |ui| {
            let secs = self.fx.secs;
            for (label, k) in [("Silence", 0u8), ("Tonalité 440 Hz", 1), ("Bruit blanc", 2)] {
                if ui.button(format!("{label} ({secs:.1} s) au curseur")).clicked() {
                    let mut part = Clip { rate: self.clip.rate, ch: vec![Vec::new(); self.clip.ch.len().max(1)] };
                    part.generate(0, secs, k);
                    self.insert_clip(part, label);
                }
            }
            ui.add(egui::Slider::new(&mut self.fx.secs, 0.1..=120.0).logarithmic(true).suffix(" s"));
            ui.label(RichText::new(tl("Plus de sons dans le panneau Générer")).size(11.5).color(FAINT));
        });
        ui.menu_button(tl("Affichage"), |ui| {
            ui.menu_button(tl("Espace de travail"), studio_ui::dock::menu);
            ui.separator();
            if ui.button(tl("Tout afficher               Ctrl+0")).clicked() {
                self.fit_pending = true;
            }
            if ui.add_enabled(self.sel.is_some(), egui::Button::new("Zoom sur la sélection    Z")).clicked() {
                self.zoom_sel();
            }
            if ui.button(tl("Zoom avant                   Ctrl++")).clicked() {
                self.zoom_by(0.5);
            }
            if ui.button(tl("Zoom arrière                Ctrl+−")).clicked() {
                self.zoom_by(2.0);
            }
            ui.separator();
            let mut spec = self.view_mode == 1;
            if ui.checkbox(&mut spec, tl("Spectrogramme                S")).changed() {
                self.view_mode = spec as usize;
            }
            ui.checkbox(&mut self.follow, tl("Suivre la lecture              F"));
            ui.checkbox(&mut self.looping, tl("Lecture en boucle            L"));
        });
        self.help.menu(ui);
    }

    fn transport(&mut self, root: &mut Ui, playing: bool, pos: usize) {
        egui::Panel::top("transport").exact_size(46.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let small = |ui: &mut Ui, kind: u8, tip: &str, on: bool| -> bool {
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                    ui.painter().rect_filled(r, CornerRadius::same(3), if on { ACCENT.gamma_multiply(0.35) } else if resp.hovered() { FIELD } else { Color32::TRANSPARENT });
                    transport_glyph(ui.painter(), r.center(), kind, if on { INK_STRONG } else { INK });
                    resp.on_hover_text(tip).clicked()
                };
                if small(ui, 0, "Début (Origine)", false) {
                    self.cursor = 0;
                    self.sel = None;
                    self.view_start = 0.0;
                }
                if small(ui, 1, "Marqueur précédent (↑)", false) {
                    self.jump(-1);
                }
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
                ui.painter().circle_filled(r.center(), 15.0, if resp.hovered() { ACCENT.lerp_to_gamma(Color32::WHITE, 0.12) } else { ACCENT });
                let c = r.center();
                if playing {
                    for dx in [-4.5, 4.5] {
                        ui.painter().rect_filled(Rect::from_center_size(c + vec2(dx * 0.75, 0.0), vec2(3.5, 11.0)), CornerRadius::same(1), Color32::WHITE);
                    }
                } else {
                    ui.painter().add(Shape::convex_polygon(vec![c + vec2(-3.5, -6.5), c + vec2(6.5, 0.0), c + vec2(-3.5, 6.5)], Color32::WHITE, Stroke::NONE));
                }
                if resp.on_hover_text(tl("Lecture / pause (Espace)")).clicked() {
                    self.toggle_play();
                }
                if small(ui, 2, "Stop", false) {
                    self.player.stop();
                }
                if small(ui, 3, "Marqueur suivant (↓)", false) {
                    self.jump(1);
                }
                if small(ui, 4, "Fin", false) {
                    self.cursor = self.clip.len();
                    self.sel = None;
                    self.reveal(self.cursor);
                }
                let rec = self.recorder.is_some();
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                ui.painter().rect_filled(r, CornerRadius::same(3), if rec { RED.gamma_multiply(0.3) } else if resp.hovered() { FIELD } else { Color32::TRANSPARENT });
                ui.painter().circle_filled(r.center(), if rec { 4.5 + (ui.input(|i| i.time) * 4.0).sin().abs() as f32 * 1.5 } else { 5.0 }, RED);
                if resp.on_hover_text(tl("Enregistrer au micro au curseur (R)")).clicked() {
                    self.toggle_record();
                }
                if small(ui, 5, "Boucle sur la sélection (L)", self.looping) {
                    self.looping = !self.looping;
                }
                ui.add_space(12.0);
                let shown = if playing { pos } else { self.cursor };
                ui.vertical(|ui| {
                    ui.add_space(5.0);
                    ui.label(RichText::new(fmt_time(self.clip.secs(shown))).font(FontId::monospace(17.0)).color(INK_STRONG));
                    ui.label(RichText::new(format!("sur {}", fmt_time(self.clip.secs(self.clip.len())))).monospace().size(10.0).color(MUTED));
                });
                ui.add_space(12.0);
                if let Some(r) = &self.recorder {
                    ui.label(RichText::new(format!("● Enregistrement {:.1} s", r.seconds())).font(semibold(12.5)).color(RED));
                } else {
                    let rate = self.clip.rate as f64;
                    let n = self.clip.len();
                    let (mut a, mut b) = self.sel.map_or((self.cursor as f64 / rate, self.cursor as f64 / rate), |(a, b)| (a as f64 / rate, b as f64 / rate));
                    ui.label(RichText::new(tl("Sélection")).size(12.0).color(MUTED));
                    let ra = ui.add(egui::DragValue::new(&mut a).range(0.0..=n as f64 / rate).speed(0.01).max_decimals(3).suffix(" s"));
                    let rb = ui.add(egui::DragValue::new(&mut b).range(0.0..=n as f64 / rate).speed(0.01).max_decimals(3).suffix(" s"));
                    if ra.changed() || rb.changed() {
                        let (x, y) = (((a * rate) as usize).min(n), ((b * rate) as usize).min(n));
                        self.sel = if y > x { Some((x, y)) } else { None };
                        self.cursor = x;
                    }
                    if let Some((a, b)) = self.sel {
                        ui.label(RichText::new(format!("durée {}", fmt_time(self.clip.secs(b - a)))).size(12.0).color(WAVE));
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let levels: Vec<f32> = if let Some(r) = &self.recorder {
                        vec![r.level(); 2]
                    } else if playing {
                        let a = pos.saturating_sub(1024).min(self.clip.len());
                        let b = (pos + 1024).min(self.clip.len());
                        self.clip.ch.iter().map(|c| c[a..b].iter().fold(0f32, |m, v| m.max(v.abs()))).collect()
                    } else {
                        vec![0.0; self.clip.ch.len()]
                    };
                    meters(ui, &levels);
                });
            });
        });
    }

    fn status(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(22.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                match self.notice.clone() {
                    Some((n, ok)) => {
                        ui.label(RichText::new(n).size(12.0).color(if ok { GREEN } else { RED }));
                        if ui.small_button("×").clicked() {
                            self.notice = None;
                        }
                    }
                    None => {
                        pro::shortcuts(ui, "Glisser : sélection · bords : ajuster · molette : zoom · Maj+molette : défiler · double-clic : entre marqueurs · M marqueur · Z zoom sélection · S spectrogramme");
                    }
                }
            });
        });
    }

    fn dock(&mut self, root: &mut Ui, pos: usize, playing: bool) {
        egui::Panel::right("fx").resizable(false).exact_size(304.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                studio_ui::dock::begin(ui, "sound");
                let scope = if self.sel.is_some() { "à la sélection" } else { "à tout le fichier" };
                let at = if playing { pos } else { self.sel.map_or(self.cursor, |s| s.0) };
                if self.spectrum.as_ref().is_none_or(|s| s.0.abs_diff(at) > (self.clip.rate / 10) as usize || s.1 != self.version) {
                    self.spectrum = Some((at, self.version, self.clip.spectrum(at, 96)));
                }
                let (ra, rb) = self.range();
                if self.stats.as_ref().is_none_or(|s| s.key != (self.version, ra, rb)) && rb > ra {
                    self.stats = Some(Stats { key: (self.version, ra, rb), peak: db(self.clip.peak(ra, rb)), rms: self.clip.rms_db(ra, rb), lufs: self.clip.loudness(ra, rb), clipped: self.clip.clipped(ra, rb), true_peak: db(self.clip.true_peak(ra, rb)) });
                }
                if let Some((_, _, spec)) = &self.spectrum {
                    let spec = spec.clone();
                    let stats = self.stats.as_ref().map(|s| (s.peak, s.rms, s.lufs, s.clipped, s.true_peak));
                    pro::dock_panel(ui, "analysis", tl("Analyse"), |ui| {
                        draw_spectrum(ui, &spec);
                        if let Some((p, r, l, c, tp)) = stats {
                            egui::Grid::new("stats").num_columns(4).spacing(vec2(6.0, 4.0)).show(ui, |ui| {
                                for (k, v, col) in [("Crête", format!("{p:.1} dB"), if p > -0.1 { RED } else { INK }), ("RMS", format!("{r:.1} dB"), INK)] {
                                    ui.label(RichText::new(k).size(11.5).color(MUTED));
                                    ui.label(RichText::new(v).size(12.0).color(col).monospace());
                                }
                                ui.end_row();
                                ui.label(RichText::new(tl("Volume")).size(11.5).color(MUTED));
                                ui.label(RichText::new(format!("{l:.1} LUFS")).size(12.0).color(INK).monospace());
                                ui.label(RichText::new(tl("Écrêtés")).size(11.5).color(MUTED));
                                ui.label(RichText::new(c.to_string()).size(12.0).color(if c > 0 { RED } else { GREEN }).monospace());
                                ui.end_row();
                                ui.label(RichText::new(tl("Crête réelle")).size(11.5).color(MUTED));
                                ui.label(RichText::new(format!("{tp:.1} dBTP")).size(12.0).color(if tp > -1.0 { RED } else { INK }).monospace());
                                ui.end_row();
                            });
                        }
                    });
                }
                self.rack(ui);
                self.analysis_tools(ui);
                let mut action: Option<(bool, String, Op)> = None;
                let mut preset = None;
                let mut capture = false;
                let mut generated: Option<(Clip, String)> = None;
                let mut truncate = false;
                let has_noise = self.noise.is_some();
                let rate = self.clip.rate;
                let nch = self.clip.ch.len().max(1);
                let fx = &mut self.fx;
                macro_rules! act {
                    ($ui:expr, $label:expr, $op:expr) => {
                        match fx_buttons($ui) {
                            Some(p) => action = Some((p, $label.to_string(), Box::new($op))),
                            None => {}
                        }
                    };
                }
                pro::dock_panel(ui, "presets", tl("Préréglages"), |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(RichText::new(format!("Un clic, appliqué {scope}")).size(11.5).color(FAINT));
                    ui.add_space(4.0);
                    for (k, (name, hint)) in PRESETS.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(r, CornerRadius::same(4), Color32::from_white_alpha(10));
                        }
                        ui.painter().circle_filled(r.left_center() + vec2(12.0, 0.0), 4.0, WAVE);
                        ui.painter().text(r.left_top() + vec2(24.0, 3.0), Align2::LEFT_TOP, *name, FontId::proportional(12.5), INK);
                        ui.painter().text(r.left_top() + vec2(24.0, 20.0), Align2::LEFT_TOP, *hint, FontId::proportional(11.0), FAINT);
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            preset = Some(k);
                        }
                    }
                });
                pro::dock_panel(ui, "amp", tl("Volume"), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.label(RichText::new(format!("Appliqué {scope}")).size(11.5).color(FAINT));
                    slider(ui, "Gain", &mut fx.gain, -24.0..=24.0, " dB");
                    let g = fx.gain;
                    act!(ui, "Gain", move |c: &mut Clip, a, b| c.gain_db(a, b, g));
                    slider(ui, "Normaliser la crête à", &mut fx.peak, -12.0..=0.0, " dB");
                    let pk = fx.peak;
                    act!(ui, "Normalisation", move |c: &mut Clip, a, b| c.normalize(a, b, pk));
                    ui.label(RichText::new(tl("Volume perçu (LUFS)")).size(12.0).color(MUTED));
                    let labels: Vec<&str> = LUFS.iter().map(|l| l.0).collect();
                    pro::segmented(ui, &mut fx.lufs, &labels);
                    ui.label(RichText::new(["Spotify, YouTube, réseaux", "Podcasts", "Voix posée, livres audio", "Télévision (EBU R128)"][fx.lufs]).size(11.5).color(FAINT));
                    let target = LUFS[fx.lufs].1;
                    act!(ui, "Volume perçu", move |c: &mut Clip, a, b| c.loudness_normalize(a, b, target));
                    slider(ui, "Limiteur (plafond)", &mut fx.ceiling, -12.0..=0.0, " dB");
                    let ce = fx.ceiling;
                    act!(ui, "Limiteur", move |c: &mut Clip, a, b| c.limiter(a, b, ce));
                    ui.label(RichText::new(tl("Courbe des fondus")).size(12.0).color(MUTED));
                    pro::segmented(ui, &mut fx.fade_shape, &[tl("Linéaire"), tl("Log"), tl("En S")]);
                    let shape = fx.fade_shape as u8;
                    ui.horizontal(|ui| {
                        if pro::secondary(ui, tl("Fondu entrant")).clicked() {
                            action = Some((false, "Fondu entrant".into(), Box::new(move |c: &mut Clip, a, b| c.fade_shape(a, b, true, shape))));
                        }
                        if pro::secondary(ui, tl("Fondu sortant")).clicked() {
                            action = Some((false, "Fondu sortant".into(), Box::new(move |c: &mut Clip, a, b| c.fade_shape(a, b, false, shape))));
                        }
                    });
                });
                pro::dock_panel(ui, "clean", tl("Nettoyage"), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.label(RichText::new(tl("Réduction de bruit")).size(12.0).color(INK));
                    ui.label(RichText::new(tl("1. Sélectionne un passage où l'on n'entend que le bruit, 2. capture-le, 3. sélectionne tout et applique.")).size(11.5).color(FAINT));
                    if pro::secondary(ui, if has_noise { "✓ Profil capturé · recapturer" } else { "Capturer le profil de bruit" }).clicked() {
                        capture = true;
                    }
                    slider(ui, "Intensité", &mut fx.denoise, 0.1..=1.0, "");
                    if has_noise {
                        let amt = fx.denoise;
                        let prof = self.noise.clone().unwrap_or_default();
                        act!(ui, "Réduction de bruit", move |c: &mut Clip, a, b| c.denoise(a, b, &prof, amt));
                    }
                    ui.add_space(4.0);
                    slider(ui, "Porte de bruit", &mut fx.gate, -80.0..=-10.0, " dB");
                    let gt = fx.gate;
                    act!(ui, "Porte de bruit", move |c: &mut Clip, a, b| c.gate(a, b, gt));
                    slider(ui, "Coupe-bas", &mut fx.low_cut, 20.0..=500.0, " Hz");
                    slider(ui, "Coupe-haut", &mut fx.high_cut, 1000.0..=20_000.0, " Hz");
                    let (lo, hi) = (fx.low_cut, fx.high_cut);
                    act!(ui, "Filtres coupe-bas / coupe-haut", move |c: &mut Clip, a, b| {
                        c.filter(a, b, 4, lo, 0.0, 0.707);
                        c.filter(a, b, 3, hi, 0.0, 0.707);
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new(tl("Supprimer les silences")).size(12.0).color(INK));
                    slider(ui, "Seuil", &mut fx.silence[0], -70.0..=-20.0, " dB");
                    slider(ui, "Pauses de plus de", &mut fx.silence[1], 100.0..=3000.0, " ms");
                    slider(ui, "Garder", &mut fx.silence[2], 0.0..=1000.0, " ms");
                    if pro::secondary(ui, tl("Raccourcir les blancs")).clicked() {
                        truncate = true;
                    }
                });
                pro::dock_panel(ui, "eq", tl("Égaliseur 5 bandes"), |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    eq_curve(ui, &fx.eq);
                    for (k, (_, name)) in EQ_BANDS.iter().enumerate() {
                        slider(ui, name, &mut fx.eq[k], -15.0..=15.0, " dB");
                    }
                    let eq = fx.eq;
                    ui.horizontal_wrapped(|ui| {
                        for (label, preset) in [("Plat", [0.0; 5]), ("Voix", [-6.0, -2.0, 1.0, 4.0, 2.0]), ("Basses", [6.0, 2.0, 0.0, 0.0, 0.0]), ("Brillant", [0.0, 0.0, 0.0, 3.0, 6.0])] {
                            if ui.small_button(label).clicked() {
                                fx.eq = preset;
                            }
                        }
                    });
                    act!(ui, "Égaliseur", move |c: &mut Clip, a, b| {
                        for (k, (f, _)) in EQ_BANDS.iter().enumerate() {
                            if eq[k] != 0.0 {
                                let kind = match k {
                                    0 => 1,
                                    4 => 2,
                                    _ => 0,
                                };
                                c.filter(a, b, kind, *f, eq[k], 0.9);
                            }
                        }
                    });
                });
                pro::dock_panel(ui, "dyn", tl("Dynamique"), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    slider(ui, "Seuil", &mut fx.comp[0], -60.0..=0.0, " dB");
                    slider(ui, "Ratio", &mut fx.comp[1], 1.0..=20.0, ":1");
                    slider(ui, "Gain de sortie", &mut fx.comp[2], 0.0..=24.0, " dB");
                    let cp = fx.comp;
                    act!(ui, "Compresseur", move |c: &mut Clip, a, b| c.compress(a, b, cp[0], cp[1], cp[2]));
                    slider(ui, "Saturation", &mut fx.drive, 0.0..=1.0, "");
                    let dr = fx.drive;
                    act!(ui, "Saturation", move |c: &mut Clip, a, b| c.saturate(a, b, dr));
                    slider(ui, "Bits", &mut fx.crush[0], 2.0..=16.0, "");
                    slider(ui, "Sous-échantillonnage", &mut fx.crush[1], 1.0..=20.0, "×");
                    let cr = fx.crush;
                    act!(ui, "Bitcrusher", move |c: &mut Clip, a, b| c.bitcrush(a, b, cr[0], cr[1] as usize));
                });
                pro::dock_panel(ui, "mod", tl("Modulation"), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.label(RichText::new(tl("Chorus")).size(12.0).color(INK));
                    slider(ui, "Profondeur", &mut fx.chorus[0], 1.0..=20.0, " ms");
                    slider(ui, "Vitesse", &mut fx.chorus[1], 0.1..=5.0, " Hz");
                    slider(ui, "Mélange", &mut fx.chorus[2], 0.0..=1.0, "");
                    let ch = fx.chorus;
                    act!(ui, "Chorus", move |c: &mut Clip, a, b| c.mod_delay(a, b, 20.0, ch[0], ch[1], 0.0, ch[2]));
                    ui.label(RichText::new(tl("Flanger")).size(12.0).color(INK));
                    slider(ui, "Vitesse", &mut fx.flanger[0], 0.05..=3.0, " Hz");
                    slider(ui, "Profondeur", &mut fx.flanger[1], 0.5..=8.0, " ms");
                    slider(ui, "Réinjection", &mut fx.flanger[2], 0.0..=0.9, "");
                    let fl = fx.flanger;
                    act!(ui, "Flanger", move |c: &mut Clip, a, b| c.mod_delay(a, b, 1.5, fl[1], fl[0], fl[2], 0.7));
                    ui.label(RichText::new(tl("Trémolo")).size(12.0).color(INK));
                    slider(ui, "Vitesse", &mut fx.trem[0], 0.5..=20.0, " Hz");
                    slider(ui, "Profondeur", &mut fx.trem[1], 0.0..=1.0, "");
                    let tr = fx.trem;
                    act!(ui, "Trémolo", move |c: &mut Clip, a, b| c.tremolo(a, b, tr[0], tr[1]));
                    ui.label(RichText::new(tl("Panoramique automatique")).size(12.0).color(INK));
                    slider(ui, "Vitesse", &mut fx.autopan[0], 0.05..=8.0, " Hz");
                    slider(ui, "Amplitude", &mut fx.autopan[1], 0.0..=1.0, "");
                    let ap = fx.autopan;
                    act!(ui, "Panoramique automatique", move |c: &mut Clip, a, b| c.auto_pan(a, b, ap[0], ap[1]));
                    slider(ui, "Modulation en anneau (robot)", &mut fx.ring, 10.0..=1000.0, " Hz");
                    let rf = fx.ring;
                    act!(ui, "Modulation en anneau", move |c: &mut Clip, a, b| c.ring(a, b, rf));
                });
                pro::dock_panel(ui, "space", tl("Espace"), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    slider(ui, "Délai d'écho", &mut fx.echo[0], 20.0..=1500.0, " ms");
                    slider(ui, "Réinjection", &mut fx.echo[1], 0.0..=0.9, "");
                    slider(ui, "Mélange", &mut fx.echo[2], 0.0..=1.0, "");
                    let e = fx.echo;
                    act!(ui, "Écho", move |c: &mut Clip, a, b| c.echo(a, b, e[0], e[1], e[2]));
                    slider(ui, "Taille de la pièce", &mut fx.verb[0], 0.0..=1.0, "");
                    slider(ui, "Mélange réverb", &mut fx.verb[1], 0.0..=1.0, "");
                    let v = fx.verb;
                    act!(ui, "Réverbération", move |c: &mut Clip, a, b| c.reverb(a, b, v[0], v[1]));
                });
                pro::dock_panel(ui, "time", tl("Temps et hauteur"), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    slider(ui, "Vitesse (comme une bande)", &mut fx.speed, 25.0..=400.0, " %");
                    let sp = fx.speed;
                    act!(ui, "Vitesse", move |c: &mut Clip, a, b| c.speed(a, b, sp / 100.0));
                    slider(ui, "Tempo (même hauteur)", &mut fx.tempo, 50.0..=200.0, " %");
                    let tp = fx.tempo;
                    act!(ui, "Tempo", move |c: &mut Clip, a, b| c.stretch(a, b, 100.0 / tp));
                    slider(ui, "Hauteur en demi-tons (même durée)", &mut fx.semis, -12.0..=12.0, "");
                    let st = fx.semis;
                    act!(ui, "Hauteur", move |c: &mut Clip, a, b| c.pitch(a, b, st));
                });
                pro::dock_panel(ui, "stereo", tl("Stéréo"), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    slider(ui, "Balance", &mut fx.pan, -1.0..=1.0, "");
                    slider(ui, "Largeur stéréo", &mut fx.width, 0.0..=200.0, " %");
                    let (pn, wd) = (fx.pan, fx.width);
                    act!(ui, "Stéréo", move |c: &mut Clip, a, b| c.stereo(a, b, pn, wd / 100.0));
                    ui.horizontal(|ui| {
                        if pro::ghost(ui, tl("Mono")).clicked() {
                            action = Some((false, "Mono".into(), Box::new(|c: &mut Clip, _, _| c.to_mono())));
                        }
                        if pro::ghost(ui, tl("Échanger G/D")).clicked() {
                            action = Some((false, "Échange gauche/droite".into(), Box::new(|c: &mut Clip, _, _| c.swap())));
                        }
                    });
                });
                pro::dock_panel(ui, "gen", tl("Générer"), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.label(RichText::new(tl("Inséré au curseur")).size(11.5).color(FAINT));
                    slider(ui, "Durée", &mut fx.secs, 0.1..=120.0, " s");
                    slider(ui, "Volume", &mut fx.amp, 0.0..=1.0, "");
                    ui.label(RichText::new(tl("Tonalité")).size(12.0).color(INK));
                    pro::segmented(ui, &mut fx.wave, &WAVES);
                    slider(ui, "Fréquence", &mut fx.freq, 20.0..=10_000.0, " Hz");
                    if pro::secondary(ui, tl("Insérer la tonalité")).clicked() {
                        generated = Some((Clip::synth(rate, nch, fx.secs, fx.wave as u8, fx.freq, 0.0, fx.amp), WAVES[fx.wave].to_string()));
                    }
                    ui.label(RichText::new(tl("Bruit")).size(12.0).color(INK));
                    pro::segmented(ui, &mut fx.noise_kind, &[tl("Blanc"), tl("Rose"), tl("Brun")]);
                    if pro::secondary(ui, tl("Insérer le bruit")).clicked() {
                        let c = if fx.noise_kind == 0 {
                            let mut c = Clip { rate, ch: vec![Vec::new(); nch] };
                            c.generate(0, fx.secs, 2);
                            let n = c.len();
                            c.gain_db(0, n, db(fx.amp / 0.3));
                            c
                        } else {
                            Clip::synth(rate, nch, fx.secs, 3 + fx.noise_kind as u8, 0.0, 0.0, fx.amp)
                        };
                        generated = Some((c, "Bruit".into()));
                    }
                    ui.label(RichText::new(tl("Balayage")).size(12.0).color(INK));
                    slider(ui, "Jusqu'à", &mut fx.freq2, 20.0..=20_000.0, " Hz");
                    if pro::secondary(ui, tl("Insérer un balayage")).clicked() {
                        generated = Some((Clip::synth(rate, nch, fx.secs, 6, fx.freq, fx.freq2, fx.amp), "Balayage".into()));
                    }
                    ui.label(RichText::new(tl("Métronome")).size(12.0).color(INK));
                    slider(ui, "Tempo", &mut fx.bpm, 30.0..=240.0, " BPM");
                    if pro::secondary(ui, tl("Insérer le métronome")).clicked() {
                        generated = Some((Clip::synth(rate, nch, fx.secs, 7, fx.bpm, 0.0, fx.amp), "Métronome".into()));
                    }
                });
                self.markers_panel(ui);
                if let Some(k) = preset {
                    self.apply_preset(k);
                }
                if capture {
                    let (a, b) = self.range();
                    if b - a < 2048 {
                        self.notice = Some(("Sélectionne au moins un dixième de seconde de bruit seul".into(), false));
                    } else {
                        self.noise = Some(self.clip.noise_profile(a, b));
                        self.notice = Some(("Profil de bruit capturé : sélectionne tout puis Appliquer".into(), true));
                    }
                }
                if truncate {
                    let s = self.fx.silence;
                    let (a, b) = self.range();
                    self.snapshot("Suppression des silences");
                    let removed = self.clip.truncate_silence(a, b, s[0], s[1], s[2]);
                    self.modified = true;
                    self.sel = None;
                    self.changed();
                    self.notice = Some((format!("{:.1} s de blancs retirés", removed as f64 / self.clip.rate as f64), true));
                }
                if let Some((c, label)) = generated {
                    self.insert_clip(c, &label);
                }
                if let Some((preview, label, f)) = action {
                    if preview {
                        self.preview_fx(f);
                    } else {
                        let whole = self.sel.is_none();
                        self.edit(&label, f);
                        if whole {
                            self.fit_pending = true;
                        }
                    }
                }
                studio_ui::dock::end(ui);
            });
        });
    }

    fn analysis_tools(&mut self, ui: &mut Ui) {
        let mut bpm = false;
        let mut regions = false;
        let mut beats = false;
        pro::dock_panel(ui, "detect", tl("Détection"), |ui| {
            ui.spacing_mut().item_spacing.y = 5.0;
            ui.horizontal(|ui| {
                if pro::secondary(ui, tl("Détecter le tempo")).on_hover_text(tl("Analyse la sélection ou tout le fichier")).clicked() {
                    bpm = true;
                }
                if let Some((b, _)) = self.bpm {
                    ui.label(RichText::new(format!("{b:.1} BPM")).font(semibold(12.5)).color(WAVE));
                }
            });
            if self.bpm.is_some() && pro::ghost(ui, tl("Marqueur sur chaque mesure")).clicked() {
                beats = true;
            }
            ui.add_space(4.0);
            ui.label(RichText::new(tl("Repérer les sons (un marqueur par prise)")).size(12.0).color(INK));
            slider(ui, "Seuil du silence", &mut self.regions[0], -70.0..=-10.0, " dB");
            slider(ui, "Silence minimum", &mut self.regions[1], 50.0..=3000.0, " ms");
            slider(ui, "Son minimum", &mut self.regions[2], 10.0..=2000.0, " ms");
            if pro::secondary(ui, tl("Poser les marqueurs")).clicked() {
                regions = true;
            }
        });
        let (a, b) = self.range();
        if bpm {
            self.bpm = self.clip.detect_bpm(a, b);
            self.notice = Some(match self.bpm {
                Some((v, _)) => {
                    self.fx.bpm = v.round();
                    (format!("Tempo estimé : {v:.1} BPM"), true)
                }
                None => ("Passage trop court ou sans rythme net (il faut au moins 4 s)".into(), false),
            });
        }
        if beats && let Some((v, off)) = self.bpm {
            let bar = self.clip.rate as f64 * 60.0 / v as f64 * 4.0;
            let mut t = off as f64;
            let mut k = 1;
            while (t as usize) < b {
                if t as usize >= a {
                    self.markers.push((t as usize, format!("Mesure {k}")));
                    k += 1;
                }
                t += bar;
            }
            self.markers.sort_by_key(|m| m.0);
            self.markers.dedup_by_key(|m| m.0);
        }
        if regions {
            let r = self.clip.sound_regions(a, b, self.regions[0], self.regions[1], self.regions[2]);
            let n = r.len();
            for (k, g) in r.into_iter().enumerate() {
                self.markers.push((g.a, format!("Prise {}", k + 1)));
            }
            self.markers.sort_by_key(|m| m.0);
            self.notice = Some((format!("{n} sons repérés"), n > 0));
        }
    }

    fn tool_rail(&mut self, root: &mut Ui) {
        egui::Panel::left("tools").resizable(false).exact_size(52.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(7, 10))).show(root, |ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            let mut cur = self.tool;
            for (k, (g, _)) in tools::GROUPS.iter().enumerate() {
                if k == 2 || k == 4 {
                    let (s, _) = ui.allocate_exact_size(vec2(36.0, 7.0), Sense::hover());
                    ui.painter().line_segment([s.left_center() + vec2(6.0, 0.0), s.right_center() - vec2(6.0, 0.0)], Stroke::new(1.0, LINE));
                }
                pro::tool_group(ui, 36.0, &mut cur, g, |p, r, c, t| tools::draw_icon(p, r, c, t));
            }
            if cur != self.tool {
                self.set_tool(cur);
            }
        });
    }

    fn rack(&mut self, ui: &mut Ui) {
        let scope = if self.sel.is_some() { "à la sélection" } else { "à tout le fichier" };
        let mut run = None;
        pro::dock_panel(ui, "rack", &format!("Effets · {}", effects::DEFS.len()), |ui| {
            ui.spacing_mut().item_spacing.y = 5.0;
            ui.add(egui::TextEdit::singleline(&mut self.rack_search).hint_text(tl("Rechercher un effet…")).desired_width(f32::INFINITY));
            let q = self.rack_search.to_lowercase();
            if q.is_empty() {
                egui::ComboBox::from_id_salt("rack-cat").width(ui.available_width()).selected_text(effects::CATS[self.rack_cat]).show_ui(ui, |ui| {
                    for (k, c) in effects::CATS.iter().enumerate() {
                        let n = effects::DEFS.iter().filter(|d| d.cat == k).count();
                        if ui.selectable_label(self.rack_cat == k, format!("{c}  ({n})")).clicked() {
                            self.rack_cat = k;
                            if let Some(i) = effects::DEFS.iter().position(|d| d.cat == k) {
                                self.rack_pick = i;
                            }
                        }
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for (k, d) in effects::DEFS.iter().enumerate() {
                    let show = if q.is_empty() { d.cat == self.rack_cat } else { d.name.to_lowercase().contains(&q) || d.hint.to_lowercase().contains(&q) };
                    if show && ui.selectable_label(self.rack_pick == k, RichText::new(d.name).size(12.0)).on_hover_text(d.hint).clicked() {
                        self.rack_pick = k;
                        self.rack_cat = d.cat;
                    }
                }
            });
            let k = self.rack_pick;
            let d = &effects::DEFS[k];
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(d.name).font(semibold(12.5)).color(INK_STRONG));
                pro::tip(ui, d.hint);
            });
            if !d.presets.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                    for (name, vals) in d.presets {
                        if pro::chip(ui, name).clicked() {
                            self.rack_vals[k] = vals.to_vec();
                        }
                    }
                });
            }
            let vals = &mut self.rack_vals[k];
            for (i, prm) in d.params.iter().enumerate() {
                ui.label(RichText::new(prm.name).size(12.0).color(MUTED));
                if !prm.choices.is_empty() {
                    let mut c = (vals[i].round() as usize).min(prm.choices.len() - 1);
                    if prm.choices.len() <= 3 {
                        if pro::segmented(ui, &mut c, prm.choices) {
                            vals[i] = c as f32;
                        }
                    } else {
                        egui::ComboBox::from_id_salt(("rack", k, i)).width(ui.available_width()).selected_text(prm.choices[c]).show_ui(ui, |ui| {
                            for (j, ch) in prm.choices.iter().enumerate() {
                                if ui.selectable_label(c == j, *ch).clicked() {
                                    vals[i] = j as f32;
                                }
                            }
                        });
                    }
                } else {
                    ui.spacing_mut().slider_width = (ui.available_width() - 118.0).max(80.0);
                    let sl = egui::Slider::new(&mut vals[i], prm.min..=prm.max).suffix(prm.unit).logarithmic(prm.log);
                    ui.add(if prm.int { sl.integer() } else { sl.max_decimals(2) });
                }
            }
            ui.horizontal(|ui| {
                if ui.small_button(tl("Valeurs par défaut")).clicked() {
                    *vals = effects::defaults(k);
                }
                ui.label(RichText::new(format!("appliqué {scope}")).size(11.0).color(FAINT));
            });
            run = fx_buttons(ui);
        });
        if let Some(p) = run {
            self.run_rack(self.rack_pick, p);
        }
    }

    fn markers_panel(&mut self, ui: &mut Ui) {
        let rate = self.clip.rate as f64;
        pro::dock_panel(ui, "markers", tl("Marqueurs"), |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            if self.markers.is_empty() {
                studio_ui::pro::tip(ui, tl("M pose un marqueur au curseur. Ils servent à naviguer (↑ ↓) et à découper l'export en morceaux."));
            }
            let mut go = None;
            let mut remove = None;
            for (k, (pos, name)) in self.markers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new(fmt_time(*pos as f64 / rate)).monospace().size(11.0)).frame(false)).on_hover_text(tl("Aller au marqueur")).clicked() {
                        go = Some(*pos);
                    }
                    ui.add(egui::TextEdit::singleline(name).desired_width(120.0));
                    if ui.small_button("×").clicked() {
                        remove = Some(k);
                    }
                });
            }
            if let Some(k) = remove {
                self.markers.remove(k);
            }
            if let Some(p) = go {
                self.cursor = p;
                self.sel = None;
                self.reveal(p);
            }
            if pro::ghost(ui, tl("+ Marqueur au curseur")).clicked() {
                self.add_marker();
            }
        });
    }

    fn sheets(&mut self, ctx: &egui::Context) {
        if let Some(mut o) = self.export.take() {
            let mut go = false;
            let mut cancel = false;
            let has_sel = self.sel.is_some();
            let has_marks = !self.markers.is_empty();
            let keep = pro::sheet(ctx, "export", 460.0, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(4), BLUE.gamma_multiply(0.25));
                    export_glyph(ui.painter(), r.shrink(9.0), BLUE);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tl("Exporter le son")).font(semibold(18.0)).color(INK_STRONG));
                        ui.label(RichText::new(tl("WAV sans perte, lisible partout")).size(12.0).color(MUTED));
                    });
                });
                ui.add_space(14.0);
                let row = |ui: &mut Ui, s: &str| {
                    ui.label(RichText::new(s).size(12.0).color(MUTED));
                };
                row(ui, "Qualité");
                pro::segmented(ui, &mut o.bits, &[tl("16 bits (CD)"), tl("24 bits"), tl("32 flottant"), tl("8 bits")]);
                if o.bits != 2 {
                    row(ui, "Dither (adoucit la réduction de bits)");
                    pro::segmented(ui, &mut o.dither, &[tl("Aucun"), tl("Triangulaire"), tl("Mise en forme")]);
                }
                ui.add_space(8.0);
                row(ui, "Fréquence");
                pro::segmented(ui, &mut o.rate, &[tl("Originale"), tl("44,1 k"), tl("48 k"), tl("22 k"), tl("96 k")]);
                if o.rate > 0 {
                    ui.checkbox(&mut o.hq, tl("Conversion haute qualité (filtre sinc)"));
                }
                ui.add_space(8.0);
                row(ui, "Canaux");
                pro::segmented(ui, &mut o.channels, &[tl("Originaux"), tl("Mono"), tl("Stéréo")]);
                ui.add_space(8.0);
                row(ui, "Plage");
                let mut r = o.range;
                pro::segmented(ui, &mut r, &[tl("Tout"), tl("Sélection"), tl("Un fichier par marqueur")]);
                if (r == 1 && has_sel) || (r == 2 && has_marks) || r == 0 {
                    o.range = r;
                }
                if !has_marks {
                    studio_ui::pro::tip(ui, tl("Pose des marqueurs (M) pour découper en plusieurs fichiers."));
                }
                ui.add_space(8.0);
                ui.checkbox(&mut o.normalize, tl("Normaliser à −1 dB avant l'export"));
                ui.checkbox(&mut o.cues, tl("Inclure les marqueurs (repères lus par les éditeurs audio)"));
                egui::CollapsingHeader::new(RichText::new(tl("Métadonnées")).size(12.0).color(MUTED)).id_salt("meta").show(ui, |ui| {
                    egui::Grid::new("meta-grid").num_columns(2).spacing(vec2(8.0, 4.0)).show(ui, |ui| {
                        for (k, (_, label)) in INFO_IDS.iter().enumerate() {
                            ui.label(RichText::new(*label).size(12.0).color(MUTED));
                            ui.add(egui::TextEdit::singleline(&mut self.meta[k]).desired_width(280.0));
                            ui.end_row();
                        }
                    });
                });
                if self.stats.as_ref().is_some_and(|s| s.clipped > 0) {
                    ui.label(RichText::new(tl("Attention : des échantillons sont écrêtés. Le limiteur ou la normalisation les corrigent.")).size(12.0).color(RED));
                }
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, tl("Exporter…")).clicked() {
                        go = true;
                    }
                    if pro::secondary(ui, tl("Annuler")).clicked() {
                        cancel = true;
                    }
                });
            });
            if go {
                self.run_export(&o);
            } else if keep && !cancel {
                self.export = Some(o);
            }
        }
        if self.props_sheet {
            let mut done = false;
            let mut resample = None;
            let mut mono = false;
            let mut stereo = false;
            let keep = pro::sheet(ctx, "props", 420.0, |ui| {
                ui.label(RichText::new(tl("Propriétés du son")).font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(10.0);
                ui.label(RichText::new(format!("{} Hz · {} canal(aux) · {} · {} échantillons", self.clip.rate, self.clip.ch.len(), fmt_time(self.clip.secs(self.clip.len())), self.clip.len())).size(12.0).color(MUTED));
                ui.add_space(10.0);
                ui.label(RichText::new(tl("Convertir la fréquence d'échantillonnage")).size(12.0).color(MUTED));
                ui.horizontal(|ui| {
                    for r in [22_050u32, 32_000, 44_100, 48_000, 96_000] {
                        if ui.selectable_label(self.clip.rate == r, format!("{r} Hz")).clicked() && self.clip.rate != r {
                            resample = Some(r);
                        }
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.clip.ch.len() > 1, egui::Button::new("Passer en mono")).clicked() {
                        mono = true;
                    }
                    if ui.add_enabled(self.clip.ch.len() == 1, egui::Button::new("Passer en stéréo")).clicked() {
                        stereo = true;
                    }
                });
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, tl("Terminé")).clicked() {
                        done = true;
                    }
                });
            });
            if let Some(r) = resample {
                self.snapshot("Conversion de fréquence");
                let k = r as f64 / self.clip.rate as f64;
                self.clip = self.clip.resampled(r);
                self.markers.iter_mut().for_each(|m| m.0 = (m.0 as f64 * k) as usize);
                self.cursor = (self.cursor as f64 * k) as usize;
                self.sel = None;
                self.modified = true;
                self.fit_pending = true;
                self.changed();
            }
            if mono {
                self.snapshot("Mono");
                self.clip.to_mono();
                self.clip.ch.truncate(1);
                self.modified = true;
                self.changed();
            }
            if stereo {
                self.snapshot("Stéréo");
                let c = self.clip.ch[0].clone();
                self.clip.ch.push(c);
                self.modified = true;
                self.changed();
            }
            if done || !keep {
                self.props_sheet = false;
            }
        }
    }

    fn options_bar(&mut self, ui: &mut Ui) {
        ui.spacing_mut().slider_width = 90.0;
        let t = self.tool;
        ui.label(RichText::new(t.name()).font(semibold(12.0)).color(INK_STRONG));
        pro::toolbar_sep(ui);
        let small = |ui: &mut Ui, s: &str| {
            ui.add(egui::Label::new(RichText::new(s).size(12.0).color(MUTED)).truncate());
        };
        match t {
            Tool::Select | Tool::Hand | Tool::Zoom => {
                if ui.small_button(tl("Tout sélectionner")).clicked() {
                    self.sel = Some((0, self.clip.len()));
                }
                if ui.add_enabled(self.sel.is_some(), egui::Button::new("Désélectionner").small()).clicked() {
                    self.sel = None;
                }
                if ui.small_button(tl("Zoom 1:1 (échantillons)")).clicked() {
                    let c = self.view_start + 500.0 * self.spp;
                    self.spp = 0.1;
                    self.view_start = c - 50.0;
                }
                small(ui, t.help());
            }
            Tool::SpecRect | Tool::Lasso | Tool::Brush | Tool::Heal => {
                small(ui, "Échelle");
                let mut l = !self.spec_log as usize;
                if pro::segmented(ui, &mut l, &[tl("Log"), tl("Linéaire")]) {
                    self.spec_log = l == 0;
                }
                small(ui, "FFT");
                egui::ComboBox::from_id_salt("fft").width(64.0).selected_text(FFT_SIZES[self.fft].to_string()).show_ui(ui, |ui| {
                    for (k, n) in FFT_SIZES.iter().enumerate() {
                        ui.selectable_value(&mut self.fft, k, n.to_string());
                    }
                });
                if matches!(t, Tool::Brush | Tool::Heal) {
                    small(ui, "Taille");
                    ui.add(egui::Slider::new(&mut self.brush, 3.0..=80.0).suffix(" px").integer());
                }
                if t != Tool::Heal {
                    pro::toolbar_sep(ui);
                    let has = self.spec_sel.is_some();
                    ui.add(egui::DragValue::new(&mut self.spec_db).range(-60.0..=24.0).suffix(" dB").speed(0.2));
                    let g = self.spec_db;
                    if ui.add_enabled(has, egui::Button::new("Appliquer le gain").small()).on_hover_text(tl("Entrée")).clicked() {
                        self.spectral_apply(dsp::SpecMode::Gain(g), "Gain spectral");
                    }
                    if ui.add_enabled(has, egui::Button::new("Supprimer").small()).clicked() {
                        self.spectral_apply(dsp::SpecMode::Gain(-90.0), "Suppression spectrale");
                    }
                    if ui.add_enabled(has, egui::Button::new("Réparer").small()).on_hover_text(tl("Reconstruit la zone à partir de ce qui l'entoure")).clicked() {
                        self.spectral_apply(dsp::SpecMode::Heal, "Réparation spectrale");
                    }
                    if ui.add_enabled(has, egui::Button::new("Isoler").small()).on_hover_text(tl("Garde seulement la zone (le reste devient silencieux)")).clicked()
                        && let Some(sel) = self.spec_sel.clone()
                    {
                        let nyq = self.clip.rate as f32 / 2.0;
                        let log = self.spec_log;
                        let (a, b) = (sel.a, sel.b.min(self.clip.len()));
                        self.snapshot("Isoler la zone spectrale");
                        let yf = move |f: f32| dsp::y_frac(f, nyq, log);
                        self.clip.spectral_edit(a, b, &|s, f| !sel.contains(s, f, &yf), dsp::SpecMode::Gain(-90.0));
                        self.modified = true;
                        self.changed();
                    }
                    if ui.add_enabled(has, egui::Button::new("×").small()).on_hover_text(tl("Effacer la zone (Échap)")).clicked() {
                        self.spec_sel = None;
                    }
                }
            }
            Tool::Pencil => {
                let ok = self.spp <= 8.0;
                small(ui, if ok { "Dessine la forme d'onde, un canal à la fois." } else { "Zoome davantage pour dessiner les échantillons." });
                if ui.small_button(tl("Zoom aux échantillons")).clicked() {
                    let c = self.view_start + 500.0 * self.spp;
                    self.spp = 0.25;
                    self.view_start = c - 125.0;
                }
            }
            Tool::Razor => {
                let cuts = self.markers.len();
                small(ui, &format!("{cuts} coupe(s) · Alt+clic supprime un segment"));
                if ui.add_enabled(cuts > 0, egui::Button::new("Exporter en morceaux…").small()).clicked() {
                    let mut o = self.export_opts();
                    o.range = 2;
                    self.export = Some(o);
                }
            }
            Tool::Fade => {
                let r = self.clip.rate as f64;
                small(ui, &format!("Entrée {:.2} s · Sortie {:.2} s", self.fades[0].0 as f64 / r, self.fades[1].0 as f64 / r));
                for (k, label) in [(0, "Courbe entrée"), (1, "Courbe sortie")] {
                    small(ui, label);
                    ui.add(egui::Slider::new(&mut self.fades[k].1, -1.0..=1.0).show_value(false));
                }
                if ui.add_enabled(self.fades[0].0 + self.fades[1].0 > 0, egui::Button::new("Appliquer").small()).on_hover_text(tl("Entrée")).clicked() {
                    self.apply_fades();
                }
                if ui.small_button(tl("Réinitialiser")).clicked() {
                    self.fades = [(0, 0.0); 2];
                }
            }
            Tool::Envelope => {
                small(ui, &format!("{} point(s)", self.env.len()));
                let n = self.clip.len();
                for (label, pts) in [("Montée", vec![(0usize, -30.0f32), (n, 0.0)]), ("Descente", vec![(0, 0.0), (n, -30.0)]), ("Creux au centre", vec![(0, 0.0), (n / 3, -12.0), (n * 2 / 3, -12.0), (n, 0.0)])] {
                    if ui.small_button(label).clicked() {
                        self.env = pts;
                    }
                }
                if ui.add_enabled(!self.env.is_empty(), egui::Button::new("Appliquer").small()).on_hover_text(tl("Entrée")).clicked() {
                    self.apply_env();
                }
                if ui.add_enabled(!self.env.is_empty(), egui::Button::new("Effacer").small()).clicked() {
                    self.env.clear();
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn tool_pointer(
        &mut self,
        resp: &egui::Response,
        wave: Rect,
        spp: f64,
        sample_at: &dyn Fn(f32) -> usize,
        x_of: &dyn Fn(f64) -> f32,
        y_to_f: &dyn Fn(f32) -> f32,
        env_y: &dyn Fn(f32) -> f32,
        env_g: &dyn Fn(f32) -> f32,
        fade_x: [f32; 2],
        lane_h: f32,
        amp: f32,
        spec_view: bool,
    ) {
        let tool = self.tool;
        let n = self.clip.len();
        let alt = resp.ctx.input(|i| i.modifiers.alt);
        let Some(p) = resp.interact_pointer_pos() else { return };
        let s = sample_at(p.x);
        let brush = |me: &Self, f: f32| SpecShape::Brush { pts: vec![(s as f64, f)], rs: me.brush as f64 * spp, ry: me.brush / wave.height() };
        if resp.drag_started() {
            self.drag = match tool {
                Tool::Zoom => Drag::Zoom(s),
                Tool::SpecRect | Tool::Lasso | Tool::Brush | Tool::Heal if spec_view => {
                    let f = y_to_f(p.y);
                    let shape = match tool {
                        Tool::SpecRect => SpecShape::Rect { f0: f, f1: f },
                        Tool::Lasso => SpecShape::Lasso(vec![(s as f64, f)]),
                        _ => brush(self, f),
                    };
                    let r = (self.brush as f64 * spp) as usize;
                    self.spec_sel = Some(SpecSel { a: s.saturating_sub(r), b: s + r, shape });
                    Drag::Spec(s, f)
                }
                Tool::SpecRect | Tool::Lasso | Tool::Brush => {
                    self.view_mode = 1;
                    Drag::None
                }
                Tool::Heal => Drag::Heal(s),
                Tool::Pencil if self.spp <= 8.0 => {
                    self.snapshot("Crayon");
                    Drag::Pencil((((p.y - wave.top()) / lane_h) as usize).min(self.clip.ch.len().saturating_sub(1)), None)
                }
                Tool::Pencil => {
                    self.notice = Some(("Crayon : zoome jusqu'aux échantillons d'abord".into(), false));
                    Drag::None
                }
                Tool::Fade => Drag::Fade((p.x - fade_x[0]).abs() <= (p.x - fade_x[1]).abs()),
                Tool::Envelope => {
                    let k = self.env.iter().position(|(es, g)| (x_of(*es as f64) - p.x).abs() < 8.0 && (env_y(*g) - p.y).abs() < 8.0);
                    Drag::Env(k.unwrap_or_else(|| {
                        let g = env_g(p.y);
                        let at = self.env.iter().position(|e| e.0 > s).unwrap_or(self.env.len());
                        self.env.insert(at, (s, g));
                        at
                    }))
                }
                _ => Drag::None,
            };
        }
        if resp.dragged() {
            match self.drag {
                Drag::None if tool == Tool::Hand => self.view_start -= resp.drag_delta().x as f64 * spp,
                Drag::Zoom(f) | Drag::Heal(f) => self.sel = Some((f.min(s), f.max(s))).filter(|(a, b)| b > a),
                Drag::Spec(s0, f0) => {
                    let f = y_to_f(p.y);
                    if let Some(sel) = &mut self.spec_sel {
                        if let SpecShape::Rect { f0: lo, f1: hi } = &mut sel.shape {
                            (*lo, *hi) = (f0.min(f), f0.max(f));
                            (sel.a, sel.b) = (s0.min(s), s0.max(s));
                        } else {
                            sel.extend(s as f64, f);
                        }
                    }
                }
                Drag::Pencil(ci, last) => {
                    let mid = wave.top() + ci as f32 * lane_h + lane_h / 2.0;
                    let v = ((mid - p.y) / amp.max(1.0)).clamp(-1.0, 1.0);
                    let s = s.min(n.saturating_sub(1));
                    let (ls, lv) = last.unwrap_or((s, v));
                    let (a, b) = (ls.min(s), ls.max(s));
                    if let Some(c) = self.clip.ch.get_mut(ci) {
                        for i in a..=b {
                            let t = if b > a { (i - ls.min(s)) as f32 / (b - a) as f32 } else { 1.0 };
                            let (va, vb) = if ls <= s { (lv, v) } else { (v, lv) };
                            c[i] = va + (vb - va) * t;
                        }
                    }
                    self.drag = Drag::Pencil(ci, Some((s, v)));
                    self.version += 1;
                }
                Drag::Fade(left) => {
                    let c = (-(p.y - wave.center().y) / (wave.height() / 2.0)).clamp(-1.0, 1.0);
                    if left {
                        self.fades[0] = (s.min(n - self.fades[1].0.min(n)), c);
                    } else {
                        self.fades[1] = (n - s.max(self.fades[0].0).min(n), c);
                    }
                }
                Drag::Env(k) if k < self.env.len() => {
                    let lo = if k > 0 { self.env[k - 1].0 + 1 } else { 0 };
                    let hi = self.env.get(k + 1).map_or(n, |e| e.0.saturating_sub(1));
                    self.env[k] = (s.clamp(lo, hi.max(lo)), env_g(p.y));
                }
                _ => {}
            }
        }
        if resp.drag_stopped() {
            match self.drag {
                Drag::Zoom(_) => {
                    self.zoom_sel();
                    self.sel = None;
                }
                Drag::Heal(_) => {
                    if self.sel.is_some() {
                        self.edit("Correction des taches", |c, a, b| c.heal(a, b));
                        self.sel = None;
                    }
                }
                Drag::Spec(..) if tool == Tool::Heal => {
                    self.spectral_apply(dsp::SpecMode::Heal, "Correction spectrale");
                    self.spec_sel = None;
                }
                Drag::Pencil(..) => {
                    self.modified = true;
                    self.changed();
                }
                _ => {}
            }
            self.drag = Drag::None;
        }
        if resp.clicked() || resp.secondary_clicked() {
            let secondary = resp.secondary_clicked();
            match tool {
                Tool::Zoom => {
                    let k = if alt || secondary { 2.0 } else { 0.5 };
                    let at = s as f64;
                    let off = at - self.view_start;
                    self.spp = (self.spp * k).max(0.05);
                    self.view_start = at - off * k;
                }
                Tool::Razor if alt => {
                    let lo = self.markers.iter().map(|m| m.0).filter(|m| *m <= s).max().unwrap_or(0);
                    let hi = self.markers.iter().map(|m| m.0).filter(|m| *m > s).min().unwrap_or(n);
                    self.markers.retain(|m| m.0 != lo || lo == 0);
                    self.sel = Some((lo, hi)).filter(|(a, b)| b > a);
                    self.cut(false);
                }
                Tool::Razor => {
                    let k = self.markers.len() + 1;
                    self.markers.push((s, format!("Coupe {k}")));
                    self.markers.sort_by_key(|m| m.0);
                }
                Tool::Envelope => {
                    let near = self.env.iter().position(|(es, g)| (x_of(*es as f64) - p.x).abs() < 8.0 && (env_y(*g) - p.y).abs() < 8.0);
                    match (near, secondary) {
                        (Some(k), true) => {
                            self.env.remove(k);
                        }
                        (None, false) => {
                            let at = self.env.iter().position(|e| e.0 > s).unwrap_or(self.env.len());
                            self.env.insert(at, (s, env_g(p.y)));
                        }
                        _ => {}
                    }
                }
                Tool::Fade if resp.double_clicked() => self.fades = [(0, 0.0); 2],
                Tool::SpecRect | Tool::Lasso | Tool::Brush => self.spec_sel = None,
                _ if !secondary => {
                    self.cursor = s;
                    self.sel = None;
                }
                _ => {}
            }
        }
    }

    fn waveform(&mut self, ui: &mut Ui, playing: bool, pos: usize) {
        let full = ui.max_rect();
        let head_h = 68.0;
        let head = Rect::from_min_size(full.min, vec2(full.width(), head_h));
        ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_size(full.min, vec2(full.width(), 30.0))), |ui| {
            ui.horizontal_centered(|ui| {
                pro::segmented(ui, &mut self.view_mode, &[tl("Forme d'onde"), tl("Spectrogramme")]);
                pro::toolbar_sep(ui);
                ui.label(RichText::new(tl("Hauteur")).size(12.0).color(MUTED));
                ui.spacing_mut().slider_width = 64.0;
                ui.add(egui::Slider::new(&mut self.amp_zoom, 1.0..=10.0).logarithmic(true).show_value(false));
                ui.toggle_value(&mut self.follow, RichText::new(tl("Suivre")).size(12.0)).on_hover_text(tl("La vue suit la lecture (F)"));
                pro::toolbar_sep(ui);
                ui.label(RichText::new(tl("Zoom")).size(12.0).color(MUTED));
                if ui.small_button("−").clicked() {
                    self.zoom_by(2.0);
                }
                if ui.small_button("+").clicked() {
                    self.zoom_by(0.5);
                }
                if ui.add_enabled(self.sel.is_some(), egui::Button::new("Sélection").small()).on_hover_text(tl("Z")).clicked() {
                    self.zoom_sel();
                }
                if ui.small_button(tl("Tout")).on_hover_text(tl("Ctrl+0")).clicked() {
                    self.fit_pending = true;
                }
            });
        });
        let bar = Rect::from_min_size(full.min + vec2(0.0, 34.0), vec2(full.width(), 30.0));
        ui.painter().rect_filled(bar, CornerRadius::same(7), PANEL);
        ui.scope_builder(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(10.0, 0.0))), |ui| {
            ui.horizontal_centered(|ui| self.options_bar(ui));
        });
        let over_h = 26.0;
        let area = Rect::from_min_max(pos2(full.left(), head.bottom() + 4.0), pos2(full.right(), full.bottom() - over_h - 6.0));
        let overview = Rect::from_min_max(pos2(full.left(), full.bottom() - over_h), full.max);
        let ruler_h = 24.0;
        let wave = Rect::from_min_max(area.min + vec2(0.0, ruler_h), area.max);
        let resp = ui.interact(area, ui.make_persistent_id("wave"), Sense::click_and_drag());
        let n = self.clip.len().max(1);
        let width = wave.width().max(1.0);
        if self.fit_pending {
            self.spp = n as f64 / width as f64;
            self.view_start = 0.0;
            self.fit_pending = false;
        }
        let max_spp = n as f64 / width as f64;
        if resp.hovered() {
            let (scroll, shift, hover) = ui.input(|i| (i.smooth_scroll_delta, i.modifiers.shift, i.pointer.hover_pos()));
            let dy = if scroll.y != 0.0 { scroll.y } else { scroll.x };
            if dy != 0.0 {
                if shift || scroll.x != 0.0 {
                    self.view_start -= dy as f64 * self.spp;
                } else if let Some(p) = hover {
                    let at = self.view_start + (p.x - wave.min.x) as f64 * self.spp;
                    self.spp = (self.spp * (-(dy as f64) / 300.0).exp()).clamp(0.05, max_spp.max(0.05));
                    self.view_start = at - (p.x - wave.min.x) as f64 * self.spp;
                }
            }
        }
        self.spp = self.spp.min(max_spp.max(0.05));
        if playing && self.follow {
            let visible = width as f64 * self.spp;
            if (pos as f64) > self.view_start + visible * 0.92 || (pos as f64) < self.view_start {
                self.view_start = pos as f64 - visible * 0.08;
            }
        }
        let visible = width as f64 * self.spp;
        self.view_start = self.view_start.clamp(0.0, (n as f64 - visible).max(0.0));
        let (start, spp) = (self.view_start, self.spp);
        let x_of = |s: f64| wave.min.x + ((s - start) / spp) as f32;
        let sample_at = |x: f32| ((start + (x - wave.min.x) as f64 * spp).round().max(0.0) as usize).min(n);
        let tool = self.tool;
        let nyq = self.clip.rate as f32 / 2.0;
        let log = self.spec_log;
        let spec_view = self.view_mode == 1;
        let y_to_f = |y: f32| dsp::freq_at((wave.bottom() - y) / wave.height(), nyq, log);
        let f_to_y = |f: f32| wave.bottom() - dsp::y_frac(f, nyq, log) * wave.height();
        let lanes = self.clip.ch.len().max(1);
        let lane_h0 = wave.height() / lanes as f32;
        let amp0 = lane_h0 * 0.45 * self.amp_zoom;
        let env_y = |g: f32| wave.top() + (12.0 - g) / 42.0 * wave.height();
        let env_g = |y: f32| (12.0 - (y - wave.top()) / wave.height() * 42.0).clamp(-30.0, 12.0);
        let fade_x = [x_of(self.fades[0].0 as f64), x_of(n.saturating_sub(self.fades[1].0) as f64)];
        if tool != Tool::Select {
            self.tool_pointer(&resp, wave, spp, &sample_at, &x_of, &y_to_f, &env_y, &env_g, fade_x, lane_h0, amp0, spec_view);
        } else if let Some(p) = resp.interact_pointer_pos() {
            let s = sample_at(p.x);
            if resp.drag_started() {
                self.drag = match self.sel {
                    Some((a, _)) if (x_of(a as f64) - p.x).abs() < 6.0 => Drag::EdgeA,
                    Some((_, b)) if (x_of(b as f64) - p.x).abs() < 6.0 => Drag::EdgeB,
                    _ => Drag::Select(s),
                };
            }
            if resp.dragged() {
                match self.drag {
                    Drag::Select(f) => self.sel = if s.abs_diff(f) > (2.0 * spp) as usize { Some((f.min(s), f.max(s))) } else { None },
                    Drag::EdgeA => {
                        if let Some((_, b)) = self.sel {
                            self.sel = Some((s.min(b), s.max(b))).filter(|(x, y)| y > x);
                        }
                    }
                    Drag::EdgeB => {
                        if let Some((a, _)) = self.sel {
                            self.sel = Some((a.min(s), a.max(s))).filter(|(x, y)| y > x);
                        }
                    }
                    _ => {}
                }
            }
            if resp.double_clicked() {
                let lo = self.markers.iter().map(|m| m.0).filter(|m| *m <= s).max().unwrap_or(0);
                let hi = self.markers.iter().map(|m| m.0).filter(|m| *m > s).min().unwrap_or(n);
                self.sel = Some((lo, hi));
                self.cursor = lo;
            } else if resp.clicked() {
                if ui.input(|i| i.modifiers.shift) {
                    let (a, b) = self.sel.unwrap_or((self.cursor, self.cursor));
                    self.sel = Some((a.min(s), b.max(s))).filter(|(x, y)| y > x);
                } else {
                    self.cursor = s;
                    self.sel = None;
                    if playing {
                        self.toggle_play();
                        self.toggle_play();
                    }
                }
            }
        }
        if tool == Tool::Select && resp.drag_stopped() {
            if let Some((a, _)) = self.sel {
                self.cursor = a;
            }
            self.drag = Drag::None;
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(match tool {
                Tool::Hand => egui::CursorIcon::Grab,
                Tool::Zoom => egui::CursorIcon::ZoomIn,
                Tool::Select => egui::CursorIcon::Text,
                _ => egui::CursorIcon::Crosshair,
            });
        }
        if resp.hovered()
            && let (Some(p), Some((a, b))) = (resp.hover_pos(), self.sel)
            && ((x_of(a as f64) - p.x).abs() < 6.0 || (x_of(b as f64) - p.x).abs() < 6.0)
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        let cols = width as usize;
        let key = (start as usize, spp, cols, self.version);
        if self.view_mode == 0 && self.peaks.as_ref().is_none_or(|p| p.key != key) {
            let peaks = self
                .clip
                .ch
                .iter()
                .map(|c| {
                    (0..cols)
                        .map(|x| {
                            let a = (start + x as f64 * spp) as usize;
                            let b = ((start + (x + 1) as f64 * spp) as usize).max(a + 1).min(c.len());
                            if a >= c.len() {
                                return (0.0, 0.0);
                            }
                            c[a..b].iter().fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)))
                        })
                        .collect()
                })
                .collect();
            self.peaks = Some(Peaks { key, cols: peaks });
        }
        if self.peaks.is_none() {
            self.peaks = Some(Peaks { key: (0, spp, cols, u64::MAX), cols: Vec::new() });
        }
        let painter = ui.painter_at(area.union(overview));
        painter.rect_filled(Rect::from_min_max(area.min, pos2(area.max.x, wave.min.y)), CornerRadius::same(6), PANEL);
        let rate = self.clip.rate as f64;
        let secs_visible = visible / rate;
        let step = [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 300.0].into_iter().find(|s| secs_visible / s <= 12.0).unwrap_or(600.0);
        let first = (start / rate / step).floor() as i64;
        let mut k = first;
        loop {
            let t = k as f64 * step;
            if t > start / rate + secs_visible {
                break;
            }
            let x = x_of(t * rate);
            painter.line_segment([pos2(x, area.min.y + 14.0), pos2(x, wave.min.y)], Stroke::new(1.0, FAINT));
            painter.text(pos2(x + 3.0, area.min.y + 3.0), Align2::LEFT_TOP, fmt_time(t), FontId::monospace(10.0), MUTED);
            k += 1;
        }
        let nch = self.clip.ch.len().max(1);
        let lane_h = wave.height() / nch as f32;
        if self.view_mode == 1 {
            let rows = (wave.height() / 3.0).clamp(32.0, 160.0) as usize;
            let skey = (start as usize, spp.to_bits(), cols, self.version, rows * 16 + self.fft * 2 + log as usize);
            if self.specgram.as_ref().is_none_or(|s| s.0 != skey) {
                let data = self.clip.spectrogram_ex(start, spp, cols.max(1), rows, FFT_SIZES[self.fft], log);
                let mut img = egui::ColorImage::filled([cols.max(1), rows], Color32::BLACK);
                for (x, col) in data.iter().enumerate() {
                    for (y, v) in col.iter().enumerate() {
                        img[(x, rows - 1 - y)] = heat(((v + 90.0) / 90.0).clamp(0.0, 1.0));
                    }
                }
                let tex = ui.ctx().load_texture("specgram", img, TextureOptions::LINEAR);
                self.specgram = Some((skey, tex));
            }
            if let Some((_, tex)) = &self.specgram {
                painter.image(tex.id(), wave, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            for (f, l) in [(100.0, "100 Hz"), (500.0, "500 Hz"), (1000.0, "1 kHz"), (2000.0, "2 kHz"), (5000.0, "5 kHz"), (10_000.0, "10 kHz"), (15_000.0, "15 kHz")] {
                if f < nyq && (log || f >= 1000.0) {
                    let y = f_to_y(f);
                    painter.line_segment([pos2(wave.left(), y), pos2(wave.left() + 8.0, y)], Stroke::new(1.0, Color32::WHITE));
                    painter.text(pos2(wave.left() + 11.0, y), Align2::LEFT_CENTER, l, FontId::proportional(10.0), Color32::from_white_alpha(200));
                }
            }
        } else if let Some(peaks) = &self.peaks {
            for (ci, col) in peaks.cols.iter().enumerate() {
                let lane = Rect::from_min_size(pos2(wave.min.x, wave.min.y + ci as f32 * lane_h), vec2(width, lane_h));
                let mid = lane.center().y;
                let amp = lane_h * 0.45 * self.amp_zoom;
                painter.rect_filled(lane.shrink2(vec2(0.0, 3.0)), CornerRadius::same(4), Color32::from_rgb(0x1c, 0x1c, 0x1c));
                for g in [-6.0f32, -12.0] {
                    let h = 10f32.powf(g / 20.0) * amp;
                    if h < lane_h * 0.48 {
                        for y in [mid - h, mid + h] {
                            painter.line_segment([pos2(lane.min.x, y), pos2(lane.max.x, y)], Stroke::new(1.0, Color32::from_white_alpha(8)));
                        }
                    }
                }
                painter.line_segment([pos2(lane.min.x, mid), pos2(lane.max.x, mid)], Stroke::new(1.0, LINE));
                painter.text(lane.left_top() + vec2(10.0, 8.0), Align2::LEFT_TOP, if nch == 1 { "Mono" } else if ci == 0 { "G" } else { "D" }, semibold(11.0), FAINT);
                let lp = painter.with_clip_rect(lane.shrink2(vec2(0.0, 3.0)));
                for (x, (lo, hi)) in col.iter().enumerate() {
                    if lo > hi {
                        continue;
                    }
                    let px = wave.min.x + x as f32 + 0.5;
                    let y0 = mid - hi * amp;
                    let y1 = (mid - lo * amp).max(y0 + 1.0);
                    let c = if hi.abs() >= 0.999 || lo.abs() >= 0.999 { RED } else { WAVE };
                    lp.line_segment([pos2(px, y0), pos2(px, y1)], Stroke::new(1.0, c));
                }
            }
        }
        if let Some((a, b)) = self.sel {
            let r = Rect::from_x_y_ranges(x_of(a as f64).max(wave.min.x)..=x_of(b as f64).min(wave.max.x), wave.y_range());
            painter.rect_filled(r, CornerRadius::ZERO, ACCENT.gamma_multiply(0.22));
            for e in [a, b] {
                let x = x_of(e as f64);
                if x >= wave.min.x && x <= wave.max.x {
                    painter.line_segment([pos2(x, wave.min.y), pos2(x, wave.max.y)], Stroke::new(1.0, ACCENT.lerp_to_gamma(Color32::WHITE, 0.3)));
                }
            }
        }
        for (m, name) in &self.markers {
            let x = x_of(*m as f64);
            if x >= wave.min.x && x <= wave.max.x {
                painter.line_segment([pos2(x, wave.min.y), pos2(x, area.max.y)], Stroke::new(1.0, MARK.gamma_multiply(0.6)));
                painter.add(Shape::convex_polygon(vec![pos2(x - 5.0, area.min.y + 12.0), pos2(x + 5.0, area.min.y + 12.0), pos2(x, wave.min.y)], MARK, Stroke::NONE));
                let g = painter.layout_no_wrap(name.clone(), FontId::proportional(10.5), Color32::BLACK);
                let tag = Rect::from_min_size(pos2(x + 3.0, wave.min.y + 2.0), g.size() + vec2(8.0, 4.0));
                painter.rect_filled(tag, CornerRadius::same(4), MARK);
                painter.galley(tag.min + vec2(4.0, 2.0), g, Color32::BLACK);
            }
        }
        if spec_view && let Some(sel) = &self.spec_sel {
            let col = Color32::from_rgb(120, 230, 255);
            match &sel.shape {
                SpecShape::Rect { f0, f1 } => {
                    let r = Rect::from_x_y_ranges(x_of(sel.a as f64)..=x_of(sel.b as f64), f_to_y(*f1)..=f_to_y(*f0));
                    painter.rect_filled(r, CornerRadius::ZERO, col.gamma_multiply(0.15));
                    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.5, col), egui::StrokeKind::Middle);
                }
                SpecShape::Lasso(pts) => {
                    let v: Vec<Pos2> = pts.iter().map(|(s, f)| pos2(x_of(*s), f_to_y(*f))).collect();
                    painter.add(Shape::closed_line(v, Stroke::new(1.5, col)));
                }
                SpecShape::Brush { pts, rs, .. } => {
                    let r = (*rs / spp) as f32;
                    for (s, f) in pts.iter().step_by((pts.len() / 400).max(1)) {
                        painter.circle_filled(pos2(x_of(*s), f_to_y(*f)), r, col.gamma_multiply(0.18));
                    }
                }
            }
        }
        if tool == Tool::Fade || self.fades[0].0 + self.fades[1].0 > 0 {
            let fcol = Color32::from_rgb(255, 170, 60);
            for (k, (len, curve)) in self.fades.iter().enumerate() {
                let (a, b) = if k == 0 { (0, *len) } else { (n.saturating_sub(*len), n) };
                if b > a {
                    let pts: Vec<Pos2> = (0..=40).map(|i| {
                        let t = i as f32 / 40.0;
                        let g = dsp::fade_gain(if k == 0 { t } else { 1.0 - t }, *curve);
                        pos2(x_of(a as f64 + (b - a) as f64 * t as f64), wave.bottom() - g * wave.height())
                    }).collect();
                    painter.add(Shape::line(pts, Stroke::new(2.0, fcol)));
                }
                let x = fade_x[k];
                if x >= wave.min.x - 6.0 && x <= wave.max.x + 6.0 {
                    painter.rect_filled(Rect::from_center_size(pos2(x, wave.top() + 7.0), Vec2::splat(10.0)), CornerRadius::same(2), fcol);
                }
            }
        }
        if tool == Tool::Envelope || !self.env.is_empty() {
            let ecol = Color32::from_rgb(120, 255, 160);
            let y0 = env_y(0.0);
            painter.line_segment([pos2(wave.left(), y0), pos2(wave.right(), y0)], Stroke::new(1.0, ecol.gamma_multiply(0.25)));
            if !self.env.is_empty() {
                let mut v = vec![pos2(wave.left(), env_y(self.env_gain(start as usize)))];
                v.extend(self.env.iter().map(|(s, g)| pos2(x_of(*s as f64), env_y(*g))));
                v.push(pos2(wave.right(), env_y(self.env_gain((start + visible) as usize))));
                painter.add(Shape::line(v, Stroke::new(1.8, ecol)));
                for (s, g) in &self.env {
                    painter.circle_filled(pos2(x_of(*s as f64), env_y(*g)), 4.0, ecol);
                }
            }
        }
        let line = |p: &Painter, s: usize, c: Color32| {
            let x = x_of(s as f64);
            if x >= wave.min.x && x <= wave.max.x {
                p.line_segment([Pos2::new(x, area.min.y), Pos2::new(x, area.max.y)], Stroke::new(1.5, c));
            }
        };
        line(&painter, self.cursor, INK);
        if playing {
            line(&painter, pos, if self.preview.is_some() { MARK } else { ACCENT.lerp_to_gamma(Color32::WHITE, 0.3) });
        }

        let ocols = overview.width().max(1.0) as usize;
        if self.overview.as_ref().is_none_or(|o| o.0 != self.version || o.1 != ocols) {
            let per = (n as f64 / ocols as f64).max(1.0);
            let v: Vec<f32> = (0..ocols)
                .map(|x| {
                    let a = (x as f64 * per) as usize;
                    let b = (((x + 1) as f64 * per) as usize).clamp(a + 1, n);
                    if a >= self.clip.len() { 0.0 } else { self.clip.peak(a, b.min(self.clip.len())) }
                })
                .collect();
            self.overview = Some((self.version, ocols, v));
        }
        painter.rect_filled(overview, CornerRadius::same(6), Color32::from_rgb(0x1a, 0x1a, 0x1a));
        if let Some((_, _, v)) = &self.overview {
            let mid = overview.center().y;
            for (x, p) in v.iter().enumerate() {
                let h = p.min(1.0) * overview.height() * 0.45;
                let px = overview.left() + x as f32 + 0.5;
                painter.line_segment([pos2(px, mid - h), pos2(px, mid + h.max(0.5))], Stroke::new(1.0, WAVE.gamma_multiply(0.5)));
            }
        }
        let ox = |s: f64| overview.left() + (s / n as f64) as f32 * overview.width();
        let win = Rect::from_x_y_ranges(ox(start)..=ox(start + visible).max(ox(start) + 3.0), overview.y_range());
        painter.rect_stroke(win, CornerRadius::same(3), Stroke::new(1.5, Color32::from_white_alpha(170)), egui::StrokeKind::Inside);
        painter.rect_filled(win, CornerRadius::same(3), Color32::from_white_alpha(14));
        let or = ui.interact(overview, ui.make_persistent_id("overview"), Sense::click_and_drag()).on_hover_text(tl("Vue d'ensemble : clique ou glisse pour te déplacer"));
        if let Some(p) = or.interact_pointer_pos()
            && (or.clicked() || or.dragged())
        {
            let s = ((p.x - overview.left()) / overview.width()).clamp(0.0, 1.0) as f64 * n as f64;
            self.view_start = s - visible / 2.0;
        }
    }
}

fn heat(t: f32) -> Color32 {
    let stops = [(0.0, [8, 6, 20]), (0.35, [70, 30, 140]), (0.6, [200, 50, 120]), (0.8, [255, 150, 40]), (1.0, [255, 250, 200])];
    for w in stops.windows(2) {
        let ((t0, c0), (t1, c1)) = (w[0], w[1]);
        if t <= t1 {
            let k = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
            let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * k) as u8;
            return Color32::from_rgb(m(c0[0], c1[0]), m(c0[1], c1[1]), m(c0[2], c1[2]));
        }
    }
    Color32::from_rgb(255, 250, 200)
}

fn meters(ui: &mut Ui, levels: &[f32]) {
    let (r, _) = ui.allocate_exact_size(vec2(160.0, 30.0), Sense::hover());
    let p = ui.painter();
    let n = levels.len().max(1);
    let h = (r.height() - 4.0) / n as f32;
    for (k, v) in levels.iter().enumerate() {
        let bar = Rect::from_min_size(pos2(r.left(), r.top() + k as f32 * (h + 4.0)), vec2(r.width(), h - 2.0));
        p.rect_filled(bar, CornerRadius::same(2), Color32::from_rgb(0x16, 0x16, 0x16));
        let t = ((db(*v) + 60.0) / 60.0).clamp(0.0, 1.0);
        let segs = 30;
        for s in 0..((t * segs as f32) as usize) {
            let f = s as f32 / segs as f32;
            let c = if f > 0.92 { RED } else if f > 0.75 { Color32::from_rgb(255, 196, 0) } else { GREEN };
            let x = bar.left() + f * bar.width();
            p.rect_filled(Rect::from_min_size(pos2(x + 0.5, bar.top()), vec2(bar.width() / segs as f32 - 1.0, bar.height())), CornerRadius::ZERO, c);
        }
    }
    ui.label(RichText::new(tl("G\nD")).size(9.0).color(FAINT));
}

fn transport_glyph(p: &Painter, c: Pos2, kind: u8, color: Color32) {
    let tri = |dir: f32, at: Pos2, s: f32| vec![at + vec2(-s * 0.6 * dir, -s), at + vec2(s * 0.8 * dir, 0.0), at + vec2(-s * 0.6 * dir, s)];
    match kind {
        0 | 4 => {
            let dir = if kind == 4 { 1.0 } else { -1.0 };
            p.add(Shape::convex_polygon(tri(dir, c - vec2(1.0 * dir, 0.0), 6.0), color, Stroke::NONE));
            p.line_segment([c + vec2(6.0 * dir, -6.0), c + vec2(6.0 * dir, 6.0)], Stroke::new(2.0, color));
        }
        1 | 3 => {
            let dir = if kind == 3 { 1.0 } else { -1.0 };
            p.add(Shape::convex_polygon(tri(dir, c - vec2(3.5 * dir, 0.0), 5.0), color, Stroke::NONE));
            p.add(Shape::convex_polygon(vec![c + vec2(4.0 * dir, -5.0), c + vec2(9.0 * dir, 0.0), c + vec2(4.0 * dir, 5.0), c + vec2(-1.0 * dir, 0.0)], MARK, Stroke::NONE));
        }
        2 => {
            p.rect_filled(Rect::from_center_size(c, Vec2::splat(11.0)), CornerRadius::same(2), color);
        }
        _ => {
            let pts: Vec<Pos2> = (0..=20).map(|k| {
                let a = (-40.0 + k as f32 * 13.0f32).to_radians();
                c + vec2(a.cos(), a.sin()) * 7.0
            }).collect();
            let end = *pts.last().unwrap();
            p.add(Shape::line(pts, Stroke::new(1.8, color)));
            p.add(Shape::convex_polygon(vec![end + vec2(-4.0, -0.5), end + vec2(1.0, -3.0), end + vec2(0.5, 2.5)], color, Stroke::NONE));
        }
    }
}

fn export_glyph(p: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(1.6, c);
    let cen = r.center();
    let u = r.width() / 2.0;
    p.line_segment([cen + vec2(0.0, u * 0.3), cen + vec2(0.0, -u * 0.95)], s);
    p.line_segment([cen + vec2(-u * 0.45, -u * 0.5), cen + vec2(0.0, -u * 0.95)], s);
    p.line_segment([cen + vec2(u * 0.45, -u * 0.5), cen + vec2(0.0, -u * 0.95)], s);
    p.add(Shape::line(vec![cen + vec2(-u * 0.9, -u * 0.1), cen + vec2(-u * 0.9, u * 0.9), cen + vec2(u * 0.9, u * 0.9), cen + vec2(u * 0.9, -u * 0.1)], s));
}

fn slider(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) {
    ui.label(RichText::new(label).size(12.0).color(MUTED));
    ui.spacing_mut().slider_width = (ui.available_width() - 118.0).max(80.0);
    ui.add(egui::Slider::new(v, range).suffix(suffix).max_decimals(1));
}

fn fx_buttons(ui: &mut Ui) -> Option<bool> {
    let mut out = None;
    ui.horizontal(|ui| {
        if pro::ghost(ui, tl("▶ Écouter")).on_hover_text(tl("Essaie l'effet sur 15 s sans rien modifier")).clicked() {
            out = Some(true);
        }
        if pro::secondary(ui, tl("Appliquer")).clicked() {
            out = Some(false);
        }
    });
    ui.add_space(4.0);
    out
}

fn eq_curve(ui: &mut Ui, eq: &[f32; 5]) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 70.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(4), Color32::from_rgb(0x14, 0x14, 0x14));
    p.line_segment([r.left_center(), r.right_center()], Stroke::new(1.0, LINE));
    let pts: Vec<Pos2> = (0..=60)
        .map(|k| {
            let t = k as f32 / 60.0;
            let f = 20.0 * 1000f32.powf(t);
            let g: f32 = EQ_BANDS
                .iter()
                .enumerate()
                .map(|(i, (bf, _))| {
                    let d = (f / bf).log2();
                    match i {
                        0 => eq[i] / (1.0 + (d * 2.0).exp()),
                        4 => eq[i] / (1.0 + (-d * 2.0).exp()),
                        _ => eq[i] * (-d * d * 1.6).exp(),
                    }
                })
                .sum();
            pos2(r.left() + t * r.width(), r.center().y - g / 15.0 * r.height() * 0.45)
        })
        .collect();
    p.add(Shape::line(pts, Stroke::new(2.0, WAVE)));
    ui.add_space(4.0);
}

fn draw_spectrum(ui: &mut Ui, spec: &[f32]) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 90.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(4), Color32::from_rgb(0x14, 0x14, 0x14));
    let w = r.width() / spec.len().max(1) as f32;
    for (i, db) in spec.iter().enumerate() {
        let h = ((db + 90.0) / 90.0).clamp(0.0, 1.0) * (r.height() - 8.0);
        let x = r.left() + i as f32 * w;
        p.rect_filled(Rect::from_min_max(pos2(x + 0.5, r.bottom() - 4.0 - h), pos2(x + w - 0.5, r.bottom() - 4.0)), CornerRadius::ZERO, WAVE.gamma_multiply(0.85));
    }
    for (f, label) in [(100.0, "100"), (1000.0, "1k"), (10_000.0, "10k")] {
        let x = r.left() + r.width() * ((f / 20.0f32).ln() / (22_050.0f32 / 20.0).ln());
        p.text(pos2(x, r.top() + 4.0), Align2::CENTER_TOP, label, FontId::proportional(10.0), FAINT);
    }
    ui.add_space(6.0);
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_decorations(false)
            .with_title("Sound")
            .with_icon(Arc::new(studio_ui::app_window_icon("sound")))
            .with_inner_size([1440.0, 880.0])
            .with_min_inner_size([1040.0, 620.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Sound", options, Box::new(|cc| Ok(Box::new(Sound::new(cc)))))
}
