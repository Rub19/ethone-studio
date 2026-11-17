#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Ethone Studio installer: one stub, one app appended per installer (Pixel-Setup.exe, Lumen-Setup.exe…).
//! `ethone-setup --pack <app.exe> <id> <version> <out.exe>` builds an offline installer, `--web <id> <out.exe>`
//! a small one that downloads the app (checked by SHA-256) like Adobe's Set-up.exe; `/S` or `--silent` installs
//! without a window; `--uninstall` (used by « Applications installées ») removes the app.

mod payload;
mod system;

use eframe::egui::{self, Color32, RichText};
use studio_ui::i18n::tl;
use studio_ui::pro;

/// The main call to action, at installer scale: wider and taller than the in-app buttons.
fn cta(ui: &mut egui::Ui, label: &str, primary: bool) -> bool {
    let text = RichText::new(tl(&label)).font(pro::semibold(13.5)).color(if primary { Color32::WHITE } else { pro::INK_STRONG });
    let b = egui::Button::new(text).fill(if primary { pro::BLUE } else { pro::FIELD }).corner_radius(4).min_size([150.0, 34.0].into());
    ui.add(b).clicked()
}
use std::sync::{Arc, Mutex};

#[derive(Clone, PartialEq)]
enum Step {
    Ready,
    Working(f32, String),
    Done,
    Failed(String),
}

struct Job {
    file: Arc<Vec<u8>>,
    footer: payload::Footer,
}

fn install(job: &Job, desktop: bool, report: &dyn Fn(f32, &str)) -> Result<(), String> {
    let id = job.footer.id.as_str();
    report(0.02, "Fermeture de l'application si elle est ouverte");
    system::close_app(id);
    std::fs::create_dir_all(system::dir(id)).map_err(|e| format!("dossier impossible à créer : {e}"))?;
    let version = if job.footer.payload_len == 0 {
        report(0.05, "Recherche de la dernière version");
        let catalog = studio_core::Catalog::fetch().map_err(|e| format!("catalogue inaccessible : {e}"))?;
        let app = catalog.apps.into_iter().find(|a| a.id == id && a.published()).ok_or("cette application n'est pas encore publiée")?;
        studio_core::install(&app, |done, total| report(0.05 + 0.85 * done as f32 / total.max(1) as f32, "Téléchargement"))?;
        app.version.unwrap_or_default()
    } else {
        report(0.05, "Copie des fichiers");
        payload::extract(&job.file, &job.footer, &system::exe(id), &|p| report(0.05 + p * 0.85, "Copie des fichiers"))?;
        system::track(id, &job.footer.version);
        job.footer.version.clone()
    };
    report(0.92, "Création du désinstalleur");
    std::fs::write(system::dir(id).join(system::UNINSTALLER), &job.file[..job.footer.stub_len as usize]).map_err(|e| e.to_string())?;
    report(0.95, "Raccourcis, types de fichiers et inscription dans Windows");
    system::shortcuts(id, desktop)?;
    system::associate(id)?;
    system::register(id, &version)?;
    report(1.0, "Terminé");
    Ok(())
}

fn uninstall(id: &str, purge: bool, report: &dyn Fn(f32, &str)) -> Result<(), String> {
    report(0.1, "Fermeture de l'application");
    system::close_app(id);
    report(0.4, "Suppression des raccourcis");
    system::remove_shortcuts(id);
    report(0.7, "Suppression des fichiers");
    let exe = system::exe(id);
    if exe.exists() {
        std::fs::remove_file(&exe).map_err(|e| format!("suppression impossible : {e}"))?;
    }
    system::unregister(id);
    system::dissociate(id);
    system::untrack(id);
    if purge {
        report(0.85, "Suppression des préférences");
        let _ = std::fs::remove_dir_all(studio_core::data_dir().join(id));
    }
    report(1.0, "Terminé");
    Ok(())
}

/// The uninstaller is the bare stub: it finds its app from the folder it lives in.
fn uninstall_target() -> Option<String> {
    let me = std::env::current_exe().ok()?;
    let id = me.parent()?.file_name()?.to_str()?.to_string();
    system::APPS.iter().any(|a| a.0 == id).then_some(id)
}

struct Setup {
    job: Option<Arc<Job>>,
    uninstall_id: Option<String>,
    step: Arc<Mutex<Step>>,
    desktop: bool,
    open_after: bool,
    purge: bool,
    previous: Option<String>,
}

impl Setup {
    fn id(&self) -> String {
        self.job.as_ref().map(|j| j.footer.id.clone()).or_else(|| self.uninstall_id.clone()).unwrap_or_default()
    }

    fn start(&self) {
        let step = self.step.clone();
        let report = move |p: f32, label: &str| *step.lock().unwrap_or_else(|e| e.into_inner()) = Step::Working(p, label.to_string());
        let done = self.step.clone();
        let (job, id, desktop, purge) = (self.job.clone(), self.uninstall_id.clone(), self.desktop, self.purge);
        report(0.0, "Préparation");
        std::thread::spawn(move || {
            let r = match (&job, &id) {
                (Some(j), _) => install(j, desktop, &report),
                (None, Some(id)) => uninstall(id, purge, &report),
                _ => Err("rien à installer".into()),
            };
            *done.lock().unwrap_or_else(|e| e.into_inner()) = match r {
                Ok(()) => Step::Done,
                Err(e) => Step::Failed(e),
            };
        });
    }
}

impl eframe::App for Setup {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let step = self.step.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if matches!(step, Step::Working(..)) {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        let id = self.id();
        let removing = self.job.is_none();
        let version = self.job.as_ref().map(|j| j.footer.version.clone()).unwrap_or_default();
        egui::CentralPanel::default().show(root, |ui| {
            let outer = studio_ui::chrome::bar(ui, 0.0);
            studio_ui::chrome::buttons(ui, outer);
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                ui.add_space(28.0);
                studio_ui::app_icon(ui, 96.0, if id == "ethone-studio" { "studio" } else { &id }, Color32::WHITE);
                ui.add_space(18.0);
                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    ui.label(RichText::new(system::name(&id)).font(pro::semibold(26.0)).color(pro::INK_STRONG));
                    let desc = system::APPS.iter().find(|a| a.0 == id).map_or("", |a| a.2);
                    ui.label(RichText::new(tl(&desc)).size(13.5).color(pro::MUTED));
                    if !removing {
                        let line = match &self.previous {
                            Some(v) if *v == version => format!("Version {version} · déjà installée, elle sera réparée"),
                            Some(v) => format!("Version {version} · remplace la {v}"),
                            None => format!("Version {version}"),
                        };
                        ui.label(RichText::new(tl(&line)).size(12.0).color(pro::FAINT));
                    }
                });
            });
            ui.add_space(26.0);
            ui.horizontal(|ui| {
                ui.add_space(28.0);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - 28.0);
                    match &step {
                        Step::Ready if removing => {
                            ui.label(format!("{} va être supprimée de cet ordinateur. Tes documents ne sont jamais touchés.", system::name(&id)));
                            ui.add_space(10.0);
                            ui.checkbox(&mut self.purge, tl("Supprimer aussi les préférences de l'application"));
                            ui.add_space(14.0);
                            if cta(ui, tl("Désinstaller"), false) {
                                self.start();
                            }
                        }
                        Step::Ready => {
                            ui.label(format!("Installation pour ton compte Windows, sans droits administrateur, dans :\n{}", system::dir(&id).display()));
                            ui.add_space(10.0);
                            ui.checkbox(&mut self.desktop, tl("Créer un raccourci sur le Bureau"));
                            ui.checkbox(&mut self.open_after, tl("Ouvrir l'application à la fin"));
                            ui.add_space(14.0);
                            if cta(ui, tl("Installer"), true) {
                                self.start();
                            }
                        }
                        Step::Working(p, label) => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(tl(&label)).size(13.0).color(pro::INK));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(RichText::new(format!("{:.0} %", p * 100.0)).size(12.0).color(pro::MUTED));
                                });
                            });
                            ui.add_space(6.0);
                            ui.add(egui::ProgressBar::new(*p).fill(pro::BLUE).corner_radius(2).desired_height(5.0));
                        }
                        Step::Done => {
                            ui.label(RichText::new(if removing { "Désinstallation terminée." } else { "Installation terminée." }).font(pro::semibold(16.0)).color(pro::INK_STRONG));
                            ui.add_space(14.0);
                            if cta(ui, if removing || !self.open_after { "Fermer" } else { "Ouvrir" }, true) {
                                if removing {
                                    system::remove_self_later();
                                } else if self.open_after {
                                    system::launch(&id);
                                }
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                        Step::Failed(e) => {
                            ui.colored_label(pro::RED, format!("Échec : {e}"));
                            ui.add_space(12.0);
                            if cta(ui, tl("Réessayer"), false) {
                                *self.step.lock().unwrap_or_else(|e| e.into_inner()) = Step::Ready;
                            }
                        }
                    }
                });
            });
            let foot = egui::Rect::from_min_max(egui::pos2(ui.max_rect().left(), ui.max_rect().bottom() - 34.0), ui.max_rect().max);
            ui.painter().hline(foot.x_range(), foot.top(), egui::Stroke::new(1.0, pro::LINE));
            ui.painter().text(foot.left_center() + egui::vec2(28.0, 0.0), egui::Align2::LEFT_CENTER, tl("Ethone Studio"), pro::semibold(11.5), pro::MUTED);
            ui.painter().text(foot.right_center() - egui::vec2(28.0, 0.0), egui::Align2::RIGHT_CENTER, tl("Sans compte · sans droits administrateur"), egui::FontId::proportional(11.0), pro::FAINT);
        });
    }
}

fn stub() -> Result<Vec<u8>, String> {
    let me = std::fs::read(std::env::current_exe().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let len = payload::footer(&me).map_or(me.len(), |f| f.stub_len as usize);
    Ok(me[..len].to_vec())
}

fn web_cli(id: &str, out: &str) -> Result<(), String> {
    std::fs::write(out, payload::pack(&stub()?, &[], id, "web")).map_err(|e| format!("{out} : {e}"))
}

fn pack_cli(args: &[String]) -> Result<(), String> {
    let [app, id, version, out] = args else { return Err("usage : --pack <app.exe> <id> <version> <sortie.exe>".into()) };
    let me = std::fs::read(std::env::current_exe().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let stub = payload::footer(&me).map_or(&me[..], |f| &me[..f.stub_len as usize]);
    let exe = std::fs::read(app).map_err(|e| format!("{app} : {e}"))?;
    std::fs::write(out, payload::pack(stub, &exe, id, version)).map_err(|e| format!("{out} : {e}"))
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--web") {
        let r = match &args[1..] {
            [id, out] => web_cli(id, out),
            _ => Err("usage : --web <id> <sortie.exe>".into()),
        };
        if let Err(e) = r {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("--pack") {
        if let Err(e) = pack_cli(&args[1..]) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    let silent = args.iter().any(|a| a == "/S" || a == "--silent");
    let file = std::env::current_exe().and_then(std::fs::read).unwrap_or_default();
    let job = payload::footer(&file).map(|footer| Arc::new(Job { file: Arc::new(file), footer }));
    let uninstall_id = if job.is_none() || args.iter().any(|a| a == "--uninstall") { uninstall_target() } else { None };
    let job = if uninstall_id.is_some() && args.iter().any(|a| a == "--uninstall") { None } else { job };
    if job.is_none() && uninstall_id.is_none() {
        eprintln!("Cet installateur ne contient aucune application.");
        std::process::exit(1);
    }
    if silent {
        let quiet = |_: f32, _: &str| {};
        let r = match (&job, &uninstall_id) {
            (Some(j), _) => install(j, false, &quiet),
            (None, Some(id)) => uninstall(id, args.iter().any(|a| a == "--purge"), &quiet).map(|_| system::remove_self_later()),
            _ => Ok(()),
        };
        std::process::exit(if r.is_ok() { 0 } else { 1 });
    }
    let id = job.as_ref().map(|j| j.footer.id.clone()).or_else(|| uninstall_id.clone()).unwrap_or_default();
    let title = format!("{} de {}", if job.is_some() { "Installation" } else { "Désinstallation" }, system::name(&id));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_title(&title)
            .with_icon(Arc::new(studio_ui::app_window_icon(if id == "ethone-studio" { "studio" } else { &id })))
            .with_inner_size([720.0, 430.0])
            .with_resizable(false),
        ..Default::default()
    };
    let previous = system::installed_version(&id);
    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            studio_ui::pro::apply(&cc.egui_ctx);
            studio_ui::i18n::set_lang(studio_ui::palette::load_prefs().lang);
            Ok(Box::new(Setup { job, uninstall_id, step: Arc::new(Mutex::new(Step::Ready)), desktop: true, open_after: true, purge: false, previous }))
        }),
    )
}
