//! Times ingest against a real card: listing it, reading its capture times,
//! and copying a run of photos off it one at a time and four at a time.
//!
//!     cargo run --release --example ingest -- /Volumes/CARD/DCIM /tmp/somewhere
//!
//! The card is only ever read. What is copied goes into a folder made inside
//! the second path, which is removed again before this exits.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use opencull_lib::ingest::{self, Item, Policy};

/// How many photos each timed copy takes.
const RUN: usize = 40;

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(source), Some(scratch)) = (args.next(), args.next()) else {
        eprintln!("usage: ingest <folder on a card> <a temporary folder>");
        std::process::exit(2);
    };
    let dest = PathBuf::from(scratch).join(format!("opencull-ingest-bench-{}", std::process::id()));

    let t = Instant::now();
    let photos = ingest::scan(Path::new(&source));
    println!("listed {} photos in {:.0} ms", photos.len(), t.elapsed().as_secs_f64() * 1000.0);

    let t = Instant::now();
    let sample: Vec<_> = photos.iter().take(500).collect();
    let timed = sample.iter().filter(|p| ingest::details(&p.path).0.is_some()).count();
    println!(
        "capture time and stars read for {} photos in {:.0} ms ({:.2} ms each, one at a time); {timed} had a time",
        sample.len(),
        t.elapsed().as_secs_f64() * 1000.0,
        t.elapsed().as_secs_f64() * 1000.0 / sample.len().max(1) as f64
    );

    // Each timed copy takes photos nothing has read yet. A second pass over
    // the same ones would be served from memory and measure nothing.
    let from_end: Vec<Item> = photos.iter().rev().map(|p| Item { path: p.path.clone(), display: p.display.clone() }).collect();
    for (round, workers) in [1usize, 4, 8].into_iter().enumerate() {
        let items: Vec<Item> = from_end.iter().skip(round * RUN).take(RUN).cloned().collect();
        if items.len() < RUN {
            println!("too few photos left for a run of {workers}");
            break;
        }
        let into = dest.join(format!("x{workers}"));
        std::fs::create_dir_all(&into).expect("make the temporary folder");
        let bytes = std::sync::atomic::AtomicU64::new(0);
        let t = Instant::now();
        let outcome = ingest::copy(&items, &into, Policy::UniqueName, workers, &AtomicBool::new(false), &|p| {
            bytes.store(p.bytes_total, std::sync::atomic::Ordering::Relaxed);
        });
        let seconds = t.elapsed().as_secs_f64();
        let mb = bytes.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e6;
        println!("{workers} at a time: {RUN} photos, {mb:.0} MB in {seconds:.2} s = {:.0} MB/s  ({outcome:?})", mb / seconds);
    }
    println!("solid state destination, so the app would copy {} at a time", ingest::parallel_copies(&dest));

    std::fs::remove_dir_all(&dest).ok();
    println!("temporary copies removed: {}", !dest.exists());
}
