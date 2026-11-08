import type { Metadata, Viewport } from "next";
import { Instrument_Sans, JetBrains_Mono } from "next/font/google";
import type { ReactNode } from "react";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { SITE_URL, TAGLINE } from "@/lib/site";
import "./globals.css";

const sans = Instrument_Sans({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  variable: "--font-instrument",
  display: "swap",
});

const mono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains",
  display: "swap",
});

const description =
  "Snipslate is a tray app for Windows, macOS and Linux. Snip any text on your screen, then translate it with the AI provider you choose or copy it with on-device recognition.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: { default: TAGLINE, template: "%s · Snipslate" },
  description,
  alternates: { canonical: "/" },
  openGraph: {
    type: "website",
    siteName: "Snipslate",
    url: "/",
    title: TAGLINE,
    description,
  },
  twitter: {
    card: "summary_large_image",
    title: TAGLINE,
    description,
    images: ["/opengraph-image"],
  },
};

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#F1F5F9" },
    { media: "(prefers-color-scheme: dark)", color: "#0F172A" },
  ],
};

type Props = { children: ReactNode };

export default function RootLayout({ children }: Props) {
  return (
    <html className={`${sans.variable} ${mono.variable}`} lang="en">
      <body>
        <a
          className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-control focus:bg-accent focus:px-4 focus:py-2 focus:text-on-accent"
          href="#main"
        >
          Skip to content
        </a>
        <SiteHeader />
        {children}
        <SiteFooter />
      </body>
    </html>
  );
}
