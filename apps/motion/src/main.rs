#![windows_subsystem = "windows"]

#[allow(dead_code)]
#[path = "../../cut/src/media.rs"]
mod media;
mod export;
mod graph;
mod menus;
mod model;
mod panels;
mod tutos;
mod viewer;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key as EKey, Layout, Margin, Painter,
    Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2,
    pos2, vec2,
};
use export::ExportOpts;
use model::{BLENDS, Comp, Content, Ease, Ease2, Layer, MATTES, Marker, PROPS, Prop};
use viewer::{Tool, ToolOpts, VDrag};
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
const PRESETS: [(&str, &str, u32, u32); 12] = [
    ("1080p", "Vidéo 16:9", 1920, 1080),
    ("720p", "Vidéo HD légère", 1280, 720),
    ("4K UHD", "Vidéo 16:9", 3840, 2160),
    ("2K DCI", "Cinéma", 2048, 1080),
    ("Cinémascope", "Large 2,39:1", 1920, 804),
    ("Vertical", "Shorts, Reels, TikTok", 1080, 1920),
    ("Carré", "Publication 1:1", 1080, 1080),
    ("Portrait", "Publication 4:5", 1080, 1350),
    ("Bannière GIF", "Web", 800, 400),
    ("En-tête X", "Bannière 3:1", 1500, 500),
    ("LinkedIn", "Bannière 4:1", 1584, 396),
    ("Sticker", "Emoji animé", 512, 512),
];
const FONTS: [&str; 14] = ["Segoe UI", "Segoe UI Black", "Arial", "Arial Black", "Impact", "Bahnschrift", "Calibri", "Cambria", "Georgia", "Times New Roman", "Consolas", "Verdana", "Trebuchet MS", "Comic Sans MS"];
const GROUPS: [&str; 7] = ["Entrées", "Sorties", "Boucles", "Effets", "3D", "Tracés", "Texte"];
const ANIMS: [(&str, &str, usize, usize); 45] = [
    ("Retournement 3D", "Pivote sur l'axe vertical", 4, 27),
    ("Bascule 3D", "Bascule vers l'avant", 4, 28),
    ("Carte qui tourne", "Rotation Y continue", 4, 41),
    ("Tracé dessiné", "Le contour se dessine", 5, 29),
    ("Tracé effacé", "Le contour s'efface", 5, 30),
    ("Tracé qui court", "Un segment fait le tour", 5, 31),
    ("Lettres qui montent", "Une lettre après l'autre", 6, 35),
    ("Lettres qui tombent", "Arrivent du haut en tournant", 6, 36),
    ("Lettres en vrac", "Ordre aléatoire, zoom", 6, 37),
    ("Lettres qui s'envolent", "Sortie lettre par lettre", 6, 44),
    ("Étirement élastique", "S'étire en largeur", 0, 32),
    ("Écrasement", "Squash and stretch", 2, 33),
    ("Arc-en-ciel", "La teinte tourne sans fin", 3, 34),
    ("Glitch", "Tranches décalées et RVB", 3, 38),
    ("Zoom flou", "Arrive avec un flou radial", 0, 39),
    ("Pendule", "Balance sans fin", 2, 43),
    ("Tremblement de rotation", "Tourne nerveusement", 3, 42),
    ("Apparition colorée", "Du blanc vers sa couleur", 0, 40),
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
const TEMPLATES: [(&str, &str); 8] = [
    ("Bandeau titre", "Nom et fonction en bas d'écran"),
    ("Titre cinéma", "Grand titre espacé qui sort du flou"),
    ("Bouton S'abonner", "Bouton rouge qui rebondit"),
    ("Compte à rebours", "10 → 0 avec anneau"),
    ("Barre de progression", "Se remplit de 0 à 100 %"),
    ("Citation", "Texte en italique et auteur"),
    ("Coche validée", "Cercle puis coche qui se dessine"),
    ("Titre lettre par lettre", "Entrée et sortie animées"),
];
const FPS_CHOICES: [u32; 6] = [12, 24, 25, 30, 50, 60];

const QUALITY: [&str; 3] = ["Complète", "Demie", "Quart"];
const ZOOMS: [f32; 9] = [0.1, 0.25, 0.33, 0.5, 0.75, 1.0, 2.0, 4.0, 8.0];
const SAFE: [(&str, (f32, f32)); 3] = [("Classique 90/80 %", (0.9, 0.8)), ("Moderne 93/90 %", (0.93, 0.9)), ("Réseaux sociaux 85/75 %", (0.85, 0.75))];
const GUIDE: Color32 = Color32::from_rgb(0, 220, 255);
const SNAP: Color32 = Color32::from_rgb(255, 64, 160);

#[derive(Clone, Copy, PartialEq)]
enum G {
    Path,
    Onion,
    Checker,
    Ruler,
    Ruler2,
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
    rendered: Option<(Comp, f32, u32, bool)>,
    vdrag: Option<VDrag>,
    bar_drag: Option<(usize, u8, f32, Layer)>,
    key_drag: Option<(f32, Vec<(usize, usize, usize, f32)>)>,
    ksel: Vec<(usize, usize, f32)>,
    clip_keys: Option<Vec<(usize, model::Key)>>,
    tl_filter: Option<Vec<usize>>,
    gkey: Option<(usize, usize, usize)>,
    graph_speed: bool,
    tool_mem: [usize; 11],
    topts: ToolOpts,
    pen: Option<usize>,
    focus_text: bool,
    quality: usize,
    rulers: bool,
    user_guides: bool,
    checker: bool,
    onion: bool,
    onion_n: u32,
    onion_step: u32,
    onion_tex: Option<TextureHandle>,
    onion_key: Option<(Comp, f32, u32, u32, u32)>,
    paths: bool,
    grid_div: u32,
    safe_kind: usize,
    cursor: Option<(f32, f32)>,
    queue: Vec<export::QItem>,
    queue_sheet: bool,
    pps: f32,
    job: Option<Job>,
    error: Option<String>,
    title: String,
    icon: TextureHandle,
    help: studio_ui::help::Help,
    inbox: studio_ui::handoff::Receiver,
    palette: studio_ui::palette::Palette,
    tool: Tool,
    zoom: Option<f32>,
    pan: Vec2,
    grid: bool,
    safe: bool,
    guides: bool,
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
    if studio_fx::stack::active(&comp.effects) {
        let mut img = unpremultiply(&pix);
        studio_fx::stack::apply(&mut img, &comp.effects);
        for (d, s) in pix.pixels_mut().iter_mut().zip(img.pixels()) {
            *d = tiny_skia::ColorU8::from_rgba(s.0[0], s.0[1], s.0[2], s.0[3]).premultiply();
        }
    }
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
            vdrag: None,
            bar_drag: None,
            key_drag: None,
            ksel: Vec::new(),
            clip_keys: None,
            tl_filter: None,
            gkey: None,
            graph_speed: false,
            tool_mem: [0; 11],
            topts: ToolOpts::default(),
            pen: None,
            focus_text: false,
            quality: 0,
            rulers: false,
            user_guides: true,
            checker: false,
            onion: false,
            onion_n: 2,
            onion_step: 3,
            onion_tex: None,
            onion_key: None,
            paths: true,
            grid_div: 10,
            safe_kind: 0,
            cursor: None,
            queue: Vec::new(),
            queue_sheet: false,
            pps: 160.0,
            job: None,
            error: None,
            title: String::new(),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "motion"),
            help: studio_ui::help::Help::new("Motion", tutos::MOTION),
            inbox: studio_ui::handoff::Receiver::new("motion"),
            palette: studio_ui::palette::Palette::new(&cc.egui_ctx),
            tool: Tool::Select,
            zoom: None,
            pan: Vec2::ZERO,
            grid: false,
            safe: false,
            guides: false,
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
            } else if p.is_file() {
                me.receive(&p);
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
            12 => self.add_layer("Nul", Content::Null),
            13 => {
                let i = self.add_layer("Calque d'effets", Content::Adjust { w: cw, h: ch });
                self.comp.layers[i].fx.saturation = 130.0;
                i
            }
            14 => self.add_layer("Solide", Content::Rect { w: cw, h: ch, r: 0.0, color: [255, 92, 120, 255] }),
            15 => self.add_layer("Rectangle arrondi", Content::Rect { w: u * 0.37, h: u * 0.22, r: u * 0.06, color: [48, 209, 88, 255] }),
            16 => self.add_layer("Losange", Content::Star { points: 4, radius: u * 0.15, inner: 1.0, color: [255, 159, 10, 255] }),
            17 => {
                let mut c = Content::text("Vertical", (u * 0.07).max(20.0), [255; 4], true);
                if let Content::Text { vertical, .. } = &mut c {
                    *vertical = true;
                }
                self.add_layer("Texte vertical", c)
            }
            18..=21 => {
                let k = u * 0.15;
                let (name, pts, closed, fill) = match kind {
                    18 => ("Cœur", vec![model::Vtx { p: [0.0, k * 0.95], i: [k * 0.1, -k * 0.1], o: [-k * 0.1, -k * 0.1] }, model::Vtx { p: [-k, -k * 0.2], i: [0.0, k * 0.5], o: [0.0, -k * 0.6] }, model::Vtx { p: [0.0, -k * 0.45], i: [-k * 0.5, -k * 0.6], o: [k * 0.5, -k * 0.6] }, model::Vtx { p: [k, -k * 0.2], i: [0.0, -k * 0.6], o: [0.0, k * 0.5] }], true, true),
                    19 => ("Spirale", (0..14).map(|j| {
                        let a = j as f32 * 0.9;
                        let r = k * 0.08 * (1.0 + j as f32 * 0.55);
                        let (s, c) = a.sin_cos();
                        model::Vtx::smooth(c * r, s * r, -s * r * 0.3, c * r * 0.3)
                    }).collect(), false, false),
                    20 => ("Vague", (0..7).map(|j| model::Vtx::smooth(-k * 1.8 + j as f32 * k * 0.6, if j % 2 == 0 { -k * 0.25 } else { k * 0.25 }, k * 0.2, 0.0)).collect(), false, false),
                    _ => ("Coche", vec![model::Vtx::at(-k * 0.6, 0.0), model::Vtx::at(-k * 0.15, k * 0.45), model::Vtx::at(k * 0.7, -k * 0.45)], false, false),
                };
                let i = self.add_layer(name, Content::Path { pts, closed, color: [255, 69, 120, 255], fill });
                let l = &mut self.comp.layers[i];
                if !fill {
                    l.fx.stroke = Some([255, 255, 255, 255]);
                    l.fx.stroke_w = (u * 0.012).max(3.0);
                    l.props[17].base = l.fx.stroke_w;
                    l.fx.cap = 1;
                }
                i
            }
            22 => {
                let i = self.add_layer("Peinture", Content::Strokes { strokes: Vec::new(), width: self.topts.brush, color: self.topts.brush_color });
                self.set_tool(Tool::Brush);
                i
            }
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
        if let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "svg"]).pick_file() {
            self.add_image_path(&p);
        }
    }

    fn add_image_path(&mut self, p: &Path) {
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
            6 => {
                let mut ring = mk("Cercle", Content::Ring { radius: u * 0.16, thickness: u * 0.02, color: [48, 209, 88, 255] }, cw / 2.0, ch / 2.0);
                ring.props[7].add_key(t, 0.0);
                ring.props[7].add_key(t + 0.6, 100.0);
                ring.props[7].keys[0].set_ease(Ease::Strong);
                self.push_layer(ring);
                let k = u * 0.15;
                let mut check = mk("Coche", Content::Path { pts: vec![model::Vtx::at(-k * 0.55, 0.0), model::Vtx::at(-k * 0.12, k * 0.42), model::Vtx::at(k * 0.6, -k * 0.4)], closed: false, color: [255; 4], fill: false }, cw / 2.0, ch / 2.0);
                check.fx.stroke = Some([48, 209, 88, 255]);
                check.fx.stroke_w = u * 0.03;
                check.fx.cap = 1;
                check.normalize();
                check.props[17].base = u * 0.03;
                check.props[15].add_key(t + 0.5, 0.0);
                check.props[15].add_key(t + 0.9, 100.0);
                check.props[15].keys[0].set_ease(Ease::Out);
                check.props[2].add_key(t + 0.9, 100.0);
                check.props[2].add_key(t + 1.05, 112.0);
                check.props[2].add_key(t + 1.25, 100.0);
                self.push_layer(check);
            }
            7 => {
                let mut title = mk("Titre animé", Content::text("MOTION", u * 0.14, [255; 4], true).styled("Segoe UI Black", false, u * 0.01, 0), cw / 2.0, ch / 2.0);
                title.fx.ta = model::TextAnim { on: true, mode: 2, dy: u * 0.08, rot: 12.0, delay: 0.06, dur: 0.5, ease: Ease::Back, ..Default::default() };
                title.fx.gradient = Some([255, 92, 120, 255]);
                title.fx.gradient_angle = 90.0;
                self.push_layer(title);
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
            27 | 28 => {
                let pi = if kind == 27 { 13 } else { 12 };
                l.props[pi].add_key(t, 90.0);
                l.props[pi].add_key(t + d * 1.2, 0.0);
                ease_all(&mut l.props[pi], Ease::Back);
                l.props[4].add_key(t, 0.0);
                l.props[4].add_key(t + d * 0.4, 100.0);
            }
            29 | 30 => {
                let pi = if kind == 29 { 15 } else { 14 };
                l.props[pi].add_key(t, 0.0);
                l.props[pi].add_key(t + d * 2.0, 100.0);
                ease_all(&mut l.props[pi], Ease::Strong);
                if l.fx.stroke.is_none() && !matches!(l.content, Content::Line { .. } | Content::Ring { .. } | Content::Strokes { .. }) {
                    l.fx.stroke = Some([255; 4]);
                    l.props[17].set(t, 6.0);
                }
            }
            31 => {
                l.props[15].set(t, 25.0);
                l.props[16].add_key(t, 0.0);
                l.props[16].add_key(t + 1.0, 100.0);
                ease_all(&mut l.props[16], Ease::Linear);
                l.props[16].cycle = 3;
                if l.fx.stroke.is_none() && !matches!(l.content, Content::Line { .. } | Content::Ring { .. } | Content::Strokes { .. }) {
                    l.fx.stroke = Some([255; 4]);
                    l.props[17].set(t, 6.0);
                }
            }
            32 => {
                l.props[10].add_key(t, 0.0);
                l.props[10].add_key(t + d * 1.5, 100.0);
                ease_all(&mut l.props[10], Ease::Elastic);
            }
            33 => {
                for (dt, sx, sy) in [(0.0, 100.0, 100.0), (0.15, 120.0, 80.0), (0.3, 90.0, 112.0), (0.45, 100.0, 100.0), (0.9, 100.0, 100.0)] {
                    l.props[10].add_key(t + dt, sx);
                    l.props[11].add_key(t + dt, sy);
                }
                l.props[10].cycle = 1;
                l.props[11].cycle = 1;
            }
            34 => {
                l.props[18].add_key(t, 0.0);
                l.props[18].add_key(t + 2.0, 360.0);
                ease_all(&mut l.props[18], Ease::Linear);
                l.props[18].cycle = 3;
            }
            35..=37 | 44 => {
                let ta = &mut l.fx.ta;
                *ta = model::TextAnim { on: true, ..Default::default() };
                match kind {
                    36 => {
                        ta.dy = -120.0;
                        ta.rot = -40.0;
                        ta.ease = Ease::Bounce;
                        ta.dur = 0.6;
                    }
                    37 => {
                        ta.order = 3;
                        ta.dy = 0.0;
                        ta.scale = 0.0;
                        ta.ease = Ease::Back;
                    }
                    44 => {
                        ta.mode = 1;
                        ta.dy = -80.0;
                        ta.ease = Ease::In;
                    }
                    _ => {}
                }
                if kind != 44 {
                    l.start = t.min(l.end - 0.1).max(0.0);
                }
            }
            38 => {
                l.fx.glitch = 40.0;
                l.fx.glitch_freq = 10.0;
                l.fx.rgb_split = 6.0;
            }
            39 => {
                l.fx.zoom_blur = 30.0;
                l.props[2].add_key(t, 140.0);
                l.props[2].add_key(t + d, 100.0);
                l.props[4].add_key(t, 0.0);
                l.props[4].add_key(t + d * 0.6, 100.0);
                ease_all(&mut l.props[2], Ease::Out);
            }
            40 => {
                if let Some(c) = l.content.color() {
                    model::set_color_key(&mut l.color_keys, t, [255, 255, 255, c[3]]);
                    model::set_color_key(&mut l.color_keys, t + d * 1.5, c);
                }
            }
            41 => {
                l.props[13].add_key(t, 0.0);
                l.props[13].add_key(t + 2.0, 360.0);
                ease_all(&mut l.props[13], Ease::Linear);
                l.props[13].cycle = 3;
            }
            42 => {
                l.props[3].wiggle = 6.0;
                l.props[3].wiggle_freq = 3.0;
            }
            43 => {
                l.props[3].add_key(t, -20.0);
                l.props[3].add_key(t + 1.0, 20.0);
                ease_all(&mut l.props[3], Ease::Smooth);
                l.props[3].cycle = 2;
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

    fn place(&mut self, i: usize, wx: f32, wy: f32) {
        let t = self.t;
        let (lx, ly) = model::apply(model::invert(self.comp.parent_matrix(i, t)), wx, wy);
        let (gx, gy) = self.comp.layers[i].wiggle(t, i);
        let l = &mut self.comp.layers[i];
        l.props[0].set(t, (lx - gx).round());
        l.props[1].set(t, (ly - gy).round());
    }

    fn world_box(&self, i: usize) -> (f32, f32, f32, f32) {
        let m = self.comp.matrix(i, self.t);
        let (x0, y0, x1, y1) = self.comp.layers[i].bounds();
        let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
            let (a, c) = model::apply(m, x, y);
            b = (b.0.min(a), b.1.min(c), b.2.max(a), b.3.max(c));
        }
        b
    }

    fn align(&mut self, how: u8) {
        let Some(i) = self.sel else { return };
        let t = self.t;
        let (wx, wy, _, _) = self.comp.world(i, t);
        let (x0, y0, x1, y1) = self.world_box(i);
        let (cw, ch) = (self.comp.width as f32, self.comp.height as f32);
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let (dx, dy) = match how {
            0 => (-x0, 0.0),
            1 => (cw / 2.0 - cx, 0.0),
            2 => (cw - x1, 0.0),
            3 => (0.0, -y0),
            4 => (0.0, ch / 2.0 - cy),
            5 => (0.0, ch - y1),
            _ => (cw / 2.0 - cx, ch / 2.0 - cy),
        };
        self.place(i, wx + dx, wy + dy);
    }

    fn fit_to_comp(&mut self, mode: u8) {
        let Some(i) = self.sel else { return };
        let t = self.t;
        let (x0, y0, x1, y1) = self.world_box(i);
        let (cw, ch) = (self.comp.width as f32, self.comp.height as f32);
        let (w, h) = ((x1 - x0).max(1e-3), (y1 - y0).max(1e-3));
        let k = match mode {
            0 => (cw / w).min(ch / h),
            1 => cw / w,
            _ => ch / h,
        };
        let s = self.comp.layers[i].props[2].at(t);
        self.comp.layers[i].props[2].set(t, (s * k * 10.0).round() / 10.0);
        self.sel = Some(i);
        self.align(6);
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
        if sc(M::COMMAND, EKey::C) {
            if !self.ksel.is_empty() {
                self.copy_keys();
            } else if let Some(i) = self.sel {
                self.clip_layer = self.comp.layers.get(i).cloned();
                self.clip_keys = None;
            }
        }
        if sc(M::COMMAND, EKey::V) {
            if self.clip_keys.is_some() {
                self.paste_keys();
            } else if let Some(mut l) = self.clip_layer.clone() {
                l.parent = None;
                self.push_layer(l);
            }
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
                [p(EKey::J), p(EKey::K), p(EKey::B), p(EKey::N), p(EKey::M), p(EKey::V), p(EKey::W), p(EKey::H), p(EKey::L), p(EKey::U), i.key_pressed(EKey::F9)],
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
            if self.ksel.is_empty() {
                self.delete();
            } else {
                self.delete_keys();
            }
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
            self.set_tool(Tool::Select);
        }
        if w {
            self.set_tool(Tool::Rotate);
        }
        if h {
            self.set_tool(Tool::Hand);
        }
        let more = ctx.input(|i| {
            let p = |k| i.key_pressed(k) && !i.modifiers.command && !i.modifiers.alt;
            [p(EKey::Z), p(EKey::Y), p(EKey::Q), p(EKey::G), p(EKey::P), p(EKey::S), p(EKey::R), p(EKey::T), p(EKey::A), p(EKey::Period), p(EKey::Comma), p(EKey::OpenBracket), p(EKey::CloseBracket), p(EKey::PageUp), p(EKey::PageDown), p(EKey::F2), i.key_pressed(EKey::Escape), i.key_pressed(EKey::Enter)]
        });
        let [z, y, q, g, kp, ks, kr, kt, ka, zin, zout, inp, outp, pgu, pgd, f2, esc, enter] = more;
        for (on, grp) in [(z, 2), (y, 4), (q, 5), (g, 6)] {
            if on {
                self.cycle_group(grp);
            }
        }
        if sc(M::COMMAND, EKey::T) {
            self.cycle_group(7);
        }
        if sc(M::COMMAND, EKey::B) {
            self.cycle_group(8);
        }
        if (esc || enter) && self.pen.is_some() {
            self.pen = None;
        }
        for (on, props) in [(kp, vec![0, 1]), (ks, vec![2, 10, 11]), (kr, vec![3, 12, 13]), (kt, vec![4]), (ka, vec![8, 9])] {
            if on && let Some(i) = self.sel {
                self.tl_filter = if self.tl_filter.as_ref() == Some(&props) { None } else { Some(props) };
                if let Some(e) = self.expanded.get_mut(i) {
                    *e = true;
                }
            }
        }
        if zin || zout {
            let z0 = self.zoom.unwrap_or(0.5);
            self.zoom = Some(if zin { ZOOMS.iter().copied().find(|z| *z > z0 + 1e-3).unwrap_or(8.0) } else { ZOOMS.iter().rev().copied().find(|z| *z < z0 - 1e-3).unwrap_or(0.1) });
        }
        if let Some(i) = self.sel.filter(|i| *i < self.comp.layers.len()) {
            let t = self.t;
            let alt = ctx.input(|i| i.modifiers.alt);
            let l = &mut self.comp.layers[i];
            if inp {
                let d = t - l.start;
                l.shift(d);
                l.end = l.end.min(self.comp.duration);
            }
            if outp {
                let d = t - l.end;
                l.shift(d);
            }
            if alt && ctx.input(|i| i.key_pressed(EKey::OpenBracket)) {
                l.start = t.min(l.end - 0.05);
            }
            if alt && ctx.input(|i| i.key_pressed(EKey::CloseBracket)) {
                l.end = t.max(l.start + 0.05);
            }
        }
        if pgu {
            self.t = self.snap_frame((self.t - step).max(0.0));
        }
        if pgd {
            self.t = self.snap_frame((self.t + step).min(self.comp.duration));
        }
        if f2 {
            self.sel = None;
            self.ksel.clear();
        }
        if sc(M::COMMAND, EKey::A) && let Some(i) = self.sel {
            self.ksel = self.comp.layers[i].props.iter().enumerate().flat_map(|(pi, p)| p.keys.iter().map(move |k| (i, pi, k.t))).collect();
        }
        if sc(M::COMMAND | M::ALT, EKey::H) {
            self.with_keys(|k| k.set_ease(if k.ease == Ease::Hold && k.ease2 == Ease2::None && k.bez.is_none() { Ease::Linear } else { Ease::Hold }));
        }
        if sc(M::COMMAND | M::ALT, EKey::R) {
            self.reverse_keys();
        }
        if sc(M::COMMAND | M::ALT, EKey::Home) {
            self.anchor_to(4);
        }
        if sc(M::COMMAND | M::ALT, EKey::F) {
            self.fit_to_comp(0);
        }
        if sc(M::COMMAND | M::ALT, EKey::M) {
            self.queue_sheet = true;
        }
        if sc(M::COMMAND, EKey::R) {
            self.rulers = !self.rulers;
        }
        if sc(M::COMMAND, EKey::Quote) {
            self.grid = !self.grid;
        }
        if sc(M::COMMAND, EKey::Semicolon) {
            self.user_guides = !self.user_guides;
        }
        if l {
            self.loop_play = !self.loop_play;
        }
        if u && let Some(i) = self.sel {
            if let Some(e) = self.expanded.get_mut(i) {
                *e = !*e;
            }
        }
        if f9 {
            let (sh, ctrl) = ctx.input(|i| (i.modifiers.shift, i.modifiers.ctrl));
            self.easy_ease(match (sh, ctrl) {
                (true, true) => 2,
                (true, false) => 1,
                _ => 0,
            });
        }
        if u {
            self.tl_filter = None;
        }
        self.comp.normalize();
        self.expanded.resize(self.comp.layers.len(), false);
        if self.sel.is_some_and(|i| i >= self.comp.layers.len()) {
            self.sel = None;
        }
    }

}

impl Motion {

    fn commands(&self) -> Vec<studio_ui::palette::Cmd> {
        let mut v = vec![

        ];
        for (size, fps) in [(128u32, 12u32), (256, 12), (256, 24), (512, 12)] {
            v.push(studio_ui::palette::Cmd::new(format!("Planche de sprites {size} px, {fps} i/s"), "Export pour jeux · PNG transparent", format!("sprites:{size}:{fps}")));
        }
        for (step, name) in [(0.1, "0,1 s"), (0.25, "0,25 s"), (0.5, "0,5 s"), (1.0, "1 s")] {
            v.push(studio_ui::palette::Cmd::new(format!("Calques en cascade : {name} d'écart"), "Composition", format!("cascade:{step}")));
        }
        v.push(studio_ui::palette::Cmd::new("Aligner tous les calques au même départ", "Composition", "cascade:0"));
        for (k, name) in model::ANIM_PRESETS.iter().enumerate() {
            v.push(studio_ui::palette::Cmd::new(format!("Animer : {name}"), "Calque sélectionné", format!("anim:{k}")));
        }
        v.extend(studio_ui::palette::send_cmds(&[("cut", "ajouter au montage"), ("pixel", "retoucher l'image"), ("page", "mettre en page"), ("vector", "placer l'image"), ("lumen", "développer")]));
        v.extend(studio_ui::palette::effect_cmds());
        v
    }

    fn run_cmd(&mut self, id: &str, ctx: &egui::Context) {
        if let Some(t) = id.strip_prefix("send:") {
            let t: &'static str = studio_core::handoff::APPS.iter().find(|a| a.0 == t).map_or("pixel", |a| a.0);
            self.send_to(t, ctx);
            return;
        }
        if let Some(k) = id.strip_prefix("fx:").and_then(studio_fx::stack::kind) {
            self.comp.effects.push(studio_fx::stack::Fx::new(k));
            return;
        }
        if let Some((size, fps)) = id.strip_prefix("sprites:").and_then(|s| s.split_once(':')).and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?))) {
            let Some(p) = rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name("sprites.png").save_file() else { return };
            let (cw, ch) = (self.comp.width.max(1) as f32, self.comp.height.max(1) as f32);
            let k = size as f32 / cw.max(ch);
            let (fw, fh) = (((cw * k).round() as u32).max(1), ((ch * k).round() as u32).max(1));
            let (a, b) = self.comp.range();
            let frames = (((b - a) * fps as f32).round() as u32).clamp(1, 400);
            let cols = (frames as f32).sqrt().ceil() as u32;
            let rows = frames.div_ceil(cols);
            let mut sheet = image::RgbaImage::new(cols * fw, rows * fh);
            for f in 0..frames {
                let t = a + f as f32 / fps as f32;
                let Ok(pix) = render(&self.comp, t, fw, fh, &self.fontdb, true) else { continue };
                let img = image::RgbaImage::from_fn(fw, fh, |x, y| {
                    let c = pix.pixel(x, y).map(|c| c.demultiply()).unwrap_or(tiny_skia::ColorU8::from_rgba(0, 0, 0, 0));
                    image::Rgba([c.red(), c.green(), c.blue(), c.alpha()])
                });
                image::imageops::overlay(&mut sheet, &img, ((f % cols) * fw) as i64, ((f / cols) * fh) as i64);
            }
            match sheet.save(&p) {
                Ok(()) => studio_ui::handoff::toast(ctx, format!("{frames} images de {fw}×{fh} en {cols}×{rows} : {}", p.display()), true),
                Err(e) => studio_ui::handoff::toast(ctx, e.to_string(), false),
            }
            return;
        }
        if let Some(step) = id.strip_prefix("cascade:").and_then(|s| s.parse::<f32>().ok()) {
            model::cascade(&mut self.comp.layers, step);
            let end = self.comp.layers.iter().map(|l| l.end).fold(0.0, f32::max);
            if end > self.comp.duration {
                self.comp.duration = end;
            }
            studio_ui::handoff::toast(ctx, if step > 0.0 { format!("{} calques en cascade", self.comp.layers.len()) } else { "Calques alignés".into() }, true);
            return;
        }
        if let Some(k) = id.strip_prefix("anim:").and_then(|k| k.parse::<usize>().ok()).filter(|k| *k < model::ANIM_PRESETS.len()) {
            let (w, h) = (self.comp.width as f32, self.comp.height as f32);
            match self.sel.and_then(|i| self.comp.layers.get_mut(i)) {
                Some(l) => {
                    model::apply_anim_preset(l, k, w, h);
                    studio_ui::handoff::toast(ctx, format!("{} appliqué à « {} »", model::ANIM_PRESETS[k], l.name), true);
                }
                None => studio_ui::handoff::toast(ctx, "Sélectionne d'abord un calque".into(), false),
            }
            return;
        }
        match id {

            _ => {}
        }
    }
    fn send_to(&mut self, target: &str, ctx: &egui::Context) {
        let p = studio_core::handoff::outgoing("motion", "image de l'animation", "png");
        match render(&self.comp, self.t, self.comp.width, self.comp.height, &self.fontdb, false).and_then(|pix| pix.save_png(&p).map_err(|e| e.to_string())) {
            Ok(()) => studio_ui::handoff::send(ctx, target, &p),
            Err(e) => studio_ui::handoff::toast(ctx, format!("Envoi impossible : {e}"), false),
        }
    }

    fn receive(&mut self, p: &Path) {
        if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("emot")) {
            self.open_path(p);
        } else {
            self.home = false;
            self.add_image_path(p);
        }
    }
}

impl eframe::App for Motion {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        studio_ui::chrome::resize(ctx);
        for p in self.inbox.poll(ctx) {
            self.receive(&p);
        }
        let before = self.comp.clone();
        self.palette.keys(ctx);
        if self.palette.open {
            let cmds = self.commands();
            if let Some(id) = self.palette.show(ctx, &mut self.help, cmds) {
                self.run_cmd(&id, ctx);
            }
        }
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
            self.viewer(root);
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
            let full = studio_ui::chrome::bar(ui, 12.0);
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

    fn status(&mut self, root: &mut Ui) {
        egui::Panel::bottom("status").exact_size(26.0).frame(Frame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                if !self.home {
                    ui.label(small("Espace lire · J K clés · M repère · B N zone · V H Z W Y Q G outils · Ctrl+T texte · Ctrl+B pinceau · P S R T A U propriétés · F9 courbe douce".into()));
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

    fn sheets(&mut self, ctx: &egui::Context) {
        self.export_sheet(ctx);
        self.queue_sheet(ctx);
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
        let dur = self.comp.duration.max(0.1);
        let n_rows: f32 = (0..self.comp.layers.len()).map(|i| row_h + if self.expanded.get(i).copied().unwrap_or(false) { sub_h * self.shown_props(i).len() as f32 } else { 0.0 }).sum();
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
            p.text(pos2(rect.min.x + 102.0, row.center().y), Align2::LEFT_CENTER, format!("{parent_tag}{}", short(&l.name, 15)), if on { semibold(12.5) } else { FontId::proportional(12.5) }, name_col);
            if l.has_effects() {
                let fr = Rect::from_center_size(pos2(x0 - 16.0, row.center().y), vec2(20.0, 14.0));
                p.rect_filled(fr, CornerRadius::same(4), BLUE.gamma_multiply(0.35));
                p.text(fr.center(), Align2::CENTER_CENTER, "fx", FontId::proportional(10.0), INK);
            }
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
                for pi in self.shown_props(li) {
                    let name = PROPS[pi];
                    let sub = Rect::from_min_max(pos2(rect.min.x, y), pos2(rect.max.x, y + sub_h));
                    p.rect_filled(sub, CornerRadius::ZERO, Color32::from_rgb(0x1c, 0x1c, 0x20));
                    let Some(prop) = self.comp.layers[li].props.get(pi) else { continue };
                    let live = prop.animated() || prop.expressive();
                    p.text(pos2(rect.min.x + 44.0, sub.center().y), Align2::LEFT_CENTER, if prop.expressive() { format!("{name} ≈") } else { name.to_string() }, FontId::proportional(11.5), if live { INK } else { FAINT });
                    p.text(pos2(x0 - 10.0, sub.center().y), Align2::RIGHT_CENTER, format!("{:.1}", prop.at(self.t)), FontId::monospace(11.0), if live { KEY } else { FAINT });
                    if prop.keys.len() > 1 {
                        p.line_segment([pos2(x_of(prop.keys[0].t), sub.center().y), pos2(x_of(prop.keys[prop.keys.len() - 1].t), sub.center().y)], Stroke::new(1.0, KEY.gamma_multiply(0.35)));
                    }
                    let keys: Vec<model::Key> = prop.keys.clone();
                    for (ki, key) in keys.iter().enumerate() {
                        let c = pos2(x_of(key.t), sub.center().y);
                        let kr = Rect::from_center_size(c, Vec2::splat(14.0));
                        let resp = ui.interact(kr, ui.make_persistent_id(("key", li, pi, ki)), Sense::click_and_drag()).on_hover_text(format!("{} · {:.1} · {}\nClic : choisir · Maj/Ctrl+clic : ajouter · glisser : déplacer", secs(key.t), key.v, key.ease_name()));
                        let selected = self.ksel.iter().any(|s| s.0 == li && s.1 == pi && (s.2 - key.t).abs() < 1e-3);
                        let dragging = self.key_drag.as_ref().is_some_and(|(_, v)| v.iter().any(|d| d.0 == li && d.1 == pi && d.2 == ki));
                        key_shape(&p, c, if resp.hovered() || dragging { 6.5 } else { 5.0 }, if selected || dragging { Color32::WHITE } else { KEY }, key);
                        if selected {
                            p.circle_stroke(c, 8.0, Stroke::new(1.0, BLUE));
                        }
                        let multi = ui.input(|i| i.modifiers.shift || i.modifiers.command);
                        if resp.clicked() {
                            self.t = key.t;
                            self.sel = Some(li);
                            self.gkey = Some((li, pi, ki));
                            if multi {
                                if selected {
                                    self.ksel.retain(|s| !(s.0 == li && s.1 == pi && (s.2 - key.t).abs() < 1e-3));
                                } else {
                                    self.ksel.push((li, pi, key.t));
                                }
                            } else {
                                self.ksel = vec![(li, pi, key.t)];
                            }
                        }
                        if resp.drag_started() {
                            if !selected {
                                if !multi {
                                    self.ksel.clear();
                                }
                                self.ksel.push((li, pi, key.t));
                            }
                            let group: Vec<(usize, usize, usize, f32)> = self
                                .ksel
                                .iter()
                                .filter_map(|(sl, sp, st)| self.comp.layers.get(*sl)?.props.get(*sp)?.key_at(*st).map(|k| (*sl, *sp, k, *st)))
                                .collect();
                            self.key_drag = Some((key.t, group));
                            self.sel = Some(li);
                        }
                        resp.context_menu(|ui| {
                            if !selected {
                                self.ksel = vec![(li, pi, key.t)];
                            }
                            for (e, n) in Ease::ALL {
                                if ui.selectable_label(key.ease2 == Ease2::None && key.bez.is_none() && key.ease == e, n).clicked() {
                                    self.with_keys(|k| k.set_ease(e));
                                    ui.close();
                                }
                            }
                            ui.menu_button("Autres courbes", |ui| {
                                for (e, n) in Ease2::ALL {
                                    if ui.selectable_label(key.ease2 == e && key.bez.is_none(), n).clicked() {
                                        self.with_keys(|k| k.set_ease2(e));
                                        ui.close();
                                    }
                                }
                                ui.separator();
                                for (n, b) in model::BEZ_PRESETS {
                                    if ui.selectable_label(key.bez == Some(b), n).clicked() {
                                        self.with_keys(|k| k.bez = Some(b));
                                        ui.close();
                                    }
                                }
                            });
                            ui.separator();
                            if ui.button("Courbe douce (F9)").clicked() {
                                self.easy_ease(0);
                                ui.close();
                            }
                            if ui.button("Éditer la courbe…").clicked() {
                                self.gkey = Some((li, pi, ki));
                                self.sel = Some(li);
                                ui.close();
                            }
                            if ui.button("Copier (Ctrl+C)").clicked() {
                                self.copy_keys();
                                ui.close();
                            }
                            if ui.button("Sélectionner toute la propriété").clicked() {
                                self.ksel = keys.iter().map(|k| (li, pi, k.t)).collect();
                                ui.close();
                            }
                            if ui.button("Supprimer (Suppr)").clicked() {
                                self.delete_keys();
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

        if let Some((anchor, group)) = self.key_drag.clone() {
            let (down, pos) = ui.input(|i| (i.pointer.primary_down(), i.pointer.hover_pos()));
            if let (true, Some(pp)) = (down, pos) {
                let skip = group.first().map(|g| (g.0, g.1));
                let nt = self.snap_time(t_of(pp.x), None, skip);
                let dt = nt - anchor;
                for (li, pi, ki, t0) in &group {
                    if let Some(k) = self.comp.layers.get_mut(*li).and_then(|l| l.props.get_mut(*pi)).and_then(|p| p.keys.get_mut(*ki)) {
                        k.t = (t0 + dt).max(0.0);
                    }
                }
                self.ksel = group.iter().map(|(li, pi, _, t0)| (*li, *pi, (t0 + dt).max(0.0))).collect();
                self.t = nt;
            } else {
                for (li, pi, _, _) in &group {
                    if let Some(prop) = self.comp.layers.get_mut(*li).and_then(|l| l.props.get_mut(*pi)) {
                        prop.keys.sort_by(|a, b| a.t.total_cmp(&b.t));
                        prop.keys.dedup_by(|a, b| (a.t - b.t).abs() < 1e-3);
                    }
                }
                self.gkey = None;
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

const KINDS: [(&str, u8); 23] = [
    ("Texte", 0),
    ("Vertical", 17),
    ("Compteur", 7),
    ("Rectangle", 1),
    ("Arrondi", 15),
    ("Ellipse", 2),
    ("Étoile", 4),
    ("Polygone", 5),
    ("Triangle", 8),
    ("Losange", 16),
    ("Ligne", 6),
    ("Flèche", 9),
    ("Anneau", 10),
    ("Cœur", 18),
    ("Spirale", 19),
    ("Vague", 20),
    ("Coche", 21),
    ("Peinture", 22),
    ("Image…", 3),
    ("Fond", 11),
    ("Solide", 14),
    ("Nul", 12),
    ("Effets", 13),
];

fn secs(t: f32) -> String {
    format!("{:.1} s", t).replace('.', ",")
}

fn slider(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) {
    ui.label(RichText::new(label).size(12.5).color(MUTED));
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = (ui.available_width() - 112.0).max(60.0);
        ui.add(egui::Slider::new(v, range).suffix(suffix).max_decimals(2));
    });
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

fn content_editor(ui: &mut Ui, content: &mut Content, focus: bool) {
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
        Content::Text { text, size, color, bold, family, italic, tracking, align, vertical } => {
            let te = ui.add(egui::TextEdit::multiline(text).desired_rows(2).desired_width(f32::INFINITY).id_salt("text-content"));
            if focus {
                te.request_focus();
            }
            ui.checkbox(vertical, "Texte vertical");
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
        Content::Path { pts, closed, color, fill } => {
            ui.label(RichText::new(format!("{} sommets · outil Plume (G) pour éditer", pts.len())).size(12.0).color(FAINT));
            ui.horizontal(|ui| {
                ui.checkbox(closed, "Tracé fermé");
                ui.checkbox(fill, "Remplir");
            });
            ui.horizontal(|ui| {
                if ui.small_button("Tout lisser").clicked() {
                    smooth_all(pts, *closed);
                }
                if ui.small_button("Tout en angles").clicked() {
                    for v in pts.iter_mut() {
                        v.i = [0.0; 2];
                        v.o = [0.0; 2];
                    }
                }
                if ui.small_button("Inverser").clicked() {
                    pts.reverse();
                    for v in pts.iter_mut() {
                        std::mem::swap(&mut v.i, &mut v.o);
                    }
                }
            });
            if *fill {
                color_row(ui, color);
            } else {
                ui.label(RichText::new("Sans remplissage : règle le contour dans Apparence.").size(11.5).color(FAINT));
            }
        }
        Content::Strokes { strokes, width, color } => {
            ui.label(RichText::new(format!("{} traits · Pinceau (Ctrl+B) pour peindre, Gomme pour effacer", strokes.len())).size(12.0).color(FAINT));
            ui.label(RichText::new("Épaisseur").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(width, 0.5..=200.0).logarithmic(true).suffix(" px"));
            ui.horizontal(|ui| {
                if ui.add_enabled(!strokes.is_empty(), egui::Button::new("Annuler le dernier trait").small()).clicked() {
                    strokes.pop();
                }
                if ui.add_enabled(!strokes.is_empty(), egui::Button::new("Tout effacer").small()).clicked() {
                    strokes.clear();
                }
            });
            color_row(ui, color);
        }
        Content::Null => {
            ui.label(RichText::new("Un nul est invisible au rendu : choisis-le comme parent pour piloter plusieurs calques à la fois.").size(12.0).color(FAINT));
        }
        Content::Adjust { w, h } => {
            size_row(ui, w, h);
            ui.label(RichText::new("Calque d'effets : ses effets (couleur, flou, grain, distorsion…) s'appliquent à tous les calques situés en dessous. Son opacité dose le mélange.").size(12.0).color(FAINT));
        }
    }
}

fn smooth_all(pts: &mut [model::Vtx], closed: bool) {
    for k in 0..pts.len() {
        smooth_vertex(pts, k, closed);
    }
}

fn smooth_vertex(pts: &mut [model::Vtx], k: usize, closed: bool) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let prev = if k == 0 { if closed { pts[n - 1].p } else { pts[0].p } } else { pts[k - 1].p };
    let next = if k + 1 == n { if closed { pts[0].p } else { pts[n - 1].p } } else { pts[k + 1].p };
    let (dx, dy) = ((next[0] - prev[0]) / 6.0, (next[1] - prev[1]) / 6.0);
    pts[k].i = [-dx, -dy];
    pts[k].o = [dx, dy];
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
        G::Path => {
            let pts: Vec<Pos2> = (0..=16).map(|k| {
                let u2 = k as f32 / 16.0;
                pos2(r.left() + u2 * r.width(), cen.y + (u2 * std::f32::consts::TAU).sin() * u * 0.55)
            }).collect();
            for q in pts.iter().step_by(2) {
                p.circle_filled(*q, 1.3, c);
            }
            p.rect_filled(Rect::from_center_size(pts[4], Vec2::splat(u * 0.5)), CornerRadius::ZERO, c);
            p.rect_filled(Rect::from_center_size(pts[12], Vec2::splat(u * 0.5)), CornerRadius::ZERO, c);
        }
        G::Onion => {
            p.circle_stroke(cen - vec2(u * 0.4, 0.0), u * 0.55, Stroke::new(1.0, c.gamma_multiply(0.5)));
            p.circle_stroke(cen + vec2(u * 0.4, 0.0), u * 0.55, Stroke::new(1.0, c.gamma_multiply(0.5)));
            p.circle_filled(cen, u * 0.55, c);
        }
        G::Checker => {
            p.rect_stroke(r, CornerRadius::same(2), Stroke::new(1.0, c), StrokeKind::Middle);
            let h = r.width() / 4.0;
            for y in 0..4 {
                for x in 0..4 {
                    if (x + y) % 2 == 0 {
                        p.rect_filled(Rect::from_min_size(r.min + vec2(x as f32 * h, y as f32 * h), Vec2::splat(h)), CornerRadius::ZERO, c);
                    }
                }
            }
        }
        G::Ruler => {
            p.rect_stroke(Rect::from_min_max(r.min, pos2(r.right(), r.top() + u * 0.6)), CornerRadius::ZERO, s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(r.min, pos2(r.left() + u * 0.6, r.bottom())), CornerRadius::ZERO, s, StrokeKind::Middle);
            for k in 1..4 {
                let f = k as f32 / 4.0;
                p.line_segment([pos2(r.left() + r.width() * f, r.top()), pos2(r.left() + r.width() * f, r.top() + u * 0.35)], Stroke::new(1.0, c));
                p.line_segment([pos2(r.left(), r.top() + r.height() * f), pos2(r.left() + u * 0.35, r.top() + r.height() * f)], Stroke::new(1.0, c));
            }
        }
        G::Ruler2 => {
            p.line_segment([pos2(cen.x - u * 0.3, r.top()), pos2(cen.x - u * 0.3, r.bottom())], Stroke::new(1.4, c));
            p.line_segment([pos2(r.left(), cen.y + u * 0.3), pos2(r.right(), cen.y + u * 0.3)], Stroke::new(1.4, c));
            p.rect_stroke(r, CornerRadius::same(2), Stroke::new(1.0, c.gamma_multiply(0.5)), StrokeKind::Middle);
        }
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
        11 | 14 => {
            p.rect_filled(r.shrink2(vec2(0.0, 2.0)), CornerRadius::same(3), c.gamma_multiply(if kind == 14 { 0.9 } else { 0.5 }));
            p.rect_stroke(r.shrink2(vec2(0.0, 2.0)), CornerRadius::same(3), s, StrokeKind::Middle);
        }
        12 => {
            p.rect_stroke(r.shrink(2.0), CornerRadius::ZERO, Stroke::new(1.4, Color32::from_rgb(255, 120, 120)), StrokeKind::Middle);
            p.line_segment([r.center() - vec2(4.0, 0.0), r.center() + vec2(4.0, 0.0)], Stroke::new(1.2, c));
            p.line_segment([r.center() - vec2(0.0, 4.0), r.center() + vec2(0.0, 4.0)], Stroke::new(1.2, c));
        }
        13 => {
            p.rect_stroke(r.shrink(1.0), CornerRadius::same(3), s, StrokeKind::Middle);
            p.rect_filled(Rect::from_min_max(r.shrink(1.0).min, pos2(r.center().x, r.shrink(1.0).bottom())), CornerRadius::same(3), c.gamma_multiply(0.6));
        }
        15 => {
            p.rect_stroke(r.shrink2(vec2(0.0, 3.0)), CornerRadius::same(7), s, StrokeKind::Middle);
        }
        16 => {
            p.add(Shape::closed_line(ring(4, 1.0), s));
        }
        17 => {
            p.line_segment([r.center_top() + vec2(-5.0, 2.0), r.center_top() + vec2(5.0, 2.0)], Stroke::new(2.2, c));
            p.line_segment([r.center_top() + vec2(0.0, 2.0), r.center_bottom()], Stroke::new(2.2, c));
            p.line_segment([r.right_top() + vec2(-1.0, 3.0), r.right_bottom() - vec2(1.0, 2.0)], Stroke::new(1.0, c));
        }
        18 => {
            let cc = r.center();
            p.circle_filled(cc + vec2(-3.5, -2.5), 4.5, c);
            p.circle_filled(cc + vec2(3.5, -2.5), 4.5, c);
            p.add(Shape::convex_polygon(vec![cc + vec2(-8.0, -1.0), cc + vec2(8.0, -1.0), cc + vec2(0.0, 8.0)], c, Stroke::NONE));
        }
        19 => {
            let pts: Vec<Pos2> = (0..40).map(|k| {
                let a = k as f32 * 0.32;
                r.center() + vec2(a.cos(), a.sin()) * (1.0 + k as f32 * 0.22)
            }).collect();
            p.add(Shape::line(pts, Stroke::new(1.6, c)));
        }
        20 => {
            let pts: Vec<Pos2> = (0..=24).map(|k| pos2(r.left() + k as f32 / 24.0 * r.width(), r.center().y + (k as f32 / 24.0 * std::f32::consts::TAU * 1.5).sin() * 4.0)).collect();
            p.add(Shape::line(pts, Stroke::new(2.0, c)));
        }
        21 => {
            p.add(Shape::line(vec![r.center() + vec2(-7.0, 0.0), r.center() + vec2(-2.0, 5.0), r.center() + vec2(8.0, -6.0)], Stroke::new(2.4, c)));
        }
        22 => {
            p.line_segment([r.right_top() + vec2(-2.0, 2.0), r.center()], Stroke::new(2.4, c));
            p.circle_filled(r.center() + vec2(-3.0, 3.0), 4.0, c);
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
        viewport: egui::ViewportBuilder::default().with_decorations(false)
            .with_title("Motion")
            .with_icon(Arc::new(studio_ui::app_window_icon("motion")))
            .with_inner_size([1560.0, 960.0])
            .with_min_inner_size([1180.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native("Motion", options, Box::new(|cc| Ok(Box::new(Motion::new(cc)))))
}
