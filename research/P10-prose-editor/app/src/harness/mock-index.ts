import { corpusEntries } from './corpus-files.ts';
import { developerWords, findingsFor } from './findings.ts';
import { splitFrontMatter } from './frontmatter.ts';
import { keymap } from './keymap.ts';
import { ALL_KINDS, allowedKinds } from './places.ts';
import type { EditorContext, Kind, LinkInfo, LinkPlace } from './types.ts';

/**
 * Ids that records and the reference screens link to but the corpus does not define as
 * records. They resolve in the mock index so that the chips of the corpus are not all broken.
 * `song-cellar` is deliberately missing: it is the corpus's dangling link (S-901,
 * synthetic-links-explaining) and must stay broken. Titles come from the reference screens
 * (design/theos-ui previews) where there is one; the others are plain placeholders.
 */
export const EXTRA_INFOS: readonly LinkInfo[] = [
  { id: 'fates-shrine', kind: 'location', title: "Fates' shrine" },
  { id: 'echo-vault', kind: 'location', title: 'echo-vault' },
  { id: 'throat-kin', kind: 'people', title: 'Throat-Kin binders' },
  { id: 'hearth-folk', kind: 'people', title: 'Hearth folk' },
  { id: 'fates-cult', kind: 'faction', title: 'Fates cult' },
  { id: 'T-012', kind: 'truth', title: 'The spirits (Listeners) are the unwoven Tuners' },
  { id: 'B-014', kind: 'belief', title: 'Synthetic belief B-014' },
  { id: 'B-016', kind: 'belief', title: 'Synthetic belief B-016' },
  { id: 'B-020', kind: 'belief', title: 'Listeners are ancestors who answer to the right song' },
  { id: 'C-033', kind: 'clue', title: 'operator-plate names a Tuner a bound Listener answers to' },
  { id: 'f04-terrain-v2', kind: 'map-version', title: 'Choir-Hall terrain, seed 2' },
];

/** A front-matter scalar without its quotes; the corpus uses plain, single and double quotes. */
function scalar(frontMatter: string, key: string): string | undefined {
  const match = new RegExp(`^${key}:[ \t]*(.*?)[ \t]*\r?$`, 'm').exec(frontMatter);
  const value = match?.[1];
  if (value === undefined || value === '') return undefined;
  if (value.length >= 2 && value.startsWith("'") && value.endsWith("'")) {
    return value.slice(1, -1).replaceAll("''", "'");
  }
  if (value.length >= 2 && value.startsWith('"') && value.endsWith('"')) {
    return JSON.parse(value) as string;
  }
  return value;
}

function isKind(value: string): value is Kind {
  return (ALL_KINDS as readonly string[]).includes(value);
}

/** One `LinkInfo` per corpus record: `id`, `kind` and `title` from its front matter. */
export function corpusInfos(): LinkInfo[] {
  return corpusEntries.map(({ path, text }) => {
    const { frontMatter } = splitFrontMatter(text);
    const id = scalar(frontMatter, 'id');
    const kind = scalar(frontMatter, 'kind');
    const title = scalar(frontMatter, 'title');
    if (id === undefined || title === undefined || kind === undefined || !isKind(kind)) {
      throw new Error(`mock-index: ${path} has no usable id, kind and title in its front matter`);
    }
    return { id, kind, title };
  });
}

/** How well a record matches a query: lower is better, `undefined` is no match. */
function rank(info: LinkInfo, query: string): number | undefined {
  if (query === '') return 5;
  const id = info.id.toLowerCase();
  const title = info.title.toLowerCase();
  if (id === query) return 0;
  if (id.startsWith(query)) return 1;
  if (title.startsWith(query)) return 2;
  if (title.split(/[^\p{L}\p{N}]+/u).some((word) => word.startsWith(query))) return 3;
  if (id.includes(query) || title.includes(query)) return 4;
  return undefined;
}

/** The mock link index: what the real one will be when the editor sits on the project index. */
export class MockIndex {
  private readonly byId = new Map<string, LinkInfo>();

  constructor(infos: Iterable<LinkInfo>) {
    this.patch(infos);
  }

  resolve(id: string): LinkInfo | undefined {
    return this.byId.get(id);
  }

  /** Add or replace records by id (`updateIndex` of the bridge). */
  patch(infos: Iterable<LinkInfo>): void {
    for (const info of infos) this.byId.set(info.id, { ...info });
  }

  /** Records of the given kinds whose id or title matches the query, best match first. */
  search(query: string, kinds: readonly Kind[], limit = 8): LinkInfo[] {
    const needle = query.trim().toLowerCase();
    return [...this.byId.values()]
      .filter((info) => kinds.includes(info.kind))
      .flatMap((info) => {
        const score = rank(info, needle);
        return score === undefined ? [] : [{ info, score }];
      })
      .sort((a, b) => a.score - b.score || a.info.id.localeCompare(b.info.id))
      .slice(0, limit)
      .map(({ info }) => ({ ...info }));
  }
}

/** A fresh index of every corpus record and the extra ids. */
export function createMockIndex(): MockIndex {
  return new MockIndex([...corpusInfos(), ...EXTRA_INFOS]);
}

export interface MockContextOptions {
  /** The manifest path of the open file; picks the mock findings. */
  fileId: string;
  index?: MockIndex;
  save?: () => void;
}

/** The editor context of the harness: mock index, places, findings, developer words, keymap. */
export function createMockContext({ fileId, index = createMockIndex(), save = () => {} }: MockContextOptions): EditorContext {
  return {
    resolve: (id) => index.resolve(id),
    allowedKinds,
    search: (query: string, place: LinkPlace) => index.search(query, allowedKinds(place)),
    findings: findingsFor(fileId),
    developerWords: [...developerWords],
    keymap,
    save,
  };
}
