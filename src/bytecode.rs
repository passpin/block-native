use crate::model::{
    Asset, Command, Event, Expr, ListDecl, Procedure, Project, Stage, Value, Variable,
    PROJECT_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAGIC_V1: &[u8; 4] = b"BLK1";
const MAGIC_V2: &[u8; 4] = b"BLK2";
const BYTECODE_VERSION: u16 = 2;
pub const MAX_INSTRUCTIONS_PER_SPRITE: usize = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Program {
    pub name: String,
    pub stage: Stage,
    #[serde(default)]
    pub globals: Vec<Variable>,
    #[serde(default)]
    pub lists: Vec<ListDecl>,
    #[serde(default)]
    pub assets: Vec<Asset>,
    pub sprites: Vec<CompiledSprite>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompiledSprite {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub direction: f32,
    pub size: f32,
    pub color: [u8; 4],
    pub costume: Option<String>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub lists: Vec<ListDecl>,
    #[serde(default)]
    pub scripts: Vec<CompiledScript>,
    #[serde(default)]
    pub procedures: Vec<CompiledProcedure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompiledScript {
    pub event: Event,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompiledProcedure {
    pub name: String,
    pub params: Vec<String>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Instruction {
    Move {
        value: Expr,
    },
    Turn {
        value: Expr,
    },
    Wait {
        value: Expr,
    },
    Repeat {
        times: Expr,
        body: Vec<Instruction>,
    },
    While {
        condition: Expr,
        body: Vec<Instruction>,
    },
    If {
        condition: Expr,
        then_body: Vec<Instruction>,
        else_body: Vec<Instruction>,
    },
    Set {
        name: String,
        value: Expr,
    },
    Change {
        name: String,
        delta: Expr,
    },
    Push {
        list: String,
        value: Expr,
    },
    Broadcast {
        message: String,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    PenDown,
    PenUp,
    PenClear,
    Play {
        sound: String,
    },
}

#[derive(Debug, Error)]
pub enum BytecodeError {
    #[error("invalid project: {0}")]
    InvalidProject(String),
    #[error("script contains more than {MAX_INSTRUCTIONS_PER_SPRITE} instructions")]
    ExpansionLimit,
    #[error("invalid bytecode: {0}")]
    InvalidBytecode(String),
}

pub fn compile(project: &Project) -> Result<Vec<u8>, BytecodeError> {
    validate_project(project)?;
    let program = Program {
        name: project.name.clone(),
        stage: project.stage.clone(),
        globals: project.globals.clone(),
        lists: project.lists.clone(),
        assets: project.assets.clone(),
        sprites: project
            .sprites
            .iter()
            .map(|sprite| {
                let count = sprite
                    .scripts
                    .iter()
                    .map(|script| count_commands(&script.body))
                    .chain(
                        sprite
                            .procedures
                            .iter()
                            .map(|proc_| count_commands(&proc_.body)),
                    )
                    .try_fold(0usize, |total, next| {
                        total
                            .checked_add(next?)
                            .filter(|value| *value <= MAX_INSTRUCTIONS_PER_SPRITE)
                            .ok_or(BytecodeError::ExpansionLimit)
                    })?;
                if count > MAX_INSTRUCTIONS_PER_SPRITE {
                    return Err(BytecodeError::ExpansionLimit);
                }
                Ok(CompiledSprite {
                    id: sprite.id.clone(),
                    name: sprite.name.clone(),
                    x: sprite.x,
                    y: sprite.y,
                    direction: sprite.direction,
                    size: sprite.size,
                    color: sprite.color,
                    costume: sprite.costume.clone(),
                    variables: sprite.variables.clone(),
                    lists: sprite.lists.clone(),
                    scripts: sprite
                        .scripts
                        .iter()
                        .map(|script| CompiledScript {
                            event: script.event.clone(),
                            instructions: compile_commands(&script.body),
                        })
                        .collect(),
                    procedures: sprite
                        .procedures
                        .iter()
                        .map(|proc_| compile_procedure(proc_))
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, BytecodeError>>()?,
    };

    let payload = serde_json::to_vec(&program).map_err(|error| {
        BytecodeError::InvalidProject(format!("cannot serialize program: {error}"))
    })?;
    let payload_len = u32::try_from(payload.len())
        .map_err(|_| BytecodeError::InvalidProject("compiled program is too large".into()))?;
    let mut out = Vec::with_capacity(10 + payload.len());
    out.extend_from_slice(MAGIC_V2);
    out.extend_from_slice(&BYTECODE_VERSION.to_le_bytes());
    out.extend_from_slice(&payload_len.to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

pub fn decode(bytes: &[u8]) -> Result<Program, BytecodeError> {
    match bytes.get(..4) {
        Some(magic) if magic == MAGIC_V2 => decode_v2(bytes),
        Some(magic) if magic == MAGIC_V1 => decode_v1(bytes),
        _ => Err(BytecodeError::InvalidBytecode("bad magic".into())),
    }
}

fn decode_v2(bytes: &[u8]) -> Result<Program, BytecodeError> {
    if bytes.len() < 10 {
        return Err(BytecodeError::InvalidBytecode(
            "truncated BLK2 header".into(),
        ));
    }
    let version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
    if version != BYTECODE_VERSION {
        return Err(BytecodeError::InvalidBytecode(format!(
            "unsupported BLK2 version {version}"
        )));
    }
    let len = u32::from_le_bytes(bytes[6..10].try_into().unwrap()) as usize;
    let payload = bytes
        .get(10..10 + len)
        .ok_or_else(|| BytecodeError::InvalidBytecode("truncated BLK2 payload".into()))?;
    if bytes.len() != 10 + len {
        return Err(BytecodeError::InvalidBytecode(
            "trailing bytes after BLK2 payload".into(),
        ));
    }
    serde_json::from_slice(payload)
        .map_err(|error| BytecodeError::InvalidBytecode(format!("invalid BLK2 payload: {error}")))
}

fn compile_procedure(procedure: &Procedure) -> CompiledProcedure {
    CompiledProcedure {
        name: procedure.name.clone(),
        params: procedure.params.clone(),
        instructions: compile_commands(&procedure.body),
    }
}

fn compile_commands(commands: &[Command]) -> Vec<Instruction> {
    commands
        .iter()
        .map(|command| match command {
            Command::Move { steps, .. } => Instruction::Move {
                value: steps.clone(),
            },
            Command::Turn { degrees, .. } => Instruction::Turn {
                value: degrees.clone(),
            },
            Command::Wait { seconds, .. } => Instruction::Wait {
                value: seconds.clone(),
            },
            Command::Repeat { times, body, .. } => Instruction::Repeat {
                times: times.clone(),
                body: compile_commands(body),
            },
            Command::While {
                condition, body, ..
            } => Instruction::While {
                condition: condition.clone(),
                body: compile_commands(body),
            },
            Command::If {
                condition,
                then_body,
                else_body,
                ..
            } => Instruction::If {
                condition: condition.clone(),
                then_body: compile_commands(then_body),
                else_body: compile_commands(else_body),
            },
            Command::Set { name, value, .. } => Instruction::Set {
                name: name.clone(),
                value: value.clone(),
            },
            Command::Change { name, delta, .. } => Instruction::Change {
                name: name.clone(),
                delta: delta.clone(),
            },
            Command::Push { list, value, .. } => Instruction::Push {
                list: list.clone(),
                value: value.clone(),
            },
            Command::Broadcast { message, .. } => Instruction::Broadcast {
                message: message.clone(),
            },
            Command::Call { name, args, .. } => Instruction::Call {
                name: name.clone(),
                args: args.clone(),
            },
            Command::PenDown { .. } => Instruction::PenDown,
            Command::PenUp { .. } => Instruction::PenUp,
            Command::PenClear { .. } => Instruction::PenClear,
            Command::Play { sound, .. } => Instruction::Play {
                sound: sound.clone(),
            },
        })
        .collect()
}

fn validate_project(project: &Project) -> Result<(), BytecodeError> {
    if project.version != PROJECT_VERSION {
        return Err(BytecodeError::InvalidProject(format!(
            "unsupported project version {}; expected {PROJECT_VERSION}",
            project.version
        )));
    }
    if project.name.trim().is_empty() {
        return Err(BytecodeError::InvalidProject(
            "project name cannot be empty".into(),
        ));
    }
    if project.stage.width == 0 || project.stage.height == 0 {
        return Err(BytecodeError::InvalidProject(
            "stage dimensions must be positive".into(),
        ));
    }
    for sprite in &project.sprites {
        if sprite.name.trim().is_empty() {
            return Err(BytecodeError::InvalidProject(
                "sprite name cannot be empty".into(),
            ));
        }
        if !sprite.x.is_finite()
            || !sprite.y.is_finite()
            || !sprite.direction.is_finite()
            || !sprite.size.is_finite()
            || sprite.size <= 0.0
        {
            return Err(BytecodeError::InvalidProject(format!(
                "sprite '{}' has invalid transform data",
                sprite.name
            )));
        }
        for script in &sprite.scripts {
            validate_commands(&script.body)?;
        }
        for procedure in &sprite.procedures {
            validate_commands(&procedure.body)?;
        }
    }
    Ok(())
}

fn validate_commands(commands: &[Command]) -> Result<(), BytecodeError> {
    for command in commands {
        match command {
            Command::Move { steps, .. } => validate_expr(steps)?,
            Command::Turn { degrees, .. } => validate_expr(degrees)?,
            Command::Wait { seconds, .. } => validate_expr(seconds)?,
            Command::Repeat { times, body, .. } => {
                validate_expr(times)?;
                validate_commands(body)?;
            }
            Command::While {
                condition, body, ..
            } => {
                validate_expr(condition)?;
                validate_commands(body)?;
            }
            Command::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                validate_expr(condition)?;
                validate_commands(then_body)?;
                validate_commands(else_body)?;
            }
            Command::Set { value, .. } => validate_expr(value)?,
            Command::Change { delta, .. } => validate_expr(delta)?,
            Command::Push { value, .. } => validate_expr(value)?,
            Command::Call { args, .. } => {
                for arg in args {
                    validate_expr(arg)?;
                }
            }
            Command::Broadcast { .. }
            | Command::PenDown { .. }
            | Command::PenUp { .. }
            | Command::PenClear { .. }
            | Command::Play { .. } => {}
        }
    }
    Ok(())
}

fn validate_expr(expr: &Expr) -> Result<(), BytecodeError> {
    match expr {
        Expr::Literal {
            value: Value::Number(value),
        } if !value.is_finite() => Err(BytecodeError::InvalidProject(
            "expression contains non-finite number".into(),
        )),
        Expr::Binary { left, right, .. } => {
            validate_expr(left)?;
            validate_expr(right)
        }
        Expr::Unary { value, .. } => validate_expr(value),
        _ => Ok(()),
    }
}

fn count_commands(commands: &[Command]) -> Result<usize, BytecodeError> {
    let mut total = 0usize;
    for command in commands {
        let nested = match command {
            Command::Repeat { body, .. } | Command::While { body, .. } => count_commands(body)?,
            Command::If {
                then_body,
                else_body,
                ..
            } => count_commands(then_body)?
                .checked_add(count_commands(else_body)?)
                .ok_or(BytecodeError::ExpansionLimit)?,
            _ => 0,
        };
        total = total
            .checked_add(1 + nested)
            .filter(|value| *value <= MAX_INSTRUCTIONS_PER_SPRITE)
            .ok_or(BytecodeError::ExpansionLimit)?;
    }
    Ok(total)
}

fn decode_v1(bytes: &[u8]) -> Result<Program, BytecodeError> {
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != MAGIC_V1 {
        return Err(BytecodeError::InvalidBytecode("bad BLK1 magic".into()));
    }
    let version = reader.u16()?;
    if version != 1 {
        return Err(BytecodeError::InvalidBytecode(format!(
            "unsupported BLK1 version {version}"
        )));
    }
    let name = reader.string()?;
    let width = reader.u16()?;
    let height = reader.u16()?;
    let background = reader.rgba()?;
    let sprite_count = reader.u16()? as usize;
    let mut sprites = Vec::with_capacity(sprite_count);
    for _ in 0..sprite_count {
        let sprite_name = reader.string()?;
        let x = reader.f32()?;
        let y = reader.f32()?;
        let direction = reader.f32()?;
        let size = reader.f32()?;
        let color = reader.rgba()?;
        let instruction_count = reader.u32()? as usize;
        if instruction_count > MAX_INSTRUCTIONS_PER_SPRITE {
            return Err(BytecodeError::InvalidBytecode(
                "BLK1 instruction count exceeds runtime limit".into(),
            ));
        }
        let mut instructions = Vec::with_capacity(instruction_count);
        for _ in 0..instruction_count {
            let opcode = reader.u8()?;
            let value = reader.f32()?;
            if !value.is_finite() {
                return Err(BytecodeError::InvalidBytecode(
                    "BLK1 instruction contains non-finite value".into(),
                ));
            }
            instructions.push(match opcode {
                1 => Instruction::Move {
                    value: Expr::number(value as f64),
                },
                2 => Instruction::Turn {
                    value: Expr::number(value as f64),
                },
                3 if value >= 0.0 => Instruction::Wait {
                    value: Expr::number(value as f64),
                },
                3 => {
                    return Err(BytecodeError::InvalidBytecode(
                        "BLK1 wait duration cannot be negative".into(),
                    ))
                }
                other => {
                    return Err(BytecodeError::InvalidBytecode(format!(
                        "unknown BLK1 opcode {other}"
                    )))
                }
            });
        }
        sprites.push(CompiledSprite {
            id: String::new(),
            name: sprite_name,
            x,
            y,
            direction,
            size,
            color,
            costume: None,
            variables: Vec::new(),
            lists: Vec::new(),
            scripts: vec![CompiledScript {
                event: Event::Start,
                instructions,
            }],
            procedures: Vec::new(),
        });
    }
    if !reader.is_finished() {
        return Err(BytecodeError::InvalidBytecode(
            "trailing bytes after BLK1 program".into(),
        ));
    }
    Ok(Program {
        name,
        stage: Stage {
            width,
            height,
            background,
        },
        globals: Vec::new(),
        lists: Vec::new(),
        assets: Vec::new(),
        sprites,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], BytecodeError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| BytecodeError::InvalidBytecode("offset overflow".into()))?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| BytecodeError::InvalidBytecode("truncated file".into()))?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, BytecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, BytecodeError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, BytecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32, BytecodeError> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn rgba(&mut self) -> Result<[u8; 4], BytecodeError> {
        Ok(self.take(4)?.try_into().unwrap())
    }

    fn string(&mut self) -> Result<String, BytecodeError> {
        let len = self.u16()? as usize;
        String::from_utf8(self.take(len)?.to_vec())
            .map_err(|_| BytecodeError::InvalidBytecode("invalid UTF-8 string".into()))
    }

    fn is_finished(&self) -> bool {
        self.pos == self.bytes.len()
    }
}
