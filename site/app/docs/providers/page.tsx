import { DocPage } from "@/components/doc-page";
import { providers } from "@/lib/docs-content";
import { docMetadata } from "@/lib/docs-nav";

export const metadata = docMetadata("/docs/providers");

export default function Page() {
  return <DocPage blocks={providers} href="/docs/providers" />;
}
