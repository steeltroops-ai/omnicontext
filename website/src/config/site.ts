export const siteConfig = {
  name: "OmniContext",
  shortName: "Omni",
  description:
    "The foundation for AI coding agents. A highly parallel, local, and universal code context engine written in Rust.",
  url: "https://omnicontext.dev",
  links: {
    github: "https://github.com/steeltroops-ai/omnicontext",
    twitter: "https://twitter.com/steeltroops",
    portfolio: "https://steeltroops.vercel.app",
    email: "steeltroops.ai@gmail.com",
    docs: "/docs",
    // Removed dead links - pages don't exist yet
    // support: "/support",
    // enterprise: "/enterprise",
    // blog: "/blog",
  },
  branding: {
    logo: "/logo.svg", // Static path for favicon/external
    sizes: {
      header: 22,
      footer: 18,
      hero: 48,
      feature: 32,
    },
  },
  author: {
    name: "Mayank (steeltroops)",
    url: "https://steeltroops.vercel.app",
    email: "steeltroops.ai@gmail.com",
  },
  meta: {
    themeColor: "#000000",
  },
};

export type SiteConfig = typeof siteConfig;
