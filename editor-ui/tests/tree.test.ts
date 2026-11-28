// The Codex tree's state (P11 §5.8): folding rows and unfolding the rows above an opened record.

import { beforeEach, expect, test } from 'vitest';

import type { RecordRef, TreeNode } from '../src/lib/ipc/bindings';
import { expandTo, nodeKey, toggleFold, tree } from '../src/parts/codex/tree.svelte';

/** A synthetic record reference. */
function ref(id: string, kind: string): RecordRef {
  return { id, kind, path: null, title: id, status: null, never: false, locked: [] };
}

/** A synthetic record row with `children`. */
function node(id: string, kind: string, children: TreeNode[] = []): TreeNode {
  return { type: 'record', record: ref(id, kind), tag: null, children };
}

const NODES: TreeNode[] = [
  node('tower', 'tower', [node('hearth', 'band', [node('f04', 'floor', [node('shrine', 'location')])])]),
  { type: 'group', label: 'Truths', children: [node('T-012', 'truth')] },
];

beforeEach(() => {
  tree.folded = [];
});

test('a row folds and unfolds', () => {
  toggleFold('hearth');
  expect(tree.folded).toEqual(['hearth']);
  toggleFold('hearth');
  expect(tree.folded).toEqual([]);
});

test('opening a record unfolds only the rows above it', () => {
  tree.folded = ['tower', 'hearth', 'f04', 'shrine', 'group:Truths'];
  expandTo(NODES, 'shrine');
  expect(tree.folded).toEqual(['shrine', 'group:Truths']);
  expandTo(NODES, 'T-012');
  expect(tree.folded).toEqual(['shrine']);
});

test('a record that is not in the tree unfolds nothing', () => {
  tree.folded = ['tower'];
  expandTo(NODES, 'S-004');
  expect(tree.folded).toEqual(['tower']);
});

test('a group row is keyed by its label', () => {
  const group = NODES[1];
  expect(group === undefined ? null : nodeKey(group)).toBe('group:Truths');
});

test('two files with one id are two rows, keyed and unfolded by their paths', () => {
  const twin = (path: string): TreeNode => ({
    type: 'record',
    record: { ...ref('synthetic-twin', 'location'), path },
    tag: null,
    children: [],
  });
  const nodes: TreeNode[] = [
    node('tower', 'tower', [twin('world/synthetic-twin.md')]),
    { type: 'group', label: 'Copies', children: [twin('world/synthetic-copy/synthetic-twin.md')] },
  ];
  const keys = nodes.flatMap((top) => top.children.map(nodeKey));
  expect(keys).toEqual(['world/synthetic-twin.md', 'world/synthetic-copy/synthetic-twin.md']);
  tree.folded = ['tower', 'group:Copies'];
  expandTo(nodes, 'world/synthetic-copy/synthetic-twin.md');
  expect(tree.folded).toEqual(['tower']);
});
