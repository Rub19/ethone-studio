use crate::Pixel;
use crate::doc::Area;
use eframe::egui;
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

impl Pixel {
    /// Quick action "Supprimer l'arrière-plan": the background touching the image edges is hidden
    /// with a layer mask (nothing is erased, paint the mask to fix it).
    pub fn remove_background(&mut self) {
        let (w, h) = (self.doc.width(), self.doc.height());
        let Some(all) = Area::from_points((0.0, 0.0), (w as f32, h as f32), w, h) else { return };
        let mask = object_mask(&self.composite, all, 20.0 + self.tolerance as f32 * 1.2);
        let kept = mask.iter().filter(|m| **m > 0).count();
        if kept == 0 || kept == mask.len() {
            self.error = Some("Arrière-plan introuvable : il doit toucher les bords et être assez uni".into());
            return;
        }
        let m = RgbaImage::from_fn(w, h, |x, y| {
            let v = mask[(y * w + x) as usize];
            Rgba([v, v, v, 255])
        });
        self.doc.next_label = Some("Supprimer l'arrière-plan".into());
        self.doc.checkpoint();
        let l = &mut self.doc.layers[self.doc.active];
        l.mask = Some(m);
        l.mask_on = true;
        self.doc.touch_all();
    }

    /// Photoshop's Generative Expand, done with content-aware fill: grows the canvas and rebuilds the new border.
    pub fn expand_image(&mut self, margin: f32, aspect: Option<f32>, ctx: &egui::Context) {
        let (w, h) = (self.doc.width(), self.doc.height());
        let (nw, nh, ox, oy) = expand_size(w, h, margin, aspect);
        if (nw, nh) == (w, h) {
            self.error = Some("L'image a déjà ce format".into());
            return;
        }
        self.doc.next_label = Some("Étendre l'image".into());
        self.doc.checkpoint();
        self.doc.canvas_size(nw, nh, (0.5, 0.5), None);
        let mask: Vec<bool> = (0..nw * nh).map(|i| {
            let (x, y) = (i % nw, i / nw);
            x < ox || y < oy || x >= ox + w || y >= oy + h
        }).collect();
        self.fit_pending = true;
        self.start_fill(mask, ctx);
    }

    /// Matches the active layer's colours and contrast to what lies beneath it (Photoshop's Harmonize).
    pub fn harmonize_layer(&mut self, amount: f32) {
        let layer = self.doc.active;
        if self.doc.layers.len() < 2 || layer == 0 {
            self.error = Some("Harmoniser : place un calque au-dessus d'une image".into());
            return;
        }
        let src = self.doc.layers[layer].img.clone();
        let below = self.doc.layers[..layer].iter().filter(|l| l.visible).fold(None::<RgbaImage>, |acc, l| {
            let mut base = acc.unwrap_or_else(|| RgbaImage::new(l.img.width(), l.img.height()));
            image::imageops::overlay(&mut base, &l.img, 0, 0);
            Some(base)
        });
        let Some(below) = below else { return };
        let from = color_stats(&src, |x, y| src.get_pixel(x, y).0[3] > 128);
        let to = color_stats(&below, |x, y| below.get_pixel(x, y).0[3] > 128);
        self.doc.next_label = Some("Harmoniser".into());
        self.doc.checkpoint();
        self.doc.layers[layer].img = harmonize(&src, from, to, amount);
        self.doc.touch_all();
    }

    pub fn object_select(&mut self) {
        let Some(a) = self.doc.selection else { return };
        let prev = self.sel_prev.take();
        let mask = object_mask(&self.composite, a, 20.0 + self.tolerance as f32 * 1.2);
        if mask.iter().any(|m| *m > 0) {
            self.doc.select_mask(mask);
            self.doc.feather_selection(self.feather);
        }
        self.doc.combine_selection(prev, self.sel_mode);
    }

    pub fn fix_red_eye(&mut self, x: f32, y: f32) {
        let base = self.doc.active_target();
        let r = self.brush.radius.max(4.0);
        let out = red_eye(&base, x, y, r, self.red_eye_dark);
        self.doc.next_label = Some("Yeux rouges".into());
        self.doc.checkpoint();
        self.doc.put_filtered(&base, &out);
    }

    pub fn content_move(&mut self, offset: (f32, f32), ctx: &egui::Context) {
        if self.doc.selection.is_none() {
            return;
        }
        let (w, h) = (self.doc.width(), self.doc.height());
        let (dx, dy) = (offset.0.round() as i64, offset.1.round() as i64);
        let sel: Vec<bool> = (0..w * h).map(|i| self.doc.selected(i % w, i / w)).collect();
        let layer = self.doc.active;
        let base = self.doc.layers[layer].img.clone();
        let mut out = base.clone();
        let mut dest = vec![false; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                if !sel[(y * w + x) as usize] {
                    continue;
                }
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                    out.put_pixel(nx as u32, ny as u32, *base.get_pixel(x, y));
                    dest[(ny as u32 * w + nx as u32) as usize] = true;
                }
            }
        }
        self.doc.next_label = Some("Déplacement basé sur le contenu".into());
        self.doc.checkpoint();
        self.doc.layers[layer].img = out;
        self.doc.touch_all();
        let hole: Vec<bool> = sel.iter().zip(&dest).map(|(s, d)| *s && !*d).collect();
        self.doc.set_selection(None, None);
        self.start_fill(hole, ctx);
    }

    pub fn perspective_crop(&mut self) {
        if self.persp.len() != 4 {
            return;
        }
        let q = [self.persp[0], self.persp[1], self.persp[2], self.persp[3]];
        let len = |a: (f32, f32), b: (f32, f32)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let ow = ((len(q[0], q[1]) + len(q[3], q[2])) / 2.0).round().max(1.0) as u32;
        let oh = ((len(q[0], q[3]) + len(q[1], q[2])) / 2.0).round().max(1.0) as u32;
        self.persp.clear();
        self.doc.set_selection(None, None);
        self.doc.next_label = Some("Recadrage de la perspective".into());
        self.doc.transform(|img| warp_quad(img, q, ow, oh));
        self.fit_pending = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_sizes() {
        assert_eq!(expand_size(100, 50, 0.1, None), (120, 60, 10, 5));
        assert_eq!(expand_size(160, 90, 0.0, Some(1.0)), (160, 160, 0, 35));
        assert_eq!(expand_size(90, 160, 0.0, Some(16.0 / 9.0)), (284, 160, 97, 0));
        assert_eq!(expand_size(100, 100, 0.0, Some(1.0)), (100, 100, 0, 0));
    }

    #[test]
    fn harmonize_pulls_toward_target() {
        let src = RgbaImage::from_pixel(4, 4, Rgba([200, 60, 60, 255]));
        let from = color_stats(&src, |_, _| true);
        let to = ([60.0, 90.0, 200.0], [10.0, 10.0, 10.0]);
        let out = harmonize(&src, from, to, 1.0);
        assert_eq!(out.get_pixel(0, 0).0, [60, 90, 200, 255]);
        let half = harmonize(&src, from, to, 0.5);
        assert_eq!(half.get_pixel(0, 0).0, [130, 75, 130, 255]);
    }

    #[test]
    fn object_red_eye_warp_curve() {
        let mut img = RgbaImage::from_pixel(40, 40, Rgba([240, 240, 240, 255]));
        for y in 15..25 {
            for x in 12..28 {
                img.put_pixel(x, y, Rgba([200, 20, 20, 255]));
            }
        }
        let a = Area::from_points((5.0, 5.0), (35.0, 35.0), 40, 40).unwrap();
        let m = object_mask(&img, a, 30.0);
        assert_eq!(m[20 * 40 + 20], 255);
        assert_eq!(m[8 * 40 + 8], 0);
        assert_eq!(m.iter().filter(|v| **v > 0).count(), 160);
        let fixed = red_eye(&img, 20.0, 20.0, 6.0, 0.5);
        assert!(fixed.get_pixel(20, 20).0[0] < 60);
        assert_eq!(fixed.get_pixel(2, 2), img.get_pixel(2, 2));
        let wq = warp_quad(&img, [(12.0, 15.0), (28.0, 15.0), (28.0, 25.0), (12.0, 25.0)], 16, 10);
        assert!(wq.pixels().all(|p| p.0[0] == 200));
        let c = catmull(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], 8);
        assert_eq!(c.len(), 32);
        assert_eq!(c[0], (0.0, 0.0));
    }
}
