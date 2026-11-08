import { DocPage } from "@/components/doc-page";
import { privacy } from "@/lib/docs-content";
import { docMetadata } from "@/lib/docs-nav";

export const metadata = docMetadata("/docs/privacy");

export default function Page() {
  return <DocPage blocks={privacy} href="/docs/privacy" />;
}
