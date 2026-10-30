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
import { SOFTWARE_GPU, chromium, serve } from './web-server.mjs';

const runs = Number(process.argv[2] || 9);
const { url, server } = serve();

const times = [];
for (let i = 0; i < runs; i++) {
  const browser = await chromium.launch({
    channel: 'chromium',
    args: [...SOFTWARE_GPU, '--disable-gpu-shader-disk-cache'],
  });
  const page = await browser.newPage({ viewport: { width: 48, height: 32 } });
  const t0 = Date.now();
  await page.goto(url);
  await page.waitForFunction(
    () => document.body.classList.contains('awake') || document.body.classList.contains('failed'),
    null,
    { timeout: 60000, polling: 10 },
  );
  if (await page.evaluate(() => document.body.classList.contains('failed')))
    throw new Error('the page gave up (body.failed)');
  times.push(Date.now() - t0);
  await browser.close();
}
server.close();
times.sort((a, b) => a - b);
const slow = times.filter((t) => t > times[times.length - 1] / 2);
console.log(`ms ${times.join(' ')}  median ${slow[slow.length >> 1]} (of the ${slow.length} uncached)`);
