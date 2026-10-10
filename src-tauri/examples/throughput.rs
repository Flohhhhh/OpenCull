//! Runs the real pipeline — scan, queue, decode, cache — over a whole folder
//! and reports how fast it went and what it could not open.
//!
//!     cargo run --release --example throughput -- /path/to/folder [limit]
//!
//! Uses a throwaway cache, so every preview is made from scratch.

use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use opencull_lib::cache::Cache;
use opencull_lib::pool::Priority;
use opencull_lib::preview::Tier;
use opencull_lib::scan;
use opencull_lib::serve::{Previews, Wanted};

fn run(previews: &Previews, images: &[scan::Scanned], tier: Tier, label: &str) {
    run_at(previews, images, tier, Priority::Soon, label)
}

fn run_at(previews: &Previews, images: &[scan::Scanned], tier: Tier, priority: Priority, label: &str) {
    let (tx, rx) = mpsc::channel();
    let started = Instant::now();
    for image in images {
        let tx = tx.clone();
        let name = image.name.clone();
        previews.get(
            Wanted { path: image.display.clone(), version: image.version.clone(), tier },
            priority,
            move |r| {
                tx.send((name, r.map(|s| s.bytes.len()))).ok();
            },
        );
    }
    let mut bytes = 0usize;
    let mut failed = Vec::new();
    for _ in images {
        match rx.recv().unwrap() {
            (_, Ok(n)) => bytes += n,
            (name, Err(e)) => failed.push(format!("{name}: {e}")),
        }
    }
    let secs = started.elapsed().as_secs_f64();
    let ok = images.len() - failed.len();
    println!(
        "{label:<22} {ok:>5} made in {secs:6.2} s   {:7.1} per second   {:6.2} ms each   avg {:4} KB   {} failed",
        ok as f64 / secs,
        secs * 1000.0 / ok.max(1) as f64,
        bytes / ok.max(1) / 1024,
        failed.len(),
    );
    for f in failed.iter().take(8) {
        println!("    {f}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let folder = Path::new(&args[1]);
    let limit: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);

    let t = Instant::now();
    let result = scan::scan_folder(folder, true);
    println!("scan: {} photos in {:.0} ms", result.images.len(), t.elapsed().as_secs_f64() * 1000.0);
    let images: Vec<_> = result.images.into_iter().take(limit).collect();

    let dir = std::env::temp_dir().join(format!("opencull-throughput-{}", std::process::id()));
    let previews = Previews::new(Cache::new(dir.clone()));

    run(&previews, &images, Tier::Thumb(320), "thumbnails, 320");
    run(&previews, &images, Tier::Thumb(320), "  again, from cache");
    let some: Vec<_> = images.iter().take(120).cloned().collect();
    run(&previews, &some, Tier::Fit(2400), "loupe previews");
    run(&previews, &some, Tier::Fit(2400), "  again, from cache");
    run(&previews, &some, Tier::Full, "full size, untouched");
    run(&previews, &some, Tier::Screen, "screen size, untouched");

    // Prepared ahead on spare time: fewer threads, by design, and a pause
    // before it starts.
    let ahead: Vec<_> = images.iter().skip(120).take(300).cloned().collect();
    if !ahead.is_empty() {
        run_at(&previews, &ahead, Tier::Fit(2400), Priority::Idle, "prepared while idle");
    }

    std::fs::remove_dir_all(&dir).ok();
}
