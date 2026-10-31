import type { Page } from '@playwright/test';
import { readCorpusText, type CorpusFile } from '../../src/harness/manifest.ts';
import type { ScriptedEdit, VariantInfo } from '../../src/harness/types.ts';
import { variants } from '../../src/harness/variants.ts';

export { record, type RecordKind } from './record.ts';

/**
 * The variants a browser suite runs: `P10_VARIANTS=a,b` names them, otherwise every registered
 * variant except the baseline. The baseline only runs when it is named.
 */
export function variantsUnderTest(): VariantInfo[] {
  const named = (process.env.P10_VARIANTS ?? '')
    .split(',')
    .map((name) => name.trim())
    .filter((name) => name !== '');
  if (named.length === 0) return variants.filter((variant) => variant.baseline !== true);
  const unknown = named.filter((name) => !variants.some((variant) => variant.name === name));
  if (unknown.length > 0) {
    throw new Error(
      `P10_VARIANTS names unknown variants: ${unknown.join(', ')} (registered: ${variants.map((v) => v.name).join(', ')})`,
    );
  }
  return variants.filter((variant) => named.includes(variant.name));
}

/** `P10_STRICT=1`: a byte mismatch or a failed feature check fails its test instead of being recorded. */
export function strict(): boolean {
  return process.env.P10_STRICT === '1';
}

/**
 * Open the harness (once per page) and load the corpus file into the variant. Returns the text
 * exactly as the file holds it, so the caller can build the oracle from it.
 */
export async function loadFile(page: Page, variant: string, file: CorpusFile): Promise<string> {
  const text = readCorpusText(file);
  if (page.url() === 'about:blank') await page.goto('/');
  await page.waitForFunction(() => window.__harness !== undefined);
  await page.evaluate(([name, content, fileId]) => window.__harness!.load(name, content, fileId), [
    variant,
    text,
    file.path,
  ] as const);
  return text;
}

/** How a scripted edit gets into the editor: its own commands (`api`) or real input (`input`). */
export type EditDriver = 'api' | 'input';
/** `original` is the file's text as `loadFile` returned it; a driver that needs offsets resolves anchors against it. */
type DriverFunction = (page: Page, edit: ScriptedEdit, original?: string) => Promise<void>;

const drivers: Partial<Record<EditDriver, DriverFunction>> = {
  api: async (page, scripted) => {
    await page.evaluate((payload) => window.__harness!.apply(payload), scripted);
  },
};

/** Plug in a driver; `drivers.ts` registers `input` this way, so this file needs no change. */
export function registerDriver(name: EditDriver, driver: DriverFunction): void {
  drivers[name] = driver;
}

/**
 * Run a scripted edit on the loaded editor through the named driver. Pass `original` (what
 * `loadFile` returned) so the offsets come from the file and not from what the editor shows.
 */
export async function edit(
  page: Page,
  scripted: ScriptedEdit,
  driver: EditDriver = 'api',
  original?: string,
): Promise<void> {
  const run = drivers[driver];
  if (run === undefined) {
    throw new Error(`helpers: no "${driver}" driver is registered (import ./drivers.ts first)`);
  }
  await run(page, scripted, original);
}
