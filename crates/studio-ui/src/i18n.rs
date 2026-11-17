//! Interface language. Code keeps its French text and wraps it in [`tl`]; English comes from
//! `FR<TAB>EN` dictionaries (one shared, one per app). A missing entry falls back to French, so an
//! app can be translated progressively.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{OnceLock, RwLock};

pub const LANGS: [&str; 2] = ["Français", "English"];

static LANG: AtomicU8 = AtomicU8::new(0);
static DICT: OnceLock<RwLock<HashMap<&'static str, &'static str>>> = OnceLock::new();

fn dict() -> &'static RwLock<HashMap<&'static str, &'static str>> {
    DICT.get_or_init(|| {
        let mut m = HashMap::new();
        parse_into(&mut m, include_str!("../i18n/en.tsv"));
        RwLock::new(m)
    })
}

fn parse_into(m: &mut HashMap<&'static str, &'static str>, tsv: &'static str) {
    for line in tsv.lines() {
        if let Some((fr, en)) = line.split_once('\t')
            && !fr.is_empty()
            && !en.trim().is_empty()
            && !fr.starts_with('#')
        {
            m.insert(fr, en.trim_end());
        }
    }
}

/// Adds an app's dictionary (call once at startup with `include_str!`).
pub fn register(tsv: &'static str) {
    if let Ok(mut m) = dict().write() {
        parse_into(&mut m, tsv);
    }
}

pub fn set_lang(k: usize) {
    LANG.store(k.min(LANGS.len() - 1) as u8, Ordering::Relaxed);
}

pub fn lang() -> usize {
    LANG.load(Ordering::Relaxed) as usize
}

/// The text in the interface language: English when a translation exists, French otherwise.
pub fn tl<'a>(fr: &'a str) -> &'a str {
    if lang() == 0 {
        return fr;
    }
    dict().read().ok().and_then(|m| m.get(fr).copied()).unwrap_or(fr)
}

/// A shortcut label ("Ctrl+Maj+Suppr") with its key names in the interface language.
pub fn keys(fr: &str) -> String {
    if lang() == 0 {
        return fr.to_owned();
    }
    let mut out = String::with_capacity(fr.len());
    let mut word = String::new();
    let flush = |w: &mut String, out: &mut String| {
        out.push_str(match w.as_str() {
            "Maj" => "Shift",
            "Suppr" => "Del",
            "Entrée" => "Enter",
            "Début" => "Home",
            "Fin" => "End",
            "Échap" => "Esc",
            "Espace" => "Space",
            "Retour" => "Backspace",
            "Flèches" => "Arrows",
            "Inser" => "Ins",
            w => w,
        });
        w.clear();
    };
    for c in fr.chars() {
        if c.is_alphanumeric() {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names_follow_the_language() {
        set_lang(1);
        assert_eq!(keys("Ctrl+Maj+Suppr"), "Ctrl+Shift+Del");
        assert_eq!(keys("Entrée"), "Enter");
        assert_eq!(keys("Final"), "Final");
        set_lang(0);
        assert_eq!(keys("Ctrl+Maj+S"), "Ctrl+Maj+S");
    }

    #[test]
    fn english_when_known_french_otherwise() {
        register("Calque test\tTest layer\nVide test\t\n# commentaire\tignored\n");
        set_lang(0);
        assert_eq!(tl("Calque test"), "Calque test");
        set_lang(1);
        assert_eq!(tl("Calque test"), "Test layer");
        assert_eq!(tl("Vide test"), "Vide test", "an empty translation falls back to French");
        assert_eq!(tl("Jamais vu"), "Jamais vu");
        set_lang(0);
    }
}
