//! Pure image algorithms shared by Pixel and the ethone-mcp engine.

use crate::doc::Area;
use image::{Rgba, RgbaImage};

pub fn catmull(pts: &[(f32, f32)], steps: usize) -> Vec<(f32, f32)> {
    let n = pts.len();
    if n < 3 {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(n * steps);
    for i in 0..n {
        let (p0, p1, p2, p3) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
        for s in 0..steps {
            let t = s as f32 / steps as f32;
            let (t2, t3) = (t * t, t * t * t);
            let f = |a: f32, b: f32, c: f32, d: f32| 0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (3.0 * b - a - 3.0 * c + d) * t3);
            out.push((f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
        }
    }
    out
}

fn dist(a: [u8; 4], b: [u8; 4]) -> f32 {
    (0..3).map(|k| (a[k] as f32 - b[k] as f32).powi(2)).sum::<f32>().sqrt()
}

pub fn object_mask(img: &RgbaImage, a: Area, tol: f32) -> Vec<u8> {
    let (w, h) = img.dimensions();
    let (x0, y0, x1, y1) = (a.x0.min(w), a.y0.min(h), a.x1.min(w), a.y1.min(h));
    let mut mask = vec![0u8; (w * h) as usize];
    if x1 <= x0 + 2 || y1 <= y0 + 2 {
        return mask;
    }
    let mut border = Vec::new();
    let step = (((x1 - x0) + (y1 - y0)) / 60).max(1);
    for x in (x0..x1).step_by(step as usize) {
        border.push(img.get_pixel(x, y0).0);
        border.push(img.get_pixel(x, y1 - 1).0);
    }
    for y in (y0..y1).step_by(step as usize) {
        border.push(img.get_pixel(x0, y).0);
        border.push(img.get_pixel(x1 - 1, y).0);
    }
    let bg = |x: u32, y: u32| {
        let p = img.get_pixel(x, y).0;
        border.iter().any(|b| dist(*b, p) < tol)
    };
    let (bw, bh) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let mut seen = vec![false; bw * bh];
    let mut stack: Vec<(u32, u32)> = Vec::new();
    for x in x0..x1 {
        stack.push((x, y0));
        stack.push((x, y1 - 1));
    }
    for y in y0..y1 {
        stack.push((x0, y));
        stack.push((x1 - 1, y));
    }
    while let Some((x, y)) = stack.pop() {
        let i = (y - y0) as usize * bw + (x - x0) as usize;
        if seen[i] || !bg(x, y) {
            continue;
        }
        seen[i] = true;
        if x > x0 {
            stack.push((x - 1, y));
        }
        if x + 1 < x1 {
            stack.push((x + 1, y));
        }
        if y > y0 {
            stack.push((x, y - 1));
        }
        if y + 1 < y1 {
            stack.push((x, y + 1));
        }
    }
    for y in y0..y1 {
        for x in x0..x1 {
            if !seen[(y - y0) as usize * bw + (x - x0) as usize] {
                mask[(y * w + x) as usize] = 255;
            }
        }
    }
    mask
}

pub fn red_eye(img: &RgbaImage, cx: f32, cy: f32, r: f32, darken: f32) -> RgbaImage {
    let mut out = img.clone();
    let (w, h) = img.dimensions();
    let (x0, x1) = ((cx - r).max(0.0) as u32, ((cx + r) as u32).min(w.saturating_sub(1)));
    let (y0, y1) = ((cy - r).max(0.0) as u32, ((cy + r) as u32).min(h.saturating_sub(1)));
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / r;
            if d > 1.0 {
                continue;
            }
            let p = img.get_pixel(x, y).0;
            let (rr, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
            let red = rr - g.max(b);
            if red > 25.0 && rr > 70.0 {
                let k = ((red - 25.0) / 60.0).clamp(0.0, 1.0) * (1.0 - d.powi(4));
                let nr = (g + b) / 2.0 * (1.0 - darken);
                let mix = |a: f32, z: f32| (a + (z - a) * k).round().clamp(0.0, 255.0) as u8;
                out.put_pixel(x, y, Rgba([mix(rr, nr), mix(g, g * (1.0 - darken * 0.5)), mix(b, b * (1.0 - darken * 0.5)), p[3]]));
            }
        }
    }
    out
}

pub fn warp_quad(img: &RgbaImage, q: [(f32, f32); 4], ow: u32, oh: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    RgbaImage::from_fn(ow.max(1), oh.max(1), |x, y| {
        let (u, v) = ((x as f32 + 0.5) / ow as f32, (y as f32 + 0.5) / oh as f32);
        let top = (q[0].0 + (q[1].0 - q[0].0) * u, q[0].1 + (q[1].1 - q[0].1) * u);
        let bot = (q[3].0 + (q[2].0 - q[3].0) * u, q[3].1 + (q[2].1 - q[3].1) * u);
        let (sx, sy) = (top.0 + (bot.0 - top.0) * v, top.1 + (bot.1 - top.1) * v);
        let (fx, fy) = ((sx - 0.5).clamp(0.0, w as f32 - 1.0), (sy - 0.5).clamp(0.0, h as f32 - 1.0));
        let (ix, iy) = (fx as u32, fy as u32);
        let (jx, jy) = ((ix + 1).min(w - 1), (iy + 1).min(h - 1));
        let (tx, ty) = (fx - ix as f32, fy - iy as f32);
        let p = |a: u32, b: u32| img.get_pixel(a, b).0.map(|c| c as f32);
        let (a, b, c, d) = (p(ix, iy), p(jx, iy), p(ix, jy), p(jx, jy));
        Rgba(std::array::from_fn(|k| ((a[k] * (1.0 - tx) + b[k] * tx) * (1.0 - ty) + (c[k] * (1.0 - tx) + d[k] * tx) * ty).round() as u8))
    })
}

/// Mean and standard deviation of R, G, B over pixels where `keep` is true.
pub fn color_stats(img: &RgbaImage, keep: impl Fn(u32, u32) -> bool) -> ([f32; 3], [f32; 3]) {
    let (mut s, mut s2, mut n) = ([0f64; 3], [0f64; 3], 0f64);
    for (x, y, p) in img.enumerate_pixels() {
        if keep(x, y) {
            for k in 0..3 {
                let v = p.0[k] as f64;
                s[k] += v;
                s2[k] += v * v;
            }
            n += 1.0;
        }
    }
    let n = n.max(1.0);
    let mean = s.map(|v| (v / n) as f32);
    let sd = [0, 1, 2].map(|k| ((s2[k] / n - (s[k] / n).powi(2)).max(0.0).sqrt() as f32).max(1.0));
    (mean, sd)
}

/// Moves `src`'s colours toward the target mean/contrast ("Harmoniser"), weighted by `amount` 0..1.
pub fn harmonize(src: &RgbaImage, from: ([f32; 3], [f32; 3]), to: ([f32; 3], [f32; 3]), amount: f32) -> RgbaImage {
    let mut out = src.clone();
    for p in out.pixels_mut() {
        if p.0[3] == 0 {
            continue;
        }
        for k in 0..3 {
            let v = p.0[k] as f32;
            let t = (v - from.0[k]) * (to.1[k] / from.1[k]).clamp(0.4, 2.5) + to.0[k];
            p.0[k] = (v + (t - v) * amount).round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// New canvas size for "Étendre" : a margin in % on every side, or a target aspect ratio (w/h)
/// reached by growing only the short side. Returns (w, h, x offset, y offset).
pub fn expand_size(w: u32, h: u32, margin: f32, aspect: Option<f32>) -> (u32, u32, u32, u32) {
    let (nw, nh) = match aspect {
        Some(a) if (w as f32 / h as f32) < a => (((h as f32) * a).round() as u32, h),
        Some(a) => (w, ((w as f32) / a).round() as u32),
        None => (((w as f32) * (1.0 + 2.0 * margin)).round() as u32, ((h as f32) * (1.0 + 2.0 * margin)).round() as u32),
    };
    (nw.max(w), nh.max(h), (nw.max(w) - w) / 2, (nh.max(h) - h) / 2)
}


/// Also clears enclosed pockets of background colour (inside letters, between arms…):
/// any kept pixel whose colour is within `tol` of the median border colour goes transparent.
pub fn clear_holes(img: &RgbaImage, mask: &mut [u8], tol: f32) {
    let (w, h) = img.dimensions();
    if w < 2 || h < 2 {
        return;
    }
    let mut border: Vec<[u8; 4]> = Vec::new();
    for x in 0..w {
        border.push(img.get_pixel(x, 0).0);
        border.push(img.get_pixel(x, h - 1).0);
    }
    for y in 0..h {
        border.push(img.get_pixel(0, y).0);
        border.push(img.get_pixel(w - 1, y).0);
    }
    let med = |k: usize| {
        let mut v: Vec<u8> = border.iter().map(|p| p[k]).collect();
        v.sort_unstable();
        v[v.len() / 2]
    };
    let bg = [med(0), med(1), med(2), 255];
    for (i, m) in mask.iter_mut().enumerate() {
        if *m > 0 && dist(img.get_pixel(i as u32 % w, i as u32 / w).0, bg) < tol {
            *m = 0;
        }
    }
}

/// Finishes an expanded canvas. Where the picture's edge is smooth (sky, wall, gradient) the
/// edge gradient is carried on outward; where it is textured, the content-aware fill is kept.
/// `original` is the untouched picture, placed at (`ox`, `oy`) inside `filled`; it is copied back as is.
pub fn smooth_expand(filled: &RgbaImage, original: &RgbaImage, ox: u32, oy: u32) -> RgbaImage {
    let (w, h) = filled.dimensions();
    let (iw, ih) = original.dimensions();
    let mut base = filled.clone();
    image::imageops::replace(&mut base, original, ox as i64, oy as i64);
    if iw < 4 || ih < 4 {
        return base;
    }
    let filled = &base;
    let stretched = RgbaImage::from_fn(w, h, |x, y| *original.get_pixel(x.clamp(ox, ox + iw - 1) - ox, y.clamp(oy, oy + ih - 1) - oy));
    let fine = image::imageops::fast_blur(&stretched, 2.0);
    let detail = RgbaImage::from_fn(w, h, |x, y| {
        let (a, b) = (stretched.get_pixel(x, y).0, fine.get_pixel(x, y).0);
        let e = (0..3).map(|k| (a[k] as i32 - b[k] as i32).unsigned_abs()).sum::<u32>().min(255) as u8;
        Rgba([e, e, e, 255])
    });
    let energy = image::imageops::fast_blur(&detail, 6.0);
    let band = (iw.min(ih) / 8).clamp(4, 40) as f32;
    let mut out = filled.clone();
    for (x, y, p) in out.enumerate_pixels_mut() {
        if x >= ox && y >= oy && x < ox + iw && y < oy + ih {
            continue;
        }
        let (ex, ey) = (x.clamp(ox, ox + iw - 1), y.clamp(oy, oy + ih - 1));
        let (dx, dy) = (x as f32 - ex as f32, y as f32 - ey as f32);
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        let (nx, ny) = (-dx / d, -dy / d);
        let ix = (ex as f32 + nx * band).round().clamp(ox as f32, (ox + iw - 1) as f32) as u32;
        let iy = (ey as f32 + ny * band).round().clamp(oy as f32, (oy + ih - 1) as f32) as u32;
        let e = fine.get_pixel(ex, ey).0;
        let i = fine.get_pixel(ix, iy).0;
        let damp = d * 0.5 / (1.0 + d / (band * 4.0));
        let t = (energy.get_pixel(ex, ey).0[0] as f32 / 18.0).clamp(0.0, 1.0);
        let f = filled.get_pixel(x, y).0;
        for c in 0..3 {
            let slope = (e[c] as f32 - i[c] as f32) / band;
            let base = (e[c] as f32 + slope * damp).clamp(0.0, 255.0);
            p.0[c] = (base * (1.0 - t) + f[c] as f32 * t).round().clamp(0.0, 255.0) as u8;
        }
        p.0[3] = 255;
    }
    out
}

#[cfg(test)]
mod expand_tests {
    use super::*;

    #[test]
    fn holes_and_smooth_edges() {
        let mut img = RgbaImage::from_pixel(30, 30, Rgba([10, 10, 10, 255]));
        for y in 5..25 {
            for x in 5..25 {
                img.put_pixel(x, y, Rgba([240, 240, 240, 255]));
            }
        }
        for y in 12..18 {
            for x in 12..18 {
                img.put_pixel(x, y, Rgba([10, 10, 10, 255]));
            }
        }
        let a = Area::from_points((0.0, 0.0), (30.0, 30.0), 30, 30).unwrap();
        let mut m = object_mask(&img, a, 30.0);
        assert_eq!(m[15 * 30 + 15], 255);
        clear_holes(&img, &mut m, 30.0);
        assert_eq!(m[15 * 30 + 15], 0);
        assert_eq!(m[8 * 30 + 8], 255);
        let grad = RgbaImage::from_fn(40, 60, |_, y| Rgba([0, 0, (y * 4) as u8, 255]));
        let inner = image::imageops::crop_imm(&grad, 0, 10, 40, 40).to_image();
        let s = smooth_expand(&grad, &inner, 0, 10);
        assert_eq!(s.get_pixel(20, 30), grad.get_pixel(20, 30));
        let (top, edge, bottom) = (s.get_pixel(20, 2).0[2], s.get_pixel(20, 10).0[2], s.get_pixel(20, 58).0[2]);
        assert!(top < edge && edge < bottom, "{top} {edge} {bottom}");
        assert!((s.get_pixel(20, 9).0[2] as i32 - edge as i32).abs() <= 6);
    }
}

