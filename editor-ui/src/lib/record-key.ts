// The key of a record in a list: its file's path, else its id. Two files may carry one id (a
// duplicate the index reports); each is still its own row, so lists key rows by the file.

import type { RecordRef } from './ipc/bindings';

/** What opens a record: its id, and the path of its file when known (it picks the file of a
 * duplicated id). */
export type RecordTarget = Pick<RecordRef, 'id' | 'path'>;

/** The key of `record` in a list: its project-relative path, or its id when it has none. */
export function recordKey(record: RecordTarget): string {
  return record.path ?? record.id;
}
