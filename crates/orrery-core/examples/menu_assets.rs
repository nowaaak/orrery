use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::write::ZlibEncoder;
use flate2::{Compression, Crc};

const FACE: usize = 1024;
const ICON: usize = 128;

type Vec3 = [f32; 3];

const NAVY_DEEP: Vec3 = rgb(0x07, 0x09, 0x18);
const NAVY: Vec3 = rgb(0x0a, 0x0d, 0x24);
const NAVY_MID: Vec3 = rgb(0x15, 0x1a, 0x45);
const NAVY_LIGHT: Vec3 = rgb(0x2a, 0x2f, 0x7a);
const INDIGO: Vec3 = rgb(0x63, 0x66, 0xf1);
const LAVENDER: Vec3 = rgb(0xa5, 0xb4, 0xfc);
const CYAN: Vec3 = rgb(0x67, 0xe8, 0xf9);
const TEAL: Vec3 = rgb(0x08, 0x91, 0xb2);
const VIOLET: Vec3 = rgb(0x8b, 0x5c, 0xf6);
const SUN_CORE: Vec3 = rgb(0xff, 0xf4, 0xc2);
const SUN: Vec3 = rgb(0xfb, 0xbf, 0x24);
const SUN_EDGE: Vec3 = rgb(0xd9, 0x77, 0x06);

const fn rgb(r: u8, g: u8, b: u8) -> Vec3 {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(a: Vec3) -> f32 {
    dot(a, a).sqrt()
}

fn normalize(a: Vec3) -> Vec3 {
    scale(a, 1.0 / length(a))
}

fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    add(a, scale(sub(b, a), t))
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix_bits(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

fn hash3(cell: [i32; 3], seed: u32) -> u32 {
    mix_bits(cell[0] as u32 ^ mix_bits(cell[1] as u32 ^ mix_bits(cell[2] as u32 ^ mix_bits(seed))))
}

fn unit(bits: u32) -> f32 {
    bits as f32 / u32::MAX as f32
}

fn value_noise(p: Vec3, seed: u32) -> f32 {
    let cell = p.map(|v| v.floor());
    let f: Vec<f32> = (0..3).map(|i| p[i] - cell[i]).collect();
    let u: Vec<f32> = f.iter().map(|t| t * t * (3.0 - 2.0 * t)).collect();
    let corner = |dx: i32, dy: i32, dz: i32| {
        unit(hash3(
            [
                cell[0] as i32 + dx,
                cell[1] as i32 + dy,
                cell[2] as i32 + dz,
            ],
            seed,
        ))
    };
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = lerp(corner(0, 0, 0), corner(1, 0, 0), u[0]);
    let x10 = lerp(corner(0, 1, 0), corner(1, 1, 0), u[0]);
    let x01 = lerp(corner(0, 0, 1), corner(1, 0, 1), u[0]);
    let x11 = lerp(corner(0, 1, 1), corner(1, 1, 1), u[0]);
    lerp(lerp(x00, x10, u[1]), lerp(x01, x11, u[1]), u[2])
}

fn fbm(p: Vec3, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut total = 0.0;
    let mut point = p;
    for octave in 0..5 {
        value += value_noise(point, seed + octave) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        point = scale(point, 2.03);
    }
    value / total
}

struct StarLayer {
    cells: f32,
    density: f32,
    sigma: f32,
    brightness: (f32, f32),
    halo: f32,
    seed: u32,
}

const STAR_LAYERS: [StarLayer; 3] = [
    StarLayer {
        cells: 230.0,
        density: 0.022,
        sigma: 0.0011,
        brightness: (0.25, 0.7),
        halo: 0.0,
        seed: 11,
    },
    StarLayer {
        cells: 90.0,
        density: 0.03,
        sigma: 0.0016,
        brightness: (0.6, 1.0),
        halo: 0.0,
        seed: 23,
    },
    StarLayer {
        cells: 32.0,
        density: 0.05,
        sigma: 0.0022,
        brightness: (0.9, 1.2),
        halo: 0.12,
        seed: 37,
    },
];

fn star_tint(bits: u32) -> Vec3 {
    match bits % 6 {
        0 => rgb(0xcf, 0xe0, 0xff),
        1 => rgb(0xff, 0xe6, 0xb0),
        2 => LAVENDER,
        _ => [1.0, 1.0, 1.0],
    }
}

fn stars(dir: Vec3) -> Vec3 {
    let mut light = [0.0; 3];
    for layer in &STAR_LAYERS {
        let grid = scale(dir, layer.cells);
        let base = grid.map(|v| v.floor() as i32);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let cell = [base[0] + dx, base[1] + dy, base[2] + dz];
                    let bits = hash3(cell, layer.seed);
                    if unit(bits) > layer.density {
                        continue;
                    }
                    let offset = [
                        unit(mix_bits(bits ^ 1)),
                        unit(mix_bits(bits ^ 2)),
                        unit(mix_bits(bits ^ 3)),
                    ];
                    let position = [
                        cell[0] as f32 + offset[0],
                        cell[1] as f32 + offset[1],
                        cell[2] as f32 + offset[2],
                    ];
                    if (length(position) - layer.cells).abs() > 0.5 {
                        continue;
                    }
                    let distance = length(sub(dir, normalize(position)));
                    let (low, high) = layer.brightness;
                    let brightness = low + (high - low) * unit(mix_bits(bits ^ 4));
                    let core = (-(distance / layer.sigma).powi(2)).exp();
                    let halo = layer.halo * (-(distance / (layer.sigma * 5.0)).powi(2)).exp();
                    light = add(
                        light,
                        scale(star_tint(mix_bits(bits ^ 5)), (core + halo) * brightness),
                    );
                }
            }
        }
    }
    light
}

struct Orbit {
    normal: Vec3,
    radius: f32,
    width: f32,
    colors: (Vec3, Vec3),
    planet_angle: f32,
    planet_radius: f32,
    planet_color: Vec3,
}

struct Scene {
    sun: Vec3,
    sun_radius: f32,
    orbits: [Orbit; 2],
    band: Vec3,
}

impl Scene {
    fn new() -> Self {
        Self {
            sun: normalize([-0.32, 0.2, 1.0]),
            sun_radius: 0.052,
            orbits: [
                Orbit {
                    normal: normalize([0.25, 1.0, -0.62]),
                    radius: 0.4,
                    width: 0.0065,
                    colors: (LAVENDER, INDIGO),
                    planet_angle: 2.2,
                    planet_radius: 0.026,
                    planet_color: rgb(0x8e, 0x9c, 0xf8),
                },
                Orbit {
                    normal: normalize([-0.7, 1.0, -0.15]),
                    radius: 0.25,
                    width: 0.0055,
                    colors: (CYAN, TEAL),
                    planet_angle: 5.1,
                    planet_radius: 0.019,
                    planet_color: rgb(0x4f, 0xd1, 0xe6),
                },
            ],
            band: normalize([0.35, 0.85, 0.4]),
        }
    }

    fn orbit_basis(&self, orbit: &Orbit) -> (Vec3, Vec3) {
        let first = normalize(cross(orbit.normal, [0.0, 0.0, 1.0]));
        (first, cross(orbit.normal, first))
    }

    fn background(&self, dir: Vec3) -> Vec3 {
        let height = dir[1];
        let mut color = if height >= 0.0 {
            mix(NAVY_MID, NAVY, smoothstep(0.0, 0.75, height))
        } else {
            mix(NAVY_MID, NAVY_DEEP, smoothstep(0.0, 0.5, -height))
        };
        let toward_sun = dot(
            normalize([dir[0], 0.0, dir[2]]),
            normalize([self.sun[0], 0.0, self.sun[2]]),
        );
        let horizon_glow = (1.0 - height.abs()).powi(6) * smoothstep(-0.2, 1.0, toward_sun);
        color = add(color, scale(NAVY_LIGHT, 0.35 * horizon_glow));

        let band = (-(dot(dir, self.band) / 0.32).powi(2)).exp();
        let cloud = fbm(scale(dir, 2.4), 101);
        let detail = fbm(scale(dir, 7.0), 211);
        let density = smoothstep(0.38, 0.8, cloud * 0.75 + detail * 0.25) * (0.25 + 0.75 * band);
        let hue = fbm(scale(dir, 1.6), 307);
        let tint = if hue < 0.5 {
            mix(INDIGO, VIOLET, hue * 2.0)
        } else {
            mix(INDIGO, TEAL, (hue - 0.5) * 2.0)
        };
        color = add(color, scale(tint, density * 0.42));
        let dust = smoothstep(0.55, 0.75, fbm(scale(dir, 5.0), 401)) * band;
        color = scale(color, 1.0 - 0.35 * dust);
        add(color, scale(stars(dir), 1.0 - 0.6 * density))
    }

    fn sun_glow(&self, dir: Vec3) -> Vec3 {
        let angle = length(sub(dir, self.sun));
        let glow = 0.5 * (-angle / 0.05).exp() + 0.18 * (-angle / 0.22).exp();
        scale(SUN, glow)
    }

    fn sun_disc(&self, dir: Vec3) -> Option<(Vec3, f32)> {
        let b = dot(dir, self.sun);
        let c = 1.0 - self.sun_radius * self.sun_radius;
        let disc = b * b - c;
        if disc < 0.0 {
            return None;
        }
        let t = b - disc.sqrt();
        let normal = normalize(sub(scale(dir, t), self.sun));
        let facing = (-dot(normal, dir)).clamp(0.0, 1.0);
        let color = if facing > 0.6 {
            mix(SUN, SUN_CORE, smoothstep(0.6, 1.0, facing))
        } else {
            mix(SUN_EDGE, SUN, smoothstep(0.0, 0.6, facing))
        };
        Some((color, t))
    }

    fn orbit_hit(&self, orbit: &Orbit, dir: Vec3) -> Option<(Vec3, f32, f32)> {
        let denom = dot(dir, orbit.normal);
        if denom.abs() < 1e-5 {
            return None;
        }
        let t = dot(self.sun, orbit.normal) / denom;
        if t <= 0.0 {
            return None;
        }
        let offset = sub(scale(dir, t), self.sun);
        let distance = length(offset);
        let coverage = 1.0
            - smoothstep(
                orbit.width * 0.5,
                orbit.width,
                (distance - orbit.radius).abs(),
            );
        if coverage <= 0.0 {
            return None;
        }
        let (first, second) = self.orbit_basis(orbit);
        let angle = dot(offset, second).atan2(dot(offset, first));
        let along = 0.5 + 0.5 * angle.sin();
        let color = mix(orbit.colors.0, orbit.colors.1, along);
        Some((color, coverage * 0.9, t))
    }

    fn planet_hit(&self, orbit: &Orbit, dir: Vec3) -> Option<(Vec3, f32)> {
        let (first, second) = self.orbit_basis(orbit);
        let center = add(
            self.sun,
            scale(
                add(
                    scale(first, orbit.planet_angle.cos()),
                    scale(second, orbit.planet_angle.sin()),
                ),
                orbit.radius,
            ),
        );
        let b = dot(dir, center);
        let c = dot(center, center) - orbit.planet_radius * orbit.planet_radius;
        let disc = b * b - c;
        if disc < 0.0 {
            return None;
        }
        let t = b - disc.sqrt();
        let normal = normalize(sub(scale(dir, t), center));
        let light = dot(normal, normalize(sub(self.sun, center))).max(0.0);
        Some((scale(orbit.planet_color, 0.3 + 0.8 * light), t))
    }

    fn geometry(&self, dir: Vec3) -> Option<(Vec3, f32)> {
        let mut layers: Vec<(Vec3, f32, f32)> = Vec::new();
        if let Some((color, t)) = self.sun_disc(dir) {
            layers.push((color, 1.0, t));
        }
        for orbit in &self.orbits {
            if let Some(hit) = self.orbit_hit(orbit, dir) {
                layers.push(hit);
            }
            if let Some((color, t)) = self.planet_hit(orbit, dir) {
                layers.push((color, 1.0, t));
            }
        }
        if layers.is_empty() {
            return None;
        }
        layers.sort_by(|a, b| b.2.total_cmp(&a.2));
        let mut color = [0.0; 3];
        let mut alpha = 0.0;
        for (layer, coverage, _) in layers {
            color = mix(color, layer, coverage);
            alpha = alpha + (1.0 - alpha) * coverage;
        }
        Some((scale(color, 1.0 / alpha.max(1e-4)), alpha))
    }

    fn shade(&self, center: Vec3, samples: &[Vec3]) -> Vec3 {
        let mut color = add(self.background(center), self.sun_glow(center));
        let mut geometry = [0.0; 3];
        let mut coverage = 0.0;
        for &sample in samples {
            if let Some((layer, alpha)) = self.geometry(sample) {
                geometry = add(geometry, scale(layer, alpha));
                coverage += alpha;
            }
        }
        if coverage > 0.0 {
            let average = coverage / samples.len() as f32;
            color = mix(color, scale(geometry, 1.0 / coverage), average);
        }
        color
    }
}

struct FaceBasis {
    forward: Vec3,
    right: Vec3,
    up: Vec3,
}

fn face_basis(face: usize) -> FaceBasis {
    let up = [0.0, 1.0, 0.0];
    match face {
        0 => FaceBasis {
            forward: [0.0, 0.0, 1.0],
            right: [-1.0, 0.0, 0.0],
            up,
        },
        1 => FaceBasis {
            forward: [-1.0, 0.0, 0.0],
            right: [0.0, 0.0, -1.0],
            up,
        },
        2 => FaceBasis {
            forward: [0.0, 0.0, -1.0],
            right: [1.0, 0.0, 0.0],
            up,
        },
        3 => FaceBasis {
            forward: [1.0, 0.0, 0.0],
            right: [0.0, 0.0, 1.0],
            up,
        },
        4 => FaceBasis {
            forward: [0.0, 1.0, 0.0],
            right: [-1.0, 0.0, 0.0],
            up: [0.0, 0.0, -1.0],
        },
        _ => FaceBasis {
            forward: [0.0, -1.0, 0.0],
            right: [-1.0, 0.0, 0.0],
            up: [0.0, 0.0, 1.0],
        },
    }
}

fn face_direction(basis: &FaceBasis, x: f32, y: f32) -> Vec3 {
    let u = 2.0 * x / FACE as f32 - 1.0;
    let v = 2.0 * y / FACE as f32 - 1.0;
    normalize(add(
        basis.forward,
        add(scale(basis.right, u), scale(basis.up, -v)),
    ))
}

fn to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn panorama_face(scene: &Scene, face: usize) -> Vec<u8> {
    let basis = face_basis(face);
    let mut pixels = Vec::with_capacity(FACE * FACE * 3);
    for y in 0..FACE {
        for x in 0..FACE {
            let (fx, fy) = (x as f32, y as f32);
            let center = face_direction(&basis, fx + 0.5, fy + 0.5);
            let samples = [
                face_direction(&basis, fx + 0.25, fy + 0.25),
                face_direction(&basis, fx + 0.75, fy + 0.25),
                face_direction(&basis, fx + 0.25, fy + 0.75),
                face_direction(&basis, fx + 0.75, fy + 0.75),
            ];
            let color = scene.shade(center, &samples);
            pixels.extend(color.map(to_byte));
        }
    }
    pixels
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let (pa, pb, pc) = (
        (p - i16::from(a)).abs(),
        (p - i16::from(b)).abs(),
        (p - i16::from(c)).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

fn filter_row(filter: u8, row: &[u8], previous: &[u8], channels: usize) -> Vec<u8> {
    (0..row.len())
        .map(|i| {
            let a = if i >= channels { row[i - channels] } else { 0 };
            let b = previous[i];
            let c = if i >= channels {
                previous[i - channels]
            } else {
                0
            };
            let predicted = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((u16::from(a) + u16::from(b)) / 2) as u8,
                _ => paeth(a, b, c),
            };
            row[i].wrapping_sub(predicted)
        })
        .collect()
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut crc = Crc::new();
    crc.update(kind);
    crc.update(data);
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc.sum().to_be_bytes());
}

fn encode_png(width: usize, height: usize, channels: usize, pixels: &[u8]) -> Vec<u8> {
    let stride = width * channels;
    let mut raw = Vec::with_capacity(height * (stride + 1));
    let mut previous = vec![0u8; stride];
    for row in pixels.chunks(stride) {
        let (filter, filtered) = (0..5u8)
            .map(|filter| (filter, filter_row(filter, row, &previous, channels)))
            .min_by_key(|(_, filtered)| {
                filtered
                    .iter()
                    .map(|&v| u32::from((v as i8).unsigned_abs()))
                    .sum::<u32>()
            })
            .expect("five filters");
        raw.push(filter);
        raw.extend_from_slice(&filtered);
        previous = row.to_vec();
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&raw).expect("compress in memory");
    let compressed = encoder.finish().expect("compress in memory");

    let color_type = if channels == 4 { 6 } else { 2 };
    let mut header = Vec::new();
    header.extend_from_slice(&(width as u32).to_be_bytes());
    header.extend_from_slice(&(height as u32).to_be_bytes());
    header.extend_from_slice(&[8, color_type, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &compressed);
    chunk(&mut png, b"IEND", &[]);
    png
}

struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<[u8; 4]>,
}

impl Canvas {
    fn new(width: usize, height: usize, fill: [u8; 4]) -> Self {
        Self {
            width,
            height,
            pixels: vec![fill; width * height],
        }
    }

    fn set(&mut self, x: usize, y: usize, color: [u8; 4]) {
        self.pixels[y * self.width + x] = color;
    }

    fn rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: [u8; 4]) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.set(x, y, color);
            }
        }
    }

    fn png(&self) -> Vec<u8> {
        let bytes: Vec<u8> = self.pixels.iter().flatten().copied().collect();
        encode_png(self.width, self.height, 4, &bytes)
    }
}

fn opaque(color: Vec3) -> [u8; 4] {
    let [r, g, b] = color.map(to_byte);
    [r, g, b, 255]
}

fn translucent(color: Vec3, alpha: u8) -> [u8; 4] {
    let [r, g, b] = color.map(to_byte);
    [r, g, b, alpha]
}

struct ButtonStyle {
    outline: Vec3,
    light: Vec3,
    top: Vec3,
    bottom: Vec3,
    shadow: Vec3,
}

fn bevel(width: usize, height: usize, style: &ButtonStyle) -> Canvas {
    let mut canvas = Canvas::new(width, height, opaque(style.outline));
    for y in 1..height - 1 {
        let t = (y - 1) as f32 / (height - 3).max(1) as f32;
        let fill = opaque(mix(style.top, style.bottom, t));
        canvas.rect(1, y, width - 1, y + 1, fill);
    }
    canvas.rect(1, 1, width - 1, 2, opaque(style.light));
    canvas.rect(1, 1, 2, height - 3, opaque(style.light));
    canvas.rect(1, height - 3, width - 1, height - 1, opaque(style.shadow));
    canvas
}

fn flat(width: usize, height: usize, outline: Vec3, fill: Vec3) -> Canvas {
    let mut canvas = Canvas::new(width, height, opaque(outline));
    canvas.rect(1, 1, width - 1, height - 1, opaque(fill));
    canvas
}

fn nine_slice(width: usize, height: usize, border: &str) -> String {
    format!(
        "{{\n    \"gui\": {{\n        \"scaling\": {{\n            \"type\": \"nine_slice\",\n            \"width\": {width},\n            \"height\": {height},\n            \"border\": {border}\n        }}\n    }}\n}}\n"
    )
}

fn widget_files() -> Vec<(String, Vec<u8>)> {
    let button = ButtonStyle {
        outline: NAVY_DEEP,
        light: rgb(0x4a, 0x51, 0xb8),
        top: rgb(0x24, 0x2a, 0x6e),
        bottom: rgb(0x1a, 0x1f, 0x55),
        shadow: rgb(0x11, 0x14, 0x3c),
    };
    let highlighted = ButtonStyle {
        outline: LAVENDER,
        light: rgb(0x8a, 0x93, 0xf5),
        top: rgb(0x3a, 0x41, 0xa8),
        bottom: rgb(0x2e, 0x34, 0x8f),
        shadow: rgb(0x1e, 0x23, 0x6b),
    };
    let handle = ButtonStyle {
        outline: NAVY_DEEP,
        light: CYAN,
        top: rgb(0x16, 0xa8, 0xc9),
        bottom: TEAL,
        shadow: rgb(0x06, 0x5f, 0x75),
    };
    let handle_highlighted = ButtonStyle {
        outline: rgb(0xe0, 0xf7, 0xff),
        light: rgb(0xa5, 0xf3, 0xfc),
        top: rgb(0x2c, 0xc4, 0xe2),
        bottom: rgb(0x0e, 0xa5, 0xc7),
        shadow: rgb(0x07, 0x74, 0x8f),
    };
    let handle_border = "{ \"left\": 2, \"top\": 2, \"right\": 2, \"bottom\": 3 }";
    let sprites: Vec<(&str, Canvas, String)> = vec![
        ("button", bevel(200, 20, &button), nine_slice(200, 20, "3")),
        (
            "button_highlighted",
            bevel(200, 20, &highlighted),
            nine_slice(200, 20, "3"),
        ),
        (
            "button_disabled",
            flat(200, 20, rgb(0x05, 0x06, 0x10), rgb(0x14, 0x17, 0x30)),
            nine_slice(200, 20, "1"),
        ),
        (
            "slider",
            flat(200, 20, NAVY_DEEP, rgb(0x10, 0x13, 0x30)),
            nine_slice(200, 20, "1"),
        ),
        (
            "slider_highlighted",
            flat(200, 20, LAVENDER, rgb(0x16, 0x1a, 0x40)),
            nine_slice(200, 20, "1"),
        ),
        (
            "slider_handle",
            bevel(8, 20, &handle),
            nine_slice(8, 20, handle_border),
        ),
        (
            "slider_handle_highlighted",
            bevel(8, 20, &handle_highlighted),
            nine_slice(8, 20, handle_border),
        ),
    ];
    let mut files = Vec::new();
    for (name, canvas, meta) in sprites {
        let path = format!("assets/minecraft/textures/gui/sprites/widget/{name}.png");
        files.push((format!("{path}.mcmeta"), meta.into_bytes()));
        files.push((path, canvas.png()));
    }
    files
}

fn background_files() -> Vec<(String, Vec<u8>)> {
    let gui = "assets/minecraft/textures/gui";
    let mut legacy = Canvas::new(16, 16, opaque(rgb(0x38, 0x3e, 0x96)));
    for (x, y) in [(3, 2), (11, 6), (6, 12), (14, 13)] {
        legacy.set(x, y, opaque(rgb(0x9a, 0xa3, 0xf0)));
    }
    let mut header = Canvas::new(32, 2, translucent(NAVY_DEEP, 191));
    header.rect(0, 0, 32, 1, translucent(LAVENDER, 64));
    let mut footer = Canvas::new(32, 2, translucent(NAVY_DEEP, 191));
    footer.rect(0, 1, 32, 2, translucent(LAVENDER, 64));
    vec![
        (format!("{gui}/options_background.png"), legacy.png()),
        (
            format!("{gui}/menu_background.png"),
            Canvas::new(16, 16, translucent(NAVY, 120)).png(),
        ),
        (
            format!("{gui}/menu_list_background.png"),
            Canvas::new(16, 16, translucent(NAVY, 170)).png(),
        ),
        (format!("{gui}/header_separator.png"), header.png()),
        (format!("{gui}/footer_separator.png"), footer.png()),
    ]
}

fn ellipse_distance(point: (f32, f32), radii: (f32, f32), rotation: f32) -> f32 {
    let (sin, cos) = rotation.to_radians().sin_cos();
    let x = point.0 * cos + point.1 * sin;
    let y = -point.0 * sin + point.1 * cos;
    let normalized = ((x / radii.0).powi(2) + (y / radii.1).powi(2)).sqrt();
    (normalized - 1.0) * radii.0.min(radii.1)
}

fn icon_color(x: f32, y: f32) -> [f32; 4] {
    let inside_rounded = {
        let (cx, cy) = ((x - 512.0).abs() - 256.0, (y - 512.0).abs() - 256.0);
        let corner = (cx.max(0.0).powi(2) + cy.max(0.0).powi(2)).sqrt();
        corner <= 224.0 && (x - 512.0).abs() <= 480.0 && (y - 512.0).abs() <= 480.0
    };
    if !inside_rounded {
        return [0.0; 4];
    }
    let sky_distance = (((x - 512.0) / 768.0).powi(2) + ((y - 389.0) / 768.0).powi(2)).sqrt();
    let mut color = if sky_distance < 0.55 {
        mix(NAVY_LIGHT, NAVY_MID, sky_distance / 0.55)
    } else {
        mix(NAVY_MID, NAVY, ((sky_distance - 0.55) / 0.45).min(1.0))
    };
    let point = (x - 512.0, y - 512.0);
    if ellipse_distance(point, (370.0, 150.0), -24.0).abs() < 20.0 {
        color = mix(LAVENDER, INDIGO, (x + y) / 2048.0);
    }
    if ellipse_distance(point, (300.0, 112.0), 32.0).abs() < 18.0 {
        color = mix(CYAN, TEAL, (2048.0 - x + y) / 2048.0 * 0.5);
    }
    let sun_distance = (point.0.powi(2) + point.1.powi(2)).sqrt();
    if sun_distance < 140.0 {
        let highlight = (((x - 453.0).powi(2) + (y - 459.0).powi(2)).sqrt() / 182.0).min(1.0);
        color = if highlight < 0.45 {
            mix(SUN_CORE, SUN, highlight / 0.45)
        } else {
            mix(SUN, SUN_EDGE, (highlight - 0.45) / 0.55)
        };
    }
    [color[0], color[1], color[2], 1.0]
}

fn pack_icon() -> Vec<u8> {
    let samples = 4;
    let mut canvas = Canvas::new(ICON, ICON, [0; 4]);
    for y in 0..ICON {
        for x in 0..ICON {
            let mut sum = [0.0f32; 4];
            for sy in 0..samples {
                for sx in 0..samples {
                    let px = (x as f32 + (sx as f32 + 0.5) / samples as f32) * 1024.0 / ICON as f32;
                    let py = (y as f32 + (sy as f32 + 0.5) / samples as f32) * 1024.0 / ICON as f32;
                    let [r, g, b, a] = icon_color(px, py);
                    sum = [sum[0] + r * a, sum[1] + g * a, sum[2] + b * a, sum[3] + a];
                }
            }
            let alpha = sum[3] / (samples * samples) as f32;
            if alpha > 0.0 {
                let color = [sum[0] / sum[3], sum[1] / sum[3], sum[2] / sum[3]];
                let [r, g, b] = color.map(to_byte);
                canvas.set(x, y, [r, g, b, to_byte(alpha)]);
            }
        }
    }
    canvas.png()
}

const TITLE_WIDTH: usize = 1600;
const TITLE_HEIGHT: usize = 900;
const GLOW: usize = 256;
const UI_ICON: usize = 48;
const ICON_SAMPLES: usize = 8;
const STROKE: f32 = 0.09;
const TITLE_BASE: Vec3 = rgb(0x0b, 0x0b, 0x12);

struct Glow {
    center: (f32, f32),
    radius: f32,
    color: Vec3,
    strength: f32,
}

const TITLE_GLOWS: [Glow; 4] = [
    Glow {
        center: (0.78, 0.3),
        radius: 0.42,
        color: INDIGO,
        strength: 0.5,
    },
    Glow {
        center: (0.98, 0.88),
        radius: 0.38,
        color: VIOLET,
        strength: 0.34,
    },
    Glow {
        center: (0.52, 1.08),
        radius: 0.34,
        color: TEAL,
        strength: 0.26,
    },
    Glow {
        center: (0.05, -0.12),
        radius: 0.36,
        color: NAVY_LIGHT,
        strength: 0.22,
    },
];

const ORBIT_CENTER: (f32, f32) = (0.74, 0.42);

fn title_color(u: f32, v: f32) -> Vec3 {
    let aspect = TITLE_WIDTH as f32 / TITLE_HEIGHT as f32;
    let mut color = TITLE_BASE;
    for glow in &TITLE_GLOWS {
        let dx = (u - glow.center.0) * aspect;
        let dy = v - glow.center.1;
        let falloff = (-(dx * dx + dy * dy) / (glow.radius * glow.radius) * 2.0).exp();
        color = add(color, scale(glow.color, glow.strength * falloff));
    }
    let point = (
        (u - ORBIT_CENTER.0) * aspect * 1000.0,
        (v - ORBIT_CENTER.1) * 1000.0,
    );
    for (radii, width, alpha) in [
        ((520.0, 150.0), 2.2, 0.16),
        ((800.0, 240.0), 1.8, 0.1),
        ((1120.0, 330.0), 1.6, 0.06),
    ] {
        let distance = ellipse_distance(point, radii, -16.0).abs();
        let line = 1.0 - smoothstep(width * 0.5, width * 1.5, distance);
        color = mix(color, LAVENDER, line * alpha);
    }
    let core = (point.0 * point.0 + point.1 * point.1).sqrt();
    color = add(
        color,
        scale(
            LAVENDER,
            0.5 * (-core / 26.0).exp() + 0.12 * (-core / 140.0).exp(),
        ),
    );
    for (angle, radii, size) in [(2.5f32, (520.0, 150.0), 9.0), (5.6f32, (800.0, 240.0), 7.0)] {
        let (sin, cos) = (-16.0f32).to_radians().sin_cos();
        let local = (radii.0 * angle.cos(), radii.1 * angle.sin());
        let planet = (local.0 * cos - local.1 * sin, local.0 * sin + local.1 * cos);
        let distance = ((point.0 - planet.0).powi(2) + (point.1 - planet.1).powi(2)).sqrt();
        color = mix(
            color,
            rgb(0xe0, 0xe7, 0xff),
            1.0 - smoothstep(size - 1.5, size + 1.5, distance),
        );
    }
    let vignette = 1.0
        - 0.35
            * smoothstep(
                0.35,
                1.0,
                ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 1.4,
            );
    scale(color, vignette)
}

fn title_background() -> Vec<u8> {
    let mut pixels = Vec::with_capacity(TITLE_WIDTH * TITLE_HEIGHT * 3);
    for y in 0..TITLE_HEIGHT {
        for x in 0..TITLE_WIDTH {
            let color = title_color(
                (x as f32 + 0.5) / TITLE_WIDTH as f32,
                (y as f32 + 0.5) / TITLE_HEIGHT as f32,
            );
            let dither = (unit(hash3([x as i32, y as i32, 0], 977)) - 0.5) / 255.0;
            pixels.extend(color.map(|channel| to_byte(channel + dither)));
        }
    }
    pixels
}

fn glow_texture() -> Vec<u8> {
    let mut canvas = Canvas::new(GLOW, GLOW, [255, 255, 255, 0]);
    for y in 0..GLOW {
        for x in 0..GLOW {
            let dx = (x as f32 + 0.5) / GLOW as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / GLOW as f32 * 2.0 - 1.0;
            let distance = (dx * dx + dy * dy).sqrt();
            let alpha =
                (-(distance * distance) * 4.0).exp() * (1.0 - smoothstep(0.85, 1.0, distance));
            let dither = (unit(hash3([x as i32, y as i32, 1], 983)) - 0.5) / 255.0;
            canvas.set(x, y, [255, 255, 255, to_byte(alpha + dither)]);
        }
    }
    canvas.png()
}

fn segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (px, py) = (p.0 - a.0, p.1 - a.1);
    let (bx, by) = (b.0 - a.0, b.1 - a.1);
    let t = ((px * bx + py * by) / (bx * bx + by * by)).clamp(0.0, 1.0);
    ((px - bx * t).powi(2) + (py - by * t).powi(2)).sqrt()
}

fn polyline(p: (f32, f32), points: &[(f32, f32)]) -> f32 {
    points
        .windows(2)
        .map(|pair| segment(p, pair[0], pair[1]))
        .fold(f32::MAX, f32::min)
}

fn ring(p: (f32, f32), radius: f32) -> f32 {
    ((p.0 * p.0 + p.1 * p.1).sqrt() - radius).abs()
}

fn ellipse_ring(p: (f32, f32), radii: (f32, f32), rotation: f32) -> f32 {
    ellipse_distance((p.0, p.1), radii, rotation).abs()
}

fn inside_triangle(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let edge = |u: (f32, f32), v: (f32, f32)| (v.0 - u.0) * (p.1 - u.1) - (v.1 - u.1) * (p.0 - u.0);
    let (d1, d2, d3) = (edge(a, b), edge(b, c), edge(c, a));
    (d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0) || (d1 <= 0.0 && d2 <= 0.0 && d3 <= 0.0)
}

fn icon_inside(name: &str, p: (f32, f32)) -> bool {
    let stroke = |distance: f32| distance <= STROKE;
    let length = (p.0 * p.0 + p.1 * p.1).sqrt();
    match name {
        "play" => inside_triangle(p, (-0.42, -0.64), (0.7, 0.0), (-0.42, 0.64)),
        "cube" => {
            let top = (0.0, -0.82);
            let upper_right = (0.71, -0.41);
            let lower_right = (0.71, 0.41);
            let bottom = (0.0, 0.82);
            let lower_left = (-0.71, 0.41);
            let upper_left = (-0.71, -0.41);
            let hexagon = polyline(
                p,
                &[
                    top,
                    upper_right,
                    lower_right,
                    bottom,
                    lower_left,
                    upper_left,
                    top,
                ],
            );
            let spokes = polyline(p, &[upper_left, (0.0, 0.0), upper_right]).min(segment(
                p,
                (0.0, 0.0),
                bottom,
            ));
            stroke(hexagon.min(spokes))
        }
        "globe" => stroke(
            ring(p, 0.8)
                .min(ellipse_ring((p.0, p.1), (0.36, 0.8), 0.0))
                .min(segment(p, (-0.8, 0.0), (0.8, 0.0))),
        ),
        "sliders" => {
            let lines = [(-0.5, 0.3), (0.0, -0.35), (0.5, 0.15)];
            lines.iter().any(|&(y, knob)| {
                let knob_distance = ((p.0 - knob).powi(2) + (p.1 - y).powi(2)).sqrt();
                if knob_distance <= 0.2 + STROKE {
                    stroke(ring((p.0 - knob, p.1 - y), 0.2))
                } else {
                    stroke(segment(p, (-0.82, y), (0.82, y)))
                }
            })
        }
        "orbit" => {
            let planet = ((p.0 - 0.62).powi(2) + (p.1 + 0.36).powi(2)).sqrt();
            length <= 0.26
                || planet <= 0.13
                || (stroke(ellipse_ring((p.0, p.1), (0.86, 0.38), -28.0))
                    && planet > 0.13 + STROKE * 1.6)
        }
        "power" => {
            let angle = p.1.atan2(p.0);
            let gap = (angle + std::f32::consts::FRAC_PI_2).abs() < 0.62;
            (stroke(ring(p, 0.68)) && !gap) || stroke(segment(p, (0.0, -0.88), (0.0, -0.12)))
        }
        "chevron" => stroke(polyline(p, &[(-0.5, -0.2), (0.0, 0.3), (0.5, -0.2)])),
        "arrow" => stroke(
            segment(p, (-0.62, 0.0), (0.62, 0.0))
                .min(polyline(p, &[(0.14, -0.46), (0.62, 0.0), (0.14, 0.46)])),
        ),
        _ => false,
    }
}

fn ui_icon(name: &str) -> Vec<u8> {
    let mut canvas = Canvas::new(UI_ICON, UI_ICON, [255, 255, 255, 0]);
    let total = (ICON_SAMPLES * ICON_SAMPLES) as f32;
    for y in 0..UI_ICON {
        for x in 0..UI_ICON {
            let mut hits = 0;
            for sy in 0..ICON_SAMPLES {
                for sx in 0..ICON_SAMPLES {
                    let px = (x as f32 + (sx as f32 + 0.5) / ICON_SAMPLES as f32) / UI_ICON as f32
                        * 2.0
                        - 1.0;
                    let py = (y as f32 + (sy as f32 + 0.5) / ICON_SAMPLES as f32) / UI_ICON as f32
                        * 2.0
                        - 1.0;
                    if icon_inside(name, (px * 1.08, py * 1.08)) {
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                canvas.set(x, y, [255, 255, 255, to_byte(hits as f32 / total)]);
            }
        }
    }
    canvas.png()
}

fn companion_files() -> Vec<(String, Vec<u8>)> {
    let gui = "assets/orrery-companion/textures/gui";
    let blur = b"{\n    \"texture\": {\n        \"blur\": true\n    }\n}\n".to_vec();
    let mut files = vec![
        (
            format!("{gui}/title/background.png"),
            encode_png(TITLE_WIDTH, TITLE_HEIGHT, 3, &title_background()),
        ),
        (format!("{gui}/title/background.png.mcmeta"), blur.clone()),
        (format!("{gui}/title/glow.png"), glow_texture()),
        (format!("{gui}/title/glow.png.mcmeta"), blur.clone()),
    ];
    for name in [
        "play", "cube", "globe", "sliders", "orbit", "power", "chevron", "arrow",
    ] {
        files.push((format!("{gui}/icons/{name}.png"), ui_icon(name)));
        files.push((format!("{gui}/icons/{name}.png.mcmeta"), blur.clone()));
    }
    files
}

fn write(root: &Path, name: &str, bytes: &[u8]) -> std::io::Result<()> {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().expect("asset folder"))?;
    std::fs::write(&path, bytes)?;
    println!("wrote {name} ({} bytes)", bytes.len());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../menupack");
    let scene = Scene::new();
    for face in 0..6 {
        let png = encode_png(FACE, FACE, 3, &panorama_face(&scene, face));
        write(
            &root,
            &format!("assets/minecraft/textures/gui/title/background/panorama_{face}.png"),
            &png,
        )?;
    }
    for (name, bytes) in widget_files().into_iter().chain(background_files()) {
        write(&root, &name, &bytes)?;
    }
    write(&root, "pack.png", &pack_icon())?;
    let companion =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../companion/src/main/resources");
    for (name, bytes) in companion_files() {
        write(&companion, &name, &bytes)?;
    }
    Ok(())
}
