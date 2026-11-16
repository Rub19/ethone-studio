//! 37 one-click layout tools (Ctrl+K « Outil : … »): pages and format, ready-made layouts on the
//! current page, and adjustments of the selected blocks. Sizes are in points (A4 = 595 × 842).

use crate::model::{Doc, Frame, Kind, Page};

/// What a tool works on: the document, the current page index and the selected blocks on it.
pub struct Ctx<'a> {
    pub doc: &'a mut Doc,
    pub page: usize,
    pub sel: &'a [usize],
}

type Tool = fn(&mut Ctx) -> bool;

/// (id, label, minimum selected blocks, tool)
pub const TOOLS: [(&str, &str, usize, Tool); 37] = [
    ("add-page", "Ajouter une page à la fin", 0, |c| { c.doc.pages.push(Page::default()); true }),
    ("dup-page", "Dupliquer la page", 0, |c| { let p = c.doc.pages[c.page].clone(); c.doc.pages.insert(c.page + 1, p); true }),
    ("drop-empty", "Supprimer les pages vides", 0, |c| { let n = c.doc.pages.len(); c.doc.pages.retain(|p| !p.frames.is_empty()); if c.doc.pages.is_empty() { c.doc.pages.push(Page::default()); } c.doc.pages.len() != n }),
    ("reverse-pages", "Inverser l'ordre des pages", 0, |c| { c.doc.pages.reverse(); c.doc.pages.len() > 1 }),
    ("numbers", "Numéroter les pages en bas", 0, |c| { let n = c.doc.pages.len(); for (k, p) in c.doc.pages.iter_mut().enumerate() { let (w, h) = p.size.map_or((c.doc.width, c.doc.height), |s| (s[0], s[1])); let mut f = Frame::new(w / 2.0 - 40.0, h - 34.0, 80.0, 16.0, text(&format!("{} / {n}", k + 1), 9.0, false, 1)); f.name = "Numéro de page".into(); p.frames.push(f); } true }),
    ("headers", "En-tête avec le titre sur chaque page", 0, |c| { let title = if c.doc.title.is_empty() { "Titre du document".to_string() } else { c.doc.title.clone() }; let m = c.doc.margin; for p in &mut c.doc.pages { let w = p.size.map_or(c.doc.width, |s| s[0]); p.frames.push(Frame::new(m, 16.0, w - 2.0 * m, 16.0, text(&title, 9.0, true, 0))); } true }),
    ("margins-20", "Marges étroites (20 pt)", 0, |c| set(&mut c.doc.margin, 20.0)),
    ("margins-60", "Marges larges (60 pt)", 0, |c| set(&mut c.doc.margin, 60.0)),
    ("a5", "Format A5", 0, |c| size(c.doc, 420.0, 595.0)),
    ("letter", "Format Lettre US", 0, |c| size(c.doc, 612.0, 792.0)),
    ("square", "Format carré (réseaux sociaux)", 0, |c| size(c.doc, 810.0, 810.0)),
    ("landscape", "Passer en paysage", 0, |c| { let (w, h) = (c.doc.width, c.doc.height); size(c.doc, w.max(h), w.min(h)) }),
    ("cream", "Papier crème", 0, |c| { let was = c.doc.paper; c.doc.paper = [250, 244, 230, 255]; was != c.doc.paper }),
    ("grid2", "Grille de 2 colonnes", 0, |c| set_u(&mut c.doc.grid, 2)),
    ("grid3", "Grille de 3 colonnes", 0, |c| set_u(&mut c.doc.grid, 3)),
    ("band-top", "Bandeau de couleur en haut", 0, |c| { let (w, _) = c.doc.page_size(c.page); add(c, Frame::new(0.0, 0.0, w, 90.0, rect([10, 132, 255, 255], 0.0))) }),
    ("band-bottom", "Bandeau de couleur en bas", 0, |c| { let (w, h) = c.doc.page_size(c.page); add(c, Frame::new(0.0, h - 60.0, w, 60.0, rect([20, 34, 80, 255], 0.0))) }),
    ("page-frame", "Cadre autour de la page", 0, |c| { let (w, h) = c.doc.page_size(c.page); let m = c.doc.margin / 2.0; let mut f = Frame::new(m, m, w - 2.0 * m, h - 2.0 * m, rect([0, 0, 0, 0], 0.0)); (f.stroke, f.stroke_w) = (Some([30, 30, 36, 255]), 1.5); add(c, f) }),
    ("divider", "Filet de séparation au milieu", 0, |c| { let (w, h) = c.doc.page_size(c.page); let m = c.doc.margin; add(c, Frame::new(m, h / 2.0, w - 2.0 * m, 1.0, Kind::line([120, 120, 128, 255], 1.0))) }),
    ("title-block", "Titre et sous-titre", 0, |c| { let (w, _) = c.doc.page_size(c.page); let m = c.doc.margin; add(c, Frame::new(m, 120.0, w - 2.0 * m, 60.0, text("Grand titre", 40.0, true, 0))) & add(c, Frame::new(m, 186.0, w - 2.0 * m, 30.0, text("Sous-titre ou chapeau de l'article", 16.0, false, 0))) }),
    ("quote", "Citation centrée", 0, |c| { let (w, h) = c.doc.page_size(c.page); let mut k = text("« Une phrase forte, mise en valeur au centre de la page. »", 26.0, false, 1); if let Kind::Text { italic, .. } = &mut k { *italic = true; } add(c, Frame::new(w * 0.12, h * 0.4, w * 0.76, 120.0, k)) }),
    ("contact", "Bloc de coordonnées en pied de page", 0, |c| { let (w, h) = c.doc.page_size(c.page); let m = c.doc.margin; add(c, Frame::new(m, h - m - 46.0, w - 2.0 * m, 46.0, text("Nom · Adresse · Téléphone · e-mail · site", 10.0, false, 1))) }),
    ("callout", "Encadré coloré", 0, |c| { let (w, h) = c.doc.page_size(c.page); let (x, y, bw, bh) = (w * 0.15, h * 0.62, w * 0.7, 110.0); add(c, Frame::new(x, y, bw, bh, rect([232, 242, 255, 255], 10.0))) & add(c, Frame::new(x + 16.0, y + 16.0, bw - 32.0, bh - 32.0, text("À retenir : écris ici l'information importante.", 14.0, true, 0))) }),
    ("image-slot", "Emplacement d'image pleine largeur", 0, |c| { let (w, h) = c.doc.page_size(c.page); let m = c.doc.margin; add(c, Frame::new(m, h * 0.3, w - 2.0 * m, h * 0.35, rect([226, 228, 234, 255], 4.0))) }),
    ("same-width", "Même largeur que le premier bloc", 2, |c| { let w = get(c, c.sel[0]).w; each(c, |f| set(&mut f.w, w)) }),
    ("same-height", "Même hauteur que le premier bloc", 2, |c| { let h = get(c, c.sel[0]).h; each(c, |f| set(&mut f.h, h)) }),
    ("align-left", "Aligner les bords gauches", 2, |c| { let x = c.sel.iter().map(|i| get(c, *i).x).fold(f32::MAX, f32::min); each(c, |f| set(&mut f.x, x)) }),
    ("align-top", "Aligner les bords hauts", 2, |c| { let y = c.sel.iter().map(|i| get(c, *i).y).fold(f32::MAX, f32::min); each(c, |f| set(&mut f.y, y)) }),
    ("stack", "Empiler avec 12 pt d'écart", 2, |c| { let mut order = c.sel.to_vec(); order.sort_by(|a, b| get(c, *a).y.total_cmp(&get(c, *b).y)); let mut y = get(c, order[0]).y; let mut any = false; for i in order { let f = &mut frames(c)[i]; any |= set(&mut f.y, y); y += f.h + 12.0; } any }),
    ("row", "Mettre en rangée avec 12 pt d'écart", 2, |c| { let mut order = c.sel.to_vec(); order.sort_by(|a, b| get(c, *a).x.total_cmp(&get(c, *b).x)); let mut x = get(c, order[0]).x; let mut any = false; for i in order { let f = &mut frames(c)[i]; any |= set(&mut f.x, x); x += f.w + 12.0; } any }),
    ("grow", "Agrandir de 10 %", 1, |c| each(c, |f| { let (cx, cy) = (f.x + f.w / 2.0, f.y + f.h / 2.0); (f.w, f.h) = (f.w * 1.1, f.h * 1.1); (f.x, f.y) = (cx - f.w / 2.0, cy - f.h / 2.0); true })),
    ("shrink", "Réduire de 10 %", 1, |c| each(c, |f| { let (cx, cy) = (f.x + f.w / 2.0, f.y + f.h / 2.0); (f.w, f.h) = (f.w * 0.9, f.h * 0.9); (f.x, f.y) = (cx - f.w / 2.0, cy - f.h / 2.0); true })),
    ("shadow", "Ombre portée", 1, |c| each(c, |f| { let was = f.shadow; f.shadow = true; !was })),
    ("rounded", "Coins arrondis", 1, |c| each(c, |f| match &mut f.kind { Kind::Rect { radius, .. } | Kind::Image { radius, .. } => set(radius, 14.0), _ => false })),
    ("thin-stroke", "Contour fin", 1, |c| each(c, |f| { let was = (f.stroke, f.stroke_w); (f.stroke, f.stroke_w) = (Some([40, 40, 46, 255]), 0.75); was != (f.stroke, f.stroke_w) })),
    ("half-opacity", "Opacité à 50 %", 1, |c| each(c, |f| set(&mut f.opacity, 0.5))),
    ("uppercase", "Texte en majuscules", 1, |c| each(c, |f| match &mut f.kind { Kind::Text { upper, .. } => { let was = *upper; *upper = true; !was } _ => false })),
];

fn text(s: &str, size: f32, bold: bool, align: u8) -> Kind {
    let mut k = Kind::text(s, size, bold);
    if let Kind::Text { align: a, .. } = &mut k {
        *a = align;
    }
    k
}

fn rect(fill: [u8; 4], radius: f32) -> Kind {
    Kind::Rect { fill, radius }
}

fn set(v: &mut f32, x: f32) -> bool {
    let changed = (*v - x).abs() > 1e-4;
    *v = x;
    changed
}

fn set_u(v: &mut u32, x: u32) -> bool {
    let changed = *v != x;
    *v = x;
    changed
}

fn size(doc: &mut Doc, w: f32, h: f32) -> bool {
    let changed = (doc.width, doc.height) != (w, h);
    (doc.width, doc.height) = (w, h);
    changed
}

fn frames<'a>(c: &'a mut Ctx) -> &'a mut Vec<Frame> {
    &mut c.doc.pages[c.page].frames
}

fn get<'a>(c: &'a Ctx, i: usize) -> &'a Frame {
    &c.doc.pages[c.page].frames[i]
}

fn add(c: &mut Ctx, f: Frame) -> bool {
    frames(c).push(f);
    true
}

fn each(c: &mut Ctx, mut f: impl FnMut(&mut Frame) -> bool) -> bool {
    let mut any = false;
    for &i in c.sel {
        if let Some(fr) = c.doc.pages[c.page].frames.get_mut(i) {
            any |= f(fr);
        }
    }
    any
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Doc {
        let mut d = Doc::a4();
        d.pages.push(Page::default());
        let p = &mut d.pages[0].frames;
        p.push(Frame::new(50.0, 300.0, 200.0, 40.0, Kind::text("Bonjour", 12.0, false)));
        p.push(Frame::new(80.0, 120.0, 120.0, 90.0, Kind::Rect { fill: [200, 0, 0, 255], radius: 0.0 }));
        p.push(Frame::new(300.0, 500.0, 150.0, 60.0, Kind::text("Monde", 14.0, false)));
        d
    }

    #[test]
    fn every_tool_changes_the_document() {
        let mut ids = std::collections::HashSet::new();
        for (id, label, _, f) in TOOLS {
            assert!(ids.insert(id), "{id} en double");
            let before = doc();
            let mut d = before.clone();
            let mut c = Ctx { doc: &mut d, page: 0, sel: &[0, 1, 2] };
            assert!(f(&mut c), "{label} ne fait rien");
            assert!(d != before, "{label} ne change rien");
            assert!(!d.pages.is_empty(), "{label}");
            assert!(serde_json::to_string(&d).is_ok(), "{label}");
        }
    }
}
