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

// Each shape gets a full-card composition: one hero wireframe + constellation dots + accent
const SHAPES = [
  // 0 — Hexagon (top-right) + triangle (bottom-left)
  `<g transform="translate(960, 30)" opacity="0.35">
    <polygon points="70,0 140,40 140,120 70,160 0,120 0,40" fill="none" stroke="#f59e0b" stroke-width="1.5"/>
    <polygon points="70,28 112,48 112,112 70,132 28,112 28,48" fill="none" stroke="#f59e0b" stroke-width="1" opacity="0.45"/>
    <line x1="70" y1="0" x2="70" y2="160" stroke="#f59e0b" stroke-width="0.5" opacity="0.3"/>
    <line x1="0" y1="40" x2="140" y2="120" stroke="#f59e0b" stroke-width="0.5" opacity="0.3"/>
    <line x1="140" y1="40" x2="0" y2="120" stroke="#f59e0b" stroke-width="0.5" opacity="0.3"/>
    <circle cx="70" cy="80" r="3" fill="#f59e0b" opacity="0.6"/>
    <circle cx="70" cy="0" r="2" fill="#f59e0b" opacity="0.5"/><circle cx="140" cy="40" r="2" fill="#f59e0b" opacity="0.5"/>
    <circle cx="140" cy="120" r="2" fill="#f59e0b" opacity="0.5"/><circle cx="70" cy="160" r="2" fill="#f59e0b" opacity="0.5"/>
    <circle cx="0" cy="120" r="2" fill="#f59e0b" opacity="0.5"/><circle cx="0" cy="40" r="2" fill="#f59e0b" opacity="0.5"/>
  </g>
  <g transform="translate(55, 540)" opacity="0.25">
    <polygon points="35,0 70,60 0,60" fill="none" stroke="#f59e0b" stroke-width="1.2"/>
    <polygon points="35,12 58,52 12,52" fill="none" stroke="#f59e0b" stroke-width="0.8" opacity="0.5"/>
  </g>`,

  // 1 — Diamond (bottom-right) + constellation
  `<g transform="translate(970, 410)" opacity="0.3">
    <rect x="10" y="10" width="110" height="110" fill="none" stroke="#f59e0b" stroke-width="1.5" transform="rotate(45, 65, 65)"/>
    <rect x="30" y="30" width="70" height="70" fill="none" stroke="#f59e0b" stroke-width="1" opacity="0.5" transform="rotate(45, 65, 65)"/>
    <line x1="65" y1="0" x2="65" y2="130" stroke="#f59e0b" stroke-width="0.5" opacity="0.4"/>
    <line x1="0" y1="65" x2="130" y2="65" stroke="#f59e0b" stroke-width="0.5" opacity="0.4"/>
    <circle cx="65" cy="65" r="4" fill="#f59e0b" opacity="0.5"/>
    <circle cx="65" cy="0" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="130" cy="65" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="65" cy="130" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="0" cy="65" r="2" fill="#f59e0b" opacity="0.4"/>
  </g>`,

  // 2 — Octagon (right-center)
  `<g transform="translate(960, 220)" opacity="0.3">
    <polygon points="47,0 113,0 160,47 160,113 113,160 47,160 0,113 0,47" fill="none" stroke="#f59e0b" stroke-width="1.5"/>
    <polygon points="47,28 105,28 132,55 132,105 105,132 47,132 28,105 28,55" fill="none" stroke="#f59e0b" stroke-width="1" opacity="0.45"/>
    <line x1="80" y1="0" x2="80" y2="160" stroke="#f59e0b" stroke-width="0.5" opacity="0.25"/>
    <line x1="0" y1="80" x2="160" y2="80" stroke="#f59e0b" stroke-width="0.5" opacity="0.25"/>
    <circle cx="80" cy="80" r="3" fill="#f59e0b" opacity="0.5"/>
    <circle cx="47" cy="0" r="2" fill="#f59e0b" opacity="0.4"/><circle cx="113" cy="0" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="160" cy="47" r="2" fill="#f59e0b" opacity="0.4"/><circle cx="160" cy="113" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="113" cy="160" r="2" fill="#f59e0b" opacity="0.4"/><circle cx="47" cy="160" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="0" cy="113" r="2" fill="#f59e0b" opacity="0.4"/><circle cx="0" cy="47" r="2" fill="#f59e0b" opacity="0.4"/>
  </g>`,

  // 3 — Pentagon (top-right) + small triangle
  `<g transform="translate(975, 30)" opacity="0.35">
    <polygon points="70,0 140,50 115,130 25,130 0,50" fill="none" stroke="#f59e0b" stroke-width="1.5"/>
    <polygon points="70,25 110,58 93,110 47,110 30,58" fill="none" stroke="#f59e0b" stroke-width="1" opacity="0.45"/>
    <line x1="70" y1="0" x2="70" y2="130" stroke="#f59e0b" stroke-width="0.5" opacity="0.3"/>
    <circle cx="70" cy="65" r="3" fill="#f59e0b" opacity="0.6"/>
    <circle cx="70" cy="0" r="2" fill="#f59e0b" opacity="0.5"/><circle cx="140" cy="50" r="2" fill="#f59e0b" opacity="0.5"/>
    <circle cx="115" cy="130" r="2" fill="#f59e0b" opacity="0.5"/><circle cx="25" cy="130" r="2" fill="#f59e0b" opacity="0.5"/>
    <circle cx="0" cy="50" r="2" fill="#f59e0b" opacity="0.5"/>
  </g>
  <g transform="translate(60, 545)" opacity="0.2">
    <polygon points="25,0 50,44 0,44" fill="none" stroke="#f59e0b" stroke-width="1"/>
  </g>`,

  // 4 — Double triangle / Star of David (right-center)
  `<g transform="translate(975, 200)" opacity="0.3">
    <polygon points="65,0 130,112 0,112" fill="none" stroke="#f59e0b" stroke-width="1.5"/>
    <polygon points="65,112 130,0 0,0" fill="none" stroke="#f59e0b" stroke-width="1.5"/>
    <circle cx="65" cy="56" r="3" fill="#f59e0b" opacity="0.5"/>
    <circle cx="65" cy="0" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="130" cy="112" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="0" cy="112" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="65" cy="112" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="130" cy="0" r="2" fill="#f59e0b" opacity="0.4"/>
    <circle cx="0" cy="0" r="2" fill="#f59e0b" opacity="0.4"/>
  </g>`,
];

// Shared constellation dots + lines (always present)
const constellation = `
  <g opacity="0.25">
    <circle cx="420" cy="55" r="2" fill="#f59e0b"/>
    <circle cx="580" cy="85" r="1.5" fill="#f59e0b"/>
    <circle cx="730" cy="50" r="2.5" fill="#f59e0b"/>
    <circle cx="500" cy="510" r="2" fill="#f59e0b"/>
    <circle cx="680" cy="540" r="2.5" fill="#f59e0b"/>
    <line x1="420" y1="55" x2="580" y2="85" stroke="#f59e0b" stroke-width="0.5" opacity="0.3"/>
    <line x1="580" y1="85" x2="730" y2="50" stroke="#f59e0b" stroke-width="0.5" opacity="0.3"/>
    <line x1="500" y1="510" x2="680" y2="540" stroke="#f59e0b" stroke-width="0.5" opacity="0.3"/>
  </g>`;

// Deterministic pick from slug hash (no Math.random — must be stable at build time)
function pickShape(slug) {
  let hash = 0;
  for (let i = 0; i < slug.length; i++) {
    hash = ((hash << 5) - hash + slug.charCodeAt(i)) | 0;
  }
  return SHAPES[Math.abs(hash) % SHAPES.length];
}

export async function GET({ props }) {
  const { post } = props;
  const title = post.data.title;
  const description = post.data.description || '';
  const tag = (post.data.tags && post.data.tags[0]) || 'GPUI';

  // Step 1: Generate text layout SVG via satori
  const textSvg = await satori(
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
        },
        children: [
          // Left accent bar
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
          // Brand + tag
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
                  props: { style: { fontSize: 28, fontWeight: 600, color: '#f59e0b' }, children: 'gpui-starter' },
                },
                {
                  type: 'span',
                  props: { style: { fontSize: 20, color: '#4b4b5a' }, children: '|' },
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
          // Accent bar
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
          // Title
          {
            type: 'div',
            props: {
              style: {
                fontSize: 52,
                fontWeight: 800,
                color: '#e7e7ed',
                lineHeight: 1.2,
                letterSpacing: '-0.02em',
                maxWidth: 850,
              },
              children: title,
            },
          },
          // Bottom row: description + url
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
                      lineHeight: 1.4,
                    },
                    children: description,
                  },
                },
                {
                  type: 'span',
                  props: {
                    style: { fontSize: 14, fontFamily: 'monospace', color: '#4b4b5a' },
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

  // Step 2: Inject one random shape + constellation into the SVG string
  const shape = pickShape(post.id);
  const fullSvg = textSvg.replace('</svg>', `\n  ${shape}\n  ${constellation}\n</svg>`);

  // Step 3: Render to PNG via Resvg
  const resvg = new Resvg(fullSvg, {
    fitTo: { mode: 'width', value: 1200 },
  });
  const pngBuffer = resvg.render().asPng();

  return new Response(pngBuffer, {
    headers: { 'Content-Type': 'image/png' },
  });
}
