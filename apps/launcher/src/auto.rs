use crate::P;
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Key, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, pos2, vec2};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use studio_auto::plan::{Kind, kind_of, plan};
use studio_auto::run::{Msg, Outcome, run};
use studio_ui::semibold;

const VIOLET: Color32 = Color32::from_rgb(0x8b, 0x6c, 0xff);

#[derive(Default)]
pub struct Auto {
    pub prompt: String,
    pub file: Option<PathBuf>,
    thumb: Option<(PathBuf, Option<TextureHandle>)>,
    pub open: bool,
    running: Option<Receiver<Msg>>,
    log: Vec<String>,
    result: Option<Result<Outcome, String>>,
    result_tex: Option<TextureHandle>,
    started: Option<std::time::Instant>,
}

fn suggestions(k: Kind) -> &'static [&'static str] {
    match k {
        Kind::Image | Kind::Raw => &["Retire les oiseaux", "Supprime le fond", "Étends l'image en format story", "Efface le texte", "Traduis le texte en anglais", "Noir et blanc et plus lumineuse", "Couleurs vives et plus nette", "Recadre en carré", "Rends-la vintage", "Anime-la en GIF", "Lis le texte"],
        Kind::Video => &["Extrais le son", "Monte cette vidéo"],
        Kind::Audio => &["Nettoie ce podcast", "Supprime les silences", "Rends-la bouclable", "Plus fort avec des fondus", "Ajoute de la réverbération", "Accélère un peu"],
        Kind::Pdf => &["Résume ce document", "Fais l'OCR", "Je veux le signer", "Compresse-le", "Protège-le par mot de passe", "Ajoute un filigrane"],
        Kind::Nothing => &["Un logo « Nova Café »", "Une intro animée « Bienvenue »", "Je veux faire une affiche", "Un bip de 3 secondes", "Monter une vidéo"],
        _ => &[],
    }
}

fn sparkle(p: &egui::Painter, c: egui::Pos2, r: f32, col: Color32) {
    let pts: Vec<egui::Pos2> = (0..8).map(|i| {
        let a = i as f32 / 8.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let rr = if i % 2 == 0 { r } else { r * 0.3 };
        c + vec2(a.cos(), a.sin()) * rr
    }).collect();
    p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
}

fn preview_image(f: &Path) -> Option<image::DynamicImage> {
    use image::AnimationDecoder;
    if f.extension().is_some_and(|e| e.eq_ignore_ascii_case("gif")) {
        let dec = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(std::fs::File::open(f).ok()?)).ok()?;
        let last = dec.into_frames().filter_map(Result::ok).last()?;
        return Some(image::DynamicImage::ImageRgba8(last.into_buffer()));
    }
    image::open(f).ok()
}

impl Auto {
    pub fn new() -> Self {
        Self { open: true, ..Default::default() }
    }

    pub fn set_file(&mut self, f: PathBuf) {
        self.file = Some(f);
        self.result = None;
        self.result_tex = None;
    }

    fn start(&mut self, ctx: &egui::Context) {
        let prompt = self.prompt.trim().to_string();
        let file = self.file.clone();
        let open = self.open;
        let (tx, rx) = channel();
        self.running = Some(rx);
        self.log.clear();
        self.result = None;
        self.result_tex = None;
        self.started = Some(std::time::Instant::now());
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let r = plan(&prompt, file.as_deref()).and_then(|p| run(&p, file.as_deref(), open, &tx));
            let _ = tx.send(Msg::Done(r));
            ctx.request_repaint();
        });
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.running else { return };
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
        let msgs: Vec<Msg> = rx.try_iter().collect();
        for m in msgs {
            match m {
                Msg::Step(s) => self.log.push(s),
                Msg::Done(r) => {
                    self.running = None;
                    if let Ok(o) = &r
                        && let Some(f) = &o.file
                        && let Some(img) = preview_image(f)
                    {
                        let t = img.thumbnail(900, 600).to_rgba8();
                        let ci = egui::ColorImage::from_rgba_unmultiplied([t.width() as usize, t.height() as usize], t.as_raw());
                        self.result_tex = Some(ctx.load_texture("auto-result", ci, TextureOptions::LINEAR));
                    }
                    self.result = Some(r);
                }
            }
        }
    }

    fn thumb(&mut self, ctx: &egui::Context) -> Option<TextureHandle> {
        let f = self.file.clone()?;
        if self.thumb.as_ref().is_none_or(|t| t.0 != f) {
            let tex = matches!(kind_of(Some(&f)), Kind::Image).then(|| image::open(&f).ok()).flatten().map(|img| {
                let t = img.thumbnail(320, 220).to_rgba8();
                ctx.load_texture("auto-thumb", egui::ColorImage::from_rgba_unmultiplied([t.width() as usize, t.height() as usize], t.as_raw()), TextureOptions::LINEAR)
            });
            self.thumb = Some((f, tex));
        }
        self.thumb.as_ref().and_then(|t| t.1.clone())
    }

    pub fn show(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        self.poll(ctx);
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let w = ui.available_width().min(860.0);
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(11), VIOLET.gamma_multiply(0.25));
                sparkle(ui.painter(), r.center() + vec2(-3.0, 3.0), 12.0, VIOLET);
                sparkle(ui.painter(), r.center() + vec2(9.0, -9.0), 5.0, VIOLET);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Assistant").font(semibold(26.0)).color(P.ink_strong));
                    ui.label(RichText::new("Dépose une image, une vidéo, un son ou un PDF, dis ce que tu veux : Ethone choisit l'app et les outils.").size(13.0).color(P.muted));
                });
            });
            ui.add_space(18.0);
            let thumb = self.thumb(ctx);
            let (drop, resp) = ui.allocate_exact_size(vec2(w, 120.0), Sense::click());
            let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
            ui.painter().rect(drop, CornerRadius::same(14), if hovering || resp.hovered() { P.tile_hover } else { P.tile }, Stroke::new(1.0, if hovering { VIOLET } else { P.line }), StrokeKind::Inside);
            match &self.file {
                Some(f) => {
                    let mut x = drop.left() + 18.0;
                    if let Some(t) = &thumb {
                        let s = t.size_vec2();
                        let k = (84.0 / s.y).min(130.0 / s.x);
                        let r = Rect::from_min_size(pos2(x, drop.center().y - s.y * k / 2.0), s * k);
                        ui.painter().image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                        x = r.right() + 16.0;
                    }
                    let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    ui.painter().text(pos2(x, drop.center().y - 10.0), Align2::LEFT_CENTER, name, semibold(15.0), P.ink_strong);
                    let kind = match kind_of(Some(f)) {
                        Kind::Image => "Image",
                        Kind::Raw => "Photo RAW",
                        Kind::Video => "Vidéo",
                        Kind::Audio => "Son",
                        Kind::Pdf => "PDF",
                        _ => "Fichier",
                    };
                    ui.painter().text(pos2(x, drop.center().y + 12.0), Align2::LEFT_CENTER, format!("{kind} · clique pour en choisir un autre"), FontId::proportional(12.0), P.muted);
                }
                None => {
                    ui.painter().text(drop.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "Glisse un fichier ici, ou clique pour le choisir", semibold(14.0), P.ink);
                    ui.painter().text(drop.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "Facultatif : sans fichier, l'assistant crée (logo, titre animé, affiche…)", FontId::proportional(12.0), P.faint);
                }
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                && let Some(f) = rfd::FileDialog::new().add_filter("Images, vidéos, sons, PDF", &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "cr2", "cr3", "nef", "arw", "dng", "raf", "mp4", "mov", "mkv", "webm", "wav", "mp3", "flac", "ogg", "m4a", "pdf", "svg", "evec", "epage", "emot", "ecut"]).pick_file()
            {
                self.set_file(f);
            }
            if self.file.is_some() {
                ui.add_space(4.0);
                if ui.add(egui::Button::new(RichText::new("Retirer le fichier").size(12.0).color(P.muted)).frame(false)).clicked() {
                    self.file = None;
                }
            }
            ui.add_space(14.0);
            ui.label(RichText::new("Que veux-tu faire ?").font(semibold(14.0)).color(P.ink_strong));
            ui.add_space(6.0);
            let edit = ui.add(egui::TextEdit::multiline(&mut self.prompt).hint_text("Ex. : retire les oiseaux du ciel et rends la photo plus lumineuse").desired_rows(2).desired_width(w).font(FontId::proportional(15.0)));
            let go_key = edit.has_focus() && ui.input(|i| i.key_pressed(Key::Enter) && i.modifiers.command);
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(w);
                for s in suggestions(kind_of(self.file.as_deref())) {
                    let b = egui::Button::new(RichText::new(*s).size(12.5).color(P.ink)).fill(P.field).corner_radius(CornerRadius::same(14));
                    if ui.add(b).clicked() {
                        self.prompt = s.to_string();
                    }
                }
            });
            ui.add_space(14.0);
            let preview = (!self.prompt.trim().is_empty()).then(|| plan(&self.prompt, self.file.as_deref()));
            egui::Frame::new().fill(P.tile).corner_radius(CornerRadius::same(14)).stroke(Stroke::new(1.0, P.line)).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                ui.set_width(w - 32.0);
                match &preview {
                    None => {
                        ui.label(RichText::new("Le plan s'affichera ici dès que tu écris ta demande.").size(12.5).color(P.faint));
                    }
                    Some(Err(e)) => {
                        ui.label(RichText::new(e).size(13.0).color(Color32::from_rgb(255, 160, 90)));
                    }
                    Some(Ok(p)) => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Ce que je vais faire").font(semibold(13.0)).color(P.ink_strong));
                            ui.label(RichText::new(format!("→ {}", studio_core::handoff::name(p.app))).font(semibold(13.0)).color(VIOLET));
                        });
                        ui.add_space(4.0);
                        for (k, s) in p.summary.iter().enumerate() {
                            ui.label(RichText::new(format!("{}. {s}", k + 1)).size(13.0).color(P.ink));
                        }
                    }
                }
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let can = matches!(preview, Some(Ok(_))) && self.running.is_none();
                let b = egui::Button::new(RichText::new("  Lancer  ").font(semibold(14.0)).color(Color32::WHITE)).fill(if can { VIOLET } else { P.field }).corner_radius(CornerRadius::same(18)).min_size(vec2(120.0, 36.0));
                if (ui.add_enabled(can, b).on_hover_text("Ctrl+Entrée").clicked() || (go_key && can)) && can {
                    self.start(ctx);
                }
                ui.checkbox(&mut self.open, "Ouvrir le résultat dans l'app");
                if self.running.is_some() {
                    ui.add(egui::Spinner::new().color(VIOLET));
                    if let Some(t) = self.started {
                        ui.label(RichText::new(format!("{} s", t.elapsed().as_secs())).size(12.0).color(P.muted));
                    }
                }
            });
            if !self.log.is_empty() || self.result.is_some() {
                ui.add_space(14.0);
                for l in &self.log {
                    ui.label(RichText::new(format!("• {l}")).size(12.5).color(P.muted));
                }
            }
            match &self.result {
                Some(Ok(o)) => {
                    ui.add_space(10.0);
                    ui.label(RichText::new(if o.opened.is_empty() { "Terminé".to_string() } else { format!("Terminé · {}", o.opened) }).font(semibold(15.0)).color(P.green));
                    for n in &o.notes {
                        ui.label(RichText::new(n).size(12.5).color(P.ink));
                    }
                    if let Some(t) = &self.result_tex {
                        ui.add_space(8.0);
                        let s = t.size_vec2();
                        let k = (w / s.x).min(420.0 / s.y).min(1.0);
                        ui.add(egui::Image::new((t.id(), s * k)).corner_radius(CornerRadius::same(10)));
                    }
                    if let Some(f) = o.file.clone() {
                        ui.horizontal(|ui| {
                            if ui.button("Ouvrir le dossier").clicked()
                                && let Some(d) = f.parent()
                            {
                                let _ = std::process::Command::new("explorer").arg(d).spawn();
                            }
                            if !matches!(kind_of(Some(&f)), Kind::Other | Kind::Nothing) && ui.button("Continuer avec ce résultat").on_hover_text("Le résultat devient le fichier de départ").clicked() {
                                self.set_file(f);
                                self.prompt.clear();
                            }
                        });
                    }
                }
                Some(Err(e)) => {
                    ui.add_space(10.0);
                    ui.label(RichText::new(e).size(13.0).color(P.red));
                }
                None => {}
            }
        });
    }
}
