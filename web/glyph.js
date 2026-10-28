// Every word on this page arrives as something else first. Each letter passes through a circle, a
// shape, a sign and a piece of markup before it settles, tinted by the object and thrown off it; when
// a word leaves, it goes back the same way. The object writes --tint, --unrest, --ox and --oy on
// <html> as it lives (src/aura.rs). Elements marked data-glyph are watched; the wasm only sets text.

const STAGES = [
  [...'○◌◍◎●◐◑◒◓◯◦∘⊙⊚⊛⦾⦿◉'],
  [...'△▽◇◆□■▲▼◢◣◤◥◊⬡⬢⬠▱▰⟁⧖'],
  [...'∴∵∷⊕⊗☉☽✶⟡⌬⍜⍟'],
  [...'<>/{}[]=#;&|\\_~^*+'],
];
/** Glyphs sit a little smaller than the letters they become. */
const SCALE = [0.62, 0.7, 0.82, 0.86];
const STEP = 85; // ms a letter spends in each stage
const STAGGER = 28; // ms between neighbouring letters
const SWAP = 60; // ms before an unsettled glyph changes its mind
const SPREAD = 650; // ms from the first letter starting to the last

const root = document.documentElement;
const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
const pick = (a) => a[(Math.random() * a.length) | 0];
const jobs = new Map();
/** Elements that have already said goodbye to their current words. */
const gone = new WeakSet();
let frame = 0;

/** The object's state, as the wasm last wrote it. Inline style, so reading it costs no layout. */
function aura() {
  const v = (k, d) => parseFloat(root.style.getPropertyValue(k)) || d;
  return {
    unrest: v('--unrest', 0),
    x: (v('--ox', 50) / 100) * innerWidth,
    y: (v('--oy', 50) / 100) * innerHeight,
  };
}

/** Splits an element's text into one <i> per letter, words kept whole. */
function wrap(el) {
  unwrap(el);
  const letters = [];
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
  const nodes = [];
  while (walker.nextNode()) nodes.push(walker.currentNode);
  for (const n of nodes) {
    const frag = document.createDocumentFragment();
    for (const part of n.data.split(/(\s+)/)) {
      if (!part) continue;
      if (/^\s+$/.test(part)) {
        frag.append(part);
        continue;
      }
      const word = document.createElement('span');
      word.className = 'gw';
      word.dataset.text = part;
      for (const ch of part) {
        const i = document.createElement('i');
        i.textContent = ch;
        word.append(i);
        letters.push({ el: i, ch, since: -1e9, stage: -2 });
      }
      frag.append(word);
    }
    n.replaceWith(frag);
  }
  // Each letter keeps the width of what it will become, so nothing around it moves.
  const rects = letters.map((l) => l.el.getBoundingClientRect());
  letters.forEach((l, k) => {
    l.el.style.width = `${rects[k].width}px`;
    l.cx = rects[k].left + rects[k].width / 2;
    l.cy = rects[k].top + rects[k].height / 2;
  });
  return letters;
}

/** Puts plain text back, kerning and all. */
function unwrap(el) {
  for (const w of el.querySelectorAll('.gw')) w.replaceWith(w.dataset.text);
  el.normalize();
}

/**
 * dir 'in': from nothing to words. 'out': from words to nothing.
 * 'stir': the words are already there; a few letters forget themselves and come back.
 */
function decode(el, dir, done) {
  if (reduced) {
    done?.();
    return;
  }
  const letters = wrap(el);
  if (!letters.length) {
    done?.();
    return;
  }
  const a = aura();
  const n = letters.length;
  const order = letters.map((_, k) => k);
  if (dir === 'out') order.sort(() => Math.random() - 0.5);
  let stir = null;
  if (dir === 'stir') {
    const len = 1 + ((Math.random() * Math.min(4, n)) | 0);
    const from = (Math.random() * (n - len + 1)) | 0;
    stir = [from, from + len];
  }
  // Long lines spread faster, so every line arrives in about the same breath.
  const gap = Math.min(STAGGER, SPREAD / n);
  letters.forEach((l, k) => {
    const rank = order.indexOf(k);
    const chaos = 1 + a.unrest * 3;
    l.delay = dir === 'out' ? rank * gap * 0.5 : rank * gap + Math.random() * gap * 3 * chaos;
    l.dx = a.x - l.cx;
    l.dy = a.y - l.cy;
    if (stir) l.delay = k >= stir[0] && k < stir[1] ? Math.random() * STEP : Infinity;
    if (dir === 'in') l.el.className = 'hid'; // not even one frame of the answer
  });
  jobs.set(el, { letters, dir, start: performance.now(), done });
  wake();
}

/** Draws one letter at time t (ms since its own start). Returns true once it is done moving. */
function paint(l, t, dir, now) {
  const step = Math.floor(t / STEP);
  let stage, finished;
  if (dir === 'in') {
    stage = t < 0 ? -1 : step;
    finished = stage >= 4;
  } else if (dir === 'out') {
    stage = t < 0 ? 4 : 3 - step;
    finished = stage < 0;
  } else {
    // A stirred letter skips the circles; the rest never move.
    stage = t < 0 ? 4 : 1 + step;
    finished = t === -Infinity || stage >= 4;
  }

  const st = l.el.style;
  if (stage >= 4) {
    if (l.stage !== 4) {
      l.el.textContent = l.ch;
      l.el.className = '';
      st.transform = '';
      l.stage = 4;
    }
  } else if (stage < 0) {
    if (l.stage !== -1) {
      l.el.className = 'hid';
      l.stage = -1;
    }
  } else {
    if (stage !== l.stage || now - l.since > SWAP * (0.6 + Math.random())) {
      l.el.textContent = pick(STAGES[stage]);
      l.el.className = `g s${stage}`;
      l.since = now;
      l.stage = stage;
    }
    // Letters are thrown off the object and land as they settle; leaving, they are drawn back in.
    const p = (stage + (dir === 'out' ? 1 - (t / STEP - step) : t / STEP - step)) / 4;
    const pull = dir === 'stir' ? 0 : 0.45 * (1 - p) ** 3;
    st.transform = `translate(${l.dx * pull}px, ${l.dy * pull}px) scale(${SCALE[stage]})`;
  }
  return finished;
}

function tick(now) {
  frame = 0;
  for (const [el, job] of jobs) {
    let finished = true;
    for (const l of job.letters) {
      const t = l.delay === Infinity ? -Infinity : now - job.start - l.delay;
      if (!paint(l, t, job.dir, now)) finished = false;
    }
    if (finished) {
      jobs.delete(el);
      if (job.dir !== 'out') unwrap(el);
      job.done?.();
    }
  }
  stir(now);
  observer.takeRecords(); // our own writes are not news
  if (jobs.size || aura().unrest > 0.25) wake();
}

function wake() {
  if (!frame) frame = requestAnimationFrame(tick);
}

/** When it is upset, the words it has already said do not hold still. */
let lastStir = 0;
function stir(now) {
  const u = aura().unrest;
  if (u < 0.25 || now - lastStir < 90) return;
  lastStir = now;
  for (const el of watched) {
    if (jobs.has(el) || !shown(el)) continue;
    if (Math.random() < (u - 0.2) * 0.12) decode(el, 'stir');
  }
}

const watched = [...document.querySelectorAll('[data-glyph]')];
function shown(el) {
  if (!el.textContent.trim() || !document.body.classList.contains('awake')) return false;
  if (el.id === 'toast') return el.classList.contains('show');
  if (el.id === 'hint') return !el.classList.contains('gone');
  return el.closest('#veil') === null;
}

const observer = new MutationObserver((records) => {
  const fresh = new Set();
  const changed = new Set();
  for (const r of records) {
    const node = r.target.nodeType === 1 ? r.target : r.target.parentElement;
    const el = node?.closest('[data-glyph]');
    if (!el) continue;
    if (r.type === 'attributes') changed.add(el);
    else fresh.add(el);
  }
  for (const el of fresh) {
    gone.delete(el);
    decode(el, 'in');
  }
  for (const el of changed) {
    if (fresh.has(el)) continue;
    const leaving =
      (el.id === 'toast' && !el.classList.contains('show')) ||
      (el.id === 'hint' && el.classList.contains('gone'));
    // The wasm re-sends the same class every frame; only the first one is news.
    if (leaving && !gone.has(el)) {
      gone.add(el);
      decode(el, 'out');
    }
  }
  observer.takeRecords();
});
for (const el of watched) {
  if (el.closest('#veil')) continue;
  observer.observe(el, {
    childList: true,
    subtree: true,
    characterData: true,
    attributes: true,
    attributeFilter: ['class'],
  });
}

// Before it appears: the loader murmurs one line at a time, each arriving and leaving as glyphs.
const whisper = [...document.querySelectorAll('#whisper span:not(.lost)')];
const lost = document.querySelector('#whisper .lost');
let line = 0;
let murmuring = !reduced;
function murmur() {
  if (!murmuring) return;
  const el = whisper[line % whisper.length];
  line++;
  el.classList.add('on');
  decode(el, 'in', () =>
    setTimeout(() => {
      if (!murmuring) return;
      decode(el, 'out', () => {
        el.classList.remove('on');
        setTimeout(murmur, 500);
      });
    }, 3200),
  );
}
const veilWatch = new MutationObserver(() => {
  const b = document.body.classList;
  if (b.contains('awake') || b.contains('failed')) {
    murmuring = false;
    veilWatch.disconnect();
  }
  if (b.contains('awake')) {
    const hint = document.getElementById('hint');
    if (!hint.classList.contains('gone')) decode(hint, 'in');
    const count = document.getElementById('count');
    if (count.textContent) decode(count, 'in');
  } else if (b.contains('failed')) {
    for (const el of whisper) {
      jobs.delete(el);
      el.classList.remove('on');
    }
    lost.hidden = false;
    decode(lost, 'in');
  }
});
veilWatch.observe(document.body, { attributes: true, attributeFilter: ['class'] });

// Widths are measured in the real faces, so wait for them (but not for long).
Promise.race([document.fonts.ready, new Promise((r) => setTimeout(r, 600))]).then(() => {
  if (murmuring) murmur();
});
