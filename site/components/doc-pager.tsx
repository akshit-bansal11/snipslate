import Link from "next/link";
import type { DocEntry } from "@/lib/docs-nav";

const pager =
  "block rounded-card border border-line bg-surface p-4 hover:border-accent";
const pagerRight = `${pager} sm:text-right`;

type LinkProps = { doc: DocEntry; label: string; right?: boolean };

function PagerLink({ doc, label, right }: LinkProps) {
  return (
    <Link className={right ? pagerRight : pager} href={doc.href}>
      <span className="block text-caption text-muted">{label}</span>
      <span className="block font-medium text-accent-text">{doc.title}</span>
    </Link>
  );
}

type Props = { prev?: DocEntry; next?: DocEntry };

export function DocPager({ prev, next }: Props) {
  return (
    <nav
      aria-label="Previous and next pages"
      className="mt-16 grid gap-3 border-t border-line pt-8 sm:grid-cols-2"
    >
      {prev ? <PagerLink doc={prev} label="Previous" /> : <span />}
      {next ? <PagerLink doc={next} label="Next" right /> : <span />}
    </nav>
  );
}
