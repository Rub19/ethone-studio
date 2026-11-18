//! "Texte de la photo": finds the text in a picture (Windows OCR), then lets you
//! rewrite, erase, blur or copy it, keeping the original colour and size.

use crate::Pixel;
use studio_ui::i18n::tl;
use crate::doc::Area;
use crate::lookalike::{self, Hand, Look};
use crate::shapes::{FONTS, TextStyle, render_text};
use eframe::egui::{self, Color32, RichText, Ui};
use image::{Rgba, RgbaImage, imageops};
use std::sync::mpsc::{Receiver, channel};
use studio_ui::pro::{self, FAINT, INK, MUTED, semibold};

#[derive(Clone)]
pub struct Line {
    pub text: String,
    pub orig: String,
    /// x, y, w, h in document pixels
    pub rect: [f32; 4],
    pub color: [u8; 4],
    pub bg: [u8; 4],
    pub on: bool,
    /// Font, grain and softness found for the original text.
    pub look: Option<Look>,
}

pub enum Job {
    Ocr(Result<Vec<Line>, String>),
    Erased(usize, RgbaImage, Vec<Drawn>, &'static str),
}

/// New text already drawn, ready to lay on the erased photo.
pub struct Drawn {
    pub rect: [f32; 4],
    /// The part of the line whose old ink goes (only the changed words of handwriting).
    pub erase: [f32; 4],
    pub text: String,
    pub img: RgbaImage,
    pub x: i64,
    pub y: i64,
}

pub struct State {
    pub lines: Vec<Line>,
    pub langs: Vec<(String, String)>,
    pub lang: usize,
    pub layer: usize,
    pub job: Option<Receiver<Job>>,
    pub busy: String,
    pub sel: Option<usize>,
    pub boxes: bool,
    pub pad: f32,
    pub method: usize,
    pub fit: bool,
    pub font: usize,
    pub bold: bool,
    pub italic: bool,
    pub auto_color: bool,
    pub color: [u8; 4],
    pub size_k: f32,
    pub own_layer: bool,
    pub upscale: bool,
    pub selection_only: bool,
    pub min_conf: usize,
    pub find: String,
    pub repl: String,
    pub case: bool,
    pub blur_px: f32,
    pub auto_font: bool,
    pub hand: usize,
    pub blend: bool,
}

const METHODS: [&str; 4] = ["Auto", "Reconstruction", "Couleur du fond", "Flou du fond"];
const HAND: [&str; 3] = ["Auto", "Lettres d'origine", "Police manuscrite"];

impl Default for State {
    fn default() -> Self {
        let langs = studio_ocr::languages();
        let lang = langs.iter().position(|l| l.0.starts_with("fr")).unwrap_or(0);
        Self {
            lines: Vec::new(),
            langs,
            lang,
            layer: 0,
            job: None,
            busy: String::new(),
            sel: None,
            boxes: true,
            pad: 3.0,
            method: 0,
            fit: true,
            font: 0,
            bold: false,
            italic: false,
            auto_color: true,
            color: [0, 0, 0, 255],
            size_k: 1.0,
            own_layer: true,
            upscale: true,
            selection_only: false,
            min_conf: 1,
            find: String::new(),
            repl: String::new(),
            case: false,
            blur_px: 0.0,
            auto_font: true,
            hand: 0,
            blend: true,
        }
    }
}

/// Renders `text` so its ink fills `rect` the way the original line did.
pub fn line_image(text: &str, rect: [f32; 4], st: &TextStyle, fit: bool, size_k: f32) -> Option<RgbaImage> {
    let target_h = (rect[3] * size_k).max(4.0);
    let big = TextStyle { size: (target_h * 1.6).clamp(12.0, 600.0), ..*st };
    let img = render_text(text, &big)?;
    let (w, h) = img.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p.0[3] > 8 {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    if x1 < x0 {
        return None;
    }
    let ink = imageops::crop_imm(&img, x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image();
    let k = target_h / ink.height() as f32;
    let mut nw = (ink.width() as f32 * k).round().max(1.0);
    if fit && nw > rect[2] * 1.02 {
        nw = rect[2].max(1.0);
    }
    Some(imageops::resize(&ink, nw as u32, target_h.round().max(1.0) as u32, imageops::FilterType::Lanczos3))
}

pub fn mask_of(lines: &[&Line], pad: f32, w: u32, h: u32) -> Vec<bool> {
    let mut m = vec![false; (w * h) as usize];
    for l in lines {
        let x0 = (l.rect[0] - pad).max(0.0) as u32;
        let y0 = (l.rect[1] - pad).max(0.0) as u32;
        let x1 = ((l.rect[0] + l.rect[2] + pad).ceil() as u32).min(w);
        let y1 = ((l.rect[1] + l.rect[3] + pad).ceil() as u32).min(h);
        for y in y0..y1 {
            for x in x0..x1 {
                m[(y * w + x) as usize] = true;
            }
        }
    }
    m
}

fn fill_flat(img: &mut RgbaImage, lines: &[&Line], pad: f32) {
    let (w, h) = img.dimensions();
    for l in lines {
        let (_, bg) = studio_ocr::colors(img, [l.rect[0] - pad - 2.0, l.rect[1] - pad - 2.0, l.rect[2] + 2.0 * pad + 4.0, l.rect[3] + 2.0 * pad + 4.0]);
        let x0 = (l.rect[0] - pad).max(0.0) as u32;
        let y0 = (l.rect[1] - pad).max(0.0) as u32;
        for y in y0..((l.rect[1] + l.rect[3] + pad).ceil() as u32).min(h) {
            for x in x0..((l.rect[0] + l.rect[2] + pad).ceil() as u32).min(w) {
                let a = img.get_pixel(x, y).0[3];
                img.put_pixel(x, y, Rgba([bg[0], bg[1], bg[2], a]));
            }
        }
    }
}

fn blur_rects(img: &mut RgbaImage, lines: &[&Line], pad: f32, block: f32) {
    let (w, h) = img.dimensions();
    for l in lines {
        let b = if block > 0.0 { block } else { (l.rect[3] / 3.0).max(4.0) } as u32;
        let x0 = (l.rect[0] - pad).max(0.0) as u32;
        let y0 = (l.rect[1] - pad).max(0.0) as u32;
        let x1 = ((l.rect[0] + l.rect[2] + pad).ceil() as u32).min(w);
        let y1 = ((l.rect[1] + l.rect[3] + pad).ceil() as u32).min(h);
        let mut by = y0;
        while by < y1 {
            let mut bx = x0;
            while bx < x1 {
                let (ex, ey) = ((bx + b).min(x1), (by + b).min(y1));
                let mut acc = [0u32; 4];
                let n = (ex - bx) * (ey - by);
                for y in by..ey {
                    for x in bx..ex {
                        let p = img.get_pixel(x, y).0;
                        for k in 0..4 {
                            acc[k] += p[k] as u32;
                        }
                    }
                }
                let avg = Rgba(acc.map(|v| (v / n.max(1)) as u8));
                for y in by..ey {
                    for x in bx..ex {
                        img.put_pixel(x, y, avg);
                    }
                }
                bx += b;
            }
            by += b;
        }
    }
}

fn hex(c: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

impl Pixel {
    pub fn pt_detect(&mut self, ctx: &egui::Context) {
        if self.pt.job.is_some() {
            return;
        }
        let img = self.composite.clone();
        let (off, src) = match self.doc.selection.filter(|_| self.pt.selection_only) {
            Some(a) => ((a.x0 as f32, a.y0 as f32), imageops::crop_imm(&img, a.x0, a.y0, a.x1 - a.x0, a.y1 - a.y0).to_image()),
            None => ((0.0, 0.0), img.clone()),
        };
        let lang = self.pt.langs.get(self.pt.lang).map(|l| l.0.clone()).unwrap_or_default();
        let (up, min, auto) = (self.pt.upscale, self.pt.min_conf, self.pt.auto_font);
        self.pt.layer = self.doc.active;
        let (tx, rx) = channel();
        self.pt.job = Some(rx);
        self.pt.busy = "Lecture du texte…".into();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let r = studio_ocr::recognize_with(&src, &lang, up).map(|p| {
                p.lines
                    .into_iter()
                    .filter(|l| l.text.trim().chars().filter(|c| c.is_alphanumeric()).count() >= min)
                    .map(|l| {
                        let rect = [l.rect[0] + off.0, l.rect[1] + off.1, l.rect[2], l.rect[3]];
                        let (color, bg) = studio_ocr::colors(&img, rect);
                        let look = auto.then(|| {
                            let (mask, _, _) = lookalike::ink(&img, rect, color, bg);
                            let (font, bold, italic, score) = lookalike::match_font(&mask, &l.text)?;
                            let (grain, soft) = lookalike::texture(&img, rect, &mask);
                            Some(Look { font, bold, italic, score, grain, soft })
                        });
                        Line { text: l.text.clone(), orig: l.text, rect, color, bg, on: true, look: look.flatten() }
                    })
                    .collect()
            });
            let _ = tx.send(Job::Ocr(r));
            ctx.request_repaint();
        });
    }

    pub fn pt_poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.pt.job else { return };
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
        let Ok(msg) = rx.try_recv() else { return };
        self.pt.job = None;
        self.pt.busy.clear();
        match msg {
            Job::Ocr(Ok(lines)) => {
                let n = lines.len();
                self.pt.lines = lines;
                self.pt.sel = None;
                studio_ui::handoff::toast(ctx, if n > 0 { format!("{n} ligne(s) de texte trouvée(s)") } else { "Aucun texte trouvé".into() }, n > 0);
            }
            Job::Ocr(Err(e)) => studio_ui::handoff::toast(ctx, e, false),
            Job::Erased(layer, img, write, label) => {
                if layer < self.doc.layers.len() && self.doc.layers[layer].img.dimensions() == img.dimensions() {
                    self.doc.next_label = Some(label.into());
                    self.doc.checkpoint();
                    self.doc.layers[layer].img = img;
                    self.doc.touch_all();
                    self.pt_write(&write, layer);
                }
            }
        }
    }

    fn pt_style(&self, l: &Line) -> TextStyle {
        let (font, bold, italic) = match l.look.filter(|_| self.pt.auto_font) {
            Some(k) => (k.font, k.bold, k.italic),
            None => (self.pt.font, self.pt.bold, self.pt.italic),
        };
        TextStyle {
            font,
            size: 32.0,
            bold,
            italic,
            underline: false,
            align: 0,
            tracking: 0.0,
            leading: 1.2,
            vertical: false,
            color: if self.pt.auto_color { l.color } else { self.pt.color },
        }
    }

    /// Whether `l` is rewritten with the writer's own letters rather than a font.
    fn pt_handwritten(&self, l: &Line) -> bool {
        match self.pt.hand {
            1 | 2 => true,
            _ => l.look.is_some_and(|k| lookalike::is_handwriting(k.font)),
        }
    }

    /// Draws the new text of `l` so it looks like the original in `src` (the photo before erasing).
    fn pt_draw(&self, l: &Line, src: &RgbaImage) -> Option<Drawn> {
        let text = l.text.trim();
        if text.is_empty() {
            return None;
        }
        let st = self.pt_style(l);
        let (mut img, x, y, erase) = if self.pt_handwritten(l) {
            let (mask, x0, y0) = lookalike::ink(src, l.rect, l.color, l.bg);
            let mut hand = Hand::from_line(src, &mask, x0, y0, &l.orig, (st.font, st.bold, st.italic));
            match self.pt.hand {
                1 => hand.own_letters = true,
                2 => hand.own_letters = false,
                _ => {}
            }
            let (mut img, start, end) = hand.rewrite(&l.orig, text, (st.font, st.bold, st.italic), st.color)?;
            let room = l.rect[0] + l.rect[2] - (x0 + start) as f32;
            if self.pt.fit && img.width() as f32 > room * 1.15 {
                img = imageops::resize(&img, room.max(1.0) as u32, img.height(), imageops::FilterType::Triangle);
            }
            let erase = [(x0 + start) as f32 - 2.0, l.rect[1] - l.rect[3] * 0.2, (end.max(start) - start) as f32 + 4.0, l.rect[3] * 1.4];
            (img, (x0 + start) as i64, y0 as i64, erase)
        } else {
            let img = line_image(text, l.rect, &st, self.pt.fit, self.pt.size_k)?;
            (img, l.rect[0].round() as i64, (l.rect[1] + l.rect[3] * (1.0 - self.pt.size_k) / 2.0).round() as i64, l.rect)
        };
        if self.pt.blend
            && let Some(k) = l.look
        {
            lookalike::blend(&mut img, &k, st.color);
        }
        Some(Drawn { rect: l.rect, erase, text: text.to_owned(), img, x, y })
    }

    fn pt_write(&mut self, drawn: &[Drawn], layer: usize) {
        let (w, h) = (self.doc.width(), self.doc.height());
        let mut flat: Option<RgbaImage> = None;
        for d in drawn {
            let (img, x, y) = (&d.img, d.x, d.y);
            if self.pt.own_layer {
                let mut canvas = RgbaImage::new(w, h);
                imageops::overlay(&mut canvas, img, x, y);
                let name: String = format!("Texte : {}", d.text).chars().take(30).collect();
                self.doc.next_label = Some("Texte de la photo".into());
                self.doc.add_layer(Some(name), Some(canvas));
            } else {
                let base = flat.get_or_insert_with(|| self.doc.layers[layer].img.clone());
                imageops::overlay(base, img, x, y);
            }
        }
        if let Some(img) = flat {
            self.doc.layers[layer].img = img;
            self.doc.touch_all();
        }
        for l in self.pt.lines.iter_mut() {
            if drawn.iter().any(|x| x.rect == l.rect) {
                l.orig = l.text.clone();
            }
        }
    }

    /// Erases the chosen lines from the photo, then (when `write`) draws their new text.
    pub fn pt_apply(&mut self, idx: &[usize], write: bool, ctx: &egui::Context) {
        if self.pt.job.is_some() || idx.is_empty() {
            return;
        }
        let layer = self.pt.layer.min(self.doc.layers.len().saturating_sub(1));
        let lines: Vec<Line> = idx.iter().filter_map(|i| self.pt.lines.get(*i).cloned()).collect();
        let mut img = self.doc.layers[layer].img.clone();
        let (w, h) = img.dimensions();
        let label = if write { "Remplacement du texte" } else { "Texte effacé" };
        let todo: Vec<Drawn> = if write { lines.iter().filter_map(|l| self.pt_draw(l, &self.composite)).collect() } else { Vec::new() };
        let erase: Vec<Line> = if write { todo.iter().filter_map(|d| Some(Line { rect: d.erase, ..lines.iter().find(|l| l.rect == d.rect)?.clone() })).collect() } else { lines };
        let refs: Vec<&Line> = erase.iter().collect();
        let plain = erase.iter().all(|l| l.look.is_some_and(|k| k.grain < 5.0));
        match (self.pt.method, plain) {
            (0, true) => {
                for l in &erase {
                    let p = self.pt.pad;
                    lookalike::paper_fill(&mut img, [l.rect[0] - p, l.rect[1] - p, l.rect[2] + 2.0 * p, l.rect[3] + 2.0 * p], l.color, l.bg, l.look.map_or(0.0, |k| k.grain));
                }
            }
            (0 | 1, _) => {
                let mask = mask_of(&refs, self.pt.pad, w, h);
                let (tx, rx) = channel();
                self.pt.job = Some(rx);
                self.pt.busy = "Reconstruction du fond…".into();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let out = crate::inpaint::content_aware_fill(&img, &mask, 11);
                    let _ = tx.send(Job::Erased(layer, out, todo, label));
                    ctx.request_repaint();
                });
                return;
            }
            (2, _) => fill_flat(&mut img, &refs, self.pt.pad),
            _ => {
                fill_flat(&mut img, &refs, self.pt.pad);
                blur_rects(&mut img, &refs, self.pt.pad + 2.0, 0.0);
            }
        }
        self.doc.next_label = Some(label.into());
        self.doc.checkpoint();
        self.doc.layers[layer].img = img;
        self.doc.touch_all();
        self.pt_write(&todo, layer);
    }

    pub fn pt_blur(&mut self, idx: &[usize]) {
        let layer = self.pt.layer.min(self.doc.layers.len().saturating_sub(1));
        let lines: Vec<&Line> = idx.iter().filter_map(|i| self.pt.lines.get(*i)).collect();
        if lines.is_empty() {
            return;
        }
        let mut img = self.doc.layers[layer].img.clone();
        blur_rects(&mut img, &lines, self.pt.pad, self.pt.blur_px);
        self.doc.next_label = Some("Texte flouté".into());
        self.doc.checkpoint();
        self.doc.layers[layer].img = img;
        self.doc.touch_all();
    }

    fn pt_select(&mut self, idx: &[usize]) {
        let (w, h) = (self.doc.width(), self.doc.height());
        let rects: Vec<[f32; 4]> = idx.iter().filter_map(|i| self.pt.lines.get(*i)).map(|l| l.rect).collect();
        if rects.is_empty() {
            return;
        }
        let pad = self.pt.pad;
        let (x0, y0) = rects.iter().fold((f32::MAX, f32::MAX), |a, r| (a.0.min(r[0] - pad), a.1.min(r[1] - pad)));
        let (x1, y1) = rects.iter().fold((f32::MIN, f32::MIN), |a, r| (a.0.max(r[0] + r[2] + pad), a.1.max(r[1] + r[3] + pad)));
        let Some(area) = Area::from_points((x0, y0), (x1, y1), w, h) else { return };
        let all = mask_of(&idx.iter().filter_map(|i| self.pt.lines.get(*i)).collect::<Vec<_>>(), pad, w, h);
        let m: Vec<u8> = all.iter().map(|b| if *b { 255 } else { 0 }).collect();
        self.doc.set_selection(Some(area), Some(m));
    }

    /// Adds the selected area as a text line the OCR missed; the user types what it says.
    fn pt_add_selection(&mut self) {
        let Some(a) = self.doc.selection else { return };
        let img = &self.composite;
        let rect = [a.x0 as f32, a.y0 as f32, (a.x1 - a.x0) as f32, (a.y1 - a.y0) as f32];
        let (color, bg) = studio_ocr::colors(img, rect);
        let (mask, _, _) = lookalike::ink(img, rect, color, bg);
        let (grain, soft) = lookalike::texture(img, rect, &mask);
        let hand = FONTS.iter().position(|f| f.0 == "Segoe Print").unwrap_or(0);
        let look = Some(Look { font: hand, bold: false, italic: false, score: 0.0, grain, soft });
        self.pt.layer = self.doc.active;
        self.pt.lines.push(Line { text: String::new(), orig: String::new(), rect, color, bg, on: true, look });
        self.pt.sel = Some(self.pt.lines.len() - 1);
    }

    pub fn pt_hit(&self, x: f32, y: f32) -> Option<usize> {
        self.pt.lines.iter().position(|l| x >= l.rect[0] - 3.0 && x <= l.rect[0] + l.rect[2] + 3.0 && y >= l.rect[1] - 3.0 && y <= l.rect[1] + l.rect[3] + 3.0)
    }

    pub fn pt_options(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        if self.pt.langs.is_empty() {
            ui.label(RichText::new(tl("Aucune langue OCR installée dans Windows")).size(12.0).color(MUTED));
            return;
        }
        let cur = self.pt.langs.get(self.pt.lang).map(|l| l.1.clone()).unwrap_or_default();
        egui::ComboBox::from_id_salt("pt-lang").selected_text(cur).width(150.0).show_ui(ui, |ui| {
            for (k, l) in self.pt.langs.iter().enumerate() {
                ui.selectable_value(&mut self.pt.lang, k, &l.1);
            }
        });
        if ui.add_enabled(self.pt.job.is_none(), egui::Button::new("Détecter le texte")).clicked() {
            self.pt_detect(ctx);
        }
        ui.checkbox(&mut self.pt.selection_only, tl("Dans la sélection"));
        ui.checkbox(&mut self.pt.boxes, tl("Cadres"));
        if !self.pt.busy.is_empty() {
            ui.add(egui::Spinner::new());
            ui.label(RichText::new(&self.pt.busy).size(12.0).color(MUTED));
        } else {
            ui.label(RichText::new(format!("{} ligne(s) · clic sur un cadre pour la modifier", self.pt.lines.len())).size(12.0).color(FAINT));
        }
    }

    pub fn pt_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let lab = |ui: &mut Ui, s: &str| {
            ui.label(RichText::new(tl(&s)).size(12.0).color(MUTED));
        };
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(self.pt.job.is_none(), egui::Button::new(RichText::new(tl("Détecter le texte")).color(Color32::WHITE)).fill(studio_ui::pro::BLUE)).clicked() {
                self.pt_detect(ctx);
            }
            if !self.pt.busy.is_empty() {
                ui.add(egui::Spinner::new());
            }
        });
        if !self.pt.busy.is_empty() {
            lab(ui, &self.pt.busy);
        }
        if ui
            .add_enabled(self.doc.selection.is_some(), egui::Button::new(tl("Sélection → ligne")))
            .on_hover_text(tl("Texte non détecté (écriture difficile à lire) : sélectionne-le au rectangle, puis ajoute-le comme ligne et tape ce qui est écrit"))
            .clicked()
        {
            self.pt_add_selection();
        }
        egui::CollapsingHeader::new(RichText::new(tl("Options de lecture")).size(12.0)).id_salt("pt-read").show(ui, |ui| {
            if !self.pt.langs.is_empty() {
                let cur = self.pt.langs.get(self.pt.lang).map(|l| l.1.clone()).unwrap_or_default();
                egui::ComboBox::from_id_salt("pt-lang2").selected_text(cur).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                    for (k, l) in self.pt.langs.iter().enumerate() {
                        ui.selectable_value(&mut self.pt.lang, k, &l.1);
                    }
                });
            }
            ui.checkbox(&mut self.pt.upscale, tl("Agrandir les petites images (petits caractères)"));
            ui.checkbox(&mut self.pt.selection_only, tl("Seulement dans la sélection"));
            ui.checkbox(&mut self.pt.boxes, tl("Afficher les cadres sur l'image"));
            ui.horizontal(|ui| {
                lab(ui, tl("Ignorer les lignes de moins de"));
                ui.add(egui::DragValue::new(&mut self.pt.min_conf).range(1..=10));
                lab(ui, tl("caractère(s)"));
            });
        });
        if self.pt.lines.is_empty() {
            ui.label(RichText::new(tl("Lis le texte d'une photo (panneau, affiche, document, capture), puis modifie-le, efface-le ou floute-le en gardant sa couleur et sa taille.")).size(11.5).color(FAINT));
            return;
        }
        ui.add_space(6.0);
        lab(ui, &format!("{} ligne(s) · coche celles à traiter", self.pt.lines.len()));
        let mut act: Option<(u8, usize)> = None;
        let sel = self.pt.sel;
        egui::ScrollArea::vertical().id_salt("pt-lines").max_height(260.0).show(ui, |ui| {
            for (i, l) in self.pt.lines.iter_mut().enumerate() {
                let changed = l.text != l.orig;
                ui.horizontal(|ui| {
                    ui.checkbox(&mut l.on, "");
                    let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, egui::CornerRadius::same(3), Color32::from_rgb(l.color[0], l.color[1], l.color[2]));
                    let te = egui::TextEdit::singleline(&mut l.text).desired_width(ui.available_width() - 4.0);
                    let resp = ui.add(if changed { te.text_color(Color32::from_rgb(255, 180, 80)) } else { te });
                    if sel == Some(i) && resp.ctx.memory(|m| m.focused().is_none()) {
                        resp.request_focus();
                    }
                    if resp.gained_focus() {
                        act = Some((9, i));
                    }
                    let font = l.look.map(|k| format!("\n{} : {}{}{}", tl("Police"), FONTS[k.font].0, if k.bold { " · Bold" } else { "" }, if k.italic { " · Italic" } else { "" })).unwrap_or_default();
                    resp.on_hover_text(format!("Original : {}\nCouleur {} sur fond {}{font}", l.orig, hex(l.color), hex(l.bg)));
                });
                if sel == Some(i) {
                    ui.horizontal_wrapped(|ui| {
                        for (k, name) in [(0u8, "Remplacer"), (1, "Effacer"), (2, "Flouter"), (3, "Copier"), (4, "Sélectionner"), (5, "Rétablir")] {
                            if ui.small_button(tl(&name)).clicked() {
                                act = Some((k, i));
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tl("Ce qui est écrit")).size(11.5).color(FAINT));
                        ui.add(egui::TextEdit::singleline(&mut l.orig).desired_width(ui.available_width() - 4.0))
                            .on_hover_text(tl("Corrige la lecture si l'OCR s'est trompé : l'écriture manuscrite est recomposée lettre par lettre à partir de ce texte"));
                    });
                }
            }
        });
        if let Some((k, i)) = act {
            match k {
                0 => self.pt_apply(&[i], true, ctx),
                1 => self.pt_apply(&[i], false, ctx),
                2 => self.pt_blur(&[i]),
                3 => ctx.copy_text(self.pt.lines[i].text.clone()),
                4 => self.pt_select(&[i]),
                5 => self.pt.lines[i].text = self.pt.lines[i].orig.clone(),
                _ => self.pt.sel = Some(i),
            }
        }
        let on: Vec<usize> = (0..self.pt.lines.len()).filter(|i| self.pt.lines[*i].on).collect();
        let edited: Vec<usize> = on.iter().copied().filter(|i| self.pt.lines[*i].text != self.pt.lines[*i].orig).collect();
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(!edited.is_empty() && self.pt.job.is_none(), egui::Button::new(RichText::new(format!("Appliquer ({})", edited.len())).color(Color32::WHITE)).fill(studio_ui::pro::BLUE)).on_hover_text(tl("Efface le texte d'origine des lignes modifiées et écrit le nouveau à la même place")).clicked() {
                self.pt_apply(&edited, true, ctx);
            }
            if ui.add_enabled(!on.is_empty(), egui::Button::new("Tout effacer")).on_hover_text(tl("Supprime le texte des lignes cochées")).clicked() {
                self.pt_apply(&on, false, ctx);
            }
            if ui.add_enabled(!on.is_empty(), egui::Button::new("Flouter")).on_hover_text(tl("Mosaïque sur les lignes cochées (plaques, adresses, noms…)")).clicked() {
                self.pt_blur(&on);
            }
            if ui.button(tl("Sélectionner")).on_hover_text(tl("Crée une sélection autour des lignes cochées")).clicked() {
                self.pt_select(&on);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button(tl("Tout cocher")).clicked() {
                self.pt.lines.iter_mut().for_each(|l| l.on = true);
            }
            if ui.button(tl("Rien")).clicked() {
                self.pt.lines.iter_mut().for_each(|l| l.on = false);
            }
            if ui.button(tl("Copier le texte")).clicked() {
                ctx.copy_text(self.pt.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"));
                studio_ui::handoff::toast(ctx, "Texte copié".into(), true);
            }
            if ui.button(tl("Enregistrer .txt…")).clicked()
                && let Some(p) = rfd::FileDialog::new().add_filter("Texte", &["txt"]).set_file_name("texte de la photo.txt").save_file()
            {
                let _ = std::fs::write(p, self.pt.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"));
            }
            if ui.button(tl("Oublier")).clicked() {
                self.pt.lines.clear();
                self.pt.sel = None;
            }
        });
        egui::CollapsingHeader::new(RichText::new(tl("Rechercher et remplacer")).size(12.0)).id_salt("pt-find").show(ui, |ui| {
            ui.add(egui::TextEdit::singleline(&mut self.pt.find).hint_text(tl("Rechercher")).desired_width(f32::INFINITY));
            ui.add(egui::TextEdit::singleline(&mut self.pt.repl).hint_text(tl("Remplacer par")).desired_width(f32::INFINITY));
            ui.checkbox(&mut self.pt.case, tl("Respecter la casse"));
            if ui.add_enabled(!self.pt.find.is_empty(), egui::Button::new("Remplacer partout")).clicked() {
                let (f, r, case) = (self.pt.find.clone(), self.pt.repl.clone(), self.pt.case);
                let mut n = 0;
                for l in self.pt.lines.iter_mut().filter(|l| l.on) {
                    let new = if case { l.text.replace(&f, &r) } else { replace_ci(&l.text, &f, &r) };
                    if new != l.text {
                        l.text = new;
                        n += 1;
                    }
                }
                studio_ui::handoff::toast(ctx, format!("{n} ligne(s) modifiée(s) : « Appliquer » pour les écrire dans l'image"), n > 0);
            }
        });
        egui::CollapsingHeader::new(RichText::new(tl("Rendu du nouveau texte")).size(12.0)).id_salt("pt-style").default_open(true).show(ui, |ui| {
            ui.checkbox(&mut self.pt.auto_font, tl("Police détectée sur chaque ligne"));
            lab(ui, tl("Écriture manuscrite"));
            pro::segmented(ui, &mut self.pt.hand, &HAND.map(tl));
            ui.label(RichText::new(tl("Seuls les mots modifiés sont effacés et réécrits. Lettres d'origine : recompose les mots avec les lettres découpées dans la photo (écriture détachée). Police manuscrite : la police la plus proche, à la taille et dans l'encre de l'original (écriture liée). Auto choisit selon l'écriture.")).size(11.0).color(FAINT));
            ui.checkbox(&mut self.pt.blend, tl("Imiter le grain et la netteté de la photo"));
            ui.add_enabled_ui(!self.pt.auto_font, |ui| egui::ComboBox::from_id_salt("pt-font").selected_text(FONTS[self.pt.font].0).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                for (k, f) in FONTS.iter().enumerate() {
                    ui.selectable_value(&mut self.pt.font, k, f.0);
                }
            }));
            ui.horizontal(|ui| {
                ui.toggle_value(&mut self.pt.bold, RichText::new(tl("G")).strong()).on_hover_text(tl("Gras"));
                ui.toggle_value(&mut self.pt.italic, RichText::new(tl("I")).italics()).on_hover_text(tl("Italique"));
                ui.checkbox(&mut self.pt.auto_color, tl("Couleur d'origine"));
            });
            if !self.pt.auto_color {
                let mut c = Color32::from_rgba_unmultiplied(self.pt.color[0], self.pt.color[1], self.pt.color[2], 255);
                if ui.color_edit_button_srgba(&mut c).changed() {
                    self.pt.color = c.to_array();
                }
            }
            pro::option_slider(ui, tl("Taille"), &mut self.pt.size_k, 0.5..=1.6, "×", false);
            ui.checkbox(&mut self.pt.fit, tl("Tenir dans la largeur d'origine"));
            ui.checkbox(&mut self.pt.own_layer, tl("Un calque par ligne (déplaçable)"));
        });
        egui::CollapsingHeader::new(RichText::new(tl("Effacement")).size(12.0)).id_salt("pt-erase").show(ui, |ui| {
            pro::segmented(ui, &mut self.pt.method, &METHODS);
            pro::option_slider(ui, tl("Marge"), &mut self.pt.pad, 0.0..=20.0, " px", false);
            pro::option_slider(ui, tl("Mosaïque"), &mut self.pt.blur_px, 0.0..=40.0, " px", false);
            ui.label(RichText::new(tl("Auto : fond uni (papier, mur) recalculé sans recopier d'encre, sinon reconstruction. Reconstruction : recrée le fond (photos). Couleur du fond : aplat rapide (documents, captures). Mosaïque 0 = taille automatique.")).size(11.0).color(FAINT));
        });
        let _ = (INK, semibold(12.0));
    }
}

fn replace_ci(s: &str, f: &str, r: &str) -> String {
    let low = s.to_lowercase();
    let fl = f.to_lowercase();
    if fl.is_empty() || low.len() != s.len() {
        return s.replace(f, r);
    }
    let mut out = String::new();
    let mut i = 0;
    while let Some(p) = low[i..].find(&fl) {
        out.push_str(&s[i..i + p]);
        out.push_str(r);
        i += p + fl.len();
    }
    out.push_str(&s[i..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_text_and_replace() {
        let l = Line { text: "x".into(), orig: "x".into(), rect: [2.0, 2.0, 4.0, 3.0], color: [0; 4], bg: [255; 4], on: true, look: None };
        let m = mask_of(&[&l], 1.0, 10, 10);
        assert_eq!(m.iter().filter(|b| **b).count(), 6 * 5);
        assert_eq!(replace_ci("Prix PROMO promo", "promo", "SOLDES"), "Prix SOLDES SOLDES");
        let mut img = RgbaImage::from_pixel(20, 10, Rgba([200, 200, 200, 255]));
        img.put_pixel(5, 5, Rgba([0, 0, 0, 255]));
        let big = Line { rect: [0.0, 0.0, 20.0, 10.0], ..l };
        blur_rects(&mut img, &[&big], 0.0, 10.0);
        assert!(img.get_pixel(5, 5).0[0] > 150);
        let st = TextStyle { font: 1, size: 20.0, bold: false, italic: false, underline: false, align: 0, tracking: 0.0, leading: 1.2, vertical: false, color: [0, 0, 0, 255] };
        if let Some(t) = line_image("Bonjour", [0.0, 0.0, 60.0, 18.0], &st, true, 1.0) {
            assert_eq!(t.height(), 18);
            assert!(t.width() <= 62);
        }
    }
}

#[cfg(all(test, windows))]
mod e2e {
    use super::*;

    #[test]
    #[ignore]
    fn read_and_replace_on_sign() {
        let out = std::env::var("PT_OUT").unwrap_or_default();
        let mut img = RgbaImage::from_fn(900, 420, |x, y| Rgba([(40 + x / 8) as u8, (90 + y / 6) as u8, 170, 255]));
        let sign = TextStyle { font: 1, size: 96.0, bold: true, italic: false, underline: false, align: 0, tracking: 0.0, leading: 1.2, vertical: false, color: [255, 220, 40, 255] };
        let t = render_text("SOLDES -50%", &sign).unwrap();
        imageops::overlay(&mut img, &t, 90, 120);
        let p = studio_ocr::recognize_with(&img, "fr-FR", true).unwrap();
        let l = p.lines.iter().find(|l| l.text.contains("SOLDES")).expect(&p.text);
        let (color, _) = studio_ocr::colors(&img, l.rect);
        assert!(color[0] > 180 && color[2] < 140, "{color:?}");
        let refs = Line { text: "PROMO -70%".into(), orig: l.text.clone(), rect: l.rect, color, bg: [0; 4], on: true, look: None };
        let mask = mask_of(&[&refs], 4.0, img.width(), img.height());
        let mut erased = crate::inpaint::content_aware_fill(&img, &mask, 3);
        let st = TextStyle { color, ..sign };
        let new = line_image("PROMO -70%", l.rect, &st, true, 1.0).unwrap();
        imageops::overlay(&mut erased, &new, l.rect[0] as i64, l.rect[1] as i64);
        if !out.is_empty() {
            img.save(format!("{out}/pt-before.png")).unwrap();
            erased.save(format!("{out}/pt-after.png")).unwrap();
        }
        let back = studio_ocr::recognize_with(&erased, "fr-FR", true).unwrap();
        assert!(back.text.contains("PROMO") && !back.text.contains("SOLDES"), "{}", back.text);
    }


    /// A note in `font` on paper, rewritten from "Rendez-vous lundi" to `new` the way the panel does it.
    /// `ocr` reads the line like "Détecter le texte"; otherwise the line is added by hand like "Sélection → ligne".
    fn rewrite_note(font: &str, ocr: bool, new: &str) -> (RgbaImage, RgbaImage, Look) {
        let paper = |x: u32, y: u32| {
            let n = ((x.wrapping_mul(7919) ^ y.wrapping_mul(104_729)) % 13) as u8;
            Rgba([236 - n, 228 - n, 210 - n, 255])
        };
        let mut img = RgbaImage::from_fn(900, 240, paper);
        let fi = FONTS.iter().position(|f| f.0 == font).unwrap();
        let pen = TextStyle { font: fi, size: 84.0, bold: false, italic: false, underline: false, align: 0, tracking: 0.0, leading: 1.2, vertical: false, color: [25, 40, 110, 255] };
        let t = render_text("Rendez-vous lundi", &pen).unwrap();
        imageops::overlay(&mut img, &t, 60, 60);
        let (rect, orig) = if ocr {
            let p = studio_ocr::recognize_with(&img, "fr-FR", true).unwrap();
            let l = p.lines.iter().find(|l| l.text.contains("lundi")).unwrap_or_else(|| panic!("OCR read: {:?}", p.text));
            (l.rect, l.text.clone())
        } else {
            let mut b = (u32::MAX, u32::MAX, 0, 0);
            for (x, y, p) in t.enumerate_pixels().filter(|q| q.2.0[3] > 96) {
                b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
                let _ = p;
            }
            ([60.0 + b.0 as f32 - 4.0, 60.0 + b.1 as f32 - 4.0, (b.2 - b.0) as f32 + 8.0, (b.3 - b.1) as f32 + 8.0], "Rendez-vous lundi".to_string())
        };
        let (color, bg) = studio_ocr::colors(&img, rect);
        let (mask, x0, y0) = lookalike::ink(&img, rect, color, bg);
        let (f, bold, italic, score) = lookalike::match_font(&mask, &orig).unwrap();
        let (grain, soft) = lookalike::texture(&img, rect, &mask);
        let look = Look { font: f, bold, italic, score, grain, soft };
        let hand = Hand::from_line(&img, &mask, x0, y0, &orig, (f, bold, italic));
        let mut drawn = hand.write(new, (f, bold, italic), color).unwrap();
        lookalike::blend(&mut drawn, &look, color);
        let line = Line { text: String::new(), orig, rect, color, bg, on: true, look: None };
        let mut after = crate::inpaint::content_aware_fill(&img, &mask_of(&[&line], 4.0, img.width(), img.height()), 3);
        imageops::overlay(&mut after, &drawn, x0 as i64, y0 as i64);
        if let Ok(out) = std::env::var("PT_OUT") {
            let tag = font.replace(' ', "-").to_lowercase();
            img.save(format!("{out}/hand-{tag}-before.png")).unwrap();
            after.save(format!("{out}/hand-{tag}-after.png")).unwrap();
        }
        (img, after, look)
    }

    #[test]
    #[ignore]
    fn rewrites_a_handwritten_note() {
        let (_, after, look) = rewrite_note("Segoe Print", true, "Rendez-vous mardi");
        assert!(lookalike::is_handwriting(look.font), "matched {}", FONTS[look.font].0);
        let back = studio_ocr::recognize_with(&after, "fr-FR", true).unwrap();
        assert!(back.text.contains("mardi") && !back.text.contains("lundi"), "read back: {}", back.text);
    }

    #[test]
    #[ignore]
    fn rewrites_handwriting_the_ocr_cannot_read() {
        let (before, after, look) = rewrite_note("Ink Free", false, "Rendez-vous dimanche");
        assert_eq!(FONTS[look.font].0, "Ink Free");
        let ink = |img: &RgbaImage| img.pixels().filter(|p| p.0[2] as u16 > p.0[0] as u16 + 40).count();
        assert!(ink(&after) > ink(&before) / 2, "new ink {} vs {}", ink(&after), ink(&before));
    }
}

#[cfg(all(test, windows))]
mod real_photo {
    use super::*;

    /// Runs the panel's pipeline on a real photo. PT_PHOTO=<image> PT_OUT=<folder> PT_NEW=<new text>, and either
    /// PT_LINE=<word the OCR reads in the line> or PT_RECT=x,y,w,h with PT_ORIG=<what it says> (like "Sélection → ligne").
    #[test]
    #[ignore]
    fn rewrite_real_photo() {
        let Ok(photo) = std::env::var("PT_PHOTO") else { return };
        let img = image::open(photo).unwrap().to_rgba8();
        let var = |k: &str| std::env::var(k).ok();
        let (rect, orig) = if let Some(r) = var("PT_RECT") {
            let v: Vec<f32> = r.split(',').map(|x| x.trim().parse().unwrap()).collect();
            ([v[0], v[1], v[2], v[3]], var("PT_ORIG").expect("PT_ORIG"))
        } else {
            let p = studio_ocr::recognize_with(&img, &var("PT_LANG").unwrap_or_else(|| "en-US".into()), true).unwrap();
            for l in &p.lines {
                eprintln!("LINE {:?} rect {:?}", l.text, l.rect.map(|v| v.round()));
            }
            let Some(word) = var("PT_LINE") else { return };
            let l = p.lines.iter().find(|l| l.text.contains(&word)).expect("line");
            (l.rect, var("PT_ORIG").unwrap_or_else(|| l.text.clone()))
        };
        let (Some(new), Some(out)) = (var("PT_NEW"), var("PT_OUT")) else { return };
        let (color, bg) = studio_ocr::colors(&img, rect);
        let (mask, x0, y0) = lookalike::ink(&img, rect, color, bg);
        let (font, bold, italic, score) = lookalike::match_font(&mask, &orig).unwrap();
        let (grain, soft) = lookalike::texture(&img, rect, &mask);
        eprintln!("MATCH {} bold={bold} italic={italic} score={score:.2} grain={grain:.1} soft={soft:.2} color={color:?} bg={bg:?}", FONTS[font].0);
        let mut hand = Hand::from_line(&img, &mask, x0, y0, &orig, (font, bold, italic));
        if std::env::var("PT_FONTONLY").is_ok() {
            hand.own_letters = false;
        }
        if std::env::var("PT_LETTERS").is_ok() {
            hand.own_letters = true;
        }
        eprintln!("OWN LETTERS {}", hand.own_letters);
        eprintln!("LETTERS {}", hand.letters());
        let (mut drawn, start, end) = hand.rewrite(&orig, &new, (font, bold, italic), color).unwrap();
        lookalike::blend(&mut drawn, &Look { font, bold, italic, score, grain, soft }, color);
        let mut after = img.clone();
        let p = 3.0;
        lookalike::paper_fill(&mut after, [(x0 + start) as f32 - 2.0 - p, rect[1] - rect[3] * 0.2 - p, (end - start) as f32 + 4.0 + 2.0 * p, rect[3] * 1.4 + 2.0 * p], color, bg, grain);
        imageops::overlay(&mut after, &drawn, (x0 + start) as i64, y0 as i64);
        let pad = 80.0;
        let (cx, cy) = ((rect[0] - pad).max(0.0) as u32, (rect[1] - pad).max(0.0) as u32);
        let (cw, ch) = (((rect[2] + 2.0 * pad) as u32).min(img.width() - cx), ((rect[3] + 2.0 * pad) as u32).min(img.height() - cy));
        imageops::crop_imm(&img, cx, cy, cw, ch).to_image().save(format!("{out}/real-before.png")).unwrap();
        imageops::crop_imm(&after, cx, cy, cw, ch).to_image().save(format!("{out}/real-after.png")).unwrap();
        mask.save(format!("{out}/real-mask.png")).unwrap();
    }
}
