The in-world voice: anything a people, faction, NPC or book author says, set in the book serif on a warm ground so it can never be mistaken for developer truth.

**Use**
```html
<figure class="tu-voice">
  <div class="tu-voice__head"><span class="tu-kind" data-kind="perspective"></span>In-world · Throat-Kin binders<span class="tu-voice__head-right">B-020 · T1 lens</span></div>
  <h3 class="tu-voice__title">The naming-song</h3>            <!-- optional: an in-world title -->
  <blockquote class="tu-voice__body"><p>…</p></blockquote>
  <figcaption class="tu-voice__by">a binder, teaching the naming-song</figcaption>
</figure>
```
- Type: Spectral 17/28 (`iw-body`) in `ink-inworld` on `panel-inworld`, max `measure-inworld` (62ch); title `iw-title`; attribution `iw-attribution` in `ink-inworld-dim`.
- The head always names who speaks (the perspective) and, on the right, the record and lens it belongs to.
- Radius `radius-manuscript` (2px), 1px `line-inworld` edge. No left bar, no quote-mark ornament.
- Parchment is not amber. Never use `ink-inworld` for UI, and never put a status or kind colour inside the block.
- Sample text: the lines in the preview are placeholder writing in the binders' voice, built on B-020; they are not canon.

**Why Spectral**: it was drawn for screens yet keeps the calligraphic stress of a book face, so in-world text reads as written by someone and stays legible at 17px on a dark ground, where Garamond-style faces thin out.
