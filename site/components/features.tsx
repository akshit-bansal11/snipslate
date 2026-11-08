import { features } from "@/lib/landing";
import { container, sectionClass, titleClass } from "@/lib/ui";

export function Features() {
  return (
    <section
      aria-labelledby="features-title"
      className={sectionClass}
      id="features"
    >
      <div className={container}>
        <h2 className={titleClass} id="features-title">
          Features
        </h2>
        <dl className="mt-10 grid gap-x-12 gap-y-10 sm:grid-cols-2">
          {features.map((feature) => (
            <div key={feature.title}>
              <dt className="flex items-center gap-3 text-heading font-semibold text-fg">
                <feature.icon
                  aria-hidden="true"
                  className="size-5 shrink-0 text-accent-text"
                />
                {feature.title}
              </dt>
              <dd className="mt-2 text-muted">{feature.body}</dd>
            </div>
          ))}
        </dl>
      </div>
    </section>
  );
}
