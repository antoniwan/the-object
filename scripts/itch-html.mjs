// Turns a page written for itch.io in Markdown into the HTML its editors take, so a post or the
// game's description is pasted whole, pictures and all (the pictures are served from GitHub):
//
//     node scripts/itch-html.mjs devlog/2026-10-29.md     # writes devlog/2026-10-29.html
//     node scripts/itch-html.mjs itch/page.md             # writes itch/page.html
//     node scripts/itch-html.mjs --check                  # every page's .html is up to date
//
// Only what the pages use: paragraphs, a picture on a line of its own, **strong**, *em*, links
// and ---. A first line `# title` is left out: itch takes the title in a field of its own.
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));

const escape = (s) => s.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');

function inline(text) {
  return escape(text)
    .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, t, u) => `<a href="${u}" target="_blank" rel="noreferrer">${t}</a>`)
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\*([^*]+)\*/g, '<em>$1</em>');
}

export function toHtml(md) {
  const blocks = md.replace(/^# .*\n/, '').split(/\n\s*\n/).map((b) => b.trim()).filter(Boolean);
  return blocks
    .map((b) => {
      if (b === '---') return '<hr>';
      const pic = b.match(/^!\[([^\]]*)\]\(([^)\s]+)\)$/);
      if (pic) return `<p><img src="${pic[2]}" alt="${escape(pic[1])}"></p>`;
      return `<p>${inline(b.split('\n').join(' '))}</p>`;
    })
    .join('\n') + '\n';
}

// The pages there are: the game's description and every daily post.
export function pages() {
  const daily = readdirSync(`${root}devlog`).filter((n) => /^\d{4}-\d\d-\d\d\.md$/.test(n)).map((n) => `devlog/${n}`);
  return ['itch/page.md', ...daily];
}

const htmlOf = (page) => page.replace(/\.md$/, '.html');

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  if (args[0] === '--check') {
    const stale = pages().filter((p) => {
      try {
        return readFileSync(root + htmlOf(p), 'utf8') !== toHtml(readFileSync(root + p, 'utf8'));
      } catch {
        return true;
      }
    });
    if (stale.length) {
      console.error(`out of date, run node scripts/itch-html.mjs ${stale.join(' ')}`);
      process.exit(1);
    }
  } else {
    const wanted = args.length ? args.map((a) => relative(root, resolve(a))) : pages();
    for (const p of wanted) {
      writeFileSync(root + htmlOf(p), toHtml(readFileSync(root + p, 'utf8')));
      console.log(htmlOf(p));
    }
  }
}
