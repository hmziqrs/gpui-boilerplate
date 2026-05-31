export const meta = {
  name: 'fix-seo-issues',
  description: 'Fix all 11 SEO issues from the audit across multiple files',
  phases: [
    { title: 'Fix', detail: 'Apply SEO fixes across all files in parallel' },
    { title: 'Verify', detail: 'Build and verify changes compile' },
  ],
}

// ─── Phase 1: Fix all SEO issues in parallel ───
phase('Fix')

const fixes = await parallel([

  // FIX 1: MarketingLayout.astro — viewport initial-scale=1
  () => agent("Fix the file /Users/hmziq/os/gpui-app/web/layouts/MarketingLayout.astro line 80.\n\nChange:\n  <meta name=\"viewport\" content=\"width=device-width\" />\nTo:\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\nThis is a single line edit. Only change that one line.", { label: 'fix-viewport', phase: 'Fix' }),

  // FIX 2: Docs index.md — fix title from "gpui-starter" to "Documentation"
  () => agent("Fix the file /Users/hmziq/os/gpui-app/web/content/docs/docs/index.md\n\nChange the frontmatter title from:\n  title: gpui-starter\nTo:\n  title: Documentation\n\nThis fixes the duplicate \"gpui-starter | gpui-starter\" title tag. Only change that one line in the frontmatter.", { label: 'fix-docs-title', phase: 'Fix' }),

  // FIX 3: astro.config.mjs — Starlight head injection + sitemap integration
  () => agent("Modify /Users/hmziq/os/gpui-app/astro.config.mjs to fix multiple SEO issues for the docs/Starlight section.\n\nThe current file starts with these imports:\n  import { defineConfig } from \"astro/config\";\n  import starlight from \"@astrojs/starlight\";\n  import tailwindcss from \"@tailwindcss/vite\";\n  import icon from \"astro-icon\";\n  import { readFileSync } from \"fs\";\n  import { join } from \"path\";\n\nAdd a sitemap import right after the existing imports:\n  import sitemap from \"@astrojs/sitemap\";\n\nThen inside the starlight() config object, add a head option after the lastUpdated: true line. The head option injects SEO meta tags for all docs pages:\n\n      head: (head) => {\n        head.push(\n          { tag: 'meta', attrs: { property: 'og:title', content: 'gpui-starter Documentation' } },\n          { tag: 'meta', attrs: { property: 'og:description', content: 'A production-ready boilerplate for building desktop apps with GPUI' } },\n          { tag: 'meta', attrs: { property: 'og:type', content: 'website' } },\n          { tag: 'meta', attrs: { property: 'og:site_name', content: 'gpui-starter' } },\n          { tag: 'meta', attrs: { property: 'og:locale', content: 'en_US' } },\n          { tag: 'meta', attrs: { property: 'og:image', content: 'https://gpui-starter.hmziq.xyz/og-image.png' } },\n          { tag: 'meta', attrs: { property: 'og:image:width', content: '1200' } },\n          { tag: 'meta', attrs: { property: 'og:image:height', content: '630' } },\n          { tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },\n          { tag: 'meta', attrs: { name: 'twitter:site', content: '@hmziqrs' } },\n          { tag: 'meta', attrs: { name: 'twitter:creator', content: '@hmziqrs' } },\n        );\n      },\n\nThen add sitemap() to the integrations array. The integrations array currently has icon({...}). Add sitemap() as a new integration BEFORE icon:\n\n    sitemap({\n      filter: (page) => !page.includes('/api/'),\n    }),\n    icon({...}),\n\nRead the file first, then make these edits carefully.", { label: 'fix-starlight-head', phase: 'Fix' }),

  // FIX 4: sitemap.xml redirect
  () => agent("Create a new file at /Users/hmziq/os/gpui-app/web/pages/sitemap.xml.ts\n\nThis file should be an Astro API endpoint that redirects to the sitemap-index.xml. Write this exact content:\n\n---\nreturn Astro.redirect('/sitemap-index.xml');\n---\n\nThat is the entire file. This handles the case where crawlers check /sitemap.xml by convention.", { label: 'fix-sitemap-redirect', phase: 'Fix' }),

  // FIX 5: Blog posts — unique OG images via per-post image reference
  () => agent("Two tasks:\n\nTASK A: Modify /Users/hmziq/os/gpui-app/web/pages/blog/[...slug].astro\nRead the file first. Find the Layout component call (around line 25-30). Currently it has ogType=\"article\" on one line. Add a new prop on the next line:\n  ogImage={`/og/blog/${post.id}.png`}\n\nThis makes each blog post reference a unique OG image path instead of the default shared one.\n\nTASK B: Create the OG image generation endpoint at /Users/hmziq/os/gpui-app/web/pages/og/blog/[...slug].png.ts\n\nThis endpoint generates SVG-based OG images at build time for each blog post. Write this content:\n\n---\nimport { getCollection } from 'astro:content';\n\nexport async function getStaticPaths() {\n  const posts = await getCollection('blog');\n  return posts.map((post) => ({\n    params: { slug: post.id },\n    props: { post },\n  }));\n}\n\nconst { post } = Astro.props;\nconst title = post.data.title;\nconst description = post.data.description || '';\n\nconst svg = `<svg width=\"1200\" height=\"630\" xmlns=\"http://www.w3.org/2000/svg\">\n  <defs>\n    <linearGradient id=\"bg\" x1=\"0%\" y1=\"0%\" x2=\"100%\" y2=\"100%\">\n      <stop offset=\"0%\" style=\"stop-color:#06060a\"/>\n      <stop offset=\"100%\" style=\"stop-color:#1a1a2e\"/>\n    </linearGradient>\n    <linearGradient id=\"accent\" x1=\"0%\" y1=\"0%\" x2=\"100%\" y2=\"0%\">\n      <stop offset=\"0%\" style=\"stop-color:#fbbf24\"/>\n      <stop offset=\"100%\" style=\"stop-color:#f59e0b\"/>\n    </linearGradient>\n  </defs>\n  <rect width=\"1200\" height=\"630\" fill=\"url(#bg)\"/>\n  <rect x=\"0\" y=\"0\" width=\"6\" height=\"630\" fill=\"url(#accent)\"/>\n  <text x=\"80\" y=\"140\" font-family=\"system-ui, -apple-system, sans-serif\" font-size=\"28\" font-weight=\"600\" fill=\"#f59e0b\">gpui-starter</text>\n  <text x=\"80\" y=\"140\" font-family=\"system-ui, -apple-system, sans-serif\" font-size=\"20\" fill=\"#818194\" dx=\"210\">Blog</text>\n  <rect x=\"80\" y=\"180\" width=\"120\" height=\"3\" rx=\"1.5\" fill=\"url(#accent)\"/>\n  <foreignObject x=\"80\" y=\"210\" width=\"1040\" height=\"300\">\n    <div xmlns=\"http://www.w3.org/1999/xhtml\" style=\"font-family:system-ui,-apple-system,sans-serif; font-size:48px; font-weight:800; color:#e7e7ed; line-height:1.2; letter-spacing:-0.02em; word-wrap:break-word;\">${title}</div>\n  </foreignObject>\n  <foreignObject x=\"80\" y=\"500\" width=\"1040\" height=\"80\">\n    <div xmlns=\"http://www.w3.org/1999/xhtml\" style=\"font-family:system-ui,-apple-system,sans-serif; font-size:20px; color:#a8a8b8; line-height:1.4; overflow:hidden; text-overflow:ellipsis;\">${description}</div>\n  </foreignObject>\n  <text x=\"80\" y=\"610\" font-family=\"monospace\" font-size=\"14\" fill=\"#4b4b5a\">gpui-starter.hmziq.xyz</text>\n</svg>`;\n\nreturn new Response(svg, {\n  headers: { 'Content-Type': 'image/svg+xml' },\n});\n---\n\nMake sure to read the blog [...slug].astro file before editing it.", { label: 'fix-og-images', phase: 'Fix' }),
])

log("Applied " + fixes.filter(Boolean).length + "/5 fix groups")

// ─── Phase 2: Verify build ───
phase('Verify')

const verified = await agent("Run the Astro docs build to verify the changes compile correctly.\n\nFirst install @astrojs/sitemap if not already installed:\n  cd /Users/hmziq/os/gpui-app && bun add -d @astrojs/sitemap\n\nThen run the build:\n  cd /Users/hmziq/os/gpui-app && bun run docs:build 2>&1\n\nReport whether the build succeeds or fails, and if it fails, include the full error output.", { label: 'verify-build', phase: 'Verify' })

return { fixesApplied: fixes.filter(Boolean).length, buildResult: verified }
