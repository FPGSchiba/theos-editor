//! PNG of one run: 2 px per raster cell.
use crate::analysis::Metrics;
use crate::graph::{Graph, Kind, Role};
use crate::space::Raster;
use std::{fs::File, io::BufWriter};


fn role_colour(r: Role) -> [u8; 3] {
    match r {
        Role::Start => [60, 200, 90],
        Role::Goal => [230, 190, 60],
        Role::Path => [90, 130, 200],
        Role::Leaf => [190, 150, 100],
        Role::LockedLeaf => [170, 90, 200],
        Role::Extraction => [60, 200, 210],
    }
}

pub fn write(path: &str, g: &Graph, ras: &Raster, m: &Metrics, cr: i32) -> Result<(), Box<dyn std::error::Error>> {
    let n = ras.n;
    let px: i32 = if ras.dense { (600 / n).clamp(2, 12) } else { 2 };
    let dim = n * px;
    let mut buf = vec![[18u8, 18, 24]; (dim * dim) as usize];
    let put = |buf: &mut Vec<[u8; 3]>, x: i32, y: i32, c: [u8; 3]| {
        if x < 0 || y < 0 || x >= n || y >= n { return; }
        for dy in 0..px { for dx in 0..px { buf[((y * px + dy) * dim + x * px + dx) as usize] = c; } }
    };
    for ly in 0..g.h { for lx in 0..g.w {
        if !g.enabled[(ly * g.w + lx) as usize] { continue; }
        for y in ly * cr..(ly + 1) * cr { for x in lx * cr..(lx + 1) * cr { put(&mut buf, x, y, if ras.dense { [48, 48, 62] } else { [30, 30, 40] }); } }
    } }
    for y in 0..n { for x in 0..n {
        let i = (y * n + x) as usize;
        if !ras.walkable[i] { continue; }
        let c = if ras.rooms[i] > 0 { role_colour(g.nodes[ras.rooms[i] as usize - 1].role) } else { [140, 140, 140] };
        put(&mut buf, x, y, c);
    } }
    for &(x, y) in &m.route { put(&mut buf, x, y, [255, 255, 255]); }
    for (i, nd) in g.nodes.iter().enumerate() {
        if nd.key.is_some() { let c = ras.rects[i].centre(); for dy in -1..=1 { for dx in -1..=1 { put(&mut buf, c.0 + dx, c.1 + dy, [255, 240, 0]); } } }
    }
    for (i, e) in g.edges.iter().enumerate() {
        let (x, y) = ras.marks[i];
        let (col, r) = match e.kind { Kind::Locked(_) => ([230, 30, 30], if ras.dense { 1 } else { 2 }), Kind::Drop => ([255, 140, 0], if ras.dense { 1 } else { 2 }), Kind::Open => continue };
        for dy in -r..=r { for dx in -r..=r { put(&mut buf, x + dx, y + dy, col); } }
        if e.kind == Kind::Drop { put(&mut buf, x, y, [20, 20, 20]); }
    }
    let mut enc = png::Encoder::new(BufWriter::new(File::create(path)?), dim as u32, dim as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let flat: Vec<u8> = buf.into_iter().flatten().collect();
    enc.write_header()?.write_image_data(&flat)?;
    Ok(())
}
