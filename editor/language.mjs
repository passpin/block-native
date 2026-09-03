let idCounter = 0;

export function makeId() {
  idCounter += 1;
  const random = globalThis.crypto?.randomUUID?.().replaceAll('-', '').slice(0, 8)
    ?? Math.random().toString(36).slice(2, 10);
  return `n_${random}_${idCounter.toString(36)}`;
}

export function ensureProjectIds(project) {
  for (const sprite of project.sprites ?? []) {
    sprite.id ||= makeId();
    ensureCommandIds(sprite.script ?? []);
  }
  return project;
}

function ensureCommandIds(commands) {
  for (const command of commands) {
    command.id ||= makeId();
    if (command.op === 'repeat') ensureCommandIds(command.body ?? []);
  }
}

export function reuseIds(previous, next) {
  const oldSprites = previous?.sprites ?? [];
  for (let i = 0; i < (next.sprites ?? []).length; i += 1) {
    const sprite = next.sprites[i];
    const old = oldSprites[i];
    if (old && old.name === sprite.name) sprite.id = old.id || sprite.id;
    reuseCommandIds(old?.script ?? [], sprite.script ?? []);
  }
  return next;
}

function reuseCommandIds(previous, next) {
  for (let i = 0; i < next.length; i += 1) {
    const command = next[i];
    const old = previous[i];
    if (old && old.op === command.op) {
      command.id = old.id || command.id;
      if (command.op === 'repeat') reuseCommandIds(old.body ?? [], command.body ?? []);
    }
  }
}

export function parseProjectSource(source) {
  const parser = new Parser(tokenize(source));
  return parser.parseProject();
}

export function formatProject(project) {
  const lines = [];
  lines.push(`project ${JSON.stringify(project.name)} {`);
  lines.push(`  stage ${formatNumber(project.stage.width)} ${formatNumber(project.stage.height)} background ${rgbaToHex(project.stage.background)}`);
  for (const sprite of project.sprites) {
    lines.push(`  sprite ${JSON.stringify(sprite.name)} at ${formatNumber(sprite.x)} ${formatNumber(sprite.y)} direction ${formatNumber(sprite.direction)} size ${formatNumber(sprite.size)} color ${rgbaToHex(sprite.color)} {`);
    formatCommands(sprite.script, 2, lines);
    lines.push('  }');
  }
  lines.push('}');
  return lines.join('\n');
}

function formatCommands(commands, depth, lines) {
  const prefix = '  '.repeat(depth);
  for (const command of commands) {
    if (command.op === 'move') lines.push(`${prefix}move ${formatNumber(command.steps)}`);
    else if (command.op === 'turn') lines.push(`${prefix}turn ${formatNumber(command.degrees)}`);
    else if (command.op === 'wait') lines.push(`${prefix}wait ${formatNumber(command.seconds)}`);
    else if (command.op === 'repeat') {
      lines.push(`${prefix}repeat ${formatNumber(command.times)} {`);
      formatCommands(command.body, depth + 1, lines);
      lines.push(`${prefix}}`);
    } else throw new Error(`Cannot format unknown command ${command.op}`);
  }
}

function formatNumber(value) {
  if (!Number.isFinite(Number(value))) throw new Error(`Non-finite number: ${value}`);
  return String(Number(value));
}

function rgbaToHex(rgba) {
  return `#${rgba.map(value => Math.max(0, Math.min(255, Number(value) | 0)).toString(16).padStart(2, '0')).join('')}`;
}

function tokenize(source) {
  const tokens = [];
  let index = 0;
  let line = 1;
  let column = 1;
  const advance = () => {
    const char = source[index++];
    if (char === '\n') { line += 1; column = 1; } else column += 1;
    return char;
  };

  while (index < source.length) {
    const char = source[index];
    if (/\s/.test(char)) { advance(); continue; }
    if (char === '/' && source[index + 1] === '/') {
      while (index < source.length && source[index] !== '\n') advance();
      continue;
    }
    const startLine = line;
    const startColumn = column;
    if (char === '{' || char === '}') {
      tokens.push({ type: char, value: char, line: startLine, column: startColumn });
      advance();
      continue;
    }
    if (char === '"') {
      const start = index;
      advance();
      let escaped = false;
      while (index < source.length) {
        const current = advance();
        if (!escaped && current === '"') break;
        if (!escaped && current === '\\') escaped = true;
        else escaped = false;
      }
      if (source[index - 1] !== '"') throw syntaxError('Unterminated string', startLine, startColumn);
      const raw = source.slice(start, index);
      let value;
      try { value = JSON.parse(raw); } catch { throw syntaxError('Invalid string literal', startLine, startColumn); }
      tokens.push({ type: 'string', value, line: startLine, column: startColumn });
      continue;
    }
    if (char === '#') {
      let raw = advance();
      while (index < source.length && /[0-9a-fA-F]/.test(source[index])) raw += advance();
      if (!/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(raw)) throw syntaxError('Expected #RRGGBB or #RRGGBBAA color', startLine, startColumn);
      tokens.push({ type: 'color', value: raw, line: startLine, column: startColumn });
      continue;
    }
    if (/[+\-.0-9]/.test(char)) {
      const match = source.slice(index).match(/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?/);
      if (match) {
        for (let i = 0; i < match[0].length; i += 1) advance();
        const value = Number(match[0]);
        if (!Number.isFinite(value)) throw syntaxError('Expected finite number', startLine, startColumn);
        tokens.push({ type: 'number', value, line: startLine, column: startColumn });
        continue;
      }
    }
    if (/[A-Za-z_]/.test(char)) {
      let value = '';
      while (index < source.length && /[A-Za-z0-9_]/.test(source[index])) value += advance();
      tokens.push({ type: 'word', value, line: startLine, column: startColumn });
      continue;
    }
    throw syntaxError(`Unexpected character ${JSON.stringify(char)}`, startLine, startColumn);
  }
  tokens.push({ type: 'eof', value: '', line, column });
  return tokens;
}

class Parser {
  constructor(tokens) { this.tokens = tokens; this.index = 0; }
  current() { return this.tokens[this.index]; }
  take() { return this.tokens[this.index++]; }
  expectType(type, description = type) {
    const token = this.current();
    if (token.type !== type) throw this.error(`Expected ${description}, found ${displayToken(token)}`);
    return this.take().value;
  }
  expectWord(word) {
    const token = this.current();
    if (token.type !== 'word' || token.value !== word) throw this.error(`Expected '${word}', found ${displayToken(token)}`);
    this.take();
  }
  number() { return this.expectType('number', 'number'); }
  string() { return this.expectType('string', 'string'); }

  parseProject() {
    this.expectWord('project');
    const name = this.string();
    this.expectType('{');
    this.expectWord('stage');
    const width = this.number();
    const height = this.number();
    this.expectWord('background');
    const background = parseColor(this.expectType('color', 'color'));
    if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0 || width > 65535 || height > 65535) throw this.error('Expected stage width and height to be positive integers <= 65535');
    const sprites = [];
    while (this.current().type !== '}') sprites.push(this.parseSprite());
    this.expectType('}');
    this.expectType('eof', 'end of source');
    if (!name.trim()) throw this.error('Expected non-empty project name');
    return { version: 1, name, stage: { width, height, background }, sprites };
  }

  parseSprite() {
    this.expectWord('sprite');
    const name = this.string();
    this.expectWord('at');
    const x = this.number();
    const y = this.number();
    this.expectWord('direction');
    const direction = this.number();
    this.expectWord('size');
    const size = this.number();
    this.expectWord('color');
    const color = parseColor(this.expectType('color', 'color'));
    this.expectType('{');
    const script = this.parseCommands();
    this.expectType('}');
    if (!name.trim()) throw this.error('Expected non-empty sprite name');
    if (size <= 0) throw this.error('Expected sprite size > 0');
    return { id: makeId(), name, x, y, direction, size, color, script };
  }

  parseCommands() {
    const commands = [];
    while (this.current().type !== '}') {
      const token = this.current();
      if (token.type === 'eof') throw this.error("Expected '}' before end of source");
      if (token.type !== 'word') throw this.error(`Expected command, found ${displayToken(token)}`);
      const op = token.value;
      this.take();
      if (op === 'move') commands.push({ id: makeId(), op, steps: this.number() });
      else if (op === 'turn') commands.push({ id: makeId(), op, degrees: this.number() });
      else if (op === 'wait') {
        const seconds = this.number();
        if (seconds < 0) throw this.error('Expected wait duration >= 0');
        commands.push({ id: makeId(), op, seconds });
      } else if (op === 'repeat') {
        const times = this.number();
        if (!Number.isInteger(times) || times < 0) throw this.error('Expected repeat count to be a non-negative integer');
        this.expectType('{');
        const body = this.parseCommands();
        this.expectType('}');
        commands.push({ id: makeId(), op, times, body });
      } else throw this.error(`Expected command move, turn, wait, or repeat; found '${op}'`);
    }
    return commands;
  }
  error(message) {
    const token = this.current();
    return syntaxError(message, token.line, token.column);
  }
}

function parseColor(value) {
  const hex = value.slice(1);
  const full = hex.length === 6 ? `${hex}ff` : hex;
  return [0, 2, 4, 6].map(offset => Number.parseInt(full.slice(offset, offset + 2), 16));
}

function displayToken(token) {
  if (token.type === 'eof') return 'end of source';
  if (token.type === 'string') return JSON.stringify(token.value);
  return `'${token.value}'`;
}

function syntaxError(message, line, column) {
  return new SyntaxError(`${message} at ${line}:${column}`);
}
