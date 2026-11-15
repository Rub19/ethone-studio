use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const SOUND: &[Tuto] = &[
    t("Envoyer vers une autre app", "", "Comme AirDrop entre les apps de la suite : ton travail s'ouvre dans l'autre app en un clic.", &["Fichier > Envoyer vers, puis choisis l'app.", "Si l'app est déjà ouverte, elle reçoit le fichier tout de suite (une notification « Reçu » apparaît en bas à droite). Sinon elle s'ouvre avec ton envoi.", "Les fichiers reçus sont gardés dans le dossier EthoneStudio › handoff › received."]),
    t("Ambiances et voix", "", "Radio, téléphone, mégaphone, sous l'eau, cassette, vinyle, monstre, ange…", &["Rack d'effets > catégorie Ambiances et voix : 20 transformations prêtes, chacune avec son réglage principal et ses préréglages.", "▶ Écouter essaie sur 15 s, Appliquer agit sur la sélection ou tout le fichier."]),
    t("Égaliseurs", "", "Graphique 10 bandes et paramétrique 4 bandes.", &["Rack d'effets > Filtres. Le graphique règle 10 fréquences fixes, le paramétrique 4 cloches libres (fréquence, gain, Q)."]),
    t("Sélection temporelle", "V", "Sélectionne une durée sur la forme d'onde.", &["Glisse pour sélectionner, tire les bords pour ajuster, Maj+clic pour étendre.", "Double-clic : sélectionne l'espace entre deux marqueurs."]),
    t("Main (défilement)", "H", "Fait défiler la vue sans toucher à la sélection.", &["Glisse à gauche ou à droite. H une seconde fois passe au zoom."]),
    t("Zoom", "H", "Zoome au clic ou sur une zone.", &["Clic : zoom avant au point cliqué. Alt+clic ou clic droit : zoom arrière.", "Glisse une zone pour la faire remplir l'écran."]),
    t("Rectangle de fréquences", "E", "Sélectionne une zone temps × fréquences dans le spectrogramme.", &["L'outil passe la vue en spectrogramme. Glisse un rectangle autour du bruit (sifflement, sonnerie).", "Dans la barre d'options : Appliquer le gain (Entrée), Supprimer, Réparer ou Isoler la zone.", "Échelle Log ou Linéaire et taille de FFT règlent la précision de l'affichage."]),
    t("Lasso spectral", "E", "Détoure librement un bruit dans le spectrogramme.", &["Dessine le contour d'une toux ou d'un grincement, puis Supprimer ou Réparer."]),
    t("Pinceau spectral", "E", "Peins les zones à traiter dans le spectrogramme.", &["La taille du pinceau se règle dans la barre d'options. Peins, puis applique un gain ou répare."]),
    t("Correction des taches", "B", "Efface un clic ou un bruit bref en le reconstruisant.", &["Sur la forme d'onde : glisse sur le clic, il est reconstruit au relâchement.", "Sur le spectrogramme : peins sur le bruit, seule la zone peinte est réparée."]),
    t("Crayon", "B", "Redessine la forme d'onde à la main.", &["Zoome jusqu'aux échantillons (bouton dans la barre d'options), puis dessine sur le canal voulu."]),
    t("Rasoir", "C", "Coupe le fichier en morceaux.", &["Clic : pose une coupe (un marqueur). Alt+clic : supprime le segment entre deux coupes.", "Exporter en morceaux crée un fichier par segment."]),
    t("Fondus par poignées", "G", "Fondus d'entrée et de sortie à la souris.", &["Tire les carrés orange : horizontalement la durée, verticalement la courbe.", "Appliquer (Entrée) grave les fondus, double-clic les retire."]),
    t("Enveloppe de volume", "G", "Dessine l'évolution du volume dans le temps.", &["Clic : ajoute un point, glisse pour le déplacer, clic droit pour le retirer.", "Montée, Descente et Creux sont des enveloppes prêtes. Appliquer (Entrée) grave l'enveloppe."]),
    t("Effets", "", "Plus de 60 effets classés, avec préréglages.", &["Choisis une catégorie (dynamique, filtres, restauration, distorsion, échos, réverbération, modulation, voix, stéréo, temps) ou cherche par nom.", "Chaque effet a ses réglages et ses préréglages. ▶ Écouter essaie sans modifier, Appliquer agit sur la sélection ou tout le fichier."]),
    t("Détection", "", "Tempo, mesures et prises.", &["Détecter le tempo estime les BPM ; Marqueur sur chaque mesure pose un repère toutes les 4 temps.", "Repérer les sons pose un marqueur au début de chaque prise séparée par un silence."]),
    t("Ouvrir", "Ctrl+O", "WAV, MP3, FLAC, OGG, M4A, et le son des vidéos MP4/MKV.", &["Fichier > Ouvrir, ou glisse un fichier sur la fenêtre."]),
    t("Exporter", "Ctrl+E", "WAV 16, 24 ou 32 bits, sélection seule ou un fichier par marqueur.", &[
        "Le bouton Exporter ouvre les options : qualité, fréquence (44,1 ou 48 kHz), mono ou stéréo.",
        "Plage : tout, la sélection, ou un fichier par marqueur, nommé d'après le marqueur (idéal pour découper un podcast en chapitres).",
        "Normaliser à −1 dB avant l'export évite toute saturation.",
    ]),
    t("Enregistrer au micro", "R", "Capture ta voix ou un instrument directement dans Sound.", &["Clique ● (ou R) : la capture démarre et les vumètres bougent.", "Clique à nouveau pour arrêter : la prise est insérée au curseur."]),
    t("Lecture", "Espace", "Lit la sélection, ou depuis le curseur.", &["Espace lit ou met en pause, L boucle sur la sélection.", "F fait suivre la lecture par la vue. Les vumètres G/D montrent le niveau en direct."]),
    t("Sélection", "Ctrl+A", "Choisis la partie à modifier.", &[
        "Glisse dans la forme d'onde, puis tire les bords pour ajuster. Maj+clic étend la sélection.",
        "Double-clic sélectionne le passage entre deux marqueurs.",
        "Tape les temps exacts de début et de fin dans la barre de transport.",
        "Sans sélection, les effets s'appliquent à tout le fichier.",
    ]),
    t("Zoom et vue d'ensemble", "Z / Ctrl+0", "Navigue dans les longs enregistrements.", &["Molette : zoom, Maj+molette : défilement, Z : zoom sur la sélection, Ctrl+0 : tout.", "La bande du bas montre tout le fichier : clique ou glisse pour te déplacer.", "Hauteur grossit les sons faibles à l'écran sans les modifier."]),
    t("Spectrogramme", "S", "Voir les fréquences dans le temps.", &["S bascule entre forme d'onde et spectrogramme : graves en bas, aigus en haut, le clair est fort.", "Parfait pour repérer un sifflement, un bourdonnement ou un clic."]),
    t("Couper, copier, coller", "Ctrl+X / C / V", "Montage précis à l'échantillon près.", &["Suppr efface la sélection, Ctrl+T rogne autour, Ctrl+D la duplique juste après.", "Ctrl+V colle au curseur ou remplace la sélection."]),
    t("Marqueurs", "M", "Repères nommés pour les moments clés.", &["M ajoute ou retire un marqueur au curseur ; renomme-le dans le panneau Marqueurs.", "↑ et ↓ sautent de marqueur en marqueur ; ils servent aussi à découper l'export."]),
    t("Écouter avant d'appliquer", "", "Essaie chaque effet sans rien modifier.", &["Chaque effet a un bouton ▶ Écouter : il joue 15 s avec l'effet, la tête de lecture devient jaune.", "Si ça te plaît, clique Appliquer. Ctrl+Z annule toujours."]),
    t("Préréglages", "", "Dix chaînes d'effets en un clic.", &["Voix de podcast nettoie, compresse et règle le volume à −16 LUFS.", "Radio, téléphone, mégaphone, robot, voix grave ou aiguë, lo-fi, grande salle, sous l'eau."]),
    t("Volume", "", "Gain, normalisation, LUFS, limiteur, fondus.", &[
        "Normaliser règle le pic ; le volume perçu (LUFS) règle la puissance ressentie : −14 pour Spotify et YouTube, −16 pour un podcast.",
        "Le limiteur empêche de dépasser un plafond sans écrêter.",
        "Fondus linéaires, logarithmiques (plus naturels) ou en S.",
    ]),
    t("Réduction de bruit", "", "Retire un souffle, un ventilateur ou un bourdonnement constant.", &[
        "Sélectionne un passage où l'on n'entend que le bruit, puis Capturer le profil.",
        "Sélectionne tout, écoute, puis Appliquer. Monte l'intensité doucement pour garder une voix naturelle.",
    ]),
    t("Supprimer les silences", "", "Raccourcit automatiquement les blancs trop longs.", &["Règle le seuil, la durée minimale des pauses et ce qu'il faut en garder, puis Raccourcir les blancs."]),
    t("Égaliseur", "", "Cinq bandes avec courbe en direct.", &["Règle 60 Hz à 12 kHz, la courbe montre le résultat. Préréglages Plat, Voix, Basses, Brillant.", "Coupe-bas et coupe-haut sont dans Nettoyage."]),
    t("Dynamique", "", "Compresseur, saturation, bitcrusher.", &["Compresseur : le seuil fixe où il agit, le ratio sa force, le gain compense.", "Saturation pour la chaleur, bitcrusher pour un son 8 bits."]),
    t("Modulation", "", "Chorus, flanger, trémolo, panoramique, robot.", &["Chorus épaissit, flanger fait « décoller », trémolo fait vibrer le volume, le panoramique promène le son de gauche à droite."]),
    t("Espace", "", "Écho et réverbération.", &["Écho : délai, réinjection et mélange.", "Réverbération : taille de la pièce et mélange."]),
    t("Temps et hauteur", "", "Vitesse, tempo, hauteur.", &["Vitesse change tout comme une bande. Tempo change la durée sans changer la hauteur.", "Hauteur change la voix en demi-tons sans changer la durée."]),
    t("Stéréo", "", "Balance, largeur, mono, inversion.", &["Largeur au-delà de 100 % élargit l'image stéréo. La fréquence et les canaux se changent depuis la pastille en haut."]),
    t("Générer", "", "Tonalités, bruits, balayage, métronome.", &["Sinus, carré, scie ou triangle à la fréquence voulue ; bruit blanc, rose ou brun ; balayage ; clics de métronome au tempo voulu.", "Tout est inséré au curseur."]),
    t("Analyse", "", "Spectre, crête, RMS, LUFS, écrêtage.", &["Le panneau Analyse mesure la sélection ou tout le fichier. En rouge : les échantillons écrêtés, à corriger au limiteur."]),
];
