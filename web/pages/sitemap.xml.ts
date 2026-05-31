import type { APIRoute } from 'astro';

export const GET: APIRoute = async ({ redirect }) => {
  return redirect('/sitemap-index.xml', 301);
};
