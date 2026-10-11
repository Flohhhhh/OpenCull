//! Writes a rating into a JPEG, the way the app does when a JPEG is rated.
//!
//!     cargo run --release --example embed -- <a COPY of a jpeg> <rating 0-5>
//!
//! THIS WRITES INTO THE FILE IT IS GIVEN. Point it at a copy.

use std::path::Path;

use opencull_lib::embed;
use opencull_lib::xmp::Marks;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [file, rating] = args.as_slice() else {
        eprintln!("usage: embed <a COPY of a jpeg> <rating 0-5>");
        std::process::exit(2);
    };
    let marks = Marks { rating: rating.parse().expect("a rating from 0 to 5"), ..Default::default() };
    let started = std::time::Instant::now();
    match embed::write(Path::new(file), marks) {
        Ok(done) => println!("{done:?} in {:.1} ms", started.elapsed().as_secs_f64() * 1000.0),
        Err(e) => {
            eprintln!("left alone: {e}");
            std::process::exit(1);
        }
    }
}
