//! Native layered project (.epix): an 8-byte signature, a JSON manifest, then every layer and mask as PNG.
//! Each block is prefixed by its length (u32, little endian), so the file can be read back without guessing.

use crate::doc::{BLENDS, Document};
use image::{ImageEncoder, RgbaImage};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 8] = b"EPIX\0\x01\0\0";

#[derive(Serialize, Deserialize)]
struct Manifest {
    name: String,
    active: usize,
    layers: Vec<LayerMeta>,
}

#[derive(Serialize, Deserialize)]
struct LayerMeta {
    name: String,
    visible: bool,
    opacity: f32,
    blend: usize,
    locked: bool,
    lock_alpha: bool,
    lock_pos: bool,
    clip: bool,
    tag: u8,
    mask: bool,
    mask_on: bool,
    adjust: Option<(usize, Vec<f32>)>,
}

fn png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(&mut out, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::Adaptive)
        .write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

fn push_block(out: &mut Vec<u8>, block: &[u8]) {
    out.extend_from_slice(&(block.len() as u32).to_le_bytes());
    out.extend_from_slice(block);
}

pub fn encode(doc: &Document) -> Result<Vec<u8>, String> {
    let layers = doc
        .layers
        .iter()
        .map(|l| LayerMeta {
            name: l.name.clone(),
            visible: l.visible,
            opacity: l.opacity,
            blend: BLENDS.iter().position(|(b, _)| *b == l.blend).unwrap_or(0),
            locked: l.locked,
            lock_alpha: l.lock_alpha,
            lock_pos: l.lock_pos,
            clip: l.clip,
            tag: l.tag,
            mask: l.mask.is_some(),
            mask_on: l.mask_on,
            adjust: l.adjust.as_ref().and_then(|(k, p)| studio_fx::filters::ADJUSTMENTS.iter().position(|a| a == k).map(|i| (i, p.clone()))),
        })
        .collect();
    let manifest = serde_json::to_vec(&Manifest { name: doc.name.clone(), active: doc.active, layers }).map_err(|e| e.to_string())?;
    let mut out = MAGIC.to_vec();
    push_block(&mut out, &manifest);
    for l in &doc.layers {
        push_block(&mut out, &png(&l.img)?);
        if let Some(m) = &l.mask {
            push_block(&mut out, &png(m)?);
        }
    }
    Ok(out)
}

pub fn decode(bytes: &[u8]) -> Result<Document, String> {
    let rest = bytes.strip_prefix(&MAGIC[..]).ok_or("ce fichier n'est pas un projet Pixel")?;
    let mut blocks = Blocks(rest);
    let manifest: Manifest = serde_json::from_slice(blocks.next()?).map_err(|e| format!("manifeste illisible : {e}"))?;
    let mut doc: Option<Document> = None;
    for meta in &manifest.layers {
        let img = image::load_from_memory(blocks.next()?).map_err(|e| e.to_string())?.into_rgba8();
        let mask = if meta.mask { Some(image::load_from_memory(blocks.next()?).map_err(|e| e.to_string())?.into_rgba8()) } else { None };
        let d = match &mut doc {
            None => doc.insert(Document::from_image(manifest.name.clone(), img)),
            Some(d) => {
                d.add_layer(Some(meta.name.clone()), Some(img));
                d
            }
        };
        let l = d.layers.last_mut().expect("a layer was just added");
        l.name = meta.name.clone();
        l.visible = meta.visible;
        l.opacity = meta.opacity;
        l.blend = BLENDS.get(meta.blend).map_or(l.blend, |(b, _)| *b);
        (l.locked, l.lock_alpha, l.lock_pos, l.clip, l.tag, l.mask_on) = (meta.locked, meta.lock_alpha, meta.lock_pos, meta.clip, meta.tag, meta.mask_on);
        l.mask = mask;
        l.adjust = meta.adjust.as_ref().and_then(|(i, p)| studio_fx::filters::ADJUSTMENTS.get(*i).map(|k| (*k, p.clone())));
    }
    let mut doc = doc.ok_or("projet sans calque")?;
    doc.active = manifest.active.min(doc.layers.len() - 1);
    doc.clear_history();
    doc.touch_all();
    doc.modified = false;
    Ok(doc)
}

struct Blocks<'a>(&'a [u8]);

impl<'a> Blocks<'a> {
    fn next(&mut self) -> Result<&'a [u8], String> {
        let (len, rest) = self.0.split_first_chunk::<4>().ok_or("fichier tronqué")?;
        let len = u32::from_le_bytes(*len) as usize;
        let block = rest.get(..len).ok_or("fichier tronqué")?;
        self.0 = &rest[len..];
        Ok(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Blend;
    use image::Rgba;

    #[test]
    fn layers_masks_and_settings_survive_a_round_trip() {
        let mut doc = Document::from_image("affiche".into(), RgbaImage::from_pixel(8, 6, Rgba([200, 10, 10, 255])));
        doc.add_layer(Some("Texte".into()), Some(RgbaImage::from_pixel(8, 6, Rgba([0, 0, 255, 128]))));
        {
            let l = doc.layers.last_mut().unwrap();
            (l.visible, l.opacity, l.blend, l.clip, l.tag, l.lock_alpha) = (false, 0.4, Blend::Multiply, true, 3, true);
            l.mask = Some(RgbaImage::from_pixel(8, 6, Rgba([255, 255, 255, 90])));
            l.mask_on = false;
        }
        doc.active = 1;
        let back = decode(&encode(&doc).unwrap()).unwrap();
        assert_eq!(back.name, "affiche");
        assert_eq!(back.layers.len(), 2);
        assert_eq!(back.active, 1);
        assert_eq!(back.layers[0].img.get_pixel(3, 3).0, [200, 10, 10, 255]);
        let t = &back.layers[1];
        assert_eq!((t.name.as_str(), t.visible, t.opacity, t.blend, t.clip, t.tag, t.lock_alpha, t.mask_on), ("Texte", false, 0.4, Blend::Multiply, true, 3, true, false));
        assert_eq!(t.img.get_pixel(0, 0).0, [0, 0, 255, 128]);
        assert_eq!(t.mask.as_ref().unwrap().get_pixel(1, 1).0, [255, 255, 255, 90]);
        assert!(!back.modified);
    }

    #[test]
    fn truncated_or_foreign_files_are_refused() {
        let doc = Document::from_image("x".into(), RgbaImage::new(2, 2));
        let bytes = encode(&doc).unwrap();
        assert!(decode(&bytes[..bytes.len() - 3]).is_err());
        assert!(decode(b"\x89PNG not a project").is_err());
    }
}
