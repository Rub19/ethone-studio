use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const VECTOR: &[Tuto] = &[
    t("Sélection", "V", "Sélectionne, déplace et redimensionne les objets ; modifie les points d'un tracé.", &[
        "Clique un objet pour le sélectionner, glisse pour le déplacer.",
        "Tire un coin pour redimensionner, Maj garde les proportions.",
        "Sur un tracé, glisse un point carré ou une poignée ronde pour changer la courbe. Maj casse la symétrie des poignées.",
        "Flèches du clavier : décalage de 1 px, Maj+flèches : 10 px.",
    ]),
    t("Plume", "P", "Dessine des courbes de Bézier précises, point par point.", &[
        "Clic : point d'angle. Clic-glisser : point courbe avec ses poignées.",
        "Clique sur le premier point pour fermer la forme.",
        "Entrée ou Échap termine un tracé ouvert.",
    ]),
    t("Crayon", "N", "Dessin à main levée, lissé automatiquement en courbes propres.", &[
        "Dessine librement en maintenant le clic.",
        "Le réglage Lissage simplifie plus ou moins le tracé obtenu.",
        "Termine près du point de départ pour fermer la forme.",
    ]),
    t("Rectangle", "M", "Rectangles et carrés, avec coins arrondis réglables.", &["Glisse pour dessiner, Maj pour un carré.", "Règle l'arrondi dans le panneau Rectangle."]),
    t("Ellipse", "L", "Ellipses et cercles.", &["Glisse pour dessiner, Maj pour un cercle parfait."]),
    t("Polygone", "Y", "Polygones réguliers de 3 à 24 côtés.", &["Choisis le nombre de côtés dans la barre d'options.", "Glisse depuis le centre : la distance définit la taille, la direction n'importe pas."]),
    t("Étoile", "S", "Étoiles à branches réglables.", &["Choisis le nombre de branches et le creux.", "Glisse depuis le centre."]),
    t("Ligne", "\\", "Segments droits.", &["Glisse d'un point à l'autre, Maj contraint par pas de 45°."]),
    t("Flèche", "", "Flèches pleines prêtes pour les schémas et miniatures.", &["Règle l'épaisseur dans la barre d'options.", "Glisse de la base vers la pointe."]),
    t("Spirale", "", "Spirales lisses à nombre de tours réglable.", &["Choisis le nombre de tours, puis glisse depuis le centre."]),
    t("Cœur", "", "Forme de cœur vectorielle.", &["Glisse pour définir son cadre, Maj pour garder les proportions."]),
    t("Texte", "T", "Ajoute du texte, modifiable à tout moment.", &["Clique sur le plan de travail.", "Change le texte, la police, la taille et le gras dans le panneau Texte."]),
    t("Rotation", "R", "Fait pivoter l'objet sélectionné autour de son centre.", &[
        "Sélectionne un objet puis prends l'outil Rotation.",
        "Glisse autour de l'objet ; Maj tourne par pas de 15°.",
        "Les rectangles et ellipses deviennent des tracés pour pouvoir tourner. Le texte ne pivote pas.",
    ]),
    t("Pipette", "I", "Copie le style (fond, contour, épaisseur) d'un objet.", &["Avec un objet sélectionné : clique un autre objet pour lui prendre son style.", "Sans sélection : le style devient celui des nouveaux objets."]),
    t("Main", "H", "Fait défiler le plan de travail. Espace l'active temporairement.", &["Garde Espace enfoncé et glisse."]),
    t("Zoom", "Z", "Agrandit ou réduit l'affichage.", &["Clic pour zoomer, Alt+clic pour dézoomer, Ctrl+molette partout."]),
    t("Dégradé", "", "Remplit une forme avec un dégradé linéaire entre deux couleurs.", &["Sélectionne un objet avec un fond.", "Dans Apparence, active Dégradé et choisis la deuxième couleur.", "Règle l'angle pour l'orienter."]),
    t("Pointillés", "", "Contour en tirets.", &["Active un contour, puis règle Pointillés dans Apparence (0 = trait plein)."]),
    t("Ombre portée et flou", "", "Effets non destructifs appliqués à l'objet.", &["Dans Apparence, coche Ombre portée.", "Le curseur Flou adoucit l'objet entier : idéal pour des lueurs."]),
    t("Alignement", "", "Aligne l'objet sur le plan de travail.", &["Avec l'outil Sélection, utilise les six boutons d'alignement de la barre d'options."]),
    t("Répétition", "", "Crée une série de copies décalées, tournées ou non.", &["Objet > Répétition…", "Choisis le nombre de copies, le décalage X/Y et la rotation par copie.", "Parfait pour les motifs, rosaces et frises."]),
    t("Tracés", "", "Convertir, lisser, simplifier, inverser.", &[
        "Objet > Convertir en tracé transforme un rectangle ou une ellipse en points modifiables.",
        "Lisser arrondit tous les angles, Simplifier retire les points inutiles.",
        "Inverser le sens change le départ du tracé.",
    ]),
    t("Grille et magnétisme", "Ctrl+'", "Aligne le dessin sur une grille régulière.", &["Affichage > Grille pour la voir, Magnétisme pour que les points s'y collent.", "La taille de la grille se règle dans le même menu."]),
    t("Verrouillage", "Ctrl+2", "Empêche de sélectionner ou déplacer un objet par erreur.", &["Objet > Verrouiller, ou le cadenas dans le panneau Calques."]),
    t("Image", "", "Place une photo ou un logo PNG/JPEG dans le dessin.", &["Fichier > Importer une image…", "Redimensionne-la avec les coins comme n'importe quel objet."]),
    t("Sélection multiple", "Maj+clic", "Travaille sur plusieurs objets à la fois.", &["Maj+clic ajoute ou retire un objet, glisse dans le vide pour sélectionner une zone, Ctrl+A tout.", "Déplace, duplique, supprime ou aligne-les ensemble ; Répartir les espace régulièrement.", "Édition > Sélectionner même fond attrape tous les objets de la même couleur."]),
    t("Copier et coller", "Ctrl+C / Ctrl+V", "Duplique des objets, aussi au même endroit.", &["Ctrl+V colle décalé, Ctrl+Maj+V colle exactement en place."]),
    t("Formes prêtes", "", "Bulle, croix, éclair, repère, nuage, coche, engrenage, goutte, lune, badge.", &["Sans rien sélectionner, clique une forme dans le panneau Formes prêtes : elle arrive au centre."]),
    t("Nuancier", "", "Six palettes harmonieuses prêtes à l'emploi.", &["Clic sur une pastille : couleur de fond des nouveaux objets. Clic droit : couleur de contour."]),
    t("Points d'ancrage", "Alt / Ctrl+clic", "Retouche fine des tracés.", &["Alt+clic sur un point le supprime, Ctrl+clic le passe d'angle à arrondi et inversement.", "Objet > Ajouter des points double le nombre de points sans changer la forme."]),
    t("Contour avancé", "", "Tirets, bouts, angles, pointe de flèche.", &["Dans Apparence : pointillés et espace, bouts ronds/plats/carrés, angles ronds/pointus/biseautés.", "Sur un tracé ouvert, coche Pointe de flèche."]),
    t("Mode contours", "Ctrl+Maj+Y", "Affiche le squelette du dessin pour repérer les objets cachés.", &["Affichage > Mode contours. Les repères (centre et tiers) aident à composer."]),
    t("Exporter", "Ctrl+E", "SVG pour le web, PNG haute définition pour le reste.", &["Fichier > Exporter en SVG ou en PNG ×1, ×2, ×4."]),
];
