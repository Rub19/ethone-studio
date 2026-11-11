use egui::epaint::CubicBezierShape;
use egui::{
    Color32, Context, CornerRadius, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    TextureHandle, TextureOptions, Ui, Vec2, vec2,
};

pub const BG: Color32 = Color32::from_rgb(12, 13, 17);
pub const PANEL: Color32 = Color32::from_rgb(18, 20, 26);
pub const CARD: Color32 = Color32::from_rgb(24, 27, 35);
pub const CARD_HOVER: Color32 = Color32::from_rgb(30, 34, 44);
pub const TILE: Color32 = Color32::from_rgb(16, 18, 24);
pub const TEXT: Color32 = Color32::from_rgb(232, 234, 240);
pub const MUTED: Color32 = Color32::from_rgb(140, 146, 160);
pub const BRAND: Color32 = Color32::from_rgb(124, 92, 255);
pub const DANGER: Color32 = Color32::from_rgb(255, 90, 90);
pub const OK: Color32 = Color32::from_rgb(70, 220, 160);

const ETHONE_LOGO_PNG: &[u8] = include_bytes!("../assets/ethone-logo.png");

pub fn apply_style(ctx: &Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|s| {
        s.visuals.panel_fill = PANEL;
        s.visuals.window_fill = PANEL;
        s.visuals.extreme_bg_color = BG;
        s.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(34, 38, 48);
        s.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(44, 49, 62);
        s.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(54, 60, 76);
        for w in [
            &mut s.visuals.widgets.inactive,
            &mut s.visuals.widgets.hovered,
            &mut s.visuals.widgets.active,
            &mut s.visuals.widgets.noninteractive,
        ] {
            w.corner_radius = CornerRadius::same(8);
        }
        s.visuals.selection.bg_fill = BRAND;
        s.spacing.button_padding = vec2(14.0, 7.0);
        s.spacing.item_spacing = vec2(8.0, 6.0);
    });
}

fn decode(bytes: &[u8]) -> (Vec<u8>, u32, u32) {
    let img = image::load_from_memory(bytes).expect("logo").into_rgba8();
    let (w, h) = img.dimensions();
    (img.into_raw(), w, h)
}

pub fn window_icon() -> egui::IconData {
    let (rgba, width, height) = decode(ETHONE_LOGO_PNG);
    egui::IconData { rgba, width, height }
}

pub fn logo_texture(ctx: &Context) -> TextureHandle {
    let (rgba, w, h) = decode(ETHONE_LOGO_PNG);
    let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    ctx.load_texture("ethone-logo", img, TextureOptions::LINEAR)
}

pub fn app_icon(ui: &mut Ui, size: f32, id: &str, accent: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_app_icon(ui.painter(), rect, id, accent);
}

pub fn paint_app_icon(painter: &Painter, r: Rect, id: &str, accent: Color32) {
    let s = r.width();
    let radius = CornerRadius::same((s * 0.27) as u8);
    painter.rect_filled(r, radius, TILE);
    painter.rect_filled(r, radius, accent.gamma_multiply(0.10));
    painter.rect_stroke(r, radius, Stroke::new(s / 40.0, accent.gamma_multiply(0.75)), StrokeKind::Inside);

    let p = |x: f32, y: f32| -> Pos2 { r.min + vec2(x, y) * (s / 64.0) };
    let w = s / 64.0;
    let line = Stroke::new(3.0 * w, accent);
    let thin = Stroke::new(1.5 * w, accent.gamma_multiply(0.6));

    match id {
        "pixel" => {
            for (i, o) in [(0, 0.0), (1, 7.0), (2, 14.0)] {
                let rect = Rect::from_min_size(p(14.0 + o, 28.0 - o), Vec2::splat(22.0 * w));
                let cr = CornerRadius::same((4.0 * w) as u8);
                if i == 2 {
                    painter.rect_filled(rect, cr, accent);
                } else {
                    painter.rect_filled(rect, cr, TILE);
                    painter.rect_stroke(rect, cr, Stroke::new(2.0 * w, accent.gamma_multiply(0.45 + 0.25 * i as f32)), StrokeKind::Inside);
                }
            }
        }
        "cut" => {
            painter.line_segment([p(24.0, 39.0), p(46.0, 13.0)], line);
            painter.line_segment([p(40.0, 39.0), p(18.0, 13.0)], line);
            painter.circle_stroke(p(21.0, 45.0), 6.5 * w, line);
            painter.circle_stroke(p(43.0, 45.0), 6.5 * w, line);
            painter.circle_filled(p(32.0, 26.5), 2.2 * w, TILE);
        }
        "vector" => {
            let pts = [p(12.0, 46.0), p(20.0, 12.0), p(44.0, 52.0), p(52.0, 18.0)];
            painter.line_segment([pts[0], pts[1]], thin);
            painter.line_segment([pts[3], pts[2]], thin);
            painter.add(CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, line));
            for a in [pts[0], pts[3]] {
                painter.rect_filled(Rect::from_center_size(a, Vec2::splat(7.0 * w)), CornerRadius::ZERO, accent);
            }
            for c in [pts[1], pts[2]] {
                painter.circle_filled(c, 3.0 * w, accent);
            }
        }
        "motion" => {
            painter.line_segment([p(10.0, 46.0), p(54.0, 46.0)], thin);
            for x in [18.0, 32.0, 46.0] {
                let c = p(x, 46.0);
                let d = 5.0 * w;
                painter.add(Shape::convex_polygon(
                    vec![c + vec2(0.0, -d), c + vec2(d, 0.0), c + vec2(0.0, d), c + vec2(-d, 0.0)],
                    accent,
                    Stroke::NONE,
                ));
            }
            for (y, x0) in [(16.0, 16.0), (22.0, 12.0), (28.0, 20.0)] {
                painter.line_segment([p(x0, y), p(34.0, y)], thin);
            }
            painter.circle_filled(p(44.0, 22.0), 8.0 * w, accent);
        }
        "lumen" => {
            let c = p(32.0, 32.0);
            let big = 19.0 * w;
            painter.circle_stroke(c, big, line);
            for k in 0..6 {
                let a = k as f32 * std::f32::consts::TAU / 6.0;
                let b = a + 1.9;
                painter.line_segment([c + vec2(a.cos(), a.sin()) * big, c + vec2(b.cos(), b.sin()) * 8.0 * w], Stroke::new(2.2 * w, accent));
            }
        }
        "page" => {
            let doc = Rect::from_min_max(p(17.0, 10.0), p(47.0, 54.0));
            painter.rect_stroke(doc, CornerRadius::same((4.0 * w) as u8), line, StrokeKind::Middle);
            painter.rect_filled(Rect::from_min_max(p(23.0, 17.0), p(41.0, 28.0)), CornerRadius::same((2.0 * w) as u8), accent.gamma_multiply(0.8));
            for (y, x1) in [(35.0, 41.0), (41.0, 41.0), (47.0, 34.0)] {
                painter.line_segment([p(23.0, y), p(x1, y)], Stroke::new(2.0 * w, accent.gamma_multiply(0.7)));
            }
        }
        "sound" => {
            for (i, h) in [6.0, 16.0, 30.0, 20.0, 34.0, 14.0, 22.0, 8.0].iter().enumerate() {
                let x = 13.0 + i as f32 * 5.5;
                painter.line_segment([p(x, 32.0 - h / 2.0), p(x, 32.0 + h / 2.0)], Stroke::new(3.0 * w, accent));
            }
        }
        _ => {
            painter.text(r.center(), egui::Align2::CENTER_CENTER, id.chars().next().unwrap_or('?').to_uppercase().to_string(), FontId::proportional(s * 0.45), accent);
        }
    }
}

pub fn logo_image(ui: &mut Ui, tex: &TextureHandle, size: f32) {
    ui.add(egui::Image::new((tex.id(), Vec2::splat(size))));
}
