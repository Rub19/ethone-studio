//! 3D colour lookup tables in the .cube format: parsing, trilinear sampling, a bundled bank of
//! film looks (derby-film, MIT, see assets/luts) and a cache for LUT files chosen by the user.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, PartialEq)]
pub struct Lut3 {
    pub n: usize,
    data: Vec<[f32; 3]>,
    min: [f32; 3],
    max: [f32; 3],
}

pub fn parse_cube(s: &str) -> Result<Lut3, String> {
    let (mut n, mut min, mut max, mut data) = (0usize, [0.0f32; 3], [1.0f32; 3], Vec::new());
    let triple = |rest: &str| -> Option<[f32; 3]> {
        let v: Vec<f32> = rest.split_whitespace().filter_map(|x| x.parse().ok()).collect();
        (v.len() == 3).then(|| [v[0], v[1], v[2]])
    };
    for line in s.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        if let Some(r) = line.strip_prefix("LUT_3D_SIZE") {
            n = r.trim().parse().map_err(|_| "taille invalide")?;
        } else if let Some(r) = line.strip_prefix("DOMAIN_MIN") {
            min = triple(r).ok_or("DOMAIN_MIN invalide")?;
        } else if let Some(r) = line.strip_prefix("DOMAIN_MAX") {
            max = triple(r).ok_or("DOMAIN_MAX invalide")?;
        } else if line.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.') {
            data.push(triple(line).ok_or("ligne de couleur invalide")?);
        }
    }
    if n < 2 || n > 129 {
        return Err("LUT 3D absente (LUT_3D_SIZE manquant ou hors limites)".into());
    }
    if data.len() != n * n * n {
        return Err(format!("{} couleurs au lieu de {}", data.len(), n * n * n));
    }
    Ok(Lut3 { n, data, min, max })
}

impl Lut3 {
    /// Trilinear lookup; red varies fastest, as in every .cube file.
    pub fn apply(&self, c: [f32; 3]) -> [f32; 3] {
        let m = (self.n - 1) as f32;
        let f: [f32; 3] = std::array::from_fn(|k| ((c[k] - self.min[k]) / (self.max[k] - self.min[k]).max(1e-6)).clamp(0.0, 1.0) * m);
        let i0: [usize; 3] = std::array::from_fn(|k| (f[k] as usize).min(self.n - 2));
        let t: [f32; 3] = std::array::from_fn(|k| f[k] - i0[k] as f32);
        let at = |r: usize, g: usize, b: usize| self.data[r + self.n * (g + self.n * b)];
        let mut out = [0.0; 3];
        for (dr, wr) in [(0, 1.0 - t[0]), (1, t[0])] {
            for (dg, wg) in [(0, 1.0 - t[1]), (1, t[1])] {
                for (db, wb) in [(0, 1.0 - t[2]), (1, t[2])] {
                    let w = wr * wg * wb;
                    let v = at(i0[0] + dr, i0[1] + dg, i0[2] + db);
                    for k in 0..3 {
                        out[k] += v[k] * w;
                    }
                }
            }
        }
        out
    }

    /// Mixes the graded colour with the original (amount 0..1).
    pub fn apply_mix(&self, c: [f32; 3], amount: f32) -> [f32; 3] {
        let g = self.apply(c);
        std::array::from_fn(|k| c[k] + (g[k] - c[k]) * amount)
    }
}

/// (key, French name, film stock it models) of the bundled looks.
pub const BANK: [(&str, &str, &str); 7] = [
    ("blush", "Rose poudré", "Kodachrome 64"),
    ("chilly", "Frisquet", "Agfa Ultra 100"),
    ("dreamy", "Rêveur", "Kodak Portra 400"),
    ("moonlit", "Clair de lune", "Kodak Portra 800"),
    ("sunshine", "Plein soleil", "Kodak Ektachrome E100VS"),
    ("toasty", "Doré", "Kodak Gold 200"),
    ("zesty", "Acidulé", "Fujicolor Superia X-TRA 400"),
];

fn bank_bytes(key: &str) -> Option<&'static [u8]> {
    Some(match key {
        "blush" => include_bytes!("../assets/luts/blush.lut3"),
        "chilly" => include_bytes!("../assets/luts/chilly.lut3"),
        "dreamy" => include_bytes!("../assets/luts/dreamy.lut3"),
        "moonlit" => include_bytes!("../assets/luts/moonlit.lut3"),
        "sunshine" => include_bytes!("../assets/luts/sunshine.lut3"),
        "toasty" => include_bytes!("../assets/luts/toasty.lut3"),
        "zesty" => include_bytes!("../assets/luts/zesty.lut3"),
        _ => return None,
    })
}

/// Compact bank format: one byte for the size n, then n³ RGB triplets in 8 bits, red fastest.
fn parse_bin(b: &[u8]) -> Option<Lut3> {
    let n = *b.first()? as usize;
    let body = b.get(1..1 + n * n * n * 3)?;
    let data = body.chunks(3).map(|c| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0]).collect();
    (n >= 2).then_some(Lut3 { n, data, min: [0.0; 3], max: [1.0; 3] })
}

/// Loads "bank:KEY" or a .cube file path once; later calls reuse the parsed table.
pub fn load(source: &str) -> Option<Arc<Lut3>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Arc<Lut3>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(hit) = cache.lock().ok()?.get(source) {
        return hit.clone();
    }
    let lut = match source.strip_prefix("bank:") {
        Some(key) => bank_bytes(key).and_then(parse_bin),
        None => std::fs::read_to_string(source).ok().and_then(|t| parse_cube(&t).ok()),
    }
    .map(Arc::new);
    cache.lock().ok()?.insert(source.to_string(), lut.clone());
    lut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_and_bank() {
        let mut s = String::from("LUT_3D_SIZE 2\n");
        for b in 0..2 {
            for g in 0..2 {
                for r in 0..2 {
                    s += &format!("{r} {g} {b}\n");
                }
            }
        }
        let id = parse_cube(&s).unwrap();
        let c = [0.2, 0.55, 0.9];
        let o = id.apply(c);
        assert!((0..3).all(|k| (o[k] - c[k]).abs() < 1e-5), "{o:?}");
        assert!(parse_cube("LUT_3D_SIZE 3\n0 0 0\n").is_err());
        for (key, _, _) in BANK {
            let l = load(&format!("bank:{key}")).unwrap_or_else(|| panic!("{key}"));
            assert_eq!(l.n, 17);
            let g = l.apply([0.5, 0.5, 0.5]);
            assert!((g[0] - g[2]).abs() < 0.08, "{key} {g:?}");
        }
        assert!(load("bank:nope").is_none());
    }
}
