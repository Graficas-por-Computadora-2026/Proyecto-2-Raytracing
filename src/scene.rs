use raylib::prelude::*;

use crate::generated_map::{BLOCK_COUNT, BLOCK_DATA_B64, MAP_SIZE, PALETTE, TEXTURE_PATHS};
use crate::geometry::{Aabb, Bvh, Cuboid};
use crate::math::{Vec3, hash01};

pub const WATER_SURFACE: u8 = 9;
pub const WATER_LEVEL: f32 = -0.65;

pub struct CpuTexture {
    width: usize,
    height: usize,
    pixels: Vec<Vec3>,
}

impl CpuTexture {
    fn load(path: &str) -> Self {
        let image = Image::load_image(path).unwrap_or_else(|_| {
            Image::gen_image_checked(16, 16, 8, 8, Color::MAGENTA, Color::BLACK)
        });
        Self::from_image(image)
    }
    fn from_image(image: Image) -> Self {
        let width = image.width() as usize;
        let height = image.height() as usize;
        let colors = image.get_image_data();
        let pixels = colors
            .iter()
            .map(|c| {
                Vec3::new(
                    (c.r as f32 / 255.0).powf(2.2),
                    (c.g as f32 / 255.0).powf(2.2),
                    (c.b as f32 / 255.0).powf(2.2),
                )
            })
            .collect();
        Self {
            width,
            height,
            pixels,
        }
    }
    pub fn sample(&self, uv: (f32, f32)) -> Vec3 {
        let u = ((uv.0.fract() + 1.0).fract() * self.width as f32) as usize % self.width;
        let v = (((-uv.1).fract() + 1.0).fract() * self.height as f32) as usize % self.height;
        self.pixels[v * self.width + u]
    }
    pub fn sample_normalized(&self, uv: (f32, f32)) -> Vec3 {
        let x = uv.0.clamp(0.0, 0.999_999) * (self.width - 1) as f32;
        let y = uv.1.clamp(0.0, 0.999_999) * (self.height - 1) as f32;
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let top = self.pixels[y0 * self.width + x0].mix(self.pixels[y0 * self.width + x1], tx);
        let bottom = self.pixels[y1 * self.width + x0].mix(self.pixels[y1 * self.width + x1], tx);
        top.mix(bottom, ty)
    }
    pub fn height(&self, uv: (f32, f32)) -> f32 {
        let c = self.sample(uv);
        c.x * 0.3 + c.y * 0.59 + c.z * 0.11
    }
}

#[derive(Clone, Copy)]
pub struct Surface {
    pub albedo: f32,
    pub specular: f32,
    pub transparency: f32,
    pub reflectivity: f32,
    pub ior: f32,
    pub emission: f32,
    pub roughness: f32,
    pub normal_strength: f32,
}

pub fn surface(id: u8) -> Surface {
    match id {
        2 => Surface {
            albedo: 0.78,
            specular: 0.65,
            transparency: 0.82,
            reflectivity: 0.16,
            ior: 1.50,
            emission: 0.,
            roughness: 0.04,
            normal_strength: 0.,
        },
        3 => Surface {
            albedo: 0.9,
            specular: 0.25,
            transparency: 0.,
            reflectivity: 0.06,
            ior: 1.,
            emission: 3.0,
            roughness: 0.35,
            normal_strength: 0.1,
        },
        4 => Surface {
            albedo: 0.55,
            specular: 1.0,
            transparency: 0.,
            reflectivity: 0.52,
            ior: 1.,
            emission: 0.,
            roughness: 0.08,
            normal_strength: 0.08,
        },
        5 => Surface {
            albedo: 0.9,
            specular: 0.12,
            transparency: 0.,
            reflectivity: 0.02,
            ior: 1.,
            emission: 0.,
            roughness: 0.9,
            normal_strength: 0.32,
        },
        6 => Surface {
            albedo: 0.88,
            specular: 0.16,
            transparency: 0.,
            reflectivity: 0.03,
            ior: 1.,
            emission: 0.,
            roughness: 0.72,
            normal_strength: 0.24,
        },
        7 => Surface {
            albedo: 0.82,
            specular: 0.2,
            transparency: 0.,
            reflectivity: 0.04,
            ior: 1.,
            emission: 0.,
            roughness: 0.68,
            normal_strength: 0.25,
        },
        8 => Surface {
            albedo: 0.9,
            specular: 0.1,
            transparency: 0.,
            reflectivity: 0.015,
            ior: 1.,
            emission: 0.,
            roughness: 0.95,
            normal_strength: 0.18,
        },
        9 => Surface {
            albedo: 0.65,
            specular: 0.9,
            transparency: 0.48,
            reflectivity: 0.45,
            ior: 1.333,
            emission: 0.,
            roughness: 0.04,
            normal_strength: 0.18,
        },
        10 => Surface {
            albedo: 0.64,
            specular: 0.14,
            transparency: 0.,
            reflectivity: 0.02,
            ior: 1.,
            emission: 0.,
            roughness: 0.78,
            normal_strength: 0.08,
        },
        _ => Surface {
            albedo: 0.88,
            specular: 0.2,
            transparency: 0.,
            reflectivity: 0.035,
            ior: 1.,
            emission: 0.,
            roughness: 0.75,
            normal_strength: 0.12,
        },
    }
}

pub struct Scene {
    pub bvh: Bvh,
    pub textures: Vec<CpuTexture>,
    idol_sky: Vec<CpuTexture>,
    pub center: Vec3,
    pub block_count: usize,
    pub cuboid_count: usize,
}

fn decode_base64(text: &str) -> Vec<u8> {
    let mut result = Vec::with_capacity(text.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;
    for b in text.bytes() {
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    result
}

fn add_box(out: &mut Vec<Cuboid>, p: Vec3, a: Vec3, b: Vec3, texture: u8, surface: u8) {
    out.push(Cuboid {
        bounds: Aabb {
            min: p + a,
            max: p + b,
        },
        texture,
        surface,
    });
}

fn add_vertical_frame(
    out: &mut Vec<Cuboid>,
    p: Vec3,
    along_x: bool,
    offset: f32,
    texture: u8,
    surface: u8,
) {
    let d = 0.12;
    let border = 0.19;
    if along_x {
        add_box(
            out,
            p,
            Vec3::new(0., 0., offset),
            Vec3::new(border, 1., offset + d),
            texture,
            surface,
        );
        add_box(
            out,
            p,
            Vec3::new(1. - border, 0., offset),
            Vec3::new(1., 1., offset + d),
            texture,
            surface,
        );
        add_box(
            out,
            p,
            Vec3::new(border, 0., offset),
            Vec3::new(1. - border, border, offset + d),
            texture,
            surface,
        );
        add_box(
            out,
            p,
            Vec3::new(border, 1. - border, offset),
            Vec3::new(1. - border, 1., offset + d),
            texture,
            surface,
        );
    } else {
        add_box(
            out,
            p,
            Vec3::new(offset, 0., 0.),
            Vec3::new(offset + d, 1., border),
            texture,
            surface,
        );
        add_box(
            out,
            p,
            Vec3::new(offset, 0., 1. - border),
            Vec3::new(offset + d, 1., 1.),
            texture,
            surface,
        );
        add_box(
            out,
            p,
            Vec3::new(offset, 0., border),
            Vec3::new(offset + d, border, 1. - border),
            texture,
            surface,
        );
        add_box(
            out,
            p,
            Vec3::new(offset, 1. - border, border),
            Vec3::new(offset + d, 1., 1. - border),
            texture,
            surface,
        );
    }
}

fn add_horizontal_frame(out: &mut Vec<Cuboid>, p: Vec3, offset: f32, texture: u8, surface: u8) {
    let d = 0.12;
    let border = 0.19;
    add_box(
        out,
        p,
        Vec3::new(0., offset, 0.),
        Vec3::new(border, offset + d, 1.),
        texture,
        surface,
    );
    add_box(
        out,
        p,
        Vec3::new(1. - border, offset, 0.),
        Vec3::new(1., offset + d, 1.),
        texture,
        surface,
    );
    add_box(
        out,
        p,
        Vec3::new(border, offset, 0.),
        Vec3::new(1. - border, offset + d, border),
        texture,
        surface,
    );
    add_box(
        out,
        p,
        Vec3::new(border, offset, 1. - border),
        Vec3::new(1. - border, offset + d, 1.),
        texture,
        surface,
    );
}

fn add_block(out: &mut Vec<Cuboid>, p: Vec3, def: crate::generated_map::BlockDef) {
    let t = def.texture;
    let s = def.surface;
    let f = def.facing;
    match def.shape {
        1 => add_box(out, p, Vec3::ZERO, Vec3::new(1., 0.5, 1.), t, s),
        2 => add_box(out, p, Vec3::new(0., 0.5, 0.), Vec3::ONE, t, s),
        3 | 4 => {
            let top = def.shape == 4;
            let (y0, y1, sy0, sy1) = if top {
                (0.5, 1., 0., 0.5)
            } else {
                (0., 0.5, 0.5, 1.)
            };
            add_box(out, p, Vec3::new(0., y0, 0.), Vec3::new(1., y1, 1.), t, s);
            match f {
                0 => add_box(
                    out,
                    p,
                    Vec3::new(0., sy0, 0.5),
                    Vec3::new(1., sy1, 1.),
                    t,
                    s,
                ),
                1 => add_box(
                    out,
                    p,
                    Vec3::new(0., sy0, 0.),
                    Vec3::new(0.5, sy1, 1.),
                    t,
                    s,
                ),
                2 => add_box(
                    out,
                    p,
                    Vec3::new(0., sy0, 0.),
                    Vec3::new(1., sy1, 0.5),
                    t,
                    s,
                ),
                _ => add_box(
                    out,
                    p,
                    Vec3::new(0.5, sy0, 0.),
                    Vec3::new(1., sy1, 1.),
                    t,
                    s,
                ),
            }
        }
        5 => {
            let c = def.flags;
            if c & 3 != 0 || c == 0 {
                add_box(
                    out,
                    p,
                    Vec3::new(0., 0., 0.44),
                    Vec3::new(1., 1., 0.56),
                    t,
                    s,
                )
            }
            if c & 12 != 0 {
                add_box(
                    out,
                    p,
                    Vec3::new(0.44, 0., 0.),
                    Vec3::new(0.56, 1., 1.),
                    t,
                    s,
                )
            }
        }
        6 => {
            add_box(
                out,
                p,
                Vec3::new(0.34, 0., 0.34),
                Vec3::new(0.66, 1., 0.66),
                t,
                s,
            );
            let c = def.flags;
            if c & 1 != 0 {
                add_box(
                    out,
                    p,
                    Vec3::new(0.5, 0.28, 0.4),
                    Vec3::new(1., 0.82, 0.6),
                    t,
                    s,
                )
            }
            if c & 2 != 0 {
                add_box(
                    out,
                    p,
                    Vec3::new(0., 0.28, 0.4),
                    Vec3::new(0.5, 0.82, 0.6),
                    t,
                    s,
                )
            }
            if c & 4 != 0 {
                add_box(
                    out,
                    p,
                    Vec3::new(0.4, 0.28, 0.),
                    Vec3::new(0.6, 0.82, 0.5),
                    t,
                    s,
                )
            }
            if c & 8 != 0 {
                add_box(
                    out,
                    p,
                    Vec3::new(0.4, 0.28, 0.5),
                    Vec3::new(0.6, 0.82, 1.),
                    t,
                    s,
                )
            }
        }
        7 => add_box(out, p, Vec3::ZERO, Vec3::new(1., 0.08, 1.), t, s),
        8 => {
            let d = 0.12;
            if def.flags & 16 != 0 {
                match f {
                    0 => add_vertical_frame(out, p, true, 1. - d, t, s),
                    1 => add_vertical_frame(out, p, false, 0., t, s),
                    2 => add_vertical_frame(out, p, true, 0., t, s),
                    _ => add_vertical_frame(out, p, false, 1. - d, t, s),
                }
            } else {
                add_horizontal_frame(out, p, if def.flags & 32 != 0 { 1. - d } else { 0. }, t, s)
            }
        }
        9 => add_box(
            out,
            p,
            Vec3::new(0.3, 0., 0.3),
            Vec3::new(0.7, 0.55, 0.7),
            t,
            s,
        ),
        10 => {
            let post = 0.18;
            if matches!(f, 0 | 2) {
                add_box(
                    out,
                    p,
                    Vec3::new(0., 0., 0.42),
                    Vec3::new(post, 1., 0.58),
                    t,
                    s,
                );
                add_box(
                    out,
                    p,
                    Vec3::new(1. - post, 0., 0.42),
                    Vec3::new(1., 1., 0.58),
                    t,
                    s,
                );
                if def.flags & 16 != 0 {
                    let (z0, z1) = if f == 0 { (0.42, 1.) } else { (0., 0.58) };
                    for (x0, x1) in [(0., post), (1. - post, 1.)] {
                        add_box(
                            out,
                            p,
                            Vec3::new(x0, 0.24, z0),
                            Vec3::new(x1, 0.41, z1),
                            t,
                            s,
                        );
                        add_box(
                            out,
                            p,
                            Vec3::new(x0, 0.59, z0),
                            Vec3::new(x1, 0.76, z1),
                            t,
                            s,
                        );
                    }
                } else {
                    add_box(
                        out,
                        p,
                        Vec3::new(post, 0.24, 0.42),
                        Vec3::new(1. - post, 0.41, 0.58),
                        t,
                        s,
                    );
                    add_box(
                        out,
                        p,
                        Vec3::new(post, 0.59, 0.42),
                        Vec3::new(1. - post, 0.76, 0.58),
                        t,
                        s,
                    );
                }
            } else {
                add_box(
                    out,
                    p,
                    Vec3::new(0.42, 0., 0.),
                    Vec3::new(0.58, 1., post),
                    t,
                    s,
                );
                add_box(
                    out,
                    p,
                    Vec3::new(0.42, 0., 1. - post),
                    Vec3::new(0.58, 1., 1.),
                    t,
                    s,
                );
                if def.flags & 16 != 0 {
                    let (x0, x1) = if f == 1 { (0., 0.58) } else { (0.42, 1.) };
                    for (z0, z1) in [(0., post), (1. - post, 1.)] {
                        add_box(
                            out,
                            p,
                            Vec3::new(x0, 0.24, z0),
                            Vec3::new(x1, 0.41, z1),
                            t,
                            s,
                        );
                        add_box(
                            out,
                            p,
                            Vec3::new(x0, 0.59, z0),
                            Vec3::new(x1, 0.76, z1),
                            t,
                            s,
                        );
                    }
                } else {
                    add_box(
                        out,
                        p,
                        Vec3::new(0.42, 0.24, post),
                        Vec3::new(0.58, 0.41, 1. - post),
                        t,
                        s,
                    );
                    add_box(
                        out,
                        p,
                        Vec3::new(0.42, 0.59, post),
                        Vec3::new(0.58, 0.76, 1. - post),
                        t,
                        s,
                    );
                }
            }
        }
        _ => add_box(out, p, Vec3::ZERO, Vec3::ONE, t, s),
    }
}

impl Scene {
    pub fn load() -> Self {
        let textures = TEXTURE_PATHS.iter().map(|p| CpuTexture::load(p)).collect();
        let idol_sky = std::fs::read_to_string("assets/idol_sky/list.txt")
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| {
                Image::load_image(line.split('|').next().unwrap_or(line).trim()).ok()
            })
            .map(CpuTexture::from_image)
            .collect();
        let bytes = decode_base64(BLOCK_DATA_B64);
        let mut cursor = 0;
        let mut linear = 0usize;
        let (sx, _, sz) = MAP_SIZE;
        let mut boxes = Vec::with_capacity(BLOCK_COUNT * 2);
        for _ in 0..BLOCK_COUNT {
            let mut delta = 0usize;
            let mut shift = 0;
            loop {
                let b = bytes[cursor];
                cursor += 1;
                delta |= ((b & 127) as usize) << shift;
                if b & 128 == 0 {
                    break;
                }
                shift += 7
            }
            linear += delta;
            let state = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]) as usize;
            cursor += 2;
            let x = linear % sx;
            let z = (linear / sx) % sz;
            let y = linear / (sx * sz);
            let p = Vec3::new(
                x as f32 - sx as f32 * 0.5,
                y as f32,
                z as f32 - sz as f32 * 0.5,
            );
            add_block(&mut boxes, p, PALETTE[state]);
        }
        // A refractive ocean, represented by a single shallow rectangular cube.
        boxes.push(Cuboid {
            bounds: Aabb {
                min: Vec3::new(-50000., -2.4, -50000.),
                max: Vec3::new(50000., WATER_LEVEL, 50000.),
            },
            texture: 4,
            surface: WATER_SURFACE,
        });
        // Procedural 16x16 seabed: one large cubic cell per sample, fully covering the ocean.
        for z in 0..16 {
            for x in 0..16 {
                let h = hash01((x + z * 16) as u32) * 1.2;
                let p = Vec3::new(x as f32 * 14. - 112., -8. - h, z as f32 * 14. - 112.);
                add_box(&mut boxes, p, Vec3::ZERO, Vec3::new(14., 2. + h, 14.), 2, 8);
            }
        }
        add_box(
            &mut boxes,
            Vec3::new(-50000., -42., -50000.),
            Vec3::ZERO,
            Vec3::new(100000., 2., 100000.),
            2,
            8,
        );
        let cuboid_count = boxes.len();
        Self {
            bvh: Bvh::new(boxes),
            textures,
            idol_sky,
            center: Vec3::new(0., 21., 0.),
            block_count: BLOCK_COUNT,
            cuboid_count,
        }
    }
    pub fn texture(&self, id: u8, uv: (f32, f32)) -> Vec3 {
        self.textures[id as usize].sample(uv)
    }
    pub fn idol_count(&self) -> usize {
        self.idol_sky.len()
    }
    pub fn idol_sky(&self, direction: Vec3, index: usize) -> Option<Vec3> {
        if self.idol_sky.is_empty() {
            return None;
        }
        let image = &self.idol_sky[index % self.idol_sky.len()];
        // Repeat complete, correctly proportioned panels around all 360 degrees.
        // The number of panels adapts to each image aspect ratio, while the polar
        // cap remains procedural instead of stretching the image's top row.
        let aspect = image.width as f32 / image.height as f32;
        let desired_height = 0.20;
        let panel_count = (std::f32::consts::TAU / (desired_height * aspect))
            .round()
            .max(1.0);
        let panel_width = std::f32::consts::TAU / panel_count;
        let panel_height = panel_width / aspect;
        let around = (direction.z.atan2(direction.x) / std::f32::consts::TAU + 1.0).fract();
        let u = (around * panel_count).fract();
        let elevation = direction.y.clamp(-1.0, 1.0).asin();
        let v = 0.5 - (elevation - 0.10) / panel_height;
        if !(0.0..1.0).contains(&v) {
            return None;
        }
        Some(image.sample_normalized((u, v)))
    }
    pub fn normal_mapped(&self, id: u8, uv: (f32, f32), normal: Vec3, strength: f32) -> Vec3 {
        if strength <= 0. || id as usize >= self.textures.len() {
            return normal;
        }
        let e = 0.035;
        let tex = &self.textures[id as usize];
        let dx = tex.height((uv.0 + e, uv.1)) - tex.height((uv.0 - e, uv.1));
        let dy = tex.height((uv.0, uv.1 + e)) - tex.height((uv.0, uv.1 - e));
        let tangent = if normal.y.abs() > 0.8 {
            Vec3::new(1., 0., 0.)
        } else {
            Vec3::new(0., 1., 0.).cross(normal).normalized()
        };
        let bitangent = normal.cross(tangent);
        (normal + tangent * (dx * strength) + bitangent * (dy * strength)).normalized()
    }
}
