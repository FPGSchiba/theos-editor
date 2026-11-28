// The OS folder dialog (P11 §5.9), through Tauri's dialog plugin. The capability allows only
// `dialog:allow-open`.

import { open } from '@tauri-apps/plugin-dialog';

/** Asks for a folder with the OS dialog: its path, or `null` when the user cancelled. */
export function pickFolder(): Promise<string | null> {
  return open({ directory: true, multiple: false, title: 'Open project' });
}
