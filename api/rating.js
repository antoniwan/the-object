// The answers to the object's question. POST {v: 1..5} from the page; GET the aggregate,
// which web/index.html puts into the page's structured data. Stored in Vercel Blob (private):
// ratings/summary.json holds the counts, ratings/visitors/<hash>.json each visitor's last answer.
import { get, put, BlobPreconditionFailedError } from '@vercel/blob';
import { AGAIN_AFTER, aggregate, allowedOrigin, clientIp, parseAnswer, tally, visitor } from './_rating.js';

const SUMMARY = 'ratings/summary.json';
const json = (body, status = 200, headers = {}) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json', ...headers } });

async function read(pathname) {
  const r = await get(pathname, { access: 'private', useCache: false });
  if (!r || r.statusCode !== 200) return null;
  return { data: JSON.parse(await new Response(r.stream).text()), etag: r.blob.etag };
}

function write(pathname, data, etag) {
  return put(pathname, JSON.stringify(data), {
    access: 'private',
    addRandomSuffix: false,
    contentType: 'application/json',
    ...(etag ? { ifMatch: etag } : { allowOverwrite: true }),
  });
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
  try {
    const before = await read(who);
    const prev = before?.data.v ?? null;
    if (before && Date.now() - before.data.at < AGAIN_AFTER) return json({ error: 'again later' }, 429);
    if (prev !== v) {
      // Two answers at once would race on the counts; the loser reads again.
      for (let tries = 0; ; tries++) {
        const s = await read(SUMMARY);
        try {
          await write(SUMMARY, { counts: tally(s?.data.counts, prev, v) }, s?.etag);
          break;
        } catch (e) {
          if (!(e instanceof BlobPreconditionFailedError) || tries >= 4) throw e;
        }
      }
    }
    await write(who, { v, at: Date.now() });
    return new Response(null, { status: 204 });
  } catch (e) {
    console.error(e);
    return json({ error: 'not kept' }, 500);
  }
}
