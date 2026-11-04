// The long documents of spec §4.8: a script record of about 5 KB, 250 KB and 2.5 MB, built from
// one fixed block of beat, attributes, prose with links and a voice block. Deterministic: the
// same size gives the same bytes. Nothing is committed; run `node corpus/generate-long.ts` from
// research/P10-prose-editor/ to write corpus/generated/{typical,large,stress}.md (ignored).
//
// Plain node runs this file (type stripping): erasable TypeScript syntax only, Node built-ins
// only, and a `.ts` extension on every relative import.
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

const MARKER = '# synthetic fixture (P10), not canon';

const FRONT_MATTER = [
  '---',
  MARKER,
  'id: S-999',
  'kind: script',
  'title: Synthetic long script',
  'status: draft',
  'where: [f04-choir-hall]',
  '---',
  '',
].join('\n');

const INTRO =
  "The player's first night on F04. Mother Ilse welcomes them, the hall hums at dusk, and under the shrine one of the drifting lights says a word nobody alive can speak.";

const BEAT_NAMES = ['Dusk at the stair', "Ilse's welcome", 'The murmur', 'The vigil'];

/** The repeated block: a beat with every attribute, prose with links, and a voice block. */
function block(n: number): string {
  const when = `T${(n % 3) + 1}`;
  return [
    `## Beat ${n} — ${BEAT_NAMES[n % BEAT_NAMES.length]}`,
    'at:: [[fates-shrine]]',
    'with:: [[mother-ilse]]',
    'says:: [[B-016]]',
    'reveals:: [[C-031]]',
    `when:: ${when}`,
    '',
    'The stair is crowded with candles. The hum rises as the light-strip dims, and the villagers',
    "stop talking until it settles. A light drifts down the cellar steps, deep in the [[fates-shrine|Fates' shrine]],",
    'and the [[hearth-folk]] keep the children in.',
    '',
    `> [!voice] [[mother-ilse]] · [[B-016]] · ${when}`,
    '> The Fates spin for those who climb. Hum with me, stair-child, and they will hear you.',
    '> — a saying on the shrine stair',
  ].join('\n');
}

const SEPARATOR = '\n\n';

/**
 * A script record of about `targetBytes` UTF-8 bytes: the front matter, an intro paragraph and
 * as many blocks as fit, taking the block count that lands nearest the target. The text ends
 * with one newline and has no CR.
 */
export function generateLong(targetBytes: number): string {
  const head = `${FRONT_MATTER}${INTRO}`;
  const parts: string[] = [head];
  let size = Buffer.byteLength(head) + 1; // the final newline
  for (let n = 1; ; n++) {
    const next = SEPARATOR + block(n);
    const nextSize = Buffer.byteLength(next);
    const overshoot = size + nextSize - targetBytes;
    if (overshoot > 0) {
      if (overshoot < targetBytes - size) parts.push(next);
      break;
    }
    parts.push(next);
    size += nextSize;
  }
  return `${parts.join('')}\n`;
}

const SIZES: ReadonlyArray<readonly [string, number]> = [
  ['typical', 5_000],
  ['large', 250_000],
  ['stress', 2_500_000],
];

function writeAll(): void {
  const dir = new URL('./generated/', import.meta.url);
  mkdirSync(dir, { recursive: true });
  for (const [name, size] of SIZES) {
    const text = generateLong(size);
    writeFileSync(new URL(`${name}.md`, dir), text, 'utf8');
    console.log(`${fileURLToPath(new URL(`${name}.md`, dir))}: ${Buffer.byteLength(text)} bytes`);
  }
}

// Only when this file is the script node runs, never when a test imports it.
const runAsScript =
  import.meta.main ?? (process.argv[1] !== undefined && pathToFileURL(process.argv[1]).href === import.meta.url);
if (runAsScript) writeAll();
