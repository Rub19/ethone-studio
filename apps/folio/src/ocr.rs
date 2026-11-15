use image::RgbaImage;

pub struct Word {
    pub text: String,
    /// x, y, w, h in image pixels
    pub rect: [f32; 4],
}

pub struct Page {
    pub text: String,
    pub words: Vec<Word>,
}

#[cfg(windows)]
pub fn languages() -> Vec<(String, String)> {
    use windows::Media::Ocr::OcrEngine;
    OcrEngine::AvailableRecognizerLanguages().map(|v| v.into_iter().filter_map(|l| Some((l.LanguageTag().ok()?.to_string(), l.DisplayName().ok()?.to_string()))).collect()).unwrap_or_default()
}

#[cfg(windows)]
pub fn recognize(img: &RgbaImage, lang: &str) -> Result<Page, String> {
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;
    use windows::core::HSTRING;
    let e = |x: windows::core::Error| x.message().to_string();
    let engine = if lang.is_empty() { OcrEngine::TryCreateFromUserProfileLanguages() } else { Language::CreateLanguage(&HSTRING::from(lang)).and_then(|l| OcrEngine::TryCreateFromLanguage(&l)) }.map_err(|_| "Aucune langue de reconnaissance installée (Paramètres Windows › Langue)".to_string())?;
    let max = OcrEngine::MaxImageDimension().unwrap_or(10000).max(1000);
    let k = (max as f32 / img.width().max(img.height()) as f32).min(1.0);
    let src = if k < 1.0 { image::imageops::resize(img, (img.width() as f32 * k) as u32, (img.height() as f32 * k) as u32, image::imageops::FilterType::Triangle) } else { img.clone() };
    let mut bgra = src.into_raw();
    for p in bgra.chunks_mut(4) {
        p.swap(0, 2);
    }
    let w = DataWriter::new().map_err(e)?;
    w.WriteBytes(&bgra).map_err(e)?;
    let buf = w.DetachBuffer().map_err(e)?;
    let (iw, ih) = ((img.width() as f32 * k) as i32, (img.height() as f32 * k) as i32);
    let bmp = SoftwareBitmap::CreateCopyFromBuffer(&buf, BitmapPixelFormat::Bgra8, iw, ih).map_err(e)?;
    let res = engine.RecognizeAsync(&bmp).map_err(e)?.join().map_err(e)?;
    let mut text = String::new();
    let mut words = Vec::new();
    for line in res.Lines().map_err(e)? {
        text.push_str(&line.Text().map_err(e)?.to_string());
        text.push('\n');
        for wd in line.Words().map_err(e)? {
            let r = wd.BoundingRect().map_err(e)?;
            words.push(Word { text: wd.Text().map_err(e)?.to_string(), rect: [r.X / k, r.Y / k, r.Width / k, r.Height / k] });
        }
    }
    Ok(Page { text, words })
}

#[cfg(not(windows))]
pub fn languages() -> Vec<(String, String)> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn recognize(_: &RgbaImage, _: &str) -> Result<Page, String> {
    Err("La reconnaissance de texte utilise le moteur OCR de Windows".into())
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn reads_rendered_text() {
        let mut d = crate::engine::images_to_pdf(&[image::RgbaImage::from_pixel(600, 200, image::Rgba([255, 255, 255, 255]))], None).unwrap();
        let s = crate::engine::Stamp { text: "Bonjour Folio 2026".into(), size: 36.0, color: [0, 0, 0], opacity: 1.0, angle: 0.0, bold: false, place: 0, under: false };
        crate::engine::stamp_text(&mut d, &[0], &s, "", None).unwrap();
        let bytes = crate::engine::save(&mut d).unwrap();
        let pdf = crate::render::open(std::sync::Arc::new(bytes)).unwrap();
        let img = crate::render::page(&pdf, 0, 2.0).unwrap();
        let p = super::recognize(&img, "").unwrap();
        assert!(p.text.contains("Bonjour"), "{}", p.text);
        assert!(p.words.iter().any(|w| w.text.contains("Folio") && w.rect[2] > 20.0));
    }
}
