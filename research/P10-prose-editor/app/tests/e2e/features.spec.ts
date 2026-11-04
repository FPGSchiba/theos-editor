import { test, type Page } from '@playwright/test';
import { resolveAnchor } from '../../src/harness/anchor.ts';
import { compareBytes } from '../../src/harness/compare.ts';
import { developerWords } from '../../src/harness/findings.ts';
import type { CorpusFile } from '../../src/harness/manifest.ts';
import { expectedAfter, toLf } from '../../src/harness/oracle.ts';
import type { Anchor } from '../../src/harness/types.ts';
import {
  caretAt, collectRows, collectTexts, corpusFile, count, eventually, judge, runCheck, savedText,
  settle, texts, visibleText,
  type Verdict,
} from './checks.ts';
import { loadFile, variantsUnderTest } from './helpers.ts';

// The rendered features behind CE-06.1 criteria 3 to 5 and the Write/Source modes: link chips and
// the `[[` popover, the three callouts and the developer words, beat headings and rows, live
// preview. Each check reads the DOM contract of the shared components (context.md) on one corpus
// file and records `ok` with a detail; a variant that renders none of it (the baseline textarea)
// records `ok: false` for every one. A callout may be one wrapper or a run of line elements, so
// no selector here assumes a single element: the checks use counts, `.first()` and the text of
// all matching elements.

const LINKS = corpusFile('records/lore/topics/synthetic-links-explaining.md');
const VOICES = corpusFile('records/lore/topics/synthetic-voices-in-world.md');
const NOTES = corpusFile('records/maps/synthetic-notes.md');
const SCRIPT = corpusFile('records/story/scripts/S-004.md');

type Tag = '@links' | '@callouts' | '@beats' | '@base';

interface Check {
  id: string;
  tag: Tag;
  file: CorpusFile;
  /** `source` is the LF form of the file as loaded. */
  run: (page: Page, source: string) => Promise<Verdict>;
}

// ── Page queries ─────────────────────────────────────────────────────────────

interface ChipInfo {
  kind: string | null;
  title: string;
  broken: boolean;
}

/** Every chip of the editor with this link id, over the whole scroll range. */
function chipsOf(page: Page, id: string): Promise<ChipInfo[]> {
  return collectRows(page, '.tu-link', (chip) => ({
    id: chip.getAttribute('data-link-id'),
    kind: chip.querySelector('.tu-kind')?.getAttribute('data-kind') ?? null,
    title: (chip.querySelector('.tu-link__title')?.textContent ?? '').replace(/\s+/g, ' ').trim(),
    broken: chip.classList.contains('is-broken'),
  })).then((rows) => rows.filter((row) => row.id === id));
}

/** Wait for a chip with this id to be rendered; a variant that draws none never gets one. */
const chipsShown = (page: Page, id: string): Promise<boolean> =>
  eventually(async () => (await chipsOf(page, id)).length > 0);

const POPOVER = '.tu-pop[data-link-popover]';

const popoverShown = (page: Page): Promise<boolean> =>
  eventually(async () => (await count(page, POPOVER, false)) > 0, 1_500);

/** The rows of the popover, which may sit outside the editor. */
function popoverRows(page: Page): Promise<Array<{ id: string; kind: string }>> {
  return page.evaluate((selector) =>
    [...document.querySelectorAll(`${selector} .tu-pop__row`)].map((row) => ({
      id: row.getAttribute('data-link-option') ?? '',
      kind: row.getAttribute('data-kind') ?? '',
    })), POPOVER);
}

/** The text of the heading of a beat, read over the whole scroll range. */
const beatHeadingAnywhere = async (page: Page, beat: number): Promise<string | undefined> =>
  (await collectTexts(page, `[data-beat="${beat}"]`))[0];

/** Whether any element of the editor matches, wherever it is scrolled to. */
const existsAnywhere = async (page: Page, selector: string): Promise<boolean> =>
  (await collectRows(page, selector, () => ({}))).length > 0;

/** The (note, state) pairs of every note callout, over the whole scroll range; a note drawn as a run of lines shows up once. */
async function notesOf(page: Page): Promise<Array<[string, string]>> {
  const rows = await collectRows(page, '[data-callout="note"]', (node) => ({
    note: node.getAttribute('data-note') ?? '',
    state: node.getAttribute('data-note-state') ?? '',
  }));
  return [...new Map(rows.map((row) => [row.note, row.state] as [string, string]))];
}

/** The end of a line, as an anchor: its text found once, the caret behind it. */
const endOf = (find: string): Anchor => ({ find, offset: find.length });

/** The last prose paragraph of the link file; typing there opens a `[[` in prose. */
const PROSE_END = endOf('operator-plate names a Listener.');

// ── The checks ───────────────────────────────────────────────────────────────

const checks: Check[] = [
  // Inline link chips (criterion 3)
  {
    id: 'F-link-chip',
    tag: '@links',
    file: LINKS,
    async run(page) {
      if (!(await chipsShown(page, 'mother-ilse'))) return judge(['no .tu-link chip for [[mother-ilse]] is rendered']);
      const [chip] = await chipsOf(page, 'mother-ilse');
      const problems: string[] = [];
      if (chip?.kind !== 'npc') problems.push(`the kind is ${JSON.stringify(chip?.kind)}, not "npc"`);
      if (chip?.title !== 'Mother Ilse') problems.push(`the title is ${JSON.stringify(chip?.title)}, not "Mother Ilse"`);
      if (chip?.broken === true) problems.push('the chip is marked is-broken');
      return judge(problems, 'npc chip titled "Mother Ilse"');
    },
  },
  {
    id: 'F-link-alias',
    tag: '@links',
    file: LINKS,
    async run(page) {
      // [[B-020|Listeners answer to the right song]]: the record's own title is another text.
      if (!(await chipsShown(page, 'B-020'))) return judge(['no .tu-link chip for [[B-020|…]] is rendered']);
      const titles = (await chipsOf(page, 'B-020')).map((chip) => chip.title);
      const problems: string[] = [];
      if (!titles.includes('Listeners answer to the right song')) {
        problems.push(`no B-020 chip shows the alias (titles: ${titles.map((t) => JSON.stringify(t)).join(', ')})`);
      }
      const raw = (await collectTexts(page, '.tu-link__title')).find((title) => title.includes('|') || title.includes('[['));
      if (raw !== undefined) problems.push(`a chip shows link syntax: ${JSON.stringify(raw)}`);
      return judge(problems, 'the alias is the shown text');
    },
  },
  {
    id: 'F-link-broken',
    tag: '@links',
    file: LINKS,
    async run(page) {
      if (!(await chipsShown(page, 'song-cellar'))) return judge(['no .tu-link chip for the dangling [[song-cellar]] is rendered']);
      const dangling = await chipsOf(page, 'song-cellar');
      const resolved = await chipsOf(page, 'fates-shrine');
      const problems: string[] = [];
      if (dangling.some((chip) => !chip.broken)) problems.push('the chip of [[song-cellar]] lacks is-broken');
      if (resolved.length === 0) problems.push('no chip for the resolved [[fates-shrine]] to compare with');
      if (resolved.some((chip) => chip.broken)) problems.push('a resolved chip is marked is-broken');
      return judge(problems, 'only the dangling link is broken');
    },
  },
  {
    id: 'F-link-atomic',
    tag: '@links',
    file: LINKS,
    async run(page, source) {
      // The caret in front of the first link steps over all of it with one ArrowRight.
      const link = '[[fates-shrine]]';
      const start = resolveAnchor(source, { find: link });
      await caretAt(page, start);
      await page.keyboard.press('ArrowRight');
      await settle(page);
      const caret = await page.evaluate(() => window.__harness!.caret());
      const behind = start + link.length;
      return caret === behind
        ? judge([], 'one ArrowRight moves the caret behind ]]')
        : judge([`ArrowRight from before "[[" put the caret at ${caret - start} characters, not ${link.length}`]);
    },
  },
  {
    id: 'F-link-popover-prose',
    tag: '@links',
    file: LINKS,
    async run(page, source) {
      await caretAt(page, resolveAnchor(source, PROSE_END));
      await page.keyboard.type('[[Listen');
      if (!(await popoverShown(page))) return judge(['typing "[[Listen" in prose opens no .tu-pop[data-link-popover]']);
      await settle(page);
      const rows = await popoverRows(page);
      const kinds = [...new Set(rows.map((row) => row.kind))].sort();
      const problems: string[] = [];
      if (rows.length === 0) problems.push('the popover has no rows');
      else if (kinds.length < 3) problems.push(`the options are of ${kinds.length} kind(s) (${kinds.join(', ')}), expected several`);
      return judge(problems, `${rows.length} options of ${kinds.length} kinds: ${kinds.join(', ')}`);
    },
  },
  {
    id: 'F-link-popover-kind-filter',
    tag: '@links',
    file: SCRIPT,
    async run(page, source) {
      // In an `at::` line only locations and floors may be linked (data model §9.4). "fates" also
      // matches a faction and a concept version, which a filter has to leave out.
      await caretAt(page, resolveAnchor(source, endOf('at:: [[fates-shrine]]')));
      await page.keyboard.type(' [[fates');
      if (!(await popoverShown(page))) return judge(['typing "[[fates" in an at:: line opens no popover']);
      await settle(page);
      const rows = await popoverRows(page);
      const stray = rows.filter((row) => row.kind !== 'location' && row.kind !== 'floor');
      const problems: string[] = [];
      if (rows.length === 0) problems.push('the popover has no rows');
      if (stray.length > 0) problems.push(`options of other kinds in an at:: line: ${stray.map((row) => `${row.id} (${row.kind})`).join(', ')}`);
      if (rows.length > 0 && !rows.some((row) => row.id === 'fates-shrine')) problems.push('the location fates-shrine is not offered');
      return judge(problems, `only location and floor options: ${rows.map((row) => row.id).join(', ')}`);
    },
  },
  {
    id: 'F-link-popover-escape',
    tag: '@links',
    file: LINKS,
    async run(page, source) {
      // Review Focus 2: "[[" then Escape closes the popover and keeps what was typed, as typed.
      const problems: string[] = [];
      await caretAt(page, resolveAnchor(source, PROSE_END));
      await page.keyboard.type('[[son');
      if (!(await popoverShown(page))) problems.push('typing "[[son" opens no popover, so there is nothing to close');
      await page.keyboard.press('Escape');
      await settle(page);
      if ((await count(page, POPOVER, false)) > 0) problems.push('the popover is still open after Escape');
      const expected = expectedAfter(source, { id: 'F-link-popover-escape', kind: 'insert', at: PROSE_END, text: '[[son' });
      const result = compareBytes(await savedText(page), expected);
      if (!result.identical) {
        const at = result.firstDiff === undefined ? '' : ` at ${result.firstDiff.line}:${result.firstDiff.column}`;
        problems.push(`the text differs from the oracle insert of "[[son"${at}`);
      }
      return judge(problems, 'no popover, and the text is the typed "[[son"');
    },
  },

  // Callouts (criterion 4)
  {
    id: 'F-voice',
    tag: '@callouts',
    file: VOICES,
    async run(page) {
      const voiceRows = () =>
        collectRows(page, '[data-callout="voice"]', (node) => ({ tuVoice: node.classList.contains('tu-voice') }));
      let voices = await voiceRows();
      if (voices.length === 0 && (await eventually(async () => (await voiceRows()).length > 0))) voices = await voiceRows();
      if (voices.length === 0) return judge(['no [data-callout="voice"] element is rendered']);
      const problems: string[] = [];
      if (voices.some((voice) => !voice.tuVoice)) problems.push('a voice element lacks the tu-voice class');
      const heads = await collectTexts(page, '.tu-voice__head');
      for (const speaker of ['Hearth folk', 'Throat-Kin binders', 'Mother Ilse']) {
        if (!heads.some((head) => head.includes(speaker))) problems.push(`no .tu-voice__head names ${speaker}`);
      }
      const captions = await collectTexts(page, '.tu-voice__by');
      for (const caption of ['a binder, teaching the naming-song', 'a saying on the shrine stair', 'a real caption after it']) {
        if (!captions.some((shown) => shown.includes(caption))) problems.push(`no .tu-voice__by shows "${caption}"`);
      }
      // A line that starts with "--" is voice text, not a caption (voice-caption-double-hyphen).
      if (captions.some((shown) => shown.includes('two hyphens'))) problems.push('a line starting with "--" is shown as a caption');
      return judge(problems, `${heads.length} heads, ${captions.length} captions`);
    },
  },
  {
    id: 'F-design',
    tag: '@callouts',
    file: VOICES,
    async run(page) {
      if (!(await eventually(() => existsAnywhere(page, '[data-callout="design"]')))) {
        return judge(['no [data-callout="design"] element is rendered']);
      }
      const problems: string[] = [];
      const design = (await collectTexts(page, '[data-callout="design"]')).join(' ');
      if (!design.includes('A synthetic designer remark inside an in-world text.')) problems.push('the design callout does not show its text');
      const notes = await collectTexts(page, '.tu-note');
      if (!notes.some((line) => line.includes('the player first meets this belief'))) problems.push('no .tu-note element holds the "Design note:" line');
      if (await existsAnywhere(page, '.tu-note[data-callout]')) problems.push('the "Design note:" line carries data-callout');
      return judge(problems, 'design callout and "Design note:" line');
    },
  },
  {
    id: 'F-note',
    tag: '@callouts',
    file: NOTES,
    async run(page) {
      if (!(await eventually(() => existsAnywhere(page, '[data-callout="note"]')))) {
        return judge(['no [data-callout="note"] element is rendered']);
      }
      const found = new Map(await notesOf(page));
      const expected: Array<[string, string]> = [['n1', 'open'], ['n2', 'open'], ['n3', 'resolved'], ['n4', 'dropped'], ['n5', 'dropped']];
      const problems: string[] = [];
      for (const [note, state] of expected) {
        const shown = found.get(note);
        if (shown !== state) problems.push(`${note} has data-note-state ${JSON.stringify(shown)}, expected "${state}"`);
      }
      return judge(problems, expected.map(([note, state]) => `${note}=${state}`).join(' '));
    },
  },
  {
    id: 'F-dev-word',
    tag: '@callouts',
    file: VOICES,
    async run(page) {
      // The only developer word inside voice text is "ledger"; T-012 sits in the design note and
      // "canon" in the (hidden) front matter, so neither may be flagged.
      const words = await collectRows(
        page,
        '[data-dev-word]',
        (node) => ({ text: (node.textContent ?? '').trim(), inVoice: node.closest('[data-callout="voice"]') !== null }),
        (row) => `${row.text}|${row.inVoice}`,
      );
      if (words.length === 0) return judge(['no [data-dev-word] element is rendered']);
      const problems: string[] = [];
      const listed = developerWords.map((word) => word.toLowerCase());
      if (!words.some((word) => word.text.toLowerCase() === 'ledger' && word.inVoice)) problems.push('"ledger" is not flagged inside a voice block');
      for (const word of words) {
        if (!word.inVoice) problems.push(`"${word.text}" is flagged outside a voice block`);
        if (!listed.includes(word.text.toLowerCase())) problems.push(`"${word.text}" is flagged but is not a developer word`);
      }
      return judge(problems, `flagged: ${words.map((word) => word.text).join(', ')}`);
    },
  },

  // Beat headings with attribute lines (criterion 5)
  {
    id: 'F-beat-heading',
    tag: '@beats',
    file: SCRIPT,
    async run(page) {
      if (!(await eventually(() => existsAnywhere(page, '[data-beat]')))) return judge(['no [data-beat] heading is rendered']);
      const problems: string[] = [];
      const first = await beatHeadingAnywhere(page, 1);
      const second = await beatHeadingAnywhere(page, 2);
      if (first !== '1 · Arrival') problems.push(`beat 1 shows ${JSON.stringify(first)}, not "1 · Arrival"`);
      if (second !== '2 · The murmur') problems.push(`beat 2 shows ${JSON.stringify(second)}, not "2 · The murmur"`);
      return judge(problems, '"1 · Arrival" and "2 · The murmur"');
    },
  },
  {
    id: 'F-beat-row',
    tag: '@beats',
    file: SCRIPT,
    async run(page) {
      if (!(await eventually(() => existsAnywhere(page, '[data-beat-row="1"]')))) return judge(['no [data-beat-row="1"] is rendered']);
      const problems: string[] = [];
      if (!(await existsAnywhere(page, '[data-beat-row="1"] .tu-beat'))) problems.push('the row has no .tu-beat');
      const chips = await existsAnywhere(page, '[data-beat-row="1"] [data-beat-param="at"] .tu-link[data-link-id="fates-shrine"]');
      if (!chips) problems.push('the at param holds no chip for fates-shrine');
      return judge(problems, 'the at param holds the fates-shrine chip');
    },
  },
  {
    id: 'F-beat-finding',
    tag: '@beats',
    file: SCRIPT,
    async run(page) {
      if (!(await eventually(() => existsAnywhere(page, '[data-beat-row="2"]')))) return judge(['no [data-beat-row="2"] is rendered']);
      const problems: string[] = [];
      const own = '[data-beat-row="2"] [data-beat-finding][data-finding-rule="never-confirm"]';
      if (!(await existsAnywhere(page, own))) problems.push('beat 2 shows no never-confirm finding');
      if (await existsAnywhere(page, '[data-beat-row="1"] [data-beat-finding]')) problems.push('beat 1 shows a finding it does not have');
      return judge(problems, 'never-confirm on beat 2 only');
    },
  },

  // Live preview, Source mode and the front matter (spec §4.5)
  {
    id: 'F-live-preview',
    tag: '@base',
    file: SCRIPT,
    async run(page, source) {
      const raw = '## Beat 1 — Arrival';
      const problems: string[] = [];

      // The caret on the heading of beat 1: that line is raw, the other beat is still rendered.
      await caretAt(page, resolveAnchor(source, { find: '## Beat 1', offset: 3 }));
      const showsRaw = async (): Promise<boolean> => (await visibleText(page)).toLowerCase().includes(raw.toLowerCase());
      const rawShown = await eventually(showsRaw);
      if (!rawShown) problems.push(`with the caret on it, "${raw}" is not shown raw`);
      if ((await beatHeadingAnywhere(page, 2)) !== '2 · The murmur') problems.push('with the caret on beat 1, beat 2 is not rendered as "2 · The murmur"');

      // The caret elsewhere: beat 1 is rendered again and its raw heading is gone.
      await caretAt(page, resolveAnchor(source, { find: "The player's first night", offset: 4 }));
      const rendered = await eventually(async () => (await beatHeadingAnywhere(page, 1)) === '1 · Arrival');
      if (!rendered) problems.push('with the caret elsewhere, beat 1 is not rendered as "1 · Arrival"');
      else if (await showsRaw()) problems.push(`with the caret elsewhere, "${raw}" is still shown raw`);
      return judge(problems, 'raw under the caret, rendered elsewhere');
    },
  },
  {
    id: 'F-source-mode',
    tag: '@base',
    file: SCRIPT,
    async run(page, source) {
      const problems: string[] = [];
      await page.evaluate(() => window.__harness!.setMode('source'));
      await settle(page);
      const shown = await visibleText(page);
      if (!shown.includes('id: S-004') || !shown.includes('kind: script')) problems.push('Source mode does not show the front matter');
      const chips = (await collectRows(page, '.tu-link', () => ({}))).length;
      if (chips > 0) problems.push(`Source mode renders ${chips} chip(s), it has to show the raw text`);

      // The front matter is protected: typing into it leaves the text as it was.
      await caretAt(page, resolveAnchor(source, { find: 'id: S-004', offset: 4 }));
      await page.keyboard.type('zz');
      await settle(page);
      const result = compareBytes(await savedText(page), source);
      if (!result.identical) {
        const at = result.firstDiff === undefined ? '' : ` at ${result.firstDiff.line}:${result.firstDiff.column}`;
        problems.push(`typing into the front matter changed the text${at}`);
      }
      return judge(problems, 'front matter shown raw, no chips, typing into it changes nothing');
    },
  },
  {
    id: 'F-frontmatter-hidden',
    tag: '@base',
    file: SCRIPT,
    async run(page) {
      await settle(page);
      const shown = await visibleText(page);
      const problems: string[] = [];
      if (!shown.includes("The player's first night")) problems.push('the body is not shown, so the front matter cannot be judged');
      for (const line of ['id: S-004', 'kind: script', 'synthetic fixture (P10)']) {
        if (shown.includes(line)) problems.push(`Write mode shows the front matter line "${line}"`);
      }
      return judge(problems, 'Write mode shows the body and no front matter');
    },
  },
];

for (const variant of variantsUnderTest()) {
  for (const check of checks) {
    test(`${check.tag} ${check.id} · ${variant.name}`, async ({ page, browserName }) => {
      await runCheck('features', { variant: variant.name, engine: browserName, check: check.id }, async () => {
        const text = await loadFile(page, variant.name, check.file);
        return check.run(page, toLf(text));
      });
    });
  }
}
