# theos-editor

Library (Tauri 2) with a thin binary. The authoring shell: Tauri commands map onto
`theos-studio`'s views (P11 is read-only; the command layer joins when editing does), and typed
IPC bindings are generated for the UI with tauri-specta. Every key and value on the IPC surface is
snake_case.

- `src/lib.rs`: `builder` sets up the app (state, folder dialog, commands) on any Tauri
  runtime, so the tests run the same app on Tauri's mock runtime. `specta_builder` lists every
  command. `run` starts logging and runs the app until its window closes.
- `src/main.rs`: `run` with Tauri's runtime and the app menu, the app's context and the command
  line, nothing else.
- `src/menu.rs`: the app menu. On macOS it replaces Tauri's default menu, whose "Close Window"
  sits on Cmd+W where the editor's Close tab is: the app submenu with Quit, and Edit. Windows and
  Linux get no menu. The menu is a plain list (`MENU`) that every OS tests; building it needs the
  macOS main thread, so a manual start on macOS checks it.
- `src/ipc/`: one module per group of commands, and `IpcError`, which every command returns as
  `{ "kind": …, "message": … }`.
- `src/ipc/project.rs`: opening and syncing. `project_sync(reason)`: a `focus` sync (the window
  got the focus back) runs at most once a second (`SYNC_INTERVAL`) and within the second answers
  the open view; a `reload` (Ctrl+R) always syncs. A record that only moved, with the same id, is
  never reported removed. Unknown parts in `enabled` are logged once, when the project opens.
- `src/state.rs`: the state file, `<data dir>/theos/editor-state.json`. It remembers the last
  project and is the only file the app writes. Tests pass their own folder to
  `AppState::new` through `builder_with`.
- `src/app_state.rs`: the open project, the time of the last sync and the folder named on the
  command line.
- The tests open copies of `../theos-studio/tests/fixtures/projects/editor-codex/`, a synthetic
  content project that the Codex views of `theos-studio` test on too; the copies leave out a
  `.theos/` folder, so a cache left by a manual start never reaches a test.

The build needs no web UI: `tauri.conf.json` names `devUrl` (`http://localhost:5179`), which the
app loads when `custom-protocol` is off. On Windows `build.rs` embeds the app manifest in every
target, so the test harnesses start too.

The cargo feature `engine` is reserved for the engine part. It stays empty; its one cfg'd item is
`presets::engine_presets()`, an empty list.

## Running it

The app opens the folder named on the command line, else the last project, else an empty shell
(spec §5.9). Opening a project writes P03's index cache into its `.theos/cache/`, so open a copy
of a fixture rather than the fixture itself. From the repository root, copy it into the ignored
`target/` (in PowerShell: `Copy-Item -Recurse`) and start the dev server and the app with one line:

```text
cp -r crates/theos-studio/tests/fixtures/projects/editor-codex target/editor-codex
pnpm --dir editor-ui app target/editor-codex
```

or in two terminals, the dev server first:

```text
pnpm --dir editor-ui dev
cargo run -p theos-editor -- target/editor-codex
```

Logging follows `RUST_LOG` (for example `RUST_LOG=theos_editor=debug`). Closing the window stops
the app; `pnpm … app` then stops the dev server too, and its Ctrl+C
stops the app's whole process tree.

## Tests

```text
cargo nextest run -p theos-editor
cargo nextest run -p theos-editor --features engine
cargo clippy -p theos-editor --all-targets --locked -- -D warnings
```

The tests build the app on Tauri's mock runtime and call every command through IPC; no window
opens and no web UI is needed.

## Bindings and recordings

Two tests compare generated files with the checked-in ones and fail when they differ:

- `bindings_are_current` (`tests/bindings.rs`): `editor-ui/src/lib/ipc/bindings.ts`, the
  commands and types tauri-specta exports;
- `ipc_recordings_are_current` (`tests/recordings.rs`): `editor-ui/e2e/ipc/*.json`, real answers
  of the commands on copies of the fixtures, which the UI's page tests replay. A recording made on
  another fixture than `editor-codex` names it in its `fixture` key.

After changing a command, a view type or the fixtures, write them again and commit the result:

```text
UPDATE_BINDINGS=1 cargo test -p theos-editor --test bindings --test recordings
```
