import { getCollection } from 'astro:content';
import satori from 'satori';
import { Resvg } from '@resvg/resvg-js';

export async function getStaticPaths() {
  const posts = await getCollection('blog');
  return posts.map((post) => ({
    params: { slug: post.id },
    props: { post },
  }));
}

export async function GET({ props }) {
  const { post } = props;
  const title = post.data.title;
  const description = post.data.description || '';
  const tag = (post.data.tags && post.data.tags[0]) || 'GPUI';

  const svg = await satori(
    {
      type: 'div',
      props: {
        style: {
          width: 1200,
          height: 630,
          display: 'flex',
          flexDirection: 'column',
          background: 'linear-gradient(135deg, #06060a 0%, #1a1a2e 100%)',
          padding: '80px',
          position: 'relative',
        },
        children: [
          // ─── Left accent bar ───
          {
            type: 'div',
            props: {
              style: {
                position: 'absolute',
                left: 0,
                top: 0,
                bottom: 0,
                width: 6,
                background: 'linear-gradient(180deg, #fbbf24, #f59e0b)',
              },
            },
          },
          // ─── Geometric decorations ───
          // Hexagon wireframe (top-right)
          {
            type: 'img',
            props: {
              width: 160,
              height: 180,
              style: { position: 'absolute', right: 80, top: 40 },
              src: `data:image/svg+xml,${encodeURIComponent('<svg width="160" height="180" viewBox="0 0 160 180" fill="none" xmlns="http://www.w3.org/2000/svg"><polygon points="80,8 148,48 148,132 80,172 12,132 12,48" stroke="rgba(245,158,11,0.35)" stroke-width="1.5" fill="none"/><polygon points="80,32 126,56 126,124 80,148 34,124 34,56" stroke="rgba(245,158,11,0.15)" stroke-width="1" fill="none"/><line x1="80" y1="8" x2="80" y2="172" stroke="rgba(245,158,11,0.08)" stroke-width="0.5"/><line x1="12" y1="48" x2="148" y2="132" stroke="rgba(245,158,11,0.08)" stroke-width="0.5"/><line x1="148" y1="48" x2="12" y2="132" stroke="rgba(245,158,11,0.08)" stroke-width="0.5"/><circle cx="80" cy="90" r="3" fill="rgba(245,158,11,0.5)"/></svg>')}`,
            },
          },
          // Icosahedron wireframe (bottom-right)
          {
            type: 'img',
            props: {
              width: 200,
              height: 200,
              style: { position: 'absolute', right: 30, bottom: 50 },
              src: `data:image/svg+xml,${encodeURIComponent('<svg width="200" height="200" viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg"><polygon points="100,12 180,58 180,142 100,188 20,142 20,58" stroke="rgba(245,158,11,0.12)" stroke-width="1" fill="none"/><polygon points="100,38 160,66 160,134 100,162 40,134 40,66" stroke="rgba(245,158,11,0.3)" stroke-width="1.5" fill="none"/><line x1="100" y1="38" x2="100" y2="162" stroke="rgba(245,158,11,0.12)" stroke-width="1"/><line x1="40" y1="66" x2="160" y2="134" stroke="rgba(245,158,11,0.12)" stroke-width="1"/><line x1="160" y1="66" x2="40" y2="134" stroke="rgba(245,158,11,0.12)" stroke-width="1"/><circle cx="100" cy="100" r="4" fill="rgba(245,158,11,0.4)"/><circle cx="100" cy="38" r="2.5" fill="rgba(245,158,11,0.35)"/><circle cx="160" cy="66" r="2.5" fill="rgba(245,158,11,0.35)"/><circle cx="160" cy="134" r="2.5" fill="rgba(245,158,11,0.35)"/><circle cx="100" cy="162" r="2.5" fill="rgba(245,158,11,0.35)"/><circle cx="40" cy="134" r="2.5" fill="rgba(245,158,11,0.35)"/><circle cx="40" cy="66" r="2.5" fill="rgba(245,158,11,0.35)"/></svg>')}`,
            },
          },
          // Floating dots + constellation lines (scattered)
          {
            type: 'img',
            props: {
              width: 700,
              height: 580,
              style: { position: 'absolute', left: 350, top: 30 },
              src: `data:image/svg+xml,${encodeURIComponent('<svg width="700" height="580" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="80" r="2" fill="rgba(245,158,11,0.3)"/><circle cx="200" cy="40" r="1.5" fill="rgba(245,158,11,0.25)"/><circle cx="380" cy="100" r="2.5" fill="rgba(245,158,11,0.3)"/><circle cx="550" cy="60" r="2" fill="rgba(245,158,11,0.25)"/><circle cx="650" cy="150" r="1.5" fill="rgba(245,158,11,0.2)"/><circle cx="120" cy="450" r="2" fill="rgba(245,158,11,0.25)"/><circle cx="400" cy="500" r="2.5" fill="rgba(245,158,11,0.3)"/><circle cx="580" cy="480" r="1.5" fill="rgba(245,158,11,0.2)"/><circle cx="300" cy="300" r="1.5" fill="rgba(245,158,11,0.15)"/><circle cx="500" cy="250" r="2" fill="rgba(245,158,11,0.2)"/><line x1="50" y1="80" x2="200" y2="40" stroke="rgba(245,158,11,0.1)" stroke-width="0.5"/><line x1="200" y1="40" x2="380" y2="100" stroke="rgba(245,158,11,0.1)" stroke-width="0.5"/><line x1="380" y1="100" x2="550" y2="60" stroke="rgba(245,158,11,0.1)" stroke-width="0.5"/><line x1="120" y1="450" x2="400" y2="500" stroke="rgba(245,158,11,0.1)" stroke-width="0.5"/><line x1="400" y1="500" x2="580" y2="480" stroke="rgba(245,158,11,0.1)" stroke-width="0.5"/></svg>')}`,
            },
          },
          // Triangle accent (bottom-left)
          {
            type: 'img',
            props: {
              width: 70,
              height: 62,
              style: { position: 'absolute', left: 60, bottom: 35 },
              src: `data:image/svg+xml,${encodeURIComponent('<svg width="70" height="62" viewBox="0 0 70 62" fill="none" xmlns="http://www.w3.org/2000/svg"><polygon points="35,4 66,58 4,58" stroke="rgba(245,158,11,0.25)" stroke-width="1.2" fill="none"/></svg>')}`,
            },
          },
          {
            type: 'div',
            props: {
              style: {
                display: 'flex',
                alignItems: 'center',
                gap: 16,
                marginBottom: 24,
              },
              children: [
                {
                  type: 'span',
                  props: {
                    style: {
                      fontSize: 28,
                      fontWeight: 600,
                      color: '#f59e0b',
                    },
                    children: 'gpui-starter',
                  },
                },
                {
                  type: 'span',
                  props: {
                    style: { fontSize: 20, color: '#4b4b5a' },
                    children: '|',
                  },
                },
                {
                  type: 'span',
                  props: {
                    style: {
                      fontSize: 18,
                      color: '#818194',
                      background: 'rgba(245, 158, 11, 0.1)',
                      padding: '4px 12px',
                      borderRadius: 6,
                      border: '1px solid rgba(245, 158, 11, 0.15)',
                    },
                    children: tag,
                  },
                },
              ],
            },
          },
          {
            type: 'div',
            props: {
              style: {
                width: 120,
                height: 3,
                background: 'linear-gradient(90deg, #fbbf24, #f59e0b)',
                borderRadius: 2,
                marginBottom: 40,
              },
            },
          },
          {
            type: 'div',
            props: {
              style: {
                fontSize: 52,
                fontWeight: 800,
                color: '#e7e7ed',
                lineHeight: 1.2,
                letterSpacing: '-0.02em',
                maxWidth: 1040,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                display: '-webkit-box',
                WebkitLineClamp: 3,
                WebkitBoxOrient: 'vertical',
              },
              children: title,
            },
          },
          {
            type: 'div',
            props: {
              style: {
                marginTop: 'auto',
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'flex-end',
              },
              children: [
                {
                  type: 'div',
                  props: {
                    style: {
                      fontSize: 20,
                      color: '#a8a8b8',
                      maxWidth: 800,
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      display: '-webkit-box',
                      WebkitLineClamp: 2,
                      WebkitBoxOrient: 'vertical',
                      lineHeight: 1.4,
                    },
                    children: description,
                  },
                },
                {
                  type: 'span',
                  props: {
                    style: {
                      fontSize: 14,
                      fontFamily: 'monospace',
                      color: '#4b4b5a',
                    },
                    children: 'gpui-starter.hmziq.xyz',
                  },
                },
              ],
            },
          },
        ],
      },
    },
    {
      width: 1200,
      height: 630,
      fonts: [
        {
          name: 'Inter',
          data: await fetch('https://cdn.jsdelivr.net/npm/@fontsource/inter@5/files/inter-latin-700-normal.woff').then(r => r.arrayBuffer()),
          weight: 700,
          style: 'normal',
        },
        {
          name: 'Inter',
          data: await fetch('https://cdn.jsdelivr.net/npm/@fontsource/inter@5/files/inter-latin-800-normal.woff').then(r => r.arrayBuffer()),
          weight: 800,
          style: 'normal',
        },
      ],
    },
  );

  const resvg = new Resvg(svg, {
    fitTo: { mode: 'width', value: 1200 },
  });
  const pngData = resvg.render();
  const pngBuffer = pngData.asPng();

  return new Response(pngBuffer, {
    headers: { 'Content-Type': 'image/png' },
  });
}
