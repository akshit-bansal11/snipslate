import { DocPage } from "@/components/doc-page";
import { troubleshooting } from "@/lib/docs-content";
import { docMetadata } from "@/lib/docs-nav";

export const metadata = docMetadata("/docs/troubleshooting");

export default function Page() {
  return <DocPage blocks={troubleshooting} href="/docs/troubleshooting" />;
}
