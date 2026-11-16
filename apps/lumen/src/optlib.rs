//! Lumen option library: every look × intensity × white balance, searchable in Ctrl+K.

use crate::Lumen;
use crate::develop::{self, Settings};
use eframe::egui;
use serde_json::Value;
use studio_ui::palette::{Cmd, Library};

const AMOUNTS: [u8; 10] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
const WB: [(f32, &str); 8] = [(0.0, "balance neutre"), (-30.0, "froid −30"), (-20.0, "froid −20"), (-10.0, "froid −10"), (10.0, "chaud +10"), (20.0, "chaud +20"), (30.0, "chaud +30"), (45.0, "très chaud +45")];
/// Geometry stays the photo's own when a look is applied.
const KEEP: [&str; 6] = ["crop", "straighten", "rotate", "flip", "flip_v", "aspect"];

pub fn lib() -> &'static Library {
    static L: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    L.get_or_init(|| {
        let mut v = Vec::new();
        for (p, (group, name, _)) in develop::preset_catalog().iter().enumerate() {
            for a in AMOUNTS {
                for (w, (_, wn)) in WB.iter().enumerate() {
                    v.push(Cmd::new(format!("Look {name} · {a} % · {wn}"), format!("Lumen · {group} · photo courante ou choisies"), format!("lm:{p}:{a}:{w}")));
                }
            }
        }
        Library::new(v)
    })
}

/// Blends every number of two settings (arrays and nested values included); flags switch at 50 %.
pub fn lerp_json(a: &Value, b: &Value, t: f32) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) if x.is_i64() && y.is_i64() => {
            let (x, y) = (x.as_i64().unwrap_or(0) as f64, y.as_i64().unwrap_or(0) as f64);
            serde_json::json!((x + (y - x) * t as f64).round() as i64)
        }
        (Value::Number(x), Value::Number(y)) if x.is_u64() && y.is_u64() => {
            let (x, y) = (x.as_u64().unwrap_or(0) as f64, y.as_u64().unwrap_or(0) as f64);
            serde_json::json!((x + (y - x) * t as f64).round() as u64)
        }
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap_or(0.0), y.as_f64().unwrap_or(0.0));
            serde_json::json!(x + (y - x) * t as f64)
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => Value::Array(x.iter().zip(y).map(|(p, q)| lerp_json(p, q, t)).collect()),
        (Value::Object(x), Value::Object(y)) => Value::Object(y.iter().map(|(k, q)| (k.clone(), x.get(k).map_or(q.clone(), |p| lerp_json(p, q, t)))).collect()),
        _ => if t >= 0.5 { b.clone() } else { a.clone() },
    }
}

pub fn look(current: &Settings, preset: &Settings, amount: f32, wb: f32) -> Settings {
    let (base, cur, target) = (serde_json::to_value(Settings::default()).unwrap_or_default(), serde_json::to_value(current).unwrap_or_default(), serde_json::to_value(preset).unwrap_or_default());
    let mut out = lerp_json(&base, &target, amount);
    if let (Value::Object(o), Value::Object(c)) = (&mut out, &cur) {
        for k in KEEP {
            if let Some(v) = c.get(k) {
                o.insert(k.into(), v.clone());
            }
        }
    }
    let mut s: Settings = serde_json::from_value(out).unwrap_or_else(|_| preset.clone());
    s.temperature = (s.temperature + wb).clamp(-100.0, 100.0);
    s
}

impl Lumen {
    pub fn run_lib(&mut self, id: &str, ctx: &egui::Context) -> bool {
        let Some(rest) = id.strip_prefix("lm:") else { return false };
        let n: Vec<usize> = rest.split(':').filter_map(|x| x.parse().ok()).collect();
        let cat = develop::preset_catalog();
        let (Some(&p), Some(&a), Some(&w)) = (n.first(), n.get(1), n.get(2)) else { return true };
        let (Some((_, name, preset)), Some((wb, _))) = (cat.get(p), WB.get(w)) else { return true };
        let targets: Vec<usize> = self.targets().into_iter().filter(|i| *i < self.files.len()).collect();
        if targets.is_empty() {
            studio_ui::handoff::toast(ctx, "Choisis d'abord une photo".into(), false);
            return true;
        }
        for i in &targets {
            let s = look(&self.settings(*i), preset, a as f32 / 100.0, *wb);
            self.set_settings(*i, s);
        }
        studio_ui::handoff::toast(ctx, format!("Look {name} à {a} % appliqué à {} photo(s)", targets.len()), true);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_size_and_blend() {
        assert!(lib().len() >= 6000, "{}", lib().len());
        let preset = Settings { exposure: 1.0, contrast: 40.0, mono: true, ..Settings::default() };
        let cur = Settings { straighten: 3.0, ..Settings::default() };
        let half = look(&cur, &preset, 0.5, 10.0);
        assert!((half.exposure - 0.5).abs() < 1e-4 && (half.contrast - 20.0).abs() < 1e-3);
        assert!(half.mono);
        assert_eq!(half.straighten, 3.0);
        assert_eq!(half.temperature, 10.0);
        assert!(!look(&cur, &preset, 0.3, 0.0).mono);
    }
}
