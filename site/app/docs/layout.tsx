import type { ReactNode } from "react";
import { DocNav } from "@/components/doc-nav";
import { container } from "@/lib/ui";

type Props = { children: ReactNode };

export default function DocsLayout({ children }: Props) {
  return (
    <div className={container}>
      <div className="grid gap-8 py-8 lg:grid-cols-[13rem_minmax(0,1fr)] lg:gap-12 lg:py-14">
        <div className="lg:sticky lg:top-8 lg:self-start">
          <details className="rounded-card border border-line bg-surface lg:hidden">
            <summary className="cursor-pointer px-4 py-3 text-small font-medium text-fg">
              Documentation menu
            </summary>
            <div className="px-2 pb-3">
              <DocNav />
            </div>
          </details>
          <div className="hidden lg:block">
            <DocNav />
          </div>
        </div>
        <main className="min-w-0 max-w-[720px] pb-8" id="main">
          {children}
        </main>
      </div>
    </div>
  );
}
