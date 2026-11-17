use crate::pro::{FAINT, INK_STRONG, MUTED};
use crate::i18n::tl;
use egui::{Align, Layout, RichText, Ui};
use studio_fx::stack::{self, Fx};

pub fn effects_panel(ui: &mut Ui, id: &str, list: &mut Vec<Fx>) -> bool {
    let before = list.clone();
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.label(RichText::new(format!("{} effets, empilés dans l'ordre, chacun avec son intensité", stack::count())).size(11.5).color(FAINT));
    ui.menu_button(RichText::new(tl("+ Ajouter un effet")).size(12.5), |ui| {
        for (g, kinds) in stack::groups() {
            ui.menu_button(g, |ui| {
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    for k in kinds {
                        if ui.button(k.title()).clicked() {
                            list.push(Fx::new(*k));
                            ui.close();
                        }
                    }
                });
            });
        }
    });
    let (mut remove, mut up) = (None, None);
    for (i, c) in list.iter_mut().enumerate() {
        let Some(k) = stack::kind(&c.kind) else { continue };
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut c.on, "");
            ui.label(RichText::new(k.title()).size(12.5).color(INK_STRONG));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button("×").on_hover_text(tl("Retirer")).clicked() {
                    remove = Some(i);
                }
                if i > 0 && ui.small_button("▲").on_hover_text(tl("Monter")).clicked() {
                    up = Some(i);
                }
            });
        });
        ui.add(egui::Slider::new(&mut c.amount, 0.0..=100.0).suffix(" %").text("Intensité"));
        egui::CollapsingHeader::new(RichText::new(tl("Réglages")).size(12.0).color(MUTED)).id_salt((id, "fx", i, &c.kind)).show(ui, |ui| {
            if !k.presets().is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for (name, vals) in k.presets() {
                        if ui.small_button(*name).clicked() {
                            c.vals = vals.to_vec();
                        }
                    }
                });
            }
            c.vals.resize(k.params().len(), 0.0);
            for (pi, prm) in k.params().iter().enumerate() {
                let v = &mut c.vals[pi];
                if !prm.choices.is_empty() {
                    let mut idx = (v.round().max(0.0) as usize).min(prm.choices.len() - 1);
                    egui::ComboBox::from_id_salt((id, "fx-c", i, pi)).width(190.0).selected_text(format!("{} : {}", prm.label, prm.choices[idx])).show_ui(ui, |ui| {
                        for (j, ch) in prm.choices.iter().enumerate() {
                            ui.selectable_value(&mut idx, j, *ch);
                        }
                    });
                    *v = idx as f32;
                } else if prm.is_toggle() {
                    let mut b = *v >= 0.5;
                    ui.checkbox(&mut b, prm.label);
                    *v = b as u8 as f32;
                } else {
                    ui.add(egui::Slider::new(v, prm.min..=prm.max).text(prm.label).suffix(prm.suffix));
                }
            }
            if ui.small_button(tl("Valeurs par défaut")).clicked() {
                c.vals = k.defaults();
            }
        });
    }
    if let Some(i) = remove {
        list.remove(i);
    }
    if let Some(i) = up {
        list.swap(i, i - 1);
    }
    *list != before
}
