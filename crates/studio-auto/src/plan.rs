//! Turns a French (or English) request plus an optional file into a plan of steps.

use serde_json::{Value, json};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Image,
    Raw,
    Video,
    Audio,
    Pdf,
    Vector,
    Layout,
    Anim,
    Montage,
    Other,
    Nothing,
}

pub fn kind_of(file: Option<&Path>) -> Kind {
    let Some(f) = file else { return Kind::Nothing };
    let ext = f.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "tif" | "tiff" => Kind::Image,
        "cr2" | "cr3" | "nef" | "arw" | "dng" | "raf" | "orf" | "rw2" => Kind::Raw,
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" => Kind::Video,
        "wav" | "mp3" | "flac" | "ogg" | "m4a" | "aac" => Kind::Audio,
        "pdf" => Kind::Pdf,
        "svg" | "evec" => Kind::Vector,
        "epage" => Kind::Layout,
        "emot" => Kind::Anim,
        "ecut" => Kind::Montage,
        _ => Kind::Other,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// pixel_edit operations; `{"op":"square"}` and `{"op":"upscale","factor":2}` are resolved with the image size.
    Pixel(Vec<Value>),
    /// lumen_develop settings
    Develop(Value),
    /// Find these objects with the vision model and remove them.
    Remove(String),
    /// OCR then remove every line of text.
    EraseText,
    Sound(Vec<Value>),
    ExtractAudio,
    Animate,
    Logo(String),
    Title(String),
    Tone(f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// What will happen, in words, one line per step.
    pub summary: Vec<String>,
    pub steps: Vec<Step>,
    /// App that opens the result.
    pub app: &'static str,
    /// Extra launch arguments for that app (placed before the file).
    pub args: Vec<String>,
}

pub fn norm(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.to_lowercase().chars() {
        o.push(match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            '’' | '\'' | '-' | ',' | '.' | '!' | '?' | ';' | ':' | '"' => ' ',
            c => c,
        });
    }
    format!(" {} ", o.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn has(t: &str, words: &[&str]) -> bool {
    words.iter().any(|w| t.contains(&format!(" {w}")))
}

fn number_after(t: &str, words: &[&str]) -> Option<f32> {
    for w in words {
        if let Some(i) = t.find(&format!(" {w}")) {
            let rest = &t[i + w.len() + 1..];
            if let Some(n) = rest.split_whitespace().take(4).find_map(|x| x.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<f32>().ok()) {
                return Some(n);
            }
        }
    }
    None
}

const REMOVE: [&str; 9] = ["retire", "enleve", "supprime", "efface", "vire", "remove", "erase", "fais disparaitre", "ote"];
const STOP: [&str; 14] = [" de la photo", " de l image", " sur la photo", " sur l image", " dans la photo", " dans l image", " de ma photo", " stp", " svp", " s il te plait", " from the", " in the", " de cette photo", " de cette image"];

/// The object named after a removal verb: "retire les oiseaux du ciel" → "oiseaux du ciel".
pub fn object_of(t: &str) -> Option<String> {
    let (i, w) = REMOVE.iter().filter_map(|w| t.find(&format!(" {w}")).map(|i| (i, *w))).min_by_key(|x| x.0)?;
    let mut rest = t[i + w.len() + 1..].trim_start_matches(|c: char| c.is_alphabetic()).to_string();
    rest = format!(" {} ", rest.trim());
    for s in STOP {
        if let Some(k) = rest.find(s) {
            rest.truncate(k);
        }
    }
    let mut words: Vec<&str> = rest.split_whitespace().collect();
    let articles = ["le", "la", "les", "l", "un", "une", "des", "du", "de", "d", "tous", "toutes", "ce", "ces", "cette", "the", "a"];
    if let Some(k) = words.iter().enumerate().position(|(i, w)| ["puis", "ensuite", "apres", "then"].contains(w) || (["et", "and"].contains(w) && words.get(i + 1).is_some_and(|n| !articles.contains(n)))) {
        words.truncate(k);
    }
    while words.first().is_some_and(|w| ["le", "la", "les", "l", "un", "une", "des", "du", "de", "d", "tous", "toutes", "tout", "ce", "ces", "cette", "cet", "mon", "ma", "mes", "the", "all", "a", "moi"].contains(w)) {
        words.remove(0);
    }
    (!words.is_empty()).then(|| words.join(" "))
}

fn image_filters(t: &str, s: &mut Vec<String>) -> (Vec<Value>, serde_json::Map<String, Value>) {
    let mut ops = Vec::new();
    let mut dev = serde_json::Map::new();
    let mut set = |k: &str, v: Value, label: &str, s: &mut Vec<String>| {
        dev.insert(k.into(), v);
        s.push(label.into());
    };
    let less = has(t, &["moins", "reduis", "baisse", "diminue"]);
    if has(t, &["noir et blanc", "n&b", "nb ", "monochrome", "black and white", "gris"]) {
        set("mono", json!(true), "Passer en noir et blanc", s);
    }
    if has(t, &["eclaircis", "plus clair", "plus lumineu", "lumineu", "brighter", "eclaire", "trop sombre"]) {
        set("exposure", json!(0.7), "Éclaircir (+0,7 IL)", s);
        set("shadows", json!(30), "Déboucher les ombres", s);
    }
    if has(t, &["assombris", "plus sombre", "darker", "trop clair", "surexpos"]) {
        set("exposure", json!(-0.6), "Assombrir (−0,6 IL)", s);
        set("highlights", json!(-30), "Récupérer les hautes lumières", s);
    }
    if has(t, &["contraste", "contrast", "punch"]) {
        set("contrast", json!(if less { -30 } else { 30 }), if less { "Baisser le contraste" } else { "Renforcer le contraste" }, s);
    }
    if has(t, &["chaud", "chaleureu", "warm", "doree", "dore", "coucher de soleil"]) {
        set("temperature", json!(25), "Réchauffer les couleurs", s);
    }
    if has(t, &["froid", "cool", "bleute"]) {
        set("temperature", json!(-25), "Refroidir les couleurs", s);
    }
    if has(t, &["couleurs vives", "vives", "sature", "eclatant", "pop", "vibrant", "plus de couleur"]) && !has(t, &["pop art", "desature"]) {
        set("vibrance", json!(if less { -30 } else { 40 }), "Couleurs plus vives", s);
    }
    if has(t, &["desature", "terne", "delave"]) {
        set("saturation", json!(-50), "Désaturer", s);
    }
    if has(t, &["nette", "nettete", "net ", "accentue", "detail", "sharp", "plus precis"]) {
        set("sharpen", json!(80), "Accentuer la netteté", s);
        set("clarity", json!(20), "Ajouter de la clarté", s);
    }
    if has(t, &["brume", "voile", "brouillard", "haze", "dehaze"]) && has(t, &["retire", "enleve", "supprime", "reduis", "dehaze"]) {
        set("dehaze", json!(45), "Retirer la brume", s);
    }
    if has(t, &["bruit", "grain", "noise"]) && has(t, &["retire", "enleve", "supprime", "reduis", "debruite", "nettoie"]) {
        set("noise", json!(55), "Réduire le bruit", s);
    } else if has(t, &["grain", "argentique", "film"]) {
        set("grain", json!(30), "Ajouter du grain", s);
    }
    if has(t, &["vignet"]) {
        set("vignette", json!(-30), "Vignettage", s);
    }
    let filters: [(&[&str], &str, &str); 16] = [
        (&["sepia", "vintage", "ancienne", "vieille photo", "retro"], "Sépia", "Effet sépia"),
        (&["croquis", "dessin au crayon", "crayon", "sketch"], "Croquis au crayon", "Croquis au crayon"),
        (&["peinture", "huile", "tableau", "painting"], "Peinture à l'huile", "Peinture à l'huile"),
        (&["bande dessinee", "bd ", "cartoon", "comic"], "Bande dessinée", "Bande dessinée"),
        (&["pixel art", "pixelise", "mosaique"], "Mosaïque", "Mosaïque"),
        (&["negatif", "inverse les couleurs"], "Négatif", "Négatif"),
        (&["pop art", "warhol"], "PopArt", "Pop art"),
        (&["aquarelle", "watercolor"], "Watercolor", "Aquarelle"),
        (&["glitch"], "Glitch", "Glitch"),
        (&["neige", "snow"], "Snow", "Ajouter de la neige"),
        (&["pluie", "rain"], "Rain", "Ajouter de la pluie"),
        (&["flou artistique", "flou d arriere", "bokeh"], "LensBlur", "Flou d'objectif"),
        (&["floute tout", "flou gaussien", "rends flou", "floute la photo", "floute l image"], "Flou gaussien", "Flou gaussien"),
        (&["ameliore", "embelli", "rends la belle", "plus belle", "corrige", "retouche auto", "auto"], "Ton automatique", "Ton automatique"),
        (&["relight", "reeclaire", "lumiere de studio"], "Relight", "Rééclairer"),
        (&["film", "pellicule", "kodak", "argentique"], "FilmSim", "Simulation de pellicule"),
    ];
    for (words, name, label) in filters {
        if has(t, words) {
            ops.push(json!({ "op": "filter", "name": name }));
            s.push(label.into());
        }
    }
    if has(t, &["carre", "square", "instagram"]) && has(t, &["recadre", "format", "crop", "coupe", "carre"]) {
        ops.push(json!({ "op": "square" }));
        s.push("Recadrer au format carré".into());
    }
    if has(t, &["agrandi", "upscale", "haute resolution", "x2", "×2", "double la taille"]) {
        ops.push(json!({ "op": "upscale", "factor": if has(t, &["x4", "×4"]) { 4 } else { 2 } }));
        s.push("Agrandir l'image".into());
    }
    if let Some(n) = number_after(t, &["redimensionne", "reduis", "largeur", "resize", "taille"]).filter(|n| *n >= 16.0) {
        ops.push(json!({ "op": "resize", "width": n }));
        s.push(format!("Redimensionner à {n:.0} px de large"));
    }
    if has(t, &["tourne", "pivote", "rotate"]) {
        let d = if has(t, &["gauche", "left"]) { 270 } else if has(t, &["180", "envers", "tete en bas"]) { 180 } else { 90 };
        ops.push(json!({ "op": "rotate", "degrees": d }));
        s.push(format!("Pivoter de {d}°"));
    }
    if has(t, &["miroir", "symetrie", "retourne", "flip"]) {
        ops.push(json!({ "op": "flip", "axis": if has(t, &["vertical", "haut en bas"]) { "vertical" } else { "horizontal" } }));
        s.push("Retourner en miroir".into());
    }
    (ops, dev)
}

fn quoted_or_after(raw: &str, t: &str, words: &[&str]) -> String {
    if let Some(a) = raw.find(['"', '«', '“']) {
        let rest = &raw[a + raw[a..].chars().next().map_or(1, char::len_utf8)..];
        if let Some(b) = rest.find(['"', '»', '”']) {
            return rest[..b].trim().to_string();
        }
    }
    for w in words {
        if let Some(i) = t.find(&format!(" {w} ")) {
            let after: Vec<&str> = t[i + w.len() + 2..].split_whitespace().filter(|x| !["pour", "de", "du", "nomme", "appele", "qui", "s", "appelle", "avec", "le", "la", "texte", "mon", "ma", "entreprise", "marque", "societe"].contains(x)).take(4).collect();
            if !after.is_empty() {
                let raw_words: Vec<&str> = raw.split_whitespace().collect();
                let cap: Vec<String> = after.iter().map(|w| raw_words.iter().find(|r| norm(r).trim() == *w).map_or(w.to_string(), |r| r.trim_matches(|c: char| !c.is_alphanumeric()).to_string())).collect();
                return cap.join(" ");
            }
        }
    }
    String::new()
}

pub fn plan(prompt: &str, file: Option<&Path>) -> Result<Plan, String> {
    let t = norm(prompt);
    let kind = kind_of(file);
    let mut s = Vec::new();
    let mut steps = Vec::new();
    let done = |app: &'static str, args: &[&str], s: Vec<String>, steps: Vec<Step>| Ok(Plan { summary: s, steps, app, args: args.iter().map(|a| a.to_string()).collect() });
    match kind {
        Kind::Image | Kind::Raw => {
            let text_words = ["texte", "ecriture", "mots", "lettres", "filigrane", "watermark", "inscription", "text"];
            if has(&t, &["traduis", "translate", "modifie le texte", "change le texte", "remplace le texte", "corrige le texte", "edite le texte", "reecris"]) {
                return done("pixel", &["--texte-photo"], vec!["Ouvrir Pixel avec l'outil Texte de la photo et lire le texte".into()], steps);
            }
            if has(&t, &["lis ", "lire", "ocr", "extrais le texte", "recupere le texte", "copie le texte", "transcris", "scan"]) {
                return done("folio", &["--ocr"], vec!["Convertir en PDF et reconnaître le texte (OCR) dans Folio".into()], steps);
            }
            if has(&t, &["pdf"]) {
                return done("folio", &[], vec!["Ouvrir l'image en PDF dans Folio".into()], steps);
            }
            if let Some(obj) = object_of(&t) {
                if text_words.iter().any(|w| obj.contains(w)) {
                    steps.push(Step::EraseText);
                    s.push("Lire le texte (OCR) et l'effacer en reconstruisant le fond".into());
                } else if has(&t, &["fond", "arriere plan", "background", "decor"]) && !has(&t, &["sur le fond", "du fond", "dans le fond"]) {
                    return done("pixel", &[], vec!["Ouvrir dans Pixel : utilise Sélection d'objet (W) puis Calque › Masque pour détourer".into()], steps);
                } else {
                    s.push(format!("Repérer « {obj} » avec l'IA de vision locale"));
                    s.push("Supprimer chaque élément et reconstruire le fond".into());
                    steps.push(Step::Remove(obj));
                }
            }
            if has(&t, &["anime", "gif", "fais bouger", "animation", "ken burns"]) {
                steps.push(Step::Animate);
                s.push("Créer une animation (zoom lent) en GIF et l'ouvrir dans Motion".into());
                return done("motion", &[], s, steps);
            }
            let (ops, dev) = image_filters(&t, &mut s);
            if !dev.is_empty() {
                steps.push(Step::Develop(Value::Object(dev)));
            }
            if !ops.is_empty() {
                steps.push(Step::Pixel(ops));
            }
            let only_develop = steps.iter().all(|x| matches!(x, Step::Develop(_)));
            if steps.is_empty() {
                let app = if kind == Kind::Raw || has(&t, &["developpe", "lumen", "catalogue"]) { "lumen" } else { "pixel" };
                return done(app, &[], vec![format!("Ouvrir dans {}", studio_core::handoff::name(app))], steps);
            }
            done(if only_develop || kind == Kind::Raw { "lumen" } else { "pixel" }, &[], s, steps)
        }
        Kind::Video => {
            if has(&t, &["son", "audio", "musique", "voix", "mp3", "wav"]) && has(&t, &["extrais", "recupere", "sors", "garde", "isole", "extract"]) {
                steps.push(Step::ExtractAudio);
                return done("sound", &[], vec!["Extraire la bande son en WAV et l'ouvrir dans Sound".into()], steps);
            }
            done("cut", &[], vec!["Ouvrir la vidéo dans Cut pour la monter".into()], steps)
        }
        Kind::Audio => {
            let mut ops = Vec::new();
            if has(&t, &["nettoie", "bruit", "souffle", "parasite", "propre", "clean"]) {
                ops.extend([json!({"op":"low_cut","hz":80}), json!({"op":"gate","threshold":-48}), json!({"op":"compress","threshold":-20,"ratio":3,"makeup":3})]);
                s.push("Nettoyer : coupe-bas, porte de bruit, compression".into());
            }
            if has(&t, &["podcast", "voix", "radio", "micro"]) {
                ops.extend([json!({"op":"low_cut","hz":90}), json!({"op":"eq","low":-2,"mid":2,"high":3}), json!({"op":"compress","threshold":-22,"ratio":3.5,"makeup":4}), json!({"op":"loudness","db":-16})]);
                s.push("Voix de podcast : égalisation, compression, −16 LUFS".into());
            }
            if has(&t, &["plus fort", "volume", "normalise", "augmente", "louder"]) && !has(&t, &["moins fort", "baisse"]) {
                ops.push(json!({"op":"normalize","db":-1}));
                s.push("Monter le volume au maximum sans saturer".into());
            }
            if has(&t, &["moins fort", "baisse le volume", "quieter"]) {
                ops.push(json!({"op":"gain","db":-6}));
                s.push("Baisser le volume de 6 dB".into());
            }
            if has(&t, &["fondu", "fade"]) {
                ops.extend([json!({"op":"fade_in","seconds":1}), json!({"op":"fade_out","seconds":2})]);
                s.push("Fondus d'entrée et de sortie".into());
            }
            if has(&t, &["echo", "reverb", "cathedrale", "salle", "eglise"]) {
                ops.push(json!({"op":"reverb","size":0.7,"mix":0.3}));
                s.push("Ajouter de la réverbération".into());
            }
            if has(&t, &["accelere", "plus vite", "faster"]) {
                ops.push(json!({"op":"speed","factor":1.25}));
                s.push("Accélérer (×1,25)".into());
            }
            if has(&t, &["ralenti", "plus lent", "slower"]) {
                ops.push(json!({"op":"speed","factor":0.8}));
                s.push("Ralentir (×0,8)".into());
            }
            if has(&t, &["a l envers", "inverse", "reverse"]) {
                ops.push(json!({"op":"reverse"}));
                s.push("Lire à l'envers".into());
            }
            if has(&t, &["mono"]) {
                ops.push(json!({"op":"mono"}));
                s.push("Passer en mono".into());
            }
            if ops.is_empty() {
                return done("sound", &[], vec!["Ouvrir le son dans Sound".into()], steps);
            }
            steps.push(Step::Sound(ops));
            done("sound", &[], s, steps)
        }
        Kind::Pdf => {
            let panel = [
                (&["resume", "explique", "question", "analyse", "points cles", "summar"][..], "ai", "Résumer et répondre avec l'assistant IA"),
                (&["ocr", "scan", "texte", "lis ", "consultable", "searchable"][..], "", "Reconnaître le texte (OCR) et rendre le PDF consultable"),
                (&["signe", "signature", "remplis", "formulaire"][..], "fillsign", "Remplir et signer"),
                (&["fusionne", "combine", "assemble", "merge"][..], "combine", "Combiner avec d'autres fichiers"),
                (&["compresse", "allege", "reduis la taille", "trop lourd", "compress"][..], "compress", "Compresser le fichier"),
                (&["protege", "mot de passe", "chiffre", "password"][..], "protect", "Protéger par mot de passe"),
                (&["filigrane", "numerote", "numeros de page", "bates", "en tete", "pied de page"][..], "stamp", "Ajouter filigrane ou numérotation"),
                (&["caviarde", "masque", "cache les", "anonymise"][..], "redact", "Caviarder des informations"),
                (&["image", "png", "jpg", "word", "exporte", "convertis"][..], "export", "Exporter en images, texte ou Word"),
                (&["pivote", "tourne", "supprime la page", "reorganise", "organise", "extrais la page", "divise", "separe"][..], "organize", "Organiser les pages"),
                (&["compare", "difference"][..], "compare", "Comparer avec une autre version"),
                (&["cherche", "trouve", "recherche"][..], "search", "Rechercher dans le document"),
            ];
            for (words, p, label) in panel {
                if has(&t, words) {
                    let args: Vec<&str> = if p.is_empty() { vec!["--ocr"] } else { vec!["--outil", p] };
                    return done("folio", &args, vec![format!("Ouvrir dans Folio : {label}")], steps);
                }
            }
            done("folio", &[], vec!["Ouvrir le PDF dans Folio".into()], steps)
        }
        Kind::Vector => done("vector", &[], vec!["Ouvrir dans Vector".into()], steps),
        Kind::Layout => done("page", &[], vec!["Ouvrir dans Page".into()], steps),
        Kind::Anim => done("motion", &[], vec!["Ouvrir dans Motion".into()], steps),
        Kind::Montage => done("cut", &[], vec!["Ouvrir dans Cut".into()], steps),
        Kind::Other => Err("Je ne sais pas encore traiter ce type de fichier".into()),
        Kind::Nothing => {
            if has(&t, &["logo", "embleme", "icone"]) {
                let name = quoted_or_after(prompt, &t, &["logo", "embleme", "icone"]);
                let name = if name.is_empty() { "Ethone".to_string() } else { name };
                steps.push(Step::Logo(name.clone()));
                return done("vector", &[], vec![format!("Dessiner un logo « {name} » et l'ouvrir dans Vector")], steps);
            }
            if has(&t, &["titre anime", "intro", "animation", "anime", "gif", "texte anime", "generique"]) {
                let text = quoted_or_after(prompt, &t, &["titre", "texte", "intro", "animation", "anime", "gif"]);
                let text = if text.is_empty() { "Ethone Studio".to_string() } else { text };
                steps.push(Step::Title(text.clone()));
                return done("motion", &[], vec![format!("Animer le titre « {text} » en GIF et l'ouvrir dans Motion")], steps);
            }
            if has(&t, &["bip", "tonalite", "son de test", "bruit blanc", "sine"]) {
                let secs = number_after(&t, &["de", "pendant"]).unwrap_or(3.0).clamp(0.5, 600.0);
                steps.push(Step::Tone(secs));
                return done("sound", &[], vec![format!("Générer une tonalité de {secs:.0} s et l'ouvrir dans Sound")], steps);
            }
            let apps: [(&[&str], &str); 7] = [
                (&["affiche", "flyer", "magazine", "carte de visite", "cv", "brochure", "mise en page", "menu", "invitation"], "page"),
                (&["montage", "video", "clip", "youtube", "tiktok", "film"], "cut"),
                (&["pdf", "document a signer"], "folio"),
                (&["photo", "retouche", "image", "dessin", "peins", "peinture"], "pixel"),
                (&["vectoriel", "illustration", "icones", "svg"], "vector"),
                (&["son", "audio", "podcast", "musique", "voix"], "sound"),
                (&["raw", "catalogue", "developpe"], "lumen"),
            ];
            for (words, app) in apps {
                if has(&t, words) {
                    return done(app, &[], vec![format!("Ouvrir {}", studio_core::handoff::name(app))], steps);
                }
            }
            Err("Je n'ai pas compris : ajoute un fichier ou précise (ex. « retire les oiseaux », « logo Nova », « nettoie ce podcast »)".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(prompt: &str, f: &str) -> Plan {
        plan(prompt, if f.is_empty() { None } else { Some(Path::new(f)) }).unwrap()
    }

    #[test]
    fn understands_common_requests() {
        assert_eq!(object_of(&norm("Retire les oiseaux du ciel stp")).as_deref(), Some("oiseaux du ciel"));
        assert_eq!(object_of(&norm("enlève la personne à gauche de la photo")).as_deref(), Some("personne a gauche"));
        assert_eq!(object_of(&norm("Retire les oiseaux et rends-la plus lumineuse")).as_deref(), Some("oiseaux"));
        assert_eq!(object_of(&norm("enlève le chat et le chien puis recadre")).as_deref(), Some("chat et le chien"));
        let a = p("retire les oiseaux", "x.jpg");
        assert_eq!(a.steps, vec![Step::Remove("oiseaux".into())]);
        assert_eq!(a.app, "pixel");
        assert_eq!(p("efface le texte", "x.png").steps, vec![Step::EraseText]);
        assert_eq!(p("traduis le texte en anglais", "x.png").args, vec!["--texte-photo"]);
        assert_eq!(p("lis le texte de ce scan", "x.png").app, "folio");
        let b = p("mets-la en noir et blanc, plus lumineuse et recadre en carré", "x.jpg");
        assert!(matches!(&b.steps[0], Step::Develop(v) if v["mono"] == true && v["exposure"] == 0.7));
        assert!(matches!(&b.steps[1], Step::Pixel(o) if o[0]["op"] == "square"));
        assert_eq!(p("développe cette photo", "x.cr3").app, "lumen");
        assert_eq!(p("rends-la vintage", "x.jpg").steps, vec![Step::Pixel(vec![json!({"op":"filter","name":"Sépia"})])]);
        assert_eq!(p("extrais le son", "v.mp4").steps, vec![Step::ExtractAudio]);
        assert_eq!(p("monte cette vidéo", "v.mp4").app, "cut");
        assert!(matches!(&p("nettoie ce podcast", "a.wav").steps[0], Step::Sound(o) if o.len() > 3));
        assert_eq!(p("résume ce document", "d.pdf").args, vec!["--outil", "ai"]);
        assert_eq!(p("fais l'ocr", "d.pdf").args, vec!["--ocr"]);
        assert_eq!(p("fais un logo pour Nova Café", "").steps, vec![Step::Logo("Nova Café".into())]);
        assert_eq!(p("logo \"Studio Lune\"", "").steps, vec![Step::Logo("Studio Lune".into())]);
        assert!(matches!(&p("une intro animée « Bienvenue »", "").steps[0], Step::Title(t) if t == "Bienvenue"));
        assert_eq!(p("je veux faire une affiche", "").app, "page");
        assert!(plan("bonjour", None).is_err());
        assert_eq!(kind_of(Some(&PathBuf::from("a.PDF"))), Kind::Pdf);
    }
}
