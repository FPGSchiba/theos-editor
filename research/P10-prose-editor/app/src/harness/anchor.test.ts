import { describe, expect, it } from 'vitest';
import { resolveAnchor } from './anchor.ts';

describe('resolveAnchor', () => {
  it('finds the nth occurrence plus offset', () => {
    expect(resolveAnchor('a b a', { find: 'a', occurrence: 2, offset: 1 })).toBe(5);
  });

  it('defaults to the first occurrence and offset 0', () => {
    expect(resolveAnchor('a b a', { find: 'a' })).toBe(0);
    expect(resolveAnchor('a b a', { find: 'b' })).toBe(2);
  });

  it('allows a negative offset that stays inside the text', () => {
    expect(resolveAnchor('one two', { find: 'two', offset: -1 })).toBe(3);
  });

  it('counts occurrences without overlap', () => {
    expect(resolveAnchor('aaaa', { find: 'aa', occurrence: 2 })).toBe(2);
    expect(() => resolveAnchor('aaa', { find: 'aa', occurrence: 2 })).toThrow(/occurrence 2/);
  });

  it('throws when the text is missing', () => {
    expect(() => resolveAnchor('a b a', { find: 'zzz' })).toThrow(/zzz/);
  });

  it('throws when there are fewer occurrences than asked for', () => {
    expect(() => resolveAnchor('a b a', { find: 'a', occurrence: 3 })).toThrow(/occurrence 3/);
  });

  it('throws on an empty find or an occurrence below 1', () => {
    expect(() => resolveAnchor('abc', { find: '' })).toThrow();
    expect(() => resolveAnchor('abc', { find: 'a', occurrence: 0 })).toThrow();
  });

  it('throws when the offset leaves the text', () => {
    expect(() => resolveAnchor('abc', { find: 'a', offset: -1 })).toThrow();
    expect(() => resolveAnchor('abc', { find: 'c', offset: 2 })).toThrow();
  });

  it('accepts the end of the text as a position', () => {
    expect(resolveAnchor('abc', { find: 'c', offset: 1 })).toBe(3);
  });
});
