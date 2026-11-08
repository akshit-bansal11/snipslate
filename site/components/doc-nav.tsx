"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { DOCS } from "@/lib/docs-nav";

const idleClass =
  "block rounded-control px-3 py-2 text-small text-muted hover:bg-sunken hover:text-fg";
const activeClass =
  "block rounded-control bg-sunken px-3 py-2 text-small font-medium text-fg";

export function DocNav() {
  const pathname = usePathname();
  return (
    <nav aria-label="Documentation">
      <ul className="m-0 flex list-none flex-col gap-1 p-0">
        {DOCS.map((doc) => {
          const current = pathname === doc.href;
          return (
            <li key={doc.href}>
              <Link
                aria-current={current ? "page" : undefined}
                className={current ? activeClass : idleClass}
                href={doc.href}
              >
                {doc.title}
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
