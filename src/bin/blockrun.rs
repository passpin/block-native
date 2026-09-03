use block_native::{bytecode, vm::Runtime};
use macroquad::prelude::*;
use std::{env, fs};

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
            eprintln!("usage: blockrun <project.bcode>");
            return;
        }
    };

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("blockrun: failed to read {:?}: {error}", path);
            return;
        }
    };
    let program = match bytecode::decode(&bytes) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("blockrun: {error}");
            return;
        }
    };

    let mut runtime = Runtime::new(program);
    request_new_screen_size(runtime.stage().width as f32, runtime.stage().height as f32);
    next_frame().await;

    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        runtime.update(get_frame_time());
        draw_runtime(&runtime);
        next_frame().await;
    }
}

fn draw_runtime(runtime: &Runtime) {
    clear_background(rgba(runtime.stage().background));
    let center_x = screen_width() * 0.5;
    let center_y = screen_height() * 0.5;
    for sprite in runtime.sprites() {
        draw_directional_triangle(
            center_x + sprite.x,
            center_y - sprite.y,
            sprite.size,
            sprite.direction,
            rgba(sprite.color),
        );
    }
    draw_text(runtime.name(), 10.0, 22.0, 20.0, BLACK);
    if runtime.is_finished() {
        draw_text(
            "finished - Esc to close",
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

fn rgba(value: [u8; 4]) -> Color {
    Color::from_rgba(value[0], value[1], value[2], value[3])
}
