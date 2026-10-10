//! Points the preview finder at real files and says what it found and how
//! long that took.
//!
//!     cargo run --release --example probe -- /path/to/photo.ARW [more...]
//!
//! Each file is probed twice. The first run pays for the disk; the second is
//! the cost of the parsing alone, which is the number that says whether the
//! approach is sound.

use std::path::Path;
use std::time::Instant;

use opencull_lib::raw;

fn main() {
    for arg in std::env::args().skip(1) {
        let path = Path::new(&arg);
        let cold = Instant::now();
        let first = raw::probe(path);
        let cold = cold.elapsed();

        let warm = Instant::now();
        let probe = raw::probe(path);
        let warm = warm.elapsed();

        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or(&arg);
        match (first, probe) {
            (Ok(_), Ok(p)) => {
                let previews: Vec<String> = p
                    .embedded
                    .iter()
                    .map(|e| format!("{}x{} ({} KB)", e.width, e.height, e.len / 1024))
                    .collect();
                let e = &p.exif;
                println!(
                    "{name}\n  previews  {}\n  facing    {}   size {:?}x{:?}\n  camera    {} {}  |  {}\n  exposure  {:?}s  f/{:?}  ISO {:?}  {:?}mm\n  taken     {:?}  ({:?} ms)\n  rating    {:?}   xmp {}\n  time      {:.2} ms cold, {:.3} ms warm",
                    if previews.is_empty() { "NONE".into() } else { previews.join(", ") },
                    p.orientation,
                    e.width,
                    e.height,
                    e.make.as_deref().unwrap_or("?"),
                    e.model.as_deref().unwrap_or("?"),
                    e.lens.as_deref().unwrap_or("?"),
                    e.shutter,
                    e.aperture,
                    e.iso,
                    e.focal_length,
                    e.date,
                    e.capture_ms,
                    p.rating,
                    p.xmp.as_ref().map(|x| format!("{} bytes", x.len())).unwrap_or("none".into()),
                    cold.as_secs_f64() * 1000.0,
                    warm.as_secs_f64() * 1000.0,
                );
            }
            (Err(e), _) | (_, Err(e)) => println!("{name}\n  ERROR {e}"),
        }
    }
}
