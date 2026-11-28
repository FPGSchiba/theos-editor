// The window getting the focus back (`tauri://focus`, P11 §5.9), which syncs the open project.
// The capability allows listening (and unlistening) to events, nothing else of the event plugin.

import { TauriEvent } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Calls `handler` each time the app's window gets the focus; resolves to the unlisten function,
 * and rejects when the window cannot be listened to.
 */
export async function onWindowFocus(handler: () => void): Promise<() => void> {
  return await getCurrentWindow().listen(TauriEvent.WINDOW_FOCUS, handler);
}
