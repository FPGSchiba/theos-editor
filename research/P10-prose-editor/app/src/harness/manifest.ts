import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { ScriptedEdit } from './types.ts';

/** One file of the corpus (spec §4.2) with the constructs it covers and the edits run on it. */
export interface CorpusFile {
  /** Relative to `corpus/`, e.g. `records/story/scripts/S-004.md`. */
  path: string;
  origin: 'data-model §14' | 'synthetic';
  lineEnds: 'lf' | 'crlf';
  /** Tags of CONSTRUCTS; the corpus test confirms each claim with a detector on the file's text. */
  covers: string[];
  edits: ScriptedEdit[];
}

export interface Manifest {
  files: CorpusFile[];
}

/** `research/P10-prose-editor/corpus/`, with a trailing separator. */
export const CORPUS_DIR = fileURLToPath(new URL('../../../corpus/', import.meta.url));

/**
 * The constructs of spec §4.2 as tags, plus the edge cases of spec §8 (Review Focus) and the
 * three feature fixtures the later suites need (a developer word in a voice block,
 * `[[mother-ilse]]` in prose, a beat with an `at::` line).
 * Every tag is claimed by at least one file of the manifest and confirmed by a detector in
 * `tests/docs/corpus-detectors.ts`.
 */
export const CONSTRUCTS: readonly string[] = [
  // Front matter
  'fm-comment-line',
  'fm-comment-trailing',
  'fm-scalar-plain',
  'fm-scalar-single-quoted',
  'fm-scalar-double-quoted',
  'fm-list-inline',
  'fm-list-block',
  'fm-map',
  'fm-blank-line',
  // Prose
  'hard-wrapped-paragraph',
  'atx-heading-1',
  'atx-heading-2',
  'atx-heading-3',
  'setext-heading',
  'emphasis-star',
  'emphasis-underscore',
  'strong-star',
  'strong-underscore',
  'inline-code',
  'code-fence',
  'list-dash',
  'list-star',
  'list-plus',
  'list-ordered-dot',
  'list-ordered-paren',
  'list-nested',
  'list-ones',
  'block-quote',
  'gfm-table',
  'hr-dashes',
  'hr-stars',
  'html-comment',
  'hard-break-spaces',
  'hard-break-backslash',
  'backslash-escape',
  'autolink',
  'md-link',
  // Links
  'link-id',
  'link-alias',
  'link-dangling',
  'link-punctuation',
  'link-line-start',
  'link-line-end',
  'link-in-emphasis',
  'link-in-callout-title',
  'link-in-beat-attribute',
  'link-prose-mother-ilse',
  // Callouts
  'voice-speaker-only',
  'voice-beliefs-tier',
  'voice-caption',
  'voice-no-caption',
  'voice-caption-double-hyphen',
  'voice-ledger-word',
  'callout-empty-line',
  'callout-lazy-line',
  'callout-design',
  'design-note-line',
  'note-open',
  'note-resolved',
  'note-dropped',
  // Beats
  'beat-all-attributes',
  'beat-no-attributes',
  'beat-voice-adjacent',
  'beat-extra-spaces',
  'beat-unknown-attribute',
  'beat-heading-no-dash',
  'beat-at-line',
  // Characters and whitespace
  'char-em-dash',
  'char-middle-dot',
  'char-curly-quotes',
  'e-precomposed',
  'e-combining',
  'nbsp',
  'emoji',
  'tab-in-prose',
  'trailing-spaces',
  'blank-lines-two',
  'blank-lines-three',
  'no-final-newline',
  // Whole files
  'empty-body',
  'crlf',
];

/** Read and check the shape of `corpus/manifest.json`. */
export function readManifest(): Manifest {
  const parsed: unknown = JSON.parse(readFileSync(join(CORPUS_DIR, 'manifest.json'), 'utf8'));
  const files = (parsed as { files?: unknown } | null)?.files;
  if (!Array.isArray(files)) throw new Error('corpus/manifest.json: "files" must be an array');
  for (const file of files as Array<Partial<CorpusFile>>) {
    if (typeof file.path !== 'string' || !Array.isArray(file.covers) || !Array.isArray(file.edits)) {
      throw new Error(`corpus/manifest.json: a file entry needs path, covers and edits (${String(file.path)})`);
    }
    if (file.lineEnds !== 'lf' && file.lineEnds !== 'crlf') {
      throw new Error(`corpus/manifest.json: ${file.path} has no valid lineEnds`);
    }
    if (file.origin !== 'data-model §14' && file.origin !== 'synthetic') {
      throw new Error(`corpus/manifest.json: ${file.path} has no valid origin`);
    }
  }
  return { files: files as CorpusFile[] };
}

/** The file's bytes as UTF-8 text, unchanged: line ends, a missing final newline and all. */
export function readCorpusText(file: CorpusFile): string {
  return readFileSync(join(CORPUS_DIR, file.path), 'utf8');
}
