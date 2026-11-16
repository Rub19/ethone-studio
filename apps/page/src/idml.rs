//! InDesign Markup (.idml, "Enregistrer sous… IDML" in InDesign) to a Page document: page size,
//! text frames with their story text and point size, placed images. Styles, threads between
//! frames, rotation and groups are not carried over.

use crate::model::{Doc, Frame, Kind, Page};
use studio_fx::archive::{unescape, unzip};

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!(" {name}=\"");
    let i = tag.find(&key)? + key.len();
    Some(unescape(&tag[i..i + tag[i..].find('"')?]))
}

fn nums(s: &str) -> Vec<f32> {
    s.split_whitespace().filter_map(|x| x.parse().ok()).collect()
}

/// (start tag, body) of every `<name …>…</name>` or `<name …/>` element.
fn elements<'a>(xml: &'a str, name: &str) -> Vec<(&'a str, &'a str)> {
    let (open, close) = (format!("<{name} "), format!("</{name}>"));
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = xml[at..].find(&open).map(|i| i + at) {
        let Some(end) = xml[i..].find('>').map(|e| e + i) else { break };
        let tag = &xml[i..=end];
        if tag.ends_with("/>") {
            out.push((tag, ""));
            at = end + 1;
        } else {
            let body_end = xml[end..].find(&close).map(|e| e + end).unwrap_or(xml.len());
            out.push((tag, &xml[end + 1..body_end]));
            at = body_end;
        }
    }
    out
}

/// Affine transform "a b c d tx ty" applied to a point.
fn apply(m: &[f32], p: [f32; 2]) -> [f32; 2] {
    if m.len() < 6 {
        return p;
    }
    [m[0] * p[0] + m[2] * p[1] + m[4], m[1] * p[0] + m[3] * p[1] + m[5]]
}

/// Spread-space bounding box of an item from its path anchors.
fn bbox(tag: &str, body: &str) -> Option<[f32; 4]> {
    let m = attr(tag, "ItemTransform").map(|t| nums(&t)).unwrap_or_default();
    let pts: Vec<[f32; 2]> = elements(body, "PathPointType").iter().filter_map(|(t, _)| attr(t, "Anchor")).map(|a| nums(&a)).filter(|v| v.len() == 2).map(|v| apply(&m, [v[0], v[1]])).collect();
    if pts.is_empty() {
        return None;
    }
    let (x0, x1) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])));
    let (y0, y1) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[1]), b.max(p[1])));
    Some([x0, y0, x1, y1])
}

/// What Page keeps of an InDesign story: its text and the look of its first paragraph.
#[derive(Clone, Default, Debug, PartialEq)]
struct Story {
    text: String,
    size: Option<f32>,
    /// 0 left, 1 centre, 2 right, 3 justified (Page's text alignment).
    align: u8,
    family: Option<String>,
    bold: bool,
}

/// Story text with paragraph breaks (`<Br/>`, `<Br />` and paragraph ends) and its first paragraph's style.
fn story(xml: &str) -> Story {
    let xml = xml.replace("<Br />", "<Br/>").replace("</ParagraphStyleRange>", "<Br/></ParagraphStyleRange>");
    let mut text = String::new();
    let mut rest = xml.as_str();
    loop {
        let (c, b) = (rest.find("<Content>"), rest.find("<Br/>"));
        match (c, b) {
            (Some(ci), bi) if bi.is_none_or(|bi| ci < bi) => {
                let s = &rest[ci + 9..];
                let Some(e) = s.find("</Content>") else { break };
                text.push_str(&unescape(&s[..e]));
                rest = &s[e..];
            }
            (_, Some(bi)) => {
                text.push('\n');
                rest = &rest[bi + 5..];
            }
            _ => break,
        }
    }
    let ranges = elements(&xml, "CharacterStyleRange");
    let size = ranges.iter().find_map(|(t, _)| attr(t, "PointSize")).and_then(|s| s.parse().ok());
    let family = ranges.iter().find_map(|(_, body)| {
        let i = body.find("<AppliedFont")?;
        let start = i + body[i..].find('>')? + 1;
        Some(unescape(body[start..start + body[start..].find('<')?].trim())).filter(|f| !f.is_empty())
    });
    let bold = ranges.first().and_then(|(t, _)| attr(t, "FontStyle")).is_some_and(|f| f.contains("Bold"));
    let align = match elements(&xml, "ParagraphStyleRange").first().and_then(|(t, _)| attr(t, "Justification")).as_deref() {
        Some("CenterAlign" | "CenterJustified") => 1,
        Some("RightAlign" | "RightJustified" | "AwayFromBindingSide") => 2,
        Some("LeftJustified" | "FullyJustified") => 3,
        _ => 0,
    };
    Story { text: text.trim_end().to_string(), size, align, family, bold }
}

fn file_uri(uri: &str) -> String {
    let p = uri.strip_prefix("file:").unwrap_or(uri).trim_start_matches('/');
    let mut out = Vec::new();
    let b = p.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && let Some(v) = p.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    let s = String::from_utf8_lossy(&out).into_owned();
    if s.get(1..2) == Some(":") { s } else { format!("/{s}") }
}

pub fn import(bytes: &[u8]) -> Result<(Doc, usize), String> {
    let files = unzip(bytes)?;
    let get = |name: &str| files.iter().find(|(n, _)| n == name).map(|(_, d)| String::from_utf8_lossy(d).into_owned());
    let design = get("designmap.xml").ok_or("ce n'est pas un fichier IDML (designmap.xml manquant)")?;
    let stories: std::collections::HashMap<String, Story> = files
        .iter()
        .filter(|(n, _)| n.starts_with("Stories/"))
        .filter_map(|(_, d)| {
            let xml = String::from_utf8_lossy(d);
            let id = elements(&xml, "Story").first().and_then(|(t, _)| attr(t, "Self"))?;
            Some((id, story(&xml)))
        })
        .collect();
    let mut doc = Doc::a4();
    doc.pages.clear();
    let mut placed = 0;
    let spreads: Vec<String> = elements(&design, "idPkg:Spread").iter().filter_map(|(t, _)| attr(t, "src")).collect();
    for src in spreads {
        let Some(xml) = get(&src) else { continue };
        let mut pages: Vec<[f32; 4]> = elements(&xml, "Page")
            .iter()
            .filter_map(|(t, _)| {
                let g = nums(&attr(t, "GeometricBounds")?);
                let m = attr(t, "ItemTransform").map(|x| nums(&x)).unwrap_or_default();
                let tl = apply(&m, [g.get(1).copied()?, g.first().copied()?]);
                let br = apply(&m, [g.get(3).copied()?, g.get(2).copied()?]);
                Some([tl[0], tl[1], br[0], br[1]])
            })
            .collect();
        pages.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let first = doc.pages.len();
        for p in &pages {
            if doc.pages.is_empty() {
                (doc.width, doc.height) = (p[2] - p[0], p[3] - p[1]);
            }
            let size = [p[2] - p[0], p[3] - p[1]];
            doc.pages.push(Page { size: (size != [doc.width, doc.height]).then_some(size), ..Page::default() });
        }
        let mut place = |b: [f32; 4], kind: Kind| {
            let cx = (b[0] + b[2]) / 2.0;
            let Some(k) = pages.iter().position(|p| cx >= p[0] && cx <= p[2]).or((!pages.is_empty()).then_some(0)) else { return };
            let p = pages[k];
            doc.pages[first + k].frames.push(Frame::new(b[0] - p[0], b[1] - p[1], (b[2] - b[0]).max(4.0), (b[3] - b[1]).max(4.0), kind));
            placed += 1;
        };
        for (tag, body) in elements(&xml, "TextFrame") {
            let (Some(b), Some(id)) = (bbox(tag, body), attr(tag, "ParentStory")) else { continue };
            let st = stories.get(&id).cloned().unwrap_or_default();
            let mut kind = Kind::text(&st.text, st.size.unwrap_or(12.0), st.bold);
            if let Kind::Text { align, family, .. } = &mut kind {
                *align = st.align;
                if let Some(f) = st.family {
                    *family = f;
                }
            }
            place(b, kind);
        }
        for (tag, body) in elements(&xml, "Rectangle") {
            let Some(uri) = elements(body, "Link").first().and_then(|(t, _)| attr(t, "LinkResourceURI")) else { continue };
            if let Some(b) = bbox(tag, body) {
                place(b, Kind::image(&file_uri(&uri), 0.0));
            }
        }
    }
    if doc.pages.is_empty() {
        return Err("aucune page trouvée dans ce fichier IDML".into());
    }
    Ok((doc, placed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_and_uri() {
        let st = story(r#"<Story Self="u1"><ParagraphStyleRange Justification="CenterAlign"><CharacterStyleRange PointSize="18" FontStyle="Bold"><Properties><AppliedFont type="string">Georgia</AppliedFont></Properties><Content>Bonjour &amp; bienvenue</Content><Br /><Content>Ligne 2</Content></CharacterStyleRange></ParagraphStyleRange><ParagraphStyleRange><CharacterStyleRange><Content>Paragraphe 2</Content></CharacterStyleRange></ParagraphStyleRange></Story>"#);
        assert_eq!(st.text, "Bonjour & bienvenue\nLigne 2\nParagraphe 2");
        assert_eq!((st.size, st.align, st.family.as_deref(), st.bold), (Some(18.0), 1, Some("Georgia"), true));
        assert_eq!(file_uri("file:///C:/Photos/mon%20chat.jpg"), "C:/Photos/mon chat.jpg");
        assert_eq!(file_uri("file:/Users/a/b.png"), "/Users/a/b.png");
        let tf = r#"<TextFrame Self="t" ParentStory="u1" ItemTransform="1 0 0 1 10 20"><Properties><PathGeometry><GeometryPathType><PathPointArray><PathPointType Anchor="0 0"/><PathPointType Anchor="100 50"/></PathPointArray></GeometryPathType></PathGeometry></Properties></TextFrame>"#;
        let (tag, body) = elements(tf, "TextFrame")[0];
        assert_eq!(bbox(tag, body), Some([10.0, 20.0, 110.0, 70.0]));
    }
}

#[cfg(test)]
mod real_file {
    #[test]
    fn opens_a_real_indesign_export() {
        // tests-data/example.idml comes from jorisros/IDMLlib (BSD-2-Clause).
        let (doc, placed) = super::import(include_bytes!("../tests-data/example.idml")).unwrap();
        assert!(!doc.pages.is_empty() && doc.width > 50.0 && doc.height > 50.0, "{} pages {}×{}", doc.pages.len(), doc.width, doc.height);
        let texts: Vec<String> = doc.pages.iter().flat_map(|p| &p.frames).filter_map(|f| match &f.kind {
            crate::model::Kind::Text { text, .. } => Some(text.clone()),
            _ => None,
        }).collect();
        println!("{} pages, {placed} blocs, textes : {texts:?}", doc.pages.len());
        assert!(placed > 0 && texts.iter().any(|t| t.starts_with("Hello world\nSecond line of the test")), "{texts:?}");
    }
}
