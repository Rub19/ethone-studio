//! Cut option library: looks, grading, texture, speed, transitions, framing and fades for the
//! selected clip (Ctrl+K).

use crate::model::{Clip, LOOKS, TRANSITIONS};
use crate::{Cut, Sel};
use eframe::egui;
use studio_ui::palette::{Cmd, Library};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Look(u8, f32, f32, f32),
    Grade(f32, f32, f32),
    Tone(f32, f32, f32),
    Texture(f32, f32),
    Retro(f32, f32),
    Speed(f32),
    Transition(u8, f64, u8),
    Frame(f32, f32, f32),
    KenBurns(f32, f32, f32),
    Fades(f64, f64),
    Opacity(f32),
}

const ANCHORS: [(f32, f32, &str); 9] = [(0.0, 0.0, "centre"), (-1.0, -1.0, "haut gauche"), (0.0, -1.0, "haut"), (1.0, -1.0, "haut droite"), (-1.0, 0.0, "gauche"), (1.0, 0.0, "droite"), (-1.0, 1.0, "bas gauche"), (0.0, 1.0, "bas"), (1.0, 1.0, "bas droite")];
const ALIGNS: [&str; 3] = ["centrée", "au début", "à la fin"];

pub fn ops() -> &'static Vec<(Op, String, &'static str)> {
    static O: std::sync::OnceLock<Vec<(Op, String, &'static str)>> = std::sync::OnceLock::new();
    O.get_or_init(|| {
        let mut v: Vec<(Op, String, &'static str)> = Vec::new();
        let sign = |x: f32| if x > 0.0 { format!("+{x}") } else { format!("{x}") };
        for (l, look) in LOOKS.iter().enumerate().skip(1) {
            for amt in (1..=10).map(|k| k as f32 / 10.0) {
                for t in [-30.0, -20.0, -10.0, 0.0, 10.0, 20.0, 30.0] {
                    for s in [-20.0, 0.0, 20.0] {
                        let warm = if t == 0.0 { String::new() } else { format!(" · température {}", sign(t)) };
                        let sat = if s == 0.0 { String::new() } else { format!(" · saturation {}", sign(s)) };
                        v.push((Op::Look(l as u8, amt, t, s), format!("Look {look} {:.0} %{warm}{sat}", amt * 100.0), "Look"));
                    }
                }
            }
        }
        for t in (-10..=10).map(|k| k as f32 * 5.0) {
            for tint in (-5..=5).map(|k| k as f32 * 10.0) {
                for vig in [0.0, 20.0, 40.0, 60.0, 80.0] {
                    v.push((Op::Grade(t, tint, vig), format!("Étalonnage température {} · teinte {} · vignettage {vig}", sign(t), sign(tint)), "Étalonnage"));
                }
            }
        }
        for c in (-4..=4).map(|k| k as f32 * 10.0) {
            for s in (-4..=4).map(|k| k as f32 * 10.0) {
                for b in [-20.0, -10.0, 0.0, 10.0, 20.0] {
                    v.push((Op::Tone(c, s, b), format!("Contraste {} · saturation {} · luminosité {}", sign(c), sign(s), sign(b)), "Correction"));
                }
            }
        }
        for g in [0.0, 20.0, 40.0, 60.0, 80.0, 100.0] {
            for sh in [0.0, 20.0, 40.0, 60.0, 80.0, 100.0] {
                v.push((Op::Texture(g, sh), format!("Grain {g} · netteté {sh}"), "Texture"));
            }
        }
        for px in (1..=10).map(|k| k as f32 * 10.0) {
            for po in [0.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 12.0] {
                v.push((Op::Retro(px, po), format!("Rétro : pixels {px} · {} niveaux", if po == 0.0 { "sans réduction de".into() } else { po.to_string() }), "Rétro"));
            }
        }
        for s in (1..=40).map(|k| k as f32 / 10.0) {
            v.push((Op::Speed(s), format!("Vitesse ×{s}"), "Vitesse"));
        }
        for (t, name) in TRANSITIONS.iter().enumerate().skip(1) {
            for d in [0.2, 0.3, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0] {
                for (a, an) in ALIGNS.iter().enumerate() {
                    v.push((Op::Transition(t as u8, d, a as u8), format!("Transition {name} {d} s {an}"), "Transition vers le clip"));
                }
            }
        }
        for z in (10..=30).map(|k| k as f32 / 10.0) {
            for (x, y, an) in ANCHORS {
                v.push((Op::Frame(z, x, y), format!("Cadrage zoom {:.0} % · {an}", z * 100.0), "Cadrage"));
            }
        }
        for e in (1..=10).map(|k| 1.0 + k as f32 * 0.05) {
            for (x, y, an) in ANCHORS {
                v.push((Op::KenBurns(e, x, y), format!("Ken Burns zoom ×{e:.2} vers {an}"), "Mouvement"));
            }
        }
        let fades = [0.0, 0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0];
        for a in fades {
            for b in fades {
                if a > 0.0 || b > 0.0 {
                    v.push((Op::Fades(a, b), format!("Fondu d'entrée {a} s · sortie {b} s"), "Fondus"));
                }
            }
        }
        for o in (2..=20).map(|k| k as f32 * 0.05) {
            v.push((Op::Opacity(o), format!("Opacité {:.0} %", o * 100.0), "Calque"));
        }
        v
    })
}

pub fn lib() -> &'static Library {
    static L: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    L.get_or_init(|| Library::new(ops().iter().enumerate().map(|(k, (_, label, hint))| Cmd::new(label.clone(), format!("Cut · {hint} · clip sélectionné"), format!("ct:{k}"))).collect()))
}

pub fn apply(c: &mut Clip, op: Op) {
    match op {
        Op::Look(l, amt, t, s) => (c.look, c.look_amt, c.temperature, c.saturation) = (l, amt, t, s),
        Op::Grade(t, tint, vig) => (c.temperature, c.tint, c.vignette) = (t, tint, vig),
        Op::Tone(ct, s, b) => (c.contrast, c.saturation, c.brightness) = (ct, s, b),
        Op::Texture(g, sh) => (c.grain, c.sharpen) = (g, sh),
        Op::Retro(px, po) => (c.pixelate, c.posterize) = (px, po),
        Op::Speed(s) => c.speed = s as f64,
        Op::Transition(t, d, a) => (c.trans, c.trans_dur, c.trans_align) = (t, d, a),
        Op::Frame(z, x, y) => {
            let reach = (z - 1.0) / z;
            (c.zoom, c.pos_x, c.pos_y, c.zoom_end) = (z, -x * reach, -y * reach, 0.0);
        }
        Op::KenBurns(e, x, y) => {
            let reach = (e - 1.0) / e;
            (c.zoom, c.zoom_end, c.pos_x, c.pos_y) = (1.0, e, -x * reach, -y * reach);
        }
        Op::Fades(a, b) => {
            let half = c.len() / 2.0;
            (c.fade_in, c.fade_out) = (a.min(half), b.min(half));
        }
        Op::Opacity(o) => c.opacity = o,
    }
}

impl Cut {
    pub fn run_lib(&mut self, id: &str, ctx: &egui::Context) -> bool {
        let Some(k) = id.strip_prefix("ct:").and_then(|k| k.parse::<usize>().ok()) else { return false };
        let Some((op, label, _)) = ops().get(k).cloned() else { return true };
        let sel = match self.sel {
            Some(s @ (Sel::Clip(_) | Sel::Over(_))) => s,
            _ => {
                studio_ui::handoff::toast(ctx, "Sélectionne d'abord un clip".into(), false);
                return true;
            }
        };
        if matches!(op, Op::Transition(..)) && matches!(sel, Sel::Clip(0) | Sel::Over(_)) {
            studio_ui::handoff::toast(ctx, "Une transition se place sur un clip de la piste principale qui suit un autre clip".into(), false);
            return true;
        }
        if let Some(c) = self.clip_mut(sel) {
            apply(c, op);
            studio_ui::handoff::toast(ctx, format!("{label} appliqué"), true);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_and_ops() {
        assert!(lib().len() >= 6000, "{}", lib().len());
        let mut c = Clip::default();
        apply(&mut c, Op::Look(3, 0.6, 10.0, -20.0));
        assert_eq!((c.look, c.look_amt, c.temperature, c.saturation), (3, 0.6, 10.0, -20.0));
        apply(&mut c, Op::Frame(2.0, 1.0, 0.0));
        assert_eq!((c.zoom, c.pos_x, c.pos_y), (2.0, -0.5, 0.0));
    }
}
