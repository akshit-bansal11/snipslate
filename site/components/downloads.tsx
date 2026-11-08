import { Download } from "lucide-react";
import { TextLink } from "@/components/text-link";
import { downloads } from "@/lib/landing";
import { RELEASES_URL } from "@/lib/site";
import {
  btnPrimary,
  container,
  h3Class,
  panel,
  sectionClass,
  titleClass,
} from "@/lib/ui";

export function Downloads() {
  return (
    <section
      aria-labelledby="download-title"
      className={sectionClass}
      id="download"
    >
      <div className={container}>
        <h2 className={titleClass} id="download-title">
          Download
        </h2>
        <p className="mt-4 text-lead text-muted">
          Free and open source under the MIT license.
        </p>
        <ul className="mt-8 space-y-4">
          {downloads.map((row) => (
            <li
              className={`${panel} grid gap-4 p-6 md:grid-cols-[1fr_auto] md:items-center`}
              key={row.system}
            >
              <div>
                <h3 className={h3Class}>{row.system}</h3>
                <p className="mt-1 text-muted">{row.requirement}</p>
                <p className="mt-2 text-small text-muted">{row.note}</p>
              </div>
              <a className={btnPrimary} href={RELEASES_URL}>
                <Download aria-hidden="true" className="size-5" />
                Download for {row.system}
              </a>
            </li>
          ))}
        </ul>
        <p className="mt-6">
          <TextLink href="/docs/install#verify">
            Verify with SHA256SUMS.txt
          </TextLink>
        </p>
      </div>
    </section>
  );
}
