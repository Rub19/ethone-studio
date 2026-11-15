use crate::audio::{Clip, fft};
use std::f32::consts::{PI, TAU};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

pub fn hann(n: usize) -> Vec<f32> {
    (0..n).map(|i| 0.5 - 0.5 * (TAU * i as f32 / n as f32).cos()).collect()
}

pub fn db_to(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

fn coef(ms: f32, rate: u32) -> f32 {
    (-1.0 / (ms.max(0.01) / 1000.0 * rate as f32)).exp()
}

fn smooth(env: f32, lv: f32, ka: f32, kr: f32) -> f32 {
    let k = if lv > env { ka } else { kr };
    env * k + lv * (1.0 - k)
}

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32
    }
}

pub fn coeffs(kind: u8, freq: f32, gain_db: f32, q: f32, rate: u32) -> [f32; 5] {
    let sr = rate as f32;
    let w = TAU * freq.clamp(5.0, sr * 0.49) / sr;
    let (sn, cs) = w.sin_cos();
    let amp = 10f32.powf(gain_db / 40.0);
    let alpha = sn / (2.0 * q.max(0.05));
    let (b0, b1, b2, a0, a1, a2) = match kind {
        0 => (1.0 + alpha * amp, -2.0 * cs, 1.0 - alpha * amp, 1.0 + alpha / amp, -2.0 * cs, 1.0 - alpha / amp),
        1 | 2 => {
            let s = if kind == 1 { 1.0 } else { -1.0 };
            let k = 2.0 * amp.sqrt() * alpha;
            (
                amp * ((amp + 1.0) - s * (amp - 1.0) * cs + k),
                s * 2.0 * amp * ((amp - 1.0) - s * (amp + 1.0) * cs),
                amp * ((amp + 1.0) - s * (amp - 1.0) * cs - k),
                (amp + 1.0) + s * (amp - 1.0) * cs + k,
                -s * 2.0 * ((amp - 1.0) + s * (amp + 1.0) * cs),
                (amp + 1.0) + s * (amp - 1.0) * cs - k,
            )
        }
        3 => ((1.0 - cs) / 2.0, 1.0 - cs, (1.0 - cs) / 2.0, 1.0 + alpha, -2.0 * cs, 1.0 - alpha),
        4 => ((1.0 + cs) / 2.0, -(1.0 + cs), (1.0 + cs) / 2.0, 1.0 + alpha, -2.0 * cs, 1.0 - alpha),
        5 => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cs, 1.0 - alpha),
        _ => (1.0, -2.0 * cs, 1.0, 1.0 + alpha, -2.0 * cs, 1.0 - alpha),
    };
    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
}

#[cfg(test)]
pub fn response_db(co: &[f32; 5], f: f32, rate: u32) -> f32 {
    let w = std::f32::consts::TAU * f / rate as f32;
    let (c1, s1, c2, s2) = (w.cos(), w.sin(), (2.0 * w).cos(), (2.0 * w).sin());
    let nr = co[0] + co[1] * c1 + co[2] * c2;
    let ni = -(co[1] * s1 + co[2] * s2);
    let dr = 1.0 + co[3] * c1 + co[4] * c2;
    let di = -(co[3] * s1 + co[4] * s2);
    10.0 * ((nr * nr + ni * ni) / (dr * dr + di * di).max(1e-20)).log10()
}

pub fn biquad_run(x: &mut [f32], co: [f32; 5]) {
    let [b0, b1, b2, a1, a2] = co;
    let (mut x1, mut x2, mut y1, mut y2) = (0f32, 0f32, 0f32, 0f32);
    for v in x {
        let x0 = *v;
        let y = b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        (x2, x1, y2, y1) = (x1, x0, y1, y);
        *v = y;
    }
}

pub fn y_frac(f: f32, nyq: f32, log: bool) -> f32 {
    if log { ((f.max(30.0) / 30.0).ln() / (nyq / 30.0).ln()).clamp(0.0, 1.0) } else { (f / nyq).clamp(0.0, 1.0) }
}

pub fn freq_at(t: f32, nyq: f32, log: bool) -> f32 {
    if log { 30.0 * (nyq / 30.0).powf(t.clamp(0.0, 1.0)) } else { nyq * t.clamp(0.0, 1.0) }
}

pub fn fade_gain(t: f32, curve: f32) -> f32 {
    t.clamp(0.0, 1.0).powf(2f32.powf(-2.0 * curve))
}

fn heal_ch(c: &mut [f32], s: usize, e: usize) {
    if s < 2 || e + 1 >= c.len() || e <= s {
        return;
    }
    let (p0, p1) = (c[s - 1], c[e]);
    let (m0, m1) = (c[s - 1] - c[s - 2], c[e + 1] - c[e]);
    let l = (e - s + 1) as f32;
    for i in s..e {
        let t = (i - s + 1) as f32 / l;
        let (t2, t3) = (t * t, t * t * t);
        c[i] = (2.0 * t3 - 3.0 * t2 + 1.0) * p0 + (t3 - 2.0 * t2 + t) * m0 * l + (-2.0 * t3 + 3.0 * t2) * p1 + (t3 - t2) * m1 * l;
    }
}

fn stft_ola(src: &[f32], n: usize, hop: usize, mut f: impl FnMut(usize, &mut [f32], &mut [f32])) -> Vec<f32> {
    let win = hann(n);
    let len = src.len();
    let mut padded = vec![0f32; n];
    padded.extend_from_slice(src);
    padded.extend(std::iter::repeat_n(0.0, n));
    let mut out = vec![0f32; padded.len()];
    let mut norm = vec![0f32; padded.len()];
    let (mut s, mut frame) = (0usize, 0usize);
    while s + n <= padded.len() {
        let mut re: Vec<f32> = (0..n).map(|i| padded[s + i] * win[i]).collect();
        let mut im = vec![0f32; n];
        fft(&mut re, &mut im, false);
        f(frame, &mut re, &mut im);
        for k in 1..n / 2 {
            re[n - k] = re[k];
            im[n - k] = -im[k];
        }
        im[0] = 0.0;
        im[n / 2] = 0.0;
        fft(&mut re, &mut im, true);
        for i in 0..n {
            out[s + i] += re[i] * win[i];
            norm[s + i] += win[i] * win[i];
        }
        s += hop;
        frame += 1;
    }
    (0..len).map(|i| if norm[i + n] > 0.1 { out[i + n] / norm[i + n] } else { out[i + n] }).collect()
}

#[derive(Clone, Copy)]
pub enum SpecMode {
    Gain(f32),
    Heal,
}

pub struct Region {
    pub a: usize,
    pub b: usize,
}

impl Clip {
    fn level(&self, i: usize) -> f32 {
        self.ch.iter().fold(0f32, |m, c| m.max(c[i].abs()))
    }

    pub fn band(&mut self, a: usize, b: usize, kind: u8, freq: f32, gain: f32, q: f32, stages: usize) {
        let co = coeffs(kind, freq, gain, q, self.rate);
        for c in &mut self.ch {
            for _ in 0..stages.max(1) {
                biquad_run(&mut c[a..b], co);
            }
        }
    }

    pub fn normalize_each(&mut self, a: usize, b: usize, target: f32) {
        let t = db_to(target);
        for c in &mut self.ch {
            let p = c[a..b].iter().fold(0f32, |m, v| m.max(v.abs()));
            if p > 1e-6 {
                c[a..b].iter_mut().for_each(|v| *v *= t / p);
            }
        }
    }

    pub fn ramp(&mut self, a: usize, b: usize, db0: f32, db1: f32) {
        let n = (b - a).max(1) as f32;
        self.each(a, b, |i, v| v * db_to(db0 + (db1 - db0) * i as f32 / n));
    }

    pub fn limit(&mut self, a: usize, b: usize, ceiling_db: f32, release_ms: f32) {
        let ceil = db_to(ceiling_db);
        let kr = coef(release_ms, self.rate);
        let look = (self.rate as usize / 500).max(1);
        let n = b - a;
        let lv: Vec<f32> = (a..b).map(|i| self.level(i)).collect();
        let mut g = 1f32;
        let mut gains = vec![1f32; n];
        for i in 0..n {
            let ahead = lv[i..(i + look).min(n)].iter().fold(0f32, |m, v| m.max(*v));
            let need = if ahead > ceil { ceil / ahead } else { 1.0 };
            g = if need < g { need } else { g * kr + need * (1.0 - kr) };
            gains[i] = g;
        }
        for c in &mut self.ch {
            for (i, v) in c[a..b].iter_mut().enumerate() {
                *v = (*v * gains[i]).clamp(-ceil, ceil);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn dyn_core(ch: &mut [Vec<f32>], a: usize, b: usize, rate: u32, thr: f32, ratio: f32, att: f32, rel: f32, knee: f32, makeup: f32, expand: bool) {
        let (ka, kr) = (coef(att, rate), coef(rel, rate));
        let mk = db_to(makeup);
        let ratio = ratio.max(1.0);
        let mut env = 0f32;
        for i in a..b {
            let lv = ch.iter().fold(0f32, |m, c| m.max(c[i].abs()));
            env = smooth(env, lv, ka, kr);
            let o = 20.0 * env.max(1e-6).log10() - thr;
            let gr = if expand {
                if o < 0.0 { (o * (ratio - 1.0)).max(-80.0) } else { 0.0 }
            } else if 2.0 * o < -knee {
                0.0
            } else if knee > 0.0 && 2.0 * o.abs() <= knee {
                (1.0 / ratio - 1.0) * (o + knee / 2.0).powi(2) / (2.0 * knee)
            } else {
                (1.0 / ratio - 1.0) * o
            };
            let g = db_to(gr) * mk;
            for c in ch.iter_mut() {
                c[i] *= g;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn compress_ex(&mut self, a: usize, b: usize, thr: f32, ratio: f32, att: f32, rel: f32, knee: f32, makeup: f32) {
        let rate = self.rate;
        Self::dyn_core(&mut self.ch, a, b, rate, thr, ratio, att, rel, knee, makeup, false);
    }

    pub fn expand(&mut self, a: usize, b: usize, thr: f32, ratio: f32, att: f32, rel: f32) {
        let rate = self.rate;
        Self::dyn_core(&mut self.ch, a, b, rate, thr, ratio, att, rel, 0.0, 0.0, true);
    }

    pub fn gate_ex(&mut self, a: usize, b: usize, thr_db: f32, att: f32, hold_ms: f32, rel: f32, floor_db: f32) {
        let (thr, floor) = (db_to(thr_db), db_to(floor_db));
        let (ka, kr) = (coef(att, self.rate), coef(rel, self.rate));
        let hold = (hold_ms / 1000.0 * self.rate as f32) as usize;
        let (mut held, mut g) = (0usize, floor);
        for i in a..b {
            let target = if self.level(i) >= thr {
                held = hold;
                1.0
            } else if held > 0 {
                held -= 1;
                1.0
            } else {
                floor
            };
            let k = if target > g { ka } else { kr };
            g = g * k + target * (1.0 - k);
            for c in &mut self.ch {
                c[i] *= g;
            }
        }
    }

    pub fn multiband(&mut self, a: usize, b: usize, f1: f32, f2: f32, thr: [f32; 3], ratio: f32, makeup: f32) {
        let rate = self.rate;
        let n = b - a;
        let lo_co = coeffs(3, f1, 0.0, 0.707, rate);
        let hi_co = coeffs(4, f2.max(f1 * 1.5), 0.0, 0.707, rate);
        let mut bands: [Vec<Vec<f32>>; 3] = Default::default();
        for c in &self.ch {
            let x = &c[a..b];
            let mut lo = x.to_vec();
            biquad_run(&mut lo, lo_co);
            biquad_run(&mut lo, lo_co);
            let mut hi = x.to_vec();
            biquad_run(&mut hi, hi_co);
            biquad_run(&mut hi, hi_co);
            let mid: Vec<f32> = x.iter().zip(&lo).zip(&hi).map(|((v, l), h)| v - l - h).collect();
            bands[0].push(lo);
            bands[1].push(mid);
            bands[2].push(hi);
        }
        for (k, band) in bands.iter_mut().enumerate() {
            Self::dyn_core(band, 0, n, rate, thr[k], ratio, 10.0, 150.0, 6.0, 0.0, false);
        }
        let mk = db_to(makeup);
        for (ci, c) in self.ch.iter_mut().enumerate() {
            for i in 0..n {
                c[a + i] = (bands[0][ci][i] + bands[1][ci][i] + bands[2][ci][i]) * mk;
            }
        }
    }

    pub fn transient(&mut self, a: usize, b: usize, attack: f32, sustain: f32) {
        let r = self.rate;
        let (fa, fr, sa, sr) = (coef(1.0, r), coef(40.0, r), coef(25.0, r), coef(250.0, r));
        let (mut ef, mut es) = (0f32, 0f32);
        for i in a..b {
            let lv = self.level(i);
            ef = smooth(ef, lv, fa, fr);
            es = smooth(es, lv, sa, sr);
            let d = (ef - es) / (es + 1e-4);
            let gdb = if d > 0.0 { attack / 100.0 * 12.0 * d.min(1.0) } else { sustain / 100.0 * 12.0 * (-d).min(1.0) };
            let g = db_to(gdb);
            for c in &mut self.ch {
                c[i] *= g;
            }
        }
    }

    pub fn deess(&mut self, a: usize, b: usize, freq: f32, thr_db: f32, max_db: f32) {
        let co = coeffs(4, freq, 0.0, 0.707, self.rate);
        let highs: Vec<Vec<f32>> = self
            .ch
            .iter()
            .map(|c| {
                let mut h = c[a..b].to_vec();
                biquad_run(&mut h, co);
                biquad_run(&mut h, co);
                h
            })
            .collect();
        let (ka, kr) = (coef(0.5, self.rate), coef(60.0, self.rate));
        let mut env = 0f32;
        for i in 0..b - a {
            let lv = highs.iter().fold(0f32, |m, h| m.max(h[i].abs()));
            env = smooth(env, lv, ka, kr);
            let over = (20.0 * env.max(1e-6).log10() - thr_db).clamp(0.0, max_db);
            let g = db_to(-over);
            for (ci, c) in self.ch.iter_mut().enumerate() {
                c[a + i] += highs[ci][i] * (g - 1.0);
            }
        }
    }

    pub fn distort(&mut self, a: usize, b: usize, kind: u8, drive: f32, tone: f32, mix: f32) {
        let k = 1.0 + drive.clamp(0.0, 1.0) * 30.0;
        let lp = coeffs(3, tone, 0.0, 0.707, self.rate);
        let dc = coeffs(4, 20.0, 0.0, 0.707, self.rate);
        for c in &mut self.ch {
            let dry = c[a..b].to_vec();
            let mut w: Vec<f32> = dry
                .iter()
                .map(|&x| {
                    let y = x * k;
                    match kind {
                        0 => y.tanh(),
                        1 => y.clamp(-1.0, 1.0),
                        2 => (y * 3.0).signum() * (1.0 - (-(y * 3.0).abs()).exp()),
                        3 => y.max(0.0).tanh(),
                        4 => y.abs().tanh(),
                        5 => (y * 0.5).sin(),
                        _ => {
                            if y >= 0.0 { y.tanh() } else { (y * 0.6).tanh() * 0.8 }
                        }
                    }
                })
                .collect();
            if matches!(kind, 3 | 4 | 6) {
                biquad_run(&mut w, dc);
            }
            biquad_run(&mut w, lp);
            for (v, (d, x)) in c[a..b].iter_mut().zip(dry.iter().zip(&w)) {
                *v = d * (1.0 - mix) + x * mix * 0.9;
            }
        }
    }

    pub fn ring_mix(&mut self, a: usize, b: usize, freq: f32, mix: f32) {
        let sr = self.rate as f32;
        self.each(a, b, |i, v| v * (1.0 - mix) + v * mix * (i as f32 / sr * freq * TAU).sin());
    }

    pub fn dehum(&mut self, a: usize, b: usize, base: f32, harmonics: usize, q: f32) {
        for k in 1..=harmonics.max(1) {
            let f = base * k as f32;
            if f < self.rate as f32 * 0.45 {
                self.band(a, b, 6, f, 0.0, q * k as f32, 1);
            }
        }
    }

    pub fn auto_wah(&mut self, a: usize, b: usize, rate_hz: f32, lo: f32, hi: f32, q: f32, mix: f32) {
        let rate = self.rate;
        let sr = rate as f32;
        for c in &mut self.ch {
            let (mut x1, mut x2, mut y1, mut y2) = (0f32, 0f32, 0f32, 0f32);
            let mut co = [0f32; 5];
            for (i, v) in c[a..b].iter_mut().enumerate() {
                if i % 32 == 0 {
                    let t = 0.5 - 0.5 * (TAU * rate_hz * i as f32 / sr).cos();
                    co = coeffs(5, lo * (hi / lo.max(1.0)).powf(t), 0.0, q, rate);
                }
                let x = *v;
                let y = co[0] * x + co[1] * x1 + co[2] * x2 - co[3] * y1 - co[4] * y2;
                (x2, x1, y2, y1) = (x1, x, y1, y);
                *v = x * (1.0 - mix) + y * mix * 1.5;
            }
        }
    }

    pub fn heal(&mut self, a: usize, b: usize) {
        if b <= a {
            return;
        }
        if b - a > self.rate as usize / 50 {
            self.spectral_edit(a, b, &|s, _| s >= a && s < b, SpecMode::Heal);
        } else {
            for c in &mut self.ch {
                heal_ch(c, a, b);
            }
        }
    }

    pub fn declick(&mut self, a: usize, b: usize, sens: f32, max_ms: f32) -> usize {
        let maxw = ((max_ms / 1000.0) * self.rate as f32).max(2.0) as usize;
        let factor = 4.0 + (10.0 - sens.clamp(1.0, 10.0)) * 2.0;
        let n = b - a;
        if n < 64 {
            return 0;
        }
        let mut total = 0;
        for c in &mut self.ch {
            let e: Vec<f32> = (0..n).map(|i| if a + i < 2 { 0.0 } else { (c[a + i] - 2.0 * c[a + i - 1] + c[a + i - 2]).abs() }).collect();
            let mut pre = vec![0f64; n + 1];
            for i in 0..n {
                pre[i + 1] = pre[i] + e[i] as f64;
            }
            let mean = |i: usize| {
                let (l, r) = (i.saturating_sub(256), (i + 256).min(n));
                ((pre[r] - pre[l]) / (r - l).max(1) as f64) as f32
            };
            let mut runs: Vec<(usize, usize)> = Vec::new();
            for i in 0..n {
                if e[i] > factor * mean(i) && e[i] > 1e-3 {
                    match runs.last_mut() {
                        Some(r) if i - r.1 < 8 => r.1 = i,
                        _ => runs.push((i, i)),
                    }
                }
            }
            for (s, t) in runs {
                let (s, t) = (a + s.saturating_sub(2), (a + t + 3).min(c.len()));
                if t - s <= maxw + 5 {
                    heal_ch(c, s.max(2), t);
                    total += 1;
                }
            }
        }
        total
    }

    pub fn declip(&mut self, a: usize, b: usize, thr: f32) -> usize {
        let peak = self.peak(a, b);
        if peak < 1e-4 {
            return 0;
        }
        let level = thr * peak * 0.999;
        let mut count = 0;
        for c in &mut self.ch {
            let mut i = a;
            while i < b {
                if c[i].abs() >= level {
                    let s = i;
                    while i < b && c[i].abs() >= level && c[i].signum() == c[s].signum() {
                        i += 1;
                    }
                    if i - s >= 2 {
                        heal_ch(c, s, i);
                        count += 1;
                    }
                } else {
                    i += 1;
                }
            }
        }
        let np = self.peak(a, b);
        if np > peak {
            let g = peak / np;
            self.each(a, b, |_, v| v * g);
        }
        count
    }

    pub fn auto_denoise(&mut self, a: usize, b: usize, amount: f32) {
        const N: usize = 1024;
        if b - a < N * 2 {
            return;
        }
        let win = hann(N);
        let frames = (b - a - N) / (N / 2);
        let step = (frames / 1500).max(1);
        let mut mags: Vec<Vec<f32>> = vec![Vec::new(); N / 2 + 1];
        for c in &self.ch {
            let mut f = 0;
            while f < frames {
                let s = a + f * N / 2;
                let mut re: Vec<f32> = (0..N).map(|i| c[s + i] * win[i]).collect();
                let mut im = vec![0f32; N];
                fft(&mut re, &mut im, false);
                for k in 0..=N / 2 {
                    mags[k].push((re[k] * re[k] + im[k] * im[k]).sqrt());
                }
                f += step;
            }
        }
        let profile: Vec<f32> = mags
            .into_iter()
            .map(|mut m| {
                m.sort_by(|x, y| x.total_cmp(y));
                m.get(m.len() * 15 / 100).copied().unwrap_or(0.0)
            })
            .collect();
        self.denoise(a, b, &profile, amount);
    }

    pub fn pingpong(&mut self, a: usize, b: usize, delay_ms: f32, fb: f32, mix: f32) {
        if self.ch.len() < 2 {
            return self.echo(a, b, delay_ms, fb, mix);
        }
        let d = ((delay_ms / 1000.0) * self.rate as f32) as usize;
        let n = b - a;
        if d == 0 || d >= n {
            return;
        }
        let fb = fb.clamp(0.0, 0.95);
        let m: Vec<f32> = (a..b).map(|i| (self.ch[0][i] + self.ch[1][i]) * 0.5).collect();
        let (mut wl, mut wr) = (vec![0f32; n], vec![0f32; n]);
        for i in d..n {
            wl[i] = m[i - d] + fb * wr[i - d];
            wr[i] = fb * wl[i - d];
        }
        for i in 0..n {
            self.ch[0][a + i] += wl[i] * mix;
            self.ch[1][a + i] += wr[i] * mix;
        }
    }

    pub fn multitap(&mut self, a: usize, b: usize, delay_ms: f32, taps: usize, decay: f32, spacing: f32, mix: f32) {
        let sr = self.rate as f32;
        let delays: Vec<(usize, f32)> = (1..=taps.max(1)).map(|k| (((delay_ms / 1000.0) * sr * (1.0 + (k - 1) as f32 * spacing)) as usize, decay.powi(k as i32 - 1))).collect();
        let stereo = self.ch.len() > 1;
        for (ci, c) in self.ch.iter_mut().enumerate() {
            let dry = c[a..b].to_vec();
            for (k, (d, g)) in delays.iter().enumerate() {
                let side = if stereo && k % 2 != ci % 2 { 0.6 } else { 1.0 };
                for i in *d..dry.len() {
                    c[a + i] += dry[i - d] * g * mix * side;
                }
            }
        }
    }

    pub fn echo_filtered(&mut self, a: usize, b: usize, delay_ms: f32, fb: f32, tone: f32, mix: f32) {
        let d = ((delay_ms / 1000.0) * self.rate as f32) as usize;
        if d == 0 {
            return;
        }
        let al = 1.0 - (-TAU * tone / self.rate as f32).exp();
        for c in &mut self.ch {
            let dry = c[a..b].to_vec();
            let mut wet = vec![0f32; dry.len()];
            let mut z = 0f32;
            for i in d..dry.len() {
                let x = dry[i - d] + wet[i - d] * fb.clamp(0.0, 0.95);
                z += al * (x - z);
                wet[i] = z;
            }
            for (i, v) in c[a..b].iter_mut().enumerate() {
                *v = dry[i] + wet[i] * mix;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn freeverb(&mut self, a: usize, b: usize, size: f32, damp: f32, pre_ms: f32, width: f32, mix: f32) {
        const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        const ALLP: [usize; 4] = [556, 441, 341, 225];
        let r = self.rate as f32 / 44_100.0;
        let room = 0.7 + size.clamp(0.0, 1.0) * 0.28;
        let dmp = damp.clamp(0.0, 1.0) * 0.4;
        let pd = (pre_ms / 1000.0 * self.rate as f32) as usize;
        let n = b - a;
        let nch = self.ch.len().max(1) as f32;
        let input: Vec<f32> = (a..b).map(|i| self.ch.iter().map(|c| c[i]).sum::<f32>() / nch * 0.015).collect();
        let mut outs = vec![vec![0f32; n]; 2];
        for (side, out) in outs.iter_mut().enumerate() {
            for t in COMBS {
                let d = (((t + side * 23) as f32 * r) as usize).max(1);
                let mut buf = vec![0f32; d];
                let (mut idx, mut store) = (0usize, 0f32);
                for i in 0..n {
                    let x = if i >= pd { input[i - pd] } else { 0.0 };
                    let y = buf[idx];
                    store = y * (1.0 - dmp) + store * dmp;
                    buf[idx] = x + store * room;
                    idx = (idx + 1) % d;
                    out[i] += y;
                }
            }
            for t in ALLP {
                let d = (((t + side * 23) as f32 * r) as usize).max(1);
                let mut buf = vec![0f32; d];
                let mut idx = 0usize;
                for v in out.iter_mut() {
                    let bo = buf[idx];
                    buf[idx] = *v + bo * 0.5;
                    *v = bo - *v;
                    idx = (idx + 1) % d;
                }
            }
        }
        let wet = mix * 3.0;
        let (w1, w2) = (wet * (width / 2.0 + 0.5), wet * ((1.0 - width) / 2.0));
        let dry = 1.0 - mix * 0.4;
        for (ci, c) in self.ch.iter_mut().enumerate() {
            let (o1, o2) = if ci % 2 == 0 { (0, 1) } else { (1, 0) };
            for i in 0..n {
                c[a + i] = c[a + i] * dry + outs[o1][i] * w1 + outs[o2][i] * w2;
            }
        }
    }

    pub fn reverse_reverb(&mut self, a: usize, b: usize, size: f32, mix: f32) {
        self.reverse(a, b);
        self.freeverb(a, b, size, 0.3, 0.0, 1.0, mix);
        self.reverse(a, b);
    }

    pub fn phaser(&mut self, a: usize, b: usize, rate_hz: f32, depth: f32, stages: usize, fb: f32, mix: f32) {
        let sr = self.rate as f32;
        for (ci, c) in self.ch.iter_mut().enumerate() {
            let mut st = vec![(0f32, 0f32); stages.clamp(1, 16)];
            let mut last = 0f32;
            for (i, v) in c[a..b].iter_mut().enumerate() {
                let lfo = 0.5 + 0.5 * (TAU * rate_hz * i as f32 / sr + ci as f32 * 0.5).sin();
                let f = 200.0 * (1.0 + depth * 20.0 * lfo);
                let t = (PI * f.min(sr * 0.45) / sr).tan();
                let g = (t - 1.0) / (t + 1.0);
                let mut x = *v + last * fb.clamp(0.0, 0.9);
                for s in st.iter_mut() {
                    let y = g * x + s.0 - g * s.1;
                    s.0 = x;
                    s.1 = y;
                    x = y;
                }
                last = x;
                *v = (*v + x * mix) / (1.0 + mix);
            }
        }
    }

    pub fn vibrato(&mut self, a: usize, b: usize, rate_hz: f32, depth_ms: f32) {
        let sr = self.rate as f32;
        for c in &mut self.ch {
            let dry = c[a..b].to_vec();
            for (i, v) in c[a..b].iter_mut().enumerate() {
                let d = depth_ms * 0.5 * (1.0 + (TAU * rate_hz * i as f32 / sr).sin()) / 1000.0 * sr + 1.0;
                let p = i as f32 - d;
                *v = if p < 0.0 {
                    dry[0]
                } else {
                    let k = p as usize;
                    let f = p - k as f32;
                    dry[k] * (1.0 - f) + dry[(k + 1).min(dry.len() - 1)] * f
                };
            }
        }
    }

    pub fn tremolo_shape(&mut self, a: usize, b: usize, rate_hz: f32, depth: f32, shape: u8) {
        let sr = self.rate as f32;
        self.each(a, b, |i, v| {
            let ph = (i as f32 / sr * rate_hz).fract();
            let l = match shape {
                1 => 1.0 - 2.0 * (ph - 0.5).abs(),
                2 => {
                    if ph < 0.5 { 1.0 } else { 0.0 }
                }
                _ => 0.5 + 0.5 * (ph * TAU).sin(),
            };
            v * (1.0 - depth * l)
        });
    }

    pub fn robotize(&mut self, a: usize, b: usize, pitch_hz: f32) {
        let hop = ((self.rate as f32 / pitch_hz.max(20.0)) as usize).clamp(32, 512);
        for c in &mut self.ch {
            let y = stft_ola(&c[a..b], 1024, hop, |_, re, im| {
                for k in 0..=512 {
                    let m = (re[k] * re[k] + im[k] * im[k]).sqrt();
                    re[k] = if k % 2 == 0 { m } else { -m };
                    im[k] = 0.0;
                }
            });
            c[a..b].copy_from_slice(&y);
        }
    }

    pub fn whisper(&mut self, a: usize, b: usize) {
        let mut rng = Rng(0x2545_F491);
        for c in &mut self.ch {
            let y = stft_ola(&c[a..b], 512, 128, |_, re, im| {
                for k in 1..256 {
                    let m = (re[k] * re[k] + im[k] * im[k]).sqrt();
                    let ph = rng.next() * TAU;
                    re[k] = m * ph.cos();
                    im[k] = m * ph.sin();
                }
            });
            c[a..b].copy_from_slice(&y);
        }
    }

    pub fn vocode(&mut self, a: usize, b: usize, kind: u8, freq: f32, bands: usize) {
        const N: usize = 1024;
        let sr = self.rate as f32;
        let n = b - a;
        let before = self.peak(a, b);
        let mut rng = Rng(0x1357_9BDF);
        let mut ph = 0f32;
        let carrier: Vec<f32> = (0..n)
            .map(|_| {
                ph = (ph + freq / sr).fract();
                match kind {
                    1 => {
                        if ph < 0.5 { 1.0 } else { -1.0 }
                    }
                    2 => rng.next() * 2.0 - 1.0,
                    _ => ph * 2.0 - 1.0,
                }
            })
            .collect();
        let mut cpad = vec![0f32; N];
        cpad.extend_from_slice(&carrier);
        cpad.extend(std::iter::repeat_n(0.0, N));
        let win = hann(N);
        let bands = bands.clamp(2, 64);
        let edges: Vec<usize> = (0..=bands).map(|j| ((N as f32 / 2.0).powf(j as f32 / bands as f32) as usize).clamp(1, N / 2 + 1)).collect();
        for c in &mut self.ch {
            let y = stft_ola(&c[a..b], N, N / 4, |frame, re, im| {
                let s = frame * N / 4;
                let mut cr: Vec<f32> = (0..N).map(|i| cpad.get(s + i).copied().unwrap_or(0.0) * win[i]).collect();
                let mut ci = vec![0f32; N];
                fft(&mut cr, &mut ci, false);
                re[0] = 0.0;
                for j in 0..bands {
                    let (lo, hi) = (edges[j], edges[j + 1].max(edges[j] + 1).min(N / 2 + 1));
                    let em = (lo..hi).map(|k| re[k] * re[k] + im[k] * im[k]).sum::<f32>().sqrt();
                    let ec = (lo..hi).map(|k| cr[k] * cr[k] + ci[k] * ci[k]).sum::<f32>().sqrt();
                    let g = em / (ec + 1e-6);
                    for k in lo..hi {
                        re[k] = cr[k] * g;
                        im[k] = ci[k] * g;
                    }
                }
            });
            c[a..b].copy_from_slice(&y);
        }
        let after = self.peak(a, b);
        if after > 1e-6 {
            let g = before.max(0.1) / after;
            self.each(a, b, |_, v| v * g);
        }
    }

    pub fn stutter(&mut self, a: usize, b: usize, block_ms: f32, repeats: usize) {
        let l = ((block_ms / 1000.0) * self.rate as f32).max(16.0) as usize;
        let reps = repeats.max(2);
        let fade = (l / 16).max(1);
        for c in &mut self.ch {
            let mut s = a;
            while s + l <= b {
                let src: Vec<f32> = c[s..s + l].to_vec();
                for r in 1..reps {
                    let o = s + r * l;
                    if o >= b {
                        break;
                    }
                    for i in 0..l.min(b - o) {
                        let w = (i.min(l - 1 - i) as f32 / fade as f32).min(1.0);
                        c[o + i] = src[i] * w;
                    }
                }
                s += l * reps;
            }
        }
    }

    pub fn tape_stop(&mut self, a: usize, b: usize, start: bool) {
        let l = (b - a) as f64;
        if l < 2.0 {
            return;
        }
        for c in &mut self.ch {
            let src = c[a..b].to_vec();
            let at = |p: f64| {
                let k = (p.max(0.0) as usize).min(src.len() - 1);
                let f = (p - k as f64) as f32;
                src[k] * (1.0 - f) + src[(k + 1).min(src.len() - 1)] * f
            };
            for i in 0..src.len() {
                let x = i as f64;
                let p = if start { l - (l * l - x * x) / (2.0 * l) - l / 2.0 } else { x - x * x / (2.0 * l) };
                let g = if start { (x / l * 4.0).min(1.0) } else { ((l - x) / l * 4.0).min(1.0) };
                c[a + i] = at(p) * g as f32;
            }
        }
    }

    pub fn haas(&mut self, a: usize, b: usize, ms: f32, side: usize) {
        if self.ch.len() < 2 {
            return;
        }
        let d = (ms / 1000.0 * self.rate as f32) as usize;
        let c = &mut self.ch[side.min(1)];
        let src = c[a..b].to_vec();
        for i in 0..src.len() {
            c[a + i] = if i >= d { src[i - d] } else { 0.0 };
        }
    }

    pub fn ms_encode(&mut self, a: usize, b: usize, decode: bool) {
        if self.ch.len() < 2 {
            return;
        }
        let k = if decode { 1.0 } else { 0.5 };
        for i in a..b {
            let (l, r) = (self.ch[0][i], self.ch[1][i]);
            self.ch[0][i] = (l + r) * k;
            self.ch[1][i] = (l - r) * k;
        }
    }

    pub fn invert_ch(&mut self, a: usize, b: usize, ch: usize) {
        if let Some(c) = self.ch.get_mut(ch) {
            c[a..b].iter_mut().for_each(|v| *v = -*v);
        }
    }

    pub fn extract(&mut self, a: usize, b: usize, mode: u8) {
        if self.ch.len() < 2 {
            return;
        }
        for i in a..b {
            let (l, r) = (self.ch[0][i], self.ch[1][i]);
            let v = match mode {
                0 => l,
                1 => r,
                2 => (l + r) * 0.5,
                _ => (l - r) * 0.5,
            };
            self.ch[0][i] = v;
            self.ch[1][i] = v;
        }
    }

    pub fn bit_reduce(&mut self, a: usize, b: usize, bits: u32, dither: u8) {
        let q = 2f32.powi(bits.clamp(2, 24) as i32 - 1);
        let mut rng = Rng(0xA5A5_1234);
        for c in &mut self.ch {
            let mut e = 0f32;
            for v in &mut c[a..b] {
                let x = *v - if dither == 2 { e } else { 0.0 };
                let d = if dither > 0 { (rng.next() + rng.next() - 1.0) / q } else { 0.0 };
                let y = ((x + d) * q).round().clamp(-q, q - 1.0) / q;
                e = y - x;
                *v = y;
            }
        }
    }

    pub fn resample_hq(&self, rate: u32) -> Clip {
        if rate == self.rate || self.len() == 0 {
            return self.clone();
        }
        let ratio = rate as f64 / self.rate as f64;
        let cut = ratio.min(1.0) * 0.95;
        let h = (16.0 / cut).ceil() as isize;
        let n_out = (self.len() as f64 * ratio) as usize;
        let ch = self
            .ch
            .iter()
            .map(|c| {
                let len = c.len() as isize;
                (0..n_out)
                    .map(|i| {
                        let t = i as f64 / ratio;
                        let centre = t.floor() as isize;
                        let mut acc = 0f64;
                        for j in (centre - h + 1)..=(centre + h) {
                            if j < 0 || j >= len {
                                continue;
                            }
                            let x = t - j as f64;
                            let sinc = if x.abs() < 1e-9 { 1.0 } else { (std::f64::consts::PI * cut * x).sin() / (std::f64::consts::PI * cut * x) };
                            let wx = x / h as f64;
                            let w = 0.42 + 0.5 * (std::f64::consts::PI * wx).cos() + 0.08 * (2.0 * std::f64::consts::PI * wx).cos();
                            acc += c[j as usize] as f64 * cut * sinc * w;
                        }
                        acc as f32
                    })
                    .collect()
            })
            .collect();
        Clip { rate, ch }
    }

    pub fn detect_bpm(&self, a: usize, b: usize) -> Option<(f32, usize)> {
        const HOP: usize = 512;
        let frames = (b - a) / HOP;
        let fps = self.rate as f32 / HOP as f32;
        let lag_min = (60.0 * fps / 200.0) as usize;
        let lag_max = (60.0 * fps / 55.0) as usize;
        if frames < lag_max * 3 {
            return None;
        }
        let e: Vec<f32> = (0..frames)
            .map(|f| {
                let s = a + f * HOP;
                let en: f32 = (s..s + HOP).map(|i| self.ch.iter().map(|c| c[i] * c[i]).sum::<f32>()).sum();
                (1.0 + 1000.0 * en / HOP as f32).ln()
            })
            .collect();
        let mut flux: Vec<f32> = (0..frames).map(|i| if i == 0 { 0.0 } else { (e[i] - e[i - 1]).max(0.0) }).collect();
        let mean = flux.iter().sum::<f32>() / frames as f32;
        flux.iter_mut().for_each(|v| *v = (*v - mean).max(0.0));
        let ac = |lag: usize| (0..frames - lag).map(|i| flux[i] * flux[i + lag]).sum::<f32>() / (frames - lag) as f32;
        let scores: Vec<f32> = (lag_min..=lag_max)
            .map(|lag| {
                let bpm = 60.0 * fps / lag as f32;
                let w = (-0.5 * ((bpm / 120.0).log2() / 0.9).powi(2)).exp();
                (ac(lag) + 0.5 * if lag * 2 < frames { ac(lag * 2) } else { 0.0 }) * (0.5 + w)
            })
            .collect();
        let (bi, _) = scores.iter().enumerate().fold((0, f32::MIN), |m, (i, v)| if *v > m.1 { (i, *v) } else { m });
        let lag = lag_min + bi;
        let (l, c, r) = (if lag > lag_min { ac(lag - 1) } else { ac(lag) }, ac(lag), ac(lag + 1));
        let den = l - 2.0 * c + r;
        let frac = if den.abs() > 1e-9 { (0.5 * (l - r) / den).clamp(-0.5, 0.5) } else { 0.0 };
        let lagf = lag as f32 + frac;
        if c <= 0.0 {
            return None;
        }
        let (off, _) = (0..lag).map(|o| (o, (0..).map(|k| o as f32 + k as f32 * lagf).take_while(|p| (*p as usize) < frames).map(|p| flux[p as usize]).sum::<f32>())).fold((0, f32::MIN), |m, (o, s)| if s > m.1 { (o, s) } else { m });
        Some((60.0 * fps / lagf, a + off * HOP))
    }

    pub fn sound_regions(&self, a: usize, b: usize, thr_db: f32, min_sil_ms: f32, min_sound_ms: f32) -> Vec<Region> {
        let blk = (self.rate as usize / 100).max(1);
        let thr = db_to(thr_db);
        let ms = |m: f32| (m / 1000.0 * self.rate as f32) as usize;
        let mut runs: Vec<Region> = Vec::new();
        let mut s = a;
        while s < b {
            let e = (s + blk).min(b);
            let loud = (s..e).any(|i| self.level(i) >= thr);
            if loud {
                match runs.last_mut() {
                    Some(r) if s - r.b < ms(min_sil_ms) => r.b = e,
                    _ => runs.push(Region { a: s, b: e }),
                }
            }
            s = e;
        }
        runs.retain(|r| r.b - r.a >= ms(min_sound_ms));
        runs
    }

    pub fn true_peak(&self, a: usize, b: usize) -> f32 {
        let mut m = 0f32;
        for c in &self.ch {
            for i in a..b {
                m = m.max(c[i].abs());
                if i >= 1 && i + 2 < c.len() {
                    let (p0, p1, p2, p3) = (c[i - 1], c[i], c[i + 1], c[i + 2]);
                    for t in [0.25f32, 0.5, 0.75] {
                        let v = 0.5 * ((2.0 * p1) + (-p0 + p2) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t);
                        m = m.max(v.abs());
                    }
                }
            }
        }
        m
    }

    pub fn spectrogram_ex(&self, start: f64, spp: f64, cols: usize, rows: usize, n: usize, log: bool) -> Vec<Vec<f32>> {
        let win = hann(n);
        let len = self.len();
        let nch = self.ch.len().max(1) as f32;
        let nyq = self.rate as f32 / 2.0;
        let bins: Vec<f32> = (0..rows).map(|k| freq_at((k as f32 + 0.5) / rows as f32, nyq, log) / self.rate as f32 * n as f32).collect();
        (0..cols)
            .map(|x| {
                let centre = (start + (x as f64 + 0.5) * spp) as isize;
                let mut re: Vec<f32> = (0..n)
                    .map(|i| {
                        let p = centre - (n as isize / 2) + i as isize;
                        if p < 0 || p as usize >= len { 0.0 } else { self.ch.iter().map(|c| c[p as usize]).sum::<f32>() / nch * win[i] }
                    })
                    .collect();
                let mut im = vec![0f32; n];
                fft(&mut re, &mut im, false);
                bins.iter()
                    .map(|bf| {
                        let k = (*bf as usize).min(n / 2 - 1);
                        let f = (bf - k as f32).clamp(0.0, 1.0);
                        let m0 = (re[k] * re[k] + im[k] * im[k]).sqrt();
                        let m1 = (re[k + 1] * re[k + 1] + im[k + 1] * im[k + 1]).sqrt();
                        20.0 * ((m0 * (1.0 - f) + m1 * f) / n as f32 * 4.0).max(1e-7).log10()
                    })
                    .collect()
            })
            .collect()
    }

    pub fn spectral_edit(&mut self, a: usize, b: usize, mask: &dyn Fn(usize, f32) -> bool, mode: SpecMode) {
        const N: usize = 2048;
        const HOP: usize = 512;
        let len = self.len();
        if b <= a || len == 0 {
            return;
        }
        let lo = a.saturating_sub(N);
        let hi = (b + N).min(len);
        let bin_hz = self.rate as f32 / N as f32;
        let win = hann(N);
        for c in &mut self.ch {
            let base = lo as isize - N as isize;
            let padded: Vec<f32> = (0..hi - lo + 2 * N).map(|i| {
                let p = base + i as isize;
                if p >= 0 && (p as usize) < len { c[p as usize] } else { 0.0 }
            }).collect();
            let mut frames: Vec<(Vec<f32>, Vec<f32>)> = Vec::new();
            let mut marks: Vec<Vec<bool>> = Vec::new();
            let mut s = 0usize;
            while s + N <= padded.len() {
                let mut re: Vec<f32> = (0..N).map(|i| padded[s + i] * win[i]).collect();
                let mut im = vec![0f32; N];
                fft(&mut re, &mut im, false);
                let centre = (lo + s + N / 2) as isize - N as isize;
                let m: Vec<bool> = (0..=N / 2).map(|k| centre >= 0 && mask(centre as usize, k as f32 * bin_hz)).collect();
                frames.push((re, im));
                marks.push(m);
                s += HOP;
            }
            let nf = frames.len();
            match mode {
                SpecMode::Gain(g) => {
                    for (j, (re, im)) in frames.iter_mut().enumerate() {
                        for k in 0..=N / 2 {
                            if marks[j][k] {
                                re[k] *= g;
                                im[k] *= g;
                            }
                        }
                    }
                }
                SpecMode::Heal => {
                    let r = N / HOP / 2;
                    let orig = marks.clone();
                    for j in 0..nf {
                        for k in 0..=N / 2 {
                            if !orig[j][k] && (j.saturating_sub(r)..(j + r + 1).min(nf)).any(|q| orig[q][k]) {
                                marks[j][k] = true;
                            }
                        }
                    }
                    let mag = |f: &(Vec<f32>, Vec<f32>), k: usize| (f.0[k] * f.0[k] + f.1[k] * f.1[k]).sqrt();
                    let mut targets: Vec<(usize, usize, f32, f32)> = Vec::new();
                    for k in 0..=N / 2 {
                        let mut j = 0;
                        while j < nf {
                            if !marks[j][k] {
                                j += 1;
                                continue;
                            }
                            let s0 = j;
                            while j < nf && marks[j][k] {
                                j += 1;
                            }
                            let before = s0.checked_sub(1).map(|p| mag(&frames[p], k));
                            let after = (j < nf).then(|| mag(&frames[j], k));
                            let expect = TAU * k as f32 * HOP as f32 / N as f32;
                            let (phi0, omega) = match s0.checked_sub(1) {
                                Some(p) => {
                                    let ph = frames[p].1[k].atan2(frames[p].0[k]);
                                    let om = match p.checked_sub(1) {
                                        Some(pp) => {
                                            let d = ph - frames[pp].1[k].atan2(frames[pp].0[k]) - expect;
                                            expect + (d + PI).rem_euclid(TAU) - PI
                                        }
                                        None => expect,
                                    };
                                    (ph, om)
                                }
                                None if j < nf => {
                                    let ph = frames[j].1[k].atan2(frames[j].0[k]);
                                    (ph - expect * (j - s0 + 1) as f32, expect)
                                }
                                None => continue,
                            };
                            let span = (j - s0 + 1) as f32;
                            for (q, fi) in (s0..j).enumerate() {
                                let t = (q + 1) as f32 / span;
                                let m = match (before, after) {
                                    (Some(x), Some(y)) => x + (y - x) * t,
                                    (Some(x), None) => x,
                                    (None, Some(y)) => y,
                                    _ => continue,
                                };
                                targets.push((fi, k, m, phi0 + omega * (q + 1) as f32));
                            }
                        }
                    }
                    for (fi, k, m, ph) in targets {
                        let (re, im) = &mut frames[fi];
                        re[k] = m * ph.cos();
                        im[k] = m * ph.sin();
                    }
                }
            }
            let mut out = vec![0f32; padded.len()];
            let mut norm = vec![0f32; padded.len()];
            for (j, (mut re, mut im)) in frames.into_iter().enumerate() {
                for k in 1..N / 2 {
                    re[N - k] = re[k];
                    im[N - k] = -im[k];
                }
                im[0] = 0.0;
                im[N / 2] = 0.0;
                fft(&mut re, &mut im, true);
                let s = j * HOP;
                for i in 0..N {
                    out[s + i] += re[i] * win[i];
                    norm[s + i] += win[i] * win[i];
                }
            }
            for (i, v) in c[lo..hi].iter_mut().enumerate() {
                if norm[i + N] > 0.1 {
                    *v = out[i + N] / norm[i + N];
                }
            }
        }
    }
}

fn put_chunk(buf: &mut Vec<u8>, id: &[u8; 4], data: &[u8]) {
    buf.extend_from_slice(id);
    buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
    buf.extend_from_slice(data);
    if data.len() % 2 == 1 {
        buf.push(0);
    }
}

pub fn export_wav(clip: &Clip, path: &Path, bits: u16, dither: u8, info: &[(&[u8; 4], String)], cues: &[(usize, String)]) -> Result<(), String> {
    let mut c = clip.clone();
    let n = c.len();
    if bits < 32 && dither > 0 {
        c.bit_reduce(0, n, bits as u32, dither);
    }
    let float = bits == 32;
    let spec = hound::WavSpec {
        channels: c.ch.len() as u16,
        sample_rate: c.rate,
        bits_per_sample: bits,
        sample_format: if float { hound::SampleFormat::Float } else { hound::SampleFormat::Int },
    };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for i in 0..n {
        for ch in &c.ch {
            let v = ch[i].clamp(-1.0, 1.0);
            let r = match bits {
                32 => w.write_sample(v),
                24 => w.write_sample((v * 8_388_607.0).round() as i32),
                8 => w.write_sample((v * 127.0).round() as i8),
                _ => w.write_sample((v * 32767.0).round() as i16),
            };
            r.map_err(|e| e.to_string())?;
        }
    }
    w.finalize().map_err(|e| e.to_string())?;
    let mut extra = Vec::new();
    if !cues.is_empty() {
        let mut d = (cues.len() as u32).to_le_bytes().to_vec();
        for (k, (pos, _)) in cues.iter().enumerate() {
            let pos = *pos as u32;
            d.extend_from_slice(&(k as u32 + 1).to_le_bytes());
            d.extend_from_slice(&pos.to_le_bytes());
            d.extend_from_slice(b"data");
            d.extend_from_slice(&0u32.to_le_bytes());
            d.extend_from_slice(&0u32.to_le_bytes());
            d.extend_from_slice(&pos.to_le_bytes());
        }
        put_chunk(&mut extra, b"cue ", &d);
        let mut adtl = b"adtl".to_vec();
        for (k, (_, name)) in cues.iter().enumerate() {
            let mut l = (k as u32 + 1).to_le_bytes().to_vec();
            l.extend_from_slice(name.as_bytes());
            l.push(0);
            put_chunk(&mut adtl, b"labl", &l);
        }
        put_chunk(&mut extra, b"LIST", &adtl);
    }
    let fields: Vec<_> = info.iter().filter(|(_, v)| !v.trim().is_empty()).collect();
    if !fields.is_empty() {
        let mut d = b"INFO".to_vec();
        for (id, v) in fields {
            let mut s = v.trim().as_bytes().to_vec();
            s.push(0);
            put_chunk(&mut d, id, &s);
        }
        put_chunk(&mut extra, b"LIST", &d);
    }
    if extra.is_empty() {
        return Ok(());
    }
    let io = |e: std::io::Error| e.to_string();
    let mut f = std::fs::OpenOptions::new().read(true).write(true).open(path).map_err(io)?;
    let mut len = f.seek(SeekFrom::End(0)).map_err(io)?;
    if len % 2 == 1 {
        f.write_all(&[0]).map_err(io)?;
        len += 1;
    }
    f.write_all(&extra).map_err(io)?;
    f.seek(SeekFrom::Start(4)).map_err(io)?;
    f.write_all(&((len + extra.len() as u64 - 8) as u32).to_le_bytes()).map_err(io)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(rate: u32, secs: f32, f: f32) -> Clip {
        let n = (rate as f32 * secs) as usize;
        let c: Vec<f32> = (0..n).map(|i| (i as f32 * f * TAU / rate as f32).sin() * 0.5).collect();
        Clip { rate, ch: vec![c.clone(), c] }
    }

    fn ok(c: &Clip) -> bool {
        c.ch.iter().flatten().all(|v| v.is_finite())
    }

    #[test]
    fn filters_and_dynamics() {
        let co = coeffs(6, 1000.0, 0.0, 10.0, 8000);
        assert!(response_db(&co, 1000.0, 8000) < -30.0 && response_db(&co, 200.0, 8000).abs() < 0.5);
        let co = coeffs(0, 1000.0, 6.0, 1.0, 8000);
        assert!((response_db(&co, 1000.0, 8000) - 6.0).abs() < 0.2);
        let mut n = tone(8000, 1.0, 1000.0);
        n.band(0, 8000, 6, 1000.0, 0.0, 5.0, 1);
        assert!(n.peak(4000, 8000) < 0.05);
        let mut m = tone(8000, 1.0, 440.0);
        let orig = m.clone();
        m.multiband(0, 8000, 200.0, 2000.0, [0.0, 0.0, 0.0], 1.0, 0.0);
        assert!(m.ch[0].iter().zip(&orig.ch[0]).all(|(x, y)| (x - y).abs() < 1e-3));
        let mut c = tone(8000, 1.0, 440.0);
        c.compress_ex(0, 8000, -20.0, 10.0, 1.0, 50.0, 0.0, 0.0);
        assert!(c.peak(4000, 8000) < 0.2);
        let mut q = Clip { rate: 8000, ch: vec![vec![0.001; 8000]] };
        q.gate_ex(0, 8000, -40.0, 1.0, 10.0, 20.0, -80.0);
        assert!(q.peak(4000, 8000) < 1e-5);
        let mut l = tone(8000, 1.0, 440.0);
        l.gain_db(0, 8000, 12.0);
        l.limit(0, 8000, -3.0, 50.0);
        assert!(l.peak(0, 8000) <= db_to(-3.0) + 1e-4);
        let mut e = tone(8000, 1.0, 440.0);
        e.normalize_each(0, 8000, -6.0);
        assert!((e.peak(0, 8000) - 0.501).abs() < 0.01);
        for k in 0..7 {
            let mut d = tone(8000, 0.5, 440.0);
            d.distort(0, 4000, k, 0.5, 6000.0, 1.0);
            assert!(ok(&d) && d.peak(0, 4000) <= 1.2, "dist {k}");
        }
        let mut t = tone(8000, 1.0, 440.0);
        t.transient(0, 8000, 50.0, -50.0);
        t.deess(0, 8000, 3000.0, -40.0, 12.0);
        t.expand(0, 8000, -30.0, 2.0, 1.0, 100.0);
        t.ramp(0, 8000, -6.0, 0.0);
        t.dehum(0, 8000, 50.0, 4, 20.0);
        t.auto_wah(0, 8000, 1.0, 300.0, 2000.0, 3.0, 0.8);
        assert!(ok(&t));
    }

    #[test]
    fn restoration_and_spectral() {
        let mut c = tone(8000, 1.0, 300.0);
        let clean = c.clone();
        c.ch[0][4000] += 0.8;
        c.ch[1][4000] += 0.8;
        assert!(c.declick(0, 8000, 6.0, 2.0) >= 2);
        assert!((c.ch[0][4000] - clean.ch[0][4000]).abs() < 0.1, "{}", c.ch[0][4000]);
        let mut h = tone(8000, 1.0, 300.0);
        h.ch[0][2000..2010].iter_mut().for_each(|v| *v = 0.9);
        h.heal(2000, 2010);
        assert!(h.ch[0][2000..2010].iter().zip(&clean.ch[0][2000..2010]).all(|(a, b)| (a - b).abs() < 0.1));
        let mut cl = tone(8000, 1.0, 300.0);
        cl.gain_db(0, 8000, 6.0);
        cl.each(0, 8000, |_, v| v.clamp(-0.7, 0.7));
        assert!(cl.declip(0, 8000, 0.99) > 10);
        assert!(ok(&cl));
        let mut mix = tone(8000, 1.0, 300.0);
        let hi = tone(8000, 1.0, 2500.0);
        for (v, h) in mix.ch[0].iter_mut().zip(&hi.ch[0]) {
            *v += h;
        }
        mix.ch.truncate(1);
        mix.spectral_edit(0, 8000, &|_, f| f > 1500.0, SpecMode::Gain(0.0));
        let sp = mix.spectrum(3000, 64);
        let bin = |f: f32| ((f / 20.0).ln() / (4000f32 / 20.0).ln() * 64.0) as usize;
        assert!(sp[bin(2500.0)] < sp[bin(300.0)] - 30.0, "{} {}", sp[bin(2500.0)], sp[bin(300.0)]);
        let mut gap = tone(8000, 1.0, 300.0);
        gap.ch.truncate(1);
        gap.ch[0][3000..4000].iter_mut().for_each(|v| *v = 0.0);
        gap.spectral_edit(3000, 4000, &|s, _| (3000..4000).contains(&s), SpecMode::Heal);
        let rms = (gap.ch[0][3300..3700].iter().map(|v| v * v).sum::<f32>() / 400.0).sqrt();
        assert!(rms > 0.2, "{rms}");
        let mut un = tone(8000, 1.0, 300.0);
        un.spectral_edit(0, 8000, &|_, _| false, SpecMode::Gain(0.0));
        assert!(un.ch[0].iter().zip(&clean.ch[0]).all(|(a, b)| (a - b).abs() < 1e-3));
        let noise = Clip::synth(8000, 1, 1.0, 4, 0.0, 0.0, 0.1);
        let mut nz = tone(8000, 1.0, 300.0);
        nz.ch.truncate(1);
        nz.ch[0][..4000].iter_mut().for_each(|v| *v = 0.0);
        for (v, n) in nz.ch[0].iter_mut().zip(&noise.ch[0]) {
            *v += n;
        }
        let before = nz.ch[0][500..3500].iter().map(|v| v * v).sum::<f32>();
        nz.auto_denoise(0, 8000, 0.8);
        assert!(nz.ch[0][500..3500].iter().map(|v| v * v).sum::<f32>() < before * 0.5);
    }

    #[test]
    fn creative_time_and_analysis() {
        let mut c = tone(8000, 1.0, 440.0);
        c.pingpong(0, 8000, 100.0, 0.5, 0.5);
        c.multitap(0, 8000, 50.0, 4, 0.6, 1.0, 0.4);
        c.echo_filtered(0, 8000, 80.0, 0.5, 2000.0, 0.4);
        c.freeverb(0, 8000, 0.6, 0.4, 20.0, 1.0, 0.3);
        c.reverse_reverb(0, 8000, 0.6, 0.3);
        c.phaser(0, 8000, 0.5, 0.7, 6, 0.4, 0.7);
        c.vibrato(0, 8000, 5.0, 2.0);
        c.tremolo_shape(0, 8000, 5.0, 0.5, 1);
        c.ring_mix(0, 8000, 30.0, 0.5);
        c.haas(0, 8000, 15.0, 1);
        c.ms_encode(0, 8000, false);
        c.ms_encode(0, 8000, true);
        c.invert_ch(0, 8000, 0);
        c.extract(0, 8000, 2);
        assert!(ok(&c) && c.peak(0, 8000) < 4.0);
        let mut imp = Clip { rate: 8000, ch: vec![vec![0.0; 8000]] };
        imp.ch[0][0] = 1.0;
        imp.freeverb(0, 8000, 0.8, 0.3, 0.0, 1.0, 1.0);
        assert!(imp.peak(4000, 8000) > 1e-4);
        for f in 0..4 {
            let mut v = tone(8000, 1.0, 220.0);
            match f {
                0 => v.robotize(0, 8000, 100.0),
                1 => v.whisper(0, 8000),
                2 => v.vocode(0, 8000, 0, 110.0, 16),
                _ => v.stutter(0, 8000, 100.0, 3),
            }
            assert!(ok(&v) && v.peak(0, 8000) > 0.01 && v.len() == 8000, "creative {f}");
        }
        let mut ts = tone(8000, 1.0, 220.0);
        ts.tape_stop(0, 8000, false);
        ts.tape_stop(0, 8000, true);
        assert!(ok(&ts) && ts.len() == 8000);
        let mut br = tone(8000, 1.0, 220.0);
        br.bit_reduce(0, 8000, 8, 2);
        assert!(br.ch[0].iter().all(|v| ((v * 128.0).round() - v * 128.0).abs() < 1e-3));
        let up = tone(8000, 1.0, 440.0).resample_hq(16000);
        assert_eq!(up.len(), 16000);
        assert!((up.peak(4000, 12000) - 0.5).abs() < 0.03);
        let met = Clip::synth(8000, 1, 12.0, 7, 120.0, 0.0, 0.8);
        let (bpm, _) = met.detect_bpm(0, met.len()).unwrap();
        assert!((bpm - 120.0).abs() < 3.0, "{bpm}");
        let mut gaps = tone(8000, 0.5, 440.0);
        gaps.generate(2000, 1.0, 0);
        let r = gaps.sound_regions(0, gaps.len(), -40.0, 300.0, 50.0);
        assert_eq!(r.len(), 2);
        assert!(tone(8000, 1.0, 440.0).true_peak(0, 8000) >= 0.49);
        assert_eq!(tone(8000, 1.0, 440.0).spectrogram_ex(0.0, 100.0, 40, 32, 1024, false).len(), 40);
        assert!((fade_gain(0.5, 0.0) - 0.5).abs() < 1e-6 && fade_gain(1.0, 1.0) == 1.0);
        assert!((y_frac(freq_at(0.3, 4000.0, true), 4000.0, true) - 0.3).abs() < 1e-4);

        let p = std::env::temp_dir().join("ethone-sound-meta.wav");
        let src = Clip { rate: 8000, ch: vec![tone(8000, 0.1, 440.0).ch[0][..799].to_vec()] };
        export_wav(&src, &p, 8, 1, &[(b"INAM", "Titre é".into())], &[(100, "Début".into())]).unwrap();
        let bytes = std::fs::read(&p).unwrap();
        assert!(bytes.windows(4).any(|w| w == b"INAM") && bytes.windows(4).any(|w| w == b"cue "));
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize, bytes.len() - 8);
        let back = Clip::decode(&p).unwrap();
        let _ = std::fs::remove_file(&p);
        assert_eq!(back.len(), 799);
        assert!((back.ch[0][100] - src.ch[0][100]).abs() < 0.02);
    }
}
