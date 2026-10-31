The 54px vertical rail that switches named layout presets; each item is a glyph over an 8px mono label, with an optional count badge.

**Use**
```html
<nav class="tu-rail">
  <div class="tu-rail__item is-active"><span class="tu-rail__glyph">◴</span><span class="tu-rail__label">Review</span><span class="tu-rail__badge">4</span></div>
  …
  <div class="tu-rail__spacer"></div>
  <div class="tu-rail__item">…Settings…</div>
</nav>
```
- A rail item activates a whole layout preset (the dock tree), never a single window.
- The active preset is amber: 2px left edge, 6% amber wash, full-opacity label.
- The badge is amber for "items wait for you" and red (`is-reject`) for "something is broken". Never magenta: an AI count is still a count of things you must review.
- Settings sits alone at the bottom, after `tu-rail__spacer`.
- Every tool uses this rail, the Magic sandbox included (its presets differ, the component does not).
- Source glyphs `◴ ✱ ⚙` are not in JetBrains Mono and fall back to the OS symbol font; see Iconography.
