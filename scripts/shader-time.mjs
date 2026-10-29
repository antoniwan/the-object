// Times how long the built page (web/, after ./build.sh) takes to be ready, in headless Chromium
// at 48x32 with the software GPU, where compiling the shader is most of the wait. Each run starts
// a fresh browser so nothing compiled before is reused. Run it before and after a shader change
// and put both numbers in the pull request (CONTRIBUTING.md, The shader compile budget).
//
//     node scripts/shader-time.mjs [runs]     # needs Playwright and its Chromium (npm i -g playwright)
//
// A few runs come back far quicker than the rest when the browser still finds an earlier
// compile; the median of the slow group is the number to compare. It launches the full Chromium
// (channel 'chromium'): the headless shell is ready far sooner and measures nothing.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { createRequire } from 'node:module';

// Through require, so a global Playwright is found with NODE_PATH=$(npm root -g).
const { chromium } = createRequire(import.meta.url)('playwright');

const runs = Number(process.argv[2] || 9);
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

const times = [];
for (let i = 0; i < runs; i++) {
  const browser = await chromium.launch({ channel: 'chromium', args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--disable-gpu-shader-disk-cache'] });
  const page = await browser.newPage({ viewport: { width: 48, height: 32 } });
  const t0 = Date.now();
  await page.goto(url);
  await page.waitForFunction(() => document.body.classList.contains('awake') || document.body.classList.contains('failed'), null, { timeout: 60000, polling: 10 });
  if (await page.evaluate(() => document.body.classList.contains('failed'))) throw new Error('the page gave up (body.failed)');
  times.push(Date.now() - t0);
  await browser.close();
}
server.close();
times.sort((a, b) => a - b);
const slow = times.filter((t) => t > times[times.length - 1] / 2);
console.log(`ms ${times.join(' ')}  median ${slow[slow.length >> 1]} (of the ${slow.length} uncached)`);
