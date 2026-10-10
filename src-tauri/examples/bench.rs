//! Which decoder, and at what size? Measured rather than assumed.
//!
//!     cargo run --release --example bench -- /path/to/photo.ARW [more...]

use std::path::Path;
use std::time::Instant;

use opencull_lib::raw::{self, io::ReadAt};
use fast_image_resize as fr;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn best(mut f: impl FnMut() -> f64) -> f64 {
    (0..3).map(|_| f()).fold(f64::MAX, f64::min)
}

fn main() {
    for arg in std::env::args().skip(1) {
        let path = Path::new(&arg);
        let src = raw::io::FileSource::open(path).unwrap();
        let probe = raw::probe_source(&src);
        println!("\n{}", path.extension().unwrap().to_string_lossy().to_uppercase());

        for e in &probe.embedded {
            if e.long_edge() < 1000 {
                continue;
            }
            let bytes = src.read_at(e.offset, e.len as usize).unwrap();
            println!("  source {}x{} ({} KB)", e.width, e.height, e.len / 1024);

            // jpeg-decoder, letting it shrink while decoding.
            for target in [320u16, 640, 2400] {
                if target as u32 > e.long_edge() {
                    continue;
                }
                let mut dims = (0, 0);
                let t = best(|| {
                    let t = Instant::now();
                    let mut d = jpeg_decoder::Decoder::new(&bytes[..]);
                    d.read_info().unwrap();
                    let (w, h) = (e.width as f64, e.height as f64);
                    let k = target as f64 / w.max(h);
                    let (sw, sh) = d.scale((w * k).ceil() as u16, (h * k).ceil() as u16).unwrap();
                    let px = d.decode().unwrap();
                    dims = (sw, sh);
                    std::hint::black_box(px);
                    ms(t)
                });
                println!("    jpeg-decoder scaled for {target:>4}  -> {:>5}x{:<5} {t:7.1} ms", dims.0, dims.1);
            }

            // zune-jpeg, full size.
            let mut full: Vec<u8> = Vec::new();
            let t = best(|| {
                let t = Instant::now();
                let mut d = zune_jpeg::JpegDecoder::new(zune_jpeg::zune_core::bytestream::ZCursor::new(&bytes[..]));
                full = d.decode().unwrap();
                ms(t)
            });
            println!("    zune-jpeg full                              {t:7.1} ms");

            // Resizing the full frame down, and encoding the result.
            for target in [640u32, 2400] {
                if target > e.long_edge() || full.len() != (e.width * e.height * 3) as usize {
                    continue;
                }
                let k = target as f64 / e.long_edge() as f64;
                let (dw, dh) = ((e.width as f64 * k).round() as u32, (e.height as f64 * k).round() as u32);
                let mut out = Vec::new();
                let t = best(|| {
                    let t = Instant::now();
                    let src = fr::images::ImageRef::new(e.width, e.height, &full, fr::PixelType::U8x3).unwrap();
                    let mut dst = fr::images::Image::new(dw, dh, fr::PixelType::U8x3);
                    let mut r = fr::Resizer::new();
                    r.resize(
                        &src,
                        &mut dst,
                        &fr::ResizeOptions::new().resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::Hamming)),
                    )
                    .unwrap();
                    out = dst.into_vec();
                    ms(t)
                });
                let mut size = 0;
                let te = best(|| {
                    let t = Instant::now();
                    let mut buf = Vec::new();
                    let enc = jpeg_encoder::Encoder::new(&mut buf, 85);
                    enc.encode(&out, dw as u16, dh as u16, jpeg_encoder::ColorType::Rgb).unwrap();
                    size = buf.len();
                    ms(t)
                });
                println!("    resize full -> {target:>4}  {t:6.1} ms    encode {te:6.1} ms ({} KB)", size / 1024);
            }
        }
    }
}
