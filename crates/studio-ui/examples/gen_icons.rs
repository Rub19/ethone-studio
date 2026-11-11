use resvg::{tiny_skia, usvg};

const APPS: [(&str, [u8; 3]); 7] = [
    ("pixel", [255, 92, 120]),
    ("cut", [168, 110, 255]),
    ("vector", [255, 176, 46]),
    ("motion", [64, 200, 255]),
    ("lumen", [70, 220, 160]),
    ("page", [255, 120, 60]),
    ("sound", [120, 230, 90]),
];

fn glyph(id: &str, a: &str) -> String {
    let line = format!(r#"stroke="{a}" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" fill="none""#);
    let thin = format!(r#"stroke="{a}" stroke-opacity="0.6" stroke-width="1.6" fill="none""#);
    match id {
        "pixel" => format!(
            r##"<rect x="14" y="28" width="22" height="22" rx="4" fill="#10121a" stroke="{a}" stroke-opacity="0.45" stroke-width="2"/>
<rect x="21" y="21" width="22" height="22" rx="4" fill="#10121a" stroke="{a}" stroke-opacity="0.7" stroke-width="2"/>
<rect x="28" y="14" width="22" height="22" rx="4" fill="{a}"/>"##
        ),
        "cut" => format!(
            r##"<path d="M24 39 L46 13 M40 39 L18 13" {line}/><circle cx="21" cy="45" r="6.5" {line}/><circle cx="43" cy="45" r="6.5" {line}/><circle cx="32" cy="26.5" r="2.2" fill="#10121a"/>"##
        ),
        "vector" => format!(
            r#"<path d="M12 46 L20 12 M52 18 L44 52" {thin}/><path d="M12 46 C20 12 44 52 52 18" {line}/>
<rect x="8.5" y="42.5" width="7" height="7" fill="{a}"/><rect x="48.5" y="14.5" width="7" height="7" fill="{a}"/>
<circle cx="20" cy="12" r="3" fill="{a}"/><circle cx="44" cy="52" r="3" fill="{a}"/>"#
        ),
        "motion" => format!(
            r#"<path d="M10 46 L54 46 M16 16 L34 16 M12 22 L34 22 M20 28 L34 28" {thin}/>
<path d="M18 41 L23 46 L18 51 L13 46 Z M32 41 L37 46 L32 51 L27 46 Z M46 41 L51 46 L46 51 L41 46 Z" fill="{a}"/>
<circle cx="44" cy="22" r="8" fill="{a}"/>"#
        ),
        "lumen" => {
            let mut s = format!(r#"<circle cx="32" cy="32" r="19" {line}/>"#);
            for k in 0..6 {
                let t = k as f32 * std::f32::consts::TAU / 6.0;
                let b = t + 1.9;
                s += &format!(
                    r#"<path d="M{} {} L{} {}" stroke="{a}" stroke-width="2.2" stroke-linecap="round"/>"#,
                    32.0 + t.cos() * 19.0,
                    32.0 + t.sin() * 19.0,
                    32.0 + b.cos() * 8.0,
                    32.0 + b.sin() * 8.0
                );
            }
            s
        }
        "page" => format!(
            r#"<rect x="17" y="10" width="30" height="44" rx="4" {line}/><rect x="23" y="17" width="18" height="11" rx="2" fill="{a}" fill-opacity="0.8"/>
<path d="M23 35 L41 35 M23 41 L41 41 M23 47 L34 47" stroke="{a}" stroke-opacity="0.7" stroke-width="2" stroke-linecap="round"/>"#
        ),
        _ => {
            let hs = [6.0, 16.0, 30.0, 20.0, 34.0, 14.0, 22.0, 8.0];
            hs.iter()
                .enumerate()
                .map(|(i, h)| {
                    let x = 13.0 + i as f32 * 5.5;
                    format!(r#"<path d="M{x} {} L{x} {}" stroke="{a}" stroke-width="3" stroke-linecap="round"/>"#, 32.0 - h / 2.0, 32.0 + h / 2.0)
                })
                .collect()
        }
    }
}

fn svg(id: &str, c: [u8; 3]) -> String {
    let a = format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
<rect x="1" y="1" width="62" height="62" rx="17" fill="#10121a"/>
<rect x="1" y="1" width="62" height="62" rx="17" fill="{a}" fill-opacity="0.12"/>
<rect x="1.8" y="1.8" width="60.4" height="60.4" rx="16.3" fill="none" stroke="{a}" stroke-opacity="0.8" stroke-width="1.6"/>
{}
</svg>"##,
        glyph(id, &a)
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
    let mut all: Vec<(&str, Vec<u8>)> = Vec::new();
    for (id, c) in APPS {
        let s = svg(id, c);
        render(&s, 256).save(dir.join(format!("{id}.png"))).unwrap();
        let frames: Vec<image::codecs::ico::IcoFrame> = [16, 24, 32, 48, 64, 128, 256]
            .iter()
            .map(|&n| {
                let img = render(&s, n);
                image::codecs::ico::IcoFrame::as_png(img.as_raw(), n, n, image::ExtendedColorType::Rgba8).unwrap()
            })
            .collect();
        let file = std::fs::File::create(dir.join(format!("{id}.ico"))).unwrap();
        image::codecs::ico::IcoEncoder::new(file).encode_images(&frames).unwrap();
        all.push((id, s.into_bytes()));
    }
    let studio = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#7c5cff"/><stop offset="0.55" stop-color="#40c8ff"/><stop offset="1" stop-color="#46dca0"/></linearGradient></defs>
<rect x="1" y="1" width="62" height="62" rx="17" fill="#10121a"/>
<rect x="2" y="2" width="60" height="60" rx="16" fill="none" stroke="url(#g)" stroke-width="2.6"/>
<rect x="14" y="14" width="16" height="16" rx="5" fill="#ff5c78"/>
<rect x="34" y="14" width="16" height="16" rx="5" fill="#a86eff"/>
<rect x="14" y="34" width="16" height="16" rx="5" fill="#ffb02e"/>
<rect x="34" y="34" width="16" height="16" rx="5" fill="#40c8ff"/>
</svg>"##;
    render(studio, 256).save(dir.join("studio.png")).unwrap();
    let frames: Vec<image::codecs::ico::IcoFrame> = [16, 24, 32, 48, 64, 128, 256]
        .iter()
        .map(|&n| {
            let img = render(studio, n);
            image::codecs::ico::IcoFrame::as_png(img.as_raw(), n, n, image::ExtendedColorType::Rgba8).unwrap()
        })
        .collect();
    image::codecs::ico::IcoEncoder::new(std::fs::File::create(dir.join("studio.ico")).unwrap()).encode_images(&frames).unwrap();
    println!("{} icônes écrites dans {}", all.len() + 1, dir.display());
}
