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
    t("Crayon", "Maj+B", "Trait net sans anticrénelage, parfait pour le pixel art.", &["Zoome fort (Ctrl+molette) et dessine pixel par pixel avec une petite taille."]),
    t("Gomme", "E", "Rend les pixels transparents sur le calque actif.", &["Utilise un flux faible pour effacer progressivement."]),
    t("Dégradé", "G", "Fond de la couleur principale vers la secondaire.", &["Glisse sur l'image : la direction et la longueur du trait définissent le dégradé.", "Fais-le dans une sélection pour le limiter."]),
    t("Pot de peinture", "Maj+G", "Remplit une zone de couleur proche.", &["Clique dans la zone à remplir.", "La tolérance règle jusqu'où la couleur se propage."]),
    t("Goutte", "R", "Adoucit localement : peau, arrière-plan, bords trop durs.", &["Passe plusieurs fois pour renforcer l'effet."]),
    t("Netteté", "", "Renforce les détails là où tu peins.", &["Utilise-le avec parcimonie sur les yeux ou les textures."]),
    t("Doigt", "", "Étale la peinture comme un doigt sur une toile fraîche.", &["Glisse lentement : la couleur de départ est entraînée."]),
    t("Densité −", "O", "Éclaircit les zones peintes (dodge).", &["Idéal pour faire ressortir des reflets ou des yeux."]),
    t("Densité +", "", "Assombrit les zones peintes (burn).", &["Pour creuser des ombres et donner du volume."]),
    t("Éponge", "", "Sature ou désature localement.", &["Choisis Saturer ou Désaturer dans la barre d'options, puis peins."]),
    t("Rectangle", "U", "Dessine un rectangle plein de la couleur principale.", &["Glisse sur l'image. Par défaut, chaque forme va sur son propre calque."]),
    t("Ellipse", "", "Dessine une ellipse pleine et lissée.", &["Glisse sur l'image pour définir son cadre."]),
    t("Trait", "", "Trace une ligne droite, avec flèches en option.", &["Glisse d'un point à l'autre ; Maj aligne tous les 45°.", "Règle l'épaisseur et coche Flèche au début ou à la fin."]),
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
    t("Exporter", "Ctrl+E", "Enregistre l'image en PNG, JPEG, WebP, BMP, TIFF, GIF ou ICO.", &[
        "Ctrl+E ouvre la fenêtre d'export : format, qualité JPEG, compression PNG, échelle.",
        "JPEG et BMP n'ont pas de transparence : choisis la couleur de fond.",
        "ICO crée une icône Windows avec plusieurs tailles d'un coup (16 à 256 px).",
        "Ctrl+Maj+Alt+S exporte directement en PNG ; Fichier > Exporter la sélection ou Exporter les calques en PNG.",
    ]),
    t("Outils groupés", "Maj+lettre", "Les outils voisins partagent une case, comme dans Photoshop.", &[
        "Un petit triangle dans le coin signale une case qui contient plusieurs outils.",
        "Clic droit, clic maintenu ou clic sur le triangle ouvre la liste des outils de la case.",
        "La lettre choisit le dernier outil utilisé de la case ; Maj+lettre passe au suivant (Maj+B : pinceau, crayon, remplacement de couleur, mélangeur).",
    ]),
    t("Sélections", "M", "Rectangle, ellipse, ligne ou colonne d'un pixel, avec modes et contour progressif.", &[
        "Dans la barre d'options, choisis Nouvelle, Ajouter, Soustraire ou Intersection pour combiner avec la sélection existante.",
        "Maj contraint en carré ou en cercle, Alt trace depuis le centre.",
        "Style : Proportions fixes (ex. 16 × 9) ou Taille fixe pour un cadre exact posé d'un clic.",
        "Contour progressif adoucit le bord ; Sélection > Modifier propose aussi Lisser, Dilater, Contracter et Bordure.",
        "Sélection > Mémoriser garde une sélection ; Ctrl+Maj+D resélectionne la dernière.",
    ]),
    t("Lassos", "L", "Lasso libre, polygonal ou magnétique.", &[
        "Lasso : trace à main levée.",
        "Lasso polygonal : clique chaque sommet, puis double-clic, Entrée ou clic sur le premier point pour fermer. Échap annule.",
        "Lasso magnétique : glisse près d'un bord, le tracé s'accroche aux contrastes ; règle la largeur de recherche.",
    ]),
    t("Sélection rapide et baguette", "W", "Sélectionne une zone en peignant dessus, ou d'un clic par couleur.", &[
        "Sélection rapide : peins sur l'objet, la sélection s'étend aux pixels de couleur proche ; Alt retire.",
        "Baguette magique : un clic sélectionne la couleur ; décoche Contiguë pour toute l'image.",
        "Sélection > Plage de couleurs sélectionne par couleur, tons foncés, clairs, teintes ou tons chair, de façon progressive.",
    ]),
    t("Pinceaux et dynamique", "B", "Préréglages et réglages fins de la forme du pinceau.", &[
        "Le panneau Pinceaux propose des préréglages : rond dur, aérographe, encre calligraphique, craie, feuillage…",
        "Espacement, angle et rondeur dessinent la forme ; variation de taille, de flux et dispersion la rendent vivante.",
        "Lissage stabilise le trait ; le mode du pinceau (Produit, Incrustation…) mélange avec ce qui est dessous.",
        "Les touches 1 à 0 règlent le flux, Maj+[ et Maj+] la dureté.",
    ]),
    t("Remplacement de couleur", "B", "Change la teinte d'un objet en gardant ses ombres et lumières.", &["Choisis la nouvelle couleur principale, puis peins sur l'objet."]),
    t("Pinceau mélangeur", "B", "Mélange la couleur principale avec la peinture déjà posée.", &["Humidité haute : la toile se mélange beaucoup ; basse : la couleur couvre davantage."]),
    t("Gommes", "E", "Gomme, gomme d'arrière-plan et gomme magique.", &[
        "Gomme : rend transparent ; Effacer vers l'historique restaure l'instantané à la place.",
        "Gomme d'arrière-plan : n'efface que la couleur sous le centre du pinceau, selon la tolérance.",
        "Gomme magique : un clic efface toute la zone de couleur proche.",
    ]),
    t("Retouche locale", "J", "Correcteur et Pièce pour réparer peau, taches et défauts.", &[
        "Correcteur : Alt+clic sur une zone saine, puis peins ; la texture est copiée et la couleur s'adapte autour.",
        "Pièce : entoure le défaut, puis fais glisser la sélection vers une zone saine ; la diffusion règle la fusion.",
    ]),
    t("Historique", "Y", "Revenir à n'importe quelle étape ou repeindre l'état d'origine.", &[
        "Le panneau Historique liste chaque action : clique une ligne pour y revenir.",
        "Forme d'historique (Y) repeint l'instantané ; clique Instantané pour en prendre un nouveau.",
        "Édition > Préférences règle le nombre d'états conservés.",
    ]),
    t("Dégradés", "G", "Cinq types de dégradés et des préréglages.", &[
        "Choisis Linéaire, Radial, Angulaire, Réfléchi ou Losange.",
        "Préréglages : principale → transparent, arc-en-ciel, chrome, cuivre…",
        "Maj aligne tous les 45° ; Inverser, Opacité, Mode et Tramage sont dans la barre d'options.",
    ]),
    t("Formes", "U", "Rectangle arrondi, ellipse, polygone, étoile, trait et formes personnalisées.", &[
        "Fond utilise la couleur principale, Contour la couleur secondaire.",
        "Polygone : nombre de côtés et option Étoile avec retrait.",
        "Forme personnalisée : cœur, étoile, flèche, bulle, coche, croix, éclair, goutte, lune.",
        "Maj contraint les proportions, Alt dessine depuis le centre.",
    ]),
    t("Pipette, règle et notes", "I", "Mesurer, lire des couleurs et annoter.", &[
        "Pipette : taille d'échantillon jusqu'à 31 × 31 ; Alt+clic prend la couleur secondaire.",
        "Échantillonnage de couleur : pose jusqu'à 4 points lus en continu dans Infos.",
        "Règle : mesure distance et angle ; Redresser le calque corrige un horizon penché.",
        "Annotation : pose des notes, survole-les pour les lire, clique pour les modifier.",
    ]),
    t("Navigation", "H", "Main, rotation de la vue et zoom.", &[
        "Espace maintenu donne la main à tout moment.",
        "Rotation de la vue (Maj+H) tourne l'affichage sans modifier l'image ; Maj aligne tous les 15°.",
        "Tab masque les panneaux pour travailler en plein écran.",
    ]),
    t("Déplacement et alignement", "V", "Déplace, aligne et choisit les calques au clic.", &[
        "Sélection automatique choisit le calque visible sous le pointeur.",
        "Les six boutons alignent le calque sur la sélection ou sur la zone de travail.",
        "Flèches : 1 px ; Maj+flèches : 10 px.",
    ]),
    t("Courbes et niveaux", "Ctrl+M", "Réglages tonals précis avec préréglages.", &[
        "Courbes : règle noirs, tons foncés, moyens, clairs et blancs, sur RVB ou une seule couche.",
        "Niveaux : entrée, gamma, sortie et couche.",
        "Chaque réglage a des préréglages ; donne un nom et clique Enregistrer pour garder le tien.",
        "Estomper mélange le résultat avec l'original (opacité et mode).",
    ]),
    t("Réglages de couleur", "Ctrl+B", "Balance des couleurs, filtre photo, mélangeur de couches, correction sélective.", &[
        "Balance des couleurs : décale tons foncés, moyens et clairs séparément.",
        "Noir et blanc avancé : dose chaque couleur dans le gris, avec virage.",
        "Filtre photo, courbe de transfert de dégradé, remplacement de couleur et correction sélective sont dans Image > Réglages.",
    ]),
    t("Calques de réglage", "", "Réglages non destructifs, modifiables à tout moment.", &[
        "Calque > Nouveau calque de réglage, ou le bouton ◑ du panneau Calques.",
        "Double-clique le calque pour modifier ses valeurs ; son masque limite l'effet.",
        "Baisse l'opacité ou change le mode de fusion du calque pour doser.",
    ]),
    t("Masques de fusion", "\\", "Cache une partie du calque sans rien effacer.", &[
        "Bouton ◐ : masque depuis la sélection, ou tout visible.",
        "Clique la vignette du masque (ou \\) puis peins en noir pour cacher, en blanc pour révéler.",
        "Calque > Masque de fusion : désactiver, intervertir, appliquer ou supprimer.",
    ]),
    t("Masque d'écrêtage", "Ctrl+Alt+G", "Limite un calque à la forme du calque du dessous.", &["Sélectionne le calque du dessus puis Ctrl+Alt+G : une flèche ↳ apparaît.", "Idéal pour mettre une texture dans du texte."]),
    t("Modes de fusion", "", "27 modes : Produit, Superposition, Incrustation, Teinte, Luminosité…", &["Choisis le mode dans Propriétés ou d'un clic droit sur le calque.", "Pinceaux, formes, dégradés et remplissages ont aussi leur propre mode."]),
    t("Styles de calque", "fx", "Ombres, lueurs, biseau, contour et incrustations.", &[
        "Bouton fx du panneau Calques ou Calque > Effets de calque.",
        "Ombre et lueur internes, biseau et estampage, incrustation couleur ou dégradé.",
        "Chaque effet est un calque à part : tu peux le masquer, l'effacer ou baisser son opacité.",
    ]),
    t("Filtres créatifs", "Ctrl+Alt+F", "Plus de 50 filtres : flou de surface, cristallisation, peinture à l'huile, éclairage…", &[
        "Menu Filtre, par familles : Flou, Bruit, Pixellisation, Esthétiques, Artistiques, Déformation, Rendu, Autres.",
        "Aperçu en direct, préréglages et Estomper dans la fenêtre du filtre.",
        "Ctrl+Alt+F répète le dernier filtre avec les mêmes réglages.",
    ]),
    t("Transformation manuelle", "Ctrl+T", "Échelle, rotation, inclinaison, perspective et symétrie.", &[
        "Échelle liée ou séparée en largeur et hauteur.",
        "Inclinaison et perspective corrigent les bâtiments ou posent un élément sur un plan.",
        "Pixels nets garde un rendu sans flou pour le pixel art.",
    ]),
    t("Taille de l'image", "Ctrl+Alt+I", "Redimensionne avec le rééchantillonnage de ton choix.", &[
        "En pixels ou en pourcentage, proportions liées.",
        "Plus proche voisin pour le pixel art, Lanczos pour réduire net, Gaussien pour adoucir.",
        "La résolution affiche la taille d'impression en centimètres.",
    ]),
    t("Règles, repères et grille", "Ctrl+R", "Aligne précisément avec règles, repères et magnétisme.", &[
        "Ctrl+R affiche les règles ; tire depuis une règle pour poser un repère.",
        "Avec Déplacement, glisse un repère pour le bouger, ou hors de l'image pour le supprimer.",
        "Affichage : repères au centre ou aux tiers, grille avec subdivisions et couleur, magnétisme.",
    ]),
    t("Nuancier et harmonies", "", "Bibliothèques de couleurs, harmonies et couleurs récentes.", &[
        "Choisis une bibliothèque (Pastel, Terre, Néon…) ; clic droit sur une nuance pour la supprimer.",
        "Harmonies propose complémentaire, triade, tétrade et nuances de la couleur principale.",
        "Le panneau Couleur passe en RVB, TSL ou CMJN et accepte un code hexadécimal.",
    ]),
];
