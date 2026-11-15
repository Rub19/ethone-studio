use crate::audio::Clip;

pub struct P {
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub def: f32,
    pub unit: &'static str,
    pub log: bool,
    pub int: bool,
    pub choices: &'static [&'static str],
}

const fn p(name: &'static str, min: f32, max: f32, def: f32, unit: &'static str) -> P {
    P { name, min, max, def, unit, log: false, int: false, choices: &[] }
}

const fn hz(name: &'static str, min: f32, max: f32, def: f32) -> P {
    P { name, min, max, def, unit: " Hz", log: true, int: false, choices: &[] }
}

const fn ms(name: &'static str, min: f32, max: f32, def: f32) -> P {
    P { name, min, max, def, unit: " ms", log: true, int: false, choices: &[] }
}

const fn db(name: &'static str, min: f32, max: f32, def: f32) -> P {
    p(name, min, max, def, " dB")
}

const fn int(name: &'static str, min: f32, max: f32, def: f32, unit: &'static str) -> P {
    P { name, min, max, def, unit, log: false, int: true, choices: &[] }
}

const fn pick(name: &'static str, choices: &'static [&'static str], def: f32) -> P {
    P { name, min: 0.0, max: (choices.len() - 1) as f32, def, unit: "", log: false, int: true, choices }
}

pub struct Def {
    pub id: &'static str,
    pub cat: usize,
    pub name: &'static str,
    pub hint: &'static str,
    pub params: &'static [P],
    pub presets: &'static [(&'static str, &'static [f32])],
}

pub const CATS: [&str; 11] = ["Amplitude", "Dynamique", "Filtres", "Restauration", "Distorsion", "Délai et écho", "Réverbération", "Modulation", "Voix et créatif", "Stéréo", "Temps et hauteur"];

const MIX: P = p("Mélange", 0.0, 1.0, 0.4, "");
const FB: P = p("Réinjection", 0.0, 0.9, 0.35, "");

pub const DEFS: &[Def] = &[
    Def { id: "ramp", cat: 0, name: "Rampe de volume", hint: "Monte ou descend le volume progressivement sur la plage.", params: &[db("Début", -40.0, 12.0, 0.0), db("Fin", -40.0, 12.0, -12.0)], presets: &[("Montée", &[-24.0, 0.0]), ("Descente", &[0.0, -24.0]), ("Légère montée", &[-6.0, 0.0])] },
    Def { id: "norm_ch", cat: 0, name: "Normaliser par canal", hint: "Chaque canal atteint la même crête, utile si un côté est plus faible.", params: &[db("Crête", -12.0, 0.0, -1.0)], presets: &[("−0,1 dB", &[-0.1]), ("−1 dB", &[-1.0]), ("−3 dB", &[-3.0])] },
    Def { id: "limit", cat: 0, name: "Limiteur avancé", hint: "Plafond infranchissable avec anticipation de 2 ms.", params: &[db("Plafond", -12.0, 0.0, -1.0), ms("Relâchement", 5.0, 500.0, 50.0)], presets: &[("Transparent", &[-1.0, 120.0]), ("Fort", &[-0.3, 20.0]), ("Diffusion", &[-2.0, 80.0])] },
    Def { id: "invert", cat: 0, name: "Inverser la polarité", hint: "Retourne la forme d'onde (même son, phase opposée).", params: &[], presets: &[] },
    Def { id: "silence", cat: 0, name: "Rendre silencieux", hint: "Remplace la plage par du silence sans changer la durée.", params: &[], presets: &[] },
    Def { id: "comp", cat: 1, name: "Compresseur", hint: "Réduit l'écart entre passages forts et faibles.", params: &[db("Seuil", -60.0, 0.0, -18.0), p("Ratio", 1.0, 20.0, 4.0, ":1"), ms("Attaque", 0.1, 200.0, 5.0), ms("Relâchement", 5.0, 1000.0, 120.0), db("Genou", 0.0, 24.0, 6.0), db("Gain de sortie", -12.0, 24.0, 3.0)], presets: &[("Voix douce", &[-20.0, 2.5, 10.0, 150.0, 6.0, 3.0]), ("Voix serrée", &[-24.0, 6.0, 3.0, 80.0, 4.0, 6.0]), ("Batterie", &[-16.0, 4.0, 20.0, 100.0, 2.0, 4.0]), ("Bus de mix", &[-10.0, 2.0, 30.0, 300.0, 8.0, 1.5]), ("Écrasé", &[-40.0, 20.0, 0.5, 50.0, 0.0, 12.0])] },
    Def { id: "expand", cat: 1, name: "Expandeur", hint: "Baisse encore les passages faibles pour creuser le fond.", params: &[db("Seuil", -80.0, 0.0, -40.0), p("Ratio", 1.0, 10.0, 2.0, ":1"), ms("Attaque", 0.1, 100.0, 2.0), ms("Relâchement", 5.0, 1000.0, 150.0)], presets: &[("Léger", &[-45.0, 1.5, 2.0, 200.0]), ("Fort", &[-35.0, 4.0, 1.0, 100.0])] },
    Def { id: "gate2", cat: 1, name: "Porte avancée", hint: "Coupe sous le seuil avec maintien et plancher réglables.", params: &[db("Seuil", -80.0, -10.0, -45.0), ms("Attaque", 0.1, 50.0, 1.0), ms("Maintien", 1.0, 500.0, 40.0), ms("Relâchement", 5.0, 1000.0, 120.0), db("Plancher", -80.0, 0.0, -80.0)], presets: &[("Voix", &[-50.0, 2.0, 60.0, 200.0, -30.0]), ("Batterie", &[-30.0, 0.2, 20.0, 60.0, -80.0]), ("Doux", &[-55.0, 5.0, 100.0, 400.0, -15.0])] },
    Def { id: "multiband", cat: 1, name: "Compresseur 3 bandes", hint: "Compresse graves, médiums et aigus séparément.", params: &[hz("Coupure basse", 40.0, 1000.0, 200.0), hz("Coupure haute", 1000.0, 12000.0, 3000.0), db("Seuil graves", -60.0, 0.0, -24.0), db("Seuil médiums", -60.0, 0.0, -20.0), db("Seuil aigus", -60.0, 0.0, -22.0), p("Ratio", 1.0, 20.0, 3.0, ":1"), db("Gain", -12.0, 24.0, 3.0)], presets: &[("Voix", &[150.0, 4000.0, -24.0, -18.0, -26.0, 3.0, 3.0]), ("Master", &[120.0, 5000.0, -18.0, -16.0, -18.0, 2.0, 2.0]), ("Radio", &[200.0, 3000.0, -30.0, -26.0, -30.0, 6.0, 6.0])] },
    Def { id: "transient", cat: 1, name: "Modeleur de transitoires", hint: "Renforce ou adoucit l'attaque et la queue des sons.", params: &[p("Attaque", -100.0, 100.0, 40.0, " %"), p("Maintien", -100.0, 100.0, 0.0, " %")], presets: &[("Percutant", &[60.0, -20.0]), ("Adouci", &[-60.0, 0.0]), ("Plus de salle", &[0.0, 60.0]), ("Sec", &[0.0, -70.0])] },
    Def { id: "lp", cat: 2, name: "Passe-bas", hint: "Garde les graves, retire les aigus au-dessus de la fréquence.", params: &[hz("Fréquence", 20.0, 20000.0, 8000.0), p("Q", 0.3, 10.0, 0.707, ""), pick("Pente", &["12 dB/oct", "24 dB/oct", "48 dB/oct"], 0.0)], presets: &[("Étouffé", &[800.0, 0.707, 1.0]), ("Doux", &[6000.0, 0.707, 0.0]), ("Résonant", &[1500.0, 6.0, 0.0])] },
    Def { id: "hp", cat: 2, name: "Passe-haut", hint: "Retire les graves sous la fréquence (souffle de vent, grondements).", params: &[hz("Fréquence", 20.0, 5000.0, 80.0), p("Q", 0.3, 10.0, 0.707, ""), pick("Pente", &["12 dB/oct", "24 dB/oct", "48 dB/oct"], 1.0)], presets: &[("Grondements", &[40.0, 0.707, 1.0]), ("Voix", &[100.0, 0.707, 1.0]), ("Fin", &[400.0, 0.707, 0.0])] },
    Def { id: "bp", cat: 2, name: "Passe-bande", hint: "Ne garde qu'une bande autour du centre.", params: &[hz("Centre", 50.0, 15000.0, 1000.0), p("Q", 0.3, 20.0, 1.0, "")], presets: &[("Téléphone", &[1500.0, 0.8]), ("Radio", &[1800.0, 1.2]), ("Talkie-walkie", &[1700.0, 2.5])] },
    Def { id: "notch", cat: 2, name: "Coupe-bande", hint: "Retire une fréquence précise (sifflement, larsen).", params: &[hz("Fréquence", 20.0, 20000.0, 1000.0), p("Q", 0.5, 50.0, 10.0, "")], presets: &[("Sifflement 1 kHz", &[1000.0, 20.0]), ("Larsen 2,5 kHz", &[2500.0, 30.0]), ("Large", &[1000.0, 1.5])] },
    Def { id: "peak", cat: 2, name: "Cloche", hint: "Monte ou baisse une zone de fréquences.", params: &[hz("Fréquence", 20.0, 20000.0, 1000.0), db("Gain", -24.0, 24.0, 6.0), p("Q", 0.1, 10.0, 1.0, "")], presets: &[("Présence", &[3000.0, 4.0, 1.0]), ("Boue", &[300.0, -5.0, 1.2]), ("Nasal", &[1000.0, -4.0, 2.0])] },
    Def { id: "lshelf", cat: 2, name: "Plateau grave", hint: "Monte ou baisse tout ce qui est sous la fréquence.", params: &[hz("Fréquence", 20.0, 1000.0, 120.0), db("Gain", -24.0, 24.0, 4.0)], presets: &[("Plus de basses", &[100.0, 6.0]), ("Moins de basses", &[150.0, -6.0])] },
    Def { id: "hshelf", cat: 2, name: "Plateau aigu", hint: "Monte ou baisse tout ce qui est au-dessus de la fréquence.", params: &[hz("Fréquence", 1000.0, 18000.0, 8000.0), db("Gain", -24.0, 24.0, 4.0)], presets: &[("Air", &[10000.0, 4.0]), ("Adoucir", &[6000.0, -5.0])] },
    Def { id: "wah", cat: 2, name: "Wah automatique", hint: "Filtre qui balaie les fréquences en rythme.", params: &[p("Vitesse", 0.1, 10.0, 1.5, " Hz"), hz("Fréquence basse", 100.0, 2000.0, 400.0), hz("Fréquence haute", 500.0, 8000.0, 2500.0), p("Q", 0.5, 10.0, 3.0, ""), p("Mélange", 0.0, 1.0, 0.8, "")], presets: &[("Funk", &[3.0, 350.0, 2200.0, 4.0, 0.9]), ("Lent", &[0.3, 300.0, 3000.0, 2.0, 0.7])] },
    Def { id: "declick", cat: 3, name: "Anti-clics", hint: "Détecte les clics et craquements et les reconstruit.", params: &[p("Sensibilité", 1.0, 10.0, 6.0, ""), ms("Largeur max", 0.1, 10.0, 2.0)], presets: &[("Vinyle", &[7.0, 1.5]), ("Doux", &[4.0, 1.0]), ("Agressif", &[9.0, 4.0])] },
    Def { id: "declip", cat: 3, name: "Déécrêtage", hint: "Redessine les crêtes écrêtées puis baisse le gain.", params: &[p("Seuil", 0.7, 1.0, 0.98, "")], presets: &[("Léger", &[0.99]), ("Fort", &[0.9])] },
    Def { id: "dehum", cat: 3, name: "Anti-ronflement", hint: "Retire le bourdonnement du secteur et ses harmoniques.", params: &[pick("Secteur", &["50 Hz", "60 Hz"], 0.0), int("Harmoniques", 1.0, 12.0, 6.0, ""), p("Finesse", 2.0, 60.0, 20.0, "")], presets: &[("Europe", &[0.0, 6.0, 20.0]), ("Amérique", &[1.0, 6.0, 20.0]), ("Fort", &[0.0, 10.0, 8.0])] },
    Def { id: "deess", cat: 3, name: "Dé-esseur", hint: "Adoucit les « s » et « ch » trop sifflants.", params: &[hz("Fréquence", 2000.0, 12000.0, 6000.0), db("Seuil", -60.0, 0.0, -30.0), db("Réduction max", 0.0, 24.0, 10.0)], presets: &[("Léger", &[6500.0, -28.0, 6.0]), ("Fort", &[5500.0, -36.0, 14.0]), ("Voix aiguë", &[7500.0, -30.0, 10.0])] },
    Def { id: "autodenoise", cat: 3, name: "Souffle automatique", hint: "Estime le bruit de fond tout seul et le réduit.", params: &[p("Intensité", 0.1, 1.0, 0.5, "")], presets: &[("Léger", &[0.3]), ("Moyen", &[0.55]), ("Fort", &[0.85])] },
    Def { id: "dc", cat: 3, name: "Supprimer le décalage DC", hint: "Recentre la forme d'onde sur zéro.", params: &[], presets: &[] },
    Def { id: "heal", cat: 3, name: "Reconstruire la plage", hint: "Remplace la sélection par une interpolation de ses bords.", params: &[], presets: &[] },
    Def { id: "dist", cat: 4, name: "Distorsion", hint: "Sept caractères de saturation avec tonalité.", params: &[pick("Type", &["Douce", "Dure", "Fuzz", "Redresseur", "Redresseur double", "Repliement", "Lampe"], 0.0), p("Drive", 0.0, 1.0, 0.4, ""), hz("Tonalité", 500.0, 20000.0, 8000.0), p("Mélange", 0.0, 1.0, 1.0, "")], presets: &[("Chaleur", &[0.0, 0.15, 12000.0, 0.5]), ("Overdrive", &[6.0, 0.5, 6000.0, 1.0]), ("Fuzz", &[2.0, 0.8, 4000.0, 1.0]), ("Octaveur", &[4.0, 0.4, 5000.0, 0.7]), ("Métal", &[1.0, 0.9, 5000.0, 1.0])] },
    Def { id: "crush", cat: 4, name: "Bitcrusher", hint: "Réduit les bits et la fréquence, son 8 bits.", params: &[int("Bits", 1.0, 16.0, 8.0, ""), int("Sous-échantillonnage", 1.0, 32.0, 2.0, "×")], presets: &[("8 bits", &[8.0, 1.0]), ("Console", &[6.0, 4.0]), ("Radio cassée", &[4.0, 8.0])] },
    Def { id: "bitred", cat: 4, name: "Réduire la profondeur", hint: "Quantifie avec dither, comme un export basse résolution.", params: &[int("Bits", 4.0, 24.0, 16.0, ""), pick("Dither", &["Aucun", "Triangulaire", "Mise en forme"], 1.0)], presets: &[("CD", &[16.0, 1.0]), ("12 bits", &[12.0, 1.0]), ("8 bits", &[8.0, 2.0])] },
    Def { id: "ring", cat: 4, name: "Modulation en anneau", hint: "Multiplie par une sinusoïde : métal, robot, cloche.", params: &[hz("Fréquence", 5.0, 5000.0, 60.0), p("Mélange", 0.0, 1.0, 1.0, "")], presets: &[("Dalek", &[30.0, 1.0]), ("Cloche", &[440.0, 0.6]), ("Métallique", &[1200.0, 0.5])] },
    Def { id: "echo", cat: 5, name: "Écho", hint: "Répétitions régulières qui s'éteignent.", params: &[ms("Délai", 20.0, 2000.0, 250.0), FB, MIX], presets: &[("Court", &[120.0, 0.2, 0.3]), ("Long", &[600.0, 0.5, 0.4]), ("Canyon", &[900.0, 0.6, 0.5])] },
    Def { id: "pingpong", cat: 5, name: "Ping-pong", hint: "Les répétitions rebondissent de gauche à droite.", params: &[ms("Délai", 20.0, 2000.0, 300.0), p("Réinjection", 0.0, 0.9, 0.45, ""), MIX], presets: &[("Croches à 120 BPM", &[250.0, 0.45, 0.4]), ("Large", &[400.0, 0.55, 0.5])] },
    Def { id: "slap", cat: 5, name: "Slapback", hint: "Une seule répétition très courte, son rockabilly.", params: &[ms("Délai", 40.0, 200.0, 100.0), p("Niveau", 0.0, 1.0, 0.5, "")], presets: &[("Rockabilly", &[90.0, 0.6]), ("Doublage", &[45.0, 0.4])] },
    Def { id: "multitap", cat: 5, name: "Multi-répétitions", hint: "Plusieurs répétitions espacées, alternées en stéréo.", params: &[ms("Délai", 20.0, 1000.0, 150.0), int("Répétitions", 2.0, 8.0, 4.0, ""), p("Décroissance", 0.1, 1.0, 0.6, ""), p("Espacement", 0.5, 2.0, 1.0, "×"), p("Mélange", 0.0, 1.0, 0.5, "")], presets: &[("Rebonds", &[120.0, 6.0, 0.7, 0.8, 0.5]), ("Régulier", &[200.0, 4.0, 0.6, 1.0, 0.4])] },
    Def { id: "techo", cat: 5, name: "Écho analogique", hint: "Chaque répétition est plus sombre, comme une bande.", params: &[ms("Délai", 20.0, 2000.0, 350.0), p("Réinjection", 0.0, 0.9, 0.5, ""), hz("Tonalité", 300.0, 12000.0, 2500.0), MIX], presets: &[("Bande", &[350.0, 0.55, 2500.0, 0.4]), ("Sombre", &[500.0, 0.6, 1000.0, 0.45])] },
    Def { id: "verb", cat: 6, name: "Réverbération", hint: "Simule une pièce : taille, amortissement, pré-délai.", params: &[p("Taille", 0.0, 1.0, 0.5, ""), p("Amortissement", 0.0, 1.0, 0.5, ""), ms("Pré-délai", 1.0, 200.0, 20.0), p("Largeur", 0.0, 1.0, 1.0, ""), p("Mélange", 0.0, 1.0, 0.3, "")], presets: &[("Petite pièce", &[0.25, 0.6, 5.0, 0.8, 0.2]), ("Studio", &[0.4, 0.5, 10.0, 1.0, 0.22]), ("Salle", &[0.65, 0.45, 25.0, 1.0, 0.3]), ("Grande salle", &[0.8, 0.35, 40.0, 1.0, 0.35]), ("Cathédrale", &[0.97, 0.2, 60.0, 1.0, 0.45]), ("Plaque", &[0.7, 0.1, 1.0, 1.0, 0.3])] },
    Def { id: "revverb", cat: 6, name: "Réverbération inversée", hint: "La réverbération arrive avant le son, effet aspiré.", params: &[p("Taille", 0.0, 1.0, 0.8, ""), p("Mélange", 0.0, 1.0, 0.5, "")], presets: &[("Aspiration", &[0.9, 0.6]), ("Subtile", &[0.6, 0.3])] },
    Def { id: "chorus", cat: 7, name: "Chorus", hint: "Épaissit le son comme plusieurs voix.", params: &[ms("Profondeur", 1.0, 20.0, 6.0), p("Vitesse", 0.1, 5.0, 0.8, " Hz"), p("Mélange", 0.0, 1.0, 0.5, "")], presets: &[("Léger", &[3.0, 0.5, 0.3]), ("Épais", &[10.0, 1.2, 0.6])] },
    Def { id: "flanger", cat: 7, name: "Flanger", hint: "Balayage métallique, effet « avion ».", params: &[p("Vitesse", 0.05, 3.0, 0.25, " Hz"), ms("Profondeur", 0.5, 8.0, 3.0), FB], presets: &[("Avion", &[0.1, 6.0, 0.8]), ("Doux", &[0.4, 2.0, 0.3])] },
    Def { id: "phaser", cat: 7, name: "Phaser", hint: "Creux qui tournent dans le spectre.", params: &[p("Vitesse", 0.05, 5.0, 0.5, " Hz"), p("Profondeur", 0.0, 1.0, 0.7, ""), int("Étages", 2.0, 12.0, 4.0, ""), FB, p("Mélange", 0.0, 1.0, 0.7, "")], presets: &[("Classique", &[0.4, 0.7, 4.0, 0.3, 0.7]), ("Profond", &[0.2, 1.0, 8.0, 0.6, 0.8]), ("Rapide", &[3.0, 0.5, 4.0, 0.2, 0.6])] },
    Def { id: "vibrato", cat: 7, name: "Vibrato", hint: "Fait onduler la hauteur.", params: &[p("Vitesse", 0.5, 12.0, 5.0, " Hz"), ms("Profondeur", 0.1, 10.0, 2.0)], presets: &[("Chanteur", &[5.5, 1.5]), ("Bande usée", &[1.0, 4.0]), ("Extrême", &[8.0, 6.0])] },
    Def { id: "trem", cat: 7, name: "Trémolo", hint: "Fait vibrer le volume.", params: &[p("Vitesse", 0.5, 20.0, 5.0, " Hz"), p("Profondeur", 0.0, 1.0, 0.5, ""), pick("Forme", &["Sinus", "Triangle", "Carré"], 0.0)], presets: &[("Ampli vintage", &[6.0, 0.5, 0.0]), ("Hachoir", &[8.0, 1.0, 2.0])] },
    Def { id: "autopan", cat: 7, name: "Panoramique automatique", hint: "Promène le son de gauche à droite.", params: &[p("Vitesse", 0.05, 8.0, 0.5, " Hz"), p("Amplitude", 0.0, 1.0, 0.8, "")], presets: &[("Lent", &[0.2, 0.8]), ("Rotatif", &[4.0, 0.6])] },
    Def { id: "robot", cat: 8, name: "Robot", hint: "Voix monocorde de robot (phase remise à zéro).", params: &[hz("Hauteur", 40.0, 400.0, 110.0)], presets: &[("Robot grave", &[70.0]), ("Robot", &[110.0]), ("Androïde", &[220.0])] },
    Def { id: "whisper", cat: 8, name: "Chuchotement", hint: "Transforme une voix en murmure (phases aléatoires).", params: &[], presets: &[] },
    Def { id: "vocoder", cat: 8, name: "Vocodeur", hint: "La voix sculpte une onde de synthé.", params: &[pick("Porteuse", &["Scie", "Carré", "Bruit"], 0.0), hz("Note", 40.0, 800.0, 110.0), int("Bandes", 4.0, 48.0, 16.0, "")], presets: &[("Synthé", &[0.0, 110.0, 24.0]), ("Électro", &[1.0, 98.0, 16.0]), ("Souffle", &[2.0, 110.0, 32.0])] },
    Def { id: "stutter", cat: 8, name: "Bégaiement", hint: "Répète de courts blocs en rafale, effet glitch.", params: &[ms("Bloc", 20.0, 500.0, 120.0), int("Répétitions", 2.0, 8.0, 3.0, "")], presets: &[("Glitch", &[60.0, 4.0]), ("DJ", &[250.0, 2.0])] },
    Def { id: "tapestop", cat: 8, name: "Arrêt de bande", hint: "Le son ralentit jusqu'à l'arrêt, ou démarre.", params: &[pick("Sens", &["Ralentir", "Démarrer"], 0.0)], presets: &[] },
    Def { id: "reverse", cat: 8, name: "À l'envers", hint: "Joue la plage à rebours.", params: &[], presets: &[] },
    Def { id: "stereo", cat: 9, name: "Balance et largeur", hint: "Place le son et élargit ou resserre l'image stéréo.", params: &[p("Balance", -1.0, 1.0, 0.0, ""), p("Largeur", 0.0, 200.0, 100.0, " %")], presets: &[("Mono", &[0.0, 0.0]), ("Large", &[0.0, 150.0]), ("Très large", &[0.0, 200.0])] },
    Def { id: "haas", cat: 9, name: "Effet Haas", hint: "Retarde un canal de quelques ms pour élargir.", params: &[ms("Retard", 1.0, 40.0, 15.0), pick("Côté", &["Gauche", "Droite"], 1.0)], presets: &[("Subtil", &[8.0, 1.0]), ("Large", &[25.0, 1.0])] },
    Def { id: "msenc", cat: 9, name: "Encoder en M/S", hint: "Gauche devient le milieu, droite les côtés.", params: &[], presets: &[] },
    Def { id: "msdec", cat: 9, name: "Décoder le M/S", hint: "Revient en gauche/droite depuis milieu/côtés.", params: &[], presets: &[] },
    Def { id: "invl", cat: 9, name: "Inverser la phase gauche", hint: "Corrige un canal câblé à l'envers.", params: &[], presets: &[] },
    Def { id: "invr", cat: 9, name: "Inverser la phase droite", hint: "Corrige un canal câblé à l'envers.", params: &[], presets: &[] },
    Def { id: "extl", cat: 9, name: "Extraire le canal gauche", hint: "Copie le canal gauche des deux côtés.", params: &[], presets: &[] },
    Def { id: "extr", cat: 9, name: "Extraire le canal droit", hint: "Copie le canal droit des deux côtés.", params: &[], presets: &[] },
    Def { id: "center", cat: 9, name: "Isoler le centre", hint: "Garde ce qui est commun aux deux canaux.", params: &[], presets: &[] },
    Def { id: "karaoke", cat: 9, name: "Supprimer le centre", hint: "Retire la voix centrée (karaoké).", params: &[], presets: &[] },
    Def { id: "mono", cat: 9, name: "Convertir en mono", hint: "Mélange les canaux.", params: &[], presets: &[] },
    Def { id: "swap", cat: 9, name: "Échanger gauche et droite", hint: "Permute les deux canaux.", params: &[], presets: &[] },
    Def { id: "speed", cat: 10, name: "Vitesse", hint: "Comme une bande : la hauteur suit.", params: &[p("Vitesse", 25.0, 400.0, 100.0, " %")], presets: &[("Demi", &[50.0]), ("×1,25", &[125.0]), ("Double", &[200.0])] },
    Def { id: "tempo", cat: 10, name: "Tempo", hint: "Change la durée sans changer la hauteur.", params: &[p("Tempo", 50.0, 200.0, 100.0, " %")], presets: &[("−10 %", &[90.0]), ("+10 %", &[110.0]), ("+25 %", &[125.0])] },
    Def { id: "pitch", cat: 10, name: "Hauteur", hint: "Change la hauteur sans changer la durée.", params: &[p("Demi-tons", -24.0, 24.0, 0.0, ""), p("Centièmes", -100.0, 100.0, 0.0, "")], presets: &[("Octave grave", &[-12.0, 0.0]), ("Quinte", &[7.0, 0.0]), ("Quarte", &[5.0, 0.0]), ("Tierce", &[4.0, 0.0]), ("Chipmunk", &[9.0, 0.0]), ("Démon", &[-7.0, 0.0]), ("Accordage 432 Hz", &[0.0, -32.0])] },
];

pub fn defaults(k: usize) -> Vec<f32> {
    DEFS[k].params.iter().map(|p| p.def).collect()
}

pub fn run(id: &str, c: &mut Clip, a: usize, b: usize, p: &[f32]) -> Option<String> {
    let v = |k: usize| p.get(k).copied().unwrap_or(0.0);
    match id {
        "ramp" => c.ramp(a, b, v(0), v(1)),
        "norm_ch" => c.normalize_each(a, b, v(0)),
        "limit" => c.limit(a, b, v(0), v(1)),
        "invert" => c.invert(a, b),
        "silence" => c.silence(a, b),
        "comp" => c.compress_ex(a, b, v(0), v(1), v(2), v(3), v(4), v(5)),
        "expand" => c.expand(a, b, v(0), v(1), v(2), v(3)),
        "gate2" => c.gate_ex(a, b, v(0), v(1), v(2), v(3), v(4)),
        "multiband" => c.multiband(a, b, v(0), v(1), [v(2), v(3), v(4)], v(5), v(6)),
        "transient" => c.transient(a, b, v(0), v(1)),
        "lp" | "hp" => c.band(a, b, if id == "lp" { 3 } else { 4 }, v(0), 0.0, v(1), [1, 2, 4][v(2) as usize % 3]),
        "bp" => c.band(a, b, 5, v(0), 0.0, v(1), 1),
        "notch" => c.band(a, b, 6, v(0), 0.0, v(1), 1),
        "peak" => c.band(a, b, 0, v(0), v(1), v(2), 1),
        "lshelf" => c.band(a, b, 1, v(0), v(1), 0.707, 1),
        "hshelf" => c.band(a, b, 2, v(0), v(1), 0.707, 1),
        "wah" => c.auto_wah(a, b, v(0), v(1), v(2), v(3), v(4)),
        "declick" => return Some(format!("{} clics réparés", c.declick(a, b, v(0), v(1)))),
        "declip" => return Some(format!("{} crêtes reconstruites", c.declip(a, b, v(0)))),
        "dehum" => c.dehum(a, b, if v(0) < 0.5 { 50.0 } else { 60.0 }, v(1) as usize, v(2)),
        "deess" => c.deess(a, b, v(0), v(1), v(2)),
        "autodenoise" => c.auto_denoise(a, b, v(0)),
        "dc" => c.remove_dc(a, b),
        "heal" => c.heal(a, b),
        "dist" => c.distort(a, b, v(0) as u8, v(1), v(2), v(3)),
        "crush" => c.bitcrush(a, b, v(0), v(1) as usize),
        "bitred" => c.bit_reduce(a, b, v(0) as u32, v(1) as u8),
        "ring" => c.ring_mix(a, b, v(0), v(1)),
        "echo" => c.echo(a, b, v(0), v(1), v(2)),
        "pingpong" => c.pingpong(a, b, v(0), v(1), v(2)),
        "slap" => c.echo(a, b, v(0), 0.0, v(1)),
        "multitap" => c.multitap(a, b, v(0), v(1) as usize, v(2), v(3), v(4)),
        "techo" => c.echo_filtered(a, b, v(0), v(1), v(2), v(3)),
        "verb" => c.freeverb(a, b, v(0), v(1), v(2), v(3), v(4)),
        "revverb" => c.reverse_reverb(a, b, v(0), v(1)),
        "chorus" => c.mod_delay(a, b, 20.0, v(0), v(1), 0.0, v(2)),
        "flanger" => c.mod_delay(a, b, 1.5, v(1), v(0), v(2), 0.7),
        "phaser" => c.phaser(a, b, v(0), v(1), v(2) as usize, v(3), v(4)),
        "vibrato" => c.vibrato(a, b, v(0), v(1)),
        "trem" => c.tremolo_shape(a, b, v(0), v(1), v(2) as u8),
        "autopan" => c.auto_pan(a, b, v(0), v(1)),
        "robot" => c.robotize(a, b, v(0)),
        "whisper" => c.whisper(a, b),
        "vocoder" => c.vocode(a, b, v(0) as u8, v(1), v(2) as usize),
        "stutter" => c.stutter(a, b, v(0), v(1) as usize),
        "tapestop" => c.tape_stop(a, b, v(0) > 0.5),
        "reverse" => c.reverse(a, b),
        "stereo" => c.stereo(a, b, v(0), v(1) / 100.0),
        "haas" => c.haas(a, b, v(0), v(1) as usize),
        "msenc" => c.ms_encode(a, b, false),
        "msdec" => c.ms_encode(a, b, true),
        "invl" => c.invert_ch(a, b, 0),
        "invr" => c.invert_ch(a, b, 1),
        "extl" => c.extract(a, b, 0),
        "extr" => c.extract(a, b, 1),
        "center" => c.extract(a, b, 2),
        "karaoke" => c.extract(a, b, 3),
        "mono" => {
            if c.ch.len() > 1 {
                for i in a..b {
                    let m = (c.ch[0][i] + c.ch[1][i]) * 0.5;
                    c.ch[0][i] = m;
                    c.ch[1][i] = m;
                }
            }
        }
        "swap" => {
            if c.ch.len() > 1 {
                for i in a..b {
                    let (l, r) = (c.ch[0][i], c.ch[1][i]);
                    c.ch[0][i] = r;
                    c.ch[1][i] = l;
                }
            }
        }
        "speed" => c.speed(a, b, v(0) / 100.0),
        "tempo" => c.stretch(a, b, 100.0 / v(0).max(1.0)),
        "pitch" => c.pitch(a, b, v(0) + v(1) / 100.0),
        _ => {}
    }
    None
}

#[cfg(test)]
pub fn option_count() -> usize {
    DEFS.iter().map(|d| 1 + d.params.iter().map(|p| p.choices.len().max(1)).sum::<usize>() + d.presets.len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_effect_runs() {
        for (k, d) in DEFS.iter().enumerate() {
            assert!(d.presets.iter().all(|p| p.1.len() == d.params.len()), "{}", d.id);
            assert!(d.params.iter().all(|p| p.def >= p.min && p.def <= p.max), "{}", d.id);
            let n = 8000;
            let s: Vec<f32> = (0..n).map(|i| (i as f32 * 0.07).sin() * 0.4).collect();
            let mut c = Clip { rate: 8000, ch: vec![s.clone(), s] };
            run(d.id, &mut c, 1000, 7000, &defaults(k));
            assert!(c.ch.iter().flatten().all(|v| v.is_finite()), "{}", d.id);
            assert!(c.len() > 0, "{}", d.id);
        }
        assert!(option_count() > 300, "{}", option_count());
    }
}
