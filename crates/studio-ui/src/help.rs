use crate::pro::{BLUE, CHROME, FIELD, INK, INK_STRONG, LINE, MUTED, PANEL, semibold};
use egui::{Align2, Color32, Context, CornerRadius, Frame, Key, Margin, RichText, Stroke, Ui, vec2};

pub struct Tuto {
    pub title: &'static str,
    pub key: &'static str,
    pub summary: &'static str,
    pub steps: &'static [&'static str],
}

/// Aide > Apprendre (F1): searchable tutorials of the app's real tools.
pub struct Help {
    app: &'static str,
    tutos: &'static [Tuto],
    pub open: bool,
    query: String,
    selected: Option<usize>,
}

impl Help {
    pub fn new(app: &'static str, tutos: &'static [Tuto]) -> Self {
        Self { app, tutos, open: false, query: String::new(), selected: None }
    }

    pub fn titles(&self) -> Vec<&'static str> {
        self.tutos.iter().map(|t| t.title).collect()
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn learn(&mut self, title: &str) {
        self.open = true;
        self.selected = self.tutos.iter().position(|t| t.title == title);
    }

    pub fn menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Aide", |ui| {
            if ui.button("Apprendre                F1").clicked() {
                self.open = true;
            }
        });
    }

    pub fn show(&mut self, ctx: &Context) {
        if ctx.input(|i| i.key_pressed(Key::F1)) {
            self.toggle();
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
                    ui.label(RichText::new("Apprendre").font(semibold(15.0)).color(INK_STRONG));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("✕").color(MUTED)).frame(false)).clicked() {
                            open = false;
                        }
                    });
                });
                ui.add_space(8.0);
                self.learn_tab(ui);
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
}

fn key_chip(ui: &mut Ui, key: &str) {
    Frame::new().fill(FIELD).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(5)).inner_margin(Margin::symmetric(6, 1)).show(ui, |ui| {
        ui.label(RichText::new(key).size(11.5).color(Color32::from_gray(200)));
    });
}
