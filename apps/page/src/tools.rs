use crate::model::{self, Frame, Kind, MASTER, Node};
use crate::{HANDLES, MM, PageApp};
use eframe::egui::{self, Align2, Color32, CornerRadius, Key, Modifiers, Painter, Pos2, Rect, RichText, Shape, Stroke, StrokeKind, Ui, pos2, vec2};
use studio_ui::pro::{FAINT, MUTED, semibold};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Select,
    Direct,
    PageTool,
    Gap,
    Text,
    TextPath,
    Table,
    Pen,
    AddPoint,
    DelPoint,
    ConvPoint,
    Pencil,
    Line,
    FrameRect,
    FrameEllipse,
    FramePolygon,
    Rect,
    Ellipse,
    Polygon,
    Star,
    Scissors,
    FreeTransform,
    Rotate,
    Scale,
    Shear,
    Gradient,
    GradFeather,
    Note,
    Pipette,
    ColorTheme,
    Measure,
    Hand,
    Zoom,
}

use Tool::*;

pub const GROUPS: &[&[(Tool, &str, &str)]] = &[
    &[(Select, "Sélection", "V"), (Direct, "Sélection directe", "A")],
    &[(PageTool, "Page", "Maj+P"), (Gap, "Espace", "U")],
    &[(Text, "Texte", "T"), (TextPath, "Texte sur tracé", "Maj+T"), (Table, "Tableau", "J")],
    &[(Pen, "Plume", "P"), (AddPoint, "Ajout de point d'ancrage", "="), (DelPoint, "Suppression de point d'ancrage", "-"), (ConvPoint, "Conversion de point directionnel", "Maj+C")],
    &[(Pencil, "Crayon", "N")],
    &[(Line, "Trait", "\\")],
    &[(FrameRect, "Bloc rectangulaire", "F"), (FrameEllipse, "Bloc elliptique", "F"), (FramePolygon, "Bloc polygonal", "F")],
    &[(Rect, "Rectangle", "M"), (Ellipse, "Ellipse", "L"), (Polygon, "Polygone", "M"), (Star, "Étoile", "M")],
    &[(Scissors, "Ciseaux", "C")],
    &[(FreeTransform, "Transformation manuelle", "E"), (Rotate, "Rotation", "R"), (Scale, "Mise à l'échelle", "S"), (Shear, "Déformation", "O")],
    &[(Gradient, "Nuancier de dégradé", "G"), (GradFeather, "Contour progressif du dégradé", "Maj+G")],
    &[(Note, "Note", "")],
    &[(Pipette, "Pipette", "I"), (ColorTheme, "Thème de couleur", "Maj+I"), (Measure, "Mesure", "K")],
    &[(Hand, "Main", "H")],
    &[(Zoom, "Zoom", "Z")],
];

pub enum Drag {
    None,
    Move { last: [f32; 2], acc: [f32; 2], origin: [f32; 2], others: Vec<(usize, [f32; 2])> },
    Create { start: [f32; 2] },
    Resize { orig: [f32; 4], hx: f32, hy: f32 },
    Guide { index: usize },
    Node { k: usize, part: u8 },
    Content { last: [f32; 2] },
    PageSize { orig: [f32; 2] },
    Gap { a: Vec<usize>, b: Vec<usize>, vertical: bool, last: f32 },
    Pen { k: usize },
    Pencil,
    Rotate { start: f32, orig: f32 },
    Scale { orig: [f32; 4], d0: f32 },
    Shear { x0: f32, orig: f32 },
    Grad { start: [f32; 2], feather: bool },
    Measure,
    Pan,
}

pub fn tool_name(t: Tool) -> &'static str {
    GROUPS.iter().flat_map(|g| g.iter()).find(|x| x.0 == t).map_or("", |x| x.1)
}

fn key_of(s: &str) -> Option<(Key, bool)> {
    let (shift, k) = match s.strip_prefix("Maj+") {
        Some(k) => (true, k),
        None => (false, s),
    };
    let key = match k {
        "=" => Key::Equals,
        "-" => Key::Minus,
        "\\" => Key::Backslash,
        "" => return None,
        k => Key::from_name(k)?,
    };
    Some((key, shift))
}

pub fn pick_tool(current: Tool, pressed: &[(Key, bool)]) -> Option<Tool> {
    for g in GROUPS {
        for (t, _, s) in g.iter() {
            let Some(k) = key_of(s) else { continue };
            if !pressed.contains(&k) {
                continue;
            }
            if let Some(pos) = g.iter().position(|x| x.0 == current) {
                return Some(g[(pos + 1) % g.len()].0);
            }
            return Some(*t);
        }
    }
    None
}

pub fn hint(t: Tool) -> &'static str {
    match t {
        Select => "Clique un bloc · Maj+clic pour en ajouter · Alt+glisser duplique · double-clic pour écrire",
        Direct => "Déplace l'image dans son cadre ou les points d'un tracé",
        PageTool => "Glisse pour redimensionner la page affichée",
        Gap => "Glisse dans l'espace entre deux blocs pour le déplacer",
        Text => "Trace un bloc de texte",
        TextPath => "Clique une forme ou un tracé pour y poser du texte, ou trace un arc",
        Table => "Trace le tableau, puis règle lignes et colonnes à droite",
        Pen => "Clic : point d'angle · glisser : point lisse · clic sur le premier point pour fermer · Entrée pour finir",
        AddPoint => "Clique un segment du tracé sélectionné pour ajouter un point",
        DelPoint => "Clique un point d'ancrage pour le supprimer",
        ConvPoint => "Clique un point pour basculer angle / lisse, glisse pour tirer ses poignées",
        Pencil => "Dessine à main levée ; termine près du départ pour fermer le tracé",
        Line => "Trace un trait · Maj contraint à 45°",
        FrameRect | FrameEllipse | FramePolygon => "Trace un bloc d'image vide, double-clic pour placer une image",
        Rect | Ellipse => "Trace la forme · Maj garde les proportions",
        Scissors => "Clique un tracé ou une forme pour le couper à cet endroit",
        FreeTransform => "Poignées pour mettre à l'échelle, glisse hors du bloc pour pivoter",
        Rotate => "Glisse pour faire pivoter le bloc sélectionné · Maj par pas de 15°",
        Scale => "Glisse pour agrandir ou réduire le bloc depuis son centre",
        Shear => "Glisse horizontalement pour incliner le bloc",
        Gradient => "Glisse sur le bloc pour orienter son dégradé",
        GradFeather => "Glisse pour fondre le bloc vers la transparence",
        Note => "Clique un bloc pour y attacher une note (non imprimée)",
        Pipette => "Sélectionne un bloc, puis clique un autre bloc pour lui prendre son style",
        ColorTheme => "Clique une image ou un bloc : ses 5 couleurs dominantes rejoignent le nuancier",
        Measure => "Glisse pour mesurer une distance et un angle",
        Hand => "Glisse pour faire défiler · Espace maintenu marche avec tous les outils",
        Zoom => "Clic pour zoomer · Alt+clic pour dézoomer",
        Polygon | Star => "",
    }
}

pub fn path_frame(abs: &[Node]) -> (f32, f32, f32, f32, Vec<Node>) {
    let pts = abs.iter().flat_map(|n| [n.p, n.i, n.o]);
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in pts {
        x0 = x0.min(p[0]);
        y0 = y0.min(p[1]);
        x1 = x1.max(p[0]);
        y1 = y1.max(p[1]);
    }
    let (w, h) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
    let nm = |p: [f32; 2]| [(p[0] - x0) / w, (p[1] - y0) / h];
    (x0, y0, w, h, abs.iter().map(|n| Node { p: nm(n.p), i: nm(n.i), o: nm(n.o) }).collect())
}

fn abs_nodes(f: &Frame, nodes: &[Node]) -> Vec<Node> {
    let a = |p: [f32; 2]| [f.x + p[0] * f.w, f.y + p[1] * f.h];
    nodes.iter().map(|n| Node { p: a(n.p), i: a(n.i), o: a(n.o) }).collect()
}

fn ang(c: [f32; 2], p: [f32; 2]) -> f32 {
    (p[1] - c[1]).atan2(p[0] - c[0]).to_degrees()
}

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

impl PageApp {
    pub fn selection(&self) -> Vec<usize> {
        if self.picks.len() > 1 { self.picks.clone() } else { self.sel.into_iter().collect() }
    }

    pub fn hit(&self, p: [f32; 2], tol: f32) -> Option<usize> {
        let fr = self.frames_ref();
        self.doc.draw_order(fr).into_iter().rev().find(|&i| {
            let f = &fr[i];
            !f.locked && !f.hidden && self.doc.layer_shown(f.fx.layer, false) && !self.doc.layer_locked(f.fx.layer) && f.contains(p, tol)
        })
    }

    fn select_hit(&mut self, hit: Option<usize>, shift: bool) {
        let Some(i) = hit else {
            if !shift {
                self.sel = None;
                self.picks.clear();
            }
            return;
        };
        let g = self.frames_ref()[i].fx.group;
        let members: Vec<usize> = if g > 0 { (0..self.frames_ref().len()).filter(|k| self.frames_ref()[*k].fx.group == g).collect() } else { vec![i] };
        let mut cur = self.selection();
        if shift {
            if cur.contains(&i) {
                cur.retain(|k| !members.contains(k));
            } else {
                cur.extend(members);
            }
            self.sel = cur.last().copied();
            self.picks = if cur.len() > 1 { cur } else { Vec::new() };
        } else {
            if !cur.contains(&i) {
                self.picks = if members.len() > 1 { members } else { Vec::new() };
            }
            self.sel = Some(i);
        }
    }

    fn new_frame(&mut self, mut f: Frame) {
        f.fx.layer = self.layer;
        self.push_frame(f);
        self.picks.clear();
    }

    pub fn as_path(&mut self, i: usize) -> bool {
        let f = &mut self.frames()[i];
        let kind = match &f.kind {
            Kind::Path { .. } | Kind::TextPath { .. } => return true,
            Kind::Rect { fill, .. } | Kind::Ellipse { fill } | Kind::Polygon { fill, .. } => Kind::Path { nodes: f.kind.nodes(), closed: true, fill: Some(*fill), color: f.stroke.unwrap_or([20, 20, 24, 255]), width: if f.stroke.is_some() { f.stroke_w } else { 0.0 } },
            Kind::Line { color, width, .. } => Kind::Path { nodes: f.kind.nodes(), closed: false, fill: None, color: *color, width: *width },
            _ => return false,
        };
        f.kind = kind;
        f.stroke = None;
        true
    }

    pub fn renormalize(&mut self, i: usize) {
        let f = &self.frames_ref()[i];
        let (Kind::Path { nodes, .. } | Kind::TextPath { nodes, .. }) = &f.kind else { return };
        let abs = abs_nodes(f, nodes);
        let (x, y, w, h, n) = path_frame(&abs);
        let f = &mut self.frames()[i];
        (f.x, f.y, f.w, f.h) = (x, y, w, h);
        if let Kind::Path { nodes, .. } | Kind::TextPath { nodes, .. } = &mut f.kind {
            *nodes = n;
        }
    }

    pub fn commit_pen(&mut self, closed: bool) {
        let pen = std::mem::take(&mut self.pen);
        if pen.len() < 2 {
            return;
        }
        let (x, y, w, h, nodes) = path_frame(&pen);
        let fill = closed.then_some([255, 196, 0, 255]);
        self.new_frame(Frame::new(x, y, w, h, Kind::Path { nodes, closed, fill, color: [20, 20, 24, 255], width: 1.5 }));
    }

    fn node_at(&self, i: usize, p: [f32; 2], tol: f32, handles: bool) -> Option<(usize, u8)> {
        let f = &self.frames_ref()[i];
        let (Kind::Path { nodes, .. } | Kind::TextPath { nodes, .. }) = &f.kind else { return None };
        let abs = abs_nodes(f, nodes);
        for (k, n) in abs.iter().enumerate() {
            if dist(n.p, p) <= tol * 1.3 {
                return Some((k, 0));
            }
            if handles && nodes[k].smooth() {
                if dist(n.i, p) <= tol {
                    return Some((k, 1));
                }
                if dist(n.o, p) <= tol {
                    return Some((k, 2));
                }
            }
        }
        None
    }

    fn path_edit(&mut self, p: [f32; 2], tol: f32) {
        let target = self.sel.filter(|i| self.frames_ref().get(*i).is_some_and(|f| f.contains(p, tol * 3.0))).or_else(|| self.hit(p, tol));
        let Some(i) = target else { return };
        self.sel = Some(i);
        self.picks.clear();
        match self.tool {
            DelPoint => {
                if let Some((k, 0)) = self.node_at(i, p, tol, false)
                    && let Kind::Path { nodes, closed, .. } | Kind::TextPath { nodes, closed, .. } = &mut self.frames()[i].kind
                    && nodes.len() > if *closed { 3 } else { 2 }
                {
                    nodes.remove(k);
                }
            }
            ConvPoint => {
                if !self.as_path(i) {
                    return;
                }
                if let Some((k, _)) = self.node_at(i, p, tol, false)
                    && let Kind::Path { nodes, closed, .. } | Kind::TextPath { nodes, closed, .. } = &mut self.frames()[i].kind
                {
                    if nodes[k].smooth() {
                        let q = nodes[k].p;
                        nodes[k] = Node::corner(q[0], q[1]);
                    } else {
                        let c = *closed;
                        model::smooth_node(nodes, k, c);
                    }
                    self.drag = Drag::Node { k, part: 2 };
                }
            }
            _ => {
                if !self.as_path(i) {
                    self.notice = Some(("Cet outil agit sur les formes, traits et tracés".into(), false));
                    return;
                }
                let f = self.frames_ref()[i].clone();
                let (Kind::Path { nodes, closed, .. } | Kind::TextPath { nodes, closed, .. }) = &f.kind else { return };
                let q = [(p[0] - f.x) / f.w.max(0.01), (p[1] - f.y) / f.h.max(0.01)];
                let Some((seg, t, d)) = model::nearest_seg(nodes, *closed, f.w, f.h, q) else { return };
                if d > tol * 1.6 {
                    return;
                }
                let mut nodes = nodes.clone();
                let closed = *closed;
                model::insert_node(&mut nodes, seg, t);
                if self.tool == AddPoint {
                    if let Kind::Path { nodes: n, .. } | Kind::TextPath { nodes: n, .. } = &mut self.frames()[i].kind {
                        *n = nodes;
                    }
                    return;
                }
                let k = seg + 1;
                if closed {
                    let mut open: Vec<Node> = nodes[k..].to_vec();
                    open.extend_from_slice(&nodes[..=k]);
                    if let Kind::Path { nodes: n, closed: c, .. } | Kind::TextPath { nodes: n, closed: c, .. } = &mut self.frames()[i].kind {
                        *n = open;
                        *c = false;
                    }
                    self.notice = Some(("Tracé ouvert à cet endroit".into(), true));
                } else {
                    let mut second = self.frames_ref()[i].clone();
                    if let Kind::Path { nodes: n, .. } | Kind::TextPath { nodes: n, .. } = &mut self.frames()[i].kind {
                        *n = nodes[..=k].to_vec();
                    }
                    if let Kind::Path { nodes: n, .. } | Kind::TextPath { nodes: n, .. } = &mut second.kind {
                        *n = nodes[k..].to_vec();
                    }
                    self.renormalize(i);
                    self.frames().push(second);
                    let j = self.frames_ref().len() - 1;
                    self.renormalize(j);
                    self.notice = Some(("Tracé coupé en deux".into(), true));
                }
            }
        }
    }

    fn find_gap(&self, p: [f32; 2]) -> Option<Drag> {
        let fr = self.frames_ref();
        for vertical in [false, true] {
            let (a0, a1, b0) = if vertical { (1, 3, 0) } else { (0, 2, 1) };
            let r = |f: &Frame| [f.x, f.y, f.x + f.w, f.y + f.h];
            let cross = |f: &Frame| r(f)[b0] <= p[b0] && r(f)[b0 + 2] >= p[b0];
            let before = fr.iter().filter(|f| !f.locked && cross(f) && r(f)[a1] <= p[a0]).map(|f| r(f)[a1]).fold(f32::MIN, f32::max);
            let after = fr.iter().filter(|f| !f.locked && cross(f) && r(f)[a0] >= p[a0]).map(|f| r(f)[a0]).fold(f32::MAX, f32::min);
            if before > f32::MIN && after < f32::MAX {
                let a = (0..fr.len()).filter(|&i| cross(&fr[i]) && (r(&fr[i])[a1] - before).abs() < 0.5).collect();
                let b = (0..fr.len()).filter(|&i| cross(&fr[i]) && (r(&fr[i])[a0] - after).abs() < 0.5).collect();
                return Some(Drag::Gap { a, b, vertical, last: p[a0] });
            }
        }
        None
    }

    pub fn press(&mut self, p: [f32; 2], tol: f32, mods: Modifiers) {
        self.snap_lines.clear();
        let (shift, alt) = (mods.shift, mods.alt);
        match self.tool {
            ColorTheme => {
                let Some(i) = self.hit(p, tol) else { return };
                let f = &self.frames_ref()[i];
                let colors = match &f.kind {
                    Kind::Image { path, .. } => image::open(path).map(|im| theme_colors(&im.thumbnail(96, 96).to_rgba8(), 5)).unwrap_or_default(),
                    _ => f.stroke.into_iter().collect(),
                };
                let mut added = 0;
                for c in colors {
                    if !self.doc.swatches.contains(&c) {
                        self.doc.swatches.push(c);
                        added += 1;
                    }
                }
                self.notice = Some((if added > 0 { format!("{added} couleur(s) ajoutée(s) au nuancier") } else { "Aucune nouvelle couleur trouvée".into() }, added > 0));
            }
            Select | FreeTransform | Pipette => {
                if self.tool != Pipette
                    && let Some(i) = self.sel
                    && let Some(f) = self.frames_ref().get(i)
                    && !f.locked
                {
                    if let Some((hx, hy)) = HANDLES.iter().find(|(hx, hy)| (f.x + f.w * hx - p[0]).abs() <= tol && (f.y + f.h * hy - p[1]).abs() <= tol) {
                        self.drag = Drag::Resize { orig: [f.x, f.y, f.w, f.h], hx: *hx, hy: *hy };
                        return;
                    }
                    if self.tool == FreeTransform && !f.contains(p, tol) && f.contains(p, tol * 5.0) {
                        self.drag = Drag::Rotate { start: ang(f.center(), p), orig: f.rotation };
                        return;
                    }
                }
                let hit = self.hit(p, tol);
                if self.tool == Pipette {
                    match (self.sel, hit) {
                        (Some(to), Some(from)) if to != from => {
                            self.copy_style(from, to);
                            self.notice = Some(("Style copié".into(), true));
                        }
                        (None, Some(from)) => {
                            self.sel = Some(from);
                            self.notice = Some(("Bloc cible choisi : clique maintenant le bloc dont tu veux le style".into(), true));
                        }
                        _ => {}
                    }
                    return;
                }
                if hit.is_none()
                    && !self.preview
                    && let Some(g) = self.doc.guides.iter().position(|g| ((if g.vertical { p[0] } else { p[1] }) - g.pos).abs() <= tol)
                {
                    self.drag = Drag::Guide { index: g };
                    return;
                }
                if hit != self.sel {
                    self.edit_text = None;
                }
                self.select_hit(hit, shift);
                if hit.is_some()
                    && let Some(i) = self.sel
                {
                    let mut i = i;
                    if alt {
                        let sel = self.selection();
                        let base = self.frames_ref().len();
                        for k in &sel {
                            let mut f = self.frames_ref()[*k].clone();
                            if let Kind::Text { link, .. } = &mut f.kind {
                                *link = None;
                            }
                            self.frames().push(f);
                        }
                        self.picks = if sel.len() > 1 { (base..base + sel.len()).collect() } else { Vec::new() };
                        i = base + sel.iter().position(|k| *k == i).unwrap_or(0);
                        self.sel = Some(i);
                    }
                    let f = &self.frames_ref()[i];
                    let origin = [f.x, f.y];
                    let others = self.selection().into_iter().filter(|k| *k != i).map(|k| (k, [self.frames_ref()[k].x, self.frames_ref()[k].y])).collect();
                    self.drag = Drag::Move { last: p, acc: [0.0, 0.0], origin, others };
                }
            }
            Direct => {
                if let Some(i) = self.sel.filter(|i| *i < self.frames_ref().len()) {
                    if let Some((k, part)) = self.node_at(i, p, tol, true) {
                        self.drag = Drag::Node { k, part };
                        return;
                    }
                    if matches!(self.frames_ref()[i].kind, Kind::Image { .. }) && self.frames_ref()[i].contains(p, 0.0) {
                        self.drag = Drag::Content { last: p };
                        return;
                    }
                }
                let hit = self.hit(p, tol);
                self.sel = hit;
                self.picks.clear();
                if let Some(i) = hit {
                    let f = &self.frames_ref()[i];
                    self.drag = if matches!(f.kind, Kind::Image { .. }) { Drag::Content { last: p } } else { Drag::Move { last: p, acc: [0.0, 0.0], origin: [f.x, f.y], others: Vec::new() } };
                }
            }
            PageTool => {
                if self.page != MASTER {
                    let (w, h) = self.doc.page_size(self.page);
                    self.drag = Drag::PageSize { orig: [w, h] };
                }
            }
            Gap => match self.find_gap(p) {
                Some(d) => self.drag = d,
                None => self.notice = Some(("Aucun espace entre deux blocs à cet endroit".into(), false)),
            },
            Pen => {
                if self.pen.len() >= 2 && dist(p, self.pen[0].p) <= tol * 1.5 {
                    self.commit_pen(true);
                    self.tool = Select;
                    return;
                }
                self.pen.push(Node::corner(p[0], p[1]));
                self.drag = Drag::Pen { k: self.pen.len() - 1 };
            }
            AddPoint | DelPoint | ConvPoint | Scissors => self.path_edit(p, tol),
            Pencil => {
                self.pencil = vec![p];
                self.drag = Drag::Pencil;
            }
            Rotate | Scale | Shear | Gradient | GradFeather => {
                let Some(i) = self.sel.filter(|i| *i < self.frames_ref().len()) else {
                    self.sel = self.hit(p, tol);
                    return;
                };
                let f = &self.frames_ref()[i];
                self.drag = match self.tool {
                    Rotate => Drag::Rotate { start: ang(f.center(), p), orig: f.rotation },
                    Scale => Drag::Scale { orig: [f.x, f.y, f.w, f.h], d0: dist(f.center(), p).max(1.0) },
                    Shear => Drag::Shear { x0: p[0], orig: f.fx.skew },
                    t => Drag::Grad { start: p, feather: t == GradFeather },
                };
            }
            Note => match self.hit(p, tol) {
                Some(i) => {
                    self.sel = Some(i);
                    self.note_edit = Some(i);
                }
                None => self.notice = Some(("Clique un bloc pour y attacher une note".into(), false)),
            },
            Measure => {
                self.measure_line = Some((p, p));
                self.drag = Drag::Measure;
            }
            Hand | Zoom => self.drag = Drag::Pan,
            TextPath => {
                if let Some(i) = self.hit(p, tol)
                    && matches!(self.frames_ref()[i].kind, Kind::Path { .. } | Kind::Ellipse { .. } | Kind::Line { .. } | Kind::Rect { .. } | Kind::Polygon { .. })
                {
                    let f = self.frames_ref()[i].clone();
                    let kind = Kind::TextPath { text: "Texte sur tracé".into(), size: 18.0, family: "Segoe UI".into(), bold: false, italic: false, color: [20, 20, 24, 255], tracking: 0.0, nodes: f.kind.nodes(), closed: f.kind.closed(), offset: 0.0, show_path: false };
                    self.new_frame(Frame { kind, name: String::new(), ..f });
                    self.edit_text = self.sel;
                    self.tool = Select;
                    return;
                }
                let k = 0.5523 * 0.5;
                let nodes = vec![Node { p: [0.0, 1.0], i: [0.0, 1.0], o: [0.0, 1.0 - 2.0 * k] }, Node { p: [0.5, 0.0], i: [0.5 - k, 0.0], o: [0.5 + k, 0.0] }, Node { p: [1.0, 1.0], i: [1.0, 1.0 - 2.0 * k], o: [1.0, 1.0] }];
                let kind = Kind::TextPath { text: "Texte sur tracé".into(), size: 18.0, family: "Segoe UI".into(), bold: false, italic: false, color: [20, 20, 24, 255], tracking: 0.0, nodes, closed: false, offset: 0.0, show_path: false };
                self.new_frame(Frame::new(p[0], p[1], 0.0, 0.0, kind));
                self.drag = Drag::Create { start: p };
            }
            tool => {
                let img = |shape: u8, sides: u32| {
                    let mut k = Kind::image("", 0.0);
                    if let Kind::Image { shape: s, sides: n, .. } = &mut k {
                        (*s, *n) = (shape, sides);
                    }
                    k
                };
                let kind = match tool {
                    Text => Kind::text("", 12.0, false),
                    Table => Kind::table(4, 3),
                    Line => Kind::line([20, 20, 24, 255], 1.5),
                    FrameRect => img(0, 6),
                    FrameEllipse => img(1, 6),
                    FramePolygon => img(2, self.polygon_sides),
                    Ellipse => Kind::Ellipse { fill: [255, 120, 60, 255] },
                    Polygon => Kind::Polygon { sides: self.polygon_sides, inner: 1.0, fill: [255, 196, 0, 255] },
                    Star => Kind::Polygon { sides: self.star_points, inner: self.polygon_inner, fill: [255, 196, 0, 255] },
                    _ => Kind::Rect { fill: [255, 120, 60, 255], radius: 0.0 },
                };
                self.new_frame(Frame::new(p[0], p[1], 0.0, 0.0, kind));
                self.drag = Drag::Create { start: p };
            }
        }
    }

    pub fn drag_to(&mut self, p: [f32; 2], tol: f32, shift: bool) {
        if let Drag::Guide { index } = self.drag {
            if let Some(g) = self.doc.guides.get_mut(index) {
                g.pos = if g.vertical { p[0] } else { p[1] };
            }
            return;
        }
        match &mut self.drag {
            Drag::Pen { k } => {
                let k = *k;
                if let Some(n) = self.pen.get_mut(k) {
                    n.o = p;
                    n.i = [2.0 * n.p[0] - p[0], 2.0 * n.p[1] - p[1]];
                }
                return;
            }
            Drag::Pencil => {
                if self.pencil.last().is_none_or(|l| dist(*l, p) >= 2.0) {
                    self.pencil.push(p);
                }
                return;
            }
            Drag::Measure => {
                if let Some(m) = &mut self.measure_line {
                    m.1 = p;
                }
                return;
            }
            Drag::PageSize { orig } => {
                let mut s = [p[0].max(36.0), p[1].max(36.0)];
                if shift {
                    s[1] = s[0] * orig[1] / orig[0].max(1.0);
                }
                let s = [(s[0] / MM).round() * MM, (s[1] / MM).round() * MM];
                if self.page != MASTER {
                    self.doc.pages[self.page].size = Some(s);
                }
                return;
            }
            Drag::Gap { a, b, vertical, last } => {
                let v = if *vertical { p[1] } else { p[0] };
                let d = v - *last;
                *last = v;
                let (a, b, vert) = (a.clone(), b.clone(), *vertical);
                for k in a {
                    let f = &mut self.frames()[k];
                    if vert { f.h = (f.h + d).max(4.0) } else { f.w = (f.w + d).max(4.0) }
                }
                for k in b {
                    let f = &mut self.frames()[k];
                    if vert {
                        f.y += d;
                        f.h = (f.h - d).max(4.0);
                    } else {
                        f.x += d;
                        f.w = (f.w - d).max(4.0);
                    }
                }
                return;
            }
            _ => {}
        }
        let Some(i) = self.sel.filter(|i| *i < self.frames_ref().len()) else { return };
        let (fw, fh) = (self.frames_ref()[i].w, self.frames_ref()[i].h);
        self.snap_lines.clear();
        match &mut self.drag {
            Drag::Move { last, acc, origin, others } => {
                acc[0] += p[0] - last[0];
                acc[1] += p[1] - last[1];
                *last = p;
                let mut t = [origin[0] + acc[0], origin[1] + acc[1]];
                if shift {
                    if acc[0].abs() > acc[1].abs() {
                        t[1] = origin[1];
                    } else {
                        t[0] = origin[0];
                    }
                }
                let (origin, others) = (*origin, others.clone());
                let (x, lx) = self.snap_value(t[0], fw, 0, tol, Some(i));
                let (y, ly) = self.snap_value(t[1], fh, 1, tol, Some(i));
                self.snap_lines.extend(lx.map(|v| (true, v)).into_iter().chain(ly.map(|v| (false, v))));
                let f = &mut self.frames()[i];
                f.x = x;
                f.y = y;
                for (k, o) in others {
                    if let Some(f) = self.frames().get_mut(k) {
                        f.x = o[0] + x - origin[0];
                        f.y = o[1] + y - origin[1];
                    }
                }
                return;
            }
            Drag::Node { k, part } => {
                let (k, part) = (*k, *part);
                let f = &mut self.frames()[i];
                let q = [(p[0] - f.x) / f.w.max(0.01), (p[1] - f.y) / f.h.max(0.01)];
                if let Kind::Path { nodes, .. } | Kind::TextPath { nodes, .. } = &mut f.kind
                    && let Some(n) = nodes.get_mut(k)
                {
                    match part {
                        0 => {
                            let d = [q[0] - n.p[0], q[1] - n.p[1]];
                            for v in [&mut n.p, &mut n.i, &mut n.o] {
                                v[0] += d[0];
                                v[1] += d[1];
                            }
                        }
                        1 => {
                            n.i = q;
                            if !shift {
                                n.o = [2.0 * n.p[0] - q[0], 2.0 * n.p[1] - q[1]];
                            }
                        }
                        _ => {
                            n.o = q;
                            if !shift {
                                n.i = [2.0 * n.p[0] - q[0], 2.0 * n.p[1] - q[1]];
                            }
                        }
                    }
                }
                return;
            }
            Drag::Content { last } => {
                let d = [p[0] - last[0], p[1] - last[1]];
                *last = p;
                if let Kind::Image { ox, oy, .. } = &mut self.frames()[i].kind {
                    *ox += d[0];
                    *oy += d[1];
                }
                return;
            }
            Drag::Rotate { start, orig } => {
                let (start, orig) = (*start, *orig);
                let f = &mut self.frames()[i];
                let mut r = orig + ang(f.center(), p) - start;
                if shift {
                    r = (r / 15.0).round() * 15.0;
                }
                f.rotation = (r + 540.0).rem_euclid(360.0) - 180.0;
                return;
            }
            Drag::Scale { orig, d0 } => {
                let (o, d0) = (*orig, *d0);
                let c = [o[0] + o[2] / 2.0, o[1] + o[3] / 2.0];
                let s = (dist(c, p) / d0).clamp(0.02, 50.0);
                let f = &mut self.frames()[i];
                (f.w, f.h) = (o[2] * s, o[3] * s);
                (f.x, f.y) = (c[0] - f.w / 2.0, c[1] - f.h / 2.0);
                return;
            }
            Drag::Shear { x0, orig } => {
                let (x0, orig) = (*x0, *orig);
                let f = &mut self.frames()[i];
                f.fx.skew = (orig + ((p[0] - x0) / f.h.max(1.0)).atan().to_degrees()).clamp(-70.0, 70.0).round();
                return;
            }
            Drag::Grad { start, feather } => {
                let (start, feather) = (*start, *feather);
                self.grad_line = Some((start, p));
                if dist(start, p) < 2.0 {
                    return;
                }
                let a = ang(start, p).round();
                let sw = self.doc.swatches.first().copied().unwrap_or([10, 132, 255, 255]);
                let f = &mut self.frames()[i];
                if feather {
                    f.fx.gfeather = true;
                    f.fx.gf_angle = a;
                    f.fx.gf_start = 0.0;
                    f.fx.gf_end = 1.0;
                } else {
                    if f.fill2.is_none() {
                        f.fill2 = Some(if matches!(f.kind, Kind::Text { .. }) { [255, 255, 255, 255] } else { sw });
                    }
                    if let Kind::Text { background, .. } = &mut f.kind
                        && background.is_none()
                    {
                        *background = Some([245, 245, 247, 255]);
                    }
                    f.fx.radial = false;
                    f.grad_angle = a;
                }
                return;
            }
            _ => {}
        }
        let (qx, lx) = self.snap_value(p[0], 0.0, 0, tol, Some(i));
        let (qy, ly) = self.snap_value(p[1], 0.0, 1, tol, Some(i));
        self.snap_lines.extend(lx.map(|v| (true, v)).into_iter().chain(ly.map(|v| (false, v))));
        let mut q = [qx, qy];
        match self.drag {
            Drag::Create { start } => {
                let is_line = matches!(self.frames_ref()[i].kind, Kind::Line { .. });
                if shift && !is_line {
                    let d = (q[0] - start[0]).abs().max((q[1] - start[1]).abs());
                    q = [start[0] + d * (q[0] - start[0]).signum(), start[1] + d * (q[1] - start[1]).signum()];
                }
                let (dx, dy) = (q[0] - start[0], q[1] - start[1]);
                let f = &mut self.frames()[i];
                f.x = start[0].min(q[0]);
                f.y = start[1].min(q[1]);
                f.w = dx.abs();
                f.h = dy.abs();
                if let Kind::Line { dir, .. } = &mut f.kind {
                    let a = dy.atan2(dx).to_degrees().abs();
                    let flat = dy.abs() < dx.abs() * 0.2 || (shift && !(22.5..157.5).contains(&a));
                    let tall = dx.abs() < dy.abs() * 0.2 || (shift && (67.5..112.5).contains(&a));
                    *dir = if flat {
                        f.y = start[1] - 3.0;
                        f.h = 6.0;
                        0
                    } else if tall {
                        f.x = start[0] - 3.0;
                        f.w = 6.0;
                        1
                    } else if dx * dy > 0.0 {
                        2
                    } else {
                        3
                    };
                    if shift && *dir > 1 {
                        f.h = f.w;
                        if dy < 0.0 {
                            f.y = start[1] - f.w;
                        }
                    }
                }
            }
            Drag::Resize { orig, hx, hy } => {
                let (mut x0, mut y0, mut x1, mut y1) = (orig[0], orig[1], orig[0] + orig[2], orig[1] + orig[3]);
                if hx == 0.0 {
                    x0 = q[0];
                } else if hx == 1.0 {
                    x1 = q[0];
                }
                if hy == 0.0 {
                    y0 = q[1];
                } else if hy == 1.0 {
                    y1 = q[1];
                }
                if (shift || self.tool == FreeTransform) && hx != 0.5 && hy != 0.5 && orig[3] > 0.0 {
                    let ratio = orig[2] / orig[3];
                    let h = (x1 - x0).abs() / ratio;
                    if hy == 0.0 {
                        y0 = y1 - h * (y1 - y0).signum();
                    } else {
                        y1 = y0 + h * (y1 - y0).signum();
                    }
                }
                let f = &mut self.frames()[i];
                f.x = x0.min(x1);
                f.y = y0.min(y1);
                f.w = (x1 - x0).abs().max(1.0);
                f.h = (y1 - y0).abs().max(1.0);
            }
            _ => {}
        }
    }

    pub fn release(&mut self) {
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::Create { .. } => {
                if let Some(i) = self.sel.filter(|i| *i < self.frames_ref().len()) {
                    if self.frames_ref()[i].w < 4.0 {
                        let kind = self.frames_ref()[i].kind.clone();
                        let f = &mut self.frames()[i];
                        (f.w, f.h) = match kind {
                            Kind::Text { .. } => (240.0, 80.0),
                            Kind::Line { .. } => (200.0, 6.0),
                            Kind::Table { .. } => (280.0, 110.0),
                            Kind::TextPath { .. } => (240.0, 90.0),
                            _ => (160.0, 120.0),
                        };
                    }
                    if matches!(self.frames_ref()[i].kind, Kind::Text { .. } | Kind::TextPath { .. }) {
                        self.edit_text = Some(i);
                    }
                }
                self.tool = Select;
            }
            Drag::Guide { index } => {
                if let Some(g) = self.doc.guides.get(index) {
                    let span = if g.vertical { self.doc.width } else { self.doc.height };
                    if g.pos < -30.0 || g.pos > span + 30.0 {
                        self.doc.guides.remove(index);
                    }
                }
            }
            Drag::Node { .. } => {
                if let Some(i) = self.sel {
                    self.renormalize(i);
                }
            }
            Drag::Pencil => {
                let pts = std::mem::take(&mut self.pencil);
                if pts.len() >= 3 {
                    let closed = pts.len() > 6 && dist(pts[0], *pts.last().unwrap()) < 10.0;
                    let mut keep: Vec<[f32; 2]> = vec![pts[0]];
                    for q in &pts[1..] {
                        if dist(*keep.last().unwrap(), *q) >= 6.0 {
                            keep.push(*q);
                        }
                    }
                    if closed && keep.len() > 3 {
                        keep.pop();
                    }
                    let mut nodes: Vec<Node> = keep.iter().map(|q| Node::corner(q[0], q[1])).collect();
                    for k in 0..nodes.len() {
                        model::smooth_node(&mut nodes, k, closed);
                    }
                    let (x, y, w, h, n) = path_frame(&nodes);
                    self.new_frame(Frame::new(x, y, w, h, Kind::Path { nodes: n, closed, fill: closed.then_some([120, 200, 255, 255]), color: [20, 20, 24, 255], width: 1.5 }));
                }
            }
            Drag::Grad { .. } => self.grad_line = None,
            _ => {}
        }
        self.snap_lines.clear();
    }

    pub fn tool_options(&mut self, ui: &mut Ui) {
        let lbl = |ui: &mut Ui, t: &str| {
            ui.label(RichText::new(t).size(12.0).color(MUTED));
        };
        match self.tool {
            Polygon | FramePolygon => {
                lbl(ui, "Côtés");
                ui.add(egui::DragValue::new(&mut self.polygon_sides).range(3..=24));
            }
            Star => {
                lbl(ui, "Branches");
                ui.add(egui::DragValue::new(&mut self.star_points).range(3..=48));
                lbl(ui, "Creux");
                ui.add(egui::Slider::new(&mut self.polygon_inner, 0.15..=0.95).show_value(false));
            }
            Measure => {
                if let Some((a, b)) = self.measure_line {
                    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                    for (t, v) in [("D", dx.hypot(dy) / MM), ("ΔX", dx / MM), ("ΔY", dy / MM)] {
                        lbl(ui, t);
                        ui.label(RichText::new(format!("{v:.1} mm")).size(12.0).monospace());
                    }
                    lbl(ui, "Angle");
                    ui.label(RichText::new(format!("{:.1}°", -dy.atan2(dx).to_degrees())).size(12.0).monospace());
                    return;
                }
            }
            PageTool if self.page != MASTER => {
                let (w, h) = self.doc.page_size(self.page);
                let (mut wm, mut hm) = (w / MM, h / MM);
                lbl(ui, "Largeur");
                let a = ui.add(egui::DragValue::new(&mut wm).range(10.0..=2000.0).suffix(" mm").max_decimals(1)).changed();
                lbl(ui, "Hauteur");
                let b = ui.add(egui::DragValue::new(&mut hm).range(10.0..=2000.0).suffix(" mm").max_decimals(1)).changed();
                if a || b {
                    self.doc.pages[self.page].size = Some([wm * MM, hm * MM]);
                }
                if ui.add_enabled(self.doc.pages[self.page].size.is_some(), egui::Button::new("Format du document")).clicked() {
                    self.doc.pages[self.page].size = None;
                }
                return;
            }
            Pen if !self.pen.is_empty() => {
                lbl(ui, &format!("{} points", self.pen.len()));
                if ui.button("Terminer (Entrée)").clicked() {
                    self.commit_pen(false);
                }
                if ui.button("Fermer le tracé").clicked() {
                    self.commit_pen(true);
                }
                return;
            }
            _ => {}
        }
        ui.label(RichText::new(hint(self.tool)).size(12.0).color(MUTED));
    }

    pub fn tool_overlay(&self, painter: &Painter, to_scr: &dyn Fn([f32; 2]) -> Pos2) {
        let cyan = Color32::from_rgb(0, 200, 255);
        let s = Stroke::new(1.2, cyan);
        if !self.pen.is_empty() {
            let pts: Vec<Pos2> = self.pen.iter().map(|n| to_scr(n.p)).collect();
            for w in self.pen.windows(2) {
                let seg: Vec<Pos2> = (0..=16).map(|k| to_scr(model::bez(&w[0], &w[1], k as f32 / 16.0))).collect();
                painter.add(Shape::line(seg, s));
            }
            for (n, p) in self.pen.iter().zip(&pts) {
                painter.rect(Rect::from_center_size(*p, vec2(7.0, 7.0)), CornerRadius::ZERO, Color32::WHITE, s, StrokeKind::Middle);
                if n.smooth() {
                    for h in [n.i, n.o] {
                        painter.line_segment([*p, to_scr(h)], Stroke::new(1.0, cyan));
                        painter.circle_filled(to_scr(h), 3.0, cyan);
                    }
                }
            }
        }
        if self.pencil.len() > 1 {
            painter.add(Shape::line(self.pencil.iter().map(|q| to_scr(*q)).collect(), Stroke::new(1.5, cyan)));
        }
        for (a, b) in [self.measure_line, self.grad_line].into_iter().flatten() {
            painter.line_segment([to_scr(a), to_scr(b)], Stroke::new(1.5, Color32::from_rgb(255, 214, 10)));
            painter.circle_filled(to_scr(a), 3.5, Color32::from_rgb(255, 214, 10));
            painter.circle_stroke(to_scr(b), 4.0, Stroke::new(1.5, Color32::from_rgb(255, 214, 10)));
        }
        if let Some(i) = self.sel
            && let Some(f) = self.frames_ref().get(i)
            && matches!(self.tool, Direct | Pen | AddPoint | DelPoint | ConvPoint | Scissors | TextPath)
        {
            match &f.kind {
                Kind::Path { nodes, closed, .. } | Kind::TextPath { nodes, closed, .. } => {
                    let abs = abs_nodes(f, nodes);
                    let n = abs.len();
                    let segs = if *closed { n } else { n.saturating_sub(1) };
                    for k in 0..segs {
                        let line: Vec<Pos2> = (0..=20).map(|t| to_scr(model::bez(&abs[k], &abs[(k + 1) % n], t as f32 / 20.0))).collect();
                        painter.add(Shape::line(line, Stroke::new(1.0, cyan)));
                    }
                    for (k, a) in abs.iter().enumerate() {
                        if nodes[k].smooth() {
                            for h in [a.i, a.o] {
                                painter.line_segment([to_scr(a.p), to_scr(h)], Stroke::new(1.0, cyan));
                                painter.circle_filled(to_scr(h), 3.0, cyan);
                            }
                        }
                        painter.rect(Rect::from_center_size(to_scr(a.p), vec2(7.0, 7.0)), CornerRadius::ZERO, if k == 0 { cyan } else { Color32::WHITE }, s, StrokeKind::Middle);
                    }
                }
                Kind::Image { ox, oy, zoom, .. } if self.tool == Direct => {
                    let (cw, ch) = (f.w * zoom, f.h * zoom);
                    let r = Rect::from_min_max(to_scr([f.x + (f.w - cw) / 2.0 + ox, f.y + (f.h - ch) / 2.0 + oy]), to_scr([f.x + (f.w + cw) / 2.0 + ox, f.y + (f.h + ch) / 2.0 + oy]));
                    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.2, Color32::from_rgb(255, 160, 40)), StrokeKind::Middle);
                }
                _ => {}
            }
        }
    }
}

pub fn cursor(t: Tool) -> egui::CursorIcon {
    match t {
        Select | Direct | FreeTransform => egui::CursorIcon::Default,
        Pipette | ColorTheme => egui::CursorIcon::Copy,
        Hand => egui::CursorIcon::Grab,
        Zoom => egui::CursorIcon::ZoomIn,
        Gap => egui::CursorIcon::ResizeColumn,
        Rotate => egui::CursorIcon::Alias,
        Text | TextPath => egui::CursorIcon::Text,
        Note => egui::CursorIcon::ContextMenu,
        _ => egui::CursorIcon::Crosshair,
    }
}

pub fn tool_icon(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    let at = |x: f32, y: f32| r.min + vec2(x, y) * r.width();
    let s = Stroke::new(1.6, c);
    let thin = Stroke::new(1.2, c);
    let arrow = |filled: bool| {
        let pts = vec![at(0.2, 0.0), at(0.85, 0.6), at(0.52, 0.62), at(0.33, 1.0)];
        if filled {
            p.add(Shape::convex_polygon(pts, c, Stroke::NONE));
        } else {
            p.add(Shape::closed_line(pts, Stroke::new(1.4, c)));
        }
    };
    let nib = |dx: f32| {
        p.add(Shape::closed_line(vec![at(0.5 + dx, 0.0), at(0.82 + dx, 0.45), at(0.5 + dx, 0.95), at(0.18 + dx, 0.45)], thin));
        p.line_segment([at(0.5 + dx, 0.3), at(0.5 + dx, 0.95)], thin);
    };
    let badge = |t: &str| {
        p.text(at(0.95, 0.15), Align2::CENTER_CENTER, t, semibold(r.width() * 0.55), c);
    };
    let star = |pts: usize, inner: f32, fill: bool| {
        let v: Vec<Pos2> = (0..pts * 2)
            .map(|k| {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / pts as f32;
                let rr = if k % 2 == 1 { inner } else { 0.5 };
                r.center() + vec2(a.cos(), a.sin()) * r.width() * rr
            })
            .collect();
        if fill {
            p.add(Shape::closed_line(v, Stroke::new(1.5, c)));
        } else {
            p.add(Shape::closed_line(v, thin));
        }
    };
    let cross = || {
        p.line_segment([at(0.0, 0.0), at(1.0, 1.0)], Stroke::new(0.9, c));
        p.line_segment([at(1.0, 0.0), at(0.0, 1.0)], Stroke::new(0.9, c));
    };
    match tool {
        Select => arrow(true),
        Direct => arrow(false),
        PageTool => {
            p.rect_stroke(Rect::from_min_max(at(0.1, 0.0), at(0.8, 0.85)), CornerRadius::same(1), thin, StrokeKind::Middle);
            p.add(Shape::convex_polygon(vec![at(0.55, 0.6), at(1.0, 1.0), at(0.55, 1.0)], c, Stroke::NONE));
        }
        Gap => {
            p.rect_filled(Rect::from_min_max(at(0.0, 0.1), at(0.3, 0.9)), CornerRadius::same(1), c);
            p.rect_filled(Rect::from_min_max(at(0.7, 0.1), at(1.0, 0.9)), CornerRadius::same(1), c);
            p.line_segment([at(0.38, 0.5), at(0.62, 0.5)], thin);
        }
        Text => {
            p.text(r.center(), Align2::CENTER_CENTER, "T", semibold(r.width() * 1.05), c);
        }
        TextPath => {
            p.text(at(0.38, 0.38), Align2::CENTER_CENTER, "T", semibold(r.width() * 0.8), c);
            p.add(Shape::line((0..=10).map(|k| at(k as f32 / 10.0, 0.95 - (k as f32 / 10.0 * std::f32::consts::PI).sin() * 0.25)).collect(), thin));
        }
        Table => {
            let b = Rect::from_min_max(at(0.0, 0.1), at(1.0, 0.9));
            p.rect_stroke(b, CornerRadius::same(2), Stroke::new(1.4, c), StrokeKind::Middle);
            p.rect_filled(Rect::from_min_max(b.min, pos2(b.right(), b.top() + b.height() / 3.0)), CornerRadius::same(2), c);
            p.line_segment([pos2(b.left(), b.top() + b.height() * 2.0 / 3.0), pos2(b.right(), b.top() + b.height() * 2.0 / 3.0)], Stroke::new(1.0, c));
            p.line_segment([pos2(b.center().x, b.top()), pos2(b.center().x, b.bottom())], Stroke::new(1.0, c));
        }
        Pen => nib(0.0),
        AddPoint => {
            nib(-0.12);
            badge("+");
        }
        DelPoint => {
            nib(-0.12);
            badge("−");
        }
        ConvPoint => {
            p.add(Shape::line(vec![at(0.1, 0.9), at(0.5, 0.2), at(0.9, 0.9)], s));
            p.rect_filled(Rect::from_center_size(at(0.5, 0.2), vec2(4.0, 4.0)), CornerRadius::ZERO, c);
        }
        Pencil => {
            p.line_segment([at(0.15, 0.85), at(0.8, 0.2)], Stroke::new(3.0, c));
            p.add(Shape::convex_polygon(vec![at(0.05, 0.95), at(0.1, 0.75), at(0.25, 0.9)], c, Stroke::NONE));
        }
        Line => {
            p.line_segment([at(0.05, 0.95), at(0.95, 0.05)], Stroke::new(1.8, c));
        }
        FrameRect => {
            p.rect_stroke(r.shrink(1.0), CornerRadius::ZERO, thin, StrokeKind::Middle);
            cross();
        }
        FrameEllipse => {
            p.circle_stroke(r.center(), r.width() * 0.48, thin);
            let k = 0.34;
            p.line_segment([at(0.5 - k, 0.5 - k), at(0.5 + k, 0.5 + k)], Stroke::new(0.9, c));
            p.line_segment([at(0.5 + k, 0.5 - k), at(0.5 - k, 0.5 + k)], Stroke::new(0.9, c));
        }
        FramePolygon => {
            let v: Vec<Pos2> = (0..6).map(|k| r.center() + vec2((k as f32 * 1.047 - 1.57).cos(), (k as f32 * 1.047 - 1.57).sin()) * r.width() * 0.5).collect();
            p.add(Shape::closed_line(v.clone(), thin));
            p.line_segment([v[0], v[3]], Stroke::new(0.9, c));
            p.line_segment([v[1], v[4]], Stroke::new(0.9, c));
        }
        Rect => {
            p.rect_stroke(Rect::from_min_max(at(0.05, 0.15), at(0.95, 0.85)), CornerRadius::same(1), s, StrokeKind::Middle);
        }
        Ellipse => {
            p.circle_stroke(r.center(), r.width() * 0.45, s);
        }
        Polygon => {
            let v: Vec<Pos2> = (0..6).map(|k| r.center() + vec2((k as f32 * 1.047 - 1.57).cos(), (k as f32 * 1.047 - 1.57).sin()) * r.width() * 0.5).collect();
            p.add(Shape::closed_line(v, s));
        }
        Star => star(5, 0.22, true),
        Scissors => {
            p.circle_stroke(at(0.22, 0.78), r.width() * 0.16, thin);
            p.circle_stroke(at(0.78, 0.78), r.width() * 0.16, thin);
            p.line_segment([at(0.32, 0.66), at(0.85, 0.0)], thin);
            p.line_segment([at(0.68, 0.66), at(0.15, 0.0)], thin);
        }
        FreeTransform => {
            p.rect_stroke(Rect::from_min_max(at(0.1, 0.1), at(0.9, 0.9)), CornerRadius::ZERO, Stroke::new(1.0, c), StrokeKind::Middle);
            for q in [at(0.1, 0.1), at(0.9, 0.1), at(0.9, 0.9), at(0.1, 0.9)] {
                p.rect_filled(Rect::from_center_size(q, vec2(4.0, 4.0)), CornerRadius::ZERO, c);
            }
            arrow(true);
        }
        Rotate => {
            p.add(Shape::line((0..=14).map(|k| r.center() + vec2((k as f32 * 0.35 - 1.0).cos(), (k as f32 * 0.35 - 1.0).sin()) * r.width() * 0.42).collect(), s));
            p.add(Shape::convex_polygon(vec![at(0.62, 0.0), at(0.98, 0.1), at(0.72, 0.36)], c, Stroke::NONE));
        }
        Scale => {
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.4), at(0.6, 1.0)), CornerRadius::ZERO, thin, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(at(0.0, 0.0), at(1.0, 1.0)), CornerRadius::ZERO, Stroke::new(0.8, c), StrokeKind::Middle);
            p.line_segment([at(0.35, 0.65), at(0.9, 0.1)], thin);
        }
        Shear => {
            p.add(Shape::closed_line(vec![at(0.3, 0.1), at(1.0, 0.1), at(0.7, 0.9), at(0.0, 0.9)], s));
        }
        Gradient => {
            let b = Rect::from_min_max(at(0.0, 0.15), at(1.0, 0.85));
            for k in 0..6 {
                let x = b.left() + b.width() * k as f32 / 6.0;
                p.rect_filled(Rect::from_min_max(pos2(x, b.top()), pos2(x + b.width() / 6.0 + 0.5, b.bottom())), CornerRadius::ZERO, c.gamma_multiply(1.0 - k as f32 / 7.0));
            }
        }
        GradFeather => {
            for k in 0..6 {
                p.circle_filled(r.center(), r.width() * (0.5 - k as f32 * 0.07), c.gamma_multiply(0.15 + k as f32 * 0.15));
            }
        }
        Note => {
            p.add(Shape::closed_line(vec![at(0.0, 0.05), at(1.0, 0.05), at(1.0, 0.7), at(0.4, 0.7), at(0.15, 0.95), at(0.2, 0.7), at(0.0, 0.7)], thin));
            for y in [0.25, 0.45] {
                p.line_segment([at(0.2, y), at(0.8, y)], Stroke::new(1.0, c));
            }
        }
        Pipette => {
            p.line_segment([at(0.15, 0.85), at(0.62, 0.38)], Stroke::new(2.4, c));
            p.circle_filled(at(0.72, 0.28), r.width() * 0.2, c);
            p.circle_filled(at(0.12, 0.88), r.width() * 0.07, c);
        }
        ColorTheme => {
            p.line_segment([at(0.1, 0.9), at(0.5, 0.5)], Stroke::new(2.2, c));
            for (k, col) in [(0, Color32::from_rgb(255, 107, 53)), (1, Color32::from_rgb(255, 204, 0)), (2, Color32::from_rgb(10, 132, 255))] {
                p.rect_filled(Rect::from_min_size(at(0.5 + k as f32 * 0.17, 0.05), vec2(r.width() * 0.15, r.height() * 0.4)), CornerRadius::same(1), col);
            }
        }
        Measure => {
            p.line_segment([at(0.0, 0.85), at(1.0, 0.15)], Stroke::new(1.6, c));
            for k in 1..5 {
                let q = at(k as f32 * 0.2, 0.85 - k as f32 * 0.14);
                p.line_segment([q, q + vec2(-2.5, -3.5)], Stroke::new(1.0, c));
            }
        }
        Hand => {
            p.rect_filled(Rect::from_min_max(at(0.2, 0.45), at(0.85, 1.0)), CornerRadius::same(3), c);
            for (k, x) in [0.25, 0.42, 0.59, 0.76].iter().enumerate() {
                p.rect_filled(Rect::from_min_max(at(*x, [0.15, 0.0, 0.05, 0.2][k]), at(x + 0.1, 0.6)), CornerRadius::same(2), c);
            }
        }
        Zoom => {
            p.circle_stroke(at(0.42, 0.42), r.width() * 0.32, s);
            p.line_segment([at(0.65, 0.65), at(0.98, 0.98)], Stroke::new(2.4, c));
        }
    }
    let _ = FAINT;
}

pub fn theme_colors(img: &image::RgbaImage, n: usize) -> Vec<[u8; 4]> {
    let mut bins: std::collections::HashMap<u16, (u32, [u64; 3])> = std::collections::HashMap::new();
    for px in img.pixels().filter(|p| p.0[3] > 128) {
        let [r, g, b, _] = px.0;
        let e = bins.entry(((r as u16 >> 4) << 8) | ((g as u16 >> 4) << 4) | (b as u16 >> 4)).or_default();
        e.0 += 1;
        e.1[0] += r as u64;
        e.1[1] += g as u64;
        e.1[2] += b as u64;
    }
    let mut v: Vec<(u32, [u8; 4])> = bins.into_values().map(|(k, s)| (k, [(s[0] / k as u64) as u8, (s[1] / k as u64) as u8, (s[2] / k as u64) as u8, 255])).collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out: Vec<[u8; 4]> = Vec::new();
    for (_, c) in v {
        if out.iter().all(|o| (0..3).map(|k| (o[k] as i32 - c[k] as i32).abs()).sum::<i32>() > 60) {
            out.push(c);
            if out.len() == n {
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod theme_tests {
    #[test]
    fn dominant_colors() {
        let img = image::RgbaImage::from_fn(10, 10, |x, _| if x < 7 { image::Rgba([200, 30, 30, 255]) } else { image::Rgba([20, 40, 220, 255]) });
        let c = super::theme_colors(&img, 5);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0], [200, 30, 30, 255]);
    }
}
