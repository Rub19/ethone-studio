//! Photoshop .psd reading (8-bit RGB): the flattened image, or every pixel layer at canvas size.

use image::RgbaImage;
use std::path::Path;

pub struct PsdLayer {
    pub name: String,
    pub img: RgbaImage,
    pub visible: bool,
    pub opacity: f32,
}

pub fn is_psd(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("psd"))
}

fn parse(bytes: &[u8]) -> Result<psd::Psd, String> {
    psd::Psd::from_bytes(bytes).map_err(|e| format!("PSD illisible : {e}"))
}

pub fn flat(bytes: &[u8]) -> Result<RgbaImage, String> {
    let p = parse(bytes)?;
    RgbaImage::from_raw(p.width(), p.height(), p.rgba()).ok_or_else(|| "PSD : image composite invalide".into())
}

/// Layers bottom to top. A file without pixel layers gives its flattened image as one layer.
pub fn layers(bytes: &[u8]) -> Result<(u32, u32, Vec<PsdLayer>), String> {
    let p = parse(bytes)?;
    let (w, h) = (p.width(), p.height());
    let mut out: Vec<PsdLayer> = p
        .layers()
        .iter()
        .filter_map(|l| {
            let img = RgbaImage::from_raw(w, h, l.rgba())?;
            Some(PsdLayer { name: l.name().to_string(), img, visible: l.visible(), opacity: l.opacity() as f32 / 255.0 })
        })
        .collect();
    if out.is_empty() {
        out.push(PsdLayer { name: "Fond".into(), img: RgbaImage::from_raw(w, h, p.rgba()).ok_or("PSD : image composite invalide")?, visible: true, opacity: 1.0 });
    }
    Ok((w, h, out))
}

/// Opens any image the `image` crate reads, plus .psd (flattened).
pub fn open_any(path: &Path) -> Result<RgbaImage, String> {
    if is_psd(path) {
        flat(&std::fs::read(path).map_err(|e| e.to_string())?)
    } else {
        crate::color::open_srgb(path).map(|(i, _)| i.into_rgba8()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_garbage_and_detects_extension() {
        assert!(super::flat(b"not a psd").is_err());
        assert!(super::is_psd(std::path::Path::new("a/B.PSD")));
        assert!(!super::is_psd(std::path::Path::new("a/b.png")));
    }

    #[test]
    fn reads_layers_of_a_real_file() {
        // Fixture from the psd crate (MIT): a red layer over a blue one, 1×1, normal blend.
        let bytes = include_bytes!("../tests-data/blue-red-1x1-normal.psd");
        let (w, h, l) = super::layers(bytes).unwrap();
        assert_eq!((w, h, l.len()), (1, 1, 2), "{:?}", l.iter().map(|x| &x.name).collect::<Vec<_>>());
        assert!(l.iter().all(|x| x.img.dimensions() == (1, 1)));
        assert_eq!(super::flat(bytes).unwrap().dimensions(), (1, 1));
    }
}

/// Where apps that reference images by path keep the PNG copy of a .psd.
pub fn png_copy_path(psd: &Path, dir: &Path) -> std::path::PathBuf {
    dir.join(format!("{}.png", psd.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "psd".into())))
}
