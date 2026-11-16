use crate::develop::{self, Settings};
use image::{ImageDecoder, RgbImage, imageops};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub const FIELDS: &[(&str, &str, usize)] = &[
    ("profile", "Profil", 0),
    ("mono", "Noir et blanc", 0),
    ("exposure", "Exposition", 0),
    ("contrast", "Contraste", 0),
    ("highlights", "Hautes lumières", 0),
    ("shadows", "Ombres", 0),
    ("whites", "Blancs", 0),
    ("blacks", "Noirs", 0),
    ("temperature", "Température", 1),
    ("tint", "Teinte", 1),
    ("vibrance", "Vibrance", 1),
    ("saturation", "Saturation", 1),
    ("texture", "Texture", 2),
    ("clarity", "Clarté", 2),
    ("dehaze", "Correction du voile", 2),
    ("curve", "Courbe paramétrique", 3),
    ("curve_rgb", "Courbe RVB", 3),
    ("curve_r", "Courbe rouge", 3),
    ("curve_g", "Courbe verte", 3),
    ("curve_b", "Courbe bleue", 3),
    ("hsl", "TSL", 4),
    ("split_shadow_hue", "Étalonnage des ombres", 5),
    ("split_shadow_sat", "Étalonnage des ombres", 5),
    ("split_high_hue", "Étalonnage des hautes lumières", 5),
    ("split_high_sat", "Étalonnage des hautes lumières", 5),
    ("split_mid_hue", "Étalonnage des tons moyens", 5),
    ("split_mid_sat", "Étalonnage des tons moyens", 5),
    ("split_balance", "Équilibre", 5),
    ("grade_global_hue", "Étalonnage global", 5),
    ("grade_global_sat", "Étalonnage global", 5),
    ("grade_shadow_lum", "Luminance des ombres", 5),
    ("grade_mid_lum", "Luminance des tons moyens", 5),
    ("grade_high_lum", "Luminance des hautes lumières", 5),
    ("grade_global_lum", "Luminance globale", 5),
    ("grade_blend", "Fusion", 5),
    ("sharpen", "Netteté", 6),
    ("sharpen_radius", "Rayon de netteté", 6),
    ("sharpen_detail", "Détail de netteté", 6),
    ("sharpen_masking", "Masquage de netteté", 6),
    ("noise", "Réduction du bruit", 6),
    ("noise_detail", "Détail du bruit", 6),
    ("color_noise", "Bruit de couleur", 6),
    ("color_noise_detail", "Détail du bruit de couleur", 6),
    ("color_noise_smooth", "Lissage du bruit de couleur", 6),
    ("lens_vignette", "Vignetage de l'objectif", 7),
    ("lens_vignette_mid", "Point milieu du vignetage", 7),
    ("distortion", "Distorsion", 7),
    ("ca_red", "Aberration rouge/cyan", 7),
    ("ca_blue", "Aberration bleu/jaune", 7),
    ("defringe_purple", "Frange violette", 7),
    ("defringe_green", "Frange verte", 7),
    ("persp_v", "Perspective verticale", 8),
    ("persp_h", "Perspective horizontale", 8),
    ("persp_rotate", "Rotation", 8),
    ("persp_aspect", "Aspect", 8),
    ("persp_scale", "Échelle", 8),
    ("persp_x", "Décalage X", 8),
    ("persp_y", "Décalage Y", 8),
    ("flip_v", "Miroir vertical", 8),
    ("straighten", "Redressement", 9),
    ("crop", "Recadrage", 9),
    ("rotate", "Rotation 90°", 9),
    ("flip", "Miroir horizontal", 9),
    ("vignette", "Vignettage", 10),
    ("vignette_mid", "Point milieu", 10),
    ("vignette_style", "Style de vignettage", 10),
    ("vignette_round", "Arrondi", 10),
    ("vignette_feather", "Contour progressif", 10),
    ("vignette_highlights", "Hautes lumières du vignettage", 10),
    ("grain", "Grain", 10),
    ("grain_size", "Taille du grain", 10),
    ("grain_rough", "Rugosité du grain", 10),
    ("glow", "Halo lumineux", 10),
    ("fade", "Délavé", 10),
    ("lut", "LUT pellicule", 10),
    ("lut_amount", "Intensité du LUT", 10),
    ("cal_shadow_tint", "Teinte des ombres (étalonnage)", 11),
    ("cal_red_hue", "Primaire rouge", 11),
    ("cal_red_sat", "Primaire rouge", 11),
    ("cal_green_hue", "Primaire verte", 11),
    ("cal_green_sat", "Primaire verte", 11),
    ("cal_blue_hue", "Primaire bleue", 11),
    ("cal_blue_sat", "Primaire bleue", 11),
    ("masks", "Masques", 12),
    ("creative", "Effets créatifs", 10),
    ("grad_on", "Filtre gradué", 12),
    ("grad_angle", "Filtre gradué", 12),
    ("grad_pos", "Filtre gradué", 12),
    ("grad_feather", "Filtre gradué", 12),
    ("grad_exposure", "Filtre gradué", 12),
    ("grad_temp", "Filtre gradué", 12),
    ("grad_sat", "Filtre gradué", 12),
    ("radial_on", "Filtre radial", 12),
    ("radial_x", "Filtre radial", 12),
    ("radial_y", "Filtre radial", 12),
    ("radial_size", "Filtre radial", 12),
    ("radial_feather", "Filtre radial", 12),
    ("radial_exposure", "Filtre radial", 12),
    ("radial_temp", "Filtre radial", 12),
    ("radial_sat", "Filtre radial", 12),
    ("radial_invert", "Filtre radial", 12),
    ("spots", "Retouche des taches", 13),
    ("red_eyes", "Yeux rouges", 13),
];

pub const GROUPS: [&str; 14] = ["Profil et lumière", "Couleur", "Présence", "Courbe des tonalités", "TSL", "Étalonnage", "Détail", "Optique", "Transformation", "Recadrage et rotation", "Effets", "Étalonnage de l'appareil", "Masques", "Retouche"];

fn to_value(s: &Settings) -> Value {
    serde_json::to_value(s).unwrap_or(Value::Null)
}

fn from_value(v: Value) -> Settings {
    serde_json::from_value(v).unwrap_or_default()
}

pub fn changed_label(a: &Settings, b: &Settings) -> String {
    let (va, vb) = (to_value(a), to_value(b));
    let mut names: Vec<&str> = Vec::new();
    for (k, label, _) in FIELDS {
        if va.get(k) != vb.get(k) && !names.contains(label) {
            names.push(label);
        }
    }
    match names.len() {
        0 => "Réglage".into(),
        1 => names[0].into(),
        2 => format!("{} et {}", names[0], names[1]),
        n => format!("{} et {} autres", names[0], n - 1),
    }
}

pub fn copy_groups(dst: &Settings, src: &Settings, groups: &[bool]) -> Settings {
    let (mut vd, vs) = (to_value(dst), to_value(src));
    if let (Some(d), Some(s)) = (vd.as_object_mut(), vs.as_object()) {
        for (k, _, g) in FIELDS {
            if groups.get(*g).copied().unwrap_or(false)
                && let Some(v) = s.get(*k)
            {
                d.insert(k.to_string(), v.clone());
            }
        }
    }
    from_value(vd)
}

fn lerp_value(a: &Value, b: &Value, t: f64) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let int = x.is_u64() && y.is_u64();
            let (x, y) = (x.as_f64().unwrap_or(0.0), y.as_f64().unwrap_or(0.0));
            let v = x + (y - x) * t;
            if int { Value::from(v.round().max(0.0) as u64) } else { serde_json::Number::from_f64(v).map(Value::Number).unwrap_or(b.clone()) }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => Value::Array(x.iter().zip(y).map(|(p, q)| lerp_value(p, q, t)).collect()),
        (Value::Object(x), Value::Object(y)) => Value::Object(y.iter().map(|(k, q)| (k.clone(), x.get(k).map(|p| lerp_value(p, q, t)).unwrap_or(q.clone()))).collect()),
        _ => {
            if t >= 0.5 {
                b.clone()
            } else {
                a.clone()
            }
        }
    }
}

pub fn blend(a: &Settings, b: &Settings, t: f32) -> Settings {
    let mut s = from_value(lerp_value(&to_value(a), &to_value(b), t as f64));
    s.rotate = b.rotate;
    s.profile = if t >= 0.5 { b.profile } else { a.profile };
    s.vignette_style = b.vignette_style;
    s
}

fn exif_bytes(path: &Path) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut buf = Vec::new();
    std::fs::File::open(path).ok()?.take(1 << 20).read_to_end(&mut buf).ok()?;
    if buf.starts_with(b"II*\0") || buf.starts_with(b"MM\0*") {
        return Some(buf);
    }
    if buf.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        while i + 4 < buf.len() && buf[i] == 0xFF {
            let marker = buf[i + 1];
            let len = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
            if marker == 0xDA || len < 2 {
                break;
            }
            let data = &buf[i + 4..(i + 2 + len).min(buf.len())];
            if marker == 0xE1 && data.starts_with(b"Exif\0\0") {
                return Some(data[6..].to_vec());
            }
            i += 2 + len;
        }
        return None;
    }
    image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.into_decoder().ok()?.exif_metadata().ok().flatten()
}

struct Tiff<'a> {
    b: &'a [u8],
    le: bool,
}

impl Tiff<'_> {
    fn u16(&self, o: usize) -> Option<u16> {
        let s = self.b.get(o..o + 2)?;
        Some(if self.le { u16::from_le_bytes([s[0], s[1]]) } else { u16::from_be_bytes([s[0], s[1]]) })
    }
    fn u32(&self, o: usize) -> Option<u32> {
        let s = self.b.get(o..o + 4)?;
        let a = [s[0], s[1], s[2], s[3]];
        Some(if self.le { u32::from_le_bytes(a) } else { u32::from_be_bytes(a) })
    }
    fn entries(&self, ifd: usize, out: &mut Vec<(u16, Val)>, depth: u8) {
        let Some(n) = self.u16(ifd) else { return };
        for k in 0..n.min(400) as usize {
            let e = ifd + 2 + k * 12;
            let (Some(tag), Some(typ), Some(count)) = (self.u16(e), self.u16(e + 2), self.u32(e + 4)) else { return };
            let size = match typ {
                1 | 2 | 6 | 7 => 1,
                3 | 8 => 2,
                4 | 9 => 4,
                5 | 10 => 8,
                _ => continue,
            } * count as usize;
            let at = if size <= 4 { e + 8 } else { self.u32(e + 8).unwrap_or(0) as usize };
            let v = match typ {
                2 => Val::Text(String::from_utf8_lossy(self.b.get(at..at + size).unwrap_or_default()).trim_end_matches('\0').trim().to_string()),
                3 => Val::Num(self.u16(at).unwrap_or(0) as f64),
                4 => Val::Num(self.u32(at).unwrap_or(0) as f64),
                5 => Val::Num(self.u32(at).unwrap_or(0) as f64 / self.u32(at + 4).unwrap_or(1).max(1) as f64),
                10 => Val::Num(self.u32(at).unwrap_or(0) as i32 as f64 / (self.u32(at + 4).unwrap_or(1) as i32).max(1) as f64),
                _ => Val::Num(0.0),
            };
            if (tag == 0x8769 || tag == 0x8825) && depth < 2 {
                out.push((tag, Val::Num(1.0)));
                if let Val::Num(off) = v {
                    self.entries(off as usize, out, depth + 1);
                }
                continue;
            }
            out.push((tag, v));
        }
    }
}

#[derive(Clone)]
enum Val {
    Text(String),
    Num(f64),
}

pub fn read_exif(path: &Path) -> Vec<(&'static str, String)> {
    let Some(b) = exif_bytes(path) else { return Vec::new() };
    let le = b.starts_with(b"II");
    let t = Tiff { b: &b, le };
    let mut tags = Vec::new();
    if let Some(first) = t.u32(4) {
        t.entries(first as usize, &mut tags, 0);
    }
    let get = |id: u16| tags.iter().find(|(t, _)| *t == id).map(|(_, v)| v.clone());
    let text = |id: u16| match get(id) {
        Some(Val::Text(s)) if !s.is_empty() => Some(s),
        _ => None,
    };
    let num = |id: u16| match get(id) {
        Some(Val::Num(n)) => Some(n),
        _ => None,
    };
    let mut out = Vec::new();
    let camera = match (text(0x010F), text(0x0110)) {
        (Some(m), Some(md)) if md.starts_with(&m) => Some(md),
        (Some(m), Some(md)) => Some(format!("{m} {md}")),
        (a, b) => a.or(b),
    };
    if let Some(c) = camera {
        out.push(("Appareil", c));
    }
    if let Some(l) = text(0xA434) {
        out.push(("Objectif", l));
    }
    if let Some(d) = text(0x9003).or(text(0x0132)) {
        out.push(("Prise de vue", pretty_date(&d)));
    }
    if let Some(e) = num(0x829A).filter(|e| *e > 0.0) {
        out.push(("Vitesse", if e < 1.0 { format!("1/{} s", (1.0 / e).round()) } else { format!("{e} s") }));
    }
    if let Some(f) = num(0x829D).filter(|f| *f > 0.0) {
        out.push(("Ouverture", format!("f/{}", (f * 10.0).round() / 10.0)));
    }
    if let Some(i) = num(0x8827).filter(|i| *i > 0.0) {
        out.push(("ISO", format!("{i}")));
    }
    if let Some(f) = num(0x920A).filter(|f| *f > 0.0) {
        let eq = num(0xA405).filter(|v| *v > 0.0).map(|v| format!(" (éq. {v} mm)")).unwrap_or_default();
        out.push(("Focale", format!("{} mm{eq}", (f * 10.0).round() / 10.0)));
    }
    if let Some(b) = num(0x9204).filter(|b| *b != 0.0) {
        out.push(("Correction d'expo", format!("{:+.1} IL", b)));
    }
    if let Some(f) = num(0x9209) {
        out.push(("Flash", if f as u32 & 1 == 1 { "Déclenché".into() } else { "Non déclenché".into() }));
    }
    if let Some(a) = text(0x013B) {
        out.push(("Auteur", a));
    }
    if let Some(c) = text(0x8298) {
        out.push(("Copyright", c));
    }
    if let Some(s) = text(0x0131) {
        out.push(("Logiciel", s));
    }
    if get(0x8825).is_some() {
        out.push(("Localisation", "Coordonnées GPS présentes".into()));
    }
    out
}

fn pretty_date(d: &str) -> String {
    let p: Vec<&str> = d.split([':', ' ']).collect();
    if p.len() >= 5 { format!("{}/{}/{} {}:{}", p[2], p[1], p[0], p[3], p[4]) } else { d.to_string() }
}

pub fn build_exif(tags: &[(u16, String)]) -> Vec<u8> {
    let mut tags: Vec<&(u16, String)> = tags.iter().filter(|(_, v)| !v.is_empty()).collect();
    tags.sort_by_key(|t| t.0);
    let n = tags.len();
    let mut out = b"II*\0".to_vec();
    out.extend_from_slice(&8u32.to_le_bytes());
    out.extend_from_slice(&(n as u16).to_le_bytes());
    let mut data_off = 8 + 2 + n * 12 + 4;
    let mut data = Vec::new();
    for (tag, v) in tags {
        let mut bytes = v.as_bytes().to_vec();
        bytes.push(0);
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        if bytes.len() <= 4 {
            bytes.resize(4, 0);
            out.extend_from_slice(&bytes);
        } else {
            out.extend_from_slice(&(data_off as u32).to_le_bytes());
            data_off += bytes.len();
            data.extend_from_slice(&bytes);
        }
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&data);
    out
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ExportOpts {
    pub format: usize,
    pub quality: u8,
    pub size_mode: usize,
    pub size: u32,
    pub size_h: u32,
    pub no_enlarge: bool,
    pub template: String,
    pub custom: String,
    pub start: u32,
    pub suffix: String,
    pub same_folder: bool,
    pub folder: Option<PathBuf>,
    pub subfolder: String,
    pub collision: usize,
    pub sharpen_for: usize,
    pub sharpen_amount: usize,
    pub metadata: usize,
    pub creator: String,
    pub copyright: String,
    pub watermark: bool,
    pub watermark_text: String,
    pub watermark_size: f32,
    pub watermark_opacity: f32,
    pub watermark_pos: usize,
    pub watermark_dark: bool,
    pub open_after: bool,
}

impl Default for ExportOpts {
    fn default() -> Self {
        ExportOpts {
            format: 0,
            quality: 90,
            size_mode: 0,
            size: 0,
            size_h: 0,
            no_enlarge: true,
            template: "{nom}".into(),
            custom: String::new(),
            start: 1,
            suffix: String::new(),
            same_folder: false,
            folder: None,
            subfolder: String::new(),
            collision: 0,
            sharpen_for: 0,
            sharpen_amount: 1,
            metadata: 1,
            creator: String::new(),
            copyright: String::new(),
            watermark: false,
            watermark_text: String::new(),
            watermark_size: 4.0,
            watermark_opacity: 70.0,
            watermark_pos: 3,
            watermark_dark: false,
            open_after: false,
        }
    }
}

pub const FORMATS: [&str; 4] = ["JPEG", "PNG", "TIFF", "WebP"];
pub const SIZE_MODES: [&str; 5] = ["Grand côté", "Petit côté", "Largeur", "Hauteur", "Cadre L × H"];
pub const SIZE_PRESETS: [(&str, usize, u32, u32); 9] = [
    ("Taille d'origine", 0, 0, 0),
    ("4K · 3840 px", 0, 3840, 0),
    ("Web · 2048 px", 0, 2048, 0),
    ("Écran HD · 1920 px", 0, 1920, 0),
    ("Instagram carré · 1080", 4, 1080, 1080),
    ("Instagram portrait · 1080 × 1350", 4, 1080, 1350),
    ("Story · 1080 × 1920", 4, 1080, 1920),
    ("Courriel · 1200 px", 0, 1200, 0),
    ("Miniature · 640 px", 0, 640, 0),
];
pub const TEMPLATES: [(&str, &str); 7] = [
    ("Nom d'origine", "{nom}"),
    ("Nom + suffixe", "{nom}{suffixe}"),
    ("Texte + numéro", "{texte}-{n3}"),
    ("Date + nom", "{date}-{nom}"),
    ("Titre + numéro", "{titre}-{n}"),
    ("Nom + note", "{nom}-{note}étoiles"),
    ("Personnalisé", ""),
];
pub const SHARPEN_FOR: [&str; 4] = ["Aucune", "Écran", "Papier mat", "Papier brillant"];
pub const SHARPEN_AMOUNT: [&str; 3] = ["Faible", "Standard", "Élevée"];
pub const METADATA: [&str; 4] = ["Toutes", "Toutes sauf la localisation", "Copyright uniquement", "Aucune"];
pub const COLLISION: [&str; 3] = ["Nom unique", "Écraser", "Ignorer"];
pub const POSITIONS: [&str; 5] = ["Haut gauche", "Haut droite", "Centre", "Bas droite", "Bas gauche"];

pub struct Job {
    pub path: PathBuf,
    pub name: String,
    pub settings: Settings,
    pub title: String,
    pub rating: u8,
}

pub struct Mark {
    pub w: u32,
    pub h: u32,
    pub alpha: Vec<u8>,
}

pub fn file_name(template: &str, o: &ExportOpts, job: &Job, n: u32) -> String {
    let date = std::fs::metadata(&job.path).and_then(|m| m.modified()).ok().map(|t| {
        let secs = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
        let days = secs.div_euclid(86400);
        let (y, m, d) = civil(days);
        format!("{y:04}-{m:02}-{d:02}")
    });
    let t = if template.is_empty() { "{nom}" } else { template };
    let s = t
        .replace("{nom}", &job.name)
        .replace("{suffixe}", &o.suffix)
        .replace("{texte}", if o.custom.is_empty() { "photo" } else { &o.custom })
        .replace("{n3}", &format!("{n:03}"))
        .replace("{n}", &n.to_string())
        .replace("{date}", &date.unwrap_or_default())
        .replace("{titre}", if job.title.is_empty() { &job.name } else { &job.title })
        .replace("{note}", &job.rating.to_string());
    s.chars().map(|c| if r#"\/:*?"<>|"#.contains(c) { '_' } else { c }).collect::<String>().trim().to_string()
}

fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn target_size(w: u32, h: u32, o: &ExportOpts) -> (u32, u32) {
    if o.size == 0 {
        return (w, h);
    }
    let (fw, fh) = (w as f32, h as f32);
    let r = match o.size_mode {
        1 => o.size as f32 / fw.min(fh),
        2 => o.size as f32 / fw,
        3 => o.size as f32 / fh,
        4 => (o.size as f32 / fw).min(o.size_h.max(1) as f32 / fh),
        _ => o.size as f32 / fw.max(fh),
    };
    let r = if o.no_enlarge { r.min(1.0) } else { r };
    (((fw * r).round() as u32).max(1), ((fh * r).round() as u32).max(1))
}

fn output_sharpen(img: &mut RgbImage, o: &ExportOpts) {
    if o.sharpen_for == 0 {
        return;
    }
    let radius = [0.0, 0.6, 1.0, 0.8][o.sharpen_for.min(3)];
    let amount = [0.35, 0.6, 0.9][o.sharpen_amount.min(2)];
    let b = imageops::fast_blur(img, radius);
    for (p, q) in img.pixels_mut().zip(b.pixels()) {
        for k in 0..3 {
            let v = p.0[k] as f32 + (p.0[k] as f32 - q.0[k] as f32) * amount;
            p.0[k] = v.clamp(0.0, 255.0).round() as u8;
        }
    }
}

fn watermark(img: &mut RgbImage, mark: &Mark, o: &ExportOpts) {
    if mark.w == 0 || mark.h == 0 {
        return;
    }
    let target_h = (img.height().min(img.width()) as f32 * o.watermark_size.clamp(1.0, 30.0) / 100.0).max(6.0);
    let scale = target_h / mark.h as f32;
    let (mw, mh) = ((mark.w as f32 * scale) as u32, (mark.h as f32 * scale) as u32);
    let margin = (img.width().min(img.height()) as f32 * 0.03) as u32;
    let (x0, y0) = match o.watermark_pos {
        0 => (margin, margin),
        1 => (img.width().saturating_sub(mw + margin), margin),
        2 => ((img.width().saturating_sub(mw)) / 2, (img.height().saturating_sub(mh)) / 2),
        4 => (margin, img.height().saturating_sub(mh + margin)),
        _ => (img.width().saturating_sub(mw + margin), img.height().saturating_sub(mh + margin)),
    };
    let ink = if o.watermark_dark { 0.0 } else { 255.0 };
    let op = o.watermark_opacity.clamp(0.0, 100.0) / 100.0;
    for y in 0..mh {
        for x in 0..mw {
            let (ix, iy) = (x0 + x, y0 + y);
            if ix >= img.width() || iy >= img.height() {
                continue;
            }
            let sx = ((x as f32 / scale) as u32).min(mark.w - 1);
            let sy = ((y as f32 / scale) as u32).min(mark.h - 1);
            let a = mark.alpha[(sy * mark.w + sx) as usize] as f32 / 255.0 * op;
            if a <= 0.0 {
                continue;
            }
            let p = img.get_pixel_mut(ix, iy);
            for k in 0..3 {
                p.0[k] = (p.0[k] as f32 + (ink - p.0[k] as f32) * a).round() as u8;
            }
        }
    }
}

fn metadata_for(job: &Job, o: &ExportOpts) -> Vec<u8> {
    let ours = |extra: Vec<(u16, String)>| {
        let mut t = vec![(0x0131u16, "Lumen · Ethone Studio".to_string()), (0x013B, o.creator.clone()), (0x8298, o.copyright.clone()), (0x010E, job.title.clone())];
        t.extend(extra);
        build_exif(&t)
    };
    match o.metadata {
        0 => exif_bytes(&job.path).unwrap_or_else(|| ours(Vec::new())),
        1 => {
            let ex = read_exif(&job.path);
            let find = |k: &str| ex.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone()).unwrap_or_default();
            ours(vec![(0x0110, find("Appareil")), (0x0132, find("Prise de vue"))])
        }
        2 => build_exif(&[(0x013B, o.creator.clone()), (0x8298, o.copyright.clone())]),
        _ => Vec::new(),
    }
}

pub fn export_one(job: &Job, o: &ExportOpts, dir: &Path, n: u32, mark: Option<&Mark>) -> Result<Option<PathBuf>, String> {
    let ext = ["jpg", "png", "tif", "webp"][o.format.min(3)];
    let template = TEMPLATES.iter().find(|t| t.1 == o.template).map(|t| t.1).unwrap_or(&o.template);
    let base = file_name(template, o, job, n);
    let base = if base.is_empty() { job.name.clone() } else { base };
    let mut target = dir.join(format!("{base}.{ext}"));
    if target.exists() {
        match o.collision {
            1 => {}
            2 => return Ok(None),
            _ => {
                let mut k = 2;
                while target.exists() {
                    target = dir.join(format!("{base}-{k}.{ext}"));
                    k += 1;
                }
            }
        }
    }
    let img = develop::load(&job.path, 0)?;
    let mut out = develop::process(&img, &job.settings);
    let (tw, th) = target_size(out.width(), out.height(), o);
    if (tw, th) != out.dimensions() {
        out = imageops::resize(&out, tw, th, imageops::FilterType::Lanczos3);
    }
    output_sharpen(&mut out, o);
    if o.watermark
        && let Some(m) = mark
    {
        watermark(&mut out, m, o);
    }
    let exif = metadata_for(job, o);
    let file = std::fs::File::create(&target).map_err(|e| e.to_string())?;
    let w = std::io::BufWriter::new(file);
    use image::ImageEncoder;
    let (pw, ph) = out.dimensions();
    let r = match o.format {
        0 => {
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(w, o.quality);
            let _ = enc.set_exif_metadata(exif);
            enc.write_image(out.as_raw(), pw, ph, image::ExtendedColorType::Rgb8)
        }
        1 => {
            let mut enc = image::codecs::png::PngEncoder::new(w);
            let _ = enc.set_exif_metadata(exif);
            enc.write_image(out.as_raw(), pw, ph, image::ExtendedColorType::Rgb8)
        }
        2 => image::codecs::tiff::TiffEncoder::new(w).write_image(out.as_raw(), pw, ph, image::ExtendedColorType::Rgb8),
        _ => {
            let mut enc = image::codecs::webp::WebPEncoder::new_lossless(w);
            let _ = enc.set_exif_metadata(exif);
            enc.write_image(out.as_raw(), pw, ph, image::ExtendedColorType::Rgb8)
        }
    };
    r.map_err(|e| e.to_string())?;
    Ok(Some(target))
}

pub fn run_export(jobs: Vec<Job>, o: ExportOpts, dir: Option<PathBuf>, mark: Option<Mark>, done: Arc<AtomicUsize>, errors: Arc<Mutex<Vec<String>>>) {
    use rayon::prelude::*;
    let mut last_dir = None;
    let dirs: Vec<PathBuf> = jobs
        .iter()
        .map(|j| {
            let base = if o.same_folder || dir.is_none() { j.path.parent().map(|p| p.to_path_buf()).unwrap_or_default() } else { dir.clone().unwrap_or_default() };
            if o.subfolder.trim().is_empty() { base } else { base.join(o.subfolder.trim()) }
        })
        .collect();
    for d in &dirs {
        let _ = std::fs::create_dir_all(d);
        last_dir = Some(d.clone());
    }
    jobs.par_iter().enumerate().for_each(|(k, job)| {
        if let Err(e) = export_one(job, &o, &dirs[k], o.start + k as u32, mark.as_ref())
            && let Ok(mut list) = errors.lock()
        {
            list.push(format!("{} : {e}", job.path.display()));
        }
        done.fetch_add(1, Ordering::Relaxed);
    });
    if o.open_after
        && let Some(d) = last_dir
    {
        let _ = std::process::Command::new("explorer").arg(d).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_has_a_label_and_group() {
        let v = to_value(&Settings::default());
        for k in v.as_object().unwrap().keys() {
            assert!(FIELDS.iter().any(|f| f.0 == k), "champ sans libellé : {k}");
        }
        for f in FIELDS {
            assert!(f.2 < GROUPS.len());
        }
    }

    #[test]
    fn helpers_work() {
        let a = Settings::default();
        let b = Settings { exposure: 1.0, contrast: 40.0, mono: true, ..Default::default() };
        let m = blend(&a, &b, 0.5);
        assert!((m.exposure - 0.5).abs() < 1e-4 && (m.contrast - 20.0).abs() < 1e-3 && m.mono);
        assert_eq!(changed_label(&a, &Settings { exposure: 1.0, ..Default::default() }), "Exposition");
        let mut groups = vec![false; GROUPS.len()];
        groups[0] = true;
        let c = copy_groups(&a, &Settings { exposure: 1.0, temperature: 20.0, ..Default::default() }, &groups);
        assert!(c.exposure == 1.0 && c.temperature == 0.0);
        let exif = build_exif(&[(0x010F, "Canon".into()), (0x0110, "EOS R6".into()), (0x013B, "Moi".into())]);
        let dir = std::env::temp_dir().join("lumen_exif_test");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("t.tif");
        std::fs::write(&p, &exif).unwrap();
        let r = read_exif(&p);
        assert!(r.iter().any(|(k, v)| *k == "Appareil" && v == "Canon EOS R6"), "{:?}", r.iter().map(|x| &x.1).collect::<Vec<_>>());
        assert!(r.iter().any(|(k, v)| *k == "Auteur" && v == "Moi"));
        let o = ExportOpts { size: 1080, size_h: 1350, size_mode: 4, ..Default::default() };
        assert_eq!(target_size(4000, 3000, &o), (1080, 810));
        assert_eq!(target_size(800, 600, &ExportOpts { size: 2048, ..Default::default() }), (800, 600));
        let job = Job { path: p.clone(), name: "IMG_1".into(), settings: Settings::default(), title: String::new(), rating: 4 };
        assert_eq!(file_name("{texte}-{n3}", &ExportOpts { custom: "mariage".into(), ..Default::default() }, &job, 7), "mariage-007");
        assert_eq!(file_name("{nom}-{note}", &ExportOpts::default(), &job, 1), "IMG_1-4");
        assert_eq!(civil(19723), (2024, 1, 1));
        let src = RgbImage::from_pixel(30, 20, image::Rgb([100, 120, 140]));
        let img_path = dir.join("src.png");
        src.save(&img_path).unwrap();
        let job = Job { path: img_path, name: "src".into(), settings: Settings { exposure: 0.5, ..Default::default() }, title: "Titre".into(), rating: 0 };
        let opts = ExportOpts { format: 0, size: 15, metadata: 2, creator: "Moi".into(), watermark: true, ..Default::default() };
        let mark = Mark { w: 4, h: 2, alpha: vec![255; 8] };
        let out = export_one(&job, &opts, &dir, 1, Some(&mark)).unwrap().unwrap();
        let back = image::open(&out).unwrap();
        assert_eq!((back.width(), back.height()), (15, 10));
        assert!(read_exif(&out).iter().any(|(k, v)| *k == "Auteur" && v == "Moi"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
