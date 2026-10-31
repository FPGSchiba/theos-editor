import { createBaselineAdapter } from '../adapters/baseline/index.ts';
import { createCodeMirrorAdapter } from '../adapters/codemirror/index.ts';
import type { ProseAdapter } from './types.ts';
import { variants } from './variants.ts';

export { variants };

/** The factory of every variant, by the `name` of its entry in `variants.ts`. */
const factories: Record<string, () => ProseAdapter> = {
  baseline: createBaselineAdapter,
  codemirror: createCodeMirrorAdapter,
};

/** A new, unmounted adapter of the named variant. */
export function createAdapter(name: string): ProseAdapter {
  const create = factories[name];
  if (create === undefined) {
    throw new Error(`registry: unknown variant ${JSON.stringify(name)} (known: ${variants.map((v) => v.name).join(', ')})`);
  }
  return create();
}
