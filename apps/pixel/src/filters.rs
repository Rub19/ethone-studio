use crate::doc::{hsl_to_rgb, rgb_to_hsl};
use image::{RgbaImage, imageops};
use rayon::prelude::*;

pub struct Param {
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub suffix: &'static str,
}

const fn p(label: &'static str, min: f32, max: f32, default: f32, suffix: &'static str) -> Param {
    Param { label, min, max, default, suffix }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    BrightnessContrast,
    HueSat,
    Levels,
    Exposure,
    Vibrance,
    Temperature,
    ShadowsHighlights,
    Threshold,
    Posterize,
    BlackWhite,
    Invert,
    Sepia,
    AutoContrast,
    Equalize,
    Solarize,
    Blur,
    MotionBlur,
    RadialBlur,
    Unsharp,
    Noise,
    Denoise,
    Pixelate,
    Emboss,
    FindEdges,
    Vignette,
    Glow,
    Chromatic,
}

pub const ADJUSTMENTS: &[Kind] = &[
    Kind::BrightnessContrast,
    Kind::Levels,
    Kind::Exposure,
    Kind::ShadowsHighlights,
    Kind::HueSat,
    Kind::Vibrance,
    Kind::Temperature,
    Kind::Threshold,
    Kind::Posterize,
];
pub const QUICK: &[Kind] = &[Kind::AutoContrast, Kind::Equalize, Kind::BlackWhite, Kind::Sepia, Kind::Invert, Kind::Solarize];
pub const FILTERS: &[(&str, &[Kind])] = &[
    ("Flou", &[Kind::Blur, Kind::MotionBlur, Kind::RadialBlur]),
    ("Netteté", &[Kind::Unsharp]),
    ("Bruit", &[Kind::Noise, Kind::Denoise]),
    ("Pixellisation", &[Kind::Pixelate]),
    ("Esthétiques", &[Kind::Emboss, Kind::FindEdges, Kind::Glow, Kind::Chromatic]),
    ("Rendu", &[Kind::Vignette]),
];

impl Kind {
    pub fn title(self) -> &'static str {
        match self {
            Kind::BrightnessContrast => "Luminosité et contraste",
            Kind::HueSat => "Teinte et saturation",
            Kind::Levels => "Niveaux",
            Kind::Exposure => "Exposition",
            Kind::Vibrance => "Vibrance",
            Kind::Temperature => "Température et teinte",
            Kind::ShadowsHighlights => "Tons foncés et tons clairs",
            Kind::Threshold => "Seuil",
            Kind::Posterize => "Postérisation",
            Kind::BlackWhite => "Noir et blanc",
            Kind::Invert => "Négatif",
            Kind::Sepia => "Sépia",
            Kind::AutoContrast => "Contraste automatique",
            Kind::Equalize => "Égaliser",
            Kind::Solarize => "Solarisation",
            Kind::Blur => "Flou gaussien",
            Kind::MotionBlur => "Flou de mouvement",
            Kind::RadialBlur => "Flou radial",
            Kind::Unsharp => "Accentuation",
            Kind::Noise => "Ajout de bruit",
            Kind::Denoise => "Réduction du bruit",
            Kind::Pixelate => "Mosaïque",
            Kind::Emboss => "Estampage",
            Kind::FindEdges => "Contours",
            Kind::Vignette => "Vignettage",
            Kind::Glow => "Lueur diffuse",
            Kind::Chromatic => "Aberration chromatique",
        }
    }

    pub fn params(self) -> &'static [Param] {
        match self {
            Kind::BrightnessContrast => const { &[p("Luminosité", -100.0, 100.0, 0.0, ""), p("Contraste", -100.0, 100.0, 0.0, "")] },
            Kind::HueSat => const { &[p("Teinte", -180.0, 180.0, 0.0, "°"), p("Saturation", -100.0, 100.0, 0.0, ""), p("Luminosité", -100.0, 100.0, 0.0, "")] },
            Kind::Levels => const { &[p("Noir", 0.0, 250.0, 0.0, ""), p("Gamma", 0.1, 3.0, 1.0, ""), p("Blanc", 5.0, 255.0, 255.0, "")] },
            Kind::Exposure => const { &[p("Exposition", -4.0, 4.0, 0.0, " IL"), p("Décalage", -0.5, 0.5, 0.0, ""), p("Gamma", 0.2, 3.0, 1.0, "")] },
            Kind::Vibrance => const { &[p("Vibrance", -100.0, 100.0, 0.0, ""), p("Saturation", -100.0, 100.0, 0.0, "")] },
            Kind::Temperature => const { &[p("Température", -100.0, 100.0, 0.0, ""), p("Teinte", -100.0, 100.0, 0.0, "")] },
            Kind::ShadowsHighlights => const { &[p("Tons foncés", 0.0, 100.0, 35.0, " %"), p("Tons clairs", 0.0, 100.0, 0.0, " %")] },
            Kind::Threshold => const { &[p("Seuil", 1.0, 255.0, 128.0, "")] },
            Kind::Posterize => const { &[p("Niveaux", 2.0, 16.0, 4.0, "")] },
            Kind::Blur => const { &[p("Rayon", 0.5, 60.0, 4.0, " px")] },
            Kind::MotionBlur => const { &[p("Distance", 2.0, 120.0, 20.0, " px"), p("Angle", -180.0, 180.0, 0.0, "°")] },
            Kind::RadialBlur => const { &[p("Intensité", 1.0, 100.0, 20.0, "")] },
            Kind::Unsharp => const { &[p("Gain", 0.0, 300.0, 80.0, " %"), p("Rayon", 0.5, 20.0, 1.5, " px")] },
            Kind::Noise => const { &[p("Quantité", 0.0, 100.0, 12.0, " %"), p("Monochrome", 0.0, 1.0, 1.0, "")] },
            Kind::Denoise => const { &[p("Rayon", 1.0, 3.0, 1.0, " px")] },
            Kind::Pixelate => const { &[p("Taille de cellule", 2.0, 120.0, 12.0, " px")] },
            Kind::Emboss => const { &[p("Relief", 0.5, 6.0, 2.0, "")] },
            Kind::Vignette => const { &[p("Quantité", -100.0, 100.0, -50.0, ""), p("Taille", 0.0, 100.0, 50.0, " %")] },
            Kind::Glow => const { &[p("Rayon", 2.0, 80.0, 18.0, " px"), p("Intensité", 0.0, 100.0, 45.0, " %")] },
            Kind::Chromatic => const { &[p("Décalage", 1.0, 30.0, 4.0, " px")] },
            _ => &[],
        }
    }

    pub fn defaults(self) -> Vec<f32> {
        self.params().iter().map(|p| p.default).collect()
    }

    pub fn apply(self, img: &RgbaImage, v: &[f32]) -> RgbaImage {
        let g = |i: usize| v.get(i).copied().unwrap_or(0.0);
        match self {
            Kind::BrightnessContrast => {
                let (b, k) = (g(0) / 100.0, ((g(1) / 100.0) + 1.0).powi(2));
                map(img, move |c| c.map(|x| (x + b - 0.5) * k + 0.5))
            }
            Kind::HueSat => {
                let (h, s, l) = (g(0) / 360.0, g(1) / 100.0, g(2) / 100.0);
                map(img, move |c| {
                    let (hh, ss, ll) = rgb_to_hsl(c[0], c[1], c[2]);
                    let ll = if l >= 0.0 { ll + (1.0 - ll) * l } else { ll * (1.0 + l) };
                    let (r, g, b) = hsl_to_rgb(hh + h, (ss * (1.0 + s)).clamp(0.0, 1.0), ll);
                    [r, g, b]
                })
            }
            Kind::Levels => {
                let (lo, gamma, hi) = (g(0) / 255.0, g(1).max(0.05), (g(2) / 255.0).max(g(0) / 255.0 + 0.01));
                map(img, move |c| c.map(|x| ((x - lo) / (hi - lo)).clamp(0.0, 1.0).powf(1.0 / gamma)))
            }
            Kind::Exposure => {
                let (gain, off, gamma) = (2f32.powf(g(0)), g(1), g(2).max(0.05));
                map(img, move |c| c.map(|x| (srgb_to_lin(x) * gain + off).max(0.0).powf(1.0 / gamma)).map(lin_to_srgb))
            }
            Kind::Vibrance => {
                let (vib, sat) = (g(0) / 100.0, g(1) / 100.0);
                map(img, move |c| {
                    let (h, s, l) = rgb_to_hsl(c[0], c[1], c[2]);
                    let s = (s * (1.0 + vib * (1.0 - s) * 1.5)).clamp(0.0, 1.0);
                    let (r, g, b) = hsl_to_rgb(h, (s * (1.0 + sat)).clamp(0.0, 1.0), l);
                    [r, g, b]
                })
            }
            Kind::Temperature => {
                let (t, tint) = (g(0) / 100.0 * 0.18, g(1) / 100.0 * 0.12);
                map(img, move |c| [c[0] + t, c[1] - tint, c[2] - t])
            }
            Kind::ShadowsHighlights => {
                let (sh, hi) = (g(0) / 100.0 * 0.6, g(1) / 100.0 * 0.6);
                map(img, move |c| {
                    let l = luma(c);
                    let d = sh * (1.0 - l).powi(3) - hi * l.powi(3);
                    c.map(|x| x + d)
                })
            }
            Kind::Threshold => {
                let t = g(0) / 255.0;
                map(img, move |c| [if luma(c) >= t { 1.0 } else { 0.0 }; 3])
            }
            Kind::Posterize => {
                let n = g(0).round().max(2.0) - 1.0;
                map(img, move |c| c.map(|x| (x * n).round() / n))
            }
            Kind::BlackWhite => map(img, |c| [luma(c); 3]),
            Kind::Invert => map(img, |c| c.map(|x| 1.0 - x)),
            Kind::Sepia => map(img, |c| {
                [
                    0.393 * c[0] + 0.769 * c[1] + 0.189 * c[2],
                    0.349 * c[0] + 0.686 * c[1] + 0.168 * c[2],
                    0.272 * c[0] + 0.534 * c[1] + 0.131 * c[2],
                ]
            }),
            Kind::Solarize => map(img, |c| c.map(|x| if x > 0.5 { 1.0 - x } else { x })),
            Kind::AutoContrast => {
                let mut lo = [0f32; 3];
                let mut hi = [1f32; 3];
                for ch in 0..3 {
                    let mut hist = [0u32; 256];
                    img.pixels().filter(|p| p.0[3] > 0).for_each(|p| hist[p.0[ch] as usize] += 1);
                    let total: u32 = hist.iter().sum();
                    let cut = (total as f32 * 0.005) as u32;
                    lo[ch] = percentile(&hist, cut) as f32 / 255.0;
                    hi[ch] = (255 - percentile(&rev(&hist), cut)) as f32 / 255.0;
                    if hi[ch] - lo[ch] < 0.02 {
                        (lo[ch], hi[ch]) = (0.0, 1.0);
                    }
                }
                map(img, move |c| std::array::from_fn(|k| (c[k] - lo[k]) / (hi[k] - lo[k])))
            }
            Kind::Equalize => {
                let mut hist = [0u32; 256];
                img.pixels().filter(|p| p.0[3] > 0).for_each(|p| hist[(luma(rgb(p.0)) * 255.0).round() as usize] += 1);
                let total = hist.iter().sum::<u32>().max(1) as f32;
                let mut cdf = [0f32; 256];
                let mut acc = 0u32;
                for (i, h) in hist.iter().enumerate() {
                    acc += h;
                    cdf[i] = acc as f32 / total;
                }
                map(img, move |c| {
                    let l = luma(c).clamp(0.0, 1.0);
                    let target = cdf[(l * 255.0).round() as usize];
                    let k = if l > 1e-3 { target / l } else { 0.0 };
                    if l > 1e-3 { c.map(|x| x * k) } else { [target; 3] }
                })
            }
            Kind::Blur => imageops::fast_blur(img, g(0)),
            Kind::MotionBlur => {
                let (dist, a) = (g(0), g(1).to_radians());
                let n = dist.round().max(2.0) as i32;
                let (dx, dy) = (a.cos(), a.sin());
                sample(img, move |x, y, at| {
                    let mut acc = [0f32; 4];
                    for i in 0..n {
                        let t = i as f32 - n as f32 / 2.0;
                        add(&mut acc, at(x as f32 + dx * t, y as f32 + dy * t));
                    }
                    acc.map(|v| v / n as f32)
                })
            }
            Kind::RadialBlur => {
                let amount = g(0) / 100.0 * 0.25;
                let (cx, cy) = (img.width() as f32 / 2.0, img.height() as f32 / 2.0);
                sample(img, move |x, y, at| {
                    let n = 24;
                    let mut acc = [0f32; 4];
                    for i in 0..n {
                        let s = 1.0 - amount * i as f32 / n as f32;
                        add(&mut acc, at(cx + (x as f32 - cx) * s, cy + (y as f32 - cy) * s));
                    }
                    acc.map(|v| v / n as f32)
                })
            }
            Kind::Unsharp => {
                let amount = g(0) / 100.0;
                let blurred = imageops::fast_blur(img, g(1));
                zip(img, &blurred, move |a, b| std::array::from_fn(|k| a[k] + (a[k] - b[k]) * amount))
            }
            Kind::Noise => {
                let (amount, mono) = (g(0) / 100.0 * 0.5, g(1) >= 0.5);
                let w = img.width();
                let mut out = img.clone();
                out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
                    let (x, y) = (i as u32 % w, i as u32 / w);
                    let n = |k: u32| hash(x, y, k) - 0.5;
                    for c in 0..3 {
                        let d = if mono { n(0) } else { n(c as u32) } * amount;
                        px[c] = ((px[c] as f32 / 255.0 + d).clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                });
                out
            }
            Kind::Denoise => {
                let r = g(0).round().max(1.0) as i32;
                let (w, h) = (img.width() as i32, img.height() as i32);
                let src = img.as_raw();
                let mut out = img.clone();
                out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
                    let (x, y) = (i as i32 % w, i as i32 / w);
                    let mut vals: [Vec<u8>; 3] = Default::default();
                    for yy in (y - r).max(0)..=(y + r).min(h - 1) {
                        for xx in (x - r).max(0)..=(x + r).min(w - 1) {
                            let o = ((yy * w + xx) * 4) as usize;
                            for c in 0..3 {
                                vals[c].push(src[o + c]);
                            }
                        }
                    }
                    for c in 0..3 {
                        let m = vals[c].len() / 2;
                        px[c] = *vals[c].select_nth_unstable(m).1;
                    }
                });
                out
            }
            Kind::Pixelate => {
                let s = g(0).round().max(2.0) as u32;
                let (w, h) = img.dimensions();
                let mut out = img.clone();
                for by in (0..h).step_by(s as usize) {
                    for bx in (0..w).step_by(s as usize) {
                        let (x1, y1) = ((bx + s).min(w), (by + s).min(h));
                        let mut acc = [0u64; 4];
                        for y in by..y1 {
                            for x in bx..x1 {
                                for (a, v) in acc.iter_mut().zip(img.get_pixel(x, y).0) {
                                    *a += v as u64;
                                }
                            }
                        }
                        let n = ((x1 - bx) * (y1 - by)) as u64;
                        let avg = image::Rgba(acc.map(|a| (a / n) as u8));
                        for y in by..y1 {
                            for x in bx..x1 {
                                out.put_pixel(x, y, avg);
                            }
                        }
                    }
                }
                out
            }
            Kind::Emboss => {
                let k = g(0);
                sample(img, move |x, y, at| {
                    let (a, b, c) = (at(x as f32 - 1.0, y as f32 - 1.0), at(x as f32 + 1.0, y as f32 + 1.0), at(x as f32, y as f32));
                    let d = (luma(rgb4(b)) - luma(rgb4(a))) * k;
                    [0.5 + d, 0.5 + d, 0.5 + d, c[3]]
                })
            }
            Kind::FindEdges => sample(img, |x, y, at| {
                let l = |dx: f32, dy: f32| luma(rgb4(at(x as f32 + dx, y as f32 + dy)));
                let gx = l(1.0, -1.0) + 2.0 * l(1.0, 0.0) + l(1.0, 1.0) - l(-1.0, -1.0) - 2.0 * l(-1.0, 0.0) - l(-1.0, 1.0);
                let gy = l(-1.0, 1.0) + 2.0 * l(0.0, 1.0) + l(1.0, 1.0) - l(-1.0, -1.0) - 2.0 * l(0.0, -1.0) - l(1.0, -1.0);
                let e = 1.0 - (gx * gx + gy * gy).sqrt().min(1.0);
                [e, e, e, at(x as f32, y as f32)[3]]
            }),
            Kind::Vignette => {
                let (amount, size) = (g(0) / 100.0, g(1) / 100.0);
                let (w, h) = (img.width() as f32, img.height() as f32);
                let mut out = img.clone();
                let ww = img.width();
                out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
                    let (x, y) = ((i as u32 % ww) as f32 + 0.5, (i as u32 / ww) as f32 + 0.5);
                    let d = (((x / w - 0.5) * 2.0).powi(2) + ((y / h - 0.5) * 2.0).powi(2)).sqrt() / std::f32::consts::SQRT_2;
                    let start = 0.15 + size * 0.6;
                    let t = ((d - start) / (1.0 - start + 1e-3)).clamp(0.0, 1.0);
                    let t = t * t * (3.0 - 2.0 * t) * amount.abs();
                    for c in 0..3 {
                        let v = px[c] as f32 / 255.0;
                        let v = if amount < 0.0 { v * (1.0 - t) } else { v + (1.0 - v) * t };
                        px[c] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                });
                out
            }
            Kind::Glow => {
                let k = g(1) / 100.0;
                let blurred = imageops::fast_blur(img, g(0));
                zip(img, &blurred, move |a, b| std::array::from_fn(|c| {
                    let glow = (b[c] - 0.35).max(0.0) / 0.65 * k;
                    1.0 - (1.0 - a[c]) * (1.0 - glow)
                }))
            }
            Kind::Chromatic => {
                let s = g(0);
                sample(img, move |x, y, at| {
                    let (x, y) = (x as f32, y as f32);
                    let c = at(x, y);
                    [at(x - s, y)[0], c[1], at(x + s, y)[2], c[3]]
                })
            }
        }
    }
}

fn rgb(p: [u8; 4]) -> [f32; 3] {
    [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0]
}

fn rgb4(p: [f32; 4]) -> [f32; 3] {
    [p[0], p[1], p[2]]
}

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn srgb_to_lin(x: f32) -> f32 {
    if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) }
}

fn lin_to_srgb(x: f32) -> f32 {
    if x <= 0.0031308 { x * 12.92 } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 }
}

fn percentile(hist: &[u32; 256], cut: u32) -> usize {
    let mut acc = 0;
    for (i, h) in hist.iter().enumerate() {
        acc += h;
        if acc > cut {
            return i;
        }
    }
    255
}

fn rev(hist: &[u32; 256]) -> [u32; 256] {
    std::array::from_fn(|i| hist[255 - i])
}

fn hash(x: u32, y: u32, k: u32) -> f32 {
    let mut h = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ k.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xFFFF) as f32 / 65535.0
}

fn add(acc: &mut [f32; 4], p: [f32; 4]) {
    for k in 0..4 {
        acc[k] += p[k];
    }
}

fn map(img: &RgbaImage, f: impl Fn([f32; 3]) -> [f32; 3] + Sync) -> RgbaImage {
    let mut out = img.clone();
    out.par_chunks_mut(4).for_each(|px| {
        let c = f([px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0]);
        for k in 0..3 {
            px[k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    });
    out
}

fn zip(a: &RgbaImage, b: &RgbaImage, f: impl Fn([f32; 3], [f32; 3]) -> [f32; 3] + Sync) -> RgbaImage {
    let mut out = a.clone();
    out.par_chunks_mut(4).zip(b.par_chunks(4)).for_each(|(px, q)| {
        let c = f([px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0], [q[0] as f32 / 255.0, q[1] as f32 / 255.0, q[2] as f32 / 255.0]);
        for k in 0..3 {
            px[k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    });
    out
}

fn sample(img: &RgbaImage, f: impl Fn(u32, u32, &dyn Fn(f32, f32) -> [f32; 4]) -> [f32; 4] + Sync) -> RgbaImage {
    let (w, h) = img.dimensions();
    let src = img.as_raw();
    let at = |x: f32, y: f32| {
        let xi = (x.round() as i64).clamp(0, w as i64 - 1) as usize;
        let yi = (y.round() as i64).clamp(0, h as i64 - 1) as usize;
        let o = (yi * w as usize + xi) * 4;
        [src[o] as f32 / 255.0, src[o + 1] as f32 / 255.0, src[o + 2] as f32 / 255.0, src[o + 3] as f32 / 255.0]
    };
    let mut out = img.clone();
    out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
        let c = f(i as u32 % w, i as u32 / w, &at);
        for k in 0..4 {
            px[k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_filter_runs_and_keeps_alpha() {
        let img = RgbaImage::from_fn(40, 30, |x, y| image::Rgba([(x * 6) as u8, (y * 8) as u8, 120, if x < 3 { 0 } else { 255 }]));
        let all = ADJUSTMENTS.iter().chain(QUICK).chain(FILTERS.iter().flat_map(|f| f.1.iter()));
        for k in all {
            let out = k.apply(&img, &k.defaults());
            assert_eq!(out.dimensions(), img.dimensions(), "{k:?}");
            if !matches!(k, Kind::Blur | Kind::MotionBlur | Kind::RadialBlur | Kind::Pixelate | Kind::Unsharp | Kind::Glow | Kind::Chromatic) {
                assert_eq!(out.get_pixel(1, 5).0[3], 0, "{k:?}");
            }
        }
        let same = Kind::BrightnessContrast.apply(&img, &[0.0, 0.0]);
        assert_eq!(same.get_pixel(20, 20), img.get_pixel(20, 20));
        let inv = Kind::Invert.apply(&img, &[]);
        assert_eq!(inv.get_pixel(10, 10).0[2], 135);
        let th = Kind::Threshold.apply(&img, &[128.0]);
        assert!(th.pixels().all(|p| p.0[0] == 0 || p.0[0] == 255));
    }
}
