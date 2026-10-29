// Records short looping GIFs of the built page (web/, after ./build.sh) for the daily devlog.
// Each scene in scripts/gif-scenes.mjs is played in headless Chromium on a clock that only moves
// when this script moves it, so a slow software GPU still gives smooth, repeatable frames, and
// every pointer, key and wait in a scene lands on the same frame each run.
//
//     node scripts/gif.mjs --list                       # the scenes there are
//     node scripts/gif.mjs cells tree [--out dir]       # dir/cells.gif, dir/tree.gif (dist/gifs by default)
//     node scripts/gif.mjs all --width 400 --fps 12     # every scene, smaller
//
// Needs Playwright and its Chromium (npm i -g playwright; NODE_PATH=$(npm root -g)) and ffmpeg.
// devlog/README.md says how a post uses them.
import { createServer } from 'node:http';
import { mkdtemp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { extname, join, normalize, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import scenes from './gif-scenes.mjs';

// Through require, so a global Playwright is found with NODE_PATH=$(npm root -g).
const { chromium } = createRequire(import.meta.url)('playwright');

const args = process.argv.slice(2);
const flag = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  return i < 0 ? fallback : args.splice(i, 2)[1];
};
const out = resolve(flag('out', 'dist/gifs'));
const width = Number(flag('width', 400));
const fps = Number(flag('fps', 12));
const colors = Number(flag('colors', 112));

if (args.includes('--list') || !args.length) {
  for (const [name, s] of Object.entries(scenes)) console.log(`${name.padEnd(12)} ${s.about}`);
  process.exit(0);
}
const wanted = args.includes('all') ? Object.keys(scenes) : args;
for (const name of wanted) if (!scenes[name]) throw new Error(`no scene "${name}" (--list)`);

const root = new URL('../web/', import.meta.url).pathname;
await stat(join(root, 'pkg')).catch(() => {
  throw new Error('web/pkg is missing: run ./build.sh first');
});
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

// Runs in the page before anything else. Time is `now`, moved only by __step; every frame
// callback waits for it. Event times read the same clock, so a drag's speed is what the scene
// says it is. Only a frame that is filmed is drawn: the rest cost only the simulation, and a
// software GPU never falls behind.
function clockwork({ saved, voice }) {
  try {
    localStorage.clear();
    const now = String(Math.floor(Date.now() / 1000));
    for (const [k, v] of Object.entries(saved)) localStorage.setItem(k, v.replaceAll('NOW', now));
  } catch {}
  // Headless Chromium falls over when the page asks it about speech. A scene with `voice` gets a
  // stand-in that hears whatever __say is given, as if the browser had sent it away and back.
  delete window.SpeechRecognition;
  delete window.webkitSpeechRecognition;
  if (voice) {
    let heard = [];
    window.SpeechRecognition = class {
      start() {
        window.__rec = this;
      }
      stop() {}
      abort() {}
    };
    window.__say = (text) => {
      heard = [...heard, Object.assign([{ transcript: text, confidence: 1 }], { isFinal: true })];
      window.__rec?.onresult?.({ resultIndex: heard.length - 1, results: heard });
      return !!window.__rec;
    };
  }
  let now = 0;
  let queue = [];
  performance.now = () => now;
  window.requestAnimationFrame = (cb) => queue.push(cb);
  window.cancelAnimationFrame = () => {};
  Object.defineProperty(Event.prototype, 'timeStamp', { get: () => now });
  let dark = false;
  for (const proto of [WebGL2RenderingContext.prototype, WebGLRenderingContext.prototype]) {
    const draw = proto.drawArrays;
    proto.drawArrays = function (...a) {
      if (!dark) draw.apply(this, a);
    };
  }
  // `n` frames of `ms` each; only the last is drawn, and only if `drawn`.
  window.__step = (ms, n, drawn) => {
    for (let i = 0; i < n; i++) {
      now += ms;
      dark = !(drawn && i === n - 1);
      const due = queue;
      queue = [];
      for (const cb of due) cb(now);
    }
  };
  // Pointers are dispatched straight at the canvas, so two fingers are as easy as one.
  window.__pointer = (type, id, x, y) => {
    const c = document.querySelector('canvas');
    const r = c.getBoundingClientRect();
    const at = { clientX: r.left + r.width / 2 + x * r.height, clientY: r.top + r.height / 2 + y * r.height };
    const init = { pointerId: id, pointerType: 'touch', isPrimary: id === 1, ...at, bubbles: true, cancelable: true, buttons: type === 'pointerup' ? 0 : 1 };
    c.dispatchEvent(new PointerEvent(type, init));
  };
}

// A second of near silence, as a WAV file, for the stand-in microphone.
async function quiet() {
  const rate = 48000;
  const wav = Buffer.alloc(44 + rate * 2);
  wav.write('RIFF', 0);
  wav.writeUInt32LE(36 + rate * 2, 4);
  wav.write('WAVEfmt ', 8);
  wav.writeUInt32LE(16, 16);
  wav.writeUInt16LE(1, 20); // PCM
  wav.writeUInt16LE(1, 22); // mono
  wav.writeUInt32LE(rate, 24);
  wav.writeUInt32LE(rate * 2, 28);
  wav.writeUInt16LE(2, 32);
  wav.writeUInt16LE(16, 34);
  wav.write('data', 36);
  wav.writeUInt32LE(rate * 2, 40);
  const path = join(await mkdtemp(join(tmpdir(), 'gif-quiet-')), 'quiet.wav');
  await writeFile(path, wav);
  return path;
}

const STEP = 20; // ms of its time per frame: under the speed at which it starts drawing less.
const every = Math.max(1, Math.round(1000 / fps / STEP));

async function record(name, scene) {
  const [w, h] = scene.size || [width, Math.round((width * 5) / 8)];
  // A scene with `voice` gets a microphone, given without asking, that hears a quiet room.
  const ears = scene.voice ? ['--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream', `--use-file-for-fake-audio-capture=${await quiet()}`, '--autoplay-policy=no-user-gesture-required'] : [];
  const browser = await chromium.launch({ channel: 'chromium', args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', ...ears] });
  const page = await browser.newPage({ viewport: { width: w, height: h }, permissions: scene.voice ? ['microphone'] : [] });
  page.on('pageerror', (e) => console.error(`${name}: page error: ${e.message}`));
  await page.addInitScript(clockwork, { saved: scene.saved || {}, voice: !!scene.voice });
  await page.goto(url);
  const frames = await mkdtemp(join(tmpdir(), `gif-${name}-`));
  let shot = 0;
  let tick = 0;
  let rolling = false;
  let per = every;
  const film = () => page.screenshot({ path: join(frames, `f${String(shot++).padStart(5, '0')}.png`) });
  // Frames run in batches up to the next one filmed; when nothing is filmed, in batches of 50.
  const step = async (n = 1) => {
    while (n > 0) {
      const k = rolling ? Math.min(n, per - (tick % per)) : Math.min(n, 50);
      const shoot = rolling && (tick + k) % per === 0;
      await page.evaluate(([ms, k, d]) => window.__step(ms, k, d), [STEP, k, shoot]);
      tick += k;
      n -= k;
      if (shoot) await film();
    }
  };
  const frames_ = (secs) => Math.max(1, Math.round((secs * 1000) / STEP));
  const p = (type, id, x, y) => page.evaluate(([t, i, a, b]) => window.__pointer(t, i, a, b), [type, id, x, y]);
  const t = {
    // Lets its time run for `secs`, filmed if rolling.
    wait: (secs) => step(frames_(secs)),
    // Films from here on; `speed` above 1 films faster than it happens.
    roll(speed = 1) {
      rolling = true;
      per = Math.max(1, Math.round(every * speed));
    },
    cut: () => (rolling = false),
    down: (id, x, y) => p('pointerdown', id, x, y),
    move: (id, x, y) => p('pointermove', id, x, y),
    up: (id, x, y) => p('pointerup', id, x, y),
    // A finger along `path` ([[x, y], ...]) over `secs`. Every place is measured from the
    // middle of the view in view heights, as the camera is: x right, y down, [0, 0.5] the bottom edge.
    async drag(path, secs, id = 1, { lift = true } = {}) {
      const n = frames_(secs);
      const at = (k) => {
        const f = (k / n) * (path.length - 1);
        const i = Math.min(Math.floor(f), path.length - 2);
        const u = f - i;
        return [path[i][0] + (path[i + 1][0] - path[i][0]) * u, path[i][1] + (path[i + 1][1] - path[i][1]) * u];
      };
      await t.down(id, ...path[0]);
      for (let k = 1; k <= n; k++) {
        await t.move(id, ...at(k));
        await step();
      }
      if (lift) await t.up(id, ...path[path.length - 1]);
    },
    async tap(x, y, id = 1) {
      await t.down(id, x, y);
      await step(2);
      await t.up(id, x, y);
    },
    // Pressed and held still for `secs`.
    async hold(x, y, secs, id = 1) {
      await t.down(id, x, y);
      await t.wait(secs);
      await t.up(id, x, y);
    },
    // Lets its time run until `test(arg)` (run in the page) is true, at most `secs`.
    async until(test, arg, secs = 60) {
      for (let left = frames_(secs); left > 0; left -= 10) {
        if (await page.evaluate(test, arg)) return;
        await step(10);
      }
      throw new Error(`${name}: waited ${secs} s for ${test}`);
    },
    // Waits, at most `secs`, until a line on the page says `words`.
    asked: (words, secs) => t.until((w) => [...document.querySelectorAll('[data-glyph]')].some((e) => e.textContent.includes(w)), words, secs),
    // Taps a word on the page (its id).
    click: (id) => page.evaluate((id) => document.getElementById(id).click(), id),
    // Words, as if said aloud: for a scene with `voice`, once it has been let hear them.
    say: (text) => page.evaluate((text) => window.__say(text), text),
    // A still of this moment, for working out where things are: <out>/<scene>-<label>.png.
    async snap(label) {
      await page.evaluate((ms) => window.__step(ms, 1, true), STEP);
      tick++;
      await mkdir(out, { recursive: true });
      await page.screenshot({ path: join(out, `${name}-${label}.png`) });
    },
    key: (code, secs = 0.1) => page.keyboard.down(code).then(() => t.wait(secs)).then(() => page.keyboard.up(code)),
    page,
  };
  await page.waitForFunction(() => document.body.classList.contains('awake') || document.body.classList.contains('failed') || (window.__step(20, 1, true), false), null, { timeout: 120000, polling: 50 }).catch(() => {});
  if (await page.evaluate(() => !document.body.classList.contains('awake'))) throw new Error(`${name}: the page never woke`);
  const t0 = Date.now();
  await scene.play(t);
  await browser.close();
  if (!shot) throw new Error(`${name}: nothing was filmed (call t.roll())`);
  await mkdir(out, { recursive: true });
  const gif = join(out, `${name}.gif`);
  const scale = w === width ? '' : `scale=${width}:-1:flags=lanczos,`;
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', String(1000 / (STEP * every)), '-i', join(frames, 'f%05d.png'),
    '-vf', `${scale}split[a][b];[a]palettegen=max_colors=${colors}:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle`,
    '-loop', '0', gif]);
  await rm(frames, { recursive: true });
  const kb = Math.round((await stat(gif)).size / 1024);
  console.log(`${gif}  ${shot} frames, ${(shot / (1000 / (STEP * every))).toFixed(1)} s, ${kb} KB, ${((Date.now() - t0) / 1000).toFixed(0)} s to make`);
  if (kb > 2000) console.log(`  over 2 MB, and it is committed: try --width 360, --fps 10 or a shorter take`);
}

try {
  for (const name of wanted) await record(name, scenes[name]);
} finally {
  server.close();
}
