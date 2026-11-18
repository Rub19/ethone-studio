//! Makes replacement text look like the text it replaces: finds the closest installed
//! font, rebuilds handwriting from the photo's own letters, and copies grain and softness.

use crate::shapes::{FONTS, TextStyle, render_text};
use image::{GrayImage, Luma, Rgba, RgbaImage, imageops};
use std::collections::{HashMap, HashSet};

pub const HANDWRITING: [&str; 8] = ["Segoe Print", "Segoe Script", "Ink Free", "Lucida Handwriting", "Gabriola", "Bradley Hand", "Mistral", "Comic Sans MS"];

pub fn is_handwriting(font: usize) -> bool {
    HANDWRITING.contains(&FONTS[font.min(FONTS.len() - 1)].0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub font: usize,
    pub bold: bool,
    pub italic: bool,
    pub score: f32,
    pub grain: f32,
    pub soft: f32,
}

/// Ink coverage of `rect`: 255 where a pixel has the text colour, 0 on the background.
/// Returns the mask and its top-left corner in the image.
pub fn ink(img: &RgbaImage, rect: [f32; 4], ink: [u8; 4], bg: [u8; 4]) -> (GrayImage, u32, u32) {
    let (w, h) = img.dimensions();
    let x0 = (rect[0].max(0.0) as u32).min(w.saturating_sub(1));
    let y0 = (rect[1].max(0.0) as u32).min(h.saturating_sub(1));
    let x1 = ((rect[0] + rect[2]).ceil() as u32).clamp(x0 + 1, w);
    let y1 = ((rect[1] + rect[3]).ceil() as u32).clamp(y0 + 1, h);
    let d = [0, 1, 2].map(|k| ink[k] as f32 - bg[k] as f32);
    let dd = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).max(1.0);
    let m = GrayImage::from_fn(x1 - x0, y1 - y0, |x, y| {
        let p = img.get_pixel(x0 + x, y0 + y).0;
        let t = (0..3).map(|k| (p[k] as f32 - bg[k] as f32) * d[k]).sum::<f32>() / dd;
        Luma([(t.clamp(0.0, 1.0) * 255.0) as u8])
    });
    (m, x0, y0)
}

fn tight(m: &GrayImage, thr: u8) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in m.enumerate_pixels() {
        if p.0[0] >= thr {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    (x0 <= x1).then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

fn norm(m: &GrayImage, b: (u32, u32, u32, u32), w: u32, h: u32) -> Vec<f32> {
    let c = imageops::crop_imm(m, b.0, b.1, b.2, b.3).to_image();
    let r = imageops::blur(&imageops::resize(&c, w, h, imageops::FilterType::Triangle), 1.0);
    r.pixels().map(|p| p.0[0] as f32 / 255.0).collect()
}

fn ncc(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len()) as f32;
    let (ma, mb) = (a.iter().sum::<f32>() / n, b.iter().sum::<f32>() / n);
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        let (x, y) = (x - ma, y - mb);
        ab += x * y;
        aa += x * x;
        bb += y * y;
    }
    ab / (aa * bb).sqrt().max(1e-6)
}

fn alpha_of(img: &RgbaImage) -> GrayImage {
    GrayImage::from_fn(img.width(), img.height(), |x, y| Luma([img.get_pixel(x, y).0[3]]))
}

fn font_dir() -> String {
    format!("{}\\Fonts", std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into()))
}

fn style(font: usize, size: f32, bold: bool, italic: bool, color: [u8; 4]) -> TextStyle {
    TextStyle { font, size, bold, italic, underline: false, align: 0, tracking: 0.0, leading: 1.2, vertical: false, color }
}

/// The installed font (and weight, slant) that draws `text` most like the ink in `mask`.
pub fn match_font(mask: &GrayImage, text: &str) -> Option<(usize, bool, bool, f32)> {
    let text = text.trim();
    let b = tight(mask, 96)?;
    if text.is_empty() || b.3 < 6 {
        return None;
    }
    let th = 32u32;
    let at = b.2 as f32 / b.3 as f32;
    let tw = (at * th as f32).round().clamp(8.0, 1600.0) as u32;
    let target = norm(mask, b, tw, th);
    let dir = font_dir();
    let mut seen = HashSet::new();
    let mut best: Option<(usize, bool, bool, f32)> = None;
    for (fi, (_, files)) in FONTS.iter().enumerate() {
        for pick in 0..4 {
            let file = files[pick];
            if !seen.insert(file.to_lowercase()) || !std::path::Path::new(&format!("{dir}\\{file}")).exists() {
                continue;
            }
            let (bold, italic) = (pick & 1 == 1, pick & 2 == 2);
            let Some(img) = render_text(text, &style(fi, 48.0, bold, italic, [0, 0, 0, 255])) else { continue };
            let a = alpha_of(&img);
            let Some(cb) = tight(&a, 96) else { continue };
            let ac = cb.2 as f32 / cb.3 as f32;
            let s = ncc(&target, &norm(&a, cb, tw, th)) - 0.6 * (ac / at).ln().abs();
            if best.is_none_or(|b| s > b.3) {
                best = Some((fi, bold, italic, s));
            }
        }
    }
    best
}

/// Share of soft (anti-aliased or blurred) edge pixels against solid ink pixels.
fn softness(m: &GrayImage) -> f32 {
    let (mut mid, mut core) = (0u32, 0u32);
    for p in m.pixels() {
        match p.0[0] {
            40..=214 => mid += 1,
            215.. => core += 1,
            _ => {}
        }
    }
    mid as f32 / core.max(1) as f32
}

/// Background noise around `rect` (pixel-to-pixel luminance jitter) and edge softness of the ink.
pub fn texture(img: &RgbaImage, rect: [f32; 4], mask: &GrayImage) -> (f32, f32) {
    let (w, h) = img.dimensions();
    let pad = (rect[3] * 0.4).max(3.0);
    let x0 = (rect[0] - pad).max(0.0) as u32;
    let y0 = (rect[1] - pad).max(0.0) as u32;
    let x1 = ((rect[0] + rect[2] + pad) as u32).min(w.saturating_sub(1));
    let y1 = ((rect[1] + rect[3] + pad) as u32).min(h);
    let inside = |x: u32, y: u32| x as f32 >= rect[0] && x as f32 <= rect[0] + rect[2] && y as f32 >= rect[1] && y as f32 <= rect[1] + rect[3];
    let lum = |x: u32, y: u32| {
        let p = img.get_pixel(x, y).0;
        0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
    };
    let mut d = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if !inside(x, y) && !inside(x + 1, y) {
                d.push((lum(x, y) - lum(x + 1, y)).abs());
            }
        }
    }
    d.sort_by(f32::total_cmp);
    let grain = d.get(d.len() / 2).copied().unwrap_or(0.0) / std::f32::consts::SQRT_2 * 1.48;
    (grain.min(40.0), softness(mask))
}

fn rnd(i: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B1) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x as f32 / u32::MAX as f32 * 2.0 - 1.0
}

/// Softens and adds grain to freshly drawn text so it sits in the photo like the original.
pub fn blend(img: &mut RgbaImage, look: &Look, color: [u8; 4]) {
    for p in img.pixels_mut().filter(|p| p.0[3] == 0) {
        *p = Rgba([color[0], color[1], color[2], 0]);
    }
    let sigma = ((look.soft - softness(&alpha_of(img))) * 0.5).clamp(0.0, 1.2);
    if sigma > 0.15 {
        *img = imageops::blur(img, sigma);
    }
    let amp = look.grain * 1.7;
    if amp > 0.5 {
        let w = img.width();
        for (x, y, p) in img.enumerate_pixels_mut() {
            if p.0[3] > 0 {
                let n = rnd(y * w + x) * amp;
                for k in 0..3 {
                    p.0[k] = (p.0[k] as f32 + n).clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

const DESC: &str = "gjpqyç";
const TALL: &str = "bdfhklt";
/// Letters drawn the same in both cases (only bigger), so one can stand in for the other.
const SAME_CASE: &str = "cosuvwxz";

/// The writer's own letters, cut out of the photo, with their spacing.
pub struct Hand {
    glyphs: HashMap<char, Vec<RgbaImage>>,
    gap: f32,
    space: f32,
    h: u32,
    cap: f32,
    base: f32,
    ink: Option<[u8; 4]>,
    /// Pen width of the original writing, in pixels.
    pen: f32,
    /// Each original word, cut out whole, and its columns in the line.
    words: Vec<RgbaImage>,
    spans: Vec<(u32, u32)>,
    /// Rebuild new words from the photo's letters (otherwise the closest font draws them).
    pub own_letters: bool,
}

/// The ink of columns `a..b` of the line, as an image with the photo's own pixels.
fn cut(img: &RgbaImage, mask: &GrayImage, x0: u32, y0: u32, a: u32, b: u32) -> RgbaImage {
    RgbaImage::from_fn(b - a, mask.height(), |x, y| {
        let m = mask.get_pixel(a + x, y).0[0] as f32;
        let p = img.get_pixel(x0 + a + x, y0 + y).0;
        Rgba([p[0], p[1], p[2], ((m - 40.0) * 255.0 / 175.0).clamp(0.0, 255.0) as u8])
    })
}

fn runs(m: &GrayImage, from: u32, to: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut start = None;
    for x in from..to {
        let on = (0..m.height()).any(|y| m.get_pixel(x, y).0[0] >= 110);
        match (on, start) {
            (true, None) => start = Some(x),
            (false, Some(s)) => {
                out.push((s, x));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, to));
    }
    out
}

fn col_ink(m: &GrayImage, x: u32) -> u32 {
    (0..m.height()).map(|y| m.get_pixel(x, y).0[0] as u32).sum()
}

/// Splits or merges ink runs until there is one per letter (joined handwriting is cut at its thinnest column).
fn fit_runs(m: &GrayImage, mut r: Vec<(u32, u32)>, n: usize) -> Vec<(u32, u32)> {
    while r.len() > n && r.len() > 1 {
        let i = (0..r.len() - 1).min_by_key(|&i| r[i + 1].0 - r[i].1).unwrap();
        r[i].1 = r[i + 1].1;
        r.remove(i + 1);
    }
    while r.len() < n && !r.is_empty() {
        let i = (0..r.len()).max_by_key(|&i| r[i].1 - r[i].0).unwrap();
        let (a, b) = r[i];
        if b - a < 2 {
            break;
        }
        let (lo, hi) = (a + (b - a) / 3, (a + 2 * (b - a) / 3).max(a + (b - a) / 3 + 1));
        let cut = (lo..hi).min_by_key(|&x| col_ink(m, x)).unwrap_or((a + b) / 2);
        r[i] = (a, cut);
        r.insert(i + 1, (cut, b));
    }
    r
}

fn median(mut v: Vec<f32>, or: f32) -> f32 {
    if v.is_empty() {
        return or;
    }
    v.sort_by(f32::total_cmp);
    v[v.len() / 2]
}

impl Hand {
    /// Cuts the letters of `text` out of the line whose ink is `mask` (top-left at `x0`, `y0` in `img`).
    pub fn from_line(img: &RgbaImage, mask: &GrayImage, x0: u32, y0: u32, text: &str, guide: (usize, bool, bool)) -> Hand {
        let (w, h) = mask.dimensions();
        let words: Vec<Vec<char>> = text.split_whitespace().map(|s| s.chars().collect()).collect();
        let all = runs(mask, 0, w);
        let mut gaps: Vec<(u32, usize)> = all.windows(2).enumerate().map(|(i, p)| (p[1].0 - p[0].1, i)).collect();
        gaps.sort_by(|a, b| b.0.cmp(&a.0));
        let mut cuts: Vec<usize> = gaps.iter().take(words.len().saturating_sub(1)).map(|g| g.1).collect();
        cuts.sort();
        let mut groups: Vec<Vec<(u32, u32)>> = Vec::new();
        let mut cur = Vec::new();
        for (i, r) in all.iter().enumerate() {
            cur.push(*r);
            if cuts.contains(&i) {
                groups.push(std::mem::take(&mut cur));
            }
        }
        groups.push(cur);
        let word_gap: Vec<f32> = cuts.iter().map(|&i| (all[i + 1].0 - all[i].1) as f32).collect();
        let mut glyphs: HashMap<char, Vec<RgbaImage>> = HashMap::new();
        let mut gap = Vec::new();
        let (mut caps, mut bases) = (Vec::new(), Vec::new());
        let spans: Vec<(u32, u32)> = if groups.len() == words.len() { groups.iter().filter_map(|g| Some((g.first()?.0, g.last()?.1))).collect() } else { Vec::new() };
        let whole: Vec<RgbaImage> = spans.iter().map(|&(a, b)| cut(img, mask, x0, y0, a, b)).collect();
        if groups.len() == words.len() {
            for (g, word) in groups.into_iter().zip(&words) {
                let rs = guided_cut(mask, &g, word, guide).unwrap_or_else(|| fit_runs(mask, g.clone(), word.len()));
                if rs.len() != word.len() {
                    continue;
                }
                gap.extend(rs.windows(2).map(|p| p[1].0 as f32 - p[0].1 as f32));
                for (&(a, b), &ch) in rs.iter().zip(word) {
                    let cut = cut(img, mask, x0, y0, a, b);
                    if let Some(t) = tight(&alpha_of(&cut), 110)
                        && !DESC.contains(ch.to_ascii_lowercase())
                        && ch.is_alphanumeric()
                    {
                        bases.push((t.1 + t.3) as f32);
                        if ch.is_uppercase() || ch.is_ascii_digit() || TALL.contains(ch) {
                            caps.push(t.3 as f32);
                        }
                    }
                    glyphs.entry(ch).or_default().push(cut);
                }
            }
        }
        let ink = solid_ink(&glyphs);
        let band = xheight_band(mask);
        Hand {
            glyphs,
            gap: median(gap, h as f32 * 0.06).max(0.0),
            space: median(word_gap, h as f32 * 0.35),
            h,
            cap: band.map_or_else(|| median(caps.clone(), h as f32 * 0.7), |b| (b.1 - b.0) / 0.62),
            base: band.map_or_else(|| median(bases.clone(), h as f32 * 0.8), |b| b.1),
            ink,
            pen: stroke_width(mask),
            words: whole,
            spans,
            own_letters: all.len() as f32 >= 0.7 * words.iter().map(|w| w.len()).sum::<usize>() as f32,
        }
    }

    /// Rewrites the line from `orig` to `new`, keeping the words that did not change.
    /// Returns the new ink for the columns from `start` on, and where the old ink to erase ends
    /// (columns of the line); `None` when nothing changed.
    pub fn rewrite(&self, orig: &str, new: &str, font: (usize, bool, bool), color: [u8; 4]) -> Option<(RgbaImage, u32, u32)> {
        let (ow, nw): (Vec<&str>, Vec<&str>) = (orig.split_whitespace().collect(), new.split_whitespace().collect());
        if self.spans.len() != ow.len() || ow.is_empty() {
            return Some((self.write(new, font, color)?, 0, self.spans.last().map_or(0, |s| s.1)));
        }
        let pre = ow.iter().zip(&nw).take_while(|(a, b)| a == b).count();
        let max_suf = ow.len().min(nw.len()) - pre;
        let suf = ow.iter().rev().zip(nw.iter().rev()).take(max_suf).take_while(|(a, b)| a == b).count();
        if pre == ow.len() && pre == nw.len() {
            return None;
        }
        let start = if pre < ow.len() { self.spans[pre].0 } else { self.spans[pre - 1].1 + self.space as u32 };
        let end = if pre < ow.len() { self.spans[ow.len() - 1].1 } else { start };
        let mut parts: Vec<RgbaImage> = Vec::new();
        let middle = nw[pre..nw.len() - suf].join(" ");
        if !middle.is_empty() {
            parts.push(self.write(&middle, font, color)?);
        }
        parts.extend(self.words[ow.len() - suf..].iter().cloned());
        let gap = self.space.round() as u32;
        let width = parts.iter().map(|p| p.width()).sum::<u32>() + gap * parts.len().saturating_sub(1) as u32;
        let height = parts.iter().map(|p| p.height()).max()?;
        let mut out = RgbaImage::new(width.max(1), height);
        let mut x = 0i64;
        for p in &parts {
            imageops::overlay(&mut out, p, x, 0);
            x += (p.width() + gap) as i64;
        }
        Some((out, start, end))
    }

    #[cfg(test)]
    pub fn letters(&self) -> usize {
        self.glyphs.len()
    }

    /// Writes `text` with the bank's letters; letters it lacks come from `font` in `color`.
    pub fn write(&self, text: &str, font: (usize, bool, bool), color: [u8; 4]) -> Option<RgbaImage> {
        let h = self.h;
        let mut pieces: Vec<(RgbaImage, i64, i64)> = Vec::new();
        let mut x = 0.0f32;
        let mut used: HashMap<char, usize> = HashMap::new();
        for (i, ch) in text.trim().chars().enumerate() {
            if ch.is_whitespace() {
                x += self.space;
                continue;
            }
            let j = (rnd(i as u32 * 7 + 1), rnd(i as u32 * 7 + 2));
            let swap = if ch.is_lowercase() { ch.to_ascii_uppercase() } else { ch.to_ascii_lowercase() };
            let own = if self.own_letters { self.glyphs.get(&ch).or_else(|| self.glyphs.get(&swap).filter(|_| SAME_CASE.contains(ch.to_ascii_lowercase()))) } else { None };
            let (g, y) = if let Some(v) = own {
                let n = used.entry(ch).or_default();
                let g = &v[*n % v.len()];
                *n += 1;
                let k = 1.0 + j.0 * 0.04;
                let g = imageops::resize(g, ((g.width() as f32 * k).round() as u32).max(1), ((h as f32 * k).round() as u32).max(1), imageops::FilterType::Triangle);
                let y = self.base - self.base * k + j.1 * h as f32 * 0.025;
                (g, y)
            } else {
                let img = render_text(&ch.to_string(), &style(font.0, 96.0, font.1, font.2, self.ink.unwrap_or(color)))?;
                let Some(t) = tight(&alpha_of(&img), 40) else {
                    x += self.space * 0.6;
                    continue;
                };
                let lower = ch.is_lowercase() && !TALL.contains(ch);
                let target = self.cap * if lower { 0.62 } else { 1.0 } * if DESC.contains(ch) { 1.35 } else { 1.0 };
                let k = target / t.3 as f32;
                let g = imageops::resize(&imageops::crop_imm(&img, t.0, t.1, t.2, t.3).to_image(), ((t.2 as f32 * k).round() as u32).max(1), (target.round() as u32).max(1), imageops::FilterType::Lanczos3);
                let g = thicken(g, self.pen);
                let drop = if DESC.contains(ch) { target * 0.26 } else { 0.0 };
                (g, self.base - target + drop + j.1 * h as f32 * 0.025)
            };
            pieces.push((g, x.round() as i64, y.round() as i64));
            x += pieces.last().unwrap().0.width() as f32 + self.gap;
        }
        let top = pieces.iter().map(|p| p.2).min()?.min(0);
        let bottom = pieces.iter().map(|p| p.2 + p.0.height() as i64).max()?.max(h as i64);
        let width = pieces.iter().map(|p| p.1 + p.0.width() as i64).max()?.max(1);
        let mut out = RgbaImage::new(width as u32, (bottom - top) as u32);
        for (g, gx, gy) in &pieces {
            imageops::overlay(&mut out, g, *gx, gy - top);
        }
        if top < 0 {
            // ponytail: rows above the line are cropped; a letter jittered upward loses at most 2.5 % of its height
            out = imageops::crop_imm(&out, 0, (-top) as u32, out.width(), h.min(out.height() - (-top) as u32)).to_image();
        }
        Some(out)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn line(text: &str, font: usize) -> (RgbaImage, [f32; 4]) {
        let mut img = RgbaImage::from_pixel(700, 120, Rgba([245, 240, 230, 255]));
        let t = render_text(text, &style(font, 64.0, false, false, [20, 30, 120, 255])).unwrap();
        imageops::overlay(&mut img, &t, 20, 20);
        let b = tight(&alpha_of(&t), 96).unwrap();
        (img, [20.0 + b.0 as f32 - 2.0, 20.0 + b.1 as f32 - 2.0, b.2 as f32 + 4.0, b.3 as f32 + 4.0])
    }

    #[test]
    fn finds_the_font_that_drew_the_line() {
        let georgia = FONTS.iter().position(|f| f.0 == "Georgia").unwrap();
        let (img, r) = line("Bonjour Paris", georgia);
        let (m, _, _) = ink(&img, r, [20, 30, 120, 255], [245, 240, 230, 255]);
        let (font, bold, _, _) = match_font(&m, "Bonjour Paris").unwrap();
        assert_eq!((FONTS[font].0, bold), ("Georgia", false));
    }

    #[test]
    fn rewrites_with_the_writers_own_letters() {
        let ink_c = [20, 30, 120, 255];
        let script = FONTS.iter().position(|f| f.0 == "Ink Free").unwrap();
        let (img, r) = line("Le chat dort", script);
        let (m, x0, y0) = ink(&img, r, ink_c, [245, 240, 230, 255]);
        let hand = Hand::from_line(&img, &m, x0, y0, "Le chat dort", (script, false, false));
        assert!(hand.letters() >= 8, "{} letters", hand.letters());
        let out = hand.write("Le rat dort", (script, false, false), ink_c).unwrap();
        assert_eq!(out.height(), m.height());
        assert!(out.width() > m.width() / 2 && out.width() < m.width() * 3 / 2, "{} vs {}", out.width(), m.width());
        let (grain, soft) = texture(&img, r, &m);
        let mut o = out.clone();
        blend(&mut o, &Look { font: script, bold: false, italic: false, score: 1.0, grain, soft }, ink_c);
        assert!(alpha_of(&o).pixels().any(|p| p.0[0] > 200));
    }
}

/// Average colour of the fully inked pixels of the cut-out letters: the real pen colour.
fn solid_ink(glyphs: &HashMap<char, Vec<RgbaImage>>) -> Option<[u8; 4]> {
    let mut acc = [0u64; 3];
    let mut n = 0u64;
    let mut solid: Vec<[u8; 4]> = glyphs.values().flatten().flat_map(|g| g.pixels()).filter(|p| p.0[3] >= 240).map(|p| p.0).collect();
    solid.sort_by_key(|p| p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114);
    for p in &solid[..solid.len().div_ceil(3)] {
        (0..3).for_each(|k| acc[k] += p[k] as u64);
        n += 1;
    }
    (n > 0).then(|| [(acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8, 255])
}

/// Erases `r` (x, y, w, h) on a plain background such as paper: a smooth blend of the clean pixels
/// around it (never copies ink from elsewhere), with the photo's grain added back.
pub fn paper_fill(img: &mut RgbaImage, r: [f32; 4], ink: [u8; 4], bg: [u8; 4], grain: f32) {
    let (w, h) = img.dimensions();
    if w < 2 || h < 2 {
        return;
    }
    let x0 = (r[0].max(0.0) as u32).min(w - 1);
    let y0 = (r[1].max(0.0) as u32).min(h - 1);
    let x1 = ((r[0] + r[2]).ceil() as u32).clamp(x0 + 1, w);
    let y1 = ((r[1] + r[3]).ceil() as u32).clamp(y0 + 1, h);
    let d = [0, 1, 2].map(|k| ink[k] as f32 - bg[k] as f32);
    let dd = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).max(1.0);
    let src = img.clone();
    let coverage = |p: [u8; 4]| (0..3).map(|k| (p[k] as f32 - bg[k] as f32) * d[k]).sum::<f32>() / dd;
    // the cleanest pixel within 6 px outward from (x, y) along (dx, dy)
    let clean = |x: i64, y: i64, dx: i64, dy: i64| -> [f32; 3] {
        let mut best = ([bg[0] as f32, bg[1] as f32, bg[2] as f32], f32::MAX);
        for s in 1..=6 {
            let (px, py) = ((x + dx * s).clamp(0, w as i64 - 1) as u32, (y + dy * s).clamp(0, h as i64 - 1) as u32);
            let p = src.get_pixel(px, py).0;
            let c = coverage(p);
            if c < best.1 {
                best = ([p[0] as f32, p[1] as f32, p[2] as f32], c);
            }
        }
        best.0
    };
    let smooth = |v: Vec<[f32; 3]>| -> Vec<[f32; 3]> {
        let n = v.len() as i64;
        (0..n).map(|i| {
            let (a, b) = ((i - 6).max(0), (i + 6).min(n - 1));
            let mut s = [0.0; 3];
            for j in a..=b {
                (0..3).for_each(|k| s[k] += v[j as usize][k]);
            }
            s.map(|c| c / (b - a + 1) as f32)
        }).collect()
    };
    let (cw, ch) = ((x1 - x0) as i64, (y1 - y0) as i64);
    let top = smooth((0..cw).map(|i| clean(x0 as i64 + i, y0 as i64, 0, -1)).collect());
    let bottom = smooth((0..cw).map(|i| clean(x0 as i64 + i, y1 as i64 - 1, 0, 1)).collect());
    let left = smooth((0..ch).map(|j| clean(x0 as i64, y0 as i64 + j, -1, 0)).collect());
    let right = smooth((0..ch).map(|j| clean(x1 as i64 - 1, y0 as i64 + j, 1, 0)).collect());
    let amp = grain * 1.7;
    for j in 0..ch {
        let v = if ch > 1 { j as f32 / (ch - 1) as f32 } else { 0.5 };
        for i in 0..cw {
            let u = if cw > 1 { i as f32 / (cw - 1) as f32 } else { 0.5 };
            let (x, y) = (x0 + i as u32, y0 + j as u32);
            let mut c = [0u8; 4];
            for k in 0..3 {
                let p = (1.0 - v) * top[i as usize][k] + v * bottom[i as usize][k] + (1.0 - u) * left[j as usize][k] + u * right[j as usize][k]
                    - ((1.0 - u) * (1.0 - v) * top[0][k] + u * (1.0 - v) * top[cw as usize - 1][k] + (1.0 - u) * v * bottom[0][k] + u * v * bottom[cw as usize - 1][k]);
                c[k] = (p + rnd(y * w + x + 977) * amp).clamp(0.0, 255.0) as u8;
            }
            c[3] = src.get_pixel(x, y).0[3];
            img.put_pixel(x, y, Rgba(c));
        }
    }
}

/// Cuts a joined word into letters where `guide` (the closest font) says each letter ends,
/// moved to the thinnest stroke nearby. `None` when the font cannot draw the word.
fn guided_cut(m: &GrayImage, runs: &[(u32, u32)], word: &[char], guide: (usize, bool, bool)) -> Option<Vec<(u32, u32)>> {
    let (a, b) = (runs.first()?.0, runs.last()?.1);
    let n = word.len();
    if n < 2 {
        return Some(vec![(a, b)]);
    }
    let right = |k: usize| -> Option<f32> {
        let s: String = word[..k].iter().collect();
        let img = render_text(&s, &style(guide.0, 64.0, guide.1, guide.2, [0, 0, 0, 255]))?;
        let t = tight(&alpha_of(&img), 60)?;
        Some((t.0 + t.2) as f32)
    };
    let full = right(n)?;
    let left = {
        let img = render_text(&word.iter().collect::<String>(), &style(guide.0, 64.0, guide.1, guide.2, [0, 0, 0, 255]))?;
        tight(&alpha_of(&img), 60)?.0 as f32
    };
    let span = (full - left).max(1.0);
    let wpx = (b - a) as f32;
    let reach = (wpx / n as f32 * 0.35).max(1.0) as u32;
    let mut cuts = vec![a];
    for k in 1..n {
        let guess = a + (((right(k)? - left) / span).clamp(0.0, 1.0) * wpx) as u32;
        let lo = guess.saturating_sub(reach).max(*cuts.last()? + 1);
        let hi = (guess + reach).min(b - 1);
        cuts.push(if lo < hi { (lo..=hi).min_by_key(|&x| (col_ink(m, x), x.abs_diff(guess))).unwrap() } else { lo.min(b - 1) });
    }
    cuts.push(b);
    Some(cuts.windows(2).filter(|c| c[1] > c[0]).map(|c| (c[0], c[1])).collect::<Vec<_>>()).filter(|v| v.len() == n)
}

/// Top and bottom rows of the x-height band (where the ink is densest): the baseline is its bottom.
fn xheight_band(m: &GrayImage) -> Option<(f32, f32)> {
    let rows: Vec<u32> = (0..m.height()).map(|y| (0..m.width()).filter(|&x| m.get_pixel(x, y).0[0] >= 110).count() as u32).collect();
    let (peak, &max) = rows.iter().enumerate().max_by_key(|r| r.1)?;
    if max == 0 {
        return None;
    }
    let dense = |y: usize| rows[y] * 2 >= max;
    let top = (0..=peak).rev().take_while(|&y| dense(y)).last()?;
    let bottom = (peak..rows.len()).take_while(|&y| dense(y)).last()?;
    Some((top as f32, bottom as f32 + 1.0))
}

/// Average stroke width of the ink: twice its area over its outline.
fn stroke_width(m: &GrayImage) -> f32 {
    let (w, h) = m.dimensions();
    let on = |x: i64, y: i64| x >= 0 && y >= 0 && x < w as i64 && y < h as i64 && m.get_pixel(x as u32, y as u32).0[0] >= 128;
    let (mut area, mut edge) = (0u32, 0u32);
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            if on(x, y) {
                area += 1;
                if !(on(x - 1, y) && on(x + 1, y) && on(x, y - 1) && on(x, y + 1)) {
                    edge += 1;
                }
            }
        }
    }
    if edge == 0 { 0.0 } else { 2.0 * area as f32 / edge as f32 }
}

/// Grows a letter's strokes (alpha only) until they are about as wide as the pen.
fn thicken(mut g: RgbaImage, pen: f32) -> RgbaImage {
    for _ in 0..3 {
        if pen <= 0.0 || stroke_width(&alpha_of(&g)) >= pen * 0.8 {
            break;
        }
        let src = g.clone();
        let (w, h) = g.dimensions();
        for (x, y, p) in g.enumerate_pixels_mut() {
            let mut a = p.0[3];
            for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 {
                    let q = src.get_pixel(nx as u32, ny as u32).0;
                    if q[3] > a {
                        a = q[3];
                        p.0[..3].copy_from_slice(&q[..3]);
                    }
                }
            }
            p.0[3] = a;
        }
    }
    g
}
