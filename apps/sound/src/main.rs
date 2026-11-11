#![windows_subsystem = "windows"]

mod audio;

use audio::Clip;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui::{
    self, Color32, CornerRadius, Frame, Key, Margin, Pos2, Rect, RichText, Sense, Stroke, pos2,
    vec2,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use studio_ui::{BG, CARD, DANGER, MUTED, PANEL, TEXT};

const ACCENT: Color32 = Color32::from_rgb(120, 230, 90);
const UNDO_LIMIT: usize = 20;

struct Shared {
    clip: Arc<Clip>,
    pos: f64,
    playing: bool,
    end: usize,
}

struct Player {
    shared: Arc<Mutex<Shared>>,
    _stream: Option<cpal::Stream>,
    error: Option<String>,
}

impl Player {
    fn new(clip: Arc<Clip>) -> Self {
        let shared = Arc::new(Mutex::new(Shared { clip, pos: 0.0, playing: false, end: 0 }));
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
                        let i = s.pos as usize;
                        if s.playing && i < s.end {
                            for (c, out) in frame.iter_mut().enumerate() {
                                *out = s.clip.ch[c % nch][i];
                            }
                            s.pos += step;
                        } else {
                            if s.playing {
                                s.playing = false;
                            }
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

    fn play(&self, from: usize, to: usize) {
        let mut s = self.shared.lock().unwrap();
        s.pos = from as f64;
        s.end = to.min(s.clip.len());
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

struct Peaks {
    key: (usize, f64, usize, u64),
    cols: Vec<Vec<(f32, f32)>>,
}

struct Sound {
    clip: Clip,
    version: u64,
    file: Option<PathBuf>,
    modified: bool,
    player: Player,
    undo: Vec<Clip>,
    redo: Vec<Clip>,
    clipboard: Option<Clip>,
    cursor: usize,
    sel: Option<(usize, usize)>,
    drag_from: Option<usize>,
    view_start: f64,
    spp: f64,
    fit_pending: bool,
    peaks: Option<Peaks>,
    gain: f32,
    error: Option<String>,
    title: String,
}

impl Sound {
    fn new(cc: &eframe::CreationContext) -> Self {
        studio_ui::apply_style(&cc.egui_ctx);
        cc.egui_ctx.all_styles_mut(|s| s.visuals.selection.bg_fill = ACCENT);
        let clip = Clip { rate: 44_100, ch: vec![vec![0.0; 44_100 * 5], vec![0.0; 44_100 * 5]] };
        let player = Player::new(Arc::new(clip.clone()));
        let mut me = Self {
            error: player.error.clone().map(|e| format!("Lecture audio indisponible : {e}")),
            clip,
            version: 0,
            file: None,
            modified: false,
            player,
            undo: Vec::new(),
            redo: Vec::new(),
            clipboard: None,
            cursor: 0,
            sel: None,
            drag_from: None,
            view_start: 0.0,
            spp: 100.0,
            fit_pending: true,
            peaks: None,
            gain: 0.0,
            title: String::new(),
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
    }

    fn edit(&mut self, f: impl FnOnce(&mut Clip, usize, usize)) {
        if self.clip.len() == 0 {
            return;
        }
        let (a, b) = self.sel.unwrap_or((0, self.clip.len()));
        self.undo.push(self.clip.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
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
        let replace = self.sel;
        self.sel = None;
        self.undo.push(self.clip.clone());
        self.redo.clear();
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

    fn export(&mut self, float: bool) {
        let stem = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("audio".into());
        let Some(path) = rfd::FileDialog::new().add_filter("WAV", &["wav"]).set_file_name(format!("{stem}.wav")).save_file() else { return };
        match self.clip.save_wav(&path, float) {
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
            self.player.play(a, b);
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
        let (space, del, home, end) = ctx.input(|i| {
            (i.key_pressed(Key::Space), i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace), i.key_pressed(Key::Home), i.key_pressed(Key::End))
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
    }

    fn open_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("Audio", &["wav", "mp3", "flac", "ogg", "m4a", "aac", "mp4", "mkv", "webm"])
            .pick_file()
        {
            self.open_path(&p);
        }
    }
}

fn fmt_time(s: f64) -> String {
    let m = (s / 60.0).floor();
    format!("{:02}:{:06.3}", m as u64, s - m * 60.0)
}

impl eframe::App for Sound {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        self.keys(ctx);
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf())).filter(|p| !p.as_os_str().is_empty()) {
            self.open_path(&p);
        }
        let name = self.file.as_ref().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = format!("Sound — {name}{}", if self.modified { " •" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        let (playing, pos) = self.player.state();
        if playing {
            ctx.request_repaint();
        }

        egui::Panel::top("top")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 8)))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    studio_ui::app_icon(ui, 26.0, "sound", ACCENT);
                    ui.label(RichText::new("Sound").size(17.0).strong().color(TEXT));
                    ui.add_space(10.0);
                    if ui.button("Ouvrir…").clicked() {
                        self.open_dialog();
                    }
                    ui.menu_button("Exporter", |ui| {
                        if ui.button("WAV 16 bits").clicked() {
                            self.export(false);
                        }
                        if ui.button("WAV 32 bits flottant").clicked() {
                            self.export(true);
                        }
                    });
                    ui.separator();
                    let label = if playing { "⏸ Pause" } else { "▶ Lire" };
                    if ui.add(egui::Button::new(RichText::new(label).strong().color(Color32::from_rgb(14, 15, 20))).fill(ACCENT).min_size(vec2(90.0, 28.0))).clicked() {
                        self.toggle_play();
                    }
                    if ui.button("⏹").on_hover_text("Stop").clicked() {
                        self.player.stop();
                    }
                    ui.separator();
                    let shown = if playing { pos } else { self.cursor };
                    ui.label(RichText::new(fmt_time(self.clip.secs(shown))).monospace().size(18.0).color(TEXT));
                    ui.label(RichText::new(format!("/ {}", fmt_time(self.clip.secs(self.clip.len())))).monospace().color(MUTED));
                    if let Some((a, b)) = self.sel {
                        ui.separator();
                        ui.label(RichText::new(format!("Sélection {}", fmt_time(self.clip.secs(b - a)))).color(ACCENT));
                    }
                });
            });

        egui::Panel::bottom("status")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 5)))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                    ui.label(small(format!("{} Hz · {} canal(aux)", self.clip.rate, self.clip.ch.len())));
                    ui.separator();
                    ui.label(small("Espace lire · glisser = sélection · molette = zoom · Maj+molette = défiler · Ctrl+X/C/V · Suppr · Ctrl+T rogner".into()));
                    if let Some(e) = self.error.clone() {
                        ui.separator();
                        ui.label(RichText::new(e).size(12.0).color(DANGER));
                        if ui.small_button("×").clicked() {
                            self.error = None;
                        }
                    }
                });
            });

        egui::Panel::right("fx")
            .resizable(false)
            .exact_size(260.0)
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::same(14)))
            .show(root, |ui| {
                let scope = if self.sel.is_some() { "sur la sélection" } else { "sur tout le fichier" };
                ui.label(RichText::new(format!("EFFETS — {scope}")).size(11.0).color(MUTED));
                ui.add_space(6.0);
                ui.add(egui::Slider::new(&mut self.gain, -24.0..=24.0).suffix(" dB").text("Gain"));
                if ui.button("Appliquer le gain").clicked() {
                    let g = self.gain;
                    self.edit(|c, a, b| c.gain_db(a, b, g));
                }
                ui.add_space(6.0);
                let w = ui.available_width();
                let fx = |ui: &mut egui::Ui, label: &str| ui.add_sized([w, 28.0], egui::Button::new(label)).clicked();
                if fx(ui, "Normaliser (−1 dB)") {
                    self.edit(|c, a, b| c.normalize(a, b, -1.0));
                }
                if fx(ui, "Fondu entrant") {
                    self.edit(|c, a, b| c.fade(a, b, true));
                }
                if fx(ui, "Fondu sortant") {
                    self.edit(|c, a, b| c.fade(a, b, false));
                }
                if fx(ui, "Inverser") {
                    self.edit(|c, a, b| c.reverse(a, b));
                }
                if fx(ui, "Silence") {
                    self.edit(|c, a, b| c.silence(a, b));
                }
                ui.add_space(10.0);
                ui.label(RichText::new("NETTOYAGE").size(11.0).color(MUTED));
                ui.add_space(4.0);
                if fx(ui, "Couper les graves (80 Hz)") {
                    self.edit(|c, a, b| c.high_pass(a, b, 80.0));
                }
                if fx(ui, "Porte de bruit (−45 dB)") {
                    self.edit(|c, a, b| c.gate(a, b, -45.0));
                }
                ui.add_space(10.0);
                ui.label(RichText::new("ÉDITION").size(11.0).color(MUTED));
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    let has = self.sel.is_some();
                    if ui.add_enabled(has, egui::Button::new("Couper")).clicked() {
                        self.cut(true);
                    }
                    if ui.add_enabled(has, egui::Button::new("Copier")).clicked() {
                        let (a, b) = self.sel.unwrap();
                        self.clipboard = Some(self.clip.slice(a, b));
                    }
                    if ui.add_enabled(self.clipboard.is_some(), egui::Button::new("Coller")).clicked() {
                        self.paste();
                    }
                    if ui.add_enabled(has, egui::Button::new("Rogner")).clicked() {
                        self.trim();
                    }
                    if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler")).clicked() {
                        self.undo();
                    }
                    if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir")).clicked() {
                        self.redo();
                    }
                });
            });

        egui::CentralPanel::default().frame(Frame::new().fill(BG).inner_margin(Margin::same(12))).show(root, |ui| {
            self.waveform(ui, playing, pos);
        });
    }
}

impl Sound {
    fn waveform(&mut self, ui: &mut egui::Ui, playing: bool, pos: usize) {
        let area = ui.max_rect();
        let ruler_h = 22.0;
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
                    self.player.play(s, self.clip.len());
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
        painter.rect_filled(Rect::from_min_max(area.min, pos2(area.max.x, wave.min.y)), CornerRadius::ZERO, PANEL);
        let secs_visible = visible / self.clip.rate as f64;
        let step = [0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 300.0].into_iter().find(|s| secs_visible / s <= 12.0).unwrap_or(600.0);
        let t0 = (self.clip.secs(start as usize) / step).floor() * step;
        let mut t = t0;
        while t <= self.clip.secs(start as usize) + secs_visible {
            let x = x_of(t * self.clip.rate as f64, start, spp);
            painter.line_segment([pos2(x, area.min.y + 12.0), pos2(x, wave.min.y)], Stroke::new(1.0, MUTED));
            painter.text(pos2(x + 3.0, area.min.y + 2.0), egui::Align2::LEFT_TOP, fmt_time(t), egui::FontId::monospace(10.0), MUTED);
            t += step;
        }

        if let Some((a, b)) = self.sel {
            let r = Rect::from_x_y_ranges(x_of(a as f64, start, spp)..=x_of(b as f64, start, spp), wave.y_range());
            painter.rect_filled(r, CornerRadius::ZERO, ACCENT.gamma_multiply(0.15));
        }

        let nch = self.clip.ch.len().max(1);
        let lane_h = wave.height() / nch as f32;
        if let Some(peaks) = &self.peaks {
            for (ci, col) in peaks.cols.iter().enumerate() {
                let lane = Rect::from_min_size(pos2(wave.min.x, wave.min.y + ci as f32 * lane_h), vec2(width, lane_h));
                let mid = lane.center().y;
                let amp = lane_h * 0.45;
                painter.rect_filled(lane.shrink2(vec2(0.0, 2.0)), CornerRadius::same(4), CARD);
                painter.line_segment([pos2(lane.min.x, mid), pos2(lane.max.x, mid)], Stroke::new(1.0, MUTED.gamma_multiply(0.3)));
                for (x, (lo, hi)) in col.iter().enumerate() {
                    if lo > hi {
                        continue;
                    }
                    let px = wave.min.x + x as f32 + 0.5;
                    let y0 = mid - hi.clamp(-1.0, 1.0) * amp;
                    let y1 = (mid - lo.clamp(-1.0, 1.0) * amp).max(y0 + 1.0);
                    painter.line_segment([pos2(px, y0), pos2(px, y1)], Stroke::new(1.0, ACCENT));
                }
            }
        }

        let line = |p: &egui::Painter, s: usize, c: Color32| {
            let x = x_of(s as f64, start, spp);
            if x >= wave.min.x && x <= wave.max.x {
                p.line_segment([Pos2::new(x, area.min.y), Pos2::new(x, area.max.y)], Stroke::new(1.5, c));
            }
        };
        line(&painter, self.cursor, TEXT);
        if playing {
            line(&painter, pos, ACCENT);
            let x = x_of(pos as f64, start, spp);
            if x > wave.max.x {
                self.view_start = pos as f64;
            }
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Sound")
            .with_icon(Arc::new(studio_ui::app_window_icon("sound")))
            .with_inner_size([1300.0, 760.0])
            .with_min_inner_size([900.0, 500.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Sound", options, Box::new(|cc| Ok(Box::new(Sound::new(cc)))))
}
