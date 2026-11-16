use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings};
use image::RgbaImage;
use std::sync::Arc;

pub fn open(bytes: Arc<Vec<u8>>) -> Option<Pdf> {
    Pdf::new(bytes).ok()
}

pub fn count(pdf: &Pdf) -> usize {
    pdf.pages().len()
}

/// Renders a page at `scale` (1.0 = 72 ppp), capped so the bitmap stays under 8000 px.
pub fn page(pdf: &Pdf, i: usize, scale: f32) -> Option<RgbaImage> {
    let page = pdf.pages().get(i)?;
    let (w, h) = page.render_dimensions();
    let scale = scale.min(8000.0 / w.max(h).max(1.0)).max(0.01);
    let cache = RenderCache::new();
    let pix = hayro::render(page, &cache, &InterpreterSettings::default(), &RenderSettings::default(), &PixmapSettings { x_scale: scale, y_scale: scale, bg_color: WHITE });
    RgbaImage::from_raw(pix.width() as u32, pix.height() as u32, pix.data_as_u8_slice().to_vec())
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_generated_pdf() {
        let img = image::RgbaImage::from_pixel(40, 30, image::Rgba([10, 200, 10, 255]));
        let mut d = crate::engine::images_to_pdf(&[img], None).unwrap();
        let bytes = crate::engine::save(&mut d).unwrap();
        let pdf = super::open(std::sync::Arc::new(bytes)).unwrap();
        assert_eq!(super::count(&pdf), 1);
        let r = super::page(&pdf, 0, 2.0).unwrap();
        assert_eq!((r.width(), r.height()), (60, 45));
        let p = r.get_pixel(30, 22).0;
        assert!(p[1] > 150 && p[0] < 80, "{p:?}");
    }
}

#[cfg(all(test, windows))]
mod ocr_tests {
    #[test]
    fn reads_rendered_text() {
        let mut d = crate::engine::images_to_pdf(&[image::RgbaImage::from_pixel(600, 200, image::Rgba([255, 255, 255, 255]))], None).unwrap();
        let s = crate::engine::Stamp { text: "Bonjour Folio 2026".into(), size: 36.0, color: [0, 0, 0], opacity: 1.0, angle: 0.0, bold: false, place: 0, under: false };
        crate::engine::stamp_text(&mut d, &[0], &s, "", None).unwrap();
        let bytes = crate::engine::save(&mut d).unwrap();
        let pdf = super::open(std::sync::Arc::new(bytes)).unwrap();
        let img = super::page(&pdf, 0, 2.0).unwrap();
        let p = studio_ocr::recognize(&img, "").unwrap();
        assert!(p.text.contains("Bonjour"), "{}", p.text);
        assert!(p.words.iter().any(|w| w.text.contains("Folio") && w.rect[2] > 20.0));
        assert_eq!(p.lines.len(), 1);
    }
}
