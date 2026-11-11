use image::{RgbImage, imageops};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const RAW_EXT: [&str; 14] = ["cr2", "cr3", "nef", "nrw", "arw", "srf", "sr2", "dng", "raf", "orf", "rw2", "pef", "srw", "3fr"];
pub const IMG_EXT: [&str; 6] = ["jpg", "jpeg", "png", "webp", "tif", "tiff"];

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub exposure: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
    pub temperature: f32,
    pub tint: f32,
    pub vibrance: f32,
    pub saturation: f32,
    pub vignette: f32,
    pub rotate: u8,
}

impl Settings {
    pub fn is_default(&self) -> bool {
        *self == Settings::default()
    }
}

pub fn is_supported(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| RAW_EXT.contains(&e.as_str()) || IMG_EXT.contains(&e.as_str()))
}

pub fn is_raw(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| RAW_EXT.contains(&e.to_ascii_lowercase().as_str()))
}

pub fn load(path: &Path, max: u32) -> Result<RgbImage, String> {
    let img = if is_raw(path) {
        let m = max as usize;
        let s = imagepipe::simple_decode_8bit(path, m, m)?;
        RgbImage::from_raw(s.width as u32, s.height as u32, s.data).ok_or("RAW illisible")?
    } else {
        image::open(path).map_err(|e| e.to_string())?.into_rgb8()
    };
    Ok(if max > 0 && (img.width() > max || img.height() > max) {
        let r = max as f32 / img.width().max(img.height()) as f32;
        imageops::resize(&img, (img.width() as f32 * r) as u32, (img.height() as f32 * r) as u32, imageops::FilterType::Triangle)
    } else {
        img
    })
}

fn to_lin(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn to_srgb(v: f32) -> f32 {
    let v = v.max(0.0);
    if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn process(src: &RgbImage, s: &Settings) -> RgbImage {
    let lut: Vec<f32> = (0..256).map(|i| to_lin(i as f32 / 255.0)).collect();
    let gain = 2f32.powf(s.exposure);
    let t = s.temperature / 100.0;
    let ti = s.tint / 100.0;
    let wb = [gain * (1.0 + 0.25 * t), gain * (1.0 - 0.2 * ti), gain * (1.0 - 0.25 * t)];
    let contrast = 1.0 + s.contrast / 100.0;
    let (w, h) = src.dimensions();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let maxd = (cx * cx + cy * cy).sqrt();
    let mut out = RgbImage::new(w, h);
    out.par_chunks_mut(w as usize * 3).enumerate().for_each(|(y, row)| {
        for x in 0..w as usize {
            let p = src.get_pixel(x as u32, y as u32).0;
            let mut c = [0f32; 3];
            for k in 0..3 {
                c[k] = to_srgb(lut[p[k] as usize] * wb[k]);
            }
            let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            let mut nl = l;
            nl += s.shadows / 100.0 * 0.5 * (1.0 - smooth(0.0, 0.5, nl)) * (1.0 - nl);
            nl += s.highlights / 100.0 * 0.5 * smooth(0.5, 1.0, nl) * nl;
            nl = nl + s.blacks / 100.0 * 0.15 * (1.0 - smooth(0.0, 0.25, nl)) + s.whites / 100.0 * 0.15 * smooth(0.75, 1.0, nl);
            nl = (nl - 0.5) * contrast + 0.5;
            let ratio = if l > 1e-4 { nl / l } else { 1.0 };
            for v in &mut c {
                *v = if l > 1e-4 { *v * ratio } else { nl };
            }
            let l2 = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            let maxc = c[0].max(c[1]).max(c[2]);
            let minc = c[0].min(c[1]).min(c[2]);
            let sat_now = if maxc > 1e-4 { (maxc - minc) / maxc } else { 0.0 };
            let amount = 1.0 + s.saturation / 100.0 + s.vibrance / 100.0 * (1.0 - sat_now);
            for v in &mut c {
                *v = l2 + (*v - l2) * amount.max(0.0);
            }
            if s.vignette != 0.0 {
                let d = (((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / maxd).powi(2);
                let f = 1.0 + s.vignette / 100.0 * d;
                for v in &mut c {
                    *v *= f;
                }
            }
            for k in 0..3 {
                row[x * 3 + k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    });
    match s.rotate % 4 {
        1 => imageops::rotate90(&out),
        2 => imageops::rotate180(&out),
        3 => imageops::rotate270(&out),
        _ => out,
    }
}

/// Red, green, blue and luminance counts, 256 bins each.
pub fn histogram(img: &RgbImage) -> [[u32; 256]; 4] {
    let mut h = [[0u32; 256]; 4];
    for p in img.pixels() {
        let [r, g, b] = p.0;
        h[0][r as usize] += 1;
        h[1][g as usize] += 1;
        h[2][b as usize] += 1;
        let l = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32).round() as usize;
        h[3][l.min(255)] += 1;
    }
    h
}

/// A starting point: exposure that brings the average to mid-grey, a little contrast and lift.
pub fn auto(img: &RgbImage) -> Settings {
    let n = (img.width() * img.height()).max(1) as f64;
    let mean: f64 = img.pixels().map(|p| to_lin((0.2126 * p.0[0] as f32 + 0.7152 * p.0[1] as f32 + 0.0722 * p.0[2] as f32) / 255.0) as f64).sum::<f64>() / n;
    let exposure = ((0.18 / mean.max(1e-4)).log2() as f32).clamp(-3.0, 3.0);
    Settings { exposure: (exposure * 100.0).round() / 100.0, contrast: 12.0, shadows: 15.0, highlights: -15.0, vibrance: 12.0, ..Default::default() }
}

pub fn presets() -> Vec<(&'static str, Settings)> {
    vec![
        ("Naturel", Settings::default()),
        ("Vif", Settings { contrast: 18.0, vibrance: 35.0, saturation: 8.0, ..Default::default() }),
        ("Doux", Settings { contrast: -18.0, highlights: -20.0, shadows: 20.0, saturation: -10.0, ..Default::default() }),
        ("Chaud", Settings { temperature: 30.0, tint: 6.0, vibrance: 10.0, ..Default::default() }),
        ("Froid", Settings { temperature: -28.0, contrast: 8.0, ..Default::default() }),
        ("Noir et blanc", Settings { saturation: -100.0, contrast: 22.0, ..Default::default() }),
        ("Noir et blanc doux", Settings { saturation: -100.0, contrast: -10.0, shadows: 25.0, ..Default::default() }),
        ("Vintage", Settings { temperature: 22.0, contrast: -12.0, blacks: 25.0, saturation: -25.0, vignette: -35.0, ..Default::default() }),
        ("Cinéma", Settings { contrast: 20.0, temperature: -8.0, tint: -6.0, highlights: -25.0, saturation: -12.0, vignette: -25.0, ..Default::default() }),
        ("Haute clé", Settings { exposure: 0.6, contrast: -20.0, highlights: -30.0, whites: 20.0, ..Default::default() }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_is_identity_and_sliders_move() {
        let mut img = RgbImage::new(16, 8);
        for (i, p) in img.pixels_mut().enumerate() {
            let v = (i * 2) as u8;
            p.0 = [v, v.wrapping_add(40), 200];
        }
        let same = process(&img, &Settings::default());
        for (a, b) in img.pixels().zip(same.pixels()) {
            for k in 0..3 {
                assert!(a.0[k].abs_diff(b.0[k]) <= 1);
            }
        }
        let bright = process(&img, &Settings { exposure: 1.0, ..Default::default() });
        assert!(bright.get_pixel(3, 2).0[0] > img.get_pixel(3, 2).0[0]);
        let gray = process(&img, &Settings { saturation: -100.0, ..Default::default() });
        let p = gray.get_pixel(5, 5).0;
        assert!(p[0].abs_diff(p[2]) <= 1);
        let rot = process(&img, &Settings { rotate: 1, ..Default::default() });
        assert_eq!(rot.dimensions(), (8, 16));

        let h = histogram(&img);
        assert_eq!(h[2][200], 128);
        assert_eq!(h[3].iter().sum::<u32>(), 128);
        let dark = RgbImage::from_pixel(10, 10, image::Rgb([20, 20, 20]));
        assert!(auto(&dark).exposure > 1.0);
        let bright = RgbImage::from_pixel(10, 10, image::Rgb([240, 240, 240]));
        assert!(auto(&bright).exposure < 0.0);
        assert_eq!(presets()[0].1, Settings::default());
    }
}
