// Code (P18): the project's file tree and a code view. It has no reference screen yet.

import { row, type Preset } from '../layout';
import { empty } from './empty';

const phase = 'P18';

export const code: Preset = {
  id: 'code',
  windows: row(
    [
      '0 0 248px',
      empty('code-files', ['Files'], {
        mark: '</>',
        text: "The project's files that are not records, such as recipes and schemas.",
        phase,
      }),
    ],
    [
      '1 1 640px',
      empty('code-view', ['Code'], {
        mark: '</>',
        text: 'The file you pick, with its syntax highlighted. Pick a file in the tree to open it.',
        phase,
      }),
    ],
  ),
};
