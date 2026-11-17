use crate::pro::{CHROME, FAINT, GREEN, INK, LINE, RED};
use crate::i18n::tl;
use egui::{Align2, Context, CornerRadius, Frame, Margin, RichText, Stroke, Ui};
use std::path::{Path, PathBuf};
use studio_core::handoff::{Inbox, name};

pub fn menu(ui: &mut Ui, targets: &[(&'static str, &str)]) -> Option<&'static str> {
    let mut pick = None;
    ui.menu_button(tl("Envoyer vers"), |ui| {
        ui.label(RichText::new(tl("Ouvre ton travail dans une autre app, en un clic")).size(11.5).color(FAINT));
        for (id, what) in targets {
            let r = ui
                .horizontal(|ui| {
                    crate::app_icon(ui, 18.0, id, INK);
                    ui.button(format!("{}  ·  {what}", name(id)))
                })
                .inner;
            if r.clicked() {
                pick = Some(*id);
                ui.close();
            }
        }
    });
    pick
}

fn toast_id() -> egui::Id {
    egui::Id::NULL.with("handoff-toast")
}

pub fn toast(ctx: &Context, text: String, ok: bool) {
    let until = ctx.input(|i| i.time) + 3.5;
    ctx.data_mut(|d| d.insert_temp(toast_id(), (text, ok, until)));
    ctx.request_repaint();
}

pub fn send(ctx: &Context, target: &str, file: &Path) {
    match studio_core::handoff::send(target, file) {
        Ok(m) => toast(ctx, m, true),
        Err(e) => toast(ctx, format!("Envoi impossible : {e}"), false),
    }
}

pub struct Receiver(Inbox);

impl Receiver {
    pub fn new(id: &'static str) -> Self {
        Receiver(Inbox::new(id))
    }

    pub fn poll(&mut self, ctx: &Context) -> Vec<PathBuf> {
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        let got = self.0.poll();
        if let Some(p) = got.last() {
            let from = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            toast(ctx, format!("Reçu : {from}"), true);
        }
        let shown: Option<(String, bool, f64)> = ctx.data(|d| d.get_temp(toast_id()));
        if let Some((text, ok, until)) = shown {
            if ctx.input(|i| i.time) > until {
                ctx.data_mut(|d| d.remove::<(String, bool, f64)>(toast_id()));
            } else {
                egui::Area::new(toast_id().with("area")).order(egui::Order::Tooltip).anchor(Align2::RIGHT_BOTTOM, [-18.0, -40.0]).interactable(false).show(ctx, |ui| {
                    Frame::new().fill(CHROME).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(12)).inner_margin(Margin::symmetric(14, 9)).show(ui, |ui| {
                        ui.label(RichText::new(format!("{}  {text}", if ok { "✓" } else { "✕" })).size(13.0).color(if ok { GREEN } else { RED }));
                    });
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
        }
        got
    }
}
