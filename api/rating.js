// The answers to the object's question. POST {v: 1..5} from the page; GET the aggregate,
// which web/index.html puts into the page's structured data. Stored in Vercel Blob (private):
// ratings/summary.json holds the counts, ratings/visitors/<hash>.json each visitor's last answer
// and which answer of theirs the counts hold.
import { randomUUID } from 'node:crypto';
import { get, put, BlobError } from '@vercel/blob';
import { AGAIN_AFTER, aggregate, allowedOrigin, clientIp, parseAnswer, tally, visitor } from './_rating.js';

const SUMMARY = 'ratings/summary.json';
const json = (body, status = 200, headers = {}) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json', ...headers } });

async function read(pathname) {
  const r = await get(pathname, { access: 'private', useCache: false });
  if (!r || r.statusCode !== 200) return null;
  return { data: JSON.parse(await new Response(r.stream).text()), etag: r.blob.etag };
}

// Writes over `etag` only if the blob is still that one; with no etag, only if there is none yet.
function write(pathname, data, etag) {
  return put(pathname, JSON.stringify(data), {
    access: 'private',
    addRandomSuffix: false,
    contentType: 'application/json',
    ...(etag ? { ifMatch: etag, allowOverwrite: true } : { allowOverwrite: false }),
  });
}

// Writes `data` (which carries this request's own `id`) over what was read. Returns the new
// etag, or null when another write got there first. The store can take a write and lose its
// answer, and the library's retry is then refused: finding its own id there, it was kept.
async function attempt(pathname, data, etag) {
  try {
    return (await write(pathname, data, etag)).etag;
  } catch (e) {
    if (!(e instanceof BlobError)) throw e;
    const now = await read(pathname);
    if (now && now.data.id === data.id) return now.etag;
    if ((now?.etag ?? null) !== (etag ?? null)) return null;
    throw e;
  }
}

export async function GET() {
  if (!process.env.BLOB_READ_WRITE_TOKEN) return json(aggregate([]));
  try {
    const s = await read(SUMMARY);
    return json(aggregate(s?.data.counts), 200, {
      'cache-control': 'public, max-age=0, must-revalidate',
      'cdn-cache-control': 'public, s-maxage=900, stale-while-revalidate=86400',
    });
  } catch (e) {
    console.error(e);
    return json(aggregate([]), 200, { 'cache-control': 'no-store' });
  }
}

export async function POST(request) {
  if (!allowedOrigin(request.headers.get('origin'))) return json({ error: 'origin' }, 403);
  if (!process.env.BLOB_READ_WRITE_TOKEN) return json({ error: 'no store' }, 503);
  let v;
  try {
    v = parseAnswer(await request.json());
  } catch {
    v = null;
  }
  if (!v) return json({ error: 'v must be 1..5' }, 400);

  // The store's own secret doubles as the salt unless one is set; it never leaves the function.
  const salt = process.env.RATING_SALT || process.env.BLOB_READ_WRITE_TOKEN;
  const who = `ratings/visitors/${visitor(clientIp(request.headers), salt)}.json`;
  const id = randomUUID();
  try {
    const before = await read(who);
    const was = before?.data;
    if (was && Date.now() - was.at < AGAIN_AFTER) return json({ error: 'again later' }, 429);
    // Which answer of theirs the counts hold (records from before `counted` were written only
    // once counted).
    const prev = (was && ('counted' in was ? was.counted : was.v)) ?? null;
    const at = Date.now();
    // The visitor's answer is claimed first, against what was read: of several answers sent at
    // once, only one gets past here, so one visitor is never counted twice.
    const claimed = await attempt(who, { v, at, counted: prev, id }, before?.etag);
    if (!claimed) return json({ error: 'again later' }, 429);
    if (prev !== v) {
      // Two visitors at once would race on the counts; the loser reads again.
      for (let tries = 0; ; tries++) {
        const s = await read(SUMMARY);
        if (await attempt(SUMMARY, { counts: tally(s?.data.counts, prev, v), id }, s?.etag)) break;
        if (tries >= 4) throw new Error('the counts kept changing');
      }
      // Only now does the record say it is counted. Should this write fail, the record still
      // says what the counts hold, and the next answer puts it right.
      try {
        await write(who, { v, at, counted: v, id }, claimed);
      } catch (e) {
        console.error(e);
      }
    }
    return new Response(null, { status: 204 });
  } catch (e) {
    console.error(e);
    return json({ error: 'not kept' }, 500);
  }
}
