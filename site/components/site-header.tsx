import Link from "next/link";
import { Logo } from "@/components/logo";
import { container } from "@/lib/ui";

const navLink =
  "rounded-control px-2 py-2 text-small font-medium text-muted hover:text-fg";
const navLinkWide =
  "hidden rounded-control px-2 py-2 text-small font-medium text-muted hover:text-fg sm:inline-block";
const navButton =
  "rounded-control bg-accent px-3 py-2 text-small font-medium text-on-accent hover:bg-accent-hover";

export function SiteHeader() {
  return (
    <header className="border-b border-line bg-bg">
      <div className={container}>
        <div className="flex h-16 items-center justify-between gap-2">
          <Link
            className="flex items-center gap-2.5 text-heading font-semibold text-fg"
            href="/"
          >
            <Logo />
            Snipslate
          </Link>
          <nav aria-label="Main" className="flex items-center gap-1 sm:gap-2">
            <Link className={navLinkWide} href="/#features">
              Features
            </Link>
            <Link className={navLink} href="/docs">
              Docs
            </Link>
            <Link className={navButton} href="/#download">
              Download
            </Link>
          </nav>
        </div>
      </div>
    </header>
  );
}
