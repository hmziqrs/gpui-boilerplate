import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import tailwindcss from "@tailwindcss/vite";
import icon from "astro-icon";

export default defineConfig({
  site: "https://gpui-starter.hmziq.xyz",
  srcDir: "./web",
  vite: {
    plugins: [tailwindcss()],
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
      description: "A boilerplate for building desktop apps with GPUI",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/hmziqrs/gpui-boilerplate",
        },
      ],
      editLink: {
        baseUrl: "https://github.com/hmziqrs/gpui-boilerplate/edit/main/",
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
          ],
        },
        {
          label: "Architecture",
          items: [{ slug: "docs/architecture" }, { slug: "docs/performance" }],
        },
      ],
      customCss: ["/web/styles/starlight.css"],
      lastUpdated: true,
      favicon: "/favicon.svg",
    }),
  ],
});
