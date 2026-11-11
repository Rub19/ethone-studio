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

    pub fn save_wav(&self, path: &Path, float: bool) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: self.ch.len() as u16,
            sample_rate: self.rate,
            bits_per_sample: if float { 32 } else { 16 },
            sample_format: if float { hound::SampleFormat::Float } else { hound::SampleFormat::Int },
        };
        let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
        for i in 0..self.len() {
            for c in &self.ch {
                let v = c[i].clamp(-1.0, 1.0);
                if float {
                    w.write_sample(v).map_err(|e| e.to_string())?;
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

        let dir = std::env::temp_dir().join("ethone-sound-test.wav");
        c.save_wav(&dir, true).unwrap();
        let back = Clip::decode(&dir).unwrap();
        let _ = std::fs::remove_file(&dir);
        assert_eq!(back.len(), c.len());
        assert_eq!(back.ch.len(), 2);
        assert!((back.ch[0][5000] - c.ch[0][5000]).abs() < 1e-5);
    }
}
