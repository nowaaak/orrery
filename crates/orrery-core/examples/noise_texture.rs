use std::io::Write;
use std::path::PathBuf;

use flate2::Compression;
use flate2::write::ZlibEncoder;

const SIZE: usize = 128;

fn mix_bits(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

fn hash(x: usize, y: usize, seed: u32) -> f32 {
    let value = mix_bits(x as u32 ^ mix_bits(y as u32 ^ mix_bits(seed)));
    value as f32 / u32::MAX as f32
}

fn periodic_noise(x: f32, y: f32, period: usize, seed: u32) -> f32 {
    let (cx, cy) = (x.floor(), y.floor());
    let (fx, fy) = (x - cx, y - cy);
    let (ux, uy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let corner = |dx: usize, dy: usize| {
        hash(
            (cx as usize + dx) % period,
            (cy as usize + dy) % period,
            seed,
        )
    };
    let top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * ux;
    let bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * ux;
    top + (bottom - top) * uy
}

fn fbm(x: usize, y: usize) -> f32 {
    let mut value = 0.0;
    let mut total = 0.0;
    for (period, weight) in [(8usize, 0.5f32), (16, 0.28), (32, 0.14), (64, 0.08)] {
        let scale = period as f32 / SIZE as f32;
        value += periodic_noise(x as f32 * scale, y as f32 * scale, period, period as u32) * weight;
        total += weight;
    }
    value / total
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut red = vec![0u8; SIZE * SIZE];
    for y in 0..SIZE {
        for x in 0..SIZE {
            red[y * SIZE + x] = (hash(x, y, 1) * 255.0).round() as u8;
        }
    }
    let smooth: Vec<f32> = (0..SIZE * SIZE).map(|i| fbm(i % SIZE, i / SIZE)).collect();
    let (low, high) = smooth
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));

    let mut raw = Vec::with_capacity(SIZE * (SIZE * 4 + 1));
    for y in 0..SIZE {
        raw.push(0);
        for x in 0..SIZE {
            let shifted = red[((y + 17) % SIZE) * SIZE + (x + 37) % SIZE];
            let blue = (smooth[y * SIZE + x] - low) / (high - low);
            raw.extend_from_slice(&[
                red[y * SIZE + x],
                shifted,
                (blue * 255.0).round() as u8,
                (hash(x, y, 2) * 255.0).round() as u8,
            ]);
        }
    }

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&raw)?;
    let compressed = encoder.finish()?;

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend_from_slice(&(SIZE as u32).to_be_bytes());
    header.extend_from_slice(&(SIZE as u32).to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &compressed);
    chunk(&mut png, b"IEND", &[]);

    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../shaderpack/shaders/textures/noise.png");
    std::fs::create_dir_all(target.parent().expect("texture folder"))?;
    std::fs::write(&target, png)?;
    println!("wrote {}", target.display());
    Ok(())
}
