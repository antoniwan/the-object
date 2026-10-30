// api/rating.js against a store kept in memory, which can fail the way the real one does.
import { test, mock, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

class BlobError extends Error {}
class BlobPreconditionFailedError extends BlobError {}
const store = new Map();
const faults = { countsFail: 0, countsKeptUnanswered: 0, markFail: 0, countsTakenThenOverwritten: 0 };
let n = 0;

function take(p, body, o) {
  const cur = store.get(p);
  if (o.ifMatch && (!cur || cur.etag !== o.ifMatch)) throw new BlobPreconditionFailedError();
  if (o.allowOverwrite === false && cur) throw new BlobError('exists');
  const etag = `e${++n}`;
  store.set(p, { body, etag });
  return { etag };
}

mock.module('@vercel/blob', {
  namedExports: {
    BlobError,
    BlobPreconditionFailedError,
    async get(p) {
      const b = store.get(p);
      return b ? { statusCode: 200, stream: new Blob([b.body]).stream(), blob: { etag: b.etag } } : null;
    },
    async put(p, body, o) {
      const counts = p.endsWith('summary.json');
      const data = JSON.parse(body);
      // Only the write that marks an answer counted fails.
      if (!counts && faults.markFail > 0 && data.counted === data.v) {
        faults.markFail--;
        throw new BlobError('service unavailable');
      }
      if (counts && faults.countsTakenThenOverwritten > 0) {
        faults.countsTakenThenOverwritten--;
        take(p, body, o);
        // Another visitor's answer of 1 lands on top before the answer comes back.
        const cur = JSON.parse(store.get(p).body);
        cur.counts[0] += 1;
        cur.recent = [...(cur.recent ?? []), 'other'];
        cur.id = 'other';
        take(p, JSON.stringify(cur), { ifMatch: store.get(p).etag, allowOverwrite: true });
        throw new BlobError('lost answer');
      }
      if (counts && faults.countsFail > 0) {
        faults.countsFail--;
        throw new BlobError('service unavailable');
      }
      const r = take(p, body, o);
      // The store took it but the answer was lost: the library sends the same request again.
      if (counts && faults.countsKeptUnanswered > 0) {
        faults.countsKeptUnanswered--;
        return take(p, body, o);
      }
      return r;
    },
  },
});

const { POST } = await import('./rating.js');
process.env.BLOB_READ_WRITE_TOKEN = 'test';
console.error = () => {};
let now = 1_000_000;
Date.now = () => now;

const answer = (v, ip) =>
  POST(
    new Request('http://localhost/api/rating', {
      method: 'POST',
      headers: { origin: 'https://the-object.antoniwan.online', 'x-forwarded-for': ip },
      body: JSON.stringify({ v }),
    }),
  ).then((r) => r.status);
const counts = () => JSON.parse(store.get('ratings/summary.json')?.body ?? '{"counts":[0,0,0,0,0]}').counts;
const later = () => (now += 61_000);

beforeEach(() => {
  store.clear();
  faults.countsFail = 0;
  faults.countsKeptUnanswered = 0;
  faults.markFail = 0;
  faults.countsTakenThenOverwritten = 0;
});

test('one answer is counted once', async () => {
  assert.equal(await answer(4, '1.1.1.1'), 204);
  assert.deepEqual(counts(), [0, 0, 0, 1, 0]);
});

test('answers sent at once by one visitor are counted once', async () => {
  const s = await Promise.all([answer(5, '1.1.1.1'), answer(5, '1.1.1.1'), answer(5, '1.1.1.1')]);
  assert.equal(s.filter((x) => x === 204).length, 1);
  assert.deepEqual(counts(), [0, 0, 0, 0, 1]);
});

test('a changed answer replaces the earlier one', async () => {
  await answer(5, '1.1.1.1');
  later();
  await answer(2, '1.1.1.1');
  assert.deepEqual(counts(), [0, 1, 0, 0, 0]);
});

test('an answer the counts could not take is counted when given again, and takes nothing from another', async () => {
  await answer(5, '1.1.1.1');
  faults.countsFail = 1;
  assert.equal(await answer(5, '2.2.2.2'), 500);
  assert.deepEqual(counts(), [0, 0, 0, 0, 1]);
  later();
  assert.equal(await answer(5, '2.2.2.2'), 204);
  assert.deepEqual(counts(), [0, 0, 0, 0, 2]);
  later();
  faults.countsFail = 1;
  await answer(1, '3.3.3.3');
  later();
  await answer(2, '3.3.3.3');
  assert.deepEqual(counts(), [0, 1, 0, 0, 2]);
});

test('counts the store took without answering are not counted again', async () => {
  faults.countsKeptUnanswered = 1;
  assert.equal(await answer(4, '1.1.1.1'), 204);
  assert.deepEqual(counts(), [0, 0, 0, 1, 0]);
});

test('records from before are read as counted', async () => {
  await answer(3, '1.1.1.1');
  for (const [p, b] of store) {
    if (p.includes('visitors')) store.set(p, { ...b, body: JSON.stringify({ v: 3, at: 0 }) });
  }
  later();
  await answer(1, '1.1.1.1');
  assert.deepEqual(counts(), [1, 0, 0, 0, 0]);
});

test('a changed answer whose record could not be marked counted is not counted twice', async () => {
  await answer(5, '1.1.1.1');
  await answer(5, '9.9.9.9');
  later();
  faults.markFail = 1;
  assert.equal(await answer(2, '1.1.1.1'), 204);
  assert.deepEqual(counts(), [0, 1, 0, 0, 1]);
  later();
  assert.equal(await answer(3, '1.1.1.1'), 204);
  assert.deepEqual(counts(), [0, 0, 1, 0, 1]);
});

test('counts taken without an answer, then written over by another visitor, are not counted again', async () => {
  faults.countsTakenThenOverwritten = 1;
  assert.equal(await answer(4, '1.1.1.1'), 204);
  // one from the other visitor (index 0), one from this one (index 3)
  assert.deepEqual(counts(), [1, 0, 0, 1, 0]);
});
