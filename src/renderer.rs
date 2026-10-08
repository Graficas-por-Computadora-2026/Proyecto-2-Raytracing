use std::f32::consts::PI;

use crate::geometry::{Hit, Ray};
use crate::math::{Vec3, hash01};
use crate::scene::{Scene, WATER_SURFACE, surface};

#[derive(Clone, Copy)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
    pub fov: f32,
}

impl Camera {
    pub fn position(self) -> Vec3 {
        let cp = self.pitch.cos();
        self.target
            + Vec3::new(self.yaw.sin() * cp, self.pitch.sin(), self.yaw.cos() * cp) * self.distance
    }
    pub fn ray(self, x: f32, y: f32, aspect: f32) -> Ray {
        let origin = self.position();
        let forward = (self.target - origin).normalized();
        let right = forward.cross(Vec3::new(0., 1., 0.)).normalized();
        let up = right.cross(forward);
        let scale = (self.fov * 0.5).tan();
        Ray::new(
            origin,
            (forward + right * (x * aspect * scale) + up * (y * scale)).normalized(),
        )
    }
}

fn refract(i: Vec3, n: Vec3, ior: f32) -> Option<Vec3> {
    let mut cosi = i.dot(n).clamp(-1., 1.);
    let (etai, etat, nn) = if cosi < 0. {
        cosi = -cosi;
        (1., ior, n)
    } else {
        (ior, 1., -n)
    };
    let eta = etai / etat;
    let k = 1. - eta * eta * (1. - cosi * cosi);
    if k < 0. {
        None
    } else {
        Some(i * eta + nn * (eta * cosi - k.sqrt()))
    }
}

fn sky(scene: &Scene, direction: Vec3, time: f32, idol_mode: bool, idol_index: usize) -> Vec3 {
    let angle = (time / 24.) * 2. * PI - PI * 0.5;
    let sun = Vec3::new(angle.cos(), angle.sin(), -0.28).normalized();
    // The moon follows the opposite point of the same celestial orbit.
    let moon = -sun;
    let daylight = (sun.y * 3.0).clamp(0., 1.);
    let twilight = (1.0 - (sun.y.abs() * 8.0).min(1.0)) * (1.0 - daylight);
    let horizon = Vec3::new(0.004, 0.008, 0.025).mix(Vec3::new(0.25, 0.72, 1.), daylight)
        + Vec3::new(0.34, 0.07, 0.025) * twilight;
    let zenith = Vec3::new(0.0015, 0.003, 0.014).mix(Vec3::new(0.08, 0.42, 0.82), daylight);
    let mut color = horizon.mix(zenith, direction.y.max(0.).sqrt());
    if idol_mode {
        if let Some(gallery) = scene.idol_sky(direction, idol_index) {
            let adapted = gallery * (0.12 + daylight * 0.88);
            color = color.mix(adapted, 0.94);
        }
    }
    let disk = direction.dot(sun);
    if sun.y > -0.08 && disk > 0.997 {
        color +=
            Vec3::new(1., 0.72, 0.32) * ((disk - 0.997) / 0.003).powi(2) * (1.5 + daylight * 2.)
    }
    let moon_disk = direction.dot(moon);
    if moon.y > 0.0 && moon_disk > 0.999 {
        let core = ((moon_disk - 0.999) / 0.001).clamp(0., 1.).powi(2);
        color += Vec3::new(0.72, 0.82, 1.0) * (core * 2.8);
    } else if moon.y > 0.0 && moon_disk > 0.993 {
        color += Vec3::new(0.16, 0.22, 0.38) * ((moon_disk - 0.993) / 0.006).powi(3);
    }
    if daylight < 0.2 && direction.y > 0. {
        let q = ((direction.x * 731.3).floor() as i32 as u32)
            ^ ((direction.y * 917.7).floor() as i32 as u32).rotate_left(11)
            ^ ((direction.z * 613.1).floor() as i32 as u32).rotate_left(22);
        if hash01(q) > 0.998 {
            color += Vec3::splat((0.22 - daylight).max(0.0) * 4.2)
        }
    }
    color
}

fn sun_data(time: f32) -> (Vec3, Vec3, f32) {
    let a = (time / 24.) * 2. * PI - PI * 0.5;
    let dir = Vec3::new(a.cos(), a.sin(), -0.28).normalized();
    let daylight = (dir.y * 3.0).clamp(0., 1.);
    if dir.y > 0.0 {
        (
            dir,
            Vec3::new(1., 0.86, 0.65).mix(Vec3::new(1., 0.98, 0.92), daylight),
            daylight,
        )
    } else {
        let moon = -dir;
        (moon, Vec3::new(0.30, 0.38, 0.58), daylight)
    }
}

fn water_texture(p: Vec3) -> Vec3 {
    let wave = (p.x * 0.23 + (p.z * 0.17).sin()).sin() * 0.5 + 0.5;
    Vec3::new(0.015, 0.22, 0.34).mix(Vec3::new(0.03, 0.48, 0.62), wave * 0.3)
}

fn direct_light(
    scene: &Scene,
    hit: Hit,
    normal: Vec3,
    view: Vec3,
    time: f32,
    albedo: Vec3,
) -> Vec3 {
    let (sun_dir, sun_color, day) = sun_data(time);
    let mut light = Vec3::new(0.025, 0.035, 0.065).mix(Vec3::new(0.36, 0.40, 0.42), day);
    let shadow_origin = hit.point + normal * 0.003;
    let blocked = scene
        .bvh
        .intersect(Ray::new(shadow_origin, sun_dir), 500., hit.primitive)
        .map(|h| surface(h.surface).transparency < 0.5)
        .unwrap_or(false);
    if !blocked {
        let diffuse = normal.dot(sun_dir).max(0.);
        let halfway = (sun_dir + view).normalized();
        let mat = surface(hit.surface);
        let shininess = (2. / (mat.roughness * mat.roughness).max(0.002)).min(512.);
        let spec = normal.dot(halfway).max(0.).powf(shininess) * mat.specular;
        light += sun_color * (diffuse + spec);
    }
    // Warm porch illumination makes the emissive feature visibly affect nearby blocks at night.
    let lamp = Vec3::new(5., 17., -12.);
    let delta = lamp - hit.point;
    let distance = delta.length();
    if distance < 28. {
        let l = delta / distance;
        let blocked =
            scene
                .bvh
                .occluded(Ray::new(shadow_origin, l), distance - 0.02, hit.primitive);
        if !blocked {
            let strength = (1. - distance / 28.).powi(2) * (1. - day) * 5.;
            light += Vec3::new(1., 0.38, 0.08) * (normal.dot(l).max(0.) * strength);
        }
    }
    albedo * light
}

fn trace(
    scene: &Scene,
    ray: Ray,
    time: f32,
    idol_mode: bool,
    idol_index: usize,
    depth: u8,
    max_depth: u8,
    ignore: usize,
) -> Vec3 {
    if depth >= max_depth {
        return sky(scene, ray.direction, time, idol_mode, idol_index);
    }
    let Some(hit) = scene.bvh.intersect(ray, 100000., ignore) else {
        return sky(scene, ray.direction, time, idol_mode, idol_index);
    };
    let mat = surface(hit.surface);
    let mut base = if hit.surface == WATER_SURFACE {
        water_texture(hit.point)
    } else {
        scene.texture(hit.texture, hit.uv)
    };
    if hit.surface == 5 {
        base = base * Vec3::new(0.48, 1.16, 0.34)
    }
    if hit.surface != WATER_SURFACE {
        let cube_edge = 0.74 + 0.26 * (hit.edge / 0.065).clamp(0., 1.);
        base = base * cube_edge;
    }
    base = base * mat.albedo;
    let normal = if hit.surface == WATER_SURFACE {
        let wave = Vec3::new(
            (hit.point.z * 0.19).cos() * 0.08,
            1.,
            (hit.point.x * 0.23).sin() * 0.08,
        )
        .normalized();
        if hit.normal.y.abs() > 0.8 {
            wave * hit.normal.y
        } else {
            hit.normal
        }
    } else {
        scene.normal_mapped(hit.texture, hit.uv, hit.normal, mat.normal_strength)
    };
    let view = -ray.direction;
    let mut color = direct_light(scene, hit, normal, view, time, base) + base * mat.emission;
    let facing = (-ray.direction.dot(normal)).clamp(0., 1.);
    let fresnel = (1. - facing).powi(5);
    if mat.reflectivity > 0. {
        let reflected = trace(
            scene,
            Ray::new(hit.point + normal * 0.004, ray.direction.reflect(normal)),
            time,
            idol_mode,
            idol_index,
            depth + 1,
            max_depth,
            hit.primitive,
        );
        color = color.mix(reflected, (mat.reflectivity + fresnel * 0.35).min(0.85));
    }
    if mat.transparency > 0. {
        if let Some(dir) = refract(ray.direction, normal, mat.ior) {
            let transmitted = trace(
                scene,
                Ray::new(hit.point - normal * 0.006, dir),
                time,
                idol_mode,
                idol_index,
                depth + 1,
                max_depth,
                hit.primitive,
            );
            color = color.mix(transmitted, mat.transparency * (1. - fresnel * 0.7));
        }
    }
    let fog = ((hit.distance - 190.) / 420.).clamp(0., 0.38);
    color.mix(sky(scene, ray.direction, time, idol_mode, idol_index), fog)
}

pub fn render(
    scene: &Scene,
    camera: Camera,
    time: f32,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    pass: u8,
    mask_bits: u8,
    max_depth: u8,
    idol_mode: bool,
    idol_index: usize,
) {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(height);
    let rows = (height + threads - 1) / threads;
    let aspect = width as f32 / height as f32;
    let mask = (1u32 << mask_bits) - 1;
    std::thread::scope(|scope| {
        for (chunk_index, chunk) in pixels.chunks_mut(rows * width * 4).enumerate() {
            let first_y = chunk_index * rows;
            scope.spawn(move || {
                for (local_y, row) in chunk.chunks_mut(width * 4).enumerate() {
                    let y = first_y + local_y;
                    if y >= height {
                        break;
                    }
                    for x in 0..width {
                        let slot = if mask_bits == 3 {
                            (((x & 3) << 1) | ((y ^ (x >> 2)) & 1)) as u32
                        } else {
                            ((x & 1) | ((y & 1) << 1)) as u32
                        };
                        if slot & mask != pass as u32 {
                            continue;
                        }
                        let sx = ((x as f32 + 0.5) / width as f32) * 2. - 1.;
                        let sy = 1. - ((y as f32 + 0.5) / height as f32) * 2.;
                        let c = trace(
                            scene,
                            camera.ray(sx, sy, aspect),
                            time,
                            idol_mode,
                            idol_index,
                            0,
                            max_depth,
                            usize::MAX,
                        )
                        .clamp01();
                        let exposure = 1.12;
                        let mapped = Vec3::new(
                            1. - (-c.x * exposure).exp(),
                            1. - (-c.y * exposure).exp(),
                            1. - (-c.z * exposure).exp(),
                        );
                        let p = &mut row[x * 4..x * 4 + 4];
                        p[0] = (mapped.x.powf(1. / 2.2) * 255.) as u8;
                        p[1] = (mapped.y.powf(1. / 2.2) * 255.) as u8;
                        p[2] = (mapped.z.powf(1. / 2.2) * 255.) as u8;
                        p[3] = 255;
                    }
                }
            });
        }
    });
}
