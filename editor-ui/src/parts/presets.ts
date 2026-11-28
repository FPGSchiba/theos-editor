// The layout presets, one per rail entry (P11 §5.6). A new preset is one file and one line here.

import type { RailEntry } from '../lib/ipc/bindings';
import { codex } from './codex/preset';
import { row, type Preset } from './layout';
import { atlas } from './placeholder/atlas';
import { changes } from './placeholder/changes';
import { code } from './placeholder/code';
import { concepts } from './placeholder/concepts';
import { empty } from './placeholder/empty';
import { graph } from './placeholder/graph';
import { maps } from './placeholder/maps';
import { review } from './placeholder/review';
import { scripts } from './placeholder/scripts';
import { settings } from './placeholder/settings';

export type { Preset, Split, WindowSpec } from './layout';

/** The presets by rail entry id. */
export const presets: Record<string, Preset> = {
  codex,
  atlas,
  graph,
  scripts,
  concepts,
  maps,
  review,
  changes,
  code,
  settings,
};

/**
 * The preset of the rail entry `entry`. An entry this UI has no preset for (a part newer than the
 * UI) gets one empty window that names it, and its phase when it has one.
 */
export function presetFor(entry: RailEntry): Preset {
  return (
    presets[entry.id] ?? {
      id: entry.id,
      windows: row([
        '1 1 auto',
        empty(entry.id, [entry.label], {
          mark: entry.glyph,
          text: `${entry.label} has no layout in this build of the editor.`,
          ...(entry.phase === null ? {} : { phase: entry.phase }),
        }),
      ]),
    }
  );
}
