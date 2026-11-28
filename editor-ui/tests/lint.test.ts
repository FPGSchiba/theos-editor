// The ESLint configuration keeps hand-written IPC and HTML out of the UI: `invoke` only inside
// src/lib/ipc/ (P11 §5.5), and no `{@html}` (P11 constraint "No HTML from records").

import { ESLint } from 'eslint';
import { expect, test } from 'vitest';

const eslint = new ESLint({ cwd: process.cwd() });

const IMPORT_INVOKE = `import { invoke } from '@tauri-apps/api/core';

export const ping = (): Promise<unknown> => invoke('ping');
`;

/** The rule ids of the errors ESLint reports for `code` as if it were the file `filePath`. */
async function errors(code: string, filePath: string): Promise<string[]> {
  const [result] = await eslint.lintText(code, { filePath });
  return (result?.messages ?? [])
    .filter((m) => m.severity === 2)
    .map((m) => m.ruleId ?? `fatal: ${m.message}`);
}

// AC: CE-32.4
test('invoke outside src/lib/ipc/ is a no-restricted-imports error', async () => {
  expect(await errors(IMPORT_INVOKE, 'src/shell/Ping.ts')).toEqual(['no-restricted-imports']);
  const svelte = `<script lang="ts">\n${IMPORT_INVOKE}</script>\n`;
  expect(await errors(svelte, 'src/shell/Ping.svelte')).toEqual(['no-restricted-imports']);
});

// AC: CE-32.4
test('invoke inside src/lib/ipc/ is allowed', async () => {
  expect(await errors(IMPORT_INVOKE, 'src/lib/ipc/ping.ts')).toEqual([]);
});

test('{@html} is a svelte/no-at-html-tags error', async () => {
  const code = `<script lang="ts">\n  let { text }: { text: string } = $props();\n</script>\n\n<p>{@html text}</p>\n`;
  expect(await errors(code, 'src/shell/Raw.svelte')).toEqual(['svelte/no-at-html-tags']);
});
