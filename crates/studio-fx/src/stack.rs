use crate::filters::{ADJUSTMENTS, FILTERS, Kind, QUICK};
use image::RgbaImage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fx {
    pub kind: String,
    pub vals: Vec<f32>,
    pub amount: f32,
    pub on: bool,
}

impl Fx {
    pub fn new(k: Kind) -> Self {
        Fx { kind: format!("{k:?}"), vals: k.defaults(), amount: 100.0, on: true }
    }
}

pub fn groups() -> Vec<(&'static str, &'static [Kind])> {
    let mut g: Vec<(&'static str, &'static [Kind])> = vec![("Réglages", ADJUSTMENTS), ("Rapides", QUICK)];
    g.extend(crate::fx::GROUPS.iter().copied());
    g.extend(crate::fx2::GROUPS.iter().copied());
    g.extend(FILTERS.iter().copied());
    g
}

pub fn kind(name: &str) -> Option<Kind> {
    groups().into_iter().flat_map(|g| g.1.iter().copied()).find(|k| format!("{k:?}") == name)
}

pub fn active(list: &[Fx]) -> bool {
    list.iter().any(|f| f.on && f.amount > 0.0)
}

pub fn apply(img: &mut RgbaImage, list: &[Fx]) {
    for f in list.iter().filter(|f| f.on && f.amount > 0.0) {
        let Some(k) = kind(&f.kind) else { continue };
        let out = k.apply(img, &f.vals);
        let a = (f.amount / 100.0).clamp(0.0, 1.0);
        img.pixels_mut().zip(out.pixels()).for_each(|(p, q)| {
            for i in 0..3 {
                p.0[i] = (p.0[i] as f32 + (q.0[i] as f32 - p.0[i] as f32) * a).round() as u8;
            }
        });
    }
}

pub fn count() -> usize {
    groups().iter().map(|g| g.1.len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_roundtrip() {
        let f = Fx::new(Kind::Relight);
        assert_eq!(kind(&f.kind), Some(Kind::Relight));
        let mut img = RgbaImage::from_pixel(8, 8, image::Rgba([100, 120, 140, 255]));
        apply(&mut img, &[Fx { amount: 0.0, ..Fx::new(Kind::Invert) }]);
        assert_eq!(img.get_pixel(0, 0).0, [100, 120, 140, 255]);
        apply(&mut img, &[Fx::new(Kind::Invert)]);
        assert_eq!(img.get_pixel(0, 0).0, [155, 135, 115, 255]);
        assert!(count() > 150);
        println!("fx2 options: {}", crate::fx2::option_count());
    }
}
