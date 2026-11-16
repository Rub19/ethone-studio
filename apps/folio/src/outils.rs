//! 35 one-click PDF tools (Ctrl+K « Outil PDF : … ») on the open document and its current page.
//! A tool returns Ok(false) when there was nothing to do.

use crate::engine::{self, Mark, MarkKind, R, Stamp};
use lopdf::Document;

type Tool = fn(&mut Document, usize, &str) -> R<bool>;

pub const TOOLS: [(&str, &str, Tool); 35] = [
    ("dup-page", "Dupliquer la page courante", |d, p, _| { let one = engine::load(&engine::extract(d, &[p])?, "")?; engine::insert_file(d, p + 1, one)?; Ok(true) }),
    ("blank-after", "Page blanche après la page courante", |d, p, _| { let (w, h) = size(d, p); engine::insert_blank(d, p + 1, w, h)?; Ok(true) }),
    ("cover", "Page blanche au début (couverture)", |d, _, _| { let (w, h) = size(d, 0); engine::insert_blank(d, 0, w, h)?; Ok(true) }),
    ("blank-end", "Page blanche à la fin", |d, _, _| { let n = count(d); let (w, h) = size(d, n - 1); engine::insert_blank(d, n, w, h)?; Ok(true) }),
    ("reverse", "Inverser l'ordre des pages", |d, _, _| { let n = count(d); if n < 2 { return Ok(false); } engine::arrange(d, &(0..n).rev().collect::<Vec<_>>())?; Ok(true) }),
    ("odd-first", "Pages impaires puis paires", |d, _, _| { let n = count(d); if n < 3 { return Ok(false); } let o: Vec<usize> = (0..n).step_by(2).chain((1..n).step_by(2)).collect(); engine::arrange(d, &o)?; Ok(true) }),
    ("interleave", "Remettre en ordre un scan recto puis verso", |d, _, _| { let n = count(d); if n < 2 { return Ok(false); } let half = n.div_ceil(2); let o: Vec<usize> = (0..n).map(|k| if k % 2 == 0 { k / 2 } else { half + k / 2 }).collect(); engine::arrange(d, &o)?; Ok(true) }),
    ("swap-pairs", "Échanger les pages deux à deux", |d, _, _| { let n = count(d); if n < 2 { return Ok(false); } let o: Vec<usize> = (0..n).map(|k| if k % 2 == 0 { (k + 1).min(n - 1) } else { k - 1 }).collect(); engine::arrange(d, &o)?; Ok(true) }),
    ("drop-even", "Supprimer les pages paires", |d, _, _| { let idx: Vec<usize> = (1..count(d)).step_by(2).collect(); if idx.is_empty() { return Ok(false); } engine::delete(d, &idx)?; Ok(true) }),
    ("drop-odd", "Supprimer les pages impaires", |d, _, _| { let n = count(d); let idx: Vec<usize> = (0..n).step_by(2).collect(); if n < 2 { return Ok(false); } engine::delete(d, &idx)?; Ok(true) }),
    ("drop-first", "Supprimer la première page", |d, _, _| { if count(d) < 2 { return Ok(false); } engine::delete(d, &[0])?; Ok(true) }),
    ("drop-last", "Supprimer la dernière page", |d, _, _| { let n = count(d); if n < 2 { return Ok(false); } engine::delete(d, &[n - 1])?; Ok(true) }),
    ("to-front", "Mettre la page courante en premier", |d, p, _| { let n = count(d); if p == 0 || p >= n { return Ok(false); } let o: Vec<usize> = std::iter::once(p).chain((0..n).filter(|k| *k != p)).collect(); engine::arrange(d, &o)?; Ok(true) }),
    ("to-back", "Mettre la page courante en dernier", |d, p, _| { let n = count(d); if p + 1 >= n { return Ok(false); } let o: Vec<usize> = (0..n).filter(|k| *k != p).chain(std::iter::once(p)).collect(); engine::arrange(d, &o)?; Ok(true) }),
    ("notes-between", "Page blanche de notes entre chaque page", |d, _, _| { let n = count(d); for k in (1..=n).rev() { let (w, h) = size(d, k - 1); engine::insert_blank(d, k, w, h)?; } Ok(n > 0) }),
    ("flip-even", "Pivoter les pages paires de 180° (scan recto verso)", |d, _, _| { let idx: Vec<usize> = (1..count(d)).step_by(2).collect(); if idx.is_empty() { return Ok(false); } engine::rotate(d, &idx, 180)?; Ok(true) }),
    ("flip-all", "Pivoter toutes les pages de 180°", |d, _, _| { let idx: Vec<usize> = (0..count(d)).collect(); engine::rotate(d, &idx, 180)?; Ok(true) }),
    ("portrait", "Mettre les pages paysage en portrait", |d, _, _| { let idx: Vec<usize> = (0..count(d)).filter(|k| { let (w, h) = size(d, *k); w > h }).collect(); if idx.is_empty() { return Ok(false); } engine::rotate(d, &idx, 90)?; Ok(true) }),
    ("landscape", "Mettre les pages portrait en paysage", |d, _, _| { let idx: Vec<usize> = (0..count(d)).filter(|k| { let (w, h) = size(d, *k); h > w }).collect(); if idx.is_empty() { return Ok(false); } engine::rotate(d, &idx, 90)?; Ok(true) }),
    ("trim5", "Rogner les marges de 5 %", |d, _, _| trim(d, 0.05)),
    ("trim10", "Rogner les marges de 10 %", |d, _, _| trim(d, 0.10)),
    ("crop-square", "Recadrer la page courante en carré", |d, p, _| { let (w, h) = size(d, p); let (mx, my) = if w > h { ((w - h) / 2.0, 0.0) } else { (0.0, (h - w) / 2.0) }; if mx + my < 0.5 { return Ok(false); } engine::crop(d, &[p], [mx, my, mx, my])?; Ok(true) }),
    ("2up", "2 pages par feuille", |d, _, _| { if count(d) < 2 { return Ok(false); } engine::n_up(d, 2)?; Ok(true) }),
    ("4up", "4 pages par feuille", |d, _, _| { if count(d) < 2 { return Ok(false); } engine::n_up(d, 4)?; Ok(true) }),
    ("no-comments", "Retirer tous les commentaires", |d, _, _| { let idx: Vec<usize> = (0..count(d)).collect(); Ok(engine::remove_comments(d, &idx)? > 0) }),
    ("clear-meta", "Effacer les informations du document", |d, _, _| { let had = engine::info(d).iter().any(|v| !v.is_empty()); engine::set_info(d, &[]); Ok(had) }),
    ("title-from-name", "Titre du document = nom du fichier", |d, _, file| { let mut v = engine::info(d); let t = std::path::Path::new(file).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(); if v.is_empty() || t.is_empty() || v[0] == t { return Ok(false); } v[0] = t; engine::set_info(d, &v); Ok(true) }),
    ("bookmark-pages", "Un signet par page", |d, _, _| { let items: Vec<(usize, String, usize)> = (0..count(d)).map(|k| (1, format!("Page {}", k + 1), k)).collect(); engine::set_outline(d, &items)?; Ok(true) }),
    ("bookmark-lines", "Signets depuis la première ligne de chaque page", |d, _, _| { let items: Vec<(usize, String, usize)> = (0..count(d)).filter_map(|k| { let t = engine::page_text(d, k); let line = t.lines().map(str::trim).find(|l| !l.is_empty())?.chars().take(60).collect::<String>(); Some((1, line, k)) }).collect(); if items.is_empty() { return Ok(false); } engine::set_outline(d, &items)?; Ok(true) }),
    ("no-bookmarks", "Effacer les signets", |d, _, _| { let had = !engine::outline(d).is_empty(); engine::set_outline(d, &[])?; Ok(had) }),
    ("date-stamp", "Date du jour en bas à droite", |d, _, file| { let idx: Vec<usize> = (0..count(d)).collect(); engine::stamp_text(d, &idx, &Stamp { text: "{date}".into(), size: 9.0, color: [90, 90, 96], opacity: 1.0, angle: 0.0, bold: false, place: 6, under: false }, file, None)?; Ok(true) }),
    ("title-page", "Page de garde avec le titre", |d, _, file| { let (w, h) = size(d, 0); engine::insert_blank(d, 0, w, h)?; let t = engine::info(d).first().filter(|t| !t.is_empty()).cloned().unwrap_or_else(|| std::path::Path::new(file).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Document".into())); engine::stamp_text(d, &[0], &Stamp { text: t, size: 28.0, color: [20, 20, 24], opacity: 1.0, angle: 0.0, bold: true, place: 0, under: false }, file, None)?; Ok(true) }),
    ("end-page", "Page de fin « Fin du document »", |d, _, file| { let n = count(d); let (w, h) = size(d, n - 1); engine::insert_blank(d, n, w, h)?; engine::stamp_text(d, &[n], &Stamp { text: "Fin du document".into(), size: 20.0, color: [60, 60, 66], opacity: 1.0, angle: 0.0, bold: false, place: 0, under: false }, file, None)?; Ok(true) }),
    ("signature-line", "Ligne de signature sur la dernière page", |d, _, _| { let n = count(d); let (w, h) = size(d, n - 1); let (x0, x1, y) = (w * 0.55, w * 0.9, h * 0.85); engine::add_mark(d, n - 1, &Mark { kind: MarkKind::Line, pts: vec![[x0, y], [x1, y]], strokes: Vec::new(), color: [20, 20, 24], width: 1.0, text: String::new(), size: 10.0 })?; engine::add_mark(d, n - 1, &Mark { kind: MarkKind::TextBox, pts: vec![[x0, y + 4.0], [x1, y + 20.0]], strokes: Vec::new(), color: [60, 60, 66], width: 0.0, text: "Signature".into(), size: 9.0 })?; Ok(true) }),
    ("page-frame", "Cadre autour de chaque page", |d, _, _| { for k in 0..count(d) { let (w, h) = size(d, k); let m = w.min(h) * 0.04; engine::add_mark(d, k, &Mark { kind: MarkKind::Rect, pts: vec![[m, m], [w - m, h - m]], strokes: Vec::new(), color: [40, 40, 46], width: 1.2, text: String::new(), size: 10.0 })?; } Ok(true) }),
];

fn count(d: &Document) -> usize {
    engine::pages(d).len()
}

/// Displayed size of a page (rotation applied), in points.
fn size(d: &Document, k: usize) -> (f32, f32) {
    let ids = engine::pages(d);
    let Some(id) = ids.get(k.min(ids.len().saturating_sub(1))) else { return (595.0, 842.0) };
    let (b, rot) = engine::geometry(d, *id);
    engine::view_size(b, rot)
}

fn trim(d: &mut Document, frac: f32) -> R<bool> {
    for k in 0..count(d) {
        let (w, h) = size(d, k);
        engine::crop(d, &[k], [w * frac, h * frac, w * frac, h * frac])?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Document {
        let pages: Vec<image::RgbaImage> = (0..4).map(|k| { let (w, h) = if k == 2 { (560, 400) } else { (400, 560) }; image::RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, 255])) }).collect();
        let mut d = engine::images_to_pdf(&pages, None).unwrap();
        let bytes = engine::save(&mut d).unwrap();
        engine::load(&bytes, "").unwrap()
    }

    #[test]
    fn every_tool_runs_and_saves() {
        let mut ids = std::collections::HashSet::new();
        for (id, label, f) in TOOLS {
            assert!(ids.insert(id), "{id} en double");
            let mut d = doc();
            let before = engine::save(&mut d.clone()).unwrap();
            let changed = f(&mut d, 1, "rapport.pdf").unwrap_or_else(|e| panic!("{label} : {e}"));
            let after = engine::save(&mut d).unwrap_or_else(|e| panic!("{label} : {e}"));
            assert!(engine::load(&after, "").is_ok(), "{label} : PDF illisible");
            if !matches!(id, "no-comments" | "clear-meta" | "bookmark-lines" | "no-bookmarks" | "title-from-name") {
                assert!(changed && after != before, "{label} ne change rien");
            }
        }
        let mut d = doc();
        TOOLS.iter().find(|t| t.0 == "reverse").unwrap().2(&mut d, 0, "x.pdf").unwrap();
        let first = engine::geometry(&d, engine::pages(&d)[1]).0;
        assert!((first[3] - first[1]) < (engine::geometry(&d, engine::pages(&d)[0]).0[3] - engine::geometry(&d, engine::pages(&d)[0]).0[1]), "la page courte devrait être en 2e position");
        let mut n = doc();
        TOOLS.iter().find(|t| t.0 == "notes-between").unwrap().2(&mut n, 0, "x.pdf").unwrap();
        assert_eq!(count(&n), 8);
    }
}
