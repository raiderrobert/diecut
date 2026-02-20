// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
export default defineConfig({
  integrations: [
    starlight({
      title: "diecut",
      logo: {
        src: "./src/assets/logo.svg",
      },
      favicon: "/favicon.png",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/raiderrobert/diecut",
        },
      ],
      sidebar: [
        {
          label: "Getting Started",
          items: [
            { label: "Installation & Quick Start", slug: "getting-started" },
          ],
        },
        {
          label: "Guides",
          items: [
            { label: "Using Templates", slug: "using-templates" },
            { label: "Creating Templates", slug: "creating-templates" },
          ],
        },
        {
          label: "Tutorials",
          items: [
            { label: "Personal project template", slug: "tutorials/personal-template" },
            { label: "Monorepo packages", slug: "tutorials/monorepo-packages" },
            { label: "Repeating in-project patterns", slug: "tutorials/repeating-patterns" },
            { label: "Multi-file feature scaffolding", slug: "tutorials/code-scaffolding" },
            { label: "FastAPI resource scaffolding", slug: "tutorials/fastapi-endpoint" },
            { label: "Structured content", slug: "tutorials/structured-content" },
            { label: "Prompt and skill templates", slug: "tutorials/prompt-template" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "Commands", slug: "reference/commands" },
            { label: "diecut.toml", slug: "reference/diecut-toml" },
            { label: "Hooks", slug: "reference/hooks" },
          ],
        },
      ],
    }),
  ],
});
