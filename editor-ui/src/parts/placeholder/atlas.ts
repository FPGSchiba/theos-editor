// Atlas (P25), from reference screen 12-atlas: the floor's records, the floor map with its layers,
// and the place panel.

import { row, type Preset } from '../layout';
import { empty } from './empty';

const phase = 'P25';

export const atlas: Preset = {
  id: 'atlas',
  windows: row(
    [
      '0 0 236px',
      empty('atlas-contents', ['On this floor', 'Unplaced'], {
        mark: '⊟',
        text: 'The records placed on this floor, and the ones that still need a place. Drag an unplaced record onto the map to pin it.',
        phase,
      }),
    ],
    [
      '1 1 600px',
      empty('atlas-map', ['Floor map'], {
        mark: '⌾',
        text: "The floor's map, with layers for terrain, places and routes, and the places pinned on it. Pick a floor to see its map.",
        phase,
      }),
    ],
    [
      '0 0 330px',
      empty('atlas-place', ['Place', 'Pin'], {
        mark: '⌂',
        text: 'The place you pick on the map: the lore, people and quests found there, and what lies nearby.',
        phase,
      }),
    ],
  ),
};
