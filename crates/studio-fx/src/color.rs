//! Colour management: embedded ICC profiles are honoured on import (converted to sRGB, the
//! working space of the suite), sRGB is embedded on export, and RGB can be converted to CMYK
//! with a printer profile for soft proofing and print files.

use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};
use moxcms::{ColorProfile, DataColorSpace, Layout, TransformOptions};
use std::path::{Path, PathBuf};

/// The sRGB profile bytes, to embed in exported PNG and JPEG files.
pub fn srgb_icc() -> Vec<u8> {
    ColorProfile::new_srgb().encode().unwrap_or_default()
}

fn is_srgb_like(p: &ColorProfile) -> bool {
    let s = ColorProfile::new_srgb();
    let close = |a: moxcms::Xyzd, b: moxcms::Xyzd| (a.x - b.x).abs() < 0.002 && (a.y - b.y).abs() < 0.002 && (a.z - b.z).abs() < 0.002;
    close(p.red_colorant, s.red_colorant) && close(p.green_colorant, s.green_colorant) && close(p.blue_colorant, s.blue_colorant)
}

/// Converts pixels described by `icc` to sRGB in place. Returns false when the profile is
/// unreadable, not RGB, or already sRGB (the pixels are then left untouched).
pub fn to_srgb(img: &mut RgbaImage, icc: &[u8]) -> bool {
    let Ok(src) = ColorProfile::new_from_slice(icc) else { return false };
    if src.color_space != DataColorSpace::Rgb || is_srgb_like(&src) {
        return false;
    }
    let Ok(t) = src.create_transform_8bit(Layout::Rgba, &ColorProfile::new_srgb(), Layout::Rgba, TransformOptions::default()) else { return false };
    let mut out = vec![0u8; img.as_raw().len()];
    if t.transform(img.as_raw(), &mut out).is_err() {
        return false;
    }
    img.copy_from_slice(&out);
    true
}

/// Opens an image and converts it to sRGB when it carries another RGB profile (AdobeRGB,
/// Display P3…). The flag says whether a conversion happened.
pub fn open_srgb(path: &Path) -> image::ImageResult<(DynamicImage, bool)> {
    let mut decoder = ImageReader::open(path)?.with_guessed_format()?.into_decoder()?;
    let icc = decoder.icc_profile().ok().flatten();
    let img = DynamicImage::from_decoder(decoder)?;
    let Some(icc) = icc else { return Ok((img, false)) };
    let mut rgba = img.to_rgba8();
    if to_srgb(&mut rgba, &icc) { Ok((DynamicImage::ImageRgba8(rgba), true)) } else { Ok((img, false)) }
}

/// A CMYK output profile (a printer or press condition such as FOGRA39).
pub struct Cmyk {
    pub name: String,
    profile: ColorProfile,
}

impl Cmyk {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let profile = ColorProfile::new_from_slice(&bytes).map_err(|e| format!("profil illisible : {e:?}"))?;
        if profile.color_space != DataColorSpace::Cmyk {
            return Err("ce profil n'est pas un profil CMJN".into());
        }
        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(Self { name, profile })
    }

    /// sRGB pixels (RGBA) to CMYK bytes, four per pixel, 0 = no ink.
    pub fn from_srgb(&self, rgba: &[u8]) -> Result<Vec<u8>, String> {
        let t = ColorProfile::new_srgb().create_transform_8bit(Layout::Rgba, &self.profile, Layout::Rgba, TransformOptions::default()).map_err(|e| format!("{e:?}"))?;
        let mut out = vec![0u8; rgba.len()];
        t.transform(rgba, &mut out).map_err(|e| format!("{e:?}"))?;
        Ok(out)
    }

    /// CMYK bytes back to sRGB (RGBA, opaque).
    pub fn to_srgb(&self, cmyk: &[u8]) -> Result<Vec<u8>, String> {
        let t = self.profile.create_transform_8bit(Layout::Rgba, &ColorProfile::new_srgb(), Layout::Rgba, TransformOptions::default()).map_err(|e| format!("{e:?}"))?;
        let mut out = vec![0u8; cmyk.len()];
        t.transform(cmyk, &mut out).map_err(|e| format!("{e:?}"))?;
        for px in out.chunks_exact_mut(4) {
            px[3] = 255;
        }
        Ok(out)
    }

    /// Soft proof: how the image will look once printed with this profile (alpha kept).
    pub fn proof(&self, img: &RgbaImage) -> Result<RgbaImage, String> {
        let mut back = self.to_srgb(&self.from_srgb(img.as_raw())?)?;
        for (b, s) in back.chunks_exact_mut(4).zip(img.as_raw().chunks_exact(4)) {
            b[3] = s[3];
        }
        RgbaImage::from_raw(img.width(), img.height(), back).ok_or_else(|| "taille incohérente".into())
    }
}

/// Writes CMYK bytes (four per pixel, from [`Cmyk::from_srgb`]) as a print-ready TIFF.
pub fn write_cmyk_tiff(path: &Path, width: u32, height: u32, ink: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = tiff::encoder::TiffEncoder::new(std::io::BufWriter::new(file)).map_err(|e| e.to_string())?;
    enc.write_image::<tiff::encoder::colortype::CMYK8>(width, height, ink).map_err(|e| e.to_string())
}

/// CMYK profiles installed on this PC (Windows colour folder), by file name.
pub fn installed_cmyk_profiles() -> Vec<PathBuf> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let dir = Path::new(&windir).join("System32").join("spool").join("drivers").join("color");
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("icc") || x.eq_ignore_ascii_case("icm")))
        .filter(|p| std::fs::read(p).ok().and_then(|b| ColorProfile::new_from_slice(&b).ok()).is_some_and(|c| c.color_space == DataColorSpace::Cmyk))
        .collect();
    v.sort_by_key(|p| {
        let n = p.file_name().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        (!n.contains("fogra39"), !n.contains("fogra"), n)
    });
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_p3_red_is_converted_and_srgb_is_left_alone() {
        let p3 = ColorProfile::new_display_p3().encode().unwrap();
        let mut img = RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
        assert!(to_srgb(&mut img, &p3), "P3 must be converted");
        let px = img.get_pixel(0, 0).0;
        assert!(px[0] >= 250 && px[3] == 255, "red stays red and opaque: {px:?}");
        let mut same = RgbaImage::from_pixel(2, 2, image::Rgba([10, 120, 200, 255]));
        assert!(!to_srgb(&mut same, &srgb_icc()));
        assert_eq!(same.get_pixel(0, 0).0, [10, 120, 200, 255]);
    }

    #[test]
    fn cmyk_tiff_is_written_as_four_channel_cmyk() {
        let path = std::env::temp_dir().join(format!("ethone-cmyk-{}.tif", std::process::id()));
        let ink = [0u8, 255, 255, 0, 10, 20, 30, 40];
        write_cmyk_tiff(&path, 2, 1, &ink).unwrap();
        let mut dec = tiff::decoder::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap())).unwrap();
        assert_eq!(dec.colortype().unwrap(), tiff::ColorType::CMYK(8));
        assert_eq!(dec.dimensions().unwrap(), (2, 1));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cmyk_round_trip_with_an_installed_profile() {
        let Some(path) = installed_cmyk_profiles().into_iter().next() else {
            eprintln!("no CMYK profile installed, skipped");
            return;
        };
        let cmyk = Cmyk::load(&path).unwrap();
        let red = [237u8, 28, 36, 255];
        let ink = cmyk.from_srgb(&red).unwrap();
        assert!(ink[0] < 40 && ink[1] > 200 && ink[2] > 200, "{}: red should be mostly M and Y, got {ink:?}", cmyk.name);
        let back = cmyk.to_srgb(&ink).unwrap();
        assert!(back[0] > 180 && back[1] < 90 && back[2] < 90, "back to red, got {back:?}");
        let proof = cmyk.proof(&RgbaImage::from_pixel(1, 1, image::Rgba([0, 255, 0, 128]))).unwrap();
        assert_eq!(proof.get_pixel(0, 0).0[3], 128, "alpha is kept");
        assert!(proof.get_pixel(0, 0).0[1] < 250, "pure RGB green is out of CMYK gamut, so it dulls");
    }
}
