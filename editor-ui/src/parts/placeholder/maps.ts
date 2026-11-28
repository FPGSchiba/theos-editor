// Maps (P16), from reference screen 11-maps: the maps and their recipes, the runs with their
// results per seed, and the seed's notes.

import { row, type Preset } from '../layout';
import { empty } from './empty';

const phase = 'P16';

export const maps: Preset = {
  id: 'maps',
  windows: row(
    [
      '0 0 236px',
      empty('maps-list', ['Maps', 'Recipes', 'Stages'], {
        mark: '△',
        text: "The project's maps, the recipes that generate them and the stages a recipe runs.",
        phase,
      }),
    ],
    [
      '1 1 580px',
      empty('maps-runs', ['Runs', 'Recipe', 'Stage catalogue'], {
        mark: '△',
        text: 'The runs of a recipe, one result per seed, side by side. Pick the seed that fits the floor; nothing is kept until you pick.',
        phase,
      }),
    ],
    [
      '0 0 330px',
      empty('maps-seed', ['Seed', 'Engine package'], {
        mark: '⊛',
        text: 'The seed you pick: the constraints it met, the places it adds to the Codex, and your notes on it.',
        phase,
      }),
    ],
  ),
};
