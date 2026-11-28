The app's own title bar for the frameless window: mark, menu bar, breadcrumb and drag region, validator pip and window controls in one `h-titlebar` (30px) row. The toolbar row stays below it.

**Use**
```html
<header class="tu-titlebar tu-titlebar--frameless" data-tauri-drag-region>  <!-- add is-maximized when maximized -->
  <span class="tu-titlebar__brand" data-tauri-drag-region="false"><span class="tu-mark" role="img" aria-label="Theos"></span>THEOS//EDITOR</span>
  <nav class="tu-menubar">…</nav>                               <!-- see MenuBar -->
  <div class="tu-titlebar__drag" data-tauri-drag-region="deep"><span>tower-of-theos</span><span class="tu-titlebar__sep">▸</span>…</div>
  <span class="tu-statuspip" data-tauri-drag-region="false">…</span>
  <div class="tu-winctl">…</div>                                <!-- see WindowControls -->
</header>
```
macOS (no menu bar, no window controls):
```html
<header class="tu-titlebar tu-titlebar--mac" data-tauri-drag-region>
  <div class="tu-titlebar__left" data-tauri-drag-region="deep">
    <span class="tu-titlebar__inset" aria-hidden="true"></span>   <!-- room for the native traffic lights -->
    <span class="tu-titlebar__brand" data-tauri-drag-region="false"><span class="tu-mark" role="img" aria-label="Theos"></span>THEOS//EDITOR</span>
  </div>
  <div class="tu-titlebar__drag" data-tauri-drag-region="deep"><span>tower-of-theos</span><span class="tu-titlebar__sep">▸</span>…</div>
  <div class="tu-titlebar__right" data-tauri-drag-region="deep"><span class="tu-statuspip" data-tauri-drag-region="false">…</span></div>
</header>
```
- **Drag regions.** Tauri 2.12.5 treats a bare `data-tauri-drag-region` (or `="true"`) as direct clicks only: a click on a child span does not drag. Only `data-tauri-drag-region="deep"` covers the descendants, so every drag region here is written `="deep"`; the breadcrumb text then drags the window. Interactive children (menu labels, window controls) are never inside a drag region. The `<header>` itself carries a bare `data-tauri-drag-region` (direct hits only) so its padding and the gaps between its children drag too. What blocks a drag: Tauri's drag script does not start one from a clickable tag (button, a, input and the like), from `contenteditable`, a `tabindex` other than -1, or a role of button, link, menuitem, tab, checkbox, radio, switch or option; `data-tauri-drag-region="false"` opts an element out explicitly. A `role="img"` does not block a drag, so the mark (`tu-titlebar__brand`: the icon, which keeps `role="img"` and its `aria-label` for accessibility, and the text THEOS//EDITOR; on macOS the bare text node would otherwise drag inside the `="deep"` left column) and every `tu-statuspip` in the title bar carry `data-tauri-drag-region="false"`, in every OS variant. Spec §5.3: the mark, menu labels, pip and controls are not draggable.
- **Windows and Linux:** `tu-titlebar--frameless` (no clipping so menus hang below it, no right padding so the controls sit flush). `tu-titlebar__drag` takes the free space, centres the breadcrumb, and is the drag region; a double-click on it maximizes or restores. Menu labels and window controls are not drag regions.
- **macOS:** `tu-titlebar--mac` alone (it needs no `--frameless`; spec §5.2 allows either): a three-column grid (left, breadcrumb, right) so the breadcrumb is centred on the window. The left column (`tu-titlebar__left`) starts with `tu-titlebar__inset`, `w-macos-inset` (72px) of room for the native traffic lights, then the mark (icon and THEOS//EDITOR); the right column (`tu-titlebar__right`; `--mac` gives it its 10px end padding, add none inline) holds the validator pip. All three columns are drag regions (`="deep"`: they hold only the inset spacer, the mark, the breadcrumb and the pip, nothing interactive), so the whole title bar drags except the mark (icon and THEOS//EDITOR, the `tu-titlebar__brand` element) and the pip, which carry `data-tauri-drag-region="false"` (spec §5.3: they are not draggable; inside a `="deep"` column they would drag otherwise). There is no in-window menu bar (the menus live in the system menu bar) and no window controls.
- The title bar is `box-sizing: border-box`, so with its 1px border it is exactly its grid row.
- Type: brand and menu labels are Instrument (`font-ui`); breadcrumb and pip are mono `textDim`; the separator `▸` is `textMute`; the layout name is `sig-attention`.
