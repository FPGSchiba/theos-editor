import type { CorpusFile } from '../../src/harness/manifest.ts';
import type { VariantInfo } from '../../src/harness/types.ts';
import type { RoundTripRow } from '../e2e/write-reports.ts';

/** The engines of playwright.config.ts. */
export const ENGINES = ['chromium', 'webkit'];
/** The drivers every edit runs through; `unedited` runs once, with the driver `load`. */
const EDIT_DRIVERS = ['api', 'input'] as const;

const keyOf = (variant: string, engine: string, file: string, check: string, driver: string): string =>
  [variant, engine, file, check, driver].join(' | ');

/**
 * The rows `rows` lacks: one for every non-baseline variant × engine × file × (unedited, and every
 * edit with both drivers). Each gap is named as `variant | engine | file | check | driver`.
 */
export function missingRows(rows: RoundTripRow[], variants: VariantInfo[], files: CorpusFile[]): string[] {
  const have = new Set(rows.map((row) => keyOf(row.variant, row.engine, row.file, row.check, row.driver)));
  const missing: string[] = [];
  for (const variant of variants.filter((candidate) => candidate.baseline !== true)) {
    for (const engine of ENGINES) {
      for (const file of files) {
        const wanted = [keyOf(variant.name, engine, file.path, 'unedited', 'load')];
        for (const edit of file.edits) {
          for (const driver of EDIT_DRIVERS) wanted.push(keyOf(variant.name, engine, file.path, edit.id, driver));
        }
        missing.push(...wanted.filter((key) => !have.has(key)));
      }
    }
  }
  return missing;
}

/**
 * The columns `rows` lacks: every non-baseline WYSIWYG component needs a registered `stock` and a
 * registered `best-effort` variant, each with rows.
 */
export function missingColumns(rows: RoundTripRow[], variants: VariantInfo[]): string[] {
  const measured = new Set(rows.map((row) => row.variant));
  const candidates = variants.filter((variant) => variant.baseline !== true);
  const missing: string[] = [];
  for (const component of new Set(candidates.filter((variant) => variant.wysiwyg).map((variant) => variant.component))) {
    for (const column of ['stock', 'best-effort'] as const) {
      const variant = candidates.find((candidate) => candidate.component === component && candidate.column === column);
      if (variant === undefined) missing.push(`${component}: no ${column} variant is registered`);
      else if (!measured.has(variant.name)) missing.push(`${component}: no rows for ${variant.name}`);
    }
  }
  return missing;
}
