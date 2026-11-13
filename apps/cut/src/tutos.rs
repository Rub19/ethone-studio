use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const CUT: &[Tuto] = &[
    t("Importer des médias", "Ctrl+I", "Vidéos, photos (PNG, JPG…) et sons (MP3, WAV, M4A…).", &[
        "Fichier > Importer des médias, ou glisse les fichiers sur la fenêtre.",
        "Les vidéos et photos se placent à la suite sur V1, les sons sur A2 à la tête de lecture.",
        "Clic droit sur un média : insérer à la tête, incruster sur V2, remplacer le clip, étiquette, remplacer le fichier.",
        "Le bouton ✦ crée un élément : cache couleur, dégradé, mire de barres, damier, bruit, fond vert.",
    ]),
    t("Panneau Médias", "", "Recherche, filtres, tri, étiquettes.", &[
        "Filtre Tout / Vidéo / Son / Image, tri ⇅ par nom, durée, type ou étiquette.",
        "Un point vert signale un média utilisé dans la timeline ; la vue liste indique combien de fois.",
        "Fichier > Retirer les médias inutilisés fait le ménage.",
    ]),
    t("Lecture et navigation", "Espace", "Lis, avance image par image, navette JKL.", &[
        "Espace lit ou met en pause. L lit en avant, J en arrière ; appuie encore pour aller 2×, 4×, 8×. K arrête.",
        "Flèches gauche/droite : une image (Maj : une seconde). Origine/Fin : début et fin.",
        "Flèches haut/bas : point de montage précédent / suivant. Maj+haut/bas : marqueur précédent / suivant.",
        "Affichage > Qualité de lecture 1/2 ou 1/4 pour une lecture plus fluide sur les gros projets.",
    ]),
    t("Outils groupés", "V B C Y P H T", "Comme dans Premiere : les outils voisins partagent un emplacement.", &[
        "Clic sur un outil pour le prendre ; clic droit, clic maintenu ou petit triangle pour voir les autres.",
        "Appuie plusieurs fois sur la même touche pour passer d'un outil du groupe à l'autre.",
        "Groupes : Sélection (V/A), Propagation (B/N/R), Cutter (C), Glissement (Y/U), Plume (P), Main (H/Z), Texte (T).",
    ]),
    t("Sélection", "V", "Sélectionne, déplace et rogne les clips.", &[
        "Clic : sélectionne. Ctrl+clic : ajoute ou retire de la sélection. Maj+clic : plage de clips.",
        "Glisse le milieu d'un clip V1 pour le déplacer ; un clip V2, un titre ou un son se déplacent librement.",
        "Glisser un bord rogne sans bouger le reste : un espace vide apparaît. Clique l'espace puis Suppr pour le refermer.",
        "Dans le moniteur, glisse pour déplacer le clip ou le titre sélectionné.",
    ]),
    t("Sélection de piste", "A / Maj+A", "Sélectionne tout ce qui suit (ou précède) un clip.", &[
        "A : clique un clip pour sélectionner ce clip et tout ce qui vient après, sur toutes les pistes.",
        "Glisse-le pour ouvrir ou refermer un espace avant lui : titres, sons et V2 suivent.",
        "Maj+A fait la même chose vers l'arrière.",
    ]),
    t("Propagation, déplacement, allongement", "B / N / R", "Les trois outils de montage fin.", &[
        "Propagation (B) : rogne un bord et décale tout ce qui suit, sans laisser de vide.",
        "Modification par déplacement (N) : glisse une coupe entre deux clips ; la durée totale ne change pas.",
        "Allongement de la durée (R) : tire un bord pour accélérer ou ralentir le clip.",
    ]),
    t("Cutter", "C / Maj+C", "Coupe un clip à l'endroit cliqué.", &["Cutter (C) coupe la piste cliquée.", "Couper toutes les pistes (Maj+C) coupe V1, V2, titres et sons au même instant.", "S ou Ctrl+K coupent à la tête de lecture, Ctrl+Maj+K sur toutes les pistes."]),
    t("Glissement et déplacement", "Y / U", "Ajuste un clip sans changer la durée du montage.", &["Glissement (Y) : change la partie de la vidéo montrée dans le clip.", "Déplacement (U) : déplace le clip entre ses voisins, qui s'allongent ou raccourcissent."]),
    t("Rogner, lever, extraire", "Q / W", "Raccourcis de montage rapide.", &["Q supprime le début du clip jusqu'à la tête, W la fin.", "Maj+Suppr supprime et referme ; Alt+Suppr lève le clip en laissant un vide.", "Séquence > Supprimer tous les espaces vides recolle tout."]),
    t("Vitesse et durée", "Ctrl+R", "Ralenti, accéléré, lecture inversée, image figée.", &[
        "Ctrl+R ouvre Vitesse/Durée : vitesse en %, durée exacte, lecture inversée, maintien.",
        "Raccourcis 10 % à 800 % dans l'onglet Clip de l'inspecteur.",
        "Ctrl+Maj+F fige l'image 2 secondes à la tête de lecture.",
    ]),
    t("Transitions", "Ctrl+Maj+D", "25 transitions entre deux clips V1.", &[
        "Sélectionne le second clip, onglet Clip > Transition d'entrée : fondu enchaîné, au noir, au blanc, volets, glissements, poussées, iris, zoom, flou, horloge, store, damier, portes…",
        "Règle la durée et l'alignement : centrée, début ou fin à la coupe.",
        "Ctrl+Maj+D applique un fondu enchaîné d'une seconde aux clips sélectionnés.",
    ]),
    t("Images clés", "", "Anime échelle, position, rotation, opacité, flou, couleur et volume.", &[
        "Place la tête de lecture, clique le losange à côté du réglage : une image clé est créée.",
        "Déplace la tête, change la valeur : une nouvelle image clé s'ajoute automatiquement.",
        "Les flèches ‹ › sautent d'image clé en image clé ; Lissage adoucit le mouvement.",
    ]),
    t("Incrustation et V2", "", "Image dans l'image, fond vert, logos.", &[
        "Clic droit sur un média > Incruster sur V2, ou clic droit sur un clip > Monter sur V2.",
        "Onglet Image > Incrustation d'image : coins, centre, écran partagé.",
        "Onglet Effets > Incrustation (fond vert) : choisis la couleur, règle tolérance et douceur.",
    ]),
    t("Masques", "P", "Plume, rectangle et ellipse dans le moniteur.", &[
        "Rectangle ou Ellipse : glisse dans le moniteur sur le clip à la tête de lecture.",
        "Plume : clique point par point, puis clique le premier point ou Entrée pour fermer, Échap pour annuler.",
        "Onglet Effets > Masque : contour progressif et inversion. Avec la Sélection, déplace les poignées jaunes.",
    ]),
    t("Couleur", "", "Correction type Lumetri : base, créatif, courbes, roues.", &[
        "Correction de base : exposition, contraste, hautes lumières, ombres, blancs, noirs, température, teinte, saturation, vibrance.",
        "Créatif : 19 looks avec intensité réglable, rotation de teinte.",
        "Courbes maître, rouge, vert, bleu ; roues ombres, tons moyens et hautes lumières.",
        "Copier / Coller / Appliquer à tous recopie l'étalonnage d'un clip à l'autre.",
    ]),
    t("Effets", "", "Flou, netteté, mosaïque, postérisation, grain, négatif, vignettage.", &["Onglet Effets : règle chaque effet ou choisis un préréglage (VHS, Film ancien, Bande dessinée…).", "Réinitialiser tous les effets remet le clip à zéro."]),
    t("Titres", "T / Ctrl+T", "Textes à l'écran, styles, générique.", &[
        "Outil Texte (T) : clique dans le moniteur pour poser un titre à cet endroit. Texte vertical dans le même groupe.",
        "Ctrl+T ajoute un titre centré. Styles prédéfinis : cinéma, tiers inférieur, générique défilant, bandeau d'info, gros titre YouTube…",
        "Apparence : police, contour, ombre, bandeau coloré, espacement, interligne, alignement.",
        "Défilement : générique vertical ou bandeau horizontal ; 10 animations d'entrée.",
    ]),
    t("Marqueurs", "M", "Repères colorés avec nom et commentaire.", &["M ajoute un marqueur ; M sur un marqueur existant l'ouvre pour lui donner nom, couleur, commentaire et durée.", "Maj+M supprime le marqueur à la tête de lecture.", "Marqueurs > Copier comme chapitres YouTube, ou Fichier > Exporter les marqueurs (CSV)."]),
    t("Son", "", "Volume, balance, normalisation, solo, atténuation.", &[
        "Onglet Audio : volume animable, balance, normalisation à −1, −3 ou −6 dB.",
        "Ctrl+L détache le son d'un clip sur A2 pour le monter séparément.",
        "En-têtes de pistes : S pour solo, haut-parleur pour couper ; l'indicateur à droite du moniteur montre le niveau.",
        "Panneau Mixage : volume général et atténuation automatique de la musique sous les voix.",
    ]),
    t("Pistes", "", "Verrouiller, masquer, hauteur.", &["Le cadenas verrouille une piste, l'œil la masque dans le programme.", "Affichage > Hauteur des pistes S, M, L ou XL."]),
    t("Format de séquence", "", "16:9, 4K, vertical, carré, 4:5, cinéma…", &["Séquence > Réglages : 12 formats, 8 fréquences d'images, couleur de fond.", "« Comme le 1er clip » reprend la taille et la fréquence de ta vidéo."]),
    t("Main et zoom", "H / Z", "Navigue dans une longue timeline.", &["Main (H) : glisse pour faire défiler.", "Zoom (Z) : clic pour zoomer, Alt+clic pour dézoomer, \\ pour tout voir."]),
    t("Entrée et sortie", "I / O", "Délimite une plage à lire en boucle ou à exporter.", &["I pose l'entrée, O la sortie, Alt+X efface, Maj+I / Maj+O y vont.", "Active la boucle pour répéter la plage, ou choisis « Entre entrée et sortie » à l'export."]),
    t("Affichage du moniteur", "F", "Repères et mode visionneuse.", &["Zones de sécurité, grille des tiers, repères vertical 9:16.", "F masque les panneaux pour un grand moniteur."]),
    t("Historique", "Ctrl+Z", "Reviens à n'importe quelle étape.", &["Ctrl+Z / Ctrl+Y annulent et rétablissent.", "Désélectionne tout : le panneau Historique liste les dernières étapes, clique pour y revenir."]),
    t("Exporter", "Ctrl+M", "MP4, GIF, WAV, séquence PNG, image PNG ou JPEG.", &[
        "16 préréglages : YouTube 720p à 4K, TikTok/Reels/Shorts, Instagram, X, Vimeo, master, brouillon, GIF, audio, images.",
        "Règle taille, cadrage (ajuster ou remplir), images/s, débit vidéo et audio, plage (tout, entrée/sortie, clip).",
        "L'export peut être annulé ; « Afficher » ouvre le dossier une fois terminé.",
        "Ctrl+Maj+E exporte l'image affichée en PNG.",
    ]),
];
