import type { Locator, Page } from '@playwright/test';
import { resolveAnchor } from '../../src/harness/anchor.ts';
import { editOffset, expectedAfter, NOTE_STATE, toLf } from '../../src/harness/oracle.ts';
import type { ScriptedEdit } from '../../src/harness/types.ts';
import { revealElement } from './checks.ts';
import { registerDriver, type EditDriver } from './helpers.ts';

// The `input` driver: a scripted edit as a person would make it, with the keyboard and the mouse
// on the rendered page. The `api` driver (the editor's own `apply`) lives in helpers.ts.
// Importing this file registers `input`; every spec that runs `edit(…, 'input')` imports it.

/** How long a rendered feature (popover, beat row) may take to show up before it counts as missing. */
const FEATURE_WAIT_MS = 3_000;
/** The most `undo` presses `undo-all` makes before it gives up. */
const MAX_UNDO = 50;

const readText = (page: Page): Promise<string> => page.evaluate(() => window.__harness!.text());

/** Focus the editor and put the caret at `offset` (into the LF form of the file). */
async function caretAt(page: Page, offset: number): Promise<void> {
  await page.evaluate((at) => {
    const harness = window.__harness!;
    harness.focus();
    harness.caretTo(at);
  }, offset);
}

async function pressTimes(page: Page, key: string, times: number): Promise<void> {
  for (let n = 0; n < times; n++) await page.keyboard.press(key);
}

/**
 * Wait for a feature the variant has to render. Without it the edit cannot be made by input;
 * the error starts with "feature missing", which is how the report tells it from a mismatch.
 */
async function need(locator: Locator, what: string): Promise<void> {
  try {
    await locator.first().waitFor({ state: 'attached', timeout: FEATURE_WAIT_MS });
  } catch {
    throw new Error(`feature missing: ${what} is not rendered`);
  }
}

const attr = (value: string | number): string => JSON.stringify(String(value));

/** `beat-param`: click the parameter (or "+ add" and the key), type the value, Enter. */
async function beatParam(page: Page, edit: ScriptedEdit & { kind: 'beat-param' }): Promise<void> {
  const rowSelector = `[data-beat-row=${attr(edit.beat)}]`;
  // A viewport editor draws only the rows near the window: scroll to the row, as a person would.
  // Never move the caret there (the beat would turn raw and the row would go).
  const reveal = async (selector: string, what: string): Promise<void> => {
    try {
      await revealElement(page, selector, FEATURE_WAIT_MS);
    } catch {
      throw new Error(`feature missing: ${what} is not rendered`);
    }
  };
  await reveal(rowSelector, `the beat row of beat ${edit.beat}`);
  const row = page.locator(rowSelector).first();
  // A row may be mounted before its params; whether the key exists is only known once they are.
  await reveal(`${rowSelector} [data-beat-param], ${rowSelector} [data-beat-add]`, `the parameters of beat ${edit.beat}`);
  const paramSelector = `${rowSelector} [data-beat-param=${attr(edit.key)}]`;
  const existing = row.locator(`[data-beat-param=${attr(edit.key)}]`).first();
  if ((await existing.count()) > 0) {
    await reveal(paramSelector, `the parameter ${edit.key} of beat ${edit.beat}`);
    // The key label is outside any chip; a click on the middle of the param may land on one.
    const label = existing.locator('.tu-beat__key').first();
    await ((await label.count()) > 0 ? label : existing).click({ timeout: FEATURE_WAIT_MS });
  } else {
    await reveal(`${rowSelector} [data-beat-add]`, `the add button of beat ${edit.beat}`);
    await row.locator('[data-beat-add]').first().click({ timeout: FEATURE_WAIT_MS });
    await reveal(`${rowSelector} [data-beat-add-key=${attr(edit.key)}]`, `the key choice ${edit.key}`);
    await row.locator(`[data-beat-add-key=${attr(edit.key)}]`).first().click({ timeout: FEATURE_WAIT_MS });
  }
  await reveal(`${rowSelector} [data-beat-input]`, `the value field of beat ${edit.beat}`);
  const field = row.locator('[data-beat-input]').first();
  await field.fill(edit.value, { timeout: FEATURE_WAIT_MS });
  await field.press('Enter');
}

/** `note-state`: the caret on the state segment of the title line, Shift+End, type the new state. */
async function noteState(page: Page, source: string, edit: ScriptedEdit & { kind: 'note-state' }): Promise<void> {
  const lineStart = editOffset(source, edit);
  if (lineStart === undefined) throw new Error('note-state: the oracle found no title line');
  const newline = source.indexOf('\n', lineStart);
  const lineEnd = newline === -1 ? source.length : newline;
  const line = source.slice(lineStart, lineEnd);
  const separator = line.lastIndexOf(' · ');
  const last = separator === -1 ? undefined : line.slice(separator + 3);
  if (last !== undefined && NOTE_STATE.test(last)) {
    await caretAt(page, lineStart + separator + 3);
    await page.keyboard.press('Shift+End');
    await page.keyboard.type(edit.state);
  } else {
    // A title without a state gets one appended (as the oracle does).
    await caretAt(page, lineEnd);
    await page.keyboard.type(` · ${edit.state}`);
  }
}

/** `link-insert`: type `[[` and the query, then click the record in the popover. */
async function linkInsert(page: Page, source: string, edit: ScriptedEdit & { kind: 'link-insert' }): Promise<void> {
  await caretAt(page, resolveAnchor(source, edit.at));
  await page.keyboard.type(`[[${edit.query}`);
  await need(page.locator('[data-link-popover]'), 'the [[ popover');
  await page.locator(`[data-link-option=${attr(edit.pick)}]`).first().click({ timeout: FEATURE_WAIT_MS });
}

/** `paste`: a `paste` event with the clipboard holding `text/plain` (and `text/html`) at the caret. */
async function paste(page: Page, plain: string, html: string | undefined): Promise<void> {
  await page.evaluate(
    ({ plainText, htmlText }) => {
      const target = document.activeElement ?? document.body;
      const data = new DataTransfer();
      data.setData('text/plain', plainText);
      if (htmlText !== undefined) data.setData('text/html', htmlText);
      target.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
    },
    { plainText: plain, htmlText: html },
  );
}

/** Press `ControlOrMeta+z` until the text stops changing, at most MAX_UNDO times. */
async function undoUntilStable(page: Page): Promise<void> {
  let text = await readText(page);
  for (let n = 0; n < MAX_UNDO; n++) {
    await page.keyboard.press('ControlOrMeta+z');
    const next = await readText(page);
    if (next === text) return;
    text = next;
  }
}

async function input(page: Page, edit: ScriptedEdit, original?: string): Promise<void> {
  // Offsets come from the file; what the editor shows may already differ from it.
  const source = toLf(original ?? (await readText(page)));
  switch (edit.kind) {
    case 'insert':
      await caretAt(page, resolveAnchor(source, edit.at));
      await page.keyboard.type(edit.text);
      return;
    case 'delete':
      await caretAt(page, resolveAnchor(source, edit.at));
      await pressTimes(page, edit.backward === true ? 'Backspace' : 'Delete', edit.length);
      return;
    case 'beat-param':
      await beatParam(page, edit);
      return;
    case 'note-state':
      await noteState(page, source, edit);
      return;
    case 'link-insert':
      await linkInsert(page, source, edit);
      return;
    case 'link-delete': {
      // The caret goes behind the closing "]]"; one Backspace takes the whole chip.
      const close = source.indexOf(']]', resolveAnchor(source, edit.link) + 2);
      await caretAt(page, close + 2);
      await page.keyboard.press('Backspace');
      return;
    }
    case 'select-all-type':
      await page.evaluate(() => window.__harness!.focus());
      await page.keyboard.press('ControlOrMeta+a');
      await page.keyboard.type(edit.text);
      return;
    case 'paste':
      await caretAt(page, resolveAnchor(source, edit.at));
      await paste(page, edit.plain, edit.html);
      return;
    case 'external': {
      // The local edit comes in as one text insertion (one input event, so one undo step in
      // every editor); the external one is the editor's own `applyExternal`, at the offset the
      // original anchor has once the local insert is in.
      const local = resolveAnchor(source, edit.local.at);
      const external = resolveAnchor(source, edit.external.at);
      await caretAt(page, local);
      await page.keyboard.insertText(edit.local.text);
      const at = external > local ? external + edit.local.text.length : external;
      await page.evaluate(
        (change) => window.__harness!.applyExternal([change]),
        { from: at, to: at, insert: edit.external.text },
      );
      return;
    }
    case 'undo-all': {
      // Every step is made on the text the previous one left, then everything is taken back.
      let model = source;
      for (const step of edit.edits) {
        await input(page, step, model);
        model = expectedAfter(model, step);
      }
      await undoUntilStable(page);
      return;
    }
  }
}

/** One undo, as the driver would do it: the keyboard for `input`, the editor's `undo` for `api`. */
export async function undoOnce(page: Page, driver: EditDriver): Promise<void> {
  if (driver === 'input') await page.keyboard.press('ControlOrMeta+z');
  else await page.evaluate(() => window.__harness!.undo());
}

registerDriver('input', input);
