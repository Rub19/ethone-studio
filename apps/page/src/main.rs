#![windows_subsystem = "windows"]

mod model;
mod panels;
mod templates;
mod tools;
mod tutos;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame as EFrame, Key, Layout, Margin, Painter,
    Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2,
    pos2, vec2,
};
use krilla_svg::{SurfaceExt, SvgSettings};
use model::{BLENDS, BULLETS, CORNERS, CharStyle, Doc, FILTERS, Frame, Guide, Kind, LISTS, MASTER, Node, Page, RenderOpts, TextStyle, WRAPS};
use templates::{LOREM, PRESETS, TEMPLATES, starter, template};
use tools::{Drag, Tool, tool_icon, tool_name};
use resvg::{tiny_skia, usvg};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use studio_ui::pro::{self, BLUE, CHROME, FAINT, FIELD, GREEN, INK, INK_STRONG, LINE, MUTED, PANEL, PASTEBOARD, RED, semibold};
use usvg::fontdb;

const FONTS: [&str; 22] = [
    "Segoe UI", "Segoe UI Semibold", "Arial", "Arial Black", "Bahnschrift", "Calibri", "Candara", "Corbel", "Century Gothic", "Franklin Gothic Medium", "Trebuchet MS", "Verdana",
    "Georgia", "Cambria", "Constantia", "Garamond", "Palatino Linotype", "Book Antiqua", "Times New Roman", "Consolas", "Impact", "Comic Sans MS",
];
const STYLES: [(&str, f32, bool, bool, f32); 8] = [
    ("Titre 1", 32.0, true, false, 1.1),
    ("Titre 2", 21.0, true, false, 1.15),
    ("Titre 3", 15.0, true, false, 1.2),
    ("Chapeau", 13.0, false, true, 1.4),
    ("Corps", 10.5, false, false, 1.45),
    ("Légende", 8.0, false, true, 1.3),
    ("Citation", 16.0, false, true, 1.35),
    ("Petit texte", 7.5, false, false, 1.3),
];
const FORMATS: [&str; 4] = ["PDF", "PNG", "JPEG", "SVG"];
const DPIS: [(&str, f32); 5] = [("72", 72.0), ("96", 96.0), ("150", 150.0), ("300", 300.0), ("600", 600.0)];
const PDF_PRESETS: [(&str, &str); 4] = [("Impression haute qualité", "Vectoriel, images d'origine, sans repères"), ("PDF/X (imprimeur)", "Fond perdu, traits de coupe, repères de montage et infos"), ("Web léger", "Images réduites et recompressées pour l'écran"), ("Personnalisé", "Tes propres réglages")];
const FITS: [(&str, &str); 5] = [("Remplir le cadre proportionnellement", "Ctrl+Alt+Maj+E"), ("Ajuster le contenu proportionnellement", "Ctrl+Alt+Maj+C"), ("Ajuster le contenu au cadre", ""), ("Centrer le contenu", "Ctrl+Maj+E"), ("Ajuster le cadre au contenu", "Ctrl+Alt+C")];
const TRACKS: [f32; 8] = [-2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 5.0];
const MM: f32 = 72.0 / 25.4;

struct ExportOpts {
    format: usize,
    preset: usize,
    pages: usize,
    range: String,
    dpi: usize,
    quality: u8,
    marks: bool,
    reg: bool,
    info: bool,
    spreads: bool,
    gray: bool,
    transparent: bool,
    nonprint: bool,
    light: bool,
    open_after: bool,
}

struct Fonts {
    db: Arc<fontdb::Database>,
}

impl Fonts {
    fn width(&self, text: &str, family: &str, bold: bool, size: f32) -> f32 {
        let q = fontdb::Query {
            families: &[fontdb::Family::Name(family), fontdb::Family::SansSerif],
            weight: if bold { fontdb::Weight::BOLD } else { fontdb::Weight::NORMAL },
            ..Default::default()
        };
        self.db
            .query(&q)
            .and_then(|id| {
                self.db.with_face_data(id, |data, idx| {
                    let face = ttf_parser::Face::parse(data, idx).ok()?;
                    let upem = face.units_per_em() as f32;
                    let units: f32 = text.chars().map(|c| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map_or(upem * 0.5, |a| a as f32)).sum();
                    Some(units * size / upem)
                })
            })
            .flatten()
            .unwrap_or(text.chars().count() as f32 * size * 0.5)
    }
}

struct PageApp {
    doc: Doc,
    saved: Doc,
    file: Option<PathBuf>,
    home: bool,
    recents: Vec<PathBuf>,
    undo: Vec<Doc>,
    redo: Vec<Doc>,
    editing: bool,
    page: usize,
    sel: Option<usize>,
    tool: Tool,
    drag: Drag,
    fonts: Fonts,
    tex: Option<TextureHandle>,
    rendered: Option<(Doc, usize, f32, bool)>,
    overflow: Vec<bool>,
    zoom: f32,
    fit_pending: bool,
    snap: bool,
    guides: bool,
    rulers: bool,
    preview: bool,
    snap_lines: Vec<(bool, f32)>,
    new_guide: Option<bool>,
    find: Option<panels::FindState>,
    notice: Option<(String, bool)>,
    export: Option<ExportOpts>,
    doc_sheet: bool,
    edit_text: Option<usize>,
    edit_cursor: (usize, usize),
    style_name: Option<String>,
    clip: Vec<Frame>,
    polygon_sides: u32,
    polygon_inner: f32,
    star_points: u32,
    picks: Vec<usize>,
    pen: Vec<Node>,
    pencil: Vec<[f32; 2]>,
    measure_line: Option<([f32; 2], [f32; 2])>,
    grad_line: Option<([f32; 2], [f32; 2])>,
    layer: usize,
    note_edit: Option<usize>,
    tab: usize,
    show_notes: bool,
    edges: bool,
    step: Option<panels::StepOpts>,
    toc: Option<panels::TocOpts>,
    sections_sheet: bool,
    char_sheet: Option<CharStyle>,
    obj_name: Option<String>,
    preflight: Option<(Doc, Vec<panels::Issue>)>,
    families: std::collections::HashSet<String>,
    title: String,
    icon: TextureHandle,
    help: studio_ui::help::Help,
    inbox: studio_ui::handoff::Receiver,
    palette: studio_ui::palette::Palette,
}

fn recents_path() -> PathBuf {
    studio_core::data_dir().join("page").join("recents.txt")
}

impl PageApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        pro::apply(&cc.egui_ctx);
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let families = db.faces().flat_map(|f| f.families.iter().map(|(n, _)| n.clone())).collect();
        let doc = starter(595.0, 842.0);
        let recents = std::fs::read_to_string(recents_path()).map(|s| s.lines().map(PathBuf::from).filter(|p| p.exists()).collect()).unwrap_or_default();
        let mut me = Self {
            saved: doc.clone(),
            doc,
            file: None,
            home: true,
            recents,
            undo: Vec::new(),
            redo: Vec::new(),
            editing: false,
            page: 0,
            sel: None,
            tool: Tool::Select,
            drag: Drag::None,
            fonts: Fonts { db: Arc::new(db) },
            tex: None,
            rendered: None,
            overflow: Vec::new(),
            zoom: 1.0,
            fit_pending: true,
            snap: true,
            guides: true,
            rulers: true,
            preview: false,
            snap_lines: Vec::new(),
            new_guide: None,
            find: None,
            notice: None,
            export: None,
            doc_sheet: false,
            edit_text: None,
            edit_cursor: (0, 0),
            style_name: None,
            clip: Vec::new(),
            polygon_sides: 6,
            polygon_inner: 0.5,
            star_points: 5,
            picks: Vec::new(),
            pen: Vec::new(),
            pencil: Vec::new(),
            measure_line: None,
            grad_line: None,
            layer: 0,
            note_edit: None,
            tab: 0,
            show_notes: true,
            edges: true,
            step: None,
            toc: None,
            sections_sheet: false,
            char_sheet: None,
            obj_name: None,
            preflight: None,
            families,
            title: String::new(),
            icon: studio_ui::icon_texture(&cc.egui_ctx, "page"),
            help: studio_ui::help::Help::new("Page", tutos::PAGE),
            inbox: studio_ui::handoff::Receiver::new("page"),
            palette: studio_ui::palette::Palette::new(&cc.egui_ctx),
        };
        if let Some(p) = std::env::args().nth(1).map(PathBuf::from).filter(|p| p.is_file()) {
            me.receive(&p);
        }
        me
    }

    fn set_doc(&mut self, doc: Doc, file: Option<PathBuf>) {
        self.saved = doc.clone();
        self.doc = doc;
        self.file = file;
        self.page = 0;
        self.sel = None;
        self.edit_text = None;
        self.undo.clear();
        self.redo.clear();
        self.fit_pending = true;
        self.home = false;
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

    fn measure(&self) -> impl Fn(&str, &str, bool, f32) -> f32 + '_ {
        |t: &str, f: &str, b: bool, s: f32| self.fonts.width(t, f, b, s)
    }

    fn opts(&self) -> usvg::Options<'static> {
        usvg::Options { fontdb: self.fonts.db.clone(), ..Default::default() }
    }

    fn render_opts(&self, o: &ExportOpts) -> RenderOpts {
        let name = self.file.as_ref().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("document".into());
        RenderOpts { guides: false, marks: o.marks && self.doc.bleed >= 0.0, reg: o.reg, info: o.info.then(|| format!("{name} · {}", model::today())), print: !o.nonprint, transparent: o.transparent && o.format == 1, gray: o.gray }
    }

    fn export_svg(&self, doc: &Doc, group: &[usize], ro: &RenderOpts) -> String {
        if group.len() > 1 { doc.spread_svg(group, &self.measure(), ro) } else { doc.to_svg_opts(group[0], &self.measure(), ro).0 }
    }

    fn raster_svg(&self, svg: &str, dpi: f32) -> Result<tiny_skia::Pixmap, String> {
        let tree = usvg::Tree::from_str(svg, &self.opts()).map_err(|e| e.to_string())?;
        let scale = dpi / 72.0;
        let size = tree.size();
        let mut pix = tiny_skia::Pixmap::new((size.width() * scale).ceil() as u32, (size.height() * scale).ceil() as u32).ok_or("taille")?;
        resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
        Ok(pix)
    }

    fn default_export(&self) -> ExportOpts {
        ExportOpts { format: 0, preset: 0, pages: 0, range: format!("1-{}", self.doc.pages.len()), dpi: 3, quality: 90, marks: false, reg: false, info: false, spreads: false, gray: false, transparent: false, nonprint: false, light: false, open_after: true }
    }

    fn apply_preset(o: &mut ExportOpts) {
        match o.preset {
            0 => (o.marks, o.reg, o.info, o.light) = (false, false, false, false),
            1 => (o.marks, o.reg, o.info, o.light) = (true, true, true, false),
            2 => (o.marks, o.reg, o.info, o.light) = (false, false, false, true),
            _ => {}
        }
    }

    fn light_doc(&self) -> Doc {
        let mut doc = self.doc.clone();
        let dir = std::env::temp_dir().join("ethone-page-web");
        let _ = std::fs::create_dir_all(&dir);
        for f in doc.pages.iter_mut().flat_map(|p| p.frames.iter_mut()).chain(doc.master.frames.iter_mut()) {
            if let Kind::Image { path, .. } = &mut f.kind
                && !path.is_empty()
                && let Ok(img) = image::open(&*path)
            {
                let small = img.thumbnail(1600, 1600).to_rgb8();
                let out = dir.join(format!("{:x}.jpg", path.bytes().fold(1469598103934665603u64, |h, b| (h ^ b as u64).wrapping_mul(1099511628211))));
                if let Ok(file) = std::fs::File::create(&out) {
                    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), 75);
                    if enc.encode_image(&small).is_ok() {
                        *path = out.to_string_lossy().into_owned();
                    }
                }
            }
        }
        doc
    }

    fn run_export(&mut self, o: &ExportOpts) {
        let n = self.doc.pages.len();
        let cur = if self.page == MASTER { 0 } else { self.page };
        let list: Vec<usize> = match o.pages {
            1 => vec![cur],
            2 => panels::parse_range(&o.range, n),
            _ => (0..n).collect(),
        };
        if list.is_empty() {
            self.notice = Some(("Aucune page à exporter : vérifie la plage".into(), false));
            return;
        }
        let groups: Vec<Vec<usize>> = if o.spreads && self.doc.facing {
            let mut g: Vec<Vec<usize>> = Vec::new();
            for &i in &list {
                match g.last_mut() {
                    Some(last) if i % 2 == 0 && i > 0 && last.len() == 1 && last[0] + 1 == i && last[0] % 2 == 1 => last.push(i),
                    _ => g.push(vec![i]),
                }
            }
            g
        } else {
            list.iter().map(|i| vec![*i]).collect()
        };
        let ext = ["pdf", "png", "jpg", "svg"][o.format];
        let base = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("document".into());
        let single = o.format == 0 || groups.len() == 1;
        let target = if single {
            rfd::FileDialog::new().add_filter(FORMATS[o.format], &[ext]).set_file_name(format!("{base}.{ext}")).save_file()
        } else {
            rfd::FileDialog::new().set_title("Dossier d'export").pick_folder()
        };
        let Some(target) = target else { return };
        let ro = self.render_opts(o);
        let doc = if o.light && o.format == 0 { self.light_doc() } else { self.doc.clone() };
        let dpi = DPIS[o.dpi].1;
        let result = (|| -> Result<PathBuf, String> {
            match o.format {
                0 => {
                    let opt = self.opts();
                    let mut document = krilla::Document::new();
                    for g in &groups {
                        let svg = self.export_svg(&doc, g, &ro);
                        let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| e.to_string())?;
                        let size = krilla::geom::Size::from_wh(tree.size().width(), tree.size().height()).ok_or("taille invalide")?;
                        let mut page = document.start_page_with(krilla::page::PageSettings::new(size));
                        let mut surface = page.surface();
                        surface.draw_svg(&tree, size, SvgSettings::default()).ok_or("rendu SVG impossible")?;
                        surface.finish();
                        page.finish();
                    }
                    let bytes = document.finish().map_err(|e| format!("{e:?}"))?;
                    std::fs::write(&target, bytes).map_err(|e| e.to_string())?;
                    Ok(target.clone())
                }
                f => {
                    for g in &groups {
                        let path = if single { target.clone() } else { target.join(format!("{base}-{}.{ext}", g.iter().map(|i| doc.page_label(*i)).collect::<Vec<_>>().join("-"))) };
                        let svg = self.export_svg(&doc, g, &ro);
                        match f {
                            3 => std::fs::write(&path, svg).map_err(|e| e.to_string())?,
                            1 => self.raster_svg(&svg, dpi)?.save_png(&path).map_err(|e| e.to_string())?,
                            _ => {
                                let pix = self.raster_svg(&svg, dpi)?;
                                let mut rgb = image::RgbImage::new(pix.width(), pix.height());
                                for (d, s) in rgb.pixels_mut().zip(pix.pixels()) {
                                    let c = s.demultiply();
                                    let a = c.alpha() as u32;
                                    let mix = |v: u8| ((v as u32 * a + 255 * (255 - a)) / 255) as u8;
                                    d.0 = [mix(c.red()), mix(c.green()), mix(c.blue())];
                                }
                                let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
                                let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), o.quality);
                                enc.encode_image(&rgb).map_err(|e| e.to_string())?;
                            }
                        }
                    }
                    Ok(target.clone())
                }
            }
        })();
        self.notice = Some(match result {
            Ok(p) => {
                if o.open_after {
                    let _ = std::process::Command::new("explorer").arg(&p).spawn();
                }
                (format!("{} exporté : {}", FORMATS[o.format], p.display()), true)
            }
            Err(e) => (format!("Export {} : {e}", FORMATS[o.format]), false),
        });
    }

    fn package(&mut self) {
        let Some(dir) = rfd::FileDialog::new().set_title("Dossier où assembler le document").pick_folder() else { return };
        let base = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("document".into());
        let root = dir.join(format!("{base} dossier"));
        let links = root.join("Liens");
        let result = (|| -> Result<(usize, usize), String> {
            std::fs::create_dir_all(&links).map_err(|e| e.to_string())?;
            let mut doc = self.doc.clone();
            let (mut ok, mut missing) = (0, 0);
            let mut fonts = std::collections::BTreeSet::new();
            for f in doc.pages.iter_mut().flat_map(|p| p.frames.iter_mut()).chain(doc.master.frames.iter_mut()) {
                match &mut f.kind {
                    Kind::Image { path, .. } if !path.is_empty() => {
                        let src = PathBuf::from(&*path);
                        match src.file_name() {
                            Some(name) if src.is_file() => {
                                let dst = links.join(name);
                                std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
                                *path = dst.to_string_lossy().into_owned();
                                ok += 1;
                            }
                            _ => missing += 1,
                        }
                    }
                    Kind::Text { family, .. } | Kind::Table { family, .. } | Kind::TextPath { family, .. } => {
                        fonts.insert(family.clone());
                    }
                    _ => {}
                }
            }
            std::fs::write(root.join(format!("{base}.epage")), serde_json::to_vec_pretty(&doc).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            let report = format!("Assemblage de {base}\n\nPages : {}\nImages copiées : {ok}\nImages manquantes : {missing}\nPolices utilisées :\n{}\n", doc.pages.len(), fonts.iter().map(|f| format!("  - {f}")).collect::<Vec<_>>().join("\n"));
            std::fs::write(root.join("Instructions.txt"), report).map_err(|e| e.to_string())?;
            Ok((ok, missing))
        })();
        self.notice = Some(match result {
            Ok((ok, missing)) => {
                let _ = std::process::Command::new("explorer").arg(&root).spawn();
                (format!("Assemblé dans {} : {ok} image(s), {missing} manquante(s)", root.display()), missing == 0)
            }
            Err(e) => (format!("Assemblage : {e}"), false),
        });
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.file, as_new) {
            (Some(p), false) => p.clone(),
            _ => match rfd::FileDialog::new().add_filter("Projet Page", &["epage"]).set_file_name("document.epage").save_file() {
                Some(p) => p,
                None => return,
            },
        };
        match serde_json::to_vec_pretty(&self.doc).map_err(|e| e.to_string()).and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string())) {
            Ok(()) => {
                self.remember(&path);
                self.file = Some(path);
                self.saved = self.doc.clone();
            }
            Err(e) => self.notice = Some((format!("Enregistrement : {e}"), false)),
        }
    }

    fn open_path(&mut self, p: &Path) {
        match std::fs::read(p).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Doc>(b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&b)).map_err(|e| e.to_string())) {
            Ok(d) => {
                self.set_doc(d, Some(p.to_path_buf()));
                self.remember(p);
            }
            Err(e) => self.notice = Some((format!("Ouverture : {e}"), false)),
        }
    }

    fn open(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("Projet Page", &["epage"]).pick_file() {
            self.open_path(&p);
        }
    }

    fn frames(&mut self) -> &mut Vec<Frame> {
        if self.page == MASTER { &mut self.doc.master.frames } else { &mut self.doc.pages[self.page].frames }
    }

    fn frames_ref(&self) -> &Vec<Frame> {
        if self.page == MASTER { &self.doc.master.frames } else { &self.doc.pages[self.page].frames }
    }

    fn push_frame(&mut self, f: Frame) {
        self.frames().push(f);
        self.sel = Some(self.frames().len() - 1);
    }

    fn remove_frame(&mut self, i: usize) {
        let fr = self.frames();
        if i >= fr.len() {
            return;
        }
        fr.remove(i);
        for f in fr.iter_mut() {
            if let Kind::Text { link, .. } = &mut f.kind {
                *link = match *link {
                    Some(l) if l == i => None,
                    Some(l) if l > i => Some(l - 1),
                    l => l,
                };
            }
        }
        self.sel = None;
        self.edit_text = None;
    }

    fn duplicate(&mut self) {
        self.copy_sel();
        self.paste(false);
    }

    fn paste(&mut self, in_place: bool) {
        if self.clip.is_empty() {
            return;
        }
        let base = self.frames_ref().len();
        for mut f in self.clip.clone() {
            if !in_place {
                f.x += 12.0;
                f.y += 12.0;
            }
            if let Kind::Text { link, .. } = &mut f.kind {
                *link = None;
            }
            self.frames().push(f);
        }
        let n = self.frames_ref().len();
        self.sel = Some(n - 1);
        self.picks = if n - base > 1 { (base..n).collect() } else { Vec::new() };
    }

    fn order(&mut self, to: i32) {
        let Some(i) = self.sel else { return };
        let n = self.frames().len();
        let j = match to {
            i32::MAX => n - 1,
            i32::MIN => 0,
            d => (i as i32 + d).clamp(0, n as i32 - 1) as usize,
        };
        if i == j {
            return;
        }
        let mut order: Vec<usize> = (0..n).collect();
        let x = order.remove(i);
        order.insert(j, x);
        let new_of: Vec<usize> = (0..n).map(|old| order.iter().position(|o| *o == old).unwrap()).collect();
        let old = std::mem::take(self.frames());
        let mut slots: Vec<Option<Frame>> = old.into_iter().map(Some).collect();
        let mut fresh: Vec<Frame> = order.iter().map(|o| slots[*o].take().unwrap()).collect();
        for f in &mut fresh {
            if let Kind::Text { link: Some(l), .. } = &mut f.kind {
                *l = new_of[*l];
            }
        }
        *self.frames() = fresh;
        self.sel = Some(j);
    }

    fn align(&mut self, axis: usize, to: f32, page: bool) {
        let Some(i) = self.sel else { return };
        let (m, pw, ph) = (if page { 0.0 } else { self.doc.margin }, self.doc.width, self.doc.height);
        let f = &mut self.frames()[i];
        let (size, span) = if axis == 0 { (f.w, pw) } else { (f.h, ph) };
        let v = m + (span - 2.0 * m - size) * to;
        if axis == 0 { f.x = v } else { f.y = v }
    }

    fn fit_text(&mut self) {
        let Some(i) = self.sel else { return };
        let f = self.frames_ref()[i].clone();
        let h = self.doc.fit_height(&f, &self.measure());
        if let Some(h) = h {
            self.frames()[i].h = h.ceil();
        }
    }

    fn fit_frame_to_image(&mut self) {
        let Some(i) = self.sel else { return };
        let Kind::Image { path, .. } = &self.frames_ref()[i].kind else { return };
        if let Ok((w, h)) = image::image_dimensions(path) {
            let f = &mut self.frames()[i];
            f.h = f.w * h as f32 / w.max(1) as f32;
        }
    }

    fn add_page(&mut self, at: usize, copy: bool) {
        let p = if copy && self.page != MASTER {
            let mut p = self.doc.pages[self.page].clone();
            for f in &mut p.frames {
                if let Kind::Text { link, .. } = &mut f.kind {
                    *link = None;
                }
            }
            p
        } else {
            Page::default()
        };
        let at = at.min(self.doc.pages.len());
        self.doc.pages.insert(at, p);
        self.page = at;
        self.sel = None;
    }

    fn move_page(&mut self, by: i32) {
        if self.page == MASTER {
            return;
        }
        let to = self.page as i32 + by;
        if to >= 0 && (to as usize) < self.doc.pages.len() {
            self.doc.pages.swap(self.page, to as usize);
            self.page = to as usize;
        }
    }

    fn delete_page(&mut self, i: usize) {
        if self.doc.pages.len() > 1 && i < self.doc.pages.len() {
            self.doc.pages.remove(i);
            self.page = self.page.min(self.doc.pages.len() - 1);
            self.sel = None;
        }
    }

    fn word_count(&self) -> usize {
        self.doc.pages.iter().flat_map(|p| p.frames.iter()).map(|f| match &f.kind {
            Kind::Text { text, .. } => text.split_whitespace().count(),
            Kind::Table { cells, .. } => cells.iter().flatten().map(|c| c.split_whitespace().count()).sum(),
            _ => 0,
        }).sum()
    }

    fn apply_style(&mut self, k: usize) {
        let user = self.doc.styles.clone();
        for i in self.selection() {
            if let Kind::Text { size, bold, italic, leading, family, color, tracking, para_space, align, indent, upper, opts, .. } = &mut self.frames()[i].kind {
                if k < STYLES.len() {
                    let (name, sz, b, it, lead) = STYLES[k];
                    (*size, *bold, *italic, *leading) = (sz, b, it, lead);
                    opts.style = name.into();
                } else if let Some(s) = user.get(k - STYLES.len()) {
                    (*size, *bold, *italic, *leading, *tracking, *para_space, *align, *indent, *upper) = (s.size, s.bold, s.italic, s.leading, s.tracking, s.para_space, s.align, s.indent, s.upper);
                    opts.space_before = s.space_before;
                    *family = s.family.clone();
                    *color = s.color;
                    opts.style = s.name.clone();
                }
            }
        }
    }

    fn save_style(&mut self, name: String) {
        let Some(i) = self.sel else { return };
        if let Kind::Text { size, bold, italic, leading, family, color, tracking, para_space, align, indent, upper, opts, .. } = &self.frames_ref()[i].kind {
            let st = TextStyle { name, size: *size, family: family.clone(), bold: *bold, italic: *italic, leading: *leading, tracking: *tracking, color: *color, para_space: *para_space, align: *align, indent: *indent, upper: *upper, space_before: opts.space_before };
            self.doc.styles.retain(|s| s.name != st.name);
            let name = st.name.clone();
            self.doc.styles.push(st);
            if let Kind::Text { opts, .. } = &mut self.frames()[i].kind {
                opts.style = name;
            }
        }
    }

    fn copy_style(&mut self, from: usize, to: usize) {
        let src = self.frames_ref()[from].clone();
        panels::copy_look(&src, &mut self.frames()[to]);
    }

    fn undo_step(&mut self) {
        if let Some(d) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, d));
        }
    }

    fn redo_step(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, d));
        }
    }

    fn cur_size(&self) -> (f32, f32) {
        if self.page == MASTER { (self.doc.width, self.doc.height) } else { self.doc.page_size(self.page) }
    }

    fn copy_sel(&mut self) {
        self.clip = self.selection().into_iter().filter_map(|i| self.frames_ref().get(i).cloned()).collect();
    }

    fn text_sel(&mut self, f: impl Fn(&mut Kind)) {
        for i in self.selection() {
            f(&mut self.frames()[i].kind);
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            if self.edit_text.is_some() && ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.edit_text = None;
            }
            return;
        }
        use egui::{KeyboardShortcut as K, Modifiers as M};
        let sc = |m, k| ctx.input_mut(|i| i.consume_shortcut(&K::new(m, k)));
        if sc(M::COMMAND, Key::N) {
            self.home = true;
        }
        if sc(M::COMMAND, Key::O) {
            self.open();
        }
        if self.home {
            return;
        }
        let ca = M::COMMAND | M::ALT;
        let cs = M::COMMAND | M::SHIFT;
        let cas = M::COMMAND | M::ALT | M::SHIFT;
        if sc(cs, Key::Z) || sc(M::COMMAND, Key::Y) {
            self.redo_step();
        } else if sc(M::COMMAND, Key::Z) {
            self.undo_step();
        }
        if sc(cs, Key::S) {
            self.save(true);
        } else if sc(M::COMMAND, Key::S) {
            self.save(false);
        }
        if sc(M::COMMAND, Key::E) || sc(M::COMMAND, Key::P) {
            self.export = Some(self.default_export());
        }
        if sc(M::COMMAND, Key::K) {
            self.doc_sheet = true;
        }
        if sc(cs, Key::I) && self.sel.is_none_or(|i| !matches!(self.frames_ref()[i].kind, Kind::Text { .. })) {
            self.place_image();
        }
        if sc(ca, Key::U) && self.sel.is_some() {
            self.step = Some(panels::StepOpts { count: 3, dx: 20.0, dy: 20.0, rot: 0.0, grid: false, rows: 2, cols: 2 });
        }
        if sc(cas, Key::C) {
            self.fitting(1);
        } else if sc(ca, Key::C) {
            self.fitting(4);
        }
        if sc(cas, Key::E) {
            self.fitting(0);
        } else if sc(cs, Key::E) {
            self.fitting(3);
        }
        if sc(cs, Key::G) {
            self.group(false);
        } else if sc(M::COMMAND, Key::G) {
            self.group(true);
        }
        if sc(cs, Key::A) {
            self.sel = None;
            self.picks.clear();
        } else if sc(M::COMMAND, Key::A) {
            self.select_all();
        }
        if sc(ca, Key::Num3) {
            self.frames().iter_mut().for_each(|f| f.hidden = false);
        } else if sc(M::COMMAND, Key::Num3) {
            for i in self.selection() {
                self.frames()[i].hidden = true;
            }
            self.sel = None;
            self.picks.clear();
        }
        if sc(M::COMMAND, Key::D) {
            self.duplicate();
        }
        if sc(M::COMMAND, Key::C) {
            self.copy_sel();
        }
        if sc(M::COMMAND, Key::X) {
            self.copy_sel();
            self.delete_selection();
        }
        if sc(cs, Key::V) {
            self.paste(true);
        } else if sc(M::COMMAND, Key::V) {
            self.paste(false);
        }
        if sc(M::COMMAND, Key::F) {
            self.find = Some(panels::FindState::new(String::new()));
        }
        if sc(M::COMMAND, Key::L) {
            for i in self.selection() {
                self.frames()[i].locked = true;
            }
            self.sel = None;
            self.picks.clear();
        }
        if sc(M::COMMAND, Key::R) {
            self.rulers = !self.rulers;
        }
        if sc(M::COMMAND, Key::H) {
            self.edges = !self.edges;
        }
        if sc(M::COMMAND, Key::Semicolon) {
            self.guides = !self.guides;
        }
        if sc(cs, Key::CloseBracket) {
            self.order(i32::MAX);
        } else if sc(M::COMMAND, Key::CloseBracket) {
            self.order(1);
        }
        if sc(cs, Key::OpenBracket) {
            self.order(i32::MIN);
        } else if sc(M::COMMAND, Key::OpenBracket) {
            self.order(-1);
        }
        for (k, v) in [(Key::B, 0usize), (Key::Slash, 1), (Key::K, 2), (Key::H, 3)] {
            if sc(cs, k) {
                self.text_sel(|kind| {
                    if let Kind::Text { bold, upper, opts, .. } = kind {
                        match v {
                            0 => *bold = !*bold,
                            1 => opts.strike = !opts.strike,
                            2 => *upper = !*upper,
                            _ => opts.small_caps = !opts.small_caps,
                        }
                    }
                });
            }
        }
        if sc(cs, Key::U) {
            self.text_sel(|kind| {
                if let Kind::Text { underline, .. } = kind {
                    *underline = !*underline;
                }
            });
        }
        for (k, a) in [(Key::L, 0u8), (Key::C, 1), (Key::R, 2), (Key::J, 3), (Key::F, 4)] {
            if sc(cs, k) {
                self.text_sel(|kind| {
                    if let Kind::Text { align, .. } = kind {
                        *align = a;
                    }
                });
            }
        }
        for (k, d) in [(Key::Period, 2.0f32), (Key::Comma, -2.0)] {
            if sc(cs, k) {
                self.text_sel(|kind| {
                    if let Kind::Text { size, .. } | Kind::TextPath { size, .. } | Kind::Table { size, .. } = kind {
                        *size = (*size + d).clamp(4.0, 400.0);
                    }
                });
            }
        }
        for (k, dt, dl) in [(Key::ArrowLeft, -0.5f32, 0.0f32), (Key::ArrowRight, 0.5, 0.0), (Key::ArrowUp, 0.0, -0.05), (Key::ArrowDown, 0.0, 0.05)] {
            if sc(M::ALT, k) {
                self.text_sel(|kind| {
                    if let Kind::Text { tracking, leading, .. } = kind {
                        *tracking = (*tracking + dt).clamp(-5.0, 50.0);
                        *leading = (*leading + dl).clamp(0.6, 4.0);
                    }
                });
            }
        }
        if sc(M::COMMAND, Key::Num0) {
            self.fit_pending = true;
        }
        if sc(M::COMMAND, Key::Num1) {
            self.zoom = 1.0;
        }
        if sc(M::COMMAND, Key::Num2) {
            self.zoom = 2.0;
        }
        if sc(M::COMMAND, Key::Num5) {
            self.zoom = 0.5;
        }
        if sc(M::COMMAND, Key::Plus) || sc(M::COMMAND, Key::Equals) {
            self.zoom = (self.zoom * 1.25).min(8.0);
        }
        if sc(M::COMMAND, Key::Minus) {
            self.zoom = (self.zoom / 1.25).max(0.05);
        }
        if sc(M::NONE, Key::F7) {
            self.tab = if self.tab == 1 { 0 } else { 1 };
        }
        if sc(cs, Key::D) {
            self.tab = if self.tab == 2 { 0 } else { 2 };
        }
        if sc(cas, Key::F) {
            self.tab = if self.tab == 3 { 0 } else { 3 };
        }
        let (del, pressed, nudge, pg, w_key, enter, esc, home_end) = ctx.input(|i| {
            let mut pressed = Vec::new();
            if !i.modifiers.command && !i.modifiers.alt {
                for e in &i.events {
                    if let egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } = e {
                        pressed.push((*key, modifiers.shift));
                    }
                }
            }
            let step = if i.modifiers.shift { 10.0 } else { 1.0 };
            let mut n = [0.0, 0.0];
            if !i.modifiers.alt {
                if i.key_pressed(Key::ArrowLeft) {
                    n[0] -= step;
                }
                if i.key_pressed(Key::ArrowRight) {
                    n[0] += step;
                }
                if i.key_pressed(Key::ArrowUp) {
                    n[1] -= step;
                }
                if i.key_pressed(Key::ArrowDown) {
                    n[1] += step;
                }
            }
            let pg = if i.key_pressed(Key::PageDown) { 1 } else if i.key_pressed(Key::PageUp) { -1 } else { 0 };
            let he = if i.key_pressed(Key::Home) { -1 } else if i.key_pressed(Key::End) { 1 } else { 0 };
            (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace), pressed, n, pg, i.key_pressed(Key::W) && !i.modifiers.command && !i.modifiers.shift, i.key_pressed(Key::Enter), i.key_pressed(Key::Escape), he)
        });
        if let Some(t) = tools::pick_tool(self.tool, &pressed) {
            if self.tool == Tool::Pen && t != Tool::Pen {
                self.commit_pen(false);
            }
            self.tool = t;
        }
        if w_key {
            self.preview = !self.preview;
        }
        if esc {
            if !self.pen.is_empty() {
                self.commit_pen(false);
            } else {
                self.edit_text = None;
                self.sel = None;
                self.picks.clear();
                self.measure_line = None;
                self.tool = Tool::Select;
            }
        }
        if enter {
            if !self.pen.is_empty() {
                self.commit_pen(false);
            } else if let Some(i) = self.sel.filter(|i| matches!(self.frames_ref().get(*i).map(|f| &f.kind), Some(Kind::Text { .. } | Kind::TextPath { .. }))) {
                self.edit_text = Some(i);
            }
        }
        if del && self.pen.pop().is_none() && self.sel.is_some() {
            self.delete_selection();
        }
        if nudge != [0.0, 0.0] {
            for i in self.selection() {
                let f = &mut self.frames()[i];
                f.x += nudge[0];
                f.y += nudge[1];
            }
        }
        if (pg != 0 || home_end != 0) && self.page != MASTER {
            let last = self.doc.pages.len() as i32 - 1;
            self.page = match home_end {
                -1 => 0,
                1 => last as usize,
                _ => (self.page as i32 + pg).clamp(0, last) as usize,
            };
            self.sel = None;
            self.picks.clear();
        }
        if self.page != MASTER {
            self.page = self.page.min(self.doc.pages.len() - 1);
        }
        let n = self.frames_ref().len();
        if self.sel.is_some_and(|i| i >= n) {
            self.sel = None;
        }
        self.picks.retain(|i| *i < n);
        if self.sel.is_none() {
            self.picks.clear();
        }
        if self.edit_text.is_some_and(|i| i >= n) {
            self.edit_text = None;
        }
        if self.note_edit.is_some_and(|i| i >= n) {
            self.note_edit = None;
        }
    }
}

impl PageApp {

    fn commands(&self) -> Vec<studio_ui::palette::Cmd> {
        let mut v = vec![

        ];
        v.push(studio_ui::palette::Cmd::new("Fusion de données (CSV)…", "Publipostage : «Nom» dans les textes", "data-merge"));
        v.extend(studio_ui::palette::send_cmds(&[("pixel", "retoucher la page"), ("lumen", "développer"), ("vector", "placer la page"), ("cut", "ajouter au montage"), ("motion", "animer la page")]));
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
            let i = self.sel.filter(|i| matches!(self.frames_ref().get(*i).map(|f| &f.kind), Some(Kind::Image { .. })));
            match i {
                Some(i) => self.frames()[i].effects.push(studio_fx::stack::Fx::new(k)),
                None => studio_ui::handoff::toast(ctx, "Sélectionne d'abord un bloc image".into(), false),
            }
            return;
        }
        if id == "data-merge" {
            self.data_merge(ctx);
        }
    }

    fn data_merge(&mut self, ctx: &egui::Context) {
        let fields = model::merge_fields(&self.doc);
        if fields.is_empty() {
            studio_ui::handoff::toast(ctx, "Écris d'abord des champs dans tes textes, par ex. «Nom» ou <<Ville>>, puis relance la fusion".into(), false);
            return;
        }
        let Some(p) = rfd::FileDialog::new().add_filter("Tableau CSV", &["csv", "tsv", "txt"]).pick_file() else { return };
        let text = match std::fs::read(&p) {
            Ok(b) => String::from_utf8(b.clone()).unwrap_or_else(|_| b.iter().map(|c| *c as char).collect()),
            Err(e) => return studio_ui::handoff::toast(ctx, e.to_string(), false),
        };
        let (head, rows) = model::parse_csv(&text);
        let missing: Vec<&String> = fields.iter().filter(|f| !head.contains(f)).collect();
        if rows.is_empty() || head.is_empty() {
            return studio_ui::handoff::toast(ctx, "Le fichier ne contient pas de lignes".into(), false);
        }
        let merged = model::data_merge(&self.doc, &head, &rows);
        let n = rows.len();
        self.doc = merged;
        self.page = 0;
        let warn = if missing.is_empty() { String::new() } else { format!(" · colonne(s) absente(s) : {}", missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")) };
        studio_ui::handoff::toast(ctx, format!("{n} fiche(s) générée(s), {} page(s){warn}", self.doc.pages.len()), missing.is_empty());
    }

    fn send_to(&mut self, target: &str, ctx: &egui::Context) {
        let cur = if self.page == MASTER { 0 } else { self.page.min(self.doc.pages.len().saturating_sub(1)) };
        let svg = self.export_svg(&self.doc, &[cur], &RenderOpts::screen(false));
        let p = studio_core::handoff::outgoing("page", &format!("page {}", cur + 1), "png");
        match self.raster_svg(&svg, 150.0).and_then(|pix| pix.save_png(&p).map_err(|e| e.to_string())) {
            Ok(()) => studio_ui::handoff::send(ctx, target, &p),
            Err(e) => studio_ui::handoff::toast(ctx, format!("Envoi impossible : {e}"), false),
        }
    }

    fn receive(&mut self, p: &Path) {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ["png", "jpg", "jpeg", "webp", "gif", "svg"].contains(&ext.as_str()) {
            self.place_image_path(p);
        } else {
            self.open_path(p);
        }
    }
}

impl eframe::App for PageApp {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        studio_ui::chrome::resize(ctx);
        for p in self.inbox.poll(ctx) {
            self.receive(&p);
        }
        let before = self.doc.clone();
        self.palette.keys(ctx);
        if self.palette.open {
            let cmds = self.commands();
            if let Some(id) = self.palette.show(ctx, &mut self.help, cmds) {
                self.run_cmd(&id, ctx);
            }
        }
        self.keys(ctx);
        let name = self.file.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or("Sans titre".into());
        let title = if self.home { "Page".to_string() } else { format!("{name}{} — Page", if self.doc != self.saved { " •" } else { "" }) };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.top_bar(root);
        if self.home {
            self.home_screen(root);
        } else {
            self.options_bar(root);
            self.status(root);
            self.tool_rail(root);
            self.pages_panel(root);
            self.props(root);
            self.canvas(root, &name);
            self.text_editor(ctx);
        }
        self.sheets(ctx);
        self.help.show(ctx);
        if !self.home && self.doc != before && !self.editing {
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

impl PageApp {
    fn top_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("menu").exact_size(44.0).frame(EFrame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            let full = studio_ui::chrome::bar(ui, 12.0);
            let right_w = if self.home { 0.0 } else { 300.0 };
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
                        if pro::primary_icon(ui, "Exporter", export_glyph).on_hover_text("PDF, PNG, JPEG ou SVG · Ctrl+E").clicked() {
                            self.export = Some(self.default_export());
                        }
                        ui.add_space(4.0);
                        let fmt = PRESETS.iter().find(|p| (p.2 - self.doc.width).abs() < 0.5 && (p.3 - self.doc.height).abs() < 0.5).map_or(format!("{:.0} × {:.0} mm", self.doc.width / MM, self.doc.height / MM), |p| p.0.to_string());
                        let n = self.doc.pages.len();
                        if pro::chip(ui, &format!("{fmt} · {n} page{}", if n > 1 { "s" } else { "" })).on_hover_text("Format du document · Ctrl+K").clicked() {
                            self.doc_sheet = true;
                        }
                    });
                });
            }
        });
    }

    fn menus(&mut self, ui: &mut Ui) {
        ui.menu_button("Fichier", |ui| {
            if let Some(t) = studio_ui::handoff::menu(ui, &[("pixel", "retoucher la page"), ("lumen", "développer"), ("vector", "placer la page"), ("cut", "ajouter au montage"), ("motion", "animer la page")]) {
                let ctx = ui.ctx().clone();
                self.send_to(t, &ctx);
            }
            ui.separator();
            if mi(ui, true, "Nouveau…", "Ctrl+N") {
                self.home = true;
            }
            if mi(ui, true, "Ouvrir…", "Ctrl+O") {
                self.open();
            }
            ui.menu_button("Ouvrir un document récent", |ui| {
                let mut open = None;
                for p in &self.recents {
                    if ui.button(p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).clicked() {
                        open = Some(p.clone());
                    }
                }
                if self.recents.is_empty() {
                    ui.label(RichText::new("Aucun document récent").color(FAINT));
                }
                if let Some(p) = open {
                    self.open_path(&p);
                }
            });
            ui.add_enabled_ui(!self.home, |ui| {
                ui.separator();
                if mi(ui, true, "Enregistrer", "Ctrl+S") {
                    self.save(false);
                }
                if mi(ui, true, "Enregistrer sous…", "Ctrl+Maj+S") {
                    self.save(true);
                }
                if mi(ui, self.file.is_some() && self.doc != self.saved, "Revenir à la version enregistrée", "") {
                    self.doc = self.saved.clone();
                    self.sel = None;
                }
                ui.separator();
                if mi(ui, true, "Placer une image…", "Ctrl+Maj+I") {
                    self.place_image();
                }
                if mi(ui, true, "Assembler…", "") {
                    self.package();
                }
                ui.separator();
                if mi(ui, true, "Exporter…", "Ctrl+E") {
                    self.export = Some(self.default_export());
                }
                ui.menu_button("Exporter avec un préréglage", |ui| {
                    for (k, (name, _)) in PDF_PRESETS.iter().enumerate().take(3) {
                        if ui.button(format!("PDF · {name}")).clicked() {
                            let mut o = self.default_export();
                            o.preset = k;
                            Self::apply_preset(&mut o);
                            self.export = Some(o);
                        }
                    }
                    for (k, name) in [(1usize, "PNG 300 ppp"), (2, "JPEG 150 ppp"), (3, "SVG")] {
                        if ui.button(name).clicked() {
                            let mut o = self.default_export();
                            o.format = k;
                            o.dpi = if k == 2 { 2 } else { 3 };
                            self.export = Some(o);
                        }
                    }
                });
                if mi(ui, true, "Format du document…", "Ctrl+K") {
                    self.doc_sheet = true;
                }
                if mi(ui, true, "Fermer", "") {
                    self.home = true;
                }
            });
        });
        if self.home {
            self.help.menu(ui);
            return;
        }
        let has = self.sel.is_some();
        let multi = self.selection().len() > 1;
        ui.menu_button("Édition", |ui| {
            if mi(ui, !self.undo.is_empty(), "Annuler", "Ctrl+Z") {
                self.undo_step();
            }
            if mi(ui, !self.redo.is_empty(), "Rétablir", "Ctrl+Y") {
                self.redo_step();
            }
            ui.separator();
            if mi(ui, has, "Couper", "Ctrl+X") {
                self.copy_sel();
                self.delete_selection();
            }
            if mi(ui, has, "Copier", "Ctrl+C") {
                self.copy_sel();
            }
            if mi(ui, !self.clip.is_empty(), "Coller", "Ctrl+V") {
                self.paste(false);
            }
            if mi(ui, !self.clip.is_empty(), "Coller sur place", "Ctrl+Maj+V") {
                self.paste(true);
            }
            if mi(ui, has, "Dupliquer", "Ctrl+D") {
                self.duplicate();
            }
            if mi(ui, has, "Dupliquer et décaler…", "Ctrl+Alt+U") {
                self.step = Some(panels::StepOpts { count: 3, dx: 20.0, dy: 20.0, rot: 0.0, grid: false, rows: 2, cols: 2 });
            }
            if mi(ui, has, "Supprimer", "Suppr") {
                self.delete_selection();
            }
            ui.separator();
            if mi(ui, true, "Tout sélectionner", "Ctrl+A") {
                self.select_all();
            }
            if mi(ui, has, "Tout désélectionner", "Ctrl+Maj+A") {
                self.sel = None;
                self.picks.clear();
            }
            ui.separator();
            if mi(ui, true, "Rechercher et remplacer…", "Ctrl+F") {
                self.find = Some(panels::FindState::new(String::new()));
            }
            ui.menu_button("Corrections typographiques", |ui| {
                for (k, t) in panels::TYPO.iter().enumerate() {
                    if ui.button(*t).clicked() {
                        let n = self.typo_all(&[k]);
                        self.notice = Some((format!("{t} : {n} bloc(s) corrigé(s)"), true));
                    }
                }
                ui.separator();
                if ui.button("Tout corriger").clicked() {
                    let n = self.typo_all(&[0, 1, 2, 3, 4]);
                    self.notice = Some((format!("Corrections typographiques : {n} bloc(s)"), true));
                }
            });
        });
        ui.menu_button("Objet", |ui| {
            ui.add_enabled_ui(has, |ui| {
                ui.menu_button("Disposition", |ui| {
                    for (label, key, to) in [("Premier plan", "Ctrl+Maj+]", i32::MAX), ("En avant", "Ctrl+]", 1), ("En arrière", "Ctrl+[", -1), ("Arrière-plan", "Ctrl+Maj+[", i32::MIN)] {
                        if mi(ui, true, label, key) {
                            self.order(to);
                        }
                    }
                });
                if mi(ui, multi, "Grouper", "Ctrl+G") {
                    self.group(true);
                }
                if mi(ui, true, "Dissocier", "Ctrl+Maj+G") {
                    self.group(false);
                }
                ui.menu_button("Transformation", |ui| {
                    for (label, a) in [("Rotation de 90° horaire", 90.0), ("Rotation de 90° antihoraire", -90.0), ("Rotation de 180°", 180.0)] {
                        if ui.button(label).clicked() {
                            self.rotate_by(a);
                        }
                    }
                    if ui.button("Symétrie horizontale").clicked() {
                        self.flip(true);
                    }
                    if ui.button("Symétrie verticale").clicked() {
                        self.flip(false);
                    }
                    if ui.button("Effacer les transformations").clicked() {
                        for i in self.selection() {
                            let f = &mut self.frames()[i];
                            (f.rotation, f.fx.skew, f.fx.flip_h, f.fx.flip_v) = (0.0, 0.0, false, false);
                        }
                    }
                });
                ui.menu_button("Aligner", |ui| {
                    for (label, axis, to) in [("Gauche", 0, 0.0), ("Centre horizontal", 0, 0.5), ("Droite", 0, 1.0), ("Haut", 1, 0.0), ("Centre vertical", 1, 0.5), ("Bas", 1, 1.0)] {
                        if ui.button(label).clicked() {
                            self.align_to(axis, to, if multi { 0 } else { 1 });
                        }
                    }
                    ui.separator();
                    if ui.add_enabled(multi, egui::Button::new("Répartir horizontalement")).clicked() {
                        self.distribute(0, None);
                    }
                    if ui.add_enabled(multi, egui::Button::new("Répartir verticalement")).clicked() {
                        self.distribute(1, None);
                    }
                });
                if ui.button("Centrer sur la page").clicked() {
                    self.align_to(0, 0.5, 2);
                    self.align_to(1, 0.5, 2);
                }
                if ui.button("Remplir la page (fond perdu compris)").clicked()
                    && let Some(i) = self.sel
                {
                    let (w, h) = self.cur_size();
                    let b = self.doc.bleed;
                    let f = &mut self.frames()[i];
                    (f.x, f.y, f.w, f.h) = (-b, -b, w + 2.0 * b, h + 2.0 * b);
                }
                ui.separator();
                ui.menu_button("Ajustement", |ui| {
                    for (k, (label, key)) in FITS.iter().enumerate() {
                        if mi(ui, true, label, key) {
                            self.fitting(k);
                        }
                    }
                });
                ui.menu_button("Options d'angle", |ui| {
                    for (k, t) in CORNERS.iter().enumerate() {
                        if ui.button(*t).clicked() {
                            for i in self.selection() {
                                self.frames()[i].fx.corner = k as u8;
                            }
                        }
                    }
                });
                ui.menu_button("Habillage", |ui| {
                    for (k, t) in WRAPS.iter().enumerate() {
                        if ui.button(*t).clicked() {
                            for i in self.selection() {
                                self.frames()[i].fx.wrap = k as u8;
                            }
                        }
                    }
                });
                ui.menu_button("Effets", |ui| {
                    for (k, t) in ["Ombre portée", "Lueur externe", "Contour progressif", "Contour progressif directionnel"].iter().enumerate() {
                        if ui.button(*t).clicked() {
                            for i in self.selection() {
                                let f = &mut self.frames()[i];
                                match k {
                                    0 => f.shadow = !f.shadow,
                                    1 => f.fx.glow = !f.fx.glow,
                                    2 => f.fx.feather = if f.fx.feather > 0.0 { 0.0 } else { 8.0 },
                                    _ => f.fx.gfeather = !f.fx.gfeather,
                                }
                            }
                        }
                    }
                    ui.menu_button("Mode de fusion", |ui| {
                        for (k, (t, _)) in BLENDS.iter().enumerate() {
                            if ui.button(*t).clicked() {
                                for i in self.selection() {
                                    self.frames()[i].fx.blend = k as u8;
                                }
                            }
                        }
                    });
                    if ui.button("Effacer les effets").clicked() {
                        for i in self.selection() {
                            let f = &mut self.frames()[i];
                            f.shadow = false;
                            (f.fx.glow, f.fx.feather, f.fx.gfeather, f.fx.blend) = (false, 0.0, false, 0);
                        }
                    }
                });
                ui.menu_button("Styles d'objet", |ui| {
                    let names: Vec<String> = self.doc.obj_styles.iter().map(|s| s.name.clone()).collect();
                    for (k, n) in names.iter().enumerate() {
                        if ui.button(n).clicked() {
                            self.apply_obj_style(k);
                        }
                    }
                    if !names.is_empty() {
                        ui.separator();
                    }
                    if ui.button("Nouveau à partir de la sélection…").clicked() {
                        self.obj_name = Some(format!("Objet {}", names.len() + 1));
                    }
                });
                if ui.button("Convertir en tracé").on_hover_text("Rectangle, ellipse, polygone ou trait deviennent des tracés modifiables à la Plume").clicked() {
                    for i in self.selection() {
                        self.as_path(i);
                    }
                }
                if mi(ui, true, "Note…", "") {
                    self.note_edit = self.sel;
                }
            });
            ui.separator();
            if mi(ui, has, "Verrouiller", "Ctrl+L") {
                for i in self.selection() {
                    self.frames()[i].locked = true;
                }
                self.sel = None;
            }
            if mi(ui, true, "Tout déverrouiller", "") {
                self.frames().iter_mut().for_each(|f| f.locked = false);
            }
            if mi(ui, has, "Masquer", "Ctrl+3") {
                for i in self.selection() {
                    self.frames()[i].hidden = true;
                }
                self.sel = None;
            }
            if mi(ui, true, "Tout afficher", "Ctrl+Alt+3") {
                self.frames().iter_mut().for_each(|f| f.hidden = false);
            }
            if mi(ui, has, "Non imprimable", "") {
                for i in self.selection() {
                    let f = &mut self.frames()[i];
                    f.fx.nonprint = !f.fx.nonprint;
                }
            }
        });
        ui.menu_button("Texte", |ui| {
            let is_text = self.sel.is_some_and(|i| matches!(self.frames_ref().get(i).map(|f| &f.kind), Some(Kind::Text { .. } | Kind::TextPath { .. })));
            ui.add_enabled_ui(is_text, |ui| {
                ui.menu_button("Police", |ui| {
                    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                        for name in FONTS {
                            if ui.button(RichText::new(name)).clicked() {
                                self.text_sel(|k| {
                                    if let Kind::Text { family, .. } | Kind::TextPath { family, .. } = k {
                                        *family = name.into();
                                    }
                                });
                            }
                        }
                    });
                });
                ui.menu_button("Corps", |ui| {
                    for s in [6.0, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 18.0, 24.0, 30.0, 36.0, 48.0, 60.0, 72.0] {
                        if ui.button(format!("{s:.0} pt")).clicked() {
                            self.text_sel(|k| {
                                if let Kind::Text { size, .. } | Kind::TextPath { size, .. } = k {
                                    *size = s;
                                }
                            });
                        }
                    }
                });
                ui.menu_button("Style de paragraphe", |ui| {
                    for (k, (name, ..)) in STYLES.iter().enumerate() {
                        if ui.button(*name).clicked() {
                            self.apply_style(k);
                        }
                    }
                    let user: Vec<String> = self.doc.styles.iter().map(|s| s.name.clone()).collect();
                    if !user.is_empty() {
                        ui.separator();
                    }
                    for (k, n) in user.iter().enumerate() {
                        if ui.button(n).clicked() {
                            self.apply_style(STYLES.len() + k);
                        }
                    }
                });
                ui.menu_button("Casse", |ui| {
                    for (k, t) in panels::CASES.iter().enumerate() {
                        if ui.button(*t).clicked() {
                            self.case_sel(k);
                        }
                    }
                });
                ui.menu_button("Listes", |ui| {
                    for (k, t) in LISTS.iter().enumerate() {
                        if ui.button(*t).clicked() {
                            self.text_sel(|kind| {
                                if let Kind::Text { list, .. } = kind {
                                    *list = k as u8;
                                }
                            });
                        }
                    }
                });
                ui.menu_button("Lettrine", |ui| {
                    for (n, t) in [(0u32, "Aucune"), (2, "Sur 2 lignes"), (3, "Sur 3 lignes"), (4, "Sur 4 lignes")] {
                        if ui.button(t).clicked() {
                            self.text_sel(|kind| {
                                if let Kind::Text { opts, .. } = kind {
                                    opts.drop_lines = n;
                                }
                            });
                        }
                    }
                });
                ui.menu_button("Insérer une variable", |ui| {
                    for (v, t) in [("{page}", "Numéro de page"), ("{pages}", "Nombre de pages"), ("{title}", "Titre du document"), ("{date}", "Date du jour"), ("{section}", "Nom de section")] {
                        if ui.button(t).clicked() {
                            self.text_sel(|k| {
                                if let Kind::Text { text, .. } | Kind::TextPath { text, .. } = k {
                                    text.push_str(v);
                                }
                            });
                        }
                    }
                });
                if mi(ui, true, "Modifier le texte", "Entrée") {
                    self.edit_text = self.sel;
                }
                if mi(ui, true, "Ajuster la hauteur au texte", "") {
                    self.fit_text();
                }
                if mi(ui, true, "Remplir avec du faux texte", "") {
                    self.lorem();
                }
            });
            ui.separator();
            if mi(ui, true, "Créer la table des matières…", "") {
                self.open_toc();
            }
            if mi(ui, true, "Nouveau style de caractère…", "") {
                self.char_sheet = Some(CharStyle { name: format!("Caractère {}", self.doc.char_styles.len() + 1), bold: true, italic: false, underline: false, strike: false, caps: false, color: None, scale: 1.0 });
            }
            ui.separator();
            ui.label(RichText::new("Balises : <b> <i> <u> <s> <sup> <sub> <sc> <c=#ff0000>").size(11.5).color(FAINT));
            ui.label(RichText::new("Variables : {page} {pages} {title} {date} {section}").size(11.5).color(FAINT));
        });
        ui.menu_button("Page", |ui| {
            let cur = if self.page == MASTER { 0 } else { self.page };
            if ui.button("Ajouter une page après").clicked() {
                self.add_page(cur + 1, false);
            }
            if ui.button("Ajouter une page avant").clicked() {
                self.add_page(cur, false);
            }
            if ui.button("Dupliquer la page").clicked() {
                self.add_page(cur + 1, true);
            }
            if ui.add_enabled(self.page != MASTER && self.page > 0, egui::Button::new("Déplacer vers le haut")).clicked() {
                self.move_page(-1);
            }
            if ui.add_enabled(self.page != MASTER && self.page + 1 < self.doc.pages.len(), egui::Button::new("Déplacer vers le bas")).clicked() {
                self.move_page(1);
            }
            if ui.add_enabled(self.page != MASTER && self.doc.pages.len() > 1, egui::Button::new("Supprimer la page")).clicked() {
                self.delete_page(self.page);
            }
            ui.separator();
            if mi(ui, true, "Première page", "Début") {
                self.page = 0;
            }
            if mi(ui, true, "Dernière page", "Fin") {
                self.page = self.doc.pages.len() - 1;
            }
            ui.separator();
            if mi(ui, self.page != MASTER, "Format de cette page…", "Maj+P") {
                self.tool = Tool::PageTool;
            }
            if mi(ui, true, "Numérotation et sections…", "") {
                self.sections_sheet = true;
            }
            ui.checkbox(&mut self.doc.facing, "Pages en vis-à-vis");
            ui.separator();
            if ui.button("Modifier le gabarit").clicked() {
                self.page = MASTER;
                self.sel = None;
            }
            if ui.button("Appliquer le gabarit à toutes les pages").clicked() {
                self.doc.pages.iter_mut().for_each(|p| p.no_master = false);
            }
            if ui.button("Retirer le gabarit de toutes les pages").clicked() {
                self.doc.pages.iter_mut().for_each(|p| p.no_master = true);
            }
        });
        ui.menu_button("Affichage", |ui| {
            if mi(ui, true, "Ajuster à la fenêtre", "Ctrl+0") {
                self.fit_pending = true;
            }
            if mi(ui, true, "Taille réelle", "Ctrl+1") {
                self.zoom = 1.0;
            }
            ui.menu_button("Zoom", |ui| {
                for z in [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0] {
                    if ui.button(format!("{:.0} %", z * 100.0)).clicked() {
                        self.zoom = z;
                    }
                }
            });
            ui.separator();
            ui.checkbox(&mut self.preview, "Aperçu (masquer les repères)   W");
            ui.checkbox(&mut self.guides, "Marges, colonnes et grille   Ctrl+;");
            ui.checkbox(&mut self.edges, "Bords des blocs   Ctrl+H");
            ui.checkbox(&mut self.rulers, "Règles   Ctrl+R");
            ui.checkbox(&mut self.snap, "Magnétisme");
            ui.checkbox(&mut self.show_notes, "Notes");
            ui.checkbox(&mut self.doc.facing, "Pages en vis-à-vis");
            ui.separator();
            if ui.add_enabled(!self.doc.guides.is_empty(), egui::Button::new("Effacer tous les repères")).clicked() {
                self.doc.guides.clear();
            }
        });
        ui.menu_button("Fenêtre", |ui| {
            for (k, (t, key)) in [("Propriétés", ""), ("Calques", "F7"), ("Liens", "Ctrl+Maj+D"), ("Contrôle en amont", "Ctrl+Alt+Maj+F")].iter().enumerate() {
                if mi(ui, true, &format!("{}{t}", if self.tab == k { "✓ " } else { "" }), key) {
                    self.tab = k;
                }
            }
        });
        self.help.menu(ui);
    }

    fn lorem(&mut self) {
        let Some(i) = self.sel else { return };
        if let Kind::Text { text, .. } = &mut self.frames()[i].kind {
            *text = format!("{LOREM}\n\n{LOREM}");
        }
    }

    fn home_screen(&mut self, root: &mut Ui) {
        egui::CentralPanel::default().frame(EFrame::new().fill(PASTEBOARD).inner_margin(Margin { left: 40, right: 40, top: 34, bottom: 20 })).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new((self.icon.id(), Vec2::splat(40.0))));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Bienvenue dans Page").font(semibold(28.0)).color(INK_STRONG));
                        ui.label(RichText::new("Mise en page de documents, magazines, affiches et PDF").color(MUTED));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::secondary(ui, "Ouvrir…").clicked() {
                            self.open();
                        }
                    });
                });
                ui.add_space(26.0);
                ui.label(RichText::new("Nouveau document").font(semibold(16.0)).color(INK));
                ui.add_space(12.0);
                let mut chosen = None;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                    for (name, kind, w, h) in PRESETS {
                        if preset_card(ui, name, &format!("{kind} · {w:.0} × {h:.0} pt"), w, h, false).clicked() {
                            chosen = Some(starter(w, h));
                        }
                    }
                });
                ui.add_space(24.0);
                ui.label(RichText::new("Modèles").font(semibold(16.0)).color(INK));
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                    for (k, (name, hint)) in TEMPLATES.iter().enumerate() {
                        let d = template(k);
                        if preset_card(ui, name, hint, d.width, d.height, true).clicked() {
                            chosen = Some(d);
                        }
                    }
                });
                if let Some(d) = chosen {
                    self.set_doc(d, None);
                }
                ui.add_space(26.0);
                ui.label(RichText::new("Documents récents").font(semibold(16.0)).color(INK));
                ui.add_space(10.0);
                if self.recents.is_empty() {
                    ui.label(RichText::new("Tes documents .epage apparaîtront ici").color(FAINT));
                }
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
            });
        });
    }

    fn options_bar(&mut self, root: &mut Ui) {
        egui::Panel::top("options").exact_size(42.0).frame(EFrame::new().fill(Color32::from_rgb(0x20, 0x20, 0x24)).inner_margin(Margin::symmetric(12, 0)).stroke(Stroke::new(1.0, LINE))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(7), FIELD);
                tool_icon(ui.painter(), r.shrink(7.0), self.tool, INK);
                ui.label(RichText::new(tool_name(self.tool)).font(semibold(13.0)).color(INK));
                pro::toolbar_sep(ui);
                let swatches = self.doc.swatches.clone();
                let sel = self.sel.filter(|i| *i < self.frames_ref().len());
                let n_sel = self.selection().len();
                let select_like = matches!(self.tool, Tool::Select | Tool::Direct | Tool::FreeTransform);
                match (select_like, sel) {
                    (true, Some(_)) if n_sel > 1 => {
                        ui.label(RichText::new(format!("{n_sel} blocs")).size(12.5).color(MUTED));
                        for (t, axis, to) in [("⇤", 0, 0.0), ("↔", 0, 0.5), ("⇥", 0, 1.0), ("⤒", 1, 0.0), ("↕", 1, 0.5), ("⤓", 1, 1.0)] {
                            if ui.button(t).on_hover_text("Aligner sur la sélection").clicked() {
                                self.align_to(axis, to, 0);
                            }
                        }
                        pro::toolbar_sep(ui);
                        if ui.button("Répartir ↔").clicked() {
                            self.distribute(0, None);
                        }
                        if ui.button("Répartir ↕").clicked() {
                            self.distribute(1, None);
                        }
                        if ui.button("Grouper").on_hover_text("Ctrl+G").clicked() {
                            self.group(true);
                        }
                    }
                    (true, Some(i)) => {
                        let mut style = None;
                        let f = &mut self.frames()[i];
                        match &mut f.kind {
                            Kind::Text { family, size, bold, italic, underline, upper, align, color, leading, tracking, .. } => {
                                egui::ComboBox::from_id_salt("ofont").width(140.0).selected_text(family.as_str()).show_ui(ui, |ui| {
                                    for name in FONTS {
                                        ui.selectable_value(family, name.to_string(), name);
                                    }
                                });
                                ui.add(egui::DragValue::new(size).range(4.0..=400.0).speed(0.2).suffix(" pt").max_decimals(1)).on_hover_text("Corps · Ctrl+Maj+< / >");
                                ui.add(egui::DragValue::new(leading).range(0.6..=4.0).speed(0.01).prefix("↕ ").max_decimals(2)).on_hover_text("Interligne · Alt+↑ / ↓");
                                ui.add(egui::DragValue::new(tracking).range(-5.0..=50.0).speed(0.05).prefix("↔ ").max_decimals(1)).on_hover_text("Approche · Alt+← / →");
                                ui.toggle_value(bold, RichText::new("G").strong()).on_hover_text("Gras · Ctrl+Maj+B");
                                ui.toggle_value(italic, RichText::new("I").italics()).on_hover_text("Italique");
                                ui.toggle_value(underline, RichText::new("S").underline()).on_hover_text("Souligné · Ctrl+Maj+U");
                                ui.toggle_value(upper, RichText::new("AA")).on_hover_text("Majuscules · Ctrl+Maj+K");
                                pro::toolbar_sep(ui);
                                let mut a = (*align as usize).min(3);
                                if pro::segmented(ui, &mut a, &["≡ G", "≡ C", "≡ D", "≡ J"]) {
                                    *align = a as u8;
                                }
                                pro::toolbar_sep(ui);
                                swatch_button(ui, color, &swatches);
                                ui.menu_button("Styles", |ui| {
                                    for (k, (name, ..)) in STYLES.iter().enumerate() {
                                        if ui.button(*name).clicked() {
                                            style = Some(k);
                                        }
                                    }
                                });
                            }
                            Kind::Rect { fill, .. } | Kind::Ellipse { fill } | Kind::Polygon { fill, .. } => {
                                ui.label(RichText::new("Fond").size(12.5).color(MUTED));
                                swatch_button(ui, fill, &swatches);
                                ui.label(RichText::new("Contour").size(12.5).color(MUTED));
                                let mut on = f.stroke.is_some();
                                if ui.checkbox(&mut on, "").changed() {
                                    f.stroke = on.then_some([20, 20, 24, 255]);
                                }
                                if let Some(c) = &mut f.stroke {
                                    swatch_button(ui, c, &swatches);
                                    ui.add(egui::DragValue::new(&mut f.stroke_w).range(0.25..=40.0).speed(0.1).suffix(" pt"));
                                }
                                ui.add(egui::DragValue::new(&mut f.rotation).range(-180.0..=180.0).suffix("°").prefix("⟳ "));
                            }
                            Kind::Path { fill, color, width, .. } => {
                                let mut on = fill.is_some();
                                if ui.checkbox(&mut on, "Fond").changed() {
                                    *fill = on.then_some([255, 196, 0, 255]);
                                }
                                if let Some(c) = fill {
                                    swatch_button(ui, c, &swatches);
                                }
                                ui.label(RichText::new("Trait").size(12.5).color(MUTED));
                                swatch_button(ui, color, &swatches);
                                ui.add(egui::DragValue::new(width).range(0.0..=40.0).speed(0.1).suffix(" pt"));
                            }
                            Kind::Line { color, width, arrow, start_arrow, .. } => {
                                swatch_button(ui, color, &swatches);
                                ui.add(egui::DragValue::new(width).range(0.25..=40.0).speed(0.1).suffix(" pt"));
                                ui.checkbox(start_arrow, "Début");
                                ui.checkbox(arrow, "Flèche");
                            }
                            Kind::Image { cover, filter, zoom, stretch, .. } => {
                                let mut c = if *stretch { 2 } else if *cover { 0 } else { 1 };
                                if pro::segmented(ui, &mut c, &["Remplir", "Ajuster", "Déformer"]) {
                                    (*cover, *stretch) = (c == 0, c == 2);
                                }
                                ui.add(egui::DragValue::new(zoom).range(0.05..=20.0).speed(0.01).prefix("Échelle ").max_decimals(2));
                                egui::ComboBox::from_id_salt("ofilter").selected_text(FILTERS[(*filter as usize).min(FILTERS.len() - 1)]).show_ui(ui, |ui| {
                                    for (k, n) in FILTERS.iter().enumerate() {
                                        ui.selectable_value(filter, k as u8, *n);
                                    }
                                });
                            }
                            Kind::Table { size, .. } | Kind::TextPath { size, .. } => {
                                ui.label(RichText::new("Taille du texte").size(12.5).color(MUTED));
                                ui.add(egui::DragValue::new(size).range(4.0..=200.0).speed(0.2).suffix(" pt"));
                            }
                        }
                        if let Some(k) = style {
                            self.apply_style(k);
                        }
                    }
                    _ => self.tool_options(ui),
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::ghost(ui, "?  Tutoriel").on_hover_text("F1").clicked() {
                        self.help.learn(tool_name(self.tool));
                    }
                });
            });
        });
    }

    fn status(&mut self, root: &mut Ui) {
        let busy = root.ctx().input(|i| i.pointer.any_down()) || self.edit_text.is_some();
        let errors = if busy { self.preflight.as_ref().map_or(0, |p| p.1.iter().filter(|i| i.error).count()) } else { self.issues().iter().filter(|i| i.error).count() };
        egui::Panel::bottom("status").exact_size(28.0).frame(EFrame::new().fill(CHROME).inner_margin(Margin::symmetric(12, 0))).show(root, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: String| RichText::new(t).size(12.0).color(MUTED);
                let (dot, resp) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::click());
                ui.painter().circle_filled(dot.center(), 5.0, if errors > 0 { RED } else { GREEN });
                if resp.on_hover_text(if errors > 0 { format!("{errors} erreur(s) de contrôle en amont") } else { "Aucune erreur de contrôle en amont".into() }).clicked() {
                    self.tab = 3;
                }
                let (w, h) = self.cur_size();
                let where_ = if self.page == MASTER { "Gabarit".to_string() } else { format!("Page {} sur {}", self.doc.page_label(self.page), self.doc.pages.len()) };
                ui.label(small(format!("{where_} · {:.0} × {:.0} mm · {} mots", w / MM, h / MM, self.word_count())));
                let n = self.selection().len();
                if n > 1 {
                    ui.label(small(format!("· {n} blocs sélectionnés")));
                }
                if self.overflow.iter().any(|o| *o) {
                    ui.label(RichText::new("· texte en excès sur cette page").size(12.0).color(RED));
                }
                if let Some((n, ok)) = self.notice.clone() {
                    ui.label(RichText::new(n).size(12.0).color(if ok { GREEN } else { RED }));
                    if ui.small_button("×").clicked() {
                        self.notice = None;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Ajuster").on_hover_text("Ctrl+0").clicked() {
                        self.fit_pending = true;
                    }
                    if ui.small_button("+").clicked() {
                        self.zoom = (self.zoom * 1.25).min(8.0);
                    }
                    ui.label(RichText::new(format!("{:.0} %", self.zoom * 100.0)).size(12.0).color(INK).monospace());
                    if ui.small_button("−").clicked() {
                        self.zoom = (self.zoom / 1.25).max(0.05);
                    }
                    ui.add_space(8.0);
                    ui.toggle_value(&mut self.preview, RichText::new("Aperçu").size(12.0)).on_hover_text("Masque cadres et repères (W)");
                });
            });
        });
    }

    fn tool_rail(&mut self, root: &mut Ui) {
        egui::Panel::left("tools").resizable(false).exact_size(52.0).frame(EFrame::new().fill(PANEL).inner_margin(Margin::symmetric(7, 10))).show(root, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                for (k, g) in tools::GROUPS.iter().enumerate() {
                    if [1, 4, 8, 11, 13].contains(&k) {
                        let (sep, _) = ui.allocate_exact_size(vec2(38.0, 7.0), Sense::hover());
                        ui.painter().line_segment([sep.left_center() + vec2(6.0, 0.0), sep.right_center() - vec2(6.0, 0.0)], Stroke::new(1.0, LINE));
                    }
                    let before = self.tool;
                    pro::tool_group(ui, 38.0, &mut self.tool, g, |p, r, c, t| tool_icon(p, r, t, c));
                    if before == Tool::Pen && self.tool != Tool::Pen {
                        self.commit_pen(false);
                    }
                }
            });
        });
    }

    fn thumb(&self, ui: &mut Ui, page: Option<&Page>, frames: &[Frame], selected: bool, label: &str) -> egui::Response {
        let ratio = self.doc.height / self.doc.width;
        let w = 96.0f32.min(130.0 / ratio);
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), w * ratio + 24.0), Sense::click());
        let card = Rect::from_center_size(pos2(r.center().x, r.min.y + w * ratio / 2.0 + 4.0), vec2(w, w * ratio));
        let p = ui.painter();
        p.add(egui::epaint::Shadow { offset: [0, 2], blur: 6, spread: 0, color: Color32::from_black_alpha(90) }.as_shape(card, CornerRadius::same(2)));
        p.rect_filled(card, CornerRadius::same(2), Color32::from_rgb(self.doc.paper[0], self.doc.paper[1], self.doc.paper[2]));
        let s = w / self.doc.width;
        let master: &[Frame] = if page.is_some_and(|p| !p.no_master) { &self.doc.master.frames } else { &[] };
        let pc = p.with_clip_rect(card);
        for f in master.iter().chain(frames).filter(|f| !f.hidden) {
            let fr = Rect::from_min_size(card.min + vec2(f.x, f.y) * s, vec2(f.w, f.h) * s);
            match &f.kind {
                Kind::Text { size, color, background, .. } => {
                    if let Some(bg) = background {
                        pc.rect_filled(fr, CornerRadius::ZERO, Color32::from_rgb(bg[0], bg[1], bg[2]));
                    }
                    let lh = (size * s * 1.3).max(1.5);
                    let mut y = fr.top() + lh * 0.5;
                    let c = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 150);
                    while y < fr.bottom() {
                        pc.line_segment([pos2(fr.left(), y), pos2(fr.right() - (y * 7.0) % (fr.width() * 0.3).max(1.0), y)], Stroke::new((size * s * 0.55).max(0.6), c));
                        y += lh;
                    }
                }
                Kind::Image { .. } => {
                    pc.rect_filled(fr, CornerRadius::ZERO, Color32::from_rgb(180, 196, 214));
                }
                Kind::Rect { fill, .. } | Kind::Ellipse { fill } | Kind::Polygon { fill, .. } => {
                    pc.rect_filled(fr, CornerRadius::ZERO, Color32::from_rgb(fill[0], fill[1], fill[2]));
                }
                Kind::Line { color, .. } => {
                    pc.line_segment([fr.left_center(), fr.right_center()], Stroke::new(1.0, Color32::from_rgb(color[0], color[1], color[2])));
                }
                Kind::Table { .. } | Kind::Path { .. } | Kind::TextPath { .. } => {
                    pc.rect_stroke(fr, CornerRadius::ZERO, Stroke::new(0.8, Color32::from_gray(150)), StrokeKind::Inside);
                }
            }
        }
        if selected {
            p.rect_stroke(card.expand(3.0), CornerRadius::same(4), Stroke::new(2.0, BLUE), StrokeKind::Outside);
        } else if resp.hovered() {
            p.rect_stroke(card.expand(3.0), CornerRadius::same(4), Stroke::new(1.0, MUTED), StrokeKind::Outside);
        }
        p.text(pos2(r.center().x, r.max.y - 9.0), Align2::CENTER_CENTER, label, FontId::proportional(11.5), if selected { INK } else { MUTED });
        resp
    }

    fn pages_panel(&mut self, root: &mut Ui) {
        egui::Panel::left("pages").resizable(false).exact_size(170.0).frame(EFrame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 12))).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("PAGES").font(semibold(11.0)).color(FAINT));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("+").on_hover_text("Ajouter une page à la fin").clicked() {
                        let at = self.doc.pages.len();
                        self.add_page(at, false);
                    }
                });
            });
            ui.add_space(6.0);
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let master = self.doc.master.frames.clone();
                if self.thumb(ui, None, &master, self.page == MASTER, "Gabarit").on_hover_text("Ses blocs apparaissent sur toutes les pages").clicked() {
                    self.page = MASTER;
                    self.sel = None;
                }
                ui.add_space(6.0);
                let mut act: Option<(usize, u8)> = None;
                for i in 0..self.doc.pages.len() {
                    let pg = self.doc.pages[i].clone();
                    let label = format!("{}{}", self.doc.page_label(i), if pg.no_master { " · sans gabarit" } else { "" });
                    let r = self.thumb(ui, Some(&pg), &pg.frames, self.page == i, &label);
                    if r.clicked() {
                        self.page = i;
                        self.sel = None;
                    }
                    r.context_menu(|ui| {
                        for (k, l) in ["Insérer une page avant", "Insérer une page après", "Dupliquer", "Monter", "Descendre", "Utiliser / retirer le gabarit", "Supprimer"].iter().enumerate() {
                            if ui.button(*l).clicked() {
                                act = Some((i, k as u8));
                                ui.close();
                            }
                        }
                    });
                }
                if let Some((i, k)) = act {
                    self.page = i;
                    match k {
                        0 => self.add_page(i, false),
                        1 => self.add_page(i + 1, false),
                        2 => self.add_page(i + 1, true),
                        3 => self.move_page(-1),
                        4 => self.move_page(1),
                        5 => self.doc.pages[i].no_master = !self.doc.pages[i].no_master,
                        _ => self.delete_page(i),
                    }
                }
                ui.add_space(4.0);
                if pro::secondary(ui, "+ Page").clicked() {
                    let at = self.doc.pages.len();
                    self.add_page(at, false);
                }
            });
        });
    }

    fn props(&mut self, root: &mut Ui) {
        egui::Panel::right("props").resizable(false).exact_size(304.0).frame(EFrame::new().fill(PANEL)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    pro::segmented(ui, &mut self.tab, &["Propriétés", "Calques", "Liens", "Contrôle"]);
                });
                ui.add_space(8.0);
                match self.tab {
                    1 => {
                        self.layers_panel(ui);
                        self.objects_panel(ui);
                    }
                    2 => self.links_panel(ui),
                    3 => self.preflight_panel(ui),
                    _ => {
                        match self.sel.filter(|i| *i < self.frames_ref().len()) {
                            Some(i) => self.frame_panels(ui, i),
                            None => self.doc_panel(ui),
                        }
                        self.styles_panels(ui);
                    }
                }
            });
        });
    }

    fn frame_panels(&mut self, ui: &mut Ui, i: usize) {
        let over = self.overflow.get(i).copied().unwrap_or(false);
        let swatches = self.doc.swatches.clone();
        let user_styles: Vec<String> = self.doc.styles.iter().map(|s| s.name.clone()).collect();
        let texts: Vec<(usize, String)> = self.frames_ref().iter().enumerate().filter(|(k, f)| *k != i && matches!(f.kind, Kind::Text { .. })).map(|(k, f)| (k, f.title())).collect();
        let (mut fit, mut order, mut align, mut style, mut save_style, mut fit_img, mut edit, mut add_swatch) = (false, None, None, None, false, false, false, None);
        let f = &mut self.frames()[i];
        pro::dock_panel(ui, "pos", "Bloc", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.add(egui::TextEdit::singleline(&mut f.name).hint_text(f.kind.label()).desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                ui.label(RichText::new("X").color(MUTED));
                ui.add(egui::DragValue::new(&mut f.x).max_decimals(1).suffix(" pt"));
                ui.label(RichText::new("Y").color(MUTED));
                ui.add(egui::DragValue::new(&mut f.y).max_decimals(1).suffix(" pt"));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("L").color(MUTED));
                ui.add(egui::DragValue::new(&mut f.w).range(1.0..=5000.0).max_decimals(1).suffix(" pt"));
                ui.label(RichText::new("H").color(MUTED));
                ui.add(egui::DragValue::new(&mut f.h).range(1.0..=5000.0).max_decimals(1).suffix(" pt"));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Rotation").color(MUTED));
                ui.add(egui::DragValue::new(&mut f.rotation).range(-180.0..=180.0).speed(0.5).suffix("°"));
                for a in [-90.0, 0.0, 90.0] {
                    if ui.small_button(format!("{a:.0}°")).clicked() {
                        f.rotation = a;
                    }
                }
            });
        });
        if matches!(f.kind, Kind::Image { .. }) {
            pro::dock_panel(ui, "img-fx", "Effets de l'image", |ui| {
                studio_ui::fxpanel::effects_panel(ui, "page", &mut f.effects);
            });
        }
        match &mut f.kind {
            Kind::Text { text, size, family, bold, color, align: al, leading, italic, tracking, columns, gutter, background, padding, underline, upper, list, para_space, indent, valign, link, opts } => {
                pro::dock_panel(ui, "text", "Texte", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    egui::ScrollArea::vertical().id_salt("textbox").max_height(170.0).show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(text).desired_rows(6).desired_width(f32::INFINITY));
                    });
                    if over {
                        ui.label(RichText::new("Le texte dépasse du bloc.").size(12.0).color(RED));
                        ui.horizontal(|ui| {
                            if pro::secondary(ui, "Ajuster la hauteur").clicked() {
                                fit = true;
                            }
                        });
                    }
                    if pro::ghost(ui, "Écrire sur la page (Entrée)").clicked() {
                        edit = true;
                    }
                });
                pro::dock_panel(ui, "char", "Caractère", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    egui::ComboBox::from_id_salt("font").selected_text(family.as_str()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        for name in FONTS {
                            ui.selectable_value(family, name.to_string(), name);
                        }
                    });
                    ui.label(RichText::new("Taille").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(size, 4.0..=200.0).logarithmic(true).suffix(" pt"));
                    ui.label(RichText::new("Approche").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(tracking, -2.0..=20.0).suffix(" pt"));
                    ui.horizontal(|ui| {
                        ui.toggle_value(bold, RichText::new(" G ").strong());
                        ui.toggle_value(italic, RichText::new(" I ").italics());
                        ui.toggle_value(underline, RichText::new(" S ").underline());
                        ui.toggle_value(upper, RichText::new(" AA "));
                    });
                    color_row(ui, "Couleur", color, &swatches);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut opts.small_caps, "Petites capitales");
                        ui.checkbox(&mut opts.strike, "Barré");
                    });
                    ui.label(RichText::new("Décalage de la ligne de base").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(&mut opts.baseline_shift, -30.0..=30.0).suffix(" pt"));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Échelle H").size(12.5).color(MUTED));
                        let mut h = opts.hscale * 100.0;
                        if ui.add(egui::DragValue::new(&mut h).range(40.0..=250.0).suffix(" %").max_decimals(0)).changed() {
                            opts.hscale = h / 100.0;
                        }
                        ui.label(RichText::new("V").size(12.5).color(MUTED));
                        let mut v = opts.vscale * 100.0;
                        if ui.add(egui::DragValue::new(&mut v).range(40.0..=250.0).suffix(" %").max_decimals(0)).changed() {
                            opts.vscale = v / 100.0;
                        }
                    });
                    let mut on = opts.outline.is_some();
                    if ui.checkbox(&mut on, "Contour des lettres").changed() {
                        opts.outline = on.then_some([255, 255, 255, 255]);
                    }
                    if let Some(c) = &mut opts.outline {
                        color_row(ui, "Couleur du contour", c, &swatches);
                        ui.add(egui::Slider::new(&mut opts.outline_w, 0.1..=6.0).suffix(" pt"));
                    }
                });
                pro::dock_panel(ui, "para", "Paragraphe", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    let mut a = *al as usize;
                    if pro::segmented(ui, &mut a, &["Gauche", "Centre", "Droite", "Justifié"]) {
                        *al = a as u8;
                    }
                    ui.label(RichText::new("Interligne").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(leading, 0.8..=2.5));
                    ui.label(RichText::new("Espace après paragraphe").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(para_space, 0.0..=40.0).suffix(" pt"));
                    ui.label(RichText::new("Retrait de première ligne").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(indent, 0.0..=60.0).suffix(" pt"));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Liste").size(12.5).color(MUTED));
                        egui::ComboBox::from_id_salt("list").selected_text(LISTS[(*list as usize).min(LISTS.len() - 1)]).width(90.0).show_ui(ui, |ui| {
                            for (k, n) in LISTS.iter().enumerate() {
                                ui.selectable_value(list, k as u8, *n);
                            }
                        });
                        if *list == 1 {
                            egui::ComboBox::from_id_salt("bullet").selected_text(opts.bullet.as_str()).width(50.0).show_ui(ui, |ui| {
                                for b in BULLETS {
                                    ui.selectable_value(&mut opts.bullet, b.to_string(), b);
                                }
                            });
                        } else if *list > 1 {
                            ui.add(egui::DragValue::new(&mut opts.list_start).range(1..=999).prefix("début "));
                        }
                    });
                    if *list > 0 {
                        ui.checkbox(&mut opts.hang, "Retrait négatif (texte aligné après la puce)");
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Style").size(12.5).color(MUTED));
                        egui::ComboBox::from_id_salt("pstyle").selected_text("Appliquer…").width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                            for (k, (n, ..)) in STYLES.iter().enumerate() {
                                if ui.button(*n).clicked() {
                                    style = Some(k);
                                }
                            }
                            for (k, n) in user_styles.iter().enumerate() {
                                if ui.button(n).clicked() {
                                    style = Some(STYLES.len() + k);
                                }
                            }
                        });
                    });
                    if pro::ghost(ui, "+ Créer un style à partir de ce texte").clicked() {
                        save_style = true;
                    }
                });
                pro::dock_panel(ui, "para2", "Paragraphe avancé", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 100.0).max(80.0);
                    ui.label(RichText::new("Espace avant paragraphe").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(&mut opts.space_before, 0.0..=40.0).suffix(" pt"));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Retrait gauche").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(&mut opts.indent_left).range(0.0..=200.0).suffix(" pt"));
                        ui.label(RichText::new("droit").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(&mut opts.indent_right).range(0.0..=200.0).suffix(" pt"));
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Lettrine sur").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(&mut opts.drop_lines).range(0..=6).suffix(" lignes"));
                    });
                    ui.checkbox(&mut opts.hyphen, "Césure des mots longs");
                    ui.checkbox(&mut opts.orphans, "Éviter les lignes veuves et orphelines");
                    ui.checkbox(&mut opts.grid_align, "Aligner sur la grille de base");
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Tabulation tous les").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(&mut opts.tab).range(4.0..=400.0).suffix(" pt"));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut opts.leader, "Points de conduite");
                        ui.checkbox(&mut opts.tab_right, "Caler à droite");
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut opts.rule_above, "Filet au-dessus");
                        ui.checkbox(&mut opts.rule_below, "en dessous");
                    });
                    if opts.rule_above || opts.rule_below {
                        color_row(ui, "Couleur du filet", &mut opts.rule_color, &swatches);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Épaisseur").size(12.5).color(MUTED));
                            ui.add(egui::DragValue::new(&mut opts.rule_w).range(0.1..=20.0).speed(0.1).suffix(" pt"));
                            ui.label(RichText::new("Écart").size(12.5).color(MUTED));
                            ui.add(egui::DragValue::new(&mut opts.rule_off).range(0.0..=40.0).suffix(" pt"));
                        });
                    }
                });
                pro::dock_panel(ui, "tframe", "Bloc de texte", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Colonnes").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(columns).range(1..=6));
                        ui.label(RichText::new("Gouttière").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(gutter).range(0.0..=80.0).suffix(" pt"));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut opts.balance, "Équilibrer");
                        ui.checkbox(&mut opts.col_rule, "Filet entre colonnes");
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut opts.auto_h, "Hauteur auto");
                        ui.checkbox(&mut opts.no_wrap, "Ignorer l'habillage");
                    });
                    ui.label(RichText::new("Alignement vertical").size(12.5).color(MUTED));
                    let mut v = *valign as usize;
                    if pro::segmented(ui, &mut v, &["Haut", "Milieu", "Bas"]) {
                        *valign = v as u8;
                    }
                    ui.label(RichText::new("Marge intérieure").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(padding, 0.0..=60.0).suffix(" pt"));
                    let mut on = background.is_some();
                    if ui.checkbox(&mut on, "Fond coloré").changed() {
                        *background = on.then_some([245, 245, 247, 255]);
                    }
                    if let Some(bg) = background {
                        color_row(ui, "Fond", bg, &swatches);
                    }
                    ui.label(RichText::new("Suite du texte dans").size(12.5).color(MUTED));
                    let cur = link.and_then(|l| texts.iter().find(|t| t.0 == l)).map_or("Aucun bloc".to_string(), |t| t.1.clone());
                    egui::ComboBox::from_id_salt("link").selected_text(cur).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        ui.selectable_value(link, None, "Aucun bloc");
                        for (k, n) in &texts {
                            ui.selectable_value(link, Some(*k), n);
                        }
                    });
                    ui.label(RichText::new("Le texte qui ne tient pas continue dans ce bloc (texte chaîné).").size(11.5).color(FAINT));
                });
            }
            Kind::Image { path, cover, radius, filter, flip, ox, oy, zoom, stretch, shape, sides } => {
                pro::dock_panel(ui, "img", "Image", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    ui.label(RichText::new(if path.is_empty() { "Aucune image" } else { path.as_str() }).size(11.5).color(FAINT));
                    ui.horizontal(|ui| {
                        if pro::secondary(ui, "Choisir…").clicked()
                            && let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif", "svg"]).pick_file()
                        {
                            *path = p.to_string_lossy().into_owned();
                        }
                        if pro::ghost(ui, "Cadre aux proportions").clicked() {
                            fit_img = true;
                        }
                    });
                    let mut c = if *cover { 0 } else { 1 };
                    if pro::segmented(ui, &mut c, &["Remplir le cadre", "Ajuster dans le cadre"]) {
                        *cover = c == 0;
                    }
                    ui.checkbox(stretch, "Étirer sans garder les proportions");
                    ui.label(RichText::new("Zoom de l'image dans le cadre").size(12.5).color(MUTED));
                    let mut z = *zoom * 100.0;
                    if ui.add(egui::Slider::new(&mut z, 10.0..=600.0).logarithmic(true).suffix(" %")).changed() {
                        *zoom = z / 100.0;
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Décalage X").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(ox).speed(0.5).suffix(" pt"));
                        ui.label(RichText::new("Y").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(oy).speed(0.5).suffix(" pt"));
                        if ui.small_button("Centrer").clicked() {
                            (*ox, *oy, *zoom) = (0.0, 0.0, 1.0);
                        }
                    });
                    ui.label(RichText::new("Forme du cadre").size(12.5).color(MUTED));
                    let mut sh = *shape as usize;
                    if pro::segmented(ui, &mut sh, &["Rectangle", "Ellipse", "Polygone"]) {
                        *shape = sh as u8;
                    }
                    if *shape == 2 {
                        ui.add(egui::DragValue::new(sides).range(3..=24).prefix("Côtés "));
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Filtre").size(12.5).color(MUTED));
                        egui::ComboBox::from_id_salt("ifilter").selected_text(FILTERS[(*filter as usize).min(FILTERS.len() - 1)]).width(150.0).show_ui(ui, |ui| {
                            for (k, n) in FILTERS.iter().enumerate() {
                                ui.selectable_value(filter, k as u8, *n);
                            }
                        });
                    });
                    ui.label(RichText::new("Arrondi des coins").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(radius, 0.0..=300.0).suffix(" pt"));
                    ui.checkbox(flip, "Miroir horizontal");
                });
            }
            Kind::Rect { fill, radius } => {
                pro::dock_panel(ui, "shape", "Forme", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    color_row(ui, "Fond", fill, &swatches);
                    ui.label(RichText::new("Arrondi").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(radius, 0.0..=300.0).suffix(" pt"));
                    gradient_row(ui, &mut f.fill2, &mut f.grad_angle, &swatches);
                });
            }
            Kind::Ellipse { fill } => {
                pro::dock_panel(ui, "shape", "Forme", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    color_row(ui, "Fond", fill, &swatches);
                    gradient_row(ui, &mut f.fill2, &mut f.grad_angle, &swatches);
                });
            }
            Kind::Polygon { sides, inner, fill } => {
                pro::dock_panel(ui, "shape", "Forme", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    color_row(ui, "Fond", fill, &swatches);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Côtés / branches").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(sides).range(3..=48));
                    });
                    ui.label(RichText::new("Creux (1 = polygone)").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(inner, 0.1..=1.0));
                    gradient_row(ui, &mut f.fill2, &mut f.grad_angle, &swatches);
                });
            }
            Kind::Line { color, width, arrow, start_arrow, cap, dir } => {
                pro::dock_panel(ui, "shape", "Ligne", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    color_row(ui, "Couleur", color, &swatches);
                    ui.label(RichText::new("Épaisseur").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(width, 0.25..=30.0).suffix(" pt"));
                    ui.label(RichText::new("Pointillés (0 = plein)").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(&mut f.dash, 0.0..=40.0).suffix(" pt"));
                    ui.horizontal(|ui| {
                        ui.checkbox(arrow, "Flèche à la fin");
                        ui.checkbox(start_arrow, "au début");
                    });
                    ui.label(RichText::new("Extrémités").size(12.5).color(MUTED));
                    let mut c = *cap as usize;
                    if pro::segmented(ui, &mut c, &["Rondes", "Carrées", "Droites"]) {
                        *cap = c as u8;
                    }
                    ui.label(RichText::new("Direction").size(12.5).color(MUTED));
                    let mut d = *dir as usize;
                    if pro::segmented(ui, &mut d, &["—", "|", "╲", "╱"]) {
                        *dir = d as u8;
                    }
                });
            }
            Kind::Path { closed, fill, color, width, .. } => {
                pro::dock_panel(ui, "shape", "Tracé", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    ui.checkbox(closed, "Tracé fermé");
                    let mut on = fill.is_some();
                    if ui.checkbox(&mut on, "Fond").changed() {
                        *fill = on.then_some([120, 200, 255, 255]);
                    }
                    if let Some(c) = fill {
                        color_row(ui, "Couleur du fond", c, &swatches);
                    }
                    color_row(ui, "Couleur du trait", color, &swatches);
                    ui.label(RichText::new("Épaisseur").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(width, 0.0..=30.0).suffix(" pt"));
                    ui.label(RichText::new("Sélection directe (A) pour déplacer les points et leurs poignées.").size(11.5).color(FAINT));
                });
            }
            Kind::TextPath { text, size, family, bold, italic, color, tracking, closed, offset, show_path, .. } => {
                pro::dock_panel(ui, "text", "Texte sur tracé", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
                    ui.add(egui::TextEdit::multiline(text).desired_rows(2).desired_width(f32::INFINITY));
                    egui::ComboBox::from_id_salt("tpfont").selected_text(family.as_str()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        for name in FONTS {
                            ui.selectable_value(family, name.to_string(), name);
                        }
                    });
                    ui.label(RichText::new("Taille").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(size, 4.0..=200.0).logarithmic(true).suffix(" pt"));
                    ui.label(RichText::new("Approche").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(tracking, -2.0..=30.0).suffix(" pt"));
                    ui.label(RichText::new("Départ le long du tracé").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(offset, 0.0..=1.0));
                    ui.horizontal(|ui| {
                        ui.toggle_value(bold, RichText::new(" G ").strong());
                        ui.toggle_value(italic, RichText::new(" I ").italics());
                        ui.checkbox(show_path, "Voir le tracé");
                        ui.checkbox(closed, "Fermé");
                    });
                    color_row(ui, "Couleur", color, &swatches);
                });
            }
            Kind::Table { cells, size, family, color, line, header, stripe, align: talign, first_col } => {
                pro::dock_panel(ui, "table", "Tableau", |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let cols = cells.iter().map(|r| r.len()).max().unwrap_or(1);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{} lignes", cells.len())).size(12.5).color(MUTED));
                        if ui.small_button("−").clicked() && cells.len() > 1 {
                            cells.pop();
                        }
                        if ui.small_button("+").clicked() {
                            cells.push(vec![String::new(); cols]);
                        }
                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("{cols} colonnes")).size(12.5).color(MUTED));
                        if ui.small_button("−").clicked() && cols > 1 {
                            cells.iter_mut().for_each(|r| {
                                r.truncate(cols - 1);
                            });
                        }
                        if ui.small_button("+").clicked() {
                            cells.iter_mut().for_each(|r| r.resize(cols + 1, String::new()));
                        }
                    });
                    let cw = (ui.available_width() - 4.0 * cols as f32) / cols.max(1) as f32;
                    egui::Grid::new("cells").spacing(vec2(4.0, 4.0)).show(ui, |ui| {
                        for row in cells.iter_mut() {
                            row.resize(cols, String::new());
                            for c in row.iter_mut() {
                                ui.add(egui::TextEdit::singleline(c).desired_width(cw - 8.0));
                            }
                            ui.end_row();
                        }
                    });
                    egui::ComboBox::from_id_salt("tfont").selected_text(family.as_str()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        for name in FONTS {
                            ui.selectable_value(family, name.to_string(), name);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Taille").size(12.5).color(MUTED));
                        ui.add(egui::DragValue::new(size).range(4.0..=72.0).speed(0.2).suffix(" pt"));
                    });
                    color_row(ui, "Texte", color, &swatches);
                    color_row(ui, "Traits", line, &swatches);
                    let mut a = *talign as usize;
                    if pro::segmented(ui, &mut a, &["Gauche", "Centre", "Droite"]) {
                        *talign = a as u8;
                    }
                    ui.checkbox(first_col, "Première colonne en gras");
                    let mut on = header.is_some();
                    if ui.checkbox(&mut on, "Ligne d'en-tête").changed() {
                        *header = on.then_some([30, 30, 40, 255]);
                    }
                    if let Some(h) = header {
                        color_row(ui, "Fond de l'en-tête", h, &swatches);
                    }
                    let mut on = stripe.is_some();
                    if ui.checkbox(&mut on, "Lignes alternées").changed() {
                        *stripe = on.then_some([245, 245, 248, 255]);
                    }
                    if let Some(s) = stripe {
                        color_row(ui, "Fond alterné", s, &swatches);
                    }
                });
            }
        }
        pro::dock_panel(ui, "look", "Aspect", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(80.0);
            ui.label(RichText::new("Opacité").size(12.5).color(MUTED));
            let mut op = f.opacity * 100.0;
            if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).suffix(" %")).changed() {
                f.opacity = op / 100.0;
            }
            if !matches!(f.kind, Kind::Line { .. }) {
                let mut on = f.stroke.is_some();
                if ui.checkbox(&mut on, "Contour").changed() {
                    f.stroke = on.then_some([20, 20, 24, 255]);
                }
                if let Some(c) = &mut f.stroke {
                    color_row(ui, "Couleur du contour", c, &swatches);
                    ui.add(egui::Slider::new(&mut f.stroke_w, 0.25..=20.0).suffix(" pt"));
                    ui.label(RichText::new("Pointillés").size(12.5).color(MUTED));
                    ui.add(egui::Slider::new(&mut f.dash, 0.0..=40.0).suffix(" pt"));
                }
            }
            ui.label(RichText::new("Mode de fusion").size(12.5).color(MUTED));
            egui::ComboBox::from_id_salt("fblend").selected_text(BLENDS[(f.fx.blend as usize).min(BLENDS.len() - 1)].0).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                for (k, (n, _)) in BLENDS.iter().enumerate() {
                    ui.selectable_value(&mut f.fx.blend, k as u8, *n);
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut f.locked, "Verrouillé");
                ui.checkbox(&mut f.hidden, "Masqué");
            });
            ui.checkbox(&mut f.fx.nonprint, "Non imprimable (visible à l'écran seulement)");
            if let Kind::Rect { fill, .. } | Kind::Ellipse { fill } | Kind::Polygon { fill, .. } = &f.kind
                && pro::ghost(ui, "+ Ajouter la couleur au nuancier").clicked()
            {
                add_swatch = Some(*fill);
            }
        });
        pro::dock_panel(ui, "fx", "Effets", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 100.0).max(80.0);
            ui.checkbox(&mut f.shadow, "Ombre portée");
            if f.shadow {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("X").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut f.fx.shadow_dx).range(-60.0..=60.0).speed(0.2).suffix(" pt"));
                    ui.label(RichText::new("Y").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut f.fx.shadow_dy).range(-60.0..=60.0).speed(0.2).suffix(" pt"));
                    ui.label(RichText::new("Flou").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut f.fx.shadow_blur).range(0.0..=60.0).speed(0.2));
                });
                color_row(ui, "Couleur de l'ombre", &mut f.fx.shadow_color, &swatches);
            }
            ui.checkbox(&mut f.fx.glow, "Lueur externe");
            if f.fx.glow {
                color_row(ui, "Couleur de la lueur", &mut f.fx.glow_color, &swatches);
                ui.add(egui::Slider::new(&mut f.fx.glow_size, 0.5..=40.0).suffix(" pt"));
            }
            ui.label(RichText::new("Contour progressif").size(12.5).color(MUTED));
            ui.add(egui::Slider::new(&mut f.fx.feather, 0.0..=60.0).suffix(" pt"));
            ui.checkbox(&mut f.fx.gfeather, "Contour progressif dégradé");
            if f.fx.gfeather {
                ui.add(egui::Slider::new(&mut f.fx.gf_angle, -180.0..=180.0).suffix("°"));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("De").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut f.fx.gf_start).range(0.0..=1.0).speed(0.01));
                    ui.label(RichText::new("à").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut f.fx.gf_end).range(0.0..=1.0).speed(0.01));
                });
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("Angles").size(12.5).color(MUTED));
                egui::ComboBox::from_id_salt("corner").selected_text(CORNERS[(f.fx.corner as usize).min(CORNERS.len() - 1)]).width(120.0).show_ui(ui, |ui| {
                    for (k, n) in CORNERS.iter().enumerate() {
                        ui.selectable_value(&mut f.fx.corner, k as u8, *n);
                    }
                });
                if f.fx.corner > 0 {
                    ui.add(egui::DragValue::new(&mut f.fx.corner_r).range(0.0..=200.0).suffix(" pt"));
                }
            });
            ui.checkbox(&mut f.fx.radial, "Dégradé radial (si un dégradé est actif)");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Inclinaison").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut f.fx.skew).range(-60.0..=60.0).speed(0.5).suffix("°"));
                ui.checkbox(&mut f.fx.flip_h, "Miroir ↔");
                ui.checkbox(&mut f.fx.flip_v, "↕");
            });
        });
        pro::dock_panel(ui, "wrap", "Habillage du texte", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            egui::ComboBox::from_id_salt("wrapk").selected_text(model::WRAPS[(f.fx.wrap as usize).min(3)]).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                for (k, n) in model::WRAPS.iter().enumerate() {
                    ui.selectable_value(&mut f.fx.wrap, k as u8, *n);
                }
            });
            if f.fx.wrap > 0 {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Distance").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut f.fx.wrap_off).range(0.0..=80.0).suffix(" pt"));
                });
                ui.label(RichText::new("Les textes voisins contournent ce bloc.").size(11.5).color(FAINT));
            }
        });
        pro::dock_panel(ui, "note", "Note", |ui| {
            ui.add(egui::TextEdit::multiline(&mut f.fx.note).desired_rows(2).hint_text("Remarque pour toi ou un relecteur (non imprimée)").desired_width(f32::INFINITY));
        });
        pro::dock_panel(ui, "arrange", "Disposition", |ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            let w = (ui.available_width() - 8.0) / 3.0;
            ui.label(RichText::new("Aligner sur les marges").size(12.5).color(MUTED));
            for row in [[("Gauche", 0, 0.0), ("Centre ↔", 0, 0.5), ("Droite", 0, 1.0)], [("Haut", 1, 0.0), ("Milieu ↕", 1, 0.5), ("Bas", 1, 1.0)]] {
                ui.horizontal(|ui| {
                    for (label, axis, to) in row {
                        if ui.add_sized([w, 26.0], egui::Button::new(RichText::new(label).size(12.0))).clicked() {
                            align = Some((axis, to));
                        }
                    }
                });
            }
            ui.add_space(4.0);
            ui.label(RichText::new("Ordre").size(12.5).color(MUTED));
            let w4 = (ui.available_width() - 12.0) / 4.0;
            ui.horizontal(|ui| {
                for (label, to) in [("Devant", i32::MAX), ("Avancer", 1), ("Reculer", -1), ("Derrière", i32::MIN)] {
                    if ui.add_sized([w4, 26.0], egui::Button::new(RichText::new(label).size(11.5))).clicked() {
                        order = Some(to);
                    }
                }
            });
        });
        if fit {
            self.fit_text();
        }
        if fit_img {
            self.fit_frame_to_image();
        }
        if edit {
            self.edit_text = Some(i);
        }
        if let Some(to) = order {
            self.order(to);
        }
        if let Some((axis, to)) = align {
            self.align(axis, to, false);
        }
        if let Some(k) = style {
            self.apply_style(k);
        }
        if save_style {
            self.style_name = Some(format!("Style {}", self.doc.styles.len() + 1));
        }
        if let Some(c) = add_swatch
            && !self.doc.swatches.contains(&c)
        {
            self.doc.swatches.push(c);
        }
    }

    fn doc_panel(&mut self, ui: &mut Ui) {
        let page = self.page;
        pro::dock_panel(ui, "doc", "Document", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{:.0} × {:.0} mm", self.doc.width / MM, self.doc.height / MM)).size(12.5).color(INK));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::secondary(ui, "Format…").clicked() {
                        self.doc_sheet = true;
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Titre").size(12.5).color(MUTED));
                ui.add(egui::TextEdit::singleline(&mut self.doc.title).hint_text("{title}").desired_width(ui.available_width() - 16.0));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Marges").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.margin).range(0.0..=300.0).suffix(" pt"));
                ui.label(RichText::new("Colonnes").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.grid).range(1..=12));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Gouttière").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.grid_gutter).range(0.0..=80.0).suffix(" pt"));
                ui.label(RichText::new("Grille").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.baseline).range(0.0..=60.0).suffix(" pt")).on_hover_text("Grille de base (0 = masquée)");
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Fond perdu").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.bleed).range(0.0..=36.0).suffix(" pt")).on_hover_text("3 mm = 8,5 pt pour un imprimeur");
                ui.label(RichText::new("1re page n°").size(12.5).color(MUTED));
                ui.add(egui::DragValue::new(&mut self.doc.first_number).range(1..=9999));
            });
            let sw = self.doc.swatches.clone();
            color_row(ui, "Papier", &mut self.doc.paper, &sw);
            if page != MASTER {
                let mut use_master = !self.doc.pages[page].no_master;
                if ui.checkbox(&mut use_master, "Utiliser le gabarit sur cette page").changed() {
                    self.doc.pages[page].no_master = !use_master;
                }
            } else {
                ui.label(RichText::new("Tu modifies le gabarit : ses blocs apparaissent sur toutes les pages.").size(12.0).color(FAINT));
            }
            if pro::secondary(ui, "?  Tutoriels").clicked() {
                self.help.learn("Texte");
            }
        });
        pro::dock_panel(ui, "swatches", "Nuancier", |ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            if self.doc.swatches.is_empty() {
                ui.label(RichText::new("Les couleurs du document, réutilisables partout d'un clic.").size(12.0).color(FAINT));
            }
            let mut remove = None;
            ui.horizontal_wrapped(|ui| {
                for (k, c) in self.doc.swatches.iter_mut().enumerate() {
                    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                    let r = ui.color_edit_button_srgba(&mut col);
                    *c = col.to_srgba_unmultiplied();
                    r.context_menu(|ui| {
                        if ui.button("Supprimer cette couleur").clicked() {
                            remove = Some(k);
                            ui.close();
                        }
                    });
                }
                if ui.button("+").on_hover_text("Ajouter une couleur").clicked() {
                    self.doc.swatches.push([10, 132, 255, 255]);
                }
            });
            if let Some(k) = remove {
                self.doc.swatches.remove(k);
            }
        });
        pro::dock_panel(ui, "styles", "Styles de paragraphe", |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            if self.doc.styles.is_empty() {
                ui.label(RichText::new("Sélectionne un texte puis « Créer un style » pour réutiliser sa mise en forme.").size(12.0).color(FAINT));
            }
            let mut remove = None;
            for (k, s) in self.doc.styles.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&s.name).size(12.5).color(INK));
                    ui.label(RichText::new(format!("{} {:.1} pt", s.family, s.size)).size(11.0).color(FAINT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("×").clicked() {
                            remove = Some(k);
                        }
                    });
                });
            }
            if let Some(k) = remove {
                self.doc.styles.remove(k);
            }
        });
        pro::dock_panel(ui, "guides", "Repères", |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            if self.doc.guides.is_empty() {
                ui.label(RichText::new("Glisse depuis une règle pour poser un repère ; ramène-le sur la règle pour le retirer.").size(12.0).color(FAINT));
            }
            let mut remove = None;
            for (k, g) in self.doc.guides.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if g.vertical { "Vertical" } else { "Horizontal" }).size(12.0).color(MUTED));
                    ui.add(egui::DragValue::new(&mut g.pos).speed(0.5).suffix(" pt"));
                    if ui.small_button("×").clicked() {
                        remove = Some(k);
                    }
                });
            }
            if let Some(k) = remove {
                self.doc.guides.remove(k);
            }
            if pro::ghost(ui, "+ Repère vertical au centre").clicked() {
                self.doc.guides.push(Guide { vertical: true, pos: self.doc.width / 2.0 });
            }
            if pro::ghost(ui, "+ Repère horizontal au centre").clicked() {
                self.doc.guides.push(Guide { vertical: false, pos: self.doc.height / 2.0 });
            }
        });
    }

    fn objects_panel(&mut self, ui: &mut Ui) {
        let mut pick = None;
        let mut toggles: Vec<(usize, u8)> = Vec::new();
        let sel = self.sel;
        let items: Vec<(usize, String, bool, bool, u8)> = self
            .frames_ref()
            .iter()
            .enumerate()
            .rev()
            .map(|(i, f)| {
                let k = match f.kind {
                    Kind::Text { .. } => 0,
                    Kind::Image { .. } => 1,
                    Kind::Rect { .. } => 2,
                    Kind::Ellipse { .. } => 3,
                    Kind::Line { .. } => 4,
                    Kind::Polygon { .. } => 5,
                    Kind::Table { .. } => 6,
                    Kind::Path { .. } => 7,
                    Kind::TextPath { .. } => 8,
                };
                (i, f.title(), f.hidden, f.locked, k)
            })
            .collect();
        pro::dock_panel(ui, "objects", "Objets de la page", |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            if items.is_empty() {
                ui.label(RichText::new("Aucun bloc sur cette page.").size(12.0).color(FAINT));
            }
            for (i, name, hidden, locked, k) in items {
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
                let p = ui.painter();
                if sel == Some(i) {
                    p.rect_filled(r, CornerRadius::same(6), BLUE.gamma_multiply(0.28));
                } else if resp.hovered() {
                    p.rect_filled(r, CornerRadius::same(6), Color32::from_white_alpha(8));
                }
                let t = [Tool::Text, Tool::FrameRect, Tool::Rect, Tool::Ellipse, Tool::Line, Tool::Polygon, Tool::Table, Tool::Pen, Tool::TextPath][k as usize];
                tool_icon(p, Rect::from_center_size(r.left_center() + vec2(14.0, 0.0), Vec2::splat(12.0)), t, MUTED);
                p.text(r.left_center() + vec2(28.0, 0.0), Align2::LEFT_CENTER, name, FontId::proportional(12.5), if hidden { FAINT } else { INK });
                for (n, (label, on)) in [("👁", !hidden), ("🔒", locked)].into_iter().enumerate() {
                    let br = Rect::from_center_size(r.right_center() - vec2(14.0 + n as f32 * 22.0, 0.0), Vec2::splat(20.0));
                    let b = ui.interact(br, ui.make_persistent_id(("obj", i, n)), Sense::click());
                    let shown = if n == 0 { true } else { on || b.hovered() };
                    if shown {
                        ui.painter().text(br.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), if (n == 0 && on) || (n == 1 && on) { INK } else { FAINT });
                    }
                    if b.clicked() {
                        toggles.push((i, n as u8));
                    }
                }
                if resp.clicked() {
                    pick = Some(i);
                }
            }
        });
        for (i, n) in toggles {
            let f = &mut self.frames()[i];
            if n == 0 {
                f.hidden = !f.hidden;
            } else {
                f.locked = !f.locked;
            }
        }
        if let Some(i) = pick {
            self.sel = Some(i);
        }
    }

    fn snap_value(&self, v: f32, size: f32, axis: usize, tol: f32, skip: Option<usize>) -> (f32, Option<f32>) {
        if !self.snap {
            return (v, None);
        }
        let (m, span) = (self.doc.margin, if axis == 0 { self.doc.width } else { self.doc.height });
        let mut lines = vec![0.0, m, span / 2.0, span - m, span];
        lines.extend(self.doc.guides.iter().filter(|g| g.vertical == (axis == 0)).map(|g| g.pos));
        let g = self.doc.grid.max(1);
        if axis == 0 && g > 1 {
            let cw = (span - 2.0 * m - self.doc.grid_gutter * (g as f32 - 1.0)) / g as f32;
            for k in 0..g {
                let x = m + k as f32 * (cw + self.doc.grid_gutter);
                lines.push(x);
                lines.push(x + cw);
            }
        }
        for (i, f) in self.frames_ref().iter().enumerate() {
            if Some(i) == skip || f.hidden {
                continue;
            }
            let (a, s) = if axis == 0 { (f.x, f.w) } else { (f.y, f.h) };
            lines.extend([a, a + s / 2.0, a + s]);
        }
        let mut best: Option<(f32, f32, f32)> = None;
        for (edge, off) in [(v, 0.0), (v + size / 2.0, size / 2.0), (v + size, size)] {
            for l in &lines {
                let d = (edge - l).abs();
                if d <= tol && best.is_none_or(|b| d < b.0) {
                    best = Some((d, l - off, *l));
                }
            }
        }
        match best {
            Some((_, nv, line)) => (nv, Some(line)),
            None => (v, None),
        }
    }

    fn canvas(&mut self, root: &mut Ui, name: &str) {
        egui::CentralPanel::default().frame(EFrame::new().fill(PASTEBOARD)).show(root, |ui| {
            let full = ui.max_rect();
            let tab_bar = Rect::from_min_max(full.min, pos2(full.right(), full.top() + 32.0));
            ui.painter().rect_filled(tab_bar, CornerRadius::ZERO, PANEL);
            let where_ = if self.page == MASTER { "Gabarit".to_string() } else { format!("page {}", self.doc.page_label(self.page)) };
            let label = format!("{name} · {where_} @ {:.0} %{}", self.zoom * 100.0, if self.doc != self.saved { " •" } else { "" });
            let g = ui.painter().layout_no_wrap(label, FontId::proportional(12.5), INK);
            let tab = Rect::from_min_size(tab_bar.min + vec2(8.0, 4.0), vec2(g.size().x + 28.0, 28.0));
            ui.painter().rect_filled(tab, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, PASTEBOARD);
            ui.painter().galley(tab.left_center() + vec2(14.0, -g.size().y / 2.0), g, INK);
            let rw = if self.rulers { 18.0 } else { 0.0 };
            let area = Rect::from_min_max(pos2(full.left() + rw, tab_bar.bottom() + rw), full.max);
            if self.fit_pending {
                self.zoom = ((area.height() - 80.0) / self.doc.height).min((area.width() - 80.0) / self.doc.width).clamp(0.1, 4.0);
                self.fit_pending = false;
            }
            let (scroll, ctrl) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.command));
            if ctrl && scroll != 0.0 && ui.rect_contains_pointer(area) {
                self.zoom = (self.zoom * (scroll / 400.0).exp()).clamp(0.1, 6.0);
            }
            let px = ui.ctx().pixels_per_point();
            let size = vec2(self.doc.width, self.doc.height) * self.zoom;
            let show_guides = self.guides && !self.preview;
            let page_rect = ui
                .scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                        let avail = ui.available_size();
                        let pad = 60.0 + self.doc.bleed * self.zoom;
                        let (outer, _) = ui.allocate_exact_size(vec2(avail.x.max(size.x + 2.0 * pad), (size.y + 2.0 * pad).max(avail.y)), Sense::hover());
                        let page_rect = Rect::from_min_size(pos2(outer.center().x - size.x / 2.0, outer.min.y + pad), size);
                        let scale = self.zoom * px;
                        if self.rendered.as_ref().is_none_or(|(d, p, s, gd)| *d != self.doc || *p != self.page || (*s - scale).abs() > 1e-3 || *gd != show_guides) {
                            let (svg, over) = self.doc.to_svg(self.page, &self.measure(), show_guides);
                            self.overflow = over;
                            match usvg::Tree::from_str(&svg, &self.opts()) {
                                Ok(tree) => {
                                    if let Some(mut pix) = tiny_skia::Pixmap::new((self.doc.width * scale).ceil() as u32, (self.doc.height * scale).ceil() as u32) {
                                        resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
                                        let img = egui::ColorImage::from_rgba_premultiplied([pix.width() as usize, pix.height() as usize], pix.data());
                                        match &mut self.tex {
                                            Some(t) => t.set(img, TextureOptions::LINEAR),
                                            None => self.tex = Some(ui.ctx().load_texture("page", img, TextureOptions::LINEAR)),
                                        }
                                    }
                                }
                                Err(e) => self.notice = Some((format!("Rendu : {e}"), false)),
                            }
                            self.rendered = Some((self.doc.clone(), self.page, scale, show_guides));
                        }
                        let painter = ui.painter();
                        let z = self.zoom;
                        let to_doc = |p: Pos2| [(p.x - page_rect.min.x) / z, (p.y - page_rect.min.y) / z];
                        let to_scr = |p: [f32; 2]| pos2(page_rect.min.x + p[0] * z, page_rect.min.y + p[1] * z);
                        painter.add(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(120) }.as_shape(page_rect, CornerRadius::ZERO));
                        if let Some(t) = &self.tex {
                            painter.image(t.id(), page_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                        }
                        if !self.preview {
                            let b = self.doc.bleed;
                            if b > 0.0 && show_guides {
                                painter.rect_stroke(Rect::from_min_max(to_scr([-b, -b]), to_scr([self.doc.width + b, self.doc.height + b])), CornerRadius::ZERO, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 59, 48, 140)), StrokeKind::Middle);
                            }
                            for gd in &self.doc.guides {
                                let (a, c) = if gd.vertical { (to_scr([gd.pos, -40.0]), to_scr([gd.pos, self.doc.height + 40.0])) } else { (to_scr([-40.0, gd.pos]), to_scr([self.doc.width + 40.0, gd.pos])) };
                                painter.line_segment([a, c], Stroke::new(1.0, Color32::from_rgb(0, 200, 255)));
                            }
                            for (vertical, pos) in &self.snap_lines {
                                let (a, c) = if *vertical { (to_scr([*pos, -20.0]), to_scr([*pos, self.doc.height + 20.0])) } else { (to_scr([-20.0, *pos]), to_scr([self.doc.width + 20.0, *pos])) };
                                painter.line_segment([a, c], Stroke::new(1.0, Color32::from_rgb(255, 47, 160)));
                            }
                            for (i, f) in self.frames_ref().iter().enumerate() {
                                let r = Rect::from_min_max(to_scr([f.x, f.y]), to_scr([f.x + f.w, f.y + f.h]));
                                if self.sel == Some(i) {
                                    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.5, BLUE), StrokeKind::Middle);
                                    for (hx, hy) in HANDLES {
                                        let c = to_scr([f.x + f.w * hx, f.y + f.h * hy]);
                                        painter.rect(Rect::from_center_size(c, Vec2::splat(8.0)), CornerRadius::same(2), Color32::WHITE, Stroke::new(1.2, BLUE), StrokeKind::Inside);
                                    }
                                    if let Kind::Text { link: Some(l), .. } = &f.kind
                                        && let Some(t) = self.frames_ref().get(*l)
                                    {
                                        let a = r.right_bottom();
                                        let b = to_scr([t.x, t.y]);
                                        painter.line_segment([a, b], Stroke::new(1.2, Color32::from_rgb(0, 200, 255)));
                                        painter.circle_filled(b, 3.5, Color32::from_rgb(0, 200, 255));
                                    }
                                } else if show_guides && !f.hidden {
                                    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(0.5, Color32::from_rgba_unmultiplied(80, 140, 255, 90)), StrokeKind::Middle);
                                }
                                if f.locked {
                                    painter.text(r.right_top() + vec2(-6.0, 8.0), Align2::RIGHT_CENTER, "🔒", FontId::proportional(10.0), FAINT);
                                }
                                if self.overflow.get(i).copied().unwrap_or(false) {
                                    let q = Rect::from_center_size(r.right_bottom() - vec2(6.0, 6.0), Vec2::splat(11.0));
                                    painter.rect_filled(q, CornerRadius::ZERO, RED);
                                    painter.text(q.center(), Align2::CENTER_CENTER, "+", FontId::monospace(10.0), Color32::WHITE);
                                }
                            }
                        }
                        let resp = ui.interact(outer, ui.scope_id().with("page"), Sense::click_and_drag());
                        let tol = 6.0 / z;
                        if let Some(sp) = resp.interact_pointer_pos() {
                            let p = to_doc(sp);
                            if resp.double_clicked() {
                                if let Some(i) = self.sel
                                    && matches!(self.frames_ref()[i].kind, Kind::Text { .. })
                                    && self.frames_ref()[i].contains(p, tol)
                                {
                                    self.edit_text = Some(i);
                                }
                            } else if resp.drag_started() || (resp.clicked() && matches!(self.drag, Drag::None)) {
                                let mods = ui.input(|i| i.modifiers);
                                self.press(p, tol, mods);
                            }
                            if resp.dragged() {
                                let shift = ui.input(|i| i.modifiers.shift);
                                self.drag_to(p, tol, shift);
                            }
                        }
                        if resp.drag_stopped() || resp.clicked() {
                            self.release();
                        }
                        if !self.preview {
                            self.tool_overlay(ui.painter(), &to_scr);
                            for k in self.selection() {
                                if Some(k) != self.sel
                                    && let Some(f) = self.frames_ref().get(k)
                                {
                                    ui.painter().rect_stroke(Rect::from_min_max(to_scr([f.x, f.y]), to_scr([f.x + f.w, f.y + f.h])), CornerRadius::ZERO, Stroke::new(1.2, BLUE.gamma_multiply(0.7)), StrokeKind::Middle);
                                }
                            }
                        }
                        if resp.hovered()
                            && let Some(hp) = resp.hover_pos()
                        {
                            let p = to_doc(hp);
                            let cursor = match self.tool {
                                Tool::Select => self
                                    .sel
                                    .and_then(|i| self.frames_ref().get(i))
                                    .and_then(|f| HANDLES.iter().find(|(hx, hy)| (f.x + f.w * hx - p[0]).abs() <= tol && (f.y + f.h * hy - p[1]).abs() <= tol))
                                    .map(|(hx, hy)| match (*hx == 0.5, *hy == 0.5, (hx - 0.5) * (hy - 0.5) > 0.0) {
                                        (true, _, _) => egui::CursorIcon::ResizeVertical,
                                        (_, true, _) => egui::CursorIcon::ResizeHorizontal,
                                        (_, _, true) => egui::CursorIcon::ResizeNwSe,
                                        _ => egui::CursorIcon::ResizeNeSw,
                                    })
                                    .unwrap_or(egui::CursorIcon::Default),
                                t => tools::cursor(t),
                            };
                            ui.ctx().set_cursor_icon(cursor);
                        }
                        page_rect
                    })
                    .inner
                })
                .inner;
            if self.rulers {
                self.draw_rulers(ui, full, tab_bar, area, page_rect);
            }
        });
    }

    fn draw_rulers(&mut self, ui: &mut Ui, full: Rect, tab_bar: Rect, area: Rect, page_rect: Rect) {
        let z = self.zoom;
        let top = Rect::from_min_max(pos2(area.left(), tab_bar.bottom()), pos2(full.right(), area.top()));
        let left = Rect::from_min_max(pos2(full.left(), area.top()), pos2(area.left(), full.bottom()));
        let corner = Rect::from_min_max(pos2(full.left(), tab_bar.bottom()), pos2(area.left(), area.top()));
        let p = ui.painter();
        let bg = Color32::from_rgb(0x1d, 0x1d, 0x21);
        for r in [top, left, corner] {
            p.rect_filled(r, CornerRadius::ZERO, bg);
        }
        p.line_segment([top.left_bottom(), top.right_bottom()], Stroke::new(1.0, LINE));
        p.line_segment([left.right_top(), left.right_bottom()], Stroke::new(1.0, LINE));
        let step_mm = [1.0, 5.0, 10.0, 20.0, 50.0, 100.0].into_iter().find(|s| s * MM * z >= 40.0).unwrap_or(200.0);
        let sub = step_mm / 5.0;
        let pt = p.with_clip_rect(top);
        let mut k = (-(page_rect.left() - top.left()) / (sub * MM * z)).floor() as i64 - 1;
        loop {
            let mm = k as f32 * sub;
            let x = page_rect.left() + mm * MM * z;
            if x > top.right() {
                break;
            }
            let major = (mm / step_mm).fract().abs() < 1e-3;
            pt.line_segment([pos2(x, top.bottom() - if major { 9.0 } else { 4.0 }), pos2(x, top.bottom())], Stroke::new(1.0, FAINT));
            if major {
                pt.text(pos2(x + 2.0, top.top() + 1.0), Align2::LEFT_TOP, format!("{mm:.0}"), FontId::monospace(9.0), MUTED);
            }
            k += 1;
        }
        let pl = p.with_clip_rect(left);
        let mut k = (-(page_rect.top() - left.top()) / (sub * MM * z)).floor() as i64 - 1;
        loop {
            let mm = k as f32 * sub;
            let y = page_rect.top() + mm * MM * z;
            if y > left.bottom() {
                break;
            }
            let major = (mm / step_mm).fract().abs() < 1e-3;
            pl.line_segment([pos2(left.right() - if major { 9.0 } else { 4.0 }, y), pos2(left.right(), y)], Stroke::new(1.0, FAINT));
            if major {
                pl.text(pos2(left.left() + 2.0, y + 2.0), Align2::LEFT_TOP, format!("{mm:.0}"), FontId::monospace(9.0), MUTED);
            }
            k += 1;
        }
        p.text(corner.center(), Align2::CENTER_CENTER, "mm", FontId::proportional(8.0), FAINT);
        for (rect, vertical) in [(top, false), (left, true)] {
            let r = ui.interact(rect, ui.make_persistent_id(("ruler", vertical)), Sense::drag()).on_hover_text("Glisse vers la page pour poser un repère");
            if r.drag_started() {
                self.new_guide = Some(vertical);
            }
            if self.new_guide == Some(vertical)
                && let Some(pp) = ui.input(|i| i.pointer.hover_pos())
            {
                let line = if vertical { [pos2(pp.x, area.top()), pos2(pp.x, area.bottom())] } else { [pos2(area.left(), pp.y), pos2(area.right(), pp.y)] };
                ui.painter().line_segment(line, Stroke::new(1.0, Color32::from_rgb(0, 200, 255)));
                if !ui.input(|i| i.pointer.primary_down()) {
                    if area.contains(pp) {
                        let pos = if vertical { (pp.x - page_rect.left()) / z } else { (pp.y - page_rect.top()) / z };
                        self.doc.guides.push(Guide { vertical, pos: (pos * 2.0).round() / 2.0 });
                    }
                    self.new_guide = None;
                }
            }
        }
    }

    fn text_editor(&mut self, ctx: &egui::Context) {
        let Some(i) = self.edit_text else { return };
        let page = self.page;
        let mut close = false;
        let mut fit = false;
        let fr = if page == MASTER { &mut self.doc.master.frames } else { &mut self.doc.pages[page].frames };
        let Some(f) = fr.get_mut(i) else {
            self.edit_text = None;
            return;
        };
        let Kind::Text { text, family, size, .. } = &mut f.kind else {
            self.edit_text = None;
            return;
        };
        let fam = family.clone();
        let sz = *size;
        let styles: Vec<String> = self.doc.char_styles.iter().map(|c| c.name.clone()).collect();
        let mut wrap: Option<(String, String)> = None;
        let mut map: Option<u8> = None;
        let mut track: Option<f32> = None;
        let mut cursor = self.edit_cursor;
        egui::Window::new("Modifier le texte")
            .id(egui::Id::NULL.with("text-editor"))
            .collapsible(false)
            .resizable(true)
            .default_width(420.0)
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -48.0))
            .frame(EFrame::window(&ctx.global_style()).fill(CHROME).corner_radius(CornerRadius::same(14)).inner_margin(Margin::same(14)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{fam} · {sz:.1} pt")).size(12.0).color(MUTED));
                    ui.label(RichText::new("Le rendu se met à jour sur la page en direct").size(11.5).color(FAINT));
                });
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                    for (label, tip, open, close) in [
                        ("G", "Gras", "<b>", "</b>"),
                        ("I", "Italique", "<i>", "</i>"),
                        ("x²", "Exposant", "<sup>", "</sup>"),
                        ("x₂", "Indice", "<sub>", "</sub>"),
                        ("Pc", "Petites capitales", "<sc>", "</sc>"),
                    ] {
                        if ui.small_button(label).on_hover_text(format!("{tip} sur la sélection")).clicked() {
                            wrap = Some((open.into(), close.into()));
                        }
                    }
                    for (k, label, tip) in [(0u8, "AA", "Majuscules"), (1, "aa", "Minuscules"), (2, "Aa", "Majuscule à chaque mot"), (3, "Aa.", "Majuscule en début de phrase"), (4, "«»", "Guillemets français autour"), (5, "⎵", "Espaces multiples réduits")] {
                        if ui.small_button(label).on_hover_text(tip).clicked() {
                            map = Some(k);
                        }
                    }
                    ui.menu_button("Couleur", |ui| {
                        for (name, hex) in [("Rouge", "#e5383b"), ("Bleu", "#2f6fed"), ("Vert", "#2a9d4b"), ("Orange", "#f08c00"), ("Violet", "#7b4ddb"), ("Gris", "#777777"), ("Noir", "#000000"), ("Blanc", "#ffffff")] {
                            if ui.button(name).clicked() {
                                wrap = Some((format!("<c={hex}>"), "</c>".into()));
                                ui.close();
                            }
                        }
                    });
                    if !styles.is_empty() {
                        ui.menu_button("Style de caractère", |ui| {
                            for n in &styles {
                                if ui.button(n).clicked() {
                                    wrap = Some((format!("<st={}>", n.replace(' ', "_")), "</st>".into()));
                                    ui.close();
                                }
                            }
                        });
                    }
                    ui.menu_button("Approche", |ui| {
                        for t in TRACKS {
                            if ui.button(format!("{t:+} pt")).clicked() {
                                track = Some(t);
                                ui.close();
                            }
                        }
                    });
                });
                let out = egui::TextEdit::multiline(text).desired_rows(8).desired_width(f32::INFINITY).font(FontId::proportional(14.0)).show(ui);
                if let Some(cr) = out.cursor_range {
                    cursor = (cr.primary.index.0, cr.secondary.index.0);
                }
                let r = out.response;
                if !r.has_focus() && ui.memory(|m| m.focused().is_none()) {
                    r.request_focus();
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Échap pour fermer · {page} {pages} {title} {date}").size(11.5).color(FAINT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::primary(ui, "Terminé").clicked() {
                            close = true;
                        }
                        if pro::ghost(ui, "Ajuster la hauteur").clicked() {
                            fit = true;
                        }
                    });
                });
            });
        self.edit_cursor = cursor;
        if let Some((o, c)) = wrap {
            self.wrap_selection(&o, &c);
        }
        if let Some(k) = map {
            self.map_selection(|t| match k {
                0 => t.to_uppercase(),
                1 => t.to_lowercase(),
                2 => t.split(' ').map(cap_first).collect::<Vec<_>>().join(" "),
                3 => {
                    let mut up = true;
                    t.chars().map(|c| {
                        let o = if up && c.is_alphabetic() { up = false; c.to_uppercase().collect::<String>() } else { c.to_string() };
                        if ".!?\n".contains(c) {
                            up = true;
                        }
                        o
                    }).collect()
                }
                4 => format!("« {} »", t.trim()),
                _ => t.split(' ').filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" "),
            });
        }
        if let Some(t) = track
            && let Some(f) = self.frames().get_mut(i)
            && let Kind::Text { tracking, .. } = &mut f.kind
        {
            *tracking = t;
        }
        if fit {
            self.sel = Some(i);
            self.fit_text();
        }
        if close {
            self.edit_text = None;
        }
    }

    fn sheets(&mut self, ctx: &egui::Context) {
        if let Some(mut o) = self.export.take() {
            let mut go = false;
            let mut cancel = false;
            let n = self.doc.pages.len();
            let keep = pro::sheet(ctx, "export", 470.0, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(9), BLUE.gamma_multiply(0.25));
                    export_glyph(ui.painter(), r.shrink(9.0), BLUE);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Exporter le document").font(semibold(18.0)).color(INK_STRONG));
                        ui.label(RichText::new(["Vectoriel, toutes les pages dans un fichier, prêt à imprimer", "Image sans perte, transparence conservée", "Image légère pour le web et les messages", "Vectoriel modifiable dans Vector"][o.format]).size(12.0).color(MUTED));
                    });
                });
                ui.add_space(14.0);
                let row = |ui: &mut Ui, s: &str| {
                    ui.label(RichText::new(s).size(12.5).color(MUTED));
                };
                row(ui, "Format");
                pro::segmented(ui, &mut o.format, &FORMATS);
                ui.add_space(8.0);
                row(ui, "Pages");
                ui.horizontal(|ui| {
                    pro::segmented(ui, &mut o.pages, &["Toutes", "Page affichée", "Plage"]);
                    if o.pages == 2 {
                        ui.add(egui::TextEdit::singleline(&mut o.range).hint_text(format!("ex. 1-3, 5 (sur {n})")).desired_width(120.0));
                    }
                });
                ui.add_space(8.0);
                if o.format == 1 || o.format == 2 {
                    row(ui, "Résolution");
                    let labels: Vec<&str> = DPIS.iter().map(|d| d.0).collect();
                    pro::segmented(ui, &mut o.dpi, &labels);
                    let s = DPIS[o.dpi].1 / 72.0;
                    ui.label(RichText::new(format!("{:.0} × {:.0} px par page", self.doc.width * s, self.doc.height * s)).size(11.5).color(FAINT));
                    ui.add_space(8.0);
                }
                if o.format == 2 {
                    row(ui, "Qualité JPEG");
                    ui.add(egui::Slider::new(&mut o.quality, 40..=100).suffix(" %"));
                    ui.add_space(8.0);
                }
                ui.add_enabled_ui(self.doc.bleed > 0.0, |ui| {
                    ui.checkbox(&mut o.marks, "Fond perdu et traits de coupe (impression)");
                });
                if self.doc.bleed <= 0.0 {
                    ui.label(RichText::new("Règle un fond perdu dans le format du document pour l'activer.").size(11.5).color(FAINT));
                }
                ui.checkbox(&mut o.open_after, "Afficher le fichier après l'export");
                if self.overflow.iter().any(|v| *v) {
                    ui.label(RichText::new("Attention : du texte dépasse d'un bloc sur la page affichée.").size(12.0).color(RED));
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
        if self.doc_sheet {
            let mut done = false;
            let keep = pro::sheet(ctx, "docfmt", 500.0, |ui| {
                ui.label(RichText::new("Format du document").font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    for (name, _, w, h) in PRESETS {
                        let on = (self.doc.width - w).abs() < 0.5 && (self.doc.height - h).abs() < 0.5;
                        if ui.selectable_label(on, name).clicked() {
                            self.doc.width = w;
                            self.doc.height = h;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let mut wm = self.doc.width / MM;
                    let mut hm = self.doc.height / MM;
                    ui.label(RichText::new("Largeur").size(12.5).color(MUTED));
                    if ui.add(egui::DragValue::new(&mut wm).range(10.0..=2000.0).speed(0.5).suffix(" mm").max_decimals(1)).changed() {
                        self.doc.width = wm * MM;
                    }
                    ui.label(RichText::new("Hauteur").size(12.5).color(MUTED));
                    if ui.add(egui::DragValue::new(&mut hm).range(10.0..=2000.0).speed(0.5).suffix(" mm").max_decimals(1)).changed() {
                        self.doc.height = hm * MM;
                    }
                });
                ui.horizontal(|ui| {
                    let portrait = self.doc.height >= self.doc.width;
                    if ui.selectable_label(portrait, "Portrait").clicked() && !portrait || ui.selectable_label(!portrait, "Paysage").clicked() && portrait {
                        std::mem::swap(&mut self.doc.width, &mut self.doc.height);
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let mut mm = self.doc.margin / MM;
                    ui.label(RichText::new("Marges").size(12.5).color(MUTED));
                    if ui.add(egui::DragValue::new(&mut mm).range(0.0..=100.0).speed(0.2).suffix(" mm").max_decimals(1)).changed() {
                        self.doc.margin = mm * MM;
                    }
                    let mut bm = self.doc.bleed / MM;
                    ui.label(RichText::new("Fond perdu").size(12.5).color(MUTED));
                    if ui.add(egui::DragValue::new(&mut bm).range(0.0..=12.0).speed(0.1).suffix(" mm").max_decimals(1)).changed() {
                        self.doc.bleed = bm * MM;
                    }
                    if ui.small_button("3 mm").on_hover_text("Valeur standard des imprimeurs").clicked() {
                        self.doc.bleed = 3.0 * MM;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Colonnes").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut self.doc.grid).range(1..=12));
                    ui.label(RichText::new("Gouttière").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut self.doc.grid_gutter).range(0.0..=80.0).suffix(" pt"));
                    ui.label(RichText::new("Grille de base").size(12.5).color(MUTED));
                    ui.add(egui::DragValue::new(&mut self.doc.baseline).range(0.0..=60.0).suffix(" pt"));
                });
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Terminé").clicked() {
                        done = true;
                    }
                });
            });
            if done || !keep {
                self.doc_sheet = false;
            }
        }
        if let Some(mut name) = self.style_name.take() {
            let mut ok = false;
            let mut cancel = false;
            let keep = pro::sheet(ctx, "style", 360.0, |ui| {
                ui.label(RichText::new("Nouveau style de paragraphe").font(semibold(17.0)).color(INK_STRONG));
                ui.add_space(8.0);
                ui.label(RichText::new("Police, taille, graisse, couleur, interligne et espacements du texte sélectionné.").size(12.0).color(MUTED));
                ui.add_space(8.0);
                let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY));
                r.request_focus();
                if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    ok = true;
                }
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Créer").clicked() {
                        ok = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        cancel = true;
                    }
                });
            });
            if ok && !name.trim().is_empty() {
                self.save_style(name.trim().to_string());
            } else if keep && !cancel {
                self.style_name = Some(name);
            }
        }
        self.extra_sheets(ctx);
    }
}

const HANDLES: [(f32, f32); 8] = [(0.0, 0.0), (0.5, 0.0), (1.0, 0.0), (1.0, 0.5), (1.0, 1.0), (0.5, 1.0), (0.0, 1.0), (0.0, 0.5)];

fn mi(ui: &mut Ui, enabled: bool, label: &str, key: &str) -> bool {
    ui.add_enabled(enabled, egui::Button::new(label).shortcut_text(key)).clicked()
}

fn swatch_button(ui: &mut Ui, c: &mut [u8; 4], swatches: &[[u8; 4]]) {
    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    if ui.color_edit_button_srgba(&mut col).changed() {
        *c = col.to_srgba_unmultiplied();
    }
    for s in swatches.iter().take(6) {
        let (r, resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
        ui.painter().circle_filled(r.center(), 6.5, Color32::from_rgb(s[0], s[1], s[2]));
        if resp.on_hover_text("Couleur du nuancier").clicked() {
            *c = *s;
        }
    }
}

fn color_row(ui: &mut Ui, label: &str, c: &mut [u8; 4], swatches: &[[u8; 4]]) {
    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.color_edit_button_srgba(&mut col);
            for s in swatches.iter().rev().take(7) {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                ui.painter().circle_filled(r.center(), 6.0, Color32::from_rgb(s[0], s[1], s[2]));
                if resp.clicked() {
                    col = Color32::from_rgba_unmultiplied(s[0], s[1], s[2], s[3]);
                }
            }
        });
    });
    *c = col.to_srgba_unmultiplied();
}

fn gradient_row(ui: &mut Ui, fill2: &mut Option<[u8; 4]>, angle: &mut f32, swatches: &[[u8; 4]]) {
    let mut on = fill2.is_some();
    if ui.checkbox(&mut on, "Dégradé").changed() {
        *fill2 = on.then_some([10, 132, 255, 255]);
    }
    if let Some(c) = fill2 {
        color_row(ui, "Vers", c, swatches);
        ui.label(RichText::new("Angle").size(12.5).color(MUTED));
        ui.add(egui::Slider::new(angle, -180.0..=180.0).suffix("°"));
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

fn preset_card(ui: &mut Ui, name: &str, kind: &str, w: f32, h: f32, template: bool) -> egui::Response {
    let width = 168.0;
    let (r, resp) = ui.allocate_exact_size(vec2(width, 140.0), Sense::click());
    let p = ui.painter();
    p.rect(r, CornerRadius::same(12), if resp.hovered() { Color32::from_rgb(0x2f, 0x2f, 0x35) } else { Color32::from_rgb(0x27, 0x27, 0x2c) }, Stroke::new(1.0, LINE), StrokeKind::Inside);
    let area = Rect::from_center_size(r.center_top() + vec2(0.0, 48.0), vec2(width - 60.0, 64.0));
    let s = (area.width() / w).min(area.height() / h);
    let sheet = Rect::from_center_size(area.center(), vec2(w, h) * s);
    p.rect(sheet, CornerRadius::same(2), Color32::from_white_alpha(230), Stroke::NONE, StrokeKind::Inside);
    if template {
        let band = Rect::from_min_size(sheet.min, vec2(sheet.width(), sheet.height() * 0.3));
        p.rect_filled(band, CornerRadius { nw: 2, ne: 2, sw: 0, se: 0 }, Color32::from_rgb(255, 112, 52));
        for k in 0..4 {
            let y = band.bottom() + 5.0 + k as f32 * 5.0;
            if y < sheet.bottom() - 3.0 {
                p.line_segment([pos2(sheet.left() + 4.0, y), pos2(sheet.right() - 4.0 - (k as f32 * 6.0) % 14.0, y)], Stroke::new(1.5, Color32::from_gray(170)));
            }
        }
    }
    p.text(pos2(r.center().x, r.bottom() - 36.0), Align2::CENTER_CENTER, name, semibold(13.0), INK_STRONG);
    p.text(pos2(r.center().x, r.bottom() - 17.0), Align2::CENTER_CENTER, kind, FontId::proportional(10.5), MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_decorations(false)
            .with_title("Page")
            .with_icon(Arc::new(studio_ui::app_window_icon("page")))
            .with_inner_size([1480.0, 940.0])
            .with_min_inner_size([1100.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native("Page", options, Box::new(|cc| Ok(Box::new(PageApp::new(cc)))))
}

#[cfg(test)]
mod pdf_tests {
    use super::*;

    #[test]
    fn pdf_export_produces_two_pages() {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let fonts = Fonts { db: Arc::new(db) };
        let mut doc = starter(595.0, 842.0);
        doc.pages.push(Page::default());
        let m = |t: &str, f: &str, b: bool, s: f32| fonts.width(t, f, b, s);
        assert!(m("WWW", "Arial", false, 20.0) > m("iii", "Arial", false, 20.0));
        let opt = usvg::Options { fontdb: fonts.db.clone(), ..Default::default() };
        let mut document = krilla::Document::new();
        for i in 0..2 {
            let (svg, _) = doc.to_svg(i, &m, false);
            assert!(svg.contains(&format!("{} / 2", i + 1)));
            let tree = usvg::Tree::from_str(&svg, &opt).unwrap();
            let size = krilla::geom::Size::from_wh(doc.width, doc.height).unwrap();
            let mut page = document.start_page_with(krilla::page::PageSettings::new(size));
            let mut surface = page.surface();
            surface.draw_svg(&tree, size, SvgSettings::default()).unwrap();
            surface.finish();
            page.finish();
        }
        let bytes = document.finish().unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 1000);
    }

    #[test]
    fn templates_render() {
        let m = |t: &str, _: &str, _: bool, s: f32| t.chars().count() as f32 * s * 0.5;
        for k in 0..TEMPLATES.len() {
            let d = template(k);
            for i in 0..d.pages.len() {
                let (svg, _) = d.to_svg_with(i, &m, false, true);
                usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap_or_else(|e| panic!("modèle {k} : {e}"));
            }
        }
    }
}

#[cfg(test)]
mod dump {
    #[test]
    #[ignore]
    fn write_template() {
        let Ok(path) = std::env::var("PAGE_DEMO") else { return };
        std::fs::write(path, serde_json::to_vec_pretty(&super::template(0)).unwrap()).unwrap();
    }
}

fn cap_first(w: &str) -> String {
    let mut c = w.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().chain(c.flat_map(|x| x.to_lowercase())).collect())
}
