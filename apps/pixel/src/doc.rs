use image::{Rgba, RgbaImage, imageops};

#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub img: RgbaImage,
    pub visible: bool,
    pub opacity: f32,
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
            layers: vec![Layer { name: layer, img, visible: true, opacity: 1.0 }],
            active: 0,
            dirty: None,
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
        self.dirty = Some(self.full());
    }

    fn touch(&mut self, a: Area) {
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
                    let p = l.img.get_pixel(x, y).0;
                    over(&mut acc, p, l.opacity);
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
        let (w, h) = (self.width() as i32, self.height() as i32);
        let r = brush.radius.max(0.5);
        let x0 = ((cx - r - 1.0).floor() as i32).clamp(0, w);
        let x1 = ((cx + r + 1.0).ceil() as i32).clamp(0, w);
        let y0 = ((cy - r - 1.0).floor() as i32).clamp(0, h);
        let y1 = ((cy + r + 1.0).ceil() as i32).clamp(0, h);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let hard = brush.hardness.clamp(0.0, 0.99);
        let base = color[3] as f32 / 255.0 * brush.flow;
        let img = &mut self.layers[self.active].img;
        for y in y0..y1 {
            for x in x0..x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                let edge = (r + 0.5 - d).clamp(0.0, 1.0);
                if edge <= 0.0 {
                    continue;
                }
                let t = d / r;
                let soft = if t <= hard { 1.0 } else { smooth(1.0 - (t - hard) / (1.0 - hard)) };
                let a = base * edge * soft;
                let px = img.get_pixel_mut(x as u32, y as u32);
                if erase {
                    px.0[3] = (px.0[3] as f32 * (1.0 - a)).round() as u8;
                } else {
                    let mut acc = [
                        px.0[0] as f32 / 255.0,
                        px.0[1] as f32 / 255.0,
                        px.0[2] as f32 / 255.0,
                        px.0[3] as f32 / 255.0,
                    ];
                    over(&mut acc, [color[0], color[1], color[2], 255], a);
                    px.0 = to_u8(acc);
                }
            }
        }
        self.touch(Area { x0: x0 as u32, y0: y0 as u32, x1: x1 as u32, y1: y1 as u32 });
    }

    pub fn stroke(&mut self, from: (f32, f32), to: (f32, f32), brush: &Brush, color: [u8; 4], erase: bool) {
        let dist = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
        let step = (brush.radius * 0.25).max(0.5);
        let n = (dist / step).ceil().max(1.0) as usize;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            self.stamp(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t, brush, color, erase);
        }
    }

    pub fn flood_fill(&mut self, x: u32, y: u32, color: [u8; 4], tolerance: u8) -> usize {
        let (w, h) = (self.width(), self.height());
        if x >= w || y >= h {
            return 0;
        }
        let img = &mut self.layers[self.active].img;
        let target = img.get_pixel(x, y).0;
        if target == color {
            return 0;
        }
        let close = |p: [u8; 4]| p.iter().zip(target).all(|(a, b)| a.abs_diff(b) <= tolerance);
        let mut seen = vec![false; (w * h) as usize];
        let mut stack = vec![(x, y)];
        let mut count = 0;
        while let Some((px, py)) = stack.pop() {
            let i = (py * w + px) as usize;
            if seen[i] || !close(img.get_pixel(px, py).0) {
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
        self.layers.insert(self.active, Layer { name, img, visible: true, opacity: 1.0 });
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
                over(&mut acc, s.0, top.opacity);
            }
            d.0 = to_u8(acc);
        }
        self.touch_all();
    }

    pub fn adjust(&mut self, f: impl Fn(&mut [u8; 4]) + Sync) {
        self.checkpoint();
        for p in self.layers[self.active].img.pixels_mut() {
            f(&mut p.0);
        }
        self.touch_all();
    }

    pub fn blur(&mut self, sigma: f32) {
        self.checkpoint();
        let l = &mut self.layers[self.active];
        l.img = imageops::blur(&l.img, sigma);
        self.touch_all();
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

        let (h, s, l) = rgb_to_hsl(0.2, 0.6, 0.4);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!((r - 0.2).abs() < 1e-4 && (g - 0.6).abs() < 1e-4 && (b - 0.4).abs() < 1e-4);
    }
}
