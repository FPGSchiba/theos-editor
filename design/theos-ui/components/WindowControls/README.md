The window controls of the frameless window: minimize, maximize (restore when maximized) and close, at the right end of the title bar.

**Use**
```html
<div class="tu-winctl" role="group" aria-label="Window">
  <button class="tu-winctl__btn" type="button" aria-label="Minimise">−</button>
  <button class="tu-winctl__btn" type="button" aria-label="Maximise">□</button>   <!-- ▫ and "Restore" when maximized -->
  <button class="tu-winctl__btn is-close" type="button" aria-label="Close">✕</button>
</div>
```
- Each control is `w-winctl` (46px) wide, flat, no radius; glyphs are mono `textDim` (− □ ▫ ✕, all in JetBrains Mono). Hover fills `panel2` with `text`; only close takes a signal on hover (`sig-reject` fill, `onFill` glyph). The group is separated from the pip by one `border` line.
- **Resize edge.** The top `h-resize-edge` (4px) of the window is the resize strip. `tu-winctl` is padded down by it, so a control's hit area (and hover fill) is 46 × 26, from 4px to 30px. A click in the strip resizes the window. When the title bar is `is-maximized` the padding drops and the controls are 46 × 30. The group's bottom margin of -1px lets the controls cover the title bar's 1px border line.
- **Snap layouts (Windows 11).** The maximize button gets the native snap-layout flyout from a transparent Win32 child window laid over it, so the button's size and position are fixed: the second control from the right edge, 46 × 26 restored and 46 × 30 maximized. The controls never change width or order, and nothing to their right moves. On Linux the button is an ordinary maximize. macOS has no window controls here (native traffic lights).
- Keep the glyphs: the old maximize glyph U+25A2 is not in JetBrains Mono and falls back to a system font.
