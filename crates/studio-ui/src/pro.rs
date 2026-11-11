use egui::{
    Align2, Color32, Context, CornerRadius, FontId, Frame, Margin, Rect, Response, RichText, Sense,
    Shadow, Stroke, Ui, Vec2, pos2, vec2,
};

pub const CHROME: Color32 = Color32::from_rgb(0x24, 0x24, 0x28);
pub const PANEL: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x23);
pub const PANEL_HEAD: Color32 = Color32::from_rgb(0x27, 0x27, 0x2c);
pub const PASTEBOARD: Color32 = Color32::from_rgb(0x17, 0x17, 0x1a);
pub const WELL: Color32 = Color32::from_rgb(0x16, 0x16, 0x19);
pub const FIELD: Color32 = Color32::from_rgb(0x2c, 0x2c, 0x31);
pub const FIELD_HOVER: Color32 = Color32::from_rgb(0x36, 0x36, 0x3c);
pub const LINE: Color32 = Color32::from_rgb(0x33, 0x33, 0x39);
pub const INK: Color32 = Color32::from_rgb(0xe6, 0xe6, 0xeb);
pub const INK_STRONG: Color32 = Color32::WHITE;
pub const MUTED: Color32 = Color32::from_rgb(0x9a, 0x9a, 0xa3);
pub const FAINT: Color32 = Color32::from_rgb(0x6e, 0x6e, 0x77);
pub const BLUE: Color32 = Color32::from_rgb(0x0a, 0x84, 0xff);
pub const GREEN: Color32 = Color32::from_rgb(0x30, 0xd1, 0x58);
pub const RED: Color32 = Color32::from_rgb(0xff, 0x45, 0x3a);

pub fn semibold(size: f32) -> FontId {
    crate::semibold(size)
}

pub fn apply(ctx: &Context) {
    crate::apply_style(ctx);
    ctx.all_styles_mut(|s| {
        let v = &mut s.visuals;
        v.panel_fill = PANEL;
        v.window_fill = CHROME;
        v.window_stroke = Stroke::new(1.0, LINE);
        v.window_corner_radius = CornerRadius::same(14);
        v.menu_corner_radius = CornerRadius::same(10);
        v.popup_shadow = Shadow { offset: [0, 10], blur: 28, spread: 0, color: Color32::from_black_alpha(140) };
        v.window_shadow = Shadow { offset: [0, 16], blur: 44, spread: 0, color: Color32::from_black_alpha(150) };
        v.extreme_bg_color = WELL;
        v.faint_bg_color = FIELD;
        v.selection.bg_fill = BLUE;
        v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
        v.override_text_color = Some(INK);
        v.slider_trailing_fill = true;
        for (w, fill) in [
            (&mut v.widgets.noninteractive, PANEL),
            (&mut v.widgets.inactive, FIELD),
            (&mut v.widgets.hovered, FIELD_HOVER),
            (&mut v.widgets.active, FIELD_HOVER),
            (&mut v.widgets.open, FIELD_HOVER),
        ] {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.corner_radius = CornerRadius::same(7);
            w.bg_stroke = Stroke::NONE;
            w.expansion = 0.0;
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, INK);
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, INK_STRONG);
        v.widgets.active.fg_stroke = Stroke::new(1.0, INK_STRONG);
        s.spacing.button_padding = vec2(12.0, 6.0);
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.interact_size.y = 26.0;
        s.spacing.slider_width = 120.0;
        s.spacing.menu_margin = Margin::same(6);
    });
}

pub fn pill(ui: &mut Ui, label: &str, fill: Color32, ink: Color32) -> Response {
    let galley = ui.painter().layout_no_wrap(label.to_string(), semibold(13.5), ink);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 32.0, 34.0), Sense::click());
    let fill = if resp.hovered() { fill.lerp_to_gamma(Color32::WHITE, 0.12) } else { fill };
    ui.painter().rect_filled(rect, CornerRadius::same(17), fill);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, ink);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn primary(ui: &mut Ui, label: &str) -> Response {
    pill(ui, label, BLUE, Color32::WHITE)
}

pub fn secondary(ui: &mut Ui, label: &str) -> Response {
    pill(ui, label, FIELD, INK_STRONG)
}

/// A square icon button drawn by `paint`, highlighted when `on`.
pub fn icon_button(ui: &mut Ui, size: f32, on: bool, tip: &str, paint: impl FnOnce(&egui::Painter, Rect, Color32)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if on {
        ui.painter().rect_filled(rect, CornerRadius::same(8), BLUE.gamma_multiply(0.28));
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), Color32::from_white_alpha(14));
    }
    paint(ui.painter(), rect.shrink(size * 0.27), if on { INK_STRONG } else { INK });
    resp.on_hover_text(tip)
}

/// A collapsible dock panel with a header; returns whether it is open.
pub fn dock_panel(ui: &mut Ui, id: &str, title: &str, add: impl FnOnce(&mut Ui)) {
    let key = ui.make_persistent_id(("dock", id));
    let mut open = ui.ctx().data_mut(|d| *d.get_persisted_mut_or(key, true));
    let (head, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    ui.painter().rect_filled(head, CornerRadius::ZERO, PANEL_HEAD);
    let c = pos2(head.left() + 14.0, head.center().y);
    let pts = if open {
        vec![c + vec2(-4.0, -2.0), c + vec2(4.0, -2.0), c + vec2(0.0, 3.0)]
    } else {
        vec![c + vec2(-2.0, -4.0), c + vec2(3.0, 0.0), c + vec2(-2.0, 4.0)]
    };
    ui.painter().add(egui::Shape::convex_polygon(pts, MUTED, Stroke::NONE));
    ui.painter().text(pos2(head.left() + 26.0, head.center().y), Align2::LEFT_CENTER, title, semibold(12.5), INK);
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        open = !open;
        ui.ctx().data_mut(|d| d.insert_persisted(key, open));
    }
    if open {
        let inner = ui.available_width() - 24.0;
        Frame::new().inner_margin(Margin { left: 12, right: 12, top: 10, bottom: 12 }).show(ui, |ui| {
            ui.set_width(inner);
            ui.set_max_width(inner);
            add(ui);
        });
    }
    let line = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover()).0;
    ui.painter().rect_filled(line, CornerRadius::ZERO, LINE);
}

/// A compact colour picker: saturation/value square, hue bar and a hex field. Returns whether it changed.
pub fn color_picker(ui: &mut Ui, color: &mut Color32, width: f32) -> bool {
    let id = ui.make_persistent_id("pro-color-hsv");
    let mut hsv = ui.ctx().data(|d| d.get_temp::<[f32; 3]>(id)).filter(|h| {
        let c: Color32 = egui::ecolor::Hsva::new(h[0], h[1], h[2], 1.0).into();
        c == *color
    }).unwrap_or_else(|| {
        let h = egui::ecolor::Hsva::from(*color);
        [h.h, h.s, h.v]
    });
    let mut changed = false;
    let (sq, resp) = ui.allocate_exact_size(vec2(width, (width * 0.62).round()), Sense::click_and_drag());
    let hue: Color32 = egui::ecolor::Hsva::new(hsv[0], 1.0, 1.0, 1.0).into();
    let p = ui.painter();
    let mut mesh = egui::Mesh::default();
    let quad = |mesh: &mut egui::Mesh, r: Rect, tl: Color32, tr: Color32, br: Color32, bl: Color32| {
        let i = mesh.vertices.len() as u32;
        for (pos, c) in [(r.left_top(), tl), (r.right_top(), tr), (r.right_bottom(), br), (r.left_bottom(), bl)] {
            mesh.colored_vertex(pos, c);
        }
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    };
    quad(&mut mesh, sq, Color32::WHITE, hue, hue, Color32::WHITE);
    quad(&mut mesh, sq, Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::BLACK, Color32::BLACK);
    p.add(mesh);
    p.rect_stroke(sq, CornerRadius::same(6), Stroke::new(1.0, LINE), egui::StrokeKind::Outside);
    if let Some(pos) = resp.interact_pointer_pos() {
        hsv[1] = ((pos.x - sq.left()) / sq.width()).clamp(0.0, 1.0);
        hsv[2] = 1.0 - ((pos.y - sq.top()) / sq.height()).clamp(0.0, 1.0);
        changed = true;
    }
    let knob = pos2(sq.left() + hsv[1] * sq.width(), sq.top() + (1.0 - hsv[2]) * sq.height());
    p.circle_stroke(knob, 7.0, Stroke::new(2.5, Color32::WHITE));
    p.circle_stroke(knob, 8.5, Stroke::new(1.0, Color32::from_black_alpha(120)));

    ui.add_space(8.0);
    let (bar, bresp) = ui.allocate_exact_size(vec2(width, 14.0), Sense::click_and_drag());
    let mut mesh = egui::Mesh::default();
    for k in 0..6 {
        let a = egui::ecolor::Hsva::new(k as f32 / 6.0, 1.0, 1.0, 1.0).into();
        let b = egui::ecolor::Hsva::new((k + 1) as f32 / 6.0, 1.0, 1.0, 1.0).into();
        let r = Rect::from_min_max(pos2(bar.left() + bar.width() * k as f32 / 6.0, bar.top()), pos2(bar.left() + bar.width() * (k + 1) as f32 / 6.0, bar.bottom()));
        quad(&mut mesh, r, a, b, b, a);
    }
    ui.painter().add(mesh);
    if let Some(pos) = bresp.interact_pointer_pos() {
        hsv[0] = ((pos.x - bar.left()) / bar.width()).clamp(0.0, 0.9999);
        changed = true;
    }
    let hx = bar.left() + hsv[0] * bar.width();
    ui.painter().circle(pos2(hx, bar.center().y), 8.0, hue, Stroke::new(2.5, Color32::WHITE));

    ui.add_space(8.0);
    let hex_id = ui.make_persistent_id("pro-color-hex");
    let current = format!("{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
    let mut text = ui.ctx().data(|d| d.get_temp::<(String, String)>(hex_id)).filter(|(base, _)| *base == current).map_or(current.clone(), |(_, t)| t);
    ui.horizontal(|ui| {
        let (sw, _) = ui.allocate_exact_size(vec2(28.0, 26.0), Sense::hover());
        ui.painter().rect(sw, CornerRadius::same(6), *color, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
        ui.label(RichText::new("#").color(MUTED));
        let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(90.0).char_limit(6).font(FontId::monospace(13.0)));
        if r.changed() && text.len() == 6
            && let Ok(v) = u32::from_str_radix(&text, 16)
        {
            *color = Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
            let h = egui::ecolor::Hsva::from(*color);
            hsv = [h.h, h.s, h.v];
        }
        ui.label(RichText::new(format!("R {}  V {}  B {}", color.r(), color.g(), color.b())).size(11.5).color(FAINT));
    });
    ui.ctx().data_mut(|d| d.insert_temp(hex_id, (format!("{:02X}{:02X}{:02X}", color.r(), color.g(), color.b()), text)));
    if changed {
        *color = egui::ecolor::Hsva::new(hsv[0], hsv[1], hsv[2], 1.0).into();
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, hsv));
    changed
}

pub fn label_value(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.5).color(INK));
        });
    });
}

/// A labelled compact slider row for option bars: "Taille [====] 12 px".
pub fn option_slider<N: egui::emath::Numeric>(ui: &mut Ui, label: &str, value: &mut N, range: std::ops::RangeInclusive<N>, suffix: &str, log: bool) -> Response {
    ui.label(RichText::new(label).size(12.5).color(MUTED));
    let r = ui.add(egui::Slider::new(value, range).logarithmic(log).suffix(suffix).show_value(true).trailing_fill(true));
    ui.add_space(10.0);
    r
}

/// A modal sheet: dims the window and centres a rounded card. Returns `false` when dismissed.
pub fn sheet(ctx: &Context, id: &str, width: f32, add: impl FnOnce(&mut Ui)) -> bool {
    let screen = ctx.content_rect();
    let mut keep = true;
    egui::Area::new(egui::Id::NULL.with(("scrim", id)))
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(r, CornerRadius::ZERO, Color32::from_black_alpha(150));
            if resp.clicked() {
                keep = false;
            }
        });
    egui::Area::new(egui::Id::NULL.with(("sheet", id)))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            Frame::new()
                .fill(CHROME)
                .stroke(Stroke::new(1.0, LINE))
                .corner_radius(CornerRadius::same(16))
                .shadow(Shadow { offset: [0, 18], blur: 48, spread: 0, color: Color32::from_black_alpha(160) })
                .inner_margin(Margin::same(24))
                .show(ui, |ui| {
                    ui.set_width(width);
                    add(ui);
                });
        });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        keep = false;
    }
    keep
}
