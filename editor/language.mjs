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
    for (const script of sprite.scripts ?? []) {
      script.id ||= makeId();
      ensureCommandIds(script.body ?? []);
    }
    for (const procedure of sprite.procedures ?? []) {
      procedure.id ||= makeId();
      ensureCommandIds(procedure.body ?? []);
    }
  }
  return project;
}

function ensureCommandIds(commands) {
  for (const command of commands) {
    command.id ||= makeId();
    if (command.op === 'repeat' || command.op === 'while') ensureCommandIds(command.body ?? []);
    if (command.op === 'if') {
      ensureCommandIds(command.then_body ?? []);
      ensureCommandIds(command.else_body ?? []);
    }
  }
}

export function reuseIds(previous, next) {
  const oldSprites = previous?.sprites ?? [];
  for (let i = 0; i < (next.sprites ?? []).length; i += 1) {
    const sprite = next.sprites[i];
    const old = oldSprites[i];
    if (old && old.name === sprite.name) sprite.id = old.id || sprite.id;
    reuseScriptIds(old?.scripts ?? [], sprite.scripts ?? []);
    reuseProcedureIds(old?.procedures ?? [], sprite.procedures ?? []);
  }
  return next;
}

function reuseScriptIds(previous, next) {
  for (let i = 0; i < next.length; i += 1) {
    const script = next[i];
    const old = previous[i];
    if (old && sameEvent(old.event, script.event)) {
      script.id = old.id || script.id;
      reuseCommandIds(old.body ?? [], script.body ?? []);
    }
  }
}

function sameEvent(left, right) {
  if (!left || !right || left.kind !== right.kind) return false;
  if (left.kind === 'key') return left.key === right.key;
  if (left.kind === 'message') return left.message === right.message;
  return true;
}

function reuseProcedureIds(previous, next) {
  for (let i = 0; i < next.length; i += 1) {
    const procedure = next[i];
    const old = previous[i];
    if (old && old.name === procedure.name) {
      procedure.id = old.id || procedure.id;
      reuseCommandIds(old.body ?? [], procedure.body ?? []);
    }
  }
}

function reuseCommandIds(previous, next) {
  for (let i = 0; i < next.length; i += 1) {
    const command = next[i];
    const old = previous[i];
    if (!old || old.op !== command.op) continue;
    command.id = old.id || command.id;
    if (command.op === 'repeat' || command.op === 'while') reuseCommandIds(old.body ?? [], command.body ?? []);
    if (command.op === 'if') {
      reuseCommandIds(old.then_body ?? [], command.then_body ?? []);
      reuseCommandIds(old.else_body ?? [], command.else_body ?? []);
    }
  }
}

export function upgradeProject(input) {
  const project = structuredCloneSafe(input);
  if (Number(project?.version ?? 1) === 2) {
    project.globals ??= [];
    project.lists ??= [];
    project.assets ??= [];
    project.sprites ??= [];
    for (const sprite of project.sprites) {
      sprite.costume ??= null;
      sprite.variables ??= [];
      sprite.lists ??= [];
      sprite.scripts ??= [];
      sprite.procedures ??= [];
    }
    return ensureProjectIds(project);
  }
  if (Number(project?.version ?? 1) !== 1) throw new Error(`Unsupported project version ${project?.version}`);
  const upgraded = {
    version: 2,
    name: String(project.name ?? 'project'),
    stage: project.stage ?? { width: 480, height: 360, background: [255, 255, 255, 255] },
    globals: [],
    lists: [],
    assets: [],
    sprites: (project.sprites ?? []).map(sprite => ({
      id: sprite.id || makeId(),
      name: String(sprite.name ?? 'Sprite'),
      x: Number(sprite.x ?? 0),
      y: Number(sprite.y ?? 0),
      direction: Number(sprite.direction ?? 0),
      size: Number(sprite.size ?? 20),
      color: sprite.color ?? [76, 151, 255, 255],
      costume: null,
      variables: [],
      lists: [],
      scripts: [{ id: makeId(), event: { kind: 'start' }, body: upgradeV1Commands(sprite.script ?? []) }],
      procedures: [],
    })),
  };
  return ensureProjectIds(upgraded);
}

function upgradeV1Commands(commands) {
  return commands.map(command => {
    if (command.op === 'move') return { id: command.id || makeId(), op: 'move', steps: literal(Number(command.steps ?? 0)) };
    if (command.op === 'turn') return { id: command.id || makeId(), op: 'turn', degrees: literal(Number(command.degrees ?? 0)) };
    if (command.op === 'wait') return { id: command.id || makeId(), op: 'wait', seconds: literal(Number(command.seconds ?? 0)) };
    if (command.op === 'repeat') return {
      id: command.id || makeId(), op: 'repeat', times: literal(Number(command.times ?? 0)), body: upgradeV1Commands(command.body ?? []),
    };
    throw new Error(`Cannot upgrade unknown v1 command ${command.op}`);
  });
}

function structuredCloneSafe(value) {
  if (globalThis.structuredClone) return globalThis.structuredClone(value);
  return JSON.parse(JSON.stringify(value));
}

export function parseProjectSource(source) {
  return new Parser(tokenize(source)).parseProject();
}

export function parseExpression(source) {
  const parser = new Parser(tokenize(source));
  const expression = parser.expression(0);
  parser.expectType('eof', 'end of expression');
  return expression;
}

export function formatExpression(expression) {
  return formatExprPrec(expression, 0);
}

export function formatProject(projectInput) {
  const project = upgradeProject(projectInput);
  const lines = [];
  lines.push(`project ${JSON.stringify(project.name)} {`);
  lines.push(`  stage ${formatNumber(project.stage.width)} ${formatNumber(project.stage.height)} background ${rgbaToHex(project.stage.background)}`);
  for (const variable of project.globals ?? []) lines.push(`  global ${variable.name} = ${formatValue(variable.value)}`);
  for (const list of project.lists ?? []) lines.push(`  list ${list.name} = ${formatList(list.items ?? [])}`);
  for (const asset of project.assets ?? []) lines.push(`  asset ${asset.kind} ${JSON.stringify(asset.name)} = ${JSON.stringify(asset.path)}`);
  for (const sprite of project.sprites ?? []) {
    let header = `  sprite ${JSON.stringify(sprite.name)} at ${formatNumber(sprite.x)} ${formatNumber(sprite.y)} direction ${formatNumber(sprite.direction)} size ${formatNumber(sprite.size)} color ${rgbaToHex(sprite.color)}`;
    if (sprite.costume) header += ` costume ${JSON.stringify(sprite.costume)}`;
    lines.push(`${header} {`);
    for (const variable of sprite.variables ?? []) lines.push(`    var ${variable.name} = ${formatValue(variable.value)}`);
    for (const list of sprite.lists ?? []) lines.push(`    list ${list.name} = ${formatList(list.items ?? [])}`);
    for (const script of sprite.scripts ?? []) {
      lines.push(`    ${formatEvent(script.event)} {`);
      formatCommands(script.body ?? [], 3, lines);
      lines.push('    }');
    }
    for (const procedure of sprite.procedures ?? []) {
      lines.push(`    proc ${procedure.name}(${(procedure.params ?? []).join(', ')}) {`);
      formatCommands(procedure.body ?? [], 3, lines);
      lines.push('    }');
    }
    lines.push('  }');
  }
  lines.push('}');
  return lines.join('\n');
}

function formatEvent(event) {
  if (event.kind === 'start') return 'when start';
  if (event.kind === 'key') return `when key ${JSON.stringify(event.key)}`;
  if (event.kind === 'message') return `when message ${JSON.stringify(event.message)}`;
  throw new Error(`Unknown event ${event.kind}`);
}

function formatCommands(commands, depth, lines) {
  const prefix = '  '.repeat(depth);
  for (const command of commands) {
    if (command.op === 'move') lines.push(`${prefix}move ${formatExpression(command.steps)}`);
    else if (command.op === 'turn') lines.push(`${prefix}turn ${formatExpression(command.degrees)}`);
    else if (command.op === 'wait') lines.push(`${prefix}wait ${formatExpression(command.seconds)}`);
    else if (command.op === 'set') lines.push(`${prefix}set ${command.name} = ${formatExpression(command.value)}`);
    else if (command.op === 'change') lines.push(`${prefix}change ${command.name} by ${formatExpression(command.delta)}`);
    else if (command.op === 'push') lines.push(`${prefix}push ${formatExpression(command.value)} to ${command.list}`);
    else if (command.op === 'broadcast') lines.push(`${prefix}broadcast ${JSON.stringify(command.message)}`);
    else if (command.op === 'call') lines.push(`${prefix}call ${command.name}(${(command.args ?? []).map(formatExpression).join(', ')})`);
    else if (command.op === 'pen_down') lines.push(`${prefix}pen down`);
    else if (command.op === 'pen_up') lines.push(`${prefix}pen up`);
    else if (command.op === 'pen_clear') lines.push(`${prefix}pen clear`);
    else if (command.op === 'play') lines.push(`${prefix}play ${JSON.stringify(command.sound)}`);
    else if (command.op === 'repeat' || command.op === 'while') {
      const head = command.op === 'repeat'
        ? `repeat ${formatExpression(command.times)}`
        : `while ${formatExpression(command.condition)}`;
      lines.push(`${prefix}${head} {`);
      formatCommands(command.body ?? [], depth + 1, lines);
      lines.push(`${prefix}}`);
    } else if (command.op === 'if') {
      lines.push(`${prefix}if ${formatExpression(command.condition)} {`);
      formatCommands(command.then_body ?? [], depth + 1, lines);
      if ((command.else_body ?? []).length) {
        lines.push(`${prefix}} else {`);
        formatCommands(command.else_body, depth + 1, lines);
      }
      lines.push(`${prefix}}`);
    } else throw new Error(`Cannot format unknown command ${command.op}`);
  }
}

function formatExprPrec(expr, parentPrecedence) {
  if (!expr) return '0';
  if (expr.kind === 'literal') return formatValue(expr.value);
  if (expr.kind === 'var') return expr.name;
  if (expr.kind === 'key') return `key(${JSON.stringify(expr.key)})`;
  if (expr.kind === 'touching') return `touching(${JSON.stringify(expr.sprite)})`;
  if (expr.kind === 'list_len') return `len(${expr.name})`;
  if (expr.kind === 'unary') {
    const text = expr.op === 'neg' ? `-${formatExprPrec(expr.value, 8)}` : `not ${formatExprPrec(expr.value, 8)}`;
    return 8 < parentPrecedence ? `(${text})` : text;
  }
  if (expr.kind === 'binary') {
    const entry = OP_FORMAT[expr.op];
    if (!entry) throw new Error(`Unknown binary operator ${expr.op}`);
    const [symbol, precedence] = entry;
    const text = `${formatExprPrec(expr.left, precedence)} ${symbol} ${formatExprPrec(expr.right, precedence + 1)}`;
    return precedence < parentPrecedence ? `(${text})` : text;
  }
  throw new Error(`Unknown expression kind ${expr.kind}`);
}

const OP_FORMAT = {
  or: ['or', 1], and: ['and', 2], eq: ['==', 3], ne: ['!=', 3],
  lt: ['<', 4], le: ['<=', 4], gt: ['>', 4], ge: ['>=', 4],
  add: ['+', 5], sub: ['-', 5], mul: ['*', 6], div: ['/', 6], mod: ['%', 6],
};

function formatNumber(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) throw new Error(`Non-finite number: ${value}`);
  return String(number);
}

function formatValue(value) {
  if (typeof value === 'number') return formatNumber(value);
  if (typeof value === 'boolean') return String(value);
  if (typeof value === 'string') return JSON.stringify(value);
  throw new Error(`Unsupported literal value ${value}`);
}

function formatList(items) {
  return `[${items.map(formatValue).join(', ')}]`;
}

function rgbaToHex(rgba) {
  return `#${rgba.map(value => Math.max(0, Math.min(255, Number(value) | 0)).toString(16).padStart(2, '0')).join('')}`;
}

function literal(value) { return { kind: 'literal', value }; }

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
    if ('{}()[],'.includes(char)) {
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
    const numberMatch = source.slice(index).match(/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?/);
    if (numberMatch) {
      for (let i = 0; i < numberMatch[0].length; i += 1) advance();
      const value = Number(numberMatch[0]);
      if (!Number.isFinite(value)) throw syntaxError('Expected finite number', startLine, startColumn);
      tokens.push({ type: 'number', value, line: startLine, column: startColumn });
      continue;
    }
    if (/[A-Za-z_]/.test(char)) {
      let value = '';
      while (index < source.length && /[A-Za-z0-9_]/.test(source[index])) value += advance();
      tokens.push({ type: 'word', value, line: startLine, column: startColumn });
      continue;
    }
    if ('+-*/%<>=!'.includes(char)) {
      let value = advance();
      if ('<>=!'.includes(char) && source[index] === '=') value += advance();
      tokens.push({ type: 'op', value, line: startLine, column: startColumn });
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
  is(type, value = undefined) { return this.current().type === type && (value === undefined || this.current().value === value); }
  wordIs(value) { return this.is('word', value); }
  expectType(type, description = type) {
    const token = this.current();
    if (token.type !== type) throw this.error(`Expected ${description}, found ${displayToken(token)}`);
    return this.take().value;
  }
  expectWord(word) {
    if (!this.wordIs(word)) throw this.error(`Expected '${word}', found ${displayToken(this.current())}`);
    this.take();
  }
  expectOp(op) {
    if (!this.is('op', op)) throw this.error(`Expected '${op}', found ${displayToken(this.current())}`);
    this.take();
  }
  number() { return this.expectType('number', 'number'); }
  string() { return this.expectType('string', 'string'); }
  identifier() { return this.expectType('word', 'identifier'); }

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
    const globals = [];
    const lists = [];
    const assets = [];
    const sprites = [];
    while (!this.is('}')) {
      if (this.wordIs('global')) { this.take(); globals.push(this.parseVariable()); }
      else if (this.wordIs('list')) { this.take(); lists.push(this.parseList()); }
      else if (this.wordIs('asset')) { this.take(); assets.push(this.parseAsset()); }
      else if (this.wordIs('sprite')) sprites.push(this.parseSprite());
      else throw this.error("Expected global, list, asset, sprite, or '}'");
    }
    this.expectType('}');
    this.expectType('eof', 'end of source');
    if (!name.trim()) throw this.error('Expected non-empty project name');
    return ensureProjectIds({ version: 2, name, stage: { width, height, background }, globals, lists, assets, sprites });
  }

  parseVariable() {
    const name = this.identifier();
    this.expectOp('=');
    return { name, value: this.literalValue() };
  }

  parseList() {
    const name = this.identifier();
    this.expectOp('=');
    this.expectType('[');
    const items = [];
    if (!this.is(']')) {
      while (true) {
        items.push(this.literalValue());
        if (this.is(']')) break;
        this.expectType(',');
      }
    }
    this.expectType(']');
    return { name, items };
  }

  parseAsset() {
    const kind = this.identifier();
    if (kind !== 'image' && kind !== 'sound') throw this.error('Asset kind must be image or sound');
    const name = this.string();
    this.expectOp('=');
    return { kind, name, path: this.string() };
  }

  parseSprite() {
    this.expectWord('sprite');
    const name = this.string();
    this.expectWord('at');
    const x = this.signedNumber();
    const y = this.signedNumber();
    this.expectWord('direction');
    const direction = this.signedNumber();
    this.expectWord('size');
    const size = this.signedNumber();
    this.expectWord('color');
    const color = parseColor(this.expectType('color', 'color'));
    let costume = null;
    if (this.wordIs('costume')) { this.take(); costume = this.string(); }
    this.expectType('{');
    const variables = [];
    const lists = [];
    const scripts = [];
    const procedures = [];
    while (!this.is('}')) {
      if (this.wordIs('var')) { this.take(); variables.push(this.parseVariable()); }
      else if (this.wordIs('list')) { this.take(); lists.push(this.parseList()); }
      else if (this.wordIs('when')) scripts.push(this.parseScript());
      else if (this.wordIs('proc')) procedures.push(this.parseProcedure());
      else throw this.error("Expected var, list, when, proc, or '}'");
    }
    this.expectType('}');
    if (!name.trim()) throw this.error('Expected non-empty sprite name');
    if (!(size > 0)) throw this.error('Expected sprite size > 0');
    return { id: makeId(), name, x, y, direction, size, color, costume, variables, lists, scripts, procedures };
  }

  parseScript() {
    this.expectWord('when');
    const kind = this.identifier();
    let event;
    if (kind === 'start') event = { kind: 'start' };
    else if (kind === 'key') event = { kind: 'key', key: this.string() };
    else if (kind === 'message') event = { kind: 'message', message: this.string() };
    else throw this.error('Event must be start, key, or message');
    this.expectType('{');
    const body = this.parseCommands();
    this.expectType('}');
    return { id: makeId(), event, body };
  }

  parseProcedure() {
    this.expectWord('proc');
    const name = this.identifier();
    this.expectType('(');
    const params = [];
    if (!this.is(')')) {
      while (true) {
        params.push(this.identifier());
        if (this.is(')')) break;
        this.expectType(',');
      }
    }
    this.expectType(')');
    this.expectType('{');
    const body = this.parseCommands();
    this.expectType('}');
    return { id: makeId(), name, params, body };
  }

  parseCommands() {
    const commands = [];
    while (!this.is('}')) {
      if (this.is('eof')) throw this.error("Expected '}' before end of source");
      commands.push(this.parseCommand());
    }
    return commands;
  }

  parseCommand() {
    const op = this.identifier();
    const id = makeId();
    if (op === 'move') return { id, op, steps: this.expression(0) };
    if (op === 'turn') return { id, op, degrees: this.expression(0) };
    if (op === 'wait') return { id, op, seconds: this.expression(0) };
    if (op === 'set') { const name = this.identifier(); this.expectOp('='); return { id, op, name, value: this.expression(0) }; }
    if (op === 'change') { const name = this.identifier(); this.expectWord('by'); return { id, op, name, delta: this.expression(0) }; }
    if (op === 'push') { const value = this.expression(0); this.expectWord('to'); return { id, op, list: this.identifier(), value }; }
    if (op === 'broadcast') return { id, op, message: this.string() };
    if (op === 'call') {
      const name = this.identifier();
      this.expectType('(');
      const args = [];
      if (!this.is(')')) {
        while (true) {
          args.push(this.expression(0));
          if (this.is(')')) break;
          this.expectType(',');
        }
      }
      this.expectType(')');
      return { id, op, name, args };
    }
    if (op === 'pen') {
      const mode = this.identifier();
      if (mode === 'down') return { id, op: 'pen_down' };
      if (mode === 'up') return { id, op: 'pen_up' };
      if (mode === 'clear') return { id, op: 'pen_clear' };
      throw this.error('Pen command must be down, up, or clear');
    }
    if (op === 'play') return { id, op, sound: this.string() };
    if (op === 'repeat') {
      const times = this.expression(0); this.expectType('{'); const body = this.parseCommands(); this.expectType('}');
      return { id, op, times, body };
    }
    if (op === 'while') {
      const condition = this.expression(0); this.expectType('{'); const body = this.parseCommands(); this.expectType('}');
      return { id, op, condition, body };
    }
    if (op === 'if') {
      const condition = this.expression(0); this.expectType('{'); const then_body = this.parseCommands(); this.expectType('}');
      let else_body = [];
      if (this.wordIs('else')) { this.take(); this.expectType('{'); else_body = this.parseCommands(); this.expectType('}'); }
      return { id, op, condition, then_body, else_body };
    }
    throw this.error(`Unknown command '${op}'`);
  }

  expression(minPrecedence) {
    let left = this.prefix();
    while (true) {
      const entry = this.binaryOperator();
      if (!entry || entry[1] < minPrecedence) break;
      const [op, precedence] = entry;
      this.take();
      const right = this.expression(precedence + 1);
      left = { kind: 'binary', op, left, right };
    }
    return left;
  }

  prefix() {
    if (this.wordIs('not')) { this.take(); return { kind: 'unary', op: 'not', value: this.expression(8) }; }
    if (this.is('op', '-')) { this.take(); return { kind: 'unary', op: 'neg', value: this.expression(8) }; }
    if (this.is('number')) return literal(this.take().value);
    if (this.is('string')) return literal(this.take().value);
    if (this.wordIs('true') || this.wordIs('false')) return literal(this.take().value === 'true');
    if (this.is('word')) {
      const name = this.take().value;
      if (!this.is('(')) return { kind: 'var', name };
      this.take();
      let result;
      if (name === 'key') result = { kind: 'key', key: this.string() };
      else if (name === 'touching') result = { kind: 'touching', sprite: this.string() };
      else if (name === 'len') result = { kind: 'list_len', name: this.identifier() };
      else throw this.error(`Unknown expression function '${name}'`);
      this.expectType(')');
      return result;
    }
    if (this.is('(')) { this.take(); const result = this.expression(0); this.expectType(')'); return result; }
    throw this.error(`Expected expression, found ${displayToken(this.current())}`);
  }

  binaryOperator() {
    if (this.wordIs('or')) return ['or', 1];
    if (this.wordIs('and')) return ['and', 2];
    const map = { '==': ['eq', 3], '!=': ['ne', 3], '<': ['lt', 4], '<=': ['le', 4], '>': ['gt', 4], '>=': ['ge', 4], '+': ['add', 5], '-': ['sub', 5], '*': ['mul', 6], '/': ['div', 6], '%': ['mod', 6] };
    return this.current().type === 'op' ? map[this.current().value] : undefined;
  }

  literalValue() {
    if (this.is('op', '-')) {
      this.take();
      const number = this.number();
      return -number;
    }
    if (this.is('number') || this.is('string')) return this.take().value;
    if (this.wordIs('true') || this.wordIs('false')) return this.take().value === 'true';
    throw this.error('Expected number, bool, or string literal');
  }

  signedNumber() {
    if (this.is('op', '-')) { this.take(); return -this.number(); }
    if (this.is('op', '+')) this.take();
    return this.number();
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
  return new SyntaxError(`${message} at line ${line}, column ${column}`);
}
