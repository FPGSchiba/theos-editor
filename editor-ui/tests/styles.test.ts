// Tokens only (P11 §5.7): a component names no colour, font or size; it uses `var(--…)` tokens
// and `tu-*` classes. ESLint and svelte-check cannot see CSS, so this test reads the CSS of every
// `.svelte` file: its `<style>` block, its `style=` attributes and its `style:` directives.

import { globSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from 'vitest';

import { findStyleViolations } from './support/style-rules';

const PACKAGE = fileURLToPath(new URL('..', import.meta.url));
const SOURCES = globSync('src/**/*.svelte', { cwd: PACKAGE });

test('every component uses tokens only', () => {
  const violations = SOURCES.flatMap((file) =>
    findStyleViolations(readFileSync(join(PACKAGE, file), 'utf8')).map((v) => `${file}: ${v}`),
  );
  expect(violations).toEqual([]);
});

test('the test reads at least one component', () => {
  expect(SOURCES.length).toBeGreaterThan(0);
});

const FIXTURE = `<script lang="ts">
  let on = $state(false);
</script>

<p class="a" style="color: rgb(1, 2, 3)">one</p>
<p style:background="hsl(1 2% 3%)" class:on>two</p>

<style>
  .a { color: #ff0000; }
  .b { border-color: rebeccapurple; }
  .c { font-family: Inter, sans-serif; }
  .d { opacity: 0.5; }
  .e { font-size: 12px; }
  @media (min-width: 10px) { .f { background: red; } }
</style>
`;

test('a fixture with one of each breach fails on each', () => {
  const violations = findStyleViolations(FIXTURE);
  expect(violations).toEqual([
    'style: `color: rgb(1, 2, 3)` names a colour (`rgb(`); use a var(--…) token',
    'style:background: `background: hsl(1 2% 3%)` names a colour (`hsl(`); use a var(--…) token',
    '<style>: `color: #ff0000` names a colour (`#ff0000`); use a var(--…) token',
    '<style>: `border-color: rebeccapurple` names a colour (`rebeccapurple`); use a var(--…) token',
    '<style>: `font-family: Inter, sans-serif` is not a var(--…) token',
    '<style>: `opacity: 0.5` fades text; use a dimmer colour token',
    '<style>: `font-size: 12px` is a px font size; use a token or a tu-* class',
    '<style>: `background: red` names a colour (`red`); use a var(--…) token',
  ]);
});

test('tokens, keywords and var() names are allowed', () => {
  const clean = `<div style="color: var(--text)"></div>
<style>
  .a { color: var(--red); background: transparent; border: 1px solid currentColor; }
  .b { font-family: var(--font-ui); font-size: var(--size-body); width: 248px; }
  .c { grid-template-columns: 248px 1fr 320px; }
</style>
`;
  expect(findStyleViolations(clean)).toEqual([]);
});
