//! Integrated title bar for undecorated windows: one logo, menus, drag area and window buttons.

use egui::{Align2, Color32, Context, CornerRadius, CursorIcon, FontId, Rect, ResizeDirection, Sense, Stroke, Ui, ViewportCommand, pos2, vec2};

pub const BTN_W: f32 = 46.0;
const EDGE: f32 = 5.0;

fn maximized(ctx: &Context) -> bool {
    ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

/// Call at the start of an app's top panel. `margin` is the panel's horizontal inner margin.
/// Returns the usable rect (the panel minus the window buttons on the right).
pub fn bar(ui: &mut Ui, margin: f32) -> Rect {
    let inner = ui.max_rect();
    let outer = inner.expand2(vec2(margin, 0.0));
    let ctx = ui.ctx().clone();
    let resp = ui.interact(outer, egui::Id::NULL.with("ethone-title-drag"), Sense::click_and_drag());
    if resp.double_clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized(&ctx)));
    } else if resp.drag_started_by(egui::PointerButton::Primary) {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    let title = ctx.input(|i| i.viewport().title.clone()).unwrap_or_default();
    if outer.width() > 1180.0 && !title.is_empty() {
        ui.painter().text(outer.center(), Align2::CENTER_CENTER, title, FontId::proportional(12.0), crate::pro::FAINT);
    }
    buttons(ui, outer);
    Rect::from_min_max(inner.min, pos2(outer.right() - 3.0 * BTN_W - 8.0, inner.bottom()))
}

/// Minimise, maximise / restore and close buttons, flush with the right edge of `outer`.
pub fn buttons(ui: &mut Ui, outer: Rect) {
    let ctx = ui.ctx().clone();
    let max = maximized(&ctx);
    let ink = crate::pro::INK;
    for k in 0..3 {
        let r = Rect::from_min_max(pos2(outer.right() - BTN_W * (3 - k) as f32, outer.top()), pos2(outer.right() - BTN_W * (2 - k) as f32, outer.bottom()));
        let resp = ui.interact(r, egui::Id::NULL.with(("ethone-win-btn", k)), Sense::click());
        let hover = resp.hovered();
        if hover {
            ui.painter().rect_filled(r, CornerRadius::ZERO, if k == 2 { Color32::from_rgb(0xc4, 0x2b, 0x1c) } else { Color32::from_white_alpha(18) });
        }
        let c = r.center();
        let s = Stroke::new(1.0, if hover && k == 2 { Color32::WHITE } else { ink });
        let p = ui.painter();
        match k {
            0 => {
                p.line_segment([c + vec2(-5.0, 0.5), c + vec2(5.0, 0.5)], s);
            }
            1 if max => {
                p.rect_stroke(Rect::from_center_size(c + vec2(-1.0, 1.0), vec2(8.0, 8.0)), CornerRadius::same(1), s, egui::StrokeKind::Middle);
                p.line_segment([c + vec2(-3.0, -3.0), c + vec2(-3.0, -5.0)], s);
                p.line_segment([c + vec2(-3.0, -5.0), c + vec2(5.0, -5.0)], s);
                p.line_segment([c + vec2(5.0, -5.0), c + vec2(5.0, 3.0)], s);
                p.line_segment([c + vec2(5.0, 3.0), c + vec2(3.0, 3.0)], s);
            }
            1 => {
                p.rect_stroke(Rect::from_center_size(c, vec2(10.0, 10.0)), CornerRadius::same(1), s, egui::StrokeKind::Middle);
            }
            _ => {
                p.line_segment([c + vec2(-5.0, -5.0), c + vec2(5.0, 5.0)], s);
                p.line_segment([c + vec2(-5.0, 5.0), c + vec2(5.0, -5.0)], s);
            }
        }
        if resp.clicked() {
            ctx.send_viewport_cmd(match k {
                0 => ViewportCommand::Minimized(true),
                1 => ViewportCommand::Maximized(!max),
                _ => ViewportCommand::Close,
            });
        }
    }
}

/// Lets the user resize an undecorated window from its edges. Call once per frame.
pub fn resize(ctx: &Context) {
    if maximized(ctx) {
        return;
    }
    let screen = ctx.content_rect();
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    let (l, r, t, b) = (p.x - screen.left() < EDGE, screen.right() - p.x < EDGE, p.y - screen.top() < EDGE, screen.bottom() - p.y < EDGE);
    let dir = match (l, r, t, b) {
        (true, _, true, _) => ResizeDirection::NorthWest,
        (_, true, true, _) => ResizeDirection::NorthEast,
        (true, _, _, true) => ResizeDirection::SouthWest,
        (_, true, _, true) => ResizeDirection::SouthEast,
        (true, ..) => ResizeDirection::West,
        (_, true, ..) => ResizeDirection::East,
        (_, _, true, _) => ResizeDirection::North,
        (.., true) => ResizeDirection::South,
        _ => return,
    };
    ctx.set_cursor_icon(match dir {
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::ResizeNwSe,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::ResizeNeSw,
        ResizeDirection::West | ResizeDirection::East => CursorIcon::ResizeHorizontal,
        _ => CursorIcon::ResizeVertical,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
    }
}
