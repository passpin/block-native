use crate::bytecode::{Instruction, Program};
use crate::model::Stage;

const MAX_STEPS_PER_UPDATE: usize = 4096;

#[derive(Debug, Clone)]
pub struct RuntimeSprite {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub direction: f32,
    pub size: f32,
    pub color: [u8; 4],
    instructions: Vec<Instruction>,
    pc: usize,
    wait_remaining: f32,
    finished: bool,
}

#[derive(Debug, Clone)]
pub struct Runtime {
    name: String,
    stage: Stage,
    sprites: Vec<RuntimeSprite>,
}

impl Runtime {
    pub fn new(program: Program) -> Self {
        let sprites = program
            .sprites
            .into_iter()
            .map(|sprite| RuntimeSprite {
                name: sprite.name,
                x: sprite.x,
                y: sprite.y,
                direction: normalize_degrees(sprite.direction),
                size: sprite.size,
                color: sprite.color,
                instructions: sprite.instructions,
                pc: 0,
                wait_remaining: 0.0,
                finished: false,
            })
            .collect();

        Self {
            name: program.name,
            stage: program.stage,
            sprites,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    pub fn sprites(&self) -> &[RuntimeSprite] {
        &self.sprites
    }

    pub fn is_finished(&self) -> bool {
        self.sprites.iter().all(|sprite| sprite.finished)
    }

    pub fn update(&mut self, dt: f32) {
        let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
        for sprite in &mut self.sprites {
            update_sprite(sprite, dt);
        }
    }
}

fn update_sprite(sprite: &mut RuntimeSprite, dt: f32) {
    if sprite.finished {
        return;
    }

    if sprite.wait_remaining > 0.0 {
        sprite.wait_remaining = (sprite.wait_remaining - dt).max(0.0);
        if sprite.wait_remaining > 0.0 {
            return;
        }
    }

    let mut steps = 0usize;
    while steps < MAX_STEPS_PER_UPDATE {
        let Some(instruction) = sprite.instructions.get(sprite.pc).copied() else {
            sprite.finished = true;
            return;
        };
        sprite.pc += 1;
        steps += 1;

        match instruction {
            Instruction::Move(distance) => {
                let radians = sprite.direction.to_radians();
                sprite.x += radians.cos() * distance;
                sprite.y += radians.sin() * distance;
            }
            Instruction::Turn(degrees) => {
                sprite.direction = normalize_degrees(sprite.direction + degrees);
            }
            Instruction::Wait(seconds) => {
                if seconds > 0.0 {
                    sprite.wait_remaining = seconds;
                    return;
                }
            }
        }
    }
}

fn normalize_degrees(value: f32) -> f32 {
    value.rem_euclid(360.0)
}
