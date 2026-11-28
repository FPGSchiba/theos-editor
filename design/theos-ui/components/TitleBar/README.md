The app's own title bar for the frameless window: mark, menu bar, breadcrumb and drag region, validator pip and window controls in one `h-titlebar` (30px) row. The toolbar row stays below it.

**Use**
```html
<header class="tu-titlebar tu-titlebar--frameless">            <!-- add is-maximized when maximized -->
  <span class="tu-titlebar__brand"><span class="tu-mark"></span>THEOS//EDITOR</span>
  <nav class="tu-menubar">…</nav>                               <!-- see MenuBar -->
  <div class="tu-titlebar__drag" data-tauri-drag-region><span>tower-of-theos</span><span class="tu-titlebar__sep">▸</span>…</div>
  <span class="tu-statuspip">…</span>
  <div class="tu-winctl">…</div>                                <!-- see WindowControls -->
</header>
```
- **Windows and Linux:** `tu-titlebar--frameless` (no clipping so menus hang below it, no right padding so the controls sit flush). `tu-titlebar__drag` takes the free space, centres the breadcrumb, and is the drag region; a double-click on it maximizes or restores. Menu labels and window controls are not drag regions.
- **macOS:** `tu-titlebar--mac` together with `--frameless`: a three-column grid (left, breadcrumb, right) so the breadcrumb is centred on the window. The left column starts with `tu-titlebar__inset`, `w-macos-inset` (72px) of room for the native traffic lights. There is no in-window menu bar (the menus live in the system menu bar) and no window controls.
- The title bar is `box-sizing: border-box`, so with its 1px border it is exactly its grid row.
- Type: brand and menu labels are Instrument (`font-ui`); breadcrumb and pip are mono `textDim`; the separator `▸` is `textMute`; the layout name is `sig-attention`.
