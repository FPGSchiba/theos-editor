// What the app says about itself (`app_info`): its version and the OS, which picks Ctrl or Cmd.

import { commands } from '../ipc/bindings';
import { failureText } from '../ipc/client';
import { asOs, type Os } from '../keymap';

/** The app's version and OS; `version` is empty until `app_info` answered. */
export const app = $state<{ version: string; os: Os }>({ version: '', os: 'windows' });

/**
 * Asks the app for its version and OS. On a failure the defaults stay, and the result is the
 * failure as text; else `null`.
 */
export async function loadApp(): Promise<string | null> {
  try {
    const info = await commands.appInfo();
    app.version = info.version;
    app.os = asOs(info.os);
    return null;
  } catch (failure) {
    return `The app did not say its version and OS: ${failureText(failure)}`;
  }
}
