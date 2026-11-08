import { DocPage } from "@/components/doc-page";
import { copyText } from "@/lib/docs-content";
import { docMetadata } from "@/lib/docs-nav";

export const metadata = docMetadata("/docs/copy-text");

export default function Page() {
  return <DocPage blocks={copyText} href="/docs/copy-text" />;
}
