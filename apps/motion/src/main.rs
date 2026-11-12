#![windows_subsystem = "windows"]

#[allow(dead_code)]
#[path = "../../cut/src/media.rs"]
mod media;
mod model;
mod tutos;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key as EKey, Layout, Margin, Painter,
    Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2,
    pos2, vec2,
};
use model::{BLENDS, Comp, Content, Ease, Ease2, Layer, MATTES, Marker, PROP_DEFAULT, PROPS, Prop};
use resvg::{tiny_skia, usvg};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};

const KEY: Color32 = Color32::from_rgb(255, 196, 0);
const PLAYHEAD: Color32 = Color32::from_rgb(255, 69, 58);
const MARK: Color32 = Color32::from_rgb(100, 210, 255);
const LABELS: [Color32; 6] = [
    Color32::from_rgb(10, 132, 255),
    Color32::from_rgb(255, 92, 120),
    Color32::from_rgb(48, 209, 88),
    Color32::from_rgb(255, 159, 10),
    Color32::from_rgb(191, 90, 242),
    Color32::from_rgb(100, 210, 255),
];
const PRESETS: [(&str, &str, u32, u32); 6] = [
    ("1080p", "Vidéo 16:9", 1920, 1080),
    ("4K UHD", "Vidéo 16:9", 3840, 2160),
    ("Vertical", "Shorts, Reels, TikTok", 1080, 1920),
    ("Carré", "Publication 1:1", 1080, 1080),
    ("Portrait", "Publication 4:5", 1080, 1350),
    ("Bannière GIF", "Web", 800, 400),
];
const FONTS: [&str; 14] = ["Segoe UI", "Segoe UI Black", "Arial", "Arial Black", "Impact", "Bahnschrift", "Calibri", "Cambria", "Georgia", "Times New Roman", "Consolas", "Verdana", "Trebuchet MS", "Comic Sans MS"];
const GROUPS: [&str; 4] = ["Entrées", "Sorties", "Boucles", "Effets"];
const ANIMS: [(&str, &str, usize, usize); 27] = [
    ("Fondu entrant", "Opacité 0 → 100 %", 0, 0),
    ("Glisser depuis la gauche", "Entre par le côté", 0, 2),
    ("Glisser depuis la droite", "Entre par l'autre côté", 0, 15),
    ("Monter depuis le bas", "Entre par le bas", 0, 3),
    ("Descendre depuis le haut", "Tombe du haut", 0, 16),
    ("Zoom pop", "Grossit avec un rebond", 0, 4),
    ("Zoom arrière", "Arrive de très près", 0, 17),
    ("Fondu et montée", "Doux et élégant", 0, 18),
    ("Tourbillon entrant", "Tourne et grandit", 0, 10),
    ("Élastique", "Apparaît en vibrant", 0, 12),
    ("Rebond", "Tombe et rebondit", 0, 6),
    ("Révélation", "Se dévoile de gauche à droite", 0, 21),
    ("Flou entrant", "Sort du flou", 0, 22),
    ("Inclinaison", "Arrive penché puis se redresse", 0, 23),
    ("Machine à écrire", "Texte lettre par lettre", 0, 8),
    ("Fondu sortant", "Opacité 100 → 0 %", 1, 1),
    ("Sortie à droite", "Quitte l'écran", 1, 14),
    ("Sortie vers le bas", "S'efface en descendant", 1, 25),
    ("Pulsation", "Respire en boucle", 2, 7),
    ("Battement de cœur", "Double pulsation", 2, 20),
    ("Balancement", "Oscille de gauche à droite", 2, 19),
    ("Flottement", "Monte et descend en douceur", 2, 24),
    ("Rotation continue", "Un tour complet", 2, 5),
    ("Clignotement", "Apparaît et disparaît", 3, 13),
    ("Tremblement", "Caméra à l'épaule", 3, 9),
    ("Zoom lent", "Effet Ken Burns", 3, 11),
    ("Progression", "Révélation sur toute la durée", 3, 26),
];
const TEMPLATES: [(&str, &str); 6] = [
    ("Bandeau titre", "Nom et fonction en bas d'écran"),
    ("Titre cinéma", "Grand titre espacé qui sort du flou"),
    ("Bouton S'abonner", "Bouton rouge qui rebondit"),
    ("Compte à rebours", "10 → 0 avec anneau"),
    ("Barre de progression", "Se remplit de 0 à 100 %"),
    ("Citation", "Texte en italique et auteur"),
];
const FORMATS: [&str; 5] = ["MP4", "GIF", "PNG (séquence)", "PNG (image)", "SVG"];
const SCALES: [(&str, f32); 4] = [("25 %", 0.25), ("50 %", 0.5), ("100 %", 1.0), ("200 %", 2.0)];
const GIF_FPS: [(&str, u32); 3] = [("Composition", 0), ("15 i/s", 15), ("10 i/s", 10)];
const FPS_CHOICES: [u32; 6] = [12, 24, 25, 30, 50, 60];

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Rotate,
    Hand,
}

#[derive(Clone, Copy, PartialEq)]
enum G {
    Select,
    Rotate,
    Hand,
    Grid,
    Safe,
    Guides,
    Export,
    Eye,
    Lock,
    Solo,
    Loop,
    Magnet,
    Marker,
    Half,
}

struct ExportOpts {
    format: usize,
    scale: usize,
    range: usize,
    transparent: bool,
    gif_fps: usize,
}

#[derive(Clone, Copy)]
enum XDrag {
    Move { wx: f32, wy: f32, from: Pos2 },
    Scale { ws: f32, d0: f32 },
    Rotate { r0: f32, a0: f32 },
    Pan { p0: Vec2, from: Pos2 },
}

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
    rendered: Option<(Comp, f32, u32)>,
    xdrag: Option<XDrag>,
    bar_drag: Option<(usize, u8, f32, Layer)>,
    key_drag: Option<(usize, usize, usize)>,
    pps: f32,
    job: Option<Job>,
    error: Option<String>,
    title: String,
    icon: TextureHandle,
    help: studio_ui::help::Help,
    tool: Tool,
    zoom: Option<f32>,
    pan: Vec2,
    grid: bool,
    safe: bool,
    guides: bool,
    half: bool,
    loop_play: bool,
    snap: bool,
    export: Option<ExportOpts>,
    comp_sheet: bool,
    clip_layer: Option<Layer>,
    clip_anim: Option<(f32, Vec<Prop>)>,
    anim_query: String,
}

fn recents_path() -> PathBuf {
    studio_core::data_dir().join("motion").join("recents.txt")
}

fn out_size(comp: &Comp, scale: f32, even: bool) -> (u32, u32) {
    let f = |v: u32| {
        let n = ((v as f32 * scale).round() as u32).max(2);
        if even { n & !1 } else { n }
    };
    (f(comp.width), f(comp.height))
}

fn render(comp: &Comp, t: f32, w: u32, h: u32, db: &Arc<usvg::fontdb::Database>, transparent: bool) -> Result<tiny_skia::Pixmap, String> {
    let opt = usvg::Options { fontdb: db.clone(), ..Default::default() };
    let svg = if transparent { comp.to_svg_with(t, true) } else { comp.to_svg(t) };
    let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| e.to_string())?;
    let mut pix = tiny_skia::Pixmap::new(w.max(1), h.max(1)).ok_or("taille invalide")?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(w as f32 / comp.width as f32, h as f32 / comp.height as f32), &mut pix.as_mut());
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

fn rot(x: f32, y: f32, deg: f32) -> (f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    (x * c - y * s, x * s + y * c)
}

fn demo_comp(w: u32, h: u32) -> Comp {
    let mut comp = Comp { width: w, height: h, ..Comp::new() };
    let mut title = Layer::new("Titre".into(), Content::text("Ethone Motion", (w.min(h) as f32 * 0.11).max(24.0), [255, 255, 255, 255], true), w as f32 / 2.0, h as f32 / 2.0, 5.0);
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
        let recents = std::fs::read_to_string(recents_path()).map(|s| s.lines().map(PathBuf::from).filter(|p| p.exists()).collect()).unwrap_or_default();
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
            xdrag: None,
            bar_drag: None,
            key_drag: None,
            pps: 160.0,
            job: None,
            error: None,
            title: String::new(),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "motion"),
            help: studio_ui::help::Help::new("Motion", tutos::MOTION),
            tool: Tool::Select,
            zoom: None,
            pan: Vec2::ZERO,
            grid: false,
            safe: false,
            guides: false,
            half: false,
            loop_play: true,
            snap: true,
            export: None,
            comp_sheet: false,
            clip_layer: None,
            clip_anim: None,
            anim_query: String::new(),
        };
        if let Some(p) = std::env::args().nth(1).map(PathBuf::from) {
            if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("emot")) {
                me.open_path(&p);
            } else if p.as_os_str() == "--demo" {
                me.home = false;
                me.t = 1.2;
            }
        }
        me.comp.normalize();
        me
    }

    fn snap_frame(&self, t: f32) -> f32 {
        (t * self.comp.fps as f32).round() / self.comp.fps as f32
    }

    fn snap_time(&self, t: f32, skip_layer: Option<usize>, skip_prop: Option<(usize, usize)>) -> f32 {
        let t = self.snap_frame(t);
        if !self.snap {
            return t;
        }
        let tol = 7.0 / self.pps;
        let mut cands = vec![0.0, self.comp.duration, self.t];
        cands.extend(self.comp.markers.iter().map(|m| m.t));
        for (li, l) in self.comp.layers.iter().enumerate() {
            if skip_layer == Some(li) {
                continue;
            }
            cands.push(l.start);
            cands.push(l.end);
            for (pi, p) in l.props.iter().enumerate() {
                if skip_prop != Some((li, pi)) {
                    cands.extend(p.keys.iter().map(|k| k.t));
                }
            }
        }
        cands.into_iter().filter(|c| (c - t).abs() < tol).min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs())).unwrap_or(t)
    }

    fn set_comp(&mut self, mut comp: Comp, file: Option<PathBuf>) {
        comp.normalize();
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
        self.zoom = None;
        self.pan = Vec2::ZERO;
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

    fn push_layer(&mut self, mut l: Layer) -> usize {
        l.normalize();
        let at = self.sel.map_or(self.comp.layers.len(), |s| (s + 1).min(self.comp.layers.len()));
        for o in &mut self.comp.layers {
            if let Some(p) = &mut o.parent
                && *p >= at
            {
                *p += 1;
            }
        }
        self.comp.layers.insert(at, l);
        self.expanded.insert(at.min(self.expanded.len()), false);
        self.sel = Some(at);
        at
    }

    fn add_layer(&mut self, name: &str, content: Content) -> usize {
        let (cx, cy) = (self.comp.width as f32 / 2.0, self.comp.height as f32 / 2.0);
        let l = Layer::new(format!("{name} {}", self.comp.layers.len() + 1), content, cx, cy, self.comp.duration);
        self.push_layer(l)
    }

    fn add_kind(&mut self, kind: u8) {
        let (cw, ch) = (self.comp.width as f32, self.comp.height as f32);
        let u = cw.min(ch);
        match kind {
            0 => self.add_layer("Texte", Content::text("Texte", (u * 0.09).max(24.0), [255, 255, 255, 255], true)),
            1 => self.add_layer("Rectangle", Content::Rect { w: u * 0.37, h: u * 0.22, r: u * 0.02, color: [10, 132, 255, 255] }),
            2 => self.add_layer("Ellipse", Content::Ellipse { w: u * 0.28, h: u * 0.28, color: [255, 92, 120, 255] }),
            3 => {
                self.add_image();
                return;
            }
            4 => self.add_layer("Étoile", Content::Star { points: 5, radius: u * 0.15, inner: 0.45, color: [255, 196, 0, 255] }),
            5 => self.add_layer("Polygone", Content::Star { points: 6, radius: u * 0.15, inner: 1.0, color: [48, 209, 88, 255] }),
            6 => self.add_layer("Ligne", Content::Line { w: u * 0.55, thickness: (u * 0.011).max(2.0), color: [255, 255, 255, 255] }),
            7 => self.add_layer("Compteur", Content::Counter { from: 0.0, to: 100.0, decimals: 0, size: u * 0.15, color: [255, 255, 255, 255], prefix: String::new(), suffix: String::new() }),
            8 => self.add_layer("Triangle", Content::Star { points: 3, radius: u * 0.15, inner: 1.0, color: [191, 90, 242, 255] }),
            9 => self.add_layer("Flèche", Content::Arrow { w: u * 0.4, thickness: (u * 0.03).max(4.0), color: [255, 255, 255, 255] }),
            10 => self.add_layer("Anneau", Content::Ring { radius: u * 0.14, thickness: (u * 0.025).max(3.0), color: [100, 210, 255, 255] }),
            _ => {
                let i = self.add_layer("Fond", Content::Rect { w: cw, h: ch, r: 0.0, color: [36, 38, 52, 255] });
                let l = self.comp.layers.remove(i);
                self.comp.layers.insert(0, l);
                self.expanded.insert(0, false);
                self.expanded.remove(i + 1);
                for o in &mut self.comp.layers[1..] {
                    if let Some(p) = &mut o.parent
                        && *p < i
                    {
                        *p += 1;
                    }
                }
                self.sel = Some(0);
                return;
            }
        };
    }

    fn add_image(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "svg"]).pick_file() else { return };
        let dims = if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
            std::fs::read(&p).ok().and_then(|b| usvg::Tree::from_data(&b, &usvg::Options::default()).ok()).map(|t| (t.size().width() as u32, t.size().height() as u32)).ok_or("SVG illisible".to_string())
        } else {
            image::image_dimensions(&p).map_err(|e| e.to_string())
        };
        match dims {
            Ok((w, h)) => {
                let fit = (self.comp.width as f32 * 0.6 / w as f32).min(self.comp.height as f32 * 0.6 / h as f32).min(1.0);
                let name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or("Image".into());
                self.add_layer(&name, Content::Image { path: p.to_string_lossy().into_owned(), w: w as f32 * fit, h: h as f32 * fit });
            }
            Err(e) => self.error = Some(format!("Image illisible : {e}")),
        }
    }

    fn template(&mut self, k: usize) {
        let (cw, ch, dur) = (self.comp.width as f32, self.comp.height as f32, self.comp.duration);
        let u = cw.min(ch);
        let t = self.t;
        let mk = |name: &str, c: Content, x: f32, y: f32| {
            let mut l = Layer::new(name.into(), c, x, y, dur);
            l.start = t.min(dur - 0.1).max(0.0);
            l
        };
        match k {
            0 => {
                let (bw, bh) = (cw * 0.42, u * 0.13);
                let (x, y) = (cw * 0.06 + bw / 2.0, ch * 0.8);
                let mut bar = mk("Bandeau", Content::Rect { w: bw, h: bh, r: u * 0.012, color: [10, 132, 255, 235] }, x, y);
                bar.props[7].add_key(t, 0.0);
                bar.props[7].add_key(t + 0.5, 100.0);
                bar.props[7].keys[0].set_ease(Ease::Strong);
                bar.fx.shadow = true;
                let mut name = mk("Nom", Content::text("Prénom Nom", bh * 0.36, [255; 4], true).styled("Segoe UI", false, 0.0, 1), cw * 0.06 + u * 0.03, y - bh * 0.13);
                name.props[4].add_key(t + 0.3, 0.0);
                name.props[4].add_key(t + 0.7, 100.0);
                name.props[0].add_key(t + 0.3, cw * 0.06 + u * 0.01);
                name.props[0].add_key(t + 0.7, cw * 0.06 + u * 0.03);
                name.props[0].keys[0].set_ease(Ease::Out);
                let mut role = mk("Fonction", Content::text("Fonction ou sujet", bh * 0.22, [220, 230, 255, 255], false).styled("Segoe UI", false, 0.0, 1), cw * 0.06 + u * 0.03, y + bh * 0.22);
                role.props[4].add_key(t + 0.5, 0.0);
                role.props[4].add_key(t + 0.9, 100.0);
                self.push_layer(bar);
                self.push_layer(name);
                self.push_layer(role);
            }
            1 => {
                let mut title = mk("Titre cinéma", Content::text("TITRE", u * 0.13, [255; 4], false).styled("Segoe UI", false, u * 0.03, 0), cw / 2.0, ch / 2.0);
                title.props[6].add_key(t, 24.0);
                title.props[6].add_key(t + 1.4, 0.0);
                title.props[4].add_key(t, 0.0);
                title.props[4].add_key(t + 1.0, 100.0);
                title.props[2].add_key(t, 112.0);
                title.props[2].add_key(t + 2.5, 100.0);
                for p in [6, 4, 2] {
                    title.props[p].keys[0].set_ease(Ease::Out);
                }
                let mut line = mk("Filet", Content::Line { w: u * 0.5, thickness: (u * 0.004).max(2.0), color: [255, 255, 255, 200] }, cw / 2.0, ch / 2.0 + u * 0.11);
                line.props[7].add_key(t + 0.6, 0.0);
                line.props[7].add_key(t + 1.6, 100.0);
                line.props[7].keys[0].set_ease(Ease::Strong);
                self.push_layer(title);
                self.push_layer(line);
            }
            2 => {
                let mut btn = mk("Bouton", Content::Rect { w: u * 0.36, h: u * 0.1, r: u * 0.02, color: [255, 48, 64, 255] }, cw / 2.0, ch * 0.75);
                btn.props[2].add_key(t, 0.0);
                btn.props[2].add_key(t + 0.8, 100.0);
                btn.props[2].keys[0].set_ease(Ease::Elastic);
                btn.fx.shadow = true;
                let bi = self.push_layer(btn);
                let mut text = mk("Texte du bouton", Content::text("S'ABONNER", u * 0.042, [255; 4], true), cw / 2.0, ch * 0.75);
                text.parent = Some(bi);
                text.props[0].base = 0.0;
                text.props[1].base = 0.0;
                self.push_layer(text);
            }
            3 => {
                let mut ring = mk("Anneau", Content::Ring { radius: u * 0.2, thickness: u * 0.025, color: [255, 159, 10, 255] }, cw / 2.0, ch / 2.0);
                ring.props[7].add_key(t, 100.0);
                ring.props[7].add_key((t + dur * 0.8).min(dur), 0.0);
                ring.props[7].keys[0].set_ease(Ease::Linear);
                self.push_layer(ring);
                let mut n = mk("Compte", Content::Counter { from: 10.0, to: 0.0, decimals: 0, size: u * 0.2, color: [255; 4], prefix: String::new(), suffix: String::new() }, cw / 2.0, ch / 2.0);
                n.fx.glow = 8.0;
                n.fx.glow_color = [255, 159, 10, 160];
                self.push_layer(n);
            }
            4 => {
                let w = cw * 0.6;
                self.push_layer(mk("Piste", Content::Rect { w, h: u * 0.03, r: u * 0.015, color: [255, 255, 255, 40] }, cw / 2.0, ch / 2.0));
                let mut fill = mk("Remplissage", Content::Rect { w, h: u * 0.03, r: u * 0.015, color: [48, 209, 88, 255] }, cw / 2.0, ch / 2.0);
                fill.fx.gradient = Some([100, 210, 255, 255]);
                fill.fx.gradient_angle = -90.0;
                fill.props[7].add_key(t, 0.0);
                fill.props[7].add_key((t + dur * 0.8).min(dur), 100.0);
                self.push_layer(fill);
                let mut pct = mk("Pourcentage", Content::Counter { from: 0.0, to: 100.0, decimals: 0, size: u * 0.06, color: [255; 4], prefix: String::new(), suffix: " %".into() }, cw / 2.0, ch / 2.0 - u * 0.07);
                pct.start = t;
                self.push_layer(pct);
            }
            _ => {
                let mut q = mk("Citation", Content::text("« L'imagination est plus\nimportante que le savoir. »", u * 0.06, [255; 4], false).styled("Georgia", true, 0.0, 0), cw / 2.0, ch / 2.0 - u * 0.03);
                q.fx.typewriter = 2.5;
                let mut a = mk("Auteur", Content::text("— Albert Einstein", u * 0.035, [255, 214, 10, 255], true), cw / 2.0, ch / 2.0 + u * 0.12);
                a.props[4].add_key(t + 2.6, 0.0);
                a.props[4].add_key(t + 3.2, 100.0);
                self.push_layer(q);
                self.push_layer(a);
            }
        }
        self.expanded.resize(self.comp.layers.len(), false);
    }

    fn preset(&mut self, kind: usize) {
        let Some(i) = self.sel else { return };
        let t = self.t;
        let d = 0.6;
        let (cw, ch, dur) = (self.comp.width as f32, self.comp.height as f32, self.comp.duration);
        let l = &mut self.comp.layers[i];
        l.normalize();
        let x = l.props[0].at(t);
        let y = l.props[1].at(t);
        let (w, h) = l.size();
        let ease_all = |p: &mut Prop, e: Ease| p.keys.iter_mut().for_each(|k| k.set_ease(e));
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
                l.props[0].add_key(t, -w / 2.0);
                l.props[0].add_key(t + d, x);
                ease_all(&mut l.props[0], Ease::Out);
            }
            3 => {
                l.props[1].add_key(t, ch + h / 2.0);
                l.props[1].add_key(t + d, y);
                ease_all(&mut l.props[1], Ease::Out);
            }
            4 => {
                l.props[2].add_key(t, 0.0);
                l.props[2].add_key(t + d * 0.7, 115.0);
                l.props[2].add_key(t + d, 100.0);
            }
            5 => {
                l.props[3].add_key(t, 0.0);
                l.props[3].add_key(dur, 360.0);
                ease_all(&mut l.props[3], Ease::Linear);
            }
            6 => {
                let top = y - ch * 0.35;
                for (dt, v, e) in [(0.0, top, Ease::In), (0.45, y, Ease::Out), (0.7, y - ch * 0.08, Ease::In), (0.9, y, Ease::Smooth)] {
                    l.props[1].add_key(t + dt, v);
                    if let Some(k) = l.props[1].key_at(t + dt) {
                        l.props[1].keys[k].set_ease(e);
                    }
                }
            }
            7 => {
                let mut tt = t;
                let mut big = false;
                while tt <= dur + 1e-3 {
                    l.props[2].add_key(tt, if big { 110.0 } else { 100.0 });
                    big = !big;
                    tt += 0.5;
                }
            }
            8 => {
                if matches!(l.content, Content::Text { .. }) {
                    l.start = t.min(l.end - 0.1).max(0.0);
                    l.fx.typewriter = 1.5;
                }
            }
            9 => {
                l.fx.wiggle = 10.0;
                l.fx.wiggle_freq = 4.0;
            }
            10 => {
                l.props[3].add_key(t, -180.0);
                l.props[3].add_key(t + d, 0.0);
                l.props[2].add_key(t, 0.0);
                l.props[2].add_key(t + d, 100.0);
                for pi in [2, 3] {
                    ease_all(&mut l.props[pi], Ease::Out);
                }
            }
            11 => {
                l.props[2].add_key(t, 100.0);
                l.props[2].add_key(dur, 118.0);
                ease_all(&mut l.props[2], Ease::Linear);
            }
            12 => {
                l.props[2].add_key(t, 0.0);
                l.props[2].add_key(t + 1.0, 100.0);
                if let Some(k) = l.props[2].key_at(t) {
                    l.props[2].keys[k].set_ease(Ease::Elastic);
                }
            }
            13 => {
                let mut on = false;
                let mut tt = t;
                while tt <= t + 1.2 + 1e-3 {
                    l.props[4].add_key(tt, if on { 100.0 } else { 0.0 });
                    on = !on;
                    tt += 0.15;
                }
                l.props[4].add_key(tt, 100.0);
                ease_all(&mut l.props[4], Ease::Hold);
            }
            14 => {
                l.props[0].add_key(t, x);
                l.props[0].add_key(t + d, cw + w / 2.0);
                ease_all(&mut l.props[0], Ease::In);
            }
            15 => {
                l.props[0].add_key(t, cw + w / 2.0);
                l.props[0].add_key(t + d, x);
                ease_all(&mut l.props[0], Ease::Out);
            }
            16 => {
                l.props[1].add_key(t, -h / 2.0);
                l.props[1].add_key(t + d, y);
                ease_all(&mut l.props[1], Ease::Out);
            }
            17 => {
                l.props[2].add_key(t, 170.0);
                l.props[2].add_key(t + d, 100.0);
                l.props[4].add_key(t, 0.0);
                l.props[4].add_key(t + d * 0.6, 100.0);
                ease_all(&mut l.props[2], Ease::Out);
            }
            18 => {
                l.props[4].add_key(t, 0.0);
                l.props[4].add_key(t + d, 100.0);
                l.props[1].add_key(t, y + h.min(ch * 0.08));
                l.props[1].add_key(t + d, y);
                ease_all(&mut l.props[1], Ease::Strong);
            }
            19 => {
                let mut tt = t;
                let mut left = true;
                while tt <= dur + 1e-3 {
                    l.props[3].add_key(tt, if left { -8.0 } else { 8.0 });
                    left = !left;
                    tt += 0.6;
                }
                ease_all(&mut l.props[3], Ease::Smooth);
            }
            20 => {
                for (dt, v) in [(0.0, 100.0), (0.12, 118.0), (0.24, 100.0), (0.36, 112.0), (0.6, 100.0), (1.0, 100.0)] {
                    l.props[2].add_key(t + dt, v);
                }
                l.fx.loop_keys = true;
            }
            21 => {
                l.props[7].add_key(t, 0.0);
                l.props[7].add_key(t + d * 1.4, 100.0);
                ease_all(&mut l.props[7], Ease::Strong);
            }
            22 => {
                l.props[6].add_key(t, 30.0);
                l.props[6].add_key(t + d, 0.0);
                l.props[4].add_key(t, 0.0);
                l.props[4].add_key(t + d, 100.0);
                ease_all(&mut l.props[6], Ease::Out);
            }
            23 => {
                l.props[5].add_key(t, 30.0);
                l.props[5].add_key(t + d, 0.0);
                l.props[4].add_key(t, 0.0);
                l.props[4].add_key(t + d * 0.5, 100.0);
                ease_all(&mut l.props[5], Ease::Back);
            }
            24 => {
                l.props[1].add_key(t, y);
                l.props[1].add_key(t + 1.2, y - ch * 0.02);
                for k in &mut l.props[1].keys {
                    k.ease2 = Ease2::Sine;
                }
                l.fx.pingpong = true;
            }
            25 => {
                l.props[4].add_key(t, 100.0);
                l.props[4].add_key(t + d, 0.0);
                l.props[1].add_key(t, y);
                l.props[1].add_key(t + d, y + h.min(ch * 0.08));
                ease_all(&mut l.props[1], Ease::In);
            }
            26 => {
                l.props[7].add_key(l.start, 0.0);
                l.props[7].add_key(l.end, 100.0);
                ease_all(&mut l.props[7], Ease::Linear);
            }
            _ => {}
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
        let read = std::fs::read(p).map_err(|e| e.to_string()).and_then(|b| {
            let b = b.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&b);
            serde_json::from_slice::<Comp>(b).map_err(|e| e.to_string())
        });
        match read {
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

    fn busy(&self) -> bool {
        self.job.as_ref().is_some_and(|j| j.done.load(Ordering::Relaxed) != usize::MAX)
    }

    fn run_export(&mut self, o: &ExportOpts) {
        let comp = self.comp.clone();
        let db = self.fontdb.clone();
        let scale = SCALES[o.scale].1;
        let transparent = o.transparent;
        let (a, b) = if o.range == 1 { comp.range() } else { (0.0, comp.duration) };
        let first = (a * comp.fps as f32).round() as u32;
        let last = ((b * comp.fps as f32).round() as u32).max(first + 1);
        match o.format {
            3 | 4 => {
                let svg = o.format == 4;
                let ext = if svg { "svg" } else { "png" };
                let Some(path) = rfd::FileDialog::new().add_filter(ext.to_uppercase(), &[ext]).set_file_name(format!("image.{ext}")).save_file() else { return };
                let r = if svg {
                    std::fs::write(&path, comp.to_svg_with(self.t, transparent)).map_err(|e| e.to_string())
                } else {
                    let (w, h) = out_size(&comp, scale, false);
                    render(&comp, self.t, w, h, &db, transparent).and_then(|pix| pix.save_png(&path).map_err(|e| e.to_string()))
                };
                if let Err(e) = r {
                    self.error = Some(format!("Export : {e}"));
                }
                return;
            }
            _ => {}
        }
        let done = Arc::new(AtomicUsize::new(0));
        let error = Arc::new(Mutex::new(None));
        let (d2, e2) = (done.clone(), error.clone());
        let finish = move |r: Result<(), String>| {
            if let Err(e) = r
                && let Ok(mut slot) = e2.lock()
            {
                *slot = Some(e);
            }
            d2.store(usize::MAX, Ordering::Relaxed);
        };
        let d3 = done.clone();
        let fps = comp.fps;
        let (label, total): (&str, usize) = match o.format {
            0 => {
                let Some(path) = rfd::FileDialog::new().add_filter("Vidéo MP4", &["mp4"]).set_file_name("animation.mp4").save_file() else { return };
                std::thread::spawn(move || {
                    let r = (|| -> Result<(), String> {
                        let (w, h) = out_size(&comp, scale, true);
                        let bitrate = ((w * h) as f32 * fps as f32 * 0.14).max(2_000_000.0) as u32;
                        let mut enc = media::Encoder::new(&path, w, h, fps, bitrate)?;
                        let mut written = 0u64;
                        for (n, f) in (first..last).enumerate() {
                            let pix = render(&comp, f as f32 / fps as f32, w, h, &db, false)?;
                            enc.video(pix.data())?;
                            let need = ((n as u64 + 1) * media::OUT_RATE as u64 / fps as u64).saturating_sub(written);
                            enc.audio(&vec![0.0; need as usize * 2])?;
                            written += need;
                            d3.fetch_add(1, Ordering::Relaxed);
                        }
                        enc.finish()
                    })();
                    finish(r);
                });
                ("MP4", (last - first) as usize)
            }
            1 => {
                let Some(path) = rfd::FileDialog::new().add_filter("GIF", &["gif"]).set_file_name("animation.gif").save_file() else { return };
                let target = GIF_FPS[o.gif_fps].1;
                let step = if target == 0 || target >= fps { 1 } else { (fps as f32 / target as f32).round().max(1.0) as u32 };
                std::thread::spawn(move || {
                    let r = (|| -> Result<(), String> {
                        let (w, h) = out_size(&comp, scale, false);
                        let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
                        let mut enc = image::codecs::gif::GifEncoder::new_with_speed(std::io::BufWriter::new(file), 10);
                        enc.set_repeat(image::codecs::gif::Repeat::Infinite).map_err(|e| e.to_string())?;
                        let mut f = first;
                        while f < last {
                            let pix = render(&comp, f as f32 / fps as f32, w, h, &db, transparent)?;
                            let frame = image::Frame::from_parts(unpremultiply(&pix), 0, 0, image::Delay::from_numer_denom_ms(1000 * step, fps));
                            enc.encode_frame(frame).map_err(|e| e.to_string())?;
                            d3.fetch_add(step as usize, Ordering::Relaxed);
                            f += step;
                        }
                        Ok(())
                    })();
                    finish(r);
                });
                ("GIF", (last - first) as usize)
            }
            _ => {
                let Some(dir) = rfd::FileDialog::new().set_title("Dossier de la séquence PNG").pick_folder() else { return };
                std::thread::spawn(move || {
                    let r = (|| -> Result<(), String> {
                        let (w, h) = out_size(&comp, scale, false);
                        for (n, f) in (first..last).enumerate() {
                            let pix = render(&comp, f as f32 / fps as f32, w, h, &db, transparent)?;
                            pix.save_png(dir.join(format!("frame_{n:05}.png"))).map_err(|e| e.to_string())?;
                            d3.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(())
                    })();
                    finish(r);
                });
                ("séquence PNG", (last - first) as usize)
            }
        };
        self.job = Some(Job { done, total, error, label: label.into() });
    }

    fn split(&mut self) {
        let Some(i) = self.sel else { return };
        let t = self.t;
        let l = &self.comp.layers[i];
        if t <= l.start + 1e-3 || t >= l.end - 1e-3 {
            return;
        }
        let mut second = l.clone();
        second.start = t;
        second.name = format!("{} (suite)", l.name);
        self.comp.layers[i].end = t;
        self.sel = Some(i);
        self.push_layer(second);
    }

    fn move_layer(&mut self, from: usize, to: usize) {
        let n = self.comp.layers.len();
        if from >= n || to >= n || from == to {
            return;
        }
        let mut order: Vec<usize> = (0..n).collect();
        let x = order.remove(from);
        order.insert(to, x);
        let new_of: Vec<usize> = (0..n).map(|old| order.iter().position(|o| *o == old).unwrap()).collect();
        let old = std::mem::take(&mut self.comp.layers);
        let mut slots: Vec<Option<Layer>> = old.into_iter().map(Some).collect();
        self.comp.layers = order.iter().map(|o| slots[*o].take().unwrap()).collect();
        for l in &mut self.comp.layers {
            if let Some(p) = &mut l.parent {
                *p = new_of[*p];
            }
        }
        let exp = self.expanded.clone();
        self.expanded = order.iter().map(|o| exp.get(*o).copied().unwrap_or(false)).collect();
        self.sel = Some(to);
    }

    fn reorder(&mut self, by: i32) {
        let Some(i) = self.sel else { return };
        let to = i as i32 + by;
        if to >= 0 && (to as usize) < self.comp.layers.len() {
            self.move_layer(i, to as usize);
        }
    }

    fn duplicate(&mut self) {
        if let Some(i) = self.sel {
            let mut l = self.comp.layers[i].clone();
            l.name = format!("{} copie", l.name);
            self.push_layer(l);
        }
    }

    fn delete(&mut self) {
        if let Some(i) = self.sel.take() {
            self.comp.layers.remove(i);
            if i < self.expanded.len() {
                self.expanded.remove(i);
            }
            for l in &mut self.comp.layers {
                match l.parent {
                    Some(p) if p == i => l.parent = None,
                    Some(p) if p > i => l.parent = Some(p - 1),
                    _ => {}
                }
            }
        }
    }

    fn copy_anim(&mut self) {
        if let Some(l) = self.sel.and_then(|i| self.comp.layers.get(i)) {
            let t0 = l.key_times().first().copied().unwrap_or(0.0);
            self.clip_anim = Some((t0, l.props.clone()));
        }
    }

    fn paste_anim(&mut self) {
        let (Some(i), Some((t0, props))) = (self.sel, self.clip_anim.clone()) else { return };
        let dt = self.t - t0;
        let l = &mut self.comp.layers[i];
        for (pi, p) in props.into_iter().enumerate() {
            if p.animated()
                && let Some(dst) = l.props.get_mut(pi)
            {
                dst.keys = p.keys;
                dst.shift(dt);
            }
        }
    }

    fn fit_duration(&mut self) {
        let last = self.comp.layers.iter().flat_map(|l| l.key_times().last().copied()).fold(0.5f32, f32::max);
        let old = self.comp.duration;
        let new = (last + 0.5).max(0.5);
        for l in &mut self.comp.layers {
            if (l.end - old).abs() < 1e-3 || l.end > new {
                l.end = new;
            }
            l.start = l.start.min(new - 0.05);
        }
        self.comp.duration = new;
        self.t = self.t.min(new);
    }

    fn add_marker(&mut self) {
        let t = self.t;
        if !self.comp.markers.iter().any(|m| (m.t - t).abs() < 1e-3) {
            let n = self.comp.markers.len() + 1;
            self.comp.markers.push(Marker { t, name: format!("Repère {n}") });
            self.comp.markers.sort_by(|a, b| a.t.total_cmp(&b.t));
        }
    }

    fn set_work(&mut self, start: bool) {
        let (a, b) = self.comp.work.unwrap_or((0.0, self.comp.duration));
        self.comp.work = Some(if start { (self.t, b.max(self.t + 0.05)) } else { (a.min(self.t - 0.05).max(0.0), self.t) });
    }

    fn parent_frame(&self, i: usize, t: f32) -> (f32, f32, f32, f32) {
        match self.comp.layers[i].parent.filter(|p| *p < self.comp.layers.len() && *p != i) {
            Some(p) => self.comp.world(p, t),
            None => (0.0, 0.0, 1.0, 0.0),
        }
    }

    fn place(&mut self, i: usize, wx: f32, wy: f32) {
        let t = self.t;
        let (px, py, ps, pr) = self.parent_frame(i, t);
        let (lx, ly) = rot(wx - px, wy - py, -pr);
        let (gx, gy) = self.comp.layers[i].wiggle(t, i);
        let ps = ps.max(1e-4);
        let l = &mut self.comp.layers[i];
        l.props[0].set(t, (lx / ps - gx).round());
        l.props[1].set(t, (ly / ps - gy).round());
    }

    fn align(&mut self, how: u8) {
        let Some(i) = self.sel else { return };
        let t = self.t;
        let (wx, wy, ws, _) = self.comp.world(i, t);
        let (w, h) = self.comp.layers[i].size();
        let (hw, hh) = (w * ws / 2.0, h * ws / 2.0);
        let (cw, ch) = (self.comp.width as f32, self.comp.height as f32);
        let (nx, ny) = match how {
            0 => (hw, wy),
            1 => (cw / 2.0, wy),
            2 => (cw - hw, wy),
            3 => (wx, hh),
            4 => (wx, ch / 2.0),
            5 => (wx, ch - hh),
            _ => (cw / 2.0, ch / 2.0),
        };
        self.place(i, nx, ny);
    }

    fn undo_step(&mut self) {
        if let Some(c) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.comp, c));
        }
    }

    fn redo_step(&mut self) {
        if let Some(c) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.comp, c));
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() || self.home {
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        if sc(M::COMMAND | M::SHIFT, EKey::Z) || sc(M::COMMAND, EKey::Y) {
            self.redo_step();
        } else if sc(M::COMMAND, EKey::Z) {
            self.undo_step();
        }
        if sc(M::COMMAND | M::SHIFT, EKey::S) {
            self.save(true);
        } else if sc(M::COMMAND, EKey::S) {
            self.save(false);
        }
        if sc(M::COMMAND, EKey::O) {
            self.open();
        }
        if sc(M::COMMAND | M::SHIFT, EKey::D) {
            self.split();
        } else if sc(M::COMMAND, EKey::D) {
            self.duplicate();
        }
        if sc(M::COMMAND, EKey::M) {
            self.export = Some(self.default_export());
        }
        if sc(M::COMMAND, EKey::K) {
            self.comp_sheet = true;
        }
        if sc(M::COMMAND, EKey::CloseBracket) {
            self.reorder(1);
        }
        if sc(M::COMMAND, EKey::OpenBracket) {
            self.reorder(-1);
        }
        if sc(M::COMMAND, EKey::C)
            && let Some(i) = self.sel
        {
            self.clip_layer = self.comp.layers.get(i).cloned();
        }
        if sc(M::COMMAND, EKey::V)
            && let Some(mut l) = self.clip_layer.clone()
        {
            l.parent = None;
            self.push_layer(l);
        }
        if sc(M::COMMAND, EKey::Num0) {
            self.zoom = None;
            self.pan = Vec2::ZERO;
        }
        if sc(M::COMMAND, EKey::Num1) {
            self.zoom = Some(1.0);
        }
        let step = 1.0 / self.comp.fps as f32;
        let nudge = |me: &mut Self, dx: f32, dy: f32| {
            if let Some(i) = me.sel {
                let (wx, wy, _, _) = me.comp.world(i, me.t);
                me.place(i, wx + dx, wy + dy);
            }
        };
        for (k, dx, dy) in [(EKey::ArrowLeft, -1.0, 0.0), (EKey::ArrowRight, 1.0, 0.0), (EKey::ArrowUp, 0.0, -1.0), (EKey::ArrowDown, 0.0, 1.0)] {
            if sc(M::COMMAND | M::SHIFT, k) {
                nudge(self, dx * 10.0, dy * 10.0);
            } else if sc(M::COMMAND, k) {
                nudge(self, dx, dy);
            }
        }
        let (space, left, right, shift, del, home, end, letters) = ctx.input(|i| {
            let p = |k| i.key_pressed(k) && !i.modifiers.command;
            (
                i.key_pressed(EKey::Space),
                i.key_pressed(EKey::ArrowLeft),
                i.key_pressed(EKey::ArrowRight),
                i.modifiers.shift,
                i.key_pressed(EKey::Delete),
                i.key_pressed(EKey::Home),
                i.key_pressed(EKey::End),
                [p(EKey::J), p(EKey::K), p(EKey::B), p(EKey::N), p(EKey::M), p(EKey::V), p(EKey::W), p(EKey::H), p(EKey::L), p(EKey::U), p(EKey::F9)],
            )
        });
        let jump = if shift { step * 10.0 } else { step };
        if space {
            self.playing = !self.playing;
        }
        if left {
            self.t = self.snap_frame((self.t - jump).max(0.0));
        }
        if right {
            self.t = self.snap_frame((self.t + jump).min(self.comp.duration));
        }
        if home {
            self.t = self.comp.range().0;
        }
        if end {
            self.t = self.comp.range().1;
        }
        if del {
            self.delete();
        }
        let [j, k, b, n, m, v, w, h, l, u, f9] = letters;
        if j && let Some(t) = self.comp.next_key(self.t, false, None) {
            self.t = t;
        }
        if k && let Some(t) = self.comp.next_key(self.t, true, None) {
            self.t = t;
        }
        if b {
            self.set_work(true);
        }
        if n {
            self.set_work(false);
        }
        if m {
            self.add_marker();
        }
        if v {
            self.tool = Tool::Select;
        }
        if w {
            self.tool = Tool::Rotate;
        }
        if h {
            self.tool = Tool::Hand;
        }
        if l {
            self.loop_play = !self.loop_play;
        }
        if u && let Some(i) = self.sel {
            if let Some(e) = self.expanded.get_mut(i) {
                *e = !*e;
            }
        }
        if f9 && let Some(i) = self.sel {
            for p in &mut self.comp.layers[i].props {
                p.keys.iter_mut().for_each(|k| k.set_ease(Ease::Smooth));
            }
        }
        self.comp.normalize();
        self.expanded.resize(self.comp.layers.len(), false);
        if self.sel.is_some_and(|i| i >= self.comp.layers.len()) {
            self.sel = None;
        }
    }

    fn default_export(&self) -> ExportOpts {
        ExportOpts { format: 0, scale: 2, range: if self.comp.work.is_some() { 1 } else { 0 }, transparent: false, gif_fps: 1 }
    }
}

impl eframe::App for Motion {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let before = self.comp.clone();
        self.keys(ctx);
        if self.playing {
            let dt = ctx.input(|i| i.stable_dt).min(0.1);
            let (a, b) = self.comp.range();
            self.t += dt;
            if self.t > b {
                if self.loop_play {
                    self.t = a;
                } else {
                    self.t = b;
                    self.playing = false;
                }
            }
            ctx.request_repaint();
        }
        let name = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = if self.home { "Motion".to_string() } else { format!("{name}{} — Motion", if self.comp != self.saved { " •" } else { "" }) };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        self.top_bar(root);
        self.status(root);
        if self.home {
            self.home_screen(root);
        } else {
            self.timeline(root);
            self.library(root);
            self.props(root);
            self.viewer(root, &name);
        }
        self.sheets(ctx);
        self.help.show(ctx);

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
    fn top_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(44.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let full = ui.max_rect();
            let right_w = if self.home { 0.0 } else { 330.0 };
            let left = Rect::from_min_max(full.min, pos2(full.right() - right_w, full.bottom()));
            ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(22.0))));
                    ui.add_space(4.0);
                    egui::MenuBar::new().ui(ui, |ui| self.menus(ui));
                });
            });
            if !self.home {
                let right = Rect::from_min_max(pos2(full.right() - right_w, full.top()), full.max);
                ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self.busy() {
                            if let Some(j) = &self.job {
                                let d = j.done.load(Ordering::Relaxed);
                                ui.add(egui::ProgressBar::new(d as f32 / j.total.max(1) as f32).desired_width(150.0).corner_radius(CornerRadius::same(6)).text(format!("{} {}%", j.label, d * 100 / j.total.max(1))));
                            }
                        } else if pro::primary_icon(ui, "Exporter", |p, r, c| glyph(p, r, G::Export, c)).on_hover_text("MP4, GIF, PNG ou SVG · Ctrl+M").clicked() {
                            self.export = Some(self.default_export());
                        }
                        ui.add_space(4.0);
                        let fps = self.comp.fps;
                        if pro::chip(ui, &format!("{} × {} · {} i/s · {}", self.comp.width, self.comp.height, fps, secs(self.comp.duration))).on_hover_text("Réglages de la composition · Ctrl+K").clicked() {
                            self.comp_sheet = true;
                        }
                    });
                });
            }
        });
    }

    fn menus(&mut self, ui: &mut Ui) {
        let has = self.sel.is_some();
        ui.menu_button("Fichier", |ui| {
            if ui.button("Nouvelle composition").clicked() {
                self.home = true;
            }
            if ui.button("Ouvrir…                        Ctrl+O").clicked() {
                self.open();
            }
            ui.menu_button("Ouvrir un projet récent", |ui| {
                let mut open = None;
                for p in &self.recents {
                    if ui.button(p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).clicked() {
                        open = Some(p.clone());
                    }
                }
                if self.recents.is_empty() {
                    ui.label(RichText::new("Aucun projet récent").color(FAINT));
                }
                if let Some(p) = open {
                    self.open_path(&p);
                }
            });
            ui.add_enabled_ui(!self.home, |ui| {
                ui.separator();
                if ui.button("Enregistrer                   Ctrl+S").clicked() {
                    self.save(false);
                }
                if ui.button("Enregistrer sous…     Ctrl+Maj+S").clicked() {
                    self.save(true);
                }
                ui.separator();
                if ui.button("Exporter…                     Ctrl+M").clicked() {
                    self.export = Some(self.default_export());
                }
                if ui.button("Exporter l'image courante en PNG…").clicked() {
                    self.run_export(&ExportOpts { format: 3, scale: 2, range: 0, transparent: false, gif_fps: 0 });
                }
                if ui.button("Exporter l'image courante en SVG…").clicked() {
                    self.run_export(&ExportOpts { format: 4, scale: 2, range: 0, transparent: false, gif_fps: 0 });
                }
            });
        });
        if self.home {
            self.help.menu(ui);
            return;
        }
        ui.menu_button("Édition", |ui| {
            if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler                    Ctrl+Z")).clicked() {
                self.undo_step();
            }
            if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir                   Ctrl+Y")).clicked() {
                self.redo_step();
            }
            ui.separator();
            if ui.add_enabled(has, egui::Button::new("Copier le calque         Ctrl+C")).clicked()
                && let Some(i) = self.sel
            {
                self.clip_layer = self.comp.layers.get(i).cloned();
            }
            if ui.add_enabled(self.clip_layer.is_some(), egui::Button::new("Coller le calque          Ctrl+V")).clicked()
                && let Some(mut l) = self.clip_layer.clone()
            {
                l.parent = None;
                self.push_layer(l);
            }
            if ui.add_enabled(has, egui::Button::new("Dupliquer                  Ctrl+D")).clicked() {
                self.duplicate();
            }
            if ui.add_enabled(has, egui::Button::new("Supprimer                  Suppr")).clicked() {
                self.delete();
            }
            ui.separator();
            if ui.add_enabled(has, egui::Button::new("Copier l'animation")).clicked() {
                self.copy_anim();
            }
            if ui.add_enabled(has && self.clip_anim.is_some(), egui::Button::new("Coller l'animation à la tête de lecture")).clicked() {
                self.paste_anim();
            }
            ui.separator();
            if ui.add_enabled(has, egui::Button::new("Tout désélectionner")).clicked() {
                self.sel = None;
            }
        });
        ui.menu_button("Calque", |ui| {
            ui.menu_button("Nouveau", |ui| {
                for (label, kind) in KINDS {
                    if ui.button(label).clicked() {
                        self.add_kind(kind);
                        ui.close();
                    }
                }
            });
            ui.menu_button("Modèle", |ui| {
                for (k, (name, _)) in TEMPLATES.iter().enumerate() {
                    if ui.button(*name).clicked() {
                        self.template(k);
                        ui.close();
                    }
                }
            });
            ui.separator();
            ui.add_enabled_ui(has, |ui| {
                if ui.button("Fractionner à la tête       Ctrl+Maj+D").clicked() {
                    self.split();
                }
                if ui.button("Monter                          Ctrl+]").clicked() {
                    self.reorder(1);
                }
                if ui.button("Descendre                     Ctrl+[").clicked() {
                    self.reorder(-1);
                }
                if ui.button("Premier plan").clicked()
                    && let Some(i) = self.sel
                {
                    self.move_layer(i, self.comp.layers.len() - 1);
                }
                if ui.button("Arrière-plan").clicked()
                    && let Some(i) = self.sel
                {
                    self.move_layer(i, 0);
                }
                ui.separator();
                if let Some(i) = self.sel {
                    let t = self.t;
                    let dur = self.comp.duration;
                    let l = &mut self.comp.layers[i];
                    if ui.button("Commencer à la tête de lecture").clicked() {
                        l.start = t.min(l.end - 0.05);
                    }
                    if ui.button("Finir à la tête de lecture").clicked() {
                        l.end = t.max(l.start + 0.05);
                    }
                    if ui.button("Étendre à toute la composition").clicked() {
                        l.start = 0.0;
                        l.end = dur;
                    }
                    ui.separator();
                    if ui.button("Courbe douce sur toutes les clés     F9").clicked() {
                        for p in &mut l.props {
                            p.keys.iter_mut().for_each(|k| k.set_ease(Ease::Smooth));
                        }
                    }
                    if ui.button("Inverser les images clés").clicked() {
                        let (a, b) = (l.key_times().first().copied().unwrap_or(0.0), l.key_times().last().copied().unwrap_or(0.0));
                        for p in &mut l.props {
                            p.reverse(a, b);
                        }
                    }
                    if ui.button("Supprimer toutes les images clés").clicked() {
                        for p in &mut l.props {
                            p.clear(t);
                        }
                    }
                    ui.separator();
                    ui.checkbox(&mut l.visible, "Visible");
                    ui.checkbox(&mut l.locked, "Verrouillé");
                    ui.checkbox(&mut l.solo, "Solo");
                }
            });
        });
        ui.menu_button("Composition", |ui| {
            if ui.button("Réglages de la composition…   Ctrl+K").clicked() {
                self.comp_sheet = true;
            }
            if ui.button("Ajuster la durée à l'animation").clicked() {
                self.fit_duration();
            }
            ui.separator();
            if ui.button("Ajouter un repère                    M").clicked() {
                self.add_marker();
            }
            if ui.add_enabled(!self.comp.markers.is_empty(), egui::Button::new("Supprimer tous les repères")).clicked() {
                self.comp.markers.clear();
            }
            ui.separator();
            if ui.button("Début de la zone de travail        B").clicked() {
                self.set_work(true);
            }
            if ui.button("Fin de la zone de travail            N").clicked() {
                self.set_work(false);
            }
            if ui.add_enabled(self.comp.work.is_some(), egui::Button::new("Effacer la zone de travail")).clicked() {
                self.comp.work = None;
            }
        });
        ui.menu_button("Affichage", |ui| {
            ui.checkbox(&mut self.grid, "Grille et tiers");
            ui.checkbox(&mut self.safe, "Zones sûres (titres et action)");
            ui.checkbox(&mut self.guides, "Repères centraux");
            ui.checkbox(&mut self.snap, "Magnétisme");
            ui.checkbox(&mut self.half, "Aperçu en demi-résolution");
            ui.checkbox(&mut self.loop_play, "Lecture en boucle               L");
            ui.separator();
            if ui.button("Ajuster à la fenêtre            Ctrl+0").clicked() {
                self.zoom = None;
                self.pan = Vec2::ZERO;
            }
            if ui.button("Taille réelle (100 %)          Ctrl+1").clicked() {
                self.zoom = Some(1.0);
            }
            if ui.button("50 %").clicked() {
                self.zoom = Some(0.5);
            }
        });
        self.help.menu(ui);
    }

    fn status(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                if !self.home {
                    ui.label(small("Espace lire · ← → image · J K clé préc./suiv. · M repère · B N zone · V W H outils · F9 courbe douce · U déplier".into()));
                }
                if let Some(j) = &self.job
                    && j.done.load(Ordering::Relaxed) == usize::MAX
                {
                    match j.error.lock().ok().and_then(|e| e.clone()) {
                        Some(e) => ui.label(RichText::new(format!("Export {} : {e}", j.label)).size(12.0).color(RED)),
                        None => ui.label(RichText::new(format!("Export {} terminé", j.label)).size(12.0).color(GREEN)),
                    };
                }
                if self.busy() {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
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
        egui::Panel::left("library").resizable(false).exact_size(256.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                pro::dock_panel(ui, "add", "Ajouter un calque", |ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    let w = (ui.available_width() - 12.0) / 3.0;
                    for row in KINDS.chunks(3) {
                        ui.horizontal(|ui| {
                            for (label, kind) in row {
                                let (r, resp) = ui.allocate_exact_size(vec2(w, 58.0), Sense::click());
                                let p = ui.painter();
                                p.rect(r, CornerRadius::same(9), if resp.hovered() { FIELD } else { Color32::from_rgb(0x26, 0x26, 0x2b) }, Stroke::new(1.0, LINE), StrokeKind::Inside);
                                layer_glyph(p, Rect::from_center_size(r.center() - vec2(0.0, 8.0), Vec2::splat(18.0)), *kind, INK);
                                p.text(r.center() + vec2(0.0, 16.0), Align2::CENTER_CENTER, *label, FontId::proportional(11.5), INK);
                                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    self.add_kind(*kind);
                                }
                            }
                        });
                    }
                });
                pro::dock_panel(ui, "templates", "Modèles", |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let mut pick = None;
                    for (k, (name, hint)) in TEMPLATES.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
                        let p = ui.painter();
                        if resp.hovered() {
                            p.rect_filled(r, CornerRadius::same(8), Color32::from_white_alpha(10));
                        }
                        let ic = Rect::from_center_size(r.left_center() + vec2(14.0, 0.0), vec2(16.0, 11.0));
                        p.rect(ic, CornerRadius::same(2), LABELS[k % LABELS.len()].gamma_multiply(0.35), Stroke::new(1.2, LABELS[k % LABELS.len()]), StrokeKind::Inside);
                        p.text(r.left_top() + vec2(30.0, 5.0), Align2::LEFT_TOP, *name, FontId::proportional(13.0), INK);
                        p.text(r.left_top() + vec2(30.0, 22.0), Align2::LEFT_TOP, *hint, FontId::proportional(11.0), FAINT);
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Ajouté à la tête de lecture").clicked() {
                            pick = Some(k);
                        }
                    }
                    if let Some(k) = pick {
                        self.template(k);
                    }
                });
                pro::dock_panel(ui, "anims", "Animations prêtes", |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.add(egui::TextEdit::singleline(&mut self.anim_query).hint_text("Rechercher une animation").desired_width(f32::INFINITY));
                    ui.add_space(4.0);
                    let enabled = self.sel.is_some();
                    if !enabled {
                        ui.label(RichText::new("Sélectionne un calque pour lui appliquer une animation.").size(12.0).color(FAINT));
                    }
                    let q = self.anim_query.to_lowercase();
                    let mut pick = None;
                    for (gi, group) in GROUPS.iter().enumerate() {
                        let items: Vec<_> = ANIMS.iter().filter(|a| a.2 == gi && (q.is_empty() || a.0.to_lowercase().contains(&q) || a.1.to_lowercase().contains(&q))).collect();
                        if items.is_empty() {
                            continue;
                        }
                        ui.add_space(6.0);
                        ui.label(RichText::new(group.to_uppercase()).size(10.5).color(FAINT).strong());
                        for (name, hint, _, id) in items {
                            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), if enabled { Sense::click() } else { Sense::hover() });
                            let p = ui.painter();
                            if resp.hovered() && enabled {
                                p.rect_filled(r, CornerRadius::same(8), Color32::from_white_alpha(10));
                            }
                            diamond(p, r.left_center() + vec2(12.0, 0.0), 4.5, if enabled { KEY } else { FAINT });
                            p.text(r.left_top() + vec2(26.0, 4.0), Align2::LEFT_TOP, *name, FontId::proportional(13.0), if enabled { INK } else { FAINT });
                            p.text(r.left_top() + vec2(26.0, 21.0), Align2::LEFT_TOP, *hint, FontId::proportional(11.0), FAINT);
                            if resp.on_hover_text("Ajoutée à partir de la tête de lecture").clicked() && enabled {
                                pick = Some(*id);
                            }
                        }
                    }
                    if let Some(id) = pick {
                        self.preset(id);
                    }
                });
            });
        });
    }

    fn props(&mut self, root: &mut Ui) {
        egui::Panel::right("props").resizable(false).exact_size(304.0).frame(Frame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                match self.sel.filter(|i| *i < self.comp.layers.len()) {
                    None => self.comp_panels(ui),
                    Some(i) => self.layer_panels(ui, i),
                }
            });
        });
    }

    fn comp_panels(&mut self, ui: &mut Ui) {
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
            if pro::secondary(ui, "Réglages…").clicked() {
                self.comp_sheet = true;
            }
            ui.add_space(4.0);
            bg_editor(ui, &mut self.comp);
        });
        pro::dock_panel(ui, "markers", "Repères", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            if self.comp.markers.is_empty() {
                ui.label(RichText::new("Appuie sur M pour poser un repère à la tête de lecture : il sert de point d'accroche et de raccourci J/K.").size(12.0).color(FAINT));
            }
            let mut remove = None;
            let mut go = None;
            for (k, m) in self.comp.markers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new(secs(m.t)).monospace().size(11.5)).frame(false)).on_hover_text("Aller au repère").clicked() {
                        go = Some(m.t);
                    }
                    ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(150.0));
                    if ui.small_button("×").clicked() {
                        remove = Some(k);
                    }
                });
            }
            if let Some(k) = remove {
                self.comp.markers.remove(k);
            }
            if let Some(t) = go {
                self.t = t;
            }
            if pro::ghost(ui, "+ Repère à la tête de lecture").clicked() {
                self.add_marker();
            }
        });
    }

    fn layer_panels(&mut self, ui: &mut Ui, i: usize) {
        let t = self.t;
        let dur = self.comp.duration;
        let names: Vec<(usize, String)> = self.comp.layers.iter().enumerate().filter(|(k, _)| *k != i && !self.comp.chain_has(*k, i)).map(|(k, l)| (k, l.name.clone())).collect();
        let mut new_parent: Option<Option<usize>> = None;
        let cur_parent = self.comp.layers[i].parent;
        {
            let l = &mut self.comp.layers[i];
            pro::dock_panel(ui, "layer", "Calque", |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.add(egui::TextEdit::singleline(&mut l.name).desired_width(f32::INFINITY));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut l.visible, "Visible");
                    ui.checkbox(&mut l.locked, "Verrou");
                    ui.checkbox(&mut l.solo, "Solo");
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("De").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut l.start).range(0.0..=l.end).speed(0.05).suffix(" s"));
                    ui.label(RichText::new("à").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut l.end).range(l.start..=dur).speed(0.05).suffix(" s"));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Parent").size(12.5).color(MUTED));
                    let label = cur_parent.and_then(|p| names.iter().find(|n| n.0 == p)).map_or("Aucun".to_string(), |n| n.1.clone());
                    egui::ComboBox::from_id_salt("parent").selected_text(short(&label, 22)).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        if ui.selectable_label(cur_parent.is_none(), "Aucun").clicked() {
                            new_parent = Some(None);
                        }
                        for (k, n) in &names {
                            if ui.selectable_label(cur_parent == Some(*k), n).clicked() {
                                new_parent = Some(Some(*k));
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Couleur").size(12.5).color(MUTED));
                    let cur = l.label.unwrap_or((i % LABELS.len()) as u8);
                    for (k, c) in LABELS.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                        ui.painter().circle_filled(r.center(), 7.0, *c);
                        if cur as usize == k {
                            ui.painter().circle_stroke(r.center(), 9.0, Stroke::new(1.5, Color32::WHITE));
                        }
                        if resp.clicked() {
                            l.label = Some(k as u8);
                        }
                    }
                });
            });
        }
        if let Some(p) = new_parent
            && !self.comp.set_parent(i, p, t)
        {
            self.error = Some("Ce parent créerait une boucle".into());
        }
        let mut go = None;
        {
            let l = &mut self.comp.layers[i];
            pro::dock_panel(ui, "transform", "Transformation", |ui| {
                ui.spacing_mut().item_spacing.y = 5.0;
                for (pi, label) in PROPS.iter().enumerate() {
                    let p = &mut l.props[pi];
                    ui.horizontal(|ui| {
                        let has = p.key_at(t).is_some();
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                        let c = r.center();
                        let d = 5.5;
                        ui.painter().add(Shape::convex_polygon(vec![c + vec2(0.0, -d), c + vec2(d, 0.0), c + vec2(0.0, d), c + vec2(-d, 0.0)], if has { KEY } else { Color32::TRANSPARENT }, Stroke::new(1.5, if p.animated() { KEY } else { FAINT })));
                        if resp.on_hover_text("Ajouter ou retirer une image clé ici").clicked() {
                            p.toggle_key(t);
                        }
                        let lab = ui.add(egui::Label::new(RichText::new(*label).size(12.5).color(if p.animated() { INK } else { MUTED })).sense(Sense::click()));
                        lab.context_menu(|ui| {
                            if ui.button("Réinitialiser la valeur").clicked() {
                                p.set(t, PROP_DEFAULT[pi]);
                                ui.close();
                            }
                            if ui.add_enabled(p.animated(), egui::Button::new("Supprimer l'animation")).clicked() {
                                p.clear(t);
                                ui.close();
                            }
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if p.animated() {
                                if ui.small_button("›").on_hover_text("Clé suivante").clicked() {
                                    go = p.keys.iter().map(|k| k.t).find(|k| *k > t + 1e-3);
                                }
                            }
                            let mut v = p.at(t);
                            let speed = match pi {
                                2 | 4 | 7 => 0.5,
                                3 | 5 => 0.5,
                                6 => 0.2,
                                _ => 1.0,
                            };
                            let mut dv = egui::DragValue::new(&mut v).speed(speed).max_decimals(1);
                            if matches!(pi, 4 | 7) {
                                dv = dv.range(0.0..=100.0);
                            }
                            if pi == 6 {
                                dv = dv.range(0.0..=200.0);
                            }
                            if ui.add(dv).changed() {
                                p.set(t, v);
                            }
                            if p.animated() {
                                if ui.small_button("‹").on_hover_text("Clé précédente").clicked() {
                                    go = p.keys.iter().rev().map(|k| k.t).find(|k| *k < t - 1e-3);
                                }
                            }
                        });
                    });
                    if let Some(k) = p.key_at(t) {
                        ui.horizontal(|ui| {
                            ui.add_space(24.0);
                            ui.label(RichText::new("Courbe").size(11.5).color(FAINT));
                            ease_combo(ui, ("ease", pi), &mut p.keys[k]);
                        });
                    }
                }
            });
        }
        if let Some(g) = go {
            self.t = g;
        }
        pro::dock_panel(ui, "align", "Aligner sur la composition", |ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            let w = (ui.available_width() - 8.0) / 3.0;
            let mut how = None;
            for row in [[("Gauche", 0u8), ("Centre ↔", 1), ("Droite", 2)], [("Haut", 3), ("Centre ↕", 4), ("Bas", 5)]] {
                ui.horizontal(|ui| {
                    for (label, k) in row {
                        if ui.add_sized([w, 26.0], egui::Button::new(RichText::new(label).size(12.0))).clicked() {
                            how = Some(k);
                        }
                    }
                });
            }
            if ui.add_sized([ui.available_width(), 26.0], egui::Button::new(RichText::new("Centrer au milieu").size(12.0))).clicked() {
                how = Some(6);
            }
            if let Some(h) = how {
                self.align(h);
            }
        });
        let l = &mut self.comp.layers[i];
        pro::dock_panel(ui, "content", "Contenu", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            content_editor(ui, &mut l.content);
        });
        let mut help_open = false;
        let is_text = matches!(l.content, Content::Text { .. });
        let fx = &mut l.fx;
        pro::dock_panel(ui, "look", "Apparence", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            let mut on = fx.stroke.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut on, "Contour").changed() {
                    fx.stroke = on.then_some([255, 255, 255, 255]);
                    if fx.stroke_w <= 0.0 {
                        fx.stroke_w = 4.0;
                    }
                }
                if let Some(c) = &mut fx.stroke {
                    color_button(ui, c);
                }
            });
            if fx.stroke.is_some() {
                slider(ui, "Épaisseur", &mut fx.stroke_w, 0.5..=60.0, " px");
                slider(ui, "Pointillés (0 = plein)", &mut fx.dash, 0.0..=120.0, " px");
            }
            let mut grad = fx.gradient.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut grad, "Dégradé vers").changed() {
                    fx.gradient = grad.then_some([10, 132, 255, 255]);
                }
                if let Some(c) = &mut fx.gradient {
                    color_button(ui, c);
                }
            });
            if fx.gradient.is_some() {
                slider(ui, "Angle du dégradé", &mut fx.gradient_angle, -180.0..=180.0, "°");
            }
            ui.label(RichText::new("Mode de fusion").size(12.5).color(MUTED));
            egui::ComboBox::from_id_salt("blend").selected_text(BLENDS.get(fx.blend as usize).map_or("Normal", |b| b.1)).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                for (k, (_, name)) in BLENDS.iter().enumerate() {
                    ui.selectable_value(&mut fx.blend, k as u8, *name);
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut fx.flip_x, "Miroir horizontal");
                ui.checkbox(&mut fx.flip_y, "Vertical");
            });
        });
        pro::dock_panel(ui, "fx", "Effets", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            slider(ui, "Flou", &mut fx.blur, 0.0..=80.0, " px");
            ui.horizontal(|ui| {
                ui.checkbox(&mut fx.shadow, "Ombre portée");
                if fx.shadow {
                    color_button(ui, &mut fx.shadow_color);
                }
            });
            if fx.shadow {
                slider(ui, "Décalage X", &mut fx.shadow_dx, -100.0..=100.0, " px");
                slider(ui, "Décalage Y", &mut fx.shadow_dy, -100.0..=100.0, " px");
                slider(ui, "Douceur", &mut fx.shadow_blur, 0.0..=80.0, " px");
                slider(ui, "Opacité", &mut fx.shadow_alpha, 0.0..=100.0, " %");
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("Lueur").size(12.5).color(MUTED));
                color_button(ui, &mut fx.glow_color);
            });
            ui.add(egui::Slider::new(&mut fx.glow, 0.0..=80.0).suffix(" px"));
            ui.label(RichText::new("Couleur").size(12.5).color(INK));
            slider(ui, "Teinte", &mut fx.hue, -180.0..=180.0, "°");
            slider(ui, "Saturation", &mut fx.saturation, 0.0..=300.0, " %");
            slider(ui, "Luminosité", &mut fx.brightness, 0.0..=300.0, " %");
        });
        pro::dock_panel(ui, "motion", "Mouvement", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            if is_text {
                slider(ui, "Machine à écrire (0 = non)", &mut fx.typewriter, 0.0..=10.0, " s");
            }
            slider(ui, "Tremblement", &mut fx.wiggle, 0.0..=200.0, " px");
            if fx.wiggle > 0.0 {
                if fx.wiggle_freq <= 0.0 {
                    fx.wiggle_freq = 2.0;
                }
                slider(ui, "Fréquence", &mut fx.wiggle_freq, 0.1..=12.0, " /s");
            }
            ui.label(RichText::new("Répéter les images clés").size(12.5).color(MUTED));
            let mut mode = if fx.pingpong { 2 } else if fx.loop_keys { 1 } else { 0 };
            if pro::segmented(ui, &mut mode, &["Non", "Boucle", "Aller-retour"]) {
                fx.loop_keys = mode == 1;
                fx.pingpong = mode == 2;
            }
            ui.add_space(4.0);
            let mut echo = fx.echo as f32;
            slider(ui, "Écho (traînée)", &mut echo, 0.0..=12.0, " copies");
            fx.echo = echo.round() as u32;
            if fx.echo > 0 {
                slider(ui, "Délai entre échos", &mut fx.echo_delay, 0.01..=0.5, " s");
            }
            let mut rep = fx.repeat as f32;
            slider(ui, "Répéteur", &mut rep, 0.0..=40.0, " copies");
            fx.repeat = rep.round() as u32;
            if fx.repeat > 0 {
                slider(ui, "Décalage X", &mut fx.rep_dx, -1000.0..=1000.0, " px");
                slider(ui, "Décalage Y", &mut fx.rep_dy, -1000.0..=1000.0, " px");
                slider(ui, "Rotation par copie", &mut fx.rep_rot, -180.0..=180.0, "°");
                slider(ui, "Échelle par copie", &mut fx.rep_scale, 20.0..=200.0, " %");
                slider(ui, "Fondu par copie", &mut fx.rep_fade, 0.0..=100.0, " %");
            }
        });
        pro::dock_panel(ui, "matte", "Masque", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            egui::ComboBox::from_id_salt("matte").selected_text(MATTES[(fx.matte as usize).min(2)]).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                for (k, n) in MATTES.iter().enumerate() {
                    ui.selectable_value(&mut fx.matte, k as u8, *n);
                }
            });
            ui.label(RichText::new("Le calque juste au-dessus dans la timeline sert de pochoir : il devient invisible et ne laisse voir ce calque qu'à travers sa forme.").size(11.5).color(FAINT));
            ui.add_space(4.0);
            if pro::secondary(ui, "?  Tutoriels").clicked() {
                help_open = true;
            }
        });
        if help_open {
            self.help.learn("Effets");
        }
    }

    fn sheets(&mut self, ctx: &egui::Context) {
        if let Some(mut o) = self.export.take() {
            let mut go = false;
            let mut cancel = false;
            let keep = pro::sheet(ctx, "export", 460.0, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(9), BLUE.gamma_multiply(0.25));
                    glyph(ui.painter(), r.shrink(9.0), G::Export, BLUE);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Exporter l'animation").font(semibold(18.0)).color(INK_STRONG));
                        ui.label(RichText::new(["Vidéo H.264, lisible partout", "Animation légère pour le web et les messages", "Une image par frame, idéal pour le montage", "L'image affichée à la tête de lecture", "Vectoriel, l'image affichée, modifiable"][o.format]).size(12.0).color(MUTED));
                    });
                });
                ui.add_space(14.0);
                let row = |ui: &mut Ui, s: &str| {
                    ui.label(RichText::new(s).size(12.5).color(MUTED));
                };
                row(ui, "Format");
                pro::segmented(ui, &mut o.format, &FORMATS);
                ui.add_space(8.0);
                if o.format != 4 {
                    row(ui, "Taille");
                    let labels: Vec<&str> = SCALES.iter().map(|s| s.0).collect();
                    pro::segmented(ui, &mut o.scale, &labels);
                    let (w, h) = out_size(&self.comp, SCALES[o.scale].1, o.format == 0);
                    ui.label(RichText::new(format!("{w} × {h} px")).size(11.5).color(FAINT));
                    ui.add_space(8.0);
                }
                if o.format <= 2 {
                    row(ui, "Plage");
                    ui.add_enabled_ui(self.comp.work.is_some(), |ui| {
                        pro::segmented(ui, &mut o.range, &["Toute la composition", "Zone de travail"]);
                    });
                    if self.comp.work.is_none() {
                        o.range = 0;
                    }
                    ui.add_space(8.0);
                }
                if o.format == 1 {
                    row(ui, "Images par seconde");
                    let labels: Vec<&str> = GIF_FPS.iter().map(|s| s.0).collect();
                    pro::segmented(ui, &mut o.gif_fps, &labels);
                    ui.add_space(8.0);
                }
                if o.format != 0 {
                    ui.checkbox(&mut o.transparent, "Fond transparent");
                } else {
                    o.transparent = false;
                }
                ui.add_space(10.0);
                let (a, b) = if o.range == 1 { self.comp.range() } else { (0.0, self.comp.duration) };
                if o.format <= 2 {
                    let n = if o.range == 1 { ((b - a) * self.comp.fps as f32).round() as u32 } else { self.comp.frames() };
                    ui.label(RichText::new(format!("{} · {n} images", secs(b - a))).size(12.5).color(FAINT));
                }
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
                self.run_export(&o);
            } else if keep && !cancel {
                self.export = Some(o);
            }
        }
        if self.comp_sheet {
            let mut done = false;
            let keep = pro::sheet(ctx, "comp", 470.0, |ui| {
                ui.label(RichText::new("Réglages de la composition").font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(12.0);
                ui.label(RichText::new("Format").size(12.5).color(MUTED));
                ui.horizontal_wrapped(|ui| {
                    for (name, _, w, h) in PRESETS {
                        let on = self.comp.width == w && self.comp.height == h;
                        if ui.selectable_label(on, name).clicked() {
                            self.comp.width = w;
                            self.comp.height = h;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.comp.width).range(16..=7680).prefix("L ").suffix(" px"));
                    ui.label(RichText::new("×").color(MUTED));
                    ui.add(egui::DragValue::new(&mut self.comp.height).range(16..=4320).prefix("H ").suffix(" px"));
                    if ui.small_button("⇄").on_hover_text("Inverser largeur et hauteur").clicked() {
                        std::mem::swap(&mut self.comp.width, &mut self.comp.height);
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Images par seconde").size(12.5).color(MUTED));
                ui.horizontal(|ui| {
                    for f in FPS_CHOICES {
                        if ui.selectable_label(self.comp.fps == f, f.to_string()).clicked() {
                            self.comp.fps = f;
                        }
                    }
                    ui.add(egui::DragValue::new(&mut self.comp.fps).range(1..=120));
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Durée").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut self.comp.duration).range(0.1..=600.0).speed(0.1).suffix(" s"));
                    if ui.button("Ajuster à l'animation").clicked() {
                        self.fit_duration();
                    }
                });
                ui.add_space(8.0);
                bg_editor(ui, &mut self.comp);
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Terminé").clicked() {
                        done = true;
                    }
                });
            });
            if done || !keep {
                self.comp_sheet = false;
            }
        }
    }

    fn viewer(&mut self, root: &mut Ui, name: &str) {
        egui::CentralPanel::default().frame(Frame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let bar_h = 40.0;
            let bar = Rect::from_min_size(full.min, vec2(full.width(), bar_h));
            ui.painter().rect_filled(bar, CornerRadius::ZERO, Color32::from_rgb(0x1c, 0x1c, 0x20));
            ui.painter().line_segment([bar.left_bottom(), bar.right_bottom()], Stroke::new(1.0, LINE));
            ui.scope_builder(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(10.0, 0.0))), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (tool, g, tip) in [(Tool::Select, G::Select, "Sélection (V) : déplace, coins = échelle, rond = rotation"), (Tool::Rotate, G::Rotate, "Rotation (W)"), (Tool::Hand, G::Hand, "Main (H) : déplace la vue")] {
                        if pro::icon_button(ui, 30.0, self.tool == tool, tip, |p, r, c| glyph(p, r, g, c)).clicked() {
                            self.tool = tool;
                        }
                    }
                    pro::toolbar_sep(ui);
                    let mut z = match self.zoom {
                        None => 0,
                        Some(v) if (v - 0.5).abs() < 1e-3 => 1,
                        Some(v) if (v - 1.0).abs() < 1e-3 => 2,
                        _ => 3,
                    };
                    let labels: &[&str] = if z == 3 { &["Ajuster", "50 %", "100 %", "Autre"] } else { &["Ajuster", "50 %", "100 %"] };
                    if pro::segmented(ui, &mut z, labels) {
                        self.zoom = [None, Some(0.5), Some(1.0), self.zoom][z];
                        self.pan = Vec2::ZERO;
                    }
                    pro::toolbar_sep(ui);
                    for (on, g, tip) in [(&mut self.grid, G::Grid, "Grille et tiers"), (&mut self.safe, G::Safe, "Zones sûres"), (&mut self.guides, G::Guides, "Repères centraux"), (&mut self.snap, G::Magnet, "Magnétisme"), (&mut self.half, G::Half, "Aperçu en demi-résolution (plus fluide)")] {
                        if pro::icon_button(ui, 30.0, *on, tip, |p, r, c| glyph(p, r, g, c)).clicked() {
                            *on = !*on;
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(short(name, 28)).size(12.5).color(MUTED));
                    });
                });
            });

            let controls_h = 54.0;
            let canvas = Rect::from_min_max(pos2(full.left(), bar.bottom()), pos2(full.right(), full.bottom() - controls_h));
            let area = canvas.shrink(22.0);
            let (cw, ch) = (self.comp.width as f32, self.comp.height as f32);
            let ppp = ui.ctx().pixels_per_point();
            let fit = (area.width() / cw).min(area.height() / ch).max(0.01);
            let s = self.zoom.map_or(fit, |z| z / ppp);
            let view = Rect::from_center_size(area.center() + if self.zoom.is_some() { self.pan } else { Vec2::ZERO }, vec2(cw, ch) * s);
            let rs = (s * ppp * if self.half { 0.5 } else { 1.0 }).min(2.0);
            let (rw, rh) = (((cw * rs).round() as u32).max(1), ((ch * rs).round() as u32).max(1));
            let t = self.t;
            let stale = self.rendered.as_ref().is_none_or(|(c, rt, w)| *c != self.comp || (*rt - t).abs() > 1e-4 || *w != rw);
            if stale {
                match render(&self.comp, t, rw, rh, &self.fontdb, false) {
                    Ok(pix) => {
                        let img = egui::ColorImage::from_rgba_premultiplied([pix.width() as usize, pix.height() as usize], pix.data());
                        match &mut self.tex {
                            Some(tx) => tx.set(img, TextureOptions::LINEAR),
                            None => self.tex = Some(ui.ctx().load_texture("frame", img, TextureOptions::LINEAR)),
                        }
                        self.rendered = Some((self.comp.clone(), t, rw));
                    }
                    Err(e) => self.error = Some(format!("Rendu : {e}")),
                }
            }
            let painter = ui.painter_at(canvas);
            painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(140) }.as_shape(view, CornerRadius::same(4)));
            if let Some(tx) = &self.tex {
                painter.image(tx.id(), view, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            let to_scr = |x: f32, y: f32| pos2(view.min.x + x * s, view.min.y + y * s);
            let thin = Stroke::new(1.0, Color32::from_white_alpha(46));
            if self.grid {
                for k in 1..10 {
                    let f = k as f32 / 10.0;
                    painter.line_segment([to_scr(cw * f, 0.0), to_scr(cw * f, ch)], Stroke::new(1.0, Color32::from_white_alpha(16)));
                    painter.line_segment([to_scr(0.0, ch * f), to_scr(cw, ch * f)], Stroke::new(1.0, Color32::from_white_alpha(16)));
                }
                for f in [1.0 / 3.0, 2.0 / 3.0] {
                    painter.line_segment([to_scr(cw * f, 0.0), to_scr(cw * f, ch)], thin);
                    painter.line_segment([to_scr(0.0, ch * f), to_scr(cw, ch * f)], thin);
                }
            }
            if self.safe {
                for (k, a) in [(0.9, 70), (0.8, 110)] {
                    painter.rect_stroke(Rect::from_center_size(view.center(), view.size() * k), CornerRadius::ZERO, Stroke::new(1.0, Color32::from_white_alpha(a)), StrokeKind::Middle);
                }
            }
            if self.guides {
                painter.line_segment([to_scr(cw / 2.0, 0.0), to_scr(cw / 2.0, ch)], Stroke::new(1.0, MARK.gamma_multiply(0.6)));
                painter.line_segment([to_scr(0.0, ch / 2.0), to_scr(cw, ch / 2.0)], Stroke::new(1.0, MARK.gamma_multiply(0.6)));
            }

            let geo = |me: &Self, i: usize| -> Option<(Pos2, f32, f32, f32, f32)> {
                let l = me.comp.layers.get(i)?;
                if !l.active(t) {
                    return None;
                }
                let (x, y, ws, wr) = me.comp.world(i, t);
                let (w, h) = l.size();
                Some((to_scr(x, y), w * ws * s / 2.0, h * ws * s / 2.0, wr, ws))
            };
            let corners = |c: Pos2, hw: f32, hh: f32, r: f32| -> [Pos2; 4] {
                [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(a, b)| {
                    let (dx, dy) = rot(a * hw, b * hh, r);
                    c + vec2(dx, dy)
                })
            };
            let rot_handle = |c: Pos2, hh: f32, r: f32| {
                let (dx, dy) = rot(0.0, -hh - 26.0, r);
                c + vec2(dx, dy)
            };
            let sel_geo = self.sel.and_then(|i| geo(self, i));
            if let Some((c, hw, hh, r, _)) = sel_geo {
                let q = corners(c, hw, hh, r);
                painter.add(Shape::closed_line(q.to_vec(), Stroke::new(1.0, BLUE)));
                for p in q {
                    painter.rect(Rect::from_center_size(p, Vec2::splat(8.0)), CornerRadius::same(2), Color32::WHITE, Stroke::new(1.0, BLUE), StrokeKind::Inside);
                }
                let rh = rot_handle(c, hh, r);
                let top = {
                    let (dx, dy) = rot(0.0, -hh, r);
                    c + vec2(dx, dy)
                };
                painter.line_segment([top, rh], Stroke::new(1.0, BLUE));
                painter.circle(rh, 5.0, Color32::WHITE, Stroke::new(1.0, BLUE));
                painter.circle(c, 3.5, Color32::WHITE, Stroke::new(1.0, BLUE));
                if let Some(p) = self.sel.and_then(|i| self.comp.layers[i].parent)
                    && let Some((pc, ..)) = geo(self, p)
                {
                    painter.line_segment([c, pc], Stroke::new(1.0, Color32::from_white_alpha(60)));
                }
            }

            let resp = ui.interact(canvas, ui.make_persistent_id("view"), Sense::click_and_drag());
            if resp.hovered() {
                let (scroll, cmd) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.command));
                if cmd && scroll != 0.0 {
                    let z = (self.zoom.unwrap_or(fit * ppp) * (1.0 + scroll * 0.002)).clamp(0.05, 8.0);
                    self.zoom = Some(z);
                }
                ui.ctx().set_cursor_icon(match self.tool {
                    Tool::Hand => egui::CursorIcon::Grab,
                    Tool::Rotate => egui::CursorIcon::Alias,
                    Tool::Select => egui::CursorIcon::Default,
                });
            }
            let middle = ui.input(|i| i.pointer.middle_down());
            if resp.drag_started()
                && let Some(p) = resp.interact_pointer_pos()
            {
                self.xdrag = None;
                if self.tool == Tool::Hand || middle {
                    if self.zoom.is_none() {
                        self.zoom = Some(fit * ppp);
                    }
                    self.xdrag = Some(XDrag::Pan { p0: self.pan, from: p });
                } else if let (Some(i), Some((c, hw, hh, r, ws))) = (self.sel, sel_geo)
                    && !self.comp.layers[i].locked
                {
                    let near_corner = corners(c, hw, hh, r).iter().any(|q| q.distance(p) < 9.0);
                    if self.tool == Tool::Rotate || rot_handle(c, hh, r).distance(p) < 9.0 {
                        let a0 = (p - c).angle().to_degrees();
                        self.xdrag = Some(XDrag::Rotate { r0: self.comp.layers[i].value(3, t), a0 });
                    } else if near_corner {
                        self.xdrag = Some(XDrag::Scale { ws, d0: c.distance(p).max(1.0) });
                    }
                }
                if self.xdrag.is_none() && self.tool == Tool::Select {
                    let (px, py) = ((p.x - view.min.x) / s, (p.y - view.min.y) / s);
                    let solo = self.comp.layers.iter().any(|l| l.solo);
                    let hit = (0..self.comp.layers.len()).rev().find(|&i| {
                        let l = &self.comp.layers[i];
                        if !l.active(t) || l.locked || (solo && !l.solo) || (i > 0 && self.comp.layers[i - 1].fx.matte > 0) {
                            return false;
                        }
                        let (x, y, ws, wr) = self.comp.world(i, t);
                        let (w, h) = l.size();
                        let (lx, ly) = rot(px - x, py - y, -wr);
                        lx.abs() <= w * ws.abs() / 2.0 + 4.0 / s && ly.abs() <= h * ws.abs() / 2.0 + 4.0 / s
                    });
                    if let Some(i) = hit {
                        self.sel = Some(i);
                        let (wx, wy, _, _) = self.comp.world(i, t);
                        self.xdrag = Some(XDrag::Move { wx, wy, from: p });
                    }
                }
            }
            let (shift, ptr) = ui.input(|i| (i.modifiers.shift, i.pointer.interact_pos()));
            if resp.dragged()
                && let (Some(d), Some(p)) = (self.xdrag, ptr)
            {
                match (d, self.sel) {
                    (XDrag::Pan { p0, from }, _) => self.pan = p0 + (p - from),
                    (XDrag::Move { wx, wy, from }, Some(i)) => {
                        let mut dx = (p.x - from.x) / s;
                        let mut dy = (p.y - from.y) / s;
                        if shift {
                            if dx.abs() > dy.abs() {
                                dy = 0.0;
                            } else {
                                dx = 0.0;
                            }
                        }
                        let (mut nx, mut ny) = (wx + dx, wy + dy);
                        if self.snap {
                            let (_, _, ws, _) = self.comp.world(i, t);
                            let (w, h) = self.comp.layers[i].size();
                            let tol = 8.0 / s;
                            for c in [cw / 2.0, w * ws / 2.0, cw - w * ws / 2.0] {
                                if (nx - c).abs() < tol {
                                    nx = c;
                                    painter.line_segment([to_scr(c, 0.0), to_scr(c, ch)], Stroke::new(1.0, Color32::from_rgb(255, 64, 160)));
                                    break;
                                }
                            }
                            for c in [ch / 2.0, h * ws / 2.0, ch - h * ws / 2.0] {
                                if (ny - c).abs() < tol {
                                    ny = c;
                                    painter.line_segment([to_scr(0.0, c), to_scr(cw, c)], Stroke::new(1.0, Color32::from_rgb(255, 64, 160)));
                                    break;
                                }
                            }
                        }
                        self.place(i, nx, ny);
                    }
                    (XDrag::Scale { ws, d0 }, Some(i)) => {
                        if let Some((c, ..)) = sel_geo {
                            let k = c.distance(p) / d0;
                            let (_, _, ps, _) = self.parent_frame(i, t);
                            let v = (ws * k / ps.max(1e-4) * 100.0 * 10.0).round() / 10.0;
                            self.comp.layers[i].props[2].set(t, v.max(0.1));
                        }
                    }
                    (XDrag::Rotate { r0, a0 }, Some(i)) => {
                        if let Some((c, ..)) = sel_geo {
                            let a = (p - c).angle().to_degrees();
                            let mut d = a - a0;
                            if d > 180.0 {
                                d -= 360.0;
                            }
                            if d < -180.0 {
                                d += 360.0;
                            }
                            let mut v = r0 + d;
                            v = if shift { (v / 15.0).round() * 15.0 } else { (v * 10.0).round() / 10.0 };
                            self.comp.layers[i].props[3].set(t, v);
                        }
                    }
                    _ => {}
                }
            }
            if resp.drag_stopped() {
                self.xdrag = None;
            }
            if resp.clicked() && self.tool == Tool::Select {
                self.sel = None;
            }
            if resp.double_clicked() {
                self.zoom = None;
                self.pan = Vec2::ZERO;
            }

            let tb = Rect::from_min_max(pos2(full.left() + 18.0, full.bottom() - controls_h + 6.0), pos2(full.right() - 18.0, full.bottom() - 6.0));
            ui.painter().line_segment([pos2(full.left(), tb.top() - 6.0), pos2(full.right(), tb.top() - 6.0)], Stroke::new(1.0, LINE));
            let fps = self.comp.fps;
            ui.painter().text(tb.left_center(), Align2::LEFT_CENTER, tc(self.t, fps), FontId::monospace(16.0), BLUE);
            ui.painter().text(tb.left_center() + vec2(104.0, 0.0), Align2::LEFT_CENTER, format!("image {}", (self.t * fps as f32).round() as u32), FontId::proportional(11.5), FAINT);
            let (a, b) = self.comp.range();
            ui.painter().text(tb.right_center(), Align2::RIGHT_CENTER, if self.comp.work.is_some() { format!("zone {} → {}", tc(a, fps), tc(b, fps)) } else { tc(self.comp.duration, fps) }, FontId::monospace(13.0), MUTED);
            let step = 1.0 / fps as f32;
            for (dx, kind, tip) in [(-150.0, 0u8, "Début (Origine)"), (-108.0, 5, "Image clé précédente (J)"), (-66.0, 1, "Image précédente (←)"), (0.0, 2, "Lecture / pause (Espace)"), (66.0, 3, "Image suivante (→)"), (108.0, 6, "Image clé suivante (K)"), (150.0, 4, "Fin"), (200.0, 7, "Lecture en boucle (L)")] {
                let size = if kind == 2 { 40.0 } else { 32.0 };
                let r = Rect::from_center_size(tb.center() + vec2(dx, 0.0), Vec2::splat(size));
                let bt = ui.interact(r, ui.make_persistent_id(("transport", kind)), Sense::click()).on_hover_text(tip);
                let on = kind == 7 && self.loop_play;
                let fill = if kind == 2 {
                    if bt.hovered() { BLUE.lerp_to_gamma(Color32::WHITE, 0.12) } else { BLUE }
                } else if on {
                    BLUE.gamma_multiply(0.28)
                } else if bt.hovered() {
                    FIELD
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().circle_filled(r.center(), size / 2.0, fill);
                match kind {
                    5 | 6 => {
                        let dir = if kind == 6 { 1.0 } else { -1.0 };
                        diamond(ui.painter(), r.center() + vec2(3.0 * dir, 0.0), 5.0, KEY);
                        ui.painter().add(Shape::convex_polygon(vec![r.center() + vec2(-6.0 * dir, -4.5), r.center() + vec2(-2.0 * dir, 0.0), r.center() + vec2(-6.0 * dir, 4.5)], INK, Stroke::NONE));
                    }
                    7 => glyph(ui.painter(), r.shrink(8.0), G::Loop, if on { INK_STRONG } else { INK }),
                    _ => transport_glyph(ui.painter(), r.center(), kind, self.playing, if kind == 2 { Color32::WHITE } else { INK }),
                }
                if bt.clicked() {
                    match kind {
                        0 => self.t = a,
                        1 => self.t = self.snap_frame((self.t - step).max(0.0)),
                        2 => self.playing = !self.playing,
                        3 => self.t = self.snap_frame((self.t + step).min(self.comp.duration)),
                        4 => self.t = b,
                        5 => self.t = self.comp.next_key(self.t, false, None).unwrap_or(self.t),
                        6 => self.t = self.comp.next_key(self.t, true, None).unwrap_or(self.t),
                        _ => self.loop_play = !self.loop_play,
                    }
                }
            }
        });
    }

    fn timeline(&mut self, root: &mut Ui) {
        egui::Panel::bottom("timeline").resizable(true).default_size(300.0).min_size(200.0).frame(Frame::new().fill(PANEL).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let full = ui.max_rect();
            let head = Rect::from_min_size(full.min, vec2(full.width(), 38.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(head.shrink2(vec2(12.0, 4.0))), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("Timeline").font(semibold(13.0)).color(INK));
                    ui.add_space(10.0);
                    if pro::icon_button(ui, 28.0, false, "Ajouter un repère (M)", |p, r, c| glyph(p, r, G::Marker, c)).clicked() {
                        self.add_marker();
                    }
                    if pro::icon_button(ui, 28.0, self.snap, "Magnétisme : accroche aux clés, repères et à la tête", |p, r, c| glyph(p, r, G::Magnet, c)).clicked() {
                        self.snap = !self.snap;
                    }
                    pro::toolbar_sep(ui);
                    if ui.small_button("[ Début zone").on_hover_text("B").clicked() {
                        self.set_work(true);
                    }
                    if ui.small_button("Fin zone ]").on_hover_text("N").clicked() {
                        self.set_work(false);
                    }
                    if self.comp.work.is_some() && ui.small_button("×").on_hover_text("Effacer la zone de travail").clicked() {
                        self.comp.work = None;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Ajuster").on_hover_text("Toute la durée visible").clicked() {
                            self.pps = ((ui.max_rect().width() + 300.0) / self.comp.duration.max(0.1)).clamp(30.0, 800.0);
                        }
                        ui.spacing_mut().slider_width = 130.0;
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
        let label_w = 236.0;
        let row_h = 30.0;
        let sub_h = 22.0;
        let np = PROPS.len() as f32;
        let dur = self.comp.duration.max(0.1);
        let n_rows: f32 = self.comp.layers.iter().enumerate().map(|(i, _)| row_h + if self.expanded.get(i).copied().unwrap_or(false) { sub_h * np } else { 0.0 }).sum();
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
        if let Some(_) = self.comp.work {
            let (a, b) = self.comp.range();
            p.rect_filled(Rect::from_min_max(pos2(x_of(a), ruler.min.y + 16.0), pos2(x_of(b), ruler.max.y)), CornerRadius::same(2), BLUE.gamma_multiply(0.55));
            p.rect_filled(Rect::from_min_max(pos2(x_of(0.0), ruler.max.y), pos2(x_of(a), rect.max.y)), CornerRadius::ZERO, Color32::from_black_alpha(40));
            p.rect_filled(Rect::from_min_max(pos2(x_of(b), ruler.max.y), pos2(x_of(dur), rect.max.y)), CornerRadius::ZERO, Color32::from_black_alpha(40));
        }
        let step = [0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0].into_iter().find(|s| s * pps >= 70.0).unwrap_or(60.0);
        let mut k = 0u32;
        loop {
            let tt = k as f32 * step;
            if tt > dur + 1e-3 {
                break;
            }
            let x = x_of(tt);
            p.line_segment([pos2(x, ruler.min.y + 14.0), pos2(x, ruler.max.y)], Stroke::new(1.0, FAINT));
            p.text(pos2(x + 3.0, ruler.min.y + 2.0), Align2::LEFT_TOP, secs(tt), FontId::monospace(10.0), MUTED);
            k += 1;
        }
        let r = ui.interact(ruler, ui.make_persistent_id("ruler"), Sense::click_and_drag());
        if let Some(pp) = r.interact_pointer_pos()
            && (r.dragged() || r.clicked())
        {
            self.t = self.snap_time(t_of(pp.x), None, None);
            self.playing = false;
        }
        let ruler_t = r.hover_pos().map(|pp| t_of(pp.x));
        r.context_menu(|ui| {
            if ui.button("Ajouter un repère ici").clicked() {
                if let Some(t) = ruler_t {
                    self.t = self.snap_frame(t);
                }
                self.add_marker();
                ui.close();
            }
            if ui.button("Début de zone à la tête").clicked() {
                self.set_work(true);
                ui.close();
            }
            if ui.button("Fin de zone à la tête").clicked() {
                self.set_work(false);
                ui.close();
            }
            if ui.add_enabled(self.comp.work.is_some(), egui::Button::new("Effacer la zone")).clicked() {
                self.comp.work = None;
                ui.close();
            }
        });
        let mut drop_marker = None;
        for (mi, m) in self.comp.markers.iter().enumerate() {
            let x = x_of(m.t);
            p.add(Shape::convex_polygon(vec![pos2(x - 5.0, ruler.min.y + 1.0), pos2(x + 5.0, ruler.min.y + 1.0), pos2(x + 5.0, ruler.min.y + 8.0), pos2(x, ruler.min.y + 13.0), pos2(x - 5.0, ruler.min.y + 8.0)], MARK, Stroke::NONE));
            p.line_segment([pos2(x, ruler.max.y), pos2(x, rect.max.y)], Stroke::new(1.0, MARK.gamma_multiply(0.25)));
            let mr = ui.interact(Rect::from_center_size(pos2(x, ruler.min.y + 7.0), vec2(12.0, 14.0)), ui.make_persistent_id(("marker", mi)), Sense::click()).on_hover_text(&m.name);
            if mr.clicked() {
                self.t = m.t;
            }
            mr.context_menu(|ui| {
                if ui.button("Supprimer ce repère").clicked() {
                    drop_marker = Some(mi);
                    ui.close();
                }
            });
        }
        if let Some(mi) = drop_marker {
            self.comp.markers.remove(mi);
        }

        let mut y = ruler.max.y + 4.0;
        let mut action: Option<(usize, u8)> = None;
        for li in (0..self.comp.layers.len()).rev() {
            let l = &self.comp.layers[li];
            let color = LABELS[l.label.map_or(li % LABELS.len(), |c| c as usize % LABELS.len())];
            let on = self.sel == Some(li);
            let expanded = self.expanded.get(li).copied().unwrap_or(false);
            let row = Rect::from_min_max(pos2(rect.min.x, y), pos2(rect.max.x, y + row_h));
            p.rect_filled(row.shrink2(vec2(0.0, 1.0)), CornerRadius::ZERO, if on { Color32::from_rgb(0x2a, 0x2c, 0x36) } else { Color32::from_rgb(0x22, 0x22, 0x26) });
            let tw = Rect::from_center_size(pos2(rect.min.x + 12.0, row.center().y), Vec2::splat(16.0));
            let tri = if expanded { vec![tw.center() + vec2(-4.5, -2.5), tw.center() + vec2(4.5, -2.5), tw.center() + vec2(0.0, 3.5)] } else { vec![tw.center() + vec2(-2.5, -4.5), tw.center() + vec2(3.5, 0.0), tw.center() + vec2(-2.5, 4.5)] };
            p.add(Shape::convex_polygon(tri, MUTED, Stroke::NONE));
            if ui.interact(tw.expand(3.0), ui.make_persistent_id(("twirl", li)), Sense::click()).clicked()
                && let Some(e) = self.expanded.get_mut(li)
            {
                *e = !*e;
            }
            for (k, (g, active, tip)) in [(G::Eye, l.visible, "Afficher / masquer"), (G::Lock, l.locked, "Verrouiller"), (G::Solo, l.solo, "Solo : n'afficher que ce calque")].into_iter().enumerate() {
                let r = Rect::from_center_size(pos2(rect.min.x + 32.0 + k as f32 * 20.0, row.center().y), Vec2::splat(18.0));
                let resp = ui.interact(r, ui.make_persistent_id(("sw", li, k)), Sense::click()).on_hover_text(tip);
                let shown = if k == 0 { active } else { active || resp.hovered() };
                let c = if k == 0 { if active { INK } else { FAINT } } else if active { KEY } else { FAINT };
                if shown || k == 0 {
                    glyph(&p, r.shrink(2.0), g, c);
                } else {
                    p.circle_filled(r.center(), 1.5, Color32::from_white_alpha(30));
                }
                if resp.clicked() {
                    action = Some((li, k as u8));
                }
            }
            p.rect_filled(Rect::from_min_size(pos2(rect.min.x + 92.0, row.center().y - 6.0), vec2(4.0, 12.0)), CornerRadius::same(2), color);
            let name_col = if l.visible { if on { INK_STRONG } else { INK } } else { FAINT };
            let parent_tag = l.parent.map(|_| "↳ ").unwrap_or("");
            p.text(pos2(rect.min.x + 102.0, row.center().y), Align2::LEFT_CENTER, format!("{parent_tag}{}", short(&l.name, 17)), if on { semibold(12.5) } else { FontId::proportional(12.5) }, name_col);
            let lab = ui.interact(Rect::from_min_max(pos2(rect.min.x + 90.0, row.min.y), pos2(x0, row.max.y)), ui.make_persistent_id(("label", li)), Sense::click());
            if lab.clicked() {
                self.sel = Some(li);
            }
            if lab.double_clicked()
                && let Some(e) = self.expanded.get_mut(li)
            {
                *e = !*e;
            }
            lab.context_menu(|ui| {
                self.sel = Some(li);
                self.layer_menu(ui, li);
            });
            let l = &self.comp.layers[li];
            let bar = Rect::from_min_max(pos2(x_of(l.start), row.min.y + 5.0), pos2(x_of(l.end), row.max.y - 5.0));
            p.rect_filled(bar, CornerRadius::same(5), color.gamma_multiply(if on { 0.6 } else { 0.38 }));
            if on {
                p.rect_stroke(bar, CornerRadius::same(5), Stroke::new(1.0, color), StrokeKind::Inside);
            }
            if l.locked {
                for k in 0..((bar.width() / 10.0) as i32) {
                    let x = bar.left() + k as f32 * 10.0;
                    p.line_segment([pos2(x, bar.bottom()), pos2((x + 8.0).min(bar.right()), bar.top())], Stroke::new(1.0, Color32::from_black_alpha(60)));
                }
            }
            let times = l.key_times();
            for kt in &times {
                diamond(&p, pos2(x_of(*kt), row.center().y), 4.5, KEY);
            }
            let locked = l.locked;
            let br = ui.interact(bar.expand2(vec2(4.0, 0.0)), ui.make_persistent_id(("bar", li)), Sense::click_and_drag());
            if br.hovered()
                && !locked
                && let Some(pp) = br.hover_pos()
                && ((pp.x - bar.min.x).abs() < 6.0 || (pp.x - bar.max.x).abs() < 6.0)
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if br.drag_started()
                && !locked
                && let Some(pp) = br.interact_pointer_pos()
            {
                let mode = if (pp.x - bar.min.x).abs() < 6.0 { 1 } else if (pp.x - bar.max.x).abs() < 6.0 { 2 } else { 0 };
                self.bar_drag = Some((li, mode, t_of(pp.x), self.comp.layers[li].clone()));
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
            br.context_menu(|ui| {
                self.sel = Some(li);
                self.layer_menu(ui, li);
            });
            y += row_h;
            if expanded {
                for (pi, name) in PROPS.iter().enumerate() {
                    let sub = Rect::from_min_max(pos2(rect.min.x, y), pos2(rect.max.x, y + sub_h));
                    p.rect_filled(sub, CornerRadius::ZERO, Color32::from_rgb(0x1c, 0x1c, 0x20));
                    let Some(prop) = self.comp.layers[li].props.get(pi) else { continue };
                    p.text(pos2(rect.min.x + 44.0, sub.center().y), Align2::LEFT_CENTER, *name, FontId::proportional(11.5), if prop.animated() { INK } else { FAINT });
                    p.text(pos2(x0 - 10.0, sub.center().y), Align2::RIGHT_CENTER, format!("{:.1}", prop.at(self.t)), FontId::monospace(11.0), if prop.animated() { KEY } else { FAINT });
                    if prop.keys.len() > 1 {
                        p.line_segment([pos2(x_of(prop.keys[0].t), sub.center().y), pos2(x_of(prop.keys[prop.keys.len() - 1].t), sub.center().y)], Stroke::new(1.0, KEY.gamma_multiply(0.35)));
                    }
                    let keys: Vec<model::Key> = prop.keys.clone();
                    for (ki, key) in keys.iter().enumerate() {
                        let c = pos2(x_of(key.t), sub.center().y);
                        let kr = Rect::from_center_size(c, Vec2::splat(14.0));
                        let resp = ui.interact(kr, ui.make_persistent_id(("key", li, pi, ki)), Sense::click_and_drag()).on_hover_text(format!("{} · {:.1} · {}\nGlisser pour déplacer · clic droit : courbe", secs(key.t), key.v, key.ease_name()));
                        let active = self.key_drag == Some((li, pi, ki));
                        key_shape(&p, c, if resp.hovered() || active { 6.5 } else { 5.0 }, if active { Color32::WHITE } else { KEY }, key);
                        if resp.clicked() {
                            self.t = key.t;
                            self.sel = Some(li);
                        }
                        if resp.drag_started() {
                            self.key_drag = Some((li, pi, ki));
                        }
                        resp.context_menu(|ui| {
                            if let Some(k) = self.comp.layers.get_mut(li).and_then(|l| l.props.get_mut(pi)).and_then(|p| p.keys.get_mut(ki)) {
                                for (e, n) in Ease::ALL {
                                    if ui.selectable_label(k.ease2 == Ease2::None && k.ease == e, n).clicked() {
                                        k.set_ease(e);
                                        ui.close();
                                    }
                                }
                                ui.separator();
                                for (e, n) in Ease2::ALL {
                                    if ui.selectable_label(k.ease2 == e, n).clicked() {
                                        k.ease2 = e;
                                        ui.close();
                                    }
                                }
                                ui.separator();
                            }
                            if ui.button("Supprimer cette clé").clicked() {
                                if let Some(pr) = self.comp.layers.get_mut(li).and_then(|l| l.props.get_mut(pi))
                                    && ki < pr.keys.len()
                                {
                                    let v = pr.keys[ki].v;
                                    pr.keys.remove(ki);
                                    if pr.keys.is_empty() {
                                        pr.base = v;
                                    }
                                }
                                ui.close();
                            }
                        });
                    }
                    y += sub_h;
                }
            }
        }
        if let Some((li, k)) = action
            && let Some(l) = self.comp.layers.get_mut(li)
        {
            match k {
                0 => l.visible = !l.visible,
                1 => l.locked = !l.locked,
                _ => l.solo = !l.solo,
            }
        }

        if let Some((li, pi, ki)) = self.key_drag {
            let (down, pos) = ui.input(|i| (i.pointer.primary_down(), i.pointer.hover_pos()));
            if let (true, Some(pp)) = (down, pos) {
                let nt = self.snap_time(t_of(pp.x), None, Some((li, pi)));
                if let Some(k) = self.comp.layers.get_mut(li).and_then(|l| l.props.get_mut(pi)).and_then(|p| p.keys.get_mut(ki)) {
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

        if let Some((li, mode, from, orig)) = self.bar_drag.clone() {
            if let Some(pp) = ui.input(|i| i.pointer.hover_pos())
                && ui.input(|i| i.pointer.primary_down())
                && li < self.comp.layers.len()
            {
                let d = t_of(pp.x) - from;
                let mut l = orig.clone();
                match mode {
                    1 => l.start = self.snap_time(orig.start + d, Some(li), None).clamp(0.0, orig.end - 0.05),
                    2 => l.end = self.snap_time(orig.end + d, Some(li), None).clamp(orig.start + 0.05, dur),
                    _ => {
                        let s = self.snap_time(orig.start + d, Some(li), None);
                        let e = self.snap_time(orig.end + d, Some(li), None);
                        let dd = if (s - (orig.start + d)).abs() <= (e - (orig.end + d)).abs() { s - orig.start } else { e - orig.end };
                        l.shift(dd.clamp(-orig.start, dur - orig.end));
                    }
                }
                self.comp.layers[li] = l;
            } else {
                self.bar_drag = None;
            }
        }

        let x = x_of(self.t.min(dur));
        p.line_segment([pos2(x, ruler.min.y), pos2(x, rect.max.y)], Stroke::new(1.5, PLAYHEAD));
        p.add(Shape::convex_polygon(vec![pos2(x - 7.0, ruler.min.y), pos2(x + 7.0, ruler.min.y), pos2(x + 7.0, ruler.min.y + 8.0), pos2(x, ruler.min.y + 14.0), pos2(x - 7.0, ruler.min.y + 8.0)], PLAYHEAD, Stroke::NONE));
    }

    fn layer_menu(&mut self, ui: &mut Ui, li: usize) {
        if ui.button("Dupliquer").clicked() {
            self.duplicate();
            ui.close();
        }
        if ui.button("Fractionner à la tête").clicked() {
            self.split();
            ui.close();
        }
        if ui.button("Copier l'animation").clicked() {
            self.copy_anim();
            ui.close();
        }
        if ui.add_enabled(self.clip_anim.is_some(), egui::Button::new("Coller l'animation")).clicked() {
            self.paste_anim();
            ui.close();
        }
        ui.separator();
        if ui.button("Premier plan").clicked() {
            self.move_layer(li, self.comp.layers.len() - 1);
            ui.close();
        }
        if ui.button("Arrière-plan").clicked() {
            self.move_layer(li, 0);
            ui.close();
        }
        ui.menu_button("Couleur d'étiquette", |ui| {
            for (k, n) in ["Bleu", "Rose", "Vert", "Orange", "Violet", "Cyan"].iter().enumerate() {
                if ui.button(*n).clicked() {
                    self.comp.layers[li].label = Some(k as u8);
                    ui.close();
                }
            }
        });
        ui.separator();
        if ui.button("Supprimer").clicked() {
            self.sel = Some(li);
            self.delete();
            ui.close();
        }
    }
}

const KINDS: [(&str, u8); 12] = [
    ("Texte", 0),
    ("Rectangle", 1),
    ("Ellipse", 2),
    ("Étoile", 4),
    ("Polygone", 5),
    ("Triangle", 8),
    ("Ligne", 6),
    ("Flèche", 9),
    ("Anneau", 10),
    ("Compteur", 7),
    ("Image…", 3),
    ("Fond", 11),
];

fn secs(t: f32) -> String {
    format!("{:.1} s", t).replace('.', ",")
}

fn slider(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) {
    ui.label(RichText::new(label).size(12.5).color(MUTED));
    ui.add(egui::Slider::new(v, range).suffix(suffix).max_decimals(2));
}

fn color_button(ui: &mut Ui, c: &mut [u8; 4]) {
    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    if ui.color_edit_button_srgba(&mut col).changed() {
        *c = col.to_srgba_unmultiplied();
    }
}

fn ease_combo(ui: &mut Ui, id: (&str, usize), k: &mut model::Key) {
    egui::ComboBox::from_id_salt(id).width(130.0).selected_text(k.ease_name()).show_ui(ui, |ui| {
        for (e, n) in Ease::ALL {
            if ui.selectable_label(k.ease2 == Ease2::None && k.ease == e, n).clicked() {
                k.set_ease(e);
            }
        }
        ui.separator();
        for (e, n) in Ease2::ALL {
            if ui.selectable_label(k.ease2 == e, n).clicked() {
                k.ease2 = e;
            }
        }
    });
}

fn bg_editor(ui: &mut Ui, comp: &mut Comp) {
    let mut bg = Color32::from_rgba_unmultiplied(comp.background[0], comp.background[1], comp.background[2], comp.background[3]);
    ui.label(RichText::new("Fond").size(12.5).color(MUTED));
    let w = ui.available_width();
    pro::color_picker(ui, &mut bg, w);
    comp.background = bg.to_srgba_unmultiplied();
    let mut grad = comp.background2.is_some();
    ui.horizontal(|ui| {
        if ui.checkbox(&mut grad, "Dégradé vers le bas").changed() {
            comp.background2 = grad.then_some([60, 20, 90, 255]);
        }
        if let Some(c) = &mut comp.background2 {
            color_button(ui, c);
        }
    });
}

fn content_editor(ui: &mut Ui, content: &mut Content) {
    match content {
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
        Content::Text { text, size, color, bold, family, italic, tracking, align } => {
            ui.add(egui::TextEdit::multiline(text).desired_rows(2).desired_width(f32::INFINITY));
            ui.label(RichText::new("Taille").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(size, 8.0..=600.0).logarithmic(true));
            egui::ComboBox::from_id_salt("font").selected_text(family.as_str()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                for f in FONTS {
                    ui.selectable_value(family, f.to_string(), RichText::new(f));
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(bold, "Gras");
                ui.checkbox(italic, "Italique");
            });
            ui.label(RichText::new("Alignement sur le point").size(12.5).color(MUTED));
            let mut a = *align as usize;
            if pro::segmented(ui, &mut a, &["Centre", "Gauche", "Droite"]) {
                *align = a as u8;
            }
            ui.label(RichText::new("Espacement des lettres").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(tracking, -20.0..=120.0).suffix(" px"));
            color_row(ui, color);
        }
        Content::Image { path, w, h } => {
            ui.label(RichText::new(Path::new(path.as_str()).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).size(12.0).color(FAINT));
            size_row(ui, w, h);
        }
        Content::Star { points, radius, inner, color } => {
            ui.add(egui::DragValue::new(points).range(3..=24).prefix("Branches "));
            ui.label(RichText::new("Rayon").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(radius, 4.0..=2000.0).logarithmic(true));
            ui.label(RichText::new("Creux (1 = polygone)").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(inner, 0.1..=1.0));
            color_row(ui, color);
        }
        Content::Line { w, thickness, color } | Content::Arrow { w, thickness, color } => {
            ui.label(RichText::new("Longueur").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(w, 4.0..=4000.0).logarithmic(true));
            ui.label(RichText::new("Épaisseur").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(thickness, 1.0..=200.0));
            color_row(ui, color);
        }
        Content::Ring { radius, thickness, color } => {
            ui.label(RichText::new("Rayon").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(radius, 4.0..=2000.0).logarithmic(true));
            ui.label(RichText::new("Épaisseur").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(thickness, 1.0..=300.0));
            ui.label(RichText::new("Révélation % = longueur de l'arc : anime-la pour une jauge.").size(11.5).color(FAINT));
            color_row(ui, color);
        }
        Content::Counter { from, to, decimals, size, color, prefix, suffix } => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("De").color(MUTED));
                ui.add(egui::DragValue::new(from).speed(1.0));
                ui.label(RichText::new("à").color(MUTED));
                ui.add(egui::DragValue::new(to).speed(1.0));
            });
            ui.add(egui::DragValue::new(decimals).range(0..=3).prefix("Décimales "));
            ui.label(RichText::new("Taille").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(size, 8.0..=600.0).logarithmic(true));
            ui.horizontal(|ui| {
                ui.label(RichText::new("Avant").color(MUTED));
                ui.add(egui::TextEdit::singleline(prefix).desired_width(60.0));
                ui.label(RichText::new("Après").color(MUTED));
                ui.add(egui::TextEdit::singleline(suffix).desired_width(60.0));
            });
            color_row(ui, color);
            ui.label(RichText::new("Compte pendant 80 % de la durée du calque").size(12.0).color(FAINT));
        }
    }
}

fn diamond(p: &Painter, c: Pos2, d: f32, color: Color32) {
    p.add(Shape::convex_polygon(vec![c + vec2(0.0, -d), c + vec2(d, 0.0), c + vec2(0.0, d), c + vec2(-d, 0.0)], color, Stroke::new(1.0, Color32::from_black_alpha(120))));
}

fn key_shape(p: &Painter, c: Pos2, d: f32, color: Color32, k: &model::Key) {
    let edge = Stroke::new(1.0, Color32::from_black_alpha(120));
    match (k.ease, k.ease2) {
        (Ease::Hold, Ease2::None) => {
            p.rect(Rect::from_center_size(c, Vec2::splat(d * 1.5)), CornerRadius::same(1), color, edge, StrokeKind::Middle);
        }
        (Ease::Linear, Ease2::None) => diamond(p, c, d, color),
        _ => {
            p.add(Shape::convex_polygon(vec![c + vec2(-d * 0.8, -d), c + vec2(d * 0.8, -d), c + vec2(0.0, 0.0)], color, edge));
            p.add(Shape::convex_polygon(vec![c + vec2(0.0, 0.0), c + vec2(d * 0.8, d), c + vec2(-d * 0.8, d)], color, edge));
        }
    }
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

fn glyph(p: &Painter, r: Rect, g: G, c: Color32) {
    let s = Stroke::new(1.6, c);
    let cen = r.center();
    let u = r.width() / 2.0;
    match g {
        G::Select => {
            p.add(Shape::convex_polygon(vec![r.left_top() + vec2(u * 0.3, 0.0), r.left_top() + vec2(u * 0.3, u * 1.7), cen + vec2(-u * 0.05, u * 0.35), cen + vec2(u * 0.75, u * 0.35)], c, Stroke::NONE));
        }
        G::Rotate | G::Loop => {
            let pts: Vec<Pos2> = (0..=20).map(|k| {
                let a = (-40.0 + k as f32 * 13.0f32).to_radians();
                cen + vec2(a.cos(), a.sin()) * u * 0.8
            }).collect();
            let end = *pts.last().unwrap();
            p.add(Shape::line(pts, s));
            p.add(Shape::convex_polygon(vec![end + vec2(-u * 0.45, -u * 0.05), end + vec2(u * 0.15, -u * 0.35), end + vec2(u * 0.05, u * 0.3)], c, Stroke::NONE));
        }
        G::Hand => {
            for k in 0..4 {
                let x = r.left() + u * (0.45 + k as f32 * 0.38);
                p.line_segment([pos2(x, r.top() + u * (0.25 + (k as f32 - 1.5).abs() * 0.15)), pos2(x, cen.y + u * 0.2)], Stroke::new(2.0, c));
            }
            p.rect_filled(Rect::from_min_max(pos2(r.left() + u * 0.3, cen.y), pos2(r.right() - u * 0.25, r.bottom())), CornerRadius::same(4), c);
        }
        G::Grid => {
            p.rect_stroke(r, CornerRadius::same(2), s, StrokeKind::Middle);
            for f in [1.0 / 3.0, 2.0 / 3.0] {
                p.line_segment([pos2(r.left() + r.width() * f, r.top()), pos2(r.left() + r.width() * f, r.bottom())], Stroke::new(1.0, c));
                p.line_segment([pos2(r.left(), r.top() + r.height() * f), pos2(r.right(), r.top() + r.height() * f)], Stroke::new(1.0, c));
            }
        }
        G::Safe => {
            p.rect_stroke(r, CornerRadius::same(2), s, StrokeKind::Middle);
            p.rect_stroke(r.shrink(u * 0.4), CornerRadius::same(1), Stroke::new(1.0, c), StrokeKind::Middle);
        }
        G::Guides => {
            p.line_segment([pos2(cen.x, r.top()), pos2(cen.x, r.bottom())], s);
            p.line_segment([pos2(r.left(), cen.y), pos2(r.right(), cen.y)], s);
            p.circle_stroke(cen, u * 0.35, Stroke::new(1.0, c));
        }
        G::Export => {
            p.line_segment([cen + vec2(0.0, u * 0.3), cen + vec2(0.0, -u * 0.95)], s);
            p.line_segment([cen + vec2(-u * 0.45, -u * 0.5), cen + vec2(0.0, -u * 0.95)], s);
            p.line_segment([cen + vec2(u * 0.45, -u * 0.5), cen + vec2(0.0, -u * 0.95)], s);
            p.add(Shape::line(vec![cen + vec2(-u * 0.9, -u * 0.1), cen + vec2(-u * 0.9, u * 0.9), cen + vec2(u * 0.9, u * 0.9), cen + vec2(u * 0.9, -u * 0.1)], s));
        }
        G::Eye => {
            let pts: Vec<Pos2> = (0..=16).map(|k| {
                let a = k as f32 / 16.0 * std::f32::consts::TAU;
                cen + vec2(a.cos() * u, a.sin() * u * 0.55)
            }).collect();
            p.add(Shape::closed_line(pts, Stroke::new(1.3, c)));
            p.circle_filled(cen, u * 0.3, c);
        }
        G::Lock => {
            p.rect_filled(Rect::from_min_max(pos2(r.left() + u * 0.2, cen.y - u * 0.1), pos2(r.right() - u * 0.2, r.bottom())), CornerRadius::same(2), c);
            let pts: Vec<Pos2> = (0..=10).map(|k| {
                let a = std::f32::consts::PI + k as f32 / 10.0 * std::f32::consts::PI;
                pos2(cen.x, cen.y - u * 0.1) + vec2(a.cos() * u * 0.5, a.sin() * u * 0.7)
            }).collect();
            p.add(Shape::line(pts, Stroke::new(1.4, c)));
        }
        G::Solo => {
            p.circle_stroke(cen, u * 0.85, Stroke::new(1.3, c));
            p.circle_filled(cen, u * 0.38, c);
        }
        G::Magnet => {
            let pts: Vec<Pos2> = (0..=12).map(|k| {
                let a = k as f32 / 12.0 * std::f32::consts::PI;
                pos2(cen.x, cen.y + u * 0.1) + vec2(a.cos() * u * 0.65, a.sin() * u * 0.75)
            }).collect();
            p.add(Shape::line(pts, Stroke::new(2.4, c)));
            p.line_segment([pos2(cen.x - u * 0.65, cen.y + u * 0.1), pos2(cen.x - u * 0.65, r.top())], Stroke::new(2.4, c));
            p.line_segment([pos2(cen.x + u * 0.65, cen.y + u * 0.1), pos2(cen.x + u * 0.65, r.top())], Stroke::new(2.4, c));
        }
        G::Marker => {
            p.add(Shape::convex_polygon(vec![r.left_top() + vec2(u * 0.3, 0.0), r.right_top() - vec2(u * 0.3, 0.0), pos2(r.right() - u * 0.3, cen.y + u * 0.2), pos2(cen.x, r.bottom()), pos2(r.left() + u * 0.3, cen.y + u * 0.2)], c, Stroke::NONE));
        }
        G::Half => {
            p.rect_stroke(r, CornerRadius::same(2), s, StrokeKind::Middle);
            p.rect_filled(Rect::from_min_max(r.min, pos2(cen.x, r.bottom())), CornerRadius::same(2), c);
        }
    }
}

fn layer_glyph(p: &Painter, r: Rect, kind: u8, c: Color32) {
    let s = Stroke::new(1.8, c);
    let ring = |n: usize, inner: f32| -> Vec<Pos2> {
        let count = if inner < 0.99 { n * 2 } else { n };
        (0..count)
            .map(|k| {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::TAU / count as f32;
                let rr = if inner < 0.99 && k % 2 == 1 { inner } else { 1.0 };
                r.center() + vec2(a.cos(), a.sin()) * r.width() * 0.5 * rr
            })
            .collect()
    };
    match kind {
        4 => {
            p.add(Shape::closed_line(ring(5, 0.45), s));
        }
        5 => {
            p.add(Shape::closed_line(ring(6, 1.0), s));
        }
        8 => {
            p.add(Shape::closed_line(ring(3, 1.0), s));
        }
        6 => {
            p.line_segment([r.left_center(), r.right_center()], Stroke::new(3.0, c));
        }
        9 => {
            p.line_segment([r.left_center(), r.right_center() - vec2(4.0, 0.0)], Stroke::new(2.4, c));
            p.add(Shape::convex_polygon(vec![r.right_center(), r.right_center() + vec2(-7.0, -6.0), r.right_center() + vec2(-7.0, 6.0)], c, Stroke::NONE));
        }
        10 => {
            let pts: Vec<Pos2> = (0..=24).map(|k| {
                let a = (-90.0 + k as f32 * 11.0f32).to_radians();
                r.center() + vec2(a.cos(), a.sin()) * r.width() * 0.45
            }).collect();
            p.add(Shape::line(pts, Stroke::new(2.6, c)));
        }
        7 => {
            p.text(r.center(), Align2::CENTER_CENTER, "123", semibold(12.5), c);
        }
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
        11 => {
            p.rect_filled(r.shrink2(vec2(0.0, 2.0)), CornerRadius::same(3), c.gamma_multiply(0.5));
            p.rect_stroke(r.shrink2(vec2(0.0, 2.0)), CornerRadius::same(3), s, StrokeKind::Middle);
        }
        _ => {
            p.rect_stroke(r.shrink(1.0), CornerRadius::same(3), s, StrokeKind::Middle);
            p.circle_filled(r.left_top() + vec2(6.0, 6.0), 2.0, c);
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
            .with_inner_size([1560.0, 960.0])
            .with_min_inner_size([1180.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native("Motion", options, Box::new(|cc| Ok(Box::new(Motion::new(cc)))))
}
