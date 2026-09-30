use crate::math::{noise3, Color, Vec3, WHITE};
use raylib::prelude::Image;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialId {
    Grass,
    Dirt,
    Stone,
    Wood,
    Leaves,
    Water,
    Glass,
    Lava,
    Glowstone,
    Snow,
    Sand,
    Netherrack,
    Obsidian,
    NetherPortal,
    Magma,
    SoulSand,
    Blackstone,
}

#[derive(Clone, Copy)]
pub struct Material {
    pub albedo: Color,
    pub specular: f32,
    pub shininess: f32,
    pub reflectivity: f32,
    pub transparency: f32,
    pub ior: f32,
    pub emission: Color,
    pub normal_strength: f32,
}

#[derive(Clone)]
struct CpuTexture {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
}

impl CpuTexture {
    fn load(path: &Path) -> Option<Self> {
        let image = Image::load_image(path.to_str()?).ok()?;
        let width = image.width() as usize;
        let height = image.height() as usize;
        let pixels = image
            .get_image_data()
            .iter()
            .map(|c| Vec3::new(c.r as f32 / 255., c.g as f32 / 255., c.b as f32 / 255.))
            .collect();
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    fn sample(&self, u: f32, v: f32) -> Color {
        let u = u.rem_euclid(1.0);
        let v = v.rem_euclid(1.0);
        let x = (u * self.width as f32) as usize % self.width;
        let y = ((1.0 - v) * self.height as f32) as usize % self.height;
        self.pixels[y * self.width + x]
    }
}

pub struct TextureLibrary {
    maps: Vec<Option<CpuTexture>>,
    normal_maps: Vec<Option<CpuTexture>>,
    skybox: [Option<CpuTexture>; 6],
    grass_top: Option<CpuTexture>,
    wood_top: Option<CpuTexture>,
    loaded: usize,
}

impl TextureLibrary {
    pub fn load(folder: impl AsRef<Path>) -> Self {
        use MaterialId::*;
        let folder = folder.as_ref();
        let mut maps = vec![None; 17];
        let mut normal_maps = vec![None; 17];
        let files = [
            (Grass, "grass_side.png"),
            (Dirt, "dirt.png"),
            (Stone, "stone.png"),
            (Wood, "wood_side.png"),
            (Leaves, "leaves.png"),
            (Water, "water.png"),
            (Glass, "glass.png"),
            (Lava, "lava.png"),
            (Glowstone, "glowstone.png"),
            (Snow, "snow.png"),
            (Sand, "sand.png"),
            (Netherrack, "netherrack.png"),
            (Obsidian, "obsidian.png"),
            (NetherPortal, "nether_portal.png"),
            (Magma, "magma.png"),
            (SoulSand, "soul_sand.png"),
            (Blackstone, "blackstone.png"),
        ];
        let mut loaded = 0;
        for (id, name) in files {
            let image = CpuTexture::load(&folder.join(name));
            loaded += usize::from(image.is_some());
            maps[id as usize] = image;
        }
        let grass_top = CpuTexture::load(&folder.join("grass_top.png"));
        loaded += usize::from(grass_top.is_some());
        let wood_top = CpuTexture::load(&folder.join("wood_top.png"))
            .or_else(|| CpuTexture::load(&folder.join("wood_top.webp")));
        loaded += usize::from(wood_top.is_some());
        for (id, name) in [
            (Stone, "stone_normal.png"),
            (Water, "water_normal.png"),
            (Netherrack, "netherrack_normal.png"),
            (Obsidian, "obsidian_normal.png"),
        ] {
            normal_maps[id as usize] = CpuTexture::load(&folder.join(name));
        }
        let sky_folder = folder.parent().unwrap_or(folder).join("skybox");
        let sky_names = [
            "sky_px.png",
            "sky_nx.png",
            "sky_py.png",
            "sky_ny.png",
            "sky_pz.png",
            "sky_nz.png",
        ];
        let skybox =
            std::array::from_fn(|index| CpuTexture::load(&sky_folder.join(sky_names[index])));
        Self {
            maps,
            normal_maps,
            skybox,
            grass_top,
            wood_top,
            loaded,
        }
    }

    pub fn loaded_count(&self) -> usize {
        self.loaded
    }

    pub fn normal_map_count(&self) -> usize {
        self.normal_maps.iter().filter(|map| map.is_some()).count()
    }

    pub fn skybox_face_count(&self) -> usize {
        self.skybox.iter().filter(|face| face.is_some()).count()
    }

    pub fn sample(&self, id: MaterialId, p: Vec3, n: Vec3) -> Color {
        let selected = match id {
            MaterialId::Grass if n.y > 0.5 => self.grass_top.as_ref(),
            MaterialId::Wood if n.y.abs() > 0.5 => self.wood_top.as_ref(),
            _ => self.maps[id as usize].as_ref(),
        };
        let (u, v) = if n.x.abs() > 0.5 {
            (p.z, p.y)
        } else if n.y.abs() > 0.5 {
            (p.x, p.z)
        } else {
            (p.x, p.y)
        };
        selected.map_or_else(
            || procedural_texture(id, p, n),
            |texture| texture.sample(u, v),
        )
    }

    pub fn sample_normal(&self, id: MaterialId, p: Vec3, n: Vec3) -> Option<Color> {
        let texture = self.normal_maps[id as usize].as_ref()?;
        let (u, v) = face_uv(p, n);
        Some(texture.sample(u, v))
    }

    pub fn sample_sky(&self, direction: Vec3) -> Option<Color> {
        let ax = direction.x.abs();
        let ay = direction.y.abs();
        let az = direction.z.abs();
        let (face, u, v) = if ax >= ay && ax >= az {
            if direction.x > 0.0 {
                (0, -direction.z / ax, -direction.y / ax)
            } else {
                (1, direction.z / ax, -direction.y / ax)
            }
        } else if ay >= az {
            if direction.y > 0.0 {
                (2, direction.x / ay, direction.z / ay)
            } else {
                (3, direction.x / ay, -direction.z / ay)
            }
        } else if direction.z > 0.0 {
            (4, direction.x / az, -direction.y / az)
        } else {
            (5, -direction.x / az, -direction.y / az)
        };
        self.skybox[face]
            .as_ref()
            .map(|texture| texture.sample((u + 1.0) * 0.5, (1.0 - v) * 0.5))
    }
}

fn face_uv(p: Vec3, n: Vec3) -> (f32, f32) {
    if n.x.abs() > 0.5 {
        (p.z, p.y)
    } else if n.y.abs() > 0.5 {
        (p.x, p.z)
    } else {
        (p.x, p.y)
    }
}

pub fn properties(id: MaterialId) -> Material {
    use MaterialId::*;
    match id {
        Grass => Material {
            albedo: Vec3::new(0.25, 0.65, 0.16),
            specular: 0.08,
            shininess: 16.,
            reflectivity: 0.02,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.28,
        },
        Dirt => Material {
            albedo: Vec3::new(0.43, 0.25, 0.10),
            specular: 0.03,
            shininess: 8.,
            reflectivity: 0.,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.35,
        },
        Stone => Material {
            albedo: Vec3::new(0.48, 0.50, 0.52),
            specular: 0.18,
            shininess: 30.,
            reflectivity: 0.08,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.45,
        },
        Wood => Material {
            albedo: Vec3::new(0.48, 0.28, 0.10),
            specular: 0.08,
            shininess: 12.,
            reflectivity: 0.02,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.25,
        },
        Leaves => Material {
            albedo: Vec3::new(0.12, 0.48, 0.16),
            specular: 0.08,
            shininess: 16.,
            reflectivity: 0.01,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.4,
        },
        Water => Material {
            albedo: Vec3::new(0.08, 0.32, 0.55),
            specular: 0.9,
            shininess: 110.,
            reflectivity: 0.25,
            transparency: 0.68,
            ior: 1.333,
            emission: Vec3::default(),
            normal_strength: 0.55,
        },
        Glass => Material {
            albedo: Vec3::new(0.55, 0.82, 0.88),
            specular: 1.,
            shininess: 160.,
            reflectivity: 0.18,
            transparency: 0.78,
            ior: 1.5,
            emission: Vec3::default(),
            normal_strength: 0.04,
        },
        Lava => Material {
            albedo: Vec3::new(0.95, 0.16, 0.015),
            specular: 0.15,
            shininess: 20.,
            reflectivity: 0.04,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::new(3.5, 0.55, 0.03),
            normal_strength: 0.3,
        },
        Glowstone => Material {
            albedo: Vec3::new(0.9, 0.58, 0.18),
            specular: 0.1,
            shininess: 12.,
            reflectivity: 0.02,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::new(2.4, 1.5, 0.35),
            normal_strength: 0.25,
        },
        Snow => Material {
            albedo: Vec3::new(0.88, 0.94, 1.),
            specular: 0.28,
            shininess: 55.,
            reflectivity: 0.09,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.2,
        },
        Sand => Material {
            albedo: Vec3::new(0.78, 0.66, 0.38),
            specular: 0.04,
            shininess: 9.,
            reflectivity: 0.,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.3,
        },
        Netherrack => Material {
            albedo: Vec3::new(0.48, 0.08, 0.09),
            specular: 0.04,
            shininess: 8.,
            reflectivity: 0.,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.42,
        },
        Obsidian => Material {
            albedo: Vec3::new(0.045, 0.025, 0.08),
            specular: 0.65,
            shininess: 100.,
            reflectivity: 0.28,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.16,
        },
        NetherPortal => Material {
            albedo: Vec3::new(0.34, 0.03, 0.72),
            specular: 0.8,
            shininess: 90.,
            reflectivity: 0.12,
            transparency: 0.22,
            ior: 1.18,
            emission: Vec3::new(0.8, 0.08, 2.7),
            normal_strength: 0.2,
        },
        Magma => Material {
            albedo: Vec3::new(0.72, 0.15, 0.025),
            specular: 0.12,
            shininess: 18.,
            reflectivity: 0.03,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::new(2.5, 0.38, 0.025),
            normal_strength: 0.4,
        },
        SoulSand => Material {
            albedo: Vec3::new(0.29, 0.21, 0.17),
            specular: 0.02,
            shininess: 6.,
            reflectivity: 0.,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.5,
        },
        Blackstone => Material {
            albedo: Vec3::new(0.09, 0.09, 0.12),
            specular: 0.2,
            shininess: 32.,
            reflectivity: 0.09,
            transparency: 0.,
            ior: 1.,
            emission: Vec3::default(),
            normal_strength: 0.38,
        },
    }
}

fn procedural_texture(id: MaterialId, p: Vec3, n: Vec3) -> Color {
    use MaterialId::*;
    let u = if n.x.abs() > 0.5 { p.z } else { p.x };
    let v = if n.y.abs() > 0.5 { p.z } else { p.y };
    let cell = noise3(
        (u * 13.).floor() as i32,
        (v * 13.).floor() as i32,
        id as i32,
    );
    let grain = 0.82 + 0.3 * cell;
    match id {
        Grass => {
            if n.y > 0.5 {
                Vec3::new(0.22, 0.63, 0.13) * grain
            } else {
                let turf = if p.y.fract() > 0.74 {
                    Vec3::new(0.2, 0.57, 0.12)
                } else {
                    Vec3::new(0.44, 0.27, 0.11)
                };
                turf * grain
            }
        }
        Dirt => Vec3::new(0.45, 0.27, 0.12) * grain,
        Stone => Vec3::new(0.48, 0.50, 0.53) * (0.72 + 0.45 * cell),
        Wood => {
            let rings = ((u * 9. + cell * 2.).sin().abs()) * 0.22 + 0.78;
            Vec3::new(0.52, 0.3, 0.11) * rings
        }
        Leaves => Vec3::new(0.1, 0.45, 0.12) * (0.7 + 0.55 * cell),
        Water => Vec3::new(0.06, 0.30, 0.53) * (1. + 0.12 * (u * 8. + v * 5.).sin()),
        Glass => Vec3::new(0.65, 0.88, 0.92),
        Lava => {
            let vein = ((u * 5.).sin() * (v * 6.).cos()).abs();
            Vec3::new(1., 0.08 + 0.5 * vein, 0.01)
        }
        Glowstone => Vec3::new(1., 0.55 + 0.35 * cell, 0.12),
        Snow => WHITE * (0.88 + 0.12 * cell),
        Sand => Vec3::new(0.8, 0.69, 0.4) * grain,
        Netherrack => Vec3::new(0.5, 0.07, 0.08) * grain,
        Obsidian => Vec3::new(0.045, 0.025, 0.075) * (0.8 + cell * 0.3),
        NetherPortal => Vec3::new(0.35 + cell * 0.2, 0.02, 0.7 + cell * 0.25),
        Magma => Vec3::new(0.9, 0.12 + cell * 0.35, 0.015),
        SoulSand => Vec3::new(0.31, 0.22, 0.17) * grain,
        Blackstone => Vec3::new(0.1, 0.1, 0.13) * (0.75 + cell * 0.35),
    }
}
