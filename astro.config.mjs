import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import tailwindcss from "@tailwindcss/vite";
import icon from "astro-icon";
import { readFileSync } from "fs";
import { join } from "path";

function serveLocalAudio() {
  return {
    name: "serve-local-audio",
    configureServer(server) {
      server.middlewares.use("/audio", (req, res, next) => {
        const filePath = join(process.cwd(), "audio", req.url.replace(/^\//, ""));
        try {
          const data = readFileSync(filePath);
          res.setHeader("Content-Type", "audio/mpeg");
          res.end(data);
        } catch {
          next();
        }
      });
    },
  };
}

export default defineConfig({
  site: "https://gpui-starter.hmziq.xyz",
  srcDir: "./web",
  vite: {
    plugins: [tailwindcss(), serveLocalAudio()],
    build: {
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (id.includes("node_modules/three")) return "three";
          },
        },
      },
    },
  },
  integrations: [
    icon({
      include: {
        "simple-icons": ["github", "rust", "x", "linkedin", "telegram", "reddit"],
        lucide: ["globe"],
      },
    }),
    starlight({
      title: "gpui-starter",
      description: "A production-ready Rust boilerplate for GPUI desktop apps with themes, i18n, forms, and more",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/hmziqrs/gpui-boilerplate",
        },
      ],
      editLink: {
        baseUrl: "https://github.com/hmziqrs/gpui-boilerplate/edit/master/",
      },
      sidebar: [
        {
          label: "Getting Started",
          items: [{ slug: "docs/getting-started" }],
        },
        {
          label: "Features",
          items: [
            { slug: "docs/themes" },
            { slug: "docs/i18n" },
            { slug: "docs/forms" },
            { slug: "docs/command-launcher" },
            { slug: "docs/notifications" },
            { slug: "docs/secure-storage" },
          ],
        },
        {
          label: "Architecture",
          items: [
            { slug: "docs/architecture" },
            { slug: "docs/routing" },
            { slug: "docs/testing" },
            { slug: "docs/performance" },
          ],
        },
      ],
      customCss: ["/web/styles/starlight.css"],
      lastUpdated: true,
      favicon: "/favicon.svg",
    }),
  ],
});
