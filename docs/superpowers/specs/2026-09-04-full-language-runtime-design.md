# Full Language Runtime Design

## Goal

Turn Block Native from a movement demo into a small but usable Scratch-like programming system where the same project can be edited as keyboard-first blocks or text, compiled from `.json` or `.bn`, packaged with assets as `.bnp`, and run in the native runtime.

## Language model

`Project` remains the single semantic model shared by editor, text parser, compiler, and runtime.

Values are `number`, `bool`, `string`, and `list`. Expressions include literals, variables, unary `-`/`not`, arithmetic, comparisons, boolean operators, list length, keyboard state, and sprite collision queries.

Projects have global variables/lists. Sprites have local variables/lists, event scripts, procedures, optional costume asset, and the existing transform/color fields.

Events supported in this milestone:
- `when start`
- `when key "..."`
- `when message "..."`

Commands supported:
- motion: `move`, `turn`
- timing: `wait`
- state: `set`, `change`
- control: `repeat`, `while`, `if`/`else`
- lists: `push ... to ...`
- events: `broadcast`
- procedures: `call`
- pen: `pen down`, `pen up`, `pen clear`
- audio: `play "asset-name"`

Procedures take positional parameters and execute cooperatively in the same thread as their caller.

## Text syntax

Example:

```text
project "demo" {
  stage 480 360 background #f5f7faff
  global score = 0
  list points = []

  sprite "Cat" at 0 0 direction 0 size 32 color #4c97ffff costume "cat.png" {
    var speed = 4

    when start {
      repeat 10 {
        move speed
        change score by 1
      }
      broadcast "done"
    }

    when key "space" {
      if key("left") {
        turn -15
      }
    }

    proc hop(amount) {
      move amount
      wait 0.05
    }
  }
}
```

Expression precedence follows conventional programming languages: unary, multiplicative, additive, comparisons/equality, `and`, then `or`.

## Runtime architecture

The current flattened `Move/Turn/Wait` stream is replaced by BLK2 compiled instructions and a cooperative thread scheduler.

Each runnable event owns a thread with a program counter, value stack, call frames, wait state, and sprite identity. A bounded instruction budget prevents infinite loops from freezing rendering. `wait` yields. `broadcast` starts matching message scripts. key events are edge-triggered by the native runner.

Variables resolve local first then global. Procedures resolve within the current sprite. Lists follow the same local-first lookup rule.

## Bytecode format

The file starts with `BLK2` and a bytecode version. The payload is a serde-serialized compiled `Program` containing instruction vectors, procedure entry points, event entry points, constants, stage metadata, sprite metadata, and asset metadata.

Using a serialized compiled instruction model keeps the VM genuinely instruction-based while avoiding a large hand-maintained binary codec during this milestone.

## Assets and `.bnp`

Project assets are named entries with kinds `image` or `sound` and source paths in editable project files.

A `.bnp` file is a ZIP container:

```text
program.bcode
assets/<asset-name>
manifest.json
```

`blockc` accepts `.json` and `.bn`. When output ends in `.bnp`, it packages the compiled program plus referenced assets. Otherwise it writes `.bcode`.

`blockrun` accepts `.bcode` or `.bnp`. Image assets are decoded from package bytes into Macroquad textures; sound assets use Macroquad's byte-loading audio API.

## Rendering and interaction

Without a costume, a sprite keeps the current directional triangle fallback. With a costume, the runtime draws the texture centered at the sprite position with scale and rotation.

Collision uses axis-aligned bounds for this milestone: costume dimensions when available, otherwise the fallback sprite size. This makes `touching("Sprite")` deterministic and inexpensive.

Pen draws line segments into an in-memory list whenever a pen-down sprite moves. `pen clear` clears the stage pen layer.

## Editor

The browser editor remains dependency-free. It gains blocks for expressions, variables, lists, control flow, events, procedures, broadcast, pen, and sound.

Keyboard-first behavior remains mandatory:
- arrows navigate structural siblings/parents/children
- `Ctrl+Space` opens a searchable insertion palette
- `Enter` inserts after selection
- `Shift+Enter` inserts into container bodies
- `Alt+Up/Down` reorders
- `Delete` removes
- `Tab` moves through editable fields

Blocks and text continue to round-trip through one AST. Code parse errors preserve the last valid AST.

## Browser/native boundary

A normal web page cannot launch arbitrary local executables. Therefore this milestone does not pretend the web editor can directly start `blockrun`. The editor exports `.bn`/`.json`; native compilation/execution is via `blockc` and `blockrun`. A future desktop shell or registered local protocol can bridge that boundary.

## Compatibility

Project format becomes version 2. The parser/compiler accepts version-1 JSON and upgrades the old single `script` into an implicit `when start` script. BLK1 bytecode remains readable by the runtime for existing demos; new compilation emits BLK2.

## Testing

Required automated coverage:
- JavaScript parser/formatter round-trips and stable IDs
- Rust `.bn` parser equivalence with representative JSON
- v1 project upgrade
- expression evaluation and variable/list lookup
- `if`, `while`, `repeat`, procedure calls, waits, broadcasts, key events
- pen path generation and collision query
- BLK2 compile/decode round-trip and BLK1 compatibility
- `.bnp` package create/read with in-memory test assets
- Linux and Windows release builds through GitHub Actions

## Non-goals for this milestone

Pixel-perfect Scratch compatibility, clones, cloud variables, networking, physics, a full debugger, and a desktop editor shell are deliberately excluded. The goal is a coherent usable language/runtime, not a complete Scratch reimplementation.