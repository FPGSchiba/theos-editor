// The layouts of the unbuilt parts (P11 §5.9), Atlas included, which the fixture rail of the page
// tests does not have: each preset's id, its number of windows and the phase every window names.

import { expect, test } from 'vitest';

import { isSplit, type Split, type WindowSpec } from './layout';
import { presets } from './presets';

/** The windows of `split`, in reading order. */
function windowsOf(split: Split): WindowSpec[] {
  return split.items.flatMap(({ child }) => (isSplit(child) ? windowsOf(child) : [child]));
}

/** The phase a window names, from its empty-state props. */
function phaseOf(window: WindowSpec): unknown {
  return (window.props as { phase?: unknown } | undefined)?.phase;
}

/** Each placeholder preset: how many windows it has and the phase that brings it (spec §5.9). */
const PLACEHOLDERS: Record<string, { windows: number; phase: string }> = {
  atlas: { windows: 3, phase: 'P25' },
  graph: { windows: 3, phase: 'P15' },
  scripts: { windows: 5, phase: 'P15' },
  concepts: { windows: 3, phase: 'P16' },
  maps: { windows: 3, phase: 'P16' },
  review: { windows: 3, phase: 'P13' },
  changes: { windows: 4, phase: 'P13' },
  code: { windows: 2, phase: 'P18' },
  settings: { windows: 4, phase: 'P14' },
};

// AC: CE-32.1
test('every placeholder preset has its windows and names its phase', () => {
  for (const [id, expected] of Object.entries(PLACEHOLDERS)) {
    const preset = presets[id];
    expect(preset?.id, `preset ${id}`).toBe(id);
    const windows = windowsOf(preset?.windows ?? { dir: 'row', items: [] });
    expect(windows, `windows of ${id}`).toHaveLength(expected.windows);
    for (const window of windows) {
      expect(phaseOf(window), `phase of ${window.id}`).toBe(expected.phase);
      expect(window.tabs.length, `tabs of ${window.id}`).toBeGreaterThan(0);
    }
  }
});

test('the presets are the placeholders and the Codex', () => {
  expect(Object.keys(presets).sort()).toEqual([...Object.keys(PLACEHOLDERS), 'codex'].sort());
});

test('window ids are unique across all presets', () => {
  const ids = Object.values(presets).flatMap((preset) => windowsOf(preset.windows).map((w) => w.id));
  expect(new Set(ids).size).toBe(ids.length);
});
