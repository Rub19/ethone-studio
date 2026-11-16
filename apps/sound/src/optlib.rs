//! Sound option library: pitch, speed, stretch, gain, EQ, modulation, space and dynamics settings
//! applied to the selection (or the whole file) from Ctrl+K.

use crate::Sound;
use crate::audio::Clip;
use eframe::egui;
use studio_ui::palette::{Cmd, Library};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Pitch(f32),
    Speed(f32),
    Stretch(f32),
    Gain(f32),
    Eq(f32, f32, f32),
    Tremolo(f32, f32),
    AutoPan(f32, f32),
    Crush(f32, usize),
    Ring(f32),
    Echo(f32, f32, f32),
    Reverb(f32, f32),
    Compress(f32, f32),
    Limit(f32),
    LowCut(f32),
    HighCut(f32),
    Peak(f32),
    Lufs(f32),
    Fade(bool, f32, u8),
    Chain(u16, u16),
}

fn steps(a: f32, b: f32, step: f32) -> Vec<f32> {
    let n = ((b - a) / step).round() as usize;
    (0..=n).map(|k| ((a + k as f32 * step) * 100.0).round() / 100.0).collect()
}

const SHAPES: [&str; 3] = ["linéaire", "douce", "exponentielle"];

pub fn ops() -> &'static Vec<(Op, String, &'static str)> {
    static O: std::sync::OnceLock<Vec<(Op, String, &'static str)>> = std::sync::OnceLock::new();
    O.get_or_init(|| {
        let mut v: Vec<(Op, String, &'static str)> = Vec::new();
        let sign = |x: f32| if x > 0.0 { format!("+{x}") } else { format!("{x}") };
        for s in steps(-24.0, 24.0, 0.5).into_iter().filter(|s| *s != 0.0) {
            v.push((Op::Pitch(s), format!("Hauteur {} demi-tons", sign(s)), "Hauteur · même durée"));
        }
        for f in steps(0.5, 2.0, 0.01).into_iter().filter(|f| *f != 1.0) {
            v.push((Op::Speed(f), format!("Vitesse ×{f}"), "Vitesse · change aussi la hauteur"));
            v.push((Op::Stretch(f), format!("Étirement ×{f} (même hauteur)"), "Durée · hauteur conservée"));
        }
        for g in steps(-24.0, 24.0, 0.5).into_iter().filter(|g| *g != 0.0) {
            v.push((Op::Gain(g), format!("Gain {} dB", sign(g)), "Volume"));
        }
        let bands = steps(-12.0, 12.0, 1.5);
        for l in &bands {
            for m in &bands {
                for h in &bands {
                    if (*l, *m, *h) != (0.0, 0.0, 0.0) {
                        v.push((Op::Eq(*l, *m, *h), format!("Égaliseur graves {} · médiums {} · aigus {} dB", sign(*l), sign(*m), sign(*h)), "Égaliseur 3 bandes"));
                    }
                }
            }
        }
        for r in steps(0.5, 12.0, 0.5) {
            for d in steps(0.1, 1.0, 0.1) {
                v.push((Op::Tremolo(r, d), format!("Trémolo {r} Hz · profondeur {:.0} %", d * 100.0), "Modulation"));
                v.push((Op::AutoPan(r, d), format!("Auto-pan {r} Hz · amplitude {:.0} %", d * 100.0), "Spatialisation"));
            }
        }
        for bits in 2..=16 {
            for hold in 1..=8 {
                v.push((Op::Crush(bits as f32, hold), format!("Bitcrusher {bits} bits · réduction ×{hold}"), "Lo-fi"));
            }
        }
        for f in (1..=50).map(|k| k as f32 * 40.0) {
            v.push((Op::Ring(f), format!("Modulation en anneau {f} Hz"), "Effet robot"));
        }
        for d in (1..=20).map(|k| k as f32 * 50.0) {
            for fb in steps(0.1, 0.8, 0.1) {
                for (mix, mn) in [(0.2, "léger"), (0.35, "moyen"), (0.5, "fort")] {
                    v.push((Op::Echo(d, fb, mix), format!("Écho {d} ms · retour {:.0} % · {mn}", fb * 100.0), "Espace"));
                }
            }
        }
        for size in steps(0.1, 1.0, 0.1) {
            for mix in steps(0.1, 1.0, 0.1) {
                v.push((Op::Reverb(size, mix), format!("Réverbération taille {:.0} % · mélange {:.0} %", size * 100.0, mix * 100.0), "Espace"));
            }
        }
        for t in steps(-40.0, -4.0, 4.0) {
            for r in [2.0, 3.0, 4.0, 6.0, 8.0, 10.0, 20.0] {
                v.push((Op::Compress(t, r), format!("Compresseur seuil {t} dB · ratio {r}:1"), "Dynamique"));
            }
        }
        for c in steps(-12.0, -0.5, 0.5) {
            v.push((Op::Limit(c), format!("Limiteur plafond {c} dB"), "Dynamique"));
        }
        for f in (1..=20).map(|k| k as f32 * 20.0) {
            v.push((Op::LowCut(f), format!("Coupe-bas {f} Hz"), "Filtre"));
        }
        for f in (0..=32).map(|k| 2000.0 + k as f32 * 500.0) {
            v.push((Op::HighCut(f), format!("Coupe-haut {f} Hz"), "Filtre"));
        }
        for p in steps(-24.0, 0.0, 0.5) {
            v.push((Op::Peak(p), format!("Normaliser la crête à {p} dB"), "Volume"));
        }
        for l in steps(-30.0, -8.0, 1.0) {
            v.push((Op::Lufs(l), format!("Volume perçu {l} LUFS"), "Volume"));
        }
        for secs in steps(0.25, 5.0, 0.25) {
            for (k, sn) in SHAPES.iter().enumerate() {
                v.push((Op::Fade(true, secs, k as u8), format!("Fondu d'entrée {secs} s · {sn}"), "Fondus"));
                v.push((Op::Fade(false, secs, k as u8), format!("Fondu de sortie {secs} s · {sn}"), "Fondus"));
            }
        }
        let idx = |f: &dyn Fn(&Op) -> bool| -> Vec<usize> { v.iter().enumerate().filter(|(_, o)| f(&o.0)).map(|(i, _)| i).collect() };
        let pitch = idx(&|o| matches!(o, Op::Pitch(s) if s.fract() == 0.0 && s.abs() <= 12.0));
        let reverb = idx(&|o| matches!(o, Op::Reverb(..)));
        let echo = idx(&|o| matches!(o, Op::Echo(..)));
        let lowcut = idx(&|o| matches!(o, Op::LowCut(..)));
        let mut chains = Vec::new();
        for (a, b) in pitch.iter().flat_map(|a| reverb.iter().map(move |b| (*a, *b))).chain(echo.iter().flat_map(|a| lowcut.iter().map(move |b| (*a, *b)))) {
            chains.push((Op::Chain(a as u16, b as u16), format!("{} + {}", v[a].1, v[b].1.to_lowercase()), "Chaîne d'effets"));
        }
        v.extend(chains);
        v
    })
}

pub fn lib() -> &'static Library {
    static L: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    L.get_or_init(|| {
        let ops = ops().iter().enumerate().map(|(k, (_, label, hint))| Cmd::new(label.clone(), format!("Sound · {hint} · sélection ou fichier"), format!("sd:{k}")));
        let sfx = crate::sfx::SFX.iter().enumerate().map(|(k, (pack, name, kw, _, _))| Cmd::new(format!("Bruitage {} · {name}", pack.to_lowercase()), format!("Sound · bruitages libres CC0 · {kw} · insérer au curseur"), format!("sx:{k}")));
        Library::new(ops.chain(sfx).collect())
    })
}

pub fn apply(c: &mut Clip, a: usize, b: usize, op: Op) {
    let rate = c.rate as f32;
    match op {
        Op::Pitch(s) => c.pitch(a, b, s),
        Op::Speed(f) => c.speed(a, b, f),
        Op::Stretch(f) => c.stretch(a, b, f),
        Op::Gain(g) => c.gain_db(a, b, g),
        Op::Eq(l, m, h) => {
            c.filter(a, b, 1, 120.0, l, 0.707);
            c.filter(a, b, 0, 1000.0, m, 0.9);
            c.filter(a, b, 2, 8000.0, h, 0.707);
        }
        Op::Tremolo(r, d) => c.tremolo(a, b, r, d),
        Op::AutoPan(r, d) => c.auto_pan(a, b, r, d),
        Op::Crush(bits, hold) => c.bitcrush(a, b, bits, hold),
        Op::Ring(f) => c.ring(a, b, f),
        Op::Echo(d, fb, mix) => c.echo(a, b, d, fb, mix),
        Op::Reverb(size, mix) => c.reverb(a, b, size, mix),
        Op::Compress(t, r) => c.compress(a, b, t, r, 2.0),
        Op::Limit(l) => c.limiter(a, b, l),
        Op::LowCut(f) => c.filter(a, b, 4, f, 0.0, 0.707),
        Op::HighCut(f) => c.filter(a, b, 3, f, 0.0, 0.707),
        Op::Peak(p) => c.normalize(a, b, p),
        Op::Lufs(l) => c.loudness_normalize(a, b, l),
        Op::Chain(x, y) => {
            for k in [x, y] {
                if let Some((op, _, _)) = ops().get(k as usize)
                    && !matches!(op, Op::Chain(..))
                {
                    apply(c, a, b.min(c.len()), *op);
                }
            }
        }
        Op::Fade(fade_in, secs, shape) => {
            let n = ((secs * rate) as usize).min(b.saturating_sub(a));
            if fade_in { c.fade_shape(a, a + n, true, shape) } else { c.fade_shape(b - n, b, false, shape) }
        }
    }
}

/// Decodes one bundled sound effect through a temporary file (the decoder reads paths).
pub fn sfx_clip(k: usize) -> Result<Clip, String> {
    let (_, name, _, bytes, ext) = crate::sfx::SFX.get(k).ok_or("bruitage inconnu")?;
    let path = std::env::temp_dir().join(format!("ethone-sfx-{k}.{ext}"));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    let clip = Clip::decode(&path).map_err(|e| format!("{name} : {e}"));
    let _ = std::fs::remove_file(&path);
    clip
}

impl Sound {
    pub fn run_lib(&mut self, id: &str, _ctx: &egui::Context) -> bool {
        if let Some(k) = id.strip_prefix("sx:").and_then(|k| k.parse::<usize>().ok()) {
            match sfx_clip(k) {
                Ok(c) => self.insert_sfx(c, crate::sfx::SFX[k].1),
                Err(e) => self.notice = Some((e, false)),
            }
            return true;
        }
        let Some(k) = id.strip_prefix("sd:").and_then(|k| k.parse::<usize>().ok()) else { return false };
        let Some((op, label, _)) = ops().get(k).cloned() else { return true };
        if self.clip.len() == 0 {
            self.notice = Some(("Ouvre ou enregistre d'abord un son".into(), false));
            return true;
        }
        self.edit(&label, |c, a, b| apply(c, a, b, op));
        self.notice = Some((format!("{label} appliqué"), true));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_and_ops() {
        assert!(lib().len() >= 6973 + 313 + 12_000, "{}", lib().len());
        let probe = Clip { rate: 8000, ch: vec![(0..800).map(|i| (i as f32 * 0.07).sin() * 0.5).collect(), (0..800).map(|i| (i as f32 * 0.05).cos() * 0.4).collect()] };
        for (op, label, _) in ops() {
            let mut c = probe.clone();
            let n = c.len();
            apply(&mut c, 0, n, *op);
            assert!(c.len() > 0 && c.ch.iter().flatten().all(|v| v.is_finite()), "{label}");
        }
        for k in 0..crate::sfx::SFX.len() {
            let c = sfx_clip(k).unwrap_or_else(|e| panic!("{e}"));
            assert!(c.len() > 100 && !c.ch.is_empty(), "{}", crate::sfx::SFX[k].1);
        }
        let mut ids = std::collections::HashSet::new();
        assert!(ops().iter().all(|o| ids.insert(o.1.clone())), "libellés en double");
        let mut c = Clip { rate: 1000, ch: vec![vec![0.5; 2000]] };
        apply(&mut c, 0, 2000, Op::Gain(-6.0206));
        assert!((c.ch[0][10] - 0.25).abs() < 1e-3);
        apply(&mut c, 0, 2000, Op::Fade(true, 1.0, 0));
        assert!(c.ch[0][0].abs() < 1e-3 && (c.ch[0][1500] - 0.25).abs() < 1e-3);
    }
}
