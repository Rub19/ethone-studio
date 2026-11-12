use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymErr;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Clone, PartialEq, Debug)]
pub struct Clip {
    pub rate: u32,
    pub ch: Vec<Vec<f32>>,
}

impl Clip {
    pub fn len(&self) -> usize {
        self.ch.first().map_or(0, |c| c.len())
    }

    pub fn secs(&self, n: usize) -> f64 {
        n as f64 / self.rate as f64
    }

    pub fn decode(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| format!("format non reconnu ({e})"))?;
        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("aucune piste audio")?;
        let id = track.id;
        let rate = track.codec_params.sample_rate.unwrap_or(44_100);
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| format!("codec non pris en charge ({e})"))?;
        let mut ch: Vec<Vec<f32>> = Vec::new();
        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(SymErr::IoError(_)) | Err(SymErr::ResetRequired) => break,
                Err(e) => return Err(e.to_string()),
            };
            if packet.track_id() != id {
                continue;
            }
            match decoder.decode(&packet) {
                Ok(buf) => {
                    let spec = *buf.spec();
                    let n = spec.channels.count();
                    if ch.is_empty() {
                        ch = vec![Vec::new(); n];
                    }
                    let mut sb = SampleBuffer::<f32>::new(buf.capacity() as u64, spec);
                    sb.copy_interleaved_ref(buf);
                    for frame in sb.samples().chunks(n) {
                        for (c, v) in frame.iter().enumerate() {
                            ch[c].push(*v);
                        }
                    }
                }
                Err(SymErr::DecodeError(_)) => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        if ch.is_empty() || ch[0].is_empty() {
            return Err("fichier vide".into());
        }
        Ok(Self { rate, ch })
    }

    pub fn save_wav(&self, path: &Path, bits: u16) -> Result<(), String> {
        let float = bits == 32;
        let spec = hound::WavSpec {
            channels: self.ch.len() as u16,
            sample_rate: self.rate,
            bits_per_sample: bits,
            sample_format: if float { hound::SampleFormat::Float } else { hound::SampleFormat::Int },
        };
        let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
        for i in 0..self.len() {
            for c in &self.ch {
                let v = c[i].clamp(-1.0, 1.0);
                if float {
                    w.write_sample(v).map_err(|e| e.to_string())?;
                } else if bits == 24 {
                    w.write_sample((v * 8_388_607.0).round() as i32).map_err(|e| e.to_string())?;
                } else {
                    w.write_sample((v * 32767.0).round() as i16).map_err(|e| e.to_string())?;
                }
            }
        }
        w.finalize().map_err(|e| e.to_string())
    }

    pub fn slice(&self, a: usize, b: usize) -> Clip {
        Clip { rate: self.rate, ch: self.ch.iter().map(|c| c[a..b].to_vec()).collect() }
    }

    pub fn remove(&mut self, a: usize, b: usize) {
        for c in &mut self.ch {
            c.drain(a..b);
        }
    }

    pub fn insert(&mut self, at: usize, other: &Clip) {
        let other = other.resampled(self.rate);
        for (i, c) in self.ch.iter_mut().enumerate() {
            let src = &other.ch[i.min(other.ch.len() - 1)];
            c.splice(at..at, src.iter().copied());
        }
    }

    pub fn resampled(&self, rate: u32) -> Clip {
        if rate == self.rate {
            return self.clone();
        }
        let ratio = self.rate as f64 / rate as f64;
        let n = (self.len() as f64 / ratio) as usize;
        let ch = self
            .ch
            .iter()
            .map(|c| {
                (0..n)
                    .map(|i| {
                        let p = i as f64 * ratio;
                        let k = p as usize;
                        let f = (p - k as f64) as f32;
                        let a = c[k.min(c.len() - 1)];
                        let b = c[(k + 1).min(c.len() - 1)];
                        a + (b - a) * f
                    })
                    .collect()
            })
            .collect();
        Clip { rate, ch }
    }

    pub fn each(&mut self, a: usize, b: usize, mut f: impl FnMut(usize, f32) -> f32) {
        for c in &mut self.ch {
            for (i, v) in c[a..b].iter_mut().enumerate() {
                *v = f(i, *v);
            }
        }
    }

    pub fn peak(&self, a: usize, b: usize) -> f32 {
        self.ch.iter().flat_map(|c| c[a..b].iter()).fold(0f32, |m, v| m.max(v.abs()))
    }

    pub fn gain_db(&mut self, a: usize, b: usize, db: f32) {
        let g = 10f32.powf(db / 20.0);
        self.each(a, b, |_, v| v * g);
    }

    pub fn normalize(&mut self, a: usize, b: usize, target_db: f32) {
        let p = self.peak(a, b);
        if p > 1e-6 {
            let g = 10f32.powf(target_db / 20.0) / p;
            self.each(a, b, |_, v| v * g);
        }
    }

    pub fn fade(&mut self, a: usize, b: usize, fade_in: bool) {
        let n = (b - a).max(1) as f32;
        self.each(a, b, |i, v| {
            let t = i as f32 / n;
            v * if fade_in { t } else { 1.0 - t }
        });
    }

    pub fn reverse(&mut self, a: usize, b: usize) {
        for c in &mut self.ch {
            c[a..b].reverse();
        }
    }

    pub fn silence(&mut self, a: usize, b: usize) {
        self.each(a, b, |_, _| 0.0);
    }

    pub fn high_pass(&mut self, a: usize, b: usize, cutoff: f32) {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let dt = 1.0 / self.rate as f32;
        let alpha = rc / (rc + dt);
        for c in &mut self.ch {
            let mut prev_in = c.get(a).copied().unwrap_or(0.0);
            let mut prev_out = 0.0;
            for v in &mut c[a..b] {
                let x = *v;
                let y = alpha * (prev_out + x - prev_in);
                prev_in = x;
                prev_out = y;
                *v = y;
            }
        }
    }

    pub fn gate(&mut self, a: usize, b: usize, threshold_db: f32) {
        let thr = 10f32.powf(threshold_db / 20.0);
        let attack = (-1.0 / (0.002 * self.rate as f32)).exp();
        let release = (-1.0 / (0.08 * self.rate as f32)).exp();
        let n = self.ch.len();
        let mut env = 0f32;
        let mut g = 0f32;
        for i in a..b {
            let level = (0..n).fold(0f32, |m, c| m.max(self.ch[c][i].abs()));
            env = if level > env { level } else { env * release + level * (1.0 - release) };
            let target = if env >= thr { 1.0 } else { 0.0 };
            let k = if target > g { attack } else { release };
            g = g * k + target * (1.0 - k);
            for c in 0..n {
                self.ch[c][i] *= g;
            }
        }
    }

    fn biquad(&mut self, a: usize, b: usize, coeffs: [f32; 5]) {
        let [b0, b1, b2, a1, a2] = coeffs;
        for c in &mut self.ch {
            let (mut x1, mut x2, mut y1, mut y2) = (0f32, 0f32, 0f32, 0f32);
            for v in &mut c[a..b] {
                let x = *v;
                let y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
                (x2, x1, y2, y1) = (x1, x, y1, y);
                *v = y;
            }
        }
    }

    /// RBJ cookbook filters. kind: 0 peak, 1 low shelf, 2 high shelf, 3 low pass, 4 high pass.
    pub fn filter(&mut self, a: usize, b: usize, kind: u8, freq: f32, gain_db: f32, q: f32) {
        let w = std::f32::consts::TAU * freq.min(self.rate as f32 * 0.45) / self.rate as f32;
        let (sn, cs) = w.sin_cos();
        let amp = 10f32.powf(gain_db / 40.0);
        let alpha = sn / (2.0 * q);
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
            _ => ((1.0 + cs) / 2.0, -(1.0 + cs), (1.0 + cs) / 2.0, 1.0 + alpha, -2.0 * cs, 1.0 - alpha),
        };
        self.biquad(a, b, [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]);
    }

    pub fn compress(&mut self, a: usize, b: usize, threshold_db: f32, ratio: f32, makeup_db: f32) {
        let attack = (-1.0 / (0.005 * self.rate as f32)).exp();
        let release = (-1.0 / (0.12 * self.rate as f32)).exp();
        let makeup = 10f32.powf(makeup_db / 20.0);
        let n = self.ch.len();
        let mut env = 0f32;
        for i in a..b {
            let level = (0..n).fold(0f32, |m, c| m.max(self.ch[c][i].abs()));
            let k = if level > env { attack } else { release };
            env = env * k + level * (1.0 - k);
            let db = 20.0 * env.max(1e-6).log10();
            let over = (db - threshold_db).max(0.0);
            let g = 10f32.powf(-(over - over / ratio.max(1.0)) / 20.0) * makeup;
            for c in 0..n {
                self.ch[c][i] *= g;
            }
        }
    }

    pub fn saturate(&mut self, a: usize, b: usize, drive: f32) {
        let k = 1.0 + drive * 9.0;
        let norm = k.tanh();
        self.each(a, b, |_, v| (v * k).tanh() / norm);
    }

    pub fn echo(&mut self, a: usize, b: usize, delay_ms: f32, feedback: f32, mix: f32) {
        let d = ((delay_ms / 1000.0) * self.rate as f32) as usize;
        if d == 0 {
            return;
        }
        for c in &mut self.ch {
            let dry: Vec<f32> = c[a..b].to_vec();
            let mut wet = vec![0f32; dry.len()];
            for i in d..dry.len() {
                wet[i] = dry[i - d] + wet[i - d] * feedback.clamp(0.0, 0.95);
            }
            for (i, v) in c[a..b].iter_mut().enumerate() {
                *v = dry[i] + wet[i] * mix;
            }
        }
    }

    /// Schroeder reverb: four parallel combs into two allpasses.
    pub fn reverb(&mut self, a: usize, b: usize, size: f32, mix: f32) {
        let r = self.rate as f32 / 44_100.0;
        let combs = [1116, 1188, 1277, 1356].map(|d| ((d as f32 * r * (0.6 + size * 0.8)) as usize).max(1));
        let alls = [556, 441].map(|d| ((d as f32 * r) as usize).max(1));
        let fb = 0.7 + size * 0.25;
        for (ci, c) in self.ch.iter_mut().enumerate() {
            let dry: Vec<f32> = c[a..b].to_vec();
            let mut wet = vec![0f32; dry.len()];
            for d in combs {
                let d = d + ci * 23;
                let mut buf = vec![0f32; dry.len()];
                for i in 0..dry.len() {
                    let back = if i >= d { buf[i - d] } else { 0.0 };
                    buf[i] = dry[i] + back * fb;
                    wet[i] += back * 0.25;
                }
            }
            for d in alls {
                let input = wet.clone();
                for i in 0..wet.len() {
                    let xd = if i >= d { input[i - d] } else { 0.0 };
                    let yd = if i >= d { wet[i - d] } else { 0.0 };
                    wet[i] = -0.5 * input[i] + xd + 0.5 * yd;
                }
            }
            for (i, v) in c[a..b].iter_mut().enumerate() {
                *v = dry[i] * (1.0 - mix * 0.5) + wet[i] * mix;
            }
        }
    }

    /// Plays the range faster or slower (pitch follows, like tape).
    pub fn speed(&mut self, a: usize, b: usize, factor: f32) {
        let part = self.slice(a, b);
        let stretched = Clip { rate: (self.rate as f32 * factor.clamp(0.25, 4.0)) as u32, ch: part.ch }.resampled(self.rate);
        self.remove(a, b);
        self.insert(a, &stretched);
    }

    pub fn rms_normalize(&mut self, a: usize, b: usize, target_db: f32) {
        let n = ((b - a) * self.ch.len()).max(1) as f32;
        let rms = (self.ch.iter().flat_map(|c| c[a..b].iter()).map(|v| v * v).sum::<f32>() / n).sqrt();
        if rms > 1e-6 {
            let g = 10f32.powf(target_db / 20.0) / rms;
            self.each(a, b, |_, v| (v * g).clamp(-1.0, 1.0));
        }
    }

    pub fn invert(&mut self, a: usize, b: usize) {
        self.each(a, b, |_, v| -v);
    }

    pub fn to_mono(&mut self) {
        if self.ch.len() < 2 {
            return;
        }
        let n = self.ch.len() as f32;
        let mono: Vec<f32> = (0..self.len()).map(|i| self.ch.iter().map(|c| c[i]).sum::<f32>() / n).collect();
        self.ch = vec![mono.clone(), mono];
    }

    pub fn swap(&mut self) {
        if self.ch.len() >= 2 {
            self.ch.swap(0, 1);
        }
    }

    /// pan: -1 left … 1 right; width: 0 mono … 2 extra wide.
    pub fn stereo(&mut self, a: usize, b: usize, pan: f32, width: f32) {
        if self.ch.len() < 2 {
            return;
        }
        let (gl, gr) = ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0));
        for i in a..b {
            let (l, r) = (self.ch[0][i], self.ch[1][i]);
            let (m, s) = ((l + r) / 2.0, (l - r) / 2.0 * width);
            self.ch[0][i] = (m + s) * gl;
            self.ch[1][i] = (m - s) * gr;
        }
    }

    /// kind: 0 silence, 1 tone 440 Hz, 2 white noise.
    pub fn generate(&mut self, at: usize, secs: f32, kind: u8) {
        let n = (secs * self.rate as f32) as usize;
        let mut seed = 0x1234_5678u32;
        let one: Vec<f32> = (0..n)
            .map(|i| match kind {
                1 => (i as f32 * 440.0 * std::f32::consts::TAU / self.rate as f32).sin() * 0.5,
                2 => {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    (seed as f32 / u32::MAX as f32 * 2.0 - 1.0) * 0.3
                }
                _ => 0.0,
            })
            .collect();
        let ch = self.ch.len().max(1);
        let part = Clip { rate: self.rate, ch: vec![one; ch] };
        if self.ch.is_empty() {
            *self = part;
        } else {
            self.insert(at.min(self.len()), &part);
        }
    }

    /// Magnitude spectrum (dB) of up to 2048 samples from `a`, on `bins` log-spaced bands from 20 Hz.
    pub fn spectrum(&self, a: usize, bins: usize) -> Vec<f32> {
        let n = 2048.min(self.len().saturating_sub(a));
        if n < 64 {
            return vec![-90.0; bins];
        }
        let x: Vec<f32> = (0..n)
            .map(|i| {
                let w = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos();
                self.ch.iter().map(|c| c[a + i]).sum::<f32>() / self.ch.len() as f32 * w
            })
            .collect();
        let nyq = self.rate as f32 / 2.0;
        (0..bins)
            .map(|k| {
                let f = 20.0 * (nyq / 20.0).powf(k as f32 / bins as f32);
                let w = std::f32::consts::TAU * f / self.rate as f32;
                let (mut re, mut im) = (0f32, 0f32);
                for (i, v) in x.iter().enumerate() {
                    let (s, c) = (w * i as f32).sin_cos();
                    re += v * c;
                    im -= v * s;
                }
                20.0 * ((re * re + im * im).sqrt() / n as f32 * 4.0).max(1e-5).log10()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(rate: u32, secs: f32) -> Clip {
        let n = (rate as f32 * secs) as usize;
        let c: Vec<f32> = (0..n).map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 0.5).collect();
        Clip { rate, ch: vec![c.clone(), c] }
    }

    #[test]
    fn edits_effects_and_wav_roundtrip() {
        let mut c = tone(8000, 1.0);
        let part = c.slice(1000, 3000);
        assert_eq!(part.len(), 2000);
        c.remove(1000, 3000);
        assert_eq!(c.len(), 6000);
        c.insert(1000, &part);
        assert_eq!(c.len(), 8000);

        c.normalize(0, 8000, 0.0);
        assert!((c.peak(0, 8000) - 1.0).abs() < 1e-4);
        c.gain_db(0, 8000, -6.0);
        assert!((c.peak(0, 8000) - 0.501).abs() < 0.01);
        c.fade(0, 4000, true);
        assert_eq!(c.ch[0][0], 0.0);
        c.silence(7000, 8000);
        assert_eq!(c.peak(7000, 8000), 0.0);

        let mut dc = Clip { rate: 8000, ch: vec![vec![0.5; 8000]] };
        dc.high_pass(0, 8000, 80.0);
        assert!(dc.ch[0][7999].abs() < 0.01);

        let mut quiet = Clip { rate: 8000, ch: vec![vec![0.001; 8000]] };
        quiet.gate(0, 8000, -40.0);
        assert!(quiet.peak(4000, 8000) < 1e-4);

        let up = c.resampled(16000);
        assert_eq!(up.len(), 16000);

        let mut lp = tone(8000, 1.0);
        lp.filter(0, 8000, 3, 100.0, 0.0, 0.707);
        assert!(lp.peak(4000, 8000) < 0.1, "low pass keeps 440 Hz: {}", lp.peak(4000, 8000));
        let mut boost = tone(8000, 1.0);
        boost.filter(0, 8000, 0, 440.0, 12.0, 1.0);
        assert!(boost.peak(4000, 8000) > 1.5);
        let mut loud = tone(8000, 1.0);
        loud.compress(0, 8000, -20.0, 8.0, 0.0);
        assert!(loud.peak(4000, 8000) < 0.25);
        let mut sat = tone(8000, 1.0);
        sat.saturate(0, 8000, 1.0);
        assert!(sat.peak(0, 8000) <= 1.0);
        let mut imp = Clip { rate: 8000, ch: vec![{
            let mut v = vec![0.0; 8000];
            v[0] = 1.0;
            v
        }] };
        imp.echo(0, 8000, 100.0, 0.5, 1.0);
        assert!((imp.ch[0][800] - 1.0).abs() < 1e-6 && (imp.ch[0][1600] - 0.5).abs() < 1e-6);
        let mut rv = imp.clone();
        rv.reverb(0, 8000, 0.5, 0.5);
        assert!(rv.peak(2000, 8000) > 0.0);
        let mut sp = tone(8000, 1.0);
        sp.speed(0, 8000, 2.0);
        assert_eq!(sp.len(), 4000);
        let mut st = tone(8000, 1.0);
        st.stereo(0, 8000, -1.0, 1.0);
        assert!(st.peak(0, 8000) > 0.4 && st.ch[1].iter().all(|v| v.abs() < 1e-6));
        st.generate(0, 0.5, 2);
        assert_eq!(st.len(), 12000);
        let spec = tone(8000, 1.0).spectrum(0, 64);
        let peak_bin = spec.iter().enumerate().fold((0, f32::MIN), |m, (i, v)| if *v > m.1 { (i, *v) } else { m }).0;
        let f = 20.0 * (4000f32 / 20.0).powf(peak_bin as f32 / 64.0);
        assert!((f - 440.0).abs() < 60.0, "{f}");
        let mut m = tone(8000, 1.0);
        m.rms_normalize(0, 8000, -12.0);
        assert!((m.peak(0, 8000) - 0.354).abs() < 0.02);

        let dir = std::env::temp_dir().join("ethone-sound-test.wav");
        c.save_wav(&dir, 32).unwrap();
        let back = Clip::decode(&dir).unwrap();
        let _ = std::fs::remove_file(&dir);
        assert_eq!(back.len(), c.len());
        assert_eq!(back.ch.len(), 2);
        assert!((back.ch[0][5000] - c.ch[0][5000]).abs() < 1e-5);
    }
}
