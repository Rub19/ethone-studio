use crate::doc::{hsl_to_rgb, rgb_to_hsl};
use image::{Rgba, RgbaImage, imageops};
use rayon::prelude::*;
use std::f32::consts::{PI, TAU};

pub struct Param {
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub suffix: &'static str,
    pub choices: &'static [&'static str],
}

const fn p(label: &'static str, min: f32, max: f32, default: f32, suffix: &'static str) -> Param {
    Param { label, min, max, default, suffix, choices: &[] }
}

const fn c(label: &'static str, choices: &'static [&'static str], default: f32) -> Param {
    Param { label, min: 0.0, max: choices.len() as f32 - 1.0, default, suffix: "", choices }
}

const fn on(label: &'static str, default: f32) -> Param {
    Param { label, min: 0.0, max: 1.0, default, suffix: "", choices: &[] }
}

impl Param {
    pub fn is_toggle(&self) -> bool {
        self.choices.is_empty() && self.min == 0.0 && self.max == 1.0 && self.suffix.is_empty()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    BrightnessContrast,
    HueSat,
    Levels,
    Curves,
    Exposure,
    Vibrance,
    Temperature,
    ColorBalance,
    BlackWhiteMix,
    PhotoFilter,
    ChannelMixer,
    GradientMap,
    SelectiveColor,
    ReplaceColor,
    ShadowsHighlights,
    Threshold,
    Posterize,
    BlackWhite,
    Invert,
    Sepia,
    AutoTone,
    AutoContrast,
    AutoColor,
    Equalize,
    Solarize,
    Blur,
    BoxBlur,
    SurfaceBlur,
    MotionBlur,
    RadialBlur,
    SpinBlur,
    TiltShift,
    Average,
    SharpenMore,
    Unsharp,
    Noise,
    Denoise,
    Median,
    DustScratches,
    Pixelate,
    Crystallize,
    ColorHalftone,
    Mezzotint,
    Emboss,
    FindEdges,
    GlowingEdges,
    TraceContour,
    Diffuse,
    Wind,
    Tiles,
    Vignette,
    Glow,
    Chromatic,
    OilPaint,
    Sketch,
    Cartoon,
    Cutout,
    FilmGrain,
    Wave,
    Ripple,
    Twirl,
    Pinch,
    Spherize,
    Polar,
    Shear,
    Halftone,
    Lighting,
    LensFlare,
    Clouds,
    HighPass,
    Maximum,
    Minimum,
    Offset,
}

pub const ADJUSTMENTS: &[Kind] = &[
    Kind::BrightnessContrast,
    Kind::Levels,
    Kind::Curves,
    Kind::Exposure,
    Kind::ShadowsHighlights,
    Kind::Vibrance,
    Kind::HueSat,
    Kind::ColorBalance,
    Kind::BlackWhiteMix,
    Kind::PhotoFilter,
    Kind::ChannelMixer,
    Kind::GradientMap,
    Kind::SelectiveColor,
    Kind::ReplaceColor,
    Kind::Temperature,
    Kind::Threshold,
    Kind::Posterize,
];
pub const QUICK: &[Kind] = &[Kind::AutoTone, Kind::AutoContrast, Kind::AutoColor, Kind::Equalize, Kind::BlackWhite, Kind::Sepia, Kind::Invert, Kind::Solarize];
pub const FILTERS: &[(&str, &[Kind])] = &[
    ("Flou", &[Kind::Blur, Kind::BoxBlur, Kind::SurfaceBlur, Kind::MotionBlur, Kind::RadialBlur, Kind::SpinBlur, Kind::TiltShift, Kind::Average]),
    ("Netteté", &[Kind::SharpenMore, Kind::Unsharp]),
    ("Bruit", &[Kind::Noise, Kind::Denoise, Kind::Median, Kind::DustScratches]),
    ("Pixellisation", &[Kind::Pixelate, Kind::Crystallize, Kind::ColorHalftone, Kind::Mezzotint]),
    ("Esthétiques", &[Kind::Emboss, Kind::FindEdges, Kind::GlowingEdges, Kind::TraceContour, Kind::Diffuse, Kind::Wind, Kind::Tiles, Kind::Glow, Kind::Chromatic, Kind::Halftone]),
    ("Artistiques", &[Kind::OilPaint, Kind::Sketch, Kind::Cartoon, Kind::Cutout, Kind::FilmGrain]),
    ("Déformation", &[Kind::Wave, Kind::Ripple, Kind::Twirl, Kind::Pinch, Kind::Spherize, Kind::Polar, Kind::Shear]),
    ("Rendu", &[Kind::Vignette, Kind::Lighting, Kind::LensFlare, Kind::Clouds]),
    ("Autres", &[Kind::HighPass, Kind::Maximum, Kind::Minimum, Kind::Offset]),
];

const PHOTO_FILTERS: &[(&str, [f32; 3])] = &[
    ("Chaud (85)", [0.925, 0.541, 0.0]),
    ("Chaud (LBA)", [0.98, 0.541, 0.0]),
    ("Froid (80)", [0.0, 0.427, 1.0]),
    ("Froid (LBB)", [0.0, 0.365, 1.0]),
    ("Rouge", [0.918, 0.102, 0.102]),
    ("Orange", [0.953, 0.518, 0.09]),
    ("Jaune", [0.976, 0.89, 0.11]),
    ("Vert", [0.098, 0.788, 0.098]),
    ("Cyan", [0.114, 0.796, 0.918]),
    ("Bleu", [0.114, 0.208, 0.918]),
    ("Violet", [0.608, 0.114, 0.918]),
    ("Magenta", [0.89, 0.094, 0.89]),
    ("Sépia", [0.675, 0.478, 0.2]),
    ("Immersion", [0.0, 0.757, 0.694]),
];
const PHOTO_NAMES: &[&str] = &["Chaud (85)", "Chaud (LBA)", "Froid (80)", "Froid (LBB)", "Rouge", "Orange", "Jaune", "Vert", "Cyan", "Bleu", "Violet", "Magenta", "Sépia", "Immersion"];

pub const GRADIENT_MAPS: &[(&str, &[[u8; 3]])] = &[
    ("Noir → blanc", &[[0, 0, 0], [255, 255, 255]]),
    ("Sépia", &[[40, 22, 10], [170, 120, 70], [255, 240, 210]]),
    ("Bleu → orange", &[[18, 32, 90], [120, 120, 130], [255, 150, 40]]),
    ("Violet → jaune", &[[60, 10, 90], [220, 60, 110], [255, 230, 80]]),
    ("Cuivre", &[[20, 8, 4], [180, 90, 50], [255, 220, 180]]),
    ("Infrarouge", &[[0, 0, 0], [140, 0, 160], [255, 80, 0], [255, 255, 200]]),
    ("Spectre", &[[255, 0, 0], [255, 255, 0], [0, 255, 0], [0, 255, 255], [0, 0, 255], [255, 0, 255]]),
    ("Nuit", &[[4, 6, 30], [20, 80, 160], [160, 230, 255]]),
    ("Coucher de soleil", &[[30, 10, 60], [200, 40, 60], [255, 170, 60], [255, 245, 200]]),
];
const GRADIENT_NAMES: &[&str] = &["Noir → blanc", "Sépia", "Bleu → orange", "Violet → jaune", "Cuivre", "Infrarouge", "Spectre", "Nuit", "Coucher de soleil"];
const SELECTIVE: &[&str] = &["Rouges", "Jaunes", "Verts", "Cyans", "Bleus", "Magentas", "Blancs", "Neutres", "Noirs"];

impl Kind {
    pub fn title(self) -> &'static str {
        match self {
            Kind::BrightnessContrast => "Luminosité et contraste",
            Kind::HueSat => "Teinte et saturation",
            Kind::Levels => "Niveaux",
            Kind::Curves => "Courbes",
            Kind::Exposure => "Exposition",
            Kind::Vibrance => "Vibrance",
            Kind::Temperature => "Température et teinte",
            Kind::ColorBalance => "Balance des couleurs",
            Kind::BlackWhiteMix => "Noir et blanc avancé",
            Kind::PhotoFilter => "Filtre photo",
            Kind::ChannelMixer => "Mélangeur de couches",
            Kind::GradientMap => "Courbe de transfert de dégradé",
            Kind::SelectiveColor => "Correction sélective",
            Kind::ReplaceColor => "Remplacement de couleur",
            Kind::ShadowsHighlights => "Tons foncés et tons clairs",
            Kind::Threshold => "Seuil",
            Kind::Posterize => "Postérisation",
            Kind::BlackWhite => "Noir et blanc",
            Kind::Invert => "Négatif",
            Kind::Sepia => "Sépia",
            Kind::AutoTone => "Ton automatique",
            Kind::AutoContrast => "Contraste automatique",
            Kind::AutoColor => "Couleur automatique",
            Kind::Equalize => "Égaliser",
            Kind::Solarize => "Solarisation",
            Kind::Blur => "Flou gaussien",
            Kind::BoxBlur => "Flou rectangulaire",
            Kind::SurfaceBlur => "Flou de surface",
            Kind::MotionBlur => "Flou de mouvement",
            Kind::RadialBlur => "Flou radial",
            Kind::SpinBlur => "Flou de rotation",
            Kind::TiltShift => "Inclinaison-décalage",
            Kind::Average => "Moyenne",
            Kind::SharpenMore => "Plus net",
            Kind::Unsharp => "Accentuation",
            Kind::Noise => "Ajout de bruit",
            Kind::Denoise => "Réduction du bruit",
            Kind::Median => "Médiane",
            Kind::DustScratches => "Antipoussière",
            Kind::Pixelate => "Mosaïque",
            Kind::Crystallize => "Cristallisation",
            Kind::ColorHalftone => "Demi-teintes couleur",
            Kind::Mezzotint => "Manière noire",
            Kind::Emboss => "Estampage",
            Kind::FindEdges => "Contours",
            Kind::GlowingEdges => "Contours lumineux",
            Kind::TraceContour => "Tracé des contours",
            Kind::Diffuse => "Diffusion",
            Kind::Wind => "Vent",
            Kind::Tiles => "Carreaux",
            Kind::Vignette => "Vignettage",
            Kind::Glow => "Lueur diffuse",
            Kind::Chromatic => "Aberration chromatique",
            Kind::OilPaint => "Peinture à l'huile",
            Kind::Sketch => "Croquis au crayon",
            Kind::Cartoon => "Bande dessinée",
            Kind::Cutout => "Découpage",
            Kind::FilmGrain => "Grain du film",
            Kind::Wave => "Ondulation",
            Kind::Ripple => "Onde concentrique",
            Kind::Twirl => "Tourbillon",
            Kind::Pinch => "Pincement",
            Kind::Spherize => "Sphérisation",
            Kind::Polar => "Coordonnées polaires",
            Kind::Shear => "Cisaillement",
            Kind::Halftone => "Trame de demi-teintes",
            Kind::Lighting => "Effets d'éclairage",
            Kind::LensFlare => "Halo",
            Kind::Clouds => "Nuages",
            Kind::HighPass => "Passe-haut",
            Kind::Maximum => "Maximum",
            Kind::Minimum => "Minimum",
            Kind::Offset => "Décalage",
        }
    }

    pub fn is_point(self) -> bool {
        ADJUSTMENTS.contains(&self) || matches!(self, Kind::BlackWhite | Kind::Invert | Kind::Sepia | Kind::Solarize)
    }

    pub fn params(self) -> &'static [Param] {
        match self {
            Kind::BrightnessContrast => const { &[p("Luminosité", -100.0, 100.0, 0.0, ""), p("Contraste", -100.0, 100.0, 0.0, "")] },
            Kind::HueSat => const { &[p("Teinte", -180.0, 180.0, 0.0, "°"), p("Saturation", -100.0, 100.0, 0.0, ""), p("Luminosité", -100.0, 100.0, 0.0, ""), on("Redéfinir", 0.0)] },
            Kind::Levels => const { &[p("Noir", 0.0, 250.0, 0.0, ""), p("Gamma", 0.1, 3.0, 1.0, ""), p("Blanc", 5.0, 255.0, 255.0, ""), p("Sortie noire", 0.0, 255.0, 0.0, ""), p("Sortie blanche", 0.0, 255.0, 255.0, ""), c("Couche", &["RVB", "Rouge", "Vert", "Bleu"], 0.0)] },
            Kind::Curves => const { &[c("Couche", &["RVB", "Rouge", "Vert", "Bleu"], 0.0), p("Noirs", 0.0, 255.0, 0.0, ""), p("Tons foncés", 0.0, 255.0, 64.0, ""), p("Tons moyens", 0.0, 255.0, 128.0, ""), p("Tons clairs", 0.0, 255.0, 192.0, ""), p("Blancs", 0.0, 255.0, 255.0, "")] },
            Kind::Exposure => const { &[p("Exposition", -4.0, 4.0, 0.0, " IL"), p("Décalage", -0.5, 0.5, 0.0, ""), p("Gamma", 0.2, 3.0, 1.0, "")] },
            Kind::Vibrance => const { &[p("Vibrance", -100.0, 100.0, 0.0, ""), p("Saturation", -100.0, 100.0, 0.0, "")] },
            Kind::Temperature => const { &[p("Température", -100.0, 100.0, 0.0, ""), p("Teinte", -100.0, 100.0, 0.0, "")] },
            Kind::ColorBalance => const {
                &[
                    p("Foncés · cyan ↔ rouge", -100.0, 100.0, 0.0, ""),
                    p("Foncés · magenta ↔ vert", -100.0, 100.0, 0.0, ""),
                    p("Foncés · jaune ↔ bleu", -100.0, 100.0, 0.0, ""),
                    p("Moyens · cyan ↔ rouge", -100.0, 100.0, 0.0, ""),
                    p("Moyens · magenta ↔ vert", -100.0, 100.0, 0.0, ""),
                    p("Moyens · jaune ↔ bleu", -100.0, 100.0, 0.0, ""),
                    p("Clairs · cyan ↔ rouge", -100.0, 100.0, 0.0, ""),
                    p("Clairs · magenta ↔ vert", -100.0, 100.0, 0.0, ""),
                    p("Clairs · jaune ↔ bleu", -100.0, 100.0, 0.0, ""),
                    on("Préserver la luminosité", 1.0),
                ]
            },
            Kind::BlackWhiteMix => const {
                &[
                    p("Rouges", -200.0, 300.0, 40.0, " %"),
                    p("Jaunes", -200.0, 300.0, 60.0, " %"),
                    p("Verts", -200.0, 300.0, 40.0, " %"),
                    p("Cyans", -200.0, 300.0, 60.0, " %"),
                    p("Bleus", -200.0, 300.0, 20.0, " %"),
                    p("Magentas", -200.0, 300.0, 80.0, " %"),
                    on("Virage", 0.0),
                    p("Teinte du virage", 0.0, 360.0, 42.0, "°"),
                    p("Saturation du virage", 0.0, 100.0, 25.0, " %"),
                ]
            },
            Kind::PhotoFilter => const { &[c("Filtre", PHOTO_NAMES, 0.0), p("Densité", 0.0, 100.0, 25.0, " %"), on("Préserver la luminosité", 1.0)] },
            Kind::ChannelMixer => const {
                &[
                    p("Rouge ← rouge", -200.0, 200.0, 100.0, " %"),
                    p("Rouge ← vert", -200.0, 200.0, 0.0, " %"),
                    p("Rouge ← bleu", -200.0, 200.0, 0.0, " %"),
                    p("Rouge · constante", -100.0, 100.0, 0.0, " %"),
                    p("Vert ← rouge", -200.0, 200.0, 0.0, " %"),
                    p("Vert ← vert", -200.0, 200.0, 100.0, " %"),
                    p("Vert ← bleu", -200.0, 200.0, 0.0, " %"),
                    p("Vert · constante", -100.0, 100.0, 0.0, " %"),
                    p("Bleu ← rouge", -200.0, 200.0, 0.0, " %"),
                    p("Bleu ← vert", -200.0, 200.0, 0.0, " %"),
                    p("Bleu ← bleu", -200.0, 200.0, 100.0, " %"),
                    p("Bleu · constante", -100.0, 100.0, 0.0, " %"),
                    on("Monochrome (couche rouge)", 0.0),
                ]
            },
            Kind::GradientMap => const { &[c("Dégradé", GRADIENT_NAMES, 0.0), on("Inverser", 0.0)] },
            Kind::SelectiveColor => const { &[c("Couleurs", SELECTIVE, 0.0), p("Cyan", -100.0, 100.0, 0.0, " %"), p("Magenta", -100.0, 100.0, 0.0, " %"), p("Jaune", -100.0, 100.0, 0.0, " %"), p("Noir", -100.0, 100.0, 0.0, " %"), c("Méthode", &["Relative", "Absolue"], 0.0)] },
            Kind::ReplaceColor => const { &[p("Teinte visée", 0.0, 360.0, 0.0, "°"), p("Tolérance", 1.0, 180.0, 30.0, "°"), p("Nouvelle teinte", -180.0, 180.0, 120.0, "°"), p("Saturation", -100.0, 100.0, 0.0, ""), p("Luminosité", -100.0, 100.0, 0.0, "")] },
            Kind::ShadowsHighlights => const { &[p("Tons foncés", 0.0, 100.0, 35.0, " %"), p("Tons clairs", 0.0, 100.0, 0.0, " %"), p("Contraste des tons moyens", -100.0, 100.0, 0.0, ""), p("Correction des couleurs", -100.0, 100.0, 0.0, "")] },
            Kind::Threshold => const { &[p("Seuil", 1.0, 255.0, 128.0, "")] },
            Kind::Posterize => const { &[p("Niveaux", 2.0, 16.0, 4.0, "")] },
            Kind::Blur => const { &[p("Rayon", 0.5, 60.0, 4.0, " px")] },
            Kind::BoxBlur => const { &[p("Rayon horizontal", 0.0, 100.0, 6.0, " px"), p("Rayon vertical", 0.0, 100.0, 6.0, " px")] },
            Kind::SurfaceBlur => const { &[p("Rayon", 1.0, 10.0, 4.0, " px"), p("Seuil", 1.0, 100.0, 15.0, "")] },
            Kind::MotionBlur => const { &[p("Distance", 2.0, 120.0, 20.0, " px"), p("Angle", -180.0, 180.0, 0.0, "°")] },
            Kind::RadialBlur => const { &[p("Intensité", 1.0, 100.0, 20.0, ""), p("Centre X", 0.0, 100.0, 50.0, " %"), p("Centre Y", 0.0, 100.0, 50.0, " %")] },
            Kind::SpinBlur => const { &[p("Angle", 1.0, 60.0, 10.0, "°"), p("Centre X", 0.0, 100.0, 50.0, " %"), p("Centre Y", 0.0, 100.0, 50.0, " %")] },
            Kind::TiltShift => const { &[p("Position", 0.0, 100.0, 50.0, " %"), p("Zone nette", 0.0, 100.0, 20.0, " %"), p("Transition", 1.0, 100.0, 25.0, " %"), p("Flou", 1.0, 40.0, 10.0, " px")] },
            Kind::Unsharp => const { &[p("Gain", 0.0, 300.0, 80.0, " %"), p("Rayon", 0.5, 20.0, 1.5, " px"), p("Seuil", 0.0, 255.0, 0.0, "")] },
            Kind::Noise => const { &[p("Quantité", 0.0, 100.0, 12.0, " %"), on("Monochrome", 1.0), c("Répartition", &["Uniforme", "Gaussienne"], 0.0)] },
            Kind::Denoise => const { &[p("Rayon", 1.0, 3.0, 1.0, " px")] },
            Kind::Median => const { &[p("Rayon", 1.0, 5.0, 2.0, " px")] },
            Kind::DustScratches => const { &[p("Rayon", 1.0, 5.0, 2.0, " px"), p("Seuil", 0.0, 128.0, 20.0, "")] },
            Kind::Pixelate => const { &[p("Taille de cellule", 2.0, 120.0, 12.0, " px")] },
            Kind::Crystallize => const { &[p("Taille de cellule", 3.0, 100.0, 16.0, " px")] },
            Kind::ColorHalftone => const { &[p("Rayon maximal", 4.0, 40.0, 8.0, " px")] },
            Kind::Mezzotint => const { &[c("Type", &["Points fins", "Points moyens", "Gros grains", "Traits courts", "Traits longs"], 0.0)] },
            Kind::Emboss => const { &[p("Relief", 0.5, 6.0, 2.0, ""), p("Angle", -180.0, 180.0, 135.0, "°"), p("Hauteur", 1.0, 10.0, 1.0, " px")] },
            Kind::GlowingEdges => const { &[p("Largeur", 1.0, 5.0, 1.0, " px"), p("Luminosité", 10.0, 400.0, 150.0, " %")] },
            Kind::TraceContour => const { &[p("Niveau", 1.0, 254.0, 128.0, "")] },
            Kind::Diffuse => const { &[p("Rayon", 1.0, 10.0, 2.0, " px")] },
            Kind::Wind => const { &[p("Force", 2.0, 120.0, 30.0, " px"), c("Sens", &["Depuis la gauche", "Depuis la droite"], 1.0)] },
            Kind::Tiles => const { &[p("Nombre de carreaux", 2.0, 50.0, 10.0, ""), p("Décalage maximal", 0.0, 50.0, 10.0, " %")] },
            Kind::Vignette => const { &[p("Quantité", -100.0, 100.0, -50.0, ""), p("Taille", 0.0, 100.0, 50.0, " %")] },
            Kind::Glow => const { &[p("Rayon", 2.0, 80.0, 18.0, " px"), p("Intensité", 0.0, 100.0, 45.0, " %")] },
            Kind::Chromatic => const { &[p("Décalage", 1.0, 30.0, 4.0, " px")] },
            Kind::OilPaint => const { &[p("Pinceau", 2.0, 10.0, 4.0, " px")] },
            Kind::Sketch => const { &[p("Rayon", 1.0, 30.0, 8.0, " px"), p("Contraste", 0.0, 100.0, 40.0, "")] },
            Kind::Cartoon => const { &[p("Niveaux", 2.0, 10.0, 5.0, ""), p("Contours", 0.0, 100.0, 50.0, ""), p("Lissage", 0.0, 8.0, 2.0, " px")] },
            Kind::Cutout => const { &[p("Niveaux", 2.0, 8.0, 4.0, ""), p("Simplicité", 1.0, 4.0, 2.0, "")] },
            Kind::FilmGrain => const { &[p("Intensité", 0.0, 100.0, 30.0, " %"), p("Taille du grain", 1.0, 4.0, 1.0, " px")] },
            Kind::Wave => const { &[p("Amplitude", 1.0, 100.0, 12.0, " px"), p("Longueur d'onde", 10.0, 500.0, 80.0, " px")] },
            Kind::Ripple => const { &[p("Amplitude", 1.0, 60.0, 8.0, " px"), p("Longueur d'onde", 4.0, 200.0, 30.0, " px")] },
            Kind::Twirl => const { &[p("Angle", -720.0, 720.0, 180.0, "°"), p("Rayon", 5.0, 100.0, 60.0, " %")] },
            Kind::Pinch => const { &[p("Intensité", -100.0, 100.0, 50.0, " %")] },
            Kind::Spherize => const { &[p("Intensité", -100.0, 100.0, 60.0, "")] },
            Kind::Polar => const { &[c("Conversion", &["Rectangulaire → polaire", "Polaire → rectangulaire"], 0.0)] },
            Kind::Shear => const { &[p("Courbure", -100.0, 100.0, 30.0, " %")] },
            Kind::Halftone => const { &[p("Taille du point", 3.0, 40.0, 8.0, " px")] },
            Kind::Lighting => const { &[p("Lumière X", 0.0, 100.0, 50.0, " %"), p("Lumière Y", 0.0, 100.0, 30.0, " %"), p("Portée", 10.0, 150.0, 70.0, " %"), p("Intensité", 0.0, 200.0, 100.0, " %"), p("Ambiance", 0.0, 100.0, 25.0, " %"), p("Relief", 0.0, 10.0, 2.0, "")] },
            Kind::LensFlare => const { &[p("Centre X", 0.0, 100.0, 30.0, " %"), p("Centre Y", 0.0, 100.0, 30.0, " %"), p("Luminosité", 10.0, 300.0, 100.0, " %")] },
            Kind::Clouds => const { &[p("Échelle", 10.0, 600.0, 160.0, " px"), p("Détail", 1.0, 8.0, 5.0, ""), c("Mode", &["Remplacer", "Différence", "Incrustation"], 0.0)] },
            Kind::HighPass => const { &[p("Rayon", 0.5, 100.0, 10.0, " px")] },
            Kind::Maximum | Kind::Minimum => const { &[p("Rayon", 1.0, 20.0, 2.0, " px")] },
            Kind::Offset => const { &[p("Horizontal", -100.0, 100.0, 50.0, " %"), p("Vertical", -100.0, 100.0, 0.0, " %")] },
            _ => &[],
        }
    }

    pub fn defaults(self) -> Vec<f32> {
        self.params().iter().map(|p| p.default).collect()
    }

    pub fn presets(self) -> &'static [(&'static str, &'static [f32])] {
        match self {
            Kind::Curves => &[
                ("Contraste moyen", &[0.0, 0.0, 52.0, 128.0, 204.0, 255.0]),
                ("Contraste fort", &[0.0, 0.0, 40.0, 128.0, 216.0, 255.0]),
                ("Éclaircir", &[0.0, 0.0, 90.0, 165.0, 220.0, 255.0]),
                ("Assombrir", &[0.0, 0.0, 40.0, 92.0, 160.0, 255.0]),
                ("Film délavé", &[0.0, 40.0, 80.0, 128.0, 190.0, 235.0]),
                ("Négatif", &[0.0, 255.0, 192.0, 128.0, 64.0, 0.0]),
            ],
            Kind::HueSat => &[
                ("Sépia", &[30.0, -60.0, 0.0, 1.0]),
                ("Saturation renforcée", &[0.0, 35.0, 0.0, 0.0]),
                ("Ancien", &[0.0, -40.0, 5.0, 0.0]),
                ("Cyanotype", &[-160.0, -50.0, 0.0, 1.0]),
            ],
            Kind::BrightnessContrast => &[("Punch", &[5.0, 25.0]), ("Doux", &[8.0, -20.0]), ("Sombre", &[-20.0, 10.0])],
            Kind::Levels => &[("Éclaircir les tons moyens", &[0.0, 1.4, 255.0, 0.0, 255.0, 0.0]), ("Augmenter le contraste", &[20.0, 1.0, 235.0, 0.0, 255.0, 0.0]), ("Délavé", &[0.0, 1.0, 255.0, 30.0, 230.0, 0.0])],
            Kind::BlackWhiteMix => &[
                ("Filtre rouge", &[120.0, 110.0, -10.0, -50.0, -50.0, 120.0, 0.0, 42.0, 25.0]),
                ("Filtre vert", &[-40.0, 120.0, 120.0, -50.0, -50.0, -50.0, 0.0, 42.0, 25.0]),
                ("Filtre bleu", &[-50.0, -50.0, -50.0, 120.0, 110.0, 110.0, 0.0, 42.0, 25.0]),
                ("Infrarouge", &[-40.0, 235.0, 144.0, -68.0, -3.0, -107.0, 0.0, 42.0, 25.0]),
                ("Virage sépia", &[40.0, 60.0, 40.0, 60.0, 20.0, 80.0, 1.0, 38.0, 30.0]),
            ],
            Kind::ChannelMixer => &[
                ("Noir et blanc infrarouge", &[-70.0, 200.0, -30.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.0, 100.0, 0.0, 1.0]),
                ("Inverser rouge et bleu", &[0.0, 0.0, 100.0, 0.0, 0.0, 100.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.0]),
                ("Sarcelle et orange", &[110.0, 0.0, -10.0, 0.0, 0.0, 100.0, 0.0, 0.0, -10.0, 10.0, 100.0, 0.0, 0.0]),
            ],
            Kind::Vignette => &[("Assombrir les bords", &[-60.0, 45.0]), ("Éclaircir les bords", &[60.0, 45.0]), ("Discret", &[-25.0, 60.0])],
            Kind::Unsharp => &[("Portrait", &[60.0, 1.0, 4.0]), ("Paysage", &[120.0, 1.2, 0.0]), ("Web", &[180.0, 0.5, 0.0])],
            _ => &[],
        }
    }

    pub fn apply(self, img: &RgbaImage, v: &[f32]) -> RgbaImage {
        let g = |i: usize| v.get(i).copied().unwrap_or_else(|| self.params().get(i).map_or(0.0, |p| p.default));
        let (wf, hf) = (img.width() as f32, img.height() as f32);
        match self {
            Kind::BrightnessContrast => {
                let (b, k) = (g(0) / 100.0, ((g(1) / 100.0) + 1.0).powi(2));
                map(img, move |c| c.map(|x| (x + b - 0.5) * k + 0.5))
            }
            Kind::HueSat => {
                let (h, s, l, colorize) = (g(0) / 360.0, g(1) / 100.0, g(2) / 100.0, g(3) >= 0.5);
                map(img, move |c| {
                    let (hh, ss, ll) = rgb_to_hsl(c[0], c[1], c[2]);
                    let ll = if l >= 0.0 { ll + (1.0 - ll) * l } else { ll * (1.0 + l) };
                    let (r, g, b) = if colorize { hsl_to_rgb((h + 1.0).rem_euclid(1.0), (0.25 * (1.0 + s)).clamp(0.0, 1.0), ll) } else { hsl_to_rgb(hh + h, (ss * (1.0 + s)).clamp(0.0, 1.0), ll) };
                    [r, g, b]
                })
            }
            Kind::Levels => {
                let (lo, gamma, hi) = (g(0) / 255.0, g(1).max(0.05), (g(2) / 255.0).max(g(0) / 255.0 + 0.01));
                let (olo, ohi, ch) = (g(3) / 255.0, g(4) / 255.0, g(5).round() as usize);
                map(img, move |c| {
                    let mut o = c;
                    for k in 0..3 {
                        if ch == 0 || ch == k + 1 {
                            o[k] = olo + (ohi - olo) * ((c[k] - lo) / (hi - lo)).clamp(0.0, 1.0).powf(1.0 / gamma);
                        }
                    }
                    o
                })
            }
            Kind::Curves => {
                let ch = g(0).round() as usize;
                let lut = curve_lut([g(1), g(2), g(3), g(4), g(5)].map(|y| y / 255.0));
                map(img, move |c| {
                    let mut o = c;
                    for k in 0..3 {
                        if ch == 0 || ch == k + 1 {
                            o[k] = lut[(c[k].clamp(0.0, 1.0) * 255.0).round() as usize];
                        }
                    }
                    o
                })
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
            Kind::ColorBalance => {
                let a: [f32; 9] = std::array::from_fn(|i| g(i) / 100.0 * 0.35);
                let keep = g(9) >= 0.5;
                map(img, move |c| {
                    let l = luma(c);
                    let ws = (1.0 - l).powi(2);
                    let wh = l * l;
                    let wm = (1.0 - ws - wh).max(0.0);
                    let mut o = [0f32; 3];
                    for k in 0..3 {
                        o[k] = c[k] + a[k] * ws + a[3 + k] * wm + a[6 + k] * wh;
                    }
                    if keep {
                        let d = l - luma(o);
                        o = o.map(|x| x + d);
                    }
                    o
                })
            }
            Kind::BlackWhiteMix => {
                let wt: [f32; 6] = std::array::from_fn(|i| g(i) / 100.0);
                let (tint, th, ts) = (g(6) >= 0.5, g(7) / 360.0, g(8) / 100.0);
                map(img, move |c| {
                    let [r, gg, b] = c;
                    let mn = r.min(gg).min(b);
                    let mx = r.max(gg).max(b);
                    let gray = if r == mn {
                        let mid = gg.min(b);
                        mn + wt[3] * (mid - mn) + if gg > b { wt[2] } else { wt[4] } * (mx - mid)
                    } else if gg == mn {
                        let mid = r.min(b);
                        mn + wt[5] * (mid - mn) + if r > b { wt[0] } else { wt[4] } * (mx - mid)
                    } else {
                        let mid = r.min(gg);
                        mn + wt[1] * (mid - mn) + if r > gg { wt[0] } else { wt[2] } * (mx - mid)
                    }
                    .clamp(0.0, 1.0);
                    if tint {
                        let (r2, g2, b2) = hsl_to_rgb(th, ts, gray);
                        [r2, g2, b2]
                    } else {
                        [gray; 3]
                    }
                })
            }
            Kind::PhotoFilter => {
                let f = PHOTO_FILTERS[(g(0).round() as usize).min(PHOTO_FILTERS.len() - 1)].1;
                let (d, keep) = (g(1) / 100.0, g(2) >= 0.5);
                let fl = luma(f).max(0.2);
                map(img, move |c| {
                    let mut o: [f32; 3] = std::array::from_fn(|k| c[k] + (c[k] * f[k] / fl - c[k]) * d);
                    if keep {
                        let dl = luma(c) - luma(o);
                        o = o.map(|x| x + dl);
                    }
                    o
                })
            }
            Kind::ChannelMixer => {
                let m: [f32; 12] = std::array::from_fn(|i| g(i) / 100.0);
                let mono = g(12) >= 0.5;
                map(img, move |c| {
                    let row = |r: usize| m[r * 4] * c[0] + m[r * 4 + 1] * c[1] + m[r * 4 + 2] * c[2] + m[r * 4 + 3];
                    if mono { [row(0); 3] } else { [row(0), row(1), row(2)] }
                })
            }
            Kind::GradientMap => {
                let stops = GRADIENT_MAPS[(g(0).round() as usize).min(GRADIENT_MAPS.len() - 1)].1;
                let inv = g(1) >= 0.5;
                map(img, move |c| {
                    let l = luma(c).clamp(0.0, 1.0);
                    gradient_at(stops, if inv { 1.0 - l } else { l })
                })
            }
            Kind::SelectiveColor => {
                let (target, adj, absolute) = (g(0).round() as usize, [g(1), g(2), g(3), g(4)].map(|x| x / 100.0), g(5) >= 0.5);
                map(img, move |c| {
                    let (h, s, l) = rgb_to_hsl(c[0], c[1], c[2]);
                    let mx = c[0].max(c[1]).max(c[2]);
                    let mn = c[0].min(c[1]).min(c[2]);
                    let w = match target {
                        0..=5 => {
                            let center = target as f32 / 6.0;
                            let d = ((h - center + 0.5).rem_euclid(1.0) - 0.5).abs() * 6.0;
                            (1.0 - d).max(0.0) * (mx - mn)
                        }
                        6 => ((mn - 0.5) * 2.0).max(0.0),
                        7 => (1.0 - s) * (1.0 - (l - 0.5).abs() * 2.0),
                        _ => ((0.5 - mx) * 2.0).max(0.0),
                    };
                    let mut o = c;
                    for k in 0..3 {
                        let base = if absolute { 1.0 } else { 1.0 - c[k] };
                        o[k] -= (adj[k] * base + adj[3] * if absolute { 1.0 } else { c[k] }) * w;
                    }
                    o
                })
            }
            Kind::ReplaceColor => {
                let (hue, tol, shift, sat, lig) = (g(0) / 360.0, g(1) / 360.0, g(2) / 360.0, g(3) / 100.0, g(4) / 100.0);
                map(img, move |c| {
                    let (h, s, l) = rgb_to_hsl(c[0], c[1], c[2]);
                    let d = ((h - hue + 0.5).rem_euclid(1.0) - 0.5).abs();
                    let w = smooth(1.0 - d / tol.max(1e-3)) * smooth(s / 0.15);
                    if w <= 0.0 {
                        return c;
                    }
                    let nl = if lig >= 0.0 { l + (1.0 - l) * lig } else { l * (1.0 + lig) };
                    let (r, g2, b) = hsl_to_rgb(h + shift, (s * (1.0 + sat)).clamp(0.0, 1.0), nl);
                    [c[0] + (r - c[0]) * w, c[1] + (g2 - c[1]) * w, c[2] + (b - c[2]) * w]
                })
            }
            Kind::ShadowsHighlights => {
                let (sh, hi, mid, col) = (g(0) / 100.0 * 0.6, g(1) / 100.0 * 0.6, g(2) / 100.0, g(3) / 100.0);
                map(img, move |c| {
                    let l = luma(c);
                    let d = sh * (1.0 - l).powi(3) - hi * l.powi(3);
                    let m = mid * (l - 0.5) * (1.0 - (l - 0.5).abs() * 2.0) * 0.8;
                    let o = c.map(|x| x + d + m);
                    let ol = luma(o);
                    o.map(|x| ol + (x - ol) * (1.0 + col * (d.abs() * 3.0).min(1.0)))
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
                let (lo, hi) = channel_stretch(img);
                map(img, move |c| std::array::from_fn(|k| (c[k] - lo[k]) / (hi[k] - lo[k])))
            }
            Kind::AutoTone => {
                let mut hist = [0u32; 256];
                img.pixels().filter(|p| p.0[3] > 0).for_each(|p| hist[(luma(rgb(p.0)) * 255.0).round() as usize] += 1);
                let cut = (hist.iter().sum::<u32>() as f32 * 0.005) as u32;
                let lo = percentile(&hist, cut) as f32 / 255.0;
                let hi = ((255 - percentile(&rev(&hist), cut)) as f32 / 255.0).max(lo + 0.02);
                map(img, move |c| c.map(|x| (x - lo) / (hi - lo)))
            }
            Kind::AutoColor => {
                let mut sum = [0f64; 3];
                let mut n = 0f64;
                for px in img.pixels().filter(|p| p.0[3] > 0) {
                    for k in 0..3 {
                        sum[k] += px.0[k] as f64;
                    }
                    n += 1.0;
                }
                let mean = sum.map(|s| (s / n.max(1.0) / 255.0) as f32);
                let gray = (mean[0] + mean[1] + mean[2]) / 3.0;
                let gain = mean.map(|m| if m > 1e-3 { gray / m } else { 1.0 });
                let balanced = map(img, move |c| std::array::from_fn(|k| c[k] * gain[k]));
                Kind::AutoTone.apply(&balanced, &[])
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
            Kind::BoxBlur => box_blur(img, g(0).round() as usize, g(1).round() as usize),
            Kind::SurfaceBlur => {
                let (r, t) = (g(0).round() as i32, g(1) / 255.0 * 2.5);
                sample_i(img, move |x, y, at| {
                    let c0 = at(x, y);
                    let mut acc = [0f32; 4];
                    let mut wsum = 0.0;
                    for dy in -r..=r {
                        for dx in -r..=r {
                            let q = at(x + dx, y + dy);
                            let d = ((q[0] - c0[0]).abs() + (q[1] - c0[1]).abs() + (q[2] - c0[2]).abs()) / 3.0;
                            let w = (1.0 - d / t.max(1e-3)).max(0.0);
                            for k in 0..3 {
                                acc[k] += q[k] * w;
                            }
                            wsum += w;
                        }
                    }
                    [acc[0] / wsum, acc[1] / wsum, acc[2] / wsum, c0[3]]
                })
            }
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
                let (cx, cy) = (wf * g(1) / 100.0, hf * g(2) / 100.0);
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
            Kind::SpinBlur => {
                let ang = g(0).to_radians();
                let (cx, cy) = (wf * g(1) / 100.0, hf * g(2) / 100.0);
                sample(img, move |x, y, at| {
                    let n = 24;
                    let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                    let mut acc = [0f32; 4];
                    for i in 0..n {
                        let a = ang * (i as f32 / (n - 1) as f32 - 0.5);
                        let (sn, cs) = a.sin_cos();
                        add(&mut acc, at(cx + dx * cs - dy * sn, cy + dx * sn + dy * cs));
                    }
                    acc.map(|v| v / n as f32)
                })
            }
            Kind::TiltShift => {
                let (pos, band, soft) = (g(0) / 100.0, g(1) / 200.0, g(2) / 100.0);
                let blurred = imageops::fast_blur(img, g(3));
                let h = img.height() as f32;
                mix_by(img, &blurred, move |_, y| smooth(((y as f32 / h - pos).abs() - band) / soft.max(1e-3)))
            }
            Kind::Average => {
                let mut sum = [0f64; 3];
                let mut n = 0f64;
                for px in img.pixels().filter(|p| p.0[3] > 0) {
                    for k in 0..3 {
                        sum[k] += px.0[k] as f64;
                    }
                    n += 1.0;
                }
                let m = sum.map(|s| (s / n.max(1.0) / 255.0) as f32);
                map(img, move |_| m)
            }
            Kind::SharpenMore => sample_i(img, |x, y, at| {
                let c = at(x, y);
                let n = [at(x - 1, y), at(x + 1, y), at(x, y - 1), at(x, y + 1)];
                std::array::from_fn(|k| if k == 3 { c[3] } else { c[k] * 5.0 - n.iter().map(|q| q[k]).sum::<f32>() })
            }),
            Kind::Unsharp => {
                let (amount, th) = (g(0) / 100.0, g(2) / 255.0);
                let blurred = imageops::fast_blur(img, g(1));
                zip(img, &blurred, move |a, b| std::array::from_fn(|k| if (a[k] - b[k]).abs() * 2.0 >= th { a[k] + (a[k] - b[k]) * amount } else { a[k] }))
            }
            Kind::Noise => {
                let (amount, mono, gauss) = (g(0) / 100.0 * 0.5, g(1) >= 0.5, g(2) >= 0.5);
                let w = img.width();
                let mut out = img.clone();
                out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
                    let (x, y) = (i as u32 % w, i as u32 / w);
                    let n = |k: u32| if gauss { (hash(x, y, k) + hash(x, y, k + 7) + hash(x, y, k + 13) - 1.5) * 1.2 } else { hash(x, y, k) - 0.5 };
                    for c in 0..3 {
                        let d = if mono { n(0) } else { n(c as u32) } * amount;
                        px[c] = ((px[c] as f32 / 255.0 + d).clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                });
                out
            }
            Kind::Denoise | Kind::Median => median(img, g(0).round().max(1.0) as i32, 0.0),
            Kind::DustScratches => median(img, g(0).round().max(1.0) as i32, g(1) / 255.0),
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
                        let avg = Rgba(acc.map(|a| (a / n) as u8));
                        for y in by..y1 {
                            for x in bx..x1 {
                                out.put_pixel(x, y, avg);
                            }
                        }
                    }
                }
                out
            }
            Kind::Crystallize => {
                let cell = g(0).max(2.0);
                let seed = |cx: i32, cy: i32| ((cx as f32 + hash(cx as u32, cy as u32, 1)) * cell, (cy as f32 + hash(cx as u32, cy as u32, 2)) * cell);
                sample(img, move |x, y, at| {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let (gx, gy) = ((fx / cell).floor() as i32, (fy / cell).floor() as i32);
                    let mut best = (f32::MAX, (fx, fy));
                    for oy in -1..=1 {
                        for ox in -1..=1 {
                            let s = seed(gx + ox, gy + oy);
                            let d = (s.0 - fx).powi(2) + (s.1 - fy).powi(2);
                            if d < best.0 {
                                best = (d, s);
                            }
                        }
                    }
                    let mut c = at(best.1.0, best.1.1);
                    c[3] = at(fx, fy)[3];
                    c
                })
            }
            Kind::ColorHalftone => {
                let cell = g(0).max(2.0) * 1.4;
                let angles = [15f32, 75.0, 0.0].map(|a| a.to_radians());
                sample(img, move |x, y, at| {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let mut o = [0f32; 4];
                    for k in 0..3 {
                        let (sn, cs) = angles[k].sin_cos();
                        let (u, v) = (fx * cs + fy * sn, -fx * sn + fy * cs);
                        let (cu, cv) = (((u / cell).floor() + 0.5) * cell, ((v / cell).floor() + 0.5) * cell);
                        let (sx, sy) = (cu * cs - cv * sn, cu * sn + cv * cs);
                        let ink = 1.0 - at(sx, sy)[k];
                        let radius = ink.max(0.0).sqrt() * cell * 0.7;
                        let d = ((u - cu).powi(2) + (v - cv).powi(2)).sqrt();
                        o[k] = if d < radius { 0.0 } else { 1.0 };
                    }
                    o[3] = at(fx, fy)[3];
                    o
                })
            }
            Kind::Mezzotint => {
                let kind = g(0).round() as u32;
                sample_i(img, move |x, y, at| {
                    let c = at(x, y);
                    let (xu, yu) = (x as u32, y as u32);
                    let n = |k: u32| match kind {
                        0 => hash(xu, yu, k),
                        1 => hash(xu / 2, yu / 2, k),
                        2 => hash(xu / 4, yu / 4, k),
                        3 => hash(xu / 6, yu, k),
                        _ => hash(xu / 16, yu, k),
                    };
                    [if c[0] > n(0) { 1.0 } else { 0.0 }, if c[1] > n(1) { 1.0 } else { 0.0 }, if c[2] > n(2) { 1.0 } else { 0.0 }, c[3]]
                })
            }
            Kind::Emboss => {
                let (k, a, hgt) = (g(0), g(1).to_radians(), g(2));
                let (dx, dy) = (a.cos() * hgt, -a.sin() * hgt);
                sample(img, move |x, y, at| {
                    let (fx, fy) = (x as f32, y as f32);
                    let d = (luma(rgb4(at(fx + dx, fy + dy))) - luma(rgb4(at(fx - dx, fy - dy)))) * k;
                    [0.5 + d, 0.5 + d, 0.5 + d, at(fx, fy)[3]]
                })
            }
            Kind::FindEdges => sample(img, |x, y, at| {
                let e = 1.0 - sobel(at, x as f32, y as f32, 1.0).min(1.0);
                [e, e, e, at(x as f32, y as f32)[3]]
            }),
            Kind::GlowingEdges => {
                let (wd, br) = (g(0), g(1) / 100.0);
                sample(img, move |x, y, at| {
                    let c = at(x as f32, y as f32);
                    let e = (sobel(at, x as f32, y as f32, wd) * br).min(1.0);
                    [c[0] * e * 1.4, c[1] * e * 1.4, c[2] * e * 1.4, c[3]]
                })
            }
            Kind::TraceContour => {
                let t = g(0) / 255.0;
                sample_i(img, move |x, y, at| {
                    let l = luma(rgb4(at(x, y))) >= t;
                    let edge = l != (luma(rgb4(at(x + 1, y))) >= t) || l != (luma(rgb4(at(x, y + 1))) >= t);
                    let v = if edge { 0.0 } else { 1.0 };
                    [v, v, v, at(x, y)[3]]
                })
            }
            Kind::Diffuse => {
                let r = g(0);
                sample(img, move |x, y, at| {
                    let (xu, yu) = (x, y);
                    at(x as f32 + (hash(xu, yu, 3) - 0.5) * 2.0 * r, y as f32 + (hash(xu, yu, 4) - 0.5) * 2.0 * r)
                })
            }
            Kind::Wind => {
                let (n, dir) = (g(0).max(2.0), if g(1) >= 0.5 { 1.0 } else { -1.0 });
                sample(img, move |x, y, at| {
                    let c = at(x as f32, y as f32);
                    let len = n * (0.3 + 0.7 * hash(0, y as u32, 9));
                    let mut o = c;
                    let mut t = 2.0;
                    while t < len {
                        let q = at(x as f32 + dir * t, y as f32);
                        let fade = 1.0 - t / len;
                        for k in 0..3 {
                            o[k] = o[k].max(q[k] * fade + c[k] * (1.0 - fade));
                        }
                        t += 2.0;
                    }
                    o
                })
            }
            Kind::Tiles => {
                let size = (wf / g(0).max(1.0)).max(2.0);
                let maxo = g(1) / 100.0 * size;
                sample(img, move |x, y, at| {
                    let (fx, fy) = (x as f32, y as f32);
                    let (cx, cy) = ((fx / size).floor(), (fy / size).floor());
                    for (ox, oy) in [(0.0, 0.0), (-1.0, 0.0), (0.0, -1.0), (-1.0, -1.0)] {
                        let (tx, ty) = (cx + ox, cy + oy);
                        let off = ((hash(tx as u32, ty as u32, 5) - 0.5) * 2.0 * maxo, (hash(tx as u32, ty as u32, 6) - 0.5) * 2.0 * maxo);
                        let (lx, ly) = (fx - tx * size - off.0, fy - ty * size - off.1);
                        if lx >= 0.0 && ly >= 0.0 && lx < size - 1.0 && ly < size - 1.0 {
                            return at(tx * size + lx, ty * size + ly);
                        }
                    }
                    [1.0, 1.0, 1.0, at(fx, fy)[3]]
                })
            }
            Kind::Vignette => {
                let (amount, size) = (g(0) / 100.0, g(1) / 100.0);
                let ww = img.width();
                let mut out = img.clone();
                out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
                    let (x, y) = ((i as u32 % ww) as f32 + 0.5, (i as u32 / ww) as f32 + 0.5);
                    let d = (((x / wf - 0.5) * 2.0).powi(2) + ((y / hf - 0.5) * 2.0).powi(2)).sqrt() / std::f32::consts::SQRT_2;
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
                zip(img, &blurred, move |a, b| {
                    std::array::from_fn(|c| {
                        let glow = (b[c] - 0.35).max(0.0) / 0.65 * k;
                        1.0 - (1.0 - a[c]) * (1.0 - glow)
                    })
                })
            }
            Kind::Chromatic => {
                let s = g(0);
                sample(img, move |x, y, at| {
                    let (x, y) = (x as f32, y as f32);
                    let c = at(x, y);
                    [at(x - s, y)[0], c[1], at(x + s, y)[2], c[3]]
                })
            }
            Kind::OilPaint => kuwahara(img, g(0).round().max(1.0) as i32),
            Kind::Sketch => {
                let contrast = 1.0 + g(1) / 50.0;
                let inv = map(img, |c| [1.0 - luma(c); 3]);
                let blurred = imageops::fast_blur(&inv, g(0));
                zip(img, &blurred, move |a, b| {
                    let v = (luma(a) / (1.0 - b[0]).max(1e-3)).min(1.0).powf(contrast);
                    [v; 3]
                })
            }
            Kind::Cartoon => {
                let (n, edges, smooth_r) = (g(0).round().max(2.0) - 1.0, g(1) / 100.0, g(2));
                let base = if smooth_r > 0.0 { imageops::fast_blur(img, smooth_r) } else { img.clone() };
                let flat = map(&base, move |c| c.map(|x| (x * n).round() / n));
                let edge_src = base.clone();
                let ew = edge_src.width();
                let mut out = flat;
                let raw = edge_src.as_raw();
                let (w, h) = (edge_src.width() as i64, edge_src.height() as i64);
                let at = |x: i64, y: i64| {
                    let o = ((y.clamp(0, h - 1) * w + x.clamp(0, w - 1)) * 4) as usize;
                    (0.2126 * raw[o] as f32 + 0.7152 * raw[o + 1] as f32 + 0.0722 * raw[o + 2] as f32) / 255.0
                };
                out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
                    let (x, y) = ((i as u32 % ew) as i64, (i as u32 / ew) as i64);
                    let gx = at(x + 1, y) - at(x - 1, y);
                    let gy = at(x, y + 1) - at(x, y - 1);
                    if (gx * gx + gy * gy).sqrt() > 0.25 * (1.05 - edges) {
                        px[0] = 20;
                        px[1] = 20;
                        px[2] = 24;
                    }
                });
                out
            }
            Kind::Cutout => {
                let n = g(0).round().max(2.0) - 1.0;
                let m = median(img, g(1).round().max(1.0) as i32, 0.0);
                map(&m, move |c| {
                    let l = luma(c);
                    let ql = (l * n).round() / n;
                    let k = if l > 1e-3 { ql / l } else { 0.0 };
                    c.map(|x| ((x * k) * n).round() / n)
                })
            }
            Kind::FilmGrain => {
                let (amt, size) = (g(0) / 100.0 * 0.35, g(1).max(1.0) as u32);
                let w = img.width();
                let mut out = img.clone();
                out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
                    let (x, y) = (i as u32 % w / size, i as u32 / w / size);
                    let c = [px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0];
                    let l = luma(c);
                    let n = (hash(x, y, 11) + hash(x, y, 12) - 1.0) * amt * (1.0 - (l - 0.5).abs() * 1.4).max(0.2);
                    for k in 0..3 {
                        px[k] = ((c[k] + n).clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                });
                out
            }
            Kind::Wave => {
                let (amp, len) = (g(0), g(1).max(1.0));
                sample(img, move |x, y, at| {
                    let (x, y) = (x as f32, y as f32);
                    at(x + amp * (y / len * TAU).sin(), y + amp * (x / len * TAU).sin())
                })
            }
            Kind::Ripple => {
                let (amp, len) = (g(0), g(1).max(1.0));
                let (cx, cy) = (wf / 2.0, hf / 2.0);
                sample(img, move |x, y, at| {
                    let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                    let d = (dx * dx + dy * dy).sqrt().max(1e-3);
                    let off = amp * (d / len * TAU).sin();
                    at(x as f32 + dx / d * off, y as f32 + dy / d * off)
                })
            }
            Kind::Twirl => {
                let (ang, frac) = (g(0).to_radians(), g(1) / 100.0);
                let (cx, cy) = (wf / 2.0, hf / 2.0);
                let radius = cx.min(cy) * frac;
                sample(img, move |x, y, at| {
                    let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                    let d = (dx * dx + dy * dy).sqrt();
                    if d >= radius {
                        return at(x as f32, y as f32);
                    }
                    let a = ang * (1.0 - d / radius).powi(2);
                    let (sn, cs) = a.sin_cos();
                    at(cx + dx * cs - dy * sn, cy + dx * sn + dy * cs)
                })
            }
            Kind::Pinch => {
                let k = g(0) / 100.0;
                let (cx, cy) = (wf / 2.0, hf / 2.0);
                let r = cx.min(cy);
                sample(img, move |x, y, at| {
                    let (dx, dy) = ((x as f32 - cx) / r, (y as f32 - cy) / r);
                    let d = (dx * dx + dy * dy).sqrt();
                    if d >= 1.0 || d == 0.0 {
                        return at(x as f32, y as f32);
                    }
                    let f = (d * PI / 2.0).sin().powf(-k * 0.9) * d / d.max(1e-3);
                    let f = f.min(1.0 / d);
                    at(cx + dx * f * r, cy + dy * f * r)
                })
            }
            Kind::Spherize => {
                let k = g(0) / 100.0;
                let (cx, cy) = (wf / 2.0, hf / 2.0);
                let r = cx.min(cy);
                sample(img, move |x, y, at| {
                    let (dx, dy) = ((x as f32 - cx) / r, (y as f32 - cy) / r);
                    let d = (dx * dx + dy * dy).sqrt();
                    if d >= 1.0 || d == 0.0 {
                        return at(x as f32, y as f32);
                    }
                    let nd = if k >= 0.0 { d.powf(1.0 + k) } else { d.powf(1.0 / (1.0 - k)) };
                    let f = nd / d;
                    at(cx + dx * f * r, cy + dy * f * r)
                })
            }
            Kind::Polar => {
                let to_polar = g(0) < 0.5;
                let (cx, cy) = (wf / 2.0, hf / 2.0);
                let rmax = cx.min(cy);
                sample(img, move |x, y, at| {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    if to_polar {
                        let (dx, dy) = (fx - cx, fy - cy);
                        let a = (dx.atan2(-dy) + TAU) % TAU;
                        let d = (dx * dx + dy * dy).sqrt() / rmax;
                        if d > 1.0 {
                            return [0.0; 4];
                        }
                        at(a / TAU * wf, d * hf)
                    } else {
                        let a = fx / wf * TAU;
                        let d = fy / hf * rmax;
                        at(cx + a.sin() * d, cy - a.cos() * d)
                    }
                })
            }
            Kind::Shear => {
                let k = g(0) / 100.0 * wf * 0.25;
                sample(img, move |x, y, at| {
                    let off = k * (PI * y as f32 / hf).sin();
                    at((x as f32 - off).rem_euclid(wf), y as f32)
                })
            }
            Kind::Halftone => {
                let cell = g(0).max(2.0);
                sample(img, move |x, y, at| {
                    let (cx, cy) = (((x as f32 / cell).floor() + 0.5) * cell, ((y as f32 / cell).floor() + 0.5) * cell);
                    let c = at(cx, cy);
                    let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                    let radius = (1.0 - l).sqrt() * cell * 0.7;
                    let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                    let v = if d < radius { 0.0 } else { 1.0 };
                    [v, v, v, at(x as f32, y as f32)[3]]
                })
            }
            Kind::Lighting => {
                let (lx, ly, reach, power, ambient, bump) = (wf * g(0) / 100.0, hf * g(1) / 100.0, wf.max(hf) * g(2) / 100.0, g(3) / 100.0, g(4) / 100.0, g(5));
                sample(img, move |x, y, at| {
                    let (fx, fy) = (x as f32, y as f32);
                    let c = at(fx, fy);
                    let d = ((fx - lx).powi(2) + (fy - ly).powi(2)).sqrt() / reach.max(1.0);
                    let fall = (1.0 - d).max(0.0).powf(1.5);
                    let nx = (luma(rgb4(at(fx + 1.0, fy))) - luma(rgb4(at(fx - 1.0, fy)))) * bump;
                    let ny = (luma(rgb4(at(fx, fy + 1.0))) - luma(rgb4(at(fx, fy - 1.0)))) * bump;
                    let (vx, vy) = ((lx - fx) / reach.max(1.0), (ly - fy) / reach.max(1.0));
                    let shade = (1.0 + (nx * vx + ny * vy) * 4.0).clamp(0.3, 1.7);
                    let k = ambient + fall * power * shade;
                    [c[0] * k, c[1] * k, c[2] * k, c[3]]
                })
            }
            Kind::LensFlare => {
                let (fx0, fy0, br) = (wf * g(0) / 100.0, hf * g(1) / 100.0, g(2) / 100.0);
                let (cx, cy) = (wf / 2.0, hf / 2.0);
                let diag = (wf * wf + hf * hf).sqrt();
                let rings = [(0.5f32, 0.05f32, [0.4f32, 0.6, 1.0]), (0.8, 0.03, [1.0, 0.6, 0.3]), (1.3, 0.08, [0.5, 1.0, 0.6]), (1.7, 0.04, [0.8, 0.5, 1.0])];
                map_xy(img, move |x, y, c| {
                    let d = ((x - fx0).powi(2) + (y - fy0).powi(2)).sqrt() / diag;
                    let mut add = [0f32; 3];
                    let core = (-d * 40.0).exp() * 1.2 + (-d * 8.0).exp() * 0.35;
                    for k in 0..3 {
                        add[k] += core * [1.0, 0.95, 0.85][k];
                    }
                    for (t, rad, col) in rings {
                        let (rx, ry) = (fx0 + (cx - fx0) * t, fy0 + (cy - fy0) * t);
                        let dd = ((x - rx).powi(2) + (y - ry).powi(2)).sqrt() / diag;
                        let ring = (1.0 - ((dd - rad).abs() / (rad * 0.5)).min(1.0)) * 0.25 + if dd < rad { 0.06 } else { 0.0 };
                        for k in 0..3 {
                            add[k] += ring * col[k];
                        }
                    }
                    std::array::from_fn(|k| 1.0 - (1.0 - c[k]) * (1.0 - (add[k] * br).min(1.0)))
                })
            }
            Kind::Clouds => {
                let (scale, oct, mode) = (g(0).max(2.0), g(1).round().max(1.0) as u32, g(2).round() as u32);
                map_xy(img, move |x, y, c| {
                    let n = fbm(x / scale, y / scale, oct);
                    match mode {
                        0 => [n; 3],
                        1 => c.map(|v| (v - n).abs()),
                        _ => c.map(|v| if v < 0.5 { 2.0 * v * n } else { 1.0 - 2.0 * (1.0 - v) * (1.0 - n) }),
                    }
                })
            }
            Kind::HighPass => {
                let blurred = imageops::fast_blur(img, g(0));
                zip(img, &blurred, |a, b| std::array::from_fn(|k| 0.5 + a[k] - b[k]))
            }
            Kind::Maximum => morph(img, g(0).round() as usize, true),
            Kind::Minimum => morph(img, g(0).round() as usize, false),
            Kind::Offset => {
                let (ox, oy) = (g(0) / 100.0 * wf, g(1) / 100.0 * hf);
                sample(img, move |x, y, at| at((x as f32 - ox).rem_euclid(wf), (y as f32 - oy).rem_euclid(hf)))
            }
        }
    }
}

pub fn gradient_at(stops: &[[u8; 3]], t: f32) -> [f32; 3] {
    if stops.len() == 1 {
        return stops[0].map(|v| v as f32 / 255.0);
    }
    let f = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (f.floor() as usize).min(stops.len() - 2);
    let k = f - i as f32;
    std::array::from_fn(|c| (stops[i][c] as f32 + (stops[i + 1][c] as f32 - stops[i][c] as f32) * k) / 255.0)
}

fn curve_lut(ys: [f32; 5]) -> [f32; 256] {
    let xs = [0.0, 0.25, 0.5, 0.75, 1.0];
    let n = 5;
    let mut d = [0f32; 4];
    for i in 0..4 {
        d[i] = (ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]);
    }
    let mut m = [0f32; 5];
    m[0] = d[0];
    m[4] = d[3];
    for i in 1..4 {
        m[i] = if d[i - 1] * d[i] <= 0.0 { 0.0 } else { (d[i - 1] + d[i]) / 2.0 };
    }
    for i in 0..4 {
        if d[i] == 0.0 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
        } else {
            let (a, b) = (m[i] / d[i], m[i + 1] / d[i]);
            let s = a * a + b * b;
            if s > 9.0 {
                let t = 3.0 / s.sqrt();
                m[i] = t * a * d[i];
                m[i + 1] = t * b * d[i];
            }
        }
    }
    std::array::from_fn(|k| {
        let x = k as f32 / 255.0;
        let i = ((x * (n - 1) as f32).floor() as usize).min(3);
        let h = xs[i + 1] - xs[i];
        let t = (x - xs[i]) / h;
        let (t2, t3) = (t * t, t * t * t);
        let y = (2.0 * t3 - 3.0 * t2 + 1.0) * ys[i] + (t3 - 2.0 * t2 + t) * h * m[i] + (-2.0 * t3 + 3.0 * t2) * ys[i + 1] + (t3 - t2) * h * m[i + 1];
        y.clamp(0.0, 1.0)
    })
}

fn channel_stretch(img: &RgbaImage) -> ([f32; 3], [f32; 3]) {
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
    (lo, hi)
}

fn sobel(at: &dyn Fn(f32, f32) -> [f32; 4], x: f32, y: f32, s: f32) -> f32 {
    let l = |dx: f32, dy: f32| luma(rgb4(at(x + dx * s, y + dy * s)));
    let gx = l(1.0, -1.0) + 2.0 * l(1.0, 0.0) + l(1.0, 1.0) - l(-1.0, -1.0) - 2.0 * l(-1.0, 0.0) - l(-1.0, 1.0);
    let gy = l(-1.0, 1.0) + 2.0 * l(0.0, 1.0) + l(1.0, 1.0) - l(-1.0, -1.0) - 2.0 * l(0.0, -1.0) - l(1.0, -1.0);
    (gx * gx + gy * gy).sqrt()
}

fn median(img: &RgbaImage, r: i32, threshold: f32) -> RgbaImage {
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
            let med = *vals[c].select_nth_unstable(m).1;
            if (px[c] as f32 - med as f32).abs() / 255.0 >= threshold {
                px[c] = med;
            }
        }
    });
    out
}

fn box_blur(img: &RgbaImage, rx: usize, ry: usize) -> RgbaImage {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut buf: Vec<[f32; 4]> = img.pixels().map(|p| p.0.map(|v| v as f32)).collect();
    let pass = |src: &[[f32; 4]], horizontal: bool, r: usize| -> Vec<[f32; 4]> {
        if r == 0 {
            return src.to_vec();
        }
        let (lines, len) = if horizontal { (h, w) } else { (w, h) };
        let mut out = vec![[0f32; 4]; src.len()];
        for line in 0..lines {
            let idx = |k: usize| if horizontal { line * w + k } else { k * w + line };
            let mut acc = [0f32; 4];
            let mut count = 0f32;
            for k in 0..=r.min(len - 1) {
                add(&mut acc, src[idx(k)]);
                count += 1.0;
            }
            for k in 0..len {
                out[idx(k)] = acc.map(|v| v / count);
                if k + r + 1 < len {
                    add(&mut acc, src[idx(k + r + 1)]);
                    count += 1.0;
                }
                if k >= r {
                    let q = src[idx(k - r)];
                    for c in 0..4 {
                        acc[c] -= q[c];
                    }
                    count -= 1.0;
                }
            }
        }
        out
    };
    buf = pass(&buf, true, rx);
    buf = pass(&buf, false, ry);
    let mut out = img.clone();
    for (p, v) in out.pixels_mut().zip(buf) {
        p.0 = v.map(|x| x.round().clamp(0.0, 255.0) as u8);
    }
    out
}

fn morph(img: &RgbaImage, r: usize, max: bool) -> RgbaImage {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let pick = |a: u8, b: u8| if max { a.max(b) } else { a.min(b) };
    let pass = |src: &RgbaImage, horizontal: bool| -> RgbaImage {
        let mut out = src.clone();
        for y in 0..h {
            for x in 0..w {
                let mut v = src.get_pixel(x as u32, y as u32).0;
                for d in 1..=r {
                    for s in [-(d as i64), d as i64] {
                        let (xx, yy) = if horizontal { (x as i64 + s, y as i64) } else { (x as i64, y as i64 + s) };
                        if xx >= 0 && yy >= 0 && (xx as usize) < w && (yy as usize) < h {
                            let q = src.get_pixel(xx as u32, yy as u32).0;
                            for c in 0..3 {
                                v[c] = pick(v[c], q[c]);
                            }
                        }
                    }
                }
                out.put_pixel(x as u32, y as u32, Rgba(v));
            }
        }
        out
    };
    pass(&pass(img, true), false)
}

fn kuwahara(img: &RgbaImage, r: i32) -> RgbaImage {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut integ = vec![[0f64; 5]; (w + 1) * (h + 1)];
    for y in 0..h {
        let mut row = [0f64; 5];
        for x in 0..w {
            let p = img.get_pixel(x as u32, y as u32).0;
            let c = [p[0] as f64 / 255.0, p[1] as f64 / 255.0, p[2] as f64 / 255.0];
            let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            let v = [c[0], c[1], c[2], l, l * l];
            for k in 0..5 {
                row[k] += v[k];
                integ[(y + 1) * (w + 1) + x + 1][k] = integ[y * (w + 1) + x + 1][k] + row[k];
            }
        }
    }
    let sum = |x0: i32, y0: i32, x1: i32, y1: i32| -> ([f64; 5], f64) {
        let (x0, y0) = (x0.clamp(0, w as i32) as usize, y0.clamp(0, h as i32) as usize);
        let (x1, y1) = (x1.clamp(0, w as i32) as usize, y1.clamp(0, h as i32) as usize);
        let n = ((x1 - x0) * (y1 - y0)).max(1) as f64;
        let g = |x: usize, y: usize| integ[y * (w + 1) + x];
        let (a, b, c, d) = (g(x1, y1), g(x0, y1), g(x1, y0), g(x0, y0));
        (std::array::from_fn(|k| a[k] - b[k] - c[k] + d[k]), n)
    };
    let mut out = img.clone();
    let ww = w as u32;
    out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
        let (x, y) = ((i as u32 % ww) as i32, (i as u32 / ww) as i32);
        let mut best = (f64::MAX, [0f64; 3]);
        for (x0, y0, x1, y1) in [(x - r, y - r, x + 1, y + 1), (x, y - r, x + r + 1, y + 1), (x - r, y, x + 1, y + r + 1), (x, y, x + r + 1, y + r + 1)] {
            let (s, n) = sum(x0, y0, x1, y1);
            let mean_l = s[3] / n;
            let var = s[4] / n - mean_l * mean_l;
            if var < best.0 {
                best = (var, [s[0] / n, s[1] / n, s[2] / n]);
            }
        }
        for k in 0..3 {
            px[k] = (best.1[k].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    });
    out
}

fn fbm(x: f32, y: f32, oct: u32) -> f32 {
    let noise = |x: f32, y: f32, k: u32| {
        let (xi, yi) = (x.floor(), y.floor());
        let (fx, fy) = (smooth(x - xi), smooth(y - yi));
        let v = |dx: f32, dy: f32| hash((xi + dx + 10000.0) as u32, (yi + dy + 10000.0) as u32, k + 20);
        let a = v(0.0, 0.0) + (v(1.0, 0.0) - v(0.0, 0.0)) * fx;
        let b = v(0.0, 1.0) + (v(1.0, 1.0) - v(0.0, 1.0)) * fx;
        a + (b - a) * fy
    };
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for k in 0..oct {
        sum += noise(x * freq, y * freq, k) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn rgb(p: [u8; 4]) -> [f32; 3] {
    [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0]
}

fn rgb4(p: [f32; 4]) -> [f32; 3] {
    [p[0], p[1], p[2]]
}

pub fn luma(c: [f32; 3]) -> f32 {
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

pub fn hash(x: u32, y: u32, k: u32) -> f32 {
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

fn map_xy(img: &RgbaImage, f: impl Fn(f32, f32, [f32; 3]) -> [f32; 3] + Sync) -> RgbaImage {
    let w = img.width();
    let mut out = img.clone();
    out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
        let c = f((i as u32 % w) as f32 + 0.5, (i as u32 / w) as f32 + 0.5, [px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0]);
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

fn mix_by(a: &RgbaImage, b: &RgbaImage, f: impl Fn(u32, u32) -> f32 + Sync) -> RgbaImage {
    let w = a.width();
    let mut out = a.clone();
    out.par_chunks_mut(4).zip(b.par_chunks(4)).enumerate().for_each(|(i, (px, q))| {
        let t = f(i as u32 % w, i as u32 / w).clamp(0.0, 1.0);
        for k in 0..4 {
            px[k] = (px[k] as f32 + (q[k] as f32 - px[k] as f32) * t).round() as u8;
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

fn sample_i(img: &RgbaImage, f: impl Fn(i32, i32, &dyn Fn(i32, i32) -> [f32; 4]) -> [f32; 4] + Sync) -> RgbaImage {
    let (w, h) = img.dimensions();
    let src = img.as_raw();
    let at = |x: i32, y: i32| {
        let o = (y.clamp(0, h as i32 - 1) as usize * w as usize + x.clamp(0, w as i32 - 1) as usize) * 4;
        [src[o] as f32 / 255.0, src[o + 1] as f32 / 255.0, src[o + 2] as f32 / 255.0, src[o + 3] as f32 / 255.0]
    };
    let mut out = img.clone();
    out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
        let c = f((i as u32 % w) as i32, (i as u32 / w) as i32, &at);
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
        let all: Vec<Kind> = ADJUSTMENTS.iter().chain(QUICK).chain(FILTERS.iter().flat_map(|f| f.1.iter())).copied().collect();
        assert!(all.len() >= 70);
        for k in all {
            let out = k.apply(&img, &k.defaults());
            assert_eq!(out.dimensions(), img.dimensions(), "{k:?}");
            assert_eq!(k.defaults().len(), k.params().len());
            for (pr, d) in k.params().iter().zip(k.defaults()) {
                assert!(d >= pr.min && d <= pr.max, "{k:?} {}", pr.label);
            }
            for (_, vals) in k.presets() {
                assert_eq!(vals.len(), k.params().len(), "{k:?}");
            }
            let moves = matches!(
                k,
                Kind::Blur | Kind::BoxBlur | Kind::MotionBlur | Kind::RadialBlur | Kind::SpinBlur | Kind::TiltShift | Kind::Pixelate | Kind::Crystallize | Kind::ColorHalftone | Kind::Unsharp | Kind::Glow | Kind::Chromatic | Kind::Wave | Kind::Ripple | Kind::Twirl | Kind::Pinch | Kind::Spherize | Kind::Polar | Kind::Shear | Kind::Diffuse | Kind::Wind | Kind::Tiles | Kind::HighPass | Kind::Offset | Kind::Emboss | Kind::SurfaceBlur | Kind::Cartoon
            );
            if !moves {
                assert_eq!(out.get_pixel(1, 5).0[3], 0, "{k:?}");
            }
        }
        let same = Kind::BrightnessContrast.apply(&img, &[0.0, 0.0]);
        assert_eq!(same.get_pixel(20, 20), img.get_pixel(20, 20));
        let inv = Kind::Invert.apply(&img, &[]);
        assert_eq!(inv.get_pixel(10, 10).0[2], 135);
        let th = Kind::Threshold.apply(&img, &[128.0]);
        assert!(th.pixels().all(|p| p.0[0] == 0 || p.0[0] == 255));
        let id = Kind::Curves.apply(&img, &Kind::Curves.defaults());
        assert!(id.pixels().zip(img.pixels()).all(|(a, b)| a.0.iter().zip(b.0).all(|(x, y)| x.abs_diff(y) <= 1)));
        let mixer = Kind::ChannelMixer.apply(&img, &Kind::ChannelMixer.defaults());
        assert_eq!(mixer.get_pixel(20, 20), img.get_pixel(20, 20));
        let bw = Kind::BlackWhiteMix.apply(&RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255])), &Kind::BlackWhiteMix.defaults());
        assert_eq!(bw.get_pixel(0, 0).0[0], 102);
    }
}
