# Full Language Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand Block Native into a usable keyboard-first block/text language with expressions, state, control flow, events, procedures, assets, package files, and native execution.

**Architecture:** Keep `Project` as the shared semantic model. Compile it to BLK2 instruction programs executed by a cooperative scheduler. Keep the web editor dependency-free and add a matching Rust parser so `.bn` and JSON both compile natively.

**Tech Stack:** Rust 2021, serde/serde_json, macroquad 0.4.14, zip, browser ES modules, Node test runner, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-09-04-full-language-runtime-design.md`

## Global Constraints

- Preserve BLK1 runtime compatibility while new compilation emits BLK2.
- Accept version-1 JSON by upgrading the old sprite `script` into `when start`.
- Browser blocks and code must represent the same AST.
- Code parse errors must preserve the last valid editor AST.
- Native release builds must pass on Ubuntu and Windows.
- The web editor must remain usable without a mouse for normal block insertion/navigation/editing.

---

### Task 1: Shared model and Rust text parser

**Files:**
- Modify: `src/model.rs`, `src/lib.rs`, `Cargo.toml`
- Create: `src/parser.rs`
- Test: `tests/language_v2.rs`

**Interfaces:**
- Produces `Project::upgrade_from_json`, `parser::parse_project`, v2 `Expr`, `Command`, `Event`, `Script`, `Procedure`, `Asset` types.

- [ ] Write parser/model tests covering v1 upgrade, representative v2 source, expression precedence, events, procedures, lists, and invalid input.
- [ ] Confirm the new tests fail against the v1 model.
- [ ] Implement the v2 model, v1 upgrade path, tokenizer, Pratt expression parser, statement parser, and canonical formatter.
- [ ] Run the focused Rust tests and the existing test suite.

### Task 2: BLK2 compiler and cooperative VM

**Files:**
- Replace: `src/bytecode.rs`, `src/vm.rs`
- Test: `tests/runtime_v2.rs`, existing bytecode/runtime tests

**Interfaces:**
- Produces BLK2 `Program`, serializable `Instruction`, compile/decode functions, scheduler APIs for key transitions and runtime updates.

- [ ] Write failing tests for expression/state operations, repeat/while/if jumps, procedure calls, wait yielding, broadcast spawning, key events, list mutation, collision, and BLK1 decode compatibility.
- [ ] Implement instruction compilation with labels/jump patching and local/global name resolution metadata.
- [ ] Implement cooperative runtime threads with stack/call frames, step budget, event spawning, variables/lists, pen segments, and collision queries.
- [ ] Run all Rust tests.

### Task 3: Assets, packaging, and native runner

**Files:**
- Create: `src/package.rs`
- Modify: `src/bin/blockc.rs`, `src/bin/blockrun.rs`, `src/lib.rs`, `Cargo.toml`
- Test: `tests/package_v2.rs`

**Interfaces:**
- `package::build_package`, `package::read_package`; `blockc` accepts `.json`/`.bn`; `blockrun` accepts `.bcode`/`.bnp`.

- [ ] Write failing in-memory package tests for manifest, program bytes, image/sound assets, and unsafe package paths.
- [ ] Add ZIP packaging and source-relative asset loading.
- [ ] Update `blockc` source detection and output handling.
- [ ] Update `blockrun` texture/sound loading, costume rendering, pen rendering, keyboard forwarding, and audio playback.
- [ ] Run all Rust tests and release builds in CI.

### Task 4: Editor language and keyboard-first blocks

**Files:**
- Replace: `editor/language.mjs`, `editor/language.test.mjs`
- Modify: `editor/app.js`, `editor/index.html`, `editor/style.css`

**Interfaces:**
- JS parser/formatter matches Rust v2 AST JSON shape; editor manipulates that AST directly.

- [ ] Write failing Node tests for v2 grammar, round-trip formatting, expressions, v1 upgrade, stable IDs, and parse-error preservation helpers.
- [ ] Implement v2 parser/formatter and migration utilities.
- [ ] Expand block rendering/editing for events, variables, lists, control blocks, procedures, broadcast, pen, sound, and expression fields.
- [ ] Expand `Ctrl+Space` searchable insertion palette and structural keyboard navigation for nested `if`/`else`, loops, events, and procedures.
- [ ] Run Node syntax/tests and browser-independent state tests.

### Task 5: Documentation and final CI

**Files:**
- Modify: `README.md`, `.github/workflows/build.yml`
- Add/update examples under `examples/`

**Interfaces:**
- Documents `.bn`, JSON v2, `.bcode`, `.bnp`, editor keyboard shortcuts, compiler/runner commands.

- [ ] Add representative text and JSON v2 examples.
- [ ] Update CI to run Node tests, Rust tests, format checks, and Ubuntu/Windows release builds with artifacts.
- [ ] Run a fresh final workflow on the feature branch and verify both platform artifacts are uploaded.
- [ ] Open a PR against `main` with implementation and verification summary.