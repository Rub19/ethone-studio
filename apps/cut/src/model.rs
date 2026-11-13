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
    #[serde(default)]
    pub label: u8,
}

pub const STILL_EXT: [&str; 6] = ["png", "jpg", "jpeg", "bmp", "webp", "gif"];

pub const GENERATORS: [(&str, &str); 6] = [
    ("gen:color:1e1e24", "Cache couleur"),
    ("gen:gradient:0a84ff:8c5cf6", "Dégradé"),
    ("gen:bars", "Mire de barres"),
    ("gen:checker", "Damier"),
    ("gen:noise", "Bruit"),
    ("gen:color:00b140", "Fond vert"),
];

impl Source {
    pub fn name(&self) -> String {
        if let Some(rest) = self.path.strip_prefix("gen:") {
            let kind = rest.split(':').next().unwrap_or("");
            let base = GENERATORS.iter().find(|g| g.0.strip_prefix("gen:").is_some_and(|p| p.split(':').next() == Some(kind))).map_or("Élément", |g| g.1);
            return match rest.split(':').nth(1) {
                Some(hex) if kind == "color" => format!("{base} #{}", hex.to_uppercase()),
                _ => base.to_string(),
            };
        }
        std::path::Path::new(&self.path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub fn is_generator(&self) -> bool {
        self.path.starts_with("gen:")
    }

    pub fn is_still(&self) -> bool {
        std::path::Path::new(&self.path).extension().is_some_and(|e| STILL_EXT.iter().any(|x| e.eq_ignore_ascii_case(x)))
    }

    pub fn kind(&self) -> u8 {
        if self.is_generator() || self.is_still() {
            2
        } else if self.has_video {
            0
        } else {
            1
        }
    }
}

fn one() -> f64 {
    1.0
}

fn one32() -> f32 {
    1.0
}

fn green() -> [u8; 3] {
    [0, 177, 64]
}

fn tol() -> f32 {
    30.0
}

fn soft() -> f32 {
    12.0
}

pub const LOOKS: [&str; 20] = [
    "Aucun",
    "Noir et blanc",
    "Sépia",
    "Vintage",
    "Cinéma",
    "Vif",
    "Teal & orange",
    "Froid",
    "Chaud",
    "Délavé",
    "Nuit américaine",
    "Sans blanchiment",
    "Rétro 70",
    "Noir contrasté",
    "Pastel",
    "Matrice",
    "Désaturé",
    "Coucher de soleil",
    "Infrarouge",
    "Polaroïd",
];

pub const TRANSITIONS: [&str; 26] = [
    "Aucune",
    "Fondu enchaîné",
    "Fondu au noir",
    "Fondu au blanc",
    "Volet vers la droite",
    "Volet vers la gauche",
    "Volet vers le bas",
    "Volet vers le haut",
    "Glissement depuis la droite",
    "Glissement depuis la gauche",
    "Glissement depuis le bas",
    "Glissement depuis le haut",
    "Poussée vers la gauche",
    "Poussée vers la droite",
    "Poussée vers le haut",
    "Poussée vers le bas",
    "Iris rond",
    "Iris carré",
    "Zoom croisé",
    "Flou croisé",
    "Volet horloge",
    "Volet diagonal",
    "Store vénitien",
    "Damier",
    "Portes",
    "Fondu additif",
];

pub const ALIGNS: [&str; 3] = ["Centrée sur la coupe", "Début à la coupe", "Fin à la coupe"];

pub const KPROPS: [&str; 10] = ["Échelle", "Position X", "Position Y", "Rotation", "Opacité", "Flou", "Luminosité", "Saturation", "Exposition", "Volume"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub prop: u8,
    pub t: f64,
    pub v: f32,
    #[serde(default)]
    pub ease: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mask {
    pub kind: u8,
    pub pts: Vec<[f32; 2]>,
    #[serde(default)]
    pub feather: f32,
    #[serde(default)]
    pub invert: bool,
}

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
    #[serde(default = "one32")]
    pub opacity: f32,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub pos_x: f32,
    #[serde(default)]
    pub pos_y: f32,
    #[serde(default)]
    pub crop: [f32; 4],
    #[serde(default)]
    pub tint: f32,
    #[serde(default)]
    pub vignette: f32,
    #[serde(default)]
    pub blur: f32,
    #[serde(default)]
    pub zoom_end: f32,
    #[serde(default)]
    pub hold: f64,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub pan: f32,
    #[serde(default)]
    pub label: u8,
    #[serde(default)]
    pub gap: f64,
    #[serde(default)]
    pub start: f64,
    #[serde(default)]
    pub reverse: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub trans: u8,
    #[serde(default = "one")]
    pub trans_dur: f64,
    #[serde(default)]
    pub trans_align: u8,
    #[serde(default)]
    pub exposure: f32,
    #[serde(default)]
    pub highlights: f32,
    #[serde(default)]
    pub shadows: f32,
    #[serde(default)]
    pub whites: f32,
    #[serde(default)]
    pub blacks: f32,
    #[serde(default)]
    pub vibrance: f32,
    #[serde(default)]
    pub hue: f32,
    #[serde(default)]
    pub wheels: [[f32; 2]; 3],
    #[serde(default)]
    pub curves: [[f32; 5]; 4],
    #[serde(default = "one32")]
    pub look_amt: f32,
    #[serde(default)]
    pub sharpen: f32,
    #[serde(default)]
    pub grain: f32,
    #[serde(default)]
    pub pixelate: f32,
    #[serde(default)]
    pub posterize: f32,
    #[serde(default)]
    pub invert: bool,
    #[serde(default)]
    pub flip_v: bool,
    #[serde(default)]
    pub key_on: bool,
    #[serde(default = "green")]
    pub key_color: [u8; 3],
    #[serde(default = "tol")]
    pub key_tol: f32,
    #[serde(default = "soft")]
    pub key_soft: f32,
    #[serde(default)]
    pub mask: Option<Mask>,
    #[serde(default)]
    pub keys: Vec<Keyframe>,
}

pub const LABELS: [(&str, [u8; 3]); 8] = [
    ("Violet", [140, 92, 246]),
    ("Bleu", [10, 132, 255]),
    ("Vert", [48, 180, 110]),
    ("Jaune", [230, 180, 20]),
    ("Orange", [255, 140, 40]),
    ("Rose", [255, 80, 140]),
    ("Cyan", [40, 190, 210]),
    ("Gris", [130, 130, 140]),
];

impl Default for Clip {
    fn default() -> Self {
        Clip {
            src: 0,
            inp: 0.0,
            out: 0.0,
            volume: 1.0,
            fade_in: 0.0,
            fade_out: 0.0,
            speed: 1.0,
            brightness: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            temperature: 0.0,
            look: 0,
            zoom: 1.0,
            flip: false,
            fade_white: false,
            opacity: 1.0,
            rotation: 0.0,
            pos_x: 0.0,
            pos_y: 0.0,
            crop: [0.0; 4],
            tint: 0.0,
            vignette: 0.0,
            blur: 0.0,
            zoom_end: 0.0,
            hold: 0.0,
            mute: false,
            pan: 0.0,
            label: 0,
            gap: 0.0,
            start: 0.0,
            reverse: false,
            disabled: false,
            name: String::new(),
            trans: 0,
            trans_dur: 1.0,
            trans_align: 0,
            exposure: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            whites: 0.0,
            blacks: 0.0,
            vibrance: 0.0,
            hue: 0.0,
            wheels: [[0.0; 2]; 3],
            curves: [[0.0; 5]; 4],
            look_amt: 1.0,
            sharpen: 0.0,
            grain: 0.0,
            pixelate: 0.0,
            posterize: 0.0,
            invert: false,
            flip_v: false,
            key_on: false,
            key_color: green(),
            key_tol: tol(),
            key_soft: soft(),
            mask: None,
            keys: Vec::new(),
        }
    }
}

impl Clip {
    pub fn len(&self) -> f64 {
        if self.hold > 0.0 { self.hold } else { (self.out - self.inp) / self.speed.max(0.05) }
    }

    pub fn src_at(&self, local: f64) -> f64 {
        if self.hold > 0.0 {
            self.inp
        } else if self.reverse {
            self.out - local * self.speed
        } else {
            self.inp + local * self.speed
        }
    }

    pub fn graded(&self) -> bool {
        let d = Clip::default();
        self.look != 0 || self.brightness != 0.0 || self.contrast != 0.0 || self.saturation != 0.0 || self.temperature != 0.0 || self.tint != 0.0 || self.exposure != 0.0 || self.highlights != 0.0 || self.shadows != 0.0 || self.whites != 0.0 || self.blacks != 0.0 || self.vibrance != 0.0 || self.hue != 0.0 || self.wheels != d.wheels || self.curves != d.curves
    }

    pub fn styled(&self) -> bool {
        self.blur > 0.0 || self.sharpen > 0.0 || self.grain > 0.0 || self.pixelate > 0.0 || self.posterize > 0.0 || self.invert || self.key_on || self.mask.is_some() || self.vignette != 0.0
    }

    pub fn reset_look(&mut self) {
        let d = Clip::default();
        self.reset_color();
        (self.vignette, self.blur, self.sharpen, self.grain, self.pixelate, self.posterize, self.invert, self.key_on, self.mask) = (d.vignette, d.blur, d.sharpen, d.grain, d.pixelate, d.posterize, d.invert, d.key_on, None);
        (self.zoom, self.zoom_end, self.rotation, self.pos_x, self.pos_y, self.crop, self.flip, self.flip_v, self.opacity) = (d.zoom, d.zoom_end, d.rotation, d.pos_x, d.pos_y, d.crop, d.flip, d.flip_v, d.opacity);
        self.keys.retain(|k| k.prop == 9);
    }

    pub fn reset_color(&mut self) {
        let d = Clip::default();
        (self.brightness, self.contrast, self.saturation, self.temperature, self.tint, self.look, self.look_amt) = (d.brightness, d.contrast, d.saturation, d.temperature, d.tint, d.look, d.look_amt);
        (self.exposure, self.highlights, self.shadows, self.whites, self.blacks, self.vibrance, self.hue, self.wheels, self.curves) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, d.wheels, d.curves);
    }

    pub fn prop(&self, k: u8) -> f32 {
        match k {
            0 => self.zoom,
            1 => self.pos_x,
            2 => self.pos_y,
            3 => self.rotation,
            4 => self.opacity,
            5 => self.blur,
            6 => self.brightness,
            7 => self.saturation,
            8 => self.exposure,
            _ => 20.0 * self.volume.max(1e-4).log10(),
        }
    }

    pub fn set_prop(&mut self, k: u8, v: f32) {
        match k {
            0 => self.zoom = v,
            1 => self.pos_x = v,
            2 => self.pos_y = v,
            3 => self.rotation = v,
            4 => self.opacity = v,
            5 => self.blur = v,
            6 => self.brightness = v,
            7 => self.saturation = v,
            8 => self.exposure = v,
            _ => self.volume = 10f32.powf(v / 20.0),
        }
    }

    pub fn has_keys(&self, k: u8) -> bool {
        self.keys.iter().any(|x| x.prop == k)
    }

    pub fn key_value(&self, prop: u8, local: f64) -> Option<f32> {
        let mut ks: Vec<&Keyframe> = self.keys.iter().filter(|k| k.prop == prop).collect();
        if ks.is_empty() {
            return None;
        }
        ks.sort_by(|a, b| a.t.total_cmp(&b.t));
        if local <= ks[0].t {
            return Some(ks[0].v);
        }
        for w in ks.windows(2) {
            if local <= w[1].t {
                let span = (w[1].t - w[0].t).max(1e-9);
                let mut u = ((local - w[0].t) / span) as f32;
                if w[0].ease {
                    u = u * u * (3.0 - 2.0 * u);
                }
                return Some(w[0].v + (w[1].v - w[0].v) * u);
            }
        }
        ks.last().map(|k| k.v)
    }

    pub fn set_key(&mut self, prop: u8, local: f64, v: f32) {
        if let Some(k) = self.keys.iter_mut().find(|k| k.prop == prop && (k.t - local).abs() < 0.02) {
            k.v = v;
        } else {
            self.keys.push(Keyframe { prop, t: local.max(0.0), v, ease: false });
        }
    }

    pub fn eval(&self, local: f64) -> Clip {
        let mut c = self.clone();
        for k in 0..9u8 {
            if let Some(v) = self.key_value(k, local) {
                c.set_prop(k, v);
            }
        }
        c
    }

    pub fn set_start(&mut self, dt: f64, max: f64) -> f64 {
        let before = self.len();
        if self.hold > 0.0 {
            self.hold = (self.hold - dt).max(0.1);
        } else {
            let (sp, d) = (self.speed.max(0.05), dt * self.speed.max(0.05));
            if self.reverse {
                self.out = (self.out - d).clamp(self.inp + 0.1 * sp, max.max(self.inp + 0.1 * sp));
            } else {
                self.inp = (self.inp + d).clamp(0.0, (self.out - 0.1 * sp).max(0.0));
            }
        }
        let applied = before - self.len();
        for k in &mut self.keys {
            k.t -= applied;
        }
        applied
    }

    pub fn set_end(&mut self, dt: f64, max: f64) -> f64 {
        let before = self.len();
        if self.hold > 0.0 {
            self.hold = (self.hold + dt).max(0.1);
        } else {
            let (sp, d) = (self.speed.max(0.05), dt * self.speed.max(0.05));
            if self.reverse {
                self.inp = (self.inp - d).clamp(0.0, (self.out - 0.1 * sp).max(0.0));
            } else {
                self.out = (self.out + d).clamp(self.inp + 0.1 * sp, max.max(self.inp + 0.1 * sp));
            }
        }
        self.len() - before
    }

    pub fn cut(&self, local: f64) -> Option<(Clip, Clip)> {
        let len = self.len();
        if local < 0.05 || len - local < 0.05 {
            return None;
        }
        let (mut a, mut b) = (self.clone(), self.clone());
        if self.hold > 0.0 {
            a.hold = local;
            b.hold = len - local;
        } else {
            let st = self.src_at(local);
            if self.reverse {
                a.inp = st;
                b.out = st;
            } else {
                a.out = st;
                b.inp = st;
            }
        }
        a.fade_out = 0.0;
        (b.fade_in, b.gap, b.trans) = (0.0, 0.0, 0);
        b.start = self.start + local;
        a.keys.retain(|k| k.t <= local);
        b.keys.retain(|k| k.t >= local);
        for k in &mut b.keys {
            k.t -= local;
        }
        for p in 0..10u8 {
            if let Some(v) = self.key_value(p, local) {
                a.keys.push(Keyframe { prop: p, t: local, v, ease: false });
                b.keys.push(Keyframe { prop: p, t: 0.0, v, ease: false });
            }
        }
        Some((a, b))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
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
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub pan: f32,
    #[serde(default)]
    pub label: u8,
}

impl Music {
    pub fn end(&self) -> f64 {
        self.start + self.out - self.inp
    }
}

pub const ANIMS: [&str; 10] = ["Aucune", "Fondu", "Glisse vers le haut", "Machine à écrire", "Zoom", "Depuis la gauche", "Depuis la droite", "Descente", "Lettres espacées", "Pop"];
pub const FONTS: [(&str, &str); 14] = [
    ("Segoe UI", "segoeuib.ttf"),
    ("Arial", "arialbd.ttf"),
    ("Impact", "impact.ttf"),
    ("Georgia", "georgiab.ttf"),
    ("Verdana", "verdanab.ttf"),
    ("Consolas", "consolab.ttf"),
    ("Times New Roman", "timesbd.ttf"),
    ("Trebuchet", "trebucbd.ttf"),
    ("Tahoma", "tahomabd.ttf"),
    ("Calibri", "calibrib.ttf"),
    ("Segoe UI Light", "segoeuil.ttf"),
    ("Comic Sans", "comicbd.ttf"),
    ("Courier New", "courbd.ttf"),
    ("Segoe Script", "segoesc.ttf"),
];

pub const ROLLS: [&str; 3] = ["Fixe", "Générique défilant", "Défilement horizontal"];

fn band() -> [u8; 4] {
    [0, 0, 0, 166]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Title {
    pub text: String,
    pub start: f64,
    pub dur: f64,
    pub size: f32,
    pub place: u8,
    pub color: [u8; 4],
    pub band: bool,
    #[serde(default)]
    pub anim: u8,
    #[serde(default)]
    pub shadow: bool,
    #[serde(default)]
    pub font: u8,
    #[serde(default)]
    pub dx: f32,
    #[serde(default)]
    pub dy: f32,
    #[serde(default)]
    pub outline: f32,
    #[serde(default = "black")]
    pub outline_color: [u8; 4],
    #[serde(default = "band")]
    pub band_color: [u8; 4],
    #[serde(default)]
    pub align: u8,
    #[serde(default)]
    pub vertical: bool,
    #[serde(default)]
    pub roll: u8,
    #[serde(default)]
    pub spacing: f32,
    #[serde(default)]
    pub leading: f32,
}

impl Default for Title {
    fn default() -> Self {
        Title { text: "Ton titre".into(), start: 0.0, dur: 3.0, size: 0.08, place: 1, color: [255; 4], band: false, anim: 1, shadow: true, font: 0, dx: 0.0, dy: 0.0, outline: 0.0, outline_color: black(), band_color: band(), align: 0, vertical: false, roll: 0, spacing: 0.0, leading: 0.0 }
    }
}

fn black() -> [u8; 4] {
    [0, 0, 0, 255]
}

pub const MARKER_COLORS: [(&str, [u8; 3]); 8] = [
    ("Jaune", [255, 196, 0]),
    ("Vert", [48, 209, 88]),
    ("Rouge", [255, 69, 58]),
    ("Violet", [175, 82, 222]),
    ("Orange", [255, 149, 0]),
    ("Bleu", [10, 132, 255]),
    ("Cyan", [90, 200, 250]),
    ("Blanc", [235, 235, 240]),
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(from = "MarkerRepr")]
pub struct Marker {
    pub t: f64,
    pub name: String,
    pub note: String,
    pub color: u8,
    pub dur: f64,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MarkerRepr {
    Time(f64),
    Full {
        t: f64,
        #[serde(default)]
        name: String,
        #[serde(default)]
        note: String,
        #[serde(default)]
        color: u8,
        #[serde(default)]
        dur: f64,
    },
}

impl From<MarkerRepr> for Marker {
    fn from(r: MarkerRepr) -> Self {
        match r {
            MarkerRepr::Time(t) => Marker { t, ..Default::default() },
            MarkerRepr::Full { t, name, note, color, dur } => Marker { t, name, note, color, dur },
        }
    }
}

pub struct Blend {
    pub a: usize,
    pub a_local: f64,
    pub b: usize,
    pub b_local: f64,
    pub p: f32,
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
    pub markers: Vec<Marker>,
    #[serde(default = "black")]
    pub background: [u8; 4],
    #[serde(default)]
    pub overlays: Vec<Clip>,
    #[serde(default)]
    pub mute_v2: bool,
    #[serde(default)]
    pub hide: [bool; 3],
    #[serde(default)]
    pub lock: [bool; 4],
    #[serde(default)]
    pub solo: u8,
    #[serde(default)]
    pub master_db: f32,
    #[serde(default)]
    pub duck_db: f32,
}

impl Project {
    pub fn new() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 30,
            sources: Vec::new(),
            clips: Vec::new(),
            music: Vec::new(),
            mute_v1: false,
            mute_a2: false,
            titles: Vec::new(),
            markers: Vec::new(),
            background: black(),
            overlays: Vec::new(),
            mute_v2: false,
            hide: [false; 3],
            lock: [false; 4],
            solo: 0,
            master_db: 0.0,
            duck_db: 0.0,
        }
    }

    pub fn starts(&self) -> Vec<f64> {
        let mut t = 0.0;
        self.clips
            .iter()
            .map(|c| {
                let s = t + c.gap.max(0.0);
                t = s + c.len();
                s
            })
            .collect()
    }

    pub fn v1_end(&self) -> f64 {
        self.clips.iter().map(|c| c.gap.max(0.0) + c.len()).sum()
    }

    pub fn duration(&self) -> f64 {
        let m = self.music.iter().map(Music::end).fold(0.0, f64::max);
        let t = self.titles.iter().map(|t| t.start + t.dur).fold(0.0, f64::max);
        let o = self.overlays.iter().map(|o| o.start + o.len()).fold(0.0, f64::max);
        self.v1_end().max(m).max(t).max(o)
    }

    #[cfg(test)]
    pub fn video_duration(&self) -> f64 {
        self.v1_end()
    }

    #[cfg(test)]
    pub fn clip_at(&self, t: f64) -> Option<(usize, f64)> {
        self.local_at(t).map(|(i, l)| (i, self.clips[i].src_at(l)))
    }

    pub fn local_at(&self, t: f64) -> Option<(usize, f64)> {
        let mut start = 0.0;
        for (i, c) in self.clips.iter().enumerate() {
            start += c.gap.max(0.0);
            if t >= start && t < start + c.len() {
                return Some((i, t - start));
            }
            start += c.len();
        }
        None
    }

    #[cfg(test)]
    pub fn fade_at(&self, t: f64) -> f32 {
        let Some((i, local)) = self.local_at(t) else { return 1.0 };
        fade(&self.clips[i], local)
    }

    pub fn transition_at(&self, t: f64) -> Option<Blend> {
        let starts = self.starts();
        for i in 1..self.clips.len() {
            let (a, b) = (&self.clips[i - 1], &self.clips[i]);
            if b.trans == 0 || b.gap > 1e-6 {
                continue;
            }
            let d = b.trans_dur.clamp(0.05, 10.0).min(a.len() + b.len());
            let (pre, post) = match b.trans_align {
                1 => (0.0, d),
                2 => (d, 0.0),
                _ => (d / 2.0, d / 2.0),
            };
            let s = starts[i];
            if t >= s - pre && t < s + post {
                return Some(Blend { a: i - 1, a_local: t - starts[i - 1], b: i, b_local: t - s, p: ((t - (s - pre)) / d) as f32 });
            }
        }
        None
    }

    pub fn split(&mut self, t: f64) -> bool {
        let Some((i, local)) = self.local_at(t) else { return false };
        let Some((a, b)) = self.clips[i].cut(local) else { return false };
        self.clips[i] = a;
        self.clips.insert(i + 1, b);
        true
    }

    pub fn split_all(&mut self, t: f64) -> bool {
        let mut any = self.split(t);
        let mut music = Vec::new();
        for m in std::mem::take(&mut self.music) {
            if t > m.start + 0.05 && t < m.end() - 0.05 {
                let at = m.inp + (t - m.start);
                music.push(Music { out: at, fade_out: 0.0, ..m.clone() });
                music.push(Music { start: t, inp: at, fade_in: 0.0, ..m });
                any = true;
            } else {
                music.push(m);
            }
        }
        self.music = music;
        let mut titles = Vec::new();
        for x in std::mem::take(&mut self.titles) {
            if t > x.start + 0.05 && t < x.start + x.dur - 0.05 {
                titles.push(Title { dur: t - x.start, ..x.clone() });
                titles.push(Title { start: t, dur: x.start + x.dur - t, ..x });
                any = true;
            } else {
                titles.push(x);
            }
        }
        self.titles = titles;
        let mut over = Vec::new();
        for o in std::mem::take(&mut self.overlays) {
            match o.cut(t - o.start) {
                Some((a, b)) => {
                    over.push(a);
                    over.push(b);
                    any = true;
                }
                None => over.push(o),
            }
        }
        self.overlays = over;
        any
    }

    pub fn titles_at(&self, t: f64) -> Vec<&Title> {
        self.titles.iter().filter(|x| t >= x.start && t < x.start + x.dur).collect()
    }

    pub fn overlays_at(&self, t: f64) -> Vec<(usize, f64)> {
        self.overlays.iter().enumerate().filter(|(_, o)| t >= o.start && t < o.start + o.len()).map(|(i, o)| (i, t - o.start)).collect()
    }

    pub fn trim_to(&mut self, t: f64, start: bool) -> bool {
        let Some((i, local)) = self.local_at(t) else { return false };
        let max = self.sources.get(self.clips[i].src).map_or(f64::MAX, |s| s.duration);
        let c = &mut self.clips[i];
        if start && c.len() - local > 0.05 {
            c.set_start(local, max);
        } else if !start && local > 0.05 {
            let len = c.len();
            c.set_end(local - len, max);
        } else {
            return false;
        }
        true
    }

    pub fn lift(&mut self, i: usize) {
        if i >= self.clips.len() {
            return;
        }
        let c = self.clips.remove(i);
        if let Some(n) = self.clips.get_mut(i) {
            n.gap += c.gap + c.len();
        }
    }

    pub fn ripple_delete(&mut self, i: usize) {
        if i >= self.clips.len() {
            return;
        }
        let c = self.clips.remove(i);
        if let Some(n) = self.clips.get_mut(i) {
            n.gap += c.gap;
        }
    }

    pub fn close_gaps(&mut self) {
        for c in &mut self.clips {
            c.gap = 0.0;
        }
    }

    pub fn shift_after(&mut self, at: f64, d: f64) {
        for m in &mut self.music {
            if m.start >= at - 1e-6 {
                m.start = (m.start + d).max(0.0);
            }
        }
        for x in &mut self.titles {
            if x.start >= at - 1e-6 {
                x.start = (x.start + d).max(0.0);
            }
        }
        for o in &mut self.overlays {
            if o.start >= at - 1e-6 {
                o.start = (o.start + d).max(0.0);
            }
        }
        for m in &mut self.markers {
            if m.t >= at - 1e-6 {
                m.t = (m.t + d).max(0.0);
            }
        }
    }

    fn max_of(&self, i: usize) -> f64 {
        self.sources.get(self.clips[i].src).map_or(f64::MAX, |s| s.duration)
    }

    pub fn roll(&mut self, i: usize, d: f64) {
        if i + 1 >= self.clips.len() {
            return;
        }
        if self.clips[i + 1].gap > 1e-6 {
            let (ma, gap) = (self.max_of(i), self.clips[i + 1].gap);
            let applied = self.clips[i].set_end(d.min(gap), ma);
            self.clips[i + 1].gap -= applied;
            return;
        }
        let (ma, mb) = (self.max_of(i), self.max_of(i + 1));
        let (a0, b0) = (self.clips[i].clone(), self.clips[i + 1].clone());
        let da = self.clips[i].set_end(d, ma);
        let db = self.clips[i + 1].set_start(da, mb);
        if (db - da).abs() > 1e-9 {
            self.clips[i] = a0;
            self.clips[i].set_end(db, ma);
            self.clips[i + 1] = b0;
            self.clips[i + 1].set_start(db, mb);
        }
    }

    pub fn slide(&mut self, i: usize, d: f64) {
        if i >= self.clips.len() || d == 0.0 {
            return;
        }
        let mut lo = f64::NEG_INFINITY;
        let mut hi = f64::INFINITY;
        if self.clips[i].gap > 1e-6 || i == 0 {
            lo = -self.clips[i].gap;
        } else {
            let p = &self.clips[i - 1];
            lo = lo.max(-(p.len() - 0.1));
            if p.hold <= 0.0 {
                let room = if p.reverse { p.inp } else { self.max_of(i - 1) - p.out };
                hi = hi.min(room / p.speed.max(0.05));
            }
        }
        if let Some(n) = self.clips.get(i + 1) {
            if n.gap > 1e-6 {
                hi = hi.min(n.gap);
            } else {
                hi = hi.min(n.len() - 0.1);
                if n.hold <= 0.0 {
                    let room = if n.reverse { self.max_of(i + 1) - n.out } else { n.inp };
                    lo = lo.max(-room / n.speed.max(0.05));
                }
            }
        }
        if lo > hi {
            return;
        }
        let d = d.clamp(lo, hi);
        if self.clips[i].gap > 1e-6 || i == 0 {
            self.clips[i].gap += d;
        } else {
            let m = self.max_of(i - 1);
            self.clips[i - 1].set_end(d, m);
        }
        if i + 1 < self.clips.len() {
            if self.clips[i + 1].gap > 1e-6 {
                self.clips[i + 1].gap -= d;
            } else {
                let m = self.max_of(i + 1);
                self.clips[i + 1].set_start(d, m);
            }
        }
    }

    pub fn mix(&self, audio: &dyn Fn(usize) -> Option<std::sync::Arc<Vec<f32>>>, rate: u32) -> Vec<f32> {
        let total = (self.duration() * rate as f64).ceil() as usize;
        let mut out = vec![0f32; total * 2];
        let audible = |bit: u8| self.solo == 0 || self.solo & bit != 0;
        let block = (rate / 100).max(1) as usize;
        let mut duck = vec![1f32; total / block + 2];
        if self.duck_db < 0.0 {
            let g = 10f32.powf(self.duck_db / 20.0);
            for (c, s) in self.clips.iter().zip(self.starts()) {
                if c.mute || c.disabled || c.hold > 0.0 || self.mute_v1 || audio(c.src).is_none_or(|d| d.is_empty()) {
                    continue;
                }
                let (a, b) = ((s * rate as f64) as usize / block, (((s + c.len()) * rate as f64) as usize / block).min(duck.len()));
                for v in duck.iter_mut().take(b).skip(a) {
                    *v = g;
                }
            }
            let step = 1.0 / 30.0;
            for k in 1..duck.len() {
                duck[k] = duck[k].max(duck[k - 1] - step);
            }
            for k in (0..duck.len() - 1).rev() {
                duck[k] = duck[k].max(duck[k + 1] - step);
            }
        }
        let place = |out: &mut Vec<f32>, data: &[f32], at: f64, c: &Clip, ducked: bool| {
            let (gl, gr) = ((1.0 - c.pan).min(1.0), (1.0 + c.pan).min(1.0));
            let a = (c.inp * rate as f64) as usize;
            let b = ((c.out * rate as f64) as usize).min(data.len() / 2);
            let dst = (at.max(0.0) * rate as f64) as usize;
            let speed = c.speed.max(0.05);
            let n = (b.saturating_sub(a) as f64 / speed) as usize;
            let keyed = c.has_keys(9);
            let mut gain = c.volume;
            for k in 0..n {
                let d = dst + k;
                if d >= total {
                    break;
                }
                let tt = k as f64 / rate as f64;
                if keyed && k % block == 0 {
                    gain = c.key_value(9, tt).map_or(c.volume, |db| 10f32.powf(db / 20.0));
                }
                let mut g = gain;
                if c.fade_in > 0.0 {
                    g *= (tt / c.fade_in).min(1.0) as f32;
                }
                if c.fade_out > 0.0 {
                    g *= ((n - k) as f64 / rate as f64 / c.fade_out).min(1.0) as f32;
                }
                if ducked {
                    g *= duck[(d / block).min(duck.len() - 1)];
                }
                let off = (k as f64 * speed) as usize;
                let srcf = if c.reverse { b.saturating_sub(1 + off).max(a) } else { (a + off).min(b.saturating_sub(1)) };
                out[d * 2] += data[srcf * 2] * g * gl;
                out[d * 2 + 1] += data[srcf * 2 + 1] * g * gr;
            }
        };
        if !self.mute_v1 && audible(1) {
            for (c, s) in self.clips.iter().zip(self.starts()) {
                if c.mute || c.hold > 0.0 || c.disabled {
                    continue;
                }
                if let Some(data) = audio(c.src) {
                    place(&mut out, &data, s, c, false);
                }
            }
        }
        if !self.mute_v2 && audible(2) {
            for c in &self.overlays {
                if c.mute || c.hold > 0.0 || c.disabled {
                    continue;
                }
                if let Some(data) = audio(c.src) {
                    place(&mut out, &data, c.start, c, false);
                }
            }
        }
        if !self.mute_a2 && audible(4) {
            for m in &self.music {
                if m.mute {
                    continue;
                }
                if let Some(data) = audio(m.src) {
                    let c = Clip { inp: m.inp, out: m.out, volume: m.volume, fade_in: m.fade_in, fade_out: m.fade_out, pan: m.pan, ..Default::default() };
                    place(&mut out, &data, m.start, &c, true);
                }
            }
        }
        if self.master_db != 0.0 {
            let g = 10f32.powf(self.master_db / 20.0);
            for v in &mut out {
                *v *= g;
            }
        }
        out
    }
}

pub fn fade(c: &Clip, local: f64) -> f32 {
    let a = if c.fade_in > 0.0 { (local / c.fade_in).min(1.0) } else { 1.0 };
    let b = if c.fade_out > 0.0 { ((c.len() - local) / c.fade_out).min(1.0) } else { 1.0 };
    (a.min(b)).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn src(d: f64) -> Source {
        Source { path: "a.mp4".into(), duration: d, width: 1920, height: 1080, fps: 30.0, has_video: true, has_audio: true, label: 0 }
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
        p.music.push(Music { src: 0, start: 6.0, inp: 0.0, out: 2.0, volume: 0.25, ..Default::default() });
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
        p.mute_a2 = false;
        p.solo = 4;
        let solo = p.mix(&|_| Some(data.clone()), rate);
        assert!(solo[300 * 2].abs() < 1e-6 && (solo[750 * 2] - 0.25).abs() < 1e-6);
        p.solo = 0;
        p.duck_db = -12.0;
        let ducked = p.mix(&|_| Some(data.clone()), rate);
        assert!(ducked[650 * 2] < 0.75 && ducked[650 * 2] > 0.5);

        let mut q = Project::new();
        q.sources.push(src(10.0));
        q.clips.push(Clip { src: 0, inp: 0.0, out: 4.0, speed: 2.0, ..Default::default() });
        assert_eq!(q.duration(), 2.0);
        assert_eq!(q.clip_at(1.0), Some((0, 2.0)));
        assert_eq!(q.mix(&|_| Some(data.clone()), rate).len(), 2 * rate as usize * 2);
        assert!(q.trim_to(0.5, true));
        assert_eq!(q.clips[0].inp, 1.0);
        q.titles.push(Title { text: "Hi".into(), start: 3.0, dur: 2.0, ..Default::default() });
        assert_eq!(q.duration(), 5.0);
        assert_eq!(q.titles_at(4.0).len(), 1);
        assert!(q.titles_at(5.5).is_empty());

        let mut h = Project::new();
        h.sources.push(src(10.0));
        h.clips.push(Clip { src: 0, inp: 3.0, out: 5.0, hold: 1.5, fade_in: 0.5, ..Default::default() });
        h.clips.push(Clip { src: 0, inp: 0.0, out: 1.0, mute: true, ..Default::default() });
        assert_eq!(h.duration(), 2.5);
        assert_eq!(h.clip_at(1.2), Some((0, 3.0)));
        assert!((h.fade_at(0.25) - 0.5).abs() < 1e-6);
        assert_eq!(h.local_at(2.0), Some((1, 0.5)));
        let silent = h.mix(&|_| Some(data.clone()), rate);
        assert!(silent.iter().all(|v| v.abs() < 1e-6));
    }

    #[test]
    fn gaps_edits_transitions_keys() {
        let mut p = Project::new();
        p.sources.push(src(20.0));
        for k in 0..3 {
            p.clips.push(Clip { src: 0, inp: 2.0 + k as f64 * 4.0, out: 4.0 + k as f64 * 4.0, ..Default::default() });
        }
        p.lift(1);
        assert_eq!(p.clips.len(), 2);
        assert_eq!(p.clips[1].gap, 2.0);
        assert_eq!(p.duration(), 6.0);
        assert_eq!(p.local_at(3.0), None);
        assert_eq!(p.starts(), vec![0.0, 4.0]);
        p.ripple_delete(0);
        assert_eq!(p.starts(), vec![2.0]);
        p.close_gaps();
        assert_eq!(p.duration(), 2.0);

        let mut r = Project::new();
        r.sources.push(src(20.0));
        r.clips.push(Clip { src: 0, inp: 2.0, out: 4.0, ..Default::default() });
        r.clips.push(Clip { src: 0, inp: 8.0, out: 10.0, ..Default::default() });
        r.clips.push(Clip { src: 0, inp: 12.0, out: 14.0, ..Default::default() });
        r.roll(0, 0.5);
        assert_eq!((r.clips[0].out, r.clips[1].inp), (4.5, 8.5));
        assert_eq!(r.duration(), 6.0);
        r.slide(1, -1.0);
        assert_eq!((r.clips[0].out, r.clips[2].inp), (3.5, 11.0));
        assert_eq!(r.starts()[1], 1.5);
        assert_eq!(r.duration(), 6.0);

        r.clips[1].trans = 1;
        r.clips[1].trans_dur = 1.0;
        let b = r.transition_at(1.25).unwrap();
        assert_eq!((b.a, b.b), (0, 1));
        assert!((b.p - 0.25).abs() < 1e-5);
        assert!(r.transition_at(0.5).is_none());

        let mut c = Clip { src: 0, inp: 0.0, out: 4.0, ..Default::default() };
        c.set_key(0, 0.0, 1.0);
        c.set_key(0, 2.0, 2.0);
        assert_eq!(c.eval(1.0).zoom, 1.5);
        assert_eq!(c.eval(3.0).zoom, 2.0);
        let (a, b) = c.cut(1.0).unwrap();
        assert_eq!(b.eval(0.0).zoom, 1.5);
        assert_eq!(a.eval(1.0).zoom, 1.5);
        let rev = Clip { src: 0, inp: 1.0, out: 3.0, reverse: true, ..Default::default() };
        assert_eq!(rev.src_at(0.5), 2.5);

        let mut s = Project::new();
        s.sources.push(src(20.0));
        s.clips.push(Clip { src: 0, inp: 0.0, out: 4.0, ..Default::default() });
        s.music.push(Music { src: 0, start: 1.0, inp: 0.0, out: 4.0, volume: 1.0, ..Default::default() });
        s.titles.push(Title { start: 0.0, dur: 4.0, ..Default::default() });
        assert!(s.split_all(2.0));
        assert_eq!((s.clips.len(), s.music.len(), s.titles.len()), (2, 2, 2));
        assert_eq!(s.music[1].inp, 1.0);

        let m: Vec<Marker> = serde_json::from_str("[1.5, {\"t\": 2.0, \"name\": \"A\"}]").unwrap();
        assert_eq!((m[0].t, m[1].name.as_str()), (1.5, "A"));
    }
}
