use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const MOTION: &[Tuto] = &[
    t("Composition", "", "Le cadre de ton animation : format, images par seconde, durée, fond.", &[
        "Choisis un format sur l'accueil (16:9, vertical, carré, bannière GIF).",
        "Menu Composition ou panneau de droite (sans calque sélectionné) pour la durée et le fond.",
    ]),
    t("Calques", "", "Texte, rectangle, ellipse, étoile, polygone, ligne, compteur, image.", &[
        "Clique une tuile dans Ajouter un calque.",
        "Glisse le calque dans la composition pour le placer.",
        "Dans la timeline, glisse la barre d'un calque ou ses bords pour changer quand il apparaît.",
    ]),
    t("Images clés", "◆", "Le cœur de l'animation : une valeur à un instant, Motion calcule le reste.", &[
        "Place la tête de lecture, clique le ◆ à côté d'une propriété (position, échelle, rotation, opacité).",
        "Avance la tête, change la valeur : une deuxième clé est créée automatiquement.",
        "Glisse les ◆ dans la timeline pour changer le rythme.",
    ]),
    t("Courbes d'accélération", "", "Linéaire, douce, accélère, ralentit, maintien, dépassement, élastique, rebond.", &[
        "Place la tête sur une image clé : le menu Courbe apparaît sous la propriété.",
        "Dépassement va un peu trop loin puis revient, Élastique oscille, Rebond rebondit comme une balle.",
    ]),
    t("Animations prêtes", "", "15 animations en un clic, à partir de la tête de lecture.", &[
        "Sélectionne un calque, place la tête, clique une animation (fondu, glissé, pop, tourbillon, élastique…).",
        "Elles créent de vraies images clés que tu peux ajuster ensuite.",
    ]),
    t("Machine à écrire", "", "Le texte apparaît lettre par lettre.", &["Sélectionne un calque texte, règle Machine à écrire dans Effets (durée en secondes).", "Ou applique l'animation prête Machine à écrire."]),
    t("Compteur", "", "Un nombre qui défile de A à B : abonnés, pourcentages, prix.", &["Ajoute un calque Compteur, règle De, À, décimales, préfixe et suffixe.", "Il compte pendant les 80 % premiers de la durée du calque, en ralentissant à la fin."]),
    t("Tremblement", "", "Mouvement aléatoire continu, comme une caméra à l'épaule.", &["Dans Effets, règle l'amplitude (pixels) et la fréquence (secousses par seconde)."]),
    t("Boucle", "", "Répète les images clés pendant toute la vie du calque.", &["Anime un cycle (par exemple une pulsation), puis coche Boucler les images clés."]),
    t("Effets", "", "Flou, ombre portée, contour, dégradé, mode de fusion.", &[
        "Le flou adoucit le calque, l'ombre portée le détache du fond.",
        "Contour : couleur et épaisseur autour de la forme ou du texte.",
        "Dégradé : de la couleur du calque vers une seconde couleur.",
        "Mode de fusion : Produit assombrit, Superposition éclaircit, Différence inverse.",
    ]),
    t("Fractionner", "Ctrl+Maj+D", "Coupe un calque en deux à la tête de lecture.", &["Sélectionne le calque, place la tête, Édition > Fractionner."]),
    t("Ordre des calques", "Ctrl+] / Ctrl+[", "Le calque du haut de la liste passe devant.", &["Édition > Monter ou Descendre le calque."]),
    t("Exporter", "Ctrl+M", "GIF animé, séquence PNG, ou image fixe.", &[
        "GIF : idéal pour le web et les messageries (960 px de large maximum).",
        "Séquence PNG : chaque image en pleine qualité, à importer dans Cut ou un autre logiciel.",
        "Exporter l'image courante pour une vignette.",
    ]),
];
