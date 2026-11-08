import { Kbd } from "@/components/kbd";
import { Screenshot } from "@/components/screenshot";
import { TextLink } from "@/components/text-link";
import { steps } from "@/lib/landing";
import { container, h3Class, sectionClass, titleClass } from "@/lib/ui";

export function Steps() {
  return (
    <section
      aria-labelledby="how-title"
      className={sectionClass}
      id="how-it-works"
    >
      <div className={container}>
        <div className="grid gap-10 lg:grid-cols-[2fr_3fr] lg:gap-16">
          <div>
            <h2 className={titleClass} id="how-title">
              How it works
            </h2>
            <p className="mt-4 text-lead text-muted">
              From shortcut to translation in three steps.
            </p>
            <div className="mt-8">
              <Screenshot
                alt="Snipslate waiting for a drag, with the hint Copy text, Drag to select an area, Esc to cancel."
                height={800}
                sizes="(min-width: 1024px) 40vw, calc(100vw - 32px)"
                src="/screenshots/overlay-idle.png"
                width={1280}
              />
            </div>
          </div>
          <ol className="list-decimal pl-8 marker:font-mono marker:text-accent-text">
            {steps.map((step) => (
              <li
                className="border-t border-line py-6 pl-2 first:border-t-0 first:pt-0"
                key={step.title}
              >
                <h3 className={h3Class}>{step.title}</h3>
                <p className="mt-2 text-muted">{step.body}</p>
                {step.keys ? (
                  <div className="mt-3 flex flex-wrap items-center gap-3 text-small">
                    <Kbd combo={step.keys} />
                    <TextLink href={step.href}>{step.linkLabel}</TextLink>
                  </div>
                ) : null}
              </li>
            ))}
          </ol>
        </div>
      </div>
    </section>
  );
}
