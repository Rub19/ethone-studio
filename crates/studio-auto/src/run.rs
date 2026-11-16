//! Executes a plan with the suite's engines (ethone-mcp), the Windows OCR and a local vision model.

use crate::plan::{Plan, Step};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

pub enum Msg {
    Step(String),
    Done(Result<Outcome, String>),
}

pub struct Outcome {
    pub file: Option<PathBuf>,
    pub app: &'static str,
    pub opened: String,
    pub notes: Vec<String>,
}

const HOST: &str = "http://127.0.0.1:11434";

fn agent(secs: u64) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(secs))).build().into()
}

pub fn mcp_exe() -> Option<PathBuf> {
    let name = format!("ethone-mcp{}", std::env::consts::EXE_SUFFIX);
    let exe = std::env::current_exe().ok();
    let here = exe.as_ref().and_then(|e| e.parent().map(|d| d.join(&name)));
    let up = exe.as_ref().and_then(|e| e.parent()?.parent().map(|d| d.join(&name)));
    let installed = studio_core::data_dir().join("mcp").join(&name);
    [here, up, Some(installed)].into_iter().flatten().find(|p| p.is_file())
}

pub fn mcp(tool: &str, args: &Value, work: &Path) -> Result<String, String> {
    let exe = mcp_exe().ok_or("Moteur ethone-mcp introuvable : installe-le depuis le launcher")?;
    let f = work.join(format!("{tool}-args.json"));
    std::fs::write(&f, args.to_string()).map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--call").arg(tool).arg(&f).stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_slice(&out.stdout).map_err(|_| format!("{tool} : réponse illisible ({})", String::from_utf8_lossy(&out.stderr).trim()))?;
    let text = v["text"].as_str().unwrap_or_default().to_string();
    if v["ok"].as_bool() == Some(true) { Ok(text) } else { Err(text) }
}

fn ollama_exe() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Programs\\Ollama\\ollama.exe"));
    if let Some(p) = local.filter(|p| p.exists()) {
        return Some(p);
    }
    std::env::var_os("PATH")?.to_str()?.split(';').map(|d| Path::new(d).join("ollama.exe")).find(|p| p.exists())
}

fn ollama_up() -> bool {
    agent(2).get(format!("{HOST}/api/version")).call().is_ok()
}

pub fn ensure_ollama() -> Result<(), String> {
    if ollama_up() {
        return Ok(());
    }
    let exe = ollama_exe().ok_or("Ollama n'est pas installé (ollama.com) : il faut une IA locale pour repérer les objets")?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("serve").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.spawn().map_err(|e| e.to_string())?;
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(500));
        if ollama_up() {
            return Ok(());
        }
    }
    Err("Ollama ne répond pas".into())
}

pub fn vision_model() -> Result<String, String> {
    let mut r = agent(5).get(format!("{HOST}/api/tags")).call().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap_or_default()).unwrap_or_default();
    let names: Vec<String> = v["models"].as_array().map(|a| a.iter().filter_map(|m| m["name"].as_str().map(String::from)).collect()).unwrap_or_default();
    ["qwen3-vl", "qwen2.5vl", "qwen2.5-vl", "gemma3", "llava", "moondream", "minicpm-v", "llama3.2-vision"]
        .iter()
        .find_map(|k| names.iter().find(|n| n.contains(k)).cloned())
        .ok_or("Aucun modèle de vision dans Ollama : lance « ollama pull qwen3-vl:8b »".into())
}

pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            s.push(if k <= c.len() { T[(n >> (18 - 6 * k) & 63) as usize] as char } else { '=' });
        }
    }
    s
}

/// Every `bbox_2d` (or bare 4-number box) found in a model answer.
pub fn boxes(answer: &str) -> Vec<[f32; 4]> {
    let mut out = Vec::new();
    let mut rest = answer;
    while let Some(i) = rest.find("bbox") {
        rest = &rest[i + 4..];
        let Some(a) = rest.find('[') else { break };
        let Some(b) = rest[a..].find(']') else { break };
        let nums: Vec<f32> = rest[a + 1..a + b].split(',').filter_map(|x| x.trim().parse().ok()).collect();
        if nums.len() == 4 {
            out.push([nums[0], nums[1], nums[2], nums[3]]);
        }
        rest = &rest[a + b..];
    }
    out
}

/// Finds `what` in an image; boxes in image pixels (x0, y0, x1, y1).
pub fn detect(img: &image::DynamicImage, what: &str) -> Result<Vec<[f32; 4]>, String> {
    ensure_ollama()?;
    let model = vision_model()?;
    let small = img.thumbnail(1024, 1024).to_rgb8();
    let (sw, sh) = small.dimensions();
    let mut jpg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpg, 90).encode_image(&small).map_err(|e| e.to_string())?;
    let prompt = format!("Detect every instance of: {what}. Answer fast, do not over-think: give the boxes directly as JSON [{{\"bbox_2d\":[x1,y1,x2,y2],\"label\":\"...\"}}] with coordinates normalised 0-1000. If there is none, answer [].");
    let body = json!({ "model": model, "stream": false, "options": { "temperature": 0, "num_predict": 8000 }, "messages": [{ "role": "user", "content": prompt, "images": [base64(&jpg)] }] });
    let mut r = agent(900).post(format!("{HOST}/api/chat")).header("Content-Type", "application/json").send(body.to_string()).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap_or_default()).unwrap_or_default();
    let answer = format!("{} {}", v["message"]["content"].as_str().unwrap_or_default(), if v["message"]["content"].as_str().unwrap_or_default().contains("bbox") { "" } else { v["message"]["thinking"].as_str().unwrap_or_default() });
    let found = boxes(&answer);
    let absolute = found.iter().flatten().any(|v| *v > 1000.0);
    let (iw, ih) = (img.width() as f32, img.height() as f32);
    let k = |v: f32, full: f32, small: f32| if absolute { v / small * full } else { v / 1000.0 * full };
    let mut out: Vec<[f32; 4]> = found.into_iter().map(|b| [k(b[0], iw, sw as f32), k(b[1], ih, sh as f32), k(b[2], iw, sw as f32), k(b[3], ih, sh as f32)]).filter(|b| b[2] > b[0] + 1.0 && b[3] > b[1] + 1.0).collect();
    out.dedup();
    Ok(out)
}

/// Pixels that still stand out around a removed box, as a box (None when the area is clean).
pub fn leftover(img: &image::RgbaImage, b: [f32; 4]) -> Option<[f32; 4]> {
    let r = refine(img, [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0, (b[0] + b[2]) / 2.0 + 0.5, (b[1] + b[3]) / 2.0 + 0.5]);
    let wide = refine(img, b);
    let grew = wide[0] < b[0] - 0.5 || wide[1] < b[1] - 0.5 || wide[2] > b[2] + 0.5 || wide[3] > b[3] + 0.5;
    let inner = r[2] - r[0] > 1.5 || r[3] - r[1] > 1.5;
    (grew || inner).then(|| [wide[0].min(r[0]), wide[1].min(r[1]), wide[2].max(r[2]), wide[3].max(r[3])])
}

/// Grows a detected box so it covers the whole object: inside a window 1.6× larger, every pixel
/// that stands out from the window's border colour joins the box (the window caps the growth).
pub fn refine(img: &image::RgbaImage, b: [f32; 4]) -> [f32; 4] {
    let (w, h) = img.dimensions();
    let (bw, bh) = (b[2] - b[0], b[3] - b[1]);
    let wx0 = (b[0] - bw * 0.6).max(0.0) as u32;
    let wy0 = (b[1] - bh * 0.6).max(0.0) as u32;
    let wx1 = ((b[2] + bw * 0.6) as u32).min(w);
    let wy1 = ((b[3] + bh * 0.6) as u32).min(h);
    if wx1 <= wx0 + 2 || wy1 <= wy0 + 2 {
        return b;
    }
    let (_, bg) = studio_ocr::colors(img, [wx0 as f32, wy0 as f32, (wx1 - wx0) as f32, (wy1 - wy0) as f32]);
    let (mut x0, mut y0, mut x1, mut y1) = (b[0], b[1], b[2], b[3]);
    for y in wy0..wy1 {
        for x in wx0..wx1 {
            let p = img.get_pixel(x, y).0;
            if (0..3).map(|k| (p[k] as i32 - bg[k] as i32).abs()).sum::<i32>() > 70 {
                x0 = x0.min(x as f32);
                y0 = y0.min(y as f32);
                x1 = x1.max(x as f32 + 1.0);
                y1 = y1.max(y as f32 + 1.0);
            }
        }
    }
    [x0, y0, x1, y1]
}

fn stamp() -> String {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis()).to_string()
}

fn stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or("résultat".into())
}

fn remove_ops(rects: &[[f32; 4]], pad_k: f32, pad_px: f32, w: u32, h: u32) -> Vec<Value> {
    rects
        .iter()
        .map(|r| {
            let (pw, ph) = ((r[2] - r[0]) * pad_k + pad_px, (r[3] - r[1]) * pad_k + pad_px);
            let x0 = (r[0] - pw).max(0.0);
            let y0 = (r[1] - ph).max(0.0);
            let x1 = (r[2] + pw).min(w as f32);
            let y1 = (r[3] + ph).min(h as f32);
            json!({ "op": "remove", "x": x0.round(), "y": y0.round(), "width": (x1 - x0).round().max(1.0), "height": (y1 - y0).round().max(1.0) })
        })
        .collect()
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn logo_shapes(name: &str) -> Value {
    let h = name.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    let palettes = [([255, 122, 60], [255, 59, 107]), ([60, 160, 255], [120, 80, 255]), ([40, 210, 150], [20, 140, 200]), ([255, 196, 0], [255, 110, 40]), ([190, 90, 255], [255, 80, 160])];
    let (a, b) = palettes[(h % palettes.len() as u32) as usize];
    let initials: String = name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase();
    let size = if initials.chars().count() > 1 { 230 } else { 300 };
    let text_size = (760.0 / (name.chars().count() as f32 * 0.6).max(1.0)).min(110.0);
    json!([
        { "type": "rect", "x": 0, "y": 0, "w": 1000, "h": 1000, "fill": "#0f1117", "stroke": null },
        { "type": "ellipse", "cx": 500, "cy": 400, "rx": 250, "ry": 250, "fill": hex(a), "gradient": hex(b), "gradient_angle": 45, "shadow": true, "stroke": null },
        { "type": "ellipse", "cx": 500, "cy": 400, "rx": 205, "ry": 205, "fill": null, "stroke": "#ffffff", "stroke_width": 6, "opacity": 0.35 },
        { "type": "text", "x": 500.0 - initials.chars().count() as f32 * size as f32 * 0.33, "y": 400.0 + size as f32 * 0.36, "text": initials, "size": size, "bold": true, "fill": "#ffffff", "stroke": null },
        { "type": "text", "x": 500.0 - name.chars().count() as f32 * text_size * 0.29, "y": 820, "text": name, "size": text_size, "bold": true, "fill": "#ffffff", "stroke": null }
    ])
}

fn title_comp(text: &str) -> Value {
    let size = (1100.0 / (text.chars().count() as f32 * 0.55).max(1.0)).min(120.0);
    json!({ "width": 1280, "height": 720, "fps": 25, "duration": 3.0, "background": [12, 13, 20, 255], "layers": [
        { "name": "Halo", "start": 0.0, "end": 3.0, "visible": true, "content": { "Ellipse": { "w": 900, "h": 900, "color": [124, 92, 255, 60] } },
          "props": [ { "base": 640, "keys": [] }, { "base": 360, "keys": [] }, { "base": 100, "keys": [{ "t": 0, "v": 40, "ease": "Out" }, { "t": 1.2, "v": 100, "ease": "Smooth" }] }, { "base": 0, "keys": [] }, { "base": 100, "keys": [] } ] },
        { "name": "Titre", "start": 0.0, "end": 3.0, "visible": true, "content": { "Text": { "text": text, "size": size, "color": [255, 255, 255, 255], "bold": true, "family": "Segoe UI" } },
          "props": [ { "base": 640, "keys": [] }, { "base": 360, "keys": [] }, { "base": 100, "keys": [{ "t": 0.1, "v": 60, "ease": "Back" }, { "t": 0.9, "v": 100, "ease": "Smooth" }] }, { "base": 0, "keys": [] }, { "base": 100, "keys": [{ "t": 0.1, "v": 0, "ease": "Smooth" }, { "t": 0.7, "v": 100, "ease": "Smooth" }] } ] },
        { "name": "Ligne", "start": 0.0, "end": 3.0, "visible": true, "content": { "Line": { "w": 420, "thickness": 6, "color": [124, 92, 255, 255] } },
          "props": [ { "base": 640, "keys": [] }, { "base": 360.0 + size * 0.75, "keys": [] }, { "base": 100, "keys": [{ "t": 0.6, "v": 0, "ease": "Out" }, { "t": 1.4, "v": 100, "ease": "Smooth" }] }, { "base": 0, "keys": [] }, { "base": 100, "keys": [] } ] }
    ]})
}

fn ken_burns(path: &Path, w: u32, h: u32) -> Value {
    let k = (960.0 / w.max(h) as f32).min(1.0);
    let (cw, ch) = ((w as f32 * k).round(), (h as f32 * k).round());
    json!({ "width": cw, "height": ch, "fps": 20, "duration": 4.0, "background": [0, 0, 0, 255], "layers": [
        { "name": stem(path), "start": 0.0, "end": 4.0, "visible": true, "content": { "Image": { "path": path.to_string_lossy(), "w": cw, "h": ch } },
          "props": [ { "base": cw / 2.0, "keys": [] }, { "base": ch / 2.0, "keys": [] }, { "base": 100, "keys": [{ "t": 0, "v": 100, "ease": "Linear" }, { "t": 4, "v": 118, "ease": "Linear" }] }, { "base": 0, "keys": [] }, { "base": 100, "keys": [] } ] }
    ]})
}

/// Runs the plan; `open` launches the target app with the result at the end.
pub fn run(plan: &Plan, file: Option<&Path>, open: bool, tx: &Sender<Msg>) -> Result<Outcome, String> {
    let say = |s: String| {
        let _ = tx.send(Msg::Step(s));
    };
    let work = studio_core::data_dir().join("auto").join(stamp());
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let mut cur: Option<PathBuf> = file.map(Path::to_path_buf);
    let mut open_file: Option<PathBuf> = None;
    let mut notes = Vec::new();
    let base = file.map(stem).unwrap_or("Ethone".into());
    for (k, step) in plan.steps.iter().enumerate() {
        let out_png = work.join(format!("{base} - étape {}.png", k + 1));
        match step {
            Step::Develop(settings) => {
                say("Développement de la photo…".into());
                let src = cur.clone().ok_or("Aucune image")?;
                mcp("lumen_develop", &json!({ "input": src, "output": out_png, "settings": settings }), &work)?;
                cur = Some(out_png);
            }
            Step::Pixel(ops) => {
                say("Retouche dans le moteur de Pixel…".into());
                let src = cur.clone().ok_or("Aucune image")?;
                let (w, h) = image::image_dimensions(&src).map_err(|e| e.to_string())?;
                let ops: Vec<Value> = ops
                    .iter()
                    .map(|o| match o["op"].as_str() {
                        Some("square") => {
                            let s = w.min(h);
                            json!({ "op": "crop", "x": (w - s) / 2, "y": (h - s) / 2, "width": s, "height": s })
                        }
                        Some("upscale") => json!({ "op": "resize", "width": w * o["factor"].as_u64().unwrap_or(2) as u32 }),
                        _ => o.clone(),
                    })
                    .collect();
                mcp("pixel_edit", &json!({ "input": src, "output": out_png, "operations": ops }), &work)?;
                cur = Some(out_png);
            }
            Step::Remove(what) => {
                let src = cur.clone().ok_or("Aucune image")?;
                let img = image::open(&src).map_err(|e| e.to_string())?;
                say(format!("Recherche de « {what} » avec l'IA de vision locale (1 à 2 min)…"));
                let found = detect(&img, what)?;
                if found.is_empty() {
                    return Err(format!("Je n'ai trouvé aucun « {what} » dans l'image"));
                }
                say(format!("{} élément(s) trouvé(s), suppression et reconstruction du fond…", found.len()));
                let rgba = img.to_rgba8();
                let found: Vec<[f32; 4]> = found.iter().map(|b| refine(&rgba, *b)).collect();
                let ops = remove_ops(&found, 0.06, 4.0, img.width(), img.height());
                mcp("pixel_edit", &json!({ "input": src, "output": out_png, "operations": ops }), &work)?;
                for pass in 1..=2 {
                    let done = image::open(&out_png).map_err(|e| e.to_string())?.to_rgba8();
                    let rest: Vec<[f32; 4]> = found.iter().filter_map(|b| leftover(&done, *b)).collect();
                    if rest.is_empty() {
                        break;
                    }
                    say(format!("Finition {pass} : {} reste(s) à nettoyer…", rest.len()));
                    let again = work.join(format!("{base} - finition {pass}.png"));
                    mcp("pixel_edit", &json!({ "input": out_png, "output": again, "operations": remove_ops(&rest, 0.08, 4.0, img.width(), img.height()) }), &work)?;
                    std::fs::rename(&again, &out_png).map_err(|e| e.to_string())?;
                }
                notes.push(format!("{} élément(s) « {what} » supprimé(s)", found.len()));
                cur = Some(out_png);
            }
            Step::EraseText => {
                let src = cur.clone().ok_or("Aucune image")?;
                say("Lecture du texte (OCR Windows)…".into());
                let img = image::open(&src).map_err(|e| e.to_string())?;
                let page = studio_ocr::recognize_with(&img.to_rgba8(), "", true)?;
                if page.lines.is_empty() {
                    return Err("Aucun texte trouvé dans l'image".into());
                }
                say(format!("{} ligne(s) trouvée(s), effacement…", page.lines.len()));
                let rects: Vec<[f32; 4]> = page.lines.iter().map(|l| [l.rect[0], l.rect[1], l.rect[0] + l.rect[2], l.rect[1] + l.rect[3]]).collect();
                mcp("pixel_edit", &json!({ "input": src, "output": out_png, "operations": remove_ops(&rects, 0.0, 4.0, img.width(), img.height()) }), &work)?;
                notes.push(format!("{} ligne(s) de texte effacée(s)", page.lines.len()));
                cur = Some(out_png);
            }
            Step::Sound(ops) => {
                say("Traitement du son…".into());
                let src = cur.clone().ok_or("Aucun son")?;
                let out = work.join(format!("{base} - traité.wav"));
                mcp("sound_process", &json!({ "input": src, "output": out, "operations": ops }), &work)?;
                cur = Some(out);
            }
            Step::ExtractAudio => {
                say("Extraction de la bande son…".into());
                let src = cur.clone().ok_or("Aucune vidéo")?;
                let out = work.join(format!("{base} - son.wav"));
                mcp("sound_process", &json!({ "input": src, "output": out, "operations": [] }), &work)?;
                cur = Some(out);
            }
            Step::Animate => {
                say("Animation de l'image…".into());
                let src = cur.clone().ok_or("Aucune image")?;
                let (w, h) = image::image_dimensions(&src).map_err(|e| e.to_string())?;
                let gif = work.join(format!("{base}.gif"));
                let proj = work.join(format!("{base}.emot"));
                mcp("motion_render", &json!({ "comp": ken_burns(&src, w, h), "output": gif, "project": proj, "gif_width": 720 }), &work)?;
                notes.push(format!("GIF : {}", gif.display()));
                cur = Some(gif);
                open_file = Some(proj);
            }
            Step::Logo(name) => {
                say(format!("Dessin du logo « {name} »…"));
                let out = work.join(format!("Logo {name}"));
                mcp("vector_draw", &json!({ "output": out, "width": 1000, "height": 1000, "background": null, "png_scale": 1, "shapes": logo_shapes(name) }), &work)?;
                cur = Some(out.with_extension("png"));
                open_file = Some(work.join(format!("Logo {name}.evec")));
            }
            Step::Title(text) => {
                say(format!("Animation du titre « {text} »…"));
                let gif = work.join("Titre animé.gif");
                let proj = work.join("Titre animé.emot");
                mcp("motion_render", &json!({ "comp": title_comp(text), "output": gif, "project": proj, "gif_width": 800 }), &work)?;
                cur = Some(gif);
                open_file = Some(proj);
            }
            Step::Tone(secs) => {
                say("Génération du son…".into());
                let out = work.join("Tonalité.wav");
                mcp("sound_process", &json!({ "output": out, "operations": [{ "op": "generate", "kind": "tone", "seconds": secs }, { "op": "fade_in", "seconds": 0.05 }, { "op": "fade_out", "seconds": 0.2 }, { "op": "normalize", "db": -3 }] }), &work)?;
                cur = Some(out);
            }
        }
    }
    let target = open_file.clone().or(cur.clone());
    let mut opened = String::new();
    if open {
        say(format!("Ouverture dans {}…", studio_core::handoff::name(plan.app)));
        let mut args: Vec<std::ffi::OsString> = plan.args.iter().map(Into::into).collect();
        if let Some(t) = &target {
            args.push(t.into());
        }
        opened = studio_core::handoff::open_with(plan.app, &args)?;
    }
    Ok(Outcome { file: cur, app: plan.app, opened, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(base64(b"Ethone"), "RXRob25l");
        assert_eq!(base64(b"ab"), "YWI=");
        let b = boxes("```json\n[{\"bbox_2d\": [150, 200, 260, 250], \"label\": \"bird\"},\n{\"bbox_2d\": [540,150,620,200], \"label\": \"");
        assert_eq!(b, vec![[150.0, 200.0, 260.0, 250.0], [540.0, 150.0, 620.0, 200.0]]);
        let ops = remove_ops(&[[10.0, 10.0, 20.0, 30.0]], 0.1, 2.0, 100, 100);
        assert_eq!(ops[0]["x"], 7.0);
        assert_eq!(ops[0]["height"], 28.0);
        assert!(logo_shapes("Nova Café").as_array().unwrap().len() == 5);
        let mut img = image::RgbaImage::from_pixel(100, 100, image::Rgba([90, 150, 230, 255]));
        for x in 40..70 {
            img.put_pixel(x, 50, image::Rgba([20, 20, 20, 255]));
        }
        assert_eq!(refine(&img, [45.0, 45.0, 60.0, 55.0]), [40.0, 45.0, 69.0, 55.0]);
    }

    #[test]
    #[ignore]
    fn remove_birds_end_to_end() {
        let src = std::env::var("AUTO_IMG").unwrap();
        let plan = crate::plan::plan(&std::env::var("AUTO_PROMPT").unwrap_or("retire les oiseaux".into()), Some(Path::new(&src))).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let r = run(&plan, Some(Path::new(&src)), false, &tx).unwrap();
        for m in rx.try_iter() {
            if let Msg::Step(s) = m {
                println!("étape : {s}");
            }
        }
        println!("{:?} {:?}", r.file, r.notes);
        let out = r.file.unwrap();
        std::fs::copy(&out, std::env::var("AUTO_OUT").unwrap()).unwrap();
    }
}

#[cfg(test)]
mod more {
    use super::*;

    #[test]
    #[ignore]
    fn text_logo_title() {
        let dir = std::env::var("AUTO_DIR").unwrap();
        let (tx, _rx) = std::sync::mpsc::channel();
        for (prompt, file, out) in [("efface le texte", Some(format!("{dir}/pt-before.png")), "auto-text.png"), ("un logo « Nova Café »", None, "auto-logo.png"), ("une intro animée « Bienvenue »", None, "auto-title.gif")] {
            let p = crate::plan::plan(prompt, file.as_deref().map(Path::new)).unwrap();
            let r = run(&p, file.as_deref().map(Path::new), false, &tx).unwrap();
            std::fs::copy(r.file.unwrap(), format!("{dir}/{out}")).unwrap();
            println!("{prompt} → {} {:?}", p.app, r.notes);
        }
    }
}
