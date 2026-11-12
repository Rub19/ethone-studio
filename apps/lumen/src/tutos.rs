use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const LUMEN: &[Tuto] = &[
    t("Importer des photos", "", "Lumen travaille sur un dossier : tes originaux ne sont jamais modifiés.", &[
        "Fichier > Ouvrir un dossier… (JPEG, PNG, TIFF, WebP et RAW : CR2, CR3, NEF, ARW, DNG, RAF…).",
        "Les vignettes apparaissent dans la Bibliothèque ; les réglages sont gardés dans le catalogue.",
    ]),
    t("Noter et trier", "1 à 5", "Étoiles et collections pour retrouver tes meilleures images.", &[
        "Touches 1 à 5 pour noter, 0 pour retirer la note.",
        "Collections : Toutes, Notées (3 étoiles et plus), Retouchées.",
        "Filtre d'étoiles minimum dans la barre de la Bibliothèque.",
    ]),
    t("Développement", "D", "Le module de retouche : la photo au centre, les réglages à droite.", &[
        "D (ou Entrée) ouvre la photo en Développement, G revient à la Bibliothèque.",
        "Flèches gauche/droite : photo suivante ou précédente.",
        "Maintiens \\ ou le bouton Avant / après pour comparer avec l'original.",
        "Double-clic sur le nom d'un réglage pour le remettre à zéro.",
    ]),
    t("Réglage automatique", "Ctrl+U", "Un point de départ équilibré calculé sur la photo.", &["Clique Réglage automatique puis affine à la main."]),
    t("Lumière", "", "Exposition, contraste, hautes lumières, ombres, blancs, noirs.", &[
        "Commence par l'exposition pour la luminosité générale.",
        "Baisse les Hautes lumières pour récupérer un ciel, monte les Ombres pour déboucher les zones sombres.",
        "Blancs et Noirs fixent les extrémités : surveille l'histogramme.",
    ]),
    t("Courbe des tonalités", "", "Ajuste précisément quatre zones de luminosité.", &["Ombres, Tons foncés, Tons clairs, Hautes lumières : chaque curseur agit sur sa zone seulement.", "Une courbe en S (ombres −, hautes lumières +) donne du peps."]),
    t("Couleur", "", "Balance des blancs, vibrance, saturation, noir et blanc.", &[
        "Température : plus froid ou plus chaud. Teinte : vert ou magenta.",
        "Vibrance protège les tons chair et les couleurs déjà saturées, Saturation agit sur tout.",
        "Coche Noir et blanc pour une conversion monochrome, puis règle-la avec le TSL.",
    ]),
    t("TSL", "", "Teinte, saturation et luminance de 8 familles de couleurs.", &[
        "Choisis l'onglet Teinte, Saturation ou Luminance.",
        "Exemple : Bleus en luminance −40 pour assombrir un ciel, Oranges en luminance +15 pour éclaircir la peau.",
        "En noir et blanc, la luminance des couleurs règle leur gris.",
    ]),
    t("Virage partiel", "", "Teinte les ombres et les hautes lumières séparément (look ciné, vintage).", &["Choisis une teinte et une saturation pour les ombres, puis pour les hautes lumières.", "Équilibre décale l'effet vers l'un ou l'autre.", "Classique : ombres bleu-vert, hautes lumières orangées."]),
    t("Présence", "", "Texture, clarté et correction du voile.", &[
        "Texture renforce ou adoucit les petits détails (négatif pour lisser la peau).",
        "Clarté agit sur le contraste des tons moyens : relief et punch.",
        "Correction du voile retire la brume d'un paysage, ou en ajoute en négatif.",
    ]),
    t("Détail", "", "Netteté et réduction du bruit.", &["Netteté 30 à 60 pour une photo nette à l'écran, rayon 1 par défaut.", "Réduction du bruit pour les photos de nuit à ISO élevé, à doser avec la netteté."]),
    t("Effets", "", "Vignettage et grain argentique.", &["Vignettage négatif assombrit les bords, le point milieu règle où il commence.", "Le grain imite la pellicule ; la taille le rend plus fin ou plus gros."]),
    t("Géométrie", "", "Redresser, recadrer, miroir, rotation.", &[
        "Redresser corrige un horizon penché ; les bords vides sont recadrés automatiquement.",
        "Les quatre curseurs de recadrage retirent une marge de chaque côté.",
        "Miroir inverse gauche et droite, les boutons tournent par quart de tour.",
    ]),
    t("Préréglages", "", "14 looks en un clic : Cinéma, Vintage, Paysage, Portrait, Argentique…", &["Clique un préréglage à gauche, puis ajuste les curseurs à ta guise."]),
    t("Copier et coller les réglages", "Ctrl+C / Ctrl+V", "Applique la même retouche à toute une série.", &["Ctrl+C sur la photo réussie, Ctrl+V sur une autre.", "Photo > Coller sur les photos affichées applique à toute la sélection filtrée."]),
    t("Drapeaux et étiquettes", "P / X / U, 6-9", "Trie une série comme un pro : retenues, rejetées, couleurs.", &["P retient, X rejette, U retire le drapeau.", "6 à 9 posent une étiquette rouge, jaune, verte ou bleue.", "Les collections Retenues, Rejetées et chaque couleur filtrent la bibliothèque ; les rejetées sont masquées de Toutes les photos."]),
    t("Sélection multiple", "Ctrl / Maj+clic", "Travaille sur plusieurs photos à la fois.", &["Ctrl+clic ajoute une photo, Maj+clic une plage.", "La barre du haut synchronise les réglages de la photo active, applique un préréglage, retient ou rejette toute la sélection."]),
    t("Recherche et tri", "", "Retrouve une photo en un instant.", &["Tape un bout du nom dans la recherche.", "Trie par nom, par note ou par date de modification."]),
    t("Masques", "", "Retouche locale : ciel, visage, coin sombre.", &["Filtre gradué : assombrit ou réchauffe un côté de l'image, idéal pour un ciel. Glisse sur la photo pour placer la transition.", "Filtre radial : éclaire un sujet ou, coché à l'extérieur, crée un vignetage sur mesure. Glisse pour déplacer le centre.", "Chaque masque a son exposition, sa température et sa saturation."]),
    t("Optique", "", "Corrige les défauts de l'objectif.", &["Correction du vignetage éclaircit les coins assombris par l'objectif.", "Distorsion redresse les lignes bombées (grand-angle) ou creusées (téléobjectif)."]),
    t("Balance des blancs", "", "Préréglages et automatique.", &["Choisis Lumière du jour, Nuageux, Ombre, Tungstène, Fluorescent ou Flash.", "Auto neutralise la dominante de couleur de la photo."]),
    t("Étalonnage", "", "Teinte les ombres, tons moyens et hautes lumières.", &["Choisis une teinte et une intensité pour chaque zone : sarcelle dans les ombres et orangé dans les lumières pour un look cinéma."]),
    t("Format de recadrage", "", "1:1, 4:5, 3:2, 16:9, 9:16 en un clic.", &["Dans Géométrie, clique un format : le recadrage centré est calculé pour toi.", "Affine ensuite avec les curseurs de marge."]),
    t("Comparer", "\\ / Y", "Avant et après, maintenu ou côte à côte.", &["Maintiens le bouton original (ou \\) pour voir la photo d'origine.", "Avant | Après (Y) coupe l'image en deux : glisse le séparateur."]),
    t("Écrêtage et zoom 1:1", "J / Z", "Vérifie les détails perdus et la netteté.", &["Écrêtage (J) montre en rouge les blancs brûlés, en bleu les noirs bouchés.", "100 % (Z ou double-clic) affiche les vrais pixels ; glisse pour te déplacer."]),
    t("Annuler les réglages", "Ctrl+Z / Ctrl+Y", "Reviens en arrière dans la retouche.", &["Ctrl+Z annule le dernier changement de réglage, Ctrl+Y le rétablit."]),
    t("Mes préréglages", "", "Enregistre ton propre look.", &["En bas des préréglages, Enregistrer les réglages actuels puis nomme-le.", "Clic pour l'appliquer, clic droit pour le supprimer."]),
    t("Exporter", "", "JPEG, PNG ou TIFF, pleine définition ou redimensionné, en lot.", &["Fichier > Options d'export : format, qualité, taille du grand côté et suffixe du nom.", "Le bouton Exporter en haut à droite exporte la photo active ou la sélection.", "Exporter la photo… ou Exporter les photos affichées… puis choisis le dossier."]),
];
