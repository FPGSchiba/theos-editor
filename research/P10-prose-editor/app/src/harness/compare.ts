import { createTwoFilesPatch } from 'diff';
import { splitFrontMatter } from './frontmatter.ts';
import type { Comparison } from './types.ts';

const encoder = new TextEncoder();

const isHighSurrogate = (unit: number): boolean => unit >= 0xd800 && unit <= 0xdbff;
const isLowSurrogate = (unit: number): boolean => unit >= 0xdc00 && unit <= 0xdfff;

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

/**
 * The part of the two texts that differs: what is left after the common prefix and the common
 * suffix (which never overlap). Neither cut splits a surrogate pair.
 */
function differingRange(saved: string, expected: string): { prefix: number; suffix: number } {
  const shorter = Math.min(saved.length, expected.length);
  let prefix = 0;
  while (prefix < shorter && saved.charCodeAt(prefix) === expected.charCodeAt(prefix)) prefix++;
  if (prefix > 0 && isHighSurrogate(saved.charCodeAt(prefix - 1))) prefix--;

  const room = shorter - prefix;
  let suffix = 0;
  while (
    suffix < room &&
    saved.charCodeAt(saved.length - 1 - suffix) === expected.charCodeAt(expected.length - 1 - suffix)
  ) {
    suffix++;
  }
  if (suffix > 0 && isLowSurrogate(saved.charCodeAt(saved.length - suffix))) suffix--;
  return { prefix, suffix };
}

/** Line and column (1-based, UTF-16 units; only LF breaks a line) of an offset. */
function position(text: string, offset: number): { line: number; column: number } {
  let line = 1;
  let lineStart = 0;
  for (
    let newline = text.indexOf('\n');
    newline !== -1 && newline < offset;
    newline = text.indexOf('\n', newline + 1)
  ) {
    line++;
    lineStart = newline + 1;
  }
  return { line, column: offset - lineStart + 1 };
}

const isBlank = (line: string): boolean => /^[ \t\r]*$/.test(line);

/** The line that holds `pos`; `end` is the offset of its newline, or the text length. */
function lineAt(text: string, pos: number): { start: number; end: number } {
  const start = pos === 0 ? 0 : text.lastIndexOf('\n', pos - 1) + 1;
  const newline = text.indexOf('\n', pos);
  return { start, end: newline === -1 ? text.length : newline };
}

/**
 * The block around `offset`: the run of non-blank lines that holds it, from the start of its
 * first line through the newline of its last line. On a blank line the block is that line. The
 * front matter is never part of a block, also when the body follows the closing fence directly.
 */
function blockAround(text: string, offset: number): { start: number; end: number } {
  const bodyStart = splitFrontMatter(text).frontMatter.length;
  let { start, end } = lineAt(text, Math.min(Math.max(offset, 0), text.length));
  if (isBlank(text.slice(start, end))) return { start, end: Math.min(end + 1, text.length) };
  while (start > bodyStart) {
    const previous = lineAt(text, start - 1);
    if (isBlank(text.slice(previous.start, previous.end))) break;
    start = previous.start;
  }
  start = Math.max(start, bodyStart);
  while (end < text.length) {
    const next = lineAt(text, end + 1);
    if (isBlank(text.slice(next.start, next.end))) break;
    end = next.end;
  }
  return { start, end: Math.min(end + 1, text.length) };
}

const VISIBLE: Record<string, string> = { ' ': '␣', '\t': '⇥', '\r': '␍', '\u00a0': '⍽' };

/** Spaces, tabs, CR and U+00A0 shown as ␣ ⇥ ␍ ⍽; line feeds stay, so the diff keeps its lines. */
function showWhitespace(text: string): string {
  return text.replace(/[ \t\r\u00a0]/g, (char) => VISIBLE[char] ?? char);
}

/** A diff with more changed lines than this is too slow to compute and too long to read. */
const HUNK_MAX_EDIT_LENGTH = 2000;
/** Lines shown from each side when the diff is too big. */
const HUNK_FALLBACK_LINES = 10;

/**
 * A unified diff (expected to saved) with whitespace made visible. When the texts differ in more
 * than HUNK_MAX_EDIT_LENGTH lines (a serializer that rewrote the whole file), the diff is not
 * computed; the hunk then shows the first lines of each side from the first difference.
 */
function visibleHunk(saved: string, expected: string, firstDiffLine: number): string {
  const visibleSaved = showWhitespace(saved);
  const visibleExpected = showWhitespace(expected);
  const patch = createTwoFilesPatch('expected', 'saved', visibleExpected, visibleSaved, undefined, undefined, {
    context: 3,
    maxEditLength: HUNK_MAX_EDIT_LENGTH,
  });
  if (patch !== undefined) return patch.replace(/^=+\n/, '');

  const head = (text: string, sign: string): string =>
    text
      .split('\n')
      .slice(firstDiffLine - 1, firstDiffLine - 1 + HUNK_FALLBACK_LINES)
      .map((line) => sign + line)
      .join('\n');
  return [
    '--- expected',
    '+++ saved',
    `@@ -${firstDiffLine} +${firstDiffLine} @@ too many changed lines for a diff; ` +
      `first ${HUNK_FALLBACK_LINES} lines of each side`,
    head(visibleExpected, '-'),
    head(visibleSaved, '+'),
    '',
  ].join('\n');
}

/**
 * Compare the saved text with the expected one as UTF-8 bytes.
 *
 * On a difference: `firstDiff` is where the texts first differ, `extraBytes` the UTF-8 size of
 * the larger differing middle (after the common prefix and suffix), `hunk` a unified diff
 * (expected to saved) with whitespace made visible, and, given `editOffset` (the offset in the
 * original where the edit starts, which is also an offset in `expected`), `withinBlock` says
 * whether the differing range lies inside the block of non-blank lines around that offset in
 * `expected`.
 */
export function compareBytes(saved: string, expected: string, editOffset?: number): Comparison {
  const savedBytes = encoder.encode(saved);
  const expectedBytes = encoder.encode(expected);
  if (sameBytes(savedBytes, expectedBytes)) return { identical: true, extraBytes: 0 };

  const { prefix, suffix } = differingRange(saved, expected);
  const savedMiddle = saved.slice(prefix, saved.length - suffix);
  const expectedMiddle = expected.slice(prefix, expected.length - suffix);
  const firstDiff = position(saved, prefix);
  const result: Comparison = {
    identical: false,
    firstDiff,
    extraBytes: Math.max(encoder.encode(savedMiddle).length, encoder.encode(expectedMiddle).length),
    hunk: visibleHunk(saved, expected, firstDiff.line),
  };
  if (editOffset !== undefined) {
    const block = blockAround(expected, editOffset);
    result.withinBlock = prefix >= block.start && expected.length - suffix <= block.end;
  }
  return result;
}

/**
 * `compareBytes` for an edit that changes the text in more than one place (`external`,
 * `undo-all`): `withinBlock` is true when the damage stays inside the block around any one of
 * `sites` (offsets into `expected`, see `expectedSites`). Without sites it is `compareBytes`
 * without an offset.
 */
export function compareAtSites(saved: string, expected: string, sites: number[]): Comparison {
  const [first, ...rest] = sites;
  const result = compareBytes(saved, expected, first);
  if (result.identical || result.withinBlock === true) return result;
  for (const site of rest) {
    if (compareBytes(saved, expected, site).withinBlock === true) return { ...result, withinBlock: true };
  }
  return result;
}
