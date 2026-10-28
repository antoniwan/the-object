// Rewrites web/index.html for itch.io (see itch.sh): itch serves the game from a CDN
// subfolder in an iframe, so root-absolute links and the Vercel-only parts go.
import { readFileSync, writeFileSync } from 'node:fs';

const [src, dest] = process.argv.slice(2);
let html = readFileSync(src, 'utf8');
html = html.split('\n').filter((l) => !/_vercel\/insights|window\.va = |rel="manifest"|rel="apple-touch-icon"/.test(l)).join('\n');
html = html.replaceAll('href="/favicon', 'href="favicon');
// The ratings script posts to the site's own /api/rating; itch has its own ratings.
html = html.replace(/<script type="module">\n\/\/ After contact it asks one question[\s\S]*?<\/script>\n/, '');

if (html.includes('api/rating')) throw new Error('itch-page: rating script left in index.html');
const abs = html.match(/(src|href)="\/[^/][^"]*"/);
if (abs) throw new Error(`itch-page: root-absolute path left in index.html: ${abs[0]}`);
writeFileSync(dest, html);
