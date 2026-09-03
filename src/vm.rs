use crate::bytecode::{CompiledProcedure, CompiledScript, Instruction, Program};
use crate::model::{Asset, BinaryOp, Event, Expr, Stage, UnaryOp, Value};
use std::collections::{HashMap, HashSet};

const MAX_STEPS_PER_UPDATE: usize = 4096;
const MAX_REPEAT_COUNT: u64 = 1_000_000;

#[derive(Debug, Clone, PartialEq)]
pub struct PenSegment {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub color: [u8; 4],
    pub width: f32,
}

#[derive(Debug, Clone)]
pub struct RuntimeSprite {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub direction: f32,
    pub size: f32,
    pub color: [u8; 4],
    pub costume: Option<String>,
    variables: HashMap<String, Value>,
    lists: HashMap<String, Vec<Value>>,
    scripts: Vec<CompiledScript>,
    procedures: Vec<CompiledProcedure>,
    pen_down: bool,
}

#[derive(Debug, Clone)]
struct Thread {
    sprite: usize,
    frames: Vec<Frame>,
    scopes: Vec<HashMap<String, Value>>,
    wait_remaining: f32,
    done: bool,
}

#[derive(Debug, Clone)]
struct Frame {
    code: Vec<Instruction>,
    pc: usize,
    kind: FrameKind,
}

#[derive(Debug, Clone)]
enum FrameKind {
    Plain,
    Repeat { remaining: u64 },
    While { condition: Expr },
    Procedure,
}

#[derive(Debug, Clone)]
pub struct Runtime {
    name: String,
    stage: Stage,
    assets: Vec<Asset>,
    globals: HashMap<String, Value>,
    global_lists: HashMap<String, Vec<Value>>,
    sprites: Vec<RuntimeSprite>,
    threads: Vec<Thread>,
    keys: HashSet<String>,
    pen_segments: Vec<PenSegment>,
    audio_events: Vec<String>,
}

impl Runtime {
    pub fn new(program: Program) -> Self {
        let mut runtime = Self {
            name: program.name,
            stage: program.stage,
            assets: program.assets,
            globals: program
                .globals
                .into_iter()
                .map(|entry| (entry.name, entry.value))
                .collect(),
            global_lists: program
                .lists
                .into_iter()
                .map(|entry| (entry.name, entry.items))
                .collect(),
            sprites: program
                .sprites
                .into_iter()
                .map(|sprite| RuntimeSprite {
                    id: sprite.id,
                    name: sprite.name,
                    x: sprite.x,
                    y: sprite.y,
                    direction: normalize_degrees(sprite.direction),
                    size: sprite.size,
                    color: sprite.color,
                    costume: sprite.costume,
                    variables: sprite
                        .variables
                        .into_iter()
                        .map(|entry| (entry.name, entry.value))
                        .collect(),
                    lists: sprite
                        .lists
                        .into_iter()
                        .map(|entry| (entry.name, entry.items))
                        .collect(),
                    scripts: sprite.scripts,
                    procedures: sprite.procedures,
                    pen_down: false,
                })
                .collect(),
            threads: Vec::new(),
            keys: HashSet::new(),
            pen_segments: Vec::new(),
            audio_events: Vec::new(),
        };
        runtime.spawn_start_events();
        runtime
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    pub fn assets(&self) -> &[Asset] {
        &self.assets
    }

    pub fn sprites(&self) -> &[RuntimeSprite] {
        &self.sprites
    }

    pub fn pen_segments(&self) -> &[PenSegment] {
        &self.pen_segments
    }

    pub fn global_value(&self, name: &str) -> Option<&Value> {
        self.globals.get(name)
    }

    pub fn sprite_value(&self, sprite: usize, name: &str) -> Option<&Value> {
        self.sprites.get(sprite)?.variables.get(name)
    }

    pub fn sprite_list_len(&self, sprite: usize, name: &str) -> Option<usize> {
        self.sprites.get(sprite)?.lists.get(name).map(Vec::len)
    }

    pub fn is_finished(&self) -> bool {
        self.threads.is_empty()
    }

    pub fn set_key(&mut self, key: &str, down: bool) {
        let key = normalize_key(key);
        if down {
            if self.keys.insert(key.clone()) {
                self.spawn_key_events(&key);
            }
        } else {
            self.keys.remove(&key);
        }
    }

    pub fn take_audio_events(&mut self) -> Vec<String> {
        std::mem::take(&mut self.audio_events)
    }

    pub fn update(&mut self, dt: f32) {
        let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
        let active = std::mem::take(&mut self.threads);
        for mut thread in active {
            if thread.wait_remaining > 0.0 {
                thread.wait_remaining = (thread.wait_remaining - dt).max(0.0);
                if thread.wait_remaining > 0.0 {
                    self.threads.push(thread);
                    continue;
                }
            }
            self.run_thread(&mut thread);
            if !thread.done {
                self.threads.push(thread);
            }
        }
    }

    fn run_thread(&mut self, thread: &mut Thread) {
        let mut steps = 0usize;
        while steps < MAX_STEPS_PER_UPDATE {
            if thread.frames.is_empty() {
                thread.done = true;
                return;
            }

            let top = thread.frames.len() - 1;
            if thread.frames[top].pc >= thread.frames[top].code.len() {
                steps += 1;
                let kind = thread.frames[top].kind.clone();
                match kind {
                    FrameKind::Plain => {
                        thread.frames.pop();
                    }
                    FrameKind::Procedure => {
                        thread.frames.pop();
                        thread.scopes.pop();
                    }
                    FrameKind::Repeat { remaining } => {
                        if remaining > 1 {
                            thread.frames[top].pc = 0;
                            thread.frames[top].kind = FrameKind::Repeat {
                                remaining: remaining - 1,
                            };
                        } else {
                            thread.frames.pop();
                        }
                    }
                    FrameKind::While { condition } => {
                        if truthy(&self.eval_expr(thread, &condition)) {
                            thread.frames[top].pc = 0;
                        } else {
                            thread.frames.pop();
                        }
                    }
                }
                continue;
            }

            let instruction = {
                let frame = &mut thread.frames[top];
                let instruction = frame.code[frame.pc].clone();
                frame.pc += 1;
                instruction
            };
            steps += 1;

            match instruction {
                Instruction::Move { value } => {
                    let distance = as_number(&self.eval_expr(thread, &value)) as f32;
                    let (from, to, color, pen_down) = {
                        let sprite = &mut self.sprites[thread.sprite];
                        let from = (sprite.x, sprite.y);
                        let radians = sprite.direction.to_radians();
                        sprite.x += radians.cos() * distance;
                        sprite.y += radians.sin() * distance;
                        (from, (sprite.x, sprite.y), sprite.color, sprite.pen_down)
                    };
                    if pen_down && from != to {
                        self.pen_segments.push(PenSegment {
                            from,
                            to,
                            color,
                            width: 2.0,
                        });
                    }
                }
                Instruction::Turn { value } => {
                    let degrees = as_number(&self.eval_expr(thread, &value)) as f32;
                    let sprite = &mut self.sprites[thread.sprite];
                    sprite.direction = normalize_degrees(sprite.direction + degrees);
                }
                Instruction::Wait { value } => {
                    let seconds = as_number(&self.eval_expr(thread, &value)).max(0.0) as f32;
                    if seconds > 0.0 {
                        thread.wait_remaining = seconds;
                        return;
                    }
                }
                Instruction::Repeat { times, body } => {
                    let count = as_number(&self.eval_expr(thread, &times))
                        .floor()
                        .max(0.0)
                        .min(MAX_REPEAT_COUNT as f64) as u64;
                    if count > 0 {
                        thread.frames.push(Frame {
                            code: body,
                            pc: 0,
                            kind: FrameKind::Repeat { remaining: count },
                        });
                    }
                }
                Instruction::While { condition, body } => {
                    if truthy(&self.eval_expr(thread, &condition)) {
                        thread.frames.push(Frame {
                            code: body,
                            pc: 0,
                            kind: FrameKind::While { condition },
                        });
                    }
                }
                Instruction::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let body = if truthy(&self.eval_expr(thread, &condition)) {
                        then_body
                    } else {
                        else_body
                    };
                    if !body.is_empty() {
                        thread.frames.push(Frame {
                            code: body,
                            pc: 0,
                            kind: FrameKind::Plain,
                        });
                    }
                }
                Instruction::Set { name, value } => {
                    let value = self.eval_expr(thread, &value);
                    self.set_value(thread, &name, value);
                }
                Instruction::Change { name, delta } => {
                    let current = self.resolve_value(thread, &name);
                    let value = Value::Number(
                        as_number(&current) + as_number(&self.eval_expr(thread, &delta)),
                    );
                    self.set_value(thread, &name, value);
                }
                Instruction::Push { list, value } => {
                    let value = self.eval_expr(thread, &value);
                    self.push_list(thread.sprite, &list, value);
                }
                Instruction::Broadcast { message } => self.spawn_message_events(&message),
                Instruction::Call { name, args } => {
                    let procedure = self.sprites[thread.sprite]
                        .procedures
                        .iter()
                        .find(|procedure| procedure.name == name)
                        .cloned();
                    if let Some(procedure) = procedure {
                        let values = args
                            .iter()
                            .map(|arg| self.eval_expr(thread, arg))
                            .collect::<Vec<_>>();
                        let scope = procedure
                            .params
                            .iter()
                            .enumerate()
                            .map(|(index, param)| {
                                (
                                    param.clone(),
                                    values.get(index).cloned().unwrap_or(Value::Number(0.0)),
                                )
                            })
                            .collect();
                        thread.scopes.push(scope);
                        thread.frames.push(Frame {
                            code: procedure.instructions,
                            pc: 0,
                            kind: FrameKind::Procedure,
                        });
                    }
                }
                Instruction::PenDown => self.sprites[thread.sprite].pen_down = true,
                Instruction::PenUp => self.sprites[thread.sprite].pen_down = false,
                Instruction::PenClear => self.pen_segments.clear(),
                Instruction::Play { sound } => self.audio_events.push(sound),
            }
        }
    }

    fn eval_expr(&self, thread: &Thread, expr: &Expr) -> Value {
        match expr {
            Expr::Literal { value } => value.clone(),
            Expr::Var { name } => self.resolve_value(thread, name),
            Expr::Key { key } => Value::Bool(self.keys.contains(&normalize_key(key))),
            Expr::Touching { sprite } => Value::Bool(self.touching(thread.sprite, sprite)),
            Expr::ListLen { name } => Value::Number(self.list_len(thread.sprite, name) as f64),
            Expr::Unary { op, value } => {
                let value = self.eval_expr(thread, value);
                match op {
                    UnaryOp::Neg => Value::Number(-as_number(&value)),
                    UnaryOp::Not => Value::Bool(!truthy(&value)),
                }
            }
            Expr::Binary { op, left, right } => {
                if *op == BinaryOp::And {
                    let left = self.eval_expr(thread, left);
                    if !truthy(&left) {
                        return Value::Bool(false);
                    }
                    return Value::Bool(truthy(&self.eval_expr(thread, right)));
                }
                if *op == BinaryOp::Or {
                    let left = self.eval_expr(thread, left);
                    if truthy(&left) {
                        return Value::Bool(true);
                    }
                    return Value::Bool(truthy(&self.eval_expr(thread, right)));
                }
                let left = self.eval_expr(thread, left);
                let right = self.eval_expr(thread, right);
                eval_binary(*op, left, right)
            }
        }
    }

    fn resolve_value(&self, thread: &Thread, name: &str) -> Value {
        for scope in thread.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return value.clone();
            }
        }
        if let Some(value) = self.sprites[thread.sprite].variables.get(name) {
            return value.clone();
        }
        self.globals
            .get(name)
            .cloned()
            .unwrap_or(Value::Number(0.0))
    }

    fn set_value(&mut self, thread: &mut Thread, name: &str, value: Value) {
        for scope in thread.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return;
            }
        }
        if self.sprites[thread.sprite].variables.contains_key(name) {
            self.sprites[thread.sprite]
                .variables
                .insert(name.to_string(), value);
            return;
        }
        if self.globals.contains_key(name) {
            self.globals.insert(name.to_string(), value);
            return;
        }
        self.sprites[thread.sprite]
            .variables
            .insert(name.to_string(), value);
    }

    fn push_list(&mut self, sprite: usize, name: &str, value: Value) {
        if let Some(list) = self.sprites[sprite].lists.get_mut(name) {
            list.push(value);
        } else if let Some(list) = self.global_lists.get_mut(name) {
            list.push(value);
        } else {
            self.sprites[sprite]
                .lists
                .insert(name.to_string(), vec![value]);
        }
    }

    fn list_len(&self, sprite: usize, name: &str) -> usize {
        self.sprites[sprite]
            .lists
            .get(name)
            .or_else(|| self.global_lists.get(name))
            .map(Vec::len)
            .unwrap_or(0)
    }

    fn touching(&self, sprite: usize, target_name: &str) -> bool {
        let Some(current) = self.sprites.get(sprite) else {
            return false;
        };
        self.sprites.iter().enumerate().any(|(index, target)| {
            index != sprite
                && target.name == target_name
                && (current.x - target.x).abs() * 2.0 <= current.size + target.size
                && (current.y - target.y).abs() * 2.0 <= current.size + target.size
        })
    }

    fn spawn_start_events(&mut self) {
        for sprite in 0..self.sprites.len() {
            let codes = self.sprites[sprite]
                .scripts
                .iter()
                .filter(|script| matches!(script.event, Event::Start))
                .map(|script| script.instructions.clone())
                .collect::<Vec<_>>();
            for code in codes {
                self.spawn_thread(sprite, code);
            }
        }
    }

    fn spawn_key_events(&mut self, key: &str) {
        for sprite in 0..self.sprites.len() {
            let codes = self.sprites[sprite]
                .scripts
                .iter()
                .filter_map(|script| match &script.event {
                    Event::Key { key: event_key } if normalize_key(event_key) == key => {
                        Some(script.instructions.clone())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            for code in codes {
                self.spawn_thread(sprite, code);
            }
        }
    }

    fn spawn_message_events(&mut self, message: &str) {
        for sprite in 0..self.sprites.len() {
            let codes = self.sprites[sprite]
                .scripts
                .iter()
                .filter_map(|script| match &script.event {
                    Event::Message {
                        message: event_message,
                    } if event_message == message => Some(script.instructions.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for code in codes {
                self.spawn_thread(sprite, code);
            }
        }
    }

    fn spawn_thread(&mut self, sprite: usize, code: Vec<Instruction>) {
        self.threads.push(Thread {
            sprite,
            frames: vec![Frame {
                code,
                pc: 0,
                kind: FrameKind::Plain,
            }],
            scopes: Vec::new(),
            wait_remaining: 0.0,
            done: false,
        });
    }
}

fn normalize_key(key: &str) -> String {
    key.trim().to_ascii_lowercase()
}

fn normalize_degrees(value: f32) -> f32 {
    value.rem_euclid(360.0)
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::Number(value) => *value != 0.0 && !value.is_nan(),
        Value::String(value) => !value.is_empty(),
    }
}

fn as_number(value: &Value) -> f64 {
    match value {
        Value::Number(value) => *value,
        Value::Bool(value) => u8::from(*value) as f64,
        Value::String(value) => value.parse().unwrap_or(0.0),
    }
}

fn as_string(value: &Value) -> String {
    match value {
        Value::Number(value) if value.fract() == 0.0 => format!("{value:.0}"),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::String(value) => value.clone(),
    }
}

fn eval_binary(op: BinaryOp, left: Value, right: Value) -> Value {
    match op {
        BinaryOp::Add => {
            if matches!(left, Value::String(_)) || matches!(right, Value::String(_)) {
                Value::String(format!("{}{}", as_string(&left), as_string(&right)))
            } else {
                Value::Number(as_number(&left) + as_number(&right))
            }
        }
        BinaryOp::Sub => Value::Number(as_number(&left) - as_number(&right)),
        BinaryOp::Mul => Value::Number(as_number(&left) * as_number(&right)),
        BinaryOp::Div => {
            let divisor = as_number(&right);
            Value::Number(if divisor == 0.0 {
                0.0
            } else {
                as_number(&left) / divisor
            })
        }
        BinaryOp::Mod => {
            let divisor = as_number(&right);
            Value::Number(if divisor == 0.0 {
                0.0
            } else {
                as_number(&left) % divisor
            })
        }
        BinaryOp::Eq => Value::Bool(left == right),
        BinaryOp::Ne => Value::Bool(left != right),
        BinaryOp::Lt => Value::Bool(as_number(&left) < as_number(&right)),
        BinaryOp::Le => Value::Bool(as_number(&left) <= as_number(&right)),
        BinaryOp::Gt => Value::Bool(as_number(&left) > as_number(&right)),
        BinaryOp::Ge => Value::Bool(as_number(&left) >= as_number(&right)),
        BinaryOp::And => Value::Bool(truthy(&left) && truthy(&right)),
        BinaryOp::Or => Value::Bool(truthy(&left) || truthy(&right)),
    }
}
