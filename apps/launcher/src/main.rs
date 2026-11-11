#![windows_subsystem = "windows"]

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Frame, Label, Layout, Margin, RichText, Sense,
    Stroke, TextureHandle, Vec2, vec2,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use studio_core::{AppEntry, Catalog, Installed};
use studio_ui::{BG, BRAND, CARD, CARD_HOVER, DANGER, MUTED, OK, PANEL, TEXT};

enum Job {
    Running(u64, u64),
    Failed(String),
    Done,
}

enum Source {
    Loading,
    Online,
    Offline(String),
}

#[derive(PartialEq, Clone, Copy)]
enum Filter {
    All,
    Installed,
    Category(&'static str),
}

const CATEGORIES: [&str; 5] = ["Image", "Vidéo", "Photo", "Publication", "Audio"];
const CARD_H: f32 = 176.0;

struct Launcher {
    catalog: Arc<Mutex<(Catalog, Source)>>,
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    installed: Installed,
    filter: Filter,
    confirm_uninstall: Option<String>,
    notice: Option<String>,
    logo: TextureHandle,
}

impl Launcher {
    fn new(cc: &eframe::CreationContext) -> Self {
        studio_ui::apply_style(&cc.egui_ctx);
        let me = Self {
            catalog: Arc::new(Mutex::new((Catalog::bundled(), Source::Loading))),
            jobs: Arc::default(),
            installed: Installed::load(),
            filter: Filter::All,
            confirm_uninstall: None,
            notice: None,
            logo: studio_ui::icon_texture(&cc.egui_ctx, "studio"),
        };
        me.refresh(&cc.egui_ctx);
        me
    }

    fn refresh(&self, ctx: &egui::Context) {
        let catalog = self.catalog.clone();
        let ctx = ctx.clone();
        catalog.lock().unwrap().1 = Source::Loading;
        std::thread::spawn(move || {
            let result = Catalog::fetch();
            let mut c = catalog.lock().unwrap();
            match result {
                Ok(fetched) => *c = (fetched, Source::Online),
                Err(e) => c.1 = Source::Offline(e),
            }
            ctx.request_repaint();
        });
    }

    fn install(&self, ctx: &egui::Context, app: AppEntry) {
        let jobs = self.jobs.clone();
        let ctx = ctx.clone();
        jobs.lock().unwrap().insert(app.id.clone(), Job::Running(0, 0));
        std::thread::spawn(move || {
            let progress = |done, total| {
                jobs.lock()
                    .unwrap()
                    .insert(app.id.clone(), Job::Running(done, total));
                ctx.request_repaint();
            };
            let result = studio_core::install(&app, progress);
            jobs.lock().unwrap().insert(
                app.id.clone(),
                match result {
                    Ok(()) => Job::Done,
                    Err(e) => Job::Failed(e),
                },
            );
            ctx.request_repaint();
        });
    }

    fn collect_finished(&mut self) {
        let mut jobs = self.jobs.lock().unwrap();
        let before = jobs.len();
        jobs.retain(|_, j| !matches!(j, Job::Done));
        if jobs.len() != before {
            self.installed = Installed::load();
        }
    }
}

impl eframe::App for Launcher {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        self.collect_finished();
        let (apps, source_label, online) = {
            let c = self.catalog.lock().unwrap();
            let (label, online) = match &c.1 {
                Source::Loading => ("Connexion au catalogue…".to_string(), None),
                Source::Online => ("Catalogue à jour".to_string(), Some(true)),
                Source::Offline(e) => (format!("Hors ligne · {e}"), Some(false)),
            };
            (c.0.apps.clone(), label, online)
        };

        egui::Panel::top("top")
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(20, 12)))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    studio_ui::logo_image(ui, &self.logo, 30.0);
                    ui.add_space(4.0);
                    ui.label(RichText::new("Ethone Studio").size(18.0).strong().color(TEXT));
                    ui.label(
                        RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .size(11.0)
                            .color(MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Actualiser").clicked() {
                            self.refresh(ctx);
                        }
                        let pending: Vec<AppEntry> = apps
                            .iter()
                            .filter(|a| a.published() && !self.installed.apps.contains_key(&a.id))
                            .cloned()
                            .collect();
                        if ui
                            .add_enabled(!pending.is_empty(), egui::Button::new("Tout installer"))
                            .clicked()
                        {
                            for app in pending {
                                self.install(ctx, app);
                            }
                        }
                        ui.add_space(8.0);
                        let color = match online {
                            Some(true) => OK,
                            Some(false) => DANGER,
                            None => MUTED,
                        };
                        ui.label(RichText::new(source_label).color(color).size(12.0));
                        let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                        ui.painter().circle_filled(dot.center(), 4.0, color);
                    });
                });
            });

        egui::Panel::left("nav")
            .resizable(false)
            .exact_size(210.0)
            .frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 18)))
            .show(root, |ui| {
                section(ui, "BIBLIOTHÈQUE");
                let total = apps.len();
                let inst = self.installed.apps.len();
                nav(ui, &mut self.filter, Filter::All, "Toutes les apps", total);
                nav(ui, &mut self.filter, Filter::Installed, "Installées", inst);
                ui.add_space(18.0);
                section(ui, "CATÉGORIES");
                for cat in CATEGORIES {
                    let n = apps.iter().filter(|a| a.category == cat).count();
                    nav(ui, &mut self.filter, Filter::Category(cat), cat, n);
                }
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG).inner_margin(Margin::same(28)))
            .show(root, |ui| {
                let title = match self.filter {
                    Filter::All => "Toutes les apps",
                    Filter::Installed => "Installées",
                    Filter::Category(c) => c,
                };
                ui.label(RichText::new(title).size(24.0).strong().color(TEXT));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Installe uniquement les apps dont tu as besoin.").color(MUTED),
                );
                ui.add_space(20.0);

                if let Some(n) = self.notice.clone() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&n).color(DANGER));
                        if ui.small_button("×").clicked() {
                            self.notice = None;
                        }
                    });
                    ui.add_space(8.0);
                }

                let visible: Vec<&AppEntry> = apps
                    .iter()
                    .filter(|a| match self.filter {
                        Filter::All => true,
                        Filter::Installed => self.installed.apps.contains_key(&a.id),
                        Filter::Category(c) => a.category == c,
                    })
                    .collect();

                if visible.is_empty() {
                    ui.add_space(60.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Aucune application ici pour l'instant.").color(MUTED));
                    });
                    return;
                }

                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    let gap = 16.0;
                    let avail = ui.available_width();
                    let cols = ((avail + gap) / (300.0 + gap)).floor().max(1.0) as usize;
                    let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
                    for row in visible.chunks(cols) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for app in row {
                                ui.allocate_ui_with_layout(
                                    vec2(card_w, CARD_H),
                                    Layout::top_down(Align::Min),
                                    |ui| self.card(ui, ctx, app, card_w),
                                );
                            }
                        });
                        ui.add_space(gap);
                    }
                });
            });
    }
}

impl Launcher {
    fn card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, app: &AppEntry, width: f32) {
        let accent = Color32::from_rgb(app.accent[0], app.accent[1], app.accent[2]);
        let hover_id = ui.scope_id().with(("hover", &app.id));
        let hovered = ui.ctx().data(|d| d.get_temp::<bool>(hover_id).unwrap_or(false));
        let pad = 18.0;
        let resp = Frame::new()
            .fill(if hovered { CARD_HOVER } else { CARD })
            .corner_radius(CornerRadius::same(16))
            .stroke(Stroke::new(
                1.0,
                if hovered { accent.gamma_multiply(0.5) } else { Color32::from_white_alpha(10) },
            ))
            .inner_margin(Margin::same(pad as i8))
            .show(ui, |ui| {
                let inner = vec2(width - pad * 2.0 - 2.0, CARD_H - pad * 2.0 - 2.0);
                ui.set_min_size(inner);
                ui.set_max_size(inner);
                ui.horizontal(|ui| {
                    studio_ui::app_icon(ui, 52.0, &app.id, accent);
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.add_space(4.0);
                        ui.label(RichText::new(&app.name).size(18.0).strong().color(TEXT));
                        let meta = match &app.version {
                            Some(v) => format!("{} · v{v}", app.category),
                            None => app.category.clone(),
                        };
                        ui.label(RichText::new(meta).size(12.0).color(accent.gamma_multiply(0.85)));
                    });
                });
                ui.add_space(10.0);
                ui.add(Label::new(RichText::new(&app.tagline).color(MUTED)).wrap());
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    self.actions(ui, ctx, app, accent);
                });
            })
            .response;
        let now = resp.contains_pointer();
        if now != hovered {
            ui.ctx().data_mut(|d| d.insert_temp(hover_id, now));
            ui.ctx().request_repaint();
        }
    }

    fn actions(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, app: &AppEntry, accent: Color32) {
        let job = self.jobs.lock().unwrap().get(&app.id).map(|j| match j {
            Job::Running(d, t) => Ok((*d, *t)),
            Job::Failed(e) => Err(e.clone()),
            Job::Done => Ok((1, 1)),
        });

        if let Some(Ok((done, total))) = job {
            let frac = if total > 0 { done as f32 / total as f32 } else { 0.0 };
            ui.add(
                egui::ProgressBar::new(frac)
                    .fill(accent)
                    .text(format!("{:.1} / {:.1} Mo", mb(done), mb(total))),
            );
            return;
        }

        if let Some(Err(e)) = &job {
            ui.label(RichText::new(e).size(12.0).color(DANGER));
        }

        let installed = self.installed.apps.get(&app.id).cloned();
        ui.horizontal(|ui| {
            if !app.published() {
                ui.label(RichText::new("Bientôt disponible").size(12.0).color(MUTED));
                return;
            }
            match installed {
                Some(v) => {
                    if primary(ui, "Ouvrir", accent).clicked() {
                        if let Err(e) = studio_core::launch(&app.id) {
                            self.notice = Some(format!("{} : {e}", app.name));
                        }
                    }
                    if app.version.as_deref() != Some(v.as_str())
                        && ui.button("Mettre à jour").clicked()
                    {
                        self.install(ctx, app.clone());
                    }
                    let confirming = self.confirm_uninstall.as_deref() == Some(app.id.as_str());
                    let label = if confirming { "Confirmer ?" } else { "Désinstaller" };
                    if ui
                        .button(RichText::new(label).color(if confirming { DANGER } else { MUTED }))
                        .clicked()
                    {
                        if confirming {
                            if let Err(e) = studio_core::uninstall(&app.id) {
                                self.notice = Some(format!("{} : {e}", app.name));
                            }
                            self.installed = Installed::load();
                            self.confirm_uninstall = None;
                        } else {
                            self.confirm_uninstall = Some(app.id.clone());
                        }
                    }
                }
                None => {
                    let label = match app.size {
                        Some(s) => format!("Installer · {:.0} Mo", mb(s)),
                        None => "Installer".into(),
                    };
                    if primary(ui, &label, accent).clicked() {
                        self.install(ctx, app.clone());
                    }
                }
            }
        });
    }
}

fn mb(b: u64) -> f64 {
    b as f64 / 1_048_576.0
}

fn primary(ui: &mut egui::Ui, label: &str, accent: Color32) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).strong().color(Color32::from_rgb(14, 15, 20)))
            .fill(accent)
            .corner_radius(CornerRadius::same(8))
            .min_size(Vec2::new(96.0, 32.0)),
    )
}

fn section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(label).size(11.0).color(MUTED.gamma_multiply(0.8)));
    ui.add_space(4.0);
}

fn nav(ui: &mut egui::Ui, current: &mut Filter, value: Filter, label: &str, count: usize) {
    let selected = *current == value;
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, CornerRadius::same(8), CARD_HOVER);
        p.rect_filled(
            egui::Rect::from_min_size(rect.min + vec2(0.0, 9.0), vec2(3.0, 16.0)),
            CornerRadius::same(2),
            BRAND,
        );
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(8), CARD);
    }
    let color = if selected || resp.hovered() { TEXT } else { MUTED };
    p.text(
        rect.left_center() + vec2(14.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        color,
    );
    p.text(
        rect.right_center() - vec2(12.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        count.to_string(),
        FontId::proportional(12.0),
        MUTED,
    );
    if resp.clicked() {
        *current = value;
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Ethone Studio")
            .with_icon(Arc::new(studio_ui::app_window_icon("studio")))
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([760.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Ethone Studio",
        options,
        Box::new(|cc| Ok(Box::new(Launcher::new(cc)))),
    )
}
