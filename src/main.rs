mod framebuffer;
mod generated_map;
mod geometry;
mod math;
mod renderer;
mod scene;

use framebuffer::Framebuffer;
use geometry::Ray;
use raylib::prelude::*;
use renderer::{Camera, render};
use scene::{Scene, WATER_LEVEL};
use std::f32::consts::PI;
use std::time::Instant;

const RENDER_WIDTH: usize = 720;
const RENDER_HEIGHT: usize = 405;

struct Track {
    title: String,
    artist: String,
    path: String,
}

fn load_tracks() -> Vec<Track> {
    std::fs::read_to_string("assets/music/list.txt")
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split('|').map(str::trim);
            Some(Track {
                path: fields.next()?.to_owned(),
                artist: fields.next()?.to_owned(),
                title: fields.next()?.to_owned(),
            })
        })
        .collect()
}

fn start_track<'a>(audio: &'a RaylibAudio, track: &Track) -> Option<Sound<'a>> {
    let sound = audio.new_sound(&track.path).ok()?;
    sound.set_volume(0.55);
    sound.play();
    Some(sound)
}

fn write_ppm(path: &str, pixels: &[u8], width: usize, height: usize) {
    use std::io::Write;
    let mut file = std::fs::File::create(path).expect("No se pudo crear la captura");
    write!(file, "P6\n{} {}\n255\n", width, height).unwrap();
    for pixel in pixels.chunks_exact(4) {
        file.write_all(&pixel[..3]).unwrap()
    }
}

fn render_offline(idol_mode: bool, time: f32) {
    let scene = Scene::load();
    let idol_index = ((time.rem_euclid(24.0) / 24.0) * scene.idol_count() as f32) as usize;
    let camera = Camera {
        yaw: 1.52,
        pitch: 0.16,
        distance: 92.,
        target: scene.center,
        fov: 42f32.to_radians(),
    };
    let mut pixels = vec![0u8; RENDER_WIDTH * RENDER_HEIGHT * 4];
    for pass in 0..4 {
        render(
            &scene,
            camera,
            time,
            &mut pixels,
            RENDER_WIDTH,
            RENDER_HEIGHT,
            pass,
            2,
            3,
            idol_mode,
            idol_index,
        )
    }
    write_ppm(
        "kame_house_preview.ppm",
        &pixels,
        RENDER_WIDTH,
        RENDER_HEIGHT,
    );
}

fn render_video_frames() {
    const W: usize = 480;
    const H: usize = 270;
    const FRAMES: usize = 32;
    std::fs::create_dir_all("video_frames").unwrap();
    let scene = Scene::load();
    let mut pixels = vec![0u8; W * H * 4];
    for frame in 0..FRAMES {
        let phase = frame as f32 / FRAMES as f32;
        let camera = Camera {
            yaw: 1.52 + phase * std::f32::consts::TAU,
            pitch: 0.16,
            distance: 92.,
            target: scene.center,
            fov: 42f32.to_radians(),
        };
        let time = 7. + phase * 16.;
        for pass in 0..4 {
            render(
                &scene,
                camera,
                time,
                &mut pixels,
                W,
                H,
                pass,
                2,
                3,
                false,
                0,
            )
        }
        write_ppm(&format!("video_frames/frame_{frame:03}.ppm"), &pixels, W, H);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--offline") {
        let idol_mode = args.iter().any(|arg| arg == "--idols");
        let time = if args.iter().any(|arg| arg == "--night") {
            5.4
        } else {
            15.2
        };
        render_offline(idol_mode, time);
        return;
    }
    if args.iter().any(|arg| arg == "--video-frames") {
        render_video_frames();
        return;
    }
    let (mut window, thread) = raylib::init()
        .size(1280, 720)
        .title("KAME HOUSE")
        .resizable()
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();
    window.set_target_fps(60);
    let audio = RaylibAudio::init_audio_device().ok();
    let tracks = load_tracks();
    let mut track_index = 0usize;
    let mut song = audio
        .as_ref()
        .zip(tracks.first())
        .and_then(|(device, track)| start_track(device, track));
    let scene = Scene::load();
    let mut framebuffer = Framebuffer::new(&mut window, &thread, RENDER_WIDTH, RENDER_HEIGHT);
    let mut camera = Camera {
        yaw: 1.52,
        pitch: 0.16,
        distance: 92.,
        target: scene.center,
        fov: 42f32.to_radians(),
    };
    let mut time_of_day = 15.2f32;
    let mut auto_rotate = false;
    let mut auto_time = false;
    let mut idol_mode = false;
    let mut progressive_pass = 0u8;
    let mut interactive_pass = 0u8;
    let mut was_moving = false;
    let mut last_mouse = window.get_mouse_position();
    let mut dragging = false;
    let mut render_ms = 0.;
    while !window.window_should_close() {
        let dt = window.get_frame_time().min(0.05);
        let mut moved = false;
        let rotation_speed = 1.1 * dt;
        if window.is_key_down(KeyboardKey::KEY_LEFT) || window.is_key_down(KeyboardKey::KEY_A) {
            camera.yaw -= rotation_speed;
            moved = true
        }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) || window.is_key_down(KeyboardKey::KEY_D) {
            camera.yaw += rotation_speed;
            moved = true
        }
        if window.is_key_down(KeyboardKey::KEY_UP) {
            camera.pitch = (camera.pitch + 0.7 * dt).min(0.72);
            moved = true
        }
        if window.is_key_down(KeyboardKey::KEY_DOWN) {
            camera.pitch = (camera.pitch - 0.7 * dt).max(-0.08);
            moved = true
        }
        if window.is_key_down(KeyboardKey::KEY_W) {
            camera.distance *= (1.0 - 1.6 * dt).max(0.1);
            moved = true
        }
        if window.is_key_down(KeyboardKey::KEY_S) {
            camera.distance *= 1.0 + 1.6 * dt;
            moved = true
        }
        let wheel = window.get_mouse_wheel_move();
        if wheel != 0. {
            camera.distance *= 0.86f32.powf(wheel);
            moved = true
        }
        if window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            dragging = true;
            last_mouse = window.get_mouse_position()
        }
        if window.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT) {
            dragging = false
        }
        if dragging {
            let p = window.get_mouse_position();
            let delta = p - last_mouse;
            if delta.x != 0. || delta.y != 0. {
                camera.yaw -= delta.x * 0.006;
                camera.pitch = (camera.pitch + delta.y * 0.004).clamp(-0.08, 0.72);
                last_mouse = p;
                moved = true
            }
        }
        if window.is_key_pressed(KeyboardKey::KEY_SPACE) {
            auto_rotate = !auto_rotate
        }
        if window.is_key_pressed(KeyboardKey::KEY_T) {
            auto_time = !auto_time
        }
        if window.is_key_pressed(KeyboardKey::KEY_I) {
            idol_mode = !idol_mode;
            moved = true
        }
        let previous_track = window.is_key_pressed(KeyboardKey::KEY_B);
        let next_track = window.is_key_pressed(KeyboardKey::KEY_N);
        if (previous_track || next_track) && !tracks.is_empty() {
            if let Some(sound) = song.take() {
                sound.stop();
            }
            track_index = if previous_track {
                (track_index + tracks.len() - 1) % tracks.len()
            } else {
                (track_index + 1) % tracks.len()
            };
            song = audio
                .as_ref()
                .and_then(|device| start_track(device, &tracks[track_index]));
        }
        if window.is_key_pressed(KeyboardKey::KEY_R) {
            camera = Camera {
                yaw: 1.52,
                pitch: 0.16,
                distance: 92.,
                target: scene.center,
                fov: 42f32.to_radians(),
            };
            time_of_day = 15.2;
            moved = true
        }
        if window.is_key_down(KeyboardKey::KEY_Q) {
            time_of_day = (time_of_day - 3. * dt).rem_euclid(24.);
            moved = true
        }
        if window.is_key_down(KeyboardKey::KEY_E) {
            time_of_day = (time_of_day + 3. * dt).rem_euclid(24.);
            moved = true
        }
        if auto_rotate {
            camera.yaw = (camera.yaw + 0.16 * dt).rem_euclid(PI * 2.);
            moved = true
        }
        if auto_time {
            time_of_day = (time_of_day + 0.32 * dt).rem_euclid(24.);
            moved = true
        }
        let outward = (camera.position() - camera.target).normalized();
        let minimum_zoom = scene
            .bvh
            .intersect(Ray::new(camera.target, outward), 500., usize::MAX)
            .map(|hit| hit.distance + 0.8)
            .unwrap_or(2.0);
        camera.distance = camera.distance.clamp(minimum_zoom, 2500.0);
        let water_safe_pitch = ((WATER_LEVEL + 1.0 - camera.target.y) / camera.distance)
            .clamp(-1.0, 1.0)
            .asin();
        camera.pitch = camera.pitch.max(water_safe_pitch).min(0.72);
        let idol_index =
            ((time_of_day.rem_euclid(24.0) / 24.0) * scene.idol_count() as f32) as usize;
        if moved {
            interactive_pass = (interactive_pass + 1) & 15;
            progressive_pass = 0
        } else if was_moving {
            progressive_pass = 0
        }
        let should_render = moved || progressive_pass < 4;
        if should_render {
            let (pass, bits, depth) = if moved {
                (interactive_pass & 7, 3, 2)
            } else {
                (progressive_pass, 2, 3)
            };
            let start = Instant::now();
            render(
                &scene,
                camera,
                time_of_day,
                &mut framebuffer.pixels,
                framebuffer.width,
                framebuffer.height,
                pass,
                bits,
                depth,
                idol_mode,
                idol_index,
            );
            render_ms = start.elapsed().as_secs_f32() * 1000.;
            framebuffer.upload();
            if !moved {
                progressive_pass += 1
            }
        }
        was_moving = moved;
        if song.as_ref().is_some_and(|sound| !sound.is_playing()) && !tracks.is_empty() {
            if let Some(sound) = song.take() {
                sound.stop();
            }
            track_index = (track_index + 1) % tracks.len();
            song = audio
                .as_ref()
                .and_then(|device| start_track(device, &tracks[track_index]));
        }
        let fps = window.get_fps();
        let screen_w = window.get_screen_width();
        let screen_h = window.get_screen_height();
        let mut d = window.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        framebuffer.draw(&mut d, screen_w, screen_h);
        d.draw_rectangle(12, 12, 560, 108, Color::new(5, 10, 18, 205));
        d.draw_text("KAME HOUSE", 24, 20, 25, Color::new(255, 223, 166, 255));
        d.draw_text(
            &format!(
                "Hora {:05.2}  |  FPS {:>3}  |  {:0.1} ms  |  {} bloques / {} cubos",
                time_of_day, fps, render_ms, scene.block_count, scene.cuboid_count
            ),
            24,
            52,
            15,
            Color::RAYWHITE,
        );
        let sky_label = if idol_mode {
            format!(
                "Pared IDOLS: {}/{} segun hora (I: normal)",
                idol_index + 1,
                scene.idol_count()
            )
        } else {
            "Cielo: NORMAL (I para idols)".to_owned()
        };
        d.draw_text(&sky_label, 24, 76, 14, Color::new(112, 235, 220, 255));
        let music_label = tracks
            .get(track_index)
            .map(|track| format!("Musica: {} - {}  (B/N cambiar)", track.artist, track.title))
            .unwrap_or_else(|| "Musica: lista vacia".to_owned());
        d.draw_text(&music_label, 24, 98, 14, Color::new(255, 196, 218, 255));
        d.draw_rectangle(12, screen_h - 58, 690, 46, Color::new(5, 10, 18, 205));
        d.draw_text(
            "MOUSE/A-D: rotar   ARRIBA/ABAJO: altura   RUEDA/W-S: zoom   Q-E: hora",
            24,
            screen_h - 51,
            14,
            Color::RAYWHITE,
        );
        d.draw_text(
            "ESPACIO: rotacion   T: ciclo   I: idols   B/N: musica   R: reiniciar",
            24,
            screen_h - 30,
            13,
            Color::LIGHTGRAY,
        );
        d.draw_rectangle_lines_ex(
            Rectangle::new(3., 3., screen_w as f32 - 6., screen_h as f32 - 6.),
            2.,
            Color::new(112, 235, 220, 150),
        );
        let mode = if auto_rotate {
            "ROTACION AUTO"
        } else if progressive_pass < 4 {
            "REFINANDO"
        } else {
            "CALIDAD COMPLETA"
        };
        let tw = d.measure_text(mode, 16);
        d.draw_rectangle(
            screen_w - tw - 28,
            16,
            tw + 16,
            26,
            Color::new(10, 25, 34, 190),
        );
        d.draw_text(
            mode,
            screen_w - tw - 20,
            21,
            16,
            Color::new(112, 235, 220, 255),
        );
        drop(d);
    }
}
