use super::*;
use serde_json::{Value, json};

pub const FORMATS: [(&str, &str); 8] = [
    ("MP4", "Vidéo H.264, lisible partout"),
    ("GIF", "Animation légère pour le web et les messages"),
    ("PNG séquence", "Une image par frame, idéal pour le montage"),
    ("PNG image", "L'image affichée à la tête de lecture"),
    ("SVG", "Vectoriel, l'image affichée, modifiable"),
    ("SVG séquence", "Un fichier vectoriel par image"),
    ("Planche", "Grille d'images pour présenter l'animation"),
    ("Lottie JSON", "Animation vectorielle pour le web et les apps"),
];
const SCALES: [(&str, f32); 5] = [("25 %", 0.25), ("50 %", 0.5), ("75 %", 0.75), ("100 %", 1.0), ("200 %", 2.0)];
const GIF_FPS: [(&str, u32); 4] = [("Compo", 0), ("24 i/s", 24), ("15 i/s", 15), ("10 i/s", 10)];
const MP4_Q: [(&str, f32); 4] = [("Légère", 0.05), ("Moyenne", 0.09), ("Haute", 0.14), ("Maximale", 0.25)];
const GIF_LOOP: [&str; 4] = ["Infinie", "Une fois", "3 fois", "5 fois"];
const GIF_SPEED: [(&str, i32); 3] = [("Meilleure", 1), ("Bonne", 10), ("Rapide", 30)];

#[derive(Clone)]
pub struct ExportOpts {
    pub format: usize,
    pub scale: usize,
    pub range: usize,
    pub from_f: u32,
    pub to_f: u32,
    pub transparent: bool,
    pub gif_fps: usize,
    pub quality: usize,
    pub gif_loop: usize,
    pub gif_speed: usize,
    pub prefix: String,
    pub start_no: u32,
    pub step: u32,
    pub cols: u32,
    pub rows: u32,
    pub at: f32,
}

pub struct QItem {
    pub o: ExportOpts,
    pub path: PathBuf,
}

fn frames_of(comp: &Comp, o: &ExportOpts) -> Vec<u32> {
    let fps = comp.fps as f32;
    let (first, last) = match o.range {
        1 => {
            let (a, b) = comp.range();
            ((a * fps).round() as u32, (b * fps).round() as u32)
        }
        2 => (o.from_f.min(o.to_f), o.to_f.max(o.from_f) + 1),
        _ => (0, comp.frames()),
    };
    let last = last.min(comp.frames()).max(first + 1);
    let step = match o.format {
        1 => {
            let target = GIF_FPS[o.gif_fps.min(GIF_FPS.len() - 1)].1;
            if target == 0 || target >= comp.fps { 1 } else { (fps / target as f32).round().max(1.0) as u32 }
        }
        2 | 5 => o.step.max(1),
        _ => 1,
    };
    (first..last).step_by(step as usize).collect()
}

fn work_of(comp: &Comp, o: &ExportOpts) -> usize {
    match o.format {
        3 | 4 | 7 => 1,
        6 => (o.cols * o.rows).max(1) as usize,
        _ => frames_of(comp, o).len(),
    }
}

fn export_one(comp: &Comp, db: &Arc<usvg::fontdb::Database>, o: &ExportOpts, path: &Path, done: &AtomicUsize) -> Result<(), String> {
    let scale = SCALES[o.scale.min(SCALES.len() - 1)].1;
    let fps = comp.fps;
    let frames = frames_of(comp, o);
    match o.format {
        0 => {
            let (w, h) = out_size(comp, scale, true);
            let bitrate = ((w * h) as f32 * fps as f32 * MP4_Q[o.quality.min(3)].1).max(1_500_000.0) as u32;
            let mut enc = media::Encoder::new(path, w, h, fps, bitrate)?;
            let mut written = 0u64;
            for (n, f) in frames.iter().enumerate() {
                let pix = render(comp, *f as f32 / fps as f32, w, h, db, false)?;
                enc.video(pix.data())?;
                let need = ((n as u64 + 1) * media::OUT_RATE as u64 / fps as u64).saturating_sub(written);
                enc.audio(&vec![0.0; need as usize * 2])?;
                written += need;
                done.fetch_add(1, Ordering::Relaxed);
            }
            enc.finish()
        }
        1 => {
            let (w, h) = out_size(comp, scale, false);
            let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
            let mut enc = image::codecs::gif::GifEncoder::new_with_speed(std::io::BufWriter::new(file), GIF_SPEED[o.gif_speed.min(2)].1);
            match o.gif_loop {
                0 => enc.set_repeat(image::codecs::gif::Repeat::Infinite).map_err(|e| e.to_string())?,
                1 => {}
                k => enc.set_repeat(image::codecs::gif::Repeat::Finite(if k == 2 { 2 } else { 4 })).map_err(|e| e.to_string())?,
            }
            let step = frames.get(1).map_or(1, |f| f - frames[0]);
            for f in &frames {
                let pix = render(comp, *f as f32 / fps as f32, w, h, db, o.transparent)?;
                enc.encode_frame(image::Frame::from_parts(unpremultiply(&pix), 0, 0, image::Delay::from_numer_denom_ms(1000 * step, fps))).map_err(|e| e.to_string())?;
                done.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        }
        2 | 5 => {
            std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
            let (w, h) = out_size(comp, scale, false);
            for (n, f) in frames.iter().enumerate() {
                let t = *f as f32 / fps as f32;
                let no = o.start_no + n as u32;
                if o.format == 2 {
                    render(comp, t, w, h, db, o.transparent)?.save_png(path.join(format!("{}{no:05}.png", o.prefix))).map_err(|e| e.to_string())?;
                } else {
                    std::fs::write(path.join(format!("{}{no:05}.svg", o.prefix)), comp.to_svg_with(t, o.transparent)).map_err(|e| e.to_string())?;
                }
                done.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        }
        3 => {
            let (w, h) = out_size(comp, scale, false);
            render(comp, o.at, w, h, db, o.transparent)?.save_png(path).map_err(|e| e.to_string())?;
            done.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        4 => {
            std::fs::write(path, comp.to_svg_with(o.at, o.transparent)).map_err(|e| e.to_string())?;
            done.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        6 => {
            let (cols, rows) = (o.cols.max(1), o.rows.max(1));
            let n = cols * rows;
            let (w, h) = out_size(comp, scale, false);
            let gap = 12u32;
            let mut sheet = image::RgbaImage::from_pixel(cols * w + (cols + 1) * gap, rows * h + (rows + 1) * gap, image::Rgba([24, 24, 28, 255]));
            let first = *frames.first().unwrap_or(&0);
            let last = *frames.last().unwrap_or(&0);
            for k in 0..n {
                let f = if n > 1 { first + (last - first) * k / (n - 1) } else { first };
                let img = unpremultiply(&render(comp, f as f32 / fps as f32, w, h, db, o.transparent)?);
                let (x, y) = (gap + (k % cols) * (w + gap), gap + (k / cols) * (h + gap));
                image::imageops::overlay(&mut sheet, &img, x as i64, y as i64);
                done.fetch_add(1, Ordering::Relaxed);
            }
            sheet.save(path).map_err(|e| e.to_string())
        }
        _ => {
            let v = lottie(comp);
            std::fs::write(path, serde_json::to_vec(&v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            done.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }
}

fn anim(props: &[&Prop], f: impl Fn(&[f32]) -> Value, fps: f32) -> Value {
    let mut times: Vec<f32> = props.iter().flat_map(|p| p.keys.iter().map(|k| k.t)).collect();
    times.sort_by(f32::total_cmp);
    times.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
    if times.is_empty() {
        let v: Vec<f32> = props.iter().map(|p| p.base).collect();
        return json!({ "a": 0, "k": f(&v) });
    }
    let n = times.len();
    let keys: Vec<Value> = times
        .iter()
        .enumerate()
        .map(|(j, t)| {
            let v: Vec<f32> = props.iter().map(|p| p.at(*t)).collect();
            let key = props.iter().find_map(|p| p.key_at(*t).map(|i| p.keys[i]));
            let b = key.map_or([0.0, 0.0, 1.0, 1.0], |k| match k.bez {
                Some(b) => b,
                None if k.ease == Ease::Linear && k.ease2 == Ease2::None => [0.0, 0.0, 1.0, 1.0],
                None => model::ease_to_bez(k.ease),
            });
            let hold = key.is_some_and(|k| k.ease == Ease::Hold && k.ease2 == Ease2::None && k.bez.is_none());
            let mut o = json!({ "t": t * fps, "s": f(&v) });
            if j + 1 < n {
                o["o"] = json!({ "x": [b[0]], "y": [b[1]] });
                o["i"] = json!({ "x": [b[2]], "y": [b[3]] });
                if hold {
                    o["h"] = json!(1);
                }
            }
            o
        })
        .collect();
    json!({ "a": 1, "k": keys })
}

fn rgba(c: [u8; 4]) -> Value {
    json!([c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0])
}

fn path_shape(pts: &[[f32; 2]], ins: &[[f32; 2]], outs: &[[f32; 2]], closed: bool) -> Value {
    json!({ "ty": "sh", "ks": { "a": 0, "k": { "v": pts, "i": ins, "o": outs, "c": closed } } })
}

fn poly_shape(pts: &[[f32; 2]], closed: bool) -> Value {
    let z = vec![[0.0f32; 2]; pts.len()];
    path_shape(pts, &z, &z, closed)
}

pub fn lottie(comp: &Comp) -> Value {
    let fps = comp.fps as f32;
    let mut layers = Vec::new();
    for (li, l) in comp.layers.iter().enumerate().rev() {
        let p = &l.props;
        let one = |i: usize| anim(&[&p[i]], |v| json!(v[0]), fps);
        let ks = json!({
            "o": one(4),
            "r": one(3),
            "p": anim(&[&p[0], &p[1]], |v| json!([v[0], v[1], 0.0]), fps),
            "a": anim(&[&p[8], &p[9]], |v| json!([v[0], v[1], 0.0]), fps),
            "s": anim(&[&p[2], &p[10], &p[11]], |v| json!([v[0] * v[1] / 100.0, v[0] * v[2] / 100.0, 100.0]), fps),
        });
        let mut items = Vec::new();
        let mut fill = l.content.color();
        let mut line: Option<([u8; 4], f32)> = None;
        match &l.content {
            Content::Rect { w, h, r, .. } => items.push(json!({ "ty": "rc", "p": { "a": 0, "k": [0, 0] }, "s": { "a": 0, "k": [w, h] }, "r": { "a": 0, "k": r } })),
            Content::Ellipse { w, h, .. } => items.push(json!({ "ty": "el", "p": { "a": 0, "k": [0, 0] }, "s": { "a": 0, "k": [w, h] } })),
            Content::Star { points, radius, inner, .. } => {
                let star = *inner < 0.99;
                items.push(json!({ "ty": "sr", "sy": if star { 1 } else { 2 }, "d": 1, "pt": { "a": 0, "k": points }, "p": { "a": 0, "k": [0, 0] }, "r": { "a": 0, "k": 0 }, "or": { "a": 0, "k": radius }, "os": { "a": 0, "k": 0 }, "ir": { "a": 0, "k": radius * inner }, "is": { "a": 0, "k": 0 } }));
            }
            Content::Arrow { w, thickness, .. } => items.push(poly_shape(&model::arrow_points(*w, *thickness), true)),
            Content::Line { w, thickness, color } => {
                items.push(poly_shape(&[[-w / 2.0 + thickness / 2.0, 0.0], [w / 2.0 - thickness / 2.0, 0.0]], false));
                line = Some((*color, *thickness));
                fill = None;
            }
            Content::Ring { radius, thickness, color } => {
                items.push(json!({ "ty": "el", "p": { "a": 0, "k": [0, 0] }, "s": { "a": 0, "k": [radius * 2.0, radius * 2.0] } }));
                line = Some((*color, *thickness));
                fill = None;
            }
            Content::Path { pts, closed, fill: f, .. } => {
                let v: Vec<[f32; 2]> = pts.iter().map(|q| q.p).collect();
                let ins: Vec<[f32; 2]> = pts.iter().map(|q| q.i).collect();
                let outs: Vec<[f32; 2]> = pts.iter().map(|q| q.o).collect();
                items.push(path_shape(&v, &ins, &outs, *closed));
                if !*f {
                    fill = None;
                }
            }
            Content::Strokes { strokes, width, color } => {
                for s in strokes {
                    items.push(poly_shape(s, false));
                }
                line = Some((*color, *width));
                fill = None;
            }
            Content::Null => {}
            _ => continue,
        }
        let trim = p[14].animated() || p[15].animated() || p[14].base > 0.0 || p[15].base < 100.0;
        if trim {
            items.push(json!({ "ty": "tm", "s": one(14), "e": one(15), "o": anim(&[&p[16]], |v| json!(v[0] * 3.6), fps), "m": 1 }));
        }
        if let Some(c) = fill {
            items.push(json!({ "ty": "fl", "c": { "a": 0, "k": rgba(c) }, "o": { "a": 0, "k": c[3] as f32 / 2.55 }, "r": 1 }));
        }
        let stroke = line.or_else(|| l.fx.stroke.map(|c| (c, l.value(17, 0.0))).filter(|s| s.1 > 0.0));
        if let Some((c, w)) = stroke {
            items.push(json!({ "ty": "st", "c": { "a": 0, "k": rgba(c) }, "o": { "a": 0, "k": c[3] as f32 / 2.55 }, "w": { "a": 0, "k": w }, "lc": 2, "lj": 2 }));
        }
        items.push(json!({ "ty": "tr", "p": { "a": 0, "k": [0, 0] }, "a": { "a": 0, "k": [0, 0] }, "s": { "a": 0, "k": [100, 100] }, "r": { "a": 0, "k": 0 }, "o": { "a": 0, "k": 100 } }));
        let mut layer = json!({
            "ddd": 0, "ind": li + 1, "nm": l.name, "sr": 1, "ks": ks, "ao": if l.fx.auto_orient { 1 } else { 0 },
            "ip": l.start * fps, "op": l.end * fps, "st": 0, "bm": 0,
        });
        if matches!(l.content, Content::Null) {
            layer["ty"] = json!(3);
        } else {
            layer["ty"] = json!(4);
            layer["shapes"] = json!([{ "ty": "gr", "it": items }]);
        }
        if !l.visible {
            layer["hd"] = json!(true);
        }
        if let Some(pp) = l.parent {
            layer["parent"] = json!(pp + 1);
        }
        layers.push(layer);
    }
    json!({ "v": "5.7.0", "fr": comp.fps, "ip": 0, "op": comp.frames(), "w": comp.width, "h": comp.height, "nm": "Motion", "ddd": 0, "assets": [], "layers": layers })
}

impl Motion {
    pub(crate) fn default_export(&self) -> ExportOpts {
        ExportOpts {
            format: 0,
            scale: 3,
            range: if self.comp.work.is_some() { 1 } else { 0 },
            from_f: 0,
            to_f: self.comp.frames().saturating_sub(1),
            transparent: false,
            gif_fps: 2,
            quality: 2,
            gif_loop: 0,
            gif_speed: 1,
            prefix: "image_".into(),
            start_no: 0,
            step: 1,
            cols: 4,
            rows: 3,
            at: self.t,
        }
    }

    fn pick_path(&self, o: &ExportOpts) -> Option<PathBuf> {
        let (filter, ext, name): (&str, &str, &str) = match o.format {
            0 => ("Vidéo MP4", "mp4", "animation.mp4"),
            1 => ("GIF", "gif", "animation.gif"),
            3 | 6 => ("PNG", "png", if o.format == 6 { "planche.png" } else { "image.png" }),
            4 => ("SVG", "svg", "image.svg"),
            7 => ("Lottie JSON", "json", "animation.json"),
            _ => return rfd::FileDialog::new().set_title("Dossier de la séquence").pick_folder(),
        };
        rfd::FileDialog::new().add_filter(filter, &[ext]).set_file_name(name).save_file()
    }

    fn spawn(&mut self, items: Vec<(ExportOpts, PathBuf)>) {
        if items.is_empty() || self.busy() {
            return;
        }
        let comp = self.comp.clone();
        let db = self.fontdb.clone();
        let total = items.iter().map(|(o, _)| work_of(&comp, o)).sum();
        let label = if items.len() == 1 { FORMATS[items[0].0.format].0.to_string() } else { format!("File ({})", items.len()) };
        let done = Arc::new(AtomicUsize::new(0));
        let error = Arc::new(Mutex::new(None));
        let (d2, e2) = (done.clone(), error.clone());
        std::thread::spawn(move || {
            let mut err = None;
            for (o, path) in &items {
                if let Err(e) = export_one(&comp, &db, o, path, &d2) {
                    err = Some(format!("{} : {e}", path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()));
                    break;
                }
            }
            if let Some(e) = err
                && let Ok(mut slot) = e2.lock()
            {
                *slot = Some(e);
            }
            d2.store(usize::MAX, Ordering::Relaxed);
        });
        self.job = Some(Job { done, total, error, label });
    }

    pub(crate) fn run_export(&mut self, o: &ExportOpts) {
        let mut o = o.clone();
        o.at = self.t;
        if let Some(p) = self.pick_path(&o) {
            self.spawn(vec![(o, p)]);
        }
    }

    pub(crate) fn export_sheet(&mut self, ctx: &egui::Context) {
        let Some(mut o) = self.export.take() else { return };
        let mut go = false;
        let mut cancel = false;
        let mut enqueue = false;
        let keep = pro::sheet(ctx, "export", 520.0, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(9), BLUE.gamma_multiply(0.25));
                glyph(ui.painter(), r.shrink(9.0), G::Export, BLUE);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Exporter l'animation").font(semibold(18.0)).color(INK_STRONG));
                    ui.label(RichText::new(FORMATS[o.format].1).size(12.0).color(MUTED));
                });
            });
            ui.add_space(14.0);
            let row = |ui: &mut Ui, s: &str| {
                ui.label(RichText::new(s).size(12.5).color(MUTED));
            };
            row(ui, "Format");
            let w = (ui.available_width() - 18.0) / 4.0;
            for chunk in FORMATS.chunks(4).enumerate() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (k, (name, _)) in chunk.1.iter().enumerate() {
                        let idx = chunk.0 * 4 + k;
                        let on = o.format == idx;
                        let (r, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
                        ui.painter().rect(r, CornerRadius::same(7), if on { BLUE.gamma_multiply(0.3) } else if resp.hovered() { FIELD } else { pro::WELL }, Stroke::new(1.0, if on { BLUE } else { LINE }), StrokeKind::Inside);
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, *name, if on { semibold(12.0) } else { FontId::proportional(12.0) }, if on { INK_STRONG } else { INK });
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            o.format = idx;
                        }
                    }
                });
            }
            ui.add_space(10.0);
            if !matches!(o.format, 4 | 5 | 7) {
                row(ui, "Taille");
                let labels: Vec<&str> = SCALES.iter().map(|s| s.0).collect();
                pro::segmented(ui, &mut o.scale, &labels);
                let (w, h) = out_size(&self.comp, SCALES[o.scale].1, o.format == 0);
                ui.label(RichText::new(format!("{w} × {h} px")).size(11.5).color(FAINT));
                ui.add_space(8.0);
            }
            if matches!(o.format, 0 | 1 | 2 | 5 | 6) {
                row(ui, "Plage");
                let has_work = self.comp.work.is_some();
                pro::segmented(ui, &mut o.range, &["Toute la composition", "Zone de travail", "Personnalisée"]);
                if o.range == 1 && !has_work {
                    o.range = 0;
                }
                if o.range == 2 {
                    let max = self.comp.frames().saturating_sub(1);
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut o.from_f).range(0..=max).prefix("De l'image "));
                        ui.add(egui::DragValue::new(&mut o.to_f).range(0..=max).prefix("à "));
                        if ui.small_button("Tête → début").clicked() {
                            o.from_f = (self.t * self.comp.fps as f32).round() as u32;
                        }
                        if ui.small_button("Tête → fin").clicked() {
                            o.to_f = (self.t * self.comp.fps as f32).round() as u32;
                        }
                    });
                }
                ui.add_space(8.0);
            }
            match o.format {
                0 => {
                    row(ui, "Qualité (débit)");
                    let labels: Vec<&str> = MP4_Q.iter().map(|s| s.0).collect();
                    pro::segmented(ui, &mut o.quality, &labels);
                    let (w, h) = out_size(&self.comp, SCALES[o.scale].1, true);
                    let br = ((w * h) as f32 * self.comp.fps as f32 * MP4_Q[o.quality].1).max(1_500_000.0) / 1e6;
                    ui.label(RichText::new(format!("≈ {br:.1} Mb/s · audio muet AAC")).size(11.5).color(FAINT));
                }
                1 => {
                    row(ui, "Images par seconde");
                    let labels: Vec<&str> = GIF_FPS.iter().map(|s| s.0).collect();
                    pro::segmented(ui, &mut o.gif_fps, &labels);
                    ui.add_space(6.0);
                    row(ui, "Lecture en boucle");
                    pro::segmented(ui, &mut o.gif_loop, &GIF_LOOP);
                    ui.add_space(6.0);
                    row(ui, "Palette");
                    let labels: Vec<&str> = GIF_SPEED.iter().map(|s| s.0).collect();
                    pro::segmented(ui, &mut o.gif_speed, &labels);
                }
                2 | 5 => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Préfixe").size(12.5).color(MUTED));
                        ui.add(egui::TextEdit::singleline(&mut o.prefix).desired_width(110.0));
                        ui.add(egui::DragValue::new(&mut o.start_no).range(0..=99999).prefix("Premier n° "));
                        ui.add(egui::DragValue::new(&mut o.step).range(1..=30).prefix("Une image sur "));
                    });
                    ui.label(RichText::new(format!("{}00000.{}", o.prefix, if o.format == 2 { "png" } else { "svg" })).size(11.5).color(FAINT));
                }
                6 => {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut o.cols).range(1..=12).prefix("Colonnes "));
                        ui.add(egui::DragValue::new(&mut o.rows).range(1..=12).prefix("Lignes "));
                    });
                }
                7 => {
                    ui.label(RichText::new("Formes, tracés, nuls, parents, transformations, tracés coupés, remplissages et contours. Les textes, images et effets ne sont pas exportés.").size(11.5).color(FAINT));
                }
                _ => {}
            }
            ui.add_space(6.0);
            if matches!(o.format, 1 | 2 | 3 | 4 | 5 | 6) {
                ui.checkbox(&mut o.transparent, "Fond transparent");
            } else {
                o.transparent = false;
            }
            ui.add_space(8.0);
            let n = work_of(&self.comp, &o);
            if matches!(o.format, 0 | 1 | 2 | 5) {
                ui.label(RichText::new(format!("{n} images · {}", secs(n as f32 / self.comp.fps as f32))).size(12.5).color(FAINT));
            }
            ui.add_space(14.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if pro::primary(ui, "Exporter…").clicked() {
                    go = true;
                }
                if pro::secondary(ui, "Ajouter à la file").on_hover_text("Ctrl+Alt+M ouvre la file de rendu").clicked() {
                    enqueue = true;
                }
                if pro::secondary(ui, "Annuler").clicked() {
                    cancel = true;
                }
            });
        });
        if go {
            self.run_export(&o);
        } else if enqueue {
            o.at = self.t;
            if let Some(p) = self.pick_path(&o) {
                self.queue.push(QItem { o: o.clone(), path: p });
                self.queue_sheet = true;
            }
        } else if keep && !cancel {
            self.export = Some(o);
        }
    }

    pub(crate) fn queue_sheet(&mut self, ctx: &egui::Context) {
        if !self.queue_sheet {
            return;
        }
        let mut close = false;
        let mut run = false;
        let mut remove = None;
        let mut add = false;
        let keep = pro::sheet(ctx, "queue", 520.0, |ui| {
            ui.label(RichText::new("File de rendu").font(semibold(18.0)).color(INK_STRONG));
            ui.label(RichText::new("Plusieurs exports à la suite, en une seule fois.").size(12.0).color(MUTED));
            ui.add_space(12.0);
            if self.queue.is_empty() {
                ui.label(RichText::new("La file est vide : dans Exporter, choisis « Ajouter à la file ».").size(12.5).color(FAINT));
            }
            for (k, q) in self.queue.iter().enumerate() {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::hover());
                let p = ui.painter();
                p.rect(r, CornerRadius::same(8), pro::WELL, Stroke::new(1.0, LINE), StrokeKind::Inside);
                p.text(r.left_top() + vec2(12.0, 6.0), Align2::LEFT_TOP, format!("{}. {} · {}", k + 1, FORMATS[q.o.format].0, SCALES[q.o.scale].0), semibold(12.5), INK);
                p.text(r.left_top() + vec2(12.0, 24.0), Align2::LEFT_TOP, short(&q.path.display().to_string(), 70), FontId::proportional(11.0), FAINT);
                let xr = Rect::from_center_size(r.right_center() - vec2(18.0, 0.0), Vec2::splat(22.0));
                if ui.put(xr, egui::Button::new("×").small()).on_hover_text("Retirer").clicked() {
                    remove = Some(k);
                }
                ui.add_space(4.0);
            }
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add_enabled(!self.queue.is_empty() && !self.busy(), egui::Button::new(RichText::new(format!("Lancer le rendu ({})", self.queue.len())).color(Color32::WHITE)).fill(BLUE).corner_radius(CornerRadius::same(7))).clicked() {
                    run = true;
                }
                if pro::secondary(ui, "Ajouter…").clicked() {
                    add = true;
                }
                if pro::secondary(ui, "Vider").clicked() {
                    self.queue.clear();
                }
                if pro::secondary(ui, "Fermer").clicked() {
                    close = true;
                }
            });
        });
        if let Some(k) = remove {
            self.queue.remove(k);
        }
        if run {
            let items = self.queue.drain(..).map(|q| (q.o, q.path)).collect();
            self.spawn(items);
            close = true;
        }
        if add {
            self.export = Some(self.default_export());
            close = true;
        }
        if close || !keep {
            self.queue_sheet = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lottie_and_frames() {
        let mut c = Comp::new();
        let mut l = Layer::new("r".into(), Content::Rect { w: 100.0, h: 50.0, r: 4.0, color: [255, 0, 0, 255] }, 10.0, 20.0, 5.0);
        l.normalize();
        l.props[0].add_key(0.0, 0.0);
        l.props[0].add_key(1.0, 100.0);
        l.props[15].add_key(0.0, 0.0);
        l.props[15].add_key(2.0, 100.0);
        c.layers.push(l);
        let mut n = Layer::new("n".into(), Content::Null, 0.0, 0.0, 5.0);
        n.normalize();
        c.layers.push(n);
        c.layers[0].parent = Some(1);
        let v = lottie(&c);
        assert_eq!(v["layers"].as_array().unwrap().len(), 2);
        assert_eq!(v["layers"][0]["ty"], 3);
        assert_eq!(v["layers"][1]["parent"], 2);
        assert_eq!(v["layers"][1]["ks"]["p"]["a"], 1);
        assert!(v["layers"][1]["shapes"][0]["it"].as_array().unwrap().iter().any(|s| s["ty"] == "tm"));
        let mut o = ExportOpts { format: 2, scale: 3, range: 2, from_f: 10, to_f: 19, transparent: false, gif_fps: 0, quality: 0, gif_loop: 0, gif_speed: 0, prefix: String::new(), start_no: 0, step: 3, cols: 2, rows: 2, at: 0.0 };
        assert_eq!(frames_of(&c, &o), vec![10, 13, 16, 19]);
        o.format = 6;
        assert_eq!(work_of(&c, &o), 4);
    }
}
