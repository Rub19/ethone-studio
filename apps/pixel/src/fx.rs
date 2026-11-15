use crate::doc::{hsl_to_rgb, rgb_to_hsl};
use crate::filters::{Kind, Param, box_blur, c, fbm, gradient_at, hash, kuwahara, luma, map, map_xy, morph, on, p, rgb, sample, zip};
use image::{Rgba, RgbaImage, imageops};
use rayon::prelude::*;
use std::f32::consts::TAU;

pub const GROUPS: &[(&str, &[Kind])] = &[
    ("Nouveautés 2027", &[Kind::Relight, Kind::Dehaze, Kind::JpegClean, Kind::Looks, Kind::GlowPro, Kind::Blemish]),
    ("Couleur créative", &[Kind::SplitTone, Kind::ColorGrade, Kind::Duotone, Kind::HslMixer, Kind::Kelvin, Kind::CrossProcess, Kind::BleachBypass, Kind::Infrared, Kind::Thermal, Kind::NightVision, Kind::Lomo, Kind::Faded]),
    ("Photo", &[Kind::Clarity, Kind::Orton, Kind::Hdr, Kind::LensBlur, Kind::LensCorrect, Kind::SmartSharpen, Kind::SunRays, Kind::LightLeak, Kind::Bokeh]),
    ("Rétro et glitch", &[Kind::Glitch, Kind::Vhs, Kind::Scanlines, Kind::PixelSort, Kind::Dither]),
    ("Atmosphère", &[Kind::Rain, Kind::Snow, Kind::Fog, Kind::Stars]),
    ("Peinture et graphisme", &[Kind::Watercolor, Kind::ColorPencil, Kind::Pointillism, Kind::StainedGlass, Kind::NeonEdges, Kind::Kaleidoscope, Kind::Mirror, Kind::Fisheye, Kind::FrostedGlass, Kind::Turbulence]),
];

pub const LOOK_NAMES: &[&str] = &[
    "Aucun", "Sarcelle et orange", "Blockbuster", "Film chaud", "Film froid", "Kodak portrait", "Fuji vert", "Polaroïd", "Instantané délavé", "Années 70",
    "Années 80", "Années 90", "Vintage crème", "Sépia doux", "Cyanotype", "Noir et blanc neutre", "N&B contrasté", "N&B doux", "N&B sélénium", "N&B chaud",
    "Nuit américaine", "Heure dorée", "Heure bleue", "Été brûlant", "Automne", "Hiver glacé", "Printemps pastel", "Matrice", "Néon nocturne", "Cyberpunk",
    "Dune", "Mars", "Arctique", "Tropical", "Forêt profonde", "Océan", "Désert", "Brume matinale", "Mariage lumineux", "Mode éditoriale",
    "Café", "Chocolat", "Pêche", "Rose poudré", "Lavande", "Menthe", "Citron", "Ambre", "Bronze", "Argentique 400",
    "Diapositive", "Négatif couleur", "Super 8", "Western", "Horreur", "Rêve", "Bande dessinée", "Pop vif", "Mat cinéma", "Teal profond",
];

const LOOK_VALS: &[([f32; 9], bool)] = &[
    ([0.0; 9], false),
    ([0.1, 0.0, 0.15, 0.15, 0.03, 190.0, 0.45, 30.0, 0.4], false),
    ([0.05, 0.0, 0.1, 0.25, 0.0, 200.0, 0.35, 35.0, 0.3], false),
    ([0.35, 0.05, 0.0, 0.08, 0.06, 30.0, 0.15, 40.0, 0.2], false),
    ([-0.35, 0.0, -0.05, 0.08, 0.05, 210.0, 0.25, 200.0, 0.1], false),
    ([0.15, 0.05, -0.05, 0.05, 0.04, 20.0, 0.1, 35.0, 0.15], false),
    ([-0.05, -0.15, 0.05, 0.1, 0.03, 160.0, 0.2, 90.0, 0.1], false),
    ([0.2, 0.0, -0.2, -0.05, 0.12, 190.0, 0.2, 50.0, 0.25], false),
    ([0.1, 0.0, -0.3, -0.1, 0.18, 200.0, 0.1, 40.0, 0.15], false),
    ([0.35, 0.1, -0.1, 0.05, 0.1, 30.0, 0.25, 45.0, 0.3], false),
    ([-0.1, 0.15, 0.25, 0.15, 0.05, 280.0, 0.35, 320.0, 0.3], false),
    ([0.05, -0.05, -0.05, 0.1, 0.08, 170.0, 0.2, 60.0, 0.15], false),
    ([0.25, 0.0, -0.35, -0.05, 0.14, 40.0, 0.15, 45.0, 0.2], false),
    ([0.2, 0.0, 0.0, 0.0, 0.05, 30.0, 0.35, 40.0, 0.4], true),
    ([0.0, 0.0, 0.0, 0.05, 0.04, 215.0, 0.45, 200.0, 0.3], true),
    ([0.0; 9], true),
    ([0.0, 0.0, 0.0, 0.35, 0.0, 0.0, 0.0, 0.0, 0.0], true),
    ([0.0, 0.0, 0.0, -0.15, 0.1, 0.0, 0.0, 0.0, 0.0], true),
    ([0.0, 0.0, 0.0, 0.1, 0.02, 280.0, 0.2, 30.0, 0.1], true),
    ([0.0, 0.0, 0.0, 0.08, 0.04, 30.0, 0.2, 45.0, 0.25], true),
    ([-0.6, 0.0, -0.3, 0.15, 0.0, 220.0, 0.4, 210.0, 0.2], false),
    ([0.5, 0.05, 0.15, 0.08, 0.02, 25.0, 0.2, 40.0, 0.45], false),
    ([-0.45, 0.05, 0.05, 0.05, 0.03, 230.0, 0.35, 210.0, 0.2], false),
    ([0.4, 0.0, 0.3, 0.2, 0.0, 15.0, 0.25, 45.0, 0.35], false),
    ([0.3, 0.05, 0.05, 0.1, 0.04, 20.0, 0.3, 35.0, 0.35], false),
    ([-0.5, 0.0, -0.25, 0.05, 0.06, 210.0, 0.3, 190.0, 0.25], false),
    ([0.05, 0.1, -0.15, -0.1, 0.12, 300.0, 0.15, 100.0, 0.15], false),
    ([-0.1, -0.35, 0.0, 0.2, 0.02, 130.0, 0.4, 100.0, 0.3], false),
    ([-0.2, 0.25, 0.35, 0.2, 0.0, 280.0, 0.5, 320.0, 0.45], false),
    ([-0.3, 0.3, 0.3, 0.25, 0.02, 190.0, 0.55, 310.0, 0.5], false),
    ([0.45, 0.05, -0.1, 0.15, 0.04, 25.0, 0.3, 40.0, 0.4], false),
    ([0.5, 0.1, 0.1, 0.2, 0.02, 10.0, 0.45, 25.0, 0.4], false),
    ([-0.55, 0.0, -0.35, 0.1, 0.06, 200.0, 0.3, 190.0, 0.2], false),
    ([0.15, -0.1, 0.35, 0.15, 0.0, 170.0, 0.35, 55.0, 0.3], false),
    ([-0.1, -0.25, 0.0, 0.15, 0.03, 150.0, 0.4, 80.0, 0.2], false),
    ([-0.35, 0.0, 0.1, 0.1, 0.02, 200.0, 0.45, 185.0, 0.3], false),
    ([0.45, 0.05, -0.15, 0.05, 0.06, 30.0, 0.25, 40.0, 0.35], false),
    ([-0.05, 0.0, -0.25, -0.2, 0.2, 210.0, 0.1, 40.0, 0.1], false),
    ([0.1, 0.05, -0.1, -0.1, 0.12, 30.0, 0.1, 40.0, 0.12], false),
    ([0.0, 0.0, -0.2, 0.25, 0.02, 220.0, 0.15, 30.0, 0.15], false),
    ([0.35, 0.05, -0.2, 0.1, 0.06, 25.0, 0.35, 35.0, 0.3], false),
    ([0.4, 0.1, -0.1, 0.18, 0.03, 15.0, 0.4, 30.0, 0.3], false),
    ([0.25, 0.15, -0.05, -0.05, 0.08, 10.0, 0.2, 25.0, 0.3], false),
    ([0.05, 0.25, -0.1, -0.05, 0.1, 330.0, 0.2, 350.0, 0.25], false),
    ([-0.05, 0.25, -0.1, -0.05, 0.08, 270.0, 0.3, 290.0, 0.25], false),
    ([-0.15, -0.2, -0.1, 0.0, 0.08, 160.0, 0.3, 140.0, 0.2], false),
    ([0.2, -0.15, 0.05, 0.05, 0.04, 70.0, 0.2, 55.0, 0.3], false),
    ([0.4, 0.05, 0.0, 0.1, 0.04, 35.0, 0.35, 40.0, 0.4], false),
    ([0.3, 0.0, -0.25, 0.2, 0.03, 25.0, 0.4, 35.0, 0.35], false),
    ([0.05, 0.0, -0.05, 0.1, 0.05, 180.0, 0.15, 40.0, 0.15], false),
    ([0.0, -0.05, 0.25, 0.3, 0.0, 220.0, 0.2, 50.0, 0.15], false),
    ([0.15, 0.1, -0.2, -0.1, 0.15, 180.0, 0.25, 30.0, 0.2], false),
    ([0.4, 0.05, -0.05, 0.15, 0.1, 25.0, 0.3, 45.0, 0.35], false),
    ([0.5, 0.05, -0.3, 0.2, 0.06, 30.0, 0.4, 40.0, 0.45], false),
    ([-0.2, -0.1, -0.5, 0.3, 0.02, 120.0, 0.3, 200.0, 0.2], false),
    ([0.1, 0.25, -0.15, -0.2, 0.15, 290.0, 0.25, 330.0, 0.3], false),
    ([0.0, 0.0, 0.5, 0.35, 0.0, 230.0, 0.2, 50.0, 0.2], false),
    ([0.05, 0.05, 0.6, 0.25, 0.0, 300.0, 0.25, 60.0, 0.25], false),
    ([0.05, 0.0, -0.15, -0.05, 0.15, 210.0, 0.15, 35.0, 0.15], false),
    ([-0.2, 0.0, 0.0, 0.2, 0.02, 185.0, 0.6, 25.0, 0.3], false),
];

const HSL_LABELS: [Param; 24] = [
    p("Rouges · teinte", -100.0, 100.0, 0.0, ""), p("Rouges · saturation", -100.0, 100.0, 0.0, ""), p("Rouges · luminance", -100.0, 100.0, 0.0, ""),
    p("Orangés · teinte", -100.0, 100.0, 0.0, ""), p("Orangés · saturation", -100.0, 100.0, 0.0, ""), p("Orangés · luminance", -100.0, 100.0, 0.0, ""),
    p("Jaunes · teinte", -100.0, 100.0, 0.0, ""), p("Jaunes · saturation", -100.0, 100.0, 0.0, ""), p("Jaunes · luminance", -100.0, 100.0, 0.0, ""),
    p("Verts · teinte", -100.0, 100.0, 0.0, ""), p("Verts · saturation", -100.0, 100.0, 0.0, ""), p("Verts · luminance", -100.0, 100.0, 0.0, ""),
    p("Cyans · teinte", -100.0, 100.0, 0.0, ""), p("Cyans · saturation", -100.0, 100.0, 0.0, ""), p("Cyans · luminance", -100.0, 100.0, 0.0, ""),
    p("Bleus · teinte", -100.0, 100.0, 0.0, ""), p("Bleus · saturation", -100.0, 100.0, 0.0, ""), p("Bleus · luminance", -100.0, 100.0, 0.0, ""),
    p("Violets · teinte", -100.0, 100.0, 0.0, ""), p("Violets · saturation", -100.0, 100.0, 0.0, ""), p("Violets · luminance", -100.0, 100.0, 0.0, ""),
    p("Magentas · teinte", -100.0, 100.0, 0.0, ""), p("Magentas · saturation", -100.0, 100.0, 0.0, ""), p("Magentas · luminance", -100.0, 100.0, 0.0, ""),
];
const HSL_CENTERS: [f32; 8] = [0.0, 30.0, 60.0, 120.0, 180.0, 230.0, 275.0, 320.0];

const DITHER_PALETTES: &[&str] = &["Noir et blanc", "Écran vert", "CGA", "Ambre", "Console 16", "Sépia 4", "Pastel 8", "Néon 6"];
const PALETTES: [&[[u8; 3]]; 8] = [
    &[[0, 0, 0], [255, 255, 255]],
    &[[15, 56, 15], [48, 98, 48], [139, 172, 15], [155, 188, 15]],
    &[[0, 0, 0], [85, 255, 255], [255, 85, 255], [255, 255, 255]],
    &[[20, 10, 0], [120, 70, 0], [200, 130, 0], [255, 190, 40]],
    &[[0, 0, 0], [29, 43, 83], [126, 37, 83], [0, 135, 81], [171, 82, 54], [95, 87, 79], [194, 195, 199], [255, 241, 232], [255, 0, 77], [255, 163, 0], [255, 236, 39], [0, 228, 54], [41, 173, 255], [131, 118, 156], [255, 119, 168], [255, 204, 170]],
    &[[40, 26, 13], [112, 76, 44], [190, 150, 100], [245, 228, 196]],
    &[[255, 209, 220], [255, 236, 179], [200, 240, 200], [180, 220, 255], [220, 200, 255], [255, 255, 255], [120, 120, 140], [60, 60, 80]],
    &[[10, 10, 20], [255, 0, 128], [0, 255, 200], [255, 230, 0], [120, 0, 255], [255, 255, 255]],
];
const THERMAL: [&[[u8; 3]]; 4] = [
    &[[0, 0, 0], [40, 0, 100], [180, 0, 140], [255, 90, 0], [255, 200, 0], [255, 255, 230]],
    &[[0, 0, 128], [0, 128, 255], [0, 255, 128], [255, 255, 0], [255, 128, 0], [255, 0, 0]],
    &[[0, 0, 0], [255, 255, 255]],
    &[[255, 255, 255], [0, 0, 0]],
];

pub fn title(k: Kind) -> &'static str {
    match k {
        Kind::Relight => "Rééclairer (Relight)",
        Kind::Dehaze => "Supprimer la brume et les effets d'ambiance",
        Kind::JpegClean => "Débruitage JPEG",
        Kind::Looks => "Looks (3 LUT empilables)",
        Kind::GlowPro => "Lueur améliorée",
        Kind::Blemish => "Suppression des imperfections",
        Kind::SplitTone => "Virage partiel",
        Kind::ColorGrade => "Étalonnage 3 voies",
        Kind::Duotone => "Bichromie",
        Kind::HslMixer => "Mélangeur TSL (8 couleurs)",
        Kind::Kelvin => "Balance des blancs (Kelvin)",
        Kind::CrossProcess => "Traitement croisé",
        Kind::BleachBypass => "Sans blanchiment",
        Kind::Infrared => "Infrarouge couleur",
        Kind::Thermal => "Caméra thermique",
        Kind::NightVision => "Vision nocturne",
        Kind::Lomo => "Lomo",
        Kind::Faded => "Film délavé",
        Kind::Clarity => "Clarté et texture",
        Kind::Orton => "Effet Orton",
        Kind::Hdr => "Virage HDR",
        Kind::LensBlur => "Flou de l'objectif (bokeh)",
        Kind::LensCorrect => "Correction de l'objectif",
        Kind::SmartSharpen => "Netteté optimisée",
        Kind::SunRays => "Rayons de soleil",
        Kind::LightLeak => "Fuite de lumière",
        Kind::Bokeh => "Lumières bokeh",
        Kind::Glitch => "Glitch numérique",
        Kind::Vhs => "Cassette VHS",
        Kind::Scanlines => "Écran cathodique",
        Kind::PixelSort => "Tri de pixels",
        Kind::Dither => "Tramage rétro",
        Kind::Rain => "Pluie",
        Kind::Snow => "Neige",
        Kind::Fog => "Brouillard",
        Kind::Stars => "Ciel étoilé",
        Kind::Watercolor => "Aquarelle",
        Kind::ColorPencil => "Crayons de couleur",
        Kind::Pointillism => "Pointillisme",
        Kind::StainedGlass => "Vitrail",
        Kind::NeonEdges => "Contours néon",
        Kind::Kaleidoscope => "Kaléidoscope",
        Kind::Mirror => "Miroir",
        Kind::Fisheye => "Fisheye",
        Kind::FrostedGlass => "Verre dépoli",
        Kind::Turbulence => "Turbulence",
        _ => "",
    }
}

pub fn params(k: Kind) -> &'static [Param] {
    match k {
        Kind::Relight => const {
            &[
                p("Température 1", 2000.0, 10000.0, 4500.0, " K"),
                p("Gauche ↔ droite 1", -100.0, 100.0, -50.0, ""),
                p("Bas ↔ haut 1", -100.0, 100.0, 40.0, ""),
                p("Proche ↔ loin", 0.0, 100.0, 40.0, ""),
                p("Intensité 1", 0.0, 200.0, 80.0, " %"),
                p("Diffusion", 0.0, 100.0, 50.0, " %"),
                on("Deuxième lumière", 0.0),
                p("Température 2", 2000.0, 10000.0, 7500.0, " K"),
                p("Gauche ↔ droite 2", -100.0, 100.0, 60.0, ""),
                p("Bas ↔ haut 2", -100.0, 100.0, 0.0, ""),
                p("Intensité 2", 0.0, 200.0, 50.0, " %"),
                p("Ambiance", 0.0, 100.0, 55.0, " %"),
                p("Relief", 0.0, 10.0, 3.0, ""),
            ]
        },
        Kind::Dehaze => const { &[p("Quantité", -100.0, 100.0, 50.0, " %"), p("Rayon", 2.0, 60.0, 15.0, " px"), on("Protéger le ciel", 1.0), p("Saturation", -50.0, 50.0, 10.0, "")] },
        Kind::JpegClean => const { &[p("Artefacts de blocs", 0.0, 100.0, 60.0, " %"), p("Bruit de couleur", 0.0, 100.0, 50.0, " %"), p("Bruit de luminance", 0.0, 100.0, 30.0, " %"), p("Détails préservés", 0.0, 100.0, 50.0, " %")] },
        Kind::Looks => const { &[c("Look 1", LOOK_NAMES, 1.0), p("Intensité 1", 0.0, 100.0, 100.0, " %"), c("Look 2", LOOK_NAMES, 0.0), p("Intensité 2", 0.0, 100.0, 100.0, " %"), c("Look 3", LOOK_NAMES, 0.0), p("Intensité 3", 0.0, 100.0, 100.0, " %")] },
        Kind::GlowPro => const {
            &[
                p("Rayon", 2.0, 120.0, 24.0, " px"),
                p("Intensité", 0.0, 200.0, 70.0, " %"),
                p("Seuil", 0.0, 95.0, 60.0, " %"),
                p("Teinte", 0.0, 360.0, 40.0, "°"),
                p("Teinte de la lueur", 0.0, 100.0, 30.0, " %"),
                c("Mode", &["Écran", "Addition", "Lumière douce", "Incrustation"], 0.0),
                p("Étirement anamorphique", 0.0, 100.0, 0.0, " %"),
                on("Préserver les couleurs", 0.0),
            ]
        },
        Kind::Blemish => const { &[p("Taille max", 1.0, 30.0, 6.0, " px"), p("Sensibilité", 0.0, 100.0, 55.0, " %"), p("Lissage de la peau", 0.0, 100.0, 0.0, " %"), on("Seulement la peau", 1.0)] },
        Kind::SplitTone => const { &[p("Teinte des foncés", 0.0, 360.0, 210.0, "°"), p("Saturation des foncés", 0.0, 100.0, 30.0, " %"), p("Teinte des clairs", 0.0, 360.0, 40.0, "°"), p("Saturation des clairs", 0.0, 100.0, 30.0, " %"), p("Équilibre", -100.0, 100.0, 0.0, "")] },
        Kind::ColorGrade => const {
            &[
                p("Foncés · teinte", 0.0, 360.0, 210.0, "°"),
                p("Foncés · saturation", 0.0, 100.0, 0.0, " %"),
                p("Foncés · luminance", -100.0, 100.0, 0.0, ""),
                p("Moyens · teinte", 0.0, 360.0, 30.0, "°"),
                p("Moyens · saturation", 0.0, 100.0, 0.0, " %"),
                p("Moyens · luminance", -100.0, 100.0, 0.0, ""),
                p("Clairs · teinte", 0.0, 360.0, 45.0, "°"),
                p("Clairs · saturation", 0.0, 100.0, 0.0, " %"),
                p("Clairs · luminance", -100.0, 100.0, 0.0, ""),
                p("Fusion", 0.0, 100.0, 50.0, " %"),
                p("Équilibre", -100.0, 100.0, 0.0, ""),
            ]
        },
        Kind::Duotone => const { &[p("Teinte foncée", 0.0, 360.0, 230.0, "°"), p("Teinte claire", 0.0, 360.0, 45.0, "°"), p("Saturation", 0.0, 100.0, 70.0, " %"), p("Contraste", -50.0, 50.0, 0.0, "")] },
        Kind::HslMixer => &HSL_LABELS,
        Kind::Kelvin => const { &[p("Température de la scène", 2000.0, 12000.0, 6500.0, " K"), p("Teinte", -100.0, 100.0, 0.0, ""), p("Intensité", 0.0, 100.0, 100.0, " %")] },
        Kind::CrossProcess => const { &[p("Intensité", 0.0, 100.0, 70.0, " %"), c("Type", &["Diapo développée en négatif", "Négatif développé en diapo"], 0.0)] },
        Kind::BleachBypass => const { &[p("Intensité", 0.0, 100.0, 60.0, " %"), p("Contraste", 0.0, 100.0, 40.0, "")] },
        Kind::Infrared => const { &[p("Intensité", 0.0, 100.0, 80.0, " %"), on("Échanger rouge et bleu", 1.0), p("Lueur", 0.0, 100.0, 30.0, " %")] },
        Kind::Thermal => const { &[c("Palette", &["Fer", "Arc-en-ciel", "Blanc chaud", "Noir chaud"], 0.0), p("Contraste", -50.0, 100.0, 20.0, "")] },
        Kind::NightVision => const { &[p("Gain", 50.0, 300.0, 180.0, " %"), p("Bruit", 0.0, 100.0, 30.0, " %"), p("Vignettage", 0.0, 100.0, 60.0, " %"), on("Lignes de balayage", 1.0)] },
        Kind::Lomo => const { &[p("Saturation", 0.0, 100.0, 40.0, " %"), p("Vignettage", 0.0, 100.0, 70.0, " %"), p("Contraste", 0.0, 100.0, 40.0, ""), p("Dominante", -50.0, 50.0, 10.0, "")] },
        Kind::Faded => const { &[p("Délavage", 0.0, 100.0, 40.0, " %"), p("Grain", 0.0, 100.0, 20.0, " %"), p("Chaleur", -50.0, 50.0, 15.0, ""), p("Désaturation", 0.0, 100.0, 25.0, " %")] },
        Kind::Clarity => const { &[p("Clarté", -100.0, 100.0, 30.0, ""), p("Texture", -100.0, 100.0, 0.0, ""), p("Rayon", 5.0, 80.0, 30.0, " px")] },
        Kind::Orton => const { &[p("Flou", 2.0, 60.0, 20.0, " px"), p("Intensité", 0.0, 100.0, 50.0, " %"), p("Luminosité", 0.0, 100.0, 30.0, " %")] },
        Kind::Hdr => const { &[p("Force", 0.0, 100.0, 60.0, " %"), p("Rayon", 5.0, 100.0, 40.0, " px"), p("Détail", 0.0, 100.0, 50.0, " %"), p("Saturation", -50.0, 100.0, 20.0, ""), p("Gamma", 0.5, 2.0, 1.0, "")] },
        Kind::LensBlur => const { &[p("Rayon", 1.0, 40.0, 10.0, " px"), p("Éclat des reflets", 0.0, 100.0, 40.0, " %"), p("Seuil des reflets", 50.0, 99.0, 80.0, " %"), c("Forme", &["Disque", "Hexagone", "Anneau"], 0.0), p("Zone nette (hauteur)", 0.0, 100.0, 50.0, " %"), p("Taille de la zone nette", 0.0, 100.0, 0.0, " %")] },
        Kind::LensCorrect => const { &[p("Distorsion", -100.0, 100.0, 0.0, ""), p("Vignettage", -100.0, 100.0, 0.0, ""), p("Aberration", -10.0, 10.0, 0.0, " px"), p("Échelle", 50.0, 150.0, 100.0, " %")] },
        Kind::SmartSharpen => const { &[p("Gain", 0.0, 500.0, 150.0, " %"), p("Rayon", 0.5, 10.0, 1.2, " px"), p("Réduction du bruit", 0.0, 100.0, 10.0, " %"), p("Atténuer dans les foncés", 0.0, 100.0, 0.0, " %"), p("Atténuer dans les clairs", 0.0, 100.0, 0.0, " %")] },
        Kind::SunRays => const { &[p("Source X", 0.0, 100.0, 70.0, " %"), p("Source Y", 0.0, 100.0, 20.0, " %"), p("Longueur", 10.0, 100.0, 60.0, " %"), p("Intensité", 0.0, 200.0, 80.0, " %"), p("Seuil", 0.0, 95.0, 60.0, " %"), p("Chaleur", 0.0, 100.0, 50.0, " %")] },
        Kind::LightLeak => const { &[c("Position", &["Gauche", "Droite", "Haut", "Coin"], 0.0), p("Teinte", 0.0, 360.0, 20.0, "°"), p("Intensité", 0.0, 100.0, 60.0, " %"), p("Taille", 10.0, 100.0, 50.0, " %")] },
        Kind::Bokeh => const { &[p("Nombre", 5.0, 200.0, 40.0, ""), p("Taille", 4.0, 80.0, 24.0, " px"), p("Teinte", 0.0, 360.0, 40.0, "°"), p("Variation de teinte", 0.0, 100.0, 50.0, " %"), p("Intensité", 0.0, 100.0, 60.0, " %"), p("Graine", 0.0, 999.0, 7.0, "")] },
        Kind::Glitch => const { &[p("Décalage RVB", 0.0, 40.0, 8.0, " px"), p("Tranches", 1.0, 60.0, 14.0, ""), p("Déplacement", 0.0, 200.0, 40.0, " px"), p("Blocs corrompus", 0.0, 100.0, 20.0, " %"), p("Graine", 0.0, 999.0, 3.0, "")] },
        Kind::Vhs => const { &[p("Flou horizontal", 0.0, 10.0, 3.0, " px"), p("Décalage chroma", 0.0, 12.0, 4.0, " px"), p("Bruit", 0.0, 100.0, 25.0, " %"), p("Parasites", 0.0, 100.0, 35.0, " %"), p("Délavage", 0.0, 100.0, 30.0, " %")] },
        Kind::Scanlines => const { &[p("Espacement", 2.0, 12.0, 3.0, " px"), p("Intensité", 0.0, 100.0, 50.0, " %"), p("Masque RVB", 0.0, 100.0, 40.0, " %"), p("Courbure", 0.0, 100.0, 20.0, " %")] },
        Kind::PixelSort => const { &[c("Sens", &["Horizontal", "Vertical"], 0.0), p("Seuil bas", 0.0, 100.0, 25.0, " %"), p("Seuil haut", 0.0, 100.0, 80.0, " %"), on("Ordre inversé", 0.0)] },
        Kind::Dither => const { &[c("Palette", DITHER_PALETTES, 1.0), c("Méthode", &["Bayer 4×4", "Bayer 8×8", "Floyd-Steinberg", "Atkinson"], 0.0), p("Taille des pixels", 1.0, 8.0, 2.0, " px")] },
        Kind::Rain => const { &[p("Densité", 0.0, 100.0, 40.0, " %"), p("Longueur", 5.0, 80.0, 25.0, " px"), p("Angle", -45.0, 45.0, -10.0, "°"), p("Opacité", 0.0, 100.0, 40.0, " %"), p("Graine", 0.0, 999.0, 1.0, "")] },
        Kind::Snow => const { &[p("Densité", 0.0, 100.0, 35.0, " %"), p("Taille", 1.0, 8.0, 3.0, " px"), p("Douceur", 0.0, 100.0, 30.0, " %"), p("Opacité", 0.0, 100.0, 80.0, " %"), p("Graine", 0.0, 999.0, 1.0, "")] },
        Kind::Fog => const { &[p("Densité", 0.0, 100.0, 45.0, " %"), p("Échelle", 20.0, 600.0, 180.0, " px"), p("Hauteur", 5.0, 100.0, 60.0, " %"), p("Teinte", 0.0, 360.0, 210.0, "°"), p("Saturation", 0.0, 100.0, 10.0, " %")] },
        Kind::Stars => const { &[p("Densité", 0.0, 100.0, 30.0, " %"), p("Éclat", 0.0, 100.0, 70.0, " %"), p("Zones sombres jusqu'à", 0.0, 100.0, 60.0, " %"), p("Graine", 0.0, 999.0, 1.0, "")] },
        Kind::Watercolor => const { &[p("Lissage", 2.0, 10.0, 4.0, " px"), p("Bords", 0.0, 100.0, 40.0, " %"), p("Texture du papier", 0.0, 100.0, 40.0, " %"), p("Saturation", -50.0, 50.0, 10.0, "")] },
        Kind::ColorPencil => const { &[p("Épaisseur", 1.0, 10.0, 3.0, " px"), p("Pression", 0.0, 100.0, 60.0, " %"), p("Grain du papier", 0.0, 100.0, 30.0, " %")] },
        Kind::Pointillism => const { &[p("Taille des points", 2.0, 30.0, 6.0, " px"), p("Densité", 10.0, 100.0, 70.0, " %"), p("Fond clair", 0.0, 100.0, 90.0, " %")] },
        Kind::StainedGlass => const { &[p("Taille de cellule", 5.0, 100.0, 22.0, " px"), p("Épaisseur des joints", 1.0, 8.0, 2.0, " px"), p("Luminosité", 0.0, 100.0, 20.0, " %")] },
        Kind::NeonEdges => const { &[p("Épaisseur", 1.0, 6.0, 2.0, " px"), p("Teinte", 0.0, 360.0, 300.0, "°"), on("Arc-en-ciel", 1.0), p("Assombrir le fond", 0.0, 100.0, 85.0, " %")] },
        Kind::Kaleidoscope => const { &[p("Segments", 2.0, 24.0, 6.0, ""), p("Rotation", 0.0, 360.0, 0.0, "°"), p("Centre X", 0.0, 100.0, 50.0, " %"), p("Centre Y", 0.0, 100.0, 50.0, " %")] },
        Kind::Mirror => const { &[c("Axe", &["Gauche → droite", "Droite → gauche", "Haut → bas", "Bas → haut", "Quatre coins"], 0.0)] },
        Kind::Fisheye => const { &[p("Intensité", 0.0, 100.0, 60.0, " %"), p("Zoom", 50.0, 150.0, 100.0, " %")] },
        Kind::FrostedGlass => const { &[p("Dispersion", 1.0, 30.0, 6.0, " px"), on("Adoucir", 1.0)] },
        Kind::Turbulence => const { &[p("Force", 0.0, 100.0, 25.0, " px"), p("Échelle", 10.0, 400.0, 80.0, " px"), p("Détail", 1.0, 6.0, 3.0, ""), p("Graine", 0.0, 999.0, 1.0, "")] },
        _ => &[],
    }
}

pub fn presets(k: Kind) -> &'static [(&'static str, &'static [f32])] {
    match k {
        Kind::Relight => &[
            ("Rembrandt", &[3800.0, -60.0, 55.0, 30.0, 110.0, 40.0, 0.0, 7500.0, 60.0, 0.0, 50.0, 35.0, 4.0]),
            ("Papillon", &[5200.0, 0.0, 80.0, 40.0, 90.0, 60.0, 0.0, 7500.0, 60.0, 0.0, 50.0, 50.0, 3.0]),
            ("Lumière latérale dramatique", &[4200.0, -95.0, 10.0, 20.0, 140.0, 30.0, 0.0, 7500.0, 60.0, 0.0, 50.0, 25.0, 5.0]),
            ("Contre-jour", &[3600.0, 0.0, 95.0, 70.0, 120.0, 70.0, 1.0, 6500.0, 0.0, -60.0, 30.0, 45.0, 3.0]),
            ("Néon rose et bleu", &[2600.0, -80.0, 20.0, 30.0, 120.0, 50.0, 1.0, 10000.0, 80.0, 20.0, 120.0, 30.0, 3.0]),
            ("Heure dorée", &[2900.0, 70.0, 30.0, 60.0, 110.0, 70.0, 0.0, 7500.0, 60.0, 0.0, 50.0, 55.0, 2.0]),
            ("Studio doux", &[5600.0, -40.0, 50.0, 70.0, 80.0, 90.0, 1.0, 5600.0, 60.0, 30.0, 40.0, 65.0, 2.0]),
            ("Clair de lune", &[9500.0, 60.0, 70.0, 50.0, 70.0, 60.0, 0.0, 7500.0, 60.0, 0.0, 50.0, 25.0, 3.0]),
            ("Feu de camp", &[2000.0, 0.0, -80.0, 25.0, 150.0, 40.0, 0.0, 7500.0, 60.0, 0.0, 50.0, 20.0, 4.0]),
            ("Fenêtre nord", &[7000.0, 90.0, 40.0, 60.0, 90.0, 85.0, 0.0, 7500.0, 60.0, 0.0, 50.0, 50.0, 2.0]),
        ],
        Kind::Dehaze => &[("Léger", &[25.0, 15.0, 1.0, 5.0]), ("Fort", &[75.0, 20.0, 1.0, 15.0]), ("Paysage lointain", &[60.0, 30.0, 1.0, 10.0]), ("Ville polluée", &[85.0, 12.0, 0.0, 20.0]), ("Ajouter de la brume", &[-50.0, 15.0, 0.0, -10.0])],
        Kind::JpegClean => &[("Léger", &[35.0, 30.0, 15.0, 70.0]), ("Moyen", &[60.0, 50.0, 30.0, 50.0]), ("Fort", &[90.0, 80.0, 55.0, 35.0]), ("Capture d'écran", &[80.0, 70.0, 10.0, 85.0]), ("Photo de messagerie", &[85.0, 75.0, 45.0, 40.0])],
        Kind::Looks => &[
            ("Cinéma", &[1.0, 80.0, 59.0, 60.0, 0.0, 100.0]),
            ("Film argentique", &[49.0, 100.0, 3.0, 50.0, 0.0, 100.0]),
            ("Rétro chaud", &[9.0, 80.0, 12.0, 60.0, 0.0, 100.0]),
            ("Nuit bleue", &[20.0, 90.0, 58.0, 40.0, 0.0, 100.0]),
            ("N&B dramatique", &[16.0, 100.0, 18.0, 40.0, 0.0, 100.0]),
            ("Triple pastel", &[26.0, 70.0, 42.0, 50.0, 7.0, 50.0]),
            ("Néon", &[28.0, 80.0, 29.0, 50.0, 57.0, 40.0]),
            ("Doré", &[21.0, 90.0, 47.0, 50.0, 0.0, 100.0]),
        ],
        Kind::GlowPro => &[
            ("Rêveur", &[40.0, 60.0, 45.0, 40.0, 20.0, 2.0, 0.0, 0.0]),
            ("Néon", &[18.0, 140.0, 70.0, 300.0, 70.0, 1.0, 0.0, 0.0]),
            ("Bloom cinéma", &[60.0, 80.0, 75.0, 35.0, 25.0, 0.0, 30.0, 1.0]),
            ("Anamorphique bleu", &[30.0, 110.0, 80.0, 210.0, 80.0, 0.0, 100.0, 0.0]),
            ("Lueur chaude", &[35.0, 70.0, 55.0, 30.0, 50.0, 0.0, 0.0, 0.0]),
            ("Halo subtil", &[20.0, 35.0, 65.0, 40.0, 0.0, 0.0, 0.0, 1.0]),
        ],
        Kind::Blemish => &[("Léger", &[4.0, 40.0, 0.0, 1.0]), ("Portrait", &[6.0, 55.0, 25.0, 1.0]), ("Gros plan", &[12.0, 60.0, 30.0, 1.0]), ("Imperfections marquées", &[10.0, 80.0, 20.0, 1.0]), ("Peau lisse", &[8.0, 65.0, 70.0, 1.0])],
        Kind::SplitTone => &[("Sarcelle et orange", &[195.0, 45.0, 30.0, 45.0, 0.0]), ("Vintage", &[60.0, 25.0, 35.0, 30.0, 20.0]), ("Froid et chaud", &[220.0, 40.0, 40.0, 35.0, 0.0]), ("Lavande", &[270.0, 35.0, 300.0, 20.0, -10.0]), ("Désert", &[25.0, 30.0, 45.0, 40.0, 20.0]), ("Forêt", &[140.0, 30.0, 60.0, 25.0, 0.0])],
        Kind::ColorGrade => &[
            ("Blockbuster", &[195.0, 45.0, -5.0, 30.0, 10.0, 0.0, 35.0, 40.0, 5.0, 50.0, 0.0]),
            ("Vert Matrice", &[130.0, 40.0, -10.0, 120.0, 25.0, 0.0, 90.0, 20.0, 0.0, 60.0, 0.0]),
            ("Pastel", &[280.0, 15.0, 15.0, 330.0, 10.0, 5.0, 50.0, 15.0, 5.0, 70.0, 10.0]),
            ("Nocturne", &[230.0, 60.0, -15.0, 220.0, 30.0, -10.0, 200.0, 15.0, 0.0, 50.0, -20.0]),
            ("Été", &[40.0, 20.0, 0.0, 45.0, 15.0, 5.0, 55.0, 30.0, 5.0, 50.0, 10.0]),
            ("Automne", &[20.0, 35.0, -5.0, 30.0, 25.0, 0.0, 40.0, 30.0, 0.0, 50.0, 0.0]),
            ("Hiver", &[215.0, 40.0, -5.0, 205.0, 20.0, 0.0, 190.0, 15.0, 5.0, 50.0, 0.0]),
            ("Rétro 70", &[30.0, 30.0, 10.0, 45.0, 20.0, 0.0, 60.0, 25.0, -5.0, 60.0, 0.0]),
        ],
        Kind::Duotone => &[("Bleu et rose", &[240.0, 330.0, 80.0, 10.0]), ("Or et noir", &[30.0, 45.0, 60.0, 15.0]), ("Rouge et crème", &[355.0, 40.0, 60.0, 0.0]), ("Vert néon", &[260.0, 110.0, 90.0, 10.0]), ("Coucher violet", &[270.0, 25.0, 70.0, 5.0]), ("Cyan profond", &[220.0, 180.0, 75.0, 0.0])],
        Kind::HslMixer => &[
            ("Ciel profond", &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -10.0, 20.0, -20.0, -10.0, 30.0, -35.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            ("Verdure", &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -20.0, 10.0, 0.0, -10.0, 30.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            ("Peau douce", &[0.0, -10.0, 5.0, 5.0, -15.0, 15.0, 0.0, -10.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            ("Automne", &[0.0, 15.0, 0.0, 0.0, 30.0, 0.0, -30.0, 20.0, 0.0, -60.0, -20.0, -10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            ("Couleurs vives", &[0.0, 25.0, 0.0, 0.0, 25.0, 0.0, 0.0, 25.0, 0.0, 0.0, 25.0, 0.0, 0.0, 25.0, 0.0, 0.0, 25.0, 0.0, 0.0, 25.0, 0.0, 0.0, 25.0, 0.0]),
            ("Rouges seulement", &[0.0, 30.0, 0.0, 0.0, -100.0, 0.0, 0.0, -100.0, 0.0, 0.0, -100.0, 0.0, 0.0, -100.0, 0.0, 0.0, -100.0, 0.0, 0.0, -100.0, 0.0, 0.0, -60.0, 0.0]),
        ],
        Kind::Kelvin => &[("Tungstène (3200 K)", &[3200.0, 0.0, 100.0]), ("Fluorescent (4000 K)", &[4000.0, 15.0, 100.0]), ("Lumière du jour (5500 K)", &[5500.0, 0.0, 100.0]), ("Nuageux (7000 K)", &[7000.0, 0.0, 100.0]), ("Ombre (8000 K)", &[8000.0, 0.0, 100.0]), ("Bougie (2000 K)", &[2000.0, 0.0, 80.0])],
        Kind::CrossProcess => &[("Diapo → négatif", &[70.0, 0.0]), ("Négatif → diapo", &[70.0, 1.0])],
        Kind::BleachBypass => &[("Léger", &[35.0, 25.0]), ("Guerre", &[75.0, 60.0]), ("Thriller", &[60.0, 45.0])],
        Kind::Infrared => &[("Aérochrome", &[90.0, 1.0, 20.0]), ("Rêve rose", &[70.0, 0.0, 50.0]), ("Doux", &[45.0, 1.0, 10.0])],
        Kind::Thermal => &[("Fer", &[0.0, 20.0]), ("Arc-en-ciel", &[1.0, 30.0])],
        Kind::NightVision => &[("Militaire", &[200.0, 35.0, 70.0, 1.0]), ("Caméra de surveillance", &[150.0, 20.0, 40.0, 0.0])],
        Kind::Lomo => &[("Classique", &[40.0, 70.0, 40.0, 10.0]), ("Vert", &[30.0, 60.0, 35.0, -30.0]), ("Fort", &[70.0, 90.0, 60.0, 20.0])],
        Kind::Faded => &[("Polaroïd", &[45.0, 15.0, 25.0, 25.0]), ("Kodachrome délavé", &[30.0, 20.0, 15.0, 10.0]), ("Instax", &[35.0, 10.0, 5.0, 15.0]), ("Années 90", &[40.0, 35.0, -10.0, 30.0])],
        Kind::Clarity => &[("Paysage", &[45.0, 20.0, 40.0]), ("Portrait doux", &[-25.0, -20.0, 30.0]), ("Architecture", &[60.0, 40.0, 25.0]), ("Détails", &[20.0, 60.0, 15.0])],
        Kind::Orton => &[("Classique", &[20.0, 50.0, 30.0]), ("Rêve", &[35.0, 70.0, 40.0]), ("Subtil", &[12.0, 25.0, 15.0])],
        Kind::Hdr => &[("Naturel", &[35.0, 40.0, 30.0, 10.0, 1.0]), ("Réaliste", &[55.0, 50.0, 50.0, 20.0, 1.0]), ("Surréaliste", &[90.0, 60.0, 90.0, 60.0, 0.9]), ("Doux", &[50.0, 80.0, 20.0, 0.0, 1.1])],
        Kind::LensBlur => &[("Portrait", &[12.0, 50.0, 80.0, 0.0, 40.0, 30.0]), ("Arrière-plan doux", &[8.0, 20.0, 85.0, 0.0, 50.0, 0.0]), ("Reflets hexagonaux", &[16.0, 80.0, 75.0, 1.0, 50.0, 0.0]), ("Miroir catadioptrique", &[14.0, 70.0, 75.0, 2.0, 50.0, 0.0])],
        Kind::LensCorrect => &[("Grand-angle", &[-30.0, 20.0, 1.0, 105.0]), ("Fisheye léger", &[45.0, -20.0, 0.0, 95.0]), ("Caméra d'action", &[-60.0, 30.0, 2.0, 112.0]), ("Vintage", &[10.0, -50.0, 2.0, 100.0])],
        Kind::SmartSharpen => &[("Portrait", &[100.0, 1.0, 30.0, 20.0, 10.0]), ("Paysage", &[200.0, 1.2, 10.0, 0.0, 0.0]), ("Web", &[250.0, 0.6, 5.0, 0.0, 0.0]), ("Impression", &[300.0, 2.0, 20.0, 10.0, 10.0])],
        Kind::SunRays => &[("Matin", &[80.0, 10.0, 50.0, 70.0, 55.0, 60.0]), ("Forêt", &[50.0, 0.0, 80.0, 110.0, 65.0, 40.0]), ("Coucher", &[60.0, 45.0, 45.0, 90.0, 50.0, 90.0])],
        Kind::LightLeak => &[("Rouge à gauche", &[0.0, 10.0, 60.0, 45.0]), ("Orangé à droite", &[1.0, 30.0, 55.0, 50.0]), ("Magenta en haut", &[2.0, 320.0, 50.0, 60.0]), ("Coin doré", &[3.0, 40.0, 70.0, 55.0])],
        Kind::Bokeh => &[("Guirlandes", &[60.0, 20.0, 45.0, 30.0, 60.0, 4.0]), ("Ville la nuit", &[90.0, 30.0, 200.0, 80.0, 55.0, 9.0]), ("Doux et doré", &[30.0, 50.0, 40.0, 10.0, 40.0, 2.0])],
        Kind::Glitch => &[("Léger", &[4.0, 10.0, 20.0, 5.0, 3.0]), ("Corrompu", &[14.0, 30.0, 90.0, 50.0, 8.0]), ("RVB seulement", &[10.0, 1.0, 0.0, 0.0, 1.0])],
        Kind::Vhs => &[("Cassette usée", &[4.0, 6.0, 40.0, 50.0, 40.0]), ("Caméscope", &[2.0, 3.0, 20.0, 15.0, 20.0])],
        Kind::Scanlines => &[("Arcade", &[3.0, 55.0, 50.0, 25.0]), ("Télé ancienne", &[4.0, 45.0, 20.0, 45.0]), ("Moniteur", &[2.0, 30.0, 60.0, 5.0])],
        Kind::PixelSort => &[("Coulure horizontale", &[0.0, 20.0, 85.0, 0.0]), ("Pluie de pixels", &[1.0, 30.0, 95.0, 1.0]), ("Ciel seulement", &[0.0, 60.0, 100.0, 0.0])],
        Kind::Dither => &[("Écran vert", &[1.0, 0.0, 3.0]), ("Mac 1984", &[0.0, 3.0, 1.0]), ("Console 16", &[4.0, 2.0, 3.0]), ("Néon", &[7.0, 1.0, 2.0])],
        Kind::Rain => &[("Bruine", &[25.0, 15.0, -5.0, 25.0, 1.0]), ("Averse", &[60.0, 35.0, -12.0, 45.0, 2.0]), ("Tempête", &[100.0, 60.0, -30.0, 55.0, 3.0])],
        Kind::Snow => &[("Flocons légers", &[20.0, 2.0, 40.0, 70.0, 1.0]), ("Tempête de neige", &[90.0, 4.0, 60.0, 85.0, 2.0]), ("Gros flocons", &[25.0, 7.0, 20.0, 90.0, 3.0])],
        Kind::Fog => &[("Brume au sol", &[50.0, 200.0, 35.0, 210.0, 5.0]), ("Brouillard épais", &[85.0, 300.0, 100.0, 210.0, 5.0]), ("Fumée", &[45.0, 80.0, 70.0, 30.0, 5.0])],
        Kind::Stars => &[("Ciel clair", &[30.0, 70.0, 50.0, 1.0]), ("Voie lactée", &[90.0, 50.0, 60.0, 2.0])],
        Kind::Watercolor => &[("Lavis", &[5.0, 30.0, 50.0, 0.0]), ("Vif", &[3.0, 50.0, 30.0, 30.0]), ("Doux", &[7.0, 20.0, 60.0, -10.0])],
        Kind::ColorPencil => &[("Fin", &[2.0, 50.0, 30.0]), ("Pastel gras", &[5.0, 80.0, 50.0])],
        Kind::Pointillism => &[("Seurat", &[5.0, 80.0, 95.0]), ("Gros points", &[14.0, 60.0, 90.0])],
        Kind::StainedGlass => &[("Cathédrale", &[18.0, 3.0, 30.0]), ("Mosaïque", &[10.0, 1.0, 10.0])],
        Kind::NeonEdges => &[("Arc-en-ciel", &[2.0, 0.0, 1.0, 90.0]), ("Rose", &[2.0, 320.0, 0.0, 85.0]), ("Cyan", &[3.0, 185.0, 0.0, 80.0])],
        Kind::Kaleidoscope => &[("Fleur", &[8.0, 0.0, 50.0, 50.0]), ("Triangle", &[3.0, 30.0, 50.0, 50.0]), ("Rosace", &[16.0, 0.0, 50.0, 50.0])],
        Kind::Fisheye => &[("Léger", &[30.0, 100.0]), ("Fort", &[90.0, 110.0])],
        Kind::FrostedGlass => &[("Douche", &[12.0, 1.0]), ("Fin", &[3.0, 0.0])],
        Kind::Turbulence => &[("Chaleur", &[8.0, 30.0, 4.0, 1.0]), ("Liquide", &[40.0, 150.0, 3.0, 2.0])],
        _ => &[],
    }
}

fn hsl(h_deg: f32, s: f32, l: f32) -> [f32; 3] {
    let (r, g, b) = hsl_to_rgb((h_deg / 360.0).rem_euclid(1.0), s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    [r, g, b]
}

pub fn kelvin(k: f32) -> [f32; 3] {
    let t = k / 100.0;
    let r = if t <= 66.0 { 255.0 } else { 329.698_73 * (t - 60.0).powf(-0.133_204_76) };
    let g = if t <= 66.0 { 99.470_8 * t.ln() - 161.119_57 } else { 288.122_17 * (t - 60.0).powf(-0.075_514_85) };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_73 * (t - 10.0).ln() - 305.044_8
    };
    [r, g, b].map(|v| (v / 255.0).clamp(0.0, 1.0))
}

fn look_px(c: [f32; 3], v: &[f32; 9], bw: bool) -> [f32; 3] {
    let mut o = [c[0] + v[0] * 0.1, c[1] - v[1] * 0.08, c[2] - v[0] * 0.1];
    let l = luma(o);
    o = if bw { [l; 3] } else { o.map(|x| l + (x - l) * (1.0 + v[2])) };
    o = o.map(|x| ((x - 0.5) * (1.0 + v[3]) + 0.5).clamp(0.0, 1.0));
    o = o.map(|x| v[4] + x * (1.0 - v[4]));
    let l = luma(o).clamp(0.0, 1.0);
    let (sh, hi) = (hsl(v[5], 1.0, 0.5), hsl(v[7], 1.0, 0.5));
    std::array::from_fn(|k| o[k] + (sh[k] - 0.5) * v[6] * (1.0 - l).powi(2) + (hi[k] - 0.5) * v[8] * l * l)
}

fn norm3(v: [f32; 3]) -> [f32; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    v.map(|x| x / n)
}

fn screen(a: f32, b: f32) -> f32 {
    1.0 - (1.0 - a) * (1.0 - b.clamp(0.0, 1.0))
}

fn is_skin(c: [f32; 3]) -> bool {
    let (h, s, l) = rgb_to_hsl(c[0], c[1], c[2]);
    (h < 50.0 / 360.0 || h > 340.0 / 360.0) && (0.1..0.75).contains(&s) && (0.15..0.92).contains(&l)
}

fn lum_vec(img: &RgbaImage) -> Vec<f32> {
    img.pixels().map(|q| luma(rgb(q.0))).collect()
}

fn grad(lv: &[f32], w: usize, h: usize, x: usize, y: usize) -> (f32, f32) {
    let at = |dx: i32, dy: i32| lv[((y as i32 + dy).clamp(0, h as i32 - 1) as usize) * w + (x as i32 + dx).clamp(0, w as i32 - 1) as usize];
    (at(1, 0) - at(-1, 0), at(0, 1) - at(0, -1))
}

fn px(img: &RgbaImage, x: f32, y: f32) -> [f32; 3] {
    let (w, h) = img.dimensions();
    rgb(img.get_pixel((x.max(0.0) as u32).min(w - 1), (y.max(0.0) as u32).min(h - 1)).0)
}

fn overlay(img: &RgbaImage, buf: Vec<[f32; 3]>, opacity: f32) -> RgbaImage {
    let w = img.width() as usize;
    map_xy(img, move |x, y, c| {
        let b = buf[y as usize * w + x as usize];
        std::array::from_fn(|k| screen(c[k], b[k] * opacity))
    })
}

fn disk(buf: &mut [[f32; 3]], w: usize, h: usize, cx: f32, cy: f32, r: f32, col: [f32; 3], soft: f32) {
    let r = r.max(0.5);
    let (x0, x1) = ((cx - r).floor().max(0.0) as usize, ((cx + r).ceil() as usize).min(w.saturating_sub(1)));
    let (y0, y1) = ((cy - r).floor().max(0.0) as usize, ((cy + r).ceil() as usize).min(h.saturating_sub(1)));
    if cx + r < 0.0 || cy + r < 0.0 {
        return;
    }
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt() / r;
            if d < 1.0 {
                let e = ((1.0 - d) / soft.max(0.02)).min(1.0);
                let o = &mut buf[y * w + x];
                for k in 0..3 {
                    o[k] += col[k] * e;
                }
            }
        }
    }
}

fn kernel(shape: u32) -> Vec<(f32, f32)> {
    let mut v = vec![(0.0, 0.0)];
    for ring in 1..=4 {
        let n = ring * 8;
        let rr = ring as f32 / 4.0;
        if shape == 2 && ring < 3 {
            continue;
        }
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU;
            let k = if shape == 1 {
                let seg = TAU / 6.0;
                let t = (a.rem_euclid(seg) - seg / 2.0).abs();
                (seg / 2.0).cos() / t.cos().max(0.5)
            } else {
                1.0
            };
            v.push((a.cos() * rr * k, a.sin() * rr * k));
        }
    }
    v
}

fn dither(img: &RgbaImage, pal: &[[u8; 3]], method: u32, ps: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (sw, sh) = (w.div_ceil(ps).max(1), h.div_ceil(ps).max(1));
    let pal: Vec<[f32; 3]> = pal.iter().map(|c| c.map(|v| v as f32 / 255.0)).collect();
    let near = |c: [f32; 3]| -> [f32; 3] { *pal.iter().min_by(|a, b| dist2(**a, c).total_cmp(&dist2(**b, c))).unwrap_or(&[0.0; 3]) };
    let mut s: Vec<[f32; 3]> = (0..sw * sh).map(|i| px(img, ((i % sw) * ps + ps / 2) as f32, ((i / sw) * ps + ps / 2) as f32)).collect();
    let spread = 0.6 / (pal.len() as f32).sqrt();
    let (swu, shu) = (sw as usize, sh as usize);
    match method {
        0 | 1 => {
            let n = if method == 0 { 4 } else { 8 };
            for (i, c) in s.iter_mut().enumerate() {
                let (x, y) = (i % swu, i / swu);
                let t = (bayer(x % n, y % n, n) - 0.5) * spread;
                *c = near(c.map(|v| v + t));
            }
        }
        _ => {
            let taps: &[(i32, i32, f32)] = if method == 2 { &[(1, 0, 7.0 / 16.0), (-1, 1, 3.0 / 16.0), (0, 1, 5.0 / 16.0), (1, 1, 1.0 / 16.0)] } else { &[(1, 0, 0.125), (2, 0, 0.125), (-1, 1, 0.125), (0, 1, 0.125), (1, 1, 0.125), (0, 2, 0.125)] };
            for i in 0..s.len() {
                let old = s[i];
                let new = near(old);
                s[i] = new;
                let (x, y) = ((i % swu) as i32, (i / swu) as i32);
                for (dx, dy, k) in taps {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && (nx as usize) < swu && (ny as usize) < shu {
                        let j = ny as usize * swu + nx as usize;
                        for q in 0..3 {
                            s[j][q] += (old[q] - new[q]) * k;
                        }
                    }
                }
            }
        }
    }
    map_xy(img, move |x, y, _| s[(y as u32 / ps) as usize * swu + (x as u32 / ps) as usize])
}

fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) * 0.3 + (a[1] - b[1]).powi(2) * 0.59 + (a[2] - b[2]).powi(2) * 0.11
}

fn bayer(x: usize, y: usize, n: usize) -> f32 {
    let mut v = 0;
    let (mut x, mut y, mut bit) = (x, y, n / 2);
    let mut shift = 0;
    while bit > 0 {
        let (bx, by) = ((x & bit != 0) as usize, (y & bit != 0) as usize);
        v |= ((bx ^ by) | (by << 1)) << shift;
        shift += 2;
        x &= !bit;
        y &= !bit;
        bit /= 2;
    }
    let v = v.reverse_bits() >> (usize::BITS as usize - shift);
    (v as f32 + 0.5) / (n * n) as f32
}

fn pixel_sort_rows(img: &RgbaImage, lo: f32, hi: f32, rev: bool) -> RgbaImage {
    let w = img.width() as usize;
    let mut out = img.clone();
    out.par_chunks_mut(w * 4).for_each(|row| {
        let mut x = 0;
        while x < w {
            let l = |i: usize| luma(rgb([row[i * 4], row[i * 4 + 1], row[i * 4 + 2], 255]));
            if !(lo..=hi).contains(&l(x)) {
                x += 1;
                continue;
            }
            let s = x;
            while x < w && (lo..=hi).contains(&l(x)) {
                x += 1;
            }
            let mut run: Vec<[u8; 4]> = (s..x).map(|i| [row[i * 4], row[i * 4 + 1], row[i * 4 + 2], row[i * 4 + 3]]).collect();
            run.sort_by(|a, b| {
                let o = luma(rgb(*a)).total_cmp(&luma(rgb(*b)));
                if rev { o.reverse() } else { o }
            });
            for (k, p) in run.into_iter().enumerate() {
                row[(s + k) * 4..(s + k) * 4 + 3].copy_from_slice(&p[..3]);
            }
        }
    });
    out
}

pub fn apply(k: Kind, img: &RgbaImage, g: &dyn Fn(usize) -> f32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (wf, hf) = (w as f32, h as f32);
    let (wu, hu) = (w as usize, h as usize);
    let md = wf.max(hf).max(1.0);
    match k {
        Kind::Relight => {
            let sig = 1.0 + g(5) / 100.0 * md * 0.012;
            let lv = lum_vec(&imageops::fast_blur(img, sig));
            let relief = g(12) * 6.0;
            let amb = g(11) / 100.0;
            let light = |t: f32, lr: f32, ud: f32, i: f32| (kelvin(t), wf * (0.5 + lr / 100.0 * 0.7), hf * (0.5 - ud / 100.0 * 0.7), (0.15 + g(3) / 100.0 * 1.2) * md, md * (0.6 + g(3) / 100.0), i / 100.0);
            let mut lights = vec![light(g(0), g(1), g(2), g(4))];
            if g(6) >= 0.5 {
                lights.push(light(g(7), g(8), g(9), g(10)));
            }
            map_xy(img, move |x, y, c| {
                let (gx, gy) = grad(&lv, wu, hu, x as usize, y as usize);
                let n = norm3([-gx * relief, -gy * relief, 1.0]);
                let mut acc = [amb; 3];
                for (col, lx, ly, lz, reach, inten) in &lights {
                    let d = [lx - x, ly - y, *lz];
                    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-3);
                    let ndl = ((n[0] * d[0] + n[1] * d[1] + n[2] * d[2]) / len).max(0.0);
                    let fall = 1.0 / (1.0 + (len / reach).powi(2));
                    for q in 0..3 {
                        acc[q] += col[q] * inten * ndl * fall * 2.0;
                    }
                }
                std::array::from_fn(|q| c[q] * acc[q])
            })
        }
        Kind::Dehaze => {
            let amt = g(0) / 100.0;
            let r = (g(1).round() as usize).clamp(1, 30);
            let dark0 = RgbaImage::from_fn(w, h, |x, y| {
                let q = img.get_pixel(x, y).0;
                let m = q[0].min(q[1]).min(q[2]);
                Rgba([m, m, m, 255])
            });
            let dv: Vec<f32> = box_blur(&morph(&dark0, r / 2 + 1, false), r, r).pixels().map(|q| q.0[0] as f32 / 255.0).collect();
            let mut idx: Vec<usize> = (0..dv.len()).collect();
            let top = (dv.len() / 1000).max(1);
            idx.select_nth_unstable_by(dv.len() - top, |a, b| dv[*a].total_cmp(&dv[*b]));
            let mut a = [0f32; 3];
            for &i in &idx[dv.len() - top..] {
                let q = rgb(img.get_pixel((i % wu) as u32, (i / wu) as u32).0);
                for k in 0..3 {
                    a[k] += q[k] / top as f32;
                }
            }
            let amax = a.iter().fold(0.05f32, |m, v| m.max(*v));
            let (sky, sat) = (g(2) >= 0.5, g(3) / 100.0);
            map_xy(img, move |x, y, c| {
                let i = y as usize * wu + x as usize;
                let mut am = amt;
                if sky {
                    let (mx, mn) = (c.iter().fold(0f32, |m, v| m.max(*v)), c.iter().fold(1f32, |m, v| m.min(*v)));
                    if luma(c) > 0.7 && mx - mn < 0.15 {
                        am *= 0.3;
                    }
                }
                let o: [f32; 3] = if am >= 0.0 {
                    let t = (1.0 - 0.95 * am * dv[i] / amax).max(0.12);
                    std::array::from_fn(|q| (c[q] - a[q]) / t + a[q])
                } else {
                    std::array::from_fn(|q| c[q] + (a[q].max(0.75) - c[q]) * -am * 0.6)
                };
                let l = luma(o);
                o.map(|v| l + (v - l) * (1.0 + sat))
            })
        }
        Kind::JpegClean => {
            let (blk, chr, lumn, det) = (g(0) / 100.0, g(1) / 100.0, g(2) / 100.0, g(3) / 100.0);
            let b3 = box_blur(img, 1, 1);
            let rc = 1 + (chr * 6.0) as usize;
            let bc = box_blur(img, rc, rc);
            let lv = lum_vec(img);
            map_xy(img, move |x, y, c| {
                let (xi, yi) = (x as usize, y as usize);
                let (gx, gy) = grad(&lv, wu, hu, xi, yi);
                let keep = ((gx.abs() + gy.abs()) * 6.0 * (0.3 + det)).min(1.0);
                let q3 = rgb(b3.get_pixel(xi as u32, yi as u32).0);
                let qc = rgb(bc.get_pixel(xi as u32, yi as u32).0);
                let y0 = luma(c);
                let mut yl = y0 + (luma(q3) - y0) * lumn * (1.0 - keep);
                if xi % 8 == 0 || xi % 8 == 7 || yi % 8 == 0 || yi % 8 == 7 {
                    yl += (luma(q3) - yl) * blk * (1.0 - keep) * 0.8;
                }
                let lc = luma(qc);
                std::array::from_fn(|q| {
                    let ch = c[q] - y0;
                    yl + ch + (qc[q] - lc - ch) * chr
                })
            })
        }
        Kind::Looks => {
            let slots: Vec<(usize, f32)> = (0..3).map(|s| ((g(s * 2).round().max(0.0) as usize).min(LOOK_VALS.len() - 1), g(s * 2 + 1) / 100.0)).filter(|s| s.0 > 0 && s.1 > 0.0).collect();
            map(img, move |c| {
                let mut o = c;
                for (k, a) in &slots {
                    let (v, bw) = &LOOK_VALS[*k];
                    let t = look_px(o, v, *bw);
                    o = std::array::from_fn(|q| o[q] + (t[q] - o[q]) * a);
                }
                o
            })
        }
        Kind::GlowPro => {
            let (r, inten, thr, gs, mode, ana, keep) = (g(0), g(1) / 100.0, g(2) / 100.0, g(4) / 100.0, g(5).round() as u32, g(6) / 100.0, g(7) >= 0.5);
            let tint = hsl(g(3), 1.0, 0.5);
            let bright = map(img, move |c| {
                let k = ((luma(c) - thr) / (1.0 - thr).max(0.01)).clamp(0.0, 1.0);
                std::array::from_fn(|q| (c[q] + (tint[q] - c[q]) * gs) * k)
            });
            let (rx, ry) = ((r * (1.0 + ana * 4.0) / 2.0) as usize + 1, (r / 2.0) as usize + 1);
            let bl = box_blur(&box_blur(&bright, rx, ry), rx, ry);
            zip(img, &bl, move |c, b| {
                let b = b.map(|v| v * inten * 1.5);
                let o: [f32; 3] = std::array::from_fn(|q| match mode {
                    0 => screen(c[q], b[q]),
                    1 => c[q] + b[q],
                    2 => c[q] + b[q] * (0.25 + c[q] * (1.0 - c[q]) * 2.0),
                    _ => {
                        if c[q] < 0.5 {
                            c[q] + b[q] * c[q]
                        } else {
                            c[q] + b[q] * (1.0 - c[q])
                        }
                    }
                });
                if keep {
                    let lc = luma(c).max(1e-3);
                    let lo = luma(o);
                    c.map(|v| v * lo / lc)
                } else {
                    o
                }
            })
        }
        Kind::Blemish => {
            let (size, sens, sm, skin_only) = (g(0), g(1) / 100.0, g(2) / 100.0, g(3) >= 0.5);
            let rr = (size as usize).max(1);
            let base = box_blur(img, rr * 2, rr * 2);
            let soft = box_blur(img, 3, 3);
            let thr = 0.02 + (1.0 - sens) * 0.12;
            map_xy(img, move |x, y, c| {
                let (xi, yi) = (x as u32, y as u32);
                let b = rgb(base.get_pixel(xi, yi).0);
                let skin = is_skin(c) || is_skin(b);
                if skin_only && !skin {
                    return c;
                }
                let dark = luma(b) - luma(c);
                let red = (c[0] - c[1]) - (b[0] - b[1]);
                let s = ((dark.max(red * 0.7) - thr) / thr).clamp(0.0, 1.0);
                let mut o: [f32; 3] = std::array::from_fn(|q| c[q] + (b[q] - c[q]) * s);
                if skin && sm > 0.0 {
                    let f = rgb(soft.get_pixel(xi, yi).0);
                    o = std::array::from_fn(|q| o[q] + (f[q] - o[q]) * sm * 0.85);
                }
                o
            })
        }
        Kind::SplitTone => {
            let (sh, ss, hh, hs, bal) = (hsl(g(0), 1.0, 0.5), g(1) / 100.0, hsl(g(2), 1.0, 0.5), g(3) / 100.0, g(4) / 100.0);
            map(img, move |c| {
                let pp = luma(c) - 0.5 - bal * 0.3;
                let (ws, wh) = ((-pp * 2.0).clamp(0.0, 1.0), (pp * 2.0).clamp(0.0, 1.0));
                std::array::from_fn(|q| c[q] + (sh[q] - 0.5) * ss * ws * 0.6 + (hh[q] - 0.5) * hs * wh * 0.6)
            })
        }
        Kind::ColorGrade => {
            let zones: [([f32; 3], f32, f32); 3] = std::array::from_fn(|z| (hsl(g(z * 3), 1.0, 0.5), g(z * 3 + 1) / 100.0, g(z * 3 + 2) / 100.0));
            let (spread, bal) = (0.5 + g(9) / 100.0, g(10) / 100.0);
            map(img, move |c| {
                let l = (luma(c) + bal * 0.25).clamp(0.0, 1.0);
                let ws = ((0.5 - l) / (0.5 * spread)).clamp(0.0, 1.0);
                let wh = ((l - 0.5) / (0.5 * spread)).clamp(0.0, 1.0);
                let wm = (1.0 - ws - wh).max(0.0);
                std::array::from_fn(|q| c[q] + [ws, wm, wh].iter().zip(&zones).map(|(wz, (col, s, lu))| wz * ((col[q] - 0.5) * s * 0.5 + lu * 0.3)).sum::<f32>())
            })
        }
        Kind::Duotone => {
            let (s, con) = (g(2) / 100.0, g(3) / 100.0);
            let (dark, light) = (hsl(g(0), s, 0.12), hsl(g(1), s, 0.88));
            map(img, move |c| {
                let l = ((luma(c) - 0.5) * (1.0 + con) + 0.5).clamp(0.0, 1.0);
                std::array::from_fn(|q| dark[q] + (light[q] - dark[q]) * l)
            })
        }
        Kind::HslMixer => {
            let v: [f32; 24] = std::array::from_fn(|i| g(i) / 100.0);
            map(img, move |c| {
                let (hh, s, l) = rgb_to_hsl(c[0], c[1], c[2]);
                let hd = hh * 360.0;
                let (mut dh, mut ds, mut dl) = (0.0, 0.0, 0.0);
                for (b, cen) in HSL_CENTERS.iter().enumerate() {
                    let d = ((hd - cen).rem_euclid(360.0)).min((cen - hd).rem_euclid(360.0));
                    let wgt = (1.0 - d / 45.0).max(0.0);
                    dh += wgt * v[b * 3];
                    ds += wgt * v[b * 3 + 1];
                    dl += wgt * v[b * 3 + 2];
                }
                let (r, gg, b) = hsl_to_rgb((hh + dh * 30.0 / 360.0).rem_euclid(1.0), (s * (1.0 + ds)).clamp(0.0, 1.0), (l + dl * 0.3 * s).clamp(0.0, 1.0));
                [r, gg, b]
            })
        }
        Kind::Kelvin => {
            let (t, r0) = (kelvin(g(0)), kelvin(6500.0));
            let m: [f32; 3] = std::array::from_fn(|q| r0[q] / t[q].max(0.05));
            let lm = luma(m).max(1e-3);
            let m = m.map(|v| v / lm);
            let (tint, a) = (g(1) / 100.0 * 0.1, g(2) / 100.0);
            map(img, move |c| std::array::from_fn(|q| c[q] + (c[q] * m[q] + if q == 1 { -tint } else { tint * 0.5 } - c[q]) * a))
        }
        Kind::CrossProcess => {
            let (a, t) = (g(0) / 100.0, g(1) >= 0.5);
            map(img, move |c| {
                let s = |x: f32, k: f32| 0.5 + (x - 0.5) * k;
                let o = if t { [0.15 + c[0] * 0.7, s(c[1], 1.25), s(c[2], 1.45)] } else { [s(c[0], 1.4), s(c[1], 1.2), 0.25 + c[2] * 0.5] };
                std::array::from_fn(|q| c[q] + (o[q] - c[q]) * a)
            })
        }
        Kind::BleachBypass => {
            let (a, con) = (g(0) / 100.0, g(1) / 100.0);
            map(img, move |c| {
                let l = luma(c);
                let ov = |x: f32| if l < 0.5 { 2.0 * x * l } else { 1.0 - 2.0 * (1.0 - x) * (1.0 - l) };
                std::array::from_fn(|q| {
                    let d = c[q] + (l - c[q]) * 0.6 * a;
                    let v = d + (ov(d) - d) * a;
                    0.5 + (v - 0.5) * (1.0 + con * 0.6)
                })
            })
        }
        Kind::Infrared => {
            let (a, swap, glow) = (g(0) / 100.0, g(1) >= 0.5, g(2) / 100.0);
            map(img, move |c| {
                let base = if swap { [c[2], c[1], c[0]] } else { c };
                let fol = (c[1] - (c[0] + c[2]) / 2.0).max(0.0);
                let l = luma(c);
                let o: [f32; 3] = std::array::from_fn(|q| base[q] + fol * [1.6, 1.2, 1.4][q] + l * l * glow * 0.3);
                std::array::from_fn(|q| c[q] + (o[q] - c[q]) * a)
            })
        }
        Kind::Thermal => {
            let stops = THERMAL[(g(0).round() as usize).min(3)];
            let con = g(1) / 100.0;
            map(img, move |c| gradient_at(stops, ((luma(c) - 0.5) * (1.0 + con) + 0.5).clamp(0.0, 1.0)))
        }
        Kind::NightVision => {
            let (gain, noise, vig, lines) = (g(0) / 100.0, g(1) / 100.0, g(2) / 100.0, g(3) >= 0.5);
            map_xy(img, move |x, y, c| {
                let mut l = luma(c) * gain + (hash(x as u32, y as u32, 7) - 0.5) * noise * 0.4;
                let d = ((x / wf - 0.5).powi(2) + (y / hf - 0.5).powi(2)) * 2.0;
                l *= 1.0 - vig * d;
                if lines && (y as u32) % 3 == 0 {
                    l *= 0.75;
                }
                [l * 0.3, l, l * 0.35]
            })
        }
        Kind::Lomo => {
            let (sat, vig, con, tint) = (g(0) / 100.0, g(1) / 100.0, g(2) / 100.0, g(3) / 100.0);
            map_xy(img, move |x, y, c| {
                let l = luma(c);
                let d = ((x / wf - 0.5).powi(2) + (y / hf - 0.5).powi(2)) * 2.0;
                let v = 1.0 - vig * d * 0.9;
                std::array::from_fn(|q| {
                    let s = l + (c[q] - l) * (1.0 + sat);
                    let s = 0.5 + (s - 0.5) * (1.0 + con * 0.8);
                    (s + [tint * 0.1, 0.0, -tint * 0.1][q]) * v
                })
            })
        }
        Kind::Faded => {
            let (fade, grain, warm, des) = (g(0) / 100.0 * 0.3, g(1) / 100.0, g(2) / 100.0, g(3) / 100.0);
            map_xy(img, move |x, y, c| {
                let l = luma(c);
                let n = (hash(x as u32, y as u32, 3) - 0.5) * grain * 0.15;
                std::array::from_fn(|q| {
                    let v = c[q] + (l - c[q]) * des;
                    fade + v * (1.0 - fade * 1.3) + [warm * 0.06, 0.0, -warm * 0.06][q] + n
                })
            })
        }
        Kind::Clarity => {
            let (cl, tex) = (g(0) / 100.0, g(1) / 100.0);
            let big = imageops::fast_blur(img, g(2).max(1.0));
            let small = box_blur(img, 2, 2);
            map_xy(img, move |x, y, c| {
                let (b, s) = (rgb(big.get_pixel(x as u32, y as u32).0), rgb(small.get_pixel(x as u32, y as u32).0));
                let l = luma(c);
                let mid = 4.0 * l * (1.0 - l);
                std::array::from_fn(|q| c[q] + (c[q] - b[q]) * cl * mid * 1.2 + (c[q] - s[q]) * tex * 1.5)
            })
        }
        Kind::Orton => {
            let (inten, br) = (g(1) / 100.0, g(2) / 100.0);
            let bl = imageops::fast_blur(&map(img, move |c| c.map(|v| v * (1.0 + br))), g(0));
            zip(img, &bl, move |c, b| std::array::from_fn(|q| c[q] + (screen(c[q], b[q]) - c[q]) * inten))
        }
        Kind::Hdr => {
            let (force, detail, sat, gamma) = (g(0) / 100.0, g(2) / 100.0, g(3) / 100.0, g(4).max(0.1));
            let base = imageops::fast_blur(img, g(1));
            zip(img, &base, move |c, b| {
                let (l, bl) = (luma(c).max(1e-4), luma(b));
                let b2 = 0.5 + (bl - 0.5) * (1.0 - force * 0.7);
                let l2 = (b2 + (l - bl) * (1.0 + detail * 2.5)).clamp(0.0, 1.0).powf(1.0 / gamma);
                let o = c.map(|v| v * l2 / l);
                let lo = luma(o);
                o.map(|v| lo + (v - lo) * (1.0 + sat))
            })
        }
        Kind::LensBlur => {
            let (rad, hl, thr, fy, zone) = (g(0), g(1) / 100.0, g(2) / 100.0, g(4) / 100.0, g(5) / 100.0);
            let pts = kernel(g(3).round() as u32);
            sample(img, move |x, y, at| {
                let s = if zone <= 0.0 { 1.0 } else { (((y as f32 / hf - fy).abs() - zone / 2.0) / 0.2).clamp(0.0, 1.0) };
                let r = rad * s;
                let me = at(x as f32, y as f32);
                if r < 0.5 {
                    return me;
                }
                let (mut acc, mut ws) = ([0f32; 3], 0f32);
                for (dx, dy) in &pts {
                    let q = at(x as f32 + dx * r, y as f32 + dy * r);
                    let l = luma([q[0], q[1], q[2]]);
                    let wgt = 1.0 + if l > thr { (l - thr) / (1.0 - thr).max(0.01) * hl * 8.0 } else { 0.0 };
                    for k in 0..3 {
                        acc[k] += q[k] * wgt;
                    }
                    ws += wgt;
                }
                [acc[0] / ws, acc[1] / ws, acc[2] / ws, me[3]]
            })
        }
        Kind::LensCorrect => {
            let (k, vig, ab, scale) = (g(0) / 100.0 * 0.35, g(1) / 100.0, g(2) * 2.0 / md, g(3) / 100.0);
            sample(img, move |x, y, at| {
                let (cx, cy) = (wf / 2.0, hf / 2.0);
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let r2 = (dx * dx + dy * dy) / (md * md / 4.0);
                let f = (1.0 + k * r2) / scale.max(0.1);
                let src = |e: f32| at(cx + dx * f * (1.0 + e), cy + dy * f * (1.0 + e));
                let v = 1.0 + vig * r2 * 0.5;
                [src(ab)[0] * v, src(0.0)[1] * v, src(-ab)[2] * v, at(x as f32, y as f32)[3]]
            })
        }
        Kind::SmartSharpen => {
            let (gain, nr, fs, fh) = (g(0) / 100.0, g(2) / 100.0, g(3) / 100.0, g(4) / 100.0);
            let bl = imageops::fast_blur(img, g(1));
            zip(img, &bl, move |c, b| {
                let l = luma(c);
                let d: [f32; 3] = std::array::from_fn(|q| c[q] - b[q]);
                let mag = luma(d).abs();
                let nrk = if nr > 0.0 { ((mag - nr * 0.03) / (nr * 0.03)).clamp(0.0, 1.0) } else { 1.0 };
                let fade = (1.0 - fs * (1.0 - l).powi(2) - fh * l * l).max(0.0);
                std::array::from_fn(|q| c[q] + d[q] * gain * nrk * fade)
            })
        }
        Kind::SunRays => {
            let (sx, sy, len, inten, thr, warm) = (g(0) / 100.0 * wf, g(1) / 100.0 * hf, g(2) / 100.0, g(3) / 100.0, g(4) / 100.0, g(5) / 100.0);
            let bright = map(img, move |c| {
                let k = ((luma(c) - thr) / (1.0 - thr).max(0.01)).clamp(0.0, 1.0);
                c.map(|v| v * k)
            });
            let rays = sample(&bright, move |x, y, at| {
                let mut acc = [0f32; 3];
                let n = 32;
                for i in 0..n {
                    let t = i as f32 / n as f32 * len;
                    let q = at(x as f32 + (sx - x as f32) * t, y as f32 + (sy - y as f32) * t);
                    for k in 0..3 {
                        acc[k] += q[k] * (1.0 - i as f32 / n as f32);
                    }
                }
                [acc[0] / n as f32 * 2.0, acc[1] / n as f32 * 2.0, acc[2] / n as f32 * 2.0, 1.0]
            });
            let col = [1.0, 1.0 - warm * 0.15, 1.0 - warm * 0.45];
            zip(img, &rays, move |c, r| std::array::from_fn(|q| screen(c[q], r[q] * inten * col[q])))
        }
        Kind::LightLeak => {
            let (pos, inten, size) = (g(0).round() as u32, g(2) / 100.0, g(3) / 100.0);
            let col = hsl(g(1), 1.0, 0.55);
            let (cx, cy) = match pos {
                0 => (0.0, hf / 2.0),
                1 => (wf, hf / 2.0),
                2 => (wf / 2.0, 0.0),
                _ => (wf, 0.0),
            };
            map_xy(img, move |x, y, c| {
                let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / (md * size.max(0.05));
                let f = (-d * d * 2.0).exp() * inten;
                std::array::from_fn(|q| screen(c[q], col[q] * f))
            })
        }
        Kind::Bokeh => {
            let seed = g(5) as u32;
            let mut buf = vec![[0f32; 3]; wu * hu];
            for i in 0..g(0) as u32 {
                let (x0, y0) = (hash(i, 1, seed) * wf, hash(i, 2, seed) * hf);
                let r = g(1) * (0.5 + hash(i, 3, seed));
                let col = hsl(g(2) + (hash(i, 4, seed) - 0.5) * g(3) * 3.6, 0.8, 0.6).map(|v| v * (0.4 + hash(i, 5, seed) * 0.6));
                disk(&mut buf, wu, hu, x0, y0, r, col, 0.15);
            }
            overlay(img, buf, g(4) / 100.0)
        }
        Kind::Glitch => {
            let (rs, disp, blocks, seed) = (g(0), g(2), g(3) / 100.0, g(4) as u32);
            let sh = (hf / g(1).max(1.0)).max(1.0);
            sample(img, move |x, y, at| {
                let row = (y as f32 / sh) as u32;
                let off = if hash(row, 11, seed) < 0.35 { (hash(row, 12, seed) - 0.5) * 2.0 * disp } else { 0.0 };
                let (xs, yf) = (x as f32 + off, y as f32);
                let mut o = [at(xs + rs, yf)[0], at(xs, yf)[1], at(xs - rs, yf)[2], at(x as f32, yf)[3]];
                if hash(x / 16, y / 16, seed + 3) < blocks * 0.3 {
                    o = [o[2], o[0], o[1], o[3]];
                }
                o
            })
        }
        Kind::Vhs => {
            let (hb, chroma, noise, lines, fade) = (g(0).round() as i32, g(1), g(2) / 100.0, g(3) / 100.0, g(4) / 100.0);
            sample(img, move |x, y, at| {
                let (xf, yf) = (x as f32, y as f32);
                let mut acc = [0f32; 3];
                for d in -hb..=hb {
                    let q = at(xf + d as f32, yf);
                    let (r, b) = (at(xf + d as f32 + chroma, yf)[0], at(xf + d as f32 - chroma, yf)[2]);
                    acc[0] += r;
                    acc[1] += q[1];
                    acc[2] += b;
                }
                let n = (2 * hb + 1) as f32;
                let mut o = acc.map(|v| v / n);
                let nz = (hash(y, 0, 9) - 0.5) * noise * 0.3 + (hash(x, y, 8) - 0.5) * noise * 0.2;
                let streak = if hash(y / 2, 1, 5) < lines * 0.05 { 0.3 } else { 0.0 };
                let l = luma(o);
                for k in 0..3 {
                    o[k] = fade * 0.15 + (o[k] + (l - o[k]) * fade * 0.4) * (1.0 - fade * 0.2) + nz + streak;
                }
                [o[0], o[1], o[2], at(xf, yf)[3]]
            })
        }
        Kind::Scanlines => {
            let (sp, inten, mask, curve) = (g(0).max(2.0), g(1) / 100.0, g(2) / 100.0, g(3) / 100.0);
            sample(img, move |x, y, at| {
                let (cx, cy) = (wf / 2.0, hf / 2.0);
                let (dx, dy) = ((x as f32 + 0.5 - cx) / cx, (y as f32 + 0.5 - cy) / cy);
                let f = 1.0 + curve * 0.25 * (dx * dx + dy * dy);
                let (sx, sy) = (cx + dx * f * cx, cy + dy * f * cy);
                let alpha = at(x as f32, y as f32)[3];
                if sx < 0.0 || sy < 0.0 || sx >= wf || sy >= hf {
                    return [0.0, 0.0, 0.0, alpha];
                }
                let q = at(sx, sy);
                let line = 1.0 - inten * (0.5 + 0.5 * (TAU * y as f32 / sp).cos());
                let m = (x % 3) as usize;
                let o: [f32; 3] = std::array::from_fn(|k| q[k] * line * if k == m { 1.0 + mask * 0.4 } else { 1.0 - mask * 0.3 });
                [o[0], o[1], o[2], alpha]
            })
        }
        Kind::PixelSort => {
            let (lo, hi, rev) = (g(1) / 100.0, g(2) / 100.0, g(3) >= 0.5);
            if g(0) < 0.5 {
                pixel_sort_rows(img, lo, hi, rev)
            } else {
                imageops::rotate270(&pixel_sort_rows(&imageops::rotate90(img), lo, hi, rev))
            }
        }
        Kind::Dither => dither(img, PALETTES[(g(0).round() as usize).min(7)], g(1).round() as u32, g(2).round().max(1.0) as u32),
        Kind::Rain => {
            let (len, ang, seed) = (g(1), g(2).to_radians(), g(4) as u32);
            let mut buf = vec![[0f32; 3]; wu * hu];
            let n = (g(0) / 100.0 * (wu * hu) as f32 / 300.0) as u32;
            for i in 0..n {
                let (x0, y0) = (hash(i, 1, seed) * wf, hash(i, 2, seed) * hf);
                let l = len * (0.6 + 0.8 * hash(i, 3, seed));
                let (sx, sy) = (ang.sin(), ang.cos());
                for t in 0..l as u32 {
                    let (x, y) = (x0 + sx * t as f32, y0 + sy * t as f32);
                    if x >= 0.0 && y >= 0.0 && x < wf && y < hf {
                        let o = &mut buf[y as usize * wu + x as usize];
                        let v = 0.5 * (1.0 - t as f32 / l * 0.5);
                        *o = [o[0] + v * 0.85, o[1] + v * 0.9, o[2] + v];
                    }
                }
            }
            overlay(img, buf, g(3) / 100.0)
        }
        Kind::Snow => {
            let (size, soft, seed) = (g(1), 0.1 + g(2) / 100.0 * 0.9, g(4) as u32);
            let mut buf = vec![[0f32; 3]; wu * hu];
            let n = (g(0) / 100.0 * (wu * hu) as f32 / 400.0) as u32;
            for i in 0..n {
                disk(&mut buf, wu, hu, hash(i, 1, seed) * wf, hash(i, 2, seed) * hf, size * (0.5 + hash(i, 3, seed)), [1.0; 3], soft);
            }
            overlay(img, buf, g(3) / 100.0)
        }
        Kind::Fog => {
            let (dens, scale, height) = (g(0) / 100.0, g(1).max(2.0), g(2) / 100.0);
            let col = hsl(g(3), g(4) / 100.0, 0.85);
            map_xy(img, move |x, y, c| {
                let n = fbm(x / scale, y / scale, 5);
                let hg = ((y / hf - (1.0 - height)) / height.max(0.05) + 0.3).clamp(0.0, 1.0);
                let a = (dens * (0.4 + 0.6 * n) * hg).clamp(0.0, 1.0);
                std::array::from_fn(|q| c[q] + (col[q] - c[q]) * a)
            })
        }
        Kind::Stars => {
            let (dens, br, dark, seed) = (g(0) / 100.0 * 0.004, g(1) / 100.0, g(2) / 100.0, g(3) as u32);
            map_xy(img, move |x, y, c| {
                if luma(c) > dark || hash(x as u32, y as u32, seed + 40) > dens {
                    return c;
                }
                let b = hash(x as u32, y as u32, seed + 41).powi(3) * br * 1.5 + 0.2 * br;
                c.map(|v| screen(v, b))
            })
        }
        Kind::Watercolor => {
            let (edges, paper, sat) = (g(1) / 100.0, g(2) / 100.0, g(3) / 100.0);
            let k = kuwahara(img, g(0).round() as i32);
            let lv = lum_vec(&k);
            map_xy(&k, move |x, y, c| {
                let (gx, gy) = grad(&lv, wu, hu, x as usize, y as usize);
                let e = ((gx.abs() + gy.abs()) * 3.0 * edges).min(0.6);
                let pn = (fbm(x / 3.0, y / 3.0, 3) - 0.5) * paper * 0.15;
                let l = luma(c);
                std::array::from_fn(|q| (l + (c[q] - l) * (1.0 + sat)) * (1.0 - e) + pn + 0.03)
            })
        }
        Kind::ColorPencil => {
            let (th, press, paper) = (g(0), g(1) / 100.0, g(2) / 100.0);
            let r = th as usize + 1;
            let bl = box_blur(img, r, r);
            map_xy(img, move |x, y, c| {
                let b = rgb(bl.get_pixel(x as u32, y as u32).0);
                let v = (luma(c) / luma(b).max(0.02)).min(1.0).powf(2.0);
                let hatch = 0.85 + 0.15 * ((x + y) * TAU / (th * 3.0)).sin();
                let pn = (hash(x as u32, y as u32, 2) - 0.5) * paper * 0.12;
                std::array::from_fn(|q| (1.0 - (1.0 - b[q]) * press) * v * hatch + pn)
            })
        }
        Kind::Pointillism => {
            let (size, dens, bg) = (g(0), g(1) / 100.0, g(2) / 100.0);
            let mut buf = vec![[bg; 3]; wu * hu];
            let step = (size * (1.2 - dens * 0.6)).max(1.0);
            let (nx, ny) = ((wf / step) as u32 + 1, (hf / step) as u32 + 1);
            for j in 0..ny {
                for i in 0..nx {
                    let (cx, cy) = ((i as f32 + hash(i, j, 1)) * step, (j as f32 + hash(i, j, 2)) * step);
                    let col = px(img, cx, cy);
                    let r = size / 2.0;
                    let (x0, x1) = ((cx - r).max(0.0) as usize, ((cx + r) as usize).min(wu.saturating_sub(1)));
                    let (y0, y1) = ((cy - r).max(0.0) as usize, ((cy + r) as usize).min(hu.saturating_sub(1)));
                    for y in y0..=y1 {
                        for x in x0..=x1 {
                            if (x as f32 - cx).powi(2) + (y as f32 - cy).powi(2) <= r * r {
                                buf[y * wu + x] = col;
                            }
                        }
                    }
                }
            }
            map_xy(img, move |x, y, _| buf[y as usize * wu + x as usize])
        }
        Kind::StainedGlass => {
            let (cs, lead, br) = (g(0).max(2.0), g(1), g(2) / 100.0);
            map_xy(img, |x, y, _| {
                let (ci, cj) = ((x / cs) as i32, (y / cs) as i32);
                let (mut d1, mut d2, mut best) = (f32::MAX, f32::MAX, (0.0, 0.0));
                for j in cj - 1..=cj + 1 {
                    for i in ci - 1..=ci + 1 {
                        let (ui, uj) = ((i + 2) as u32, (j + 2) as u32);
                        let (sx, sy) = ((i as f32 + hash(ui, uj, 31)) * cs, (j as f32 + hash(ui, uj, 32)) * cs);
                        let d = ((x - sx).powi(2) + (y - sy).powi(2)).sqrt();
                        if d < d1 {
                            d2 = d1;
                            d1 = d;
                            best = (sx, sy);
                        } else if d < d2 {
                            d2 = d;
                        }
                    }
                }
                if d2 - d1 < lead {
                    return [0.05; 3];
                }
                px(img, best.0, best.1).map(|v| v * (1.0 + br))
            })
        }
        Kind::NeonEdges => {
            let (t, hue, rainbow, bg) = (g(0) as usize, g(1), g(2) >= 0.5, g(3) / 100.0);
            let lv = lum_vec(&box_blur(img, t, t));
            map_xy(img, move |x, y, c| {
                let (gx, gy) = grad(&lv, wu, hu, x as usize, y as usize);
                let e = ((gx * gx + gy * gy).sqrt() * 6.0).min(1.0);
                let col = if rainbow { hsl(gy.atan2(gx).to_degrees() + hue, 1.0, 0.55) } else { hsl(hue, 1.0, 0.55) };
                std::array::from_fn(|q| c[q] * (1.0 - bg) + col[q] * e * 1.5)
            })
        }
        Kind::Kaleidoscope => {
            let (seg, rot, cx, cy) = (g(0).max(2.0), g(1).to_radians(), g(2) / 100.0 * wf, g(3) / 100.0 * hf);
            sample(img, move |x, y, at| {
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                let r = (dx * dx + dy * dy).sqrt();
                let sa = TAU / seg;
                let mut a = (dy.atan2(dx) - rot).rem_euclid(sa);
                if a > sa / 2.0 {
                    a = sa - a;
                }
                let q = at(cx + r * (a + rot).cos(), cy + r * (a + rot).sin());
                [q[0], q[1], q[2], at(x as f32, y as f32)[3]]
            })
        }
        Kind::Mirror => {
            let axis = g(0).round() as u32;
            sample(img, move |x, y, at| {
                let (xf, yf) = (x as f32, y as f32);
                let (sx, sy) = match axis {
                    0 => (if xf > wf / 2.0 { wf - 1.0 - xf } else { xf }, yf),
                    1 => (if xf < wf / 2.0 { wf - 1.0 - xf } else { xf }, yf),
                    2 => (xf, if yf > hf / 2.0 { hf - 1.0 - yf } else { yf }),
                    3 => (xf, if yf < hf / 2.0 { hf - 1.0 - yf } else { yf }),
                    _ => (if xf > wf / 2.0 { wf - 1.0 - xf } else { xf }, if yf > hf / 2.0 { hf - 1.0 - yf } else { yf }),
                };
                let q = at(sx, sy);
                [q[0], q[1], q[2], at(xf, yf)[3]]
            })
        }
        Kind::Fisheye => {
            let (amt, zoom) = (g(0) / 100.0, g(1) / 100.0);
            let rad = wf.min(hf) / 2.0;
            sample(img, move |x, y, at| {
                let (cx, cy) = (wf / 2.0, hf / 2.0);
                let (dx, dy) = ((x as f32 - cx) / rad, (y as f32 - cy) / rad);
                let r = (dx * dx + dy * dy).sqrt();
                let f = if r < 1.0 && r > 0.0 { r.powf(1.0 + amt * 1.5) / r } else { 1.0 } / zoom.max(0.1);
                let q = at(cx + dx * f * rad, cy + dy * f * rad);
                [q[0], q[1], q[2], at(x as f32, y as f32)[3]]
            })
        }
        Kind::FrostedGlass => {
            let (d, soft) = (g(0), g(1) >= 0.5);
            sample(img, move |x, y, at| {
                let n = if soft { 4 } else { 1 };
                let mut acc = [0f32; 3];
                for k in 0..n {
                    let q = at(x as f32 + (hash(x, y, 50 + k) - 0.5) * 2.0 * d, y as f32 + (hash(x, y, 60 + k) - 0.5) * 2.0 * d);
                    for c in 0..3 {
                        acc[c] += q[c] / n as f32;
                    }
                }
                [acc[0], acc[1], acc[2], at(x as f32, y as f32)[3]]
            })
        }
        Kind::Turbulence => {
            let (force, sc, det, seed) = (g(0), g(1).max(2.0), g(2).round().max(1.0) as u32, g(3));
            sample(img, move |x, y, at| {
                let (xf, yf) = (x as f32 / sc + seed * 7.13, y as f32 / sc);
                let ox = (fbm(xf, yf, det) - 0.5) * 2.0 * force;
                let oy = (fbm(xf + 31.7, yf + 17.3, det) - 0.5) * 2.0 * force;
                let q = at(x as f32 + ox, y as f32 + oy);
                [q[0], q[1], q[2], at(x as f32, y as f32)[3]]
            })
        }
        _ => img.clone(),
    }
}

#[cfg(test)]
pub fn option_count() -> usize {
    GROUPS.iter().flat_map(|g| g.1.iter()).map(|k| 1 + params(*k).iter().map(|p| p.choices.len().max(1)).sum::<usize>() + presets(*k).len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_and_bayer_are_consistent() {
        assert_eq!(LOOK_NAMES.len(), LOOK_VALS.len());
        let mut seen: Vec<f32> = (0..16).map(|i| bayer(i % 4, i / 4, 4)).collect();
        seen.sort_by(f32::total_cmp);
        seen.dedup();
        assert_eq!(seen.len(), 16);
        assert!(kelvin(6500.0).iter().all(|v| *v > 0.9));
        assert!(kelvin(2000.0)[2] < 0.2);
        println!("fx options: {}", option_count());
        assert!(option_count() > 500, "{}", option_count());
    }
}
