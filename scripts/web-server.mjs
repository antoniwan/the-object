// What gif.mjs, shader-time.mjs and load-check.mjs share: Playwright's Chromium, the flags that
// give it a software GPU, and a small local server for the built page (web/, after ./build.sh).
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { createRequire } from 'node:module';

// Through require, so a global Playwright is found with NODE_PATH=$(npm root -g).
export const { chromium } = createRequire(import.meta.url)('playwright');

/** WebGL2 on the CPU, so it runs headless with no GPU. */
export const SOFTWARE_GPU = ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'];

export const root = new URL('../web/', import.meta.url).pathname;
const types = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.wasm': 'application/wasm',
  '.woff2': 'font/woff2',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
};

/** Serves web/ on a free port; returns the page's address and the server, to close when done. */
export function serve() {
  const server = createServer(async (req, res) => {
    const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^(\.\.[/\\])+/, '');
    try {
      const body = await readFile(join(root, path.endsWith('/') ? path + 'index.html' : path));
      res.writeHead(200, { 'content-type': types[extname(path) || '.html'] || 'application/octet-stream' }).end(body);
    } catch {
      res.writeHead(404).end();
    }
  }).listen(0);
  return { url: `http://localhost:${server.address().port}/`, server };
}
