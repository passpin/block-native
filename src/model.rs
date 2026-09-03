use serde::{Deserialize, Serialize};

pub const PROJECT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub stage: Stage,
    pub sprites: Vec<Sprite>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Stage {
    pub width: u16,
    pub height: u16,
    pub background: [u8; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sprite {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub direction: f32,
    pub size: f32,
    pub color: [u8; 4],
    pub script: Vec<Command>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Command {
    Move {
        #[serde(default)]
        id: String,
        steps: f32,
    },
    Turn {
        #[serde(default)]
        id: String,
        degrees: f32,
    },
    Wait {
        #[serde(default)]
        id: String,
        seconds: f32,
    },
    Repeat {
        #[serde(default)]
        id: String,
        times: u32,
        body: Vec<Command>,
    },
}
