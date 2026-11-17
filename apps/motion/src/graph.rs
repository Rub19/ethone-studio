use super::*;
use studio_ui::i18n::tl;
use model::{BEZ_PRESETS, Key, ease_to_bez};

impl Motion {
    pub(crate) fn shown_props(&self, li: usize) -> Vec<usize> {
        if let Some(f) = &self.tl_filter
            && self.sel == Some(li)
        {
            return f.clone();
        }
        let Some(l) = self.comp.layers.get(li) else { return Vec::new() };
        let v: Vec<usize> = (0..l.props.len()).filter(|pi| l.props[*pi].animated() || l.props[*pi].expressive()).collect();
        if v.is_empty() { vec![0, 1, 2, 3, 4] } else { v }
    }

    pub(crate) fn key_refs(&self) -> Vec<(usize, usize, usize)> {
        if self.ksel.is_empty() {
            return match self.sel.filter(|i| *i < self.comp.layers.len()) {
                Some(i) => self.comp.layers[i].props.iter().enumerate().flat_map(|(pi, p)| (0..p.keys.len()).map(move |ki| (i, pi, ki))).collect(),
                None => Vec::new(),
            };
        }
        self.ksel
            .iter()
            .filter_map(|(li, pi, t)| self.comp.layers.get(*li)?.props.get(*pi)?.key_at(*t).map(|ki| (*li, *pi, ki)))
            .collect()
    }

    pub(crate) fn with_keys(&mut self, f: impl Fn(&mut Key)) {
        for (li, pi, ki) in self.key_refs() {
            f(&mut self.comp.layers[li].props[pi].keys[ki]);
        }
    }

    pub(crate) fn copy_keys(&mut self) {
        let refs = self.key_refs();
        let t0 = refs.iter().map(|(li, pi, ki)| self.comp.layers[*li].props[*pi].keys[*ki].t).fold(f32::MAX, f32::min);
        let keys: Vec<(usize, Key)> = refs
            .iter()
            .map(|(li, pi, ki)| {
                let mut k = self.comp.layers[*li].props[*pi].keys[*ki];
                k.t -= t0;
                (*pi, k)
            })
            .collect();
        if !keys.is_empty() {
            self.clip_keys = Some(keys);
        }
    }

    pub(crate) fn paste_keys(&mut self) {
        let (Some(i), Some(keys)) = (self.sel, self.clip_keys.clone()) else { return };
        let t = self.t;
        let l = &mut self.comp.layers[i];
        self.ksel.clear();
        for (pi, k) in keys {
            if let Some(p) = l.props.get_mut(pi) {
                p.add_key(t + k.t, k.v);
                if let Some(ki) = p.key_at(t + k.t) {
                    p.keys[ki] = Key { t: t + k.t, ..k };
                }
                self.ksel.push((i, pi, t + k.t));
            }
        }
    }

    pub(crate) fn delete_keys(&mut self) {
        let mut refs = self.key_refs();
        refs.sort_by(|a, b| b.2.cmp(&a.2));
        for (li, pi, ki) in refs {
            let p = &mut self.comp.layers[li].props[pi];
            let v = p.keys[ki].v;
            p.keys.remove(ki);
            if p.keys.is_empty() {
                p.base = v;
            }
        }
        self.ksel.clear();
    }

    pub(crate) fn reverse_keys(&mut self) {
        let Some(i) = self.sel else { return };
        let l = &mut self.comp.layers[i];
        let times = l.key_times();
        let (a, b) = (times.first().copied().unwrap_or(0.0), times.last().copied().unwrap_or(0.0));
        for p in &mut l.props {
            p.reverse(a, b);
        }
        self.ksel.clear();
    }

    pub(crate) fn spread_keys(&mut self) {
        let Some(i) = self.sel else { return };
        for p in &mut self.comp.layers[i].props {
            p.spread();
        }
        self.ksel.clear();
    }

    pub(crate) fn easy_ease(&mut self, mode: u8) {
        for (li, pi, ki) in self.key_refs() {
            let keys = &mut self.comp.layers[li].props[pi].keys;
            match mode {
                0 => keys[ki].set_ease(Ease::Smooth),
                1 if ki > 0 => {
                    let prev = &mut keys[ki - 1];
                    let mut b = prev.bez.unwrap_or_else(|| ease_to_bez(prev.ease));
                    b[2] = 0.42;
                    b[3] = 1.0;
                    prev.bez = Some(b);
                }
                2 => {
                    let k = &mut keys[ki];
                    let mut b = k.bez.unwrap_or_else(|| ease_to_bez(k.ease));
                    b[0] = 0.42;
                    b[1] = 0.0;
                    k.bez = Some(b);
                }
                _ => {}
            }
        }
    }

    fn graph_target(&self) -> Option<(usize, usize, usize)> {
        let valid = |(li, pi, ki): (usize, usize, usize)| self.comp.layers.get(li).and_then(|l| l.props.get(pi)).is_some_and(|p| ki + 1 < p.keys.len());
        if let Some(g) = self.gkey.filter(|g| valid(*g) && Some(g.0) == self.sel) {
            return Some(g);
        }
        if let Some(r) = self.key_refs().into_iter().find(|r| valid(*r) && !self.ksel.is_empty()) {
            return Some(r);
        }
        let i = self.sel?;
        let t = self.t;
        self.comp.layers.get(i)?.props.iter().enumerate().find_map(|(pi, p)| p.keys.iter().rposition(|k| k.t <= t + 1e-3).filter(|ki| ki + 1 < p.keys.len()).map(|ki| (i, pi, ki)))
    }

    pub(crate) fn graph_panel(&mut self, ui: &mut Ui) {
        let Some((li, pi, ki)) = self.graph_target() else { return };
        let speed = self.graph_speed;
        pro::dock_panel(ui, "graph", tl("Éditeur de courbe"), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let (k, next) = {
                let p = &self.comp.layers[li].props[pi];
                (p.keys[ki], p.keys[ki + 1])
            };
            ui.label(RichText::new(format!("{} · {} → {}", PROPS[pi], secs(k.t), secs(next.t))).size(12.0).color(MUTED));
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(vec2(w, 150.0), Sense::hover());
            let p = ui.painter();
            p.rect(r, CornerRadius::same(4), pro::WELL, Stroke::new(1.0, LINE), StrokeKind::Inside);
            let plot = r.shrink(12.0);
            let (lo, hi) = (-0.4f32, 1.4f32);
            let to = |x: f32, y: f32| pos2(plot.left() + x * plot.width(), plot.bottom() - (y - lo) / (hi - lo) * plot.height());
            for y in [0.0, 1.0] {
                p.line_segment([to(0.0, y), to(1.0, y)], Stroke::new(1.0, Color32::from_white_alpha(22)));
            }
            for x in [0.25, 0.5, 0.75] {
                p.line_segment([to(x, lo), to(x, hi)], Stroke::new(1.0, Color32::from_white_alpha(10)));
            }
            let n = 80;
            let vals: Vec<f32> = (0..=n).map(|j| k.curve(j as f32 / n as f32)).collect();
            let pts: Vec<Pos2> = if speed {
                let d: Vec<f32> = (0..=n).map(|j| (vals[(j + 1).min(n)] - vals[j.saturating_sub(1)]) / (2.0 / n as f32)).collect();
                let m = d.iter().map(|v| v.abs()).fold(1e-3f32, f32::max);
                d.iter().enumerate().map(|(j, v)| to(j as f32 / n as f32, v / m)).collect()
            } else {
                vals.iter().enumerate().map(|(j, v)| to(j as f32 / n as f32, *v)).collect()
            };
            p.add(Shape::line(pts, Stroke::new(2.0, KEY)));
            let bez = k.bez.unwrap_or_else(|| ease_to_bez(k.ease));
            let live = k.bez.is_some();
            let hc = if live { Color32::WHITE } else { Color32::from_white_alpha(90) };
            let h1 = to(bez[0], bez[1]);
            let h2 = to(bez[2], bez[3]);
            if !speed {
                p.line_segment([to(0.0, 0.0), h1], Stroke::new(1.0, hc));
                p.line_segment([to(1.0, 1.0), h2], Stroke::new(1.0, hc));
                p.circle(h1, 5.5, BLUE, Stroke::new(1.5, hc));
                p.circle(h2, 5.5, BLUE, Stroke::new(1.5, hc));
            }
            diamond(p, to(0.0, 0.0), 5.0, KEY);
            diamond(p, to(1.0, 1.0), 5.0, KEY);
            let mut nb = None;
            if !speed {
                for (h, which) in [(h1, 0usize), (h2, 2)] {
                    let resp = ui.interact(Rect::from_center_size(h, Vec2::splat(16.0)), ui.make_persistent_id(("bez-h", which)), Sense::drag()).on_hover_cursor(egui::CursorIcon::Grab);
                    if resp.dragged()
                        && let Some(pp) = resp.interact_pointer_pos()
                    {
                        let x = ((pp.x - plot.left()) / plot.width()).clamp(0.0, 1.0);
                        let y = (lo + (plot.bottom() - pp.y) / plot.height() * (hi - lo)).clamp(-1.0, 2.0);
                        let mut b = bez;
                        b[which] = (x * 100.0).round() / 100.0;
                        b[which + 1] = (y * 100.0).round() / 100.0;
                        nb = Some(Some(b));
                    }
                }
            }
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.graph_speed, tl("Vitesse"));
                ui.label(RichText::new(if live { "Bézier personnalisée" } else { k.ease_name() }).size(11.5).color(FAINT));
            });
            let mut b = bez;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                let mut ch = false;
                for (j, v) in b.iter_mut().enumerate() {
                    let range = if j % 2 == 0 { 0.0..=1.0 } else { -1.0..=2.0 };
                    ch |= ui.add(egui::DragValue::new(v).range(range).speed(0.01).max_decimals(2)).changed();
                }
                if ch {
                    nb = Some(Some(b));
                }
            });
            ui.label(RichText::new(tl("Préréglages")).size(12.0).color(MUTED));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for (name, pb) in BEZ_PRESETS {
                    if ui.selectable_label(k.bez == Some(pb), RichText::new(name).size(11.5)).clicked() {
                        nb = Some(Some(pb));
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.add_enabled(live, egui::Button::new("Revenir").small()).on_hover_text(tl("Revenir à la courbe choisie dans la liste")).clicked() {
                    nb = Some(None);
                }
                if ui.small_button(tl("Copier sur la sélection")).on_hover_text(tl("Même courbe pour toutes les clés sélectionnées (ou tout le calque)")).clicked() {
                    let bb = k.bez;
                    let (e, e2) = (k.ease, k.ease2);
                    self.with_keys(|kk| {
                        kk.bez = bb;
                        kk.ease = e;
                        kk.ease2 = e2;
                    });
                }
            });
            if let Some(b) = nb {
                self.comp.layers[li].props[pi].keys[ki].bez = b;
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new(tl("Valeur")).size(12.0).color(MUTED));
                let mut v = self.comp.layers[li].props[pi].keys[ki].v;
                if ui.add(egui::DragValue::new(&mut v).speed(0.5).max_decimals(2)).changed() {
                    self.comp.layers[li].props[pi].keys[ki].v = v;
                }
                ui.label(RichText::new("→").color(FAINT));
                let mut v2 = self.comp.layers[li].props[pi].keys[ki + 1].v;
                if ui.add(egui::DragValue::new(&mut v2).speed(0.5).max_decimals(2)).changed() {
                    self.comp.layers[li].props[pi].keys[ki + 1].v = v2;
                }
            });
        });
    }
}
