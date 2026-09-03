import test from 'node:test';
import assert from 'node:assert/strict';
import { formatProject, parseProjectSource, reuseIds } from './language.mjs';

const source = `project "triangle-demo" {
  stage 480 360 background #f5f7faff
  sprite "Sprite 1" at -80 0 direction 0 size 26 color #4c97ffff {
    repeat 2 {
      move 80
      turn 90
      wait 0.15
    }
  }
}`;

test('parses the full project language into the shared AST shape', () => {
  const project = parseProjectSource(source);
  assert.equal(project.version, 1);
  assert.equal(project.name, 'triangle-demo');
  assert.deepEqual(project.stage, { width: 480, height: 360, background: [245, 247, 250, 255] });
  assert.equal(project.sprites.length, 1);
  assert.match(project.sprites[0].id, /^n_/);
  assert.equal(project.sprites[0].script[0].op, 'repeat');
  assert.match(project.sprites[0].script[0].id, /^n_/);
  assert.equal(project.sprites[0].script[0].body[2].seconds, 0.15);
});

test('canonical formatting round-trips without changing semantics', () => {
  const first = parseProjectSource(source);
  const text = formatProject(first);
  const second = parseProjectSource(text);
  const stripIds = value => {
    if (Array.isArray(value)) return value.map(stripIds);
    if (value && typeof value === 'object') {
      return Object.fromEntries(Object.entries(value)
        .filter(([key]) => key !== 'id')
        .map(([key, child]) => [key, stripIds(child)]));
    }
    return value;
  };
  assert.deepEqual(stripIds(second), stripIds(first));
  assert.match(text, /repeat 2 \{/);
  assert.match(text, /color #4c97ffff/);
});

test('rejects incomplete source instead of producing a partial project', () => {
  assert.throws(
    () => parseProjectSource('project "x" { sprite "S" at 0 0 direction 0 size 20 color #ffffffff { repeat 2 { move 10 }'),
    /Expected/,
  );
});

test('reuses stable node ids across compatible code edits', () => {
  const first = parseProjectSource(source);
  const edited = parseProjectSource(source.replace('move 80', 'move 81'));
  reuseIds(first, edited);
  assert.equal(edited.sprites[0].id, first.sprites[0].id);
  assert.equal(edited.sprites[0].script[0].id, first.sprites[0].script[0].id);
  assert.equal(edited.sprites[0].script[0].body[0].id, first.sprites[0].script[0].body[0].id);
  assert.equal(edited.sprites[0].script[0].body[0].steps, 81);
});
