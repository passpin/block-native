use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROJECT_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub stage: Stage,
    #[serde(default)]
    pub globals: Vec<Variable>,
    #[serde(default)]
    pub lists: Vec<ListDecl>,
    #[serde(default)]
    pub assets: Vec<Asset>,
    pub sprites: Vec<Sprite>,
}

impl Project {
    pub fn from_json_str(source: &str) -> Result<Self, ProjectLoadError> {
        let value: serde_json::Value = serde_json::from_str(source)?;
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1) as u32;
        match version {
            1 => Ok(upgrade_v1(serde_json::from_value(value)?)),
            PROJECT_VERSION => Ok(serde_json::from_value(value)?),
            other => Err(ProjectLoadError::UnsupportedVersion(other)),
        }
    }

    pub fn from_json_slice(source: &[u8]) -> Result<Self, ProjectLoadError> {
        let text = std::str::from_utf8(source).map_err(ProjectLoadError::Utf8)?;
        Self::from_json_str(text)
    }
}

#[derive(Debug, Error)]
pub enum ProjectLoadError {
    #[error("invalid project JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("project JSON is not UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("unsupported project version {0}")]
    UnsupportedVersion(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Stage {
    pub width: u16,
    pub height: u16,
    pub background: [u8; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Variable {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListDecl {
    pub name: String,
    #[serde(default)]
    pub items: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Value {
    Number(f64),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Image,
    Sound,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Asset {
    pub kind: AssetKind,
    pub name: String,
    pub path: String,
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
    #[serde(default)]
    pub costume: Option<String>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub lists: Vec<ListDecl>,
    #[serde(default)]
    pub scripts: Vec<Script>,
    #[serde(default)]
    pub procedures: Vec<Procedure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Script {
    #[serde(default)]
    pub id: String,
    pub event: Event,
    #[serde(default)]
    pub body: Vec<Command>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    Start,
    Key { key: String },
    Message { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Procedure {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub params: Vec<String>,
    #[serde(default)]
    pub body: Vec<Command>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expr {
    Literal {
        value: Value,
    },
    Var {
        name: String,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
    },
    Key {
        key: String,
    },
    Touching {
        sprite: String,
    },
    ListLen {
        name: String,
    },
}

impl Expr {
    pub fn number(value: impl Into<f64>) -> Self {
        Self::Literal {
            value: Value::Number(value.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Command {
    Move {
        #[serde(default)]
        id: String,
        steps: Expr,
    },
    Turn {
        #[serde(default)]
        id: String,
        degrees: Expr,
    },
    Wait {
        #[serde(default)]
        id: String,
        seconds: Expr,
    },
    Repeat {
        #[serde(default)]
        id: String,
        times: Expr,
        #[serde(default)]
        body: Vec<Command>,
    },
    While {
        #[serde(default)]
        id: String,
        condition: Expr,
        #[serde(default)]
        body: Vec<Command>,
    },
    If {
        #[serde(default)]
        id: String,
        condition: Expr,
        #[serde(default)]
        then_body: Vec<Command>,
        #[serde(default)]
        else_body: Vec<Command>,
    },
    Set {
        #[serde(default)]
        id: String,
        name: String,
        value: Expr,
    },
    Change {
        #[serde(default)]
        id: String,
        name: String,
        delta: Expr,
    },
    Push {
        #[serde(default)]
        id: String,
        list: String,
        value: Expr,
    },
    Broadcast {
        #[serde(default)]
        id: String,
        message: String,
    },
    Call {
        #[serde(default)]
        id: String,
        name: String,
        #[serde(default)]
        args: Vec<Expr>,
    },
    PenDown {
        #[serde(default)]
        id: String,
    },
    PenUp {
        #[serde(default)]
        id: String,
    },
    PenClear {
        #[serde(default)]
        id: String,
    },
    Play {
        #[serde(default)]
        id: String,
        sound: String,
    },
}

#[derive(Debug, Deserialize)]
struct ProjectV1 {
    version: u32,
    name: String,
    stage: Stage,
    sprites: Vec<SpriteV1>,
}

#[derive(Debug, Deserialize)]
struct SpriteV1 {
    #[serde(default)]
    id: String,
    name: String,
    x: f32,
    y: f32,
    direction: f32,
    size: f32,
    color: [u8; 4],
    #[serde(default)]
    script: Vec<CommandV1>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum CommandV1 {
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
        #[serde(default)]
        body: Vec<CommandV1>,
    },
}

fn upgrade_v1(old: ProjectV1) -> Project {
    debug_assert_eq!(old.version, 1);
    Project {
        version: PROJECT_VERSION,
        name: old.name,
        stage: old.stage,
        globals: Vec::new(),
        lists: Vec::new(),
        assets: Vec::new(),
        sprites: old
            .sprites
            .into_iter()
            .map(|sprite| Sprite {
                id: sprite.id,
                name: sprite.name,
                x: sprite.x,
                y: sprite.y,
                direction: sprite.direction,
                size: sprite.size,
                color: sprite.color,
                costume: None,
                variables: Vec::new(),
                lists: Vec::new(),
                scripts: vec![Script {
                    id: String::new(),
                    event: Event::Start,
                    body: sprite.script.into_iter().map(upgrade_command_v1).collect(),
                }],
                procedures: Vec::new(),
            })
            .collect(),
    }
}

fn upgrade_command_v1(command: CommandV1) -> Command {
    match command {
        CommandV1::Move { id, steps } => Command::Move {
            id,
            steps: Expr::number(steps as f64),
        },
        CommandV1::Turn { id, degrees } => Command::Turn {
            id,
            degrees: Expr::number(degrees as f64),
        },
        CommandV1::Wait { id, seconds } => Command::Wait {
            id,
            seconds: Expr::number(seconds as f64),
        },
        CommandV1::Repeat { id, times, body } => Command::Repeat {
            id,
            times: Expr::number(times as f64),
            body: body.into_iter().map(upgrade_command_v1).collect(),
        },
    }
}
