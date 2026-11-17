use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2, pos2, vec2};
use image::RgbImage;
use studio_ui::pro::{BLUE, FAINT, FIELD, INK, INK_STRONG, MUTED, semibold};

pub fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(semibold(11.0)).color(FAINT));
    ui.add_space(4.0);
}

pub fn caption(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.5).color(FAINT));
}

pub fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>()) }
}

pub fn color_image(img: &RgbImage) -> egui::ColorImage {
    egui::ColorImage::from_rgb([img.width() as usize, img.height() as usize], img.as_raw())
}

pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub fn nav_row(ui: &mut Ui, label: &str, count: usize, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, CornerRadius::same(7), BLUE.gamma_multiply(0.24));
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(7), Color32::from_white_alpha(8));
    }
    p.text(rect.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, short(label, 28), if selected { semibold(12.5) } else { FontId::proportional(12.5) }, if selected { INK_STRONG } else { INK });
    if count != usize::MAX {
        p.text(rect.right_center() - vec2(8.0, 0.0), Align2::RIGHT_CENTER, count.to_string(), FontId::proportional(12.0), FAINT);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn track_color(track: Option<[Color32; 2]>, t: f32) -> Option<Color32> {
    track.map(|[a, b]| a.lerp_to_gamma(b, t))
}

pub fn slider_full(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, step: f32, default: f32, track: Option<[Color32; 2]>, suffix: &str) -> bool {
    let (lo, hi) = (*range.start(), *range.end());
    let before = *v;
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
    let label_w = if label.is_empty() { 0.0 } else { (width * 0.38).min(112.0) };
    let value_w = 46.0;
    let lr = Rect::from_min_size(row.min, vec2(label_w, row.height()));
    let lresp = ui.interact(lr, ui.make_persistent_id(("sl-label", label, row.min.y as i32)), Sense::click());
    if !label.is_empty() {
        ui.painter().text(lr.left_center(), Align2::LEFT_CENTER, short(label, (label_w / 6.6) as usize), FontId::proportional(12.0), if *v != default { INK } else { MUTED });
    }
    if lresp.on_hover_text(format!("{label}\nDouble-clic : valeur par défaut")).double_clicked() {
        *v = default;
    }
    let tr = Rect::from_min_max(pos2(row.left() + label_w + 4.0, row.top()), pos2(row.right() - value_w - 6.0, row.bottom()));
    let resp = ui.interact(tr, ui.make_persistent_id(("sl-track", label, row.min.y as i32, tr.min.x as i32)), Sense::click_and_drag());
    let p = ui.painter();
    let line = Rect::from_min_max(pos2(tr.left(), tr.center().y - 1.5), pos2(tr.right(), tr.center().y + 1.5));
    let frac = |x: f32| ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    if track.is_some() {
        let n = 24;
        for k in 0..n {
            let (a, b) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
            let seg = Rect::from_min_max(pos2(line.left() + line.width() * a, line.top()), pos2(line.left() + line.width() * b + 0.5, line.bottom()));
            p.rect_filled(seg, CornerRadius::ZERO, track_color(track, (a + b) / 2.0).unwrap_or(FIELD));
        }
    } else {
        p.rect_filled(line, CornerRadius::same(2), Color32::from_rgb(0x3a, 0x3a, 0x42));
        let zero = frac(default.clamp(lo, hi));
        let (a, b) = (zero.min(frac(*v)), zero.max(frac(*v)));
        p.rect_filled(Rect::from_min_max(pos2(line.left() + line.width() * a, line.top()), pos2(line.left() + line.width() * b, line.bottom())), CornerRadius::same(2), BLUE.gamma_multiply(0.85));
    }
    let kx = line.left() + line.width() * frac(*v);
    let hot = resp.hovered() || resp.dragged();
    p.circle(pos2(kx, line.center().y), if hot { 6.5 } else { 5.5 }, Color32::WHITE, Stroke::new(1.0, Color32::from_black_alpha(120)));
    if resp.double_clicked() {
        *v = default;
    } else if (resp.dragged() || resp.clicked())
        && let Some(pp) = resp.interact_pointer_pos()
    {
        let fine = ui.input(|i| i.modifiers.shift);
        if fine && resp.dragged() {
            *v += resp.drag_delta().x / line.width() * (hi - lo) * 0.1;
        } else {
            *v = lo + (hi - lo) * ((pp.x - line.left()) / line.width()).clamp(0.0, 1.0);
        }
        if step > 0.0 {
            *v = (*v / step).round() * step;
        }
        *v = v.clamp(lo, hi);
    }
    let vr = Rect::from_min_max(pos2(row.right() - value_w, row.top() + 2.0), pos2(row.right(), row.bottom() - 2.0));
    let decimals = if step < 0.05 { 2 } else if step < 0.5 { 1 } else { 0 };
    ui.put(vr, egui::DragValue::new(v).range(lo..=hi).speed(step.max(0.01) as f64).max_decimals(decimals).suffix(suffix));
    ui.add_space(1.0);
    *v != before
}

pub fn slider(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, step: f32) -> bool {
    slider_full(ui, label, v, range, step, 0.0, None, "")
}

pub fn slider_track(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, step: f32, track: [Color32; 2]) -> bool {
    slider_full(ui, label, v, range, step, 0.0, Some(track), "")
}

pub fn pct(ui: &mut Ui, label: &str, v: &mut f32, lo: f32, hi: f32, default: f32) -> bool {
    let mut x = *v * 100.0;
    let c = slider_full(ui, label, &mut x, lo * 100.0..=hi * 100.0, 1.0, default * 100.0, None, "");
    if c {
        *v = x / 100.0;
    }
    c
}

pub fn hue_slider(ui: &mut Ui, label: &str, v: &mut f32) -> bool {
    let before = *v;
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(width, 22.0), Sense::hover());
    let label_w = (width * 0.38).min(112.0);
    ui.painter().text(row.left_center(), Align2::LEFT_CENTER, short(label, (label_w / 6.6) as usize), FontId::proportional(12.0), if *v != 0.0 { INK } else { MUTED });
    let r = Rect::from_min_max(pos2(row.left() + label_w + 4.0, row.top() + 5.0), pos2(row.right() - 52.0, row.bottom() - 5.0));
    let resp = ui.interact(r.expand(4.0), ui.make_persistent_id(("hue", label, row.min.y as i32)), Sense::click_and_drag());
    let p = ui.painter();
    let n = 36;
    for k in 0..n {
        let h = k as f32 / n as f32;
        let seg = Rect::from_min_max(pos2(r.left() + r.width() * h, r.top()), pos2(r.left() + r.width() * (k + 1) as f32 / n as f32 + 0.5, r.bottom()));
        p.rect_filled(seg, CornerRadius::ZERO, Color32::from(egui::ecolor::Hsva::new(h, 0.75, 0.95, 1.0)));
    }
    let x = r.left() + r.width() * (*v / 360.0);
    p.circle(pos2(x, r.center().y), 6.0, Color32::WHITE, Stroke::new(1.5, Color32::from_black_alpha(160)));
    p.text(pos2(row.right(), row.center().y), Align2::RIGHT_CENTER, format!("{:.0}°", *v), FontId::proportional(12.0), MUTED);
    if resp.double_clicked() {
        *v = 0.0;
    } else if let Some(pos) = resp.interact_pointer_pos()
        && (resp.dragged() || resp.clicked())
    {
        *v = ((pos.x - r.left()) / r.width()).clamp(0.0, 1.0) * 360.0;
    }
    ui.add_space(2.0);
    *v != before
}

pub fn color_wheel(ui: &mut Ui, title: &str, size: f32, hue: &mut f32, sat: &mut f32) -> bool {
    let before = (*hue, *sat);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(title).size(12.0).color(if *sat > 0.0 { INK } else { MUTED }));
        let (r, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
        let c = r.center();
        let rad = size / 2.0 - 4.0;
        let mut mesh = egui::Mesh::default();
        let n = 48;
        let rings = 6;
        let point = |a: f32, f: f32| c + vec2(a.cos(), a.sin()) * rad * f;
        let col = |a: f32, f: f32| Color32::from(egui::ecolor::Hsva::new((a / std::f32::consts::TAU).rem_euclid(1.0), f * 0.85, 0.85, 1.0));
        for ring in 0..rings {
            let (f0, f1) = (ring as f32 / rings as f32, (ring + 1) as f32 / rings as f32);
            for k in 0..n {
                let (a0, a1) = (k as f32 / n as f32 * std::f32::consts::TAU, (k + 1) as f32 / n as f32 * std::f32::consts::TAU);
                let i = mesh.vertices.len() as u32;
                mesh.colored_vertex(point(a0, f0), col(a0, f0));
                mesh.colored_vertex(point(a1, f0), col(a1, f0));
                mesh.colored_vertex(point(a1, f1), col(a1, f1));
                mesh.colored_vertex(point(a0, f1), col(a0, f1));
                mesh.add_triangle(i, i + 1, i + 2);
                mesh.add_triangle(i, i + 2, i + 3);
            }
        }
        let p = ui.painter();
        p.add(mesh);
        p.circle_stroke(c, rad, Stroke::new(1.0, Color32::from_black_alpha(140)));
        p.circle_stroke(c, rad * 0.5, Stroke::new(1.0, Color32::from_black_alpha(50)));
        let a = -hue.to_radians();
        let k = c + vec2(a.cos(), a.sin()) * rad * (*sat / 100.0).clamp(0.0, 1.0);
        p.circle(k, 5.5, Color32::WHITE, Stroke::new(1.5, Color32::from_black_alpha(170)));
        if resp.double_clicked() {
            (*hue, *sat) = (0.0, 0.0);
        } else if let Some(pp) = resp.interact_pointer_pos()
            && (resp.dragged() || resp.clicked())
        {
            let d = pp - c;
            *sat = ((d.length() / rad).clamp(0.0, 1.0) * 100.0).round();
            *hue = (-d.y.atan2(d.x)).to_degrees().rem_euclid(360.0).round();
        }
        resp.on_hover_text("Glisse pour choisir la teinte et l'intensité · double-clic pour remettre à zéro");
        ui.label(RichText::new(format!("{:.0}° · {:.0}", *hue, *sat)).size(11.0).color(FAINT));
    });
    (*hue, *sat) != before
}

pub fn curve_editor(ui: &mut Ui, pts: &mut Vec<[f32; 2]>, ink: Color32, parametric: Option<&[f32; 4]>, hist: Option<&[u32; 256]>) -> bool {
    let before = pts.clone();
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ui.available_width().min(220.0)), Sense::click_and_drag());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(8), Color32::from_rgb(0x14, 0x14, 0x17));
    if let Some(h) = hist {
        let max = h[2..254].iter().copied().max().unwrap_or(1).max(1) as f32;
        let mut mesh = egui::Mesh::default();
        let fill = Color32::from_white_alpha(18);
        for x in 0..255 {
            let (x0, x1) = (r.left() + r.width() * x as f32 / 255.0, r.left() + r.width() * (x + 1) as f32 / 255.0);
            let y = r.bottom() - r.height() * (h[x] as f32 / max).sqrt().min(1.0) * 0.9;
            let i = mesh.vertices.len() as u32;
            for q in [pos2(x0, y), pos2(x1, y), pos2(x1, r.bottom()), pos2(x0, r.bottom())] {
                mesh.colored_vertex(q, fill);
            }
            mesh.add_triangle(i, i + 1, i + 2);
            mesh.add_triangle(i, i + 2, i + 3);
        }
        p.add(mesh);
    }
    for k in 1..4 {
        let f = k as f32 / 4.0;
        let g = Stroke::new(1.0, Color32::from_white_alpha(16));
        p.line_segment([pos2(r.left() + r.width() * f, r.top()), pos2(r.left() + r.width() * f, r.bottom())], g);
        p.line_segment([pos2(r.left(), r.top() + r.height() * f), pos2(r.right(), r.top() + r.height() * f)], g);
    }
    p.line_segment([r.left_bottom(), r.right_top()], Stroke::new(1.0, Color32::from_white_alpha(30)));
    let to_screen = |q: [f32; 2]| pos2(r.left() + r.width() * q[0], r.bottom() - r.height() * q[1]);
    let from_screen = |s: Pos2| [((s.x - r.left()) / r.width()).clamp(0.0, 1.0), ((r.bottom() - s.y) / r.height()).clamp(0.0, 1.0)];
    let lut = crate::develop::curve_lut(pts);
    let line: Vec<Pos2> = (0..=96)
        .map(|k| {
            let x = k as f32 / 96.0;
            let mut y = lut.as_ref().map(|l| l[(x * 255.0).round() as usize]).unwrap_or(x);
            if let Some(c) = parametric {
                for (i, a) in c.iter().enumerate() {
                    y += a / 100.0 * 0.18 * (-((y - (0.125 + 0.25 * i as f32)) / 0.15).powi(2)).exp();
                }
            }
            to_screen([x, y.clamp(0.0, 1.0)])
        })
        .collect();
    p.add(egui::Shape::line(line, Stroke::new(2.0, ink)));
    if pts.len() < 2 {
        *pts = vec![[0.0, 0.0], [1.0, 1.0]];
    }
    let drag_id = ui.make_persistent_id(("curve-drag", ink));
    let mut dragging: Option<usize> = ui.data(|d| d.get_temp(drag_id)).flatten();
    let hover = resp.hover_pos();
    let near = |s: Pos2, pts: &Vec<[f32; 2]>| pts.iter().position(|q| to_screen(*q).distance(s) < 9.0);
    for (k, q) in pts.iter().enumerate() {
        let hot = Some(k) == dragging || hover.is_some_and(|h| to_screen(*q).distance(h) < 9.0);
        p.circle(to_screen(*q), if hot { 5.5 } else { 4.0 }, Color32::from_rgb(0x14, 0x14, 0x17), Stroke::new(1.5, ink));
    }
    if resp.drag_started()
        && let Some(s) = resp.interact_pointer_pos()
    {
        dragging = near(s, pts).or_else(|| {
            let q = from_screen(s);
            let idx = pts.iter().position(|e| e[0] > q[0]).unwrap_or(pts.len());
            pts.insert(idx, q);
            Some(idx)
        });
    }
    if resp.dragged()
        && let (Some(k), Some(s)) = (dragging, resp.interact_pointer_pos())
        && k < pts.len()
    {
        let mut q = from_screen(s);
        let n = pts.len();
        let lo = if k == 0 { 0.0 } else { pts[k - 1][0] + 0.01 };
        let hi = if k + 1 == n { 1.0 } else { pts[k + 1][0] - 0.01 };
        q[0] = q[0].clamp(lo, hi.max(lo));
        pts[k] = q;
    }
    if resp.drag_stopped() {
        dragging = None;
    }
    if resp.clicked()
        && let Some(s) = resp.interact_pointer_pos()
        && near(s, pts).is_none()
    {
        let q = from_screen(s);
        let idx = pts.iter().position(|e| e[0] > q[0]).unwrap_or(pts.len());
        pts.insert(idx, [q[0], lut.as_ref().map(|l| l[(q[0] * 255.0).round() as usize]).unwrap_or(q[0])]);
    }
    if (resp.double_clicked() || resp.secondary_clicked())
        && let Some(s) = resp.interact_pointer_pos()
        && let Some(k) = near(s, pts)
        && pts.len() > 2
    {
        pts.remove(k);
    }
    ui.data_mut(|d| d.insert_temp(drag_id, dragging));
    if pts.len() == 2 && pts[0] == [0.0, 0.0] && pts[1] == [1.0, 1.0] {
        pts.clear();
    }
    resp.on_hover_text("Clic : ajouter un point · glisser : déplacer · double-clic ou clic droit : supprimer");
    *pts != before
}

pub fn draw_histogram(ui: &mut Ui, hist: Option<&[[u32; 256]; 4]>, clip: (bool, bool)) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(8), Color32::from_rgb(0x14, 0x14, 0x17));
    let Some(h) = hist else {
        p.text(r.center(), Align2::CENTER_CENTER, "—", FontId::proportional(13.0), FAINT);
        return;
    };
    let max = h[..3].iter().flat_map(|c| c[2..254].iter()).copied().max().unwrap_or(1).max(1) as f32;
    let inner = r.shrink(6.0);
    for (k, color) in [(0, Color32::from_rgba_unmultiplied(255, 70, 70, 110)), (1, Color32::from_rgba_unmultiplied(70, 220, 110, 110)), (2, Color32::from_rgba_unmultiplied(80, 140, 255, 110))] {
        let mut mesh = egui::Mesh::default();
        for x in 0..255 {
            let (fx0, fx1) = (inner.left() + inner.width() * x as f32 / 255.0, inner.left() + inner.width() * (x + 1) as f32 / 255.0);
            let (y0, y1) = (inner.bottom() - inner.height() * (h[k][x] as f32 / max).sqrt().min(1.0), inner.bottom() - inner.height() * (h[k][x + 1] as f32 / max).sqrt().min(1.0));
            let i = mesh.vertices.len() as u32;
            mesh.colored_vertex(pos2(fx0, y0), color);
            mesh.colored_vertex(pos2(fx1, y1), color);
            mesh.colored_vertex(pos2(fx1, inner.bottom()), color);
            mesh.colored_vertex(pos2(fx0, inner.bottom()), color);
            mesh.add_triangle(i, i + 1, i + 2);
            mesh.add_triangle(i, i + 2, i + 3);
        }
        p.add(mesh);
    }
    let total: u32 = h[3].iter().sum::<u32>().max(1);
    let (lo, hi) = (h[3][..2].iter().sum::<u32>() as f32 / total as f32, h[3][254..].iter().sum::<u32>() as f32 / total as f32);
    for (on, pct, at, color) in [(clip.0, lo, inner.left_top(), Color32::from_rgb(80, 140, 255)), (clip.1, hi, inner.right_top() - vec2(10.0, 0.0), Color32::from_rgb(255, 80, 80))] {
        let tri = vec![at + vec2(0.0, 0.0), at + vec2(10.0, 0.0), at + vec2(if at.x < inner.center().x { 0.0 } else { 10.0 }, 10.0)];
        p.add(egui::Shape::convex_polygon(tri, if on || pct > 0.002 { color } else { Color32::from_white_alpha(40) }, Stroke::NONE));
    }
}
