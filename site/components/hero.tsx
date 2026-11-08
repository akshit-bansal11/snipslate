import { BookOpen, Download } from "lucide-react";
import Link from "next/link";
import { ResultCard } from "@/components/result-card";
import { Screenshot } from "@/components/screenshot";
import { hero } from "@/lib/landing";
import { btnPrimary, btnSecondary, container } from "@/lib/ui";

export function Hero() {
  return (
    <section className="pb-16 pt-12 sm:pb-24 sm:pt-20">
      <div className={container}>
        <div className="rise">
          <h1 className="max-w-3xl text-balance text-display font-semibold tracking-tight text-fg">
            {hero.title}
          </h1>
          <p className="mt-5 max-w-2xl text-lead text-muted">{hero.lead}</p>
          <div className="mt-8 flex flex-wrap items-center gap-3">
            <Link className={btnPrimary} href="#download">
              <Download aria-hidden="true" className="size-5" />
              Download
            </Link>
            <Link className={btnSecondary} href="/docs">
              <BookOpen aria-hidden="true" className="size-5" />
              Read the docs
            </Link>
          </div>
          <p className="mt-4 text-small text-muted">{hero.note}</p>
        </div>
        <div className="rise-late relative mt-12 sm:pb-14">
          <Screenshot
            alt="Snipslate's selection box with corner handles over a French update notice, with a size label below it."
            height={800}
            priority
            sizes="(min-width: 1120px) 1088px, calc(100vw - 32px)"
            src="/screenshots/overlay.png"
            width={1280}
          />
          <ResultCard />
        </div>
      </div>
    </section>
  );
}
