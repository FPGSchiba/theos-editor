# P51 · Window chrome — phase spec

Status: waiting for Jann's approval · 28 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P51, added by umbrella version 9 in this same PR. Scope and
acceptance criteria come from there; this spec decides how.

## 1. Goal

After P51 the editor draws its own window chrome. On Windows and Linux the OS title bar ("Theos
Editor", − □ ✕) is gone. In its place is one Theos title bar row that holds:
- the `THEOS//EDITOR` mark;
- an in-window menu bar (File, Edit, View, Go, Help);
- the breadcrumb, which is the drag region;
- the validator pip;
- the window controls.

On macOS the native traffic lights stay, inset in the Theos title bar, and the system menu bar
carries the same menus. The page never scrolls; only window bodies do, with thin scrollbars in
Theos UI's style. The window remembers its size and place, and it has a minimum size.

Jann's words: "I don't want the default bar there. It looks horrible." and "no scrollbars are
ever allowed over the whole page."

The phase runs in three steps, in this order (Jann):
1. **This spec proposes the layout**: the reference page `docs/ui/screens/00-chrome.html`
   with its screenshots in both themes.
2. **A design PR** changes the design documents: `design/theos-ui/` and `docs/ui/`.
3. **The build** implements it, working.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-84 Window chrome**
- CE-84.1 On Windows and Linux the window has no OS title bar: the Theos title bar holds the
  window controls (minimize, maximize or restore, close), drags the window, maximizes and
  restores it on double-click, and the window still resizes from its edges; on Windows 11 the
  maximize button shows the snap-layout flyout. On macOS the native traffic lights sit inset in
  the Theos title bar.
- CE-84.2 The commands of the command table are listed by menu, each with its keys from the one
  keymap: in a menu bar in the title bar on Windows and Linux, and in the system menu bar on
  macOS. On Windows and Linux, Alt or F10 focuses the menu bar, the arrow keys move and Esc
  leaves it.
- CE-84.3 The page never scrolls: at every window size from the minimum size up, the shell fills
  the window exactly and only window bodies scroll.
- CE-84.4 The window keeps its size, position and maximized state between starts, and cannot be
  made smaller than its minimum size.
- CE-84.5 `design/theos-ui/` and `docs/ui/` describe the title bar, the menu bar, the window
  controls and the scrollbars in both themes, and the build uses only what they describe.

## 3. Scope

**Goals**

The design PR (Part A, protected paths, waits for Jann):
- `design/theos-ui/` gains:
  - the components and classes of the proposal: `tu-titlebar--frameless`, `tu-titlebar--mac`,
    `tu-titlebar__drag`, `tu-menubar*`, `tu-menu*`, `tu-winctl*`;
  - the tokens: `w-winctl` 46px, `w-macos-inset`, menu sizes, `w-scrollbar`, `scrollbar-thumb`,
    `scrollbar-thumb-hover`, `scrollbar-thumb-active`, `scrollbar-track`;
  - the scrollbar rules;
  - the README sections and a component card.

  It also fixes the 1px overflow of `.tu-statusbar` and `.tu-titlebar` (§4.2).
- `docs/ui/` gains:
  - the reference screen of the chrome in both themes, the proposal promoted to
    `screens/00-chrome.html`;
  - the new title bar in every reference screen that shows one, so later phases copy the right
    chrome.

The build (Part B):
- the frameless window on Windows and Linux, and the inset title bar on macOS;
- the Theos title bar with the menu bar, the drag region and the window controls;
- the menus built from the command table and the keymap, in-window on Windows and Linux and
  native on macOS;
- a new `app.quit` command;
- the Windows 11 snap-layout flyout through `tauri-plugin-snap-layout`;
- window state through `tauri-plugin-window-state`;
- a minimum window size;
- a page that never scrolls, and themed scrollbars.

**Non-goals**
- The theme switch (P14) and any new setting (P14).
- New working commands beyond `app.quit` and `help.about`. Edit's items are new commands that stay
  disabled, naming P12, with no keys until P12 gives them some (§5.4).
- Custom title bars for other windows (none exist), and multi-window support.
- The installer and the app icon set (P18).

**Anti-goals** (P51 fails if any of these holds, even with every test green)
- A key combination or a menu label written in two places. The menus, the palette and the
  native macOS menu all come from the one command table and the one keymap.
- A page that scrolls at any size from the minimum up. Hiding the scrollbar with
  `overflow: hidden` while content still overflows counts as a page that scrolls.
- Window controls drawn by us on macOS, or an in-window menu bar there.
- A colour, size or font in a component that is not a Theos UI token, or a class that
  `design/theos-ui/` does not describe (CE-84.5).
- Unsafe code in our crates (G3). The snap-layout plugin keeps its unsafe code inside its own
  crate.
- A window state file that brings the OS title bar back.

## 4. Decisions

### 4.1 Decided by Jann (28 Nov 2026)

- **A follow-up phase to P11**, run as: propose a layout, change the design documents, build it.
  Umbrella version 9 adds P51 with CE-84, in slice S5, depending on P11.
- **The layout of the proposal**, with these choices:
  - **Windows and Linux:** one title bar row (mark, menus, breadcrumb as the drag region,
    validator pip, window controls).
  - **macOS:** native traffic lights inset, and the system menu bar.
  - **Window controls:** 46px wide (Windows 11's width), with □ becoming ▫, the restore glyph, while
    the window is maximized. Close alone turns `sig-reject` on hover.
  - **Snap layouts:** the Windows 11 snap-layout flyout through `tauri-plugin-snap-layout`.
  - **Edit:** shown, with every item disabled and naming P12.
  - **Quit:** a new command `app.quit` on Ctrl+Q (⌘Q on macOS); Alt+F4 still closes the window
    through Windows.
- **No page scrollbar ever.**
- **Also take:** window state (`tauri-plugin-window-state`), a minimum window size, and themed
  thin scrollbars inside the windows.

### 4.2 Probes (28 Nov 2026, Windows 11 build 26300 at 100 % scaling, Tauri 2.12.5, toolchain 1.99.1)

All probes ran outside the repo, in a scratch Tauri app on the repo's `Cargo.lock`.

- **Undecorated window on Windows 11** (`decorations: false`):
  - **Resize:** works on every edge. With `shadow: true` (the default), tao answers
    `WM_NCHITTEST` in the invisible border outside the frame. The top edge is a strip of about
    4px inside the client area (`TAURI_DRAG_RESIZE_BORDERS`). A maximized window does not
    resize.
  - **Aero Snap:** Win+Left, Win+Right, Win+Up and Win+Down work, and so does dragging the drag
    region to the left edge. Dragging a maximized window's bar down restores it.
  - **`data-tauri-drag-region`:** drags the window, and a double-click toggles maximize. The
    value `"deep"` covers child elements. Buttons, links and elements with a role block the drag
    (`tauri-2.12.5/src/window/scripts/drag.js`).
  - **Permissions:** `core:window:allow-start-dragging`, `allow-minimize`,
    `allow-toggle-maximize` and `allow-close`. Double-click maximize is already in
    `core:window:default`.
  - **Shadow and corners:** the shadow is kept, with rounded Windows 11 corners.
  - **Snap layouts:** a custom HTML maximize button gets no snap-layout flyout, because
    `WM_NCHITTEST` answers CLIENT and Windows needs HTMAXBUTTON. Tauri 2.12.5, wry and tao have
    nothing for it.
    - `tauri-plugin-snap-layout` 1.0.9 (MIT, 14 Jun 2026, tauri ^2, Windows 11 only) puts a
      transparent Win32 child window over the button that answers HTMAXBUTTON. Its unsafe code
      stays inside the crate. It is young: one author, ten releases in 17 days. Not tested in the
      probe, so the build's first task tests it.
    - `tauri-plugin-decorum` 1.1.1 fakes Win+Z key presses (input injection). Rejected.
- **macOS** (from source): `titleBarStyle: "Overlay"`, `hiddenTitle: true` and
  `trafficLightPosition: {x, y}` exist in tauri-utils 2.10.4. `trafficLightPosition` needs
  `titleBarStyle: "Overlay"` and `decorations: true`. The app must give its own drag region. An
  unfocused window cannot be dragged (tauri#4316). Not tested on a Mac.
- **Linux** (from source): `decorations: false` gives client-side decorations. Tauri adds a 5px
  resize area inside the webview while the window is not maximized, and there is no shadow.
  KDE Wayland once ignored `decorations: false` (tao#899). Not tested.
- **Menus:** Tauri adds a default menu only on macOS; Windows and Linux show none when the app
  sets none. P11's `menu.rs` installs a macOS-only menu.
- **The page scrollbar Jann saw:**
  - **Measured:** `scrollHeight` is 1px more than `clientHeight` at every size: 901 against 900
    at 1440×900, 641 against 640 at 1024×640, and 948 against 947 at 1449×947.
  - **Cause:** `.tu-statusbar` in `design/theos-ui/components/bundle.css` is 22px plus a 1px
    border in `content-box`, so it is 23px in a 22px grid row. `.tu-titlebar` has the same 1px,
    hidden under the toolbar.
  - **Fix:** `box-sizing: border-box` on both, measured 900/900 and 640/640. The fix is in the
    design system, so it goes in the design PR.
- **`tauri-plugin-window-state`** 2.5.0 (26 Sep 2026, Apache-2.0 OR MIT, tauri ^2.12):
  - size, position and maximized state came back exactly over five starts of the undecorated
    window;
  - its default `StateFlags::all()` also restores decorations, so a state file could bring the
    OS bar back. The build uses `StateFlags::all() - StateFlags::DECORATIONS`.
- **Minimum size:**
  - the record column is the window width minus 638px, and the toolbar clips at 900px or less;
  - proposed minimum: 1024 × 640 (the record column 386px, the tree body about 500px).

### 4.3 Approaches considered

| Question | Chosen | Considered |
|---|---|---|
| Chrome on Windows and Linux | frameless window, Theos title bar in HTML | the native title bar (Jann: "horrible"); `titleBarOverlay`-style hybrids (not in Tauri 2.12.5 for Windows) |
| Chrome on macOS | native traffic lights inset (`titleBarStyle: Overlay`), system menu bar | our own controls and an in-window menu (against platform convention, and loses system features) |
| Snap layouts | `tauri-plugin-snap-layout` (Jann) | none, with Win+Z and drag-to-edge only; `tauri-plugin-decorum` (injects Win+Z); our own hit-test (needs unsafe Win32 code, G3) |
| One source for menus | the UI's command table and keymap; on macOS the UI hands the menu model to Rust, which builds the native menu | a second menu table in Rust (two places for every label and key) |
| Per-OS window config | Tauri's platform files `tauri.windows.conf.json`, `tauri.linux.conf.json`, `tauri.macos.conf.json`, merged over `tauri.conf.json` | builder code with `cfg` per OS (more code, and coverage on one OS only) |
| No page scroll | fix the 1px in the design system; the shell fills the window; a test at many sizes | `overflow: hidden` on the page (hides a real overflow and still clips) |
| Window state | `tauri-plugin-window-state` without the decorations flag | our own state file (P11's `editor-state.json` holds only the last project) |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Where | Crate or tool | Our code, and why |
|---|---|---|---|
| Window config per OS | `crates/theos-editor/tauri*.conf.json` | Tauri's platform config files | none |
| Window controls, drag | `editor-ui/src/shell/TitleBar.svelte`, `WindowControls.svelte` | `@tauri-apps/api/window` (through `src/lib/ipc/`), `data-tauri-drag-region` | the components |
| Snap layouts | `crates/theos-editor` (cfg windows) | `tauri-plugin-snap-layout` =1.0.9 | registration and alignment |
| Menu model | `editor-ui/src/lib/menus.ts` | the command table (`commands.ts`) and the keymap (`keymap.ts`) | the grouping |
| In-window menu bar | `editor-ui/src/shell/MenuBar.svelte`, `Menu.svelte` | – | keyboard handling per the WAI-ARIA menubar pattern |
| Native macOS menu | `crates/theos-editor/src/menu.rs` | Tauri's menu API | built from the model the UI sends |
| Window state | `crates/theos-editor` | `tauri-plugin-window-state` =2.5.0 | the flags |
| Minimum size | `tauri.conf.json` | Tauri | none |
| Scrollbars | `design/theos-ui/components/bundle.css` | `::-webkit-scrollbar` (read by WebView2, WKWebView and WebKitGTK) | the tokens and rules (design PR) |

New crates: `tauri-plugin-snap-layout` (Windows only, `[target.'cfg(windows)'.dependencies]`) and
`tauri-plugin-window-state`, both pinned with `=` in `[workspace.dependencies]`. Both licences
(MIT; Apache-2.0 OR MIT) are on `deny.toml`'s list. The npm side needs
`@tauri-apps/plugin-window-state` only if the UI calls it; it does not plan to.

### 5.2 The window per OS

- **`tauri.conf.json`** (all OSes): `minWidth: 1024`, `minHeight: 640`; the size 1440 × 900 stays
  the first-start size.
- **`tauri.windows.conf.json` and `tauri.linux.conf.json`**: `decorations: false`, `shadow: true`
  (on Linux the shadow is not supported and is ignored).
- **`tauri.macos.conf.json`**: `decorations: true`, `titleBarStyle: "Overlay"`,
  `hiddenTitle: true`, and `trafficLightPosition` so the lights are centred in the 30px row.
- **Capabilities** (`capabilities/main.json`): add `core:window:allow-start-dragging`,
  `allow-minimize`, `allow-toggle-maximize`, `allow-close` and `allow-is-maximized`,
  `core:window:allow-internal-toggle-maximize` for the double-click of the drag region, and the
  events the restore glyph needs. Nothing else, and not the whole `core:window:default` set.
- **The UI knows its OS** from `app_info().os` (P11), and sets `data-os` on the root, so the title
  bar picks its variant (`tu-titlebar--frameless` or `tu-titlebar--mac`).

### 5.3 The title bar

One row of `h-titlebar` (30px), the classes of the design PR:

| Part | Windows and Linux | macOS |
|---|---|---|
| left | the mark, then the menu bar | the `w-macos-inset` space for the lights, then the mark |
| middle | the breadcrumb, the drag region (`data-tauri-drag-region`) | the same, centred on the window |
| right | the validator pip, then − □ ✕ (46px each) | the validator pip |

- **The drag region** is the title bar's empty space and the breadcrumb text. The mark, the menu
  labels, the pip and the controls are not draggable, because they are interactive.
  Double-click on the drag region maximizes or restores.
- **The controls:**
  - minimize (−);
  - maximize (□, U+25A1) or restore (▫, U+25AB): the glyph follows the window state from the
    resize and maximize events. JetBrains Mono has neither ❐ nor ⧉, nor today's ▢, so the design
    PR replaces ▢ everywhere;
  - close (✕).

  They take the full row height below Tauri's top resize strip (the token `h-resize-edge`, 4px).
  Restored, their hit area runs from 4px to 30px; maximized, it takes the full height.
- **The snap-layout flyout:** `tauri-plugin-snap-layout` places its child window over the maximize
  button. The button has a fixed size and a fixed place: the second 46px slot from the right
  edge, the full row height. The build's first task checks on Windows that the flyout appears
  and that a click on the button still maximizes. If the plugin cannot do that on Tauri 2.12.5,
  the build stops, and the PR raises it with Jann as an AC that cannot be met.
- **The toolbar** stays the second row, as in P11. The title bar no longer repeats "THEOS//EDITOR"
  above it: the OS bar is gone, and the mark sits in the Theos row.

### 5.4 Menus from one source

- **`menus.ts`** holds the menu model: `Menu { id, label, accessKey, items }`, where an item is a
  command id, a separator or a submenu. It names commands by id only. Each label comes from
  `commands.ts`, each key from `keymap.ts`, and each enabled state and phase from the command
  table. A command in a menu that is not in the command table is a type error.
- **The menus** follow the proposal:

  | Menu | Items |
  |---|---|
  | File | Open project…, Reload project, Save (disabled, P12), Close tab, Quit |
  | Edit | Undo, Redo, Cut, Copy and Paste: new commands `edit.undo` … `edit.paste`, disabled, naming P12, with no keys yet |
  | View | Command palette; the Layout submenu, which is the rail; the Codex tree tabs |
  | Go | Find record, Next tab, Previous tab |
  | Help | About Theos Editor (`help.about`) |

  "About" is a small read-only dialog (version, licence, links as text), a Theos UI popover; it
  is the only new view.
- **New commands:**
  - `app.quit` on `Mod+Q`, in File (Windows and Linux) and in the palette; it closes the window
    through Tauri;
  - `help.about`, with no keys;
  - `edit.undo`, `edit.redo`, `edit.cut`, `edit.copy` and `edit.paste`, disabled, naming P12,
    with no keys. Giving them keys now would make the shell's key listener prevent Ctrl+C and
    Ctrl+V in text fields; P12 adds the keys together with the commands' work.
- **A command appears in at most one menu.** A test checks it. "Keyboard shortcuts" is not in
  Help, because it would repeat View's Command palette.
- **The in-window menu bar** (Windows and Linux) follows the WAI-ARIA menubar pattern:
  - Alt (pressed and released) or F10 focuses the first menu, and the access letters underline;
  - Left and Right move between menus, Down, Enter or Space opens one, and Up and Down move in
    it;
  - Right opens a submenu and Left closes it;
  - Enter runs the item, a letter jumps to it, and Esc closes and returns the focus where it was;
  - Alt+letter opens the menu of that access letter. A test checks that no keymap entry owns
    Alt+ an access letter;
  - a click opens a menu, and hovering another top label switches while one is open;
  - disabled items stay listed, dimmed, with their phase, and do nothing.

  Alt pressed and released alone focuses the menu bar. Alt with a key that is neither an access
  letter nor in the keymap goes to the webview.
- **The native menu** (macOS):
  - **What is sent:** the UI sends the model, already resolved (labels, keys as Tauri
    accelerators, enabled state), through one IPC command `menu_set(model)`. It sends it at start
    and again whenever a project opens, because the Layout submenu is the project's rail.
  - **What Rust builds**, with Tauri's menu API and item ids equal to command ids:
    - the app menu: About (runs `help.about`), Services, Hide, Hide Others, Show All, and Quit
      (`app.quit`, ⌘Q);
    - Edit: Tauri's built-in items (Undo, Redo, Cut, Copy, Paste, Select All). WKWebView needs
      them for copy and paste in text fields, so the model's `edit.*` items are not built on
      macOS until P12;
    - File, View, Go and Help from the model, without the items the app menu carries.
  - **Enabled state:** only commands whose phase is not built are disabled natively. The
    enabled state that changes while the app runs (Next tab, Close tab) is not mirrored. The
    command checks it when it runs, and does nothing when it is disabled.
  - **Keys:** the native menu owns every accelerator it shows. On macOS the shell's key listener
    skips the commands the native menu carries with a key, so a key never runs a command twice.
    Close tab is shown without an accelerator, so Cmd+W stays with the webview and closes a tab,
    never the window (P11's ruling).
  - A menu click emits an event with the command id, and the UI runs that command. P11's
    hard-coded macOS `MENU` goes.
- **The palette** keeps listing every command. The menus and the palette are two views of one
  table.

### 5.5 The page never scrolls

- The design PR fixes `.tu-statusbar` and `.tu-titlebar` with `box-sizing: border-box`.
- The shell is the grid `100vh` (`tu-shell`). The UI sets `html, body { height: 100%; margin: 0 }`
  and nothing that overflows the shell.
- Scrolling happens only in `tu-window__body` and the menus' own lists.
- **A test** (Playwright) measures at several sizes: 1024 × 640 (the minimum), 1280 × 720,
  1440 × 900, 1920 × 1080, and a tall 1024 × 1200. At each size:
  - `documentElement.scrollHeight === clientHeight`, and the same for widths;
  - `body` likewise;
  - every open layout: the Codex with a long record, each placeholder layout, the palette open
    and a menu open.

  A further test lists every element whose box ends outside the viewport, and expects none.
- **Themed scrollbars:** the design PR's tokens and rules (10px track, 6px thumb; the thumb
  `textDim` on hover, amber while dragging). The UI uses them unchanged.

### 5.6 Window state and minimum size

- **`tauri-plugin-window-state`** with `StateFlags::all() - StateFlags::DECORATIONS`. It saves when
  the window closes and restores before the window shows, so the window opens where it was,
  maximized if it was.
- **A saved place off every current monitor** (a monitor unplugged) falls back to the default
  size, centred. The plugin does this; a test with an invented state file checks it where it
  can.
- **The minimum** of 1024 × 640 comes from §4.2, enforced by Tauri on resize.

### 5.7 Tests, fixtures and evidence

- **Rust:**
  - the platform config files parse and hold the values of §5.2;
  - the capability file grants exactly the listed permissions;
  - the window-state flags exclude DECORATIONS;
  - `menu_set` builds a menu whose ids, labels and accelerators equal the model, testable on
    every OS through the model type; the native build runs on macOS in `platforms`.
- **Vitest:**
  - `menus.ts`: every menu item is a command of the table, every key comes from the keymap,
    there are no duplicates, and every disabled item names its phase;
  - the menu bar's keyboard model (Alt, F10, arrows, Esc, letters, submenus);
  - the title bar picks its variant from the OS.
- **Page tests** (Playwright, with P11's recorded IPC):
  - the title bar row on `os = windows` (menus, drag attributes, controls) and on `os = macos`
    (the inset, no menus, no controls);
  - the controls call the window commands, answered by a mock;
  - every menu opens and runs a command by keyboard and by mouse;
  - the no-scroll tests of §5.5.
- **CE-84.5:** a test reads every `tu-*` class and every `var(--…)` the UI uses and checks that
  `design/theos-ui/` defines it, so the build uses nothing the design does not describe.
- **Manual evidence** on Windows 11, attached to the build PR:
  - the frameless window with screenshots in both themes;
  - drag, double-click, resize from each edge, Aero Snap;
  - the snap-layout flyout on hovering the maximize button;
  - window state after a restart.

  The window behaviour comes from the OS and cannot run in CI. macOS and Linux get one manual
  start each when a machine is at hand, listed under "Needs Jann" otherwise. The PR gets the
  labels `windows` and `macos`.

### 5.8 How each AC is proven

| AC | Proven by |
|---|---|
| CE-84.1 | Rust: the platform configs (no decorations on Windows and Linux; Overlay, hidden title and traffic-light position on macOS) and the capabilities. Page: the title bar's controls call minimize, toggle-maximize and close; the restore glyph follows the maximized state; the drag region carries `data-tauri-drag-region` and the interactive parts do not; on `os = macos` there are no controls and the inset is there. The manual Windows evidence of §5.7 for drag, resize, snap and the flyout. |
| CE-84.2 | Vitest: the menu model against the command table and the keymap; the keyboard model. Page: Alt and F10 focus File; the arrows, Enter and Esc; a click opens a menu and runs an item; keys shown are the platform's (Ctrl on Windows, ⌘ on macOS). Rust: `menu_set` builds the native menu from the model. |
| CE-84.3 | Page: the no-scroll measurements of §5.5 at every listed size and layout, and the overflow test. |
| CE-84.4 | Rust: the window-state plugin is registered with the flags without DECORATIONS; `minWidth` and `minHeight` are set. Manual: size, place and maximized state come back after a restart; the window does not shrink below 1024 × 640. |
| CE-84.5 | Part A's design PR. Vitest: every class and token the UI uses is defined in `design/theos-ui/`. |

## 6. Interfaces

### 6.1 From earlier phases

| Phase | What P51 relies on |
|---|---|
| P11 | The shell (`TitleBar`, `Toolbar`, `Shell`), the command table and keymap, `app_info().os`, the IPC pattern (bindings, recordings, replay), `menu.rs` and its macOS ruling |

### 6.2 For other phases

| Phase | Uses |
|---|---|
| Every UI phase | A new command appears in the palette and, when listed in `menus.ts`, in the menus on every OS; the no-scroll test covers new layouts once they are added to its list |
| P12 | Enables Edit's items and Save by giving their commands a `run` |
| P14 | The theme switch and settings join View or a Settings item |
| P18 | The app icon in the title bar's mark slot, and the installer window defaults |

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| `tauri-plugin-snap-layout` is young and has one author | It breaks with a Tauri update, or its child window misplaces at other DPI | `=` pin; the build's first task tests it at 100 %, 125 % and 150 %; if it cannot meet CE-84.1, the PR says so to Jann (an AC that cannot be met) |
| Only 100 % DPI and one monitor were probed | Edges, the drag strip or the flyout misplace on HiDPI or a second monitor | Manual evidence at 125 % and 150 %; the controls' box is in CSS pixels, which Tauri scales |
| Tauri's top resize strip (about 4px inside the client) | Clicks at the very top of a control are taken by the resize | The design gives the controls their full height and accepts the top 4px as resize, like Windows itself |
| Linux window managers (KDE Wayland ignored `decorations: false` once) | A double title bar on one desktop | Documented; Linux is checked by hand when a machine is at hand |
| macOS Overlay differs per OS version | The lights sit off-centre | `trafficLightPosition` pinned to the 30px row, with a manual check |
| Alt focusing the menu bar fights Alt+key shortcuts | A shortcut is lost | Only Alt pressed and released alone focuses it; the keymap's own keys win |
| A state file from before P51 | It could restore decorations | The DECORATIONS flag is excluded; a test pins it |

## 8. Review Focus — inputs most likely to break it

- **Keyboard:** Alt pressed and released, against Alt held with a letter; F10; Esc while a submenu
  is open; Alt while the palette or the find field has the focus.
- **Window size:** a window at exactly the minimum size; one maximized, then restored; snapped to
  half the screen (the page must still not scroll).
- **The record:** a very long record and a very long title; both themes.
- **Drag region:** a double-click on the breadcrumb text against a double-click on a menu label
  (only the first maximizes).
- **The window state file:** missing, corrupt, or naming a monitor that is gone.
- **macOS:**
  - the menu model sent before the first window shows;
  - a command enabled later (P12) updating the native menu;
  - Cmd+W.
- **The page scroll:** the 1px class anywhere else, such as a new component with a border in
  `content-box`.

## 9. Rulings (Claude's calls)

1. Ruling: the menus have one source, the UI's command table and keymap. On macOS the UI sends
   the resolved model to Rust (`menu_set`) — a second table in Rust would write every label and
   key twice — cost if wrong: one IPC command.
2. Ruling: the per-OS window options live in Tauri's platform config files — no `cfg` code to
   cover per OS — cost if wrong: none.
3. Ruling: the minimum window size is 1024 × 640 — measured (§4.2): the record column keeps
   386px and the toolbar does not clip — cost if wrong: two numbers.
4. Ruling: the page-scroll fix belongs to the design system (`box-sizing: border-box` on
   `.tu-statusbar` and `.tu-titlebar`) and goes in the design PR. `overflow: hidden` is not the
   fix — the 1px comes from `bundle.css` — cost if wrong: none.
5. Ruling: Alt pressed and released alone focuses the menu bar, and so does F10; Alt+ an access
   letter opens that menu, and no keymap entry may own one; any other Alt+key goes to the keymap
   or the webview — the Windows convention, without losing shortcuts — cost if
   wrong: one rule.
6. Ruling: "About Theos Editor" is a small read-only popover (version, licence) — Help needs a
   target, and this is the smallest one — cost if wrong: one component.
7. Ruling: the reference screens all get the new title bar in the design PR — later phases copy
   the chrome from them — cost if wrong: a few HTML edits.
8. Ruling: the window-state plugin restores everything except decorations — a pre-P51 state file
   must not bring the OS bar back — cost if wrong: none.
9. Ruling: maximize is □ (U+25A1) and restore is ▫ (U+25AB), and ▢ goes everywhere — JetBrains
   Mono, the face of every UI glyph, has neither ❐, ⧉ nor ▢, so those fall back to the OS font —
   cost if wrong: two glyphs.
10. Ruling: Edit's items and About are new commands (`edit.*` disabled and keyless until P12,
    `help.about`), so the menus name commands only — a second kind of menu item would bypass the
    one table; keys on Edit now would swallow Ctrl+C and Ctrl+V in text fields — cost if wrong:
    none.
11. Ruling: the native macOS menu keeps Tauri's built-in Edit items and the app menu's Hide and
    Quit, is sent again when a project opens, mirrors only the phase-disabled state, and owns the
    accelerators it shows — WKWebView needs the built-in Edit items for copy and paste; mirroring
    every enabled change would mean IPC on each tab switch — cost if wrong: one more `menu_set`
    call site.

## 10. Left to the builder

- The exact `trafficLightPosition`, checked on a Mac.
- The About popover's text.
- How the no-scroll test enumerates layouts (a list in the test, kept in step with the presets
  by a check).
