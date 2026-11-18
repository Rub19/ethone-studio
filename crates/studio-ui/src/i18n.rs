//! Interface language. Code keeps its French text and wraps it in [`tl`]; other languages come from
//! `FR<TAB>translation` dictionaries, one per language (`i18n/<code>.tsv`), shared plus one set per app.
//! A missing entry falls back to English, then French, so a language can be completed progressively.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{OnceLock, RwLock};

pub const LANGS: [&str; 6] = ["Français", "English", "Español", "Deutsch", "Italiano", "Português"];
pub const CODES: [&str; 6] = ["fr", "en", "es", "de", "it", "pt"];

static LANG: AtomicU8 = AtomicU8::new(0);
type Dict = HashMap<&'static str, &'static str>;
static DICTS: OnceLock<RwLock<Vec<Dict>>> = OnceLock::new();

fn dicts() -> &'static RwLock<Vec<Dict>> {
    DICTS.get_or_init(|| {
        let mut v = vec![HashMap::new(); LANGS.len()];
        for (k, tsv) in [
            (1, include_str!("../i18n/en.tsv")),
            (2, include_str!("../i18n/es.tsv")),
            (3, include_str!("../i18n/de.tsv")),
            (4, include_str!("../i18n/it.tsv")),
            (5, include_str!("../i18n/pt.tsv")),
        ] {
            parse_into(&mut v[k], tsv);
        }
        RwLock::new(v)
    })
}

fn parse_into(m: &mut Dict, tsv: &'static str) {
    for line in tsv.lines() {
        if let Some((fr, tr)) = line.split_once('\t')
            && !fr.is_empty()
            && !tr.trim().is_empty()
            && !fr.starts_with('#')
        {
            m.insert(fr, tr.trim_end());
        }
    }
}

/// Adds a dictionary for language `k` (an index into [`LANGS`]).
pub fn register_lang(k: usize, tsv: &'static str) {
    if let Ok(mut v) = dicts().write()
        && let Some(m) = v.get_mut(k)
    {
        parse_into(m, tsv);
    }
}

/// Adds an app's English dictionary.
pub fn register(tsv: &'static str) {
    register_lang(1, tsv);
}

/// Registers every `i18n/<code>.tsv` of the calling crate (call once at startup).
#[macro_export]
macro_rules! register_i18n {
    () => {{
        $crate::i18n::register_lang(1, include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/i18n/en.tsv")));
        $crate::i18n::register_lang(2, include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/i18n/es.tsv")));
        $crate::i18n::register_lang(3, include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/i18n/de.tsv")));
        $crate::i18n::register_lang(4, include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/i18n/it.tsv")));
        $crate::i18n::register_lang(5, include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/i18n/pt.tsv")));
    }};
}

pub fn set_lang(k: usize) {
    LANG.store(k.min(LANGS.len() - 1) as u8, Ordering::Relaxed);
}

pub fn lang() -> usize {
    LANG.load(Ordering::Relaxed) as usize
}

/// The text in the interface language; falls back to English, then to the French source.
pub fn tl<'a>(fr: &'a str) -> &'a str {
    let k = lang();
    if k == 0 {
        return fr;
    }
    let Ok(v) = dicts().read() else { return fr };
    v[k].get(fr).or_else(|| v[1].get(fr)).copied().unwrap_or(fr)
}

/// A shortcut label ("Ctrl+Maj+Suppr") with its key names in the interface language.
pub fn keys(fr: &str) -> String {
    let k = lang();
    if k == 0 {
        return fr.to_owned();
    }
    //            en           es           de             it           pt
    let names: [(&str, [&str; 5]); 11] = [
        ("Ctrl", ["Ctrl", "Ctrl", "Strg", "Ctrl", "Ctrl"]),
        ("Maj", ["Shift", "Mayús", "Umschalt", "Maiusc", "Shift"]),
        ("Suppr", ["Del", "Supr", "Entf", "Canc", "Del"]),
        ("Entrée", ["Enter", "Intro", "Eingabe", "Invio", "Enter"]),
        ("Début", ["Home", "Inicio", "Pos1", "Home", "Home"]),
        ("Fin", ["End", "Fin", "Ende", "Fine", "End"]),
        ("Échap", ["Esc", "Esc", "Esc", "Esc", "Esc"]),
        ("Espace", ["Space", "Espacio", "Leertaste", "Spazio", "Espaço"]),
        ("Retour", ["Backspace", "Retroceso", "Rücktaste", "Backspace", "Backspace"]),
        ("Flèches", ["Arrows", "Flechas", "Pfeiltasten", "Frecce", "Setas"]),
        ("Inser", ["Ins", "Insert", "Einfg", "Ins", "Ins"]),
    ];
    let mut out = String::with_capacity(fr.len());
    let mut word = String::new();
    let flush = |w: &mut String, out: &mut String| {
        match names.iter().find(|n| n.0 == w.as_str()) {
            Some(n) => out.push_str(n.1[k - 1]),
            None => out.push_str(w),
        }
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
        set_lang(3);
        assert_eq!(keys("Ctrl+Maj+Suppr"), "Strg+Umschalt+Entf");
        set_lang(0);
        assert_eq!(keys("Ctrl+Maj+S"), "Ctrl+Maj+S");
    }

    #[test]
    fn translation_then_english_then_french() {
        register("Calque test\tTest layer\nVide test\t\n# commentaire\tignored\nCalque seul\tLone layer\n");
        register_lang(2, "Calque test\tCapa de prueba\n");
        set_lang(0);
        assert_eq!(tl("Calque test"), "Calque test");
        set_lang(1);
        assert_eq!(tl("Calque test"), "Test layer");
        assert_eq!(tl("Vide test"), "Vide test", "an empty translation falls back to French");
        assert_eq!(tl("Jamais vu"), "Jamais vu");
        set_lang(2);
        assert_eq!(tl("Calque test"), "Capa de prueba");
        assert_eq!(tl("Calque seul"), "Lone layer", "missing in Spanish falls back to English");
        set_lang(0);
    }
}
