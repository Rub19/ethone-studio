#![windows_subsystem = "windows"]

mod audio;
mod tutos;

use audio::Clip;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2, pos2,
    vec2,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use studio_ui::pro::{self, CHROME, FAINT, FIELD, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};

const ACCENT: Color32 = Color32::from_rgb(100, 95, 240);
const WAVE: Color32 = Color32::from_rgb(140, 135, 255);
const MARK: Color32 = Color32::from_rgb(255, 196, 0);
const UNDO_LIMIT: usize = 20;

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
        let dev = cpal::default_host().default_input_device().ok_or("aucun micro dÃ©tectÃ©")?;
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
    eq: [f32; 3],
    comp: [f32; 3],
    gate: f32,
    drive: f32,
    echo: [f32; 3],
    verb: [f32; 2],
    speed: f32,
    pan: f32,
    width: f32,
    gen_secs: f32,
}

struct Sound {
    clip: Clip,
    version: u64,
    file: Option<PathBuf>,
    modified: bool,
    player: Player,
    recorder: Option<Recorder>,
    undo: Vec<Clip>,
    redo: Vec<Clip>,
    clipboard: Option<Clip>,
    cursor: usize,
    sel: Option<(usize, usize)>,
    markers: Vec<usize>,
    looping: bool,
    drag_from: Option<usize>,
    view_start: f64,
    spp: f64,
    fit_pending: bool,
    peaks: Option<Peaks>,
    spectrum: Option<(usize, u64, Vec<f32>)>,
    fx: Fx,
    error: Option<String>,
    title: String,
    icon: egui::TextureHandle,
    help: studio_ui::help::Help,
}

impl Sound {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        let clip = Clip { rate: 44_100, ch: vec![vec![0.0; 44_100 * 5], vec![0.0; 44_100 * 5]] };
        let player = Player::new(Arc::new(clip.clone()));
        let mut me = Self {
            error: player.error.clone().map(|e| format!("Lecture audio indisponible : {e}")),
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
            drag_from: None,
            view_start: 0.0,
            spp: 100.0,
            fit_pending: true,
            peaks: None,
            spectrum: None,
            fx: Fx { gain: 0.0, eq: [0.0; 3], comp: [-18.0, 4.0, 3.0], gate: -45.0, drive: 0.3, echo: [250.0, 0.35, 0.4], verb: [0.5, 0.3], speed: 100.0, pan: 0.0, width: 100.0, gen_secs: 1.0 },
            title: String::new(),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "sound"),
            help: studio_ui::help::Help::new("Sound", tutos::SOUND),
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
            Err(e) => self.error = Some(format!("Ouverture impossible : {e}")),
        }
    }

    fn changed(&mut self) {
        self.version += 1;
        self.player.set_clip(Arc::new(self.clip.clone()));
        let n = self.clip.len();
        self.cursor = self.cursor.min(n);
        self.sel = self.sel.map(|(a, b)| (a.min(n), b.min(n))).filter(|(a, b)| b > a);
        self.markers.retain(|m| *m <= n);
    }

    fn snapshot(&mut self) {
        self.undo.push(self.clip.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn edit(&mut self, f: impl FnOnce(&mut Clip, usize, usize)) {
        if self.clip.len() == 0 {
            return;
        }
        let (a, b) = self.sel.unwrap_or((0, self.clip.len()));
        self.snapshot();
        f(&mut self.clip, a, b);
        self.modified = true;
        self.changed();
    }

    fn undo(&mut self) {
        if let Some(c) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.clip, c));
            self.modified = true;
            self.changed();
        }
    }

    fn redo(&mut self) {
        if let Some(c) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.clip, c));
            self.modified = true;
            self.changed();
        }
    }

    fn cut(&mut self, keep_copy: bool) {
        let Some((a, b)) = self.sel else { return };
        if keep_copy {
            self.clipboard = Some(self.clip.slice(a, b));
        }
        self.edit(|c, a, b| c.remove(a, b));
        self.cursor = a;
        self.sel = None;
    }

    fn paste(&mut self) {
        let Some(cb) = self.clipboard.clone() else { return };
        let at = self.sel.map_or(self.cursor, |s| s.0);
        let replace = self.sel.take();
        self.snapshot();
        if let Some((a, b)) = replace {
            self.clip.remove(a, b);
        }
        self.clip.insert(at, &cb);
        let len = cb.resampled(self.clip.rate).len();
        self.sel = Some((at, at + len));
        self.modified = true;
        self.changed();
    }

    fn trim(&mut self) {
        let Some((a, b)) = self.sel else { return };
        self.edit(|c, _, _| *c = c.slice(a, b));
        self.sel = None;
        self.cursor = 0;
        self.fit_pending = true;
    }

    fn insert_clip(&mut self, part: Clip) {
        self.snapshot();
        let at = self.cursor;
        let len = part.resampled(self.clip.rate).len();
        if self.clip.len() == 0 || self.clip.peak(0, self.clip.len()) == 0.0 && self.file.is_none() {
            self.clip = part.resampled(self.clip.rate);
            self.sel = Some((0, self.clip.len()));
        } else {
            self.clip.insert(at, &part);
            self.sel = Some((at, at + len));
        }
        self.modified = true;
        self.fit_pending = true;
        self.changed();
    }

    fn toggle_record(&mut self) {
        match self.recorder.take() {
            Some(r) => {
                let take = r.finish();
                if take.len() > 0 {
                    self.insert_clip(take);
                }
            }
            None => {
                self.player.stop();
                match Recorder::start() {
                    Ok(r) => self.recorder = Some(r),
                    Err(e) => self.error = Some(format!("Enregistrement : {e}")),
                }
            }
        }
    }

    fn export(&mut self, bits: u16) {
        let stem = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("audio".into());
        let Some(path) = rfd::FileDialog::new().add_filter("WAV", &["wav"]).set_file_name(format!("{stem}.wav")).save_file() else { return };
        match self.clip.save_wav(&path, bits) {
            Ok(()) => self.modified = false,
            Err(e) => self.error = Some(format!("Export : {e}")),
        }
    }

    fn toggle_play(&mut self) {
        let (playing, _) = self.player.state();
        if playing {
            self.player.stop();
        } else {
            let (a, b) = self.sel.unwrap_or((self.cursor, self.clip.len()));
            self.player.play(a, b, self.looping && self.sel.is_some());
        }
    }

    fn add_marker(&mut self) {
        let tol = (self.clip.rate / 50) as usize;
        if let Some(k) = self.markers.iter().position(|m| m.abs_diff(self.cursor) < tol) {
            self.markers.remove(k);
        } else {
            self.markers.push(self.cursor);
            self.markers.sort();
        }
    }

    fn jump(&mut self, dir: i32) {
        let c = self.cursor;
        let target = if dir > 0 { self.markers.iter().copied().find(|m| *m > c) } else { self.markers.iter().rev().copied().find(|m| *m < c) };
        if let Some(t) = target {
            self.cursor = t;
            self.sel = None;
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
        if sc(M::COMMAND, Key::T) {
            self.trim();
        }
        if sc(M::COMMAND, Key::A) {
            self.sel = Some((0, self.clip.len()));
        }
        if sc(M::COMMAND, Key::O) {
            self.open_dialog();
        }
        if sc(M::COMMAND, Key::Num0) {
            self.fit_pending = true;
        }
        let (space, del, home, end, m, l, r, up, down) = ctx.input(|i| {
            let plain = !i.modifiers.command;
            (
                i.key_pressed(Key::Space),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::Home),
                i.key_pressed(Key::End),
                i.key_pressed(Key::M) && plain,
                i.key_pressed(Key::L) && plain,
                i.key_pressed(Key::R) && plain,
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::ArrowDown),
            )
        });
        if space {
            self.toggle_play();
        }
        if del {
            self.cut(false);
        }
        if home {
            self.cursor = 0;
            self.sel = None;
        }
        if end {
            self.cursor = self.clip.len();
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
}

fn fmt_time(s: f64) -> String {
    let m = (s / 60.0).floor();
    format!("{:02}:{:06.3}", m as u64, s - m * 60.0)
}

impl eframe::App for Sound {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        self.keys(ctx);
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf())).filter(|p| !p.as_os_str().is_empty()) {
            self.open_path(&p);
        }
        let name = self.file.as_ref().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = format!("{name}{} â€” Sound", if self.modified { " â€¢" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        let (playing, pos) = self.player.state();
        if playing || self.recorder.is_some() {
            ctx.request_repaint();
        }
        self.menu(root);
        self.transport(root, playing, pos);
        self.status(root);
        self.dock(root, pos, playing);
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD).inner_margin(Margin::same(14))).show(root, |ui| {
            self.waveform(ui, playing, pos);
        });
        self.help.show(ctx);
    }
}

impl Sound {
    fn menu(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(34.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(10, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add(egui::Image::new((self.icon.id(), Vec2::splat(20.0))));
                ui.add_space(6.0);
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouveau (5 s de silence)").clicked() {
                            self.clip = Clip { rate: 44_100, ch: vec![vec![0.0; 44_100 * 5]; 2] };
                            self.file = None;
                            self.undo.clear();
                            self.fit_pending = true;
                            self.changed();
                        }
                        if ui.button("Ouvrirâ€¦                    Ctrl+O").clicked() {
                            self.open_dialog();
                        }
                        ui.separator();
                        for (label, bits) in [("Exporter en WAV 16 bitsâ€¦", 16), ("Exporter en WAV 24 bitsâ€¦", 24), ("Exporter en WAV 32 bits flottantâ€¦", 32)] {
                            if ui.button(label).clicked() {
                                self.export(bits);
                            }
                        }
                    });
                    ui.menu_button("Ã‰dition", |ui| {
                        let has = self.sel.is_some();
                        if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler                 Ctrl+Z")).clicked() {
                            self.undo();
                        }
                        if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("RÃ©tablir                Ctrl+Y")).clicked() {
                            self.redo();
                        }
                        ui.separator();
                        if ui.add_enabled(has, egui::Button::new("Couper                   Ctrl+X")).clicked() {
                            self.cut(true);
                        }
                        if ui.add_enabled(has, egui::Button::new("Copier                   Ctrl+C")).clicked()
                            && let Some((a, b)) = self.sel
                        {
                            self.clipboard = Some(self.clip.slice(a, b));
                        }
                        if ui.add_enabled(self.clipboard.is_some(), egui::Button::new("Coller                    Ctrl+V")).clicked() {
                            self.paste();
                        }
                        if ui.add_enabled(has, egui::Button::new("Rogner                   Ctrl+T")).clicked() {
                            self.trim();
                        }
                        if ui.add_enabled(has, egui::Button::new("Supprimer               Suppr")).clicked() {
                            self.cut(false);
                        }
                        ui.separator();
                        if ui.button("Tout sÃ©lectionner       Ctrl+A").clicked() {
                            self.sel = Some((0, self.clip.len()));
                        }
                        if ui.button("Ajouter un marqueur      M").clicked() {
                            self.add_marker();
                        }
                        if ui.add_enabled(!self.markers.is_empty(), egui::Button::new("Supprimer les marqueurs")).clicked() {
                            self.markers.clear();
                        }
                    });
                    ui.menu_button("Effets", |ui| {
                        if ui.button("Normaliser (âˆ’1 dB)").clicked() {
                            self.edit(|c, a, b| c.normalize(a, b, -1.0));
                        }
                        if ui.button("Volume perÃ§u (âˆ’16 dB RMS)").clicked() {
                            self.edit(|c, a, b| c.rms_normalize(a, b, -16.0));
                        }
                        if ui.button("Fondu entrant").clicked() {
                            self.edit(|c, a, b| c.fade(a, b, true));
                        }
                        if ui.button("Fondu sortant").clicked() {
                            self.edit(|c, a, b| c.fade(a, b, false));
                        }
                        ui.separator();
                        if ui.button("Coupe-bas 80 Hz").clicked() {
                            self.edit(|c, a, b| c.high_pass(a, b, 80.0));
                        }
                        if ui.button("Coupe-haut 12 kHz").clicked() {
                            self.edit(|c, a, b| c.filter(a, b, 3, 12_000.0, 0.0, 0.707));
                        }
                        if ui.button("Porte de bruit (âˆ’45 dB)").clicked() {
                            self.edit(|c, a, b| c.gate(a, b, -45.0));
                        }
                        ui.separator();
                        if ui.button("Inverser (Ã  l'envers)").clicked() {
                            self.edit(|c, a, b| c.reverse(a, b));
                        }
                        if ui.button("Inverser la polaritÃ©").clicked() {
                            self.edit(|c, a, b| c.invert(a, b));
                        }
                        if ui.button("Silence").clicked() {
                            self.edit(|c, a, b| c.silence(a, b));
                        }
                        ui.separator();
                        if ui.button("Convertir en mono").clicked() {
                            self.edit(|c, _, _| c.to_mono());
                        }
                        if ui.button("Ã‰changer gauche et droite").clicked() {
                            self.edit(|c, _, _| c.swap());
                        }
                    });
                    ui.menu_button("GÃ©nÃ©rer", |ui| {
                        let secs = self.fx.gen_secs;
                        for (label, kind) in [("Silence", 0u8), ("TonalitÃ© 440 Hz", 1), ("Bruit blanc", 2)] {
                            if ui.button(format!("{label} ({secs:.1} s)")).clicked() {
                                let mut part = Clip { rate: self.clip.rate, ch: vec![Vec::new(); self.clip.ch.len().max(1)] };
                                part.generate(0, secs, kind);
                                self.insert_clip(part);
                            }
                        }
                        ui.add(egui::Slider::new(&mut self.fx.gen_secs, 0.1..=60.0).logarithmic(true).suffix(" s"));
                    });
                    ui.menu_button("Affichage", |ui| {
                        if ui.button("Tout afficher             Ctrl+0").clicked() {
                            self.fit_pending = true;
                        }
                        ui.checkbox(&mut self.looping, "Lecture en boucle         L");
                    });
                    self.help.menu(ui);
                });
            });
        });
    }

    fn transport(&mut self, root: &mut Ui, playing: bool, pos: usize) {
        egui::Panel::top("transport").exact_size(60.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(16, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(42.0), Sense::click());
                ui.painter().circle_filled(r.center(), 21.0, if resp.hovered() { ACCENT.lerp_to_gamma(Color32::WHITE, 0.12) } else { ACCENT });
                let c = r.center();
                if playing {
                    for dx in [-4.5, 4.5] {
                        ui.painter().rect_filled(Rect::from_center_size(c + vec2(dx, 0.0), vec2(4.5, 15.0)), CornerRadius::same(1), Color32::WHITE);
                    }
                } else {
                    ui.painter().add(egui::Shape::convex_polygon(vec![c + vec2(-5.0, -9.0), c + vec2(9.0, 0.0), c + vec2(-5.0, 9.0)], Color32::WHITE, Stroke::NONE));
                }
                if resp.on_hover_text("Lecture / pause (Espace)").clicked() {
                    self.toggle_play();
                }
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                ui.painter().circle_filled(r.center(), 17.0, if resp.hovered() { FIELD } else { Color32::TRANSPARENT });
                ui.painter().rect_filled(Rect::from_center_size(r.center(), Vec2::splat(11.0)), CornerRadius::same(2), INK);
                if resp.on_hover_text("Stop").clicked() {
                    self.player.stop();
                }
                let rec = self.recorder.is_some();
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                ui.painter().circle_filled(r.center(), 17.0, if rec { RED.gamma_multiply(0.25) } else if resp.hovered() { FIELD } else { Color32::TRANSPARENT });
                ui.painter().circle_filled(r.center(), 7.0, RED);
                if resp.on_hover_text("Enregistrer au micro (R)").clicked() {
                    self.toggle_record();
                }
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                ui.painter().circle_filled(r.center(), 17.0, if self.looping { ACCENT.gamma_multiply(0.35) } else if resp.hovered() { FIELD } else { Color32::TRANSPARENT });
                ui.painter().text(r.center(), Align2::CENTER_CENTER, "âŸ²", FontId::proportional(18.0), if self.looping { INK_STRONG } else { MUTED });
                if resp.on_hover_text("Boucle (L)").clicked() {
                    self.looping = !self.looping;
                }
                ui.add_space(14.0);
                let shown = if playing { pos } else { self.cursor };
                ui.label(RichText::new(fmt_time(self.clip.secs(shown))).font(FontId::monospace(22.0)).color(INK_STRONG));
                ui.label(RichText::new(format!("/ {}", fmt_time(self.clip.secs(self.clip.len())))).monospace().color(MUTED));
                if let Some(r) = &self.recorder {
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("â— Enregistrement {:.1} s", r.seconds())).font(semibold(13.0)).color(RED));
                } else if let Some((a, b)) = self.sel {
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("SÃ©lection {}", fmt_time(self.clip.secs(b - a)))).font(semibold(13.0)).color(WAVE));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::secondary(ui, "?  Tutoriels").clicked() {
                        self.help.learn("Lecture");
                    }
                    ui.label(RichText::new(format!("{} Hz Â· {}", self.clip.rate, if self.clip.ch.len() > 1 { "stÃ©rÃ©o" } else { "mono" })).size(12.5).color(MUTED));
                });
            });
        });
    }

    fn status(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(RichText::new("Glisser = sÃ©lection Â· molette = zoom Â· Maj+molette = dÃ©filer Â· M marqueur Â· â†‘â†“ marqueurs Â· R enregistrer").size(12.0).color(MUTED));
                if let Some(e) = self.error.clone() {
                    ui.label(RichText::new(e).size(12.0).color(RED));
                    if ui.small_button("Ã—").clicked() {
                        self.error = None;
                    }
                }
            });
        });
    }

    fn dock(&mut self, root: &mut Ui, pos: usize, playing: bool) {
        egui::Panel::right("fx").resizable(false).exact_size(292.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let scope = if self.sel.is_some() { "sur la sÃ©lection" } else { "sur tout le fichier" };
                let at = if playing { pos } else { self.sel.map_or(self.cursor, |s| s.0) };
                if self.spectrum.as_ref().is_none_or(|s| s.0.abs_diff(at) > (self.clip.rate / 10) as usize || s.1 != self.version) {
                    self.spectrum = Some((at, self.version, self.clip.spectrum(at, 96)));
                }
                if let Some((_, _, spec)) = &self.spectrum {
                    let spec = spec.clone();
                    pro::dock_panel(ui, "analysis", "Analyse", |ui| draw_spectrum(ui, &spec));
                }
                let mut action: Option<Box<dyn FnOnce(&mut Clip, usize, usize)>> = None;
                let fx = &mut self.fx;
                pro::dock_panel(ui, "amp", "Amplitude", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.label(RichText::new(format!("Les effets s'appliquent {scope}")).size(12.0).color(FAINT));
                    slider(ui, "Gain", &mut fx.gain, -24.0..=24.0, " dB");
                    let g = fx.gain;
                    if apply(ui) {
                        action = Some(Box::new(move |c, a, b| c.gain_db(a, b, g)));
                    }
                });
                pro::dock_panel(ui, "eq", "Ã‰galiseur", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    slider(ui, "Graves (120 Hz)", &mut fx.eq[0], -12.0..=12.0, " dB");
                    slider(ui, "MÃ©diums (1 kHz)", &mut fx.eq[1], -12.0..=12.0, " dB");
                    slider(ui, "Aigus (8 kHz)", &mut fx.eq[2], -12.0..=12.0, " dB");
                    let eq = fx.eq;
                    if apply(ui) {
                        action = Some(Box::new(move |c, a, b| {
                            c.filter(a, b, 1, 120.0, eq[0], 0.707);
                            c.filter(a, b, 0, 1000.0, eq[1], 0.9);
                            c.filter(a, b, 2, 8000.0, eq[2], 0.707);
                        }));
                    }
                });
                pro::dock_panel(ui, "dyn", "Dynamique", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    slider(ui, "Seuil", &mut fx.comp[0], -60.0..=0.0, " dB");
                    slider(ui, "Ratio", &mut fx.comp[1], 1.0..=20.0, ":1");
                    slider(ui, "Gain de sortie", &mut fx.comp[2], 0.0..=24.0, " dB");
                    let cp = fx.comp;
                    if pro::secondary(ui, "Compresser").clicked() {
                        action = Some(Box::new(move |c, a, b| c.compress(a, b, cp[0], cp[1], cp[2])));
                    }
                    ui.add_space(4.0);
                    slider(ui, "Porte de bruit", &mut fx.gate, -80.0..=-10.0, " dB");
                    let gt = fx.gate;
                    if pro::secondary(ui, "Appliquer la porte").clicked() {
                        action = Some(Box::new(move |c, a, b| c.gate(a, b, gt)));
                    }
                    ui.add_space(4.0);
                    slider(ui, "Saturation", &mut fx.drive, 0.0..=1.0, "");
                    let dr = fx.drive;
                    if pro::secondary(ui, "Saturer").clicked() {
                        action = Some(Box::new(move |c, a, b| c.saturate(a, b, dr)));
                    }
                });
                pro::dock_panel(ui, "space", "Espace", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    slider(ui, "DÃ©lai d'Ã©cho", &mut fx.echo[0], 20.0..=1500.0, " ms");
                    slider(ui, "RÃ©injection", &mut fx.echo[1], 0.0..=0.9, "");
                    slider(ui, "MÃ©lange", &mut fx.echo[2], 0.0..=1.0, "");
                    let e = fx.echo;
                    if pro::secondary(ui, "Ajouter l'Ã©cho").clicked() {
                        action = Some(Box::new(move |c, a, b| c.echo(a, b, e[0], e[1], e[2])));
                    }
                    ui.add_space(4.0);
                    slider(ui, "Taille de la piÃ¨ce", &mut fx.verb[0], 0.0..=1.0, "");
                    slider(ui, "MÃ©lange rÃ©verb", &mut fx.verb[1], 0.0..=1.0, "");
                    let v = fx.verb;
                    if pro::secondary(ui, "Ajouter la rÃ©verbÃ©ration").clicked() {
                        action = Some(Box::new(move |c, a, b| c.reverb(a, b, v[0], v[1])));
                    }
                });
                pro::dock_panel(ui, "time", "Vitesse et stÃ©rÃ©o", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    slider(ui, "Vitesse", &mut fx.speed, 25.0..=400.0, " %");
                    let sp = fx.speed;
                    if pro::secondary(ui, "Changer la vitesse").clicked() {
                        action = Some(Box::new(move |c, a, b| c.speed(a, b, sp / 100.0)));
                    }
                    ui.add_space(4.0);
                    slider(ui, "Balance", &mut fx.pan, -1.0..=1.0, "");
                    slider(ui, "Largeur stÃ©rÃ©o", &mut fx.width, 0.0..=200.0, " %");
                    let (pn, wd) = (fx.pan, fx.width);
                    if pro::secondary(ui, "Appliquer la stÃ©rÃ©o").clicked() {
                        action = Some(Box::new(move |c, a, b| c.stereo(a, b, pn, wd / 100.0)));
                    }
                });
                if let Some(f) = action {
                    let old = self.sel;
                    self.edit(f);
                    if old.is_none() {
                        self.fit_pending = true;
                    }
                }
            });
        });
    }

    fn waveform(&mut self, ui: &mut Ui, playing: bool, pos: usize) {
        let area = ui.max_rect();
        let ruler_h = 24.0;
        let wave = Rect::from_min_max(area.min + vec2(0.0, ruler_h), area.max);
        let resp = ui.allocate_rect(area, Sense::click_and_drag());
        let n = self.clip.len().max(1);
        let width = wave.width().max(1.0);
        if self.fit_pending {
            self.spp = n as f64 / width as f64;
            self.view_start = 0.0;
            self.fit_pending = false;
        }
        let max_spp = n as f64 / width as f64;
        let x_of = |s: f64, start: f64, spp: f64| wave.min.x + ((s - start) / spp) as f32;
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
        let visible = width as f64 * self.spp;
        self.view_start = self.view_start.clamp(0.0, (n as f64 - visible).max(0.0));
        let (start, spp) = (self.view_start, self.spp);
        let sample_at = |x: f32| ((start + (x - wave.min.x) as f64 * spp).round().max(0.0) as usize).min(n);
        if let Some(p) = resp.interact_pointer_pos() {
            let s = sample_at(p.x);
            if resp.drag_started() {
                self.drag_from = Some(s);
            }
            if resp.dragged()
                && let Some(f) = self.drag_from
            {
                self.sel = if s.abs_diff(f) > (2.0 * spp) as usize { Some((f.min(s), f.max(s))) } else { None };
            }
            if resp.clicked() {
                self.cursor = s;
                self.sel = None;
                if playing {
                    self.player.play(s, self.clip.len(), false);
                }
            }
        }
        if resp.drag_stopped() {
            if let Some((a, _)) = self.sel {
                self.cursor = a;
            }
            self.drag_from = None;
        }
        let cols = width as usize;
        let key = (start as usize, spp, cols, self.version);
        if self.peaks.as_ref().is_none_or(|p| p.key != key) {
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
        let painter = ui.painter_at(area);
        painter.rect_filled(Rect::from_min_max(area.min, pos2(area.max.x, wave.min.y)), CornerRadius::same(6), PANEL);
        let secs_visible = visible / self.clip.rate as f64;
        let step = [0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 300.0].into_iter().find(|s| secs_visible / s <= 12.0).unwrap_or(600.0);
        let mut t = (self.clip.secs(start as usize) / step).floor() * step;
        while t <= self.clip.secs(start as usize) + secs_visible {
            let x = x_of(t * self.clip.rate as f64, start, spp);
            painter.line_segment([pos2(x, area.min.y + 14.0), pos2(x, wave.min.y)], Stroke::new(1.0, FAINT));
            painter.text(pos2(x + 3.0, area.min.y + 3.0), Align2::LEFT_TOP, fmt_time(t), FontId::monospace(10.0), MUTED);
            t += step;
        }
        if let Some((a, b)) = self.sel {
            let r = Rect::from_x_y_ranges(x_of(a as f64, start, spp)..=x_of(b as f64, start, spp), wave.y_range());
            painter.rect_filled(r, CornerRadius::ZERO, ACCENT.gamma_multiply(0.22));
        }
        let nch = self.clip.ch.len().max(1);
        let lane_h = wave.height() / nch as f32;
        if let Some(peaks) = &self.peaks {
            for (ci, col) in peaks.cols.iter().enumerate() {
                let lane = Rect::from_min_size(pos2(wave.min.x, wave.min.y + ci as f32 * lane_h), vec2(width, lane_h));
                let mid = lane.center().y;
                let amp = lane_h * 0.45;
                painter.rect_filled(lane.shrink2(vec2(0.0, 3.0)), CornerRadius::same(8), Color32::from_rgb(0x1c, 0x1c, 0x22));
                painter.line_segment([pos2(lane.min.x, mid), pos2(lane.max.x, mid)], Stroke::new(1.0, LINE));
                painter.text(lane.left_top() + vec2(10.0, 8.0), Align2::LEFT_TOP, if nch == 1 { "Mono" } else if ci == 0 { "G" } else { "D" }, semibold(11.0), FAINT);
                for (x, (lo, hi)) in col.iter().enumerate() {
                    if lo > hi {
                        continue;
                    }
                    let px = wave.min.x + x as f32 + 0.5;
                    let y0 = mid - hi.clamp(-1.0, 1.0) * amp;
                    let y1 = (mid - lo.clamp(-1.0, 1.0) * amp).max(y0 + 1.0);
                    painter.line_segment([pos2(px, y0), pos2(px, y1)], Stroke::new(1.0, WAVE));
                }
            }
        }
        for m in &self.markers {
            let x = x_of(*m as f64, start, spp);
            if x >= wave.min.x && x <= wave.max.x {
                painter.line_segment([pos2(x, wave.min.y), pos2(x, area.max.y)], Stroke::new(1.0, MARK.gamma_multiply(0.6)));
                painter.add(egui::Shape::convex_polygon(vec![pos2(x - 5.0, area.min.y + 12.0), pos2(x + 5.0, area.min.y + 12.0), pos2(x, wave.min.y)], MARK, Stroke::NONE));
            }
        }
        let line = |p: &egui::Painter, s: usize, c: Color32| {
            let x = x_of(s as f64, start, spp);
            if x >= wave.min.x && x <= wave.max.x {
                p.line_segment([Pos2::new(x, area.min.y), Pos2::new(x, area.max.y)], Stroke::new(1.5, c));
            }
        };
        line(&painter, self.cursor, INK);
        if playing {
            line(&painter, pos, ACCENT.lerp_to_gamma(Color32::WHITE, 0.3));
            if x_of(pos as f64, start, spp) > wave.max.x {
                self.view_start = pos as f64;
            }
        }
    }
}

fn slider(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) {
    ui.label(RichText::new(label).size(12.5).color(MUTED));
    ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(80.0);
    ui.add(egui::Slider::new(v, range).suffix(suffix).max_decimals(2));
}

fn apply(ui: &mut Ui) -> bool {
    pro::secondary(ui, "Appliquer").clicked()
}

fn draw_spectrum(ui: &mut Ui, spec: &[f32]) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 90.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(8), Color32::from_rgb(0x14, 0x14, 0x17));
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
        viewport: egui::ViewportBuilder::default()
            .with_title("Sound")
            .with_icon(Arc::new(studio_ui::app_window_icon("sound")))
            .with_inner_size([1400.0, 860.0])
            .with_min_inner_size([1000.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Sound", options, Box::new(|cc| Ok(Box::new(Sound::new(cc)))))
}
