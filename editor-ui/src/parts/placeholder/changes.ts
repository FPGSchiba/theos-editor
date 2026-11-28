// Changes (P13), from reference screen 08-git: the changes by record, the diff, and the history
// with the branches.

import { column, row, type Preset } from '../layout';
import { empty } from './empty';

const phase = 'P13';

export const changes: Preset = {
  id: 'changes',
  windows: row(
    [
      '0 0 330px',
      empty('changes-list', ['Source control'], {
        mark: '±',
        text: 'The changed records, staged and not staged, and the message of your next commit.',
        phase,
      }),
    ],
    [
      '1 1 520px',
      empty('changes-diff', ['Diff'], {
        mark: '±',
        text: 'The changes of the record you pick, as a readable diff of its front matter and body.',
        phase,
      }),
    ],
    [
      '0 0 320px',
      column(
        [
          '1 1 0',
          empty('changes-history', ['History', 'Who wrote what'], {
            mark: '§',
            text: "The record's history, commit by commit, and who wrote each line.",
            phase,
          }),
        ],
        [
          '1 1 0',
          empty('changes-branches', ['Branches'], {
            mark: '±',
            text: 'Your branches, and how far each one is from its remote.',
            phase,
          }),
        ],
      ),
    ],
  ),
};
