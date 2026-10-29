// Loads the built page (web/, after ./build.sh) in headless Chromium as a few browsers would
// look to it, and fails if any of them throws or gives up before it wakes. Each one is loaded a
// second time with what the first visit saved, as a returning player would.
//
//     node scripts/load-check.mjs     # needs Playwright and its Chromium (npm i -g playwright)
//
// "safari" has a speech recognizer but no navigator.userAgentData, which once took the page down
// on load (1.16.7). Add a browser here when one of them breaks the page in a new way.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { createRequire } from 'node:module';

// Through require, so a global Playwright is found with NODE_PATH=$(npm root -g).
const { chromium } = createRequire(import.meta.url)('playwright');

const root = new URL('../web/', import.meta.url).pathname;
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.woff2': 'font/woff2', '.png': 'image/png', '.ico': 'image/x-icon' };

const server = createServer(async (req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^(\.\.[/\\])+/, '');
  try {
    const body = await readFile(join(root, path.endsWith('/') ? path + 'index.html' : path));
    res.writeHead(200, { 'content-type': types[extname(path) || '.html'] || 'application/octet-stream' }).end(body);
  } catch {
    res.writeHead(404).end();
  }
}).listen(0);
const url = `http://localhost:${server.address().port}/`;

const hide = (name) => `Object.defineProperty(Navigator.prototype, ${JSON.stringify(name)}, { get: () => undefined, configurable: true });`;
const browsers = {
  chromium: '',
  safari: hide('userAgentData') + 'window.webkitSpeechRecognition = window.webkitSpeechRecognition || function () {}; delete window.SpeechRecognition;',
  firefox: hide('userAgentData') + 'delete window.SpeechRecognition; delete window.webkitSpeechRecognition;',
};

const browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
let bad = 0;
for (const [name, script] of Object.entries(browsers)) {
  const context = await browser.newContext({ viewport: { width: 96, height: 64 } });
  if (script) await context.addInitScript(script);
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  for (const visit of ['first', 'return']) {
    await (visit === 'first' ? page.goto(url) : page.reload());
    const woke = await page
      .waitForFunction(() => document.body.classList.contains('awake') || document.body.classList.contains('failed'), null, { timeout: 60000 })
      .then(() => page.evaluate(() => document.body.classList.contains('awake')), () => false);
    await page.waitForTimeout(1500);
    const ok = woke && errors.length === 0;
    console.log(`${ok ? 'ok  ' : 'FAIL'} ${name} ${visit}${errors.length ? ': ' + errors.join(' | ') : woke ? '' : ': never woke'}`);
    if (!ok) bad++;
    errors.length = 0;
  }
  await context.close();
}
await browser.close();
server.close();
if (bad) process.exit(1);
