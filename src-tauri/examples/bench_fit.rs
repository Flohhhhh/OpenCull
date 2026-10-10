//! The loupe's fit rendering, made two ways, timed end to end.
//!
//!     cargo run --release --example bench_fit -- /path/to/photo.ARW [more...]
//!
//! One is what the app ships: a pure-Rust decoder and encoder. The other is
//! libjpeg-turbo, which is hand-tuned for each processor. "On one core" is
//! the figure that decides how fast a folder is prepared in the background,
//! where many are made at once and each gets a core to itself.

// Only means anything in a build that has libjpeg-turbo to compare against.
#[cfg(not(any(feature = "turbo", feature = "turbo-system")))]
fn main() {
    eprintln!("Build with `--features turbo` or `--features turbo-system` to run this comparison.");
}

#[cfg(any(feature = "turbo", feature = "turbo-system"))]
fn main() {
    compare::main()
}

#[cfg(any(feature = "turbo", feature = "turbo-system"))]
mod compare {
    use std::path::Path;
    use std::time::Instant;

    use opencull_lib::raw::{self, io::ReadAt};

    fn best(mut f: impl FnMut() -> usize) -> (f64, usize) {
        let mut size = 0;
        let ms = (0..5)
            .map(|_| {
                let t = Instant::now();
                size = f();
                t.elapsed().as_secs_f64() * 1000.0
            })
            .fold(f64::MAX, f64::min);
        (ms, size)
    }

    fn pure_rust(bytes: &[u8], target: u16) -> usize {
        let mut d = jpeg_decoder::Decoder::new(bytes);
        d.read_info().unwrap();
        let info = d.info().unwrap();
        let k = target as f64 / info.width.max(info.height) as f64;
        let (w, h) = d.scale((info.width as f64 * k).ceil() as u16, (info.height as f64 * k).ceil() as u16).unwrap();
        let px = d.decode().unwrap();
        let mut out = Vec::new();
        let mut enc = jpeg_encoder::Encoder::new(&mut out, 88);
        enc.set_sampling_factor(jpeg_encoder::SamplingFactor::F_2_2);
        enc.encode(&px, w, h, jpeg_encoder::ColorType::Rgb).unwrap();
        out.len()
    }

    /// `fine` allows libjpeg-turbo's in-between sizes — three eighths, five
    /// eighths — which the pure-Rust decoder does not have. Without it the two
    /// produce the same picture, which is the like-for-like comparison.
    fn turbo(bytes: &[u8], target: usize, fine: bool) -> usize {
        let mut d = turbojpeg::Decompressor::new().unwrap();
        let header = d.read_header(bytes).unwrap();
        // The smallest of its fractions that still reaches the target.
        let long = header.width.max(header.height);
        let halvings: &[(usize, usize)] = &[(1, 8), (1, 4), (1, 2), (1, 1)];
        let all: &[(usize, usize)] = &[(1, 8), (1, 4), (3, 8), (1, 2), (5, 8), (3, 4), (7, 8), (1, 1)];
        let factor = (if fine { all } else { halvings })
            .iter()
            .copied()
            .map(|(n, d)| turbojpeg::ScalingFactor::new(n, d))
            .find(|f| f.scale(long) >= target)
            .unwrap_or(turbojpeg::ScalingFactor::ONE);
        d.set_scaling_factor(factor).unwrap();
        let scaled = header.scaled(factor);
        let mut image = turbojpeg::Image {
            pixels: vec![0u8; scaled.width * scaled.height * 3],
            width: scaled.width,
            pitch: scaled.width * 3,
            height: scaled.height,
            format: turbojpeg::PixelFormat::RGB,
        };
        d.decompress(bytes, image.as_deref_mut()).unwrap();

        let mut c = turbojpeg::Compressor::new().unwrap();
        c.set_quality(88).unwrap();
        c.set_subsamp(turbojpeg::Subsamp::Sub2x2).unwrap();
        c.compress_to_vec(image.as_deref()).unwrap().len()
    }

    pub fn main() {
        for arg in std::env::args().skip(1) {
            let path = Path::new(&arg);
            let src = raw::io::FileSource::open(path).unwrap();
            let probe = raw::probe_source(&src);
            let Some(e) = probe.largest() else { continue };
            let bytes = src.read_at(e.offset, e.len as usize).unwrap();
            println!(
                "\n{}  full-size preview {}x{} ({} KB)",
                path.extension().unwrap().to_string_lossy().to_uppercase(),
                e.width,
                e.height,
                e.len / 1024
            );

            let target = 2040; // what the app asks for: 2400, less its tolerance
            let (ms, size) = best(|| pure_rust(&bytes, target as u16));
            println!("  pure Rust, all cores    {ms:6.1} ms   ({} KB out)", size / 1024);

            // The same, confined to one thread, as it is when a folder is being
            // prepared and every core is busy with a photo of its own.
            let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
            let (ms, _) = one.install(|| best(|| pure_rust(&bytes, target as u16)));
            println!("  pure Rust, one core     {ms:6.1} ms");

            let (ms, size) = best(|| turbo(&bytes, target, false));
            println!("  libjpeg-turbo, one core {ms:6.1} ms   ({} KB out, same picture)", size / 1024);
            let (ms, size) = best(|| turbo(&bytes, target, true));
            println!("  ...with a closer size   {ms:6.1} ms   ({} KB out)", size / 1024);
        }
    }
}
