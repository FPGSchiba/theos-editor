// Settings (P14), from reference screen 10-settings: the pages, the values of a page, and where
// each value is stored, with its layers.

import { column, row, type Preset } from '../layout';
import { empty } from './empty';

const phase = 'P14';

export const settings: Preset = {
  id: 'settings',
  windows: row(
    [
      '0 0 236px',
      empty('settings-pages', ['Settings'], {
        mark: '⚙',
        text: 'The pages of settings, one for each part of the editor.',
        phase,
      }),
    ],
    [
      '1 1 560px',
      empty('settings-page', ['Page'], {
        mark: '⚙',
        text: "The values of the page you pick, each showing whether it comes from the part's default, from you or from the project.",
        phase,
      }),
    ],
    [
      '0 0 320px',
      column(
        [
          '1 1 0',
          empty('settings-files', ['Project file', 'User file'], {
            mark: '⌸',
            text: 'The file a value is stored in: the project file is shared through git, the user file stays on this computer.',
            phase,
          }),
        ],
        [
          '1 1 0',
          empty('settings-layers', ['How settings stack'], {
            mark: '≣',
            text: 'The three layers, part default, user and project, and which one wins for the value you pick.',
            phase,
          }),
        ],
      ),
    ],
  ),
};
