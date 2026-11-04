import { createMockContext, createMockIndex, type MockIndex } from './mock-index.ts';
import { createAdapter, variants } from './registry.ts';
import type {
  EditorContext, LinkInfo, Mode, ProseAdapter, ScriptedEdit, TextChange, VariantInfo,
} from './types.ts';

/** What the Playwright suites call: `window.__harness`. Offsets are into the LF form of the file. */
export interface Harness {
  variants(): VariantInfo[];
  /** Destroy the previous adapter and mount a fresh one of `variant` on `text`; `fileId` is the manifest path. */
  load(variant: string, text: string, fileId: string): Promise<void>;
  text(): string;
  apply(edit: ScriptedEdit): void;
  setMode(mode: Mode): void;
  focus(): void;
  caretTo(offset: number): void;
  caret(): number;
  applyExternal(changes: TextChange[]): void;
  undo(): void;
  canUndo(): boolean;
  markSaved(): void;
  /** How often the editor asked the harness to save (`ctx.save()`) since the last load. */
  saveCount(): number;
  /** Add or replace records in the mock index and tell the adapter (`updateContext`, no re-mount). */
  updateIndex(patch: LinkInfo[]): void;
  lastSaveLevel(): string | undefined;
}

declare global {
  interface Window {
    __harness?: Harness;
  }
}

/** What the page shows about the open file; the toolbar mirrors it. */
export interface HarnessState {
  variant: string | undefined;
  fileId: string | undefined;
  mode: Mode;
}

/** The element of index.html the adapters mount into, one fresh child per load. */
function hostElement(): HTMLElement {
  const host = document.getElementById('editor-host');
  if (host === null) throw new Error('bridge: index.html has no #editor-host');
  return host;
}

class Bridge implements Harness {
  private adapter: ProseAdapter | undefined;
  private target: HTMLElement | undefined;
  private index: MockIndex = createMockIndex();
  private fileId: string | undefined;
  private variant: string | undefined;
  private mode: Mode = 'write';
  private saves = 0;
  /** Loads run one after another, so a toolbar click and a test never mount side by side. */
  private queue: Promise<void> = Promise.resolve();
  private readonly listeners = new Set<() => void>();

  variants(): VariantInfo[] {
    return variants.map((variant) => ({ ...variant, packages: [...variant.packages] }));
  }

  load(variant: string, text: string, fileId: string): Promise<void> {
    const run = this.queue.then(() => this.mount(variant, text, fileId));
    this.queue = run.catch(() => undefined);
    return run;
  }

  text(): string {
    return this.current().text();
  }

  apply(edit: ScriptedEdit): void {
    this.current().apply(edit);
  }

  setMode(mode: Mode): void {
    this.current().setMode(mode);
    this.mode = mode;
    this.notify();
  }

  focus(): void {
    this.current().focus();
  }

  caretTo(offset: number): void {
    this.current().caretTo(offset);
  }

  caret(): number {
    return this.current().caret();
  }

  applyExternal(changes: TextChange[]): void {
    this.current().applyExternal(changes);
  }

  undo(): void {
    this.current().undo();
  }

  canUndo(): boolean {
    return this.current().canUndo();
  }

  markSaved(): void {
    this.current().markSaved();
  }

  saveCount(): number {
    return this.saves;
  }

  updateIndex(patch: LinkInfo[]): void {
    const adapter = this.current();
    this.index.patch(patch);
    adapter.updateContext(this.context());
  }

  lastSaveLevel(): string | undefined {
    return this.current().lastSaveLevel?.();
  }

  state(): HarnessState {
    return { variant: this.variant, fileId: this.fileId, mode: this.mode };
  }

  /** Call `listener` after every load and mode change; returns the way to stop. */
  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private async mount(variant: string, text: string, fileId: string): Promise<void> {
    this.unmount();
    const adapter = createAdapter(variant);
    const target = document.createElement('div');
    target.className = 'hx-adapter';
    hostElement().append(target);
    this.target = target;
    this.index = createMockIndex();
    this.saves = 0;
    this.fileId = fileId;
    this.variant = variant;
    this.mode = 'write';
    this.adapter = adapter;
    try {
      adapter.mount(target, text, this.context());
    } catch (error) {
      this.unmount();
      throw error;
    }
    this.notify();
  }

  private unmount(): void {
    const adapter = this.adapter;
    this.adapter = undefined;
    try {
      adapter?.destroy();
    } finally {
      this.target?.remove();
      this.target = undefined;
    }
  }

  private context(): EditorContext {
    if (this.fileId === undefined) throw new Error('bridge: no file is loaded');
    return createMockContext({ fileId: this.fileId, index: this.index, save: () => { this.saves += 1; } });
  }

  private current(): ProseAdapter {
    if (this.adapter === undefined) throw new Error('bridge: nothing is loaded; call load() first');
    return this.adapter;
  }

  private notify(): void {
    for (const listener of this.listeners) listener();
  }
}

export const harness = new Bridge();
