import { defineConfig, passthroughImageService } from "astro/config";
import starlight from "@astrojs/starlight";

// Static output. The folder also holds contributor notes (README.md, how-to/,
// *-plan.md); only src/content/docs/ is part of the site.
export default defineConfig({
  site: "https://docs.usecactus.dev",
  // Screenshots are already sized; no sharp needed.
  image: { service: passthroughImageService() },
  integrations: [
    starlight({
      title: "cactus docs",
      logo: { src: "./src/assets/logo.svg", alt: "" },
      favicon: "/favicon.svg",
      customCss: ["./src/styles/custom.css"],
      lastUpdated: false,
      sidebar: [
        {
          label: "Start here",
          items: [
            { label: "What cactus is", slug: "start/introduction" },
            { label: "Install", slug: "start/install" },
            { label: "Getting started", slug: "start/getting-started" },
          ],
        },
        {
          label: "Guides",
          items: [
            { label: "Requests", slug: "guides/requests" },
            { label: "Variables and environments", slug: "guides/variables" },
            { label: "Auth", slug: "guides/auth" },
            { label: "Checks and captures", slug: "guides/checks-and-captures" },
            { label: "Bots", slug: "guides/critters" },
            { label: "Flows", slug: "guides/flows" },
            { label: "Schedules and alerts", slug: "guides/schedules" },
            { label: "Secrets and the vault", slug: "guides/secrets" },
            { label: "Git", slug: "guides/git" },
            { label: "Importing", slug: "guides/importing" },
            { label: "AI agents (MCP)", slug: "guides/mcp" },
          ],
        },
        {
          label: "Recipes",
          items: [
            { label: "API tests in CI", slug: "recipes/ci" },
            { label: "Check every role's permissions", slug: "recipes/permissions" },
            { label: "Wait for a background job", slug: "recipes/polling" },
            { label: "A support tool in a flow", slug: "recipes/support-tool" },
            { label: "Watch staging, alert Slack", slug: "recipes/monitoring" },
            { label: "Security checks in CI", slug: "recipes/security-checks" },
            { label: "Sign in with OAuth2", slug: "recipes/oauth2" },
            { label: "Moving from Postman", slug: "recipes/from-postman" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "CLI", slug: "reference/cli" },
            { label: "Flow file", slug: "reference/flow-file" },
            { label: "Flow tools", slug: "reference/flow-tools" },
            { label: "Expressions and operators", slug: "reference/operators" },
            { label: "Project layout", slug: "reference/layout" },
            { label: "Keyboard shortcuts", slug: "reference/shortcuts" },
          ],
        },
        {
          label: "Extending",
          items: [
            { label: "Write a flow tool", slug: "extending/flow-tool" },
            { label: "Add an operator or generator", slug: "extending/operators-and-generators" },
            { label: "Add a bot species", slug: "extending/species" },
            { label: "Add a theme", slug: "extending/theme" },
          ],
        },
      ],
    }),
  ],
});
