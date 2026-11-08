import { Screenshot } from "@/components/screenshot";
import { settingsNotes } from "@/lib/landing";
import { container, h3Class, sectionClass, titleClass } from "@/lib/ui";

export function SettingsShowcase() {
  return (
    <section
      aria-labelledby="settings-title"
      className={sectionClass}
      id="settings"
    >
      <div className={container}>
        <div className="grid items-center gap-10 lg:grid-cols-[auto_1fr] lg:gap-16">
          <div className="mx-auto w-full max-w-[420px]">
            <Screenshot
              alt="The Snipslate Settings window in its dark theme, showing the AI provider, translation language and shortcut sections."
              height={720}
              sizes="(min-width: 1024px) 420px, calc(100vw - 32px)"
              src="/screenshots/settings.png"
              width={560}
            />
          </div>
          <div>
            <h2 className={titleClass} id="settings-title">
              Set it up once
            </h2>
            <dl className="mt-8 space-y-6">
              {settingsNotes.map((note) => (
                <div key={note.title}>
                  <dt className={h3Class}>{note.title}</dt>
                  <dd className="mt-1 text-muted">{note.body}</dd>
                </div>
              ))}
            </dl>
          </div>
        </div>
      </div>
    </section>
  );
}
