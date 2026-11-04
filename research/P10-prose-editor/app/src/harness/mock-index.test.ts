import { describe, expect, it } from 'vitest';
import { developerWords, findingsFor } from './findings.ts';
import { keymap } from './keymap.ts';
import { corpusInfos, createMockContext, createMockIndex } from './mock-index.ts';
import { readCorpusText, readManifest } from './manifest.ts';
import { allowedKinds } from './places.ts';

/** Every id the corpus links to: `[[id]]` and `[[id|alias]]`, in every record. */
function linkedIds(): string[] {
  const ids = new Set<string>();
  for (const file of readManifest().files) {
    for (const match of readCorpusText(file).matchAll(/\[\[([^\]|]+)(?:\|[^\]]*)?\]\]/g)) ids.add(match[1]!);
  }
  return [...ids].sort();
}

describe('the mock index', () => {
  it('holds every corpus record with its id, kind and title', () => {
    const infos = corpusInfos();
    expect(infos).toHaveLength(readManifest().files.length);
    const index = createMockIndex();
    expect(index.resolve('mother-ilse')).toEqual({ id: 'mother-ilse', kind: 'npc', title: 'Mother Ilse' });
    expect(index.resolve('B-901')?.title).toBe('Synthetic belief: a double-quoted title');
    expect(index.resolve('T-901')?.title).toBe('Synthetic truth: lists, a quote, a table and breaks');
    expect(index.resolve('f04-terrain-v1')?.kind).toBe('map-version');
  });

  it('resolves every linked id except the corpus dangling link song-cellar', () => {
    const index = createMockIndex();
    const ids = linkedIds();
    expect(ids).toContain('song-cellar');
    const unresolved = ids.filter((id) => index.resolve(id) === undefined);
    expect(unresolved).toEqual(['song-cellar']);
  });

  it('answers the query "Listen" with T-012 among the matches', () => {
    const hits = createMockIndex().search('Listen', allowedKinds({ in: 'prose' }));
    expect(hits.map((hit) => hit.id)).toEqual(expect.arrayContaining(['T-012', 'B-020', 'C-033']));
  });

  it('patches records in place', () => {
    const index = createMockIndex();
    index.patch([{ id: 'mother-ilse', kind: 'npc', title: 'Ilse the keeper' }]);
    expect(index.resolve('mother-ilse')?.title).toBe('Ilse the keeper');
    expect(index.resolve('B-020')).toBeDefined();
  });

  it('returns copies, so a caller cannot change the index', () => {
    const index = createMockIndex();
    index.search('ilse', allowedKinds({ in: 'prose' }))[0]!.title = 'changed';
    expect(index.resolve('mother-ilse')?.title).toBe('Mother Ilse');
  });
});

describe('places', () => {
  it('allows per data model §9.4', () => {
    expect(allowedKinds({ in: 'beat', key: 'at' })).toEqual(['location', 'floor']);
    expect(allowedKinds({ in: 'beat', key: 'with' })).toEqual(['npc', 'faction', 'people']);
    expect(allowedKinds({ in: 'beat', key: 'says' })).toEqual(['belief']);
    expect(allowedKinds({ in: 'beat', key: 'reveals' })).toEqual(['clue']);
    expect(allowedKinds({ in: 'beat', key: 'when' })).toEqual([]);
    expect(allowedKinds({ in: 'prose' })).toHaveLength(20);
  });

  it('filters the search by place', () => {
    const context = createMockContext({ fileId: 'records/story/scripts/S-004.md' });
    const at = context.search('', { in: 'beat', key: 'at' });
    expect(at.length).toBeGreaterThan(0);
    expect(new Set(at.map((hit) => hit.kind))).toEqual(new Set(['location', 'floor']));
    expect(context.search('', { in: 'beat', key: 'when' })).toEqual([]);
  });
});

describe('the mock context', () => {
  it('carries the keymap, the developer words and the findings of the file', () => {
    const context = createMockContext({ fileId: 'records/story/scripts/S-004.md' });
    expect(context.keymap).toEqual({ save: 'Mod-s', undo: 'Mod-z', redo: 'Mod-Shift-z', selectAll: 'Mod-a' });
    expect(context.keymap).toEqual(keymap);
    expect(context.developerWords).toEqual(['T-012', 'never-confirm', 'tier', 'ledger', 'canon']);
    expect(context.developerWords).toEqual(developerWords);
    expect(context.findings).toEqual([
      { rule: 'never-confirm', severity: 'error', message: expect.any(String), at: { beat: 2 } },
    ]);
  });

  it('has no findings for other files and counts saves through the callback', () => {
    expect(findingsFor('records/story/scripts/S-901.md')).toEqual([]);
    let saves = 0;
    const context = createMockContext({ fileId: 'x', save: () => saves++ });
    context.save();
    expect(saves).toBe(1);
  });
});
