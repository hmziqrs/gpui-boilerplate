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
