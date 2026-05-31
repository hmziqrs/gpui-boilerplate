---
import { getCollection } from 'astro:content';

export async function getStaticPaths() {
  const posts = await getCollection('blog');
  return posts.map((post) => ({
    params: { slug: post.id },
    props: { post },
  }));
}

const { post } = Astro.props;
const title = post.data.title;
const description = post.data.description || '';

const svg = `<svg width="1200" height="630" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#06060a"/>
      <stop offset="100%" style="stop-color:#1a1a2e"/>
    </linearGradient>
    <linearGradient id="accent" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" style="stop-color:#fbbf24"/>
      <stop offset="100%" style="stop-color:#f59e0b"/>
    </linearGradient>
  </defs>
  <rect width="1200" height="630" fill="url(#bg)"/>
  <rect x="0" y="0" width="6" height="630" fill="url(#accent)"/>
  <text x="80" y="140" font-family="system-ui, -apple-system, sans-serif" font-size="28" font-weight="600" fill="#f59e0b">gpui-starter</text>
  <text x="80" y="140" font-family="system-ui, -apple-system, sans-serif" font-size="20" fill="#818194" dx="210">Blog</text>
  <rect x="80" y="180" width="120" height="3" rx="1.5" fill="url(#accent)"/>
  <foreignObject x="80" y="210" width="1040" height="300">
    <div xmlns="http://www.w3.org/1999/xhtml" style="font-family:system-ui,-apple-system,sans-serif; font-size:48px; font-weight:800; color:#e7e7ed; line-height:1.2; letter-spacing:-0.02em; word-wrap:break-word;">${title}</div>
  </foreignObject>
  <foreignObject x="80" y="500" width="1040" height="80">
    <div xmlns="http://www.w3.org/1999/xhtml" style="font-family:system-ui,-apple-system,sans-serif; font-size:20px; color:#a8a8b8; line-height:1.4; overflow:hidden; text-overflow:ellipsis;">${description}</div>
  </foreignObject>
  <text x="80" y="610" font-family="monospace" font-size="14" fill="#4b4b5a">gpui-starter.hmziq.xyz</text>
</svg>`;

return new Response(svg, {
  headers: { 'Content-Type': 'image/svg+xml' },
});
---
