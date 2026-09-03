import { ensureProjectIds, formatProject, makeId, parseProjectSource, reuseIds } from './language.mjs';

const clone = value => JSON.parse(JSON.stringify(value));

const demoProject = ensureProjectIds({
  version: 1,
  name: 'triangle-demo',
  stage: { width: 480, height: 360, background: [245, 247, 250, 255] },
  sprites: [{
    id: 'sprite_demo',
    name: 'Sprite 1', x: -80, y: 0, direction: 0, size: 26,
    color: [76, 151, 255, 255],
    script: [{ id: 'repeat_demo', op: 'repeat', times: 4, body: [
      { id: 'move_demo', op: 'move', steps: 80 },
      { id: 'turn_demo', op: 'turn', degrees: 90 },
      { id: 'wait_demo', op: 'wait', seconds: 0.15 },
    ] }],
  }],
});

let project = clone(demoProject);
let selectedSpriteId = project.sprites[0].id;
let selectedCommandId = project.sprites[0].script[0].id;
let mode = 'blocks';
let codeFocused = false;
let codeTimer = null;
let paletteContext = null;
let paletteIndex = 0;

const elements = {
  script: document.querySelector('#script'), blockPane: document.querySelector('#blockPane'), codePane: document.querySelector('#codePane'),
  editorArea: document.querySelector('.editor-area'), codeEditor: document.querySelector('#codeEditor'), codeStatus: document.querySelector('#codeStatus'),
  codeError: document.querySelector('#codeError'), spriteList: document.querySelector('#spriteList'), spriteEditor: document.querySelector('#spriteEditor'),
  selectedSpriteLabel: document.querySelector('#selectedSpriteLabel'), jsonPreview: document.querySelector('#jsonPreview'), stage: document.querySelector('#stage'),
  projectName: document.querySelector('#projectName'), stageWidth: document.querySelector('#stageWidth'), stageHeight: document.querySelector('#stageHeight'),
  stageBackground: document.querySelector('#stageBackground'), blockTemplate: document.querySelector('#blockTemplate'),
  commandPalette: document.querySelector('#commandPalette'), commandSearch: document.querySelector('#commandSearch'), commandResults: document.querySelector('#commandResults'),
};

function selectedSprite() { return project.sprites.find(sprite => sprite.id === selectedSpriteId) ?? project.sprites[0]; }
function commandFor(op) {
  if (op === 'move') return { id: makeId(), op, steps: 10 };
  if (op === 'turn') return { id: makeId(), op, degrees: 15 };
  if (op === 'wait') return { id: makeId(), op, seconds: 0.2 };
  if (op === 'repeat') return { id: makeId(), op, times: 10, body: [] };
  throw new Error(`Unknown block ${op}`);
}
function makeSprite(name) { return { id: makeId(), name, x: 0, y: 0, direction: 0, size: 26, color: [76, 151, 255, 255], script: [] }; }

function findCommand(id, commands = selectedSprite()?.script ?? [], parent = null) {
  for (let index = 0; index < commands.length; index += 1) {
    const command = commands[index];
    if (command.id === id) return { command, list: commands, index, parent };
    if (command.op === 'repeat') {
      const found = findCommand(id, command.body, command);
      if (found) return found;
    }
  }
  return null;
}

function syncSelection() {
  if (!project.sprites.length) project.sprites.push(makeSprite('Sprite 1'));
  if (!project.sprites.some(sprite => sprite.id === selectedSpriteId)) selectedSpriteId = project.sprites[0].id;
  if (selectedCommandId && !findCommand(selectedCommandId)) selectedCommandId = selectedSprite().script[0]?.id ?? null;
}

function render({ syncCode = true } = {}) {
  syncSelection(); renderMode();
  elements.projectName.value = project.name; elements.stageWidth.value = project.stage.width; elements.stageHeight.value = project.stage.height;
  elements.stageBackground.value = rgbaToHex(project.stage.background); elements.selectedSpriteLabel.textContent = selectedSprite().name;
  renderScript(); renderSprites(); renderStage(); elements.jsonPreview.textContent = JSON.stringify(project, null, 2);
  if (syncCode && !codeFocused) setCanonicalCode();
}

function renderMode() {
  document.querySelectorAll('[data-mode]').forEach(button => button.classList.toggle('selected', button.dataset.mode === mode));
  elements.blockPane.hidden = mode === 'code'; elements.codePane.hidden = mode === 'blocks'; elements.editorArea.classList.toggle('split', mode === 'split');
}

function renderScript() {
  elements.script.replaceChildren();
  const script = selectedSprite().script;
  if (!script.length) {
    const empty = document.createElement('div'); empty.className = 'empty'; empty.tabIndex = 0;
    empty.textContent = 'No blocks. Press Enter or Ctrl+Space to insert one.';
    empty.addEventListener('keydown', event => {
      if (event.key === 'Enter' || (event.ctrlKey && event.code === 'Space')) { event.preventDefault(); openPalette({ list: script, index: -1 }); }
    });
    elements.script.append(empty); return;
  }
  renderList(script, elements.script);
}
function renderList(list, container) { list.forEach(command => container.append(renderBlock(command))); }

function renderBlock(command) {
  const node = elements.blockTemplate.content.firstElementChild.cloneNode(true);
  node.dataset.op = command.op; node.dataset.id = command.id; node.classList.toggle('selected', command.id === selectedCommandId);
  node.querySelector('.block-title').textContent = command.op;
  node.addEventListener('focus', () => {
    selectedCommandId = command.id; elements.script.querySelectorAll('.block.selected').forEach(item => item.classList.remove('selected')); node.classList.add('selected');
  });
  node.addEventListener('click', event => { if (!event.target.closest('button,input')) { selectedCommandId = command.id; node.focus(); } });
  node.addEventListener('keydown', event => handleBlockKey(event, command.id));
  const fields = node.querySelector('.block-fields');
  if (command.op === 'move') addNumberField(fields, 'steps', command.steps, value => { command.steps = value; changed(); });
  if (command.op === 'turn') addNumberField(fields, 'degrees', command.degrees, value => { command.degrees = value; changed(); });
  if (command.op === 'wait') addNumberField(fields, 'seconds', command.seconds, value => { command.seconds = Math.max(0, value); changed(); }, 0.01);
  if (command.op === 'repeat') addNumberField(fields, 'times', command.times, value => { command.times = Math.max(0, Math.floor(value)); changed(); }, 1);
  node.querySelectorAll('[data-action]').forEach(button => button.addEventListener('click', () => mutateSelected(command.id, button.dataset.action)));
  if (command.op === 'repeat') {
    const body = node.querySelector('.repeat-body'); body.hidden = false; const container = body.querySelector('.repeat-commands');
    if (!command.body.length) { const empty = document.createElement('div'); empty.className = 'repeat-label'; empty.textContent = 'empty — Shift+Enter inserts here'; container.append(empty); }
    else renderList(command.body, container);
  }
  return node;
}

function addNumberField(parent, name, value, onChange, step = 1) {
  const label = document.createElement('label'); label.textContent = name; const input = document.createElement('input');
  input.type = 'number'; input.step = step; input.value = value; input.addEventListener('keydown', event => event.stopPropagation());
  input.addEventListener('change', () => { const parsed = Number(input.value); if (Number.isFinite(parsed)) onChange(parsed); });
  label.append(input); parent.append(label);
}

function handleBlockKey(event, id) {
  const found = findCommand(id); if (!found) return; const { command, list, index, parent } = found;
  if (event.altKey && event.key === 'ArrowUp') { event.preventDefault(); moveCommand(list, index, -1, id); return; }
  if (event.altKey && event.key === 'ArrowDown') { event.preventDefault(); moveCommand(list, index, 1, id); return; }
  if (event.key === 'Delete' || event.key === 'Backspace') { event.preventDefault(); deleteCommand(list, index); return; }
  if (event.key === 'ArrowUp') { event.preventDefault(); focusBlock(list[index - 1]?.id ?? id); return; }
  if (event.key === 'ArrowDown') { event.preventDefault(); focusBlock(list[index + 1]?.id ?? id); return; }
  if (event.key === 'ArrowLeft' && parent) { event.preventDefault(); focusBlock(parent.id); return; }
  if (event.key === 'ArrowRight' && command.op === 'repeat' && command.body.length) { event.preventDefault(); focusBlock(command.body[0].id); return; }
  if (event.key === 'Enter' && event.shiftKey && command.op === 'repeat') { event.preventDefault(); openPalette({ list: command.body, index: -1 }); return; }
  if (event.key === 'Enter') { event.preventDefault(); openPalette({ list, index }); return; }
  if (event.ctrlKey && event.code === 'Space') { event.preventDefault(); openPalette(event.shiftKey && command.op === 'repeat' ? { list: command.body, index: -1 } : { list, index }); }
}

function mutateSelected(id, action) {
  const found = findCommand(id); if (!found) return;
  if (action === 'delete') deleteCommand(found.list, found.index);
  if (action === 'up') moveCommand(found.list, found.index, -1, id);
  if (action === 'down') moveCommand(found.list, found.index, 1, id);
}
function moveCommand(list, index, delta, id) {
  const next = index + delta; if (next < 0 || next >= list.length) return;
  [list[index], list[next]] = [list[next], list[index]]; changed(); requestAnimationFrame(() => focusBlock(id));
}
function deleteCommand(list, index) {
  const fallback = list[index + 1]?.id ?? list[index - 1]?.id ?? null; list.splice(index, 1); selectedCommandId = fallback; changed();
  if (fallback) requestAnimationFrame(() => focusBlock(fallback));
}
function focusBlock(id) { selectedCommandId = id; elements.script.querySelector(`[data-id="${CSS.escape(id)}"]`)?.focus({ preventScroll: false }); }

function openPalette(context) { paletteContext = context; paletteIndex = 0; elements.commandSearch.value = ''; renderPalette(); elements.commandPalette.showModal(); requestAnimationFrame(() => elements.commandSearch.focus()); }
function paletteOptions() { const query = elements.commandSearch.value.trim().toLowerCase(); return ['move', 'turn', 'wait', 'repeat'].filter(op => op.includes(query)); }
function renderPalette() {
  const options = paletteOptions(); if (paletteIndex >= options.length) paletteIndex = Math.max(0, options.length - 1); elements.commandResults.replaceChildren();
  options.forEach((op, index) => { const button = document.createElement('button'); button.type = 'button'; button.textContent = op; button.classList.toggle('active', index === paletteIndex); button.addEventListener('click', () => insertFromPalette(op)); elements.commandResults.append(button); });
}
function insertFromPalette(op = paletteOptions()[paletteIndex]) {
  if (!op || !paletteContext) return; const command = commandFor(op); const at = paletteContext.index < 0 ? paletteContext.list.length : paletteContext.index + 1;
  paletteContext.list.splice(at, 0, command); selectedCommandId = command.id; elements.commandPalette.close(); changed(); requestAnimationFrame(() => focusBlock(command.id));
}

function renderSprites() {
  elements.spriteList.replaceChildren();
  for (const sprite of project.sprites) {
    const button = document.createElement('button'); button.className = `sprite-chip${sprite.id === selectedSpriteId ? ' selected' : ''}`; button.textContent = sprite.name;
    button.addEventListener('click', () => { selectedSpriteId = sprite.id; selectedCommandId = sprite.script[0]?.id ?? null; render(); }); elements.spriteList.append(button);
  }
  const sprite = selectedSprite();
  elements.spriteEditor.innerHTML = `<div class="sprite-grid"><label class="wide">Name<input data-sprite="name" type="text"></label><label>X<input data-sprite="x" type="number"></label><label>Y<input data-sprite="y" type="number"></label><label>Direction<input data-sprite="direction" type="number"></label><label>Size<input data-sprite="size" type="number" min="1"></label><label class="wide">Color<input data-sprite="color" type="color"></label></div><button id="deleteSprite" class="danger ghost">Delete sprite</button>`;
  const setValue = (key, value) => { elements.spriteEditor.querySelector(`[data-sprite="${key}"]`).value = value; };
  setValue('name', sprite.name); setValue('x', sprite.x); setValue('y', sprite.y); setValue('direction', sprite.direction); setValue('size', sprite.size); setValue('color', rgbaToHex(sprite.color));
  elements.spriteEditor.querySelectorAll('[data-sprite]').forEach(input => input.addEventListener('change', () => {
    const key = input.dataset.sprite;
    if (key === 'name') sprite.name = input.value.trim() || 'Sprite'; else if (key === 'color') sprite.color = hexToRgba(input.value);
    else { const value = Number(input.value); if (Number.isFinite(value)) sprite[key] = key === 'size' ? Math.max(1, value) : value; }
    changed();
  }));
  elements.spriteEditor.querySelector('#deleteSprite').addEventListener('click', () => {
    if (project.sprites.length === 1) return; const index = project.sprites.findIndex(item => item.id === sprite.id); project.sprites.splice(index, 1);
    selectedSpriteId = project.sprites[Math.max(0, index - 1)].id; selectedCommandId = selectedSprite().script[0]?.id ?? null; changed();
  });
}

function renderStage() {
  const canvas = elements.stage; canvas.width = Math.min(project.stage.width, 960); canvas.height = Math.min(project.stage.height, 720); const ctx = canvas.getContext('2d');
  ctx.fillStyle = rgbaCss(project.stage.background); ctx.fillRect(0, 0, canvas.width, canvas.height); const sx = canvas.width / project.stage.width, sy = canvas.height / project.stage.height;
  for (const sprite of project.sprites) drawTriangle(ctx, canvas.width / 2 + sprite.x * sx, canvas.height / 2 - sprite.y * sy, sprite.size * Math.min(sx, sy), sprite.direction, rgbaCss(sprite.color));
}
function drawTriangle(ctx, x, y, size, direction, color) {
  const angle = -direction * Math.PI / 180; const points = [angle, angle + 2.45, angle - 2.45].map((a, i) => { const radius = i === 0 ? size : size * 0.75; return [x + Math.cos(a) * radius, y + Math.sin(a) * radius]; });
  ctx.beginPath(); ctx.moveTo(...points[0]); ctx.lineTo(...points[1]); ctx.lineTo(...points[2]); ctx.closePath(); ctx.fillStyle = color; ctx.fill();
}

function changed() { render(); }
function setCanonicalCode() { elements.codeEditor.value = formatProject(project); markCodeValid(); }
function parseCodeNow() {
  try { project = reuseIds(project, parseProjectSource(elements.codeEditor.value)); syncSelection(); markCodeValid(); render({ syncCode: false }); }
  catch (error) { elements.codeStatus.textContent = 'invalid — blocks kept at last valid program'; elements.codeStatus.classList.add('invalid'); elements.codeError.textContent = error.message; elements.codeError.hidden = false; }
}
function markCodeValid() { elements.codeStatus.textContent = 'valid'; elements.codeStatus.classList.remove('invalid'); elements.codeError.hidden = true; }
function rgbaToHex([r, g, b]) { return `#${[r, g, b].map(value => value.toString(16).padStart(2, '0')).join('')}`; }
function hexToRgba(hex) { return [1, 3, 5].map(index => Number.parseInt(hex.slice(index, index + 2), 16)).concat(255); }
function rgbaCss([r, g, b, a]) { return `rgba(${r},${g},${b},${a / 255})`; }
function download(content, name, type) { const url = URL.createObjectURL(new Blob([content], { type })); const a = document.createElement('a'); a.href = url; a.download = name; a.click(); URL.revokeObjectURL(url); }
function clampInt(value, min, max, fallback) { const parsed = Number(value); return Number.isInteger(parsed) ? Math.max(min, Math.min(max, parsed)) : fallback; }

for (const button of document.querySelectorAll('[data-op]')) button.addEventListener('click', () => { const command = commandFor(button.dataset.op); selectedSprite().script.push(command); selectedCommandId = command.id; changed(); });
for (const button of document.querySelectorAll('[data-mode]')) button.addEventListener('click', () => { mode = button.dataset.mode; renderMode(); });
document.querySelector('#addSprite').addEventListener('click', () => { const sprite = makeSprite(`Sprite ${project.sprites.length + 1}`); project.sprites.push(sprite); selectedSpriteId = sprite.id; selectedCommandId = null; changed(); });
document.querySelector('#clearScript').addEventListener('click', () => { selectedSprite().script = []; selectedCommandId = null; changed(); });
document.querySelector('#resetButton').addEventListener('click', () => { project = clone(demoProject); selectedSpriteId = project.sprites[0].id; selectedCommandId = project.sprites[0].script[0].id; setCanonicalCode(); render(); });
document.querySelector('#exportCodeButton').addEventListener('click', () => download(codeFocused ? elements.codeEditor.value : formatProject(project), `${project.name || 'project'}.bn`, 'text/plain'));
document.querySelector('#exportButton').addEventListener('click', () => download(JSON.stringify(project, null, 2), `${project.name || 'project'}.json`, 'application/json'));
document.querySelector('#importCodeFile').addEventListener('change', async event => { const file = event.target.files[0]; if (!file) return; try { project = reuseIds(project, parseProjectSource(await file.text())); selectedSpriteId = project.sprites[0]?.id ?? null; selectedCommandId = project.sprites[0]?.script?.[0]?.id ?? null; elements.codeEditor.value = formatProject(project); markCodeValid(); render({ syncCode: false }); } catch (error) { alert(`Code import failed: ${error.message}`); } event.target.value = ''; });
document.querySelector('#importFile').addEventListener('change', async event => { const file = event.target.files[0]; if (!file) return; try { const next = ensureProjectIds(JSON.parse(await file.text())); if (next.version !== 1 || !next.stage || !Array.isArray(next.sprites)) throw new Error('Not a Block Native v1 project'); project = next; selectedSpriteId = project.sprites[0]?.id ?? null; selectedCommandId = project.sprites[0]?.script?.[0]?.id ?? null; render(); } catch (error) { alert(`Import failed: ${error.message}`); } event.target.value = ''; });
elements.projectName.addEventListener('change', () => { project.name = elements.projectName.value.trim() || 'project'; changed(); });
elements.stageWidth.addEventListener('change', () => { project.stage.width = clampInt(elements.stageWidth.value, 1, 65535, project.stage.width); changed(); });
elements.stageHeight.addEventListener('change', () => { project.stage.height = clampInt(elements.stageHeight.value, 1, 65535, project.stage.height); changed(); });
elements.stageBackground.addEventListener('change', () => { project.stage.background = hexToRgba(elements.stageBackground.value); changed(); });
elements.codeEditor.addEventListener('focus', () => { codeFocused = true; }); elements.codeEditor.addEventListener('blur', () => { codeFocused = false; });
elements.codeEditor.addEventListener('input', () => { clearTimeout(codeTimer); codeTimer = setTimeout(parseCodeNow, 250); });
elements.codeEditor.addEventListener('keydown', event => { if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') { event.preventDefault(); parseCodeNow(); } if (event.key === 'Tab') { event.preventDefault(); elements.codeEditor.setRangeText('  ', elements.codeEditor.selectionStart, elements.codeEditor.selectionEnd, 'end'); } });
document.querySelector('#formatCode').addEventListener('click', () => { parseCodeNow(); if (!elements.codeStatus.classList.contains('invalid')) setCanonicalCode(); });
elements.commandSearch.addEventListener('input', () => { paletteIndex = 0; renderPalette(); });
elements.commandSearch.addEventListener('keydown', event => { const options = paletteOptions(); if (event.key === 'ArrowDown') { event.preventDefault(); paletteIndex = Math.min(options.length - 1, paletteIndex + 1); renderPalette(); } if (event.key === 'ArrowUp') { event.preventDefault(); paletteIndex = Math.max(0, paletteIndex - 1); renderPalette(); } if (event.key === 'Enter') { event.preventDefault(); insertFromPalette(); } });
document.addEventListener('keydown', event => { if (elements.commandPalette.open || codeFocused) return; if (event.ctrlKey && event.code === 'Space') { const found = selectedCommandId ? findCommand(selectedCommandId) : null; event.preventDefault(); openPalette(found ? { list: found.list, index: found.index } : { list: selectedSprite().script, index: -1 }); } });

setCanonicalCode();
render();
