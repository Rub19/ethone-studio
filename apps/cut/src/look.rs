use crate::model::{Clip, FONTS, Title};
use ab_glyph::{Font, FontVec, PxScale, ScaleFont, point};
use std::sync::OnceLock;

fn font(index: u8) -> Option<&'static FontVec> {
    static FONTS_LOADED: OnceLock<Vec<Option<FontVec>>> = OnceLock::new();
    let all = FONTS_LOADED.get_or_init(|| {
        let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        FONTS
            .iter()
            .map(|(_, file)| {
                [*file, "segoeuib.ttf", "arial.ttf"].iter().find_map(|f| std::fs::read(format!("{dir}\\Fonts\\{f}")).ok().and_then(|b| FontVec::try_from_vec(b).ok()))
            })
            .collect()
    });
    all.get(index as usize).and_then(|f| f.as_ref()).or_else(|| all.first().and_then(|f| f.as_ref()))
}

/// Everything that decides how one output frame looks besides the source pixels.
pub struct Frame<'a> {
    pub clip: Option<&'a Clip>,
    pub fade: f32,
    pub progress: f32,
    pub background: [u8; 4],
    pub titles: &'a [&'a Title],
    pub t: f64,
}

/// One output frame: source placed (zoom with Ken Burns, position, rotation, crop, mirror),
/// graded, blended over the background, blurred, titled and faded.
pub fn compose(rgba: &[u8], sw: u32, sh: u32, w: u32, h: u32, f: &Frame) -> Vec<u8> {
    let bg = f.background;
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        out.extend_from_slice(&[bg[0], bg[1], bg[2], 255]);
    }
    let def = Clip::default();
    let c = f.clip.unwrap_or(&def);
    if sw > 0 && sh > 0 && rgba.len() >= (sw * sh * 4) as usize {
        let zoom = if c.zoom_end > 0.0 { c.zoom + (c.zoom_end - c.zoom) * f.progress.clamp(0.0, 1.0) } else { c.zoom };
        let s = (w as f32 / sw as f32).min(h as f32 / sh as f32) * zoom.max(0.05);
        let (cx, cy) = (w as f32 / 2.0 + c.pos_x * w as f32 / 2.0, h as f32 / 2.0 + c.pos_y * h as f32 / 2.0);
        let (sn, cs) = (-c.rotation.to_radians()).sin_cos();
        let crop = c.crop.map(|v| v.clamp(0.0, 0.49));
        let (lx, rx) = (crop[0] * sw as f32, sw as f32 * (1.0 - crop[2]));
        let (ty, by) = (crop[1] * sh as f32, sh as f32 * (1.0 - crop[3]));
        let grade = Grade::of(c);
        let op = c.opacity.clamp(0.0, 1.0);
        let vig = c.vignette / 100.0;
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let (rxp, ryp) = (dx * cs - dy * sn, dx * sn + dy * cs);
                let mut sx = rxp / s + sw as f32 / 2.0;
                let sy = ryp / s + sh as f32 / 2.0;
                if sx < lx || sx >= rx || sy < ty || sy >= by {
                    continue;
                }
                if c.flip {
                    sx = sw as f32 - 1.0 - sx;
                }
                let si = ((sy as u32).min(sh - 1) * sw + (sx.max(0.0) as u32).min(sw - 1)) as usize * 4;
                let mut px = [rgba[si] as f32 / 255.0, rgba[si + 1] as f32 / 255.0, rgba[si + 2] as f32 / 255.0];
                grade.apply(&mut px);
                if vig != 0.0 {
                    let d = ((dx / (w as f32 / 2.0)).powi(2) + (dy / (h as f32 / 2.0)).powi(2)) / 2.0;
                    let k = 1.0 + vig * d;
                    px = px.map(|v| v * k);
                }
                let di = ((y * w + x) * 4) as usize;
                for k in 0..3 {
                    let v = px[k].clamp(0.0, 1.0) * 255.0;
                    out[di + k] = (bg[k] as f32 * (1.0 - op) + v * op).round() as u8;
                }
            }
        }
        if c.blur > 0.0 {
            box_blur(&mut out, w, h, ((c.blur / 100.0) * w as f32 / 40.0).round().max(1.0) as u32);
        }
    }
    for t in f.titles {
        draw_title(&mut out, w, h, t, f.t);
    }
    if f.fade < 1.0 {
        let target = if c.fade_white { 255.0 } else { 0.0 };
        for px in out.chunks_exact_mut(4) {
            for v in &mut px[..3] {
                *v = (target + (*v as f32 - target) * f.fade).round() as u8;
            }
        }
    }
    out
}

fn box_blur(buf: &mut [u8], w: u32, h: u32, r: u32) {
    let (w, h, r) = (w as usize, h as usize, r as usize);
    let mut tmp = buf.to_vec();
    blur_pass(buf, &mut tmp, w, h, r, true);
    blur_pass(&tmp, buf, w, h, r, false);
}

fn blur_pass(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, horizontal: bool) {
    {
        let (lines, len) = if horizontal { (h, w) } else { (w, h) };
        for line in 0..lines {
            let idx = |k: usize| if horizontal { (line * w + k) * 4 } else { (k * w + line) * 4 };
            let mut acc = [0u32; 3];
            for k in 0..=r.min(len - 1) {
                for c in 0..3 {
                    acc[c] += src[idx(k) + c] as u32;
                }
            }
            let mut count = (r + 1).min(len) as u32;
            for k in 0..len {
                for c in 0..3 {
                    dst[idx(k) + c] = (acc[c] / count.max(1)) as u8;
                }
                dst[idx(k) + 3] = 255;
                if k + r + 1 < len {
                    for c in 0..3 {
                        acc[c] += src[idx(k + r + 1) + c] as u32;
                    }
                    count += 1;
                }
                if k >= r {
                    for c in 0..3 {
                        acc[c] -= src[idx(k - r) + c] as u32;
                    }
                    count -= 1;
                }
            }
        }
    }
}

struct Grade {
    on: bool,
    bright: f32,
    contrast: f32,
    sat: f32,
    temp: f32,
    tint: f32,
    look: u8,
}

impl Grade {
    fn of(c: &Clip) -> Self {
        let on = c.brightness != 0.0 || c.contrast != 0.0 || c.saturation != 0.0 || c.temperature != 0.0 || c.tint != 0.0 || c.look != 0;
        Grade { on, bright: c.brightness / 100.0 * 0.4, contrast: 1.0 + c.contrast / 100.0, sat: 1.0 + c.saturation / 100.0, temp: c.temperature / 100.0 * 0.12, tint: c.tint / 100.0 * 0.08, look: c.look }
    }

    fn apply(&self, p: &mut [f32; 3]) {
        if !self.on {
            return;
        }
        p[0] += self.temp;
        p[2] -= self.temp;
        p[1] -= self.tint;
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

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn draw_title(out: &mut [u8], w: u32, h: u32, t: &Title, now: f64) {
    let Some(font) = font(t.font) else { return };
    let local = (now - t.start).max(0.0) as f32;
    let dur = t.dur.max(0.1) as f32;
    let ramp = |d: f32| (local / d).min((dur - local) / d).clamp(0.0, 1.0);
    let (alpha, lift, scale, reveal) = match t.anim {
        1 => (ramp(0.5), 0.0, 1.0, 1.0),
        2 => (ramp(0.4), (1.0 - ease(local / 0.6)) * 0.06, 1.0, 1.0),
        3 => (1.0, 0.0, 1.0, (local / (dur * 0.5)).clamp(0.0, 1.0)),
        4 => (ramp(0.4), 0.0, 0.6 + 0.4 * ease(local / 0.5), 1.0),
        _ => (1.0, 0.0, 1.0, 1.0),
    };
    if alpha <= 0.0 {
        return;
    }
    let size = (t.size * h as f32 * scale).max(6.0);
    let scaled = font.as_scaled(PxScale::from(size));
    let line_h = scaled.height() + scaled.line_gap();
    let total_chars = t.text.chars().filter(|c| *c != '\n').count();
    let mut budget = (total_chars as f32 * reveal).round() as usize;
    let lines: Vec<String> = t
        .text
        .lines()
        .map(|l| {
            let part: String = l.chars().take(budget).collect();
            budget = budget.saturating_sub(l.chars().count());
            part
        })
        .collect();
    if lines.is_empty() {
        return;
    }
    let full_widths: Vec<f32> = t.text.lines().map(|l| l.chars().map(|ch| scaled.h_advance(scaled.glyph_id(ch))).sum()).collect();
    let block_h = line_h * lines.len() as f32;
    let (ox, oy) = (t.dx * w as f32, t.dy * h as f32 + lift * h as f32);
    let top = oy + match t.place {
        0 => h as f32 * 0.08,
        1 => (h as f32 - block_h) / 2.0,
        2 => h as f32 * 0.92 - block_h,
        _ => h as f32 * 0.78 - block_h / 2.0,
    };
    let lower_third = t.place == 3;
    if t.band || lower_third {
        let pad = size * 0.4;
        let maxw = full_widths.iter().cloned().fold(0.0, f32::max);
        let (x0, x1) = if lower_third { (w as f32 * 0.06 - pad + ox, w as f32 * 0.06 + maxw + pad + ox) } else { ((w as f32 - maxw) / 2.0 - pad + ox, (w as f32 + maxw) / 2.0 + pad + ox) };
        let (y0, y1) = ((top - pad * 0.6).max(0.0) as u32, ((top + block_h + pad * 0.6).max(0.0) as u32).min(h));
        for y in y0..y1 {
            for x in (x0.max(0.0) as u32)..(x1.clamp(0.0, w as f32) as u32) {
                let i = ((y * w + x) * 4) as usize;
                for k in 0..3 {
                    out[i + k] = (out[i + k] as f32 * (1.0 - 0.65 * alpha)) as u8;
                }
            }
        }
    }
    let passes: &[(f32, [u8; 3], f32)] = if t.shadow { &[(size * 0.05, [0, 0, 0], 0.6), (0.0, [t.color[0], t.color[1], t.color[2]], 1.0)] } else { &[(0.0, [t.color[0], t.color[1], t.color[2]], 1.0)] };
    for (offset, rgb, strength) in passes {
        for (li, line) in lines.iter().enumerate() {
            let x_start = ox + offset + if lower_third { w as f32 * 0.06 } else { (w as f32 - full_widths[li]) / 2.0 };
            let base = top + offset + scaled.ascent() + li as f32 * line_h;
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
                    let a = cov.clamp(0.0, 1.0) * t.color[3] as f32 / 255.0 * alpha * strength;
                    for k in 0..3 {
                        out[i + k] = (out[i + k] as f32 * (1.0 - a) + rgb[k] as f32 * a).round() as u8;
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame<'a>(clip: Option<&'a Clip>, fade: f32, titles: &'a [&'a Title]) -> Frame<'a> {
        Frame { clip, fade, progress: 0.0, background: [0, 0, 0, 255], titles, t: 0.5 }
    }

    #[test]
    fn compose_places_grades_fades_and_titles() {
        let src = vec![200u8; 4 * 4 * 4];
        let plain = compose(&src, 4, 4, 8, 4, &frame(None, 1.0, &[]));
        assert_eq!(&plain[0..4], &[0, 0, 0, 255]);
        assert_eq!(plain[((8 + 4) * 4) as usize], 200);
        let mono = Clip { look: 1, saturation: 50.0, ..Default::default() };
        let g = compose(&[200, 50, 50, 255].repeat(16), 4, 4, 4, 4, &frame(Some(&mono), 1.0, &[]));
        assert!(g[0] == g[1] && g[1] == g[2]);
        let white = Clip { fade_white: true, ..Default::default() };
        assert_eq!(compose(&src, 4, 4, 4, 4, &frame(Some(&white), 0.0, &[]))[0], 255);
        let half = Clip { opacity: 0.5, ..Default::default() };
        assert_eq!(compose(&src, 4, 4, 4, 4, &Frame { background: [100, 100, 100, 255], ..frame(Some(&half), 1.0, &[]) })[0], 150);
        let moved = Clip { pos_x: 1.0, ..Default::default() };
        let m = compose(&src, 4, 4, 8, 4, &frame(Some(&moved), 1.0, &[]));
        assert_eq!(m[((8 + 2) * 4) as usize], 0);
        assert_eq!(m[((8 + 7) * 4) as usize], 200);
        let cropped = Clip { crop: [0.25, 0.0, 0.25, 0.0], ..Default::default() };
        let cr = compose(&src, 4, 4, 4, 4, &frame(Some(&cropped), 1.0, &[]));
        assert_eq!(cr[0], 0);
        assert_eq!(cr[4], 200);
        let mut grad = Vec::new();
        for y in 0..20u32 {
            for x in 0..20u32 {
                let _ = y;
                grad.extend_from_slice(&[if x < 10 { 0 } else { 255 }, 0, 0, 255]);
            }
        }
        let blurred = Clip { blur: 100.0, ..Default::default() };
        let b = compose(&grad, 20, 20, 20, 20, &frame(Some(&blurred), 1.0, &[]));
        let mid = b[((10 * 20 + 10) * 4) as usize];
        assert!(mid > 20 && mid < 250, "{mid}");
        if font(0).is_some() {
            let title = Title { text: "Ethone".into(), start: 0.0, dur: 1.0, size: 0.3, place: 1, color: [255, 0, 0, 255], band: false, anim: 1, shadow: true, font: 2, dx: 0.0, dy: 0.0 };
            let black = vec![0u8; 200 * 100 * 4];
            let t = compose(&black, 200, 100, 200, 100, &frame(None, 1.0, &[&title]));
            assert!(t.chunks(4).any(|p| p[0] > 200 && p[1] < 50));
            let early = compose(&black, 200, 100, 200, 100, &Frame { t: 0.0, ..frame(None, 1.0, &[&title]) });
            assert!(early.chunks(4).all(|p| p[0] == 0));
        }
    }
}
