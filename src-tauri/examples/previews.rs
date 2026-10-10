//! Writes small upright copies of photos, for trying face finders on.
//!
//!     cargo run --release --features turbo-system --example previews -- \
//!         <list.csv: name,path per line> <long edge in pixels> <folder to write into>
//!
//! Photographs are only read. Point the last argument outside any repository.

use std::path::{Path, PathBuf};

use opencull_lib::{faces, preview};
use rayon::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [list, edge, out] = args.as_slice() else {
        eprintln!("usage: previews <list.csv> <long edge> <output folder>");
        std::process::exit(2);
    };
    let edge: u32 = edge.parse().expect("the long edge, in pixels");
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).expect("make the output folder");

    let text = std::fs::read_to_string(list).expect("read the list");
    let jobs: Vec<(&str, &str)> = text.lines().filter_map(|l| l.split_once(',')).collect();
    let failed: usize = jobs
        .par_iter()
        .map(|(name, path)| {
            let target = out.join(format!("{name}.jpg"));
            if target.exists() {
                return 0;
            }
            let made = preview::decode_upright(Path::new(path), edge).and_then(|p| {
                let k = (edge as f64 / p.width.max(p.height) as f64).min(1.0);
                let (w, h) = (((p.width as f64 * k).round() as u32).max(1), ((p.height as f64 * k).round() as u32).max(1));
                let rgb = faces::cut(&p.rgb, p.width, p.height, (0, 0, p.width, p.height), w, h)?;
                let bytes = preview::encode_jpeg(rgb, w, h, 90)?;
                std::fs::write(&target, bytes).map_err(|e| e.to_string())
            });
            usize::from(made.is_err())
        })
        .sum();
    println!("{} previews asked for, {failed} could not be made", jobs.len());
}
