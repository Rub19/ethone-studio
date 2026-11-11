#![windows_subsystem = "windows"]

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Pos2, Rect, RichText,
    Sense, Shadow, Stroke, StrokeKind, TextureHandle, Ui, Vec2, pos2, vec2,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use studio_core::{AppEntry, Catalog, Installed};
use studio_ui::semibold;

struct Palette {
    page: Color32,
    sidebar: Color32,
    tile: Color32,
    tile_hover: Color32,
    field: Color32,
    ink: Color32,
    ink_strong: Color32,
    muted: Color32,
    faint: Color32,
    line: Color32,
    blue: Color32,
    green: Color32,
    red: Color32,
}

const P: Palette = Palette {
    page: Color32::from_rgb(0x1c, 0x1c, 0x1f),
    sidebar: Color32::from_rgb(0x15, 0x15, 0x18),
    tile: Color32::from_rgb(0x27, 0x27, 0x2b),
    tile_hover: Color32::from_rgb(0x2e, 0x2e, 0x33),
    field: Color32::from_rgb(0x2a, 0x2a, 0x2e),
    ink: Color32::from_rgb(0xe8, 0xe8, 0xed),
    ink_strong: Color32::WHITE,
    muted: Color32::from_rgb(0x98, 0x98, 0xa0),
    faint: Color32::from_rgb(0x6c, 0x6c, 0x74),
    line: Color32::from_rgb(0x34, 0x34, 0x39),
    blue: Color32::from_rgb(0x0a, 0x84, 0xff),
    green: Color32::from_rgb(0x30, 0xd1, 0x58),
    red: Color32::from_rgb(0xff, 0x45, 0x3a),
};

const SIDEBAR_W: f32 = 240.0;
const TILE_H: f32 = 168.0;
const TILE_MIN_W: f32 = 330.0;
const GAP: f32 = 18.0;

const CATEGORIES: [(&str, &str); 5] = [
    ("Image", "Image et illustration"),
    ("Photo", "Photo"),
    ("Vidéo", "Vidéo et animation"),
    ("Publication", "Publication"),
    ("Audio", "Audio"),
];

#[derive(Clone, Copy, PartialEq)]
enum View {
    All,
    Installed,
    Updates,
    Category(&'static str),
}

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

struct Toast {
    text: String,
    good: bool,
    until: f64,
}

struct Launcher {
    catalog: Arc<Mutex<(Catalog, Source)>>,
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    installed: Installed,
    view: View,
    search: String,
    menu: Option<String>,
    confirm_remove: Option<String>,
    settings_open: bool,
    toasts: Vec<Toast>,
    icons: HashMap<String, TextureHandle>,
    logo: TextureHandle,
}

impl Launcher {
    fn new(cc: &eframe::CreationContext) -> Self {
        studio_ui::apply_style(&cc.egui_ctx);
        cc.egui_ctx.all_styles_mut(|s| {
            s.visuals.panel_fill = P.page;
            s.visuals.window_fill = P.tile;
            s.visuals.window_stroke = Stroke::new(1.0, P.line);
            s.visuals.window_corner_radius = CornerRadius::same(14);
            s.visuals.menu_corner_radius = CornerRadius::same(10);
            s.visuals.popup_shadow = Shadow { offset: [0, 10], blur: 30, spread: 0, color: Color32::from_black_alpha(130) };
            s.visuals.window_shadow = s.visuals.popup_shadow;
            s.visuals.selection.bg_fill = P.blue;
            s.visuals.extreme_bg_color = P.field;
            s.visuals.override_text_color = Some(P.ink);
            s.spacing.button_padding = vec2(14.0, 7.0);
        });
        let me = Self {
            catalog: Arc::new(Mutex::new((Catalog::bundled(), Source::Loading))),
            jobs: Arc::default(),
            installed: Installed::load(),
            view: View::All,
            search: String::new(),
            menu: None,
            confirm_remove: None,
            settings_open: false,
            toasts: Vec::new(),
            icons: HashMap::new(),
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
                jobs.lock().unwrap().insert(app.id.clone(), Job::Running(done, total));
                ctx.request_repaint();
            };
            let result = studio_core::install(&app, progress);
            jobs.lock().unwrap().insert(app.id.clone(), match result {
                Ok(()) => Job::Done,
                Err(e) => Job::Failed(e),
            });
            ctx.request_repaint();
        });
    }

    fn toast(&mut self, ctx: &egui::Context, text: String, good: bool) {
        let now = ctx.input(|i| i.time);
        self.toasts.push(Toast { text, good, until: now + 4.0 });
    }

    fn collect_finished(&mut self, ctx: &egui::Context, apps: &[AppEntry]) {
        let finished: Vec<String> = {
            let mut jobs = self.jobs.lock().unwrap();
            let done: Vec<String> = jobs.iter().filter(|(_, j)| matches!(j, Job::Done)).map(|(k, _)| k.clone()).collect();
            for k in &done {
                jobs.remove(k);
            }
            done
        };
        if !finished.is_empty() {
            self.installed = Installed::load();
            for id in finished {
                let name = apps.iter().find(|a| a.id == id).map_or(id.clone(), |a| a.name.clone());
                self.toast(ctx, format!("{name} est installé"), true);
            }
        }
    }

    fn icon(&mut self, ctx: &egui::Context, id: &str) -> TextureHandle {
        self.icons.entry(id.to_string()).or_insert_with(|| studio_ui::icon_texture(ctx, id)).clone()
    }

    fn has_update(&self, app: &AppEntry) -> bool {
        self.installed.apps.get(&app.id).is_some_and(|v| app.version.as_deref().is_some_and(|nv| nv != v))
    }

    fn matches(&self, app: &AppEntry) -> bool {
        let q = self.search.trim().to_lowercase();
        let found = q.is_empty() || app.name.to_lowercase().contains(&q) || app.tagline.to_lowercase().contains(&q);
        let in_view = match self.view {
            View::All => true,
            View::Installed => self.installed.apps.contains_key(&app.id),
            View::Updates => self.has_update(app),
            View::Category(c) => app.category == c,
        };
        found && in_view
    }
}

impl eframe::App for Launcher {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let (apps, source) = {
            let c = self.catalog.lock().unwrap();
            let s = match &c.1 {
                Source::Loading => (P.muted, "Recherche des mises à jour…".to_string()),
                Source::Online => (P.green, "Catalogue à jour".to_string()),
                Source::Offline(e) => (P.red, format!("Hors ligne · {}", short(e, 26))),
            };
            (c.0.apps.clone(), s)
        };
        self.collect_finished(ctx, &apps);
        if self.jobs.lock().unwrap().values().any(|j| matches!(j, Job::Running(..))) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        egui::Panel::left("sidebar")
            .resizable(false)
            .exact_size(SIDEBAR_W)
            .frame(Frame::new().fill(P.sidebar).inner_margin(Margin::symmetric(14, 18)))
            .show(root, |ui| self.sidebar(ui, &apps, &source));

        egui::CentralPanel::default()
            .frame(Frame::new().fill(P.page).inner_margin(Margin { left: 32, right: 32, top: 22, bottom: 0 }))
            .show(root, |ui| self.main(ui, ctx, &apps));

        self.settings_window(ctx);
        self.draw_toasts(ctx);
    }
}

impl Launcher {
    fn sidebar(&mut self, ui: &mut Ui, apps: &[AppEntry], source: &(Color32, String)) {
        ui.horizontal(|ui| {
            ui.add(egui::Image::new((self.logo.id(), Vec2::splat(30.0))));
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.add_space(1.0);
                ui.label(RichText::new("Ethone Studio").font(semibold(16.0)).color(P.ink_strong));
                ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(11.5).color(P.faint));
            });
        });
        ui.add_space(22.0);

        let installed = apps.iter().filter(|a| self.installed.apps.contains_key(&a.id)).count();
        let updates = apps.iter().filter(|a| self.has_update(a)).count();
        heading(ui, "BIBLIOTHÈQUE");
        nav_row(ui, &mut self.view, View::All, "Toutes les apps", None, Some(apps.len()));
        nav_row(ui, &mut self.view, View::Installed, "Installées", None, Some(installed));
        nav_row(ui, &mut self.view, View::Updates, "Mises à jour", None, (updates > 0).then_some(updates));

        ui.add_space(18.0);
        heading(ui, "CATÉGORIES");
        for (key, label) in CATEGORIES {
            let n = apps.iter().filter(|a| a.category == key).count();
            if n == 0 {
                continue;
            }
            let color = apps.iter().find(|a| a.category == key).map(|a| Color32::from_rgb(a.accent[0], a.accent[1], a.accent[2]));
            nav_row(ui, &mut self.view, View::Category(key), label, color, Some(n));
        }

        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), 3.5, source.0);
                ui.label(RichText::new(&source.1).size(12.0).color(P.muted));
            });
            ui.add_space(6.0);
            if plain_row(ui, "Paramètres") {
                self.settings_open = true;
            }
        });
    }

    fn main(&mut self, ui: &mut Ui, ctx: &egui::Context, apps: &[AppEntry]) {
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 60.0).clamp(220.0, 520.0);
            search_pill(ui, &mut self.search, width);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if refresh_button(ui).clicked() {
                    self.refresh(ctx);
                }
            });
        });
        ui.add_space(26.0);

        let title = match self.view {
            View::All => "Toutes les apps",
            View::Installed => "Installées",
            View::Updates => "Mises à jour",
            View::Category(c) => CATEGORIES.iter().find(|(k, _)| *k == c).map_or(c, |x| x.1),
        };
        let visible: Vec<AppEntry> = apps.iter().filter(|a| self.matches(a)).cloned().collect();
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).font(semibold(30.0)).color(P.ink_strong));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let pending: Vec<AppEntry> = visible.iter().filter(|a| a.published() && !self.installed.apps.contains_key(&a.id)).cloned().collect();
                if pending.len() > 1 && pill(ui, &format!("Tout installer ({})", pending.len()), P.line, P.ink_strong, true).clicked() {
                    for a in pending {
                        self.install(ctx, a);
                    }
                }
            });
        });
        let subtitle = match visible.len() {
            0 => "Aucune app".to_string(),
            1 => "1 app".to_string(),
            n => format!("{n} apps"),
        };
        ui.label(RichText::new(subtitle).size(13.5).color(P.muted));
        ui.add_space(18.0);

        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            if visible.is_empty() {
                ui.add_space(80.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Rien à afficher ici").font(semibold(17.0)).color(P.ink));
                    ui.label(RichText::new("Essaie une autre recherche ou une autre catégorie.").color(P.muted));
                });
                return;
            }
            let (mine, others): (Vec<_>, Vec<_>) = visible.into_iter().partition(|a| self.installed.apps.contains_key(&a.id));
            let split = !mine.is_empty() && !others.is_empty();
            if split {
                section_title(ui, "Installées");
            }
            self.grid(ui, ctx, &mine);
            if split {
                ui.add_space(8.0);
                section_title(ui, "Disponibles");
            }
            self.grid(ui, ctx, &others);
            ui.add_space(24.0);
        });
    }

    fn grid(&mut self, ui: &mut Ui, ctx: &egui::Context, apps: &[AppEntry]) {
        if apps.is_empty() {
            return;
        }
        let width = ui.available_width();
        let cols = (((width + GAP) / (TILE_MIN_W + GAP)).floor() as usize).max(1);
        let tile_w = (width - GAP * (cols as f32 - 1.0)) / cols as f32;
        let rows = apps.len().div_ceil(cols);
        let (area, _) = ui.allocate_exact_size(vec2(width, rows as f32 * (TILE_H + GAP)), Sense::hover());
        for (i, app) in apps.iter().enumerate() {
            let (r, c) = (i / cols, i % cols);
            let rect = Rect::from_min_size(area.min + vec2(c as f32 * (tile_w + GAP), r as f32 * (TILE_H + GAP)), vec2(tile_w, TILE_H));
            self.tile(ui, ctx, app, rect);
        }
    }

    fn tile(&mut self, ui: &mut Ui, ctx: &egui::Context, app: &AppEntry, rect: Rect) {
        let accent = Color32::from_rgb(app.accent[0], app.accent[1], app.accent[2]);
        let hovered = ui.rect_contains_pointer(rect);
        let lift = ctx.animate_bool_with_time(egui::Id::NULL.with(("lift", &app.id)), hovered, 0.18);
        let painter = ui.painter().clone();
        let radius = CornerRadius::same(16);
        let shadow = Shadow {
            offset: [0, (3.0 + 7.0 * lift) as i8],
            blur: (14.0 + 18.0 * lift) as u8,
            spread: 0,
            color: Color32::from_black_alpha((70.0 + 50.0 * lift) as u8),
        };
        painter.add(shadow.as_shape(rect, radius));
        painter.rect(rect, radius, P.tile.lerp_to_gamma(P.tile_hover, lift), Stroke::new(1.0, P.line.lerp_to_gamma(accent, 0.35 * lift)), StrokeKind::Inside);

        let pad = 20.0;
        let icon_rect = Rect::from_min_size(rect.min + vec2(pad, pad), Vec2::splat(56.0));
        painter.circle_filled(icon_rect.center(), 34.0, accent.gamma_multiply(0.10 * (0.6 + lift)));
        let tex = self.icon(ctx, &app.id);
        painter.image(tex.id(), icon_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);

        let text_x = icon_rect.right() + 16.0;
        painter.text(pos2(text_x, icon_rect.top() + 6.0), Align2::LEFT_TOP, &app.name, semibold(19.0), P.ink_strong);
        painter.text(pos2(text_x, icon_rect.top() + 33.0), Align2::LEFT_TOP, &app.tagline, FontId::proportional(13.5), P.muted);

        let installed = self.installed.apps.get(&app.id).cloned();
        let job = self.jobs.lock().unwrap().get(&app.id).map(|j| match j {
            Job::Running(d, t) => Ok((*d, *t)),
            Job::Failed(e) => Err(e.clone()),
            Job::Done => Ok((1, 1)),
        });

        let status = match (&installed, &app.version) {
            (Some(v), Some(nv)) if v != nv => format!("Mise à jour {v} → {nv}"),
            (Some(v), _) => format!("Installée · v{v}"),
            (None, Some(nv)) => match app.size {
                Some(s) => format!("v{nv} · {:.0} Mo", s as f64 / 1_048_576.0),
                None => format!("v{nv}"),
            },
            (None, None) => "Bientôt disponible".to_string(),
        };
        let status_color = if installed.is_some() { P.green } else { P.faint };
        let foot_y = rect.bottom() - pad - 17.0;
        painter.text(pos2(rect.left() + pad, foot_y), Align2::LEFT_CENTER, &status, FontId::proportional(12.5), status_color);

        let bar = Rect::from_min_max(pos2(rect.center().x - 20.0, foot_y - 17.0), pos2(rect.right() - pad, foot_y + 17.0));
        let mut actions = ui.new_child(egui::UiBuilder::new().max_rect(bar).layout(Layout::right_to_left(Align::Center)));

        if let Some(Ok((done, total))) = job {
            let frac = if total > 0 { done as f32 / total as f32 } else { 0.0 };
            let w = actions.available_width().min(170.0);
            let (r, _) = actions.allocate_exact_size(vec2(w, 6.0), Sense::hover());
            actions.painter().rect_filled(r, CornerRadius::same(3), P.line);
            actions.painter().rect_filled(Rect::from_min_size(r.min, vec2(r.width() * frac.max(0.03), r.height())), CornerRadius::same(3), P.blue);
            painter.text(pos2(r.left(), r.top() - 10.0), Align2::LEFT_CENTER, format!("Téléchargement {:.0} %", frac * 100.0), FontId::proportional(11.5), P.muted);
            return;
        }
        if let Some(Err(e)) = &job {
            painter.text(pos2(text_x, icon_rect.top() + 54.0), Align2::LEFT_TOP, short(e, 44), FontId::proportional(12.0), P.red);
        }

        if installed.is_some() {
            if dots_button(&mut actions).clicked() {
                self.menu = if self.menu.as_deref() == Some(app.id.as_str()) { None } else { Some(app.id.clone()) };
            }
            actions.add_space(8.0);
        }
        match installed {
            Some(_) => {
                if pill(&mut actions, "Ouvrir", P.blue, Color32::WHITE, true).clicked()
                    && let Err(e) = studio_core::launch(&app.id)
                {
                    self.toast(ctx, format!("{} : {e}", app.name), false);
                }
                if self.has_update(app) && app.published() {
                    actions.add_space(8.0);
                    if pill(&mut actions, "Mettre à jour", P.line, P.ink_strong, true).clicked() {
                        self.install(ctx, app.clone());
                    }
                }
            }
            None => {
                let enabled = app.published();
                let label = if enabled { "Installer" } else { "Bientôt" };
                let (fill, ink) = if enabled { (P.blue, Color32::WHITE) } else { (P.line, P.muted) };
                if pill(&mut actions, label, fill, ink, enabled).clicked() {
                    self.install(ctx, app.clone());
                }
            }
        }

        if self.menu.as_deref() == Some(app.id.as_str()) {
            self.app_menu(ctx, app, pos2(rect.right() - pad, foot_y + 22.0));
        }
    }

    fn app_menu(&mut self, ctx: &egui::Context, app: &AppEntry, at: Pos2) {
        let mut close = false;
        let area = egui::Area::new(egui::Id::NULL.with(("menu", &app.id)))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .pivot(Align2::RIGHT_TOP)
            .show(ctx, |ui| {
                Frame::popup(ui.style()).inner_margin(Margin::same(6)).show(ui, |ui| {
                    ui.set_min_width(220.0);
                    if menu_item(ui, "Afficher dans l'explorateur", P.ink) {
                        let _ = std::process::Command::new("explorer").arg(studio_core::app_dir(&app.id)).spawn();
                        close = true;
                    }
                    if app.published() && menu_item(ui, "Réinstaller", P.ink) {
                        self.install(ctx, app.clone());
                        close = true;
                    }
                    let confirming = self.confirm_remove.as_deref() == Some(app.id.as_str());
                    let label = if confirming { "Confirmer la désinstallation" } else { "Désinstaller…" };
                    if menu_item(ui, label, P.red) {
                        if confirming {
                            match studio_core::uninstall(&app.id) {
                                Ok(()) => self.toast(ctx, format!("{} a été désinstallé", app.name), true),
                                Err(e) => self.toast(ctx, format!("{} : {e}", app.name), false),
                            }
                            self.installed = Installed::load();
                            close = true;
                        } else {
                            self.confirm_remove = Some(app.id.clone());
                        }
                    }
                });
            });
        let outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !area.response.rect.expand(4.0).contains(p)));
        if close || outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.menu = None;
            self.confirm_remove = None;
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        egui::Window::new(RichText::new("Paramètres").font(semibold(17.0)))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(Frame::window(&ctx.global_style()).inner_margin(Margin::same(22)))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_width(420.0);
                ui.label(RichText::new("Dossier des apps").font(semibold(13.5)).color(P.ink));
                ui.label(RichText::new(studio_core::data_dir().join("apps").display().to_string()).size(12.5).color(P.muted));
                ui.add_space(6.0);
                if pill(ui, "Ouvrir le dossier", P.line, P.ink_strong, true).clicked() {
                    let dir = studio_core::data_dir().join("apps");
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::process::Command::new("explorer").arg(dir).spawn();
                }
                ui.add_space(16.0);
                ui.label(RichText::new("Source des mises à jour").font(semibold(13.5)).color(P.ink));
                ui.label(RichText::new(studio_core::MANIFEST_URL).size(12.0).color(P.muted));
                ui.add_space(16.0);
                ui.label(RichText::new("À propos").font(semibold(13.5)).color(P.ink));
                ui.label(RichText::new(format!("Ethone Studio {} · écrit en Rust", env!("CARGO_PKG_VERSION"))).size(12.5).color(P.muted));
            });
        if !open {
            self.settings_open = false;
        }
    }

    fn draw_toasts(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        self.toasts.retain(|t| t.until > now);
        if self.toasts.is_empty() {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
        egui::Area::new(egui::Id::NULL.with("toasts"))
            .order(egui::Order::Tooltip)
            .anchor(Align2::RIGHT_BOTTOM, vec2(-24.0, -24.0))
            .show(ctx, |ui| {
                for t in &self.toasts {
                    Frame::popup(ui.style()).inner_margin(Margin::symmetric(16, 12)).corner_radius(CornerRadius::same(12)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                            ui.painter().circle_filled(dot.center(), 4.0, if t.good { P.green } else { P.red });
                            ui.label(RichText::new(&t.text).color(P.ink_strong));
                        });
                    });
                    ui.add_space(8.0);
                }
            });
    }
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n).collect::<String>()) }
}

fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(text).font(semibold(11.0)).color(P.faint));
    ui.add_space(4.0);
}

fn section_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(semibold(15.0)).color(P.ink));
    ui.add_space(10.0);
}

fn nav_row(ui: &mut Ui, view: &mut View, value: View, label: &str, dot: Option<Color32>, count: Option<usize>) {
    let selected = *view == value;
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, CornerRadius::same(9), P.blue.gamma_multiply(0.22));
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(9), Color32::from_white_alpha(8));
    }
    let mut x = rect.left() + 12.0;
    if let Some(c) = dot {
        p.circle_filled(pos2(x + 4.0, rect.center().y), 4.0, c);
        x += 18.0;
    }
    let font = if selected { semibold(14.0) } else { FontId::proportional(14.0) };
    p.text(pos2(x, rect.center().y), Align2::LEFT_CENTER, label, font, if selected { P.ink_strong } else { P.ink });
    if let Some(n) = count {
        p.text(pos2(rect.right() - 12.0, rect.center().y), Align2::RIGHT_CENTER, n.to_string(), FontId::proportional(12.5), if selected { P.ink } else { P.faint });
    }
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        *view = value;
    }
}

fn plain_row(ui: &mut Ui, label: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(9), Color32::from_white_alpha(8));
    }
    let c = rect.left_center() + vec2(18.0, 0.0);
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::TAU / 8.0;
        ui.painter().line_segment([c + vec2(a.cos(), a.sin()) * 5.0, c + vec2(a.cos(), a.sin()) * 7.5], Stroke::new(2.0, P.muted));
    }
    ui.painter().circle_stroke(c, 4.0, Stroke::new(1.6, P.muted));
    ui.painter().text(rect.left_center() + vec2(34.0, 0.0), Align2::LEFT_CENTER, label, FontId::proportional(14.0), P.ink);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn search_pill(ui: &mut Ui, text: &mut String, width: f32) {
    Frame::new().fill(P.field).corner_radius(CornerRadius::same(18)).inner_margin(Margin::symmetric(14, 0)).stroke(Stroke::new(1.0, P.line)).show(ui, |ui| {
        ui.set_width(width - 28.0);
        ui.horizontal_centered(|ui| {
            let (g, _) = ui.allocate_exact_size(vec2(16.0, 36.0), Sense::hover());
            let c = g.center() - vec2(1.5, 1.5);
            ui.painter().circle_stroke(c, 5.0, Stroke::new(1.6, P.muted));
            ui.painter().line_segment([c + vec2(3.6, 3.6), c + vec2(7.0, 7.0)], Stroke::new(1.6, P.muted));
            ui.add(
                egui::TextEdit::singleline(text)
                    .hint_text(RichText::new("Rechercher une app").color(P.faint))
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY)
                    .font(FontId::proportional(14.0)),
            );
        });
    });
}

fn pill(ui: &mut Ui, label: &str, fill: Color32, ink: Color32, enabled: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(label.to_string(), semibold(13.5), ink);
    let size = vec2(galley.size().x + 32.0, 34.0);
    let (rect, resp) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    let fill = if resp.hovered() && enabled { fill.lerp_to_gamma(Color32::WHITE, 0.12) } else { fill };
    ui.painter().rect_filled(rect, CornerRadius::same(17), fill);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, ink);
    if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp }
}

fn dots_button(ui: &mut Ui) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
    let fill = if resp.hovered() { Color32::from_white_alpha(22) } else { Color32::from_white_alpha(10) };
    ui.painter().circle_filled(rect.center(), 17.0, fill);
    for dx in [-6.0, 0.0, 6.0] {
        ui.painter().circle_filled(rect.center() + vec2(dx, 0.0), 1.9, P.ink);
    }
    resp.on_hover_text("Plus d'options").on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn refresh_button(ui: &mut Ui) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::click());
    let fill = if resp.hovered() { P.tile_hover } else { P.field };
    ui.painter().circle(rect.center(), 19.0, fill, Stroke::new(1.0, P.line));
    let c = rect.center();
    let r = 7.0;
    let pts: Vec<Pos2> = (0..=20).map(|k| {
        let a = -0.6 + k as f32 / 20.0 * 5.0;
        c + vec2(a.cos(), a.sin()) * r
    }).collect();
    let end = *pts.last().unwrap_or(&c);
    ui.painter().add(egui::Shape::line(pts, Stroke::new(1.8, P.ink)));
    ui.painter().add(egui::Shape::convex_polygon(vec![end + vec2(-4.0, -1.0), end + vec2(3.0, -3.5), end + vec2(1.5, 3.5)], P.ink, Stroke::NONE));
    resp.on_hover_text("Rechercher des mises à jour").on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn menu_item(ui: &mut Ui, label: &str, color: Color32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(220.0), 32.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(7), Color32::from_white_alpha(14));
    }
    ui.painter().text(rect.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, label, FontId::proportional(14.0), color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Ethone Studio")
            .with_icon(Arc::new(studio_ui::app_window_icon("studio")))
            .with_inner_size([1240.0, 800.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native("Ethone Studio", options, Box::new(|cc| Ok(Box::new(Launcher::new(cc)))))
}
