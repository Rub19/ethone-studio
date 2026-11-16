use crate::model::{self, CharStyle, Doc, Frame, Kind, Layer, MASTER, NUMBERING, ObjStyle, Section};
use crate::{PageApp, STYLES, color_row};
use eframe::egui::{self, Align, Align2, Color32, CornerRadius, FontId, Key, Layout, RichText, Sense, Ui, Vec2, vec2};
use std::path::PathBuf;
use studio_ui::pro::{self, BLUE, FAINT, GREEN, INK, INK_STRONG, MUTED, RED, semibold};

pub struct FindState {
    pub find: String,
    pub with: String,
    pub case: bool,
    pub word: bool,
    pub scope: usize,
    pub style: String,
    pub result: Option<String>,
    pub next: usize,
}

impl FindState {
    pub fn new(find: String) -> Self {
        FindState { find, with: String::new(), case: false, word: false, scope: 0, style: String::new(), result: None, next: 0 }
    }
}

pub struct StepOpts {
    pub count: u32,
    pub dx: f32,
    pub dy: f32,
    pub rot: f32,
    pub grid: bool,
    pub rows: u32,
    pub cols: u32,
}

pub struct TocOpts {
    pub title: String,
    pub styles: Vec<(String, bool)>,
    pub new_page: bool,
}

#[derive(Clone, PartialEq)]
pub struct Issue {
    pub page: usize,
    pub frame: Option<usize>,
    pub error: bool,
    pub msg: String,
}

pub fn parse_range(s: &str, n: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for part in s.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.trim().parse::<usize>().unwrap_or(1), b.trim().parse::<usize>().unwrap_or(n)),
            None => match part.parse::<usize>() {
                Ok(v) => (v, v),
                Err(_) => continue,
            },
        };
        for p in a.max(1)..=b.min(n) {
            if !out.contains(&(p - 1)) {
                out.push(p - 1);
            }
        }
    }
    out
}

pub fn find_in(hay: &str, needle: &str, case: bool, word: bool) -> Vec<(usize, usize)> {
    let h: Vec<(usize, char)> = hay.char_indices().collect();
    let n: Vec<char> = needle.chars().collect();
    let mut out = Vec::new();
    if n.is_empty() {
        return out;
    }
    let eq = |a: char, b: char| if case { a == b } else { a.to_lowercase().eq(b.to_lowercase()) };
    let mut k = 0;
    while k + n.len() <= h.len() {
        if (0..n.len()).all(|j| eq(h[k + j].1, n[j])) {
            let before = k == 0 || !h[k - 1].1.is_alphanumeric();
            let after = k + n.len() == h.len() || !h[k + n.len()].1.is_alphanumeric();
            if !word || (before && after) {
                let end = if k + n.len() < h.len() { h[k + n.len()].0 } else { hay.len() };
                out.push((h[k].0, end));
                k += n.len();
                continue;
            }
        }
        k += 1;
    }
    out
}

pub fn replace_in(hay: &str, needle: &str, with: &str, case: bool, word: bool) -> (String, usize) {
    let m = find_in(hay, needle, case, word);
    if m.is_empty() {
        return (hay.to_string(), 0);
    }
    let mut out = String::new();
    let mut last = 0;
    for (a, b) in &m {
        out.push_str(&hay[last..*a]);
        out.push_str(with);
        last = *b;
    }
    out.push_str(&hay[last..]);
    (out, m.len())
}

pub const TYPO: [&str; 5] = ["Espaces doubles", "Apostrophes typographiques", "Guillemets français", "Points de suspension", "Espaces fines avant ; : ! ?"];

pub fn typo_fix(t: &str, k: usize) -> String {
    match k {
        0 => {
            let mut s = t.to_string();
            while s.contains("  ") {
                s = s.replace("  ", " ");
            }
            s
        }
        1 => t.replace('\'', "’"),
        2 => {
            let mut open = true;
            t.chars()
                .map(|c| {
                    if c == '"' {
                        let r = if open { "«\u{a0}" } else { "\u{a0}»" };
                        open = !open;
                        r.to_string()
                    } else {
                        c.to_string()
                    }
                })
                .collect()
        }
        3 => t.replace("...", "…"),
        _ => {
            let mut s = String::new();
            for c in t.chars() {
                if matches!(c, ';' | ':' | '!' | '?') {
                    while s.ends_with(' ') {
                        s.pop();
                    }
                    if !s.ends_with('\u{202f}') && !s.is_empty() && !s.ends_with('\n') && !s.ends_with("http") && !s.ends_with("https") {
                        s.push('\u{202f}');
                    }
                }
                s.push(c);
            }
            s
        }
    }
}

pub fn case_fix(t: &str, k: usize) -> String {
    match k {
        0 => t.to_uppercase(),
        1 => t.to_lowercase(),
        2 => {
            let mut up = true;
            t.chars()
                .map(|c| {
                    let r = if up { c.to_uppercase().collect::<String>() } else { c.to_lowercase().collect() };
                    up = !c.is_alphanumeric() && c != '\'' && c != '’';
                    r
                })
                .collect()
        }
        _ => {
            let mut up = true;
            t.chars()
                .map(|c| {
                    let r = if up && c.is_alphabetic() {
                        up = false;
                        c.to_uppercase().collect::<String>()
                    } else {
                        c.to_lowercase().collect()
                    };
                    if matches!(c, '.' | '!' | '?' | '\n') {
                        up = true;
                    }
                    r
                })
                .collect()
        }
    }
}

pub const CASES: [&str; 4] = ["MAJUSCULES", "minuscules", "Première Lettre En Capitale", "Phrase normale."];

pub fn copy_look(src: &Frame, dst: &mut Frame) {
    dst.opacity = src.opacity;
    dst.stroke = src.stroke;
    dst.stroke_w = src.stroke_w;
    dst.shadow = src.shadow;
    dst.dash = src.dash;
    dst.fill2 = src.fill2;
    dst.grad_angle = src.grad_angle;
    let keep = (dst.fx.note.clone(), dst.fx.layer, dst.fx.group, dst.fx.wrap, dst.fx.wrap_off);
    dst.fx = src.fx.clone();
    (dst.fx.note, dst.fx.layer, dst.fx.group, dst.fx.wrap, dst.fx.wrap_off) = keep;
    match (&src.kind, &mut dst.kind) {
        (Kind::Text { size, family, bold, color, align, leading, italic, tracking, background, padding, underline, upper, para_space, indent, list, opts, .. }, Kind::Text { size: s2, family: f2, bold: b2, color: c2, align: a2, leading: l2, italic: i2, tracking: t2, background: bg2, padding: p2, underline: u2, upper: up2, para_space: ps2, indent: in2, list: li2, opts: o2, .. }) => {
            (*s2, *b2, *c2, *a2, *l2, *i2, *t2, *bg2, *p2, *u2, *up2, *ps2, *in2, *li2) = (*size, *bold, *color, *align, *leading, *italic, *tracking, *background, *padding, *underline, *upper, *para_space, *indent, *list);
            *f2 = family.clone();
            let auto = o2.auto_h;
            *o2 = opts.clone();
            o2.auto_h = auto;
        }
        (Kind::Rect { fill, radius }, Kind::Rect { fill: f2, radius: r2 }) => {
            *f2 = *fill;
            *r2 = *radius;
        }
        (Kind::Rect { fill, .. } | Kind::Ellipse { fill } | Kind::Polygon { fill, .. }, Kind::Rect { fill: f2, .. } | Kind::Ellipse { fill: f2 } | Kind::Polygon { fill: f2, .. }) => *f2 = *fill,
        (Kind::Line { color, width, arrow, start_arrow, cap, .. }, Kind::Line { color: c2, width: w2, arrow: a2, start_arrow: s2, cap: k2, .. }) => (*c2, *w2, *a2, *s2, *k2) = (*color, *width, *arrow, *start_arrow, *cap),
        (Kind::Image { radius, filter, cover, stretch, .. }, Kind::Image { radius: r2, filter: f2, cover: c2, stretch: s2, .. }) => (*r2, *f2, *c2, *s2) = (*radius, *filter, *cover, *stretch),
        (Kind::Path { fill, color, width, .. }, Kind::Path { fill: f2, color: c2, width: w2, .. }) => (*f2, *c2, *w2) = (*fill, *color, *width),
        (Kind::Table { size, family, color, line, header, stripe, align, first_col, .. }, Kind::Table { size: s2, family: f2, color: c2, line: l2, header: h2, stripe: st2, align: a2, first_col: fc2, .. }) => {
            (*s2, *c2, *l2, *h2, *st2, *a2, *fc2) = (*size, *color, *line, *header, *stripe, *align, *first_col);
            *f2 = family.clone();
        }
        _ => {}
    }
}

fn texts_mut(doc: &mut Doc) -> impl Iterator<Item = (usize, &mut Frame)> {
    doc.pages.iter_mut().enumerate().flat_map(|(i, p)| p.frames.iter_mut().map(move |f| (i, f))).chain(doc.master.frames.iter_mut().map(|f| (MASTER, f)))
}

impl PageApp {
    pub fn replace_all(&mut self, st: &FindState) -> usize {
        if st.find.is_empty() {
            return 0;
        }
        let (page, sel) = (self.page, self.sel);
        let mut n = 0;
        let mut idx = 0usize;
        let mut last_page = usize::MAX - 1;
        for (pg, f) in texts_mut(&mut self.doc) {
            if pg != last_page {
                idx = 0;
                last_page = pg;
            } else {
                idx += 1;
            }
            let in_scope = match st.scope {
                1 => pg == page,
                2 => pg == page && sel == Some(idx),
                _ => true,
            };
            if !in_scope {
                continue;
            }
            match &mut f.kind {
                Kind::Text { text, opts, .. } if st.style.is_empty() || opts.style == st.style => {
                    let (t, c) = replace_in(text, &st.find, &st.with, st.case, st.word);
                    *text = t;
                    n += c;
                }
                Kind::TextPath { text, .. } if st.style.is_empty() => {
                    let (t, c) = replace_in(text, &st.find, &st.with, st.case, st.word);
                    *text = t;
                    n += c;
                }
                Kind::Table { cells, .. } if st.style.is_empty() => {
                    for cell in cells.iter_mut().flatten() {
                        let (t, c) = replace_in(cell, &st.find, &st.with, st.case, st.word);
                        *cell = t;
                        n += c;
                    }
                }
                _ => {}
            }
        }
        n
    }

    fn find_next(&mut self, st: &mut FindState) {
        let mut hits = Vec::new();
        for (pi, p) in self.doc.pages.iter().enumerate() {
            for (fi, f) in p.frames.iter().enumerate() {
                let txt = match &f.kind {
                    Kind::Text { text, opts, .. } if st.style.is_empty() || opts.style == st.style => text.clone(),
                    Kind::TextPath { text, .. } => text.clone(),
                    Kind::Table { cells, .. } => cells.iter().flatten().cloned().collect::<Vec<_>>().join(" "),
                    _ => continue,
                };
                if !find_in(&txt, &st.find, st.case, st.word).is_empty() {
                    hits.push((pi, fi));
                }
            }
        }
        if hits.is_empty() {
            st.result = Some("Aucune occurrence".into());
            return;
        }
        let k = st.next % hits.len();
        st.next = k + 1;
        (self.page, self.sel) = (hits[k].0, Some(hits[k].1));
        self.picks.clear();
        st.result = Some(format!("Bloc {} sur {} contenant « {} » (page {})", k + 1, hits.len(), st.find, self.doc.page_label(hits[k].0)));
    }

    pub fn typo_all(&mut self, kinds: &[usize]) -> usize {
        let mut n = 0;
        for (_, f) in texts_mut(&mut self.doc) {
            if let Kind::Text { text, .. } | Kind::TextPath { text, .. } = &mut f.kind {
                let mut t = text.clone();
                for k in kinds {
                    t = typo_fix(&t, *k);
                }
                if t != *text {
                    n += 1;
                    *text = t;
                }
            }
        }
        n
    }

    pub fn case_sel(&mut self, k: usize) {
        for i in self.selection() {
            if let Kind::Text { text, .. } | Kind::TextPath { text, .. } = &mut self.frames()[i].kind {
                *text = case_fix(text, k);
            }
        }
    }

    pub fn group(&mut self, on: bool) {
        let sel = self.selection();
        let id = if on { self.doc.pages.iter().flat_map(|p| p.frames.iter()).chain(&self.doc.master.frames).map(|f| f.fx.group).max().unwrap_or(0) + 1 } else { 0 };
        if on && sel.len() < 2 {
            self.notice = Some(("Sélectionne au moins deux blocs (Maj+clic) pour les grouper".into(), false));
            return;
        }
        for i in sel {
            self.frames()[i].fx.group = id;
        }
    }

    pub fn select_all(&mut self) {
        let n = self.frames_ref().len();
        self.picks = (0..n).filter(|i| !self.frames_ref()[*i].locked && !self.frames_ref()[*i].hidden).collect();
        self.sel = self.picks.last().copied();
    }

    pub fn delete_selection(&mut self) {
        let mut sel = self.selection();
        sel.sort_unstable();
        for i in sel.into_iter().rev() {
            self.remove_frame(i);
        }
        self.picks.clear();
    }

    pub fn bounds(&self, sel: &[usize]) -> Option<[f32; 4]> {
        let fr = self.frames_ref();
        let mut b: Option<[f32; 4]> = None;
        for &i in sel {
            let f = fr.get(i)?;
            b = Some(match b {
                None => [f.x, f.y, f.x + f.w, f.y + f.h],
                Some(b) => [b[0].min(f.x), b[1].min(f.y), b[2].max(f.x + f.w), b[3].max(f.y + f.h)],
            });
        }
        b
    }

    pub fn align_to(&mut self, axis: usize, to: f32, target: usize) {
        let sel = self.selection();
        let (pw, ph) = self.cur_size();
        let m = self.doc.margin;
        let area = match target {
            0 if sel.len() > 1 => self.bounds(&sel),
            1 => Some([m, m, pw - m, ph - m]),
            2 => Some([0.0, 0.0, pw, ph]),
            _ => self.sel.and_then(|k| self.bounds(&[k])),
        };
        let Some(a) = area else { return };
        for i in sel {
            let f = &mut self.frames()[i];
            let (lo, hi, size) = if axis == 0 { (a[0], a[2], f.w) } else { (a[1], a[3], f.h) };
            let v = lo + (hi - lo - size) * to;
            if axis == 0 { f.x = v } else { f.y = v }
        }
    }

    pub fn distribute(&mut self, axis: usize, gap: Option<f32>) {
        let mut sel = self.selection();
        if sel.len() < 3 && gap.is_none() || sel.len() < 2 {
            self.notice = Some(("Sélectionne au moins trois blocs (deux avec un espacement fixe)".into(), false));
            return;
        }
        let pos = |f: &Frame| if axis == 0 { (f.x, f.w) } else { (f.y, f.h) };
        sel.sort_by(|a, b| pos(&self.frames_ref()[*a]).0.total_cmp(&pos(&self.frames_ref()[*b]).0));
        let first = pos(&self.frames_ref()[sel[0]]);
        let last = pos(&self.frames_ref()[*sel.last().unwrap()]);
        let total: f32 = sel.iter().map(|i| pos(&self.frames_ref()[*i]).1).sum();
        let g = gap.unwrap_or(((last.0 + last.1 - first.0) - total) / (sel.len() - 1) as f32);
        let mut x = first.0;
        for i in sel {
            let f = &mut self.frames()[i];
            if axis == 0 {
                f.x = x;
                x += f.w + g;
            } else {
                f.y = x;
                x += f.h + g;
            }
        }
    }

    pub fn flip(&mut self, h: bool) {
        for i in self.selection() {
            let f = &mut self.frames()[i];
            if h { f.fx.flip_h = !f.fx.flip_h } else { f.fx.flip_v = !f.fx.flip_v }
        }
    }

    pub fn rotate_by(&mut self, a: f32) {
        for i in self.selection() {
            let f = &mut self.frames()[i];
            f.rotation = (f.rotation + a + 540.0).rem_euclid(360.0) - 180.0;
        }
    }

    pub fn fitting(&mut self, k: usize) {
        let Some(i) = self.sel else { return };
        let f = &mut self.frames()[i];
        let Kind::Image { path, cover, ox, oy, zoom, stretch, .. } = &mut f.kind else { return };
        match k {
            0 => (*cover, *stretch, *zoom, *ox, *oy) = (true, false, 1.0, 0.0, 0.0),
            1 => (*cover, *stretch, *zoom, *ox, *oy) = (false, false, 1.0, 0.0, 0.0),
            2 => (*stretch, *zoom, *ox, *oy) = (true, 1.0, 0.0, 0.0),
            3 => (*ox, *oy) = (0.0, 0.0),
            _ => {
                if let Ok((w, h)) = image::image_dimensions(&*path) {
                    let ratio = h as f32 / w.max(1) as f32;
                    (*zoom, *ox, *oy, *stretch) = (1.0, 0.0, 0.0, false);
                    f.h = f.w * ratio;
                }
            }
        }
    }

    pub fn place_image(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif", "svg", "psd"]).pick_file() {
            self.place_image_path(&p);
        }
    }

    pub fn place_image_path(&mut self, p: &std::path::Path) {
        let png;
        let p = if studio_fx::psd::is_psd(p) {
            let dir = studio_core::data_dir().join("psd");
            png = studio_fx::psd::png_copy_path(p, &dir);
            if let Err(e) = std::fs::create_dir_all(&dir).map_err(|e| e.to_string()).and_then(|_| studio_fx::psd::open_any(p)).and_then(|img| img.save(&png).map_err(|e| e.to_string())) {
                self.notice = Some((format!("PSD : {e}"), false));
                return;
            }
            png.as_path()
        } else {
            p
        };
        let path = p.to_string_lossy().into_owned();
        if let Some(i) = self.sel
            && let Kind::Image { path: pp, .. } = &mut self.frames()[i].kind
        {
            *pp = path;
            return;
        }
        let (pw, ph) = self.cur_size();
        let svg = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
        let dims = if svg {
            std::fs::read(p).ok().and_then(|b| resvg::usvg::Tree::from_data(&b, &resvg::usvg::Options::default()).ok()).map(|t| (t.size().width(), t.size().height()))
        } else {
            image::image_dimensions(p).ok().map(|(w, h)| (w as f32, h as f32))
        };
        let (w, h) = dims.unwrap_or((400.0, 300.0));
        let s = ((pw - 2.0 * self.doc.margin) / w).min((ph - 2.0 * self.doc.margin) / h).min(1.0);
        let mut f = Frame::new((pw - w * s) / 2.0, (ph - h * s) / 2.0, w * s, h * s, Kind::image(&path, 0.0));
        f.fx.layer = self.layer;
        self.push_frame(f);
    }

    pub fn step_repeat(&mut self, o: &StepOpts) {
        let Some(i) = self.sel else { return };
        let src = self.frames_ref()[i].clone();
        let mut made = 0;
        if o.grid {
            for r in 0..o.rows.max(1) {
                for c in 0..o.cols.max(1) {
                    if r == 0 && c == 0 {
                        continue;
                    }
                    let mut f = src.clone();
                    f.x += c as f32 * (src.w + o.dx);
                    f.y += r as f32 * (src.h + o.dy);
                    self.frames().push(f);
                    made += 1;
                }
            }
        } else {
            for k in 1..=o.count {
                let mut f = src.clone();
                f.x += o.dx * k as f32;
                f.y += o.dy * k as f32;
                f.rotation += o.rot * k as f32;
                self.frames().push(f);
                made += 1;
            }
        }
        for f in self.frames().iter_mut().rev().take(made) {
            if let Kind::Text { link, .. } = &mut f.kind {
                *link = None;
            }
        }
        self.notice = Some((format!("{made} copie(s) créée(s)"), true));
    }

    pub fn make_toc(&mut self, o: &TocOpts) {
        let wanted: Vec<&str> = o.styles.iter().filter(|s| s.1).map(|s| s.0.as_str()).collect();
        let mut lines = Vec::new();
        if !o.title.is_empty() {
            lines.push(o.title.clone());
        }
        let first_level = wanted.first().copied().unwrap_or("");
        for (pi, p) in self.doc.pages.iter().enumerate() {
            for f in &p.frames {
                if let Kind::Text { text, opts, .. } = &f.kind
                    && wanted.contains(&opts.style.as_str())
                {
                    let title = model::visible(text).lines().find(|l| !l.trim().is_empty()).unwrap_or("").replace('\t', " ");
                    if title.is_empty() {
                        continue;
                    }
                    let pad = if opts.style == first_level { "" } else { "\u{a0}\u{a0}\u{a0}\u{a0}" };
                    lines.push(format!("{pad}{title}\t{}", self.doc.page_label(pi + usize::from(o.new_page))));
                }
            }
        }
        if lines.len() <= usize::from(!o.title.is_empty()) {
            self.notice = Some(("Aucun titre trouvé : applique d'abord les styles Titre 1 ou Titre 2 à tes titres".into(), false));
            return;
        }
        if o.new_page {
            self.doc.pages.insert(0, Default::default());
            self.page = 0;
        } else if self.page == MASTER {
            self.page = 0;
        }
        let (pw, ph) = self.doc.page_size(self.page);
        let m = self.doc.margin;
        let mut k = Kind::text(&lines.join("\n"), 11.0, false);
        if let Kind::Text { opts, para_space, .. } = &mut k {
            opts.leader = true;
            opts.tab_right = true;
            *para_space = 6.0;
        }
        let mut f = Frame::new(m, m, pw - 2.0 * m, ph - 2.0 * m, k);
        f.name = "Table des matières".into();
        self.push_frame(f);
        self.notice = Some((format!("Table des matières : {} entrée(s)", lines.len() - usize::from(!o.title.is_empty())), true));
    }

    pub fn compute_preflight(&self) -> Vec<Issue> {
        let mut out = Vec::new();
        let m = self.measure();
        let mut push = |page: usize, frame: Option<usize>, error: bool, msg: String| out.push(Issue { page, frame, error, msg });
        for pi in 0..self.doc.pages.len() {
            let (_, over) = self.doc.to_svg(pi, &m, false);
            let (pw, ph) = self.doc.page_size(pi);
            let b = self.doc.bleed;
            for (fi, f) in self.doc.pages[pi].frames.iter().enumerate() {
                let name = f.title();
                if over.get(fi).copied().unwrap_or(false) {
                    push(pi, Some(fi), true, format!("Texte en excès : {name}"));
                }
                match &f.kind {
                    Kind::Image { path, zoom, .. } => {
                        if path.is_empty() {
                            push(pi, Some(fi), false, "Bloc image vide".into());
                        } else if !std::path::Path::new(path).is_file() {
                            push(pi, Some(fi), true, format!("Lien manquant : {name}"));
                        } else if let Ok((w, _)) = image::image_dimensions(path) {
                            let ppi = w as f32 / (f.w * zoom / 72.0).max(0.01);
                            if ppi < 150.0 {
                                push(pi, Some(fi), false, format!("Résolution faible ({ppi:.0} ppp) : {name}"));
                            }
                        }
                    }
                    Kind::Text { text, family, size, .. } => {
                        if text.trim().is_empty() {
                            push(pi, Some(fi), false, "Bloc de texte vide".into());
                        }
                        if text.contains("  ") {
                            push(pi, Some(fi), false, format!("Espaces doubles : {name}"));
                        }
                        if *size < 6.0 {
                            push(pi, Some(fi), false, format!("Corps très petit ({size:.1} pt) : {name}"));
                        }
                        if !self.families.contains(family) {
                            push(pi, Some(fi), true, format!("Police absente : {family}"));
                        }
                    }
                    _ => {}
                }
                if f.x + f.w < -b || f.y + f.h < -b || f.x > pw + b || f.y > ph + b {
                    push(pi, Some(fi), false, format!("Hors de la page : {name}"));
                } else if b <= 0.0 && (f.x < -0.5 || f.y < -0.5 || f.x + f.w > pw + 0.5 || f.y + f.h > ph + 0.5) && !matches!(f.kind, Kind::Text { .. }) {
                    push(pi, Some(fi), false, format!("Déborde de la page sans fond perdu : {name}"));
                }
                if !f.fx.note.is_empty() {
                    push(pi, Some(fi), false, format!("Note : {}", f.fx.note.lines().next().unwrap_or("")));
                }
            }
        }
        out
    }

    pub fn issues(&mut self) -> &[Issue] {
        let stale = self.preflight.as_ref().is_none_or(|(d, _)| *d != self.doc);
        if stale {
            let v = self.compute_preflight();
            self.preflight = Some((self.doc.clone(), v));
        }
        &self.preflight.as_ref().unwrap().1
    }

    pub fn preflight_panel(&mut self, ui: &mut Ui) {
        let issues = self.issues().to_vec();
        let errors = issues.iter().filter(|i| i.error).count();
        let mut go = None;
        pro::dock_panel(ui, "preflight", "Contrôle en amont", |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 5.0, if errors > 0 { RED } else { GREEN });
                ui.label(RichText::new(if errors > 0 { format!("{errors} erreur(s), {} avertissement(s)", issues.len() - errors) } else if issues.is_empty() { "Aucun problème".into() } else { format!("Aucune erreur, {} avertissement(s)", issues.len()) }).size(12.5).color(INK));
            });
            ui.add_space(6.0);
            for (k, is) in issues.iter().enumerate().take(80) {
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(6), Color32::from_white_alpha(8));
                }
                ui.painter().circle_filled(r.left_center() + vec2(8.0, 0.0), 3.5, if is.error { RED } else { Color32::from_rgb(255, 196, 0) });
                let label = format!("p. {} · {}", self.doc.page_label(is.page), is.msg);
                let g = ui.painter().layout_no_wrap(label, FontId::proportional(12.0), INK);
                let pc = ui.painter().with_clip_rect(r);
                pc.galley(r.left_center() + vec2(18.0, -g.size().y / 2.0), g, INK);
                if resp.on_hover_text(&is.msg).clicked() {
                    go = Some(k);
                }
            }
        });
        if let Some(k) = go {
            let is = &issues[k];
            self.page = is.page;
            self.sel = is.frame;
            self.picks.clear();
        }
        self.notes_panel(ui);
    }

    fn notes_panel(&mut self, ui: &mut Ui) {
        let notes: Vec<(usize, usize, String)> = self.doc.pages.iter().enumerate().flat_map(|(p, pg)| pg.frames.iter().enumerate().filter(|(_, f)| !f.fx.note.is_empty()).map(move |(i, f)| (p, i, f.fx.note.clone()))).collect();
        let mut go = None;
        pro::dock_panel(ui, "notes", "Notes", |ui| {
            if notes.is_empty() {
                ui.label(RichText::new("Outil Note : clique un bloc pour y attacher un commentaire. Les notes ne sont jamais imprimées.").size(12.0).color(FAINT));
            }
            for (p, i, n) in &notes {
                if ui.add(egui::Button::new(RichText::new(format!("p. {} · {}", self.doc.page_label(*p), n.lines().next().unwrap_or(""))).size(12.0)).frame(false)).clicked() {
                    go = Some((*p, *i));
                }
            }
        });
        if let Some((p, i)) = go {
            (self.page, self.sel) = (p, Some(i));
            self.note_edit = Some(i);
        }
    }

    pub fn links_panel(&mut self, ui: &mut Ui) {
        let links: Vec<(usize, usize, String)> = self.doc.pages.iter().enumerate().flat_map(|(p, pg)| pg.frames.iter().enumerate().filter_map(move |(i, f)| if let Kind::Image { path, .. } = &f.kind { (!path.is_empty()).then(|| (p, i, path.clone())) } else { None })).chain(self.doc.master.frames.iter().enumerate().filter_map(|(i, f)| if let Kind::Image { path, .. } = &f.kind { (!path.is_empty()).then(|| (MASTER, i, path.clone())) } else { None })).collect();
        let mut act: Option<(usize, usize, u8)> = None;
        let mut folder = false;
        pro::dock_panel(ui, "links", "Liens", |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let missing = links.iter().filter(|l| !std::path::Path::new(&l.2).is_file()).count();
            ui.label(RichText::new(format!("{} image(s) placée(s) · {missing} manquante(s)", links.len())).size(12.5).color(if missing > 0 { RED } else { MUTED }));
            for (p, i, path) in &links {
                let ok = std::path::Path::new(path).is_file();
                let name = std::path::Path::new(path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let dims = if ok { image::image_dimensions(path).map(|(w, h)| format!("{w} × {h} px")).unwrap_or_default() } else { "introuvable".into() };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if ok { "●" } else { "▲" }).size(11.0).color(if ok { GREEN } else { RED }));
                    ui.vertical(|ui| {
                        ui.set_max_width(150.0);
                        ui.add(egui::Label::new(RichText::new(&name).size(12.0).color(INK)).truncate());
                        ui.label(RichText::new(format!("{} · {dims}", if *p == MASTER { "gabarit".into() } else { format!("p. {}", self.doc.page_label(*p)) })).size(10.5).color(FAINT));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("↗").on_hover_text("Afficher dans l'explorateur").clicked() {
                            act = Some((*p, *i, 2));
                        }
                        if ui.small_button("⟲").on_hover_text("Remplacer le lien…").clicked() {
                            act = Some((*p, *i, 1));
                        }
                        if ui.small_button("◎").on_hover_text("Atteindre").clicked() {
                            act = Some((*p, *i, 0));
                        }
                    });
                });
            }
            if missing > 0 && pro::secondary(ui, "Rechercher les liens manquants dans un dossier…").clicked() {
                folder = true;
            }
        });
        if let Some((p, i, k)) = act {
            let path = links.iter().find(|l| l.0 == p && l.1 == i).map(|l| l.2.clone()).unwrap_or_default();
            match k {
                0 => {
                    (self.page, self.sel) = (p, Some(i));
                    self.picks.clear();
                }
                1 => {
                    if let Some(np) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif", "svg"]).pick_file() {
                        let fr = if p == MASTER { &mut self.doc.master.frames } else { &mut self.doc.pages[p].frames };
                        if let Some(Frame { kind: Kind::Image { path, .. }, .. }) = fr.get_mut(i) {
                            *path = np.to_string_lossy().into_owned();
                        }
                    }
                }
                _ => {
                    let _ = std::process::Command::new("explorer").arg(format!("/select,{path}")).spawn();
                }
            }
        }
        if folder && let Some(dir) = rfd::FileDialog::new().pick_folder() {
            let mut found = 0;
            for (_, f) in texts_mut(&mut self.doc) {
                if let Kind::Image { path, .. } = &mut f.kind
                    && !path.is_empty()
                    && !std::path::Path::new(path.as_str()).is_file()
                    && let Some(name) = std::path::Path::new(path.as_str()).file_name()
                {
                    let cand: PathBuf = dir.join(name);
                    if cand.is_file() {
                        *path = cand.to_string_lossy().into_owned();
                        found += 1;
                    }
                }
            }
            self.notice = Some((format!("{found} lien(s) rétabli(s)"), found > 0));
        }
    }

    fn ensure_layers(&mut self) {
        if self.doc.layers.is_empty() {
            self.doc.layers.push(Layer { name: "Calque 1".into(), visible: true, locked: false, print: true, color: [10, 132, 255, 255] });
        }
    }

    pub fn layers_panel(&mut self, ui: &mut Ui) {
        const COLORS: [[u8; 4]; 6] = [[10, 132, 255, 255], [255, 69, 58, 255], [48, 209, 88, 255], [255, 159, 10, 255], [191, 90, 242, 255], [100, 210, 255, 255]];
        let mut add = false;
        let mut act: Option<(usize, u8)> = None;
        let implicit = self.doc.layers.is_empty();
        let n = self.doc.layers.len().max(1);
        let counts: Vec<usize> = (0..n).map(|l| self.frames_ref().iter().filter(|f| f.fx.layer.min(n - 1) == l).count()).collect();
        pro::dock_panel(ui, "layers", "Calques", |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for l in (0..n).rev() {
                let (name, vis, lock, print, col) = match self.doc.layers.get(l) {
                    Some(x) => (x.name.clone(), x.visible, x.locked, x.print, x.color),
                    None => ("Calque 1".into(), true, false, true, COLORS[0]),
                };
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                let p = ui.painter();
                if self.layer == l {
                    p.rect_filled(r, CornerRadius::same(6), BLUE.gamma_multiply(0.28));
                } else if resp.hovered() {
                    p.rect_filled(r, CornerRadius::same(6), Color32::from_white_alpha(8));
                }
                p.rect_filled(egui::Rect::from_center_size(r.left_center() + vec2(58.0, 0.0), vec2(4.0, 16.0)), CornerRadius::same(2), Color32::from_rgb(col[0], col[1], col[2]));
                p.text(r.left_center() + vec2(68.0, 0.0), Align2::LEFT_CENTER, &name, FontId::proportional(12.5), if vis { INK } else { FAINT });
                p.text(r.right_center() - vec2(8.0, 0.0), Align2::RIGHT_CENTER, counts[l].to_string(), FontId::proportional(11.0), FAINT);
                for (k, (glyph, on, tip)) in [("👁", vis, "Afficher / masquer"), ("🔒", lock, "Verrouiller"), ("🖨", print, "Imprimer ce calque")].into_iter().enumerate() {
                    let br = egui::Rect::from_center_size(r.left_center() + vec2(10.0 + k as f32 * 16.0, 0.0), Vec2::splat(16.0));
                    let b = ui.interact(br, ui.make_persistent_id(("layer", l, k)), Sense::click()).on_hover_text(tip);
                    ui.painter().text(br.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(10.0), if on { INK } else { FAINT.gamma_multiply(0.6) });
                    if b.clicked() {
                        act = Some((l, k as u8));
                    }
                }
                if resp.clicked() {
                    act = Some((l, 9));
                }
                resp.context_menu(|ui| {
                    for (k, t) in [(3u8, "Monter"), (4, "Descendre"), (5, "Placer la sélection ici"), (6, "Sélectionner ses blocs"), (7, "Supprimer le calque")] {
                        if ui.button(t).clicked() {
                            act = Some((l, k));
                            ui.close();
                        }
                    }
                });
            }
            if let Some(x) = self.doc.layers.get_mut(self.layer) {
                ui.add_space(4.0);
                ui.add(egui::TextEdit::singleline(&mut x.name).desired_width(f32::INFINITY));
            }
            ui.horizontal(|ui| {
                if pro::ghost(ui, "+ Nouveau calque").clicked() {
                    add = true;
                }
                if pro::ghost(ui, "Placer la sélection").on_hover_text("Déplace les blocs sélectionnés sur le calque actif").clicked() {
                    act = Some((self.layer, 5));
                }
            });
            ui.label(RichText::new("Clic droit : ordre, sélection, suppression. Le calque du haut passe devant.").size(11.0).color(FAINT));
        });
        if add {
            self.ensure_layers();
            let k = self.doc.layers.len();
            self.doc.layers.push(Layer { name: format!("Calque {}", k + 1), visible: true, locked: false, print: true, color: COLORS[k % COLORS.len()] });
            self.layer = k;
        }
        let Some((l, k)) = act else { return };
        if k < 9 {
            self.ensure_layers();
        }
        let _ = implicit;
        let n = self.doc.layers.len();
        let remap = |doc: &mut Doc, f: &dyn Fn(usize) -> usize| {
            for (_, fr) in texts_mut(doc) {
                fr.fx.layer = f(fr.fx.layer.min(n - 1));
            }
        };
        match k {
            0 => self.doc.layers[l].visible = !self.doc.layers[l].visible,
            1 => self.doc.layers[l].locked = !self.doc.layers[l].locked,
            2 => self.doc.layers[l].print = !self.doc.layers[l].print,
            3 | 4 => {
                let j = if k == 3 { l + 1 } else { l.wrapping_sub(1) };
                if j < n {
                    self.doc.layers.swap(l, j);
                    remap(&mut self.doc, &|x| if x == l { j } else if x == j { l } else { x });
                    self.layer = j;
                }
            }
            5 => {
                for i in self.selection() {
                    self.frames()[i].fx.layer = l;
                }
            }
            6 => {
                self.picks = (0..self.frames_ref().len()).filter(|i| self.frames_ref()[*i].fx.layer.min(n - 1) == l).collect();
                self.sel = self.picks.last().copied();
            }
            7 if n > 1 => {
                self.doc.layers.remove(l);
                remap(&mut self.doc, &|x| if x > l { x - 1 } else if x == l { l.saturating_sub(1) } else { x });
                self.layer = self.layer.min(n - 2);
            }
            9 => self.layer = l,
            _ => {}
        }
    }

    pub fn styles_panels(&mut self, ui: &mut Ui) {
        let mut del_c = None;
        let mut new_c = false;
        pro::dock_panel(ui, "cstyles", "Styles de caractère", |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            if self.doc.char_styles.is_empty() {
                ui.label(RichText::new("Mise en forme à appliquer à quelques mots : dans la fenêtre d'écriture, sélectionne le texte puis choisis le style.").size(12.0).color(FAINT));
            }
            for (k, s) in self.doc.char_styles.iter().enumerate() {
                ui.horizontal(|ui| {
                    let c = s.color.unwrap_or([230, 230, 235, 255]);
                    let mut t = RichText::new(&s.name).size(12.5).color(Color32::from_rgb(c[0], c[1], c[2]));
                    if s.bold {
                        t = t.strong();
                    }
                    if s.italic {
                        t = t.italics();
                    }
                    ui.label(t);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("×").clicked() {
                            del_c = Some(k);
                        }
                    });
                });
            }
            if pro::ghost(ui, "+ Nouveau style de caractère").clicked() {
                new_c = true;
            }
        });
        if let Some(k) = del_c {
            self.doc.char_styles.remove(k);
        }
        if new_c {
            self.char_sheet = Some(CharStyle { name: format!("Caractère {}", self.doc.char_styles.len() + 1), bold: true, italic: false, underline: false, strike: false, caps: false, color: None, scale: 1.0 });
        }
        let mut del_o = None;
        let mut apply_o = None;
        let has = self.sel.is_some();
        pro::dock_panel(ui, "ostyles", "Styles d'objet", |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            if self.doc.obj_styles.is_empty() {
                ui.label(RichText::new("Enregistre l'aspect d'un bloc (couleurs, contour, effets, angles) pour le réappliquer.").size(12.0).color(FAINT));
            }
            for (k, s) in self.doc.obj_styles.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.add_enabled(has, egui::Button::new(RichText::new(&s.name).size(12.5)).frame(false)).on_hover_text("Appliquer à la sélection").clicked() {
                        apply_o = Some(k);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("×").clicked() {
                            del_o = Some(k);
                        }
                    });
                });
            }
        });
        if let Some(k) = del_o {
            self.doc.obj_styles.remove(k);
        }
        if let Some(k) = apply_o {
            self.apply_obj_style(k);
        }
    }

    pub fn apply_obj_style(&mut self, k: usize) {
        let Some(src) = self.doc.obj_styles.get(k).map(|s| s.frame.clone()) else { return };
        for i in self.selection() {
            copy_look(&src, &mut self.frames()[i]);
        }
    }

    pub fn wrap_selection(&mut self, open: &str, close: &str) {
        let Some(i) = self.edit_text else { return };
        let (a, b) = (self.edit_cursor.0.min(self.edit_cursor.1), self.edit_cursor.0.max(self.edit_cursor.1));
        if let Kind::Text { text, .. } | Kind::TextPath { text, .. } = &mut self.frames()[i].kind {
            let n = text.chars().count();
            let (a, b) = if a == b { (0, n) } else { (a.min(n), b.min(n)) };
            let ba = text.char_indices().nth(a).map_or(text.len(), |c| c.0);
            let bb = text.char_indices().nth(b).map_or(text.len(), |c| c.0);
            text.insert_str(bb, close);
            text.insert_str(ba, open);
        }
    }

    pub fn map_selection(&mut self, f: impl Fn(&str) -> String) {
        let Some(i) = self.edit_text else { return };
        let (a, b) = (self.edit_cursor.0.min(self.edit_cursor.1), self.edit_cursor.0.max(self.edit_cursor.1));
        if let Kind::Text { text, .. } | Kind::TextPath { text, .. } = &mut self.frames()[i].kind {
            let n = text.chars().count();
            let (a, b) = if a == b { (0, n) } else { (a.min(n), b.min(n)) };
            let ba = text.char_indices().nth(a).map_or(text.len(), |c| c.0);
            let bb = text.char_indices().nth(b).map_or(text.len(), |c| c.0);
            let mid = f(&text[ba..bb]);
            text.replace_range(ba..bb, &mid);
        }
    }

    pub fn extra_sheets(&mut self, ctx: &egui::Context) {
        if let Some(mut st) = self.find.take() {
            let (mut all, mut next, mut close) = (false, false, false);
            let mut styles: Vec<String> = STYLES.iter().map(|s| s.0.to_string()).collect();
            styles.extend(self.doc.styles.iter().map(|s| s.name.clone()));
            let keep = pro::sheet(ctx, "find", 420.0, |ui| {
                ui.label(RichText::new("Rechercher et remplacer").font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(10.0);
                ui.label(RichText::new("Rechercher").size(12.5).color(MUTED));
                let r = ui.add(egui::TextEdit::singleline(&mut st.find).desired_width(f32::INFINITY));
                if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    next = true;
                }
                ui.label(RichText::new("Remplacer par").size(12.5).color(MUTED));
                ui.add(egui::TextEdit::singleline(&mut st.with).desired_width(f32::INFINITY));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut st.case, "Respecter la casse");
                    ui.checkbox(&mut st.word, "Mot entier");
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Dans").size(12.5).color(MUTED));
                    pro::segmented(ui, &mut st.scope, &["Document", "Page", "Bloc sélectionné"]);
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Style de paragraphe").size(12.5).color(MUTED));
                    egui::ComboBox::from_id_salt("fstyle").selected_text(if st.style.is_empty() { "Tous" } else { st.style.as_str() }).show_ui(ui, |ui| {
                        ui.selectable_value(&mut st.style, String::new(), "Tous");
                        for s in &styles {
                            ui.selectable_value(&mut st.style, s.clone(), s);
                        }
                    });
                });
                ui.label(RichText::new("Astuce : \\t n'est pas interprété, colle une vraie tabulation si besoin.").size(11.0).color(FAINT));
                if let Some(r) = &st.result {
                    ui.label(RichText::new(r).size(12.5).color(GREEN));
                }
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Tout remplacer").clicked() {
                        all = true;
                    }
                    if pro::secondary(ui, "Suivant").clicked() {
                        next = true;
                    }
                    if pro::secondary(ui, "Fermer").clicked() {
                        close = true;
                    }
                });
            });
            if all {
                let n = self.replace_all(&st);
                st.result = Some(format!("{n} remplacement(s)"));
            }
            if next {
                self.find_next(&mut st);
            }
            if keep && !close {
                self.find = Some(st);
            }
        }
        if let Some(mut o) = self.step.take() {
            let (mut go, mut close) = (false, false);
            let keep = pro::sheet(ctx, "step", 380.0, |ui| {
                ui.label(RichText::new("Dupliquer et décaler").font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(10.0);
                ui.checkbox(&mut o.grid, "Créer une grille");
                egui::Grid::new("stepg").num_columns(2).spacing(vec2(10.0, 6.0)).show(ui, |ui| {
                    if o.grid {
                        ui.label("Lignes");
                        ui.add(egui::DragValue::new(&mut o.rows).range(1..=50));
                        ui.end_row();
                        ui.label("Colonnes");
                        ui.add(egui::DragValue::new(&mut o.cols).range(1..=50));
                        ui.end_row();
                        ui.label("Gouttière horizontale");
                    } else {
                        ui.label("Nombre de copies");
                        ui.add(egui::DragValue::new(&mut o.count).range(1..=200));
                        ui.end_row();
                        ui.label("Décalage horizontal");
                    }
                    ui.add(egui::DragValue::new(&mut o.dx).speed(0.5).suffix(" pt"));
                    ui.end_row();
                    ui.label(if o.grid { "Gouttière verticale" } else { "Décalage vertical" });
                    ui.add(egui::DragValue::new(&mut o.dy).speed(0.5).suffix(" pt"));
                    ui.end_row();
                    if !o.grid {
                        ui.label("Rotation par copie");
                        ui.add(egui::DragValue::new(&mut o.rot).range(-180.0..=180.0).suffix("°"));
                        ui.end_row();
                    }
                });
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Créer").clicked() {
                        go = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        close = true;
                    }
                });
            });
            if go {
                self.step_repeat(&o);
            } else if keep && !close {
                self.step = Some(o);
            }
        }
        if let Some(mut o) = self.toc.take() {
            let (mut go, mut close) = (false, false);
            let keep = pro::sheet(ctx, "toc", 400.0, |ui| {
                ui.label(RichText::new("Table des matières").font(semibold(18.0)).color(INK_STRONG));
                ui.label(RichText::new("Liste les blocs portant ces styles de paragraphe, avec leur numéro de page et des points de conduite.").size(12.0).color(MUTED));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Titre").size(12.5).color(MUTED));
                    ui.add(egui::TextEdit::singleline(&mut o.title).desired_width(f32::INFINITY));
                });
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    for (name, on) in &mut o.styles {
                        ui.checkbox(on, name.as_str());
                    }
                });
                ui.checkbox(&mut o.new_page, "Sur une nouvelle page au début du document");
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Créer").clicked() {
                        go = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        close = true;
                    }
                });
            });
            if go {
                self.make_toc(&o);
            } else if keep && !close {
                self.toc = Some(o);
            }
        }
        if self.sections_sheet {
            let mut done = false;
            let cur = if self.page == MASTER { 0 } else { self.page };
            let keep = pro::sheet(ctx, "sections", 460.0, |ui| {
                ui.label(RichText::new("Numérotation et sections").font(semibold(18.0)).color(INK_STRONG));
                ui.label(RichText::new("Chaque section repart d'un numéro, avec son style et son préfixe. {section} affiche son nom.").size(12.0).color(MUTED));
                ui.add_space(8.0);
                let mut remove = None;
                let n = self.doc.pages.len();
                for (k, s) in self.doc.sections.iter_mut().enumerate() {
                    egui::Frame::new().fill(Color32::from_white_alpha(6)).corner_radius(CornerRadius::same(8)).inner_margin(8).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Dès la page").size(12.0).color(MUTED));
                            let mut p = s.start + 1;
                            if ui.add(egui::DragValue::new(&mut p).range(1..=n)).changed() {
                                s.start = p - 1;
                            }
                            ui.add(egui::TextEdit::singleline(&mut s.name).hint_text("Nom").desired_width(130.0));
                            if ui.small_button("×").clicked() {
                                remove = Some(k);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut s.prefix).hint_text("Préfixe").desired_width(60.0));
                            egui::ComboBox::from_id_salt(("numst", k)).selected_text(NUMBERING[s.style as usize % NUMBERING.len()]).show_ui(ui, |ui| {
                                for (j, t) in NUMBERING.iter().enumerate() {
                                    ui.selectable_value(&mut s.style, j as u8, *t);
                                }
                            });
                            ui.label(RichText::new("Début").size(12.0).color(MUTED));
                            ui.add(egui::DragValue::new(&mut s.first).range(1..=9999));
                        });
                    });
                }
                if let Some(k) = remove {
                    self.doc.sections.remove(k);
                }
                if pro::ghost(ui, &format!("+ Nouvelle section à la page {}", cur + 1)).clicked() && !self.doc.sections.iter().any(|s| s.start == cur) {
                    self.doc.sections.push(Section { start: cur, name: format!("Section {}", self.doc.sections.len() + 1), prefix: String::new(), first: 1, style: 0 });
                }
                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Terminé").clicked() {
                        done = true;
                    }
                });
            });
            if done || !keep {
                self.sections_sheet = false;
            }
        }
        if let Some(mut cs) = self.char_sheet.take() {
            let (mut ok, mut close) = (false, false);
            let sw = self.doc.swatches.clone();
            let keep = pro::sheet(ctx, "cstyle", 380.0, |ui| {
                ui.label(RichText::new("Style de caractère").font(semibold(18.0)).color(INK_STRONG));
                ui.add_space(8.0);
                ui.add(egui::TextEdit::singleline(&mut cs.name).desired_width(f32::INFINITY));
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut cs.bold, RichText::new(" G ").strong());
                    ui.toggle_value(&mut cs.italic, RichText::new(" I ").italics());
                    ui.toggle_value(&mut cs.underline, RichText::new(" S ").underline());
                    ui.toggle_value(&mut cs.strike, RichText::new(" B ").strikethrough());
                    ui.toggle_value(&mut cs.caps, "Petites cap.");
                });
                let mut on = cs.color.is_some();
                if ui.checkbox(&mut on, "Couleur").changed() {
                    cs.color = on.then_some([255, 112, 52, 255]);
                }
                if let Some(c) = &mut cs.color {
                    color_row(ui, "Couleur", c, &sw);
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Taille relative").size(12.5).color(MUTED));
                    let mut pct = cs.scale * 100.0;
                    if ui.add(egui::DragValue::new(&mut pct).range(30.0..=400.0).suffix(" %")).changed() {
                        cs.scale = pct / 100.0;
                    }
                });
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Enregistrer").clicked() {
                        ok = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        close = true;
                    }
                });
            });
            if ok && !cs.name.trim().is_empty() {
                self.doc.char_styles.retain(|c| c.name != cs.name);
                self.doc.char_styles.push(cs);
            } else if keep && !close {
                self.char_sheet = Some(cs);
            }
        }
        if let Some(mut name) = self.obj_name.take() {
            let (mut ok, mut close) = (false, false);
            let keep = pro::sheet(ctx, "ostyle", 340.0, |ui| {
                ui.label(RichText::new("Nouveau style d'objet").font(semibold(17.0)).color(INK_STRONG));
                ui.label(RichText::new("Couleurs, contour, opacité, angles et effets du bloc sélectionné.").size(12.0).color(MUTED));
                ui.add_space(8.0);
                let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY));
                if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    ok = true;
                }
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pro::primary(ui, "Créer").clicked() {
                        ok = true;
                    }
                    if pro::secondary(ui, "Annuler").clicked() {
                        close = true;
                    }
                });
            });
            if ok && let Some(i) = self.sel {
                let frame = self.frames_ref()[i].clone();
                self.doc.obj_styles.retain(|s| s.name != name);
                self.doc.obj_styles.push(ObjStyle { name, frame });
            } else if keep && !close {
                self.obj_name = Some(name);
            }
        }
        if let Some(i) = self.note_edit {
            let mut close = false;
            let keep = match self.frames().get_mut(i) {
                Some(f) => pro::sheet(ctx, "note", 380.0, |ui| {
                    ui.label(RichText::new("Note").font(semibold(18.0)).color(INK_STRONG));
                    ui.label(RichText::new(format!("Sur « {} » · jamais imprimée ni exportée", f.title())).size(12.0).color(MUTED));
                    ui.add_space(8.0);
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut f.fx.note).desired_rows(5).desired_width(f32::INFINITY));
                    });
                    ui.add_space(10.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if pro::primary(ui, "Terminé").clicked() {
                            close = true;
                        }
                        if pro::secondary(ui, "Supprimer la note").clicked() {
                            f.fx.note.clear();
                            close = true;
                        }
                    });
                }),
                None => false,
            };
            if close || !keep {
                self.note_edit = None;
            }
        }
    }

    pub fn open_toc(&mut self) {
        let mut names: Vec<String> = STYLES.iter().map(|s| s.0.to_string()).collect();
        names.extend(self.doc.styles.iter().map(|s| s.name.clone()));
        self.toc = Some(TocOpts { title: "Sommaire".into(), styles: names.into_iter().map(|n| (n.clone(), n == "Titre 1" || n == "Titre 2")).collect(), new_page: false });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_replace_typo_ranges() {
        assert_eq!(replace_in("Chat chat CHAT achat", "chat", "chien", false, true), ("chien chien chien achat".into(), 3));
        assert_eq!(replace_in("Chat chat", "chat", "x", true, false).1, 1);
        assert_eq!(parse_range("1-3, 5", 6), vec![0, 1, 2, 4]);
        assert_eq!(parse_range("4-", 5), vec![3, 4]);
        assert_eq!(typo_fix("Il dit \"oui\"...  Bien !", 2), "Il dit «\u{a0}oui\u{a0}»...  Bien !");
        assert_eq!(typo_fix("a  b", 0), "a b");
        assert_eq!(typo_fix("Quoi ?", 4), "Quoi\u{202f}?");
        assert_eq!(case_fix("le petit chat", 2), "Le Petit Chat");
        assert_eq!(case_fix("BONJOUR. ÇA VA", 3), "Bonjour. Ça va");
    }
}
