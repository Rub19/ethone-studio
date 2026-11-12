use image::{Rgba, RgbaImage};

const R: i32 = 3;

#[derive(Clone)]
struct Level {
    w: i32,
    h: i32,
    px: Vec<[f32; 4]>,
    hole: Vec<bool>,
}

impl Level {
    fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }

    fn down(&self) -> Level {
        let (w, h) = ((self.w + 1) / 2, (self.h + 1) / 2);
        let mut px = vec![[0.0; 4]; (w * h) as usize];
        let mut hole = vec![false; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0.0f32; 4];
                let mut n = 0.0;
                let mut any_hole = false;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (sx, sy) = (2 * x + dx, 2 * y + dy);
                    if sx < self.w && sy < self.h {
                        let i = self.idx(sx, sy);
                        if self.hole[i] {
                            any_hole = true;
                        } else {
                            for k in 0..4 {
                                acc[k] += self.px[i][k];
                            }
                            n += 1.0;
                        }
                    }
                }
                let i = (y * w + x) as usize;
                hole[i] = any_hole;
                if n > 0.0 {
                    px[i] = acc.map(|v| v / n);
                }
            }
        }
        Level { w, h, px, hole }
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn diffuse(l: &mut Level) {
    let holes: Vec<usize> = (0..l.px.len()).filter(|&i| l.hole[i]).collect();
    let mut known: Vec<bool> = l.hole.iter().map(|h| !h).collect();
    for _ in 0..(l.w.max(l.h) as usize * 2).max(8) {
        let mut changed = false;
        let snapshot = l.px.clone();
        let known_snap = known.clone();
        for &i in &holes {
            let (x, y) = ((i as i32) % l.w, (i as i32) / l.w);
            let mut acc = [0.0f32; 4];
            let mut n = 0.0;
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (-1, 1), (1, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < l.w && ny < l.h {
                    let j = l.idx(nx, ny);
                    if known_snap[j] {
                        for k in 0..4 {
                            acc[k] += snapshot[j][k];
                        }
                        n += 1.0;
                    }
                }
            }
            if n > 0.0 {
                l.px[i] = acc.map(|v| v / n);
                if !known[i] {
                    known[i] = true;
                    changed = true;
                }
            }
        }
        if !changed && holes.iter().all(|&i| known[i]) {
            break;
        }
    }
}

fn patch_dist(l: &Level, tx: i32, ty: i32, sx: i32, sy: i32, best: f32) -> f32 {
    let mut d = 0.0;
    for dy in -R..=R {
        let (ty2, sy2) = (ty + dy, sy + dy);
        if ty2 < 0 || ty2 >= l.h {
            continue;
        }
        for dx in -R..=R {
            let tx2 = tx + dx;
            if tx2 < 0 || tx2 >= l.w {
                continue;
            }
            let a = l.px[l.idx(tx2, ty2)];
            let b = l.px[l.idx(sx + dx, sy2)];
            let e = [a[0] - b[0], a[1] - b[1], a[2] - b[2], (a[3] - b[3]) * 0.5];
            d += e[0] * e[0] + e[1] * e[1] + e[2] * e[2] + e[3] * e[3];
        }
        if d > best {
            return d;
        }
    }
    d
}

fn valid_sources(l: &Level) -> (Vec<bool>, Vec<(i32, i32)>) {
    let mut ok = vec![false; l.px.len()];
    let mut list = Vec::new();
    for y in R..l.h - R {
        for x in R..l.w - R {
            let mut good = true;
            'p: for dy in -R..=R {
                for dx in -R..=R {
                    if l.hole[l.idx(x + dx, y + dy)] {
                        good = false;
                        break 'p;
                    }
                }
            }
            if good {
                ok[l.idx(x, y)] = true;
                list.push((x, y));
            }
        }
    }
    (ok, list)
}

fn solve(l: &mut Level, init: Option<&[(i32, i32)]>, iters: usize, rng: &mut Rng) -> Vec<(i32, i32)> {
    let targets: Vec<(i32, i32)> = (0..l.px.len()).filter(|&i| l.hole[i]).map(|i| ((i as i32) % l.w, (i as i32) / l.w)).collect();
    let (ok, sources) = valid_sources(l);
    if sources.is_empty() || targets.is_empty() {
        return Vec::new();
    }
    let clamp_src = |p: (i32, i32)| -> (i32, i32) { (p.0.clamp(R, l.w - R - 1), p.1.clamp(R, l.h - R - 1)) };
    let mut nnf: Vec<(i32, i32)> = targets
        .iter()
        .enumerate()
        .map(|(k, _)| {
            let c = init.and_then(|v| v.get(k).copied()).map(clamp_src);
            match c {
                Some(p) if ok[l.idx(p.0, p.1)] => p,
                _ => sources[rng.below(sources.len())],
            }
        })
        .collect();
    let mut cost: Vec<f32> = targets.iter().zip(&nnf).map(|(t, s)| patch_dist(l, t.0, t.1, s.0, s.1, f32::MAX)).collect();
    let index: std::collections::HashMap<(i32, i32), usize> = targets.iter().enumerate().map(|(k, t)| (*t, k)).collect();
    let max_r = l.w.max(l.h);
    for it in 0..iters {
        for pass in 0..2 {
            let order: Box<dyn Iterator<Item = usize>> = if (it + pass) % 2 == 0 { Box::new(0..targets.len()) } else { Box::new((0..targets.len()).rev()) };
            let step = if (it + pass) % 2 == 0 { -1 } else { 1 };
            for k in order {
                let (tx, ty) = targets[k];
                for (nx, ny) in [(tx + step, ty), (tx, ty + step)] {
                    if let Some(&nk) = index.get(&(nx, ny)) {
                        let cand = (nnf[nk].0 - (nx - tx), nnf[nk].1 - (ny - ty));
                        if cand.0 >= R && cand.1 >= R && cand.0 < l.w - R && cand.1 < l.h - R && ok[l.idx(cand.0, cand.1)] {
                            let d = patch_dist(l, tx, ty, cand.0, cand.1, cost[k]);
                            if d < cost[k] {
                                cost[k] = d;
                                nnf[k] = cand;
                            }
                        }
                    }
                }
                let mut rad = max_r;
                while rad >= 1 {
                    let cx = nnf[k].0 + (rng.below((2 * rad + 1) as usize) as i32 - rad);
                    let cy = nnf[k].1 + (rng.below((2 * rad + 1) as usize) as i32 - rad);
                    if cx >= R && cy >= R && cx < l.w - R && cy < l.h - R && ok[l.idx(cx, cy)] {
                        let d = patch_dist(l, tx, ty, cx, cy, cost[k]);
                        if d < cost[k] {
                            cost[k] = d;
                            nnf[k] = (cx, cy);
                        }
                    }
                    rad /= 2;
                }
            }
        }
        let mut sorted = cost.clone();
        sorted.sort_by(f32::total_cmp);
        let sigma2 = sorted.get(sorted.len() * 3 / 4).copied().unwrap_or(1.0).max(1.0);
        let mut acc = vec![[0.0f32; 5]; targets.len()];
        for (k, &(tx, ty)) in targets.iter().enumerate() {
            let w = (-cost[k] / (2.0 * sigma2)).exp().max(1e-6);
            for dy in -R..=R {
                for dx in -R..=R {
                    if let Some(&q) = index.get(&(tx + dx, ty + dy)) {
                        let s = l.px[l.idx(nnf[k].0 + dx, nnf[k].1 + dy)];
                        for c in 0..4 {
                            acc[q][c] += s[c] * w;
                        }
                        acc[q][4] += w;
                    }
                }
            }
        }
        for (q, &(x, y)) in targets.iter().enumerate() {
            if acc[q][4] > 0.0 {
                let i = l.idx(x, y);
                l.px[i] = [acc[q][0] / acc[q][4], acc[q][1] / acc[q][4], acc[q][2] / acc[q][4], acc[q][3] / acc[q][4]];
            }
        }
        cost = targets.iter().zip(&nnf).map(|(t, s)| patch_dist(l, t.0, t.1, s.0, s.1, f32::MAX)).collect();
    }
    nnf
}

/// Removes the faint edge where the fill meets the original: the colour step along the hole's
/// border is spread smoothly across the hole (a membrane), so the fill matches its surroundings.
fn blend_seams(l: &mut Level) {
    let n = l.px.len();
    let mut diff = Level { w: l.w, h: l.h, px: vec![[0.0; 4]; n], hole: vec![false; n] };
    for i in 0..n {
        if !l.hole[i] {
            continue;
        }
        let (x, y) = ((i as i32) % l.w, (i as i32) / l.w);
        let mut acc = [0.0f32; 4];
        let mut k = 0.0;
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && ny >= 0 && nx < l.w && ny < l.h && !l.hole[l.idx(nx, ny)] {
                let p = l.px[l.idx(nx, ny)];
                for c in 0..4 {
                    acc[c] += p[c];
                }
                k += 1.0;
            }
        }
        if k > 0.0 {
            diff.px[i] = [acc[0] / k - l.px[i][0], acc[1] / k - l.px[i][1], acc[2] / k - l.px[i][2], acc[3] / k - l.px[i][3]];
        } else {
            diff.hole[i] = true;
        }
    }
    diffuse(&mut diff);
    for i in 0..n {
        if l.hole[i] {
            for c in 0..4 {
                l.px[i][c] += diff.px[i][c] * 0.85;
            }
        }
    }
}

/// Fills the `mask`ed pixels of `img` from the surrounding content, so what was painted over
/// disappears and the background (sky, grass, wall…) carries on through it.
pub fn content_aware_fill(img: &RgbaImage, mask: &[bool], seed: u64) -> RgbaImage {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let mut out = img.clone();
    let grown: Vec<bool> = (0..(w * h) as usize)
        .map(|i| {
            let (x, y) = ((i as i32) % w, (i as i32) / w);
            (-2..=2).any(|dy| (-2..=2).any(|dx| {
                let (nx, ny) = (x + dx, y + dy);
                nx >= 0 && ny >= 0 && nx < w && ny < h && mask[(ny * w + nx) as usize]
            }))
        })
        .collect();
    let mask = &grown[..];
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, -1, -1);
    for y in 0..h {
        for x in 0..w {
            if mask[(y * w + x) as usize] {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    if x1 < 0 {
        return out;
    }
    let margin = ((x1 - x0).max(y1 - y0) * 2).clamp(40, 400);
    let (rx0, ry0) = ((x0 - margin).max(0), (y0 - margin).max(0));
    let (rx1, ry1) = ((x1 + margin).min(w - 1), (y1 + margin).min(h - 1));
    let (rw, rh) = (rx1 - rx0 + 1, ry1 - ry0 + 1);
    let mut base = Level { w: rw, h: rh, px: Vec::with_capacity((rw * rh) as usize), hole: Vec::with_capacity((rw * rh) as usize) };
    for y in ry0..=ry1 {
        for x in rx0..=rx1 {
            let p = img.get_pixel(x as u32, y as u32).0;
            base.px.push(p.map(|v| v as f32));
            base.hole.push(mask[(y * w + x) as usize]);
        }
    }
    let mut pyramid = vec![base];
    while let Some(top) = pyramid.last() {
        let holes = top.hole.iter().filter(|h| **h).count();
        if top.w.min(top.h) < 24 || holes < 30 || pyramid.len() > 7 {
            break;
        }
        let next = top.down();
        if next.w.min(next.h) <= 2 * R + 2 {
            break;
        }
        pyramid.push(next);
    }
    let mut rng = Rng(seed | 1);
    let top = pyramid.len() - 1;
    diffuse(&mut pyramid[top]);
    let mut nnf: Option<(Vec<(i32, i32)>, i32, Vec<(i32, i32)>)> = None;
    for lv in (0..pyramid.len()).rev() {
        if lv < top {
            let (coarse, cw, ct) = nnf.take().unwrap_or_default();
            let coarse_px = pyramid[lv + 1].px.clone();
            let cwid = pyramid[lv + 1].w;
            let l = &mut pyramid[lv];
            for y in 0..l.h {
                for x in 0..l.w {
                    let i = l.idx(x, y);
                    if l.hole[i] {
                        let (cx, cy) = ((x / 2).min(cwid - 1), (y / 2).min((coarse_px.len() as i32 / cwid) - 1));
                        l.px[i] = coarse_px[(cy * cwid + cx) as usize];
                    }
                }
            }
            let index: std::collections::HashMap<(i32, i32), usize> = ct.iter().enumerate().map(|(k, t)| (*t, k)).collect();
            let targets: Vec<(i32, i32)> = (0..l.px.len()).filter(|&i| l.hole[i]).map(|i| ((i as i32) % l.w, (i as i32) / l.w)).collect();
            let init: Vec<(i32, i32)> = targets
                .iter()
                .map(|&(x, y)| index.get(&(x / 2, y / 2)).and_then(|&k| coarse.get(k)).map_or((-1, -1), |s| (s.0 * 2 + (x % 2), s.1 * 2 + (y % 2))))
                .collect();
            let _ = cw;
            let iters = if lv == 0 { 3 } else { 5 };
            let f = solve(l, Some(&init), iters, &mut rng);
            nnf = Some((f, l.w, targets));
        } else {
            let l = &mut pyramid[lv];
            let targets: Vec<(i32, i32)> = (0..l.px.len()).filter(|&i| l.hole[i]).map(|i| ((i as i32) % l.w, (i as i32) / l.w)).collect();
            let f = solve(l, None, 7, &mut rng);
            nnf = Some((f, l.w, targets));
        }
    }
    blend_seams(&mut pyramid[0]);
    let l = &pyramid[0];
    for y in 0..rh {
        for x in 0..rw {
            let i = l.idx(x, y);
            if l.hole[i] {
                let p = l.px[i].map(|v| v.round().clamp(0.0, 255.0) as u8);
                out.put_pixel((rx0 + x) as u32, (ry0 + y) as u32, Rgba(p));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn visual_cloudy_sky() {
        let Ok(dir) = std::env::var("INPAINT_OUT") else { return };
        let (w, h) = (640u32, 400u32);
        let noise = |x: f32, y: f32| -> f32 {
            let mut v = 0.0;
            let mut amp = 1.0;
            let mut f = 1.0 / 90.0;
            for o in 0..5 {
                let (xi, yi) = ((x * f).floor(), (y * f).floor());
                let (xf, yf) = (x * f - xi, y * f - yi);
                let hash = |a: f32, b: f32| ((a * 127.1 + b * 311.7 + o as f32 * 17.0).sin() * 43758.547).fract().abs();
                let s = |t: f32| t * t * (3.0 - 2.0 * t);
                let top = hash(xi, yi) + (hash(xi + 1.0, yi) - hash(xi, yi)) * s(xf);
                let bot = hash(xi, yi + 1.0) + (hash(xi + 1.0, yi + 1.0) - hash(xi, yi + 1.0)) * s(xf);
                v += (top + (bot - top) * s(yf)) * amp;
                amp *= 0.5;
                f *= 2.0;
            }
            v / 1.9
        };
        let mut photo = RgbaImage::from_fn(w, h, |x, y| {
            let c = (noise(x as f32, y as f32) - 0.45).max(0.0) * 2.2;
            let base = [70.0 + y as f32 * 0.25, 130.0 + y as f32 * 0.2, 225.0];
            Rgba([(base[0] + (255.0 - base[0]) * c.min(1.0)) as u8, (base[1] + (255.0 - base[1]) * c.min(1.0)) as u8, (base[2] + (255.0 - base[2]) * c.min(1.0)) as u8, 255])
        });
        let mut mask = vec![false; (w * h) as usize];
        for (bx, by) in [(150i32, 90i32), (300, 140), (420, 80), (520, 200), (230, 260)] {
            for t in -14..=14 {
                let x = bx + t;
                let y = by + (t.abs() as f32 * 0.45) as i32 - 6;
                for d in -2..=2 {
                    photo.put_pixel(x as u32, (y + d) as u32, Rgba([25, 25, 30, 255]));
                }
            }
            for y in by - 12..by + 12 {
                for x in bx - 18..bx + 18 {
                    mask[(y as u32 * w + x as u32) as usize] = true;
                }
            }
        }
        photo.save(format!("{dir}/sky-before.png")).unwrap();
        content_aware_fill(&photo, &mask, 3).save(format!("{dir}/sky-after.png")).unwrap();
    }

    #[test]
    fn removes_birds_from_a_sky() {
        let (w, h) = (160u32, 110u32);
        let sky = |y: u32| [(90.0 + y as f32 * 0.8) as u8, (150.0 + y as f32 * 0.5) as u8, 235u8, 255u8];
        let clean = RgbaImage::from_fn(w, h, |_, y| Rgba(sky(y)));
        let mut photo = clean.clone();
        let mut mask = vec![false; (w * h) as usize];
        for (bx, by) in [(40u32, 30u32), (95, 55), (120, 25)] {
            for y in by - 6..by + 6 {
                for x in bx - 9..bx + 9 {
                    photo.put_pixel(x, y, Rgba([20, 20, 25, 255]));
                }
            }
            for y in by - 8..by + 8 {
                for x in bx - 11..bx + 11 {
                    mask[(y * w + x) as usize] = true;
                }
            }
        }
        let fixed = content_aware_fill(&photo, &mask, 7);
        let mut worst = 0u8;
        for y in 0..h {
            for x in 0..w {
                if mask[(y * w + x) as usize] {
                    let a = fixed.get_pixel(x, y).0;
                    let b = clean.get_pixel(x, y).0;
                    for k in 0..3 {
                        worst = worst.max(a[k].abs_diff(b[k]));
                    }
                }
            }
        }
        assert!(worst <= 12, "écart max {worst}");
        assert_eq!(fixed.get_pixel(5, 5).0, photo.get_pixel(5, 5).0);
    }
}
