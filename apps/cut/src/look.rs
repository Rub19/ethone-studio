use crate::model::{Clip, Title};
use ab_glyph::{Font, FontVec, PxScale, ScaleFont, point};
use std::sync::OnceLock;

fn font() -> Option<&'static FontVec> {
    static FONT: OnceLock<Option<FontVec>> = OnceLock::new();
    FONT.get_or_init(|| {
        let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        ["segoeuib.ttf", "segoeui.ttf", "arial.ttf"].iter().find_map(|f| std::fs::read(format!("{dir}\\Fonts\\{f}")).ok().and_then(|b| FontVec::try_from_vec(b).ok()))
    })
    .as_ref()
}

/// One output frame: source fitted (zoom, mirror), graded, faded, with the active titles on top.
pub fn compose(rgba: &[u8], sw: u32, sh: u32, w: u32, h: u32, clip: Option<&Clip>, fade: f32, titles: &[&Title]) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for px in out.chunks_exact_mut(4) {
        px[3] = 255;
    }
    let def = Clip::default();
    let c = clip.unwrap_or(&def);
    if sw > 0 && sh > 0 && rgba.len() >= (sw * sh * 4) as usize {
        let s = (w as f32 / sw as f32).min(h as f32 / sh as f32) * c.zoom.max(0.1);
        let (ox, oy) = ((w as f32 - sw as f32 * s) / 2.0, (h as f32 - sh as f32 * s) / 2.0);
        let grade = Grade::of(c);
        for y in 0..h {
            let sy = ((y as f32 + 0.5 - oy) / s) as i64;
            if sy < 0 || sy >= sh as i64 {
                continue;
            }
            for x in 0..w {
                let mut sx = ((x as f32 + 0.5 - ox) / s) as i64;
                if sx < 0 || sx >= sw as i64 {
                    continue;
                }
                if c.flip {
                    sx = sw as i64 - 1 - sx;
                }
                let si = ((sy as u32 * sw + sx as u32) * 4) as usize;
                let mut px = [rgba[si] as f32 / 255.0, rgba[si + 1] as f32 / 255.0, rgba[si + 2] as f32 / 255.0];
                grade.apply(&mut px);
                let di = ((y * w + x) * 4) as usize;
                for k in 0..3 {
                    out[di + k] = (px[k].clamp(0.0, 1.0) * 255.0).round() as u8;
                }
            }
        }
    }
    for t in titles {
        draw_title(&mut out, w, h, t);
    }
    if fade < 1.0 {
        let target = if c.fade_white { 255.0 } else { 0.0 };
        for px in out.chunks_exact_mut(4) {
            for v in &mut px[..3] {
                *v = (target + (*v as f32 - target) * fade).round() as u8;
            }
        }
    }
    out
}

struct Grade {
    on: bool,
    bright: f32,
    contrast: f32,
    sat: f32,
    temp: f32,
    look: u8,
}

impl Grade {
    fn of(c: &Clip) -> Self {
        let on = c.brightness != 0.0 || c.contrast != 0.0 || c.saturation != 0.0 || c.temperature != 0.0 || c.look != 0;
        Grade { on, bright: c.brightness / 100.0 * 0.4, contrast: 1.0 + c.contrast / 100.0, sat: 1.0 + c.saturation / 100.0, temp: c.temperature / 100.0 * 0.12, look: c.look }
    }

    fn apply(&self, p: &mut [f32; 3]) {
        if !self.on {
            return;
        }
        p[0] += self.temp;
        p[2] -= self.temp;
        for v in p.iter_mut() {
            *v = (*v + self.bright - 0.5) * self.contrast + 0.5;
        }
        let l = 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
        for v in p.iter_mut() {
            *v = l + (*v - l) * self.sat.max(0.0);
        }
        match self.look {
            1 => *p = [l; 3],
            2 => *p = [l * 1.07 + 0.04, l * 0.95 + 0.02, l * 0.78],
            3 => {
                let q = *p;
                *p = [q[0] * 0.9 + 0.08, q[1] * 0.85 + 0.06, q[2] * 0.7 + 0.08];
            }
            4 => {
                let shadow = (1.0 - l).powi(2) * 0.06;
                let high = l.powi(2) * 0.05;
                *p = [p[0] - shadow + high, p[1] + shadow * 0.3, p[2] + shadow - high];
                for v in p.iter_mut() {
                    *v = (*v - 0.5) * 1.12 + 0.5;
                }
            }
            5 => {
                for v in p.iter_mut() {
                    *v = l + (*v - l) * 1.35;
                }
            }
            _ => {}
        }
    }
}

fn draw_title(out: &mut [u8], w: u32, h: u32, t: &Title) {
    let Some(font) = font() else { return };
    let size = (t.size * h as f32).max(6.0);
    let scaled = font.as_scaled(PxScale::from(size));
    let line_h = scaled.height() + scaled.line_gap();
    let lines: Vec<&str> = t.text.lines().collect();
    if lines.is_empty() {
        return;
    }
    let widths: Vec<f32> = lines.iter().map(|l| l.chars().map(|ch| scaled.h_advance(scaled.glyph_id(ch))).sum()).collect();
    let block_h = line_h * lines.len() as f32;
    let top = match t.place {
        0 => h as f32 * 0.08,
        1 => (h as f32 - block_h) / 2.0,
        2 => h as f32 * 0.92 - block_h,
        _ => h as f32 * 0.78 - block_h / 2.0,
    };
    let lower_third = t.place == 3;
    if t.band || lower_third {
        let pad = size * 0.4;
        let maxw = widths.iter().cloned().fold(0.0, f32::max);
        let (x0, x1) = if lower_third { (w as f32 * 0.06 - pad, w as f32 * 0.06 + maxw + pad) } else { ((w as f32 - maxw) / 2.0 - pad, (w as f32 + maxw) / 2.0 + pad) };
        let (y0, y1) = ((top - pad * 0.6).max(0.0) as u32, ((top + block_h + pad * 0.6) as u32).min(h));
        for y in y0..y1 {
            for x in (x0.max(0.0) as u32)..(x1.min(w as f32) as u32) {
                let i = ((y * w + x) * 4) as usize;
                for k in 0..3 {
                    out[i + k] = (out[i + k] as f32 * 0.35) as u8;
                }
            }
        }
    }
    for (li, line) in lines.iter().enumerate() {
        let x_start = if lower_third { w as f32 * 0.06 } else { (w as f32 - widths[li]) / 2.0 };
        let base = top + scaled.ascent() + li as f32 * line_h;
        let mut x = x_start;
        for ch in line.chars() {
            let id = scaled.glyph_id(ch);
            let g = id.with_scale_and_position(size, point(x, base));
            x += scaled.h_advance(id);
            let Some(o) = font.outline_glyph(g) else { continue };
            let b = o.px_bounds();
            o.draw(|gx, gy, cov| {
                let (px, py) = (gx as i64 + b.min.x as i64, gy as i64 + b.min.y as i64);
                if px < 0 || py < 0 || px >= w as i64 || py >= h as i64 {
                    return;
                }
                let i = ((py as u32 * w + px as u32) * 4) as usize;
                let a = cov.clamp(0.0, 1.0) * t.color[3] as f32 / 255.0;
                for k in 0..3 {
                    out[i + k] = (out[i + k] as f32 * (1.0 - a) + t.color[k] as f32 * a).round() as u8;
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_fits_grades_fades_and_titles() {
        let src = vec![200u8; 4 * 4 * 4];
        let plain = compose(&src, 4, 4, 8, 4, None, 1.0, &[]);
        assert_eq!(&plain[0..4], &[0, 0, 0, 255]);
        assert_eq!(plain[((1 * 8 + 4) * 4) as usize], 200);
        let mono = Clip { look: 1, saturation: 50.0, ..Default::default() };
        let g = compose(&[200, 50, 50, 255].repeat(16), 4, 4, 4, 4, Some(&mono), 1.0, &[]);
        assert!(g[0] == g[1] && g[1] == g[2]);
        let white = Clip { fade_white: true, ..Default::default() };
        let f = compose(&src, 4, 4, 4, 4, Some(&white), 0.0, &[]);
        assert_eq!(f[0], 255);
        if font().is_some() {
            let title = Title { text: "Ethone".into(), start: 0.0, dur: 1.0, size: 0.3, place: 1, color: [255, 0, 0, 255], band: false };
            let black = vec![0u8; 200 * 100 * 4];
            let t = compose(&black, 200, 100, 200, 100, None, 1.0, &[&title]);
            assert!(t.chunks(4).any(|p| p[0] > 200 && p[1] < 50));
        }
    }
}
