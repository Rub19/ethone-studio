use crate::{Folio, Job, Opened, STAMPS, Tool, engine, render, toast};
use studio_ocr as ocr;
use eframe::egui::{self, Color32, CornerRadius, Rect, RichText, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, pos2, vec2};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use studio_ui::pro::{self, BLUE, FAINT, INK, INK_STRONG, MUTED, semibold};

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum Panel {
    All,
    Comment,
    FillSign,
    Organize,
    Combine,
    Create,
    Ocr,
    Redact,
    Protect,
    Compress,
    Stamp,
    Export,
    Compare,
    Search,
    Bookmarks,
    Props,
    Measure,
}

pub const LIST: [(Panel, &str); 16] = [
    (Panel::Comment, "Commenter"),
    (Panel::FillSign, "Remplir et signer"),
    (Panel::Organize, "Organiser les pages"),
    (Panel::Combine, "Combiner des fichiers"),
    (Panel::Create, "Créer un PDF"),
    (Panel::Ocr, "Numériser et OCR"),
    (Panel::Redact, "Caviarder"),
    (Panel::Protect, "Protéger"),
    (Panel::Compress, "Compresser"),
    (Panel::Stamp, "Filigrane et numérotation"),
    (Panel::Export, "Exporter"),
    (Panel::Compare, "Comparer des fichiers"),
    (Panel::Search, "Rechercher"),
    (Panel::Bookmarks, "Signets"),
    (Panel::Props, "Propriétés"),
    (Panel::Measure, "Mesurer"),
];

const DESC: [&str; 17] = [
    "Surligner, notes, formes, dessin, tampons",
    "Champs de formulaire, signature, date",
    "Pivoter, supprimer, déplacer, insérer, extraire",
    "Fusionner PDF et images",
    "Depuis des images ou une page vierge",
    "Reconnaître le texte des scans",
    "Effacer définitivement des zones",
    "Mot de passe AES-256 et droits",
    "Réduire la taille du fichier",
    "Filigrane, en-tête, pied, numéros, Bates",
    "Images, texte ou Word",
    "Trouver les différences entre versions",
    "Un mot dans tout le document",
    "Table des matières du PDF",
    "Titre, auteur, informations",
    "Distances réelles en mm",
    "Résumé et questions, en local",
];

pub fn by_name(n: &str) -> Option<Panel> {
    Some(match n {
        "comment" => Panel::Comment,
        "fillsign" | "signer" => Panel::FillSign,
        "organize" | "organiser" => Panel::Organize,
        "combine" | "combiner" => Panel::Combine,
        "create" => Panel::Create,
        "ocr" => Panel::Ocr,
        "redact" | "caviarder" => Panel::Redact,
        "protect" | "proteger" => Panel::Protect,
        "compress" | "compresser" => Panel::Compress,
        "stamp" | "filigrane" => Panel::Stamp,
        "export" | "exporter" => Panel::Export,
        "compare" | "comparer" => Panel::Compare,
        "search" | "rechercher" => Panel::Search,
        "bookmarks" | "signets" => Panel::Bookmarks,
        "props" => Panel::Props,
        "measure" => Panel::Measure,
        _ => return None,
    })
}

pub struct Cmp {
    pub name: String,
    pub bytes: Arc<Vec<u8>>,
    pub diffs: Vec<f32>,
}

pub struct State {
    pub combine: Vec<PathBuf>,
    pub langs: Vec<(String, String)>,
    pub ocr_lang: usize,
    pub ocr_scope: usize,
    pub ocr_layer: bool,
    pub ocr_text: String,
    pub redact_ocr: bool,
    pub pw: String,
    pub pw_owner: String,
    pub perm: [bool; 3],
    pub comp_dpi: usize,
    pub comp_q: u8,
    pub st: engine::Stamp,
    pub st_mode: usize,
    pub st_scope: usize,
    pub bates: (u32, usize),
    pub bg: [u8; 3],
    pub exp_fmt: usize,
    pub exp_dpi: usize,
    pub exp_scope: usize,
    pub cmp: Option<Cmp>,
    pub cmp_show: bool,
    pub diff_tex: HashMap<(usize, u64), TextureHandle>,
    pub search: String,
    pub results: Vec<(usize, String)>,
    pub bm: Vec<(usize, String, usize)>,
    pub bm_title: String,
    pub meta: Vec<String>,
    pub fields: Vec<engine::Field>,
    pub vals: Vec<String>,
    pub crop: [f32; 4],
    pub split_n: usize,
    pub blank: usize,
}

impl Default for State {
    fn default() -> Self {
        let langs = ocr::languages();
        let ocr_lang = langs.iter().position(|l| l.0.starts_with("fr")).unwrap_or(0);
        Self {
            combine: Vec::new(),
            langs,
            ocr_lang,
            ocr_scope: 1,
            ocr_layer: true,
            ocr_text: String::new(),
            redact_ocr: true,
            pw: String::new(),
            pw_owner: String::new(),
            perm: [true, false, false],
            comp_dpi: 1,
            comp_q: 70,
            st: stamp_preset(0),
            st_mode: 0,
            st_scope: 1,
            bates: (1, 6),
            bg: [255, 250, 235],
            exp_fmt: 0,
            exp_dpi: 1,
            exp_scope: 1,
            cmp: None,
            cmp_show: false,
            diff_tex: HashMap::new(),
            search: String::new(),
            results: Vec::new(),
            bm: Vec::new(),
            bm_title: String::new(),
            meta: vec![String::new(); engine::META.len()],
            fields: Vec::new(),
            vals: Vec::new(),
            crop: [10.0; 4],
            split_n: 1,
            blank: 0,
        }
    }
}

impl State {
    pub fn loaded(&mut self, o: &Opened) {
        self.meta = engine::info(&o.doc);
        self.bm = engine::outline(&o.doc);
        self.fields = engine::fields(&o.doc);
        self.vals = self.fields.iter().map(|f| f.value.clone()).collect();
        self.results.clear();
        self.ocr_text.clear();
        self.diff_tex.clear();
        self.cmp = None;
    }
}

fn stamp_preset(mode: usize) -> engine::Stamp {
    let s = |text: &str, size: f32, place: u8| engine::Stamp { text: text.into(), size, color: [60, 60, 60], opacity: 1.0, angle: 0.0, bold: false, place, under: false };
    match mode {
        0 => engine::Stamp { text: "CONFIDENTIEL".into(), size: 64.0, color: [200, 30, 40], opacity: 0.22, angle: 45.0, bold: true, place: 0, under: false },
        1 => s("{fichier}", 9.0, 5),
        2 => s("{date}", 9.0, 2),
        3 => s("Page {page} sur {pages}", 9.0, 3),
        _ => engine::Stamp { bold: true, ..s("DOSSIER-", 9.0, 3) },
    }
}

const PLACES: [&str; 7] = ["Centre", "Bas gauche", "Bas centre", "Bas droite", "Haut gauche", "Haut centre", "Haut droite"];
const SIZES: [(&str, f32, f32); 5] = [("A4", 595.0, 842.0), ("A5", 420.0, 595.0), ("A3", 842.0, 1191.0), ("Lettre US", 612.0, 792.0), ("Comme la page courante", 0.0, 0.0)];
const COLORS: [[u8; 3]; 8] = [[255, 214, 10], [255, 69, 58], [48, 209, 88], [10, 132, 255], [191, 90, 242], [255, 149, 0], [20, 20, 20], [255, 255, 255]];

fn title(ui: &mut Ui, app: &mut Folio, t: &str) {
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new(RichText::new("‹ Outils").size(12.0).color(MUTED)).frame(false)).clicked() {
            app.panel = Panel::All;
        }
    });
    ui.label(RichText::new(t).font(semibold(17.0)).color(INK_STRONG));
    ui.add_space(8.0);
}

fn hint(ui: &mut Ui, s: &str) {
    ui.label(RichText::new(s).size(11.5).color(FAINT));
}

fn label(ui: &mut Ui, s: &str) {
    ui.label(RichText::new(s).size(12.0).color(MUTED));
}

fn scope_pick(ui: &mut Ui, v: &mut usize, has_sel: bool) {
    let items: &[&str] = if has_sel { &["Page courante", "Toutes", "Sélection"] } else { &["Page courante", "Toutes"] };
    if !has_sel && *v == 2 {
        *v = 1;
    }
    pro::segmented(ui, v, items);
}

pub fn show(app: &mut Folio, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    egui::Frame::new().inner_margin(egui::Margin::same(14)).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.set_width(ui.available_width());
            match app.panel {
                Panel::All => all(app, ui),
                Panel::Comment => comment(app, ui, &ctx),
                Panel::FillSign => fill_sign(app, ui, &ctx),
                Panel::Organize => organize(app, ui, &ctx),
                Panel::Combine => combine(app, ui, &ctx),
                Panel::Create => create(app, ui, &ctx),
                Panel::Ocr => ocr_panel(app, ui, &ctx),
                Panel::Redact => redact(app, ui, &ctx),
                Panel::Protect => protect(app, ui, &ctx),
                Panel::Compress => compress(app, ui, &ctx),
                Panel::Stamp => stamp(app, ui, &ctx),
                Panel::Export => export(app, ui, &ctx),
                Panel::Compare => compare(app, ui, &ctx),
                Panel::Search => search(app, ui),
                Panel::Bookmarks => bookmarks(app, ui, &ctx),
                Panel::Props => props(app, ui, &ctx),
                Panel::Measure => measure(app, ui, &ctx),
            }
        });
    });
}

fn all(app: &mut Folio, ui: &mut Ui) {
    ui.label(RichText::new("Tous les outils").font(semibold(17.0)).color(INK_STRONG));
    ui.add_space(8.0);
    for ((p, name), d) in LIST.iter().zip(DESC) {
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(8), Color32::from_rgb(0x2a, 0x2a, 0x30));
        }
        let ic = Rect::from_min_size(r.min + vec2(8.0, 9.0), egui::Vec2::splat(30.0));
        ui.painter().rect_filled(ic, CornerRadius::same(8), crate::ACCENT.gamma_multiply(0.2));
        panel_glyph(ui.painter(), ic.shrink(7.0), *p, crate::ACCENT);
        ui.painter().text(r.min + vec2(48.0, 16.0), egui::Align2::LEFT_CENTER, *name, semibold(12.5), INK);
        ui.painter().text(r.min + vec2(48.0, 33.0), egui::Align2::LEFT_CENTER, d, egui::FontId::proportional(11.0), FAINT);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            app.panel = *p;
        }
    }
}

fn color_row(ui: &mut Ui, c: &mut [u8; 3]) {
    ui.horizontal_wrapped(|ui| {
        for k in COLORS {
            let (r, resp) = ui.allocate_exact_size(egui::Vec2::splat(22.0), Sense::click());
            ui.painter().circle_filled(r.center(), 9.0, Color32::from_rgb(k[0], k[1], k[2]));
            if *c == k {
                ui.painter().circle_stroke(r.center(), 11.0, Stroke::new(2.0, BLUE));
            }
            if resp.clicked() {
                *c = k;
            }
        }
        let mut col = Color32::from_rgb(c[0], c[1], c[2]);
        if ui.color_edit_button_srgba(&mut col).changed() {
            *c = [col.r(), col.g(), col.b()];
        }
    });
}

fn tool_buttons(app: &mut Folio, ui: &mut Ui, tools: &[Tool]) {
    ui.horizontal_wrapped(|ui| {
        for t in tools {
            let name = crate::TOOLS.iter().find(|x| x.0 == *t).map_or("", |x| x.1);
            if pro::icon_button(ui, 32.0, app.tool == *t, name, |p, r, c| crate::glyph(p, r, *t, c)).clicked() {
                app.tool = *t;
            }
        }
    });
}

fn comment(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Commenter");
    tool_buttons(app, ui, &[Tool::Highlight, Tool::Underline, Tool::Strike, Tool::Note, Tool::TextBox, Tool::Rect, Tool::Ellipse, Tool::Line, Tool::Arrow, Tool::Ink, Tool::Stamp]);
    ui.add_space(8.0);
    label(ui, "Couleur");
    color_row(ui, &mut app.color);
    pro::option_slider(ui, "Épaisseur", &mut app.width, 0.5..=12.0, " pt", false);
    pro::option_slider(ui, "Taille du texte", &mut app.text_size, 6.0..=48.0, " pt", false);
    ui.add_space(8.0);
    label(ui, "Tampon");
    ui.horizontal_wrapped(|ui| {
        for (k, (name, c)) in STAMPS.iter().enumerate() {
            let t = RichText::new(name.to_uppercase()).size(11.0).color(Color32::from_rgb(c[0], c[1], c[2]));
            if ui.selectable_label(app.stamp == k, t).clicked() {
                app.stamp = k;
                app.tool = Tool::Stamp;
            }
        }
    });
    hint(ui, "Glisse sur la page pour poser le tampon, ou clique pour une taille standard.");
    ui.add_space(10.0);
    if pro::secondary(ui, "Supprimer les commentaires de la page").clicked() {
        let p = app.scope(0);
        app.edit(ctx, "Commentaires supprimés", |d| engine::remove_comments(d, &p).map(|_| ()));
    }
}

fn fill_sign(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Remplir et signer");
    label(ui, "Signature");
    ui.horizontal(|ui| {
        if pro::primary(ui, if app.signature.is_empty() { "Créer ma signature" } else { "Poser ma signature" }).clicked() {
            if app.signature.is_empty() {
                app.sign_pad = Some(Vec::new());
            } else {
                app.tool = Tool::Sign;
                toast(ctx, "Clique sur la page pour signer", true);
            }
        }
        if !app.signature.is_empty() && ui.button("Redessiner").clicked() {
            app.sign_pad = Some(Vec::new());
        }
    });
    ui.add_space(6.0);
    tool_buttons(app, ui, &[Tool::TextBox, Tool::Date, Tool::Sign]);
    hint(ui, "Texte : clique où écrire. Date du jour : un clic.");
    ui.add_space(12.0);
    label(ui, &format!("Champs du formulaire · {}", app.st.fields.len()));
    if app.st.fields.is_empty() {
        hint(ui, "Ce PDF n'a pas de champs : utilise Texte, Date et Signature pour le remplir.");
        return;
    }
    let st = &mut app.st;
    for (f, v) in st.fields.iter().zip(st.vals.iter_mut()) {
        ui.label(RichText::new(&f.name).size(11.5).color(INK));
        if f.check.is_some() {
            let mut on = v == "1" || (!v.is_empty() && v != "Off" && v != "0");
            if ui.checkbox(&mut on, "Coché").changed() {
                *v = if on { "1".into() } else { "0".into() };
            }
        } else {
            ui.add(egui::TextEdit::singleline(v).desired_width(f32::INFINITY));
        }
    }
    ui.add_space(8.0);
    if pro::primary(ui, "Appliquer les valeurs").clicked() {
        let todo: Vec<(engine::Field, String)> = app.st.fields.iter().zip(&app.st.vals).filter(|(f, v)| f.value != **v).map(|(f, v)| (f.clone(), v.clone())).collect();
        let n = todo.len();
        if app.edit(ctx, &format!("{n} champ(s) rempli(s)"), |d| todo.iter().try_for_each(|(f, v)| engine::set_field(d, f, v)))
            && let Some(o) = &app.od
        {
            app.st.fields = engine::fields(&o.doc);
            app.st.vals = app.st.fields.iter().map(|f| f.value.clone()).collect();
        }
    }
}

fn organize(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Organiser les pages");
    let n = app.od.as_ref().map_or(0, |o| o.n());
    let idx = app.scope(2);
    hint(ui, &format!("Agit sur {} : sélectionne des vignettes avec Ctrl+clic ou Maj+clic.", if app.sel.is_empty() { format!("la page {}", app.page + 1) } else { format!("{} page(s) sélectionnée(s)", app.sel.len()) }));
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("↺ Pivoter à gauche").clicked() {
            app.edit(ctx, "Pages pivotées", |d| engine::rotate(d, &idx, -90));
        }
        if ui.button("↻ Pivoter à droite").clicked() {
            app.edit(ctx, "Pages pivotées", |d| engine::rotate(d, &idx, 90));
        }
        if ui.button("Supprimer").clicked() {
            app.edit(ctx, "Pages supprimées", |d| engine::delete(d, &idx));
            app.sel.clear();
        }
        if ui.button("Dupliquer").clicked() {
            let order: Vec<usize> = (0..n).flat_map(|i| if idx.contains(&i) { vec![i, i] } else { vec![i] }).collect();
            app.edit(ctx, "Pages dupliquées", |d| engine::arrange(d, &order));
        }
        if ui.button("▲ Monter").clicked() && idx.first().is_some_and(|f| *f > 0) {
            let mut order: Vec<usize> = (0..n).collect();
            for &i in &idx {
                order.swap(i - 1, i);
            }
            if app.edit(ctx, "", |d| engine::arrange(d, &order)) {
                app.sel = idx.iter().map(|i| i - 1).collect();
                app.goto = Some(idx[0] - 1);
            }
        }
        if ui.button("▼ Descendre").clicked() && idx.last().is_some_and(|l| l + 1 < n) {
            let mut order: Vec<usize> = (0..n).collect();
            for &i in idx.iter().rev() {
                order.swap(i, i + 1);
            }
            if app.edit(ctx, "", |d| engine::arrange(d, &order)) {
                app.sel = idx.iter().map(|i| i + 1).collect();
                app.goto = Some(idx[0] + 1);
            }
        }
        if ui.button("Inverser l'ordre").clicked() {
            let order: Vec<usize> = (0..n).rev().collect();
            app.edit(ctx, "Ordre inversé", |d| engine::arrange(d, &order));
        }
        if ui.button("Tout sélectionner").clicked() {
            app.sel = (0..n).collect();
        }
    });
    ui.add_space(10.0);
    label(ui, "Insérer après la page courante");
    egui::ComboBox::from_id_salt("blank").selected_text(SIZES[app.st.blank].0).show_ui(ui, |ui| {
        for (k, s) in SIZES.iter().enumerate() {
            ui.selectable_value(&mut app.st.blank, k, s.0);
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Page vierge").clicked() {
            let (_, mut w, mut h) = SIZES[app.st.blank];
            if w == 0.0 {
                (w, h) = app.od.as_ref().and_then(|o| o.sizes.get(app.page).copied()).unwrap_or((595.0, 842.0));
            }
            let at = app.page + 1;
            app.edit(ctx, "Page insérée", |d| engine::insert_blank(d, at, w, h));
        }
        if ui.button("Depuis un fichier…").clicked()
            && let Some(files) = rfd::FileDialog::new().add_filter("PDF et images", &["pdf", "png", "jpg", "jpeg"]).pick_files()
        {
            match load_many(&files) {
                Ok(other) => {
                    let at = app.page + 1;
                    app.edit(ctx, "Fichier inséré", |d| engine::insert_file(d, at, other));
                }
                Err(e) => toast(ctx, e, false),
            }
        }
    });
    ui.add_space(10.0);
    label(ui, "Extraire et diviser");
    ui.horizontal(|ui| {
        if ui.button("Extraire la sélection…").clicked()
            && let Some(o) = &app.od
            && let Some(p) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name(format!("{} - extrait.pdf", stem(&o.name))).save_file()
        {
            match engine::extract(&o.doc, &idx).and_then(|b| std::fs::write(&p, b).map_err(|e| e.to_string())) {
                Ok(()) => toast(ctx, format!("Extrait : {}", p.display()), true),
                Err(e) => toast(ctx, e, false),
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Diviser toutes les").size(12.0).color(MUTED));
        ui.add(egui::DragValue::new(&mut app.st.split_n).range(1..=500));
        ui.label(RichText::new("page(s)").size(12.0).color(MUTED));
        if ui.button("Diviser…").clicked()
            && let Some(o) = &app.od
            && let Some(dir) = rfd::FileDialog::new().pick_folder()
        {
            match engine::split(&o.doc, app.st.split_n) {
                Ok(parts) => {
                    let k = parts.len();
                    for (i, b) in parts.into_iter().enumerate() {
                        let _ = std::fs::write(dir.join(format!("{} - partie {}.pdf", stem(&o.name), i + 1)), b);
                    }
                    toast(ctx, format!("{k} fichier(s) créés dans {}", dir.display()), true);
                }
                Err(e) => toast(ctx, e, false),
            }
        }
    });
    ui.add_space(10.0);
    if ui.button("Supprimer les pages blanches").on_hover_text("Détecte les pages presque vides (dos de scans, séparateurs) et les retire").clicked() {
        let n = app.od.as_ref().map_or(0, |o| o.n());
        let blank: Vec<usize> = (0..n).filter(|i| app.render_page(*i, 0.35).is_some_and(|img| render::ink_ratio(&img) < 0.002)).collect();
        if blank.is_empty() {
            toast(ctx, "Aucune page blanche", true);
        } else if blank.len() == n {
            toast(ctx, "Toutes les pages semblent vides : rien n'a été supprimé", false);
        } else {
            let k = blank.len();
            app.edit(ctx, &format!("{k} page(s) blanche(s) supprimée(s)"), |d| engine::delete(d, &blank));
        }
    }
    ui.add_space(6.0);
    label(ui, "Impression économique");
    ui.horizontal(|ui| {
        for (k, name) in [(2usize, "2 pages par feuille"), (4, "4 pages par feuille")] {
            if ui.button(name).on_hover_text("Nouvelle mise en page A4 : chaque feuille regroupe plusieurs pages réduites").clicked() {
                app.edit(ctx, &format!("{name} : document réorganisé"), |d| engine::n_up(d, k));
            }
        }
    });
    ui.add_space(10.0);
    label(ui, "Recadrer (marges en mm : gauche, haut, droite, bas)");
    ui.horizontal(|ui| {
        for v in app.st.crop.iter_mut() {
            ui.add(egui::DragValue::new(v).range(0.0..=200.0).speed(0.5));
        }
    });
    if ui.button("Recadrer la sélection").clicked() {
        let m = app.st.crop.map(|v| v * 72.0 / 25.4);
        app.edit(ctx, "Pages recadrées", |d| engine::crop(d, &idx, m));
    }
}

fn stem(name: &str) -> String {
    name.trim_end_matches(".pdf").trim_end_matches(".PDF").to_string()
}

fn load_many(files: &[PathBuf]) -> engine::R<lopdf::Document> {
    let mut docs = Vec::new();
    let mut imgs = Vec::new();
    for f in files {
        let ext = f.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext == "pdf" {
            if !imgs.is_empty() {
                docs.push(engine::images_to_pdf(&std::mem::take(&mut imgs), Some((595.0, 842.0)))?);
            }
            let b = std::fs::read(f).map_err(|e| e.to_string())?;
            docs.push(engine::load(&b, "").map_err(|e| format!("{} : {e}", f.display()))?);
        } else {
            imgs.push(image::open(f).map_err(|e| format!("{} : {e}", f.display()))?.to_rgba8());
        }
    }
    if !imgs.is_empty() {
        docs.push(engine::images_to_pdf(&imgs, Some((595.0, 842.0)))?);
    }
    engine::merge(docs)
}

fn combine(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Combiner des fichiers");
    hint(ui, "Ajoute des PDF et des images (ou glisse-les dans la fenêtre), mets-les dans l'ordre, puis combine.");
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("Ajouter des fichiers…").clicked()
            && let Some(f) = rfd::FileDialog::new().add_filter("PDF et images", &["pdf", "png", "jpg", "jpeg"]).pick_files()
        {
            app.st.combine.extend(f);
        }
        if let Some(p) = app.od.as_ref().and_then(|o| o.path.clone())
            && ui.button("+ Document ouvert").clicked()
        {
            app.st.combine.push(p);
        }
    });
    let mut act: Option<(usize, i32)> = None;
    for (k, f) in app.st.combine.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{}. {}", k + 1, f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())).size(12.0).color(INK));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕").clicked() {
                    act = Some((k, 0));
                }
                if ui.small_button("▼").clicked() {
                    act = Some((k, 1));
                }
                if ui.small_button("▲").clicked() {
                    act = Some((k, -1));
                }
            });
        });
    }
    if let Some((k, d)) = act {
        let n = app.st.combine.len();
        match d {
            0 => {
                app.st.combine.remove(k);
            }
            -1 if k > 0 => app.st.combine.swap(k, k - 1),
            1 if k + 1 < n => app.st.combine.swap(k, k + 1),
            _ => {}
        }
    }
    ui.add_space(10.0);
    if ui.add_enabled(app.st.combine.len() > 1, egui::Button::new(RichText::new("Combiner").color(egui::Color32::WHITE)).fill(BLUE)).clicked() {
        match load_many(&app.st.combine).and_then(|mut d| engine::save(&mut d)) {
            Ok(b) => {
                let r = Opened::new(b, None, "Combiné.pdf".into(), "");
                app.set_doc(r, ctx);
                app.st.combine.clear();
                app.panel = Panel::Organize;
                toast(ctx, "Fichiers combinés : pense à enregistrer", true);
            }
            Err(e) => toast(ctx, e, false),
        }
    }
}

pub fn create_from_images(app: &mut Folio, ctx: &egui::Context, size: Option<(f32, f32)>) {
    let Some(files) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp"]).pick_files() else { return };
    let imgs: Result<Vec<_>, String> = files.iter().map(|f| image::open(f).map(|i| i.to_rgba8()).map_err(|e| format!("{} : {e}", f.display()))).collect();
    match imgs.and_then(|i| engine::images_to_pdf(&i, size)).and_then(|mut d| engine::save(&mut d)) {
        Ok(b) => {
            app.set_doc(Opened::new(b, None, "Nouveau PDF.pdf".into(), ""), ctx);
            toast(ctx, format!("{} image(s) réunies en PDF", files.len()), true);
        }
        Err(e) => toast(ctx, e, false),
    }
}

pub fn blank(app: &mut Folio, ctx: &egui::Context) {
    let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
    let r = engine::images_to_pdf(&[img], Some((595.0, 842.0))).and_then(|mut d| {
        engine::insert_blank(&mut d, 0, 595.0, 842.0)?;
        engine::arrange(&mut d, &[0])?;
        engine::save(&mut d)
    });
    match r {
        Ok(b) => app.set_doc(Opened::new(b, None, "Sans titre.pdf".into(), ""), ctx),
        Err(e) => toast(ctx, e, false),
    }
}

fn create(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Créer un PDF");
    if pro::primary(ui, "Depuis des images (pages A4)").clicked() {
        create_from_images(app, ctx, Some((595.0, 842.0)));
    }
    ui.add_space(4.0);
    if pro::secondary(ui, "Depuis des images (taille d'origine)").clicked() {
        create_from_images(app, ctx, None);
    }
    ui.add_space(4.0);
    if pro::secondary(ui, "PDF vierge A4").clicked() {
        blank(app, ctx);
    }
    ui.add_space(8.0);
    hint(ui, "Chaque image devient une page. Lance ensuite l'OCR pour rendre le texte des scans consultable.");
}

pub fn send_page(app: &mut Folio, ctx: &egui::Context, target: &'static str) {
    let Some(o) = &app.od else { return };
    let Some(img) = app.render_page(app.page, 200.0 / 72.0) else { return };
    let path = studio_core::handoff::outgoing("folio", &format!("{} p{}", stem(&o.name), app.page + 1), "png");
    match img.save(&path) {
        Ok(()) => studio_ui::handoff::send(ctx, target, &path),
        Err(e) => toast(ctx, e.to_string(), false),
    }
}

pub fn start_ocr(app: &mut Folio, pages: Vec<usize>, layer: bool) {
    let Some(o) = &app.od else { return };
    let (tx, rx) = mpsc::channel();
    let bytes = o.bytes.clone();
    let sizes = o.sizes.clone();
    let lang = app.st.langs.get(app.st.ocr_lang).map(|l| l.0.clone()).unwrap_or_default();
    app.jobs = Some(rx);
    app.progress = Some((0, pages.len()));
    std::thread::spawn(move || {
        let Some(pdf) = render::open(bytes) else {
            let _ = tx.send(Job::Err("Rendu impossible".into()));
            return;
        };
        let mut out = Vec::new();
        for (k, &i) in pages.iter().enumerate() {
            let _ = tx.send(Job::Progress(k, pages.len()));
            let Some(img) = render::page(&pdf, i, 3.0) else { continue };
            let s = img.width() as f32 / sizes.get(i).map_or(1.0, |z| z.0.max(1.0));
            match ocr::recognize(&img, &lang) {
                Ok(p) => out.push((i, p.text, p.words.into_iter().map(|w| (w.text, w.rect.map(|v| v / s))).collect())),
                Err(e) => {
                    let _ = tx.send(Job::Err(e));
                    return;
                }
            }
        }
        let _ = tx.send(Job::Ocr(out, layer));
    });
}

pub fn ocr_done(app: &mut Folio, ctx: &egui::Context, pages: Vec<(usize, String, Vec<(String, [f32; 4])>)>, layer: bool) {
    let words: usize = pages.iter().map(|p| p.2.len()).sum();
    app.st.ocr_text = pages.iter().map(|(i, t, _)| format!("— Page {} —\n{}", i + 1, t.trim())).collect::<Vec<_>>().join("\n\n");
    if layer && words > 0 {
        app.edit(ctx, &format!("{words} mots reconnus : le PDF est maintenant consultable"), |d| pages.iter().try_for_each(|(i, _, w)| engine::add_text_layer(d, *i, w)));
    } else {
        toast(ctx, if words > 0 { format!("{words} mots reconnus") } else { "Aucun texte trouvé".into() }, words > 0);
    }
    app.panel = Panel::Ocr;
}

fn ocr_panel(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Numériser et OCR");
    hint(ui, "Lit le texte des pages scannées ou photographiées avec le moteur OCR de Windows, sans rien envoyer en ligne.");
    ui.add_space(6.0);
    label(ui, "Langue du document");
    if app.st.langs.is_empty() {
        hint(ui, "Aucune langue OCR installée : Paramètres Windows › Heure et langue › Langue.");
    } else {
        let cur = app.st.langs.get(app.st.ocr_lang).map(|l| l.1.clone()).unwrap_or_default();
        egui::ComboBox::from_id_salt("ocr-lang").selected_text(cur).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
            for (k, l) in app.st.langs.iter().enumerate() {
                ui.selectable_value(&mut app.st.ocr_lang, k, &l.1);
            }
        });
    }
    label(ui, "Pages");
    scope_pick(ui, &mut app.st.ocr_scope, !app.sel.is_empty());
    ui.checkbox(&mut app.st.ocr_layer, "Rendre le PDF consultable (calque de texte invisible)");
    ui.add_space(6.0);
    if ui.add_enabled(app.jobs.is_none(), egui::Button::new(RichText::new("Reconnaître le texte").color(Color32::WHITE)).fill(BLUE)).clicked() {
        let pages = app.scope(app.st.ocr_scope);
        start_ocr(app, pages, app.st.ocr_layer);
    }
    if !app.st.ocr_text.is_empty() {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("Copier le texte").clicked() {
                ctx.copy_text(app.st.ocr_text.clone());
                toast(ctx, "Texte copié", true);
            }
            if ui.button("Enregistrer en .txt…").clicked()
                && let Some(p) = rfd::FileDialog::new().add_filter("Texte", &["txt"]).set_file_name("texte reconnu.txt").save_file()
            {
                let _ = std::fs::write(p, &app.st.ocr_text);
            }
        });
        ui.add(egui::TextEdit::multiline(&mut app.st.ocr_text.as_str()).desired_width(f32::INFINITY).desired_rows(14));
    }
}

fn redact(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Caviarder");
    hint(ui, "Trace des rectangles avec l'outil Caviardage sur ce qui doit disparaître. Appliquer remplace ces pages par une image : le texte caché n'existe plus du tout dans le fichier.");
    ui.add_space(6.0);
    tool_buttons(app, ui, &[Tool::Redact]);
    label(ui, &format!("{} zone(s) à caviarder", app.redactions.len()));
    let mut del = None;
    for (k, (p, r)) in app.redactions.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Page {} · {:.0} × {:.0} mm", p + 1, r[2] * 25.4 / 72.0, r[3] * 25.4 / 72.0)).size(12.0).color(INK));
            if ui.small_button("✕").clicked() {
                del = Some(k);
            }
        });
    }
    if let Some(k) = del {
        app.redactions.remove(k);
    }
    ui.checkbox(&mut app.st.redact_ocr, "Relancer l'OCR sur ces pages (garde le reste consultable)");
    ui.add_space(6.0);
    if ui.add_enabled(!app.redactions.is_empty(), egui::Button::new(RichText::new("Appliquer le caviardage").color(Color32::WHITE)).fill(Color32::from_rgb(200, 30, 40))).clicked() {
        let mut pages: Vec<usize> = app.redactions.iter().map(|r| r.0).collect();
        pages.sort();
        pages.dedup();
        let s = 200.0 / 72.0;
        let mut imgs = Vec::new();
        for &p in &pages {
            let Some(mut img) = app.render_page(p, s) else { continue };
            let k = img.width() as f32 / app.od.as_ref().map_or(1.0, |o| o.sizes[p].0);
            for (_, r) in app.redactions.iter().filter(|x| x.0 == p) {
                let (x0, y0) = ((r[0] * k) as u32, (r[1] * k) as u32);
                let (x1, y1) = (((r[0] + r[2]) * k).ceil() as u32, ((r[1] + r[3]) * k).ceil() as u32);
                for y in y0..y1.min(img.height()) {
                    for x in x0..x1.min(img.width()) {
                        img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
                    }
                }
            }
            imgs.push((p, img));
        }
        if app.edit(ctx, "Caviardage appliqué", |d| imgs.iter().try_for_each(|(p, i)| engine::replace_with_image(d, *p, i, 90))) {
            app.redactions.clear();
            if app.st.redact_ocr {
                start_ocr(app, pages, true);
            }
        }
    }
    if !app.redactions.is_empty() && ui.button("Effacer les zones").clicked() {
        app.redactions.clear();
    }
}

fn protect(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Protéger");
    hint(ui, "Chiffrement AES-256. Une copie protégée est enregistrée, le document ouvert reste modifiable.");
    ui.add_space(6.0);
    label(ui, "Mot de passe d'ouverture");
    ui.add(egui::TextEdit::singleline(&mut app.st.pw).password(true).desired_width(f32::INFINITY));
    label(ui, "Mot de passe des autorisations (facultatif)");
    ui.add(egui::TextEdit::singleline(&mut app.st.pw_owner).password(true).desired_width(f32::INFINITY));
    ui.checkbox(&mut app.st.perm[0], "Autoriser l'impression");
    ui.checkbox(&mut app.st.perm[1], "Autoriser la copie du texte");
    ui.checkbox(&mut app.st.perm[2], "Autoriser les modifications");
    ui.add_space(6.0);
    if ui.add_enabled(!app.st.pw.is_empty(), egui::Button::new(RichText::new("Enregistrer une copie protégée…").color(Color32::WHITE)).fill(BLUE)).clicked()
        && let Some(o) = &app.od
        && let Some(p) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name(format!("{} - protégé.pdf", stem(&o.name))).save_file()
    {
        let mut d = o.doc.clone();
        let [pr, cp, md] = app.st.perm;
        match engine::encrypt(&mut d, &app.st.pw, &app.st.pw_owner, pr, cp, md).and_then(|_| engine::save(&mut d)).and_then(|b| std::fs::write(&p, b).map_err(|e| e.to_string())) {
            Ok(()) => toast(ctx, format!("Copie protégée : {}", p.display()), true),
            Err(e) => toast(ctx, e, false),
        }
    }
    ui.add_space(10.0);
    hint(ui, "Pour retirer un mot de passe : ouvre le PDF protégé, puis Enregistrer : la copie n'en a plus.");
}

fn size_text(n: usize) -> String {
    if n > 1 << 20 { format!("{:.1} Mo", n as f32 / 1048576.0) } else { format!("{:.0} Ko", n as f32 / 1024.0) }
}

fn compress(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Compresser");
    let before = app.od.as_ref().map_or(0, |o| o.bytes.len());
    label(ui, &format!("Taille actuelle : {}", size_text(before)));
    ui.add_space(6.0);
    if pro::secondary(ui, "Compression sans perte").clicked() && app.edit(ctx, "", |d| {
        engine::compress(d);
        Ok(())
    }) {
        let after = app.od.as_ref().map_or(0, |o| o.bytes.len());
        toast(ctx, format!("{} → {}", size_text(before), size_text(after)), true);
    }
    ui.add_space(10.0);
    label(ui, "Compression forte (pages en images)");
    pro::segmented(ui, &mut app.st.comp_dpi, &["72 ppp", "110 ppp", "150 ppp"]);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Qualité JPEG").size(12.0).color(MUTED));
        ui.add(egui::Slider::new(&mut app.st.comp_q, 30..=95));
    });
    hint(ui, "Idéal pour les scans lourds. Le texte n'est plus sélectionnable : relance l'OCR ensuite.");
    if pro::secondary(ui, "Compression forte").clicked() {
        let dpi = [72.0, 110.0, 150.0][app.st.comp_dpi];
        let n = app.od.as_ref().map_or(0, |o| o.n());
        let imgs: Vec<_> = (0..n).filter_map(|i| app.render_page(i, dpi / 72.0).map(|im| (i, im))).collect();
        let q = app.st.comp_q;
        if app.edit(ctx, "", |d| {
            imgs.iter().try_for_each(|(i, im)| engine::replace_with_image(d, *i, im, q))?;
            engine::compress(d);
            Ok(())
        }) {
            let after = app.od.as_ref().map_or(0, |o| o.bytes.len());
            toast(ctx, format!("{} → {}", size_text(before), size_text(after)), true);
        }
    }
}

fn stamp(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Filigrane et numérotation");
    let mut mode = app.st.st_mode;
    if pro::segmented(ui, &mut mode, &["Filigrane", "En-tête", "Pied", "Numéros", "Bates"]) {
        app.st.st_mode = mode;
        app.st.st = stamp_preset(mode);
    }
    ui.add_space(6.0);
    let s = &mut app.st.st;
    label(ui, if mode == 4 { "Préfixe" } else { "Texte · variables {page} {pages} {date} {fichier}" });
    ui.add(egui::TextEdit::singleline(&mut s.text).desired_width(f32::INFINITY));
    if mode == 4 {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Premier numéro").size(12.0).color(MUTED));
            ui.add(egui::DragValue::new(&mut app.st.bates.0).range(0..=9_999_999));
            ui.label(RichText::new("Chiffres").size(12.0).color(MUTED));
            ui.add(egui::DragValue::new(&mut app.st.bates.1).range(1..=10));
        });
    }
    pro::option_slider(ui, "Taille", &mut s.size, 5.0..=160.0, " pt", false);
    pro::option_slider(ui, "Opacité", &mut s.opacity, 0.05..=1.0, "", false);
    if mode == 0 {
        pro::option_slider(ui, "Angle", &mut s.angle, -90.0..=90.0, "°", false);
    }
    color_row(ui, &mut s.color);
    ui.checkbox(&mut s.bold, "Gras");
    ui.checkbox(&mut s.under, "Derrière le contenu");
    let mut place = s.place as usize;
    egui::ComboBox::from_id_salt("st-place").selected_text(PLACES[place.min(6)]).show_ui(ui, |ui| {
        for (k, p) in PLACES.iter().enumerate() {
            ui.selectable_value(&mut place, k, *p);
        }
    });
    s.place = place as u8;
    label(ui, "Pages");
    scope_pick(ui, &mut app.st.st_scope, !app.sel.is_empty());
    ui.add_space(6.0);
    if pro::primary(ui, "Appliquer").clicked() {
        let idx = app.scope(app.st.st_scope);
        let st = app.st.st.clone();
        let bates = (mode == 4).then_some(app.st.bates);
        let name = app.od.as_ref().map(|o| o.name.clone()).unwrap_or_default();
        app.edit(ctx, "Texte ajouté aux pages", |d| engine::stamp_text(d, &idx, &st, &name, bates));
    }
    ui.add_space(12.0);
    label(ui, "Arrière-plan");
    color_row(ui, &mut app.st.bg);
    if ui.button("Appliquer la couleur de fond").clicked() {
        let idx = app.scope(app.st.st_scope);
        let c = app.st.bg;
        app.edit(ctx, "Arrière-plan ajouté", |d| engine::background(d, &idx, c));
    }
}

fn export(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Exporter");
    pro::segmented(ui, &mut app.st.exp_fmt, &["PNG", "JPEG", "Texte", "Word"]);
    if app.st.exp_fmt < 2 {
        label(ui, "Résolution");
        pro::segmented(ui, &mut app.st.exp_dpi, &["72 ppp", "150 ppp", "300 ppp"]);
    }
    label(ui, "Pages");
    scope_pick(ui, &mut app.st.exp_scope, !app.sel.is_empty());
    ui.add_space(6.0);
    if pro::primary(ui, "Exporter…").clicked() {
        let idx = app.scope(app.st.exp_scope);
        let name = app.od.as_ref().map(|o| stem(&o.name)).unwrap_or_default();
        match app.st.exp_fmt {
            0 | 1 => {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    let dpi = [72.0, 150.0, 300.0][app.st.exp_dpi];
                    let mut ok = 0;
                    for &i in &idx {
                        if let Some(img) = app.render_page(i, dpi / 72.0) {
                            let r = if app.st.exp_fmt == 0 { img.save(dir.join(format!("{name} - page {}.png", i + 1))).map_err(|e| e.to_string()) } else { engine::jpeg(&img, 90).and_then(|b| std::fs::write(dir.join(format!("{name} - page {}.jpg", i + 1)), b).map_err(|e| e.to_string())) };
                            ok += usize::from(r.is_ok());
                        }
                    }
                    toast(ctx, format!("{ok} image(s) exportée(s) dans {}", dir.display()), ok > 0);
                }
            }
            k => {
                let texts: Vec<String> = match &mut app.od {
                    Some(o) => idx.iter().map(|i| o.text(*i).to_string()).collect(),
                    None => Vec::new(),
                };
                if texts.iter().all(|t| t.trim().is_empty()) {
                    toast(ctx, "Aucun texte dans ces pages : lance d'abord l'OCR", false);
                    return;
                }
                let (ext, filter, body) = if k == 2 { ("txt", "Texte", texts.join("\n\n\u{c}")) } else { ("rtf", "Document Word (RTF)", engine::to_rtf(&texts)) };
                if let Some(p) = rfd::FileDialog::new().add_filter(filter, &[ext]).set_file_name(format!("{name}.{ext}")).save_file() {
                    match std::fs::write(&p, body) {
                        Ok(()) => toast(ctx, format!("Exporté : {}", p.display()), true),
                        Err(e) => toast(ctx, e.to_string(), false),
                    }
                }
            }
        }
    }
    ui.add_space(6.0);
    if ui.button("Extraire les images du PDF…").on_hover_text("Enregistre en PNG chaque image contenue dans les pages choisies").clicked()
        && let Some(o) = &app.od
    {
        let idx = app.scope(app.st.exp_scope);
        let found = engine::extract_images(&o.doc, &idx);
        if found.is_empty() {
            toast(ctx, "Aucune image dans ces pages", false);
        } else if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            let name = stem(&o.name);
            let mut ok = 0;
            for (k, (page, img)) in found.iter().enumerate() {
                ok += usize::from(img.save(dir.join(format!("{name} - p{} - image {}.png", page + 1, k + 1))).is_ok());
            }
            toast(ctx, format!("{ok} image(s) extraite(s) dans {}", dir.display()), ok > 0);
        }
    }
    ui.add_space(10.0);
    label(ui, "Envoyer la page courante");
    ui.horizontal(|ui| {
        if ui.button("Vers Pixel").clicked() {
            send_page(app, ctx, "pixel");
        }
        if ui.button("Vers Lumen").clicked() {
            send_page(app, ctx, "lumen");
        }
    });
}

fn compare(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Comparer des fichiers");
    if ui.button("Choisir l'autre version…").clicked()
        && let Some(o) = &app.od
        && let Some(p) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file()
    {
        match std::fs::read(&p) {
            Ok(b) => {
                let other = Arc::new(b);
                app.st.cmp = Some(Cmp { name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), bytes: other.clone(), diffs: Vec::new() });
                app.st.diff_tex.clear();
                let (tx, rx) = mpsc::channel();
                let mine = o.bytes.clone();
                app.jobs = Some(rx);
                std::thread::spawn(move || {
                    let (Some(a), Some(b)) = (render::open(mine), render::open(other)) else {
                        let _ = tx.send(Job::Err("Lecture de l'autre PDF impossible".into()));
                        return;
                    };
                    let n = render::count(&a).max(render::count(&b));
                    let mut v = Vec::new();
                    for i in 0..n {
                        let _ = tx.send(Job::Progress(i, n));
                        v.push(match (render::page(&a, i, 0.7), render::page(&b, i, 0.7)) {
                            (Some(x), Some(y)) => diff_ratio(&x, &y),
                            _ => 1.0,
                        });
                    }
                    let _ = tx.send(Job::Compare(v));
                });
            }
            Err(e) => toast(ctx, e.to_string(), false),
        }
    }
    let Some(c) = &app.st.cmp else {
        hint(ui, "Compare le document ouvert à une autre version : chaque page reçoit un taux de différence, et les zones modifiées peuvent s'afficher en rouge.");
        return;
    };
    label(ui, &format!("Comparé à : {}", c.name));
    ui.checkbox(&mut app.st.cmp_show, "Afficher les différences sur les pages");
    let mut go = None;
    for (i, d) in c.diffs.iter().enumerate() {
        let t = if *d <= 0.0005 { "identique".to_string() } else if *d >= 1.0 { "absente d'un des fichiers".into() } else { format!("{:.1} % modifié", d * 100.0) };
        let col = if *d <= 0.0005 { FAINT } else { Color32::from_rgb(255, 120, 100) };
        if ui.add(egui::Label::new(RichText::new(format!("Page {} · {t}", i + 1)).size(12.0).color(col)).sense(Sense::click())).clicked() {
            go = Some(i);
        }
    }
    if let Some(i) = go {
        app.goto = Some(i);
    }
}

fn diff_ratio(a: &image::RgbaImage, b: &image::RgbaImage) -> f32 {
    if a.dimensions() != b.dimensions() {
        return 1.0;
    }
    let n = a.pixels().zip(b.pixels()).filter(|(p, q)| (0..3).map(|k| (p.0[k] as i32 - q.0[k] as i32).abs()).sum::<i32>() > 60).count();
    n as f32 / (a.width() * a.height()).max(1) as f32
}

pub fn diff_texture(app: &mut Folio, ctx: &egui::Context, i: usize) -> Option<TextureHandle> {
    if !app.st.cmp_show {
        return None;
    }
    let rev = app.od.as_ref()?.rev;
    if let Some(t) = app.st.diff_tex.get(&(i, rev)) {
        return Some(t.clone());
    }
    let c = app.st.cmp.as_ref()?;
    let other = render::open(c.bytes.clone())?;
    let a = app.render_page(i, 1.0)?;
    let b = render::page(&other, i, 1.0).filter(|b| b.dimensions() == a.dimensions());
    let img = image::RgbaImage::from_fn(a.width(), a.height(), |x, y| {
        let p = a.get_pixel(x, y).0;
        let differs = b.as_ref().is_none_or(|b| {
            let q = b.get_pixel(x, y).0;
            (0..3).map(|k| (p[k] as i32 - q[k] as i32).abs()).sum::<i32>() > 60
        });
        if differs { image::Rgba([255, 40, 40, 110]) } else { image::Rgba([0, 0, 0, 0]) }
    });
    let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
    let t = ctx.load_texture(format!("folio-diff{i}"), ci, TextureOptions::LINEAR);
    app.st.diff_tex.insert((i, rev), t.clone());
    Some(t)
}

fn search(app: &mut Folio, ui: &mut Ui) {
    title(ui, app, "Rechercher");
    let r = ui.add(egui::TextEdit::singleline(&mut app.st.search).hint_text("Mot ou phrase").desired_width(f32::INFINITY));
    r.request_focus();
    if r.changed() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
        let q = app.st.search.trim().to_lowercase();
        app.st.results.clear();
        if q.chars().count() >= 2
            && let Some(o) = &mut app.od
        {
            for i in 0..o.n() {
                let t = o.text(i).to_string();
                let low = t.to_lowercase();
                let hits = low.matches(&q).count();
                if let Some(pos) = low.find(&q) {
                    let chars: Vec<char> = t.chars().collect();
                    let cpos = low[..pos].chars().count();
                    let a = cpos.saturating_sub(30);
                    let b = (cpos + q.chars().count() + 40).min(chars.len());
                    let snip: String = chars[a..b].iter().collect::<String>().replace('\n', " ");
                    app.st.results.push((i, format!("{hits} × · …{snip}…")));
                }
            }
        }
    }
    ui.add_space(6.0);
    if app.st.search.trim().chars().count() >= 2 {
        label(ui, &format!("{} page(s) contiennent ce texte", app.st.results.len()));
    }
    let mut go = None;
    for (i, s) in &app.st.results {
        ui.add_space(4.0);
        let resp = ui.add(egui::Label::new(RichText::new(format!("Page {}", i + 1)).font(semibold(12.0)).color(INK)).sense(Sense::click()));
        ui.label(RichText::new(s).size(11.5).color(MUTED));
        if resp.clicked() {
            go = Some(*i);
        }
    }
    if let Some(i) = go {
        app.goto = Some(i);
    }
    hint(ui, "Un scan doit d'abord passer par Numériser et OCR.");
}

pub fn bookmarks_list(app: &mut Folio, ui: &mut Ui) {
    if app.st.bm.is_empty() {
        hint(ui, "Aucun signet. Outils › Signets pour en créer.");
        return;
    }
    let mut go = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (level, t, p) in &app.st.bm {
            ui.horizontal(|ui| {
                ui.add_space(level.saturating_sub(1) as f32 * 12.0);
                if ui.add(egui::Label::new(RichText::new(t).size(12.0).color(INK)).sense(Sense::click())).on_hover_text(format!("Page {}", p + 1)).clicked() {
                    go = Some(*p);
                }
            });
        }
    });
    if let Some(p) = go {
        app.goto = Some(p);
    }
}

fn bookmarks(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Signets");
    ui.add(egui::TextEdit::singleline(&mut app.st.bm_title).hint_text(format!("Titre du signet (page {})", app.page + 1)).desired_width(f32::INFINITY));
    let mut changed = false;
    ui.horizontal(|ui| {
        if ui.button("Ajouter pour la page courante").clicked() {
            let t = if app.st.bm_title.trim().is_empty() { format!("Page {}", app.page + 1) } else { app.st.bm_title.trim().to_string() };
            let at = app.st.bm.iter().position(|b| b.2 > app.page).unwrap_or(app.st.bm.len());
            app.st.bm.insert(at, (1, t, app.page));
            app.st.bm_title.clear();
            changed = true;
        }
        if ui.button("Un par page").clicked() {
            let n = app.od.as_ref().map_or(0, |o| o.n());
            app.st.bm = (0..n).map(|i| (1, format!("Page {}", i + 1), i)).collect();
            changed = true;
        }
    });
    ui.add_space(6.0);
    let mut del = None;
    for (k, (level, t, p)) in app.st.bm.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.add_space(level.saturating_sub(1) as f32 * 12.0);
            if ui.add(egui::TextEdit::singleline(t).desired_width(170.0)).lost_focus() {
                changed = true;
            }
            ui.label(RichText::new(format!("p. {}", *p + 1)).size(11.0).color(FAINT));
            if ui.small_button("→").on_hover_text("Niveau inférieur").clicked() {
                *level = (*level + 1).min(4);
                changed = true;
            }
            if ui.small_button("✕").clicked() {
                del = Some(k);
            }
        });
    }
    if let Some(k) = del {
        app.st.bm.remove(k);
        changed = true;
    }
    if changed {
        let mut items = app.st.bm.clone();
        if let Some(f) = items.first_mut() {
            f.0 = 1;
        }
        app.edit(ctx, "", |d| engine::set_outline(d, &items));
    }
}

fn props(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Propriétés");
    if let Some(o) = &app.od {
        pro::label_value(ui, "Pages", &o.n().to_string());
        pro::label_value(ui, "Taille", &size_text(o.bytes.len()));
        pro::label_value(ui, "Version PDF", &o.doc.version);
        if let Some((w, h)) = o.sizes.get(app.page) {
            pro::label_value(ui, "Format (page courante)", &format!("{:.0} × {:.0} mm", w * 25.4 / 72.0, h * 25.4 / 72.0));
        }
        if let Some(p) = &o.path {
            pro::label_value(ui, "Emplacement", &p.parent().map(|d| d.display().to_string()).unwrap_or_default());
        }
    }
    ui.add_space(8.0);
    for ((_, name), v) in engine::META.iter().zip(app.st.meta.iter_mut()) {
        label(ui, name);
        ui.add(egui::TextEdit::singleline(v).desired_width(f32::INFINITY));
    }
    ui.add_space(6.0);
    if pro::primary(ui, "Appliquer").clicked() {
        let m = app.st.meta.clone();
        app.edit(ctx, "Propriétés mises à jour", |d| {
            engine::set_info(d, &m);
            Ok(())
        });
    }
}

fn measure(app: &mut Folio, ui: &mut Ui, ctx: &egui::Context) {
    title(ui, app, "Mesurer");
    tool_buttons(app, ui, &[Tool::Measure]);
    hint(ui, "Glisse entre deux points de la page : la distance s'affiche en millimètres (à l'échelle 1:1 du document).");
    if let Some((p, a, b)) = app.measure {
        let d = (a[0] - b[0]).hypot(a[1] - b[1]) * 25.4 / 72.0;
        ui.add_space(6.0);
        ui.label(RichText::new(format!("{d:.1} mm")).font(semibold(22.0)).color(INK_STRONG));
        ui.label(RichText::new(format!("{:.2} cm · {:.2} pouces · {:.1} pt", d / 10.0, d / 25.4, d * 72.0 / 25.4)).size(12.0).color(MUTED));
        if ui.button("Ajouter comme annotation").clicked() {
            let m = engine::Mark { kind: engine::MarkKind::Arrow, pts: vec![a, b], strokes: Vec::new(), color: [10, 132, 255], width: 1.0, text: format!("{d:.1} mm"), size: 9.0 };
            app.edit(ctx, "Mesure ajoutée", |doc| engine::add_mark(doc, p, &m));
        }
    }
}

pub fn panel_glyph(p: &egui::Painter, r: Rect, k: Panel, c: Color32) {
    use egui::Shape;
    let s = Stroke::new(1.6, c);
    let at = |x: f32, y: f32| pos2(r.left() + x * r.width(), r.top() + y * r.height());
    let doc = |p: &egui::Painter| {
        p.add(Shape::closed_line(vec![at(0.18, 0.0), at(0.62, 0.0), at(0.85, 0.24), at(0.85, 1.0), at(0.18, 1.0)], s));
    };
    let lines = |p: &egui::Painter, ys: &[f32]| {
        for y in ys {
            p.line_segment([at(0.32, *y), at(0.72, *y)], Stroke::new(1.3, c));
        }
    };
    match k {
        Panel::All => doc(p),
        Panel::Comment => crate::glyph(p, r, Tool::Note, c),
        Panel::FillSign => {
            let pts: Vec<egui::Pos2> = (0..=30).map(|i| {
                let t = i as f32 / 30.0;
                at(0.05 + t * 0.9, 0.45 - (t * 12.0).sin() * 0.22)
            }).collect();
            p.add(Shape::line(pts, s));
            p.line_segment([at(0.0, 0.88), at(1.0, 0.88)], s);
        }
        Panel::Organize => {
            for (x, y) in [(0.0, 0.0), (0.55, 0.0), (0.0, 0.55), (0.55, 0.55)] {
                p.rect_stroke(Rect::from_min_size(at(x, y), r.size() * 0.42), CornerRadius::same(2), s, StrokeKind::Middle);
            }
        }
        Panel::Combine => {
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.0), at(0.5, 0.62)), CornerRadius::same(2), s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(at(0.5, 0.38), at(1.0, 1.0)), CornerRadius::same(2), s, StrokeKind::Middle);
            p.line_segment([at(0.3, 0.75), at(0.42, 0.75)], s);
            p.line_segment([at(0.3, 0.75), at(0.3, 0.62)], s);
        }
        Panel::Create => {
            doc(p);
            p.line_segment([at(0.52, 0.38), at(0.52, 0.8)], s);
            p.line_segment([at(0.31, 0.59), at(0.73, 0.59)], s);
        }
        Panel::Ocr => {
            for (x, y) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                let (dx, dy) = (if x > 0.5 { -0.28 } else { 0.28 }, if y > 0.5 { -0.28 } else { 0.28 });
                p.line_segment([at(x, y), at(x + dx, y)], s);
                p.line_segment([at(x, y), at(x, y + dy)], s);
            }
            p.text(at(0.5, 0.52), egui::Align2::CENTER_CENTER, "Aa", egui::FontId::proportional(r.height() * 0.5), c);
        }
        Panel::Redact => {
            lines(p, &[0.15, 0.85]);
            p.rect_filled(Rect::from_min_max(at(0.1, 0.36), at(0.9, 0.64)), CornerRadius::same(1), c);
        }
        Panel::Protect => {
            p.rect_stroke(Rect::from_min_max(at(0.12, 0.45), at(0.88, 1.0)), CornerRadius::same(3), s, StrokeKind::Middle);
            let arc: Vec<egui::Pos2> = (0..=16).map(|i| {
                let a = std::f32::consts::PI * (1.0 + i as f32 / 16.0);
                at(0.5 + a.cos() * 0.24, 0.45 + a.sin() * 0.32)
            }).collect();
            p.add(Shape::line(arc, s));
            p.circle_filled(at(0.5, 0.7), 2.0, c);
        }
        Panel::Compress => {
            for (y, d) in [(0.0, 1.0), (1.0, -1.0)] {
                p.line_segment([at(0.5, y), at(0.5, y + 0.38 * d)], s);
                p.line_segment([at(0.32, y + 0.2 * d), at(0.5, y + 0.38 * d)], s);
                p.line_segment([at(0.68, y + 0.2 * d), at(0.5, y + 0.38 * d)], s);
            }
            p.line_segment([at(0.05, 0.5), at(0.95, 0.5)], s);
        }
        Panel::Stamp => {
            doc(p);
            p.text(at(0.52, 0.6), egui::Align2::CENTER_CENTER, "#", egui::FontId::proportional(r.height() * 0.5), c);
        }
        Panel::Export => {
            p.rect_stroke(Rect::from_min_max(at(0.05, 0.4), at(0.95, 1.0)), CornerRadius::same(2), s, StrokeKind::Middle);
            p.line_segment([at(0.5, 0.0), at(0.5, 0.65)], s);
            p.line_segment([at(0.32, 0.18), at(0.5, 0.0)], s);
            p.line_segment([at(0.68, 0.18), at(0.5, 0.0)], s);
        }
        Panel::Compare => {
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.1), at(0.42, 0.9)), CornerRadius::same(2), s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(at(0.58, 0.1), at(1.0, 0.9)), CornerRadius::same(2), s, StrokeKind::Middle);
            p.line_segment([at(0.5, 0.0), at(0.5, 1.0)], Stroke::new(1.0, c));
        }
        Panel::Search => {
            p.circle_stroke(at(0.42, 0.42), r.width() * 0.32, s);
            p.line_segment([at(0.66, 0.66), at(0.98, 0.98)], Stroke::new(2.2, c));
        }
        Panel::Bookmarks => {
            p.add(Shape::closed_line(vec![at(0.22, 0.0), at(0.78, 0.0), at(0.78, 1.0), at(0.5, 0.72), at(0.22, 1.0)], s));
        }
        Panel::Props => {
            p.circle_stroke(r.center(), r.width() * 0.47, s);
            p.circle_filled(at(0.5, 0.28), 1.8, c);
            p.line_segment([at(0.5, 0.45), at(0.5, 0.78)], Stroke::new(2.0, c));
        }
        Panel::Measure => crate::glyph(p, r, Tool::Measure, c),
    }
}
