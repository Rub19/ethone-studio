use crate::look;
use crate::media::VideoReader;
use crate::model::{Clip, Project, Source, Title, fade};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

type Pic = (u32, u32, Arc<Vec<u8>>);

#[derive(Default)]
pub struct Pics {
    readers: HashMap<(usize, usize), VideoReader>,
    stills: HashMap<usize, Pic>,
}

pub fn hex(s: &str) -> [u8; 3] {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

pub fn generate(path: &str, w: u32, h: u32) -> Vec<u8> {
    let parts: Vec<&str> = path.split(':').collect();
    let kind = parts.get(1).copied().unwrap_or("color");
    let mut out = vec![0u8; (w * h * 4) as usize];
    let bars: [[u8; 3]; 7] = [[192, 192, 192], [192, 192, 0], [0, 192, 192], [0, 192, 0], [192, 0, 192], [192, 0, 0], [0, 0, 192]];
    let a = hex(parts.get(2).unwrap_or(&"1e1e24"));
    let b = hex(parts.get(3).unwrap_or(&"000000"));
    let mut seed = 0x9e3779b9u32;
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let c = match kind {
                "gradient" => {
                    let u = (x as f32 / w as f32 * 0.6 + y as f32 / h as f32 * 0.4).clamp(0.0, 1.0);
                    [0, 1, 2].map(|k| (a[k] as f32 * (1.0 - u) + b[k] as f32 * u) as u8)
                }
                "bars" => {
                    if y < h * 3 / 4 { bars[(x * 7 / w.max(1)) as usize % 7] } else { [(x * 255 / w.max(1)) as u8; 3] }
                }
                "checker" => {
                    let cell = (h / 9).max(1);
                    if ((x / cell) + (y / cell)) % 2 == 0 { [220; 3] } else { [60; 3] }
                }
                "noise" => {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    [(seed & 0xff) as u8; 3]
                }
                _ => a,
            };
            out[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    out
}

pub fn load_still(path: &str) -> Result<Pic, String> {
    let img = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
    let (w, h) = img.dimensions();
    let k = (2048.0 / w.max(h) as f32).min(1.0);
    let img = if k < 1.0 { image::imageops::resize(&img, ((w as f32 * k) as u32).max(1), ((h as f32 * k) as u32).max(1), image::imageops::FilterType::Triangle) } else { img };
    let (w, h) = img.dimensions();
    Ok((w, h, Arc::new(img.into_raw())))
}

impl Pics {
    pub fn clear(&mut self) {
        self.readers.clear();
        self.stills.clear();
    }

    fn get(&mut self, proj: &Project, src: usize, slot: usize, st: f64) -> Result<Option<Pic>, String> {
        let Some(s) = proj.sources.get(src) else { return Ok(None) };
        if s.is_generator() {
            let (w, h) = ((proj.width / 4).max(16), (proj.height / 4).max(16));
            let hit = self.stills.get(&src).is_some_and(|p| p.0 == w && p.1 == h);
            if !hit {
                self.stills.insert(src, (w, h, Arc::new(generate(&s.path, w, h))));
            }
            return Ok(self.stills.get(&src).cloned());
        }
        if s.is_still() {
            if !self.stills.contains_key(&src) {
                let pic = load_still(&s.path)?;
                self.stills.insert(src, pic);
            }
            return Ok(self.stills.get(&src).cloned());
        }
        if !s.has_video {
            return Ok(None);
        }
        if !self.readers.contains_key(&(src, slot)) {
            let (r, _) = VideoReader::open(Path::new(&s.path))?;
            self.readers.insert((src, slot), r);
        }
        let r = self.readers.get_mut(&(src, slot)).ok_or("lecteur")?;
        let (w, h) = (r.width, r.height);
        let t = st.clamp(0.0, (s.duration - 1.0 / s.fps.max(1.0)).max(0.0));
        Ok(r.frame_at(t, s.fps)?.map(|(_, px)| (w, h, Arc::new(px.clone()))))
    }
}

struct Part {
    pic: Pic,
    clip: Clip,
    progress: f32,
}

pub fn render(proj: &Project, t: f64, w: u32, h: u32, pics: &mut Pics) -> Result<Vec<u8>, String> {
    let titles: Vec<&Title> = if proj.hide[0] { Vec::new() } else { proj.titles_at(t) };
    let mut base: Option<Part> = None;
    let mut from: Option<(Part, f32, u8)> = None;
    let (mut fd, mut white) = (1.0, false);
    if !proj.hide[2] {
        if let Some(b) = proj.transition_at(t) {
            let (ca, cb) = (&proj.clips[b.a], &proj.clips[b.b]);
            if !cb.disabled
                && let Some(pic) = pics.get(proj, cb.src, 0, cb.src_at(b.b_local))?
            {
                base = Some(Part { pic, clip: cb.eval(b.b_local.max(0.0)), progress: (b.b_local / cb.len().max(1e-6)) as f32 });
            }
            if !ca.disabled
                && let Some(pic) = pics.get(proj, ca.src, 1, ca.src_at(b.a_local))?
            {
                from = Some((Part { pic, clip: ca.eval(b.a_local.min(ca.len())), progress: (b.a_local / ca.len().max(1e-6)) as f32 }, b.p, cb.trans));
            }
        } else if let Some((i, local)) = proj.local_at(t) {
            let c = &proj.clips[i];
            fd = fade(c, local);
            white = c.fade_white;
            if !c.disabled
                && let Some(pic) = pics.get(proj, c.src, 0, c.src_at(local))?
            {
                base = Some(Part { pic, clip: c.eval(local), progress: (local / c.len().max(1e-6)) as f32 });
            }
        }
    }
    let mut overs = Vec::new();
    if !proj.hide[1] {
        for (k, local) in proj.overlays_at(t) {
            let o = &proj.overlays[k];
            if o.disabled {
                continue;
            }
            if let Some(pic) = pics.get(proj, o.src, 2 + k, o.src_at(local))? {
                let mut clip = o.eval(local);
                clip.opacity *= fade(o, local);
                overs.push(Part { pic, clip, progress: (local / o.len().max(1e-6)) as f32 });
            }
        }
    }
    let seed = (t * proj.fps as f64) as u32;
    let lay = |p| layer(p, seed);
    let overlays: Vec<look::Layer> = overs.iter().map(lay).collect();
    Ok(look::compose(
        w,
        h,
        &look::Frame {
            layer: base.as_ref().map(lay),
            from: from.as_ref().map(|(p, k, kind)| (lay(p), *k, *kind)),
            overlays: &overlays,
            fade: fd,
            white,
            background: proj.background,
            titles: &titles,
            t,
        },
    ))
}

fn layer(p: &Part, seed: u32) -> look::Layer<'_> {
    look::Layer { px: &p.pic.2, sw: p.pic.0, sh: p.pic.1, clip: &p.clip, progress: p.progress, seed }
}

pub fn render_fit(proj: &Project, t: f64, ow: u32, oh: u32, fill: bool, pics: &mut Pics) -> Result<Vec<u8>, String> {
    let (pw, ph) = (proj.width as f32, proj.height as f32);
    let k = if fill { (ow as f32 / pw).max(oh as f32 / ph) } else { (ow as f32 / pw).min(oh as f32 / ph) };
    let (rw, rh) = (((pw * k).round() as u32).max(2), ((ph * k).round() as u32).max(2));
    if rw == ow && rh == oh {
        return render(proj, t, ow, oh, pics);
    }
    let frame = render(proj, t, rw, rh, pics)?;
    let bg = proj.background;
    let mut out = Vec::with_capacity((ow * oh * 4) as usize);
    for _ in 0..ow * oh {
        out.extend_from_slice(&[bg[0], bg[1], bg[2], 255]);
    }
    let (ox, oy) = ((ow as i64 - rw as i64) / 2, (oh as i64 - rh as i64) / 2);
    for y in 0..oh as i64 {
        let sy = y - oy;
        if sy < 0 || sy >= rh as i64 {
            continue;
        }
        for x in 0..ow as i64 {
            let sx = x - ox;
            if sx < 0 || sx >= rw as i64 {
                continue;
            }
            let (d, s) = (((y * ow as i64 + x) * 4) as usize, ((sy * rw as i64 + sx) * 4) as usize);
            out[d..d + 4].copy_from_slice(&frame[s..s + 4]);
        }
    }
    Ok(out)
}

pub fn probe_still(path: &Path) -> Result<Source, String> {
    let (w, h) = image::image_dimensions(path).map_err(|e| e.to_string())?;
    Ok(Source { path: path.to_string_lossy().into_owned(), duration: 3600.0, width: w, height: h, fps: 30.0, has_video: true, has_audio: false, label: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generators_and_fit() {
        let px = generate("gen:color:ff0000", 4, 4);
        assert_eq!(&px[0..4], &[255, 0, 0, 255]);
        let mut p = Project::new();
        p.width = 16;
        p.height = 9;
        p.background = [0, 0, 255, 255];
        p.sources.push(Source { path: "gen:color:ff0000".into(), duration: 3600.0, width: 16, height: 9, fps: 30.0, has_video: true, has_audio: false, label: 0 });
        p.clips.push(Clip { src: 0, out: 2.0, ..Default::default() });
        let mut pics = Pics::default();
        let f = render_fit(&p, 0.5, 9, 16, false, &mut pics).unwrap();
        assert_eq!(&f[0..3], &[0, 0, 255]);
        assert_eq!(&f[((8 * 9 + 4) * 4)..((8 * 9 + 4) * 4 + 3)], &[255, 0, 0]);
        let full = render_fit(&p, 0.5, 9, 16, true, &mut pics).unwrap();
        assert_eq!(&full[0..3], &[255, 0, 0]);
    }
}
