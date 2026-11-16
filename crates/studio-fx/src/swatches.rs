//! Adobe swatch files: .ase (Adobe Swatch Exchange, all Creative Cloud apps) and .aco (Photoshop).
//! RGB, CMYK, grey and Lab colours are converted to sRGB; spot/process flags are ignored.

fn be16(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]))
}

fn be32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

fn f32be(b: &[u8], i: usize) -> Option<f32> {
    be32(b, i).map(f32::from_bits)
}

fn to8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn cmyk(c: f32, m: f32, y: f32, k: f32) -> [u8; 3] {
    [to8((1.0 - c) * (1.0 - k)), to8((1.0 - m) * (1.0 - k)), to8((1.0 - y) * (1.0 - k))]
}

/// CIE Lab (D50, L 0..100) to sRGB.
fn lab(l: f32, a: f32, b: f32) -> [u8; 3] {
    let fy = (l + 16.0) / 116.0;
    let (fx, fz) = (fy + a / 500.0, fy - b / 200.0);
    let inv = |t: f32| if t > 6.0 / 29.0 { t * t * t } else { 3.0 * (6.0f32 / 29.0).powi(2) * (t - 4.0 / 29.0) };
    let (x, y, z) = (0.9642 * inv(fx), inv(fy), 0.8251 * inv(fz));
    let lin = [3.1339 * x - 1.6169 * y - 0.4906 * z, -0.9788 * x + 1.9161 * y + 0.0335 * z, 0.0719 * x - 0.2290 * y + 1.4052 * z];
    lin.map(|v| to8(if v <= 0.0031308 { 12.92 * v } else { 1.055 * v.max(0.0).powf(1.0 / 2.4) - 0.055 }))
}

pub fn parse_ase(b: &[u8]) -> Result<Vec<(String, [u8; 3])>, String> {
    if b.get(0..4) != Some(b"ASEF") {
        return Err("ce n'est pas un fichier .ase".into());
    }
    let blocks = be32(b, 8).ok_or("fichier tronqué")? as usize;
    let (mut i, mut out) = (12usize, Vec::new());
    for _ in 0..blocks {
        let (Some(kind), Some(len)) = (be16(b, i), be32(b, i + 2)) else { break };
        let body = i + 6;
        i = body + len as usize;
        if kind != 0x0001 {
            continue;
        }
        let Some(nlen) = be16(b, body) else { break };
        let name_end = body + 2 + nlen as usize * 2;
        let units: Vec<u16> = (body + 2..name_end).step_by(2).filter_map(|k| be16(b, k)).take_while(|u| *u != 0).collect();
        let name = String::from_utf16_lossy(&units);
        let model = b.get(name_end..name_end + 4).ok_or("couleur tronquée")?;
        let v = |k: usize| f32be(b, name_end + 4 + k * 4).unwrap_or(0.0);
        let rgb = match model {
            b"RGB " => [to8(v(0)), to8(v(1)), to8(v(2))],
            b"CMYK" => cmyk(v(0), v(1), v(2), v(3)),
            b"Gray" => [to8(v(0)); 3],
            b"LAB " => lab(v(0) * 100.0, v(1), v(2)),
            _ => continue,
        };
        out.push((name, rgb));
    }
    if out.is_empty() { Err("aucune couleur dans ce nuancier".into()) } else { Ok(out) }
}

pub fn parse_aco(b: &[u8]) -> Result<Vec<(String, [u8; 3])>, String> {
    let read = |start: usize, named: bool| -> Option<(Vec<(String, [u8; 3])>, usize)> {
        if be16(b, start)? != if named { 2 } else { 1 } {
            return None;
        }
        let n = be16(b, start + 2)? as usize;
        let (mut i, mut out) = (start + 4, Vec::new());
        for k in 0..n {
            let space = be16(b, i)?;
            let w: [f32; 4] = std::array::from_fn(|j| be16(b, i + 2 + j * 2).unwrap_or(0) as f32);
            i += 10;
            let mut name = format!("Couleur {}", k + 1);
            if named {
                let len = be32(b, i + 2)? as usize;
                let units: Vec<u16> = (0..len).filter_map(|j| be16(b, i + 6 + j * 2)).take_while(|u| *u != 0).collect();
                name = String::from_utf16_lossy(&units);
                i += 6 + len * 2;
            }
            let rgb = match space {
                0 => [w[0], w[1], w[2]].map(|v| to8(v / 65535.0)),
                2 => cmyk(1.0 - w[0] / 65535.0, 1.0 - w[1] / 65535.0, 1.0 - w[2] / 65535.0, 1.0 - w[3] / 65535.0),
                7 => lab(w[0] / 100.0, (w[1] as u16 as i16) as f32 / 100.0, (w[2] as u16 as i16) as f32 / 100.0),
                8 => [to8(w[0] / 10000.0); 3],
                _ => continue,
            };
            out.push((name, rgb));
        }
        Some((out, i))
    };
    let (v1, end) = read(0, false).ok_or("ce n'est pas un fichier .aco")?;
    let colors = read(end, true).map(|(v2, _)| v2).filter(|v2| v2.len() == v1.len()).unwrap_or(v1);
    if colors.is_empty() { Err("aucune couleur dans ce nuancier".into()) } else { Ok(colors) }
}

/// Picks the parser from the extension.
pub fn load(path: &std::path::Path) -> Result<Vec<(String, [u8; 3])>, String> {
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    match path.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("ase") => parse_ase(&b),
        Some("aco") => parse_aco(&b),
        _ => Err("nuancier .ase ou .aco attendu".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ase(colors: &[(&str, &[u8; 4], &[f32])]) -> Vec<u8> {
        let mut b = b"ASEF".to_vec();
        b.extend(1u16.to_be_bytes());
        b.extend(0u16.to_be_bytes());
        b.extend((colors.len() as u32).to_be_bytes());
        for (name, model, vals) in colors {
            let mut body = Vec::new();
            let units: Vec<u16> = name.encode_utf16().chain([0]).collect();
            body.extend((units.len() as u16).to_be_bytes());
            units.iter().for_each(|u| body.extend(u.to_be_bytes()));
            body.extend(*model);
            vals.iter().for_each(|v| body.extend(v.to_be_bytes()));
            body.extend(2u16.to_be_bytes());
            b.extend(1u16.to_be_bytes());
            b.extend((body.len() as u32).to_be_bytes());
            b.extend(body);
        }
        b
    }

    #[test]
    fn ase_and_aco() {
        let file = ase(&[("Rouge", b"RGB ", &[1.0, 0.0, 0.0]), ("Noir", b"CMYK", &[0.0, 0.0, 0.0, 1.0]), ("Gris", b"Gray", &[0.5]), ("Blanc Lab", b"LAB ", &[1.0, 0.0, 0.0])]);
        let c = parse_ase(&file).unwrap();
        assert_eq!(c[0], ("Rouge".to_string(), [255, 0, 0]));
        assert_eq!(c[1].1, [0, 0, 0]);
        assert_eq!(c[2].1, [128, 128, 128]);
        assert!(c[3].1.iter().all(|v| *v >= 250), "{:?}", c[3]);
        assert!(parse_ase(b"nope").is_err());

        let mut aco = Vec::new();
        aco.extend(1u16.to_be_bytes());
        aco.extend(1u16.to_be_bytes());
        for v in [0u16, 0, 65535, 0, 0] {
            aco.extend(v.to_be_bytes());
        }
        aco.extend(2u16.to_be_bytes());
        aco.extend(1u16.to_be_bytes());
        for v in [0u16, 0, 65535, 0, 0, 0] {
            aco.extend(v.to_be_bytes());
        }
        aco.extend(5u32.to_be_bytes());
        for u in "Vert".encode_utf16().chain([0]) {
            aco.extend(u.to_be_bytes());
        }
        assert_eq!(parse_aco(&aco).unwrap(), vec![("Vert".to_string(), [0, 255, 0])]);
    }
}
