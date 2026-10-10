//! Times the screen-sized picture of each photo: lifted from the file where
//! it holds one, made where it does not.
//!
//!     cargo run --release --features turbo-system --example screen -- <folder> [how many]
//!
//! Photographs are only read.

use std::time::Instant;

use opencull_lib::preview::{self, Tier};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let folder = args.first().expect("usage: screen <folder> [how many]");
    let limit: usize = args.get(1).and_then(|n| n.parse().ok()).unwrap_or(40);
    let mut files: Vec<_> = std::fs::read_dir(folder).expect("read the folder").flatten().map(|e| e.path()).filter(|p| !p.file_name().unwrap().to_string_lossy().starts_with('.')).collect();
    files.sort();
    let (mut lifted, mut made, mut sizes) = (Vec::new(), Vec::new(), Vec::new());
    for path in files.iter().filter(|p| opencull_lib::raw::is_jpeg(p) || opencull_lib::raw::is_raw(p)).take(limit) {
        let t = Instant::now();
        let stored = preview::lift_screen(path).ok().flatten().is_some();
        let Ok(r) = preview::render(path, Tier::Screen) else { continue };
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if stored { lifted.push(ms) } else { made.push(ms) }
        if let Some((w, h)) = preview::dimensions(&r.bytes) {
            sizes.push((w, h, r.bytes.len() / 1024));
        }
    }
    let median = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v.get(v.len() / 2).copied().unwrap_or(0.0)
    };
    println!("{} lifted from the file (median {:.1} ms), {} made (median {:.1} ms)", lifted.len(), median(&mut lifted), made.len(), median(&mut made));
    if let Some((w, h, kb)) = sizes.first() {
        println!("the first is {w}x{h}, {kb} KB");
    }
}
