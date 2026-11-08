import { DocPage } from "@/components/doc-page";
import { translate } from "@/lib/docs-content";
import { docMetadata } from "@/lib/docs-nav";

export const metadata = docMetadata("/docs/translate");

export default function Page() {
  return <DocPage blocks={translate} href="/docs/translate" />;
}
