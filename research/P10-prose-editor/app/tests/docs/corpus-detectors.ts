import { splitFrontMatter } from '../../src/harness/frontmatter.ts';
import { toLf } from '../../src/harness/oracle.ts';

/**
 * One detector per construct tag of the corpus (spec §4.2). A manifest file may only claim a tag
 * when the tag's detector confirms it on the file's text, so a claim cannot drift from the bytes.
 * Prose detectors look at the body only (the comment line in the front matter is not a heading)
 * and skip fenced code blocks (except the one that detects the fence itself).
 */

/** A file's text cut into the pieces the detectors need. */
export interface Parts {
  raw: string;
  /** LF form of the file. */
  lf: string;
  /** The lines between the two fences of the front matter. */
  fm: string[];
  /** Every line of the body (the last element is empty when the body ends with a newline). */
  body: string[];
  /** The body without fenced code blocks, fences included. */
  prose: string[];
}

export interface DetectorContext {
  /** The ids of all corpus files, to tell a dangling link from one that has a file. */
  ids: ReadonlySet<string>;
}

export type Detector = (parts: Parts, context: DetectorContext) => boolean;

/** The id that is a dangling link on purpose: the reference screens show "Song-cellar" as broken. */
export const DANGLING_ID = 'song-cellar';

export function partsOf(raw: string): Parts {
  const lf = toLf(raw);
  const split = splitFrontMatter(lf);
  const fm =
    split.frontMatter === '' ? [] : split.frontMatter.replace(/\n$/, '').split('\n').slice(1, -1);
  const body = split.body.split('\n');
  return { raw, lf, fm, body, prose: withoutFences(body) };
}

function withoutFences(lines: string[]): string[] {
  const kept: string[] = [];
  let inFence = false;
  for (const line of lines) {
    if (/^```/.test(line)) {
      inFence = !inFence;
      continue;
    }
    if (!inFence) kept.push(line);
  }
  return kept;
}

const line = (lines: string[], index: number): string => lines[index] ?? '';
const some = (lines: string[], pattern: RegExp): boolean => lines.some((l) => pattern.test(l));

export interface Callout {
  type: string;
  /** From the `> [!type]` line to the line before the next blank line (lazy lines included). */
  lines: string[];
}

export function calloutsOf(lines: string[]): Callout[] {
  const found: Callout[] = [];
  for (let i = 0; i < lines.length; i++) {
    const match = /^> \[!(\w+)\]/.exec(line(lines, i));
    if (match === null) continue;
    const block = [line(lines, i)];
    while (i + 1 < lines.length && line(lines, i + 1) !== '') block.push(line(lines, ++i));
    found.push({ type: match[1] ?? '', lines: block });
  }
  return found;
}

export interface Beat {
  heading: string;
  attributes: Array<{ key: string; gap: string; value: string }>;
  /** The lines after the attribute lines, up to the end of the text. */
  after: string[];
}

export function beatsOf(lines: string[]): Beat[] {
  const found: Beat[] = [];
  for (let i = 0; i < lines.length; i++) {
    if (!/^## Beat \d+/.test(line(lines, i))) continue;
    const attributes: Beat['attributes'] = [];
    let next = i + 1;
    for (;;) {
      const match = /^([a-z]+)::( *)(.*)$/.exec(line(lines, next));
      if (match === null) break;
      attributes.push({ key: match[1] ?? '', gap: match[2] ?? '', value: match[3] ?? '' });
      next++;
    }
    found.push({ heading: line(lines, i), attributes, after: lines.slice(next) });
  }
  return found;
}

const BEAT_KEYS = ['at', 'with', 'says', 'reveals', 'when'];
const KEY = '[a-z][a-z0-9_]*';
const MARKER_OR_BLOCK_START = /^(?:>|#|[-*+] |\d+[.)] |\||<|```|---|\*\*\*|===|\s|$)/;

/** A line of ordinary paragraph text: no block marker, no beat attribute, no hard break. */
function isPlainLine(text: string): boolean {
  return (
    text !== '' &&
    !MARKER_OR_BLOCK_START.test(text) &&
    !/^[a-z]+::/.test(text) &&
    !/(?: {2}|\\)$/.test(text) &&
    !text.startsWith('Design note:')
  );
}

/** Prose lines with list markers and escaped characters taken out, for the emphasis detectors. */
function emphasisText(parts: Parts): string[] {
  return parts.prose.map((l) => l.replace(/^\s*(?:[-*+]|\d+[.)]) /, '').replace(/\\./g, ''));
}

const isCalloutOrAttribute = (text: string): boolean => text.startsWith('>') || /^[a-z]+::/.test(text);
const voiceBlocks = (parts: Parts): Callout[] =>
  calloutsOf(parts.prose).filter((callout) => callout.type === 'voice');
const lastLine = (callout: Callout): string => line(callout.lines, callout.lines.length - 1);
const bodyText = (parts: Parts): string => parts.body.join('\n');

export const DETECTORS: Record<string, Detector> = {
  // Front matter
  'fm-comment-line': (p) => some(p.fm, /^\s*#/),
  'fm-comment-trailing': (p) => some(p.fm, new RegExp(`^\\s*${KEY}: +[^\\s#'"\\[].*? #`)),
  'fm-scalar-plain': (p) => some(p.fm, new RegExp(`^\\s*${KEY}: +[^\\s#'"\\[]`)),
  'fm-scalar-single-quoted': (p) => some(p.fm, new RegExp(`^\\s*${KEY}: +'[^']*'`)),
  'fm-scalar-double-quoted': (p) => some(p.fm, new RegExp(`^\\s*${KEY}: +"[^"]*"`)),
  'fm-list-inline': (p) => some(p.fm, new RegExp(`^\\s*${KEY}: +\\[[^\\]]*\\]`)),
  'fm-list-block': (p) =>
    p.fm.some((l, i) => new RegExp(`^${KEY}:\\s*$`).test(l) && /^\s*- \S/.test(line(p.fm, i + 1))),
  'fm-map': (p) =>
    p.fm.some(
      (l, i) =>
        new RegExp(`^${KEY}:\\s*$`).test(l) && new RegExp(`^ {2,}${KEY}: +\\S`).test(line(p.fm, i + 1)),
    ),
  'fm-blank-line': (p) => p.fm.some((l) => l.trim() === ''),

  // Prose
  'hard-wrapped-paragraph': (p) => {
    let run = 0;
    for (const text of [...p.prose, '']) {
      if (isPlainLine(text)) run++;
      else if (run >= 3) return true;
      else run = 0;
    }
    return false;
  },
  'atx-heading-1': (p) => some(p.prose, /^# \S/),
  'atx-heading-2': (p) => some(p.prose, /^## \S/),
  'atx-heading-3': (p) => some(p.prose, /^### \S/),
  'setext-heading': (p) =>
    p.prose.some(
      (l, i) =>
        /^(?:=+|-+)\s*$/.test(l) &&
        isPlainLine(line(p.prose, i - 1)) &&
        (i === 1 || line(p.prose, i - 2) === ''),
    ),
  'emphasis-star': (p) => some(emphasisText(p).map((l) => l.replace(/\*\*[^*]+\*\*/g, '')), /\*[^*\s][^*]*\*/),
  'emphasis-underscore': (p) =>
    some(
      emphasisText(p).map((l) => l.replace(/__[^_]+__/g, '')),
      /(?:^|[\s(])_[^_\s][^_]*_(?=$|[\s.,;:!?)])/,
    ),
  'strong-star': (p) => some(emphasisText(p), /\*\*[^*\s][^*]*\*\*/),
  'strong-underscore': (p) => some(emphasisText(p), /__[^_\s][^_]*__/),
  'inline-code': (p) => some(p.prose, /`[^`]+`/),
  'code-fence': (p) => p.body.filter((l) => /^```/.test(l)).length >= 2,
  'list-dash': (p) => some(p.prose, /^\s*- \S/),
  'list-star': (p) => some(p.prose, /^\s*\* \S/),
  'list-plus': (p) => some(p.prose, /^\s*\+ \S/),
  'list-ordered-dot': (p) => some(p.prose, /^\s*\d+\. \S/),
  'list-ordered-paren': (p) => some(p.prose, /^\s*\d+\) \S/),
  'list-nested': (p) =>
    p.prose.some(
      (l, i) =>
        /^(?: {2,}|\t)(?:[-*+]|\d+[.)]) \S/.test(l) &&
        p.prose.slice(0, i).some((earlier) => /^(?:[-*+]|\d+[.)]) \S/.test(earlier)),
    ),
  'list-ones': (p) =>
    p.prose.some(
      (_, i) => [0, 1, 2].every((offset) => /^1\. \S/.test(line(p.prose, i + offset))),
    ),
  'block-quote': (p) =>
    p.prose.some(
      (l, i) => l.startsWith('>') && !/^>\s*\[!/.test(l) && !line(p.prose, i - 1).startsWith('>'),
    ),
  'gfm-table': (p) =>
    p.prose.some(
      (l, i) =>
        /^\|.+\|\s*$/.test(l) && /^\|(?:\s*:?-{3,}:?\s*\|)+\s*$/.test(line(p.prose, i + 1)),
    ),
  'hr-dashes': (p) => p.prose.some((l, i) => /^---\s*$/.test(l) && (i === 0 || line(p.prose, i - 1) === '')),
  'hr-stars': (p) => p.prose.some((l, i) => /^\*\*\*\s*$/.test(l) && (i === 0 || line(p.prose, i - 1) === '')),
  'html-comment': (p) => /<!--[\s\S]*?-->/.test(p.prose.join('\n')),
  'hard-break-spaces': (p) => p.prose.some((l, i) => /\S {2,}$/.test(l) && line(p.prose, i + 1) !== ''),
  'hard-break-backslash': (p) =>
    p.prose.some((l, i) => /(?:^|[^\\])\\$/.test(l) && line(p.prose, i + 1) !== ''),
  'backslash-escape': (p) => some(p.prose, /\\[*_[\]`#]/),
  autolink: (p) => some(p.prose, /<https?:\/\/[^>\s]+>/),
  'md-link': (p) => some(p.prose, /\[[^\]]+\]\([^)\s]+\)/),

  // Links
  'link-id': (p) => some(p.prose, /\[\[[^\]|]+\]\]/),
  'link-alias': (p) => some(p.prose, /\[\[[^\]|]+\|[^\]]+\]\]/),
  'link-dangling': (p, { ids }) =>
    some(p.prose, new RegExp(`\\[\\[${DANGLING_ID}(?:\\|[^\\]]*)?\\]\\]`)) && !ids.has(DANGLING_ID),
  'link-punctuation': (p) =>
    some(p.prose, /[(\u201c"]\[\[[^\]]+\]\]|\[\[[^\]]+\]\][.,;:!?)\u201d"]/),
  'link-line-start': (p) => p.prose.some((l) => l.startsWith('[[')),
  'link-line-end': (p) => p.prose.some((l) => !isCalloutOrAttribute(l) && l.endsWith(']]')),
  'link-in-emphasis': (p) => some(p.prose, /(?:\*{1,2}|_{1,2})\[\[[^\]]+\]\](?:\*{1,2}|_{1,2})/),
  'link-in-callout-title': (p) => some(p.prose, /^> \[!\w+\].*\[\[[^\]]+\]\]/),
  'link-in-beat-attribute': (p) => some(p.prose, /^[a-z]+::\s*.*\[\[[^\]]+\]\]/),
  'link-prose-mother-ilse': (p) =>
    p.prose.some((l) => !isCalloutOrAttribute(l) && l.includes('[[mother-ilse]]')),

  // Callouts
  'voice-speaker-only': (p) => some(p.prose, /^> \[!voice\] \[\[[^\]|]+\]\]\s*$/),
  'voice-beliefs-tier': (p) =>
    some(p.prose, /^> \[!voice\] \[\[[^\]|]+\]\] · \[\[[^\]|]+\]\] · T[123]\s*$/),
  'voice-caption': (p) => voiceBlocks(p).some((c) => /^> — \S/.test(lastLine(c))),
  'voice-no-caption': (p) => voiceBlocks(p).some((c) => !/^> — \S/.test(lastLine(c))),
  'voice-caption-double-hyphen': (p) =>
    voiceBlocks(p).some((c) => {
      const first = c.lines.findIndex((l) => l.startsWith('> -- '));
      return first !== -1 && c.lines.slice(first + 1).some((l) => l.startsWith('> — '));
    }),
  'voice-ledger-word': (p) => voiceBlocks(p).some((c) => /\bledger\b/.test(c.lines.slice(1).join(' '))),
  'callout-empty-line': (p) => calloutsOf(p.prose).some((c) => some(c.lines.slice(1), /^>\s*$/)),
  'callout-lazy-line': (p) =>
    calloutsOf(p.prose).some((c) => c.lines.slice(1).some((l) => !l.startsWith('>'))),
  'callout-design': (p) => some(p.prose, /^> \[!design\]/),
  'design-note-line': (p) => some(p.prose, /^Design note:/),
  'note-open': (p) => some(p.prose, /^> \[!note\] n\d+(?: · \[\[[^\]]+\]\])? · open\s*$/),
  'note-resolved': (p) =>
    some(p.prose, /^> \[!note\] n\d+(?: · \[\[[^\]]+\]\])? · resolved by \[\[[^\]]+\]\]\s*$/),
  'note-dropped': (p) => some(p.prose, /^> \[!note\] n\d+(?: · \[\[[^\]]+\]\])? · dropped\s*$/),

  // Beats
  'beat-all-attributes': (p) =>
    beatsOf(p.prose).some((beat) =>
      BEAT_KEYS.every((key) => beat.attributes.some((attribute) => attribute.key === key)),
    ),
  'beat-no-attributes': (p) => beatsOf(p.prose).some((beat) => beat.attributes.length === 0),
  'beat-voice-adjacent': (p) =>
    beatsOf(p.prose).some((beat) => {
      if (beat.attributes.length === 0) return false;
      const next = line(beat.after, 0) === '' ? line(beat.after, 1) : line(beat.after, 0);
      return next.startsWith('> [!voice]');
    }),
  'beat-extra-spaces': (p) =>
    beatsOf(p.prose).some((beat) => beat.attributes.some((attribute) => attribute.gap.length >= 2)),
  'beat-unknown-attribute': (p) =>
    beatsOf(p.prose).some((beat) => beat.attributes.some((attribute) => !BEAT_KEYS.includes(attribute.key))),
  'beat-heading-no-dash': (p) => beatsOf(p.prose).some((beat) => !/^## Beat \d+ — /.test(beat.heading)),
  'beat-at-line': (p) =>
    beatsOf(p.prose).some((beat) => beat.attributes.some((attribute) => attribute.key === 'at')),

  // Characters and whitespace
  'char-em-dash': (p) =>
    p.prose.some((l) => !l.startsWith('>') && !l.startsWith('#') && l.includes('—')),
  'char-middle-dot': (p) => p.prose.some((l) => !l.startsWith('>') && l.includes('·')),
  'char-curly-quotes': (p) =>
    ['\u201c', '\u201d', '\u2018', '\u2019'].every((mark) => bodyText(p).includes(mark)),
  'e-precomposed': (p) => bodyText(p).includes('\u00e9'),
  'e-combining': (p) => bodyText(p).includes('e\u0301'),
  nbsp: (p) => bodyText(p).includes('\u00a0'),
  emoji: (p) => /[\ud800-\udbff][\udc00-\udfff]/.test(bodyText(p)),
  'tab-in-prose': (p) => some(p.prose, /[^\t]\t/),
  'trailing-spaces': (p) => some(p.body, /\S $/) && some(p.body, /\S {3,}$/),
  'blank-lines-two': (p) => /[^\n]\n\n\n[^\n]/.test(bodyText(p)),
  'blank-lines-three': (p) => /[^\n]\n\n\n\n[^\n]/.test(bodyText(p)),
  'no-final-newline': (p) => p.raw !== '' && !p.raw.endsWith('\n'),

  // Whole files
  'empty-body': (p) => bodyText(p) === '',
  crlf: (p) => p.raw.includes('\r\n'),
};
