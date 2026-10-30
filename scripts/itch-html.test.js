// The pages pasted into itch.io: each one's HTML is what its Markdown makes, and every picture
// they show from this repository is in it.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { pages, toHtml } from './itch-html.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
const RAW = 'https://raw.githubusercontent.com/antoniwan/the-object/main/';

for (const page of pages()) {
  test(`${page} has its HTML, up to date`, () => {
    const html = page.replace(/\.md$/, '.html');
    assert.ok(existsSync(root + html), `${html} is missing: node scripts/itch-html.mjs ${page}`);
    assert.equal(
      readFileSync(root + html, 'utf8'),
      toHtml(readFileSync(root + page, 'utf8')),
      `${html} is out of date: node scripts/itch-html.mjs ${page}`,
    );
  });
  test(`${page} shows only pictures that are here`, () => {
    for (const [, url] of readFileSync(root + page, 'utf8').matchAll(/!\[[^\]]*\]\(([^)\s]+)\)/g)) {
      if (!url.startsWith('https://raw.githubusercontent.com/')) continue;
      assert.ok(url.startsWith(RAW), `${page}: a picture not served from main: ${url}`);
      assert.ok(existsSync(root + url.slice(RAW.length)), `${page}: ${url.slice(RAW.length)} is not in the repository`);
    }
  });
}
