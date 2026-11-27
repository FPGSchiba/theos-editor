# P11 · Editor shell and Codex — phase spec

Status: waiting for Jann's approval · 27 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P11, as of umbrella version 8. Scope and acceptance
criteria come from there; this spec decides how.

## 1. Goal

After P11, Jann starts the desktop app, opens a content project and sees the editor as it will
be: the title bar, the toolbar, the rail, the windows and the status bar of
`docs/ui/screens/01-main.html`. The Codex works read-only. It shows the tree of the project, opens
records in tabs and shows each one as on screen 01: its header, its body with link chips and voice
blocks, its front matter, what links to it and its findings. Search jumps to a record. Shortcuts
and a command palette run the commands that exist.

Every other rail entry opens its layout with its windows in place. Each window is empty and says
what will appear there and which phase brings it. Jann's words: "I don't really want to do
anything beside viewing files and see the general layout of the editor. So I have a sense what
will come and where it's placed."

P11 is the first UI phase, so it also lays the ground every later screen stands on:
- the Tauri 2 app crate;
- the Svelte 5 UI in `editor-ui/`;
- IPC types and command wrappers generated from Rust;
- the editor-part registry for the rail;
- one keymap;
- the `ui` check of the gate (`docs/gate.md` §1), added by a gate PR inside this phase.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-32 Editor shell**
- CE-32.1 The app opens a project and shows the rail of `docs/architecture.md` §6.
- CE-32.2 Shortcuts (Ctrl on Windows and Linux, Cmd on macOS) and a command palette.
- CE-32.3 Builds with and without the `engine` feature (the feature is empty).
- CE-32.4 IPC types for the UI are generated from Rust; a mismatch fails the build.

**CE-33 Codex view**
- CE-33.1 A floor shows as on `docs/ui/screens/01-main.html`.
- CE-33.2 Backlinks are derived.
- CE-33.3 Search jumps to a record.

## 3. Scope

**Goals**
- `crates/theos-editor`: the Tauri 2 app (§5.2), its IPC commands (§5.4), the bindings test
  (§5.5) and the recorded IPC answers for the UI tests (§5.12).
- `editor-ui/`: the Svelte 5 shell, the Codex screen, the layouts of the unbuilt parts, the
  keymap and the palette (§5.6–§5.11).
- `theos-studio`: the editor-part registry for the rail (§5.3), and the Codex views that the
  screen needs beyond P48's (§5.8). P48's `codex` queries come from whichever of P11 and P48 is
  built first (§6.1).
- A gate PR inside this phase (§5.13): the `ui` job, the Linux packages Tauri builds against,
  the `engine` feature in `clippy` and `test`, and the advisory ignore that Tauri needs.

**Non-goals**
- Editing of any kind: Write and Source mode, saving, the front-matter editor, undo and redo,
  merging an external change into a dirty record (P12). P11 calls no command of P06.
- A file watcher (P12, for CE-34.2). P11 syncs when the project opens, when the window gets the
  focus back, and on the Reload command (§5.9).
- The Start screen, recent and pinned projects, Settings, the light theme switch (P14, and the
  light theme when Jann's new Theos UI is in `design/theos-ui/`).
- Git: the branch and change counts in the title and status bars (P46), the "edited 20 min ago ·
  Jann" line of the record header (P46's history).
- Pictures: images from the binaries folder (P16, CE-43.3).
- Lens, truth, script and graph views (P15); Atlas (P25); every other screen.
- The installer and a shippable bundle (P18). P11 builds and runs the app from the repo.
- The agent marker of the status bar ("claude · mcp · propose only", P07).

**Anti-goals** (P11 fails if any of these holds, even with every test green)
- The UI parses Markdown or front matter itself, or renders record text as HTML. The body comes
  from Rust as a typed tree, and the UI only draws it (§5.8).
- A hand-written IPC type or command name in the UI.
- A colour, font or size in a component that is not a Theos UI token (`design/theos-ui/`), so
  the light theme could not drop in.
- A shortcut defined anywhere but the one keymap.
- A control that looks active and does nothing. An unbuilt control is disabled and says which
  phase brings it.
- An open project that writes anything to the content project besides P03's cache.
- Network access of any kind (G12): the UI is served from the app, with a strict CSP.

## 4. Decisions

### 4.1 Decided by Jann (27 Nov 2026)

- **Read-only.** P11 is for looking: the layout of the whole editor and records in the Codex.
  Editing starts in P12.
- **Unbuilt parts show their layout.** Each rail entry whose phase is not built opens its windows
  in place, empty, each saying what will appear and which phase brings it.
- **No sample project.** tower-of-theos has no records yet, and an empty tree is fine. Jann adds
  records by hand or with `theos new` when he wants to see the tree. The tests use synthetic
  fixtures (§5.12).
- **IPC: tauri-specta** generates the command wrappers and the types (§5.5).
- **UI tests: Vitest for units, Playwright for whole-shell page tests**, with IPC answered from
  JSON that Rust tests record from the real commands (§5.12). Playwright's Svelte component
  testing package is deprecated (§4.2), so the gate PR rewords the `ui` row of `docs/gate.md`
  from "Playwright component tests" to "Playwright tests".
- **Linux packages for CI: cache the `.deb` files** with `actions/cache` and install them with
  `apt-get` (§5.13).
- **The rail shows the enabled parts.** These are the parts in `theos.toml` `[parts] enabled`, in
  the order of architecture §6, plus Code and Settings (§5.3).
- **Light theme later.** Jann has added a light mode to Theos UI and will hand it over once it
  is final. P11 takes every colour from tokens and sets the theme on the root element, so the
  light theme is a new token set and a switch, with no component change (§5.7).
- **P11's Read mode stays.** P11 builds Read mode as Svelte components over Rust's block tree.
  P12 adds Write and Source mode on CodeMirror (decision 0001), and its widgets mount the same
  Svelte components (link chip, voice block, callouts, beat row), so a chip looks the same in
  every mode. Decision 0001 says "Read mode … P12 builds it"; P11 builds it instead, and P12's
  spec records that (§6.3).

### 4.2 Probes (27 Nov 2026, toolchain 1.99.1, Windows 11; Linux in Docker `ubuntu:24.04`)

All probes ran in a scratch workspace with the repo's `rust-toolchain.toml`, `[workspace.lints]`,
`clippy.toml` and `deny.toml` copied verbatim. Versions are the newest at least two weeks old.

**Tauri 2** (`tauri` 2.12.5, `tauri-build` 2.7.4, both 21 Oct 2026):
- `#[tauri::command]`, `generate_handler!` and `generate_context!` raise no lint under the
  workspace lints. The one lint is `clippy::needless_pass_by_value`:
  - on a command's own argument struct, which disappears when the command hands it by value to
    a function that destructures it;
  - on `tauri::State` and `tauri::AppHandle`, which Tauri injects by value. No wrapper avoids
    it, so those commands carry
    `#[expect(clippy::needless_pass_by_value, reason = "Tauri injects State and AppHandle by value")]`.
- `tauri-build` declares the cfgs `desktop`, `mobile` and `dev` through `rustc-check-cfg`, so
  `unexpected_cfgs = "forbid"` is satisfied. Our code uses none of them (`gate/cfg-names.toml`).
- Without `frontendDist` on disk, `generate_context!` panics, unless `devUrl` is also set and
  the `custom-protocol` feature is off. Then `cargo build`, clippy and the tests pass with no web
  UI built, and the binary loads `devUrl`. So the Rust jobs never need Node.
- Needed at the crate root: `tauri.conf.json`, `build.rs` and `icons/icon.ico` (Windows). Also
  `icons/icon.png`, the window icon on Linux. `capabilities/` is not needed to build. The build
  script writes `gen/schemas/*.json` into the crate folder.
- **Testing commands without a window.** `tauri::test` (feature `test`: `mock_builder`,
  `mock_context`, `noop_assets`, `get_ipc_response`) invokes a command end to end through IPC:
  typed success, typed errors, rejected arguments, `State`, async commands. It runs in
  0.01–0.03 s per test on Windows and on Linux without a display.
- **A Windows test harness needs the app manifest.** Without it the test exe exits with
  `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139), because `tauri-build` links the Common Controls v6
  manifest into bins only. Fix, in `build.rs`, no new crate:
  - `WindowsAttributes::new_without_app_manifest()`;
  - `cargo:rustc-link-arg=/MANIFEST:EMBED` and `/MANIFESTINPUT:<crate>/windows-app-manifest.xml`
    for every target. The manifest is a copy of `tauri-build`'s.
- Clean dev build 49 s on Windows (63 s with the repo's `opt-level = 1` for dependencies);
  rebuild after touching a command file 2–3 s; Linux `cargo check` 24 s, `cargo test` 32 s.

**tauri-specta** (2.0.0-rc.29, 13 Nov 2026; `specta` =2.0.0-rc.28, `specta-typescript` 0.0.15;
all MIT):
- It compiles with Tauri 2.12.5 on 1.99.1, and clippy is clean.
- It exports `commands.<name>(…)` wrappers and the types to one `.ts` file. The file has LF line
  ends and the same bytes run after run. It keeps doc comments and honours serde's `rename_all`,
  `tag` and `content`.
- `ErrorHandlingMode::Result` types each command's error. The default mode does not.
- It refuses `u64`, `i64`, `usize` and `isize` ("Specta forbids exporting BigInt-style types").
- A test that exports to a temporary file and compares it with the checked-in file fails on a
  stale file. With `UPDATE_BINDINGS=1` it writes the file instead.
- It is a release candidate and asks for `=` pins. ts-rs 12.0.1 is stable but types only: it has
  no command wrappers, so a renamed command or argument would go unnoticed.

**`cargo deny`** (0.20.2, the repo's `deny.toml`):
- Licences, bans and sources pass, Tauri's Linux tree included.
- Two advisories fail, both "unmaintained":
  - `RUSTSEC-2024-0370` (`proc-macro-error` 1.0.4, via `glib-macros` 0.18 ← GTK ← `tao` and
    `muda` ← `tauri`, Linux only);
  - `RUSTSEC-2024-0436` (`paste` 1.0.15, via `specta`).
- About 370 packages are new to `Cargo.lock`, and about 60 more crates appear in two versions
  (warnings only).

**Linux** (Docker `ubuntu:24.04`):
- `cargo check` fails at `libdbus-sys` without system packages.
- `build-essential`, `pkg-config` and `libwebkit2gtk-4.1-dev` with `--no-install-recommends`
  are enough for check, clippy and test. They are 352 packages, a 168 MB download and 51 s to
  install.
- The GitHub `ubuntu-24.04` image (20261116) has none of the WebKit, GTK or soup packages.

**Svelte 5 page and component tests** (Windows, Chromium):
- `@playwright/experimental-ct-svelte` 1.58.2 mounts a runes component. But:
  - it has had no release since February 2026;
  - it pins Playwright 1.58, Vite 6 and an old Chromium;
  - Playwright issue #42139 (closed 31 Oct 2026) says the built-in `mount` and a gallery replace
    the experimental packages, whose React and Vue variants are no longer published.
- Playwright 1.65.1's built-in `mount` works with a hand-written Svelte gallery, but its API is
  "not final yet".
- Vitest 5.1.0 browser mode with `vitest-browser-svelte` 3.1.0 works too (its `render` is async).
- `mockIPC` from `@tauri-apps/api/mocks` 2.12.4 answers `invoke` in all three.
- `svelte-check` 4.7.8 does not run on TypeScript 7 alone: its peer range is TS 5 or 6. With
  TypeScript 6.0.3 it passes.

### 4.3 Approaches considered

| Question | Chosen | Considered |
|---|---|---|
| Scope | Read-only shell and Codex (Jann) | front-matter editing with undo in P11 (P06 §6.2 offered `run`, `undo`, `redo`); it belongs with CE-34.3's field undo in P12 |
| IPC generation | tauri-specta (Jann) | ts-rs (types only, command names hand-written); our own generator over ts-rs (custom code where a crate exists) |
| UI tests | Vitest + Playwright page tests on recorded IPC (Jann) | Playwright's built-in `mount` (API not final); Vitest browser mode (component level only); experimental CT (deprecated); driving the real app with `tauri-driver` (no macOS, a full app build in the `ui` job) |
| Linux packages | cached `.deb` files (Jann) | `cache-apt-pkgs-action` (third-party, skips postinst, v2 under two weeks old); our own container image (one more thing to build and keep secure) |
| Rail | enabled parts + Code + Settings (Jann) | all ten entries of architecture §6 whatever `theos.toml` says |
| Body | a typed block tree built in Rust from P02's parser, links resolved by the index | Markdown to HTML in the UI (`markdown-it`): a second parser beside P02's, an injection surface, and chips without titles |
| Where the IPC types live | `theos-editor`, mapped from studio views | specta derives on the studio views (an RC crate in a library that `theos` and `theos-mcp` also build) |
| UI app | plain Vite + Svelte 5 | SvelteKit with the static adapter (routing and SSR the shell does not use) |
| Opening a project | an argument, an "Open project…" folder dialog, and the last project | a path typed into the palette; the Start screen (P14) |
| Freshness | sync on open, on focus and on Reload | a file watcher now (P12 needs it for CE-34.2 and builds it there) |
| `engine` feature | `clippy` and `test` also build `theos-editor` with `--features engine` | a slow test that runs `cargo check --features engine` (a second build of the Tauri tree) |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Where | Crate or tool | Our code, and why |
|---|---|---|---|
| App, windows, IPC | `crates/theos-editor` | `tauri` =2.12.5, `tauri-build` =2.7.4 | Setup, commands as thin maps onto studio calls (§5.4) |
| Folder dialog | `theos-editor` | `tauri-plugin-dialog` (official, newest 2.x at least two weeks old) | none |
| IPC types and wrappers | `theos-editor` | `tauri-specta` =2.0.0-rc.29, `specta` =2.0.0-rc.28, `specta-typescript` =0.0.15 | The stale test (§5.5) |
| Command tests | `theos-editor/tests` | `tauri::test` (feature `test`, dev-dependency) | Fixture setup, the recorder (§5.12) |
| Part registry | `theos_studio::parts` | – | One module per part (§5.3) |
| Codex queries | `theos_studio::codex` | P03's index; P48's shapes (§6.1) | Whichever of P11 and P48 builds first |
| Codex views | `theos_studio::codex_view` (indicative) | P02's `body` (pulldown-cmark events, callouts, links), P02's front-matter model, P03's index | The block tree and the front-matter rows (§5.8) |
| Search syntax | `theos_studio::codex_view::query` | P03's `search`, records by id and kind | The `#id @people :kind` prefixes (§5.10) |
| Last project | `theos-editor` | `dirs` (workspace), `serde_json` | A small state file (§5.9) |
| UI | `editor-ui/` | Svelte 5.57.3, Vite 8.3.8, TypeScript 6.0.3, pnpm 12.23.0, Node 24 | Components per `tu-*` class (§5.7) |
| Styles and fonts | `editor-ui/` imports `design/theos-ui/` | `tokens.css`, `components/bundle.css`, `fonts/` | none: imported from their one place |
| IPC in the UI | `editor-ui/src/lib/ipc/bindings.ts` | generated; `@tauri-apps/api` 2.12.4 | none |
| Keymap | `editor-ui/src/lib/keymap.ts` | `tinykeys` if its `$mod` can take our platform, else our ~30 lines (§5.11) | One table |
| Palette matching | `editor-ui` | `fuzzysort` (MIT) | none |
| Unit tests | `editor-ui` | Vitest 5.1.0 | – |
| Page tests | `editor-ui/e2e` | `@playwright/test` 1.65.1, `mockIPC` | The replay of recorded IPC (§5.12) |
| Static checks | `editor-ui` | `svelte-check` 4.7.8, ESLint 10 with `typescript-eslint` and `eslint-plugin-svelte` | – |

New Rust dependencies, pinned in `[workspace.dependencies]`: `tauri`, `tauri-build`,
`tauri-plugin-dialog`, `tauri-specta`, `specta`, `specta-typescript`. The Tauri and specta
crates take `=` pins, because tauri-specta asks for it and the bindings must not change under a
patch release.

### 5.2 The app crate

`crates/theos-editor` becomes a library with a thin binary:
- `src/lib.rs`: `pub fn builder<R: Runtime>(tauri::Builder<R>) -> tauri::Builder<R>` registers
  the state, the plugin and the invoke handler of tauri-specta. Tests build the same app on
  `MockRuntime` (§4.2).
- `src/main.rs`: `builder(tauri::Builder::default()).run(generate_context!())` and the `tracing`
  subscriber. Nothing else, so coverage measures the library.
- `src/ipc/`: one module per group of commands (§5.4), and `src/ipc/error.rs` (§5.4).
- `build.rs`: `tauri-build` with the Windows manifest fix of §4.2.
- `tauri.conf.json`:
  - `devUrl` `http://localhost:5179` and `frontendDist` `../../editor-ui/dist`;
  - one window `main`, 1440 × 900 by default, the title "Theos Editor";
  - `app.security.csp`: `default-src 'self'`, with `style-src 'self' 'unsafe-inline'` only if
    Svelte's style output needs it, and no remote source;
  - no updater and no bundle settings (P18).
- `capabilities/main.json`: the window `main` gets the app's own commands and
  `dialog:allow-open`, nothing else. There is no `fs`, `shell` or `http` plugin.
- `icons/`: `icon.ico` and `icon.png`, a placeholder built from the Theos mark of the title bar
  (`tu-mark`). P18 brings the real icon set.
- `gen/` is ignored by git (`crates/theos-editor/.gitignore`).
- Running it: `pnpm --dir editor-ui dev` and `cargo run -p theos-editor -- <project>`, in two
  terminals. A one-line `pnpm --dir editor-ui app` script starts both; the crate README says
  how. No Tauri CLI is needed for P11; P18 adds it for the bundle.
- **`engine` feature.** It stays `engine = []` (CE-32.3). One `#[cfg(feature = "engine")]`
  item exists, so that both builds compile something different: `parts::engine_presets()`,
  which returns an empty list, beside `parts::content_presets()`. The engine part will fill it
  (architecture §14).

### 5.3 The editor-part registry and the rail (CE-32.1)

P06 ruling 12 left the editor-part registry to the first phase that registers per part beyond
kinds and rules. That is P11's rail.

- `theos_studio::parts`:
  - `Part { id, rail: Option<RailEntry> }`;
  - `RailEntry { label, glyph, order, phase }`, where `phase` is the phase that builds the part's
    screen, or `None` once it is built;
  - `parts::content() -> Registry` lists one module per part: `codex`, `atlas`, `graph`,
    `scripts`, `concepts`, `maps`, `review`, `changes`, `mcp` (no rail entry). It also has two
    entries that are not parts of `theos.toml`: `code` and `settings`.
- **A new part is one module and one line.** The kinds stay in `theos_content::parts` (P03). The
  two registries meet when the engine part brings both.
- **The rail of a project:**
  - the parts in `[parts] enabled` that have a rail entry, in architecture §6's order (Codex,
    Atlas, Graph, Scripts, Concepts, Maps, Review, Changes);
  - then Code, and Settings at the bottom (screen 01's spacer).
  - An unknown name in `enabled` is skipped with a `warn` log line. P03 reads `enabled` and does
    not judge it.
  - tower-of-theos enables no `atlas` today, so its rail matches screens 01–11. Atlas appears
    when the project enables it, which matches `docs/ui/README.md` ("P25 adds the entry").
- **Glyphs and labels** come from screen 01: Codex ◈, Graph ⋈, Scripts ¶, Concepts ◫, Maps △,
  Review ◴, Changes ±, Code `</>`, Settings ⚙. Atlas takes screen 12's glyph.
- **Phases** of the unbuilt entries: Atlas P25, Graph P15, Scripts P15, Concepts P16, Maps P16,
  Review P13, Changes P13, Code P18, Settings P14. Codex is built (`None`).
- **Badges** (Review 3, Changes 2 on screen 01) come with the phases that count them. P11 draws
  the badge slot and sends none.

### 5.4 IPC commands

Each command is a thin map: it takes the `Workspace` from the app state, calls `theos-studio`
and returns a view type of `theos-editor`. The view types derive `serde::Serialize` and
`specta::Type` and are built from the studio's views (§6.1). Numbers are `u32`, or strings where
they could exceed it (§4.2).

| Command | Returns | Studio call |
|---|---|---|
| `app_info()` | version, OS (`windows`, `macos`, `linux`), whether `engine` is on | – |
| `project_open(path)` | `ProjectView`: name, root, kind, rail, record count, error and warning counts | `Workspace::open` (P05) and `parts` |
| `project_last()` | the last project's path, or `null` | the state file (§5.9) |
| `project_sync()` | `ProjectView`, and the ids of records whose views changed | `index.sync` |
| `codex_tree(tab)` | `TreeNode`s of `where`, `who`, `lore` or `things` | `codex_view::tree` (§5.8) |
| `record_view(id)` | `RecordView` (§5.8) | `codex::read` (P48) and `codex_view::record` |
| `codex_search(query, limit)` | hits (§5.10) | `codex_view::query` over `codex::search` |

- One `Workspace` is open at a time, held in `Mutex<Option<Workspace>>` app state. A command
  without an open project fails with `NoProject`.
- **Errors.** `IpcError` (`thiserror`, serialised as `{ kind, message }` with serde's `tag` and
  `content`) has the variants:
  - `NoProject`;
  - `NotAProject { path, message }` (P03's messages, which name the file and what to do);
  - `NotFound { id }`;
  - `Index { message }`;
  - `Io { path, message }`.

  tauri-specta's `ErrorHandlingMode::Result` gives the UI a typed result for every command. The
  UI shows the message, never a stack.
- **Logging:** `info` for each opened project and sync, with counts; `debug` for each command
  and its time; `warn` for skipped `enabled` names; `error` for every `IpcError` returned.

### 5.5 IPC types generated from Rust (CE-32.4)

- `theos-editor` exports the bindings to `editor-ui/src/lib/ipc/bindings.ts` through
  tauri-specta's `Builder::export`, with `ErrorHandlingMode::Result`. The file is checked in.
- **The test `bindings_are_current`** exports to a temporary file and compares it byte for byte
  with the checked-in file. A difference fails with "editor-ui/src/lib/ipc/bindings.ts is stale:
  run `UPDATE_BINDINGS=1 cargo test -p theos-editor bindings`". With `UPDATE_BINDINGS=1` it writes
  the file and passes. It runs in `test`, so a changed Rust type with an old file fails the
  build.
- **The other direction.** The UI calls only `commands.*` of the bindings and imports types only
  from there. A changed type therefore fails `svelte-check` in `ui` wherever the UI still uses the
  old shape. An ESLint rule (`no-restricted-imports`) forbids `invoke` from `@tauri-apps/api/core`
  outside `src/lib/ipc/`, so no command can be called by a hand-written name.
- **Formatting.** The bindings file is excluded from ESLint's stylistic rules and from any
  formatter, so its bytes stay tauri-specta's.

### 5.6 The UI app

`editor-ui/` is a pnpm package (`packageManager` pinned, `engines.node` `>=24`), with Vite and
Svelte 5 in runes mode, TypeScript strict and no SvelteKit.

```text
editor-ui/
  src/main.ts             mounts the shell, imports design/theos-ui (§5.7)
  src/lib/ipc/            bindings.ts (generated) · client.ts (the one place that awaits results)
  src/lib/keymap.ts       the one keymap (§5.11)
  src/lib/commands.ts     the palette's command table (§5.11)
  src/lib/state/          project, open tabs, selection: Svelte 5 runes in .svelte.ts modules
  src/shell/              TitleBar, Toolbar, Rail, StatusBar, Palette, Dock
  src/parts/codex/        CodexTree, RecordWindow, RecordHeader, Body, FrontMatter, UsedBy, Validation
  src/parts/placeholder/  the layouts of the unbuilt parts (§5.9)
  src/components/         one Svelte component per design-system component (§5.7)
  e2e/                    Playwright page tests and their recorded IPC (§5.12)
```

- **Layouts.** A rail preset replaces the whole dock tree (architecture §6). A preset is data:
  `{ id, windows: Split }`, where a `Split` nests rows and columns of windows with flex sizes. A
  window has an id, its tabs and a component. `Dock` draws any such tree. A new preset is one
  file and one line in `src/parts/presets.ts`. Splits are not resizable in P11.
- **No routing.** The active preset is state, not a URL.

### 5.7 Theos UI in the app

- `main.ts` imports `design/theos-ui/tokens.css` and `design/theos-ui/components/bundle.css`
  from their place in the repo. The fonts load from `design/theos-ui/fonts/` through
  `tokens.css`. Nothing is copied. Vite serves and bundles them, so the app ships them and the
  CSP allows them as `'self'`.
- **Components follow the design system.** Each `design/theos-ui/components/<Name>` maps to one
  Svelte component (README "Consuming"), and screen 01's classes are the contract: `tu-shell`,
  `tu-titlebar`, `tu-toolbar`, `tu-rail`, `tu-window`, `tu-tabbar`, `tu-tree`, `tu-rechead`,
  `tu-prose`, `tu-link`, `tu-voice`, `tu-fm`, `tu-prop`, `tu-val`, `tu-statusbar`, `tu-kind`
  with `data-kind`, and the `is-*` states. Page styles of screen 01 that are not in the bundle
  (the 248 px tree, the 320 px side column, the lore table's grid) live in the component that
  needs them and use tokens only.
- **The theme.** The root element carries `data-theme` with the design system's own theme name,
  today `grid-noir` (`tokens.css` scopes its tokens to `:root, [data-theme="grid-noir"]`). The
  light theme's name comes with Jann's Theos UI change, and the switch (P14) sets that name. A
  component never names a colour,
  font or size: only `var(--…)` tokens and `tu-*` classes. ESLint and `svelte-check` cannot see
  CSS, so a Vitest test reads every `.svelte` file's `<style>` and fails on a hex, `rgb(`,
  `hsl(` or named colour, and on a `font-family` that is not a token.
- **Register.** Chrome is Instrument; the record body is `tu-manuscript`; the front matter,
  validation and the record's top line stay Instrument (Theos UI README "The two registers").

### 5.8 The Codex screen (CE-33.1, CE-33.2)

The Codex preset is screen 01's three columns: the tree (248 px), the record window (flex), and
the side column (320 px) with Front matter / Links in, Used by and Validation.

**Tree** (`codex_view::tree(tab)`):

| Tab | Roots and children |
|---|---|
| Where | `tower` → `band` → `floor` → `location` and `spawn`, by `parent`; children ordered by `number`, then title |
| Who | `people` → `faction` → `npc`, by `parent` |
| Lore | groups Topics, Truths, Beliefs, Clues and Equations, each sorted by id |
| Things | groups Things and Enemies |

- The table names the kinds of each tab; a kind that the registry does not know yet is left
  out, with no finding and no empty group. `spawn` and `enemy` are not registered until P21
  (CE-57), so today the Where tab ends at locations and Things has only Things. When P21
  registers them, they appear with no change to P11's code.
- A row has the kind glyph, the title (the id for lore kinds, as `T-012 title`), and a tag: a
  band's floor range (`1–20`) or a record's `status`.
- A record whose `parent` is missing or dangling sits at the top level of its tab, and its
  `link/dangling` finding shows when it is opened.
- Kinds of other parts (scripts, quests, concepts, maps, styles) are not in the Codex tree.
  Search still finds them, and they open in the Codex record window, the generic view of any
  record, until their own screens exist.
- The footer of the Where tab keeps screen 01's three lines ("Where: one parent per place" …).
- An empty tab says what will appear and how to add a record ("Records whose kind is tower,
  band, floor, location or spawn appear here. Add one with `theos new`, or as a file under
  `world/`.").

**Record window.** Records open in tabs (kind glyph and id). Clicking a tree row, a link chip, a
"Used by" row or a search hit opens the record, or focuses its tab. `RecordView` holds:

- **Header** (`tu-rechead`):
  - the top line: glyph, id, "`kind` · in `parent title`" (the parent from `parent`, if any), and
    the status chip, `draft` outlined and `canon` green;
  - the title;
  - the meta line: the path, then "N links in", then "N children" where there are any. "edited
    … · author" comes with P46.
- **Body: a block tree from Rust.** `codex_view::record` walks P02's body events and grammar
  into blocks:
  - `Heading { level, inlines }`, `Paragraph { inlines }`, `List { ordered, items }`,
    `Quote { blocks }`, `Code { text }` and `Rule`;
  - `Voice { speaker, lens, body, by }`, from P02's voice callout; `Design { blocks }` and
    `Note { … }`, from P02's other callouts;
  - `Beat { heading, params, blocks }`, from P02's beat grammar, so scripts show readable
    attribute rows until P15;
  - `Raw { text }`, for anything P02 reports as malformed. It is shown as plain text, with the
    finding in Validation.

  Inlines are `Text`, `Emphasis`, `Strong`, `Code`, `Break` and
  `Link { id, fragment, shown, target }`. `target` is the resolved record (kind, title, status,
  and P48's marks) or `null` for a dangling link, which draws as a red chip (`is-reject` with ✕).
  A chip shows the target's title, and for lore kinds also its id (screen 01's `C-031 the
  murmur`). Nothing is HTML: the UI draws each block with its component. External Markdown links
  and images draw as text with their URL, never fetched (G12).
- **Lore this record carries** (screen 01's table, under the body): the records of the lore
  kinds (truth, belief, clue) that link to this one, from its backlinks. Each row shows the glyph,
  the id, the title, "Touches" (the row record's `reveals`, `distorts` or `topic` target with its
  verb: "hints ■ T-012", "distorts ■ T-001", and "· never" for a `never` truth, from P48's
  marks), and the tier where the record has one. The table appears only when there is a row.
- **Pictures** (screen 01's grid): the concepts whose `for` names the record, as captions
  "◫ concept · picked vN" or "◫ concept · no pick", with an empty frame that says "Images arrive
  with P16". The section appears only when there is a concept.

**Side column:**
- **Front matter:** the fields in file order, with key and value:
  - a link is a chip, dangling ones red;
  - a list shows its items;
  - a `tier` value draws the tier selector (`tu-tier`), read-only;
  - a lens entry shows its key and value.

  The footer shows "+ field", disabled ("Editing arrives with P12"), and "schema · `kind` v1".
  The rows come from P02's front-matter model, so the order and the spelling are the file's.
- **Links in · N** (second tab): every backlink, one row each, with the source's glyph, id and
  title, and the slot.
- **Used by:** the backlinks grouped by source, each with its relation. The relation is taken
  from P48's `slot` (§6.1): `field:<key>` gives the key with `_` as a space ("found at",
  "where", "about", "parent" as "child of"); `beat:<key>` gives "in a beat"; `voice:*` gives
  "voice"; `lens:<key>` gives "lens"; `note:*` gives "note"; `mention` gives "mentions". A
  source with several links shows each relation once. **CE-33.2:** nothing here is stored on the
  record; it is the index's backlinks at the time of the view.
- **Validation:** the record's findings, errors first, each with its severity glyph, message and
  rule (`tu-val`). With none: "No findings on this record." Screen 01's pass rows ("All 12 links
  resolve.") need per-rule pass messages, which no rule has. P11 shows findings only.

**Title bar, toolbar, status bar** (screen 01):
- **Title bar:** `THEOS//EDITOR` ▸ project name ▸ the open record's path without `.md` ▸
  "layout · `preset`". On the right, the validator pip: "validator · N errors" (red), "· N warn"
  (yellow) or "validator · clean" (green). The git pip comes with P46.
- **Toolbar:**
  - the find field (§5.10);
  - Read / Write / Source, with Read active and the others disabled ("arrives with P12");
  - "Open in graph", disabled (P15);
  - "New record", disabled (P12);
  - "Save Ctrl+S", disabled (P12). The label shows the platform's key, from the keymap.
- **Status bar:** `THEOS//EDITOR`, the version and "`preset` layout", "N records · N errors ·
  N warnings". The other slots of screen 01 come with their phases.

### 5.9 Opening, freshness, unbuilt parts

- **Opening a project:**
  - `theos-editor <path>` opens that folder;
  - without an argument, the last project, if `project_last` names one that still opens;
  - otherwise an empty shell, whose work area says "Open a content project: Ctrl+O" (Cmd on
    macOS).

  "Open project…" (Ctrl+O, and in the palette) shows the OS folder dialog. A folder without
  `theos.toml` shows P03's message in the work area and keeps the open project.
- **The last project** is a path in `<data dir>/theos/editor-state.json` (`dirs::data_dir()`).
  It is app state, not a setting: P14's settings layers stay untouched, and P14's recent
  projects may take it over.
- **Freshness.** The app syncs the index:
  - when a project opens;
  - when the window gets the focus back (`tauri://focus`), at most once a second;
  - on Reload (Ctrl+R).

  The open views reload when their records changed. A record deleted outside the app closes its
  tab and says so in the status bar. P12 replaces the focus sync with a watcher.
- **A cache held by another process** (`theos check`, `theos-mcp`): P05's `Workspace` falls back
  to an index in memory. The status bar shows "index in memory: another program holds the
  cache".
- **Unbuilt parts.** Each rail entry with a `phase` opens its preset from
  `src/parts/placeholder/`. The preset has the windows of its reference screen, with their tab
  titles, in place and empty. Each body says what will appear and which phase brings it:

  | Entry | Reference | Windows |
  |---|---|---|
  | Atlas | `12-atlas` | floor map, layers, place panel — P25 |
  | Graph | `05-graph` | graph, filters, record — P15 |
  | Scripts | `04-script` | scripts, beats, cast and ledger — P15 |
  | Concepts | `06-concepts` | concepts, versions and pipeline, notes — P16 |
  | Maps | `11-maps` | runs and seeds, results, notes — P16 |
  | Review | `07-review` | sessions, proposal, findings — P13 |
  | Changes | `08-git` | changes by record, diff, history — P13 |
  | Code | – | file tree, code view — P18 |
  | Settings | `10-settings` | pages, values with their layers — P14 |

  The builder reads each reference screen for the windows, the tab titles and the wording, and
  writes each empty state in the style of Theos UI's "Empty states": what will appear and what
  to do. Example: "Agent proposals land here. Nothing changes the world until you approve it.
  Arrives with P13."

### 5.10 Search (CE-33.3)

The find field of the toolbar (Ctrl+P, or Cmd+P) shows results under it as you type, with a
debounce of 120 ms.

- **Query syntax**, parsed in `codex_view::query`:
  - `#<text>`: records whose id starts with the text, ignoring case (`#f04`, `#T-01`);
  - `:<kind>`: only records of that kind (`:floor`);
  - `@<text>`: only records of the kinds of the Who tab (people, faction, npc), then like a
    plain word;
  - every other word goes to P03's `search`, as P03 reads it (quoted phrases included).

  Prefixes combine: `:clue murmur` is the clues that match "murmur". A query with only `#` or `:`
  filters lists the matching records by id. An unknown kind after `:` gives no hits and the
  line "no kind `x`".
- **A hit** shows the glyph, the id, the title and the first match's line, with the match marked
  (P48's `match` and `text`). At most 20 hits; "more…" when P48 says `truncated`.
- **Arrows** move the selection, and **Enter** or a click opens the record: a tab, the tree's tab
  switched to the record's tab with its ancestors expanded and the row selected, and the record
  scrolled into view. **Escape** closes the results.

### 5.11 Shortcuts and the palette (CE-32.2)

- **One keymap**, `editor-ui/src/lib/keymap.ts`: a table of `{ command, keys }`, where `keys`
  uses `Mod` for Ctrl on Windows and Linux and Cmd on macOS (G13). The platform comes from
  `app_info().os`, not from the browser, so the tests can set it.
  - The table is the only place a key combination is written. The palette, the menu labels and
    the "Save Ctrl+S" text all read it.
  - A Vitest test fails on two commands with the same keys on one platform.
  - It uses `tinykeys` if its `$mod` can take our platform value; otherwise a small matcher of
    our own (about 30 lines).
  - P12's CodeMirror keymap will read the same table.
- **Commands** (`commands.ts`): `{ id, label, run, enabled, phase? }`. Those of P11:

  | Command | Keys |
  |---|---|
  | Command palette | Mod+K, Mod+Shift+P |
  | Find record | Mod+P |
  | Open project… | Mod+O |
  | Reload project | Mod+R |
  | Rail entry 1–9 | Mod+1 … Mod+9 |
  | Next / previous tab | Ctrl+Tab / Ctrl+Shift+Tab (Ctrl on every OS, as in browsers) |
  | Close tab | Mod+W |
  | Tree tab Where / Who / Lore / Things | Mod+Shift+1 … 4 |
  | Save | Mod+S (disabled: P12) |

  Mod+R and Mod+W replace the webview's reload and close. Mod+P replaces print, which the app
  does not offer.
- **The palette** (`tu-` popover, Instrument) lists every command with its label and keys.
  `fuzzysort` filters it as you type. Disabled commands stay listed, dimmed, with "arrives with
  PNN", and Enter does nothing on them. Arrows move, Enter runs, and Escape closes and gives the
  focus back.

### 5.12 Tests, fixtures and recorded IPC

- **Fixture** `crates/theos-editor/tests/fixtures/codex/` is a small synthetic project, marked
  synthetic in its README. It is built from the data model's examples (§14) with no new lore:
  - a tower, a band with `floors: [1, 20]`, a floor with two locations (no spawn: the kind
    comes with P21);
  - people → faction → npc with a lens;
  - a truth with target `never` and one without;
  - two clues, one revealing the truth with `where` the floor, and a belief that distorts the
    truth;
  - a script whose beat is `at` the floor;
  - a concept `for` a location, with a picked version;
  - a voice block, a design callout, a dangling link and a malformed construct in the floor's
    body.

  `theos-editor` also opens it through `Workspace` in the Rust tests.
- **Rust tests** (`crates/theos-editor/tests/`) build the app with `builder(mock_builder())` and
  call every command through `get_ipc_response`.
- **Recorded IPC** (`ipc_recordings_are_current`): one Rust test runs a fixed list of calls on a
  copy of the fixture: `app_info` once per OS value, `project_open`, `codex_tree` for each tab,
  `record_view` for every record, and `codex_search` for each query the page tests type. It
  writes each call and its answer to `editor-ui/e2e/ipc/<name>.json` with `serde_json`'s pretty
  printer, and fails when a checked-in file differs. `UPDATE_BINDINGS=1` updates the files, like
  the bindings.
- **Page tests** (`editor-ui/e2e/*.spec.ts`) run the Vite preview of the built UI in Chromium.
  `mockIPC` answers every command from the recordings and fails the test on a call that has no
  recording. So the pages are tested against real Rust answers, and a changed answer fails
  `test` until the recordings are updated, and then `ui` if the UI does not handle it.
- **Vitest** covers the keymap, the command table, the query echo in the find field, the slot to
  relation mapping and the style test of §5.7.
- **Evidence for Jann.** The build PR attaches a screenshot of the fixture's floor next to
  `docs/ui/screens/01-main.png`, and one of an unbuilt part's layout.

### 5.13 The gate PR (inside P11)

The `ui` check and Tauri's build needs are protected changes, so they go in their own PR,
`ci(gate): the ui check and the editor's build needs (gate)`, on `gate/P11-ui`. It merges
before the build PR, which cannot pass without it: the Tauri crates fail `deny`, and the Linux
jobs fail without the packages. It contains:

- **`ui` in xtask:** one module `xtask/src/checks/ui.rs` and one line where checks are
  registered. It runs, in `editor-ui/`:
  - `pnpm install --frozen-lockfile`;
  - `pnpm exec svelte-check --fail-on-warnings`;
  - `pnpm exec eslint .`;
  - `pnpm exec vitest run`;
  - `pnpm exec playwright test`.

  Each step's failure is a finding, and every step runs. While `editor-ui/package.json` does not
  exist, `ui` reports "not needed: no editor-ui yet", so the gate PR itself stays green.
  `cargo xtask tools` also checks `node` (major 24) and `pnpm` (the `packageManager` pin).
- **The job `ui`** in `ci.yml`. It runs beside the other three and uses the cached xtask of
  `.github/actions/xtask`, Node from `actions/setup-node` with the pnpm store cached by
  `pnpm-lock.yaml`, and Playwright's Chromium cached by its version. It runs
  `cargo xtask gate --only ui`. It needs no Rust build: the page tests use the recordings.
  `gate/ruleset-main.json` adds `ui` to the required checks. Jann switches the live ruleset with
  `cargo xtask github apply` when he merges.
- **`platforms`:** the `windows` and `macos` jobs add `ui` to their `--only` list (gate.md §1
  already says "from P11"), so the page tests run in Chromium on both.
- **Linux packages:** the setup action gets an input `gui-packages`. The `gate`, `test` and
  `coverage` jobs set it, and the step does three things:
  - restore `actions/cache` of the `.deb` files, keyed by the image version and the package list;
  - `apt-get install --no-install-recommends build-essential pkg-config libwebkit2gtk-4.1-dev`;
  - save the cache on `main`.

  The list lives in one file, `gate/linux-packages.txt`, which the workflow and the docs read.
- **`engine` (CE-32.3):** `clippy` also runs
  `cargo clippy -p theos-editor --all-targets --features engine --locked -- -D warnings`, and
  `test` also runs `cargo nextest run -p theos-editor --features engine --locked`. Only
  `theos-editor` recompiles; its dependencies are the same.
- **`deny.toml`:** ignores for `RUSTSEC-2024-0370` ("unmaintained, not vulnerable; a proc macro
  of glib 0.18 that Tauri's GTK 3 stack pulls in on Linux; leaves with Tauri's move off GTK 3")
  and `RUSTSEC-2024-0436` ("unmaintained, not vulnerable; a proc macro that specta uses at build
  time"). The licence list is unchanged.
- **`docs/gate.md` §1:**
  - the `ui` row: "Playwright tests" instead of "Playwright component tests";
  - `ui` among the required checks;
  - the Linux packages;
  - a row for `ui` in the table of what a non-code change cannot alter: `ui` reads `editor-ui/**`,
    the bindings and the recordings, all code.
- **`gate/code-paths.toml` stays as it is.** `editor-ui/**` is not in the non-code list, so it is
  code. `*.ts` is code everywhere already.
- **Tests in `xtask/tests/`:** the `ui` module (not needed without `package.json`, a failing
  step is a finding, every step runs), the workflow tests (the `ui` job, the packages input,
  `ui` in `platforms`), and gate.md's tables in step with the checks.

Raising the coverage floor of `theos-editor` is not part of it: the binary floor of 60 % applies
as it is.

### 5.14 How each AC is proven

Rust tests carry `// AC: <id>` above `#[test]`; TypeScript tests carry the same comment above
`test(…)` (gate.md §4).

| AC | Proven by |
|---|---|
| CE-32.1 | **Rust:** `project_open` on the fixture returns the rail of its `enabled` parts in architecture §6's order, then Code and Settings, with labels, glyphs and phases. A project that also enables `atlas` gets Atlas after Codex. A name not known is skipped. A folder without `theos.toml` returns `NotAProject` with P03's message. **Page:** the shell opens the recorded project and shows the rail in that order with Codex active. Each unbuilt entry opens its layout, with every window's empty state naming its phase. With no project, the work area offers "Open project". |
| CE-32.2 | **Vitest:** the keymap resolves `Mod` to Control on `windows` and `linux` and to Meta on `macos`; no two commands share keys on any platform; every command of the palette that has keys appears in the keymap. **Page:** with `os = windows`, Ctrl+K opens the palette, typing "tree lore" narrows it, Enter runs it, and Escape gives the focus back; Ctrl+P focuses the find field; Ctrl+2 switches the rail; Ctrl+W closes a tab. With `os = macos` (a second recording of `app_info`), Meta+K opens the palette and Ctrl+K does not. A disabled command shows its phase and does nothing. |
| CE-32.3 | **Rust,** in `theos-editor`, compiled and run in both builds (§5.13): the app builds on `MockRuntime` and lists its presets; `app_info().engine` equals `cfg!(feature = "engine")`; `engine_presets()` is empty; and the manifest's `engine` feature has no entries (read with `cargo_metadata`). `clippy` builds both. |
| CE-32.4 | **Rust:** `bindings_are_current` fails on a stale `bindings.ts` and passes after the update; a unit test of the comparison shows that one changed byte is a failure. **Static:** `svelte-check` runs over the UI that imports the bindings. The ESLint rule forbids `invoke` outside `src/lib/ipc/`, and a lint test feeds it a file that calls `invoke` directly and expects the error. |
| CE-33.1 | **Rust:** `record_view` of the fixture floor holds the header (glyph kind, id, "floor · in" the band's title, status, title, path, links-in count); the body's blocks in order: a paragraph whose chips carry their targets' titles and kinds, a heading, a list with a lore chip showing id and title, the voice block with speaker, lens and attribution, the design callout, a red chip for the dangling link and `Raw` for the malformed construct; the lore table rows with their verbs and the `never` mark; the concept caption; the front-matter rows in file order with chips; and the findings. **Page:** opening the floor from the Where tab draws screen 01's structure: `tu-rechead` with the id and title, `tu-link` chips with titles, a `tu-voice` block, the lore table, `tu-fm` rows in order, the "Used by" rows and the `tu-val` rows, each with the recorded texts. No element has inline colour (the §5.7 style test covers the components). The PR's screenshots are the visual comparison with `01-main.png`. |
| CE-33.2 | **Rust:** a clue whose `where` names the floor appears in `record_view(floor).used_by` with "where"; the floor's file is not touched (bytes hashed before and after); after the clue's file is deleted and `project_sync` runs, it is gone; after a new belief file that links the floor is written and synced, it appears. **Page:** Used by and Links in show the recorded backlinks, and clicking one opens its record. |
| CE-33.3 | **Rust:** `codex_search` with `#f04` returns the floor; with `:clue` both clues; with `:clue murmur` the one that matches; with `@` and a person's name the npc; with a plain word ranks a title match first (P03); with `:nokind` returns no hits and the line "no kind `nokind`". **Page:** typing in the find field shows the recorded hits, and the arrows and Enter open the second hit in a tab. The tree switches to its tab and selects its row with its ancestors expanded. Escape closes the results. |

Untagged tests cover the rest: `IpcError` serialisation, the state file, focus syncs at most
once a second, a record deleted outside the app, a cache in memory, the theme style test.

The build changes OS-specific code (Tauri's Windows manifest, the webview on each OS), so the
build PR gets the labels `windows` and `macos` and links their runs. `cargo xtask gate`, with
`ui`, runs on Windows before the phase is claimed done (CLAUDE.md). macOS's webview (WKWebView)
is not covered by the page tests, which run Chromium. A manual start of the app on macOS is
recorded in the PR when a Mac is at hand; otherwise it is listed under "Needs Jann".

## 6. Interfaces

### 6.1 With P48 (agreed on 27 Nov 2026 by both spec sessions; P48 spec PR #53, §4.2)

- The Codex queries live in `theos_studio::codex`: `search`, `read` and `list`, with `Marks` and
  `RecordRef`, in the shape of P48's spec §5.5–§5.7. **Whichever build reaches `codex` first
  builds them**, and the second merges `origin/main` onto it.
- **Order.** P11's plan starts only after P48's spec PR (#53) is merged, so P11 never builds
  from a shape that review may still change. If P48's build has not delivered `codex` by then,
  P11's plan includes it as P48's merged spec defines it: `Index::field_texts` (P48 §5.4),
  `Marks` (§5.5) and the `search`, `read` and `list` views (§5.6–§5.7), with their tests. P48's
  build then only maps its tools onto them.
- P11 adds nothing to P48's views and changes none of their keys. Its Codex-only views (the
  block tree, the ordered front-matter rows, the tree tabs, the query prefixes) live in its own
  studio module (`codex_view`).
- P11's IPC types live in `theos-editor` and are mapped from P48's views, so specta stays out of
  the library crates.
- Every `Link` of `Read.links` and `Read.backlinks` carries `slot`, P03's `Slot::as_db` text:
  `field:<key>`, `lens:<key>`, `beat:<key>`, `voice:speaker`, `voice:belief`, `note:version`,
  `note:resolved-by` or `mention`. A backlink's `source` is a `RecordRef`. "Used by" takes its
  relation from the slot (§5.8).

### 6.2 From earlier phases

| Phase | What P11 relies on |
|---|---|
| P02 | The body grammar (events, callouts, beats, malformed constructs) and the front-matter model with its order and spans |
| P03 | `Project::open` and its messages; `[parts] enabled`; `search`, records, links, backlinks, children; findings |
| P05 | `Workspace::open` and its in-memory fallback when the cache is held; `find_root` |
| P06 | Nothing yet: P11 runs no command (§3). P12 uses `run`, `undo`, `redo`, `read` and `Base` |

### 6.3 For other phases

| Phase | Uses |
|---|---|
| Every UI phase | The app crate, the IPC pattern (a command module, a view type, the bindings test, recordings), `Dock` and presets, the keymap and the command table, the components of `src/components/`, the `ui` check |
| P12 | The record window, to which it adds Write and Source mode; P11's Read mode, which stays (decision 0001's "P12 builds it" is done by P11); the body components (link chip, voice block, callouts, beat row), which its CodeMirror widgets mount, so every mode draws them alike; the keymap for CodeMirror; the focus sync, which it replaces with a watcher; the disabled Save, New record and "+ field" |
| P13, P15, P16, P18, P25, P14 | Each replaces its placeholder preset with its screen, sets its part's `phase` to `None`, and fills its badge. P14 adds the theme switch and may take over the state file |
| P46 | The git pip of the title and status bars, and the header's "edited … · author" |
| Engine part | `engine_presets()` and the `engine` feature |

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| tauri-specta and specta are release candidates | A future RC changes the output or the API | `=` pins; the bindings test shows every change in the diff; moving a pin is its own PR |
| Tauri adds about 370 crates | Longer cold builds, more advisories | The build cache of `main`; `deny` runs on every PR; the two ignores carry reasons and name what removes them |
| Linux packages in three jobs | Minutes, or a mirror outage that fails CI | The `.deb` cache; one package list; the `ui` job needs none |
| The page tests run Chromium; the app runs WebView2, WKWebView and WebKitGTK | A webview-only bug passes CI | The screen uses plain CSS and no browser-specific API; the `windows` and `macos` runs; manual starts on Windows (before the phase is done) and macOS (§5.14) |
| Recorded IPC drifts from the real answers | UI tests on stale data | The recordings test fails `test` on any difference; recordings are made by the real commands |
| `tauri-build`'s static CRT and the manifest link arguments on Windows | Test or coverage builds break on Windows | The probe's fix; `cargo xtask gate` on Windows before done; the weekly `windows` run |
| The focus sync on a large project | A pause when the window comes back | P03's sync compares sizes and times only (CE-12.1); at most once a second; P12's watcher replaces it |
| Glyphs outside JetBrains Mono (◴ ✱ ⚙) | They render differently per OS | Known flag of Theos UI README; kept as the reference does |
| `svelte-check` lags TypeScript 7 | A TS major cannot be taken | TypeScript 6.0.3 now; revisit when `svelte-check` supports TS 7 alone |

## 8. Review Focus — inputs most likely to break it

- A body with a voice block inside a list, a callout with a nested quote, a beat heading with
  odd attribute lines, `[[id#frag|shown]]`, two links in one line, a link to itself, a dangling
  link, a link inside inline code (no chip), and a malformed callout.
- Front matter with flow lists, quoted scalars, an empty list, a lens map, comments, a CRLF
  record and a record without a final newline (the rows must show the file's spelling).
- A record whose `parent` is dangling, a parent cycle (`a` → `b` → `a`), two records with the
  same id, a band with no floors, a floor whose number is outside its band's range.
- `theos.toml` with `enabled` empty, with an unknown name, with `atlas`, with `mcp` only.
- Search: `#`, `:`, `@` alone; `:Floor` in another case; `"quoted phrase"`; FTS operators as
  text (`AND`, `*`, `-`); an id prefix that matches nothing; a query typed faster than the
  debounce.
- A record changed, deleted or renamed outside the app while its tab is open, then the focus
  comes back; the cache held by `theos check` while the app opens.
- A project path with spaces, non-ASCII letters, a UNC path on Windows, a symbolic link.
- Keyboard: Mod+K while the find field has focus; Escape in the palette and in the results;
  shortcuts on `os = macos`; Ctrl+Tab with one tab; Mod+W with none.
- A truth with target `never` in every place it can show: tree, chip, lore table, search hit.
- Text that looks like HTML or a script in a title, a field or the body: it must show as text.
- An IPC error from every command, and a command called before a project is open.

## 9. Rulings (Claude's calls)

1. Ruling: the rail of a project is its enabled parts in architecture §6's order, plus Code and
   Settings, which are always there — architecture §6 lists the content rail, P03 §5.3 says
   `enabled` decides the rail, and `docs/ui/README.md` has P25 add Atlas, which an `enabled`
   rail reconciles; Code and Settings are not parts of `theos.toml` — cost if wrong: one filter.
2. Ruling: the editor-part registry (rail entries) lives in `theos_studio::parts`, beside the
   kinds' registry in `theos_content::parts`, and holds only rail entries in P11 — P06 ruling 12
   placed it with the first phase that registers per part beyond kinds; commands, MCP tools and
   settings join when a phase registers them per part — cost if wrong: one module moves.
3. Ruling: presets (dock trees) are data in the UI, not in Rust — they name Svelte components,
   which Rust cannot name — cost if wrong: a preset id in Rust maps to the same data.
4. Ruling: Read mode is drawn from a typed block tree built in Rust from P02's parser — one
   parser for every reader, links resolved by the index, no HTML injection (Jann chose that the
   view stays, §4.1) — cost if wrong: none for P12, whose CodeMirror editor works on the source
   text and mounts the same body components as widgets.
5. Ruling: IPC view types live in `theos-editor` and are mapped from studio views — specta is a
   release candidate and stays out of the libraries that `theos` and `theos-mcp` build — cost if
   wrong: one derive moves.
6. Ruling: the page tests answer IPC from recordings that the real commands write, checked by a
   Rust test — the UI is tested against real answers without building the app in `ui`, and a
   drift fails `test` — cost if wrong: one regeneration per changed view.
7. Ruling: P11 syncs on open, on focus and on Reload, and builds no watcher — P12 needs a watcher
   for CE-34.2 and proves it there; a read-only view is fresh enough when you come back to it —
   cost if wrong: a stale view until the next focus.
8. Ruling: the last project is kept in `<data dir>/theos/editor-state.json`, not in the user
   settings — it is app state, and P14 owns the settings layers — cost if wrong: one key moves.
9. Ruling: the header's "edited … · author", the git pips, the agent pip, the rail badges and
   screen 01's pass rows are left out until the phases that know them (P46, P07, P13, the rules)
   — showing invented values would mislead — cost if wrong: none.
10. Ruling: "Pictures" shows the concept captions with an empty frame naming P16 — CE-43.3 loads
    images from the binaries folder in P16 — cost if wrong: images in P11 would pull P16's
    binaries-folder handling forward.
11. Ruling: unbuilt controls (Write, Source, Save, New record, Open in graph, + field) are
    disabled and name their phase — Jann wants to see the layout and what comes; an active
    control that does nothing would mislead — cost if wrong: none.
12. Ruling: `engine` gets one cfg'd item (`engine_presets`) and is built and tested in both
    states by `clippy` and `test` — an empty feature with no cfg'd code would build the same
    bytes, so "builds with and without" would prove nothing about the code paths — cost if
    wrong: one extra `theos-editor` build per CI run (seconds).
13. Ruling: TypeScript 6.0.3, not 7 — `svelte-check` 4.7.8 runs only on TS 5 or 6 (§4.2) —
    cost if wrong: one version bump later.
14. Ruling: a plain Vite app, no SvelteKit — the shell has no routes and no server rendering —
    cost if wrong: a later move to SvelteKit's static adapter.
15. Ruling: `Ctrl+Tab` stays Ctrl on every OS — browsers and macOS editors use Ctrl+Tab for tabs
    — cost if wrong: one keymap line.
16. Ruling: the Codex tree shows only registered kinds, and the fixture has no spawn — `spawn`
    and `enemy` arrive with P21 (CE-57); an unknown kind would add findings to every recording
    — cost if wrong: none; they appear when P21 registers them.
17. Ruling: P11's plan waits for P48's spec (#53) to merge, and builds `codex` itself when P48's
    build has not — never build from a shape still in review — cost if wrong: a later start of
    P11's build.
18. Ruling: `data-theme` carries the design system's theme name (`grid-noir`), not `dark` — the
    tokens are scoped by that name, and the light theme will come with its own — cost if wrong:
    one attribute value.
19. Ruling: the window's CSP is `default-src 'self'` with no remote source, and the capability
    file grants only the app's commands and the folder dialog — G12 and the anti-goal "no
    network" — cost if wrong: none.

## 10. Left to the builder

- Module names inside `theos-editor`, `theos-studio::codex_view` and `editor-ui/src`.
- The exact empty-state texts of each placeholder window, within §5.9 and Theos UI's "Writing in
  the UI".
- Whether the keymap uses `tinykeys` or its own matcher (§5.11).
- The ESLint configuration beyond the restricted import, and the stylistic rules.
- The debounce of the find field within 80–200 ms, and the number of hits within 10–50.
- The placeholder icon's pixels.
- How the `ui` xtask module reports each step's output in the job summary.
