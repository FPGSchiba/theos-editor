import type { Node } from 'prosemirror-model';
import { PositionMap } from '../prosemirror/positions.ts';
import { childrenOf, positionOf, runAround } from './blocks.ts';
import type { Change } from './changes.ts';
import { isEmptyParagraph } from './blocks.ts';
import { docOf, type ChangedBlocks, type SourceEdit, type SourceState } from './edit.ts';
import { continuation, insertsFor, keepFinalNewline, leadSeparator, trailSeparator, type Insert } from './spans.ts';
import { Verifier } from './verify.ts';

/** Which end of a change a position is: where to look first among its candidate offsets. */
type Side = 'start' | 'end' | 'insert';

const unique = (offsets: number[]): number[] => [...new Set(offsets)];

/**
 * The change as a span edit of the source: its start and end mapped onto source offsets, the
 * content it inserts written as source, and the run of blocks around it parsed again to prove that
 * the result stands for the new document. Undefined when no candidate does, or when a position
 * cannot be mapped (a boundary inside a callout or a list, a run whose map is not exact).
 */
export function spanEdit(state: SourceState, after: Node, change: Change, blocks: ChangedBlocks): SourceEdit | undefined {
  const { file, bodyStart, doc: before } = state;
  const n = before.childCount;
  const m = after.childCount;

  // The top-level children a position touches: its own, or both neighbours of a boundary.
  const touched = (pos: number): [number, number] => {
    const $pos = before.resolve(pos);
    if ($pos.depth > 0) return [$pos.index(0), $pos.index(0)];
    const index = $pos.index(0);
    return [Math.max(index - 1, 0), Math.min(index, n - 1)];
  };
  const [first, last] = runAround(
    file,
    state.blocks,
    Math.min(touched(change.start)[0], blocks.changed),
    Math.max(touched(change.endBefore)[1] + 1, blocks.endBefore),
  );
  const runFrom = state.blocks[first]!.from;
  const runTo = state.blocks[last - 1]!.to;
  const runPos = positionOf(before, first);
  let map: PositionMap | undefined;

  const offsetsAt = (pos: number, side: Side): number[] | undefined => {
    const $pos = before.resolve(pos);
    if ($pos.depth === 0) {
      const index = $pos.index(0);
      const all = state.blocks;
      if (index === 0) return unique(side === 'insert' ? [all[0]!.from, bodyStart] : [bodyStart, all[0]!.from]);
      if (index === n) return unique([file.length, all[n - 1]!.to]);
      return unique([all[index]!.from, all[index - 1]!.to]);
    }
    if (!$pos.parent.isTextblock) return undefined;
    // An empty top-level textblock without a token (an empty paragraph, an empty body) is written at the start of its line.
    const own = state.blocks[$pos.index(0)]!;
    if ($pos.depth === 1 && $pos.parent.content.size === 0 && (own.from === own.to || isEmptyParagraph($pos.parent))) return [own.from];
    map ??= new PositionMap(file.slice(runFrom, runTo), 0, docOf(childrenOf(before, first, last)));
    const local = map.offsets(pos - runPos);
    if (local === undefined) return undefined;
    const found = local.map((offset) => offset + runFrom);
    // The start of a removed range as late as possible, its end as early as possible: narrow first.
    return side === 'start' ? found.reverse() : found;
  };

  const insertion = change.start === change.endBefore;
  const fromOptions = offsetsAt(change.start, insertion ? 'insert' : 'start');
  const toOptions = insertion ? fromOptions : offsetsAt(change.endBefore, 'end');
  if (fromOptions === undefined || toOptions === undefined) return undefined;
  // The inserted content is written with the prefix a new line takes where it goes in.
  const forms = new Map<string, Insert[] | undefined>();
  const insertsAt = (from: number): Insert[] | undefined => {
    const prefix = continuation(file, from);
    if (!forms.has(prefix)) forms.set(prefix, insertsFor(after, change.start, change.endAfter, prefix));
    return forms.get(prefix);
  };
  const pairs: Array<[number, number]> = insertion
    ? fromOptions.map((offset) => [offset, offset])
    : fromOptions.flatMap((from) => toOptions.filter((to) => to >= from).map((to): [number, number] => [from, to]));

  // Text typed into an empty top-level textblock that has no line (a beat without attribute lines) needs a line of its own.
  const $start = before.resolve(change.start);
  const home = state.blocks[$start.index(0)];
  const intoEmpty = insertion && $start.depth === 1 && $start.parent.content.size === 0 && home !== undefined && home.from === home.to;
  const expected = childrenOf(after, first, last + m - n);
  const verifier = new Verifier();
  for (const strictness of ['strict', 'loose'] as const) {
    for (const [from, to] of pairs) {
      const unitFrom = Math.min(runFrom, from);
      const unitTo = Math.max(runTo, to);
      for (const insert of insertsAt(from) ?? []) {
        const text = (insert.opensBlock ? leadSeparator(file, bodyStart, from) : '') + insert.text + (insert.closesBlock ? trailSeparator(file, to) : '');
        const lines = intoEmpty && text !== '' ? (file[to] === '\n' ? [`${text}\n`, text] : [text, `${text}\n`]) : [text];
        for (const variant of new Set(lines.map((line) => keepFinalNewline(file, bodyStart, from, to, line)))) {
          const unit = file.slice(unitFrom, from) + variant + file.slice(to, unitTo);
          if (verifier.check(unit, expected, strictness)) {
            return { from, to, insert: variant, first, last, runFrom: unitFrom, text: unit, serialized: false, blocks };
          }
        }
      }
    }
  }
  return undefined;
}
