use crate::model::{Clip, FONTS, Mask, Title};
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

pub struct Layer<'a> {
    pub px: &'a [u8],
    pub sw: u32,
    pub sh: u32,
    pub clip: &'a Clip,
    pub progress: f32,
    pub seed: u32,
}

pub struct Frame<'a> {
    pub layer: Option<Layer<'a>>,
    pub from: Option<(Layer<'a>, f32, u8)>,
    pub overlays: &'a [Layer<'a>],
    pub fade: f32,
    pub white: bool,
    pub background: [u8; 4],
    pub titles: &'a [&'a Title],
    pub t: f64,
}

fn filled(w: u32, h: u32, bg: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        out.extend_from_slice(&[bg[0], bg[1], bg[2], 255]);
    }
    out
}

pub fn compose(w: u32, h: u32, f: &Frame) -> Vec<u8> {
    let mut out = filled(w, h, f.background);
    if let Some(l) = &f.layer {
        draw_layer(&mut out, w, h, l);
    }
    if let Some((a, p, kind)) = &f.from {
        let mut prev = filled(w, h, f.background);
        draw_layer(&mut prev, w, h, a);
        out = transition(&prev, &out, w, h, *p, *kind);
    }
    for o in f.overlays {
        draw_layer(&mut out, w, h, o);
    }
    for t in f.titles {
        draw_title(&mut out, w, h, t, f.t);
    }
    if f.fade < 1.0 {
        let target = if f.white { 255.0 } else { 0.0 };
        for px in out.chunks_exact_mut(4) {
            for v in &mut px[..3] {
                *v = (target + (*v as f32 - target) * f.fade).round() as u8;
            }
        }
    }
    out
}

struct Keyer {
    cb: f32,
    cr: f32,
    tol: f32,
    soft: f32,
    dom: usize,
}

impl Keyer {
    fn of(c: &Clip) -> Self {
        let k = c.key_color.map(|v| v as f32 / 255.0);
        let (cb, cr) = chroma(&k);
        let dom = (0..3).max_by(|a, b| k[*a].total_cmp(&k[*b])).unwrap_or(1);
        Keyer { cb, cr, tol: c.key_tol / 100.0 * 0.45, soft: (c.key_soft / 100.0 * 0.45).max(1e-3), dom }
    }

    fn alpha(&self, p: &[f32; 3]) -> f32 {
        let (cb, cr) = chroma(p);
        let d = ((cb - self.cb).powi(2) + (cr - self.cr).powi(2)).sqrt();
        ((d - self.tol) / self.soft).clamp(0.0, 1.0)
    }

    fn despill(&self, p: &mut [f32; 3]) {
        let others: f32 = (0..3).filter(|k| *k != self.dom).map(|k| p[k]).fold(0.0, f32::max);
        p[self.dom] = p[self.dom].min(others);
    }
}

fn chroma(p: &[f32; 3]) -> (f32, f32) {
    let y = 0.299 * p[0] + 0.587 * p[1] + 0.114 * p[2];
    ((p[2] - y) * 0.564, (p[0] - y) * 0.713)
}

struct MaskEval {
    kind: u8,
    pts: Vec<[f32; 2]>,
    feather: f32,
    invert: bool,
}

impl MaskEval {
    fn new(m: &Mask, w: u32, h: u32) -> Self {
        let pts = m.pts.iter().map(|p| [p[0] * w as f32, p[1] * h as f32]).collect();
        MaskEval { kind: m.kind, pts, feather: m.feather / 100.0 * h as f32 * 0.5, invert: m.invert }
    }

    fn alpha(&self, x: f32, y: f32) -> f32 {
        let sd = match self.kind {
            0 | 1 => {
                let (a, b) = (self.pts[0], self.pts[1]);
                let (cx, cy) = ((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0);
                let (hx, hy) = (((a[0] - b[0]) / 2.0).abs().max(1.0), ((a[1] - b[1]) / 2.0).abs().max(1.0));
                let (qx, qy) = ((x - cx).abs(), (y - cy).abs());
                if self.kind == 0 {
                    let (ox, oy) = (qx - hx, qy - hy);
                    (ox.max(0.0).powi(2) + oy.max(0.0).powi(2)).sqrt() + ox.max(oy).min(0.0)
                } else {
                    ((qx / hx).powi(2) + (qy / hy).powi(2)).sqrt().mul_add(1.0, -1.0) * hx.min(hy)
                }
            }
            _ => {
                let n = self.pts.len();
                let mut inside = false;
                let mut best = f32::MAX;
                for k in 0..n {
                    let (a, b) = (self.pts[k], self.pts[(k + 1) % n]);
                    if (a[1] > y) != (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0] {
                        inside = !inside;
                    }
                    if self.feather > 0.0 {
                        let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
                        let u = (((x - a[0]) * ex + (y - a[1]) * ey) / (ex * ex + ey * ey).max(1e-6)).clamp(0.0, 1.0);
                        best = best.min((x - a[0] - ex * u).powi(2) + (y - a[1] - ey * u).powi(2));
                    }
                }
                let d = if self.feather > 0.0 { best.sqrt() } else { 0.5 };
                if inside { -d } else { d }
            }
        };
        let a = if self.feather > 0.0 { (0.5 - sd / self.feather).clamp(0.0, 1.0) } else if sd <= 0.0 { 1.0 } else { 0.0 };
        if self.invert { 1.0 - a } else { a }
    }
}

fn draw_layer(out: &mut [u8], w: u32, h: u32, l: &Layer) {
    let (sw, sh, c) = (l.sw, l.sh, l.clip);
    if sw == 0 || sh == 0 || l.px.len() < (sw * sh * 4) as usize {
        return;
    }
    let rgba = l.px;
    let zoom = if c.zoom_end > 0.0 { c.zoom + (c.zoom_end - c.zoom) * l.progress.clamp(0.0, 1.0) } else { c.zoom };
    let s = (w as f32 / sw as f32).min(h as f32 / sh as f32) * zoom.max(0.01);
    let (cx, cy) = (w as f32 / 2.0 + c.pos_x * w as f32 / 2.0, h as f32 / 2.0 + c.pos_y * h as f32 / 2.0);
    let (sn, cs) = (-c.rotation.to_radians()).sin_cos();
    let crop = c.crop.map(|v| v.clamp(0.0, 0.49));
    let (lx, rx) = (crop[0] * sw as f32, sw as f32 * (1.0 - crop[2]));
    let (ty, by) = (crop[1] * sh as f32, sh as f32 * (1.0 - crop[3]));
    let grade = Grade::of(c);
    let op = c.opacity.clamp(0.0, 1.0);
    let vig = c.vignette / 100.0;
    let keyer = c.key_on.then(|| Keyer::of(c));
    let mask = c.mask.as_ref().filter(|m| m.pts.len() >= 2 && (m.kind < 2 || m.pts.len() >= 3)).map(|m| MaskEval::new(m, w, h));
    let block = if c.pixelate > 0.0 { (1.0 + c.pixelate / 100.0 * w as f32 / 24.0) as u32 } else { 1 };
    let levels = c.posterize.round();
    let mut lay = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let (qx, qy) = if block > 1 { ((x / block) * block + block / 2, (y / block) * block + block / 2) } else { (x, y) };
            let (dx, dy) = (qx as f32 + 0.5 - cx, qy as f32 + 0.5 - cy);
            let (rxp, ryp) = (dx * cs - dy * sn, dx * sn + dy * cs);
            let mut sx = rxp / s + sw as f32 / 2.0;
            let mut sy = ryp / s + sh as f32 / 2.0;
            if sx < lx || sx >= rx || sy < ty || sy >= by {
                continue;
            }
            if c.flip {
                sx = sw as f32 - 1.0 - sx;
            }
            if c.flip_v {
                sy = sh as f32 - 1.0 - sy;
            }
            let si = ((sy.max(0.0) as u32).min(sh - 1) * sw + (sx.max(0.0) as u32).min(sw - 1)) as usize * 4;
            let mut px = [rgba[si] as f32 / 255.0, rgba[si + 1] as f32 / 255.0, rgba[si + 2] as f32 / 255.0];
            let mut a = op * rgba[si + 3] as f32 / 255.0;
            if let Some(k) = &keyer {
                a *= k.alpha(&px);
                if a <= 0.0 {
                    continue;
                }
                k.despill(&mut px);
            }
            if let Some(m) = &mask {
                a *= m.alpha(x as f32 + 0.5, y as f32 + 0.5);
                if a <= 0.0 {
                    continue;
                }
            }
            grade.apply(&mut px);
            if c.invert {
                px = px.map(|v| 1.0 - v.clamp(0.0, 1.0));
            }
            if levels >= 2.0 {
                px = px.map(|v| (v.clamp(0.0, 1.0) * (levels - 1.0)).round() / (levels - 1.0));
            }
            if vig != 0.0 {
                let (vx, vy) = (x as f32 + 0.5 - w as f32 / 2.0, y as f32 + 0.5 - h as f32 / 2.0);
                let d = ((vx / (w as f32 / 2.0)).powi(2) + (vy / (h as f32 / 2.0)).powi(2)) / 2.0;
                let k = 1.0 - vig * d;
                px = px.map(|v| v * k);
            }
            let di = ((y * w + x) * 4) as usize;
            for k in 0..3 {
                lay[di + k] = (px[k].clamp(0.0, 1.0) * a * 255.0).round() as u8;
            }
            lay[di + 3] = (a * 255.0).round() as u8;
        }
    }
    if c.blur > 0.0 {
        box_blur(&mut lay, w, h, ((c.blur / 100.0) * w as f32 / 40.0).round().max(1.0) as u32);
    }
    if c.sharpen > 0.0 {
        let mut soft = lay.clone();
        box_blur(&mut soft, w, h, 1);
        let k = c.sharpen / 100.0 * 2.0;
        for (v, s) in lay.iter_mut().zip(&soft) {
            *v = (*v as f32 + (*v as f32 - *s as f32) * k).clamp(0.0, 255.0) as u8;
        }
    }
    if c.grain > 0.0 {
        let amt = c.grain / 100.0 * 70.0;
        let mut st = l.seed.wrapping_mul(2654435761).wrapping_add(12345);
        for px in lay.chunks_exact_mut(4) {
            st ^= st << 13;
            st ^= st >> 17;
            st ^= st << 5;
            let top = px[3] as f32;
            let n = ((st & 0xffff) as f32 / 65535.0 - 0.5) * amt * top / 255.0;
            for v in &mut px[..3] {
                *v = (*v as f32 + n).clamp(0.0, top) as u8;
            }
        }
    }
    for (o, s) in out.chunks_exact_mut(4).zip(lay.chunks_exact(4)) {
        if s[3] == 0 {
            continue;
        }
        let k = 1.0 - s[3] as f32 / 255.0;
        for i in 0..3 {
            o[i] = (s[i] as f32 + o[i] as f32 * k).round().min(255.0) as u8;
        }
    }
}

fn transition(a: &[u8], b: &[u8], w: u32, h: u32, p: f32, kind: u8) -> Vec<u8> {
    let p = p.clamp(0.0, 1.0);
    let (wf, hf) = (w as f32, h as f32);
    let mut out = vec![0u8; a.len()];
    let px = |buf: &[u8], x: i64, y: i64| -> Option<[u8; 3]> {
        if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
            return None;
        }
        let i = ((y as u32 * w + x as u32) * 4) as usize;
        Some([buf[i], buf[i + 1], buf[i + 2]])
    };
    let mix = |x: [u8; 3], y: [u8; 3], k: f32| [0, 1, 2].map(|i| (x[i] as f32 * (1.0 - k) + y[i] as f32 * k).round() as u8);
    let blurred = (kind == 19).then(|| {
        let r = ((p * std::f32::consts::PI).sin() * wf / 60.0).max(1.0) as u32;
        let (mut x, mut y) = (a.to_vec(), b.to_vec());
        box_blur(&mut x, w, h, r);
        box_blur(&mut y, w, h, r);
        (x, y)
    });
    let (sa, sb): (&[u8], &[u8]) = match &blurred {
        Some((x, y)) => (x, y),
        None => (a, b),
    };
    let soft = |d: f32, edge: f32| ((d / edge) * 0.5 + 0.5).clamp(0.0, 1.0);
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            let pa = px(sa, x, y).unwrap_or([0; 3]);
            let pb = px(sb, x, y).unwrap_or([0; 3]);
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let e = wf.max(hf) * 0.015;
            let v: [u8; 3] = match kind {
                2 | 3 => {
                    let t = if kind == 2 { 0.0 } else { 255.0 };
                    let (src, k) = if p < 0.5 { (pa, p * 2.0) } else { (pb, (1.0 - p) * 2.0) };
                    src.map(|v| (v as f32 + (t - v as f32) * k).round() as u8)
                }
                4..=7 => {
                    let d = match kind {
                        4 => p * (wf + 2.0 * e) - e - fx,
                        5 => fx - (wf + e - p * (wf + 2.0 * e)),
                        6 => p * (hf + 2.0 * e) - e - fy,
                        _ => fy - (hf + e - p * (hf + 2.0 * e)),
                    };
                    mix(pa, pb, soft(d, e))
                }
                8..=11 => {
                    let (ox, oy) = match kind {
                        8 => ((1.0 - p) * wf, 0.0),
                        9 => (-(1.0 - p) * wf, 0.0),
                        10 => (0.0, (1.0 - p) * hf),
                        _ => (0.0, -(1.0 - p) * hf),
                    };
                    px(b, x - ox as i64, y - oy as i64).unwrap_or(pa)
                }
                12..=15 => {
                    let (dx, dy) = match kind {
                        12 => (-wf, 0.0),
                        13 => (wf, 0.0),
                        14 => (0.0, -hf),
                        _ => (0.0, hf),
                    };
                    let (ax, ay) = ((dx * p) as i64, (dy * p) as i64);
                    let (bx, by) = ((-dx * (1.0 - p)) as i64, (-dy * (1.0 - p)) as i64);
                    px(b, x - bx, y - by).or_else(|| px(a, x - ax, y - ay)).unwrap_or([0; 3])
                }
                16 => {
                    let r = p * (wf * wf + hf * hf).sqrt() / 2.0;
                    let d = r - ((fx - wf / 2.0).powi(2) + (fy - hf / 2.0).powi(2)).sqrt();
                    mix(pa, pb, soft(d, e))
                }
                17 => {
                    let d = (p * wf / 2.0 - (fx - wf / 2.0).abs()).min(p * hf / 2.0 - (fy - hf / 2.0).abs());
                    mix(pa, pb, soft(d, e))
                }
                18 => {
                    let za = 1.0 + p * 1.5;
                    let zb = 2.5 - 1.5 * p;
                    let sample = |buf: &[u8], z: f32| px(buf, ((fx - wf / 2.0) / z + wf / 2.0) as i64, ((fy - hf / 2.0) / z + hf / 2.0) as i64).unwrap_or([0; 3]);
                    mix(sample(a, za), sample(b, zb), p)
                }
                20 => {
                    let ang = (fx - wf / 2.0).atan2(-(fy - hf / 2.0));
                    let u = (ang / std::f32::consts::TAU).rem_euclid(1.0);
                    mix(pa, pb, ((p - u) * 40.0 + 0.5).clamp(0.0, 1.0))
                }
                21 => {
                    let d = p * (wf + hf + 2.0 * e) - e - (fx + fy);
                    mix(pa, pb, soft(d, e))
                }
                22 => {
                    let band = (hf / 10.0).max(4.0);
                    let d = p * band - fy.rem_euclid(band);
                    mix(pa, pb, (d + 0.5).clamp(0.0, 1.0))
                }
                23 => {
                    let cell = wf.max(hf) / 8.0;
                    let (cx, cy) = ((fx / cell) as i64, (fy / cell) as i64);
                    let delay = if (cx + cy) % 2 == 0 { 0.0 } else { 0.5 };
                    mix(pa, pb, ((p - delay) * 2.0).clamp(0.0, 1.0))
                }
                24 => {
                    let d = p * wf / 2.0 - (fx - wf / 2.0).abs();
                    mix(pa, pb, soft(d, e))
                }
                25 => {
                    let (ka, kb) = ((2.0 * (1.0 - p)).min(1.0), (2.0 * p).min(1.0));
                    [0, 1, 2].map(|k| (pa[k] as f32 * ka + pb[k] as f32 * kb).min(255.0) as u8)
                }
                _ => mix(pa, pb, p),
            };
            out[i..i + 3].copy_from_slice(&v);
            out[i + 3] = 255;
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
    let (lines, len) = if horizontal { (h, w) } else { (w, h) };
    for line in 0..lines {
        let idx = |k: usize| if horizontal { (line * w + k) * 4 } else { (k * w + line) * 4 };
        let mut acc = [0u32; 4];
        for k in 0..=r.min(len - 1) {
            for c in 0..4 {
                acc[c] += src[idx(k) + c] as u32;
            }
        }
        let mut count = (r + 1).min(len) as u32;
        for k in 0..len {
            for c in 0..4 {
                dst[idx(k) + c] = (acc[c] / count.max(1)) as u8;
            }
            if k + r + 1 < len {
                for c in 0..4 {
                    acc[c] += src[idx(k + r + 1) + c] as u32;
                }
                count += 1;
            }
            if k >= r {
                for c in 0..4 {
                    acc[c] -= src[idx(k - r) + c] as u32;
                }
                count -= 1;
            }
        }
    }
}

struct Grade {
    on: bool,
    exposure: f32,
    bright: f32,
    contrast: f32,
    sat: f32,
    vib: f32,
    temp: f32,
    tint: f32,
    high: f32,
    shadow: f32,
    black: f32,
    white: f32,
    lift: [f32; 3],
    gamma: [f32; 3],
    gain: [f32; 3],
    hue: Option<[[f32; 3]; 3]>,
    look: u8,
    amt: f32,
    lut: Option<Vec<[f32; 256]>>,
}

fn wheel(v: [f32; 2]) -> [f32; 3] {
    [v[0], -0.5 * v[0] + 0.866 * v[1], -0.5 * v[0] - 0.866 * v[1]]
}

pub fn curve_lut(off: &[f32; 5]) -> [f32; 256] {
    let ys: Vec<f32> = (0..5).map(|k| k as f32 / 4.0 + off[k]).collect();
    let at = |k: i32| ys[k.clamp(0, 4) as usize];
    let mut lut = [0f32; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let x = i as f32 / 255.0 * 4.0;
        let k = (x.floor() as i32).min(3);
        let u = x - k as f32;
        let (p0, p1, p2, p3) = (at(k - 1), at(k), at(k + 1), at(k + 2));
        let v = 0.5 * (2.0 * p1 + (-p0 + p2) * u + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u * u + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * u * u * u);
        *slot = v.clamp(0.0, 1.0);
    }
    lut
}

fn hue_matrix(deg: f32) -> [[f32; 3]; 3] {
    let (s, c) = deg.to_radians().sin_cos();
    let k = 1.0 / 3.0;
    let q = (1.0f32 / 3.0).sqrt();
    let a = c + (1.0 - c) * k;
    let b = k * (1.0 - c) - q * s;
    let d = k * (1.0 - c) + q * s;
    [[a, b, d], [d, a, b], [b, d, a]]
}

impl Grade {
    fn of(c: &Clip) -> Self {
        let lut = (c.curves != [[0.0; 5]; 4]).then(|| c.curves.iter().map(curve_lut).collect());
        Grade {
            on: c.graded(),
            exposure: 2f32.powf(c.exposure / 100.0 * 2.0),
            bright: c.brightness / 100.0 * 0.4,
            contrast: 1.0 + c.contrast / 100.0,
            sat: 1.0 + c.saturation / 100.0,
            vib: c.vibrance / 100.0,
            temp: c.temperature / 100.0 * 0.12,
            tint: c.tint / 100.0 * 0.08,
            high: c.highlights / 100.0 * 0.3,
            shadow: c.shadows / 100.0 * 0.3,
            black: -c.blacks / 100.0 * 0.15,
            white: 1.0 - c.whites / 100.0 * 0.15,
            lift: wheel(c.wheels[0]).map(|v| v * 0.15),
            gamma: wheel(c.wheels[1]).map(|v| v * 0.6),
            gain: wheel(c.wheels[2]).map(|v| v * 0.3),
            hue: (c.hue != 0.0).then(|| hue_matrix(c.hue)),
            look: c.look,
            amt: c.look_amt.clamp(0.0, 1.0),
            lut,
        }
    }

    fn apply(&self, p: &mut [f32; 3]) {
        if !self.on {
            return;
        }
        for v in p.iter_mut() {
            *v *= self.exposure;
        }
        p[0] += self.temp;
        p[2] -= self.temp;
        p[1] -= self.tint;
        let span = (self.white - self.black).max(0.05);
        for v in p.iter_mut() {
            *v = (*v - self.black) / span;
        }
        let l = 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
        let lc = l.clamp(0.0, 1.0);
        let lift = self.shadow * (1.0 - lc).powi(2) + self.high * lc.powi(2);
        for (k, v) in p.iter_mut().enumerate() {
            *v += lift;
            *v = (*v + self.bright - 0.5) * self.contrast + 0.5;
            *v = *v * (1.0 + self.gain[k]) + self.lift[k] * (1.0 - *v);
            if self.gamma[k] != 0.0 {
                *v = v.max(0.0).powf(1.0 / (1.0 + self.gamma[k]).max(0.1));
            }
        }
        let l = 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
        let cur = p.iter().cloned().fold(f32::MIN, f32::max) - p.iter().cloned().fold(f32::MAX, f32::min);
        let s = (self.sat * (1.0 + self.vib * (1.0 - cur.clamp(0.0, 1.0)))).max(0.0);
        for v in p.iter_mut() {
            *v = l + (*v - l) * s;
        }
        if let Some(m) = &self.hue {
            let q = *p;
            for k in 0..3 {
                p[k] = m[k][0] * q[0] + m[k][1] * q[1] + m[k][2] * q[2];
            }
        }
        if self.look != 0 && self.amt > 0.0 {
            let q = look(self.look, *p);
            for k in 0..3 {
                p[k] += (q[k] - p[k]) * self.amt;
            }
        }
        if let Some(lut) = &self.lut {
            let read = |t: &[f32; 256], v: f32| {
                let x = v.clamp(0.0, 1.0) * 255.0;
                let i = (x as usize).min(254);
                t[i] + (t[i + 1] - t[i]) * (x - i as f32)
            };
            for k in 0..3 {
                p[k] = read(&lut[k + 1], read(&lut[0], p[k]));
            }
        }
    }
}

fn look(kind: u8, p: [f32; 3]) -> [f32; 3] {
    let l = 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
    let sat = |q: [f32; 3], s: f32| q.map(|v| l + (v - l) * s);
    let con = |q: [f32; 3], c: f32| q.map(|v| (v - 0.5) * c + 0.5);
    match kind {
        1 => [l; 3],
        2 => [l * 1.07 + 0.04, l * 0.95 + 0.02, l * 0.78],
        3 => [p[0] * 0.9 + 0.08, p[1] * 0.85 + 0.06, p[2] * 0.7 + 0.08],
        4 => {
            let shadow = (1.0 - l).powi(2) * 0.06;
            let high = l.powi(2) * 0.05;
            con([p[0] - shadow + high, p[1] + shadow * 0.3, p[2] + shadow - high], 1.12)
        }
        5 => sat(p, 1.35),
        6 => {
            let w = l.clamp(0.0, 1.0);
            con([p[0] + 0.08 * w - 0.06 * (1.0 - w), p[1] + 0.02 * w + 0.02 * (1.0 - w), p[2] - 0.08 * w + 0.08 * (1.0 - w)], 1.08)
        }
        7 => [p[0] * 0.9, p[1] * 0.98, p[2] * 1.1 + 0.03],
        8 => [p[0] * 1.08 + 0.03, p[1] * 1.0 + 0.01, p[2] * 0.85],
        9 => sat(p, 0.7).map(|v| v * 0.85 + 0.1),
        10 => {
            let q = sat(p, 0.4);
            [q[0] * 0.55, q[1] * 0.7, q[2] * 0.95 + 0.04]
        }
        11 => {
            let q = sat(p, 0.45);
            con(q, 1.3)
        }
        12 => {
            let q = sat(p, 0.85);
            [q[0] * 0.95 + 0.1, q[1] * 0.9 + 0.07, q[2] * 0.75 + 0.02]
        }
        13 => con([l; 3], 1.5),
        14 => sat(p, 0.6).map(|v| v * 0.7 + 0.3),
        15 => {
            let q = sat(p, 0.5);
            con([q[0] * 0.85, q[1] * 1.08 + 0.02, q[2] * 0.85], 1.1)
        }
        16 => sat(p, 0.35),
        17 => [p[0] * 1.12 + 0.05, p[1] * 0.95 + 0.02, p[2] * 0.75 + 0.05],
        18 => {
            let g = p[1];
            [g * 0.9 + p[0] * 0.3, (p[0] * 0.6 + 0.2).min(1.0), p[2] * 0.5 + g * 0.3]
        }
        19 => {
            let q = con(p, 0.88);
            [q[0] + 0.04, q[1] + 0.02, q[2] - 0.03]
        }
        _ => p,
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
    let (wf, hf) = (w as f32, h as f32);
    let (alpha, lift, scale, reveal, slide, track) = match t.anim {
        1 => (ramp(0.5), 0.0, 1.0, 1.0, 0.0, 0.0),
        2 => (ramp(0.4), (1.0 - ease(local / 0.6)) * 0.06, 1.0, 1.0, 0.0, 0.0),
        3 => (1.0, 0.0, 1.0, (local / (dur * 0.5)).clamp(0.0, 1.0), 0.0, 0.0),
        4 => (ramp(0.4), 0.0, 0.6 + 0.4 * ease(local / 0.5), 1.0, 0.0, 0.0),
        5 => (ramp(0.3), 0.0, 1.0, 1.0, -(1.0 - ease(local / 0.6)) * 0.3, 0.0),
        6 => (ramp(0.3), 0.0, 1.0, 1.0, (1.0 - ease(local / 0.6)) * 0.3, 0.0),
        7 => (ramp(0.4), -(1.0 - ease(local / 0.6)) * 0.06, 1.0, 1.0, 0.0, 0.0),
        8 => (ramp(0.5), 0.0, 1.0, 1.0, 0.0, (1.0 - ease(local / 1.2)) * 0.6),
        9 => {
            let u = (local / 0.45).clamp(0.0, 1.0);
            let pop = if u < 1.0 { 0.3 + 0.95 * ease(u) - 0.25 * (u * std::f32::consts::PI).sin() * (1.0 - u) } else { 1.0 };
            (ramp(0.25), 0.0, pop.max(0.1), 1.0, 0.0, 0.0)
        }
        _ => (1.0, 0.0, 1.0, 1.0, 0.0, 0.0),
    };
    if alpha <= 0.0 {
        return;
    }
    let size = (t.size * hf * scale).max(6.0);
    let scaled = font.as_scaled(PxScale::from(size));
    let line_h = (scaled.height() + scaled.line_gap()) * (1.0 + t.leading);
    let gap = size * (t.spacing + track);
    let text: String = if t.vertical { t.text.chars().filter(|c| *c != '\n').map(|c| c.to_string()).collect::<Vec<_>>().join("\n") } else { t.text.clone() };
    let total_chars = text.chars().filter(|c| *c != '\n').count();
    let mut budget = (total_chars as f32 * reveal).round() as usize;
    let lines: Vec<String> = text
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
    let advance = |ch: char| scaled.h_advance(scaled.glyph_id(ch)) + gap;
    let full_widths: Vec<f32> = text.lines().map(|l| l.chars().map(advance).sum::<f32>() - if l.is_empty() { 0.0 } else { gap }).collect();
    let block_h = line_h * lines.len() as f32;
    let maxw = full_widths.iter().cloned().fold(0.0, f32::max);
    let progress = (local / dur).clamp(0.0, 1.0);
    let roll_x = if t.roll == 2 { wf - (wf + maxw) * progress - (wf - maxw) / 2.0 } else { 0.0 };
    let (ox, oy) = (t.dx * wf + slide * wf + roll_x, t.dy * hf + lift * hf);
    let mut top = oy + match t.place {
        0 => hf * 0.08,
        1 => (hf - block_h) / 2.0,
        2 => hf * 0.92 - block_h,
        _ => hf * 0.78 - block_h / 2.0,
    };
    if t.roll == 1 {
        top = hf - (hf + block_h) * progress + t.dy * hf;
    }
    let lower_third = t.place == 3;
    let align = if t.align == 0 { if lower_third { 1 } else { 2 } } else { t.align };
    let line_x = |lw: f32| ox + match align {
        1 => wf * 0.06,
        3 => wf * 0.94 - lw,
        _ => (wf - lw) / 2.0,
    };
    if t.band || lower_third {
        let pad = size * 0.4;
        let (x0, x1) = match align {
            1 => (wf * 0.06 - pad + ox, wf * 0.06 + maxw + pad + ox),
            3 => (wf * 0.94 - maxw - pad + ox, wf * 0.94 + pad + ox),
            _ => ((wf - maxw) / 2.0 - pad + ox, (wf + maxw) / 2.0 + pad + ox),
        };
        let (y0, y1) = ((top - pad * 0.6).max(0.0) as u32, ((top + block_h + pad * 0.6).max(0.0) as u32).min(h));
        let bc = t.band_color;
        let ba = bc[3] as f32 / 255.0 * alpha;
        for y in y0..y1 {
            for x in (x0.max(0.0) as u32)..(x1.clamp(0.0, wf) as u32) {
                let i = ((y * w + x) * 4) as usize;
                for k in 0..3 {
                    out[i + k] = (out[i + k] as f32 * (1.0 - ba) + bc[k] as f32 * ba) as u8;
                }
            }
        }
    }
    let rgb = [t.color[0], t.color[1], t.color[2]];
    let oc = [t.outline_color[0], t.outline_color[1], t.outline_color[2]];
    let mut passes: Vec<(f32, f32, [u8; 3], f32)> = Vec::new();
    if t.shadow {
        passes.push((size * 0.05, size * 0.05, [0, 0, 0], 0.6));
    }
    if t.outline > 0.0 {
        let r = t.outline / 100.0 * size * 0.12;
        for k in 0..12 {
            let a = k as f32 * std::f32::consts::TAU / 12.0;
            passes.push((a.cos() * r, a.sin() * r, oc, t.outline_color[3] as f32 / 255.0));
        }
    }
    passes.push((0.0, 0.0, rgb, 1.0));
    for (offx, offy, col, strength) in passes {
        for (li, line) in lines.iter().enumerate() {
            let base = top + offy + scaled.ascent() + li as f32 * line_h;
            if base + size < 0.0 || base - size > hf {
                continue;
            }
            let mut x = line_x(full_widths[li]) + offx;
            for ch in line.chars() {
                let id = scaled.glyph_id(ch);
                let g = id.with_scale_and_position(size, point(x, base));
                x += scaled.h_advance(id) + gap;
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
                        out[i + k] = (out[i + k] as f32 * (1.0 - a) + col[k] as f32 * a).round() as u8;
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer<'a>(px: &'a [u8], sw: u32, sh: u32, clip: &'a Clip) -> Layer<'a> {
        Layer { px, sw, sh, clip, progress: 0.0, seed: 1 }
    }

    fn frame<'a>(l: Option<Layer<'a>>, fade: f32, white: bool, titles: &'a [&'a Title]) -> Frame<'a> {
        Frame { layer: l, from: None, overlays: &[], fade, white, background: [0, 0, 0, 255], titles, t: 0.5 }
    }

    #[test]
    fn compose_places_grades_fades_and_titles() {
        let src = [200u8, 200, 200, 255].repeat(16);
        let def = Clip::default();
        let plain = compose(8, 4, &frame(Some(layer(&src, 4, 4, &def)), 1.0, false, &[]));
        assert_eq!(&plain[0..4], &[0, 0, 0, 255]);
        assert_eq!(plain[((8 + 4) * 4) as usize], 200);
        let mono = Clip { look: 1, saturation: 50.0, ..Default::default() };
        let px = [200, 50, 50, 255].repeat(16);
        let g = compose(4, 4, &frame(Some(layer(&px, 4, 4, &mono)), 1.0, false, &[]));
        assert!(g[0] == g[1] && g[1] == g[2]);
        assert_eq!(compose(4, 4, &frame(Some(layer(&src, 4, 4, &def)), 0.0, true, &[]))[0], 255);
        let half = Clip { opacity: 0.5, ..Default::default() };
        assert_eq!(compose(4, 4, &Frame { background: [100, 100, 100, 255], ..frame(Some(layer(&src, 4, 4, &half)), 1.0, false, &[]) })[0], 150);
        let moved = Clip { pos_x: 1.0, ..Default::default() };
        let m = compose(8, 4, &frame(Some(layer(&src, 4, 4, &moved)), 1.0, false, &[]));
        assert_eq!(m[((8 + 2) * 4) as usize], 0);
        assert_eq!(m[((8 + 7) * 4) as usize], 200);
        let cropped = Clip { crop: [0.25, 0.0, 0.25, 0.0], ..Default::default() };
        let cr = compose(4, 4, &frame(Some(layer(&src, 4, 4, &cropped)), 1.0, false, &[]));
        assert_eq!(cr[0], 0);
        assert_eq!(cr[4], 200);
        let mut grad = Vec::new();
        for _ in 0..20u32 {
            for x in 0..20u32 {
                grad.extend_from_slice(&[if x < 10 { 0 } else { 255 }, 0, 0, 255]);
            }
        }
        let blurred = Clip { blur: 100.0, ..Default::default() };
        let b = compose(20, 20, &frame(Some(layer(&grad, 20, 20, &blurred)), 1.0, false, &[]));
        let mid = b[((10 * 20 + 10) * 4) as usize];
        assert!(mid > 20 && mid < 250, "{mid}");

        let green = [0u8, 177, 64, 255].repeat(16);
        let keyed = Clip { key_on: true, ..Default::default() };
        let k = compose(4, 4, &Frame { background: [10, 20, 30, 255], ..frame(Some(layer(&green, 4, 4, &keyed)), 1.0, false, &[]) });
        assert_eq!(&k[0..3], &[10, 20, 30]);
        let masked = Clip { mask: Some(Mask { kind: 0, pts: vec![[0.0, 0.0], [0.5, 1.0]], feather: 0.0, invert: false }), ..Default::default() };
        let mk = compose(8, 8, &frame(Some(layer(&[200u8, 200, 200, 255].repeat(64), 8, 8, &masked)), 1.0, false, &[]));
        assert_eq!((mk[(4 * 8 + 1) * 4], mk[(4 * 8 + 6) * 4]), (200, 0));
        let inv = Clip { invert: true, ..Default::default() };
        assert_eq!(compose(4, 4, &frame(Some(layer(&src, 4, 4, &inv)), 1.0, false, &[]))[0], 55);

        let white = vec![255u8; 4 * 4 * 4];
        for kind in 1..26u8 {
            let half = transition(&src, &white, 4, 4, 0.0, kind);
            assert!(half[0] <= 200, "{kind}");
            let done = transition(&src, &white, 4, 4, 1.0, kind);
            assert!(done.chunks(4).filter(|p| p[0] == 255).count() >= 8, "{kind}");
        }

        let curved = Clip { curves: [[0.0, 0.1, 0.2, 0.1, 0.0], [0.0; 5], [0.0; 5], [0.0; 5]], ..Default::default() };
        let mid = [128u8, 128, 128, 255].repeat(16);
        assert!(compose(4, 4, &frame(Some(layer(&mid, 4, 4, &curved)), 1.0, false, &[]))[0] > 160);

        if font(0).is_some() {
            let title = Title { text: "Ethone".into(), start: 0.0, dur: 1.0, size: 0.3, color: [255, 0, 0, 255], anim: 1, font: 2, outline: 40.0, ..Default::default() };
            let t = compose(200, 100, &frame(None, 1.0, false, &[&title]));
            assert!(t.chunks(4).any(|p| p[0] > 200 && p[1] < 50));
            let early = compose(200, 100, &Frame { t: 0.0, ..frame(None, 1.0, false, &[&title]) });
            assert!(early.chunks(4).all(|p| p[0] == 0));
        }
    }
}
