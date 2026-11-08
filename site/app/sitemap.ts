import type { MetadataRoute } from "next";
import { DOCS } from "@/lib/docs-nav";
import { SITE_URL } from "@/lib/site";

export default function sitemap(): MetadataRoute.Sitemap {
  const hrefs = ["/", ...DOCS.map((doc) => doc.href)];
  return hrefs.map((href) => ({ url: `${SITE_URL}${href}` }));
}
