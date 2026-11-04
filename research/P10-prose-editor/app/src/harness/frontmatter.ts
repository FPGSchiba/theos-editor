/**
 * Front matter is the block that opens the file with a `---` line and ends at the next `---`
 * line. Only the first closing fence counts, so a `---` thematic break in the body is body.
 * Both fence lines and the newline after the closing fence stay in the front matter, and every
 * byte is kept as it is (`\r\n` included), so `frontMatter + body` is always the input.
 */
const FRONT_MATTER = /^---\r?\n(?:[\s\S]*?\r?\n)?---(?:\r?\n|$)/;

export function splitFrontMatter(text: string): { frontMatter: string; body: string } {
  const frontMatter = FRONT_MATTER.exec(text)?.[0] ?? '';
  return { frontMatter, body: text.slice(frontMatter.length) };
}
