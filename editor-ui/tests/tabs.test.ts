// The Codex tabs (P11 §5.11): the commands Next tab, Previous tab and Close tab work on the record
// tabs, and the tree-tab commands on the tree tab. With too few tabs they do nothing.

import { beforeEach, expect, test } from 'vitest';

import {
  closeRecordTab,
  closeTab,
  codexTabs,
  nextTab,
  openRecordTab,
  previousTab,
  setTreeTab,
} from '../src/parts/codex/tabs.svelte';

beforeEach(() => {
  codexTabs.records = [];
  codexTabs.active = -1;
  codexTabs.tree = 'where';
});

// AC: CE-32.2
test('close tab closes the active record tab and activates its neighbour', () => {
  openRecordTab('hearth', 'Hearth');
  openRecordTab('tower', 'The Tower');
  openRecordTab('reed-marsh', 'Reed Marsh');
  codexTabs.active = 1;
  closeTab();
  expect(codexTabs.records.map((tab) => tab.id)).toEqual(['hearth', 'reed-marsh']);
  expect(codexTabs.active).toBe(1);
  closeTab();
  expect(codexTabs.active).toBe(0);
  closeTab();
  expect(codexTabs.records).toEqual([]);
  expect(codexTabs.active).toBe(-1);
});

// AC: CE-32.2
test('next and previous tab wrap around', () => {
  openRecordTab('hearth', 'Hearth');
  openRecordTab('tower', 'The Tower');
  expect(codexTabs.active).toBe(1);
  nextTab();
  expect(codexTabs.active).toBe(0);
  previousTab();
  expect(codexTabs.active).toBe(1);
});

// AC: CE-32.2
test('with one tab or none the tab commands do nothing', () => {
  expect(() => closeTab()).not.toThrow();
  expect(() => nextTab()).not.toThrow();
  expect(codexTabs.active).toBe(-1);
  openRecordTab('hearth', 'Hearth');
  nextTab();
  previousTab();
  expect(codexTabs.active).toBe(0);
});

test('opening an open record focuses its tab', () => {
  openRecordTab('hearth', 'Hearth');
  openRecordTab('tower', 'The Tower');
  openRecordTab('hearth', 'Hearth');
  expect(codexTabs.records).toHaveLength(2);
  expect(codexTabs.active).toBe(0);
});

test('the tree tab is set by name', () => {
  setTreeTab('lore');
  expect(codexTabs.tree).toBe('lore');
});

test('closing another record tab keeps the active record active', () => {
  openRecordTab('hearth', 'Hearth');
  openRecordTab('tower', 'The Tower');
  openRecordTab('reed-marsh', 'Reed Marsh');
  codexTabs.active = 2;
  closeRecordTab('hearth');
  expect(codexTabs.records.map((tab) => tab.id)).toEqual(['tower', 'reed-marsh']);
  expect(codexTabs.active).toBe(1);
  closeRecordTab('nowhere');
  expect(codexTabs.records).toHaveLength(2);
  closeRecordTab('reed-marsh');
  expect(codexTabs.active).toBe(0);
});
