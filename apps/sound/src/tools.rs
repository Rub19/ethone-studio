use eframe::egui::{Color32, Key, Painter, Rect};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Hand,
    Zoom,
    SpecRect,
    Lasso,
    Brush,
    Heal,
    Pencil,
    Razor,
    Fade,
    Envelope,
}

pub type Group = &'static [(Tool, &'static str, &'static str)];

pub const GROUPS: [(Group, Key); 6] = [
    (&[(Tool::Select, "Sélection temporelle", "V")], Key::V),
    (&[(Tool::Hand, "Main (défilement)", "H"), (Tool::Zoom, "Zoom", "H")], Key::H),
    (&[(Tool::SpecRect, "Rectangle de fréquences", "E"), (Tool::Lasso, "Lasso spectral", "E"), (Tool::Brush, "Pinceau spectral", "E")], Key::E),
    (&[(Tool::Heal, "Correction des taches", "B"), (Tool::Pencil, "Crayon", "B")], Key::B),
    (&[(Tool::Razor, "Rasoir", "C")], Key::C),
    (&[(Tool::Fade, "Fondus par poignées", "G"), (Tool::Envelope, "Enveloppe de volume", "G")], Key::G),
];

impl Tool {
    pub fn spectral(self) -> bool {
        matches!(self, Tool::SpecRect | Tool::Lasso | Tool::Brush)
    }

    pub fn name(self) -> &'static str {
        GROUPS.iter().flat_map(|g| g.0.iter()).find(|t| t.0 == self).map_or("", |t| t.1)
    }

    pub fn help(self) -> &'static str {
        match self {
            Tool::Select => "Glisse pour sélectionner une durée, tire les bords pour ajuster. Maj+clic étend, double-clic prend l'espace entre deux marqueurs.",
            Tool::Hand => "Glisse pour faire défiler la vue sans toucher à la sélection.",
            Tool::Zoom => "Clic : zoom avant au point cliqué. Alt+clic ou clic droit : zoom arrière. Glisse pour zoomer sur une zone.",
            Tool::SpecRect => "Dans le spectrogramme, glisse un rectangle temps × fréquences, puis atténue, supprime ou répare la zone.",
            Tool::Lasso => "Dessine librement le contour d'un bruit dans le spectrogramme (sifflet, toux, sonnerie).",
            Tool::Brush => "Peins sur le spectrogramme les zones à traiter ; la taille se règle ci-dessous.",
            Tool::Heal => "Peins sur un clic ou un bruit bref : il est reconstruit à partir de ce qui l'entoure. Dans le spectrogramme, seule la zone peinte est réparée.",
            Tool::Pencil => "Zoome jusqu'aux échantillons puis dessine la forme d'onde à la main pour effacer un clic.",
            Tool::Razor => "Clic : pose une coupe (le fichier s'exporte en morceaux). Alt+clic : supprime le segment entre deux coupes.",
            Tool::Fade => "Tire les carrés en haut à gauche et à droite : horizontalement la durée, verticalement la courbe. Double-clic : retire le fondu.",
            Tool::Envelope => "Clic sur la forme d'onde : ajoute un point de volume. Glisse un point pour le déplacer, clic droit pour le retirer.",
        }
    }
}

#[derive(Clone)]
pub enum SpecShape {
    Rect { f0: f32, f1: f32 },
    Lasso(Vec<(f64, f32)>),
    Brush { pts: Vec<(f64, f32)>, rs: f64, ry: f32 },
}

#[derive(Clone)]
pub struct SpecSel {
    pub a: usize,
    pub b: usize,
    pub shape: SpecShape,
}

impl SpecSel {
    pub fn contains(&self, s: usize, f: f32, yf: &dyn Fn(f32) -> f32) -> bool {
        if s < self.a || s > self.b {
            return false;
        }
        match &self.shape {
            SpecShape::Rect { f0, f1 } => f >= *f0 && f <= *f1,
            SpecShape::Lasso(pts) => {
                let (x, y) = (s as f64, yf(f) as f64);
                let mut inside = false;
                let n = pts.len();
                for i in 0..n {
                    let (xi, yi) = (pts[i].0, yf(pts[i].1) as f64);
                    let (xj, yj) = (pts[(i + n - 1) % n].0, yf(pts[(i + n - 1) % n].1) as f64);
                    if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                        inside = !inside;
                    }
                }
                inside
            }
            SpecShape::Brush { pts, rs, ry } => {
                let y = yf(f);
                pts.iter().any(|(ps, pf)| {
                    let dx = (s as f64 - ps) / rs.max(1.0);
                    let dy = ((y - yf(*pf)) / ry.max(1e-4)) as f64;
                    dx * dx + dy * dy <= 1.0
                })
            }
        }
    }

    pub fn extend(&mut self, s: f64, f: f32) {
        match &mut self.shape {
            SpecShape::Rect { .. } => {}
            SpecShape::Lasso(p) => p.push((s, f)),
            SpecShape::Brush { pts, .. } => pts.push((s, f)),
        }
        let r = match &self.shape {
            SpecShape::Brush { rs, .. } => *rs,
            _ => 0.0,
        };
        self.a = self.a.min((s - r).max(0.0) as usize);
        self.b = self.b.max((s + r) as usize);
    }
}

/// Lucide icon names, so this app's icons match the rest of the suite.
fn draw_icon_lucide(t: Tool) -> &'static str {
    use Tool::*;
    match t {
        Select => "text-cursor",
        Hand => "hand",
        Zoom => "zoom-in",
        SpecRect => "square-dashed",
        Lasso => "lasso",
        Brush => "paintbrush",
        Heal => "bandage",
        Pencil => "pencil",
        Razor => "scissors",
        Fade => "trending-down",
        Envelope => "activity",
    }
}

pub fn draw_icon(p: &Painter, r: Rect, c: Color32, t: Tool) {
    studio_ui::pro::lucide(p, r.expand(r.width() * 0.12), draw_icon_lucide(t), c);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectral_masks() {
        let yf = |f: f32| f / 4000.0;
        let r = SpecSel { a: 100, b: 200, shape: SpecShape::Rect { f0: 500.0, f1: 1000.0 } };
        assert!(r.contains(150, 700.0, &yf) && !r.contains(150, 1500.0, &yf) && !r.contains(300, 700.0, &yf));
        let l = SpecSel { a: 0, b: 100, shape: SpecShape::Lasso(vec![(0.0, 0.0), (100.0, 0.0), (100.0, 2000.0), (0.0, 2000.0)]) };
        assert!(l.contains(50, 1000.0, &yf) && !l.contains(50, 3000.0, &yf));
        let mut b = SpecSel { a: 50, b: 50, shape: SpecShape::Brush { pts: vec![], rs: 10.0, ry: 0.05 } };
        b.extend(50.0, 1000.0);
        assert!(b.contains(55, 1000.0, &yf) && !b.contains(55, 2000.0, &yf) && b.a == 40 && b.b == 60);
        assert_eq!(GROUPS.iter().map(|g| g.0.len()).sum::<usize>(), 11);
    }
}
