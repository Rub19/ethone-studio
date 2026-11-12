use studio_ui::help::Tuto;

const fn t(title: &'static str, key: &'static str, summary: &'static str, steps: &'static [&'static str]) -> Tuto {
    Tuto { title, key, summary, steps }
}

pub const SOUND: &[Tuto] = &[
    t("Ouvrir et enregistrer", "Ctrl+O", "WAV, MP3, FLAC, OGG, M4A, et le son des vidéos MP4/MKV.", &["Fichier > Ouvrir, ou glisse un fichier sur la fenêtre.", "Fichier > Exporter en WAV 16, 24 ou 32 bits."]),
    t("Enregistrer au micro", "R", "Capture ta voix ou un instrument directement dans Sound.", &["Clique ● Enregistrer (ou R) : la capture démarre.", "Clique à nouveau pour arrêter : la prise est insérée à la tête de lecture."]),
    t("Lecture", "Espace", "Lit la sélection, ou depuis la tête de lecture.", &["Espace lit ou met en pause.", "L active la lecture en boucle de la sélection.", "Clique dans la forme d'onde pour placer la tête de lecture."]),
    t("Sélection et zoom", "Ctrl+A", "Choisis la partie à modifier.", &["Glisse dans la forme d'onde pour sélectionner.", "Molette : zoom, Maj+molette : défilement, Ctrl+0 : tout afficher.", "Sans sélection, les effets s'appliquent à tout le fichier."]),
    t("Couper, copier, coller", "Ctrl+X / C / V", "Montage précis au sample près.", &["Suppr efface la sélection, Ctrl+T rogne autour.", "Ctrl+V colle à la tête de lecture ou remplace la sélection."]),
    t("Marqueurs", "M", "Repères pour retrouver les moments clés.", &["M ajoute un marqueur à la tête de lecture.", "Flèches haut/bas sautent de marqueur en marqueur."]),
    t("Amplitude", "", "Gain, normalisation, volume perçu, fondus.", &[
        "Normaliser met le pic à −1 dB sans saturer.",
        "Volume perçu égalise la puissance moyenne : pratique pour harmoniser plusieurs voix.",
        "Fondus entrant et sortant sur la sélection.",
    ]),
    t("Égaliseur", "", "Graves, médiums, aigus, coupe-bas, coupe-haut.", &["Règle les trois bandes puis Appliquer.", "Coupe-bas 80 Hz retire les grondements, coupe-haut 12 kHz adoucit le souffle."]),
    t("Dynamique", "", "Compresseur, porte de bruit, saturation.", &[
        "Compresseur : le seuil fixe où il agit, le ratio sa force, le gain de sortie compense.",
        "Porte de bruit : coupe tout ce qui est sous le seuil (souffle entre les phrases).",
        "Saturation : chaleur et grain, à petite dose.",
    ]),
    t("Espace", "", "Écho et réverbération.", &["Écho : délai en millisecondes, réinjection (nombre de répétitions) et mélange.", "Réverbération : taille de la pièce et mélange."]),
    t("Vitesse", "", "Accélère ou ralentit la sélection (la hauteur suit).", &["Règle le pourcentage puis Appliquer : 50 % = deux fois plus lent et plus grave."]),
    t("Stéréo", "", "Mono, inversion des canaux, balance, largeur.", &["Largeur au-delà de 100 % élargit l'image stéréo, en dessous elle la resserre."]),
    t("Générer", "", "Silence, tonalité 440 Hz ou bruit blanc.", &["Choisis la durée puis le type : il est inséré à la tête de lecture."]),
    t("Analyse", "", "Spectre des fréquences à la tête de lecture.", &["Le panneau Analyse montre l'énergie de 20 Hz à 20 kHz : utile pour repérer un bourdonnement à couper à l'égaliseur."]),
];
