use crate::math::Vec3;

#[derive(Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub inv_direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        let direction = direction.normalized();
        Self {
            origin,
            direction,
            inv_direction: Vec3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z),
        }
    }
    pub fn at(self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }
    pub fn union(self, rhs: Self) -> Self {
        Self {
            min: self.min.min(rhs.min),
            max: self.max.max(rhs.max),
        }
    }
    pub fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    pub fn extent(self) -> Vec3 {
        self.max - self.min
    }
    #[inline]
    pub fn hit(self, ray: Ray, max_t: f32) -> bool {
        let tx1 = (self.min.x - ray.origin.x) * ray.inv_direction.x;
        let tx2 = (self.max.x - ray.origin.x) * ray.inv_direction.x;
        let ty1 = (self.min.y - ray.origin.y) * ray.inv_direction.y;
        let ty2 = (self.max.y - ray.origin.y) * ray.inv_direction.y;
        let tz1 = (self.min.z - ray.origin.z) * ray.inv_direction.z;
        let tz2 = (self.max.z - ray.origin.z) * ray.inv_direction.z;
        let near = tx1.min(tx2).max(ty1.min(ty2)).max(tz1.min(tz2)).max(0.0001);
        let far = tx1.max(tx2).min(ty1.max(ty2)).min(tz1.max(tz2)).min(max_t);
        near <= far
    }
}

#[derive(Clone, Copy)]
pub struct Cuboid {
    pub bounds: Aabb,
    pub texture: u8,
    pub surface: u8,
}

#[derive(Clone, Copy)]
pub struct Hit {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub uv: (f32, f32),
    pub edge: f32,
    pub primitive: usize,
    pub texture: u8,
    pub surface: u8,
}

impl Cuboid {
    pub fn intersect(self, ray: Ray, max_t: f32, primitive: usize) -> Option<Hit> {
        let tx1 = (self.bounds.min.x - ray.origin.x) * ray.inv_direction.x;
        let tx2 = (self.bounds.max.x - ray.origin.x) * ray.inv_direction.x;
        let ty1 = (self.bounds.min.y - ray.origin.y) * ray.inv_direction.y;
        let ty2 = (self.bounds.max.y - ray.origin.y) * ray.inv_direction.y;
        let tz1 = (self.bounds.min.z - ray.origin.z) * ray.inv_direction.z;
        let tz2 = (self.bounds.max.z - ray.origin.z) * ray.inv_direction.z;
        let near = tx1.min(tx2).max(ty1.min(ty2)).max(tz1.min(tz2));
        let far = tx1.max(tx2).min(ty1.max(ty2)).min(tz1.max(tz2));
        let t = if near > 0.0002 { near } else { far };
        if near > far || t <= 0.0002 || t >= max_t {
            return None;
        }
        let p = ray.at(t);
        let eps = 0.002;
        let (normal, uv) = if (p.x - self.bounds.min.x).abs() < eps {
            (Vec3::new(-1., 0., 0.), (p.z, p.y))
        } else if (p.x - self.bounds.max.x).abs() < eps {
            (Vec3::new(1., 0., 0.), (p.z, p.y))
        } else if (p.y - self.bounds.min.y).abs() < eps {
            (Vec3::new(0., -1., 0.), (p.x, p.z))
        } else if (p.y - self.bounds.max.y).abs() < eps {
            (Vec3::new(0., 1., 0.), (p.x, p.z))
        } else if (p.z - self.bounds.min.z).abs() < eps {
            (Vec3::new(0., 0., -1.), (p.x, p.y))
        } else {
            (Vec3::new(0., 0., 1.), (p.x, p.y))
        };
        let size = self.bounds.max - self.bounds.min;
        let local_uv = if normal.x.abs() > 0.5 {
            (
                (p.z - self.bounds.min.z) / size.z,
                (p.y - self.bounds.min.y) / size.y,
            )
        } else if normal.y.abs() > 0.5 {
            (
                (p.x - self.bounds.min.x) / size.x,
                (p.z - self.bounds.min.z) / size.z,
            )
        } else {
            (
                (p.x - self.bounds.min.x) / size.x,
                (p.y - self.bounds.min.y) / size.y,
            )
        };
        let edge = local_uv
            .0
            .min(1.0 - local_uv.0)
            .min(local_uv.1.min(1.0 - local_uv.1));
        Some(Hit {
            distance: t,
            point: p,
            normal,
            uv,
            edge,
            primitive,
            texture: self.texture,
            surface: self.surface,
        })
    }
}

#[derive(Clone, Copy)]
struct BvhNode {
    bounds: Aabb,
    start: u32,
    count: u16,
    right: u32,
}

pub struct Bvh {
    pub cuboids: Vec<Cuboid>,
    indices: Vec<usize>,
    nodes: Vec<BvhNode>,
}

impl Bvh {
    pub fn new(cuboids: Vec<Cuboid>) -> Self {
        let mut result = Self {
            indices: (0..cuboids.len()).collect(),
            cuboids,
            nodes: Vec::new(),
        };
        if !result.indices.is_empty() {
            result.build_node(0, result.indices.len());
        }
        result
    }
    fn build_node(&mut self, start: usize, end: usize) -> u32 {
        let node_index = self.nodes.len() as u32;
        let mut bounds = Aabb::empty();
        for &i in &self.indices[start..end] {
            bounds = bounds.union(self.cuboids[i].bounds);
        }
        self.nodes.push(BvhNode {
            bounds,
            start: start as u32,
            count: 0,
            right: 0,
        });
        if end - start <= 6 {
            self.nodes[node_index as usize].count = (end - start) as u16;
            return node_index;
        }
        let e = bounds.extent();
        let axis = if e.x > e.y && e.x > e.z {
            0
        } else if e.y > e.z {
            1
        } else {
            2
        };
        self.indices[start..end].sort_unstable_by(|&a, &b| {
            self.cuboids[a]
                .bounds
                .center()
                .component(axis)
                .total_cmp(&self.cuboids[b].bounds.center().component(axis))
        });
        let middle = (start + end) / 2;
        self.build_node(start, middle);
        let right = self.build_node(middle, end);
        self.nodes[node_index as usize].right = right;
        node_index
    }
    pub fn intersect(&self, ray: Ray, max_t: f32, ignore: usize) -> Option<Hit> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut stack = [0u32; 64];
        let mut top = 1usize;
        let mut closest = max_t;
        let mut result = None;
        while top > 0 {
            top -= 1;
            let ni = stack[top] as usize;
            let node = self.nodes[ni];
            if !node.bounds.hit(ray, closest) {
                continue;
            }
            if node.count > 0 {
                for j in node.start as usize..node.start as usize + node.count as usize {
                    let pi = self.indices[j];
                    if pi != ignore {
                        if let Some(h) = self.cuboids[pi].intersect(ray, closest, pi) {
                            closest = h.distance;
                            result = Some(h);
                        }
                    }
                }
            } else {
                if top + 2 <= 64 {
                    stack[top] = ni as u32 + 1;
                    stack[top + 1] = node.right;
                    top += 2;
                }
            }
        }
        result
    }
    pub fn occluded(&self, ray: Ray, max_t: f32, ignore: usize) -> bool {
        self.intersect(ray, max_t, ignore).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(min: Vec3, max: Vec3) -> Cuboid {
        Cuboid {
            bounds: Aabb { min, max },
            texture: 0,
            surface: 1,
        }
    }

    #[test]
    fn cube_intersection_returns_front_face() {
        let hit = cube(Vec3::ZERO, Vec3::ONE)
            .intersect(
                Ray::new(Vec3::new(0.5, 0.5, 3.0), Vec3::new(0.0, 0.0, -1.0)),
                10.0,
                0,
            )
            .unwrap();
        assert!((hit.distance - 2.0).abs() < 0.001);
        assert!(hit.normal.z > 0.9);
    }

    #[test]
    fn bvh_selects_nearest_cube_and_honors_ignore() {
        let bvh = Bvh::new(vec![
            cube(Vec3::new(0.0, 0.0, 0.0), Vec3::ONE),
            cube(Vec3::new(0.0, 0.0, -3.0), Vec3::new(1.0, 1.0, -2.0)),
        ]);
        let ray = Ray::new(Vec3::new(0.5, 0.5, 3.0), Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(bvh.intersect(ray, 20.0, usize::MAX).unwrap().primitive, 0);
        assert_eq!(bvh.intersect(ray, 20.0, 0).unwrap().primitive, 1);
    }
}
