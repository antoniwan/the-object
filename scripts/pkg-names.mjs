// Called by build.sh after wasm-bindgen. With --hash (the Vercel build) it renames the two
// files in web/pkg/ to carry a hash of their contents, chaos_object.<hash>.js and
// chaos_object_bg.<hash>.wasm, so vercel.json can cache /pkg/ for a year: a deploy that
// changes them changes their names. Without --hash the names stay plain. Either way it points
// the generated JS and every pkg/ link in web/index.html at the names now on disk; on a plain
// build of a clean tree that leaves index.html untouched.
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, renameSync } from 'node:fs';

const hash = process.argv.includes('--hash');
const pkg = 'web/pkg';
const page = 'web/index.html';
const short = (buf) => createHash('sha256').update(buf).digest('hex').slice(0, 8);

let wasm = 'chaos_object_bg.wasm';
if (hash) {
  const named = `chaos_object_bg.${short(readFileSync(`${pkg}/${wasm}`))}.wasm`;
  renameSync(`${pkg}/${wasm}`, `${pkg}/${named}`);
  wasm = named;
}

let js = readFileSync(`${pkg}/chaos_object.js`, 'utf8');
const ref = "new URL('chaos_object_bg.wasm', import.meta.url)";
if (!js.includes(ref)) throw new Error(`pkg-names: ${ref} not found in the generated JS`);
js = js.replace(ref, `new URL('${wasm}', import.meta.url)`);
let jsName = 'chaos_object.js';
if (hash) {
  jsName = `chaos_object.${short(js)}.js`;
  renameSync(`${pkg}/chaos_object.js`, `${pkg}/${jsName}`);
}
writeFileSync(`${pkg}/${jsName}`, js);

const before = readFileSync(page, 'utf8');
const after = before
  .replace(/pkg\/chaos_object(\.[0-9a-f]{8})?\.js/g, `pkg/${jsName}`)
  .replace(/pkg\/chaos_object_bg(\.[0-9a-f]{8})?\.wasm/g, `pkg/${wasm}`);
if (!after.includes(`pkg/${jsName}`)) throw new Error(`pkg-names: ${page} never loads pkg/chaos_object.js`);
if (after !== before) writeFileSync(page, after);
console.log(`pkg-names: ${jsName}, ${wasm}`);
