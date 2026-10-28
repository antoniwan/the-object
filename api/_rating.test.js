import { test } from 'node:test';
import assert from 'node:assert/strict';
import { aggregate, allowedOrigin, clientIp, parseAnswer, tally, visitor } from './_rating.js';

test('only whole answers from one to five count', () => {
  assert.equal(parseAnswer({ v: 3 }), 3);
  for (const v of [0, 6, 2.5, '4', null]) assert.equal(parseAnswer({ v }), null);
  assert.equal(parseAnswer(null), null);
});

test('a visitor who answers again replaces their answer', () => {
  let c = tally([], null, 5);
  c = tally(c, null, 3);
  assert.deepEqual(c, [0, 0, 1, 0, 1]);
  c = tally(c, 5, 1);
  assert.deepEqual(c, [1, 0, 1, 0, 0]);
  assert.deepEqual(tally([0, 0, 0, 0, 0], 2, 4), [0, 0, 0, 1, 0], 'never below zero');
});

test('the aggregate is the mean and the count', () => {
  assert.deepEqual(aggregate([]), { count: 0, value: null, best: 5, worst: 1 });
  assert.deepEqual(aggregate([1, 0, 0, 0, 2]), { count: 3, value: 3.67, best: 5, worst: 1 });
  assert.equal(aggregate(['x', -2, 1]).count, 1);
});

test('answers come only from the page, and nobody is kept by address', () => {
  assert.ok(allowedOrigin('https://the-object.antoniwan.online'));
  assert.ok(!allowedOrigin('https://example.com'));
  assert.ok(!allowedOrigin(null));
  const a = visitor('1.2.3.4', 's');
  assert.equal(a, visitor('1.2.3.4', 's'));
  assert.notEqual(a, visitor('1.2.3.5', 's'));
  assert.ok(!a.includes('1.2.3.4'));
  assert.equal(clientIp(new Headers({ 'x-forwarded-for': '9.9.9.9, 10.0.0.1' })), '9.9.9.9');
});
