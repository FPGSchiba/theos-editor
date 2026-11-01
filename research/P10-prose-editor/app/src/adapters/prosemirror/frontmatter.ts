import { splitFrontMatter } from '../../harness/frontmatter.ts';

/**
 * Write mode keeps the front matter outside the ProseMirror document: it is split off when the
 * file is loaded and put back in front of the serialized body on save.
 */
export { splitFrontMatter };

/** The file: the front matter bytes plus the body; a front matter that lacks its final newline gets one. */
export function joinFrontMatter(frontMatter: string, body: string): string {
  if (frontMatter === '' || body === '' || frontMatter.endsWith('\n')) return frontMatter + body;
  return `${frontMatter}\n${body}`;
}
