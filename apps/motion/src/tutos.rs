use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const MOTION: &[Tuto] = &[
    t("Composition", "", "Le cadre de ton animation : format, images par seconde, durée, fond.", &[
        "Choisis un format sur l'accueil (16:9, vertical, carré, bannière GIF).",
        "Menu Composition ou panneau de droite (sans calque sélectionné) pour la durée et le fond.",
    ]),
    t("Calques", "", "Texte, formes, ligne, flèche, anneau, compteur, image, fond.", &[
        "Clique une tuile dans Ajouter un calque.",
        "Glisse le calque dans la composition pour le placer.",
        "Dans la timeline, glisse la barre d'un calque ou ses bords pour changer quand il apparaît.",
    ]),
    t("Images clés", "◆", "Le cœur de l'animation : une valeur à un instant, Motion calcule le reste.", &[
        "Place la tête de lecture, clique le ◆ à côté d'une propriété (position, échelle, rotation, opacité, inclinaison, flou, révélation).",
        "Avance la tête, change la valeur : une deuxième clé est créée automatiquement.",
        "Glisse les ◆ dans la timeline pour changer le rythme.",
    ]),
    t("Courbes d'accélération", "F9", "14 courbes : douce, douce forte, expo, sinus, dépassement, élastique, rebond, saccadé…", &[
        "Place la tête sur une image clé : le menu Courbe apparaît sous la propriété.",
        "Dans la timeline, clic droit sur une clé pour changer sa courbe. Losange = linéaire, carré = maintien, sablier = courbe.",
        "F9 applique une courbe douce à toutes les clés du calque sélectionné.",
    ]),
    t("Nouvelles propriétés", "", "Inclinaison, flou animé et révélation.", &[
        "Inclinaison penche le calque comme une italique géante.",
        "Flou px s'anime : parfait pour une mise au point ou un titre qui sort du flou.",
        "Révélation % dévoile le calque de gauche à droite ; sur un anneau, c'est la longueur de l'arc.",
    ]),
    t("Animations prêtes", "", "27 animations classées : entrées, sorties, boucles, effets.", &[
        "Sélectionne un calque, place la tête, clique une animation. La recherche filtre la liste.",
        "Elles créent de vraies images clés que tu peux ajuster ensuite.",
    ]),
    t("Modèles", "", "Bandeau titre, titre cinéma, bouton S'abonner, compte à rebours, barre de progression, citation.", &[
        "Clique un modèle : ses calques arrivent déjà animés à la tête de lecture.",
        "Change ensuite les textes et les couleurs dans le panneau Contenu.",
    ]),
    t("Outils de la vue", "V / W / H", "Sélection, rotation et main.", &[
        "Sélection : glisse un calque pour le déplacer (Maj bloque l'axe), un coin pour l'échelle, le rond au-dessus pour la rotation.",
        "Rotation (W) : glisse n'importe où pour tourner le calque, Maj par pas de 15°.",
        "Main (H) ou clic molette : déplace la vue. Ctrl+molette zoome, double-clic revient à Ajuster.",
        "Ctrl+flèches déplace le calque de 1 px, Ctrl+Maj+flèches de 10 px.",
    ]),
    t("Grille, zones sûres, magnétisme", "", "Composer proprement.", &[
        "La grille montre les tiers, les zones sûres gardent titres et action loin des bords.",
        "Avec le magnétisme, un calque déplacé s'accroche au centre et aux bords (ligne rose), et les clés s'accrochent aux repères et à la tête.",
        "Demi-résolution rend l'aperçu plus fluide sur les grosses compositions.",
    ]),
    t("Parent", "", "Un calque suit les mouvements d'un autre.", &[
        "Dans Calque > Parent, choisis le calque à suivre : position, échelle et rotation du parent s'appliquent à l'enfant.",
        "Le calque ne bouge pas au moment où tu l'attaches. Exemple : le texte du bouton S'abonner suit le bouton.",
    ]),
    t("Masque", "", "Voir un calque à travers la forme d'un autre.", &[
        "Place le calque pochoir juste au-dessus dans la timeline.",
        "Sur le calque du dessous, choisis Masque > Alpha (la forme) ou Luminance (le clair laisse passer).",
        "Idéal pour un dégradé ou une image visible seulement à travers un texte.",
    ]),
    t("Machine à écrire", "", "Le texte apparaît lettre par lettre.", &["Sélectionne un calque texte, règle Machine à écrire dans Mouvement (durée en secondes).", "Ou applique l'animation prête Machine à écrire."]),
    t("Compteur", "", "Un nombre qui défile de A à B : abonnés, pourcentages, prix.", &["Ajoute un calque Compteur, règle De, À, décimales, préfixe et suffixe.", "Il compte pendant les 80 % premiers de la durée du calque, en ralentissant à la fin."]),
    t("Tremblement", "", "Mouvement aléatoire continu, comme une caméra à l'épaule.", &["Dans Mouvement, règle l'amplitude (pixels) et la fréquence (secousses par seconde)."]),
    t("Boucle et aller-retour", "", "Répète les images clés pendant toute la vie du calque.", &["Anime un cycle (par exemple une pulsation), puis Mouvement > Répéter : Boucle.", "Aller-retour rejoue l'animation à l'envers puis à l'endroit, sans à-coup."]),
    t("Écho et répéteur", "", "Traînées et motifs à partir d'un seul calque.", &[
        "Écho dessine des copies du calque dans le passé : une traînée qui suit le mouvement.",
        "Répéteur crée des copies décalées, tournées, réduites et estompées : rosaces, rangées, spirales.",
    ]),
    t("Effets", "", "Flou, ombre, lueur, couleur.", &[
        "Ombre portée : décalage, douceur, opacité et couleur réglables.",
        "Lueur : un halo coloré autour du calque, superbe sur du texte ou un anneau.",
        "Teinte, saturation et luminosité recolorent le calque, même une image.",
    ]),
    t("Apparence", "", "Contour, pointillés, dégradé, fusion, miroir.", &[
        "Contour : couleur, épaisseur et pointillés autour de la forme ou du texte.",
        "Dégradé : de la couleur du calque vers une seconde couleur, avec un angle.",
        "Mode de fusion : Produit assombrit, Superposition éclaircit, Lumière tamisée adoucit.",
    ]),
    t("Texte avancé", "", "Italique, alignement et espacement des lettres.", &["Gauche ou Droite cale le texte sur son point de position, pratique pour les bandeaux.", "L'espacement des lettres donne un look cinéma aux titres en capitales."]),
    t("Calques : afficher, verrouiller, solo", "", "Les trois icônes devant chaque calque.", &[
        "L'œil masque le calque, le cadenas empêche de le déplacer par erreur, Solo n'affiche que les calques en solo.",
        "Clic droit sur un calque : dupliquer, copier l'animation, premier plan, couleur d'étiquette…",
    ]),
    t("Repères et zone de travail", "M / B / N", "Marquer les temps forts et limiter la lecture.", &[
        "M pose un repère à la tête de lecture ; J et K sautent d'une clé ou d'un repère à l'autre.",
        "B et N fixent le début et la fin de la zone de travail : la lecture boucle dedans et l'export peut s'y limiter.",
    ]),
    t("Copier une animation", "", "Réutiliser le même mouvement sur un autre calque.", &["Édition > Copier l'animation sur le calque modèle.", "Sélectionne un autre calque, place la tête, Coller l'animation : les clés partent de la tête de lecture."]),
    t("Fractionner", "Ctrl+Maj+D", "Coupe un calque en deux à la tête de lecture.", &["Sélectionne le calque, place la tête, Calque > Fractionner."]),
    t("Ordre des calques", "Ctrl+] / Ctrl+[", "Le calque du haut de la liste passe devant.", &["Calque > Monter, Descendre, Premier plan ou Arrière-plan."]),
    t("Composition", "Ctrl+K", "Format, images par seconde, durée et fond.", &["Clique la pastille en haut à droite. Ajuster à l'animation coupe la durée juste après la dernière clé.", "Le fond peut être un dégradé vertical."]),
    t("Exporter", "Ctrl+M", "MP4, GIF, séquence PNG, image PNG ou SVG.", &[
        "MP4 : la vidéo à publier partout. GIF : léger, pour le web et les messages.",
        "Choisis la taille (25 à 200 %), la plage (toute la composition ou la zone de travail) et le fond transparent pour PNG et GIF.",
        "SVG exporte l'image affichée en vectoriel, modifiable dans Vector.",
    ]),
];
