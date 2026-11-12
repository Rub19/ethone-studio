use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const PIXEL: &[Tuto] = &[
    t("Suppression", "J", "Efface un objet (oiseau, passant, câble) et reconstruit le fond autour comme s'il n'avait jamais existé.", &[
        "Choisis l'outil Suppression (J) dans la barre d'outils.",
        "Règle la taille du pinceau un peu plus large que l'objet avec [ et ].",
        "Peins sur l'objet : il se couvre de rouge. Déborde légèrement sur le fond.",
        "Relâche : Pixel analyse les textures voisines (ciel, herbe, mur) et remplit la zone en quelques secondes.",
        "Si le résultat ne te plaît pas, Ctrl+Z puis repasse : chaque essai donne une variante différente.",
    ]),
    t("Remplir selon le contenu", "", "Même moteur que Suppression, appliqué à une sélection rectangulaire, elliptique ou au lasso.", &[
        "Sélectionne la zone à faire disparaître avec un outil de sélection.",
        "Menu Édition > Remplir selon le contenu.",
        "Garde un peu de marge autour de l'objet pour un raccord invisible.",
    ]),
    t("Déplacement", "V", "Déplace tout le contenu du calque actif.", &[
        "Choisis le calque dans le panneau Calques.",
        "Glisse sur l'image avec l'outil Déplacement (V).",
    ]),
    t("Sélection rectangulaire", "M", "Délimite une zone rectangulaire : peinture, filtres et réglages ne toucheront qu'elle.", &[
        "Glisse sur l'image pour tracer le rectangle.",
        "Ctrl+A sélectionne tout, Ctrl+D désélectionne.",
        "Sélection > Intervertir pour travailler sur tout le reste.",
    ]),
    t("Sélection elliptique", "Maj+M", "Comme la sélection rectangulaire, mais en ellipse ou en cercle.", &[
        "Garde le bouton de la sélection appuyé (clic droit) et choisis Sélection elliptique, ou appuie sur Maj+M.",
        "Glisse sur l'image : l'ellipse s'inscrit dans le cadre tracé.",
    ]),
    t("Lasso", "L", "Sélection à main levée en suivant le contour d'un objet.", &[
        "Maintiens le clic et fais le tour de l'objet.",
        "Relâche : le contour se referme tout seul.",
    ]),
    t("Baguette magique", "W", "Sélectionne d'un clic les pixels de couleur proche.", &[
        "Clique sur une couleur (un ciel uni par exemple).",
        "Augmente la tolérance pour inclure plus de nuances.",
        "Décoche « Contiguë » pour prendre la couleur partout dans l'image.",
    ]),
    t("Recadrage", "C", "Coupe l'image au cadre choisi.", &[
        "Glisse pour tracer le cadre à garder.",
        "Clique sur Valider ou appuie sur Entrée. Échap annule.",
    ]),
    t("Pipette", "I", "Prélève une couleur dans l'image comme couleur principale.", &["Clique sur l'image : la couleur principale change aussitôt."]),
    t("Tampon de duplication", "S", "Recopie une partie de l'image ailleurs, idéal pour les retouches fines.", &[
        "Alt+clic sur la zone source à copier.",
        "Peins ailleurs : la source suit ton pinceau avec le même décalage.",
        "Un nouvel Alt+clic change la source.",
    ]),
    t("Pinceau", "B", "Peint avec la couleur principale, bords doux ou durs.", &[
        "Règle Taille, Dureté et Flux dans la barre d'options.",
        "[ et ] changent la taille sans quitter l'image.",
        "X inverse les couleurs principale et secondaire, D remet noir et blanc.",
    ]),
    t("Crayon", "N", "Trait net sans anticrénelage, parfait pour le pixel art.", &["Zoome fort (Ctrl+molette) et dessine pixel par pixel avec une petite taille."]),
    t("Gomme", "E", "Rend les pixels transparents sur le calque actif.", &["Utilise un flux faible pour effacer progressivement."]),
    t("Dégradé", "G", "Fond de la couleur principale vers la secondaire.", &["Glisse sur l'image : la direction et la longueur du trait définissent le dégradé.", "Fais-le dans une sélection pour le limiter."]),
    t("Pot de peinture", "K", "Remplit une zone de couleur proche.", &["Clique dans la zone à remplir.", "La tolérance règle jusqu'où la couleur se propage."]),
    t("Goutte", "R", "Adoucit localement : peau, arrière-plan, bords trop durs.", &["Passe plusieurs fois pour renforcer l'effet."]),
    t("Netteté", "", "Renforce les détails là où tu peins.", &["Utilise-le avec parcimonie sur les yeux ou les textures."]),
    t("Doigt", "", "Étale la peinture comme un doigt sur une toile fraîche.", &["Glisse lentement : la couleur de départ est entraînée."]),
    t("Densité −", "O", "Éclaircit les zones peintes (dodge).", &["Idéal pour faire ressortir des reflets ou des yeux."]),
    t("Densité +", "", "Assombrit les zones peintes (burn).", &["Pour creuser des ombres et donner du volume."]),
    t("Éponge", "", "Sature ou désature localement.", &["Choisis Saturer ou Désaturer dans la barre d'options, puis peins."]),
    t("Rectangle", "U", "Dessine un rectangle plein de la couleur principale.", &["Glisse sur l'image. Par défaut, chaque forme va sur son propre calque."]),
    t("Ellipse", "", "Dessine une ellipse pleine et lissée.", &["Glisse sur l'image pour définir son cadre."]),
    t("Trait", "", "Trace une ligne droite à l'épaisseur du pinceau.", &["Glisse d'un point à l'autre."]),
    t("Texte", "T", "Ajoute du texte sur un nouveau calque.", &["Clique à l'endroit voulu.", "Tape le texte, choisis la taille, puis Ajouter.", "Déplace-le ensuite avec l'outil Déplacement."]),
    t("Main", "H", "Fait défiler l'image. Espace l'active temporairement.", &["Garde Espace enfoncé et glisse, depuis n'importe quel outil."]),
    t("Zoom", "Z", "Agrandit ou réduit l'affichage.", &["Clic pour zoomer, Alt+clic pour dézoomer.", "Ctrl+0 ajuste à l'écran, Ctrl+1 affiche à 100 %."]),
    t("Réglages", "", "Niveaux, exposition, vibrance, température, tons foncés… avec aperçu en direct.", &[
        "Menu Image > Réglages et choisis un réglage.",
        "Bouge les curseurs : l'image se met à jour en direct.",
        "Appliquer valide, Annuler revient à l'original.",
        "Avec une sélection active, seul l'intérieur est modifié.",
    ]),
    t("Filtres", "", "Flous, netteté, bruit, mosaïque, lueur, vignettage, aberration chromatique…", &[
        "Menu Filtre, puis une catégorie.",
        "Règle l'intensité avec l'aperçu en direct.",
        "Duplique le calque avant (Ctrl+J) pour pouvoir doser avec l'opacité.",
    ]),
    t("Modes de fusion", "", "Mélange un calque avec ceux du dessous : produit, superposition, incrustation…", &[
        "Sélectionne un calque.",
        "Dans Propriétés, choisis le mode : Produit assombrit, Superposition éclaircit, Incrustation renforce le contraste.",
        "Combine avec l'opacité pour doser.",
    ]),
    t("Calques", "Ctrl+Maj+N", "Empile des éléments indépendants pour retoucher sans détruire.", &[
        "+ crée un calque, ⧉ le duplique, ⤓ le fusionne avec celui du dessous.",
        "Double-clique sur le nom pour le renommer, l'œil pour le masquer.",
    ]),
    t("Copier et coller", "Ctrl+C / Ctrl+V", "Copie une sélection, colle une capture d'écran ou une image d'une autre app.", &["Ctrl+C copie la sélection (ou le calque), aussi dans le presse-papiers de Windows.", "Ctrl+V colle l'image du presse-papiers sur un nouveau calque, même une capture faite avec Win+Maj+S.", "Ctrl+J crée un calque par copie de la sélection, Ctrl+Maj+J par coupure."]),
    t("Transformation", "Ctrl+T", "Agrandit, réduit ou fait pivoter le calque actif.", &["Ctrl+T, puis règle l'échelle et l'angle."]),
    t("Effets de calque", "", "Ombre portée, lueur externe, contour.", &["Calque > Effets de calque, choisis l'effet, sa taille et sa couleur.", "L'effet est posé sur un nouveau calque sous le calque actif : tu peux l'effacer ou baisser son opacité."]),
    t("Zone de travail", "", "Agrandit la toile sans étirer l'image, ou rogne les bords vides.", &["Clique la taille du document en haut à droite, ou Image > Taille de la zone de travail.", "Choisis où placer l'image actuelle dans la grille 3 × 3.", "Image > Rogner les bords transparents enlève le vide autour d'un détourage."]),
    t("Sélection avancée", "", "Agrandir, réduire, bordure, pixels du calque, remplir, contour.", &["Menu Sélection pour modifier la sélection de quelques pixels.", "Maj+F5 remplit la sélection avec la couleur principale.", "Édition > Contour de la sélection trace un liseré."]),
    t("Symétrie", "", "Peint en miroir : visages, mandalas, logos.", &["Avec un pinceau, coche Symétrie dans la barre d'options : chaque trait est répété de l'autre côté."]),
    t("Navigateur et infos", "", "Vue d'ensemble, zoom, couleur sous le curseur, histogramme.", &["Clique dans le Navigateur pour aller à un endroit, règle le zoom avec le curseur.", "Le panneau Infos lit la couleur (hex et RVB) sous le curseur.", "Affichage > Grille (Ctrl+') pour aligner ; la grille de pixels apparaît dès 1000 %."]),
    t("Exporter", "Ctrl+E", "Enregistre l'image aplatie en PNG, JPEG ou WebP.", &["Fichier > Exporter et choisis le format avec l'extension."]),
];
