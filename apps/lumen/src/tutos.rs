use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const LUMEN: &[Tuto] = &[
    t("Envoyer vers une autre app", "", "Comme AirDrop entre les apps de la suite : ton travail s'ouvre dans l'autre app en un clic.", &["Fichier > Envoyer vers, puis choisis l'app.", "Si l'app est déjà ouverte, elle reçoit le fichier tout de suite (une notification « Reçu » apparaît en bas à droite). Sinon elle s'ouvre avec ton envoi.", "Les fichiers reçus sont gardés dans le dossier EthoneStudio › handoff › received."]),
    t("Effets créatifs", "", "Plus de 100 effets de Pixel empilés sur le développement.", &["Panneau Effets créatifs > + Ajouter un effet : Relight, brume, looks, lueur, glitch, aquarelle, pluie, neige…", "Chaque effet a son intensité, ses réglages et ses préréglages ; ▲ change l'ordre, la case l'active ou le désactive.", "Les effets sont enregistrés avec la photo, copiables vers d'autres photos et appliqués à l'export."]),
    t("Importer des photos", "", "Lumen travaille sur un dossier : tes originaux ne sont jamais modifiés.", &[
        "Fichier > Ouvrir un dossier… (JPEG, PNG, TIFF, WebP et RAW : CR2, CR3, NEF, ARW, DNG, RAF…).",
        "Coche Inclure les sous-dossiers pour tout voir d'un coup ; le panneau Dossiers liste les sous-dossiers et le dossier parent.",
        "Les réglages, notes, mots-clés et instantanés sont gardés dans le catalogue.",
    ]),
    t("Noter et trier", "1 à 5", "Étoiles, drapeaux et collections pour retrouver tes meilleures images.", &[
        "Touches 1 à 5 pour noter, 0 pour retirer la note.",
        "Le filtre ≥ / = / ≤ choisit « au moins », « exactement » ou « au plus » ce nombre d'étoiles.",
        "Tri par nom, note, date de modification, date de prise de vue, taille, type ou étiquette ; la flèche inverse l'ordre.",
    ]),
    t("Drapeaux et étiquettes", "P / X / U, 6-9", "Trie une série comme un pro : retenues, rejetées, couleurs.", &["P retient, X rejette, U retire le drapeau.", "6 à 9 posent une étiquette rouge, jaune, verte ou bleue ; le violet se pose depuis Photo > Étiquette.", "Les rejetées sont masquées de Toutes les photos."]),
    t("Sélection multiple", "Ctrl / Maj+clic", "Travaille sur plusieurs photos à la fois.", &["Ctrl+clic ajoute une photo, Maj+clic une plage, Ctrl+A tout, Ctrl+D rien.", "La barre bleue synchronise les réglages, applique un préréglage, empile, retient ou rejette la sélection.", "Notes, drapeaux, étiquettes et Développement rapide s'appliquent à toute la sélection."]),
    t("Mots-clés, titre et légende", "", "Décris tes photos pour les retrouver.", &["Dans le panneau de droite, tape des mots-clés séparés par des virgules puis Entrée.", "Clique un mot-clé pour le retirer, ou une suggestion pour l'ajouter.", "La recherche trouve le nom, le titre, la légende et les mots-clés ; chaque mot-clé devient un filtre à gauche."]),
    t("Métadonnées EXIF", "", "Appareil, objectif, vitesse, ouverture, ISO, focale.", &["Le panneau Métadonnées lit l'EXIF des JPEG, TIFF et RAW.", "Le tri Prise de vue range les photos par date de déclenchement.", "I en Développement affiche ces informations sur la photo."]),
    t("Collections dynamiques", "", "Des filtres enregistrés qui se mettent à jour seuls.", &["Clique + à côté de Collections dynamiques.", "Combine note minimale, drapeau, étiquette, mot-clé, texte, retouche et type (RAW ou non).", "Clic droit sur une collection pour la modifier ou la supprimer."]),
    t("Piles et copies virtuelles", "Ctrl+G / Ctrl+'", "Range les rafales et essaie plusieurs versions.", &["Ctrl+G empile la sélection, S réduit ou déplie la pile, Ctrl+Maj+G désempile.", "Ctrl+' crée une copie virtuelle : même fichier, réglages indépendants (noir et blanc et couleur côte à côte).", "Le badge ⧉ marque une copie, le chiffre la taille d'une pile."]),
    t("Comparer et Ensemble", "C / N", "Choisis la meilleure image d'une série.", &["C : la photo active à gauche, un candidat à droite ; les flèches font défiler les candidats.", "N : toutes les photos sélectionnées en grand ; × écarte une photo de l'ensemble.", "Double-clic pour ouvrir en Développement."]),
    t("Développement rapide", "", "Corrige sans quitter la Bibliothèque.", &["Les flèches ◀◀ ◀ ▶ ▶▶ ajoutent ou retirent de l'exposition (1 IL ou 1/3 IL), du contraste, de la clarté…", "Balance des blancs et Noir et blanc en un clic, sur toute la sélection."]),
    t("Développement", "D", "Le module de retouche : la photo au centre, les outils à gauche, les réglages à droite.", &[
        "D (ou Entrée) ouvre la photo en Développement, G revient à la Bibliothèque.",
        "Flèches gauche/droite : photo suivante ou précédente ; le film en bas a ses propres filtres.",
        "Tab masque les panneaux, I affiche les infos, double-clic sur un curseur ou son nom pour le remettre à zéro, Maj+glisser pour un réglage fin.",
    ]),
    t("Barre d'outils", "R Q E M K W T H", "Les outils liés partagent un emplacement, comme chez Adobe.", &[
        "Clic : prend l'outil affiché. Clic droit, clic maintenu ou le petit triangle : ouvre le menu du groupe.",
        "Appuyer de nouveau sur le raccourci passe à l'outil suivant du groupe (R : recadrage puis règle, Q : suppression, correcteur, clonage, M : gradué, radial, pinceau).",
        "Les options de l'outil actif s'affichent dans la barre au-dessus de la photo ; Échap revient à la main.",
    ]),
    t("Recadrage et redressement", "R", "Recadre à la main et redresse l'horizon.", &[
        "R affiche toute l'image : tire les coins ou les bords, glisse l'intérieur pour déplacer, glisse à l'extérieur pour tourner.",
        "Choisis un format (1:1, 4:5, 16:9…), verrouille-le, ⇄ inverse l'orientation ; superposition tiers, grille, nombre d'or ou diagonales.",
        "Règle (R deux fois) : trace une ligne le long de l'horizon ; Auto redresse tout seul. Terminé ou Entrée valide.",
    ]),
    t("Retouche des taches", "Q", "Suppression, correcteur et clonage.", &[
        "Clique sur une poussière : Lumen choisit seul une zone de remplacement. Glisse depuis la tache pour choisir toi-même la source.",
        "Correcteur fond la texture dans les couleurs voisines, Clonage copie à l'identique, Suppression choisit et fond automatiquement.",
        "Taille ([ et ]), contour, opacité ; Nouvelle source, Suppr ou clic droit pour effacer une retouche.",
    ]),
    t("Yeux rouges", "E", "Rend leur couleur naturelle aux pupilles.", &["Clique sur l'œil ou glisse pour tracer le cercle.", "Pupille agrandit la zone traitée, Assombrir règle la noirceur de la pupille."]),
    t("Masques", "M / Maj+M / K", "Retouche locale avec tous les réglages.", &[
        "Filtre gradué (M) : glisse pour tracer la transition, idéal pour un ciel. Filtre radial (Maj+M) : glisse depuis le centre.",
        "Pinceau (K) : peins la zone ; Alt ou Gomme efface ; taille, contour progressif et débit dans la barre d'options.",
        "Chaque masque a sa température, teinte, exposition, contraste, ombres, clarté, voile, netteté, bruit, couleur… et une quantité globale.",
        "O affiche la superposition rouge de la zone touchée.",
    ]),
    t("Ciel, sujet et plages", "", "Sélections automatiques et affinées.", &[
        "Sélectionner le ciel ou le sujet (groupe des masques) crée un masque prêt à régler.",
        "Ajouter, Soustraire ou Intersecter combine des composants : un radial moins le ciel, un pinceau limité à une plage de luminance…",
        "Plage de luminance et Plage de couleur ciblent les tons ou une teinte précise.",
    ]),
    t("Pipette et réglage ciblé", "W / T", "Clique ou glisse directement sur la photo.", &["Pipette (W) : clique une zone grise ou blanche pour neutraliser la dominante.", "Réglage ciblé (T) : glisse vers le haut ou le bas sur un ton (courbe) ou une couleur (teinte, saturation, luminance)."]),
    t("Réglages de base", "", "Profil, balance des blancs, tonalité et présence.", &[
        "Profil : Couleur, Vif, Neutre, Paysage, Portrait ou Monochrome comme point de départ.",
        "Exposition, contraste, hautes lumières, ombres, blancs, noirs ; texture, clarté, correction du voile, vibrance, saturation.",
        "Survole la photo : l'histogramme affiche les valeurs RVB du point.",
    ]),
    t("Courbe des tonalités", "", "Paramétrique ou par points, par couche.", &["Param. : ombres, tons foncés, tons clairs, hautes lumières.", "RVB, R, V, B : clique pour ajouter un point, glisse pour le déplacer, double-clic pour le retirer.", "Préréglages de courbe : contraste moyen ou fort, film mat, inversion."]),
    t("TSL", "", "Teinte, saturation et luminance de 8 familles de couleurs.", &["Onglets Teinte, Saturation, Luminance ou Tout.", "Exemple : Bleus en luminance −40 pour assombrir un ciel ; en noir et blanc, la luminance règle le gris de chaque couleur."]),
    t("Étalonnage des couleurs", "", "Roues ombres, tons moyens, hautes lumières et global.", &["Glisse dans une roue : l'angle choisit la teinte, la distance l'intensité.", "Luminance par zone, Fusion (recouvrement des zones) et Équilibre (vers les ombres ou les lumières)."]),
    t("Détail", "", "Netteté et réduction du bruit.", &["Netteté : gain, rayon, détail et masquage (le masquage protège les aplats comme le ciel ou la peau).", "Bruit : luminance et son détail, couleur, détail et lissage de la couleur."]),
    t("Optique et transformation", "", "Corrige l'objectif et les perspectives.", &["Vignetage de l'objectif et son point milieu, distorsion, aberrations chromatiques rouge/cyan et bleu/jaune, franges violettes et vertes.", "Transformation : perspective verticale et horizontale, rotation fine, aspect, échelle, décalages, miroir vertical."]),
    t("Effets", "", "Vignettage, grain, halo et délavé.", &["Trois styles de vignettage : priorité couleur, priorité hautes lumières, superposition de peinture ; arrondi et contour progressif.", "Grain avec taille et rugosité, Halo lumineux pour un rendu rêveur, Délavé pour un look mat."]),
    t("Étalonnage de l'appareil", "", "Ajuste les primaires de couleur.", &["Teinte des ombres, puis teinte et saturation des primaires rouge, verte et bleue."]),
    t("Préréglages", "", "Plus de 60 looks rangés par catégorie.", &[
        "Couleur, Couleurs vives, Portrait, Paysage, Noir et blanc, Cinéma, Vintage, Style, Nuit et ville, Correction.",
        "Le curseur Quantité (0 à 200 %) dose le dernier préréglage appliqué ; clic droit pour l'ajouter aux favoris.",
        "Mes préréglages : enregistre tes réglages, clic droit pour mettre à jour ou supprimer.",
    ]),
    t("Instantanés et historique", "Ctrl+N", "Garde des versions et reviens en arrière.", &["+ à côté d'Instantanés (ou Ctrl+N) mémorise l'état actuel ; clic pour y revenir.", "L'historique liste chaque réglage ; clique une étape pour revenir avant elle. Ctrl+Z / Ctrl+Y annulent et rétablissent."]),
    t("Copier et coller les réglages", "Ctrl+C / Ctrl+V", "Applique la même retouche à toute une série.", &["Ctrl+Maj+C choisit les groupes à copier (lumière, couleur, masques, recadrage…).", "Ctrl+V colle sur la photo ou la sélection, Ctrl+Maj+V sur toutes les photos affichées.", "Réglages > Réinitialiser un groupe… remet un seul panneau à zéro."]),
    t("Comparer avant / après", "\\ / Y", "Rideau, côte à côte ou maintien.", &["Maintiens Original (ou \\) pour voir la photo d'origine.", "Y fait défiler : rideau vertical, côte à côte, rideau horizontal ; glisse le séparateur."]),
    t("Écrêtage et zoom", "J / Z", "Vérifie les détails perdus et la netteté.", &["Écrêtage (J) montre en rouge les blancs brûlés, en bleu les noirs bouchés.", "Zoom : Ajuster, 25 à 400 %, Z ou double-clic ; le navigateur à gauche montre la zone vue, clique dedans pour t'y rendre."]),
    t("Exporter", "Ctrl+Maj+E", "JPEG, PNG, TIFF ou WebP, en lot, avec nommage et filigrane.", &[
        "Emplacement : un dossier ou celui de l'original, sous-dossier, nom unique, écraser ou ignorer.",
        "Nom : modèles avec {nom} {n} {n3} {date} {titre} {note} {texte} {suffixe} et numéro de départ.",
        "Taille : 4K, Web 2048, écran 1920, Instagram 1080 carré ou 1080 × 1350, story, miniature ou sur mesure ; Ne pas agrandir.",
        "Accentuation de sortie (écran, papier mat ou brillant), métadonnées (toutes, sans localisation, copyright ou aucune), filigrane texte.",
        "Fichier > Exporter comme précédemment réutilise les derniers réglages.",
    ]),
];
