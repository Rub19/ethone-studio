use image::RgbaImage;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

type P = (f32, f32);

pub fn rect(x0: f32, y0: f32, x1: f32, y1: f32, radius: f32) -> Vec<P> {
    let r = radius.min((x1 - x0).abs() / 2.0).min((y1 - y0).abs() / 2.0).max(0.0);
    if r < 0.5 {
        return vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    }
    let mut pts = Vec::new();
    for (cx, cy, start) in [(x1 - r, y0 + r, -FRAC_PI_2), (x1 - r, y1 - r, 0.0), (x0 + r, y1 - r, FRAC_PI_2), (x0 + r, y0 + r, PI)] {
        for k in 0..=10 {
            let a = start + k as f32 / 10.0 * FRAC_PI_2;
            pts.push((cx + a.cos() * r, cy + a.sin() * r));
        }
    }
    pts
}

pub fn ellipse(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<P> {
    let (cx, cy, rx, ry) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0, (x1 - x0).abs() / 2.0, (y1 - y0).abs() / 2.0);
    (0..128).map(|k| {
        let a = k as f32 / 128.0 * TAU;
        (cx + a.cos() * rx, cy + a.sin() * ry)
    }).collect()
}

pub fn polygon(x0: f32, y0: f32, x1: f32, y1: f32, sides: u32, star: Option<f32>) -> Vec<P> {
    let (cx, cy, rx, ry) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0, (x1 - x0).abs() / 2.0, (y1 - y0).abs() / 2.0);
    let n = sides.max(3) as usize * if star.is_some() { 2 } else { 1 };
    (0..n).map(|k| {
        let a = k as f32 / n as f32 * TAU - FRAC_PI_2;
        let s = if star.is_some() && k % 2 == 1 { 1.0 - star.unwrap_or(0.5) } else { 1.0 };
        (cx + a.cos() * rx * s, cy + a.sin() * ry * s)
    }).collect()
}

pub fn line(a: P, b: P, width: f32, arrow_start: bool, arrow_end: bool) -> Vec<Vec<P>> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (ux, uy) = (dx / len, dy / len);
    let (nx, ny) = (-uy * width / 2.0, ux * width / 2.0);
    let head = (width * 4.0).max(10.0);
    let a2 = if arrow_start { (a.0 + ux * head * 0.8, a.1 + uy * head * 0.8) } else { a };
    let b2 = if arrow_end { (b.0 - ux * head * 0.8, b.1 - uy * head * 0.8) } else { b };
    let mut out = vec![vec![(a2.0 + nx, a2.1 + ny), (b2.0 + nx, b2.1 + ny), (b2.0 - nx, b2.1 - ny), (a2.0 - nx, a2.1 - ny)]];
    let arrow = |tip: P, ux: f32, uy: f32| {
        let base = (tip.0 - ux * head, tip.1 - uy * head);
        let (px, py) = (-uy * head * 0.45, ux * head * 0.45);
        vec![tip, (base.0 + px, base.1 + py), (base.0 - px, base.1 - py)]
    };
    if arrow_end {
        out.push(arrow(b, ux, uy));
    }
    if arrow_start {
        out.push(arrow(a, -ux, -uy));
    }
    out
}

pub fn custom(kind: usize, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<P> {
    let unit: Vec<P> = match kind {
        0 => (0..96).map(|k| {
            let t = k as f32 / 96.0 * TAU;
            let x = 16.0 * t.sin().powi(3);
            let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            (0.5 + x / 34.0, 0.45 - y / 32.0)
        }).collect(),
        1 => polygon(0.0, 0.0, 1.0, 1.0, 5, Some(0.55)),
        2 => vec![(0.0, 0.35), (0.6, 0.35), (0.6, 0.1), (1.0, 0.5), (0.6, 0.9), (0.6, 0.65), (0.0, 0.65)],
        3 => {
            let mut p = rect(0.0, 0.0, 1.0, 0.72, 0.15);
            let i = p.iter().position(|q| q.1 > 0.7 && q.0 < 0.5).unwrap_or(0);
            p.splice(i..i, [(0.35, 0.72), (0.2, 1.0), (0.5, 0.72)]);
            p
        }
        4 => vec![(0.0, 0.55), (0.15, 0.4), (0.38, 0.62), (0.85, 0.1), (1.0, 0.25), (0.38, 0.9)],
        5 => vec![(0.15, 0.0), (0.5, 0.35), (0.85, 0.0), (1.0, 0.15), (0.65, 0.5), (1.0, 0.85), (0.85, 1.0), (0.5, 0.65), (0.15, 1.0), (0.0, 0.85), (0.35, 0.5), (0.0, 0.15)],
        6 => vec![(0.55, 0.0), (0.15, 0.55), (0.45, 0.55), (0.35, 1.0), (0.85, 0.4), (0.55, 0.4), (0.7, 0.0)],
        7 => {
            let mut p = vec![(0.5, 0.0)];
            for k in 0..=40 {
                let a = -0.15 * PI + k as f32 / 40.0 * 1.3 * PI;
                p.push((0.5 + a.cos() * 0.38, 0.62 + a.sin() * 0.38));
            }
            p
        }
        _ => {
            let mut p: Vec<P> = (0..=40).map(|k| {
                let a = FRAC_PI_2 + k as f32 / 40.0 * PI;
                (0.5 + a.cos() * 0.5, 0.5 - a.sin() * 0.5)
            }).collect();
            p.extend((0..=40).rev().map(|k| {
                let a = FRAC_PI_2 + k as f32 / 40.0 * PI;
                (0.72 + a.cos() * 0.4, 0.5 - a.sin() * 0.46)
            }));
            p
        }
    };
    let (w, h) = (x1 - x0, y1 - y0);
    unit.into_iter().map(|(x, y)| (x0 + x * w, y0 + y * h)).collect()
}

pub const FONTS: [(&str, [&str; 4]); 11] = [
    ("Segoe UI", ["segoeui.ttf", "segoeuib.ttf", "segoeuii.ttf", "segoeuiz.ttf"]),
    ("Arial", ["arial.ttf", "arialbd.ttf", "ariali.ttf", "arialbi.ttf"]),
    ("Calibri", ["calibri.ttf", "calibrib.ttf", "calibrii.ttf", "calibriz.ttf"]),
    ("Verdana", ["verdana.ttf", "verdanab.ttf", "verdanai.ttf", "verdanaz.ttf"]),
    ("Trebuchet MS", ["trebuc.ttf", "trebucbd.ttf", "trebucit.ttf", "trebucbi.ttf"]),
    ("Georgia", ["georgia.ttf", "georgiab.ttf", "georgiai.ttf", "georgiaz.ttf"]),
    ("Times New Roman", ["times.ttf", "timesbd.ttf", "timesi.ttf", "timesbi.ttf"]),
    ("Consolas", ["consola.ttf", "consolab.ttf", "consolai.ttf", "consolaz.ttf"]),
    ("Courier New", ["cour.ttf", "courbd.ttf", "couri.ttf", "courbi.ttf"]),
    ("Impact", ["impact.ttf", "impact.ttf", "impact.ttf", "impact.ttf"]),
    ("Comic Sans MS", ["comic.ttf", "comicbd.ttf", "comici.ttf", "comicz.ttf"]),
];

pub struct TextStyle {
    pub font: usize,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub align: usize,
    pub tracking: f32,
    pub leading: f32,
    pub vertical: bool,
    pub color: [u8; 4],
}

pub fn render_text(text: &str, st: &TextStyle) -> Option<RgbaImage> {
    use ab_glyph::{Font, PxScale, ScaleFont, point};
    let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let files = FONTS[st.font.min(FONTS.len() - 1)].1;
    let pick = (st.bold as usize) + 2 * (st.italic as usize);
    let data = std::fs::read(format!("{dir}\\Fonts\\{}", files[pick])).or_else(|_| std::fs::read(format!("{dir}\\Fonts\\{}", files[0]))).or_else(|_| std::fs::read(format!("{dir}\\Fonts\\segoeui.ttf"))).ok()?;
    let font = ab_glyph::FontVec::try_from_vec(data).ok()?;
    let size = st.size;
    let scaled = font.as_scaled(PxScale::from(size));
    let line_h = (scaled.height() + scaled.line_gap()) * st.leading;
    let track = st.tracking / 1000.0 * size;
    let lines: Vec<&str> = text.lines().collect();
    let mut placed: Vec<(ab_glyph::Glyph, usize)> = Vec::new();
    let mut widths = Vec::new();
    for (li, line) in lines.iter().enumerate() {
        let mut x = 0.0;
        let mut prev = None;
        for ch in line.chars() {
            let id = scaled.glyph_id(ch);
            if st.vertical {
                let adv = scaled.h_advance(id);
                let col_x = li as f32 * size * 1.2 * st.leading;
                placed.push((id.with_scale_and_position(size, point(col_x + (size - adv) / 2.0, scaled.ascent() + x)), li));
                x += line_h / st.leading + track;
                continue;
            }
            if let Some(p) = prev {
                x += scaled.kern(p, id);
            }
            placed.push((id.with_scale_and_position(size, point(x, scaled.ascent() + li as f32 * line_h)), li));
            x += scaled.h_advance(id) + track;
            prev = Some(id);
        }
        widths.push(x);
    }
    let maxw = widths.iter().cloned().fold(0.0, f32::max);
    let (w, h) = if st.vertical {
        ((size * 1.2 * st.leading * lines.len().max(1) as f32).ceil() as u32 + 4, maxw.ceil() as u32 + 4)
    } else {
        (maxw.ceil() as u32 + 4, (line_h * lines.len().max(1) as f32).ceil() as u32 + 4)
    };
    let mut img = RgbaImage::new(w.max(1), h.max(1));
    let color = st.color;
    let slant = if st.italic && files[pick] == files[0] { 0.2 } else { 0.0 };
    for (g, li) in placed {
        let shift = if st.vertical { 0.0 } else { (maxw - widths[li]) * st.align as f32 / 2.0 };
        if let Some(o) = font.outline_glyph(g) {
            let b = o.px_bounds();
            o.draw(|x, y, c| {
                let skew = (b.max.y - (y as f32 + b.min.y)) * slant;
                let (px, py) = (x as i32 + b.min.x as i32 + shift as i32 + skew as i32, y as i32 + b.min.y as i32);
                if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                    let a = (c.clamp(0.0, 1.0) * color[3] as f32).round() as u8;
                    let p = img.get_pixel_mut(px as u32, py as u32);
                    if a > p.0[3] {
                        p.0 = [color[0], color[1], color[2], a];
                    }
                }
            });
        }
    }
    if st.underline && !st.vertical {
        let thick = (size / 16.0).max(1.0) as u32;
        for (li, lw) in widths.iter().enumerate() {
            let y = (scaled.ascent() + li as f32 * line_h + size * 0.12) as u32;
            let shift = (maxw - lw) * st.align as f32 / 2.0;
            for yy in y..(y + thick).min(h) {
                for x in shift as u32..((shift + lw) as u32).min(w) {
                    img.put_pixel(x, yy, image::Rgba(color));
                }
            }
        }
    }
    Some(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_are_closed_polygons() {
        assert_eq!(rect(0.0, 0.0, 10.0, 10.0, 0.0).len(), 4);
        assert!(rect(0.0, 0.0, 100.0, 50.0, 10.0).len() > 20);
        assert_eq!(polygon(0.0, 0.0, 10.0, 10.0, 6, None).len(), 6);
        assert_eq!(polygon(0.0, 0.0, 10.0, 10.0, 5, Some(0.5)).len(), 10);
        assert_eq!(line((0.0, 0.0), (100.0, 0.0), 4.0, true, true).len(), 3);
        for k in 0..9 {
            let p = custom(k, 0.0, 0.0, 100.0, 100.0);
            assert!(p.len() >= 3, "{k}");
            assert!(p.iter().all(|q| q.0 >= -1.0 && q.0 <= 101.0 && q.1 >= -1.0 && q.1 <= 101.0), "{k}");
        }
    }
}
