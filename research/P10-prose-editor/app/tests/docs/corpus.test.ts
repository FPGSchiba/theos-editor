import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { describe, expect, it } from 'vitest';
import { generateLong } from '../../../corpus/generate-long.ts';
import { resolveAnchor } from '../../src/harness/anchor.ts';
import { splitFrontMatter } from '../../src/harness/frontmatter.ts';
import { CONSTRUCTS, CORPUS_DIR, readCorpusText, readManifest } from '../../src/harness/manifest.ts';
import type { CorpusFile } from '../../src/harness/manifest.ts';
import { editOffset, expectedAfter, expectedAfterExternalUndo, toLf } from '../../src/harness/oracle.ts';
import type { Anchor, ScriptedEdit } from '../../src/harness/types.ts';
import { beatsOf, DETECTORS, partsOf } from './corpus-detectors.ts';

const MARKER = '# synthetic fixture (P10), not canon';
const manifest = readManifest();
const files = manifest.files;
const texts = new Map<string, string>(files.map((file) => [file.path, readCorpusText(file)]));
const textOf = (file: CorpusFile): string => texts.get(file.path) ?? '';
const lfOf = (file: CorpusFile): string => toLf(textOf(file));
const ids = new Set(files.map((file) => file.path.replace(/^.*\//, '').replace(/\.md$/, '')));

/** Every edit of the manifest with its file; the inner edits of an undo-all are not listed. */
const allEdits = files.flatMap((file) => file.edits.map((edit) => ({ file, edit })));
const editNamed = (id: string): { file: CorpusFile; edit: ScriptedEdit } => {
  const found = allEdits.find((entry) => entry.edit.id === id);
  if (found === undefined) throw new Error(`the manifest has no edit "${id}"`);
  return found;
};

describe('the corpus manifest', () => {
  it('lists every record file and nothing else', () => {
    const onDisk: string[] = [];
    const walk = (dir: string): void => {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const path = join(dir, entry.name);
        if (entry.isDirectory()) walk(path);
        else onDisk.push(relative(CORPUS_DIR, path).split('\\').join('/'));
      }
    };
    walk(join(CORPUS_DIR, 'records'));
    expect(files.map((file) => file.path).sort()).toEqual(onDisk.sort());
  });

  it('names an id that equals the file name, in the folder of its kind', () => {
    const folders: Record<string, string> = {
      floor: 'records/world/',
      npc: 'records/people/',
      clue: 'records/lore/clues/',
      script: 'records/story/scripts/',
      map: 'records/maps/',
      'map-version': 'records/maps/',
      'concept-version': 'records/concepts/',
      belief: 'records/lore/beliefs/',
      truth: 'records/lore/truths/',
      topic: 'records/lore/topics/',
      thing: 'records/things/',
    };
    for (const file of files) {
      const { fm } = partsOf(textOf(file));
      const id = fm.find((l) => l.startsWith('id: '))?.slice(4).trim();
      const kind = fm.find((l) => l.startsWith('kind: '))?.slice(6).trim() ?? '';
      expect(`${id}.md`, file.path).toBe(file.path.replace(/^.*\//, ''));
      expect(file.path.startsWith(folders[kind] ?? '<unknown kind>'), `${file.path} (${kind})`).toBe(true);
    }
  });

  it('gives synthetic records ids that follow the data model and are marked as synthetic', () => {
    const pattern: Record<string, RegExp> = {
      script: /^S-\d{3,}$/,
      belief: /^B-\d{3,}$/,
      truth: /^T-\d{3,}$/,
      clue: /^C-\d{3,}$/,
      topic: /^[a-z0-9]+(?:-[a-z0-9]+)*-(?:explaining|in-world)$/,
    };
    for (const file of files.filter((candidate) => candidate.origin === 'synthetic')) {
      const { fm } = partsOf(textOf(file));
      const id = fm.find((l) => l.startsWith('id: '))?.slice(4).trim() ?? '';
      const kind = fm.find((l) => l.startsWith('kind: '))?.slice(6).trim() ?? '';
      expect(id, file.path).toMatch(pattern[kind] ?? /^[a-z0-9]+(?:-[a-z0-9]+)*$/);
      expect(/^[A-Z]-9\d\d$/.test(id) || id.startsWith('synthetic-'), `${id} is marked synthetic`).toBe(true);
    }
  });

  it('copies the examples of data model §14 and adds only the marker line', () => {
    const model = toLf(
      readFileSync(fileURLToPath(new URL('../../../../../docs/specs/data-model.md', import.meta.url)), 'utf8'),
    );
    const examples = new Map<string, string>();
    for (const block of model.slice(model.indexOf('\n## 14. Examples')).matchAll(/```markdown\n([\s\S]*?)\n```/g)) {
      let current: string | undefined;
      let lines: string[] = [];
      const flush = (): void => {
        if (current === undefined) return;
        while (lines[lines.length - 1] === '') lines.pop();
        examples.set(current, `${lines.join('\n')}\n`);
      };
      for (const line of (block[1] ?? '').split('\n')) {
        const header = /^--- (\S+\.md)$/.exec(line);
        if (header === null) lines.push(line);
        else {
          flush();
          current = header[1];
          lines = [];
        }
      }
      flush();
    }
    expect(examples.size).toBe(7);

    const fromModel = files.filter((file) => file.origin === 'data-model §14');
    expect(fromModel.map((file) => file.path.replace(/^records\//, '')).sort()).toEqual([...examples.keys()].sort());
    for (const file of fromModel) {
      const original = examples.get(file.path.replace(/^records\//, '')) ?? '';
      expect(textOf(file), file.path).toBe(original.replace('---\n', `---\n${MARKER}\n`));
    }
  });

  it('keeps S-004 with its two beats and the at line of beat 1', () => {
    const s004 = files.find((file) => file.path === 'records/story/scripts/S-004.md');
    expect(s004).toBeDefined();
    const beats = beatsOf(partsOf(textOf(s004 as CorpusFile)).prose);
    expect(beats.map((beat) => beat.heading)).toEqual(['## Beat 1 — Arrival', '## Beat 2 — The murmur']);
    expect(beats[0]?.attributes.find((attribute) => attribute.key === 'at')?.value).toBe('[[fates-shrine]]');
  });

  it('uses only the three note states', () => {
    const states = /^> \[!note\] n\d+(?: · \[\[[^\]]+\]\])? · (?:open|dropped|resolved by \[\[[^\]]+\]\])$/;
    for (const file of files) {
      for (const line of lfOf(file).split('\n').filter((l) => l.startsWith('> [!note]'))) {
        expect(line, file.path).toMatch(states);
      }
    }
  });
});

describe('corpus constructs', () => {
  it('has one detector per construct tag', () => {
    expect(Object.keys(DETECTORS).sort()).toEqual([...CONSTRUCTS].sort());
    expect(new Set(CONSTRUCTS).size).toBe(CONSTRUCTS.length);
  });

  it('has detectors that can say no', () => {
    // An empty record has no construct but an empty body.
    const empty = partsOf('---\n---\n');
    const firing = CONSTRUCTS.filter((tag) => DETECTORS[tag]?.(empty, { ids }) === true);
    expect(firing).toEqual(['empty-body']);
  });

  // AC: CE-06.2
  it('corpus covers every construct', () => {
    const claimed = new Set<string>();
    const wrong: string[] = [];
    for (const file of files) {
      const parts = partsOf(textOf(file));
      for (const tag of file.covers) {
        const detector = DETECTORS[tag];
        if (detector === undefined) wrong.push(`${file.path} claims the unknown tag "${tag}"`);
        else if (!detector(parts, { ids })) wrong.push(`${file.path} claims "${tag}" but its text does not show it`);
        else claimed.add(tag);
      }
    }
    expect(wrong).toEqual([]);
    expect(CONSTRUCTS.filter((tag) => !claimed.has(tag))).toEqual([]);
  });
});

describe('corpus bytes', () => {
  it('every file has the synthetic marker', () => {
    for (const file of files) {
      const lines = lfOf(file).split('\n');
      expect(lines[0], file.path).toBe('---');
      expect(lines[1], file.path).toBe(MARKER);
      expect(partsOf(textOf(file)).fm, file.path).toContain(MARKER);
    }
  });

  it('line ends match the manifest', () => {
    expect(files.filter((file) => file.lineEnds === 'crlf')).toHaveLength(1);
    for (const file of files) {
      const text = textOf(file);
      if (file.lineEnds === 'lf') expect(text.includes('\r'), file.path).toBe(false);
      else {
        expect(text.includes('\r\n'), file.path).toBe(true);
        expect(text.replace(/\r\n/g, '').match(/[\r\n]/), file.path).toBeNull();
      }
    }
  });

  it('git keeps corpus bytes', () => {
    const research = join(CORPUS_DIR, '..');
    const output = execFileSync('git', ['check-attr', 'text', '--', ...files.map((file) => `corpus/${file.path}`)], {
      cwd: research,
      encoding: 'utf8',
    });
    const lines = output.split('\n').filter((l) => l !== '');
    expect(lines).toHaveLength(files.length);
    for (const line of lines) expect(line).toMatch(/: text: unset$/);
  });

  it('keeps the front matter closed: a body that has a thematic break is still body', () => {
    const t901 = files.find((file) => file.path === 'records/lore/truths/T-901.md');
    expect(t901).toBeDefined();
    const { frontMatter, body } = splitFrontMatter(lfOf(t901 as CorpusFile));
    expect(frontMatter.endsWith('---\n')).toBe(true);
    expect(body).toContain('\n---\n');
  });
});

describe('scripted edits', () => {
  it('every edit kind is used', () => {
    const kinds = new Set(allEdits.map(({ edit }) => edit.kind));
    expect([...kinds].sort()).toEqual(
      ['beat-param', 'delete', 'external', 'insert', 'link-delete', 'link-insert', 'note-state', 'paste', 'select-all-type', 'undo-all'],
    );
  });

  it('gives every edit an id that is unique in the manifest', () => {
    const idsSeen = allEdits.flatMap(({ edit }) =>
      edit.kind === 'undo-all' ? [edit.id, ...edit.edits.map((inner) => inner.id)] : [edit.id],
    );
    expect(idsSeen.filter((id, index) => idsSeen.indexOf(id) !== index)).toEqual([]);
  });

  it('every edit resolves', () => {
    for (const { file, edit } of allEdits) {
      const original = textOf(file);
      expect(() => expectedAfter(original, edit), edit.id).not.toThrow();
      if (edit.kind === 'external') expect(() => expectedAfterExternalUndo(original, edit), edit.id).not.toThrow();
      if (edit.kind === 'undo-all') {
        expect(edit.edits.length, edit.id).toBeGreaterThanOrEqual(2);
        for (const inner of edit.edits) expect(() => expectedAfter(original, inner), inner.id).not.toThrow();
      }
    }
  });

  it('every edit changes the file, except two that must not', () => {
    const unchanged = new Set(['body-start-backspace', 'undo-all']);
    for (const { file, edit } of allEdits) {
      const changed = expectedAfter(textOf(file), edit) !== lfOf(file);
      expect(changed, edit.id).toBe(!unchanged.has(edit.id));
    }
  });

  it('every anchor is unique unless it names an occurrence', () => {
    const anchorsOf = (edit: ScriptedEdit): Anchor[] => {
      switch (edit.kind) {
        case 'insert':
        case 'delete':
        case 'link-insert':
        case 'paste':
          return [edit.at];
        case 'link-delete':
          return [edit.link];
        case 'external':
          return [edit.local.at, edit.external.at];
        case 'undo-all':
          return edit.edits.map((inner) => inner.at);
        case 'beat-param':
        case 'note-state':
        case 'select-all-type':
          return [];
      }
    };
    for (const { file, edit } of allEdits) {
      for (const anchor of anchorsOf(edit)) {
        if (anchor.occurrence !== undefined) continue;
        const count = lfOf(file).split(anchor.find).length - 1;
        expect(count, `${edit.id}: ${JSON.stringify(anchor.find)}`).toBe(1);
      }
    }
  });

  it("the spec's edits exist", () => {
    const wanted: Record<string, ScriptedEdit['kind']> = {
      'word-insert': 'insert',
      'word-delete': 'delete',
      'voice-end-sentence': 'insert',
      'note-state-change': 'note-state',
      'beat-param-add': 'beat-param',
      'beat-param-change': 'beat-param',
      'link-insert-popover': 'link-insert',
      'link-chip-delete': 'link-delete',
      'select-all-type': 'select-all-type',
      'paste-crlf': 'paste',
      'paste-html': 'paste',
      'external-while-dirty': 'external',
      'undo-all': 'undo-all',
    };
    for (const [id, kind] of Object.entries(wanted)) expect(editNamed(id).edit.kind, id).toBe(kind);

    // Mid-paragraph: inside a line of plain text, neither at its start nor at its end.
    for (const id of ['word-insert', 'word-delete']) {
      const { file, edit } = editNamed(id);
      const lf = lfOf(file);
      const from = editOffset(textOf(file), edit) ?? -1;
      const to = edit.kind === 'delete' ? from + edit.length : from;
      expect(from, id).toBeGreaterThan(splitFrontMatter(lf).frontMatter.length);
      expect(lf[from - 1], id).not.toBe('\n');
      expect(lf[to], id).not.toBe('\n');
    }

    // The sentence goes to the end of a voice block's text line.
    const voice = editNamed('voice-end-sentence');
    const voiceLf = lfOf(voice.file);
    const voiceAt = editOffset(textOf(voice.file), voice.edit) ?? -1;
    expect(voiceLf[voiceAt]).toBe('\n');
    const voiceLine = voiceLf.slice(voiceLf.lastIndexOf('\n', voiceAt - 1) + 1, voiceAt);
    expect(voiceLine.startsWith('> ')).toBe(true);
    expect(voiceLf.slice(0, voiceAt)).toMatch(/> \[!voice\][^\n]*\n(?:>[^\n]*\n)*>[^\n]*$/);

    // A beat parameter is added where the key is absent and changed where it is present.
    for (const [id, present] of [['beat-param-add', false], ['beat-param-change', true]] as const) {
      const { file, edit } = editNamed(id);
      if (edit.kind !== 'beat-param') throw new Error(`${id} is not a beat-param edit`);
      const beat = beatsOf(partsOf(textOf(file)).prose).find((candidate) =>
        candidate.heading.startsWith(`## Beat ${edit.beat}`),
      );
      expect(beat, id).toBeDefined();
      expect(beat?.attributes.some((attribute) => attribute.key === edit.key), id).toBe(present);
    }

    // The two pastes: CRLF text inserted as LF, and rich HTML whose plain text is what counts.
    const crlf = editNamed('paste-crlf').edit;
    const html = editNamed('paste-html').edit;
    if (crlf.kind !== 'paste' || html.kind !== 'paste') throw new Error('a paste edit is not a paste');
    expect(crlf.plain).toContain('\r\n');
    expect(expectedAfter(textOf(editNamed('paste-crlf').file), crlf)).not.toContain('\r');
    expect(html.html).toMatch(/<\w+/);
    expect(html.plain).not.toMatch(/[<>]/);

    // The link goes in after a space, as when the author types "[[Listen" after "names ".
    const popover = editNamed('link-insert-popover');
    const popoverAt = editOffset(textOf(popover.file), popover.edit) ?? -1;
    expect(lfOf(popover.file)[popoverAt - 1]).toBe(' ');

    // An undo-all edit lists the edits to undo, each of them an insert or a delete.
    const undo = editNamed('undo-all').edit;
    if (undo.kind !== 'undo-all') throw new Error('undo-all is not an undo-all edit');
    expect(undo.edits.every((inner) => inner.kind === 'insert' || inner.kind === 'delete')).toBe(true);
  });

  it('Review Focus edits exist', () => {
    const backspace = editNamed('body-start-backspace');
    expect(backspace.file.path).toBe('records/story/scripts/S-004.md');
    if (backspace.edit.kind !== 'delete') throw new Error('body-start-backspace is not a delete');
    expect(backspace.edit.backward).toBe(true);
    const s004 = lfOf(backspace.file);
    expect(resolveAnchor(s004, backspace.edit.at)).toBe(splitFrontMatter(s004).frontMatter.length);

    const eof = editNamed('eof-insert');
    expect(eof.edit.kind).toBe('insert');
    expect(DETECTORS['no-final-newline']?.(partsOf(textOf(eof.file)), { ids })).toBe(true);
    if (eof.edit.kind !== 'insert') throw new Error('eof-insert is not an insert');
    expect(resolveAnchor(lfOf(eof.file), eof.edit.at)).toBe(lfOf(eof.file).length);

    // A paste at the first body character, directly after the closing fence.
    const pasteStart = editNamed('paste-body-start');
    expect(pasteStart.edit.kind).toBe('paste');
    if (pasteStart.edit.kind !== 'paste') throw new Error('paste-body-start is not a paste');
    const startLf = lfOf(pasteStart.file);
    expect(resolveAnchor(startLf, pasteStart.edit.at)).toBe(splitFrontMatter(startLf).frontMatter.length);

    const afterBreak = editNamed('after-thematic-break');
    expect(afterBreak.edit.kind).toBe('insert');
    const lf = lfOf(afterBreak.file);
    const at = editOffset(textOf(afterBreak.file), afterBreak.edit) ?? -1;
    const rule = lf.lastIndexOf('\n---\n', at);
    expect(rule).toBeGreaterThan(splitFrontMatter(lf).frontMatter.length - 1);
    // Directly after the break comes one blank line and then the paragraph; the edit is inside it.
    expect(lf.slice(rule + 5, at)).toMatch(/^\n[^\n]+$/);
  });
});

describe('long documents', () => {
  const sizes = [5_000, 250_000, 2_500_000];

  it('long documents are deterministic', () => {
    for (const size of sizes) {
      const first = generateLong(size);
      expect(generateLong(size)).toBe(first);
      const bytes = Buffer.byteLength(first, 'utf8');
      expect(bytes, `${size} bytes`).toBeGreaterThanOrEqual(size * 0.9);
      expect(bytes, `${size} bytes`).toBeLessThanOrEqual(size * 1.1);
    }
  });

  it('are valid records built from beats, voice blocks and links', () => {
    const text = generateLong(5_000);
    const parts = partsOf(text);
    expect(parts.fm).toContain(MARKER);
    expect(text.includes('\r')).toBe(false);
    expect(text.endsWith('\n')).toBe(true);
    for (const tag of ['beat-all-attributes', 'voice-beliefs-tier', 'voice-caption', 'link-id', 'link-alias']) {
      expect(DETECTORS[tag]?.(parts, { ids }), tag).toBe(true);
    }
    expect(generateLong(250_000).length).toBeGreaterThan(text.length);
  });

  it('are written only when the file runs as a script, under plain node', () => {
    const sandbox = mkdtempSync(join(tmpdir(), 'p10-long-'));
    try {
      const script = join(sandbox, 'generate-long.ts');
      copyFileSync(join(CORPUS_DIR, 'generate-long.ts'), script);

      // Imported, it writes nothing.
      const importIt = `await import(${JSON.stringify(pathToFileURL(script).href)})`;
      execFileSync(process.execPath, ['--input-type=module', '-e', importIt], { cwd: sandbox });
      expect(existsSync(join(sandbox, 'generated'))).toBe(false);

      // Run as a script, it writes the three files next to itself.
      execFileSync(process.execPath, [script], { cwd: sandbox });
      for (const [name, size] of [['typical', 5_000], ['large', 250_000], ['stress', 2_500_000]] as const) {
        const written = readFileSync(join(sandbox, 'generated', `${name}.md`), 'utf8');
        expect(written, name).toBe(generateLong(size));
        expect(statSync(join(sandbox, 'generated', `${name}.md`)).size, name).toBe(Buffer.byteLength(written, 'utf8'));
      }
    } finally {
      rmSync(sandbox, { recursive: true, force: true });
    }
  });
});
