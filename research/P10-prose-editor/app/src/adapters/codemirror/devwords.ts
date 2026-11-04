import type { Range } from '@codemirror/state';
import { Decoration } from '@codemirror/view';

/**
 * Developer words in voice text (the in-world vocabulary check): the words of the host's list are
 * marked, dotted underline in the harness CSS, and nothing else is touched. Only the callout code
 * calls this, for the text lines of voice blocks; prose, design notes, titles and notes are not voice text.
 */

const mark = Decoration.mark({ attributes: { 'data-dev-word': '', title: 'developer word' } });

const escapeRegExp = (text: string): string => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

let cached: { key: string; pattern: RegExp | undefined } | undefined;

/** One pattern for the whole list: longest words first, and a match is a whole word (not `tier` in `frontier`). */
function patternFor(words: readonly string[]): RegExp | undefined {
  const key = words.join('\n');
  if (cached?.key !== key) {
    const listed = words.filter((word) => word !== '').sort((a, b) => b.length - a.length).map(escapeRegExp);
    cached = {
      key,
      pattern: listed.length === 0 ? undefined : new RegExp(`(?<![\\p{L}\\p{N}_])(?:${listed.join('|')})(?![\\p{L}\\p{N}_])`, 'giu'),
    };
  }
  return cached.pattern;
}

/** Mark the developer words of `text`, which starts at document offset `start`. */
export function markDevWords(found: Array<Range<Decoration>>, text: string, start: number, words: readonly string[]): void {
  const pattern = patternFor(words);
  if (pattern === undefined) return;
  for (const match of text.matchAll(pattern)) {
    found.push(mark.range(start + match.index, start + match.index + match[0].length));
  }
}
