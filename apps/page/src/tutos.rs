use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const PAGE: &[Tuto] = &[
    t("Nouveau document", "Ctrl+N", "A4, A5, Lettre, affiche, flyer, carte de visite, carré ou story.", &["Choisis un format sur l'accueil.", "Les marges et la grille de colonnes se règlent dans le panneau Document (sans bloc sélectionné)."]),
    t("Sélection", "V", "Sélectionne, déplace et redimensionne les blocs.", &["Clique un bloc, glisse-le ; tire un coin pour le redimensionner.", "Flèches : déplacement de 1 pt, Maj+flèches : 10 pt.", "Les bords s'aimantent aux marges et au centre de la page."]),
    t("Texte", "T", "Blocs de texte avec colonnes, styles et numéros de page.", &[
        "Trace un bloc avec l'outil Texte, puis écris dans le panneau Contenu.",
        "Applique un style (Titre 1, Corps, Légende…) depuis la barre d'options.",
        "Colonnes et gouttière répartissent le texte comme dans un magazine.",
        "{page} et {pages} affichent le numéro et le total des pages.",
        "Le carré rouge signale un texte qui déborde : clique Ajuster la hauteur au texte.",
    ]),
    t("Image", "I", "Photos et logos dans un bloc.", &["Trace un bloc avec l'outil Image et choisis le fichier.", "Remplir recadre pour couvrir le bloc, Ajuster montre l'image entière.", "Arrondi pour des coins doux."]),
    t("Rectangle", "M", "Aplats de couleur, encadrés, bandeaux.", &["Trace le rectangle, règle couleur, arrondi et contour."]),
    t("Ellipse", "L", "Cercles et pastilles.", &["Trace l'ellipse ; un carré parfait donne un cercle."]),
    t("Ligne", "\\", "Filets horizontaux pour séparer les rubriques.", &["Trace la ligne de gauche à droite, règle couleur et épaisseur."]),
    t("Gabarit", "", "Éléments répétés sur toutes les pages : en-tête, pied, numéro, logo.", &[
        "Clique Gabarit en haut de la liste des pages pour le modifier.",
        "Ce que tu y places apparaît sur chaque page.",
        "Décoche « Utiliser le gabarit » pour une page de couverture sans en-tête.",
    ]),
    t("Pages", "Pg préc. / Pg suiv.", "Ajouter, dupliquer, déplacer, supprimer.", &["Menu Page, ou + Page sous les miniatures.", "Pg préc. et Pg suiv. pour naviguer."]),
    t("Aligner et organiser", "", "Aligne sur les marges, change l'ordre de superposition.", &["Panneau Disposition : six alignements sur la zone des marges.", "Premier plan, avancer, reculer, arrière-plan.", "Ctrl+D duplique, Ctrl+L verrouille."]),
    t("Rechercher et remplacer", "Ctrl+F", "Corrige un mot dans tout le document en une fois.", &["Édition > Rechercher et remplacer, puis Tout remplacer."]),
    t("Exporter", "Ctrl+E", "PDF vectoriel multipage, ou PNG 300 dpi.", &["Fichier > Exporter en PDF pour l'impression ou l'envoi.", "Exporter la page ou toutes les pages en PNG 300 dpi pour les réseaux."]),
];
