use crate::{audio, develop, filters, inpaint, motion_model, page_model, vector_model};
use image::{DynamicImage, RgbaImage, imageops};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

type Out = Result<Vec<Value>, String>;

fn text(s: impl Into<String>) -> Value {
    json!({ "type": "text", "text": s.into() })
}

fn preview(img: &RgbaImage) -> Value {
    let small = if img.width().max(img.height()) > 640 { DynamicImage::ImageRgba8(img.clone()).thumbnail(640, 640).to_rgba8() } else { img.clone() };
    let mut png = Vec::new();
    let _ = DynamicImage::ImageRgba8(small).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png);
    json!({ "type": "image", "data": vector_model::base64(&png), "mimeType": "image/png" })
}

fn str_arg<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> {
    a[k].as_str().filter(|s| !s.is_empty()).ok_or(format!("paramètre « {k} » manquant"))
}

fn f(a: &Value, k: &str, d: f32) -> f32 {
    a[k].as_f64().map_or(d, |v| v as f32)
}

fn prepare(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    if let Some(dir) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    Ok(p)
}

fn color(v: &Value) -> Option<[u8; 4]> {
    let s = v.as_str()?.trim_start_matches('#');
    let n = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
    match s.len() {
        6 => Some([n(0)?, n(2)?, n(4)?, 255]),
        8 => Some([n(0)?, n(2)?, n(4)?, n(6)?]),
        _ => None,
    }
}

fn save_image(img: &RgbaImage, path: &Path) -> Result<(), String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
    if ext == "jpg" || ext == "jpeg" { DynamicImage::ImageRgba8(img.clone()).to_rgb8().save(path) } else { img.save(path) }.map_err(|e| e.to_string())
}

fn fontdb() -> Arc<resvg::usvg::fontdb::Database> {
    static DB: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_system_fonts();
        Arc::new(db)
    })
    .clone()
}

fn fontdb47() -> Arc<resvg47::usvg::fontdb::Database> {
    static DB: OnceLock<Arc<resvg47::usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = resvg47::usvg::fontdb::Database::new();
        db.load_system_fonts();
        Arc::new(db)
    })
    .clone()
}

fn render_svg(svg: &str, w: f32, h: f32, scale: f32) -> Result<RgbaImage, String> {
    use resvg::{tiny_skia, usvg};
    let opt = usvg::Options { fontdb: fontdb(), ..Default::default() };
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| e.to_string())?;
    let mut pix = tiny_skia::Pixmap::new(((w * scale).round() as u32).max(1), ((h * scale).round() as u32).max(1)).ok_or("taille invalide")?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
    let mut img = RgbaImage::new(pix.width(), pix.height());
    for (d, s) in img.pixels_mut().zip(pix.pixels()) {
        let c = s.demultiply();
        d.0 = [c.red(), c.green(), c.blue(), c.alpha()];
    }
    Ok(img)
}

fn all_filters() -> Vec<filters::Kind> {
    filters::ADJUSTMENTS.iter().chain(filters::QUICK).chain(filters::FILTERS.iter().flat_map(|g| g.1.iter())).copied().collect()
}

fn filter_doc() -> String {
    all_filters()
        .iter()
        .map(|k| {
            let ps: Vec<String> = k.params().iter().map(|p| format!("{} {}..{} déf. {}", p.label, p.min, p.max, p.default)).collect();
            if ps.is_empty() { format!("{k:?}") } else { format!("{k:?} [{}]", ps.join(", ")) }
        })
        .collect::<Vec<_>>()
        .join(" ; ")
}

pub fn list() -> Vec<Value> {
    let presets: Vec<&str> = develop::presets().iter().map(|p| p.0).collect();
    vec![
        json!({
            "name": "pixel_edit",
            "title": "Pixel : retoucher une image",
            "description": format!("Retouche une image (PNG, JPEG, WebP, BMP) avec le moteur de Pixel et enregistre le résultat. Opérations appliquées dans l'ordre : \
                {{op:'filter', name, params?, region?:{{x,y,width,height}}}} ; {{op:'remove', x,y,width,height}} = suppression d'objet avec reconstruction du fond (oiseaux dans le ciel, passants…) ; \
                {{op:'resize', width?, height?}} ; {{op:'crop', x,y,width,height}} ; {{op:'rotate', degrees:90|180|270}} ; {{op:'flip', axis:'horizontal'|'vertical'}}. \
                Filtres disponibles (nom [paramètres dans l'ordre]) : {}", filter_doc()),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Chemin absolu de l'image source" },
                    "output": { "type": "string", "description": "Chemin absolu du résultat (.png, .jpg, .webp)" },
                    "operations": { "type": "array", "items": { "type": "object" } }
                },
                "required": ["input", "output", "operations"]
            }
        }),
        json!({
            "name": "vector_draw",
            "title": "Vector : dessiner un logo ou une illustration",
            "description": "Crée un dessin vectoriel avec le moteur de Vector et écrit <output>.evec (ouvrable dans Vector), <output>.svg et <output>.png. \
                Formes (dessinées dans l'ordre, la dernière au-dessus) : {type:'rect', x,y,w,h, radius?} ; {type:'ellipse', cx,cy,rx,ry} ; {type:'polygon', cx,cy,r, sides} ; {type:'star', cx,cy,r, points, inner(0..1)} ; \
                {type:'line', x1,y1,x2,y2} ; {type:'arrow', x1,y1,x2,y2, width} ; {type:'heart', x,y,w,h} ; {type:'spiral', cx,cy,r, turns} ; {type:'path', points:[[x,y]…], closed?, smooth?} ; \
                {type:'text', x, y (ligne de base), text, size, font?, bold?}. Style commun : fill '#rrggbb[aa]' ou null, stroke, stroke_width, gradient (2e couleur), gradient_angle, opacity 0..1, rotate (degrés), dash, shadow (bool), blur, name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "output": { "type": "string", "description": "Chemin absolu sans extension (ou finissant par .svg/.png/.evec)" },
                    "width": { "type": "number" },
                    "height": { "type": "number" },
                    "background": { "type": ["string", "null"], "description": "'#rrggbb' ou null pour transparent" },
                    "png_scale": { "type": "number", "description": "Facteur d'export PNG (défaut 2)" },
                    "shapes": { "type": "array", "items": { "type": "object" } }
                },
                "required": ["output", "width", "height", "shapes"]
            }
        }),
        json!({
            "name": "lumen_develop",
            "title": "Lumen : développer une photo",
            "description": format!("Développe une photo (JPEG, PNG, TIFF, WebP, RAW CR2/CR3/NEF/ARW/DNG/RAF…) avec le moteur de Lumen. preset parmi : {}. \
                settings (prioritaire sur le préréglage) : exposure (-4..4), contrast, highlights, shadows, whites, blacks, temperature, tint, vibrance, saturation, texture, clarity, dehaze (-100..100), \
                curve [4 valeurs -100..100], hsl [[teinte,sat,lum]×8 : rouges, oranges, jaunes, verts, turquoises, bleus, violets, magentas], mono, split_shadow_hue/sat, split_high_hue/sat, split_balance, \
                sharpen (0..150), sharpen_radius, noise (0..100), vignette, vignette_mid, grain, grain_size, straighten (-45..45), crop [g,h,d,b fractions], flip, rotate (quarts de tour).", presets.join(", ")),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input": { "type": "string" },
                    "output": { "type": "string", "description": ".jpg ou .png" },
                    "preset": { "type": "string" },
                    "settings": { "type": "object" },
                    "max_size": { "type": "number", "description": "Grand côté maximum en pixels (0 = original)" }
                },
                "required": ["input", "output"]
            }
        }),
        json!({
            "name": "motion_render",
            "title": "Motion : rendre une animation",
            "description": "Rend une composition Motion en GIF animé (output .gif) ou en séquence PNG (output = dossier), et l'enregistre en .emot si project est fourni. \
                comp = { width, height, fps, duration, background:[r,g,b,a], layers:[ { name, start, end, visible:true, \
                content: {Text:{text,size,color:[r,g,b,a],bold,family}} | {Rect:{w,h,r,color}} | {Ellipse:{w,h,color}} | {Star:{points,radius,inner,color}} | {Line:{w,thickness,color}} | {Counter:{from,to,decimals,size,color,prefix,suffix}} | {Image:{path,w,h}}, \
                props: [posX, posY, échelle %, rotation °, opacité %] chacun {base, keys:[{t, v, ease}]} avec ease parmi Linear, Smooth, In, Out, Hold, Back, Elastic, Bounce, \
                fx?: {blur, shadow, stroke:[r,g,b,a]|null, stroke_w, gradient, blend (0 normal,1 produit,2 superposition,3 incrustation…), typewriter (s), wiggle (px), wiggle_freq, loop_keys} } ]. \
                La position est le centre du calque.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "comp": { "type": "object" },
                    "output": { "type": "string" },
                    "project": { "type": "string", "description": "Chemin .emot optionnel" },
                    "gif_width": { "type": "number", "description": "Largeur max du GIF (défaut 800)" }
                },
                "required": ["comp", "output"]
            }
        }),
        json!({
            "name": "page_pdf",
            "title": "Page : mettre en page un PDF",
            "description": "Compose un document multipage avec le moteur de Page et l'exporte en PDF vectoriel (texte réel). Enregistre aussi le projet .epage si project est fourni. Unités en points (A4 = 595 × 842). \
                doc = { width, height, margin, paper?:[r,g,b,a], grid?, master?:{frames:[…]} (répété sur chaque page), pages:[ {frames:[…], no_master?} ] }. \
                frame = { x, y, w, h, opacity?, stroke?:[r,g,b,a], stroke_w?, kind } avec kind : \
                {Text:{text, size, family, bold, color:[r,g,b,a], align (0 gauche,1 centre,2 droite), leading (≈1.35), italic?, tracking?, columns?, gutter?, background?, padding?}} | \
                {Image:{path, cover (true = remplir), radius?}} | {Rect:{fill, radius}} | {Ellipse:{fill}} | {Line:{color, width}}. {page} et {pages} insèrent la numérotation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "doc": { "type": "object" },
                    "output": { "type": "string", "description": "Chemin .pdf" },
                    "project": { "type": "string", "description": "Chemin .epage optionnel" }
                },
                "required": ["doc", "output"]
            }
        }),
        json!({
            "name": "sound_process",
            "title": "Sound : traiter un son",
            "description": "Traite un fichier audio (WAV, MP3, FLAC, OGG, M4A, son d'une vidéo) avec le moteur de Sound et enregistre un WAV. Sans input, part d'un fichier vide (utile avec generate). \
                Opérations dans l'ordre, chacune avec start/end optionnels en secondes (sinon tout le fichier) : gain{db}, normalize{db}, loudness{db} (volume perçu RMS), fade_in{seconds}, fade_out{seconds}, \
                trim{start,end} (garde la plage), cut{start,end} (supprime la plage), eq{low,mid,high} (dB), low_cut{hz}, high_cut{hz}, compress{threshold,ratio,makeup}, gate{threshold}, saturate{drive 0..1}, \
                echo{delay_ms,feedback,mix}, reverb{size,mix}, speed{factor}, reverse, mono, stereo{pan,width}, generate{kind:'silence'|'tone'|'noise', seconds, at?}.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input": { "type": "string" },
                    "output": { "type": "string", "description": "Chemin .wav" },
                    "bits": { "type": "number", "description": "16, 24 ou 32 (flottant), défaut 16" },
                    "operations": { "type": "array", "items": { "type": "object" } }
                },
                "required": ["output", "operations"]
            }
        }),
        json!({
            "name": "open_in_app",
            "title": "Ouvrir dans une app Ethone",
            "description": "Ouvre un fichier dans l'app Ethone Studio installée pour que l'utilisateur continue à la main : pixel (images), vector (.evec), lumen (photos), cut (vidéos, .ecut), motion (.emot), page (.epage), sound (audio).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "enum": ["pixel", "vector", "lumen", "cut", "motion", "page", "sound"] },
                    "path": { "type": "string" }
                },
                "required": ["app"]
            }
        }),
    ]
}

pub fn call(name: &str, a: &Value) -> Out {
    match name {
        "pixel_edit" => pixel_edit(a),
        "vector_draw" => vector_draw(a),
        "lumen_develop" => lumen_develop(a),
        "motion_render" => motion_render(a),
        "page_pdf" => page_pdf(a),
        "sound_process" => sound_process(a),
        "open_in_app" => open_in_app(a),
        _ => Err(format!("outil inconnu : {name}")),
    }
}

fn rect_of(op: &Value, w: u32, h: u32) -> Result<(u32, u32, u32, u32), String> {
    let x = (f(op, "x", 0.0).max(0.0) as u32).min(w.saturating_sub(1));
    let y = (f(op, "y", 0.0).max(0.0) as u32).min(h.saturating_sub(1));
    let rw = (f(op, "width", (w - x) as f32).max(1.0) as u32).min(w - x);
    let rh = (f(op, "height", (h - y) as f32).max(1.0) as u32).min(h - y);
    Ok((x, y, rw, rh))
}

fn pixel_edit(a: &Value) -> Out {
    let input = str_arg(a, "input")?;
    let output = prepare(str_arg(a, "output")?)?;
    let ops = a["operations"].as_array().ok_or("operations doit être une liste")?;
    let mut img = image::open(input).map_err(|e| format!("lecture de {input} : {e}"))?.into_rgba8();
    let mut log = Vec::new();
    for op in ops {
        let kind = op["op"].as_str().unwrap_or_default();
        let (w, h) = img.dimensions();
        match kind {
            "filter" | "adjust" => {
                let want = op["name"].as_str().unwrap_or_default().to_lowercase();
                let k = all_filters().into_iter().find(|k| format!("{k:?}").to_lowercase() == want || k.title().to_lowercase() == want).ok_or(format!("filtre inconnu : {want}"))?;
                let mut params = k.defaults();
                if let Some(given) = op["params"].as_array() {
                    for (p, v) in params.iter_mut().zip(given) {
                        *p = v.as_f64().unwrap_or(*p as f64) as f32;
                    }
                }
                let out = k.apply(&img, &params);
                img = match op.get("region").filter(|r| r.is_object()) {
                    Some(r) => {
                        let (x, y, rw, rh) = rect_of(r, w, h)?;
                        let mut base = img.clone();
                        imageops::replace(&mut base, &imageops::crop_imm(&out, x, y, rw, rh).to_image(), x as i64, y as i64);
                        base
                    }
                    None => out,
                };
                log.push(format!("{} {:?}", k.title(), params));
            }
            "remove" => {
                let (x, y, rw, rh) = rect_of(op, w, h)?;
                let mask: Vec<bool> = (0..w * h).map(|i| (x..x + rw).contains(&(i % w)) && (y..y + rh).contains(&(i / w))).collect();
                img = inpaint::content_aware_fill(&img, &mask, 7);
                log.push(format!("suppression de {rw}×{rh} à ({x},{y})"));
            }
            "resize" => {
                let nw = op["width"].as_f64().map(|v| v as u32);
                let nh = op["height"].as_f64().map(|v| v as u32);
                let (nw, nh) = match (nw, nh) {
                    (Some(a), Some(b)) => (a, b),
                    (Some(a), None) => (a, (h as f64 * a as f64 / w as f64).round() as u32),
                    (None, Some(b)) => ((w as f64 * b as f64 / h as f64).round() as u32, b),
                    _ => return Err("resize : width ou height requis".into()),
                };
                img = imageops::resize(&img, nw.max(1), nh.max(1), imageops::FilterType::Lanczos3);
                log.push(format!("redimensionnée en {nw}×{nh}"));
            }
            "crop" => {
                let (x, y, rw, rh) = rect_of(op, w, h)?;
                img = imageops::crop_imm(&img, x, y, rw, rh).to_image();
                log.push(format!("recadrée en {rw}×{rh}"));
            }
            "rotate" => {
                img = match op["degrees"].as_i64().unwrap_or(90).rem_euclid(360) {
                    90 => imageops::rotate90(&img),
                    180 => imageops::rotate180(&img),
                    270 => imageops::rotate270(&img),
                    _ => img,
                };
                log.push("rotation".into());
            }
            "flip" => {
                img = if op["axis"].as_str() == Some("vertical") { imageops::flip_vertical(&img) } else { imageops::flip_horizontal(&img) };
                log.push("symétrie".into());
            }
            other => return Err(format!("opération inconnue : {other}")),
        }
    }
    save_image(&img, &output)?;
    Ok(vec![text(format!("Image enregistrée : {} ({}×{}). {}", output.display(), img.width(), img.height(), log.join(" ; "))), preview(&img)])
}

fn vector_draw(a: &Value) -> Out {
    use vector_model::{Doc, Geom, Item, Node};
    let raw = str_arg(a, "output")?;
    let base = Path::new(raw);
    let stem = match base.extension().and_then(|e| e.to_str()) {
        Some("svg" | "png" | "evec") => base.with_extension(""),
        _ => base.to_path_buf(),
    };
    prepare(&stem.to_string_lossy())?;
    let (w, h) = (f(a, "width", 1000.0), f(a, "height", 1000.0));
    let mut doc = Doc::new(w, h);
    doc.background = if a.get("background").is_some_and(|v| v.is_null()) { None } else { color(&a["background"]).or(Some([255, 255, 255, 255])) };
    for (i, s) in a["shapes"].as_array().ok_or("shapes doit être une liste")?.iter().enumerate() {
        let t = s["type"].as_str().unwrap_or("rect");
        let n = |k: &str, d: f32| f(s, k, d);
        let geom = match t {
            "rect" => Geom::Rect { x: n("x", 0.0), y: n("y", 0.0), w: n("w", 100.0), h: n("h", 100.0), r: n("radius", 0.0) },
            "ellipse" | "circle" => {
                let r = n("r", 50.0);
                Geom::Ellipse { cx: n("cx", w / 2.0), cy: n("cy", h / 2.0), rx: n("rx", r), ry: n("ry", r) }
            }
            "polygon" => Geom::Path { nodes: vector_model::polygon(n("cx", w / 2.0), n("cy", h / 2.0), n("r", 100.0), n("sides", 6.0) as u32, None), closed: true },
            "star" => Geom::Path { nodes: vector_model::polygon(n("cx", w / 2.0), n("cy", h / 2.0), n("r", 100.0), n("points", 5.0) as u32, Some(n("inner", 0.5))), closed: true },
            "line" => Geom::Path { nodes: vec![Node::corner([n("x1", 0.0), n("y1", 0.0)]), Node::corner([n("x2", 100.0), n("y2", 100.0)])], closed: false },
            "arrow" => Geom::Path { nodes: vector_model::arrow([n("x1", 0.0), n("y1", 0.0)], [n("x2", 100.0), n("y2", 0.0)], n("width", 16.0)), closed: true },
            "heart" => Geom::Path { nodes: vector_model::heart(n("x", 0.0), n("y", 0.0), n("w", 100.0), n("h", 90.0)), closed: true },
            "spiral" => Geom::Path { nodes: vector_model::spiral(n("cx", w / 2.0), n("cy", h / 2.0), n("r", 100.0), n("turns", 3.0)), closed: false },
            "path" => {
                let pts: Vec<[f32; 2]> = s["points"].as_array().ok_or("path : points requis")?.iter().filter_map(|p| Some([p[0].as_f64()? as f32, p[1].as_f64()? as f32])).collect();
                let closed = s["closed"].as_bool().unwrap_or(true);
                let mut nodes: Vec<Node> = pts.into_iter().map(Node::corner).collect();
                if s["smooth"].as_bool().unwrap_or(false) {
                    vector_model::smooth(&mut nodes, closed);
                }
                Geom::Path { nodes, closed }
            }
            "text" => Geom::Text {
                x: n("x", 0.0),
                y: n("y", 100.0),
                text: s["text"].as_str().unwrap_or("Texte").to_string(),
                size: n("size", 64.0),
                family: s["font"].as_str().unwrap_or("Segoe UI").to_string(),
                bold: s["bold"].as_bool().unwrap_or(false),
            },
            other => return Err(format!("forme inconnue : {other}")),
        };
        let open_line = matches!(t, "line" | "spiral") || (t == "path" && !s["closed"].as_bool().unwrap_or(true));
        let fill_default = if open_line { None } else { Some([30, 30, 36, 255]) };
        let mut it = Item {
            name: s["name"].as_str().map(String::from).unwrap_or(format!("{t} {}", i + 1)),
            geom,
            fill: if s.get("fill").is_some_and(|v| v.is_null()) { None } else { color(&s["fill"]).or(fill_default) },
            stroke: color(&s["stroke"]).or(if open_line { Some([30, 30, 36, 255]) } else { None }),
            stroke_w: n("stroke_width", if open_line { 4.0 } else { 2.0 }),
            opacity: n("opacity", 1.0),
            fill2: color(&s["gradient"]),
            grad_angle: n("gradient_angle", 90.0),
            dash: n("dash", 0.0),
            shadow: s["shadow"].as_bool().unwrap_or(false),
            blur: n("blur", 0.0),
            ..Item::blank()
        };
        if let Some(deg) = s["rotate"].as_f64() {
            it.rotate(deg as f32);
        }
        doc.items.push(it);
    }
    let svg = doc.to_svg();
    let scale = f(a, "png_scale", 2.0);
    let img = render_svg(&svg, w, h, scale)?;
    let (evec, svg_path, png) = (stem.with_extension("evec"), stem.with_extension("svg"), stem.with_extension("png"));
    std::fs::write(&evec, serde_json::to_vec_pretty(&doc).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::write(&svg_path, &svg).map_err(|e| e.to_string())?;
    img.save(&png).map_err(|e| e.to_string())?;
    Ok(vec![text(format!("Dessin créé ({} formes) : {} · {} · {} ({}×{})", doc.items.len(), evec.display(), svg_path.display(), png.display(), img.width(), img.height())), preview(&img)])
}

fn merge(base: &mut Value, over: &Value) {
    if let (Some(b), Some(o)) = (base.as_object_mut(), over.as_object()) {
        for (k, v) in o {
            b.insert(k.clone(), v.clone());
        }
    }
}

fn lumen_develop(a: &Value) -> Out {
    let input = str_arg(a, "input")?;
    let output = prepare(str_arg(a, "output")?)?;
    let mut s = match a["preset"].as_str() {
        Some(p) => develop::presets().into_iter().find(|x| x.0.eq_ignore_ascii_case(p)).map(|x| x.1).ok_or(format!("préréglage inconnu : {p}"))?,
        None => develop::Settings::default(),
    };
    if a["settings"].is_object() {
        let mut v = serde_json::to_value(s).map_err(|e| e.to_string())?;
        merge(&mut v, &a["settings"]);
        s = serde_json::from_value(v).map_err(|e| format!("settings : {e}"))?;
    }
    let src = develop::load(Path::new(input), f(a, "max_size", 0.0) as u32)?;
    let out = develop::process(&src, &s);
    let rgba = DynamicImage::ImageRgb8(out).to_rgba8();
    save_image(&rgba, &output)?;
    Ok(vec![text(format!("Photo développée : {} ({}×{})", output.display(), rgba.width(), rgba.height())), preview(&rgba)])
}

fn motion_render(a: &Value) -> Out {
    let comp: motion_model::Comp = serde_json::from_value(a["comp"].clone()).map_err(|e| format!("comp invalide : {e}"))?;
    let output = str_arg(a, "output")?;
    if let Some(p) = a["project"].as_str() {
        std::fs::write(prepare(p)?, serde_json::to_vec_pretty(&comp).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    }
    let (w, h) = (comp.width as f32, comp.height as f32);
    let frames = comp.frames();
    let mid = render_svg(&comp.to_svg(comp.duration * 0.5), w, h, (640.0 / w.max(h)).min(1.0))?;
    if output.to_ascii_lowercase().ends_with(".gif") {
        let path = prepare(output)?;
        let scale = (f(a, "gif_width", 800.0) / w).min(1.0);
        let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
        let mut enc = image::codecs::gif::GifEncoder::new_with_speed(std::io::BufWriter::new(file), 10);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).map_err(|e| e.to_string())?;
        for fr in 0..frames {
            let img = render_svg(&comp.to_svg(fr as f32 / comp.fps as f32), w, h, scale)?;
            enc.encode_frame(image::Frame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(1000, comp.fps))).map_err(|e| e.to_string())?;
        }
        Ok(vec![text(format!("GIF rendu : {} ({frames} images)", path.display())), preview(&mid)])
    } else {
        std::fs::create_dir_all(output).map_err(|e| e.to_string())?;
        for fr in 0..frames {
            let img = render_svg(&comp.to_svg(fr as f32 / comp.fps as f32), w, h, 1.0)?;
            img.save(Path::new(output).join(format!("frame_{fr:05}.png"))).map_err(|e| e.to_string())?;
        }
        Ok(vec![text(format!("Séquence PNG rendue dans {output} ({frames} images)")), preview(&mid)])
    }
}

fn text_width(db: &resvg47::usvg::fontdb::Database, text: &str, family: &str, bold: bool, size: f32) -> f32 {
    use resvg47::usvg::fontdb;
    let q = fontdb::Query { families: &[fontdb::Family::Name(family), fontdb::Family::SansSerif], weight: if bold { fontdb::Weight::BOLD } else { fontdb::Weight::NORMAL }, ..Default::default() };
    db.query(&q)
        .and_then(|id| {
            db.with_face_data(id, |data, idx| {
                let face = ttf_parser::Face::parse(data, idx).ok()?;
                let upem = face.units_per_em() as f32;
                Some(text.chars().map(|c| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map_or(upem * 0.5, |a| a as f32)).sum::<f32>() * size / upem)
            })
        })
        .flatten()
        .unwrap_or(text.chars().count() as f32 * size * 0.5)
}

fn page_pdf(a: &Value) -> Out {
    use krilla_svg::{SurfaceExt, SvgSettings};
    use resvg47::{tiny_skia, usvg};
    let doc: page_model::Doc = serde_json::from_value(a["doc"].clone()).map_err(|e| format!("doc invalide : {e}"))?;
    if doc.pages.is_empty() {
        return Err("le document n'a aucune page".into());
    }
    let output = prepare(str_arg(a, "output")?)?;
    if let Some(p) = a["project"].as_str() {
        std::fs::write(prepare(p)?, serde_json::to_vec_pretty(&doc).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    }
    let db = fontdb47();
    let measure = |t: &str, fam: &str, b: bool, s: f32| text_width(&db, t, fam, b, s);
    let opt = usvg::Options { fontdb: db.clone(), ..Default::default() };
    let mut document = krilla::Document::new();
    let mut overflow = Vec::new();
    let mut first = None;
    for i in 0..doc.pages.len() {
        let (svg, over) = doc.to_svg(i, &measure, false);
        if over.iter().any(|o| *o) {
            overflow.push(i + 1);
        }
        let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| e.to_string())?;
        if i == 0 {
            let scale = 640.0 / doc.width.max(doc.height);
            if let Some(mut pix) = tiny_skia::Pixmap::new((doc.width * scale) as u32, (doc.height * scale) as u32) {
                resvg47::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pix.as_mut());
                let mut img = RgbaImage::new(pix.width(), pix.height());
                for (d, s) in img.pixels_mut().zip(pix.pixels()) {
                    let c = s.demultiply();
                    d.0 = [c.red(), c.green(), c.blue(), c.alpha()];
                }
                first = Some(img);
            }
        }
        let size = krilla::geom::Size::from_wh(doc.width, doc.height).ok_or("taille invalide")?;
        let mut page = document.start_page_with(krilla::page::PageSettings::new(size));
        let mut surface = page.surface();
        surface.draw_svg(&tree, size, SvgSettings::default()).ok_or("rendu SVG impossible")?;
        surface.finish();
        page.finish();
    }
    let bytes = document.finish().map_err(|e| format!("{e:?}"))?;
    std::fs::write(&output, bytes).map_err(|e| e.to_string())?;
    let mut msg = format!("PDF exporté : {} ({} page(s))", output.display(), doc.pages.len());
    if !overflow.is_empty() {
        msg.push_str(&format!(". Attention : du texte déborde de son bloc sur la/les page(s) {overflow:?}, agrandis le bloc ou réduis la taille."));
    }
    let mut out = vec![text(msg)];
    if let Some(img) = first {
        out.push(preview(&img));
    }
    Ok(out)
}

fn sound_process(a: &Value) -> Out {
    let output = prepare(str_arg(a, "output")?)?;
    let mut clip = match a["input"].as_str().filter(|s| !s.is_empty()) {
        Some(p) => audio::Clip::decode(Path::new(p))?,
        None => audio::Clip { rate: 44_100, ch: vec![Vec::new(), Vec::new()] },
    };
    let ops = a["operations"].as_array().ok_or("operations doit être une liste")?;
    for op in ops {
        let name = op["op"].as_str().unwrap_or_default();
        let rate = clip.rate as f32;
        let len = clip.len();
        let at = |k: &str, d: usize| op[k].as_f64().map_or(d, |s| ((s as f32 * rate) as usize).min(len));
        let (s0, s1) = (at("start", 0), at("end", len));
        let (s0, s1) = (s0.min(s1), s1.max(s0));
        let v = |k: &str, d: f32| f(op, k, d);
        if s1 <= s0 && !matches!(name, "generate" | "mono") {
            continue;
        }
        match name {
            "gain" => clip.gain_db(s0, s1, v("db", 0.0)),
            "normalize" => clip.normalize(s0, s1, v("db", -1.0)),
            "loudness" => clip.rms_normalize(s0, s1, v("db", -16.0)),
            "fade_in" => {
                let e = (s0 + (v("seconds", 1.0) * rate) as usize).min(s1);
                clip.fade(s0, e, true);
            }
            "fade_out" => {
                let b = s1.saturating_sub((v("seconds", 1.0) * rate) as usize).max(s0);
                clip.fade(b, s1, false);
            }
            "trim" => clip = clip.slice(s0, s1),
            "cut" => clip.remove(s0, s1),
            "eq" => {
                clip.filter(s0, s1, 1, 120.0, v("low", 0.0), 0.707);
                clip.filter(s0, s1, 0, 1000.0, v("mid", 0.0), 0.9);
                clip.filter(s0, s1, 2, 8000.0, v("high", 0.0), 0.707);
            }
            "low_cut" => clip.filter(s0, s1, 4, v("hz", 80.0), 0.0, 0.707),
            "high_cut" => clip.filter(s0, s1, 3, v("hz", 12_000.0), 0.0, 0.707),
            "compress" => clip.compress(s0, s1, v("threshold", -18.0), v("ratio", 4.0), v("makeup", 3.0)),
            "gate" => clip.gate(s0, s1, v("threshold", -45.0)),
            "saturate" => clip.saturate(s0, s1, v("drive", 0.3)),
            "echo" => clip.echo(s0, s1, v("delay_ms", 250.0), v("feedback", 0.35), v("mix", 0.4)),
            "reverb" => clip.reverb(s0, s1, v("size", 0.5), v("mix", 0.3)),
            "speed" => clip.speed(s0, s1, v("factor", 1.0)),
            "reverse" => clip.reverse(s0, s1),
            "mono" => clip.to_mono(),
            "stereo" => clip.stereo(s0, s1, v("pan", 0.0), v("width", 1.0)),
            "generate" => {
                let kind = match op["kind"].as_str().unwrap_or("silence") {
                    "tone" => 1,
                    "noise" => 2,
                    _ => 0,
                };
                clip.generate(at("at", len), v("seconds", 1.0), kind);
            }
            other => return Err(format!("opération inconnue : {other}")),
        }
    }
    let bits = a["bits"].as_u64().map_or(16, |b| b as u16);
    clip.save_wav(&output, if matches!(bits, 16 | 24 | 32) { bits } else { 16 })?;
    let peak = clip.peak(0, clip.len());
    Ok(vec![text(format!(
        "Son enregistré : {} · {:.2} s · {} Hz · {} canal(aux) · crête {:.1} dB",
        output.display(),
        clip.secs(clip.len()),
        clip.rate,
        clip.ch.len(),
        20.0 * peak.max(1e-6).log10()
    ))])
}

fn open_in_app(a: &Value) -> Out {
    let app = str_arg(a, "app")?;
    if !["pixel", "vector", "lumen", "cut", "motion", "page", "sound"].contains(&app) {
        return Err(format!("app inconnue : {app}"));
    }
    let exe = studio_core::exe_path(app);
    if !exe.exists() {
        return Err(format!("{app} n'est pas installée : installe-la depuis le launcher Ethone Studio."));
    }
    let mut cmd = std::process::Command::new(&exe);
    cmd.current_dir(studio_core::app_dir(app));
    if let Some(p) = a["path"].as_str().filter(|s| !s.is_empty()) {
        cmd.arg(p);
    }
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(vec![text(format!("{app} ouvert{}", a["path"].as_str().map(|p| format!(" avec {p}")).unwrap_or_default()))])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> String {
        std::env::temp_dir().join("ethone-mcp-test").join(name).to_string_lossy().into_owned()
    }

    #[test]
    fn every_tool_produces_its_file() {
        let src = tmp("src.png");
        prepare(&src).unwrap();
        RgbaImage::from_fn(120, 80, |x, y| image::Rgba([(x * 2) as u8, (y * 3) as u8, 160, 255])).save(&src).unwrap();

        let r = call("pixel_edit", &json!({ "input": src, "output": tmp("out.jpg"), "operations": [
            { "op": "filter", "name": "Levels", "params": [10, 1.2, 240] },
            { "op": "filter", "name": "noir et blanc", "region": { "x": 0, "y": 0, "width": 60, "height": 80 } },
            { "op": "remove", "x": 40, "y": 30, "width": 20, "height": 15 },
            { "op": "resize", "width": 60 },
            { "op": "rotate", "degrees": 90 }
        ] })).unwrap();
        assert_eq!(r[1]["type"], "image");
        assert_eq!(image::image_dimensions(tmp("out.jpg")).unwrap(), (40, 60));
        assert!(call("pixel_edit", &json!({ "input": src, "output": tmp("x.png"), "operations": [{ "op": "filter", "name": "inexistant" }] })).is_err());

        let r = call("vector_draw", &json!({ "output": tmp("logo"), "width": 400, "height": 400, "background": "#101218", "shapes": [
            { "type": "ellipse", "cx": 200, "cy": 200, "r": 150, "fill": "#ff7a3c", "gradient": "#ff3b6b", "shadow": true },
            { "type": "star", "cx": 200, "cy": 200, "r": 90, "points": 5, "inner": 0.45, "fill": "#ffffff", "rotate": 10 },
            { "type": "text", "x": 120, "y": 380, "text": "Ethone", "size": 40, "bold": true, "fill": "#ffffff" }
        ] })).unwrap();
        assert!(r[0]["text"].as_str().unwrap().contains("3 formes"));
        assert!(Path::new(&tmp("logo.evec")).exists() && Path::new(&tmp("logo.svg")).exists());
        assert_eq!(image::image_dimensions(tmp("logo.png")).unwrap(), (800, 800));
        let evec: vector_model::Doc = serde_json::from_slice(&std::fs::read(tmp("logo.evec")).unwrap()).unwrap();
        assert_eq!(evec.items.len(), 3);

        call("lumen_develop", &json!({ "input": src, "output": tmp("dev.jpg"), "preset": "Cinéma", "settings": { "exposure": 0.5 } })).unwrap();
        assert!(Path::new(&tmp("dev.jpg")).exists());

        let comp = json!({ "width": 200, "height": 100, "fps": 10, "duration": 1.0, "background": [10, 10, 20, 255], "layers": [{
            "name": "T", "start": 0.0, "end": 1.0, "visible": true,
            "content": { "Text": { "text": "Hi", "size": 40, "color": [255, 255, 255, 255], "bold": true, "family": "Segoe UI" } },
            "props": [ { "base": 100, "keys": [] }, { "base": 50, "keys": [] }, { "base": 100, "keys": [{ "t": 0, "v": 0, "ease": "Back" }, { "t": 0.5, "v": 100, "ease": "Smooth" }] }, { "base": 0, "keys": [] }, { "base": 100, "keys": [] } ]
        }] });
        call("motion_render", &json!({ "comp": comp, "output": tmp("anim.gif"), "project": tmp("anim.emot") })).unwrap();
        assert!(std::fs::metadata(tmp("anim.gif")).unwrap().len() > 100 && Path::new(&tmp("anim.emot")).exists());

        let doc = json!({ "width": 595, "height": 842, "margin": 40, "master": { "frames": [{ "x": 40, "y": 800, "w": 515, "h": 14, "kind": { "Text": { "text": "{page} / {pages}", "size": 8, "family": "Segoe UI", "bold": false, "color": [100, 100, 100, 255], "align": 2, "leading": 1.2 } } }] },
            "pages": [{ "frames": [{ "x": 40, "y": 40, "w": 515, "h": 60, "kind": { "Text": { "text": "Bonjour", "size": 30, "family": "Segoe UI", "bold": true, "color": [0, 0, 0, 255], "align": 0, "leading": 1.2 } } }] }, { "frames": [] }] });
        let r = call("page_pdf", &json!({ "doc": doc, "output": tmp("doc.pdf"), "project": tmp("doc.epage") })).unwrap();
        assert!(r[0]["text"].as_str().unwrap().contains("2 page"));
        assert!(std::fs::read(tmp("doc.pdf")).unwrap().starts_with(b"%PDF"));

        let r = call("sound_process", &json!({ "output": tmp("beep.wav"), "bits": 24, "operations": [
            { "op": "generate", "kind": "tone", "seconds": 1.0 },
            { "op": "fade_out", "seconds": 0.3 },
            { "op": "reverb", "size": 0.4, "mix": 0.2 },
            { "op": "normalize", "db": -1 }
        ] })).unwrap();
        assert!(r[0]["text"].as_str().unwrap().contains("1.00 s"), "{}", r[0]["text"]);
        let back = audio::Clip::decode(Path::new(&tmp("beep.wav"))).unwrap();
        assert_eq!(back.len(), 44_100);

        assert!(call("open_in_app", &json!({ "app": "photoshop" })).is_err());
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("ethone-mcp-test"));
    }
}
