use crate::pro::{self, BLUE, CHROME, FAINT, FIELD, INK, INK_STRONG, LINE, MUTED, PANEL, semibold};
use egui::{Align2, Color32, Context, CornerRadius, Frame, Key, Margin, RichText, Stroke, Ui, vec2};
use std::io::{BufRead, BufReader};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

pub struct Tuto {
    pub title: &'static str,
    pub key: &'static str,
    pub summary: &'static str,
    pub steps: &'static [&'static str],
}

const HOST: &str = "http://127.0.0.1:11434";
pub const MODEL: &str = "qwen2.5:1.5b";

#[derive(Clone, PartialEq)]
enum Ai {
    Unknown,
    Missing,
    Offline,
    Starting,
    NoModel,
    Pulling(String),
    Ready,
    Failed(String),
}

enum Msg {
    Status(Ai),
    Reply(String),
}

#[derive(PartialEq)]
enum Tab {
    Learn,
    Chat,
}

pub struct Help {
    app: &'static str,
    tutos: &'static [Tuto],
    pub open: bool,
    tab: Tab,
    query: String,
    selected: Option<usize>,
    ai: Ai,
    chat: Vec<(bool, String)>,
    input: String,
    waiting: bool,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

fn agent(secs: u64) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(secs))).build().into()
}

fn ollama_exe() -> Option<std::path::PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA").map(|d| std::path::PathBuf::from(d).join("Programs\\Ollama\\ollama.exe"));
    if let Some(p) = local.filter(|p| p.exists()) {
        return Some(p);
    }
    std::env::var_os("PATH")?.to_str()?.split(';').map(|d| std::path::Path::new(d).join("ollama.exe")).find(|p| p.exists())
}

fn probe() -> Ai {
    match agent(2).get(format!("{HOST}/api/tags")).call() {
        Ok(mut r) => {
            let body = r.body_mut().read_to_string().unwrap_or_default();
            if body.contains(MODEL) { Ai::Ready } else { Ai::NoModel }
        }
        Err(_) if ollama_exe().is_none() => Ai::Missing,
        Err(_) => Ai::Offline,
    }
}

fn start_server() {
    let Some(exe) = ollama_exe() else { return };
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("serve").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let _ = cmd.spawn();
}

fn pull(tx: &Sender<Msg>, ctx: &Context) -> Result<(), String> {
    let body = serde_json::json!({ "model": MODEL, "stream": true }).to_string();
    let resp = agent(3600).post(format!("{HOST}/api/pull")).header("Content-Type", "application/json").send(&body).map_err(|e| e.to_string())?;
    for line in BufReader::new(resp.into_body().into_reader()).lines() {
        let line = line.map_err(|e| e.to_string())?;
        let v: serde_json::Value = serde_json::from_str(&line).unwrap_or_default();
        if let Some(e) = v["error"].as_str() {
            return Err(e.into());
        }
        let label = match (v["completed"].as_f64(), v["total"].as_f64()) {
            (Some(c), Some(t)) if t > 0.0 => format!("{:.0} %", c / t * 100.0),
            _ => v["status"].as_str().unwrap_or("…").to_string(),
        };
        let _ = tx.send(Msg::Status(Ai::Pulling(label)));
        ctx.request_repaint();
    }
    Ok(())
}

fn ask(system: String, history: Vec<(bool, String)>) -> String {
    let mut messages = vec![serde_json::json!({ "role": "system", "content": system })];
    messages.extend(history.into_iter().map(|(user, text)| serde_json::json!({ "role": if user { "user" } else { "assistant" }, "content": text })));
    let body = serde_json::json!({ "model": MODEL, "stream": false, "messages": messages }).to_string();
    match agent(300).post(format!("{HOST}/api/chat")).header("Content-Type", "application/json").send(&body) {
        Ok(mut r) => {
            let v: serde_json::Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap_or_default()).unwrap_or_default();
            v["message"]["content"].as_str().map(|s| s.trim().to_string()).unwrap_or_else(|| "Réponse vide.".into())
        }
        Err(e) => format!("Impossible de joindre l'assistant : {e}"),
    }
}

impl Help {
    pub fn new(app: &'static str, tutos: &'static [Tuto]) -> Self {
        let (tx, rx) = channel();
        Self { app, tutos, open: false, tab: Tab::Learn, query: String::new(), selected: None, ai: Ai::Unknown, chat: Vec::new(), input: String::new(), waiting: false, tx, rx }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn learn(&mut self, title: &str) {
        self.open = true;
        self.tab = Tab::Learn;
        self.selected = self.tutos.iter().position(|t| t.title == title);
    }

    fn spawn(&self, ctx: &Context, job: impl FnOnce(&Sender<Msg>, &Context) + Send + 'static) {
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            job(&tx, &ctx);
            ctx.request_repaint();
        });
    }

    fn refresh(&mut self, ctx: &Context) {
        self.ai = Ai::Starting;
        self.spawn(ctx, |tx, _| {
            let _ = tx.send(Msg::Status(probe()));
        });
    }

    fn system_prompt(&self) -> String {
        let tools: Vec<String> = self.tutos.iter().map(|t| format!("- {} ({}) : {}", t.title, t.key, t.summary)).collect();
        format!(
            "Tu es l'assistant intégré d'Ethone {app}, une application de création de la suite Ethone Studio. Réponds en français, de façon brève et concrète, avec des étapes numérotées quand c'est utile. Appuie-toi sur les outils réels de l'application :\n{}\nSi une fonction n'existe pas, dis-le simplement et propose l'outil le plus proche.",
            tools.join("\n"),
            app = self.app
        )
    }

    pub fn menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Aide", |ui| {
            if ui.button("Apprendre                F1").clicked() {
                self.open = true;
                self.tab = Tab::Learn;
            }
            if ui.button("Assistant IA").clicked() {
                self.open = true;
                self.tab = Tab::Chat;
            }
        });
    }

    pub fn show(&mut self, ctx: &Context) {
        if ctx.input(|i| i.key_pressed(Key::F1)) {
            self.toggle();
        }
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Status(s) => self.ai = s,
                Msg::Reply(r) => {
                    self.waiting = false;
                    self.chat.push((false, r));
                }
            }
        }
        if !self.open {
            return;
        }
        let mut open = self.open;
        egui::Window::new("help")
            .id(egui::Id::NULL.with(("ethone-help", self.app)))
            .title_bar(false)
            .resizable(true)
            .default_size(vec2(380.0, 560.0))
            .anchor(Align2::RIGHT_TOP, vec2(-16.0, 104.0))
            .frame(Frame::new().fill(CHROME).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(14)).inner_margin(Margin::same(14)).shadow(ctx.global_style().visuals.window_shadow))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (tab, label) in [(Tab::Learn, "Apprendre"), (Tab::Chat, "Assistant IA")] {
                        let on = self.tab == tab;
                        let fill = if on { BLUE } else { FIELD };
                        if pro::pill(ui, label, fill, if on { INK_STRONG } else { INK }).clicked() {
                            if tab == Tab::Chat && self.ai == Ai::Unknown {
                                self.refresh(ctx);
                            }
                            self.tab = tab;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("✕").color(MUTED)).frame(false)).clicked() {
                            open = false;
                        }
                    });
                });
                ui.add_space(8.0);
                match self.tab {
                    Tab::Learn => self.learn_tab(ui),
                    Tab::Chat => self.chat_tab(ui, ctx),
                }
            });
        self.open = open;
    }

    fn learn_tab(&mut self, ui: &mut Ui) {
        if let Some(i) = self.selected {
            let t = &self.tutos[i];
            if ui.add(egui::Button::new(RichText::new("‹  Tous les tutoriels").color(BLUE)).frame(false)).clicked() {
                self.selected = None;
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t.title).font(semibold(18.0)).color(INK_STRONG));
                if !t.key.is_empty() {
                    key_chip(ui, t.key);
                }
            });
            ui.label(RichText::new(t.summary).color(MUTED));
            ui.add_space(10.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (n, step) in t.steps.iter().enumerate() {
                    ui.horizontal_top(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(22.0, 22.0), egui::Sense::hover());
                        ui.painter().circle_filled(r.center(), 11.0, BLUE);
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, (n + 1).to_string(), semibold(12.0), INK_STRONG);
                        ui.add(egui::Label::new(RichText::new(*step).color(INK)).wrap());
                    });
                    ui.add_space(6.0);
                }
                ui.add_space(8.0);
                if pro::secondary(ui, "Poser une question à l'assistant").clicked() {
                    self.input = format!("Comment bien utiliser « {} » ?", t.title);
                    self.tab = Tab::Chat;
                }
            });
            return;
        }
        ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Rechercher un outil ou une fonction").desired_width(f32::INFINITY));
        ui.add_space(6.0);
        let q = self.query.to_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, t) in self.tutos.iter().enumerate() {
                if !q.is_empty() && !t.title.to_lowercase().contains(&q) && !t.summary.to_lowercase().contains(&q) {
                    continue;
                }
                let r = Frame::new().fill(PANEL).corner_radius(CornerRadius::same(10)).inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t.title).font(semibold(14.0)).color(INK_STRONG));
                        if !t.key.is_empty() {
                            key_chip(ui, t.key);
                        }
                    });
                    ui.add(egui::Label::new(RichText::new(t.summary).color(MUTED).size(12.5)).wrap());
                });
                if r.response.interact(egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.selected = Some(i);
                }
                ui.add_space(4.0);
            }
        });
    }

    fn chat_tab(&mut self, ui: &mut Ui, ctx: &Context) {
        let status = match &self.ai {
            Ai::Unknown | Ai::Starting => "Connexion à l'assistant…".to_string(),
            Ai::Missing => "L'assistant fonctionne avec Ollama, gratuit et 100 % local. Installe-le depuis ollama.com puis reviens ici.".into(),
            Ai::Offline => "Ollama est installé mais arrêté.".into(),
            Ai::NoModel => format!("Le modèle {MODEL} (environ 1 Go) n'est pas encore téléchargé."),
            Ai::Pulling(p) => format!("Téléchargement du modèle… {p}"),
            Ai::Failed(e) => format!("Erreur : {e}"),
            Ai::Ready => String::new(),
        };
        if !status.is_empty() {
            Frame::new().fill(PANEL).corner_radius(CornerRadius::same(10)).inner_margin(Margin::same(12)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add(egui::Label::new(RichText::new(&status).color(INK)).wrap());
                ui.add_space(6.0);
                match self.ai {
                    Ai::Offline if pro::primary(ui, "Démarrer l'assistant").clicked() => {
                        self.ai = Ai::Starting;
                        self.spawn(ctx, |tx, _| {
                            start_server();
                            for _ in 0..20 {
                                std::thread::sleep(Duration::from_millis(500));
                                let s = probe();
                                if s != Ai::Offline {
                                    let _ = tx.send(Msg::Status(s));
                                    return;
                                }
                            }
                            let _ = tx.send(Msg::Status(Ai::Failed("Ollama ne répond pas.".into())));
                        });
                    }
                    Ai::NoModel if pro::primary(ui, "Télécharger le modèle").clicked() => {
                        self.ai = Ai::Pulling("…".into());
                        self.spawn(ctx, |tx, ctx| {
                            let s = match pull(tx, ctx) {
                                Ok(()) => probe(),
                                Err(e) => Ai::Failed(e),
                            };
                            let _ = tx.send(Msg::Status(s));
                        });
                    }
                    Ai::Missing | Ai::Failed(_) if pro::secondary(ui, "Réessayer").clicked() => self.refresh(ctx),
                    Ai::Missing => {}
                    _ => {}
                }
            });
            if self.ai != Ai::Ready {
                return;
            }
        }
        let input_h = 64.0;
        egui::ScrollArea::vertical().max_height(ui.available_height() - input_h).stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
            if self.chat.is_empty() {
                ui.label(RichText::new(format!("Demande n'importe quoi sur Ethone {}. Tout reste sur ton PC.", self.app)).color(FAINT));
            }
            for (user, text) in &self.chat {
                let layout = if *user { egui::Layout::right_to_left(egui::Align::Min) } else { egui::Layout::left_to_right(egui::Align::Min) };
                ui.with_layout(layout, |ui| {
                    Frame::new().fill(if *user { BLUE } else { PANEL }).corner_radius(CornerRadius::same(12)).inner_margin(Margin::symmetric(10, 7)).show(ui, |ui| {
                        ui.set_max_width(270.0);
                        ui.add(egui::Label::new(RichText::new(text).color(if *user { INK_STRONG } else { INK })).wrap());
                    });
                });
                ui.add_space(4.0);
            }
            if self.waiting {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("L'assistant réfléchit…").color(MUTED));
                });
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let edit = ui.add(egui::TextEdit::singleline(&mut self.input).hint_text("Écris ta question").desired_width(ui.available_width() - 76.0));
            let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            if (pro::primary(ui, "Envoyer").clicked() || enter) && !self.waiting && !self.input.trim().is_empty() {
                self.chat.push((true, std::mem::take(&mut self.input)));
                self.waiting = true;
                let (system, history) = (self.system_prompt(), self.chat.clone());
                self.spawn(ctx, move |tx, _| {
                    let _ = tx.send(Msg::Reply(ask(system, history)));
                });
                edit.request_focus();
            }
        });
    }
}

fn key_chip(ui: &mut Ui, key: &str) {
    Frame::new().fill(FIELD).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(5)).inner_margin(Margin::symmetric(6, 1)).show(ui, |ui| {
        ui.label(RichText::new(key).size(11.5).color(Color32::from_gray(200)));
    });
}
