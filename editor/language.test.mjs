import test from 'node:test';
import assert from 'node:assert/strict';
import {
  ensureProjectIds,
  formatExpression,
  formatProject,
  parseExpression,
  parseProjectSource,
  reuseIds,
  upgradeProject,
} from './language.mjs';

const source = `project "demo" {
  stage 480 360 background #f5f7faff
  global score = 0
  list points = [1, 2]
  asset image "cat" = "assets/cat.png"
  asset sound "pop" = "assets/pop.wav"
  sprite "Cat" at 0 0 direction 0 size 32 color #4c97ffff costume "cat" {
    var speed = 4
    list trail = []
    when start {
      repeat 3 {
        move speed * 2 + 1
        change score by 1
        push score to trail
      }
      broadcast "done"
    }
    when key "space" {
      if key("left") and score >= 3 {
        turn -15
      } else {
        call hop(8)
      }
    }
    when message "done" {
      pen down
      while not touching("Cat") {
        move 1
      }
      pen up
      play "pop"
    }
    proc hop(amount) {
      move amount
      wait 0.05
    }
  }
}`;

const stripIds = value => {
  if (Array.isArray(value)) return value.map(stripIds);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value)
      .filter(([key]) => key !== 'id')
      .map(([key, child]) => [key, stripIds(child)]));
  }
  return value;
};

test('parses v2 project, state, events, procedures, and assets', () => {
  const project = parseProjectSource(source);
  assert.equal(project.version, 2);
  assert.equal(project.globals[0].name, 'score');
  assert.equal(project.lists[0].items.length, 2);
  assert.deepEqual(project.assets.map(asset => asset.kind), ['image', 'sound']);
  const sprite = project.sprites[0];
  assert.equal(sprite.costume, 'cat');
  assert.equal(sprite.variables[0].name, 'speed');
  assert.deepEqual(sprite.scripts.map(script => script.event.kind), ['start', 'key', 'message']);
  assert.equal(sprite.procedures[0].params[0], 'amount');
  assert.match(sprite.id, /^n_/);
  assert.match(sprite.scripts[0].id, /^n_/);
  assert.match(sprite.scripts[0].body[0].id, /^n_/);
});

test('canonical formatting round-trips v2 semantics', () => {
  const first = parseProjectSource(source);
  const text = formatProject(first);
  const second = parseProjectSource(text);
  assert.deepEqual(stripIds(second), stripIds(first));
  assert.match(text, /when key "space"/);
  assert.match(text, /if key\("left"\) and score >= 3 \{/);
  assert.match(text, /proc hop\(amount\)/);
});

test('expression parser preserves precedence and formats canonically', () => {
  const expression = parseExpression('speed * 2 + 1 >= 9 and not key("space")');
  assert.equal(expression.kind, 'binary');
  assert.equal(expression.op, 'and');
  assert.equal(expression.left.op, 'ge');
  assert.equal(expression.left.left.op, 'add');
  assert.equal(expression.left.left.left.op, 'mul');
  assert.equal(formatExpression(expression), 'speed * 2 + 1 >= 9 and not key("space")');
});

test('version 1 JSON shape upgrades to v2 start event and expression nodes', () => {
  const old = {
    version: 1,
    name: 'old',
    stage: { width: 480, height: 360, background: [255, 255, 255, 255] },
    sprites: [{
      id: 'sprite-old', name: 'Cat', x: 0, y: 0, direction: 0, size: 20,
      color: [76, 151, 255, 255],
      script: [{ id: 'move-old', op: 'move', steps: 10 }],
    }],
  };
  const project = upgradeProject(old);
  assert.equal(project.version, 2);
  assert.equal(project.sprites[0].scripts[0].event.kind, 'start');
  assert.equal(project.sprites[0].scripts[0].body[0].steps.kind, 'literal');
  assert.equal(project.sprites[0].scripts[0].body[0].steps.value, 10);
});

test('stable ids survive compatible nested code edits', () => {
  const first = parseProjectSource(source);
  const edited = parseProjectSource(source.replace('change score by 1', 'change score by 2'));
  reuseIds(first, edited);
  const oldRepeat = first.sprites[0].scripts[0].body[0];
  const newRepeat = edited.sprites[0].scripts[0].body[0];
  assert.equal(edited.sprites[0].id, first.sprites[0].id);
  assert.equal(edited.sprites[0].scripts[0].id, first.sprites[0].scripts[0].id);
  assert.equal(newRepeat.id, oldRepeat.id);
  assert.equal(newRepeat.body[1].id, oldRepeat.body[1].id);
  assert.equal(newRepeat.body[1].delta.value, 2);
});

test('ensureProjectIds fills ids for events, procedures and nested commands', () => {
  const project = parseProjectSource(source);
  for (const sprite of project.sprites) {
    sprite.id = '';
    for (const script of sprite.scripts) script.id = '';
    for (const procedure of sprite.procedures) procedure.id = '';
  }
  ensureProjectIds(project);
  assert.match(project.sprites[0].id, /^n_/);
  assert.match(project.sprites[0].scripts[0].id, /^n_/);
  assert.match(project.sprites[0].procedures[0].id, /^n_/);
});

test('rejects incomplete source instead of producing a partial project', () => {
  assert.throws(
    () => parseProjectSource('project "x" { stage 480 360 background #ffffffff sprite "S" at 0 0 direction 0 size 20 color #ffffffff { when start { if true { move 1 } }'),
    /line|Expected|expected/,
  );
});
