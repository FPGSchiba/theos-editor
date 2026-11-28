// The editor's commands (P11 §5.11): what each does, its label, and whether it can run now. The
// keys are in the keymap (keymap.ts); the palette lists every command, and the shell's one key
// listener runs them. A new command is one entry here and one line in the keymap.

import type { TreeTab } from './ipc/bindings';
import type { CommandId, RailSlot } from './keymap';
import { activeEntry, railOrder } from './rail';
import { focusFindField } from './state/find-field';
import { togglePalette } from './state/palette.svelte';
import { activate, chooseProject, CODEX, project, reloadProject } from './state/project.svelte';
import {
  closeTab,
  codexTabs,
  nextTab,
  previousTab,
  setTreeTab,
  TREE_TABS,
} from '../parts/codex/tabs.svelte';

/** A command of the editor. */
export interface Command {
  /** The label in the palette. */
  label: string;
  /** Does what the command does; only called when `enabled()` holds. */
  run: () => void;
  /** Whether the command can run now. */
  enabled: () => boolean;
  /** The phase that builds the command, while it is not built. */
  phase?: string;
}

/** The rail entry in place `slot` of the rail order (slot 1 is the first). */
function railCommand(slot: RailSlot): Command {
  const entry = () => (project.view === null ? undefined : railOrder(project.view.rail)[slot - 1]);
  return {
    label: `Rail entry ${slot}`,
    run: () => {
      const id = entry()?.id;
      if (id !== undefined) {
        activate(id);
      }
    },
    enabled: () => entry() !== undefined,
  };
}

/** Shows the Codex with the tree tab `tab`. */
function treeCommand(tab: TreeTab): Command {
  const title = TREE_TABS.find((entry) => entry.tab === tab)?.title ?? tab;
  return {
    label: `Tree tab: ${title}`,
    run: () => {
      activate(CODEX);
      setTreeTab(tab);
    },
    enabled: () => project.view?.rail.some((entry) => entry.id === CODEX) ?? false,
  };
}

/** Whether the Codex's layout is the one shown, so its tab commands act on what is seen. */
function codexShown(): boolean {
  return project.view !== null && activeEntry(project.view.rail, project.active)?.id === CODEX;
}

const always = (): boolean => true;
const never = (): boolean => false;
const nothing = (): void => undefined;

/** Every command, by id. */
export const commands: Record<CommandId, Command> = {
  'palette.open': { label: 'Command palette', run: togglePalette, enabled: always },
  'record.find': { label: 'Find record', run: focusFindField, enabled: always },
  'project.open': {
    label: 'Open project…',
    run: () => void chooseProject(),
    enabled: () => !project.opening,
  },
  'project.reload': {
    label: 'Reload project',
    run: () => void reloadProject('reload'),
    enabled: () => project.view !== null,
  },
  'rail.1': railCommand(1),
  'rail.2': railCommand(2),
  'rail.3': railCommand(3),
  'rail.4': railCommand(4),
  'rail.5': railCommand(5),
  'rail.6': railCommand(6),
  'rail.7': railCommand(7),
  'rail.8': railCommand(8),
  'rail.9': railCommand(9),
  'tab.next': {
    label: 'Next tab',
    run: nextTab,
    enabled: () => codexShown() && codexTabs.records.length > 1,
  },
  'tab.previous': {
    label: 'Previous tab',
    run: previousTab,
    enabled: () => codexShown() && codexTabs.records.length > 1,
  },
  'tab.close': {
    label: 'Close tab',
    run: closeTab,
    enabled: () => codexShown() && codexTabs.active >= 0,
  },
  'tree.where': treeCommand('where'),
  'tree.who': treeCommand('who'),
  'tree.lore': treeCommand('lore'),
  'tree.things': treeCommand('things'),
  'file.save': { label: 'Save', run: nothing, enabled: never, phase: 'P12' },
};

/** Runs the command `id` when it is enabled; else does nothing. */
export function runCommand(id: CommandId): void {
  const command = commands[id];
  if (command.enabled()) {
    command.run();
  }
}
