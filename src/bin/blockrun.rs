use block_native::{
    bytecode,
    model::AssetKind,
    package::{read_package, LoadedAsset},
    vm::Runtime,
};
use macroquad::{audio, prelude::*};
use std::{collections::HashMap, env, fs, path::Path};

fn window_conf() -> Conf {
    Conf {
        window_title: "Block Native".to_owned(),
        window_width: 480,
        window_height: 360,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let path = match env::args_os().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: blockrun <project.bcode|project.bnp>");
            return;
        }
    };
    let path = std::path::PathBuf::from(path);

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("blockrun: failed to read {}: {error}", path.display());
            return;
        }
    };

    let (program_bytes, packed_assets) = if is_extension(&path, "bnp") {
        match read_package(&bytes) {
            Ok(package) => (package.program, package.assets),
            Err(error) => {
                eprintln!("blockrun: {error}");
                return;
            }
        }
    } else {
        (bytes, HashMap::new())
    };

    let program = match bytecode::decode(&program_bytes) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("blockrun: {error}");
            return;
        }
    };

    let mut textures = HashMap::new();
    let mut sounds = HashMap::new();
    load_assets(packed_assets, &mut textures, &mut sounds).await;

    let mut runtime = Runtime::new(program);
    request_new_screen_size(runtime.stage().width as f32, runtime.stage().height as f32);
    next_frame().await;

    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        forward_keys(&mut runtime);
        runtime.update(get_frame_time());
        for sound in runtime.take_audio_events() {
            if let Some(sound) = sounds.get(&sound) {
                audio::play_sound_once(sound);
            }
        }
        draw_runtime(&runtime, &textures);
        next_frame().await;
    }
}

async fn load_assets(
    assets: HashMap<String, LoadedAsset>,
    textures: &mut HashMap<String, Texture2D>,
    sounds: &mut HashMap<String, audio::Sound>,
) {
    for (name, asset) in assets {
        match asset.kind {
            AssetKind::Image => {
                let texture = Texture2D::from_file_with_format(&asset.bytes, None);
                texture.set_filter(FilterMode::Linear);
                textures.insert(name, texture);
            }
            AssetKind::Sound => match audio::load_sound_from_bytes(&asset.bytes).await {
                Ok(sound) => {
                    sounds.insert(name, sound);
                }
                Err(error) => eprintln!("blockrun: failed to decode sound '{name}': {error}"),
            },
        }
    }
}

fn forward_keys(runtime: &mut Runtime) {
    for (name, code) in key_bindings() {
        runtime.set_key(name, is_key_down(*code));
    }
}

fn key_bindings() -> &'static [(&'static str, KeyCode)] {
    &[
        ("space", KeyCode::Space),
        ("left", KeyCode::Left),
        ("right", KeyCode::Right),
        ("up", KeyCode::Up),
        ("down", KeyCode::Down),
        ("enter", KeyCode::Enter),
        ("tab", KeyCode::Tab),
        ("backspace", KeyCode::Backspace),
        ("a", KeyCode::A),
        ("b", KeyCode::B),
        ("c", KeyCode::C),
        ("d", KeyCode::D),
        ("e", KeyCode::E),
        ("f", KeyCode::F),
        ("g", KeyCode::G),
        ("h", KeyCode::H),
        ("i", KeyCode::I),
        ("j", KeyCode::J),
        ("k", KeyCode::K),
        ("l", KeyCode::L),
        ("m", KeyCode::M),
        ("n", KeyCode::N),
        ("o", KeyCode::O),
        ("p", KeyCode::P),
        ("q", KeyCode::Q),
        ("r", KeyCode::R),
        ("s", KeyCode::S),
        ("t", KeyCode::T),
        ("u", KeyCode::U),
        ("v", KeyCode::V),
        ("w", KeyCode::W),
        ("x", KeyCode::X),
        ("y", KeyCode::Y),
        ("z", KeyCode::Z),
        ("0", KeyCode::Key0),
        ("1", KeyCode::Key1),
        ("2", KeyCode::Key2),
        ("3", KeyCode::Key3),
        ("4", KeyCode::Key4),
        ("5", KeyCode::Key5),
        ("6", KeyCode::Key6),
        ("7", KeyCode::Key7),
        ("8", KeyCode::Key8),
        ("9", KeyCode::Key9),
    ]
}

fn draw_runtime(runtime: &Runtime, textures: &HashMap<String, Texture2D>) {
    clear_background(rgba(runtime.stage().background));
    let center_x = screen_width() * 0.5;
    let center_y = screen_height() * 0.5;

    for segment in runtime.pen_segments() {
        draw_line(
            center_x + segment.from.0,
            center_y - segment.from.1,
            center_x + segment.to.0,
            center_y - segment.to.1,
            segment.width,
            rgba(segment.color),
        );
    }

    for sprite in runtime.sprites() {
        let x = center_x + sprite.x;
        let y = center_y - sprite.y;
        if let Some(texture) = sprite
            .costume
            .as_ref()
            .and_then(|costume| textures.get(costume))
        {
            let longest = texture.width().max(texture.height()).max(1.0);
            let scale = sprite.size * 2.0 / longest;
            let width = texture.width() * scale;
            let height = texture.height() * scale;
            draw_texture_ex(
                texture,
                x - width * 0.5,
                y - height * 0.5,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(width, height)),
                    rotation: -sprite.direction.to_radians(),
                    ..Default::default()
                },
            );
        } else {
            draw_directional_triangle(x, y, sprite.size, sprite.direction, rgba(sprite.color));
        }
    }

    draw_text(runtime.name(), 10.0, 22.0, 20.0, BLACK);
    if runtime.is_finished() {
        draw_text(
            "idle - Esc to close",
            10.0,
            screen_height() - 10.0,
            18.0,
            DARKGRAY,
        );
    }
}

fn draw_directional_triangle(x: f32, y: f32, size: f32, direction: f32, color: Color) {
    let angle = -direction.to_radians();
    let point = vec2(x + angle.cos() * size, y + angle.sin() * size);
    let left_angle = angle + 2.45;
    let right_angle = angle - 2.45;
    let left = vec2(
        x + left_angle.cos() * size * 0.75,
        y + left_angle.sin() * size * 0.75,
    );
    let right = vec2(
        x + right_angle.cos() * size * 0.75,
        y + right_angle.sin() * size * 0.75,
    );
    draw_triangle(point, left, right, color);
}

fn is_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn rgba(value: [u8; 4]) -> Color {
    Color::from_rgba(value[0], value[1], value[2], value[3])
}
