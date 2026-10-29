// After contact it asks one question (src/mind/contact.rs). The answer goes to api/rating, and what
// all the answers come to goes into the page's structured data, once there is at least one.
addEventListener('the-object:answer', (e) => {
  fetch('/api/rating', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ v: e.detail }), keepalive: true }).catch(() => {});
});
// Read only once the veil has lifted (or the page gave up), on idle: it never helps the first frame.
let asked = false;
function enrich() {
  if (asked) return;
  asked = true;
  fetch('/api/rating').then((r) => (r.ok ? r.json() : null)).then((a) => {
    if (!a || !(a.count >= 1)) return;
    const ld = document.querySelector('script[type="application/ld+json"]');
    const data = JSON.parse(ld.textContent);
    const game = data['@graph'].find((n) => n['@id'].endsWith('#the-object'));
    game.aggregateRating = { '@type': 'AggregateRating', ratingValue: a.value, ratingCount: a.count, bestRating: a.best, worstRating: a.worst };
    ld.textContent = JSON.stringify(data);
  }).catch(() => {});
}
const body = document.body.classList;
const ready = new MutationObserver(() => {
  if (!body.contains('awake') && !body.contains('failed')) return;
  ready.disconnect();
  if (window.requestIdleCallback) requestIdleCallback(enrich, { timeout: 2000 });
  else setTimeout(enrich, 500);
});
ready.observe(document.body, { attributes: true, attributeFilter: ['class'] });
setTimeout(enrich, 10000);
