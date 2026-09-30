mod material;
mod math;
mod renderer;
mod world;
use material::TextureLibrary;
use raylib::prelude::*;
use renderer::{orbit_camera, render, save_image, Settings};
use std::path::{Path, PathBuf};
use std::time::Instant;
use world::World;

struct Args {
    width: usize,
    height: usize,
    frames: usize,
    angle: f32,
    distance: f32,
    output: PathBuf,
    bounces: u32,
    no_shadows: bool,
    headless: bool,
}
fn help() {
    println!("Voxel Raytracer interactivo (Raylib + render CPU)\n\nUSO:\n  cargo run --release\n\nCONTROLES:\n  A/D o flechas   rotar cÃ¡mara\n  W/S o rueda     acercar/alejar\n  Arrastrar ratÃ³n rotar cÃ¡mara\n  R               reiniciar cÃ¡mara\n  ESC             salir\n\nOPCIONES:\n  --width N       resoluciÃ³n interna (default 560)\n  --height N      resoluciÃ³n interna (default 315)\n  --headless      renderizar a archivo sin ventana\n  --frames N      cuadros de animaciÃ³n offline\n  --output RUTA   archivo BMP/PPM o directorio\n  --bounces N     rebotes (default 3)\n  --no-shadows    desactiva sombras\n  --help           muestra esta ayuda");
}
fn parse() -> Result<Args, String> {
    let mut a = Args {
        width: 560,
        height: 315,
        frames: 1,
        angle: 270.,
        distance: 42.,
        output: "render.bmp".into(),
        bounces: 3,
        no_shadows: false,
        headless: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let val = |it: &mut std::iter::Skip<std::env::Args>| {
            it.next().ok_or_else(|| format!("Falta valor para {k}"))
        };
        match k.as_str() {
            "--width" => a.width = val(&mut it)?.parse().map_err(|_| "Ancho invÃƒÂ¡lido")?,
            "--height" => a.height = val(&mut it)?.parse().map_err(|_| "Alto invÃƒÂ¡lido")?,
            "--frames" => a.frames = val(&mut it)?.parse().map_err(|_| "Frames invÃƒÂ¡lidos")?,
            "--angle" => a.angle = val(&mut it)?.parse().map_err(|_| "ÃƒÂngulo invÃƒÂ¡lido")?,
            "--distance" => {
                a.distance = val(&mut it)?.parse().map_err(|_| "Distancia invÃƒÂ¡lida")?
            }
            "--output" => a.output = val(&mut it)?.into(),
            "--bounces" => a.bounces = val(&mut it)?.parse().map_err(|_| "Rebotes invÃƒÂ¡lidos")?,
            "--no-shadows" => a.no_shadows = true,
            "--headless" => a.headless = true,
            "--help" | "-h" => {
                help();
                std::process::exit(0)
            }
            _ => return Err(format!("OpciÃƒÂ³n desconocida: {k}")),
        }
    }
    if a.width == 0 || a.height == 0 || a.frames == 0 {
        return Err("Las dimensiones y frames deben ser mayores que cero".into());
    }
    Ok(a)
}

fn run_window(world: &World, textures: &TextureLibrary, args: &Args, settings: Settings) {
    let window_width = (args.width * 2).max(800) as i32;
    let window_height = (args.height * 2).max(450) as i32;
    let (mut rl, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Diorama Voxel - Ray Tracing CPU en Rust")
        .resizable()
        .build();
    rl.set_target_fps(60);
    let image = Image::gen_image_color(
        args.width as i32,
        args.height as i32,
        raylib::color::Color::BLACK,
    );
    let mut texture = rl
        .load_texture_from_image(&thread, &image)
        .expect("No se pudo crear la textura");
    texture.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    let mut angle = args.angle.to_radians();
    let mut distance = args.distance;
    while !rl.window_should_close() {
        let dt = rl.get_frame_time().min(0.1);
        if rl.is_key_down(KeyboardKey::KEY_A) || rl.is_key_down(KeyboardKey::KEY_LEFT) {
            angle -= 1.25 * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) || rl.is_key_down(KeyboardKey::KEY_RIGHT) {
            angle += 1.25 * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_W) || rl.is_key_down(KeyboardKey::KEY_UP) {
            distance -= 14.0 * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) || rl.is_key_down(KeyboardKey::KEY_DOWN) {
            distance += 14.0 * dt;
        }
        distance -= rl.get_mouse_wheel_move() * 2.2;
        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            angle += rl.get_mouse_delta().x * 0.008;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            angle = 270.0_f32.to_radians();
            distance = 42.0;
        }
        distance = distance.clamp(10.0, 55.0);

        let started = Instant::now();
        let camera = orbit_camera(world, angle, distance);
        let pixels = render(world, textures, camera, settings);
        let rgba = renderer::rgb_to_rgba(&pixels);
        texture
            .update_texture(&rgba)
            .expect("No se pudo actualizar la textura");
        let render_ms = started.elapsed().as_secs_f32() * 1000.0;

        let sw = rl.get_screen_width() as f32;
        let sh = rl.get_screen_height() as f32;
        let scale = (sw / args.width as f32).min(sh / args.height as f32);
        let dw = args.width as f32 * scale;
        let dh = args.height as f32 * scale;
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(raylib::color::Color::BLACK);
        d.draw_texture_pro(
            &texture,
            Rectangle::new(0.0, 0.0, args.width as f32, args.height as f32),
            Rectangle::new((sw - dw) * 0.5, (sh - dh) * 0.5, dw, dh),
            Vector2::zero(),
            0.0,
            raylib::color::Color::WHITE,
        );
        d.draw_rectangle(12, 12, 390, 64, raylib::color::Color::new(0, 0, 0, 170));
        d.draw_text(
            "A/D o arrastrar: rotar | W/S o rueda: zoom | R: reiniciar",
            22,
            20,
            16,
            raylib::color::Color::RAYWHITE,
        );
        d.draw_text(
            &format!(
                "Ray tracing CPU: {:.1} ms | Distancia: {:.1}",
                render_ms, distance
            ),
            22,
            47,
            16,
            raylib::color::Color::GOLD,
        );
    }
}
fn main() {
    let args = parse().unwrap_or_else(|e| {
        eprintln!("Error: {e}\nUse --help.");
        std::process::exit(2)
    });
    let world = World::new();
    let textures = TextureLibrary::load("assets/textures");
    let bounds = renderer::world_bounds();
    println!(
        "Mundo procedural: {}x{}x{} | {} hilos | {} texturas | {} normal maps | {}/6 skybox",
        bounds.0,
        bounds.1,
        bounds.2,
        std::thread::available_parallelism().map_or(1, |n| n.get()),
        textures.loaded_count(),
        textures.normal_map_count(),
        textures.skybox_face_count()
    );
    let settings = Settings {
        width: args.width,
        height: args.height,
        max_bounces: args.bounces,
        shadows: !args.no_shadows,
    };
    if args.frames > 1 {
        std::fs::create_dir_all(&args.output).unwrap_or_else(|e| {
            eprintln!("No se pudo crear {}: {e}", args.output.display());
            std::process::exit(1)
        });
    }
    if !args.headless && args.frames == 1 {
        run_window(&world, &textures, &args, settings);
        return;
    }
    let total = Instant::now();
    for frame in 0..args.frames {
        let phase = frame as f32 / args.frames as f32;
        let angle = (args.angle + phase * 360.).to_radians();
        let zoom = args.distance * (1. - 0.11 * (phase * std::f32::consts::TAU).sin());
        let camera = orbit_camera(&world, angle, zoom);
        let now = Instant::now();
        let pixels = render(&world, &textures, camera, settings);
        let path = if args.frames == 1 {
            args.output.clone()
        } else {
            args.output.join(format!("frame_{frame:04}.bmp"))
        };
        save_image(Path::new(&path), args.width, args.height, &pixels).unwrap_or_else(|e| {
            eprintln!("Error escribiendo {}: {e}", path.display());
            std::process::exit(1)
        });
        println!(
            "[{}/{}] {} ({:0.2}s)",
            frame + 1,
            args.frames,
            path.display(),
            now.elapsed().as_secs_f32()
        );
    }
    println!("Terminado en {:0.2}s", total.elapsed().as_secs_f32());
}
