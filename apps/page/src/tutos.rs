use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const PAGE: &[Tuto] = &[
    t("Nouveau document", "Ctrl+N", "10 formats et 6 modèles prêts : magazine, flyer, CV, carte de visite, menu, affiche.", &[
        "Choisis un format ou un modèle sur l'accueil.",
        "La pastille en haut à droite (Ctrl+K) rouvre le format : taille en mm, portrait ou paysage, marges, fond perdu.",
    ]),
    t("Sélection", "V", "Sélectionne, déplace et redimensionne les blocs.", &[
        "Clique un bloc, glisse-le ; Maj bloque l'axe.",
        "Huit poignées : les coins redimensionnent librement (Maj garde les proportions), les côtés étirent dans un seul sens.",
        "Flèches : 1 pt, Maj+flèches : 10 pt. Les bords s'aimantent aux marges, colonnes, repères et autres blocs (ligne rose).",
    ]),
    t("Texte", "T", "Blocs de texte avec colonnes, styles et variables.", &[
        "Trace un bloc avec l'outil Texte : la fenêtre d'écriture s'ouvre. Double-clic ou Entrée pour y revenir.",
        "La barre d'options change la police, la taille, le gras, l'italique, le soulignement, les majuscules, l'alignement et la couleur.",
        "Le carré rouge signale un texte qui déborde : Ajuster la hauteur, ou chaîne-le vers un autre bloc.",
    ]),
    t("Variables", "", "Du texte qui se remplit tout seul.", &["{page} le numéro de page, {pages} le total, {title} le titre du document, {date} la date du jour.", "Idéal dans le gabarit pour un pied de page."]),
    t("Paragraphe", "", "Justifié, interligne, espacements, retrait, listes.", &[
        "Justifié aligne les deux bords comme dans un livre.",
        "Espace après paragraphe aère sans lignes vides ; le retrait décale la première ligne.",
        "Liste : puces ou numéros, un par paragraphe.",
    ]),
    t("Texte chaîné", "", "Le texte qui ne tient pas continue dans un autre bloc.", &[
        "Sélectionne le premier bloc, puis Bloc de texte > Suite du texte dans : choisis le bloc suivant.",
        "Un trait bleu relie les deux blocs. Le texte du second bloc est remplacé par la suite.",
    ]),
    t("Styles de paragraphe", "", "Une mise en forme réutilisable dans tout le document.", &[
        "Mets en forme un texte, puis Paragraphe > Créer un style.",
        "Applique-le d'un clic à d'autres textes depuis Styles ou le menu Texte. 8 styles sont fournis.",
    ]),
    t("Image", "I", "Photos et logos dans un bloc.", &[
        "Trace un bloc avec l'outil Image et choisis le fichier. Un cadre vide affiche une croix.",
        "Remplir recadre pour couvrir le bloc, Ajuster montre l'image entière ; Cadre aux proportions adapte la hauteur.",
        "Filtres noir et blanc, sépia ou délavé, arrondi des coins, miroir.",
    ]),
    t("Rectangle", "M", "Aplats de couleur, encadrés, bandeaux.", &["Trace le rectangle, règle couleur, arrondi, dégradé et contour."]),
    t("Ellipse", "L", "Cercles et pastilles.", &["Trace l'ellipse ; Maj donne un cercle parfait."]),
    t("Polygone et étoile", "P", "Badges, étoiles, sceaux promotionnels.", &["Règle les côtés et le creux dans la barre d'options avant de tracer.", "Creux à 1 = polygone, plus petit = étoile. Beaucoup de branches et un creux de 0,8 donnent un sceau."]),
    t("Ligne", "\\", "Filets pour séparer les rubriques, flèches.", &["Trace la ligne, règle couleur, épaisseur, pointillés et pointe de flèche."]),
    t("Tableau", "G", "Tableaux de prix, plannings, comparatifs.", &[
        "Trace le tableau, puis ajoute ou retire lignes et colonnes avec + et −.",
        "Écris dans les cellules du panneau Tableau. Ligne d'en-tête et lignes alternées colorables.",
        "Un texte trop long dans une cellule finit par « … ».",
    ]),
    t("Pipette de style", "E", "Recopie l'apparence d'un bloc sur un autre.", &["Sélectionne le bloc à modifier, prends la pipette, puis clique le bloc modèle.", "Police, couleurs, contour, ombre et arrondi sont copiés."]),
    t("Aspect", "", "Opacité, contour, pointillés, ombre, rotation.", &["Panneau Aspect pour l'opacité, le contour et l'ombre portée.", "Rotation dans le panneau Bloc, avec raccourcis -90°, 0°, 90°."]),
    t("Nuancier", "", "Les couleurs de ta charte, à portée de clic.", &["Ajoute des couleurs dans le Nuancier (sans bloc sélectionné).", "Elles apparaissent en pastilles à côté de chaque sélecteur de couleur."]),
    t("Règles et repères", "Ctrl+R", "Des lignes d'aide pour aligner précisément.", &[
        "Glisse depuis la règle du haut ou de gauche vers la page pour poser un repère.",
        "Déplace-le à la souris, ramène-le hors de la page pour le supprimer. Les règles sont graduées en millimètres.",
        "Grille de base et colonnes se règlent dans le panneau Document.",
    ]),
    t("Objets de la page", "", "La liste de tous les blocs, comme des calques.", &["Clique un nom pour le sélectionner, l'œil le masque, le cadenas le verrouille.", "Donne un nom au bloc dans le panneau Bloc pour t'y retrouver."]),
    t("Gabarit", "", "Éléments répétés sur toutes les pages : en-tête, pied, numéro, logo.", &[
        "Clique Gabarit en haut de la liste des pages pour le modifier.",
        "Clic droit sur une miniature : utiliser ou retirer le gabarit, par exemple pour une couverture.",
    ]),
    t("Pages", "Pg préc. / Pg suiv.", "Ajouter, dupliquer, déplacer, supprimer.", &["Clic droit sur une miniature pour toutes les actions, ou + au-dessus de la liste.", "Le numéro de la première page se règle dans Document (par exemple 2 pour un dos de couverture)."]),
    t("Aligner et organiser", "Ctrl+[ / Ctrl+]", "Aligne sur les marges, change l'ordre de superposition.", &["Panneau Disposition : six alignements et quatre ordres.", "Objet > Centrer sur la page, ou Remplir la page jusqu'au fond perdu."]),
    t("Copier et coller", "Ctrl+C / Ctrl+V", "Réutilise un bloc ailleurs, même sur une autre page.", &["Ctrl+V colle décalé, Ctrl+Maj+V colle exactement au même endroit, Ctrl+X coupe."]),
    t("Aperçu", "W", "Voir la page comme elle sera imprimée.", &["W masque les cadres, marges et repères. Zoom avec Ctrl+molette ou les boutons en bas à droite."]),
    t("Rechercher et remplacer", "Ctrl+F", "Corrige un mot dans tout le document en une fois.", &["Édition > Rechercher et remplacer : textes et tableaux, toutes les pages et le gabarit."]),
    t("Exporter", "Ctrl+E", "PDF, PNG, JPEG ou SVG.", &[
        "Le bouton Exporter ouvre les options : format, pages (toutes, la page affichée ou une plage), résolution et qualité.",
        "Pour l'imprimeur : règle un fond perdu de 3 mm, étends les fonds jusqu'au trait rouge, puis coche Fond perdu et traits de coupe.",
    ]),
];
