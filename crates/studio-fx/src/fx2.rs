use crate::filters::{Kind, Param, box_blur, c, fbm, gradient_at, hash, luma, map, map_xy, on, p, rgb, sample, zip};
use crate::fx::{disk, grad, hsl, look_px, lum_vec, overlay, px, screen};
use crate::rgb_to_hsl;
use image::{RgbaImage, imageops};
use std::f32::consts::{PI, TAU};

pub const GROUPS: &[(&str, &[Kind])] = &[
    ("Films et couleurs", &[Kind::FilmSim, Kind::NegativeFilm, Kind::Colorize, Kind::ColorSplash, Kind::Tritone, Kind::FalseColor, Kind::ChannelSwap, Kind::GradientOverlay, Kind::SkyGradient]),
    ("Lumière et contraste", &[Kind::LocalContrast, Kind::DodgeBurn, Kind::Backlight, Kind::SoftGlare, Kind::VignettePro, Kind::EdgeBlur, Kind::Deband]),
    ("Graphisme et dessin", &[Kind::PopArt, Kind::Warhol, Kind::CmykHalftone, Kind::Engraving, Kind::Crosshatch, Kind::Charcoal, Kind::InkSketch, Kind::LiquidChrome]),
    ("Pixels et formes", &[Kind::TileMosaic, Kind::HexPixel, Kind::LowPoly, Kind::PixelArt]),
    ("Mouvement et optique", &[Kind::WaterReflection, Kind::CameraShake, Kind::RadialChroma, Kind::ZoomBurst, Kind::Anaglyph]),
    ("Cadres et décor", &[Kind::CineBars, Kind::PhotoFrame, Kind::Texture, Kind::Fireworks, Kind::Confetti, Kind::Glitter, Kind::Bubbles]),
];

const FILMS: &[&str] = &["Portrait 160", "Portrait 400", "Couleur 200", "Diapo vive", "Diapo douce", "Ciné 500T", "Ciné 50D", "N&B 400", "N&B 3200", "Instantané", "Couleur périmée", "Chrome classique"];
const FILM_VALS: [([f32; 9], bool); 12] = [
    ([0.12, 0.04, -0.12, 0.0, 0.04, 25.0, 0.08, 40.0, 0.12], false),
    ([0.15, 0.03, -0.05, 0.06, 0.05, 30.0, 0.1, 40.0, 0.15], false),
    ([0.2, 0.0, 0.08, 0.1, 0.03, 200.0, 0.1, 45.0, 0.2], false),
    ([0.0, -0.05, 0.35, 0.28, 0.0, 220.0, 0.15, 50.0, 0.1], false),
    ([0.05, 0.0, 0.1, 0.12, 0.02, 190.0, 0.1, 40.0, 0.1], false),
    ([-0.35, 0.05, -0.05, 0.12, 0.04, 200.0, 0.3, 30.0, 0.3], false),
    ([0.1, -0.03, 0.05, 0.15, 0.02, 190.0, 0.15, 35.0, 0.2], false),
    ([0.0, 0.0, 0.0, 0.18, 0.03, 0.0, 0.0, 0.0, 0.0], true),
    ([0.0, 0.0, 0.0, 0.32, 0.06, 0.0, 0.0, 0.0, 0.0], true),
    ([0.15, 0.05, -0.15, -0.08, 0.12, 180.0, 0.2, 50.0, 0.2], false),
    ([0.3, 0.15, -0.2, -0.05, 0.15, 160.0, 0.25, 320.0, 0.2], false),
    ([-0.05, 0.0, -0.25, 0.2, 0.03, 200.0, 0.15, 40.0, 0.1], false),
];
const FALSE_PALETTES: [&[[u8; 3]]; 6] = [
    &[[10, 20, 60], [40, 120, 220], [170, 220, 255], [255, 255, 255]],
    &[[0, 0, 0], [140, 0, 0], [255, 90, 0], [255, 220, 60], [255, 255, 255]],
    &[[0, 10, 30], [0, 80, 140], [80, 200, 230], [230, 250, 255]],
    &[[10, 0, 20], [80, 0, 120], [0, 255, 120], [230, 255, 0]],
    &[[120, 0, 160], [0, 80, 255], [0, 220, 120], [255, 240, 0], [255, 60, 0]],
    &[[10, 0, 30], [255, 0, 150], [0, 230, 255], [255, 255, 255]],
];
const SKIES: [&[[u8; 3]]; 5] = [
    &[[40, 110, 210], [120, 180, 240], [210, 230, 250]],
    &[[60, 40, 120], [230, 90, 80], [255, 190, 110]],
    &[[40, 45, 55], [90, 95, 110], [150, 155, 165]],
    &[[5, 8, 25], [20, 35, 80], [60, 70, 120]],
    &[[120, 150, 210], [250, 180, 170], [255, 225, 190]],
];
const SWAPS: [[usize; 3]; 6] = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];

pub fn title(k: Kind) -> &'static str {
    match k {
        Kind::FilmSim => "Simulation de pellicule",
        Kind::NegativeFilm => "Négatif couleur",
        Kind::Colorize => "Colorisation",
        Kind::ColorSplash => "Couleur isolée",
        Kind::Tritone => "Trichromie",
        Kind::FalseColor => "Fausses couleurs",
        Kind::ChannelSwap => "Permuter les couches",
        Kind::GradientOverlay => "Incrustation de dégradé",
        Kind::SkyGradient => "Remplacer le ciel (dégradé)",
        Kind::LocalContrast => "Contraste local",
        Kind::DodgeBurn => "Densité automatique (dodge & burn)",
        Kind::Backlight => "Correction de contre-jour",
        Kind::SoftGlare => "Voile lumineux",
        Kind::VignettePro => "Vignettage avancé",
        Kind::EdgeBlur => "Bords flous",
        Kind::Deband => "Supprimer les bandes de dégradé",
        Kind::PopArt => "Pop art",
        Kind::Warhol => "Pop art en 4 cases",
        Kind::CmykHalftone => "Trame quadrichromie",
        Kind::Engraving => "Gravure",
        Kind::Crosshatch => "Hachures croisées",
        Kind::Charcoal => "Fusain",
        Kind::InkSketch => "Esquisse à l'encre",
        Kind::LiquidChrome => "Chrome liquide",
        Kind::TileMosaic => "Mosaïque de carreaux",
        Kind::HexPixel => "Pixels hexagonaux",
        Kind::LowPoly => "Low poly",
        Kind::PixelArt => "Pixel art",
        Kind::WaterReflection => "Reflet dans l'eau",
        Kind::CameraShake => "Tremblement de caméra",
        Kind::RadialChroma => "Aberration radiale",
        Kind::ZoomBurst => "Effet de zoom",
        Kind::Anaglyph => "Anaglyphe 3D",
        Kind::CineBars => "Bandes cinéma",
        Kind::PhotoFrame => "Cadre photo",
        Kind::Texture => "Texture de surface",
        Kind::Fireworks => "Feux d'artifice",
        Kind::Confetti => "Confettis",
        Kind::Glitter => "Paillettes",
        Kind::Bubbles => "Bulles",
        _ => "",
    }
}

pub fn params(k: Kind) -> &'static [Param] {
    match k {
        Kind::FilmSim => const { &[c("Pellicule", FILMS, 0.0), p("Intensité", 0.0, 100.0, 100.0, " %"), p("Grain", 0.0, 100.0, 20.0, " %")] },
        Kind::NegativeFilm => const { &[c("Rendu", &["Négatif à masque orangé", "Simple inversion"], 0.0), p("Intensité", 0.0, 100.0, 100.0, " %")] },
        Kind::Colorize => const { &[p("Teinte", 0.0, 360.0, 30.0, "°"), p("Saturation", 0.0, 100.0, 45.0, " %"), p("Intensité", 0.0, 100.0, 100.0, " %")] },
        Kind::ColorSplash => const { &[p("Teinte gardée", 0.0, 360.0, 0.0, "°"), p("Tolérance", 5.0, 120.0, 30.0, "°"), p("Douceur", 0.0, 100.0, 40.0, " %"), p("Reste en gris", 0.0, 100.0, 100.0, " %")] },
        Kind::Tritone => const { &[p("Teinte des foncés", 0.0, 360.0, 240.0, "°"), p("Teinte des moyens", 0.0, 360.0, 330.0, "°"), p("Teinte des clairs", 0.0, 360.0, 50.0, "°"), p("Saturation", 0.0, 100.0, 70.0, " %")] },
        Kind::FalseColor => const { &[c("Palette", &["Ciel", "Feu", "Glace", "Toxique", "Arc-en-ciel", "Néon"], 0.0), p("Contraste", -50.0, 100.0, 0.0, ""), p("Intensité", 0.0, 100.0, 100.0, " %")] },
        Kind::ChannelSwap => const { &[c("Ordre", &["RVB", "RBV", "VRB", "VBR", "BRV", "BVR"], 3.0)] },
        Kind::GradientOverlay => const { &[p("Teinte de départ", 0.0, 360.0, 20.0, "°"), p("Teinte d'arrivée", 0.0, 360.0, 280.0, "°"), p("Angle", 0.0, 360.0, 90.0, "°"), p("Opacité", 0.0, 100.0, 40.0, " %"), c("Mode", &["Superposition", "Produit", "Lumière douce", "Normal"], 2.0)] },
        Kind::SkyGradient => const { &[c("Ciel", &["Bleu clair", "Coucher de soleil", "Orage", "Nuit", "Aube"], 0.0), p("Hauteur de l'horizon", 10.0, 90.0, 55.0, " %"), p("Sensibilité", 0.0, 100.0, 50.0, " %")] },
        Kind::LocalContrast => const { &[p("Quantité", 0.0, 200.0, 60.0, " %"), p("Rayon", 2.0, 60.0, 12.0, " px")] },
        Kind::DodgeBurn => const { &[p("Quantité", 0.0, 100.0, 40.0, " %"), p("Rayon", 4.0, 120.0, 40.0, " px")] },
        Kind::Backlight => const { &[p("Déboucher les ombres", 0.0, 100.0, 50.0, " %"), p("Récupérer le ciel", 0.0, 100.0, 30.0, " %")] },
        Kind::SoftGlare => const { &[p("Intensité", 0.0, 100.0, 40.0, " %"), p("Rayon", 4.0, 80.0, 25.0, " px"), p("Désaturation", 0.0, 100.0, 20.0, " %")] },
        Kind::VignettePro => const { &[p("Quantité", 0.0, 100.0, 50.0, " %"), p("Taille", 0.0, 100.0, 50.0, " %"), p("Rondeur", 0.0, 100.0, 50.0, " %"), p("Contour progressif", 5.0, 100.0, 50.0, " %"), c("Couleur", &["Noir", "Blanc", "Teinte"], 0.0), p("Teinte", 0.0, 360.0, 220.0, "°")] },
        Kind::EdgeBlur => const { &[p("Flou", 1.0, 40.0, 12.0, " px"), p("Zone nette", 10.0, 90.0, 45.0, " %")] },
        Kind::Deband => const { &[p("Quantité", 0.0, 100.0, 50.0, " %")] },
        Kind::PopArt => const { &[p("Niveaux", 2.0, 8.0, 4.0, ""), p("Saturation", 0.0, 100.0, 70.0, " %"), p("Contours", 0.0, 100.0, 60.0, " %")] },
        Kind::Warhol => const { &[p("Décalage de teinte", 30.0, 180.0, 90.0, "°"), p("Saturation", 0.0, 100.0, 80.0, " %")] },
        Kind::CmykHalftone => const { &[p("Taille de trame", 3.0, 30.0, 8.0, " px"), on("Papier blanc", 1.0)] },
        Kind::Engraving => const { &[p("Espacement", 2.0, 20.0, 6.0, " px"), p("Angle", 0.0, 180.0, 30.0, "°"), c("Encre", &["Noir", "Sépia", "Bleu nuit"], 0.0)] },
        Kind::Crosshatch => const { &[p("Espacement", 3.0, 20.0, 7.0, " px"), p("Contraste", 0.0, 100.0, 50.0, " %")] },
        Kind::Charcoal => const { &[p("Épaisseur", 1.0, 8.0, 3.0, " px"), p("Contraste", 0.0, 100.0, 60.0, " %"), p("Grain", 0.0, 100.0, 50.0, " %")] },
        Kind::InkSketch => const { &[p("Seuil des traits", 0.0, 100.0, 40.0, " %"), p("Lavis de couleur", 0.0, 100.0, 30.0, " %")] },
        Kind::LiquidChrome => const { &[p("Reflets", 1.0, 8.0, 3.0, ""), p("Teinte", 0.0, 360.0, 210.0, "°"), p("Couleur", 0.0, 100.0, 15.0, " %")] },
        Kind::TileMosaic => const { &[p("Taille des carreaux", 4.0, 80.0, 16.0, " px"), p("Joints", 0.0, 6.0, 1.0, " px"), p("Couleur des joints", 0.0, 100.0, 85.0, " %")] },
        Kind::HexPixel => const { &[p("Taille", 3.0, 80.0, 14.0, " px")] },
        Kind::LowPoly => const { &[p("Taille des triangles", 6.0, 120.0, 30.0, " px"), p("Variation", 0.0, 100.0, 40.0, " %")] },
        Kind::PixelArt => const { &[p("Taille des pixels", 2.0, 32.0, 6.0, " px"), p("Niveaux de couleur", 2.0, 16.0, 6.0, ""), on("Contour sombre", 1.0)] },
        Kind::WaterReflection => const { &[p("Ligne d'eau", 20.0, 95.0, 60.0, " %"), p("Vagues", 0.0, 20.0, 4.0, " px"), p("Teinte de l'eau", 0.0, 100.0, 30.0, " %")] },
        Kind::CameraShake => const { &[p("Amplitude", 1.0, 40.0, 8.0, " px"), p("Angle", 0.0, 180.0, 20.0, "°"), p("Copies", 2.0, 8.0, 4.0, "")] },
        Kind::RadialChroma => const { &[p("Intensité", 0.0, 100.0, 30.0, " %")] },
        Kind::ZoomBurst => const { &[p("Intensité", 0.0, 100.0, 40.0, " %"), p("Centre X", 0.0, 100.0, 50.0, " %"), p("Centre Y", 0.0, 100.0, 50.0, " %"), p("Zone nette", 0.0, 100.0, 20.0, " %")] },
        Kind::Anaglyph => const { &[p("Décalage", 1.0, 30.0, 6.0, " px"), on("Gris", 0.0)] },
        Kind::CineBars => const { &[c("Format", &["2,39:1 Scope", "2:1 Univisium", "1,85:1 Flat", "4:3 (bandes latérales)"], 0.0), p("Opacité", 0.0, 100.0, 100.0, " %")] },
        Kind::PhotoFrame => const { &[c("Style", &["Polaroïd", "Bord blanc", "Bord noir", "Passe-partout", "Pellicule 35 mm"], 0.0), p("Épaisseur", 1.0, 20.0, 6.0, " %")] },
        Kind::Texture => const { &[c("Matière", &["Papier", "Toile", "Béton", "Grain fin", "Pierre"], 0.0), p("Intensité", 0.0, 100.0, 40.0, " %"), p("Échelle", 20.0, 400.0, 100.0, " %")] },
        Kind::Fireworks => const { &[p("Nombre", 1.0, 20.0, 5.0, ""), p("Taille", 20.0, 400.0, 120.0, " px"), p("Variété de couleurs", 0.0, 100.0, 70.0, " %"), p("Graine", 0.0, 999.0, 1.0, "")] },
        Kind::Confetti => const { &[p("Densité", 0.0, 100.0, 40.0, " %"), p("Taille", 2.0, 20.0, 6.0, " px"), p("Graine", 0.0, 999.0, 1.0, "")] },
        Kind::Glitter => const { &[p("Densité", 0.0, 100.0, 30.0, " %"), p("Taille", 2.0, 24.0, 8.0, " px"), p("Seuil des reflets", 0.0, 100.0, 70.0, " %"), p("Graine", 0.0, 999.0, 1.0, "")] },
        Kind::Bubbles => const { &[p("Nombre", 5.0, 150.0, 30.0, ""), p("Taille", 6.0, 120.0, 40.0, " px"), p("Opacité", 0.0, 100.0, 60.0, " %"), p("Graine", 0.0, 999.0, 1.0, "")] },
        _ => &[],
    }
}

pub fn presets(k: Kind) -> &'static [(&'static str, &'static [f32])] {
    match k {
        Kind::FilmSim => &[("Portrait doux", &[0.0, 90.0, 15.0]), ("Paysage vif", &[3.0, 85.0, 5.0]), ("Nuit ciné", &[5.0, 100.0, 30.0]), ("N&B reportage", &[8.0, 100.0, 45.0]), ("Souvenir", &[10.0, 80.0, 25.0]), ("Années 70", &[10.0, 100.0, 35.0])],
        Kind::NegativeFilm => &[("Négatif", &[0.0, 100.0]), ("Inversion", &[1.0, 100.0])],
        Kind::Colorize => &[("Sépia", &[32.0, 40.0, 100.0]), ("Cyanotype", &[205.0, 55.0, 100.0]), ("Rose", &[330.0, 45.0, 80.0]), ("Vert émeraude", &[150.0, 45.0, 90.0]), ("Or", &[45.0, 60.0, 90.0])],
        Kind::ColorSplash => &[("Rouge seulement", &[0.0, 25.0, 40.0, 100.0]), ("Jaune seulement", &[55.0, 25.0, 40.0, 100.0]), ("Bleu seulement", &[215.0, 30.0, 40.0, 100.0]), ("Vert seulement", &[120.0, 35.0, 40.0, 100.0])],
        Kind::Tritone => &[("Nuit violette", &[250.0, 300.0, 40.0, 70.0]), ("Océan", &[220.0, 190.0, 60.0, 60.0]), ("Feu", &[0.0, 20.0, 50.0, 80.0]), ("Pastel", &[260.0, 330.0, 60.0, 40.0])],
        Kind::FalseColor => &[("Thermique", &[1.0, 20.0, 100.0]), ("Glace", &[2.0, 10.0, 100.0]), ("Psychédélique", &[4.0, 40.0, 100.0]), ("Néon doux", &[5.0, 0.0, 60.0])],
        Kind::GradientOverlay => &[("Coucher de soleil", &[20.0, 300.0, 90.0, 45.0, 2.0]), ("Bleu et rose", &[200.0, 330.0, 45.0, 40.0, 0.0]), ("Chaleur", &[30.0, 50.0, 90.0, 30.0, 2.0]), ("Nuit", &[230.0, 260.0, 90.0, 50.0, 1.0])],
        Kind::SkyGradient => &[("Ciel bleu", &[0.0, 50.0, 50.0]), ("Coucher", &[1.0, 55.0, 55.0]), ("Orage", &[2.0, 50.0, 60.0]), ("Nuit", &[3.0, 50.0, 60.0])],
        Kind::LocalContrast => &[("Léger", &[30.0, 10.0]), ("Fort", &[120.0, 20.0]), ("Grand rayon", &[60.0, 50.0])],
        Kind::DodgeBurn => &[("Portrait", &[30.0, 60.0]), ("Paysage dramatique", &[70.0, 40.0])],
        Kind::Backlight => &[("Léger", &[30.0, 15.0]), ("Fort", &[80.0, 50.0])],
        Kind::SoftGlare => &[("Rêve", &[60.0, 35.0, 30.0]), ("Brume douce", &[35.0, 50.0, 10.0])],
        Kind::VignettePro => &[("Discret", &[30.0, 60.0, 70.0, 70.0, 0.0, 0.0]), ("Dramatique", &[85.0, 35.0, 50.0, 40.0, 0.0, 0.0]), ("Blanc doux", &[60.0, 50.0, 80.0, 80.0, 1.0, 0.0]), ("Bleu nuit", &[70.0, 40.0, 60.0, 60.0, 2.0, 225.0])],
        Kind::EdgeBlur => &[("Miniature", &[10.0, 40.0]), ("Rêve", &[20.0, 55.0])],
        Kind::PopArt => &[("Affiche", &[4.0, 80.0, 70.0]), ("Comics", &[5.0, 60.0, 90.0])],
        Kind::Warhol => &[("Classique", &[90.0, 85.0]), ("Doux", &[60.0, 50.0])],
        Kind::CmykHalftone => &[("Journal", &[6.0, 1.0]), ("Affiche", &[14.0, 1.0])],
        Kind::Engraving => &[("Billet de banque", &[4.0, 30.0, 2.0]), ("Ancien", &[7.0, 45.0, 1.0])],
        Kind::Crosshatch => &[("Fin", &[4.0, 50.0]), ("Épais", &[10.0, 70.0])],
        Kind::Charcoal => &[("Esquisse", &[2.0, 50.0, 40.0]), ("Dramatique", &[5.0, 85.0, 70.0])],
        Kind::InkSketch => &[("Traits seuls", &[40.0, 0.0]), ("Aquarellé", &[35.0, 60.0])],
        Kind::LiquidChrome => &[("Argent", &[3.0, 210.0, 0.0]), ("Or", &[3.0, 45.0, 60.0]), ("Holographique", &[6.0, 280.0, 50.0])],
        Kind::TileMosaic => &[("Salle de bains", &[12.0, 2.0, 95.0]), ("Romain", &[8.0, 1.0, 40.0])],
        Kind::LowPoly => &[("Fin", &[16.0, 30.0]), ("Géométrique", &[50.0, 60.0])],
        Kind::PixelArt => &[("8 bits", &[8.0, 4.0, 1.0]), ("16 bits", &[4.0, 8.0, 1.0]), ("Doux", &[3.0, 12.0, 0.0])],
        Kind::WaterReflection => &[("Lac calme", &[60.0, 2.0, 20.0]), ("Mer agitée", &[55.0, 10.0, 40.0])],
        Kind::CameraShake => &[("Léger", &[4.0, 10.0, 3.0]), ("Séisme", &[20.0, 60.0, 6.0])],
        Kind::ZoomBurst => &[("Action", &[60.0, 50.0, 50.0, 15.0]), ("Doux", &[25.0, 50.0, 50.0, 35.0])],
        Kind::Anaglyph => &[("Couleur", &[6.0, 0.0]), ("Gris", &[8.0, 1.0])],
        Kind::PhotoFrame => &[("Polaroïd", &[0.0, 6.0]), ("Galerie", &[3.0, 12.0]), ("Film", &[4.0, 8.0])],
        Kind::Texture => &[("Papier aquarelle", &[0.0, 50.0, 100.0]), ("Toile de peintre", &[1.0, 45.0, 80.0]), ("Mur", &[2.0, 50.0, 150.0])],
        Kind::Fireworks => &[("Nouvel an", &[8.0, 160.0, 90.0, 3.0]), ("Bouquet", &[3.0, 260.0, 40.0, 5.0])],
        Kind::Confetti => &[("Fête", &[50.0, 6.0, 2.0]), ("Pluie de confettis", &[90.0, 4.0, 5.0])],
        Kind::Glitter => &[("Paillettes", &[40.0, 8.0, 65.0, 1.0]), ("Étoiles", &[15.0, 18.0, 75.0, 2.0])],
        Kind::Bubbles => &[("Savon", &[25.0, 50.0, 60.0, 1.0]), ("Aquarium", &[60.0, 20.0, 50.0, 2.0])],
        _ => &[],
    }
}

fn hue_of(c: [f32; 3]) -> (f32, f32) {
    let (h, s, _) = rgb_to_hsl(c[0], c[1], c[2]);
    (h * 360.0, s)
}

fn hue_dist(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

fn paint_buf(img: &RgbaImage, buf: Vec<Option<[f32; 3]>>) -> RgbaImage {
    let w = img.width() as usize;
    map_xy(img, move |x, y, c| buf[y as usize * w + x as usize].unwrap_or(c))
}

pub fn apply(k: Kind, img: &RgbaImage, g: &dyn Fn(usize) -> f32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (wf, hf) = (w as f32, h as f32);
    let (wu, hu) = (w as usize, h as usize);
    let md = wf.max(hf).max(1.0);
    match k {
        Kind::FilmSim => {
            let (v, bw) = FILM_VALS[(g(0).round().max(0.0) as usize).min(11)];
            let (a, gr) = (g(1) / 100.0, g(2) / 100.0);
            map_xy(img, move |x, y, c| {
                let t = look_px(c, &v, bw);
                let n = (hash(x as u32, y as u32, 77) - 0.5) * gr * 0.12;
                std::array::from_fn(|q| c[q] + (t[q] + n - c[q]) * a)
            })
        }
        Kind::NegativeFilm => {
            let (simple, a) = (g(0) >= 0.5, g(1) / 100.0);
            map(img, move |c| {
                let i = c.map(|v| 1.0 - v);
                let o = if simple { i } else { [i[0] * 0.95 + 0.2, i[1] * 0.75 + 0.08, i[2] * 0.55] };
                std::array::from_fn(|q| c[q] + (o[q] - c[q]) * a)
            })
        }
        Kind::Colorize => {
            let (hue, s, a) = (g(0), g(1) / 100.0, g(2) / 100.0);
            map(img, move |c| {
                let t = hsl(hue, s, luma(c));
                std::array::from_fn(|q| c[q] + (t[q] - c[q]) * a)
            })
        }
        Kind::ColorSplash => {
            let (target, tol, soft, gray) = (g(0), g(1), g(2) / 100.0 * 60.0 + 1.0, g(3) / 100.0);
            map(img, move |c| {
                let (hh, s) = hue_of(c);
                let keep = (((tol + soft) - hue_dist(hh, target)) / soft).clamp(0.0, 1.0) * (s * 4.0).min(1.0);
                let l = luma(c);
                let d = gray * (1.0 - keep);
                c.map(|v| v + (l - v) * d)
            })
        }
        Kind::Tritone => {
            let s = g(3) / 100.0;
            let (d, m, l) = (hsl(g(0), s, 0.12), hsl(g(1), s, 0.5), hsl(g(2), s, 0.9));
            map(img, move |c| {
                let t = luma(c);
                if t < 0.5 { std::array::from_fn(|q| d[q] + (m[q] - d[q]) * t * 2.0) } else { std::array::from_fn(|q| m[q] + (l[q] - m[q]) * (t - 0.5) * 2.0) }
            })
        }
        Kind::FalseColor => {
            let stops = FALSE_PALETTES[(g(0).round().max(0.0) as usize).min(5)];
            let (con, a) = (g(1) / 100.0, g(2) / 100.0);
            map(img, move |c| {
                let t = gradient_at(stops, ((luma(c) - 0.5) * (1.0 + con) + 0.5).clamp(0.0, 1.0));
                std::array::from_fn(|q| c[q] + (t[q] - c[q]) * a)
            })
        }
        Kind::ChannelSwap => {
            let s = SWAPS[(g(0).round().max(0.0) as usize).min(5)];
            map(img, move |c| [c[s[0]], c[s[1]], c[s[2]]])
        }
        Kind::GradientOverlay => {
            let (c1, c2, ang, op, mode) = (hsl(g(0), 0.9, 0.55), hsl(g(1), 0.9, 0.55), g(2).to_radians(), g(3) / 100.0, g(4).round() as u32);
            let (dx, dy) = (ang.cos(), ang.sin());
            let span = (wf * dx.abs() + hf * dy.abs()).max(1.0);
            map_xy(img, move |x, y, c| {
                let t = (((x - wf / 2.0) * dx + (y - hf / 2.0) * dy) / span + 0.5).clamp(0.0, 1.0);
                let col: [f32; 3] = std::array::from_fn(|q| c1[q] + (c2[q] - c1[q]) * t);
                let b: [f32; 3] = std::array::from_fn(|q| match mode {
                    0 => screen(c[q], col[q]),
                    1 => c[q] * col[q],
                    2 => c[q] + (col[q] - 0.5) * c[q] * (1.0 - c[q]) * 2.0,
                    _ => col[q],
                });
                std::array::from_fn(|q| c[q] + (b[q] - c[q]) * op)
            })
        }
        Kind::SkyGradient => {
            let stops = SKIES[(g(0).round().max(0.0) as usize).min(4)];
            let (horizon, sens) = (g(1) / 100.0, g(2) / 100.0);
            map_xy(img, move |_, y, c| {
                let yn = y / hf;
                if yn > horizon {
                    return c;
                }
                let (_, s) = hue_of(c);
                let l = luma(c);
                let blueish = c[2] > c[0] && c[2] > c[1] * 0.95;
                let sky = (((l - (0.75 - sens * 0.35)) * 6.0).clamp(0.0, 1.0) * ((0.3 - s) * 6.0).clamp(0.0, 1.0)).max(if blueish && l > 0.45 - sens * 0.2 { 0.8 } else { 0.0 });
                let fade = ((horizon - yn) / 0.08).clamp(0.0, 1.0);
                let col = gradient_at(stops, (yn / horizon.max(0.01)).clamp(0.0, 1.0));
                std::array::from_fn(|q| c[q] + (col[q] - c[q]) * sky * fade)
            })
        }
        Kind::LocalContrast => {
            let r = g(1) as usize;
            let bl = box_blur(img, r, r);
            let a = g(0) / 100.0;
            zip(img, &bl, move |c, b| std::array::from_fn(|q| c[q] + (c[q] - b[q]) * a))
        }
        Kind::DodgeBurn => {
            let bl = imageops::fast_blur(img, g(1));
            let a = g(0) / 100.0;
            zip(img, &bl, move |c, b| {
                let k = 1.0 + a * (luma(b) - 0.5) * 1.2;
                c.map(|v| v * k)
            })
        }
        Kind::Backlight => {
            let (sh, hi) = (g(0) / 100.0, g(1) / 100.0);
            map(img, move |c| {
                let l = luma(c).max(1e-3);
                let lift = sh * (1.0 - l).powi(3) * 0.6;
                let pull = hi * (l - 0.7).max(0.0) * 0.8;
                let nl = (l + lift - pull).clamp(0.0, 1.0);
                c.map(|v| v * nl / l)
            })
        }
        Kind::SoftGlare => {
            let (a, des) = (g(0) / 100.0, g(2) / 100.0);
            let bl = imageops::fast_blur(img, g(1));
            zip(img, &bl, move |c, b| {
                let o: [f32; 3] = std::array::from_fn(|q| screen(c[q], b[q] * a));
                let l = luma(o);
                o.map(|v| v + (l - v) * des)
            })
        }
        Kind::VignettePro => {
            let (amt, size, round, feather, mode) = (g(0) / 100.0, g(1) / 100.0, g(2) / 100.0, g(3) / 100.0, g(4).round() as u32);
            let col = match mode {
                0 => [0.0; 3],
                1 => [1.0; 3],
                _ => hsl(g(5), 0.7, 0.25),
            };
            map_xy(img, move |x, y, c| {
                let (nx, ny) = ((x / wf - 0.5) * 2.0, (y / hf - 0.5) * 2.0);
                let (sx, sy) = if round > 0.5 { let k = wf / hf; (nx * (1.0 + (k - 1.0) * (round - 0.5) * 2.0).max(0.2), ny) } else { (nx, ny) };
                let d = (sx * sx + sy * sy).sqrt() / std::f32::consts::SQRT_2;
                let t = ((d - (0.3 + size * 0.6)) / (0.05 + feather * 0.6)).clamp(0.0, 1.0);
                let t = t * t * (3.0 - 2.0 * t) * amt;
                std::array::from_fn(|q| c[q] + (col[q] - c[q]) * t)
            })
        }
        Kind::EdgeBlur => {
            let bl = imageops::fast_blur(img, g(0));
            let zone = g(1) / 100.0;
            let bl2 = bl.clone();
            map_xy(img, move |x, y, c| {
                let d = (((x / wf - 0.5) * 2.0).powi(2) + ((y / hf - 0.5) * 2.0).powi(2)).sqrt() / std::f32::consts::SQRT_2;
                let t = ((d - zone) / (1.0 - zone).max(0.05)).clamp(0.0, 1.0);
                let b = rgb(bl2.get_pixel(x as u32, y as u32).0);
                std::array::from_fn(|q| c[q] + (b[q] - c[q]) * t)
            })
        }
        Kind::Deband => {
            let a = g(0) / 100.0 * 3.0 / 255.0;
            map_xy(img, move |x, y, c| {
                let n = hash(x as u32, y as u32, 91) - hash(x as u32, y as u32, 92);
                c.map(|v| v + n * a)
            })
        }
        Kind::PopArt => {
            let (lv, sat, edges) = (g(0).round().max(2.0), g(1) / 100.0, g(2) / 100.0);
            let lv_ = lum_vec(img);
            map_xy(img, move |x, y, c| {
                let l = luma(c);
                let s: [f32; 3] = c.map(|v| (l + (v - l) * (1.0 + sat * 1.5)).clamp(0.0, 1.0));
                let o = s.map(|v| (v * (lv - 1.0)).round() / (lv - 1.0));
                let (gx, gy) = grad(&lv_, wu, hu, x as usize, y as usize);
                let e = ((gx.abs() + gy.abs()) * 5.0 * edges).min(1.0);
                o.map(|v| v * (1.0 - e))
            })
        }
        Kind::Warhol => {
            let (shift, sat) = (g(0), g(1) / 100.0);
            sample(img, move |x, y, at| {
                let (qx, qy) = (((x as f32 * 2.0 / wf) as u32).min(1), ((y as f32 * 2.0 / hf) as u32).min(1));
                let (sx, sy) = ((x as f32 * 2.0) % wf, (y as f32 * 2.0) % hf);
                let q = at(sx, sy);
                let l = luma([q[0], q[1], q[2]]);
                let lv = ((l * 3.0).round() / 3.0).clamp(0.08, 0.92);
                let o = hsl(((qy * 2 + qx) as f32) * shift + 20.0, sat, lv);
                [o[0], o[1], o[2], at(x as f32, y as f32)[3]]
            })
        }
        Kind::CmykHalftone => {
            let (cell, paper) = (g(0).max(2.0), g(1) >= 0.5);
            map_xy(img, move |x, y, c| {
                let k = 1.0 - c.iter().fold(0f32, |m, v| m.max(*v));
                let den = (1.0 - k).max(1e-3);
                let ink = [(1.0 - c[0] - k) / den, (1.0 - c[1] - k) / den, (1.0 - c[2] - k) / den, k];
                let mut o = if paper { [1.0f32; 3] } else { c };
                for (i, (ang, v)) in [(15f32, ink[0]), (75.0, ink[1]), (0.0, ink[2]), (45.0, ink[3])].into_iter().enumerate() {
                    let (s, co) = ang.to_radians().sin_cos();
                    let (u, w2) = (x * co + y * s, -x * s + y * co);
                    let (fu, fv) = ((u / cell).rem_euclid(1.0) - 0.5, (w2 / cell).rem_euclid(1.0) - 0.5);
                    let r = (fu * fu + fv * fv).sqrt();
                    if r < v.clamp(0.0, 1.0).sqrt() * 0.62 {
                        match i {
                            0 => o[0] *= 0.1,
                            1 => o[1] *= 0.1,
                            2 => o[2] *= 0.1,
                            _ => o = o.map(|t| t * 0.15),
                        }
                    }
                }
                o
            })
        }
        Kind::Engraving => {
            let (sp, ang, inkc) = (g(0).max(2.0), g(1).to_radians(), g(2).round() as u32);
            let ink = [[0.05, 0.05, 0.06], [0.3, 0.18, 0.08], [0.06, 0.1, 0.25]][inkc.min(2) as usize];
            let paper = [0.97, 0.95, 0.9];
            let (s, co) = ang.sin_cos();
            map_xy(img, move |x, y, c| {
                let u = x * co + y * s;
                let line = 0.5 + 0.5 * (u / sp * TAU).sin();
                if luma(c) < line { ink } else { paper }
            })
        }
        Kind::Crosshatch => {
            let (sp, con) = (g(0).max(2.0), g(1) / 100.0);
            map_xy(img, move |x, y, c| {
                let l = ((luma(c) - 0.5) * (1.0 + con) + 0.5).clamp(0.0, 1.0);
                let on = |a: f32, th: f32| -> bool {
                    let (s, co) = a.to_radians().sin_cos();
                    l < th && ((x * co + y * s) / sp).rem_euclid(1.0) < 0.18
                };
                let dark = on(45.0, 0.8) || on(-45.0, 0.6) || on(0.0, 0.4) || on(90.0, 0.2);
                if dark { [0.08; 3] } else { [0.97, 0.96, 0.93] }
            })
        }
        Kind::Charcoal => {
            let (t, con, grain) = (g(0) as usize, g(1) / 100.0, g(2) / 100.0);
            let bl = box_blur(img, t, t);
            let lv = lum_vec(&bl);
            map_xy(img, move |x, y, _| {
                let i = y as usize * wu + x as usize;
                let (gx, gy) = grad(&lv, wu, hu, x as usize, y as usize);
                let e = ((gx.abs() + gy.abs()) * 6.0).min(1.0);
                let base = ((lv[i] - 0.5) * (1.0 + con * 1.5) + 0.5).clamp(0.0, 1.0);
                let n = (fbm(x / 2.0, y / 2.0, 3) - 0.5) * grain * 0.4;
                let v = (base * (1.0 - e * 0.9) + n).clamp(0.0, 1.0);
                [v * 0.98, v * 0.97, v * 0.95]
            })
        }
        Kind::InkSketch => {
            let (thr, wash) = (0.02 + (1.0 - g(0) / 100.0) * 0.2, g(1) / 100.0);
            let lv = lum_vec(&box_blur(img, 1, 1));
            map_xy(img, move |x, y, c| {
                let (gx, gy) = grad(&lv, wu, hu, x as usize, y as usize);
                let e = (gx * gx + gy * gy).sqrt();
                let paper = [0.98, 0.97, 0.94];
                let base: [f32; 3] = std::array::from_fn(|q| paper[q] + (c[q] - paper[q]) * wash);
                if e > thr { [0.06, 0.06, 0.08] } else { base }
            })
        }
        Kind::LiquidChrome => {
            let (f, tint, amt) = (g(0), hsl(g(1), 1.0, 0.5), g(2) / 100.0);
            map(img, move |c| {
                let v = 0.5 + 0.5 * (luma(c) * f * TAU).sin();
                let v = v.powf(1.4);
                std::array::from_fn(|q| v + (tint[q] - 0.5) * amt * v)
            })
        }
        Kind::TileMosaic => {
            let (s, grout, gc) = (g(0).max(2.0), g(1), g(2) / 100.0);
            map_xy(img, |x, y, _| {
                let (fx, fy) = (x % s, y % s);
                if fx < grout || fy < grout {
                    return [gc; 3];
                }
                let col = px(img, (x / s).floor() * s + s / 2.0, (y / s).floor() * s + s / 2.0);
                let shade = 1.0 - 0.08 * ((fx - grout) / s + (fy - grout) / s);
                col.map(|v| v * shade)
            })
        }
        Kind::HexPixel => {
            let s = g(0).max(2.0);
            let (hw, hh) = (s * 3f32.sqrt(), s * 1.5);
            map_xy(img, |x, y, _| {
                let row = (y / hh).round();
                let mut best = (f32::MAX, 0.0, 0.0);
                for dr in -1..=1 {
                    let r = row + dr as f32;
                    let o = if (r as i32).rem_euclid(2) == 1 { hw / 2.0 } else { 0.0 };
                    let col = ((x - o) / hw).round();
                    for dc in -1..=1 {
                        let cx = (col + dc as f32) * hw + o;
                        let cy = r * hh;
                        let d = (x - cx).powi(2) + (y - cy).powi(2);
                        if d < best.0 {
                            best = (d, cx, cy);
                        }
                    }
                }
                px(img, best.1, best.2)
            })
        }
        Kind::LowPoly => {
            let (s, var) = (g(0).max(4.0), g(1) / 100.0);
            map_xy(img, |x, y, _| {
                let (ci, cj) = ((x / s).floor(), (y / s).floor());
                let jit = |i: f32, j: f32| ((hash((i + 9.0) as u32, (j + 9.0) as u32, 5) - 0.5) * s * var, (hash((i + 9.0) as u32, (j + 9.0) as u32, 6) - 0.5) * s * var);
                let v = |i: f32, j: f32| {
                    let (a, b) = jit(i, j);
                    (i * s + a, j * s + b)
                };
                let (fx, fy) = ((x - ci * s) / s, (y - cj * s) / s);
                let tri = if fx + fy < 1.0 { [v(ci, cj), v(ci + 1.0, cj), v(ci, cj + 1.0)] } else { [v(ci + 1.0, cj + 1.0), v(ci + 1.0, cj), v(ci, cj + 1.0)] };
                let (mx, my) = ((tri[0].0 + tri[1].0 + tri[2].0) / 3.0, (tri[0].1 + tri[1].1 + tri[2].1) / 3.0);
                px(img, mx, my)
            })
        }
        Kind::PixelArt => {
            let (ps, lv, outline) = (g(0).max(1.0), g(1).round().max(2.0), g(2) >= 0.5);
            let cell = |x: f32, y: f32| -> [f32; 3] { px(img, (x / ps).floor() * ps + ps / 2.0, (y / ps).floor() * ps + ps / 2.0).map(|v| (v * (lv - 1.0)).round() / (lv - 1.0)) };
            map_xy(img, |x, y, _| {
                let c = cell(x, y);
                if outline {
                    let n = [cell(x + ps, y), cell(x, y + ps)];
                    if n.iter().any(|o| luma(*o) - luma(c) > 0.25) {
                        return c.map(|v| v * 0.35);
                    }
                }
                c
            })
        }
        Kind::WaterReflection => {
            let (line, waves, tint) = (g(0) / 100.0 * hf, g(1), g(2) / 100.0);
            sample(img, move |x, y, at| {
                let yf = y as f32;
                let me = at(x as f32, yf);
                if yf < line {
                    return me;
                }
                let d = yf - line;
                let off = (d * 0.35).sin() * waves * (d / hf * 4.0).min(1.0);
                let q = at(x as f32 + off, line - d - 1.0);
                let dark = 0.75 - (d / hf).min(0.4);
                let water = [0.1, 0.25, 0.35];
                let o: [f32; 3] = std::array::from_fn(|k| (q[k] * dark) * (1.0 - tint) + water[k] * tint);
                [o[0], o[1], o[2], me[3]]
            })
        }
        Kind::CameraShake => {
            let (amp, ang, n) = (g(0), g(1).to_radians(), g(2).round().max(2.0) as i32);
            let (dx, dy) = (ang.cos() * amp, ang.sin() * amp);
            sample(img, move |x, y, at| {
                let mut acc = [0f32; 3];
                for i in 0..n {
                    let t = i as f32 / (n - 1) as f32 - 0.5;
                    let q = at(x as f32 + dx * t, y as f32 + dy * t);
                    for k in 0..3 {
                        acc[k] += q[k] / n as f32;
                    }
                }
                [acc[0], acc[1], acc[2], at(x as f32, y as f32)[3]]
            })
        }
        Kind::RadialChroma => {
            let k = g(0) / 100.0 * 0.03;
            sample(img, move |x, y, at| {
                let (cx, cy) = (wf / 2.0, hf / 2.0);
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                let r = at(cx + dx * (1.0 + k), cy + dy * (1.0 + k))[0];
                let b = at(cx + dx * (1.0 - k), cy + dy * (1.0 - k))[2];
                let me = at(x as f32, y as f32);
                [r, me[1], b, me[3]]
            })
        }
        Kind::ZoomBurst => {
            let (s, cx, cy, zone) = (g(0) / 100.0 * 0.25, g(1) / 100.0 * wf, g(2) / 100.0 * hf, g(3) / 100.0);
            sample(img, move |x, y, at| {
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                let d = (dx * dx + dy * dy).sqrt() / md;
                let k = ((d - zone * 0.5) / 0.3).clamp(0.0, 1.0) * s;
                let n = 20;
                let mut acc = [0f32; 3];
                for i in 0..n {
                    let t = 1.0 - k * i as f32 / n as f32;
                    let q = at(cx + dx * t, cy + dy * t);
                    for c in 0..3 {
                        acc[c] += q[c] / n as f32;
                    }
                }
                [acc[0], acc[1], acc[2], at(x as f32, y as f32)[3]]
            })
        }
        Kind::Anaglyph => {
            let (d, gray) = (g(0) / 2.0, g(1) >= 0.5);
            sample(img, move |x, y, at| {
                let (l, r) = (at(x as f32 - d, y as f32), at(x as f32 + d, y as f32));
                let me = at(x as f32, y as f32);
                if gray {
                    let (a, b) = (luma([l[0], l[1], l[2]]), luma([r[0], r[1], r[2]]));
                    [a, b, b, me[3]]
                } else {
                    [l[0], r[1], r[2], me[3]]
                }
            })
        }
        Kind::CineBars => {
            let (fmt, op) = (g(0).round() as u32, g(1) / 100.0);
            let ratio = [2.39, 2.0, 1.85, 4.0 / 3.0][fmt.min(3) as usize];
            map_xy(img, move |x, y, c| {
                let inside = if fmt == 3 {
                    let bw = ((wf - hf * ratio) / 2.0).max(0.0);
                    x >= bw && x <= wf - bw
                } else {
                    let bh = ((hf - wf / ratio) / 2.0).max(0.0);
                    y >= bh && y <= hf - bh
                };
                if inside { c } else { c.map(|v| v * (1.0 - op)) }
            })
        }
        Kind::PhotoFrame => {
            let (style, th) = (g(0).round() as u32, g(1) / 100.0 * md);
            map_xy(img, move |x, y, c| {
                let bottom = if style == 0 { th * 3.5 } else { th };
                let inside = x >= th && x <= wf - th && y >= th && y <= hf - bottom;
                match style {
                    4 => {
                        let band = th * 1.2;
                        if y < band || y > hf - band {
                            let hole = (x % (band * 0.9)) > band * 0.3 && (x % (band * 0.9)) < band * 0.6 && ((y - band * 0.5).abs() < band * 0.2 || (y - (hf - band * 0.5)).abs() < band * 0.2);
                            return if hole { [0.95, 0.93, 0.85] } else { [0.04, 0.03, 0.03] };
                        }
                        c
                    }
                    3 => {
                        if inside {
                            let edge = x < th + 3.0 || x > wf - th - 3.0 || y < th + 3.0 || y > hf - bottom - 3.0;
                            if edge { [0.82, 0.8, 0.76] } else { c }
                        } else {
                            [0.95, 0.94, 0.9]
                        }
                    }
                    _ if inside => c,
                    2 => [0.03; 3],
                    _ => [0.98, 0.98, 0.96],
                }
            })
        }
        Kind::Texture => {
            let (kind, amt, sc) = (g(0).round() as u32, g(1) / 100.0, (g(2) / 100.0).max(0.05));
            map_xy(img, move |x, y, c| {
                let (u, v) = (x / sc, y / sc);
                let t = match kind {
                    0 => fbm(u / 6.0, v / 6.0, 4) * 0.7 + hash(x as u32, y as u32, 1) * 0.3,
                    1 => 0.5 + 0.25 * ((u * 0.9).sin() * (v * 0.9).sin()) + (fbm(u / 3.0, v / 3.0, 2) - 0.5) * 0.4,
                    2 => fbm(u / 20.0, v / 20.0, 5) * 0.6 + hash(x as u32, y as u32, 2) * 0.4,
                    3 => 0.5 + (hash(x as u32, y as u32, 3) - 0.5) * 0.8,
                    _ => fbm(u / 40.0, v / 40.0, 6),
                };
                c.map(|v| v + (t - 0.5) * amt * 0.5 * (0.3 + v * (1.0 - v) * 2.8))
            })
        }
        Kind::Fireworks => {
            let (n, size, var, seed) = (g(0) as u32, g(1), g(2) / 100.0, g(3) as u32);
            let mut buf = vec![[0f32; 3]; wu * hu];
            for i in 0..n {
                let (cx, cy) = (hash(i, 1, seed) * wf, hash(i, 2, seed) * hf * 0.7);
                let base = hash(i, 3, seed) * 360.0;
                let r = size * (0.6 + hash(i, 4, seed) * 0.8);
                for ray in 0..48u32 {
                    let a = ray as f32 / 48.0 * TAU + hash(i, ray + 10, seed) * 0.1;
                    let col = hsl(base + (hash(i, ray + 60, seed) - 0.5) * var * 120.0, 1.0, 0.6);
                    for st in 0..14u32 {
                        let t = st as f32 / 14.0;
                        let (x, y) = (cx + a.cos() * r * t, cy + a.sin() * r * t + t * t * r * 0.15);
                        disk(&mut buf, wu, hu, x, y, 1.6 * (1.0 - t * 0.5), col.map(|v| v * (0.4 + t)), 0.6);
                    }
                }
            }
            overlay(img, buf, 1.0)
        }
        Kind::Confetti => {
            let (dens, size, seed) = (g(0) / 100.0, g(1), g(2) as u32);
            let mut buf: Vec<Option<[f32; 3]>> = vec![None; wu * hu];
            let n = (dens * (wu * hu) as f32 / (size * size * 6.0)).min(200_000.0) as u32;
            for i in 0..n {
                let (cx, cy) = (hash(i, 1, seed) * wf, hash(i, 2, seed) * hf);
                let col = hsl(hash(i, 3, seed) * 360.0, 0.85, 0.58);
                let a = hash(i, 4, seed) * PI;
                let (s, co) = a.sin_cos();
                let (hw, hh) = (size * 0.5, size * 0.25);
                let r = size.ceil() as i32;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let (u, v) = (dx as f32 * co + dy as f32 * s, -dx as f32 * s + dy as f32 * co);
                        let (x, y) = (cx as i32 + dx, cy as i32 + dy);
                        if u.abs() <= hw && v.abs() <= hh && x >= 0 && y >= 0 && (x as usize) < wu && (y as usize) < hu {
                            buf[y as usize * wu + x as usize] = Some(col);
                        }
                    }
                }
            }
            paint_buf(img, buf)
        }
        Kind::Glitter => {
            let (dens, size, thr, seed) = (g(0) / 100.0 * 0.004, g(1), g(2) / 100.0, g(3) as u32);
            let lv = lum_vec(img);
            let mut buf = vec![[0f32; 3]; wu * hu];
            let mut k = 0u32;
            for y in (0..hu).step_by(2) {
                for x in (0..wu).step_by(2) {
                    k += 1;
                    if lv[y * wu + x] > thr && hash(x as u32, y as u32, seed + 7) < dens * 4.0 {
                        let s = size * (0.4 + hash(k, 1, seed));
                        for t in 0..(s as i32) {
                            let f = 1.0 - t as f32 / s;
                            for (ox, oy) in [(t, 0), (-t, 0), (0, t), (0, -t)] {
                                let (xx, yy) = (x as i32 + ox, y as i32 + oy);
                                if xx >= 0 && yy >= 0 && (xx as usize) < wu && (yy as usize) < hu {
                                    let o = &mut buf[yy as usize * wu + xx as usize];
                                    for q in o.iter_mut() {
                                        *q += f * 0.9;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            overlay(img, buf, 1.0)
        }
        Kind::Bubbles => {
            let (n, size, op, seed) = (g(0) as u32, g(1), g(2) / 100.0, g(3) as u32);
            let mut buf = vec![[0f32; 3]; wu * hu];
            for i in 0..n {
                let (cx, cy, r) = (hash(i, 1, seed) * wf, hash(i, 2, seed) * hf, size * (0.4 + hash(i, 3, seed)));
                let (x0, x1) = ((cx - r).max(0.0) as usize, ((cx + r) as usize).min(wu.saturating_sub(1)));
                let (y0, y1) = ((cy - r).max(0.0) as usize, ((cy + r) as usize).min(hu.saturating_sub(1)));
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / r;
                        if d < 1.0 {
                            let rim = ((d - 0.82) / 0.18).clamp(0.0, 1.0);
                            let hl = (1.0 - (((x as f32 - (cx - r * 0.35)).powi(2) + (y as f32 - (cy - r * 0.35)).powi(2)).sqrt() / (r * 0.2))).max(0.0);
                            let tint = hsl(200.0 + d * 120.0, 0.6, 0.7);
                            let o = &mut buf[y * wu + x];
                            for q in 0..3 {
                                o[q] += tint[q] * rim * 0.6 + hl * 0.9 + 0.04;
                            }
                        }
                    }
                }
            }
            overlay(img, buf, op)
        }
        _ => img.clone(),
    }
}

#[cfg(test)]
pub fn option_count() -> usize {
    GROUPS.iter().flat_map(|g| g.1.iter()).map(|k| 1 + params(*k).iter().map(|p| p.choices.len().max(1)).sum::<usize>() + presets(*k).len()).sum()
}
