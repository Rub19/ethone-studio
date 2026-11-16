//! 38 one-click development tools (Ctrl+K « Outil : … ») for the current or chosen photos.
//! Each one gets the photo's current settings and a small preview of the original.

use crate::develop::Settings;
use image::RgbImage;

type Tool = fn(&Settings, &RgbImage) -> Settings;

/// HSL bands: 0 rouges, 1 oranges, 2 jaunes, 3 verts, 4 turquoises, 5 bleus, 6 violets, 7 magentas; [teinte, saturation, luminance].
pub const TOOLS: [(&str, &str, Tool); 38] = [
    ("auto-exposure", "Exposition automatique seule", |s, img| Settings { exposure: crate::develop::auto(img).exposure, ..s.clone() }),
    ("auto-points", "Points noir et blanc automatiques", |s, img| { let (lo, hi) = percentiles(img, 0.005, 0.995); Settings { blacks: (-(lo * 400.0)).clamp(-60.0, 0.0), whites: ((1.0 - hi) * 300.0).clamp(0.0, 60.0), ..s.clone() } }),
    ("auto-contrast", "Contraste automatique", |s, img| { let (lo, hi) = percentiles(img, 0.05, 0.95); Settings { contrast: ((0.7 - (hi - lo)) * 90.0).clamp(-30.0, 50.0), ..s.clone() } }),
    ("auto-vibrance", "Vibrance automatique", |s, img| Settings { vibrance: ((0.32 - mean_sat(img)) * 160.0).clamp(-30.0, 60.0), ..s.clone() }),
    ("auto-balance", "Équilibrer ombres et lumières", |s, img| { let (lo, hi) = percentiles(img, 0.2, 0.8); Settings { shadows: ((0.25 - lo) * 200.0).clamp(0.0, 70.0), highlights: (-(hi - 0.75) * 200.0).clamp(-70.0, 0.0), ..s.clone() } }),
    ("blue-sky", "Ciel plus bleu", |s, _| hsl(s, &[(5, [0.0, 25.0, -18.0]), (4, [0.0, 10.0, -8.0])])),
    ("greenery", "Verdure éclatante", |s, _| hsl(s, &[(3, [-5.0, 28.0, 5.0]), (2, [-8.0, 15.0, 0.0])])),
    ("skin", "Peau douce et lumineuse", |s, _| Settings { texture: -20.0, clarity: -10.0, ..hsl(s, &[(1, [0.0, -6.0, 12.0]), (0, [0.0, -4.0, 4.0])]) }),
    ("sunset", "Coucher de soleil intensifié", |s, _| Settings { temperature: (s.temperature + 15.0).min(100.0), ..hsl(s, &[(0, [0.0, 18.0, 0.0]), (1, [0.0, 25.0, 5.0]), (6, [0.0, 15.0, 0.0])]) }),
    ("day-for-night", "Nuit américaine", |s, _| Settings { exposure: s.exposure - 1.6, temperature: -45.0, saturation: -35.0, contrast: 15.0, ..s.clone() }),
    ("portrait", "Portrait lumineux", |s, _| Settings { exposure: s.exposure + 0.3, shadows: 30.0, clarity: -12.0, texture: -15.0, ..s.clone() }),
    ("landscape", "Paysage net et profond", |s, _| Settings { clarity: 25.0, dehaze: 15.0, vibrance: 22.0, ..s.clone() }),
    ("architecture", "Architecture", |s, _| Settings { contrast: 15.0, clarity: 22.0, saturation: -10.0, sharpen: 45.0, ..s.clone() }),
    ("food", "Cuisine appétissante", |s, _| Settings { temperature: (s.temperature + 10.0).min(100.0), vibrance: 25.0, sharpen: 35.0, shadows: 15.0, ..s.clone() }),
    ("bw-punchy", "Noir et blanc contrasté", |s, _| Settings { mono: true, contrast: 40.0, clarity: 20.0, blacks: -20.0, ..s.clone() }),
    ("bw-soft", "Noir et blanc doux", |s, _| Settings { mono: true, contrast: -15.0, shadows: 25.0, fade: 15.0, ..s.clone() }),
    ("high-key", "Haute clé", |s, _| Settings { exposure: s.exposure + 0.8, highlights: -25.0, contrast: -15.0, shadows: 30.0, ..s.clone() }),
    ("low-key", "Basse clé", |s, _| Settings { exposure: s.exposure - 0.7, shadows: -25.0, vignette: -40.0, contrast: 20.0, ..s.clone() }),
    ("recover-highlights", "Récupérer les hautes lumières", |s, _| Settings { highlights: -85.0, whites: -30.0, ..s.clone() }),
    ("open-shadows", "Déboucher les ombres", |s, _| Settings { shadows: 70.0, blacks: 20.0, ..s.clone() }),
    ("hdr", "Effet HDR", |s, _| Settings { shadows: 65.0, highlights: -65.0, clarity: 40.0, vibrance: 20.0, ..s.clone() }),
    ("matte", "Rendu mat", |s, _| Settings { fade: 30.0, contrast: -10.0, blacks: 10.0, ..s.clone() }),
    ("vintage", "Couleurs délavées vintage", |s, _| Settings { fade: 25.0, saturation: -25.0, temperature: (s.temperature + 12.0).min(100.0), grain: 25.0, ..s.clone() }),
    ("grain", "Grain argentique", |s, _| Settings { grain: 35.0, grain_size: 30.0, grain_rough: 50.0, ..s.clone() }),
    ("vignette-dark", "Vignette sombre", |s, _| Settings { vignette: -45.0, vignette_feather: 40.0, ..s.clone() }),
    ("vignette-light", "Vignette claire", |s, _| Settings { vignette: 35.0, vignette_feather: 40.0, ..s.clone() }),
    ("denoise-strong", "Réduction du bruit forte", |s, _| Settings { noise: 60.0, color_noise: 60.0, ..s.clone() }),
    ("sharpen-web", "Netteté pour le web", |s, _| Settings { sharpen: 60.0, sharpen_radius: 0.6, sharpen_masking: 30.0, ..s.clone() }),
    ("sharpen-print", "Netteté pour l'impression", |s, _| Settings { sharpen: 80.0, sharpen_radius: 1.4, sharpen_detail: 40.0, ..s.clone() }),
    ("tilt-left", "Redresser de 1° vers la gauche", |s, _| Settings { straighten: (s.straighten - 1.0).clamp(-45.0, 45.0), ..s.clone() }),
    ("tilt-right", "Redresser de 1° vers la droite", |s, _| Settings { straighten: (s.straighten + 1.0).clamp(-45.0, 45.0), ..s.clone() }),
    ("crop-square", "Recadrer en carré centré", |s, img| crop_ratio(s, img, 1.0)),
    ("crop-169", "Recadrer en 16:9 centré", |s, img| crop_ratio(s, img, 16.0 / 9.0)),
    ("crop-45", "Recadrer en 4:5 (portrait)", |s, img| crop_ratio(s, img, 4.0 / 5.0)),
    ("reset-color", "Remettre les couleurs à zéro", |s, _| Settings { hsl: [[0.0; 3]; 8], saturation: 0.0, vibrance: 0.0, temperature: 0.0, tint: 0.0, split_shadow_sat: 0.0, split_high_sat: 0.0, split_mid_sat: 0.0, mono: false, ..s.clone() }),
    ("reset-tone", "Remettre les tons à zéro", |s, _| Settings { exposure: 0.0, contrast: 0.0, highlights: 0.0, shadows: 0.0, whites: 0.0, blacks: 0.0, ..s.clone() }),
    ("reset-geometry", "Remettre la géométrie à zéro", |s, _| { let d = Settings::default(); Settings { crop: d.crop, straighten: 0.0, rotate: 0, flip: false, flip_v: false, ..s.clone() } }),
    ("invert-wb", "Inverser la balance des blancs (chaud ↔ froid)", |s, _| Settings { temperature: -s.temperature, tint: -s.tint, ..if s.temperature == 0.0 && s.tint == 0.0 { Settings { temperature: 20.0, ..s.clone() } } else { s.clone() } }),
];

fn luma(p: &image::Rgb<u8>) -> f32 {
    (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0
}

fn percentiles(img: &RgbImage, lo: f32, hi: f32) -> (f32, f32) {
    let mut hist = [0u32; 256];
    for p in img.pixels() {
        hist[(luma(p) * 255.0) as usize] += 1;
    }
    let total = hist.iter().sum::<u32>().max(1) as f32;
    let at = |q: f32| {
        let mut acc = 0.0;
        for (i, n) in hist.iter().enumerate() {
            acc += *n as f32;
            if acc / total >= q {
                return i as f32 / 255.0;
            }
        }
        1.0
    };
    (at(lo), at(hi))
}

fn mean_sat(img: &RgbImage) -> f32 {
    let n = (img.width() * img.height()).max(1) as f32;
    img.pixels().map(|p| { let (mx, mn) = (p[0].max(p[1]).max(p[2]) as f32, p[0].min(p[1]).min(p[2]) as f32); if mx > 0.0 { (mx - mn) / mx } else { 0.0 } }).sum::<f32>() / n
}

fn hsl(s: &Settings, bands: &[(usize, [f32; 3])]) -> Settings {
    let mut out = s.clone();
    for (b, v) in bands {
        for k in 0..3 {
            out.hsl[*b][k] = (out.hsl[*b][k] + v[k]).clamp(-100.0, 100.0);
        }
    }
    out
}

/// Centred crop to an aspect ratio; `crop` holds the margins left, top, right, bottom (fractions).
fn crop_ratio(s: &Settings, img: &RgbImage, ratio: f32) -> Settings {
    let (w, h) = (img.width().max(1) as f32, img.height().max(1) as f32);
    let (mx, my) = if w / h > ratio { ((1.0 - ratio * h / w) / 2.0, 0.0) } else { (0.0, (1.0 - w / ratio / h) / 2.0) };
    Settings { crop: [mx, my, mx, my], ..s.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_changes_the_settings_and_renders() {
        let img = RgbImage::from_fn(30, 20, |x, y| image::Rgb([(x * 8) as u8, (y * 13) as u8, (x * 4) as u8]));
        let base = Settings { temperature: 5.0, exposure: 0.4, contrast: 10.0, saturation: 5.0, straighten: 2.0, crop: [0.02, 0.03, 0.02, 0.01], ..Settings::default() };
        let mut ids = std::collections::HashSet::new();
        for (id, label, f) in TOOLS {
            assert!(ids.insert(id), "{id} en double");
            let s = f(&base, &img);
            assert!(s != base, "{label} ne change rien");
            let out = crate::develop::process(&img, &s);
            assert!(out.width() > 0 && out.height() > 0, "{label}");
        }
        let sq = crate::develop::process(&img, &crop_ratio(&Settings::default(), &img, 1.0));
        assert!((sq.width() as i32 - sq.height() as i32).abs() <= 1, "{}×{}", sq.width(), sq.height());
    }
}
