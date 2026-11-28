import { expect, test } from 'vitest';

import { count, totals, validator } from './counts';

test('the validator pip names errors first, then warnings, else clean', () => {
  expect(validator({ errors: 5, warnings: 2 })).toEqual({
    text: 'validator · 5 errors',
    signal: 'reject',
  });
  expect(validator({ errors: 1, warnings: 0 })).toEqual({
    text: 'validator · 1 error',
    signal: 'reject',
  });
  expect(validator({ errors: 0, warnings: 3 })).toEqual({
    text: 'validator · 3 warn',
    signal: 'warn',
  });
  expect(validator({ errors: 0, warnings: 0 })).toEqual({
    text: 'validator · clean',
    signal: 'approve',
  });
});

test('the totals count records, errors and warnings', () => {
  expect(totals({ records: 21, errors: 5, warnings: 2 })).toBe('21 records · 5 errors · 2 warnings');
  expect(totals({ records: 1, errors: 0, warnings: 1 })).toBe('1 record · 0 errors · 1 warning');
  expect(count(0, 'record')).toBe('0 records');
});

test('a count takes an irregular plural', () => {
  expect(count(1, 'child', 'children')).toBe('1 child');
  expect(count(2, 'child', 'children')).toBe('2 children');
  expect(count(1, 'link in', 'links in')).toBe('1 link in');
  expect(count(8, 'link in', 'links in')).toBe('8 links in');
});
