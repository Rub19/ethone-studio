//! 38 one-click editing tools (Ctrl+K « Outil : … ») on the whole edit or the selected main-track clip.
//! A tool returns false when there was nothing to do.

use crate::model::{Clip, Marker, Project, Title, TRANSITIONS};

type Tool = fn(&mut Project, Option<usize>) -> bool;

/// (id, label, needs a selected clip, tool)
pub const TOOLS: [(&str, &str, bool, Tool); 38] = [
    ("shuffle", "Mélanger l'ordre des clips", false, |p, _| reorder(p, |c| { let n = c.len(); (0..n).map(|k| (k * 7 + 3) % n).collect() })),
    ("reverse-order", "Inverser l'ordre des clips", false, |p, _| reorder(p, |c| (0..c.len()).rev().collect())),
    ("sort-length", "Trier les clips du plus court au plus long", false, |p, _| reorder(p, |c| { let mut o: Vec<usize> = (0..c.len()).collect(); o.sort_by(|a, b| c[*a].len().total_cmp(&c[*b].len())); o })),
    ("max3", "Limiter chaque clip à 3 s", false, |p, _| every(p, |c| trim_to(c, 3.0))),
    ("max1", "Montage express : 1 s par clip", false, |p, _| every(p, |c| trim_to(c, 1.0))),
    ("trim10", "Retirer 10 % au début et à la fin de chaque clip", false, |p, _| every(p, |c| { let cut = (c.out - c.inp) * 0.1; if cut <= 0.0 || c.hold > 0.0 { return false; } c.inp += cut; c.out -= cut; true })),
    ("even", "Durées égales (moyenne du montage)", false, |p, _| { let n = p.clips.len().max(1) as f64; let avg = p.clips.iter().map(Clip::len).sum::<f64>() / n; every(p, |c| trim_to(c, avg)) }),
    ("rhythm", "Rythme alterné 1 s / 2 s", false, |p, _| { let mut k = 0; every(p, |c| { k += 1; trim_to(c, if k % 2 == 1 { 1.0 } else { 2.0 }) }) }),
    ("kenburns-all", "Ken Burns alterné sur tous les clips", false, |p, _| { let mut k = 0; every(p, |c| { k += 1; (c.zoom, c.zoom_end) = if k % 2 == 1 { (1.0, 1.15) } else { (1.15, 1.0) }; true }) }),
    ("slow-zoom", "Zoom avant lent sur tous les clips", false, |p, _| every(p, |c| { (c.zoom, c.zoom_end) = (1.0, 1.08); true })),
    ("random-trans", "Transitions variées entre tous les clips", false, |p, _| { let mut k = 0; let n = TRANSITIONS.len() - 1; every_but_first(p, |c| { k += 1; (c.trans, c.trans_dur) = (1 + ((k * 5 + 2) % n) as u8, 0.6); true }) }),
    ("alt-trans", "Transitions alternées fondu / glissement", false, |p, _| { let mut k = 0; let slide = TRANSITIONS.iter().position(|t| t.to_lowercase().contains("gliss")).unwrap_or(2) as u8; every_but_first(p, |c| { k += 1; (c.trans, c.trans_dur) = (if k % 2 == 1 { 1 } else { slide }, 0.5); true }) }),
    ("speed-ramp", "Accélération progressive (×1 → ×2)", false, |p, _| { let n = p.clips.len().max(2) as f64; let mut k = 0.0; every(p, |c| { c.speed = 1.0 + k / (n - 1.0); k += 1.0; true }) }),
    ("mute-all", "Couper le son de tous les clips", false, |p, _| every(p, |c| { let was = c.mute; c.mute = true; !was })),
    ("unmute-all", "Remettre le son de tous les clips", false, |p, _| every(p, |c| { let was = c.mute; c.mute = false; was })),
    ("quieter", "Baisser le son des clips de moitié", false, |p, _| every(p, |c| { c.volume *= 0.5; true })),
    ("music-low", "Musique en fond (30 %)", false, |p, _| { let n = p.music.len(); for m in &mut p.music { m.volume = 0.3; } n > 0 }),
    ("music-fades", "Fondus de 2 s sur la musique", false, |p, _| { let n = p.music.len(); for m in &mut p.music { (m.fade_in, m.fade_out) = (2.0, 2.0); } n > 0 }),
    ("match-grade", "Étalonner tous les clips comme le clip choisi", true, |p, s| { let Some(src) = s.and_then(|i| p.clips.get(i)).cloned() else { return false }; every(p, |c| { (c.brightness, c.contrast, c.saturation, c.temperature, c.tint, c.look, c.exposure, c.highlights, c.shadows, c.whites, c.blacks, c.vibrance, c.hue, c.wheels, c.curves, c.vignette) = (src.brightness, src.contrast, src.saturation, src.temperature, src.tint, src.look, src.exposure, src.highlights, src.shadows, src.whites, src.blacks, src.vibrance, src.hue, src.wheels, src.curves, src.vignette); true }) }),
    ("reset-grade", "Effacer l'étalonnage de tous les clips", false, |p, _| { let d = Clip::default(); every(p, |c| { (c.brightness, c.contrast, c.saturation, c.temperature, c.tint, c.look, c.exposure, c.vignette) = (d.brightness, d.contrast, d.saturation, d.temperature, d.tint, d.look, d.exposure, d.vignette); true }) }),
    ("bw-all", "Tout le montage en noir et blanc", false, |p, _| every(p, |c| { c.saturation = -100.0; true })),
    ("warm-all", "Tout le montage plus chaud", false, |p, _| every(p, |c| { c.temperature = (c.temperature + 20.0).min(100.0); true })),
    ("cool-all", "Tout le montage plus froid", false, |p, _| every(p, |c| { c.temperature = (c.temperature - 20.0).max(-100.0); true })),
    ("vignette-all", "Vignettage sur tout le montage", false, |p, _| every(p, |c| { c.vignette = 45.0; true })),
    ("mirror-all", "Retourner tous les clips (miroir)", false, |p, _| every(p, |c| { c.flip = !c.flip; true })),
    ("enable-all", "Réactiver tous les clips", false, |p, _| every(p, |c| { let was = c.disabled; c.disabled = false; was })),
    ("drop-disabled", "Supprimer les clips désactivés", false, |p, _| { let n = p.clips.len(); p.clips.retain(|c| !c.disabled); p.clips.len() != n }),
    ("drop-short", "Supprimer les clips de moins d'une demi-seconde", false, |p, _| { let n = p.clips.len(); p.clips.retain(|c| c.len() >= 0.5); p.clips.len() != n }),
    ("repeat3", "Répéter le clip choisi 3 fois", true, |p, s| { let Some(i) = s.filter(|i| *i < p.clips.len()) else { return false }; let c = p.clips[i].clone(); for _ in 0..2 { p.clips.insert(i + 1, Clip { trans: 0, ..c.clone() }); } true }),
    ("boomerang", "Boomerang (aller puis retour)", true, |p, s| { let Some(i) = s.filter(|i| *i < p.clips.len()) else { return false }; let c = p.clips[i].clone(); p.clips.insert(i + 1, Clip { reverse: !c.reverse, trans: 0, ..c }); true }),
    ("freeze-end", "Arrêt sur la dernière image (2 s)", true, |p, s| { let Some(i) = s.filter(|i| *i < p.clips.len()) else { return false }; let c = p.clips[i].clone(); let last = (c.out - 0.04).max(c.inp); p.clips.insert(i + 1, Clip { inp: last, out: c.out, hold: 2.0, trans: 0, ..c }); true }),
    ("opening-title", "Titre d'ouverture", false, |p, _| { p.titles.push(Title { text: "Titre".into(), start: 0.0, dur: 3.0, size: 72.0, anim: 1, ..Title::default() }); true }),
    ("end-card", "Carton de fin", false, |p, _| { let end = p.duration(); p.titles.push(Title { text: "Fin".into(), start: (end - 3.0).max(0.0), dur: 3.0, size: 64.0, band: true, ..Title::default() }); true }),
    ("clip-names", "Nom de chaque clip en sous-titre", false, |p, _| { let mut t = 0.0; let mut added = Vec::new(); for c in &p.clips { let name = if c.name.is_empty() { p.sources.get(c.src).map(|s| std::path::Path::new(&s.path).file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default()).unwrap_or_default() } else { c.name.clone() }; if !name.is_empty() { added.push(Title { text: name, start: t, dur: c.len().min(2.5), size: 34.0, place: 2, band: true, ..Title::default() }); } t += c.len(); } let any = !added.is_empty(); p.titles.extend(added); any }),
    ("clip-markers", "Marqueur au début de chaque clip", false, |p, _| { let mut t = 0.0; let mut n = 0; for c in &p.clips { p.markers.push(Marker { t, name: format!("Clip {}", n + 1), ..Marker::default() }); t += c.len(); n += 1; } n > 0 }),
    ("clear-markers", "Effacer tous les marqueurs", false, |p, _| { let any = !p.markers.is_empty(); p.markers.clear(); any }),
    ("clear-titles", "Effacer tous les titres", false, |p, _| { let any = !p.titles.is_empty(); p.titles.clear(); any }),
    ("white-bg", "Fond du projet en blanc", false, |p, _| { let was = p.background; p.background = [255, 255, 255, 255]; was != p.background }),
];

fn every(p: &mut Project, mut f: impl FnMut(&mut Clip) -> bool) -> bool {
    let mut any = false;
    for c in &mut p.clips {
        any |= f(c);
    }
    any
}

fn every_but_first(p: &mut Project, mut f: impl FnMut(&mut Clip) -> bool) -> bool {
    let mut any = false;
    for c in p.clips.iter_mut().skip(1) {
        any |= f(c);
    }
    any
}

fn reorder(p: &mut Project, order: impl Fn(&[Clip]) -> Vec<usize>) -> bool {
    if p.clips.len() < 2 {
        return false;
    }
    let o = order(&p.clips);
    let next: Vec<Clip> = o.iter().map(|i| p.clips[*i].clone()).collect();
    let changed = next != p.clips;
    p.clips = next;
    changed
}

fn trim_to(c: &mut Clip, secs: f64) -> bool {
    if c.hold > 0.0 {
        let was = c.hold;
        c.hold = secs.max(0.1);
        return was != c.hold;
    }
    let max_src = secs * c.speed.max(0.05);
    if c.out - c.inp > max_src {
        let mid = (c.inp + c.out) / 2.0;
        (c.inp, c.out) = (mid - max_src / 2.0, mid + max_src / 2.0);
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        let mut p = Project::new();
        for k in 0..5 {
            p.clips.push(Clip { src: 0, inp: 0.0, out: 2.0 + k as f64 * 1.7, name: format!("plan {k}"), disabled: k == 3, mute: k == 1, volume: 1.0, ..Clip::default() });
        }
        p.clips[4].out = 0.3;
        (p.clips[1].temperature, p.clips[1].contrast, p.clips[1].look) = (30.0, 12.0, 2);
        p.music.push(crate::model::Music { out: 10.0, volume: 1.0, ..Default::default() });
        p.markers.push(Marker { t: 1.0, ..Marker::default() });
        p.titles.push(Title::default());
        p
    }

    #[test]
    fn every_tool_changes_the_project() {
        let mut ids = std::collections::HashSet::new();
        for (id, label, _, f) in TOOLS {
            assert!(ids.insert(id), "{id} en double");
            let before = project();
            let mut p = before.clone();
            assert!(f(&mut p, Some(1)), "{label} ne fait rien");
            assert!(p != before, "{label} ne change rien");
            assert!(p.clips.iter().all(|c| c.len().is_finite() && c.len() >= 0.0), "{label}");
            assert!(serde_json::to_string(&p).is_ok(), "{label}");
        }
        let mut p = project();
        assert!(!TOOLS.iter().find(|t| t.0 == "repeat3").unwrap().3(&mut p, None));
    }
}
