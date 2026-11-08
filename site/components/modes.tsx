import { Languages, ScanText } from "lucide-react";
import type { ReactNode } from "react";
import { Screenshot } from "@/components/screenshot";
import { modes } from "@/lib/landing";
import { container, h3Class, panel, sectionClass, titleClass } from "@/lib/ui";

type ModeProps = {
  icon: typeof Languages;
  title: string;
  body: string;
  points: string[];
  children?: ReactNode;
};

function Mode({ icon: Icon, title, body, points, children }: ModeProps) {
  return (
    <div className={`${panel} flex flex-col p-6 sm:p-8`}>
      <Icon aria-hidden="true" className="size-7 text-accent-text" />
      <h3 className={`${h3Class} mt-4`}>{title}</h3>
      <p className="mt-3 text-muted">{body}</p>
      <ul className="mt-5 list-disc space-y-1.5 pl-5 marker:text-muted">
        {points.map((point) => (
          <li key={point}>{point}</li>
        ))}
      </ul>
      {children}
    </div>
  );
}

export function Modes() {
  return (
    <section aria-labelledby="modes-title" className={sectionClass} id="modes">
      <div className={container}>
        <h2 className={titleClass} id="modes-title">
          Two ways to snip
        </h2>
        <div className="mt-10 grid gap-6 lg:grid-cols-2">
          <Mode icon={Languages} {...modes.translate} />
          <Mode icon={ScanText} {...modes.copy}>
            <div className="mt-6">
              <Screenshot
                alt="A small toast reading Copied 163 characters, Paste it anywhere."
                height={800}
                imageClass="h-60 w-full object-cover object-[30%_62%]"
                sizes="(min-width: 1024px) 520px, calc(100vw - 32px)"
                src="/screenshots/copy-toast.png"
                width={1280}
              />
            </div>
          </Mode>
        </div>
      </div>
    </section>
  );
}
