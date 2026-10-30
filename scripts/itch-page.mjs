// Rewrites web/index.html for itch.io (see itch.sh): itch serves the game from a CDN
// subfolder in an iframe, so root-absolute links and the Vercel-only parts go.
import { readFileSync, writeFileSync } from 'node:fs';

const [src, dest] = process.argv.slice(2);
let html = readFileSync(src, 'utf8');
// The ratings script (rating.js) posts to the site's own /api/rating; itch has its own ratings.
html = html
  .split('\n')
  .filter(
    (l) =>
      !/_vercel\/insights|window\.va = |antoniwan-stats|window\.umami|<!-- Stats:|rel="manifest"|rel="apple-touch-icon"|src="rating\.js"/.test(
        l,
      ),
  )
  .join('\n');
html = html.replaceAll('href="/favicon', 'href="favicon');

if (html.includes('umami')) throw new Error('itch-page: Stats left in index.html');
if (html.includes('rating.js') || html.includes('api/rating'))
  throw new Error('itch-page: rating script left in index.html');
const abs = html.match(/(src|href)="\/[^/][^"]*"/);
if (abs) throw new Error(`itch-page: root-absolute path left in index.html: ${abs[0]}`);
writeFileSync(dest, html);
