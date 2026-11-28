// Concepts (P16), from reference screen 06-concepts: the concepts, the board with the versions and
// the pipeline, and the version's decision and notes.

import { row, type Preset } from '../layout';
import { empty } from './empty';

const phase = 'P16';

export const concepts: Preset = {
  id: 'concepts',
  windows: row(
    [
      '0 0 236px',
      empty('concepts-list', ['Concepts'], {
        mark: '◫',
        text: "The project's concepts, with how many versions each has and which one is picked.",
        phase,
      }),
    ],
    [
      '1 1 600px',
      empty('concepts-board', ['Concept'], {
        mark: '◫',
        text: "A concept's versions side by side, and the pipeline that makes them: compose, compare, finish, pick. Pick a concept to see its versions.",
        phase,
      }),
    ],
    [
      '0 0 318px',
      empty('concepts-version', ['Version', 'Decisions'], {
        mark: '◫',
        text: 'The version you pick: its decision, the notes on it, and the decisions made so far.',
        phase,
      }),
    ],
  ),
};
