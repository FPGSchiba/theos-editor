import { describe, expect, it } from 'vitest';
import { splitFrontMatter } from './frontmatter.ts';

describe('splitFrontMatter', () => {
  it('keeps both fence lines and the newline', () => {
    expect(splitFrontMatter('---\nid: x\n---\nBody\n')).toEqual({
      frontMatter: '---\nid: x\n---\n',
      body: 'Body\n',
    });
  });

  it('treats a thematic break in the body as body', () => {
    const { frontMatter, body } = splitFrontMatter('---\nid: x\n---\nA\n\n---\n\nB\n');
    expect(frontMatter).toBe('---\nid: x\n---\n');
    expect(body).toBe('A\n\n---\n\nB\n');
  });

  it('without front matter returns the whole text as body', () => {
    expect(splitFrontMatter('Text')).toEqual({ frontMatter: '', body: 'Text' });
  });

  it('on CRLF keeps the \r\n bytes in both parts', () => {
    expect(splitFrontMatter('---\r\nid: x\r\n---\r\nBody\r\n')).toEqual({
      frontMatter: '---\r\nid: x\r\n---\r\n',
      body: 'Body\r\n',
    });
  });

  it('is not front matter when the closing fence is missing', () => {
    expect(splitFrontMatter('---\nid: x\nBody\n')).toEqual({
      frontMatter: '',
      body: '---\nid: x\nBody\n',
    });
  });

  it('does not take a longer dash run for a fence', () => {
    expect(splitFrontMatter('---\nid: x\n----\nBody\n')).toEqual({
      frontMatter: '',
      body: '---\nid: x\n----\nBody\n',
    });
    expect(splitFrontMatter('----\nid: x\n---\nBody\n').frontMatter).toBe('');
  });

  it('accepts an empty front matter block', () => {
    expect(splitFrontMatter('---\n---\nBody')).toEqual({ frontMatter: '---\n---\n', body: 'Body' });
  });

  it('accepts a closing fence at the very end of the text', () => {
    expect(splitFrontMatter('---\nid: x\n---')).toEqual({ frontMatter: '---\nid: x\n---', body: '' });
  });

  it('keeps front matter and body as exact halves of the input', () => {
    const text = '---\nid: x\n---\nA\n---\nB';
    const { frontMatter, body } = splitFrontMatter(text);
    expect(frontMatter + body).toBe(text);
  });
});
