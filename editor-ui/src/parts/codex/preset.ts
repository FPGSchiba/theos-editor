// The Codex preset (P11 §5.8): screen 01's tree (248 px), record window and side column (320 px)
// with Front matter / Links in, Used by and Validation. The tree tab, the record tabs and the side
// tab live in tabs.svelte.ts; the windows read them through their selections and tab getters.

import type { RecordPage } from '../../lib/ipc/bindings';
import { column, row, windowOf, type Preset, type TabTitle } from '../layout';
import CodexTree from './CodexTree.svelte';
import FrontMatter from './FrontMatter.svelte';
import { pageOf } from './pages.svelte';
import RecordWindow from './RecordWindow.svelte';
import {
  activeKey,
  recordSelection,
  recordTabTitles,
  sideSelection,
  TREE_TABS,
  treeSelection,
} from './tabs.svelte';
import UsedBy from './UsedBy.svelte';
import Validation from './Validation.svelte';

/** The Validation title: with the findings' count, red when one is an error, else none. */
function validationTitle(page: RecordPage | null): TabTitle {
  const findings = page?.findings ?? [];
  if (findings.length === 0) {
    return 'Validation';
  }
  const signal = findings.some((finding) => finding.severity === 'error') ? 'reject' : 'warn';
  return { title: 'Validation', badge: { count: findings.length, signal } };
}

export const codex: Preset = {
  id: 'codex',
  windows: row(
    [
      '0 0 248px',
      windowOf({
        id: 'codex-tree',
        tabs: TREE_TABS.map((entry) => entry.title),
        component: CodexTree,
        selected: treeSelection,
      }),
    ],
    [
      '1 1 560px',
      windowOf({
        id: 'codex-record',
        get tabs() {
          return recordTabTitles();
        },
        component: RecordWindow,
        selected: recordSelection,
      }),
    ],
    [
      '0 0 320px',
      column(
        [
          '0 1 auto',
          windowOf({
            id: 'codex-front-matter',
            get tabs() {
              const page = pageOf(activeKey());
              return ['Front matter', page === null ? 'Links in' : `Links in · ${page.links_in.length}`];
            },
            component: FrontMatter,
            selected: sideSelection,
          }),
        ],
        ['0 1 auto', windowOf({ id: 'codex-used-by', tabs: ['Used by'], component: UsedBy })],
        [
          '1 1 auto',
          windowOf({
            id: 'codex-validation',
            get tabs() {
              return [validationTitle(pageOf(activeKey()))];
            },
            component: Validation,
          }),
        ],
      ),
    ],
  ),
};
