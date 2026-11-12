use image::{Rgba, RgbaImage, imageops};

#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub img: RgbaImage,
    pub visible: bool,
    pub opacity: f32,
    pub blend: Blend,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Blend {
    Normal,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    Darken,
    Lighten,
    Difference,
    Add,
    ColorDodge,
}

pub const BLENDS: [(Blend, &str); 10] = [
    (Blend::Normal, "Normal"),
    (Blend::Multiply, "Produit"),
    (Blend::Screen, "Superposition"),
    (Blend::Overlay, "Incrustation"),
    (Blend::SoftLight, "Lumière tamisée"),
    (Blend::Darken, "Obscurcir"),
    (Blend::Lighten, "Éclaircir"),
    (Blend::Difference, "Différence"),
    (Blend::Add, "Densité linéaire −"),
    (Blend::ColorDodge, "Densité couleur −"),
];

impl Blend {
    fn mix(self, b: f32, s: f32) -> f32 {
        match self {
            Blend::Normal => s,
            Blend::Multiply => b * s,
            Blend::Screen => 1.0 - (1.0 - b) * (1.0 - s),
            Blend::Overlay => if b < 0.5 { 2.0 * b * s } else { 1.0 - 2.0 * (1.0 - b) * (1.0 - s) },
            Blend::SoftLight => (1.0 - 2.0 * s) * b * b + 2.0 * s * b,
            Blend::Darken => b.min(s),
            Blend::Lighten => b.max(s),
            Blend::Difference => (b - s).abs(),
            Blend::Add => (b + s).min(1.0),
            Blend::ColorDodge => if s >= 1.0 { 1.0 } else { (b / (1.0 - s)).min(1.0) },
        }
    }
}

pub enum Paint {
    Color([u8; 4]),
    Pencil([u8; 4]),
    Erase,
    Dodge,
    Burn,
    Sponge(f32),
    Blur(RgbaImage),
    Sharpen(RgbaImage),
    Smudge(Option<[f32; 4]>),
    Clone { src: RgbaImage, dx: f32, dy: f32 },
}

pub fn inside(sel: Option<Area>, mask: Option<&Vec<bool>>, w: u32, x: u32, y: u32) -> bool {
    sel.is_none_or(|s| s.contains(x, y)) && mask.is_none_or(|m| m[(y * w + x) as usize])
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl Area {
    fn union(self, o: Area) -> Area {
        Area {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }
    pub fn from_points(a: (f32, f32), b: (f32, f32), w: u32, h: u32) -> Option<Area> {
        let x0 = a.0.min(b.0).floor().clamp(0.0, w as f32) as u32;
        let x1 = a.0.max(b.0).ceil().clamp(0.0, w as f32) as u32;
        let y0 = a.1.min(b.1).floor().clamp(0.0, h as f32) as u32;
        let y1 = a.1.max(b.1).ceil().clamp(0.0, h as f32) as u32;
        (x1 > x0 && y1 > y0).then_some(Area { x0, y0, x1, y1 })
    }
    pub fn width(&self) -> u32 {
        self.x1 - self.x0
    }
    pub fn height(&self) -> u32 {
        self.y1 - self.y0
    }
}

#[derive(Clone)]
struct Snapshot {
    layers: Vec<Layer>,
    active: usize,
}

pub struct Brush {
    pub radius: f32,
    pub hardness: f32,
    pub flow: f32,
}

const UNDO_LIMIT: usize = 40;

pub struct Document {
    pub layers: Vec<Layer>,
    pub active: usize,
    pub dirty: Option<Area>,
    pub selection: Option<Area>,
    pub mask: Option<Vec<bool>>,
    pub version: u64,
    pub modified: bool,
    pub name: String,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl Document {
    pub fn new(w: u32, h: u32, fill: Option<[u8; 4]>) -> Self {
        let img = RgbaImage::from_pixel(w, h, Rgba(fill.unwrap_or([0, 0, 0, 0])));
        Self::with_layer("Sans titre".into(), "Fond".into(), img)
    }

    pub fn from_image(name: String, img: RgbaImage) -> Self {
        Self::with_layer(name, "Fond".into(), img)
    }

    fn with_layer(name: String, layer: String, img: RgbaImage) -> Self {
        let mut d = Self {
            layers: vec![Layer { name: layer, img, visible: true, opacity: 1.0, blend: Blend::Normal }],
            active: 0,
            dirty: None,
            selection: None,
            mask: None,
            version: 0,
            modified: false,
            name,
            undo: Vec::new(),
            redo: Vec::new(),
        };
        d.touch_all();
        d
    }

    pub fn width(&self) -> u32 {
        self.layers[0].img.width()
    }
    pub fn height(&self) -> u32 {
        self.layers[0].img.height()
    }

    pub fn full(&self) -> Area {
        Area { x0: 0, y0: 0, x1: self.width(), y1: self.height() }
    }

    pub fn touch_all(&mut self) {
        self.version += 1;
        self.dirty = Some(self.full());
    }

    fn touch(&mut self, a: Area) {
        self.version += 1;
        self.dirty = Some(self.dirty.map_or(a, |d| d.union(a)));
    }

    pub fn checkpoint(&mut self) {
        // ponytail: full-layer snapshots, switch to per-tile diffs if memory gets tight on big canvases
        self.undo.push(Snapshot { layers: self.layers.clone(), active: self.active });
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.modified = true;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(s) = self.undo.pop() {
            let cur = Snapshot { layers: std::mem::replace(&mut self.layers, s.layers), active: self.active };
            self.redo.push(cur);
            self.active = s.active.min(self.layers.len() - 1);
            self.touch_all();
        }
    }

    pub fn redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            let cur = Snapshot { layers: std::mem::replace(&mut self.layers, s.layers), active: self.active };
            self.undo.push(cur);
            self.active = s.active.min(self.layers.len() - 1);
            self.touch_all();
        }
    }

    pub fn composite_area(&self, out: &mut RgbaImage, a: Area) {
        for y in a.y0..a.y1 {
            for x in a.x0..a.x1 {
                let mut acc = [0f32; 4];
                for l in self.layers.iter().filter(|l| l.visible && l.opacity > 0.0) {
                    blend_over(&mut acc, l.img.get_pixel(x, y).0, l.opacity, l.blend);
                }
                out.put_pixel(x, y, Rgba(to_u8(acc)));
            }
        }
    }

    pub fn flatten(&self) -> RgbaImage {
        let mut out = RgbaImage::new(self.width(), self.height());
        self.composite_area(&mut out, self.full());
        out
    }

    pub fn stamp(&mut self, cx: f32, cy: f32, brush: &Brush, color: [u8; 4], erase: bool) {
        self.stamp_paint(cx, cy, brush, &mut if erase { Paint::Erase } else { Paint::Color(color) });
    }

    pub fn stroke(&mut self, from: (f32, f32), to: (f32, f32), brush: &Brush, color: [u8; 4], erase: bool) {
        self.stroke_paint(from, to, brush, &mut if erase { Paint::Erase } else { Paint::Color(color) });
    }

    pub fn stroke_paint(&mut self, from: (f32, f32), to: (f32, f32), brush: &Brush, paint: &mut Paint) {
        let dist = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
        let step = (brush.radius * 0.25).max(0.5);
        let n = (dist / step).ceil().max(1.0) as usize;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            self.stamp_paint(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t, brush, paint);
        }
    }

    pub fn stamp_paint(&mut self, cx: f32, cy: f32, brush: &Brush, paint: &mut Paint) {
        let (w, h) = (self.width() as i32, self.height() as i32);
        let r = brush.radius.max(0.5);
        let s = self.selection.unwrap_or(self.full());
        let x0 = ((cx - r - 1.0).floor() as i32).clamp(s.x0 as i32, s.x1 as i32).clamp(0, w);
        let x1 = ((cx + r + 1.0).ceil() as i32).clamp(s.x0 as i32, s.x1 as i32).clamp(0, w);
        let y0 = ((cy - r - 1.0).floor() as i32).clamp(s.y0 as i32, s.y1 as i32).clamp(0, h);
        let y1 = ((cy + r + 1.0).ceil() as i32).clamp(s.y0 as i32, s.y1 as i32).clamp(0, h);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let hard = brush.hardness.clamp(0.0, 0.99);
        let base = match paint {
            Paint::Color(c) | Paint::Pencil(c) => c[3] as f32 / 255.0,
            _ => 1.0,
        } * brush.flow;
        let (sel, mask, wu) = (self.selection, self.mask.as_ref(), w as u32);
        let img = &mut self.layers[self.active].img;
        if let Paint::Smudge(carry) = paint
            && carry.is_none()
        {
            *carry = Some(img.get_pixel(cx.clamp(0.0, w as f32 - 1.0) as u32, cy.clamp(0.0, h as f32 - 1.0) as u32).0.map(|v| v as f32 / 255.0));
        }
        let mut picked = [0f32; 4];
        let mut picked_n = 0.0;
        for y in y0..y1 {
            for x in x0..x1 {
                if !inside(sel, mask, wu, x as u32, y as u32) {
                    continue;
                }
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                let edge = (r + 0.5 - d).clamp(0.0, 1.0);
                if edge <= 0.0 {
                    continue;
                }
                let t = d / r;
                let soft = if t <= hard { 1.0 } else { smooth(1.0 - (t - hard) / (1.0 - hard)) };
                let a = base * edge * soft;
                let px = img.get_pixel_mut(x as u32, y as u32);
                let mut c = px.0.map(|v| v as f32 / 255.0);
                match paint {
                    Paint::Color(col) => over(&mut c, [col[0], col[1], col[2], 255], a),
                    Paint::Pencil(col) => {
                        if d <= r {
                            over(&mut c, [col[0], col[1], col[2], 255], base);
                        }
                    }
                    Paint::Erase => c[3] *= 1.0 - a,
                    Paint::Dodge => (0..3).for_each(|k| c[k] += (1.0 - c[k]) * a * 0.12),
                    Paint::Burn => (0..3).for_each(|k| c[k] *= 1.0 - a * 0.12),
                    Paint::Sponge(dir) => {
                        let (hh, ss, ll) = rgb_to_hsl(c[0], c[1], c[2]);
                        let (r2, g2, b2) = hsl_to_rgb(hh, (ss + *dir * a * 0.08).clamp(0.0, 1.0), ll);
                        (c[0], c[1], c[2]) = (r2, g2, b2);
                    }
                    Paint::Blur(_) | Paint::Sharpen(_) => {
                        let sharpen = matches!(paint, Paint::Sharpen(_));
                        let src = match paint {
                            Paint::Blur(s) | Paint::Sharpen(s) => &*s,
                            _ => unreachable!(),
                        };
                        let mut avg = [0f32; 4];
                        let mut n = 0.0;
                        for yy in (y - 2).max(0)..=(y + 2).min(h - 1) {
                            for xx in (x - 2).max(0)..=(x + 2).min(w - 1) {
                                let q = src.get_pixel(xx as u32, yy as u32).0;
                                (0..4).for_each(|k| avg[k] += q[k] as f32 / 255.0);
                                n += 1.0;
                            }
                        }
                        let orig = src.get_pixel(x as u32, y as u32).0.map(|v| v as f32 / 255.0);
                        for k in 0..4 {
                            let target = if sharpen { orig[k] + (orig[k] - avg[k] / n) * 1.2 } else { avg[k] / n };
                            c[k] += (target - c[k]) * a * 0.5;
                        }
                    }
                    Paint::Smudge(Some(carry)) => {
                        let old = c;
                        (0..4).for_each(|k| c[k] += (carry[k] - c[k]) * a * 0.6);
                        (0..4).for_each(|k| picked[k] += old[k] * a);
                        picked_n += a;
                    }
                    Paint::Smudge(None) => {}
                    Paint::Clone { src, dx, dy } => {
                        let sx = (x as f32 + *dx).round() as i32;
                        let sy = (y as f32 + *dy).round() as i32;
                        if sx >= 0 && sy >= 0 && sx < w && sy < h {
                            let q = src.get_pixel(sx as u32, sy as u32).0;
                            (0..4).for_each(|k| c[k] += (q[k] as f32 / 255.0 - c[k]) * a);
                        }
                    }
                }
                px.0 = to_u8(c);
            }
        }
        if let Paint::Smudge(Some(carry)) = paint
            && picked_n > 0.0
        {
            (0..4).for_each(|k| carry[k] = carry[k] * 0.7 + picked[k] / picked_n * 0.3);
        }
        self.touch(Area { x0: x0 as u32, y0: y0 as u32, x1: x1 as u32, y1: y1 as u32 });
    }

    pub fn set_selection(&mut self, area: Option<Area>, mask: Option<Vec<bool>>) {
        self.mask = if area.is_some() { mask } else { None };
        self.selection = area;
        self.version += 1;
    }

    pub fn selected(&self, x: u32, y: u32) -> bool {
        inside(self.selection, self.mask.as_ref(), self.width(), x, y)
    }

    fn select_mask(&mut self, mask: Vec<bool>) {
        let w = self.width();
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for (i, _) in mask.iter().enumerate().filter(|m| *m.1) {
            let (x, y) = (i as u32 % w, i as u32 / w);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        }
        if x1 > x0 {
            self.set_selection(Some(Area { x0, y0, x1, y1 }), Some(mask));
        } else {
            self.set_selection(None, None);
        }
    }

    pub fn select_ellipse(&mut self, a: Area) {
        let w = self.width();
        let (cx, cy) = ((a.x0 + a.x1) as f32 / 2.0, (a.y0 + a.y1) as f32 / 2.0);
        let (rx, ry) = (a.width() as f32 / 2.0, a.height() as f32 / 2.0);
        let mask = (0..w * self.height())
            .map(|i| {
                let (x, y) = ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
                ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
            })
            .collect();
        self.select_mask(mask);
    }

    pub fn select_polygon(&mut self, pts: &[(f32, f32)]) {
        if pts.len() < 3 {
            return self.set_selection(None, None);
        }
        let (w, h) = (self.width(), self.height());
        let mut mask = vec![false; (w * h) as usize];
        for y in 0..h {
            let fy = y as f32 + 0.5;
            let mut xs: Vec<f32> = Vec::new();
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                if (a.1 <= fy) != (b.1 <= fy) {
                    xs.push(a.0 + (fy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(f32::total_cmp);
            for pair in xs.chunks(2).filter(|p| p.len() == 2) {
                let from = (pair[0] - 0.5).ceil().clamp(0.0, w as f32) as u32;
                let to = (pair[1] - 0.5).floor().min(w as f32 - 1.0);
                if to < from as f32 {
                    continue;
                }
                for x in from..=to as u32 {
                    mask[(y * w + x) as usize] = true;
                }
            }
        }
        self.select_mask(mask);
    }

    pub fn select_color(&mut self, x: u32, y: u32, tolerance: u8, contiguous: bool) {
        let (w, h) = (self.width(), self.height());
        if x >= w || y >= h {
            return;
        }
        let img = &self.layers[self.active].img;
        let target = img.get_pixel(x, y).0;
        let close = |p: [u8; 4]| p.iter().zip(target).all(|(a, b)| a.abs_diff(b) <= tolerance);
        let mut mask = vec![false; (w * h) as usize];
        if contiguous {
            let mut stack = vec![(x, y)];
            while let Some((px, py)) = stack.pop() {
                let i = (py * w + px) as usize;
                if mask[i] || !close(img.get_pixel(px, py).0) {
                    continue;
                }
                mask[i] = true;
                if px > 0 { stack.push((px - 1, py)); }
                if px + 1 < w { stack.push((px + 1, py)); }
                if py > 0 { stack.push((px, py - 1)); }
                if py + 1 < h { stack.push((px, py + 1)); }
            }
        } else {
            for (i, p) in img.pixels().enumerate() {
                mask[i] = close(p.0);
            }
        }
        self.select_mask(mask);
    }

    pub fn invert_selection(&mut self) {
        if self.selection.is_none() {
            return;
        }
        let w = self.width();
        let mask = (0..w * self.height()).map(|i| !self.selected(i % w, i / w)).collect();
        self.select_mask(mask);
    }

    pub fn put_filtered(&mut self, base: &RgbaImage, out: &RgbaImage) {
        let (sel, mask, w) = (self.selection, self.mask.as_ref(), self.width());
        let img = &mut self.layers[self.active].img;
        for (i, (d, (b, o))) in img.pixels_mut().zip(base.pixels().zip(out.pixels())).enumerate() {
            *d = if inside(sel, mask, w, i as u32 % w, i as u32 / w) { *o } else { *b };
        }
        self.touch_all();
    }

    pub fn fill_shape(&mut self, a: Area, ellipse: bool, color: [u8; 4]) {
        self.checkpoint();
        let (cx, cy) = ((a.x0 + a.x1) as f32 / 2.0, (a.y0 + a.y1) as f32 / 2.0);
        let (rx, ry) = (a.width() as f32 / 2.0, a.height() as f32 / 2.0);
        let (sel, mask, w) = (self.selection, self.mask.as_ref(), self.width());
        let img = &mut self.layers[self.active].img;
        for y in a.y0..a.y1 {
            for x in a.x0..a.x1 {
                if !inside(sel, mask, w, x, y) {
                    continue;
                }
                let cover = if ellipse {
                    let d = (((x as f32 + 0.5 - cx) / rx).powi(2) + ((y as f32 + 0.5 - cy) / ry).powi(2)).sqrt();
                    ((1.0 - d) * rx.min(ry) + 0.5).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                if cover > 0.0 {
                    let px = img.get_pixel_mut(x, y);
                    let mut c = px.0.map(|v| v as f32 / 255.0);
                    over(&mut c, color, cover);
                    px.0 = to_u8(c);
                }
            }
        }
        self.touch(a);
    }

    pub fn flood_fill(&mut self, x: u32, y: u32, color: [u8; 4], tolerance: u8) -> usize {
        let (w, h) = (self.width(), self.height());
        let sel = self.selection.unwrap_or(self.full());
        if x >= w || y >= h || !self.selected(x, y) {
            return 0;
        }
        let img = &mut self.layers[self.active].img;
        let target = img.get_pixel(x, y).0;
        if target == color {
            return 0;
        }
        let close = |p: [u8; 4]| p.iter().zip(target).all(|(a, b)| a.abs_diff(b) <= tolerance);
        let mask = self.mask.as_ref();
        let mut seen = vec![false; (w * h) as usize];
        let mut stack = vec![(x, y)];
        let mut count = 0;
        while let Some((px, py)) = stack.pop() {
            let i = (py * w + px) as usize;
            if seen[i] || !sel.contains(px, py) || mask.is_some_and(|m| !m[i]) || !close(img.get_pixel(px, py).0) {
                continue;
            }
            seen[i] = true;
            img.put_pixel(px, py, Rgba(color));
            count += 1;
            if px > 0 { stack.push((px - 1, py)); }
            if px + 1 < w { stack.push((px + 1, py)); }
            if py > 0 { stack.push((px, py - 1)); }
            if py + 1 < h { stack.push((px, py + 1)); }
        }
        self.touch_all();
        count
    }

    pub fn add_layer(&mut self, name: Option<String>, img: Option<RgbaImage>) {
        self.checkpoint();
        let (w, h) = (self.width(), self.height());
        let img = match img {
            Some(src) => {
                let mut canvas = RgbaImage::new(w, h);
                imageops::overlay(&mut canvas, &src, 0, 0);
                canvas
            }
            None => RgbaImage::new(w, h),
        };
        let name = name.unwrap_or_else(|| format!("Calque {}", self.layers.len() + 1));
        self.active += 1;
        self.layers.insert(self.active, Layer { name, img, visible: true, opacity: 1.0, blend: Blend::Normal });
        self.touch_all();
    }

    pub fn duplicate_layer(&mut self) {
        self.checkpoint();
        let mut l = self.layers[self.active].clone();
        l.name = format!("{} copie", l.name);
        self.active += 1;
        self.layers.insert(self.active, l);
        self.touch_all();
    }

    pub fn delete_layer(&mut self) {
        if self.layers.len() < 2 {
            return;
        }
        self.checkpoint();
        self.layers.remove(self.active);
        self.active = self.active.saturating_sub(1);
        self.touch_all();
    }

    pub fn move_layer(&mut self, up: bool) {
        let to = if up { self.active + 1 } else { self.active.wrapping_sub(1) };
        if to >= self.layers.len() {
            return;
        }
        self.checkpoint();
        self.layers.swap(self.active, to);
        self.active = to;
        self.touch_all();
    }

    pub fn merge_down(&mut self) {
        if self.active == 0 {
            return;
        }
        self.checkpoint();
        let top = self.layers.remove(self.active);
        self.active -= 1;
        let below = &mut self.layers[self.active].img;
        for (d, s) in below.pixels_mut().zip(top.img.pixels()) {
            let mut acc = [
                d.0[0] as f32 / 255.0,
                d.0[1] as f32 / 255.0,
                d.0[2] as f32 / 255.0,
                d.0[3] as f32 / 255.0,
            ];
            if top.visible {
                blend_over(&mut acc, s.0, top.opacity, top.blend);
            }
            d.0 = to_u8(acc);
        }
        self.touch_all();
    }

    pub fn clear_selection(&mut self) {
        let Some(sel) = self.selection else { return };
        self.checkpoint();
        for y in sel.y0..sel.y1 {
            for x in sel.x0..sel.x1 {
                if self.selected(x, y) {
                    self.layers[self.active].img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
            }
        }
        self.touch(sel);
    }

    pub fn crop_to_selection(&mut self) {
        let Some(s) = self.selection.take() else { return };
        self.mask = None;
        self.transform(|img| imageops::crop_imm(img, s.x0, s.y0, s.width(), s.height()).to_image());
    }

    pub fn place_layer(&mut self, original: &RgbaImage, dx: i32, dy: i32) {
        let mut out = RgbaImage::new(original.width(), original.height());
        imageops::overlay(&mut out, original, dx as i64, dy as i64);
        self.layers[self.active].img = out;
        self.touch_all();
    }

    pub fn gradient(&mut self, from: (f32, f32), to: (f32, f32), a: [u8; 4], b: [u8; 4]) {
        self.checkpoint();
        let sel = self.selection.unwrap_or(self.full());
        let d = (to.0 - from.0, to.1 - from.1);
        let len2 = (d.0 * d.0 + d.1 * d.1).max(1e-3);
        let (s0, mask, w) = (self.selection, self.mask.as_ref(), self.width());
        let img = &mut self.layers[self.active].img;
        for y in sel.y0..sel.y1 {
            for x in sel.x0..sel.x1 {
                if !inside(s0, mask, w, x, y) {
                    continue;
                }
                let t = (((x as f32 + 0.5 - from.0) * d.0 + (y as f32 + 0.5 - from.1) * d.1) / len2).clamp(0.0, 1.0);
                let c: [u8; 4] = std::array::from_fn(|k| (a[k] as f32 + (b[k] as f32 - a[k] as f32) * t).round() as u8);
                let px = img.get_pixel_mut(x, y);
                let mut acc = px.0.map(|v| v as f32 / 255.0);
                over(&mut acc, c, 1.0);
                px.0 = to_u8(acc);
            }
        }
        self.touch(sel);
    }

    pub fn transform(&mut self, f: impl Fn(&RgbaImage) -> RgbaImage) {
        self.checkpoint();
        for l in &mut self.layers {
            l.img = f(&l.img);
        }
        self.touch_all();
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn over(acc: &mut [f32; 4], p: [u8; 4], opacity: f32) {
    let a = p[3] as f32 / 255.0 * opacity;
    if a <= 0.0 {
        return;
    }
    let out_a = a + acc[3] * (1.0 - a);
    for c in 0..3 {
        acc[c] = (p[c] as f32 / 255.0 * a + acc[c] * acc[3] * (1.0 - a)) / out_a;
    }
    acc[3] = out_a;
}

fn blend_over(acc: &mut [f32; 4], p: [u8; 4], opacity: f32, mode: Blend) {
    if mode == Blend::Normal || acc[3] <= 0.0 {
        return over(acc, p, opacity);
    }
    let s = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
    let mixed: [f32; 3] = std::array::from_fn(|k| s[k] + (mode.mix(acc[k], s[k]) - s[k]) * acc[3]);
    over(acc, [(mixed[0] * 255.0).round() as u8, (mixed[1] * 255.0).round() as u8, (mixed[2] * 255.0).round() as u8, p[3]], opacity);
}

fn to_u8(acc: [f32; 4]) -> [u8; 4] {
    acc.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s == 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let f = |mut t: f32| {
        t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_fill_layers_undo() {
        let mut d = Document::new(64, 64, Some([255, 255, 255, 255]));
        let brush = Brush { radius: 4.0, hardness: 1.0, flow: 1.0 };
        d.checkpoint();
        d.stroke((10.0, 10.0), (50.0, 10.0), &brush, [255, 0, 0, 255], false);
        assert_eq!(d.layers[0].img.get_pixel(30, 10).0, [255, 0, 0, 255]);
        assert_eq!(d.layers[0].img.get_pixel(30, 40).0, [255, 255, 255, 255]);

        d.checkpoint();
        let filled = d.flood_fill(30, 40, [0, 0, 255, 255], 0);
        assert!(filled > 3000 && filled < 64 * 64);

        d.add_layer(None, None);
        d.layers[1].opacity = 0.5;
        d.checkpoint();
        d.stamp(30.5, 40.5, &brush, [0, 0, 0, 255], false);
        let flat = d.flatten();
        assert_eq!(flat.get_pixel(30, 40).0, [0, 0, 128, 255]);

        d.undo();
        d.undo();
        d.undo();
        assert_eq!(d.layers.len(), 1);
        assert_eq!(d.layers[0].img.get_pixel(30, 40).0, [255, 255, 255, 255]);
        d.redo();
        assert_eq!(d.layers[0].img.get_pixel(30, 40).0, [0, 0, 255, 255]);

        let mut s2 = Document::new(20, 20, Some([255, 255, 255, 255]));
        s2.selection = Area::from_points((5.0, 5.0), (10.0, 10.0), 20, 20);
        s2.checkpoint();
        s2.stamp(10.0, 10.0, &Brush { radius: 8.0, hardness: 1.0, flow: 1.0 }, [0, 0, 0, 255], false);
        assert_eq!(s2.layers[0].img.get_pixel(7, 7).0, [0, 0, 0, 255]);
        assert_eq!(s2.layers[0].img.get_pixel(12, 12).0, [255, 255, 255, 255]);
        s2.gradient((5.0, 0.0), (10.0, 0.0), [255, 0, 0, 255], [0, 0, 255, 255]);
        assert!(s2.layers[0].img.get_pixel(5, 6).0[0] > 200);
        assert!(s2.layers[0].img.get_pixel(9, 6).0[2] > 150);
        s2.clear_selection();
        assert_eq!(s2.layers[0].img.get_pixel(7, 7).0[3], 0);
        s2.crop_to_selection();
        assert_eq!((s2.width(), s2.height()), (5, 5));

        let mut m = Document::new(20, 20, Some([255, 255, 255, 255]));
        m.select_ellipse(Area { x0: 0, y0: 0, x1: 20, y1: 20 });
        assert!(m.selected(10, 10) && !m.selected(0, 0));
        m.invert_selection();
        assert!(!m.selected(10, 10) && m.selected(0, 0));
        m.select_polygon(&[(2.0, 2.0), (18.0, 2.0), (2.0, 18.0)]);
        assert!(m.selected(4, 4) && !m.selected(16, 16));
        m.checkpoint();
        m.stamp(10.0, 10.0, &Brush { radius: 20.0, hardness: 1.0, flow: 1.0 }, [0, 0, 0, 255], false);
        assert_eq!(m.layers[0].img.get_pixel(16, 16).0, [255, 255, 255, 255]);
        m.layers[0].img.put_pixel(5, 5, Rgba([10, 10, 10, 255]));
        m.set_selection(None, None);
        m.select_color(0, 0, 0, false);
        assert!(m.selected(15, 15) && !m.selected(4, 4));
        m.add_layer(None, Some(RgbaImage::from_pixel(20, 20, Rgba([128, 128, 128, 255]))));
        m.layers[1].blend = Blend::Multiply;
        assert_eq!(m.flatten().get_pixel(15, 15).0, [128, 128, 128, 255]);

        let (h, s, l) = rgb_to_hsl(0.2, 0.6, 0.4);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!((r - 0.2).abs() < 1e-4 && (g - 0.6).abs() < 1e-4 && (b - 0.4).abs() < 1e-4);
    }
}
