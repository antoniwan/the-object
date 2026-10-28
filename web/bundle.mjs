// Inlines the wasm-bindgen JS glue and the wasm binary (base64) into one HTML file.
import { readFileSync, writeFileSync } from 'node:fs';
const [pkg, template, out] = process.argv.slice(2);
const glue = readFileSync(`${pkg}/chaos_object.js`, 'utf8')
  .replace(/^export default __wbg_init;.*$/m, '')
  .replace(/^export \{[^}]*\};?$/m, '')
  .replace(/^export /gm, '');
const wasm = readFileSync(`${pkg}/chaos_object_bg.wasm`).toString('base64');
const bytes = `Uint8Array.from(atob("${wasm}"), c => c.charCodeAt(0))`;
const html = readFileSync(template, 'utf8')
  .replace('/*GLUE*/', () => glue)
  .replace('/*WASM*/', () => bytes);
writeFileSync(out, html);
