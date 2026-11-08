import Link from "next/link";
import type { ReactNode } from "react";
import { textLink } from "@/lib/ui";

type Props = { href: string; children: ReactNode };

export function TextLink({ href, children }: Props) {
  if (href.startsWith("http")) {
    return (
      <a className={textLink} href={href} rel="noreferrer">
        {children}
      </a>
    );
  }
  return (
    <Link className={textLink} href={href}>
      {children}
    </Link>
  );
}
