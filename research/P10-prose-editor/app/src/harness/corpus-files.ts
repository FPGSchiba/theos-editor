/**
 * The corpus records as the browser sees them: path relative to `corpus/` (the manifest path,
 * e.g. `records/story/scripts/S-004.md`) and the text exactly as Vite reads the file.
 */
const raw = import.meta.glob('../../../corpus/records/**/*.md', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

export interface CorpusEntry {
  path: string;
  text: string;
}

export const corpusEntries: CorpusEntry[] = Object.entries(raw)
  .map(([key, text]) => ({ path: key.replace(/^(?:\.\.\/)+corpus\//, ''), text }))
  .sort((a, b) => a.path.localeCompare(b.path));
