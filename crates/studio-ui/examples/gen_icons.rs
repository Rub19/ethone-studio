use resvg::{tiny_skia, usvg};

pub const APPS: [(&str, &str, &str); 8] = [
    ("pixel", "#FF5A7E", "#E0184B"),
    ("cut", "#C77DFF", "#8A2BE2"),
    ("vector", "#FFB23F", "#F07800"),
    ("motion", "#4CC9F0", "#0A84D6"),
    ("lumen", "#3EE0A4", "#0FA36B"),
    ("page", "#FF8A5B", "#E2461B"),
    ("sound", "#7C83FF", "#4B3BDB"),
    ("studio", "#8B7CFF", "#2E8BFF"),
];

fn glyph(id: &str) -> (String, String) {
    let cut = r#"fill="black""#;
    match id {
        "pixel" => (
            r#"<rect x="15" y="25" width="23" height="23" rx="5" fill="white" fill-opacity="0.55"/>
<rect x="25" y="15" width="24" height="24" rx="5" fill="white"/>"#
                .into(),
            r#"<circle cx="31" cy="21.5" r="2.6" fill="black"/><path d="M27 35 L33 28 L37 32 L40 29 L46 35 Z" fill="black"/>"#.into(),
        ),
        "cut" => (r#"<path d="M23 16.5 Q23 13 26.2 14.7 L47 27.3 Q50.2 29.2 47 31.1 L26.2 43.7 Q23 45.4 23 41.9 Z" fill="white" transform="translate(0 2.5)"/>"#.into(), format!(r#"<path d="M15 47 L50 17" stroke="black" stroke-width="4" stroke-linecap="round" {cut}/>"#)),
        "vector" => (
            r#"<path d="M32 12 L45 31 Q46.5 33.5 44.8 35.8 L38 46 H26 L19.2 35.8 Q17.5 33.5 19 31 Z" fill="white"/>
<rect x="25" y="48" width="14" height="5" rx="2" fill="white"/>"#
                .into(),
            r#"<path d="M32 18 V33" stroke="black" stroke-width="3" stroke-linecap="round"/><circle cx="32" cy="35" r="3.4" fill="black"/>"#.into(),
        ),
        "motion" => (
            r#"<circle cx="39" cy="32" r="11" fill="white"/>
<path d="M16 25 H25 M12 32 H24 M17 39 H25" stroke="white" stroke-width="4" stroke-linecap="round" stroke-opacity="0.75"/>"#
                .into(),
            String::new(),
        ),
        "lumen" => {
            let mut cuts = String::new();
            for k in 0..6 {
                let a = (k as f32) * std::f32::consts::TAU / 6.0 - 0.35;
                let (x0, y0) = (32.0 + a.cos() * 7.0, 32.0 + a.sin() * 7.0);
                let b = a + 1.25;
                let (x1, y1) = (32.0 + b.cos() * 20.0, 32.0 + b.sin() * 20.0);
                cuts += &format!(r#"<path d="M{x0:.2} {y0:.2} L{x1:.2} {y1:.2}" stroke="black" stroke-width="2.4" stroke-linecap="round"/>"#);
            }
            (r#"<circle cx="32" cy="32" r="18" fill="white"/>"#.into(), cuts)
        }
        "page" => (
            r#"<path d="M19 15 Q19 12 22 12 H37 L46 21 V49 Q46 52 43 52 H22 Q19 52 19 49 Z" fill="white"/>"#.into(),
            r#"<path d="M37 12 V19 Q37 21 39 21 H46" fill="none" stroke="black" stroke-width="2.4"/>
<rect x="24" y="25" width="17" height="9" rx="2" fill="black"/>
<path d="M24 40 H41 M24 46 H35" stroke="black" stroke-width="3" stroke-linecap="round"/>"#
                .into(),
        ),
        "sound" => (
            r#"<path d="M17 28 V36 M24.5 22 V42 M32 15 V49 M39.5 22 V42 M47 28 V36" stroke="white" stroke-width="5" stroke-linecap="round"/>"#.into(),
            String::new(),
        ),
        _ => (
            r#"<rect x="17" y="17" width="13" height="13" rx="4" fill="white"/>
<rect x="34" y="17" width="13" height="13" rx="4" fill="white" fill-opacity="0.8"/>
<rect x="17" y="34" width="13" height="13" rx="4" fill="white" fill-opacity="0.8"/>
<rect x="34" y="34" width="13" height="13" rx="4" fill="white" fill-opacity="0.6"/>"#
                .into(),
            String::new(),
        ),
    }
}

pub fn svg(id: &str, top: &str, bottom: &str) -> String {
    let (shape, cutouts) = glyph(id);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
<defs>
<linearGradient id="bg" x1="0" y1="0" x2="0.35" y2="1"><stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient>
<linearGradient id="hi" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff" stop-opacity="0.28"/><stop offset="0.5" stop-color="#ffffff" stop-opacity="0"/></linearGradient>
<mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"><rect width="64" height="64" fill="black"/><g>{shape}</g><g>{cutouts}</g></mask>
<filter id="sh" x="-20%" y="-20%" width="140%" height="150%"><feDropShadow dx="0" dy="1.4" stdDeviation="1.4" flood-color="#000000" flood-opacity="0.28"/></filter>
</defs>
<rect x="2" y="2" width="60" height="60" rx="14" fill="url(#bg)"/>
<rect x="2" y="2" width="60" height="60" rx="14" fill="url(#hi)"/>
<rect x="2.5" y="2.5" width="59" height="59" rx="13.5" fill="none" stroke="#ffffff" stroke-opacity="0.22" stroke-width="1"/>
<g filter="url(#sh)"><rect width="64" height="64" fill="#ffffff" mask="url(#m)"/></g>
</svg>"##
    )
}

fn render(svg: &str, size: u32) -> image::RgbaImage {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut pix = tiny_skia::Pixmap::new(size, size).unwrap();
    let s = size as f32 / 64.0;
    resvg::render(&tree, tiny_skia::Transform::from_scale(s, s), &mut pix.as_mut());
    let mut img = image::RgbaImage::new(size, size);
    for (d, p) in img.pixels_mut().zip(pix.pixels()) {
        let c = p.demultiply();
        d.0 = [c.red(), c.green(), c.blue(), c.alpha()];
    }
    img
}

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("icons");
    std::fs::create_dir_all(&dir).unwrap();
    for (id, top, bottom) in APPS {
        let s = svg(id, top, bottom);
        std::fs::write(dir.join(format!("{id}.svg")), &s).unwrap();
        render(&s, 256).save(dir.join(format!("{id}.png"))).unwrap();
        let frames: Vec<image::codecs::ico::IcoFrame> = [16, 24, 32, 48, 64, 128, 256]
            .iter()
            .map(|&n| {
                let img = render(&s, n);
                image::codecs::ico::IcoFrame::as_png(img.as_raw(), n, n, image::ExtendedColorType::Rgba8).unwrap()
            })
            .collect();
        image::codecs::ico::IcoEncoder::new(std::fs::File::create(dir.join(format!("{id}.ico"))).unwrap()).encode_images(&frames).unwrap();
    }
    println!("{} icônes écrites dans {}", APPS.len(), dir.display());
}
