// Review (P13), from reference screen 07-review: the sessions and their proposals, the proposal,
// and the agent's session with the findings.

import { row, type Preset } from '../layout';
import { empty } from './empty';

const phase = 'P13';

export const review: Preset = {
  id: 'review',
  windows: row(
    [
      '0 0 400px',
      empty('review-queue', ['Review'], {
        mark: '◴',
        text: 'Agent proposals land here, grouped by session. Nothing changes the world until you approve it.',
        phase,
      }),
    ],
    [
      '1 1 520px',
      empty('review-proposal', ['Preview', 'Diff', 'Context the agent read'], {
        mark: '±',
        text: 'The proposal you pick: the record as it would read, the diff of its file, and what the agent read to write it.',
        phase,
      }),
    ],
    [
      '0 0 330px',
      empty('review-session', ['Agent', 'MCP tools'], {
        mark: '✓',
        text: "The agent's session: your brief, its scope, its log, and the validator's findings on the proposal.",
        phase,
      }),
    ],
  ),
};
