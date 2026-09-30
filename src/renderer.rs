use crate::material::{properties, MaterialId, TextureLibrary};
use crate::math::{noise3, Color, Vec3, WHITE};
use crate::world::{intersect, World, SIZE_X, SIZE_Y, SIZE_Z};
use std::io::{self, Seek, SeekFrom, Write};
use std::sync::mpsc;

#[derive(Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub fov: f32,
}
#[derive(Clone, Copy)]
pub struct Settings {
    pub width: usize,
    pub height: usize,
    pub max_bounces: u32,
    pub shadows: bool,
}

fn sky(textures: &TextureLibrary, dir: Vec3) -> Color {
    if let Some(color) = textures.sample_sky(dir) {
        return color;
    }
    let t = (dir.y * 0.5 + 0.5).clamp(0., 1.);
    let mut c = Vec3::new(0.68, 0.82, 1.).lerp(Vec3::new(0.055, 0.12, 0.3), t);
    let sun = dir.dot(Vec3::new(-0.45, 0.72, -0.32).normalized()).max(0.);
    c += Vec3::new(1., 0.72, 0.38) * sun.powf(180.) * 7.;
    let cloud_field = ((dir.x * 9. + dir.z * 3.).sin()
        + (dir.z * 13. - dir.x * 2.).sin() * 0.55
        + (dir.x * 21. + dir.z * 17.).sin() * 0.2)
        / 1.75;
    let cloud = ((cloud_field - 0.28) / 0.5).clamp(0., 1.);
    c + WHITE * cloud * (1. - t) * 0.14
}

fn mapped_normal(textures: &TextureLibrary, id: MaterialId, p: Vec3, n: Vec3) -> Vec3 {
    let m = properties(id);
    if m.normal_strength == 0. {
        return n;
    }
    let tangent = if n.y.abs() < 0.9 {
        Vec3::new(0., 1., 0.).cross(n).normalized()
    } else {
        Vec3::new(1., 0., 0.)
    };
    let bitangent = n.cross(tangent);
    if let Some(encoded) = textures.sample_normal(id, p, n) {
        let tangent_normal = Vec3::new(
            encoded.x * 2.0 - 1.0,
            encoded.y * 2.0 - 1.0,
            encoded.z * 2.0 - 1.0,
        )
        .normalized();
        let mapped =
            (tangent * tangent_normal.x + bitangent * tangent_normal.y + n * tangent_normal.z)
                .normalized();
        return n
            .lerp(mapped, m.normal_strength.clamp(0.0, 1.0))
            .normalized();
    }
    let s = 0.08;
    let h = noise3((p.x / s) as i32, (p.y / s) as i32, (p.z / s) as i32);
    let hx = noise3(((p.x + s) / s) as i32, (p.y / s) as i32, (p.z / s) as i32);
    let hy = noise3((p.x / s) as i32, ((p.y + s) / s) as i32, (p.z / s) as i32);
    (n + tangent * ((h - hx) * m.normal_strength) + bitangent * ((h - hy) * m.normal_strength))
        .normalized()
}

fn visible(world: &World, p: Vec3, to_light: Vec3, dist: f32) -> f32 {
    if intersect(world, p + to_light * 0.003, to_light, dist - 0.01).is_some() {
        0.12
    } else {
        1.
    }
}

fn trace(
    world: &World,
    textures: &TextureLibrary,
    origin: Vec3,
    dir: Vec3,
    depth: u32,
    max_depth: u32,
    shadows: bool,
) -> Color {
    let Some(hit) = intersect(world, origin, dir, 100.) else {
        return sky(textures, dir);
    };
    let mat = properties(hit.material);
    let n = mapped_normal(textures, hit.material, hit.point, hit.normal);
    let base = textures
        .sample(hit.material, hit.point, hit.normal)
        .lerp(mat.albedo, 0.12);
    let mut color = base * 0.055 + mat.emission.hadamard(base * 0.55 + WHITE * 0.45);
    let lights = [
        (Vec3::new(-8., 26., -10.), Vec3::new(1., 0.82, 0.63), 1.25),
        (Vec3::new(11.5, 7., 6.), Vec3::new(1., 0.17, 0.025), 0.85),
        (Vec3::new(5.5, 7.5, 14.5), Vec3::new(1., 0.62, 0.18), 0.7),
        (Vec3::new(35., 9., 10.5), Vec3::new(0.55, 0.08, 1.), 1.35),
        (Vec3::new(42., 7., 12.), Vec3::new(1., 0.18, 0.025), 0.9),
    ];
    for (lp, lc, power) in lights {
        let delta = lp - hit.point;
        let dist = delta.length();
        let l = delta / dist;
        let vis = if shadows {
            visible(world, hit.point + n * 0.004, l, dist)
        } else {
            1.
        };
        let diffuse = n.dot(l).max(0.);
        let half = (l - dir).normalized();
        let spec = n.dot(half).max(0.).powf(mat.shininess) * mat.specular;
        color += (base * diffuse + WHITE * spec).hadamard(lc) * (power * vis);
    }
    if depth < max_depth {
        if mat.reflectivity > 0. {
            let rd = dir.reflect(n).normalized();
            let rc = trace(
                world,
                textures,
                hit.point + n * 0.006,
                rd,
                depth + 1,
                max_depth,
                shadows,
            );
            color = color * (1. - mat.reflectivity) + rc * mat.reflectivity;
        }
        if mat.transparency > 0. {
            let entering = dir.dot(hit.normal) < 0.;
            let nn = if entering { n } else { -n };
            let eta = if entering { 1. / mat.ior } else { mat.ior };
            let td = dir
                .refract(nn, eta)
                .unwrap_or_else(|| dir.reflect(nn))
                .normalized();
            let (exit_origin, exit_direction) = exit_transparent_voxel(hit.point, td, mat.ior);
            let tc = trace(
                world,
                textures,
                exit_origin,
                exit_direction,
                depth + 1,
                max_depth,
                shadows,
            );
            let fresnel = (1. - (-dir).dot(nn).abs()).powf(5.);
            let blend = mat.transparency * (1. - 0.55 * fresnel);
            color = color * (1. - blend) + tc.hadamard(base * 0.45 + WHITE * 0.55) * blend;
        }
    }
    // Distance fog both improves depth perception and hides the finite world boundary.
    color.lerp(sky(textures, dir), ((hit.t - 34.) / 58.).clamp(0., 0.48))
}

fn exit_transparent_voxel(point: Vec3, internal: Vec3, ior: f32) -> (Vec3, Vec3) {
    // Move just inside the block so a ray entering through a negative-facing
    // side cannot mistake that same boundary for its exit.
    let point = point + internal * 0.002;
    let axis_exit = |coordinate: f32, direction: f32| {
        if direction > 1e-6 {
            ((coordinate.floor() + 1.0 - coordinate) / direction, 1.0)
        } else if direction < -1e-6 {
            ((coordinate - coordinate.floor()) / -direction, -1.0)
        } else {
            (f32::INFINITY, 0.0)
        }
    };
    let (tx, nx) = axis_exit(point.x, internal.x);
    let (ty, ny) = axis_exit(point.y, internal.y);
    let (tz, nz) = axis_exit(point.z, internal.z);
    let (distance, outward) = if tx <= ty && tx <= tz {
        (tx, Vec3::new(nx, 0.0, 0.0))
    } else if ty <= tz {
        (ty, Vec3::new(0.0, ny, 0.0))
    } else {
        (tz, Vec3::new(0.0, 0.0, nz))
    };
    let exit_point = point + internal * distance;
    let outgoing = internal
        .refract(-outward, ior)
        .unwrap_or_else(|| internal.reflect(outward))
        .normalized();
    (exit_point + outgoing * 0.006, outgoing)
}

pub fn render(world: &World, textures: &TextureLibrary, camera: Camera, s: Settings) -> Vec<u8> {
    let forward = (camera.target - camera.position).normalized();
    let right = forward.cross(Vec3::new(0., 1., 0.)).normalized();
    let up = right.cross(forward);
    let scale = (camera.fov.to_radians() * 0.5).tan();
    let aspect = s.width as f32 / s.height as f32;
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let rows = s.height.div_ceil(threads);
    let (tx, rx) = mpsc::channel();
    std::thread::scope(|scope| {
        for worker in 0..threads {
            let tx = tx.clone();
            let y0 = worker * rows;
            let y1 = (y0 + rows).min(s.height);
            if y0 >= y1 {
                continue;
            }
            scope.spawn(move || {
                let mut data = Vec::with_capacity((y1 - y0) * s.width * 3);
                for y in y0..y1 {
                    for x in 0..s.width {
                        let px = (2. * ((x as f32 + 0.5) / s.width as f32) - 1.) * aspect * scale;
                        let py = (1. - 2. * ((y as f32 + 0.5) / s.height as f32)) * scale;
                        let dir = (forward + right * px + up * py).normalized();
                        let c = trace(
                            world,
                            textures,
                            camera.position,
                            dir,
                            0,
                            s.max_bounces,
                            s.shadows,
                        );
                        let mapped =
                            Vec3::new(c.x / (1. + c.x), c.y / (1. + c.y), c.z / (1. + c.z));
                        let gamma = Vec3::new(mapped.x.sqrt(), mapped.y.sqrt(), mapped.z.sqrt())
                            .clamp(0., 1.);
                        data.extend_from_slice(&[
                            (gamma.x * 255.) as u8,
                            (gamma.y * 255.) as u8,
                            (gamma.z * 255.) as u8,
                        ]);
                    }
                }
                tx.send((y0, data)).unwrap();
            });
        }
        drop(tx);
        let mut image = vec![0; s.width * s.height * 3];
        for (y0, data) in rx {
            let start = y0 * s.width * 3;
            image[start..start + data.len()].copy_from_slice(&data);
        }
        image
    })
}

pub fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(rgb.len() / 3 * 4);
    for pixel in rgb.chunks_exact(3) {
        rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
    }
    rgba
}

pub fn save_ppm(path: &std::path::Path, w: usize, h: usize, pixels: &[u8]) -> io::Result<()> {
    let f = std::fs::File::create(path)?;
    let mut out = io::BufWriter::new(f);
    write!(out, "P6\n{} {}\n255\n", w, h)?;
    out.write_all(pixels)
}

pub fn save_bmp(path: &std::path::Path, w: usize, h: usize, pixels: &[u8]) -> io::Result<()> {
    let row_size = (w * 3 + 3) & !3;
    let image_size = row_size * h;
    let mut out = io::BufWriter::new(std::fs::File::create(path)?);
    out.write_all(b"BM")?;
    out.write_all(&(54u32 + image_size as u32).to_le_bytes())?;
    out.write_all(&[0; 4])?;
    out.write_all(&54u32.to_le_bytes())?;
    out.write_all(&40u32.to_le_bytes())?;
    out.write_all(&(w as i32).to_le_bytes())?;
    out.write_all(&(h as i32).to_le_bytes())?;
    out.write_all(&1u16.to_le_bytes())?;
    out.write_all(&24u16.to_le_bytes())?;
    out.write_all(&[0; 4])?;
    out.write_all(&(image_size as u32).to_le_bytes())?;
    out.write_all(&[0; 16])?;
    let padding = vec![0; row_size - w * 3];
    for y in (0..h).rev() {
        for rgb in pixels[y * w * 3..(y + 1) * w * 3].chunks_exact(3) {
            out.write_all(&[rgb[2], rgb[1], rgb[0]])?;
        }
        out.write_all(&padding)?;
    }
    out.flush()?;
    out.seek(SeekFrom::End(0))?;
    Ok(())
}

pub fn save_image(path: &std::path::Path, w: usize, h: usize, pixels: &[u8]) -> io::Result<()> {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("bmp"))
    {
        save_bmp(path, w, h, pixels)
    } else {
        save_ppm(path, w, h, pixels)
    }
}

pub fn orbit_camera(world: &World, angle: f32, distance: f32) -> Camera {
    let center = world.center();
    Camera {
        position: center
            + Vec3::new(
                angle.cos() * distance,
                distance * 0.48,
                angle.sin() * distance,
            ),
        target: center + Vec3::new(0., 1.5, 0.),
        fov: 48.,
    }
}

pub fn world_bounds() -> (i32, i32, i32) {
    (SIZE_X, SIZE_Y, SIZE_Z)
}
