import { Check } from "lucide-react";
import { TextLink } from "@/components/text-link";
import { privacyPoints } from "@/lib/landing";
import { container, sectionClass, titleClass } from "@/lib/ui";

export function PrivacySection() {
  return (
    <section
      aria-labelledby="privacy-title"
      className={sectionClass}
      id="privacy"
    >
      <div className={container}>
        <h2 className={titleClass} id="privacy-title">
          Privacy
        </h2>
        <ul className="mt-8 max-w-2xl space-y-3">
          {privacyPoints.map((point) => (
            <li className="flex gap-3" key={point}>
              <Check
                aria-hidden="true"
                className="mt-1 size-5 shrink-0 text-accent-text"
              />
              <span>{point}</span>
            </li>
          ))}
        </ul>
        <p className="mt-6">
          <TextLink href="/docs/privacy">Read the full privacy page</TextLink>
        </p>
      </div>
    </section>
  );
}
