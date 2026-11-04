import type { ProseAdapter } from '../../harness/types.ts';
import { ProseMirrorAdapter } from '../prosemirror/index.ts';
import { SourceBody } from './sourcebody.ts';

/**
 * Prototype B, best-effort column: the ProseMirror prototype of `../prosemirror/` with the same
 * schema, parser, node views, plugins and history, and the source text as the saved truth
 * (`SourceBody`): untouched blocks keep their bytes, changes are mapped onto the source as span
 * edits, and a block whose change cannot be mapped is written again by the stock serializer.
 */
class BestEffortAdapter extends ProseMirrorAdapter {
  constructor(private readonly sourceBody: SourceBody = new SourceBody()) {
    super(sourceBody);
  }

  /** `span` when every change in the saved text was mapped as span edits, `block` when a block was written again. */
  lastSaveLevel(): 'span' | 'block' {
    // The level of what `text()` saves now: the pending changes are mapped first.
    this.text();
    return this.sourceBody.level();
  }
}

export function createProseMirrorBestEffortAdapter(): ProseAdapter {
  return new BestEffortAdapter();
}
