//! Lightroom / Camera Raw presets (.xmp, crs: namespace) mapped onto Lumen settings.
//! Only the global adjustments with a Lumen equivalent are read; the rest of the file is ignored.

use crate::develop::Settings;

/// Value of `crs:key`, written either as an attribute or as an element.
fn get(xmp: &str, key: &str) -> Option<f32> {
    for (open, close) in [(format!("crs:{key}=\""), '"'), (format!("crs:{key}>"), '<')] {
        if let Some(i) = xmp.find(&open) {
            let rest = &xmp[i + open.len()..];
            return rest[..rest.find(close)?].trim().trim_start_matches('+').parse().ok();
        }
    }
    None
}

fn flag(xmp: &str, key: &str) -> bool {
    xmp.contains(&format!("crs:{key}=\"True\"")) || xmp.contains(&format!("crs:{key}>True<"))
}

const BANDS: [&str; 8] = ["Red", "Orange", "Yellow", "Green", "Aqua", "Blue", "Purple", "Magenta"];

pub fn import(xmp: &str, base: &Settings) -> Result<(Settings, usize), String> {
    if !xmp.contains("crs:") {
        return Err("ce fichier ne contient pas de réglages Lightroom (crs:)".into());
    }
    let mut s = base.clone();
    let mut n = 0;
    let mut set = |dst: &mut f32, key: &str, scale: f32| {
        if let Some(v) = get(xmp, key) {
            *dst = (v * scale).clamp(-100.0, 100.0);
            n += 1;
        }
    };
    set(&mut s.contrast, "Contrast2012", 1.0);
    set(&mut s.highlights, "Highlights2012", 1.0);
    set(&mut s.shadows, "Shadows2012", 1.0);
    set(&mut s.whites, "Whites2012", 1.0);
    set(&mut s.blacks, "Blacks2012", 1.0);
    set(&mut s.texture, "Texture", 1.0);
    set(&mut s.clarity, "Clarity2012", 1.0);
    set(&mut s.dehaze, "Dehaze", 1.0);
    set(&mut s.vibrance, "Vibrance", 1.0);
    set(&mut s.saturation, "Saturation", 1.0);
    set(&mut s.vignette, "PostCropVignetteAmount", 1.0);
    set(&mut s.vignette_mid, "PostCropVignetteMidpoint", 1.0);
    set(&mut s.vignette_round, "PostCropVignetteRoundness", 1.0);
    set(&mut s.vignette_feather, "PostCropVignetteFeather", 1.0);
    set(&mut s.grain, "GrainAmount", 1.0);
    set(&mut s.grain_size, "GrainSize", 1.0);
    set(&mut s.grain_rough, "GrainFrequency", 1.0);
    set(&mut s.sharpen, "Sharpness", 100.0 / 150.0);
    set(&mut s.noise, "LuminanceSmoothing", 1.0);
    set(&mut s.color_noise, "ColorNoiseReduction", 1.0);
    set(&mut s.split_shadow_sat, "SplitToningShadowSaturation", 1.0);
    set(&mut s.split_high_sat, "SplitToningHighlightSaturation", 1.0);
    set(&mut s.split_balance, "SplitToningBalance", 1.0);
    set(&mut s.tint, "IncrementalTint", 1.0);
    set(&mut s.temperature, "IncrementalTemperature", 1.0);
    if let Some(v) = get(xmp, "Exposure2012") {
        s.exposure = v.clamp(-5.0, 5.0);
        n += 1;
    }
    if let Some(k) = get(xmp, "Temperature") {
        s.temperature = ((k - 5500.0) / 45.0).clamp(-100.0, 100.0);
        n += 1;
    }
    if let Some(t) = get(xmp, "Tint") {
        s.tint = (t / 1.5).clamp(-100.0, 100.0);
        n += 1;
    }
    for (key, dst) in [("SplitToningShadowHue", &mut s.split_shadow_hue), ("SplitToningHighlightHue", &mut s.split_high_hue)] {
        if let Some(v) = get(xmp, key) {
            *dst = v.clamp(0.0, 360.0);
            n += 1;
        }
    }
    for (b, name) in BANDS.iter().enumerate() {
        for (k, kind) in ["Hue", "Saturation", "Luminance"].iter().enumerate() {
            if let Some(v) = get(xmp, &format!("{kind}Adjustment{name}")) {
                s.hsl[b][k] = v.clamp(-100.0, 100.0);
                n += 1;
            }
        }
    }
    if flag(xmp, "ConvertToGrayscale") {
        s.mono = true;
        n += 1;
    }
    if n == 0 { Err("aucun réglage reconnu dans ce préréglage".into()) } else { Ok((s, n)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_attributes_and_elements() {
        let xmp = r#"<x:xmpmeta><rdf:Description crs:Exposure2012="+0.65" crs:Contrast2012="-12" crs:Temperature="6400" crs:HueAdjustmentBlue="-20" crs:ConvertToGrayscale="False"><crs:Vibrance>+25</crs:Vibrance></rdf:Description></x:xmpmeta>"#;
        let (s, n) = import(xmp, &Settings { straighten: 2.0, ..Settings::default() }).unwrap();
        assert_eq!(n, 5);
        assert_eq!((s.exposure, s.contrast, s.vibrance, s.hsl[5][0], s.straighten), (0.65, -12.0, 25.0, -20.0, 2.0));
        assert!((s.temperature - 20.0).abs() < 0.01 && !s.mono);
        assert!(import("<xml/>", &Settings::default()).is_err());
    }
}
