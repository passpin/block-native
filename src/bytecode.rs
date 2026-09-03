use crate::model::{Command, Project, Stage, PROJECT_VERSION};
use thiserror::Error;

const MAGIC: &[u8; 4] = b"BLK1";
const BYTECODE_VERSION: u16 = 1;
pub const MAX_INSTRUCTIONS_PER_SPRITE: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub name: String,
    pub stage: Stage,
    pub sprites: Vec<CompiledSprite>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledSprite {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub direction: f32,
    pub size: f32,
    pub color: [u8; 4],
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instruction {
    Move(f32),
    Turn(f32),
    Wait(f32),
}

#[derive(Debug, Error)]
pub enum BytecodeError {
    #[error("invalid project: {0}")]
    InvalidProject(String),
    #[error("script expands beyond {MAX_INSTRUCTIONS_PER_SPRITE} instructions")]
    ExpansionLimit,
    #[error("invalid bytecode: {0}")]
    InvalidBytecode(String),
}

pub fn compile(project: &Project) -> Result<Vec<u8>, BytecodeError> {
    validate_project(project)?;

    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    push_u16(&mut out, BYTECODE_VERSION);
    push_string(&mut out, &project.name)?;
    push_u16(&mut out, project.stage.width);
    push_u16(&mut out, project.stage.height);
    out.extend_from_slice(&project.stage.background);

    let sprite_count = u16::try_from(project.sprites.len())
        .map_err(|_| BytecodeError::InvalidProject("too many sprites".into()))?;
    push_u16(&mut out, sprite_count);

    for sprite in &project.sprites {
        let instruction_count = count_commands(&sprite.script)?;
        if instruction_count > MAX_INSTRUCTIONS_PER_SPRITE {
            return Err(BytecodeError::ExpansionLimit);
        }

        let mut instructions = Vec::with_capacity(instruction_count);
        flatten_commands(&sprite.script, &mut instructions);

        push_string(&mut out, &sprite.name)?;
        push_f32(&mut out, sprite.x);
        push_f32(&mut out, sprite.y);
        push_f32(&mut out, sprite.direction);
        push_f32(&mut out, sprite.size);
        out.extend_from_slice(&sprite.color);
        push_u32(&mut out, instructions.len() as u32);

        for instruction in instructions {
            match instruction {
                Instruction::Move(value) => {
                    out.push(1);
                    push_f32(&mut out, value);
                }
                Instruction::Turn(value) => {
                    out.push(2);
                    push_f32(&mut out, value);
                }
                Instruction::Wait(value) => {
                    out.push(3);
                    push_f32(&mut out, value);
                }
            }
        }
    }

    Ok(out)
}

pub fn decode(bytes: &[u8]) -> Result<Program, BytecodeError> {
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != MAGIC {
        return Err(BytecodeError::InvalidBytecode("bad magic".into()));
    }

    let version = reader.u16()?;
    if version != BYTECODE_VERSION {
        return Err(BytecodeError::InvalidBytecode(format!(
            "unsupported bytecode version {version}"
        )));
    }

    let name = reader.string()?;
    let width = reader.u16()?;
    let height = reader.u16()?;
    if width == 0 || height == 0 {
        return Err(BytecodeError::InvalidBytecode(
            "stage dimensions must be positive".into(),
        ));
    }
    let background = reader.rgba()?;
    let sprite_count = reader.u16()? as usize;
    let mut sprites = Vec::with_capacity(sprite_count);

    for _ in 0..sprite_count {
        let sprite_name = reader.string()?;
        let x = reader.f32()?;
        let y = reader.f32()?;
        let direction = reader.f32()?;
        let size = reader.f32()?;
        if !x.is_finite() || !y.is_finite() || !direction.is_finite() || !size.is_finite() {
            return Err(BytecodeError::InvalidBytecode(
                "sprite contains non-finite numeric state".into(),
            ));
        }
        if size <= 0.0 {
            return Err(BytecodeError::InvalidBytecode(
                "sprite size must be positive".into(),
            ));
        }
        let color = reader.rgba()?;
        let instruction_count = reader.u32()? as usize;
        if instruction_count > MAX_INSTRUCTIONS_PER_SPRITE {
            return Err(BytecodeError::InvalidBytecode(
                "instruction count exceeds runtime limit".into(),
            ));
        }

        let mut instructions = Vec::with_capacity(instruction_count);
        for _ in 0..instruction_count {
            let opcode = reader.u8()?;
            let value = reader.f32()?;
            if !value.is_finite() {
                return Err(BytecodeError::InvalidBytecode(
                    "instruction contains non-finite value".into(),
                ));
            }
            let instruction = match opcode {
                1 => Instruction::Move(value),
                2 => Instruction::Turn(value),
                3 if value >= 0.0 => Instruction::Wait(value),
                3 => {
                    return Err(BytecodeError::InvalidBytecode(
                        "wait duration cannot be negative".into(),
                    ))
                }
                other => {
                    return Err(BytecodeError::InvalidBytecode(format!(
                        "unknown opcode {other}"
                    )))
                }
            };
            instructions.push(instruction);
        }

        sprites.push(CompiledSprite {
            name: sprite_name,
            x,
            y,
            direction,
            size,
            color,
            instructions,
        });
    }

    if !reader.is_finished() {
        return Err(BytecodeError::InvalidBytecode(
            "trailing bytes after program".into(),
        ));
    }

    Ok(Program {
        name,
        stage: Stage {
            width,
            height,
            background,
        },
        sprites,
    })
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
    if project.sprites.len() > u16::MAX as usize {
        return Err(BytecodeError::InvalidProject("too many sprites".into()));
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
        {
            return Err(BytecodeError::InvalidProject(format!(
                "sprite '{}' contains a non-finite numeric value",
                sprite.name
            )));
        }
        if sprite.size <= 0.0 {
            return Err(BytecodeError::InvalidProject(format!(
                "sprite '{}' size must be positive",
                sprite.name
            )));
        }
        validate_commands(&sprite.script, &sprite.name)?;
    }

    Ok(())
}

fn validate_commands(commands: &[Command], sprite_name: &str) -> Result<(), BytecodeError> {
    for command in commands {
        match command {
            Command::Move { steps, .. } if !steps.is_finite() => {
                return Err(non_finite_command(sprite_name))
            }
            Command::Turn { degrees, .. } if !degrees.is_finite() => {
                return Err(non_finite_command(sprite_name))
            }
            Command::Wait { seconds, .. } if !seconds.is_finite() || *seconds < 0.0 => {
                return Err(BytecodeError::InvalidProject(format!(
                    "sprite '{sprite_name}' has an invalid wait duration"
                )))
            }
            Command::Repeat { body, .. } => validate_commands(body, sprite_name)?,
            _ => {}
        }
    }
    Ok(())
}

fn non_finite_command(sprite_name: &str) -> BytecodeError {
    BytecodeError::InvalidProject(format!(
        "sprite '{sprite_name}' has a non-finite command value"
    ))
}

fn count_commands(commands: &[Command]) -> Result<usize, BytecodeError> {
    let mut total = 0usize;
    for command in commands {
        let add = match command {
            Command::Move { .. } | Command::Turn { .. } | Command::Wait { .. } => 1,
            Command::Repeat { times, body, .. } => {
                let inner = count_commands(body)?;
                inner
                    .checked_mul(*times as usize)
                    .ok_or(BytecodeError::ExpansionLimit)?
            }
        };
        total = total
            .checked_add(add)
            .ok_or(BytecodeError::ExpansionLimit)?;
        if total > MAX_INSTRUCTIONS_PER_SPRITE {
            return Err(BytecodeError::ExpansionLimit);
        }
    }
    Ok(total)
}

fn flatten_commands(commands: &[Command], out: &mut Vec<Instruction>) {
    for command in commands {
        match command {
            Command::Move { steps, .. } => out.push(Instruction::Move(*steps)),
            Command::Turn { degrees, .. } => out.push(Instruction::Turn(*degrees)),
            Command::Wait { seconds, .. } => out.push(Instruction::Wait(*seconds)),
            Command::Repeat { times, body, .. } => {
                for _ in 0..*times {
                    flatten_commands(body, out);
                }
            }
        }
    }
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_string(out: &mut Vec<u8>, value: &str) -> Result<(), BytecodeError> {
    let len = u16::try_from(value.len())
        .map_err(|_| BytecodeError::InvalidProject("name is too long".into()))?;
    push_u16(out, len);
    out.extend_from_slice(value.as_bytes());
    Ok(())
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
        let bytes: [u8; 2] = self.take(2)?.try_into().unwrap();
        Ok(u16::from_le_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, BytecodeError> {
        let bytes: [u8; 4] = self.take(4)?.try_into().unwrap();
        Ok(u32::from_le_bytes(bytes))
    }

    fn f32(&mut self) -> Result<f32, BytecodeError> {
        let bytes: [u8; 4] = self.take(4)?.try_into().unwrap();
        Ok(f32::from_le_bytes(bytes))
    }

    fn rgba(&mut self) -> Result<[u8; 4], BytecodeError> {
        Ok(self.take(4)?.try_into().unwrap())
    }

    fn string(&mut self) -> Result<String, BytecodeError> {
        let len = self.u16()? as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| BytecodeError::InvalidBytecode("invalid UTF-8 string".into()))
    }

    fn is_finished(&self) -> bool {
        self.pos == self.bytes.len()
    }
}
