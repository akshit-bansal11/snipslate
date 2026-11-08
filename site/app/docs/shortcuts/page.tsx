import { DocPage } from "@/components/doc-page";
import { shortcuts } from "@/lib/docs-content";
import { docMetadata } from "@/lib/docs-nav";

export const metadata = docMetadata("/docs/shortcuts");

export default function Page() {
  return <DocPage blocks={shortcuts} href="/docs/shortcuts" />;
}
