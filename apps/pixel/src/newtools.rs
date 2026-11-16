use crate::Pixel;
use crate::doc::Area;
pub use crate::algos::*;
use eframe::egui;
use image::{Rgba, RgbaImage};

impl Pixel {
    /// Quick action "Supprimer l'arrière-plan": the background touching the image edges is hidden
    /// with a layer mask (nothing is erased, paint the mask to fix it).
    pub fn remove_background(&mut self) {
        let (w, h) = (self.doc.width(), self.doc.height());
        let Some(all) = Area::from_points((0.0, 0.0), (w as f32, h as f32), w, h) else { return };
        let tol = 20.0 + self.tolerance as f32 * 1.2;
        let mut mask = object_mask(&self.composite, all, tol);
        clear_holes(&self.composite, &mut mask, tol);
        let kept = mask.iter().filter(|m| **m > 0).count();
        if kept == 0 || kept == mask.len() {
            self.error = Some("Arrière-plan introuvable : il doit toucher les bords et être assez uni".into());
            return;
        }
        let m = RgbaImage::from_fn(w, h, |x, y| {
            let v = mask[(y * w + x) as usize];
            Rgba([v, v, v, 255])
        });
        self.doc.next_label = Some("Supprimer l'arrière-plan".into());
        self.doc.checkpoint();
        let l = &mut self.doc.layers[self.doc.active];
        l.mask = Some(m);
        l.mask_on = true;
        self.doc.touch_all();
    }

    /// Photoshop's Generative Expand, done with content-aware fill: grows the canvas and rebuilds the new border.
    pub fn expand_image(&mut self, margin: f32, aspect: Option<f32>, ctx: &egui::Context) {
        let (w, h) = (self.doc.width(), self.doc.height());
        let (nw, nh, ox, oy) = expand_size(w, h, margin, aspect);
        if (nw, nh) == (w, h) {
            self.error = Some("L'image a déjà ce format".into());
            return;
        }
        let original = self.doc.layers[self.doc.active].img.clone();
        self.doc.next_label = Some("Étendre l'image".into());
        self.doc.checkpoint();
        self.doc.canvas_size(nw, nh, (0.5, 0.5), None);
        let mask: Vec<bool> = (0..nw * nh).map(|i| {
            let (x, y) = (i % nw, i / nw);
            x < ox || y < oy || x >= ox + w || y >= oy + h
        }).collect();
        self.fit_pending = true;
        self.expand_box = Some((original, ox, oy));
        self.start_fill(mask, ctx);
    }

    /// Matches the active layer's colours and contrast to what lies beneath it (Photoshop's Harmonize).
    pub fn harmonize_layer(&mut self, amount: f32) {
        let layer = self.doc.active;
        if self.doc.layers.len() < 2 || layer == 0 {
            self.error = Some("Harmoniser : place un calque au-dessus d'une image".into());
            return;
        }
        let src = self.doc.layers[layer].img.clone();
        let below = self.doc.layers[..layer].iter().filter(|l| l.visible).fold(None::<RgbaImage>, |acc, l| {
            let mut base = acc.unwrap_or_else(|| RgbaImage::new(l.img.width(), l.img.height()));
            image::imageops::overlay(&mut base, &l.img, 0, 0);
            Some(base)
        });
        let Some(below) = below else { return };
        let from = color_stats(&src, |x, y| src.get_pixel(x, y).0[3] > 128);
        let to = color_stats(&below, |x, y| below.get_pixel(x, y).0[3] > 128);
        self.doc.next_label = Some("Harmoniser".into());
        self.doc.checkpoint();
        self.doc.layers[layer].img = harmonize(&src, from, to, amount);
        self.doc.touch_all();
    }

    pub fn object_select(&mut self) {
        let Some(a) = self.doc.selection else { return };
        let prev = self.sel_prev.take();
        let mask = object_mask(&self.composite, a, 20.0 + self.tolerance as f32 * 1.2);
        if mask.iter().any(|m| *m > 0) {
            self.doc.select_mask(mask);
            self.doc.feather_selection(self.feather);
        }
        self.doc.combine_selection(prev, self.sel_mode);
    }

    pub fn fix_red_eye(&mut self, x: f32, y: f32) {
        let base = self.doc.active_target();
        let r = self.brush.radius.max(4.0);
        let out = red_eye(&base, x, y, r, self.red_eye_dark);
        self.doc.next_label = Some("Yeux rouges".into());
        self.doc.checkpoint();
        self.doc.put_filtered(&base, &out);
    }

    pub fn content_move(&mut self, offset: (f32, f32), ctx: &egui::Context) {
        if self.doc.selection.is_none() {
            return;
        }
        let (w, h) = (self.doc.width(), self.doc.height());
        let (dx, dy) = (offset.0.round() as i64, offset.1.round() as i64);
        let sel: Vec<bool> = (0..w * h).map(|i| self.doc.selected(i % w, i / w)).collect();
        let layer = self.doc.active;
        let base = self.doc.layers[layer].img.clone();
        let mut out = base.clone();
        let mut dest = vec![false; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                if !sel[(y * w + x) as usize] {
                    continue;
                }
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                    out.put_pixel(nx as u32, ny as u32, *base.get_pixel(x, y));
                    dest[(ny as u32 * w + nx as u32) as usize] = true;
                }
            }
        }
        self.doc.next_label = Some("Déplacement basé sur le contenu".into());
        self.doc.checkpoint();
        self.doc.layers[layer].img = out;
        self.doc.touch_all();
        let hole: Vec<bool> = sel.iter().zip(&dest).map(|(s, d)| *s && !*d).collect();
        self.doc.set_selection(None, None);
        self.start_fill(hole, ctx);
    }

    pub fn perspective_crop(&mut self) {
        if self.persp.len() != 4 {
            return;
        }
        let q = [self.persp[0], self.persp[1], self.persp[2], self.persp[3]];
        let len = |a: (f32, f32), b: (f32, f32)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let ow = ((len(q[0], q[1]) + len(q[3], q[2])) / 2.0).round().max(1.0) as u32;
        let oh = ((len(q[0], q[3]) + len(q[1], q[2])) / 2.0).round().max(1.0) as u32;
        self.persp.clear();
        self.doc.set_selection(None, None);
        self.doc.next_label = Some("Recadrage de la perspective".into());
        self.doc.transform(|img| warp_quad(img, q, ow, oh));
        self.fit_pending = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_sizes() {
        assert_eq!(expand_size(100, 50, 0.1, None), (120, 60, 10, 5));
        assert_eq!(expand_size(160, 90, 0.0, Some(1.0)), (160, 160, 0, 35));
        assert_eq!(expand_size(90, 160, 0.0, Some(16.0 / 9.0)), (284, 160, 97, 0));
        assert_eq!(expand_size(100, 100, 0.0, Some(1.0)), (100, 100, 0, 0));
    }

    #[test]
    fn harmonize_pulls_toward_target() {
        let src = RgbaImage::from_pixel(4, 4, Rgba([200, 60, 60, 255]));
        let from = color_stats(&src, |_, _| true);
        let to = ([60.0, 90.0, 200.0], [10.0, 10.0, 10.0]);
        let out = harmonize(&src, from, to, 1.0);
        assert_eq!(out.get_pixel(0, 0).0, [60, 90, 200, 255]);
        let half = harmonize(&src, from, to, 0.5);
        assert_eq!(half.get_pixel(0, 0).0, [130, 75, 130, 255]);
    }

    #[test]
    fn object_red_eye_warp_curve() {
        let mut img = RgbaImage::from_pixel(40, 40, Rgba([240, 240, 240, 255]));
        for y in 15..25 {
            for x in 12..28 {
                img.put_pixel(x, y, Rgba([200, 20, 20, 255]));
            }
        }
        let a = Area::from_points((5.0, 5.0), (35.0, 35.0), 40, 40).unwrap();
        let m = object_mask(&img, a, 30.0);
        assert_eq!(m[20 * 40 + 20], 255);
        assert_eq!(m[8 * 40 + 8], 0);
        assert_eq!(m.iter().filter(|v| **v > 0).count(), 160);
        let fixed = red_eye(&img, 20.0, 20.0, 6.0, 0.5);
        assert!(fixed.get_pixel(20, 20).0[0] < 60);
        assert_eq!(fixed.get_pixel(2, 2), img.get_pixel(2, 2));
        let wq = warp_quad(&img, [(12.0, 15.0), (28.0, 15.0), (28.0, 25.0), (12.0, 25.0)], 16, 10);
        assert!(wq.pixels().all(|p| p.0[0] == 200));
        let c = catmull(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], 8);
        assert_eq!(c.len(), 32);
        assert_eq!(c[0], (0.0, 0.0));
    }
}
