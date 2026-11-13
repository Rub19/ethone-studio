use crate::model::{self, Geom, Item, Node, P};
use resvg::usvg;

pub struct Grid {
    pub x0: f32,
    pub y0: f32,
    pub cell: f32,
    pub nx: usize,
    pub ny: usize,
}

impl Grid {
    pub fn around(min: P, max: P, margin: f32, res: usize) -> Grid {
        let (w, h) = (max[0] - min[0] + 2.0 * margin, max[1] - min[1] + 2.0 * margin);
        let cell = (w.max(h) / res as f32).max(1e-3);
        Grid { x0: min[0] - margin - cell, y0: min[1] - margin - cell, cell, nx: (w / cell).ceil() as usize + 2, ny: (h / cell).ceil() as usize + 2 }
    }
    fn pt(&self, i: f32, j: f32) -> P {
        [self.x0 + i * self.cell, self.y0 + j * self.cell]
    }
    fn idx(&self, i: usize, j: usize) -> usize {
        j * (self.nx + 1) + i
    }
}

pub enum Src {
    Poly { inside: Vec<bool>, rows: Vec<Vec<f32>>, cols: Vec<Vec<f32>> },
    Field(Vec<f32>),
}

fn crossings(loops: &[Vec<P>], v: f32, axis: usize) -> Vec<(f32, i32)> {
    let o = 1 - axis;
    let mut xs = Vec::new();
    for l in loops {
        let n = l.len();
        for k in 0..n {
            let (a, b) = (l[k], l[(k + 1) % n]);
            if (a[axis] <= v) != (b[axis] <= v) {
                let t = (v - a[axis]) / (b[axis] - a[axis]);
                xs.push((a[o] + (b[o] - a[o]) * t, if b[axis] > a[axis] { 1 } else { -1 }));
            }
        }
    }
    xs.sort_by(|a, b| a.0.total_cmp(&b.0));
    xs
}

impl Src {
    pub fn poly(g: &Grid, loops: &[Vec<P>]) -> Src {
        let mut inside = vec![false; (g.nx + 1) * (g.ny + 1)];
        let mut rows = Vec::with_capacity(g.ny + 1);
        for j in 0..=g.ny {
            let y = g.y0 + j as f32 * g.cell;
            let xs = crossings(loops, y, 1);
            let (mut w, mut k) = (0, 0);
            for i in 0..=g.nx {
                let x = g.x0 + i as f32 * g.cell;
                while k < xs.len() && xs[k].0 <= x {
                    w += xs[k].1;
                    k += 1;
                }
                inside[g.idx(i, j)] = w != 0;
            }
            rows.push(xs.into_iter().map(|c| c.0).collect());
        }
        let cols = (0..=g.nx).map(|i| crossings(loops, g.x0 + i as f32 * g.cell, 0).into_iter().map(|c| c.0).collect()).collect();
        Src::Poly { inside, rows, cols }
    }

    pub fn evenodd(g: &Grid, loops: &[Vec<P>]) -> Src {
        let mut s = Src::poly(g, loops);
        if let Src::Poly { inside, .. } = &mut s {
            for j in 0..=g.ny {
                let y = g.y0 + j as f32 * g.cell;
                for i in 0..=g.nx {
                    inside[g.idx(i, j)] = model::point_in(loops, [g.x0 + i as f32 * g.cell, y]);
                }
            }
        }
        s
    }

    fn inside(&self, k: usize) -> bool {
        match self {
            Src::Poly { inside, .. } => inside[k],
            Src::Field(f) => f[k] < 0.0,
        }
    }

    fn t(&self, g: &Grid, i: usize, j: usize, horiz: bool) -> f32 {
        match self {
            Src::Poly { rows, cols, .. } => {
                let (list, a) = if horiz { (&rows[j], g.x0 + i as f32 * g.cell) } else { (&cols[i], g.y0 + j as f32 * g.cell) };
                list.iter().find(|x| **x > a && **x <= a + g.cell).map_or(0.5, |x| (x - a) / g.cell)
            }
            Src::Field(f) => {
                let a = f[g.idx(i, j)];
                let b = if horiz { f[g.idx(i + 1, j)] } else { f[g.idx(i, j + 1)] };
                if (a - b).abs() < 1e-9 { 0.5 } else { (a / (a - b)).clamp(0.0, 1.0) }
            }
        }
    }
}

pub fn edt(g: &Grid, seed: &dyn Fn(usize) -> bool) -> Vec<f32> {
    let (w, h) = (g.nx + 1, g.ny + 1);
    let inf = 1e20f32;
    let mut d: Vec<f32> = (0..w * h).map(|k| if seed(k) { 0.0 } else { inf }).collect();
    fn pass(f: &[f32], out: &mut [f32]) {
        let n = f.len();
        let mut v = vec![0usize; n];
        let mut z = vec![0f32; n + 1];
        let mut k = 0;
        z[0] = f32::MIN;
        z[1] = f32::MAX;
        for q in 1..n {
            loop {
                let p = v[k];
                let s = ((f[q] + (q * q) as f32) - (f[p] + (p * p) as f32)) / (2.0 * q as f32 - 2.0 * p as f32);
                if s <= z[k] && k > 0 {
                    k -= 1;
                    continue;
                }
                if s <= z[k] {
                    v[0] = q;
                    z[1] = f32::MAX;
                    break;
                }
                k += 1;
                v[k] = q;
                z[k] = s;
                z[k + 1] = f32::MAX;
                break;
            }
        }
        k = 0;
        for (q, o) in out.iter_mut().enumerate() {
            while z[k + 1] < q as f32 {
                k += 1;
            }
            let p = v[k];
            *o = (q as f32 - p as f32).powi(2) + f[p];
        }
    }
    let mut col = vec![0f32; h];
    let mut out = vec![0f32; h];
    for i in 0..w {
        for j in 0..h {
            col[j] = d[j * w + i];
        }
        pass(&col, &mut out);
        for j in 0..h {
            d[j * w + i] = out[j];
        }
    }
    let mut row = vec![0f32; w];
    let mut out = vec![0f32; w];
    for j in 0..h {
        row.copy_from_slice(&d[j * w..(j + 1) * w]);
        pass(&row, &mut out);
        d[j * w..(j + 1) * w].copy_from_slice(&out);
    }
    d.into_iter().map(|v| v.sqrt() * g.cell).collect()
}

pub fn region(g: &Grid, srcs: &[Src], pred: &dyn Fn(u64) -> bool) -> Vec<Vec<P>> {
    let (w, h) = (g.nx + 1, g.ny + 1);
    let member = |k: usize| srcs.iter().enumerate().fold(0u64, |m, (b, s)| if s.inside(k) { m | 1 << b } else { m });
    let ins: Vec<bool> = (0..w * h).map(|k| pred(member(k))).collect();
    let mem: Vec<u64> = (0..w * h).map(member).collect();
    let eid = |i: usize, j: usize, horiz: bool| 2 * (j * w + i) + if horiz { 0 } else { 1 };
    let mut adj: Vec<[u32; 2]> = vec![[u32::MAX; 2]; 2 * w * h];
    let mut link = |a: usize, b: usize| {
        for (x, y) in [(a, b), (b, a)] {
            let s = &mut adj[x];
            if s[0] == u32::MAX { s[0] = y as u32 } else { s[1] = y as u32 }
        }
    };
    for j in 0..g.ny {
        for i in 0..g.nx {
            let (a, b, c, d) = (ins[g.idx(i, j)], ins[g.idx(i + 1, j)], ins[g.idx(i + 1, j + 1)], ins[g.idx(i, j + 1)]);
            let case = a as u8 | (b as u8) << 1 | (c as u8) << 2 | (d as u8) << 3;
            let (top, right, bottom, left) = (eid(i, j, true), eid(i + 1, j, false), eid(i, j + 1, true), eid(i, j, false));
            match case {
                0 | 15 => {}
                5 => {
                    link(left, top);
                    link(right, bottom);
                }
                10 => {
                    link(top, right);
                    link(bottom, left);
                }
                _ => {
                    let mut e = Vec::with_capacity(2);
                    if a != b { e.push(top) }
                    if b != c { e.push(right) }
                    if c != d { e.push(bottom) }
                    if d != a { e.push(left) }
                    if e.len() == 2 {
                        link(e[0], e[1]);
                    }
                }
            }
        }
    }
    let pos = |e: usize| -> P {
        let k = e / 2;
        let (i, j) = (k % w, k / w);
        let horiz = e % 2 == 0;
        let (k2, di, dj) = if horiz { (g.idx(i + 1, j), 1.0, 0.0) } else { (g.idx(i, j + 1), 0.0, 1.0) };
        let diff = mem[g.idx(i, j)] ^ mem[k2];
        let t = srcs.iter().enumerate().find(|(b, _)| diff >> b & 1 == 1).map_or(0.5, |(_, s)| s.t(g, i, j, horiz));
        g.pt(i as f32 + di * t, j as f32 + dj * t)
    };
    let mut seen = vec![false; adj.len()];
    let mut out = Vec::new();
    for start in 0..adj.len() {
        if seen[start] || adj[start][0] == u32::MAX {
            continue;
        }
        let mut lp = Vec::new();
        let (mut prev, mut cur) = (usize::MAX, start);
        loop {
            seen[cur] = true;
            lp.push(pos(cur));
            let [n0, n1] = adj[cur];
            let next = if n0 as usize != prev { n0 } else { n1 };
            if next == u32::MAX || next as usize == start {
                break;
            }
            prev = cur;
            cur = next as usize;
            if seen[cur] {
                break;
            }
        }
        if lp.len() >= 3 {
            out.push(lp);
        }
    }
    out
}

pub fn pieces(loops: Vec<Vec<P>>) -> Vec<Vec<Vec<P>>> {
    let depth: Vec<usize> = loops.iter().enumerate().map(|(i, l)| loops.iter().enumerate().filter(|(k, o)| *k != i && model::point_in(std::slice::from_ref(*o), l[0])).count()).collect();
    let mut out: Vec<(usize, Vec<Vec<P>>)> = Vec::new();
    for (i, l) in loops.iter().enumerate().filter(|(i, _)| depth[*i] % 2 == 0) {
        out.push((i, vec![l.clone()]));
    }
    for (i, l) in loops.iter().enumerate().filter(|(i, _)| depth[*i] % 2 == 1) {
        if let Some(o) = out.iter_mut().filter(|(oi, _)| depth[*oi] + 1 == depth[i] && model::point_in(std::slice::from_ref(&loops[*oi]), l[0])).next() {
            o.1.push(l.clone());
        }
    }
    out.into_iter().map(|(_, v)| v).collect()
}

pub fn to_item(loops: &[Vec<P>], tol: f32, base: &Item) -> Option<Item> {
    let mut paths: Vec<Vec<Node>> = loops
        .iter()
        .map(|l| model::simplify_loop(l, tol))
        .filter(|l| l.len() >= 3 && model::area(l).abs() > tol * tol * 4.0)
        .map(|l| model::fit(&l, true, 38.0))
        .collect();
    if paths.is_empty() {
        return None;
    }
    let first = paths.remove(0);
    Some(Item { geom: Geom::Path { nodes: first, closed: true }, sub: paths, mesh: Vec::new(), widths: Vec::new(), text_path: Vec::new(), ..base.clone() })
}

pub fn shape_loops(it: &Item) -> Vec<Vec<P>> {
    if it.stroke.is_some() && it.fill.is_none() && matches!(it.geom, Geom::Path { closed: false, .. }) {
        return outline_loops(it).unwrap_or_default();
    }
    it.loops()
}

fn grid_for(items: &[&Item], extra: f32) -> Option<Grid> {
    let b = items.iter().map(|i| i.bbox()).reduce(|a, b| a.union(&b))?;
    Some(Grid::around(b.min, b.max, extra + 2.0, 700))
}

pub const PATHFINDER: [&str; 10] = ["Unir", "Soustraire", "Intersection", "Exclure", "Diviser", "Découpe", "Fusion", "Cadrer", "Contour", "Détourage"];

pub fn pathfinder(items: &[Item], op: usize) -> Vec<Item> {
    let n = items.len().min(62);
    if n < 2 {
        return Vec::new();
    }
    let refs: Vec<&Item> = items[..n].iter().collect();
    let Some(g) = grid_for(&refs, 0.0) else { return Vec::new() };
    let srcs: Vec<Src> = refs.iter().map(|it| Src::poly(&g, &shape_loops(it))).collect();
    let tol = g.cell * 0.35;
    let full = (1u64 << n) - 1;
    let top = n - 1;
    let highest = |m: u64| 63 - m.leading_zeros() as usize;
    let mut out = Vec::new();
    let mut emit = |loops: Vec<Vec<P>>, base: &Item, split: bool| {
        if split {
            for p in pieces(loops) {
                out.extend(to_item(&p, tol, base));
            }
        } else {
            out.extend(to_item(&loops, tol, base));
        }
    };
    let combos = |pred: &dyn Fn(u64) -> bool| -> Vec<u64> {
        let mut set: Vec<u64> = Vec::new();
        for j in 0..=g.ny {
            for i in 0..=g.nx {
                let k = g.idx(i, j);
                let m = srcs.iter().enumerate().fold(0u64, |m, (b, s)| if s.inside(k) { m | 1 << b } else { m });
                if m != 0 && pred(m) && !set.contains(&m) {
                    set.push(m);
                }
            }
        }
        set
    };
    match op {
        0 => emit(region(&g, &srcs, &|m| m != 0), &items[top], false),
        1 => emit(region(&g, &srcs, &|m| m == 1), &items[0], false),
        2 => emit(region(&g, &srcs, &|m| m == full), &items[top], false),
        3 => emit(region(&g, &srcs, &|m| m.count_ones() % 2 == 1), &items[top], false),
        4 => {
            for m in combos(&|_| true) {
                emit(region(&g, &srcs, &|x| x == m), &items[highest(m)], true);
            }
        }
        5 => {
            for k in 0..n {
                emit(region(&g, &srcs, &|m| m != 0 && highest(m) == k), &items[k], true);
            }
        }
        6 => {
            let mut colors: Vec<Option<[u8; 4]>> = Vec::new();
            for k in 0..n {
                if !colors.contains(&items[k].fill) {
                    colors.push(items[k].fill);
                }
            }
            for c in colors {
                let base = items.iter().take(n).rev().find(|i| i.fill == c).unwrap();
                emit(region(&g, &srcs, &|m| m != 0 && items[highest(m)].fill == c), base, true);
            }
        }
        7 => {
            for k in 0..top {
                emit(region(&g, &srcs, &|m| m >> top & 1 == 1 && highest(m & !(1 << top)) == k && m & !(1 << top) != 0), &items[k], true);
            }
        }
        8 => {
            for m in combos(&|_| true) {
                let src = &items[highest(m)];
                let base = Item { fill: None, stroke: src.fill.or(src.stroke), stroke_w: 1.0, ..src.clone() };
                emit(region(&g, &srcs, &|x| x == m), &base, true);
            }
        }
        _ => emit(region(&g, &srcs, &|m| m == 1 << top), &items[top], false),
    }
    out
}

pub fn shape_build(items: &[Item], keys: &[u64], delete: bool, fill: Option<[u8; 4]>) -> Vec<Item> {
    let refs: Vec<&Item> = items.iter().collect();
    let Some(g) = grid_for(&refs, 0.0) else { return Vec::new() };
    let srcs: Vec<Src> = items.iter().map(|it| Src::poly(&g, &shape_loops(it))).collect();
    let tol = g.cell * 0.35;
    let highest = |m: u64| 63 - m.leading_zeros() as usize;
    let mut combos: Vec<u64> = Vec::new();
    for k in 0..(g.nx + 1) * (g.ny + 1) {
        let m = srcs.iter().enumerate().fold(0u64, |m, (b, s)| if s.inside(k) { m | 1 << b } else { m });
        if m != 0 && !combos.contains(&m) {
            combos.push(m);
        }
    }
    let mut out = Vec::new();
    for m in combos.iter().filter(|m| !keys.contains(m)) {
        for p in pieces(region(&g, &srcs, &|x| x == *m)) {
            out.extend(to_item(&p, tol, &items[highest(*m)]));
        }
    }
    if !delete && let Some(k) = keys.iter().copied().max_by_key(|k| highest(*k)) {
        let base = Item { fill: fill.or(items[highest(k)].fill), ..items[highest(k)].clone() };
        out.extend(to_item(&region(&g, &srcs, &|x| keys.contains(&x)), tol, &base));
    }
    out
}

pub fn membership(items: &[Item], p: P) -> u64 {
    items.iter().enumerate().fold(0, |m, (b, it)| if model::point_in(&shape_loops(it), p) { m | 1 << b } else { m })
}

pub fn build_region(items: &[Item], keys: &[u64]) -> Option<Item> {
    let refs: Vec<&Item> = items.iter().collect();
    let g = grid_for(&refs, 0.0)?;
    let srcs: Vec<Src> = items.iter().map(|it| Src::poly(&g, &shape_loops(it))).collect();
    let loops = region(&g, &srcs, &|m| keys.contains(&m));
    to_item(&loops, g.cell * 0.35, &Item::blank())
}

pub fn offset(it: &Item, d: f32) -> Option<Item> {
    let loops = shape_loops(it);
    let b = model::BBox::of(loops.iter().flatten().copied());
    let g = Grid::around(b.min, b.max, d.abs() + 2.0, 600);
    let src = Src::evenodd(&g, &loops);
    let Src::Poly { inside, .. } = &src else { return None };
    let din = edt(&g, &|k| !inside[k]);
    let dout = edt(&g, &|k| inside[k]);
    let field: Vec<f32> = (0..inside.len()).map(|k| if inside[k] { -(din[k] - g.cell * 0.5) } else { dout[k] - g.cell * 0.5 } - d).collect();
    let loops = region(&g, &[Src::Field(field)], &|m| m == 1);
    to_item(&loops, g.cell * 0.35, &Item { sub: Vec::new(), ..it.clone() })
}

fn stroke_field(lines: &[Vec<P>], g: &Grid, r: f32) -> Src {
    let w = g.nx + 1;
    let mut seed = vec![false; w * (g.ny + 1)];
    for l in lines {
        for s in l.windows(2) {
            let steps = (model::dist(s[0], s[1]) / (g.cell * 0.4)).ceil().max(1.0) as usize;
            for k in 0..=steps {
                let p = model::lerp(s[0], s[1], k as f32 / steps as f32);
                let (i, j) = (((p[0] - g.x0) / g.cell).round() as isize, ((p[1] - g.y0) / g.cell).round() as isize);
                if i >= 0 && j >= 0 && (i as usize) <= g.nx && (j as usize) <= g.ny {
                    seed[j as usize * w + i as usize] = true;
                }
            }
        }
    }
    let d = edt(g, &|k| seed[k]);
    Src::Field(d.into_iter().map(|v| v - r).collect())
}

pub fn outline_loops(it: &Item) -> Option<Vec<Vec<P>>> {
    let (nodes, closed) = it.path_nodes()?;
    let mut line = model::flatten(&nodes, closed, 24);
    if closed && !line.is_empty() {
        line.push(line[0]);
    }
    let mut lines = vec![line];
    for s in &it.sub {
        let mut l = model::flatten(s, true, 24);
        l.push(l[0]);
        lines.push(l);
    }
    let r = it.stroke_w.max(0.5) / 2.0;
    let b = model::BBox::of(lines.iter().flatten().copied());
    let g = Grid::around(b.min, b.max, r + 2.0, 700);
    Some(region(&g, &[stroke_field(&lines, &g, r)], &|m| m == 1))
}

pub fn outline_stroke(it: &Item) -> Option<Item> {
    if it.variable_stroke() {
        let loops = it.width_outline();
        let base = Item { fill: it.stroke, stroke: None, profile: 0, widths: Vec::new(), ..it.clone() };
        return to_item(&loops, 0.2, &base);
    }
    let loops = outline_loops(it)?;
    let b = model::BBox::of(loops.iter().flatten().copied());
    let tol = (b.size()[0].max(b.size()[1]) / 700.0 * 0.35).max(0.05);
    to_item(&loops, tol, &Item { fill: it.stroke, stroke: None, dash: 0.0, head_end: 0, head_start: 0, arrow_end: false, ..it.clone() })
}

pub fn brush_region(points: &[P], r: f32) -> Option<Vec<Vec<P>>> {
    if points.is_empty() {
        return None;
    }
    let b = model::BBox::of(points.iter().copied());
    let g = Grid::around(b.min, b.max, r + 2.0, 500);
    Some(region(&g, &[stroke_field(&[points.to_vec()], &g, r)], &|m| m == 1))
}

pub fn erase(it: &Item, path: &[P], r: f32) -> Option<Vec<Item>> {
    let loops = shape_loops(it);
    let b = model::BBox::of(loops.iter().flatten().copied());
    if !model::BBox::of(path.iter().copied()).overlaps(&model::BBox { min: [b.min[0] - r, b.min[1] - r], max: [b.max[0] + r, b.max[1] + r] }) {
        return None;
    }
    let g = Grid::around(b.min, b.max, 2.0, 700);
    let srcs = [Src::poly(&g, &loops), stroke_field(&[path.to_vec()], &g, r)];
    let res = region(&g, &srcs, &|m| m == 1);
    let out: Vec<Item> = pieces(res).iter().filter_map(|p| to_item(p, g.cell * 0.35, it)).collect();
    Some(out)
}

pub fn image_rgba(href: &str) -> Option<image::RgbaImage> {
    let data = href.split_once("base64,")?.1;
    image::load_from_memory(&model::unbase64(data)).ok().map(|i| i.to_rgba8())
}

pub struct TraceOpts {
    pub mode: usize,
    pub threshold: f32,
    pub colors: usize,
    pub noise: f32,
    pub precision: usize,
    pub corners: f32,
    pub ignore_white: bool,
}

pub fn trace(img: &image::RgbaImage, rect: [f32; 4], o: &TraceOpts) -> Vec<Item> {
    let (iw, ih) = img.dimensions();
    let s = (o.precision as f32 / iw.max(ih) as f32).min(1.0);
    let (w, h) = (((iw as f32 * s) as u32).max(2), ((ih as f32 * s) as u32).max(2));
    let small = image::imageops::resize(img, w, h, image::imageops::FilterType::Triangle);
    let g = Grid { x0: 0.0, y0: 0.0, cell: 1.0, nx: w as usize + 1, ny: h as usize + 1 };
    let (sx, sy) = (rect[2] / w as f32, rect[3] / h as f32);
    let px = |i: usize, j: usize| -> Option<[f32; 4]> {
        if i == 0 || j == 0 || i > w as usize || j > h as usize {
            return None;
        }
        let p = small.get_pixel(i as u32 - 1, j as u32 - 1).0;
        Some([p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32])
    };
    let lum = |c: [f32; 4]| (0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]) * c[3] / 255.0 + 255.0 * (1.0 - c[3] / 255.0);
    let n = (g.nx + 1) * (g.ny + 1);
    let mut layers: Vec<(Vec<f32>, [u8; 4])> = Vec::new();
    match o.mode {
        0 | 1 => {
            let levels = if o.mode == 0 { 1 } else { o.colors.clamp(2, 12) };
            for l in 0..levels {
                let thr = if o.mode == 0 { o.threshold } else { 255.0 * (l + 1) as f32 / (levels + 1) as f32 };
                let field: Vec<f32> = (0..n).map(|k| px(k % (g.nx + 1), k / (g.nx + 1)).map_or(1.0, |c| lum(c) - thr)).collect();
                let v = if o.mode == 0 { 0 } else { (thr * 0.8) as u8 };
                layers.push((field, [v, v, v, 255]));
            }
            layers.reverse();
        }
        _ => {
            let k = o.colors.clamp(2, 16);
            let mut samples: Vec<[f32; 3]> = small.pixels().filter(|p| p.0[3] > 127).map(|p| [p.0[0] as f32, p.0[1] as f32, p.0[2] as f32]).collect();
            if samples.is_empty() {
                return Vec::new();
            }
            let step = (samples.len() / 4000).max(1);
            samples = samples.into_iter().step_by(step).collect();
            let mut cent: Vec<[f32; 3]> = (0..k).map(|i| samples[i * samples.len() / k]).collect();
            let near = |c: &[f32; 3], cent: &[[f32; 3]]| (0..cent.len()).min_by(|a, b| d3(c, &cent[*a]).total_cmp(&d3(c, &cent[*b]))).unwrap();
            for _ in 0..10 {
                let mut acc = vec![[0f32; 4]; k];
                for s in &samples {
                    let q = near(s, &cent);
                    for t in 0..3 {
                        acc[q][t] += s[t];
                    }
                    acc[q][3] += 1.0;
                }
                for q in 0..k {
                    if acc[q][3] > 0.0 {
                        cent[q] = [acc[q][0] / acc[q][3], acc[q][1] / acc[q][3], acc[q][2] / acc[q][3]];
                    }
                }
            }
            let mut order: Vec<usize> = (0..k).collect();
            order.sort_by(|a, b| (cent[*b][0] + cent[*b][1] + cent[*b][2]).total_cmp(&(cent[*a][0] + cent[*a][1] + cent[*a][2])));
            let rank: Vec<usize> = (0..k).map(|q| order.iter().position(|x| *x == q).unwrap()).collect();
            let idx: Vec<Option<usize>> = (0..n).map(|kk| px(kk % (g.nx + 1), kk / (g.nx + 1)).filter(|c| c[3] > 127.0).map(|c| rank[near(&[c[0], c[1], c[2]], &cent)])).collect();
            for (r, q) in order.iter().enumerate() {
                let field: Vec<f32> = idx.iter().map(|v| if v.is_some_and(|v| v >= r) { -0.5 } else { 0.5 }).collect();
                let c = cent[*q];
                layers.push((field, [c[0] as u8, c[1] as u8, c[2] as u8, 255]));
            }
        }
    }
    let mut out = Vec::new();
    for (field, c) in layers {
        if o.ignore_white && c[0] > 240 && c[1] > 240 && c[2] > 240 {
            continue;
        }
        let loops: Vec<Vec<P>> = region(&g, &[Src::Field(field)], &|m| m == 1)
            .into_iter()
            .filter(|l| model::area(l).abs() >= o.noise)
            .map(|l| l.into_iter().map(|p| [rect[0] + (p[0] - 0.5) * sx, rect[1] + (p[1] - 0.5) * sy]).collect())
            .collect();
        let tol = sx.max(sy) * 0.6;
        let paths: Vec<Vec<Node>> = loops.iter().map(|l| model::simplify_loop(l, tol)).filter(|l| l.len() >= 3).map(|l| model::fit(&l, true, o.corners)).collect();
        let mut it = paths.into_iter();
        if let Some(first) = it.next() {
            out.push(Item { name: format!("Vectorisation #{:02x}{:02x}{:02x}", c[0], c[1], c[2]), geom: Geom::Path { nodes: first, closed: true }, sub: it.collect(), fill: Some(c), ..Item::blank() });
        }
    }
    out
}

fn d3(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

fn rgba(p: &usvg::Paint, op: f32) -> [u8; 4] {
    let a = (op * 255.0) as u8;
    match p {
        usvg::Paint::Color(c) => [c.red, c.green, c.blue, a],
        usvg::Paint::LinearGradient(g) => g.stops().first().map_or([128, 128, 128, a], |s| [s.color().red, s.color().green, s.color().blue, a]),
        usvg::Paint::RadialGradient(g) => g.stops().first().map_or([128, 128, 128, a], |s| [s.color().red, s.color().green, s.color().blue, a]),
        usvg::Paint::Pattern(_) => [128, 128, 128, a],
    }
}

fn convert_path(p: &usvg::Path, out: &mut Vec<Item>) {
    use usvg::tiny_skia_path::PathSegment;
    let tf = p.abs_transform();
    let map = |q: usvg::tiny_skia_path::Point| {
        let mut q = q;
        tf.map_point(&mut q);
        [q.x, q.y]
    };
    let mut subs: Vec<(Vec<Node>, bool)> = Vec::new();
    let mut cur: Vec<Node> = Vec::new();
    for seg in p.data().segments() {
        match seg {
            PathSegment::MoveTo(a) => {
                if cur.len() > 1 {
                    subs.push((std::mem::take(&mut cur), false));
                }
                cur = vec![Node::corner(map(a))];
            }
            PathSegment::LineTo(a) => cur.push(Node::corner(map(a))),
            PathSegment::QuadTo(c, a) => {
                let (c, a) = (map(c), map(a));
                if let Some(last) = cur.last_mut() {
                    last.cout = model::lerp(last.p, c, 2.0 / 3.0);
                }
                cur.push(Node { p: a, cin: model::lerp(a, c, 2.0 / 3.0), cout: a });
            }
            PathSegment::CubicTo(c1, c2, a) => {
                let (c1, c2, a) = (map(c1), map(c2), map(a));
                if let Some(last) = cur.last_mut() {
                    last.cout = c1;
                }
                cur.push(Node { p: a, cin: c2, cout: a });
            }
            PathSegment::Close => {
                if cur.len() > 1 && model::dist(cur[0].p, cur[cur.len() - 1].p) < 1e-3 {
                    let last = cur.pop().unwrap();
                    cur[0].cin = last.cin;
                }
                if cur.len() > 1 {
                    subs.push((std::mem::take(&mut cur), true));
                }
            }
        }
    }
    if cur.len() > 1 {
        subs.push((cur, false));
    }
    let fill = p.fill().map(|f| rgba(f.paint(), f.opacity().get()));
    let (stroke, sw) = p.stroke().map_or((None, 1.0), |s| (Some(rgba(s.paint(), s.opacity().get())), s.width().get()));
    let base = Item { name: p.id().to_string(), fill, stroke, stroke_w: sw, ..Item::blank() };
    let (closed, open): (Vec<_>, Vec<_>) = subs.into_iter().partition(|s| s.1);
    let mut closed = closed.into_iter().map(|s| s.0);
    if let Some(first) = closed.next() {
        out.push(Item { geom: Geom::Path { nodes: first, closed: true }, sub: closed.collect(), ..base.clone() });
    }
    for (nodes, _) in open {
        out.push(Item { geom: Geom::Path { nodes, closed: false }, ..base.clone() });
    }
}

fn walk(g: &usvg::Group, out: &mut Vec<Item>) {
    for n in g.children() {
        match n {
            usvg::Node::Group(g) => walk(g, out),
            usvg::Node::Path(p) => convert_path(p, out),
            usvg::Node::Text(t) => walk(t.flattened(), out),
            usvg::Node::Image(_) => {}
        }
    }
}

pub fn import_svg(data: &str, opt: &usvg::Options) -> Result<Vec<Item>, String> {
    let tree = usvg::Tree::from_str(data, opt).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    walk(tree.root(), &mut out);
    Ok(out)
}

pub fn text_outlines(it: &Item, opt: &usvg::Options) -> Option<Item> {
    let mut d = model::Doc::new(10.0, 10.0);
    d.background = None;
    let mut t = it.clone();
    t.shadow = false;
    t.opacity = 1.0;
    t.blend = 0;
    d.items.push(t);
    let svg = d.to_svg().replacen(r#"width="10" height="10" viewBox="0 0 10 10""#, r#"width="10" height="10""#, 1);
    let parts = import_svg(&svg, opt).ok()?;
    let mut loops: Vec<Vec<Node>> = parts.into_iter().flat_map(|p| match p.geom {
        Geom::Path { nodes, .. } => std::iter::once(nodes).chain(p.sub).collect::<Vec<_>>(),
        _ => Vec::new(),
    }).collect();
    if loops.is_empty() {
        return None;
    }
    let first = loops.remove(0);
    Some(Item { name: format!("{} (vectorisé)", it.name), geom: Geom::Path { nodes: first, closed: true }, sub: loops, text_path: Vec::new(), ..it.clone() })
}

pub fn scribble(it: &Item, angle: f32, spacing: f32, jitter: f32) -> Option<Item> {
    let loops = shape_loops(it);
    if loops.is_empty() {
        return None;
    }
    let c = model::BBox::of(loops.iter().flatten().copied()).center();
    let (s, co) = (-angle).to_radians().sin_cos();
    let rot = |p: P, s: f32, co: f32| [c[0] + (p[0] - c[0]) * co - (p[1] - c[1]) * s, c[1] + (p[0] - c[0]) * s + (p[1] - c[1]) * co];
    let rl: Vec<Vec<P>> = loops.iter().map(|l| l.iter().map(|p| rot(*p, s, co)).collect()).collect();
    let b = model::BBox::of(rl.iter().flatten().copied());
    let mut pts = Vec::new();
    let mut y = b.min[1] + spacing / 2.0;
    let mut flip = false;
    let mut seed = 12345u32;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f32 / u32::MAX as f32 - 0.5) * jitter
    };
    while y < b.max[1] {
        let xs = crossings(&rl, y, 1);
        if xs.len() >= 2 {
            let (a, z) = (xs[0].0, xs[xs.len() - 1].0);
            let (a, z) = if flip { (z, a) } else { (a, z) };
            pts.push([a + rnd(), y + rnd()]);
            pts.push([z + rnd(), y + spacing / 2.0 + rnd()]);
            flip = !flip;
        }
        y += spacing;
    }
    if pts.len() < 2 {
        return None;
    }
    let (s2, co2) = angle.to_radians().sin_cos();
    let mut nodes: Vec<Node> = pts.into_iter().map(|p| Node::corner(rot(p, s2, co2))).collect();
    model::smooth(&mut nodes, false);
    Some(Item { geom: Geom::Path { nodes, closed: false }, sub: Vec::new(), fill: None, stroke: it.fill.or(it.stroke), stroke_w: (spacing * 0.35).max(0.5), cap: 0, join: 0, ..it.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(x: f32, y: f32, s: f32) -> Item {
        Item { geom: Geom::Rect { x, y, w: s, h: s, r: 0.0 }, fill: Some([200, 0, 0, 255]), ..Item::blank() }
    }

    #[test]
    fn booleans_offset_trace() {
        let a = sq(0.0, 0.0, 100.0);
        let b = sq(50.0, 50.0, 100.0);
        let area = |it: &Item| it.loops().iter().map(|l| model::area(l).abs()).sum::<f32>();
        let u = pathfinder(&[a.clone(), b.clone()], 0);
        assert_eq!(u.len(), 1);
        assert!((area(&u[0]) - 17500.0).abs() < 400.0, "{}", area(&u[0]));
        let i = pathfinder(&[a.clone(), b.clone()], 2);
        assert!((area(&i[0]) - 2500.0).abs() < 150.0, "{}", area(&i[0]));
        let m = pathfinder(&[a.clone(), b.clone()], 1);
        assert!((area(&m[0]) - 7500.0).abs() < 300.0);
        assert_eq!(pathfinder(&[a.clone(), b.clone()], 4).len(), 3);
        let x = pathfinder(&[a.clone(), b.clone()], 3);
        assert!((area(&x[0]) - 15000.0).abs() < 600.0);
        let hole = pathfinder(&[a.clone(), sq(25.0, 25.0, 50.0)], 1);
        assert_eq!(hole[0].sub.len(), 1);
        let o = offset(&a, 10.0).unwrap();
        let bb = o.bbox();
        assert!((bb.min[0] + 10.0).abs() < 2.0 && (bb.max[0] - 110.0).abs() < 2.0, "{bb:?}");
        let line = Item { geom: Geom::Path { nodes: vec![Node::corner([0.0, 0.0]), Node::corner([100.0, 0.0])], closed: false }, stroke: Some([0, 0, 0, 255]), stroke_w: 10.0, ..Item::blank() };
        let ol = outline_stroke(&line).unwrap();
        assert!((ol.bbox().size()[1] - 10.0).abs() < 1.5);
        assert_eq!(membership(&[a.clone(), b.clone()], [75.0, 75.0]), 3);
        assert!(build_region(&[a.clone(), b.clone()], &[1]).is_some());
        let er = erase(&a, &[[50.0, -10.0], [50.0, 110.0]], 3.0).unwrap();
        assert_eq!(er.len(), 2);
        let mut img = image::RgbaImage::from_pixel(40, 40, image::Rgba([255, 255, 255, 255]));
        for y in 10..30 {
            for x in 10..30 {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
            }
        }
        let t = trace(&img, [0.0, 0.0, 40.0, 40.0], &TraceOpts { mode: 0, threshold: 128.0, colors: 4, noise: 2.0, precision: 200, corners: 40.0, ignore_white: true });
        assert_eq!(t.len(), 1);
        let tb = t[0].bbox();
        assert!((tb.min[0] - 10.0).abs() < 1.5 && (tb.max[0] - 30.0).abs() < 1.5, "{tb:?}");
        let tc = trace(&img, [0.0, 0.0, 40.0, 40.0], &TraceOpts { mode: 2, threshold: 128.0, colors: 2, noise: 2.0, precision: 200, corners: 40.0, ignore_white: false });
        assert_eq!(tc.len(), 2);
        let sc = scribble(&a, 45.0, 8.0, 1.0).unwrap();
        assert!(matches!(sc.geom, Geom::Path { closed: false, .. }));
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        let opt = usvg::Options { fontdb: std::sync::Arc::new(db), ..Default::default() };
        let imp = import_svg(r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><path d="M0 0 L10 0 L10 10 Z M2 2 L4 2 L4 4 Z" fill="red"/></svg>"#, &opt).unwrap();
        assert_eq!(imp.len(), 1);
        assert_eq!(imp[0].sub.len(), 1);
        assert_eq!(imp[0].fill, Some([255, 0, 0, 255]));
        let txt = Item { geom: Geom::Text { x: 0.0, y: 50.0, text: "Ab".into(), size: 40.0, family: "Arial".into(), bold: false }, fill: Some([0, 0, 0, 255]), ..Item::blank() };
        let to = text_outlines(&txt, &opt).unwrap();
        assert!(!to.sub.is_empty());
    }
}
