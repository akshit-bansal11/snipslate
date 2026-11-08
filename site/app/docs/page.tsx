import { DocPage } from "@/components/doc-page";
import { intro } from "@/lib/docs-content";
import { docMetadata } from "@/lib/docs-nav";

export const metadata = docMetadata("/docs");

export default function Page() {
  return <DocPage blocks={intro} href="/docs" />;
}
