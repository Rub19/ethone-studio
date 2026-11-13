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

/// Vertical two-colour fill, used to give buttons a soft top light like macOS controls.
pub fn gradient_rect(p: &egui::Painter, r: Rect, radius: u8, top: Color32, bottom: Color32) {
    let shape = egui::epaint::RectShape::filled(r, CornerRadius::same(radius), bottom);
    p.add(shape);
    let inner = Rect::from_min_max(r.min, pos2(r.max.x, r.center().y));
    let mut mesh = egui::Mesh::default();
    let i = 0u32;
    let k = radius as f32 * 0.6;
    let inner = inner.shrink2(vec2(k, 0.0));
    for (pos, c) in [(inner.left_top(), top), (inner.right_top(), top), (inner.right_bottom(), Color32::TRANSPARENT), (inner.left_bottom(), Color32::TRANSPARENT)] {
        mesh.colored_vertex(pos, c);
    }
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
    p.add(mesh);
}

fn button_base(ui: &mut Ui, label: &str, icon: Option<&dyn Fn(&egui::Painter, Rect, Color32)>, fill: Color32, ink: Color32, border: Option<Color32>) -> Response {
    let font = semibold(12.5);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, ink);
    let icon_w = if icon.is_some() { 20.0 } else { 0.0 };
    let pad = if label.is_empty() { 7.0 } else { 13.0 };
    let w = (galley.size().x + icon_w + pad * 2.0 - if icon.is_some() && !label.is_empty() { 2.0 } else { 0.0 }).max(28.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
    let pressed = resp.is_pointer_button_down_on();
    let base = if pressed { fill.lerp_to_gamma(Color32::BLACK, 0.12) } else if resp.hovered() { fill.lerp_to_gamma(Color32::WHITE, 0.08) } else { fill };
    let p = ui.painter();
    gradient_rect(p, rect, 7, Color32::from_white_alpha(if pressed { 0 } else { 14 }), base);
    if let Some(b) = border {
        p.rect_stroke(rect, CornerRadius::same(7), Stroke::new(1.0, b), egui::StrokeKind::Inside);
    }
    let mut x = rect.left() + pad;
    if let Some(paint) = icon {
        let ir = Rect::from_center_size(pos2(x + 8.0, rect.center().y), Vec2::splat(15.0));
        paint(p, ir, ink);
        x += icon_w;
    }
    if !label.is_empty() {
        p.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, ink);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn pill(ui: &mut Ui, label: &str, fill: Color32, ink: Color32) -> Response {
    button_base(ui, label, None, fill, ink, None)
}

pub fn primary(ui: &mut Ui, label: &str) -> Response {
    button_base(ui, label, None, BLUE, Color32::WHITE, Some(Color32::from_white_alpha(18)))
}

pub fn secondary(ui: &mut Ui, label: &str) -> Response {
    button_base(ui, label, None, FIELD, INK_STRONG, Some(Color32::from_white_alpha(10)))
}

/// Primary button with a leading glyph (e.g. the export arrow).
pub fn primary_icon(ui: &mut Ui, label: &str, paint: impl Fn(&egui::Painter, Rect, Color32)) -> Response {
    button_base(ui, label, Some(&paint), BLUE, Color32::WHITE, Some(Color32::from_white_alpha(18)))
}

/// Secondary button with a leading glyph; an empty label gives a square icon button with a frame.
pub fn secondary_icon(ui: &mut Ui, label: &str, paint: impl Fn(&egui::Painter, Rect, Color32)) -> Response {
    button_base(ui, label, Some(&paint), FIELD, INK_STRONG, Some(Color32::from_white_alpha(10)))
}

/// Frameless text button that only shows a background on hover.
pub fn ghost(ui: &mut Ui, label: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(12.5), INK);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 18.0, 26.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), Color32::from_white_alpha(12));
    }
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, if resp.hovered() { INK_STRONG } else { INK });
    resp
}

/// Small rounded info chip, clickable (e.g. "1920 × 1080 · 30 i/s").
pub fn chip(ui: &mut Ui, label: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(12.0), MUTED);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 18.0, 24.0), Sense::click());
    ui.painter().rect(rect, CornerRadius::same(6), if resp.hovered() { FIELD_HOVER } else { FIELD }, Stroke::new(1.0, Color32::from_white_alpha(8)), egui::StrokeKind::Inside);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, if resp.hovered() { INK } else { MUTED });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Keyboard shortcut badge.
pub fn kbd(ui: &mut Ui, key: &str) {
    let galley = ui.painter().layout_no_wrap(key.to_string(), FontId::proportional(10.5), MUTED);
    let (rect, _) = ui.allocate_exact_size(vec2(galley.size().x + 10.0, 18.0), Sense::hover());
    ui.painter().rect(rect, CornerRadius::same(4), WELL, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, MUTED);
}

/// Thin vertical divider for toolbars.
pub fn toolbar_sep(ui: &mut Ui) {
    ui.add_space(4.0);
    let (r, _) = ui.allocate_exact_size(vec2(1.0, 20.0), Sense::hover());
    ui.painter().rect_filled(r, CornerRadius::ZERO, LINE);
    ui.add_space(4.0);
}

/// iOS-style segmented control; returns true when the selection changed.
pub fn segmented(ui: &mut Ui, value: &mut usize, items: &[&str]) -> bool {
    let font = FontId::proportional(12.0);
    let widths: Vec<f32> = items.iter().map(|t| ui.painter().layout_no_wrap(t.to_string(), font.clone(), INK).size().x + 20.0).collect();
    let total: f32 = widths.iter().sum::<f32>() + 4.0;
    let (outer, _) = ui.allocate_exact_size(vec2(total, 26.0), Sense::hover());
    ui.painter().rect_filled(outer, CornerRadius::same(7), WELL);
    let mut x = outer.left() + 2.0;
    let mut changed = false;
    for (k, (label, w)) in items.iter().zip(&widths).enumerate() {
        let r = Rect::from_min_size(pos2(x, outer.top() + 2.0), vec2(*w, 22.0));
        let resp = ui.interact(r, ui.make_persistent_id(("seg", items.len(), label, k)), Sense::click());
        let on = *value == k;
        if on {
            ui.painter().rect(r, CornerRadius::same(5), FIELD_HOVER, Stroke::new(1.0, Color32::from_white_alpha(10)), egui::StrokeKind::Inside);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(5), Color32::from_white_alpha(6));
        }
        ui.painter().text(r.center(), Align2::CENTER_CENTER, *label, if on { semibold(12.0) } else { font.clone() }, if on { INK_STRONG } else { MUTED });
        if resp.clicked() && !on {
            *value = k;
            changed = true;
        }
        x += w;
    }
    changed
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

/// A tool slot holding related tools, like Adobe's grouped tools: click picks the tool shown,
/// right-click, a long press or the corner triangle opens the flyout. Returns true when the tool changed.
pub fn tool_group<T: Copy + PartialEq>(ui: &mut Ui, size: f32, current: &mut T, tools: &[(T, &str, &str)], paint: impl Fn(&egui::Painter, Rect, Color32, T)) -> bool {
    if tools.is_empty() {
        return false;
    }
    let id = ui.make_persistent_id(("tool-group", tools[0].1));
    let mem = id.with("shown");
    let open_id = id.with("open");
    let mut shown = ui.ctx().data_mut(|d| *d.get_persisted_mut_or(mem, 0usize)).min(tools.len() - 1);
    if let Some(k) = tools.iter().position(|t| t.0 == *current) {
        shown = k;
    }
    let active = tools.iter().any(|t| t.0 == *current);
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
    if active {
        ui.painter().rect_filled(rect, CornerRadius::same(8), BLUE.gamma_multiply(0.28));
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), Color32::from_white_alpha(14));
    }
    let ink = if active { INK_STRONG } else { INK };
    paint(ui.painter(), rect.shrink(size * 0.27), ink, tools[shown].0);
    let multi = tools.len() > 1;
    let corner = Rect::from_min_max(rect.right_bottom() - vec2(size * 0.32, size * 0.32), rect.right_bottom());
    if multi {
        let c = rect.right_bottom() - vec2(5.0, 5.0);
        ui.painter().add(egui::Shape::convex_polygon(vec![c + vec2(2.5, -3.5), c + vec2(2.5, 2.5), c + vec2(-3.5, 2.5)], if active { INK_STRONG } else { MUTED }, Stroke::NONE));
    }
    let mut open = ui.ctx().data_mut(|d| *d.get_temp_mut_or(open_id, false));
    let mut changed = false;
    let long_press = resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.press_start_time().is_some_and(|t| i.time - t > 0.35));
    if multi && (resp.secondary_clicked() || long_press || (resp.clicked() && resp.interact_pointer_pos().is_some_and(|p| corner.contains(p)))) {
        open = true;
    } else if resp.clicked() && *current != tools[shown].0 {
        *current = tools[shown].0;
        changed = true;
    }
    let (name, key) = (tools[shown].1, tools[shown].2);
    let tip = if key.is_empty() { name.to_string() } else { format!("{name} ({key})") };
    resp.on_hover_text(if multi { format!("{tip}\nClic droit ou clic maintenu : {} autres outils", tools.len() - 1) } else { tip });
    if open {
        let area = egui::Area::new(id.with("flyout")).order(egui::Order::Foreground).fixed_pos(rect.right_top() + vec2(6.0, 0.0)).show(ui.ctx(), |ui| {
            Frame::new().fill(CHROME).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(10)).shadow(Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(150) }).inner_margin(Margin::same(6)).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (k, (t, n, s)) in tools.iter().enumerate() {
                    let (r, row) = ui.allocate_exact_size(vec2(230.0, 30.0), Sense::click());
                    let on = *t == *current;
                    if on {
                        ui.painter().rect_filled(r, CornerRadius::same(7), BLUE.gamma_multiply(0.3));
                    } else if row.hovered() {
                        ui.painter().rect_filled(r, CornerRadius::same(7), Color32::from_white_alpha(12));
                    }
                    if on {
                        ui.painter().circle_filled(r.left_center() + vec2(7.0, 0.0), 2.5, INK_STRONG);
                    }
                    paint(ui.painter(), Rect::from_center_size(r.left_center() + vec2(24.0, 0.0), Vec2::splat(15.0)), if on { INK_STRONG } else { INK }, *t);
                    ui.painter().text(r.left_center() + vec2(42.0, 0.0), Align2::LEFT_CENTER, *n, FontId::proportional(13.0), if on { INK_STRONG } else { INK });
                    if !s.is_empty() {
                        ui.painter().text(r.right_center() - vec2(8.0, 0.0), Align2::RIGHT_CENTER, *s, FontId::proportional(11.5), FAINT);
                    }
                    if row.clicked() {
                        *current = *t;
                        shown = k;
                        changed = true;
                        open = false;
                    }
                }
            });
        });
        let outside = ui.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !area.response.rect.contains(p) && !rect.contains(p)));
        if outside || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = false;
        }
    }
    ui.ctx().data_mut(|d| {
        d.insert_persisted(mem, shown);
        d.insert_temp(open_id, open);
    });
    changed
}
