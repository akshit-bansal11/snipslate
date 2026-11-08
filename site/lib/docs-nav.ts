import type { Metadata } from "next";

export type DocEntry = {
  href: string;
  title: string;
  description: string;
};

export const DOCS: DocEntry[] = [
  {
    href: "/docs",
    title: "Introduction",
    description: "What Snipslate does and a five-step quick start.",
  },
  {
    href: "/docs/install",
    title: "Install",
    description: "Install Snipslate on Windows, macOS or Linux.",
  },
  {
    href: "/docs/providers",
    title: "Providers",
    description: "Set up Anthropic, OpenAI, OpenRouter or a custom server.",
  },
  {
    href: "/docs/translate",
    title: "Translate",
    description: "How a snip becomes a translation, and the result window.",
  },
  {
    href: "/docs/copy-text",
    title: "Copy text",
    description: "Copy text from the screen with on-device recognition.",
  },
  {
    href: "/docs/shortcuts",
    title: "Shortcuts",
    description: "Default shortcuts, recording your own, and Wayland setup.",
  },
  {
    href: "/docs/privacy",
    title: "Privacy",
    description: "What leaves your computer, where, and what never does.",
  },
  {
    href: "/docs/troubleshooting",
    title: "Troubleshooting",
    description: "Fixes for the problems people are most likely to hit.",
  },
];

export function docMetadata(href: string): Metadata {
  const entry = DOCS.find((doc) => doc.href === href);
  if (!entry) {
    throw new Error(`Unknown docs page: ${href}`);
  }
  return {
    title: entry.title,
    description: entry.description,
    alternates: { canonical: href },
  };
}
