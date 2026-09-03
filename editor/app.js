import {
  ensureProjectIds,
  formatExpression,
  formatProject,
  makeId,
  parseExpression,
  parseProjectSource,
  reuseIds,
  upgradeProject,
} from './language.mjs';

const clone = value => JSON.parse(JSON.stringify(value));
const lit = value => ({ kind: 'literal', value });

const demoProject = ensureProjectIds({
  version: 2,
  name: 'keyboard-demo',
  stage: { width: 480, height: 360, background: [245, 247, 250, 255] },
  globals: [{ name: 'score', value: 0 }],
  lists: [{ name: 'events', items: [] }],
  assets: [],
  sprites: [{
    id: 'sprite_demo',
    name: 'Sprite 1',
    x: -70,
    y: 0,
    direction: 0,
    size: 26,
    color: [76, 151, 255, 255],
    costume: null,
    variables: [{ name: 'speed', value: 8 }],
    lists: [{ name: 'trail', items: [] }],
    scripts: [
      {
        id: 'script_start',
        event: { kind: 'start' },
        body: [
          {
            id: 'repeat_demo',
            op: 'repeat',
            times: lit(4),
            body: [
              { id: 'move_demo', op: 'move', steps: { kind: 'var', name: 'speed' } },
              { id: 'turn_demo', op: 'turn', degrees: lit(90) },
              { id: 'change_demo', op: 'change', name: 'score', delta: lit(1) },
              { id: 'wait_demo', op: 'wait', seconds: lit(0.08) },
            ],
          },
        ],
      },
      {
        id: 'script_key',
        event: { kind: 'key', key: 'space' },
        body: [{ id: 'call_demo', op: 'call', name: 'hop', args: [lit(20)] }],
      },
    ],
    procedures: [{
      id: 'proc_demo',
      name: 'hop',
      params: ['amount'],
      body: [
        { id: 'pen_down_demo', op: 'pen_down' },
        { id: 'proc_move_demo', op: 'move', steps: { kind: 'var', name: 'amount' } },
        { id: 'pen_up_demo', op: 'pen_up' },
      ],
    }],
  }],
});

const BLOCK_OPTIONS = [
  { op: 'move', label: 'move', group: 'motion', keywords: 'motion steps' },
  { op: 'turn', label: 'turn', group: 'motion', keywords: 'motion rotate degrees' },
  { op: 'wait', label: 'wait', group: 'control', keywords: 'control time delay' },
  { op: 'repeat', label: 'repeat', group: 'control', keywords: 'control loop count' },
  { op: 'while', label: 'while', group: 'control', keywords: 'control loop condition' },
  { op: 'if', label: 'if / else', group: 'control', keywords: 'control condition branch' },
  { op: 'set', label: 'set variable', group: 'state', keywords: 'variable state assign' },
  { op: 'change', label: 'change variable', group: 'state', keywords: 'variable state add' },
  { op: 'push', label: 'push to list', group: 'state', keywords: 'list append state' },
  { op: 'broadcast', label: 'broadcast', group: 'event', keywords: 'event message' },
  { op: 'call', label: 'call procedure', group: 'procedure', keywords: 'procedure function custom block' },
  { op: 'pen_down', label: 'pen down', group: 'pen', keywords: 'draw line' },
  { op: 'pen_up', label: 'pen up', group: 'pen', keywords: 'draw stop' },
  { op: 'pen_clear', label: 'pen clear', group: 'pen', keywords: 'draw erase' },
  { op: 'play', label: 'play sound', group: 'sound', keywords: 'sound audio' },
];

let project = clone(demoProject);
let selectedSpriteId = project.sprites[0].id;
let selectedContainerId = project.sprites[0].scripts[0].id;
let selectedCommandId = project.sprites[0].scripts[0].body[0].id;
let mode = 'blocks';
let codeFocused = false;
let codeTimer = null;
let paletteContext = null;
let paletteIndex = 0;

const elements = {
  programBlocks: document.querySelector('#programBlocks'),
  blockPane: document.querySelector('#blockPane'),
  codePane: document.querySelector('#codePane'),
  editorArea: document.querySelector('.editor-area'),
  codeEditor: document.querySelector('#codeEditor'),
  codeStatus: document.querySelector('#codeStatus'),
  codeError: document.querySelector('#codeError'),
  spriteList: document.querySelector('#spriteList'),
  spriteEditor: document.querySelector('#spriteEditor'),
  selectedSpriteLabel: document.querySelector('#selectedSpriteLabel'),
  jsonPreview: document.querySelector('#jsonPreview'),
  stage: document.querySelector('#stage'),
  projectName: document.querySelector('#projectName'),
  stageWidth: document.querySelector('#stageWidth'),
  stageHeight: document.querySelector('#stageHeight'),
  stageBackground: document.querySelector('#stageBackground'),
  globalsEditor: document.querySelector('#globalsEditor'),
  globalListsEditor: document.querySelector('#globalListsEditor'),
  assetsEditor: document.querySelector('#assetsEditor'),
  quickPalette: document.querySelector('#quickPalette'),
  commandPalette: document.querySelector('#commandPalette'),
  commandSearch: document.querySelector('#commandSearch'),
  commandResults: document.querySelector('#commandResults'),
};

function selectedSprite() {
  return project.sprites.find(sprite => sprite.id === selectedSpriteId) ?? project.sprites[0];
}

function containers(sprite = selectedSprite()) {
  if (!sprite) return [];
  return [
    ...(sprite.scripts ?? []).map(value => ({ kind: 'script', value, body: value.body })),
    ...(sprite.procedures ?? []).map(value => ({ kind: 'procedure', value, body: value.body })),
  ];
}

function selectedContainer() {
  return containers().find(entry => entry.value.id === selectedContainerId) ?? containers()[0] ?? null;
}

function makeSprite(name) {
  return {
    id: makeId(), name, x: 0, y: 0, direction: 0, size: 26,
    color: [76, 151, 255, 255], costume: null, variables: [], lists: [],
    scripts: [{ id: makeId(), event: { kind: 'start' }, body: [] }], procedures: [],
  };
}

function commandFor(op) {
  const id = makeId();
  if (op === 'move') return { id, op, steps: lit(10) };
  if (op === 'turn') return { id, op, degrees: lit(15) };
  if (op === 'wait') return { id, op, seconds: lit(0.2) };
  if (op === 'repeat') return { id, op, times: lit(10), body: [] };
  if (op === 'while') return { id, op, condition: lit(true), body: [] };
  if (op === 'if') return { id, op, condition: lit(true), then_body: [], else_body: [] };
  if (op === 'set') return { id, op, name: firstVariableName(), value: lit(0) };
  if (op === 'change') return { id, op, name: firstVariableName(), delta: lit(1) };
  if (op === 'push') return { id, op, list: firstListName(), value: lit(0) };
  if (op === 'broadcast') return { id, op, message: 'message' };
  if (op === 'call') return { id, op, name: selectedSprite()?.procedures?.[0]?.name ?? 'procedure', args: [] };
  if (op === 'pen_down' || op === 'pen_up' || op === 'pen_clear') return { id, op };
  if (op === 'play') return { id, op, sound: project.assets.find(asset => asset.kind === 'sound')?.name ?? 'sound' };
  throw new Error(`Unknown block ${op}`);
}

function firstVariableName() {
  return selectedSprite()?.variables?.[0]?.name ?? project.globals?.[0]?.name ?? 'value';
}

function firstListName() {
  return selectedSprite()?.lists?.[0]?.name ?? project.lists?.[0]?.name ?? 'items';
}

function findCommand(id) {
  for (const container of containers()) {
    const found = findCommandInList(id, container.body ?? [], null, container, 'body');
    if (found) return found;
  }
  return null;
}

function findCommandInList(id, list, parentCommand, parentContainer, branch) {
  for (let index = 0; index < list.length; index += 1) {
    const command = list[index];
    if (command.id === id) return { command, list, index, parentCommand, parentContainer, branch };
    if (command.op === 'repeat' || command.op === 'while') {
      const found = findCommandInList(id, command.body ?? [], command, parentContainer, 'body');
      if (found) return found;
    } else if (command.op === 'if') {
      const thenFound = findCommandInList(id, command.then_body ?? [], command, parentContainer, 'then');
      if (thenFound) return thenFound;
      const elseFound = findCommandInList(id, command.else_body ?? [], command, parentContainer, 'else');
      if (elseFound) return elseFound;
    }
  }
  return null;
}

function primaryBody(command) {
  if (command.op === 'repeat' || command.op === 'while') return command.body;
  if (command.op === 'if') return command.then_body;
  return null;
}

function firstChildId(command) {
  return primaryBody(command)?.[0]?.id ?? (command.op === 'if' ? command.else_body?.[0]?.id : null);
}

function syncSelection() {
  if (!project.sprites.length) project.sprites.push(makeSprite('Sprite 1'));
  if (!project.sprites.some(sprite => sprite.id === selectedSpriteId)) selectedSpriteId = project.sprites[0].id;
  const all = containers();
  if (!all.some(entry => entry.value.id === selectedContainerId)) selectedContainerId = all[0]?.value.id ?? null;
  if (selectedCommandId && !findCommand(selectedCommandId)) selectedCommandId = selectedContainer()?.body?.[0]?.id ?? null;
}

function render({ syncCode = true } = {}) {
  syncSelection();
  renderMode();
  elements.projectName.value = project.name;
  elements.stageWidth.value = project.stage.width;
  elements.stageHeight.value = project.stage.height;
  elements.stageBackground.value = rgbaToHex(project.stage.background);
  elements.selectedSpriteLabel.textContent = selectedSprite()?.name ?? '';
  renderQuickPalette();
  renderProjectData();
  renderProgramBlocks();
  renderSprites();
  renderStage();
  elements.jsonPreview.textContent = JSON.stringify(project, null, 2);
  if (syncCode && !codeFocused) setCanonicalCode();
}

function renderMode() {
  document.querySelectorAll('[data-mode]').forEach(button => button.classList.toggle('selected', button.dataset.mode === mode));
  elements.blockPane.hidden = mode === 'code';
  elements.codePane.hidden = mode === 'blocks';
  elements.editorArea.classList.toggle('split', mode === 'split');
}

function renderQuickPalette() {
  elements.quickPalette.replaceChildren();
  for (const option of BLOCK_OPTIONS) {
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = option.label;
    button.className = option.group;
    button.addEventListener('click', () => insertQuick(option.op));
    elements.quickPalette.append(button);
  }
}

function insertQuick(op) {
  const found = selectedCommandId ? findCommand(selectedCommandId) : null;
  if (found) insertCommand(found.list, found.index + 1, op);
  else {
    const container = selectedContainer();
    if (container) insertCommand(container.body, container.body.length, op);
  }
}

function renderProjectData() {
  renderVariableRows(elements.globalsEditor, project.globals, () => changed());
  renderListRows(elements.globalListsEditor, project.lists, () => changed());
  renderAssetRows();
}

function renderVariableRows(container, variables, onChange) {
  container.replaceChildren();
  variables.forEach((variable, index) => {
    const row = document.createElement('div');
    row.className = 'data-row';
    const name = textInput(variable.name, value => { variable.name = normalizedIdentifier(value, `value${index + 1}`); onChange(); });
    const value = textInput(formatLiteralInput(variable.value), raw => {
      try { variable.value = parsePrimitive(raw); value.setCustomValidity(''); onChange(); }
      catch (error) { value.setCustomValidity(error.message); value.reportValidity(); }
    });
    const remove = removeButton(() => { variables.splice(index, 1); onChange(); });
    row.append(name, value, remove);
    container.append(row);
  });
}

function renderListRows(container, lists, onChange) {
  container.replaceChildren();
  lists.forEach((list, index) => {
    const row = document.createElement('div');
    row.className = 'data-row';
    const name = textInput(list.name, value => { list.name = normalizedIdentifier(value, `list${index + 1}`); onChange(); });
    const value = textInput(JSON.stringify(list.items ?? []), raw => {
      try { list.items = parsePrimitiveList(raw); value.setCustomValidity(''); onChange(); }
      catch (error) { value.setCustomValidity(error.message); value.reportValidity(); }
    });
    row.append(name, value, removeButton(() => { lists.splice(index, 1); onChange(); }));
    container.append(row);
  });
}

function renderAssetRows() {
  elements.assetsEditor.replaceChildren();
  project.assets.forEach((asset, index) => {
    const row = document.createElement('div'); row.className = 'data-row asset-row';
    const kind = document.createElement('select');
    for (const optionValue of ['image', 'sound']) {
      const option = document.createElement('option'); option.value = optionValue; option.textContent = optionValue; kind.append(option);
    }
    kind.value = asset.kind;
    stopKeyboardPropagation(kind);
    kind.addEventListener('change', () => { asset.kind = kind.value; changed(); });
    const name = textInput(asset.name, value => { asset.name = value.trim() || `asset${index + 1}`; changed(); });
    const path = textInput(asset.path, value => { asset.path = value; changed(); });
    row.append(kind, name, path, removeButton(() => { project.assets.splice(index, 1); changed(); }));
    elements.assetsEditor.append(row);
  });
}

function renderProgramBlocks() {
  elements.programBlocks.replaceChildren();
  const entries = containers();
  if (!entries.length) {
    const empty = document.createElement('div'); empty.className = 'empty-body'; empty.tabIndex = 0;
    empty.textContent = 'No event scripts or procedures. Use + start / + key / + message / + proc.';
    elements.programBlocks.append(empty);
    return;
  }
  for (const entry of entries) elements.programBlocks.append(renderContainer(entry));
}

function renderContainer(entry) {
  const container = document.createElement('section');
  container.className = `stack-container${entry.value.id === selectedContainerId ? ' selected' : ''}`;
  container.dataset.containerId = entry.value.id;
  container.tabIndex = 0;
  container.addEventListener('focus', event => {
    if (event.target !== container) return;
    selectedContainerId = entry.value.id;
    selectedCommandId = null;
    updateSelectionClasses();
  });
  container.addEventListener('keydown', event => {
    if (event.target !== container) return;
    if (event.key === 'Delete' || event.key === 'Backspace') { event.preventDefault(); deleteContainer(entry); return; }
    if (event.key === 'ArrowDown' && entry.body[0]) { event.preventDefault(); focusBlock(entry.body[0].id); return; }
    if (event.key === 'Enter' || (event.ctrlKey && event.code === 'Space')) {
      event.preventDefault(); openPalette({ list: entry.body, index: entry.body.length - 1 });
    }
  });

  const head = document.createElement('div'); head.className = 'stack-head';
  const type = document.createElement('strong');
  const fields = document.createElement('div'); fields.className = 'inline-fields';
  if (entry.kind === 'script') {
    type.textContent = 'event';
    renderEventFields(entry.value, fields);
  } else {
    type.textContent = 'procedure';
    const name = textInput(entry.value.name, value => { entry.value.name = normalizedIdentifier(value, 'procedure'); changed(); });
    name.placeholder = 'name';
    const params = textInput((entry.value.params ?? []).join(', '), value => {
      entry.value.params = value.split(',').map(item => item.trim()).filter(Boolean).map((item, index) => normalizedIdentifier(item, `arg${index + 1}`));
      changed();
    });
    params.placeholder = 'a, b';
    fields.append(name, params);
  }
  const remove = removeButton(() => deleteContainer(entry));
  head.append(type, fields, remove);
  container.append(head);

  const body = document.createElement('div'); body.className = 'stack-body';
  renderCommandList(entry.body, body, null, entry, 'body');
  container.append(body);
  return container;
}

function renderEventFields(script, fields) {
  const kind = document.createElement('select');
  for (const value of ['start', 'key', 'message']) {
    const option = document.createElement('option'); option.value = value; option.textContent = value; kind.append(option);
  }
  kind.value = script.event.kind;
  stopKeyboardPropagation(kind);
  kind.addEventListener('change', () => {
    script.event = kind.value === 'start'
      ? { kind: 'start' }
      : kind.value === 'key'
        ? { kind: 'key', key: 'space' }
        : { kind: 'message', message: 'message' };
    changed();
  });
  fields.append(kind);
  if (script.event.kind === 'key') {
    const input = textInput(script.event.key, value => { script.event.key = value.trim().toLowerCase() || 'space'; changed(); });
    input.placeholder = 'space'; fields.append(input);
  } else if (script.event.kind === 'message') {
    const input = textInput(script.event.message, value => { script.event.message = value || 'message'; changed(); });
    input.placeholder = 'message'; fields.append(input);
  }
}

function renderCommandList(list, container, parentCommand, parentContainer, branch) {
  if (!list.length) {
    const empty = document.createElement('div');
    empty.className = 'empty-body'; empty.tabIndex = 0; empty.textContent = 'empty — Enter or Ctrl+Space inserts here';
    empty.dataset.parentCommand = parentCommand?.id ?? '';
    empty.dataset.parentContainer = parentContainer?.value?.id ?? '';
    empty.dataset.branch = branch;
    empty.addEventListener('keydown', event => {
      if (event.key === 'ArrowLeft') {
        event.preventDefault();
        if (parentCommand) focusBlock(parentCommand.id); else focusContainer(parentContainer?.value?.id);
      } else if (event.key === 'Enter' || (event.ctrlKey && event.code === 'Space')) {
        event.preventDefault(); openPalette({ list, index: -1 });
      }
    });
    container.append(empty);
    return;
  }
  list.forEach(command => container.append(renderBlock(command, list, parentCommand, parentContainer, branch)));
}

function renderBlock(command, list, parentCommand, parentContainer, branch) {
  const node = document.createElement('article');
  const option = BLOCK_OPTIONS.find(entry => entry.op === command.op);
  node.className = `block ${option?.group ?? 'control'}${command.id === selectedCommandId ? ' selected' : ''}`;
  node.tabIndex = 0; node.dataset.id = command.id; node.dataset.op = command.op;
  node.addEventListener('focus', event => {
    if (event.target !== node) return;
    selectedCommandId = command.id; selectedContainerId = parentContainer.value.id; updateSelectionClasses();
  });
  node.addEventListener('keydown', event => handleBlockKey(event, command.id));

  const row = document.createElement('div'); row.className = 'block-row';
  const title = document.createElement('strong'); title.className = 'block-title'; title.textContent = option?.label ?? command.op;
  const actions = document.createElement('div'); actions.className = 'block-actions';
  actions.append(actionButton('↑', () => moveCommand(list, list.indexOf(command), -1, command.id)));
  actions.append(actionButton('↓', () => moveCommand(list, list.indexOf(command), 1, command.id)));
  actions.append(actionButton('×', () => deleteCommand(list, list.indexOf(command))));
  row.append(title, actions); node.append(row);

  const fields = document.createElement('div'); fields.className = 'block-fields';
  renderCommandFields(command, fields); node.append(fields);

  if (command.op === 'repeat' || command.op === 'while') {
    node.append(renderNestedBody('body', command.body, command, parentContainer, 'body'));
  } else if (command.op === 'if') {
    node.append(renderNestedBody('then', command.then_body, command, parentContainer, 'then'));
    node.append(renderNestedBody('else', command.else_body, command, parentContainer, 'else'));
  }
  return node;
}

function renderCommandFields(command, fields) {
  if (command.op === 'move') fields.append(expressionField('steps', command.steps, value => { command.steps = value; }));
  else if (command.op === 'turn') fields.append(expressionField('degrees', command.degrees, value => { command.degrees = value; }));
  else if (command.op === 'wait') fields.append(expressionField('seconds', command.seconds, value => { command.seconds = value; }));
  else if (command.op === 'repeat') fields.append(expressionField('times', command.times, value => { command.times = value; }));
  else if (command.op === 'while' || command.op === 'if') fields.append(expressionField('condition', command.condition, value => { command.condition = value; }));
  else if (command.op === 'set') {
    fields.append(fieldLabel('name', textInput(command.name, value => { command.name = normalizedIdentifier(value, 'value'); changed(); })));
    fields.append(expressionField('value', command.value, value => { command.value = value; }));
  } else if (command.op === 'change') {
    fields.append(fieldLabel('name', textInput(command.name, value => { command.name = normalizedIdentifier(value, 'value'); changed(); })));
    fields.append(expressionField('delta', command.delta, value => { command.delta = value; }));
  } else if (command.op === 'push') {
    fields.append(expressionField('value', command.value, value => { command.value = value; }));
    fields.append(fieldLabel('list', textInput(command.list, value => { command.list = normalizedIdentifier(value, 'items'); changed(); })));
  } else if (command.op === 'broadcast') {
    fields.append(fieldLabel('message', textInput(command.message, value => { command.message = value || 'message'; changed(); })));
  } else if (command.op === 'call') {
    fields.append(fieldLabel('procedure', textInput(command.name, value => { command.name = normalizedIdentifier(value, 'procedure'); changed(); })));
    const args = textInput((command.args ?? []).map(formatExpression).join(', '), raw => {
      try { command.args = parseArgumentList(raw); args.setCustomValidity(''); changed(); }
      catch (error) { args.setCustomValidity(error.message); args.reportValidity(); }
    });
    fields.append(fieldLabel('arguments', args));
  } else if (command.op === 'play') {
    fields.append(fieldLabel('sound', textInput(command.sound, value => { command.sound = value || 'sound'; changed(); })));
  }
}

function renderNestedBody(label, list, command, parentContainer, branch) {
  const nested = document.createElement('section'); nested.className = 'nested-body';
  const heading = document.createElement('div'); heading.className = 'nested-label';
  const text = document.createElement('span'); text.textContent = label;
  const add = document.createElement('button'); add.type = 'button'; add.className = 'ghost'; add.textContent = '+ block';
  add.addEventListener('click', event => { event.stopPropagation(); openPalette({ list, index: list.length - 1 }); });
  heading.append(text, add); nested.append(heading);
  const body = document.createElement('div'); renderCommandList(list, body, command, parentContainer, branch); nested.append(body);
  return nested;
}

function expressionField(label, expression, setter) {
  const input = textInput(formatExpression(expression), raw => {
    try {
      const parsed = parseExpression(raw); setter(parsed); input.classList.remove('invalid'); input.setCustomValidity(''); changed();
    } catch (error) {
      input.classList.add('invalid'); input.setCustomValidity(error.message); input.reportValidity();
    }
  });
  return fieldLabel(label, input);
}

function fieldLabel(text, control) {
  const label = document.createElement('label'); label.textContent = text; label.append(control); return label;
}

function textInput(value, onChange) {
  const input = document.createElement('input'); input.type = 'text'; input.value = value ?? '';
  stopKeyboardPropagation(input);
  input.addEventListener('change', () => onChange(input.value));
  return input;
}

function stopKeyboardPropagation(control) {
  control.addEventListener('keydown', event => event.stopPropagation());
}

function actionButton(text, handler) {
  const button = document.createElement('button'); button.type = 'button'; button.textContent = text;
  button.addEventListener('click', event => { event.stopPropagation(); handler(); }); return button;
}

function removeButton(handler) {
  const button = document.createElement('button'); button.type = 'button'; button.className = 'icon-button'; button.textContent = '×';
  button.title = 'Remove'; button.addEventListener('click', handler); return button;
}

function handleBlockKey(event, id) {
  const found = findCommand(id); if (!found) return;
  const { command, list, index, parentCommand, parentContainer } = found;
  if (event.altKey && event.key === 'ArrowUp') { event.preventDefault(); moveCommand(list, index, -1, id); return; }
  if (event.altKey && event.key === 'ArrowDown') { event.preventDefault(); moveCommand(list, index, 1, id); return; }
  if (event.key === 'Delete' || event.key === 'Backspace') { event.preventDefault(); deleteCommand(list, index); return; }
  if (event.key === 'ArrowUp') { event.preventDefault(); if (list[index - 1]) focusBlock(list[index - 1].id); else focusContainer(parentContainer.value.id); return; }
  if (event.key === 'ArrowDown') { event.preventDefault(); if (list[index + 1]) focusBlock(list[index + 1].id); return; }
  if (event.key === 'ArrowLeft') { event.preventDefault(); if (parentCommand) focusBlock(parentCommand.id); else focusContainer(parentContainer.value.id); return; }
  if (event.key === 'ArrowRight') {
    event.preventDefault();
    const child = firstChildId(command);
    if (child) focusBlock(child); else focusFirstEmpty(command.id);
    return;
  }
  if (event.ctrlKey && event.shiftKey && event.code === 'Space' && command.op === 'if') {
    event.preventDefault(); openPalette({ list: command.else_body, index: command.else_body.length - 1 }); return;
  }
  if (event.shiftKey && event.key === 'Enter') {
    const body = primaryBody(command);
    if (body) { event.preventDefault(); openPalette({ list: body, index: body.length - 1 }); return; }
  }
  if (event.key === 'Enter') { event.preventDefault(); openPalette({ list, index }); return; }
  if (event.ctrlKey && event.code === 'Space') { event.preventDefault(); openPalette({ list, index }); }
}

function focusFirstEmpty(commandId) {
  const node = elements.programBlocks.querySelector(`[data-id="${CSS.escape(commandId)}"]`);
  node?.querySelector('.empty-body')?.focus();
}

function moveCommand(list, index, delta, id) {
  const next = index + delta; if (next < 0 || next >= list.length) return;
  [list[index], list[next]] = [list[next], list[index]]; changed(); requestAnimationFrame(() => focusBlock(id));
}

function deleteCommand(list, index) {
  if (index < 0) return;
  const fallback = list[index + 1]?.id ?? list[index - 1]?.id ?? null;
  list.splice(index, 1); selectedCommandId = fallback; changed();
  requestAnimationFrame(() => fallback ? focusBlock(fallback) : focusContainer(selectedContainerId));
}

function deleteContainer(entry) {
  const sprite = selectedSprite();
  const list = entry.kind === 'script' ? sprite.scripts : sprite.procedures;
  const index = list.findIndex(value => value.id === entry.value.id);
  if (index >= 0) list.splice(index, 1);
  const all = containers(sprite); selectedContainerId = all[Math.min(index, all.length - 1)]?.value.id ?? null; selectedCommandId = null; changed();
}

function focusBlock(id) {
  selectedCommandId = id;
  const found = findCommand(id); if (found) selectedContainerId = found.parentContainer.value.id;
  updateSelectionClasses();
  elements.programBlocks.querySelector(`[data-id="${CSS.escape(id)}"]`)?.focus({ preventScroll: false });
}

function focusContainer(id) {
  if (!id) return;
  selectedContainerId = id; selectedCommandId = null; updateSelectionClasses();
  elements.programBlocks.querySelector(`[data-container-id="${CSS.escape(id)}"]`)?.focus({ preventScroll: false });
}

function updateSelectionClasses() {
  elements.programBlocks.querySelectorAll('.block.selected').forEach(node => node.classList.toggle('selected', node.dataset.id === selectedCommandId));
  elements.programBlocks.querySelectorAll('.stack-container').forEach(node => node.classList.toggle('selected', node.dataset.containerId === selectedContainerId));
}

function openPalette(context) {
  paletteContext = context; paletteIndex = 0; elements.commandSearch.value = ''; renderPalette(); elements.commandPalette.showModal();
  requestAnimationFrame(() => elements.commandSearch.focus());
}

function paletteOptions() {
  const query = elements.commandSearch.value.trim().toLowerCase();
  if (!query) return BLOCK_OPTIONS;
  return BLOCK_OPTIONS.filter(option => `${option.label} ${option.op} ${option.keywords}`.includes(query));
}

function renderPalette() {
  const options = paletteOptions();
  if (paletteIndex >= options.length) paletteIndex = Math.max(0, options.length - 1);
  elements.commandResults.replaceChildren();
  options.forEach((option, index) => {
    const button = document.createElement('button'); button.type = 'button'; button.classList.toggle('active', index === paletteIndex);
    const label = document.createElement('span'); label.textContent = option.label;
    const meta = document.createElement('small'); meta.textContent = option.keywords;
    button.append(label, meta); button.addEventListener('click', () => insertFromPalette(option.op)); elements.commandResults.append(button);
  });
}

function insertFromPalette(op = paletteOptions()[paletteIndex]?.op) {
  if (!op || !paletteContext) return;
  const index = paletteContext.index < 0 ? paletteContext.list.length : paletteContext.index + 1;
  insertCommand(paletteContext.list, index, op);
  elements.commandPalette.close();
}

function insertCommand(list, index, op) {
  const command = commandFor(op); list.splice(index, 0, command); selectedCommandId = command.id;
  changed(); requestAnimationFrame(() => focusBlock(command.id));
}

function renderSprites() {
  elements.spriteList.replaceChildren();
  for (const sprite of project.sprites) {
    const button = document.createElement('button'); button.className = `sprite-chip${sprite.id === selectedSpriteId ? ' selected' : ''}`; button.textContent = sprite.name;
    button.addEventListener('click', () => {
      selectedSpriteId = sprite.id; selectedContainerId = containers(sprite)[0]?.value.id ?? null; selectedCommandId = containers(sprite)[0]?.body?.[0]?.id ?? null; render();
    });
    elements.spriteList.append(button);
  }
  renderSpriteEditor();
}

function renderSpriteEditor() {
  const sprite = selectedSprite();
  if (!sprite) { elements.spriteEditor.replaceChildren(); return; }
  elements.spriteEditor.innerHTML = `
    <div class="sprite-grid">
      <label class="wide">Name<input data-sprite="name" type="text"></label>
      <label>X<input data-sprite="x" type="number"></label>
      <label>Y<input data-sprite="y" type="number"></label>
      <label>Direction<input data-sprite="direction" type="number"></label>
      <label>Size<input data-sprite="size" type="number" min="1"></label>
      <label>Color<input data-sprite="color" type="color"></label>
      <label>Costume asset<input data-sprite="costume" type="text" placeholder="asset name"></label>
    </div>
    <div class="local-state">
      <div class="section-head"><h3>Local variables</h3><button data-add-local="var" class="small secondary">+ var</button></div>
      <div data-local-vars class="row-editor"></div>
      <div class="section-head" style="margin-top:10px"><h3>Local lists</h3><button data-add-local="list" class="small secondary">+ list</button></div>
      <div data-local-lists class="row-editor"></div>
    </div>
    <button id="deleteSprite" class="danger ghost">Delete sprite</button>`;
  const set = (key, value) => { elements.spriteEditor.querySelector(`[data-sprite="${key}"]`).value = value ?? ''; };
  set('name', sprite.name); set('x', sprite.x); set('y', sprite.y); set('direction', sprite.direction); set('size', sprite.size); set('color', rgbaToHex(sprite.color)); set('costume', sprite.costume ?? '');
  elements.spriteEditor.querySelectorAll('[data-sprite]').forEach(input => {
    stopKeyboardPropagation(input);
    input.addEventListener('change', () => {
      const key = input.dataset.sprite;
      if (key === 'name') sprite.name = input.value.trim() || 'Sprite';
      else if (key === 'color') sprite.color = hexToRgba(input.value);
      else if (key === 'costume') sprite.costume = input.value.trim() || null;
      else { const value = Number(input.value); if (Number.isFinite(value)) sprite[key] = key === 'size' ? Math.max(1, value) : value; }
      changed();
    });
  });
  renderVariableRows(elements.spriteEditor.querySelector('[data-local-vars]'), sprite.variables, () => changed());
  renderListRows(elements.spriteEditor.querySelector('[data-local-lists]'), sprite.lists, () => changed());
  elements.spriteEditor.querySelector('[data-add-local="var"]').addEventListener('click', () => { sprite.variables.push({ name: uniqueName('value', sprite.variables), value: 0 }); changed(); });
  elements.spriteEditor.querySelector('[data-add-local="list"]').addEventListener('click', () => { sprite.lists.push({ name: uniqueName('items', sprite.lists), items: [] }); changed(); });
  elements.spriteEditor.querySelector('#deleteSprite').addEventListener('click', () => {
    if (project.sprites.length === 1) return;
    const index = project.sprites.findIndex(item => item.id === sprite.id); project.sprites.splice(index, 1);
    selectedSpriteId = project.sprites[Math.max(0, index - 1)].id; selectedContainerId = containers()[0]?.value.id ?? null; selectedCommandId = null; changed();
  });
}

function renderStage() {
  const canvas = elements.stage;
  canvas.width = Math.min(project.stage.width, 960); canvas.height = Math.min(project.stage.height, 720);
  const ctx = canvas.getContext('2d'); ctx.fillStyle = rgbaCss(project.stage.background); ctx.fillRect(0, 0, canvas.width, canvas.height);
  const sx = canvas.width / project.stage.width, sy = canvas.height / project.stage.height;
  for (const sprite of project.sprites) drawTriangle(ctx, canvas.width / 2 + sprite.x * sx, canvas.height / 2 - sprite.y * sy, sprite.size * Math.min(sx, sy), sprite.direction, rgbaCss(sprite.color));
}

function drawTriangle(ctx, x, y, size, direction, color) {
  const angle = -direction * Math.PI / 180;
  const points = [angle, angle + 2.45, angle - 2.45].map((a, index) => {
    const radius = index === 0 ? size : size * 0.75; return [x + Math.cos(a) * radius, y + Math.sin(a) * radius];
  });
  ctx.beginPath(); ctx.moveTo(...points[0]); ctx.lineTo(...points[1]); ctx.lineTo(...points[2]); ctx.closePath(); ctx.fillStyle = color; ctx.fill();
}

function addScript(kind) {
  const event = kind === 'start' ? { kind: 'start' } : kind === 'key' ? { kind: 'key', key: 'space' } : { kind: 'message', message: 'message' };
  const script = { id: makeId(), event, body: [] }; selectedSprite().scripts.push(script); selectedContainerId = script.id; selectedCommandId = null; changed();
  requestAnimationFrame(() => focusContainer(script.id));
}

function addProcedure() {
  const procedure = { id: makeId(), name: uniqueProcedureName(), params: [], body: [] };
  selectedSprite().procedures.push(procedure); selectedContainerId = procedure.id; selectedCommandId = null; changed(); requestAnimationFrame(() => focusContainer(procedure.id));
}

function uniqueProcedureName() {
  const names = new Set(selectedSprite().procedures.map(proc => proc.name));
  let index = 1; while (names.has(`procedure${index}`)) index += 1; return `procedure${index}`;
}

function uniqueName(base, entries) {
  const names = new Set(entries.map(entry => entry.name)); let index = 1; let candidate = base;
  while (names.has(candidate)) candidate = `${base}${++index}`; return candidate;
}

function normalizedIdentifier(value, fallback) {
  const normalized = value.trim().replace(/[^A-Za-z0-9_]/g, '_').replace(/^[^A-Za-z_]+/, '');
  return normalized || fallback;
}

function parsePrimitive(raw) {
  let value;
  try { value = JSON.parse(raw); } catch { throw new Error('Use JSON literal: 1, true, or "text"'); }
  if (!['number', 'boolean', 'string'].includes(typeof value) || (typeof value === 'number' && !Number.isFinite(value))) throw new Error('Value must be a finite number, boolean, or string');
  return value;
}

function parsePrimitiveList(raw) {
  let values;
  try { values = JSON.parse(raw); } catch { throw new Error('Use a JSON array, e.g. [1, "x"]'); }
  if (!Array.isArray(values)) throw new Error('List value must be a JSON array');
  return values.map(value => {
    if (!['number', 'boolean', 'string'].includes(typeof value) || (typeof value === 'number' && !Number.isFinite(value))) throw new Error('List elements must be primitive values');
    return value;
  });
}

function formatLiteralInput(value) {
  return JSON.stringify(value);
}

function parseArgumentList(raw) {
  const text = raw.trim(); if (!text) return [];
  const parts = []; let start = 0; let depth = 0; let quote = false; let escaped = false;
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    if (quote) {
      if (!escaped && char === '"') quote = false;
      escaped = !escaped && char === '\\'; if (char !== '\\') escaped = false; continue;
    }
    if (char === '"') { quote = true; continue; }
    if (char === '(') depth += 1;
    else if (char === ')') depth -= 1;
    else if (char === ',' && depth === 0) { parts.push(text.slice(start, index).trim()); start = index + 1; }
    if (depth < 0) throw new Error('Unbalanced parentheses');
  }
  if (quote || depth !== 0) throw new Error('Unbalanced string or parentheses');
  parts.push(text.slice(start).trim());
  return parts.filter(Boolean).map(parseExpression);
}

function changed() { render(); }
function setCanonicalCode() { elements.codeEditor.value = formatProject(project); markCodeValid(); }
function parseCodeNow() {
  try {
    const parsed = parseProjectSource(elements.codeEditor.value); project = reuseIds(project, parsed); syncSelection(); markCodeValid(); render({ syncCode: false });
  } catch (error) {
    elements.codeStatus.textContent = 'invalid — blocks kept at last valid program'; elements.codeStatus.classList.add('invalid'); elements.codeError.textContent = error.message; elements.codeError.hidden = false;
  }
}
function markCodeValid() { elements.codeStatus.textContent = 'valid'; elements.codeStatus.classList.remove('invalid'); elements.codeError.hidden = true; }

function rgbaToHex([r, g, b]) { return `#${[r, g, b].map(value => Number(value).toString(16).padStart(2, '0')).join('')}`; }
function hexToRgba(hex) { return [1, 3, 5].map(index => Number.parseInt(hex.slice(index, index + 2), 16)).concat(255); }
function rgbaCss([r, g, b, a]) { return `rgba(${r},${g},${b},${a / 255})`; }
function download(content, name, type) { const url = URL.createObjectURL(new Blob([content], { type })); const anchor = document.createElement('a'); anchor.href = url; anchor.download = name; anchor.click(); URL.revokeObjectURL(url); }
function clampInt(value, min, max, fallback) { const parsed = Number(value); return Number.isInteger(parsed) ? Math.max(min, Math.min(max, parsed)) : fallback; }

for (const button of document.querySelectorAll('[data-mode]')) button.addEventListener('click', () => { mode = button.dataset.mode; render(); });
document.querySelector('#addStartScript').addEventListener('click', () => addScript('start'));
document.querySelector('#addKeyScript').addEventListener('click', () => addScript('key'));
document.querySelector('#addMessageScript').addEventListener('click', () => addScript('message'));
document.querySelector('#addProcedure').addEventListener('click', addProcedure);
document.querySelector('#addSprite').addEventListener('click', () => {
  const sprite = makeSprite(`Sprite ${project.sprites.length + 1}`); project.sprites.push(sprite); selectedSpriteId = sprite.id; selectedContainerId = sprite.scripts[0].id; selectedCommandId = null; changed();
});
document.querySelector('#addGlobal').addEventListener('click', () => { project.globals.push({ name: uniqueName('value', project.globals), value: 0 }); changed(); });
document.querySelector('#addGlobalList').addEventListener('click', () => { project.lists.push({ name: uniqueName('items', project.lists), items: [] }); changed(); });
document.querySelector('#addAsset').addEventListener('click', () => { project.assets.push({ kind: 'image', name: `asset${project.assets.length + 1}`, path: 'assets/file.png' }); changed(); });

elements.projectName.addEventListener('change', () => { project.name = elements.projectName.value.trim() || 'project'; changed(); });
elements.stageWidth.addEventListener('change', () => { project.stage.width = clampInt(elements.stageWidth.value, 1, 65535, project.stage.width); changed(); });
elements.stageHeight.addEventListener('change', () => { project.stage.height = clampInt(elements.stageHeight.value, 1, 65535, project.stage.height); changed(); });
elements.stageBackground.addEventListener('change', () => { project.stage.background = hexToRgba(elements.stageBackground.value); changed(); });

elements.codeEditor.addEventListener('focus', () => { codeFocused = true; });
elements.codeEditor.addEventListener('blur', () => { codeFocused = false; });
elements.codeEditor.addEventListener('input', () => { clearTimeout(codeTimer); codeTimer = setTimeout(parseCodeNow, 250); });
document.querySelector('#formatCode').addEventListener('click', () => { parseCodeNow(); if (!elements.codeStatus.classList.contains('invalid')) setCanonicalCode(); });

elements.commandSearch.addEventListener('input', () => { paletteIndex = 0; renderPalette(); });
elements.commandSearch.addEventListener('keydown', event => {
  const options = paletteOptions();
  if (event.key === 'ArrowDown') { event.preventDefault(); paletteIndex = Math.min(paletteIndex + 1, Math.max(0, options.length - 1)); renderPalette(); }
  else if (event.key === 'ArrowUp') { event.preventDefault(); paletteIndex = Math.max(0, paletteIndex - 1); renderPalette(); }
  else if (event.key === 'Enter') { event.preventDefault(); insertFromPalette(); }
});

document.addEventListener('keydown', event => {
  if (event.ctrlKey && event.code === 'Space' && !elements.commandPalette.open && !['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement?.tagName)) {
    event.preventDefault();
    const found = selectedCommandId ? findCommand(selectedCommandId) : null;
    if (found) openPalette({ list: found.list, index: found.index });
    else if (selectedContainer()) openPalette({ list: selectedContainer().body, index: selectedContainer().body.length - 1 });
  }
});

document.querySelector('#exportButton').addEventListener('click', () => download(JSON.stringify(project, null, 2), `${safeName(project.name)}.json`, 'application/json'));
document.querySelector('#exportCodeButton').addEventListener('click', () => download(formatProject(project), `${safeName(project.name)}.bn`, 'text/plain'));
document.querySelector('#resetButton').addEventListener('click', () => {
  project = clone(demoProject); selectedSpriteId = project.sprites[0].id; selectedContainerId = project.sprites[0].scripts[0].id; selectedCommandId = project.sprites[0].scripts[0].body[0].id; changed();
});

document.querySelector('#importFile').addEventListener('change', async event => {
  const file = event.target.files?.[0]; if (!file) return;
  try {
    const parsed = upgradeProject(JSON.parse(await file.text())); project = parsed; selectedSpriteId = project.sprites[0]?.id; selectedContainerId = containers()[0]?.value.id ?? null; selectedCommandId = null; render();
  } catch (error) { alert(`Could not import JSON: ${error.message}`); }
  event.target.value = '';
});

document.querySelector('#importCodeFile').addEventListener('change', async event => {
  const file = event.target.files?.[0]; if (!file) return;
  try {
    project = parseProjectSource(await file.text()); selectedSpriteId = project.sprites[0]?.id; selectedContainerId = containers()[0]?.value.id ?? null; selectedCommandId = null; render(); markCodeValid();
  } catch (error) { alert(`Could not import code: ${error.message}`); }
  event.target.value = '';
});

function safeName(value) { return value.toLowerCase().replace(/[^a-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '') || 'project'; }

render();
