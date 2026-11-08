import { DocPage } from "@/components/doc-page";
import { install } from "@/lib/docs-content";
import { docMetadata } from "@/lib/docs-nav";

export const metadata = docMetadata("/docs/install");

export default function Page() {
  return <DocPage blocks={install} href="/docs/install" />;
}
