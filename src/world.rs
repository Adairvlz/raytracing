use crate::material::MaterialId;
use crate::math::{value_noise, Vec3};

pub const SIZE_X: i32 = 46;
pub const SIZE_Y: i32 = 16;
pub const SIZE_Z: i32 = 24;
pub struct World {
    cells: Vec<Option<MaterialId>>,
}
impl World {
    pub fn new() -> Self {
        let mut w = Self {
            cells: vec![None; (SIZE_X * SIZE_Y * SIZE_Z) as usize],
        };
        w.generate();
        w
    }
    fn index(x: i32, y: i32, z: i32) -> usize {
        ((y * SIZE_Z + z) * SIZE_X + x) as usize
    }
    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<MaterialId> {
        if x < 0 || y < 0 || z < 0 || x >= SIZE_X || y >= SIZE_Y || z >= SIZE_Z {
            None
        } else {
            self.cells[Self::index(x, y, z)]
        }
    }
    fn set(&mut self, x: i32, y: i32, z: i32, m: MaterialId) {
        if x >= 0 && y >= 0 && z >= 0 && x < SIZE_X && y < SIZE_Y && z < SIZE_Z {
            let i = Self::index(x, y, z);
            self.cells[i] = Some(m)
        }
    }
    fn generate(&mut self) {
        use MaterialId::*;
        for z in 0..SIZE_Z {
            for x in 0..24 {
                let broad = value_noise(x as f32 * 0.13, z as f32 * 0.13);
                let detail = value_noise(x as f32 * 0.37 + 20., z as f32 * 0.37);
                let h = (3.0 + broad * 4.0 + detail * 1.7) as i32;
                for y in 0..=h {
                    let m = if y == h {
                        if h >= 7 {
                            Snow
                        } else if x < 5 {
                            Sand
                        } else {
                            Grass
                        }
                    } else if y > h - 3 {
                        Dirt
                    } else {
                        Stone
                    };
                    self.set(x, y, z, m);
                }
                if (7..=13).contains(&x) && (7..=13).contains(&z) {
                    for y in 4..=h {
                        self.cells[Self::index(x, y, z)] = None;
                    }
                    self.set(x, 4, z, Water);
                }
            }
        }
        // Glass observation arch makes refraction deliberately visible beside the lake.
        for x in 15..20 {
            for y in 5..9 {
                if x == 15 || x == 19 || y == 8 {
                    self.set(x, y, 15, Glass)
                }
            }
        }
        // Cabin with warm emissive windows.
        for x in 3..9 {
            for z in 15..21 {
                for y in 5..9 {
                    let edge = x == 3 || x == 8 || z == 15 || z == 20;
                    if edge {
                        self.set(
                            x,
                            y,
                            z,
                            if y == 7 && (z == 15 || z == 20) && (x == 5 || x == 6) {
                                Glowstone
                            } else {
                                Wood
                            },
                        );
                    }
                }
            }
        }
        for x in 2..10 {
            for z in 14..22 {
                self.set(x, 9, z, Wood)
            }
        }
        // Tree.
        for y in 5..11 {
            self.set(18, y, 5, Wood)
        }
        for x in 16_i32..=20 {
            for z in 3_i32..=7 {
                for y in 9_i32..=12 {
                    if (x - 18).abs() + (z - 5).abs() + (y - 10).abs() < 5 {
                        self.set(x, y, z, Leaves)
                    }
                }
            }
        }
        // Lava fissure: emissive and reflected in nearby water/glass.
        for z in 3..10 {
            self.set(11, 5, z, Lava);
            if z % 2 == 0 {
                self.set(12, 5, z, Lava)
            }
        }

        // Second floating island: a hostile volcanic Nether biome.
        let nether_cx: i32 = 35;
        let nether_cz: i32 = 12;
        for z in 3..22 {
            for x in 26..45 {
                let dx = x - nether_cx;
                let dz = z - nether_cz;
                if dx * dx + dz * dz > 88 {
                    continue;
                }
                let rough = value_noise(x as f32 * 0.31 + 70.0, z as f32 * 0.31 + 40.0);
                let h = 5 + (rough * 2.8) as i32;
                let bottom = (h - 4 - ((9 - dx.abs().max(dz.abs())).max(0) / 3)).max(1);
                for y in bottom..=h {
                    let material = if y == h {
                        if (x * 3 + z * 5) % 11 == 0 {
                            Magma
                        } else if (x + z) % 7 == 0 {
                            SoulSand
                        } else {
                            Netherrack
                        }
                    } else if y <= bottom + 1 {
                        Blackstone
                    } else {
                        Netherrack
                    };
                    self.set(x, y, z, material);
                }
            }
        }

        // A lava fall over the island edge provides a strong emissive landmark.
        for y in 1..8 {
            self.set(43, y, 12, Lava);
            self.set(42, y, 12, Magma);
        }

        // Obsidian portal: five blocks wide, six high, with an emissive purple interior.
        for x in 33..=37 {
            for y in 6..=11 {
                if x == 33 || x == 37 || y == 6 || y == 11 {
                    self.set(x, y, 12, Obsidian);
                } else {
                    self.set(x, y, 12, NetherPortal);
                }
            }
        }

        // Blackstone pillars and stepping stones visually connect both islands.
        for &(x, z, height) in &[(29, 17, 11), (41, 6, 10), (39, 19, 9)] {
            for y in 6..=height {
                self.set(x, y, z, Blackstone);
            }
            self.set(x, height + 1, z, Magma);
        }
        for x in (23..29).step_by(2) {
            self.set(x, 5, 12, Obsidian);
        }
    }
    pub fn center(&self) -> Vec3 {
        Vec3::new(SIZE_X as f32 * 0.5, 5., SIZE_Z as f32 * 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terrain_is_generated_and_ray_hits_it() {
        let world = World::new();
        assert!(world.get(0, 0, 0).is_some());
        let hit = intersect(
            &world,
            Vec3::new(12.5, 15.5, 12.5),
            Vec3::new(0.0, -1.0, 0.0),
            30.0,
        );
        assert!(hit.is_some());
    }
}

#[derive(Clone, Copy)]
pub struct Hit {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub material: MaterialId,
}

pub fn intersect(world: &World, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<Hit> {
    // Amanatides & Woo voxel traversal: visits only crossed cells rather than every cube.
    let mut x = origin.x.floor() as i32;
    let mut y = origin.y.floor() as i32;
    let mut z = origin.z.floor() as i32;
    let sx = if dir.x >= 0. { 1 } else { -1 };
    let sy = if dir.y >= 0. { 1 } else { -1 };
    let sz = if dir.z >= 0. { 1 } else { -1 };
    let delta = Vec3::new(
        if dir.x.abs() < 1e-7 {
            f32::INFINITY
        } else {
            1. / dir.x.abs()
        },
        if dir.y.abs() < 1e-7 {
            f32::INFINITY
        } else {
            1. / dir.y.abs()
        },
        if dir.z.abs() < 1e-7 {
            f32::INFINITY
        } else {
            1. / dir.z.abs()
        },
    );
    let mut side = Vec3::new(
        if sx > 0 {
            (x + 1) as f32 - origin.x
        } else {
            origin.x - x as f32
        },
        if sy > 0 {
            (y + 1) as f32 - origin.y
        } else {
            origin.y - y as f32
        },
        if sz > 0 {
            (z + 1) as f32 - origin.z
        } else {
            origin.z - z as f32
        },
    );
    side = side.hadamard(delta);
    let mut t = 0.;
    let mut normal = Vec3::new(0., 1., 0.);
    for _ in 0..256 {
        if t > max_dist {
            return None;
        }
        if let Some(m) = world.get(x, y, z) {
            return Some(Hit {
                t,
                point: origin + dir * t,
                normal,
                material: m,
            });
        }
        if side.x < side.y && side.x < side.z {
            t = side.x;
            side.x += delta.x;
            x += sx;
            normal = Vec3::new(-sx as f32, 0., 0.)
        } else if side.y < side.z {
            t = side.y;
            side.y += delta.y;
            y += sy;
            normal = Vec3::new(0., -sy as f32, 0.)
        } else {
            t = side.z;
            side.z += delta.z;
            z += sz;
            normal = Vec3::new(0., 0., -sz as f32)
        }
    }
    None
}
