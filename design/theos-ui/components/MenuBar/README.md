The in-window menu bar (Windows and Linux) and its dropdowns. Menu labels are Instrument, 10px uppercase, `textDim`, `text` when hovered or open. Items are sentence case at 12px.

**Use**
```html
<nav class="tu-menubar is-keys" role="menubar" aria-label="Main menu">
  <span class="tu-menubar__slot">
    <button class="tu-menubar__item is-open" role="menuitem" aria-haspopup="menu" aria-expanded="true"><span class="tu-menubar__key">F</span>ile</button>
    <div class="tu-menu" role="menu" aria-label="File">
      <div class="tu-menu__item is-active" role="menuitem"><span class="tu-menu__glyph"></span><span class="tu-menu__label">Open project…</span><span class="tu-menu__keys">Ctrl+O</span></div>
      <div class="tu-menu__sep" role="separator"></div>
      <div class="tu-menu__item is-disabled" role="menuitem" aria-disabled="true"><span class="tu-menu__glyph"></span><span class="tu-menu__label">Save<span class="tu-menu__phase">arrives with P12</span></span><span class="tu-menu__keys">Ctrl+S</span></div>
    </div>
  </span>
</nav>
```
- **States.** `tu-menubar__item`: `is-open` (`panel2` with a `borderHi` edge on three sides) and `is-focused` (amber outline, as the focus ring of every control). `tu-menubar.is-keys` underlines the access letters (`tu-menubar__key`) while the keyboard drives the bar. `tu-menu__item`: `is-active` (the highlighted row: 13% amber wash and a 2px amber edge, as `tu-pop__row`; its keys turn `text`), `is-current` (amber glyph, for the current layout or tab), `is-disabled` (`textMute`, not highlighted on hover).
- **Parts.** `tu-menu__glyph` (22px gutter), `tu-menu__label`, `tu-menu__phase` ("arrives with PNN", mono `textDim`, inside the label), `tu-menu__keys` (mono, right-aligned), `tu-menu__more` (the `▸` of a submenu), `tu-menu__sep`, `tu-menu__group` (small uppercase heading). A submenu is a `tu-menu tu-menu__sub` inside its parent row and opens to the right. `tu-menu--static` draws a menu flat in a page.
- **Sizes.** A dropdown is at least `w-menu-min` (248px) wide; a row is `h-menu-item` (24px). The menu is `panel2` with a `borderHi` edge, `shadow-popover`, radius 0.
- **Keyboard model.** Alt pressed and released alone, or F10, focuses the first menu (`is-focused`, `is-keys`). Left and Right move between menus; Down, Enter and Space open the focused menu; a letter opens the menu with that access letter; Esc closes the menu, then returns focus to where it was. Alt with an access letter opens that menu directly. In an open menu Up and Down move the highlighted row (skipping disabled ones), Right opens a submenu, Left closes it, Enter runs the row. Hovering another label while a menu is open switches to it. One menu is open at a time.
- **Content.** Rows come from the command table (labels, keys and enabled state are not typed into the design); a disabled row names its phase.
- Contrast: label, item and key text pass 4.5:1 in both themes; disabled items are `textMute` by design.
