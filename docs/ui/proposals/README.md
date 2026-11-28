# UI proposals

A proposal is a reference page drawn before the design system changes. It is reviewed in a spec
PR; once approved, its tokens and classes move into `design/theos-ui/` in their own design PR,
and the page stays here as the record of what was decided.

## P51 — Window chrome

- `P51-window-chrome.html`: the reference page, built only from `design/theos-ui/` plus page
  styles that read existing tokens. Add `?theme=grid-light` for Grid Light. States a static page
  cannot show (hover, open, focus) are drawn with `is-*` classes.
- `P51-window-chrome.png`, `P51-window-chrome-light.png`: renders at 1440 wide, Grid Noir and
  Grid Light. If they disagree with the HTML, the HTML wins.

Frames: (a) Windows and Linux, menu closed; (a2) the window controls restored and maximised;
(b) File menu open; (c) View menu with the Layout
submenu; (d) menu bar focused by keyboard; (e) macOS; (f) a window body with the themed
scrollbar; (g) the Edit, Go and Help menus and the macOS app menu, drawn flat.

### New tokens

| Token | Value | For |
| --- | --- | --- |
| `w-winctl` | 46px | one window control (46 × `h-titlebar`), the Windows 11 width |
| `h-resize-edge` | 4px | Tauri's resize strip along the window's top edge; the controls start below it |
| `w-macos-inset` | 72px | room for the native traffic lights |
| `w-menu-min` | 248px | a dropdown's least width |
| `h-menu-item` | 24px | one menu row |
| `w-scrollbar` | 10px | the scrollbar track; the thumb is inset 2px (6px wide) |
| `scrollbar-track` | `panel` | the track, with a 1px `border` line on its inner edge |
| `scrollbar-thumb` | `color-mix(textDim 70%, panel)` | thumb at rest: 3.3:1 on the track in both themes |
| `scrollbar-thumb-hover` | `textDim` | thumb under the pointer (5.6:1 Noir, 6.7:1 Light) |
| `scrollbar-thumb-active` | `sig-attention` | thumb while dragging, as the slider rule ("amber only while dragging") |

### New component classes

| Class | What it is |
| --- | --- |
| `tu-titlebar--frameless` | the title bar when the app draws the window: no clipping (menus hang below it), no right padding (controls sit flush) |
| `tu-titlebar--mac` | macOS: a 3-column grid so the breadcrumb is centred on the window, not on the space left over |
| `tu-titlebar__drag` | the breadcrumb, also the drag region (`data-tauri-drag-region`); double-click maximises |
| `tu-titlebar__inset` | the 72px left inset the traffic lights sit in |
| `tu-menubar`, `__slot`, `__item`, `__key` | the in-window menu bar; `is-open`, `is-focus`; `tu-menubar.is-keys` underlines the access letters |
| `tu-menu` (`--sub`) | a dropdown (`panel2`, `borderHi`, `shadow-popover`, radius 0); a submenu opens to the right |
| `tu-menu__item` | one row: glyph gutter, label, keys; `is-active` (amber 13% wash + 2px amber edge, as `tu-pop__row`), `is-disabled`, `is-current` |
| `tu-menu__glyph`, `__label`, `__phase`, `__keys`, `__more` | gutter glyph, label, "arrives with PNN", shortcut, submenu `▸` |
| `tu-menu__sep`, `tu-menu__group` | separator line; small uppercase group heading |
| `tu-winctl`, `tu-winctl__btn` (`--close`) | the window controls − □ ✕ (▫ when maximised); hover `panel2`; close alone turns `sig-reject` with `onFill`; padded down by `h-resize-edge` unless the title bar `is-maximized` |

### Design-system fixes the design PR carries

- **1px page scroll.** `.tu-titlebar` and `.tu-statusbar` in `components/bundle.css` use
  `box-sizing: content-box` with a 1px border, so each is 1px taller than its grid row
  (`h-titlebar`, `h-statusbar`) and the page scrolls by 1px (measured 901 vs 900). This is the
  page scrollbar seen in the app. Fix: `box-sizing: border-box` on both (the proposal page
  already applies it).
- **Window-control glyphs.** `▢` (U+25A2), listed in the README's UI glyphs today, is **not** in
  JetBrains Mono and falls back to the OS symbol font; neither are `❐` (U+2750) nor `⧉`
  (U+29C9), checked against the font's cmap. Maximise becomes `□` (U+25A1 WHITE SQUARE) and
  restore `▫` (U+25AB WHITE SMALL SQUARE), both in JetBrains Mono. The README's glyph line
  becomes "− □ ▫ ✕ window controls"; the tab bar's `▢` in `tu-tabbar__ctrls` moves to `□` too.
- **Scrollbar.** Inside `tu-shell`, `scrollbar-width` and `scrollbar-color` go back to
`auto`, and the scrollbar is styled with `::-webkit-scrollbar` only. Blink ignores
`::-webkit-scrollbar` while either standard property is set (bundle.css sets both on `*` today),
and the standard properties have no hover or drag state. WebView2, WKWebView and WebKitGTK all
read `::-webkit-scrollbar`, so one rule covers the three OSes.

### Windows 11 snap layouts

The maximise button gets the native snap-layout flyout through `tauri-plugin-snap-layout`, a
transparent Win32 child window laid over the button. So:

- The button's hit area must line up with that child window exactly: fixed size (46 × 26
  restored, 46 × 30 maximised) and fixed position (the second control from the right edge of
  the title bar). The controls never change width or order, and nothing to their right moves.
- The top 4px of the window are Tauri's resize strip. The controls do not depend on clicks
  there: `tu-winctl` is padded down by `h-resize-edge`, so each control's hit area (and its
  hover fill) runs from 4px to 30px. A click in the strip resizes the window.
- Maximised, there is no resize strip: the padding drops (`is-maximized`) and the controls run
  the full 30px to the screen edge, and the child window grows with them.
- On Linux the plugin does nothing; the button is an ordinary maximise.

### Decisions

- **One row.** Mark, menu bar, breadcrumb, validator pip, window controls share `h-titlebar`
  (30px); the toolbar row stays below it unchanged.
- **Menu labels** are Instrument: 10px uppercase, `textDim`, `text` when open or hovered. Menu
  **items** are sentence case at 12px, the command labels of `commands.ts` as the palette shows
  them; keys come from `keyLabel()` of `keymap.ts`, mono, right-aligned.
- **Menus are built from the command table.** Each item is a `CommandId`; its enabled state is
  `enabled()`, its phase note is `phase`. A new command needs a menu entry only if it should
  appear in a menu.
- **Disabled items** are `textMute` (allowed for disabled items) and name their phase in
  `textDim` mono ("arrives with P12"). They are not highlighted on hover.
- **Highlighted rows** reuse the palette row's amber wash and 2px amber edge; keys on that row
  turn `text`, because `textDim` on the wash is 4.2:1 in Grid Noir.
- **Layout submenu** lists the project's rail in rail order with its rail glyphs and Ctrl+1 … 9;
  the current layout's glyph is amber, as the active rail entry. **Codex tree** tabs sit as a
  group in View, the current tab marked `▸`.
- **Breadcrumb centred** in the drag region on all OSes; on macOS centred on the whole window.
- **Window controls** are 46px wide (Jann), flat, no radius, separated from the pip by one
  `border` line; − □ ✕ in mono `textDim`, with ▫ for restore while maximised. Only close takes a
  signal (`sig-reject`) on hover.
- **Edit** is shown with every item disabled until P12 (Jann).
- **Quit** is a new command `app.quit` on `Mod+Q`: Ctrl+Q, ⌘Q (Jann). Alt+F4 still closes the
  window through Windows.
- **Keyboard:** Alt pressed and released alone, or F10, focuses File with an amber focus outline
  and underlines the access letters; ← → move, ↓ ⏎ Space open, a letter opens its menu, Esc
  returns focus to where it was. Alt+letter opens that menu directly.
- **macOS:** traffic lights stay native in a 72px inset; no in-window menu; the same menus go to
  the system menu bar with ⌘ keys, plus the standard app menu (About, Hide, Quit).
- **Scroll:** the shell is exactly the window (`100vh`, no page scroll); only `tu-window__body`
  scrolls.

### Contrast (checked in both themes, Playwright + computed styles)

All menu, title bar and window-control text passes 4.5:1, except the breadcrumb separators `▸`
(`textMute`, existing and redundant) and disabled items (`textMute`, 2.5:1 Noir, 3.3:1 Light).
Focus outline `sig-attention` on `bg`: 8.6:1 Noir, 4.6:1 Light. Close hover `onFill` on
`sig-reject`: 6.3:1 / 6.7:1. Scrollbar thumb at rest 3.3:1 on its track. The menu edge
`borderHi` on `panel2` is 1.6:1 / 2.1:1, as for `tu-pop` today: the edge is structural and the
popover shadow separates the layer.

### Open questions for Jann

1. **Restore glyph.** `▫` is small at 12px (see frame A2). JetBrains Mono has no
   overlapping-squares glyph; the alternative is `◫` or `⊡`, but those are the Concept and
   Template kind glyphs. Keep `▫`?
2. **Menu bar focus by Alt alone** also fires when a user releases Alt after a chord the app
   does not see; keep Alt, or F10 only?

## Settled by the spec after this proposal

The phase spec (`docs/specs/phases/P51-window-chrome.md` §5.4, rulings 5 and 10) wins where this
page differs:
- Help holds only About Theos Editor. "Keyboard shortcuts" is dropped, because it would repeat
  View's Command palette.
- Alt pressed and released, or F10, focuses the menu bar, and Alt+ an access letter opens that
  menu. No keymap entry may own Alt+ an access letter.
- Edit's items are commands that stay disabled and keyless until P12.

The design PR draws the menus as the spec says.
