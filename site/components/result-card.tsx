const caption = "Anthropic · claude-opus-5-5 · AI translation";
const description =
  "An illustration of the result window, titled Translated to English. It shows the English text of the French notice.";
const body =
  "Update available. A new version of the app is ready. It will be installed the next time you restart. Save your work before continuing.";

export function ResultCard() {
  return (
    <figure className="m-0 mt-4 sm:absolute sm:bottom-0 sm:right-6 sm:mt-0 sm:w-80">
      <div
        aria-hidden="true"
        className="rounded-surface border border-line bg-surface p-4 shadow-lift"
      >
        <p className="text-small font-semibold text-fg">
          Translated to English
        </p>
        <p className="text-caption text-muted">{caption}</p>
        <p className="mt-3 border-l-2 border-marker pl-3 text-small leading-7 text-fg">
          {body}
        </p>
        <div className="mt-4 flex items-center gap-3">
          <span className="rounded-control bg-accent px-3 py-1.5 text-small font-medium text-on-accent">
            Copy
          </span>
          <span className="text-small text-muted">Close</span>
        </div>
      </div>
      <p className="sr-only">{description}</p>
      <figcaption className="mt-2 text-caption text-muted">
        Illustration of the result window
      </figcaption>
    </figure>
  );
}
