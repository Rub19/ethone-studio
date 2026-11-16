//! 38 one-click audio tools (Ctrl+K « Outil : … ») on the selection, or the whole file.
//! Filter kinds of `Clip::filter`: 0 peak, 1 low shelf, 2 high shelf, 3 low-pass, 4 high-pass.

use crate::audio::Clip;

type Tool = fn(&mut Clip, usize, usize);

pub const TOOLS: [(&str, &str, Tool); 38] = [
    ("telephone", "Voix au téléphone", |c, a, b| { c.filter(a, b, 4, 320.0, 0.0, 0.8); c.filter(a, b, 3, 3300.0, 0.0, 0.8); c.saturate(a, b, 1.5); }),
    ("radio", "Vieille radio AM", |c, a, b| { c.filter(a, b, 4, 450.0, 0.0, 0.9); c.filter(a, b, 3, 2600.0, 0.0, 0.9); c.saturate(a, b, 2.5); hiss(c, a, b, 0.012); }),
    ("megaphone", "Mégaphone", |c, a, b| { c.filter(a, b, 4, 700.0, 0.0, 1.0); c.filter(a, b, 0, 1800.0, 9.0, 1.2); c.filter(a, b, 3, 4000.0, 0.0, 0.8); c.saturate(a, b, 4.0); }),
    ("underwater", "Sous l'eau", |c, a, b| { c.filter(a, b, 3, 550.0, 0.0, 0.9); c.mod_delay(a, b, 9.0, 3.0, 0.6, 0.2, 0.5); }),
    ("deep-voice", "Voix grave (−4 demi-tons)", |c, a, b| c.pitch(a, b, -4.0)),
    ("chipmunk", "Voix de souris (+7 demi-tons)", |c, a, b| c.pitch(a, b, 7.0)),
    ("cathedral", "Cathédrale", |c, a, b| c.reverb(a, b, 1.0, 0.55)),
    ("small-room", "Petite pièce", |c, a, b| c.reverb(a, b, 0.15, 0.25)),
    ("haas", "Élargir un son mono (effet Haas)", |c, a, b| { stereo(c); delay_channel(c, 1, a, b, 0.015); }),
    ("narrow", "Rétrécir la stéréo (50 %)", |c, a, b| width(c, a, b, 0.5)),
    ("widen", "Élargir la stéréo (160 %)", |c, a, b| width(c, a, b, 1.6)),
    ("pan-left", "Placer à gauche", |c, a, b| pan(c, a, b, -0.6)),
    ("pan-right", "Placer à droite", |c, a, b| pan(c, a, b, 0.6)),
    ("ping-pong", "Écho ping-pong gauche / droite", ping_pong),
    ("doubler", "Doubleur de voix", |c, a, b| { stereo(c); c.mod_delay(a, b, 22.0, 4.0, 0.3, 0.0, 0.45); }),
    ("fuzz", "Distorsion fuzz", |c, a, b| each(c, a, b, |v| (v * 9.0).clamp(-0.6, 0.6))),
    ("vinyl", "Disque vinyle", vinyl),
    ("cassette", "Cassette usée", cassette),
    ("lofi8k", "Lo-fi 8 kHz", |c, a, b| { c.filter(a, b, 3, 3600.0, 0.0, 0.7); hold(c, a, b, (c.rate / 8000).max(1) as usize); }),
    ("gate-chop", "Hachage rythmique (croches à 120 BPM)", |c, a, b| rhythm(c, a, b, 0.25, |t| if t < 0.5 { 1.0 } else { 0.0 })),
    ("pump", "Pompage sidechain (noires à 120 BPM)", |c, a, b| rhythm(c, a, b, 0.5, |t| (t * 4.0).min(1.0).powf(1.5).max(0.15))),
    ("stutter", "Bégaiement (répète le début ×4)", stutter),
    ("reverse-reverb", "Réverbération inversée", |c, a, b| { c.reverse(a, b); c.reverb(a, b, 0.8, 0.6); c.reverse(a, b); }),
    ("de-esser", "Adoucir les « s » (dé-esseur)", deess),
    ("hum50", "Supprimer le ronflement 50 Hz", |c, a, b| for f in [50.0, 100.0, 150.0, 200.0] { c.filter(a, b, 0, f, -30.0, 12.0) }),
    ("hum60", "Supprimer le ronflement 60 Hz", |c, a, b| for f in [60.0, 120.0, 180.0, 240.0] { c.filter(a, b, 0, f, -30.0, 12.0) }),
    ("presence", "Voix plus présente", |c, a, b| { c.filter(a, b, 4, 90.0, 0.0, 0.7); c.filter(a, b, 0, 3000.0, 4.0, 0.9); c.filter(a, b, 2, 9000.0, 2.0, 0.7); }),
    ("exciter", "Brillance (exciteur d'aigus)", exciter),
    ("keep-center", "Garder le centre (voix seule)", |c, a, b| mid_side(c, a, b, 1.0, 0.0)),
    ("karaoke", "Karaoké (retirer la voix centrée)", |c, a, b| mid_side(c, a, b, 0.0, 1.0)),
    ("phase-fix", "Corriger un canal en opposition de phase", phase_fix),
    ("declick", "Supprimer les clics", declick),
    ("bleep", "Bip de censure", |c, a, b| { let r = c.rate as f32; for ch in &mut c.ch { for (i, v) in ch[a..b].iter_mut().enumerate() { *v = (std::f32::consts::TAU * 1000.0 * i as f32 / r).sin() * 0.3; } } }),
    ("pink-bed", "Ajouter un léger bruit rose", |c, a, b| pink(c, a, b, 0.02)),
    ("freeze", "Geler le son (prolonge le début)", freeze),
    ("granular", "Nuage granulaire", granular),
    ("tape-stop", "Arrêt de bande", |c, a, b| tape(c, a, b, true)),
    ("tape-start", "Démarrage de bande", |c, a, b| tape(c, a, b, false)),
];

fn hash(i: usize, s: u32) -> f32 {
    let mut h = (i as u32).wrapping_mul(2_654_435_761) ^ s.wrapping_mul(40_503);
    h ^= h >> 15;
    h = h.wrapping_mul(2_246_822_519);
    h ^= h >> 13;
    h as f32 / u32::MAX as f32 * 2.0 - 1.0
}

fn each(c: &mut Clip, a: usize, b: usize, f: impl Fn(f32) -> f32) {
    for ch in &mut c.ch {
        for v in &mut ch[a..b] {
            *v = f(*v);
        }
    }
}

fn stereo(c: &mut Clip) {
    if c.ch.len() == 1 {
        let m = c.ch[0].clone();
        c.ch.push(m);
    }
}

fn hiss(c: &mut Clip, a: usize, b: usize, amt: f32) {
    for (k, ch) in c.ch.iter_mut().enumerate() {
        for (i, v) in ch[a..b].iter_mut().enumerate() {
            *v += hash(i, k as u32 + 3) * amt;
        }
    }
}

fn pink(c: &mut Clip, a: usize, b: usize, amt: f32) {
    for (k, ch) in c.ch.iter_mut().enumerate() {
        let mut s = [0f32; 3];
        for (i, v) in ch[a..b].iter_mut().enumerate() {
            let w = hash(i, k as u32 + 11);
            s[0] = 0.997 * s[0] + 0.029 * w;
            s[1] = 0.985 * s[1] + 0.032 * w;
            s[2] = 0.950 * s[2] + 0.048 * w;
            *v += (s[0] + s[1] + s[2] + 0.06 * w) * amt * 4.0;
        }
    }
}

fn delay_channel(c: &mut Clip, k: usize, a: usize, b: usize, secs: f32) {
    let d = (secs * c.rate as f32) as usize;
    if let Some(ch) = c.ch.get_mut(k) {
        let src = ch[a..b].to_vec();
        for i in a..b {
            ch[i] = if i - a >= d { src[i - a - d] } else { 0.0 };
        }
    }
}

fn mid_side(c: &mut Clip, a: usize, b: usize, mid: f32, side: f32) {
    stereo(c);
    for i in a..b {
        let (l, r) = (c.ch[0][i], c.ch[1][i]);
        let (m, s) = ((l + r) / 2.0 * mid, (l - r) / 2.0 * side);
        (c.ch[0][i], c.ch[1][i]) = (m + s, m - s);
    }
}

fn width(c: &mut Clip, a: usize, b: usize, w: f32) {
    mid_side(c, a, b, 1.0, w);
}

fn pan(c: &mut Clip, a: usize, b: usize, p: f32) {
    stereo(c);
    let (gl, gr) = (((1.0 - p) / 2.0).sqrt() * std::f32::consts::SQRT_2, ((1.0 + p) / 2.0).sqrt() * std::f32::consts::SQRT_2);
    for i in a..b {
        c.ch[0][i] *= gl.min(1.0);
        c.ch[1][i] *= gr.min(1.0);
    }
}

fn ping_pong(c: &mut Clip, a: usize, b: usize) {
    stereo(c);
    let d = (0.3 * c.rate as f32) as usize;
    let dry: Vec<f32> = (a..b).map(|i| (c.ch[0][i] + c.ch[1][i]) / 2.0).collect();
    let mut taps = [vec![0f32; b - a], vec![0f32; b - a]];
    for (n, gain) in [(1usize, 0.5f32), (2, 0.35), (3, 0.25), (4, 0.17)] {
        let side = (n + 1) % 2;
        for i in n * d..(b - a) {
            taps[side][i] += dry[i - n * d] * gain;
        }
    }
    for k in 0..2 {
        for i in 0..b - a {
            c.ch[k][a + i] += taps[k][i];
        }
    }
}

fn vinyl(c: &mut Clip, a: usize, b: usize) {
    c.filter(a, b, 3, 9000.0, 0.0, 0.7);
    c.filter(a, b, 4, 60.0, 0.0, 0.7);
    for (k, ch) in c.ch.iter_mut().enumerate() {
        for (i, v) in ch[a..b].iter_mut().enumerate() {
            let crackle = if hash(i, 77) > 0.9993 { hash(i, k as u32 + 5) * 0.35 } else { 0.0 };
            *v += crackle + hash(i, 9) * 0.004;
        }
    }
}

fn cassette(c: &mut Clip, a: usize, b: usize) {
    c.mod_delay(a, b, 6.0, 1.2, 0.8, 0.0, 1.0);
    c.filter(a, b, 3, 7000.0, 0.0, 0.7);
    c.saturate(a, b, 1.3);
    hiss(c, a, b, 0.006);
}

fn hold(c: &mut Clip, a: usize, b: usize, n: usize) {
    for ch in &mut c.ch {
        for i in a..b {
            ch[i] = ch[a + (i - a) / n * n];
        }
    }
}

fn rhythm(c: &mut Clip, a: usize, b: usize, beat: f32, env: impl Fn(f32) -> f32) {
    let period = (beat * c.rate as f32).max(1.0);
    for ch in &mut c.ch {
        for (i, v) in ch[a..b].iter_mut().enumerate() {
            *v *= env((i as f32 % period) / period);
        }
    }
}

fn stutter(c: &mut Clip, a: usize, b: usize) {
    let n = ((b - a) / 4).min((0.12 * c.rate as f32) as usize).max(1);
    let fade = (n / 10).max(1);
    for ch in &mut c.ch {
        let grain: Vec<f32> = ch[a..a + n].to_vec();
        for rep in 1..4 {
            for (j, g) in grain.iter().enumerate() {
                let i = a + rep * n + j;
                if i < b {
                    let e = (j.min(n - 1 - j) as f32 / fade as f32).min(1.0);
                    ch[i] = g * e;
                }
            }
        }
    }
}

fn deess(c: &mut Clip, a: usize, b: usize) {
    let mut hi = c.clone();
    hi.filter(a, b, 4, 5500.0, 0.0, 0.9);
    let win = (c.rate / 200).max(1) as usize;
    for (k, ch) in c.ch.iter_mut().enumerate() {
        let h = &hi.ch[k];
        let mut env = 0f32;
        for i in a..b {
            env = env.max(h[i].abs()) * (1.0 - 1.0 / win as f32) + h[i].abs() / win as f32;
            let over = (env / 0.08).max(1.0);
            ch[i] -= h[i] * (1.0 - 1.0 / over);
        }
    }
}

fn exciter(c: &mut Clip, a: usize, b: usize) {
    let mut hi = c.clone();
    hi.filter(a, b, 4, 4000.0, 0.0, 0.7);
    for (k, ch) in c.ch.iter_mut().enumerate() {
        for i in a..b {
            ch[i] += (hi.ch[k][i] * 4.0).tanh() * 0.12;
        }
    }
}

fn phase_fix(c: &mut Clip, a: usize, b: usize) {
    if c.ch.len() < 2 {
        stereo(c);
        for v in &mut c.ch[1][a..b] {
            *v = -*v;
        }
        return;
    }
    let corr: f32 = (a..b).map(|i| c.ch[0][i] * c.ch[1][i]).sum();
    if corr < 0.0 {
        for v in &mut c.ch[1][a..b] {
            *v = -*v;
        }
    }
}

fn declick(c: &mut Clip, a: usize, b: usize) {
    for ch in &mut c.ch {
        let lo = a.max(1);
        let hi = b.min(ch.len().saturating_sub(1));
        if hi <= lo + 2 {
            continue;
        }
        let residual = |ch: &[f32], i: usize| (ch[i] - (ch[i - 1] + ch[i + 1]) / 2.0).abs();
        let mean = (lo..hi).map(|i| residual(ch, i)).sum::<f32>() / (hi - lo) as f32;
        let limit = (mean * 10.0).max(0.05);
        for i in lo..hi {
            if residual(ch, i) > limit {
                ch[i] = (ch[i - 1] + ch[i + 1]) / 2.0;
            }
        }
    }
}

fn freeze(c: &mut Clip, a: usize, b: usize) {
    let n = ((0.08 * c.rate as f32) as usize).min((b - a) / 2).max(2);
    for ch in &mut c.ch {
        let grain: Vec<f32> = ch[a..a + n].to_vec();
        for i in a + n..b {
            let p = (i - a) % n;
            let q = (p + n / 2) % n;
            let w = (std::f32::consts::PI * p as f32 / n as f32).sin();
            ch[i] = grain[p] * w * w + grain[q] * (1.0 - w * w);
        }
    }
}

fn granular(c: &mut Clip, a: usize, b: usize) {
    let src = c.clone();
    let g = ((0.05 * c.rate as f32) as usize).clamp(4, (b - a).max(4));
    for (k, ch) in c.ch.iter_mut().enumerate() {
        for v in &mut ch[a..b] {
            *v *= 0.3;
        }
        let mut start = a;
        let mut n = 0usize;
        while start + g < b {
            let from = a + ((hash(n, 4) * 0.5 + 0.5) * (b - a - g) as f32) as usize;
            for j in 0..g {
                let w = (std::f32::consts::PI * j as f32 / g as f32).sin();
                ch[start + j] += src.ch[k][from + j] * w * 0.5;
            }
            start += g / 2;
            n += 1;
        }
    }
}

fn tape(c: &mut Clip, a: usize, b: usize, stop: bool) {
    for ch in &mut c.ch {
        let src: Vec<f32> = ch[a..b].to_vec();
        let n = src.len();
        let mut pos = 0f32;
        for i in 0..n {
            let t = i as f32 / n as f32;
            let speed = if stop { (1.0 - t).powf(1.5) } else { t.powf(0.7) };
            let k = pos as usize;
            ch[a + i] = if k + 1 < n { src[k] + (src[k + 1] - src[k]) * pos.fract() } else { 0.0 };
            pos += speed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_runs_and_changes_the_sound() {
        let rate = 8000;
        let make = |stereo: bool| {
            let l: Vec<f32> = (0..rate).map(|i| (i as f32 * 0.11).sin() * 0.4 + (i as f32 * 0.9).sin() * 0.1 + (i as f32 * 2.7).sin() * 0.3).collect();
            let r: Vec<f32> = (0..rate).map(|i| (i as f32 * 0.07).sin() * 0.3).collect();
            Clip { rate: rate as u32, ch: if stereo { vec![l, r] } else { vec![l] } }
        };
        let mut ids = std::collections::HashSet::new();
        for (id, label, f) in TOOLS {
            assert!(ids.insert(id), "{id} en double");
            for st in [true, false] {
                let src = make(st);
                let mut c = src.clone();
                let n = c.len();
                f(&mut c, n / 8, n - n / 8);
                assert!(c.len() > 0 && c.ch.iter().flatten().all(|v| v.is_finite()), "{label}");
                if !matches!(id, "phase-fix" | "declick") || (id == "phase-fix" && !st) {
                    assert!(c.ch != src.ch, "{label} ne change rien ({})", if st { "stéréo" } else { "mono" });
                }
            }
        }
        let mut anti = make(true);
        anti.ch[1] = anti.ch[0].iter().map(|v| -v).collect();
        let n = anti.len();
        phase_fix(&mut anti, 0, n);
        assert_eq!(anti.ch[0], anti.ch[1]);
        let mut clicky = Clip { rate: rate as u32, ch: vec![(0..rate).map(|i| (i as f32 * 0.05).sin() * 0.4).collect()] };
        clicky.ch[0][4000] = 0.95;
        declick(&mut clicky, 0, n);
        assert!(clicky.ch[0][4000].abs() < 0.6, "{}", clicky.ch[0][4000]);
    }
}
