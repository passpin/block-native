# Block Native

Block Native is an experiment in making visual programming feel less mouse-bound. A project has one AST and two equivalent editing surfaces: **keyboard-first blocks** and **text code**. The same JSON AST feeds a small Rust compiler and native runtime.

## Current pipeline

```text
Blocks ─┐
        ├─ shared Project AST ─ JSON ─ blockc ─ BLK1 bytecode ─ blockrun ─ native window
Code ───┘
```

The editor has **Blocks**, **Code**, and **Split** modes. Valid code edits update the block view and stage preview. Invalid/incomplete code stays in the text buffer while the last valid AST remains active, so temporarily broken source does not destroy the visual program.

## Text language

`examples/demo.bn`:

```text
project "triangle-demo" {
  stage 480 360 background #f5f7faff
  sprite "Sprite 1" at -80 0 direction 0 size 26 color #4c97ffff {
    repeat 4 {
      move 80
      turn 90
      wait 0.15
    }
  }
}
```

The current language deliberately stays small. Every text construct has a block representation and every block has a text representation: project/stage settings, sprites, `move`, `turn`, `wait`, and nested `repeat`.

The browser editor can import/export both `.json` and `.bn` source. The native `blockc` CLI currently consumes the JSON AST; `.bn` is parsed by the editor into that same AST before JSON export.

## Keyboard-only block editing

Focus a block and use:

| Key | Action |
| --- | --- |
| `↑` / `↓` | Previous / next sibling block |
| `←` | Parent repeat |
| `→` | First child of a repeat |
| `Enter` | Open command palette and insert after |
| `Shift+Enter` | Insert inside the selected repeat |
| `Ctrl+Space` | Open command palette |
| `Alt+↑` / `Alt+↓` | Reorder block |
| `Delete` / `Backspace` | Delete block |
| `Tab` | Move into editable numeric fields |

The goal is simple: visual structure should not force constant mouse travel.

## Run the editor

The editor is static HTML/CSS/JavaScript. Serve the repository directory with any local static server and open `editor/index.html` through that server; ES modules are used.

## Compile and run natively

```bash
cargo run --bin blockc -- examples/demo.json
cargo run --bin blockrun -- examples/demo.bcode
```

Release binaries:

```bash
cargo build --release --bins
```

`blockrun` opens a native GPU-backed window using Macroquad. The runtime currently draws directional triangle sprites and executes `move`, `turn`, `wait`, and compiled `repeat` instructions.

## Project JSON

Every sprite and command may carry a stable `id`. IDs are editor metadata; the bytecode compiler ignores them. Old JSON without IDs remains accepted by the Rust model through serde defaults.

## CI

GitHub Actions runs:

1. JavaScript syntax and language parser/formatter tests.
2. `cargo fmt --check`.
3. `cargo test --all-targets`.
4. Linux and Windows release builds.
5. Artifact upload for `blockc` and `blockrun` binaries.

## Next directions

The next natural step is a Scratch-like zipped project container containing `project.json` plus costumes/sounds, followed by more blocks/events and first-class native compilation from `.bn` source.

MIT licensed.
