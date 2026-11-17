//! Movable panels for the side columns: drag a panel header to move it, right-click to hide it,
//! and save named workspaces, as in Photoshop. Saved per app in `<data>/<app>/workspace.json`.
//!
//! Apps keep drawing their panels in code order between [`begin`] and [`end`]; each panel is
//! *placed* at the slot the user chose, using the heights measured on the previous frame.

use egui::{Color32, Context, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2};
use crate::i18n::tl;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub struct Layout {
    pub order: Vec<String>,
    pub hidden: BTreeSet<String>,
    pub closed: BTreeSet<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct Store {
    current: Layout,
    saved: BTreeMap<String, Layout>,
}

#[derive(Default)]
struct State {
    app: String,
    store: Store,
    titles: BTreeMap<String, String>,
    heights: HashMap<Id, HashMap<String, f32>>,
    dragging: Option<String>,
    naming: Option<String>,
}

/// One column during a frame.
#[derive(Clone)]
struct Column {
    key: Id,
    left: f32,
    top: f32,
    width: f32,
    ready: bool,
    drawn: Vec<(String, Rect)>,
}

fn state_id() -> Id {
    Id::NULL.with("ethone-dock-state")
}

fn column_id() -> Id {
    Id::NULL.with("ethone-dock-column")
}

fn path(app: &str) -> std::path::PathBuf {
    studio_core::data_dir().join(app).join("workspace.json")
}

/// Runs `f` on the shared dock state. The handle is cloned out first so `f` may call back into egui.
fn with_state<R>(ctx: &Context, f: impl FnOnce(&mut State) -> R) -> R {
    let shared = ctx.data_mut(|d| d.get_temp_mut_or_default::<std::sync::Arc<std::sync::Mutex<State>>>(state_id()).clone());
    let mut st = shared.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut st)
}

fn save(st: &State) {
    if st.app.is_empty() || st.app.starts_with("test-") {
        return;
    }
    let p = path(&st.app);
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(b) = serde_json::to_vec_pretty(&st.store) {
        let _ = std::fs::write(p, b);
    }
}

fn load(st: &mut State, app: &str) {
    if st.app == app {
        return;
    }
    st.app = app.to_string();
    st.store = std::fs::read(path(app)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    st.heights.clear();
}

/// Starts a column of movable panels. Everything drawn until [`end`] should be [`panel`]s.
pub fn begin(ui: &mut Ui, app: &str) {
    with_state(ui.ctx(), |st| load(st, app));
    let r = ui.available_rect_before_wrap();
    let key = ui.scope_id();
    let ready = with_state(ui.ctx(), |st| st.heights.get(&key).is_some_and(|h| !h.is_empty()));
    let col = Column { key, left: r.left(), top: r.top(), width: r.width(), ready, drawn: Vec::new() };
    ui.ctx().data_mut(|d| d.insert_temp(column_id(), Some(col)));
}

/// Ends the column: reserves its height, remembers panel heights and finishes a drag.
pub fn end(ui: &mut Ui) {
    let Some(col) = ui.ctx().data_mut(|d| d.get_temp::<Option<Column>>(column_id()).flatten()) else { return };
    ui.ctx().data_mut(|d| d.insert_temp::<Option<Column>>(column_id(), None));
    let bottom = col.drawn.iter().map(|(_, r)| r.bottom()).fold(col.top, f32::max);
    ui.allocate_rect(Rect::from_min_max(pos2(col.left, col.top), pos2(col.left + col.width, bottom)), Sense::hover());
    let pointer = ui.ctx().input(|i| (i.pointer.hover_pos(), i.pointer.any_released()));
    let ctx = ui.ctx().clone();
    with_state(&ctx, |st| {
        let old = st.heights.entry(col.key).or_default();
        let changed = old.len() != col.drawn.len() || col.drawn.iter().any(|(id, r)| old.get(id).is_none_or(|h| (h - r.height()).abs() > 0.5));
        *old = col.drawn.iter().map(|(id, r)| (id.clone(), r.height())).collect();
        if changed {
            ctx.request_repaint();
        }
        let Some(dragged) = st.dragging.clone() else { return };
        let (Some(p), released) = pointer else { return };
        let slot = col.drawn.iter().filter(|(id, _)| *id != dragged).take_while(|(_, r)| p.y > r.center().y).count();
        if let Some(target) = col.drawn.iter().filter(|(id, _)| *id != dragged).nth(slot).map(|(_, r)| r.top()).or_else(|| col.drawn.last().map(|(_, r)| r.bottom())) {
            ui.painter().hline(col.left..=col.left + col.width, target, Stroke::new(2.0, crate::pro::BLUE));
        }
        if released {
            let visible: Vec<String> = col.drawn.iter().map(|(id, _)| id.clone()).filter(|id| *id != dragged).collect();
            let anchor = visible.get(slot).cloned();
            let order = &mut st.store.current.order;
            order.retain(|o| *o != dragged);
            let at = anchor.and_then(|a| order.iter().position(|o| *o == a)).unwrap_or(order.len());
            order.insert(at, dragged);
            st.dragging = None;
            save(st);
        }
    });
}

/// A collapsible panel. Inside a column it can be moved, hidden and restored; outside one it
/// simply flows like any other widget.
pub fn panel(ui: &mut Ui, id: &str, title: &str, add: impl FnOnce(&mut Ui)) {
    let ctx = ui.ctx().clone();
    let col = ctx.data(|d| d.get_temp::<Option<Column>>(column_id()).flatten());
    let Some(mut col) = col else {
        draw(ui, id, title, false, add);
        return;
    };
    let (hidden, y) = with_state(&ctx, |st| {
        st.titles.entry(id.to_string()).or_insert_with(|| crate::i18n::tl(title).to_string());
        if !st.store.current.order.iter().any(|o| o == id) {
            st.store.current.order.push(id.to_string());
        }
        let hidden = st.store.current.hidden.contains(id);
        let before: f32 = st.store.current.order.iter().take_while(|o| *o != id).filter(|o| !st.store.current.hidden.contains(*o)).filter_map(|o| st.heights.get(&col.key).and_then(|h| h.get(o))).sum();
        (hidden, col.top + before)
    });
    if hidden {
        return;
    }
    let y = if col.ready { y } else { col.drawn.last().map_or(col.top, |(_, r)| r.bottom()) };
    let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(pos2(col.left, y), vec2(col.width, f32::INFINITY))).id_salt(("dock-panel", id)));
    draw(&mut child, id, title, true, add);
    let r = child.min_rect();
    col.drawn.push((id.to_string(), Rect::from_min_max(pos2(col.left, r.top()), pos2(col.left + col.width, r.bottom()))));
    col.drawn.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
    ctx.data_mut(|d| d.insert_temp(column_id(), Some(col)));
}

fn draw(ui: &mut Ui, id: &str, title: &str, movable: bool, add: impl FnOnce(&mut Ui)) {
    use crate::pro::{FAINT, INK, LINE, PANEL_HEAD, semibold};
    let ctx = ui.ctx().clone();
    let open = !with_state(&ctx, |st| st.store.current.closed.contains(id));
    let sense = if movable { Sense::click_and_drag() } else { Sense::click() };
    let (head, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), sense);
    let dragging = with_state(&ctx, |st| st.dragging.as_deref() == Some(id));
    ui.painter().rect_filled(head, CornerRadius::ZERO, if dragging { Color32::from_rgb(0x2a, 0x2a, 0x2a) } else { PANEL_HEAD });
    let c = pos2(head.left() + 11.0, head.center().y);
    let pts = if open { vec![c + vec2(-3.0, -1.5), c + vec2(3.0, -1.5), c + vec2(0.0, 2.5)] } else { vec![c + vec2(-1.5, -3.0), c + vec2(2.5, 0.0), c + vec2(-1.5, 3.0)] };
    ui.painter().add(egui::Shape::convex_polygon(pts, FAINT, Stroke::NONE));
    ui.painter().text(pos2(head.left() + 20.0, head.center().y), egui::Align2::LEFT_CENTER, crate::i18n::tl(title), semibold(11.5), INK);
    if movable && resp.hovered() {
        grip(ui, Pos2::new(head.right() - 14.0, head.center().y));
    }
    let resp = resp.on_hover_cursor(if movable { egui::CursorIcon::Grab } else { egui::CursorIcon::PointingHand });
    if movable && resp.drag_started() {
        with_state(&ctx, |st| st.dragging = Some(id.to_string()));
    }
    if resp.clicked() {
        with_state(&ctx, |st| {
            let closed = &mut st.store.current.closed;
            if !closed.remove(id) {
                closed.insert(id.to_string());
            }
            save(st);
        });
    }
    if movable {
        resp.context_menu(|ui| {
            if ui.button(tl("Masquer ce panneau")).clicked() {
                with_state(&ctx, |st| {
                    st.store.current.hidden.insert(id.to_string());
                    save(st);
                });
                ui.close();
            }
            if ui.button(tl("Réinitialiser l'espace de travail")).clicked() {
                with_state(&ctx, |st| {
                    st.store.current = Layout::default();
                    save(st);
                });
                ui.close();
            }
        });
    }
    if open {
        let inner = ui.available_width() - 20.0;
        egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 10, top: 8, bottom: 10 }).show(ui, |ui| {
            ui.set_width(inner);
            ui.set_max_width(inner);
            add(ui);
        });
    }
    let line = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover()).0;
    ui.painter().rect_filled(line, CornerRadius::ZERO, LINE);
}

fn grip(ui: &Ui, c: Pos2) {
    for dx in [-2.5, 2.5] {
        for dy in [-3.5, 0.0, 3.5] {
            ui.painter().circle_filled(c + vec2(dx, dy), 1.0, crate::pro::FAINT);
        }
    }
}

/// The « Espace de travail » menu: show hidden panels, save and load named workspaces, reset.
pub fn menu(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let (titles, hidden, saved) = with_state(&ctx, |st| (st.titles.clone(), st.store.current.hidden.clone(), st.store.saved.keys().cloned().collect::<Vec<_>>()));
    if titles.is_empty() {
        ui.label(egui::RichText::new(tl("Aucun panneau déplaçable ici")).color(crate::pro::FAINT));
        return;
    }
    ui.menu_button(tl("Panneaux"), |ui| {
        for (id, title) in &titles {
            let mut on = !hidden.contains(id);
            if ui.checkbox(&mut on, title).changed() {
                with_state(&ctx, |st| {
                    if on {
                        st.store.current.hidden.remove(id);
                    } else {
                        st.store.current.hidden.insert(id.clone());
                    }
                    save(st);
                });
            }
        }
    });
    if !saved.is_empty() {
        ui.separator();
        for name in &saved {
            if ui.button(name).clicked() {
                with_state(&ctx, |st| {
                    if let Some(l) = st.store.saved.get(name).cloned() {
                        st.store.current = l;
                        save(st);
                    }
                });
                ui.close();
            }
        }
    }
    ui.separator();
    let mut naming = with_state(&ctx, |st| st.naming.clone());
    match &mut naming {
        Some(name) => {
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(name).hint_text(tl("Nom de l'espace")).desired_width(140.0));
                r.request_focus();
                let ok = ui.button(tl("Enregistrer")).clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                if ok && !name.trim().is_empty() {
                    let n = name.trim().to_string();
                    with_state(&ctx, |st| {
                        let cur = st.store.current.clone();
                        st.store.saved.insert(n, cur);
                        st.naming = None;
                        save(st);
                    });
                    ui.close();
                }
            });
        }
        None => {
            if ui.button(tl("Enregistrer l'espace de travail…")).clicked() {
                naming = Some(String::new());
            }
        }
    }
    if naming.as_ref().is_some_and(|n| n.is_empty()) {
        with_state(&ctx, |st| st.naming = naming.clone());
    } else if let Some(n) = naming.filter(|n| !n.is_empty()) {
        with_state(&ctx, |st| {
            if st.naming.is_some() {
                st.naming = Some(n);
            }
        });
    }
    if ui.button(tl("Réinitialiser l'espace de travail")).clicked() {
        with_state(&ctx, |st| {
            st.store.current = Layout::default();
            save(st);
        });
        ui.close();
    }
}

#[cfg(test)]
pub(crate) fn set_order(ctx: &Context, order: &[&str]) {
    with_state(ctx, |st| st.store.current.order = order.iter().map(|s| s.to_string()).collect());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &Context, rects: &mut Vec<(String, Rect)>) {
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(300.0, 800.0))), ..Default::default() }, |root| {
            begin(root, "test-dock");
            for id in ["a", "b", "c"] {
                panel(root, id, &id.to_uppercase(), |ui| {
                    ui.add_space(if id == "b" { 60.0 } else { 20.0 });
                });
            }
            *rects = ctx.data(|d| d.get_temp::<Option<Column>>(column_id()).flatten()).map(|c| c.drawn).unwrap_or_default();
            end(root);
        });
        out.textures_delta.clear();
    }

    fn tops(rects: &[(String, Rect)]) -> Vec<String> {
        let mut v = rects.to_vec();
        v.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
        v.into_iter().map(|(id, _)| id).collect()
    }

    #[test]
    fn panels_follow_the_saved_order_without_overlapping() {
        let ctx = Context::default();
        let mut rects = Vec::new();
        frame(&ctx, &mut rects);
        frame(&ctx, &mut rects);
        assert_eq!(tops(&rects), ["a", "b", "c"]);
        set_order(&ctx, &["c", "a", "b"]);
        frame(&ctx, &mut rects);
        assert_eq!(tops(&rects), ["c", "a", "b"]);
        let mut sorted = rects.clone();
        sorted.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
        for w in sorted.windows(2) {
            assert!(w[0].1.bottom() <= w[1].1.top() + 0.5, "{} overlaps {}", w[0].0, w[1].0);
        }
    }

    #[test]
    fn hidden_panels_are_skipped() {
        let ctx = Context::default();
        let mut rects = Vec::new();
        frame(&ctx, &mut rects);
        with_state(&ctx, |st| {
            st.store.current.hidden.insert("b".into());
        });
        frame(&ctx, &mut rects);
        frame(&ctx, &mut rects);
        assert_eq!(tops(&rects), ["a", "c"]);
    }
}
