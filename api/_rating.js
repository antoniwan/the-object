// What the object's question comes to (see src/contact.rs, `Ask`): pure helpers for
// api/rating.js, kept apart so they can be tested without a store.
import { createHash } from 'node:crypto';

export const STEPS = 5;
/** How soon the same visitor may answer again, in milliseconds. */
export const AGAIN_AFTER = 60_000;
const ORIGINS = ['https://the-object.antoniwan.online', 'http://localhost:8000', 'http://127.0.0.1:8000'];

/** A whole number of steps, 1..=5, or null. */
export function parseAnswer(body) {
  const v = body && body.v;
  return Number.isInteger(v) && v >= 1 && v <= STEPS ? v : null;
}

/** Answers are only taken from the page itself. */
export function allowedOrigin(origin) {
  return ORIGINS.includes(origin);
}

/** Who answered, without keeping who they are: a salted hash of the address. */
export function visitor(ip, salt) {
  return createHash('sha256').update(`${salt}|${ip}`).digest('hex').slice(0, 32);
}

/** The first address in x-forwarded-for, which Vercel sets to the client's. */
export function clientIp(headers) {
  const fwd = headers.get('x-forwarded-for') || '';
  return fwd.split(',')[0].trim() || headers.get('x-real-ip') || 'unknown';
}

/** Stored counts, made whole: five counts, none negative. */
function clean(counts) {
  return Array.from({ length: STEPS }, (_, i) => Math.max(0, Math.floor(Number(counts?.[i]) || 0)));
}

/** Counts of each answer (index 0 is 1 step); a visitor's earlier answer is replaced. */
export function tally(counts, prev, v) {
  const c = clean(counts);
  if (prev) c[prev - 1] = Math.max(0, c[prev - 1] - 1);
  c[v - 1] += 1;
  return c;
}

/** The mean answer and how many there are, as schema.org AggregateRating wants them. */
export function aggregate(counts) {
  const c = clean(counts);
  const count = c.reduce((a, b) => a + b, 0);
  const sum = c.reduce((a, n, i) => a + n * (i + 1), 0);
  return { count, value: count ? Math.round((sum / count) * 100) / 100 : null, best: STEPS, worst: 1 };
}
