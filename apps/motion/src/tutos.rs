use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const MOTION: &[Tuto] = &[
    t("Masques rectangulaire et elliptique", "", "Comme dans After Effects : ne garde visible qu'une zone d'un calque.", &["Sélectionne le calque, prends l'outil Masque et trace la zone dans la vue.", "Le masque est un calque blanc placé juste en dessous et utilisé comme cache alpha. Coche « Inversé » pour cacher la zone au lieu de la garder.", "Redessiner remplace le masque existant. Anime-le comme n'importe quel calque."]),
    t("Marionnette", "", "Tire une partie d'un tracé ou d'une peinture : les points proches suivent en douceur.", &["Sélectionne un calque Tracé (plume) ou Peinture, puis glisse depuis la zone à déplacer.", "La portée règle jusqu'où la déformation s'étend."]),
    t("Envoyer vers une autre app", "", "Comme AirDrop entre les apps de la suite : ton travail s'ouvre dans l'autre app en un clic.", &["Fichier > Envoyer vers, puis choisis l'app.", "Si l'app est déjà ouverte, elle reçoit le fichier tout de suite (une notification « Reçu » apparaît en bas à droite). Sinon elle s'ouvre avec ton envoi.", "Les fichiers reçus sont gardés dans le dossier EthoneStudio › handoff › received."]),
    t("Effets de composition", "", "Plus de 100 effets image appliqués à toute l'animation.", &["Panneau Effets de composition > + Ajouter un effet : looks, lueur, glitch, VHS, écran cathodique, neige, pluie…", "Chaque effet a son intensité, ses réglages et ses préréglages ; ils s'appliquent à l'aperçu et à tous les exports."]),
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
    t("Animations prêtes", "", "45 animations classées : entrées, sorties, boucles, effets, 3D, tracés, texte.", &[
        "Sélectionne un calque, place la tête, clique une animation. La recherche filtre la liste.",
        "Elles créent de vraies images clés que tu peux ajuster ensuite.",
    ]),
    t("Modèles", "", "Bandeau titre, titre cinéma, bouton S'abonner, compte à rebours, barre de progression, citation, coche validée, titre lettre par lettre.", &[
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
        "Sur le calque du dessous, choisis Masque > Alpha (la forme) ou Luminance (le clair laisse passer), ou leurs versions inversées pour voir autour.",
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
    t("Exporter", "Ctrl+M", "MP4, GIF, séquences PNG ou SVG, image, planche contact, Lottie.", &[
        "MP4 : choisis la qualité (débit). GIF : images par seconde, nombre de boucles et qualité de palette.",
        "Plage : toute la composition, la zone de travail ou des images précises. Séquences : préfixe, premier numéro, une image sur N.",
        "Planche : une grille d'images pour présenter l'animation. Lottie JSON : formes, tracés et transformations pour le web et les apps.",
    ]),
    t("File de rendu", "Ctrl+Alt+M", "Plusieurs exports d'un coup.", &[
        "Dans Exporter, règle un format puis « Ajouter à la file » et choisis le fichier.",
        "Recommence pour d'autres formats, puis ouvre la file et lance le rendu : tout s'enchaîne en arrière-plan.",
    ]),
    t("Barre d'outils", "V H Z W Y Q G", "Les outils sont regroupés comme dans Adobe.", &[
        "Le petit triangle d'une case indique d'autres outils : clic droit ou clic maintenu pour les voir.",
        "Appuyer de nouveau sur le raccourci fait défiler les outils du groupe (Q : rectangle, arrondi, ellipse, polygone, étoile).",
        "La barre au-dessus de la vue montre les options de l'outil actif : couleurs, contour, côtés, police, taille du pinceau…",
    ]),
    t("Formes dessinées", "Q", "Rectangle, rectangle arrondi, ellipse, polygone, étoile.", &[
        "Glisse dans la vue pour tracer la forme : Maj garde les proportions, Alt part du centre.",
        "Un simple clic pose une forme de taille standard.",
    ]),
    t("Plume et tracés", "G", "Dessine des courbes de Bézier.", &[
        "Clic : un sommet en angle. Clic-glisser : un sommet courbe avec ses poignées. Clic sur le premier sommet : fermer. Entrée ou Échap : terminer.",
        "Ajout de sommet : clic sur un segment. Suppression : clic sur un sommet. Conversion : passe d'angle à courbe ou tire de nouvelles poignées.",
        "Avec la Sélection, glisse les sommets et les poignées (Alt casse la symétrie).",
    ]),
    t("Couper le tracé", "", "Fait apparaître un contour comme s'il se dessinait.", &[
        "Dans Tracé et opérateurs, anime Début, Fin et Décalage (en %). Marche sur les tracés, formes, lignes, anneaux et peintures.",
        "« Un par un » dessine plusieurs traits à la suite. Animations prêtes : Tracé dessiné, Tracé effacé, Tracé qui court.",
    ]),
    t("Opérateurs de forme", "", "Arrondir les angles, fronce et dilatation, zigzag, agitation, torsion.", &[
        "Ils déforment la silhouette sans toucher au calque : une étoile aux pointes rondes, un cercle ondulé, une forme qui frémit.",
        "Agitation avec une vitesse > 0 fait vibrer le contour en continu.",
    ]),
    t("Texte", "Ctrl+T", "Texte horizontal ou vertical, posé d'un clic.", &[
        "Clique dans la vue avec l'outil Texte : le calque est créé et le champ de saisie s'active. Ctrl+T bascule entre horizontal et vertical.",
    ]),
    t("Animateur de texte", "", "Anime les lettres une par une.", &[
        "Active « Animer lettre par lettre » : choisis entrée, sortie ou les deux, l'ordre (gauche → droite, aléatoire, centre…), le décalage et la courbe.",
        "Règle les valeurs de départ : opacité, décalage, échelle et rotation de chaque lettre.",
    ]),
    t("Pinceau et gomme", "Ctrl+B", "Peins à main levée sur un calque Peinture.", &[
        "Glisse dans la vue : chaque trait est lissé et reste vectoriel. La gomme coupe les traits qu'elle touche.",
        "Anime Fin du tracé avec « Un par un » pour rejouer le dessin trait après trait.",
    ]),
    t("Point d'ancrage", "Y", "Le centre de rotation et d'échelle.", &[
        "Avec l'outil Point d'ancrage, glisse la croix : le calque ne bouge pas mais tournera autour du nouveau point.",
        "Les boutons Centre, Haut, Bas, Gauche, Droite le placent sur la boîte du calque. Ctrl+Alt+Origine le recentre.",
    ]),
    t("Zoom et vue", "Z / . / ,", "Zoom avant et arrière, règles et repères.", &[
        "Outil Zoom : clic pour zoomer sur le point, Alt+clic pour reculer. Point et virgule zooment aussi.",
        "Ctrl+R affiche les règles : glisse depuis une règle pour tirer un repère, l'accroche aimante les calques.",
        "Damier de transparence, pelure d'oignon (images voisines en rouge et bleu) et qualité d'aperçu sont dans la barre de la vue.",
    ]),
    t("Nouvelles propriétés", "P S R T A", "Ancre, échelle X/Y, rotation 3D, teinte animée.", &[
        "Rotation X et Y donnent une fausse perspective : parfait pour une carte qui se retourne.",
        "P, S, R, T et A n'affichent dans la timeline que la position, l'échelle, la rotation, l'opacité ou l'ancre ; U revient aux propriétés animées.",
    ]),
    t("Éditeur de courbe", "", "Courbe de Bézier sur mesure entre deux clés.", &[
        "Clique une clé : l'Éditeur de courbe montre le trajet. Glisse les deux poignées ou choisis un préréglage (Material, Expo, Fluide Apple…).",
        "« Vitesse » affiche le graphique de vitesse. « Copier sur la sélection » applique la courbe à toutes les clés choisies.",
    ]),
    t("Sélection de clés", "Maj+clic", "Déplacer, copier et coller plusieurs clés.", &[
        "Maj ou Ctrl+clic ajoute une clé à la sélection, Ctrl+A prend toutes celles du calque. Glisse-en une : tout le groupe suit.",
        "Ctrl+C / Ctrl+V copie les clés et les colle à la tête de lecture ; Suppr les efface.",
        "Maj+F9 ralentit à l'arrivée, Ctrl+Maj+F9 démarre en douceur, Ctrl+Alt+H bascule le maintien.",
    ]),
    t("Expressions", "", "Tremblement et boucles sur n'importe quelle propriété.", &[
        "Clic droit sur le nom d'une propriété : règle un tremblement (amplitude, fréquence) et ce qui se passe après la dernière clé.",
        "Cycle rejoue, Aller-retour fait la navette, Décalage cumule (une rotation infinie), Continuer prolonge la vitesse.",
    ]),
    t("Calques spéciaux", "", "Nul, calque d'effets, solide, peinture.", &[
        "Un nul est invisible : sers-t'en comme parent. Calque > Créer un nul parent le fait pour toi.",
        "Un calque d'effets applique ses réglages (couleur, flou, grain…) à tout ce qui est en dessous ; son opacité dose l'effet.",
    ]),
    t("Effets avancés", "", "Couleur, stylisation, flous, lumière.", &[
        "Couleur : contraste, gamma, sépia, teinte bicolore, inversion, seuil, postérisation, remplissage.",
        "Stylisation : séparation RVB, glitch, grain, distorsion, scintillement. Flous : directionnel et radial. Ombre interne.",
        "Finition de l'image (sans calque sélectionné) : vignettage, grain film et bandes cinéma pour toute la composition.",
    ]),
    t("Temps du calque", "[ ] Alt+[ Alt+]", "Vitesse, inversion, image figée.", &[
        "Panneau Temps : vitesse en %, décalage, lecture inversée ou image figée.",
        "[ et ] déplacent le calque pour qu'il commence ou finisse à la tête ; Alt+[ et Alt+] le coupent.",
    ]),
    t("Trajectoires", "", "Le chemin d'un calque animé.", &[
        "Les points montrent chaque image, les carrés jaunes les clés de position : glisse un carré pour modifier la trajectoire.",
        "Orientation auto fait tourner le calque dans le sens de son déplacement.",
    ]),
];
