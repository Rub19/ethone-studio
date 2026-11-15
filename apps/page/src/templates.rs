use crate::model::{Doc, Frame, Kind, Page};

pub const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.";

pub const PRESETS: [(&str, &str, f32, f32); 18] = [
    ("A4", "Document, magazine", 595.0, 842.0),
    ("A5", "Livret, flyer", 420.0, 595.0),
    ("A3", "Affiche", 842.0, 1191.0),
    ("A6", "Carte, flyer de poche", 298.0, 420.0),
    ("B5", "Livre, catalogue", 499.0, 709.0),
    ("Lettre US", "Document", 612.0, 792.0),
    ("Légal US", "Contrat", 612.0, 1008.0),
    ("Tabloïd", "Journal", 792.0, 1224.0),
    ("Livre 6 × 9 in", "Roman, essai", 432.0, 648.0),
    ("DL", "Dépliant, enveloppe", 281.0, 595.0),
    ("Carte de visite", "85 × 55 mm", 241.0, 156.0),
    ("Carte postale", "A6 paysage", 420.0, 298.0),
    ("Carré", "Publication 1:1", 810.0, 810.0),
    ("Portrait 4:5", "Instagram", 810.0, 1012.0),
    ("Story", "Réseaux sociaux", 540.0, 960.0),
    ("Présentation", "16:9", 960.0, 540.0),
    ("Bannière web", "1200 × 628 px", 900.0, 471.0),
    ("Miniature vidéo", "1280 × 720 px", 960.0, 540.0),
];

pub const TEMPLATES: [(&str, &str); 14] = [
    ("Magazine", "A4, en-tête, colonnes"),
    ("Flyer", "A5, événement"),
    ("CV", "A4, colonne latérale"),
    ("Carte de visite", "85 × 55 mm"),
    ("Menu", "A4, carte et prix"),
    ("Affiche", "A3, grand titre"),
    ("Newsletter", "A4, trois rubriques"),
    ("Invitation", "A5, cérémonie"),
    ("Certificat", "A4 paysage, diplôme"),
    ("Brochure 3 volets", "A4 paysage, recto"),
    ("Couverture de livre", "6 × 9 in, plat recto"),
    ("Fiche produit", "A4, photo et caractéristiques"),
    ("Carte de vœux", "A6, fête"),
    ("Rapport", "A4, couverture et sommaire"),
];

pub fn txt(text: &str, size: f32, bold: bool, color: [u8; 4], align: u8) -> Kind {
    let mut k = Kind::text(text, size, bold);
    if let Kind::Text { color: c, align: a, .. } = &mut k {
        *c = color;
        *a = align;
    }
    k
}

pub fn styled(mut k: Kind, family: &str, italic: bool, leading: f32, tracking: f32) -> Kind {
    if let Kind::Text { family: f, italic: i, leading: l, tracking: t, .. } = &mut k {
        *f = family.into();
        *i = italic;
        *l = leading;
        *t = tracking;
    }
    k
}

fn line(color: [u8; 4], width: f32) -> Kind {
    Kind::line(color, width)
}

pub fn starter(w: f32, h: f32) -> Doc {
    let mut doc = Doc { width: w, height: h, margin: (w.min(h) * 0.067).round(), ..Doc::a4() };
    let m = doc.margin;
    let title_size = (w * 0.055).clamp(14.0, 40.0);
    doc.pages[0].frames.push(Frame::new(m, m + 20.0, w - 2.0 * m, title_size * 1.6, Kind::text("Titre du document", title_size, true)));
    doc.master.frames.push(Frame::new(m, h - m * 0.75, w - 2.0 * m, 14.0, txt("{page} / {pages}", 8.0, false, [120, 120, 130, 255], 2)));
    doc
}

fn table(cells: Vec<Vec<String>>, size: f32, family: &str, color: [u8; 4], line: [u8; 4], header: Option<[u8; 4]>, stripe: Option<[u8; 4]>) -> Kind {
    Kind::Table { cells, size, family: family.into(), color, line, header, stripe, align: 0, first_col: false }
}

pub fn template(k: usize) -> Doc {
    let dark = [24, 24, 30, 255];
    let white = [255, 255, 255, 255];
    let accent = [255, 112, 52, 255];
    let gold = [190, 150, 70, 255];
    match k {
        0 => {
            let mut d = Doc { grid: 2, title: "Ethone Mag".into(), swatches: vec![accent, dark, [255, 214, 10, 255], [10, 132, 255, 255]], ..Doc::a4() };
            let (w, m) = (d.width, d.margin);
            let mut head = Frame::new(-9.0, -9.0, w + 18.0, 200.0, Kind::Rect { fill: accent, radius: 0.0 });
            head.fill2 = Some([255, 64, 96, 255]);
            head.grad_angle = 0.0;
            d.bleed = 9.0;
            let p = &mut d.pages[0].frames;
            p.push(head);
            p.push(Frame::new(m, 60.0, w - 2.0 * m, 44.0, txt("Le magazine Ethone", 34.0, true, white, 0)));
            p.push(Frame::new(m, 110.0, w * 0.6, 40.0, styled(txt("Mise en page, colonnes liées, styles et export PDF prêt pour l'imprimeur.", 12.0, false, white, 0), "Segoe UI", true, 1.35, 0.0)));
            p.push(Frame::new(m, 230.0, w - 2.0 * m, 160.0, Kind::image("", 6.0)));
            let mut body = Frame::new(m, 410.0, w - 2.0 * m, 250.0, txt(&format!("{LOREM}\n{LOREM}\n{LOREM}\n{LOREM}"), 9.5, false, dark, 3));
            if let Kind::Text { columns, para_space, family, indent, opts, .. } = &mut body.kind {
                *columns = 2;
                *para_space = 6.0;
                *indent = 12.0;
                *family = "Georgia".into();
                opts.drop_lines = 3;
                opts.hyphen = true;
            }
            p.push(body);
            let mut quote = Frame::new(m, 680.0, w - 2.0 * m, 70.0, styled(txt("« Une suite créative entièrement écrite en Rust. »", 16.0, false, accent, 1), "Georgia", true, 1.3, 0.0));
            if let Kind::Text { background, padding, valign, .. } = &mut quote.kind {
                *background = Some([255, 243, 236, 255]);
                *padding = 12.0;
                *valign = 1;
            }
            quote.fx.corner = 1;
            p.push(quote);
            d.pages.push(Page::default());
            d.master.frames.push(Frame::new(m, d.height - 30.0, w - 2.0 * m, 12.0, txt("{title} · {page} / {pages}", 7.5, false, [130, 130, 140, 255], 2)));
            d
        }
        1 => {
            let mut d = Doc { width: 420.0, height: 595.0, margin: 28.0, bleed: 9.0, ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let mut bg = Frame::new(-9.0, -9.0, w + 18.0, h + 18.0, Kind::Rect { fill: [36, 18, 90, 255], radius: 0.0 });
            bg.fill2 = Some([255, 64, 120, 255]);
            let p = &mut d.pages[0].frames;
            p.push(bg);
            let mut badge = Frame::new(w - 150.0, 40.0, 120.0, 120.0, Kind::Polygon { sides: 12, inner: 0.8, fill: [255, 214, 10, 255] });
            badge.fx.glow = true;
            p.push(badge);
            p.push(Frame::new(w - 150.0, 82.0, 120.0, 36.0, txt("GRATUIT", 15.0, true, dark, 1)));
            p.push(Frame::new(m, 200.0, w - 2.0 * m, 140.0, styled(txt("SOIRÉE\nCRÉATIVE", 46.0, true, white, 0), "Bahnschrift", false, 1.0, 2.0)));
            p.push(Frame::new(m, 350.0, w - 2.0 * m, 24.0, txt("Samedi 14 décembre · 20 h", 14.0, true, [255, 214, 10, 255], 0)));
            p.push(Frame::new(m, 385.0, 120.0, 6.0, line(white, 2.0)));
            p.push(Frame::new(m, 405.0, w - 2.0 * m, 120.0, txt("Ateliers de design\nConcerts\nExposition photo\nBuvette et surprises", 12.0, false, white, 0)));
            if let Kind::Text { list, opts, .. } = &mut p.last_mut().unwrap().kind {
                *list = 1;
                opts.bullet = "★".into();
            }
            p.push(Frame::new(m, h - 50.0, w - 2.0 * m, 14.0, txt("ethone.dev · entrée libre", 9.0, false, [220, 220, 240, 255], 0)));
            d
        }
        2 => {
            let mut d = Doc { margin: 36.0, ..Doc::a4() };
            let (w, h) = (d.width, d.height);
            let side = 190.0;
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(0.0, 0.0, side, h, Kind::Rect { fill: [28, 36, 54, 255], radius: 0.0 }));
            let mut photo = Frame::new(45.0, 40.0, 100.0, 100.0, Kind::image("", 0.0));
            if let Kind::Image { shape, .. } = &mut photo.kind {
                *shape = 1;
            }
            p.push(photo);
            p.push(Frame::new(24.0, 165.0, side - 48.0, 16.0, txt("CONTACT", 9.0, true, [140, 170, 255, 255], 0)));
            p.push(Frame::new(24.0, 185.0, side - 48.0, 70.0, txt("prenom@exemple.fr\n06 00 00 00 00\nParis, France", 9.0, false, white, 0)));
            p.push(Frame::new(24.0, 270.0, side - 48.0, 16.0, txt("COMPÉTENCES", 9.0, true, [140, 170, 255, 255], 0)));
            let mut skills = Frame::new(24.0, 290.0, side - 48.0, 120.0, txt("Design graphique\nMise en page\nMontage vidéo\nPhotographie", 9.0, false, white, 0));
            if let Kind::Text { list, para_space, .. } = &mut skills.kind {
                *list = 1;
                *para_space = 3.0;
            }
            p.push(skills);
            let x = side + 30.0;
            let cw = w - x - 36.0;
            p.push(Frame::new(x, 46.0, cw, 40.0, txt("Prénom Nom", 30.0, true, dark, 0)));
            p.push(Frame::new(x, 88.0, cw, 18.0, styled(txt("Graphiste et monteur vidéo", 13.0, false, [90, 100, 130, 255], 0), "Segoe UI", true, 1.2, 0.0)));
            p.push(Frame::new(x, 120.0, cw, 4.0, line([28, 36, 54, 255], 1.5)));
            for (k, (title, body)) in [("EXPÉRIENCE", "2022 – aujourd'hui · Studio créatif\nConception d'identités visuelles et de supports imprimés.\n\n2019 – 2022 · Agence web\nMaquettes, vidéos courtes et réseaux sociaux."), ("FORMATION", "2019 · Diplôme en design graphique\nÉcole d'arts appliqués")].iter().enumerate() {
                let y = 140.0 + k as f32 * 210.0;
                p.push(Frame::new(x, y, cw, 18.0, txt(title, 11.0, true, [60, 90, 200, 255], 0)));
                let mut b = Frame::new(x, y + 24.0, cw, 170.0, txt(body, 10.0, false, dark, 0));
                if let Kind::Text { para_space, .. } = &mut b.kind {
                    *para_space = 2.0;
                }
                p.push(b);
            }
            d
        }
        3 => {
            let mut d = Doc { width: 241.0, height: 156.0, margin: 14.0, bleed: 6.0, ..Doc::a4() };
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(-6.0, -6.0, 78.0, 168.0, Kind::Rect { fill: [10, 132, 255, 255], radius: 0.0 }));
            p.push(Frame::new(16.0, 52.0, 40.0, 40.0, Kind::Polygon { sides: 6, inner: 1.0, fill: white }));
            p.push(Frame::new(88.0, 34.0, 140.0, 20.0, txt("Prénom Nom", 15.0, true, dark, 0)));
            p.push(Frame::new(88.0, 56.0, 140.0, 14.0, styled(txt("Directeur artistique", 8.5, false, [10, 132, 255, 255], 0), "Segoe UI", true, 1.2, 0.0)));
            p.push(Frame::new(88.0, 76.0, 60.0, 4.0, line([200, 200, 210, 255], 1.0)));
            p.push(Frame::new(88.0, 86.0, 140.0, 50.0, txt("06 00 00 00 00\nprenom@exemple.fr\nexemple.fr", 7.5, false, [70, 70, 80, 255], 0)));
            d
        }
        4 => {
            let mut d = Doc { margin: 50.0, paper: [250, 246, 238, 255], ..Doc::a4() };
            let (w, m) = (d.width, d.margin);
            let brown = [92, 58, 34, 255];
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(m, 50.0, w - 2.0 * m, 50.0, styled(txt("La Table", 38.0, false, brown, 1), "Georgia", true, 1.1, 1.0)));
            p.push(Frame::new(m, 104.0, w - 2.0 * m, 16.0, styled(txt("CUISINE DE SAISON", 9.0, true, [160, 120, 80, 255], 1), "Segoe UI", false, 1.2, 3.0)));
            p.push(Frame::new(w / 2.0 - 40.0, 128.0, 80.0, 4.0, line(brown, 1.0)));
            for (k, (title, rows)) in [("Entrées", [("Velouté de potimarron", "9 €"), ("Burrata et tomates anciennes", "12 €"), ("Œuf parfait, champignons", "11 €")]), ("Plats", [("Risotto aux cèpes", "19 €"), ("Filet de bar, beurre blanc", "24 €"), ("Joue de bœuf confite", "22 €")]), ("Desserts", [("Tarte au citron meringuée", "8 €"), ("Mousse au chocolat noir", "7 €"), ("Fromages affinés", "9 €")])].iter().enumerate() {
                let y = 160.0 + k as f32 * 200.0;
                p.push(Frame::new(m, y, w - 2.0 * m, 26.0, styled(txt(title, 20.0, false, brown, 1), "Georgia", true, 1.1, 0.0)));
                let body = rows.iter().map(|(a, b)| format!("{a}\t{b}")).collect::<Vec<_>>().join("\n");
                let mut list = Frame::new(m + 20.0, y + 40.0, w - 2.0 * m - 40.0, 96.0, styled(txt(&body, 11.0, false, [60, 40, 30, 255], 0), "Georgia", false, 1.4, 0.0));
                if let Kind::Text { para_space, opts, .. } = &mut list.kind {
                    *para_space = 8.0;
                    opts.leader = true;
                    opts.tab_right = true;
                }
                p.push(list);
            }
            d
        }
        5 => {
            let mut d = Doc { width: 842.0, height: 1191.0, margin: 50.0, ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(0.0, 0.0, w, h * 0.55, Kind::image("", 0.0)));
            p.push(Frame::new(0.0, h * 0.55, w, h * 0.45, Kind::Rect { fill: [14, 14, 18, 255], radius: 0.0 }));
            p.push(Frame::new(m, h * 0.58, w - 2.0 * m, 220.0, styled(txt("FESTIVAL\nLUMIÈRE", 92.0, true, white, 0), "Bahnschrift", false, 0.95, 4.0)));
            p.push(Frame::new(m, h * 0.78, w - 2.0 * m, 40.0, txt("12 — 15 juin · Parc des expositions", 26.0, false, [255, 214, 10, 255], 0)));
            p.push(Frame::new(w - m - 150.0, h * 0.5, 150.0, 150.0, Kind::Polygon { sides: 16, inner: 0.82, fill: [255, 214, 10, 255] }));
            p.push(Frame::new(w - m - 150.0, h * 0.5 + 55.0, 150.0, 40.0, txt("4 JOURS", 22.0, true, dark, 1)));
            p.push(Frame::new(m, h - 90.0, w - 2.0 * m, 30.0, txt("billetterie sur festival-lumiere.fr", 16.0, false, [180, 180, 190, 255], 0)));
            d
        }
        6 => {
            let blue = [20, 92, 220, 255];
            let mut d = Doc { margin: 36.0, grid: 3, title: "La lettre".into(), swatches: vec![blue, dark, [240, 244, 252, 255]], ..Doc::a4() };
            let (w, m) = (d.width, d.margin);
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(m, 34.0, w - 2.0 * m, 40.0, styled(txt("La lettre", 32.0, true, blue, 0), "Georgia", false, 1.0, 0.0)));
            p.push(Frame::new(m, 76.0, w - 2.0 * m, 14.0, txt("N° 12 · {date} · Les nouvelles du collectif", 9.0, false, [110, 110, 120, 255], 0)));
            p.push(Frame::new(m, 96.0, w - 2.0 * m, 4.0, line(blue, 2.0)));
            p.push(Frame::new(m, 116.0, w - 2.0 * m, 200.0, Kind::image("", 4.0)));
            let cw = (w - 2.0 * m - 24.0) / 3.0;
            for (k, t) in ["À la une", "Agenda", "Portrait"].iter().enumerate() {
                let x = m + k as f32 * (cw + 12.0);
                let mut h = Frame::new(x, 334.0, cw, 18.0, txt(t, 13.0, true, dark, 0));
                if let Kind::Text { opts, .. } = &mut h.kind {
                    opts.rule_below = true;
                    opts.rule_color = blue;
                    opts.rule_off = 3.0;
                }
                p.push(h);
                let mut b = Frame::new(x, 362.0, cw, 380.0, txt(LOREM, 9.0, false, dark, 3));
                if let Kind::Text { opts, .. } = &mut b.kind {
                    opts.hyphen = true;
                }
                p.push(b);
            }
            let mut box_ = Frame::new(m, 752.0, w - 2.0 * m, 50.0, txt("Abonnez-vous : lalettre@exemple.fr", 11.0, true, blue, 1));
            if let Kind::Text { background, valign, .. } = &mut box_.kind {
                *background = Some([240, 244, 252, 255]);
                *valign = 1;
            }
            box_.fx.corner = 1;
            p.push(box_);
            d
        }
        7 => {
            let mut d = Doc { width: 420.0, height: 595.0, margin: 40.0, paper: [252, 250, 245, 255], ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let p = &mut d.pages[0].frames;
            let mut border = Frame::new(m / 2.0, m / 2.0, w - m, h - m, Kind::Rect { fill: [0, 0, 0, 0], radius: 0.0 });
            border.stroke = Some(gold);
            border.stroke_w = 1.5;
            border.fx.corner = 4;
            border.fx.corner_r = 18.0;
            p.push(border);
            p.push(Frame::new(m, 110.0, w - 2.0 * m, 20.0, styled(txt("VOUS ÊTES INVITÉS", 10.0, true, gold, 1), "Segoe UI", false, 1.2, 4.0)));
            p.push(Frame::new(m, 150.0, w - 2.0 * m, 110.0, styled(txt("Camille\n&\nSacha", 36.0, false, dark, 1), "Georgia", true, 1.05, 0.0)));
            p.push(Frame::new(w / 2.0 - 30.0, 280.0, 60.0, 4.0, line(gold, 1.0)));
            p.push(Frame::new(m, 300.0, w - 2.0 * m, 80.0, txt("Samedi 21 juin\nà 16 heures\nDomaine des Tilleuls", 13.0, false, dark, 1)));
            p.push(Frame::new(m, h - 110.0, w - 2.0 * m, 30.0, styled(txt("Réponse souhaitée avant le 1er mai", 9.5, false, [120, 110, 100, 255], 1), "Georgia", true, 1.2, 0.0)));
            d
        }
        8 => {
            let mut d = Doc { width: 842.0, height: 595.0, margin: 50.0, ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let navy = [24, 40, 80, 255];
            let p = &mut d.pages[0].frames;
            let mut outer = Frame::new(24.0, 24.0, w - 48.0, h - 48.0, Kind::Rect { fill: [0, 0, 0, 0], radius: 0.0 });
            outer.stroke = Some(navy);
            outer.stroke_w = 3.0;
            p.push(outer);
            let mut inner = Frame::new(34.0, 34.0, w - 68.0, h - 68.0, Kind::Rect { fill: [0, 0, 0, 0], radius: 0.0 });
            inner.stroke = Some(gold);
            inner.stroke_w = 1.0;
            inner.fx.corner = 3;
            inner.fx.corner_r = 14.0;
            p.push(inner);
            p.push(Frame::new(m, 90.0, w - 2.0 * m, 50.0, styled(txt("CERTIFICAT", 40.0, true, navy, 1), "Georgia", false, 1.0, 8.0)));
            p.push(Frame::new(m, 145.0, w - 2.0 * m, 20.0, styled(txt("DE RÉUSSITE", 12.0, true, gold, 1), "Segoe UI", false, 1.0, 5.0)));
            p.push(Frame::new(m, 200.0, w - 2.0 * m, 20.0, txt("Ce certificat est décerné à", 12.0, false, dark, 1)));
            p.push(Frame::new(m, 230.0, w - 2.0 * m, 50.0, styled(txt("Prénom Nom", 34.0, false, navy, 1), "Georgia", true, 1.0, 0.0)));
            p.push(Frame::new(w / 2.0 - 120.0, 290.0, 240.0, 4.0, line(gold, 1.0)));
            p.push(Frame::new(m + 80.0, 310.0, w - 2.0 * m - 160.0, 50.0, txt("pour avoir suivi avec succès la formation « Mise en page professionnelle »", 12.0, false, dark, 1)));
            let mut seal = Frame::new(w / 2.0 - 45.0, h - 170.0, 90.0, 90.0, Kind::Polygon { sides: 18, inner: 0.86, fill: gold });
            seal.shadow = true;
            p.push(seal);
            for (k, t) in ["Date", "Signature"].iter().enumerate() {
                let x = if k == 0 { m + 40.0 } else { w - m - 200.0 };
                p.push(Frame::new(x, h - 110.0, 160.0, 4.0, line(dark, 0.75)));
                p.push(Frame::new(x, h - 100.0, 160.0, 16.0, txt(t, 10.0, false, [110, 110, 120, 255], 1)));
            }
            d
        }
        9 => {
            let mut d = Doc { width: 842.0, height: 595.0, margin: 24.0, bleed: 9.0, ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let teal = [0, 128, 128, 255];
            let pw = w / 3.0;
            d.guides = vec![crate::model::Guide { vertical: true, pos: pw }, crate::model::Guide { vertical: true, pos: 2.0 * pw }];
            let p = &mut d.pages[0].frames;
            for (k, t) in ["Nos services", "Qui sommes-nous ?"].iter().enumerate() {
                let x = k as f32 * pw + m;
                let mut head = Frame::new(x, m + 10.0, pw - 2.0 * m, 24.0, txt(t, 16.0, true, teal, 0));
                if let Kind::Text { opts, .. } = &mut head.kind {
                    opts.rule_below = true;
                    opts.rule_color = teal;
                }
                p.push(head);
                let mut b = Frame::new(x, m + 50.0, pw - 2.0 * m, h - 2.0 * m - 60.0, txt(&format!("{LOREM}\n{LOREM}"), 9.5, false, dark, 3));
                if let Kind::Text { para_space, .. } = &mut b.kind {
                    *para_space = 6.0;
                }
                p.push(b);
            }
            let x = 2.0 * pw;
            p.push(Frame::new(x, -9.0, pw + 9.0, h + 18.0, Kind::Rect { fill: teal, radius: 0.0 }));
            p.push(Frame::new(x + m, 120.0, pw - 2.0 * m, 120.0, Kind::image("", 6.0)));
            p.push(Frame::new(x + m, 270.0, pw - 2.0 * m, 90.0, styled(txt("Atelier\nVert", 34.0, true, white, 0), "Bahnschrift", false, 1.0, 1.0)));
            p.push(Frame::new(x + m, h - 80.0, pw - 2.0 * m, 40.0, txt("atelier-vert.fr\n01 23 45 67 89", 10.0, false, white, 0)));
            d
        }
        10 => {
            let mut d = Doc { width: 432.0, height: 648.0, margin: 36.0, bleed: 9.0, ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(-9.0, -9.0, w + 18.0, h + 18.0, Kind::image("", 0.0)));
            let mut veil = Frame::new(-9.0, h * 0.45, w + 18.0, h * 0.55 + 9.0, Kind::Rect { fill: [10, 10, 16, 255], radius: 0.0 });
            veil.fx.gfeather = true;
            veil.fx.gf_angle = -90.0;
            veil.fx.gf_start = 0.0;
            veil.fx.gf_end = 0.6;
            p.push(veil);
            p.push(Frame::new(m, h * 0.62, w - 2.0 * m, 120.0, styled(txt("Le silence\ndes villes", 40.0, true, white, 0), "Georgia", false, 1.0, 0.0)));
            p.push(Frame::new(m, h * 0.62 + 130.0, w - 2.0 * m, 20.0, styled(txt("ROMAN", 10.0, true, [255, 196, 120, 255], 0), "Segoe UI", false, 1.0, 5.0)));
            p.push(Frame::new(m, h - 60.0, w - 2.0 * m, 20.0, txt("Prénom Nom", 14.0, false, white, 0)));
            d
        }
        11 => {
            let mut d = Doc { margin: 40.0, ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let p = &mut d.pages[0].frames;
            p.push(Frame::new(m, m, w - 2.0 * m, 300.0, Kind::image("", 8.0)));
            let mut tag = Frame::new(w - m - 110.0, m + 16.0, 94.0, 94.0, Kind::Polygon { sides: 14, inner: 0.85, fill: [230, 40, 60, 255] });
            tag.rotation = 12.0;
            p.push(tag);
            p.push(Frame::new(w - m - 110.0, m + 46.0, 94.0, 34.0, txt("−20 %", 18.0, true, white, 1)));
            p.push(Frame::new(m, 360.0, w - 2.0 * m, 36.0, txt("Casque sans fil Aria", 26.0, true, dark, 0)));
            p.push(Frame::new(m, 400.0, w - 2.0 * m, 20.0, txt("Réduction de bruit active · 40 h d'autonomie", 12.0, false, [100, 100, 110, 255], 0)));
            p.push(Frame::new(m, 440.0, (w - 2.0 * m) * 0.5, 160.0, txt("Son haute fidélité\nRecharge rapide USB-C\nPliable, étui fourni\nMicro pour les appels", 11.0, false, dark, 0)));
            if let Kind::Text { list, para_space, opts, .. } = &mut p.last_mut().unwrap().kind {
                *list = 1;
                *para_space = 4.0;
                opts.bullet = "✓".into();
            }
            let cells = [["Caractéristique", "Valeur"], ["Poids", "250 g"], ["Autonomie", "40 h"], ["Bluetooth", "5.3"], ["Garantie", "2 ans"]].iter().map(|r| r.iter().map(|c| c.to_string()).collect()).collect();
            p.push(Frame::new(w / 2.0 + 10.0, 440.0, w / 2.0 - m - 10.0, 140.0, table(cells, 10.0, "Segoe UI", dark, [210, 210, 218, 255], Some([30, 30, 40, 255]), Some([245, 245, 248, 255]))));
            let mut price = Frame::new(m, h - 150.0, w - 2.0 * m, 60.0, txt("199 €", 34.0, true, [230, 40, 60, 255], 2));
            if let Kind::Text { valign, .. } = &mut price.kind {
                *valign = 1;
            }
            p.push(price);
            d
        }
        12 => {
            let mut d = Doc { width: 298.0, height: 420.0, margin: 22.0, bleed: 6.0, paper: [16, 40, 32, 255], ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let p = &mut d.pages[0].frames;
            for (k, (x, y, s)) in [(0.2, 0.18, 26.0), (0.72, 0.12, 18.0), (0.55, 0.3, 14.0), (0.1, 0.42, 12.0), (0.84, 0.4, 22.0)].iter().enumerate() {
                let mut star = Frame::new(w * x, h * y, *s, *s, Kind::Polygon { sides: 5, inner: 0.45, fill: [255, 214, 10, 255] });
                star.fx.glow = true;
                star.rotation = k as f32 * 9.0;
                p.push(star);
            }
            p.push(Frame::new(m, h * 0.52, w - 2.0 * m, 90.0, styled(txt("Joyeuses\nfêtes", 38.0, false, white, 1), "Georgia", true, 1.0, 0.0)));
            p.push(Frame::new(m, h * 0.52 + 100.0, w - 2.0 * m, 20.0, styled(txt("ET BELLE ANNÉE", 9.0, true, [255, 214, 10, 255], 1), "Segoe UI", false, 1.0, 4.0)));
            d
        }
        _ => {
            let mut d = Doc { margin: 50.0, title: "Rapport annuel".into(), ..Doc::a4() };
            let (w, h, m) = (d.width, d.height, d.margin);
            let indigo = [70, 60, 200, 255];
            let p = &mut d.pages[0].frames;
            let mut band = Frame::new(-9.0, -9.0, w + 18.0, h * 0.62, Kind::Rect { fill: indigo, radius: 0.0 });
            band.fill2 = Some([140, 80, 255, 255]);
            band.grad_angle = 45.0;
            p.push(band);
            p.push(Frame::new(m, 180.0, w - 2.0 * m, 120.0, styled(txt("Rapport\nannuel 2026", 44.0, true, white, 0), "Bahnschrift", false, 1.0, 0.0)));
            p.push(Frame::new(m, h * 0.62 + 30.0, w - 2.0 * m, 60.0, txt("Résultats, projets et perspectives de l'année écoulée.", 14.0, false, dark, 0)));
            d.pages[0].no_master = true;
            d.bleed = 9.0;
            let mut p2 = Page::default();
            let mut t = Frame::new(m, m, w - 2.0 * m, 40.0, txt("Introduction", 24.0, true, indigo, 0));
            if let Kind::Text { opts, .. } = &mut t.kind {
                opts.style = "Titre 1".into();
            }
            p2.frames.push(t);
            let mut body = Frame::new(m, m + 60.0, w - 2.0 * m, 600.0, txt(&format!("{LOREM}\n{LOREM}\n{LOREM}"), 10.5, false, dark, 3));
            if let Kind::Text { columns, para_space, opts, .. } = &mut body.kind {
                *columns = 2;
                *para_space = 6.0;
                opts.drop_lines = 2;
            }
            p2.frames.push(body);
            d.pages.push(p2);
            d.master.frames.push(Frame::new(m, h - 34.0, w - 2.0 * m, 12.0, txt("{title} · {page}", 7.5, false, [130, 130, 140, 255], 2)));
            d
        }
    }
}
