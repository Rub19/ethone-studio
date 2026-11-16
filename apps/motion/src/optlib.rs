//! Motion option library: every animation preset × duration × easing × delay (Ctrl+K).

use crate::Motion;
use crate::model::{self, ANIM_PRESETS, Ease, Key};
use eframe::egui;
use studio_ui::palette::{Cmd, Library, icon_cmds, icon_file};

pub const DURATIONS: [f32; 10] = [0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.4, 1.6, 1.8, 2.0];
pub const DELAYS: [f32; 5] = [0.0, 0.1, 0.25, 0.5, 1.0];
pub const EASES: [(Ease, &str); 9] = [(Ease::Smooth, "douce"), (Ease::Linear, "linéaire"), (Ease::In, "accélérée"), (Ease::Out, "ralentie"), (Ease::Back, "avec recul"), (Ease::Elastic, "élastique"), (Ease::Bounce, "rebond"), (Ease::Strong, "forte"), (Ease::Hold, "par paliers")];
/// Presets that animate the end of the layer instead of its start.
const EXITS: [usize; 3] = [7, 14, 15];
/// The wiggle preset has no keyframes to time, so it stays out of the library.
const SKIP: usize = 8;

pub fn lib() -> &'static Library {
    static L: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    L.get_or_init(|| {
        let mut v = Vec::new();
        for (p, name) in ANIM_PRESETS.iter().enumerate().filter(|(p, _)| *p != SKIP) {
            for (d, dur) in DURATIONS.iter().enumerate() {
                for (e, (_, en)) in EASES.iter().enumerate() {
                    for (w, delay) in DELAYS.iter().enumerate() {
                        let delay_txt = if *delay > 0.0 { format!(" · après {delay} s") } else { String::new() };
                        v.push(Cmd::new(format!("Animer : {name} · {dur} s · {en}{delay_txt}"), "Motion · calque sélectionné", format!("mo:{p}:{d}:{e}:{w}")));
                    }
                }
            }
        }
        let entries = (0..ANIM_PRESETS.len()).filter(|p| *p != SKIP && !EXITS.contains(p) && *p < 8);
        for p in entries {
            for q in EXITS {
                for (di, din) in DURATIONS.iter().enumerate() {
                    for (dq, dout) in DURATIONS.iter().enumerate() {
                        for (e, (_, en)) in EASES.iter().enumerate().take(COMBO_EASES) {
                            v.push(Cmd::new(format!("Entrée {} {din} s puis {} {dout} s · {en}", ANIM_PRESETS[p].to_lowercase(), ANIM_PRESETS[q].to_lowercase()), "Motion · entrée et sortie · calque sélectionné", format!("mc:{p}:{q}:{di}:{dq}:{e}")));
                        }
                    }
                }
            }
        }
        v.extend(icon_cmds("Motion", "mi:"));
        Library::new(v)
    })
}

const COMBO_EASES: usize = 5;

/// Applies an "mo:" or "mc:" library entry to a layer; false when the id is not one of them.
pub fn apply_id(l: &mut model::Layer, id: &str, w: f32, h: f32) -> bool {
    let Some((kind, rest)) = id.split_once(':') else { return false };
    let n: Vec<usize> = rest.split(':').filter_map(|x| x.parse().ok()).collect();
    match (kind, n.as_slice()) {
        ("mo", [p, d, e, dl]) if *p < ANIM_PRESETS.len() => match (DURATIONS.get(*d), EASES.get(*e), DELAYS.get(*dl)) {
            (Some(dur), Some((ease, _)), Some(delay)) => {
                apply_timed(l, *p, w, h, *dur, *ease, *delay);
                true
            }
            _ => false,
        },
        ("mc", [p, q, di, dq, e]) if *p < ANIM_PRESETS.len() && *q < ANIM_PRESETS.len() => match (DURATIONS.get(*di), DURATIONS.get(*dq), EASES.get(*e)) {
            (Some(din), Some(dout), Some((ease, _))) => {
                apply_timed(l, *p, w, h, *din, *ease, 0.0);
                apply_timed(l, *q, w, h, *dout, *ease, 0.0);
                true
            }
            _ => false,
        },
        _ => false,
    }
}

/// Applies a preset, then re-times the keyframes it added: new duration, easing and delay.
pub fn apply_timed(l: &mut model::Layer, p: usize, w: f32, h: f32, dur: f32, ease: Ease, delay: f32) {
    let before: Vec<Vec<Key>> = l.props.iter().map(|q| q.keys.clone()).collect();
    model::apply_anim_preset(l, p, w, h);
    let (s, e) = (l.start, l.end.max(l.start + 0.2));
    let d0 = 0.6f32.min((e - s) / 2.0).max(1e-3);
    let k = dur / d0;
    let exit = EXITS.contains(&p);
    for (i, prop) in l.props.iter_mut().enumerate() {
        let old = before.get(i).cloned().unwrap_or_default();
        let fresh: Vec<usize> = (0..prop.keys.len()).filter(|j| !old.contains(&prop.keys[*j])).collect();
        if fresh.is_empty() {
            continue;
        }
        let last = *fresh.last().unwrap_or(&0);
        for j in &fresh {
            let key = &mut prop.keys[*j];
            key.t = if exit { e - delay - (e - key.t) * k } else { s + delay + (key.t - s) * k };
            if *j != last {
                key.ease = ease;
            }
        }
        prop.keys.sort_by(|a, b| a.t.total_cmp(&b.t));
    }
}

impl Motion {
    pub fn run_lib(&mut self, id: &str, ctx: &egui::Context) -> bool {
        if let Some((_, label, _, f)) = id.strip_prefix("tool:").and_then(|t| crate::outils::TOOLS.iter().find(|x| x.0 == t)) {
            let done = f(&mut self.comp, self.sel);
            studio_ui::handoff::toast(ctx, if done { format!("{label} appliqué") } else { format!("{label} : rien à changer") }, done);
            return true;
        }
        if let Some(k) = id.strip_prefix("mi:").and_then(|k| k.parse::<usize>().ok()) {
            match icon_file(k) {
                Ok(p) => self.add_image_path(&p),
                Err(e) => studio_ui::handoff::toast(ctx, format!("Icône : {e}"), false),
            }
            return true;
        }
        if !(id.starts_with("mo:") || id.starts_with("mc:")) {
            return false;
        }
        let (cw, ch) = (self.comp.width as f32, self.comp.height as f32);
        let label = lib().cmds.iter().find(|c| c.id == id).map(|c| c.label.clone()).unwrap_or_default();
        match self.sel.and_then(|i| self.comp.layers.get_mut(i)) {
            Some(l) => {
                if apply_id(l, id, cw, ch) {
                    studio_ui::handoff::toast(ctx, format!("{label} appliqué à « {} »", l.name), true);
                }
            }
            None => studio_ui::handoff::toast(ctx, "Sélectionne d'abord un calque".into(), false),
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Content, Layer};

    #[test]
    fn timed_presets() {
        assert!(lib().len() >= 6750 + 10_500, "{}", lib().len());
        let mut ran = 0;
        for c in &lib().cmds {
            let mut l = Layer::new("t".into(), Content::Null, 100.0, 100.0, 6.0);
            if apply_id(&mut l, &c.id, 1920.0, 1080.0) {
                ran += 1;
                assert!(l.props.iter().any(|p| !p.keys.is_empty()), "{} n'ajoute aucune image clé", c.label);
                for p in &l.props {
                    assert!(p.keys.windows(2).all(|w| w[0].t <= w[1].t) && p.keys.iter().all(|k| k.t.is_finite() && k.v.is_finite()), "{}", c.label);
                }
            }
        }
        assert_eq!(ran, 6750 + 10_500);
        let mut l = Layer::new("t".into(), Content::Null, 100.0, 100.0, 4.0);
        apply_timed(&mut l, 0, 1000.0, 500.0, 1.2, Ease::Linear, 0.5);
        let k = &l.props[4].keys;
        assert_eq!(k.len(), 2);
        assert!((k[0].t - 0.5).abs() < 1e-4 && (k[1].t - 1.7).abs() < 1e-4, "{:?}", k.iter().map(|k| k.t).collect::<Vec<_>>());
        assert_eq!(k[0].ease, Ease::Linear);
        let mut x = Layer::new("x".into(), Content::Null, 100.0, 100.0, 4.0);
        apply_timed(&mut x, 7, 1000.0, 500.0, 1.0, Ease::Smooth, 0.0);
        let k = &x.props[4].keys;
        assert!((k[0].t - (x.end - 1.0)).abs() < 1e-4 && (k[1].t - x.end).abs() < 1e-4);
    }
}
