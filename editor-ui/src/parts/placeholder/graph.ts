// Graph (P15), from reference screen 05-graph: the filters, the graph and the record in focus.

import { row, type Preset } from '../layout';
import { empty } from './empty';

const phase = 'P15';

export const graph: Preset = {
  id: 'graph',
  windows: row(
    [
      '0 0 210px',
      empty('graph-filters', ['Show'], {
        mark: '⋯',
        text: 'Filters for the graph: which kinds and which links it draws, and how far it reaches from the record in focus.',
        phase,
      }),
    ],
    [
      '1 1 640px',
      empty('graph-view', ['Graph', 'Ledger'], {
        mark: '⋈',
        text: 'The links around a record, drawn as a graph you can walk, and the ledger of what each floor reveals. Open a record and choose Open in graph to start from it.',
        phase,
      }),
    ],
    [
      '0 0 300px',
      empty('graph-focus', ['Focus', 'Paths'], {
        mark: '◈',
        text: 'The record in focus: its links in and out, the paths to other records, and the gaps the graph finds around it.',
        phase,
      }),
    ],
  ),
};
