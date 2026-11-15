use image::RgbaImage;
use lopdf::content::{Content, Operation};
use lopdf::{Bookmark, Dictionary, Document, Object, ObjectId, Stream, StringFormat, dictionary};
use std::collections::HashSet;

pub type R<T> = Result<T, String>;

fn e<E: std::fmt::Display>(x: E) -> String {
    x.to_string()
}

pub fn load(bytes: &[u8], password: &str) -> R<Document> {
    let doc = if password.is_empty() { Document::load_mem(bytes) } else { Document::load_mem_with_options(bytes, lopdf::LoadOptions::with_password(password)) }.map_err(e)?;
    if doc.is_encrypted() {
        return Err("Ce PDF est protégé par un mot de passe".into());
    }
    Ok(doc)
}

pub fn save(doc: &mut Document) -> R<Vec<u8>> {
    let mut v = Vec::new();
    doc.save_to(&mut v).map_err(e)?;
    Ok(v)
}

pub fn pages(doc: &Document) -> Vec<ObjectId> {
    doc.get_pages().into_values().collect()
}

fn page_id(doc: &Document, i: usize) -> R<ObjectId> {
    pages(doc).get(i).copied().ok_or_else(|| format!("Page {} introuvable", i + 1))
}

pub fn num(o: &Object) -> f32 {
    match o {
        Object::Integer(i) => *i as f32,
        Object::Real(r) => *r,
        _ => 0.0,
    }
}

fn inherited(doc: &Document, page: ObjectId, key: &[u8]) -> Option<Object> {
    let mut cur = page;
    let mut seen = HashSet::new();
    while seen.insert(cur) {
        let node = doc.get_dictionary(cur).ok()?;
        if let Ok(v) = node.get(key)
            && !matches!(v, Object::Null)
        {
            return Some(v.clone());
        }
        cur = node.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

fn materialize(doc: &mut Document, page: ObjectId) {
    let mb = inherited(doc, page, b"MediaBox").unwrap_or_else(|| vec![0.into(), 0.into(), 612.into(), 792.into()].into());
    let cb = inherited(doc, page, b"CropBox");
    let res = inherited(doc, page, b"Resources").unwrap_or_else(|| Object::Dictionary(Dictionary::new()));
    let rot = inherited(doc, page, b"Rotate").unwrap_or(Object::Integer(0));
    if let Ok(d) = doc.get_dictionary_mut(page) {
        d.set("MediaBox", mb);
        if let Some(cb) = cb {
            d.set("CropBox", cb);
        }
        d.set("Resources", res);
        d.set("Rotate", rot);
    }
}

fn boxed(doc: &Document, o: Option<Object>) -> Option<[f32; 4]> {
    let o = o?;
    let a = match &o {
        Object::Reference(r) => doc.get_object(*r).ok()?.as_array().ok()?.clone(),
        o => o.as_array().ok()?.clone(),
    };
    if a.len() < 4 {
        return None;
    }
    let v: Vec<f32> = a.iter().map(num).collect();
    Some([v[0].min(v[2]), v[1].min(v[3]), v[0].max(v[2]), v[1].max(v[3])])
}

/// Visible box (crop box, else media box) and rotation of a page.
pub fn geometry(doc: &Document, page: ObjectId) -> ([f32; 4], i64) {
    let mb = boxed(doc, inherited(doc, page, b"MediaBox")).unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let cb = boxed(doc, inherited(doc, page, b"CropBox")).map_or(mb, |c| [c[0].max(mb[0]), c[1].max(mb[1]), c[2].min(mb[2]), c[3].min(mb[3])]);
    let rot = inherited(doc, page, b"Rotate").and_then(|o| o.as_i64().ok()).unwrap_or(0).rem_euclid(360) / 90 * 90;
    (cb, rot)
}

/// Displayed size in points (rotation applied).
pub fn view_size(b: [f32; 4], rot: i64) -> (f32, f32) {
    let (w, h) = (b[2] - b[0], b[3] - b[1]);
    if rot % 180 == 90 { (h, w) } else { (w, h) }
}

/// Converts a point of the displayed page (top-left origin, points) to PDF user space.
pub fn to_pdf(b: [f32; 4], rot: i64, p: [f32; 2]) -> [f32; 2] {
    let (w, h) = (b[2] - b[0], b[3] - b[1]);
    let (x, y) = (p[0], p[1]);
    let (u, v) = match rot {
        90 => (y, h - x),
        180 => (w - x, h - y),
        270 => (w - y, x),
        _ => (x, y),
    };
    [b[0] + u, b[3] - v]
}

fn rect_pdf(b: [f32; 4], rot: i64, a: [f32; 2], c: [f32; 2]) -> [f32; 4] {
    let (p, q) = (to_pdf(b, rot, a), to_pdf(b, rot, c));
    [p[0].min(q[0]), p[1].min(q[1]), p[0].max(q[0]), p[1].max(q[1])]
}

fn rebuild(doc: &mut Document, kids: Vec<ObjectId>) -> R<()> {
    let root = doc.trailer.get(b"Root").and_then(|r| r.as_reference()).map_err(e)?;
    let pages_id = doc.new_object_id();
    for k in &kids {
        if let Ok(d) = doc.get_dictionary_mut(*k) {
            d.set("Parent", pages_id);
        }
    }
    let n = kids.len() as i64;
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids.into_iter().map(Object::Reference).collect::<Vec<_>>(), "Count" => n }));
    doc.get_dictionary_mut(root).map_err(e)?.set("Pages", pages_id);
    doc.prune_objects();
    Ok(())
}

/// Rebuilds the page list from indices of the current pages; a repeated index duplicates the page.
pub fn arrange(doc: &mut Document, order: &[usize]) -> R<()> {
    let all = pages(doc);
    for p in &all {
        materialize(doc, *p);
    }
    let mut used = HashSet::new();
    let mut kids = Vec::new();
    for &i in order {
        let id = *all.get(i).ok_or("Page inexistante")?;
        if used.insert(id) {
            kids.push(id);
        } else {
            let mut d = doc.get_dictionary(id).map_err(e)?.clone();
            d.remove(b"Annots");
            kids.push(doc.add_object(d));
        }
    }
    if kids.is_empty() {
        return Err("Un PDF doit garder au moins une page".into());
    }
    rebuild(doc, kids)
}

pub fn rotate(doc: &mut Document, idx: &[usize], deg: i64) -> R<()> {
    for &i in idx {
        let id = page_id(doc, i)?;
        materialize(doc, id);
        let d = doc.get_dictionary_mut(id).map_err(e)?;
        let r = d.get(b"Rotate").and_then(|o| o.as_i64()).unwrap_or(0);
        d.set("Rotate", (r + deg).rem_euclid(360));
    }
    Ok(())
}

pub fn delete(doc: &mut Document, idx: &[usize]) -> R<()> {
    let n = pages(doc).len();
    let keep: Vec<usize> = (0..n).filter(|i| !idx.contains(i)).collect();
    arrange(doc, &keep)
}

pub fn insert_blank(doc: &mut Document, at: usize, w: f32, h: f32) -> R<()> {
    let all = pages(doc);
    for p in &all {
        materialize(doc, *p);
    }
    let c = doc.add_object(Stream::new(dictionary! {}, Vec::new()));
    let id = doc.add_object(dictionary! { "Type" => "Page", "MediaBox" => vec![0.into(), 0.into(), w.into(), h.into()], "Resources" => dictionary! {}, "Contents" => c });
    let mut kids = all;
    kids.insert(at.min(kids.len()), id);
    rebuild(doc, kids)
}

pub fn merge(mut docs: Vec<Document>) -> R<Document> {
    if docs.is_empty() {
        return Err("Aucun fichier".into());
    }
    let mut base = docs.remove(0);
    let mut kids = pages(&base);
    for p in &kids {
        materialize(&mut base, *p);
    }
    let mut max = base.max_id + 1;
    for mut d in docs {
        for p in pages(&d) {
            materialize(&mut d, p);
        }
        d.renumber_objects_with(max);
        max = d.max_id + 1;
        kids.extend(pages(&d));
        for (id, o) in d.objects {
            if !matches!(o.type_name().unwrap_or(b""), b"Catalog" | b"Pages" | b"Outlines") {
                base.objects.insert(id, o);
            }
        }
    }
    base.max_id = max;
    rebuild(&mut base, kids)?;
    Ok(base)
}

pub fn insert_file(doc: &mut Document, at: usize, other: Document) -> R<()> {
    let n = pages(doc).len();
    let m = pages(&other).len();
    let mut merged = merge(vec![std::mem::take(doc), other])?;
    let order: Vec<usize> = (0..at.min(n)).chain(n..n + m).chain(at.min(n)..n).collect();
    arrange(&mut merged, &order)?;
    *doc = merged;
    Ok(())
}

pub fn extract(doc: &Document, idx: &[usize]) -> R<Vec<u8>> {
    let mut d = doc.clone();
    arrange(&mut d, idx)?;
    save(&mut d)
}

pub fn split(doc: &Document, every: usize) -> R<Vec<Vec<u8>>> {
    let n = pages(doc).len();
    let every = every.max(1);
    (0..n).step_by(every).map(|s| extract(doc, &(s..(s + every).min(n)).collect::<Vec<_>>())).collect()
}

/// Margins in points: left, top, right, bottom of the displayed page.
pub fn crop(doc: &mut Document, idx: &[usize], m: [f32; 4]) -> R<()> {
    for &i in idx {
        let id = page_id(doc, i)?;
        materialize(doc, id);
        let (b, rot) = geometry(doc, id);
        let (w, h) = view_size(b, rot);
        let r = rect_pdf(b, rot, [m[0], m[1]], [w - m[2], h - m[3]]);
        if r[2] - r[0] < 10.0 || r[3] - r[1] < 10.0 {
            return Err("Marges trop grandes".into());
        }
        doc.get_dictionary_mut(id).map_err(e)?.set("CropBox", r.iter().map(|v| Object::Real(*v)).collect::<Vec<_>>());
    }
    Ok(())
}

pub fn text_obj(s: &str) -> Object {
    if s.is_ascii() {
        return Object::String(s.as_bytes().to_vec(), StringFormat::Literal);
    }
    let mut b = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        b.extend(u.to_be_bytes());
    }
    Object::String(b, StringFormat::Hexadecimal)
}

pub fn decode(o: &Object) -> String {
    match o {
        Object::String(b, _) if b.starts_with(&[0xFE, 0xFF]) => String::from_utf16_lossy(&b[2..].chunks(2).map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)])).collect::<Vec<_>>()),
        Object::String(b, _) => b.iter().map(|c| *c as char).collect(),
        Object::Name(n) => String::from_utf8_lossy(n).into_owned(),
        _ => String::new(),
    }
}

pub const META: [(&str, &str); 6] = [("Title", "Titre"), ("Author", "Auteur"), ("Subject", "Sujet"), ("Keywords", "Mots-clés"), ("Creator", "Application"), ("Producer", "Producteur")];

pub fn info(doc: &Document) -> Vec<String> {
    let d = doc.trailer.get(b"Info").ok().and_then(|o| match o {
        Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
        Object::Dictionary(d) => Some(d.clone()),
        _ => None,
    });
    META.iter().map(|(k, _)| d.as_ref().and_then(|d| d.get(k.as_bytes()).ok()).map(decode).unwrap_or_default()).collect()
}

pub fn set_info(doc: &mut Document, values: &[String]) {
    let mut d = Dictionary::new();
    for ((k, _), v) in META.iter().zip(values) {
        if !v.trim().is_empty() {
            d.set(*k, text_obj(v.trim()));
        }
    }
    let id = doc.add_object(d);
    doc.trailer.set("Info", id);
}

pub fn encrypt(doc: &mut Document, user: &str, owner: &str, print: bool, copy: bool, modify: bool) -> R<()> {
    use lopdf::encryption::crypt_filters::{Aes256CryptFilter, CryptFilter};
    use lopdf::{EncryptionState, EncryptionVersion, Permissions};
    use std::hash::{BuildHasher, Hasher};
    let mut key = [0u8; 32];
    for (k, chunk) in key.chunks_mut(8).enumerate() {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_usize(k);
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        chunk.copy_from_slice(&h.finish().to_le_bytes());
    }
    let mut perms = Permissions::COPYABLE_FOR_ACCESSIBILITY;
    if print {
        perms |= Permissions::PRINTABLE | Permissions::PRINTABLE_IN_HIGH_QUALITY;
    }
    if copy {
        perms |= Permissions::COPYABLE;
    }
    if modify {
        perms |= Permissions::MODIFIABLE | Permissions::ANNOTABLE | Permissions::FILLABLE | Permissions::ASSEMBLABLE;
    }
    let owner = if owner.is_empty() { user } else { owner };
    let filter: std::sync::Arc<dyn CryptFilter> = std::sync::Arc::new(Aes256CryptFilter);
    let v = EncryptionVersion::V5 {
        encrypt_metadata: true,
        crypt_filters: std::collections::BTreeMap::from([(b"StdCF".to_vec(), filter)]),
        file_encryption_key: &key,
        stream_filter: b"StdCF".to_vec(),
        string_filter: b"StdCF".to_vec(),
        owner_password: owner,
        user_password: user,
        permissions: perms,
    };
    let state = EncryptionState::try_from(v).map_err(e)?;
    doc.encrypt(&state).map_err(e)
}

fn winansi(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c {
            '€' => 0x80,
            '’' => 0x92,
            '‘' => 0x91,
            '“' => 0x93,
            '”' => 0x94,
            '–' => 0x96,
            '—' => 0x97,
            '•' => 0x95,
            '…' => 0x85,
            'œ' => 0x9C,
            'Œ' => 0x8C,
            c if (c as u32) < 256 => c as u8,
            _ => b'?',
        })
        .collect()
}

fn helvetica(bold: bool) -> Dictionary {
    dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => if bold { "Helvetica-Bold" } else { "Helvetica" }, "Encoding" => "WinAnsiEncoding" }
}

/// Approximate Helvetica advance width (in em) of a string.
pub fn text_em(s: &str) -> f32 {
    s.chars()
        .map(|c| match c {
            'i' | 'j' | 'l' | '\'' | '.' | ',' | ':' | ';' | '!' | '|' | 'I' => 0.24,
            'f' | 't' | 'r' | ' ' | '(' | ')' | '-' => 0.32,
            'm' | 'w' | 'M' | 'W' => 0.85,
            c if c.is_uppercase() || c.is_ascii_digit() => 0.66,
            _ => 0.54,
        })
        .sum()
}

fn op(name: &str, args: Vec<Object>) -> Operation {
    Operation::new(name, args)
}

fn f(v: f32) -> Object {
    Object::Real(v)
}

fn rgb(c: [u8; 3]) -> Vec<Object> {
    c.iter().map(|v| f(*v as f32 / 255.0)).collect()
}

fn encode(ops: Vec<Operation>) -> R<Vec<u8>> {
    Content { operations: ops }.encode().map_err(e)
}

fn res_entry(doc: &mut Document, page: ObjectId, cat: &str, name: &str, value: Object) -> R<()> {
    let res = doc.get_dictionary(page).map_err(e)?.get(b"Resources").ok().cloned();
    let mut d = match &res {
        Some(Object::Reference(r)) => doc.get_dictionary(*r).map_err(e)?.clone(),
        Some(Object::Dictionary(d)) => d.clone(),
        _ => Dictionary::new(),
    };
    let mut sub = match d.get(cat.as_bytes()) {
        Ok(Object::Reference(r)) => doc.get_dictionary(*r).map_err(e)?.clone(),
        Ok(Object::Dictionary(s)) => s.clone(),
        _ => Dictionary::new(),
    };
    sub.set(name, value);
    d.set(cat, sub);
    match res {
        Some(Object::Reference(r)) => {
            doc.objects.insert(r, Object::Dictionary(d));
        }
        _ => doc.get_dictionary_mut(page).map_err(e)?.set("Resources", d),
    }
    Ok(())
}

fn contents(doc: &Document, page: ObjectId) -> Vec<Object> {
    match doc.get_dictionary(page).ok().and_then(|d| d.get(b"Contents").ok()) {
        Some(Object::Array(a)) => a.clone(),
        Some(Object::Reference(r)) => match doc.get_object(*r) {
            Ok(Object::Array(a)) => a.clone(),
            _ => vec![Object::Reference(*r)],
        },
        _ => Vec::new(),
    }
}

/// Draws extra operations above (or under) a page's existing content.
fn overlay(doc: &mut Document, page: ObjectId, ops: Vec<Operation>, under: bool) -> R<()> {
    let old = contents(doc, page);
    let mut wrapped = vec![op("q", vec![])];
    wrapped.extend(ops);
    wrapped.push(op("Q", vec![]));
    let ours = Object::Reference(doc.add_object(Stream::new(dictionary! {}, encode(wrapped)?)));
    let list = if under {
        std::iter::once(ours).chain(old).collect::<Vec<_>>()
    } else {
        let q = Object::Reference(doc.add_object(Stream::new(dictionary! {}, b"q\n".to_vec())));
        let qq = Object::Reference(doc.add_object(Stream::new(dictionary! {}, b"\nQ\n".to_vec())));
        std::iter::once(q).chain(old).chain([qq, ours]).collect()
    };
    doc.get_dictionary_mut(page).map_err(e)?.set("Contents", list);
    Ok(())
}

fn prepare_text(doc: &mut Document, page: ObjectId, opacity: f32) -> R<()> {
    materialize(doc, page);
    res_entry(doc, page, "Font", "FoH", Object::Dictionary(helvetica(false)))?;
    res_entry(doc, page, "Font", "FoB", Object::Dictionary(helvetica(true)))?;
    res_entry(doc, page, "ExtGState", "FoG", Object::Dictionary(dictionary! { "Type" => "ExtGState", "CA" => opacity, "ca" => opacity }))
}

#[derive(Clone)]
pub struct Stamp {
    pub text: String,
    pub size: f32,
    pub color: [u8; 3],
    pub opacity: f32,
    pub angle: f32,
    pub bold: bool,
    /// 0..9 : 0 = centre, 1..3 haut (g, c, d), 4..6 bas (g, c, d)
    pub place: u8,
    pub under: bool,
}

pub fn expand(t: &str, page: usize, total: usize, name: &str) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let days = now / 86400;
    let (y, m, d) = civil(days);
    t.replace("{page}", &(page + 1).to_string()).replace("{pages}", &total.to_string()).replace("{date}", &format!("{d:02}/{m:02}/{y}")).replace("{fichier}", name)
}

fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Writes a text on each listed page (watermark, header, footer, page numbers, Bates numbers).
pub fn stamp_text(doc: &mut Document, idx: &[usize], s: &Stamp, name: &str, bates: Option<(u32, usize)>) -> R<()> {
    let total = pages(doc).len();
    for (k, &i) in idx.iter().enumerate() {
        let id = page_id(doc, i)?;
        prepare_text(doc, id, s.opacity)?;
        let (b, rot) = geometry(doc, id);
        let (w, h) = view_size(b, rot);
        let txt = match bates {
            Some((start, digits)) => format!("{}{:0digits$}", s.text, start as usize + k),
            None => expand(&s.text, i, total, name),
        };
        let tw = text_em(&txt) * s.size;
        let m = 28.0;
        let (vx, vy) = match s.place {
            1 => (m, m + s.size),
            2 => ((w - tw) / 2.0, m + s.size),
            3 => (w - m - tw, m + s.size),
            4 => (m, h - m),
            5 => ((w - tw) / 2.0, h - m),
            6 => (w - m - tw, h - m),
            _ => (w / 2.0, h / 2.0),
        };
        let page_rot = -(rot as f32).to_radians();
        let ang = s.angle.to_radians() + page_rot;
        let p = to_pdf(b, rot, [vx, vy]);
        let (sn, cs) = ang.sin_cos();
        let centered = s.place == 0;
        let shift = if centered { -tw / 2.0 } else { 0.0 };
        let base = [p[0] + cs * shift + sn * s.size * 0.35, p[1] + sn * shift - cs * s.size * 0.35];
        let ops = vec![
            op("gs", vec!["FoG".into()]),
            op("rg", rgb(s.color)),
            op("BT", vec![]),
            op("Tf", vec![if s.bold { "FoB" } else { "FoH" }.into(), f(s.size)]),
            op("Tm", vec![f(cs), f(sn), f(-sn), f(cs), f(base[0]), f(base[1])]),
            op("Tj", vec![Object::String(winansi(&txt), StringFormat::Literal)]),
            op("ET", vec![]),
        ];
        overlay(doc, id, ops, s.under)?;
    }
    Ok(())
}

pub fn background(doc: &mut Document, idx: &[usize], color: [u8; 3]) -> R<()> {
    for &i in idx {
        let id = page_id(doc, i)?;
        materialize(doc, id);
        let (b, _) = geometry(doc, id);
        overlay(doc, id, vec![op("rg", rgb(color)), op("re", vec![f(b[0]), f(b[1]), f(b[2] - b[0]), f(b[3] - b[1])]), op("f", vec![])], true)?;
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum MarkKind {
    Highlight,
    Underline,
    Strike,
    Note,
    TextBox,
    Rect,
    Ellipse,
    Line,
    Arrow,
    Ink,
    Stamp,
}

/// A comment in displayed-page coordinates (points, top-left origin).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Mark {
    pub kind: MarkKind,
    pub pts: Vec<[f32; 2]>,
    pub strokes: Vec<Vec<[f32; 2]>>,
    pub color: [u8; 3],
    pub width: f32,
    pub text: String,
    pub size: f32,
}

fn ellipse_ops(r: [f32; 4]) -> Vec<Operation> {
    let (cx, cy, rx, ry) = ((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0, (r[2] - r[0]) / 2.0, (r[3] - r[1]) / 2.0);
    let k = 0.5523;
    vec![
        op("m", vec![f(cx + rx), f(cy)]),
        op("c", vec![f(cx + rx), f(cy + ry * k), f(cx + rx * k), f(cy + ry), f(cx), f(cy + ry)]),
        op("c", vec![f(cx - rx * k), f(cy + ry), f(cx - rx), f(cy + ry * k), f(cx - rx), f(cy)]),
        op("c", vec![f(cx - rx), f(cy - ry * k), f(cx - rx * k), f(cy - ry), f(cx), f(cy - ry)]),
        op("c", vec![f(cx + rx * k), f(cy - ry), f(cx + rx), f(cy - ry * k), f(cx + rx), f(cy)]),
    ]
}

fn wrap(text: &str, size: f32, width: f32) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.lines() {
        let mut line = String::new();
        for w in para.split_whitespace() {
            let cand = if line.is_empty() { w.to_string() } else { format!("{line} {w}") };
            if text_em(&cand) * size > width && !line.is_empty() {
                out.push(std::mem::replace(&mut line, w.to_string()));
            } else {
                line = cand;
            }
        }
        out.push(line);
    }
    out
}

pub fn add_mark(doc: &mut Document, page: usize, m: &Mark) -> R<()> {
    let id = page_id(doc, page)?;
    materialize(doc, id);
    let (b, rot) = geometry(doc, id);
    let pts: Vec<[f32; 2]> = m.pts.iter().map(|p| to_pdf(b, rot, *p)).collect();
    let strokes: Vec<Vec<[f32; 2]>> = m.strokes.iter().map(|s| s.iter().map(|p| to_pdf(b, rot, *p)).collect()).collect();
    let all: Vec<[f32; 2]> = pts.iter().copied().chain(strokes.iter().flatten().copied()).collect();
    if all.is_empty() {
        return Err("Annotation vide".into());
    }
    let pad = m.width.max(1.0) * 2.0 + if matches!(m.kind, MarkKind::Arrow) { 12.0 } else { 0.0 };
    let mut r = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for p in &all {
        r = [r[0].min(p[0]), r[1].min(p[1]), r[2].max(p[0]), r[3].max(p[1])];
    }
    if m.kind == MarkKind::Note {
        r = [r[0], r[3] - 22.0, r[0] + 22.0, r[3]];
    }
    let inner = r;
    let r = [r[0] - pad, r[1] - pad, r[2] + pad, r[3] + pad];
    let col = rgb(m.color);
    let mut ops = vec![op("RG", col.clone()), op("rg", col.clone()), op("w", vec![f(m.width)]), op("J", vec![1.into()]), op("j", vec![1.into()])];
    let mut res = Dictionary::new();
    let mut extra = Dictionary::new();
    let gs = |a: f32, mult: bool| {
        let mut g = dictionary! { "Type" => "ExtGState", "CA" => a, "ca" => a };
        if mult {
            g.set("BM", "Multiply");
        }
        dictionary! { "G0" => g }
    };
    let quad = |r: [f32; 4]| -> Vec<Object> { [r[0], r[3], r[2], r[3], r[0], r[1], r[2], r[1]].iter().map(|v| f(*v)).collect() };
    let subtype = match m.kind {
        MarkKind::Highlight => {
            res.set("ExtGState", gs(0.4, true));
            ops.extend([op("gs", vec!["G0".into()]), op("re", vec![f(inner[0]), f(inner[1]), f(inner[2] - inner[0]), f(inner[3] - inner[1])]), op("f", vec![])]);
            extra.set("QuadPoints", quad(inner));
            "Highlight"
        }
        MarkKind::Underline | MarkKind::Strike => {
            let y = if m.kind == MarkKind::Underline { inner[1] + 1.0 } else { (inner[1] + inner[3]) / 2.0 };
            ops.extend([op("w", vec![f(((inner[3] - inner[1]) * 0.08).max(1.0))]), op("m", vec![f(inner[0]), f(y)]), op("l", vec![f(inner[2]), f(y)]), op("S", vec![])]);
            extra.set("QuadPoints", quad(inner));
            if m.kind == MarkKind::Underline { "Underline" } else { "StrikeOut" }
        }
        MarkKind::Rect => {
            ops.extend([op("re", vec![f(inner[0]), f(inner[1]), f(inner[2] - inner[0]), f(inner[3] - inner[1])]), op("S", vec![])]);
            "Square"
        }
        MarkKind::Ellipse => {
            ops.extend(ellipse_ops(inner));
            ops.push(op("S", vec![]));
            "Circle"
        }
        MarkKind::Line | MarkKind::Arrow => {
            let (a, c) = (pts[0], *pts.last().unwrap_or(&pts[0]));
            ops.extend([op("m", vec![f(a[0]), f(a[1])]), op("l", vec![f(c[0]), f(c[1])]), op("S", vec![])]);
            if m.kind == MarkKind::Arrow {
                let ang = (c[1] - a[1]).atan2(c[0] - a[0]);
                let s = 8.0 + m.width * 2.5;
                let p1 = [c[0] - s * (ang - 0.45).cos(), c[1] - s * (ang - 0.45).sin()];
                let p2 = [c[0] - s * (ang + 0.45).cos(), c[1] - s * (ang + 0.45).sin()];
                ops.extend([op("m", vec![f(c[0]), f(c[1])]), op("l", vec![f(p1[0]), f(p1[1])]), op("l", vec![f(p2[0]), f(p2[1])]), op("f", vec![])]);
                extra.set("LE", vec![Object::Name(b"None".to_vec()), Object::Name(b"OpenArrow".to_vec())]);
            }
            extra.set("L", vec![f(a[0]), f(a[1]), f(c[0]), f(c[1])]);
            "Line"
        }
        MarkKind::Ink => {
            let mut list = Vec::new();
            for s in &strokes {
                if let Some(p) = s.first() {
                    ops.push(op("m", vec![f(p[0]), f(p[1])]));
                    for q in s.iter().skip(1) {
                        ops.push(op("l", vec![f(q[0]), f(q[1])]));
                    }
                    if s.len() == 1 {
                        ops.push(op("l", vec![f(p[0] + 0.1), f(p[1])]));
                    }
                    ops.push(op("S", vec![]));
                    list.push(Object::Array(s.iter().flat_map(|q| [f(q[0]), f(q[1])]).collect()));
                }
            }
            extra.set("InkList", list);
            "Ink"
        }
        MarkKind::TextBox | MarkKind::Stamp => {
            res.set("Font", dictionary! { "F0" => helvetica(m.kind == MarkKind::Stamp) });
            let size = m.size.max(4.0);
            let lines = if m.kind == MarkKind::Stamp { vec![m.text.to_uppercase()] } else { wrap(&m.text, size, (inner[2] - inner[0] - 4.0).max(size * 4.0)) };
            if m.kind == MarkKind::Stamp {
                ops.extend([op("w", vec![f(2.5)]), op("re", vec![f(inner[0]), f(inner[1]), f(inner[2] - inner[0]), f(inner[3] - inner[1])]), op("S", vec![])]);
            }
            let tw = lines.iter().map(|l| text_em(l) * size).fold(0.0, f32::max);
            let x = if m.kind == MarkKind::Stamp { (inner[0] + inner[2] - tw) / 2.0 } else { inner[0] + 2.0 };
            let mut y = if m.kind == MarkKind::Stamp { (inner[1] + inner[3]) / 2.0 - size * 0.35 } else { inner[3] - size };
            ops.push(op("BT", vec![]));
            ops.push(op("Tf", vec!["F0".into(), f(size)]));
            for l in &lines {
                ops.push(op("Tm", vec![1.into(), 0.into(), 0.into(), 1.into(), f(x), f(y)]));
                ops.push(op("Tj", vec![Object::String(winansi(l), StringFormat::Literal)]));
                y -= size * 1.2;
            }
            ops.push(op("ET", vec![]));
            if m.kind == MarkKind::TextBox {
                extra.set("DA", Object::string_literal(format!("/Helv {size} Tf 0 g")));
                "FreeText"
            } else {
                extra.set("Name", "Approved");
                "Stamp"
            }
        }
        MarkKind::Note => {
            let x = inner;
            ops = vec![op("rg", vec![f(1.0), f(0.82), f(0.1)]), op("RG", vec![f(0.45), f(0.35), f(0.0)]), op("w", vec![f(1.0)]), op("re", vec![f(x[0]), f(x[1]), f(22.0), f(22.0)]), op("B", vec![])];
            for k in 0..3 {
                let yy = x[3] - 7.0 - k as f32 * 4.5;
                ops.extend([op("m", vec![f(x[0] + 5.0), f(yy)]), op("l", vec![f(x[2] - 5.0), f(yy)]), op("S", vec![])]);
            }
            extra.set("Name", "Comment");
            "Text"
        }
    };
    let ap = doc.add_object(Stream::new(dictionary! { "Type" => "XObject", "Subtype" => "Form", "BBox" => r.iter().map(|v| f(*v)).collect::<Vec<_>>(), "Resources" => res }, encode(ops)?));
    let mut a = dictionary! {
        "Type" => "Annot",
        "Subtype" => subtype,
        "Rect" => r.iter().map(|v| f(*v)).collect::<Vec<_>>(),
        "C" => col,
        "F" => 4,
        "P" => id,
        "T" => text_obj("Folio"),
        "AP" => dictionary! { "N" => ap },
        "BS" => dictionary! { "W" => m.width },
    };
    if !m.text.is_empty() {
        a.set("Contents", text_obj(&m.text));
    }
    for (k, v) in extra.into_iter() {
        a.set(k, v);
    }
    let aid = doc.add_object(a);
    push_annot(doc, id, aid)
}

fn push_annot(doc: &mut Document, page: ObjectId, aid: ObjectId) -> R<()> {
    let cur = doc.get_dictionary(page).map_err(e)?.get(b"Annots").ok().cloned();
    match cur {
        Some(Object::Reference(r)) => {
            if let Ok(Object::Array(a)) = doc.get_object_mut(r) {
                a.push(Object::Reference(aid));
            }
        }
        Some(Object::Array(mut a)) => {
            a.push(Object::Reference(aid));
            doc.get_dictionary_mut(page).map_err(e)?.set("Annots", a);
        }
        _ => doc.get_dictionary_mut(page).map_err(e)?.set("Annots", vec![Object::Reference(aid)]),
    }
    Ok(())
}

fn annots(doc: &Document, page: ObjectId) -> Vec<Object> {
    match doc.get_dictionary(page).ok().and_then(|d| d.get(b"Annots").ok()) {
        Some(Object::Array(a)) => a.clone(),
        Some(Object::Reference(r)) => doc.get_object(*r).ok().and_then(|o| o.as_array().ok().cloned()).unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Lists the comments of a page: (subtype, text, author).
pub fn comments(doc: &Document, page: usize) -> Vec<(String, String)> {
    let Ok(id) = page_id(doc, page) else { return Vec::new() };
    annots(doc, id)
        .iter()
        .filter_map(|o| {
            let d = match o {
                Object::Reference(r) => doc.get_dictionary(*r).ok()?,
                Object::Dictionary(d) => d,
                _ => return None,
            };
            let st = decode(d.get(b"Subtype").ok()?);
            if matches!(st.as_str(), "Link" | "Widget" | "Popup") {
                return None;
            }
            Some((st, d.get(b"Contents").map(decode).unwrap_or_default()))
        })
        .collect()
}

pub fn remove_comments(doc: &mut Document, idx: &[usize]) -> R<usize> {
    let mut n = 0;
    for &i in idx {
        let id = page_id(doc, i)?;
        let keep: Vec<Object> = annots(doc, id)
            .into_iter()
            .filter(|o| {
                let st = match o {
                    Object::Reference(r) => doc.get_dictionary(*r).ok().and_then(|d| d.get(b"Subtype").ok()).map(decode),
                    Object::Dictionary(d) => d.get(b"Subtype").ok().map(decode),
                    _ => None,
                };
                let k = matches!(st.as_deref(), Some("Link" | "Widget"));
                n += usize::from(!k);
                k
            })
            .collect();
        doc.get_dictionary_mut(id).map_err(e)?.set("Annots", keep);
    }
    Ok(n)
}

pub fn jpeg(img: &RgbaImage, quality: u8) -> R<Vec<u8>> {
    let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality).encode_image(&rgb).map_err(e)?;
    Ok(out)
}

fn image_page(doc: &mut Document, img: &RgbaImage, w: f32, h: f32, quality: u8) -> R<ObjectId> {
    let data = jpeg(img, quality)?;
    let xo = doc.add_object(Stream::new(dictionary! { "Type" => "XObject", "Subtype" => "Image", "Width" => img.width() as i64, "Height" => img.height() as i64, "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8, "Filter" => "DCTDecode" }, data));
    let c = doc.add_object(Stream::new(dictionary! {}, encode(vec![op("q", vec![]), op("cm", vec![f(w), 0.into(), 0.into(), f(h), 0.into(), 0.into()]), op("Do", vec!["Im0".into()]), op("Q", vec![])])?));
    Ok(doc.add_object(dictionary! { "Type" => "Page", "MediaBox" => vec![0.into(), 0.into(), f(w), f(h)], "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => xo } }, "Contents" => c }))
}

/// Builds a PDF from images; `page` = None keeps the image size at 96 ppp, else fits each image in that page size.
pub fn images_to_pdf(imgs: &[RgbaImage], page: Option<(f32, f32)>) -> R<Document> {
    let mut doc = Document::with_version("1.7");
    let root_pages = doc.new_object_id();
    let mut kids = Vec::new();
    for img in imgs {
        let (iw, ih) = (img.width() as f32 * 0.75, img.height() as f32 * 0.75);
        let (w, h) = match page {
            Some((pw, ph)) => {
                let (pw, ph) = if (iw > ih) != (pw > ph) { (ph, pw) } else { (pw, ph) };
                (pw, ph)
            }
            None => (iw, ih),
        };
        let id = image_page(&mut doc, img, iw, ih, 88)?;
        if page.is_some() {
            let k = ((w - 36.0) / iw).min((h - 36.0) / ih);
            let (dw, dh) = (iw * k, ih * k);
            let c = doc.add_object(Stream::new(dictionary! {}, encode(vec![op("q", vec![]), op("cm", vec![f(dw), 0.into(), 0.into(), f(dh), f((w - dw) / 2.0), f((h - dh) / 2.0)]), op("Do", vec!["Im0".into()]), op("Q", vec![])])?));
            let d = doc.get_dictionary_mut(id).map_err(e)?;
            d.set("Contents", c);
            d.set("MediaBox", vec![0.into(), 0.into(), f(w), f(h)]);
        }
        kids.push(id);
    }
    let n = kids.len() as i64;
    for k in &kids {
        doc.get_dictionary_mut(*k).map_err(e)?.set("Parent", root_pages);
    }
    doc.objects.insert(root_pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids.into_iter().map(Object::Reference).collect::<Vec<_>>(), "Count" => n }));
    let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => root_pages });
    doc.trailer.set("Root", cat);
    Ok(doc)
}

/// Replaces a page by a picture of it (used for true redaction and strong compression).
pub fn replace_with_image(doc: &mut Document, page: usize, img: &RgbaImage, quality: u8) -> R<()> {
    let id = page_id(doc, page)?;
    let (b, rot) = geometry(doc, id);
    let (w, h) = view_size(b, rot);
    let new = image_page(doc, img, w, h, quality)?;
    let mut d = doc.get_dictionary(new).map_err(e)?.clone();
    if let Ok(p) = doc.get_dictionary(id).map_err(e)?.get(b"Parent") {
        d.set("Parent", p.clone());
    }
    doc.objects.insert(id, Object::Dictionary(d));
    doc.objects.remove(&new);
    Ok(())
}

pub fn compress(doc: &mut Document) {
    doc.delete_zero_length_streams();
    doc.prune_objects();
    doc.compress();
}

pub fn page_text(doc: &Document, page: usize) -> String {
    doc.extract_text(&[page as u32 + 1]).unwrap_or_default()
}

/// Invisible text layer so a scanned page becomes searchable and copyable.
/// Words are (text, [x, y, w, h]) in displayed-page points.
pub fn add_text_layer(doc: &mut Document, page: usize, words: &[(String, [f32; 4])]) -> R<()> {
    let id = page_id(doc, page)?;
    prepare_text(doc, id, 1.0)?;
    let (b, rot) = geometry(doc, id);
    let mut ops = vec![op("BT", vec![]), op("Tr", vec![3.into()])];
    for (t, r) in words.iter().filter(|w| !w.0.trim().is_empty()) {
        let q = rect_pdf(b, rot, [r[0], r[1]], [r[0] + r[2], r[1] + r[3]]);
        let (w, h) = (q[2] - q[0], q[3] - q[1]);
        let size = h.max(1.0);
        let natural = text_em(t) * size;
        ops.push(op("Tf", vec!["FoH".into(), f(size)]));
        ops.push(op("Tz", vec![f((w / natural.max(0.1) * 100.0).clamp(10.0, 400.0))]));
        ops.push(op("Tm", vec![1.into(), 0.into(), 0.into(), 1.into(), f(q[0]), f(q[1] + h * 0.2)]));
        ops.push(op("Tj", vec![Object::String(winansi(&format!("{t} ")), StringFormat::Literal)]));
    }
    ops.push(op("ET", vec![]));
    overlay(doc, id, ops, false)
}

pub fn outline(doc: &Document) -> Vec<(usize, String, usize)> {
    doc.get_toc().map(|t| t.toc.into_iter().map(|x| (x.level, x.title, x.page.saturating_sub(1))).collect()).unwrap_or_default()
}

pub fn set_outline(doc: &mut Document, items: &[(usize, String, usize)]) -> R<()> {
    let all = pages(doc);
    doc.bookmarks.clear();
    doc.bookmark_table.clear();
    let mut parents: Vec<u32> = Vec::new();
    for (level, title, page) in items {
        let pid = *all.get(*page).ok_or("Signet vers une page inexistante")?;
        parents.truncate((*level).saturating_sub(1));
        let parent = parents.last().copied();
        let id = doc.add_bookmark(Bookmark::new(title.clone(), [0.0, 0.0, 0.0], 0, pid), parent);
        parents.push(id);
    }
    let root = doc.trailer.get(b"Root").and_then(|r| r.as_reference()).map_err(e)?;
    match doc.build_outline() {
        Some(o) => {
            let c = doc.get_dictionary_mut(root).map_err(e)?;
            c.set("Outlines", o);
            c.set("PageMode", "UseOutlines");
        }
        None => {
            doc.get_dictionary_mut(root).map_err(e)?.remove(b"Outlines");
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Field {
    pub id: ObjectId,
    pub name: String,
    pub value: String,
    pub check: Option<String>,
}

pub fn fields(doc: &Document) -> Vec<Field> {
    let mut out = Vec::new();
    let Some(form) = doc.catalog().ok().and_then(|c| c.get(b"AcroForm").ok()).and_then(|o| match o {
        Object::Reference(r) => doc.get_dictionary(*r).ok(),
        Object::Dictionary(d) => Some(d),
        _ => None,
    }) else {
        return out;
    };
    let mut stack: Vec<(ObjectId, String, String)> = form.get(b"Fields").and_then(|o| o.as_array()).map(|a| a.iter().filter_map(|o| o.as_reference().ok()).map(|r| (r, String::new(), String::new())).collect()).unwrap_or_default();
    let mut seen = HashSet::new();
    while let Some((id, prefix, ft)) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let Ok(d) = doc.get_dictionary(id) else { continue };
        let part = d.get(b"T").map(decode).unwrap_or_default();
        let name = match (prefix.is_empty(), part.is_empty()) {
            (_, true) => prefix.clone(),
            (true, false) => part,
            _ => format!("{prefix}.{part}"),
        };
        let ft = d.get(b"FT").map(decode).unwrap_or(ft);
        let kids: Vec<ObjectId> = d.get(b"Kids").and_then(|o| o.as_array()).map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect()).unwrap_or_default();
        let named_kids = kids.iter().any(|k| doc.get_dictionary(*k).is_ok_and(|kd| kd.has(b"T")));
        if named_kids {
            stack.extend(kids.into_iter().rev().map(|k| (k, name.clone(), ft.clone())));
            continue;
        }
        let value = d.get(b"V").map(decode).unwrap_or_default();
        match ft.as_str() {
            "Tx" | "Ch" => out.push(Field { id, name, value, check: None }),
            "Btn" => {
                let widget = kids.first().copied().unwrap_or(id);
                let on = doc
                    .get_dictionary(widget)
                    .ok()
                    .and_then(|w| w.get(b"AP").ok())
                    .and_then(|ap| match ap {
                        Object::Dictionary(d) => Some(d.clone()),
                        Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
                        _ => None,
                    })
                    .and_then(|ap| ap.get(b"N").ok().cloned())
                    .and_then(|n| match n {
                        Object::Dictionary(d) => Some(d),
                        Object::Reference(r) => doc.get_dictionary(r).ok().cloned(),
                        _ => None,
                    })
                    .and_then(|n| n.iter().map(|(k, _)| String::from_utf8_lossy(k).into_owned()).find(|k| k != "Off"));
                if let Some(on) = on {
                    out.push(Field { id, name, value, check: Some(on) });
                }
            }
            _ => {}
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn set_field(doc: &mut Document, field: &Field, value: &str) -> R<()> {
    let widgets: Vec<ObjectId> = {
        let d = doc.get_dictionary(field.id).map_err(e)?;
        let kids: Vec<ObjectId> = d.get(b"Kids").and_then(|o| o.as_array()).map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect()).unwrap_or_default();
        if kids.is_empty() { vec![field.id] } else { kids }
    };
    if let Some(on) = &field.check {
        let state = if value == "1" { on.as_str() } else { "Off" };
        doc.get_dictionary_mut(field.id).map_err(e)?.set("V", Object::Name(state.as_bytes().to_vec()));
        for w in widgets {
            doc.get_dictionary_mut(w).map_err(e)?.set("AS", Object::Name(state.as_bytes().to_vec()));
        }
        return Ok(());
    }
    doc.get_dictionary_mut(field.id).map_err(e)?.set("V", text_obj(value));
    for w in widgets {
        let rect = boxed(doc, doc.get_dictionary(w).map_err(e)?.get(b"Rect").ok().cloned()).unwrap_or([0.0, 0.0, 100.0, 20.0]);
        let (ww, hh) = (rect[2] - rect[0], rect[3] - rect[1]);
        let size = (hh * 0.65).clamp(6.0, 12.0);
        let ops = vec![
            op("BMC", vec!["Tx".into()]),
            op("q", vec![]),
            op("BT", vec![]),
            op("g", vec![0.into()]),
            op("Tf", vec!["Helv".into(), f(size)]),
            op("Td", vec![f(2.0), f((hh - size) / 2.0 + size * 0.2)]),
            op("Tj", vec![Object::String(winansi(value), StringFormat::Literal)]),
            op("ET", vec![]),
            op("Q", vec![]),
            op("EMC", vec![]),
        ];
        let ap = doc.add_object(Stream::new(dictionary! { "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), f(ww), f(hh)], "Resources" => dictionary! { "Font" => dictionary! { "Helv" => helvetica(false) } } }, encode(ops)?));
        doc.get_dictionary_mut(w).map_err(e)?.set("AP", dictionary! { "N" => ap });
    }
    if let Ok(Object::Reference(r)) = doc.catalog().map_err(e)?.get(b"AcroForm").cloned() {
        doc.get_dictionary_mut(r).map_err(e)?.set("NeedAppearances", true);
    }
    Ok(())
}

pub fn to_rtf(pages: &[String]) -> String {
    let mut s = String::from("{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Calibri;}}\\f0\\fs22\n");
    for (k, p) in pages.iter().enumerate() {
        if k > 0 {
            s.push_str("\\page\n");
        }
        for line in p.lines() {
            for c in line.chars() {
                match c {
                    '\\' | '{' | '}' => {
                        s.push('\\');
                        s.push(c);
                    }
                    c if c.is_ascii() => s.push(c),
                    c => {
                        for u in c.encode_utf16(&mut [0; 2]) {
                            s.push_str(&format!("\\u{}?", *u as i16));
                        }
                    }
                }
            }
            s.push_str("\\par\n");
        }
    }
    s.push('}');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(n: usize) -> Document {
        let imgs: Vec<RgbaImage> = (0..n).map(|k| RgbaImage::from_pixel(80 + k as u32 * 10, 60, image::Rgba([200, 40, 40, 255]))).collect();
        images_to_pdf(&imgs, None).unwrap()
    }

    #[test]
    fn organise_merge_and_reload() {
        let mut d = sample(3);
        assert_eq!(pages(&d).len(), 3);
        rotate(&mut d, &[0], 90).unwrap();
        assert_eq!(geometry(&d, pages(&d)[0]).1, 90);
        arrange(&mut d, &[2, 0, 0, 1]).unwrap();
        assert_eq!(pages(&d).len(), 4);
        delete(&mut d, &[1]).unwrap();
        assert_eq!(pages(&d).len(), 3);
        insert_blank(&mut d, 1, 595.0, 842.0).unwrap();
        let bytes = save(&mut d).unwrap();
        let mut back = load(&bytes, "").unwrap();
        assert_eq!(pages(&back).len(), 4);
        assert_eq!(geometry(&back, pages(&back)[1]).0, [0.0, 0.0, 595.0, 842.0]);
        let m = merge(vec![back.clone(), sample(2)]).unwrap();
        assert_eq!(pages(&m).len(), 6);
        insert_file(&mut back, 1, sample(2)).unwrap();
        assert_eq!(pages(&back).len(), 6);
        assert_eq!(split(&back, 4).unwrap().len(), 2);
        crop(&mut back, &[0], [5.0, 5.0, 5.0, 5.0]).unwrap();
    }

    #[test]
    fn marks_text_forms_outline() {
        let mut d = sample(2);
        for kind in [MarkKind::Highlight, MarkKind::Underline, MarkKind::Strike, MarkKind::Note, MarkKind::TextBox, MarkKind::Rect, MarkKind::Ellipse, MarkKind::Line, MarkKind::Arrow, MarkKind::Stamp] {
            add_mark(&mut d, 0, &Mark { kind, pts: vec![[5.0, 5.0], [50.0, 30.0]], strokes: Vec::new(), color: [255, 0, 0], width: 2.0, text: "Été approuvé".into(), size: 10.0 }).unwrap();
        }
        add_mark(&mut d, 1, &Mark { kind: MarkKind::Ink, pts: Vec::new(), strokes: vec![vec![[1.0, 1.0], [9.0, 9.0]]], color: [0, 0, 255], width: 1.5, text: String::new(), size: 0.0 }).unwrap();
        assert_eq!(comments(&d, 0).len(), 10);
        let s = Stamp { text: "CONFIDENTIEL {page}/{pages}".into(), size: 30.0, color: [200, 0, 0], opacity: 0.3, angle: 45.0, bold: true, place: 0, under: false };
        stamp_text(&mut d, &[0, 1], &s, "x", None).unwrap();
        stamp_text(&mut d, &[0, 1], &Stamp { place: 6, ..s.clone() }, "x", Some((1, 6))).unwrap();
        background(&mut d, &[0], [240, 240, 255]).unwrap();
        add_text_layer(&mut d, 0, &[("Bonjour".into(), [2.0, 2.0, 40.0, 10.0])]).unwrap();
        set_outline(&mut d, &[(1, "Début".into(), 0), (2, "Suite".into(), 1)]).unwrap();
        set_info(&mut d, &["Titre test".into(), "Moi".into(), String::new(), String::new(), String::new(), String::new()]);
        let bytes = save(&mut d).unwrap();
        let back = load(&bytes, "").unwrap();
        assert!(page_text(&back, 0).contains("Bonjour"));
        assert_eq!(outline(&back).len(), 2);
        assert_eq!(info(&back)[0], "Titre test");
        assert_eq!(remove_comments(&mut d, &[0, 1]).unwrap(), 11);
        assert_eq!(expand("{page}/{pages}", 0, 3, ""), "1/3");
        assert_eq!(civil(0), (1970, 1, 1));
        assert!(to_rtf(&["é{".into()]).contains("\\u233?\\{"));
    }

    #[test]
    fn password_roundtrip() {
        let mut d = sample(1);
        encrypt(&mut d, "secret", "", true, false, false).unwrap();
        let bytes = save(&mut d).unwrap();
        assert!(load(&bytes, "").is_err());
        let back = load(&bytes, "secret").unwrap();
        assert_eq!(pages(&back).len(), 1);
    }

    #[test]
    fn view_mapping() {
        let b = [0.0, 0.0, 100.0, 200.0];
        assert_eq!(to_pdf(b, 0, [0.0, 0.0]), [0.0, 200.0]);
        assert_eq!(to_pdf(b, 90, [0.0, 0.0]), [0.0, 0.0]);
        assert_eq!(to_pdf(b, 180, [0.0, 0.0]), [100.0, 0.0]);
        assert_eq!(to_pdf(b, 270, [0.0, 0.0]), [100.0, 200.0]);
        assert_eq!(view_size(b, 90), (200.0, 100.0));
    }
}
