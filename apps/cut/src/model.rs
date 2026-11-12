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

fn one() -> f64 {
    1.0
}

fn one32() -> f32 {
    1.0
}

pub const LOOKS: [&str; 6] = ["Aucun", "Noir et blanc", "Sépia", "Vintage", "Cinéma", "Vif"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub src: usize,
    pub inp: f64,
    pub out: f64,
    pub volume: f32,
    pub fade_in: f64,
    pub fade_out: f64,
    #[serde(default = "one")]
    pub speed: f64,
    #[serde(default)]
    pub brightness: f32,
    #[serde(default)]
    pub contrast: f32,
    #[serde(default)]
    pub saturation: f32,
    #[serde(default)]
    pub temperature: f32,
    #[serde(default)]
    pub look: u8,
    #[serde(default = "one32")]
    pub zoom: f32,
    #[serde(default)]
    pub flip: bool,
    #[serde(default)]
    pub fade_white: bool,
}

impl Default for Clip {
    fn default() -> Self {
        Clip { src: 0, inp: 0.0, out: 0.0, volume: 1.0, fade_in: 0.0, fade_out: 0.0, speed: 1.0, brightness: 0.0, contrast: 0.0, saturation: 0.0, temperature: 0.0, look: 0, zoom: 1.0, flip: false, fade_white: false }
    }
}

impl Clip {
    pub fn len(&self) -> f64 {
        (self.out - self.inp) / self.speed.max(0.05)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Music {
    pub src: usize,
    pub start: f64,
    pub inp: f64,
    pub out: f64,
    pub volume: f32,
    #[serde(default)]
    pub fade_in: f64,
    #[serde(default)]
    pub fade_out: f64,
}

pub const PLACES: [&str; 4] = ["En haut", "Au centre", "En bas", "Tiers inférieur"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Title {
    pub text: String,
    pub start: f64,
    pub dur: f64,
    pub size: f32,
    pub place: u8,
    pub color: [u8; 4],
    pub band: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub sources: Vec<Source>,
    pub clips: Vec<Clip>,
    pub music: Vec<Music>,
    #[serde(default)]
    pub mute_v1: bool,
    #[serde(default)]
    pub mute_a2: bool,
    #[serde(default)]
    pub titles: Vec<Title>,
    #[serde(default)]
    pub markers: Vec<f64>,
}

impl Project {
    pub fn new() -> Self {
        Self { width: 1920, height: 1080, fps: 30, sources: Vec::new(), clips: Vec::new(), music: Vec::new(), mute_v1: false, mute_a2: false, titles: Vec::new(), markers: Vec::new() }
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
        let t = self.titles.iter().map(|t| t.start + t.dur).fold(0.0, f64::max);
        v.max(m).max(t)
    }

    #[cfg(test)]
    pub fn video_duration(&self) -> f64 {
        self.clips.iter().map(Clip::len).sum()
    }

    pub fn clip_at(&self, t: f64) -> Option<(usize, f64)> {
        let mut start = 0.0;
        for (i, c) in self.clips.iter().enumerate() {
            if t >= start && t < start + c.len() {
                return Some((i, c.inp + (t - start) * c.speed));
            }
            start += c.len();
        }
        None
    }

    pub fn fade_at(&self, t: f64) -> f32 {
        let Some((i, st)) = self.clip_at(t) else { return 0.0 };
        let c = &self.clips[i];
        let sp = c.speed.max(0.05);
        let a = if c.fade_in > 0.0 { ((st - c.inp) / sp / c.fade_in).min(1.0) } else { 1.0 };
        let b = if c.fade_out > 0.0 { ((c.out - st) / sp / c.fade_out).min(1.0) } else { 1.0 };
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

    pub fn titles_at(&self, t: f64) -> Vec<&Title> {
        self.titles.iter().filter(|x| t >= x.start && t < x.start + x.dur).collect()
    }

    pub fn trim_to(&mut self, t: f64, start: bool) -> bool {
        let Some((i, st)) = self.clip_at(t) else { return false };
        let c = &mut self.clips[i];
        if start && c.out - st > 0.05 {
            c.inp = st;
        } else if !start && st - c.inp > 0.05 {
            c.out = st;
        } else {
            return false;
        }
        true
    }

    pub fn mix(&self, audio: &dyn Fn(usize) -> Option<std::sync::Arc<Vec<f32>>>, rate: u32) -> Vec<f32> {
        let total = (self.duration() * rate as f64).ceil() as usize;
        let mut out = vec![0f32; total * 2];
        let mut place = |data: &[f32], at: f64, inp: f64, out_t: f64, vol: f32, fi: f64, fo: f64, speed: f64| {
            let a = (inp * rate as f64) as usize;
            let b = ((out_t * rate as f64) as usize).min(data.len() / 2);
            let dst = (at * rate as f64) as usize;
            let n = (b.saturating_sub(a) as f64 / speed.max(0.05)) as usize;
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
                let srcf = (a + (k as f64 * speed) as usize).min(b.saturating_sub(1));
                out[d * 2] += data[srcf * 2] * g;
                out[d * 2 + 1] += data[srcf * 2 + 1] * g;
            }
        };
        for (c, s) in self.clips.iter().zip(self.starts()) {
            if self.mute_v1 {
                break;
            }
            if let Some(data) = audio(c.src) {
                place(&data, s, c.inp, c.out, c.volume, c.fade_in, c.fade_out, c.speed);
            }
        }
        for m in &self.music {
            if self.mute_a2 {
                break;
            }
            if let Some(data) = audio(m.src) {
                place(&data, m.start, m.inp, m.out, m.volume, m.fade_in, m.fade_out, 1.0);
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
        p.clips.push(Clip { src: 0, inp: 2.0, out: 6.0, volume: 1.0, fade_in: 1.0, ..Default::default() });
        p.clips.push(Clip { src: 0, inp: 0.0, out: 3.0, volume: 0.5, ..Default::default() });
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
        p.music.push(Music { src: 0, start: 6.0, inp: 0.0, out: 2.0, volume: 0.25, fade_in: 0.0, fade_out: 0.0 });
        let mix = p.mix(&|_| Some(data.clone()), rate);
        assert_eq!(mix.len(), 8 * rate as usize * 2);
        assert!(mix[0].abs() < 0.02);
        assert!((mix[300 * 2] - 1.0).abs() < 1e-6);
        assert!((mix[550 * 2] - 0.5).abs() < 1e-6);
        assert!((mix[650 * 2] - 0.75).abs() < 1e-6);
        assert!((mix[750 * 2] - 0.25).abs() < 1e-6);
        p.mute_a2 = true;
        let muted = p.mix(&|_| Some(data.clone()), rate);
        assert!(muted[750 * 2].abs() < 1e-6);

        let mut q = Project::new();
        q.sources.push(src(10.0));
        q.clips.push(Clip { src: 0, inp: 0.0, out: 4.0, speed: 2.0, ..Default::default() });
        assert_eq!(q.duration(), 2.0);
        assert_eq!(q.clip_at(1.0), Some((0, 2.0)));
        assert_eq!(q.mix(&|_| Some(data.clone()), rate).len(), 2 * rate as usize * 2);
        assert!(q.trim_to(0.5, true));
        assert_eq!(q.clips[0].inp, 1.0);
        q.titles.push(Title { text: "Hi".into(), start: 3.0, dur: 2.0, size: 0.08, place: 1, color: [255; 4], band: false });
        assert_eq!(q.duration(), 5.0);
        assert_eq!(q.titles_at(4.0).len(), 1);
        assert!(q.titles_at(5.5).is_empty());
    }
}
