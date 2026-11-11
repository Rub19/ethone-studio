use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub path: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub has_video: bool,
    pub has_audio: bool,
}

impl Source {
    pub fn name(&self) -> String {
        std::path::Path::new(&self.path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub src: usize,
    pub inp: f64,
    pub out: f64,
    pub volume: f32,
    pub fade_in: f64,
    pub fade_out: f64,
}

impl Clip {
    pub fn len(&self) -> f64 {
        self.out - self.inp
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Music {
    pub src: usize,
    pub start: f64,
    pub inp: f64,
    pub out: f64,
    pub volume: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub sources: Vec<Source>,
    pub clips: Vec<Clip>,
    pub music: Vec<Music>,
}

impl Project {
    pub fn new() -> Self {
        Self { width: 1920, height: 1080, fps: 30, sources: Vec::new(), clips: Vec::new(), music: Vec::new() }
    }

    pub fn starts(&self) -> Vec<f64> {
        let mut t = 0.0;
        self.clips
            .iter()
            .map(|c| {
                let s = t;
                t += c.len();
                s
            })
            .collect()
    }

    pub fn duration(&self) -> f64 {
        let v: f64 = self.clips.iter().map(Clip::len).sum();
        let m = self.music.iter().map(|m| m.start + m.out - m.inp).fold(0.0, f64::max);
        v.max(m)
    }

    #[cfg(test)]
    pub fn video_duration(&self) -> f64 {
        self.clips.iter().map(Clip::len).sum()
    }

    pub fn clip_at(&self, t: f64) -> Option<(usize, f64)> {
        let mut start = 0.0;
        for (i, c) in self.clips.iter().enumerate() {
            if t >= start && t < start + c.len() {
                return Some((i, c.inp + (t - start)));
            }
            start += c.len();
        }
        None
    }

    pub fn fade_at(&self, t: f64) -> f32 {
        let Some((i, st)) = self.clip_at(t) else { return 0.0 };
        let c = &self.clips[i];
        let a = if c.fade_in > 0.0 { ((st - c.inp) / c.fade_in).min(1.0) } else { 1.0 };
        let b = if c.fade_out > 0.0 { ((c.out - st) / c.fade_out).min(1.0) } else { 1.0 };
        (a.min(b)).clamp(0.0, 1.0) as f32
    }

    pub fn split(&mut self, t: f64) -> bool {
        let Some((i, st)) = self.clip_at(t) else { return false };
        let c = self.clips[i].clone();
        if st - c.inp < 0.05 || c.out - st < 0.05 {
            return false;
        }
        self.clips[i].out = st;
        self.clips[i].fade_out = 0.0;
        self.clips.insert(i + 1, Clip { inp: st, fade_in: 0.0, ..c });
        true
    }

    pub fn mix(&self, audio: &dyn Fn(usize) -> Option<std::sync::Arc<Vec<f32>>>, rate: u32) -> Vec<f32> {
        let total = (self.duration() * rate as f64).ceil() as usize;
        let mut out = vec![0f32; total * 2];
        let mut place = |data: &[f32], at: f64, inp: f64, out_t: f64, vol: f32, fi: f64, fo: f64| {
            let a = (inp * rate as f64) as usize;
            let b = ((out_t * rate as f64) as usize).min(data.len() / 2);
            let dst = (at * rate as f64) as usize;
            let n = b.saturating_sub(a);
            for k in 0..n {
                let d = dst + k;
                if d >= total {
                    break;
                }
                let tt = k as f64 / rate as f64;
                let mut g = vol;
                if fi > 0.0 {
                    g *= (tt / fi).min(1.0) as f32;
                }
                if fo > 0.0 {
                    g *= ((n - k) as f64 / rate as f64 / fo).min(1.0) as f32;
                }
                out[d * 2] += data[(a + k) * 2] * g;
                out[d * 2 + 1] += data[(a + k) * 2 + 1] * g;
            }
        };
        for (c, s) in self.clips.iter().zip(self.starts()) {
            if let Some(data) = audio(c.src) {
                place(&data, s, c.inp, c.out, c.volume, c.fade_in, c.fade_out);
            }
        }
        for m in &self.music {
            if let Some(data) = audio(m.src) {
                place(&data, m.start, m.inp, m.out, m.volume, 0.0, 0.0);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn src(d: f64) -> Source {
        Source { path: "a.mp4".into(), duration: d, width: 1920, height: 1080, fps: 30.0, has_video: true, has_audio: true }
    }

    #[test]
    fn timeline_split_fade_mix() {
        let mut p = Project::new();
        p.sources.push(src(10.0));
        p.clips.push(Clip { src: 0, inp: 2.0, out: 6.0, volume: 1.0, fade_in: 1.0, fade_out: 0.0 });
        p.clips.push(Clip { src: 0, inp: 0.0, out: 3.0, volume: 0.5, fade_in: 0.0, fade_out: 0.0 });
        assert_eq!(p.video_duration(), 7.0);
        assert_eq!(p.clip_at(5.0), Some((1, 1.0)));
        assert_eq!(p.clip_at(1.0), Some((0, 3.0)));
        assert_eq!(p.clip_at(7.5), None);
        assert!((p.fade_at(0.5) - 0.5).abs() < 1e-6);
        assert_eq!(p.fade_at(2.0), 1.0);

        assert!(p.split(1.5));
        assert_eq!(p.clips.len(), 3);
        assert_eq!((p.clips[0].inp, p.clips[0].out), (2.0, 3.5));
        assert_eq!((p.clips[1].inp, p.clips[1].out), (3.5, 6.0));
        assert_eq!(p.video_duration(), 7.0);
        assert!(!p.split(0.01));

        let rate = 100;
        let data = Arc::new(vec![1.0f32; 10 * rate as usize * 2]);
        p.music.push(Music { src: 0, start: 6.0, inp: 0.0, out: 2.0, volume: 0.25 });
        let mix = p.mix(&|_| Some(data.clone()), rate);
        assert_eq!(mix.len(), 8 * rate as usize * 2);
        assert!(mix[0].abs() < 0.02);
        assert!((mix[300 * 2] - 1.0).abs() < 1e-6);
        assert!((mix[550 * 2] - 0.5).abs() < 1e-6);
        assert!((mix[650 * 2] - 0.75).abs() < 1e-6);
        assert!((mix[750 * 2] - 0.25).abs() < 1e-6);
    }
}
