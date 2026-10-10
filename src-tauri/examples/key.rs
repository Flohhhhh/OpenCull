//! Finds what each photo is about, as the app does for a photo with no
//! faces, and writes where — to be set beside the same thing worked out
//! another way, or drawn on the photos and looked at.
//!
//!     cargo run --release --features turbo-system --example key -- \
//!         <list.csv: name,path per line> <out.csv>
//!
//! Photographs are only read.

use std::io::Write;
use std::path::Path;

use opencull_lib::{ai, preview};
use rayon::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [list, out] = args.as_slice() else {
        eprintln!("usage: key <list.csv> <out.csv>");
        std::process::exit(2);
    };
    let text = std::fs::read_to_string(list).expect("read the list");
    let jobs: Vec<(&str, &str)> = text.lines().filter_map(|l| l.split_once(',')).collect();
    let lines: Vec<String> = jobs
        .par_iter()
        .map(|(name, path)| {
            let t = std::time::Instant::now();
            match preview::decode_upright(Path::new(path), u32::MAX).and_then(|p| ai::key::find(&p.rgb, p.width, p.height)) {
                Ok(k) => format!("{name},{:.4},{:.4},{:.4},{:.4},{},{}\n", k.x, k.y, k.w, k.h, k.focus_log.map_or(String::new(), |f| format!("{f:.3}")), t.elapsed().as_millis()),
                Err(e) => format!("{name},ERROR,{e}\n"),
            }
        })
        .collect();
    let mut file = std::fs::File::create(out).expect("create the output file");
    writeln!(file, "file,x,y,w,h,focusLog,ms").unwrap();
    for line in &lines {
        file.write_all(line.as_bytes()).unwrap();
    }
}
