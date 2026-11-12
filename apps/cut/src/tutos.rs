use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const CUT: &[Tuto] = &[
    t("Importer des médias", "Ctrl+I", "Vidéos (MP4, MOV, MKV…) et sons (MP3, WAV, M4A…).", &[
        "Fichier > Importer des médias, ou glisse les fichiers sur la fenêtre.",
        "Les vidéos se placent à la suite sur V1, les sons sur A2 à la tête de lecture.",
        "Depuis le panneau Médias, clique + pour ajouter à nouveau un média.",
    ]),
    t("Lecture et navigation", "Espace", "Lis, avance image par image, saute aux extrémités.", &[
        "Espace lit ou met en pause. J recule d'une seconde, K pause, L lit.",
        "Flèches gauche/droite : une image. Origine/Fin : début et fin.",
        "Flèches haut/bas : coupe ou marqueur précédent / suivant.",
        "Clique dans la règle pour placer la tête de lecture.",
    ]),
    t("Sélection", "V", "Sélectionne, déplace et rogne les clips.", &[
        "Clique un clip pour voir ses réglages à droite.",
        "Glisse le milieu d'un clip pour le déplacer dans la séquence.",
        "Glisse un bord pour rogner l'entrée ou la sortie.",
    ]),
    t("Lame", "C", "Coupe un clip à l'endroit cliqué.", &["Prends la Lame (C) et clique sur un clip.", "S coupe directement à la tête de lecture."]),
    t("Rogner à la tête de lecture", "Q / W", "Raccourcis de montage rapide, comme dans les logiciels pros.", &["Q supprime le début du clip jusqu'à la tête de lecture.", "W supprime la fin du clip à partir de la tête de lecture.", "Le reste de la séquence se recolle automatiquement."]),
    t("Dupliquer", "Ctrl+D", "Copie le clip sélectionné juste après lui.", &["Sélectionne un clip puis Ctrl+D."]),
    t("Vitesse", "", "Ralenti ou accéléré, de 0,25× à 4×.", &["Sélectionne un clip, règle Vitesse dans l'inspecteur.", "La durée sur la timeline s'adapte ; le son suit la même vitesse."]),
    t("Couleur", "", "Luminosité, contraste, saturation, température et looks.", &[
        "Sélectionne un clip et ouvre le panneau Couleur.",
        "Choisis un look (Noir et blanc, Sépia, Vintage, Cinéma, Vif) puis affine.",
        "L'aperçu du programme se met à jour aussitôt.",
    ]),
    t("Cadrage", "", "Zoom et miroir de l'image.", &["Échelle au-delà de 100 % recadre le centre, idéal pour passer en vertical.", "Miroir inverse gauche et droite."]),
    t("Fondus", "", "Entrées et sorties en douceur, au noir ou au blanc.", &["Règle Fondu d'entrée et de sortie dans Son et fondus.", "Coche Fondu au blanc pour un flash lumineux."]),
    t("Titres", "T", "Textes à l'écran : titre, sous-titre, tiers inférieur.", &[
        "Titre > Nouveau titre (T) l'ajoute à la tête de lecture sur la piste T1.",
        "Dans l'inspecteur : texte, taille, position, couleur, bandeau.",
        "Glisse le titre sur T1 pour changer son moment, règle sa durée dans l'inspecteur.",
    ]),
    t("Marqueurs", "M", "Repères sur la règle pour caler le montage sur la musique.", &["M ajoute un marqueur à la tête de lecture.", "Flèches haut/bas sautent de marqueur en marqueur.", "Édition > Supprimer les marqueurs pour tout effacer."]),
    t("Musique", "", "Piste A2 : volume et fondus indépendants.", &["Glisse un clip de musique pour le décaler.", "Règle volume, fondu d'entrée et de sortie dans l'inspecteur.", "Le haut-parleur de la piste coupe tout le son de la piste."]),
    t("Format de séquence", "", "16:9, 4K, vertical pour les Shorts, carré.", &["Séquence > choisis le format et la fréquence d'images.", "Les clips sont adaptés automatiquement au cadre."]),
    t("Exporter", "Ctrl+M", "MP4 H.264 + AAC, prêt pour YouTube et les réseaux.", &["Fichier > Qualité d'export : Légère, Standard ou Haute.", "Exporter en MP4… et choisis l'emplacement ; la progression s'affiche en bas.", "Fichier > Exporter l'image affichée pour une miniature PNG."]),
];
