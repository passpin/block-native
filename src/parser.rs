use crate::model::{
    Asset, AssetKind, BinaryOp, Command, Event, Expr, ListDecl, Procedure, Project, Script, Sprite,
    Stage, UnaryOp, Value, Variable, PROJECT_VERSION,
};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Word(String),
    String(String),
    Number(f64),
    Color([u8; 4]),
    Symbol(char),
    Op(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    line: usize,
    column: usize,
}

pub fn parse_project(source: &str) -> Result<Project, ParseError> {
    Parser::new(tokenize(source)?).parse_project()
}

pub fn format_project(project: &Project) -> String {
    let mut out = String::new();
    push_line(&mut out, 0, &format!("project {} {{", quote(&project.name)));
    push_line(
        &mut out,
        1,
        &format!(
            "stage {} {} background {}",
            project.stage.width,
            project.stage.height,
            format_color(project.stage.background)
        ),
    );
    for variable in &project.globals {
        push_line(
            &mut out,
            1,
            &format!(
                "global {} = {}",
                variable.name,
                format_value(&variable.value)
            ),
        );
    }
    for list in &project.lists {
        push_line(
            &mut out,
            1,
            &format!("list {} = {}", list.name, format_list(&list.items)),
        );
    }
    for asset in &project.assets {
        push_line(
            &mut out,
            1,
            &format!(
                "asset {} {} = {}",
                match asset.kind {
                    AssetKind::Image => "image",
                    AssetKind::Sound => "sound",
                },
                quote(&asset.name),
                quote(&asset.path)
            ),
        );
    }
    for sprite in &project.sprites {
        let mut header = format!(
            "sprite {} at {} {} direction {} size {} color {}",
            quote(&sprite.name),
            fmt_num(sprite.x as f64),
            fmt_num(sprite.y as f64),
            fmt_num(sprite.direction as f64),
            fmt_num(sprite.size as f64),
            format_color(sprite.color)
        );
        if let Some(costume) = &sprite.costume {
            header.push_str(&format!(" costume {}", quote(costume)));
        }
        header.push_str(" {");
        push_line(&mut out, 1, &header);
        for variable in &sprite.variables {
            push_line(
                &mut out,
                2,
                &format!("var {} = {}", variable.name, format_value(&variable.value)),
            );
        }
        for list in &sprite.lists {
            push_line(
                &mut out,
                2,
                &format!("list {} = {}", list.name, format_list(&list.items)),
            );
        }
        for script in &sprite.scripts {
            let event = match &script.event {
                Event::Start => "when start".to_string(),
                Event::Key { key } => format!("when key {}", quote(key)),
                Event::Message { message } => format!("when message {}", quote(message)),
            };
            push_line(&mut out, 2, &format!("{event} {{"));
            format_commands(&script.body, 3, &mut out);
            push_line(&mut out, 2, "}");
        }
        for procedure in &sprite.procedures {
            push_line(
                &mut out,
                2,
                &format!(
                    "proc {}({}) {{",
                    procedure.name,
                    procedure.params.join(", ")
                ),
            );
            format_commands(&procedure.body, 3, &mut out);
            push_line(&mut out, 2, "}");
        }
        push_line(&mut out, 1, "}");
    }
    out.push('}');
    out
}

fn format_commands(commands: &[Command], depth: usize, out: &mut String) {
    for command in commands {
        match command {
            Command::Move { steps, .. } => {
                push_line(out, depth, &format!("move {}", format_expr(steps)))
            }
            Command::Turn { degrees, .. } => {
                push_line(out, depth, &format!("turn {}", format_expr(degrees)))
            }
            Command::Wait { seconds, .. } => {
                push_line(out, depth, &format!("wait {}", format_expr(seconds)))
            }
            Command::Set { name, value, .. } => {
                push_line(out, depth, &format!("set {name} = {}", format_expr(value)))
            }
            Command::Change { name, delta, .. } => push_line(
                out,
                depth,
                &format!("change {name} by {}", format_expr(delta)),
            ),
            Command::Push { list, value, .. } => push_line(
                out,
                depth,
                &format!("push {} to {list}", format_expr(value)),
            ),
            Command::Broadcast { message, .. } => {
                push_line(out, depth, &format!("broadcast {}", quote(message)))
            }
            Command::Call { name, args, .. } => push_line(
                out,
                depth,
                &format!(
                    "call {name}({})",
                    args.iter().map(format_expr).collect::<Vec<_>>().join(", ")
                ),
            ),
            Command::PenDown { .. } => push_line(out, depth, "pen down"),
            Command::PenUp { .. } => push_line(out, depth, "pen up"),
            Command::PenClear { .. } => push_line(out, depth, "pen clear"),
            Command::Play { sound, .. } => push_line(out, depth, &format!("play {}", quote(sound))),
            Command::Repeat { times, body, .. } => {
                push_line(out, depth, &format!("repeat {} {{", format_expr(times)));
                format_commands(body, depth + 1, out);
                push_line(out, depth, "}");
            }
            Command::While {
                condition, body, ..
            } => {
                push_line(out, depth, &format!("while {} {{", format_expr(condition)));
                format_commands(body, depth + 1, out);
                push_line(out, depth, "}");
            }
            Command::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                push_line(out, depth, &format!("if {} {{", format_expr(condition)));
                format_commands(then_body, depth + 1, out);
                if else_body.is_empty() {
                    push_line(out, depth, "}");
                } else {
                    push_line(out, depth, "} else {");
                    format_commands(else_body, depth + 1, out);
                    push_line(out, depth, "}");
                }
            }
        }
    }
}

fn format_expr(expr: &Expr) -> String {
    format_expr_prec(expr, 0)
}

fn format_expr_prec(expr: &Expr, parent_prec: u8) -> String {
    match expr {
        Expr::Literal { value } => format_value(value),
        Expr::Var { name } => name.clone(),
        Expr::Key { key } => format!("key({})", quote(key)),
        Expr::Touching { sprite } => format!("touching({})", quote(sprite)),
        Expr::ListLen { name } => format!("len({name})"),
        Expr::Unary { op, value } => {
            let text = match op {
                UnaryOp::Neg => format!("-{}", format_expr_prec(value, 8)),
                UnaryOp::Not => format!("not {}", format_expr_prec(value, 8)),
            };
            if 8 < parent_prec {
                format!("({text})")
            } else {
                text
            }
        }
        Expr::Binary { op, left, right } => {
            let (symbol, prec) = match op {
                BinaryOp::Or => ("or", 1),
                BinaryOp::And => ("and", 2),
                BinaryOp::Eq => ("==", 3),
                BinaryOp::Ne => ("!=", 3),
                BinaryOp::Lt => ("<", 4),
                BinaryOp::Le => ("<=", 4),
                BinaryOp::Gt => (">", 4),
                BinaryOp::Ge => (">=", 4),
                BinaryOp::Add => ("+", 5),
                BinaryOp::Sub => ("-", 5),
                BinaryOp::Mul => ("*", 6),
                BinaryOp::Div => ("/", 6),
                BinaryOp::Mod => ("%", 6),
            };
            let text = format!(
                "{} {symbol} {}",
                format_expr_prec(left, prec),
                format_expr_prec(right, prec + 1)
            );
            if prec < parent_prec {
                format!("({text})")
            } else {
                text
            }
        }
    }
}

fn push_line(out: &mut String, depth: usize, text: &str) {
    for _ in 0..depth {
        out.push_str("  ");
    }
    out.push_str(text);
    out.push('\n');
}

fn format_list(values: &[Value]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(format_value)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Number(value) => fmt_num(*value),
        Value::Bool(value) => value.to_string(),
        Value::String(value) => quote(value),
    }
}

fn fmt_num(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization cannot fail")
}

fn format_color(value: [u8; 4]) -> String {
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        value[0], value[1], value[2], value[3]
    )
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse_project(mut self) -> Result<Project, ParseError> {
        self.word("project")?;
        let name = self.string()?;
        self.symbol('{')?;
        self.word("stage")?;
        let width = self.positive_u16()?;
        let height = self.positive_u16()?;
        self.word("background")?;
        let background = self.color()?;

        let mut globals = Vec::new();
        let mut lists = Vec::new();
        let mut assets = Vec::new();
        let mut sprites = Vec::new();

        while !self.is_symbol('}') {
            match self.peek_word() {
                Some("global") => {
                    self.take();
                    globals.push(self.parse_variable()?);
                }
                Some("list") => {
                    self.take();
                    lists.push(self.parse_list()?);
                }
                Some("asset") => {
                    self.take();
                    assets.push(self.parse_asset()?);
                }
                Some("sprite") => sprites.push(self.parse_sprite()?),
                _ => return Err(self.error("expected global, list, asset, sprite, or '}'")),
            }
        }
        self.symbol('}')?;
        self.eof()?;
        if name.trim().is_empty() {
            return Err(self.error("project name cannot be empty"));
        }
        Ok(Project {
            version: PROJECT_VERSION,
            name,
            stage: Stage {
                width,
                height,
                background,
            },
            globals,
            lists,
            assets,
            sprites,
        })
    }

    fn parse_variable(&mut self) -> Result<Variable, ParseError> {
        let name = self.ident()?;
        self.op("=")?;
        let value = self.literal()?;
        Ok(Variable { name, value })
    }

    fn parse_list(&mut self) -> Result<ListDecl, ParseError> {
        let name = self.ident()?;
        self.op("=")?;
        self.symbol('[')?;
        let mut items = Vec::new();
        if !self.is_symbol(']') {
            loop {
                items.push(self.literal()?);
                if self.is_symbol(']') {
                    break;
                }
                self.symbol(',')?;
            }
        }
        self.symbol(']')?;
        Ok(ListDecl { name, items })
    }

    fn parse_asset(&mut self) -> Result<Asset, ParseError> {
        let kind = match self.ident()?.as_str() {
            "image" => AssetKind::Image,
            "sound" => AssetKind::Sound,
            _ => return Err(self.error("asset kind must be image or sound")),
        };
        let name = self.string()?;
        self.op("=")?;
        let path = self.string()?;
        Ok(Asset { kind, name, path })
    }

    fn parse_sprite(&mut self) -> Result<Sprite, ParseError> {
        self.word("sprite")?;
        let name = self.string()?;
        self.word("at")?;
        let x = self.number()? as f32;
        let y = self.number()? as f32;
        self.word("direction")?;
        let direction = self.number()? as f32;
        self.word("size")?;
        let size = self.number()? as f32;
        if size <= 0.0 {
            return Err(self.error("sprite size must be positive"));
        }
        self.word("color")?;
        let color = self.color()?;
        let costume = if self.peek_word() == Some("costume") {
            self.take();
            Some(self.string()?)
        } else {
            None
        };
        self.symbol('{')?;

        let mut variables = Vec::new();
        let mut lists = Vec::new();
        let mut scripts = Vec::new();
        let mut procedures = Vec::new();
        while !self.is_symbol('}') {
            match self.peek_word() {
                Some("var") => {
                    self.take();
                    variables.push(self.parse_variable()?);
                }
                Some("list") => {
                    self.take();
                    lists.push(self.parse_list()?);
                }
                Some("when") => scripts.push(self.parse_script()?),
                Some("proc") => procedures.push(self.parse_procedure()?),
                _ => return Err(self.error("expected var, list, when, proc, or '}'")),
            }
        }
        self.symbol('}')?;
        Ok(Sprite {
            id: String::new(),
            name,
            x,
            y,
            direction,
            size,
            color,
            costume,
            variables,
            lists,
            scripts,
            procedures,
        })
    }

    fn parse_script(&mut self) -> Result<Script, ParseError> {
        self.word("when")?;
        let event = match self.ident()?.as_str() {
            "start" => Event::Start,
            "key" => Event::Key {
                key: self.string()?,
            },
            "message" => Event::Message {
                message: self.string()?,
            },
            _ => return Err(self.error("event must be start, key, or message")),
        };
        self.symbol('{')?;
        let body = self.parse_commands()?;
        self.symbol('}')?;
        Ok(Script {
            id: String::new(),
            event,
            body,
        })
    }

    fn parse_procedure(&mut self) -> Result<Procedure, ParseError> {
        self.word("proc")?;
        let name = self.ident()?;
        self.symbol('(')?;
        let mut params = Vec::new();
        if !self.is_symbol(')') {
            loop {
                params.push(self.ident()?);
                if self.is_symbol(')') {
                    break;
                }
                self.symbol(',')?;
            }
        }
        self.symbol(')')?;
        self.symbol('{')?;
        let body = self.parse_commands()?;
        self.symbol('}')?;
        Ok(Procedure {
            id: String::new(),
            name,
            params,
            body,
        })
    }

    fn parse_commands(&mut self) -> Result<Vec<Command>, ParseError> {
        let mut body = Vec::new();
        while !self.is_symbol('}') {
            if matches!(self.current().kind, TokenKind::Eof) {
                return Err(self.error("expected '}' before end of source"));
            }
            body.push(self.parse_command()?);
        }
        Ok(body)
    }

    fn parse_command(&mut self) -> Result<Command, ParseError> {
        let op = self.ident()?;
        let id = String::new();
        match op.as_str() {
            "move" => Ok(Command::Move {
                id,
                steps: self.expr(0)?,
            }),
            "turn" => Ok(Command::Turn {
                id,
                degrees: self.expr(0)?,
            }),
            "wait" => Ok(Command::Wait {
                id,
                seconds: self.expr(0)?,
            }),
            "set" => {
                let name = self.ident()?;
                self.op("=")?;
                Ok(Command::Set {
                    id,
                    name,
                    value: self.expr(0)?,
                })
            }
            "change" => {
                let name = self.ident()?;
                self.word("by")?;
                Ok(Command::Change {
                    id,
                    name,
                    delta: self.expr(0)?,
                })
            }
            "push" => {
                let value = self.expr(0)?;
                self.word("to")?;
                let list = self.ident()?;
                Ok(Command::Push { id, list, value })
            }
            "broadcast" => Ok(Command::Broadcast {
                id,
                message: self.string()?,
            }),
            "call" => {
                let name = self.ident()?;
                self.symbol('(')?;
                let mut args = Vec::new();
                if !self.is_symbol(')') {
                    loop {
                        args.push(self.expr(0)?);
                        if self.is_symbol(')') {
                            break;
                        }
                        self.symbol(',')?;
                    }
                }
                self.symbol(')')?;
                Ok(Command::Call { id, name, args })
            }
            "pen" => match self.ident()?.as_str() {
                "down" => Ok(Command::PenDown { id }),
                "up" => Ok(Command::PenUp { id }),
                "clear" => Ok(Command::PenClear { id }),
                _ => Err(self.error("pen command must be down, up, or clear")),
            },
            "play" => Ok(Command::Play {
                id,
                sound: self.string()?,
            }),
            "repeat" => {
                let times = self.expr(0)?;
                self.symbol('{')?;
                let body = self.parse_commands()?;
                self.symbol('}')?;
                Ok(Command::Repeat { id, times, body })
            }
            "while" => {
                let condition = self.expr(0)?;
                self.symbol('{')?;
                let body = self.parse_commands()?;
                self.symbol('}')?;
                Ok(Command::While {
                    id,
                    condition,
                    body,
                })
            }
            "if" => {
                let condition = self.expr(0)?;
                self.symbol('{')?;
                let then_body = self.parse_commands()?;
                self.symbol('}')?;
                let else_body = if self.peek_word() == Some("else") {
                    self.take();
                    self.symbol('{')?;
                    let body = self.parse_commands()?;
                    self.symbol('}')?;
                    body
                } else {
                    Vec::new()
                };
                Ok(Command::If {
                    id,
                    condition,
                    then_body,
                    else_body,
                })
            }
            _ => Err(self.error(format!("unknown command '{op}'"))),
        }
    }

    fn expr(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut left = self.prefix()?;
        loop {
            let Some((op, prec)) = self.binary_op() else {
                break;
            };
            if prec < min_prec {
                break;
            }
            self.take();
            let right = self.expr(prec + 1)?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<Expr, ParseError> {
        if self.peek_word() == Some("not") {
            self.take();
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                value: Box::new(self.expr(8)?),
            });
        }
        if self.is_op("-") {
            self.take();
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                value: Box::new(self.expr(8)?),
            });
        }
        match self.current().kind.clone() {
            TokenKind::Number(value) => {
                self.take();
                Ok(Expr::Literal {
                    value: Value::Number(value),
                })
            }
            TokenKind::String(value) => {
                self.take();
                Ok(Expr::Literal {
                    value: Value::String(value),
                })
            }
            TokenKind::Word(ref value) if value == "true" || value == "false" => {
                let value = value == "true";
                self.take();
                Ok(Expr::Literal {
                    value: Value::Bool(value),
                })
            }
            TokenKind::Word(name) => {
                self.take();
                if self.is_symbol('(') {
                    self.take();
                    let result = match name.as_str() {
                        "key" => Expr::Key {
                            key: self.string()?,
                        },
                        "touching" => Expr::Touching {
                            sprite: self.string()?,
                        },
                        "len" => Expr::ListLen {
                            name: self.ident()?,
                        },
                        _ => {
                            return Err(self.error(format!("unknown expression function '{name}'")))
                        }
                    };
                    self.symbol(')')?;
                    Ok(result)
                } else {
                    Ok(Expr::Var { name })
                }
            }
            TokenKind::Symbol('(') => {
                self.take();
                let value = self.expr(0)?;
                self.symbol(')')?;
                Ok(value)
            }
            _ => Err(self.error("expected expression")),
        }
    }

    fn binary_op(&self) -> Option<(BinaryOp, u8)> {
        match &self.current().kind {
            TokenKind::Word(word) if word == "or" => Some((BinaryOp::Or, 1)),
            TokenKind::Word(word) if word == "and" => Some((BinaryOp::And, 2)),
            TokenKind::Op(op) if op == "==" => Some((BinaryOp::Eq, 3)),
            TokenKind::Op(op) if op == "!=" => Some((BinaryOp::Ne, 3)),
            TokenKind::Op(op) if op == "<" => Some((BinaryOp::Lt, 4)),
            TokenKind::Op(op) if op == "<=" => Some((BinaryOp::Le, 4)),
            TokenKind::Op(op) if op == ">" => Some((BinaryOp::Gt, 4)),
            TokenKind::Op(op) if op == ">=" => Some((BinaryOp::Ge, 4)),
            TokenKind::Op(op) if op == "+" => Some((BinaryOp::Add, 5)),
            TokenKind::Op(op) if op == "-" => Some((BinaryOp::Sub, 5)),
            TokenKind::Op(op) if op == "*" => Some((BinaryOp::Mul, 6)),
            TokenKind::Op(op) if op == "/" => Some((BinaryOp::Div, 6)),
            TokenKind::Op(op) if op == "%" => Some((BinaryOp::Mod, 6)),
            _ => None,
        }
    }

    fn literal(&mut self) -> Result<Value, ParseError> {
        match self.current().kind.clone() {
            TokenKind::Number(value) => {
                self.take();
                Ok(Value::Number(value))
            }
            TokenKind::String(value) => {
                self.take();
                Ok(Value::String(value))
            }
            TokenKind::Word(value) if value == "true" || value == "false" => {
                self.take();
                Ok(Value::Bool(value == "true"))
            }
            _ => Err(self.error("expected number, bool, or string literal")),
        }
    }

    fn positive_u16(&mut self) -> Result<u16, ParseError> {
        let value = self.number()?;
        if value.fract() != 0.0 || value <= 0.0 || value > u16::MAX as f64 {
            return Err(self.error("expected positive integer <= 65535"));
        }
        Ok(value as u16)
    }

    fn number(&mut self) -> Result<f64, ParseError> {
        match self.take().kind {
            TokenKind::Number(value) => Ok(value),
            _ => Err(self.error_prev("expected number")),
        }
    }

    fn string(&mut self) -> Result<String, ParseError> {
        match self.take().kind {
            TokenKind::String(value) => Ok(value),
            _ => Err(self.error_prev("expected string")),
        }
    }

    fn color(&mut self) -> Result<[u8; 4], ParseError> {
        match self.take().kind {
            TokenKind::Color(value) => Ok(value),
            _ => Err(self.error_prev("expected color")),
        }
    }

    fn ident(&mut self) -> Result<String, ParseError> {
        match self.take().kind {
            TokenKind::Word(value) => Ok(value),
            _ => Err(self.error_prev("expected identifier")),
        }
    }

    fn word(&mut self, expected: &str) -> Result<(), ParseError> {
        let token = self.take();
        match token.kind {
            TokenKind::Word(value) if value == expected => Ok(()),
            _ => Err(ParseError {
                message: format!("expected '{expected}'"),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn symbol(&mut self, expected: char) -> Result<(), ParseError> {
        let token = self.take();
        match token.kind {
            TokenKind::Symbol(value) if value == expected => Ok(()),
            _ => Err(ParseError {
                message: format!("expected '{expected}'"),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn op(&mut self, expected: &str) -> Result<(), ParseError> {
        let token = self.take();
        match token.kind {
            TokenKind::Op(value) if value == expected => Ok(()),
            _ => Err(ParseError {
                message: format!("expected '{expected}'"),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn eof(&mut self) -> Result<(), ParseError> {
        if matches!(self.current().kind, TokenKind::Eof) {
            Ok(())
        } else {
            Err(self.error("expected end of source"))
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }
    fn take(&mut self) -> Token {
        let token = self.tokens[self.index].clone();
        self.index += 1;
        token
    }
    fn peek_word(&self) -> Option<&str> {
        if let TokenKind::Word(value) = &self.current().kind {
            Some(value)
        } else {
            None
        }
    }
    fn is_symbol(&self, value: char) -> bool {
        matches!(self.current().kind, TokenKind::Symbol(found) if found == value)
    }
    fn is_op(&self, value: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Op(found) if found == value)
    }
    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            line: self.current().line,
            column: self.current().column,
        }
    }
    fn error_prev(&self, message: impl Into<String>) -> ParseError {
        let index = self.index.saturating_sub(1);
        ParseError {
            message: message.into(),
            line: self.tokens[index].line,
            column: self.tokens[index].column,
        }
    }
}

fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let mut column = 1;
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            advance(ch, &mut i, &mut line, &mut column);
            continue;
        }
        if ch == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                advance(chars[i], &mut i, &mut line, &mut column);
            }
            continue;
        }
        let start_line = line;
        let start_column = column;
        if "{}()[],".contains(ch) {
            tokens.push(Token {
                kind: TokenKind::Symbol(ch),
                line,
                column,
            });
            advance(ch, &mut i, &mut line, &mut column);
            continue;
        }
        if ch == '"' {
            let start = i;
            advance(ch, &mut i, &mut line, &mut column);
            let mut escaped = false;
            while i < chars.len() {
                let current = chars[i];
                advance(current, &mut i, &mut line, &mut column);
                if !escaped && current == '"' {
                    break;
                }
                escaped = !escaped && current == '\\';
                if current != '\\' {
                    escaped = false;
                }
            }
            if chars.get(i.saturating_sub(1)) != Some(&'"') {
                return Err(ParseError {
                    message: "unterminated string".into(),
                    line: start_line,
                    column: start_column,
                });
            }
            let raw: String = chars[start..i].iter().collect();
            let value: String = serde_json::from_str(&raw).map_err(|_| ParseError {
                message: "invalid string literal".into(),
                line: start_line,
                column: start_column,
            })?;
            tokens.push(Token {
                kind: TokenKind::String(value),
                line: start_line,
                column: start_column,
            });
            continue;
        }
        if ch == '#' {
            advance(ch, &mut i, &mut line, &mut column);
            let start = i;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                advance(chars[i], &mut i, &mut line, &mut column);
            }
            let hex: String = chars[start..i].iter().collect();
            if hex.len() != 6 && hex.len() != 8 {
                return Err(ParseError {
                    message: "expected #RRGGBB or #RRGGBBAA".into(),
                    line: start_line,
                    column: start_column,
                });
            }
            let full = if hex.len() == 6 {
                format!("{hex}ff")
            } else {
                hex
            };
            let mut rgba = [0u8; 4];
            for idx in 0..4 {
                rgba[idx] = u8::from_str_radix(&full[idx * 2..idx * 2 + 2], 16).unwrap();
            }
            tokens.push(Token {
                kind: TokenKind::Color(rgba),
                line: start_line,
                column: start_column,
            });
            continue;
        }
        if ch.is_ascii_digit() || (ch == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_digit() || matches!(chars[i], '.' | 'e' | 'E' | '+' | '-'))
            {
                if i > start
                    && (chars[i] == '+' || chars[i] == '-')
                    && !matches!(chars[i - 1], 'e' | 'E')
                {
                    break;
                }
                advance(chars[i], &mut i, &mut line, &mut column);
            }
            let raw: String = chars[start..i].iter().collect();
            let value = raw.parse::<f64>().map_err(|_| ParseError {
                message: "invalid number".into(),
                line: start_line,
                column: start_column,
            })?;
            if !value.is_finite() {
                return Err(ParseError {
                    message: "number must be finite".into(),
                    line: start_line,
                    column: start_column,
                });
            }
            tokens.push(Token {
                kind: TokenKind::Number(value),
                line: start_line,
                column: start_column,
            });
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                advance(chars[i], &mut i, &mut line, &mut column);
            }
            tokens.push(Token {
                kind: TokenKind::Word(chars[start..i].iter().collect()),
                line: start_line,
                column: start_column,
            });
            continue;
        }
        if "+-*/%<>=!".contains(ch) {
            let mut value = ch.to_string();
            advance(ch, &mut i, &mut line, &mut column);
            if matches!(ch, '<' | '>' | '=' | '!') && chars.get(i) == Some(&'=') {
                value.push('=');
                advance('=', &mut i, &mut line, &mut column);
            }
            tokens.push(Token {
                kind: TokenKind::Op(value),
                line: start_line,
                column: start_column,
            });
            continue;
        }
        return Err(ParseError {
            message: format!("unexpected character {ch:?}"),
            line: start_line,
            column: start_column,
        });
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        line,
        column,
    });
    Ok(tokens)
}

fn advance(ch: char, index: &mut usize, line: &mut usize, column: &mut usize) {
    *index += 1;
    if ch == '\n' {
        *line += 1;
        *column = 1;
    } else {
        *column += 1;
    }
}
