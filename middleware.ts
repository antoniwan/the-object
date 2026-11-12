// Markdown for AI agents on the home page. A request for / that asks for Markdown
// (Accept: text/markdown) gets this site's llms.txt, which is Markdown, as text/markdown;
// every other request, browsers included, goes on to the normal HTML page untouched.
// vercel.json adds Vary: Accept to the home page so caches keep the two apart.
export const config = { matcher: '/' };

export default async function middleware(request: Request): Promise<Response | undefined> {
  const accept = request.headers.get('accept') ?? '';
  if (!/\btext\/markdown\b/i.test(accept)) return undefined;
  const llms = await fetch(new URL('/llms.txt', request.url));
  const body = llms.ok ? await llms.text() : '';
  if (!body.trim()) return undefined;
  return new Response(body, {
    headers: {
      'content-type': 'text/markdown; charset=utf-8',
      vary: 'Accept',
      'cache-control': 'public, max-age=300',
    },
  });
}
