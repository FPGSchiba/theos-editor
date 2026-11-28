// Scripts (P15), from reference screen 04-script: the scripts, the beats of the open script, and
// the inspector with the beat and its cast, the reveals ledger and validation.

import { column, row, type Preset } from '../layout';
import { empty } from './empty';

const phase = 'P15';

export const scripts: Preset = {
  id: 'scripts',
  windows: row(
    [
      '0 0 232px',
      empty('scripts-list', ['Scripts', 'Chronicles'], {
        mark: '¶',
        text: "The project's scripts and chronicles, with the outline of the open one. Until then, a script opens in the Codex as a record.",
        phase,
      }),
    ],
    [
      '1 1 580px',
      empty('scripts-beats', ['Script'], {
        mark: '¶',
        text: 'The beats of a script, each with where it happens, who is there and what it reveals. Pick a script to write its beats.',
        phase,
      }),
    ],
    [
      '0 0 316px',
      column(
        [
          '1 1 0',
          empty('scripts-beat', ['Beat'], {
            mark: '◉',
            text: 'The beat you pick: its place, its cast and when it happens.',
            phase,
          }),
        ],
        [
          '1 1 0',
          empty('scripts-reveals', ['Reveals'], {
            mark: '◔',
            text: 'The ledger of what the script reveals, generated from its beats and checked against the truths they touch.',
            phase,
          }),
        ],
        [
          '1 1 0',
          empty('scripts-validation', ['Validation'], {
            mark: '✓',
            text: "The script's findings, such as a beat that confirms a truth that must stay unproven.",
            phase,
          }),
        ],
      ),
    ],
  ),
};
