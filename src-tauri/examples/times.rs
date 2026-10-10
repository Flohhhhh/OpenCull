//! How long it takes to learn when every photo in a folder was taken, by the
//! short route and by the full probe — and whether the two agree.
//!
//!     cargo run --release --example times -- /path/to/folder [quick|full]
//!
//! Give `quick` or `full` to run only that one. A drive remembers what it has
//! just read, so the honest figure for either is from a folder not read yet.

use std::path::Path;
use std::time::Instant;

use opencull_lib::{raw, scan};
use rayon::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let only = args.get(2).map(String::as_str);
    let images = scan::scan_folder(Path::new(&args[1]), true).images;
    let pool = rayon::ThreadPoolBuilder::new().num_threads(8).build().unwrap();
    println!("{} photos", images.len());

    let mut quick = Vec::new();
    if only != Some("full") {
        let t = Instant::now();
        quick = pool.install(|| images.par_iter().map(|i| raw::capture_time(&i.path)).collect::<Vec<_>>());
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        println!(
            "quick: {ms:7.0} ms   {:.3} ms a photo   {} with a time",
            ms / images.len().max(1) as f64,
            quick.iter().flatten().count()
        );
    }
    if only != Some("quick") {
        let t = Instant::now();
        let full: Vec<_> = pool.install(|| {
            images.par_iter().map(|i| raw::probe(&i.path).ok().and_then(|p| p.exif.capture_ms)).collect()
        });
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        println!("full:  {ms:7.0} ms   {:.3} ms a photo", ms / images.len().max(1) as f64);
        if !quick.is_empty() {
            let differ = quick.iter().zip(&full).filter(|(a, b)| a != b).count();
            println!("the two disagree on {differ} photos");
        }
    }
}
