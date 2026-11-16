//! Text recognition shared by the suite, on top of the OCR engine built into Windows.

use image::RgbaImage;

pub struct Word {
    pub text: String,
    /// x, y, w, h in image pixels
    pub rect: [f32; 4],
}

pub struct Line {
    pub text: String,
    /// x, y, w, h in image pixels (union of the words)
    pub rect: [f32; 4],
    pub words: Vec<Word>,
}

pub struct Page {
    pub text: String,
    pub words: Vec<Word>,
    pub lines: Vec<Line>,
    /// Text angle detected by the engine, in degrees.
    pub angle: f64,
}

#[cfg(windows)]
pub fn languages() -> Vec<(String, String)> {
    use windows::Media::Ocr::OcrEngine;
    OcrEngine::AvailableRecognizerLanguages().map(|v| v.into_iter().filter_map(|l| Some((l.LanguageTag().ok()?.to_string(), l.DisplayName().ok()?.to_string()))).collect()).unwrap_or_default()
}

/// Fixes classic OCR slips: "50 0/0" or "500/0" read for "50%", stray spaces before punctuation.
pub fn tidy(s: &str) -> String {
    let mut out = s.replace("o/o", "%").replace("O/O", "%");
    while let Some(i) = out.find("0/0") {
        let before = &out[..i];
        let keep = before.trim_end().chars().last().is_some_and(|c| c.is_ascii_digit());
        if !keep {
            break;
        }
        out = format!("{}%{}", out[..i].trim_end(), &out[i + 3..]);
    }
    out.replace(" %", "%")
}

fn union(r: &[[f32; 4]]) -> [f32; 4] {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for q in r {
        x0 = x0.min(q[0]);
        y0 = y0.min(q[1]);
        x1 = x1.max(q[0] + q[2]);
        y1 = y1.max(q[1] + q[3]);
    }
    if x0 > x1 { [0.0; 4] } else { [x0, y0, x1 - x0, y1 - y0] }
}

/// Recognises the text of an image. `lang` is a BCP-47 tag ("fr-FR"), or empty for the user's languages.
/// `upscale` enlarges small images first, which helps with small print.
#[cfg(windows)]
pub fn recognize_with(img: &RgbaImage, lang: &str, upscale: bool) -> Result<Page, String> {
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;
    use windows::core::HSTRING;
    let e = |x: windows::core::Error| x.message().to_string();
    let engine = if lang.is_empty() { OcrEngine::TryCreateFromUserProfileLanguages() } else { Language::CreateLanguage(&HSTRING::from(lang)).and_then(|l| OcrEngine::TryCreateFromLanguage(&l)) }.map_err(|_| "Aucune langue de reconnaissance installée (Paramètres Windows › Langue)".to_string())?;
    let max = OcrEngine::MaxImageDimension().unwrap_or(10000).max(1000) as f32;
    let big = img.width().max(img.height()) as f32;
    let k = if upscale && big < 1600.0 { (2.0f32).min(max / big) } else { (max / big).min(1.0) };
    let (iw, ih) = (((img.width() as f32 * k) as u32).max(1), ((img.height() as f32 * k) as u32).max(1));
    let src = if (k - 1.0).abs() > 1e-3 { image::imageops::resize(img, iw, ih, if k > 1.0 { image::imageops::FilterType::CatmullRom } else { image::imageops::FilterType::Triangle }) } else { img.clone() };
    let mut bgra = src.into_raw();
    for p in bgra.chunks_mut(4) {
        p.swap(0, 2);
        p[3] = 255;
    }
    let w = DataWriter::new().map_err(e)?;
    w.WriteBytes(&bgra).map_err(e)?;
    let buf = w.DetachBuffer().map_err(e)?;
    let bmp = SoftwareBitmap::CreateCopyFromBuffer(&buf, BitmapPixelFormat::Bgra8, iw as i32, ih as i32).map_err(e)?;
    let res = engine.RecognizeAsync(&bmp).map_err(e)?.join().map_err(e)?;
    let angle = res.TextAngle().ok().and_then(|a| a.Value().ok()).unwrap_or(0.0);
    let mut text = String::new();
    let mut words = Vec::new();
    let mut lines = Vec::new();
    for line in res.Lines().map_err(e)? {
        let lt = tidy(&line.Text().map_err(e)?.to_string());
        text.push_str(&lt);
        text.push('\n');
        let mut lw = Vec::new();
        for wd in line.Words().map_err(e)? {
            let r = wd.BoundingRect().map_err(e)?;
            lw.push(Word { text: tidy(&wd.Text().map_err(e)?.to_string()), rect: [r.X / k, r.Y / k, r.Width / k, r.Height / k] });
        }
        let rect = union(&lw.iter().map(|w| w.rect).collect::<Vec<_>>());
        words.extend(lw.iter().map(|w| Word { text: w.text.clone(), rect: w.rect }));
        lines.push(Line { text: lt, rect, words: lw });
    }
    Ok(Page { text, words, lines, angle })
}

#[cfg(windows)]
pub fn recognize(img: &RgbaImage, lang: &str) -> Result<Page, String> {
    recognize_with(img, lang, false)
}

#[cfg(not(windows))]
pub fn languages() -> Vec<(String, String)> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn recognize_with(_: &RgbaImage, _: &str, _: bool) -> Result<Page, String> {
    Err("La reconnaissance de texte utilise le moteur OCR de Windows".into())
}

#[cfg(not(windows))]
pub fn recognize(img: &RgbaImage, lang: &str) -> Result<Page, String> {
    recognize_with(img, lang, false)
}

/// Estimated (text, background) colours inside a rectangle of an image.
pub fn colors(img: &RgbaImage, r: [f32; 4]) -> ([u8; 4], [u8; 4]) {
    let (w, h) = img.dimensions();
    let x0 = (r[0].max(0.0) as u32).min(w.saturating_sub(1));
    let y0 = (r[1].max(0.0) as u32).min(h.saturating_sub(1));
    let x1 = ((r[0] + r[2]).ceil() as u32).clamp(x0 + 1, w);
    let y1 = ((r[1] + r[3]).ceil() as u32).clamp(y0 + 1, h);
    let mut border: Vec<[u8; 4]> = Vec::new();
    for x in x0..x1 {
        border.push(img.get_pixel(x, y0).0);
        border.push(img.get_pixel(x, y1 - 1).0);
    }
    for y in y0..y1 {
        border.push(img.get_pixel(x0, y).0);
        border.push(img.get_pixel(x1 - 1, y).0);
    }
    let med = |v: &mut Vec<u8>| {
        v.sort_unstable();
        v[v.len() / 2]
    };
    let bg = [0, 1, 2].map(|k| med(&mut border.iter().map(|p| p[k]).collect()));
    let dist = |p: [u8; 4]| (0..3).map(|k| (p[k] as i32 - bg[k] as i32).abs()).sum::<i32>();
    let mut acc = [0u64; 3];
    let mut n = 0u64;
    for y in y0..y1 {
        for x in x0..x1 {
            let p = img.get_pixel(x, y).0;
            if dist(p) > 90 {
                for k in 0..3 {
                    acc[k] += p[k] as u64;
                }
                n += 1;
            }
        }
    }
    let text = if n > 0 {
        [(acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8, 255]
    } else {
        let l = bg[0] as u32 * 3 + bg[1] as u32 * 6 + bg[2] as u32;
        if l > 1280 { [0, 0, 0, 255] } else { [255, 255, 255, 255] }
    };
    (text, [bg[0], bg[1], bg[2], 255])
}

#[cfg(test)]
mod tests {
    #[test]
    fn colors_of_dark_text_on_light() {
        let mut img = image::RgbaImage::from_pixel(40, 20, image::Rgba([240, 240, 230, 255]));
        for x in 10..30 {
            for y in 8..12 {
                img.put_pixel(x, y, image::Rgba([20, 30, 160, 255]));
            }
        }
        let (t, b) = super::colors(&img, [0.0, 0.0, 40.0, 20.0]);
        assert_eq!(b, [240, 240, 230, 255]);
        assert_eq!(t, [20, 30, 160, 255]);
        assert_eq!(super::tidy("SOLDES -500/0"), "SOLDES -50%");
        assert_eq!(super::tidy("Remise 30 0/0 ce jour"), "Remise 30% ce jour");
        assert_eq!(super::tidy("ratio 0/0"), "ratio 0/0");
        assert_eq!(super::union(&[[1.0, 2.0, 3.0, 4.0], [5.0, 1.0, 1.0, 1.0]]), [1.0, 1.0, 5.0, 5.0]);
    }
}
