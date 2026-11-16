//! 38 one-click animation tools (Ctrl+K « Outil : … ») on the selected layer or the whole composition.
//! Property indices: 0 X, 1 Y, 2 échelle, 3 rotation, 4 opacité. A tool returns false when nothing changed.

use crate::model::{Comp, Ease, Layer, Marker};

type Tool = fn(&mut Comp, Option<usize>) -> bool;

/// (id, label, needs a selected layer, tool)
pub const TOOLS: [(&str, &str, bool, Tool); 38] = [
    ("spin-reverse", "Rotation dans l'autre sens", true, |c, s| layer(c, s, |l| { let p = &mut l.props[3]; p.base = -p.base; for k in &mut p.keys { k.v = -k.v; } true })),
    ("faster", "Animation deux fois plus rapide", true, |c, s| layer(c, s, |l| { let a = l.start; keys(l, |t| a + (t - a) / 2.0); true })),
    ("slower", "Animation deux fois plus lente", true, |c, s| layer(c, s, |l| { let a = l.start; keys(l, |t| a + (t - a) * 2.0); true })),
    ("delay-keys", "Retarder l'animation de 0,5 s", true, |c, s| layer(c, s, |l| { keys(l, |t| t + 0.5); true })),
    ("clear-keys", "Supprimer toutes les images clés du calque", true, |c, s| layer(c, s, |l| { let any = l.props.iter().any(|p| !p.keys.is_empty()); for p in &mut l.props { if let Some(k) = p.keys.first() { p.base = k.v; } p.keys.clear(); } any })),
    ("ease-smooth", "Adoucir toutes les images clés", true, |c, s| layer(c, s, |l| ease(l, Ease::Smooth))),
    ("ease-hold", "Images clés par paliers", true, |c, s| layer(c, s, |l| ease(l, Ease::Hold))),
    ("ease-bounce", "Rebond sur toutes les images clés", true, |c, s| layer(c, s, |l| ease(l, Ease::Bounce))),
    ("loop", "Boucler l'animation", true, |c, s| layer(c, s, |l| { let was = l.fx.loop_keys; l.fx.loop_keys = true; l.fx.pingpong = false; !was })),
    ("pingpong", "Animation aller-retour", true, |c, s| layer(c, s, |l| { let was = l.fx.pingpong; l.fx.pingpong = true; l.fx.loop_keys = true; !was })),
    ("center", "Centrer le calque dans la scène", true, |c, s| { let (w, h) = (c.width as f32, c.height as f32); layer(c, s, |l| place(l, w / 2.0, h / 2.0)) }),
    ("top-left", "Placer en haut à gauche", true, |c, s| { let (w, h) = (c.width as f32, c.height as f32); layer(c, s, |l| place(l, w * 0.18, h * 0.18)) }),
    ("bottom-right", "Placer en bas à droite", true, |c, s| { let (w, h) = (c.width as f32, c.height as f32); layer(c, s, |l| place(l, w * 0.82, h * 0.82)) }),
    ("third-left", "Placer sur le tiers gauche", true, |c, s| { let (w, h) = (c.width as f32, c.height as f32); layer(c, s, |l| place(l, w / 3.0, h / 2.0)) }),
    ("third-right", "Placer sur le tiers droit", true, |c, s| { let (w, h) = (c.width as f32, c.height as f32); layer(c, s, |l| place(l, w * 2.0 / 3.0, h / 2.0)) }),
    ("full-length", "Calque sur toute la durée", true, |c, s| { let d = c.duration; layer(c, s, |l| { let was = (l.start, l.end); (l.start, l.end) = (0.0, d); was != (l.start, l.end) }) }),
    ("shift-later", "Décaler le calque d'une seconde plus tard", true, |c, s| layer(c, s, |l| { l.start += 1.0; l.end += 1.0; keys(l, |t| t + 1.0); true })),
    ("echo", "Écho de mouvement", true, |c, s| layer(c, s, |l| { (l.fx.echo, l.fx.echo_delay) = (4, 0.08); true })),
    ("repeat5", "Répéter 5 fois en rangée", true, |c, s| layer(c, s, |l| { (l.fx.repeat, l.fx.rep_dx) = (5, 140.0); true })),
    ("soft-shadow", "Ombre portée douce", true, |c, s| layer(c, s, |l| { l.fx.shadow = true; (l.fx.shadow_dx, l.fx.shadow_dy, l.fx.shadow_blur, l.fx.shadow_alpha) = (8.0, 12.0, 18.0, 0.45); true })),
    ("glow", "Lueur", true, |c, s| layer(c, s, |l| { (l.fx.glow, l.fx.glow_color) = (24.0, [120, 200, 255, 255]); true })),
    ("typewriter", "Effet machine à écrire", true, |c, s| layer(c, s, |l| { let was = l.fx.typewriter; l.fx.typewriter = 1.5; was != 1.5 })),
    ("duplicate-offset", "Dupliquer le calque, décalé de 0,2 s", true, |c, s| { let Some(i) = s.filter(|i| *i < c.layers.len()) else { return false }; let mut l = c.layers[i].clone(); l.name = format!("{} (copie)", l.name); l.start += 0.2; l.end += 0.2; keys(&mut l, |t| t + 0.2); c.layers.insert(i + 1, l); true }),
    ("mirror", "Retourner le calque (miroir)", true, |c, s| layer(c, s, |l| { l.fx.flip_x = !l.fx.flip_x; true })),
    ("grey", "Calque en noir et blanc", true, |c, s| layer(c, s, |l| { let was = l.fx.saturation; l.fx.saturation = 0.0; was != 0.0 })),
    ("vivid", "Couleurs vives sur le calque", true, |c, s| layer(c, s, |l| { let was = l.fx.saturation; l.fx.saturation = (l.fx.saturation * 1.4).clamp(140.0, 300.0); was != l.fx.saturation })),
    ("reverse-stack", "Inverser l'ordre des calques", false, |c, _| { c.layers.reverse(); c.layers.len() > 1 }),
    ("center-all", "Centrer tous les calques", false, |c, _| { let (w, h) = (c.width as f32, c.height as f32); let mut any = false; for l in &mut c.layers { any |= place(l, w / 2.0, h / 2.0); } any }),
    ("spread-x", "Répartir les calques sur la largeur", false, |c, _| { let n = c.layers.len(); let w = c.width as f32; let y = c.height as f32 / 2.0; let mut any = false; for (k, l) in c.layers.iter_mut().enumerate() { any |= place(l, w * (k as f32 + 1.0) / (n as f32 + 1.0), y); } any && n > 1 }),
    ("clear-markers", "Effacer tous les marqueurs", false, |c, _| { let any = !c.markers.is_empty(); c.markers.clear(); any }),
    ("extend2", "Allonger la composition de 2 s", false, |c, _| { c.duration += 2.0; true }),
    ("vertical", "Format vertical 9:16", false, |c, _| resize(c, 1080, 1920)),
    ("square", "Format carré 1:1", false, |c, _| resize(c, 1080, 1080)),
    ("gradient-bg", "Fond en dégradé", false, |c, _| { let b = c.background; let was = c.background2; c.background2 = Some([b[0].saturating_add(60), b[1].saturating_add(30), b[2].saturating_add(90), 255]); was != c.background2 }),
    ("film-grain", "Grain de film sur la composition", false, |c, _| { let was = c.grain; c.grain = 25.0; was != c.grain }),
    ("comp-vignette", "Vignettage de la composition", false, |c, _| { let was = c.vignette; c.vignette = 40.0; was != c.vignette }),
    ("letterbox", "Bandes cinéma", false, |c, _| { let was = c.letterbox; c.letterbox = 0.12; was != c.letterbox }),
    ("markers", "Marqueur à chaque seconde", false, |c, _| { let n = c.duration.floor() as usize; for k in 1..=n { c.markers.push(Marker { t: k as f32, name: format!("{k} s") }); } n > 0 }),
];

fn layer(c: &mut Comp, s: Option<usize>, f: impl FnOnce(&mut Layer) -> bool) -> bool {
    s.and_then(|i| c.layers.get_mut(i)).is_some_and(f)
}

fn keys(l: &mut Layer, f: impl Fn(f32) -> f32) {
    for p in &mut l.props {
        for k in &mut p.keys {
            k.t = f(k.t);
        }
        p.keys.sort_by(|a, b| a.t.total_cmp(&b.t));
    }
}

fn ease(l: &mut Layer, e: Ease) -> bool {
    let mut any = false;
    for p in &mut l.props {
        for k in &mut p.keys {
            any |= k.ease != e;
            k.ease = e;
        }
    }
    any
}

/// Moves a layer (and its position keys) so that its position at the start is (x, y).
fn place(l: &mut Layer, x: f32, y: f32) -> bool {
    let mut any = false;
    for (i, target) in [(0, x), (1, y)] {
        let p = &mut l.props[i];
        let d = target - p.at(l.start);
        if d.abs() > 1e-3 {
            any = true;
            p.base += d;
            for k in &mut p.keys {
                k.v += d;
            }
        }
    }
    any
}

/// New frame size; layer positions are scaled so the layout keeps its proportions.
fn resize(c: &mut Comp, w: u32, h: u32) -> bool {
    if (c.width, c.height) == (w, h) {
        return false;
    }
    let (kx, ky) = (w as f32 / c.width.max(1) as f32, h as f32 / c.height.max(1) as f32);
    for l in &mut c.layers {
        for (i, k) in [(0, kx), (1, ky)] {
            l.props[i].base *= k;
            for key in &mut l.props[i].keys {
                key.v *= k;
            }
        }
    }
    (c.width, c.height) = (w, h);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Content, Ease2, Key};

    fn comp() -> Comp {
        let mut c = Comp::new();
        for k in 0..3 {
            let mut l = Layer::new(format!("c{k}"), Content::text("Bonjour", 60.0, [255, 255, 255, 255], true), 300.0 + k as f32 * 200.0, 400.0, 4.0);
            let key = |t: f32, v: f32| Key { t, v, ease: Ease::Linear, ease2: Ease2::default(), bez: None };
            l.props[4].keys = vec![key(0.0, 0.0), key(1.0, 100.0)];
            c.layers.push(l);
        }
        c.layers[1].props[3].base = 15.0;
        c.markers.push(Marker { t: 1.0, name: "m".into() });
        c
    }

    #[test]
    fn every_tool_changes_the_composition() {
        let mut ids = std::collections::HashSet::new();
        for (id, label, _, f) in TOOLS {
            assert!(ids.insert(id), "{id} en double");
            let before = comp();
            let mut c = before.clone();
            assert!(f(&mut c, Some(1)), "{label} ne fait rien");
            assert!(c != before, "{label} ne change rien");
            for l in &c.layers {
                for p in &l.props {
                    assert!(p.keys.windows(2).all(|w| w[0].t <= w[1].t), "{label}");
                }
            }
            let _ = c.to_svg(0.5);
        }
        let mut c = comp();
        assert!(!TOOLS[0].3(&mut c, None));
        let mut f = comp();
        TOOLS.iter().find(|t| t.0 == "faster").unwrap().3(&mut f, Some(0));
        assert_eq!(f.layers[0].props[4].keys[1].t, 0.5);
    }
}
